//! UI Automation virtual element tree for library-drawn controls (`MenuStrip`, drawn menu lists, `CommandBar`,
//! `PropertyGrid`).
//!
//! Each control describes its items through [`VirtualTree`]; this module publishes them as UIA *fragments*
//! (`IRawElementProviderFragment` and, at the root, `IRawElementProviderFragmentRoot`) with Invoke, Toggle,
//! ExpandCollapse and Value patterns. The root is the window itself: UI Automation combines these properties with those
//! of the default `HWND` provider (`get_HostRawElementProvider`), and native child windows remain visible as children.
//!
//! COM objects accept calls only from the UI thread (the provider declares `ProviderOptions_UseComThreading`, and UIA
//! performs *marshaling*). Calls from other threads return `UIA_E_ELEMENTNOTAVAILABLE` without accessing Rust state.

use crate::core::application as app;
use crate::ffi::dynamic::optional;
use crate::ffi::kernel32::GetCurrentThreadId;
use crate::ffi::ole32::*;
use crate::ffi::uiautomation::*;
use crate::ffi::*;
use crate::utilities::strings::from_wide_ptr as from_wide_ptr_lossy;
use crate::utilities::to_wide;
use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::mem::offset_of;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::{Rc, Weak};

/// UIA control types used by virtual elements.
pub(crate) mod role {
    pub const BUTTON: i32 = 50000;
    pub const LIST_ITEM: i32 = 50007;
    pub const LIST: i32 = 50008;
    pub const MENU: i32 = 50009;
    pub const MENU_BAR: i32 = 50010;
    pub const MENU_ITEM: i32 = 50011;
    pub const TEXT: i32 = 50020;
    pub const TOOL_BAR: i32 = 50021;
    pub const GROUP: i32 = 50026;
    pub const SPLIT_BUTTON: i32 = 50031;
    pub const SEPARATOR: i32 = 50038;
}

/// A virtual element (menu item, toolbar button, property row, etc.).
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Node {
    /// Stable identifier within the tree (nonzero; 0 is the root).
    pub id: u32,
    /// Parent element (0 = root).
    pub parent: u32,
    pub role: i32,
    pub name: String,
    pub help: String,
    /// Access key ("Alt+A") or mnemonic.
    pub access_key: String,
    /// Shortcut ("Ctrl+S").
    pub accelerator: String,
    pub enabled: bool,
    pub focusable: bool,
    pub focused: bool,
    /// Screen rectangle in pixels (empty = off screen).
    pub bounds: RECT,
    pub invoke: bool,
    /// Toggle pattern state (`None` = no pattern).
    pub toggle: Option<bool>,
    /// ExpandCollapse pattern state (`None` = no pattern).
    pub expanded: Option<bool>,
    /// Value and read-only state of the Value pattern (`None` = no pattern).
    pub value: Option<(String, bool)>,
    /// Selection container (Selection pattern): (multiple selection, selection required).
    pub selection: Option<(bool, bool)>,
    /// Selectable item (SelectionItem pattern): whether it is selected.
    pub selected: Option<bool>,
    /// Scrolling (Scroll pattern).
    pub scroll: Option<ScrollState>,
    /// Grid (Grid pattern): rows and columns.
    pub grid: Option<(i32, i32)>,
    /// Grid cell (GridItem pattern): row and column.
    pub cell: Option<(i32, i32)>,
    /// Numeric range (RangeValue pattern).
    pub range: Option<RangeState>,
    /// Text (Text pattern, read-only) and its selection (UTF-16 units).
    pub text: Option<(String, Option<(usize, usize)>)>,
    /// Live region for announcements (`LiveSetting`): 1 = polite, 2 = assertive.
    pub live: Option<i32>,
}

/// Scroll pattern state (percentages from 0 to 100; -1 means no scrolling on that axis).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct ScrollState {
    pub horizontal: f64,
    pub vertical: f64,
    pub horizontal_view: f64,
    pub vertical_view: f64,
}

/// RangeValue pattern state.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct RangeState {
    pub value: f64,
    pub minimum: f64,
    pub maximum: f64,
    pub small_change: f64,
    pub large_change: f64,
    pub read_only: bool,
}

impl Node {
    pub(crate) fn new(id: u32, parent: u32, role: i32, name: &str) -> Node {
        Node { id, parent, role, name: name.to_string(), enabled: true, ..Node::default() }
    }

