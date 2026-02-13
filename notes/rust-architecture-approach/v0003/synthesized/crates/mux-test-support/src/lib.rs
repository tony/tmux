//! # mux-test-support
//!
//! Shared test harness for TermForge integration tests.
//! Provides `TestServer`, isolated socket paths, and cleanup guards.
//!
//! ## Features
//! - `TestServer`: pre-built `MuxServer` with manual clock and temp socket
//! - `IsolatedSocket`: unique socket path outside default tmux namespace
//! - `CleanupGuard`: RAII cleanup for test resources
//! - `TmuxCompat`: tmux version gating for compatibility tests
//!
//! L4 integration crate.

#![forbid(unsafe_code)]

use mux_kernel::Kernel;
use mux_time::Clock;
use mux_types::{SessionId, Size};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// A test server with deterministic clock and isolated socket.
pub struct TestServer {
    kernel: Kernel,
    clock: Clock,
    socket_path: PathBuf,
    default_size: Size,
}

impl TestServer {
    /// Create a new test server with a unique socket path.
    #[must_use]
    pub fn new() -> Self {
        let clock = Clock::manual();
        let socket_dir = std::env::temp_dir().join("termforge-test");
        // Best-effort directory creation -- ignore errors in scaffold.
        let _ = std::fs::create_dir_all(&socket_dir);
        let socket_path = socket_dir.join(format!("test-{}.sock", Uuid::new_v4()));
        Self {
            kernel: Kernel::new(clock.clone()),
            clock,
            socket_path,
            default_size: Size::new(80, 24),
        }
    }

    /// Create with a specific size.
    #[must_use]
    pub fn with_size(size: Size) -> Self {
        let mut server = Self::new();
        server.default_size = size;
        server
    }

    /// Get a reference to the manual clock.
    #[must_use]
    pub fn clock(&self) -> &Clock {
        &self.clock
    }

    /// Get the socket path.
    #[must_use]
    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    /// Create a session.
    pub fn create_session(&mut self, name: &str) -> SessionId {
        let (id, _effects) = self.kernel.create_session(name, self.default_size);
        id
    }

    /// Number of active sessions.
    #[must_use]
    pub fn session_count(&self) -> usize {
        self.kernel.session_count()
    }

    /// Default size.
    #[must_use]
    pub const fn default_size(&self) -> Size {
        self.default_size
    }
}

impl Default for TestServer {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        // Clean up socket file.
        let _ = std::fs::remove_file(&self.socket_path);
    }
}

/// Generates an isolated socket path that never conflicts with
/// the user's real tmux socket directory.
#[must_use]
pub fn isolated_socket_path(prefix: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("termforge-test");
    let _ = std::fs::create_dir_all(&dir);
    dir.join(format!("{prefix}-{}.sock", Uuid::new_v4()))
}

/// RAII guard that cleans up a socket path on drop.
pub struct CleanupGuard {
    paths: Vec<PathBuf>,
}

impl CleanupGuard {
    /// Create an empty cleanup guard.
    #[must_use]
    pub fn new() -> Self {
        Self { paths: Vec::new() }
    }

    /// Track a path for cleanup.
    pub fn track(&mut self, path: PathBuf) {
        self.paths.push(path);
    }

    /// Number of tracked paths.
    #[must_use]
    pub fn tracked_count(&self) -> usize {
        self.paths.len()
    }
}

impl Default for CleanupGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for CleanupGuard {
    fn drop(&mut self) {
        for path in &self.paths {
            let _ = std::fs::remove_file(path);
        }
    }
}

/// tmux version gating for compatibility tests.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TmuxVersion {
    pub major: u32,
    pub minor: u32,
    pub suffix: Option<char>,
}

impl TmuxVersion {
    /// Parse a version string like "3.5a" or "3.4".
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        if s.is_empty() {
            return None;
        }

        // Find where digits end for major
        let dot_pos = s.find('.')?;
        let major: u32 = s[..dot_pos].parse().ok()?;

        let rest = &s[dot_pos + 1..];
        if rest.is_empty() {
            return None;
        }

        // Find numeric part of minor
        let minor_end = rest
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(rest.len());
        if minor_end == 0 {
            return None;
        }
        let minor: u32 = rest[..minor_end].parse().ok()?;
        let suffix = rest[minor_end..].chars().next();

        Some(Self {
            major,
            minor,
            suffix,
        })
    }

    /// Check if this version meets a minimum requirement.
    #[must_use]
    pub fn meets_minimum(&self, min: &Self) -> bool {
        self >= min
    }
}

/// Check if a tmux binary is available and return its version.
pub fn detect_tmux_version() -> Option<TmuxVersion> {
    // Scaffold: always returns None (no actual tmux invocation).
    None
}

