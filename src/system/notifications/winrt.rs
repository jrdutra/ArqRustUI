//! Notification WinRT/COM infrastructure: progress data, Action Center history, HSTRING, activation factories, app
//! registry ID and activation/dismissal delegates.

use super::*;

/// Progress NotificationData keys/values.
pub(super) fn progress_values(progress: &Progress, with_title: bool) -> Vec<(&'static str, String)> {
    let value = match progress.value {
        Some(value) => format!("{value:.4}").trim_end_matches('0').trim_end_matches('.').to_string(),
        None => "indeterminate".to_string(),
    };
    let value = if value.is_empty() { "0".to_string() } else { value };
    let mut values = vec![
        ("progressValue", value),
        ("progressValueString", progress.value_text.clone()),
        ("progressStatus", progress.status.clone()),
    ];
    if with_title {
        values.push(("progressTitle", progress.title.clone()));
    }
    values
}

/// NotificationData with given values/sequence number.
///
/// # Safety
/// Requires available WinRT (`apis`).
pub(super) unsafe fn notification_data(
    apis: &'static WinRtApis,
    values: &[(&str, String)],
    sequence: u32,
) -> GuiResult<ComPtr> {
    let class = HString::new(apis, "Windows.UI.Notifications.NotificationData")?;
    let mut raw: *mut c_void = std::ptr::null_mut();
    check("RoActivateInstance(NotificationData)", (apis.RoActivateInstance)(class.0, &mut raw))?;
    let object = ComPtr::from_raw(raw).ok_or(GuiError::Unsupported("NotificationData"))?;
    let data = query(&object, &IID_INotificationData)?;
    let get_values: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> HRESULT = data.method(6);
    let mut raw: *mut c_void = std::ptr::null_mut();
    check("NotificationData::get_Values", get_values(data.as_ptr(), &mut raw))?;
    let map = ComPtr::from_raw(raw).ok_or(GuiError::Unsupported("IMap<String, String>"))?;
    let map = query(&map, &IID_IMap_String_String)?;
    // IMap: Lookup 6, Size 7, HasKey 8, GetView 9, Insert 10.
    let insert: unsafe extern "system" fn(*mut c_void, HSTRING, HSTRING, *mut u8) -> HRESULT = map.method(10);
    for (key, value) in values {
        let key = HString::new(apis, key)?;
        let value = HString::new(apis, value)?;
        let mut replaced = 0u8;
        check("IMap::Insert", insert(map.as_ptr(), key.0, value.0, &mut replaced))?;
    }
    let put_sequence: unsafe extern "system" fn(*mut c_void, u32) -> HRESULT = data.method(8);
    put_sequence(data.as_ptr(), sequence);
    Ok(data)
}

