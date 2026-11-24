//! GDI backend: [`Canvas`] drawing for windows, bitmaps and printers.

use super::font::Font;
use crate::core::handles::{GdiObject, Icon, IconSource, MemoryDc};
use crate::core::{GuiError, GuiResult};
use crate::ffi::gdi32::*;
use crate::ffi::user32::*;
use crate::ffi::*;
use crate::utilities::{to_logical, to_wide, Color, Point, Rect, Size};

/// Pen stroke style.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PenStyle {
    /// Solid line.
    Solid,
    /// Dashed line.
    Dash,
    /// Dotted line.
    Dot,
}

/// Pen for outlines and lines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pen {
    /// Stroke color.
    pub color: Color,
    /// Logical stroke width.
    pub width: i32,
    /// Stroke style.
    pub style: PenStyle,
}

impl Pen {
    /// Solid pen.
    pub const fn new(color: Color, width: i32) -> Pen {
        Pen { color, width, style: PenStyle::Solid }
    }

    /// Same pen with another style.
    pub const fn with_style(mut self, style: PenStyle) -> Pen {
        self.style = style;
        self
    }
}

/// Horizontal text alignment within a rectangle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TextAlign {
    /// Left.
    Left,
    /// Centered.
    Center,
    /// Right.
    Right,
}

/// Linear gradient direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum GradientDirection {
    /// Left to right.
    Horizontal,
    /// Top to bottom.
    Vertical,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Segment {
    MoveTo(Point),
    LineTo(Point),
    BezierTo(Point, Point, Point),
    Close,
}

/// Vector path of lines and Bézier curves.
///
/// The same path supports all three backends: GDI, GDI+ and Direct2D.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Path {
    segments: Vec<Segment>,
}

/// [`Path`] segment exposed for alternative backends.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum PathSegment {
    /// Starts a new figure.
    MoveTo(Point),
    /// Straight line to point.
    LineTo(Point),
    /// Cubic Bézier with two control points and endpoint.
    BezierTo(Point, Point, Point),
    /// Closes current figure.
    Close,
}

impl Path {
    /// Empty path.
    pub fn new() -> Path {
        Path::default()
    }

    /// Starts a figure at the given point.
    pub fn move_to(mut self, x: i32, y: i32) -> Path {
        self.segments.push(Segment::MoveTo(Point::new(x, y)));
        self
    }

    /// Adds a straight line.
    pub fn line_to(mut self, x: i32, y: i32) -> Path {
        self.segments.push(Segment::LineTo(Point::new(x, y)));
        self
    }

    /// Adds a cubic Bézier curve.
    pub fn bezier_to(mut self, c1: Point, c2: Point, end: Point) -> Path {
        self.segments.push(Segment::BezierTo(c1, c2, end));
        self
    }

    /// Closes current figure.
    pub fn close(mut self) -> Path {
        self.segments.push(Segment::Close);
        self
    }

    /// Whether the path has no segments.
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    /// Segments in insertion order.
    pub fn segments(&self) -> Vec<PathSegment> {
        self.segments
            .iter()
            .map(|s| match *s {
                Segment::MoveTo(p) => PathSegment::MoveTo(p),
                Segment::LineTo(p) => PathSegment::LineTo(p),
                Segment::BezierTo(a, b, c) => PathSegment::BezierTo(a, b, c),
                Segment::Close => PathSegment::Close,
            })
            .collect()
    }
}

/// Owned GDI bitmap (`HBITMAP`).
#[derive(Debug)]
pub struct Bitmap {
    handle: GdiObject,
    size: Size,
}

impl Bitmap {
    /// Loads an image file: GDI for BMP, GDI+ for PNG/JPEG/GIF/TIFF/ICO.
    pub fn from_file(path: impl AsRef<std::path::Path>) -> GuiResult<Bitmap> {
        let path = path.as_ref();
        let is_bmp = path.extension().map(|e| e.eq_ignore_ascii_case("bmp")).unwrap_or(false);
        if !is_bmp {
            return super::gdiplus::load_bitmap(path);
        }
        let wide = to_wide(&path.to_string_lossy());
        // SAFETY: string válida terminada em NUL; o handle resultante passa ao wrapper.
        let raw = unsafe {
            LoadImageW(std::ptr::null_mut(), wide.as_ptr(), IMAGE_BITMAP, 0, 0, LR_LOADFROMFILE | LR_CREATEDIBSECTION)
        };
        // SAFETY: handle recém-criado e exclusivo.
        let handle =
            unsafe { GdiObject::from_raw(raw) }.ok_or_else(|| GuiError::last_os_error("LoadImageW(bitmap)"))?;
        Bitmap::from_handle(handle)
    }

    /// Creates a 32-bit bitmap of given pixel size using `draw`.
    ///
    /// The supplied [`Canvas`] draws in pixels (96 DPI).
    pub fn render(width: i32, height: i32, draw: impl FnOnce(&Canvas)) -> GuiResult<Bitmap> {
        Bitmap::render_at(width, height, 96, None, draw).map(|(bitmap, _)| bitmap)
    }

