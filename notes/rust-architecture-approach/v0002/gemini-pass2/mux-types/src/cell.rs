use crate::style::{Color, Style};
use bitflags::bitflags;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Cell {
    pub char: char,
    pub style: Style,
    pub flags: CellFlags,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            char: ' ',
            style: Style::default(),
            flags: CellFlags::empty(),
        }
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct CellFlags: u8 {
        const BOLD = 0b0000_0001;
        const DIM = 0b0000_0010;
        const ITALIC = 0b0000_0100;
        const UNDERLINE = 0b0000_1000;
        const BLINK = 0b0001_0000;
        const REVERSE = 0b0010_0000;
        const HIDDEN = 0b0100_0000;
        const STRIKETHROUGH = 0b1000_0000;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cell_default() {
        let cell = Cell::default();
        assert_eq!(cell.char, ' ');
        assert_eq!(cell.flags, CellFlags::empty());
    }

    #[test]
    fn test_flags() {
        let mut flags = CellFlags::empty();
        flags.insert(CellFlags::BOLD);
        assert!(flags.contains(CellFlags::BOLD));
        assert!(!flags.contains(CellFlags::ITALIC));
    }
}
