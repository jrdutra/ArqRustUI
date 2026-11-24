//! Dynamic DLL loading and capability detection.
//!
//! APIs missing on supported Windows versions are never statically imported: resolved here with `LoadLibraryW` +
//! `GetProcAddress`, returning `None` if absent.

use super::kernel32::{GetProcAddress, LoadLibraryW};
use super::types::*;
use std::ffi::c_void;
use std::sync::OnceLock;

/// Process-loaded DLL, never unloaded (process lifetime).
pub struct Library(HMODULE);

impl Library {
    /// Loads/gets the DLL; `None` if absent.
    pub fn load(name: &str) -> Option<Self> {
        let wide: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
        // SAFETY: `wide` é uma string UTF-16 terminada em NUL válida durante a chamada.
        let module = unsafe { LoadLibraryW(wide.as_ptr()) };
        if module.is_null() {
            None
        } else {
            Some(Self(module))
        }
    }

    /// Resolves an exported symbol as function pointer type `T`.
    ///
    /// # Safety
    /// `T` must be an `extern "system"` function pointer exactly matching the exported signature. `name` must end in
    /// NUL.
    pub unsafe fn symbol<T: Copy>(&self, name: &[u8]) -> Option<T> {
        assert_eq!(name.last(), Some(&0), "symbol name without NUL terminator");
        assert_eq!(std::mem::size_of::<T>(), std::mem::size_of::<*mut c_void>());
        let address = GetProcAddress(self.0, name.as_ptr());
        if address.is_null() {
            None
        } else {
            Some(std::mem::transmute_copy::<*mut c_void, T>(&address))
        }
    }
}

/// Optional functions on recent Windows versions.
pub struct OptionalApis {
    /// `GetDpiForWindow` (Windows 10 1607+).
    pub GetDpiForWindow: Option<unsafe extern "system" fn(HWND) -> UINT>,
    /// `GetDpiForSystem` (Windows 10 1607+).
    pub GetDpiForSystem: Option<unsafe extern "system" fn() -> UINT>,
    /// `SetProcessDpiAwarenessContext` (Windows 10 1703+).
    pub SetProcessDpiAwarenessContext: Option<unsafe extern "system" fn(isize) -> BOOL>,
    /// `AdjustWindowRectExForDpi` (Windows 10 1607+).
    pub AdjustWindowRectExForDpi: Option<unsafe extern "system" fn(*mut RECT, DWORD, BOOL, DWORD, UINT) -> BOOL>,
    /// `GetDpiForMonitor` from `shcore.dll` (Windows 8.1+).
    pub GetDpiForMonitor: Option<unsafe extern "system" fn(HMONITOR, i32, *mut UINT, *mut UINT) -> HRESULT>,
    /// `GetSystemMetricsForDpi` (Windows 10 1607+).
    pub GetSystemMetricsForDpi: Option<unsafe extern "system" fn(i32, UINT) -> i32>,
    /// `GetPointerType` (Windows 8+).
    pub GetPointerType: Option<unsafe extern "system" fn(u32, *mut u32) -> BOOL>,
    /// `SetGestureConfig` (Windows 7+).
    pub SetGestureConfig:
        Option<unsafe extern "system" fn(HWND, DWORD, UINT, *const super::user32::GESTURECONFIG, UINT) -> BOOL>,
    /// `GetGestureInfo` (Windows 7+).
    pub GetGestureInfo: Option<unsafe extern "system" fn(isize, *mut super::user32::GESTUREINFO) -> BOOL>,
    /// `CloseGestureInfoHandle` (Windows 7+).
    pub CloseGestureInfoHandle: Option<unsafe extern "system" fn(isize) -> BOOL>,
    /// `UiaRaiseNotificationEvent` (Windows 10 1709+).
    pub UiaRaiseNotificationEvent:
        Option<unsafe extern "system" fn(*mut c_void, i32, i32, *mut u16, *mut u16) -> HRESULT>,
    /// `UiaReturnRawElementProvider`.
    pub UiaReturnRawElementProvider: Option<unsafe extern "system" fn(HWND, WPARAM, LPARAM, *mut c_void) -> LRESULT>,
    /// `UiaHostProviderFromHwnd`.
    pub UiaHostProviderFromHwnd: Option<unsafe extern "system" fn(HWND, *mut *mut c_void) -> HRESULT>,
    /// `UiaRaiseAutomationEvent`.
    pub UiaRaiseAutomationEvent: Option<unsafe extern "system" fn(*mut c_void, i32) -> HRESULT>,
    /// `UiaClientsAreListening`.
    pub UiaClientsAreListening: Option<unsafe extern "system" fn() -> BOOL>,
    /// `UiaGetReservedNotSupportedValue`.
    pub UiaGetReservedNotSupportedValue: Option<unsafe extern "system" fn(*mut *mut c_void) -> HRESULT>,
    /// `UiaRaiseAutomationPropertyChangedEvent`.
    pub UiaRaiseAutomationPropertyChangedEvent:
        Option<unsafe extern "system" fn(*mut c_void, i32, super::ole32::VARIANT, super::ole32::VARIANT) -> HRESULT>,
    /// `UiaRaiseStructureChangedEvent`.
    pub UiaRaiseStructureChangedEvent: Option<unsafe extern "system" fn(*mut c_void, i32, *mut i32, i32) -> HRESULT>,
}

