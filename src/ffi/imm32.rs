//! Declarations for `imm32.dll` (Input Method Manager).

use super::types::*;
use std::ffi::c_void;

pub const GCS_COMPSTR: DWORD = 0x0008;
pub const GCS_RESULTSTR: DWORD = 0x0800;
pub const CFS_POINT: DWORD = 0x0002;
pub const IACE_DEFAULT: DWORD = 0x0010;
pub const GCS_CURSORPOS: DWORD = 0x0080;
pub const ISC_SHOWUICANDIDATEWINDOW: u32 = 0x0000_0001;
pub const ISC_SHOWUIALLCANDIDATEWINDOW: u32 = 0x0000_000F;
pub const ISC_SHOWUICOMPOSITIONWINDOW: u32 = 0x8000_0000;
pub const IMN_CHANGECANDIDATE: usize = 0x0003;
pub const IMN_CLOSECANDIDATE: usize = 0x0004;
pub const IMN_OPENCANDIDATE: usize = 0x0005;
pub const NI_SELECTCANDIDATESTR: DWORD = 0x0012;
pub const NI_COMPOSITIONSTR: DWORD = 0x0015;
pub const NI_SETCANDIDATE_PAGESTART: DWORD = 0x0016;
pub const CPS_COMPLETE: DWORD = 0x0001;
pub const CPS_CANCEL: DWORD = 0x0004;

/// `CANDIDATELIST` header followed by `dwOffset[dwCount]` and strings.
#[repr(C)]
pub struct CANDIDATELIST {
    pub dwSize: DWORD,
    pub dwStyle: DWORD,
    pub dwCount: DWORD,
    pub dwSelection: DWORD,
    pub dwPageStart: DWORD,
    pub dwPageSize: DWORD,
    pub dwOffset: [DWORD; 1],
}

#[repr(C)]
pub struct COMPOSITIONFORM {
    pub dwStyle: DWORD,
    pub ptCurrentPos: POINT,
    pub rcArea: RECT,
}

#[link(name = "imm32")]
extern "system" {
    pub fn ImmGetContext(hwnd: HWND) -> HIMC;
    pub fn ImmReleaseContext(hwnd: HWND, himc: HIMC) -> BOOL;
    pub fn ImmGetCompositionStringW(himc: HIMC, index: DWORD, buffer: *mut c_void, size: DWORD) -> i32;
    pub fn ImmSetCompositionWindow(himc: HIMC, form: *const COMPOSITIONFORM) -> BOOL;
    pub fn ImmAssociateContextEx(hwnd: HWND, himc: HIMC, flags: DWORD) -> BOOL;
    pub fn ImmGetOpenStatus(himc: HIMC) -> BOOL;
    pub fn ImmSetOpenStatus(himc: HIMC, open: BOOL) -> BOOL;
    pub fn ImmGetCandidateListW(himc: HIMC, index: DWORD, list: *mut CANDIDATELIST, size: DWORD) -> DWORD;
    pub fn ImmNotifyIME(himc: HIMC, action: DWORD, index: DWORD, value: DWORD) -> BOOL;
}
