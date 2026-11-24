//! High-level events delivered to registered control handlers.

use crate::input::{KeyEvent, MouseEvent, PointerEvent};
use crate::utilities::{Point, Size};
use std::path::PathBuf;

/// UI event translated from Win32 messages.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Event {
    /// Click (button, label, panel).
    Click,
    /// Double click.
    DoubleClick,
    /// Cursor moved over the control.
    MouseMove(MouseEvent),
    /// Cursor entered the control.
    MouseEnter,
    /// Cursor left the control.
    MouseLeave,
    /// Mouse button pressed.
    MouseDown(MouseEvent),
    /// Mouse button released.
    MouseUp(MouseEvent),
    /// Mouse wheel rotated.
    MouseWheel(MouseEvent),
    /// Key pressed.
    KeyDown(KeyEvent),
    /// Key released.
    KeyUp(KeyEvent),
    /// Character typed, composed by keyboard layout/IME.
    Char(char),
    /// Control text changed.
    TextChanged,
    /// Control gained keyboard focus.
    Focus,
    /// Control lost keyboard focus.
    Blur,
    /// Client area resized (new logical size).
    Resize(Size),
    /// Window moved (screen pixels).
    Move(Point),
    /// Control selection changed.
    SelectionChanged,
    /// Control value changed (slider, spin, date, check box, etc.).
    ValueChanged,
    /// Control destroyed; no subsequent events are delivered.
    Destroyed,
    /// Window DPI changed (new value).
    DpiChanged(u32),
    /// Files dropped onto the window.
    DropFiles(Vec<PathBuf>),
    /// System theme changed (light/dark/high contrast).
    ThemeChanged,
    /// Context menu requested at the given control-relative logical position.
    ContextMenu(Point),
    /// Pointer event (touch, pen).
    Pointer(PointerEvent),
    /// IME composition text, not yet committed.
    ImeComposition(String),
    /// Window activated.
    Activated,
    /// Window deactivated.
    Deactivated,
    /// Item activated by double click/Enter in lists/trees; carries index.
    ItemActivated(usize),
    /// Column header clicked; carries column index.
    ColumnClick(usize),
    /// Link clicked; carries identifier or URL.
    LinkClicked(String),
}

/// [`Event`] discriminant for handler registration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
#[non_exhaustive]
pub enum EventKind {
    Click,
    DoubleClick,
    MouseMove,
    MouseEnter,
    MouseLeave,
    MouseDown,
    MouseUp,
    MouseWheel,
    KeyDown,
    KeyUp,
    Char,
    TextChanged,
    Focus,
    Blur,
    Resize,
    Move,
    SelectionChanged,
    ValueChanged,
    Destroyed,
    DpiChanged,
    DropFiles,
    ThemeChanged,
    ContextMenu,
    Pointer,
    ImeComposition,
    Activated,
    Deactivated,
    ItemActivated,
    ColumnClick,
    LinkClicked,
    /// Wildcard: handler receives all events.
    Any,
}

impl Event {
    /// Event discriminant.
    pub fn kind(&self) -> EventKind {
        match self {
            Event::Click => EventKind::Click,
            Event::DoubleClick => EventKind::DoubleClick,
            Event::MouseMove(_) => EventKind::MouseMove,
            Event::MouseEnter => EventKind::MouseEnter,
            Event::MouseLeave => EventKind::MouseLeave,
            Event::MouseDown(_) => EventKind::MouseDown,
            Event::MouseUp(_) => EventKind::MouseUp,
            Event::MouseWheel(_) => EventKind::MouseWheel,
            Event::KeyDown(_) => EventKind::KeyDown,
            Event::KeyUp(_) => EventKind::KeyUp,
            Event::Char(_) => EventKind::Char,
            Event::TextChanged => EventKind::TextChanged,
            Event::Focus => EventKind::Focus,
            Event::Blur => EventKind::Blur,
            Event::Resize(_) => EventKind::Resize,
            Event::Move(_) => EventKind::Move,
            Event::SelectionChanged => EventKind::SelectionChanged,
            Event::ValueChanged => EventKind::ValueChanged,
            Event::Destroyed => EventKind::Destroyed,
            Event::DpiChanged(_) => EventKind::DpiChanged,
            Event::DropFiles(_) => EventKind::DropFiles,
            Event::ThemeChanged => EventKind::ThemeChanged,
            Event::ContextMenu(_) => EventKind::ContextMenu,
            Event::Pointer(_) => EventKind::Pointer,
            Event::ImeComposition(_) => EventKind::ImeComposition,
            Event::Activated => EventKind::Activated,
            Event::Deactivated => EventKind::Deactivated,
            Event::ItemActivated(_) => EventKind::ItemActivated,
            Event::ColumnClick(_) => EventKind::ColumnClick,
            Event::LinkClicked(_) => EventKind::LinkClicked,
        }
    }
}

/// Registered handler identifier for later removal via [`Widget::remove_handler`](crate::core::Widget::remove_handler).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HandlerId(pub(crate) u64);
