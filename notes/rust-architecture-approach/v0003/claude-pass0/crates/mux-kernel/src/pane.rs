//! Pane entity.
//!
//! A pane owns a grid and a VT parser. It processes PTY output
//! by feeding bytes through the parser and applying actions to the grid.

use mux_types::{PaneId, Size};
use mux_grid::ChunkedGrid;
use mux_parser::{VtParser, VtAction};
use mux_grapheme_arena::GraphemeArena;

/// A terminal pane with its own grid, parser, and grapheme arena.
#[derive(Debug)]
pub struct Pane {
    /// Pane identifier.
    pub id: PaneId,
    /// Terminal grid.
    pub grid: ChunkedGrid,
    /// VT parser state machine.
    pub parser: VtParser,
    /// Grapheme arena for extended grapheme clusters.
    pub arena: GraphemeArena,
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
            arena: GraphemeArena::new(),
            exited: false,
        }
    }

    /// Process PTY output by feeding it through the parser and grid.
    pub fn process_output(&mut self, data: &[u8]) {
        let actions = self.parser.advance_all(data);
        for action in actions {
            self.apply_action(&action);
        }
    }

    /// Apply a single parser action to the grid.
    fn apply_action(&mut self, action: &VtAction) {
        match action {
            VtAction::Print(c) => {
                self.grid.write_char(*c);
            }
            VtAction::Execute(byte) => {
                self.execute_c0(*byte);
            }
            VtAction::CsiDispatch(params) => {
                self.dispatch_csi(params);
            }
            _ => {
                // Other actions (ESC, OSC, DCS) not yet implemented in scaffold
            }
        }
    }

    /// Execute a C0 control character.
    fn execute_c0(&mut self, byte: u8) {
        match byte {
            0x08 => {
                // BS: move cursor left
                let (x, y) = self.grid.cursor();
                if x > 0 {
                    self.grid.set_cursor(x - 1, y);
                }
            }
            0x09 => {
                // HT: tab (move to next tab stop)
                let (x, y) = self.grid.cursor();
                let next_tab = (x / 8 + 1) * 8;
                self.grid.set_cursor(next_tab.min(self.grid.sx() - 1), y);
            }
            0x0A | 0x0B | 0x0C => {
                // LF, VT, FF: line feed
                self.grid.line_feed();
            }
            0x0D => {
                // CR: carriage return
                let (_, y) = self.grid.cursor();
                self.grid.set_cursor(0, y);
            }
            _ => {}
        }
    }

    /// Dispatch a CSI sequence.
    fn dispatch_csi(&mut self, params: &mux_parser::CsiParams) {
        match params.final_byte {
            b'H' | b'f' => {
                // CUP: cursor position
                let row = params.get(0, 1).saturating_sub(1);
                let col = params.get(1, 1).saturating_sub(1);
                self.grid.set_cursor(col, row);
            }
            b'A' => {
                // CUU: cursor up
                let n = params.get(0, 1);
                let (x, y) = self.grid.cursor();
                self.grid.set_cursor(x, y.saturating_sub(n));
            }
            b'B' => {
                // CUD: cursor down
                let n = params.get(0, 1);
                let (x, y) = self.grid.cursor();
                self.grid.set_cursor(x, y + n);
            }
            b'C' => {
                // CUF: cursor forward
                let n = params.get(0, 1);
                let (x, y) = self.grid.cursor();
                self.grid.set_cursor(x + n, y);
            }
            b'D' => {
                // CUB: cursor backward
                let n = params.get(0, 1);
                let (x, y) = self.grid.cursor();
                self.grid.set_cursor(x.saturating_sub(n), y);
            }
            b'J' => {
                // ED: erase in display
                let mode = params.get(0, 0);
                if mode == 2 {
                    self.grid.clear_screen();
                }
            }
            b'K' => {
                // EL: erase in line
                let mode = params.get(0, 0);
                if mode == 0 {
                    self.grid.clear_to_eol();
                }
            }
            _ => {
                // Other CSI sequences not yet implemented in scaffold
            }
        }
    }

    /// Resize the pane.
    pub fn resize(&mut self, size: Size) {
        self.grid.resize(size.cols, size.rows);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pane_new() {
        let pane = Pane::new(PaneId::new(1), Size::new(80, 24));
        assert_eq!(pane.id, PaneId::new(1));
        assert!(!pane.exited);
    }

    #[test]
    fn pane_process_text() {
        let mut pane = Pane::new(PaneId::new(1), Size::new(80, 24));
        pane.process_output(b"Hello");
        let text = pane.grid.active_text();
        assert_eq!(text[0], "Hello");
    }

    #[test]
    fn pane_process_cr_lf() {
        let mut pane = Pane::new(PaneId::new(1), Size::new(80, 24));
        pane.process_output(b"Line1\r\nLine2");
        let text = pane.grid.active_text();
        assert_eq!(text[0], "Line1");
        assert_eq!(text[1], "Line2");
    }

    #[test]
    fn pane_cursor_movement() {
        let mut pane = Pane::new(PaneId::new(1), Size::new(80, 24));
        // CUP to row 5, col 10
        pane.process_output(b"\x1b[5;10H");
        assert_eq!(pane.grid.cursor(), (9, 4)); // 0-based
    }

    #[test]
    fn pane_clear_screen() {
        let mut pane = Pane::new(PaneId::new(1), Size::new(80, 24));
        pane.process_output(b"Hello\x1b[2J");
        let text = pane.grid.active_text();
        assert!(text.iter().all(|l| l.is_empty()));
    }

    #[test]
    fn pane_backspace() {
        let mut pane = Pane::new(PaneId::new(1), Size::new(80, 24));
        pane.process_output(b"AB\x08C");
        let text = pane.grid.active_text();
        assert_eq!(text[0], "AC");
    }

    #[test]
    fn pane_resize() {
        let mut pane = Pane::new(PaneId::new(1), Size::new(80, 24));
        pane.resize(Size::new(120, 40));
        assert_eq!(pane.grid.sx(), 120);
        assert_eq!(pane.grid.sy(), 40);
    }
}
