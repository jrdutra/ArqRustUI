//! Mouse/pointer events (touch/pen).

use super::keyboard::Modifiers;
use crate::utilities::Point;

/// Mouse button.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MouseButton {
    /// No button (movement/wheel).
    None,
    /// Left button.
    Left,
    /// Right button.
    Right,
    /// Middle button.
    Middle,
}

/// Mouse event in logical (96 DPI) client coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct MouseEvent {
    /// Cursor position.
    pub position: Point,
    /// Originating button.
    pub button: MouseButton,
    /// Pressed modifiers.
    pub modifiers: Modifiers,
    /// Wheel ticks, positive upward; 0 outside `on_mouse_wheel`.
    pub wheel_delta: i32,
}

/// Pointer device type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PointerKind {
    /// Mouse.
    Mouse,
    /// Touch.
    Touch,
    /// Pen.
    Pen,
    /// Other/unspecified device.
    Other,
}

/// Pointer contact phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PointerPhase {
    /// Contact began.
    Down,
    /// Contact moved.
    Move,
    /// Contact ended.
    Up,
}

/// Pointer (touch/pen/mouse) event for `on_pointer`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct PointerEvent {
    /// Contact identifier for tracking multiple fingers.
    pub id: u32,
    /// Device.
    pub kind: PointerKind,
    /// Phase.
    pub phase: PointerPhase,
    /// Logical client-relative position.
    pub position: Point,
}
