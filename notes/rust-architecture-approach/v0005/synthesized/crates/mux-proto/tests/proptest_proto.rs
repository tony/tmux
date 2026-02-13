//! Property-based tests for mux-proto wire protocol.

use proptest::prelude::*;
use mux_proto::{Frame, FrameType, is_tf01_frame};

proptest! {
    #[test]
    fn frame_encode_decode_roundtrip(payload in proptest::collection::vec(proptest::num::u8::ANY, 0..500)) {
        let frame = Frame::new(FrameType::Data, payload.clone());
        let encoded = frame.encode();
        let decoded = Frame::decode(&encoded);
        prop_assert!(decoded.is_ok());
        let decoded = decoded.unwrap_or_else(|_| Frame::new(FrameType::Error, vec![]));
        prop_assert_eq!(decoded.payload, payload);
    }

    #[test]
    fn frame_crc_detects_corruption(payload in proptest::collection::vec(proptest::num::u8::ANY, 1..200)) {
        let frame = Frame::new(FrameType::Data, payload);
        let mut encoded = frame.encode();
        // Corrupt one byte in the payload area
        if encoded.len() > 16 {
            encoded[16] ^= 0xFF;
            let result = Frame::decode(&encoded);
            prop_assert!(result.is_err());
        }
    }

    #[test]
    fn frame_type_preserved(ft in arb_frame_type()) {
        let frame = Frame::new(ft, vec![1, 2, 3]);
        let encoded = frame.encode();
        let decoded = Frame::decode(&encoded).unwrap_or_else(|_| Frame::new(FrameType::Error, vec![]));
        prop_assert_eq!(decoded.header.frame_type, ft);
    }

    #[test]
    fn empty_payload_roundtrip(ft in arb_frame_type()) {
        let frame = Frame::new(ft, vec![]);
        let encoded = frame.encode();
        let decoded = Frame::decode(&encoded).unwrap_or_else(|_| Frame::new(FrameType::Error, vec![]));
        prop_assert!(decoded.payload.is_empty());
    }

    #[test]
    fn large_payload(size in 1000..5000usize) {
        let payload: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
        let frame = Frame::new(FrameType::Data, payload.clone());
        let encoded = frame.encode();
        let decoded = Frame::decode(&encoded).unwrap_or_else(|_| Frame::new(FrameType::Error, vec![]));
        prop_assert_eq!(decoded.payload, payload);
    }

    #[test]
    fn is_tf01_for_valid_frames(payload in proptest::collection::vec(proptest::num::u8::ANY, 0..100)) {
        let frame = Frame::new(FrameType::Data, payload);
        let encoded = frame.encode();
        prop_assert!(is_tf01_frame(&encoded));
    }
}

fn arb_frame_type() -> impl Strategy<Value = FrameType> {
    prop_oneof![
        Just(FrameType::Data),
        Just(FrameType::Command),
        Just(FrameType::CommandResponse),
        Just(FrameType::Hello),
        Just(FrameType::Resize),
        Just(FrameType::KeyInput),
        Just(FrameType::Shutdown),
    ]
}

#[test]
fn truncated_frame_errors() {
    let frame = Frame::new(FrameType::Data, vec![1, 2, 3]);
    let encoded = frame.encode();
    let truncated = &encoded[..encoded.len() / 2];
    assert!(Frame::decode(truncated).is_err());
}

#[test]
fn magic_bytes_verified() {
    let bad = vec![0u8; 20];
    assert!(Frame::decode(&bad).is_err());
}

#[test]
fn shutdown_frame() {
    let frame = Frame::new(FrameType::Shutdown, vec![]);
    let encoded = frame.encode();
    let decoded = Frame::decode(&encoded).unwrap_or_else(|_| Frame::new(FrameType::Error, vec![]));
    assert_eq!(decoded.header.frame_type, FrameType::Shutdown);
}

#[test]
fn not_tf01_for_garbage() {
    assert!(!is_tf01_frame(&[0, 0, 0, 0]));
    assert!(!is_tf01_frame(&[]));
}
