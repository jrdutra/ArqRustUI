//! Column list (ListView report mode).

use super::common::{builder_common, widget_type, ControlOptions};
use super::icon_support::{ManagedImages, DEFAULT_ICON_SIZE};
use crate::core::handles::IconSource;
use crate::core::handles::ImageList;
use crate::core::native;
use crate::core::widget::{Reflected, WidgetCore};
use crate::core::{Event, EventKind, GuiError, GuiResult, HandlerId};
use crate::ffi::comctl32::*;
use crate::ffi::user32::*;
use crate::ffi::{HWND, LPARAM, POINT};
use crate::input::drag::{self, ListKind};
use crate::system::ClipboardContent;
use crate::utilities::{to_logical, to_pixels, to_wide, Point};
use std::rc::Rc;

/// Column content alignment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ColumnAlign {
    /// Left.
    Left,
    /// Centered.
    Center,
    /// Right.
    Right,
}

widget_type!(
    /// Multicolumn list with single/multiple selection, sorting and images.
    ///
    /// ```no_run
    /// # use arqrustui::prelude::*;
    /// # fn demo(window: &Window) -> GuiResult<()> {
    /// let list = ListView::builder().bounds(10, 10, 400, 200).parent(window).build()?;
    /// list.add_column("Name", 200, ColumnAlign::Left);
    /// list.add_column("Size", 100, ColumnAlign::Right);
    /// list.add_row(&["report.pdf", "1,2 MB"]);
    /// list.on_column_click({
    ///     let list = list.clone();
    ///     move |column| list.sort_by_column(column, true)
    /// });
    /// # Ok(()) }
    /// ```
    ListView
);

/// Builder for [`ListView`].
#[derive(Clone, Debug)]
pub struct ListViewBuilder {
    opts: ControlOptions,
    single_selection: bool,
    grid_lines: bool,
    checkboxes: bool,
}

impl ListViewBuilder {
    builder_common!();

    /// Restricts selection to one row (default: multiple selection).
    pub fn single_selection(mut self, single: bool) -> Self {
        self.single_selection = single;
        self
    }

    /// Draws grid lines between cells.
    pub fn grid_lines(mut self, grid: bool) -> Self {
        self.grid_lines = grid;
        self
    }

    /// Shows a check box in each row.
    pub fn checkboxes(mut self, checkboxes: bool) -> Self {
        self.checkboxes = checkboxes;
        self
    }

    /// Creates the list.
    pub fn build(self) -> GuiResult<ListView> {
        // LVS_SHAREIMAGELISTS: a lista de imagens pertence ao wrapper `ImageList`, não ao controle
        // (sem ele, o ListView a destruiria e o wrapper a destruiria de novo).
        let mut style = LVS_REPORT | LVS_SHOWSELALWAYS | LVS_SHAREIMAGELISTS;
        if self.single_selection {
            style |= LVS_SINGLESEL;
        }
        let core = self.opts.create("ListView", "SysListView32", style, WS_EX_CLIENTEDGE, true, None)?;
        let mut extended = LVS_EX_FULLROWSELECT | LVS_EX_DOUBLEBUFFER | LVS_EX_HEADERDRAGDROP;
        if self.grid_lines {
            extended |= LVS_EX_GRIDLINES;
        }
        if self.checkboxes {
            extended |= LVS_EX_CHECKBOXES;
        }
        native::send(core.hwnd(), LVM_SETEXTENDEDLISTVIEWSTYLE, 0, extended as isize);
        core.set_reflect(reflect_list_view);
        Ok(ListView { core })
    }
}

