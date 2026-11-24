//! Native handle RAII wrappers.
//!
//! Windows handle types have different release rules; each rule has its own wrapper rather than a single generic type:
//!
//! | Wrapper | Handle | Release |
//! |--------------------|--------------------------------|----------------------|
//! | [`GdiObject`] | `HFONT`, `HBRUSH`, `HPEN`, `HBITMAP`, `HRGN` | `DeleteObject` |
//! | [`Icon`] | owned/shared `HICON` | `DestroyIcon` only if owned |
//! | [`MemoryDc`] | `HDC` from `CreateCompatibleDC` | `DeleteDC` |
//! | [`WindowDc`] | `HDC` from `GetDC` | `ReleaseDC` |
//! | [`ImageList`] | `HIMAGELIST` | `ImageList_Destroy` |
//! | [`GlobalMemory`] | `HGLOBAL` | `GlobalFree` unless transferred |
//! | [`ComPtr`](crate::ffi::ole32::ComPtr) | COM interface | `Release` |
//!
//! `HWND` is absent: windows belong to the windowing system (parents destroy children) and are tracked by the
//! [`WidgetCore`](super::WidgetCore) registry.

use crate::core::errors::{GuiError, GuiResult};
use crate::ffi::comctl32;
use crate::ffi::gdi32::{CreateBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, BITMAPINFOHEADER};
use crate::ffi::kernel32::{GlobalAlloc, GlobalFree, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
use crate::ffi::user32::{
    CreateIconFromResourceEx, CreateIconIndirect, DestroyIcon, GetDC, GetIconInfo, LoadImageW, ReleaseDC, ICONINFO,
    IMAGE_ICON, LR_DEFAULTCOLOR, LR_DEFAULTSIZE, LR_LOADFROMFILE,
};
use crate::ffi::{HDC, HGDIOBJ, HGLOBAL, HICON, HIMAGELIST, HWND};
use crate::utilities::to_wide;
use std::fmt::Debug;
use std::path::Path;
use std::rc::Rc;

/// Owned GDI object (font, brush, pen, bitmap or region).
#[derive(Debug)]
pub struct GdiObject(HGDIOBJ);

impl GdiObject {
    /// Takes ownership of a newly created GDI handle; `None` if null.
    ///
    /// # Safety
    /// `handle` must be null or a valid GDI object that no other owner will delete, and must not be selected into a DC
    /// when dropped.
    pub unsafe fn from_raw(handle: HGDIOBJ) -> Option<Self> {
        if handle.is_null() {
            None
        } else {
            Some(Self(handle))
        }
    }

    /// Raw handle; wrapper retains ownership.
    pub fn raw(&self) -> HGDIOBJ {
        self.0
    }
}

impl Drop for GdiObject {
    fn drop(&mut self) {
        // SAFETY: o handle é válido e exclusivo deste wrapper (invariante de `from_raw`).
        unsafe { DeleteObject(self.0) };
    }
}

#[derive(Debug)]
struct IconInner {
    handle: HICON,
    owned: bool,
}

impl Drop for IconInner {
    fn drop(&mut self) {
        if self.owned {
            // SAFETY: ícones marcados como próprios foram criados por LoadImageW sem
            // LR_SHARED e pertencem exclusivamente a este objeto.
            unsafe { DestroyIcon(self.handle) };
        }
    }
}

/// Windows icon. Cheap reference-counted cloning.
///
/// System icons are shared and never destroyed; file-loaded icons are destroyed when the last clone drops.
#[derive(Clone, Debug)]
pub struct Icon(Rc<IconInner>);

/// Standard system icons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SystemIcon {
    /// Generic application icon.
    Application,
    /// Information.
    Information,
    /// Warning.
    Warning,
    /// Error.
    Error,
    /// Question.
    Question,
    /// Elevation shield (UAC).
    Shield,
}

