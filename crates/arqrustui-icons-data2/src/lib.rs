//! Icon bytes from `contacts` through `emoji_wink` in the ArqRustUI bundle (part 2 of 8).
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

/// `contacts`: `.ico`, `.png` at 512 px.
pub const CONTACTS: Bytes = (include_bytes!("../ico/contacts.ico"), Some(include_bytes!("../png/contacts.png")), None);
/// `copy`: `.ico`, `.png` at 512 px, `.svg`.
pub const COPY: Bytes = (
    include_bytes!("../ico/copy.ico"),
    Some(include_bytes!("../png/copy.png")),
    Some(include_bytes!("../svg/copy.svg")),
);
/// `cpp`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const CPP: Bytes = (include_bytes!("../ico/cpp.ico"), None, None);
/// `cpp_file`: `.ico`, `.png` at 512 px.
pub const CPP_FILE: Bytes = (include_bytes!("../ico/cpp_file.ico"), Some(include_bytes!("../png/cpp_file.png")), None);
/// `cpu`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const CPU: Bytes = (include_bytes!("../ico/cpu.ico"), None, None);
/// `credit_card`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const CREDIT_CARD: Bytes = (include_bytes!("../ico/credit_card.ico"), None, None);
/// `cs_file`: `.ico`, `.png` at 512 px.
pub const CS_FILE: Bytes = (include_bytes!("../ico/cs_file.ico"), Some(include_bytes!("../png/cs_file.png")), None);
/// `csharp`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const CSHARP: Bytes = (include_bytes!("../ico/csharp.ico"), None, None);
/// `css3`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const CSS3: Bytes = (include_bytes!("../ico/css3.ico"), None, None);
/// `css_file`: `.ico`, `.png` at 512 px.
pub const CSS_FILE: Bytes = (include_bytes!("../ico/css_file.ico"), Some(include_bytes!("../png/css_file.png")), None);
/// `csv_file`: `.ico`, `.png` at 512 px.
pub const CSV_FILE: Bytes = (include_bytes!("../ico/csv_file.ico"), Some(include_bytes!("../png/csv_file.png")), None);
/// `cut`: `.ico`, `.png` at 512 px, `.svg`.
pub const CUT: Bytes =
    (include_bytes!("../ico/cut.ico"), Some(include_bytes!("../png/cut.png")), Some(include_bytes!("../svg/cut.svg")));
/// `database`: `.ico`, `.png` at 512 px.
pub const DATABASE: Bytes = (include_bytes!("../ico/database.ico"), Some(include_bytes!("../png/database.png")), None);
/// `database_search`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const DATABASE_SEARCH: Bytes = (include_bytes!("../ico/database_search.ico"), None, None);
/// `database_sync`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const DATABASE_SYNC: Bytes = (include_bytes!("../ico/database_sync.ico"), None, None);
/// `db_file`: `.ico`, `.png` at 512 px.
pub const DB_FILE: Bytes = (include_bytes!("../ico/db_file.ico"), Some(include_bytes!("../png/db_file.png")), None);
/// `delete`: `.ico`, `.png` at 512 px, `.svg`.
pub const DELETE: Bytes = (
    include_bytes!("../ico/delete.ico"),
    Some(include_bytes!("../png/delete.png")),
    Some(include_bytes!("../svg/delete.svg")),
);
/// `delivery_truck`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const DELIVERY_TRUCK: Bytes = (include_bytes!("../ico/delivery_truck.ico"), None, None);
/// `developer`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const DEVELOPER: Bytes = (include_bytes!("../ico/developer.ico"), None, None);
/// `devops`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const DEVOPS: Bytes = (include_bytes!("../ico/devops.ico"), None, None);
/// `disc_download`: `.ico`, `.png` at 512 px.
pub const DISC_DOWNLOAD: Bytes =
    (include_bytes!("../ico/disc_download.ico"), Some(include_bytes!("../png/disc_download.png")), None);
/// `dll_file`: `.ico`, `.png` at 512 px.
pub const DLL_FILE: Bytes = (include_bytes!("../ico/dll_file.ico"), Some(include_bytes!("../png/dll_file.png")), None);
/// `doc_file`: `.ico`, `.png` at 512 px.
pub const DOC_FILE: Bytes = (include_bytes!("../ico/doc_file.ico"), Some(include_bytes!("../png/doc_file.png")), None);
/// `docker`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const DOCKER: Bytes = (include_bytes!("../ico/docker.ico"), None, None);
/// `docx_file`: `.ico`, `.png` at 512 px.
pub const DOCX_FILE: Bytes =
    (include_bytes!("../ico/docx_file.ico"), Some(include_bytes!("../png/docx_file.png")), None);