    /// Like [`render`](Self::render) with given canvas DPI, scaling logical drawing to `width` × `height` pixels,
    /// optional background and resulting BGRA copy (alpha 255).
    pub(crate) fn render_at(
        width: i32,
        height: i32,
        dpi: u32,
        background: Option<Color>,
        draw: impl FnOnce(&Canvas),
    ) -> GuiResult<(Bitmap, Vec<u8>)> {
        if width <= 0 || height <= 0 {
            return Err(GuiError::InvalidArgument("bitmap dimensions must be positive".into()));
        }
        let dc = MemoryDc::compatible_with(std::ptr::null_mut())?;
        let header = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            ..Default::default()
        };
        let mut bits = std::ptr::null_mut();
        // SAFETY: cabeçalho completo; `bits` recebe o ponteiro dos pixels, que pertence
        // ao bitmap criado.
        let raw = unsafe { CreateDIBSection(dc.raw(), &header, DIB_RGB_COLORS, &mut bits, std::ptr::null_mut(), 0) };
        // SAFETY: handle recém-criado e exclusivo.
        let handle = unsafe { GdiObject::from_raw(raw) }.ok_or_else(|| GuiError::last_os_error("CreateDIBSection"))?;
        let count = width as usize * height as usize * 4;
        // SAFETY: seleciona o bitmap no DC de memória e o desfaz antes de o DC ser apagado;
        // `bits` aponta para os `count` bytes da seção, vivos enquanto o bitmap existir.
        let pixels = unsafe {
            let previous = SelectObject(dc.raw(), handle.raw());
            let pixels = std::slice::from_raw_parts_mut(bits as *mut u8, count);
            if let Some(color) = background {
                for p in pixels.chunks_exact_mut(4) {
                    p.copy_from_slice(&[color.b, color.g, color.r, 0]);
                }
            }
            {
                let canvas = Canvas::for_hdc(dc.raw(), dpi.max(1), Size::new(width, height));
                draw(&canvas);
            }
            GdiFlush();
            SelectObject(dc.raw(), previous);
            let mut copy = pixels.to_vec();
            copy.chunks_exact_mut(4).for_each(|p| p[3] = 255);
            copy
        };
        Ok((Bitmap { handle, size: Size::new(width, height) }, pixels))
    }

    /// Converts an icon to a transparent 32-bit bitmap, useful in menus.
    pub fn from_icon(icon: &Icon, size: i32) -> GuiResult<Bitmap> {
        let raw_icon = icon.raw();
        Bitmap::render(size, size, |canvas| {
            // SAFETY: o ícone é mantido vivo por `icon` durante a chamada.
            unsafe { DrawIconEx(canvas.hdc(), 0, 0, raw_icon, size, size, 0, std::ptr::null_mut(), DI_NORMAL) };
        })
    }

    pub(crate) fn from_handle(handle: GdiObject) -> GuiResult<Bitmap> {
        let mut info = BITMAP::default();
        // SAFETY: buffer do tamanho de BITMAP para um handle de bitmap válido.
        let read =
            unsafe { GetObjectW(handle.raw(), std::mem::size_of::<BITMAP>() as i32, &mut info as *mut _ as *mut _) };
        if read == 0 {
            return Err(GuiError::last_os_error("GetObjectW(bitmap)"));
        }
        Ok(Bitmap { handle, size: Size::new(info.bmWidth, info.bmHeight.abs()) })
    }

    /// Pixel size.
    pub fn size(&self) -> Size {
        self.size
    }

    /// Raw handle (advanced API).
    pub fn raw(&self) -> HBITMAP {
        self.handle.raw()
    }
}

/// GDI drawing surface.
///
/// Coordinates are **logical** (96 DPI). `Canvas` configures the DC for Windows to convert to monitor pixels and
/// restores DC state on drop.
///
/// Supplied to [`Window::on_paint`](crate::core::Window::on_paint) and
/// [`Panel::on_paint`](crate::controls::Panel::on_paint).
#[derive(Debug)]
pub struct Canvas {
    hdc: HDC,
    dpi: u32,
    size: Size,
    saved: i32,
}

impl Canvas {
    /// Prepares logical-unit DC drawing; `pixel_size` is pixel dimensions.
    pub(crate) fn for_hdc(hdc: HDC, dpi: u32, pixel_size: Size) -> Canvas {
        // SAFETY: `hdc` é um DC válido fornecido pelo chamador; todas as alterações são
        // desfeitas por RestoreDC no Drop.
        let saved = unsafe {
            let saved = SaveDC(hdc);
            SetGraphicsMode(hdc, GM_ADVANCED);
            if dpi != 96 {
                SetMapMode(hdc, MM_ANISOTROPIC);
                SetWindowExtEx(hdc, 96, 96, std::ptr::null_mut());
                SetViewportExtEx(hdc, dpi as i32, dpi as i32, std::ptr::null_mut());
            }
            SetBkMode(hdc, TRANSPARENT);
            saved
        };
        let size = Size::new(to_logical(pixel_size.width, dpi), to_logical(pixel_size.height, dpi));
        Canvas { hdc, dpi, size, saved }
    }

