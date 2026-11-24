//! Declarations for `dwmapi.dll` (Desktop Window Manager).

use super::types::*;
use std::ffi::c_void;

pub const DWMWA_USE_IMMERSIVE_DARK_MODE: DWORD = 20;
pub const DWMWA_WINDOW_CORNER_PREFERENCE: DWORD = 33;
pub const DWMWA_BORDER_COLOR: DWORD = 34;
pub const DWMWA_CAPTION_COLOR: DWORD = 35;
pub const DWMWA_TEXT_COLOR: DWORD = 36;
pub const DWMWA_SYSTEMBACKDROP_TYPE: DWORD = 38;

#[repr(C)]
pub struct MARGINS {
    pub cxLeftWidth: i32,
    pub cxRightWidth: i32,
    pub cyTopHeight: i32,
    pub cyBottomHeight: i32,
}

#[link(name = "dwmapi")]
extern "system" {
    pub fn DwmSetWindowAttribute(hwnd: HWND, attribute: DWORD, value: *const c_void, size: DWORD) -> HRESULT;
    pub fn DwmExtendFrameIntoClientArea(hwnd: HWND, margins: *const MARGINS) -> HRESULT;
    pub fn DwmIsCompositionEnabled(enabled: *mut BOOL) -> HRESULT;
}
