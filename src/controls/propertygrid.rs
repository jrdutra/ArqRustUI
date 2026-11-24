//! Typed property grid (own implementation over [`DataGrid`]).

use super::datagrid::{DataGrid, EditAction};
use super::listview::ColumnAlign;
use crate::accessibility::tree::{role, Action, Node, TreeHost, VirtualTree};
use crate::core::native;
use crate::core::widget::{Widget, WidgetCore};
use crate::core::GuiResult;
use crate::ffi::comctl32::{LVIR_BOUNDS, LVM_GETHEADER, LVM_GETITEMRECT};
use crate::ffi::user32::{ClientToScreen, GetFocus, SetFocus};
use crate::ffi::{HWND, POINT, RECT};
use crate::i18n::{library_text, LibraryText};
use std::cell::RefCell;
use std::rc::Rc;

/// Typed property value.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum PropertyValue {
    /// Free text.
    Text(String),
    /// Integer (entered text is validated).
    Integer(i64),
    /// Floating-point number (accepts decimal `,` or `.`).
    Float(f64),
    /// Boolean; toggled by double click or Space.
    Bool(bool),
    /// One of several options; opens a drop-down or advances with [`ChoiceMode::Cycle`].
    Choice {
        /// Available options.
        options: Vec<String>,
        /// Selected option index.
        selected: usize,
    },
}

/// How [`PropertyGrid`] edits [`PropertyValue::Choice`] properties.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum ChoiceMode {
    /// Drop-down over the cell (default).
    #[default]
    DropDown,
    /// Each activation advances to the next option (previous behavior).
    Cycle,
}

impl PropertyValue {
    /// Displayed text with given boolean labels (`None` = library text).
    fn display_with(&self, labels: Option<&(String, String)>) -> String {
        match self {
            PropertyValue::Text(text) => text.clone(),
            PropertyValue::Integer(value) => value.to_string(),
            PropertyValue::Float(value) => value.to_string(),
            PropertyValue::Bool(value) => match labels {
                Some((yes, no)) => {
                    if *value {
                        yes.clone()
                    } else {
                        no.clone()
                    }
                }
                None => crate::i18n::library_text(if *value {
                    crate::i18n::LibraryText::Yes
                } else {
                    crate::i18n::LibraryText::No
                }),
            },
            PropertyValue::Choice { options, selected } => options.get(*selected).cloned().unwrap_or_default(),
        }
    }

    /// Parses input as the same type as the current value.
    fn parse_like(&self, text: &str) -> Option<PropertyValue> {
        match self {
            PropertyValue::Text(_) => Some(PropertyValue::Text(text.to_string())),
            PropertyValue::Integer(_) => text.trim().parse().ok().map(PropertyValue::Integer),
            PropertyValue::Float(_) => text.trim().replace(',', ".").parse().ok().map(PropertyValue::Float),
            PropertyValue::Choice { options, .. } => options
                .iter()
                .position(|o| o == text)
                .map(|selected| PropertyValue::Choice { options: options.clone(), selected }),
            PropertyValue::Bool(_) => None,
        }
    }
}

struct Entry {
    name: String,
    /// `None` marks a category row.
    value: Option<PropertyValue>,
}

type ChangedFn = Rc<dyn Fn(&str, &PropertyValue)>;

struct State {
    entries: Vec<Entry>,
    listeners: Vec<ChangedFn>,
    bool_labels: Option<(String, String)>,
    choice_mode: ChoiceMode,
}

impl State {
    fn display(&self, value: &PropertyValue) -> String {
        value.display_with(self.bool_labels.as_ref())
    }
}

/// Property grid: name/value pairs with categories, in-place editing and type validation.
///
/// Windows has no equivalent control; built on the library's [`DataGrid`].
///
/// ```no_run
/// # use arqrustui::prelude::*;
/// # fn demo(window: &Window) -> GuiResult<()> {
/// let grid = PropertyGrid::new(window, Rect::new(10, 10, 320, 240))?;
/// grid.add_category("Appearance");
/// grid.add_property("Title", PropertyValue::Text("Report".into()));
/// grid.add_property("Width", PropertyValue::Integer(800));
/// grid.add_property("Visible", PropertyValue::Bool(true));
/// grid.on_property_changed(|name, value| println!("{name} = {value:?}"));
/// # Ok(()) }
/// ```
#[derive(Clone)]
pub struct PropertyGrid {
    grid: DataGrid,
    state: Rc<RefCell<State>>,
}