/// Action Center history and current AppUserModelID.
pub(super) fn with_history(f: impl FnOnce(&'static WinRtApis, &ComPtr, &HString) -> GuiResult<()>) -> GuiResult<()> {
    let apis = winrt().ok_or(GuiError::Unsupported("system notifications require Windows 8 or later"))?;
    let id = ensure_app_id()?;
    // SAFETY: chamadas WinRT pelos índices documentados; referências em ComPtr.
    unsafe {
        let class = HString::new(apis, "Windows.UI.Notifications.ToastNotificationManager")?;
        let statics = activation_factory(apis, &class, &IID_IToastNotificationManagerStatics2)?;
        let get_history: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> HRESULT = statics.method(6);
        let mut raw: *mut c_void = std::ptr::null_mut();
        check("ToastNotificationManager::get_History", get_history(statics.as_ptr(), &mut raw))?;
        let history = ComPtr::from_raw(raw).ok_or(GuiError::Unsupported("ToastNotificationHistory"))?;
        let history = query(&history, &IID_IToastNotificationHistory)?;
        let app = HString::new(apis, &id)?;
        f(apis, &history, &app)
    }
}

/// IToastActivatedEventArgs2 UserInput values read on Windows thread by known IDs.
///
/// # Safety
/// `args` must be live Activated event arguments.
pub(super) unsafe fn read_user_input(apis: &'static WinRtApis, args: &ComPtr, ids: &[String]) -> Vec<(String, String)> {
    let mut values = Vec::new();
    let Ok(args2) = query(args, &IID_IToastActivatedEventArgs2) else { return values };
    let get: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> HRESULT = args2.method(6);
    let mut raw: *mut c_void = std::ptr::null_mut();
    if get(args2.as_ptr(), &mut raw) < 0 {
        return values;
    }
    let Some(set) = ComPtr::from_raw(raw) else { return values };
    let Ok(map) = query(&set, &IID_IMap_String_Object) else { return values };
    let lookup: unsafe extern "system" fn(*mut c_void, HSTRING, *mut *mut c_void) -> HRESULT = map.method(6);
    for id in ids {
        let Ok(key) = HString::new(apis, id) else { continue };
        let mut raw: *mut c_void = std::ptr::null_mut();
        if lookup(map.as_ptr(), key.0, &mut raw) < 0 {
            continue;
        }
        let Some(boxed) = ComPtr::from_raw(raw) else { continue };
        let Ok(value) = query(&boxed, &IID_IPropertyValue) else { continue };
        // IPropertyValue::GetString é o 14º método próprio (índice 19).
        let get_string: unsafe extern "system" fn(*mut c_void, *mut HSTRING) -> HRESULT = value.method(19);
        let mut text: HSTRING = std::ptr::null_mut();
        if get_string(value.as_ptr(), &mut text) >= 0 {
            values.push((id.clone(), hstring_text(apis, text)));
            (apis.WindowsDeleteString)(text);
        }
    }
    values
}

pub(super) fn check(context: &'static str, hr: HRESULT) -> GuiResult<()> {
    // Classe não registrada / interface ausente: o sistema não tem o recurso.
    const REGDB_E_CLASSNOTREG: HRESULT = 0x8004_0154_u32 as HRESULT;
    const E_NOTIMPL: HRESULT = 0x8000_4001_u32 as HRESULT;
    match hr {
        REGDB_E_CLASSNOTREG | E_NOTIMPL | E_NOINTERFACE => Err(GuiError::Unsupported(context)),
        _ => GuiError::check_hresult(context, hr),
    }
}

/// Automatically released HSTRING.
pub(super) struct HString(pub(super) HSTRING, &'static WinRtApis);

impl HString {
    pub(super) fn new(apis: &'static WinRtApis, text: &str) -> GuiResult<HString> {
        let wide: Vec<u16> = text.encode_utf16().collect();
        let mut handle: HSTRING = std::ptr::null_mut();
        // SAFETY: buffer válido com o comprimento informado; a cópia pertence à HSTRING.
        check("WindowsCreateString", unsafe {
            (apis.WindowsCreateString)(wide.as_ptr(), wide.len() as u32, &mut handle)
        })?;
        Ok(HString(handle, apis))
    }
}

impl Drop for HString {
    fn drop(&mut self) {
        // SAFETY: HSTRING criada por WindowsCreateString (nula também é aceita).
        unsafe { (self.1.WindowsDeleteString)(self.0) };
    }
}

/// Borrowed HSTRING text.
///
/// # Safety
/// `handle` must be valid HSTRING or null throughout the call.
pub(super) unsafe fn hstring_text(apis: &WinRtApis, handle: HSTRING) -> String {
    let mut len = 0u32;
    let buffer = (apis.WindowsGetStringRawBuffer)(handle, &mut len);
    if buffer.is_null() {
        return String::new();
    }
    String::from_utf16_lossy(std::slice::from_raw_parts(buffer, len as usize))
}

pub(super) fn activation_factory(apis: &WinRtApis, class: &HString, iid: &GUID) -> GuiResult<ComPtr> {
    let mut raw: *mut c_void = std::ptr::null_mut();
    // SAFETY: nome de classe e IID válidos; a referência devolvida passa ao ComPtr.
    unsafe {
        check("RoGetActivationFactory", (apis.RoGetActivationFactory)(class.0, iid, &mut raw))?;
        ComPtr::from_raw(raw).ok_or(GuiError::Unsupported("WinRT factory"))
    }
}

/// QueryInterface with owned result.
///
/// # Safety
/// `object` must be a live COM interface.
pub(super) unsafe fn query(object: &ComPtr, iid: &GUID) -> GuiResult<ComPtr> {
    let qi: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT = object.method(0);
    let mut raw: *mut c_void = std::ptr::null_mut();
    check("QueryInterface", qi(object.as_ptr(), iid, &mut raw))?;
    ComPtr::from_raw(raw).ok_or(GuiError::Unsupported("WinRT interface"))
}

// --- Identificador do aplicativo ---

pub(super) fn default_app_id() -> String {
    let stem = std::env::current_exe()
        .ok()
        .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "App".to_string());
    let clean: String = stem.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-').collect();
    format!("ArqRustUI.{}", if clean.is_empty() { "App" } else { &clean })
}

pub(super) fn ensure_app_id() -> GuiResult<String> {
    let (id, registered) = {
        let mut guard = APP_ID.lock().unwrap_or_else(|p| p.into_inner());
        let identity = guard.get_or_insert_with(|| AppIdentity { id: default_app_id(), registered: false });
        (identity.id.clone(), identity.registered)
    };
    if !registered {
        let name = id.trim_start_matches("ArqRustUI.").to_string();
        let icon = std::env::current_exe().ok();
        register_app_id(&id, &name, icon.as_deref())?;
        if let Some(identity) = APP_ID.lock().unwrap_or_else(|p| p.into_inner()).as_mut() {
            identity.registered = true;
        }
    }
    Ok(id)
}

pub(super) fn register_app_id(id: &str, display_name: &str, icon: Option<&Path>) -> GuiResult<()> {
    let subkey = to_wide(&format!("Software\\Classes\\AppUserModelId\\{id}"));
    let mut key: HKEY = std::ptr::null_mut();
    // SAFETY: strings terminadas em NUL; a chave aberta é fechada no fim.
    unsafe {
        let status = RegCreateKeyExW(
            HKEY_CURRENT_USER,
            subkey.as_ptr(),
            0,
            std::ptr::null(),
            0,
            KEY_WRITE,
            std::ptr::null(),
            &mut key,
            std::ptr::null_mut(),
        );
        if status != 0 {
            return Err(GuiError::from_win32("RegCreateKeyExW", status as u32));
        }
        let mut values = vec![("DisplayName", display_name.to_string())];
        if let Some(icon) = icon {
            values.push(("IconUri", icon.to_string_lossy().into_owned()));
        }
        for (name, value) in values {
            let name = to_wide(name);
            let data = to_wide(&value);
            RegSetValueExW(key, name.as_ptr(), 0, REG_SZ, data.as_ptr() as *const u8, (data.len() * 2) as u32);
        }
        RegCloseKey(key);
        let wide = to_wide(id);
        SetCurrentProcessExplicitAppUserModelID(wide.as_ptr());
    }
    Ok(())
}

// --- Delegados de eventos ---

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Activated,
    Dismissed,
    Failed,
}

