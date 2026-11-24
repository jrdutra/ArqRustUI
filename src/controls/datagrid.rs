//! Virtualized data grid with sorting, editing, filters, grouping, cell formatting, check box columns and export (own
//! implementation).

use super::common::{builder_common, ControlOptions};
use super::icon_support::{ManagedImages, DEFAULT_ICON_SIZE};
use super::listview::{self, ColumnAlign};
use crate::core::handles::{GdiObject, IconSource};
use crate::core::messages::RawMessage;
use crate::core::native;
use crate::core::widget::{Reflected, Widget, WidgetCore};
use crate::core::{Event, EventKind, GuiError, GuiResult, HandlerId};
use crate::ffi::comctl32::*;
use crate::ffi::gdi32::SelectObject;
use crate::ffi::user32::*;
use crate::ffi::POINT;
use crate::ffi::{HWND, RECT};
use crate::graphics::{Canvas, Font, Pen, TextAlign};
use crate::i18n::{library_text, LibraryText};
use crate::icons::IconAsset;
use crate::input::drag::{self, ListKind};
use crate::system::ClipboardContent;
use crate::utilities::{to_logical, to_pixels, Color, Point, Rect, Size};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

mod drawing;
mod editing;
mod export;
mod filtering;
mod reorder;
mod styling;

/// Edit hook decision when cell editing begins.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum EditAction {
    /// Opens the default text editor.
    Default,
    /// The cell cannot be edited.
    Deny,
    /// The hook handled editing itself.
    Handled,
    /// Opens a drop-down with options (selection is an index).
    Choice(Vec<String>, usize),
}

/// Visual formatting of a [`DataGrid`] cell or row.
///
/// `None`/`false` fields retain grid defaults (theme colors, normal font). Selected rows retain system selection
/// colors.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CellStyle {
    /// Text color.
    pub foreground: Option<Color>,
    /// Background color.
    pub background: Option<Color>,
    /// Bold text.
    pub bold: bool,
    /// Italic text.
    pub italic: bool,
}

impl CellStyle {
    /// Unmodified style.
    pub fn new() -> CellStyle {
        CellStyle::default()
    }

    /// Sets text color.
    pub fn foreground(mut self, color: Color) -> CellStyle {
        self.foreground = Some(color);
        self
    }

    /// Sets background color.
    pub fn background(mut self, color: Color) -> CellStyle {
        self.background = Some(color);
        self
    }

    /// Bold text.
    pub fn bold(mut self) -> CellStyle {
        self.bold = true;
        self
    }

    /// Italic text.
    pub fn italic(mut self) -> CellStyle {
        self.italic = true;
        self
    }

    /// Combines styles: fields defined in `over` take precedence.
    pub fn merged(self, over: CellStyle) -> CellStyle {
        CellStyle {
            foreground: over.foreground.or(self.foreground),
            background: over.background.or(self.background),
            bold: self.bold || over.bold,
            italic: self.italic || over.italic,
        }
    }
}

/// [`DataGrid`] column type: determines value display and editing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum ColumnKind {
    /// Text, edited in an overlaid field (default).
    #[default]
    Text,
    /// Check box: cell text is interpreted as a boolean (`true`/`1`/`sim`/`yes`/`x`); click or Space toggles and writes
    /// `"true"`/`"false"`.
    CheckBox,
    /// Drop-down with the given options; the cell stores option text.
    Choice(Vec<String>),
}

/// Interprets check box cell text.
pub fn parse_check(text: &str) -> bool {
    matches!(
        text.trim().to_lowercase().as_str(),
        "true" | "1" | "sim" | "s" | "yes" | "y" | "x" | "✓" | "on" | "verdadeiro"
    )
}

type Provider = Rc<dyn Fn(usize, usize) -> String>;
type EditedFn = Rc<dyn Fn(usize, usize, &str)>;
type EditHook = Rc<dyn Fn(usize, usize) -> EditAction>;
type Validator = Rc<dyn Fn(usize, usize, &str) -> bool>;
type IconProvider = Rc<dyn Fn(usize) -> Option<&'static IconAsset>>;
type StyleProvider = Rc<dyn Fn(usize, usize, &str) -> Option<CellStyle>>;
type FilterFn = Rc<dyn Fn(&[String]) -> bool>;
type VirtualEditor = Rc<dyn Fn(usize, usize, &str) -> bool>;
type GroupToggledFn = Rc<dyn Fn(&str, bool)>;

struct Editor {
    core: Rc<WidgetCore>,
    row: usize,
    column: usize,
    /// Drop-down (`true`) or text field.
    choice: bool,
}

#[derive(Clone, Default)]
struct Row {
    cells: Vec<String>,
    icon: i32,
    style: Option<CellStyle>,
    cell_styles: HashMap<usize, CellStyle>,
}

impl Row {
    fn new(cells: Vec<String>) -> Row {
        Row { cells, icon: I_IMAGENONE, style: None, cell_styles: HashMap::new() }
    }
}

/// Item displayed by ListView when grouped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Item {
    Row(usize),
    Group(usize),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Group {
    key: String,
    count: usize,
    collapsed: bool,
}

