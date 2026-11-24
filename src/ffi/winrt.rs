//! Minimal Windows Runtime for system notifications: `HSTRING`, class activation (`combase.dll`, dynamically resolved,
//! absent before Windows 8) and `AppUserModelID` registration (`advapi32.dll`, `shell32.dll`).

use super::dynamic::Library;
use super::types::*;
use std::ffi::c_void;
use std::sync::OnceLock;

pub type HSTRING = *mut c_void;

pub const HKEY_CURRENT_USER: HKEY = 0x8000_0001_usize as HKEY;
pub const KEY_WRITE: DWORD = 0x0002_0006;
pub const REG_SZ: DWORD = 1;

pub const IID_IAgileObject: GUID =
    GUID::new(0x94EA2B94, 0xE9CC, 0x49E0, [0xC0, 0xFF, 0xEE, 0x64, 0xCA, 0x8F, 0x5B, 0x90]);
pub const IID_IXmlDocumentIO: GUID =
    GUID::new(0x6CD0E74E, 0xEE65, 0x4489, [0x9E, 0xBF, 0xCA, 0x43, 0xE8, 0x7B, 0xA6, 0x37]);
pub const IID_IXmlDocument: GUID =
    GUID::new(0xF7F3A506, 0x1E87, 0x42D6, [0xBC, 0xFB, 0xB8, 0xC8, 0x09, 0xFA, 0x54, 0x94]);
pub const IID_IToastNotificationFactory: GUID =
    GUID::new(0x04124B20, 0x82C6, 0x4229, [0xB1, 0x09, 0xFD, 0x9E, 0xD4, 0x66, 0x2B, 0x53]);
pub const IID_IToastNotificationManagerStatics: GUID =
    GUID::new(0x50AC103F, 0xD235, 0x4598, [0xBB, 0xEF, 0x98, 0xFE, 0x4D, 0x1A, 0x3A, 0xD4]);
pub const IID_IToastActivatedEventArgs: GUID =
    GUID::new(0xE3BF92F3, 0xC197, 0x436F, [0x82, 0x65, 0x06, 0x25, 0x82, 0x4F, 0x8D, 0xAC]);
pub const IID_IToastDismissedEventArgs: GUID =
    GUID::new(0x3F89D935, 0xD9CB, 0x4538, [0xA0, 0xF0, 0xFF, 0xE7, 0x65, 0x99, 0x38, 0xF8]);
/// `TypedEventHandler<ToastNotification, IInspectable>` (IID calculated by WinRT parameterized-type rules; see
/// `system::notifications` test).
pub const IID_ActivatedHandler: GUID =
    GUID::new(0xAB54DE2D, 0x97D9, 0x5528, [0xB6, 0xAD, 0x10, 0x5A, 0xFE, 0x15, 0x65, 0x30]);
/// `TypedEventHandler<ToastNotification, ToastDismissedEventArgs>`.
pub const IID_DismissedHandler: GUID =
    GUID::new(0x61C2402F, 0x0ED0, 0x5A18, [0xAB, 0x69, 0x59, 0xF4, 0xAA, 0x99, 0xA3, 0x68]);
/// `TypedEventHandler<ToastNotification, ToastFailedEventArgs>`.
pub const IID_FailedHandler: GUID =
    GUID::new(0x95E3E803, 0xC969, 0x5E3A, [0x97, 0x53, 0xEA, 0x2A, 0xD2, 0x2A, 0x9A, 0x33]);

/// `combase.dll` functions.
pub struct WinRtApis {
    pub WindowsCreateString: unsafe extern "system" fn(*const u16, u32, *mut HSTRING) -> HRESULT,
    pub WindowsDeleteString: unsafe extern "system" fn(HSTRING) -> HRESULT,
    pub WindowsGetStringRawBuffer: unsafe extern "system" fn(HSTRING, *mut u32) -> *const u16,
    pub RoActivateInstance: unsafe extern "system" fn(HSTRING, *mut *mut c_void) -> HRESULT,
    pub RoGetActivationFactory: unsafe extern "system" fn(HSTRING, *const GUID, *mut *mut c_void) -> HRESULT,
}

static WINRT: OnceLock<Option<WinRtApis>> = OnceLock::new();

/// WinRT functions, or `None` without `combase.dll` (Windows 7).
pub fn winrt() -> Option<&'static WinRtApis> {
    WINRT
        .get_or_init(|| {
            let lib = Library::load("combase.dll")?;
            // SAFETY: cada símbolo é convertido para a assinatura documentada da função.
            unsafe {
                let apis = WinRtApis {
                    WindowsCreateString: lib.symbol(b"WindowsCreateString\0")?,
                    WindowsDeleteString: lib.symbol(b"WindowsDeleteString\0")?,
                    WindowsGetStringRawBuffer: lib.symbol(b"WindowsGetStringRawBuffer\0")?,
                    RoActivateInstance: lib.symbol(b"RoActivateInstance\0")?,
                    RoGetActivationFactory: lib.symbol(b"RoGetActivationFactory\0")?,
                };
                Some(apis)
            }
        })
        .as_ref()
}

