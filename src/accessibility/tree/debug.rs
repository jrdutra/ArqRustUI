//! Direct COM method calls for integration tests.

use super::*;

/// Element summary as UIA would see it.
#[derive(Clone, Debug, PartialEq)]
pub struct ElementInfo {
    /// Control type.
    pub control_type: i32,
    /// Name.
    pub name: String,
    /// Supported patterns (Invoke, Toggle, ExpandCollapse, Value).
    pub patterns: [bool; 4],
    /// Toggle / ExpandCollapse state (0/1), if available.
    pub states: (Option<i32>, Option<i32>),
    /// Value (Value pattern).
    pub value: Option<String>,
    /// Has keyboard focus.
    pub focused: bool,
}

/// Children (recursive, preorder) of the root provider published on `HWND`, as (depth, element).
///
/// # Safety
/// `hwnd` must be a window on this thread.
pub unsafe fn walk(hwnd: isize) -> Vec<(usize, ElementInfo)> {
    let Some(root) = root_of(hwnd as HWND) else { return Vec::new() };
    let mut out = Vec::new();
    visit(root.as_ptr(), 0, &mut out);
    out
}

/// Performs an action ("invoke", "toggle", "expand", "collapse", "focus" or `"set:<value>"`) on the element named
/// `name`; returns the HRESULT.
///
/// # Safety
/// `hwnd` must be a window on this thread.
pub unsafe fn perform(hwnd: isize, name: &str, action: &str) -> i32 {
    let Some(root) = root_of(hwnd as HWND) else { return -1 };
    let target = if name.is_empty() {
        let raw = root.as_ptr();
        add_ref(&*(raw as *mut Element));
        Some(raw)
    } else {
        find(root.as_ptr(), name)
    };
    let Some(target) = target else { return -2 };
    let element = &*(target as *mut Element);
    let hr = match action {
        "invoke" => invoke_invoke(element.pointer_to(&element.invoke)),
        "toggle" => toggle_toggle(element.pointer_to(&element.toggle)),
        "expand" => expand_expand(element.pointer_to(&element.expand)),
        "collapse" => expand_collapse(element.pointer_to(&element.expand)),
        "focus" => fragment_set_focus(element.pointer_to(&element.fragment)),
        other => match other.strip_prefix("set:") {
            Some(value) => {
                let wide = to_wide(value);
                value_set(element.pointer_to(&element.value), wide.as_ptr())
            }
            None => -3,
        },
    };
    release(element);
    hr
}

/// Additional patterns on the element named `name` (`""` = root): Selection, SelectionItem, Scroll, Grid, GridItem,
/// RangeValue and Text.
///
/// # Safety
/// `hwnd` must be a window on this thread.
pub unsafe fn extra_patterns(hwnd: isize, name: &str) -> [bool; 7] {
    let mut result = [false; 7];
    let Some(root) = root_of(hwnd as HWND) else { return result };
    let target = if name.is_empty() {
        let raw = root.as_ptr();
        add_ref(&*(raw as *mut Element));
        Some(raw)
    } else {
        find(root.as_ptr(), name)
    };
    let Some(target) = target else { return result };
    let element = &*(target as *mut Element);
    let simple = element.pointer_to(&element.simple);
    for (slot, pattern) in [
        UIA_SELECTION_PATTERN_ID,
        UIA_SELECTION_ITEM_PATTERN_ID,
        UIA_SCROLL_PATTERN_ID,
        UIA_GRID_PATTERN_ID,
        UIA_GRID_ITEM_PATTERN_ID,
        UIA_RANGE_VALUE_PATTERN_ID,
        UIA_TEXT_PATTERN_ID,
    ]
    .into_iter()
    .enumerate()
    {
        let mut out: *mut c_void = std::ptr::null_mut();
        simple_pattern(simple, pattern, &mut out);
        if !out.is_null() {
            result[slot] = true;
            release(element);
        }
    }
    release(element);
    result
}

/// Reads a numeric property (`VT_R8`/`VT_I4`/`VT_BOOL`) of element `name` (`""` = root).
///
/// # Safety
/// `hwnd` must be a window on this thread.
pub unsafe fn number_property(hwnd: isize, name: &str, property: i32) -> Option<f64> {
    let root = root_of(hwnd as HWND)?;
    let target = if name.is_empty() {
        let raw = root.as_ptr();
        add_ref(&*(raw as *mut Element));
        raw
    } else {
        find(root.as_ptr(), name)?
    };
    let element = &*(target as *mut Element);
    let mut variant = VARIANT { vt: 0, reserved: [0; 3], data: [0; 2] };
    simple_property(element.pointer_to(&element.simple), property, &mut variant);
    release(element);
    match variant.vt {
        VT_R8 => Some(variant.as_f64()),
        VT_I4 => Some(variant.data[0] as i32 as f64),
        VT_BOOL => Some(if variant.data[0] != 0 { 1.0 } else { 0.0 }),
        _ => None,
    }
}

