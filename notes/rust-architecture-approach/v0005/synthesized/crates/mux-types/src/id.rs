//! Entity identifiers and generation.
//!
//! IDs are monotonically increasing u64 values that are never reused.
//! The generator starts at 1 so that 0 can serve as a sentinel.

/// Unique session identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SessionId(pub u64);

/// Unique window identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WindowId(pub u64);

/// Unique pane identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PaneId(pub u64);

/// Unique client identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ClientId(pub u64);

/// Monotonically increasing ID generator.
///
/// Starts at 1 (not 0) so 0 can be used as a sentinel value.
#[derive(Debug)]
pub struct IdGenerator {
    next: u64,
}

impl Default for IdGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl IdGenerator {
    /// Create a new generator starting at 1.
    pub fn new() -> Self {
        Self { next: 1 }
    }

    /// Generate the next ID.
    pub fn next_session(&mut self) -> SessionId {
        let id = SessionId(self.next);
        self.next += 1;
        id
    }

    /// Generate the next window ID.
    pub fn next_window(&mut self) -> WindowId {
        let id = WindowId(self.next);
        self.next += 1;
        id
    }

    /// Generate the next pane ID.
    pub fn next_pane(&mut self) -> PaneId {
        let id = PaneId(self.next);
        self.next += 1;
        id
    }

    /// Generate the next client ID.
    pub fn next_client(&mut self) -> ClientId {
        let id = ClientId(self.next);
        self.next += 1;
        id
    }

    /// Current counter value (next ID to be issued).
    pub fn current(&self) -> u64 {
        self.next
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_start_at_one() {
        let mut id_gen = IdGenerator::new();
        assert_eq!(id_gen.next_session().0, 1);
    }

    #[test]
    fn ids_are_monotonic() {
        let mut id_gen = IdGenerator::new();
        let a = id_gen.next_pane();
        let b = id_gen.next_pane();
        assert!(b.0 > a.0);
    }

    #[test]
    fn ids_never_reuse() {
        let mut id_gen = IdGenerator::new();
        let mut seen = std::collections::HashSet::new();
        for _ in 0..100 {
            let id = id_gen.next_session();
            assert!(seen.insert(id.0));
        }
    }

    #[test]
    fn mixed_id_types_share_counter() {
        let mut id_gen = IdGenerator::new();
        let s = id_gen.next_session();
        let w = id_gen.next_window();
        let p = id_gen.next_pane();
        assert_eq!(s.0, 1);
        assert_eq!(w.0, 2);
        assert_eq!(p.0, 3);
    }

    #[test]
    fn current_counter() {
        let mut id_gen = IdGenerator::new();
        assert_eq!(id_gen.current(), 1);
        id_gen.next_session();
        assert_eq!(id_gen.current(), 2);
    }

    #[test]
    fn session_id_ordering() {
        let a = SessionId(1);
        let b = SessionId(2);
        assert!(a < b);
    }

    #[test]
    fn client_id_generation() {
        let mut id_gen = IdGenerator::new();
        let c = id_gen.next_client();
        assert_eq!(c.0, 1);
    }

    #[test]
    fn default_generator() {
        let id_gen = IdGenerator::default();
        assert_eq!(id_gen.current(), 1);
    }

    #[test]
    fn hundred_sessions_all_unique() {
        let mut id_gen = IdGenerator::new();
        let ids: Vec<_> = (0..100).map(|_| id_gen.next_session()).collect();
        let set: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(set.len(), 100);
    }

    #[test]
    fn window_id_ordering() {
        let a = WindowId(1);
        let b = WindowId(2);
        assert!(a < b);
        assert!(b > a);
    }

    #[test]
    fn pane_id_ordering() {
        let a = PaneId(5);
        let b = PaneId(3);
        assert!(b < a);
    }

    #[test]
    fn client_id_ordering() {
        let a = ClientId(1);
        let b = ClientId(1);
        assert_eq!(a, b);
    }

    #[test]
    fn ids_are_copy_clone() {
        let id = SessionId(42);
        let id2 = id;
        let id3 = id.clone();
        assert_eq!(id, id2);
        assert_eq!(id, id3);
    }

    #[test]
    fn generator_sequential_across_types() {
        let mut id_gen = IdGenerator::new();
        let s = id_gen.next_session();
        let w = id_gen.next_window();
        let p = id_gen.next_pane();
        let c = id_gen.next_client();
        assert_eq!(s.0, 1);
        assert_eq!(w.0, 2);
        assert_eq!(p.0, 3);
        assert_eq!(c.0, 4);
        assert_eq!(id_gen.current(), 5);
    }

    #[test]
    fn session_id_debug() {
        let s = SessionId(42);
        assert!(format!("{s:?}").contains("42"));
    }

    #[test]
    fn session_id_hash_consistent() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let a = SessionId(1);
        let b = SessionId(1);
        let mut ha = DefaultHasher::new();
        let mut hb = DefaultHasher::new();
        a.hash(&mut ha);
        b.hash(&mut hb);
        assert_eq!(ha.finish(), hb.finish());
    }
}
