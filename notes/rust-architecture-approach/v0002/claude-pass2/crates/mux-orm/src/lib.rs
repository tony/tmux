//! # mux-orm
//!
//! ORM-like traversal API inspired by Python's libtmux QueryList.

#![forbid(unsafe_code)]

use mux_types::{ClientId, PaneId, SessionId, WindowId};

#[derive(Debug, Clone)]
pub struct QuerySet<T> {
    items: Vec<T>,
}

impl<T> QuerySet<T> {
    #[must_use]
    pub fn new(items: Vec<T>) -> Self {
        Self { items }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    #[must_use]
    pub fn get<F>(&self, predicate: F) -> Option<&T>
    where
        F: Fn(&T) -> bool,
    {
        self.items.iter().find(|i| predicate(i))
    }

    #[must_use]
    pub fn filter<F>(&self, predicate: F) -> Self
    where
        T: Clone,
        F: Fn(&T) -> bool,
    {
        Self {
            items: self.items.iter().filter(|i| predicate(i)).cloned().collect(),
        }
    }

    #[must_use]
    pub fn first(&self) -> Option<&T> {
        self.items.first()
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.items.iter()
    }

    #[must_use]
    pub fn into_vec(self) -> Vec<T> {
        self.items
    }
}

impl<T> IntoIterator for QuerySet<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a QuerySet<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}

#[derive(Debug, Clone)]
pub struct SessionView {
    pub id: SessionId,
    pub name: String,
    pub window_ids: Vec<WindowId>,
    pub attached: bool,
    pub created: i64,
}

#[derive(Debug, Clone)]
pub struct WindowView {
    pub id: WindowId,
    pub session_id: SessionId,
    pub name: String,
    pub index: u32,
    pub pane_ids: Vec<PaneId>,
    pub active: bool,
}

#[derive(Debug, Clone)]
pub struct PaneView {
    pub id: PaneId,
    pub window_id: WindowId,
    pub session_id: SessionId,
    pub index: u32,
    pub active: bool,
    pub pid: Option<u32>,
    pub title: String,
    pub cols: u32,
    pub rows: u32,
}

#[derive(Debug, Clone)]
pub struct ClientView {
    pub id: ClientId,
    pub session_id: Option<SessionId>,
    pub name: String,
    pub tty: String,
    pub cols: u32,
    pub rows: u32,
}

#[derive(Debug, Clone)]
pub struct ServerView {
    pub sessions: QuerySet<SessionView>,
    pub windows: QuerySet<WindowView>,
    pub panes: QuerySet<PaneView>,
    pub clients: QuerySet<ClientView>,
}

impl ServerView {
    #[must_use]
    pub fn sessions(&self) -> &QuerySet<SessionView> {
        &self.sessions
    }

    #[must_use]
    pub fn windows(&self) -> &QuerySet<WindowView> {
        &self.windows
    }

    #[must_use]
    pub fn panes(&self) -> &QuerySet<PaneView> {
        &self.panes
    }

    #[must_use]
    pub fn session_by_name(&self, name: &str) -> Option<&SessionView> {
        self.sessions.get(|s| s.name == name)
    }

    #[must_use]
    pub fn session_by_id(&self, id: SessionId) -> Option<&SessionView> {
        self.sessions.get(|s| s.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queryset_filter() {
        let sessions = QuerySet::new(vec![
            SessionView {
                id: SessionId(1), name: "main".into(), window_ids: vec![],
                attached: true, created: 0,
            },
            SessionView {
                id: SessionId(2), name: "other".into(), window_ids: vec![],
                attached: false, created: 0,
            },
        ]);
        let attached = sessions.filter(|s| s.attached);
        assert_eq!(attached.len(), 1);
        assert_eq!(attached.first().map(|s| s.name.as_str()), Some("main"));
    }

    #[test]
    fn queryset_get() {
        let sessions = QuerySet::new(vec![
            SessionView {
                id: SessionId(1), name: "test".into(), window_ids: vec![],
                attached: false, created: 0,
            },
        ]);
        assert!(sessions.get(|s| s.name == "test").is_some());
        assert!(sessions.get(|s| s.name == "nope").is_none());
    }

    #[test]
    fn queryset_into_vec() {
        let qs = QuerySet::new(vec![1, 2, 3]);
        assert_eq!(qs.into_vec(), vec![1, 2, 3]);
    }
}
