//! GDI+ backend: antialiasing, gradients, paths and image decoding.

use super::font::Font;
use super::gdi::{Bitmap, Path, PathSegment};
use crate::core::handles::GdiObject;
use crate::core::{GuiError, GuiResult};
use crate::ffi::gdiplus::*;
use crate::ffi::HDC;
use crate::utilities::{to_wide, Color, Point, Rect, Size};
use std::cell::RefCell;
use std::ffi::c_void;

/// Built-in GDI+ PNG encoder CLSID (`image/png`).
const PNG_ENCODER: crate::ffi::GUID = crate::ffi::GUID {
    data1: 0x557C_F406,
    data2: 0x1A04,
    data3: 0x11D3,
    data4: [0x9A, 0x73, 0x00, 0x00, 0xF8, 0x1E, 0xF3, 0x2E],
};

struct Token(usize);

impl Drop for Token {
    fn drop(&mut self) {
        // SAFETY: token obtido de GdiplusStartup nesta thread.
        unsafe { GdiplusShutdown(self.0) };
    }
}

thread_local! {
    static TOKEN: RefCell<Option<Token>> = const { RefCell::new(None) };
}

fn check(context: &'static str, status: GpStatus) -> GuiResult<()> {
    if status == 0 {
        Ok(())
    } else {
        Err(GuiError::InvalidArgument(format!("{context}: status GDI+ {status}")))
    }
}

/// Initializes GDI+ on this thread at first use.
fn ensure_started() -> GuiResult<()> {
    TOKEN.with(|slot| {
        if slot.borrow().is_some() {
            return Ok(());
        }
        let input = GdiplusStartupInput {
            GdiplusVersion: 1,
            DebugEventCallback: std::ptr::null(),
            SuppressBackgroundThread: 0,
            SuppressExternalCodecs: 0,
        };
        let mut token = 0usize;
        // SAFETY: estrutura de entrada completa e ponteiro de saída válido.
        check("GdiplusStartup", unsafe { GdiplusStartup(&mut token, &input, std::ptr::null_mut()) })?;
        *slot.borrow_mut() = Some(Token(token));
        Ok(())
    })
}

/// Decodes PNG/JPEG/GIF/TIFF/BMP/ICO into a [`Bitmap`].
pub(crate) fn load_bitmap(path: &std::path::Path) -> GuiResult<Bitmap> {
    ensure_started()?;
    let wide = to_wide(&path.to_string_lossy());
    let mut image: *mut c_void = std::ptr::null_mut();
    // SAFETY: string terminada em NUL e ponteiro de saída válido.
    let status = unsafe { GdipCreateBitmapFromFile(wide.as_ptr(), &mut image) };
    if status != 0 || image.is_null() {
        return Err(GuiError::InvalidArgument(format!(
            "could not decode the image {} (GDI+ status {status})",
            path.display()
        )));
    }
    let mut hbitmap = std::ptr::null_mut();
    // SAFETY: `image` é um bitmap GDI+ válido, liberado logo após a conversão.
    let status = unsafe {
        let status = GdipCreateHBITMAPFromBitmap(image, &mut hbitmap, 0x00FF_FFFF);
        GdipDisposeImage(image);
        status
    };
    check("GdipCreateHBITMAPFromBitmap", status)?;
    // SAFETY: HBITMAP recém-criado cuja posse passa ao wrapper.
    let handle = unsafe { GdiObject::from_raw(hbitmap) }
        .ok_or_else(|| GuiError::InvalidArgument("GDI+ returned a null bitmap".into()))?;
    Bitmap::from_handle(handle)
}

/// GDI+-decoded image retaining alpha when drawn.
#[derive(Debug)]
pub struct PlusImage {
    image: *mut c_void,
    size: Size,
    /// Source stream read lazily by GDI+ throughout image lifetime.
    _stream: Option<StreamHandle>,
}

/// Global-memory `IStream` retained with its decoded image.
struct StreamHandle {
    _stream: crate::ffi::ole32::ComPtr,
}

impl std::fmt::Debug for StreamHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("IStream")
    }
}

