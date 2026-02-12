//! Terminal cell type.
//!
//! A [`Cell`] is the fundamental unit of terminal content: a grapheme cluster
//! with associated style and display width.
//!
//! ## Invariants
//! - INV-008: Cell grapheme is valid UTF-8.
//! - INV-029: Cell width is 0, 1, or 2 (explicit, not inferred at render time).
//! - S91: Grapheme stored as `compact_str::CompactString`.

use compact_str::CompactString;
use crate::style::Style;

/// A single terminal cell.
///
/// INV-008: The grapheme is always valid UTF-8 (enforced by CompactString).
/// INV-029: Width is always 0, 1, or 2.
/// S91: Uses `compact_str::CompactString` for inline storage of short graphemes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    /// The grapheme cluster (one or more Unicode code points).
    grapheme: CompactString,
    /// Visual style (foreground, background, attributes).
    style: Style,
    /// Display width: 0 (combining/zero-width), 1 (narrow), or 2 (wide/CJK).
    width: u8,
}

impl Cell {
    /// Create a new cell with the given grapheme, style, and width.
    ///
    /// Width values > 2 are clamped to 1.
    #[must_use]
    pub fn new(grapheme: CompactString, style: Style, width: u8) -> Self {
        let width = if width > 2 { 1 } else { width };
        Self { grapheme, style, width }
    }

    /// Create a blank cell (space character, default style, width 1).
    #[must_use]
    pub fn blank() -> Self {
        Self {
            grapheme: CompactString::const_new(" "),
            style: Style::default(),
            width: 1,
        }
    }

    /// Create a cell from a single ASCII character.
    #[must_use]
    pub fn from_char(ch: char, style: Style) -> Self {
        let mut buf = [0u8; 4];
        let s = ch.encode_utf8(&mut buf);
        let width = if ch.is_control() {
            0
        } else if is_wide_char(ch) {
            2
        } else {
            1
        };
        Self {
            grapheme: CompactString::from(&*s),
            style,
            width,
        }
    }

    /// The grapheme cluster as a string slice.
    #[must_use]
    pub fn grapheme(&self) -> &str {
        self.grapheme.as_str()
    }

    /// The visual style.
    #[must_use]
    pub fn style(&self) -> &Style {
        &self.style
    }

    /// The display width (0, 1, or 2).
    /// INV-029: Width is explicit, never inferred at render time.
    #[must_use]
    pub fn width(&self) -> u8 {
        self.width
    }

    /// The first Unicode scalar value of the grapheme.
    /// Returns U+0020 (space) if the grapheme is empty.
    #[must_use]
    pub fn first_scalar(&self) -> char {
        self.grapheme.chars().next().unwrap_or(' ')
    }

    /// True if this cell is blank (space with default style).
    #[must_use]
    pub fn is_blank(&self) -> bool {
        self.grapheme.as_str() == " " && self.style == Style::default() && self.width == 1
    }

    /// Set a new grapheme and width on this cell.
    pub fn set_grapheme(&mut self, grapheme: CompactString, width: u8) {
        self.grapheme = grapheme;
        self.width = if width > 2 { 1 } else { width };
    }

    /// Set the style on this cell.
    pub fn set_style(&mut self, style: Style) {
        self.style = style;
    }
}

impl Default for Cell {
    fn default() -> Self {
        Self::blank()
    }
}

/// Simple heuristic for wide characters (CJK Unified Ideographs).
/// A production implementation would use Unicode East Asian Width tables.
fn is_wide_char(ch: char) -> bool {
    let cp = ch as u32;
    // CJK Unified Ideographs
    (0x4E00..=0x9FFF).contains(&cp)
        // CJK Unified Ideographs Extension A
        || (0x3400..=0x4DBF).contains(&cp)
        // CJK Compatibility Ideographs
        || (0xF900..=0xFAFF).contains(&cp)
        // Fullwidth Forms
        || (0xFF01..=0xFF60).contains(&cp)
        || (0xFFE0..=0xFFE6).contains(&cp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blank_cell() {
        let cell = Cell::blank();
        assert_eq!(cell.grapheme(), " ");
        assert_eq!(cell.width(), 1);
        assert!(cell.is_blank());
    }

    #[test]
    fn test_default_is_blank() {
        let cell = Cell::default();
        assert!(cell.is_blank());
    }

    #[test]
    fn test_from_char_ascii() {
        let cell = Cell::from_char('A', Style::default());
        assert_eq!(cell.grapheme(), "A");
        assert_eq!(cell.width(), 1);
        assert_eq!(cell.first_scalar(), 'A');
    }

    #[test]
    fn test_from_char_cjk_wide() {
        let cell = Cell::from_char('\u{4E16}', Style::default()); // CJK char
        assert_eq!(cell.width(), 2);
    }

    #[test]
    fn test_from_char_control_width_zero() {
        let cell = Cell::from_char('\x07', Style::default()); // BEL
        assert_eq!(cell.width(), 0);
    }

    #[test]
    fn test_width_clamped_to_valid() {
        let cell = Cell::new(CompactString::from("x"), Style::default(), 5);
        assert_eq!(cell.width(), 1); // clamped: 5 > 2, so defaults to 1
    }

    #[test]
    fn test_width_zero_allowed() {
        let cell = Cell::new(CompactString::from("\u{0300}"), Style::default(), 0);
        assert_eq!(cell.width(), 0);
    }

    #[test]
    fn test_width_two_allowed() {
        let cell = Cell::new(CompactString::from("\u{4E16}"), Style::default(), 2);
        assert_eq!(cell.width(), 2);
    }

    #[test]
    fn test_set_grapheme() {
        let mut cell = Cell::blank();
        cell.set_grapheme(CompactString::from("X"), 1);
        assert_eq!(cell.grapheme(), "X");
        assert!(!cell.is_blank());
    }

    #[test]
    fn test_set_style() {
        let mut cell = Cell::blank();
        let style = Style::new(crate::style::Color::Indexed(1), crate::style::Color::Default);
        cell.set_style(style);
        assert_eq!(cell.style().fg, crate::style::Color::Indexed(1));
    }

    #[test]
    fn test_first_scalar_empty_fallback() {
        let cell = Cell::new(CompactString::from(""), Style::default(), 0);
        assert_eq!(cell.first_scalar(), ' ');
    }
}
