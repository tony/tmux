//! ORM-like QuerySet API for entity traversal.
//!
//! Inspired by libtmux's QuerySet pattern and Django's ORM.
//! Provides fluent filtering, traversal, and lookup across sessions/windows/panes.

#![forbid(unsafe_code)]

use mux_types::id::{SessionId, WindowId, PaneId};

/// Entity reference - a typed wrapper for entity lookups.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EntityRef {
    Session(SessionId),
    Window(WindowId),
    Pane(PaneId),
}

/// Information about a session for ORM queries.
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub id: SessionId,
    pub name: String,
    pub window_count: usize,
    pub attached: bool,
}

/// Information about a window for ORM queries.
#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub id: WindowId,
    pub session_id: SessionId,
    pub name: String,
    pub index: usize,
    pub pane_count: usize,
    pub active: bool,
}

/// Information about a pane for ORM queries.
#[derive(Debug, Clone)]
pub struct PaneInfo {
    pub id: PaneId,
    pub window_id: WindowId,
    pub index: usize,
    pub active: bool,
    pub width: u16,
    pub height: u16,
}

/// A lazy filter expression.
#[derive(Debug, Clone)]
pub enum Filter {
    NameEquals(String),
    NameContains(String),
    IsAttached(bool),
    IsActive(bool),
    IndexEquals(usize),
    And(Box<Filter>, Box<Filter>),
    Or(Box<Filter>, Box<Filter>),
}

impl Filter {
    pub fn and(self, other: Self) -> Self {
        Self::And(Box::new(self), Box::new(other))
    }

    pub fn or(self, other: Self) -> Self {
        Self::Or(Box::new(self), Box::new(other))
    }
}

/// A QuerySet for sessions.
#[derive(Debug, Clone)]
pub struct SessionQuerySet {
    sessions: Vec<SessionInfo>,
    filters: Vec<Filter>,
}

impl SessionQuerySet {
    pub fn new(sessions: Vec<SessionInfo>) -> Self {
        Self {
            sessions,
            filters: Vec::new(),
        }
    }

    pub fn filter(mut self, f: Filter) -> Self {
        self.filters.push(f);
        self
    }

    pub fn filter_by_name(self, name: &str) -> Self {
        self.filter(Filter::NameEquals(name.to_owned()))
    }

    pub fn filter_attached(self) -> Self {
        self.filter(Filter::IsAttached(true))
    }

    fn matches_filter(session: &SessionInfo, filter: &Filter) -> bool {
        match filter {
            Filter::NameEquals(n) => session.name == *n,
            Filter::NameContains(n) => session.name.contains(n.as_str()),
            Filter::IsAttached(a) => session.attached == *a,
            Filter::And(a, b) => Self::matches_filter(session, a) && Self::matches_filter(session, b),
            Filter::Or(a, b) => Self::matches_filter(session, a) || Self::matches_filter(session, b),
            _ => true,
        }
    }

    pub fn evaluate(&self) -> Vec<&SessionInfo> {
        self.sessions
            .iter()
            .filter(|s| self.filters.iter().all(|f| Self::matches_filter(s, f)))
            .collect()
    }

    pub fn first(&self) -> Option<&SessionInfo> {
        self.evaluate().into_iter().next()
    }

    pub fn count(&self) -> usize {
        self.evaluate().len()
    }

    pub fn exists(&self) -> bool {
        self.count() > 0
    }

    pub fn get_by_id(&self, id: SessionId) -> Option<&SessionInfo> {
        self.sessions.iter().find(|s| s.id == id)
    }

    pub fn names(&self) -> Vec<&str> {
        self.evaluate().iter().map(|s| s.name.as_str()).collect()
    }
}

/// A QuerySet for windows.
#[derive(Debug, Clone)]
pub struct WindowQuerySet {
    windows: Vec<WindowInfo>,
    filters: Vec<Filter>,
}

impl WindowQuerySet {
    pub fn new(windows: Vec<WindowInfo>) -> Self {
        Self {
            windows,
            filters: Vec::new(),
        }
    }

    pub fn filter(mut self, f: Filter) -> Self {
        self.filters.push(f);
        self
    }

    pub fn filter_by_name(self, name: &str) -> Self {
        self.filter(Filter::NameEquals(name.to_owned()))
    }

    pub fn filter_active(self) -> Self {
        self.filter(Filter::IsActive(true))
    }

