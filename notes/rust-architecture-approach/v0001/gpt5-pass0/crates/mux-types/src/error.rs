use core::fmt;

/// Canonical runtime errors for termlets and runtime plumbing (S98).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TermletError {
    ProtocolMismatch,
    InvalidState,
    SnapshotChecksumMismatch,
    SnapshotFormatInvalid,
    GraphemeArenaOverflow,
    PackedCellOutOfRange,
    PtySpawnFailed,
    PtyIo,
    Timeout,
    PermissionDenied,
    ResourceQuotaExceeded,
    SandboxViolation,
    ObjectDoesNotExist,
    MultipleObjectsReturned,
    DeterminismViolation,
    InternalInvariant,
}

impl TermletError {
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::ProtocolMismatch => "TFE-0001",
            Self::InvalidState => "TFE-0002",
            Self::SnapshotChecksumMismatch => "TFE-0003",
            Self::SnapshotFormatInvalid => "TFE-0004",
            Self::GraphemeArenaOverflow => "TFE-0005",
            Self::PackedCellOutOfRange => "TFE-0006",
            Self::PtySpawnFailed => "TFE-0007",
            Self::PtyIo => "TFE-0008",
            Self::Timeout => "TFE-0009",
            Self::PermissionDenied => "TFE-0010",
            Self::ResourceQuotaExceeded => "TFE-0011",
            Self::SandboxViolation => "TFE-0012",
            Self::ObjectDoesNotExist => "TFE-0013",
            Self::MultipleObjectsReturned => "TFE-0014",
            Self::DeterminismViolation => "TFE-0015",
            Self::InternalInvariant => "TFE-0016",
        }
    }
}

impl fmt::Display for TermletError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.error_code(), self.as_str())
    }
}

impl std::error::Error for TermletError {}

impl TermletError {
    fn as_str(&self) -> &'static str {
        match self {
            Self::ProtocolMismatch => "protocol mismatch",
            Self::InvalidState => "invalid state",
            Self::SnapshotChecksumMismatch => "snapshot checksum mismatch",
            Self::SnapshotFormatInvalid => "snapshot format invalid",
            Self::GraphemeArenaOverflow => "grapheme arena overflow",
            Self::PackedCellOutOfRange => "packed cell out of range",
            Self::PtySpawnFailed => "pty spawn failed",
            Self::PtyIo => "pty io error",
            Self::Timeout => "timeout",
            Self::PermissionDenied => "permission denied",
            Self::ResourceQuotaExceeded => "resource quota exceeded",
            Self::SandboxViolation => "sandbox violation",
            Self::ObjectDoesNotExist => "object does not exist",
            Self::MultipleObjectsReturned => "multiple objects returned",
            Self::DeterminismViolation => "determinism violation",
            Self::InternalInvariant => "internal invariant violated",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_error_codes_are_unique() {
        let all = [
            TermletError::ProtocolMismatch,
            TermletError::InvalidState,
            TermletError::SnapshotChecksumMismatch,
            TermletError::SnapshotFormatInvalid,
            TermletError::GraphemeArenaOverflow,
            TermletError::PackedCellOutOfRange,
            TermletError::PtySpawnFailed,
            TermletError::PtyIo,
            TermletError::Timeout,
            TermletError::PermissionDenied,
            TermletError::ResourceQuotaExceeded,
            TermletError::SandboxViolation,
            TermletError::ObjectDoesNotExist,
            TermletError::MultipleObjectsReturned,
            TermletError::DeterminismViolation,
            TermletError::InternalInvariant,
        ];
        let mut codes = all.iter().map(|e| e.error_code()).collect::<Vec<_>>();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), 16);
    }

    #[test]
    fn display_contains_error_code() {
        let err = TermletError::Timeout;
        let s = format!("{err}");
        assert!(s.contains("TFE-0009"));
    }

    #[test]
    fn trait_object_safe() {
        fn accepts_error(_: &(dyn std::error::Error + Send + Sync + 'static)) {}
        let e = TermletError::PtyIo;
        accepts_error(&e);
    }
}