    fn offscreen(&self) -> bool {
        self.bounds.right <= self.bounds.left || self.bounds.bottom <= self.bounds.top
    }
}

/// Action requested by assistive technology.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Action {
    Invoke,
    Toggle,
    Expand,
    Collapse,
    Focus,
    SetValue(String),
    /// Selects only this item.
    Select,
    AddToSelection,
    RemoveFromSelection,
    /// Relative scrolling: `ScrollAmount` values (0 = large backward, 1 = small backward, 2 = none, 3 = large forward,
    /// 4 = small forward).
    Scroll(i32, i32),
    /// Scroll to (horizontal, vertical) percentages; -1 leaves the axis unchanged.
    ScrollTo(f64, f64),
    SetRange(f64),
    /// Selects the [start, end) text range (UTF-16 units).
    SelectText(usize, usize),
}

/// Description of a drawn control's items.
pub(crate) trait VirtualTree {
    /// Window hosting the elements.
    fn hwnd(&self) -> HWND;
    /// Root control type (`None` = the window's default provider type).
    fn root_role(&self) -> Option<i32> {
        None
    }
    /// Root name (`None` = the window's default provider name).
    fn root_name(&self) -> Option<String> {
        None
    }
    /// Patterns and state of the root itself (for example, a childless custom control with the Toggle pattern). `None`
    /// means the root only groups elements.
    fn root_node(&self) -> Option<Node> {
        None
    }
    /// All elements, parents before children.
    fn nodes(&self) -> Vec<Node>;
    /// Performs an action on element `id`; returns `false` if it cannot be performed.
    fn act(&self, id: u32, action: Action) -> bool;
}

/// Tree published for a window; stored in `WidgetCore` (or the menu list).
pub(crate) struct TreeHost {
    tree: Box<dyn VirtualTree>,
    root: RefCell<Option<ComPtr>>,
    thread: u32,
    /// Name set with `set_accessible_name` (overrides the tree's name).
    pub(crate) name: RefCell<Option<String>>,
    pub(crate) description: RefCell<Option<String>>,
    last_focus: Cell<u32>,
    /// Events requested by clients (`IRawElementProviderAdviseEvents`): id → count.
    advised: RefCell<std::collections::HashMap<i32, u32>>,
    /// A client has used `AdviseEventAdded` (from then on, it is the source of truth).
    advise_seen: Cell<bool>,
}

impl TreeHost {
    pub(crate) fn new(tree: impl VirtualTree + 'static) -> Rc<TreeHost> {
        let host = Rc::new(TreeHost {
            tree: Box::new(tree),
            root: RefCell::new(None),
            // SAFETY: consulta simples.
            thread: unsafe { GetCurrentThreadId() },
            name: RefCell::new(None),
            description: RefCell::new(None),
            last_focus: Cell::new(0),
            advised: RefCell::new(std::collections::HashMap::new()),
            advise_seen: Cell::new(false),
        });
        *host.root.borrow_mut() = Some(new_element(Rc::downgrade(&host), 0, host.thread));
        host
    }

    /// Root provider (`IRawElementProviderSimple`), for `WM_GETOBJECT`.
    pub(crate) fn root_provider(&self) -> Option<ComPtr> {
        self.root.borrow().clone()
    }

    fn node(&self, id: u32) -> Option<Node> {
        if id == 0 {
            return self.tree.root_node();
        }
        self.tree.nodes().into_iter().find(|n| n.id == id)
    }

    /// Whether `event` is worth firing: a client requested it (or there is no `AdviseEventAdded` information and UIA
    /// reports clients).
    pub(crate) fn wants(&self, event: i32) -> bool {
        // O foco é sempre avisado: clientes globais nem sempre passam pelo AdviseEventAdded.
        if !self.advise_seen.get() || event == UIA_AUTOMATION_FOCUS_CHANGED_EVENT_ID {
            return true;
        }
        self.advised.borrow().get(&event).is_some_and(|&count| count > 0)
    }

    /// Events currently requested by clients (for tests).
    pub(crate) fn advised_events(&self) -> Vec<i32> {
        let mut events: Vec<i32> = self.advised.borrow().iter().filter(|(_, &c)| c > 0).map(|(&e, _)| e).collect();
        events.sort_unstable();
        events
    }

