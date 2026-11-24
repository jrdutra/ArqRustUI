//! Direct2D + DirectWrite backend: hardware-accelerated vector rendering.
//!
//! COM calls use vtable indices following declaration order in `d2d1.h`/`dwrite.h`; first three entries are
//! `QueryInterface`, `AddRef`, `Release`.

use super::font::Font;
use super::gdi::{Path, PathSegment};
use crate::core::native;
use crate::core::widget::Widget;
use crate::core::{GuiError, GuiResult};
use crate::ffi::d2d1::*;
use crate::ffi::dwrite::*;
use crate::ffi::ole32::ComPtr;
use crate::ffi::user32::{ValidateRect, WM_ERASEBKGND, WM_PAINT, WM_SIZE};
use crate::ffi::{HRESULT, HWND};
use crate::utilities::{to_wide, Color, Rect};
use std::cell::RefCell;
use std::ffi::c_void;
use std::rc::Rc;

type Raw = *mut c_void;

// ID2D1Factory
const FACTORY_CREATE_PATH_GEOMETRY: usize = 10;
const FACTORY_CREATE_HWND_RENDER_TARGET: usize = 14;
// ID2D1RenderTarget
const RT_CREATE_SOLID_COLOR_BRUSH: usize = 8;
const RT_CREATE_GRADIENT_STOP_COLLECTION: usize = 9;
const RT_CREATE_LINEAR_GRADIENT_BRUSH: usize = 10;
const RT_DRAW_LINE: usize = 15;
const RT_DRAW_RECTANGLE: usize = 16;
const RT_FILL_RECTANGLE: usize = 17;
const RT_DRAW_ROUNDED_RECTANGLE: usize = 18;
const RT_FILL_ROUNDED_RECTANGLE: usize = 19;
const RT_DRAW_ELLIPSE: usize = 20;
const RT_FILL_ELLIPSE: usize = 21;
const RT_DRAW_GEOMETRY: usize = 22;
const RT_FILL_GEOMETRY: usize = 23;
const RT_DRAW_TEXT: usize = 27;
const RT_SET_TRANSFORM: usize = 30;
const RT_SET_ANTIALIAS_MODE: usize = 32;
const RT_PUSH_AXIS_ALIGNED_CLIP: usize = 45;
const RT_POP_AXIS_ALIGNED_CLIP: usize = 46;
const RT_CLEAR: usize = 47;
const RT_BEGIN_DRAW: usize = 48;
const RT_END_DRAW: usize = 49;
const RT_SET_DPI: usize = 51;
// ID2D1HwndRenderTarget
const HWND_RT_RESIZE: usize = 58;
// ID2D1PathGeometry / ID2D1GeometrySink
const PATH_OPEN: usize = 17;
const SINK_BEGIN_FIGURE: usize = 5;
const SINK_END_FIGURE: usize = 8;
const SINK_CLOSE: usize = 9;
const SINK_ADD_LINE: usize = 10;
const SINK_ADD_BEZIER: usize = 11;
// IDWriteFactory
const DWRITE_CREATE_TEXT_FORMAT: usize = 15;

fn color_f(color: Color) -> D2D1_COLOR_F {
    D2D1_COLOR_F {
        r: color.r as f32 / 255.0,
        g: color.g as f32 / 255.0,
        b: color.b as f32 / 255.0,
        a: color.a as f32 / 255.0,
    }
}

fn rect_f(rect: Rect) -> D2D1_RECT_F {
    D2D1_RECT_F { left: rect.x as f32, top: rect.y as f32, right: rect.right() as f32, bottom: rect.bottom() as f32 }
}

const IDENTITY: D2D1_MATRIX_3X2_F = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

fn multiply(a: &D2D1_MATRIX_3X2_F, b: &D2D1_MATRIX_3X2_F) -> D2D1_MATRIX_3X2_F {
    [
        a[0] * b[0] + a[1] * b[2],
        a[0] * b[1] + a[1] * b[3],
        a[2] * b[0] + a[3] * b[2],
        a[2] * b[1] + a[3] * b[3],
        a[4] * b[0] + a[5] * b[2] + b[4],
        a[4] * b[1] + a[5] * b[3] + b[5],
    ]
}

