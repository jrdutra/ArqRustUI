//! Declarations for `gdi32.dll` and `msimg32.dll`.

use super::types::*;
use std::ffi::c_void;

pub const LF_FACESIZE: usize = 32;
pub const FW_NORMAL: i32 = 400;
pub const FW_BOLD: i32 = 700;
pub const DEFAULT_CHARSET: u8 = 1;
pub const CLEARTYPE_QUALITY: u8 = 5;
pub const TRANSPARENT: i32 = 1;
pub const OPAQUE: i32 = 2;
pub const PS_SOLID: i32 = 0;
pub const PS_DASH: i32 = 1;
pub const PS_DOT: i32 = 2;
pub const PS_NULL: i32 = 5;
pub const NULL_BRUSH: i32 = 5;
pub const NULL_PEN: i32 = 8;
pub const DEFAULT_GUI_FONT: i32 = 17;
pub const SRCCOPY: DWORD = 0x00CC_0020;
pub const RGN_OR: i32 = 2;
pub const LOGPIXELSX: i32 = 88;
pub const LOGPIXELSY: i32 = 90;
pub const HORZRES: i32 = 8;
pub const VERTRES: i32 = 10;
pub const GM_ADVANCED: i32 = 2;
pub const MWT_IDENTITY: DWORD = 1;
pub const MWT_LEFTMULTIPLY: DWORD = 2;
pub const MM_TEXT: i32 = 1;
pub const MM_ANISOTROPIC: i32 = 8;
pub const RGN_AND: i32 = 1;
pub const RGN_COPY: i32 = 5;
pub const GRADIENT_FILL_RECT_H: DWORD = 0;
pub const GRADIENT_FILL_RECT_V: DWORD = 1;
pub const AC_SRC_OVER: u8 = 0;
pub const AC_SRC_ALPHA: u8 = 1;
pub const FR_PRIVATE: DWORD = 0x10;
pub const HALFTONE: i32 = 4;
pub const ALTERNATE: i32 = 1;
pub const WINDING: i32 = 2;
pub const BI_RGB: DWORD = 0;
pub const DIB_RGB_COLORS: UINT = 0;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LOGFONTW {
    pub lfHeight: i32,
    pub lfWidth: i32,
    pub lfEscapement: i32,
    pub lfOrientation: i32,
    pub lfWeight: i32,
    pub lfItalic: u8,
    pub lfUnderline: u8,
    pub lfStrikeOut: u8,
    pub lfCharSet: u8,
    pub lfOutPrecision: u8,
    pub lfClipPrecision: u8,
    pub lfQuality: u8,
    pub lfPitchAndFamily: u8,
    pub lfFaceName: [u16; LF_FACESIZE],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct BITMAP {
    pub bmType: i32,
    pub bmWidth: i32,
    pub bmHeight: i32,
    pub bmWidthBytes: i32,
    pub bmPlanes: u16,
    pub bmBitsPixel: u16,
    pub bmBits: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct BITMAPINFOHEADER {
    pub biSize: DWORD,
    pub biWidth: i32,
    pub biHeight: i32,
    pub biPlanes: u16,
    pub biBitCount: u16,
    pub biCompression: DWORD,
    pub biSizeImage: DWORD,
    pub biXPelsPerMeter: i32,
    pub biYPelsPerMeter: i32,
    pub biClrUsed: DWORD,
    pub biClrImportant: DWORD,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct XFORM {
    pub eM11: f32,
    pub eM12: f32,
    pub eM21: f32,
    pub eM22: f32,
    pub eDx: f32,
    pub eDy: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TRIVERTEX {
    pub x: i32,
    pub y: i32,
    pub Red: u16,
    pub Green: u16,
    pub Blue: u16,
    pub Alpha: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GRADIENT_RECT {
    pub UpperLeft: u32,
    pub LowerRight: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct BLENDFUNCTION {
    pub BlendOp: u8,
    pub BlendFlags: u8,
    pub SourceConstantAlpha: u8,
    pub AlphaFormat: u8,
}

#[repr(C)]
pub struct DOCINFOW {
    pub cbSize: i32,
    pub lpszDocName: PCWSTR,
    pub lpszOutput: PCWSTR,
    pub lpszDatatype: PCWSTR,
    pub fwType: DWORD,
}

#[link(name = "gdi32")]
extern "system" {
    pub fn CreateFontIndirectW(font: *const LOGFONTW) -> HFONT;
    pub fn DeleteObject(object: HGDIOBJ) -> BOOL;
    pub fn GdiFlush() -> BOOL;
    pub fn CreateBitmap(width: i32, height: i32, planes: UINT, bit_count: UINT, bits: *const c_void) -> HBITMAP;
    pub fn GetDIBits(
        hdc: HDC,
        bitmap: HBITMAP,
        start: UINT,
        lines: UINT,
        bits: *mut c_void,
        info: *mut BITMAPINFOHEADER,
        usage: UINT,
    ) -> i32;
    pub fn SelectObject(hdc: HDC, object: HGDIOBJ) -> HGDIOBJ;
    pub fn GetStockObject(index: i32) -> HGDIOBJ;
    pub fn GetObjectW(object: HANDLE, size: i32, out: *mut c_void) -> i32;
    pub fn CreateSolidBrush(color: COLORREF) -> HBRUSH;
    pub fn CreatePen(style: i32, width: i32, color: COLORREF) -> HPEN;
    pub fn SetTextColor(hdc: HDC, color: COLORREF) -> COLORREF;
    pub fn SetBkColor(hdc: HDC, color: COLORREF) -> COLORREF;
    pub fn SetBkMode(hdc: HDC, mode: i32) -> i32;
    pub fn MoveToEx(hdc: HDC, x: i32, y: i32, previous: *mut POINT) -> BOOL;
    pub fn LineTo(hdc: HDC, x: i32, y: i32) -> BOOL;
    pub fn Rectangle(hdc: HDC, left: i32, top: i32, right: i32, bottom: i32) -> BOOL;
    pub fn Ellipse(hdc: HDC, left: i32, top: i32, right: i32, bottom: i32) -> BOOL;
    pub fn RoundRect(hdc: HDC, left: i32, top: i32, right: i32, bottom: i32, w: i32, h: i32) -> BOOL;
    pub fn Polygon(hdc: HDC, points: *const POINT, count: i32) -> BOOL;
    pub fn Polyline(hdc: HDC, points: *const POINT, count: i32) -> BOOL;
    pub fn PolyBezierTo(hdc: HDC, points: *const POINT, count: DWORD) -> BOOL;
    pub fn TextOutW(hdc: HDC, x: i32, y: i32, text: PCWSTR, count: i32) -> BOOL;
    pub fn GetTextExtentPoint32W(hdc: HDC, text: PCWSTR, count: i32, size: *mut SIZE) -> BOOL;
    pub fn CreateCompatibleDC(hdc: HDC) -> HDC;
    pub fn CreateCompatibleBitmap(hdc: HDC, width: i32, height: i32) -> HBITMAP;
    pub fn CreateDIBSection(
        hdc: HDC,
        info: *const BITMAPINFOHEADER,
        usage: UINT,
        bits: *mut *mut c_void,
        section: HANDLE,
        offset: DWORD,
    ) -> HBITMAP;
    pub fn DeleteDC(hdc: HDC) -> BOOL;
    pub fn StretchDIBits(
        hdc: HDC,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        src_x: i32,
        src_y: i32,
        src_width: i32,
        src_height: i32,
        bits: *const c_void,
        info: *const BITMAPINFOHEADER,
        usage: UINT,
        rop: DWORD,
    ) -> i32;
    pub fn SetEnhMetaFileBits(size: u32, bytes: *const u8) -> HANDLE;
    pub fn GetEnhMetaFileBits(metafile: HANDLE, size: u32, bytes: *mut u8) -> u32;
    pub fn DeleteEnhMetaFile(metafile: HANDLE) -> BOOL;
    pub fn CreateEnhMetaFileW(reference: HDC, path: PCWSTR, frame: *const RECT, description: PCWSTR) -> HDC;
    pub fn CloseEnhMetaFile(hdc: HDC) -> HANDLE;
    pub fn PlayEnhMetaFile(hdc: HDC, metafile: HANDLE, rect: *const RECT) -> BOOL;
    pub fn BitBlt(
        dest: HDC,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        src: HDC,
        src_x: i32,
        src_y: i32,
        rop: DWORD,
    ) -> BOOL;
    pub fn StretchBlt(
        dest: HDC,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        src: HDC,
        src_x: i32,
        src_y: i32,
        src_width: i32,
        src_height: i32,
        rop: DWORD,
    ) -> BOOL;
    pub fn SetStretchBltMode(hdc: HDC, mode: i32) -> i32;
    pub fn SaveDC(hdc: HDC) -> i32;
    pub fn RestoreDC(hdc: HDC, saved: i32) -> BOOL;
    pub fn BeginPath(hdc: HDC) -> BOOL;
    pub fn EndPath(hdc: HDC) -> BOOL;
    pub fn CloseFigure(hdc: HDC) -> BOOL;
    pub fn StrokePath(hdc: HDC) -> BOOL;
    pub fn FillPath(hdc: HDC) -> BOOL;
    pub fn StrokeAndFillPath(hdc: HDC) -> BOOL;
    pub fn SelectClipPath(hdc: HDC, mode: i32) -> BOOL;
    pub fn SetPolyFillMode(hdc: HDC, mode: i32) -> i32;
    pub fn CreateRectRgn(left: i32, top: i32, right: i32, bottom: i32) -> HRGN;
    pub fn CombineRgn(dest: HRGN, a: HRGN, b: HRGN, mode: i32) -> i32;
    pub fn CreateEllipticRgn(left: i32, top: i32, right: i32, bottom: i32) -> HRGN;
    pub fn CreateRoundRectRgn(left: i32, top: i32, right: i32, bottom: i32, w: i32, h: i32) -> HRGN;
    pub fn ExtSelectClipRgn(hdc: HDC, region: HRGN, mode: i32) -> i32;
    pub fn SelectClipRgn(hdc: HDC, region: HRGN) -> i32;
    pub fn IntersectClipRect(hdc: HDC, left: i32, top: i32, right: i32, bottom: i32) -> i32;
    pub fn SetGraphicsMode(hdc: HDC, mode: i32) -> i32;
    pub fn SetWorldTransform(hdc: HDC, xform: *const XFORM) -> BOOL;
    pub fn ModifyWorldTransform(hdc: HDC, xform: *const XFORM, mode: DWORD) -> BOOL;
    pub fn SetMapMode(hdc: HDC, mode: i32) -> i32;
    pub fn SetWindowExtEx(hdc: HDC, x: i32, y: i32, previous: *mut SIZE) -> BOOL;
    pub fn SetViewportExtEx(hdc: HDC, x: i32, y: i32, previous: *mut SIZE) -> BOOL;
    pub fn GetDeviceCaps(hdc: HDC, index: i32) -> i32;
    pub fn SetPixel(hdc: HDC, x: i32, y: i32, color: COLORREF) -> COLORREF;
    pub fn GetPixel(hdc: HDC, x: i32, y: i32) -> COLORREF;
    pub fn StartDocW(hdc: HDC, info: *const DOCINFOW) -> i32;
    pub fn StartPage(hdc: HDC) -> i32;
    pub fn EndPage(hdc: HDC) -> i32;
    pub fn EndDoc(hdc: HDC) -> i32;
    pub fn AddFontResourceExW(path: PCWSTR, flags: DWORD, reserved: *mut c_void) -> i32;
    pub fn RemoveFontResourceExW(path: PCWSTR, flags: DWORD, reserved: *mut c_void) -> BOOL;
}

#[link(name = "msimg32")]
extern "system" {
    pub fn GradientFill(
        hdc: HDC,
        vertices: *const TRIVERTEX,
        vertex_count: u32,
        mesh: *const c_void,
        mesh_count: u32,
        mode: DWORD,
    ) -> BOOL;
    pub fn AlphaBlend(
        dest: HDC,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        src: HDC,
        src_x: i32,
        src_y: i32,
        src_width: i32,
        src_height: i32,
        blend: BLENDFUNCTION,
    ) -> BOOL;
}