/// `dotnet`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const DOTNET: Bytes = (include_bytes!("../ico/dotnet.ico"), None, None);
/// `down`: `.ico`, `.png` at 512 px, `.svg`.
pub const DOWN: Bytes = (
    include_bytes!("../ico/down.ico"),
    Some(include_bytes!("../png/down.png")),
    Some(include_bytes!("../svg/down.svg")),
);
/// `download`: `.ico`, `.png` at 512 px, `.svg`.
pub const DOWNLOAD: Bytes = (
    include_bytes!("../ico/download.ico"),
    Some(include_bytes!("../png/download.png")),
    Some(include_bytes!("../svg/download.svg")),
);
/// `download_window`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const DOWNLOAD_WINDOW: Bytes = (include_bytes!("../ico/download_window.ico"), None, None);
/// `dual_monitors`: `.ico`, `.png` at 512 px.
pub const DUAL_MONITORS: Bytes =
    (include_bytes!("../ico/dual_monitors.ico"), Some(include_bytes!("../png/dual_monitors.png")), None);
/// `edit`: `.ico`, `.png` at 512 px, `.svg`.
pub const EDIT: Bytes = (
    include_bytes!("../ico/edit.ico"),
    Some(include_bytes!("../png/edit.png")),
    Some(include_bytes!("../svg/edit.svg")),
);
/// `edit_note`: `.ico`, `.png` at 512 px.
pub const EDIT_NOTE: Bytes =
    (include_bytes!("../ico/edit_note.ico"), Some(include_bytes!("../png/edit_note.png")), None);
/// `elasticsearch`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const ELASTICSEARCH: Bytes = (include_bytes!("../ico/elasticsearch.ico"), None, None);
/// `email`: `.ico`, `.png` at 512 px.
pub const EMAIL: Bytes = (include_bytes!("../ico/email.ico"), Some(include_bytes!("../png/email.png")), None);
/// `email_receive`: `.ico`, `.png` at 512 px.
pub const EMAIL_RECEIVE: Bytes =
    (include_bytes!("../ico/email_receive.ico"), Some(include_bytes!("../png/email_receive.png")), None);
/// `email_send`: `.ico`, `.png` at 512 px.
pub const EMAIL_SEND: Bytes =
    (include_bytes!("../ico/email_send.ico"), Some(include_bytes!("../png/email_send.png")), None);
/// `emoji_grin`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const EMOJI_GRIN: Bytes = (include_bytes!("../ico/emoji_grin.ico"), None, None);
/// `emoji_hand_over_mouth`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const EMOJI_HAND_OVER_MOUTH: Bytes = (include_bytes!("../ico/emoji_hand_over_mouth.ico"), None, None);
/// `emoji_hug`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const EMOJI_HUG: Bytes = (include_bytes!("../ico/emoji_hug.ico"), None, None);
/// `emoji_joy`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const EMOJI_JOY: Bytes = (include_bytes!("../ico/emoji_joy.ico"), None, None);
/// `emoji_kiss`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const EMOJI_KISS: Bytes = (include_bytes!("../ico/emoji_kiss.ico"), None, None);
/// `emoji_mind_blown`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const EMOJI_MIND_BLOWN: Bytes = (include_bytes!("../ico/emoji_mind_blown.ico"), None, None);
/// `emoji_nauseated`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const EMOJI_NAUSEATED: Bytes = (include_bytes!("../ico/emoji_nauseated.ico"), None, None);
/// `emoji_nerd`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const EMOJI_NERD: Bytes = (include_bytes!("../ico/emoji_nerd.ico"), None, None);
/// `emoji_nervous`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const EMOJI_NERVOUS: Bytes = (include_bytes!("../ico/emoji_nervous.ico"), None, None);
/// `emoji_party`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const EMOJI_PARTY: Bytes = (include_bytes!("../ico/emoji_party.ico"), None, None);
/// `emoji_poop`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const EMOJI_POOP: Bytes = (include_bytes!("../ico/emoji_poop.ico"), None, None);
/// `emoji_relieved`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const EMOJI_RELIEVED: Bytes = (include_bytes!("../ico/emoji_relieved.ico"), None, None);
/// `emoji_scream`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const EMOJI_SCREAM: Bytes = (include_bytes!("../ico/emoji_scream.ico"), None, None);
/// `emoji_smile`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const EMOJI_SMILE: Bytes = (include_bytes!("../ico/emoji_smile.ico"), None, None);
/// `emoji_star_struck`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const EMOJI_STAR_STRUCK: Bytes = (include_bytes!("../ico/emoji_star_struck.ico"), None, None);
/// `emoji_surprised`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const EMOJI_SURPRISED: Bytes = (include_bytes!("../ico/emoji_surprised.ico"), None, None);
/// `emoji_suspicious`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const EMOJI_SUSPICIOUS: Bytes = (include_bytes!("../ico/emoji_suspicious.ico"), None, None);
/// `emoji_thinking`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const EMOJI_THINKING: Bytes = (include_bytes!("../ico/emoji_thinking.ico"), None, None);
/// `emoji_tongue`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const EMOJI_TONGUE: Bytes = (include_bytes!("../ico/emoji_tongue.ico"), None, None);
/// `emoji_wink`: `.ico`, `.png` = the 256 px frame from the `.ico`.
pub const EMOJI_WINK: Bytes = (include_bytes!("../ico/emoji_wink.ico"), None, None);
