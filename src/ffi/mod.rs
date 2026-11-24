//! Handwritten Windows interoperability (FFI).
//!
//! Each submodule corresponds to an officially distributed Windows DLL and contains only declarations: `#[repr(C)]`
//! types, constants and `extern` blocks. No high-level logic. Names follow Microsoft documentation for easy lookup.
//!
//! Public **advanced API**; regular applications need not use it.

#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals, missing_docs)]
#![allow(clippy::upper_case_acronyms, clippy::too_many_arguments)]

pub mod comctl32;
pub mod comdlg32;
pub mod d2d1;
pub mod dwmapi;
pub mod dwrite;
pub mod dynamic;
pub mod gdi32;
pub mod gdiplus;
pub mod imm32;
pub mod kernel32;
pub mod ole32;
pub mod shell32;
pub mod types;
pub mod uiautomation;
pub mod user32;
pub mod uxtheme;
pub mod winrt;

pub use types::*;
pub mod tom;
