//! # mux-pty
//!
//! PTY allocation, raw mode guard, and signal bridge.
//! This is one of two crates allowed to use `unsafe` (with mux-ffi).

// Note: This crate is allowed to use unsafe for PTY operations.
// In the scaffold, we only define safe abstractions.

use thiserror::Error;

/// PTY errors.
#[derive(Debug, Error)]
pub enum PtyError {
    #[error("pty allocation failed: {0}")]
    AllocationFailed(String),
    #[error("terminal mode error: {0}")]
    TerminalMode(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Terminal mode policy (Gap-2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalModePolicy {
    Managed,
    External,
    SaveRestore,
}

/// RAII guard for terminal raw mode (Gap-2).
#[derive(Debug)]
pub struct RawModeGuard {
    policy: TerminalModePolicy,
    active: bool,
}

impl RawModeGuard {
    /// Create a new guard.
    pub fn new(policy: TerminalModePolicy) -> Result<Self, PtyError> {
        match policy {
            TerminalModePolicy::External => Ok(Self {
                policy,
                active: false,
            }),
            TerminalModePolicy::Managed | TerminalModePolicy::SaveRestore => Ok(Self {
                policy,
                active: true,
            }),
        }
    }

    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.active
    }

    #[must_use]
    pub const fn policy(&self) -> TerminalModePolicy {
        self.policy
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        if self.active {
            self.active = false;
        }
    }
}

/// PTY master/slave pair (scaffold representation).
#[derive(Debug)]
pub struct PtyPair {
    pub master_fd: i32,
    pub slave_fd: i32,
}

/// PTY allocation strategy (Gap-1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtyAllocStrategy {
    Traditional,
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

    #[test]
    fn raw_mode_guard_drop() {
        let guard = RawModeGuard::new(TerminalModePolicy::Managed);
        assert!(guard.is_ok());
        let guard = guard.unwrap_or_else(|_| RawModeGuard {
            policy: TerminalModePolicy::Managed,
            active: false,
        });
        assert!(guard.is_active());
        drop(guard);
        // Guard should have deactivated on drop (no panic).
    }

    #[test]
    fn terminal_mode_policy_debug() {
        let policy = TerminalModePolicy::SaveRestore;
        let debug = format!("{policy:?}");
        assert!(debug.contains("SaveRestore"));
    }

    #[test]
    fn pty_pair_debug() {
        let pair = PtyPair {
            master_fd: 3,
            slave_fd: 4,
        };
        let debug = format!("{pair:?}");
        assert!(debug.contains("3"));
        assert!(debug.contains("4"));
    }

    #[test]
    fn pty_alloc_strategy_clone() {
        let a = PtyAllocStrategy::Tiocgptpeer;
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn pty_alloc_strategy_traditional() {
        let strat = PtyAllocStrategy::Traditional;
        assert_eq!(strat, PtyAllocStrategy::Traditional);
        assert_ne!(strat, PtyAllocStrategy::Tiocgptpeer);
    }

    #[test]
    fn terminal_mode_managed_vs_external() {
        let managed = TerminalModePolicy::Managed;
        let external = TerminalModePolicy::External;
        assert_ne!(managed, external);
    }

    #[test]
    fn pty_pair_fds_distinct() {
        let pair = PtyPair {
            master_fd: 10,
            slave_fd: 11,
        };
        assert_ne!(pair.master_fd, pair.slave_fd);
    }
}
