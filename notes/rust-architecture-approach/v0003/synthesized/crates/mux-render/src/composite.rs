//! CompositeGrid for frame composition.

use mux_types::Cell;

/// A cell in the composite grid with its source pane info.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompositeCell {
    /// The cell content.
    pub cell: Cell,
    /// Whether this is a border cell.
    pub is_border: bool,
}

impl CompositeCell {
    /// Create a content cell.
    #[must_use]
    pub fn content(cell: Cell) -> Self {
        Self {
            cell,
            is_border: false,
        }
    }

    /// Create a border cell from a character.
    #[must_use]
    pub fn border(c: char) -> Self {
        Self {
            cell: Cell::from_char(c),
            is_border: true,
        }
    }
}

/// A composed frame of cells from multiple panes.
#[derive(Debug, Clone)]
pub struct CompositeGrid {
    cells: Vec<Vec<CompositeCell>>,
    cols: u16,
    rows: u16,
}

impl CompositeGrid {
    /// Create a new empty composite grid.
    #[must_use]
    pub fn new(cols: u16, rows: u16) -> Self {
        let empty = CompositeCell::content(Cell::empty());
        let cells = (0..rows)
            .map(|_| vec![empty.clone(); cols as usize])
            .collect();
        Self { cells, cols, rows }
    }

    /// Grid width.
    #[must_use]
    pub const fn cols(&self) -> u16 {
        self.cols
    }

    /// Grid height.
    #[must_use]
    pub const fn rows(&self) -> u16 {
        self.rows
    }

    /// Get a cell at the given position.
    #[must_use]
    pub fn get(&self, col: u16, row: u16) -> Option<&CompositeCell> {
        self.cells
            .get(row as usize)
            .and_then(|r| r.get(col as usize))
    }

    /// Set a cell at the given position.
    pub fn set(&mut self, col: u16, row: u16, cell: CompositeCell) {
        if let Some(r) = self.cells.get_mut(row as usize) {
            if let Some(c) = r.get_mut(col as usize) {
                *c = cell;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_composite_grid() {
        let g = CompositeGrid::new(80, 24);
        assert_eq!(g.cols(), 80);
        assert_eq!(g.rows(), 24);
    }

    #[test]
    fn get_set_cell() {
        let mut g = CompositeGrid::new(10, 5);
        g.set(3, 2, CompositeCell::border('|'));
        let cell = g.get(3, 2);
        assert!(cell.is_some());
        assert!(cell.map(|c| c.is_border).unwrap_or(false));
    }

    #[test]
    fn out_of_bounds_returns_none() {
        let g = CompositeGrid::new(10, 5);
        assert!(g.get(10, 0).is_none());
        assert!(g.get(0, 5).is_none());
    }

    #[test]
    fn border_cell() {
        let c = CompositeCell::border('-');
        assert!(c.is_border);
        assert_eq!(c.cell.grapheme, '-');
    }

    #[test]
    fn content_cell() {
        let c = CompositeCell::content(Cell::from_char('A'));
        assert!(!c.is_border);
        assert_eq!(c.cell.grapheme, 'A');
    }
}