    /// Notifies screen readers that focus moved to element `id` (0 = none).
    pub(crate) fn focus_changed(self: &Rc<TreeHost>, id: u32) {
        if self.last_focus.replace(id) == id || id == 0 {
            return;
        }
        self.raise(id, UIA_AUTOMATION_FOCUS_CHANGED_EVENT_ID);
    }

    /// Fires a UIA event on element `id` (0 = root) if clients are listening.
    pub(crate) fn raise(self: &Rc<TreeHost>, id: u32, event: i32) {
        let apis = optional();
        let (Some(listening), Some(raise)) = (apis.UiaClientsAreListening, apis.UiaRaiseAutomationEvent) else {
            return;
        };
        // SAFETY: consulta simples.
        if unsafe { listening() } == 0 || !self.wants(event) {
            return;
        }
        let element =
            if id == 0 { self.root_provider() } else { Some(new_element(Rc::downgrade(self), id, self.thread)) };
        if let Some(element) = element {
            // SAFETY: provedor vivo durante a chamada (o ComPtr mantém a referência).
            unsafe { raise(element.as_ptr(), event) };
        }
    }
}

impl TreeHost {
    /// Reports that the root's children changed (`StructureChangeType_ChildrenInvalidated`).
    pub(crate) fn structure_changed(self: &Rc<TreeHost>) {
        let apis = optional();
        let (Some(listening), Some(raise)) = (apis.UiaClientsAreListening, apis.UiaRaiseStructureChangedEvent) else {
            return;
        };
        // SAFETY: consulta simples.
        if unsafe { listening() } == 0 || !self.wants(UIA_STRUCTURE_CHANGED_EVENT_ID) {
            return;
        }
        if let Some(root) = self.root_provider() {
            // SAFETY: provedor vivo durante a chamada; sem identificador de execução (raiz do HWND).
            unsafe { raise(root.as_ptr(), 2, std::ptr::null_mut(), 0) };
        }
    }
}

impl Drop for TreeHost {
    fn drop(&mut self) {
        // A raiz pode sobreviver no UIA; sem a árvore ela passa a responder "indisponível".
        self.root.borrow_mut().take();
    }
}

// ---------------------------------------------------------------------------
// Objeto COM
// ---------------------------------------------------------------------------

type QiFn<T> = unsafe extern "system" fn(*mut T, *const GUID, *mut *mut c_void) -> HRESULT;
type RefFn<T> = unsafe extern "system" fn(*mut T) -> u32;

#[repr(C)]
struct SimpleVtbl {
    qi: QiFn<c_void>,
    add_ref: RefFn<c_void>,
    release: RefFn<c_void>,
    options: unsafe extern "system" fn(*mut c_void, *mut i32) -> HRESULT,
    pattern: unsafe extern "system" fn(*mut c_void, i32, *mut *mut c_void) -> HRESULT,
    property: unsafe extern "system" fn(*mut c_void, i32, *mut VARIANT) -> HRESULT,
    host: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> HRESULT,
}

#[repr(C)]
struct FragmentVtbl {
    qi: QiFn<c_void>,
    add_ref: RefFn<c_void>,
    release: RefFn<c_void>,
    navigate: unsafe extern "system" fn(*mut c_void, i32, *mut *mut c_void) -> HRESULT,
    runtime_id: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> HRESULT,
    bounds: unsafe extern "system" fn(*mut c_void, *mut UiaRect) -> HRESULT,
    embedded: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> HRESULT,
    set_focus: unsafe extern "system" fn(*mut c_void) -> HRESULT,
    fragment_root: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> HRESULT,
}

#[repr(C)]
struct RootVtbl {
    qi: QiFn<c_void>,
    add_ref: RefFn<c_void>,
    release: RefFn<c_void>,
    from_point: unsafe extern "system" fn(*mut c_void, f64, f64, *mut *mut c_void) -> HRESULT,
    focus: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> HRESULT,
}

#[repr(C)]
struct InvokeVtbl {
    qi: QiFn<c_void>,
    add_ref: RefFn<c_void>,
    release: RefFn<c_void>,
    invoke: unsafe extern "system" fn(*mut c_void) -> HRESULT,
}

#[repr(C)]
struct ToggleVtbl {
    qi: QiFn<c_void>,
    add_ref: RefFn<c_void>,
    release: RefFn<c_void>,
    toggle: unsafe extern "system" fn(*mut c_void) -> HRESULT,
    state: unsafe extern "system" fn(*mut c_void, *mut i32) -> HRESULT,
}

