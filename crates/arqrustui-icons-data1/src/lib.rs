//! Icon bytes from `7z_file` through `computer` in the ArqRustUI bundle (part 1 of 8).
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

/// `7z_file`: `.ico`, `.png` at 512 px.
pub const SEVEN_ZIP_FILE: Bytes =
    (include_bytes!("../ico/7z_file.ico"), Some(include_bytes!("../png/7z_file.png")), None);
/// `aac_file`: `.ico`, `.png` at 512 px.
pub const AAC_FILE: Bytes = (include_bytes!("../ico/aac_file.ico"), Some(include_bytes!("../png/aac_file.png")), None);
/// `accessibility`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const ACCESSIBILITY: Bytes = (include_bytes!("../ico/accessibility.ico"), None, None);
/// `add`: `.ico`, `.png` at 512 px, `.svg`.
pub const ADD: Bytes =
    (include_bytes!("../ico/add.ico"), Some(include_bytes!("../png/add.png")), Some(include_bytes!("../svg/add.svg")));
/// `ai_file`: `.ico`, `.png` at 512 px.
pub const AI_FILE: Bytes = (include_bytes!("../ico/ai_file.ico"), Some(include_bytes!("../png/ai_file.png")), None);
/// `alarm_clock`: `.ico`, `.png` at 512 px.
pub const ALARM_CLOCK: Bytes =
    (include_bytes!("../ico/alarm_clock.ico"), Some(include_bytes!("../png/alarm_clock.png")), None);
/// `angular`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const ANGULAR: Bytes = (include_bytes!("../ico/angular.ico"), None, None);
/// `api_hub`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const API_HUB: Bytes = (include_bytes!("../ico/api_hub.ico"), None, None);
/// `apk_file`: `.ico`, `.png` at 512 px.
pub const APK_FILE: Bytes = (include_bytes!("../ico/apk_file.ico"), Some(include_bytes!("../png/apk_file.png")), None);
/// `archive`: `.ico`, `.png` at 512 px, `.svg`.
pub const ARCHIVE: Bytes = (
    include_bytes!("../ico/archive.ico"),
    Some(include_bytes!("../png/archive.png")),
    Some(include_bytes!("../svg/archive.svg")),
);
/// `avi_file`: `.ico`, `.png` at 512 px.
pub const AVI_FILE: Bytes = (include_bytes!("../ico/avi_file.ico"), Some(include_bytes!("../png/avi_file.png")), None);
/// `azure_devops`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const AZURE_DEVOPS: Bytes = (include_bytes!("../ico/azure_devops.ico"), None, None);
/// `back`: `.ico`, `.png` at 512 px, `.svg`.
pub const BACK: Bytes = (
    include_bytes!("../ico/back.ico"),
    Some(include_bytes!("../png/back.png")),
    Some(include_bytes!("../svg/back.svg")),
);
/// `bar_chart_report`: `.ico`, `.png` at 512 px.
pub const BAR_CHART_REPORT: Bytes =
    (include_bytes!("../ico/bar_chart_report.ico"), Some(include_bytes!("../png/bar_chart_report.png")), None);
/// `barcode`: `.ico`, `.png` at 512 px.
pub const BARCODE: Bytes = (include_bytes!("../ico/barcode.ico"), Some(include_bytes!("../png/barcode.png")), None);
/// `barcode_scanner`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const BARCODE_SCANNER: Bytes = (include_bytes!("../ico/barcode_scanner.ico"), None, None);
/// `bat_file`: `.ico`, `.png` at 512 px.
pub const BAT_FILE: Bytes = (include_bytes!("../ico/bat_file.ico"), Some(include_bytes!("../png/bat_file.png")), None);
/// `battery`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const BATTERY: Bytes = (include_bytes!("../ico/battery.ico"), None, None);
/// `battery_charging`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const BATTERY_CHARGING: Bytes = (include_bytes!("../ico/battery_charging.ico"), None, None);
/// `blocked`: `.ico`, `.png` at 512 px.
pub const BLOCKED: Bytes = (include_bytes!("../ico/blocked.ico"), Some(include_bytes!("../png/blocked.png")), None);
/// `bmp_file`: `.ico`, `.png` at 512 px.
pub const BMP_FILE: Bytes = (include_bytes!("../ico/bmp_file.ico"), Some(include_bytes!("../png/bmp_file.png")), None);
/// `briefcase`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const BRIEFCASE: Bytes = (include_bytes!("../ico/briefcase.ico"), None, None);
/// `bug`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const BUG: Bytes = (include_bytes!("../ico/bug.ico"), None, None);
/// `burn_disc`: `.ico`, `.png` at 512 px.
pub const BURN_DISC: Bytes =
    (include_bytes!("../ico/burn_disc.ico"), Some(include_bytes!("../png/burn_disc.png")), None);
