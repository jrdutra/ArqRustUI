//! Input: keyboard, mouse, touch/pen, IME, drag/drop.

mod data_object;
pub mod drag;
pub mod gestures;
pub mod ime;
pub mod keyboard;
pub mod mouse;

pub use drag::{
    accept_drops, start_drag, start_drag_with_image, stop_accepting_drops, DropData, DropEffect, DropHandler,
};
pub use gestures::{GestureEvent, GestureKind, GestureOptions, GesturePhase, GestureRecognizer, GestureSubscription};
pub use ime::{Ime, ImeCandidateWindow, ImeCandidates};
pub use keyboard::{Key, KeyEvent, Modifiers, Shortcut};
pub use mouse::{MouseButton, MouseEvent, PointerEvent, PointerKind, PointerPhase};
