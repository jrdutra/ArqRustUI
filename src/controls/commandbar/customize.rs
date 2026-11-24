//! [`CommandBar`] customization: item order and visibility, saved/restored as text ([`CommandBarLayout`]), and
//! customization dialog.

use super::{BarState, CommandBar, Item, Kind};
use crate::controls::{Button, ListBox};
use crate::core::{GuiError, GuiResult, Widget, Window};
use crate::dialogs::{Dialog, DialogResult};
use crate::i18n::{library_text, LibraryText};
use crate::layout::{GridLayout, LayoutParams, Spacer, StackLayout, Track};
use crate::menus::plain_text;
use crate::utilities::Thickness;
use std::cell::RefCell;
use std::rc::Rc;

/// A position in the bar layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Slot {
    /// Application-created item (sequential creation identifier) and its visibility.
    Item { id: u32, visible: bool },
    /// User-added separator.
    NewSeparator,
}

/// [`CommandBar`] item order and visibility for persistence between runs.
///
/// Items are identified by creation order (1, 2, 3, etc.); recreating the bar with the same `add_*` calls makes the
/// saved text applicable again.
///
/// ```
/// use arqrustui::controls::CommandBarLayout;
///
/// let layout: CommandBarLayout = "3,1,|,-2".parse().unwrap();
/// assert_eq!(layout.to_string(), "3,1,|,-2");
/// assert!("3,x".parse::<CommandBarLayout>().is_err());
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CommandBarLayout {
    slots: Vec<Slot>,
}

impl std::fmt::Display for CommandBarLayout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let parts: Vec<String> = self
            .slots
            .iter()
            .map(|slot| match slot {
                Slot::Item { id, visible: true } => id.to_string(),
                Slot::Item { id, visible: false } => format!("-{id}"),
                Slot::NewSeparator => "|".to_string(),
            })
            .collect();
        f.write_str(&parts.join(","))
    }
}

impl std::str::FromStr for CommandBarLayout {
    type Err = GuiError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let mut slots = Vec::new();
        for part in text.split(',').map(str::trim).filter(|p| !p.is_empty()) {
            let slot = if part == "|" {
                Slot::NewSeparator
            } else if let Some(hidden) = part.strip_prefix('-') {
                Slot::Item { id: parse_id(hidden)?, visible: false }
            } else {
                Slot::Item { id: parse_id(part)?, visible: true }
            };
            slots.push(slot);
        }
        Ok(CommandBarLayout { slots })
    }
}

fn parse_id(text: &str) -> GuiResult<u32> {
    text.parse().map_err(|_| GuiError::InvalidArgument(format!("invalid bar layout: {text:?}")))
}

impl BarState {
    fn current_layout(&self) -> CommandBarLayout {
        let added = self.added_separators.borrow();
        let slots = self
            .items
            .borrow()
            .iter()
            .filter(|item| item.visible || !added.contains(&item.id))
            .map(|item| {
                if added.contains(&item.id) {
                    Slot::NewSeparator
                } else {
                    Slot::Item { id: item.id, visible: item.visible }
                }
            })
            .collect();
        CommandBarLayout { slots }
    }

    /// Reorders items according to the layout; unlisted items remain at the end unchanged.
    fn apply_layout(&self, layout: &CommandBarLayout) {
        {
            let mut items = self.items.borrow_mut();
            let mut added = self.added_separators.borrow_mut();
            // Separadores acrescentados antes são refeitos a partir do layout.
            items.retain(|item| !added.contains(&item.id));
            added.clear();
            let mut pool: Vec<Option<Item>> = std::mem::take(&mut *items).into_iter().map(Some).collect();
            let mut ordered = Vec::with_capacity(pool.len());
            for slot in &layout.slots {
                match *slot {
                    Slot::Item { id, visible } => {
                        if let Some(mut item) =
                            pool.iter_mut().find(|i| i.as_ref().is_some_and(|i| i.id == id)).and_then(Option::take)
                        {
                            item.visible = visible;
                            ordered.push(item);
                        }
                    }
                    Slot::NewSeparator => {
                        let id = self.next_id.get();
                        self.next_id.set(id + 1);
                        added.insert(id);
                        ordered.push(Item { id, kind: Kind::Separator, visible: true, rect: None });
                    }
                }
            }
            ordered.extend(pool.into_iter().flatten());
            *items = ordered;
        }
        self.relayout();
        let listeners = self.reordered.borrow().clone();
        for listener in listeners {
            listener();
        }
    }

