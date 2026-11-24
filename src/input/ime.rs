//! IME composition text/position and custom candidate list ([`ImeCandidates`], [`ImeCandidateWindow`]).

mod candidates;

pub use candidates::ImeCandidateWindow;

use crate::core::widget::{Widget, WidgetCore};
use crate::core::{Event, EventKind, HandlerId};
use crate::ffi::imm32::*;
use crate::ffi::user32::{WM_IME_NOTIFY, WM_IME_SETCONTEXT};
use crate::ffi::{HWND, POINT, RECT};
use crate::utilities::{to_logical, to_pixels, Point};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};

/// IME candidates, e.g. possible characters for pinyin/kana input.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct ImeCandidates {
    /// All candidates.
    pub items: Vec<String>,
    /// Selected candidate index in `items`.
    pub selection: usize,
    /// First candidate index on current page.
    pub page_start: usize,
    /// Candidates per page, at least 1.
    pub page_size: usize,
}

impl ImeCandidates {
    /// List with `items`, first selected and pages of 9 (Windows IME default). Since `#[non_exhaustive]`, use this or
    /// `default()` outside the library, e.g. for [`ImeCandidateWindow::show`](crate::input::ImeCandidateWindow::show).
    ///
    /// ```
    /// use arqrustui::prelude::*;
    /// let list = ImeCandidates::new(["你", "尼", "泥"]).with_selection(1);
    /// assert_eq!(list.page(), ["你", "尼", "泥"]);
    /// assert_eq!(list.selection, 1);
    /// ```
    pub fn new(items: impl IntoIterator<Item = impl Into<String>>) -> ImeCandidates {
        ImeCandidates { items: items.into_iter().map(Into::into).collect(), selection: 0, page_start: 0, page_size: 9 }
    }

    /// Changes selected candidate index in `items`.
    pub fn with_selection(mut self, selection: usize) -> ImeCandidates {
        self.selection = selection;
        self
    }

    /// Changes first candidate/page size (minimum 1).
    pub fn with_page(mut self, start: usize, size: usize) -> ImeCandidates {
        self.page_start = start;
        self.page_size = size.max(1);
        self
    }

    /// Current page candidates.
    pub fn page(&self) -> &[String] {
        let start = self.page_start.min(self.items.len());
        let end = (start + self.page_size.max(1)).min(self.items.len());
        &self.items[start..end]
    }

    /// Zero-based page and page count.
    pub fn page_position(&self) -> (usize, usize) {
        let size = self.page_size.max(1);
        (self.page_start / size, self.items.len().div_ceil(size).max(1))
    }

    /// Reads `CANDIDATELIST`: header, offsets, NUL-terminated UTF-16 strings.
    pub(crate) fn parse(bytes: &[u8]) -> Option<ImeCandidates> {
        let word = |at: usize| -> Option<u32> {
            let raw = bytes.get(at..at + 4)?;
            Some(u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]))
        };
        let size = (word(0)? as usize).min(bytes.len());
        let count = word(8)? as usize;
        if count > size / 4 {
            return None;
        }
        let mut items = Vec::with_capacity(count);
        for index in 0..count {
            let offset = word(24 + index * 4)? as usize;
            let mut units = Vec::new();
            let mut at = offset;
            while at + 1 < size {
                let unit = u16::from_le_bytes([bytes[at], bytes[at + 1]]);
                if unit == 0 {
                    break;
                }
                units.push(unit);
                at += 2;
            }
            items.push(String::from_utf16_lossy(&units));
        }
        let selection = (word(12)? as usize).min(count.saturating_sub(1));
        let page_size = (word(20)? as usize).max(1);
        let page_start = (word(16)? as usize).min(count.saturating_sub(1));
        Some(ImeCandidates { items, selection, page_start, page_size })
    }
}

type CandidatesFn = Rc<dyn Fn(Option<&ImeCandidates>)>;

/// Widget custom IME state living in its message filter.
struct ImeState {
    hide_candidates: Cell<bool>,
    hide_composition: Cell<bool>,
    current: RefCell<Option<ImeCandidates>>,
    callbacks: RefCell<Vec<CandidatesFn>>,
    position: Cell<Option<Point>>,
}

thread_local! {
    static STATES: RefCell<HashMap<usize, Weak<ImeState>>> = RefCell::new(HashMap::new());
}

