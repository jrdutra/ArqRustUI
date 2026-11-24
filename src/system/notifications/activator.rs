//! COM notification activator: clicking reopens a closed program or delivers to the running instance.
//!
//! Windows finds `CustomActivator` under the AppUserModelID's HKCU key and launches CLSID `LocalServer32` (this
//! executable with `-ToastActivated`). Startup [`NotificationActivator::register`] receives the click.

use super::{ensure_app_id, NotificationActivation};
use crate::core::application as app;
use crate::core::{GuiError, GuiResult, UiDispatcher};
use crate::ffi::ole32::IID_IUnknown;
use crate::ffi::winrt::*;
use crate::ffi::*;
use crate::utilities::strings::from_wide_ptr;
use crate::utilities::to_wide;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

/// Command-line argument used by Windows activator launch.
pub(crate) const LAUNCH_SWITCH: &str = "-ToastActivated";

type ActivatedFn = Rc<dyn Fn(&NotificationActivation)>;

static NEXT_KEY: AtomicU64 = AtomicU64::new(1);

thread_local! {
    static CALLBACKS: RefCell<HashMap<u64, ActivatedFn>> = RefCell::new(HashMap::new());
}

/// Parses GUID `"{XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX}"` with optional braces.
pub(crate) fn parse_guid(text: &str) -> Option<GUID> {
    let hex: String = text.trim().trim_start_matches('{').trim_end_matches('}').chars().filter(|c| *c != '-').collect();
    if hex.len() != 32 || !hex.chars().all(|c| c.is_ascii_hexdigit()) || text.matches('-').count() != 4 {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok();
    let mut data4 = [0u8; 8];
    for (slot, index) in data4.iter_mut().zip(8..16) {
        *slot = byte(index)?;
    }
    Some(GUID::new(
        u32::from_str_radix(&hex[0..8], 16).ok()?,
        u16::from_str_radix(&hex[8..12], 16).ok()?,
        u16::from_str_radix(&hex[12..16], 16).ok()?,
        data4,
    ))
}

/// Uppercase registry GUID format `{XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX}`.
pub(crate) fn format_guid(guid: &GUID) -> String {
    let d = guid.data4;
    format!(
        "{{{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
        guid.data1, guid.data2, guid.data3, d[0], d[1], d[2], d[3], d[4], d[5], d[6], d[7]
    )
}

/// Registered COM activator; dropping stops click reception but keeps HKCU registration for relaunch.
pub struct NotificationActivator {
    cookie: u32,
    clsid: GUID,
    key: u64,
}

impl std::fmt::Debug for NotificationActivator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NotificationActivator").field("clsid", &format_guid(&self.clsid)).finish()
    }
}

impl NotificationActivator {
    /// Registers application CLSID (fixed GUID generated once, e.g. with uuidgen) and HKCU relaunch data. Call early in
    /// `main` after Application::new() and optional
    /// [`SystemNotification::set_app_id`](super::SystemNotification::set_app_id). `on_activated` runs on UI thread.
    pub fn register(
        clsid: &str,
        on_activated: impl Fn(&NotificationActivation) + 'static,
    ) -> GuiResult<NotificationActivator> {
        let guid = parse_guid(clsid).ok_or_else(|| GuiError::InvalidArgument(format!("invalid CLSID: {clsid}")))?;
        let dispatcher = UiDispatcher::new(app::require()?.shared.clone());
        let app_id = ensure_app_id()?;
        write_registry(&guid, &app_id)?;
        let key = NEXT_KEY.fetch_add(1, Ordering::Relaxed);
        CALLBACKS.with(|c| c.borrow_mut().insert(key, Rc::new(on_activated)));
        let factory = Factory::create(key, dispatcher);
        let mut cookie = 0u32;
        // SAFETY: fábrica COM viva (o COM guarda a própria referência); CLSID local.
        let hr = unsafe { CoRegisterClassObject(&guid, factory, CLSCTX_LOCAL_SERVER, REGCLS_MULTIPLEUSE, &mut cookie) };
        // A referência inicial da fábrica é solta aqui: a do COM a mantém viva.
        // SAFETY: objeto criado acima com uma referência.
        unsafe { factory_release(factory) };
        if hr < 0 {
            CALLBACKS.with(|c| c.borrow_mut().remove(&key));
            return Err(GuiError::HResult { context: "CoRegisterClassObject", hresult: hr });
        }
        Ok(NotificationActivator { cookie, clsid: guid, key })
    }

    /// Registered CLSID.
    pub fn clsid(&self) -> String {
        format_guid(&self.clsid)
    }

