//! TF01 wire protocol implementation.
//!
//! Frame format: magic(4) + type(2) + flags(2) + payload_len(4) + crc32(4) + payload(N).
//! Supports auto-detection between TF01 and tmux imsg format.

#![forbid(unsafe_code)]

use thiserror::Error;

/// TF01 magic bytes: "TFO1" = 0x54464F31.
pub const TF01_MAGIC: u32 = 0x5446_4F31;

/// Minimum frame size (header only, no payload).
pub const HEADER_SIZE: usize = 16;

/// Protocol errors.
#[derive(Debug, Error)]
pub enum ProtoError {
    /// Frame too short.
    #[error("frame too short: {0} bytes (minimum {HEADER_SIZE})")]
    TooShort(usize),
    /// Invalid magic bytes.
    #[error("invalid magic: expected 0x{TF01_MAGIC:08X}, got 0x{0:08X}")]
    InvalidMagic(u32),
    /// CRC mismatch.
    #[error("CRC mismatch: expected 0x{expected:08X}, got 0x{actual:08X}")]
    CrcMismatch { expected: u32, actual: u32 },
    /// Unknown frame type.
    #[error("unknown frame type: 0x{0:04X}")]
    UnknownType(u16),
    /// Payload too large.
    #[error("payload too large: {0} bytes")]
    PayloadTooLarge(u32),
}

/// Frame types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum FrameType {
    /// Client hello with capabilities.
    Hello = 0x0001,
    /// Rendered output data.
    Data = 0x0002,
    /// Client terminal size change.
    Resize = 0x0003,
    /// tmux command string.
    Command = 0x0004,
    /// Command result.
    CommandResponse = 0x0005,
    /// Key/mouse input event.
    KeyInput = 0x0006,
    /// Graceful shutdown request.
    Shutdown = 0x0007,
    /// Protocol-level error.
    Error = 0x00FF,
}

impl FrameType {
    /// Convert from u16.
    pub fn from_u16(v: u16) -> Result<Self, ProtoError> {
        match v {
            0x0001 => Ok(Self::Hello),
            0x0002 => Ok(Self::Data),
            0x0003 => Ok(Self::Resize),
            0x0004 => Ok(Self::Command),
            0x0005 => Ok(Self::CommandResponse),
            0x0006 => Ok(Self::KeyInput),
            0x0007 => Ok(Self::Shutdown),
            0x00FF => Ok(Self::Error),
            _ => Err(ProtoError::UnknownType(v)),
        }
    }
}

/// A TF01 frame header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameHeader {
    /// Frame type.
    pub frame_type: FrameType,
    /// Frame flags.
    pub flags: u16,
    /// Payload length.
    pub payload_len: u32,
    /// CRC32 of payload.
    pub crc32: u32,
}

/// A complete TF01 frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    /// Frame header.
    pub header: FrameHeader,
    /// Payload bytes.
    pub payload: Vec<u8>,
}

impl Frame {
    /// Create a new frame with the given type and payload.
    pub fn new(frame_type: FrameType, payload: Vec<u8>) -> Self {
        let crc = crc32fast::hash(&payload);
        Self {
            header: FrameHeader {
                frame_type,
                flags: 0,
                payload_len: payload.len() as u32,
                crc32: crc,
            },
            payload,
        }
    }

    /// Encode this frame into bytes.
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(HEADER_SIZE + self.payload.len());
        buf.extend_from_slice(&TF01_MAGIC.to_le_bytes());
        buf.extend_from_slice(&(self.header.frame_type as u16).to_le_bytes());
        buf.extend_from_slice(&self.header.flags.to_le_bytes());
        buf.extend_from_slice(&self.header.payload_len.to_le_bytes());
        buf.extend_from_slice(&self.header.crc32.to_le_bytes());
        buf.extend_from_slice(&self.payload);
        buf
    }

    /// Decode a frame from bytes.
    pub fn decode(data: &[u8]) -> Result<Self, ProtoError> {
        if data.len() < HEADER_SIZE {
            return Err(ProtoError::TooShort(data.len()));
        }

        let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        if magic != TF01_MAGIC {
            return Err(ProtoError::InvalidMagic(magic));
        }

        let frame_type_raw = u16::from_le_bytes([data[4], data[5]]);
        let frame_type = FrameType::from_u16(frame_type_raw)?;
        let flags = u16::from_le_bytes([data[6], data[7]]);
        let payload_len = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
        let crc32 = u32::from_le_bytes([data[12], data[13], data[14], data[15]]);

        let expected_total = HEADER_SIZE + payload_len as usize;
        if data.len() < expected_total {
            return Err(ProtoError::TooShort(data.len()));
        }

        let payload = data[HEADER_SIZE..expected_total].to_vec();
        let actual_crc = crc32fast::hash(&payload);
        if actual_crc != crc32 {
            return Err(ProtoError::CrcMismatch {
                expected: crc32,
                actual: actual_crc,
            });
        }

        Ok(Self {
            header: FrameHeader {
                frame_type,
                flags,
                payload_len,
                crc32,
            },
            payload,
        })
    }
}

