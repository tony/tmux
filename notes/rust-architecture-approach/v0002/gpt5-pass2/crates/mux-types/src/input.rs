//! Input event core types.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCode {
    Char(char),
    Enter,
    Esc,
    Backspace,
    Tab,
    Left,
    Right,
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
    WheelUp,
    WheelDown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MouseEvent {
    pub row: u16,
    pub col: u16,
    pub button: MouseButton,
    pub dragging: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keycode_char_roundtrip() {
        assert_eq!(KeyCode::Char('x'), KeyCode::Char('x'));
    }

    #[test]
    fn mouse_event_fields() {
        let m = MouseEvent { row: 2, col: 9, button: MouseButton::Left, dragging: true };
        assert_eq!(m.row, 2);
        assert!(m.dragging);
    }
}