    /// Removes HKCU COM server/CustomActivator for CLSID, e.g. uninstall. No registered activator required.
    pub fn unregister_from_registry(clsid: &str) -> GuiResult<()> {
        let guid = parse_guid(clsid).ok_or_else(|| GuiError::InvalidArgument(format!("invalid CLSID: {clsid}")))?;
        let subkey = to_wide(&format!("Software\\Classes\\CLSID\\{}", format_guid(&guid)));
        // SAFETY: string terminada em NUL; chave inexistente devolve erro, ignorado.
        unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, subkey.as_ptr()) };
        Ok(())
    }

    /// Current AppUserModelID's registered CustomActivator for inspection.
    pub fn registered_activator() -> Option<String> {
        let app_id = super::SystemNotification::app_id();
        read_string(&format!("Software\\Classes\\AppUserModelId\\{app_id}"), "CustomActivator")
    }
}

impl Drop for NotificationActivator {
    fn drop(&mut self) {
        // SAFETY: cookie devolvido por CoRegisterClassObject.
        unsafe { CoRevokeClassObject(self.cookie) };
        let _ = CALLBACKS.try_with(|c| c.borrow_mut().remove(&self.key));
    }
}

fn set_string(subkey: &str, name: Option<&str>, value: &str) -> GuiResult<()> {
    let subkey = to_wide(subkey);
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
        let name = name.map(to_wide);
        let data = to_wide(value);
        let status = RegSetValueExW(
            key,
            name.as_ref().map_or(std::ptr::null(), |n| n.as_ptr()),
            0,
            REG_SZ,
            data.as_ptr() as *const u8,
            (data.len() * 2) as u32,
        );
        RegCloseKey(key);
        if status != 0 {
            return Err(GuiError::from_win32("RegSetValueExW", status as u32));
        }
    }
    Ok(())
}

fn read_string(subkey: &str, name: &str) -> Option<String> {
    let subkey = to_wide(subkey);
    let name = to_wide(name);
    let mut key: HKEY = std::ptr::null_mut();
    // SAFETY: strings terminadas em NUL; buffer local do tamanho informado; chave fechada.
    unsafe {
        if RegOpenKeyExW(HKEY_CURRENT_USER, subkey.as_ptr(), 0, KEY_READ, &mut key) != 0 {
            return None;
        }
        let mut buffer = [0u16; 512];
        let mut size = (buffer.len() * 2) as u32;
        let mut kind = 0u32;
        let status = RegQueryValueExW(
            key,
            name.as_ptr(),
            std::ptr::null_mut(),
            &mut kind,
            buffer.as_mut_ptr() as *mut u8,
            &mut size,
        );
        RegCloseKey(key);
        (status == 0 && kind == REG_SZ).then(|| {
            let len = (size as usize / 2).min(buffer.len());
            String::from_utf16_lossy(&buffer[..len]).trim_end_matches('\0').to_string()
        })
    }
}

/// CLSID LocalServer32 and AppUserModelID CustomActivator.
fn write_registry(clsid: &GUID, app_id: &str) -> GuiResult<()> {
    let exe = std::env::current_exe()
        .map_err(|e| GuiError::InvalidArgument(format!("executable path not available: {e}")))?;
    let clsid_text = format_guid(clsid);
    set_string(
        &format!("Software\\Classes\\CLSID\\{clsid_text}\\LocalServer32"),
        None,
        &format!("\"{}\" {LAUNCH_SWITCH}", exe.display()),
    )?;
    set_string(&format!("Software\\Classes\\AppUserModelId\\{app_id}"), Some("CustomActivator"), &clsid_text)
}

// ---------------------------------------------------------------------------
// Objetos COM: fábrica (IClassFactory) e ativador (INotificationActivationCallback)
// ---------------------------------------------------------------------------

#[repr(C)]
struct FactoryVtbl {
    qi: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    create_instance: unsafe extern "system" fn(*mut c_void, *mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
    lock_server: unsafe extern "system" fn(*mut c_void, BOOL) -> HRESULT,
}

/// Factory called on any COM thread; stores only Send data.
#[repr(C)]
struct Factory {
    vtable: *const FactoryVtbl,
    references: AtomicU32,
    key: u64,
    dispatcher: UiDispatcher,
}

static FACTORY_VTBL: FactoryVtbl = FactoryVtbl {
    qi: factory_qi,
    add_ref: factory_add_ref,
    release: factory_release,
    create_instance: factory_create,
    lock_server: factory_lock,
};

impl Factory {
    fn create(key: u64, dispatcher: UiDispatcher) -> *mut c_void {
        Box::into_raw(Box::new(Factory { vtable: &FACTORY_VTBL, references: AtomicU32::new(1), key, dispatcher }))
            as *mut c_void
    }
}

unsafe extern "system" fn factory_qi(this: *mut c_void, iid: *const GUID, out: *mut *mut c_void) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if out.is_null() || iid.is_null() {
            return E_POINTER;
        }
        if *iid == IID_IUnknown || *iid == IID_IClassFactory {
            factory_add_ref(this);
            *out = this;
            S_OK
        } else {
            *out = std::ptr::null_mut();
            E_NOINTERFACE
        }
    })
}

unsafe extern "system" fn factory_add_ref(this: *mut c_void) -> u32 {
    crate::core::ffi_guard::guard(0, move || (*(this as *mut Factory)).references.fetch_add(1, Ordering::SeqCst) + 1)
}

