//! Accessibility: accessible names and descriptions, UI Automation providers for custom controls, screen reader
//! announcements, tab order and focus indicators.
//!
//! Native controls (buttons, lists, fields, etc.) already expose roles, states and values to Microsoft UI Automation;
//! this module fills the gaps:
//!
//! * [`Widget::set_accessible_name`] / [`Widget::set_accessible_description`] annotate native controls using the
//!   official *Dynamic Annotation* API, `IAccPropServices`;
//! * [`set_custom_role`] publishes a simple UIA provider (role and name) for application-drawn controls
//!   ([`Panel`](crate::controls::Panel) with `on_paint`); [`set_accessible_model`] publishes a complete model with
//!   internal items and the Invoke, Toggle, ExpandCollapse, Value, RangeValue, Selection, SelectionItem, Scroll, Grid,
//!   GridItem and Text patterns;
//! * [`announce`] sends a notification to screen readers;
//! * [`set_tab_order`] and [`show_focus_indicators`] manage keyboard navigation.

mod model;
pub(crate) mod tree;

pub use model::{
    raise_accessible_event, set_accessible_model, AccessibleAction, AccessibleEvent, AccessibleItem, AccessibleModel,
    AccessibleRange, AccessibleScroll, LiveSetting, ScrollAmount,
};

/// Direct access to virtual UIA elements for tests.
#[doc(hidden)]
pub mod uia_debug {
    pub use super::tree::debug::*;
}

use crate::core::native;
use crate::core::widget::{Widget, WidgetCore};
use crate::core::{GuiError, GuiResult, Window};
use crate::ffi::dynamic::optional;
use crate::ffi::ole32::*;
use crate::ffi::uiautomation::*;
use crate::ffi::user32::*;
use crate::ffi::*;
use crate::utilities::to_wide;
use std::ffi::c_void;
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

fn annotate(core: &Rc<WidgetCore>, property: &GUID, text: &str) -> GuiResult<()> {
    let hwnd = core.require_alive()?;
    let mut raw: *mut c_void = std::ptr::null_mut();
    // SAFETY: CLSID/IID válidos e ponteiro de saída; a referência devolvida passa ao ComPtr.
    let services = unsafe {
        let hr = CoCreateInstance(
            &CLSID_AccPropServices,
            std::ptr::null_mut(),
            CLSCTX_INPROC_SERVER,
            &IID_IAccPropServices,
            &mut raw,
        );
        GuiError::check_hresult("CoCreateInstance(AccPropServices)", hr)?;
        ComPtr::from_raw(raw).ok_or(GuiError::Unsupported("IAccPropServices"))?
    };
    let wide = to_wide(text);
    const CHILDID_SELF: u32 = 0;
    // SAFETY: o índice 7 da vtable de IAccPropServices é
    // SetHwndPropStr(HWND, DWORD idObject, DWORD idChild, MSAAPROPID, LPCWSTR); em x64
    // um GUID por valor é passado por ponteiro, conforme a ABI do Windows.
    let hr = unsafe {
        let set: unsafe extern "system" fn(*mut c_void, HWND, u32, u32, GUID, *const u16) -> HRESULT =
            services.method(7);
        set(services.as_ptr(), hwnd, OBJID_CLIENT as u32, CHILDID_SELF, *property, wide.as_ptr())
    };
    GuiError::check_hresult("IAccPropServices::SetHwndPropStr", hr)
}

pub(crate) fn set_name(core: &Rc<WidgetCore>, name: &str) -> GuiResult<()> {
    if let Some(tree) = core.accessible_tree.borrow().as_ref() {
        *tree.name.borrow_mut() = Some(name.to_string());
        return Ok(());
    }
    if let Some(provider) = custom_provider(core) {
        provider.update(|info| info.name = name.to_string());
        return Ok(());
    }
    annotate(core, &PROPID_ACC_NAME, name)
}

pub(crate) fn set_description(core: &Rc<WidgetCore>, description: &str) -> GuiResult<()> {
    if let Some(tree) = core.accessible_tree.borrow().as_ref() {
        *tree.description.borrow_mut() = Some(description.to_string());
        return Ok(());
    }
    if let Some(provider) = custom_provider(core) {
        provider.update(|info| info.description = description.to_string());
        return Ok(());
    }
    annotate(core, &PROPID_ACC_DESCRIPTION, description)?;
    annotate(core, &PROPID_ACC_HELP, description)
}

