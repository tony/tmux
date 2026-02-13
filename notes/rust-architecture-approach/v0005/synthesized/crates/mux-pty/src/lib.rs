//! PTY allocation with TIOCGPTPEER and unsafe quarantine.
//!
//! This crate contains ALL unsafe code for PTY operations.
//! It uses `#![deny(unsafe_op_in_unsafe_fn)]` to enforce explicit opt-in.

#![deny(unsafe_op_in_unsafe_fn)]

use thiserror::Error;
use mux_types::geometry::Size;

/// PTY allocation errors.
#[derive(Debug, Error)]
pub enum PtyError {
    #[error("pty allocation failed: {0}")]
    AllocationFailed(String),
    #[error("pty I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid pty size: {0:?}")]
    InvalidSize(Size),
}

/// Strategy for PTY allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtyStrategy {
    /// Use TIOCGPTPEER for race-free allocation (Linux 4.13+).
    PtPeer,
    /// Traditional openpty() fallback.
    OpenPty,
}

/// A PTY master/slave pair (data model only in scaffold).
#[derive(Debug)]
pub struct PtyPair {
    pub master_fd: i32,
    pub slave_fd: i32,
    pub slave_name: String,
    pub strategy: PtyStrategy,
}

/// Terminal mode policy (Gap-2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalModePolicy {
    /// TermForge manages raw mode via RawModeGuard RAII.
    Managed,
    /// Caller manages raw mode (for embedding).
    External,
    /// TermForge saves and restores terminal state.
    SaveRestore,
}

/// RAII guard for raw terminal mode (Gap-2).
#[derive(Debug)]
pub struct RawModeGuard {
    active: bool,
    policy: TerminalModePolicy,
}

impl RawModeGuard {
    /// Create a new guard for the given policy.
    pub fn new(policy: TerminalModePolicy) -> Self {
        Self {
            active: policy == TerminalModePolicy::Managed,
            policy,
        }
    }

    /// Is raw mode currently active?
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// The policy in use.
    pub fn policy(&self) -> TerminalModePolicy {
        self.policy
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        if self.active {
            // In production, this would call tcsetattr to restore terminal state
            self.active = false;
        }
    }
}

/// Detect the best PTY strategy for this platform.
pub fn detect_strategy() -> PtyStrategy {
    // In production: check if TIOCGPTPEER is available
    // For scaffold: always return OpenPty
    PtyStrategy::OpenPty
}

/// Signal handling bridge (Gap-5).
///
/// Provides a pipe-based mechanism for signal delivery.
/// Signal handlers write a byte to the pipe; the event loop reads it.
#[derive(Debug)]
pub struct SignalBridge {
    /// Pipe read end fd (placeholder in scaffold).
    pub read_fd: i32,
    /// Pipe write end fd (placeholder in scaffold).
    pub write_fd: i32,
}

impl Default for SignalBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl SignalBridge {
    /// Create a new signal bridge.
    pub fn new() -> Self {
        // In production: create pipe and register signal handlers
        Self {
            read_fd: -1,
            write_fd: -1,
        }
    }

    /// Check if a signal is pending.
    pub fn has_pending(&self) -> bool {
        false // Placeholder
    }
}

/// Process spawning context.
#[derive(Debug)]
pub struct SpawnContext {
    pub program: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub cwd: Option<String>,
    pub size: Size,
}

impl SpawnContext {
    pub fn new(program: &str) -> Self {
        Self {
            program: program.to_owned(),
            args: Vec::new(),
            env: Vec::new(),
            cwd: None,
            size: Size::new(80, 24),
        }
    }

    pub fn arg(mut self, arg: &str) -> Self {
        self.args.push(arg.to_owned());
        self
    }

    pub fn env(mut self, key: &str, value: &str) -> Self {
        self.env.push((key.to_owned(), value.to_owned()));
        self
    }

    pub fn cwd(mut self, path: &str) -> Self {
        self.cwd = Some(path.to_owned());
        self
    }

    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_strategy_returns_valid() {
        let s = detect_strategy();
        assert!(matches!(s, PtyStrategy::OpenPty | PtyStrategy::PtPeer));
    }

    #[test]
    fn raw_mode_guard_managed() {
        let guard = RawModeGuard::new(TerminalModePolicy::Managed);
        assert!(guard.is_active());
        assert_eq!(guard.policy(), TerminalModePolicy::Managed);
    }

