//! Test fixtures, TestServer, and isolated socket path helpers.

#![forbid(unsafe_code)]

use mux_kernel::{Kernel, KernelEvent, KernelEffect};
use mux_time::Clock;
use mux_types::geometry::Size;
use mux_types::id::{SessionId, WindowId, PaneId};
use std::path::PathBuf;

/// Generate an isolated socket path for testing.
/// Prevents collision with real tmux sockets.
pub fn isolated_socket_path() -> PathBuf {
    let id = uuid::Uuid::new_v4();
    let dir = std::env::temp_dir().join("termforge-test");
    std::fs::create_dir_all(&dir).unwrap_or(());
    dir.join(format!("test-{id}.sock"))
}

/// RAII cleanup guard for test sockets.
pub struct CleanupGuard {
    path: PathBuf,
}

impl CleanupGuard {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }
}

impl Drop for CleanupGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// A test server wrapping the kernel with convenience methods.
pub struct TestServer {
    pub kernel: Kernel,
    pub primary_session: Option<SessionId>,
    pub primary_window: Option<WindowId>,
    pub primary_pane: Option<PaneId>,
    _guard: Option<CleanupGuard>,
}

impl TestServer {
    /// Create a new test server with a session.
    pub fn new() -> Self {
        let socket_path = isolated_socket_path();
        let guard = CleanupGuard::new(socket_path);
        let mut kernel = Kernel::new(Clock::manual());

        let effects = kernel.process_event(KernelEvent::CreateSession {
            name: "test".into(),
            size: Size::new(80, 24),
        });

        let mut primary_session = None;
        let mut primary_window = None;
        let mut primary_pane = None;

        for effect in &effects {
            if let KernelEffect::SessionCreated {
                session_id,
                window_id,
                pane_id,
            } = effect
            {
                primary_session = Some(*session_id);
                primary_window = Some(*window_id);
                primary_pane = Some(*pane_id);
            }
        }

        Self {
            kernel,
            primary_session,
            primary_window,
            primary_pane,
            _guard: Some(guard),
        }
    }

    /// Create with a specific size.
    pub fn with_size(size: Size) -> Self {
        let socket_path = isolated_socket_path();
        let guard = CleanupGuard::new(socket_path);
        let mut kernel = Kernel::new(Clock::manual());

        let effects = kernel.process_event(KernelEvent::CreateSession {
            name: "test".into(),
            size,
        });

        let mut primary_session = None;
        let mut primary_window = None;
        let mut primary_pane = None;

        for effect in &effects {
            if let KernelEffect::SessionCreated {
                session_id,
                window_id,
                pane_id,
            } = effect
            {
                primary_session = Some(*session_id);
                primary_window = Some(*window_id);
                primary_pane = Some(*pane_id);
            }
        }

        Self {
            kernel,
            primary_session,
            primary_window,
            primary_pane,
            _guard: Some(guard),
        }
    }

    /// Feed data to the primary pane.
    pub fn feed(&mut self, data: &[u8]) {
        if let Some(pane_id) = self.primary_pane {
            self.kernel.process_event(KernelEvent::PtyOutput {
                pane_id,
                data: data.to_vec(),
            });
        }
    }

    /// Get pane text.
    pub fn pane_text(&self, row: u16) -> String {
        if let Some(pane_id) = self.primary_pane {
            if let Some(pane) = self.kernel.panes.get(&pane_id) {
                return pane.grid.line_text(row, &pane.arena);
            }
        }
        String::new()
    }

