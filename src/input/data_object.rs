//! Own `IDataObject` fallback when shell `SHCreateDataObject` is unavailable (reduced systems/Wine). Stores per-format
//! `HGLOBAL` blocks, returning copies via `GetData` like shell objects.

use crate::core::handles::GlobalMemory;
use crate::ffi::kernel32::GlobalFree;
use crate::ffi::ole32::*;
use crate::ffi::*;
use crate::system::clipboard::global_bytes;
use std::cell::{Cell, RefCell};
use std::ffi::c_void;

const E_NOTIMPL: HRESULT = 0x8000_4001_u32 as HRESULT;
const OLE_E_ADVISENOTSUPPORTED: HRESULT = 0x8004_0003_u32 as HRESULT;
const DATA_S_SAMEFORMATETC: HRESULT = 0x0004_0130;
const IID_ENUM_FORMATETC: GUID = GUID::new(0x0000_0103, 0, 0, [0xC0, 0, 0, 0, 0, 0, 0, 0x46]);

/// Stored format: (format, aspect, index).
type Key = (u16, DWORD, i32);

fn key_of(format: &FORMATETC) -> Key {
    (format.cfFormat, format.dwAspect, format.lindex)
}

fn formatetc((format, aspect, index): Key) -> FORMATETC {
    FORMATETC { cfFormat: format, ptd: std::ptr::null_mut(), dwAspect: aspect, lindex: index, tymed: TYMED_HGLOBAL }
}

#[repr(C)]
struct DataVtbl {
    query_interface: unsafe extern "system" fn(*mut DataObject, *const GUID, *mut *mut c_void) -> HRESULT,
    add_ref: unsafe extern "system" fn(*mut DataObject) -> u32,
    release: unsafe extern "system" fn(*mut DataObject) -> u32,
    get_data: unsafe extern "system" fn(*mut DataObject, *const FORMATETC, *mut STGMEDIUM) -> HRESULT,
    get_data_here: unsafe extern "system" fn(*mut DataObject, *const FORMATETC, *mut STGMEDIUM) -> HRESULT,
    query_get_data: unsafe extern "system" fn(*mut DataObject, *const FORMATETC) -> HRESULT,
    get_canonical: unsafe extern "system" fn(*mut DataObject, *const FORMATETC, *mut FORMATETC) -> HRESULT,
    set_data: unsafe extern "system" fn(*mut DataObject, *const FORMATETC, *mut STGMEDIUM, BOOL) -> HRESULT,
    enum_format: unsafe extern "system" fn(*mut DataObject, DWORD, *mut *mut c_void) -> HRESULT,
    d_advise: unsafe extern "system" fn(*mut DataObject, *const FORMATETC, DWORD, *mut c_void, *mut DWORD) -> HRESULT,
    d_unadvise: unsafe extern "system" fn(*mut DataObject, DWORD) -> HRESULT,
    enum_d_advise: unsafe extern "system" fn(*mut DataObject, *mut *mut c_void) -> HRESULT,
}

#[repr(C)]
struct DataObject {
    vtable: *const DataVtbl,
    references: Cell<u32>,
    items: RefCell<Vec<(Key, HGLOBAL)>>,
}

impl Drop for DataObject {
    fn drop(&mut self) {
        for (_, memory) in self.items.borrow_mut().drain(..) {
            // SAFETY: blocos pertencentes ao objeto.
            unsafe { GlobalFree(memory) };
        }
    }
}

static DATA_VTBL: DataVtbl = DataVtbl {
    query_interface: data_query_interface,
    add_ref: data_add_ref,
    release: data_release,
    get_data: data_get,
    get_data_here: data_get_here,
    query_get_data: data_query,
    get_canonical: data_canonical,
    set_data: data_set,
    enum_format: data_enum,
    d_advise: data_advise,
    d_unadvise: data_unadvise,
    enum_d_advise: data_enum_advise,
};