    /// Logical surface size.
    pub fn size(&self) -> Size {
        self.size
    }

    /// Target device DPI.
    pub fn dpi(&self) -> u32 {
        self.dpi
    }

    /// Underlying `HDC` (advanced API).
    pub fn hdc(&self) -> HDC {
        self.hdc
    }

    fn with_objects(&self, pen: Option<&Pen>, fill: Option<Color>, draw: impl FnOnce()) {
        // SAFETY: objetos GDI temporários são selecionados, usados e desselecionados
        // antes de serem apagados; os objetos de estoque não precisam de liberação.
        unsafe {
            let pen_handle = match pen {
                Some(p) => {
                    let style = match p.style {
                        PenStyle::Solid => PS_SOLID,
                        PenStyle::Dash => PS_DASH,
                        PenStyle::Dot => PS_DOT,
                    };
                    GdiObject::from_raw(CreatePen(style, p.width.max(1), p.color.to_colorref()))
                }
                None => None,
            };
            let brush_handle = fill.and_then(|c| GdiObject::from_raw(CreateSolidBrush(c.to_colorref())));
            let old_pen = SelectObject(
                self.hdc,
                pen_handle.as_ref().map(|p| p.raw()).unwrap_or_else(|| GetStockObject(NULL_PEN)),
            );
            let old_brush = SelectObject(
                self.hdc,
                brush_handle.as_ref().map(|b| b.raw()).unwrap_or_else(|| GetStockObject(NULL_BRUSH)),
            );
            draw();
            SelectObject(self.hdc, old_pen);
            SelectObject(self.hdc, old_brush);
        }
    }

    /// Fills the whole surface with a color.
    pub fn clear(&self, color: Color) {
        self.fill_rect(Rect::new(0, 0, self.size.width + 1, self.size.height + 1), color);
    }

    /// Draws a line.
    pub fn line(&self, from: Point, to: Point, pen: &Pen) {
        self.with_objects(Some(pen), None, || {
            // SAFETY: DC válido durante a vida do Canvas.
            unsafe {
                MoveToEx(self.hdc, from.x, from.y, std::ptr::null_mut());
                LineTo(self.hdc, to.x, to.y);
            }
        });
    }

    /// Fills a rectangle.
    pub fn fill_rect(&self, rect: Rect, color: Color) {
        self.rect(rect, None, Some(color));
    }

    /// Draws a rectangle with stroke and/or fill.
    pub fn rect(&self, rect: Rect, pen: Option<&Pen>, fill: Option<Color>) {
        self.with_objects(pen, fill, || {
            let extra = if pen.is_none() { 1 } else { 0 };
            // SAFETY: DC válido durante a vida do Canvas.
            unsafe { Rectangle(self.hdc, rect.x, rect.y, rect.right() + extra, rect.bottom() + extra) };
        });
    }

    /// Draws a rounded rectangle.
    pub fn round_rect(&self, rect: Rect, radius: i32, pen: Option<&Pen>, fill: Option<Color>) {
        self.with_objects(pen, fill, || {
            let extra = if pen.is_none() { 1 } else { 0 };
            // SAFETY: DC válido durante a vida do Canvas.
            unsafe {
                RoundRect(self.hdc, rect.x, rect.y, rect.right() + extra, rect.bottom() + extra, radius * 2, radius * 2)
            };
        });
    }

    /// Draws an ellipse/circle inscribed in the rectangle.
    pub fn ellipse(&self, bounds: Rect, pen: Option<&Pen>, fill: Option<Color>) {
        self.with_objects(pen, fill, || {
            let extra = if pen.is_none() { 1 } else { 0 };
            // SAFETY: DC válido durante a vida do Canvas.
            unsafe { Ellipse(self.hdc, bounds.x, bounds.y, bounds.right() + extra, bounds.bottom() + extra) };
        });
    }

    /// Draws a closed polygon.
    pub fn polygon(&self, points: &[Point], pen: Option<&Pen>, fill: Option<Color>) {
        if points.len() < 2 {
            return;
        }
        let native: Vec<POINT> = points.iter().map(|p| POINT { x: p.x, y: p.y }).collect();
        self.with_objects(pen, fill, || {
            // SAFETY: `native` contém exatamente `len` pontos válidos.
            unsafe { Polygon(self.hdc, native.as_ptr(), native.len() as i32) };
        });
    }

    /// Draws connected line segments.
    pub fn polyline(&self, points: &[Point], pen: &Pen) {
        if points.len() < 2 {
            return;
        }
        let native: Vec<POINT> = points.iter().map(|p| POINT { x: p.x, y: p.y }).collect();
        self.with_objects(Some(pen), None, || {
            // SAFETY: `native` contém exatamente `len` pontos válidos.
            unsafe { Polyline(self.hdc, native.as_ptr(), native.len() as i32) };
        });
    }

