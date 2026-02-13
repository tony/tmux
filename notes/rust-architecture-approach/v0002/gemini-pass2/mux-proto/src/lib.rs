use bytes::Bytes;
use mux_types::{Size, Key, MouseEvent};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Header {
    pub magic: u32, // 0xTF01TF01
    pub length: u32,
    pub crc32: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Frame {
    Command(Command),
    Data { pane_id: u32, data: Bytes },
    Render(RenderOp),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    NewWindow,
    SplitPane { vertical: bool },
    Resize { width: u16, height: u16 },
    Input(Key),
    Mouse(MouseEvent),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RenderOp {
    Resize(Size),
    // ... extensive render ops for diffing
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_size() {
        let h = Header { magic: 0, length: 0, crc32: 0 };
        let encoded = bincode::serialize(&h).unwrap();
        // u32 * 3 = 12 bytes
        assert_eq!(encoded.len(), 12);
    }

    #[test]
    fn test_frame_command() {
        let f = Frame::Command(Command::NewWindow);
        let encoded = bincode::serialize(&f).unwrap();
        let decoded: Frame = bincode::deserialize(&encoded).unwrap();
        if let Frame::Command(Command::NewWindow) = decoded {
            // ok
        } else {
            panic!("Wrong variant");
        }
    }

    #[test]
    fn test_frame_data() {
        let f = Frame::Data { pane_id: 1, data: Bytes::from("hello") };
        // bytes needs serde feature
        let encoded = bincode::serialize(&f).unwrap();
        let decoded: Frame = bincode::deserialize(&encoded).unwrap();
         if let Frame::Data { pane_id, data } = decoded {
            assert_eq!(pane_id, 1);
            assert_eq!(data, Bytes::from("hello"));
        } else {
            panic!("Wrong variant");
        }
    }
}
