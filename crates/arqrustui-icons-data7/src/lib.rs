//! Icon bytes from `ts_file` through `zoom_out` and 15 social icons in the ArqRustUI bundle (part 7 of 8).
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

/// `ts_file`: `.ico`, `.png` at 512 px.
pub const TS_FILE: Bytes = (include_bytes!("../ico/ts_file.ico"), Some(include_bytes!("../png/ts_file.png")), None);
/// `tsx_file`: `.ico`, `.png` at 512 px.
pub const TSX_FILE: Bytes = (include_bytes!("../ico/tsx_file.ico"), Some(include_bytes!("../png/tsx_file.png")), None);
/// `typescript`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const TYPESCRIPT: Bytes = (include_bytes!("../ico/typescript.ico"), None, None);
/// `ui_design`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const UI_DESIGN: Bytes = (include_bytes!("../ico/ui_design.ico"), None, None);
/// `unlock`: `.ico`, `.png` at 512 px, `.svg`.
pub const UNLOCK: Bytes = (
    include_bytes!("../ico/unlock.ico"),
    Some(include_bytes!("../png/unlock.png")),
    Some(include_bytes!("../svg/unlock.svg")),
);
/// `up`: `.ico`, `.png` at 512 px, `.svg`.
pub const UP: Bytes =
    (include_bytes!("../ico/up.ico"), Some(include_bytes!("../png/up.png")), Some(include_bytes!("../svg/up.svg")));
/// `upload`: `.ico`, `.png` at 512 px, `.svg`.
pub const UPLOAD: Bytes = (
    include_bytes!("../ico/upload.ico"),
    Some(include_bytes!("../png/upload.png")),
    Some(include_bytes!("../svg/upload.svg")),
);
/// `upload_window`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const UPLOAD_WINDOW: Bytes = (include_bytes!("../ico/upload_window.ico"), None, None);
/// `usb_drive`: `.ico`, `.png` at 512 px.
pub const USB_DRIVE: Bytes =
    (include_bytes!("../ico/usb_drive.ico"), Some(include_bytes!("../png/usb_drive.png")), None);
/// `user`: `.ico`, `.png` at 512 px.
pub const USER: Bytes = (include_bytes!("../ico/user.ico"), Some(include_bytes!("../png/user.png")), None);
/// `user_female`: `.ico`, `.png` at 512 px.
pub const USER_FEMALE: Bytes =
    (include_bytes!("../ico/user_female.ico"), Some(include_bytes!("../png/user_female.png")), None);
/// `users`: `.ico`, `.png` at 512 px.
pub const USERS: Bytes = (include_bytes!("../ico/users.ico"), Some(include_bytes!("../png/users.png")), None);
/// `video_camera`: `.ico`, `.png` at 512 px.
pub const VIDEO_CAMERA: Bytes =
    (include_bytes!("../ico/video_camera.ico"), Some(include_bytes!("../png/video_camera.png")), None);
/// `vr_headset`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const VR_HEADSET: Bytes = (include_bytes!("../ico/vr_headset.ico"), None, None);
/// `vue`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const VUE: Bytes = (include_bytes!("../ico/vue.ico"), None, None);
/// `warning`: `.ico`, `.png` at 512 px, `.svg`.
pub const WARNING: Bytes = (
    include_bytes!("../ico/warning.ico"),
    Some(include_bytes!("../png/warning.png")),
    Some(include_bytes!("../svg/warning.svg")),
);
/// `wav_file`: `.ico`, `.png` at 512 px.
pub const WAV_FILE: Bytes = (include_bytes!("../ico/wav_file.ico"), Some(include_bytes!("../png/wav_file.png")), None);
/// `web_app`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const WEB_APP: Bytes = (include_bytes!("../ico/web_app.ico"), None, None);
/// `web_browser`: `.ico`, `.png` at 512 px.
pub const WEB_BROWSER: Bytes =
    (include_bytes!("../ico/web_browser.ico"), Some(include_bytes!("../png/web_browser.png")), None);
/// `web_link`: `.ico`, `.png` at 512 px.
pub const WEB_LINK: Bytes = (include_bytes!("../ico/web_link.ico"), Some(include_bytes!("../png/web_link.png")), None);
/// `web_search`: `.ico`, `.png` at 512 px.
pub const WEB_SEARCH: Bytes =
    (include_bytes!("../ico/web_search.ico"), Some(include_bytes!("../png/web_search.png")), None);
/// `web_window`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const WEB_WINDOW: Bytes = (include_bytes!("../ico/web_window.ico"), None, None);
/// `webcam`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const WEBCAM: Bytes = (include_bytes!("../ico/webcam.ico"), None, None);
/// `webm_file`: `.ico`, `.png` at 512 px.
pub const WEBM_FILE: Bytes =
    (include_bytes!("../ico/webm_file.ico"), Some(include_bytes!("../png/webm_file.png")), None);
/// `webp_file`: `.ico`, `.png` at 512 px.
pub const WEBP_FILE: Bytes =
    (include_bytes!("../ico/webp_file.ico"), Some(include_bytes!("../png/webp_file.png")), None);
/// `wifi`: `.ico`, `.png` at 512 px.
pub const WIFI: Bytes = (include_bytes!("../ico/wifi.ico"), Some(include_bytes!("../png/wifi.png")), None);
/// `window_stack`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const WINDOW_STACK: Bytes = (include_bytes!("../ico/window_stack.ico"), None, None);
/// `windows`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const WINDOWS: Bytes = (include_bytes!("../ico/windows.ico"), None, None);
/// `xls_file`: `.ico`, `.png` at 512 px.
pub const XLS_FILE: Bytes = (include_bytes!("../ico/xls_file.ico"), Some(include_bytes!("../png/xls_file.png")), None);
/// `xlsx_file`: `.ico`, `.png` at 512 px.
pub const XLSX_FILE: Bytes =
    (include_bytes!("../ico/xlsx_file.ico"), Some(include_bytes!("../png/xlsx_file.png")), None);
