//! Accessibility model for application-drawn controls: role, state and UI Automation patterns (Invoke, Toggle,
//! ExpandCollapse, Value, RangeValue, Selection, SelectionItem, Scroll, Grid, GridItem and Text) for the root and
//! internal elements.

use super::tree::{Action, Node, RangeState, ScrollState, TreeHost, VirtualTree};
use super::AccessibleRole;
use crate::core::widget::{Widget, WidgetCore};
use crate::core::{GuiError, GuiResult};
use crate::ffi::user32::ClientToScreen;
use crate::ffi::{HWND, POINT, RECT};
use crate::utilities::{to_pixels, Rect};
use std::rc::{Rc, Weak};

/// Numeric range (RangeValue pattern), for example for a rating control.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AccessibleRange {
    /// Current value.
    pub value: f64,
    /// Minimum.
    pub minimum: f64,
    /// Maximum.
    pub maximum: f64,
    /// Small step (arrow keys).
    pub small_change: f64,
    /// Large step (Page Up/Down).
    pub large_change: f64,
    /// Read-only.
    pub read_only: bool,
}

/// Scroll state (Scroll pattern): percentages from 0 to 100 and visible size (also a percentage); `None` means no
/// scrolling on that axis.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AccessibleScroll {
    /// Horizontal position.
    pub horizontal: Option<f64>,
    /// Vertical position.
    pub vertical: Option<f64>,
    /// Visible horizontal portion.
    pub horizontal_view: f64,
    /// Visible vertical portion.
    pub vertical_view: f64,
}

/// Scroll amount requested by assistive technology (`ScrollAmount`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ScrollAmount {
    /// Page backward.
    LargeDecrement,
    /// Line backward.
    SmallDecrement,
    /// None.
    NoAmount,
    /// Page forward.
    LargeIncrement,
    /// Line forward.
    SmallIncrement,
}

impl ScrollAmount {
    fn from_raw(value: i32) -> ScrollAmount {
        match value {
            0 => ScrollAmount::LargeDecrement,
            1 => ScrollAmount::SmallDecrement,
            3 => ScrollAmount::LargeIncrement,
            4 => ScrollAmount::SmallIncrement,
            _ => ScrollAmount::NoAmount,
        }
    }
}

/// Urgency of a live region announcement.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum LiveSetting {
    /// No automatic announcement.
    #[default]
    Off,
    /// Announce when the reader is idle.
    Polite,
    /// Interrupt to announce.
    Assertive,
}

/// An element in the accessibility model: the root (the control itself) or an internal item.
///
/// Optional fields enable the corresponding patterns: `toggle` → Toggle, `expanded` → ExpandCollapse, `value` → Value,
/// `range` → RangeValue, `selection` → Selection, `selected` → SelectionItem, `grid` → Grid, `cell` → GridItem,
/// `scroll` → Scroll, `text` → Text.
#[derive(Clone, Debug, PartialEq)]
pub struct AccessibleItem {
    /// Stable identifier (nonzero; the root uses 0 and ignores this field).
    pub id: u32,
    /// Parent element (0 = root).
    pub parent: u32,
    /// Role.
    pub role: AccessibleRole,
    /// Name read by screen readers.
    pub name: String,
    /// Additional description.
    pub help: String,
    /// Logical rectangle relative to the control's client area (ignored for the root).
    pub bounds: Rect,
    /// Enabled.
    pub enabled: bool,
    /// Can receive keyboard focus.
    pub focusable: bool,
    /// Has keyboard focus.
    pub focused: bool,
    /// Invoke pattern.
    pub invoke: bool,
    /// Toggle pattern (checked or unchecked).
    pub toggle: Option<bool>,
    /// ExpandCollapse pattern (expanded or collapsed).
    pub expanded: Option<bool>,
    /// Value pattern: text and read-only state.
    pub value: Option<(String, bool)>,
    /// RangeValue pattern.
    pub range: Option<AccessibleRange>,
    /// Selection pattern: (multiple selection, selection required).
    pub selection: Option<(bool, bool)>,
    /// SelectionItem pattern: selected or not.
    pub selected: Option<bool>,
    /// Grid pattern: rows and columns.
    pub grid: Option<(i32, i32)>,
    /// GridItem pattern: row and column.
    pub cell: Option<(i32, i32)>,
    /// Scroll pattern.
    pub scroll: Option<AccessibleScroll>,
    /// Text pattern (read-only).
    pub text: Option<String>,
    /// Text selection (UTF-16 units).
    pub text_selection: Option<(usize, usize)>,
    /// Live region.
    pub live: LiveSetting,
}