/// Notifications shared by ListView and DataGrid.
pub(crate) fn reflect_list_view(core: &Rc<WidgetCore>, reflected: &Reflected) -> Option<isize> {
    let Reflected::Notify { code, lparam } = reflected else { return None };
    match *code {
        LVN_ITEMCHANGED => {
            // SAFETY: nesta notificação o LPARAM aponta para um NMLISTVIEW válido.
            let info = unsafe { &*(*lparam as *const NMLISTVIEW) };
            let selection_bit = (info.uNewState ^ info.uOldState) & LVIS_SELECTED != 0;
            let check_bit = (info.uNewState ^ info.uOldState) & LVIS_STATEIMAGEMASK != 0;
            if selection_bit {
                core.emit(&Event::SelectionChanged);
            }
            if check_bit && info.uOldState & LVIS_STATEIMAGEMASK != 0 {
                core.emit(&Event::ValueChanged);
            }
            Some(0)
        }
        LVN_COLUMNCLICK => {
            // SAFETY: nesta notificação o LPARAM aponta para um NMLISTVIEW válido.
            let info = unsafe { &*(*lparam as *const NMLISTVIEW) };
            core.emit(&Event::ColumnClick(info.iSubItem.max(0) as usize));
            Some(0)
        }
        LVN_ITEMACTIVATE => {
            // SAFETY: nesta notificação o LPARAM aponta para uma estrutura iniciada por NMLISTVIEW.
            let info = unsafe { &*(*lparam as *const NMLISTVIEW) };
            if info.iItem >= 0 {
                core.emit(&Event::ItemActivated(info.iItem as usize));
            }
            Some(0)
        }
        NM_DBLCLK => {
            core.emit(&Event::DoubleClick);
            None
        }
        _ => None,
    }
}

pub(crate) fn empty_item() -> LVITEMW {
    LVITEMW {
        mask: 0,
        iItem: 0,
        iSubItem: 0,
        state: 0,
        stateMask: 0,
        pszText: std::ptr::null_mut(),
        cchTextMax: 0,
        iImage: 0,
        lParam: 0,
        iIndent: 0,
        iGroupId: 0,
        cColumns: 0,
        puColumns: std::ptr::null_mut(),
        piColFmt: std::ptr::null_mut(),
        iGroup: 0,
    }
}

/// Adds a ListView column and returns its index.
pub(crate) fn insert_column(core: &WidgetCore, index: usize, title: &str, width: i32, align: ColumnAlign) -> usize {
    let mut text = to_wide(title);
    let column = LVCOLUMNW {
        mask: LVCF_FMT | LVCF_WIDTH | LVCF_TEXT | LVCF_SUBITEM,
        fmt: match align {
            ColumnAlign::Left => LVCFMT_LEFT,
            ColumnAlign::Center => LVCFMT_CENTER,
            ColumnAlign::Right => LVCFMT_RIGHT,
        },
        cx: to_pixels(width, core.dpi()),
        pszText: text.as_mut_ptr(),
        cchTextMax: 0,
        iSubItem: index as i32,
        iImage: 0,
        iOrder: 0,
        cxMin: 0,
        cxDefault: 0,
        cxIdeal: 0,
    };
    native::send_ref(core.hwnd(), LVM_INSERTCOLUMNW, index, &column).max(0) as usize
}

pub(crate) fn header_count(hwnd: HWND) -> usize {
    let header = native::send(hwnd, LVM_GETHEADER, 0, 0) as HWND;
    native::send(header, HDM_GETITEMCOUNT, 0, 0).max(0) as usize
}

/// Selected ListView rows.
pub(crate) fn selected_rows(hwnd: HWND) -> Vec<usize> {
    let mut rows = Vec::new();
    let mut index = -1isize;
    loop {
        index = native::send(hwnd, LVM_GETNEXTITEM, index as usize, LVNI_SELECTED as isize);
        if index < 0 {
            break;
        }
        rows.push(index as usize);
    }
    rows
}

pub(crate) fn set_row_selected(hwnd: HWND, row: usize, selected: bool) {
    let mut item = empty_item();
    item.stateMask = LVIS_SELECTED | LVIS_FOCUSED;
    item.state = if selected { LVIS_SELECTED | LVIS_FOCUSED } else { 0 };
    native::send_ref(hwnd, LVM_SETITEMSTATE, row, &item);
    if selected {
        native::send(hwnd, LVM_ENSUREVISIBLE, row, 0);
    }
}