impl Icon {
    /// Gets a standard system icon.
    pub fn system(icon: SystemIcon) -> GuiResult<Icon> {
        use crate::ffi::user32::*;
        let id = match icon {
            SystemIcon::Application => IDI_APPLICATION,
            SystemIcon::Information => IDI_INFORMATION,
            SystemIcon::Warning => IDI_WARNING,
            SystemIcon::Error => IDI_ERROR,
            SystemIcon::Question => IDI_QUESTION,
            SystemIcon::Shield => IDI_SHIELD,
        };
        // SAFETY: com instância nula, o "nome" é um identificador inteiro de ícone de sistema.
        let handle = unsafe { LoadIconW(std::ptr::null_mut(), id as *const u16) };
        if handle.is_null() {
            return Err(GuiError::last_os_error("LoadIconW"));
        }
        Ok(Icon(Rc::new(IconInner { handle, owned: false })))
    }

    /// Loads an icon from an `.ico` file.
    pub fn from_file(path: impl AsRef<Path>) -> GuiResult<Icon> {
        let wide = to_wide(&path.as_ref().to_string_lossy());
        // SAFETY: `wide` é uma string válida terminada em NUL.
        let handle = unsafe {
            LoadImageW(std::ptr::null_mut(), wide.as_ptr(), IMAGE_ICON, 0, 0, LR_LOADFROMFILE | LR_DEFAULTSIZE)
        };
        if handle.is_null() {
            return Err(GuiError::last_os_error("LoadImageW(icon)"));
        }
        Ok(Icon(Rc::new(IconInner { handle, owned: true })))
    }

    /// Creates a `size`×`size` pixel icon from in-memory `.ico` file content.
    ///
    /// Selects the smallest frame at least `size` pixels, or the largest available; Windows resizes if not exact. PNG
    /// frames inside ICO are supported.
    pub fn from_ico_bytes(bytes: &[u8], size: i32) -> GuiResult<Icon> {
        if size <= 0 {
            return Err(GuiError::InvalidArgument("the icon size must be positive".into()));
        }
        let entries = ico_directory(bytes)?;
        let entry = best_ico_entry(&entries, size as u32).expect("a validated directory is not empty");
        let data = &bytes[entry.offset..entry.offset + entry.length];
        // SAFETY: `data` é um bloco de imagem de ícone (BMP sem cabeçalho de arquivo ou
        // PNG) contido em `bytes`, válido durante a chamada; o handle devolvido passa a
        // pertencer ao `Icon`.
        let handle = unsafe {
            CreateIconFromResourceEx(data.as_ptr(), data.len() as u32, 1, 0x0003_0000, size, size, LR_DEFAULTCOLOR)
        };
        if handle.is_null() {
            return Err(GuiError::last_os_error("CreateIconFromResourceEx"));
        }
        Ok(Icon(Rc::new(IconInner { handle, owned: true })))
    }

    /// Takes ownership of a newly created `HICON`, destroyed with the last clone.
    ///
    /// # Safety
    /// `handle` must be a valid icon that no other owner will destroy.
    pub unsafe fn from_raw_owned(handle: HICON) -> Option<Icon> {
        (!handle.is_null()).then(|| Icon(Rc::new(IconInner { handle, owned: true })))
    }

    /// Icon size in pixels.
    pub fn size(&self) -> crate::utilities::Size {
        icon_pixels(self.0.handle)
            .map(|(width, height, _)| crate::utilities::Size::new(width, height))
            .unwrap_or_default()
    }

    /// Disabled icon: grayscale with half opacity, like inactive Windows items.
    pub fn disabled(&self) -> GuiResult<Icon> {
        let (width, height, mut pixels) =
            icon_pixels(self.0.handle).ok_or_else(|| GuiError::last_os_error("GetIconInfo/GetDIBits"))?;
        for pixel in pixels.chunks_exact_mut(4) {
            // BGRA com alfa direto (não pré-multiplicado), como nos bitmaps de ícone.
            let luma = (u32::from(pixel[2]) * 30 + u32::from(pixel[1]) * 59 + u32::from(pixel[0]) * 11) / 100;
            let gray = (luma * 2 / 3 + 85) as u8;
            pixel[0] = gray;
            pixel[1] = gray;
            pixel[2] = gray;
            pixel[3] /= 2;
        }
        icon_from_pixels(width, height, &pixels)
    }

