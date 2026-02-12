//! TermletError: the canonical error type for termlet operations.
//!
//! ## Invariants
//! - S98: TermletError has exactly 16 variants.
//! - INV-005: Implements `std::error::Error + Send + Sync + 'static`.
//! - INV-006: No panicking in library code.

use std::fmt;

/// Error codes for machine-readable identification.
/// Stable across versions (RULE-S19-07).
pub const EC_SPAWN_FAILED: u16 = 8001;
pub const EC_SEND_KEYS_FAILED: u16 = 8002;
pub const EC_WAIT_TIMEOUT: u16 = 8003;
pub const EC_EXPECT_MISMATCH: u16 = 8004;
pub const EC_SNAPSHOT_FAILED: u16 = 8005;
pub const EC_RESIZE_FAILED: u16 = 8006;
pub const EC_KILL_FAILED: u16 = 8007;
pub const EC_ALREADY_DEAD: u16 = 8008;
pub const EC_CONFIG_INVALID: u16 = 8009;
pub const EC_QUOTA_EXCEEDED: u16 = 8010;
pub const EC_SANDBOX_VIOLATION: u16 = 8011;
pub const EC_SOCKET_ERROR: u16 = 8012;
pub const EC_PROTOCOL_ERROR: u16 = 8013;
pub const EC_DCS_FORWARD_FAILED: u16 = 8014;
pub const EC_RECORD_FAILED: u16 = 8015;
pub const EC_HANDLE_CLOSED: u16 = 8016;

/// S98: TermletError has exactly 16 variants covering all termlet failure modes.
///
/// INV-005: Implements `Error + Send + Sync + 'static`.
/// INV-006: No panics; all fallible operations return `Result<T, TermletError>`.
#[derive(Debug, Clone)]
pub enum TermletError {
    /// Termlet process failed to spawn.
    SpawnFailed(String),
    /// send_keys() could not deliver keys to child.
    SendKeysFailed(String),
    /// wait_for() timed out before pattern matched.
    WaitTimeout { pattern: String, timeout_ms: u64 },
    /// expect_or_fail() found content mismatch.
    ExpectMismatch { expected: String, actual: String },
    /// Snapshot capture failed.
    SnapshotFailed(String),
    /// Resize failed (invalid dimensions or PTY error).
    ResizeFailed { rows: u16, cols: u16, reason: String },
    /// Kill signal could not be delivered.
    KillFailed(String),
    /// Operation on a termlet that is already dead.
    AlreadyDead,
    /// TermletConfig validation failed.
    ConfigInvalid(String),
    /// Resource quota exceeded (CPU, RAM, or FD limit).
    QuotaExceeded { resource: String, limit: u64, actual: u64 },
    /// Sandbox namespace violation.
    SandboxViolation(String),
    /// Socket-level error (connection, bind, etc.).
    SocketError(String),
    /// Wire protocol error.
    ProtocolError(String),
    /// DCS passthrough forwarding failed.
    DcsForwardFailed(String),
    /// Recording file I/O error.
    RecordFailed(String),
    /// PtyHandle is closed; no further operations.
    HandleClosed,
}

impl TermletError {
    /// Unique error code for machine-readable identification.
    /// Stable across versions (RULE-S19-07).
    pub fn error_code(&self) -> u16 {
        match self {
            Self::SpawnFailed(_) => EC_SPAWN_FAILED,
            Self::SendKeysFailed(_) => EC_SEND_KEYS_FAILED,
            Self::WaitTimeout { .. } => EC_WAIT_TIMEOUT,
            Self::ExpectMismatch { .. } => EC_EXPECT_MISMATCH,
            Self::SnapshotFailed(_) => EC_SNAPSHOT_FAILED,
            Self::ResizeFailed { .. } => EC_RESIZE_FAILED,
            Self::KillFailed(_) => EC_KILL_FAILED,
            Self::AlreadyDead => EC_ALREADY_DEAD,
            Self::ConfigInvalid(_) => EC_CONFIG_INVALID,
            Self::QuotaExceeded { .. } => EC_QUOTA_EXCEEDED,
            Self::SandboxViolation(_) => EC_SANDBOX_VIOLATION,
            Self::SocketError(_) => EC_SOCKET_ERROR,
            Self::ProtocolError(_) => EC_PROTOCOL_ERROR,
            Self::DcsForwardFailed(_) => EC_DCS_FORWARD_FAILED,
            Self::RecordFailed(_) => EC_RECORD_FAILED,
            Self::HandleClosed => EC_HANDLE_CLOSED,
        }
    }
}

