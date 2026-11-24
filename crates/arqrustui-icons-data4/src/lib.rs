//! Icon bytes from `help` through `mov_file` in the ArqRustUI bundle (part 4 of 8).
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

/// `help`: `.ico`, `.png` at 512 px, `.svg`.
pub const HELP: Bytes = (
    include_bytes!("../ico/help.ico"),
    Some(include_bytes!("../png/help.png")),
    Some(include_bytes!("../svg/help.svg")),
);
/// `home`: `.ico`, `.png` at 512 px, `.svg`.
pub const HOME: Bytes = (
    include_bytes!("../ico/home.ico"),
    Some(include_bytes!("../png/home.png")),
    Some(include_bytes!("../svg/home.svg")),
);
/// `html5`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const HTML5: Bytes = (include_bytes!("../ico/html5.ico"), None, None);
/// `html_file`: `.ico`, `.png` at 512 px.
pub const HTML_FILE: Bytes =
    (include_bytes!("../ico/html_file.ico"), Some(include_bytes!("../png/html_file.png")), None);
/// `ico_file`: `.ico`, `.png` at 512 px.
pub const ICO_FILE: Bytes = (include_bytes!("../ico/ico_file.ico"), Some(include_bytes!("../png/ico_file.png")), None);
/// `img_file`: `.ico`, `.png` at 512 px.
pub const IMG_FILE: Bytes = (include_bytes!("../ico/img_file.ico"), Some(include_bytes!("../png/img_file.png")), None);
/// `info`: `.ico`, `.png` at 512 px, `.svg`.
pub const INFO: Bytes = (
    include_bytes!("../ico/info.ico"),
    Some(include_bytes!("../png/info.png")),
    Some(include_bytes!("../svg/info.svg")),
);
/// `internet_download`: `.ico`, `.png` at 512 px.
pub const INTERNET_DOWNLOAD: Bytes =
    (include_bytes!("../ico/internet_download.ico"), Some(include_bytes!("../png/internet_download.png")), None);
/// `internet_upload`: `.ico`, `.png` at 512 px.
pub const INTERNET_UPLOAD: Bytes =
    (include_bytes!("../ico/internet_upload.ico"), Some(include_bytes!("../png/internet_upload.png")), None);
/// `ipa_file`: `.ico`, `.png` at 512 px.
pub const IPA_FILE: Bytes = (include_bytes!("../ico/ipa_file.ico"), Some(include_bytes!("../png/ipa_file.png")), None);
/// `iso_file`: `.ico`, `.png` at 512 px.
pub const ISO_FILE: Bytes = (include_bytes!("../ico/iso_file.ico"), Some(include_bytes!("../png/iso_file.png")), None);
/// `java`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const JAVA: Bytes = (include_bytes!("../ico/java.ico"), None, None);
/// `java_file`: `.ico`, `.png` at 512 px.
pub const JAVA_FILE: Bytes =
    (include_bytes!("../ico/java_file.ico"), Some(include_bytes!("../png/java_file.png")), None);
/// `javascript`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const JAVASCRIPT: Bytes = (include_bytes!("../ico/javascript.ico"), None, None);
/// `javascript_file`: `.ico`, `.png` at 512 px.
pub const JAVASCRIPT_FILE: Bytes =
    (include_bytes!("../ico/javascript_file.ico"), Some(include_bytes!("../png/javascript_file.png")), None);
/// `javascript_hex`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const JAVASCRIPT_HEX: Bytes = (include_bytes!("../ico/javascript_hex.ico"), None, None);
/// `jenkins`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const JENKINS: Bytes = (include_bytes!("../ico/jenkins.ico"), None, None);
/// `joystick`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const JOYSTICK: Bytes = (include_bytes!("../ico/joystick.ico"), None, None);
/// `jpeg_file`: `.ico`, `.png` at 512 px.
pub const JPEG_FILE: Bytes =
    (include_bytes!("../ico/jpeg_file.ico"), Some(include_bytes!("../png/jpeg_file.png")), None);
/// `jpg_file`: `.ico`, `.png` at 512 px.
pub const JPG_FILE: Bytes = (include_bytes!("../ico/jpg_file.ico"), Some(include_bytes!("../png/jpg_file.png")), None);
/// `json_file`: `.ico`, `.png` at 512 px.
pub const JSON_FILE: Bytes =
    (include_bytes!("../ico/json_file.ico"), Some(include_bytes!("../png/json_file.png")), None);