impl Widget for PropertyGrid {
    fn core(&self) -> &Rc<WidgetCore> {
        self.grid.core()
    }
}

impl PropertyGrid {
    /// Creates a property grid in `parent` at the given logical rectangle.
    pub fn new(parent: &impl Widget, bounds: crate::utilities::Rect) -> GuiResult<PropertyGrid> {
        let mut builder = DataGrid::builder()
            .bounds(bounds.x, bounds.y, bounds.width, bounds.height)
            .editable(true)
            .sortable(false)
            .single_selection(true)
            .parent(parent);
        builder.kind = "PropertyGrid";
        let grid = builder.build()?;
        let half = (bounds.width - 8).max(80) / 2;
        grid.add_column(&library_text(LibraryText::PropertyColumn), half, ColumnAlign::Left);
        grid.add_column(&library_text(LibraryText::ValueColumn), half, ColumnAlign::Left);
        grid.set_column_read_only(0, true);
        let state = Rc::new(RefCell::new(State {
            entries: Vec::new(),
            listeners: Vec::new(),
            bool_labels: None,
            choice_mode: ChoiceMode::DropDown,
        }));
        let properties = PropertyGrid { grid, state };

        let hook = properties.clone();
        properties.grid.set_edit_hook(move |row, _| hook.on_begin_edit(row));
        let validate = properties.state.clone();
        properties.grid.set_validator(move |row, _, text| {
            let state = validate.borrow();
            state.entries.get(row).and_then(|e| e.value.as_ref()).map(|v| v.parse_like(text).is_some()).unwrap_or(false)
        });
        // Leitores de tela: uma lista de propriedades com padrões Value/Toggle por linha.
        let tree = PropertyTree { grid: properties.clone() };
        *properties.grid.core().accessible_tree.borrow_mut() = Some(TreeHost::new(tree));
        let announce = properties.clone();
        properties.grid.on_selection_changed(move |rows| {
            let host = announce.grid.core().accessible_tree.borrow().clone();
            if let (Some(host), Some(row)) = (host, rows.first()) {
                host.focus_changed(*row as u32 + 1);
            }
        });
        let edited = properties.clone();
        properties.grid.on_cell_edited(move |row, _, text| {
            let parsed = {
                let state = edited.state.borrow();
                state.entries.get(row).and_then(|e| e.value.as_ref()).and_then(|v| v.parse_like(text))
            };
            if let Some(value) = parsed {
                edited.store(row, value, true);
            }
        });
        Ok(properties)
    }

    fn on_begin_edit(&self, row: usize) -> EditAction {
        let next = {
            let state = self.state.borrow();
            match state.entries.get(row).map(|e| &e.value) {
                None | Some(None) => return EditAction::Deny,
                Some(Some(PropertyValue::Bool(value))) => PropertyValue::Bool(!value),
                Some(Some(PropertyValue::Choice { options, selected }))
                    if !options.is_empty() && state.choice_mode == ChoiceMode::DropDown =>
                {
                    return EditAction::Choice(options.clone(), *selected);
                }
                Some(Some(PropertyValue::Choice { options, selected })) if !options.is_empty() => {
                    PropertyValue::Choice { options: options.clone(), selected: (selected + 1) % options.len() }
                }
                Some(Some(PropertyValue::Choice { .. })) => return EditAction::Deny,
                Some(Some(_)) => return EditAction::Default,
            }
        };
        self.store(row, next, true);
        EditAction::Handled
    }

    fn store(&self, row: usize, value: PropertyValue, notify: bool) {
        let (name, listeners) = {
            let mut state = self.state.borrow_mut();
            let Some(entry) = state.entries.get_mut(row) else { return };
            entry.value = Some(value.clone());
            (entry.name.clone(), state.listeners.clone())
        };
        let text = self.state.borrow().display(&value);
        self.grid.set_cell(row, 1, &text);
        if notify {
            for listener in listeners {
                listener(&name, &value);
            }
        }
    }

    /// Adds a noneditable category/section heading row.
    pub fn add_category(&self, title: &str) {
        self.state.borrow_mut().entries.push(Entry { name: title.to_string(), value: None });
        self.grid.add_row(&[&format!("▾ {}", title.to_uppercase()), ""]);
    }

