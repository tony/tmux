//! Key and mouse input types.
//!
//! Provides strongly-typed representations of keyboard and mouse events
//! for the kernel's input processing pipeline.

use bitflags::bitflags;

/// Key code representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    /// A printable Unicode character.
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
    /// Home.
    Home,
    /// End.
    End,
    /// Page up.
    PageUp,
    /// Page down.
    PageDown,
    /// Insert.
    Insert,
    /// Delete.
    Delete,
}

bitflags! {
    /// Key modifiers.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct KeyModifiers: u8 {
        const SHIFT = 0x01;
        const ALT   = 0x02;
        const CTRL  = 0x04;
        const META  = 0x08;
    }
}

/// A keyboard event combining a key code with modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyEvent {
    /// The key that was pressed.
    pub code: KeyCode,
    /// Active modifier keys.
    pub modifiers: KeyModifiers,
}

impl KeyEvent {
    /// Create a key event with no modifiers.
    #[must_use]
    pub const fn new(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::empty(),
        }
    }

    /// Create a key event with Ctrl modifier.
    #[must_use]
    pub const fn ctrl(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::CTRL,
        }
    }

    /// Whether the Ctrl modifier is active.
    #[must_use]
    pub const fn has_ctrl(&self) -> bool {
        self.modifiers.contains(KeyModifiers::CTRL)
    }

    /// Whether the Alt modifier is active.
    #[must_use]
    pub const fn has_alt(&self) -> bool {
        self.modifiers.contains(KeyModifiers::ALT)
    }

    /// Whether the Shift modifier is active.
    #[must_use]
    pub const fn has_shift(&self) -> bool {
        self.modifiers.contains(KeyModifiers::SHIFT)
    }
}

/// Mouse button identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    /// Left button.
    Left,
    /// Middle button.
    Middle,
    /// Right button.
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
    /// Column (0-based).
    pub x: u16,
    /// Row (0-based).
    pub y: u16,
    /// Active modifier keys.
    pub modifiers: KeyModifiers,
}

impl MouseEvent {
    /// Create a simple mouse press event.
    #[must_use]
    pub const fn press(button: MouseButton, x: u16, y: u16) -> Self {
        Self {
            kind: MouseEventKind::Press(button),
            x,
            y,
            modifiers: KeyModifiers::empty(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_event_no_modifiers() {
        let ev = KeyEvent::new(KeyCode::Char('a'));
        assert!(!ev.has_ctrl());
        assert!(!ev.has_alt());
        assert!(!ev.has_shift());
    }

    #[test]
    fn key_event_ctrl() {
        let ev = KeyEvent::ctrl(KeyCode::Char('c'));
        assert!(ev.has_ctrl());
        assert!(!ev.has_alt());
    }

    #[test]
    fn key_event_equality() {
        let a = KeyEvent::new(KeyCode::Enter);
        let b = KeyEvent::new(KeyCode::Enter);
        assert_eq!(a, b);
    }

    #[test]
    fn key_modifiers_combine() {
        let m = KeyModifiers::SHIFT | KeyModifiers::ALT;
        assert!(m.contains(KeyModifiers::SHIFT));
        assert!(m.contains(KeyModifiers::ALT));
        assert!(!m.contains(KeyModifiers::CTRL));
    }

    #[test]
    fn mouse_event_press() {
        let ev = MouseEvent::press(MouseButton::Left, 10, 20);
        assert_eq!(ev.x, 10);
        assert_eq!(ev.y, 20);
        assert!(matches!(ev.kind, MouseEventKind::Press(MouseButton::Left)));
    }

    #[test]
    fn function_key_range() {
        for n in 1..=12 {
            let key = KeyCode::F(n);
            assert!(matches!(key, KeyCode::F(_)));
        }
    }

    #[test]
    fn key_event_different_modifiers_not_equal() {
        let a = KeyEvent::new(KeyCode::Char('a'));
        let b = KeyEvent::ctrl(KeyCode::Char('a'));
        assert_ne!(a, b);
    }
}
