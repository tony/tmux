//! # mux-proto
//!
//! Wire protocol v8 TLV (Type-Length-Value) frames for terminal multiplexer
//! communication.
//!
//! ## Key Design
//! - INV-001: 100% tmux wire-protocol v8 compatibility.
//! - INV-020: Frames are length-delimited.
//! - Frame structure: [1 byte type][4 bytes length LE][N bytes payload].
//! - Maximum frame size: 64 KiB.

#![forbid(unsafe_code)]

use std::fmt;

/// Ergonomic re-exports for downstream crates.
pub mod prelude {
    pub use super::{
        validate_handshake, Frame, FrameType, ProtocolError, FRAME_HEADER_SIZE, MAX_FRAME_SIZE,
        PROTOCOL_VERSION,
    };
}

/// Maximum frame payload size (64 KiB).
pub const MAX_FRAME_SIZE: usize = 64 * 1024;

/// Wire protocol version.
pub const PROTOCOL_VERSION: u8 = 8;

/// Frame header size: 1 byte type + 4 bytes length.
pub const FRAME_HEADER_SIZE: usize = 5;

/// Frame type identifier.
///
/// INV-001: Compatible with tmux wire protocol v8.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum FrameType {
    Handshake = 0x01,
    Command   = 0x02,
    Response  = 0x03,
    Event     = 0x04,
    Error     = 0x05,
    Ping      = 0x06,
    Pong      = 0x07,
    Resize    = 0x08,
    Data      = 0x09,
    Close     = 0x0A,
}

impl FrameType {
    /// Parse from a byte value.
    #[must_use]
    pub fn from_byte(b: u8) -> Option<Self> {
        match b {
            0x01 => Some(Self::Handshake),
            0x02 => Some(Self::Command),
            0x03 => Some(Self::Response),
            0x04 => Some(Self::Event),
            0x05 => Some(Self::Error),
            0x06 => Some(Self::Ping),
            0x07 => Some(Self::Pong),
            0x08 => Some(Self::Resize),
            0x09 => Some(Self::Data),
            0x0A => Some(Self::Close),
            _ => None,
        }
    }

    /// True if this frame type expects a response.
    #[must_use]
    pub fn expects_response(self) -> bool {
        matches!(self, Self::Command | Self::Ping | Self::Handshake)
    }

    /// True if this is a control frame (not data).
    #[must_use]
    pub fn is_control(self) -> bool {
        !matches!(self, Self::Data)
    }
}

/// A wire protocol frame.
///
/// INV-020: Length-delimited with a type byte prefix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub frame_type: FrameType,
    pub payload: Vec<u8>,
}

/// Protocol error type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolError {
    FrameTooLarge { size: usize, limit: usize },
    UnknownFrameType(u8),
    Truncated { expected: usize, actual: usize },
    VersionMismatch { expected: u8, actual: u8 },
    InvalidHandshake(String),
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FrameTooLarge { size, limit } =>
                write!(f, "frame too large: {size} bytes (limit: {limit})"),
            Self::UnknownFrameType(b) =>
                write!(f, "unknown frame type: 0x{b:02X}"),
            Self::Truncated { expected, actual } =>
                write!(f, "truncated frame: expected {expected} bytes, got {actual}"),
            Self::VersionMismatch { expected, actual } =>
                write!(f, "protocol version mismatch: expected v{expected}, got v{actual}"),
            Self::InvalidHandshake(msg) =>
                write!(f, "invalid handshake: {msg}"),
        }
    }
}

impl std::error::Error for ProtocolError {}

// INV-005: ProtocolError is Send + Sync.
const _: () = {
    fn assert_send_sync<T: Send + Sync + 'static>() {}
    fn check() { assert_send_sync::<ProtocolError>(); }
};

impl Frame {
    /// Create a new frame with the given type and payload.
    #[must_use]
    pub fn new(frame_type: FrameType, payload: Vec<u8>) -> Result<Self, ProtocolError> {
        if payload.len() > MAX_FRAME_SIZE {
            return Err(ProtocolError::FrameTooLarge {
                size: payload.len(),
                limit: MAX_FRAME_SIZE,
            });
        }
        Ok(Self { frame_type, payload })
    }

    /// Create a Ping frame.
    #[must_use]
    pub fn ping() -> Self {
        Self { frame_type: FrameType::Ping, payload: Vec::new() }
    }

    /// Create a Pong frame.
    #[must_use]
    pub fn pong() -> Self {
        Self { frame_type: FrameType::Pong, payload: Vec::new() }
    }

