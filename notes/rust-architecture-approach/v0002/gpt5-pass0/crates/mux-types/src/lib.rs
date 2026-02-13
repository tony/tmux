use bitflags::bitflags;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WindowId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PaneId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StyleId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Size {
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Point {
    pub x: u16,
    pub y: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect {
    pub origin: Point,
    pub size: Size,
}

impl Rect {
    #[must_use]
    pub fn contains(self, point: Point) -> bool {
        let max_x = self.origin.x.saturating_add(self.size.cols);
        let max_y = self.origin.y.saturating_add(self.size.rows);
        point.x >= self.origin.x && point.x < max_x && point.y >= self.origin.y && point.y < max_y
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct CellFlags: u8 {
        const NONE = 0x00;
        const WRAP = 0x01;
        const WIDE = 0x02;
        const PADDING = 0x04;
        const DIRTY = 0x08;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cell {
    pub grapheme: String,
    pub style: StyleId,
    pub flags: CellFlags,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            grapheme: " ".to_owned(),
            style: StyleId(0),
            flags: CellFlags::NONE,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelCaps {
    pub control_msgs: usize,
    pub data_msgs: usize,
    pub render_msgs: usize,
    pub data_chunk_bytes: usize,
}

impl Default for ChannelCaps {
    fn default() -> Self {
        Self {
            control_msgs: 512,
            data_msgs: 1024,
            render_msgs: 256,
            data_chunk_bytes: 64 * 1024,
        }
    }
}
