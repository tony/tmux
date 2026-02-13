//! Key and modifier definitions for terminal input.
//!
//! Maps terminal key codes and modifier combinations for input routing,
//! key binding dispatch, and copy mode navigation.

use bitflags::bitflags;

bitflags! {
    /// Modifier keys that can be combined with a key press.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Modifiers: u8 {
        /// Shift modifier.
        const SHIFT = 0x01;
        /// Alt/Meta modifier.
        const ALT   = 0x02;
        /// Control modifier.
        const CTRL  = 0x04;
        /// Super/Windows key modifier.
        const SUPER = 0x08;
    }
}

/// A terminal key event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    /// Regular character key.
    Char(char),
    /// Function keys F1-F12.
    F(u8),
    /// Enter/Return.
    Enter,
    /// Tab.
    Tab,
    /// Backspace.
    Backspace,
    /// Escape.
    Escape,
    /// Arrow up.
    Up,
    /// Arrow down.
    Down,
    /// Arrow left.
    Left,
    /// Arrow right.
    Right,
    /// Home key.
    Home,
    /// End key.
    End,
    /// Page Up.
    PageUp,
    /// Page Down.
    PageDown,
    /// Insert key.
    Insert,
    /// Delete key.
    Delete,
}

impl Key {
    /// Returns true if this is a printable character key.
    pub const fn is_char(&self) -> bool {
        matches!(self, Self::Char(_))
    }

    /// Returns true if this is a function key.
    pub const fn is_function(&self) -> bool {
        matches!(self, Self::F(_))
    }

    /// Returns true if this is a navigation key (arrows, home, end, page up/down).
    pub const fn is_navigation(&self) -> bool {
        matches!(
            self,
            Self::Up
                | Self::Down
                | Self::Left
                | Self::Right
                | Self::Home
                | Self::End
                | Self::PageUp
                | Self::PageDown
        )
    }

    /// Convert a char key to its byte representation (ASCII only).
    pub fn as_byte(&self) -> Option<u8> {
        match self {
            Self::Char(c) if c.is_ascii() => Some(*c as u8),
            Self::Enter => Some(b'\r'),
            Self::Tab => Some(b'\t'),
            Self::Backspace => Some(0x7F),
            Self::Escape => Some(0x1B),
            _ => None,
        }
    }

    /// Create from a byte (ASCII mapping).
    pub fn from_byte(b: u8) -> Self {
        match b {
            b'\r' | b'\n' => Self::Enter,
            b'\t' => Self::Tab,
            0x7F => Self::Backspace,
            0x1B => Self::Escape,
            c if c.is_ascii_graphic() || c == b' ' => Self::Char(c as char),
            c => Self::Char(c as char),
        }
    }
}

/// A key press event with modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyPress {
    /// The key that was pressed.
    pub key: Key,
    /// Active modifiers.
    pub modifiers: Modifiers,
}

impl KeyPress {
    /// Create a key press with no modifiers.
    pub const fn new(key: Key) -> Self {
        Self {
            key,
            modifiers: Modifiers::empty(),
        }
    }

    /// Create a key press with modifiers.
    pub const fn with_modifiers(key: Key, modifiers: Modifiers) -> Self {
        Self { key, modifiers }
    }

    /// Returns true if this key press has the control modifier.
    pub fn is_ctrl(&self) -> bool {
        self.modifiers.contains(Modifiers::CTRL)
    }

    /// Returns true if this key press has the alt modifier.
    pub fn is_alt(&self) -> bool {
        self.modifiers.contains(Modifiers::ALT)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_is_char() {
        assert!(Key::Char('a').is_char());
        assert!(!Key::Enter.is_char());
    }

    #[test]
    fn key_is_function() {
        assert!(Key::F(1).is_function());
        assert!(!Key::Enter.is_function());
    }

    #[test]
    fn key_is_navigation() {
        assert!(Key::Up.is_navigation());
        assert!(Key::PageDown.is_navigation());
        assert!(!Key::Enter.is_navigation());
    }

    #[test]
    fn key_as_byte_ascii() {
        assert_eq!(Key::Char('a').as_byte(), Some(b'a'));
        assert_eq!(Key::Enter.as_byte(), Some(b'\r'));
        assert_eq!(Key::Tab.as_byte(), Some(b'\t'));
        assert_eq!(Key::Backspace.as_byte(), Some(0x7F));
        assert_eq!(Key::Escape.as_byte(), Some(0x1B));
    }

    #[test]
    fn key_from_byte() {
        assert_eq!(Key::from_byte(b'a'), Key::Char('a'));
        assert_eq!(Key::from_byte(b'\r'), Key::Enter);
        assert_eq!(Key::from_byte(b'\n'), Key::Enter);
    }

    #[test]
    fn modifiers_ctrl() {
        let m = Modifiers::CTRL;
        assert!(m.contains(Modifiers::CTRL));
        assert!(!m.contains(Modifiers::ALT));
    }

    #[test]
    fn modifiers_combined() {
        let m = Modifiers::CTRL | Modifiers::ALT;
        assert!(m.contains(Modifiers::CTRL));
        assert!(m.contains(Modifiers::ALT));
    }

    #[test]
    fn key_press_ctrl_check() {
        let kp = KeyPress::with_modifiers(Key::Char('c'), Modifiers::CTRL);
        assert!(kp.is_ctrl());
        assert!(!kp.is_alt());
    }

    #[test]
    fn key_press_no_modifiers() {
        let kp = KeyPress::new(Key::Char('x'));
        assert!(!kp.is_ctrl());
        assert!(!kp.is_alt());
    }

    #[test]
    fn function_keys_range() {
        for i in 1..=12 {
            assert!(Key::F(i).is_function());
        }
    }

    #[test]
    fn key_as_byte_non_ascii_none() {
        assert!(Key::Up.as_byte().is_none());
        assert!(Key::F(1).as_byte().is_none());
    }

    mod prop {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn ascii_byte_roundtrip(b in 0x20u8..0x7F) {
                let key = Key::from_byte(b);
                if b == 0x7F {
                    prop_assert_eq!(key, Key::Backspace);
                } else {
                    prop_assert_eq!(key.as_byte(), Some(b));
                }
            }

            #[test]
            fn modifier_bits_roundtrip(bits in 0u8..16) {
                let m = Modifiers::from_bits_truncate(bits);
                let m2 = Modifiers::from_bits_truncate(m.bits());
                prop_assert_eq!(m, m2);
            }
        }
    }
}
