//! Icon bytes from `mp3_file` through `rar_file` in the ArqRustUI bundle (part 5 of 8).
//!
//! Data-only crate: the icon bundle is split into parts of at most 9 MB to fit the crates.io limit. Use
//! `arqrustui::icons` instead (the `icons` feature is enabled by default), through the `arqrustui-icons` facade.
//!
//! Each constant is `(.ico, .png, .svg)`. ICO frames are 16, 24, 32, 48, 64, 128 and 256 px; `None` for PNG means the
//! PNG is the ICO's 256 px frame, stored only once.
#![no_std]
#![warn(missing_docs)]

/// An icon as `(.ico, .png, .svg)`.
type Bytes = (&'static [u8], Option<&'static [u8]>, Option<&'static [u8]>);

/// `mp3_file`: `.ico`, `.png` at 512 px.
pub const MP3_FILE: Bytes = (include_bytes!("../ico/mp3_file.ico"), Some(include_bytes!("../png/mp3_file.png")), None);
/// `mp4_file`: `.ico`, `.png` at 512 px.
pub const MP4_FILE: Bytes = (include_bytes!("../ico/mp4_file.ico"), Some(include_bytes!("../png/mp4_file.png")), None);
/// `msi_file`: `.ico`, `.png` at 512 px.
pub const MSI_FILE: Bytes = (include_bytes!("../ico/msi_file.ico"), Some(include_bytes!("../png/msi_file.png")), None);
/// `music`: `.ico`, `.png` at 512 px.
pub const MUSIC: Bytes = (include_bytes!("../ico/music.ico"), Some(include_bytes!("../png/music.png")), None);
/// `music_keyboard`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const MUSIC_KEYBOARD: Bytes = (include_bytes!("../ico/music_keyboard.ico"), None, None);
/// `mysql`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const MYSQL: Bytes = (include_bytes!("../ico/mysql.ico"), None, None);
/// `network`: `.ico`, `.png` at 512 px, `.svg`.
pub const NETWORK: Bytes = (
    include_bytes!("../ico/network.ico"),
    Some(include_bytes!("../png/network.png")),
    Some(include_bytes!("../svg/network.svg")),
);
/// `network_switch`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const NETWORK_SWITCH: Bytes = (include_bytes!("../ico/network_switch.ico"), None, None);
/// `notepad`: `.ico`, `.png` at 512 px.
pub const NOTEPAD: Bytes = (include_bytes!("../ico/notepad.ico"), Some(include_bytes!("../png/notepad.png")), None);
/// `notification_bell`: `.ico`, `.png` at 512 px.
pub const NOTIFICATION_BELL: Bytes =
    (include_bytes!("../ico/notification_bell.ico"), Some(include_bytes!("../png/notification_bell.png")), None);
/// `ogg_file`: `.ico`, `.png` at 512 px.
pub const OGG_FILE: Bytes = (include_bytes!("../ico/ogg_file.ico"), Some(include_bytes!("../png/ogg_file.png")), None);
/// `openshift`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const OPENSHIFT: Bytes = (include_bytes!("../ico/openshift.ico"), None, None);
/// `optical_disc`: `.ico`, `.png` at 512 px.
pub const OPTICAL_DISC: Bytes =
    (include_bytes!("../ico/optical_disc.ico"), Some(include_bytes!("../png/optical_disc.png")), None);
/// `options`: `.ico`, `.png` at 512 px, `.svg`.
pub const OPTIONS: Bytes = (
    include_bytes!("../ico/options.ico"),
    Some(include_bytes!("../png/options.png")),
    Some(include_bytes!("../svg/options.svg")),
);
/// `oracle`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const ORACLE: Bytes = (include_bytes!("../ico/oracle.ico"), None, None);
/// `package`: `.ico`, `.png` at 512 px.
pub const PACKAGE: Bytes = (include_bytes!("../ico/package.ico"), Some(include_bytes!("../png/package.png")), None);
/// `package_open`: `.ico`, `.png` at 512 px.
pub const PACKAGE_OPEN: Bytes =
    (include_bytes!("../ico/package_open.ico"), Some(include_bytes!("../png/package_open.png")), None);
/// `package_search`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const PACKAGE_SEARCH: Bytes = (include_bytes!("../ico/package_search.ico"), None, None);
/// `paint_brush`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const PAINT_BRUSH: Bytes = (include_bytes!("../ico/paint_brush.ico"), None, None);
/// `palette`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const PALETTE: Bytes = (include_bytes!("../ico/palette.ico"), None, None);
/// `pdf_file`: `.ico`, `.png` at 512 px.
pub const PDF_FILE: Bytes = (include_bytes!("../ico/pdf_file.ico"), Some(include_bytes!("../png/pdf_file.png")), None);
/// `phone_call`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const PHONE_CALL: Bytes = (include_bytes!("../ico/phone_call.ico"), None, None);
/// `photo_gallery`: `.ico`, `.png` at 512 px.
pub const PHOTO_GALLERY: Bytes =
    (include_bytes!("../ico/photo_gallery.ico"), Some(include_bytes!("../png/photo_gallery.png")), None);
/// `php_file`: `.ico`, `.png` at 512 px.
pub const PHP_FILE: Bytes = (include_bytes!("../ico/php_file.ico"), Some(include_bytes!("../png/php_file.png")), None);
/// `picture`: `.ico`, `.png` at 512 px.
pub const PICTURE: Bytes = (include_bytes!("../ico/picture.ico"), Some(include_bytes!("../png/picture.png")), None);
/// `pie_chart_growth`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const PIE_CHART_GROWTH: Bytes = (include_bytes!("../ico/pie_chart_growth.ico"), None, None);
/// `pie_chart_report`: `.ico`, `.png` at 512 px.
pub const PIE_CHART_REPORT: Bytes =
    (include_bytes!("../ico/pie_chart_report.ico"), Some(include_bytes!("../png/pie_chart_report.png")), None);
/// `pin`: `.ico`, `.png` at 512 px.
pub const PIN: Bytes = (include_bytes!("../ico/pin.ico"), Some(include_bytes!("../png/pin.png")), None);
/// `play_disc`: `.ico`, `.png` at 512 px.
pub const PLAY_DISC: Bytes =
    (include_bytes!("../ico/play_disc.ico"), Some(include_bytes!("../png/play_disc.png")), None);
/// `plug`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const PLUG: Bytes = (include_bytes!("../ico/plug.ico"), None, None);
/// `png_file`: `.ico`, `.png` at 512 px.
pub const PNG_FILE: Bytes = (include_bytes!("../ico/png_file.ico"), Some(include_bytes!("../png/png_file.png")), None);
/// `popcorn`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const POPCORN: Bytes = (include_bytes!("../ico/popcorn.ico"), None, None);
/// `power`: `.ico`, `.png` at 512 px, `.svg`.
pub const POWER: Bytes = (
    include_bytes!("../ico/power.ico"),
    Some(include_bytes!("../png/power.png")),
    Some(include_bytes!("../svg/power.svg")),
);
/// `ppt_file`: `.ico`, `.png` at 512 px.
pub const PPT_FILE: Bytes = (include_bytes!("../ico/ppt_file.ico"), Some(include_bytes!("../png/ppt_file.png")), None);
/// `pptx_file`: `.ico`, `.png` at 512 px.
pub const PPTX_FILE: Bytes =
    (include_bytes!("../ico/pptx_file.ico"), Some(include_bytes!("../png/pptx_file.png")), None);
/// `presentation`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const PRESENTATION: Bytes = (include_bytes!("../ico/presentation.ico"), None, None);
/// `printer`: `.ico`, `.png` at 512 px, `.svg`.
pub const PRINTER: Bytes = (
    include_bytes!("../ico/printer.ico"),
    Some(include_bytes!("../png/printer.png")),
    Some(include_bytes!("../svg/printer.svg")),
);
/// `psd_file`: `.ico`, `.png` at 512 px.
pub const PSD_FILE: Bytes = (include_bytes!("../ico/psd_file.ico"), Some(include_bytes!("../png/psd_file.png")), None);
/// `puzzle`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const PUZZLE: Bytes = (include_bytes!("../ico/puzzle.ico"), None, None);
/// `py_file`: `.ico`, `.png` at 512 px.
pub const PY_FILE: Bytes = (include_bytes!("../ico/py_file.ico"), Some(include_bytes!("../png/py_file.png")), None);
/// `python`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const PYTHON: Bytes = (include_bytes!("../ico/python.ico"), None, None);
/// `qr_code`: `.ico`, `.png` at 512 px.
pub const QR_CODE: Bytes = (include_bytes!("../ico/qr_code.ico"), Some(include_bytes!("../png/qr_code.png")), None);
/// `rar_file`: `.ico`, `.png` at 512 px.
pub const RAR_FILE: Bytes = (include_bytes!("../ico/rar_file.ico"), Some(include_bytes!("../png/rar_file.png")), None);
