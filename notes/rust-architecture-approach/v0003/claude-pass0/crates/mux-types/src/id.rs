//! Strongly-typed entity IDs.
//!
//! IDs are monotonically increasing u64 values. They are never reused,
//! which simplifies garbage collection and prevents use-after-free patterns.

/// Macro to define a strongly-typed ID newtype.
macro_rules! define_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(u64);

        impl $name {
            /// Create an ID from a raw u64 value.
            #[must_use]
            pub const fn new(raw: u64) -> Self {
                Self(raw)
            }

            /// Extract the raw u64 value.
            #[must_use]
            pub const fn raw(&self) -> u64 {
                self.0
            }
        }

        impl core::fmt::Display for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "{}({})", stringify!($name), self.0)
            }
        }
    };
}

define_id!(
    /// Unique session identifier.
    SessionId
);

define_id!(
    /// Unique window identifier.
    WindowId
);

define_id!(
    /// Unique pane identifier.
    PaneId
);

define_id!(
    /// Unique client identifier.
    ClientId
);

/// Generator for monotonically increasing entity IDs.
#[derive(Debug)]
pub struct IdGenerator {
    next: u64,
}

impl IdGenerator {
    /// Create a new generator starting at 1.
    #[must_use]
    pub const fn new() -> Self {
        Self { next: 1 }
    }

    /// Generate the next session ID.
    pub fn next_session(&mut self) -> SessionId {
        let id = SessionId::new(self.next);
        self.next += 1;
        id
    }

    /// Generate the next window ID.
    pub fn next_window(&mut self) -> WindowId {
        let id = WindowId::new(self.next);
        self.next += 1;
        id
    }

    /// Generate the next pane ID.
    pub fn next_pane(&mut self) -> PaneId {
        let id = PaneId::new(self.next);
        self.next += 1;
        id
    }

    /// Generate the next client ID.
    pub fn next_client(&mut self) -> ClientId {
        let id = ClientId::new(self.next);
        self.next += 1;
        id
    }
}

impl Default for IdGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_id_display() {
        let id = SessionId::new(42);
        assert_eq!(format!("{id}"), "SessionId(42)");
    }

    #[test]
    fn ids_are_monotonic() {
        let mut id_gen = IdGenerator::new();
        let s1 = id_gen.next_session();
        let w1 = id_gen.next_window();
        let p1 = id_gen.next_pane();
        assert!(s1.raw() < w1.raw());
        assert!(w1.raw() < p1.raw());
    }

    #[test]
    fn id_equality() {
        assert_eq!(PaneId::new(1), PaneId::new(1));
        assert_ne!(PaneId::new(1), PaneId::new(2));
    }

    #[test]
    fn id_ordering() {
        assert!(SessionId::new(1) < SessionId::new(2));
    }

    #[test]
    fn id_generator_starts_at_one() {
        let mut id_gen = IdGenerator::new();
        assert_eq!(id_gen.next_session().raw(), 1);
    }

    #[test]
    fn id_generator_never_repeats() {
        let mut id_gen = IdGenerator::new();
        let ids: Vec<u64> = (0..100).map(|_| id_gen.next_pane().raw()).collect();
        let unique: std::collections::HashSet<u64> = ids.iter().copied().collect();
        assert_eq!(ids.len(), unique.len());
    }

    #[test]
    fn client_id_raw_roundtrip() {
        let id = ClientId::new(999);
        assert_eq!(id.raw(), 999);
    }
}
