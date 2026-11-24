//! UI Automation provider COM methods: `IRawElementProviderSimple`, `IRawElementProviderFragment`,
//! `IRawElementProviderFragmentRoot` and Invoke, Toggle, ExpandCollapse and Value patterns. Vtables are in [`super`].

use super::*;

// --- IRawElementProviderSimple ---

pub(super) unsafe extern "system" fn simple_options(_this: *mut c_void, out: *mut i32) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if out.is_null() {
            return E_POINTER;
        }
        *out = PROVIDER_OPTIONS_SERVER_SIDE | PROVIDER_OPTIONS_USE_COM_THREADING;
        S_OK
    })
}

pub(super) unsafe extern "system" fn simple_pattern(this: *mut c_void, pattern: i32, out: *mut *mut c_void) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if out.is_null() {
            return E_POINTER;
        }
        *out = std::ptr::null_mut();
        let element = element_of!(this, simple);
        let iid = match pattern {
            UIA_INVOKE_PATTERN_ID => IID_IInvokeProvider,
            UIA_TOGGLE_PATTERN_ID => IID_IToggleProvider,
            UIA_EXPAND_COLLAPSE_PATTERN_ID => IID_IExpandCollapseProvider,
            UIA_VALUE_PATTERN_ID => IID_IValueProvider,
            UIA_SELECTION_PATTERN_ID => IID_ISelectionProvider,
            UIA_SELECTION_ITEM_PATTERN_ID => IID_ISelectionItemProvider,
            UIA_SCROLL_PATTERN_ID => IID_IScrollProvider,
            UIA_GRID_PATTERN_ID => IID_IGridProvider,
            UIA_GRID_ITEM_PATTERN_ID => IID_IGridItemProvider,
            UIA_RANGE_VALUE_PATTERN_ID => IID_IRangeValueProvider,
            UIA_TEXT_PATTERN_ID => IID_ITextProvider,
            _ => return S_OK,
        };
        // Sem o padrão, a resposta correta é S_OK com ponteiro nulo.
        let _ = query(element, &iid, out);
        S_OK
    })
}

