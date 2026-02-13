//! Strongly-typed entity identifiers.
//!
//! Each entity in the TermForge model (session, window, pane, client) has a
//! unique integer identifier wrapped in a newtype for type safety. This
//! prevents accidentally passing a `PaneId` where a `WindowId` is expected.

/// Strongly typed session identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SessionId(pub u64);

/// Strongly typed window identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindowId(pub u64);

/// Strongly typed pane identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PaneId(pub u64);

/// Strongly typed client identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClientId(pub u64);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_id_equality() {
        assert_eq!(SessionId(1), SessionId(1));
        assert_ne!(SessionId(1), SessionId(2));
    }

    #[test]
    fn window_id_equality() {
        assert_eq!(WindowId(42), WindowId(42));
    }

    #[test]
    fn pane_id_equality() {
        assert_eq!(PaneId(0), PaneId(0));
    }

    #[test]
    fn client_id_equality() {
        assert_eq!(ClientId(100), ClientId(100));
    }

    #[test]
    fn ids_are_copy() {
        let id = PaneId(5);
        let id2 = id;
        assert_eq!(id, id2);
    }
}
