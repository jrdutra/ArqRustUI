//! Declarations for the `gdiplus.dll` flat API (GDI+).

use super::types::*;
use std::ffi::c_void;

pub type GpStatus = i32;
pub const SMOOTHING_MODE_ANTIALIAS: i32 = 4;
pub const TEXT_RENDERING_HINT_CLEARTYPE: i32 = 5;
pub const UNIT_PIXEL: i32 = 2;
pub const UNIT_POINT: i32 = 3;
pub const FILL_MODE_ALTERNATE: i32 = 0;
pub const FILL_MODE_WINDING: i32 = 1;
pub const WRAP_MODE_TILE: i32 = 0;
pub const WRAP_MODE_TILE_FLIP_XY: i32 = 3;
pub const WRAP_MODE_CLAMP: i32 = 4;
pub const MATRIX_ORDER_APPEND: i32 = 1;
pub const COMBINE_MODE_EXCLUDE: i32 = 4;
pub const LINE_JOIN_MITER: i32 = 0;
pub const LINE_JOIN_BEVEL: i32 = 1;
pub const LINE_JOIN_ROUND: i32 = 2;
pub const LINE_CAP_FLAT: i32 = 0;
pub const LINE_CAP_SQUARE: i32 = 1;
pub const LINE_CAP_ROUND: i32 = 2;
pub const MATRIX_ORDER_PREPEND: i32 = 0;
pub const COMBINE_MODE_INTERSECT: i32 = 1;
pub const PIXEL_FORMAT_32BPP_ARGB: i32 = 0x0026_200A;
pub const INTERPOLATION_MODE_HIGH_QUALITY_BICUBIC: i32 = 7;
pub const PIXEL_OFFSET_MODE_HIGH_QUALITY: i32 = 2;