/// Creates an empty data object with one reference transferred to `ComPtr`.
pub(crate) fn new_data_object() -> Option<ComPtr> {
    let object = Box::into_raw(Box::new(DataObject {
        vtable: &DATA_VTBL,
        references: Cell::new(1),
        items: RefCell::new(Vec::new()),
    }));
    // SAFETY: objeto COM recém-criado com uma referência.
    unsafe { ComPtr::from_raw(object as *mut c_void) }
}

unsafe extern "system" fn data_query_interface(
    this: *mut DataObject,
    iid: *const GUID,
    out: *mut *mut c_void,
) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if out.is_null() || iid.is_null() {
            return E_POINTER;
        }
        if *iid == IID_IUnknown || *iid == IID_IDataObject {
            data_add_ref(this);
            *out = this as *mut c_void;
            S_OK
        } else {
            *out = std::ptr::null_mut();
            E_NOINTERFACE
        }
    })
}

unsafe extern "system" fn data_add_ref(this: *mut DataObject) -> u32 {
    crate::core::ffi_guard::guard(0, move || {
        let count = (*this).references.get() + 1;
        (*this).references.set(count);
        count
    })
}

unsafe extern "system" fn data_release(this: *mut DataObject) -> u32 {
    crate::core::ffi_guard::guard(0, move || {
        let count = (*this).references.get() - 1;
        (*this).references.set(count);
        if count == 0 {
            drop(Box::from_raw(this));
        }
        count
    })
}

fn find(object: &DataObject, format: &FORMATETC) -> Option<HGLOBAL> {
    if format.tymed & TYMED_HGLOBAL == 0 {
        return None;
    }
    let wanted = key_of(format);
    object.items.borrow().iter().find(|(key, _)| key.0 == wanted.0 && key.1 == wanted.1).map(|(_, memory)| *memory)
}

unsafe extern "system" fn data_get(this: *mut DataObject, format: *const FORMATETC, medium: *mut STGMEDIUM) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if format.is_null() || medium.is_null() {
            return E_POINTER;
        }
        let Some(memory) = find(&*this, &*format) else { return DV_E_FORMATETC };
        let Ok(bytes) = global_bytes(memory) else { return E_OUTOFMEMORY };
        let Ok(copy) = GlobalMemory::with_bytes(&bytes) else { return E_OUTOFMEMORY };
        *medium = STGMEDIUM { tymed: TYMED_HGLOBAL, hGlobal: copy.into_raw(), pUnkForRelease: std::ptr::null_mut() };
        S_OK
    })
}

unsafe extern "system" fn data_get_here(_this: *mut DataObject, _f: *const FORMATETC, _m: *mut STGMEDIUM) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || E_NOTIMPL)
}

unsafe extern "system" fn data_query(this: *mut DataObject, format: *const FORMATETC) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if format.is_null() {
            return E_POINTER;
        }
        if find(&*this, &*format).is_some() {
            S_OK
        } else {
            DV_E_FORMATETC
        }
    })
}

unsafe extern "system" fn data_canonical(
    _this: *mut DataObject,
    _in: *const FORMATETC,
    out: *mut FORMATETC,
) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if !out.is_null() {
            (*out).ptd = std::ptr::null_mut();
        }
        DATA_S_SAMEFORMATETC
    })
}

unsafe extern "system" fn data_set(
    this: *mut DataObject,
    format: *const FORMATETC,
    medium: *mut STGMEDIUM,
    release: BOOL,
) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if format.is_null() || medium.is_null() {
            return E_POINTER;
        }
        if (*medium).tymed != TYMED_HGLOBAL || (*medium).hGlobal.is_null() {
            return E_NOTIMPL;
        }
        let memory = if release != 0 {
            // Com fRelease o objeto passa a ser dono do meio.
            let memory = (*medium).hGlobal;
            if !(*medium).pUnkForRelease.is_null() {
                // Meio com dono externo: copia e devolve o original a quem o criou.
                let copy = global_bytes(memory).ok().and_then(|b| GlobalMemory::with_bytes(&b).ok());
                ReleaseStgMedium(medium);
                match copy {
                    Some(copy) => copy.into_raw(),
                    None => return E_OUTOFMEMORY,
                }
            } else {
                memory
            }
        } else {
            match global_bytes((*medium).hGlobal).ok().and_then(|b| GlobalMemory::with_bytes(&b).ok()) {
                Some(copy) => copy.into_raw(),
                None => return E_OUTOFMEMORY,
            }
        };
        let key = key_of(&*format);
        let mut items = (*this).items.borrow_mut();
        if let Some(slot) = items.iter_mut().find(|(existing, _)| existing.0 == key.0 && existing.1 == key.1) {
            GlobalFree(slot.1);
            slot.1 = memory;
        } else {
            items.push((key, memory));
        }
        S_OK
    })
}

