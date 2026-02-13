//! A single grid line with Arc-based COW for snapshot isolation.

use std::sync::Arc;
use mux_types::Cell;

/// A single row of terminal cells.
///
/// Uses `Arc<Vec<Cell>>` for copy-on-write semantics: taking a snapshot
/// only increments a reference count. Mutation clones the inner Vec
/// only when shared (Arc::make_mut pattern).
#[derive(Debug, Clone)]
pub struct Line {
    cells: Arc<Vec<Cell>>,
}

impl Line {
    /// Create a new blank line of the given width.
    #[must_use]
    pub fn new(width: u32) -> Self {
        Self {
            cells: Arc::new(vec![Cell::empty(); width as usize]),
        }
    }

    /// Number of cells in this line.
    #[must_use]
    pub fn len(&self) -> u32 {
        self.cells.len() as u32
    }

    /// Whether the line is empty (zero cells).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// Get a cell by column index.
    #[must_use]
    pub fn cell(&self, col: u32) -> Option<&Cell> {
        self.cells.get(col as usize)
    }

    /// Get a slice of all cells.
    #[must_use]
    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    /// Set a cell at the given column (COW: clones inner Vec if shared).
    pub fn set_cell(&mut self, col: u32, cell: Cell) {
        let cells = Arc::make_mut(&mut self.cells);
        if let Some(slot) = cells.get_mut(col as usize) {
            *slot = cell;
        }
    }

    /// Resize the line to a new width, filling new cells with blank.
    pub fn resize(&mut self, new_width: u32) {
        let cells = Arc::make_mut(&mut self.cells);
        cells.resize(new_width as usize, Cell::empty());
    }

    /// Clear all cells to blank.
    pub fn clear(&mut self) {
        let cells = Arc::make_mut(&mut self.cells);
        cells.fill(Cell::empty());
    }

    /// Whether this line has any non-empty content (for trimming).
    #[must_use]
    pub fn has_content(&self) -> bool {
        self.cells.iter().any(|c| !c.is_empty())
    }

    /// Number of shared references to this line's data.
    #[must_use]
    pub fn ref_count(&self) -> usize {
        Arc::strong_count(&self.cells)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mux_grapheme_arena::GraphemeId;

    #[test]
    fn new_line_is_blank() {
        let line = Line::new(80);
        assert_eq!(line.len(), 80);
        assert!(line.cell(0).is_some_and(|c| c.is_empty()));
    }

    #[test]
    fn set_cell_works() {
        let mut line = Line::new(80);
        let cell = Cell { grapheme: GraphemeId::from_char('X'), ..Cell::empty() };
        line.set_cell(5, cell);
        assert_eq!(line.cell(5).map(|c| c.grapheme), Some(GraphemeId::Inline('X')));
    }

    #[test]
    fn cow_snapshot_isolation() {
        let mut line = Line::new(80);
        let snapshot = line.clone();
        // Mutation after clone should not affect the snapshot
        line.set_cell(0, Cell { grapheme: GraphemeId::from_char('A'), ..Cell::empty() });
        assert!(snapshot.cell(0).is_some_and(|c| c.is_empty()));
    }

    #[test]
    fn resize_extends_line() {
        let mut line = Line::new(10);
        line.resize(20);
        assert_eq!(line.len(), 20);
    }

    #[test]
    fn resize_truncates_line() {
        let mut line = Line::new(20);
        line.resize(10);
        assert_eq!(line.len(), 10);
    }

    #[test]
    fn clear_resets_all() {
        let mut line = Line::new(10);
        line.set_cell(3, Cell { grapheme: GraphemeId::from_char('X'), ..Cell::empty() });
        line.clear();
        assert!(line.cell(3).is_some_and(|c| c.is_empty()));
    }

    #[test]
    fn has_content_detection() {
        let blank = Line::new(10);
        assert!(!blank.has_content());

        let mut with_content = Line::new(10);
        with_content.set_cell(5, Cell { grapheme: GraphemeId::from_char('A'), ..Cell::empty() });
        assert!(with_content.has_content());
    }
}
