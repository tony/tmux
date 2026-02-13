//! # mux-proto
//!
//! TF01 wire protocol and tmux imsg compatibility layer.
//!
//! ## TF01 Frame Format
//! ```text
//! [0..4]  magic: 0x54464F31 ("TFO1")
//! [4..6]  type:  u16 LE
//! [6..8]  flags: u16 LE
//! [8..12] len:   u32 LE (payload length)
//! [12..16] crc:  CRC32 of payload
//! [16..16+len] payload
//! ```
//!
//! ## SCM_RIGHTS (fd passing)
//! For tmux compatibility mode, fd passing uses `nix::sys::socket::sendmsg/recvmsg`
//! with `ControlMessage::ScmRights`. Not used in TF01 mode.
//! See architecture.md section 15 for SCM_RIGHTS architecture.

#![forbid(unsafe_code)]

use bytes::{Buf, BufMut, BytesMut};

/// TF01 protocol magic bytes: "TFO1".
pub const TF01_MAGIC: u32 = 0x5446_4F31;

/// Header size in bytes.
pub const HEADER_SIZE: usize = 16;

/// Maximum payload size (16 MiB).
pub const MAX_PAYLOAD_SIZE: usize = 16 * 1024 * 1024;

/// TF01 frame types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    #[must_use]
    pub const fn from_u16(v: u16) -> Option<Self> {
        match v {
            0x0001 => Some(Self::Hello),
            0x0002 => Some(Self::Data),
            0x0003 => Some(Self::Resize),
            0x0004 => Some(Self::Command),
            0x0005 => Some(Self::CommandResponse),
            0x0006 => Some(Self::KeyInput),
            0x0007 => Some(Self::Shutdown),
            0x00FF => Some(Self::Error),
            _ => None,
        }
    }
}

/// A parsed TF01 frame.
#[derive(Debug, Clone)]
pub struct Frame {
    pub frame_type: FrameType,
    pub flags: u16,
    pub payload: Vec<u8>,
}

/// Protocol errors.
#[derive(Debug, thiserror::Error)]
pub enum ProtoError {
    #[error("invalid magic: expected 0x{TF01_MAGIC:08X}, got 0x{0:08X}")]
    InvalidMagic(u32),
    #[error("unknown frame type: 0x{0:04X}")]
    UnknownType(u16),
    #[error("payload too large: {0} bytes (max {MAX_PAYLOAD_SIZE})")]
    PayloadTooLarge(usize),
    #[error("CRC mismatch: expected 0x{expected:08X}, got 0x{actual:08X}")]
    CrcMismatch { expected: u32, actual: u32 },
    #[error("incomplete frame: need {0} more bytes")]
    Incomplete(usize),
}

/// Check if a byte slice starts with the TF01 magic.
#[must_use]
pub fn is_tf01(data: &[u8]) -> bool {
    data.len() >= 4
        && data[0] == 0x54
        && data[1] == 0x46
        && data[2] == 0x4F
        && data[3] == 0x31
}

/// Encode a frame into bytes.
#[must_use]
pub fn encode_frame(frame: &Frame) -> Vec<u8> {
    let payload_len = frame.payload.len();
    let crc = crc32fast::hash(&frame.payload);

    let mut buf = Vec::with_capacity(HEADER_SIZE + payload_len);
    buf.put_u32_le(TF01_MAGIC);
    buf.put_u16_le(frame.frame_type as u16);
    buf.put_u16_le(frame.flags);
    buf.put_u32_le(payload_len as u32);
    buf.put_u32_le(crc);
    buf.extend_from_slice(&frame.payload);
    buf
}

