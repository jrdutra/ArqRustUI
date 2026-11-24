//! Drag source: data object, `IDropSource`, [`start_drag`](super::start_drag).

use super::*;

use crate::core::handles::GlobalMemory;
use crate::core::widget::Widget;
use crate::core::{GuiError, GuiResult};
use crate::ffi::shell32::SHCreateDataObject;
use crate::graphics::PlusImage;
use crate::system::clipboard::ClipboardContent;
use crate::utilities::Point;
use std::cell::Cell;
use std::ffi::c_void;

// ---------------------------------------------------------------------------
// Origem: objeto de dados e IDropSource
// ---------------------------------------------------------------------------

/// Creates a shell `IDataObject` with all `content` formats.
pub(crate) fn create_data_object(content: &ClipboardContent) -> GuiResult<ComPtr> {
    if content.is_empty() {
        return Err(GuiError::InvalidArgument("nothing to drag: the content is empty".into()));
    }
    let mut raw: *mut c_void = std::ptr::null_mut();
    // SAFETY: sem pasta nem itens, o shell cria um objeto de dados vazio; o ponteiro
    // de saída recebe uma referência que passa ao ComPtr. O objeto do shell guarda
    // também os formatos extras da imagem de arraste; sem ele, usa o próprio.
    let object = unsafe {
        let hr =
            SHCreateDataObject(std::ptr::null(), 0, std::ptr::null(), std::ptr::null_mut(), &IID_IDataObject, &mut raw);
        if hr >= 0 {
            ComPtr::from_raw(raw)
        } else {
            None
        }
    };
    let object = match object {
        Some(object) => object,
        None => crate::input::data_object::new_data_object().ok_or(GuiError::Unsupported("IDataObject"))?,
    };
    for (format, bytes) in content.blocks()? {
        let memory = GlobalMemory::with_bytes(&bytes)?;
        let format = FORMATETC {
            cfFormat: format as u16,
            ptd: std::ptr::null_mut(),
            dwAspect: DVASPECT_CONTENT,
            lindex: -1,
            tymed: TYMED_HGLOBAL,
        };
        let mut medium =
            STGMEDIUM { tymed: TYMED_HGLOBAL, hGlobal: memory.raw(), pUnkForRelease: std::ptr::null_mut() };
        // SAFETY: índice 7 de IDataObject = SetData(FORMATETC*, STGMEDIUM*, BOOL fRelease).
        // Com fRelease verdadeiro e sucesso, o objeto passa a ser dono do bloco.
        let hr = unsafe {
            let set: unsafe extern "system" fn(*mut c_void, *const FORMATETC, *mut STGMEDIUM, BOOL) -> HRESULT =
                object.method(7);
            set(object.as_ptr(), &format, &mut medium, TRUE)
        };
        GuiError::check_hresult("IDataObject::SetData", hr)?;
        let _ = memory.into_raw();
    }
    Ok(object)
}

/// Premultiplied-alpha 32-bit drag bitmap.
pub(crate) struct DragBitmap {
    pub(crate) bitmap: HBITMAP,
    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) hotspot: POINT,
}

/// `SHDRAGIMAGE`.
#[repr(C)]
pub(crate) struct ShDragImage {
    size: SIZE,
    offset: POINT,
    bitmap: HBITMAP,
    color_key: COLORREF,
}