fn image_size(image: *mut c_void) -> Size {
    let (mut width, mut height) = (0u32, 0u32);
    // SAFETY: `image` é uma imagem GDI+ válida; ponteiros de saída locais.
    unsafe {
        GdipGetImageWidth(image, &mut width);
        GdipGetImageHeight(image, &mut height);
    }
    Size::new(width as i32, height as i32)
}

impl PlusImage {
    /// Loads an image file.
    pub fn from_file(path: impl AsRef<std::path::Path>) -> GuiResult<PlusImage> {
        ensure_started()?;
        let wide = to_wide(&path.as_ref().to_string_lossy());
        let mut image: *mut c_void = std::ptr::null_mut();
        // SAFETY: string terminada em NUL e ponteiro de saída válido.
        let status = unsafe { GdipLoadImageFromFile(wide.as_ptr(), &mut image) };
        if status != 0 || image.is_null() {
            return Err(GuiError::InvalidArgument(format!(
                "could not load the image {} (GDI+ status {status})",
                path.as_ref().display()
            )));
        }
        Ok(PlusImage { image, size: image_size(image), _stream: None })
    }

    /// Decodes in-memory PNG/JPEG/GIF/TIFF/BMP/ICO.
    pub fn from_bytes(bytes: &[u8]) -> GuiResult<PlusImage> {
        ensure_started()?;
        if bytes.is_empty() {
            return Err(GuiError::InvalidArgument("empty image".into()));
        }
        let memory = crate::core::GlobalMemory::with_bytes(bytes)?;
        let mut raw_stream: *mut c_void = std::ptr::null_mut();
        // SAFETY: o bloco global passa a pertencer ao fluxo (`delete_on_release`); só
        // depois do sucesso a posse é transferida com `into_raw`.
        let hr = unsafe { crate::ffi::ole32::CreateStreamOnHGlobal(memory.raw(), 1, &mut raw_stream) };
        if hr < 0 {
            return Err(GuiError::HResult { context: "CreateStreamOnHGlobal", hresult: hr });
        }
        let _ = memory.into_raw();
        // SAFETY: referência recém-criada, possuída a partir de agora pelo `ComPtr`.
        let stream = unsafe { crate::ffi::ole32::ComPtr::from_raw(raw_stream) }
            .ok_or_else(|| GuiError::InvalidArgument("CreateStreamOnHGlobal returned null".into()))?;
        let mut image: *mut c_void = std::ptr::null_mut();
        // SAFETY: fluxo válido; o GDI+ o lê durante a vida da imagem, por isso ele é
        // guardado junto com ela.
        let status = unsafe { GdipCreateBitmapFromStream(stream.as_ptr(), &mut image) };
        if status != 0 || image.is_null() {
            return Err(GuiError::InvalidArgument(format!(
                "could not decode the in-memory image (GDI+ status {status})"
            )));
        }
        Ok(PlusImage { image, size: image_size(image), _stream: Some(StreamHandle { _stream: stream }) })
    }

    /// Pixel size.
    pub fn size(&self) -> Size {
        self.size
    }

    /// Encodes PNG with transparency using GDI+.
    pub fn to_png(&self) -> GuiResult<Vec<u8>> {
        let mut raw_stream: *mut c_void = std::ptr::null_mut();
        // SAFETY: fluxo em memória nova (HGLOBAL nulo), liberado com o `ComPtr`.
        let hr = unsafe { crate::ffi::ole32::CreateStreamOnHGlobal(std::ptr::null_mut(), 1, &mut raw_stream) };
        if hr < 0 {
            return Err(GuiError::HResult { context: "CreateStreamOnHGlobal", hresult: hr });
        }
        // SAFETY: referência recém-criada, possuída a partir de agora pelo `ComPtr`.
        let stream = unsafe { crate::ffi::ole32::ComPtr::from_raw(raw_stream) }
            .ok_or_else(|| GuiError::InvalidArgument("CreateStreamOnHGlobal returned null".into()))?;
        // SAFETY: imagem e fluxo válidos; o CLSID vive durante a chamada.
        check("GdipSaveImageToStream", unsafe {
            GdipSaveImageToStream(self.image, stream.as_ptr(), &PNG_ENCODER, std::ptr::null())
        })?;
        // Tamanho real: posição do fluxo depois da gravação (o bloco pode ter sobra).
        type Seek = unsafe extern "system" fn(*mut c_void, i64, u32, *mut u64) -> i32;
        let mut written = 0u64;
        // SAFETY: IStream::Seek é o 6º método (índice 5) da vtable; STREAM_SEEK_CUR = 1.
        let hr = unsafe { stream.method::<Seek>(5)(stream.as_ptr(), 0, 1, &mut written) };
        if hr < 0 {
            return Err(GuiError::HResult { context: "IStream::Seek", hresult: hr });
        }
        let mut memory: crate::ffi::HANDLE = std::ptr::null_mut();
        // SAFETY: o bloco continua pertencendo ao fluxo; só é lido enquanto ele vive.
        unsafe {
            let hr = crate::ffi::ole32::GetHGlobalFromStream(stream.as_ptr(), &mut memory);
            if hr < 0 {
                return Err(GuiError::HResult { context: "GetHGlobalFromStream", hresult: hr });
            }
            let mut bytes = crate::system::clipboard::global_bytes(memory)?;
            bytes.truncate(written as usize);
            Ok(bytes)
        }
    }

