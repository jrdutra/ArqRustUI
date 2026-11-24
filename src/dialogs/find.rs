//! Nonmodal Find/Replace dialogs.

use crate::core::application as app;
use crate::core::widget::Widget;
use crate::core::{GuiError, GuiResult};
use crate::ffi::comdlg32::*;
use crate::ffi::user32::{DestroyWindow, IsWindow, RegisterWindowMessageW};
use crate::ffi::{HWND, LPARAM};
use crate::utilities::strings::copy_to_buffer;
use crate::utilities::{from_wide, to_wide};
use std::cell::{Cell, RefCell, UnsafeCell};
use std::rc::Rc;

const BUFFER_LEN: usize = 256;

/// User-requested Find/Replace action.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FindAction {
    /// Find Next.
    FindNext,
    /// Replace current occurrence.
    Replace,
    /// Replace All.
    ReplaceAll,
    /// Dialog closed.
    Closed,
}

/// Find/Replace event data.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct FindEvent {
    /// Requested action.
    pub action: FindAction,
    /// Search text.
    pub text: String,
    /// Replacement text (empty in Find).
    pub replacement: String,
    /// Case-sensitive.
    pub match_case: bool,
    /// Whole words only.
    pub whole_word: bool,
    /// Search downward (`false` = upward).
    pub search_down: bool,
}

struct FindState {
    /// Dialog uses structure/buffers throughout its lifetime; stable heap allocations accessed only by UI thread.
    config: UnsafeCell<Box<FINDREPLACEW>>,
    _find: Box<[u16; BUFFER_LEN]>,
    _replace: Box<[u16; BUFFER_LEN]>,
    hwnd: Cell<HWND>,
    callback: Rc<dyn Fn(&FindEvent)>,
}

thread_local! {
    static MESSAGE_ID: Cell<u32> = const { Cell::new(0) };
    static ACTIVE: RefCell<Vec<Rc<FindState>>> = const { RefCell::new(Vec::new()) };
}

fn message_id() -> u32 {
    MESSAGE_ID.with(|id| {
        if id.get() == 0 {
            let name = to_wide("commdlg_FindReplace");
            // SAFETY: string terminada em NUL; registra (ou obtém) a mensagem dos diálogos comuns.
            id.set(unsafe { RegisterWindowMessageW(name.as_ptr()) });
        }
        id.get()
    })
}

/// Handles the registered Find dialog message; returns `true` if matched.
pub(crate) fn dispatch(message: u32, lparam: LPARAM) -> bool {
    if message != MESSAGE_ID.with(|id| id.get()) || message == 0 {
        return false;
    }
    let state = ACTIVE.with(|active| {
        active
            .borrow()
            .iter()
            // SAFETY: leitura do endereço da estrutura, sem criar referências concorrentes.
            .find(|s| unsafe { (&**s.config.get()) as *const FINDREPLACEW as LPARAM } == lparam)
            .cloned()
    });
    let Some(state) = state else { return true };
    // SAFETY: `lparam` é o endereço do FINDREPLACEW pertencente a `state`, que o diálogo
    // acabou de preencher; apenas esta thread o acessa e o diálogo aguarda o retorno.
    let config = unsafe { &**state.config.get() };
    let flags = config.Flags;
    // SAFETY: os ponteiros apontam para os buffers de BUFFER_LEN unidades possuídos por `state`.
    let (text, replacement) = unsafe {
        (
            from_wide(std::slice::from_raw_parts(config.lpstrFindWhat, BUFFER_LEN)),
            if config.lpstrReplaceWith.is_null() {
                String::new()
            } else {
                from_wide(std::slice::from_raw_parts(config.lpstrReplaceWith, BUFFER_LEN))
            },
        )
    };
    let action = if flags & FR_DIALOGTERM != 0 {
        FindAction::Closed
    } else if flags & FR_REPLACEALL != 0 {
        FindAction::ReplaceAll
    } else if flags & FR_REPLACE != 0 {
        FindAction::Replace
    } else {
        FindAction::FindNext
    };
    if action == FindAction::Closed {
        let hwnd = state.hwnd.replace(std::ptr::null_mut());
        if let Some(app) = app::state() {
            app.modeless.borrow_mut().retain(|h| *h != hwnd);
        }
        ACTIVE.with(|active| active.borrow_mut().retain(|s| !Rc::ptr_eq(s, &state)));
    }
    (state.callback)(&FindEvent {
        action,
        text,
        replacement,
        match_case: flags & FR_MATCHCASE != 0,
        whole_word: flags & FR_WHOLEWORD != 0,
        search_down: flags & FR_DOWN != 0,
    });
    true
}

