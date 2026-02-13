//! # mux-test-support
//!
//! Shared test harness, fixtures, and assertion helpers.

#![forbid(unsafe_code)]

use mux_kernel::Kernel;
use mux_time::Clock;
use mux_types::{ClientId, Size};

#[must_use]
pub fn test_kernel() -> Kernel {
    Kernel::new(Clock::manual(0, 1_000_000_000))
}

pub fn test_kernel_with_session(name: &str) -> (Kernel, String) {
    let mut kernel = test_kernel();
    let _ = kernel.process_event(mux_kernel::KernelEvent::Command {
        client: ClientId(0),
        command: format!("new-session -d -s {name}"),
    });
    (kernel, name.to_owned())
}

#[must_use]
pub fn unique_session_name() -> String {
    format!("termforge_test_{}", uuid::Uuid::new_v4().as_simple())
}

#[must_use]
pub fn unique_socket_name() -> String {
    format!("termforge_test_{}", uuid::Uuid::new_v4().as_simple())
}

pub struct TestServer {
    kernel: Kernel,
    socket_name: String,
}

impl TestServer {
    #[must_use]
    pub fn new() -> Self {
        Self {
            kernel: test_kernel(),
            socket_name: unique_socket_name(),
        }
    }

    #[must_use]
    pub fn kernel(&self) -> &Kernel { &self.kernel }

    pub fn kernel_mut(&mut self) -> &mut Kernel { &mut self.kernel }

    #[must_use]
    pub fn socket_name(&self) -> &str { &self.socket_name }

    pub fn new_session(&mut self, name: &str) -> String {
        let _ = self.kernel.process_event(mux_kernel::KernelEvent::Command {
            client: ClientId(0),
            command: format!("new-session -d -s {name}"),
        });
        name.to_owned()
    }
}

impl Default for TestServer {
    fn default() -> Self { Self::new() }
}

#[macro_export]
macro_rules! assert_grid_text {
    ($grid:expr, $arena:expr, [ $( ($row:expr, $expected:expr) ),* $(,)? ]) => {
        $( {
            let snapshot = $grid.snapshot(0);
            let text = $crate::extract_line_text(&snapshot, &$arena, $row);
            assert_eq!(text.trim_end(), $expected, "Grid line {} mismatch", $row);
        } )*
    };
}

#[must_use]
pub fn extract_line_text(
    snapshot: &mux_grid::GridSnapshot,
    arena: &mux_grapheme_arena::GraphemeArena,
    row: usize,
) -> String {
    mux_snapshot::extract_text(snapshot, arena)
        .lines
        .get(row)
        .cloned()
        .unwrap_or_default()
}

pub mod sizes {
    use mux_types::Size;
    pub const STANDARD: Size = Size { cols: 80, rows: 24 };
    pub const WIDE: Size = Size { cols: 200, rows: 50 };
    pub const NARROW: Size = Size { cols: 40, rows: 12 };
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
        assert_ne!(unique_session_name(), unique_session_name());
    }
}