unsafe extern "system" fn factory_release(this: *mut c_void) -> u32 {
    crate::core::ffi_guard::guard(0, move || {
        let remaining = (*(this as *mut Factory)).references.fetch_sub(1, Ordering::SeqCst) - 1;
        if remaining == 0 {
            drop(Box::from_raw(this as *mut Factory));
        }
        remaining
    })
}

unsafe extern "system" fn factory_create(
    this: *mut c_void,
    outer: *mut c_void,
    iid: *const GUID,
    out: *mut *mut c_void,
) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        const CLASS_E_NOAGGREGATION: HRESULT = 0x8004_0110_u32 as HRESULT;
        if out.is_null() {
            return E_POINTER;
        }
        *out = std::ptr::null_mut();
        if !outer.is_null() {
            return CLASS_E_NOAGGREGATION;
        }
        let factory = &*(this as *mut Factory);
        let activator = Box::into_raw(Box::new(Activator {
            vtable: &ACTIVATOR_VTBL,
            references: AtomicU32::new(1),
            key: factory.key,
            dispatcher: factory.dispatcher.clone(),
        })) as *mut c_void;
        let hr = activator_qi(activator, iid, out);
        activator_release(activator);
        hr
    })
}

unsafe extern "system" fn factory_lock(_this: *mut c_void, _lock: BOOL) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || S_OK)
}

#[repr(C)]
struct ActivatorVtbl {
    qi: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    activate: unsafe extern "system" fn(
        *mut c_void,
        *const u16,
        *const u16,
        *const NOTIFICATION_USER_INPUT_DATA,
        u32,
    ) -> HRESULT,
}

#[repr(C)]
struct Activator {
    vtable: *const ActivatorVtbl,
    references: AtomicU32,
    key: u64,
    dispatcher: UiDispatcher,
}

static ACTIVATOR_VTBL: ActivatorVtbl = ActivatorVtbl {
    qi: activator_qi,
    add_ref: activator_add_ref,
    release: activator_release,
    activate: activator_activate,
};

unsafe extern "system" fn activator_qi(this: *mut c_void, iid: *const GUID, out: *mut *mut c_void) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if out.is_null() || iid.is_null() {
            return E_POINTER;
        }
        if *iid == IID_IUnknown || *iid == IID_INotificationActivationCallback || *iid == IID_IAgileObject {
            activator_add_ref(this);
            *out = this;
            S_OK
        } else {
            *out = std::ptr::null_mut();
            E_NOINTERFACE
        }
    })
}

unsafe extern "system" fn activator_add_ref(this: *mut c_void) -> u32 {
    crate::core::ffi_guard::guard(0, move || (*(this as *mut Activator)).references.fetch_add(1, Ordering::SeqCst) + 1)
}

unsafe extern "system" fn activator_release(this: *mut c_void) -> u32 {
    crate::core::ffi_guard::guard(0, move || {
        let remaining = (*(this as *mut Activator)).references.fetch_sub(1, Ordering::SeqCst) - 1;
        if remaining == 0 {
            drop(Box::from_raw(this as *mut Activator));
        }
        remaining
    })
}

unsafe extern "system" fn activator_activate(
    this: *mut c_void,
    _app_id: *const u16,
    arguments: *const u16,
    data: *const NOTIFICATION_USER_INPUT_DATA,
    count: u32,
) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        let activator = &*(this as *mut Activator);
        let mut activation = NotificationActivation { argument: from_wide_ptr(arguments), inputs: Vec::new() };
        if !data.is_null() {
            for entry in std::slice::from_raw_parts(data, count as usize) {
                activation.inputs.push((from_wide_ptr(entry.Key), from_wide_ptr(entry.Value)));
            }
        }
        // Lido aqui (thread do COM); o callback roda na thread da interface.
        let key = activator.key;
        let _ = activator.dispatcher.post(move || {
            let callback = CALLBACKS.with(|c| c.borrow().get(&key).cloned());
            if let Some(callback) = callback {
                callback(&activation);
            }
        });
        S_OK
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guids_parse_and_format_round_trip() {
        let text = "{0F3C1B2A-9D4E-4C11-8A77-3B5E6D7F8091}";
        let guid = parse_guid(text).unwrap();
        assert_eq!(guid.data1, 0x0F3C1B2A);
        assert_eq!(guid.data4, [0x8A, 0x77, 0x3B, 0x5E, 0x6D, 0x7F, 0x80, 0x91]);
        assert_eq!(format_guid(&guid), text);
        assert_eq!(parse_guid("0f3c1b2a-9d4e-4c11-8a77-3b5e6d7f8091"), Some(guid));
        assert_eq!(parse_guid("0f3c1b2a9d4e4c118a773b5e6d7f8091"), None, "sem hífens");
        assert_eq!(parse_guid("{0F3C1B2A-9D4E-4C11-8A77-3B5E6D7F80}"), None);
    }
}