/// `jsx_file`: `.ico`, `.png` at 512 px.
pub const JSX_FILE: Bytes = (include_bytes!("../ico/jsx_file.ico"), Some(include_bytes!("../png/jsx_file.png")), None);
/// `kafka`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const KAFKA: Bytes = (include_bytes!("../ico/kafka.ico"), None, None);
/// `key`: `.ico`, `.png` at 512 px, `.svg`.
pub const KEY: Bytes =
    (include_bytes!("../ico/key.ico"), Some(include_bytes!("../png/key.png")), Some(include_bytes!("../svg/key.svg")));
/// `keyboard`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const KEYBOARD: Bytes = (include_bytes!("../ico/keyboard.ico"), None, None);
/// `keys`: `.ico`, `.png` at 512 px.
pub const KEYS: Bytes = (include_bytes!("../ico/keys.ico"), Some(include_bytes!("../png/keys.png")), None);
/// `kubernetes`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const KUBERNETES: Bytes = (include_bytes!("../ico/kubernetes.ico"), None, None);
/// `laptop`: `.ico`, `.png` at 512 px.
pub const LAPTOP: Bytes = (include_bytes!("../ico/laptop.ico"), Some(include_bytes!("../png/laptop.png")), None);
/// `lightbulb`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const LIGHTBULB: Bytes = (include_bytes!("../ico/lightbulb.ico"), None, None);
/// `line_chart`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const LINE_CHART: Bytes = (include_bytes!("../ico/line_chart.ico"), None, None);
/// `link`: `.ico`, `.png` at 512 px, `.svg`.
pub const LINK: Bytes = (
    include_bytes!("../ico/link.ico"),
    Some(include_bytes!("../png/link.png")),
    Some(include_bytes!("../svg/link.svg")),
);
/// `linux`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const LINUX: Bytes = (include_bytes!("../ico/linux.ico"), None, None);
/// `lock`: `.ico`, `.png` at 512 px, `.svg`.
pub const LOCK: Bytes = (
    include_bytes!("../ico/lock.ico"),
    Some(include_bytes!("../png/lock.png")),
    Some(include_bytes!("../svg/lock.svg")),
);
/// `m4a_file`: `.ico`, `.png` at 512 px.
pub const M4A_FILE: Bytes = (include_bytes!("../ico/m4a_file.ico"), Some(include_bytes!("../png/m4a_file.png")), None);
/// `mail`: `.ico`, `.png` at 512 px, `.svg`.
pub const MAIL: Bytes = (
    include_bytes!("../ico/mail.ico"),
    Some(include_bytes!("../png/mail.png")),
    Some(include_bytes!("../svg/mail.svg")),
);
/// `map_pin`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const MAP_PIN: Bytes = (include_bytes!("../ico/map_pin.ico"), None, None);
/// `medal`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const MEDAL: Bytes = (include_bytes!("../ico/medal.ico"), None, None);
/// `microphone`: `.ico`, `.png` at 512 px.
pub const MICROPHONE: Bytes =
    (include_bytes!("../ico/microphone.ico"), Some(include_bytes!("../png/microphone.png")), None);
/// `microsd_card`: `.ico`, `.png` at 512 px.
pub const MICROSD_CARD: Bytes =
    (include_bytes!("../ico/microsd_card.ico"), Some(include_bytes!("../png/microsd_card.png")), None);
/// `mkv_file`: `.ico`, `.png` at 512 px.
pub const MKV_FILE: Bytes = (include_bytes!("../ico/mkv_file.ico"), Some(include_bytes!("../png/mkv_file.png")), None);
/// `modules`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const MODULES: Bytes = (include_bytes!("../ico/modules.ico"), None, None);
/// `mongodb`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const MONGODB: Bytes = (include_bytes!("../ico/mongodb.ico"), None, None);
/// `monitor`: `.ico`, `.png` at 512 px, `.svg`.
pub const MONITOR: Bytes = (
    include_bytes!("../ico/monitor.ico"),
    Some(include_bytes!("../png/monitor.png")),
    Some(include_bytes!("../svg/monitor.svg")),
);
/// `monitoring`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const MONITORING: Bytes = (include_bytes!("../ico/monitoring.ico"), None, None);
/// `mouse`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const MOUSE: Bytes = (include_bytes!("../ico/mouse.ico"), None, None);
/// `mov_file`: `.ico`, `.png` at 512 px.
pub const MOV_FILE: Bytes = (include_bytes!("../ico/mov_file.ico"), Some(include_bytes!("../png/mov_file.png")), None);
