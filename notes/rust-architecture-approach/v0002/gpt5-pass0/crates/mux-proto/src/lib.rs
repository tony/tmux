use bytes::{Buf, BufMut, Bytes, BytesMut};
use thiserror::Error;

pub const TF01_MAGIC: [u8; 4] = *b"TF01";
pub const TF01_HEADER_LEN: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum FrameType {
    Command = 1,
    Data = 2,
    Render = 3,
    Event = 4,
}

impl TryFrom<u32> for FrameType {
    type Error = ProtoError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Command),
            2 => Ok(Self::Data),
            3 => Ok(Self::Render),
            4 => Ok(Self::Event),
            other => Err(ProtoError::UnknownFrameType(other)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tf01Header {
    pub version: u16,
    pub flags: u16,
    pub frame_length: u32,
    pub frame_type: FrameType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub header: Tf01Header,
    pub payload: Bytes,
}

#[derive(Debug, Error)]
pub enum ProtoError {
    #[error("buffer too short: expected at least {expected}, got {actual}")]
    BufferTooShort { expected: usize, actual: usize },
    #[error("invalid magic bytes")]
    BadMagic,
    #[error("unknown frame type: {0}")]
    UnknownFrameType(u32),
    #[error("frame length mismatch: header={header}, actual={actual}")]
    LengthMismatch { header: u32, actual: usize },
}

impl Tf01Header {
    pub fn decode(src: &[u8]) -> Result<Self, ProtoError> {
        if src.len() < TF01_HEADER_LEN {
            return Err(ProtoError::BufferTooShort {
                expected: TF01_HEADER_LEN,
                actual: src.len(),
            });
        }

        if src[0..4] != TF01_MAGIC {
            return Err(ProtoError::BadMagic);
        }

        let mut buf = &src[4..TF01_HEADER_LEN];
        let version = buf.get_u16();
        let flags = buf.get_u16();
        let frame_length = buf.get_u32();
        let frame_type = FrameType::try_from(buf.get_u32())?;

        Ok(Self {
            version,
            flags,
            frame_length,
            frame_type,
        })
    }

    #[must_use]
    pub fn encode(self) -> [u8; TF01_HEADER_LEN] {
        let mut out = [0_u8; TF01_HEADER_LEN];
        out[0..4].copy_from_slice(&TF01_MAGIC);
        let mut tail = &mut out[4..];
        tail.put_u16(self.version);
        tail.put_u16(self.flags);
        tail.put_u32(self.frame_length);
        tail.put_u32(self.frame_type as u32);
        out
    }
}

pub fn decode_frame(src: &[u8]) -> Result<Frame, ProtoError> {
    let header = Tf01Header::decode(src)?;
    let payload = &src[TF01_HEADER_LEN..];
    if payload.len() != header.frame_length as usize {
        return Err(ProtoError::LengthMismatch {
            header: header.frame_length,
            actual: payload.len(),
        });
    }

    Ok(Frame {
        header,
        payload: Bytes::copy_from_slice(payload),
    })
}

#[must_use]
pub fn encode_frame(frame: &Frame) -> Bytes {
    let mut out = BytesMut::with_capacity(TF01_HEADER_LEN + frame.payload.len());
    out.extend_from_slice(
        &Tf01Header {
            frame_length: frame.payload.len() as u32,
            ..frame.header
        }
        .encode(),
    );
    out.extend_from_slice(&frame.payload);
    out.freeze()
}
