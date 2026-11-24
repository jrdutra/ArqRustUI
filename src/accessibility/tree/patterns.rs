//! Additional UIA patterns for virtual elements: Selection, SelectionItem, Scroll, Grid, GridItem, RangeValue and Text
//! (with `ITextRangeProvider`), plus `IRawElementProviderAdviseEvents` on the root.

use super::*;

// ---------------------------------------------------------------------------
// Vtables
// ---------------------------------------------------------------------------

type Hr0 = unsafe extern "system" fn(*mut c_void) -> HRESULT;
type HrBool = unsafe extern "system" fn(*mut c_void, *mut BOOL) -> HRESULT;
type HrF64 = unsafe extern "system" fn(*mut c_void, *mut f64) -> HRESULT;
type HrI32 = unsafe extern "system" fn(*mut c_void, *mut i32) -> HRESULT;
type HrPtr = unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> HRESULT;

#[repr(C)]
pub(crate) struct SelectionVtbl {
    qi: QiFn<c_void>,
    add_ref: RefFn<c_void>,
    release: RefFn<c_void>,
    pub(crate) get_selection: HrPtr,
    pub(crate) can_select_multiple: HrBool,
    pub(crate) is_selection_required: HrBool,
}

#[repr(C)]
pub(crate) struct SelectionItemVtbl {
    qi: QiFn<c_void>,
    add_ref: RefFn<c_void>,
    release: RefFn<c_void>,
    pub(crate) select: Hr0,
    pub(crate) add_to_selection: Hr0,
    pub(crate) remove_from_selection: Hr0,
    pub(crate) is_selected: HrBool,
    pub(crate) selection_container: HrPtr,
}

#[repr(C)]
pub(crate) struct ScrollVtbl {
    qi: QiFn<c_void>,
    add_ref: RefFn<c_void>,
    release: RefFn<c_void>,
    pub(crate) scroll: unsafe extern "system" fn(*mut c_void, i32, i32) -> HRESULT,
    pub(crate) set_scroll_percent: unsafe extern "system" fn(*mut c_void, f64, f64) -> HRESULT,
    pub(crate) horizontal_percent: HrF64,
    pub(crate) vertical_percent: HrF64,
    pub(crate) horizontal_view: HrF64,
    pub(crate) vertical_view: HrF64,
    pub(crate) horizontally_scrollable: HrBool,
    pub(crate) vertically_scrollable: HrBool,
}

#[repr(C)]
pub(crate) struct GridVtbl {
    qi: QiFn<c_void>,
    add_ref: RefFn<c_void>,
    release: RefFn<c_void>,
    pub(crate) get_item: unsafe extern "system" fn(*mut c_void, i32, i32, *mut *mut c_void) -> HRESULT,
    pub(crate) row_count: HrI32,
    pub(crate) column_count: HrI32,
}

#[repr(C)]
pub(crate) struct GridItemVtbl {
    qi: QiFn<c_void>,
    add_ref: RefFn<c_void>,
    release: RefFn<c_void>,
    row: HrI32,
    column: HrI32,
    row_span: HrI32,
    column_span: HrI32,
    containing_grid: HrPtr,
}

#[repr(C)]
pub(crate) struct RangeVtbl {
    qi: QiFn<c_void>,
    add_ref: RefFn<c_void>,
    release: RefFn<c_void>,
    pub(crate) set_value: unsafe extern "system" fn(*mut c_void, f64) -> HRESULT,
    pub(crate) value: HrF64,
    pub(crate) read_only: HrBool,
    pub(crate) maximum: HrF64,
    pub(crate) minimum: HrF64,
    pub(crate) large_change: HrF64,
    pub(crate) small_change: HrF64,
}

#[repr(C)]
pub(crate) struct TextVtbl {
    qi: QiFn<c_void>,
    add_ref: RefFn<c_void>,
    release: RefFn<c_void>,
    get_selection: HrPtr,
    visible_ranges: HrPtr,
    range_from_child: unsafe extern "system" fn(*mut c_void, *mut c_void, *mut *mut c_void) -> HRESULT,
    range_from_point: unsafe extern "system" fn(*mut c_void, UiaPoint, *mut *mut c_void) -> HRESULT,
    document_range: HrPtr,
    supported_selection: HrI32,
}

#[repr(C)]
pub(crate) struct AdviseVtbl {
    qi: QiFn<c_void>,
    add_ref: RefFn<c_void>,
    release: RefFn<c_void>,
    pub(crate) added: unsafe extern "system" fn(*mut c_void, i32, *mut c_void) -> HRESULT,
    pub(crate) removed: unsafe extern "system" fn(*mut c_void, i32, *mut c_void) -> HRESULT,
}

unknown!(selection, selection_qi, selection_add, selection_release);
unknown!(selection_item, item_qi, item_add, item_release);
unknown!(scroll, scroll_qi, scroll_add, scroll_release);
unknown!(grid, grid_qi, grid_add, grid_release);
unknown!(grid_item, cell_qi, cell_add, cell_release);
unknown!(range, range_qi, range_add, range_release);
unknown!(text, text_qi, text_add, text_release);
unknown!(advise, advise_qi, advise_add, advise_release);

