//! List box.

use super::common::{builder_common, widget_type, ControlOptions};
use super::icon_support::{self, ListMessages};
use crate::core::handles::IconSource;
use crate::core::native;
use crate::core::widget::Reflected;
use crate::core::{Event, EventKind, GuiResult, HandlerId};
use crate::ffi::user32::*;
use crate::ffi::POINT;
use crate::input::drag::{self, ListKind};
use crate::system::ClipboardContent;
use crate::utilities::{to_pixels, Point};
use std::rc::Rc;

widget_type!(
    /// Text item list with single/multiple selection.
    ListBox
);

/// Builder for [`ListBox`].
#[derive(Clone, Debug)]
pub struct ListBoxBuilder {
    opts: ControlOptions,
    multi_select: bool,
    sorted: bool,
    items: Vec<String>,
    icons: bool,
}

impl ListBoxBuilder {
    builder_common!();

    /// Allows multiple selection with Ctrl and Shift.
    pub fn multi_select(mut self, multi: bool) -> Self {
        self.multi_select = multi;
        self
    }

    /// Keeps items alphabetically sorted.
    pub fn sorted(mut self, sorted: bool) -> Self {
        self.sorted = sorted;
        self
    }

    /// Initial items.
    pub fn items(mut self, items: &[&str]) -> Self {
        self.items = items.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Icon items: enables library drawing and [`ListBox::add_item_with_icon`] / [`ListBox::set_item_icon`].
    pub fn with_icons(mut self, icons: bool) -> Self {
        self.icons = icons;
        self
    }

    /// Creates the list.
    pub fn build(self) -> GuiResult<ListBox> {
        let mut style = LBS_NOTIFY | LBS_NOINTEGRALHEIGHT | WS_VSCROLL;
        if self.icons {
            style |= LBS_OWNERDRAWFIXED | LBS_HASSTRINGS;
        }
        if self.multi_select {
            style |= LBS_EXTENDEDSEL;
        }
        if self.sorted {
            style |= LBS_SORT;
        }
        let core = self.opts.create("ListBox", "LISTBOX", style, WS_EX_CLIENTEDGE, true, None)?;
        core.set_reflect(|core, reflected| match reflected {
            Reflected::Command { code, .. } if *code == LBN_SELCHANGE => {
                core.emit(&Event::SelectionChanged);
                Some(0)
            }
            Reflected::Command { code, .. } if *code == LBN_DBLCLK => {
                let index = native::send(core.hwnd(), LB_GETCURSEL, 0, 0);
                core.emit(&Event::DoubleClick);
                if index >= 0 {
                    core.emit(&Event::ItemActivated(index as usize));
                }
                Some(0)
            }
            Reflected::DrawItem { lparam } => {
                // SAFETY: em WM_DRAWITEM o LPARAM aponta para um DRAWITEMSTRUCT válido
                // durante a mensagem.
                let draw = unsafe { &*(*lparam as *const DRAWITEMSTRUCT) };
                icon_support::paint_list_item(core, draw, MESSAGES);
                Some(1)
            }
            _ => None,
        });
        if self.icons {
            icon_support::fit_item_height(&core, MESSAGES);
        }
        let list = ListBox { core };
        for item in &self.items {
            list.add_item(item);
        }
        Ok(list)
    }
}

impl ListBox {
    /// Starts building (default size 160×120).
    pub fn builder() -> ListBoxBuilder {
        ListBoxBuilder {
            opts: ControlOptions::new(160, 120),
            multi_select: false,
            sorted: false,
            items: Vec::new(),
            icons: false,
        }
    }

    /// Adds an item with an icon and returns its resulting index.
    ///
    /// Requires a list created with [`with_icons(true)`](ListBoxBuilder::with_icons).
    pub fn add_item_with_icon(&self, text: &str, icon: &impl IconSource) -> GuiResult<usize> {
        icon_support::require_owner_drawn(&self.core, LBS_OWNERDRAWFIXED)?;
        let index = self.add_item(text);
        self.set_item_icon(index, icon)?;
        Ok(index)
    }

    /// Shows an item icon (requires [`with_icons(true)`](ListBoxBuilder::with_icons)).
    pub fn set_item_icon(&self, index: usize, icon: &impl IconSource) -> GuiResult<()> {
        icon_support::set_item_icon(&self.core, LBS_OWNERDRAWFIXED, MESSAGES, index, Some(&icon))
    }

    /// Removes an item's icon.
    pub fn clear_item_icon(&self, index: usize) -> GuiResult<()> {
        icon_support::set_item_icon(&self.core, LBS_OWNERDRAWFIXED, MESSAGES, index, None)
    }

