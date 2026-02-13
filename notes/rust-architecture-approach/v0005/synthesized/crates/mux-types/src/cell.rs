//! Terminal cell representation.
//!
//! Each cell in the grid contains a grapheme cluster reference, display width,
//! flags, SGR attributes, foreground/background/underline colours, and a hyperlink ID.

use crate::attrs::{Attrs, CellFlags};
use crate::colour::Colour;
use mux_grapheme_arena::GraphemeId;

/// A single terminal cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Cell {
    /// Arena-allocated grapheme cluster.
    pub grapheme: GraphemeId,
    /// Display width: 0=padding, 1=normal, 2=CJK wide character.
    pub width: u8,
    /// Cell flags (PADDING, EXTENDED, SELECTED, CLEARED, TAB).
    pub flags: CellFlags,
    /// SGR text attributes (BOLD, ITALIC, UNDERSCORE, etc.).
    pub attrs: Attrs,
    /// Foreground colour.
    pub fg: Colour,
    /// Background colour.
    pub bg: Colour,
    /// Underline colour (for colored underlines via SGR 58).
    pub us: Colour,
    /// Hyperlink ID for OSC 8 support (0 = no link).
    pub link: u32,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            grapheme: GraphemeId::DEFAULT,
            width: 1,
            flags: CellFlags::empty(),
            attrs: Attrs::empty(),
            fg: Colour::Default,
            bg: Colour::Default,
            us: Colour::Default,
            link: 0,
        }
    }
}

impl Cell {
    /// Create a new default cell (space character, width 1, no attributes).
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a cell with a specific grapheme and width.
    pub fn with_grapheme(grapheme: GraphemeId, width: u8) -> Self {
        Self {
            grapheme,
            width,
            ..Self::default()
        }
    }

    /// Returns true if this cell is a padding cell for a preceding wide character.
    pub fn is_padding(&self) -> bool {
        self.flags.contains(CellFlags::PADDING)
    }

    /// Returns true if this cell has been explicitly cleared.
    pub fn is_cleared(&self) -> bool {
        self.flags.contains(CellFlags::CLEARED)
    }

    /// Returns true if this is a tab character cell.
    pub fn is_tab(&self) -> bool {
        self.flags.contains(CellFlags::TAB)
    }

    /// Returns true if this cell is selected (copy mode).
    pub fn is_selected(&self) -> bool {
        self.flags.contains(CellFlags::SELECTED)
    }

    /// Returns true if this cell is a wide character (width > 1).
    pub fn is_wide(&self) -> bool {
        self.width > 1
    }

    /// Reset this cell to defaults (space, no attributes).
    pub fn clear(&mut self) {
        *self = Self::default();
        self.flags.insert(CellFlags::CLEARED);
    }

    /// Returns true if this cell has any non-default attributes.
    pub fn has_attrs(&self) -> bool {
        !self.attrs.is_empty_attrs()
            || !self.fg.is_default()
            || !self.bg.is_default()
            || !self.us.is_default()
    }