pub(crate) static SELECTION_VTBL: SelectionVtbl = SelectionVtbl {
    qi: selection_qi,
    add_ref: selection_add,
    release: selection_release,
    get_selection: selection_get,
    can_select_multiple: selection_multiple,
    is_selection_required: selection_required,
};
pub(crate) static SELECTION_ITEM_VTBL: SelectionItemVtbl = SelectionItemVtbl {
    qi: item_qi,
    add_ref: item_add,
    release: item_release,
    select: item_select,
    add_to_selection: item_add_to,
    remove_from_selection: item_remove_from,
    is_selected: item_is_selected,
    selection_container: item_container,
};
pub(crate) static SCROLL_VTBL: ScrollVtbl = ScrollVtbl {
    qi: scroll_qi,
    add_ref: scroll_add,
    release: scroll_release,
    scroll: scroll_scroll,
    set_scroll_percent: scroll_set_percent,
    horizontal_percent: scroll_h_percent,
    vertical_percent: scroll_v_percent,
    horizontal_view: scroll_h_view,
    vertical_view: scroll_v_view,
    horizontally_scrollable: scroll_h_scrollable,
    vertically_scrollable: scroll_v_scrollable,
};
pub(crate) static GRID_VTBL: GridVtbl = GridVtbl {
    qi: grid_qi,
    add_ref: grid_add,
    release: grid_release,
    get_item: grid_get_item,
    row_count: grid_rows,
    column_count: grid_columns,
};
pub(crate) static GRID_ITEM_VTBL: GridItemVtbl = GridItemVtbl {
    qi: cell_qi,
    add_ref: cell_add,
    release: cell_release,
    row: cell_row,
    column: cell_column,
    row_span: cell_span,
    column_span: cell_span,
    containing_grid: cell_grid,
};
pub(crate) static RANGE_VTBL: RangeVtbl = RangeVtbl {
    qi: range_qi,
    add_ref: range_add,
    release: range_release,
    set_value: range_set,
    value: range_value,
    read_only: range_read_only,
    maximum: range_maximum,
    minimum: range_minimum,
    large_change: range_large,
    small_change: range_small,
};
pub(crate) static TEXT_VTBL: TextVtbl = TextVtbl {
    qi: text_qi,
    add_ref: text_add,
    release: text_release,
    get_selection: text_selection,
    visible_ranges: text_visible,
    range_from_child: text_from_child,
    range_from_point: text_from_point,
    document_range: text_document,
    supported_selection: text_supported,
};
pub(crate) static ADVISE_VTBL: AdviseVtbl = AdviseVtbl {
    qi: advise_qi,
    add_ref: advise_add,
    release: advise_release,
    added: advise_added,
    removed: advise_removed,
};

// ---------------------------------------------------------------------------
// Propriedades
// ---------------------------------------------------------------------------

fn variant_f64(value: f64) -> VARIANT {
    VARIANT::from_f64(VT_R8, value)
}

/// Additional pattern properties (and basic root properties with `root_node`).
pub(super) fn pattern_property(node: &Node, property: i32) -> Option<VARIANT> {
    Some(match property {
        UIA_CONTROL_TYPE_PROPERTY_ID if node.role != 0 => variant_i4(node.role),
        UIA_NAME_PROPERTY_ID if !node.name.is_empty() => variant_text(&node.name),
        UIA_IS_ENABLED_PROPERTY_ID => variant_bool(node.enabled),
        UIA_SELECTION_CAN_SELECT_MULTIPLE_PROPERTY_ID => variant_bool(node.selection?.0),
        UIA_SELECTION_IS_SELECTION_REQUIRED_PROPERTY_ID => variant_bool(node.selection?.1),
        UIA_SELECTION_ITEM_IS_SELECTED_PROPERTY_ID => variant_bool(node.selected?),
        UIA_SCROLL_HORIZONTAL_SCROLL_PERCENT_PROPERTY_ID => variant_f64(node.scroll?.horizontal),
        UIA_SCROLL_VERTICAL_SCROLL_PERCENT_PROPERTY_ID => variant_f64(node.scroll?.vertical),
        UIA_SCROLL_HORIZONTAL_VIEW_SIZE_PROPERTY_ID => variant_f64(node.scroll?.horizontal_view),
        UIA_SCROLL_VERTICAL_VIEW_SIZE_PROPERTY_ID => variant_f64(node.scroll?.vertical_view),
        UIA_SCROLL_HORIZONTALLY_SCROLLABLE_PROPERTY_ID => variant_bool(node.scroll?.horizontal >= 0.0),
        UIA_SCROLL_VERTICALLY_SCROLLABLE_PROPERTY_ID => variant_bool(node.scroll?.vertical >= 0.0),
        UIA_GRID_ROW_COUNT_PROPERTY_ID => variant_i4(node.grid?.0),
        UIA_GRID_COLUMN_COUNT_PROPERTY_ID => variant_i4(node.grid?.1),
        UIA_GRID_ITEM_ROW_PROPERTY_ID => variant_i4(node.cell?.0),
        UIA_GRID_ITEM_COLUMN_PROPERTY_ID => variant_i4(node.cell?.1),
        UIA_RANGE_VALUE_VALUE_PROPERTY_ID => variant_f64(node.range?.value),
        UIA_RANGE_VALUE_MINIMUM_PROPERTY_ID => variant_f64(node.range?.minimum),
        UIA_RANGE_VALUE_MAXIMUM_PROPERTY_ID => variant_f64(node.range?.maximum),
        UIA_RANGE_VALUE_SMALL_CHANGE_PROPERTY_ID => variant_f64(node.range?.small_change),
        UIA_RANGE_VALUE_LARGE_CHANGE_PROPERTY_ID => variant_f64(node.range?.large_change),
        UIA_RANGE_VALUE_IS_READ_ONLY_PROPERTY_ID => variant_bool(node.range?.read_only),
        UIA_LIVE_SETTING_PROPERTY_ID => variant_i4(node.live?),
        UIA_TOGGLE_STATE_PROPERTY_ID => variant_i4(i32::from(node.toggle?)),
        UIA_EXPAND_COLLAPSE_STATE_PROPERTY_ID => variant_i4(i32::from(node.expanded?)),
        UIA_VALUE_VALUE_PROPERTY_ID => variant_text(&node.value.as_ref()?.0),
        UIA_VALUE_IS_READ_ONLY_PROPERTY_ID => variant_bool(node.value.as_ref()?.1),
        UIA_IS_KEYBOARD_FOCUSABLE_PROPERTY_ID if node.focusable => variant_bool(true),
        UIA_HAS_KEYBOARD_FOCUS_PROPERTY_ID if node.focusable => variant_bool(node.focused),
        _ => return None,
    })
}

