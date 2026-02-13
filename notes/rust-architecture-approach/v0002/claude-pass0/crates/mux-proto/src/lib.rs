//! # mux-proto
//!
//! TF01 wire protocol and tmux imsg compatibility layer.
//!
//! ## TF01 Header Layout (v25 DEFINITIVE)
//!
//! ```text
//! Offset  Size  Field         Description
//! 0       4     magic         "TF01" (0x54, 0x46, 0x30, 0x31)
//! 4       2     version       Protocol version (network byte order). v1 = 1.
//! 6       2     flags         Reserved. MUST be 0 for v1.
//! 8       4     frame_length  Total frame length INCLUDING header (network order).
//!                             Minimum: 16. Maximum: 16 MiB.
//! 12      4     frame_type    Frame type identifier (network order).
//! 16      ...   payload       frame_length - 16 bytes.
//! ```
//!
//! ## Discrimination Safety
//! TF01 magic byte 0x54 exceeds all tmux message types (max 307).
//! In little-endian, types <= 307 have first byte <= 0x33.
//! In big-endian, first byte is 0x00 for types < 256.
//! Discrimination is safe in 1 byte; 4 bytes used for defense in depth.

#![forbid(unsafe_code)]

use bytes::{Buf, BufMut, BytesMut};

/// TF01 magic bytes: "TF01".
pub const TF01_MAGIC: [u8; 4] = [0x54, 0x46, 0x30, 0x31];

/// TF01 header size in bytes.
pub const TF01_HEADER_SIZE: usize = 16;

/// Maximum frame size: 16 MiB.
pub const TF01_MAX_FRAME_SIZE: u32 = 16 * 1024 * 1024;

/// Minimum frame size: header only (no payload).
pub const TF01_MIN_FRAME_SIZE: u32 = TF01_HEADER_SIZE as u32;

/// Current protocol version.
pub const TF01_VERSION: u16 = 1;

/// TF01 frame header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tf01Header {
    /// Protocol version.
    pub version: u16,
    /// Flags (reserved, must be 0 for v1).
    pub flags: u16,
    /// Total frame length including header.
    pub frame_length: u32,
    /// Frame type identifier.
    pub frame_type: u32,
}

/// Frame types for the TF01 protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum FrameType {
    /// Ping / keepalive.
    Ping = 0x0001,
    /// Pong / keepalive response.
    Pong = 0x0002,
    /// Client identification.
    Identify = 0x0010,
    /// Server ready acknowledgment.
    Ready = 0x0011,
    /// Key input from client.
    KeyInput = 0x0020,
    /// Mouse input from client.
    MouseInput = 0x0021,
    /// Resize notification.
    Resize = 0x0030,
    /// Grid snapshot / render frame.
    RenderFrame = 0x0040,
    /// Command execution request.
    Command = 0x0050,
    /// Command response.
    CommandResponse = 0x0051,
    /// Grid data (PTY output forwarding).
    GridData = 0x0060,
    /// Session/window/pane state update.
    StateUpdate = 0x0070,
    /// Error message.
    Error = 0xFF00,
}

impl FrameType {
    /// Convert from u32 to FrameType.
    #[must_use]
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            0x0001 => Some(Self::Ping),
            0x0002 => Some(Self::Pong),
            0x0010 => Some(Self::Identify),
            0x0011 => Some(Self::Ready),
            0x0020 => Some(Self::KeyInput),
            0x0021 => Some(Self::MouseInput),
            0x0030 => Some(Self::Resize),
            0x0040 => Some(Self::RenderFrame),
            0x0050 => Some(Self::Command),
            0x0051 => Some(Self::CommandResponse),
            0x0060 => Some(Self::GridData),
            0x0070 => Some(Self::StateUpdate),
            0xFF00 => Some(Self::Error),
            _ => None,
        }
    }
}