/// Data and display model. All logic independent of Windows lives here (testable without windows).
#[derive(Default)]
struct Model {
    /// In-memory rows in current order (changed by sorting/reordering).
    data: Vec<Row>,
    /// Public row → `data` index after filtering/grouping.
    visible: Vec<usize>,
    /// ListView items when grouped (empty without grouping).
    items: Vec<Item>,
    /// Public row → ListView item (grouped only; `None` in collapsed groups).
    row_items: Vec<Option<usize>>,
    groups: Vec<Group>,
    group_by: Option<usize>,
    collapsed: HashSet<String>,
    filter: Option<FilterFn>,
    columns: Vec<String>,
    kinds: HashMap<usize, ColumnKind>,
    icon_provider: Option<IconProvider>,
    style_provider: Option<StyleProvider>,
    source: Option<(usize, Provider)>,
    virtual_editor: Option<VirtualEditor>,
    sort: Option<(usize, bool)>,
    editable: bool,
    sortable: bool,
    read_only_columns: HashSet<usize>,
    edit_hook: Option<EditHook>,
    validator: Option<Validator>,
    edited: Vec<EditedFn>,
    group_toggled: Vec<GroupToggledFn>,
    editor: Option<Editor>,
    /// Fonts (DPI, bold, italic) kept alive for custom drawing.
    fonts: Vec<(u32, bool, bool, Rc<GdiObject>)>,
}

impl Model {
    fn is_virtual(&self) -> bool {
        self.source.is_some()
    }

    /// Number of public rows.
    fn row_count(&self) -> usize {
        match &self.source {
            Some((count, _)) => *count,
            None => self.visible.len(),
        }
    }

    /// Number of ListView items (rows plus visible group headers).
    fn item_count(&self) -> usize {
        if self.group_by.is_some() && !self.is_virtual() {
            self.items.len()
        } else {
            self.row_count()
        }
    }

    fn grouped(&self) -> bool {
        self.group_by.is_some() && !self.is_virtual()
    }

    fn item(&self, item: usize) -> Option<Item> {
        if self.grouped() {
            self.items.get(item).copied()
        } else {
            (item < self.row_count()).then_some(Item::Row(item))
        }
    }

    fn item_to_row(&self, item: usize) -> Option<usize> {
        match self.item(item)? {
            Item::Row(row) => Some(row),
            Item::Group(_) => None,
        }
    }

    fn row_to_item(&self, row: usize) -> Option<usize> {
        if self.grouped() {
            self.row_items.get(row).copied().flatten()
        } else {
            (row < self.row_count()).then_some(row)
        }
    }

    fn row(&self, row: usize) -> Option<&Row> {
        self.visible.get(row).and_then(|&index| self.data.get(index))
    }

    fn row_mut(&mut self, row: usize) -> Option<&mut Row> {
        let index = *self.visible.get(row)?;
        self.data.get_mut(index)
    }

    /// Recalculates filtering/grouping from `data` (in-memory mode).
    fn rebuild(&mut self) {
        self.items.clear();
        self.row_items.clear();
        self.groups.clear();
        if self.is_virtual() {
            self.visible.clear();
            return;
        }
        let filter = self.filter.clone();
        let passing: Vec<usize> =
            (0..self.data.len()).filter(|&i| filter.as_ref().map_or(true, |f| f(&self.data[i].cells))).collect();
        let Some(column) = self.group_by else {
            self.visible = passing;
            return;
        };
        // Grupos na ordem em que a chave aparece (acompanham a ordenação).
        let mut order: Vec<String> = Vec::new();
        let mut members: HashMap<String, Vec<usize>> = HashMap::new();
        for index in passing {
            let key = self.data[index].cells.get(column).cloned().unwrap_or_default();
            if !members.contains_key(&key) {
                order.push(key.clone());
            }
            members.entry(key).or_default().push(index);
        }
        self.visible.clear();
        for key in order {
            let rows = members.remove(&key).unwrap_or_default();
            let collapsed = self.collapsed.contains(&key);
            let group = self.groups.len();
            self.items.push(Item::Group(group));
            self.groups.push(Group { key, count: rows.len(), collapsed });
            for index in rows {
                let row = self.visible.len();
                self.visible.push(index);
                if collapsed {
                    self.row_items.push(None);
                } else {
                    self.row_items.push(Some(self.items.len()));
                    self.items.push(Item::Row(row));
                }
            }
        }
    }

    fn cell(&self, row: usize, column: usize) -> String {
        self.row(row).and_then(|r| r.cells.get(column)).cloned().unwrap_or_default()
    }

    fn kind(&self, column: usize) -> ColumnKind {
        self.kinds.get(&column).cloned().unwrap_or_default()
    }

    fn has_checkboxes(&self) -> bool {
        self.kinds.values().any(|k| *k == ColumnKind::CheckBox)
    }

    fn has_styles(&self) -> bool {
        self.style_provider.is_some()
            || (!self.is_virtual() && self.data.iter().any(|r| r.style.is_some() || !r.cell_styles.is_empty()))
    }

    /// Final cell style (row < cell < style function).
    fn static_style(&self, row: usize, column: usize) -> CellStyle {
        let Some(r) = self.row(row) else { return CellStyle::default() };
        let base = r.style.unwrap_or_default();
        r.cell_styles.get(&column).map_or(base, |cell| base.merged(*cell))
    }