#[repr(C)]
struct ExpandVtbl {
    qi: QiFn<c_void>,
    add_ref: RefFn<c_void>,
    release: RefFn<c_void>,
    expand: unsafe extern "system" fn(*mut c_void) -> HRESULT,
    collapse: unsafe extern "system" fn(*mut c_void) -> HRESULT,
    state: unsafe extern "system" fn(*mut c_void, *mut i32) -> HRESULT,
}

#[repr(C)]
struct ValueVtbl {
    qi: QiFn<c_void>,
    add_ref: RefFn<c_void>,
    release: RefFn<c_void>,
    set_value: unsafe extern "system" fn(*mut c_void, *const u16) -> HRESULT,
    value: unsafe extern "system" fn(*mut c_void, *mut *mut u16) -> HRESULT,
    read_only: unsafe extern "system" fn(*mut c_void, *mut BOOL) -> HRESULT,
}

/// An element: a COM object with one vtable per interface (C++ style multiple inheritance).
#[repr(C)]
struct Element {
    simple: *const SimpleVtbl,
    fragment: *const FragmentVtbl,
    root: *const RootVtbl,
    invoke: *const InvokeVtbl,
    toggle: *const ToggleVtbl,
    expand: *const ExpandVtbl,
    value: *const ValueVtbl,
    selection: *const patterns::SelectionVtbl,
    selection_item: *const patterns::SelectionItemVtbl,
    scroll: *const patterns::ScrollVtbl,
    grid: *const patterns::GridVtbl,
    grid_item: *const patterns::GridItemVtbl,
    range: *const patterns::RangeVtbl,
    text: *const patterns::TextVtbl,
    advise: *const patterns::AdviseVtbl,
    references: Cell<u32>,
    thread: u32,
    host: Weak<TreeHost>,
    id: u32,
}

macro_rules! element_of {
    ($this:expr, $field:ident) => {
        &*(($this as *mut u8).sub(offset_of!(Element, $field)) as *const Element)
    };
}

/// Generates the three `IUnknown` functions for the interface stored in `$field`.
macro_rules! unknown {
    ($field:ident, $qi:ident, $add:ident, $release:ident) => {
        unsafe extern "system" fn $qi(this: *mut c_void, iid: *const GUID, out: *mut *mut c_void) -> HRESULT {
            query(element_of!(this, $field), iid, out)
        }
        unsafe extern "system" fn $add(this: *mut c_void) -> u32 {
            add_ref(element_of!(this, $field))
        }
        unsafe extern "system" fn $release(this: *mut c_void) -> u32 {
            release(element_of!(this, $field))
        }
    };
}

unknown!(simple, simple_qi, simple_add, simple_release);
unknown!(fragment, fragment_qi, fragment_add, fragment_release);
unknown!(root, root_qi, root_add, root_release);
unknown!(invoke, invoke_qi, invoke_add, invoke_release);
unknown!(toggle, toggle_qi, toggle_add, toggle_release);
unknown!(expand, expand_qi, expand_add, expand_release);
unknown!(value, value_qi, value_add, value_release);

mod patterns;
mod provider;
use provider::*;

static SIMPLE_VTBL: SimpleVtbl = SimpleVtbl {
    qi: simple_qi,
    add_ref: simple_add,
    release: simple_release,
    options: simple_options,
    pattern: simple_pattern,
    property: simple_property,
    host: simple_host,
};
static FRAGMENT_VTBL: FragmentVtbl = FragmentVtbl {
    qi: fragment_qi,
    add_ref: fragment_add,
    release: fragment_release,
    navigate: fragment_navigate,
    runtime_id: fragment_runtime_id,
    bounds: fragment_bounds,
    embedded: fragment_embedded,
    set_focus: fragment_set_focus,
    fragment_root,
};
static ROOT_VTBL: RootVtbl =
    RootVtbl { qi: root_qi, add_ref: root_add, release: root_release, from_point: root_from_point, focus: root_focus };
static INVOKE_VTBL: InvokeVtbl =
    InvokeVtbl { qi: invoke_qi, add_ref: invoke_add, release: invoke_release, invoke: invoke_invoke };