    fn trace_path(&self, path: &Path) {
        // SAFETY: sequência BeginPath/EndPath em um DC válido; os arrays locais têm o
        // tamanho informado.
        unsafe {
            BeginPath(self.hdc);
            for segment in &path.segments {
                match *segment {
                    Segment::MoveTo(p) => {
                        MoveToEx(self.hdc, p.x, p.y, std::ptr::null_mut());
                    }
                    Segment::LineTo(p) => {
                        LineTo(self.hdc, p.x, p.y);
                    }
                    Segment::BezierTo(a, b, c) => {
                        let points = [POINT { x: a.x, y: a.y }, POINT { x: b.x, y: b.y }, POINT { x: c.x, y: c.y }];
                        PolyBezierTo(self.hdc, points.as_ptr(), 3);
                    }
                    Segment::Close => {
                        CloseFigure(self.hdc);
                    }
                }
            }
            EndPath(self.hdc);
        }
    }

    /// Draws a vector path with stroke and/or fill.
    pub fn path(&self, path: &Path, pen: Option<&Pen>, fill: Option<Color>) {
        if path.is_empty() {
            return;
        }
        self.with_objects(pen, fill, || {
            self.trace_path(path);
            // SAFETY: há um caminho fechado por EndPath no DC.
            unsafe {
                match (pen.is_some(), fill.is_some()) {
                    (true, true) => StrokeAndFillPath(self.hdc),
                    (true, false) => StrokePath(self.hdc),
                    (false, true) => FillPath(self.hdc),
                    (false, false) => 0,
                };
            }
        });
    }

    fn with_font<R>(&self, font: &Font, color: Color, draw: impl FnOnce() -> R) -> Option<R> {
        // O mapeamento do DC já aplica a escala de DPI; a fonte é criada para 96 DPI.
        let handle = font.handle_for_dpi(96).ok()?;
        // SAFETY: a fonte é selecionada e desselecionada antes de `handle` ser solto.
        unsafe {
            let previous = SelectObject(self.hdc, handle.raw());
            SetTextColor(self.hdc, color.to_colorref());
            let result = draw();
            SelectObject(self.hdc, previous);
            Some(result)
        }
    }

    /// Draws text with top-left at `origin`.
    pub fn text(&self, text: &str, origin: Point, font: &Font, color: Color) {
        let wide = utf16(text);
        self.with_font(font, color, || {
            // SAFETY: `wide` tem exatamente `len` unidades UTF-16.
            unsafe { TextOutW(self.hdc, origin.x, origin.y, wide.as_ptr(), wide.len() as i32) };
        });
    }

    /// Draws wrapping, aligned text in a rectangle.
    pub fn text_in_rect(&self, text: &str, rect: Rect, font: &Font, color: Color, align: TextAlign) {
        let wide = utf16(text);
        let flags = DT_WORDBREAK
            | DT_NOPREFIX
            | match align {
                TextAlign::Left => DT_LEFT,
                TextAlign::Center => DT_CENTER,
                TextAlign::Right => DT_RIGHT,
            };
        self.with_font(font, color, || {
            let mut native = rect.to_native();
            // SAFETY: `wide` tem `len` unidades; `native` é um RECT local.
            unsafe { DrawTextW(self.hdc, wide.as_ptr(), wide.len() as i32, &mut native, flags) };
        });
    }

    /// Draws a line centered horizontally/vertically in a rectangle.
    pub fn text_centered(&self, text: &str, rect: Rect, font: &Font, color: Color) {
        let wide = utf16(text);
        self.with_font(font, color, || {
            let mut native = rect.to_native();
            let flags = DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS;
            // SAFETY: `wide` tem `len` unidades; `native` é um RECT local.
            unsafe { DrawTextW(self.hdc, wide.as_ptr(), wide.len() as i32, &mut native, flags) };
        });
    }

    /// Draws left-aligned, vertically centered text with ellipsis on overflow.
    pub fn text_line(&self, text: &str, rect: Rect, font: &Font, color: Color) {
        let wide = utf16(text);
        self.with_font(font, color, || {
            let mut native = rect.to_native();
            let flags = DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS;
            // SAFETY: `wide` tem `len` unidades; `native` é um RECT local.
            unsafe { DrawTextW(self.hdc, wide.as_ptr(), wide.len() as i32, &mut native, flags) };
        });
    }

    /// Vertically centered mnemonic text (`&` underlines next letter, `&&` is literal). `show_prefix = false` hides
    /// underlines, like Windows menus before Alt.
    pub(crate) fn mnemonic_text(
        &self,
        text: &str,
        rect: Rect,
        font: &Font,
        color: Color,
        align: TextAlign,
        show_prefix: bool,
    ) {
        let wide = utf16(text);
        let mut flags = DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS;
        flags |= match align {
            TextAlign::Left => DT_LEFT,
            TextAlign::Center => DT_CENTER,
            TextAlign::Right => DT_RIGHT,
        };
        if !show_prefix {
            flags |= DT_HIDEPREFIX;
        }
        self.with_font(font, color, || {
            let mut native = rect.to_native();
            // SAFETY: `wide` tem `len` unidades; `native` é um RECT local.
            unsafe { DrawTextW(self.hdc, wide.as_ptr(), wide.len() as i32, &mut native, flags) };
        });
    }