    /// Raw handle (advanced API). Valid while any `Icon` clone exists.
    pub fn raw(&self) -> HICON {
        self.0.handle
    }
}

/// Image within an `.ico` file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct IcoEntry {
    /// Pixel width (256 stored as 0).
    pub width: u32,
    /// Pixel height.
    pub height: u32,
    /// Image data file offset.
    pub offset: usize,
    /// Image data size.
    pub length: usize,
    /// `true` for PNG data.
    pub png: bool,
}

/// Reads and validates an ICO directory.
pub(crate) fn ico_directory(bytes: &[u8]) -> GuiResult<Vec<IcoEntry>> {
    let invalid = |reason: &str| GuiError::InvalidArgument(format!("invalid .ico file: {reason}"));
    let word = |at: usize| u16::from_le_bytes([bytes[at], bytes[at + 1]]);
    let dword = |at: usize| u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]);
    if bytes.len() < 6 || word(0) != 0 || word(2) != 1 {
        return Err(invalid("missing header"));
    }
    let count = word(4) as usize;
    if count == 0 {
        return Err(invalid("no images"));
    }
    if bytes.len() < 6 + 16 * count {
        return Err(invalid("truncated directory"));
    }
    (0..count)
        .map(|index| {
            let at = 6 + 16 * index;
            let offset = dword(at + 12) as usize;
            let length = dword(at + 8) as usize;
            if length < 8 || offset.checked_add(length).map_or(true, |end| end > bytes.len()) {
                return Err(invalid("image outside the file bounds"));
            }
            let dimension = |raw: u8| if raw == 0 { 256 } else { u32::from(raw) };
            Ok(IcoEntry {
                width: dimension(bytes[at]),
                height: dimension(bytes[at + 1]),
                offset,
                length,
                png: bytes[offset..offset + 8] == [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A],
            })
        })
        .collect()
}

/// Selects the smallest frame at least `size` pixels or the largest available.
pub(crate) fn best_ico_entry(entries: &[IcoEntry], size: u32) -> Option<&IcoEntry> {
    entries
        .iter()
        .filter(|entry| entry.width >= size)
        .min_by_key(|entry| entry.width)
        .or_else(|| entries.iter().max_by_key(|entry| entry.width))
}

/// Icon color image BGRA pixels (32-bit, top-down).
fn icon_pixels(icon: HICON) -> Option<(i32, i32, Vec<u8>)> {
    // SAFETY: ICONINFO aceita zeros; GetIconInfo cria cópias dos bitmaps, que são
    // apagadas ao final deste bloco.
    unsafe {
        let mut info: ICONINFO = std::mem::zeroed();
        if icon.is_null() || GetIconInfo(icon, &mut info) == 0 {
            return None;
        }
        let bitmaps = (GdiObject::from_raw(info.hbmColor), GdiObject::from_raw(info.hbmMask));
        let color = bitmaps.0.as_ref()?;
        // BITMAPINFO com espaço para a maior tabela de cores que GetDIBits pode gravar.
        #[repr(C)]
        struct Info {
            header: BITMAPINFOHEADER,
            colors: [u32; 256],
        }
        let mut info = Info {
            header: BITMAPINFOHEADER { biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32, ..Default::default() },
            colors: [0; 256],
        };
        let dc = MemoryDc::compatible_with(std::ptr::null_mut()).ok()?;
        // Primeira chamada: só preenche as dimensões.
        if GetDIBits(dc.raw(), color.raw(), 0, 0, std::ptr::null_mut(), &mut info.header, 0) == 0 {
            return None;
        }
        let (width, height) = (info.header.biWidth, info.header.biHeight.abs());
        if width <= 0 || height <= 0 {
            return None;
        }
        info.header.biHeight = -height;
        info.header.biBitCount = 32;
        info.header.biCompression = 0;
        info.header.biPlanes = 1;
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        let lines = GetDIBits(dc.raw(), color.raw(), 0, height as u32, pixels.as_mut_ptr().cast(), &mut info.header, 0);
        (lines == height).then_some((width, height, pixels))
    }
}