/// Role of a custom control in the accessibility tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum AccessibleRole {
    /// Button.
    Button,
    /// Check box.
    CheckBox,
    /// Image.
    Image,
    /// Static text.
    Text,
    /// Progress bar.
    ProgressBar,
    /// Slider.
    Slider,
    /// List.
    List,
    /// Group.
    Group,
    /// Generic panel.
    Pane,
    /// Custom control without a standard equivalent.
    Custom,
    /// Combo box.
    ComboBox,
    /// Edit field.
    Edit,
    /// Link.
    Hyperlink,
    /// List item.
    ListItem,
    /// Menu.
    Menu,
    /// Menu item.
    MenuItem,
    /// Radio button.
    RadioButton,
    /// Scroll bar.
    ScrollBar,
    /// Numeric field with arrow buttons.
    Spinner,
    /// Status bar.
    StatusBar,
    /// Tab control.
    Tab,
    /// Tab.
    TabItem,
    /// Tool bar.
    ToolBar,
    /// Tooltip.
    ToolTip,
    /// Tree.
    Tree,
    /// Tree item.
    TreeItem,
    /// Data grid.
    DataGrid,
    /// Data grid row.
    DataItem,
    /// Document (text).
    Document,
    /// Split button.
    SplitButton,
    /// Window.
    Window,
    /// Header.
    Header,
    /// Header item.
    HeaderItem,
    /// Table.
    Table,
    /// Title bar.
    TitleBar,
    /// Separator.
    Separator,
}

impl AccessibleRole {
    fn control_type(self) -> i32 {
        match self {
            AccessibleRole::Button => 50000,
            AccessibleRole::CheckBox => 50002,
            AccessibleRole::Image => 50006,
            AccessibleRole::List => 50008,
            AccessibleRole::ProgressBar => 50012,
            AccessibleRole::Slider => 50015,
            AccessibleRole::Text => 50020,
            AccessibleRole::Custom => 50025,
            AccessibleRole::Group => 50026,
            AccessibleRole::Pane => 50033,
            AccessibleRole::ComboBox => 50003,
            AccessibleRole::Edit => 50004,
            AccessibleRole::Hyperlink => 50005,
            AccessibleRole::ListItem => 50007,
            AccessibleRole::Menu => 50009,
            AccessibleRole::MenuItem => 50011,
            AccessibleRole::RadioButton => 50013,
            AccessibleRole::ScrollBar => 50014,
            AccessibleRole::Spinner => 50016,
            AccessibleRole::StatusBar => 50017,
            AccessibleRole::Tab => 50018,
            AccessibleRole::TabItem => 50019,
            AccessibleRole::ToolBar => 50021,
            AccessibleRole::ToolTip => 50022,
            AccessibleRole::Tree => 50023,
            AccessibleRole::TreeItem => 50024,
            AccessibleRole::DataGrid => 50028,
            AccessibleRole::DataItem => 50029,
            AccessibleRole::Document => 50030,
            AccessibleRole::SplitButton => 50031,
            AccessibleRole::Window => 50032,
            AccessibleRole::Header => 50034,
            AccessibleRole::HeaderItem => 50035,
            AccessibleRole::Table => 50036,
            AccessibleRole::TitleBar => 50037,
            AccessibleRole::Separator => 50038,
        }
    }
}

struct Info {
    role: AccessibleRole,
    name: String,
    description: String,
}

#[repr(C)]
struct ProviderVtbl {
    query_interface: unsafe extern "system" fn(*mut Provider, *const GUID, *mut *mut c_void) -> HRESULT,
    add_ref: unsafe extern "system" fn(*mut Provider) -> u32,
    release: unsafe extern "system" fn(*mut Provider) -> u32,
    get_provider_options: unsafe extern "system" fn(*mut Provider, *mut i32) -> HRESULT,
    get_pattern_provider: unsafe extern "system" fn(*mut Provider, i32, *mut *mut c_void) -> HRESULT,
    get_property_value: unsafe extern "system" fn(*mut Provider, i32, *mut VARIANT) -> HRESULT,
    get_host_provider: unsafe extern "system" fn(*mut Provider, *mut *mut c_void) -> HRESULT,
}

