//! # mux-fdpass
//!
//! `SCM_RIGHTS` fd-passing envelope for PTY master handoff (Gap-10).
//! Defines the data model only -- actual sendmsg/recvmsg in mux-pty.
//!
//! L2 data crate.

#![forbid(unsafe_code)]

use thiserror::Error;

/// Maximum file descriptors per message (tmux compat).
pub const MAX_FDS: usize = 16;

/// Fd-pass errors.
#[derive(Debug, Error)]
pub enum FdPassError {
    /// Too many file descriptors.
    #[error("too many fds: {count} (max {MAX_FDS})")]
    TooManyFds { count: usize },
    /// Invalid fd value.
    #[error("invalid fd: {0}")]
    InvalidFd(i32),
}

/// An envelope for passing file descriptors via `SCM_RIGHTS`.
#[derive(Debug, Clone)]
pub struct FdEnvelope {
    /// File descriptor values to pass.
    fds: Vec<i32>,
    /// Optional ancillary data.
    data: Vec<u8>,
}

impl FdEnvelope {
    /// Create a new empty envelope.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            fds: Vec::new(),
            data: Vec::new(),
        }
    }

    /// Add a file descriptor to the envelope.
    pub fn push_fd(&mut self, fd: i32) -> Result<(), FdPassError> {
        if fd < 0 {
            return Err(FdPassError::InvalidFd(fd));
        }
        if self.fds.len() >= MAX_FDS {
            return Err(FdPassError::TooManyFds {
                count: self.fds.len() + 1,
            });
        }
        self.fds.push(fd);
        Ok(())
    }

    /// Set ancillary data.
    pub fn set_data(&mut self, data: Vec<u8>) {
        self.data = data;
    }

    /// Get the file descriptors.
    #[must_use]
    pub fn fds(&self) -> &[i32] {
        &self.fds
    }

    /// Get the ancillary data.
    #[must_use]
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Number of file descriptors in the envelope.
    #[must_use]
    pub fn fd_count(&self) -> usize {
        self.fds.len()
    }
}

impl Default for FdEnvelope {
    fn default() -> Self {
        Self::new()
    }
}

/// Classification of a received file descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FdType {
    /// PTY master fd.
    PtyMaster,
    /// PTY slave/peer fd.
    PtySlave,
    /// Regular file.
    RegularFile,
    /// Unix domain socket.
    Socket,
    /// Unrecognized fd type.
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_envelope() {
        let e = FdEnvelope::new();
        assert_eq!(e.fd_count(), 0);
        assert!(e.fds().is_empty());
    }

    #[test]
    fn push_fd() {
        let mut e = FdEnvelope::new();
        assert!(e.push_fd(3).is_ok());
        assert_eq!(e.fd_count(), 1);
        assert_eq!(e.fds()[0], 3);
    }

    #[test]
    fn push_invalid_fd() {
        let mut e = FdEnvelope::new();
        assert!(e.push_fd(-1).is_err());
    }

    #[test]
    #[allow(clippy::cast_possible_wrap)]
    fn too_many_fds() {
        let mut e = FdEnvelope::new();
        for i in 0..MAX_FDS as i32 {
            assert!(e.push_fd(i + 3).is_ok());
        }
        assert!(e.push_fd(100).is_err());
    }

    #[test]
    fn max_fds_is_16() {
        assert_eq!(MAX_FDS, 16);
    }

    #[test]
    fn ancillary_data() {
        let mut e = FdEnvelope::new();
        e.set_data(b"metadata".to_vec());
        assert_eq!(e.data(), b"metadata");
    }

    #[test]
    fn default_envelope() {
        let e = FdEnvelope::default();
        assert_eq!(e.fd_count(), 0);
    }

    #[test]
    fn envelope_data_default_empty() {
        let e = FdEnvelope::new();
        assert!(e.data().is_empty());
    }

    #[test]
    fn envelope_multiple_fds() {
        let mut e = FdEnvelope::new();
        assert!(e.push_fd(3).is_ok());
        assert!(e.push_fd(4).is_ok());
        assert!(e.push_fd(5).is_ok());
        assert_eq!(e.fd_count(), 3);
    }

    #[test]
    fn envelope_fd_ordering() {
        let mut e = FdEnvelope::new();
        assert!(e.push_fd(10).is_ok());
        assert!(e.push_fd(20).is_ok());
        assert_eq!(e.fds()[0], 10);
        assert_eq!(e.fds()[1], 20);
    }

    #[test]
    fn envelope_with_data() {
        let mut e = FdEnvelope::new();
        e.set_data(vec![1, 2, 3, 4]);
        assert_eq!(e.data(), &[1, 2, 3, 4]);
    }

    #[test]
    fn fd_type_variants_distinct() {
        let master = FdType::PtyMaster;
        let slave = FdType::PtySlave;
        assert_ne!(master, slave);
    }
}