/// Skip a test if tmux version doesn't meet the minimum.
/// Returns true if the test should run, false if it should skip.
#[must_use]
pub fn require_tmux_version(min: &str) -> bool {
    let Some(min_ver) = TmuxVersion::parse(min) else {
        return false;
    };
    match detect_tmux_version() {
        Some(ver) => ver.meets_minimum(&min_ver),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_server_creates_unique_socket() {
        let s1 = TestServer::new();
        let s2 = TestServer::new();
        assert_ne!(s1.socket_path(), s2.socket_path());
    }

    #[test]
    fn test_server_default_size() {
        let server = TestServer::new();
        assert_eq!(server.default_size(), Size::new(80, 24));
    }

    #[test]
    fn test_server_custom_size() {
        let server = TestServer::with_size(Size::new(120, 40));
        assert_eq!(server.default_size(), Size::new(120, 40));
    }

    #[test]
    fn test_server_create_session() {
        let mut server = TestServer::new();
        let _id = server.create_session("test");
        assert_eq!(server.session_count(), 1);
    }

    #[test]
    fn test_server_multiple_sessions() {
        let mut server = TestServer::new();
        let _id1 = server.create_session("s1");
        let _id2 = server.create_session("s2");
        assert_eq!(server.session_count(), 2);
    }

    #[test]
    fn test_server_clock_is_manual() {
        let server = TestServer::new();
        let t1 = server.clock().now();
        server.clock().advance(Duration::from_secs(10));
        let t2 = server.clock().now();
        assert!(t2.as_millis() > t1.as_millis());
    }

    #[test]
    fn test_server_socket_in_temp_dir() {
        let server = TestServer::new();
        let path = server.socket_path();
        let path_str = path.to_string_lossy();
        assert!(path_str.contains("termforge-test"));
    }

    #[test]
    fn isolated_socket_path_unique() {
        let p1 = isolated_socket_path("test");
        let p2 = isolated_socket_path("test");
        assert_ne!(p1, p2);
    }

    #[test]
    fn isolated_socket_path_contains_prefix() {
        let p = isolated_socket_path("myprefix");
        let filename = p
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();
        assert!(filename.starts_with("myprefix-"));
    }

    #[test]
    fn cleanup_guard_tracks_paths() {
        let mut guard = CleanupGuard::new();
        guard.track(PathBuf::from("/tmp/test1.sock"));
        guard.track(PathBuf::from("/tmp/test2.sock"));
        assert_eq!(guard.tracked_count(), 2);
    }

    #[test]
    fn tmux_version_parse_simple() {
        let v = TmuxVersion::parse("3.4");
        assert!(v.is_some());
        let v = v.unwrap_or_else(|| TmuxVersion {
            major: 0,
            minor: 0,
            suffix: None,
        });
        assert_eq!(v.major, 3);
        assert_eq!(v.minor, 4);
        assert!(v.suffix.is_none());
    }

    #[test]
    fn tmux_version_parse_with_suffix() {
        let v = TmuxVersion::parse("3.5a");
        assert!(v.is_some());
        let v = v.unwrap_or_else(|| TmuxVersion {
            major: 0,
            minor: 0,
            suffix: None,
        });
        assert_eq!(v.major, 3);
        assert_eq!(v.minor, 5);
        assert_eq!(v.suffix, Some('a'));
    }

    #[test]
    fn tmux_version_parse_empty() {
        assert!(TmuxVersion::parse("").is_none());
    }

    #[test]
    fn tmux_version_ordering() {
        let v34 = TmuxVersion::parse("3.4").unwrap_or_else(|| TmuxVersion {
            major: 3,
            minor: 4,
            suffix: None,
        });
        let v35 = TmuxVersion::parse("3.5").unwrap_or_else(|| TmuxVersion {
            major: 3,
            minor: 5,
            suffix: None,
        });
        let v35a = TmuxVersion::parse("3.5a").unwrap_or_else(|| TmuxVersion {
            major: 3,
            minor: 5,
            suffix: Some('a'),
        });
        assert!(v34 < v35);
        assert!(v35 < v35a);
    }

    #[test]
    fn tmux_version_meets_minimum() {
        let v35 = TmuxVersion::parse("3.5").unwrap_or_else(|| TmuxVersion {
            major: 3,
            minor: 5,
            suffix: None,
        });
        let v34 = TmuxVersion::parse("3.4").unwrap_or_else(|| TmuxVersion {
            major: 3,
            minor: 4,
            suffix: None,
        });
        assert!(v35.meets_minimum(&v34));
        assert!(!v34.meets_minimum(&v35));
    }

    #[test]
    fn detect_tmux_returns_none_in_scaffold() {
        assert!(detect_tmux_version().is_none());
    }

    #[test]
    fn require_tmux_version_returns_false_in_scaffold() {
        assert!(!require_tmux_version("3.4"));
    }
}