/// COM object implementing `IRawElementProviderSimple`.
///
/// Heap allocated and managed by COM reference counting; all fields are thread-safe because UI Automation may query the
/// provider from other threads.
#[repr(C)]
struct Provider {
    vtable: *const ProviderVtbl,
    references: AtomicU32,
    hwnd: isize,
    info: Mutex<Info>,
}

static PROVIDER_VTBL: ProviderVtbl = ProviderVtbl {
    query_interface: provider_query_interface,
    add_ref: provider_add_ref,
    release: provider_release,
    get_provider_options: provider_options,
    get_pattern_provider: provider_pattern,
    get_property_value: provider_property,
    get_host_provider: provider_host,
};

impl Provider {
    fn update(&self, change: impl FnOnce(&mut Info)) {
        let mut info = self.info.lock().unwrap_or_else(|p| p.into_inner());
        change(&mut info);
    }
}

unsafe extern "system" fn provider_query_interface(
    this: *mut Provider,
    iid: *const GUID,
    out: *mut *mut c_void,
) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if out.is_null() || iid.is_null() {
            return E_POINTER;
        }
        if *iid == IID_IUnknown || *iid == IID_IRawElementProviderSimple {
            provider_add_ref(this);
            *out = this as *mut c_void;
            S_OK
        } else {
            *out = std::ptr::null_mut();
            E_NOINTERFACE
        }
    })
}

unsafe extern "system" fn provider_add_ref(this: *mut Provider) -> u32 {
    crate::core::ffi_guard::guard(0, move || (*this).references.fetch_add(1, Ordering::SeqCst) + 1)
}

unsafe extern "system" fn provider_release(this: *mut Provider) -> u32 {
    crate::core::ffi_guard::guard(0, move || {
        let remaining = (*this).references.fetch_sub(1, Ordering::SeqCst) - 1;
        if remaining == 0 {
            // A última referência devolve a alocação feita em `set_custom_role`.
            drop(Box::from_raw(this));
        }
        remaining
    })
}

unsafe extern "system" fn provider_options(_this: *mut Provider, out: *mut i32) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if out.is_null() {
            return E_POINTER;
        }
        const USE_COM_THREADING: i32 = 0x20;
        *out = PROVIDER_OPTIONS_SERVER_SIDE | USE_COM_THREADING;
        S_OK
    })
}

unsafe extern "system" fn provider_pattern(_this: *mut Provider, _pattern: i32, out: *mut *mut c_void) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if out.is_null() {
            return E_POINTER;
        }
        *out = std::ptr::null_mut();
        S_OK
    })
}

unsafe extern "system" fn provider_property(this: *mut Provider, property: i32, out: *mut VARIANT) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if out.is_null() {
            return E_POINTER;
        }
        let mut value = VARIANT { vt: VT_EMPTY, reserved: [0; 3], data: [0; 2] };
        let string = |text: &str| -> VARIANT {
            let wide = to_wide(text);
            VARIANT { vt: VT_BSTR, reserved: [0; 3], data: [SysAllocString(wide.as_ptr()) as usize, 0] }
        };
        let boolean = |flag: bool| VARIANT { vt: VT_BOOL, reserved: [0; 3], data: [if flag { 0xFFFF } else { 0 }, 0] };
        {
            let info = (*this).info.lock().unwrap_or_else(|p| p.into_inner());
            match property {
                UIA_CONTROL_TYPE_PROPERTY_ID => {
                    value =
                        VARIANT { vt: VT_I4, reserved: [0; 3], data: [info.role.control_type() as u32 as usize, 0] };
                }
                UIA_NAME_PROPERTY_ID => value = string(&info.name),
                UIA_HELP_TEXT_PROPERTY_ID | UIA_FULL_DESCRIPTION_PROPERTY_ID if !info.description.is_empty() => {
                    value = string(&info.description);
                }
                UIA_IS_CONTROL_ELEMENT_PROPERTY_ID | UIA_IS_CONTENT_ELEMENT_PROPERTY_ID => value = boolean(true),
                UIA_IS_KEYBOARD_FOCUSABLE_PROPERTY_ID => {
                    value = boolean(GetWindowLongPtrW((*this).hwnd as HWND, GWL_STYLE) as u32 & WS_TABSTOP != 0);
                }
                _ => {}
            }
        }
        *out = value;
        S_OK
    })
}