    /// Returns true if two cells are visually identical (same grapheme, attrs, colours).
    pub fn visually_equal(&self, other: &Self) -> bool {
        self.grapheme == other.grapheme
            && self.width == other.width
            && self.attrs == other.attrs
            && self.fg == other.fg
            && self.bg == other.bg
            && self.us == other.us
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_cell_is_space() {
        let c = Cell::default();
        assert_eq!(c.grapheme, GraphemeId::DEFAULT);
        assert_eq!(c.width, 1);
        assert!(c.flags.is_empty());
    }

    #[test]
    fn cell_with_grapheme() {
        let c = Cell::with_grapheme(GraphemeId::from_raw(5), 2);
        assert_eq!(c.grapheme, GraphemeId::from_raw(5));
        assert_eq!(c.width, 2);
        assert!(c.is_wide());
    }

    #[test]
    fn padding_cell() {
        let mut c = Cell::default();
        c.flags.insert(CellFlags::PADDING);
        c.width = 0;
        assert!(c.is_padding());
    }

    #[test]
    fn cleared_cell() {
        let mut c = Cell::default();
        c.clear();
        assert!(c.is_cleared());
    }

    #[test]
    fn tab_cell() {
        let mut c = Cell::default();
        c.flags.insert(CellFlags::TAB);
        assert!(c.is_tab());
    }

    #[test]
    fn selected_cell() {
        let mut c = Cell::default();
        c.flags.insert(CellFlags::SELECTED);
        assert!(c.is_selected());
    }

    #[test]
    fn has_attrs_with_bold() {
        let mut c = Cell::default();
        c.attrs = Attrs::BOLD;
        assert!(c.has_attrs());
    }

    #[test]
    fn has_attrs_with_fg() {
        let mut c = Cell::default();
        c.fg = Colour::Indexed(1);
        assert!(c.has_attrs());
    }

    #[test]
    fn no_attrs_default() {
        let c = Cell::default();
        assert!(!c.has_attrs());
    }

    #[test]
    fn visually_equal_same() {
        let c1 = Cell::default();
        let c2 = Cell::default();
        assert!(c1.visually_equal(&c2));
    }

    #[test]
    fn visually_not_equal_different_fg() {
        let c1 = Cell::default();
        let mut c2 = Cell::default();
        c2.fg = Colour::Indexed(3);
        assert!(!c1.visually_equal(&c2));
    }

    #[test]
    fn wide_cell_detection() {
        let mut c = Cell::default();
        c.width = 2;
        assert!(c.is_wide());
        c.width = 1;
        assert!(!c.is_wide());
    }

    #[test]
    fn cell_new_matches_default() {
        assert_eq!(Cell::new(), Cell::default());
    }

    #[test]
    fn clear_resets_grapheme() {
        let mut c = Cell::with_grapheme(GraphemeId::from_raw(42), 1);
        c.clear();
        assert_eq!(c.grapheme, GraphemeId::DEFAULT);
    }

    #[test]
    fn clear_resets_width_to_one() {
        let mut c = Cell::with_grapheme(GraphemeId::from_raw(1), 2);
        c.clear();
        assert_eq!(c.width, 1);
    }

    #[test]
    fn clear_resets_attrs() {
        let mut c = Cell::default();
        c.attrs = Attrs::BOLD | Attrs::ITALIC;
        c.clear();
        assert!(c.attrs.is_empty_attrs());
    }

    #[test]
    fn clear_resets_colours() {
        let mut c = Cell::default();
        c.fg = Colour::Indexed(5);
        c.bg = Colour::Rgb { r: 1, g: 2, b: 3 };
        c.clear();
        assert_eq!(c.fg, Colour::Default);
        assert_eq!(c.bg, Colour::Default);
    }

    #[test]
    fn clear_resets_link() {
        let mut c = Cell::default();
        c.link = 42;
        c.clear();
        assert_eq!(c.link, 0);
    }

    #[test]
    fn has_attrs_with_bg() {
        let mut c = Cell::default();
        c.bg = Colour::Indexed(7);
        assert!(c.has_attrs());
    }

    #[test]
    fn has_attrs_with_underline_colour() {
        let mut c = Cell::default();
        c.us = Colour::Rgb { r: 0, g: 255, b: 0 };
        assert!(c.has_attrs());
    }

    #[test]
    fn visually_not_equal_different_width() {
        let c1 = Cell::with_grapheme(GraphemeId::from_raw(1), 1);
        let c2 = Cell::with_grapheme(GraphemeId::from_raw(1), 2);
        assert!(!c1.visually_equal(&c2));
    }

    #[test]
    fn visually_not_equal_different_grapheme() {
        let c1 = Cell::with_grapheme(GraphemeId::from_raw(1), 1);
        let c2 = Cell::with_grapheme(GraphemeId::from_raw(2), 1);
        assert!(!c1.visually_equal(&c2));
    }

    #[test]
    fn visually_not_equal_different_bg() {
        let mut c1 = Cell::default();
        let mut c2 = Cell::default();
        c1.bg = Colour::Indexed(1);
        c2.bg = Colour::Indexed(2);
        assert!(!c1.visually_equal(&c2));
    }

    #[test]
    fn visually_not_equal_different_us() {
        let mut c1 = Cell::default();
        let mut c2 = Cell::default();
        c1.us = Colour::Indexed(1);
        assert!(!c1.visually_equal(&c2));
    }

    #[test]
    fn visually_not_equal_different_attrs() {
        let mut c1 = Cell::default();
        let mut c2 = Cell::default();
        c1.attrs = Attrs::BOLD;
        assert!(!c1.visually_equal(&c2));
    }

    #[test]
    fn multiple_flags_set() {
        let mut c = Cell::default();
        c.flags.insert(CellFlags::PADDING);
        c.flags.insert(CellFlags::SELECTED);
        assert!(c.is_padding());
        assert!(c.is_selected());
    }

    #[test]
    fn cell_clone_eq() {
        let mut c = Cell::with_grapheme(GraphemeId::from_raw(3), 2);
        c.attrs = Attrs::BOLD;
        c.fg = Colour::Indexed(5);
        let c2 = c;
        assert_eq!(c, c2);
    }

    #[test]
    fn cell_hash_consistent() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let c1 = Cell::default();
        let c2 = Cell::default();
        let mut h1 = DefaultHasher::new();
        let mut h2 = DefaultHasher::new();
        c1.hash(&mut h1);
        c2.hash(&mut h2);
        assert_eq!(h1.finish(), h2.finish());
    }

    #[test]
    fn width_zero_for_padding() {
        let mut c = Cell::default();
        c.width = 0;
        c.flags.insert(CellFlags::PADDING);
        assert!(!c.is_wide());
        assert!(c.is_padding());
    }
}