static TOGGLE_VTBL: ToggleVtbl = ToggleVtbl {
    qi: toggle_qi,
    add_ref: toggle_add,
    release: toggle_release,
    toggle: toggle_toggle,
    state: toggle_state,
};
static EXPAND_VTBL: ExpandVtbl = ExpandVtbl {
    qi: expand_qi,
    add_ref: expand_add,
    release: expand_release,
    expand: expand_expand,
    collapse: expand_collapse,
    state: expand_state,
};
static VALUE_VTBL: ValueVtbl = ValueVtbl {
    qi: value_qi,
    add_ref: value_add,
    release: value_release,
    set_value: value_set,
    value: value_get,
    read_only: value_read_only,
};

fn new_element(host: Weak<TreeHost>, id: u32, thread: u32) -> ComPtr {
    let raw = Box::into_raw(Box::new(Element {
        simple: &SIMPLE_VTBL,
        fragment: &FRAGMENT_VTBL,
        root: &ROOT_VTBL,
        invoke: &INVOKE_VTBL,
        toggle: &TOGGLE_VTBL,
        expand: &EXPAND_VTBL,
        value: &VALUE_VTBL,
        selection: &patterns::SELECTION_VTBL,
        selection_item: &patterns::SELECTION_ITEM_VTBL,
        scroll: &patterns::SCROLL_VTBL,
        grid: &patterns::GRID_VTBL,
        grid_item: &patterns::GRID_ITEM_VTBL,
        range: &patterns::RANGE_VTBL,
        text: &patterns::TEXT_VTBL,
        advise: &patterns::ADVISE_VTBL,
        references: Cell::new(1),
        thread,
        host,
        id,
    }));
    // SAFETY: objeto recém-criado com uma referência; o ponteiro do campo `simple` (o
    // primeiro) é a identidade IUnknown do objeto.
    unsafe { ComPtr::from_raw(raw as *mut c_void) }.expect("a Box pointer is never null")
}

impl Element {
    fn pointer_to<T>(&self, field: &*const T) -> *mut c_void {
        field as *const *const T as *mut c_void
    }

    fn same_thread(&self) -> bool {
        // SAFETY: consulta simples.
        unsafe { GetCurrentThreadId() == self.thread }
    }

    /// Live tree and a call on the correct thread.
    fn host(&self) -> Option<Rc<TreeHost>> {
        if !self.same_thread() {
            return None;
        }
        self.host.upgrade()
    }

    fn node(&self) -> Option<Node> {
        let host = self.host()?;
        host.node(self.id)
    }

    /// `IRawElementProviderFragment` pointer to another element in the same tree.
    fn sibling_fragment(&self, id: u32) -> *mut c_void {
        let element = new_element(self.host.clone(), id, self.thread);
        let raw = element.as_ptr() as *mut Element;
        std::mem::forget(element);
        // SAFETY: objeto recém-criado; a referência passa a quem chamou.
        unsafe { (*raw).pointer_to(&(*raw).fragment) }
    }
}

unsafe fn add_ref(element: &Element) -> u32 {
    let count = element.references.get() + 1;
    element.references.set(count);
    count
}

unsafe fn release(element: &Element) -> u32 {
    let count = element.references.get() - 1;
    element.references.set(count);
    if count == 0 {
        drop(Box::from_raw(element as *const Element as *mut Element));
    }
    count
}

unsafe fn query(element: &Element, iid: *const GUID, out: *mut *mut c_void) -> HRESULT {
    if out.is_null() || iid.is_null() {
        return E_POINTER;
    }
    *out = std::ptr::null_mut();
    let iid = *iid;
    let pointer = if iid == IID_IUnknown || iid == IID_IRawElementProviderSimple {
        element.pointer_to(&element.simple)
    } else if iid == IID_IRawElementProviderFragment {
        element.pointer_to(&element.fragment)
    } else if iid == IID_IRawElementProviderFragmentRoot && element.id == 0 {
        element.pointer_to(&element.root)
    } else if iid == IID_IRawElementProviderAdviseEvents && element.id == 0 {
        element.pointer_to(&element.advise)
    } else {
        // Padrões só para quem os tem (consultado na árvore viva).
        let Some(node) = element.node() else { return E_NOINTERFACE };
        if iid == IID_IInvokeProvider && node.invoke {
            element.pointer_to(&element.invoke)
        } else if iid == IID_IToggleProvider && node.toggle.is_some() {
            element.pointer_to(&element.toggle)
        } else if iid == IID_IExpandCollapseProvider && node.expanded.is_some() {
            element.pointer_to(&element.expand)
        } else if iid == IID_IValueProvider && node.value.is_some() {
            element.pointer_to(&element.value)
        } else if iid == IID_ISelectionProvider && node.selection.is_some() {
            element.pointer_to(&element.selection)
        } else if iid == IID_ISelectionItemProvider && node.selected.is_some() {
            element.pointer_to(&element.selection_item)
        } else if iid == IID_IScrollProvider && node.scroll.is_some() {
            element.pointer_to(&element.scroll)
        } else if iid == IID_IGridProvider && node.grid.is_some() {
            element.pointer_to(&element.grid)
        } else if iid == IID_IGridItemProvider && node.cell.is_some() {
            element.pointer_to(&element.grid_item)
        } else if iid == IID_IRangeValueProvider && node.range.is_some() {
            element.pointer_to(&element.range)
        } else if iid == IID_ITextProvider && node.text.is_some() {
            element.pointer_to(&element.text)
        } else {
            return E_NOINTERFACE;
        }
    };
    add_ref(element);
    *out = pointer;
    S_OK
}

