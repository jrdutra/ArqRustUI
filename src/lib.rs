//! # ArqRustUI
//!
//! **Native** Microsoft Windows GUI library written entirely in Rust, using only std and direct FFI to system DLLs
//! (Win32, Common Controls, GDI/GDI+, Direct2D/DirectWrite, COM). No external crates.
//!
//! ```no_run
//! use arqrustui::prelude::*;
//!
//! fn main() -> GuiResult<()> {
//!     let app = Application::new()?;
//!
//!     let window = Window::builder().title("My Application").size(800, 600).centered().build()?;
//!
//!     let button = Button::builder().text("Click here").position(20, 20).size(150, 40).parent(&window).build()?;
//!     button.on_click(|| MessageBox::info("Message", "Hello, Windows!"));
//!
//!     window.show();
//!     app.run()
//! }
//! ```
//!
//! ## Organization
//!
//! | Module | Contents |
//! |---|---|
//! | [`core`] | [`Application`](core::Application), [`Window`](core::Window), [`Widget`](core::Widget), events, errors, RAII handles, dispatcher and `async` |
//! | [`controls`] | Native/composed controls |
//! | [`layout`] | Layout managers |
//! | [`menus`] | Native [`MenuBar`](menus::MenuBar) or drawn [`MenuStrip`](menus::MenuStrip), context/system menus |
//! | [`dialogs`] | Native/custom dialogs |
//! | [`graphics`] | GDI, GDI+ and Direct2D/DirectWrite |
//! | [`themes`] | Themes, styles and DWM effects |
//! | [`system`] | Tray, clipboard, shell, monitors, printing |
//! | [`input`] | Keyboard, mouse, touch and IME |
//! | [`accessibility`] | UI Automation and keyboard navigation |
//! | [`i18n`] | Locale, regional formatting and translations |
//! | [`icons`] | 392 embedded icons (`.ico`, `.png`, `.svg`) |
//! | [`utilities`] | Geometry, colors and strings |
//! | [`ffi`] | Raw Win32 declarations (advanced API) |
//!
//! ## Fundamental rules
//!
//! * UI lives on the thread creating [`Application`](core::Application); visual types are not `Send`/`Sync`. Other
//!   threads use [`UiDispatcher`](core::UiDispatcher).
//! * Logical coordinates/sizes (96 DPI) scale automatically.
//! * Controls are cloneable *handles*; native windows live until explicitly destroyed or parent destruction.
//!
//! The compiler enforces thread affinity; this fails to compile:
//!
//! ```compile_fail
//! fn exige_send<T: Send>() {}
//! exige_send::<arqrustui::core::Window>();
//! ```
//!
//! ```compile_fail
//! fn exige_send<T: Send>() {}
//! exige_send::<arqrustui::controls::Button>();
//! ```
//!
//! ## API evolution
//!
//! * Public enums are `#[non_exhaustive]`, except inherently closed sets ([`Orientation`](layout::Orientation),
//!   [`CheckState`](controls::CheckState), [`TreeDropPosition`](controls::TreeDropPosition)). Versions may add variants
//!   without breaking consumers; external `match` needs `_`. This fails to compile:
//!
//! ```compile_fail
//! use arqrustui::themes::SystemTheme;
//! fn nome(tema: SystemTheme) -> &'static str {
//!     match tema {
//!         SystemTheme::Light => "light",
//!         SystemTheme::Dark => "dark",
//!         SystemTheme::HighContrast => "high contrast",
//!     }
//! }
//! ```
//!
//! ```
//! use arqrustui::themes::SystemTheme;
//! fn nome(tema: SystemTheme) -> &'static str {
//!     match tema {
//!         SystemTheme::Light => "light",
//!         SystemTheme::Dark => "dark",
//!         SystemTheme::HighContrast => "high contrast",
//!         _ => "other",
//!     }
//! }
//! # assert_eq!(nome(SystemTheme::Dark), "dark");
//! ```
//!
//! * Output structs (`KeyEvent`, `MouseEvent`, `PointerEvent`, `GestureEvent`, `FindEvent`, `NotificationActivation`,
//!   `FontChoice`, `TaskDialogResult`, `DetachedTab`, `ImeCandidates`, `RawMessage`, `Monitor`, `WindowsVersion`) are
//!   also `#[non_exhaustive]`: fields are readable, but external literals are disallowed. `ImeCandidates` also serves
//!   as input and offers `ImeCandidates::new`.
//!
//! * Event names: `on_click` for user actions; `on_<x>_changed`/`on_toggled`/`on_closed` for completed changes;
//!   `on_<x>_request` for vetoable requests. See `docs/EVENTS.en.md`. Previous names remain as `#[deprecated]` aliases.
//!
//! Only `cfg(windows)` contains library code; the crate compiles empty elsewhere.