fn state_for(core: &Rc<WidgetCore>) -> Rc<ImeState> {
    let key = core.hwnd() as usize;
    if let Some(state) = STATES.with(|s| s.borrow().get(&key).and_then(Weak::upgrade)) {
        return state;
    }
    let state = Rc::new(ImeState {
        hide_candidates: Cell::new(false),
        hide_composition: Cell::new(false),
        current: RefCell::new(None),
        callbacks: RefCell::new(Vec::new()),
        position: Cell::new(None),
    });
    STATES.with(|s| {
        let mut map = s.borrow_mut();
        map.retain(|_, weak| weak.strong_count() > 0);
        map.insert(key, Rc::downgrade(&state));
    });
    let filter_state = state.clone();
    let target = Rc::downgrade(core);
    core.add_message_filter(move |raw| {
        let core = target.upgrade()?;
        let state = &filter_state;
        match raw.message {
            WM_IME_SETCONTEXT if raw.wparam != 0 => {
                let mut flags = raw.lparam as u32;
                if state.hide_candidates.get() {
                    flags &= !ISC_SHOWUIALLCANDIDATEWINDOW;
                }
                if state.hide_composition.get() {
                    flags &= !ISC_SHOWUICOMPOSITIONWINDOW;
                }
                Some(core.default_proc(raw.hwnd as HWND, raw.message, raw.wparam, flags as i32 as isize))
            }
            WM_IME_NOTIFY if state.hide_candidates.get() => match raw.wparam {
                IMN_OPENCANDIDATE | IMN_CHANGECANDIDATE => {
                    let list = read_candidates(raw.hwnd as HWND, raw.lparam as u32);
                    deliver(state, list);
                    Some(0)
                }
                IMN_CLOSECANDIDATE => {
                    deliver(state, None);
                    Some(0)
                }
                _ => None,
            },
            _ => None,
        }
    });
    state
}

fn deliver(state: &ImeState, list: Option<ImeCandidates>) {
    *state.current.borrow_mut() = list.clone();
    // Copia os callbacks: um deles pode registrar outro ou consultar o estado.
    let callbacks = state.callbacks.borrow().clone();
    for callback in callbacks {
        callback(list.as_ref());
    }
}

/// Reads first candidate list in `lists` bitmask (bit 0 = list 0).
fn read_candidates(hwnd: HWND, lists: u32) -> Option<ImeCandidates> {
    let index = if lists == 0 { 0 } else { lists.trailing_zeros() };
    // SAFETY: contexto obtido e liberado no mesmo bloco; o buffer (alinhado a 4 bytes)
    // tem o tamanho que a própria API pediu.
    unsafe {
        let context = ImmGetContext(hwnd);
        if context.is_null() {
            return None;
        }
        let size = ImmGetCandidateListW(context, index, std::ptr::null_mut(), 0);
        let list = if size as usize >= std::mem::size_of::<CANDIDATELIST>() {
            let mut buffer = vec![0u32; (size as usize).div_ceil(4)];
            let written = ImmGetCandidateListW(context, index, buffer.as_mut_ptr() as *mut CANDIDATELIST, size);
            let bytes = std::slice::from_raw_parts(buffer.as_ptr() as *const u8, (written as usize).min(size as usize));
            ImeCandidates::parse(bytes)
        } else {
            None
        };
        ImmReleaseContext(hwnd, context);
        list
    }
}

fn notify(hwnd: HWND, action: u32, index: u32, value: u32) -> bool {
    // SAFETY: contexto obtido e liberado no mesmo bloco.
    unsafe {
        let context = ImmGetContext(hwnd);
        if context.is_null() {
            return false;
        }
        let ok = ImmNotifyIME(context, action, index, value) != 0;
        ImmReleaseContext(hwnd, context);
        ok
    }
}

/// Reads a composition string from window IME.
pub(crate) fn composition_string(hwnd: HWND, kind: u32) -> Option<String> {
    // SAFETY: o contexto é obtido e liberado no mesmo bloco; o buffer tem o tamanho
    // em bytes informado pela própria API.
    unsafe {
        let context = ImmGetContext(hwnd);
        if context.is_null() {
            return None;
        }
        let bytes = ImmGetCompositionStringW(context, kind, std::ptr::null_mut(), 0);
        let text = if bytes > 0 {
            let mut buffer = vec![0u16; bytes as usize / 2];
            ImmGetCompositionStringW(context, kind, buffer.as_mut_ptr() as *mut _, bytes as u32);
            Some(String::from_utf16_lossy(&buffer))
        } else {
            Some(String::new())
        };
        ImmReleaseContext(hwnd, context);
        text
    }
}

/// Widget IME control, useful for custom text editors.
pub struct Ime;

impl Ime {
    /// Enables/disables widget IME, e.g. for password fields.
    pub fn set_enabled(widget: &impl Widget, enabled: bool) {
        let hwnd = widget.core().hwnd();
        // SAFETY: associa o contexto padrão ou nenhum contexto à janela; HWND inválido falha sem efeito.
        unsafe {
            if enabled {
                ImmAssociateContextEx(hwnd, std::ptr::null_mut(), IACE_DEFAULT);
            } else {
                ImmAssociateContextEx(hwnd, std::ptr::null_mut(), 0);
            }
        }
    }