/// Nonmodal Find/Replace dialog.
///
/// Remains beside the window, invoking handlers for user actions. **Asynchronous**: returns immediately.
///
/// ```no_run
/// # use arqrustui::prelude::*;
/// # fn demo(window: &Window, editor: RichTextBox) -> GuiResult<()> {
/// let dialog = FindDialog::find(window, "", move |event| {
///     if event.action == FindAction::FindNext {
///         let from = editor.selection().1;
///         if editor.find(&event.text, from, event.match_case).is_none() {
///             MessageBox::info("Find", "Text not found.");
///         }
///     }
/// })?;
/// # drop(dialog); Ok(()) }
/// ```
#[derive(Clone)]
pub struct FindDialog {
    state: Rc<FindState>,
}

impl FindDialog {
    fn open(
        owner: &impl Widget,
        text: &str,
        replacement: Option<&str>,
        callback: Rc<dyn Fn(&FindEvent)>,
    ) -> GuiResult<FindDialog> {
        let app = app::require()?;
        let owner_hwnd = owner.core().require_alive()?;
        message_id();
        let mut find = Box::new([0u16; BUFFER_LEN]);
        let mut replace = Box::new([0u16; BUFFER_LEN]);
        copy_to_buffer(text, &mut find[..]);
        copy_to_buffer(replacement.unwrap_or(""), &mut replace[..]);
        let config = Box::new(FINDREPLACEW {
            lStructSize: std::mem::size_of::<FINDREPLACEW>() as u32,
            hwndOwner: owner_hwnd,
            hInstance: std::ptr::null_mut(),
            Flags: FR_DOWN,
            lpstrFindWhat: find.as_mut_ptr(),
            lpstrReplaceWith: if replacement.is_some() { replace.as_mut_ptr() } else { std::ptr::null_mut() },
            wFindWhatLen: BUFFER_LEN as u16,
            wReplaceWithLen: if replacement.is_some() { BUFFER_LEN as u16 } else { 0 },
            lCustData: 0,
            lpfnHook: std::ptr::null(),
            lpTemplateName: std::ptr::null(),
        });
        let state = Rc::new(FindState {
            config: UnsafeCell::new(config),
            _find: find,
            _replace: replace,
            hwnd: Cell::new(std::ptr::null_mut()),
            callback,
        });
        // SAFETY: a estrutura e os buffers vivem no heap enquanto `state` existir, e
        // `state` é mantido em ACTIVE até o diálogo avisar que fechou (FR_DIALOGTERM).
        let hwnd = unsafe {
            let pointer: *mut FINDREPLACEW = &mut **state.config.get();
            if replacement.is_some() {
                ReplaceTextW(pointer)
            } else {
                FindTextW(pointer)
            }
        };
        if hwnd.is_null() {
            // SAFETY: consulta o erro estendido do diálogo comum.
            let code = unsafe { CommDlgExtendedError() };
            return Err(GuiError::from_win32("FindTextW/ReplaceTextW", code));
        }
        state.hwnd.set(hwnd);
        app.modeless.borrow_mut().push(hwnd);
        ACTIVE.with(|active| active.borrow_mut().push(state.clone()));
        Ok(FindDialog { state })
    }

    /// Opens Find with given initial text.
    pub fn find(owner: &impl Widget, text: &str, on_event: impl Fn(&FindEvent) + 'static) -> GuiResult<FindDialog> {
        Self::open(owner, text, None, Rc::new(on_event))
    }

    /// Opens Replace.
    pub fn replace(
        owner: &impl Widget,
        text: &str,
        replacement: &str,
        on_event: impl Fn(&FindEvent) + 'static,
    ) -> GuiResult<FindDialog> {
        Self::open(owner, text, Some(replacement), Rc::new(on_event))
    }

    /// Whether still open.
    pub fn is_open(&self) -> bool {
        let hwnd = self.state.hwnd.get();
        // SAFETY: consulta simples.
        !hwnd.is_null() && unsafe { IsWindow(hwnd) } != 0
    }

    /// Closes the dialog.
    pub fn close(&self) {
        let hwnd = self.state.hwnd.get();
        if !hwnd.is_null() {
            // SAFETY: destrói o diálogo desta thread; ele envia FR_DIALOGTERM ao fechar.
            unsafe { DestroyWindow(hwnd) };
        }
    }
}