/// Additional pattern action on `name` (`""` = root): `"select"`, `"add"`, `"remove"`, `"range:<v>"`,
/// `"scroll:<h>,<v>"`, `"scrollto:<h>,<v>"`.
///
/// # Safety
/// `hwnd` must be a window on this thread.
pub unsafe fn perform_extra(hwnd: isize, name: &str, action: &str) -> i32 {
    let Some(root) = root_of(hwnd as HWND) else { return -1 };
    let target = if name.is_empty() {
        let raw = root.as_ptr();
        add_ref(&*(raw as *mut Element));
        Some(raw)
    } else {
        find(root.as_ptr(), name)
    };
    let Some(target) = target else { return -2 };
    let element = &*(target as *mut Element);
    let pair = |text: &str| -> (f64, f64) {
        let mut parts = text.split(',').map(|p| p.trim().parse::<f64>().unwrap_or(-1.0));
        (parts.next().unwrap_or(-1.0), parts.next().unwrap_or(-1.0))
    };
    let (vt_selection, vt_scroll, vt_range) = (
        element.pointer_to(&element.selection_item),
        element.pointer_to(&element.scroll),
        element.pointer_to(&element.range),
    );
    let hr = match action {
        "select" => ((*element.selection_item).select)(vt_selection),
        "add" => ((*element.selection_item).add_to_selection)(vt_selection),
        "remove" => ((*element.selection_item).remove_from_selection)(vt_selection),
        other => {
            if let Some(value) = other.strip_prefix("range:") {
                ((*element.range).set_value)(vt_range, value.parse().unwrap_or(f64::NAN))
            } else if let Some(values) = other.strip_prefix("scrollto:") {
                let (h, v) = pair(values);
                ((*element.scroll).set_scroll_percent)(vt_scroll, h, v)
            } else if let Some(values) = other.strip_prefix("scroll:") {
                let (h, v) = pair(values);
                ((*element.scroll).scroll)(vt_scroll, h as i32, v as i32)
            } else {
                -3
            }
        }
    };
    release(element);
    hr
}

/// Names of selected elements (`ISelectionProvider::GetSelection`) of `name` (`""` = root).
///
/// # Safety
/// `hwnd` must be a window on this thread.
pub unsafe fn selection_names(hwnd: isize, name: &str) -> Vec<String> {
    let Some(root) = root_of(hwnd as HWND) else { return Vec::new() };
    let target = if name.is_empty() {
        let raw = root.as_ptr();
        add_ref(&*(raw as *mut Element));
        Some(raw)
    } else {
        find(root.as_ptr(), name)
    };
    let Some(target) = target else { return Vec::new() };
    let element = &*(target as *mut Element);
    let mut array: *mut c_void = std::ptr::null_mut();
    ((*element.selection).get_selection)(element.pointer_to(&element.selection), &mut array);
    release(element);
    let mut names = Vec::new();
    if array.is_null() {
        return names;
    }
    let mut upper = -1;
    SafeArrayGetUBound(array, 1, &mut upper);
    for index in 0..=upper {
        let mut item: *mut c_void = std::ptr::null_mut();
        SafeArrayGetElement(array, &index, &mut item as *mut *mut c_void as *mut c_void);
        if !item.is_null() {
            // `item` aponta para o campo `simple`, o primeiro do Element.
            names.push(info(item).name);
            release(&*(item as *mut Element));
        }
    }
    SafeArrayDestroy(array);
    names
}

/// Cell name (row, column) for the Grid pattern of `name` (`""` = root).
///
/// # Safety
/// `hwnd` must be a window on this thread.
pub unsafe fn grid_cell(hwnd: isize, name: &str, row: i32, column: i32) -> Option<String> {
    let root = root_of(hwnd as HWND)?;
    let target = if name.is_empty() {
        let raw = root.as_ptr();
        add_ref(&*(raw as *mut Element));
        raw
    } else {
        find(root.as_ptr(), name)?
    };
    let element = &*(target as *mut Element);
    let mut cell: *mut c_void = std::ptr::null_mut();
    let hr = ((*element.grid).get_item)(element.pointer_to(&element.grid), row, column, &mut cell);
    release(element);
    if hr != S_OK || cell.is_null() {
        return None;
    }
    let name = info(cell).name;
    release(&*(cell as *mut Element));
    Some(name)
}

/// Text of the `count`-th unit (0 = character, 2 = word, 3 = line) of the Text pattern of `name` (`""` = root).
///
/// # Safety
/// `hwnd` must be a window on this thread.
pub unsafe fn text_unit(hwnd: isize, name: &str, unit: i32, count: i32) -> Option<String> {
    let root = root_of(hwnd as HWND)?;
    let target = if name.is_empty() {
        let raw = root.as_ptr();
        add_ref(&*(raw as *mut Element));
        raw
    } else {
        find(root.as_ptr(), name)?
    };
    let result = super::patterns::debug::text_unit_at(target, unit, count);
    release(&*(target as *mut Element));
    result
}