/// Check if a byte slice starts with the TF01 magic.
pub fn is_tf01_frame(data: &[u8]) -> bool {
    if data.len() < 4 {
        return false;
    }
    let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    magic == TF01_MAGIC
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_encode_decode_roundtrip() {
        let frame = Frame::new(FrameType::Hello, b"hello".to_vec());
        let encoded = frame.encode();
        let decoded = Frame::decode(&encoded).unwrap_or_else(|_| Frame::new(FrameType::Error, vec![]));
        assert_eq!(decoded, frame);
    }

    #[test]
    fn frame_empty_payload() {
        let frame = Frame::new(FrameType::Shutdown, vec![]);
        let encoded = frame.encode();
        let decoded = Frame::decode(&encoded).unwrap_or_else(|_| Frame::new(FrameType::Error, vec![]));
        assert_eq!(decoded.header.frame_type, FrameType::Shutdown);
    }

    #[test]
    fn too_short() {
        assert!(Frame::decode(b"short").is_err());
    }

    #[test]
    fn invalid_magic() {
        let mut data = Frame::new(FrameType::Hello, vec![]).encode();
        data[0] = 0xFF;
        assert!(Frame::decode(&data).is_err());
    }

    #[test]
    fn crc_mismatch() {
        let mut data = Frame::new(FrameType::Hello, b"test".to_vec()).encode();
        // Corrupt payload
        if data.len() > HEADER_SIZE {
            data[HEADER_SIZE] ^= 0xFF;
        }
        let result = Frame::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn is_tf01_frame_check() {
        let data = Frame::new(FrameType::Hello, vec![]).encode();
        assert!(is_tf01_frame(&data));
        assert!(!is_tf01_frame(b"not"));
        assert!(!is_tf01_frame(b"sh"));
    }

    #[test]
    fn frame_types() {
        assert!(FrameType::from_u16(0x0001).is_ok());
        assert!(FrameType::from_u16(0x0002).is_ok());
        assert!(FrameType::from_u16(0x00FF).is_ok());
        assert!(FrameType::from_u16(0xFFFF).is_err());
    }

    #[test]
    fn header_size_constant() {
        assert_eq!(HEADER_SIZE, 16);
    }

    #[test]
    fn magic_constant() {
        assert_eq!(TF01_MAGIC, 0x5446_4F31);
    }

    #[test]
    fn frame_data_type() {
        let frame = Frame::new(FrameType::Data, b"content".to_vec());
        assert_eq!(frame.header.frame_type, FrameType::Data);
        assert_eq!(frame.header.payload_len, 7);
    }

    #[test]
    fn frame_crc_computed() {
        let frame = Frame::new(FrameType::Hello, b"test".to_vec());
        assert_ne!(frame.header.crc32, 0);
    }

    #[test]
    fn resize_frame() {
        let frame = Frame::new(FrameType::Resize, vec![80, 0, 24, 0]);
        let encoded = frame.encode();
        let decoded = Frame::decode(&encoded).unwrap_or_else(|_| Frame::new(FrameType::Error, vec![]));
        assert_eq!(decoded.header.frame_type, FrameType::Resize);
    }

    #[test]
    fn command_frame() {
        let payload = b"new-session -s dev".to_vec();
        let frame = Frame::new(FrameType::Command, payload.clone());
        let encoded = frame.encode();
        let decoded = Frame::decode(&encoded).unwrap_or_else(|_| Frame::new(FrameType::Error, vec![]));
        assert_eq!(decoded.payload, payload);
    }

    #[test]
    fn key_input_frame() {
        let frame = Frame::new(FrameType::KeyInput, vec![0x1B, b'[', b'A']);
        assert_eq!(frame.header.frame_type, FrameType::KeyInput);
    }

    #[test]
    fn large_payload() {
        let payload = vec![0u8; 4096];
        let frame = Frame::new(FrameType::Data, payload);
        let encoded = frame.encode();
        let decoded = Frame::decode(&encoded).unwrap_or_else(|_| Frame::new(FrameType::Error, vec![]));
        assert_eq!(decoded.header.payload_len, 4096);
    }

    mod prop {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn encode_decode_roundtrip(payload in proptest::collection::vec(0u8..=255, 0..500)) {
                let frame = Frame::new(FrameType::Data, payload);
                let encoded = frame.encode();
                let decoded = Frame::decode(&encoded);
                prop_assert!(decoded.is_ok());
                prop_assert_eq!(decoded.unwrap_or_else(|_| Frame::new(FrameType::Error, vec![])), frame);
            }

            #[test]
            fn crc_validates(payload in proptest::collection::vec(0u8..=255, 1..100)) {
                let frame = Frame::new(FrameType::Command, payload);
                let expected_crc = crc32fast::hash(&frame.payload);
                prop_assert_eq!(frame.header.crc32, expected_crc);
            }
        }
    }
}
