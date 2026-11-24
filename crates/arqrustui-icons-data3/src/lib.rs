//! Icon bytes from `eps_file` through `hearing` in the ArqRustUI bundle (part 3 of 8).
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

/// `eps_file`: `.ico`, `.png` at 512 px.
pub const EPS_FILE: Bytes = (include_bytes!("../ico/eps_file.ico"), Some(include_bytes!("../png/eps_file.png")), None);
/// `exe_file`: `.ico`, `.png` at 512 px.
pub const EXE_FILE: Bytes = (include_bytes!("../ico/exe_file.ico"), Some(include_bytes!("../png/exe_file.png")), None);
/// `eyedropper`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const EYEDROPPER: Bytes = (include_bytes!("../ico/eyedropper.ico"), None, None);
/// `favorite`: `.ico`, `.png` at 512 px.
pub const FAVORITE: Bytes = (include_bytes!("../ico/favorite.ico"), Some(include_bytes!("../png/favorite.png")), None);
/// `favorite_add`: `.ico`, `.png` at 512 px.
pub const FAVORITE_ADD: Bytes =
    (include_bytes!("../ico/favorite_add.ico"), Some(include_bytes!("../png/favorite_add.png")), None);
/// `favorite_remove`: `.ico`, `.png` at 512 px.
pub const FAVORITE_REMOVE: Bytes =
    (include_bytes!("../ico/favorite_remove.ico"), Some(include_bytes!("../png/favorite_remove.png")), None);
/// `file`: `.ico`, `.png` at 512 px, `.svg`.
pub const FILE: Bytes = (
    include_bytes!("../ico/file.ico"),
    Some(include_bytes!("../png/file.png")),
    Some(include_bytes!("../svg/file.svg")),
);
/// `film_reel`: `.ico`, `.png` at 512 px.
pub const FILM_REEL: Bytes =
    (include_bytes!("../ico/film_reel.ico"), Some(include_bytes!("../png/film_reel.png")), None);
/// `firewall`: `.ico`, `.png` at 512 px.
pub const FIREWALL: Bytes = (include_bytes!("../ico/firewall.ico"), Some(include_bytes!("../png/firewall.png")), None);
/// `flac_file`: `.ico`, `.png` at 512 px.
pub const FLAC_FILE: Bytes =
    (include_bytes!("../ico/flac_file.ico"), Some(include_bytes!("../png/flac_file.png")), None);
/// `flag_blue`: `.ico`, `.png` at 512 px.
pub const FLAG_BLUE: Bytes =
    (include_bytes!("../ico/flag_blue.ico"), Some(include_bytes!("../png/flag_blue.png")), None);
/// `flag_check`: `.ico`, `.png` at 512 px.
pub const FLAG_CHECK: Bytes =
    (include_bytes!("../ico/flag_check.ico"), Some(include_bytes!("../png/flag_check.png")), None);
/// `flag_error`: `.ico`, `.png` at 512 px.
pub const FLAG_ERROR: Bytes =
    (include_bytes!("../ico/flag_error.ico"), Some(include_bytes!("../png/flag_error.png")), None);
/// `flag_green`: `.ico`, `.png` at 512 px.
pub const FLAG_GREEN: Bytes =
    (include_bytes!("../ico/flag_green.ico"), Some(include_bytes!("../png/flag_green.png")), None);
/// `flag_red`: `.ico`, `.png` at 512 px.
pub const FLAG_RED: Bytes = (include_bytes!("../ico/flag_red.ico"), Some(include_bytes!("../png/flag_red.png")), None);
/// `flowchart`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const FLOWCHART: Bytes = (include_bytes!("../ico/flowchart.ico"), None, None);
/// `folder`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const FOLDER: Bytes = (include_bytes!("../ico/folder.ico"), None, None);
/// `folder_open`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const FOLDER_OPEN: Bytes = (include_bytes!("../ico/folder_open.ico"), None, None);
/// `forward`: `.ico`, `.png` at 512 px, `.svg`.
pub const FORWARD: Bytes = (
    include_bytes!("../ico/forward.ico"),
    Some(include_bytes!("../png/forward.png")),
    Some(include_bytes!("../svg/forward.svg")),
);
/// `fountain_pen`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const FOUNTAIN_PEN: Bytes = (include_bytes!("../ico/fountain_pen.ico"), None, None);
/// `gamepad`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const GAMEPAD: Bytes = (include_bytes!("../ico/gamepad.ico"), None, None);
/// `gif_file`: `.ico`, `.png` at 512 px.
pub const GIF_FILE: Bytes = (include_bytes!("../ico/gif_file.ico"), Some(include_bytes!("../png/gif_file.png")), None);
/// `gift`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const GIFT: Bytes = (include_bytes!("../ico/gift.ico"), None, None);
/// `git`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const GIT: Bytes = (include_bytes!("../ico/git.ico"), None, None);
/// `github`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const GITHUB: Bytes = (include_bytes!("../ico/github.ico"), None, None);
/// `gitlab`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const GITLAB: Bytes = (include_bytes!("../ico/gitlab.ico"), None, None);
/// `global_network`: `.ico`, `.png` at 512 px.
pub const GLOBAL_NETWORK: Bytes =
    (include_bytes!("../ico/global_network.ico"), Some(include_bytes!("../png/global_network.png")), None);