    /// Item name in the customization dialog.
    fn display_name(&self, id: u32) -> String {
        let items = self.items.borrow();
        let Some(item) = items.iter().find(|i| i.id == id) else { return String::new() };
        match &item.kind {
            Kind::Button { data, .. } => plain_text(&data.text),
            Kind::Separator => format!("── {}", library_text(LibraryText::Separator)),
            Kind::Spacer => "↔".to_string(),
            Kind::Label(text) => plain_text(text),
            Kind::Widget(h) | Kind::Search(h) => h.name.clone(),
        }
    }
}

/// Editable customization state (without windows; testable).
#[derive(Clone, Debug, PartialEq, Eq)]
struct CustomizeModel {
    slots: Vec<Slot>,
}

impl CustomizeModel {
    fn shown(&self) -> Vec<usize> {
        (0..self.slots.len()).filter(|&i| !matches!(self.slots[i], Slot::Item { visible: false, .. })).collect()
    }

    fn hidden(&self) -> Vec<usize> {
        (0..self.slots.len()).filter(|&i| matches!(self.slots[i], Slot::Item { visible: false, .. })).collect()
    }

    /// Shows hidden item `hidden` (available-list index; `None` = new separator) after position `after` in the bar
    /// list. Returns its new position.
    fn add(&mut self, hidden: Option<usize>, after: Option<usize>) -> Option<usize> {
        let shown = self.shown();
        let insert_at = match after.and_then(|a| shown.get(a)) {
            Some(&index) => index + 1,
            None => self.slots.len(),
        };
        let slot = match hidden {
            None => Slot::NewSeparator,
            Some(h) => {
                let index = *self.hidden().get(h)?;
                let Slot::Item { id, .. } = self.slots.remove(index) else { return None };
                let insert_at = if index < insert_at { insert_at - 1 } else { insert_at };
                self.slots.insert(insert_at, Slot::Item { id, visible: true });
                return self.shown().iter().position(|&i| i == insert_at);
            }
        };
        self.slots.insert(insert_at, slot);
        self.shown().iter().position(|&i| i == insert_at)
    }

    /// Hides item `shown` from the bar list, or discards it if it is a new separator.
    fn remove(&mut self, shown: usize) -> bool {
        let Some(&index) = self.shown().get(shown) else { return false };
        match self.slots[index] {
            Slot::NewSeparator => {
                self.slots.remove(index);
            }
            Slot::Item { id, .. } => self.slots[index] = Slot::Item { id, visible: false },
        }
        true
    }

    /// Swaps bar-list item `shown` with its upper (`up`) or lower neighbor.
    fn shift(&mut self, shown: usize, up: bool) -> Option<usize> {
        let list = self.shown();
        let other = if up { shown.checked_sub(1)? } else { shown + 1 };
        let (&a, &b) = (list.get(shown)?, list.get(other)?);
        self.slots.swap(a, b);
        Some(other)
    }
}

impl CommandBar {
    /// Current item order and visibility for persistence (`to_string()` yields text).
    pub fn layout(&self) -> CommandBarLayout {
        self.state.current_layout()
    }

    /// Applies order/visibility saved with [`layout`](Self::layout).
    pub fn set_layout(&self, layout: &CommandBarLayout) {
        self.state.apply_layout(layout);
    }

    /// Stores the current arrangement as "Restore defaults" (defaults to the arrangement at first customization).
    pub fn set_default_layout(&self) {
        *self.state.defaults.borrow_mut() = Some(self.state.current_layout());
    }

    /// Restores the default arrangement.
    pub fn reset_layout(&self) {
        let defaults = self.state.defaults.borrow().clone();
        if let Some(defaults) = defaults {
            self.state.apply_layout(&defaults);
        }
    }

