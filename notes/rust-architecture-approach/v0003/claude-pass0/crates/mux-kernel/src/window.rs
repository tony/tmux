//! Window entity.
//!
//! A window contains one or more panes arranged by a layout tree.

use mux_types::WindowId;
use crate::pane::Pane;
use crate::layout::LayoutNode;

/// A multiplexer window.
#[derive(Debug)]
pub struct Window {
    /// Window identifier.
    pub id: WindowId,
    /// Window name.
    pub name: String,
    /// Panes in this window.
    pub panes: Vec<Pane>,
    /// Index of the active pane.
    pub active_pane_idx: usize,
    /// Layout tree root.
    pub layout: LayoutNode,
}

impl Window {
    /// Create a new window with one initial pane.
    #[must_use]
    pub fn new(id: WindowId, name: &str, initial_pane: Pane) -> Self {
        let pane_id = initial_pane.id;
        Self {
            id,
            name: name.to_owned(),
            panes: vec![initial_pane],
            active_pane_idx: 0,
            layout: LayoutNode::Pane(pane_id),
        }
    }

    /// Get the active pane.
    #[must_use]
    pub fn active_pane(&self) -> Option<&Pane> {
        self.panes.get(self.active_pane_idx)
    }

    /// Get the active pane mutably.
    pub fn active_pane_mut(&mut self) -> Option<&mut Pane> {
        self.panes.get_mut(self.active_pane_idx)
    }

    /// Number of panes.
    #[must_use]
    pub fn pane_count(&self) -> usize {
        self.panes.len()
    }

    /// Add a pane to this window.
    pub fn add_pane(&mut self, pane: Pane) {
        self.panes.push(pane);
    }

    /// Select the next pane.
    pub fn select_next_pane(&mut self) {
        if !self.panes.is_empty() {
            self.active_pane_idx = (self.active_pane_idx + 1) % self.panes.len();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mux_types::{PaneId, Size};

    #[test]
    fn window_new() {
        let pane = Pane::new(PaneId::new(1), Size::new(80, 24));
        let w = Window::new(WindowId::new(1), "main", pane);
        assert_eq!(w.pane_count(), 1);
        assert_eq!(w.name, "main");
    }

    #[test]
    fn window_active_pane() {
        let pane = Pane::new(PaneId::new(1), Size::new(80, 24));
        let w = Window::new(WindowId::new(1), "main", pane);
        assert!(w.active_pane().is_some());
    }

    #[test]
    fn window_add_pane() {
        let p1 = Pane::new(PaneId::new(1), Size::new(80, 24));
        let p2 = Pane::new(PaneId::new(2), Size::new(80, 24));
        let mut w = Window::new(WindowId::new(1), "main", p1);
        w.add_pane(p2);
        assert_eq!(w.pane_count(), 2);
    }

    #[test]
    fn window_cycle_panes() {
        let p1 = Pane::new(PaneId::new(1), Size::new(80, 24));
        let p2 = Pane::new(PaneId::new(2), Size::new(80, 24));
        let mut w = Window::new(WindowId::new(1), "main", p1);
        w.add_pane(p2);
        w.select_next_pane();
        assert_eq!(w.active_pane_idx, 1);
        w.select_next_pane();
        assert_eq!(w.active_pane_idx, 0);
    }
}