/// Errors during protocol operations.
#[derive(Debug, thiserror::Error)]
pub enum ProtoError {
    #[error("invalid magic: expected TF01")]
    InvalidMagic,
    #[error("unsupported version: {0}")]
    UnsupportedVersion(u16),
    #[error("frame too large: {0} bytes (max {TF01_MAX_FRAME_SIZE})")]
    FrameTooLarge(u32),
    #[error("frame too small: {0} bytes (min {TF01_MIN_FRAME_SIZE})")]
    FrameTooSmall(u32),
    #[error("unknown frame type: 0x{0:04X}")]
    UnknownFrameType(u32),
    #[error("incomplete frame: need {need} bytes, have {have}")]
    Incomplete { need: usize, have: usize },
    #[error("invalid flags: 0x{0:04X}")]
    InvalidFlags(u16),
    #[error("imsg protocol error: {0}")]
    ImsgError(String),
}

/// A decoded TF01 frame.
#[derive(Debug, Clone)]
pub struct Frame {
    /// Frame header.
    pub header: Tf01Header,
    /// Frame payload (may be empty).
    pub payload: Vec<u8>,
}

/// Encode a TF01 frame header into a buffer.
pub fn encode_header(header: &Tf01Header, buf: &mut BytesMut) {
    buf.put_slice(&TF01_MAGIC);
    buf.put_u16(header.version);
    buf.put_u16(header.flags);
    buf.put_u32(header.frame_length);
    buf.put_u32(header.frame_type);
}

/// Encode a complete TF01 frame (header + payload).
pub fn encode_frame(frame_type: FrameType, payload: &[u8], buf: &mut BytesMut) {
    let frame_length = TF01_HEADER_SIZE as u32 + payload.len() as u32;
    let header = Tf01Header {
        version: TF01_VERSION,
        flags: 0,
        frame_length,
        frame_type: frame_type as u32,
    };
    encode_header(&header, buf);
    buf.put_slice(payload);
}

/// Decode a TF01 frame header from a buffer.
///
/// Returns the header and advances the buffer past the header bytes.
///
/// # Errors
/// Returns error if the buffer is too small, magic is wrong, or fields invalid.
pub fn decode_header(buf: &mut &[u8]) -> Result<Tf01Header, ProtoError> {
    if buf.len() < TF01_HEADER_SIZE {
        return Err(ProtoError::Incomplete {
            need: TF01_HEADER_SIZE,
            have: buf.len(),
        });
    }

    let magic = &buf[0..4];
    if magic != TF01_MAGIC {
        return Err(ProtoError::InvalidMagic);
    }

    let version = u16::from_be_bytes([buf[4], buf[5]]);
    let flags = u16::from_be_bytes([buf[6], buf[7]]);
    let frame_length = u32::from_be_bytes([buf[8], buf[9], buf[10], buf[11]]);
    let frame_type = u32::from_be_bytes([buf[12], buf[13], buf[14], buf[15]]);

    if version != TF01_VERSION {
        return Err(ProtoError::UnsupportedVersion(version));
    }

    if flags != 0 {
        return Err(ProtoError::InvalidFlags(flags));
    }

    if frame_length < TF01_MIN_FRAME_SIZE {
        return Err(ProtoError::FrameTooSmall(frame_length));
    }

    if frame_length > TF01_MAX_FRAME_SIZE {
        return Err(ProtoError::FrameTooLarge(frame_length));
    }

    *buf = &buf[TF01_HEADER_SIZE..];

    Ok(Tf01Header {
        version,
        flags,
        frame_length,
        frame_type,
    })
}

/// Decode a complete frame from a buffer.
///
/// # Errors
/// Returns error if the buffer does not contain a complete frame.
pub fn decode_frame(buf: &mut &[u8]) -> Result<Frame, ProtoError> {
    let start = *buf;
    let header = decode_header(buf)?;

    let payload_len = (header.frame_length as usize) - TF01_HEADER_SIZE;
    if buf.len() < payload_len {
        // Reset buffer position
        *buf = start;
        return Err(ProtoError::Incomplete {
            need: header.frame_length as usize,
            have: start.len(),
        });
    }

    let payload = buf[..payload_len].to_vec();
    *buf = &buf[payload_len..];

    Ok(Frame { header, payload })
}

