use mux_grid::Grid;
use mux_pty::Pty;
use mux_parser::Parser;
use mux_types::Size;

pub struct Pane {
    pub id: u32,
    pub grid: Grid,
    pub pty: Option<Pty>, // Option for headless/test panes
    pub parser: Parser,
}

impl Pane {
    pub fn new(id: u32, size: Size) -> Self {
        Self {
            id,
            grid: Grid::new(size.width, size.height),
            pty: None, // Initialized later
            parser: Parser::new(),
        }
    }

    pub fn write_input(&mut self, _data: &[u8]) {
        // Write to PTY
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pane_creation() {
        let p = Pane::new(1, Size { width: 80, height: 24 });
        assert_eq!(p.id, 1);
        assert_eq!(p.grid.size.width, 80);
        assert!(p.pty.is_none());
    }

    #[test]
    fn test_pane_input() {
        let mut p = Pane::new(1, Size { width: 10, height: 10 });
        p.write_input(b"hello");
        // No assertion as it's a stub, but ensures it runs
    }
}
