//! Pane entity with grid and parser.

use mux_grid::ChunkedGrid;
use mux_parser::VtParser;
use mux_types::{PaneId, Size};

/// A terminal pane with its own grid and VT parser.
#[derive(Debug)]
pub struct Pane {
    /// Pane identifier.
    pub id: PaneId,
    /// Terminal grid.
    pub grid: ChunkedGrid,
    /// VT parser state.
    parser: VtParser,
    /// Whether the child process has exited.
    pub exited: bool,
}

impl Pane {
    /// Create a new pane with the given size.
    #[must_use]
    pub fn new(id: PaneId, size: Size) -> Self {
        Self {
            id,
            grid: ChunkedGrid::new(size.cols, size.rows, 2000),
            parser: VtParser::new(),
            exited: false,
        }
    }

    /// Feed PTY output through the parser and into the grid.
    pub fn process_output(&mut self, data: &[u8]) {
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
    fn pane_creation() {
        let pane = Pane::new(PaneId::new(1), Size::new(80, 24));
        assert_eq!(pane.size(), Size::new(80, 24));
        assert!(!pane.exited);
    }

    #[test]
    fn pane_process_output() {
        let mut pane = Pane::new(PaneId::new(1), Size::new(80, 24));
        pane.process_output(b"Hello");
        let text = pane.grid.active_text();
        assert!(text[0].contains("Hello"));
    }

    #[test]
    fn pane_id_stored() {
        let pane = Pane::new(PaneId::new(42), Size::new(80, 24));
        assert_eq!(pane.id, PaneId::new(42));
    }

    #[test]
    fn pane_exited_flag() {
        let mut pane = Pane::new(PaneId::new(1), Size::new(80, 24));
        assert!(!pane.exited);
        pane.exited = true;
        assert!(pane.exited);
    }

    #[test]
    fn pane_process_line_feed() {
        let mut pane = Pane::new(PaneId::new(1), Size::new(80, 24));
        pane.process_output(b"Line1\nLine2");
        let text = pane.grid.active_text();
        assert!(text[0].contains("Line1"));
        // Line2 should be on the next row
        assert!(text[1].contains("Line2"));
    }
}