/// Simulates `IRawElementProviderAdviseEvents` on the root (`added` = subscribe) and returns subscribed events.
///
/// # Safety
/// `hwnd` must be a window on this thread.
pub unsafe fn advise(hwnd: isize, event: i32, added: bool) -> Vec<i32> {
    let Some(root) = root_of(hwnd as HWND) else { return Vec::new() };
    let element = &*(root.as_ptr() as *mut Element);
    let advise = element.pointer_to(&element.advise);
    if added {
        ((*element.advise).added)(advise, event, std::ptr::null_mut());
    } else {
        ((*element.advise).removed)(advise, event, std::ptr::null_mut());
    }
    element.host().map(|h| h.advised_events()).unwrap_or_default()
}

/// Open menu list windows, from outermost to innermost.
pub fn menu_popups() -> Vec<isize> {
    crate::menus::popup::popup_hwnds().into_iter().map(|h| h as isize).collect()
}

unsafe fn root_of(hwnd: HWND) -> Option<ComPtr> {
    // Pergunta à janela como o UIA perguntaria (WM_GETOBJECT), mas lendo o provedor direto.
    if let Some(core) = crate::core::widget::lookup(hwnd) {
        if let Some(host) = core.accessible_tree.borrow().as_ref() {
            return host.root_provider();
        }
    }
    crate::menus::popup::accessible_root(hwnd)
}

unsafe fn info(raw: *mut c_void) -> ElementInfo {
    let element = &*(raw as *mut Element);
    let simple = element.pointer_to(&element.simple);
    let mut variant = VARIANT { vt: 0, reserved: [0; 3], data: [0; 2] };
    let read_text = |variant: &VARIANT| -> String {
        if variant.vt == VT_BSTR && variant.data[0] != 0 {
            let text = from_wide_ptr_lossy(variant.data[0] as *const u16);
            SysFreeString(variant.data[0] as *mut u16);
            text
        } else {
            String::new()
        }
    };
    simple_property(simple, UIA_CONTROL_TYPE_PROPERTY_ID, &mut variant);
    let control_type = if variant.vt == VT_I4 { variant.data[0] as i32 } else { 0 };
    simple_property(simple, UIA_NAME_PROPERTY_ID, &mut variant);
    let name = read_text(&variant);
    simple_property(simple, UIA_HAS_KEYBOARD_FOCUS_PROPERTY_ID, &mut variant);
    let focused = variant.vt == VT_BOOL && variant.data[0] != 0;
    let mut patterns = [false; 4];
    for (slot, pattern) in
        [UIA_INVOKE_PATTERN_ID, UIA_TOGGLE_PATTERN_ID, UIA_EXPAND_COLLAPSE_PATTERN_ID, UIA_VALUE_PATTERN_ID]
            .into_iter()
            .enumerate()
    {
        let mut out: *mut c_void = std::ptr::null_mut();
        simple_pattern(simple, pattern, &mut out);
        if !out.is_null() {
            patterns[slot] = true;
            release(element);
        }
    }
    let mut state = 0;
    let toggle =
        (patterns[1] && toggle_state(element.pointer_to(&element.toggle), &mut state) == S_OK).then_some(state);
    let expand =
        (patterns[2] && expand_state(element.pointer_to(&element.expand), &mut state) == S_OK).then_some(state);
    let value = if patterns[3] {
        let mut text: *mut u16 = std::ptr::null_mut();
        value_get(element.pointer_to(&element.value), &mut text);
        let value = from_wide_ptr_lossy(text);
        SysFreeString(text);
        Some(value)
    } else {
        None
    };
    ElementInfo { control_type, name, patterns, states: (toggle, expand), value, focused }
}

unsafe fn children(raw: *mut c_void) -> Vec<*mut c_void> {
    let element = &*(raw as *mut Element);
    let mut list = Vec::new();
    let mut current: *mut c_void = std::ptr::null_mut();
    fragment_navigate(element.pointer_to(&element.fragment), NAVIGATE_FIRST_CHILD, &mut current);
    while !current.is_null() {
        // `current` aponta para o campo `fragment`; guarda a identidade do objeto.
        let child = (current as *mut u8).sub(offset_of!(Element, fragment)) as *mut c_void;
        list.push(child);
        let mut next: *mut c_void = std::ptr::null_mut();
        fragment_navigate(current, NAVIGATE_NEXT, &mut next);
        current = next;
    }
    list
}

unsafe fn visit(raw: *mut c_void, depth: usize, out: &mut Vec<(usize, ElementInfo)>) {
    for child in children(raw) {
        out.push((depth, info(child)));
        visit(child, depth + 1, out);
        release(&*(child as *mut Element));
    }
}

unsafe fn find(raw: *mut c_void, name: &str) -> Option<*mut c_void> {
    let mut found = None;
    for child in children(raw) {
        if found.is_none() && info(child).name == name {
            found = Some(child);
            continue;
        }
        if found.is_none() {
            found = find(child, name);
        }
        release(&*(child as *mut Element));
    }
    found
}