struct SortContext {
    hwnd: HWND,
    column: usize,
    ascending: bool,
}

fn cell_text(hwnd: HWND, row: usize, column: usize) -> String {
    let mut buffer = vec![0u16; 512];
    let mut item = empty_item();
    item.iSubItem = column as i32;
    item.pszText = buffer.as_mut_ptr();
    item.cchTextMax = buffer.len() as i32;
    native::send_mut(hwnd, LVM_GETITEMTEXTW, row, &mut item);
    crate::utilities::from_wide(&buffer)
}

/// Compares text treating numbers numerically ("10" after "9").
pub(crate) fn natural_compare(a: &str, b: &str) -> std::cmp::Ordering {
    let parse = |s: &str| s.trim().replace(',', ".").parse::<f64>().ok();
    match (parse(a), parse(b)) {
        (Some(x), Some(y)) => x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal),
        _ => a.to_lowercase().cmp(&b.to_lowercase()),
    }
}

unsafe extern "system" fn compare_rows(first: LPARAM, second: LPARAM, context: LPARAM) -> i32 {
    crate::core::ffi_guard::guard(0, move || {
        // SAFETY: `context` é o ponteiro para o SortContext local de `sort_by_column`,
        // válido durante a chamada síncrona de LVM_SORTITEMSEX.
        let context = &*(context as *const SortContext);
        let a = cell_text(context.hwnd, first as usize, context.column);
        let b = cell_text(context.hwnd, second as usize, context.column);
        let ordering = natural_compare(&a, &b);
        let ordering = if context.ascending { ordering } else { ordering.reverse() };
        ordering as i32
    })
}

impl ListView {
    /// Starts building (default size 320×200).
    pub fn builder() -> ListViewBuilder {
        ListViewBuilder {
            opts: ControlOptions::new(320, 200),
            single_selection: false,
            grid_lines: false,
            checkboxes: false,
        }
    }

    /// Adds a column (logical width) and returns its index.
    pub fn add_column(&self, title: &str, width: i32, align: ColumnAlign) -> usize {
        insert_column(&self.core, self.column_count(), title, width, align)
    }

    /// Number of columns.
    pub fn column_count(&self) -> usize {
        header_count(self.core.hwnd())
    }

    /// Logical column width (users can drag to resize).
    pub fn column_width(&self, column: usize) -> i32 {
        to_logical(native::send(self.core.hwnd(), LVM_GETCOLUMNWIDTH, column, 0) as i32, self.core.dpi())
    }

    /// Sets logical column width.
    pub fn set_column_width(&self, column: usize, width: i32) {
        native::send(self.core.hwnd(), LVM_SETCOLUMNWIDTH, column, to_pixels(width, self.core.dpi()) as isize);
    }

    /// Inserts a row at the given position with one text per column.
    pub fn insert_row(&self, row: usize, cells: &[&str]) -> GuiResult<usize> {
        let hwnd = self.core.require_alive()?;
        let mut text = to_wide(cells.first().copied().unwrap_or(""));
        let mut item = empty_item();
        // Sem imagem até que uma seja atribuída (senão a linha mostraria a imagem 0 da lista).
        item.mask = LVIF_TEXT | LVIF_IMAGE;
        item.iImage = I_IMAGENONE;
        item.iItem = row as i32;
        item.pszText = text.as_mut_ptr();
        let index = native::send_ref(hwnd, LVM_INSERTITEMW, 0, &item);
        if index < 0 {
            return Err(GuiError::InvalidArgument("could not insert the row into the ListView".into()));
        }
        for (column, cell) in cells.iter().enumerate().skip(1) {
            self.set_cell(index as usize, column, cell);
        }
        Ok(index as usize)
    }

    /// Appends a row and returns its index.
    pub fn add_row(&self, cells: &[&str]) -> usize {
        self.insert_row(self.row_count(), cells).unwrap_or(0)
    }