    /// Moves public row `from` to public position `to`, preserving relative order of rows hidden by filtering.
    fn move_visible(&mut self, from: usize, to: usize) -> bool {
        let count = self.visible.len();
        if self.is_virtual() || from >= count || to >= count || from == to {
            return false;
        }
        let mut sequence = self.visible.clone();
        let moved = sequence.remove(from);
        sequence.insert(to, moved);
        let row = self.data[moved].clone();
        // Âncora: a linha visível que vem logo depois na nova ordem (ou a anterior).
        let anchor =
            sequence.get(to + 1).map(|&a| (a, true)).or_else(|| to.checked_sub(1).map(|p| (sequence[p], false)));
        let mut data: Vec<Option<Row>> = std::mem::take(&mut self.data).into_iter().map(Some).collect();
        data[moved] = None;
        let mut rebuilt = Vec::with_capacity(data.len());
        let mut row = Some(row);
        for (index, slot) in data.into_iter().enumerate() {
            if let Some((anchor, before)) = anchor {
                if index == anchor && before {
                    rebuilt.extend(row.take());
                }
                if let Some(existing) = slot {
                    rebuilt.push(existing);
                }
                if index == anchor && !before {
                    rebuilt.extend(row.take());
                }
            } else if let Some(existing) = slot {
                rebuilt.push(existing);
            }
        }
        rebuilt.extend(row);
        self.data = rebuilt;
        self.sort = None;
        self.rebuild();
        true
    }

    /// Moves public rows as a block in current order before public `slot` (`slot == row_count` = end). Returns the
    /// first row's new position.
    fn move_block(&mut self, rows: &[usize], slot: usize) -> Option<usize> {
        if self.is_virtual() || rows.is_empty() {
            return None;
        }
        let mut rows: Vec<usize> = rows.iter().copied().filter(|&r| r < self.visible.len()).collect();
        rows.sort_unstable();
        rows.dedup();
        if rows.is_empty() {
            return None;
        }
        let picked: HashSet<usize> = rows.iter().map(|&r| self.visible[r]).collect();
        // Âncora: primeira linha visível não movida a partir de `slot`.
        let anchor = (slot..self.visible.len()).map(|r| self.visible[r]).find(|i| !picked.contains(i));
        let block: Vec<Row> = rows.iter().map(|&r| self.data[self.visible[r]].clone()).collect();
        let mut rebuilt = Vec::with_capacity(self.data.len());
        let mut block = Some(block);
        for (index, existing) in std::mem::take(&mut self.data).into_iter().enumerate() {
            if Some(index) == anchor {
                rebuilt.extend(block.take().unwrap_or_default());
            }
            if !picked.contains(&index) {
                rebuilt.push(existing);
            }
        }
        rebuilt.extend(block.unwrap_or_default());
        self.data = rebuilt;
        self.sort = None;
        self.rebuild();
        let before = rows.iter().filter(|&&r| r < slot).count();
        Some(slot - before)
    }
}

/// Escapes a CSV field: double quotes when it contains separators, quotes or newlines.
fn csv_field(text: &str, separator: char) -> String {
    if text.contains(separator) || text.contains('"') || text.contains('\n') || text.contains('\r') {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_string()
    }
}

fn html_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(ch),
        }
    }
    out
}

fn color_css(color: Color) -> String {
    format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b)
}

/// Data grid for large volumes: only visible cells are queried.
///
/// Own implementation composed of a virtual ListView (`LVS_OWNERDATA`), Rust data model and overlaid editor.
///
/// Two data modes:
/// * **in memory** — [`set_rows`](Self::set_rows) / [`add_row`](Self::add_row); header-click sorting, double-click/F2
///   editing, filters ([`set_filter`](Self::set_filter)), column grouping ([`set_group_by`](Self::set_group_by)) and
///   row/cell styles;
/// * **virtual source** — [`set_virtual_source`](Self::set_virtual_source) takes a row count and an on-demand `(row,
///   column) -> text` function (millions of rows without memory overhead); editing is delegated to
///   [`set_virtual_editor`](Self::set_virtual_editor).
///
/// API row indices always refer to **displayed rows** after filtering/grouping; group headers do not count as rows.
///
/// ```no_run
/// # use arqrustui::prelude::*;
/// # fn demo(window: &Window) -> GuiResult<()> {
/// let grid = DataGrid::builder().bounds(10, 10, 500, 300).editable(true).parent(window).build()?;
/// grid.add_column("Product", 220, ColumnAlign::Left);
/// grid.add_column("Price", 100, ColumnAlign::Right);
/// grid.add_column("Active", 60, ColumnAlign::Center);
/// grid.set_column_kind(2, ColumnKind::CheckBox);
/// grid.add_row(&["Keyboard", "199,90", "true"]);
/// grid.set_cell_style(0, 1, CellStyle::new().foreground(Color::RED).bold());
/// grid.on_cell_edited(|row, column, text| println!("({row},{column}) = {text}"));
/// let csv = grid.to_csv(';');
/// # let _ = csv; Ok(()) }
/// ```
#[derive(Clone)]
pub struct DataGrid {
    pub(crate) core: Rc<WidgetCore>,
    model: Rc<RefCell<Model>>,
}

impl Widget for DataGrid {
    fn core(&self) -> &Rc<WidgetCore> {
        &self.core
    }
}

