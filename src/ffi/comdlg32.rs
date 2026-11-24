//! Declarations for `comdlg32.dll` (classic common dialogs).

use super::gdi32::LOGFONTW;
use super::types::*;
use std::ffi::c_void;

pub const CC_RGBINIT: DWORD = 0x1;
pub const CC_FULLOPEN: DWORD = 0x2;
pub const CC_ANYCOLOR: DWORD = 0x100;
pub const CF_SCREENFONTS: DWORD = 0x1;
pub const CF_INITTOLOGFONTSTRUCT: DWORD = 0x40;
pub const CF_EFFECTS: DWORD = 0x100;
pub const PD_SELECTION: DWORD = 0x1;
pub const PD_PAGENUMS: DWORD = 0x2;
pub const PD_NOSELECTION: DWORD = 0x4;
pub const PD_NOPAGENUMS: DWORD = 0x8;
pub const PD_COLLATE: DWORD = 0x10;
pub const PD_RETURNDC: DWORD = 0x100;
pub const PD_USEDEVMODECOPIESANDCOLLATE: DWORD = 0x40000;
pub const PSD_MARGINS: DWORD = 0x2;
pub const PSD_INHUNDREDTHSOFMILLIMETERS: DWORD = 0x8;
pub const FR_DOWN: DWORD = 0x1;
pub const FR_WHOLEWORD: DWORD = 0x2;
pub const FR_MATCHCASE: DWORD = 0x4;
pub const FR_FINDNEXT: DWORD = 0x8;
pub const FR_REPLACE: DWORD = 0x10;
pub const FR_REPLACEALL: DWORD = 0x20;
pub const FR_DIALOGTERM: DWORD = 0x40;

#[repr(C)]
pub struct CHOOSECOLORW {
    pub lStructSize: DWORD,
    pub hwndOwner: HWND,
    pub hInstance: HWND,
    pub rgbResult: COLORREF,
    pub lpCustColors: *mut COLORREF,
    pub Flags: DWORD,
    pub lCustData: LPARAM,
    pub lpfnHook: *const c_void,
    pub lpTemplateName: PCWSTR,
}

#[repr(C)]
pub struct CHOOSEFONTW {
    pub lStructSize: DWORD,
    pub hwndOwner: HWND,
    pub hDC: HDC,
    pub lpLogFont: *mut LOGFONTW,
    pub iPointSize: i32,
    pub Flags: DWORD,
    pub rgbColors: COLORREF,
    pub lCustData: LPARAM,
    pub lpfnHook: *const c_void,
    pub lpTemplateName: PCWSTR,
    pub hInstance: HINSTANCE,
    pub lpszStyle: PWSTR,
    pub nFontType: u16,
    pub missing_alignment: u16,
    pub nSizeMin: i32,
    pub nSizeMax: i32,
}

/// `PRINTDLGW`: 1-byte packing on 32-bit, natural alignment on 64-bit.
#[cfg_attr(target_pointer_width = "32", repr(C, packed(1)))]
#[cfg_attr(target_pointer_width = "64", repr(C))]
pub struct PRINTDLGW {
    pub lStructSize: DWORD,
    pub hwndOwner: HWND,
    pub hDevMode: HGLOBAL,
    pub hDevNames: HGLOBAL,
    pub hDC: HDC,
    pub Flags: DWORD,
    pub nFromPage: u16,
    pub nToPage: u16,
    pub nMinPage: u16,
    pub nMaxPage: u16,
    pub nCopies: u16,
    pub hInstance: HINSTANCE,
    pub lCustData: LPARAM,
    pub lpfnPrintHook: *const c_void,
    pub lpfnSetupHook: *const c_void,
    pub lpPrintTemplateName: PCWSTR,
    pub lpSetupTemplateName: PCWSTR,
    pub hPrintTemplate: HGLOBAL,
    pub hSetupTemplate: HGLOBAL,
}

#[repr(C)]
pub struct PAGESETUPDLGW {
    pub lStructSize: DWORD,
    pub hwndOwner: HWND,
    pub hDevMode: HGLOBAL,
    pub hDevNames: HGLOBAL,
    pub Flags: DWORD,
    pub ptPaperSize: POINT,
    pub rtMinMargin: RECT,
    pub rtMargin: RECT,
    pub hInstance: HINSTANCE,
    pub lCustData: LPARAM,
    pub lpfnPageSetupHook: *const c_void,
    pub lpfnPagePaintHook: *const c_void,
    pub lpPageSetupTemplateName: PCWSTR,
    pub hPageSetupTemplate: HGLOBAL,
}

#[repr(C)]
pub struct FINDREPLACEW {
    pub lStructSize: DWORD,
    pub hwndOwner: HWND,
    pub hInstance: HINSTANCE,
    pub Flags: DWORD,
    pub lpstrFindWhat: PWSTR,
    pub lpstrReplaceWith: PWSTR,
    pub wFindWhatLen: u16,
    pub wReplaceWithLen: u16,
    pub lCustData: LPARAM,
    pub lpfnHook: *const c_void,
    pub lpTemplateName: PCWSTR,
}

#[link(name = "comdlg32")]
extern "system" {
    pub fn ChooseColorW(cc: *mut CHOOSECOLORW) -> BOOL;
    pub fn ChooseFontW(cf: *mut CHOOSEFONTW) -> BOOL;
    pub fn PrintDlgW(pd: *mut PRINTDLGW) -> BOOL;
    pub fn PageSetupDlgW(psd: *mut PAGESETUPDLGW) -> BOOL;
    pub fn FindTextW(fr: *mut FINDREPLACEW) -> HWND;
    pub fn ReplaceTextW(fr: *mut FINDREPLACEW) -> HWND;
    pub fn CommDlgExtendedError() -> DWORD;
}