/// Supplies shell drag image. Bitmap is always consumed: shell takes it or it is deleted here.
pub(crate) fn attach_drag_image(object: &ComPtr, image: DragBitmap) {
    let mut raw: *mut c_void = std::ptr::null_mut();
    // SAFETY: CLSID/IID documentados; a referência devolvida passa ao ComPtr. O índice 3
    // de IDragSourceHelper é InitializeFromBitmap(ShDragImage*, IDataObject*); com
    // S_OK o shell assume a posse do bitmap.
    unsafe {
        let hr = CoCreateInstance(
            &CLSID_DragDropHelper,
            std::ptr::null_mut(),
            CLSCTX_INPROC_SERVER,
            &IID_IDragSourceHelper,
            &mut raw,
        );
        if let (true, Some(helper)) = (hr >= 0, ComPtr::from_raw(raw)) {
            let info = ShDragImage {
                size: SIZE { cx: image.width, cy: image.height },
                offset: image.hotspot,
                bitmap: image.bitmap,
                color_key: 0xFFFF_FFFF,
            };
            let init: unsafe extern "system" fn(*mut c_void, *const ShDragImage, *mut c_void) -> HRESULT =
                helper.method(3);
            if init(helper.as_ptr(), &info, object.as_ptr()) >= 0 {
                return;
            }
        }
        DeleteObject(image.bitmap);
    }
}

#[repr(C)]
pub(crate) struct SourceVtbl {
    query_interface: unsafe extern "system" fn(*mut DropSource, *const GUID, *mut *mut c_void) -> HRESULT,
    add_ref: unsafe extern "system" fn(*mut DropSource) -> u32,
    release: unsafe extern "system" fn(*mut DropSource) -> u32,
    query_continue_drag: unsafe extern "system" fn(*mut DropSource, BOOL, DWORD) -> HRESULT,
    give_feedback: unsafe extern "system" fn(*mut DropSource, DWORD) -> HRESULT,
}

/// `IDropSource` COM object living only during `DoDragDrop`.
#[repr(C)]
pub(crate) struct DropSource {
    vtable: *const SourceVtbl,
    references: Cell<u32>,
}

pub(crate) static SOURCE_VTBL: SourceVtbl = SourceVtbl {
    query_interface: source_query_interface,
    add_ref: source_add_ref,
    release: source_release,
    query_continue_drag: source_query_continue,
    give_feedback: source_give_feedback,
};

pub(crate) unsafe extern "system" fn source_query_interface(
    this: *mut DropSource,
    iid: *const GUID,
    out: *mut *mut c_void,
) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if out.is_null() || iid.is_null() {
            return E_POINTER;
        }
        if *iid == IID_IUnknown || *iid == IID_IDropSource {
            source_add_ref(this);
            *out = this as *mut c_void;
            S_OK
        } else {
            *out = std::ptr::null_mut();
            E_NOINTERFACE
        }
    })
}

pub(crate) unsafe extern "system" fn source_add_ref(this: *mut DropSource) -> u32 {
    crate::core::ffi_guard::guard(0, move || {
        let count = (*this).references.get() + 1;
        (*this).references.set(count);
        count
    })
}

pub(crate) unsafe extern "system" fn source_release(this: *mut DropSource) -> u32 {
    crate::core::ffi_guard::guard(0, move || {
        let count = (*this).references.get() - 1;
        (*this).references.set(count);
        if count == 0 {
            drop(Box::from_raw(this));
        }
        count
    })
}

/// Default `IDropSource::QueryContinueDrag` rule.
pub(crate) fn continue_drag(escape: bool, keys: DWORD) -> HRESULT {
    if escape {
        DRAGDROP_S_CANCEL
    } else if keys as usize & (MK_LBUTTON | MK_RBUTTON) == 0 {
        DRAGDROP_S_DROP
    } else {
        S_OK
    }
}

pub(crate) unsafe extern "system" fn source_query_continue(
    _this: *mut DropSource,
    escape: BOOL,
    keys: DWORD,
) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || continue_drag(escape != 0, keys))
}

pub(crate) unsafe extern "system" fn source_give_feedback(_this: *mut DropSource, _effect: DWORD) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || DRAGDROP_S_USEDEFAULTCURSORS)
}