/// Builder for [`DataGrid`].
#[derive(Clone, Debug)]
pub struct DataGridBuilder {
    opts: ControlOptions,
    editable: bool,
    sortable: bool,
    single_selection: bool,
    pub(crate) kind: &'static str,
}

impl DataGridBuilder {
    builder_common!();

    /// Allows double-click or F2 cell editing (default: `false`).
    pub fn editable(mut self, editable: bool) -> Self {
        self.editable = editable;
        self
    }

    /// Sorts on column header clicks (default: `true`; in-memory mode only).
    pub fn sortable(mut self, sortable: bool) -> Self {
        self.sortable = sortable;
        self
    }

    /// Restricts selection to one row (default: multiple selection).
    pub fn single_selection(mut self, single: bool) -> Self {
        self.single_selection = single;
        self
    }

    /// Creates the grid.
    pub fn build(self) -> GuiResult<DataGrid> {
        // LVS_SHAREIMAGELISTS: a lista de imagens pertence ao wrapper `ImageList`, não ao controle.
        let mut style = LVS_REPORT | LVS_OWNERDATA | LVS_SHOWSELALWAYS | LVS_SHAREIMAGELISTS | WS_CLIPCHILDREN;
        if self.single_selection {
            style |= LVS_SINGLESEL;
        }
        let core = self.opts.create(self.kind, "SysListView32", style, WS_EX_CLIENTEDGE, true, None)?;
        native::send(
            core.hwnd(),
            LVM_SETEXTENDEDLISTVIEWSTYLE,
            0,
            (LVS_EX_FULLROWSELECT | LVS_EX_DOUBLEBUFFER | LVS_EX_GRIDLINES | LVS_EX_HEADERDRAGDROP) as isize,
        );
        let model =
            Rc::new(RefCell::new(Model { editable: self.editable, sortable: self.sortable, ..Default::default() }));
        let grid = DataGrid { core, model };
        let reflect = grid.clone();
        grid.core.set_reflect(move |core, reflected| reflect.on_notify(core, reflected));
        // Os ganchos do modelo podem capturar a própria grade; ao destruir o controle
        // eles são soltos para não formar um ciclo de referências.
        let cleanup = grid.model.clone();
        grid.core.on(EventKind::Destroyed, move |_| {
            let mut model = cleanup.borrow_mut();
            let released = (
                model.edit_hook.take(),
                model.validator.take(),
                std::mem::take(&mut model.edited),
                std::mem::take(&mut model.group_toggled),
                model.editor.take(),
                model.source.take(),
                model.icon_provider.take(),
                model.style_provider.take(),
                model.filter.take(),
                model.virtual_editor.take(),
            );
            drop(model);
            drop(released);
        });
        let keys = grid.clone();
        grid.core.on(EventKind::KeyDown, move |event| {
            if let Event::KeyDown(key) = event {
                keys.on_key(key.key);
            }
        });
        // O editor acompanha a rolagem e o redimensionamento das colunas.
        let weak = Rc::downgrade(&grid.model);
        let core = Rc::downgrade(&grid.core);
        grid.core.add_message_filter(move |raw: &RawMessage| {
            let follows = match raw.message {
                WM_VSCROLL | WM_HSCROLL | WM_MOUSEWHEEL | WM_MOUSEHWHEEL | WM_SIZE => true,
                WM_NOTIFY if raw.lparam != 0 => {
                    // SAFETY: em WM_NOTIFY o LPARAM aponta para um NMHDR válido.
                    let code = unsafe { (*(raw.lparam as *const NMHDR)).code } as i32;
                    matches!(code, HDN_ITEMCHANGEDW | HDN_ENDTRACKW | HDN_TRACKW | HDN_DIVIDERDBLCLICKW)
                }
                _ => false,
            };
            if follows && weak.upgrade().is_some_and(|m| m.borrow().editor.is_some()) {
                let (weak, core) = (weak.clone(), core.clone());
                let _ = crate::core::defer(move || {
                    if let (Some(model), Some(core)) = (weak.upgrade(), core.upgrade()) {
                        DataGrid { core, model }.place_editor();
                    }
                });
            }
            None
        });
        Ok(grid)
    }
}

impl DataGrid {
    /// Starts building (default size 400×240).
    pub fn builder() -> DataGridBuilder {
        DataGridBuilder {
            opts: ControlOptions::new(400, 240),
            editable: false,
            sortable: true,
            single_selection: false,
            kind: "DataGrid",
        }
    }

    fn cell_value(&self, row: usize, column: usize) -> String {
        let provider = {
            let model = self.model.borrow();
            match &model.source {
                Some((_, provider)) => provider.clone(),
                None => return model.cell(row, column),
            }
        };
        provider(row, column)
    }

