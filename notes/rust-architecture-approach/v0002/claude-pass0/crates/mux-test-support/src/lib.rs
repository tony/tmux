//! # mux-test-support
//!
//! Shared test harness, fixtures, assertion helpers, and snapshot testing.
//!
//! Inspired by libtmux's pytest plugin and vibe-tmux's mux-test-support.
//!
//! ## Fixture Hierarchy (matching libtmux)
//! - `TestServer` -> `TestSession` -> `TestWindow` -> `TestPane`
//! - Each level provides isolated, cleanupable resources.
//!
//! ## Key APIs
//! - `test_kernel()` -- create a Kernel with manual clock for deterministic tests
//! - `test_termlet()` -- create a Termlet for snapshot testing
//! - `assert_grid_eq!()` -- compare grid content with expected text
//! - `assert_snapshot!()` -- insta-based snapshot assertion

#![forbid(unsafe_code)]

use mux_kernel::Kernel;
use mux_time::Clock;
use mux_types::{ClientId, Size};

/// Create a test kernel with a manual clock.
///
/// The clock starts at epoch 0 for deterministic testing.
#[must_use]
pub fn test_kernel() -> Kernel {
    Kernel::new(Clock::manual(0, 1_000_000_000))
}

/// Create a test kernel with a session already created.
///
/// Returns the kernel and the session name.
pub fn test_kernel_with_session(name: &str) -> (Kernel, String) {
    let mut kernel = test_kernel();
    let _ = kernel.process_event(mux_kernel::KernelEvent::Command {
        client: ClientId(0),
        command: format!("new-session -d -s {name}"),
    });
    (kernel, name.to_owned())
}

/// Generate a unique test session name.
#[must_use]
pub fn unique_session_name() -> String {
    format!("termforge_test_{}", uuid::Uuid::new_v4().as_simple())
}

/// Generate a unique socket name for test isolation.
#[must_use]
pub fn unique_socket_name() -> String {
    format!("termforge_test_{}", uuid::Uuid::new_v4().as_simple())
}

/// A test server that cleans up after itself.
///
/// Inspired by libtmux's TestServer fixture.
pub struct TestServer {
    kernel: Kernel,
    socket_name: String,
}

impl TestServer {
    /// Create a new isolated test server.
    #[must_use]
    pub fn new() -> Self {
        Self {
            kernel: test_kernel(),
            socket_name: unique_socket_name(),
        }
    }

    /// Get a reference to the kernel.
    #[must_use]
    pub fn kernel(&self) -> &Kernel {
        &self.kernel
    }

    /// Get a mutable reference to the kernel.
    pub fn kernel_mut(&mut self) -> &mut Kernel {
        &mut self.kernel
    }

    /// The socket name for this test server.
    #[must_use]
    pub fn socket_name(&self) -> &str {
        &self.socket_name
    }

    /// Create a new session on this server.
    pub fn new_session(&mut self, name: &str) -> String {
        let _ = self.kernel.process_event(mux_kernel::KernelEvent::Command {
            client: ClientId(0),
            command: format!("new-session -d -s {name}"),
        });
        name.to_owned()
    }
}

impl Default for TestServer {
    fn default() -> Self {
        Self::new()
    }
}

/// Assert that a grid contains expected text at specific positions.
///
/// ```rust,ignore
/// assert_grid_text!(grid, arena, [
///     (0, "hello world"),
///     (1, "line two"),
/// ]);
/// ```
#[macro_export]
macro_rules! assert_grid_text {
    ($grid:expr, $arena:expr, [ $( ($row:expr, $expected:expr) ),* $(,)? ]) => {
        $(
            {
                let snapshot = $grid.snapshot(0);
                let text = $crate::extract_line_text(&snapshot, &$arena, $row);
                assert_eq!(
                    text.trim_end(),
                    $expected,
                    "Grid line {} mismatch",
                    $row,
                );
            }
        )*
    };
}

/// Extract text from a single line of a grid snapshot.
#[must_use]
pub fn extract_line_text(
    snapshot: &mux_grid::GridSnapshot,
    arena: &mux_grapheme_arena::GraphemeArena,
    row: usize,
) -> String {
    let all_text = mux_snapshot::extract_text(snapshot, arena);
    all_text
        .lines
        .get(row)
        .cloned()
        .unwrap_or_default()
}

/// Standard terminal sizes for testing.
pub mod sizes {
    use mux_types::Size;

    /// Standard 80x24 terminal.
    pub const STANDARD: Size = Size { cols: 80, rows: 24 };
    /// Wide terminal.
    pub const WIDE: Size = Size { cols: 200, rows: 50 };
    /// Narrow terminal.
    pub const NARROW: Size = Size { cols: 40, rows: 12 };
    /// Minimum viable terminal.
    pub const MINIMUM: Size = Size { cols: 2, rows: 2 };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_creates_session() {
        let mut server = TestServer::new();
        server.new_session("test_session");
        assert_eq!(server.kernel().session_count(), 1);
    }

    #[test]
    fn unique_names_are_unique() {
        let a = unique_session_name();
        let b = unique_session_name();
        assert_ne!(a, b);
    }
}