// ---------------------------------------------------------------------------
// Auxiliares
// ---------------------------------------------------------------------------

/// `IRawElementProviderSimple` of another element (new reference for the caller).
fn simple_of(element: &Element, id: u32) -> *mut c_void {
    if id == 0 {
        if let Some(root) = element.host().and_then(|h| h.root_provider()) {
            let raw = root.as_ptr() as *mut Element;
            // SAFETY: a raiz é um Element vivo; a referência extra passa a quem chamou.
            unsafe {
                add_ref(&*raw);
                return (*raw).pointer_to(&(*raw).simple);
            }
        }
        return std::ptr::null_mut();
    }
    let created = new_element(element.host.clone(), id, element.thread);
    let raw = created.as_ptr() as *mut Element;
    std::mem::forget(created);
    // SAFETY: objeto recém-criado; a referência passa a quem chamou.
    unsafe { (*raw).pointer_to(&(*raw).simple) }
}

/// SAFEARRAY of `IUnknown` with the given elements/ranges (each already has its own reference, released here once the
/// SAFEARRAY acquires its reference).
unsafe fn unknown_array(items: Vec<*mut c_void>) -> *mut c_void {
    let array = SafeArrayCreateVector(VT_UNKNOWN, 0, items.len() as u32);
    for (index, item) in items.into_iter().enumerate() {
        let index = index as i32;
        if !array.is_null() {
            SafeArrayPutElement(array, &index, item);
        }
        // SafeArrayPutElement faz AddRef; solta a referência criada para o item.
        let vtable = *(item as *const *const [usize; 3]);
        let release: RefFn<c_void> = std::mem::transmute((*vtable)[2]);
        release(item);
    }
    array
}

fn node_of(element: &Element) -> Result<Node, HRESULT> {
    element.node().ok_or(UIA_E_ELEMENTNOTAVAILABLE)
}

unsafe fn write<T>(out: *mut T, value: Result<T, HRESULT>) -> HRESULT {
    if out.is_null() {
        return E_POINTER;
    }
    match value {
        Ok(value) => {
            *out = value;
            S_OK
        }
        Err(hr) => hr,
    }
}

// ---------------------------------------------------------------------------
// Selection / SelectionItem
// ---------------------------------------------------------------------------

unsafe extern "system" fn selection_get(this: *mut c_void, out: *mut *mut c_void) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if out.is_null() {
            return E_POINTER;
        }
        let element = element_of!(this, selection);
        let Some(host) = element.host() else { return UIA_E_ELEMENTNOTAVAILABLE };
        let selected: Vec<*mut c_void> = host
            .tree
            .nodes()
            .iter()
            .filter(|n| n.parent == element.id && n.selected == Some(true))
            .map(|n| simple_of(element, n.id))
            .collect();
        *out = unknown_array(selected);
        S_OK
    })
}

unsafe extern "system" fn selection_multiple(this: *mut c_void, out: *mut BOOL) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        let element = element_of!(this, selection);
        write(out, node_of(element).map(|n| BOOL::from(n.selection.is_some_and(|s| s.0))))
    })
}

unsafe extern "system" fn selection_required(this: *mut c_void, out: *mut BOOL) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        let element = element_of!(this, selection);
        write(out, node_of(element).map(|n| BOOL::from(n.selection.is_some_and(|s| s.1))))
    })
}

unsafe extern "system" fn item_select(this: *mut c_void) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        let element = element_of!(this, selection_item);
        let result = act(element, Action::Select);
        if result == S_OK {
            if let Some(host) = element.host() {
                host.raise(element.id, UIA_SELECTION_ITEM_ELEMENT_SELECTED_EVENT_ID);
            }
        }
        result
    })
}

unsafe extern "system" fn item_add_to(this: *mut c_void) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || act(element_of!(this, selection_item), Action::AddToSelection))
}

unsafe extern "system" fn item_remove_from(this: *mut c_void) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || act(element_of!(this, selection_item), Action::RemoveFromSelection))
}

unsafe extern "system" fn item_is_selected(this: *mut c_void, out: *mut BOOL) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        let element = element_of!(this, selection_item);
        write(out, node_of(element).map(|n| BOOL::from(n.selected == Some(true))))
    })
}

unsafe extern "system" fn item_container(this: *mut c_void, out: *mut *mut c_void) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        let element = element_of!(this, selection_item);
        let parent = match node_of(element) {
            Ok(node) => node.parent,
            Err(hr) => return hr,
        };
        write(out, Ok(simple_of(element, parent)))
    })
}

// ---------------------------------------------------------------------------
// Scroll
// ---------------------------------------------------------------------------

unsafe extern "system" fn scroll_scroll(this: *mut c_void, horizontal: i32, vertical: i32) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || act(element_of!(this, scroll), Action::Scroll(horizontal, vertical)))
}

unsafe extern "system" fn scroll_set_percent(this: *mut c_void, horizontal: f64, vertical: f64) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        let valid = |v: f64| v == -1.0 || (0.0..=100.0).contains(&v);
        if !valid(horizontal) || !valid(vertical) {
            return E_INVALIDARG;
        }
        act(element_of!(this, scroll), Action::ScrollTo(horizontal, vertical))
    })
}