/// Creates an icon from top-down straight-alpha BGRA pixels.
fn icon_from_pixels(width: i32, height: i32, pixels: &[u8]) -> GuiResult<Icon> {
    debug_assert_eq!(pixels.len(), (width * height * 4) as usize);
    // SAFETY: os bitmaps são criados com o tamanho exato do buffer; CreateIconIndirect
    // copia ambos, que são apagados pelos wrappers ao final.
    unsafe {
        let color = GdiObject::from_raw(CreateBitmap(width, height, 1, 32, pixels.as_ptr().cast()))
            .ok_or_else(|| GuiError::last_os_error("CreateBitmap"))?;
        // Máscara de 1 bit (linhas alinhadas a 16 bits): 1 = transparente.
        let stride = (((width + 15) / 16) * 2) as usize;
        let mut mask_bytes = vec![0u8; stride * height as usize];
        for (index, pixel) in pixels.chunks_exact(4).enumerate() {
            if pixel[3] == 0 {
                let (x, y) = (index % width as usize, index / width as usize);
                mask_bytes[y * stride + x / 8] |= 0x80 >> (x % 8);
            }
        }
        let mask = GdiObject::from_raw(CreateBitmap(width, height, 1, 1, mask_bytes.as_ptr().cast()))
            .ok_or_else(|| GuiError::last_os_error("CreateBitmap"))?;
        let info = ICONINFO { fIcon: 1, xHotspot: 0, yHotspot: 0, hbmMask: mask.raw(), hbmColor: color.raw() };
        Icon::from_raw_owned(CreateIconIndirect(&info)).ok_or_else(|| GuiError::last_os_error("CreateIconIndirect"))
    }
}

/// Anything supplying an [`Icon`] at the requested size.
///
/// Implemented by [`Icon`] (fixed size, resized by Windows as needed) and [`IconAsset`](crate::icons::IconAsset)
/// (multiple resolutions, selecting per control DPI). Icon APIs accept `&impl IconSource` or `&dyn IconSource`.
pub trait IconSource: Debug {
    /// Icon at `size`×`size` physical pixels.
    fn icon_pixels(&self, size: i32) -> GuiResult<Icon>;

    /// Stable source identity for image-list slot reuse.
    #[doc(hidden)]
    fn source_key(&self) -> usize;

    /// Owned source copy for later reloading, e.g. after DPI changes.
    #[doc(hidden)]
    fn to_shared(&self) -> Rc<dyn IconSource>;

    /// High-resolution image for large display sizes, if available.
    #[doc(hidden)]
    fn large_image(&self) -> Option<GuiResult<crate::graphics::PlusImage>> {
        None
    }
}

impl IconSource for Icon {
    fn icon_pixels(&self, _size: i32) -> GuiResult<Icon> {
        Ok(self.clone())
    }

    fn source_key(&self) -> usize {
        self.0.handle as usize
    }

    fn to_shared(&self) -> Rc<dyn IconSource> {
        Rc::new(self.clone())
    }
}

impl<T: IconSource + ?Sized> IconSource for &T {
    fn icon_pixels(&self, size: i32) -> GuiResult<Icon> {
        (**self).icon_pixels(size)
    }

    fn source_key(&self) -> usize {
        (**self).source_key()
    }

    fn to_shared(&self) -> Rc<dyn IconSource> {
        (**self).to_shared()
    }

    fn large_image(&self) -> Option<GuiResult<crate::graphics::PlusImage>> {
        (**self).large_image()
    }
}

impl IconSource for Rc<dyn IconSource> {
    fn icon_pixels(&self, size: i32) -> GuiResult<Icon> {
        (**self).icon_pixels(size)
    }

