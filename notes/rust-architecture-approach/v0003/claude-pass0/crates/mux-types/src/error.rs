//! Core error types and Result alias.

use thiserror::Error;

/// Core error type for TermForge operations.
#[derive(Debug, Error)]
pub enum TermForgeError {
    /// Entity not found.
    #[error("entity not found: {0}")]
    NotFound(String),

    /// Invalid argument.
    #[error("invalid argument: {0}")]
    InvalidArgument(String),

    /// Operation would violate an invariant.
    #[error("invariant violation: {0}")]
    InvariantViolation(String),

    /// Layout error (e.g., not enough space).
    #[error("layout error: {0}")]
    Layout(String),

    /// Parse error.
    #[error("parse error: {0}")]
    Parse(String),

    /// Configuration error.
    #[error("config error: {0}")]
    Config(String),

    /// Protocol error.
    #[error("protocol error: {0}")]
    Protocol(String),

    /// I/O error wrapper.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Capacity exceeded.
    #[error("capacity exceeded: {0}")]
    CapacityExceeded(String),
}

/// Convenience Result alias.
pub type Result<T> = std::result::Result<T, TermForgeError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_not_found() {
        let e = TermForgeError::NotFound("session $main".into());
        assert_eq!(format!("{e}"), "entity not found: session $main");
    }

    #[test]
    fn error_display_invariant() {
        let e = TermForgeError::InvariantViolation("INV-119".into());
        assert!(format!("{e}").contains("INV-119"));
    }

    #[test]
    fn error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "gone");
        let e: TermForgeError = io_err.into();
        assert!(matches!(e, TermForgeError::Io(_)));
    }

    #[test]
    fn result_alias_works() {
        fn test_fn() -> Result<u32> {
            Ok(42)
        }
        assert_eq!(test_fn().unwrap_or(0), 42);
    }
}
