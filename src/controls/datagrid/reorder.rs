//! [`DataGrid`](super::DataGrid) row reordering and external dragging.

use super::*;

impl DataGrid {
    /// Moves row `from` to `to` (in-memory mode; icon, styles and selection follow). Clears active sorting because
    /// order is now user-defined.
    pub fn move_row(&self, from: usize, to: usize) {
        if !self.model.borrow_mut().move_visible(from, to) {
            return;
        }
        let hwnd = self.core.hwnd();
        listview::set_row_selected(hwnd, usize::MAX, false);
        if let Some(item) = self.model.borrow().row_to_item(to) {
            listview::set_row_selected(hwnd, item, true);
        }
        self.update_sort_arrow();
        self.refresh();
    }

    /// Moves rows as a block in current order before `slot` (`slot == row_count()` = end). Returns the first moved
    /// row's new position.
    pub fn move_rows(&self, rows: &[usize], slot: usize) -> Option<usize> {
        let first = self.model.borrow_mut().move_block(rows, slot)?;
        let hwnd = self.core.hwnd();
        listview::set_row_selected(hwnd, usize::MAX, false);
        let count = rows.iter().collect::<HashSet<_>>().len();
        for row in first..first + count {
            if let Some(item) = self.model.borrow().row_to_item(row) {
                listview::set_row_selected(hwnd, item, true);
            }
        }
        self.update_sort_arrow();
        self.refresh();
        Some(first)
    }

    /// Enables or disables mouse drag-reordering.
    ///
    /// In-memory mode moves rows (all selected rows move together). Virtual mode only notifies: handle
    /// [`on_row_reorder_request`](Self::on_row_reorder_request), returning `true` after moving application data, or
    /// [`on_rows_reorder_request`](Self::on_rows_reorder_request).
    pub fn set_reorderable(&self, reorderable: bool) {
        let config = drag::list_config(&self.core, ListKind::ListView);
        config.reorder.set(reorderable);
        let this = self.clone();
        *config.mover.borrow_mut() = Some(Rc::new(move |from, to| {
            this.commit_edit();
            let (from, to, virtual_mode) = {
                let model = this.model.borrow();
                (model.item_to_row(from), model.item_to_row(to), model.is_virtual())
            };
            let (Some(from), Some(to)) = (from, to) else { return };
            if virtual_mode {
                this.refresh();
                let hwnd = this.core.hwnd();
                listview::set_row_selected(hwnd, usize::MAX, false);
                listview::set_row_selected(hwnd, to, true);
            } else {
                this.move_row(from, to)
            }
        }));
        let this = self.clone();
        *config.block_mover.borrow_mut() = Some(Rc::new(move |items: &[usize], slot: usize| {
            this.commit_edit();
            let (rows, slot_row) = {
                let model = this.model.borrow();
                let rows: Vec<usize> = items.iter().filter_map(|&i| model.item_to_row(i)).collect();
                let slot_row =
                    (slot..model.item_count()).find_map(|i| model.item_to_row(i)).unwrap_or(model.row_count());
                (rows, slot_row)
            };
            if this.model.borrow().is_virtual() {
                // Fonte virtual: os dados já foram movidos pela aplicação em `on_rows_reordered`.
                this.refresh();
            } else {
                this.move_rows(&rows, slot_row);
            }
        }));
    }

    /// Called before moving a dropped row: receives current and final positions; return `false` to cancel.
    pub fn on_row_reorder_request(&self, f: impl Fn(usize, usize) -> bool + 'static) {
        let model = Rc::downgrade(&self.model);
        *drag::list_config(&self.core, ListKind::ListView).veto.borrow_mut() = Some(Rc::new(move |from, to| {
            let rows = model.upgrade().map(|m| {
                let m = m.borrow();
                (m.item_to_row(from), m.item_to_row(to))
            });
            match rows {
                Some((Some(from), Some(to))) => f(from, to),
                _ => false,
            }
        }));
    }

    /// Former name of [`on_row_reorder_request`](Self::on_row_reorder_request), retained for existing code.
    #[deprecated(
        since = "0.1.0",
        note = "use `on_row_reorder_request`: events follow the naming convention in docs/EVENTS.md"
    )]
    pub fn on_reorder(&self, f: impl Fn(usize, usize) -> bool + 'static) {
        self.on_row_reorder_request(f)
    }

    /// Called when dropping a block of two or more selected rows: receives rows and target position (`row_count()` =
    /// end); return `false` to cancel. In virtual mode the application moves data here.
    pub fn on_rows_reorder_request(&self, f: impl Fn(&[usize], usize) -> bool + 'static) {
        let model = Rc::downgrade(&self.model);
        *drag::list_config(&self.core, ListKind::ListView).block_veto.borrow_mut() =
            Some(Rc::new(move |items: &[usize], slot: usize| {
                let Some(model) = model.upgrade() else { return false };
                let (rows, slot_row) = {
                    let m = model.borrow();
                    let rows: Vec<usize> = items.iter().filter_map(|&i| m.item_to_row(i)).collect();
                    let slot_row = (slot..m.item_count()).find_map(|i| m.item_to_row(i)).unwrap_or(m.row_count());
                    (rows, slot_row)
                };
                f(&rows, slot_row)
            }));
    }

    /// Former name of [`on_rows_reorder_request`](Self::on_rows_reorder_request), retained for existing code.
    #[deprecated(
        since = "0.1.0",
        note = "use `on_rows_reorder_request`: events follow the naming convention in docs/EVENTS.md"
    )]
    pub fn on_rows_reordered(&self, f: impl Fn(&[usize], usize) -> bool + 'static) {
        self.on_rows_reorder_request(f)
    }

    /// Enables dragging rows out of the grid: when the cursor leaves, `f` receives the row and returns drag content
    /// (`None` keeps the internal drag).
    pub fn on_drag_out(&self, f: impl Fn(usize) -> Option<ClipboardContent> + 'static) {
        let model = Rc::downgrade(&self.model);
        *drag::list_config(&self.core, ListKind::ListView).drag_out.borrow_mut() = Some(Rc::new(move |item| {
            let row = model.upgrade().and_then(|m| m.borrow().item_to_row(item))?;
            f(row)
        }));
    }

    /// Effects offered for external dragging (default: copy only).
    pub fn set_drag_out_effects(&self, effects: &[crate::input::DropEffect]) {
        drag::list_config(&self.core, ListKind::ListView).out_effects.set(drag::effects_mask(effects));
    }

    /// Called after an external drag with the row and target effect (`Move` = delete the row; `None` = canceled).
    pub fn on_drag_out_finished(&self, f: impl Fn(usize, crate::input::DropEffect) + 'static) {
        let model = Rc::downgrade(&self.model);
        *drag::list_config(&self.core, ListKind::ListView).out_done.borrow_mut() =
            Some(Rc::new(move |item, effect| {
                if let Some(row) = model.upgrade().and_then(|m| m.borrow().item_to_row(item)) {
                    f(row, effect)
                }
            }));
    }

    /// Row under a point (logical units relative to client area). Group headers return `None`.
    pub fn row_at(&self, point: Point) -> Option<usize> {
        let hwnd = self.core.hwnd();
        let dpi = native::dpi_for_window(hwnd);
        let item = drag::list_index_at(
            ListKind::ListView,
            hwnd,
            POINT { x: to_pixels(point.x, dpi), y: to_pixels(point.y, dpi) },
        )?;
        self.model.borrow().item_to_row(item)
    }
}