struct Inner {
    hwnd: HWND,
    factory: ComPtr,
    dwrite: ComPtr,
    target: RefCell<Option<ComPtr>>,
    painter: Rc<dyn Fn(&D2DContext)>,
}

impl Inner {
    fn create_target(&self) -> GuiResult<ComPtr> {
        let rect = native::client_rect(self.hwnd);
        let properties = D2D1_RENDER_TARGET_PROPERTIES::default();
        let hwnd_properties = D2D1_HWND_RENDER_TARGET_PROPERTIES {
            hwnd: self.hwnd,
            pixelSize: D2D1_SIZE_U { width: rect.right.max(1) as u32, height: rect.bottom.max(1) as u32 },
            presentOptions: 0,
        };
        let mut raw: Raw = std::ptr::null_mut();
        // SAFETY: índice 14 de ID2D1Factory é CreateHwndRenderTarget com esta assinatura;
        // as estruturas são locais e completas.
        let hr = unsafe {
            let create: unsafe extern "system" fn(
                Raw,
                *const D2D1_RENDER_TARGET_PROPERTIES,
                *const D2D1_HWND_RENDER_TARGET_PROPERTIES,
                *mut Raw,
            ) -> HRESULT = self.factory.method(FACTORY_CREATE_HWND_RENDER_TARGET);
            create(self.factory.as_ptr(), &properties, &hwnd_properties, &mut raw)
        };
        GuiError::check_hresult("ID2D1Factory::CreateHwndRenderTarget", hr)?;
        // SAFETY: em caso de sucesso `raw` é uma interface com uma referência nossa.
        let target = unsafe { ComPtr::from_raw(raw) }.ok_or(GuiError::Unsupported("Direct2D"))?;
        let dpi = native::dpi_for_window(self.hwnd) as f32;
        // SAFETY: índice 51 de ID2D1RenderTarget é SetDpi(float, float).
        unsafe {
            let set_dpi: unsafe extern "system" fn(Raw, f32, f32) = target.method(RT_SET_DPI);
            set_dpi(target.as_ptr(), dpi, dpi);
        }
        Ok(target)
    }

    fn resize(&self) {
        if let Some(target) = self.target.borrow().as_ref() {
            let rect = native::client_rect(self.hwnd);
            let size = D2D1_SIZE_U { width: rect.right.max(1) as u32, height: rect.bottom.max(1) as u32 };
            // SAFETY: índice 58 de ID2D1HwndRenderTarget é Resize(const D2D1_SIZE_U*).
            unsafe {
                let resize: unsafe extern "system" fn(Raw, *const D2D1_SIZE_U) -> HRESULT =
                    target.method(HWND_RT_RESIZE);
                resize(target.as_ptr(), &size);
            }
        }
    }

    fn render(&self) {
        if self.target.borrow().is_none() {
            match self.create_target() {
                Ok(target) => *self.target.borrow_mut() = Some(target),
                Err(_) => return,
            }
        }
        let Some(target) = self.target.borrow().clone() else { return };
        // SAFETY: índices 48/49 de ID2D1RenderTarget são BeginDraw() e EndDraw(tag1*, tag2*).
        let hr = unsafe {
            let begin: unsafe extern "system" fn(Raw) = target.method(RT_BEGIN_DRAW);
            begin(target.as_ptr());
            let context = D2DContext {
                target: target.clone(),
                factory: self.factory.clone(),
                dwrite: self.dwrite.clone(),
                transform: RefCell::new(vec![IDENTITY]),
            };
            (self.painter)(&context);
            let end: unsafe extern "system" fn(Raw, *mut u64, *mut u64) -> HRESULT = target.method(RT_END_DRAW);
            end(target.as_ptr(), std::ptr::null_mut(), std::ptr::null_mut())
        };
        if hr == D2DERR_RECREATE_TARGET {
            // O dispositivo foi perdido: descarta o alvo e pede um novo quadro, que o recria.
            *self.target.borrow_mut() = None;
            // SAFETY: invalida a janela à qual a superfície está ligada.
            unsafe { crate::ffi::user32::InvalidateRect(self.hwnd, std::ptr::null(), 0) };
        }
    }
}