/// `xml_file`: `.ico`, `.png` at 512 px.
pub const XML_FILE: Bytes = (include_bytes!("../ico/xml_file.ico"), Some(include_bytes!("../png/xml_file.png")), None);
/// `yaml_file`: `.ico`, `.png` at 512 px.
pub const YAML_FILE: Bytes =
    (include_bytes!("../ico/yaml_file.ico"), Some(include_bytes!("../png/yaml_file.png")), None);
/// `zip_file`: `.ico`, `.png` at 512 px.
pub const ZIP_FILE: Bytes = (include_bytes!("../ico/zip_file.ico"), Some(include_bytes!("../png/zip_file.png")), None);
/// `zip_folder`: `.ico`, `.png` at 512 px.
pub const ZIP_FOLDER: Bytes =
    (include_bytes!("../ico/zip_folder.ico"), Some(include_bytes!("../png/zip_folder.png")), None);
/// `zoom_in`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const ZOOM_IN: Bytes = (include_bytes!("../ico/zoom_in.ico"), None, None);
/// `zoom_out`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const ZOOM_OUT: Bytes = (include_bytes!("../ico/zoom_out.ico"), None, None);

/// `baidu_tieba`: social icon in ICO, PNG at 512 px and SVG (Font Awesome/CC BY 4.0; see THIRD_PARTY_NOTICES.md).
pub const BAIDU_TIEBA: Bytes = (
    include_bytes!("../ico/baidu_tieba.ico"),
    Some(include_bytes!("../png/baidu_tieba.png")),
    Some(include_bytes!("../svg/baidu_tieba.svg")),
);
/// `behance`: `.ico` original social icon, `.png` = 256 px frame.
pub const BEHANCE: Bytes = (include_bytes!("../ico/behance.ico"), None, None);
/// `bluesky`: social icon in ICO, PNG at 512 px and SVG (Font Awesome/CC BY 4.0; see THIRD_PARTY_NOTICES.md).
pub const BLUESKY: Bytes = (
    include_bytes!("../ico/bluesky.ico"),
    Some(include_bytes!("../png/bluesky.png")),
    Some(include_bytes!("../svg/bluesky.svg")),
);
/// `discord`: social icon in ICO, PNG at 512 px and SVG (Font Awesome/CC BY 4.0; see THIRD_PARTY_NOTICES.md).
pub const DISCORD: Bytes = (
    include_bytes!("../ico/discord.ico"),
    Some(include_bytes!("../png/discord.png")),
    Some(include_bytes!("../svg/discord.svg")),
);
/// `dribbble`: `.ico` original social icon, `.png` = 256 px frame.
pub const DRIBBBLE: Bytes = (include_bytes!("../ico/dribbble.ico"), None, None);
/// `facebook`: social icon in ICO, PNG at 512 px and SVG (Font Awesome/CC BY 4.0; see THIRD_PARTY_NOTICES.md).
pub const FACEBOOK: Bytes = (
    include_bytes!("../ico/facebook.ico"),
    Some(include_bytes!("../png/facebook.png")),
    Some(include_bytes!("../svg/facebook.svg")),
);
/// `flickr`: `.ico` original social icon, `.png` = 256 px frame.
pub const FLICKR: Bytes = (include_bytes!("../ico/flickr.ico"), None, None);
/// `github_social`: `.ico` original social icon, `.png` = 256 px frame.
pub const GITHUB_SOCIAL: Bytes = (include_bytes!("../ico/github_social.ico"), None, None);
/// `gitlab_social`: `.ico` original social icon, `.png` = 256 px frame.
pub const GITLAB_SOCIAL: Bytes = (include_bytes!("../ico/gitlab_social.ico"), None, None);
/// `google_meet`: `.ico` original social icon, `.png` = 256 px frame.
pub const GOOGLE_MEET: Bytes = (include_bytes!("../ico/google_meet.ico"), None, None);
/// `instagram`: social icon in ICO, PNG at 512 px and SVG (Font Awesome/CC BY 4.0; see THIRD_PARTY_NOTICES.md).
pub const INSTAGRAM: Bytes = (
    include_bytes!("../ico/instagram.ico"),
    Some(include_bytes!("../png/instagram.png")),
    Some(include_bytes!("../svg/instagram.svg")),
);
/// `line`: social icon in ICO, PNG at 512 px and SVG (Font Awesome/CC BY 4.0; see THIRD_PARTY_NOTICES.md).
pub const LINE: Bytes = (
    include_bytes!("../ico/line.ico"),
    Some(include_bytes!("../png/line.png")),
    Some(include_bytes!("../svg/line.svg")),
);
/// `linkedin`: social icon in ICO, PNG at 512 px and SVG (Font Awesome/CC BY 4.0; see THIRD_PARTY_NOTICES.md).
pub const LINKEDIN: Bytes = (
    include_bytes!("../ico/linkedin.ico"),
    Some(include_bytes!("../png/linkedin.png")),
    Some(include_bytes!("../svg/linkedin.svg")),
);
/// `mastodon`: social icon in ICO, PNG at 512 px and SVG (Font Awesome/CC BY 4.0; see THIRD_PARTY_NOTICES.md).
pub const MASTODON: Bytes = (
    include_bytes!("../ico/mastodon.ico"),
    Some(include_bytes!("../png/mastodon.png")),
    Some(include_bytes!("../svg/mastodon.svg")),
);
/// `medium`: `.ico` original social icon, `.png` = 256 px frame.
pub const MEDIUM: Bytes = (include_bytes!("../ico/medium.ico"), None, None);