impl AccessibleItem {
    /// Enabled element without patterns.
    pub fn new(id: u32, role: AccessibleRole, name: &str) -> AccessibleItem {
        AccessibleItem {
            id,
            parent: 0,
            role,
            name: name.to_string(),
            help: String::new(),
            bounds: Rect::default(),
            enabled: true,
            focusable: false,
            focused: false,
            invoke: false,
            toggle: None,
            expanded: None,
            value: None,
            range: None,
            selection: None,
            selected: None,
            grid: None,
            cell: None,
            scroll: None,
            text: None,
            text_selection: None,
            live: LiveSetting::Off,
        }
    }

    /// Sets the parent.
    pub fn parent(mut self, parent: u32) -> Self {
        self.parent = parent;
        self
    }

    /// Sets the logical rectangle relative to the control.
    pub fn bounds(mut self, bounds: Rect) -> Self {
        self.bounds = bounds;
        self
    }

    /// Enables or disables.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Can receive focus; `focused` indicates whether it has focus.
    pub fn focus(mut self, focused: bool) -> Self {
        self.focusable = true;
        self.focused = focused;
        self
    }

    /// Enables the Invoke pattern.
    pub fn invokable(mut self) -> Self {
        self.invoke = true;
        self
    }

    /// Enables the Toggle pattern.
    pub fn toggle(mut self, on: bool) -> Self {
        self.toggle = Some(on);
        self
    }

    /// Enables the ExpandCollapse pattern.
    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = Some(expanded);
        self
    }

    /// Enables the Value pattern.
    pub fn value(mut self, value: &str, read_only: bool) -> Self {
        self.value = Some((value.to_string(), read_only));
        self
    }

    /// Enables the RangeValue pattern.
    pub fn range(mut self, range: AccessibleRange) -> Self {
        self.range = Some(range);
        self
    }

    /// Enables the Selection pattern (container).
    pub fn selection(mut self, multiple: bool, required: bool) -> Self {
        self.selection = Some((multiple, required));
        self
    }

    /// Enables the SelectionItem pattern.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = Some(selected);
        self
    }

    /// Enables the Grid pattern.
    pub fn grid(mut self, rows: i32, columns: i32) -> Self {
        self.grid = Some((rows, columns));
        self
    }

    /// Enables the GridItem pattern.
    pub fn cell(mut self, row: i32, column: i32) -> Self {
        self.cell = Some((row, column));
        self
    }

    /// Enables the Scroll pattern.
    pub fn scroll(mut self, scroll: AccessibleScroll) -> Self {
        self.scroll = Some(scroll);
        self
    }

    /// Enables the Text pattern.
    pub fn text(mut self, text: &str) -> Self {
        self.text = Some(text.to_string());
        self
    }

    /// Live region.
    pub fn live(mut self, live: LiveSetting) -> Self {
        self.live = live;
        self
    }

    fn to_node(&self, id: u32, origin: POINT, dpi: u32) -> Node {
        let mut node = Node::new(id, self.parent, self.role.control_type(), &self.name);
        node.help = self.help.clone();
        node.enabled = self.enabled;
        node.focusable = self.focusable;
        node.focused = self.focused;
        node.invoke = self.invoke;
        node.toggle = self.toggle;
        node.expanded = self.expanded;
        node.value = self.value.clone();
        node.range = self.range.map(|r| RangeState {
            value: r.value,
            minimum: r.minimum,
            maximum: r.maximum,
            small_change: r.small_change,
            large_change: r.large_change,
            read_only: r.read_only,
        });
        node.selection = self.selection;
        node.selected = self.selected;
        node.grid = self.grid;
        node.cell = self.cell;
        node.scroll = self.scroll.map(|s| ScrollState {
            horizontal: s.horizontal.unwrap_or(-1.0),
            vertical: s.vertical.unwrap_or(-1.0),
            horizontal_view: s.horizontal_view,
            vertical_view: s.vertical_view,
        });
        node.text = self.text.clone().map(|t| (t, self.text_selection));
        node.live = match self.live {
            LiveSetting::Off => None,
            LiveSetting::Polite => Some(1),
            LiveSetting::Assertive => Some(2),
        };
        let b = self.bounds;
        if b.width > 0 && b.height > 0 {
            let left = origin.x + to_pixels(b.x, dpi);
            let top = origin.y + to_pixels(b.y, dpi);
            node.bounds =
                RECT { left, top, right: left + to_pixels(b.width, dpi), bottom: top + to_pixels(b.height, dpi) };
        }
        node
    }
}

