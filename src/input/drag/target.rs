//! Drop target: [`DropData`](super::DropData), [`DropHandler`](super::DropHandler), `IDropTarget`.

use super::*;

use crate::core::native;
use crate::core::widget::Widget;
use crate::core::{GuiError, GuiResult};
use crate::graphics::PlusImage;
use crate::input::Modifiers;
use crate::system::clipboard::{
    decode_dib, decode_hdrop, decode_html, decode_text, format_id, global_bytes, ClipboardContent, HTML_FORMAT,
};
use crate::utilities::{to_logical, Point};
use std::cell::{Cell, RefCell};
use std::ffi::c_void;

use std::path::PathBuf;
use std::rc::Rc;

// ---------------------------------------------------------------------------
// Destino: DropData, DropHandler e IDropTarget
// ---------------------------------------------------------------------------

/// Data dragged over a target registered with [`accept_drops`].
///
/// Reads source data on demand: querying one format stays cheap even when a huge image is offered.
pub struct DropData {
    object: ComPtr,
    position: Point,
    keys: DWORD,
    allowed: DWORD,
}

impl DropData {
    /// Builds drag data from content as if from another program, useful for mouse-free [`accept_drops`] tests.
    pub fn from_content(content: &ClipboardContent, position: Point) -> GuiResult<DropData> {
        Ok(DropData {
            object: create_data_object(content)?,
            position,
            keys: 0,
            allowed: DROPEFFECT_COPY | DROPEFFECT_MOVE | DROPEFFECT_LINK,
        })
    }

    /// Cursor position in target-relative logical client units.
    pub fn position(&self) -> Point {
        self.position
    }

    /// Pressed modifiers (Ctrl usually copies, Shift moves).
    pub fn modifiers(&self) -> Modifiers {
        Modifiers { ctrl: self.keys & 0x8 != 0, shift: self.keys & 0x4 != 0, alt: self.keys & 0x20 != 0, win: false }
    }

    /// Whether the source allows this effect.
    pub fn allows(&self, effect: DropEffect) -> bool {
        effect == DropEffect::None || self.allowed & effect.bits() != 0
    }

    /// Windows suggested effect: Ctrl copy, Shift move, Ctrl+Shift link; no modifiers copies or moves if copy
    /// forbidden.
    pub fn suggested_effect(&self) -> DropEffect {
        let keys = self.modifiers();
        let wanted = match (keys.ctrl, keys.shift) {
            (true, true) => DropEffect::Link,
            (false, true) => DropEffect::Move,
            _ => DropEffect::Copy,
        };
        negotiate(wanted, self.allowed)
    }

    fn format(id: u32) -> FORMATETC {
        FORMATETC {
            cfFormat: id as u16,
            ptd: std::ptr::null_mut(),
            dwAspect: DVASPECT_CONTENT,
            lindex: -1,
            tymed: TYMED_HGLOBAL,
        }
    }

    fn has_id(&self, id: u32) -> bool {
        let format = DropData::format(id);
        // SAFETY: índice 5 de IDataObject = QueryGetData(FORMATETC*).
        unsafe {
            let query: unsafe extern "system" fn(*mut c_void, *const FORMATETC) -> HRESULT = self.object.method(5);
            query(self.object.as_ptr(), &format) == S_OK
        }
    }

    fn bytes(&self, id: u32) -> Option<Vec<u8>> {
        let format = DropData::format(id);
        // SAFETY: índice 3 de IDataObject = GetData(FORMATETC*, STGMEDIUM*). Em caso de
        // sucesso o meio pertence a quem chamou e é liberado com ReleaseStgMedium.
        unsafe {
            let mut medium: STGMEDIUM = std::mem::zeroed();
            let get: unsafe extern "system" fn(*mut c_void, *const FORMATETC, *mut STGMEDIUM) -> HRESULT =
                self.object.method(3);
            if get(self.object.as_ptr(), &format, &mut medium) != S_OK {
                return None;
            }
            let bytes = if medium.tymed == TYMED_HGLOBAL && !medium.hGlobal.is_null() {
                global_bytes(medium.hGlobal).ok()
            } else {
                None
            };
            ReleaseStgMedium(&mut medium);
            bytes
        }
    }

    /// Whether text is available.
    pub fn has_text(&self) -> bool {
        self.has_id(CF_UNICODETEXT)
    }

    /// Dragged text, if any.
    pub fn text(&self) -> Option<String> {
        self.bytes(CF_UNICODETEXT).map(|bytes| decode_text(&bytes))
    }

    /// Whether HTML is available.
    pub fn has_html(&self) -> bool {
        format_id(HTML_FORMAT).is_ok_and(|id| self.has_id(id))
    }

    /// Dragged HTML fragment from browser/editor, if any.
    pub fn html(&self) -> Option<String> {
        self.bytes(format_id(HTML_FORMAT).ok()?).and_then(|bytes| decode_html(&bytes))
    }