    /// Writes PNG to file.
    pub fn save_png(&self, path: impl AsRef<std::path::Path>) -> GuiResult<()> {
        let bytes = self.to_png()?;
        std::fs::write(path.as_ref(), bytes)
            .map_err(|e| GuiError::InvalidArgument(format!("could not write {}: {e}", path.as_ref().display())))
    }

    /// Creates an image from BGRA bytes (blue, green, red, alpha; 4 bytes/pixel, top-down, straight alpha).
    pub fn from_pixels(width: i32, height: i32, bgra: &[u8]) -> GuiResult<PlusImage> {
        ensure_started()?;
        if width <= 0 || height <= 0 || bgra.len() != width as usize * height as usize * 4 {
            return Err(GuiError::InvalidArgument("pixel buffer size does not match the image".into()));
        }
        let mut image: *mut c_void = std::ptr::null_mut();
        // SAFETY: bitmap alocado pelo próprio GDI+ (scan0 nulo); ponteiro de saída válido.
        check("GdipCreateBitmapFromScan0", unsafe {
            GdipCreateBitmapFromScan0(width, height, 0, PIXEL_FORMAT_32BPP_ARGB, std::ptr::null_mut(), &mut image)
        })?;
        let result = PlusImage { image, size: Size::new(width, height), _stream: None };
        let mut copy = bgra.to_vec();
        result.lock(IMAGE_LOCK_MODE_WRITE, &mut copy)?;
        Ok(result)
    }

    /// Image BGRA bytes (4 bytes/pixel, top-down, straight alpha).
    pub fn to_pixels(&self) -> GuiResult<Vec<u8>> {
        let mut pixels = vec![0u8; self.size.width.max(0) as usize * self.size.height.max(0) as usize * 4];
        self.lock(IMAGE_LOCK_MODE_READ, &mut pixels)?;
        Ok(pixels)
    }

    /// Copies pixels between `buffer` and image with `LockBits`, reading/writing.
    fn lock(&self, mode: u32, buffer: &mut [u8]) -> GuiResult<()> {
        let rect = GpRect { X: 0, Y: 0, Width: self.size.width, Height: self.size.height };
        let mut data = BitmapData {
            Width: self.size.width as u32,
            Height: self.size.height as u32,
            Stride: self.size.width * 4,
            PixelFormat: PIXEL_FORMAT_32BPP_ARGB,
            Scan0: buffer.as_mut_ptr(),
            Reserved: 0,
        };
        // SAFETY: `buffer` tem exatamente largura × altura × 4 bytes e vive durante o par
        // LockBits/UnlockBits; com USER_INPUT_BUF o GDI+ copia de/para ele.
        unsafe {
            check(
                "GdipBitmapLockBits",
                GdipBitmapLockBits(
                    self.image,
                    &rect,
                    mode | IMAGE_LOCK_MODE_USER_INPUT_BUF,
                    PIXEL_FORMAT_32BPP_ARGB,
                    &mut data,
                ),
            )?;
            check("GdipBitmapUnlockBits", GdipBitmapUnlockBits(self.image, &mut data))
        }
    }