#[repr(C)]
pub(super) struct HandlerVtbl {
    query_interface: unsafe extern "system" fn(*mut Handler, *const GUID, *mut *mut c_void) -> HRESULT,
    add_ref: unsafe extern "system" fn(*mut Handler) -> u32,
    release: unsafe extern "system" fn(*mut Handler) -> u32,
    invoke: unsafe extern "system" fn(*mut Handler, *mut c_void, *mut c_void) -> HRESULT,
}

/// `TypedEventHandler<ToastNotification, …>` called on any Windows thread; stores Send data, forwards event to UI.
#[repr(C)]
pub(super) struct Handler {
    vtable: *const HandlerVtbl,
    references: AtomicU32,
    kind: Kind,
    key: u64,
    dispatcher: UiDispatcher,
    /// UserInput fields read on click.
    inputs: Vec<String>,
}

static HANDLER_VTBL: HandlerVtbl = HandlerVtbl {
    query_interface: handler_qi,
    add_ref: handler_add_ref,
    release: handler_release,
    invoke: handler_invoke,
};

impl Handler {
    pub(super) fn create(kind: Kind, key: u64, dispatcher: UiDispatcher, inputs: Vec<String>) -> ComPtr {
        let raw = Box::into_raw(Box::new(Handler {
            vtable: &HANDLER_VTBL,
            references: AtomicU32::new(1),
            kind,
            key,
            dispatcher,
            inputs,
        }));
        // SAFETY: objeto recém-criado com uma referência.
        unsafe { ComPtr::from_raw(raw as *mut c_void) }.expect("a Box pointer is never null")
    }

