//! Combo box.

use super::common::{builder_common, simple_event, widget_type, ControlOptions};
use super::icon_support::{self, ListMessages};
use crate::core::handles::IconSource;
use crate::core::native;
use crate::core::widget::{Reflected, WidgetCore};
use crate::core::{Event, EventKind, GuiResult, HandlerId};
use crate::ffi::user32::*;
use crate::ffi::{HWND, POINT, RECT};
use crate::utilities::to_pixels;

widget_type!(
    /// Combo box (drop-down list, optionally editable).
    ///
    /// ```no_run
    /// # use arqrustui::prelude::*;
    /// # fn demo(window: &Window) -> GuiResult<()> {
    /// let combo = ComboBox::builder().items(&["Small", "Medium", "Large"]).selected(1).parent(window).build()?;
    /// combo.on_selection_changed(|index| println!("selected: {index:?}"));
    /// # Ok(()) }
    /// ```
    ComboBox
);

/// Builder for [`ComboBox`].
#[derive(Clone, Debug)]
pub struct ComboBoxBuilder {
    opts: ControlOptions,
    editable: bool,
    sorted: bool,
    items: Vec<String>,
    selected: Option<usize>,
    icons: bool,
}

impl ComboBoxBuilder {
    builder_common!();

    /// Allows typing a value as well as selecting from the list.
    pub fn editable(mut self, editable: bool) -> Self {
        self.editable = editable;
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

    /// Initially selected index.
    pub fn selected(mut self, index: usize) -> Self {
        self.selected = Some(index);
        self
    }

    /// Icons in an editable combo box. Noneditable boxes are already drawn by the library and support
    /// [`ComboBox::add_item_with_icon`] and [`ComboBox::set_item_icon`] without this option.
    pub fn with_icons(mut self, icons: bool) -> Self {
        self.icons = icons;
        self
    }

    /// Creates the combo box.
    pub fn build(self) -> GuiResult<ComboBox> {
        let mut style = CBS_HASSTRINGS | WS_VSCROLL;
        // Listas não editáveis são sempre desenhadas pela biblioteca: o texto fica
        // centralizado em qualquer altura (o controle nativo o desenha no alto do campo)
        // e os itens aceitam ícones.
        if self.icons || !self.editable {
            style |= CBS_OWNERDRAWFIXED;
        }
        style |= if self.editable { CBS_DROPDOWN | CBS_AUTOHSCROLL } else { CBS_DROPDOWNLIST };
        if self.sorted {
            style |= CBS_SORT;
        }
        let core = self.opts.create("ComboBox", "COMBOBOX", style, 0, true, None)?;
        core.set_reflect(|core, reflected| match reflected {
            Reflected::Command { code, .. } if *code == CBN_SELCHANGE => {
                core.emit(&Event::SelectionChanged);
                core.emit(&Event::TextChanged);
                Some(0)
            }
            Reflected::Command { code, .. } if *code == CBN_EDITCHANGE => {
                core.emit(&Event::TextChanged);
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
        // Ajusta itens e campo de seleção à fonte (refeito a cada troca de fonte e de DPI).
        fit_field_height(&core);
        let combo = ComboBox { core };
        for item in &self.items {
            combo.add_item(item);
        }
        if let Some(index) = self.selected {
            combo.set_selected_index(Some(index));
        }
        Ok(combo)
    }
}

impl ComboBox {
    /// Starts building a combo box (default size 160 × [`DEFAULT_CONTROL_HEIGHT`](super::DEFAULT_CONTROL_HEIGHT), the
    /// same height as [`Button`](super::Button)).
    ///
    /// Height applies to the closed box; Windows calculates the drop-down list's separate height from the number of
    /// items.
    pub fn builder() -> ComboBoxBuilder {
        ComboBoxBuilder {
            opts: ControlOptions::new(160, super::DEFAULT_CONTROL_HEIGHT),
            editable: false,
            sorted: false,
            items: Vec::new(),
            selected: None,
            icons: false,
        }
    }

    /// Adds an item with an icon and returns its resulting index.
    ///
    /// Editable boxes require [`with_icons(true)`](ComboBoxBuilder::with_icons).
    pub fn add_item_with_icon(&self, text: &str, icon: &impl IconSource) -> GuiResult<usize> {
        icon_support::require_owner_drawn(&self.core, CBS_OWNERDRAWFIXED)?;
        let index = self.add_item(text);
        self.set_item_icon(index, icon)?;
        Ok(index)
    }

    /// Shows an icon on an item (editable boxes require [`with_icons(true)`](ComboBoxBuilder::with_icons)).
    pub fn set_item_icon(&self, index: usize, icon: &impl IconSource) -> GuiResult<()> {
        icon_support::set_item_icon(&self.core, CBS_OWNERDRAWFIXED, MESSAGES, index, Some(&icon))
    }

    /// Removes an item's icon.
    pub fn clear_item_icon(&self, index: usize) -> GuiResult<()> {
        icon_support::set_item_icon(&self.core, CBS_OWNERDRAWFIXED, MESSAGES, index, None)
    }

    /// Item icon index in the control's image list (`None` if absent).
    pub fn item_image(&self, index: usize) -> Option<i32> {
        icon_support::item_image(&self.core, MESSAGES, index)
    }

    /// Adds an item and returns its resulting index.
    pub fn add_item(&self, text: &str) -> usize {
        native::send_text(self.core.hwnd(), CB_ADDSTRING, 0, text).max(0) as usize
    }

    /// Inserts an item at the given position.
    pub fn insert_item(&self, index: usize, text: &str) {
        native::send_text(self.core.hwnd(), CB_INSERTSTRING, index, text);
    }

    /// Removes the item at the given position.
    pub fn remove_item(&self, index: usize) {
        native::send(self.core.hwnd(), CB_DELETESTRING, index, 0);
    }

    /// Removes all items.
    pub fn clear(&self) {
        native::send(self.core.hwnd(), CB_RESETCONTENT, 0, 0);
    }

    /// Number of items.
    pub fn count(&self) -> usize {
        native::send(self.core.hwnd(), CB_GETCOUNT, 0, 0).max(0) as usize
    }

    /// Text of the item at the given position.
    pub fn item_text(&self, index: usize) -> Option<String> {
        if index >= self.count() {
            return None;
        }
        native::read_text(self.core.hwnd(), CB_GETLBTEXTLEN, CB_GETLBTEXT, index)
    }

    /// Selected index, if any.
    pub fn selected_index(&self) -> Option<usize> {
        let index = native::send(self.core.hwnd(), CB_GETCURSEL, 0, 0);
        (index >= 0).then_some(index as usize)
    }

    /// Selects an item programmatically (`None` clears selection). Does not fire events.
    pub fn set_selected_index(&self, index: Option<usize>) {
        native::send(self.core.hwnd(), CB_SETCURSEL, index.unwrap_or(usize::MAX), 0);
    }

    /// Selected item text.
    pub fn selected_text(&self) -> Option<String> {
        self.selected_index().and_then(|index| self.item_text(index))
    }

    /// The user selected another item; receives the new index.
    pub fn on_selection_changed(&self, f: impl Fn(Option<usize>) + 'static) -> HandlerId {
        let this = self.clone();
        self.core.on(EventKind::SelectionChanged, move |_| f(this.selected_index()))
    }

    simple_event!(
        /// Text changed (typing in editable boxes or a new selection).
        on_text_changed,
        TextChanged
    );
}

/// Adjusts the selection field so the closed box has exactly the requested logical height.
///
/// The native ComboBox ignores window height: its closed height comes from the font (selection field height plus
/// borders). Height is therefore corrected through `CB_SETITEMHEIGHT(-1)` after size, font or DPI changes. The border
/// difference between closed height and field height is measured on the control, supporting any theme and Common
/// Controls version. The field never becomes shorter than its text or icon.
pub(crate) fn fit_field_height(core: &WidgetCore) {
    let hwnd = core.hwnd();
    let Some(bounds) = core.logical_bounds() else { return };
    if hwnd.is_null() {
        return;
    }
    // Itens desenhados pela biblioteca: a altura deles acompanha a fonte atual.
    if native::style(hwnd) & CBS_OWNERDRAWFIXED != 0 {
        icon_support::fit_item_height(core, MESSAGES);
    }
    let dpi = core.dpi();
    let target = to_pixels(bounds.height, dpi);
    let mut minimum = icon_support::text_height(hwnd);
    if native::style(hwnd) & CBS_OWNERDRAWFIXED != 0 {
        minimum = minimum.max(to_pixels(icon_support::DEFAULT_ICON_SIZE, dpi));
    }
    // O valor lido com CB_GETITEMHEIGHT nem sempre é o que foi gravado (o Wine, por
    // exemplo, soma a margem interna), então a correção parte sempre do último valor
    // pedido e da altura fechada medida em seguida; converge em uma ou duas passadas.
    let mut requested = native::send(hwnd, CB_GETITEMHEIGHT, usize::MAX, 0) as i32;
    if requested <= 0 {
        return;
    }
    for _ in 0..3 {
        let closed = native::window_rect(hwnd);
        let closed = closed.bottom - closed.top;
        let wanted = (requested + target - closed).max(minimum);
        if closed == target || wanted == requested {
            break;
        }
        requested = wanted;
        native::send(hwnd, CB_SETITEMHEIGHT, usize::MAX, requested as isize);
    }
    center_edit(hwnd);
}

/// Vertically centers the editable box's internal text field (the native control places it at the top). Repeated after
/// every height adjustment, when the native control repositions it.
fn center_edit(hwnd: HWND) {
    if native::style(hwnd) & 0x3 != CBS_DROPDOWN {
        return;
    }
    let mut info = COMBOBOXINFO {
        cbSize: std::mem::size_of::<COMBOBOXINFO>() as u32,
        rcItem: RECT::default(),
        rcButton: RECT::default(),
        stateButton: 0,
        hwndCombo: std::ptr::null_mut(),
        hwndItem: std::ptr::null_mut(),
        hwndList: std::ptr::null_mut(),
    };
    // SAFETY: estrutura completa com o tamanho preenchido; a caixa interna é filha
    // viva da ComboBox e é só reposicionada.
    unsafe {
        if GetComboBoxInfo(hwnd, &mut info) == 0 || info.hwndItem.is_null() {
            return;
        }
        let mut edit = RECT::default();
        GetWindowRect(info.hwndItem, &mut edit);
        let mut origin = POINT { x: edit.left, y: edit.top };
        ScreenToClient(hwnd, &mut origin);
        let field = info.rcItem;
        let line = (icon_support::text_height(info.hwndItem) + 2).min(field.bottom - field.top);
        let top = field.top + (field.bottom - field.top - line) / 2;
        if origin.y != top || edit.bottom - edit.top != line {
            MoveWindow(info.hwndItem, origin.x, top, edit.right - edit.left, line, 1);
        }
    }
}

const MESSAGES: ListMessages = ListMessages {
    text_length: CB_GETLBTEXTLEN,
    text: CB_GETLBTEXT,
    set_data: CB_SETITEMDATA,
    get_data: CB_GETITEMDATA,
    set_height: CB_SETITEMHEIGHT,
    selection_field: true,
};
