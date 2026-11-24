//! Declarations for `ole32.dll`/`oleaut32.dll` and minimal COM utilities.

use super::types::*;
use std::ffi::c_void;
use std::ptr::NonNull;

pub const CLSCTX_INPROC_SERVER: DWORD = 0x1;
pub const VT_EMPTY: u16 = 0;
pub const VT_I4: u16 = 3;
pub const VT_BSTR: u16 = 8;
pub const VT_BOOL: u16 = 11;
pub const VT_R8: u16 = 5;
pub const VT_UNKNOWN: u16 = 13;
pub const IID_IUnknown: GUID = GUID::new(0x00000000, 0x0000, 0x0000, [0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46]);

/// `FORMATETC`: requested data object format.
#[repr(C)]
pub struct FORMATETC {
    pub cfFormat: u16,
    pub ptd: *mut c_void,
    pub dwAspect: DWORD,
    pub lindex: i32,
    pub tymed: DWORD,
}

/// `STGMEDIUM` (only union member `hGlobal` is used).
#[repr(C)]
pub struct STGMEDIUM {
    pub tymed: DWORD,
    pub hGlobal: HGLOBAL,
    pub pUnkForRelease: *mut c_void,
}

pub const DVASPECT_CONTENT: DWORD = 1;
pub const TYMED_HGLOBAL: DWORD = 1;
pub const DROPEFFECT_NONE: DWORD = 0;
pub const DROPEFFECT_COPY: DWORD = 1;
pub const DROPEFFECT_MOVE: DWORD = 2;
pub const DROPEFFECT_LINK: DWORD = 4;
pub const DRAGDROP_S_DROP: HRESULT = 0x0004_0100;
pub const DRAGDROP_S_CANCEL: HRESULT = 0x0004_0101;
pub const DRAGDROP_S_USEDEFAULTCURSORS: HRESULT = 0x0004_0102;
pub const DV_E_FORMATETC: HRESULT = 0x8004_0064_u32 as HRESULT;
pub const IID_IDataObject: GUID = GUID::new(0x0000_010E, 0, 0, [0xC0, 0, 0, 0, 0, 0, 0, 0x46]);
pub const IID_IDropSource: GUID = GUID::new(0x0000_0121, 0, 0, [0xC0, 0, 0, 0, 0, 0, 0, 0x46]);
pub const IID_IDropTarget: GUID = GUID::new(0x0000_0122, 0, 0, [0xC0, 0, 0, 0, 0, 0, 0, 0x46]);
pub const CLSID_DragDropHelper: GUID =
    GUID::new(0x4657_278A, 0x411B, 0x11D2, [0x83, 0x9A, 0x00, 0xC0, 0x4F, 0xD9, 0x18, 0xD0]);
pub const IID_IDropTargetHelper: GUID =
    GUID::new(0x4657_278B, 0x411B, 0x11D2, [0x83, 0x9A, 0x00, 0xC0, 0x4F, 0xD9, 0x18, 0xD0]);
pub const IID_IDragSourceHelper: GUID =
    GUID::new(0xDE5B_F786, 0x477A, 0x11D2, [0x83, 0x9D, 0x00, 0xC0, 0x4F, 0xD9, 0x18, 0xD0]);

/// `VARIANT` restricted to used fields, preserving original size.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VARIANT {
    pub vt: u16,
    pub reserved: [u16; 3],
    pub data: [usize; 2],
}

impl VARIANT {
    /// `VT_R8` containing `value`. The double occupies the first 8 bytes of `data` (8 bytes on 32-bit, 16 on 64-bit).
    /// Do not use `to_bits() as usize`: truncates high bits on 32-bit.
    pub fn from_f64(vt_r8: u16, value: f64) -> VARIANT {
        let mut variant = VARIANT { vt: vt_r8, reserved: [0; 3], data: [0; 2] };
        // SAFETY: `data` tem pelo menos 8 bytes em qualquer alvo; a escrita não exige alinhamento.
        unsafe { std::ptr::write_unaligned(variant.data.as_mut_ptr() as *mut f64, value) };
        variant
    }

    /// Reads the first 8 `data` bytes as double (`VT_R8`).
    pub fn as_f64(&self) -> f64 {
        // SAFETY: `data` tem pelo menos 8 bytes em qualquer alvo; a leitura não exige alinhamento.
        unsafe { std::ptr::read_unaligned(self.data.as_ptr() as *const f64) }
    }
}