/// Direct2D surface attached to a window/panel.
///
/// After [`attach`](Self::attach), drawing runs on each `WM_PAINT` with a logical-unit (DIP) [`D2DContext`]. Target
/// resizes/recreates automatically.
#[derive(Clone)]
pub struct Direct2DSurface {
    inner: Rc<Inner>,
}

impl Direct2DSurface {
    /// Attaches Direct2D to a widget, usually [`Panel`](crate::controls::Panel) or [`Window`](crate::core::Window).
    ///
    /// Uses the widget raw message hook; do not combine with [`Widget::on_raw_message`] on the same widget.
    pub fn attach(widget: &impl Widget, painter: impl Fn(&D2DContext) + 'static) -> GuiResult<Direct2DSurface> {
        let hwnd = widget.core().require_alive()?;
        let mut factory: Raw = std::ptr::null_mut();
        // SAFETY: IID e ponteiro de saída válidos; opções nulas são permitidas.
        let hr = unsafe {
            D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, &IID_ID2D1Factory, std::ptr::null(), &mut factory)
        };
        GuiError::check_hresult("D2D1CreateFactory", hr)?;
        // SAFETY: referência recém-criada, agora possuída pelo ComPtr.
        let factory = unsafe { ComPtr::from_raw(factory) }.ok_or(GuiError::Unsupported("Direct2D"))?;
        let mut dwrite: Raw = std::ptr::null_mut();
        // SAFETY: IID e ponteiro de saída válidos.
        let hr = unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED, &IID_IDWriteFactory, &mut dwrite) };
        GuiError::check_hresult("DWriteCreateFactory", hr)?;
        // SAFETY: referência recém-criada, agora possuída pelo ComPtr.
        let dwrite = unsafe { ComPtr::from_raw(dwrite) }.ok_or(GuiError::Unsupported("DirectWrite"))?;
        let inner = Rc::new(Inner { hwnd, factory, dwrite, target: RefCell::new(None), painter: Rc::new(painter) });
        *inner.target.borrow_mut() = Some(inner.create_target()?);
        let hook = inner.clone();
        widget.core().set_raw_hook(move |msg| match msg.message {
            WM_PAINT => {
                hook.render();
                // SAFETY: valida a área cliente da própria janela após desenhar.
                unsafe { ValidateRect(hook.hwnd, std::ptr::null()) };
                Some(0)
            }
            WM_ERASEBKGND => Some(1),
            WM_SIZE => {
                hook.resize();
                None
            }
            _ => None,
        });
        widget.core().invalidate();
        Ok(Direct2DSurface { inner })
    }

    /// Requests a new frame.
    pub fn redraw(&self) {
        // SAFETY: invalida a janela à qual a superfície está ligada.
        unsafe { crate::ffi::user32::InvalidateRect(self.inner.hwnd, std::ptr::null(), 0) };
    }
}

/// Direct2D frame drawing context.
pub struct D2DContext {
    target: ComPtr,
    factory: ComPtr,
    dwrite: ComPtr,
    transform: RefCell<Vec<D2D1_MATRIX_3X2_F>>,
}

impl D2DContext {
    fn brush(&self, color: Color) -> Option<ComPtr> {
        let color = color_f(color);
        let mut raw: Raw = std::ptr::null_mut();
        // SAFETY: índice 8 é CreateSolidColorBrush(color*, props*, brush**).
        unsafe {
            let create: unsafe extern "system" fn(Raw, *const D2D1_COLOR_F, *const c_void, *mut Raw) -> HRESULT =
                self.target.method(RT_CREATE_SOLID_COLOR_BRUSH);
            if create(self.target.as_ptr(), &color, std::ptr::null(), &mut raw) < 0 {
                return None;
            }
            ComPtr::from_raw(raw)
        }
    }

    /// Clears the frame to a color.
    pub fn clear(&self, color: Color) {
        let color = color_f(color);
        // SAFETY: índice 47 é Clear(const D2D1_COLOR_F*).
        unsafe {
            let clear: unsafe extern "system" fn(Raw, *const D2D1_COLOR_F) = self.target.method(RT_CLEAR);
            clear(self.target.as_ptr(), &color);
        }
    }

