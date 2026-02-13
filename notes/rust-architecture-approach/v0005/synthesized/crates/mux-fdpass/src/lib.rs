//! SCM_RIGHTS fd passing envelope for tmux-compatible PTY handoff.
//!
//! Provides the data model for fd passing messages. Actual sendmsg/recvmsg
//! syscalls live in mux-pty (unsafe quarantine). The envelope has a 16-fd limit
//! per message for tmux compatibility.

#![forbid(unsafe_code)]

use thiserror::Error;

/// Maximum file descriptors per envelope.
pub const MAX_FDS: usize = 16;

/// Fd passing errors.
#[derive(Debug, Error)]
pub enum FdPassError {
    /// Too many file descriptors.
    #[error("too many fds: {0} exceeds limit of {MAX_FDS}")]
    TooManyFds(usize),
    /// Invalid fd type.
    #[error("invalid fd type: {0}")]
    InvalidType(u8),
}

/// Type classification for a passed file descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FdType {
    /// PTY master file descriptor.
    PtyMaster,
    /// PTY slave file descriptor.
    PtySlave,
    /// Unix domain socket.
    UnixSocket,
    /// Generic file descriptor.
    Generic,
}

impl FdType {
    /// Convert to a u8 tag.
    pub const fn to_tag(self) -> u8 {
        match self {
            Self::PtyMaster => 1,
            Self::PtySlave => 2,
            Self::UnixSocket => 3,
            Self::Generic => 0,
        }
    }

    /// Convert from a u8 tag.
    pub fn from_tag(tag: u8) -> Result<Self, FdPassError> {
        match tag {
            0 => Ok(Self::Generic),
            1 => Ok(Self::PtyMaster),
            2 => Ok(Self::PtySlave),
            3 => Ok(Self::UnixSocket),
            _ => Err(FdPassError::InvalidType(tag)),
        }
    }
}

/// A descriptor entry in the envelope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FdEntry {
    /// Raw fd number (for serialization only; not an actual open fd in this crate).
    pub raw_fd: i32,
    /// Type classification.
    pub fd_type: FdType,
    /// Optional label for debugging.
    pub label: String,
}

/// An envelope containing file descriptors for SCM_RIGHTS passing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FdEnvelope {
    /// File descriptor entries (max 16).
    entries: Vec<FdEntry>,
    /// Optional payload bytes.
    pub payload: Vec<u8>,
}

impl Default for FdEnvelope {
    fn default() -> Self {
        Self::new()
    }
}

