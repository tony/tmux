//! # mux-orm
//!
//! Object-relational mapping for kernel entities.
//! Provides typed queries against the session/window/pane hierarchy.
//!
//! L3 logic crate.

#![forbid(unsafe_code)]

use mux_types::{SessionId, WindowId, PaneId};

/// An entity reference that can point to any kernel entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntityRef {
    Session(SessionId),
    Window(WindowId),
    Pane(PaneId),
}

/// A query for finding entities.
#[derive(Debug, Clone)]
pub struct EntityQuery {
    /// Filter by entity type.
    pub entity_type: Option<EntityType>,
    /// Filter by name pattern.
    pub name_pattern: Option<String>,
}

/// Entity type discriminant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityType {
    Session,
    Window,
    Pane,
}

impl EntityQuery {
    /// Create a new query matching all entities.
    #[must_use]
    pub fn all() -> Self {
        Self {
            entity_type: None,
            name_pattern: None,
        }
    }

    /// Filter to sessions only.
    #[must_use]
    pub fn sessions() -> Self {
        Self {
            entity_type: Some(EntityType::Session),
            name_pattern: None,
        }
    }

    /// Filter by name pattern.
    #[must_use]
    pub fn with_name(mut self, pattern: impl Into<String>) -> Self {
        self.name_pattern = Some(pattern.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_ref_session() {
        let r = EntityRef::Session(SessionId::new(1));
        assert!(matches!(r, EntityRef::Session(_)));
    }

    #[test]
    fn query_all() {
        let q = EntityQuery::all();
        assert!(q.entity_type.is_none());
        assert!(q.name_pattern.is_none());
    }

    #[test]
    fn query_sessions() {
        let q = EntityQuery::sessions();
        assert_eq!(q.entity_type, Some(EntityType::Session));
    }

    #[test]
    fn query_with_name() {
        let q = EntityQuery::sessions().with_name("main*");
        assert_eq!(q.name_pattern.as_deref(), Some("main*"));
    }

    #[test]
    fn entity_type_equality() {
        assert_eq!(EntityType::Session, EntityType::Session);
        assert_ne!(EntityType::Session, EntityType::Window);
    }

    #[test]
    fn entity_ref_pane() {
        let r = EntityRef::Pane(PaneId::new(42));
        assert!(matches!(r, EntityRef::Pane(_)));
    }

    #[test]
    fn entity_ref_window() {
        let r = EntityRef::Window(WindowId::new(7));
        assert!(matches!(r, EntityRef::Window(_)));
    }

    #[test]
    fn entity_ref_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(EntityRef::Session(SessionId::new(1)));
        set.insert(EntityRef::Session(SessionId::new(2)));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn entity_ref_clone() {
        let r = EntityRef::Session(SessionId::new(5));
        let r2 = r;
        assert_eq!(r, r2);
    }

    #[test]
    fn query_windows() {
        let q = EntityQuery {
            entity_type: Some(EntityType::Window),
            name_pattern: None,
        };
        assert_eq!(q.entity_type, Some(EntityType::Window));
    }

    #[test]
    fn entity_query_sessions_filter() {
        let q = EntityQuery::sessions();
        assert_eq!(q.entity_type, Some(EntityType::Session));
        assert!(q.name_pattern.is_none());
    }

    #[test]
    fn entity_ref_pane_variant() {
        let r = EntityRef::Pane(PaneId::new(42));
        assert_eq!(r, EntityRef::Pane(PaneId::new(42)));
    }
}