macro_rules! scroll_getter {
    ($name:ident, $out:ty, $map:expr) => {
        unsafe extern "system" fn $name(this: *mut c_void, out: *mut $out) -> HRESULT {
            let element = element_of!(this, scroll);
            let map: fn(ScrollState) -> $out = $map;
            write(out, node_of(element).map(|n| map(n.scroll.unwrap_or_default())))
        }
    };
}

scroll_getter!(scroll_h_percent, f64, |s| s.horizontal);
scroll_getter!(scroll_v_percent, f64, |s| s.vertical);
scroll_getter!(scroll_h_view, f64, |s| s.horizontal_view);
scroll_getter!(scroll_v_view, f64, |s| s.vertical_view);
scroll_getter!(scroll_h_scrollable, BOOL, |s| BOOL::from(s.horizontal >= 0.0));
scroll_getter!(scroll_v_scrollable, BOOL, |s| BOOL::from(s.vertical >= 0.0));

// ---------------------------------------------------------------------------
// Grid / GridItem
// ---------------------------------------------------------------------------

unsafe extern "system" fn grid_get_item(this: *mut c_void, row: i32, column: i32, out: *mut *mut c_void) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if out.is_null() {
            return E_POINTER;
        }
        *out = std::ptr::null_mut();
        let element = element_of!(this, grid);
        let Some(host) = element.host() else { return UIA_E_ELEMENTNOTAVAILABLE };
        let nodes = host.tree.nodes();
        // Células são filhas da grade ou netas (linhas como grupos).
        let inside =
            |n: &Node| n.parent == element.id || nodes.iter().any(|p| p.id == n.parent && p.parent == element.id);
        match nodes.iter().find(|n| n.cell == Some((row, column)) && inside(n)) {
            Some(cell) => {
                *out = simple_of(element, cell.id);
                S_OK
            }
            None => E_INVALIDARG,
        }
    })
}

unsafe extern "system" fn grid_rows(this: *mut c_void, out: *mut i32) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        let element = element_of!(this, grid);
        write(out, node_of(element).map(|n| n.grid.map_or(0, |g| g.0)))
    })
}

unsafe extern "system" fn grid_columns(this: *mut c_void, out: *mut i32) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        let element = element_of!(this, grid);
        write(out, node_of(element).map(|n| n.grid.map_or(0, |g| g.1)))
    })
}

unsafe extern "system" fn cell_row(this: *mut c_void, out: *mut i32) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        let element = element_of!(this, grid_item);
        write(out, node_of(element).map(|n| n.cell.map_or(0, |c| c.0)))
    })
}

unsafe extern "system" fn cell_column(this: *mut c_void, out: *mut i32) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        let element = element_of!(this, grid_item);
        write(out, node_of(element).map(|n| n.cell.map_or(0, |c| c.1)))
    })
}

unsafe extern "system" fn cell_span(_this: *mut c_void, out: *mut i32) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || write(out, Ok(1)))
}

unsafe extern "system" fn cell_grid(this: *mut c_void, out: *mut *mut c_void) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        let element = element_of!(this, grid_item);
        let Some(host) = element.host() else { return UIA_E_ELEMENTNOTAVAILABLE };
        let nodes = host.tree.nodes();
        // Sobe até o primeiro ancestral com o padrão Grid (ou a raiz).
        let mut current = nodes.iter().find(|n| n.id == element.id).map(|n| n.parent);
        while let Some(id) = current {
            if id == 0 {
                break;
            }
            match nodes.iter().find(|n| n.id == id) {
                Some(n) if n.grid.is_some() => break,
                Some(n) => current = Some(n.parent),
                None => current = None,
            }
        }
        write(out, Ok(simple_of(element, current.unwrap_or(0))))
    })
}

// ---------------------------------------------------------------------------
// RangeValue
// ---------------------------------------------------------------------------

unsafe extern "system" fn range_set(this: *mut c_void, value: f64) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        let element = element_of!(this, range);
        match node_of(element) {
            Ok(node) => match node.range {
                Some(range) if range.read_only => UIA_E_ELEMENTNOTENABLED,
                Some(range) if !(range.minimum..=range.maximum).contains(&value) => E_INVALIDARG,
                Some(_) => act(element, Action::SetRange(value)),
                None => UIA_E_INVALIDOPERATION,
            },
            Err(hr) => hr,
        }
    })
}

macro_rules! range_getter {
    ($name:ident, $out:ty, $map:expr) => {
        unsafe extern "system" fn $name(this: *mut c_void, out: *mut $out) -> HRESULT {
            let element = element_of!(this, range);
            let map: fn(RangeState) -> $out = $map;
            write(out, node_of(element).map(|n| map(n.range.unwrap_or_default())))
        }
    };
}

range_getter!(range_value, f64, |r| r.value);
range_getter!(range_read_only, BOOL, |r| BOOL::from(r.read_only));
range_getter!(range_maximum, f64, |r| r.maximum);
range_getter!(range_minimum, f64, |r| r.minimum);
range_getter!(range_large, f64, |r| r.large_change);
range_getter!(range_small, f64, |r| r.small_change);

// ---------------------------------------------------------------------------
// AdviseEvents
// ---------------------------------------------------------------------------

unsafe extern "system" fn advise_added(this: *mut c_void, event: i32, _properties: *mut c_void) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        let element = element_of!(this, advise);
        if let Some(host) = element.host() {
            host.advise_seen.set(true);
            *host.advised.borrow_mut().entry(event).or_insert(0) += 1;
        }
        S_OK
    })
}

