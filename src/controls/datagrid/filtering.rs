//! [`DataGrid`](super::DataGrid) row filtering and grouping.

use super::*;

impl DataGrid {
    // ------------------------------------------------------------------
    // Filtro e agrupamento
    // ------------------------------------------------------------------

    /// Shows only rows where `filter(cells)` returns `true` (in-memory mode). `None` removes filtering. Reapplied by
    /// [`add_row`](Self::add_row), [`set_rows`](Self::set_rows), sorting and [`refresh_view`](Self::refresh_view).
    pub fn set_filter(&self, filter: Option<impl Fn(&[String]) -> bool + 'static>) {
        self.cancel_edit();
        let previous = {
            let mut model = self.model.borrow_mut();
            let previous = std::mem::replace(&mut model.filter, filter.map(|f| Rc::new(f) as FilterFn));
            model.rebuild();
            previous
        };
        drop(previous);
        listview::set_row_selected(self.core.hwnd(), usize::MAX, false);
        self.refresh();
    }

    /// Simple text filter: shows rows where any cell (or the specified column) contains `text`, case-insensitively.
    /// Empty text removes filtering.
    pub fn set_text_filter(&self, text: &str, column: Option<usize>) {
        if text.is_empty() {
            self.set_filter(None::<fn(&[String]) -> bool>);
            return;
        }
        let needle = text.to_lowercase();
        self.set_filter(Some(move |cells: &[String]| match column {
            Some(c) => cells.get(c).is_some_and(|cell| cell.to_lowercase().contains(&needle)),
            None => cells.iter().any(|cell| cell.to_lowercase().contains(&needle)),
        }));
    }

    /// Whether a filter is active.
    pub fn has_filter(&self) -> bool {
        self.model.borrow().filter.is_some()
    }

    /// Reapplies filtering/grouping after [`set_cell`](Self::set_cell) changes.
    pub fn refresh_view(&self) {
        self.model.borrow_mut().rebuild();
        self.refresh();
    }

    /// Groups in-memory rows by a column value, with a value/count header toggled by click, Space, Enter or arrows.
    /// `None` removes grouping. Groups follow value appearance order; sort by the same column to order them.
    pub fn set_group_by(&self, column: Option<usize>) {
        self.cancel_edit();
        {
            let mut model = self.model.borrow_mut();
            model.group_by = column;
            model.collapsed.clear();
            model.rebuild();
        }
        listview::set_row_selected(self.core.hwnd(), usize::MAX, false);
        self.refresh();
    }

    /// Current grouping column.
    pub fn group_by(&self) -> Option<usize> {
        self.model.borrow().group_by
    }

    /// Current groups: column value, row count and collapsed state.
    pub fn groups(&self) -> Vec<(String, usize, bool)> {
        self.model.borrow().groups.iter().map(|g| (g.key.clone(), g.count, g.collapsed)).collect()
    }

    /// Collapses or expands the group for a value.
    pub fn set_group_collapsed(&self, key: &str, collapsed: bool) {
        {
            let mut model = self.model.borrow_mut();
            if collapsed {
                model.collapsed.insert(key.to_string());
            } else {
                model.collapsed.remove(key);
            }
            model.rebuild();
        }
        self.refresh();
    }

    pub(super) fn toggle_group(&self, group: usize) {
        self.cancel_edit();
        let Some((key, collapsed)) = self.model.borrow().groups.get(group).map(|g| (g.key.clone(), !g.collapsed))
        else {
            return;
        };
        self.set_group_collapsed(&key, collapsed);
        // Mantém o foco no cabeçalho do grupo que mudou.
        let item = self.model.borrow().items.iter().position(|i| *i == Item::Group(group));
        if let Some(item) = item {
            let hwnd = self.core.hwnd();
            listview::set_row_selected(hwnd, usize::MAX, false);
            listview::set_row_selected(hwnd, item, true);
        }
        let listeners = self.model.borrow().group_toggled.clone();
        for listener in listeners {
            listener(&key, collapsed);
        }
    }

    /// The user collapsed (`true`) or expanded a group; receives its value.
    pub fn on_group_toggled(&self, f: impl Fn(&str, bool) + 'static) {
        self.model.borrow_mut().group_toggled.push(Rc::new(f));
    }
}
