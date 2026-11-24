//! ArqRustUI icon bundle: bytes for 392 icons, collected from the eight data crates `arqrustui-icons-data1` through
//! `arqrustui-icons-data8`.
//!
//! Use `arqrustui::icons` instead, with names, categories, aliases and conversion to `HICON` and images (the
//! `arqrustui` crate's `icons` feature is enabled by default). This crate allows a bundle exceeding the crates.io 10 MB
//! limit to be published in parts.
//!
//! Each constant is `(.ico, .png, .svg)`; `None` for PNG means the PNG is the ICO's own 256 px frame.
#![no_std]

pub use arqrustui_icons_data1::*;
pub use arqrustui_icons_data2::*;
pub use arqrustui_icons_data3::*;
pub use arqrustui_icons_data4::*;
pub use arqrustui_icons_data5::*;
pub use arqrustui_icons_data6::*;
pub use arqrustui_icons_data7::*;
pub use arqrustui_icons_data8::*;