/// Runs a user action without allowing a panic across the COM boundary.
fn act(element: &Element, action: Action) -> HRESULT {
    let Some(host) = element.host() else { return UIA_E_ELEMENTNOTAVAILABLE };
    match host.node(element.id) {
        Some(node) if !node.enabled => return UIA_E_ELEMENTNOTENABLED,
        None => return UIA_E_ELEMENTNOTAVAILABLE,
        _ => {}
    }
    match catch_unwind(AssertUnwindSafe(|| host.tree.act(element.id, action))) {
        Ok(true) => S_OK,
        Ok(false) => UIA_E_INVALIDOPERATION,
        Err(payload) => {
            app::store_panic(payload);
            UIA_E_INVALIDOPERATION
        }
    }
}

fn bstr(text: &str) -> *mut u16 {
    let wide = to_wide(text);
    // SAFETY: string terminada em NUL; a BSTR passa a quem chamou.
    unsafe { SysAllocString(wide.as_ptr()) }
}

fn variant_i4(value: i32) -> VARIANT {
    VARIANT { vt: VT_I4, reserved: [0; 3], data: [value as u32 as usize, 0] }
}

fn variant_bool(value: bool) -> VARIANT {
    VARIANT { vt: VT_BOOL, reserved: [0; 3], data: [if value { 0xFFFF } else { 0 }, 0] }
}

fn variant_text(text: &str) -> VARIANT {
    VARIANT { vt: VT_BSTR, reserved: [0; 3], data: [bstr(text) as usize, 0] }
}

#[doc(hidden)]
pub mod debug;

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: u32, parent: u32) -> Node {
        let mut node = Node::new(id, parent, role::BUTTON, "x");
        node.bounds = RECT { left: id as i32 * 10, top: 0, right: id as i32 * 10 + 10, bottom: 10 };
        node
    }

    #[test]
    fn navigation_follows_parents_and_siblings() {
        let nodes = vec![node(1, 0), node(2, 1), node(3, 1), node(4, 0)];
        assert_eq!(navigate(&nodes, 0, NAVIGATE_FIRST_CHILD), Some(1));
        assert_eq!(navigate(&nodes, 0, NAVIGATE_LAST_CHILD), Some(4));
        assert_eq!(navigate(&nodes, 1, NAVIGATE_NEXT), Some(4));
        assert_eq!(navigate(&nodes, 4, NAVIGATE_PREVIOUS), Some(1));
        assert_eq!(navigate(&nodes, 1, NAVIGATE_PREVIOUS), None);
        assert_eq!(navigate(&nodes, 3, NAVIGATE_PARENT), Some(1));
        assert_eq!(navigate(&nodes, 1, NAVIGATE_PARENT), Some(0));
        assert_eq!(navigate(&nodes, 0, NAVIGATE_PARENT), None);
        assert_eq!(navigate(&nodes, 2, NAVIGATE_FIRST_CHILD), None);
    }

    #[test]
    fn hit_test_prefers_the_innermost_element() {
        let mut parent = node(1, 0);
        parent.bounds = RECT { left: 0, top: 0, right: 100, bottom: 10 };
        let nodes = vec![parent, node(2, 1), node(3, 1)];
        assert_eq!(hit(&nodes, 25, 5), Some(2));
        assert_eq!(hit(&nodes, 75, 5), Some(1));
        assert_eq!(hit(&nodes, 75, 50), None);
    }
}
