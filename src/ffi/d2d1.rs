//! Declarations for `d2d1.dll` (Direct2D). COM methods use vtable indices documented in `graphics::direct2d`.

use super::types::*;
use std::ffi::c_void;

pub const IID_ID2D1Factory: GUID =
    GUID::new(0x06152247, 0x6F50, 0x465A, [0x92, 0x45, 0x11, 0x8B, 0xFD, 0x3B, 0x60, 0x07]);
pub const D2D1_FACTORY_TYPE_SINGLE_THREADED: u32 = 0;
pub const D2DERR_RECREATE_TARGET: HRESULT = 0x8899_000C_u32 as i32;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct D2D1_COLOR_F {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct D2D1_POINT_2F {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct D2D1_RECT_F {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct D2D1_ROUNDED_RECT {
    pub rect: D2D1_RECT_F,
    pub radiusX: f32,
    pub radiusY: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct D2D1_ELLIPSE {
    pub point: D2D1_POINT_2F,
    pub radiusX: f32,
    pub radiusY: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct D2D1_SIZE_U {
    pub width: u32,
    pub height: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct D2D1_PIXEL_FORMAT {
    pub format: u32,
    pub alphaMode: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct D2D1_RENDER_TARGET_PROPERTIES {
    pub r#type: u32,
    pub pixelFormat: D2D1_PIXEL_FORMAT,
    pub dpiX: f32,
    pub dpiY: f32,
    pub usage: u32,
    pub minLevel: u32,
}

#[repr(C)]
pub struct D2D1_HWND_RENDER_TARGET_PROPERTIES {
    pub hwnd: HWND,
    pub pixelSize: D2D1_SIZE_U,
    pub presentOptions: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct D2D1_GRADIENT_STOP {
    pub position: f32,
    pub color: D2D1_COLOR_F,
}

#[repr(C)]
pub struct D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES {
    pub startPoint: D2D1_POINT_2F,
    pub endPoint: D2D1_POINT_2F,
}

#[repr(C)]
pub struct D2D1_BEZIER_SEGMENT {
    pub point1: D2D1_POINT_2F,
    pub point2: D2D1_POINT_2F,
    pub point3: D2D1_POINT_2F,
}

/// Affine 3x2 matrix (`D2D1_MATRIX_3X2_F`).
pub type D2D1_MATRIX_3X2_F = [f32; 6];

#[link(name = "d2d1")]
extern "system" {
    pub fn D2D1CreateFactory(
        factory_type: u32,
        iid: *const GUID,
        options: *const c_void,
        factory: *mut *mut c_void,
    ) -> HRESULT;
}
