//! Core error type and Result alias.

use thiserror::Error;

/// Core error type for TermForge operations.
#[derive(Debug, Error)]
pub enum TermForgeError {
    /// I/O error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// Invalid argument.
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    /// Entity not found.
    #[error("not found: {0}")]
    NotFound(String),
    /// Operation not supported.
    #[error("not supported: {0}")]
    NotSupported(String),
    /// Internal error (should not happen).
    #[error("internal error: {0}")]
    Internal(String),
}

/// Result alias using `TermForgeError`.
pub type Result<T> = std::result::Result<T, TermForgeError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let e = TermForgeError::InvalidArgument("bad value".into());
        assert_eq!(format!("{e}"), "invalid argument: bad value");
    }

    #[test]
    fn error_not_found() {
        let e = TermForgeError::NotFound("session:42".into());
        assert_eq!(format!("{e}"), "not found: session:42");
    }

    #[test]
    fn error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let e: TermForgeError = io_err.into();
        assert!(format!("{e}").contains("file missing"));
    }
}
