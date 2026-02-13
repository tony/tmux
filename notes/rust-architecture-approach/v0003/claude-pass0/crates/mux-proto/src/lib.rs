//! # mux-proto
//!
//! TF01 wire protocol for TermForge client-server communication.
//!
//! ## Frame Format
//! ```text
//! [0..4]   u32  magic: 0x54464F31 ("TFO1")
//! [4..6]   u16  type (LE)
//! [6..8]   u16  flags (LE)
//! [8..12]  u32  payload_len (LE)
//! [12..16] u32  CRC32 of payload
//! [16..N]  [u8] payload bytes
//! ```
//!
//! L2 data crate -- no internal dependencies.

#![forbid(unsafe_code)]

use thiserror::Error;

/// TF01 magic number: "TFO1" in big-endian.
pub const MAGIC: u32 = 0x5446_4F31;

/// Header size in bytes.
pub const HEADER_SIZE: usize = 16;

/// Maximum payload size (1 MiB).
pub const MAX_PAYLOAD: u32 = 1 << 20;

/// Protocol errors.
#[derive(Debug, Error)]
pub enum ProtoError {
    /// Invalid magic number.
    #[error("invalid magic: expected 0x{:08X}, got 0x{got:08X}", MAGIC)]
    BadMagic { got: u32 },
    /// CRC32 mismatch.
    #[error("CRC32 mismatch: expected 0x{expected:08X}, got 0x{got:08X}")]
    CrcMismatch { expected: u32, got: u32 },
    /// Payload too large.
    #[error("payload too large: {size} bytes (max {MAX_PAYLOAD})")]
    PayloadTooLarge { size: u32 },
    /// Truncated data.
    #[error("truncated: need {need} bytes, have {have}")]
    Truncated { need: usize, have: usize },
    /// Unknown frame type.
    #[error("unknown frame type: 0x{0:04X}")]
    UnknownType(u16),
}

/// Frame types in the TF01 protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum FrameType {
    Hello = 0x0001,
    Data = 0x0002,
    Resize = 0x0003,
    Command = 0x0004,
    CommandResponse = 0x0005,
    KeyInput = 0x0006,
    Shutdown = 0x0007,
    Error = 0x00FF,
}

impl FrameType {
    /// Parse a u16 into a frame type.
    ///
    /// # Errors
    ///
    /// Returns `ProtoError::UnknownType` for unrecognized values.
    pub const fn from_u16(v: u16) -> Result<Self, u16> {
        match v {
            0x0001 => Ok(Self::Hello),
            0x0002 => Ok(Self::Data),
            0x0003 => Ok(Self::Resize),
            0x0004 => Ok(Self::Command),
            0x0005 => Ok(Self::CommandResponse),
            0x0006 => Ok(Self::KeyInput),
            0x0007 => Ok(Self::Shutdown),
            0x00FF => Ok(Self::Error),
            _ => Err(v),
        }
    }
}

/// A decoded TF01 frame.
#[derive(Debug, Clone)]
pub struct Frame {
    /// Frame type.
    pub frame_type: FrameType,
    /// Frame flags.
    pub flags: u16,
    /// Payload data.
    pub payload: Vec<u8>,
}

impl Frame {
    /// Create a new frame.
    #[must_use]
    pub fn new(frame_type: FrameType, payload: Vec<u8>) -> Self {
        Self {
            frame_type,
            flags: 0,
            payload,
        }
    }

    /// Encode the frame to bytes.
    ///
    /// # Errors
    ///
    /// Returns `ProtoError::PayloadTooLarge` if payload exceeds `MAX_PAYLOAD`.
    pub fn encode(&self) -> Result<Vec<u8>, ProtoError> {
        let payload_len = u32::try_from(self.payload.len()).unwrap_or(u32::MAX);
        if payload_len > MAX_PAYLOAD {
            return Err(ProtoError::PayloadTooLarge { size: payload_len });
        }

        let crc = crc32fast::hash(&self.payload);

        let mut buf = Vec::with_capacity(HEADER_SIZE + self.payload.len());
        buf.extend_from_slice(&MAGIC.to_le_bytes());
        buf.extend_from_slice(&(self.frame_type as u16).to_le_bytes());
        buf.extend_from_slice(&self.flags.to_le_bytes());
        buf.extend_from_slice(&payload_len.to_le_bytes());
        buf.extend_from_slice(&crc.to_le_bytes());
        buf.extend_from_slice(&self.payload);

        Ok(buf)
    }