unsafe extern "system" fn provider_host(this: *mut Provider, out: *mut *mut c_void) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if out.is_null() {
            return E_POINTER;
        }
        match optional().UiaHostProviderFromHwnd {
            Some(host) => host((*this).hwnd as HWND, out),
            None => {
                *out = std::ptr::null_mut();
                S_OK
            }
        }
    })
}

fn custom_provider(core: &Rc<WidgetCore>) -> Option<&Provider> {
    let pointer = core.accessible.borrow().as_ref().map(|p| p.as_ptr() as *const Provider)?;
    // SAFETY: o ponteiro guardado em `core.accessible` sempre vem de `set_custom_role`
    // e a referência COM mantida pelo núcleo o mantém vivo enquanto `core` existir.
    Some(unsafe { &*pointer })
}

/// Publishes an application-drawn custom control to UI Automation with the given role and name.
///
/// Without this, a panel with `on_paint` appears to screen readers as an anonymous panel. Update its name and
/// description later with [`Widget::set_accessible_name`] and [`Widget::set_accessible_description`].
pub fn set_custom_role(widget: &impl Widget, role: AccessibleRole, name: &str) -> GuiResult<()> {
    let core = widget.core();
    let hwnd = core.require_alive()?;
    if optional().UiaReturnRawElementProvider.is_none() {
        return Err(GuiError::Unsupported("UI Automation (uiautomationcore.dll)"));
    }
    let provider = Box::into_raw(Box::new(Provider {
        vtable: &PROVIDER_VTBL,
        references: AtomicU32::new(1),
        hwnd: hwnd as isize,
        info: Mutex::new(Info { role, name: name.to_string(), description: String::new() }),
    }));
    // SAFETY: `provider` é um objeto COM válido com uma referência, que passa ao ComPtr.
    let pointer = unsafe { ComPtr::from_raw(provider as *mut c_void) };
    *core.accessible.borrow_mut() = pointer;
    Ok(())
}

/// Asks screen readers to announce `message` (for example, "File saved").
///
/// Uses `UiaRaiseNotificationEvent` (Windows 10 1709+). Returns [`GuiError::Unsupported`] when the API is unavailable.
pub fn announce(widget: &impl Widget, message: &str) -> GuiResult<()> {
    announce_with(widget, message, false)
}

/// Like [`announce`]; with `important`, the reader interrupts its current speech
/// (`NotificationProcessing_ImportantMostRecent`), for example for errors.
pub fn announce_with(widget: &impl Widget, message: &str, important: bool) -> GuiResult<()> {
    let hwnd = widget.core().require_alive()?;
    let apis = optional();
    let (Some(raise), Some(host)) = (apis.UiaRaiseNotificationEvent, apis.UiaHostProviderFromHwnd) else {
        return Err(GuiError::Unsupported("UiaRaiseNotificationEvent requires Windows 10 1709+"));
    };
    let text = to_wide(message);
    let activity = to_wide("arqrustui.announce");
    // SAFETY: o provedor hospedeiro é obtido, usado e liberado (ComPtr) neste bloco; as
    // BSTRs são alocadas e liberadas aqui. 4 = NotificationKind_Other; 2 = MostRecent,
    // 0 = ImportantAll... (1 = ImportantMostRecent).
    unsafe {
        let mut raw: *mut c_void = std::ptr::null_mut();
        GuiError::check_hresult("UiaHostProviderFromHwnd", host(hwnd, &mut raw))?;
        let provider = ComPtr::from_raw(raw).ok_or(GuiError::Unsupported("window UIA provider"))?;
        let display = SysAllocString(text.as_ptr());
        let id = SysAllocString(activity.as_ptr());
        let hr = raise(provider.as_ptr(), 4, if important { 1 } else { 2 }, display, id);
        SysFreeString(display);
        SysFreeString(id);
        GuiError::check_hresult("UiaRaiseNotificationEvent", hr)
    }
}

