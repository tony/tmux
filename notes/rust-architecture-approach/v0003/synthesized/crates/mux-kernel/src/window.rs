//! Window entity.

use crate::pane::Pane;
use mux_types::WindowId;

/// A window containing one or more panes.
#[derive(Debug)]
pub struct Window {
    /// Window identifier.
    pub id: WindowId,
    /// Window name.
    pub name: String,
    /// Panes in this window.
    pub panes: Vec<Pane>,
    /// Index of the active pane.
    pub active_pane: usize,
}

impl Window {
    /// Create a new window with an initial pane.
    #[must_use]
    pub fn new(id: WindowId, name: &str, initial_pane: Pane) -> Self {
        Self {
            id,
            name: name.to_owned(),
            panes: vec![initial_pane],
            active_pane: 0,
        }
    }

    /// Number of panes in this window.
    #[must_use]
    pub fn pane_count(&self) -> usize {
        self.panes.len()
    }

    /// Get the active pane.
    #[must_use]
    pub fn active_pane(&self) -> Option<&Pane> {
        self.panes.get(self.active_pane)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mux_types::{PaneId, Size};

    #[test]
    fn window_creation() {
        let pane = Pane::new(PaneId::new(1), Size::new(80, 24));
        let win = Window::new(WindowId::new(1), "editor", pane);
        assert_eq!(win.name, "editor");
        assert_eq!(win.pane_count(), 1);
    }

    #[test]
    fn window_active_pane() {
        let pane = Pane::new(PaneId::new(1), Size::new(80, 24));
        let win = Window::new(WindowId::new(1), "test", pane);
        assert!(win.active_pane().is_some());
        assert_eq!(win.active_pane().map(|p| p.id), Some(PaneId::new(1)));
    }

    #[test]
    fn window_id_stored() {
        let pane = Pane::new(PaneId::new(1), Size::new(80, 24));
        let win = Window::new(WindowId::new(42), "named", pane);
        assert_eq!(win.id, WindowId::new(42));
    }
}