    /// Adds a property. Names must be unique.
    pub fn add_property(&self, name: &str, value: PropertyValue) {
        let display = self.state.borrow().display(&value);
        self.state.borrow_mut().entries.push(Entry { name: name.to_string(), value: Some(value) });
        self.grid.add_row(&[name, &display]);
    }

    fn row_of(&self, name: &str) -> Option<usize> {
        self.state.borrow().entries.iter().position(|e| e.value.is_some() && e.name == name)
    }

    /// Current property value.
    pub fn value(&self, name: &str) -> Option<PropertyValue> {
        let state = self.state.borrow();
        state.entries.iter().find(|e| e.value.is_some() && e.name == name).and_then(|e| e.value.clone())
    }

    /// Sets a property programmatically without firing `on_property_changed`. Returns `false` if absent.
    pub fn set_value(&self, name: &str, value: PropertyValue) -> bool {
        match self.row_of(name) {
            Some(row) => {
                self.store(row, value, false);
                true
            }
            None => false,
        }
    }

    /// All property names in display order.
    pub fn property_names(&self) -> Vec<String> {
        self.state.borrow().entries.iter().filter(|e| e.value.is_some()).map(|e| e.name.clone()).collect()
    }

    /// Simulates user editing: toggles booleans, cycles options or opens text editing. Returns `false` if not editable.
    pub fn activate(&self, name: &str) -> bool {
        self.row_of(name).map(|row| self.grid.begin_edit(row, 1)).unwrap_or(false)
    }

    /// Removes all properties and categories.
    pub fn clear(&self) {
        self.state.borrow_mut().entries.clear();
        self.grid.clear();
    }

    /// Boolean labels (default: Yes/No from [`LibraryText`]); `None` restores defaults. Updates existing rows.
    pub fn set_bool_labels(&self, labels: Option<(&str, &str)>) {
        self.state.borrow_mut().bool_labels = labels.map(|(yes, no)| (yes.to_string(), no.to_string()));
        self.refresh_values();
    }

    /// Chooses `Choice` editing mode (default: drop-down).
    pub fn set_choice_mode(&self, mode: ChoiceMode) {
        self.state.borrow_mut().choice_mode = mode;
    }

    /// Rewrites all value text after label/language changes.
    pub fn refresh_values(&self) {
        let texts: Vec<(usize, String)> = {
            let state = self.state.borrow();
            state
                .entries
                .iter()
                .enumerate()
                .filter_map(|(row, e)| e.value.as_ref().map(|v| (row, state.display(v))))
                .collect()
        };
        for (row, text) in texts {
            self.grid.set_cell(row, 1, &text);
        }
    }

    /// Whether an option drop-down is open.
    pub fn is_editing(&self) -> bool {
        self.grid.is_editing()
    }

    /// The user changed a property; receives name and new value.
    pub fn on_property_changed(&self, f: impl Fn(&str, &PropertyValue) + 'static) {
        self.state.borrow_mut().listeners.push(Rc::new(f));
    }
}

/// Grid UIA elements: categories as groups; properties with Value (text, numbers, choices) or Toggle (booleans)
/// patterns.
struct PropertyTree {
    grid: PropertyGrid,
}

impl PropertyTree {
    fn row_bounds(&self, hwnd: HWND, row: usize) -> RECT {
        let mut rect = RECT { left: LVIR_BOUNDS, ..RECT::default() };
        if native::send_mut(hwnd, LVM_GETITEMRECT, row, &mut rect) == 0 {
            return RECT::default();
        }
        let client = native::client_rect(hwnd);
        let header = native::send(hwnd, LVM_GETHEADER, 0, 0) as HWND;
        let header_bottom = if header.is_null() {
            0
        } else {
            let r = native::window_rect(header);
            r.bottom - r.top
        };
        rect.top = rect.top.max(header_bottom);
        rect.bottom = rect.bottom.min(client.bottom);
        rect.right = rect.right.min(client.right);
        if rect.bottom <= rect.top {
            return RECT::default();
        }
        let mut origin = POINT { x: 0, y: 0 };
        // SAFETY: conversão de coordenadas de uma janela viva.
        unsafe { ClientToScreen(hwnd, &mut origin) };
        RECT {
            left: rect.left + origin.x,
            top: rect.top + origin.y,
            right: rect.right + origin.x,
            bottom: rect.bottom + origin.y,
        }
    }
}

impl VirtualTree for PropertyTree {
    fn hwnd(&self) -> HWND {
        self.grid.core().hwnd()
    }