    /// Toggles shape antialiasing.
    pub fn set_antialias(&self, enabled: bool) {
        // SAFETY: índice 32 é SetAntialiasMode(D2D1_ANTIALIAS_MODE).
        unsafe {
            let set: unsafe extern "system" fn(Raw, u32) = self.target.method(RT_SET_ANTIALIAS_MODE);
            set(self.target.as_ptr(), if enabled { 0 } else { 1 });
        }
    }

    /// Line.
    pub fn line(&self, x1: f32, y1: f32, x2: f32, y2: f32, color: Color, width: f32) {
        let Some(brush) = self.brush(color) else { return };
        // SAFETY: índice 15 é DrawLine(p0, p1, brush, width, strokeStyle).
        unsafe {
            let draw: unsafe extern "system" fn(Raw, D2D1_POINT_2F, D2D1_POINT_2F, Raw, f32, Raw) =
                self.target.method(RT_DRAW_LINE);
            draw(
                self.target.as_ptr(),
                D2D1_POINT_2F { x: x1, y: y1 },
                D2D1_POINT_2F { x: x2, y: y2 },
                brush.as_ptr(),
                width,
                std::ptr::null_mut(),
            );
        }
    }

    /// Filled rectangle.
    pub fn fill_rect(&self, rect: Rect, color: Color) {
        let Some(brush) = self.brush(color) else { return };
        let rect = rect_f(rect);
        // SAFETY: índice 17 é FillRectangle(rect*, brush).
        unsafe {
            let fill: unsafe extern "system" fn(Raw, *const D2D1_RECT_F, Raw) = self.target.method(RT_FILL_RECTANGLE);
            fill(self.target.as_ptr(), &rect, brush.as_ptr());
        }
    }

    /// Rectangle outline.
    pub fn draw_rect(&self, rect: Rect, color: Color, width: f32) {
        let Some(brush) = self.brush(color) else { return };
        let rect = rect_f(rect);
        // SAFETY: índice 16 é DrawRectangle(rect*, brush, width, strokeStyle).
        unsafe {
            let draw: unsafe extern "system" fn(Raw, *const D2D1_RECT_F, Raw, f32, Raw) =
                self.target.method(RT_DRAW_RECTANGLE);
            draw(self.target.as_ptr(), &rect, brush.as_ptr(), width, std::ptr::null_mut());
        }
    }

    /// Rounded rectangle with fill and/or stroke.
    pub fn rounded_rect(&self, rect: Rect, radius: f32, fill: Option<Color>, stroke: Option<(Color, f32)>) {
        let rounded = D2D1_ROUNDED_RECT { rect: rect_f(rect), radiusX: radius, radiusY: radius };
        if let Some(brush) = fill.and_then(|c| self.brush(c)) {
            // SAFETY: índice 19 é FillRoundedRectangle(roundedRect*, brush).
            unsafe {
                let f: unsafe extern "system" fn(Raw, *const D2D1_ROUNDED_RECT, Raw) =
                    self.target.method(RT_FILL_ROUNDED_RECTANGLE);
                f(self.target.as_ptr(), &rounded, brush.as_ptr());
            }
        }
        if let Some((color, width)) = stroke {
            if let Some(brush) = self.brush(color) {
                // SAFETY: índice 18 é DrawRoundedRectangle(roundedRect*, brush, width, style).
                unsafe {
                    let d: unsafe extern "system" fn(Raw, *const D2D1_ROUNDED_RECT, Raw, f32, Raw) =
                        self.target.method(RT_DRAW_ROUNDED_RECTANGLE);
                    d(self.target.as_ptr(), &rounded, brush.as_ptr(), width, std::ptr::null_mut());
                }
            }
        }
    }

    /// Ellipse (center/radii) with fill and/or stroke.
    pub fn ellipse(&self, cx: f32, cy: f32, rx: f32, ry: f32, fill: Option<Color>, stroke: Option<(Color, f32)>) {
        let ellipse = D2D1_ELLIPSE { point: D2D1_POINT_2F { x: cx, y: cy }, radiusX: rx, radiusY: ry };
        if let Some(brush) = fill.and_then(|c| self.brush(c)) {
            // SAFETY: índice 21 é FillEllipse(ellipse*, brush).
            unsafe {
                let f: unsafe extern "system" fn(Raw, *const D2D1_ELLIPSE, Raw) = self.target.method(RT_FILL_ELLIPSE);
                f(self.target.as_ptr(), &ellipse, brush.as_ptr());
            }
        }
        if let Some((color, width)) = stroke {
            if let Some(brush) = self.brush(color) {
                // SAFETY: índice 20 é DrawEllipse(ellipse*, brush, width, style).
                unsafe {
                    let d: unsafe extern "system" fn(Raw, *const D2D1_ELLIPSE, Raw, f32, Raw) =
                        self.target.method(RT_DRAW_ELLIPSE);
                    d(self.target.as_ptr(), &ellipse, brush.as_ptr(), width, std::ptr::null_mut());
                }
            }
        }
    }