    fn iid(&self) -> GUID {
        match self.kind {
            Kind::Activated => IID_ActivatedHandler,
            Kind::Dismissed => IID_DismissedHandler,
            Kind::Failed => IID_FailedHandler,
        }
    }
}

pub(super) unsafe extern "system" fn handler_qi(
    this: *mut Handler,
    iid: *const GUID,
    out: *mut *mut c_void,
) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if out.is_null() || iid.is_null() {
            return E_POINTER;
        }
        let iid = *iid;
        if iid == IID_IUnknown || iid == IID_IAgileObject || iid == (*this).iid() {
            handler_add_ref(this);
            *out = this as *mut c_void;
            S_OK
        } else {
            *out = std::ptr::null_mut();
            E_NOINTERFACE
        }
    })
}

pub(super) unsafe extern "system" fn handler_add_ref(this: *mut Handler) -> u32 {
    crate::core::ffi_guard::guard(0, move || (*this).references.fetch_add(1, Ordering::SeqCst) + 1)
}

pub(super) unsafe extern "system" fn handler_release(this: *mut Handler) -> u32 {
    crate::core::ffi_guard::guard(0, move || {
        let remaining = (*this).references.fetch_sub(1, Ordering::SeqCst) - 1;
        if remaining == 0 {
            drop(Box::from_raw(this));
        }
        remaining
    })
}

/// Event decoded from arguments on Windows thread.
pub(super) enum Fired {
    Activated(NotificationActivation),
    Dismissed(DismissReason),
    Failed,
}

pub(super) unsafe extern "system" fn handler_invoke(
    this: *mut Handler,
    _sender: *mut c_void,
    args: *mut c_void,
) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        let handler = &*this;
        let Some(apis) = winrt() else { return S_OK };
        let fired = match handler.kind {
            Kind::Failed => Fired::Failed,
            Kind::Activated => {
                let mut activation = NotificationActivation::default();
                if let Some(args) = borrowed(args) {
                    if let Ok(activated) = query(&args, &IID_IToastActivatedEventArgs) {
                        let get: unsafe extern "system" fn(*mut c_void, *mut HSTRING) -> HRESULT = activated.method(6);
                        let mut text: HSTRING = std::ptr::null_mut();
                        if get(activated.as_ptr(), &mut text) >= 0 {
                            activation.argument = hstring_text(apis, text);
                            (apis.WindowsDeleteString)(text);
                        }
                    }
                    if !handler.inputs.is_empty() {
                        activation.inputs = read_user_input(apis, &args, &handler.inputs);
                    }
                }
                Fired::Activated(activation)
            }
            Kind::Dismissed => {
                let mut reason = 0i32;
                if let Some(args) = borrowed(args) {
                    if let Ok(dismissed) = query(&args, &IID_IToastDismissedEventArgs) {
                        let get: unsafe extern "system" fn(*mut c_void, *mut i32) -> HRESULT = dismissed.method(6);
                        get(dismissed.as_ptr(), &mut reason);
                    }
                }
                Fired::Dismissed(match reason {
                    1 => DismissReason::ApplicationHidden,
                    2 => DismissReason::TimedOut,
                    _ => DismissReason::UserCanceled,
                })
            }
        };
        let key = handler.key;
        let _ = handler.dispatcher.post(move || deliver(key, fired));
        S_OK
    })
}

/// # Safety
/// `raw` must be null or a valid COM interface.
pub(super) unsafe fn borrowed(raw: *mut c_void) -> Option<ComPtr> {
    let pointer = ComPtr::from_raw(raw)?;
    let owned = pointer.clone();
    std::mem::forget(pointer);
    Some(owned)
}

/// On UI thread, calls callback and forgets notification.
pub(super) fn deliver(key: u64, fired: Fired) {
    let Some((activated, dismissed)) = HANDLERS.with(|h| h.borrow_mut().remove(&key)) else { return };
    match fired {
        Fired::Activated(activation) => {
            if let Some(f) = activated {
                f(&activation);
            }
        }
        Fired::Dismissed(reason) => {
            if let Some(f) = dismissed {
                f(reason);
            }
        }
        Fired::Failed => {}
    }
}
