//! Key codes, modifiers, and input events.

use bitflags::bitflags;

/// Key codes representing keyboard keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    /// A regular character key.
    Char(char),
    /// Function key F1-F12.
    F(u8),
    /// Backspace.
    Backspace,
    /// Enter / Return.
    Enter,
    /// Tab.
    Tab,
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
    /// Insert.
    Insert,
    /// Delete.
    Delete,
}

bitflags! {
    /// Key modifiers.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct KeyModifiers: u8 {
        const SHIFT   = 0x01;
        const ALT     = 0x02;
        const CTRL    = 0x04;
        const META    = 0x08;
    }
}

/// A keyboard event with key code and modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyEvent {
    /// The key that was pressed.
    pub code: KeyCode,
    /// Active modifiers.
    pub modifiers: KeyModifiers,
}

impl KeyEvent {
    /// Create a new key event.
    #[must_use]
    pub const fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { code, modifiers }
    }

    /// Create a key event with no modifiers.
    #[must_use]
    pub const fn plain(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::empty(),
        }
    }
}

/// Mouse button identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    /// Left mouse button.
    Left,
    /// Middle mouse button.
    Middle,
    /// Right mouse button.
    Right,
    /// Scroll wheel up.
    WheelUp,
    /// Scroll wheel down.
    WheelDown,
}

/// Mouse event kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseEventKind {
    /// Button pressed.
    Press(MouseButton),
    /// Button released.
    Release,
    /// Mouse moved while button held.
    Drag(MouseButton),
    /// Mouse moved without button.
    Move,
}

/// A mouse event with position and modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MouseEvent {
    /// The kind of mouse event.
    pub kind: MouseEventKind,
    /// Column position (0-based).
    pub col: u16,
    /// Row position (0-based).
    pub row: u16,
    /// Active modifiers.
    pub modifiers: KeyModifiers,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_event_plain() {
        let e = KeyEvent::plain(KeyCode::Char('a'));
        assert_eq!(e.code, KeyCode::Char('a'));
        assert!(e.modifiers.is_empty());
    }

    #[test]
    fn key_event_with_modifiers() {
        let e = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CTRL);
        assert!(e.modifiers.contains(KeyModifiers::CTRL));
    }

    #[test]
    fn function_keys() {
        for i in 1..=12 {
            let e = KeyEvent::plain(KeyCode::F(i));
            assert_eq!(e.code, KeyCode::F(i));
        }
    }

    #[test]
    fn mouse_event_press() {
        let e = MouseEvent {
            kind: MouseEventKind::Press(MouseButton::Left),
            col: 10,
            row: 5,
            modifiers: KeyModifiers::empty(),
        };
        assert_eq!(e.col, 10);
        assert_eq!(e.row, 5);
    }

    #[test]
    fn modifier_combinations() {
        let mods = KeyModifiers::CTRL | KeyModifiers::ALT | KeyModifiers::SHIFT;
        assert!(mods.contains(KeyModifiers::CTRL));
        assert!(mods.contains(KeyModifiers::ALT));
        assert!(mods.contains(KeyModifiers::SHIFT));
        assert!(!mods.contains(KeyModifiers::META));
    }
}
