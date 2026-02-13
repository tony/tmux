//! Session entity.

use crate::window::Window;
use mux_types::SessionId;

/// A terminal multiplexer session.
#[derive(Debug)]
pub struct Session {
    /// Session identifier.
    pub id: SessionId,
    /// Session name.
    pub name: String,
    /// Windows belonging to this session.
    pub windows: Vec<Window>,
    /// Index of the active window.
    pub active_window: usize,
}

impl Session {
    /// Create a new session with an initial window.
    #[must_use]
    pub fn new(id: SessionId, name: String, initial_window: Window) -> Self {
        Self {
            id,
            name,
            windows: vec![initial_window],
            active_window: 0,
        }
    }

    /// Number of windows in this session.
    #[must_use]
    pub fn window_count(&self) -> usize {
        self.windows.len()
    }

    /// Get the active window.
    #[must_use]
    pub fn active_window(&self) -> Option<&Window> {
        self.windows.get(self.active_window)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pane::Pane;
    use mux_types::{PaneId, Size, WindowId};

    #[test]
    fn session_creation() {
        let pane = Pane::new(PaneId::new(1), Size::new(80, 24));
        let win = Window::new(WindowId::new(1), "win0", pane);
        let session = Session::new(SessionId::new(1), "test".into(), win);
        assert_eq!(session.name, "test");
        assert_eq!(session.window_count(), 1);
    }

    #[test]
    fn active_window() {
        let pane = Pane::new(PaneId::new(1), Size::new(80, 24));
        let win = Window::new(WindowId::new(1), "win0", pane);
        let session = Session::new(SessionId::new(1), "test".into(), win);
        assert!(session.active_window().is_some());
    }
}
