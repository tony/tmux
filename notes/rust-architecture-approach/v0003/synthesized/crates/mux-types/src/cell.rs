//! Terminal cell with grapheme, width, flags, attributes, and colours.
//!
//! CellFlags bit values match tmux's GRID_FLAG_* exactly (INV-119, tmux.h:742-749).

use crate::attrs::Attrs;
use crate::colour::Colour;
use bitflags::bitflags;

bitflags! {
    /// Cell flags matching tmux's GRID_FLAG_* constants.
    /// INV-119: values must match tmux.h:742-749 exactly.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct CellFlags: u8 {
        /// Cell has extended attributes.
        const FG          = 0x01;
        /// Cell has extended background.
        const BG          = 0x02;
        /// Cell is padding (right side of wide char).
        const PADDING     = 0x04;
        /// Cell has extended grapheme cluster.
        const EXTENDED    = 0x08;
        /// Cell was selected in copy mode.
        const SELECTED    = 0x10;
        /// Cell content has not changed.
        const NOCHANGE    = 0x20;
        /// Cell is cleared.
        const CLEARED     = 0x40;
    }
}

/// A single terminal cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    /// The character displayed.
    pub grapheme: char,
    /// Display width (0 for padding cells, 1 or 2 for normal/wide).
    pub width: u8,
    /// Cell flags (INV-119: match tmux GRID_FLAG_*).
    pub flags: CellFlags,
    /// Text attributes.
    pub attrs: Attrs,
    /// Foreground colour.
    pub fg: Colour,
    /// Background colour.
    pub bg: Colour,
}

impl Cell {
    /// Create an empty (space) cell with default attributes.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            grapheme: ' ',
            width: 1,
            flags: CellFlags::empty(),
            attrs: Attrs::empty(),
            fg: Colour::Default,
            bg: Colour::Default,
        }
    }

    /// Create a cell from a character with default attributes.
    #[must_use]
    pub fn from_char(c: char) -> Self {
        let w = unicode_width::UnicodeWidthChar::width(c).unwrap_or(1);
        Self {
            grapheme: c,
            width: w as u8,
            flags: CellFlags::empty(),
            attrs: Attrs::empty(),
            fg: Colour::Default,
            bg: Colour::Default,
        }
    }

    /// Whether this cell is a padding cell (right half of a wide char).
    #[must_use]
    pub const fn is_padding(&self) -> bool {
        self.flags.contains(CellFlags::PADDING)
    }

    /// Whether this cell is a wide character (width 2).
    #[must_use]
    pub const fn is_wide(&self) -> bool {
        self.width == 2
    }

    /// Whether this cell is empty (space with default attrs).
    #[must_use]
    pub fn is_empty_cell(&self) -> bool {
        self.grapheme == ' '
            && self.attrs.is_empty()
            && self.fg == Colour::Default
            && self.bg == Colour::Default
    }
}

impl Default for Cell {
    fn default() -> Self {
        Self::empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_cell() {
        let c = Cell::empty();
        assert_eq!(c.grapheme, ' ');
        assert_eq!(c.width, 1);
        assert!(c.flags.is_empty());
        assert!(c.is_empty_cell());
    }

    #[test]
    fn cell_from_char() {
        let c = Cell::from_char('A');
        assert_eq!(c.grapheme, 'A');
        assert_eq!(c.width, 1);
    }

    #[test]
    fn padding_flag() {
        let mut c = Cell::empty();
        c.flags |= CellFlags::PADDING;
        assert!(c.is_padding());
    }

    #[test]
    fn wide_char() {
        // CJK character has width 2
        let c = Cell::from_char('\u{4E2D}'); // Chinese "zhong"
        assert!(c.is_wide());
        assert_eq!(c.width, 2);
    }

    #[test]
    fn cell_flags_inv119() {
        // INV-119: verify flag values match tmux.h:742-749
        assert_eq!(CellFlags::FG.bits(), 0x01);
        assert_eq!(CellFlags::BG.bits(), 0x02);
        assert_eq!(CellFlags::PADDING.bits(), 0x04);
        assert_eq!(CellFlags::EXTENDED.bits(), 0x08);
        assert_eq!(CellFlags::SELECTED.bits(), 0x10);
        assert_eq!(CellFlags::NOCHANGE.bits(), 0x20);
        assert_eq!(CellFlags::CLEARED.bits(), 0x40);
    }

    #[test]
    fn cell_not_empty_with_colour() {
        let mut c = Cell::from_char(' ');
        c.fg = Colour::Rgb {
            r: 255,
            g: 0,
            b: 0,
        };
        assert!(!c.is_empty_cell());
    }

    #[test]
    fn cell_default_is_empty() {
        let c = Cell::default();
        assert!(c.is_empty_cell());
    }
}
