//! # mux-orm
//!
//! ORM-like QuerySet traversal API for navigating the entity hierarchy.
//!
//! L3 logic crate.

#![forbid(unsafe_code)]

use mux_types::{SessionId, WindowId, PaneId};

/// A lightweight view of the server entity hierarchy for querying.
#[derive(Debug, Clone)]
pub struct ServerView {
    pub sessions: Vec<SessionEntry>,
}

/// A session entry in the server view.
#[derive(Debug, Clone)]
pub struct SessionEntry {
    pub id: SessionId,
    pub name: String,
    pub windows: Vec<WindowEntry>,
}

/// A window entry.
#[derive(Debug, Clone)]
pub struct WindowEntry {
    pub id: WindowId,
    pub name: String,
    pub session_id: SessionId,
    pub panes: Vec<PaneEntry>,
}

/// A pane entry.
#[derive(Debug, Clone)]
pub struct PaneEntry {
    pub id: PaneId,
    pub window_id: WindowId,
}

impl ServerView {
    /// Create a new empty server view.
    #[must_use]
    pub fn new() -> Self {
        Self { sessions: Vec::new() }
    }

    /// Find a session by name.
    #[must_use]
    pub fn session_by_name(&self, name: &str) -> Option<&SessionEntry> {
        self.sessions.iter().find(|s| s.name == name)
    }

    /// Find a session by ID.
    #[must_use]
    pub fn session_by_id(&self, id: SessionId) -> Option<&SessionEntry> {
        self.sessions.iter().find(|s| s.id == id)
    }

    /// Iterate all windows across all sessions.
    pub fn all_windows(&self) -> impl Iterator<Item = &WindowEntry> {
        self.sessions.iter().flat_map(|s| s.windows.iter())
    }

    /// Iterate all panes across all sessions and windows.
    pub fn all_panes(&self) -> impl Iterator<Item = &PaneEntry> {
        self.all_windows().flat_map(|w| w.panes.iter())
    }

    /// Count total sessions.
    #[must_use]
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Count total windows.
    #[must_use]
    pub fn window_count(&self) -> usize {
        self.sessions.iter().map(|s| s.windows.len()).sum()
    }

    /// Count total panes.
    #[must_use]
    pub fn pane_count(&self) -> usize {
        self.all_windows().map(|w| w.panes.len()).sum()
    }
}

impl Default for ServerView {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_view() -> ServerView {
        ServerView {
            sessions: vec![
                SessionEntry {
                    id: SessionId::new(1),
                    name: "main".into(),
                    windows: vec![
                        WindowEntry {
                            id: WindowId::new(1),
                            name: "editor".into(),
                            session_id: SessionId::new(1),
                            panes: vec![
                                PaneEntry { id: PaneId::new(1), window_id: WindowId::new(1) },
                                PaneEntry { id: PaneId::new(2), window_id: WindowId::new(1) },
                            ],
                        },
                    ],
                },
                SessionEntry {
                    id: SessionId::new(2),
                    name: "build".into(),
                    windows: vec![
                        WindowEntry {
                            id: WindowId::new(2),
                            name: "cargo".into(),
                            session_id: SessionId::new(2),
                            panes: vec![
                                PaneEntry { id: PaneId::new(3), window_id: WindowId::new(2) },
                            ],
                        },
                    ],
                },
            ],
        }
    }

    #[test]
    fn session_by_name() {
        let view = sample_view();
        assert!(view.session_by_name("main").is_some());
        assert!(view.session_by_name("nonexistent").is_none());
    }

    #[test]
    fn session_by_id() {
        let view = sample_view();
        assert!(view.session_by_id(SessionId::new(1)).is_some());
    }

    #[test]
    fn counts() {
        let view = sample_view();
        assert_eq!(view.session_count(), 2);
        assert_eq!(view.window_count(), 2);
        assert_eq!(view.pane_count(), 3);
    }

    #[test]
    fn all_windows_iteration() {
        let view = sample_view();
        let windows: Vec<_> = view.all_windows().collect();
        assert_eq!(windows.len(), 2);
    }

    #[test]
    fn all_panes_iteration() {
        let view = sample_view();
        let panes: Vec<_> = view.all_panes().collect();
        assert_eq!(panes.len(), 3);
    }

    #[test]
    fn empty_view() {
        let view = ServerView::new();
        assert_eq!(view.session_count(), 0);
        assert_eq!(view.window_count(), 0);
        assert_eq!(view.pane_count(), 0);
    }
}
