//! # mux-termlet
//!
//! Testing pods that combine a VT parser + grid for integration tests.
//! A "termlet" is a lightweight, in-process terminal emulator for verifying
//! the output of programs.
//!
//! L4 integration crate.

#![forbid(unsafe_code)]

use mux_types::Size;
use mux_grid::ChunkedGrid;
use mux_parser::VtParser;

/// A minimal terminal emulator for testing.
///
/// Combines a VT parser and a grid. Feed bytes in, read text out.
#[derive(Debug)]
pub struct Termlet {
    grid: ChunkedGrid,
    parser: VtParser,
}

impl Termlet {
    /// Create a new termlet with the given dimensions.
    #[must_use]
    pub fn new(size: Size) -> Self {
        Self {
            grid: ChunkedGrid::new(size.cols, size.rows, 0),
            parser: VtParser::new(),
        }
    }

    /// Feed raw bytes (as from PTY output) into the termlet.
    pub fn feed(&mut self, data: &[u8]) {
        let actions = self.parser.advance_all(data);
        for action in &actions {
            match action {
                mux_parser::VtAction::Print(c) => {
                    self.grid.write_char(*c);
                }
                mux_parser::VtAction::Execute(0x0A) => {
                    self.grid.line_feed();
                }
                mux_parser::VtAction::Execute(0x0D) => {
                    let (_, y) = self.grid.cursor();
                    self.grid.set_cursor(0, y);
                }
                _ => {}
            }
        }
    }

    /// Get the text content of the active screen.
    #[must_use]
    pub fn screen_text(&self) -> Vec<String> {
        self.grid.active_text()
    }

    /// Get the text of a single line.
    #[must_use]
    pub fn line_text(&self, row: u16) -> String {
        self.grid
            .active_line(row)
            .map(|l| l.text_content())
            .unwrap_or_default()
    }

    /// Current cursor position.
    #[must_use]
    pub fn cursor(&self) -> (u16, u16) {
        self.grid.cursor()
    }

    /// Grid dimensions.
    #[must_use]
    pub fn size(&self) -> Size {
        Size::new(self.grid.sx(), self.grid.sy())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn termlet_basic_text() {
        let mut t = Termlet::new(Size::new(80, 24));
        t.feed(b"Hello, World!");
        assert_eq!(t.line_text(0), "Hello, World!");
    }

    #[test]
    fn termlet_newlines() {
        let mut t = Termlet::new(Size::new(80, 24));
        t.feed(b"Line1\r\nLine2\r\nLine3");
        assert_eq!(t.line_text(0), "Line1");
        assert_eq!(t.line_text(1), "Line2");
        assert_eq!(t.line_text(2), "Line3");
    }

    #[test]
    fn termlet_cursor_position() {
        let mut t = Termlet::new(Size::new(80, 24));
        t.feed(b"ABC");
        assert_eq!(t.cursor(), (3, 0));
    }

    #[test]
    fn termlet_size() {
        let t = Termlet::new(Size::new(120, 40));
        assert_eq!(t.size(), Size::new(120, 40));
    }

    #[test]
    fn termlet_screen_text() {
        let mut t = Termlet::new(Size::new(10, 3));
        t.feed(b"Hi");
        let text = t.screen_text();
        assert_eq!(text.len(), 3);
        assert_eq!(text[0], "Hi");
    }
}