unsafe extern "system" fn advise_removed(this: *mut c_void, event: i32, _properties: *mut c_void) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        let element = element_of!(this, advise);
        if let Some(host) = element.host() {
            if let Some(count) = host.advised.borrow_mut().get_mut(&event) {
                *count = count.saturating_sub(1);
            }
        }
        S_OK
    })
}

// ---------------------------------------------------------------------------
// Text e TextRange
// ---------------------------------------------------------------------------

/// UIA text units.
#[allow(dead_code)] // Formato, página e documento caem no ramo do documento inteiro.
pub(crate) mod unit {
    pub const CHARACTER: i32 = 0;
    pub const FORMAT: i32 = 1;
    pub const WORD: i32 = 2;
    pub const LINE: i32 = 3;
    pub const PARAGRAPH: i32 = 4;
    pub const PAGE: i32 = 5;
    pub const DOCUMENT: i32 = 6;
}

/// Start and end of the unit containing `at` (UTF-16).
pub(crate) fn unit_bounds(text: &[u16], at: usize, unit: i32) -> (usize, usize) {
    let len = text.len();
    let at = at.min(len);
    let is_space = |c: u16| c == b' ' as u16 || c == b'\t' as u16 || c == b'\r' as u16 || c == b'\n' as u16;
    match unit {
        unit::CHARACTER => {
            if at >= len {
                return (len, len);
            }
            // Par substituto: os dois juntos formam um caractere.
            let end = if (0xD800..0xDC00).contains(&text[at]) && at + 1 < len { at + 2 } else { at + 1 };
            let start = if (0xDC00..0xE000).contains(&text[at]) && at > 0 { at - 1 } else { at };
            (start, end)
        }
        unit::WORD => {
            let mut start = at;
            while start > 0 && !is_space(text[start - 1]) && !(start < len && is_space(text[start])) {
                start -= 1;
            }
            let mut end = at;
            while end < len && !is_space(text[end]) {
                end += 1;
            }
            // A palavra leva os espaços seguintes (como no Bloco de Notas).
            while end < len && is_space(text[end]) && text[end] != b'\n' as u16 {
                end += 1;
            }
            (start, end.max(at.min(len)))
        }
        unit::LINE | unit::PARAGRAPH => {
            let newline = b'\n' as u16;
            let mut start = at;
            while start > 0 && text[start - 1] != newline {
                start -= 1;
            }
            let mut end = at;
            while end < len && text[end] != newline {
                end += 1;
            }
            (start, (end + 1).min(len))
        }
        _ => (0, len),
    }
}

/// Moves `at` by `count` units; returns the new position and the number of units moved.
pub(crate) fn move_by(text: &[u16], at: usize, unit: i32, count: i32) -> (usize, i32) {
    let mut position = at.min(text.len());
    let mut moved = 0;
    if count > 0 {
        while moved < count && position < text.len() {
            let (_, end) = unit_bounds(text, position, unit);
            position = end.max(position + 1).min(text.len());
            moved += 1;
        }
    } else {
        while moved > count && position > 0 {
            let (start, _) = unit_bounds(text, position - 1, unit);
            position = start;
            moved -= 1;
        }
    }
    (position, moved)
}

#[repr(C)]
struct RangeObjectVtbl {
    qi: QiFn<c_void>,
    add_ref: RefFn<c_void>,
    release: RefFn<c_void>,
    clone: HrPtr,
    compare: unsafe extern "system" fn(*mut c_void, *mut c_void, *mut BOOL) -> HRESULT,
    compare_endpoints: unsafe extern "system" fn(*mut c_void, i32, *mut c_void, i32, *mut i32) -> HRESULT,
    expand: unsafe extern "system" fn(*mut c_void, i32) -> HRESULT,
    find_attribute: unsafe extern "system" fn(*mut c_void, i32, VARIANT, BOOL, *mut *mut c_void) -> HRESULT,
    find_text: unsafe extern "system" fn(*mut c_void, *mut u16, BOOL, BOOL, *mut *mut c_void) -> HRESULT,
    attribute: unsafe extern "system" fn(*mut c_void, i32, *mut VARIANT) -> HRESULT,
    rectangles: HrPtr,
    enclosing: HrPtr,
    text: unsafe extern "system" fn(*mut c_void, i32, *mut *mut u16) -> HRESULT,
    move_: unsafe extern "system" fn(*mut c_void, i32, i32, *mut i32) -> HRESULT,
    move_endpoint_by_unit: unsafe extern "system" fn(*mut c_void, i32, i32, i32, *mut i32) -> HRESULT,
    move_endpoint_by_range: unsafe extern "system" fn(*mut c_void, i32, *mut c_void, i32) -> HRESULT,
    select: Hr0,
    add_to_selection: Hr0,
    remove_from_selection: Hr0,
    scroll_into_view: unsafe extern "system" fn(*mut c_void, BOOL) -> HRESULT,
    children: HrPtr,
}

/// Element text in UTF-16 and its selection.
type RangeText = (Vec<u16>, Option<(usize, usize)>);

/// A [start, end) range of an element's text (UTF-16 units).
#[repr(C)]
struct TextRange {
    vtable: *const RangeObjectVtbl,
    references: Cell<u32>,
    host: Weak<TreeHost>,
    thread: u32,
    id: u32,
    start: Cell<usize>,
    end: Cell<usize>,
}