    /// Automatically wrapped text in `rect`, without processing `&`.
    pub(crate) fn wrapped_text(&self, text: &str, rect: Rect, font: &Font, color: Color) {
        let wide = utf16(text);
        self.with_font(font, color, || {
            let mut native = rect.to_native();
            // SAFETY: `wide` tem `len` unidades; `native` é um RECT local.
            unsafe {
                DrawTextW(
                    self.hdc,
                    wide.as_ptr(),
                    wide.len() as i32,
                    &mut native,
                    DT_WORDBREAK | DT_NOPREFIX | DT_END_ELLIPSIS,
                )
            };
        });
    }

    /// Logical height of `text` wrapped to `width`.
    pub(crate) fn measure_wrapped(&self, text: &str, font: &Font, width: i32) -> Size {
        let wide = utf16(text);
        self.with_font(font, Color::BLACK, || {
            let mut native = RECT { left: 0, top: 0, right: width, bottom: 0 };
            // SAFETY: `wide` tem `len` unidades; `native` é um RECT local.
            unsafe {
                DrawTextW(
                    self.hdc,
                    wide.as_ptr(),
                    wide.len() as i32,
                    &mut native,
                    DT_WORDBREAK | DT_NOPREFIX | DT_CALCRECT,
                )
            };
            Size::new(native.right - native.left, native.bottom - native.top)
        })
        .unwrap_or_default()
    }

    /// Measures mnemonic text (`&` takes no space).
    pub(crate) fn measure_mnemonic(&self, text: &str, font: &Font) -> Size {
        let wide = utf16(text);
        self.with_font(font, Color::BLACK, || {
            let mut native = RECT { left: 0, top: 0, right: 0, bottom: 0 };
            // SAFETY: `wide` tem `len` unidades; `native` é um RECT local.
            unsafe { DrawTextW(self.hdc, wide.as_ptr(), wide.len() as i32, &mut native, DT_SINGLELINE | DT_CALCRECT) };
            Size::new(native.right - native.left, native.bottom - native.top)
        })
        .unwrap_or_default()
    }

    /// Measures logical size of one text line.
    pub fn measure_text(&self, text: &str, font: &Font) -> Size {
        let wide = utf16(text);
        self.with_font(font, Color::BLACK, || {
            let mut size = SIZE::default();
            // SAFETY: `wide` tem `len` unidades; `size` é um SIZE local.
            unsafe { GetTextExtentPoint32W(self.hdc, wide.as_ptr(), wide.len() as i32, &mut size) };
            Size::new(size.cx, size.cy)
        })
        .unwrap_or_default()
    }

    /// Fills a rectangle with a two-color linear gradient.
    pub fn gradient_rect(&self, rect: Rect, from: Color, to: Color, direction: GradientDirection) {
        let vertex = |x: i32, y: i32, c: Color| TRIVERTEX {
            x,
            y,
            Red: (c.r as u16) << 8,
            Green: (c.g as u16) << 8,
            Blue: (c.b as u16) << 8,
            Alpha: (c.a as u16) << 8,
        };
        let vertices = [vertex(rect.x, rect.y, from), vertex(rect.right(), rect.bottom(), to)];
        let mesh = GRADIENT_RECT { UpperLeft: 0, LowerRight: 1 };
        let mode = match direction {
            GradientDirection::Horizontal => GRADIENT_FILL_RECT_H,
            GradientDirection::Vertical => GRADIENT_FILL_RECT_V,
        };
        // SAFETY: dois vértices e uma malha, com as contagens informadas.
        unsafe { GradientFill(self.hdc, vertices.as_ptr(), 2, &mesh as *const _ as *const _, 1, mode) };
    }

    fn blit(&self, bitmap: &Bitmap, dest: Rect, alpha: Option<u8>) {
        let Ok(source) = MemoryDc::compatible_with(self.hdc) else { return };
        let size = bitmap.size();
        // SAFETY: o bitmap é selecionado no DC de memória apenas durante a cópia.
        unsafe {
            let previous = SelectObject(source.raw(), bitmap.raw());
            match alpha {
                Some(alpha) => {
                    let blend = BLENDFUNCTION {
                        BlendOp: AC_SRC_OVER,
                        BlendFlags: 0,
                        SourceConstantAlpha: alpha,
                        AlphaFormat: 0,
                    };
                    AlphaBlend(
                        self.hdc,
                        dest.x,
                        dest.y,
                        dest.width,
                        dest.height,
                        source.raw(),
                        0,
                        0,
                        size.width,
                        size.height,
                        blend,
                    );
                }
                None => {
                    SetStretchBltMode(self.hdc, HALFTONE);
                    StretchBlt(
                        self.hdc,
                        dest.x,
                        dest.y,
                        dest.width,
                        dest.height,
                        source.raw(),
                        0,
                        0,
                        size.width,
                        size.height,
                        SRCCOPY,
                    );
                }
            }
            SelectObject(source.raw(), previous);
        }
    }

