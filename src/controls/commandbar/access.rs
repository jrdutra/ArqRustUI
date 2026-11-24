//! [`CommandBar`](super::CommandBar) keyboard and accessibility: focus order, arrows and Tab, UI Automation tree and
//! focused button announcements.

use super::*;

/// Announces the keyboard-focused button to screen readers.
pub(super) fn announce_focus(state: &BarState) {
    let Some(core) = state.core() else { return };
    let host = core.accessible_tree.borrow().clone();
    if let (Some(host), Some(id)) = (host, state.focus.get()) {
        host.focus_changed(id);
    }
}

/// UIA bar elements: drawn buttons, separators and labels (hosted controls such as search are windows and appear
/// automatically).
pub(super) struct BarTree {
    pub(super) state: Weak<BarState>,
    pub(super) hwnd: HWND,
}

impl VirtualTree for BarTree {
    fn hwnd(&self) -> HWND {
        self.hwnd
    }

    fn root_role(&self) -> Option<i32> {
        Some(role::TOOL_BAR)
    }

    fn root_name(&self) -> Option<String> {
        Some(crate::i18n::library_text(crate::i18n::LibraryText::CommandBarName))
    }

    fn nodes(&self) -> Vec<Node> {
        let Some(state) = self.state.upgrade() else { return Vec::new() };
        let has_focus = focused() == self.hwnd;
        let focus = state.focus.get();
        let open = state.open.get();
        let mut nodes = Vec::new();
        for item in state.items.borrow().iter().filter(|i| i.visible && i.rect.is_some()) {
            let mut node = match &item.kind {
                Kind::Button { data, .. } => {
                    let kind = if matches!(data.mode, Mode::Split(_)) { role::SPLIT_BUTTON } else { role::BUTTON };
                    let mut node = Node::new(item.id, 0, kind, &plain_text(&data.text));
                    node.help = data.tooltip.clone().unwrap_or_default();
                    node.enabled = data.enabled;
                    node.focusable = data.enabled;
                    node.focused = has_focus && focus == Some(item.id);
                    match data.mode {
                        Mode::Normal => node.invoke = true,
                        Mode::Toggle => node.toggle = Some(data.checked),
                        Mode::Dropdown(_) => node.expanded = Some(open == Some(item.id)),
                        Mode::Split(_) => {
                            node.invoke = true;
                            node.expanded = Some(open == Some(item.id));
                        }
                    }
                    node
                }
                Kind::Separator => Node::new(item.id, 0, role::SEPARATOR, ""),
                Kind::Label(text) => Node::new(item.id, 0, role::TEXT, &plain_text(text)),
                Kind::Spacer | Kind::Widget(_) | Kind::Search(_) => continue,
            };
            node.bounds = state.screen_rect(item.id).unwrap_or_default();
            nodes.push(node);
        }
        if state.overflow.get().is_some() {
            let mut node = Node::new(
                OVERFLOW_ID,
                0,
                role::BUTTON,
                &crate::i18n::library_text(crate::i18n::LibraryText::MoreOptions),
            );
            node.focusable = true;
            node.focused = has_focus && focus == Some(OVERFLOW_ID);
            node.expanded = Some(open == Some(OVERFLOW_ID));
            node.bounds = state.screen_rect(OVERFLOW_ID).unwrap_or_default();
            nodes.push(node);
        }
        nodes
    }

    fn act(&self, id: u32, action: Action) -> bool {
        let Some(state) = self.state.upgrade() else { return false };
        let is_button = id == OVERFLOW_ID || state.button(id).is_some();
        if !is_button {
            return false;
        }
        match action {
            Action::Invoke | Action::Toggle => activate(&state, id),
            Action::Expand => open_menu(&state, id, true),
            Action::Collapse => popup::cancel_for(state.hwnd()),
            Action::Focus => {
                // SAFETY: foco para a própria barra, viva nesta thread.
                unsafe { SetFocus(self.hwnd) };
                state.focus.set(Some(id));
                state.invalidate();
                announce_focus(&state);
            }
            _ => return false,
        }
        true
    }
}

/// Items receiving keyboard focus within the bar, in order.
pub(super) fn focus_order(state: &BarState) -> Vec<u32> {
    let mut ids: Vec<u32> = state
        .items
        .borrow()
        .iter()
        .filter(|i| i.rect.is_some() && matches!(&i.kind, Kind::Button { data, .. } if data.enabled))
        .map(|i| i.id)
        .collect();
    if state.overflow.get().is_some() {
        ids.push(OVERFLOW_ID);
    }
    ids
}

pub(super) fn on_key(state: &Rc<BarState>, key: Key) {
    let order = focus_order(state);
    if order.is_empty() {
        return;
    }
    let current = state.focus.get().and_then(|f| order.iter().position(|&id| id == f));
    let move_to = |index: usize| {
        state.focus.set(Some(order[index]));
        state.invalidate();
        announce_focus(state);
    };
    match key.0 as i32 {
        VK_LEFT => move_to(current.map(|c| (c + order.len() - 1) % order.len()).unwrap_or(0)),
        VK_RIGHT => move_to(current.map(|c| (c + 1) % order.len()).unwrap_or(0)),
        VK_HOME => move_to(0),
        VK_END => move_to(order.len() - 1),
        VK_RETURN | VK_SPACE => {
            if let Some(id) = state.focus.get() {
                let dropdown = matches!(state.button(id).map(|(d, _)| d.mode), Some(Mode::Dropdown(_)));
                if id == OVERFLOW_ID || dropdown {
                    open_menu(state, id, true);
                } else {
                    activate(state, id);
                }
            }
        }
        VK_DOWN | VK_F4 => {
            if let Some(id) = state.focus.get() {
                if id == OVERFLOW_ID || state.button(id).is_some_and(|(d, _)| d.menu_of().is_some()) {
                    open_menu(state, id, true);
                }
            }
        }
        _ => {}
    }
}