static RANGE_OBJECT_VTBL: RangeObjectVtbl = RangeObjectVtbl {
    qi: tr_qi,
    add_ref: tr_add,
    release: tr_release,
    clone: tr_clone,
    compare: tr_compare,
    compare_endpoints: tr_compare_endpoints,
    expand: tr_expand,
    find_attribute: tr_find_attribute,
    find_text: tr_find_text,
    attribute: tr_attribute,
    rectangles: tr_rectangles,
    enclosing: tr_enclosing,
    text: tr_text,
    move_: tr_move,
    move_endpoint_by_unit: tr_move_endpoint,
    move_endpoint_by_range: tr_move_endpoint_by_range,
    select: tr_select,
    add_to_selection: tr_select,
    remove_from_selection: tr_unsupported,
    scroll_into_view: tr_scroll,
    children: tr_children,
};

fn new_range(host: Weak<TreeHost>, thread: u32, id: u32, start: usize, end: usize) -> *mut c_void {
    Box::into_raw(Box::new(TextRange {
        vtable: &RANGE_OBJECT_VTBL,
        references: Cell::new(1),
        host,
        thread,
        id,
        start: Cell::new(start.min(end)),
        end: Cell::new(end.max(start)),
    })) as *mut c_void
}

impl TextRange {
    fn text(&self) -> Option<RangeText> {
        // SAFETY: consulta simples.
        if unsafe { GetCurrentThreadId() } != self.thread {
            return None;
        }
        let host = self.host.upgrade()?;
        let node = host.node(self.id)?;
        let (text, selection) = node.text?;
        Some((text.encode_utf16().collect(), selection))
    }

    fn clamp(&self, len: usize) {
        self.end.set(self.end.get().min(len));
        self.start.set(self.start.get().min(self.end.get()));
    }
}

unsafe fn range_of<'a>(this: *mut c_void) -> &'a TextRange {
    &*(this as *const TextRange)
}

unsafe extern "system" fn tr_qi(this: *mut c_void, iid: *const GUID, out: *mut *mut c_void) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if out.is_null() || iid.is_null() {
            return E_POINTER;
        }
        if *iid == IID_IUnknown || *iid == IID_ITextRangeProvider {
            tr_add(this);
            *out = this;
            S_OK
        } else {
            *out = std::ptr::null_mut();
            E_NOINTERFACE
        }
    })
}

unsafe extern "system" fn tr_add(this: *mut c_void) -> u32 {
    crate::core::ffi_guard::guard(0, move || {
        let range = range_of(this);
        let count = range.references.get() + 1;
        range.references.set(count);
        count
    })
}

unsafe extern "system" fn tr_release(this: *mut c_void) -> u32 {
    crate::core::ffi_guard::guard(0, move || {
        let range = range_of(this);
        let count = range.references.get() - 1;
        range.references.set(count);
        if count == 0 {
            drop(Box::from_raw(this as *mut TextRange));
        }
        count
    })
}

/// Another range of the same element (`None` if it belongs to another object).
unsafe fn other_range<'a>(this: &TextRange, other: *mut c_void) -> Option<&'a TextRange> {
    if other.is_null() || !std::ptr::eq(*(other as *const *const RangeObjectVtbl), &RANGE_OBJECT_VTBL) {
        return None;
    }
    let other = range_of(other);
    (other.id == this.id && Weak::ptr_eq(&other.host, &this.host)).then_some(other)
}

unsafe extern "system" fn tr_clone(this: *mut c_void, out: *mut *mut c_void) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        let r = range_of(this);
        write(out, Ok(new_range(r.host.clone(), r.thread, r.id, r.start.get(), r.end.get())))
    })
}

unsafe extern "system" fn tr_compare(this: *mut c_void, other: *mut c_void, out: *mut BOOL) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        let r = range_of(this);
        let same = other_range(r, other).is_some_and(|o| o.start.get() == r.start.get() && o.end.get() == r.end.get());
        write(out, Ok(BOOL::from(same)))
    })
}

fn endpoint(range: &TextRange, which: i32) -> usize {
    // 0 = TextPatternRangeEndpoint_Start, 1 = End.
    if which == 0 {
        range.start.get()
    } else {
        range.end.get()
    }
}

unsafe extern "system" fn tr_compare_endpoints(
    this: *mut c_void,
    which: i32,
    other: *mut c_void,
    other_which: i32,
    out: *mut i32,
) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        let r = range_of(this);
        let Some(o) = other_range(r, other) else { return E_INVALIDARG };
        write(out, Ok(endpoint(r, which) as i32 - endpoint(o, other_which) as i32))
    })
}

unsafe extern "system" fn tr_expand(this: *mut c_void, unit: i32) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        let r = range_of(this);
        let Some((text, _)) = r.text() else { return UIA_E_ELEMENTNOTAVAILABLE };
        r.clamp(text.len());
        let (start, _) = unit_bounds(&text, r.start.get(), unit);
        let (_, end) = unit_bounds(&text, r.start.get(), unit);
        r.start.set(start);
        r.end.set(end.max(start));
        S_OK
    })
}

unsafe extern "system" fn tr_find_attribute(
    _this: *mut c_void,
    _attribute: i32,
    _value: VARIANT,
    _backward: BOOL,
    out: *mut *mut c_void,
) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        // Texto sem formatação: nenhum trecho tem atributo diferente do resto.
        write(out, Ok(std::ptr::null_mut()))
    })
}