#![cfg(windows)]
#![warn(missing_docs)]
// O lint abaixo dispara mesmo com inicializadores `const { .. }` em alguns alvos Windows.
#![allow(clippy::missing_const_for_thread_local)]

pub mod accessibility;
pub mod controls;
pub mod core;
pub mod dialogs;
pub mod ffi;
pub mod graphics;
pub mod i18n;
pub mod icons;
pub mod input;
pub mod layout;
pub mod menus;
pub mod system;
pub mod themes;
pub mod utilities;

/// Single import for typical application needs.
pub mod prelude {
    pub use crate::accessibility::{self, AccessibleRole};
    pub use crate::controls::*;
    pub use crate::core::motion;
    pub use crate::core::{
        defer, run_blocking, sleep, spawn_background, spawn_local, Application, Easing, Event, EventKind, GuiError,
        GuiResult, HandlerId, Icon, IconSource, ImageList, ShowEffect, SystemIcon, Timer, Transition, UiDispatcher,
        Widget, Window, WindowBuilder,
    };
    pub use crate::dialogs::*;
    pub use crate::graphics::{
        Bitmap, Canvas, D2DContext, Direct2DSurface, Font, GradientDirection, Path, Pen, PenStyle, PlusGraphics,
        PlusImage, SvgImage, TextAlign,
    };
    pub use crate::i18n::{DateTime, Locale, Translations};
    pub use crate::icons::{self, IconAsset, IconCategory};
    pub use crate::input::gestures;
    pub use crate::input::{
        accept_drops, start_drag, start_drag_with_image, stop_accepting_drops, DropData, DropEffect, DropHandler,
        GestureEvent, GestureKind, GestureOptions, GesturePhase, Ime, ImeCandidateWindow, ImeCandidates, Key, KeyEvent,
        Modifiers, MouseButton, MouseEvent, PointerEvent, PointerKind, PointerPhase, Shortcut,
    };
    pub use crate::layout::{
        AbsoluteLayout, Align, AnchorLayout, Anchors, Dock, DockLayout, FlowLayout, GridLayout, Layout, LayoutParams,
        Orientation, Spacer, SplitLayout, StackLayout, StatusBarLayout, Track, MARGIN, SPACING,
    };
    pub use crate::menus::{ContextMenu, Menu, MenuBar, MenuItem, MenuStrip, MenuStyle};
    pub use crate::system::{
        Clipboard, ClipboardContent, ClipboardWatcher, Cursor, DismissReason, KnownFolder, Monitor,
        NotificationActivation, NotificationActivator, NotificationKind, NotificationScenario, PrintDocument,
        PrintOptions, PrintRange, PrinterContext, ScheduledNotification, SystemCursor, SystemNotification,
        SystemNotificationHandle, TrayIcon, WindowsVersion,
    };
    pub use crate::themes::{Backdrop, ButtonStyle, CornerStyle, StateColors, Style, SystemTheme, Theme};
    pub use crate::utilities::{Color, Point, Rect, Size, Thickness};
}