    fn on_key(&self, key: crate::input::Key) {
        use crate::input::Key;
        let focused = native::send(self.core.hwnd(), LVM_GETNEXTITEM, usize::MAX, LVNI_FOCUSED as isize);
        let group =
            (focused >= 0).then(|| self.model.borrow().item(focused as usize)).flatten().and_then(|item| match item {
                Item::Group(g) => Some(g),
                Item::Row(_) => None,
            });
        if let Some(group) = group {
            let collapsed = self.model.borrow().groups.get(group).map(|g| g.collapsed).unwrap_or(false);
            match key {
                k if k == Key::SPACE || k == Key::ENTER => self.toggle_group(group),
                k if k == Key::LEFT && !collapsed => self.toggle_group(group),
                k if k == Key::RIGHT && collapsed => self.toggle_group(group),
                _ => {}
            }
            return;
        }
        if key == Key::F2 {
            if let Some(row) = self.selected_rows().first() {
                let column = (0..self.column_count()).find(|&c| !self.model.borrow().read_only_columns.contains(&c));
                self.begin_edit(*row, column.unwrap_or(0));
            }
        } else if key == Key::SPACE {
            let column = (0..self.column_count()).find(|&c| self.model.borrow().kind(c) == ColumnKind::CheckBox);
            if let Some(column) = column {
                for row in self.selected_rows() {
                    self.toggle_check(row, column);
                }
            }
        }
    }

    fn on_notify(&self, core: &Rc<WidgetCore>, reflected: &Reflected) -> Option<isize> {
        let Reflected::Notify { code, lparam } = reflected else { return None };
        match *code {
            LVN_GETDISPINFOW => {
                // SAFETY: nesta notificação o LPARAM aponta para um NMLVDISPINFOW gravável;
                // `pszText` aponta para um buffer de `cchTextMax` unidades fornecido pelo controle.
                let info = unsafe { &mut *(*lparam as *mut NMLVDISPINFOW) };
                let item = info.item.iItem.max(0) as usize;
                let column = info.item.iSubItem.max(0) as usize;
                let (target, kind) = {
                    let model = self.model.borrow();
                    (model.item(item), model.kind(column))
                };
                if info.item.mask & LVIF_IMAGE != 0 {
                    info.item.iImage = match target {
                        Some(Item::Row(row)) if column == 0 => self.row_image_index(core, row),
                        _ => I_IMAGENONE,
                    };
                }
                if info.item.mask & LVIF_TEXT != 0 && !info.item.pszText.is_null() && info.item.cchTextMax > 0 {
                    let text = match target {
                        Some(Item::Row(_)) if kind == ColumnKind::CheckBox => String::new(),
                        Some(Item::Row(row)) => self.cell_value(row, column),
                        Some(Item::Group(group)) if column == 0 => self.group_caption(group),
                        _ => String::new(),
                    };
                    // SAFETY: o slice cobre exatamente o buffer descrito acima.
                    let buffer =
                        unsafe { std::slice::from_raw_parts_mut(info.item.pszText, info.item.cchTextMax as usize) };
                    crate::utilities::strings::copy_to_buffer(&text, buffer);
                }
                Some(0)
            }
            NM_CUSTOMDRAW => self.custom_draw(*lparam),
            LVN_ENDSCROLL => {
                self.place_editor();
                None
            }
            LVN_COLUMNCLICK => {
                // SAFETY: nesta notificação o LPARAM aponta para um NMLISTVIEW válido.
                let column = unsafe { (*(*lparam as *const NMLISTVIEW)).iSubItem }.max(0) as usize;
                let toggle = {
                    let model = self.model.borrow();
                    (model.sortable && model.source.is_none())
                        .then(|| !matches!(model.sort, Some((c, true)) if c == column))
                };
                if let Some(ascending) = toggle {
                    self.sort_by(column, ascending);
                }
                core.emit(&Event::ColumnClick(column));
                Some(0)
            }
            NM_CLICK => {
                if let Some((item, column)) = self.hit_cursor() {
                    let target = self.model.borrow().item(item);
                    match target {
                        Some(Item::Group(group)) => self.toggle_group(group),
                        Some(Item::Row(row)) if self.model.borrow().kind(column) == ColumnKind::CheckBox => {
                            self.toggle_check(row, column);
                        }
                        _ => {}
                    }
                }
                None
            }
            NM_DBLCLK => {
                core.emit(&Event::DoubleClick);
                if let Some((item, column)) = self.hit_cursor() {
                    let (target, kind) = {
                        let model = self.model.borrow();
                        (model.item(item), model.kind(column))
                    };
                    if let Some(Item::Row(row)) = target {
                        if kind != ColumnKind::CheckBox {
                            self.begin_edit(row, column);
                        }
                    }
                }
                Some(0)
            }
            LVN_ITEMACTIVATE => {
                // SAFETY: nesta notificação o LPARAM aponta para uma estrutura iniciada por NMLISTVIEW.
                let item = unsafe { (*(*lparam as *const NMLISTVIEW)).iItem };
                let row = (item >= 0).then(|| self.model.borrow().item_to_row(item as usize)).flatten();
                if let Some(row) = row {
                    core.emit(&Event::ItemActivated(row));
                }
                Some(0)
            }
            _ => listview::reflect_list_view(core, reflected),
        }
    }

    /// Item and column under the cursor (grid client pixels).
    fn hit_cursor(&self) -> Option<(usize, usize)> {
        let hwnd = self.core.hwnd();
        let mut point = POINT { x: 0, y: 0 };
        // SAFETY: ponteiro para um POINT local; janela viva desta thread.
        unsafe {
            GetCursorPos(&mut point);
            ScreenToClient(hwnd, &mut point);
        }
        self.hit_pixels(point)
    }