#[link(name = "ole32")]
extern "system" {
    pub fn OleInitialize(reserved: *mut c_void) -> HRESULT;
    pub fn OleUninitialize();
    pub fn CoCreateInstance(
        clsid: *const GUID,
        outer: *mut c_void,
        context: DWORD,
        iid: *const GUID,
        out: *mut *mut c_void,
    ) -> HRESULT;
    pub fn CoTaskMemFree(mem: *mut c_void);
    pub fn DoDragDrop(data: *mut c_void, source: *mut c_void, allowed: DWORD, effect: *mut DWORD) -> HRESULT;
    pub fn RegisterDragDrop(hwnd: HWND, target: *mut c_void) -> HRESULT;
    pub fn RevokeDragDrop(hwnd: HWND) -> HRESULT;
    pub fn ReleaseStgMedium(medium: *mut STGMEDIUM);
    pub fn CreateStreamOnHGlobal(memory: HGLOBAL, delete_on_release: BOOL, stream: *mut *mut c_void) -> HRESULT;
    pub fn GetHGlobalFromStream(stream: *mut c_void, memory: *mut HGLOBAL) -> HRESULT;
}

#[link(name = "oleaut32")]
extern "system" {
    pub fn SysAllocString(text: PCWSTR) -> *mut u16;
    pub fn SysFreeString(text: *mut u16);
    pub fn SafeArrayCreateVector(vt: u16, lower: i32, count: u32) -> *mut c_void;
    pub fn SafeArrayPutElement(array: *mut c_void, indices: *const i32, value: *const c_void) -> HRESULT;
    pub fn SafeArrayGetElement(array: *mut c_void, indices: *const i32, value: *mut c_void) -> HRESULT;
    pub fn SafeArrayGetUBound(array: *mut c_void, dimension: u32, bound: *mut i32) -> HRESULT;
    pub fn SafeArrayDestroy(array: *mut c_void) -> HRESULT;
}

/// COM interface smart pointer calling `Release` on drop.
///
/// Owns **one** reference. `Clone` calls `AddRef`.
pub struct ComPtr(NonNull<c_void>);

impl ComPtr {
    /// Takes ownership of an already-counted reference.
    ///
    /// # Safety
    /// `raw` must be null or a valid COM interface pointer whose reference is transferred to `ComPtr`.
    pub unsafe fn from_raw(raw: *mut c_void) -> Option<Self> {
        NonNull::new(raw).map(Self)
    }

    /// Raw interface pointer without ownership transfer.
    pub fn as_ptr(&self) -> *mut c_void {
        self.0.as_ptr()
    }

    /// Reads vtable entry `index` as function pointer `F`.
    ///
    /// # Safety
    /// `F` must exactly match the method at vtable `index` of this interface.
    pub unsafe fn method<F: Copy>(&self, index: usize) -> F {
        debug_assert_eq!(std::mem::size_of::<F>(), std::mem::size_of::<*const c_void>());
        let vtable = *(self.0.as_ptr() as *const *const *const c_void);
        std::mem::transmute_copy::<*const c_void, F>(&*vtable.add(index))
    }
}

impl Clone for ComPtr {
    fn clone(&self) -> Self {
        // SAFETY: o índice 1 de toda interface COM é `IUnknown::AddRef`.
        unsafe {
            let add_ref: unsafe extern "system" fn(*mut c_void) -> u32 = self.method(1);
            add_ref(self.as_ptr());
        }
        Self(self.0)
    }
}

impl Drop for ComPtr {
    fn drop(&mut self) {
        // SAFETY: o índice 2 de toda interface COM é `IUnknown::Release`; o objeto
        // possui exatamente a referência que está sendo liberada.
        unsafe {
            let release: unsafe extern "system" fn(*mut c_void) -> u32 = self.method(2);
            release(self.as_ptr());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variant_keeps_all_eight_bytes_of_a_double() {
        const VT_R8: u16 = 5;
        // 4.0 = 0x4010_0000_0000_0000: a metade baixa é zero, então truncar para 32 bits daria 0.0.
        for value in [4.0, 5.0, -0.5, 1.0e300, f64::MIN_POSITIVE, std::f64::consts::PI] {
            let variant = VARIANT::from_f64(VT_R8, value);
            assert_eq!(variant.vt, VT_R8);
            assert_eq!(variant.as_f64().to_bits(), value.to_bits(), "{value}");
        }
        assert_eq!(std::mem::size_of::<VARIANT>(), if cfg!(target_pointer_width = "64") { 24 } else { 16 });
    }
}