    /// Item icon index in the control's image list (`None` if absent).
    pub fn item_image(&self, index: usize) -> Option<i32> {
        icon_support::item_image(&self.core, MESSAGES, index)
    }

    fn is_multi(&self) -> bool {
        native::style(self.core.hwnd()) & (LBS_EXTENDEDSEL | LBS_MULTIPLESEL) != 0
    }

    /// Adds an item and returns its resulting index.
    pub fn add_item(&self, text: &str) -> usize {
        native::send_text(self.core.hwnd(), LB_ADDSTRING, 0, text).max(0) as usize
    }

    /// Inserts an item at the given position.
    pub fn insert_item(&self, index: usize, text: &str) {
        native::send_text(self.core.hwnd(), LB_INSERTSTRING, index, text);
    }

    /// Removes the item at the given position.
    pub fn remove_item(&self, index: usize) {
        native::send(self.core.hwnd(), LB_DELETESTRING, index, 0);
    }

    /// Removes all items.
    pub fn clear(&self) {
        native::send(self.core.hwnd(), LB_RESETCONTENT, 0, 0);
    }

    /// Number of items.
    pub fn count(&self) -> usize {
        native::send(self.core.hwnd(), LB_GETCOUNT, 0, 0).max(0) as usize
    }

    /// Text of the item at the given position.
    pub fn item_text(&self, index: usize) -> Option<String> {
        if index >= self.count() {
            return None;
        }
        native::read_text(self.core.hwnd(), LB_GETTEXTLEN, LB_GETTEXT, index)
    }

    /// Moves item `from` to `to`, retaining text, icon and selection.
    pub fn move_item(&self, from: usize, to: usize) {
        let hwnd = self.core.hwnd();
        let count = self.count();
        if from >= count || to >= count || from == to {
            return;
        }
        let Some(text) = self.item_text(from) else { return };
        let data = native::send(hwnd, LB_GETITEMDATA, from, 0);
        let selected = if self.is_multi() {
            native::send(hwnd, LB_GETSEL, from, 0) > 0
        } else {
            self.selected_index() == Some(from)
        };
        native::send(hwnd, LB_DELETESTRING, from, 0);
        native::send_text(hwnd, LB_INSERTSTRING, to, &text);
        native::send(hwnd, LB_SETITEMDATA, to, data);
        if self.is_multi() {
            self.set_item_selected(to, selected);
        } else if selected {
            self.set_selected_index(Some(to));
        }
    }

    /// Enables/disables mouse drag-reordering of items.
    ///
    /// The item follows the cursor, an accent insertion line marks its destination, the list scrolls near edges, and
    /// Esc cancels. Not useful in `sorted` lists (alphabetical order).
    pub fn set_reorderable(&self, reorderable: bool) {
        let config = drag::list_config(&self.core, ListKind::ListBox);
        config.reorder.set(reorderable);
        let this = self.clone();
        *config.mover.borrow_mut() = Some(Rc::new(move |from, to| this.move_item(from, to)));
        let this = self.clone();
        *config.block_mover.borrow_mut() = Some(Rc::new(move |items: &[usize], slot: usize| {
            for (from, to) in drag::block_moves(this.count(), items, slot) {
                this.move_item(from, to);
            }
        }));
    }

    /// Called when dropping multiple selected items: receives items and insertion position (`count()` = end); return
    /// `false` to cancel. Without this callback the block moves normally.
    pub fn on_items_reorder_request(&self, f: impl Fn(&[usize], usize) -> bool + 'static) {
        *drag::list_config(&self.core, ListKind::ListBox).block_veto.borrow_mut() = Some(Rc::new(f));
    }

