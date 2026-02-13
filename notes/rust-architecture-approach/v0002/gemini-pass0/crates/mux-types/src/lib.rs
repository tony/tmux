use bitflags::bitflags;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Style {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub attributes: Attributes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Color {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    Index(u8),
    RGB(u8, u8, u8),
}

bitflags! {
    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub struct Attributes: u16 {
        const BOLD       = 0x0001;
        const DIM        = 0x0002;
        const ITALIC     = 0x0004;
        const UNDERLINE  = 0x0008;
        const BLINK      = 0x0010;
        const REVERSE    = 0x0020;
        const HIDDEN     = 0x0040;
    }
}

bitflags! {
    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub struct CellFlags: u8 {
        /// Matches tmux GRID_FLAG_PADDING
        const PADDING = 0x04;
        const WRAP    = 0x08;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cell {
    pub char: char,
    pub style: Style,
    pub flags: CellFlags,
}

pub type SessionId = u64;
pub type WindowId = u64;
pub type PaneId = u64;
--- END FILE ---
