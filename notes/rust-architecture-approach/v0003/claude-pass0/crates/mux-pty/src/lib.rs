//! # mux-pty
//!
//! PTY allocation, raw mode guard, and signal bridge.
//! This is one of two crates allowed to use `unsafe` (with mux-ffi).
//!
//! ## Gap-1 (TIOCGPTPEER)
//! On Linux 4.13+, `TIOCGPTPEER` provides race-free PTY allocation by
//! obtaining the slave fd directly from the master fd without needing to
//! call `ptsname()` and `open()`. This crate documents support for
//! `TIOCGPTPEER` but falls back to the traditional `openpty()` path
//! on older kernels.
//!
//! ## Gap-2 (Raw Mode Lifecycle)
//! Terminal raw mode is managed via `RawModeGuard` with RAII semantics.
//! The `TerminalModePolicy` enum controls whether TermForge manages
//! raw mode, defers to the caller, or uses save/restore.
//!
//! ## Gap-4 (crossterm rejection rationale)
//! TermForge uses nix + libc for direct terminal control rather than
//! crossterm because:
//! - crossterm abstracts away platform details needed for tmux compatibility
//! - crossterm's event model doesn't match the pipe-based signal architecture
//! - Direct ioctl access is required for TIOCGWINSZ, TIOCGPTPEER, etc.
//! - The parser is custom for tmux behavioral compatibility
//!
//! ## Gap-5 (signal-hook)
//! signal-hook is used (not tokio signals) because:
//! - Pipe-based signal delivery works with the Sans-IO kernel model
//! - signal-hook is runtime-agnostic (works with both std and tokio)
//! - The pipe fd can be registered with tokio::io::AsyncFd for async notification

// Note: This crate is allowed to use unsafe for PTY operations.
// In the scaffold, we only define safe abstractions; actual unsafe
// implementations come in the real implementation phase.

use thiserror::Error;

/// PTY errors.
#[derive(Debug, Error)]
pub enum PtyError {
    /// Failed to allocate PTY.
    #[error("pty allocation failed: {0}")]
    AllocationFailed(String),
    /// Failed to set terminal mode.
    #[error("terminal mode error: {0}")]
    TerminalMode(String),
    /// I/O error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Terminal mode policy (Gap-2).
///
/// Controls how TermForge manages terminal raw mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalModePolicy {
    /// TermForge calls cfmakeraw/tcsetattr. Use for standalone server.
    Managed,
    /// Caller manages raw mode. Use when embedded in a terminal emulator.
    External,
    /// TermForge queries and saves/restores terminal state.
    /// Use when embedding context is unknown.
    SaveRestore,
}

/// RAII guard for terminal raw mode (Gap-2).
///
/// When dropped, restores the terminal to its previous state.
/// This prevents terminal corruption if the process crashes.
#[derive(Debug)]
pub struct RawModeGuard {
    /// The policy governing this guard's behavior.
    policy: TerminalModePolicy,
    /// Whether raw mode is currently active.
    active: bool,
}

impl RawModeGuard {
    /// Create a new guard (would enter raw mode in real implementation).
    ///
    /// # Errors
    ///
    /// Returns `PtyError::TerminalMode` if raw mode cannot be set.
    pub fn new(policy: TerminalModePolicy) -> Result<Self, PtyError> {
        match policy {
            TerminalModePolicy::External => {
                // External: caller manages raw mode, we do nothing
                Ok(Self {
                    policy,
                    active: false,
                })
            }
            TerminalModePolicy::Managed | TerminalModePolicy::SaveRestore => {
                // In real implementation: save termios, call cfmakeraw, tcsetattr
                Ok(Self {
                    policy,
                    active: true,
                })
            }
        }
    }

    /// Whether raw mode is currently active.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.active
    }

    /// The policy for this guard.
    #[must_use]
    pub const fn policy(&self) -> TerminalModePolicy {
        self.policy
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        if self.active {
            // In real implementation: restore saved termios via tcsetattr
            self.active = false;
        }
    }
}

/// PTY master/slave pair (scaffold representation).
#[derive(Debug)]
pub struct PtyPair {
    /// Master fd (would be RawFd in real implementation).
    pub master_fd: i32,
    /// Slave fd (would be RawFd in real implementation).
    pub slave_fd: i32,
}

/// Describes PTY allocation strategy (Gap-1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtyAllocStrategy {
    /// Traditional openpty() path.
    Traditional,
    /// Race-free TIOCGPTPEER (Linux 4.13+).
    Tiocgptpeer,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_mode_guard_managed() {
        let guard = RawModeGuard::new(TerminalModePolicy::Managed);
        assert!(guard.is_ok());
        let guard = guard.unwrap_or_else(|_| RawModeGuard {
            policy: TerminalModePolicy::Managed,
            active: false,
        });
        assert!(guard.is_active());
        assert_eq!(guard.policy(), TerminalModePolicy::Managed);
    }

    #[test]
    fn raw_mode_guard_external() {
        let guard = RawModeGuard::new(TerminalModePolicy::External);
        assert!(guard.is_ok());
        let guard = guard.unwrap_or_else(|_| RawModeGuard {
            policy: TerminalModePolicy::External,
            active: false,
        });
        assert!(!guard.is_active());
    }

    #[test]
    fn raw_mode_guard_save_restore() {
        let guard = RawModeGuard::new(TerminalModePolicy::SaveRestore);
        assert!(guard.is_ok());
    }

    #[test]
    fn terminal_mode_policy_equality() {
        assert_eq!(TerminalModePolicy::Managed, TerminalModePolicy::Managed);
        assert_ne!(TerminalModePolicy::Managed, TerminalModePolicy::External);
    }

    #[test]
    fn pty_alloc_strategy_variants() {
        assert_ne!(PtyAllocStrategy::Traditional, PtyAllocStrategy::Tiocgptpeer);
    }
}
