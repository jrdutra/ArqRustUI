//! Declarations for `uxtheme.dll`.

use super::types::*;

#[link(name = "uxtheme")]
extern "system" {
    pub fn SetWindowTheme(hwnd: HWND, app_name: PCWSTR, id_list: PCWSTR) -> HRESULT;
    pub fn IsAppThemed() -> BOOL;
    pub fn OpenThemeData(hwnd: HWND, class_list: PCWSTR) -> HANDLE;
    pub fn CloseThemeData(theme: HANDLE) -> HRESULT;
    pub fn DrawThemeText(
        theme: HANDLE,
        hdc: HDC,
        part: i32,
        state: i32,
        text: PCWSTR,
        count: i32,
        flags: DWORD,
        flags2: DWORD,
        rect: *const RECT,
    ) -> HRESULT;
}

/// Command button part of the `BUTTON` theme class.
pub const BP_PUSHBUTTON: i32 = 1;
pub const PBS_NORMAL: i32 = 1;
pub const PBS_HOT: i32 = 2;
pub const PBS_PRESSED: i32 = 3;
pub const PBS_DISABLED: i32 = 4;
pub const PBS_DEFAULTED: i32 = 5;