    /// Draws a bitmap stretched to destination rectangle.
    pub fn draw_bitmap(&self, bitmap: &Bitmap, dest: Rect) {
        self.blit(bitmap, dest, None);
    }

    /// Draws a bitmap with global `alpha` 0 to 255.
    pub fn draw_bitmap_alpha(&self, bitmap: &Bitmap, dest: Rect, alpha: u8) {
        self.blit(bitmap, dest, Some(alpha));
    }

    /// Draws an icon at given logical size.
    ///
    /// Accepts [`Icon`] or bundled icon ([`crate::icons`]), loading the latter at surface DPI resolution. Load failures
    /// draw nothing.
    pub fn draw_icon(&self, icon: &impl IconSource, origin: Point, size: i32) {
        let Ok(icon) = icon.icon_pixels(crate::utilities::to_pixels(size, self.dpi)) else { return };
        // SAFETY: o ícone é mantido vivo por `icon` durante a chamada.
        unsafe { DrawIconEx(self.hdc, origin.x, origin.y, icon.raw(), size, size, 0, std::ptr::null_mut(), DI_NORMAL) };
    }

    /// Draws the system dotted focus rectangle.
    pub fn focus_rect(&self, rect: Rect) {
        let native = rect.to_native();
        // SAFETY: RECT local válido.
        unsafe { DrawFocusRect(self.hdc, &native) };
    }

    /// Intersects current clip with a rectangle.
    pub fn clip_rect(&self, rect: Rect) {
        // SAFETY: DC válido.
        unsafe { IntersectClipRect(self.hdc, rect.x, rect.y, rect.right(), rect.bottom()) };
    }

    /// Clips to vector path interior.
    pub fn clip_path(&self, path: &Path) {
        if path.is_empty() {
            return;
        }
        self.trace_path(path);
        // SAFETY: há um caminho fechado por EndPath no DC.
        unsafe { SelectClipPath(self.hdc, RGN_AND) };
    }

    /// Removes all clipping.
    pub fn reset_clip(&self) {
        // SAFETY: região nula remove o recorte.
        unsafe { SelectClipRgn(self.hdc, std::ptr::null_mut()) };
    }

    /// Saves graphics state (transform/clip), returning a marker.
    pub fn save(&self) -> i32 {
        // SAFETY: DC válido.
        unsafe { SaveDC(self.hdc) }
    }

    /// Restores state saved by [`save`](Self::save).
    pub fn restore(&self, state: i32) {
        if state > self.saved {
            // SAFETY: restaura apenas estados criados depois do início deste Canvas.
            unsafe { RestoreDC(self.hdc, state) };
        }
    }

    fn transform(&self, xform: XFORM) {
        // SAFETY: XFORM local; o DC está em GM_ADVANCED.
        unsafe { ModifyWorldTransform(self.hdc, &xform, MWT_LEFTMULTIPLY) };
    }

    /// Translates coordinate origin.
    pub fn translate(&self, dx: f32, dy: f32) {
        self.transform(XFORM { eM11: 1.0, eM12: 0.0, eM21: 0.0, eM22: 1.0, eDx: dx, eDy: dy });
    }

    /// Rotates coordinates clockwise around origin in degrees.
    pub fn rotate(&self, degrees: f32) {
        let (sin, cos) = degrees.to_radians().sin_cos();
        self.transform(XFORM { eM11: cos, eM12: sin, eM21: -sin, eM22: cos, eDx: 0.0, eDy: 0.0 });
    }

    /// Scales coordinates.
    pub fn scale(&self, sx: f32, sy: f32) {
        self.transform(XFORM { eM11: sx, eM12: 0.0, eM21: 0.0, eM22: sy, eDx: 0.0, eDy: 0.0 });
    }

    /// Removes all transforms.
    pub fn reset_transform(&self) {
        // SAFETY: com MWT_IDENTITY o ponteiro de matriz é ignorado.
        unsafe { ModifyWorldTransform(self.hdc, std::ptr::null(), MWT_IDENTITY) };
    }

    /// Reads a logical pixel color; `None` outside clipping.
    pub fn pixel(&self, at: Point) -> Option<Color> {
        // SAFETY: DC válido; CLR_INVALID sinaliza ponto fora do recorte.
        let value = unsafe { GetPixel(self.hdc, at.x, at.y) };
        (value != 0xFFFF_FFFF).then(|| Color::from_colorref(value))
    }

    /// Draws SVG in a logical rectangle with GDI+ antialiasing.
    pub fn draw_svg(&self, svg: &super::SvgImage, dest: Rect) -> GuiResult<()> {
        self.with_gdiplus(|g| svg.draw(g, dest.x as f64, dest.y as f64, dest.width as f64, dest.height as f64))
    }