/// Action requested by assistive technology on an element.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum AccessibleAction {
    /// Invoke pattern.
    Invoke,
    /// Toggle pattern.
    Toggle,
    /// ExpandCollapse pattern: expand.
    Expand,
    /// ExpandCollapse pattern: collapse.
    Collapse,
    /// Move keyboard focus to the element.
    Focus,
    /// Value pattern.
    SetValue(String),
    /// RangeValue pattern.
    SetRange(f64),
    /// SelectionItem pattern: select only this item.
    Select,
    /// SelectionItem pattern: add to selection.
    AddToSelection,
    /// SelectionItem pattern: remove from selection.
    RemoveFromSelection,
    /// Scroll pattern: relative scrolling (horizontal, vertical).
    Scroll(ScrollAmount, ScrollAmount),
    /// Scroll pattern: percentage position (`None` leaves the axis unchanged).
    ScrollTo(Option<f64>, Option<f64>),
    /// Text pattern: select the [start, end) range in UTF-16 units.
    SelectText(usize, usize),
}

impl AccessibleAction {
    fn from_internal(action: Action) -> AccessibleAction {
        let percent = |v: f64| (v >= 0.0).then_some(v);
        match action {
            Action::Invoke => AccessibleAction::Invoke,
            Action::Toggle => AccessibleAction::Toggle,
            Action::Expand => AccessibleAction::Expand,
            Action::Collapse => AccessibleAction::Collapse,
            Action::Focus => AccessibleAction::Focus,
            Action::SetValue(text) => AccessibleAction::SetValue(text),
            Action::Select => AccessibleAction::Select,
            Action::AddToSelection => AccessibleAction::AddToSelection,
            Action::RemoveFromSelection => AccessibleAction::RemoveFromSelection,
            Action::Scroll(h, v) => AccessibleAction::Scroll(ScrollAmount::from_raw(h), ScrollAmount::from_raw(v)),
            Action::ScrollTo(h, v) => AccessibleAction::ScrollTo(percent(h), percent(v)),
            Action::SetRange(value) => AccessibleAction::SetRange(value),
            Action::SelectText(start, end) => AccessibleAction::SelectText(start, end),
        }
    }
}

/// Accessible description of an application-drawn control.
///
/// `root` describes the control itself; `children` describes internal items (parents before children). Methods are
/// queried for each screen reader request, so simply return the current state.
///
/// ```no_run
/// # use arqrustui::prelude::*;
/// use arqrustui::accessibility::{AccessibleAction, AccessibleItem, AccessibleModel, AccessibleRole};
/// use std::cell::Cell;
/// use std::rc::Rc;
///
/// struct Switch(Rc<Cell<bool>>);
///
/// impl AccessibleModel for Switch {
///     fn root(&self) -> AccessibleItem {
///         AccessibleItem::new(0, AccessibleRole::Button, "Airplane mode").toggle(self.0.get())
///     }
///     fn perform(&self, _id: u32, action: AccessibleAction) -> bool {
///         if action == AccessibleAction::Toggle {
///             self.0.set(!self.0.get());
///         }
///         true
///     }
/// }
/// # fn demo(panel: &Panel) -> GuiResult<()> {
/// arqrustui::accessibility::set_accessible_model(panel, Switch(Rc::new(Cell::new(false))))?;
/// # Ok(()) }
/// ```
pub trait AccessibleModel {
    /// The control itself (`id` is ignored: the root is always 0).
    fn root(&self) -> AccessibleItem;
    /// Internal items.
    fn children(&self) -> Vec<AccessibleItem> {
        Vec::new()
    }
    /// Performs an action; return `false` if it cannot be performed.
    fn perform(&self, id: u32, action: AccessibleAction) -> bool;
}

