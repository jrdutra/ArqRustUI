//! Icon bytes from `raw_file` through `trophy` in the ArqRustUI bundle (part 6 of 8).
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

/// `raw_file`: `.ico`, `.png` at 512 px.
pub const RAW_FILE: Bytes = (include_bytes!("../ico/raw_file.ico"), Some(include_bytes!("../png/raw_file.png")), None);
/// `react`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const REACT: Bytes = (include_bytes!("../ico/react.ico"), None, None);
/// `record_disc`: `.ico`, `.png` at 512 px.
pub const RECORD_DISC: Bytes =
    (include_bytes!("../ico/record_disc.ico"), Some(include_bytes!("../png/record_disc.png")), None);
/// `recycle`: `.ico`, `.png` at 512 px, `.svg`.
pub const RECYCLE: Bytes = (
    include_bytes!("../ico/recycle.ico"),
    Some(include_bytes!("../png/recycle.png")),
    Some(include_bytes!("../svg/recycle.svg")),
);
/// `redis`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const REDIS: Bytes = (include_bytes!("../ico/redis.ico"), None, None);
/// `refresh`: `.ico`, `.png` at 512 px, `.svg`.
pub const REFRESH: Bytes = (
    include_bytes!("../ico/refresh.ico"),
    Some(include_bytes!("../png/refresh.png")),
    Some(include_bytes!("../svg/refresh.svg")),
);
/// `remove`: `.ico`, `.png` at 512 px, `.svg`.
pub const REMOVE: Bytes = (
    include_bytes!("../ico/remove.ico"),
    Some(include_bytes!("../png/remove.png")),
    Some(include_bytes!("../svg/remove.svg")),
);
/// `rest_api`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const REST_API: Bytes = (include_bytes!("../ico/rest_api.ico"), None, None);
/// `robot`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const ROBOT: Bytes = (include_bytes!("../ico/robot.ico"), None, None);
/// `router`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const ROUTER: Bytes = (include_bytes!("../ico/router.ico"), None, None);
/// `rs_file`: `.ico`, `.png` at 512 px.
pub const RS_FILE: Bytes = (include_bytes!("../ico/rs_file.ico"), Some(include_bytes!("../png/rs_file.png")), None);
/// `rss`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const RSS: Bytes = (include_bytes!("../ico/rss.ico"), None, None);
/// `ruler`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const RULER: Bytes = (include_bytes!("../ico/ruler.ico"), None, None);
/// `run`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const RUN: Bytes = (include_bytes!("../ico/run.ico"), None, None);
/// `save`: `.ico`, `.png` at 512 px, `.svg`.
pub const SAVE: Bytes = (
    include_bytes!("../ico/save.ico"),
    Some(include_bytes!("../png/save.png")),
    Some(include_bytes!("../svg/save.svg")),
);
/// `scanner`: `.ico`, `.png` at 512 px.
pub const SCANNER: Bytes = (include_bytes!("../ico/scanner.ico"), Some(include_bytes!("../png/scanner.png")), None);
/// `sd_card`: `.ico`, `.png` at 512 px.
pub const SD_CARD: Bytes = (include_bytes!("../ico/sd_card.ico"), Some(include_bytes!("../png/sd_card.png")), None);
/// `search`: `.ico`, `.png` at 512 px, `.svg`.
pub const SEARCH: Bytes = (
    include_bytes!("../ico/search.ico"),
    Some(include_bytes!("../png/search.png")),
    Some(include_bytes!("../svg/search.svg")),
);
/// `server`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const SERVER: Bytes = (include_bytes!("../ico/server.ico"), None, None);
/// `server_rack`: `.ico`, `.png` at 512 px.
pub const SERVER_RACK: Bytes =
    (include_bytes!("../ico/server_rack.ico"), Some(include_bytes!("../png/server_rack.png")), None);
/// `settings`: `.ico`, `.png` at 512 px.
pub const SETTINGS: Bytes = (include_bytes!("../ico/settings.ico"), Some(include_bytes!("../png/settings.png")), None);
/// `sh_file`: `.ico`, `.png` at 512 px.
pub const SH_FILE: Bytes = (include_bytes!("../ico/sh_file.ico"), Some(include_bytes!("../png/sh_file.png")), None);
/// `shield`: `.ico`, `.png` at 512 px, `.svg`.
pub const SHIELD: Bytes = (
    include_bytes!("../ico/shield.ico"),
    Some(include_bytes!("../png/shield.png")),
    Some(include_bytes!("../svg/shield.svg")),
);
/// `shield_check`: `.ico`, `.png` at 512 px.
pub const SHIELD_CHECK: Bytes =
    (include_bytes!("../ico/shield_check.ico"), Some(include_bytes!("../png/shield_check.png")), None);