    fn source_key(&self) -> usize {
        (**self).source_key()
    }

    fn to_shared(&self) -> Rc<dyn IconSource> {
        self.clone()
    }

    fn large_image(&self) -> Option<GuiResult<crate::graphics::PlusImage>> {
        (**self).large_image()
    }
}

/// Memory DC from `CreateCompatibleDC`.
#[derive(Debug)]
pub struct MemoryDc(HDC);

impl MemoryDc {
    /// Creates a memory DC compatible with `reference`, or the screen if null.
    pub(crate) fn compatible_with(reference: HDC) -> GuiResult<Self> {
        // SAFETY: `reference` nulo é aceito; caso contrário é um DC fornecido pelo chamador.
        let hdc = unsafe { CreateCompatibleDC(reference) };
        if hdc.is_null() {
            Err(GuiError::last_os_error("CreateCompatibleDC"))
        } else {
            Ok(Self(hdc))
        }
    }

    /// Raw handle.
    pub fn raw(&self) -> HDC {
        self.0
    }
}

impl Drop for MemoryDc {
    fn drop(&mut self) {
        // SAFETY: DC criado por CreateCompatibleDC e possuído por este wrapper.
        unsafe { DeleteDC(self.0) };
    }
}

/// Window DC from `GetDC`, released with `ReleaseDC`.
#[derive(Debug)]
pub struct WindowDc {
    hwnd: HWND,
    hdc: HDC,
}

impl WindowDc {
    /// Gets window DC (entire screen if `hwnd` is null).
    pub(crate) fn get(hwnd: HWND) -> GuiResult<Self> {
        // SAFETY: GetDC aceita HWND nulo (tela) ou qualquer janela; falhas retornam nulo.
        let hdc = unsafe { GetDC(hwnd) };
        if hdc.is_null() {
            Err(GuiError::last_os_error("GetDC"))
        } else {
            Ok(Self { hwnd, hdc })
        }
    }

    /// Raw handle.
    pub fn raw(&self) -> HDC {
        self.hdc
    }
}

impl Drop for WindowDc {
    fn drop(&mut self) {
        // SAFETY: par exato do GetDC feito em `get`.
        unsafe { ReleaseDC(self.hwnd, self.hdc) };
    }
}

/// Image list (`HIMAGELIST`) for ListView, TreeView, TabControl and ToolBar.
#[derive(Debug)]
pub struct ImageList {
    handle: HIMAGELIST,
    size: i32,
}

impl ImageList {
    /// Creates an image list with `size`×`size` pixel icons.
    pub fn new(size: i32) -> GuiResult<Rc<Self>> {
        let api = comctl32::functions().ok_or(GuiError::Unsupported("comctl32.dll"))?;
        // SAFETY: argumentos simples; falha é sinalizada por retorno nulo.
        let handle = unsafe { (api.ImageList_Create)(size, size, comctl32::ILC_COLOR32 | comctl32::ILC_MASK, 4, 4) };
        if handle.is_null() {
            return Err(GuiError::last_os_error("ImageList_Create"));
        }
        Ok(Rc::new(Self { handle, size }))
    }

    /// Adds an icon and returns its list index.
    pub fn add_icon(&self, icon: &Icon) -> GuiResult<i32> {
        let api = comctl32::functions().ok_or(GuiError::Unsupported("comctl32.dll"))?;
        // SAFETY: lista e ícone são handles válidos mantidos vivos pelos wrappers; a
        // lista copia o ícone.
        let index = unsafe { (api.ImageList_ReplaceIcon)(self.handle, -1, icon.raw()) };
        if index < 0 {
            Err(GuiError::last_os_error("ImageList_ReplaceIcon"))
        } else {
            Ok(index)
        }
    }

    /// Adds any icon source loaded at list size.
    pub fn add(&self, icon: &impl IconSource) -> GuiResult<i32> {
        self.add_icon(&icon.icon_pixels(self.size)?)
    }

