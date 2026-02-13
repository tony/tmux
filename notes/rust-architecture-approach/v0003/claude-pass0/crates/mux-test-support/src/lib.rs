//! # mux-test-support
//!
//! Shared test harness and fixtures for TermForge integration tests.
//!
//! L4 integration crate.

#![forbid(unsafe_code)]

use mux_kernel::Kernel;
use mux_time::Clock;
use mux_types::Size;

/// A pre-configured test server for integration tests.
///
/// Uses `Clock::manual()` for deterministic testing.
pub struct TestServer {
    /// The kernel instance.
    pub kernel: Kernel,
    /// The manual clock for time control.
    pub clock: Clock,
}

impl TestServer {
    /// Create a new test server with default settings.
    #[must_use]
    pub fn new() -> Self {
        let clock = Clock::manual();
        let kernel = Kernel::new(clock.clone());
        Self { kernel, clock }
    }

    /// Create a test server with a session already created.
    #[must_use]
    pub fn with_session(name: &str) -> Self {
        let mut server = Self::new();
        server.kernel.create_session(name, Size::new(80, 24));
        server
    }

    /// Generate a unique session name using UUID.
    #[must_use]
    pub fn unique_session_name() -> String {
        format!("test-{}", uuid::Uuid::new_v4())
    }
}

impl Default for TestServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_new() {
        let server = TestServer::new();
        assert_eq!(server.kernel.session_count(), 0);
        assert!(server.clock.is_manual());
    }

    #[test]
    fn test_server_with_session() {
        let server = TestServer::with_session("test");
        assert_eq!(server.kernel.session_count(), 1);
    }

    #[test]
    fn unique_session_names() {
        let a = TestServer::unique_session_name();
        let b = TestServer::unique_session_name();
        assert_ne!(a, b);
    }

    #[test]
    fn test_server_default() {
        let server = TestServer::default();
        assert_eq!(server.kernel.session_count(), 0);
    }
}