    /// Get cursor position.
    pub fn cursor(&self) -> (u16, u16) {
        if let Some(pane_id) = self.primary_pane {
            if let Some(grid) = self.kernel.pane_grid(pane_id) {
                return grid.cursor();
            }
        }
        (0, 0)
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
    fn isolated_socket_paths_unique() {
        let p1 = isolated_socket_path();
        let p2 = isolated_socket_path();
        assert_ne!(p1, p2);
    }

    #[test]
    fn cleanup_guard_removes_file() {
        let path = isolated_socket_path();
        std::fs::write(&path, b"test").unwrap_or(());
        assert!(path.exists());
        let guard = CleanupGuard::new(path.clone());
        drop(guard);
        assert!(!path.exists());
    }

    #[test]
    fn test_server_creation() {
        let ts = TestServer::new();
        assert!(ts.primary_session.is_some());
        assert!(ts.primary_window.is_some());
        assert!(ts.primary_pane.is_some());
    }

    #[test]
    fn test_server_feed() {
        let mut ts = TestServer::new();
        ts.feed(b"Hello");
        assert_eq!(ts.pane_text(0), "Hello");
    }

    #[test]
    fn test_server_cursor() {
        let mut ts = TestServer::new();
        ts.feed(b"ABC");
        let (x, _) = ts.cursor();
        assert_eq!(x, 3);
    }

    #[test]
    fn test_server_with_size() {
        let ts = TestServer::with_size(Size::new(120, 40));
        assert!(ts.primary_pane.is_some());
    }

    #[test]
    fn test_server_default() {
        let ts = TestServer::default();
        assert!(ts.primary_pane.is_some());
    }

    #[test]
    fn test_server_multiline() {
        let mut ts = TestServer::new();
        ts.feed(b"Line1\r\nLine2\r\nLine3");
        assert_eq!(ts.pane_text(0), "Line1");
        assert_eq!(ts.pane_text(1), "Line2");
        assert_eq!(ts.pane_text(2), "Line3");
    }

    #[test]
    fn test_server_csi_processing() {
        let mut ts = TestServer::new();
        ts.feed(b"\x1b[2;5HX");
        let (x, y) = ts.cursor();
        assert_eq!(x, 5);
        assert_eq!(y, 1);
    }

    #[test]
    fn cleanup_guard_nonexistent_file() {
        let path = std::env::temp_dir().join("nonexistent-test-socket");
        let guard = CleanupGuard::new(path);
        drop(guard); // Should not panic
    }

    #[test]
    fn socket_path_in_temp_dir() {
        let p = isolated_socket_path();
        assert!(p.to_string_lossy().contains("termforge-test"));
    }

    #[test]
    fn cleanup_guard_path_accessor() {
        let path = isolated_socket_path();
        let guard = CleanupGuard::new(path.clone());
        assert_eq!(guard.path(), path.as_path());
    }

    #[test]
    fn test_server_kernel_access() {
        let ts = TestServer::new();
        assert!(ts.kernel.session_count() > 0);
    }

    #[test]
    fn test_server_empty_feed() {
        let mut ts = TestServer::new();
        ts.feed(b"");
        // Should not panic
    }

    #[test]
    fn test_server_large_feed() {
        let mut ts = TestServer::new();
        let data: Vec<u8> = (0..500).map(|i| (b'A' + (i % 26) as u8)).collect();
        ts.feed(&data);
    }

    #[test]
    fn test_server_escape_sequences() {
        let mut ts = TestServer::new();
        ts.feed(b"\x1b[31mRed\x1b[0m Normal");
        // Should process without panic
    }

    #[test]
    fn test_server_tab_character() {
        let mut ts = TestServer::new();
        ts.feed(b"A\tB");
        let text = ts.pane_text(0);
        assert!(text.contains('A'));
    }

    #[test]
    fn test_server_newline_cr_lf() {
        let mut ts = TestServer::new();
        ts.feed(b"ABC\r\nDEF");
        assert_eq!(ts.pane_text(0), "ABC");
        assert_eq!(ts.pane_text(1), "DEF");
    }

    #[test]
    fn test_server_overwrite() {
        let mut ts = TestServer::new();
        ts.feed(b"XXXX\r");
        ts.feed(b"YY");
        let text = ts.pane_text(0);
        assert!(text.starts_with("YY"));
    }

    #[test]
    fn multiple_test_servers() {
        let _ts1 = TestServer::new();
        let _ts2 = TestServer::new();
        // Both should have unique socket paths
    }

    #[test]
    fn test_server_cursor_initial() {
        let ts = TestServer::new();
        let (x, y) = ts.cursor();
        assert_eq!(x, 0);
        assert_eq!(y, 0);
    }

    #[test]
    fn test_server_pane_text_empty_row() {
        let ts = TestServer::new();
        let text = ts.pane_text(10);
        assert!(text.is_empty() || text.chars().all(|c| c == ' '));
    }
}