    /// Image side length in pixels.
    pub fn icon_size(&self) -> i32 {
        self.size
    }

    /// Image count.
    pub fn len(&self) -> usize {
        comctl32::functions()
            // SAFETY: a lista é válida enquanto o wrapper existir.
            .map(|api| unsafe { (api.ImageList_GetImageCount)(self.handle) }.max(0) as usize)
            .unwrap_or(0)
    }

    /// `true` if the list has no images.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Raw handle (advanced API).
    pub fn raw(&self) -> HIMAGELIST {
        self.handle
    }
}

impl Drop for ImageList {
    fn drop(&mut self) {
        if let Some(api) = comctl32::functions() {
            // SAFETY: a lista foi criada por este wrapper e os controles que a usam
            // guardam um `Rc` dela, logo ninguém mais a referencia aqui.
            unsafe { (api.ImageList_Destroy)(self.handle) };
        }
    }
}

/// Global memory block (`HGLOBAL`) for clipboard use.
#[derive(Debug)]
pub struct GlobalMemory(HGLOBAL);

impl GlobalMemory {
    /// Allocates a movable block and copies `bytes` into it.
    pub fn with_bytes(bytes: &[u8]) -> GuiResult<Self> {
        // SAFETY: alocação simples; o retorno é verificado.
        let handle = unsafe { GlobalAlloc(GMEM_MOVEABLE, bytes.len().max(1)) };
        if handle.is_null() {
            return Err(GuiError::last_os_error("GlobalAlloc"));
        }
        let memory = Self(handle);
        // SAFETY: o bloco tem ao menos `bytes.len()` bytes; o ponteiro de GlobalLock
        // é válido até GlobalUnlock.
        unsafe {
            let ptr = GlobalLock(handle) as *mut u8;
            if ptr.is_null() {
                return Err(GuiError::last_os_error("GlobalLock"));
            }
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
            GlobalUnlock(handle);
        }
        Ok(memory)
    }

    /// Raw handle.
    pub fn raw(&self) -> HGLOBAL {
        self.0
    }

    /// Transfers ownership without freeing, e.g. to the clipboard.
    pub fn into_raw(self) -> HGLOBAL {
        let handle = self.0;
        std::mem::forget(self);
        handle
    }
}

impl Drop for GlobalMemory {
    fn drop(&mut self) {
        // SAFETY: bloco alocado por este wrapper cuja posse não foi transferida.
        unsafe { GlobalFree(self.0) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ffi::gdi32::CreateSolidBrush;

    #[test]
    fn gdi_object_null_is_rejected() {
        // SAFETY: ponteiro nulo é explicitamente aceito.
        assert!(unsafe { GdiObject::from_raw(std::ptr::null_mut()) }.is_none());
    }

    #[test]
    fn gdi_object_is_deleted_once() {
        // SAFETY: pincel recém-criado, possuído só pelo wrapper.
        let brush = unsafe { GdiObject::from_raw(CreateSolidBrush(0x00FF_00FF)) }.expect("pincel");
        assert!(!brush.raw().is_null());
        drop(brush);
    }

    #[test]
    fn memory_dc_and_global_memory() {
        let dc = MemoryDc::compatible_with(std::ptr::null_mut()).expect("dc");
        assert!(!dc.raw().is_null());
        let mem = GlobalMemory::with_bytes(b"abc").expect("mem");
        // SAFETY: bloco válido possuído por `mem`.
        assert!(unsafe { crate::ffi::kernel32::GlobalSize(mem.raw()) } >= 3);
        let raw = mem.into_raw();
        // SAFETY: a posse foi devolvida por `into_raw`; liberamos manualmente.
        unsafe { GlobalFree(raw) };
    }

    #[test]
    fn system_icon_is_shared() {
        let icon = Icon::system(SystemIcon::Information).expect("ícone");
        let clone = icon.clone();
        assert_eq!(icon.raw(), clone.raw());
        assert!(Icon::from_file("Z:\\nao\\existe.ico").is_err());
    }
}
