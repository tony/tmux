//! Terminal cell type.
//!
//! A [`Cell`] is the fundamental unit of terminal content: a grapheme cluster
//! with associated style and display width.
//!
//! ## Invariants
//! - INV-119: CellFlags bit values match tmux's `GRID_FLAG_*` exactly (tmux.h:742-749).
//! - Width is 0 (padding/combining), 1 (normal), or 2 (CJK wide).

use bitflags::bitflags;
use mux_grapheme_arena::GraphemeId;
use crate::style::{Attrs, Colour};

bitflags! {
    /// Cell flags matching tmux's `GRID_FLAG_*` values (INV-119).
    ///
    /// Bit values verified against tmux.h:742-749.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct CellFlags: u8 {
        /// Wide character padding cell (tmux GRID_FLAG_PADDING = 0x04, tmux.h:744).
        const PADDING  = 0x04;
        /// Extended grapheme cluster (tmux GRID_FLAG_EXTENDED = 0x08, tmux.h:745).
        const EXTENDED = 0x08;
        /// Cell is selected (tmux GRID_FLAG_SELECTED = 0x10, tmux.h:746).
        const SELECTED = 0x10;
        /// Cell has been explicitly cleared (tmux GRID_FLAG_CLEARED = 0x40, tmux.h:748).
        const CLEARED  = 0x40;
        /// Cell contains a tab (tmux GRID_FLAG_TAB = 0x80, tmux.h:749).
        const TAB      = 0x80;
    }
}

impl Default for CellFlags {
    fn default() -> Self {
        Self::empty()
    }
}

/// A single terminal cell.
///
/// Stores a grapheme cluster reference, display width, flags, text attributes,
/// foreground/background/underline colours, and hyperlink ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    /// Arena-allocated UTF-8 grapheme cluster.
    pub grapheme: GraphemeId,
    /// Display width: 0=padding/combining, 1=normal, 2=CJK wide.
    pub width: u8,
    /// Cell flags matching tmux GRID_FLAG_* (INV-119).
    pub flags: CellFlags,
    /// Text attributes (bold, italic, etc.).
    pub attrs: Attrs,
    /// Foreground colour.
    pub fg: Colour,
    /// Background colour.
    pub bg: Colour,
    /// Underline colour (for colored underlines, e.g. CSI 58;2;R;G;Bm).
    pub us: Colour,
    /// Hyperlink ID (OSC 8). 0 = no hyperlink.
    pub link: u32,
}

impl Cell {
    /// Create a default (empty) cell.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            grapheme: GraphemeId::Empty,
            width: 1,
            flags: CellFlags::empty(),
            attrs: Attrs::empty(),
            fg: Colour::Default,
            bg: Colour::Default,
            us: Colour::Default,
            link: 0,
        }
    }

    /// Create a padding cell for wide characters.
    #[must_use]
    pub const fn padding() -> Self {
        Self {
            grapheme: GraphemeId::Empty,
            width: 0,
            flags: CellFlags::PADDING,
            attrs: Attrs::empty(),
            fg: Colour::Default,
            bg: Colour::Default,
            us: Colour::Default,
            link: 0,
        }
    }

    /// Whether this cell is a padding cell for a wide character.
    #[must_use]
    pub const fn is_padding(&self) -> bool {
        self.flags.contains(CellFlags::PADDING)
    }

    /// Whether this cell is empty (default grapheme, no flags).
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        matches!(self.grapheme, GraphemeId::Empty) && self.flags.is_empty()
    }

    /// Whether this cell has been explicitly cleared.
    #[must_use]
    pub const fn is_cleared(&self) -> bool {
        self.flags.contains(CellFlags::CLEARED)
    }

    /// Whether this cell is selected (copy mode).
    #[must_use]
    pub const fn is_selected(&self) -> bool {
        self.flags.contains(CellFlags::SELECTED)
    }

    /// Whether this cell contains a tab.
    #[must_use]
    pub const fn is_tab(&self) -> bool {
        self.flags.contains(CellFlags::TAB)
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
    fn cell_flags_padding_matches_tmux() {
        assert_eq!(CellFlags::PADDING.bits(), 0x04);
    }

    #[test]
    fn cell_flags_extended_matches_tmux() {
        assert_eq!(CellFlags::EXTENDED.bits(), 0x08);
    }

    #[test]
    fn cell_flags_selected_matches_tmux() {
        assert_eq!(CellFlags::SELECTED.bits(), 0x10);
    }

    #[test]
    fn cell_flags_cleared_matches_tmux() {
        assert_eq!(CellFlags::CLEARED.bits(), 0x40);
    }

    #[test]
    fn cell_flags_tab_matches_tmux() {
        assert_eq!(CellFlags::TAB.bits(), 0x80);
    }

    #[test]
    fn empty_cell_defaults() {
        let cell = Cell::empty();
        assert!(cell.is_empty());
        assert!(!cell.is_padding());
        assert!(!cell.is_cleared());
        assert!(!cell.is_selected());
        assert!(!cell.is_tab());
        assert_eq!(cell.width, 1);
    }

    #[test]
    fn padding_cell() {
        let cell = Cell::padding();
        assert!(cell.is_padding());
        assert_eq!(cell.width, 0);
    }

    #[test]
    fn cell_default_trait() {
        let cell = Cell::default();
        assert!(cell.is_empty());
    }

    #[test]
    fn cell_equality() {
        assert_eq!(Cell::empty(), Cell::empty());
        assert_ne!(Cell::empty(), Cell::padding());
    }

    #[test]
    fn cell_with_grapheme() {
        let cell = Cell {
            grapheme: GraphemeId::from_char('A'),
            width: 1,
            flags: CellFlags::empty(),
            ..Cell::empty()
        };
        assert!(!cell.is_empty());
        assert_eq!(cell.grapheme, GraphemeId::Inline('A'));
    }

    #[test]
    fn cell_hyperlink_default_zero() {
        assert_eq!(Cell::empty().link, 0);
    }
}