unsafe extern "system" fn tr_find_text(
    this: *mut c_void,
    wanted: *mut u16,
    backward: BOOL,
    ignore_case: BOOL,
    out: *mut *mut c_void,
) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if out.is_null() || wanted.is_null() {
            return E_POINTER;
        }
        *out = std::ptr::null_mut();
        let r = range_of(this);
        let Some((text, _)) = r.text() else { return UIA_E_ELEMENTNOTAVAILABLE };
        r.clamp(text.len());
        let haystack = String::from_utf16_lossy(&text[r.start.get()..r.end.get()]);
        let needle = from_wide_ptr_lossy(wanted);
        if needle.is_empty() {
            return S_OK;
        }
        let (hay, needle) = if ignore_case != 0 {
            (haystack.to_lowercase(), needle.to_lowercase())
        } else {
            (haystack.clone(), needle)
        };
        let found = if backward != 0 { hay.rfind(&needle) } else { hay.find(&needle) };
        if let Some(byte) = found {
            let before: usize = haystack[..byte.min(haystack.len())].encode_utf16().count();
            let len: usize = needle.encode_utf16().count();
            let start = r.start.get() + before;
            *out = new_range(r.host.clone(), r.thread, r.id, start, start + len);
        }
        S_OK
    })
}

unsafe extern "system" fn tr_attribute(_this: *mut c_void, _attribute: i32, out: *mut VARIANT) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if out.is_null() {
            return E_POINTER;
        }
        // Atributo não suportado: o valor reservado do UIA (ou vazio, sem a função).
        *out = VARIANT { vt: VT_EMPTY, reserved: [0; 3], data: [0; 2] };
        if let Some(reserved) = optional().UiaGetReservedNotSupportedValue {
            let mut unknown: *mut c_void = std::ptr::null_mut();
            if reserved(&mut unknown) >= 0 && !unknown.is_null() {
                // O valor reservado é um singleton; a VARIANT recebe uma referência.
                let vtable = *(unknown as *const *const [usize; 3]);
                let add: RefFn<c_void> = std::mem::transmute((*vtable)[1]);
                add(unknown);
                *out = VARIANT { vt: VT_UNKNOWN, reserved: [0; 3], data: [unknown as usize, 0] };
            }
        }
        S_OK
    })
}

unsafe extern "system" fn tr_rectangles(this: *mut c_void, out: *mut *mut c_void) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if out.is_null() {
            return E_POINTER;
        }
        let r = range_of(this);
        // Sem medição por caractere: o retângulo do elemento inteiro quando há texto.
        let bounds = r.host.upgrade().and_then(|h| h.node(r.id)).map(|n| n.bounds).unwrap_or_default();
        let values: Vec<f64> = if r.end.get() > r.start.get() && bounds.right > bounds.left {
            vec![
                bounds.left as f64,
                bounds.top as f64,
                (bounds.right - bounds.left) as f64,
                (bounds.bottom - bounds.top) as f64,
            ]
        } else {
            Vec::new()
        };
        let array = SafeArrayCreateVector(VT_R8, 0, values.len() as u32);
        for (index, value) in values.iter().enumerate() {
            let index = index as i32;
            if !array.is_null() {
                SafeArrayPutElement(array, &index, value as *const f64 as *const c_void);
            }
        }
        *out = array;
        S_OK
    })
}

unsafe extern "system" fn tr_enclosing(this: *mut c_void, out: *mut *mut c_void) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if out.is_null() {
            return E_POINTER;
        }
        let r = range_of(this);
        *out = std::ptr::null_mut();
        if let Some(host) = r.host.upgrade() {
            if r.id == 0 {
                if let Some(root) = host.root_provider() {
                    let raw = root.as_ptr() as *mut Element;
                    add_ref(&*raw);
                    *out = (*raw).pointer_to(&(*raw).simple);
                }
            } else {
                let created = new_element(r.host.clone(), r.id, r.thread);
                let raw = created.as_ptr() as *mut Element;
                std::mem::forget(created);
                *out = (*raw).pointer_to(&(*raw).simple);
            }
        }
        S_OK
    })
}

unsafe extern "system" fn tr_text(this: *mut c_void, max: i32, out: *mut *mut u16) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if out.is_null() {
            return E_POINTER;
        }
        let r = range_of(this);
        let Some((text, _)) = r.text() else { return UIA_E_ELEMENTNOTAVAILABLE };
        r.clamp(text.len());
        let mut slice = &text[r.start.get()..r.end.get()];
        if max >= 0 && slice.len() > max as usize {
            slice = &slice[..max as usize];
        }
        *out = bstr(&String::from_utf16_lossy(slice));
        S_OK
    })
}

unsafe extern "system" fn tr_move(this: *mut c_void, unit: i32, count: i32, out: *mut i32) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        let r = range_of(this);
        let Some((text, _)) = r.text() else { return UIA_E_ELEMENTNOTAVAILABLE };
        r.clamp(text.len());
        let degenerate = r.start.get() == r.end.get();
        // Normaliza para o início da unidade, anda e expande para uma unidade inteira.
        let (start, _) = unit_bounds(&text, r.start.get(), unit);
        let (position, moved) = move_by(&text, start, unit, count);
        let (_, end) = unit_bounds(&text, position, unit);
        r.start.set(position);
        r.end.set(if degenerate { position } else { end.max(position) });
        write(out, Ok(moved))
    })
}

unsafe extern "system" fn tr_move_endpoint(
    this: *mut c_void,
    which: i32,
    unit: i32,
    count: i32,
    out: *mut i32,
) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        let r = range_of(this);
        let Some((text, _)) = r.text() else { return UIA_E_ELEMENTNOTAVAILABLE };
        r.clamp(text.len());
        let (position, moved) = move_by(&text, endpoint(r, which), unit, count);
        if which == 0 {
            r.start.set(position);
            if position > r.end.get() {
                r.end.set(position);
            }
        } else {
            r.end.set(position);
            if position < r.start.get() {
                r.start.set(position);
            }
        }
        write(out, Ok(moved))
    })
}