    /// Converts to `size`×`size` icon using high-quality bicubic interpolation, preserving aspect ratio and centering.
    pub fn to_icon(&self, size: i32) -> GuiResult<crate::core::Icon> {
        if size <= 0 {
            return Err(GuiError::InvalidArgument("the icon size must be positive".into()));
        }
        let mut target: *mut c_void = std::ptr::null_mut();
        // SAFETY: bitmap ARGB alocado pelo próprio GDI+ (scan0 nulo).
        check("GdipCreateBitmapFromScan0", unsafe {
            GdipCreateBitmapFromScan0(size, size, 0, PIXEL_FORMAT_32BPP_ARGB, std::ptr::null_mut(), &mut target)
        })?;
        struct Owned(*mut c_void);
        impl Drop for Owned {
            fn drop(&mut self) {
                // SAFETY: bitmap criado acima e liberado uma única vez.
                unsafe { GdipDisposeImage(self.0) };
            }
        }
        let target = Owned(target);
        let scale = size as f32 / self.size.width.max(self.size.height).max(1) as f32;
        let (width, height) = (self.size.width as f32 * scale, self.size.height as f32 * scale);
        let mut graphics: *mut c_void = std::ptr::null_mut();
        // SAFETY: contexto obtido do bitmap de destino e liberado antes de convertê-lo.
        unsafe {
            check("GdipGetImageGraphicsContext", GdipGetImageGraphicsContext(target.0, &mut graphics))?;
            GdipSetInterpolationMode(graphics, INTERPOLATION_MODE_HIGH_QUALITY_BICUBIC);
            GdipSetPixelOffsetMode(graphics, PIXEL_OFFSET_MODE_HIGH_QUALITY);
            let status = GdipDrawImageRect(
                graphics,
                self.image,
                (size as f32 - width) / 2.0,
                (size as f32 - height) / 2.0,
                width,
                height,
            );
            GdipDeleteGraphics(graphics);
            check("GdipDrawImageRect", status)?;
        }
        let mut icon = std::ptr::null_mut();
        // SAFETY: bitmap válido; o HICON criado passa a pertencer ao `Icon`.
        check("GdipCreateHICONFromBitmap", unsafe { GdipCreateHICONFromBitmap(target.0, &mut icon) })?;
        // SAFETY: ícone recém-criado e exclusivo.
        unsafe { crate::core::Icon::from_raw_owned(icon) }
            .ok_or_else(|| GuiError::InvalidArgument("GDI+ returned a null icon".into()))
    }
}

impl Drop for PlusImage {
    fn drop(&mut self) {
        // SAFETY: imagem criada por este objeto e ainda não liberada.
        unsafe { GdipDisposeImage(self.image) };
    }
}

/// GDI+ drawing context with antialiasing enabled.
///
/// From [`Canvas::with_gdiplus`](super::Canvas::with_gdiplus). Logical (96 DPI) coordinates accept fractions.
#[derive(Debug)]
pub struct PlusGraphics {
    graphics: *mut c_void,
}

impl PlusGraphics {
    pub(crate) fn from_hdc(hdc: HDC, dpi: u32) -> GuiResult<PlusGraphics> {
        ensure_started()?;
        let mut graphics: *mut c_void = std::ptr::null_mut();
        // SAFETY: `hdc` é um DC válido durante a vida do objeto; ponteiro de saída válido.
        check("GdipCreateFromHDC", unsafe { GdipCreateFromHDC(hdc, &mut graphics) })?;
        let scale = dpi as f32 / 96.0;
        // SAFETY: `graphics` acabou de ser criado com sucesso.
        unsafe {
            GdipSetSmoothingMode(graphics, SMOOTHING_MODE_ANTIALIAS);
            GdipSetTextRenderingHint(graphics, TEXT_RENDERING_HINT_CLEARTYPE);
            if dpi != 96 {
                GdipScaleWorldTransform(graphics, scale, scale, MATRIX_ORDER_PREPEND);
            }
        }
        Ok(PlusGraphics { graphics })
    }