    fn geometry(&self, path: &Path) -> Option<ComPtr> {
        let point = |p: crate::utilities::Point| D2D1_POINT_2F { x: p.x as f32, y: p.y as f32 };
        // SAFETY: índices conforme d2d1.h — Factory[10] CreatePathGeometry,
        // PathGeometry[17] Open, GeometrySink[5] BeginFigure, [8] EndFigure, [9] Close,
        // [10] AddLine, [11] AddBezier. Toda figura aberta é encerrada antes de Close.
        unsafe {
            let mut raw: Raw = std::ptr::null_mut();
            let create: unsafe extern "system" fn(Raw, *mut Raw) -> HRESULT =
                self.factory.method(FACTORY_CREATE_PATH_GEOMETRY);
            if create(self.factory.as_ptr(), &mut raw) < 0 {
                return None;
            }
            let geometry = ComPtr::from_raw(raw)?;
            let mut sink_raw: Raw = std::ptr::null_mut();
            let open: unsafe extern "system" fn(Raw, *mut Raw) -> HRESULT = geometry.method(PATH_OPEN);
            if open(geometry.as_ptr(), &mut sink_raw) < 0 {
                return None;
            }
            let sink = ComPtr::from_raw(sink_raw)?;
            let begin: unsafe extern "system" fn(Raw, D2D1_POINT_2F, u32) = sink.method(SINK_BEGIN_FIGURE);
            let end: unsafe extern "system" fn(Raw, u32) = sink.method(SINK_END_FIGURE);
            let add_line: unsafe extern "system" fn(Raw, D2D1_POINT_2F) = sink.method(SINK_ADD_LINE);
            let add_bezier: unsafe extern "system" fn(Raw, *const D2D1_BEZIER_SEGMENT) = sink.method(SINK_ADD_BEZIER);
            let close: unsafe extern "system" fn(Raw) -> HRESULT = sink.method(SINK_CLOSE);
            let mut open_figure = false;
            for segment in path.segments() {
                match segment {
                    PathSegment::MoveTo(p) => {
                        if open_figure {
                            end(sink.as_ptr(), 0);
                        }
                        begin(sink.as_ptr(), point(p), 0);
                        open_figure = true;
                    }
                    PathSegment::LineTo(p) if open_figure => add_line(sink.as_ptr(), point(p)),
                    PathSegment::BezierTo(a, b, c) if open_figure => {
                        let bezier = D2D1_BEZIER_SEGMENT { point1: point(a), point2: point(b), point3: point(c) };
                        add_bezier(sink.as_ptr(), &bezier);
                    }
                    PathSegment::Close if open_figure => {
                        end(sink.as_ptr(), 1);
                        open_figure = false;
                    }
                    _ => {}
                }
            }
            if open_figure {
                end(sink.as_ptr(), 0);
            }
            if close(sink.as_ptr()) < 0 {
                return None;
            }
            Some(geometry)
        }
    }

    /// Vector path with fill and/or stroke.
    pub fn path(&self, path: &Path, fill: Option<Color>, stroke: Option<(Color, f32)>) {
        let Some(geometry) = self.geometry(path) else { return };
        if let Some(brush) = fill.and_then(|c| self.brush(c)) {
            // SAFETY: índice 23 é FillGeometry(geometry, brush, opacityBrush).
            unsafe {
                let f: unsafe extern "system" fn(Raw, Raw, Raw, Raw) = self.target.method(RT_FILL_GEOMETRY);
                f(self.target.as_ptr(), geometry.as_ptr(), brush.as_ptr(), std::ptr::null_mut());
            }
        }
        if let Some((color, width)) = stroke {
            if let Some(brush) = self.brush(color) {
                // SAFETY: índice 22 é DrawGeometry(geometry, brush, width, style).
                unsafe {
                    let d: unsafe extern "system" fn(Raw, Raw, Raw, f32, Raw) = self.target.method(RT_DRAW_GEOMETRY);
                    d(self.target.as_ptr(), geometry.as_ptr(), brush.as_ptr(), width, std::ptr::null_mut());
                }
            }
        }
    }