    /// Decode a frame from bytes.
    ///
    /// # Errors
    ///
    /// Returns protocol errors for invalid data.
    pub fn decode(data: &[u8]) -> Result<Self, ProtoError> {
        if data.len() < HEADER_SIZE {
            return Err(ProtoError::Truncated {
                need: HEADER_SIZE,
                have: data.len(),
            });
        }

        let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        if magic != MAGIC {
            return Err(ProtoError::BadMagic { got: magic });
        }

        let type_val = u16::from_le_bytes([data[4], data[5]]);
        let flags = u16::from_le_bytes([data[6], data[7]]);
        let payload_len = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
        let expected_crc = u32::from_le_bytes([data[12], data[13], data[14], data[15]]);

        if payload_len > MAX_PAYLOAD {
            return Err(ProtoError::PayloadTooLarge { size: payload_len });
        }

        let total = HEADER_SIZE + payload_len as usize;
        if data.len() < total {
            return Err(ProtoError::Truncated {
                need: total,
                have: data.len(),
            });
        }

        let payload = data[HEADER_SIZE..total].to_vec();
        let actual_crc = crc32fast::hash(&payload);
        if actual_crc != expected_crc {
            return Err(ProtoError::CrcMismatch {
                expected: expected_crc,
                got: actual_crc,
            });
        }

        let frame_type =
            FrameType::from_u16(type_val).map_err(|v| ProtoError::UnknownType(v))?;

        Ok(Self {
            frame_type,
            flags,
            payload,
        })
    }
}

/// Detect whether a byte slice starts with a TF01 frame.
/// Used for protocol auto-detection (TF01 vs tmux imsg).
#[must_use]
pub fn is_tf01_frame(data: &[u8]) -> bool {
    if data.len() < 4 {
        return false;
    }
    let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    magic == MAGIC
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_roundtrip() {
        let frame = Frame::new(FrameType::Command, b"new-session".to_vec());
        let encoded = frame.encode().unwrap_or_default();
        let decoded = Frame::decode(&encoded).unwrap_or_else(|_| Frame::new(FrameType::Error, Vec::new()));
        assert_eq!(decoded.frame_type, FrameType::Command);
        assert_eq!(decoded.payload, b"new-session");
    }

    #[test]
    fn encode_decode_empty_payload() {
        let frame = Frame::new(FrameType::Shutdown, Vec::new());
        let encoded = frame.encode().unwrap_or_default();
        let decoded = Frame::decode(&encoded).unwrap_or_else(|_| Frame::new(FrameType::Error, Vec::new()));
        assert_eq!(decoded.frame_type, FrameType::Shutdown);
        assert!(decoded.payload.is_empty());
    }

    #[test]
    fn detect_tf01_frame() {
        let frame = Frame::new(FrameType::Hello, Vec::new());
        let encoded = frame.encode().unwrap_or_default();
        assert!(is_tf01_frame(&encoded));
    }

    #[test]
    fn detect_non_tf01() {
        assert!(!is_tf01_frame(b"not a frame"));
        assert!(!is_tf01_frame(b""));
        assert!(!is_tf01_frame(b"abc"));
    }

    #[test]
    fn decode_truncated() {
        assert!(Frame::decode(b"short").is_err());
    }

    #[test]
    fn decode_bad_magic() {
        let mut data = [0u8; 16];
        data[0..4].copy_from_slice(&0xDEAD_BEEFu32.to_le_bytes());
        assert!(matches!(
            Frame::decode(&data),
            Err(ProtoError::BadMagic { .. })
        ));
    }

    #[test]
    fn decode_crc_mismatch() {
        let frame = Frame::new(FrameType::Data, b"test".to_vec());
        let mut encoded = frame.encode().unwrap_or_default();
        // Corrupt payload
        if encoded.len() > HEADER_SIZE {
            encoded[HEADER_SIZE] ^= 0xFF;
        }
        assert!(matches!(
            Frame::decode(&encoded),
            Err(ProtoError::CrcMismatch { .. })
        ));
    }

    #[test]
    fn frame_type_roundtrip() {
        for ft in [
            FrameType::Hello,
            FrameType::Data,
            FrameType::Resize,
            FrameType::Command,
            FrameType::CommandResponse,
            FrameType::KeyInput,
            FrameType::Shutdown,
            FrameType::Error,
        ] {
            assert_eq!(FrameType::from_u16(ft as u16), Ok(ft));
        }
    }

    #[test]
    fn unknown_frame_type() {
        assert!(FrameType::from_u16(0xBEEF).is_err());
    }

    #[test]
    fn header_size() {
        assert_eq!(HEADER_SIZE, 16);
    }
}