    /// Antialiased image drawing context in pixels, without DPI scaling.
    pub(crate) fn for_image(image: &PlusImage) -> GuiResult<PlusGraphics> {
        ensure_started()?;
        let mut graphics: *mut c_void = std::ptr::null_mut();
        // SAFETY: imagem válida, que vive mais que o contexto (o chamador o apaga antes).
        check("GdipGetImageGraphicsContext", unsafe { GdipGetImageGraphicsContext(image.image, &mut graphics) })?;
        // SAFETY: contexto recém-criado.
        unsafe {
            GdipSetSmoothingMode(graphics, SMOOTHING_MODE_ANTIALIAS);
            GdipSetTextRenderingHint(graphics, TEXT_RENDERING_HINT_CLEARTYPE);
            GdipSetPixelOffsetMode(graphics, PIXEL_OFFSET_MODE_HIGH_QUALITY);
        }
        Ok(PlusGraphics { graphics })
    }

    pub(crate) fn raw(&self) -> *mut c_void {
        self.graphics
    }

    /// Fills floating-point curved figures with solid/gradient paint. `even_odd` selects even-odd filling, creating
    /// holes in overlapping figures.
    pub fn fill_figures(&self, figures: &[super::Figure], even_odd: bool, paint: &super::Paint) {
        super::vector::VectorPainter { graphics: self }.fill(figures, even_odd, paint, 1.0);
    }

    /// Strokes figures with the given stroke and paint.
    pub fn stroke_figures(&self, figures: &[super::Figure], stroke: &super::Stroke, paint: &super::Paint) {
        super::vector::VectorPainter { graphics: self }.stroke(figures, stroke, paint, 1.0);
    }

    /// Draws with `matrix` appended to current transform, restoring afterward.
    pub fn with_matrix(&self, matrix: &super::Matrix, draw: impl FnOnce(&PlusGraphics)) {
        super::vector::VectorPainter { graphics: self }.with_transform(matrix, || draw(self));
    }

    fn with_pen(&self, color: Color, width: f32, draw: impl FnOnce(*mut c_void)) {
        let mut pen: *mut c_void = std::ptr::null_mut();
        // SAFETY: a caneta é criada, usada e apagada neste bloco.
        unsafe {
            if GdipCreatePen1(color.to_argb(), width, UNIT_PIXEL, &mut pen) == 0 {
                draw(pen);
                GdipDeletePen(pen);
            }
        }
    }

    fn with_brush(&self, color: Color, draw: impl FnOnce(*mut c_void)) {
        let mut brush: *mut c_void = std::ptr::null_mut();
        // SAFETY: o pincel é criado, usado e apagado neste bloco.
        unsafe {
            if GdipCreateSolidFill(color.to_argb(), &mut brush) == 0 {
                draw(brush);
                GdipDeleteBrush(brush);
            }
        }
    }

    /// Fills the whole surface.
    pub fn clear(&self, color: Color) {
        // SAFETY: contexto válido.
        unsafe { GdipGraphicsClear(self.graphics, color.to_argb()) };
    }

    /// Antialiased line.
    pub fn line(&self, x1: f32, y1: f32, x2: f32, y2: f32, color: Color, width: f32) {
        // SAFETY: contexto e caneta válidos dentro da closure.
        self.with_pen(color, width, |pen| unsafe {
            GdipDrawLine(self.graphics, pen, x1, y1, x2, y2);
        });
    }

    /// Rectangle outline.
    pub fn draw_rect(&self, x: f32, y: f32, w: f32, h: f32, color: Color, width: f32) {
        // SAFETY: contexto e caneta válidos dentro da closure.
        self.with_pen(color, width, |pen| unsafe {
            GdipDrawRectangle(self.graphics, pen, x, y, w, h);
        });
    }

    /// Filled rectangle accepting translucent colors.
    pub fn fill_rect(&self, x: f32, y: f32, w: f32, h: f32, color: Color) {
        // SAFETY: contexto e pincel válidos dentro da closure.
        self.with_brush(color, |brush| unsafe {
            GdipFillRectangle(self.graphics, brush, x, y, w, h);
        });
    }

    /// Ellipse outline.
    pub fn draw_ellipse(&self, x: f32, y: f32, w: f32, h: f32, color: Color, width: f32) {
        // SAFETY: contexto e caneta válidos dentro da closure.
        self.with_pen(color, width, |pen| unsafe {
            GdipDrawEllipse(self.graphics, pen, x, y, w, h);
        });
    }

