use bitflags::bitflags;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Key {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyCode {
    Char(char),
    Backspace,
    Enter,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    Tab,
    BackTab,
    Delete,
    Insert,
    F(u8),
    Null,
    Esc,
}

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct KeyModifiers: u8 {
        const SHIFT = 0b0000_0001;
        const CTRL = 0b0000_0010;
        const ALT = 0b0000_0100;
        const SUPER = 0b0000_1000;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MouseEvent {
    pub x: u16,
    pub y: u16,
    pub button: MouseButton,
    pub modifiers: KeyModifiers,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    WheelUp,
    WheelDown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_creation() {
        let k = Key {
            code: KeyCode::Char('a'),
            modifiers: KeyModifiers::empty(),
        };
        assert_eq!(k.code, KeyCode::Char('a'));
    }

    #[test]
    fn test_modifiers() {
        let mut m = KeyModifiers::empty();
        m.insert(KeyModifiers::CTRL);
        assert!(m.contains(KeyModifiers::CTRL));
        assert!(!m.contains(KeyModifiers::SHIFT));
    }

    #[test]
    fn test_modifier_combinations() {
        let m = KeyModifiers::CTRL | KeyModifiers::SHIFT;
        assert!(m.contains(KeyModifiers::CTRL));
        assert!(m.contains(KeyModifiers::SHIFT));
    }

    #[test]
    fn test_mouse_event() {
        let me = MouseEvent {
            x: 10,
            y: 20,
            button: MouseButton::Left,
            modifiers: KeyModifiers::empty(),
        };
        assert_eq!(me.x, 10);
        assert_eq!(me.button, MouseButton::Left);
    }
}