/// Decode a frame from a byte buffer.
///
/// # Errors
/// Returns error if the data is incomplete, corrupt, or invalid.
pub fn decode_frame(data: &mut BytesMut) -> Result<Option<Frame>, ProtoError> {
    if data.len() < HEADER_SIZE {
        return Ok(None);
    }

    let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    if magic != TF01_MAGIC {
        return Err(ProtoError::InvalidMagic(magic));
    }

    let type_val = u16::from_le_bytes([data[4], data[5]]);
    let flags = u16::from_le_bytes([data[6], data[7]]);
    let payload_len = u32::from_le_bytes([data[8], data[9], data[10], data[11]]) as usize;
    let expected_crc = u32::from_le_bytes([data[12], data[13], data[14], data[15]]);

    if payload_len > MAX_PAYLOAD_SIZE {
        return Err(ProtoError::PayloadTooLarge(payload_len));
    }

    let total_len = HEADER_SIZE + payload_len;
    if data.len() < total_len {
        return Ok(None); // Need more data
    }

    let frame_type = FrameType::from_u16(type_val)
        .ok_or(ProtoError::UnknownType(type_val))?;

    let payload = data[HEADER_SIZE..total_len].to_vec();
    let actual_crc = crc32fast::hash(&payload);

    if actual_crc != expected_crc {
        return Err(ProtoError::CrcMismatch {
            expected: expected_crc,
            actual: actual_crc,
        });
    }

    data.advance(total_len);

    Ok(Some(Frame {
        frame_type,
        flags,
        payload,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tf01_magic_detection() {
        assert!(is_tf01(&[0x54, 0x46, 0x4F, 0x31]));
        assert!(!is_tf01(&[0x00, 0x00, 0x00, 0x00]));
        assert!(!is_tf01(&[0x54]));
    }

    #[test]
    fn encode_decode_roundtrip() {
        let frame = Frame {
            frame_type: FrameType::Data,
            flags: 0,
            payload: b"hello world".to_vec(),
        };
        let encoded = encode_frame(&frame);
        let mut buf = BytesMut::from(&encoded[..]);
        let decoded = decode_frame(&mut buf);
        assert!(decoded.is_ok());
        let decoded = decoded.unwrap_or(None);
        assert!(decoded.is_some());
        let decoded = decoded.unwrap_or_else(|| Frame { frame_type: FrameType::Error, flags: 0, payload: vec![] });
        assert_eq!(decoded.frame_type, FrameType::Data);
        assert_eq!(decoded.payload, b"hello world");
    }

    #[test]
    fn incomplete_frame() {
        let mut buf = BytesMut::from(&[0x54u8, 0x46, 0x4F, 0x31][..]);
        let result = decode_frame(&mut buf);
        assert!(matches!(result, Ok(None)));
    }

    #[test]
    fn invalid_magic() {
        let mut data = BytesMut::from(&[0u8; HEADER_SIZE][..]);
        let result = decode_frame(&mut data);
        assert!(matches!(result, Err(ProtoError::InvalidMagic(_))));
    }

    #[test]
    fn crc_validation() {
        let frame = Frame {
            frame_type: FrameType::Command,
            flags: 0,
            payload: b"test".to_vec(),
        };
        let mut encoded = encode_frame(&frame);
        // Corrupt the payload
        if let Some(last) = encoded.last_mut() {
            *last ^= 0xFF;
        }
        let mut buf = BytesMut::from(&encoded[..]);
        let result = decode_frame(&mut buf);
        assert!(matches!(result, Err(ProtoError::CrcMismatch { .. })));
    }

    #[test]
    fn frame_type_roundtrip() {
        for &ft in &[
            FrameType::Hello, FrameType::Data, FrameType::Resize,
            FrameType::Command, FrameType::CommandResponse,
            FrameType::KeyInput, FrameType::Shutdown, FrameType::Error,
        ] {
            assert_eq!(FrameType::from_u16(ft as u16), Some(ft));
        }
    }

    #[test]
    fn unknown_frame_type() {
        assert!(FrameType::from_u16(0xFFFF).is_none());
    }

    #[test]
    fn empty_payload_roundtrip() {
        let frame = Frame { frame_type: FrameType::Shutdown, flags: 0, payload: vec![] };
        let encoded = encode_frame(&frame);
        let mut buf = BytesMut::from(&encoded[..]);
        let decoded = decode_frame(&mut buf);
        assert!(decoded.is_ok());
    }
}
