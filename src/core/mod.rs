//! Library foundation: application, event loop, windows, events, handles and errors.

pub mod application;
pub mod dispatcher;
pub mod errors;
pub mod event_loop;
pub mod events;
pub(crate) mod ffi_guard;
pub mod handles;
pub mod messages;
pub mod motion;
pub(crate) mod native;
pub mod timer;
pub mod widget;
pub mod window;

pub use application::{defer, Application};
pub use dispatcher::{run_blocking, sleep, spawn_background, spawn_local, Completion, UiDispatcher};
pub use errors::{GuiError, GuiResult};
pub use events::{Event, EventKind, HandlerId};
pub use handles::{GdiObject, GlobalMemory, Icon, IconSource, ImageList, MemoryDc, SystemIcon, WindowDc};
pub use messages::RawMessage;
pub use motion::{Easing, ShowEffect, Transition};
pub use timer::Timer;
pub use widget::{Widget, WidgetCore};
pub use window::{Window, WindowBuilder};
