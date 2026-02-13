use bitflags::bitflags;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Color(pub u8);

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub struct CellFlags: u32 {
        const BOLD          = 0x00000001;
        const DIM           = 0x00000002;
        const ITALIC        = 0x00000004;
        const UNDERLINE     = 0x00000008;
        const BLINK         = 0x00000010;
        const REVERSE       = 0x00000020;
        const HIDDEN        = 0x00000040;
        const PADDING       = 0x00000080; // tmux compatibility
        const WRAPPED       = 0x00000100;
        const WIDE          = 0x00000200;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cell {
    pub char: char,
    pub flags: CellFlags,
    pub fg: Color,
    pub bg: Color,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            char: ' ',
            flags: CellFlags::empty(),
            fg: Color(7),
            bg: Color(0),
        }
    }
}

// Protocol Messages (Inter-Thread)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KernelEvent {
    Resize(u16, u16),
    KeyInput(Vec<u8>),
    PaneExited(u32, i32), // PaneId, ExitCode
    ConfigReload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RenderInstruction {
    RenderFull,
    RenderDiff,
}