/// Attaches an [`AccessibleModel`] to the internal UIA tree.
struct ModelTree {
    core: Weak<WidgetCore>,
    hwnd: HWND,
    model: Box<dyn AccessibleModel>,
}

impl ModelTree {
    fn origin(&self) -> (POINT, u32) {
        let mut origin = POINT { x: 0, y: 0 };
        // SAFETY: conversão de coordenadas de uma janela possivelmente viva (sem efeito se não for).
        unsafe { ClientToScreen(self.hwnd, &mut origin) };
        let dpi = self.core.upgrade().map(|c| c.dpi()).unwrap_or(96);
        (origin, dpi)
    }
}

impl VirtualTree for ModelTree {
    fn hwnd(&self) -> HWND {
        self.hwnd
    }

    fn root_role(&self) -> Option<i32> {
        Some(self.model.root().role.control_type())
    }

    fn root_name(&self) -> Option<String> {
        Some(self.model.root().name)
    }

    fn root_node(&self) -> Option<Node> {
        let (origin, dpi) = self.origin();
        let mut node = self.model.root().to_node(0, origin, dpi);
        node.parent = 0;
        Some(node)
    }

    fn nodes(&self) -> Vec<Node> {
        let (origin, dpi) = self.origin();
        self.model.children().iter().filter(|i| i.id != 0).map(|item| item.to_node(item.id, origin, dpi)).collect()
    }

    fn act(&self, id: u32, action: Action) -> bool {
        self.model.perform(id, AccessibleAction::from_internal(action))
    }
}

/// Publishes the accessible model of an application-drawn control (typically a [`Panel`](crate::controls::Panel) with
/// `on_paint`), with UIA patterns on the root and internal items. Replaces any previous
/// [`set_custom_role`](super::set_custom_role).
pub fn set_accessible_model(widget: &impl Widget, model: impl AccessibleModel + 'static) -> GuiResult<()> {
    let core = widget.core();
    let hwnd = core.require_alive()?;
    if crate::ffi::dynamic::optional().UiaReturnRawElementProvider.is_none() {
        return Err(GuiError::Unsupported("UI Automation (uiautomationcore.dll)"));
    }
    let tree = ModelTree { core: Rc::downgrade(core), hwnd, model: Box::new(model) };
    let previous = core.accessible.borrow_mut().take();
    drop(previous);
    *core.accessible_tree.borrow_mut() = Some(TreeHost::new(tree));
    Ok(())
}

/// Event that a control with an [`AccessibleModel`] can report to screen readers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum AccessibleEvent {
    /// Keyboard focus moved to the element.
    Focus,
    /// The element was invoked.
    Invoked,
    /// The element was selected.
    Selected,
    /// The text changed.
    TextChanged,
    /// The text selection changed.
    TextSelectionChanged,
    /// Elements were added or removed.
    StructureChanged,
    /// Live region content changed (the reader announces it).
    LiveRegionChanged,
}

/// Notifies screen readers of a change to element `id` (0 = the control itself) of a control published with
/// [`set_accessible_model`] or drawn by the library.
pub fn raise_accessible_event(widget: &impl Widget, id: u32, event: AccessibleEvent) {
    use crate::ffi::uiautomation::*;
    let host = widget.core().accessible_tree.borrow().clone();
    let Some(host) = host else { return };
    let event = match event {
        AccessibleEvent::Focus => {
            host.focus_changed(id);
            return;
        }
        AccessibleEvent::Invoked => UIA_INVOKE_INVOKED_EVENT_ID,
        AccessibleEvent::Selected => UIA_SELECTION_ITEM_ELEMENT_SELECTED_EVENT_ID,
        AccessibleEvent::TextChanged => UIA_TEXT_TEXT_CHANGED_EVENT_ID,
        AccessibleEvent::TextSelectionChanged => UIA_TEXT_TEXT_SELECTION_CHANGED_EVENT_ID,
        AccessibleEvent::StructureChanged => {
            host.structure_changed();
            return;
        }
        AccessibleEvent::LiveRegionChanged => UIA_LIVE_REGION_CHANGED_EVENT_ID,
    };
    host.raise(id, event);
}