    fn hit_pixels(&self, point: POINT) -> Option<(usize, usize)> {
        let mut info = LVHITTESTINFO { pt: point, ..LVHITTESTINFO::default() };
        let item = native::send_mut(self.core.hwnd(), LVM_SUBITEMHITTEST, 0, &mut info);
        (item >= 0 && info.iItem >= 0).then(|| (info.iItem as usize, info.iSubItem.max(0) as usize))
    }

    fn group_caption(&self, group: usize) -> String {
        let model = self.model.borrow();
        let Some(g) = model.groups.get(group) else { return String::new() };
        let key = if g.key.is_empty() { library_text(LibraryText::EmptyGroup) } else { g.key.clone() };
        format!("{} {key} ({})", if g.collapsed { "▸" } else { "▾" }, g.count)
    }

    fn refresh(&self) {
        let hwnd = self.core.hwnd();
        let count = self.model.borrow().item_count();
        native::send(hwnd, LVM_SETITEMCOUNT, count, 0);
        if count > 0 {
            native::send(hwnd, LVM_REDRAWITEMS, 0, (count - 1) as isize);
        }
        self.core.invalidate();
    }

    fn redraw_row(&self, row: usize) {
        if let Some(item) = self.model.borrow().row_to_item(row) {
            native::send(self.core.hwnd(), LVM_REDRAWITEMS, item, item as isize);
        }
    }

    /// Adds a column (logical width) and returns its index.
    pub fn add_column(&self, title: &str, width: i32, align: ColumnAlign) -> usize {
        let index = listview::insert_column(&self.core, self.column_count(), title, width, align);
        let mut model = self.model.borrow_mut();
        if model.columns.len() <= index {
            model.columns.resize(index + 1, String::new());
        }
        model.columns[index] = title.to_string();
        index
    }

    /// Number of columns.
    pub fn column_count(&self) -> usize {
        listview::header_count(self.core.hwnd())
    }

    /// Column title.
    pub fn column_title(&self, column: usize) -> Option<String> {
        self.model.borrow().columns.get(column).cloned()
    }

    /// Logical column width (users can drag to resize).
    pub fn column_width(&self, column: usize) -> i32 {
        to_logical(native::send(self.core.hwnd(), LVM_GETCOLUMNWIDTH, column, 0) as i32, self.core.dpi())
    }

    /// Sets logical column width.
    pub fn set_column_width(&self, column: usize, width: i32) {
        native::send(self.core.hwnd(), LVM_SETCOLUMNWIDTH, column, to_pixels(width, self.core.dpi()) as isize);
        self.place_editor();
    }

    /// Prevents or re-enables column editing.
    pub fn set_column_read_only(&self, column: usize, read_only: bool) {
        let mut model = self.model.borrow_mut();
        if read_only {
            model.read_only_columns.insert(column);
        } else {
            model.read_only_columns.remove(&column);
        }
    }

    /// Sets column value display/editing (text, check box or list).
    pub fn set_column_kind(&self, column: usize, kind: ColumnKind) {
        {
            let mut model = self.model.borrow_mut();
            if kind == ColumnKind::Text {
                model.kinds.remove(&column);
            } else {
                model.kinds.insert(column, kind);
            }
        }
        self.refresh();
    }

    /// Column type.
    pub fn column_kind(&self, column: usize) -> ColumnKind {
        self.model.borrow().kind(column)
    }

    /// Replaces all rows (in-memory mode).
    pub fn set_rows(&self, rows: Vec<Vec<String>>) {
        self.cancel_edit();
        {
            let mut model = self.model.borrow_mut();
            model.data = rows.into_iter().map(Row::new).collect();
            model.source = None;
            model.sort = None;
            model.rebuild();
        }
        self.update_sort_arrow();
        self.refresh();
    }

    /// Appends a row (in-memory mode) and returns its displayed index (`None` if hidden by the active filter).
    pub fn add_row(&self, cells: &[&str]) -> usize {
        let index = {
            let mut model = self.model.borrow_mut();
            model.data.push(Row::new(cells.iter().map(|c| c.to_string()).collect()));
            let data_index = model.data.len() - 1;
            model.rebuild();
            model.visible.iter().position(|&i| i == data_index).unwrap_or(model.visible.len())
        };
        self.refresh();
        index
    }

    /// Removes a row (in-memory mode).
    pub fn remove_row(&self, row: usize) {
        {
            let mut model = self.model.borrow_mut();
            if let Some(&index) = model.visible.get(row) {
                model.data.remove(index);
                model.rebuild();
            }
        }
        self.refresh();
    }

    /// Removes all rows.
    pub fn clear(&self) {
        self.cancel_edit();
        {
            let mut model = self.model.borrow_mut();
            model.data.clear();
            model.source = None;
            model.rebuild();
        }
        self.refresh();
    }

    /// Uses a virtual source: `row_count` rows queried on demand with `provider(row, column)`. The application handles
    /// sorting, filtering and grouping; [`set_virtual_editor`](Self::set_virtual_editor) handles editing.
    pub fn set_virtual_source(&self, row_count: usize, provider: impl Fn(usize, usize) -> String + 'static) {
        self.cancel_edit();
        {
            let mut model = self.model.borrow_mut();
            model.data.clear();
            model.sort = None;
            model.source = Some((row_count, Rc::new(provider)));
            model.rebuild();
        }
        self.update_sort_arrow();
        self.refresh();
    }