#[repr(C)]
pub struct GdiplusStartupInput {
    pub GdiplusVersion: u32,
    pub DebugEventCallback: *const c_void,
    pub SuppressBackgroundThread: BOOL,
    pub SuppressExternalCodecs: BOOL,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PointF {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RectF {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

type P = *mut c_void;

/// GDI+ integer rectangle (`GpRect`).
#[repr(C)]
pub struct GpRect {
    pub X: i32,
    pub Y: i32,
    pub Width: i32,
    pub Height: i32,
}

/// Locked pixel description (`BitmapData`).
#[repr(C)]
pub struct BitmapData {
    pub Width: u32,
    pub Height: u32,
    pub Stride: i32,
    pub PixelFormat: i32,
    pub Scan0: *mut u8,
    pub Reserved: usize,
}

pub const IMAGE_LOCK_MODE_READ: u32 = 1;
pub const IMAGE_LOCK_MODE_WRITE: u32 = 2;
pub const IMAGE_LOCK_MODE_USER_INPUT_BUF: u32 = 4;

#[link(name = "gdiplus")]
extern "system" {
    pub fn GdiplusStartup(token: *mut usize, input: *const GdiplusStartupInput, output: *mut c_void) -> GpStatus;
    pub fn GdiplusShutdown(token: usize);
    pub fn GdipCreateFromHDC(hdc: HDC, graphics: *mut P) -> GpStatus;
    pub fn GdipDeleteGraphics(graphics: P) -> GpStatus;
    pub fn GdipSetSmoothingMode(graphics: P, mode: i32) -> GpStatus;
    pub fn GdipSetTextRenderingHint(graphics: P, hint: i32) -> GpStatus;
    pub fn GdipGraphicsClear(graphics: P, argb: u32) -> GpStatus;
    pub fn GdipCreatePen1(argb: u32, width: f32, unit: i32, pen: *mut P) -> GpStatus;
    pub fn GdipDeletePen(pen: P) -> GpStatus;
    pub fn GdipCreateSolidFill(argb: u32, brush: *mut P) -> GpStatus;
    pub fn GdipCreateLineBrush(
        p1: *const PointF,
        p2: *const PointF,
        c1: u32,
        c2: u32,
        wrap: i32,
        brush: *mut P,
    ) -> GpStatus;
    pub fn GdipDeleteBrush(brush: P) -> GpStatus;
    pub fn GdipDrawLine(graphics: P, pen: P, x1: f32, y1: f32, x2: f32, y2: f32) -> GpStatus;
    pub fn GdipDrawRectangle(graphics: P, pen: P, x: f32, y: f32, w: f32, h: f32) -> GpStatus;
    pub fn GdipFillRectangle(graphics: P, brush: P, x: f32, y: f32, w: f32, h: f32) -> GpStatus;
    pub fn GdipDrawEllipse(graphics: P, pen: P, x: f32, y: f32, w: f32, h: f32) -> GpStatus;
    pub fn GdipFillEllipse(graphics: P, brush: P, x: f32, y: f32, w: f32, h: f32) -> GpStatus;
    pub fn GdipDrawPolygon(graphics: P, pen: P, points: *const PointF, count: i32) -> GpStatus;
    pub fn GdipFillPolygon(graphics: P, brush: P, points: *const PointF, count: i32, mode: i32) -> GpStatus;
    pub fn GdipCreatePath(mode: i32, path: *mut P) -> GpStatus;
    pub fn GdipDeletePath(path: P) -> GpStatus;
    pub fn GdipAddPathLine(path: P, x1: f32, y1: f32, x2: f32, y2: f32) -> GpStatus;
    pub fn GdipAddPathBezier(
        path: P,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        x3: f32,
        y3: f32,
        x4: f32,
        y4: f32,
    ) -> GpStatus;
    pub fn GdipAddPathArc(path: P, x: f32, y: f32, w: f32, h: f32, start: f32, sweep: f32) -> GpStatus;
    pub fn GdipAddPathEllipse(path: P, x: f32, y: f32, w: f32, h: f32) -> GpStatus;
    pub fn GdipClosePathFigure(path: P) -> GpStatus;
    pub fn GdipDrawPath(graphics: P, pen: P, path: P) -> GpStatus;
    pub fn GdipFillPath(graphics: P, brush: P, path: P) -> GpStatus;
    pub fn GdipLoadImageFromFile(path: PCWSTR, image: *mut P) -> GpStatus;
    pub fn GdipCreateBitmapFromFile(path: PCWSTR, bitmap: *mut P) -> GpStatus;
    pub fn GdipCreateHBITMAPFromBitmap(bitmap: P, hbitmap: *mut HBITMAP, background: u32) -> GpStatus;
    pub fn GdipDisposeImage(image: P) -> GpStatus;
    pub fn GdipCreateBitmapFromStream(stream: P, bitmap: *mut P) -> GpStatus;
    pub fn GdipSaveImageToStream(image: P, stream: P, encoder: *const GUID, parameters: *const c_void) -> GpStatus;
    pub fn GdipSaveImageToFile(image: P, path: PCWSTR, encoder: *const GUID, parameters: *const c_void) -> GpStatus;
    pub fn GdipCreateBitmapFromScan0(
        width: i32,
        height: i32,
        stride: i32,
        format: i32,
        scan0: *mut u8,
        bitmap: *mut P,
    ) -> GpStatus;
    pub fn GdipGetImageGraphicsContext(image: P, graphics: *mut P) -> GpStatus;
    pub fn GdipBitmapLockBits(
        bitmap: P,
        rect: *const GpRect,
        flags: u32,
        format: i32,
        data: *mut BitmapData,
    ) -> GpStatus;
    pub fn GdipBitmapUnlockBits(bitmap: P, data: *mut BitmapData) -> GpStatus;
    pub fn GdipSetInterpolationMode(graphics: P, mode: i32) -> GpStatus;
    pub fn GdipSetPixelOffsetMode(graphics: P, mode: i32) -> GpStatus;
    pub fn GdipCreateHICONFromBitmap(bitmap: P, icon: *mut HICON) -> GpStatus;
    pub fn GdipGetImageWidth(image: P, width: *mut u32) -> GpStatus;
    pub fn GdipGetImageHeight(image: P, height: *mut u32) -> GpStatus;
    pub fn GdipDrawImageRect(graphics: P, image: P, x: f32, y: f32, w: f32, h: f32) -> GpStatus;
    pub fn GdipCreateFontFamilyFromName(name: PCWSTR, collection: P, family: *mut P) -> GpStatus;
    pub fn GdipDeleteFontFamily(family: P) -> GpStatus;
    pub fn GdipCreateFont(family: P, size: f32, style: i32, unit: i32, font: *mut P) -> GpStatus;
    pub fn GdipDeleteFont(font: P) -> GpStatus;
    pub fn GdipDrawString(
        graphics: P,
        text: PCWSTR,
        length: i32,
        font: P,
        layout: *const RectF,
        format: P,
        brush: P,
    ) -> GpStatus;
    pub fn GdipTranslateWorldTransform(graphics: P, dx: f32, dy: f32, order: i32) -> GpStatus;
    pub fn GdipRotateWorldTransform(graphics: P, angle: f32, order: i32) -> GpStatus;
    pub fn GdipScaleWorldTransform(graphics: P, sx: f32, sy: f32, order: i32) -> GpStatus;
    pub fn GdipResetWorldTransform(graphics: P) -> GpStatus;
    pub fn GdipSetClipRect(graphics: P, x: f32, y: f32, w: f32, h: f32, mode: i32) -> GpStatus;
    pub fn GdipResetClip(graphics: P) -> GpStatus;
    pub fn GdipStartPathFigure(path: P) -> GpStatus;
    pub fn GdipTransformPath(path: P, matrix: P) -> GpStatus;
    pub fn GdipCreateMatrix2(m11: f32, m12: f32, m21: f32, m22: f32, dx: f32, dy: f32, matrix: *mut P) -> GpStatus;
    pub fn GdipDeleteMatrix(matrix: P) -> GpStatus;
    pub fn GdipMultiplyWorldTransform(graphics: P, matrix: P, order: i32) -> GpStatus;
    pub fn GdipSaveGraphics(graphics: P, state: *mut u32) -> GpStatus;
    pub fn GdipRestoreGraphics(graphics: P, state: u32) -> GpStatus;
    pub fn GdipSetClipPath(graphics: P, path: P, mode: i32) -> GpStatus;
    pub fn GdipCreatePen2(brush: P, width: f32, unit: i32, pen: *mut P) -> GpStatus;
    pub fn GdipSetPenLineJoin(pen: P, join: i32) -> GpStatus;
    pub fn GdipSetPenStartCap(pen: P, cap: i32) -> GpStatus;
    pub fn GdipSetPenEndCap(pen: P, cap: i32) -> GpStatus;
    pub fn GdipSetPenMiterLimit(pen: P, limit: f32) -> GpStatus;
    pub fn GdipSetPenDashArray(pen: P, dashes: *const f32, count: i32) -> GpStatus;
    pub fn GdipSetLinePresetBlend(brush: P, colors: *const u32, positions: *const f32, count: i32) -> GpStatus;
    pub fn GdipMultiplyLineTransform(brush: P, matrix: P, order: i32) -> GpStatus;
    pub fn GdipCreatePathGradientFromPath(path: P, brush: *mut P) -> GpStatus;
    pub fn GdipSetPathGradientPresetBlend(brush: P, colors: *const u32, positions: *const f32, count: i32) -> GpStatus;
    pub fn GdipSetPathGradientCenterPoint(brush: P, point: *const PointF) -> GpStatus;
    pub fn GdipSetPathGradientCenterColor(brush: P, argb: u32) -> GpStatus;
    pub fn GdipSetPathGradientSurroundColorsWithCount(brush: P, colors: *const u32, count: *mut i32) -> GpStatus;
}
