//! Error types shared across the workspace.

use thiserror::Error;

/// Top-level error type for cross-crate error propagation.
#[derive(Debug, Error)]
pub enum MuxError {
    /// Grid operation error.
    #[error("grid error: {0}")]
    Grid(String),

    /// Parser error.
    #[error("parse error: {0}")]
    Parse(String),

    /// Kernel error.
    #[error("kernel error: {0}")]
    Kernel(String),

    /// Configuration error.
    #[error("config error: {0}")]
    Config(String),

    /// Protocol error.
    #[error("protocol error: {0}")]
    Protocol(String),

    /// API error.
    #[error("API error: {0}")]
    Api(String),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Entity not found.
    #[error("not found: {0}")]
    NotFound(String),

    /// Invalid argument.
    #[error("invalid argument: {0}")]
    InvalidArgument(String),

    /// Capacity exceeded.
    #[error("capacity exceeded: {0}")]
    CapacityExceeded(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_grid() {
        let e = MuxError::Grid("out of bounds".into());
        assert_eq!(e.to_string(), "grid error: out of bounds");
    }

    #[test]
    fn error_display_not_found() {
        let e = MuxError::NotFound("session 'dev'".into());
        assert_eq!(e.to_string(), "not found: session 'dev'");
    }

    #[test]
    fn error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "gone");
        let e: MuxError = io_err.into();
        assert!(matches!(e, MuxError::Io(_)));
    }

    #[test]
    fn error_is_debug() {
        let e = MuxError::Parse("bad input".into());
        let debug = format!("{e:?}");
        assert!(debug.contains("Parse"));
    }

    #[test]
    fn error_display_kernel() {
        let e = MuxError::Kernel("crash".into());
        assert_eq!(e.to_string(), "kernel error: crash");
    }

    #[test]
    fn error_display_config() {
        let e = MuxError::Config("bad option".into());
        assert_eq!(e.to_string(), "config error: bad option");
    }

    #[test]
    fn error_display_protocol() {
        let e = MuxError::Protocol("bad frame".into());
        assert_eq!(e.to_string(), "protocol error: bad frame");
    }

    #[test]
    fn error_display_api() {
        let e = MuxError::Api("not ready".into());
        assert_eq!(e.to_string(), "API error: not ready");
    }

    #[test]
    fn error_display_invalid_arg() {
        let e = MuxError::InvalidArgument("too large".into());
        assert_eq!(e.to_string(), "invalid argument: too large");
    }

    #[test]
    fn error_display_capacity() {
        let e = MuxError::CapacityExceeded("ring full".into());
        assert_eq!(e.to_string(), "capacity exceeded: ring full");
    }

    #[test]
    fn error_io_display() {
        let io_err = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "broken");
        let e: MuxError = io_err.into();
        let s = e.to_string();
        assert!(s.contains("I/O error"));
    }

    #[test]
    fn error_not_found_is_error_trait() {
        let e: Box<dyn std::error::Error> = Box::new(MuxError::NotFound("x".into()));
        assert!(e.to_string().contains("not found"));
    }
}