pub(super) unsafe extern "system" fn simple_property(this: *mut c_void, property: i32, out: *mut VARIANT) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if out.is_null() {
            return E_POINTER;
        }
        *out = VARIANT { vt: VT_EMPTY, reserved: [0; 3], data: [0; 2] };
        let element = element_of!(this, simple);
        let Some(host) = element.host() else { return UIA_E_ELEMENTNOTAVAILABLE };
        if element.id == 0 {
            if let Some(value) = host.tree.root_node().and_then(|node| patterns::pattern_property(&node, property)) {
                *out = value;
                return S_OK;
            }
            *out = match property {
                UIA_CONTROL_TYPE_PROPERTY_ID => match host.tree.root_role() {
                    Some(role) => variant_i4(role),
                    None => return S_OK,
                },
                UIA_NAME_PROPERTY_ID => match host.name.borrow().clone().or_else(|| host.tree.root_name()) {
                    Some(name) => variant_text(&name),
                    None => return S_OK,
                },
                UIA_HELP_TEXT_PROPERTY_ID | UIA_FULL_DESCRIPTION_PROPERTY_ID => {
                    match host.description.borrow().as_deref() {
                        Some(text) => variant_text(text),
                        None => return S_OK,
                    }
                }
                UIA_FRAMEWORK_ID_PROPERTY_ID => variant_text("ArqRustUI"),
                _ => return S_OK,
            };
            return S_OK;
        }
        let nodes = host.tree.nodes();
        let Some(node) = nodes.iter().find(|n| n.id == element.id) else { return UIA_E_ELEMENTNOTAVAILABLE };
        let siblings: Vec<&Node> =
            nodes.iter().filter(|n| n.parent == node.parent && n.role != role::SEPARATOR).collect();
        *out = match property {
            UIA_CONTROL_TYPE_PROPERTY_ID => variant_i4(node.role),
            UIA_NAME_PROPERTY_ID => variant_text(&node.name),
            UIA_HELP_TEXT_PROPERTY_ID | UIA_FULL_DESCRIPTION_PROPERTY_ID if !node.help.is_empty() => {
                variant_text(&node.help)
            }
            UIA_ACCESS_KEY_PROPERTY_ID if !node.access_key.is_empty() => variant_text(&node.access_key),
            UIA_ACCELERATOR_KEY_PROPERTY_ID if !node.accelerator.is_empty() => variant_text(&node.accelerator),
            UIA_AUTOMATION_ID_PROPERTY_ID => variant_text(&format!("item{}", node.id)),
            UIA_CLASS_NAME_PROPERTY_ID => variant_text("ArqRustUI.Item"),
            UIA_FRAMEWORK_ID_PROPERTY_ID => variant_text("ArqRustUI"),
            UIA_IS_ENABLED_PROPERTY_ID => variant_bool(node.enabled),
            UIA_IS_KEYBOARD_FOCUSABLE_PROPERTY_ID => variant_bool(node.focusable),
            UIA_HAS_KEYBOARD_FOCUS_PROPERTY_ID => variant_bool(node.focused),
            UIA_IS_CONTROL_ELEMENT_PROPERTY_ID => variant_bool(true),
            UIA_IS_CONTENT_ELEMENT_PROPERTY_ID => variant_bool(node.role != role::SEPARATOR),
            UIA_IS_OFFSCREEN_PROPERTY_ID => variant_bool(node.offscreen()),
            UIA_TOGGLE_STATE_PROPERTY_ID if node.toggle.is_some() => variant_i4(i32::from(node.toggle == Some(true))),
            UIA_EXPAND_COLLAPSE_STATE_PROPERTY_ID if node.expanded.is_some() => {
                variant_i4(i32::from(node.expanded == Some(true)))
            }
            UIA_VALUE_VALUE_PROPERTY_ID if node.value.is_some() => {
                variant_text(&node.value.as_ref().map(|v| v.0.clone()).unwrap_or_default())
            }
            UIA_VALUE_IS_READ_ONLY_PROPERTY_ID if node.value.is_some() => {
                variant_bool(node.value.as_ref().is_some_and(|v| v.1))
            }
            UIA_POSITION_IN_SET_PROPERTY_ID if node.role != role::SEPARATOR => {
                variant_i4(siblings.iter().position(|n| n.id == node.id).map(|p| p as i32 + 1).unwrap_or(0))
            }
            UIA_SIZE_OF_SET_PROPERTY_ID if node.role != role::SEPARATOR => variant_i4(siblings.len() as i32),
            other => match patterns::pattern_property(node, other) {
                Some(value) => value,
                None => return S_OK,
            },
        };
        S_OK
    })
}

pub(super) unsafe extern "system" fn simple_host(this: *mut c_void, out: *mut *mut c_void) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if out.is_null() {
            return E_POINTER;
        }
        *out = std::ptr::null_mut();
        let element = element_of!(this, simple);
        if element.id != 0 {
            return S_OK;
        }
        let Some(host) = element.host() else { return S_OK };
        match optional().UiaHostProviderFromHwnd {
            Some(from_hwnd) => from_hwnd(host.tree.hwnd(), out),
            None => S_OK,
        }
    })
}

// --- IRawElementProviderFragment ---

pub(super) const NAVIGATE_PARENT: i32 = 0;
pub(super) const NAVIGATE_NEXT: i32 = 1;
pub(super) const NAVIGATE_PREVIOUS: i32 = 2;
pub(super) const NAVIGATE_FIRST_CHILD: i32 = 3;
pub(super) const NAVIGATE_LAST_CHILD: i32 = 4;

/// Navigation destination (pure function, tested separately).
pub(crate) fn navigate(nodes: &[Node], from: u32, direction: i32) -> Option<u32> {
    let children = |parent: u32| nodes.iter().filter(move |n| n.parent == parent).map(|n| n.id);
    match direction {
        NAVIGATE_PARENT if from != 0 => nodes.iter().find(|n| n.id == from).map(|n| n.parent),
        NAVIGATE_FIRST_CHILD => children(from).next(),
        NAVIGATE_LAST_CHILD => children(from).next_back(),
        NAVIGATE_NEXT | NAVIGATE_PREVIOUS if from != 0 => {
            let parent = nodes.iter().find(|n| n.id == from)?.parent;
            let siblings: Vec<u32> = children(parent).collect();
            let index = siblings.iter().position(|&id| id == from)?;
            if direction == NAVIGATE_NEXT {
                siblings.get(index + 1).copied()
            } else {
                index.checked_sub(1).map(|i| siblings[i])
            }
        }
        _ => None,
    }
}