/// `c_file`: `.ico`, `.png` at 512 px.
pub const C_FILE: Bytes = (include_bytes!("../ico/c_file.ico"), Some(include_bytes!("../png/c_file.png")), None);
/// `calculator`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const CALCULATOR: Bytes = (include_bytes!("../ico/calculator.ico"), None, None);
/// `calendar`: `.ico`, `.png` at 512 px.
pub const CALENDAR: Bytes = (include_bytes!("../ico/calendar.ico"), Some(include_bytes!("../png/calendar.png")), None);
/// `camera`: `.ico`, `.png` at 512 px.
pub const CAMERA: Bytes = (include_bytes!("../ico/camera.ico"), Some(include_bytes!("../png/camera.png")), None);
/// `chat`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const CHAT: Bytes = (include_bytes!("../ico/chat.ico"), None, None);
/// `check`: `.ico`, `.png` at 512 px, `.svg`.
pub const CHECK: Bytes = (
    include_bytes!("../ico/check.ico"),
    Some(include_bytes!("../png/check.png")),
    Some(include_bytes!("../svg/check.svg")),
);
/// `checklist_done`: `.ico`, `.png` at 512 px.
pub const CHECKLIST_DONE: Bytes =
    (include_bytes!("../ico/checklist_done.ico"), Some(include_bytes!("../png/checklist_done.png")), None);
/// `checklist_pending`: `.ico`, `.png` at 512 px.
pub const CHECKLIST_PENDING: Bytes =
    (include_bytes!("../ico/checklist_pending.ico"), Some(include_bytes!("../png/checklist_pending.png")), None);
/// `clapperboard`: `.ico`, `.png` at 512 px.
pub const CLAPPERBOARD: Bytes =
    (include_bytes!("../ico/clapperboard.ico"), Some(include_bytes!("../png/clapperboard.png")), None);
/// `clipboard`: `.ico`, `.png` at 512 px, `.svg`.
pub const CLIPBOARD: Bytes = (
    include_bytes!("../ico/clipboard.ico"),
    Some(include_bytes!("../png/clipboard.png")),
    Some(include_bytes!("../svg/clipboard.svg")),
);
/// `clock`: `.ico`, `.png` at 512 px, `.svg`.
pub const CLOCK: Bytes = (
    include_bytes!("../ico/clock.ico"),
    Some(include_bytes!("../png/clock.png")),
    Some(include_bytes!("../svg/clock.svg")),
);
/// `close`: `.ico`, `.png` at 512 px, `.svg`.
pub const CLOSE: Bytes = (
    include_bytes!("../ico/close.ico"),
    Some(include_bytes!("../png/close.png")),
    Some(include_bytes!("../svg/close.svg")),
);
/// `cloud`: `.ico`, `.png` at 512 px.
pub const CLOUD: Bytes = (include_bytes!("../ico/cloud.ico"), Some(include_bytes!("../png/cloud.png")), None);
/// `cloud_download`: `.ico`, `.png` at 512 px.
pub const CLOUD_DOWNLOAD: Bytes =
    (include_bytes!("../ico/cloud_download.ico"), Some(include_bytes!("../png/cloud_download.png")), None);
/// `cloud_lock`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const CLOUD_LOCK: Bytes = (include_bytes!("../ico/cloud_lock.ico"), None, None);
/// `cloud_settings`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const CLOUD_SETTINGS: Bytes = (include_bytes!("../ico/cloud_settings.ico"), None, None);
/// `cloud_sync`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const CLOUD_SYNC: Bytes = (include_bytes!("../ico/cloud_sync.ico"), None, None);
/// `cloud_transfer`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const CLOUD_TRANSFER: Bytes = (include_bytes!("../ico/cloud_transfer.ico"), None, None);
/// `cloud_upload`: `.ico`, `.png` at 512 px.
pub const CLOUD_UPLOAD: Bytes =
    (include_bytes!("../ico/cloud_upload.ico"), Some(include_bytes!("../png/cloud_upload.png")), None);
/// `code_editor`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const CODE_EDITOR: Bytes = (include_bytes!("../ico/code_editor.ico"), None, None);
/// `code_file`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const CODE_FILE: Bytes = (include_bytes!("../ico/code_file.ico"), None, None);
/// `code_window`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const CODE_WINDOW: Bytes = (include_bytes!("../ico/code_window.ico"), None, None);
/// `compass`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const COMPASS: Bytes = (include_bytes!("../ico/compass.ico"), None, None);
/// `compressed_files`: `.ico`, `.png` at 512 px.
pub const COMPRESSED_FILES: Bytes =
    (include_bytes!("../ico/compressed_files.ico"), Some(include_bytes!("../png/compressed_files.png")), None);
/// `computer`: `.ico`, `.png` at 512 px.
pub const COMPUTER: Bytes = (include_bytes!("../ico/computer.ico"), Some(include_bytes!("../png/computer.png")), None);