impl fmt::Display for TermletError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = self.error_code();
        match self {
            Self::SpawnFailed(msg) => write!(f, "[E{code}] spawn failed: {msg}"),
            Self::SendKeysFailed(msg) => write!(f, "[E{code}] send_keys failed: {msg}"),
            Self::WaitTimeout { pattern, timeout_ms } => {
                write!(f, "[E{code}] wait_for timed out after {timeout_ms}ms waiting for '{pattern}'")
            }
            Self::ExpectMismatch { expected, actual } => {
                write!(f, "[E{code}] expect mismatch: expected '{expected}', got '{actual}'")
            }
            Self::SnapshotFailed(msg) => write!(f, "[E{code}] snapshot failed: {msg}"),
            Self::ResizeFailed { rows, cols, reason } => {
                write!(f, "[E{code}] resize to {rows}x{cols} failed: {reason}")
            }
            Self::KillFailed(msg) => write!(f, "[E{code}] kill failed: {msg}"),
            Self::AlreadyDead => write!(f, "[E{code}] termlet is already dead"),
            Self::ConfigInvalid(msg) => write!(f, "[E{code}] config invalid: {msg}"),
            Self::QuotaExceeded { resource, limit, actual } => {
                write!(f, "[E{code}] quota exceeded for {resource}: limit={limit}, actual={actual}")
            }
            Self::SandboxViolation(msg) => write!(f, "[E{code}] sandbox violation: {msg}"),
            Self::SocketError(msg) => write!(f, "[E{code}] socket error: {msg}"),
            Self::ProtocolError(msg) => write!(f, "[E{code}] protocol error: {msg}"),
            Self::DcsForwardFailed(msg) => write!(f, "[E{code}] DCS forward failed: {msg}"),
            Self::RecordFailed(msg) => write!(f, "[E{code}] recording failed: {msg}"),
            Self::HandleClosed => write!(f, "[E{code}] handle is closed"),
        }
    }
}

impl std::error::Error for TermletError {}

// INV-005: Compile-time verification that TermletError is Send + Sync.
const _: () = {
    fn assert_send_sync<T: Send + Sync + 'static>() {}
    fn check() {
        assert_send_sync::<TermletError>();
    }
};

#[cfg(test)]
mod tests {
    use super::*;

    /// S98: TermletError has exactly 16 variants.
    #[test]
    fn test_sixteen_variants() {
        // Construct one of each variant to prove they exist.
        let variants: Vec<TermletError> = vec![
            TermletError::SpawnFailed("".into()),
            TermletError::SendKeysFailed("".into()),
            TermletError::WaitTimeout { pattern: "".into(), timeout_ms: 0 },
            TermletError::ExpectMismatch { expected: "".into(), actual: "".into() },
            TermletError::SnapshotFailed("".into()),
            TermletError::ResizeFailed { rows: 0, cols: 0, reason: "".into() },
            TermletError::KillFailed("".into()),
            TermletError::AlreadyDead,
            TermletError::ConfigInvalid("".into()),
            TermletError::QuotaExceeded { resource: "".into(), limit: 0, actual: 0 },
            TermletError::SandboxViolation("".into()),
            TermletError::SocketError("".into()),
            TermletError::ProtocolError("".into()),
            TermletError::DcsForwardFailed("".into()),
            TermletError::RecordFailed("".into()),
            TermletError::HandleClosed,
        ];
        assert_eq!(variants.len(), 16);
    }

    /// Error codes are unique across all 16 variants.
    #[test]
    fn test_error_codes_unique() {
        let codes = [
            EC_SPAWN_FAILED, EC_SEND_KEYS_FAILED, EC_WAIT_TIMEOUT,
            EC_EXPECT_MISMATCH, EC_SNAPSHOT_FAILED, EC_RESIZE_FAILED,
            EC_KILL_FAILED, EC_ALREADY_DEAD, EC_CONFIG_INVALID,
            EC_QUOTA_EXCEEDED, EC_SANDBOX_VIOLATION, EC_SOCKET_ERROR,
            EC_PROTOCOL_ERROR, EC_DCS_FORWARD_FAILED, EC_RECORD_FAILED,
            EC_HANDLE_CLOSED,
        ];
        let mut sorted = codes.to_vec();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), codes.len());
    }

    /// INV-005: Error is Send + Sync.
    #[test]
    fn test_is_send_sync() {
        fn assert_send_sync<T: Send + Sync + 'static>() {}
        assert_send_sync::<TermletError>();
    }

    /// Display includes error code prefix.
    #[test]
    fn test_display_has_error_code() {
        let e = TermletError::SpawnFailed("permission denied".into());
        let s = format!("{e}");
        assert!(s.starts_with("[E8001]"));
        assert!(s.contains("spawn failed"));
    }

    /// QuotaExceeded carries structured context.
    #[test]
    fn test_quota_exceeded_context() {
        let e = TermletError::QuotaExceeded {
            resource: "ram_bytes".into(),
            limit: 1024 * 1024,
            actual: 2 * 1024 * 1024,
        };
        let s = format!("{e}");
        assert!(s.contains("ram_bytes"));
        assert!(s.contains("1048576"));
    }
}