    /// Whether a DIB image is available.
    pub fn has_image(&self) -> bool {
        self.has_id(CF_DIB) || self.has_id(CF_DIBV5)
    }

    /// Dragged image, if any.
    pub fn image(&self) -> GuiResult<Option<PlusImage>> {
        let bytes = self.bytes(CF_DIB).or_else(|| self.bytes(CF_DIBV5));
        match bytes.as_deref().and_then(decode_dib) {
            Some((width, height, pixels)) => Ok(Some(PlusImage::from_pixels(width, height, &pixels)?)),
            None => Ok(None),
        }
    }

    /// Whether files are available, e.g. from Explorer.
    pub fn has_files(&self) -> bool {
        self.has_id(CF_HDROP)
    }

    /// Dragged files, empty if absent.
    pub fn files(&self) -> Vec<PathBuf> {
        self.bytes(CF_HDROP).map(|bytes| decode_hdrop(&bytes)).unwrap_or_default()
    }

    /// Whether the named custom format is available.
    pub fn has_format(&self, format: &str) -> bool {
        format_id(format).is_ok_and(|id| self.has_id(id))
    }

    /// Named custom format bytes, if any.
    pub fn data(&self, format: &str) -> Option<Vec<u8>> {
        self.bytes(format_id(format).ok()?)
    }

    /// Registered format names offered by source; unnamed standard text/image/file formats are excluded.
    pub fn format_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        // SAFETY: índice 8 de IDataObject = EnumFormatEtc(DWORD, IEnumFORMATETC**); a
        // enumeração (índice 3 = Next) é liberada pelo ComPtr.
        unsafe {
            let mut raw: *mut c_void = std::ptr::null_mut();
            let enumerate: unsafe extern "system" fn(*mut c_void, DWORD, *mut *mut c_void) -> HRESULT =
                self.object.method(8);
            if enumerate(self.object.as_ptr(), 1, &mut raw) != S_OK {
                return names;
            }
            let Some(list) = ComPtr::from_raw(raw) else { return names };
            let next: unsafe extern "system" fn(*mut c_void, u32, *mut FORMATETC, *mut u32) -> HRESULT = list.method(3);
            loop {
                let mut format = DropData::format(0);
                let mut fetched = 0u32;
                if next(list.as_ptr(), 1, &mut format, &mut fetched) != S_OK || fetched == 0 {
                    break;
                }
                let mut buffer = [0u16; 256];
                let len = GetClipboardFormatNameW(format.cfFormat as u32, buffer.as_mut_ptr(), buffer.len() as i32);
                if len > 0 {
                    let name = String::from_utf16_lossy(&buffer[..len as usize]);
                    if !names.contains(&name) {
                        names.push(name);
                    }
                }
            }
        }
        names
    }
}

pub(crate) type OverFn = Rc<dyn Fn(&DropData) -> DropEffect>;
pub(crate) type DroppedFn = Rc<dyn Fn(&DropData, DropEffect)>;
pub(crate) type LeaveFn = Rc<dyn Fn()>;

/// Drop target reactions for [`accept_drops`].
#[derive(Clone, Default)]
pub struct DropHandler {
    over: Option<OverFn>,
    dropped: Option<DroppedFn>,
    leave: Option<LeaveFn>,
}

impl DropHandler {
    /// Accepts everything with [`DropData::suggested_effect`].
    pub fn new() -> DropHandler {
        DropHandler::default()
    }

    /// Called on entry/movement: returns effect if dropped here ([`DropEffect::None`] rejects with forbidden cursor).
    pub fn on_over(mut self, f: impl Fn(&DropData) -> DropEffect + 'static) -> DropHandler {
        self.over = Some(Rc::new(f));
        self
    }

    /// Called on drop with negotiated effect.
    pub fn on_drop(mut self, f: impl Fn(&DropData, DropEffect) + 'static) -> DropHandler {
        self.dropped = Some(Rc::new(f));
        self
    }

    /// Called when cursor leaves without dropping or drag is canceled.
    pub fn on_leave(mut self, f: impl Fn() + 'static) -> DropHandler {
        self.leave = Some(Rc::new(f));
        self
    }

    fn effect(&self, data: &DropData) -> DropEffect {
        let wanted = match &self.over {
            Some(over) => over(data),
            None => data.suggested_effect(),
        };
        if wanted == DropEffect::None {
            DropEffect::None
        } else {
            negotiate(wanted, data.allowed)
        }
    }
}