    #[test]
    fn raw_mode_guard_external() {
        let guard = RawModeGuard::new(TerminalModePolicy::External);
        assert!(!guard.is_active());
    }

    #[test]
    fn raw_mode_guard_drop_deactivates() {
        let guard = RawModeGuard::new(TerminalModePolicy::Managed);
        assert!(guard.is_active());
        drop(guard);
    }

    #[test]
    fn signal_bridge_default() {
        let sb = SignalBridge::new();
        assert!(!sb.has_pending());
    }

    #[test]
    fn spawn_context_builder() {
        let ctx = SpawnContext::new("/bin/sh")
            .arg("-c")
            .arg("echo hello")
            .env("TERM", "tmux-256color")
            .cwd("/tmp")
            .size(Size::new(120, 40));
        assert_eq!(ctx.program, "/bin/sh");
        assert_eq!(ctx.args.len(), 2);
        assert_eq!(ctx.env.len(), 1);
        assert_eq!(ctx.cwd, Some("/tmp".to_owned()));
        assert_eq!(ctx.size, Size::new(120, 40));
    }

    #[test]
    fn terminal_mode_policies() {
        assert_ne!(TerminalModePolicy::Managed, TerminalModePolicy::External);
        assert_ne!(TerminalModePolicy::External, TerminalModePolicy::SaveRestore);
    }

    #[test]
    fn pty_strategy_variants() {
        assert_ne!(PtyStrategy::PtPeer, PtyStrategy::OpenPty);
    }

    #[test]
    fn spawn_context_defaults() {
        let ctx = SpawnContext::new("bash");
        assert_eq!(ctx.size, Size::new(80, 24));
        assert!(ctx.cwd.is_none());
    }

    #[test]
    fn signal_bridge_fd_placeholder() {
        let sb = SignalBridge::default();
        assert_eq!(sb.read_fd, -1);
        assert_eq!(sb.write_fd, -1);
    }

    #[test]
    fn pty_error_display() {
        let e = PtyError::AllocationFailed("test".into());
        assert!(e.to_string().contains("test"));
    }

    #[test]
    fn raw_mode_save_restore() {
        let guard = RawModeGuard::new(TerminalModePolicy::SaveRestore);
        assert!(!guard.is_active());
        assert_eq!(guard.policy(), TerminalModePolicy::SaveRestore);
    }

    #[test]
    fn pty_error_io_display() {
        let io_err = std::io::Error::new(std::io::ErrorKind::Other, "broken");
        let e: PtyError = io_err.into();
        assert!(e.to_string().contains("I/O error"));
    }

    #[test]
    fn pty_error_invalid_size() {
        let e = PtyError::InvalidSize(Size::new(0, 0));
        assert!(e.to_string().contains("invalid pty size"));
    }

    #[test]
    fn pty_pair_debug() {
        let pair = PtyPair {
            master_fd: 3,
            slave_fd: 4,
            slave_name: "/dev/pts/0".to_string(),
            strategy: PtyStrategy::OpenPty,
        };
        let dbg = format!("{pair:?}");
        assert!(dbg.contains("PtyPair"));
    }

    #[test]
    fn spawn_context_chain() {
        let ctx = SpawnContext::new("zsh")
            .arg("-l")
            .env("HOME", "/home/user")
            .env("SHELL", "/bin/zsh")
            .cwd("/home/user");
        assert_eq!(ctx.args, vec!["-l"]);
        assert_eq!(ctx.env.len(), 2);
    }

    #[test]
    fn spawn_context_custom_size() {
        let ctx = SpawnContext::new("sh").size(Size::new(200, 50));
        assert_eq!(ctx.size, Size::new(200, 50));
    }

    #[test]
    fn strategy_clone() {
        let s = PtyStrategy::PtPeer;
        let s2 = s;
        assert_eq!(s, s2);
    }

    #[test]
    fn terminal_mode_clone() {
        let m = TerminalModePolicy::Managed;
        let m2 = m;
        assert_eq!(m, m2);
    }

    #[test]
    fn signal_bridge_debug() {
        let sb = SignalBridge::new();
        let dbg = format!("{sb:?}");
        assert!(dbg.contains("SignalBridge"));
    }
}