pub(super) unsafe extern "system" fn fragment_navigate(
    this: *mut c_void,
    direction: i32,
    out: *mut *mut c_void,
) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if out.is_null() {
            return E_POINTER;
        }
        *out = std::ptr::null_mut();
        let element = element_of!(this, fragment);
        let Some(host) = element.host() else { return UIA_E_ELEMENTNOTAVAILABLE };
        let nodes = host.tree.nodes();
        match navigate(&nodes, element.id, direction) {
            Some(0) => {
                if let Some(root) = host.root_provider() {
                    let raw = root.as_ptr() as *mut Element;
                    add_ref(&*raw);
                    *out = (*raw).pointer_to(&(*raw).fragment);
                }
            }
            Some(id) => *out = element.sibling_fragment(id),
            None => {}
        }
        S_OK
    })
}

pub(super) unsafe extern "system" fn fragment_runtime_id(this: *mut c_void, out: *mut *mut c_void) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if out.is_null() {
            return E_POINTER;
        }
        *out = std::ptr::null_mut();
        let element = element_of!(this, fragment);
        if element.id == 0 {
            // A raiz hospedada num HWND devolve nulo: o UIA usa o identificador da janela.
            return S_OK;
        }
        let array = SafeArrayCreateVector(VT_I4, 0, 2);
        if array.is_null() {
            return E_OUTOFMEMORY;
        }
        for (index, value) in [UIA_APPEND_RUNTIME_ID, element.id as i32].iter().enumerate() {
            let index = index as i32;
            SafeArrayPutElement(array, &index, value as *const i32 as *const c_void);
        }
        *out = array;
        S_OK
    })
}

pub(super) unsafe extern "system" fn fragment_bounds(this: *mut c_void, out: *mut UiaRect) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if out.is_null() {
            return E_POINTER;
        }
        *out = UiaRect::default();
        let element = element_of!(this, fragment);
        if element.id == 0 {
            return S_OK;
        }
        let Some(node) = element.node() else { return UIA_E_ELEMENTNOTAVAILABLE };
        if !node.offscreen() {
            let r = node.bounds;
            *out = UiaRect {
                left: r.left as f64,
                top: r.top as f64,
                width: (r.right - r.left) as f64,
                height: (r.bottom - r.top) as f64,
            };
        }
        S_OK
    })
}

pub(super) unsafe extern "system" fn fragment_embedded(_this: *mut c_void, out: *mut *mut c_void) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if !out.is_null() {
            *out = std::ptr::null_mut();
        }
        S_OK
    })
}

pub(super) unsafe extern "system" fn fragment_set_focus(this: *mut c_void) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        let element = element_of!(this, fragment);
        if element.id == 0 {
            return S_OK;
        }
        act(element, Action::Focus)
    })
}

pub(super) unsafe extern "system" fn fragment_root(this: *mut c_void, out: *mut *mut c_void) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if out.is_null() {
            return E_POINTER;
        }
        *out = std::ptr::null_mut();
        let element = element_of!(this, fragment);
        let Some(host) = element.host() else { return UIA_E_ELEMENTNOTAVAILABLE };
        if let Some(root) = host.root_provider() {
            let raw = root.as_ptr() as *mut Element;
            add_ref(&*raw);
            *out = (*raw).pointer_to(&(*raw).root);
        }
        S_OK
    })
}

// --- IRawElementProviderFragmentRoot ---

/// Innermost element under the point (screen pixels).
pub(crate) fn hit(nodes: &[Node], x: i32, y: i32) -> Option<u32> {
    let inside = |n: &Node| {
        !n.offscreen() && x >= n.bounds.left && x < n.bounds.right && y >= n.bounds.top && y < n.bounds.bottom
    };
    // Filhos vêm depois dos pais: o último que contém o ponto é o mais interno.
    nodes.iter().rfind(|n| inside(n)).map(|n| n.id)
}

