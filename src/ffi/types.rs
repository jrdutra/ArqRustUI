//! Fundamental types shared by Windows DLLs.

use std::ffi::c_void;

pub type HANDLE = *mut c_void;
pub type HWND = *mut c_void;
pub type HINSTANCE = *mut c_void;
pub type HMODULE = *mut c_void;
pub type HMENU = *mut c_void;
pub type HDC = *mut c_void;
pub type HICON = *mut c_void;
pub type HCURSOR = *mut c_void;
pub type HBRUSH = *mut c_void;
pub type HFONT = *mut c_void;
pub type HBITMAP = *mut c_void;
pub type HPEN = *mut c_void;
pub type HRGN = *mut c_void;
pub type HGDIOBJ = *mut c_void;
pub type HMONITOR = *mut c_void;
pub type HIMAGELIST = *mut c_void;
pub type HKEY = *mut c_void;
pub type HGLOBAL = *mut c_void;
pub type HDROP = *mut c_void;
pub type HIMC = *mut c_void;
pub type HTREEITEM = *mut c_void;

pub type BOOL = i32;
pub type UINT = u32;
pub type DWORD = u32;
pub type WORD = u16;
pub type ATOM = u16;
pub type WPARAM = usize;
pub type LPARAM = isize;
pub type LRESULT = isize;
pub type COLORREF = u32;
pub type HRESULT = i32;
pub type PCWSTR = *const u16;
pub type PWSTR = *mut u16;

pub const TRUE: BOOL = 1;
pub const FALSE: BOOL = 0;
pub const S_OK: HRESULT = 0;
pub const S_FALSE: HRESULT = 1;
pub const E_NOINTERFACE: HRESULT = 0x8000_4002_u32 as i32;
pub const E_INVALIDARG: HRESULT = 0x8007_0057_u32 as i32;
pub const E_POINTER: HRESULT = 0x8000_4003_u32 as i32;
pub const E_OUTOFMEMORY: HRESULT = 0x8007_000E_u32 as i32;

/// Window procedure signature.
pub type WNDPROC = unsafe extern "system" fn(HWND, UINT, WPARAM, LPARAM) -> LRESULT;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct POINT {
    pub x: i32,
    pub y: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SIZE {
    pub cx: i32,
    pub cy: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RECT {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GUID {
    pub data1: u32,
    pub data2: u16,
    pub data3: u16,
    pub data4: [u8; 8],
}

impl GUID {
    /// Builds a GUID from four components.
    pub const fn new(data1: u32, data2: u16, data3: u16, data4: [u8; 8]) -> Self {
        Self { data1, data2, data3, data4 }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SYSTEMTIME {
    pub wYear: u16,
    pub wMonth: u16,
    pub wDayOfWeek: u16,
    pub wDay: u16,
    pub wHour: u16,
    pub wMinute: u16,
    pub wSecond: u16,
    pub wMilliseconds: u16,
}

/// Low 16-bit word.
#[inline]
pub const fn loword(v: usize) -> u16 {
    (v & 0xFFFF) as u16
}

/// High 16-bit word of the lower 32 bits.
#[inline]
pub const fn hiword(v: usize) -> u16 {
    ((v >> 16) & 0xFFFF) as u16
}

/// Signed X coordinate in `LPARAM`.
#[inline]
pub const fn x_lparam(lp: LPARAM) -> i32 {
    (lp as usize & 0xFFFF) as u16 as i16 as i32
}

/// Signed Y coordinate in `LPARAM`.
#[inline]
pub const fn y_lparam(lp: LPARAM) -> i32 {
    ((lp as usize >> 16) & 0xFFFF) as u16 as i16 as i32
}

/// Combines two 16-bit words into `LPARAM`.
#[inline]
pub const fn make_lparam(lo: u16, hi: u16) -> LPARAM {
    (((hi as u32) << 16) | lo as u32) as LPARAM
}

/// `SUCCEEDED(hr)`.
#[inline]
pub const fn succeeded(hr: HRESULT) -> bool {
    hr >= 0
}

/// Builds a `COLORREF` (0x00BBGGRR).
#[inline]
pub const fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    (r as u32) | ((g as u32) << 8) | ((b as u32) << 16)
}