/// Discriminate between TF01 and tmux imsg protocols.
///
/// Returns `true` if the data starts with a TF01 frame,
/// `false` if it looks like a tmux imsg frame.
///
/// # Safety proof
/// TF01 magic first byte is 0x54 (84 decimal).
/// tmux message types range from 12 to 307.
/// In little-endian: types <= 307 have first byte <= 0x33 (51).
/// In big-endian: first byte is 0x00 for types < 256, 0x01 for 256-307.
/// 0x54 never appears as first byte of valid imsg.
#[must_use]
pub fn is_tf01(data: &[u8]) -> bool {
    data.len() >= 4 && data[0..4] == TF01_MAGIC
}

/// tmux imsg header structure (for compatibility).
///
/// ```c
/// struct imsg_hdr {
///     uint32_t type;     // 12-307 for tmux
///     uint32_t len;
///     uint32_t peerid;
///     uint32_t pid;
/// };  // 16 bytes total
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImsgHeader {
    /// Message type (12-307 for tmux).
    pub msg_type: u32,
    /// Total message length including header.
    pub len: u32,
    /// Peer ID.
    pub peerid: u32,
    /// Process ID.
    pub pid: u32,
}

/// Imsg header size.
pub const IMSG_HEADER_SIZE: usize = 16;

/// Decode a tmux imsg header.
///
/// # Errors
/// Returns error on insufficient data.
pub fn decode_imsg_header(buf: &[u8]) -> Result<ImsgHeader, ProtoError> {
    if buf.len() < IMSG_HEADER_SIZE {
        return Err(ProtoError::Incomplete {
            need: IMSG_HEADER_SIZE,
            have: buf.len(),
        });
    }

    // imsg uses host byte order (little-endian on x86)
    let msg_type = u32::from_ne_bytes([buf[0], buf[1], buf[2], buf[3]]);
    let len = u32::from_ne_bytes([buf[4], buf[5], buf[6], buf[7]]);
    let peerid = u32::from_ne_bytes([buf[8], buf[9], buf[10], buf[11]]);
    let pid = u32::from_ne_bytes([buf[12], buf[13], buf[14], buf[15]]);

    Ok(ImsgHeader {
        msg_type,
        len,
        peerid,
        pid,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tf01_magic_discrimination() {
        assert!(is_tf01(&TF01_MAGIC));
        assert!(!is_tf01(&[0x00, 0x00, 0x00, 0x0C])); // tmux type 12
        assert!(!is_tf01(&[0x33, 0x01, 0x00, 0x00])); // tmux type 307 LE
    }

    #[test]
    fn tf01_magic_first_byte_exceeds_tmux_max() {
        // TF01 first byte is 0x54 = 84
        // tmux max type in LE first byte is 0x33 = 51
        assert!(TF01_MAGIC[0] > 0x33);
    }

    #[test]
    fn encode_decode_roundtrip() {
        let mut buf = BytesMut::new();
        encode_frame(FrameType::Ping, b"hello", &mut buf);

        let mut slice: &[u8] = &buf;
        let frame = decode_frame(&mut slice);
        assert!(frame.is_ok());
        let frame = frame.ok();
        assert_eq!(frame.as_ref().map(|f| f.header.frame_type), Some(FrameType::Ping as u32));
        assert_eq!(frame.as_ref().map(|f| f.payload.as_slice()), Some(b"hello".as_slice()));
    }

    #[test]
    fn header_size_is_16() {
        assert_eq!(TF01_HEADER_SIZE, 16);
        assert_eq!(IMSG_HEADER_SIZE, 16);
    }

    #[test]
    fn reject_invalid_magic() {
        let bad_data = [0x00u8; 16];
        let mut slice: &[u8] = &bad_data;
        let result = decode_header(&mut slice);
        assert!(result.is_err());
    }

    #[test]
    fn reject_oversized_frame() {
        let mut buf = BytesMut::new();
        let header = Tf01Header {
            version: TF01_VERSION,
            flags: 0,
            frame_length: TF01_MAX_FRAME_SIZE + 1,
            frame_type: FrameType::Ping as u32,
        };
        encode_header(&header, &mut buf);
        let mut slice: &[u8] = &buf;
        let result = decode_header(&mut slice);
        assert!(matches!(result, Err(ProtoError::FrameTooLarge(_))));
    }
}
