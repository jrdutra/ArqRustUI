//! Declarations for `shell32.dll` and `shlwapi.dll`.

use super::types::*;
use std::ffi::c_void;

pub const NIM_ADD: DWORD = 0;
pub const NIM_MODIFY: DWORD = 1;
pub const NIM_DELETE: DWORD = 2;
pub const NIF_MESSAGE: UINT = 0x1;
pub const NIF_ICON: UINT = 0x2;
pub const NIF_TIP: UINT = 0x4;
pub const NIF_INFO: UINT = 0x10;
pub const NIIF_INFO: DWORD = 0x1;
pub const NIIF_WARNING: DWORD = 0x2;
pub const NIIF_ERROR: DWORD = 0x3;
pub const NIN_BALLOONUSERCLICK: u32 = 0x405;
pub const ASSOCSTR_EXECUTABLE: i32 = 2;
pub const ASSOCSTR_FRIENDLYAPPNAME: i32 = 4;
pub const SIGDN_FILESYSPATH: u32 = 0x8005_8000;
pub const FOS_OVERWRITEPROMPT: u32 = 0x2;
pub const FOS_PICKFOLDERS: u32 = 0x20;
pub const FOS_FORCEFILESYSTEM: u32 = 0x40;
pub const FOS_ALLOWMULTISELECT: u32 = 0x200;
pub const FOS_PATHMUSTEXIST: u32 = 0x800;
pub const FOS_FILEMUSTEXIST: u32 = 0x1000;

pub const CLSID_FileOpenDialog: GUID =
    GUID::new(0xDC1C5A9C, 0xE88A, 0x4DDE, [0xA5, 0xA1, 0x60, 0xF8, 0x2A, 0x20, 0xAE, 0xF7]);
pub const CLSID_FileSaveDialog: GUID =
    GUID::new(0xC0B4E2F3, 0xBA21, 0x4773, [0x8D, 0xBA, 0x33, 0x5E, 0xC9, 0x46, 0xEB, 0x8B]);
pub const IID_IFileOpenDialog: GUID =
    GUID::new(0xD57C7288, 0xD4AD, 0x4768, [0xBE, 0x02, 0x9D, 0x96, 0x95, 0x32, 0xD9, 0x60]);
pub const IID_IFileSaveDialog: GUID =
    GUID::new(0x84BCCD23, 0x5FDE, 0x4CDB, [0xAE, 0xA4, 0xAF, 0x64, 0xB8, 0x3D, 0x78, 0xAB]);
pub const IID_IShellItem: GUID =
    GUID::new(0x43826D1E, 0xE718, 0x42EE, [0xBC, 0x55, 0xA1, 0xE2, 0x61, 0xC3, 0x7B, 0xFE]);
pub const FOLDERID_Documents: GUID =
    GUID::new(0xFDD39AD0, 0x238F, 0x46AF, [0xAD, 0xB4, 0x6C, 0x85, 0x48, 0x03, 0x69, 0xC7]);
pub const FOLDERID_Desktop: GUID =
    GUID::new(0xB4BFCC3A, 0xDB2C, 0x424C, [0xB0, 0x29, 0x7F, 0xE9, 0x9A, 0x87, 0xC6, 0x41]);
pub const FOLDERID_Downloads: GUID =
    GUID::new(0x374DE290, 0x123F, 0x4565, [0x91, 0x64, 0x39, 0xC4, 0x92, 0x5E, 0x46, 0x7B]);
pub const FOLDERID_RoamingAppData: GUID =
    GUID::new(0x3EB685DB, 0x65F9, 0x4CF6, [0xA0, 0x3A, 0xE3, 0xEF, 0x65, 0x72, 0x9F, 0x3D]);
pub const FOLDERID_LocalAppData: GUID =
    GUID::new(0xF1B32785, 0x6FBA, 0x4FCF, [0x9D, 0x55, 0x7B, 0x8E, 0x7F, 0x15, 0x70, 0x91]);
pub const FOLDERID_Profile: GUID =
    GUID::new(0x5E6C858F, 0x0E22, 0x4760, [0x9A, 0xFE, 0xEA, 0x33, 0x17, 0xB6, 0x71, 0x73]);
pub const FOLDERID_Pictures: GUID =
    GUID::new(0x33E28130, 0x4E1E, 0x4676, [0x83, 0x5A, 0x98, 0x39, 0x5C, 0x3B, 0xC3, 0xBB]);

#[repr(C)]
pub struct NOTIFYICONDATAW {
    pub cbSize: DWORD,
    pub hWnd: HWND,
    pub uID: UINT,
    pub uFlags: UINT,
    pub uCallbackMessage: UINT,
    pub hIcon: HICON,
    pub szTip: [u16; 128],
    pub dwState: DWORD,
    pub dwStateMask: DWORD,
    pub szInfo: [u16; 256],
    pub uVersion: UINT,
    pub szInfoTitle: [u16; 64],
    pub dwInfoFlags: DWORD,
    pub guidItem: GUID,
    pub hBalloonIcon: HICON,
}

#[repr(C)]
pub struct COMDLG_FILTERSPEC {
    pub pszName: PCWSTR,
    pub pszSpec: PCWSTR,
}

#[link(name = "shell32")]
extern "system" {
    pub fn Shell_NotifyIconW(message: DWORD, data: *const NOTIFYICONDATAW) -> BOOL;
    pub fn ShellExecuteW(
        hwnd: HWND,
        operation: PCWSTR,
        file: PCWSTR,
        parameters: PCWSTR,
        directory: PCWSTR,
        show: i32,
    ) -> HINSTANCE;
    pub fn DragAcceptFiles(hwnd: HWND, accept: BOOL);
    pub fn SHCreateDataObject(
        folder: *const c_void,
        count: UINT,
        items: *const *const c_void,
        inner: *mut c_void,
        iid: *const GUID,
        out: *mut *mut c_void,
    ) -> HRESULT;
    pub fn DragQueryFileW(drop: HDROP, index: UINT, buffer: PWSTR, size: UINT) -> UINT;
    pub fn DragFinish(drop: HDROP);
    pub fn SHGetKnownFolderPath(id: *const GUID, flags: DWORD, token: HANDLE, path: *mut PWSTR) -> HRESULT;
    pub fn SHCreateItemFromParsingName(
        path: PCWSTR,
        bind_ctx: *mut c_void,
        iid: *const GUID,
        out: *mut *mut c_void,
    ) -> HRESULT;
}

#[link(name = "shlwapi")]
extern "system" {
    pub fn AssocQueryStringW(
        flags: DWORD,
        kind: i32,
        assoc: PCWSTR,
        extra: PCWSTR,
        out: PWSTR,
        size: *mut DWORD,
    ) -> HRESULT;
}
