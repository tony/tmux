//! Session entity.
//!
//! A session contains one or more windows and is the top-level container
//! for a user's work context.

use mux_types::SessionId;
use crate::window::Window;

/// A multiplexer session.
#[derive(Debug)]
pub struct Session {
    /// Session identifier.
    pub id: SessionId,
    /// Session name.
    pub name: String,
    /// Windows in this session.
    pub windows: Vec<Window>,
    /// Index of the active window.
    pub active_window_idx: usize,
}

impl Session {
    /// Create a new session with one initial window.
    #[must_use]
    pub fn new(id: SessionId, name: String, initial_window: Window) -> Self {
        Self {
            id,
            name,
            windows: vec![initial_window],
            active_window_idx: 0,
        }
    }

    /// Get the active window.
    #[must_use]
    pub fn active_window(&self) -> Option<&Window> {
        self.windows.get(self.active_window_idx)
    }

    /// Get the active window mutably.
    pub fn active_window_mut(&mut self) -> Option<&mut Window> {
        self.windows.get_mut(self.active_window_idx)
    }

    /// Number of windows in this session.
    #[must_use]
    pub fn window_count(&self) -> usize {
        self.windows.len()
    }

    /// Add a window to this session.
    pub fn add_window(&mut self, window: Window) {
        self.windows.push(window);
    }

    /// Select the next window.
    pub fn select_next_window(&mut self) {
        if !self.windows.is_empty() {
            self.active_window_idx = (self.active_window_idx + 1) % self.windows.len();
        }
    }

    /// Select the previous window.
    pub fn select_previous_window(&mut self) {
        if !self.windows.is_empty() {
            self.active_window_idx = if self.active_window_idx == 0 {
                self.windows.len() - 1
            } else {
                self.active_window_idx - 1
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pane::Pane;
    use mux_types::{WindowId, PaneId, Size};

    fn make_window(id: u64, name: &str) -> Window {
        let pane = Pane::new(PaneId::new(id + 100), Size::new(80, 24));
        Window::new(WindowId::new(id), name, pane)
    }

    #[test]
    fn session_new() {
        let w = make_window(1, "win1");
        let s = Session::new(SessionId::new(1), "test".into(), w);
        assert_eq!(s.window_count(), 1);
        assert_eq!(s.name, "test");
    }

    #[test]
    fn session_active_window() {
        let w = make_window(1, "win1");
        let s = Session::new(SessionId::new(1), "test".into(), w);
        assert!(s.active_window().is_some());
    }

    #[test]
    fn session_add_window() {
        let w1 = make_window(1, "win1");
        let w2 = make_window(2, "win2");
        let mut s = Session::new(SessionId::new(1), "test".into(), w1);
        s.add_window(w2);
        assert_eq!(s.window_count(), 2);
    }

    #[test]
    fn session_cycle_windows() {
        let w1 = make_window(1, "win1");
        let w2 = make_window(2, "win2");
        let mut s = Session::new(SessionId::new(1), "test".into(), w1);
        s.add_window(w2);

        assert_eq!(s.active_window_idx, 0);
        s.select_next_window();
        assert_eq!(s.active_window_idx, 1);
        s.select_next_window();
        assert_eq!(s.active_window_idx, 0); // wraps
    }

    #[test]
    fn session_previous_window() {
        let w1 = make_window(1, "win1");
        let w2 = make_window(2, "win2");
        let mut s = Session::new(SessionId::new(1), "test".into(), w1);
        s.add_window(w2);

        s.select_previous_window();
        assert_eq!(s.active_window_idx, 1); // wraps to end
    }
}