    /// Whether widget IME composition mode is open.
    pub fn is_open(widget: &impl Widget) -> bool {
        let hwnd = widget.core().hwnd();
        // SAFETY: contexto obtido e liberado no mesmo bloco.
        unsafe {
            let context = ImmGetContext(hwnd);
            if context.is_null() {
                return false;
            }
            let open = ImmGetOpenStatus(context) != 0;
            ImmReleaseContext(hwnd, context);
            open
        }
    }

    /// Positions IME composition window in logical widget coordinates, usually beside a custom editor's caret.
    pub fn set_composition_position(widget: &impl Widget, at: Point) {
        state_for(widget.core()).position.set(Some(at));
        let hwnd = widget.core().hwnd();
        let dpi = widget.dpi();
        let form = COMPOSITIONFORM {
            dwStyle: CFS_POINT,
            ptCurrentPos: POINT { x: to_pixels(at.x, dpi), y: to_pixels(at.y, dpi) },
            rcArea: RECT::default(),
        };
        // SAFETY: contexto obtido e liberado no mesmo bloco; estrutura local completa.
        unsafe {
            let context = ImmGetContext(hwnd);
            if !context.is_null() {
                ImmSetCompositionWindow(context, &form);
                ImmReleaseContext(hwnd, context);
            }
        }
    }

    /// Receives uncommitted composition text during IME input.
    ///
    /// Committed text arrives through normal `on_char`.
    pub fn on_composition_changed(widget: &impl Widget, f: impl Fn(&str) + 'static) -> HandlerId {
        widget.core().on(EventKind::ImeComposition, move |e| {
            if let Event::ImeComposition(text) = e {
                f(text)
            }
        })
    }

    /// Former name of [`on_composition_changed`](Self::on_composition_changed), retained for existing code.
    #[deprecated(
        since = "0.1.0",
        note = "use `on_composition_changed`: events follow the naming convention in docs/EVENTS.md"
    )]
    pub fn on_composition(widget: &impl Widget, f: impl Fn(&str) + 'static) -> HandlerId {
        Self::on_composition_changed(widget, f)
    }

    /// Enables custom candidate list: hides IME list and delivers candidates through
    /// [`on_candidates_changed`](Self::on_candidates_changed).
    pub fn set_custom_candidates(widget: &impl Widget, custom: bool) {
        state_for(widget.core()).hide_candidates.set(custom);
    }

    /// Whether custom candidate list is enabled.
    pub fn has_custom_candidates(widget: &impl Widget) -> bool {
        state_for(widget.core()).hide_candidates.get()
    }

    /// Hides IME composition window (`true`); application draws
    /// [`on_composition_changed`](Self::on_composition_changed) text in its editor.
    pub fn set_custom_composition(widget: &impl Widget, custom: bool) {
        state_for(widget.core()).hide_composition.set(custom);
    }

    /// Receives candidate list on opening/change (`Some`) and closing (`None`). Enables custom candidates.
    pub fn on_candidates_changed(widget: &impl Widget, f: impl Fn(Option<&ImeCandidates>) + 'static) {
        let state = state_for(widget.core());
        state.hide_candidates.set(true);
        state.callbacks.borrow_mut().push(Rc::new(f));
    }

    /// Former name of [`on_candidates_changed`](Self::on_candidates_changed), retained for existing code.
    #[deprecated(
        since = "0.1.0",
        note = "use `on_candidates_changed`: events follow the naming convention in docs/EVENTS.md"
    )]
    pub fn on_candidates(widget: &impl Widget, f: impl Fn(Option<&ImeCandidates>) + 'static) {
        Self::on_candidates_changed(widget, f)
    }

    /// Currently open candidates with custom list enabled.
    pub fn candidates(widget: &impl Widget) -> Option<ImeCandidates> {
        state_for(widget.core()).current.borrow().clone()
    }

    /// Selects `index` in [`ImeCandidates::items`]; returns `false` on IME rejection.
    pub fn select_candidate(widget: &impl Widget, index: usize) -> bool {
        notify(widget.core().hwnd(), NI_SELECTCANDIDATESTR, 0, index as u32)
    }

    /// Shows page starting at candidate `start`.
    pub fn set_candidate_page(widget: &impl Widget, start: usize) -> bool {
        notify(widget.core().hwnd(), NI_SETCANDIDATE_PAGESTART, 0, start as u32)
    }

    /// Commits composition like IME Enter.
    pub fn complete(widget: &impl Widget) -> bool {
        notify(widget.core().hwnd(), NI_COMPOSITIONSTR, CPS_COMPLETE, 0)
    }

    /// Discards composition.
    pub fn cancel(widget: &impl Widget) -> bool {
        notify(widget.core().hwnd(), NI_COMPOSITIONSTR, CPS_CANCEL, 0)
    }

    /// Delivers `candidates` to [`on_candidates_changed`](Self::on_candidates_changed) as IME input (`None` = closed),
    /// for IME-free tests/custom suggestions.
    pub fn simulate_candidates(widget: &impl Widget, candidates: Option<ImeCandidates>) {
        deliver(&state_for(widget.core()), candidates);
    }

    /// Uses themed library candidate window: hides IME's window, shows near text and selects/commits on click.
    pub fn use_candidate_window(widget: &impl Widget) -> crate::core::GuiResult<ImeCandidateWindow> {
        let window = ImeCandidateWindow::new(widget)?;
        let core = Rc::downgrade(widget.core());
        let target = window.clone();
        Ime::on_candidates_changed(widget, move |list| {
            let Some(core) = core.upgrade() else { return };
            match list {
                Some(list) => target.show(list, anchor(&core)),
                None => target.hide(),
            }
        });
        let core = Rc::downgrade(widget.core());
        window.on_pick(move |index| {
            if let Some(core) = core.upgrade() {
                notify(core.hwnd(), NI_SELECTCANDIDATESTR, 0, index as u32);
                notify(core.hwnd(), NI_COMPOSITIONSTR, CPS_COMPLETE, 0);
            }
        });
        Ok(window)
    }
}