impl FdEnvelope {
    /// Create an empty envelope.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            payload: Vec::new(),
        }
    }

    /// Add a file descriptor entry.
    pub fn add(&mut self, entry: FdEntry) -> Result<(), FdPassError> {
        if self.entries.len() >= MAX_FDS {
            return Err(FdPassError::TooManyFds(self.entries.len() + 1));
        }
        self.entries.push(entry);
        Ok(())
    }

    /// Get all entries.
    pub fn entries(&self) -> &[FdEntry] {
        &self.entries
    }

    /// Number of file descriptors.
    pub fn fd_count(&self) -> usize {
        self.entries.len()
    }

    /// Is the envelope empty?
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Serialize the envelope to bytes.
    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.push(self.entries.len() as u8);
        for entry in &self.entries {
            buf.extend_from_slice(&entry.raw_fd.to_le_bytes());
            buf.push(entry.fd_type.to_tag());
            let label_bytes = entry.label.as_bytes();
            buf.push(label_bytes.len().min(255) as u8);
            buf.extend_from_slice(&label_bytes[..label_bytes.len().min(255)]);
        }
        buf.extend_from_slice(&self.payload);
        buf
    }

    /// Deserialize from bytes.
    pub fn deserialize(data: &[u8]) -> Result<Self, FdPassError> {
        if data.is_empty() {
            return Ok(Self::new());
        }
        let count = data[0] as usize;
        if count > MAX_FDS {
            return Err(FdPassError::TooManyFds(count));
        }

        let mut envelope = Self::new();
        let mut pos = 1;

        for _ in 0..count {
            if pos + 6 > data.len() {
                break;
            }
            let raw_fd = i32::from_le_bytes([
                data[pos],
                data[pos + 1],
                data[pos + 2],
                data[pos + 3],
            ]);
            pos += 4;
            let fd_type = FdType::from_tag(data[pos])?;
            pos += 1;
            let label_len = data[pos] as usize;
            pos += 1;
            let label_end = (pos + label_len).min(data.len());
            let label = String::from_utf8_lossy(&data[pos..label_end]).to_string();
            pos = label_end;

            envelope.entries.push(FdEntry {
                raw_fd,
                fd_type,
                label,
            });
        }

        if pos < data.len() {
            envelope.payload = data[pos..].to_vec();
        }

        Ok(envelope)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_envelope() {
        let env = FdEnvelope::new();
        assert!(env.is_empty());
        assert_eq!(env.fd_count(), 0);
    }

    #[test]
    fn add_entry() {
        let mut env = FdEnvelope::new();
        env.add(FdEntry {
            raw_fd: 3,
            fd_type: FdType::PtyMaster,
            label: "master".into(),
        })
        .unwrap_or(());
        assert_eq!(env.fd_count(), 1);
    }

    #[test]
    fn max_fds_enforced() {
        let mut env = FdEnvelope::new();
        for i in 0..MAX_FDS {
            env.add(FdEntry {
                raw_fd: i as i32,
                fd_type: FdType::Generic,
                label: String::new(),
            })
            .unwrap_or(());
        }
        let result = env.add(FdEntry {
            raw_fd: 99,
            fd_type: FdType::Generic,
            label: String::new(),
        });
        assert!(result.is_err());
    }

    #[test]
    fn fd_type_roundtrip() {
        for fd_type in [
            FdType::PtyMaster,
            FdType::PtySlave,
            FdType::UnixSocket,
            FdType::Generic,
        ] {
            let tag = fd_type.to_tag();
            let parsed = FdType::from_tag(tag).unwrap_or(FdType::Generic);
            assert_eq!(parsed, fd_type);
        }
    }

    #[test]
    fn serialize_deserialize_roundtrip() {
        let mut env = FdEnvelope::new();
        env.add(FdEntry {
            raw_fd: 5,
            fd_type: FdType::PtyMaster,
            label: "test".into(),
        })
        .unwrap_or(());
        env.payload = b"extra".to_vec();

        let data = env.serialize();
        let parsed = FdEnvelope::deserialize(&data).unwrap_or_default();
        assert_eq!(parsed.fd_count(), 1);
        assert_eq!(parsed.entries()[0].raw_fd, 5);
        assert_eq!(parsed.entries()[0].fd_type, FdType::PtyMaster);
    }

    #[test]
    fn deserialize_empty() {
        let parsed = FdEnvelope::deserialize(b"").unwrap_or_default();
        assert!(parsed.is_empty());
    }

    #[test]
    fn invalid_fd_type_tag() {
        assert!(FdType::from_tag(99).is_err());
    }

    #[test]
    fn entries_accessor() {
        let mut env = FdEnvelope::new();
        env.add(FdEntry {
            raw_fd: 1,
            fd_type: FdType::Generic,
            label: String::new(),
        })
        .unwrap_or(());
        assert_eq!(env.entries().len(), 1);
    }

    #[test]
    fn default_envelope() {
        let env = FdEnvelope::default();
        assert!(env.is_empty());
    }

    #[test]
    fn fd_type_pty_master_tag() {
        assert_eq!(FdType::PtyMaster.to_tag(), 1);
    }

    #[test]
    fn fd_type_pty_slave_tag() {
        assert_eq!(FdType::PtySlave.to_tag(), 2);
    }

    #[test]
    fn fd_type_unix_socket_tag() {
        assert_eq!(FdType::UnixSocket.to_tag(), 3);
    }

    #[test]
    fn fd_type_generic_tag() {
        assert_eq!(FdType::Generic.to_tag(), 0);
    }

    #[test]
    fn error_display_too_many() {
        let e = FdPassError::TooManyFds(20);
        assert!(e.to_string().contains("too many fds"));
    }

    #[test]
    fn error_display_invalid_type() {
        let e = FdPassError::InvalidType(99);
        assert!(e.to_string().contains("invalid fd type"));
    }

    #[test]
    fn entry_with_label() {
        let entry = FdEntry {
            raw_fd: 7,
            fd_type: FdType::PtySlave,
            label: "my-slave".into(),
        };
        assert_eq!(entry.label, "my-slave");
        assert_eq!(entry.raw_fd, 7);
    }

    #[test]
    fn envelope_payload_preserved() {
        let mut env = FdEnvelope::new();
        env.payload = vec![1, 2, 3, 4, 5];
        let data = env.serialize();
        let parsed = FdEnvelope::deserialize(&data).unwrap_or_default();
        assert_eq!(parsed.payload, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn multiple_entries_roundtrip() {
        let mut env = FdEnvelope::new();
        for i in 0..5 {
            env.add(FdEntry {
                raw_fd: i,
                fd_type: FdType::Generic,
                label: format!("fd{i}"),
            })
            .unwrap_or(());
        }
        let data = env.serialize();
        let parsed = FdEnvelope::deserialize(&data).unwrap_or_default();
        assert_eq!(parsed.fd_count(), 5);
    }

    mod prop {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn fd_type_tag_roundtrip(tag in 0u8..4) {
                let fd_type = FdType::from_tag(tag).unwrap_or(FdType::Generic);
                prop_assert_eq!(FdType::from_tag(fd_type.to_tag()).unwrap_or(FdType::Generic), fd_type);
            }

            #[test]
            fn serialize_roundtrip(fds in proptest::collection::vec(0i32..100, 0..MAX_FDS)) {
                let mut env = FdEnvelope::new();
                for fd in &fds {
                    env.add(FdEntry {
                        raw_fd: *fd,
                        fd_type: FdType::Generic,
                        label: String::new(),
                    }).unwrap_or(());
                }
                let data = env.serialize();
                let parsed = FdEnvelope::deserialize(&data).unwrap_or_default();
                prop_assert_eq!(parsed.fd_count(), fds.len().min(MAX_FDS));
            }
        }
    }
}
