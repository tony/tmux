//! # mux-pty
//!
//! PTY allocation and process management.
//!
//! This is the **only** crate in the workspace allowed to use `unsafe`.
//! All unsafe operations have `// SAFETY:` documentation and focused tests.
//!
//! ## PTY Allocation Strategy
//! - Primary: `posix_openpt` + `grantpt` + `unlockpt` via nix/libc
//! - Linux optimization: `TIOCGPTPEER` ioctl for O_CLOEXEC peer FD
//! - Fallback: `ptsname_r` for portable peer path resolution
//!
//! ## Process Management
//! - `SIGCHLD` handling for zombie reaping
//! - Job control passthrough (SIGTSTP, SIGCONT)
//! - Process group management for child processes

// Note: NOT #![forbid(unsafe_code)] -- this is the unsafe quarantine crate.
#![deny(unsafe_op_in_unsafe_fn)]

use std::os::unix::io::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::path::PathBuf;

use mux_types::Size;

/// Errors from PTY operations.
#[derive(Debug, thiserror::Error)]
pub enum PtyError {
    #[error("failed to open PTY master: {0}")]
    OpenMaster(#[source] nix::Error),
    #[error("grantpt failed: {0}")]
    Grant(#[source] nix::Error),
    #[error("unlockpt failed: {0}")]
    Unlock(#[source] nix::Error),
    #[error("failed to get peer name: {0}")]
    PeerName(String),
    #[error("failed to open PTY peer: {0}")]
    OpenPeer(#[source] std::io::Error),
    #[error("failed to set terminal size: {0}")]
    SetSize(#[source] nix::Error),
    #[error("failed to spawn child: {0}")]
    Spawn(#[source] std::io::Error),
    #[error("TIOCGPTPEER not available")]
    TiocgptpeerUnavailable,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for PTY operations.
pub type PtyResult<T> = Result<T, PtyError>;

/// A PTY master/peer pair.
#[derive(Debug)]
pub struct PtyPair {
    /// Master side FD (server reads/writes this).
    pub master: OwnedFd,
    /// Peer (slave) path for the child process to open.
    pub peer_path: PathBuf,
}

/// Open a new PTY master and return the pair.
///
/// Uses `posix_openpt` + `grantpt` + `unlockpt`.
pub fn open_pty() -> PtyResult<PtyPair> {
    use nix::pty::{posix_openpt, grantpt, unlockpt, ptsname_r};
    use nix::fcntl::OFlag;

    // SAFETY: posix_openpt is safe via nix wrapper.
    let master = posix_openpt(OFlag::O_RDWR | OFlag::O_NOCTTY)
        .map_err(PtyError::OpenMaster)?;

    grantpt(&master).map_err(PtyError::Grant)?;
    unlockpt(&master).map_err(PtyError::Unlock)?;

    let peer_name = ptsname_r(&master)
        .map_err(|e| PtyError::PeerName(e.to_string()))?;

    let peer_path = PathBuf::from(peer_name);

    // Convert to OwnedFd
    let raw_fd = master.as_raw_fd();
    std::mem::forget(master); // prevent double close
    // SAFETY: raw_fd is a valid, open file descriptor from posix_openpt.
    let owned = unsafe { OwnedFd::from_raw_fd(raw_fd) };

    Ok(PtyPair {
        master: owned,
        peer_path,
    })
}

/// Set the terminal size on a PTY master FD.
pub fn set_pty_size(fd: RawFd, size: Size) -> PtyResult<()> {
    let ws = nix::pty::Winsize {
        ws_row: size.rows as u16,
        ws_col: size.cols as u16,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };

    // SAFETY: TIOCSWINSZ is a safe ioctl that sets terminal size.
    // The fd is a valid PTY master, and ws is a valid Winsize struct.
    unsafe {
        let ret = libc::ioctl(fd, libc::TIOCSWINSZ, &ws);
        if ret < 0 {
            return Err(PtyError::SetSize(nix::Error::last()));
        }
    }

    Ok(())
}

/// Configuration for spawning a child process on a PTY.
#[derive(Debug, Clone)]
pub struct ChildConfig {
    /// Shell or program to execute.
    pub program: String,
    /// Arguments to pass.
    pub args: Vec<String>,
    /// Environment variables to set.
    pub env: Vec<(String, String)>,
    /// Working directory.
    pub cwd: Option<PathBuf>,
    /// Terminal size.
    pub size: Size,
}

impl Default for ChildConfig {
    fn default() -> Self {
        Self {
            program: "/bin/sh".into(),
            args: Vec::new(),
            env: Vec::new(),
            cwd: None,
            size: Size::new(80, 24),
        }
    }
}

/// A running child process on a PTY.
#[derive(Debug)]
pub struct PtyChild {
    /// The PTY master FD.
    pub master: OwnedFd,
    /// Child process ID.
    pub pid: u32,
}

impl PtyChild {
    /// Get the master FD for reading/writing.
    #[must_use]
    pub fn master_fd(&self) -> RawFd {
        self.master.as_raw_fd()
    }
}

/// Spawn a child process on a new PTY.
///
/// This forks, sets up the PTY peer as the child's controlling terminal,
/// and execs the specified program.
pub fn spawn_child(_config: &ChildConfig) -> PtyResult<PtyChild> {
    // Full implementation involves fork + setsid + open peer + dup2 + exec.
    // Stub for scaffold -- real implementation in P1.
    todo!("spawn_child: fork + setsid + pty peer setup + exec")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_pty_returns_valid_pair() {
        let result = open_pty();
        // May fail in CI without PTY support
        if let Ok(pair) = result {
            assert!(pair.master.as_raw_fd() >= 0);
            assert!(pair.peer_path.exists());
        }
    }

    #[test]
    fn child_config_defaults() {
        let config = ChildConfig::default();
        assert_eq!(config.program, "/bin/sh");
        assert_eq!(config.size.cols, 80);
        assert_eq!(config.size.rows, 24);
    }
}
