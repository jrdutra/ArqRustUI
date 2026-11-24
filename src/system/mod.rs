//! Windows integration: tray, clipboard, shell, monitors, cursors, printing, version and accessibility settings.

pub mod accessibility;
pub mod clipboard;
pub mod cursor;
pub mod monitor;
pub mod notifications;
pub mod printing;
pub mod shell;
pub mod tray;
pub mod version;

pub use clipboard::{Clipboard, ClipboardContent, ClipboardWatcher, Metafile};
pub use cursor::{Cursor, SystemCursor};
pub use monitor::Monitor;
pub use notifications::{
    DismissReason, NotificationActivation, NotificationActivator, NotificationScenario, ScheduledNotification,
    SystemNotification, SystemNotificationHandle,
};
pub use printing::{PrintDocument, PrintOptions, PrintRange, PrinterContext, A4, LETTER};
pub use shell::KnownFolder;
pub use tray::{NotificationKind, TrayIcon};
pub use version::WindowsVersion;
