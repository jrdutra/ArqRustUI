//! Open drawn menu UI Automation tree.

use super::*;

/// List UIA elements: one menu item per entry.
pub(super) struct PopupTree {
    pub(super) hwnd: HWND,
}

impl PopupTree {
    /// List and its active-menu-mode position.
    fn locate(&self) -> Option<(Rc<Tracker>, usize, Rc<Level>)> {
        let tracker = current()?;
        let index = tracker.levels.borrow().iter().position(|l| l.hwnd.get() == self.hwnd)?;
        let level = tracker.level(index)?;
        Some((tracker, index, level))
    }
}

impl VirtualTree for PopupTree {
    fn hwnd(&self) -> HWND {
        self.hwnd
    }

    fn root_role(&self) -> Option<i32> {
        Some(role::MENU)
    }

    fn root_name(&self) -> Option<String> {
        let (tracker, index, level) = self.locate()?;
        if index > 0 {
            let parent = tracker.level(index - 1)?;
            return Some(plain_text(&parent.entries.get(level.child_of?)?.text));
        }
        let strip = tracker.strip.clone()?;
        let slot = strip.open_slot()?;
        strip.slot_name(slot)
    }

    fn nodes(&self) -> Vec<Node> {
        let Some(level) = level_of(self.hwnd) else { return Vec::new() };
        let located = self.locate();
        let deepest = located.as_ref().is_some_and(|(tracker, index, _)| tracker.depth() == index + 1);
        let open_child =
            located.as_ref().and_then(|(tracker, index, _)| tracker.level(index + 1)).and_then(|l| l.child_of);
        let view = level.screen.get();
        level
            .entries
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let id = i as u32 + 1;
                let mut row = level.row_screen_rect(i);
                // Linhas roladas para fora da lista ficam "fora da tela".
                if row.bottom <= view.top || row.top >= view.bottom {
                    row = RECT::default();
                } else {
                    row.top = row.top.max(view.top);
                    row.bottom = row.bottom.min(view.bottom);
                }
                if entry.is_separator() {
                    let mut node = Node::new(id, 0, role::SEPARATOR, "");
                    node.bounds = row;
                    return node;
                }
                let mut node = Node::new(id, 0, role::MENU_ITEM, &plain_text(&entry.text));
                node.enabled = entry.enabled;
                node.focusable = true;
                node.focused = deepest && level.hot.get() == Some(i);
                node.bounds = row;
                node.accelerator = entry.shortcut.clone();
                node.access_key = mnemonic_of(&entry.text).map(|c| c.to_uppercase().to_string()).unwrap_or_default();
                match &entry.kind {
                    EntryKind::Submenu(_) => node.expanded = Some(open_child == Some(i)),
                    _ => {
                        node.invoke = true;
                        if entry.checked || entry.radio {
                            node.toggle = Some(entry.checked);
                        }
                    }
                }
                node
            })
            .collect()
    }

    fn act(&self, id: u32, action: Action) -> bool {
        let Some((tracker, index, level)) = self.locate() else { return false };
        let item = id as usize - 1;
        let Some(entry) = level.entries.get(item) else { return false };
        let submenu = matches!(entry.kind, EntryKind::Submenu(_));
        match action {
            Action::Invoke | Action::Toggle if !submenu => tracker.activate(index, item),
            Action::Expand if submenu => tracker.open_child(index, item, true),
            Action::Collapse if submenu => tracker.close_from(index + 1),
            Action::Focus => level.set_hot(Some(item)),
            _ => return false,
        }
        true
    }
}