    /// Sets cell text.
    pub fn set_cell(&self, row: usize, column: usize, text: &str) {
        let mut wide = to_wide(text);
        let mut item = empty_item();
        item.iSubItem = column as i32;
        item.pszText = wide.as_mut_ptr();
        native::send_ref(self.core.hwnd(), LVM_SETITEMTEXTW, row, &item);
    }

    /// Cell text (`None` if row/column does not exist).
    pub fn cell(&self, row: usize, column: usize) -> Option<String> {
        if row >= self.row_count() || column >= self.column_count() {
            return None;
        }
        Some(cell_text(self.core.hwnd(), row, column))
    }

    /// Removes a row.
    pub fn remove_row(&self, row: usize) {
        native::send(self.core.hwnd(), LVM_DELETEITEM, row, 0);
    }

    /// Removes all rows.
    pub fn clear(&self) {
        native::send(self.core.hwnd(), LVM_DELETEALLITEMS, 0, 0);
    }

    /// Number of rows.
    pub fn row_count(&self) -> usize {
        native::send(self.core.hwnd(), LVM_GETITEMCOUNT, 0, 0).max(0) as usize
    }

    /// Selected row indices.
    pub fn selected_rows(&self) -> Vec<usize> {
        selected_rows(self.core.hwnd())
    }

    /// First selected row, if any.
    pub fn selected_row(&self) -> Option<usize> {
        self.selected_rows().first().copied()
    }

    /// Selects or deselects a row.
    pub fn set_row_selected(&self, row: usize, selected: bool) {
        set_row_selected(self.core.hwnd(), row, selected);
    }

    /// Whether the row check box is checked (`checkboxes` lists).
    pub fn is_row_checked(&self, row: usize) -> bool {
        let state = native::send(self.core.hwnd(), LVM_GETITEMSTATE, row, LVIS_STATEIMAGEMASK as isize) as u32;
        (state >> 12) == 2
    }

    /// Checks/unchecks the row check box.
    pub fn set_row_checked(&self, row: usize, checked: bool) {
        let mut item = empty_item();
        item.stateMask = LVIS_STATEIMAGEMASK;
        item.state = if checked { 2 << 12 } else { 1 << 12 };
        native::send_ref(self.core.hwnd(), LVM_SETITEMSTATE, row, &item);
    }

    /// Attaches a custom small-icon image list to rows (replaces the managed list from
    /// [`set_row_icon`](Self::set_row_icon)).
    pub fn set_image_list(&self, images: &Rc<ImageList>) {
        ManagedImages::release(&self.core);
        native::send(self.core.hwnd(), LVM_SETIMAGELIST, LVSIL_SMALL, images.raw() as isize);
        self.core.keep_alive(images.clone());
    }

    /// Shows an icon in the row's first column.
    ///
    /// The list maintains an image list sized for monitor DPI; each distinct icon occupies one slot.
    pub fn set_row_icon(&self, row: usize, icon: &impl IconSource) -> GuiResult<()> {
        let size = ManagedImages::logical_size(&self.core).unwrap_or(DEFAULT_ICON_SIZE);
        let index = ManagedImages::index_of(&self.core, icon, size, attach_images)?;
        self.set_row_image(row, index);
        Ok(())
    }

    /// Removes the row icon.
    pub fn clear_row_icon(&self, row: usize) {
        self.set_row_image(row, I_IMAGENONE);
    }

    /// Logical row icon size (default: 16). Reloads icons already used.
    pub fn set_icon_size(&self, size: i32) -> GuiResult<()> {
        ManagedImages::resize(&self.core, size.max(1), attach_images)
    }

    /// Row image index (`None` if absent).
    pub fn row_image(&self, row: usize) -> Option<i32> {
        let mut item = empty_item();
        item.mask = LVIF_IMAGE;
        item.iItem = row as i32;
        native::send_mut(self.core.hwnd(), LVM_GETITEMW, 0, &mut item);
        (item.iImage >= 0).then_some(item.iImage)
    }