pub(super) unsafe extern "system" fn root_from_point(
    this: *mut c_void,
    x: f64,
    y: f64,
    out: *mut *mut c_void,
) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if out.is_null() {
            return E_POINTER;
        }
        *out = std::ptr::null_mut();
        let element = element_of!(this, root);
        let Some(host) = element.host() else { return UIA_E_ELEMENTNOTAVAILABLE };
        if let Some(id) = hit(&host.tree.nodes(), x as i32, y as i32) {
            *out = element.sibling_fragment(id);
        }
        S_OK
    })
}

pub(super) unsafe extern "system" fn root_focus(this: *mut c_void, out: *mut *mut c_void) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if out.is_null() {
            return E_POINTER;
        }
        *out = std::ptr::null_mut();
        let element = element_of!(this, root);
        let Some(host) = element.host() else { return UIA_E_ELEMENTNOTAVAILABLE };
        if let Some(node) = host.tree.nodes().into_iter().find(|n| n.focused) {
            *out = element.sibling_fragment(node.id);
        }
        S_OK
    })
}

// --- Padrões ---

pub(super) unsafe extern "system" fn invoke_invoke(this: *mut c_void) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        let element = element_of!(this, invoke);
        let result = act(element, Action::Invoke);
        if result == S_OK {
            if let Some(host) = element.host() {
                host.raise(element.id, UIA_INVOKE_INVOKED_EVENT_ID);
            }
        }
        result
    })
}

pub(super) unsafe extern "system" fn toggle_toggle(this: *mut c_void) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || act(element_of!(this, toggle), Action::Toggle))
}

pub(super) unsafe extern "system" fn toggle_state(this: *mut c_void, out: *mut i32) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if out.is_null() {
            return E_POINTER;
        }
        let Some(node) = element_of!(this, toggle).node() else { return UIA_E_ELEMENTNOTAVAILABLE };
        *out = i32::from(node.toggle == Some(true));
        S_OK
    })
}

pub(super) unsafe extern "system" fn expand_expand(this: *mut c_void) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || act(element_of!(this, expand), Action::Expand))
}

pub(super) unsafe extern "system" fn expand_collapse(this: *mut c_void) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || act(element_of!(this, expand), Action::Collapse))
}

pub(super) unsafe extern "system" fn expand_state(this: *mut c_void, out: *mut i32) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if out.is_null() {
            return E_POINTER;
        }
        let Some(node) = element_of!(this, expand).node() else { return UIA_E_ELEMENTNOTAVAILABLE };
        // 0 = recolhido, 1 = expandido.
        *out = i32::from(node.expanded == Some(true));
        S_OK
    })
}

pub(super) unsafe extern "system" fn value_set(this: *mut c_void, text: *const u16) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if text.is_null() {
            return E_POINTER;
        }
        let element = element_of!(this, value);
        if element.node().is_some_and(|n| n.value.as_ref().is_some_and(|v| v.1)) {
            return UIA_E_ELEMENTNOTENABLED;
        }
        act(element, Action::SetValue(from_wide_ptr_lossy(text)))
    })
}

pub(super) unsafe extern "system" fn value_get(this: *mut c_void, out: *mut *mut u16) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if out.is_null() {
            return E_POINTER;
        }
        *out = std::ptr::null_mut();
        let Some(node) = element_of!(this, value).node() else { return UIA_E_ELEMENTNOTAVAILABLE };
        *out = bstr(&node.value.map(|v| v.0).unwrap_or_default());
        S_OK
    })
}

pub(super) unsafe extern "system" fn value_read_only(this: *mut c_void, out: *mut BOOL) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if out.is_null() {
            return E_POINTER;
        }
        let Some(node) = element_of!(this, value).node() else { return UIA_E_ELEMENTNOTAVAILABLE };
        *out = BOOL::from(node.value.is_some_and(|v| v.1));
        S_OK
    })
}