    /// Rectangle with linear gradient between two points.
    pub fn gradient_rect(&self, rect: Rect, start: (f32, f32), end: (f32, f32), from: Color, to: Color) {
        let stops = [
            D2D1_GRADIENT_STOP { position: 0.0, color: color_f(from) },
            D2D1_GRADIENT_STOP { position: 1.0, color: color_f(to) },
        ];
        let properties = D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES {
            startPoint: D2D1_POINT_2F { x: start.0, y: start.1 },
            endPoint: D2D1_POINT_2F { x: end.0, y: end.1 },
        };
        let native = rect_f(rect);
        // SAFETY: índices 9 (CreateGradientStopCollection), 10 (CreateLinearGradientBrush)
        // e 17 (FillRectangle) de ID2D1RenderTarget, com as assinaturas de d2d1.h.
        unsafe {
            let mut collection: Raw = std::ptr::null_mut();
            let create_stops: unsafe extern "system" fn(
                Raw,
                *const D2D1_GRADIENT_STOP,
                u32,
                u32,
                u32,
                *mut Raw,
            ) -> HRESULT = self.target.method(RT_CREATE_GRADIENT_STOP_COLLECTION);
            if create_stops(self.target.as_ptr(), stops.as_ptr(), 2, 0, 0, &mut collection) < 0 {
                return;
            }
            let Some(collection) = ComPtr::from_raw(collection) else { return };
            let mut brush: Raw = std::ptr::null_mut();
            let create_brush: unsafe extern "system" fn(
                Raw,
                *const D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES,
                *const c_void,
                Raw,
                *mut Raw,
            ) -> HRESULT = self.target.method(RT_CREATE_LINEAR_GRADIENT_BRUSH);
            if create_brush(self.target.as_ptr(), &properties, std::ptr::null(), collection.as_ptr(), &mut brush) < 0 {
                return;
            }
            let Some(brush) = ComPtr::from_raw(brush) else { return };
            let fill: unsafe extern "system" fn(Raw, *const D2D1_RECT_F, Raw) = self.target.method(RT_FILL_RECTANGLE);
            fill(self.target.as_ptr(), &native, brush.as_ptr());
        }
    }

    /// DirectWrite text inside a rectangle.
    pub fn text(&self, text: &str, rect: Rect, font: &Font, color: Color) {
        let Some(brush) = self.brush(color) else { return };
        let family = to_wide(font.family());
        let locale = to_wide("");
        let wide: Vec<u16> = text.encode_utf16().collect();
        let layout = rect_f(rect);
        // SAFETY: IDWriteFactory[15] é CreateTextFormat e ID2D1RenderTarget[27] é DrawText,
        // com as assinaturas de dwrite.h/d2d1.h; strings terminadas em NUL ou com
        // comprimento explícito.
        unsafe {
            let mut format: Raw = std::ptr::null_mut();
            let create: unsafe extern "system" fn(
                Raw,
                *const u16,
                Raw,
                u32,
                u32,
                u32,
                f32,
                *const u16,
                *mut Raw,
            ) -> HRESULT = self.dwrite.method(DWRITE_CREATE_TEXT_FORMAT);
            let hr = create(
                self.dwrite.as_ptr(),
                family.as_ptr(),
                std::ptr::null_mut(),
                if font.is_bold() { DWRITE_FONT_WEIGHT_BOLD } else { DWRITE_FONT_WEIGHT_NORMAL },
                if font.is_italic() { DWRITE_FONT_STYLE_ITALIC } else { DWRITE_FONT_STYLE_NORMAL },
                DWRITE_FONT_STRETCH_NORMAL,
                font.size() * 96.0 / 72.0,
                locale.as_ptr(),
                &mut format,
            );
            if hr < 0 {
                return;
            }
            let Some(format) = ComPtr::from_raw(format) else { return };
            let draw: unsafe extern "system" fn(Raw, *const u16, u32, Raw, *const D2D1_RECT_F, Raw, u32, u32) =
                self.target.method(RT_DRAW_TEXT);
            draw(
                self.target.as_ptr(),
                wide.as_ptr(),
                wide.len() as u32,
                format.as_ptr(),
                &layout,
                brush.as_ptr(),
                0,
                0,
            );
        }
    }

