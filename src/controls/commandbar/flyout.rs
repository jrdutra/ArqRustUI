//! [`CommandBar`](super::CommandBar) overflow `»` popup with items that do not fit, including hosted controls.

use super::*;

/// `true` if `WM_GETDLGCODE` (LPARAM = `MSG*` or null) concerns Enter. Shows a hosted control from `»` in a floating
/// window below the button; on deactivation or Esc it returns to the bar.
pub(super) fn open_flyout(state: &Rc<BarState>, id: u32) {
    close_flyout(state);
    let Some(core) = state.core() else { return };
    let hosted = state.items.borrow().iter().find(|i| i.id == id).and_then(|item| match &item.kind {
        Kind::Widget(h) | Kind::Search(h) => Some((h.core.clone(), h.width, matches!(item.kind, Kind::Search(_)))),
        _ => None,
    });
    let Some((child, width, search)) = hosted else { return };
    let anchor = state.screen_rect(OVERFLOW_ID).or_else(|| {
        let mut origin = POINT { x: 0, y: 0 };
        // SAFETY: ponteiro para um POINT local.
        unsafe { ClientToScreen(core.hwnd(), &mut origin) };
        Some(RECT {
            left: origin.x,
            top: origin.y,
            right: origin.x,
            bottom: origin.y + to_pixels(COMMAND_BAR_HEIGHT, core.dpi()),
        })
    });
    let Some(anchor) = anchor else { return };
    let dpi = core.dpi();
    let height =
        if search { menu_strip::text_height(&state.font(), dpi) + 10 } else { child.bounds().height.max(24) + 8 };
    let total = width + 16;
    let root = native::root_of(core.hwnd());
    let Some(owner) = crate::core::widget::lookup(root) else { return };
    let x = (anchor.right - to_pixels(total, dpi)).max(0);
    let window = crate::core::Window::builder()
        .borderless()
        .tool_window()
        .size(total, height)
        .position(x, anchor.bottom)
        .owner(&crate::core::Window { core: owner })
        .build();
    let Ok(window) = window else { return };
    window.set_background(state.style().popup_background);
    // SAFETY: troca o pai de uma janela filha viva da barra para a janela flutuante.
    unsafe { SetParent(child.hwnd(), window.core().hwnd()) };
    child.set_bounds(Rect::new(
        8,
        (height - child.bounds().height.min(height)) / 2,
        width,
        child.bounds().height.min(height),
    ));
    // SAFETY: janela filha viva.
    unsafe { ShowWindow(child.hwnd(), SW_SHOWNA) };
    let weak = Rc::downgrade(state);
    window.core().on(EventKind::Deactivated, move |_| {
        if let Some(state) = weak.upgrade() {
            let _ = crate::core::defer(move || close_flyout(&state));
        }
    });
    let weak = Rc::downgrade(state);
    window.add_shortcut(crate::input::Shortcut::key(Key::ESCAPE), move || {
        if let Some(state) = weak.upgrade() {
            close_flyout(&state);
        }
    });
    *state.flyout.borrow_mut() = Some((window.clone(), id));
    window.show();
    // SAFETY: foco no controle recém-mostrado, vivo nesta thread.
    unsafe { SetFocus(child.hwnd()) };
}

/// Closes the floating window and returns the control to the bar.
pub(super) fn close_flyout(state: &Rc<BarState>) {
    let open = state.flyout.borrow_mut().take();
    let Some((window, id)) = open else { return };
    let child = state.items.borrow().iter().find(|i| i.id == id).and_then(|item| match &item.kind {
        Kind::Widget(h) | Kind::Search(h) => Some(h.core.clone()),
        _ => None,
    });
    if let (Some(child), Some(core)) = (child, state.core()) {
        if child.is_alive() {
            // SAFETY: devolve a janela filha viva à barra antes de destruir a flutuante.
            unsafe { SetParent(child.hwnd(), core.hwnd()) };
        }
    }
    window.core().destroy();
    state.relayout();
}

impl CommandBar {
    /// Allows drag-reordering of bar buttons (accent-colored insertion line). Default: off.
    pub fn set_reorderable(&self, reorderable: bool) {
        self.state.reorderable.set(reorderable);
    }

    /// Called after user reordering/customization, for example to save [`layout`](Self::layout).
    pub fn on_reordered(&self, f: impl Fn() + 'static) {
        self.state.reordered.borrow_mut().push(Rc::new(f));
    }

    /// Moves an item to `index` (0 = start), as with user dragging.
    pub fn move_item(&self, item: &CommandItem, index: usize) -> bool {
        let from = self.state.items.borrow().iter().position(|i| i.id == item.id);
        match from {
            Some(from) => self.state.move_item(item.id, if index > from { index + 1 } else { index }),
            None => false,
        }
    }

    /// Items in current order.
    pub fn items(&self) -> Vec<CommandItem> {
        self.state.items.borrow().iter().map(|i| CommandItem { state: Rc::downgrade(&self.state), id: i.id }).collect()
    }

    /// Shows hosted overflow `item` in the floating window.
    pub fn open_flyout(&self, item: &CommandItem) {
        open_flyout(&self.state, item.id);
    }

    /// Whether a hosted control's floating window is open.
    pub fn is_flyout_open(&self) -> bool {
        self.state.flyout.borrow().is_some()
    }

    /// Closes the floating window (returns the control to the bar).
    pub fn close_flyout(&self) {
        close_flyout(&self.state);
    }
}