#[link(name = "advapi32")]
extern "system" {
    pub fn RegCreateKeyExW(
        key: HKEY,
        subkey: PCWSTR,
        reserved: DWORD,
        class: *const u16,
        options: DWORD,
        access: DWORD,
        security: *const c_void,
        result: *mut HKEY,
        disposition: *mut DWORD,
    ) -> i32;
    pub fn RegSetValueExW(key: HKEY, name: PCWSTR, reserved: DWORD, kind: DWORD, data: *const u8, size: DWORD) -> i32;
    pub fn RegCloseKey(key: HKEY) -> i32;
}

#[link(name = "shell32")]
extern "system" {
    pub fn SetCurrentProcessExplicitAppUserModelID(id: PCWSTR) -> HRESULT;
}

pub const IID_IToastNotification2: GUID =
    GUID::new(0x9DFB9FD1, 0x143A, 0x490E, [0x90, 0xBF, 0xB9, 0xFB, 0xA7, 0x13, 0x2D, 0xE7]);
pub const IID_IToastNotification4: GUID =
    GUID::new(0x15154935, 0x28EA, 0x4727, [0x88, 0xE9, 0xC5, 0x86, 0x80, 0xE2, 0xD1, 0x18]);
pub const IID_INotificationData: GUID =
    GUID::new(0x9FFD2312, 0x9D6A, 0x4AAF, [0xB6, 0xAC, 0xFF, 0x17, 0xF0, 0xC1, 0xF2, 0x80]);
pub const IID_IToastNotifier2: GUID =
    GUID::new(0x354389C6, 0x7C01, 0x4BD5, [0x9C, 0x20, 0x60, 0x43, 0x40, 0xCD, 0x2B, 0x74]);
pub const IID_IToastNotificationManagerStatics2: GUID =
    GUID::new(0x7AB93C52, 0x0E48, 0x4750, [0xBA, 0x9D, 0x1A, 0x41, 0x13, 0x98, 0x18, 0x47]);
pub const IID_IToastNotificationHistory: GUID =
    GUID::new(0x5CADDC63, 0x01D3, 0x4C97, [0x98, 0x6F, 0x05, 0x33, 0x48, 0x3F, 0xEE, 0x14]);
pub const IID_IScheduledToastNotificationFactory: GUID =
    GUID::new(0xE7BED191, 0x0BB9, 0x4189, [0x83, 0x94, 0x31, 0x76, 0x1B, 0x47, 0x6F, 0xD7]);
pub const IID_IScheduledToastNotification2: GUID =
    GUID::new(0xA66EA09C, 0x31B4, 0x43B0, [0xB5, 0xDD, 0x7A, 0x40, 0xE8, 0x53, 0x63, 0xB1]);
pub const IID_IToastActivatedEventArgs2: GUID =
    GUID::new(0xAB7DA512, 0xCC61, 0x568E, [0x81, 0xBE, 0x30, 0x4A, 0xC3, 0x10, 0x38, 0xFA]);
/// `IMap<String, Object>` (user-input `ValueSet`).
pub const IID_IMap_String_Object: GUID =
    GUID::new(0x1B0D3570, 0x0877, 0x5EC2, [0x8A, 0x2C, 0x3B, 0x95, 0x39, 0x50, 0x6A, 0xCA]);
/// `IMap<String, String>` (`NotificationData` values).
pub const IID_IMap_String_String: GUID =
    GUID::new(0xF6D1F700, 0x49C2, 0x52AE, [0x81, 0x54, 0x82, 0x6F, 0x99, 0x08, 0x77, 0x3C]);
pub const IID_IPropertyValue: GUID =
    GUID::new(0x4BD682DD, 0x7554, 0x40E9, [0x9A, 0x9B, 0x82, 0x65, 0x4E, 0xDE, 0x7E, 0x62]);
pub const IID_INotificationActivationCallback: GUID =
    GUID::new(0x53E31837, 0x6600, 0x4A81, [0x93, 0x95, 0x75, 0xCF, 0xFE, 0x74, 0x6F, 0x94]);
pub const IID_IClassFactory: GUID =
    GUID::new(0x00000001, 0x0000, 0x0000, [0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46]);

pub const CLSCTX_LOCAL_SERVER: DWORD = 0x4;
pub const REGCLS_MULTIPLEUSE: DWORD = 1;
pub const KEY_READ: DWORD = 0x0002_0019;

/// `NOTIFICATION_USER_INPUT_DATA`.
#[repr(C)]
pub struct NOTIFICATION_USER_INPUT_DATA {
    pub Key: PCWSTR,
    pub Value: PCWSTR,
}

#[link(name = "ole32")]
extern "system" {
    pub fn CoRegisterClassObject(
        clsid: *const GUID,
        unknown: *mut c_void,
        context: DWORD,
        flags: DWORD,
        cookie: *mut DWORD,
    ) -> HRESULT;
    pub fn CoRevokeClassObject(cookie: DWORD) -> HRESULT;
}

#[link(name = "advapi32")]
extern "system" {
    pub fn RegOpenKeyExW(key: HKEY, subkey: PCWSTR, options: DWORD, access: DWORD, result: *mut HKEY) -> i32;
    pub fn RegQueryValueExW(
        key: HKEY,
        name: PCWSTR,
        reserved: *mut DWORD,
        kind: *mut DWORD,
        data: *mut u8,
        size: *mut DWORD,
    ) -> i32;
    pub fn RegDeleteTreeW(key: HKEY, subkey: PCWSTR) -> i32;
}
