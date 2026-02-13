use bytes::Bytes;
use serde::{Deserialize, Serialize};

pub const TF01_MAGIC: u32 = 0x54463031; // "TF01"

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct TfHeader {
    pub magic: u32,
    pub version: u16,
    pub flags: u16,
    pub length: u32,
    pub msg_type: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    Handshake { version: u16 },
    Resize { rows: u16, cols: u16 },
    Stdin { data: Vec<u8> },
    Stdout { data: Vec<u8> },
    // tmux compatibility messages would go here too
}

pub trait Encoder {
    fn encode(&self) -> Bytes;
}

pub trait Decoder {
    fn decode(buf: &mut Bytes) -> Option<Message>;
}
--- END FILE ---