/// Sets tab order: controls receive focus with `Tab` in slice order.
///
/// All controls must be siblings (same parent). Creation order is the default.
pub fn set_tab_order(widgets: &[&dyn Widget]) {
    let mut previous: HWND = HWND_TOP;
    for (index, widget) in widgets.iter().enumerate() {
        let hwnd = widget.core().hwnd();
        if hwnd.is_null() {
            continue;
        }
        // A ordem de tabulação segue a ordem Z: cada controle é colocado logo após o anterior.
        // SAFETY: altera apenas a ordem Z de controles vivos desta thread.
        unsafe {
            if index == 0 {
                SetWindowPos(hwnd, HWND_TOP, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            } else {
                SetWindowPos(hwnd, previous, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            }
        }
        previous = hwnd;
    }
}

/// Shows the window's focus rectangles and mnemonic underlines even before the user uses the keyboard.
pub fn show_focus_indicators(window: &Window) {
    let flags = ((UISF_HIDEFOCUS | UISF_HIDEACCEL) as usize) << 16 | UIS_CLEAR as usize;
    native::send(window.core().hwnd(), WM_CHANGEUISTATE, flags, 0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_map_to_distinct_control_types() {
        let roles = [
            AccessibleRole::Button,
            AccessibleRole::CheckBox,
            AccessibleRole::Image,
            AccessibleRole::Text,
            AccessibleRole::ProgressBar,
            AccessibleRole::Slider,
            AccessibleRole::List,
            AccessibleRole::Group,
            AccessibleRole::Pane,
            AccessibleRole::Custom,
            AccessibleRole::ComboBox,
            AccessibleRole::Edit,
            AccessibleRole::Hyperlink,
            AccessibleRole::ListItem,
            AccessibleRole::Menu,
            AccessibleRole::MenuItem,
            AccessibleRole::RadioButton,
            AccessibleRole::ScrollBar,
            AccessibleRole::Spinner,
            AccessibleRole::StatusBar,
            AccessibleRole::Tab,
            AccessibleRole::TabItem,
            AccessibleRole::ToolBar,
            AccessibleRole::ToolTip,
            AccessibleRole::Tree,
            AccessibleRole::TreeItem,
            AccessibleRole::DataGrid,
            AccessibleRole::DataItem,
            AccessibleRole::Document,
            AccessibleRole::SplitButton,
            AccessibleRole::Window,
            AccessibleRole::Header,
            AccessibleRole::HeaderItem,
            AccessibleRole::Table,
            AccessibleRole::TitleBar,
            AccessibleRole::Separator,
        ];
        let mut ids: Vec<i32> = roles.iter().map(|r| r.control_type()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), roles.len());
        assert!(ids.iter().all(|id| (50000..=50040).contains(id)));
    }

    #[test]
    fn provider_com_lifecycle() {
        let provider = Box::into_raw(Box::new(Provider {
            vtable: &PROVIDER_VTBL,
            references: AtomicU32::new(1),
            hwnd: 0,
            info: Mutex::new(Info { role: AccessibleRole::Button, name: "Enviar".into(), description: String::new() }),
        }));
        // SAFETY: objeto recém-criado; as chamadas seguem o protocolo COM e a última
        // Release libera a alocação.
        unsafe {
            let mut out: *mut c_void = std::ptr::null_mut();
            assert_eq!(provider_query_interface(provider, &IID_IRawElementProviderSimple, &mut out), S_OK);
            assert_eq!(out, provider as *mut c_void);
            let unknown_iid = GUID::new(1, 2, 3, [4; 8]);
            assert_eq!(provider_query_interface(provider, &unknown_iid, &mut out), E_NOINTERFACE);
            let mut value = VARIANT { vt: 0, reserved: [0; 3], data: [0; 2] };
            assert_eq!(provider_property(provider, UIA_CONTROL_TYPE_PROPERTY_ID, &mut value), S_OK);
            assert_eq!((value.vt, value.data[0]), (VT_I4, 50000));
            assert_eq!(provider_property(provider, UIA_NAME_PROPERTY_ID, &mut value), S_OK);
            assert_eq!(value.vt, VT_BSTR);
            assert_eq!(crate::utilities::strings::from_wide_ptr(value.data[0] as *const u16), "Enviar");
            SysFreeString(value.data[0] as *mut u16);
            assert_eq!(provider_release(provider), 1);
            assert_eq!(provider_release(provider), 0);
        }
    }
}