/// `shield_error`: `.ico`, `.png` at 512 px.
pub const SHIELD_ERROR: Bytes =
    (include_bytes!("../ico/shield_error.ico"), Some(include_bytes!("../png/shield_error.png")), None);
/// `shield_lock`: `.ico`, `.png` at 512 px.
pub const SHIELD_LOCK: Bytes =
    (include_bytes!("../ico/shield_lock.ico"), Some(include_bytes!("../png/shield_lock.png")), None);
/// `shopping_bag`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const SHOPPING_BAG: Bytes = (include_bytes!("../ico/shopping_bag.ico"), None, None);
/// `shopping_cart`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const SHOPPING_CART: Bytes = (include_bytes!("../ico/shopping_cart.ico"), None, None);
/// `smartphone`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const SMARTPHONE: Bytes = (include_bytes!("../ico/smartphone.ico"), None, None);
/// `speaker`: `.ico`, `.png` at 512 px.
pub const SPEAKER: Bytes = (include_bytes!("../ico/speaker.ico"), Some(include_bytes!("../png/speaker.png")), None);
/// `spreadsheet`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const SPREADSHEET: Bytes = (include_bytes!("../ico/spreadsheet.ico"), None, None);
/// `spring`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const SPRING: Bytes = (include_bytes!("../ico/spring.ico"), None, None);
/// `sql_file`: `.ico`, `.png` at 512 px.
pub const SQL_FILE: Bytes = (include_bytes!("../ico/sql_file.ico"), Some(include_bytes!("../png/sql_file.png")), None);
/// `ssd`: `.ico`, `.png` at 512 px.
pub const SSD: Bytes = (include_bytes!("../ico/ssd.ico"), Some(include_bytes!("../png/ssd.png")), None);
/// `stamp`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const STAMP: Bytes = (include_bytes!("../ico/stamp.ico"), None, None);
/// `sticky_note`: `.ico`, `.png` at 512 px.
pub const STICKY_NOTE: Bytes =
    (include_bytes!("../ico/sticky_note.ico"), Some(include_bytes!("../png/sticky_note.png")), None);
/// `svg_file`: `.ico`, `.png` at 512 px.
pub const SVG_FILE: Bytes = (include_bytes!("../ico/svg_file.ico"), Some(include_bytes!("../png/svg_file.png")), None);
/// `sync`: `.ico`, `.png` at 512 px.
pub const SYNC: Bytes = (include_bytes!("../ico/sync.ico"), Some(include_bytes!("../png/sync.png")), None);
/// `sync_window`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const SYNC_WINDOW: Bytes = (include_bytes!("../ico/sync_window.ico"), None, None);
/// `tablet`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const TABLET: Bytes = (include_bytes!("../ico/tablet.ico"), None, None);
/// `tar_file`: `.ico`, `.png` at 512 px.
pub const TAR_FILE: Bytes = (include_bytes!("../ico/tar_file.ico"), Some(include_bytes!("../png/tar_file.png")), None);
/// `team_window`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const TEAM_WINDOW: Bytes = (include_bytes!("../ico/team_window.ico"), None, None);
/// `terminal`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const TERMINAL: Bytes = (include_bytes!("../ico/terminal.ico"), None, None);
/// `threads`: social icon in ICO, PNG at 512 px and SVG (Font Awesome/CC BY 4.0; see THIRD_PARTY_NOTICES.md).
pub const THREADS: Bytes = (
    include_bytes!("../ico/threads.ico"),
    Some(include_bytes!("../png/threads.png")),
    Some(include_bytes!("../svg/threads.svg")),
);
/// `tiff_file`: `.ico`, `.png` at 512 px.
pub const TIFF_FILE: Bytes =
    (include_bytes!("../ico/tiff_file.ico"), Some(include_bytes!("../png/tiff_file.png")), None);
/// `tools`: `.ico`, `.png` at 512 px.
pub const TOOLS: Bytes = (include_bytes!("../ico/tools.ico"), Some(include_bytes!("../png/tools.png")), None);
/// `trophy`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const TROPHY: Bytes = (include_bytes!("../ico/trophy.ico"), None, None);