    fn apply_transform(&self) {
        let current = *self.transform.borrow().last().unwrap_or(&IDENTITY);
        // SAFETY: índice 30 é SetTransform(const D2D1_MATRIX_3X2_F*).
        unsafe {
            let set: unsafe extern "system" fn(Raw, *const D2D1_MATRIX_3X2_F) = self.target.method(RT_SET_TRANSFORM);
            set(self.target.as_ptr(), &current);
        }
    }

    fn concat(&self, matrix: D2D1_MATRIX_3X2_F) {
        {
            let mut stack = self.transform.borrow_mut();
            if let Some(top) = stack.last_mut() {
                *top = multiply(&matrix, top);
            }
        }
        self.apply_transform();
    }

    /// Saves current transform.
    pub fn push_transform(&self) {
        let current = *self.transform.borrow().last().unwrap_or(&IDENTITY);
        self.transform.borrow_mut().push(current);
    }

    /// Restores transform saved by [`push_transform`](Self::push_transform).
    pub fn pop_transform(&self) {
        {
            let mut stack = self.transform.borrow_mut();
            if stack.len() > 1 {
                stack.pop();
            }
        }
        self.apply_transform();
    }

    /// Translates origin.
    pub fn translate(&self, dx: f32, dy: f32) {
        self.concat([1.0, 0.0, 0.0, 1.0, dx, dy]);
    }

    /// Rotates clockwise around origin in degrees.
    pub fn rotate(&self, degrees: f32) {
        let (sin, cos) = degrees.to_radians().sin_cos();
        self.concat([cos, sin, -sin, cos, 0.0, 0.0]);
    }

    /// Applies scaling.
    pub fn scale(&self, sx: f32, sy: f32) {
        self.concat([sx, 0.0, 0.0, sy, 0.0, 0.0]);
    }

    /// Pushes rectangular clipping; pair with [`pop_clip`](Self::pop_clip).
    pub fn push_clip(&self, rect: Rect) {
        let rect = rect_f(rect);
        // SAFETY: índice 45 é PushAxisAlignedClip(rect*, antialiasMode).
        unsafe {
            let push: unsafe extern "system" fn(Raw, *const D2D1_RECT_F, u32) =
                self.target.method(RT_PUSH_AXIS_ALIGNED_CLIP);
            push(self.target.as_ptr(), &rect, 0);
        }
    }

    /// Ends latest clipping region.
    pub fn pop_clip(&self) {
        // SAFETY: índice 46 é PopAxisAlignedClip().
        unsafe {
            let pop: unsafe extern "system" fn(Raw) = self.target.method(RT_POP_AXIS_ALIGNED_CLIP);
            pop(self.target.as_ptr());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrix_multiplication_composes_in_order() {
        let translate = [1.0, 0.0, 0.0, 1.0, 10.0, 5.0];
        let scale = [2.0, 0.0, 0.0, 2.0, 0.0, 0.0];
        // Primeiro translada, depois escala: (1,1) -> (11,6) -> (22,12).
        let m = multiply(&translate, &scale);
        let (x, y) = (1.0 * m[0] + 1.0 * m[2] + m[4], 1.0 * m[1] + 1.0 * m[3] + m[5]);
        assert_eq!((x, y), (22.0, 12.0));
        assert_eq!(multiply(&IDENTITY, &scale), scale);
    }

    #[test]
    fn color_and_rect_conversion() {
        let c = color_f(Color::rgba(255, 0, 51, 128));
        assert!((c.r - 1.0).abs() < 1e-6 && c.g == 0.0 && (c.b - 0.2).abs() < 1e-6);
        assert!((c.a - 128.0 / 255.0).abs() < 1e-6);
        let r = rect_f(Rect::new(1, 2, 3, 4));
        assert_eq!((r.left, r.top, r.right, r.bottom), (1.0, 2.0, 4.0, 6.0));
    }
}
