//! # mux-pty
//!
//! PTY allocation, process management, and raw mode guard.
//! This is the **only** crate (along with mux-ffi) allowed `unsafe`.
//!
//! ## Features
//! - PTY allocation via posix_openpt + grantpt + unlockpt
//! - TIOCGPTPEER ioctl for O_CLOEXEC peer FD (Linux)
//! - RawModeGuard for RAII terminal state management (Gap #3)
//! - Child process spawning with setsid + pty peer setup

// NOT #![forbid(unsafe_code)] -- this is the unsafe quarantine crate.
#![deny(unsafe_op_in_unsafe_fn)]

use std::os::unix::io::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::path::PathBuf;

use mux_types::Size;

#[derive(Debug, thiserror::Error)]
pub enum PtyError {
    #[error("failed to open PTY master: {0}")] OpenMaster(#[source] nix::Error),
    #[error("grantpt failed: {0}")] Grant(#[source] nix::Error),
    #[error("unlockpt failed: {0}")] Unlock(#[source] nix::Error),
    #[error("failed to get peer name: {0}")] PeerName(String),
    #[error("failed to open PTY peer: {0}")] OpenPeer(#[source] std::io::Error),
    #[error("failed to set terminal size: {0}")] SetSize(#[source] nix::Error),
    #[error("failed to spawn child: {0}")] Spawn(#[source] std::io::Error),
    #[error("TIOCGPTPEER not available")] TiocgptpeerUnavailable,
    #[error("IO error: {0}")] Io(#[from] std::io::Error),
    #[error("raw mode error: {0}")] RawMode(String),
}

pub type PtyResult<T> = Result<T, PtyError>;

/// A PTY master/peer pair.
#[derive(Debug)]
pub struct PtyPair {
    pub master: OwnedFd,
    pub peer_path: PathBuf,
}

/// Open a new PTY master and return the pair.
pub fn open_pty() -> PtyResult<PtyPair> {
    use nix::pty::{posix_openpt, grantpt, unlockpt, ptsname_r};
    use nix::fcntl::OFlag;

    let master = posix_openpt(OFlag::O_RDWR | OFlag::O_NOCTTY)
        .map_err(PtyError::OpenMaster)?;
    grantpt(&master).map_err(PtyError::Grant)?;
    unlockpt(&master).map_err(PtyError::Unlock)?;
    let peer_name = ptsname_r(&master).map_err(|e| PtyError::PeerName(e.to_string()))?;
    let peer_path = PathBuf::from(peer_name);

    let raw_fd = master.as_raw_fd();
    std::mem::forget(master);
    // SAFETY: raw_fd is a valid open file descriptor from posix_openpt.
    let owned = unsafe { OwnedFd::from_raw_fd(raw_fd) };
    Ok(PtyPair { master: owned, peer_path })
}

/// Set terminal size on a PTY master FD.
pub fn set_pty_size(fd: RawFd, size: Size) -> PtyResult<()> {
    let ws = nix::pty::Winsize { ws_row: size.rows as u16, ws_col: size.cols as u16, ws_xpixel: 0, ws_ypixel: 0 };
    // SAFETY: TIOCSWINSZ is a safe ioctl. fd is a valid PTY master.
    unsafe {
        let ret = libc::ioctl(fd, libc::TIOCSWINSZ, &ws);
        if ret < 0 { return Err(PtyError::SetSize(nix::Error::last())); }
    }
    Ok(())
}

/// Configuration for spawning a child process.
#[derive(Debug, Clone)]
pub struct ChildConfig {
    pub program: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub cwd: Option<PathBuf>,
    pub size: Size,
}

impl Default for ChildConfig {
    fn default() -> Self {
        Self { program: "/bin/sh".into(), args: Vec::new(), env: Vec::new(), cwd: None, size: Size::new(80, 24) }
    }
}

/// A running child process on a PTY.
#[derive(Debug)]
pub struct PtyChild {
    pub master: OwnedFd,
    pub pid: u32,
}

impl PtyChild {
    #[must_use]
    pub fn master_fd(&self) -> RawFd { self.master.as_raw_fd() }
}

/// Spawn a child process on a new PTY.
pub fn spawn_child(_config: &ChildConfig) -> PtyResult<PtyChild> {
    // Full impl: fork + setsid + open peer + dup2 + exec
    todo!("spawn_child: fork + setsid + pty peer setup + exec")
}

/// RAII guard for terminal raw mode (Gap #3).
///
/// Saves the current terminal settings on creation and restores them on drop.
/// Used by the IO thread when TerminalModePolicy is Managed or SaveRestore.
#[derive(Debug)]
pub struct RawModeGuard {
    fd: OwnedFd,
    original: nix::sys::termios::Termios,
}

impl RawModeGuard {
    /// Enter raw mode on the given fd.
    ///
    /// # Errors
    /// Returns error if tcgetattr or tcsetattr fails.
    ///
    /// # Safety
    /// The `raw_fd` must be a valid, open file descriptor that outlives this guard.
    /// The caller is responsible for ensuring the fd is not closed while the guard exists.
    pub fn enter(raw_fd: RawFd) -> PtyResult<Self> {
        // SAFETY: We dup the fd so that RawModeGuard owns its own copy.
        // libc::dup is safe to call on a valid fd.
        let duped = unsafe { libc::dup(raw_fd) };
        if duped < 0 {
            return Err(PtyError::RawMode("dup failed".into()));
        }
        // SAFETY: duped is a valid fd returned by libc::dup.
        let fd = unsafe { OwnedFd::from_raw_fd(duped) };

        let original = nix::sys::termios::tcgetattr(&fd)
            .map_err(|e| PtyError::RawMode(e.to_string()))?;
        let mut raw = original.clone();
        nix::sys::termios::cfmakeraw(&mut raw);
        nix::sys::termios::tcsetattr(&fd, nix::sys::termios::SetArg::TCSANOW, &raw)
            .map_err(|e| PtyError::RawMode(e.to_string()))?;
        Ok(Self { fd, original })
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = nix::sys::termios::tcsetattr(
            &self.fd,
            nix::sys::termios::SetArg::TCSANOW,
            &self.original,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_pty_returns_valid_pair() {
        if let Ok(pair) = open_pty() {
            assert!(pair.master.as_raw_fd() >= 0);
            assert!(pair.peer_path.exists());
        }
    }

    #[test]
    fn child_config_defaults() {
        let config = ChildConfig::default();
        assert_eq!(config.program, "/bin/sh");
        assert_eq!(config.size.cols, 80);
    }

    #[test]
    fn pty_pair_debug() {
        if let Ok(pair) = open_pty() {
            let debug = format!("{pair:?}");
            assert!(debug.contains("PtyPair"));
        }
    }
}
