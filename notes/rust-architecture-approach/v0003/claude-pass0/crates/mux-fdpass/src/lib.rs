//! # mux-fdpass
//!
//! SCM_RIGHTS file descriptor passing envelope for Unix domain sockets.
//! Used for PTY master handoff in tmux compatibility mode.
//!
//! ## Constraints
//! - Maximum 16 file descriptors per message (FD_LIMIT).
//! - Only Unix domain sockets support SCM_RIGHTS (not TCP).
//! - Receiving process must validate fd type via fstat.
//!
//! ## Gap-10 (SCM_RIGHTS)
//! This crate provides the data model and envelope types. The actual nix
//! sendmsg/recvmsg calls live in mux-pty (unsafe quarantine) because they
//! require raw fd manipulation.
//!
//! L2 data crate -- no internal dependencies.

#![forbid(unsafe_code)]

use thiserror::Error;

/// Maximum number of file descriptors per SCM_RIGHTS message.
pub const FD_LIMIT: usize = 16;

/// Errors from fd passing operations.
#[derive(Debug, Error)]
pub enum FdPassError {
    /// Too many file descriptors in a single message.
    #[error("fd limit exceeded: {count} > {FD_LIMIT}")]
    TooManyFds { count: usize },
    /// Invalid file descriptor.
    #[error("invalid fd: {0}")]
    InvalidFd(i32),
    /// I/O error during sendmsg/recvmsg.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// An envelope describing file descriptors to be passed via SCM_RIGHTS.
#[derive(Debug, Clone)]
pub struct FdEnvelope {
    /// File descriptor values to pass.
    fds: Vec<i32>,
    /// Optional data payload sent alongside the fds.
    data: Vec<u8>,
}

impl FdEnvelope {
    /// Create a new envelope with the given file descriptors.
    ///
    /// # Errors
    ///
    /// Returns `FdPassError::TooManyFds` if more than `FD_LIMIT` fds are provided.
    pub fn new(fds: Vec<i32>, data: Vec<u8>) -> Result<Self, FdPassError> {
        if fds.len() > FD_LIMIT {
            return Err(FdPassError::TooManyFds { count: fds.len() });
        }
        for &fd in &fds {
            if fd < 0 {
                return Err(FdPassError::InvalidFd(fd));
            }
        }
        Ok(Self { fds, data })
    }

    /// Create an envelope with a single fd.
    ///
    /// # Errors
    ///
    /// Returns `FdPassError::InvalidFd` if the fd is negative.
    pub fn single(fd: i32, data: Vec<u8>) -> Result<Self, FdPassError> {
        Self::new(vec![fd], data)
    }

    /// File descriptors in this envelope.
    #[must_use]
    pub fn fds(&self) -> &[i32] {
        &self.fds
    }

    /// Data payload.
    #[must_use]
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Number of file descriptors.
    #[must_use]
    pub fn fd_count(&self) -> usize {
        self.fds.len()
    }
}

/// Describes the type of a received file descriptor (after fstat validation).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FdType {
    /// PTY master.
    PtyMaster,
    /// PTY slave / peer.
    PtySlave,
    /// Regular file.
    RegularFile,
    /// Unix domain socket.
    Socket,
    /// Unknown/unsupported type.
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fd_limit_constant() {
        assert_eq!(FD_LIMIT, 16);
    }

    #[test]
    fn envelope_single() {
        let env = FdEnvelope::single(3, b"hello".to_vec());
        assert!(env.is_ok());
        let env = env.unwrap_or_else(|_| FdEnvelope { fds: vec![], data: vec![] });
        assert_eq!(env.fd_count(), 1);
        assert_eq!(env.fds()[0], 3);
        assert_eq!(env.data(), b"hello");
    }

    #[test]
    fn envelope_multiple() {
        let fds: Vec<i32> = (0..16).collect();
        let env = FdEnvelope::new(fds, Vec::new());
        assert!(env.is_ok());
    }

    #[test]
    fn envelope_too_many_fds() {
        let fds: Vec<i32> = (0..17).collect();
        let env = FdEnvelope::new(fds, Vec::new());
        assert!(env.is_err());
    }

    #[test]
    fn envelope_negative_fd() {
        let env = FdEnvelope::single(-1, Vec::new());
        assert!(env.is_err());
    }

    #[test]
    fn fd_type_variants() {
        assert_ne!(FdType::PtyMaster, FdType::PtySlave);
        assert_ne!(FdType::Socket, FdType::Unknown);
    }
}
