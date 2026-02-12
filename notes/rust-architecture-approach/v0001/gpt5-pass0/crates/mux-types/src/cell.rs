use compact_str::CompactString;
use serde::{Deserialize, Serialize};

/// A logical terminal cell.
///
/// v15 references: S91 (CompactString), INV-008 (UTF-8 grapheme), INV-009 (bitfield-friendly style/flags).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cell {
    pub grapheme: CompactString,
    pub style: u16,
    pub flags: u16,
    pub width: CellWidth,
    /// 14-bit index into GraphemeArena; 0 means no extension (INV-030).
    pub ext: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum CellWidth {
    Zero = 0,
    One = 1,
    Two = 2,
    Ambiguous = 3,
}

impl Cell {
    pub fn new(grapheme: impl Into<CompactString>, style: u16, flags: u16, width: CellWidth) -> Self {
        Self {
            grapheme: grapheme.into(),
            style,
            flags,
            width,
            ext: 0,
        }
    }

    pub fn from_char(ch: char) -> Self {
        Self {
            grapheme: CompactString::new(ch.to_string()),
            style: 0,
            flags: 0,
            width: width_from_char(ch),
            ext: 0,
        }
    }

    pub fn with_ext(mut self, ext: u16) -> Self {
        self.ext = ext;
        self
    }

    pub fn scalar(&self) -> Option<char> {
        let mut it = self.grapheme.chars();
        let ch = it.next()?;
        if it.next().is_none() {
            Some(ch)
        } else {
            None
        }
    }
}

fn width_from_char(ch: char) -> CellWidth {
    if ch == '\u{0000}' {
        return CellWidth::Zero;
    }
    if ch.is_ascii() {
        return CellWidth::One;
    }

    // Lightweight approximation for scaffold purposes.
    if matches!(
        ch as u32,
        0x1100..=0x115F
            | 0x2329..=0x232A
            | 0x2E80..=0xA4CF
            | 0xAC00..=0xD7A3
            | 0xF900..=0xFAFF
            | 0xFE10..=0xFE19
            | 0xFE30..=0xFE6F
            | 0xFF00..=0xFF60
            | 0xFFE0..=0xFFE6
    ) {
        CellWidth::Two
    } else {
        CellWidth::One
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn char_cell_defaults() {
        let c = Cell::from_char('a');
        assert_eq!(c.grapheme, "a");
        assert_eq!(c.width, CellWidth::One);
        assert_eq!(c.ext, 0);
    }

    #[test]
    fn wide_char_cell() {
        let c = Cell::from_char('界');
        assert_eq!(c.width, CellWidth::Two);
    }

    #[test]
    fn scalar_none_for_multi_grapheme() {
        let c = Cell::new("ab", 0, 0, CellWidth::One);
        assert!(c.scalar().is_none());
    }

    #[test]
    fn ext_update() {
        let c = Cell::from_char('x').with_ext(42);
        assert_eq!(c.ext, 42);
    }
}