#[repr(C)]
pub(crate) struct TargetVtbl {
    query_interface: unsafe extern "system" fn(*mut DropTarget, *const GUID, *mut *mut c_void) -> HRESULT,
    add_ref: unsafe extern "system" fn(*mut DropTarget) -> u32,
    release: unsafe extern "system" fn(*mut DropTarget) -> u32,
    drag_enter: unsafe extern "system" fn(*mut DropTarget, *mut c_void, DWORD, POINT, *mut DWORD) -> HRESULT,
    drag_over: unsafe extern "system" fn(*mut DropTarget, DWORD, POINT, *mut DWORD) -> HRESULT,
    drag_leave: unsafe extern "system" fn(*mut DropTarget) -> HRESULT,
    drop: unsafe extern "system" fn(*mut DropTarget, *mut c_void, DWORD, POINT, *mut DWORD) -> HRESULT,
}

/// Window `IDropTarget` COM object.
#[repr(C)]
pub(crate) struct DropTarget {
    vtable: *const TargetVtbl,
    references: Cell<u32>,
    hwnd: HWND,
    handler: DropHandler,
    /// `IDropTargetHelper`: draws source drag image over target.
    helper: Option<ComPtr>,
    current: RefCell<Option<ComPtr>>,
    allowed: Cell<DWORD>,
}

pub(crate) static TARGET_VTBL: TargetVtbl = TargetVtbl {
    query_interface: target_query_interface,
    add_ref: target_add_ref,
    release: target_release,
    drag_enter: target_drag_enter,
    drag_over: target_drag_over,
    drag_leave: target_drag_leave,
    drop: target_drop,
};

pub(crate) unsafe extern "system" fn target_query_interface(
    this: *mut DropTarget,
    iid: *const GUID,
    out: *mut *mut c_void,
) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if out.is_null() || iid.is_null() {
            return E_POINTER;
        }
        if *iid == IID_IUnknown || *iid == IID_IDropTarget {
            target_add_ref(this);
            *out = this as *mut c_void;
            S_OK
        } else {
            *out = std::ptr::null_mut();
            E_NOINTERFACE
        }
    })
}

pub(crate) unsafe extern "system" fn target_add_ref(this: *mut DropTarget) -> u32 {
    crate::core::ffi_guard::guard(0, move || {
        let count = (*this).references.get() + 1;
        (*this).references.set(count);
        count
    })
}

pub(crate) unsafe extern "system" fn target_release(this: *mut DropTarget) -> u32 {
    crate::core::ffi_guard::guard(0, move || {
        let count = (*this).references.get() - 1;
        (*this).references.set(count);
        if count == 0 {
            drop(Box::from_raw(this));
        }
        count
    })
}

/// Acquires an owned reference from a borrowed interface pointer.
///
/// # Safety
/// `raw` must be null or a valid COM interface.
pub(crate) unsafe fn borrowed(raw: *mut c_void) -> Option<ComPtr> {
    let pointer = ComPtr::from_raw(raw)?;
    let owned = pointer.clone();
    std::mem::forget(pointer);
    Some(owned)
}

impl DropTarget {
    fn data(&self, keys: DWORD, screen: POINT) -> Option<DropData> {
        let object = self.current.borrow().clone()?;
        let mut point = screen;
        // SAFETY: conversão de coordenadas de uma janela viva.
        unsafe { ScreenToClient(self.hwnd, &mut point) };
        let dpi = native::dpi_for_window(self.hwnd);
        Some(DropData {
            object,
            position: Point::new(to_logical(point.x, dpi), to_logical(point.y, dpi)),
            keys,
            allowed: self.allowed.get(),
        })
    }

    /// Runs user callback without unwinding across COM.
    pub(crate) fn guarded<T>(fallback: T, f: impl FnOnce() -> T) -> T {
        crate::core::ffi_guard::guard(fallback, f)
    }

    fn helper_call(&self, index: usize, call: impl FnOnce(*mut c_void, *const c_void)) {
        if let Some(helper) = &self.helper {
            // SAFETY: o chamador passa a assinatura exata do método `index`.
            unsafe {
                let vtable = *(helper.as_ptr() as *const *const *const c_void);
                call(helper.as_ptr(), *vtable.add(index));
            }
        }
    }
}

pub(crate) unsafe extern "system" fn target_drag_enter(
    this: *mut DropTarget,
    object: *mut c_void,
    keys: DWORD,
    point: POINT,
    effect: *mut DWORD,
) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if effect.is_null() {
            return E_POINTER;
        }
        let target = &*this;
        *target.current.borrow_mut() = borrowed(object);
        target.allowed.set(*effect);
        let chosen = match target.data(keys, point) {
            Some(data) => DropTarget::guarded(DropEffect::None, || target.handler.effect(&data)),
            None => DropEffect::None,
        };
        *effect = chosen.bits();
        let mut pt = point;
        target.helper_call(3, |helper, method| {
            let enter: unsafe extern "system" fn(*mut c_void, HWND, *mut c_void, *mut POINT, DWORD) -> HRESULT =
                std::mem::transmute(method);
            enter(helper, target.hwnd, object, &mut pt, chosen.bits());
        });
        S_OK
    })
}