    /// Filled ellipse.
    pub fn fill_ellipse(&self, x: f32, y: f32, w: f32, h: f32, color: Color) {
        // SAFETY: contexto e pincel válidos dentro da closure.
        self.with_brush(color, |brush| unsafe {
            GdipFillEllipse(self.graphics, brush, x, y, w, h);
        });
    }

    /// Filled polygon.
    pub fn fill_polygon(&self, points: &[(f32, f32)], color: Color) {
        let native: Vec<PointF> = points.iter().map(|&(x, y)| PointF { x, y }).collect();
        // SAFETY: `native` tem exatamente `len` pontos.
        self.with_brush(color, |brush| unsafe {
            GdipFillPolygon(self.graphics, brush, native.as_ptr(), native.len() as i32, FILL_MODE_ALTERNATE);
        });
    }

    /// Polygon outline.
    pub fn draw_polygon(&self, points: &[(f32, f32)], color: Color, width: f32) {
        let native: Vec<PointF> = points.iter().map(|&(x, y)| PointF { x, y }).collect();
        // SAFETY: `native` tem exatamente `len` pontos.
        self.with_pen(color, width, |pen| unsafe {
            GdipDrawPolygon(self.graphics, pen, native.as_ptr(), native.len() as i32);
        });
    }

    fn with_path(&self, path: &Path, draw: impl FnOnce(*mut c_void)) {
        let mut native: *mut c_void = std::ptr::null_mut();
        // SAFETY: o caminho GDI+ é criado, preenchido, usado e apagado neste bloco.
        unsafe {
            if GdipCreatePath(FILL_MODE_ALTERNATE, &mut native) != 0 {
                return;
            }
            let mut current = Point::default();
            for segment in path.segments() {
                match segment {
                    PathSegment::MoveTo(p) => {
                        current = p;
                    }
                    PathSegment::LineTo(p) => {
                        GdipAddPathLine(native, current.x as f32, current.y as f32, p.x as f32, p.y as f32);
                        current = p;
                    }
                    PathSegment::BezierTo(a, b, c) => {
                        GdipAddPathBezier(
                            native,
                            current.x as f32,
                            current.y as f32,
                            a.x as f32,
                            a.y as f32,
                            b.x as f32,
                            b.y as f32,
                            c.x as f32,
                            c.y as f32,
                        );
                        current = c;
                    }
                    PathSegment::Close => {
                        GdipClosePathFigure(native);
                    }
                }
            }
            draw(native);
            GdipDeletePath(native);
        }
    }

    /// Fills a vector path.
    pub fn fill_path(&self, path: &Path, color: Color) {
        self.with_path(path, |native| {
            // SAFETY: contexto, pincel e caminho válidos.
            self.with_brush(color, |brush| unsafe {
                GdipFillPath(self.graphics, brush, native);
            });
        });
    }

    /// Strokes a vector path.
    pub fn draw_path(&self, path: &Path, color: Color, width: f32) {
        self.with_path(path, |native| {
            // SAFETY: contexto, caneta e caminho válidos.
            self.with_pen(color, width, |pen| unsafe {
                GdipDrawPath(self.graphics, pen, native);
            });
        });
    }

    /// Fills a rectangle with a linear gradient between two points.
    pub fn gradient_rect(&self, rect: Rect, start: (f32, f32), end: (f32, f32), from: Color, to: Color) {
        let (p1, p2) = (PointF { x: start.0, y: start.1 }, PointF { x: end.0, y: end.1 });
        let mut brush: *mut c_void = std::ptr::null_mut();
        // SAFETY: o pincel de gradiente é criado, usado e apagado neste bloco.
        unsafe {
            if GdipCreateLineBrush(&p1, &p2, from.to_argb(), to.to_argb(), WRAP_MODE_TILE, &mut brush) == 0 {
                GdipFillRectangle(
                    self.graphics,
                    brush,
                    rect.x as f32,
                    rect.y as f32,
                    rect.width as f32,
                    rect.height as f32,
                );
                GdipDeleteBrush(brush);
            }
        }
    }

    /// Draws an image retaining transparency.
    pub fn draw_image(&self, image: &PlusImage, dest: Rect) {
        // SAFETY: contexto e imagem válidos.
        unsafe {
            GdipDrawImageRect(
                self.graphics,
                image.image,
                dest.x as f32,
                dest.y as f32,
                dest.width as f32,
                dest.height as f32,
            )
        };
    }

