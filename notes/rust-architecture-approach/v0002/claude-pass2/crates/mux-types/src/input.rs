//! Terminal key and mouse input types.

/// Terminal key codes for input handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    Char(char),
    F(u8),
    Up, Down, Left, Right,
    Home, End,
    Insert, Delete,
    PageUp, PageDown,
    Backspace, Tab, BackTab,
    Enter, Escape,
}

/// Key modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct KeyModifiers {
    pub shift: bool,
    pub alt: bool,
    pub ctrl: bool,
    pub meta: bool,
}

/// A key event with code and modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyEvent {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

/// Mouse button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left, Middle, Right,
    WheelUp, WheelDown,
    Extended(u8),
}

/// Mouse event kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseEventKind {
    Press(MouseButton),
    Release(MouseButton),
    Drag(MouseButton),
    Motion,
}

/// A mouse event with position and modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MouseEvent {
    pub kind: MouseEventKind,
    pub x: u16,
    pub y: u16,
    pub modifiers: KeyModifiers,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_event_equality() {
        let a = KeyEvent { code: KeyCode::Char('a'), modifiers: KeyModifiers::default() };
        let b = KeyEvent { code: KeyCode::Char('a'), modifiers: KeyModifiers::default() };
        assert_eq!(a, b);
    }

    #[test]
    fn key_modifiers_default_all_false() {
        let m = KeyModifiers::default();
        assert!(!m.shift);
        assert!(!m.alt);
        assert!(!m.ctrl);
        assert!(!m.meta);
    }

    #[test]
    fn mouse_event_debug() {
        let e = MouseEvent {
            kind: MouseEventKind::Press(MouseButton::Left),
            x: 10, y: 5,
            modifiers: KeyModifiers::default(),
        };
        let s = format!("{e:?}");
        assert!(s.contains("Press"));
    }
}
