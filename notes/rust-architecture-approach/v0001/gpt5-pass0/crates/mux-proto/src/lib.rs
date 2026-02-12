#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WireMessage {
    Hello { protocol_version: u8 },
    Data { pane_id: u64, bytes: Vec<u8> },
    Resize { pane_id: u64, cols: u16, rows: u16 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtoError {
    Truncated,
    InvalidType(u8),
    InvalidLength,
}

impl std::fmt::Display for ProtoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Truncated => f.write_str("truncated frame"),
            Self::InvalidType(t) => write!(f, "invalid frame type: {t}"),
            Self::InvalidLength => f.write_str("invalid frame length"),
        }
    }
}

impl std::error::Error for ProtoError {}

/// INV-020: protocol frames are length-delimited.
pub fn encode(msg: &WireMessage) -> Vec<u8> {
    let mut payload = Vec::new();
    match msg {
        WireMessage::Hello { protocol_version } => {
            payload.push(0);
            payload.push(*protocol_version);
        }
        WireMessage::Data { pane_id, bytes } => {
            payload.push(1);
            payload.extend_from_slice(&pane_id.to_le_bytes());
            payload.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
            payload.extend_from_slice(bytes);
        }
        WireMessage::Resize { pane_id, cols, rows } => {
            payload.push(2);
            payload.extend_from_slice(&pane_id.to_le_bytes());
            payload.extend_from_slice(&cols.to_le_bytes());
            payload.extend_from_slice(&rows.to_le_bytes());
        }
    }

    let mut frame = Vec::with_capacity(4 + payload.len());
    frame.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    frame.extend_from_slice(&payload);
    frame
}

pub fn decode(frame: &[u8]) -> Result<WireMessage, ProtoError> {
    if frame.len() < 4 {
        return Err(ProtoError::Truncated);
    }
    let len = u32::from_le_bytes(frame[..4].try_into().map_err(|_| ProtoError::Truncated)?) as usize;
    if frame.len() != 4 + len {
        return Err(ProtoError::InvalidLength);
    }
    if len == 0 {
        return Err(ProtoError::Truncated);
    }
    let payload = &frame[4..];
    match payload[0] {
        0 => {
            if payload.len() < 2 {
                return Err(ProtoError::Truncated);
            }
            Ok(WireMessage::Hello {
                protocol_version: payload[1],
            })
        }
        1 => {
            if payload.len() < 1 + 8 + 4 {
                return Err(ProtoError::Truncated);
            }
            let pane_id = u64::from_le_bytes(payload[1..9].try_into().map_err(|_| ProtoError::Truncated)?);
            let n = u32::from_le_bytes(payload[9..13].try_into().map_err(|_| ProtoError::Truncated)?) as usize;
            if payload.len() != 13 + n {
                return Err(ProtoError::InvalidLength);
            }
            Ok(WireMessage::Data {
                pane_id,
                bytes: payload[13..].to_vec(),
            })
        }
        2 => {
            if payload.len() != 1 + 8 + 2 + 2 {
                return Err(ProtoError::InvalidLength);
            }
            let pane_id = u64::from_le_bytes(payload[1..9].try_into().map_err(|_| ProtoError::Truncated)?);
            let cols = u16::from_le_bytes(payload[9..11].try_into().map_err(|_| ProtoError::Truncated)?);
            let rows = u16::from_le_bytes(payload[11..13].try_into().map_err(|_| ProtoError::Truncated)?);
            Ok(WireMessage::Resize { pane_id, cols, rows })
        }
        other => Err(ProtoError::InvalidType(other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_roundtrip() {
        let m = WireMessage::Hello { protocol_version: 8 };
        let b = encode(&m);
        let d = decode(&b).unwrap();
        assert_eq!(d, m);
    }

    #[test]
    fn data_roundtrip() {
        let m = WireMessage::Data {
            pane_id: 9,
            bytes: vec![1, 2, 3],
        };
        let b = encode(&m);
        assert_eq!(decode(&b).unwrap(), m);
    }

    #[test]
    fn invalid_type_rejected() {
        let frame = vec![1, 0, 0, 0, 9];
        assert!(matches!(decode(&frame), Err(ProtoError::InvalidType(9))));
    }

    #[test]
    fn invalid_length_rejected() {
        let frame = vec![2, 0, 0, 0, 0];
        assert!(matches!(decode(&frame), Err(ProtoError::InvalidLength)));
    }
}
