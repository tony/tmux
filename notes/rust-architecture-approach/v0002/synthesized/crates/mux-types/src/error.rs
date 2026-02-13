//! Core error type for TermForge operations.

/// Core error type for TermForge operations.
#[derive(Debug, thiserror::Error)]
pub enum TermForgeError {
    #[error("PTY error: {0}")]
    Pty(String),
    #[error("protocol error: {0}")]
    Protocol(String),
    #[error("kernel error: {0}")]
    Kernel(String),
    #[error("grid error: {0}")]
    Grid(String),
    #[error("config error: {0}")]
    Config(String),
    #[error("target not found: {0}")]
    TargetNotFound(String),
    #[error("session not found: {0}")]
    SessionNotFound(String),
    #[error("window not found: {0}")]
    WindowNotFound(String),
    #[error("pane not found: {0}")]
    PaneNotFound(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("channel closed")]
    ChannelClosed,
    #[error("timeout")]
    Timeout,
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    #[error("not supported: {0}")]
    NotSupported(String),
    #[error("capacity exceeded: {0}")]
    CapacityExceeded(String),
    #[error("layout error: {0}")]
    Layout(String),
    #[error("internal error: {0}")]
    Internal(String),
}

/// Convenience result type.
pub type Result<T> = std::result::Result<T, TermForgeError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let e = TermForgeError::Kernel("test".into());
        assert_eq!(e.to_string(), "kernel error: test");
    }

    #[test]
    fn io_error_converts() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "missing");
        let tf_err: TermForgeError = io_err.into();
        assert!(matches!(tf_err, TermForgeError::Io(_)));
    }
}