    /// Encode the frame to bytes: [type: u8][length: u32 LE][payload].
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let len = self.payload.len() as u32;
        let mut buf = Vec::with_capacity(FRAME_HEADER_SIZE + self.payload.len());
        buf.push(self.frame_type as u8);
        buf.extend_from_slice(&len.to_le_bytes());
        buf.extend_from_slice(&self.payload);
        buf
    }

    /// Decode a frame from bytes.
    #[must_use]
    pub fn decode(data: &[u8]) -> Result<(Self, usize), ProtocolError> {
        if data.len() < FRAME_HEADER_SIZE {
            return Err(ProtocolError::Truncated {
                expected: FRAME_HEADER_SIZE,
                actual: data.len(),
            });
        }

        let frame_type = FrameType::from_byte(data[0])
            .ok_or(ProtocolError::UnknownFrameType(data[0]))?;

        let payload_len = u32::from_le_bytes([data[1], data[2], data[3], data[4]]) as usize;

        if payload_len > MAX_FRAME_SIZE {
            return Err(ProtocolError::FrameTooLarge {
                size: payload_len,
                limit: MAX_FRAME_SIZE,
            });
        }

        let total_len = FRAME_HEADER_SIZE + payload_len;
        if data.len() < total_len {
            return Err(ProtocolError::Truncated {
                expected: total_len,
                actual: data.len(),
            });
        }

        let payload = data[FRAME_HEADER_SIZE..total_len].to_vec();
        Ok((Self { frame_type, payload }, total_len))
    }

    /// Payload length.
    #[must_use]
    pub fn payload_len(&self) -> usize {
        self.payload.len()
    }
}

/// Create a handshake frame with the given protocol version.
#[must_use]
pub fn handshake_frame(version: u8) -> Frame {
    Frame {
        frame_type: FrameType::Handshake,
        payload: vec![version],
    }
}

/// Validate a handshake frame.
#[must_use]
pub fn validate_handshake(frame: &Frame) -> Result<u8, ProtocolError> {
    if frame.frame_type != FrameType::Handshake {
        return Err(ProtocolError::InvalidHandshake("not a handshake frame".into()));
    }
    if frame.payload.is_empty() {
        return Err(ProtocolError::InvalidHandshake("empty payload".into()));
    }
    let version = frame.payload[0];
    if version != PROTOCOL_VERSION {
        return Err(ProtocolError::VersionMismatch {
            expected: PROTOCOL_VERSION,
            actual: version,
        });
    }
    Ok(version)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Frame encode/decode round-trip.
    #[test]
    fn test_frame_round_trip() {
        let frame = Frame::new(FrameType::Command, b"hello".to_vec()).unwrap();
        let encoded = frame.encode();
        let (decoded, consumed) = Frame::decode(&encoded).unwrap();
        assert_eq!(decoded.frame_type, FrameType::Command);
        assert_eq!(decoded.payload, b"hello");
        assert_eq!(consumed, encoded.len());
    }

    /// Frame too large is rejected.
    #[test]
    fn test_frame_too_large() {
        let big_payload = vec![0u8; MAX_FRAME_SIZE + 1];
        assert!(matches!(
            Frame::new(FrameType::Data, big_payload),
            Err(ProtocolError::FrameTooLarge { .. })
        ));
    }

    /// Unknown frame type is rejected.
    #[test]
    fn test_unknown_frame_type() {
        let data = [0xFF, 0, 0, 0, 0]; // type 0xFF is invalid
        assert!(matches!(
            Frame::decode(&data),
            Err(ProtocolError::UnknownFrameType(0xFF))
        ));
    }

    /// Handshake validation checks protocol version.
    #[test]
    fn test_handshake_validation() {
        let frame = handshake_frame(PROTOCOL_VERSION);
        assert!(validate_handshake(&frame).is_ok());

        let bad = handshake_frame(99);
        assert!(matches!(
            validate_handshake(&bad),
            Err(ProtocolError::VersionMismatch { .. })
        ));
    }

    /// FrameType::expects_response.
    #[test]
    fn test_expects_response() {
        assert!(FrameType::Command.expects_response());
        assert!(FrameType::Ping.expects_response());
        assert!(!FrameType::Pong.expects_response());
        assert!(!FrameType::Data.expects_response());
    }

    /// Ping/Pong frames.
    #[test]
    fn test_ping_pong() {
        let ping = Frame::ping();
        let pong = Frame::pong();
        assert_eq!(ping.frame_type, FrameType::Ping);
        assert_eq!(pong.frame_type, FrameType::Pong);
        assert!(ping.payload.is_empty());
    }

    /// FrameType::is_control.
    #[test]
    fn test_is_control() {
        assert!(FrameType::Ping.is_control());
        assert!(FrameType::Command.is_control());
        assert!(!FrameType::Data.is_control());
    }

    /// Truncated data is rejected.
    #[test]
    fn test_truncated_data() {
        let data = [0x01, 0xFF, 0, 0, 0]; // handshake with large length
        assert!(matches!(
            Frame::decode(&data),
            Err(ProtocolError::Truncated { .. })
        ));
    }

    /// Empty handshake payload is rejected.
    #[test]
    fn test_empty_handshake_rejected() {
        let frame = Frame { frame_type: FrameType::Handshake, payload: Vec::new() };
        assert!(matches!(
            validate_handshake(&frame),
            Err(ProtocolError::InvalidHandshake(_))
        ));
    }

    /// All 10 frame types exist.
    #[test]
    fn test_all_frame_types() {
        for i in 0x01..=0x0Au8 {
            assert!(FrameType::from_byte(i).is_some(), "byte 0x{i:02X} should be valid");
        }
        assert!(FrameType::from_byte(0x00).is_none());
        assert!(FrameType::from_byte(0x0B).is_none());
    }
}
