//! RichEdit/TOM constants for visual formatting without history changes.
use super::types::{GUID, UINT};
pub const EM_GETOLEINTERFACE: UINT = 0x043c;
pub const EM_EXGETSEL: UINT = 0x0434;
pub const EM_EXSETSEL: UINT = 0x0437;
pub const EM_GETSCROLLPOS: UINT = 0x04dd;
pub const EM_SETSCROLLPOS: UINT = 0x04de;
pub const TOM_SUSPEND: i32 = -9999995;
pub const TOM_RESUME: i32 = -9999994;
pub const IID_ITEXTDOCUMENT: GUID =
    GUID::new(0x8cc497c0, 0xa1df, 0x11ce, [0x80, 0x98, 0x00, 0xaa, 0x00, 0x47, 0xbe, 0x5d]);
#[repr(C)]
#[derive(Default)]
pub struct CharRange {
    pub start: i32,
    pub end: i32,
}