    /// Sets a row image (image list index).
    pub fn set_row_image(&self, row: usize, image: i32) {
        let mut item = empty_item();
        item.mask = LVIF_IMAGE;
        item.iItem = row as i32;
        item.iImage = image;
        native::send_ref(self.core.hwnd(), LVM_SETITEMW, 0, &item);
    }

    /// Moves row `from` to `to`, retaining cells, image, check box and selection.
    pub fn move_row(&self, from: usize, to: usize) {
        let hwnd = self.core.hwnd();
        let count = self.row_count();
        if from >= count || to >= count || from == to {
            return;
        }
        let cells: Vec<String> =
            (0..self.column_count().max(1)).map(|c| self.cell(from, c).unwrap_or_default()).collect();
        let mut item = empty_item();
        item.mask = LVIF_IMAGE | LVIF_PARAM | LVIF_STATE;
        item.iItem = from as i32;
        item.stateMask = LVIS_STATEIMAGEMASK | LVIS_SELECTED | LVIS_FOCUSED;
        native::send_mut(hwnd, LVM_GETITEMW, 0, &mut item);
        native::send(hwnd, LVM_DELETEITEM, from, 0);
        let refs: Vec<&str> = cells.iter().map(String::as_str).collect();
        let Ok(index) = self.insert_row(to, &refs) else { return };
        item.iItem = index as i32;
        item.iSubItem = 0;
        item.pszText = std::ptr::null_mut();
        native::send_ref(hwnd, LVM_SETITEMW, 0, &item);
        if item.state & LVIS_SELECTED != 0 {
            native::send(hwnd, LVM_ENSUREVISIBLE, index, 0);
        }
    }

    /// Enables/disables mouse drag-reordering: row follows cursor, insertion line marks destination, list scrolls near
    /// edges and Esc cancels.
    pub fn set_reorderable(&self, reorderable: bool) {
        let config = drag::list_config(&self.core, ListKind::ListView);
        config.reorder.set(reorderable);
        let this = self.clone();
        *config.mover.borrow_mut() = Some(Rc::new(move |from, to| this.move_row(from, to)));
        let this = self.clone();
        *config.block_mover.borrow_mut() = Some(Rc::new(move |rows: &[usize], slot: usize| {
            for (from, to) in drag::block_moves(this.row_count(), rows, slot) {
                this.move_row(from, to);
            }
        }));
    }

    /// Called when dropping multiple selected rows: receives rows and insertion position (`row_count()` = end); return
    /// `false` to cancel.
    pub fn on_rows_reorder_request(&self, f: impl Fn(&[usize], usize) -> bool + 'static) {
        *drag::list_config(&self.core, ListKind::ListView).block_veto.borrow_mut() = Some(Rc::new(f));
    }