    /// Changes virtual row count without replacing the function and redraws.
    pub fn set_virtual_row_count(&self, row_count: usize) {
        if let Some((count, _)) = self.model.borrow_mut().source.as_mut() {
            *count = row_count;
        }
        self.refresh();
    }

    /// Enables virtual source editing: `editor(row, column, text)` writes to the application's source and returns
    /// `false` to reject. Enables double clicks, F2, check boxes and lists in virtual mode if the grid is `editable`.
    pub fn set_virtual_editor(&self, editor: impl Fn(usize, usize, &str) -> bool + 'static) {
        self.model.borrow_mut().virtual_editor = Some(Rc::new(editor));
    }

    /// Displayed row count after filtering, including collapsed groups.
    pub fn row_count(&self) -> usize {
        self.model.borrow().row_count()
    }

    /// Total in-memory row count, including filtered-out rows.
    pub fn total_row_count(&self) -> usize {
        let model = self.model.borrow();
        if model.is_virtual() {
            model.row_count()
        } else {
            model.data.len()
        }
    }

    /// Cell text (`None` out of bounds).
    pub fn cell(&self, row: usize, column: usize) -> Option<String> {
        if row >= self.row_count() || column >= self.column_count() {
            return None;
        }
        Some(self.cell_value(row, column))
    }

    /// Sets cell text (in-memory mode). Does not fire `on_cell_edited` or reapply filters (use
    /// [`refresh_view`](Self::refresh_view)).
    pub fn set_cell(&self, row: usize, column: usize, text: &str) {
        {
            let mut model = self.model.borrow_mut();
            let Some(r) = model.row_mut(row) else { return };
            if r.cells.len() <= column {
                r.cells.resize(column + 1, String::new());
            }
            r.cells[column] = text.to_string();
        }
        self.redraw_row(row);
    }

    /// Copy of displayed rows in display order (in-memory mode).
    pub fn rows(&self) -> Vec<Vec<String>> {
        let model = self.model.borrow();
        model.visible.iter().map(|&i| model.data[i].cells.clone()).collect()
    }

    /// Copy of all in-memory rows, including filtered-out rows.
    pub fn all_rows(&self) -> Vec<Vec<String>> {
        self.model.borrow().data.iter().map(|r| r.cells.clone()).collect()
    }

    /// Sorts in-memory rows by column; numbers are compared numerically.
    pub fn sort_by(&self, column: usize, ascending: bool) {
        {
            let mut model = self.model.borrow_mut();
            if model.source.is_some() {
                return;
            }
            let blank = String::new();
            let mut rows = std::mem::take(&mut model.data);
            rows.sort_by(|a, b| {
                let ordering = listview::natural_compare(
                    a.cells.get(column).unwrap_or(&blank),
                    b.cells.get(column).unwrap_or(&blank),
                );
                if ascending {
                    ordering
                } else {
                    ordering.reverse()
                }
            });
            model.data = rows;
            model.sort = Some((column, ascending));
            model.rebuild();
        }
        self.update_sort_arrow();
        self.refresh();
    }

    /// Current sort column and direction, if any.
    pub fn sort_state(&self) -> Option<(usize, bool)> {
        self.model.borrow().sort
    }

    fn update_sort_arrow(&self) {
        let header = native::send(self.core.hwnd(), LVM_GETHEADER, 0, 0) as HWND;
        let sort = self.model.borrow().sort;
        for column in 0..self.column_count() {
            let mut item = HDITEMW {
                mask: HDI_FORMAT,
                cxy: 0,
                pszText: std::ptr::null_mut(),
                hbm: std::ptr::null_mut(),
                cchTextMax: 0,
                fmt: 0,
                lParam: 0,
                iImage: 0,
                iOrder: 0,
                r#type: 0,
                pvFilter: std::ptr::null_mut(),
                state: 0,
            };
            native::send_mut(header, HDM_GETITEMW, column, &mut item);
            item.fmt &= !(HDF_SORTUP | HDF_SORTDOWN);
            if let Some((sorted, ascending)) = sort {
                if sorted == column {
                    item.fmt |= if ascending { HDF_SORTUP } else { HDF_SORTDOWN };
                }
            }
            native::send_ref(header, HDM_SETITEMW, column, &item);
        }
    }

    // ------------------------------------------------------------------
    // Seleção
    // ------------------------------------------------------------------

    /// Selected row indices, excluding group headers.
    pub fn selected_rows(&self) -> Vec<usize> {
        let items = listview::selected_rows(self.core.hwnd());
        let model = self.model.borrow();
        items.into_iter().filter_map(|item| model.item_to_row(item)).collect()
    }

    /// Selects or deselects a row.
    pub fn set_row_selected(&self, row: usize, selected: bool) {
        let item = if row == usize::MAX { Some(usize::MAX) } else { self.model.borrow().row_to_item(row) };
        if let Some(item) = item {
            listview::set_row_selected(self.core.hwnd(), item, selected);
        }
    }

    /// Selection changed; receives selected rows.
    pub fn on_selection_changed(&self, f: impl Fn(Vec<usize>) + 'static) -> HandlerId {
        let this = self.clone();
        self.core.on(EventKind::SelectionChanged, move |_| f(this.selected_rows()))
    }

