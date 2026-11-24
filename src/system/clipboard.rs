//! Clipboard: text, HTML, images, files, custom formats.
//!
//! Pure DIB/HTML Format/CF_HDROP conversion functions also used by drag/drop.

use crate::core::handles::GlobalMemory;
use crate::core::{GuiError, GuiResult};
use crate::ffi::kernel32::{GlobalLock, GlobalSize, GlobalUnlock};
use crate::ffi::shell32::DragQueryFileW;
use crate::ffi::user32::*;
use crate::ffi::HANDLE;
use crate::graphics::PlusImage;
use crate::utilities::{from_wide, to_wide};
use std::path::{Path, PathBuf};

/// Keeps clipboard open, closing on drop.
struct ClipboardGuard;

impl ClipboardGuard {
    fn open() -> GuiResult<ClipboardGuard> {
        // Outro processo pode estar com o clipboard aberto por um instante; tenta de novo.
        for attempt in 0..10 {
            // SAFETY: abre o clipboard para a tarefa atual; pareado com CloseClipboard no Drop.
            if unsafe { OpenClipboard(std::ptr::null_mut()) } != 0 {
                return Ok(ClipboardGuard);
            }
            if attempt < 9 {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
        Err(GuiError::last_os_error("OpenClipboard"))
    }

    /// Writes `format` block; clipboard must be open/empty.
    fn put(&self, format: u32, bytes: &[u8]) -> GuiResult<()> {
        let memory = GlobalMemory::with_bytes(bytes)?;
        // SAFETY: clipboard aberto por esta thread; em caso de sucesso o sistema passa a
        // ser o dono do bloco de memória.
        if unsafe { SetClipboardData(format, memory.raw()) }.is_null() {
            return Err(GuiError::last_os_error("SetClipboardData"));
        }
        let _ = memory.into_raw();
        Ok(())
    }

    /// Copies `format` bytes, if available.
    fn get(&self, format: u32) -> GuiResult<Option<Vec<u8>>> {
        // SAFETY: clipboard aberto; o bloco pertence ao sistema e só é lido enquanto
        // está travado, respeitando o tamanho informado por GlobalSize.
        unsafe {
            let handle = GetClipboardData(format);
            if handle.is_null() {
                return Ok(None);
            }
            Ok(Some(global_bytes(handle)?))
        }
    }
}

impl Drop for ClipboardGuard {
    fn drop(&mut self) {
        // SAFETY: par do OpenClipboard bem-sucedido.
        unsafe { CloseClipboard() };
    }
}

/// Copies `HGLOBAL` content.
///
/// # Safety
/// `handle` must be a valid global block during the call.
pub(crate) unsafe fn global_bytes(handle: HANDLE) -> GuiResult<Vec<u8>> {
    let ptr = GlobalLock(handle) as *const u8;
    if ptr.is_null() {
        return Err(GuiError::last_os_error("GlobalLock"));
    }
    let bytes = std::slice::from_raw_parts(ptr, GlobalSize(handle)).to_vec();
    GlobalUnlock(handle);
    Ok(bytes)
}

/// Named format identifier (`RegisterClipboardFormatW`).
pub(crate) fn format_id(name: &str) -> GuiResult<u32> {
    let wide = to_wide(name);
    // SAFETY: string terminada em NUL válida durante a chamada.
    let id = unsafe { RegisterClipboardFormatW(wide.as_ptr()) };
    if id == 0 {
        return Err(GuiError::last_os_error("RegisterClipboardFormatW"));
    }
    Ok(id)
}

/// Standard Windows HTML format name.
pub(crate) const HTML_FORMAT: &str = "HTML Format";
/// Registered PNG format name used by browsers/Office.
pub(crate) const PNG_FORMAT: &str = "PNG";

/// Text bytes as NUL-terminated UTF-16.
pub(crate) fn encode_text(text: &str) -> Vec<u8> {
    to_wide(text).iter().flat_map(|unit| unit.to_le_bytes()).collect()
}

/// UTF-16 block text up to first NUL.
pub(crate) fn decode_text(bytes: &[u8]) -> String {
    let units: Vec<u16> = bytes.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    from_wide(&units)
}

/// Builds HTML Format block with UTF-8 byte offsets.
pub(crate) fn encode_html(fragment: &str) -> Vec<u8> {
    const PREFIX: &str = "<html><body>\r\n<!--StartFragment-->";
    const SUFFIX: &str = "<!--EndFragment-->\r\n</body></html>";
    let header_len = "Version:0.9\r\nStartHTML:0000000000\r\nEndHTML:0000000000\r\nStartFragment:0000000000\r\nEndFragment:0000000000\r\n".len();
    let start_html = header_len;
    let start_fragment = start_html + PREFIX.len();
    let end_fragment = start_fragment + fragment.len();
    let end_html = end_fragment + SUFFIX.len();
    let mut text = format!(
        "Version:0.9\r\nStartHTML:{start_html:010}\r\nEndHTML:{end_html:010}\r\nStartFragment:{start_fragment:010}\r\nEndFragment:{end_fragment:010}\r\n"
    );
    text.push_str(PREFIX);
    text.push_str(fragment);
    text.push_str(SUFFIX);
    let mut bytes = text.into_bytes();
    bytes.push(0);
    bytes
}

/// Extracts HTML fragment, or document without markers.
pub(crate) fn decode_html(bytes: &[u8]) -> Option<String> {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    let bytes = &bytes[..end];
    let header = String::from_utf8_lossy(&bytes[..bytes.len().min(512)]).into_owned();
    let field = |name: &str| -> Option<usize> {
        let start = header.find(name)? + name.len();
        let value: String = header[start..].chars().take_while(|c| c.is_ascii_digit() || *c == '-').collect();
        value.parse::<i64>().ok().filter(|v| *v >= 0).map(|v| v as usize)
    };
    let (from, to) = match (field("StartFragment:"), field("EndFragment:")) {
        (Some(from), Some(to)) => (from, to),
        _ => (field("StartHTML:")?, field("EndHTML:").unwrap_or(bytes.len())),
    };
    let slice = bytes.get(from..to.min(bytes.len()))?;
    Some(String::from_utf8_lossy(slice).into_owned())
}

/// Builds 32-bit DIB (`BITMAPINFOHEADER` plus bottom-up rows) from top-down BGRA.
pub(crate) fn encode_dib(width: i32, height: i32, bgra: &[u8]) -> Vec<u8> {
    let row = width as usize * 4;
    let mut bytes = Vec::with_capacity(40 + bgra.len());
    bytes.extend_from_slice(&40u32.to_le_bytes()); // tamanho do cabeçalho
    bytes.extend_from_slice(&width.to_le_bytes());
    bytes.extend_from_slice(&height.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes()); // planos
    bytes.extend_from_slice(&32u16.to_le_bytes()); // bits por pixel
    bytes.extend_from_slice(&0u32.to_le_bytes()); // BI_RGB
    bytes.extend_from_slice(&(bgra.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&[0u8; 16]); // resolução e paleta
    for y in (0..height as usize).rev() {
        bytes.extend_from_slice(&bgra[y * row..(y + 1) * row]);
    }
    bytes
}

/// Reads 24/32-bit `CF_DIB`/`CF_DIBV5` with/without masks (`BI_BITFIELDS`/`BI_ALPHABITFIELDS`, V4/V5 headers). Returns
/// width, height, top-down BGRA.
pub(crate) fn decode_dib(bytes: &[u8]) -> Option<(i32, i32, Vec<u8>)> {
    let u32_at = |at: usize| bytes.get(at..at + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
    let header = u32_at(0)? as usize;
    let width = u32_at(4)? as i32;
    let raw_height = u32_at(8)? as i32;
    let bits = u16::from_le_bytes([*bytes.get(14)?, *bytes.get(15)?]);
    let compression = u32_at(16)?;
    let colors = u32_at(32).unwrap_or(0) as usize;
    const BI_RGB: u32 = 0;
    const BI_BITFIELDS: u32 = 3;
    const BI_ALPHABITFIELDS: u32 = 6;
    if width <= 0
        || raw_height == 0
        || header < 40
        || !(bits == 24 || bits == 32)
        || !matches!(compression, BI_RGB | BI_BITFIELDS | BI_ALPHABITFIELDS)
    {
        return None;
    }
    // Máscaras: dentro do cabeçalho (V4/V5) ou logo depois dele (cabeçalho de 40 bytes).
    let after_header = |alpha: bool| {
        if header == 40 {
            if alpha {
                16
            } else {
                12
            }
        } else {
            0
        }
    };
    let (masks, extra) = match compression {
        BI_BITFIELDS | BI_ALPHABITFIELDS if bits == 32 => {
            let alpha = compression == BI_ALPHABITFIELDS || header >= 56;
            let masks = [u32_at(40)?, u32_at(44)?, u32_at(48)?, if alpha { u32_at(52)? } else { 0 }];
            (Some(masks), after_header(compression == BI_ALPHABITFIELDS))
        }
        BI_BITFIELDS | BI_ALPHABITFIELDS => (None, after_header(compression == BI_ALPHABITFIELDS)),
        _ => (None, 0),
    };
    let start = header + extra + colors * 4;
    let height = raw_height.abs();
    let bytes_per_pixel = bits as usize / 8;
    let stride = (width as usize * bytes_per_pixel).div_ceil(4) * 4;
    let channel = |value: u32, mask: u32| -> Option<u8> {
        if mask == 0 {
            return None;
        }
        let shift = mask.trailing_zeros();
        let max = (mask >> shift) as u64;
        Some(((((value & mask) >> shift) as u64) * 255 / max.max(1)) as u8)
    };
    let mut pixels = vec![0u8; width as usize * height as usize * 4];
    let mut any_alpha = false;
    for y in 0..height as usize {
        let source_row = if raw_height > 0 { height as usize - 1 - y } else { y };
        let row =
            bytes.get(start + source_row * stride..start + source_row * stride + width as usize * bytes_per_pixel)?;
        for x in 0..width as usize {
            let px = &row[x * bytes_per_pixel..];
            let out = &mut pixels[(y * width as usize + x) * 4..][..4];
            match masks {
                Some([r, g, b, a]) => {
                    let value = u32::from_le_bytes([px[0], px[1], px[2], px[3]]);
                    out[0] = channel(value, b).unwrap_or(0);
                    out[1] = channel(value, g).unwrap_or(0);
                    out[2] = channel(value, r).unwrap_or(0);
                    out[3] = channel(value, a).unwrap_or(255);
                }
                None => {
                    out[..3].copy_from_slice(&px[..3]);
                    out[3] = if bits == 32 { px[3] } else { 255 };
                }
            }
            any_alpha |= out[3] != 0;
        }
    }
    if bits == 32 && !any_alpha {
        // Muitos programas gravam 32 bits com o alfa zerado: trata como opaco.
        pixels.chunks_exact_mut(4).for_each(|p| p[3] = 255);
    }
    Some((width, height, pixels))
}

/// Builds top-down 32-bit DIB V5 with explicit BGRA masks so CF_DIBV5 readers recognize alpha.
pub(crate) fn encode_dibv5(width: i32, height: i32, bgra: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(124 + bgra.len());
    bytes.extend_from_slice(&124u32.to_le_bytes());
    bytes.extend_from_slice(&width.to_le_bytes());
    bytes.extend_from_slice(&(-height).to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&32u16.to_le_bytes());
    bytes.extend_from_slice(&3u32.to_le_bytes()); // BI_BITFIELDS
    bytes.extend_from_slice(&(bgra.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&[0u8; 16]); // resolução e paleta
    for mask in [0x00FF_0000u32, 0x0000_FF00, 0x0000_00FF, 0xFF00_0000] {
        bytes.extend_from_slice(&mask.to_le_bytes());
    }
    bytes.extend_from_slice(&0x7352_4742u32.to_le_bytes()); // LCS_sRGB
    bytes.resize(124, 0);
    bytes[108..112].copy_from_slice(&4u32.to_le_bytes()); // LCS_GM_IMAGES
    bytes.extend_from_slice(bgra);
    bytes
}

/// EMF dimensions: reference device pixel bounds (`rclBounds`) and hundredth-millimeter frame (`rclFrame`).
pub(crate) fn emf_dimensions(bytes: &[u8]) -> Option<((i32, i32), (i32, i32))> {
    let i32_at = |at: usize| bytes.get(at..at + 4).map(|b| i32::from_le_bytes([b[0], b[1], b[2], b[3]]));
    // EMR_HEADER (tipo 1) com a assinatura " EMF" no deslocamento 40.
    if i32_at(0)? != 1 || bytes.get(40..44)? != b" EMF" {
        return None;
    }
    let bounds = (i32_at(16)? - i32_at(8)? + 1, i32_at(20)? - i32_at(12)? + 1);
    let frame = (i32_at(32)? - i32_at(24)?, i32_at(36)? - i32_at(28)?);
    Some((bounds, frame))
}

/// Builds `CF_HDROP`: `DROPFILES` and double-NUL-terminated UTF-16 paths.
pub(crate) fn encode_hdrop(files: &[PathBuf]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&20u32.to_le_bytes()); // pFiles: deslocamento da lista
    bytes.extend_from_slice(&[0u8; 8]); // pt
    bytes.extend_from_slice(&0u32.to_le_bytes()); // fNC
    bytes.extend_from_slice(&1u32.to_le_bytes()); // fWide
    for file in files {
        for unit in file.to_string_lossy().encode_utf16() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
        bytes.extend_from_slice(&[0, 0]);
    }
    bytes.extend_from_slice(&[0, 0]);
    bytes
}

/// Reads copied `CF_HDROP` paths, Unicode/ANSI.
pub(crate) fn decode_hdrop(bytes: &[u8]) -> Vec<PathBuf> {
    let read = |at: usize| bytes.get(at..at + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
    let (Some(offset), Some(wide)) = (read(0), read(16)) else { return Vec::new() };
    let Some(list) = bytes.get(offset as usize..) else { return Vec::new() };
    let mut files = Vec::new();
    if wide != 0 {
        let units: Vec<u16> = list.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        for name in units.split(|&u| u == 0) {
            if name.is_empty() {
                break;
            }
            files.push(PathBuf::from(String::from_utf16_lossy(name)));
        }
    } else {
        for name in list.split(|&b| b == 0) {
            if name.is_empty() {
                break;
            }
            files.push(PathBuf::from(String::from_utf8_lossy(name).into_owned()));
        }
    }
    files
}

/// Reads `HDROP` paths.
///
/// # Safety
/// `drop` must be a valid `HDROP` during the call.
pub(crate) unsafe fn hdrop_files(drop: HANDLE) -> Vec<PathBuf> {
    let count = DragQueryFileW(drop, 0xFFFF_FFFF, std::ptr::null_mut(), 0);
    (0..count)
        .map(|index| {
            let len = DragQueryFileW(drop, index, std::ptr::null_mut(), 0) as usize;
            let mut buffer = vec![0u16; len + 1];
            DragQueryFileW(drop, index, buffer.as_mut_ptr(), buffer.len() as u32);
            PathBuf::from(from_wide(&buffer))
        })
        .collect()
}

/// Windows EMF metafile, e.g. Office copied charts/shapes. Stores bytes; [`render`](Metafile::render) rasterizes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Metafile {
    bytes: Vec<u8>,
}

impl Metafile {
    /// Validates EMF header signature `" EMF"`.
    pub fn from_bytes(bytes: &[u8]) -> GuiResult<Metafile> {
        emf_dimensions(bytes).ok_or_else(|| GuiError::InvalidArgument("not an EMF metafile".into()))?;
        Ok(Metafile { bytes: bytes.to_vec() })
    }

    /// Records `draw` in logical `width` × `height` at 96 DPI, for copying vectors to other programs.
    pub fn record(width: i32, height: i32, draw: impl FnOnce(&crate::graphics::Canvas)) -> GuiResult<Metafile> {
        use crate::ffi::gdi32::*;
        if width <= 0 || height <= 0 {
            return Err(GuiError::InvalidArgument("invalid size for the metafile".into()));
        }
        // Quadro em centésimos de milímetro (96 px = 25,4 mm).
        let frame = crate::ffi::RECT { left: 0, top: 0, right: width * 2540 / 96, bottom: height * 2540 / 96 };
        // SAFETY: DC de metarquivo em memória (sem arquivo), fechado abaixo.
        let dc = unsafe { CreateEnhMetaFileW(std::ptr::null_mut(), std::ptr::null(), &frame, std::ptr::null()) };
        if dc.is_null() {
            return Err(GuiError::last_os_error("CreateEnhMetaFileW"));
        }
        {
            let canvas = crate::graphics::Canvas::for_hdc(dc, 96, crate::utilities::Size::new(width, height));
            draw(&canvas);
        }
        // SAFETY: fecha o DC criado acima; o handle devolvido é apagado depois de copiado.
        unsafe {
            let handle = CloseEnhMetaFile(dc);
            if handle.is_null() {
                return Err(GuiError::last_os_error("CloseEnhMetaFile"));
            }
            let size = GetEnhMetaFileBits(handle, 0, std::ptr::null_mut());
            let mut bytes = vec![0u8; size as usize];
            GetEnhMetaFileBits(handle, size, bytes.as_mut_ptr());
            DeleteEnhMetaFile(handle);
            Metafile::from_bytes(&bytes)
        }
    }

    /// EMF bytes for saving to `.emf`.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Natural pixel size, drawing bounds on reference device.
    pub fn size(&self) -> crate::utilities::Size {
        let ((w, h), _) = emf_dimensions(&self.bytes).unwrap_or(((0, 0), (0, 0)));
        crate::utilities::Size::new(w.max(1), h.max(1))
    }

    /// Physical frame size in hundredths of millimeters.
    pub fn frame_hundredths_mm(&self) -> crate::utilities::Size {
        let (_, (w, h)) = emf_dimensions(&self.bytes).unwrap_or(((0, 0), (0, 0)));
        crate::utilities::Size::new(w, h)
    }

    /// Renders to `width` × `height` pixels over `background`.
    pub fn render(&self, width: i32, height: i32, background: crate::utilities::Color) -> GuiResult<PlusImage> {
        use crate::ffi::gdi32::*;
        if width <= 0 || height <= 0 {
            return Err(GuiError::InvalidArgument("invalid size for the metafile".into()));
        }
        // SAFETY: bytes validados; o handle é apagado no fim desta função.
        let handle = unsafe { SetEnhMetaFileBits(self.bytes.len() as u32, self.bytes.as_ptr()) };
        if handle.is_null() {
            return Err(GuiError::last_os_error("SetEnhMetaFileBits"));
        }
        let result = (|| {
            let (bitmap, bits) = crate::input::drag::dib_section(width, height)?;
            // SAFETY: DC de memória criado e liberado aqui; a seção tem largura × altura × 4
            // bytes e é apagada depois de copiada.
            unsafe {
                let dc = CreateCompatibleDC(std::ptr::null_mut());
                let old = SelectObject(dc, bitmap);
                let pixels = std::slice::from_raw_parts_mut(bits, width as usize * height as usize * 4);
                for p in pixels.chunks_exact_mut(4) {
                    p.copy_from_slice(&[background.b, background.g, background.r, 0]);
                }
                let rect = crate::ffi::RECT { left: 0, top: 0, right: width, bottom: height };
                let played = PlayEnhMetaFile(dc, handle, &rect);
                SelectObject(dc, old);
                DeleteDC(dc);
                // O GDI não escreve alfa: a imagem resultante é opaca.
                pixels.chunks_exact_mut(4).for_each(|p| p[3] = 255);
                let copy = pixels.to_vec();
                DeleteObject(bitmap);
                if played == 0 {
                    return Err(GuiError::last_os_error("PlayEnhMetaFile"));
                }
                PlusImage::from_pixels(width, height, &copy)
            }
        })();
        // SAFETY: handle criado acima, apagado uma única vez.
        unsafe { DeleteEnhMetaFile(handle) };
        result
    }
}

/// Simultaneous multiformat content for [`Clipboard::set`] and drag/drop
/// ([`start_drag`](crate::input::drag::start_drag)).
///
/// Each receiving program chooses its supported format: text editor text, browser/document editor HTML, image editor
/// image, Explorer files.
#[derive(Clone, Debug, Default)]
pub struct ClipboardContent {
    pub(crate) text: Option<String>,
    pub(crate) html: Option<String>,
    pub(crate) image: Option<(i32, i32, Vec<u8>)>,
    pub(crate) files: Vec<PathBuf>,
    pub(crate) custom: Vec<(String, Vec<u8>)>,
    pub(crate) metafile: Option<Metafile>,
}

impl ClipboardContent {
    /// Empty content.
    pub fn new() -> ClipboardContent {
        ClipboardContent::default()
    }

    /// Unicode text.
    pub fn text(mut self, text: &str) -> ClipboardContent {
        self.text = Some(text.to_string());
        self
    }

    /// HTML fragment; surrounding document assembled here.
    pub fn html(mut self, fragment: &str) -> ClipboardContent {
        self.html = Some(fragment.to_string());
        self
    }

    /// EMF, clipboard only, not OLE dragging.
    pub fn metafile(mut self, metafile: &Metafile) -> ClipboardContent {
        self.metafile = Some(metafile.clone());
        self
    }

    /// Transparent image as PNG, CF_DIBV5 and 32-bit CF_DIB.
    pub fn image(mut self, image: &PlusImage) -> GuiResult<ClipboardContent> {
        let size = image.size();
        self.image = Some((size.width, size.height, image.to_pixels()?));
        Ok(self)
    }

    /// Image from top-down BGRA.
    pub fn image_pixels(mut self, width: i32, height: i32, bgra: &[u8]) -> GuiResult<ClipboardContent> {
        if width <= 0 || height <= 0 || bgra.len() != width as usize * height as usize * 4 {
            return Err(GuiError::InvalidArgument("pixel buffer size does not match the image".into()));
        }
        self.image = Some((width, height, bgra.to_vec()));
        Ok(self)
    }

    /// File list as copied by Explorer.
    pub fn files(mut self, files: &[impl AsRef<Path>]) -> ClipboardContent {
        self.files = files.iter().map(|f| f.as_ref().to_path_buf()).collect();
        self
    }

    /// Application-specific named bytes, e.g. `"MyApp.Items"`; only programs knowing the name can read them.
    pub fn custom(mut self, format: &str, bytes: &[u8]) -> ClipboardContent {
        self.custom.push((format.to_string(), bytes.to_vec()));
        self
    }

    /// Whether no format exists.
    pub fn is_empty(&self) -> bool {
        self.text.is_none()
            && self.html.is_none()
            && self.image.is_none()
            && self.files.is_empty()
            && self.custom.is_empty()
            && self.metafile.is_none()
    }

    /// Formats as (identifier, bytes) in preference order.
    pub(crate) fn blocks(&self) -> GuiResult<Vec<(u32, Vec<u8>)>> {
        let mut blocks = Vec::new();
        for (name, bytes) in &self.custom {
            blocks.push((format_id(name)?, bytes.clone()));
        }
        if let Some(html) = &self.html {
            blocks.push((format_id(HTML_FORMAT)?, encode_html(html)));
        }
        if let Some((w, h, pixels)) = &self.image {
            // PNG primeiro: navegadores, Office e editores de imagem preservam o alfa.
            if let Ok(png) = PlusImage::from_pixels(*w, *h, pixels).and_then(|image| image.to_png()) {
                blocks.push((format_id(PNG_FORMAT)?, png));
            }
            blocks.push((CF_DIBV5, encode_dibv5(*w, *h, pixels)));
            blocks.push((CF_DIB, encode_dib(*w, *h, pixels)));
        }
        if !self.files.is_empty() {
            blocks.push((CF_HDROP, encode_hdrop(&self.files)));
        }
        if let Some(text) = &self.text {
            blocks.push((CF_UNICODETEXT, encode_text(text)));
        }
        Ok(blocks)
    }
}

/// System clipboard access.
pub struct Clipboard;

impl Clipboard {
    /// Replaces with `content`'s formats.
    pub fn set(content: &ClipboardContent) -> GuiResult<()> {
        let blocks = content.blocks()?;
        let guard = ClipboardGuard::open()?;
        // SAFETY: clipboard aberto por esta thread.
        if unsafe { EmptyClipboard() } == 0 {
            return Err(GuiError::last_os_error("EmptyClipboard"));
        }
        for (format, bytes) in blocks {
            guard.put(format, &bytes)?;
        }
        if let Some(metafile) = &content.metafile {
            let bytes = metafile.bytes();
            // SAFETY: bytes validados; com sucesso, o sistema passa a ser o dono do handle.
            unsafe {
                let handle = crate::ffi::gdi32::SetEnhMetaFileBits(bytes.len() as u32, bytes.as_ptr());
                if handle.is_null() {
                    return Err(GuiError::last_os_error("SetEnhMetaFileBits"));
                }
                if SetClipboardData(CF_ENHMETAFILE, handle).is_null() {
                    crate::ffi::gdi32::DeleteEnhMetaFile(handle);
                    return Err(GuiError::last_os_error("SetClipboardData"));
                }
            }
        }
        Ok(())
    }

    /// Copies Unicode text.
    pub fn set_text(text: &str) -> GuiResult<()> {
        Clipboard::set(&ClipboardContent::new().text(text))
    }

    /// Reads text, `None` if absent.
    pub fn text() -> GuiResult<Option<String>> {
        if !Clipboard::has_text() {
            return Ok(None);
        }
        let guard = ClipboardGuard::open()?;
        Ok(guard.get(CF_UNICODETEXT)?.map(|bytes| decode_text(&bytes)))
    }

    /// Whether text is available.
    pub fn has_text() -> bool {
        // SAFETY: consulta simples.
        unsafe { IsClipboardFormatAvailable(CF_UNICODETEXT) != 0 }
    }

    /// Copies HTML with `plain_text` fallback.
    pub fn set_html(fragment: &str, plain_text: &str) -> GuiResult<()> {
        Clipboard::set(&ClipboardContent::new().html(fragment).text(plain_text))
    }

    /// Reads copied selected HTML fragment, `None` if absent.
    pub fn html() -> GuiResult<Option<String>> {
        Ok(Clipboard::data(HTML_FORMAT)?.and_then(|bytes| decode_html(&bytes)))
    }

    /// Whether HTML is available.
    pub fn has_html() -> bool {
        Clipboard::has_format(HTML_FORMAT)
    }

    /// Copies transparent image; readers without alpha see opaque content.
    pub fn set_image(image: &PlusImage) -> GuiResult<()> {
        Clipboard::set(&ClipboardContent::new().image(image)?)
    }

    /// Reads best available image: PNG preserving browser/Office alpha, CF_DIBV5, CF_DIB including Windows-converted
    /// bitmaps, then naturally sized metafile.
    pub fn image() -> GuiResult<Option<PlusImage>> {
        if let Some(png) = Clipboard::png()? {
            if let Ok(image) = PlusImage::from_bytes(&png) {
                return Ok(Some(image));
            }
        }
        for format in [CF_DIBV5, CF_DIB] {
            // SAFETY: consulta simples.
            if unsafe { IsClipboardFormatAvailable(format) } == 0 {
                continue;
            }
            let bytes = {
                let guard = ClipboardGuard::open()?;
                guard.get(format)?
            };
            if let Some((width, height, pixels)) = bytes.as_deref().and_then(decode_dib) {
                return Ok(Some(PlusImage::from_pixels(width, height, &pixels)?));
            }
        }
        match Clipboard::metafile()? {
            Some(metafile) => {
                let size = metafile.size();
                let (width, height) = (size.width.min(4096), size.height.min(4096));
                Ok(Some(metafile.render(width, height, crate::utilities::Color::WHITE)?))
            }
            None => Ok(None),
        }
    }

    /// Whether PNG/DIB/bitmap/metafile is available.
    pub fn has_image() -> bool {
        // SAFETY: consultas simples.
        let native = unsafe {
            IsClipboardFormatAvailable(CF_DIB) != 0
                || IsClipboardFormatAvailable(CF_DIBV5) != 0
                || IsClipboardFormatAvailable(CF_ENHMETAFILE) != 0
        };
        native || Clipboard::has_format(PNG_FORMAT)
    }

    /// Copied bytes in registered PNG format, if any.
    pub fn png() -> GuiResult<Option<Vec<u8>>> {
        Clipboard::data(PNG_FORMAT)
    }

    /// Reads copied EMF, e.g. Office charts/shapes.
    pub fn metafile() -> GuiResult<Option<Metafile>> {
        // SAFETY: consulta simples.
        if unsafe { IsClipboardFormatAvailable(CF_ENHMETAFILE) } == 0 {
            return Ok(None);
        }
        let _guard = ClipboardGuard::open()?;
        // SAFETY: clipboard aberto; o handle pertence ao sistema e só é copiado.
        unsafe {
            let handle = GetClipboardData(CF_ENHMETAFILE);
            if handle.is_null() {
                return Ok(None);
            }
            let size = crate::ffi::gdi32::GetEnhMetaFileBits(handle, 0, std::ptr::null_mut());
            if size == 0 {
                return Ok(None);
            }
            let mut bytes = vec![0u8; size as usize];
            crate::ffi::gdi32::GetEnhMetaFileBits(handle, size, bytes.as_mut_ptr());
            Ok(Metafile::from_bytes(&bytes).ok())
        }
    }

    /// Whether EMF is available.
    pub fn has_metafile() -> bool {
        // SAFETY: consulta simples.
        unsafe { IsClipboardFormatAvailable(CF_ENHMETAFILE) != 0 }
    }

    /// Copies EMF.
    pub fn set_metafile(metafile: &Metafile) -> GuiResult<()> {
        Clipboard::set(&ClipboardContent::new().metafile(metafile))
    }

    /// Sequence number changing on every clipboard update by any program.
    pub fn sequence_number() -> u32 {
        // SAFETY: consulta simples.
        unsafe { GetClipboardSequenceNumber() }
    }

    /// Copies files for Explorer pasting.
    pub fn set_files(files: &[impl AsRef<Path>]) -> GuiResult<()> {
        Clipboard::set(&ClipboardContent::new().files(files))
    }

    /// Reads Explorer file list, empty if absent.
    pub fn files() -> GuiResult<Vec<PathBuf>> {
        // SAFETY: consulta simples.
        if unsafe { IsClipboardFormatAvailable(CF_HDROP) } == 0 {
            return Ok(Vec::new());
        }
        let _guard = ClipboardGuard::open()?;
        // SAFETY: clipboard aberto; o HDROP pertence ao sistema e só é consultado.
        unsafe {
            let drop = GetClipboardData(CF_HDROP);
            if drop.is_null() {
                return Ok(Vec::new());
            }
            Ok(hdrop_files(drop))
        }
    }

    /// Copies bytes in named custom format.
    pub fn set_data(format: &str, bytes: &[u8]) -> GuiResult<()> {
        Clipboard::set(&ClipboardContent::new().custom(format, bytes))
    }

    /// Reads named bytes, `None` if absent.
    pub fn data(format: &str) -> GuiResult<Option<Vec<u8>>> {
        let id = format_id(format)?;
        // SAFETY: consulta simples.
        if unsafe { IsClipboardFormatAvailable(id) } == 0 {
            return Ok(None);
        }
        let guard = ClipboardGuard::open()?;
        guard.get(id)
    }

    /// Whether named format is available.
    pub fn has_format(format: &str) -> bool {
        // SAFETY: consulta simples.
        format_id(format).is_ok_and(|id| unsafe { IsClipboardFormatAvailable(id) } != 0)
    }

    /// Current registered format names; standard text/image formats are unnamed and excluded.
    pub fn format_names() -> GuiResult<Vec<String>> {
        let _guard = ClipboardGuard::open()?;
        let mut names = Vec::new();
        let mut format = 0;
        loop {
            // SAFETY: clipboard aberto; enumeração simples.
            format = unsafe { EnumClipboardFormats(format) };
            if format == 0 {
                break;
            }
            let mut buffer = [0u16; 256];
            // SAFETY: buffer local com o tamanho informado.
            let len = unsafe { GetClipboardFormatNameW(format, buffer.as_mut_ptr(), buffer.len() as i32) };
            if len > 0 {
                names.push(String::from_utf16_lossy(&buffer[..len as usize]));
            }
        }
        Ok(names)
    }

    /// Empties clipboard.
    pub fn clear() -> GuiResult<()> {
        let _guard = ClipboardGuard::open()?;
        // SAFETY: clipboard aberto por esta thread.
        if unsafe { EmptyClipboard() } == 0 {
            return Err(GuiError::last_os_error("EmptyClipboard"));
        }
        Ok(())
    }
}

type WatchFn = std::rc::Rc<dyn Fn()>;

thread_local! {
    static WATCHERS: std::cell::RefCell<Vec<(u64, WatchFn)>> = const { std::cell::RefCell::new(Vec::new()) };
    static NEXT_WATCHER: std::cell::Cell<u64> = const { std::cell::Cell::new(1) };
}

/// Watches clipboard updates by any program on the UI thread. Keep handle alive; dropping unregisters.
///
/// ```no_run
/// # use arqrustui::prelude::*;
/// # fn demo() -> GuiResult<()> {
/// let watcher = ClipboardWatcher::new(|| {
///     if Clipboard::has_image() {
///         println!("an image was copied");
///     }
/// })?;
/// # drop(watcher); Ok(()) }
/// ```
pub struct ClipboardWatcher {
    id: u64,
}

impl std::fmt::Debug for ClipboardWatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClipboardWatcher").field("id", &self.id).finish()
    }
}

impl ClipboardWatcher {
    /// Starts watching via `AddClipboardFormatListener` on internal window.
    pub fn new(on_change: impl Fn() + 'static) -> GuiResult<ClipboardWatcher> {
        let app = crate::core::application::require()?;
        let first = WATCHERS.with(|w| w.borrow().is_empty());
        if first {
            // SAFETY: janela interna viva desta thread.
            if unsafe { AddClipboardFormatListener(app.dispatch_hwnd) } == 0 {
                return Err(GuiError::last_os_error("AddClipboardFormatListener"));
            }
            crate::core::application::register_message(
                WM_CLIPBOARDUPDATE,
                usize::MAX,
                std::rc::Rc::new(|_, _| {
                    let callbacks: Vec<WatchFn> =
                        WATCHERS.with(|w| w.borrow().iter().map(|(_, f)| f.clone()).collect());
                    for callback in callbacks {
                        callback();
                    }
                }),
            );
        }
        let id = NEXT_WATCHER.with(|n| {
            let id = n.get();
            n.set(id + 1);
            id
        });
        WATCHERS.with(|w| w.borrow_mut().push((id, std::rc::Rc::new(on_change))));
        Ok(ClipboardWatcher { id })
    }
}

impl Drop for ClipboardWatcher {
    fn drop(&mut self) {
        let removed = WATCHERS.try_with(|w| {
            let mut list = w.borrow_mut();
            let removed: Vec<WatchFn> = list.iter().filter(|(id, _)| *id == self.id).map(|(_, f)| f.clone()).collect();
            list.retain(|(id, _)| *id != self.id);
            (removed, list.is_empty())
        });
        if let Ok((removed, empty)) = removed {
            drop(removed);
            if empty {
                if let Some(app) = crate::core::application::state() {
                    // SAFETY: desfaz o registro feito em `new` na janela interna viva.
                    unsafe { RemoveClipboardFormatListener(app.dispatch_hwnd) };
                    crate::core::application::unregister_message(WM_CLIPBOARDUPDATE, usize::MAX);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_round_trip_keeps_the_fragment() {
        let fragment = "<b>Olá</b>, <i>mundo</i> — ç";
        let block = encode_html(fragment);
        let text = String::from_utf8_lossy(&block);
        assert!(text.starts_with("Version:0.9\r\nStartHTML:"));
        assert_eq!(decode_html(&block).as_deref(), Some(fragment));
        // Deslocamentos apontam exatamente para os marcadores.
        let start: usize = text["Version:0.9\r\nStartHTML:0000000000\r\nEndHTML:0000000000\r\nStartFragment:".len()..]
            [..10]
            .parse()
            .unwrap();
        assert!(text.as_bytes()[..start].ends_with(b"<!--StartFragment-->"));
    }

    #[test]
    fn hdrop_round_trip() {
        let files = vec![PathBuf::from(r"C:\dados\relatório.txt"), PathBuf::from(r"D:\b.png")];
        assert_eq!(decode_hdrop(&encode_hdrop(&files)), files);
        let mut ansi = vec![20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        ansi.extend_from_slice(b"C:\\a.txt\0\0");
        assert_eq!(decode_hdrop(&ansi), vec![PathBuf::from("C:\\a.txt")]);
        assert!(decode_hdrop(&[1, 2]).is_empty());
    }

    #[test]
    fn dibv5_and_bitfields_decode_with_alpha() {
        let pixels = [10u8, 20, 30, 128, 1, 2, 3, 255];
        let v5 = encode_dibv5(2, 1, &pixels);
        assert_eq!(v5.len(), 124 + 8);
        assert_eq!(decode_dib(&v5), Some((2, 1, pixels.to_vec())));
        // DIB de 40 bytes com BI_BITFIELDS em ordem RGBA (R no byte baixo).
        let mut with_masks = encode_dib(1, 1, &[0, 0, 0, 0])[..40].to_vec();
        with_masks[16..20].copy_from_slice(&3u32.to_le_bytes());
        for mask in [0x0000_00FFu32, 0x0000_FF00, 0x00FF_0000] {
            with_masks.extend_from_slice(&mask.to_le_bytes());
        }
        with_masks.extend_from_slice(&[200, 100, 50, 0]); // R=200 G=100 B=50
        assert_eq!(decode_dib(&with_masks), Some((1, 1, vec![50, 100, 200, 255])));
        assert_eq!(decode_dib(&[0u8; 10]), None);
    }

    #[test]
    fn emf_header_dimensions() {
        let mut header = vec![0u8; 88];
        header[0..4].copy_from_slice(&1i32.to_le_bytes());
        for (at, value) in [(8, 0i32), (12, 0), (16, 99), (20, 49), (24, 0), (28, 0), (32, 2646), (36, 1323)] {
            header[at..at + 4].copy_from_slice(&value.to_le_bytes());
        }
        header[40..44].copy_from_slice(b" EMF");
        assert_eq!(emf_dimensions(&header), Some(((100, 50), (2646, 1323))));
        let metafile = Metafile::from_bytes(&header).unwrap();
        assert_eq!(metafile.size(), crate::utilities::Size::new(100, 50));
        header[40] = b'X';
        assert!(Metafile::from_bytes(&header).is_err());
    }

    #[test]
    fn dib_round_trip_and_alpha_fallback() {
        let pixels: Vec<u8> = (0..2 * 3 * 4).map(|i| i as u8 * 7).collect();
        let dib = encode_dib(2, 3, &pixels);
        assert_eq!(dib.len(), 40 + pixels.len());
        assert_eq!(decode_dib(&dib), Some((2, 3, pixels)));
        // 32 bits com alfa todo zerado vira opaco.
        let clear = encode_dib(1, 1, &[10, 20, 30, 0]);
        assert_eq!(decode_dib(&clear).unwrap().2, vec![10, 20, 30, 255]);
        // 24 bits, de baixo para cima, com preenchimento de linha.
        let mut dib24 = encode_dib(1, 2, &[0; 8])[..40].to_vec();
        dib24[14] = 24;
        dib24.extend_from_slice(&[1, 2, 3, 0, 4, 5, 6, 0]); // linha de baixo, linha de cima
        assert_eq!(decode_dib(&dib24).unwrap().2, vec![4, 5, 6, 255, 1, 2, 3, 255]);
        assert_eq!(decode_dib(&[0; 10]), None);
    }

    #[test]
    fn hdrop_lists_wide_paths() {
        let block = encode_hdrop(&[PathBuf::from("C:\\a.txt"), PathBuf::from("D:\\ç.png")]);
        assert_eq!(&block[..4], &20u32.to_le_bytes());
        let list = decode_text(&block[20..]);
        assert_eq!(list, "C:\\a.txt");
        assert!(block.ends_with(&[0, 0, 0, 0]));
        assert_eq!(decode_text(&encode_text("ação")), "ação");
    }
}