unsafe extern "system" fn data_enum(this: *mut DataObject, direction: DWORD, out: *mut *mut c_void) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if out.is_null() {
            return E_POINTER;
        }
        if direction != 1 {
            *out = std::ptr::null_mut();
            return E_NOTIMPL;
        }
        let formats = (*this).items.borrow().iter().map(|(key, _)| *key).collect();
        *out = new_enumerator(formats, 0);
        S_OK
    })
}

unsafe extern "system" fn data_advise(
    _this: *mut DataObject,
    _f: *const FORMATETC,
    _flags: DWORD,
    _sink: *mut c_void,
    _connection: *mut DWORD,
) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || OLE_E_ADVISENOTSUPPORTED)
}

unsafe extern "system" fn data_unadvise(_this: *mut DataObject, _connection: DWORD) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || OLE_E_ADVISENOTSUPPORTED)
}

unsafe extern "system" fn data_enum_advise(_this: *mut DataObject, out: *mut *mut c_void) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if !out.is_null() {
            *out = std::ptr::null_mut();
        }
        OLE_E_ADVISENOTSUPPORTED
    })
}

// --- IEnumFORMATETC ---

#[repr(C)]
struct EnumVtbl {
    query_interface: unsafe extern "system" fn(*mut FormatEnum, *const GUID, *mut *mut c_void) -> HRESULT,
    add_ref: unsafe extern "system" fn(*mut FormatEnum) -> u32,
    release: unsafe extern "system" fn(*mut FormatEnum) -> u32,
    next: unsafe extern "system" fn(*mut FormatEnum, u32, *mut FORMATETC, *mut u32) -> HRESULT,
    skip: unsafe extern "system" fn(*mut FormatEnum, u32) -> HRESULT,
    reset: unsafe extern "system" fn(*mut FormatEnum) -> HRESULT,
    clone: unsafe extern "system" fn(*mut FormatEnum, *mut *mut c_void) -> HRESULT,
}

#[repr(C)]
struct FormatEnum {
    vtable: *const EnumVtbl,
    references: Cell<u32>,
    formats: Vec<Key>,
    position: Cell<usize>,
}

static ENUM_VTBL: EnumVtbl = EnumVtbl {
    query_interface: enum_query_interface,
    add_ref: enum_add_ref,
    release: enum_release,
    next: enum_next,
    skip: enum_skip,
    reset: enum_reset,
    clone: enum_clone,
};

fn new_enumerator(formats: Vec<Key>, position: usize) -> *mut c_void {
    Box::into_raw(Box::new(FormatEnum {
        vtable: &ENUM_VTBL,
        references: Cell::new(1),
        formats,
        position: Cell::new(position),
    })) as *mut c_void
}

unsafe extern "system" fn enum_query_interface(
    this: *mut FormatEnum,
    iid: *const GUID,
    out: *mut *mut c_void,
) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if out.is_null() || iid.is_null() {
            return E_POINTER;
        }
        if *iid == IID_IUnknown || *iid == IID_ENUM_FORMATETC {
            enum_add_ref(this);
            *out = this as *mut c_void;
            S_OK
        } else {
            *out = std::ptr::null_mut();
            E_NOINTERFACE
        }
    })
}

unsafe extern "system" fn enum_add_ref(this: *mut FormatEnum) -> u32 {
    crate::core::ffi_guard::guard(0, move || {
        let count = (*this).references.get() + 1;
        (*this).references.set(count);
        count
    })
}