    fn matches_filter(window: &WindowInfo, filter: &Filter) -> bool {
        match filter {
            Filter::NameEquals(n) => window.name == *n,
            Filter::NameContains(n) => window.name.contains(n.as_str()),
            Filter::IsActive(a) => window.active == *a,
            Filter::IndexEquals(i) => window.index == *i,
            Filter::And(a, b) => Self::matches_filter(window, a) && Self::matches_filter(window, b),
            Filter::Or(a, b) => Self::matches_filter(window, a) || Self::matches_filter(window, b),
            _ => true,
        }
    }

    pub fn evaluate(&self) -> Vec<&WindowInfo> {
        self.windows
            .iter()
            .filter(|w| self.filters.iter().all(|f| Self::matches_filter(w, f)))
            .collect()
    }

    pub fn first(&self) -> Option<&WindowInfo> {
        self.evaluate().into_iter().next()
    }

    pub fn count(&self) -> usize {
        self.evaluate().len()
    }

    pub fn get_by_id(&self, id: WindowId) -> Option<&WindowInfo> {
        self.windows.iter().find(|w| w.id == id)
    }
}

/// A QuerySet for panes.
#[derive(Debug, Clone)]
pub struct PaneQuerySet {
    panes: Vec<PaneInfo>,
    filters: Vec<Filter>,
}

impl PaneQuerySet {
    pub fn new(panes: Vec<PaneInfo>) -> Self {
        Self {
            panes,
            filters: Vec::new(),
        }
    }

    pub fn filter(mut self, f: Filter) -> Self {
        self.filters.push(f);
        self
    }

    pub fn filter_active(self) -> Self {
        self.filter(Filter::IsActive(true))
    }

    fn matches_filter(pane: &PaneInfo, filter: &Filter) -> bool {
        match filter {
            Filter::IsActive(a) => pane.active == *a,
            Filter::IndexEquals(i) => pane.index == *i,
            Filter::And(a, b) => Self::matches_filter(pane, a) && Self::matches_filter(pane, b),
            Filter::Or(a, b) => Self::matches_filter(pane, a) || Self::matches_filter(pane, b),
            _ => true,
        }
    }

    pub fn evaluate(&self) -> Vec<&PaneInfo> {
        self.panes
            .iter()
            .filter(|p| self.filters.iter().all(|f| Self::matches_filter(p, f)))
            .collect()
    }

    pub fn first(&self) -> Option<&PaneInfo> {
        self.evaluate().into_iter().next()
    }

    pub fn count(&self) -> usize {
        self.evaluate().len()
    }