pub(crate) unsafe extern "system" fn target_drag_over(
    this: *mut DropTarget,
    keys: DWORD,
    point: POINT,
    effect: *mut DWORD,
) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if effect.is_null() {
            return E_POINTER;
        }
        let target = &*this;
        target.allowed.set(*effect);
        let chosen = match target.data(keys, point) {
            Some(data) => DropTarget::guarded(DropEffect::None, || target.handler.effect(&data)),
            None => DropEffect::None,
        };
        *effect = chosen.bits();
        let mut pt = point;
        target.helper_call(5, |helper, method| {
            let over: unsafe extern "system" fn(*mut c_void, *mut POINT, DWORD) -> HRESULT =
                std::mem::transmute(method);
            over(helper, &mut pt, chosen.bits());
        });
        S_OK
    })
}

pub(crate) unsafe extern "system" fn target_drag_leave(this: *mut DropTarget) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        let target = &*this;
        target.current.borrow_mut().take();
        target.helper_call(4, |helper, method| {
            let leave: unsafe extern "system" fn(*mut c_void) -> HRESULT = std::mem::transmute(method);
            leave(helper);
        });
        if let Some(leave) = target.handler.leave.clone() {
            DropTarget::guarded((), || leave());
        }
        S_OK
    })
}

pub(crate) unsafe extern "system" fn target_drop(
    this: *mut DropTarget,
    object: *mut c_void,
    keys: DWORD,
    point: POINT,
    effect: *mut DWORD,
) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if effect.is_null() {
            return E_POINTER;
        }
        let target = &*this;
        *target.current.borrow_mut() = borrowed(object);
        target.allowed.set(*effect);
        let mut pt = point;
        let chosen = match target.data(keys, point) {
            Some(data) => DropTarget::guarded(DropEffect::None, || {
                let chosen = target.handler.effect(&data);
                if chosen != DropEffect::None {
                    if let Some(dropped) = target.handler.dropped.clone() {
                        dropped(&data, chosen);
                    }
                }
                chosen
            }),
            None => DropEffect::None,
        };
        target.helper_call(6, |helper, method| {
            let dropped: unsafe extern "system" fn(*mut c_void, *mut c_void, *mut POINT, DWORD) -> HRESULT =
                std::mem::transmute(method);
            dropped(helper, object, &mut pt, chosen.bits());
        });
        target.current.borrow_mut().take();
        *effect = chosen.bits();
        S_OK
    })
}

/// Registers a control/window as drop target.
///
/// Replaces prior registration. Unregistered on destruction or [`stop_accepting_drops`]. Windows draws the source drag
/// image (Explorer ghost) over the target.
pub fn accept_drops(widget: &impl Widget, handler: DropHandler) -> GuiResult<()> {
    let core = widget.core();
    let hwnd = core.require_alive()?;
    stop_accepting_drops(widget);
    let mut raw: *mut c_void = std::ptr::null_mut();
    // SAFETY: CLSID/IID documentados; a referência devolvida passa ao ComPtr (o
    // ajudante é opcional: sem ele não há imagem, mas o destino funciona).
    let helper = unsafe {
        let hr = CoCreateInstance(
            &CLSID_DragDropHelper,
            std::ptr::null_mut(),
            CLSCTX_INPROC_SERVER,
            &IID_IDropTargetHelper,
            &mut raw,
        );
        if hr >= 0 {
            ComPtr::from_raw(raw)
        } else {
            None
        }
    };
    let target = Box::into_raw(Box::new(DropTarget {
        vtable: &TARGET_VTBL,
        references: Cell::new(1),
        hwnd,
        handler,
        helper,
        current: RefCell::new(None),
        allowed: Cell::new(0),
    }));
    // SAFETY: `target` é um objeto COM válido com uma referência, que passa ao ComPtr.
    let target = unsafe { ComPtr::from_raw(target as *mut c_void) }.ok_or(GuiError::Unsupported("IDropTarget"))?;
    // SAFETY: janela viva desta thread; o OLE guarda a própria referência ao destino.
    let hr = unsafe { RegisterDragDrop(hwnd, target.as_ptr()) };
    GuiError::check_hresult("RegisterDragDrop", hr)?;
    *DragState::of(core).drop_target.borrow_mut() = Some(target);
    Ok(())
}

/// Unregisters [`accept_drops`], no effect if absent.
pub fn stop_accepting_drops(widget: &impl Widget) {
    let core = widget.core();
    let state = core.drag.borrow().clone();
    if let Some(state) = state {
        if state.drop_target.borrow_mut().take().is_some() {
            // SAFETY: desfaz o registro desta janela (sem efeito se ela já morreu).
            unsafe { RevokeDragDrop(core.hwnd()) };
        }
    }
}