    fn root_role(&self) -> Option<i32> {
        Some(role::LIST)
    }

    fn root_name(&self) -> Option<String> {
        Some(library_text(LibraryText::PropertiesName))
    }

    fn root_node(&self) -> Option<Node> {
        // Lista de seleção única: o leitor de tela anuncia a propriedade selecionada.
        let mut node = Node::new(0, 0, role::LIST, &library_text(LibraryText::PropertiesName));
        node.selection = Some((false, false));
        Some(node)
    }

    fn nodes(&self) -> Vec<Node> {
        let hwnd = self.hwnd();
        // SAFETY: consulta simples.
        let has_focus = unsafe { GetFocus() } == hwnd;
        let selected = self.grid.grid.selected_rows().first().copied();
        let state = self.grid.state.borrow();
        let mut category = 0;
        let mut nodes = Vec::with_capacity(state.entries.len());
        for (row, entry) in state.entries.iter().enumerate() {
            let id = row as u32 + 1;
            let mut node = match &entry.value {
                None => {
                    category = id;
                    Node::new(id, 0, role::GROUP, &entry.name)
                }
                Some(value) => {
                    let mut node = Node::new(id, category, role::LIST_ITEM, &entry.name);
                    match value {
                        PropertyValue::Bool(on) => {
                            node.toggle = Some(*on);
                            node.value = Some((state.display(value), true));
                        }
                        _ => node.value = Some((state.display(value), false)),
                    }
                    node.focusable = true;
                    node.focused = has_focus && selected == Some(row);
                    node.selected = Some(selected == Some(row));
                    node
                }
            };
            node.bounds = self.row_bounds(hwnd, row);
            nodes.push(node);
        }
        nodes
    }

    fn act(&self, id: u32, action: Action) -> bool {
        let row = id as usize - 1;
        let current = {
            let state = self.grid.state.borrow();
            match state.entries.get(row).and_then(|e| e.value.clone()) {
                Some(value) => value,
                None => return false,
            }
        };
        let next = match (action, &current) {
            (Action::Select | Action::AddToSelection, _) => {
                self.grid.grid.set_row_selected(usize::MAX, false);
                self.grid.grid.set_row_selected(row, true);
                return true;
            }
            (Action::Focus, _) => {
                self.grid.grid.set_row_selected(row, true);
                // SAFETY: foco para a própria lista, viva nesta thread.
                unsafe { SetFocus(self.hwnd()) };
                return true;
            }
            (Action::Toggle, PropertyValue::Bool(on)) => PropertyValue::Bool(!on),
            (Action::SetValue(text), PropertyValue::Choice { options, .. }) => {
                let wanted = text.trim().to_lowercase();
                match options.iter().position(|o| o.to_lowercase() == wanted) {
                    Some(selected) => PropertyValue::Choice { options: options.clone(), selected },
                    None => return false,
                }
            }
            (Action::SetValue(text), value) => match value.parse_like(&text) {
                Some(parsed) => parsed,
                None => return false,
            },
            _ => return false,
        };
        self.grid.store(row, next, true);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::PropertyValue;

    #[test]
    fn typed_parsing_and_display() {
        assert_eq!(PropertyValue::Integer(0).parse_like(" 42 "), Some(PropertyValue::Integer(42)));
        assert_eq!(PropertyValue::Integer(0).parse_like("4x"), None);
        assert_eq!(PropertyValue::Float(0.0).parse_like("2,5"), Some(PropertyValue::Float(2.5)));
        assert_eq!(PropertyValue::Text(String::new()).parse_like("a"), Some(PropertyValue::Text("a".into())));
        assert_eq!(PropertyValue::Bool(true).parse_like("x"), None);
        let labels = ("Ligado".to_string(), "Desligado".to_string());
        assert_eq!(PropertyValue::Bool(true).display_with(Some(&labels)), "Ligado");
        assert_eq!(PropertyValue::Bool(false).display_with(Some(&labels)), "Desligado");
        let choice = PropertyValue::Choice { options: vec!["A".into(), "B".into()], selected: 1 };
        assert_eq!(choice.display_with(None), "B");
        assert_eq!(
            choice.parse_like("A"),
            Some(PropertyValue::Choice { options: vec!["A".into(), "B".into()], selected: 0 })
        );
        assert_eq!(choice.parse_like("C"), None);
    }
}