unsafe extern "system" fn enum_release(this: *mut FormatEnum) -> u32 {
    crate::core::ffi_guard::guard(0, move || {
        let count = (*this).references.get() - 1;
        (*this).references.set(count);
        if count == 0 {
            drop(Box::from_raw(this));
        }
        count
    })
}

unsafe extern "system" fn enum_next(
    this: *mut FormatEnum,
    count: u32,
    out: *mut FORMATETC,
    fetched: *mut u32,
) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if out.is_null() {
            return E_POINTER;
        }
        let state = &*this;
        let mut done = 0u32;
        while done < count && state.position.get() < state.formats.len() {
            *out.add(done as usize) = formatetc(state.formats[state.position.get()]);
            state.position.set(state.position.get() + 1);
            done += 1;
        }
        if !fetched.is_null() {
            *fetched = done;
        }
        if done == count {
            S_OK
        } else {
            S_FALSE
        }
    })
}

unsafe extern "system" fn enum_skip(this: *mut FormatEnum, count: u32) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        let state = &*this;
        let target = state.position.get() + count as usize;
        state.position.set(target.min(state.formats.len()));
        if target <= state.formats.len() {
            S_OK
        } else {
            S_FALSE
        }
    })
}

unsafe extern "system" fn enum_reset(this: *mut FormatEnum) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        (*this).position.set(0);
        S_OK
    })
}

unsafe extern "system" fn enum_clone(this: *mut FormatEnum, out: *mut *mut c_void) -> HRESULT {
    crate::core::ffi_guard::guard_hresult(move || {
        if out.is_null() {
            return E_POINTER;
        }
        *out = new_enumerator((*this).formats.clone(), (*this).position.get());
        S_OK
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_copies_and_enumerates_formats() {
        let object = new_data_object().unwrap();
        let raw = object.as_ptr() as *mut DataObject;
        // SAFETY: chamadas diretas às funções da vtable com estruturas locais; os
        // blocos devolvidos são liberados com ReleaseStgMedium.
        unsafe {
            for (format, bytes) in [(13u16, b"a\0\0\0".to_vec()), (49000, vec![1, 2, 3])] {
                let memory = GlobalMemory::with_bytes(&bytes).unwrap();
                let mut medium =
                    STGMEDIUM { tymed: TYMED_HGLOBAL, hGlobal: memory.raw(), pUnkForRelease: std::ptr::null_mut() };
                let mut fmt = formatetc((format, DVASPECT_CONTENT, -1));
                assert_eq!(data_set(raw, &fmt, &mut medium, FALSE), S_OK, "sem fRelease o objeto copia");
                fmt.tymed = TYMED_HGLOBAL;
                assert_eq!(data_query(raw, &fmt), S_OK);
            }
            let missing = formatetc((1, DVASPECT_CONTENT, -1));
            assert_eq!(data_query(raw, &missing), DV_E_FORMATETC);
            let mut medium: STGMEDIUM = std::mem::zeroed();
            assert_eq!(data_get(raw, &formatetc((49000, DVASPECT_CONTENT, -1)), &mut medium), S_OK);
            assert_eq!(global_bytes(medium.hGlobal).unwrap()[..3], [1, 2, 3]);
            ReleaseStgMedium(&mut medium);

            let mut list: *mut c_void = std::ptr::null_mut();
            assert_eq!(data_enum(raw, 1, &mut list), S_OK);
            let list = list as *mut FormatEnum;
            let mut items = [formatetc((0, 0, 0)), formatetc((0, 0, 0)), formatetc((0, 0, 0))];
            let mut fetched = 0;
            assert_eq!(enum_next(list, 3, items.as_mut_ptr(), &mut fetched), S_FALSE);
            assert_eq!((fetched, items[0].cfFormat, items[1].cfFormat), (2, 13, 49000));
            assert_eq!(enum_reset(list), S_OK);
            assert_eq!(enum_skip(list, 5), S_FALSE);
            assert_eq!(enum_release(list), 0);
        }
    }
}