    /// A column header was clicked; receives its index.
    pub fn on_column_click(&self, f: impl Fn(usize) + 'static) -> HandlerId {
        self.core.on(EventKind::ColumnClick, move |e| {
            if let Event::ColumnClick(column) = e {
                f(*column)
            }
        })
    }

    /// A row was activated (double click or Enter); receives the row.
    pub fn on_row_activated(&self, f: impl Fn(usize) + 'static) -> HandlerId {
        self.core.on(EventKind::ItemActivated, move |e| {
            if let Event::ItemActivated(row) = e {
                f(*row)
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(rows: &[&[&str]]) -> Model {
        let mut model = Model {
            data: rows.iter().map(|r| Row::new(r.iter().map(|c| c.to_string()).collect())).collect(),
            ..Model::default()
        };
        model.rebuild();
        model
    }

    fn keys(model: &Model) -> Vec<String> {
        (0..model.row_count()).map(|r| model.cell(r, 0)).collect()
    }

    #[test]
    fn filter_and_groups_map_rows_and_items() {
        let mut m = model(&[&["a", "X"], &["b", "Y"], &["c", "X"], &["d", "Z"]]);
        assert_eq!(m.item_count(), 4);
        m.filter = Some(Rc::new(|cells: &[String]| cells[1] != "Z"));
        m.rebuild();
        assert_eq!(keys(&m), ["a", "b", "c"]);
        m.group_by = Some(1);
        m.rebuild();
        // Grupos na ordem de aparição; linhas públicas na ordem exibida.
        assert_eq!(keys(&m), ["a", "c", "b"]);
        assert_eq!(m.items, [Item::Group(0), Item::Row(0), Item::Row(1), Item::Group(1), Item::Row(2)]);
        assert_eq!((m.item_to_row(0), m.item_to_row(2), m.item_to_row(4)), (None, Some(1), Some(2)));
        assert_eq!(m.row_to_item(2), Some(4));
        m.collapsed.insert("X".into());
        m.rebuild();
        assert_eq!(m.items, [Item::Group(0), Item::Group(1), Item::Row(2)]);
        assert_eq!(m.row_count(), 3, "linhas de grupos recolhidos continuam contando");
        assert_eq!((m.row_to_item(0), m.row_to_item(2)), (None, Some(2)));
        assert_eq!(m.groups[0], Group { key: "X".into(), count: 2, collapsed: true });
    }

    #[test]
    fn moving_rows_keeps_hidden_rows_in_place() {
        let mut m = model(&[&["a"], &["b"], &["c"], &["d"], &["e"]]);
        assert!(m.move_visible(0, 3));
        assert_eq!(keys(&m), ["b", "c", "d", "a", "e"]);
        m.filter = Some(Rc::new(|cells: &[String]| cells[0] != "c"));
        m.rebuild();
        assert_eq!(keys(&m), ["b", "d", "a", "e"]);
        assert!(m.move_visible(3, 0));
        m.filter = None;
        m.rebuild();
        assert_eq!(keys(&m), ["e", "b", "c", "d", "a"]);
        assert!(!m.move_visible(1, 1));
    }

    #[test]
    fn block_moves_keep_relative_order() {
        let mut m = model(&[&["a"], &["b"], &["c"], &["d"], &["e"]]);
        // b e d para antes de "a".
        assert_eq!(m.move_block(&[3, 1], 0), Some(0));
        assert_eq!(keys(&m), ["b", "d", "a", "c", "e"]);
        // b e d (agora 0 e 1) para o fim.
        assert_eq!(m.move_block(&[0, 1], 5), Some(3));
        assert_eq!(keys(&m), ["a", "c", "e", "b", "d"]);
        // a e e para antes de "b" (posição 3).
        assert_eq!(m.move_block(&[0, 2], 3), Some(1));
        assert_eq!(keys(&m), ["c", "a", "e", "b", "d"]);
    }

    #[test]
    fn styles_merge_and_checks_parse() {
        let mut m = model(&[&["a", "1"]]);
        m.data[0].style = Some(CellStyle::new().background(Color::YELLOW).italic());
        m.data[0].cell_styles.insert(1, CellStyle::new().foreground(Color::RED).background(Color::WHITE));
        assert_eq!(m.static_style(0, 0), CellStyle::new().background(Color::YELLOW).italic());
        let cell = m.static_style(0, 1);
        assert_eq!((cell.foreground, cell.background, cell.italic), (Some(Color::RED), Some(Color::WHITE), true));
        assert!(m.has_styles());
        assert!(parse_check(" Sim ") && parse_check("TRUE") && parse_check("1") && !parse_check("false"));
        assert!(!parse_check(""));
    }

    #[test]
    fn export_escapes_fields() {
        assert_eq!(csv_field("a;b", ';'), "\"a;b\"");
        assert_eq!(csv_field("di\"z", ','), "\"di\"\"z\"");
        assert_eq!(csv_field("simples", ','), "simples");
        assert_eq!(csv_field("linha\nnova", ','), "\"linha\nnova\"");
        assert_eq!(html_escape("<b>&\""), "&lt;b&gt;&amp;&quot;");
        assert_eq!(color_css(Color::rgb(255, 0, 16)), "#ff0010");
    }
}