    /// Opens the modal customization dialog over the bar's window: available commands on the left, bar commands on the
    /// right, with Add, Remove, Up, Down and Restore defaults. Returns `true` if confirmed with OK.
    pub fn customize(&self) -> GuiResult<bool> {
        let root = crate::core::widget::lookup(crate::core::native::root_of(self.core.require_alive()?))
            .ok_or(GuiError::WidgetDestroyed)?;
        let owner = Window { core: root };
        if self.state.defaults.borrow().is_none() {
            self.set_default_layout();
        }
        let model = Rc::new(RefCell::new(CustomizeModel { slots: self.state.current_layout().slots }));
        let dialog = Dialog::new(&owner, &library_text(LibraryText::CustomizeTitle), 560, 340)?;
        let parent = dialog.window();
        let available = ListBox::builder().parent(parent).build()?;
        let current = ListBox::builder().parent(parent).build()?;
        let add = Button::builder().text(&library_text(LibraryText::Add)).parent(parent).build()?;
        let remove = Button::builder().text(&library_text(LibraryText::Remove)).parent(parent).build()?;
        let up = Button::builder().text(&library_text(LibraryText::MoveUp)).parent(parent).build()?;
        let down = Button::builder().text(&library_text(LibraryText::MoveDown)).parent(parent).build()?;
        let reset = Button::builder().text(&library_text(LibraryText::ResetDefaults)).parent(parent).build()?;
        let ok = Button::builder().text(&library_text(LibraryText::Ok)).default(true).parent(parent).build()?;
        let cancel = Button::builder().text(&library_text(LibraryText::Cancel)).cancel(true).parent(parent).build()?;
        let left_title = crate::controls::Label::builder()
            .text(&library_text(LibraryText::AvailableCommands))
            .parent(parent)
            .build()?;
        let right_title = crate::controls::Label::builder()
            .text(&library_text(LibraryText::CurrentCommands))
            .parent(parent)
            .build()?;

        let state = self.state.clone();
        let name_of = move |slot: &Slot| match slot {
            Slot::Item { id, .. } => state.display_name(*id),
            Slot::NewSeparator => format!("── {}", library_text(LibraryText::Separator)),
        };
        let name_of = Rc::new(name_of);
        let refill = {
            let (model, available, current, name_of) =
                (model.clone(), available.clone(), current.clone(), name_of.clone());
            Rc::new(move |select_shown: Option<usize>| {
                let model = model.borrow();
                available.clear();
                available.add_item(&format!("── {}", library_text(LibraryText::Separator)));
                for index in model.hidden() {
                    available.add_item(&name_of(&model.slots[index]));
                }
                current.clear();
                for index in model.shown() {
                    current.add_item(&name_of(&model.slots[index]));
                }
                current.set_selected_index(select_shown);
            })
        };
        refill(None);
        {
            let (model, available, current, refill) =
                (model.clone(), available.clone(), current.clone(), refill.clone());
            add.on_click(move || {
                let Some(choice) = available.selected_index() else { return };
                let position = model.borrow_mut().add(choice.checked_sub(1), current.selected_index());
                refill(position);
            });
        }
        {
            let (model, current, refill) = (model.clone(), current.clone(), refill.clone());
            remove.on_click(move || {
                let Some(choice) = current.selected_index() else { return };
                if model.borrow_mut().remove(choice) {
                    let count = model.borrow().shown().len();
                    refill((count > 0).then(|| choice.min(count - 1)));
                }
            });
        }
        for (button, upward) in [(&up, true), (&down, false)] {
            let (model, current, refill) = (model.clone(), current.clone(), refill.clone());
            button.on_click(move || {
                let Some(choice) = current.selected_index() else { return };
                let moved = model.borrow_mut().shift(choice, upward);
                refill(moved.or(Some(choice)));
            });
        }
        {
            let (model, refill, defaults) = (model.clone(), refill.clone(), self.state.defaults.borrow().clone());
            reset.on_click(move || {
                if let Some(defaults) = &defaults {
                    model.borrow_mut().slots = defaults.slots.clone();
                    refill(None);
                }
            });
        }
        {
            let handle = dialog.clone();
            ok.on_click(move || handle.close_with(DialogResult::Ok));
            let handle = dialog.clone();
            cancel.on_click(move || handle.close_with(DialogResult::Cancel));
        }
        {
            let add = add.clone();
            available.on_item_activated(move |_| add.click());
            let remove = remove.clone();
            current.on_item_activated(move |_| remove.click());
        }

        let middle = StackLayout::vertical().spacing(8);
        middle.add_with(Spacer, LayoutParams::new().weight(1.0));
        middle.add_with(&add, LayoutParams::new().height(28));
        middle.add_with(&remove, LayoutParams::new().height(28));
        middle.add_with(Spacer, LayoutParams::new().weight(1.0));
        let side = StackLayout::vertical().spacing(8);
        side.add_with(&up, LayoutParams::new().height(28));
        side.add_with(&down, LayoutParams::new().height(28));
        side.add_with(Spacer, LayoutParams::new().weight(1.0));
        side.add_with(&reset, LayoutParams::new().height(28));
        let grid = GridLayout::new(
            &[Track::Star(1.0), Track::Fixed(120), Track::Star(1.0), Track::Fixed(130)],
            &[Track::Fixed(20), Track::Star(1.0)],
        )
        .gaps(8, 6);
        grid.add(&left_title, 0, 0);
        grid.add(&right_title, 0, 2);
        grid.add(&available, 1, 0);
        grid.add(middle, 1, 1);
        grid.add(&current, 1, 2);
        grid.add(side, 1, 3);
        let buttons = StackLayout::horizontal().spacing(8);
        buttons.add_with(Spacer, LayoutParams::new().weight(1.0));
        buttons.add_with(&ok, LayoutParams::new().width(96));
        buttons.add_with(&cancel, LayoutParams::new().width(96));
        let root = StackLayout::vertical().spacing(12).padding(Thickness::all(12));
        root.add_with(grid, LayoutParams::new().weight(1.0));
        root.add_with(buttons, LayoutParams::new().height(28));
        parent.set_layout(root);

        if dialog.show_modal()? != DialogResult::Ok {
            return Ok(false);
        }
        let slots = model.borrow().slots.clone();
        self.state.apply_layout(&CommandBarLayout { slots });
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(text: &str) -> CustomizeModel {
        CustomizeModel { slots: text.parse::<CommandBarLayout>().unwrap().slots }
    }

    fn text(model: &CustomizeModel) -> String {
        CommandBarLayout { slots: model.slots.clone() }.to_string()
    }

    #[test]
    fn layout_text_round_trips() {
        let layout: CommandBarLayout = " 1, -2 ,|,3 ".parse().unwrap();
        assert_eq!(layout.to_string(), "1,-2,|,3");
        assert_eq!("".parse::<CommandBarLayout>().unwrap(), CommandBarLayout::default());
        assert!("1,-x".parse::<CommandBarLayout>().is_err());
    }

    #[test]
    fn customize_model_adds_removes_and_moves() {
        let mut m = model("1,-2,3,-4");
        assert_eq!((m.shown(), m.hidden()), (vec![0, 2], vec![1, 3]));
        // Mostra o 4 (2º disponível) depois do primeiro da barra.
        assert_eq!(m.add(Some(1), Some(0)), Some(1));
        assert_eq!(text(&m), "1,4,-2,3");
        // Novo separador no fim.
        assert_eq!(m.add(None, None), Some(3));
        assert_eq!(text(&m), "1,4,-2,3,|");
        assert_eq!(m.shift(3, true), Some(2));
        assert_eq!(text(&m), "1,4,-2,|,3");
        assert_eq!(m.shift(0, true), None);
        assert!(m.remove(2), "separador novo é descartado");
        assert_eq!(text(&m), "1,4,-2,3");
        assert!(m.remove(0));
        assert_eq!(text(&m), "-1,4,-2,3");
        assert!(!m.remove(9));
    }
}
