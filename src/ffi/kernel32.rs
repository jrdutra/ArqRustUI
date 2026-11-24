//! Declarations for `kernel32.dll`, `ntdll.dll` and `advapi32.dll`.

use super::types::*;
use std::ffi::c_void;

pub const FORMAT_MESSAGE_FROM_SYSTEM: DWORD = 0x0000_1000;
pub const FORMAT_MESSAGE_IGNORE_INSERTS: DWORD = 0x0000_0200;
pub const GMEM_MOVEABLE: UINT = 0x0002;
pub const ACTCTX_FLAG_ASSEMBLY_DIRECTORY_VALID: DWORD = 0x004;
pub const ACTCTX_FLAG_RESOURCE_NAME_VALID: DWORD = 0x008;
pub const ACTCTX_FLAG_SET_PROCESS_DEFAULT: DWORD = 0x010;
pub const INVALID_HANDLE_VALUE: HANDLE = -1isize as HANDLE;
pub const LOCALE_NAME_MAX_LENGTH: usize = 85;
pub const DATE_SHORTDATE: DWORD = 0x1;
pub const DATE_LONGDATE: DWORD = 0x2;
pub const TIME_NOSECONDS: DWORD = 0x2;
pub const LOCALE_IREADINGLAYOUT: DWORD = 0x0000_0070;
pub const LOCALE_SLOCALIZEDDISPLAYNAME: DWORD = 0x0000_0002;
pub const LOCALE_SDECIMAL: DWORD = 0x0000_000E;
pub const LOCALE_STHOUSAND: DWORD = 0x0000_000F;
pub const HKEY_CURRENT_USER: HKEY = 0x8000_0001_usize as HKEY;
pub const RRF_RT_REG_DWORD: DWORD = 0x0000_0010;
pub const ERROR_SUCCESS: u32 = 0;
pub const ERROR_CLASS_ALREADY_EXISTS: u32 = 1410;

#[repr(C)]
pub struct ACTCTXW {
    pub cbSize: u32,
    pub dwFlags: DWORD,
    pub lpSource: PCWSTR,
    pub wProcessorArchitecture: u16,
    pub wLangId: u16,
    pub lpAssemblyDirectory: PCWSTR,
    pub lpResourceName: PCWSTR,
    pub lpApplicationName: PCWSTR,
    pub hModule: HMODULE,
}

#[repr(C)]
pub struct OSVERSIONINFOW {
    pub dwOSVersionInfoSize: u32,
    pub dwMajorVersion: u32,
    pub dwMinorVersion: u32,
    pub dwBuildNumber: u32,
    pub dwPlatformId: u32,
    pub szCSDVersion: [u16; 128],
}

#[link(name = "kernel32")]
extern "system" {
    pub fn GetModuleHandleW(name: PCWSTR) -> HINSTANCE;
    pub fn GetLastError() -> DWORD;
    pub fn SetLastError(code: DWORD);
    pub fn FormatMessageW(
        flags: DWORD,
        source: *const c_void,
        message_id: DWORD,
        language_id: DWORD,
        buffer: PWSTR,
        size: DWORD,
        args: *mut c_void,
    ) -> DWORD;
    pub fn GetCurrentThreadId() -> DWORD;
    pub fn GetCurrentProcess() -> HANDLE;
    pub fn LoadLibraryW(name: PCWSTR) -> HMODULE;
    pub fn GetProcAddress(module: HMODULE, name: *const u8) -> *mut c_void;
    pub fn GlobalAlloc(flags: UINT, bytes: usize) -> HGLOBAL;
    pub fn GlobalLock(mem: HGLOBAL) -> *mut c_void;
    pub fn GlobalUnlock(mem: HGLOBAL) -> BOOL;
    pub fn GlobalFree(mem: HGLOBAL) -> HGLOBAL;
    pub fn GlobalSize(mem: HGLOBAL) -> usize;
    pub fn MulDiv(number: i32, numerator: i32, denominator: i32) -> i32;
    pub fn CreateActCtxW(ctx: *const ACTCTXW) -> HANDLE;
    pub fn ActivateActCtx(ctx: HANDLE, cookie: *mut usize) -> BOOL;
    pub fn DeactivateActCtx(flags: DWORD, cookie: usize) -> BOOL;
    pub fn ReleaseActCtx(ctx: HANDLE);
    pub fn GetSystemDirectoryW(buffer: PWSTR, size: UINT) -> UINT;
    pub fn GetUserDefaultLocaleName(buffer: PWSTR, size: i32) -> i32;
    pub fn GetUserDefaultUILanguage() -> u16;
    pub fn GetLocaleInfoEx(locale: PCWSTR, lctype: DWORD, data: PWSTR, size: i32) -> i32;
    pub fn GetDateFormatEx(
        locale: PCWSTR,
        flags: DWORD,
        date: *const SYSTEMTIME,
        format: PCWSTR,
        out: PWSTR,
        size: i32,
        calendar: PCWSTR,
    ) -> i32;
    pub fn GetTimeFormatEx(
        locale: PCWSTR,
        flags: DWORD,
        time: *const SYSTEMTIME,
        format: PCWSTR,
        out: PWSTR,
        size: i32,
    ) -> i32;
    pub fn GetNumberFormatEx(
        locale: PCWSTR,
        flags: DWORD,
        value: PCWSTR,
        format: *const c_void,
        out: PWSTR,
        size: i32,
    ) -> i32;
    pub fn GetCurrencyFormatEx(
        locale: PCWSTR,
        flags: DWORD,
        value: PCWSTR,
        format: *const c_void,
        out: PWSTR,
        size: i32,
    ) -> i32;
    pub fn GetLocalTime(time: *mut SYSTEMTIME);
}

#[link(name = "ntdll")]
extern "system" {
    pub fn RtlGetVersion(info: *mut OSVERSIONINFOW) -> i32;
}

#[link(name = "advapi32")]
extern "system" {
    pub fn RegGetValueW(
        key: HKEY,
        subkey: PCWSTR,
        value: PCWSTR,
        flags: DWORD,
        kind: *mut DWORD,
        data: *mut c_void,
        size: *mut DWORD,
    ) -> i32;
}