/// `globe`: `.ico`, `.png` at 512 px, `.svg`.
pub const GLOBE: Bytes = (
    include_bytes!("../ico/globe.ico"),
    Some(include_bytes!("../png/globe.png")),
    Some(include_bytes!("../svg/globe.svg")),
);
/// `go_file`: `.ico`, `.png` at 512 px.
pub const GO_FILE: Bytes = (include_bytes!("../ico/go_file.ico"), Some(include_bytes!("../png/go_file.png")), None);
/// `gps_navigation`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const GPS_NAVIGATION: Bytes = (include_bytes!("../ico/gps_navigation.ico"), None, None);
/// `gz_file`: `.ico`, `.png` at 512 px.
pub const GZ_FILE: Bytes = (include_bytes!("../ico/gz_file.ico"), Some(include_bytes!("../png/gz_file.png")), None);
/// `h_file`: `.ico`, `.png` at 512 px.
pub const H_FILE: Bytes = (include_bytes!("../ico/h_file.ico"), Some(include_bytes!("../png/h_file.png")), None);
/// `hammer`: `.ico`, `.png` at 512 px, `.svg`.
pub const HAMMER: Bytes = (
    include_bytes!("../ico/hammer.ico"),
    Some(include_bytes!("../png/hammer.png")),
    Some(include_bytes!("../svg/hammer.svg")),
);
/// `hand_backhand_point`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const HAND_BACKHAND_POINT: Bytes = (include_bytes!("../ico/hand_backhand_point.ico"), None, None);
/// `hand_call_me`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const HAND_CALL_ME: Bytes = (include_bytes!("../ico/hand_call_me.ico"), None, None);
/// `hand_clap`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const HAND_CLAP: Bytes = (include_bytes!("../ico/hand_clap.ico"), None, None);
/// `hand_fingers_crossed`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const HAND_FINGERS_CROSSED: Bytes = (include_bytes!("../ico/hand_fingers_crossed.ico"), None, None);
/// `hand_fist`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const HAND_FIST: Bytes = (include_bytes!("../ico/hand_fist.ico"), None, None);
/// `hand_flexed_biceps`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const HAND_FLEXED_BICEPS: Bytes = (include_bytes!("../ico/hand_flexed_biceps.ico"), None, None);
/// `hand_love_you`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const HAND_LOVE_YOU: Bytes = (include_bytes!("../ico/hand_love_you.ico"), None, None);
/// `hand_ok`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const HAND_OK: Bytes = (include_bytes!("../ico/hand_ok.ico"), None, None);
/// `hand_open_hands`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const HAND_OPEN_HANDS: Bytes = (include_bytes!("../ico/hand_open_hands.ico"), None, None);
/// `hand_palms_up`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const HAND_PALMS_UP: Bytes = (include_bytes!("../ico/hand_palms_up.ico"), None, None);
/// `hand_peace`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const HAND_PEACE: Bytes = (include_bytes!("../ico/hand_peace.ico"), None, None);
/// `hand_pinch`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const HAND_PINCH: Bytes = (include_bytes!("../ico/hand_pinch.ico"), None, None);
/// `hand_pinched_fingers`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const HAND_PINCHED_FINGERS: Bytes = (include_bytes!("../ico/hand_pinched_fingers.ico"), None, None);
/// `hand_point_left`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const HAND_POINT_LEFT: Bytes = (include_bytes!("../ico/hand_point_left.ico"), None, None);
/// `hand_point_up`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const HAND_POINT_UP: Bytes = (include_bytes!("../ico/hand_point_up.ico"), None, None);
/// `hand_pray`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const HAND_PRAY: Bytes = (include_bytes!("../ico/hand_pray.ico"), None, None);
/// `hand_raised`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const HAND_RAISED: Bytes = (include_bytes!("../ico/hand_raised.ico"), None, None);
/// `hand_rock`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const HAND_ROCK: Bytes = (include_bytes!("../ico/hand_rock.ico"), None, None);
/// `hand_thumbs_down`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const HAND_THUMBS_DOWN: Bytes = (include_bytes!("../ico/hand_thumbs_down.ico"), None, None);
/// `hand_thumbs_up`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const HAND_THUMBS_UP: Bytes = (include_bytes!("../ico/hand_thumbs_up.ico"), None, None);
/// `hand_vulcan`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const HAND_VULCAN: Bytes = (include_bytes!("../ico/hand_vulcan.ico"), None, None);
/// `hand_writing`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const HAND_WRITING: Bytes = (include_bytes!("../ico/hand_writing.ico"), None, None);
/// `hard_drive`: `.ico`, `.png` at 512 px.
pub const HARD_DRIVE: Bytes =
    (include_bytes!("../ico/hard_drive.ico"), Some(include_bytes!("../png/hard_drive.png")), None);
/// `headphones`: `.ico`, `.png` at 512 px.
pub const HEADPHONES: Bytes =
    (include_bytes!("../ico/headphones.ico"), Some(include_bytes!("../png/headphones.png")), None);
/// `hearing`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const HEARING: Bytes = (include_bytes!("../ico/hearing.ico"), None, None);