static OPTIONAL: OnceLock<OptionalApis> = OnceLock::new();

/// Returns optional API table, resolved once.
pub fn optional() -> &'static OptionalApis {
    OPTIONAL.get_or_init(|| {
        let user32 = Library::load("user32.dll");
        let shcore = Library::load("shcore.dll");
        let uia = Library::load("uiautomationcore.dll");
        // SAFETY: cada símbolo é convertido para a assinatura documentada da função.
        unsafe {
            OptionalApis {
                GetDpiForWindow: user32.as_ref().and_then(|l| l.symbol(b"GetDpiForWindow\0")),
                GetDpiForSystem: user32.as_ref().and_then(|l| l.symbol(b"GetDpiForSystem\0")),
                SetProcessDpiAwarenessContext: user32
                    .as_ref()
                    .and_then(|l| l.symbol(b"SetProcessDpiAwarenessContext\0")),
                AdjustWindowRectExForDpi: user32.as_ref().and_then(|l| l.symbol(b"AdjustWindowRectExForDpi\0")),
                GetDpiForMonitor: shcore.as_ref().and_then(|l| l.symbol(b"GetDpiForMonitor\0")),
                GetSystemMetricsForDpi: user32.as_ref().and_then(|l| l.symbol(b"GetSystemMetricsForDpi\0")),
                GetPointerType: user32.as_ref().and_then(|l| l.symbol(b"GetPointerType\0")),
                SetGestureConfig: user32.as_ref().and_then(|l| l.symbol(b"SetGestureConfig\0")),
                GetGestureInfo: user32.as_ref().and_then(|l| l.symbol(b"GetGestureInfo\0")),
                CloseGestureInfoHandle: user32.as_ref().and_then(|l| l.symbol(b"CloseGestureInfoHandle\0")),
                UiaRaiseNotificationEvent: uia.as_ref().and_then(|l| l.symbol(b"UiaRaiseNotificationEvent\0")),
                UiaReturnRawElementProvider: uia.as_ref().and_then(|l| l.symbol(b"UiaReturnRawElementProvider\0")),
                UiaHostProviderFromHwnd: uia.as_ref().and_then(|l| l.symbol(b"UiaHostProviderFromHwnd\0")),
                UiaRaiseAutomationEvent: uia.as_ref().and_then(|l| l.symbol(b"UiaRaiseAutomationEvent\0")),
                UiaClientsAreListening: uia.as_ref().and_then(|l| l.symbol(b"UiaClientsAreListening\0")),
                UiaGetReservedNotSupportedValue: uia
                    .as_ref()
                    .and_then(|l| l.symbol(b"UiaGetReservedNotSupportedValue\0")),
                UiaRaiseAutomationPropertyChangedEvent: uia
                    .as_ref()
                    .and_then(|l| l.symbol(b"UiaRaiseAutomationPropertyChangedEvent\0")),
                UiaRaiseStructureChangedEvent: uia.as_ref().and_then(|l| l.symbol(b"UiaRaiseStructureChangedEvent\0")),
            }
        }
    })
}