pub(crate) fn do_drag(content: &ClipboardContent, allowed: DWORD, image: Option<DragBitmap>) -> GuiResult<DropEffect> {
    let object = match create_data_object(content) {
        Ok(object) => object,
        Err(error) => {
            if let Some(image) = image {
                // SAFETY: bitmap próprio ainda não entregue a ninguém.
                unsafe { DeleteObject(image.bitmap) };
            }
            return Err(error);
        }
    };
    if let Some(image) = image {
        attach_drag_image(&object, image);
    }
    let source = Box::into_raw(Box::new(DropSource { vtable: &SOURCE_VTBL, references: Cell::new(1) }));
    // SAFETY: `source` é um objeto COM válido com uma referência, que passa ao ComPtr.
    let source = unsafe { ComPtr::from_raw(source as *mut c_void) }.ok_or(GuiError::Unsupported("IDropSource"))?;
    let mut effect: DWORD = 0;
    // SAFETY: objetos COM vivos durante a chamada; o laço modal roda nesta thread (STA
    // iniciada por OleInitialize em Application::new).
    let hr = unsafe { DoDragDrop(object.as_ptr(), source.as_ptr(), allowed, &mut effect) };
    match hr {
        DRAGDROP_S_DROP => Ok(DropEffect::from_bits(effect)),
        DRAGDROP_S_CANCEL => Ok(DropEffect::None),
        other => GuiError::check_hresult("DoDragDrop", other).map(|_| DropEffect::None),
    }
}

/// Starts OLE dragging with given content and waits for drop.
///
/// Call while mouse button is down, typically in `on_mouse_move` after a few pixels. `allowed` specifies permitted
/// actions; return indicates target action. [`DropEffect::Move`] requires deleting the original. `DropEffect::None`
/// means canceled (Esc) or rejected.
pub fn start_drag(widget: &impl Widget, content: &ClipboardContent, allowed: &[DropEffect]) -> GuiResult<DropEffect> {
    widget.core().require_alive()?;
    do_drag(content, DropEffect::mask(allowed), None)
}

/// Like [`start_drag`] with cursor-following image (Explorer ghost). `hotspot` is the image pixel under the cursor.
pub fn start_drag_with_image(
    widget: &impl Widget,
    content: &ClipboardContent,
    allowed: &[DropEffect],
    image: &PlusImage,
    hotspot: Point,
) -> GuiResult<DropEffect> {
    widget.core().require_alive()?;
    let size = image.size();
    let bitmap = premultiplied_bitmap(size.width, size.height, &image.to_pixels()?)?;
    let image =
        DragBitmap { bitmap, width: size.width, height: size.height, hotspot: POINT { x: hotspot.x, y: hotspot.y } };
    do_drag(content, DropEffect::mask(allowed), Some(image))
}

/// Top-down 32-bit DIB section; returns bitmap/pixel pointer.
pub(crate) fn dib_section(width: i32, height: i32) -> GuiResult<(HBITMAP, *mut u8)> {
    let header = BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: width,
        biHeight: -height,
        biPlanes: 1,
        biBitCount: 32,
        ..Default::default()
    };
    let mut bits: *mut c_void = std::ptr::null_mut();
    // SAFETY: cabeçalho completo; a seção é criada sem HDC (DIB_RGB_COLORS).
    let bitmap =
        unsafe { CreateDIBSection(std::ptr::null_mut(), &header, DIB_RGB_COLORS, &mut bits, std::ptr::null_mut(), 0) };
    if bitmap.is_null() || bits.is_null() {
        return Err(GuiError::last_os_error("CreateDIBSection"));
    }
    Ok((bitmap, bits as *mut u8))
}

pub(crate) fn premultiplied_bitmap(width: i32, height: i32, bgra: &[u8]) -> GuiResult<HBITMAP> {
    let (bitmap, bits) = dib_section(width, height)?;
    // SAFETY: a seção tem exatamente largura × altura × 4 bytes.
    let target = unsafe { std::slice::from_raw_parts_mut(bits, width as usize * height as usize * 4) };
    for (out, pixel) in target.chunks_exact_mut(4).zip(bgra.chunks_exact(4)) {
        let alpha = pixel[3] as u32;
        for channel in 0..3 {
            out[channel] = (pixel[channel] as u32 * alpha / 255) as u8;
        }
        out[3] = pixel[3];
    }
    Ok(bitmap)
}