    /// Runs GDI+ drawing (antialiasing, gradients, images) on this canvas.
    pub fn with_gdiplus(&self, draw: impl FnOnce(&super::gdiplus::PlusGraphics)) -> GuiResult<()> {
        // GDI+ inherits the HDC mapping on Windows. Canvas already maps 96 DPI
        // to device units, while from_hdc applies that scale to GDI+ explicitly.
        // Temporarily remove only the mapping scale to avoid applying it twice;
        // retain the canvas clipping and world transform.
        // SAFETY: HDC remains valid; the guard restores its saved state.
        let saved = unsafe { SaveDC(self.hdc) };
        if saved == 0 {
            return Err(GuiError::last_os_error("SaveDC(GDI+)"));
        }
        struct RestoreMapping(HDC, i32);
        impl Drop for RestoreMapping {
            fn drop(&mut self) {
                // SAFETY: state saved on this live DC; the GDI+ context drops
                // before this guard, including on errors and panics.
                unsafe { RestoreDC(self.0, self.1) };
            }
        }
        let _mapping = RestoreMapping(self.hdc, saved);
        // SAFETY: temporarily changes the DC mapping saved above.
        if unsafe { SetMapMode(self.hdc, MM_TEXT) } == 0 {
            return Err(GuiError::last_os_error("SetMapMode(GDI+)"));
        }
        let graphics = super::gdiplus::PlusGraphics::from_hdc(self.hdc, self.dpi)?;
        draw(&graphics);
        Ok(())
    }
}

impl Drop for Canvas {
    fn drop(&mut self) {
        // SAFETY: restaura o estado salvo em `for_hdc` no mesmo DC.
        unsafe { RestoreDC(self.hdc, self.saved) };
    }
}

/// UTF-16 text with NUL excluded from length but present after the end; even empty text has a valid pointer (Wine
/// `DrawTextW` reads the first character at length 0).
fn utf16(text: &str) -> Vec<u16> {
    let mut wide: Vec<u16> = text.encode_utf16().collect();
    wide.push(0);
    wide.pop();
    wide
}

/// Renders `width`×`height` pixels offscreen, copying to `target` at (`x`, `y`), for nonclient areas with nonzero
/// origin.
#[allow(clippy::too_many_arguments)]
pub(crate) fn paint_buffered_at(
    target: HDC,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    dpi: u32,
    background: Color,
    draw: impl FnOnce(&Canvas),
) {
    if width <= 0 || height <= 0 {
        return;
    }
    let Ok(memory) = MemoryDc::compatible_with(target) else { return };
    // SAFETY: bitmap temporário compatível com o DC de destino, selecionado no DC de
    // memória só durante o desenho e apagado pelo wrapper ao final.
    unsafe {
        let Some(bitmap) = GdiObject::from_raw(CreateCompatibleBitmap(target, width, height)) else { return };
        let previous = SelectObject(memory.raw(), bitmap.raw());
        let area = RECT { left: 0, top: 0, right: width, bottom: height };
        FillRect(memory.raw(), &area, crate::core::application::brush_for(background));
        {
            let canvas = Canvas::for_hdc(memory.raw(), dpi, Size::new(width, height));
            draw(&canvas);
        }
        BitBlt(target, x, y, width, height, memory.raw(), 0, 0, SRCCOPY);
        SelectObject(memory.raw(), previous);
    }
}