    /// Former name of [`on_items_reorder_request`](Self::on_items_reorder_request), retained for existing code.
    #[deprecated(
        since = "0.1.0",
        note = "use `on_items_reorder_request`: events follow the naming convention in docs/EVENTS.md"
    )]
    pub fn on_items_reordered(&self, f: impl Fn(&[usize], usize) -> bool + 'static) {
        self.on_items_reorder_request(f)
    }

    /// Effects offered for external dragging (default: copy only). With
    /// [`DropEffect::Move`](crate::input::DropEffect::Move), delete the item in
    /// [`on_drag_out_finished`](Self::on_drag_out_finished).
    pub fn set_drag_out_effects(&self, effects: &[crate::input::DropEffect]) {
        drag::list_config(&self.core, ListKind::ListBox).out_effects.set(drag::effects_mask(effects));
    }

    /// Called after an external drag: receives item and target effect (`Move` = target took the item; `None` =
    /// canceled).
    pub fn on_drag_out_finished(&self, f: impl Fn(usize, crate::input::DropEffect) + 'static) {
        *drag::list_config(&self.core, ListKind::ListBox).out_done.borrow_mut() = Some(Rc::new(f));
    }

    /// Called before moving a dropped item: receives current and final positions; return `false` to cancel.
    pub fn on_item_reorder_request(&self, f: impl Fn(usize, usize) -> bool + 'static) {
        *drag::list_config(&self.core, ListKind::ListBox).veto.borrow_mut() = Some(Rc::new(f));
    }

    /// Former name of [`on_item_reorder_request`](Self::on_item_reorder_request), retained for existing code.
    #[deprecated(
        since = "0.1.0",
        note = "use `on_item_reorder_request`: events follow the naming convention in docs/EVENTS.md"
    )]
    pub fn on_reorder(&self, f: impl Fn(usize, usize) -> bool + 'static) {
        self.on_item_reorder_request(f)
    }

    /// Allows dragging items outside the list to another control/program.
    ///
    /// When the cursor leaves, `f` receives the item index and returns drag content (`None` keeps internal dragging).
    /// Works without [`set_reorderable`](Self::set_reorderable).
    pub fn on_drag_out(&self, f: impl Fn(usize) -> Option<ClipboardContent> + 'static) {
        *drag::list_config(&self.core, ListKind::ListBox).drag_out.borrow_mut() = Some(Rc::new(f));
    }

    /// Item index under a logical client-area point, for example drag
    /// [`DropData::position`](crate::input::DropData::position).
    pub fn index_at(&self, point: Point) -> Option<usize> {
        let hwnd = self.core.hwnd();
        let dpi = native::dpi_for_window(hwnd);
        let pixels = POINT { x: to_pixels(point.x, dpi), y: to_pixels(point.y, dpi) };
        drag::list_index_at(ListKind::ListBox, hwnd, pixels)
    }

    /// Indices of all selected items.
    pub fn selected_indices(&self) -> Vec<usize> {
        let hwnd = self.core.hwnd();
        (0..self.count()).filter(|&index| native::send(hwnd, LB_GETSEL, index, 0) > 0).collect()
    }

    /// First selected index, if any.
    pub fn selected_index(&self) -> Option<usize> {
        if self.is_multi() {
            self.selected_indices().first().copied()
        } else {
            let index = native::send(self.core.hwnd(), LB_GETCURSEL, 0, 0);
            (index >= 0).then_some(index as usize)
        }
    }

    /// Selects an item programmatically (`None` clears selection). Does not fire events.
    pub fn set_selected_index(&self, index: Option<usize>) {
        let hwnd = self.core.hwnd();
        if self.is_multi() {
            native::send(hwnd, LB_SETSEL, 0, -1);
            if let Some(index) = index {
                native::send(hwnd, LB_SETSEL, 1, index as isize);
            }
        } else {
            native::send(hwnd, LB_SETCURSEL, index.unwrap_or(usize::MAX), 0);
        }
    }

    /// Selects/deselects an item in multiple-selection lists.
    pub fn set_item_selected(&self, index: usize, selected: bool) {
        if self.is_multi() {
            native::send(self.core.hwnd(), LB_SETSEL, selected as usize, index as isize);
        } else if selected {
            self.set_selected_index(Some(index));
        }
    }

    /// Selection changed; receives the first selected index.
    pub fn on_selection_changed(&self, f: impl Fn(Option<usize>) + 'static) -> HandlerId {
        let this = self.clone();
        self.core.on(EventKind::SelectionChanged, move |_| f(this.selected_index()))
    }

    /// An item was double-clicked; receives its index.
    pub fn on_item_activated(&self, f: impl Fn(usize) + 'static) -> HandlerId {
        self.core.on(EventKind::ItemActivated, move |e| {
            if let Event::ItemActivated(index) = e {
                f(*index)
            }
        })
    }
}

/// Readjusts library-drawn item height after font/DPI changes.
pub(crate) fn refit_items(core: &crate::core::widget::WidgetCore) {
    if native::style(core.hwnd()) & LBS_OWNERDRAWFIXED != 0 {
        icon_support::fit_item_height(core, MESSAGES);
    }
}

const MESSAGES: ListMessages = ListMessages {
    text_length: LB_GETTEXTLEN,
    text: LB_GETTEXT,
    set_data: LB_SETITEMDATA,
    get_data: LB_GETITEMDATA,
    set_height: LB_SETITEMHEIGHT,
    selection_field: false,
};