    /// Former name of [`on_rows_reorder_request`](Self::on_rows_reorder_request), retained for existing code.
    #[deprecated(
        since = "0.1.0",
        note = "use `on_rows_reorder_request`: events follow the naming convention in docs/EVENTS.md"
    )]
    pub fn on_rows_reordered(&self, f: impl Fn(&[usize], usize) -> bool + 'static) {
        self.on_rows_reorder_request(f)
    }

    /// Effects offered for external dragging (default: copy only).
    pub fn set_drag_out_effects(&self, effects: &[crate::input::DropEffect]) {
        drag::list_config(&self.core, ListKind::ListView).out_effects.set(drag::effects_mask(effects));
    }

    /// Called after an external drag with the row and target effect (`Move` = delete the row; `None` = canceled).
    pub fn on_drag_out_finished(&self, f: impl Fn(usize, crate::input::DropEffect) + 'static) {
        *drag::list_config(&self.core, ListKind::ListView).out_done.borrow_mut() = Some(Rc::new(f));
    }

    /// Called before moving a dropped row: receives current and final positions; return `false` to cancel.
    pub fn on_row_reorder_request(&self, f: impl Fn(usize, usize) -> bool + 'static) {
        *drag::list_config(&self.core, ListKind::ListView).veto.borrow_mut() = Some(Rc::new(f));
    }

    /// Former name of [`on_row_reorder_request`](Self::on_row_reorder_request), retained for existing code.
    #[deprecated(
        since = "0.1.0",
        note = "use `on_row_reorder_request`: events follow the naming convention in docs/EVENTS.md"
    )]
    pub fn on_reorder(&self, f: impl Fn(usize, usize) -> bool + 'static) {
        self.on_row_reorder_request(f)
    }

    /// Allows external row dragging: on leaving the control, `f` receives the row and returns drag content (`None`
    /// keeps internal dragging). Works without [`set_reorderable`](Self::set_reorderable).
    pub fn on_drag_out(&self, f: impl Fn(usize) -> Option<ClipboardContent> + 'static) {
        *drag::list_config(&self.core, ListKind::ListView).drag_out.borrow_mut() = Some(Rc::new(f));
    }

    /// Row at a logical point relative to the client area.
    pub fn row_at(&self, point: Point) -> Option<usize> {
        let hwnd = self.core.hwnd();
        let dpi = native::dpi_for_window(hwnd);
        drag::list_index_at(ListKind::ListView, hwnd, POINT { x: to_pixels(point.x, dpi), y: to_pixels(point.y, dpi) })
    }

    /// Sorts rows by a column's content, treating numbers numerically.
    pub fn sort_by_column(&self, column: usize, ascending: bool) {
        let hwnd = self.core.hwnd();
        let context = SortContext { hwnd, column, ascending };
        // SAFETY: o callback só é chamado durante esta chamada síncrona, com o ponteiro
        // para `context`, que vive até o fim dela.
        unsafe {
            SendMessageW(
                hwnd,
                LVM_SORTITEMSEX,
                &context as *const SortContext as usize,
                compare_rows as *const () as isize,
            )
        };
    }

    /// Selection changed.
    pub fn on_selection_changed(&self, f: impl Fn(Vec<usize>) + 'static) -> HandlerId {
        let this = self.clone();
        self.core.on(EventKind::SelectionChanged, move |_| f(this.selected_rows()))
    }

    /// A row was activated (double click/Enter); receives its index.
    pub fn on_row_activated(&self, f: impl Fn(usize) + 'static) -> HandlerId {
        self.core.on(EventKind::ItemActivated, move |e| {
            if let Event::ItemActivated(row) = e {
                f(*row)
            }
        })
    }

    /// Former name of [`on_row_activated`](Self::on_row_activated), retained for existing code.
    #[deprecated(
        since = "0.1.0",
        note = "use `on_row_activated`: events follow the naming convention in docs/EVENTS.md"
    )]
    pub fn on_item_activated(&self, f: impl Fn(usize) + 'static) -> HandlerId {
        self.on_row_activated(f)
    }

    /// A column header was clicked; receives its index.
    pub fn on_column_click(&self, f: impl Fn(usize) + 'static) -> HandlerId {
        self.core.on(EventKind::ColumnClick, move |e| {
            if let Event::ColumnClick(column) = e {
                f(*column)
            }
        })
    }

    /// The user changed a row check box.
    pub fn on_check_changed(&self, f: impl Fn() + 'static) -> HandlerId {
        self.core.on(EventKind::ValueChanged, move |_| f())
    }
}

/// Supplies a ListView small-image list (also used by [`DataGrid`](super::DataGrid)).
pub(crate) fn attach_images(hwnd: HWND, list: &ImageList) {
    native::send(hwnd, LVM_SETIMAGELIST, LVSIL_SMALL, list.raw() as isize);
}

#[cfg(test)]
mod tests {
    use super::natural_compare;
    use std::cmp::Ordering;

    #[test]
    fn natural_ordering() {
        assert_eq!(natural_compare("9", "10"), Ordering::Less);
        assert_eq!(natural_compare("2,5", "2.25"), Ordering::Greater);
        assert_eq!(natural_compare("abc", "ABD"), Ordering::Less);
        assert_eq!(natural_compare("Ana", "ana"), Ordering::Equal);
    }
}