    pub fn get_by_id(&self, id: PaneId) -> Option<&PaneInfo> {
        self.panes.iter().find(|p| p.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn sample_sessions() -> Vec<SessionInfo> {
        vec![
            SessionInfo { id: SessionId(1), name: "dev".into(), window_count: 2, attached: true },
            SessionInfo { id: SessionId(2), name: "prod".into(), window_count: 1, attached: false },
            SessionInfo { id: SessionId(3), name: "staging".into(), window_count: 3, attached: false },
        ]
    }

    fn sample_windows() -> Vec<WindowInfo> {
        vec![
            WindowInfo { id: WindowId(1), session_id: SessionId(1), name: "editor".into(), index: 0, pane_count: 2, active: true },
            WindowInfo { id: WindowId(2), session_id: SessionId(1), name: "shell".into(), index: 1, pane_count: 1, active: false },
            WindowInfo { id: WindowId(3), session_id: SessionId(2), name: "main".into(), index: 0, pane_count: 1, active: true },
        ]
    }

    fn sample_panes() -> Vec<PaneInfo> {
        vec![
            PaneInfo { id: PaneId(1), window_id: WindowId(1), index: 0, active: true, width: 80, height: 24 },
            PaneInfo { id: PaneId(2), window_id: WindowId(1), index: 1, active: false, width: 80, height: 24 },
            PaneInfo { id: PaneId(3), window_id: WindowId(2), index: 0, active: true, width: 80, height: 24 },
        ]
    }

    #[test]
    fn session_query_all() {
        let qs = SessionQuerySet::new(sample_sessions());
        assert_eq!(qs.count(), 3);
    }

    #[test]
    fn session_query_by_name() {
        let qs = SessionQuerySet::new(sample_sessions()).filter_by_name("dev");
        assert_eq!(qs.count(), 1);
        assert_eq!(qs.first().map(|s| s.name.as_str()), Some("dev"));
    }

    #[test]
    fn session_query_attached() {
        let qs = SessionQuerySet::new(sample_sessions()).filter_attached();
        assert_eq!(qs.count(), 1);
    }

    #[test]
    fn session_query_chained() {
        let qs = SessionQuerySet::new(sample_sessions())
            .filter(Filter::IsAttached(false))
            .filter(Filter::NameContains("stag".into()));
        assert_eq!(qs.count(), 1);
    }

    #[test]
    fn session_query_not_found() {
        let qs = SessionQuerySet::new(sample_sessions()).filter_by_name("nonexistent");
        assert_eq!(qs.count(), 0);
        assert!(!qs.exists());
    }

    #[test]
    fn session_get_by_id() {
        let qs = SessionQuerySet::new(sample_sessions());
        assert!(qs.get_by_id(SessionId(1)).is_some());
        assert!(qs.get_by_id(SessionId(99)).is_none());
    }

    #[test]
    fn session_names() {
        let qs = SessionQuerySet::new(sample_sessions());
        let names = qs.names();
        assert_eq!(names.len(), 3);
        assert!(names.contains(&"dev"));
    }

    #[test]
    fn window_query_by_name() {
        let qs = WindowQuerySet::new(sample_windows()).filter_by_name("editor");
        assert_eq!(qs.count(), 1);
    }

    #[test]
    fn window_query_active() {
        let qs = WindowQuerySet::new(sample_windows()).filter_active();
        assert_eq!(qs.count(), 2);
    }

    #[test]
    fn window_get_by_id() {
        let qs = WindowQuerySet::new(sample_windows());
        assert!(qs.get_by_id(WindowId(1)).is_some());
    }

    #[test]
    fn pane_query_active() {
        let qs = PaneQuerySet::new(sample_panes()).filter_active();
        assert_eq!(qs.count(), 2);
    }

    #[test]
    fn pane_query_by_index() {
        let qs = PaneQuerySet::new(sample_panes()).filter(Filter::IndexEquals(0));
        assert_eq!(qs.count(), 2);
    }

    #[test]
    fn pane_get_by_id() {
        let qs = PaneQuerySet::new(sample_panes());
        assert!(qs.get_by_id(PaneId(1)).is_some());
    }

    #[test]
    fn filter_and_composition() {
        let f = Filter::NameContains("dev".into()).and(Filter::IsAttached(true));
        let sessions = sample_sessions();
        let qs = SessionQuerySet::new(sessions).filter(f);
        assert_eq!(qs.count(), 1);
    }

    #[test]
    fn filter_or_composition() {
        let f = Filter::NameEquals("dev".into()).or(Filter::NameEquals("prod".into()));
        let qs = SessionQuerySet::new(sample_sessions()).filter(f);
        assert_eq!(qs.count(), 2);
    }

    #[test]
    fn entity_ref_variants() {
        let r1 = EntityRef::Session(SessionId(1));
        let r2 = EntityRef::Window(WindowId(1));
        let r3 = EntityRef::Pane(PaneId(1));
        assert_ne!(r1, r2);
        assert_ne!(r2, r3);
    }

    #[test]
    fn entity_ref_hashing() {
        let mut map = HashMap::new();
        map.insert(EntityRef::Session(SessionId(1)), "s1");
        map.insert(EntityRef::Window(WindowId(1)), "w1");
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn session_exists() {
        let qs = SessionQuerySet::new(sample_sessions()).filter_by_name("dev");
        assert!(qs.exists());
    }

    #[test]
    fn empty_query_set() {
        let qs = SessionQuerySet::new(vec![]);
        assert_eq!(qs.count(), 0);
        assert!(!qs.exists());
    }

    #[test]
    fn window_first() {
        let qs = WindowQuerySet::new(sample_windows());
        assert!(qs.first().is_some());
    }

    #[test]
    fn pane_first() {
        let qs = PaneQuerySet::new(sample_panes());
        assert!(qs.first().is_some());
    }

    mod prop {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn filter_count_le_total(n in 1usize..20) {
                let sessions: Vec<SessionInfo> = (0..n).map(|i| SessionInfo {
                    id: SessionId(i as u64),
                    name: format!("s{i}"),
                    window_count: 1,
                    attached: i % 2 == 0,
                }).collect();
                let qs = SessionQuerySet::new(sessions).filter_attached();
                prop_assert!(qs.count() <= n);
            }

            #[test]
            fn name_filter_exact(name in "[a-z]{1,10}") {
                let sessions = vec![
                    SessionInfo { id: SessionId(1), name: name.clone(), window_count: 1, attached: false },
                    SessionInfo { id: SessionId(2), name: "other".into(), window_count: 1, attached: false },
                ];
                let qs = SessionQuerySet::new(sessions).filter_by_name(&name);
                prop_assert_eq!(qs.count(), 1);
            }
        }
    }
}