unsafe extern "system" fn tr_move_endpoint_by_range(
    this: *mut c_void,
    which: i32,
    other: *mut c_void,
    other_which: i32,
) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        let r = range_of(this);
        let Some(o) = other_range(r, other) else { return E_INVALIDARG };
        let position = endpoint(o, other_which);
        if which == 0 {
            r.start.set(position);
            r.end.set(r.end.get().max(position));
        } else {
            r.end.set(position);
            r.start.set(r.start.get().min(position));
        }
        S_OK
    })
}

unsafe extern "system" fn tr_select(this: *mut c_void) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        let r = range_of(this);
        let Some(host) = r.host.upgrade() else { return UIA_E_ELEMENTNOTAVAILABLE };
        let element = new_element(r.host.clone(), r.id, r.thread);
        let raw = &*(element.as_ptr() as *mut Element);
        let _ = host;
        act(raw, Action::SelectText(r.start.get(), r.end.get()))
    })
}

unsafe extern "system" fn tr_unsupported(_this: *mut c_void) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || UIA_E_INVALIDOPERATION)
}

unsafe extern "system" fn tr_scroll(_this: *mut c_void, _top: BOOL) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || S_OK)
}

unsafe extern "system" fn tr_children(_this: *mut c_void, out: *mut *mut c_void) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || write(out, Ok(SafeArrayCreateVector(VT_UNKNOWN, 0, 0))))
}

// --- ITextProvider ---

unsafe extern "system" fn text_selection(this: *mut c_void, out: *mut *mut c_void) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if out.is_null() {
            return E_POINTER;
        }
        let element = element_of!(this, text);
        let node = match node_of(element) {
            Ok(node) => node,
            Err(hr) => return hr,
        };
        let ranges = match node.text.and_then(|(_, selection)| selection) {
            Some((start, end)) => vec![new_range(element.host.clone(), element.thread, element.id, start, end)],
            None => Vec::new(),
        };
        *out = unknown_array(ranges);
        S_OK
    })
}

unsafe extern "system" fn text_document(this: *mut c_void, out: *mut *mut c_void) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        let element = element_of!(this, text);
        let len = match node_of(element) {
            Ok(node) => node.text.map(|(t, _)| t.encode_utf16().count()).unwrap_or(0),
            Err(hr) => return hr,
        };
        write(out, Ok(new_range(element.host.clone(), element.thread, element.id, 0, len)))
    })
}

unsafe extern "system" fn text_visible(this: *mut c_void, out: *mut *mut c_void) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if out.is_null() {
            return E_POINTER;
        }
        let mut document: *mut c_void = std::ptr::null_mut();
        let hr = text_document(this, &mut document);
        if hr != S_OK {
            return hr;
        }
        *out = unknown_array(vec![document]);
        S_OK
    })
}

unsafe extern "system" fn text_from_child(this: *mut c_void, _child: *mut c_void, out: *mut *mut c_void) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || text_document(this, out))
}

unsafe extern "system" fn text_from_point(this: *mut c_void, _point: UiaPoint, out: *mut *mut c_void) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        let hr = text_document(this, out);
        if hr == S_OK && !(*out).is_null() {
            // Ponto sem medição por caractere: intervalo degenerado no início.
            let range = range_of(*out);
            range.end.set(0);
        }
        hr
    })
}

unsafe extern "system" fn text_supported(this: *mut c_void, out: *mut i32) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        let element = element_of!(this, text);
        // 0 = SupportedTextSelection_None, 1 = Single.
        write(out, node_of(element).map(|n| i32::from(n.text.is_some_and(|(_, s)| s.is_some()))))
    })
}

/// Direct pattern calls for integration tests.
pub(crate) mod debug {
    use super::*;

    /// Document range text after moving `count` units from the start and expanding to the unit.
    pub(crate) unsafe fn text_unit_at(element: *mut c_void, unit: i32, count: i32) -> Option<String> {
        let element = &*(element as *mut Element);
        let mut document: *mut c_void = std::ptr::null_mut();
        if text_document(element.pointer_to(&element.text), &mut document) != S_OK || document.is_null() {
            return None;
        }
        let mut moved = 0;
        let range = range_of(document);
        range.end.set(range.start.get());
        tr_move(document, unit, count, &mut moved);
        tr_expand(document, unit);
        let mut text: *mut u16 = std::ptr::null_mut();
        tr_text(document, -1, &mut text);
        let result = from_wide_ptr_lossy(text);
        SysFreeString(text);
        tr_release(document);
        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().collect()
    }

    #[test]
    fn text_units_split_characters_words_and_lines() {
        let text = wide("olá mundo\nsegunda linha");
        assert_eq!(unit_bounds(&text, 0, unit::CHARACTER), (0, 1));
        assert_eq!(unit_bounds(&text, 1, unit::WORD), (0, 4), "a palavra leva o espaço seguinte");
        assert_eq!(unit_bounds(&text, 5, unit::WORD), (4, 9));
        assert_eq!(unit_bounds(&text, 2, unit::LINE), (0, 10));
        assert_eq!(unit_bounds(&text, 12, unit::LINE), (10, text.len()));
        assert_eq!(unit_bounds(&text, 3, unit::DOCUMENT), (0, text.len()));
        assert_eq!(move_by(&text, 0, unit::WORD, 2), (9, 2), "para antes da quebra de linha");
        assert_eq!(move_by(&text, text.len(), unit::LINE, -1), (10, -1));
        assert_eq!(move_by(&text, 0, unit::CHARACTER, -3), (0, 0));
        let emoji = wide("a😀b");
        assert_eq!(unit_bounds(&emoji, 1, unit::CHARACTER), (1, 3), "par substituto é um caractere");
        assert_eq!(unit_bounds(&emoji, 2, unit::CHARACTER), (1, 3));
        assert_eq!(move_by(&emoji, 0, unit::CHARACTER, 2), (3, 2));
    }
}
