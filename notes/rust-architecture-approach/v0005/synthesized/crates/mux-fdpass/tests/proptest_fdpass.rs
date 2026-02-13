//! Property-based tests for mux-fdpass.

use proptest::prelude::*;
use mux_fdpass::{FdEnvelope, FdEntry, FdType, MAX_FDS};

proptest! {
    #[test]
    fn envelope_serialize_roundtrip(
        fds in proptest::collection::vec(3..1000i32, 0..16usize),
        payload in proptest::collection::vec(proptest::num::u8::ANY, 0..100),
    ) {
        let mut env = FdEnvelope::new();
        for fd in &fds {
            let _ = env.add(FdEntry {
                raw_fd: *fd,
                fd_type: FdType::Generic,
                label: String::new(),
            });
        }
        env.payload = payload.clone();
        let serialized = env.serialize();
        let deserialized = FdEnvelope::deserialize(&serialized).unwrap_or_default();
        prop_assert_eq!(deserialized.fd_count(), fds.len().min(MAX_FDS));
        prop_assert_eq!(deserialized.payload, payload);
    }

    #[test]
    fn envelope_fd_count_capped(fds in proptest::collection::vec(3..1000i32, 0..30usize)) {
        let mut env = FdEnvelope::new();
        for fd in &fds {
            let _ = env.add(FdEntry {
                raw_fd: *fd,
                fd_type: FdType::Generic,
                label: String::new(),
            });
        }
        prop_assert!(env.fd_count() <= MAX_FDS);
    }

    #[test]
    fn envelope_empty_valid(_dummy in 0..1u8) {
        let env = FdEnvelope::new();
        prop_assert!(env.is_empty());
        prop_assert!(env.payload.is_empty());
    }

    #[test]
    fn envelope_payload_preserved(payload in proptest::collection::vec(proptest::num::u8::ANY, 0..500)) {
        let mut env = FdEnvelope::new();
        env.payload = payload.clone();
        let serialized = env.serialize();
        let deserialized = FdEnvelope::deserialize(&serialized).unwrap_or_default();
        prop_assert_eq!(deserialized.payload, payload);
    }

    #[test]
    fn fd_type_tag_roundtrip(tag in 0u8..4) {
        let fd_type = FdType::from_tag(tag).unwrap_or(FdType::Generic);
        let roundtripped = FdType::from_tag(fd_type.to_tag()).unwrap_or(FdType::Generic);
        prop_assert_eq!(roundtripped, fd_type);
    }
}

#[test]
fn envelope_max_fds() {
    let mut env = FdEnvelope::new();
    for i in 0..MAX_FDS as i32 {
        env.add(FdEntry {
            raw_fd: 3 + i,
            fd_type: FdType::Generic,
            label: String::new(),
        })
        .unwrap_or(());
    }
    assert_eq!(env.fd_count(), MAX_FDS);
}

#[test]
fn envelope_exceeds_max_errors() {
    let mut env = FdEnvelope::new();
    for i in 0..MAX_FDS as i32 {
        env.add(FdEntry {
            raw_fd: 3 + i,
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
fn deserialize_empty() {
    let result = FdEnvelope::deserialize(b"");
    assert!(result.is_ok());
    let env = result.unwrap_or_default();
    assert!(env.is_empty());
}

#[test]
fn deserialize_corrupted_too_many() {
    // Count byte says 255 fds, which exceeds MAX_FDS
    let result = FdEnvelope::deserialize(&[0xFF]);
    assert!(result.is_err());
}

#[test]
fn serialize_deterministic() {
    let mut env = FdEnvelope::new();
    env.add(FdEntry {
        raw_fd: 3,
        fd_type: FdType::PtyMaster,
        label: "test".into(),
    })
    .unwrap_or(());
    env.payload = vec![1, 2, 3];
    let a = env.serialize();
    let b = env.serialize();
    assert_eq!(a, b);
}

#[test]
fn fd_type_all_tags() {
    assert_eq!(FdType::Generic.to_tag(), 0);
    assert_eq!(FdType::PtyMaster.to_tag(), 1);
    assert_eq!(FdType::PtySlave.to_tag(), 2);
    assert_eq!(FdType::UnixSocket.to_tag(), 3);
}

#[test]
fn fd_type_invalid_tag() {
    assert!(FdType::from_tag(99).is_err());
}

#[test]
fn envelope_entries_accessor() {
    let mut env = FdEnvelope::new();
    env.add(FdEntry {
        raw_fd: 5,
        fd_type: FdType::PtyMaster,
        label: "master".into(),
    })
    .unwrap_or(());
    let entries = env.entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].raw_fd, 5);
    assert_eq!(entries[0].fd_type, FdType::PtyMaster);
    assert_eq!(entries[0].label, "master");
}

#[test]
fn envelope_default() {
    let env = FdEnvelope::default();
    assert!(env.is_empty());
}
