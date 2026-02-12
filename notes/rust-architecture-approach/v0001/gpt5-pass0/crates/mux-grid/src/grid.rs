use mux_types::{Cell, CellWidth, Line};

use crate::Scrollback;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GridError {
    OutOfBounds { row: usize, col: usize },
}

impl std::fmt::Display for GridError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OutOfBounds { row, col } => write!(f, "grid out of bounds at ({row}, {col})"),
        }
    }
}

impl std::error::Error for GridError {}

/// Live grid with mutable `Vec<Cell>` rows and Arc-backed scrollback snapshots (S96, INV-032).
#[derive(Debug, Clone)]
pub struct Grid {
    width: usize,
    height: usize,
    rows: Vec<Vec<Cell>>,
    scrollback: Scrollback,
}

impl Grid {
    pub fn new(width: usize, height: usize, scrollback_max: usize) -> Self {
        let blank = Cell::new(" ", 0, 0, CellWidth::One);
        let rows = (0..height)
            .map(|_| (0..width).map(|_| blank.clone()).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        Self {
            width,
            height,
            rows,
            scrollback: Scrollback::new(scrollback_max),
        }
    }

    pub fn size(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    pub fn scrollback(&self) -> &Scrollback {
        &self.scrollback
    }

    pub fn rows(&self) -> &[Vec<Cell>] {
        &self.rows
    }

    pub fn put_char(&mut self, row: usize, col: usize, ch: char) -> Result<(), GridError> {
        self.put_grapheme(row, col, Cell::from_char(ch))
    }

    pub fn put_grapheme(&mut self, row: usize, col: usize, cell: Cell) -> Result<(), GridError> {
        let slot = self
            .rows
            .get_mut(row)
            .and_then(|r| r.get_mut(col))
            .ok_or(GridError::OutOfBounds { row, col })?;
        *slot = cell;
        Ok(())
    }

    pub fn scroll_up(&mut self) {
        if self.height == 0 {
            return;
        }
        if let Some(first) = self.rows.first().cloned() {
            self.scrollback.push_line(Line::new(first));
            self.rows.remove(0);
            self.rows
                .push((0..self.width).map(|_| Cell::new(" ", 0, 0, CellWidth::One)).collect());
        }
    }

    pub fn viewport(&self, start_row: usize, count: usize) -> &[Vec<Cell>] {
        let start = start_row.min(self.rows.len());
        let end = (start + count).min(self.rows.len());
        &self.rows[start..end]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn put_char_updates_cell() {
        let mut g = Grid::new(4, 2, 8);
        g.put_char(1, 2, 'x').unwrap();
        assert_eq!(g.rows()[1][2].grapheme, "x");
    }

    #[test]
    fn out_of_bounds_rejected() {
        let mut g = Grid::new(2, 2, 8);
        assert!(matches!(
            g.put_char(3, 0, 'x'),
            Err(GridError::OutOfBounds { .. })
        ));
    }

    #[test]
    fn scroll_up_moves_first_line_to_scrollback() {
        let mut g = Grid::new(2, 2, 8);
        g.put_char(0, 0, 'a').unwrap();
        g.scroll_up();
        assert_eq!(g.scrollback().len(), 1);
        assert_eq!(g.scrollback().lines()[0].as_slice()[0].grapheme, "a");
    }

    #[test]
    fn viewport_returns_slice() {
        let g = Grid::new(3, 4, 8);
        assert_eq!(g.viewport(1, 2).len(), 2);
        assert_eq!(g.viewport(99, 2).len(), 0);
    }
}
