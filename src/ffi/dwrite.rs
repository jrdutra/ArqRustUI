//! Declarations for `dwrite.dll` (DirectWrite).

use super::types::*;
use std::ffi::c_void;

pub const IID_IDWriteFactory: GUID =
    GUID::new(0xB859EE5A, 0xD838, 0x4B5B, [0xA2, 0xE8, 0x1A, 0xDC, 0x7D, 0x93, 0xDB, 0x48]);
pub const DWRITE_FACTORY_TYPE_SHARED: u32 = 0;
pub const DWRITE_FONT_WEIGHT_NORMAL: u32 = 400;
pub const DWRITE_FONT_WEIGHT_BOLD: u32 = 700;
pub const DWRITE_FONT_STYLE_NORMAL: u32 = 0;
pub const DWRITE_FONT_STYLE_ITALIC: u32 = 2;
pub const DWRITE_FONT_STRETCH_NORMAL: u32 = 5;

#[link(name = "dwrite")]
extern "system" {
    pub fn DWriteCreateFactory(factory_type: u32, iid: *const GUID, factory: *mut *mut c_void) -> HRESULT;
}
