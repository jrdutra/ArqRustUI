//! Keys, modifiers and shortcuts.

use crate::core::native::key_down;
use crate::ffi::user32::{VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT};

/// Layout-independent Windows virtual key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Key(pub u16);

impl Key {
    /// Backspace.
    pub const BACKSPACE: Key = Key(0x08);
    /// Tab.
    pub const TAB: Key = Key(0x09);
    /// Enter.
    pub const ENTER: Key = Key(0x0D);
    /// Escape.
    pub const ESCAPE: Key = Key(0x1B);
    /// Space bar.
    pub const SPACE: Key = Key(0x20);
    /// Page Up.
    pub const PAGE_UP: Key = Key(0x21);
    /// Page Down.
    pub const PAGE_DOWN: Key = Key(0x22);
    /// End.
    pub const END: Key = Key(0x23);
    /// Home.
    pub const HOME: Key = Key(0x24);
    /// Left arrow.
    pub const LEFT: Key = Key(0x25);
    /// Up arrow.
    pub const UP: Key = Key(0x26);
    /// Right arrow.
    pub const RIGHT: Key = Key(0x27);
    /// Down arrow.
    pub const DOWN: Key = Key(0x28);
    /// Insert.
    pub const INSERT: Key = Key(0x2D);
    /// Delete.
    pub const DELETE: Key = Key(0x2E);
    /// F1.
    pub const F1: Key = Key(0x70);
    /// F2.
    pub const F2: Key = Key(0x71);
    /// F3.
    pub const F3: Key = Key(0x72);
    /// F4.
    pub const F4: Key = Key(0x73);
    /// F5.
    pub const F5: Key = Key(0x74);
    /// F11.
    pub const F11: Key = Key(0x7A);
    /// F12.
    pub const F12: Key = Key(0x7B);

    /// Key for case-insensitive `'A'..='Z'` or a digit.
    ///
    /// Other characters return `None` because they depend on layout.
    pub fn from_char(c: char) -> Option<Key> {
        let upper = c.to_ascii_uppercase();
        if upper.is_ascii_uppercase() || upper.is_ascii_digit() {
            Some(Key(upper as u16))
        } else {
            None
        }
    }

    /// Function key `F1..=F24`.
    pub fn function(n: u8) -> Option<Key> {
        (1..=24).contains(&n).then(|| Key(0x70 + n as u16 - 1))
    }

    /// Readable shortcut key name, e.g. `"F5"`, `"S"`, `"Del"`.
    pub fn name(&self) -> String {
        match self.0 {
            0x08 => "Backspace".into(),
            0x09 => "Tab".into(),
            0x0D => "Enter".into(),
            0x1B => "Esc".into(),
            0x20 => "Space".into(),
            0x21 => "PgUp".into(),
            0x22 => "PgDn".into(),
            0x23 => "End".into(),
            0x24 => "Home".into(),
            0x25 => "Left".into(),
            0x26 => "Up".into(),
            0x27 => "Right".into(),
            0x28 => "Down".into(),
            0x2D => "Ins".into(),
            0x2E => "Del".into(),
            code @ 0x70..=0x87 => format!("F{}", code - 0x70 + 1),
            code @ (0x30..=0x39 | 0x41..=0x5A) => (code as u8 as char).to_string(),
            code => format!("VK{code:#04X}"),
        }
    }
}

/// Pressed modifiers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Modifiers {
    /// Ctrl.
    pub ctrl: bool,
    /// Shift.
    pub shift: bool,
    /// Alt.
    pub alt: bool,
    /// Windows key.
    pub win: bool,
}

impl Modifiers {
    /// No modifiers.
    pub const NONE: Modifiers = Modifiers { ctrl: false, shift: false, alt: false, win: false };

    /// Reads current keyboard state.
    pub fn current() -> Modifiers {
        Modifiers {
            ctrl: key_down(VK_CONTROL),
            shift: key_down(VK_SHIFT),
            alt: key_down(VK_MENU),
            win: key_down(VK_LWIN) || key_down(VK_RWIN),
        }
    }
}

/// Shortcut: key plus modifiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Shortcut {
    /// Main key.
    pub key: Key,
    /// Required modifiers.
    pub modifiers: Modifiers,
}

impl Shortcut {
    /// Unmodified shortcut, e.g. F5.
    pub const fn key(key: Key) -> Self {
        Self { key, modifiers: Modifiers::NONE }
    }

    /// Ctrl plus key.
    pub const fn ctrl(key: Key) -> Self {
        Self { key, modifiers: Modifiers { ctrl: true, shift: false, alt: false, win: false } }
    }

    /// Ctrl+Shift plus key.
    pub const fn ctrl_shift(key: Key) -> Self {
        Self { key, modifiers: Modifiers { ctrl: true, shift: true, alt: false, win: false } }
    }

    /// Alt plus key.
    pub const fn alt(key: Key) -> Self {
        Self { key, modifiers: Modifiers { ctrl: false, shift: false, alt: true, win: false } }
    }

    /// Ctrl plus letter/digit; `None` for other characters.
    pub fn ctrl_char(c: char) -> Option<Self> {
        Key::from_char(c).map(Self::ctrl)
    }

    /// Menu shortcut text, e.g. `"Ctrl+Shift+S"`.
    pub fn label(&self) -> String {
        let mut parts = Vec::new();
        if self.modifiers.ctrl {
            parts.push("Ctrl".to_string());
        }
        if self.modifiers.alt {
            parts.push("Alt".to_string());
        }
        if self.modifiers.shift {
            parts.push("Shift".to_string());
        }
        if self.modifiers.win {
            parts.push("Win".to_string());
        }
        parts.push(self.key.name());
        parts.join("+")
    }
}

/// Keyboard event for `on_key_down` / `on_key_up`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct KeyEvent {
    /// Event key.
    pub key: Key,
    /// Event-time modifiers.
    pub modifiers: Modifiers,
    /// `true` for held-key autorepeat.
    pub repeat: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_from_chars() {
        assert_eq!(Key::from_char('s'), Some(Key(0x53)));
        assert_eq!(Key::from_char('7'), Some(Key(0x37)));
        assert_eq!(Key::from_char('ç'), None);
        assert_eq!(Key::function(5), Some(Key::F5));
        assert_eq!(Key::function(0), None);
    }

    #[test]
    fn shortcut_labels() {
        assert_eq!(Shortcut::ctrl_char('s').unwrap().label(), "Ctrl+S");
        assert_eq!(Shortcut::ctrl_shift(Key::from_char('z').unwrap()).label(), "Ctrl+Shift+Z");
        assert_eq!(Shortcut::alt(Key::F4).label(), "Alt+F4");
        assert_eq!(Shortcut::key(Key::DELETE).label(), "Del");
    }
}
