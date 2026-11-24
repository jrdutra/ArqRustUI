//! Custom modal/nonmodal dialogs with results.

use crate::core::native;
use crate::core::widget::Widget;
use crate::core::{EventKind, GuiResult, HandlerId, Window, WindowBuilder};
use crate::ffi::user32::*;
use std::cell::Cell;
use std::rc::Rc;

/// Custom dialog closing result.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum DialogResult {
    /// Closed without explicit result (window Close or Esc).
    #[default]
    None,
    /// Confirmed.
    Ok,
    /// Canceled.
    Cancel,
    /// Yes.
    Yes,
    /// No.
    No,
    /// Application-defined code.
    Custom(i32),
}

/// Custom dialog: owned window with dialog frame and result.
///
/// Build content using [`window`](Self::window) as control parent.
///
/// ```no_run
/// # use arqrustui::prelude::*;
/// # fn demo(main: &Window) -> GuiResult<()> {
/// let dialog = Dialog::new(main, "Rename", 320, 120)?;
/// let name = TextBox::builder().bounds(12, 12, 296, 24).parent(dialog.window()).build()?;
/// let ok = Button::builder().text("OK").bounds(212, 80, 96, 28).default(true).parent(dialog.window()).build()?;
/// let handle = dialog.clone();
/// ok.on_click(move || handle.close_with(DialogResult::Ok));
/// if dialog.show_modal()? == DialogResult::Ok {
///     println!("new name: {}", name.text());
/// }
/// # Ok(()) }
/// ```
#[derive(Clone, Debug)]
pub struct Dialog {
    window: Window,
    result: Rc<Cell<DialogResult>>,
}

impl Dialog {
    /// Creates a dialog centered over `owner` with given logical client size.
    pub fn new(owner: &Window, title: &str, width: i32, height: i32) -> GuiResult<Dialog> {
        Self::from_builder(
            Window::builder().title(title).size(width, height).dialog_frame().close_on_escape(true).owner(owner),
            Some(owner),
        )
    }

    /// Creates from a configured [`WindowBuilder`] for custom frame/resizing/owner.
    pub fn from_builder(builder: WindowBuilder, center_over: Option<&Window>) -> GuiResult<Dialog> {
        let window = builder.build()?;
        if let Some(owner) = center_over {
            let outer = native::window_rect(owner.core().hwnd());
            let own = native::window_rect(window.core().hwnd());
            let x = outer.left + ((outer.right - outer.left) - (own.right - own.left)) / 2;
            let y = outer.top + ((outer.bottom - outer.top) - (own.bottom - own.top)) / 2;
            // SAFETY: reposiciona a janela recém-criada desta thread.
            unsafe {
                SetWindowPos(
                    window.core().hwnd(),
                    std::ptr::null_mut(),
                    x,
                    y,
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                )
            };
        }
        Ok(Dialog { window, result: Rc::new(Cell::new(DialogResult::None)) })
    }

    /// Dialog window for parenting controls.
    pub fn window(&self) -> &Window {
        &self.window
    }

    /// Sets result and closes.
    pub fn close_with(&self, result: DialogResult) {
        self.result.set(result);
        self.window.destroy();
    }

    /// Current result, useful for nonmodal dialogs.
    pub fn result(&self) -> DialogResult {
        self.result.get()
    }

    /// Shows modally and returns result on closing.
    pub fn show_modal(&self) -> GuiResult<DialogResult> {
        self.window.show_modal()?;
        Ok(self.result.get())
    }

    /// Shows nonmodally/asynchronously: returns immediately; `on_closed` receives the closing result.
    pub fn show_modeless(&self, on_closed: impl Fn(DialogResult) + 'static) -> HandlerId {
        let result = self.result.clone();
        let id = self.window.core().on(EventKind::Destroyed, move |_| on_closed(result.get()));
        self.window.show();
        id
    }
}