/// Logical widget point near the list: application composition position, caret or bottom-left.
fn anchor(core: &Rc<WidgetCore>) -> Point {
    let line = 20;
    if let Some(at) =
        STATES.with(|s| s.borrow().get(&(core.hwnd() as usize)).and_then(Weak::upgrade)).and_then(|s| s.position.get())
    {
        return Point::new(at.x, at.y + line);
    }
    let mut caret = POINT::default();
    // SAFETY: consulta simples do cursor de texto desta thread.
    let has_caret = unsafe { crate::ffi::user32::GetCaretPos(&mut caret) } != 0;
    // SAFETY: consulta simples do foco desta thread.
    let focused = unsafe { crate::ffi::user32::GetFocus() } == core.hwnd();
    let dpi = core.dpi();
    if has_caret && focused {
        return Point::new(to_logical(caret.x, dpi), to_logical(caret.y, dpi) + line);
    }
    Point::new(0, core.client_size().height)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds IMM-style `CANDIDATELIST`.
    fn list_bytes(items: &[&str], selection: u32, page_start: u32, page_size: u32) -> Vec<u8> {
        let header = 24 + 4 * items.len();
        let mut strings = Vec::new();
        let mut offsets = Vec::new();
        for item in items {
            offsets.push((header + strings.len()) as u32);
            for unit in item.encode_utf16().chain(std::iter::once(0)) {
                strings.extend_from_slice(&unit.to_le_bytes());
            }
        }
        let size = (header + strings.len()) as u32;
        let mut bytes = Vec::new();
        for word in [size, 1, items.len() as u32, selection, page_start, page_size] {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        for offset in offsets {
            bytes.extend_from_slice(&offset.to_le_bytes());
        }
        bytes.extend_from_slice(&strings);
        bytes
    }

    #[test]
    fn parses_candidate_lists() {
        let bytes = list_bytes(&["你", "尼", "泥", "𠀀"], 2, 0, 3);
        let list = ImeCandidates::parse(&bytes).unwrap();
        assert_eq!(list.items, ["你", "尼", "泥", "𠀀"]);
        assert_eq!((list.selection, list.page_start, list.page_size), (2, 0, 3));
        assert_eq!(list.page(), ["你", "尼", "泥"]);
        assert_eq!(list.page_position(), (0, 2));
        let next = ImeCandidates { page_start: 3, ..list.clone() };
        assert_eq!(next.page(), ["𠀀"], "caractere fora do plano básico (par substituto)");
        assert_eq!(next.page_position(), (1, 2));
        assert!(ImeCandidates::parse(&bytes[..10]).is_none(), "cabeçalho truncado");
        let mut lying = bytes.clone();
        lying[8..12].copy_from_slice(&9999u32.to_le_bytes());
        assert!(ImeCandidates::parse(&lying).is_none(), "contagem maior que o buffer");
        let empty = ImeCandidates::parse(&list_bytes(&[], 0, 0, 0)).unwrap();
        assert!(empty.items.is_empty() && empty.page().is_empty());
        assert_eq!(empty.page_size, 1);
    }
}