    /// Draws antialiased text inside the rectangle.
    pub fn text(&self, text: &str, rect: Rect, font: &Font, color: Color) {
        let family_name = to_wide(font.family());
        let wide: Vec<u16> = text.encode_utf16().collect();
        let style = font.is_bold() as i32
            | (font.is_italic() as i32) << 1
            | (font.is_underline() as i32) << 2
            | (font.is_strikeout() as i32) << 3;
        let layout = RectF { x: rect.x as f32, y: rect.y as f32, width: rect.width as f32, height: rect.height as f32 };
        // Em unidades lógicas de 96 DPI, 1 pt = 96/72 px.
        let pixel_size = font.size() * 96.0 / 72.0;
        // SAFETY: família e fonte GDI+ são criadas, usadas e apagadas neste bloco; as
        // strings têm o comprimento informado.
        unsafe {
            let mut family: *mut c_void = std::ptr::null_mut();
            if GdipCreateFontFamilyFromName(family_name.as_ptr(), std::ptr::null_mut(), &mut family) != 0 {
                return;
            }
            let mut native_font: *mut c_void = std::ptr::null_mut();
            if GdipCreateFont(family, pixel_size, style, UNIT_PIXEL, &mut native_font) == 0 {
                self.with_brush(color, |brush| {
                    GdipDrawString(
                        self.graphics,
                        wide.as_ptr(),
                        wide.len() as i32,
                        native_font,
                        &layout,
                        std::ptr::null_mut(),
                        brush,
                    );
                });
                GdipDeleteFont(native_font);
            }
            GdipDeleteFontFamily(family);
        }
    }

    /// Translates origin.
    pub fn translate(&self, dx: f32, dy: f32) {
        // SAFETY: contexto válido.
        unsafe { GdipTranslateWorldTransform(self.graphics, dx, dy, MATRIX_ORDER_PREPEND) };
    }

    /// Rotates around origin in degrees.
    pub fn rotate(&self, degrees: f32) {
        // SAFETY: contexto válido.
        unsafe { GdipRotateWorldTransform(self.graphics, degrees, MATRIX_ORDER_PREPEND) };
    }

    /// Applies scaling.
    pub fn scale(&self, sx: f32, sy: f32) {
        // SAFETY: contexto válido.
        unsafe { GdipScaleWorldTransform(self.graphics, sx, sy, MATRIX_ORDER_PREPEND) };
    }

    /// Clips drawing to the rectangle.
    pub fn clip_rect(&self, rect: Rect) {
        // SAFETY: contexto válido.
        unsafe {
            GdipSetClipRect(
                self.graphics,
                rect.x as f32,
                rect.y as f32,
                rect.width as f32,
                rect.height as f32,
                COMBINE_MODE_INTERSECT,
            )
        };
    }

    /// Removes clipping.
    pub fn reset_clip(&self) {
        // SAFETY: contexto válido.
        unsafe { GdipResetClip(self.graphics) };
    }
}

impl Drop for PlusGraphics {
    fn drop(&mut self) {
        // SAFETY: contexto criado por este objeto.
        unsafe { GdipDeleteGraphics(self.graphics) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn antialiased_fill_reaches_bitmap() {
        Bitmap::render(32, 32, |canvas| {
            canvas.clear(Color::WHITE);
            canvas
                .with_gdiplus(|g| {
                    g.fill_ellipse(4.0, 4.0, 24.0, 24.0, Color::rgb(200, 0, 0));
                    g.line(0.0, 0.0, 31.0, 0.0, Color::BLACK, 1.0);
                })
                .expect("gdi+");
            let center = canvas.pixel(Point::new(16, 16)).unwrap();
            assert!(center.r > 150 && center.g < 60, "centro inesperado: {center:?}");
            assert_eq!(canvas.pixel(Point::new(1, 30)), Some(Color::WHITE));
        })
        .unwrap();
    }

    #[test]
    fn missing_image_is_an_error() {
        assert!(PlusImage::from_file("Z:\\nao-existe.png").is_err());
        assert!(load_bitmap(std::path::Path::new("Z:\\nao-existe.png")).is_err());
    }
}