/// Draws offscreen and copies at once (double buffering).
pub(crate) fn paint_buffered(
    target: HDC,
    client: RECT,
    dpi: u32,
    background: Option<Color>,
    draw: impl FnOnce(&Canvas),
) {
    let (width, height) = (client.right - client.left, client.bottom - client.top);
    if width <= 0 || height <= 0 {
        return;
    }
    let Ok(memory) = MemoryDc::compatible_with(target) else { return };
    // SAFETY: bitmap temporário compatível com o DC de destino; é selecionado no DC de
    // memória só durante o desenho e apagado pelo wrapper ao final.
    unsafe {
        let Some(bitmap) = GdiObject::from_raw(CreateCompatibleBitmap(target, width, height)) else { return };
        let previous = SelectObject(memory.raw(), bitmap.raw());
        let brush = match background {
            Some(color) => crate::core::application::brush_for(color),
            None => GetSysColorBrush(COLOR_BTNFACE),
        };
        FillRect(memory.raw(), &client, brush);
        {
            let canvas = Canvas::for_hdc(memory.raw(), dpi, Size::new(width, height));
            draw(&canvas);
        }
        BitBlt(target, 0, 0, width, height, memory.raw(), 0, 0, SRCCOPY);
        SelectObject(memory.raw(), previous);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_builder_records_segments() {
        let path = Path::new().move_to(0, 0).line_to(10, 0).line_to(10, 10).close();
        assert!(!path.is_empty());
        assert_eq!(path.segments().len(), 4);
        assert_eq!(path.segments()[3], PathSegment::Close);
        assert!(Path::new().is_empty());
    }

    #[test]
    fn offscreen_drawing_produces_expected_pixels() {
        let bitmap = Bitmap::render(40, 30, |canvas| {
            canvas.clear(Color::WHITE);
            canvas.fill_rect(Rect::new(10, 10, 10, 10), Color::rgb(255, 0, 0));
            assert_eq!(canvas.pixel(Point::new(15, 15)), Some(Color::rgb(255, 0, 0)));
            assert_eq!(canvas.pixel(Point::new(2, 2)), Some(Color::WHITE));
            canvas.line(Point::new(0, 25), Point::new(39, 25), &Pen::new(Color::rgb(0, 0, 255), 1));
            assert_eq!(canvas.pixel(Point::new(20, 25)), Some(Color::rgb(0, 0, 255)));
            let state = canvas.save();
            canvas.translate(20.0, 0.0);
            canvas.fill_rect(Rect::new(0, 0, 5, 5), Color::rgb(0, 255, 0));
            canvas.restore(state);
            assert_eq!(canvas.pixel(Point::new(22, 2)), Some(Color::rgb(0, 255, 0)));
            let measured = canvas.measure_text("Olá", &Font::new("Arial", 10.0));
            assert!(measured.width > 0 && measured.height > 0);
        })
        .expect("bitmap");
        assert_eq!(bitmap.size(), Size::new(40, 30));
        assert!(Bitmap::render(0, 10, |_| {}).is_err());
    }

    #[test]
    fn gdiplus_and_gdi_use_the_same_dpi_scale() {
        for dpi in [96, 144, 192] {
            let (_, pixels) = Bitmap::render_at(160, 160, dpi, Some(Color::WHITE), |canvas| {
                canvas.fill_rect(Rect::new(10, 4, 20, 10), Color::rgb(255, 0, 0));
                canvas.with_gdiplus(|g| g.fill_rect(10.0, 24.0, 20.0, 10.0, Color::rgb(0, 255, 0))).unwrap();
                canvas.fill_rect(Rect::new(10, 44, 20, 10), Color::rgb(0, 0, 255));
            })
            .unwrap();
            let at = |x: i32, y: i32| {
                let x = x * dpi as i32 / 96;
                let y = y * dpi as i32 / 96;
                let i = ((y * 160 + x) * 4) as usize;
                (pixels[i + 2], pixels[i + 1], pixels[i])
            };
            assert_eq!(at(12, 8), (255, 0, 0), "GDI at {dpi} DPI");
            assert_eq!(at(12, 28), (0, 255, 0), "GDI+ at {dpi} DPI");
            assert_eq!(at(32, 28), (255, 255, 255), "no double GDI+ scaling at {dpi} DPI");
            assert_eq!(at(12, 48), (0, 0, 255), "GDI preserved after GDI+ at {dpi} DPI");
        }
    }

    #[test]
    fn clip_limits_drawing() {
        Bitmap::render(20, 20, |canvas| {
            canvas.clear(Color::WHITE);
            let state = canvas.save();
            canvas.clip_rect(Rect::new(0, 0, 10, 20));
            canvas.fill_rect(Rect::new(0, 0, 20, 20), Color::BLACK);
            canvas.restore(state);
            assert_eq!(canvas.pixel(Point::new(5, 5)), Some(Color::BLACK));
            assert_eq!(canvas.pixel(Point::new(15, 5)), Some(Color::WHITE));
        })
        .unwrap();
    }

    #[test]
    fn gdiplus_preserves_canvas_transform_and_clip() {
        for dpi in [96, 144, 192] {
            let (_, pixels) = Bitmap::render_at(100, 100, dpi, Some(Color::WHITE), |canvas| {
                canvas.translate(10.0, 0.0);
                canvas.clip_rect(Rect::new(0, 0, 10, 30));
                canvas.with_gdiplus(|g| g.fill_rect(0.0, 0.0, 30.0, 10.0, Color::rgb(0, 255, 0))).unwrap();
                canvas.fill_rect(Rect::new(0, 15, 30, 10), Color::rgb(0, 0, 255));
            })
            .unwrap();
            let at = |x: i32, y: i32| {
                let x = x * dpi as i32 / 96;
                let y = y * dpi as i32 / 96;
                let i = ((y * 100 + x) * 4) as usize;
                (pixels[i + 2], pixels[i + 1], pixels[i])
            };
            assert_eq!(at(12, 4), (0, 255, 0), "transformed GDI+ at {dpi} DPI");
            assert_eq!(at(5, 4), (255, 255, 255), "before translated origin at {dpi} DPI");
            assert_eq!(at(24, 4), (255, 255, 255), "GDI+ clipping at {dpi} DPI");
            assert_eq!(at(12, 19), (0, 0, 255), "restored GDI transform at {dpi} DPI");
            assert_eq!(at(24, 19), (255, 255, 255), "restored GDI clipping at {dpi} DPI");
        }
    }
}
