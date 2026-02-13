//! Terminal line type with Arc COW semantics.
//!
//! ## Invariants
//! - S96: `Arc<Vec<Cell>>` for scrollback; `Vec<Cell>` for live grid.
//! - INV-032: Scrollback lines use Arc for zero-copy sharing.
//! - INV-024: SmallVec is rejected FINAL for line cells.

use std::sync::Arc;
use crate::cell::Cell;

/// A line of terminal cells with COW (Copy-on-Write) semantics.
///
/// S96: Uses `Arc<Vec<Cell>>` internally. When the line is shared
/// (e.g., in copy mode scrollback), mutations trigger `Arc::make_mut`
/// which clones only when there are multiple owners.
///
/// INV-024: SmallVec rejected FINAL. We use Vec<Cell>.
/// INV-032: Arc for scrollback sharing.
#[derive(Debug, Clone)]
pub struct Line {
    cells: Arc<Vec<Cell>>,
}

impl Line {
    /// Create a new line with the given number of blank cells.
    #[must_use]
    pub fn new(cols: usize) -> Self {
        Self {
            cells: Arc::new(vec![Cell::blank(); cols]),
        }
    }

    /// Create a line from an existing vector of cells.
    #[must_use]
    pub fn from_cells(cells: Vec<Cell>) -> Self {
        Self {
            cells: Arc::new(cells),
        }
    }

    /// Number of cells in this line.
    #[must_use]
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    /// True if the line has no cells.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// Access a cell by index (immutable).
    #[must_use]
    pub fn get(&self, col: usize) -> Option<&Cell> {
        self.cells.get(col)
    }

    /// Access the underlying cell slice.
    #[must_use]
    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    /// Get a mutable reference to the cells, triggering COW if shared.
    ///
    /// S96: `Arc::make_mut` clones the inner Vec only when there are
    /// multiple Arc owners. Single-owner mutation is zero-copy.
    fn cells_mut(&mut self) -> &mut Vec<Cell> {
        Arc::make_mut(&mut self.cells)
    }

    /// Set a cell at the given column index.
    /// INV-012: Grid mutation goes through this method.
    pub fn set(&mut self, col: usize, cell: Cell) {
        let cells = self.cells_mut();
        if col < cells.len() {
            cells[col] = cell;
        }
    }

    /// Clear all cells to blank.
    pub fn clear(&mut self) {
        let cells = self.cells_mut();
        for cell in cells.iter_mut() {
            *cell = Cell::blank();
        }
    }

    /// Resize the line to a new column count, filling new cells with blanks.
    pub fn resize(&mut self, new_cols: usize) {
        let cells = self.cells_mut();
        cells.resize(new_cols, Cell::blank());
    }

    /// True if this line's Arc has multiple owners (shared with scrollback).
    #[must_use]
    pub fn is_shared(&self) -> bool {
        Arc::strong_count(&self.cells) > 1
    }

    /// The Arc reference count (for diagnostics).
    #[must_use]
    pub fn ref_count(&self) -> usize {
        Arc::strong_count(&self.cells)
    }

    /// True if all cells in the line are blank.
    #[must_use]
    pub fn is_blank(&self) -> bool {
        self.cells.iter().all(|c| c.is_blank())
    }

    /// Render line content as a String (for testing/display).
    #[must_use]
    pub fn to_text(&self) -> String {
        self.cells.iter().map(|c| c.grapheme()).collect::<String>().trim_end().to_string()
    }
}

impl PartialEq for Line {
    fn eq(&self, other: &Self) -> bool {
        self.cells == other.cells
    }
}

impl Eq for Line {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::Style;

    #[test]
    fn test_new_line_all_blank() {
        let line = Line::new(80);
        assert_eq!(line.len(), 80);
        assert!(line.is_blank());
    }

    #[test]
    fn test_empty_line() {
        let line = Line::new(0);
        assert!(line.is_empty());
        assert!(line.is_blank());
    }

    #[test]
    fn test_from_cells() {
        let cells = vec![Cell::from_char('A', Style::default()), Cell::blank()];
        let line = Line::from_cells(cells);
        assert_eq!(line.len(), 2);
        assert_eq!(line.get(0).map(|c| c.grapheme()), Some("A"));
    }

    #[test]
    fn test_set_cell() {
        let mut line = Line::new(80);
        let cell = Cell::from_char('X', Style::default());
        line.set(0, cell);
        assert_eq!(line.get(0).map(|c| c.grapheme()), Some("X"));
    }

    #[test]
    fn test_set_out_of_bounds_is_noop() {
        let mut line = Line::new(5);
        line.set(10, Cell::from_char('X', Style::default()));
        // should not panic, just noop
        assert!(line.is_blank());
    }

    /// S96: COW triggers clone only on shared mutation.
    #[test]
    fn test_cow_clone_on_shared_mutation() {
        let line = Line::new(10);
        let mut clone = line.clone();
        // Both share the same Arc
        assert!(clone.is_shared());
        assert_eq!(clone.ref_count(), 2);
        // Mutation triggers COW
        clone.set(0, Cell::from_char('A', Style::default()));
        // After COW, clone has its own copy
        assert!(!clone.is_shared());
        assert_eq!(clone.ref_count(), 1);
        // Original is untouched
        assert!(line.get(0).map(|c| c.is_blank()).unwrap_or(false));
    }

    #[test]
    fn test_resize_grow() {
        let mut line = Line::new(10);
        line.resize(20);
        assert_eq!(line.len(), 20);
    }

    #[test]
    fn test_resize_shrink() {
        let mut line = Line::new(10);
        line.resize(5);
        assert_eq!(line.len(), 5);
    }

    #[test]
    fn test_clear() {
        let mut line = Line::new(10);
        line.set(0, Cell::from_char('X', Style::default()));
        line.clear();
        assert!(line.is_blank());
    }

    #[test]
    fn test_to_text() {
        let mut line = Line::new(5);
        line.set(0, Cell::from_char('H', Style::default()));
        line.set(1, Cell::from_char('i', Style::default()));
        assert_eq!(line.to_text(), "Hi");
    }

    #[test]
    fn test_line_equality() {
        let a = Line::new(10);
        let b = Line::new(10);
        assert_eq!(a, b);
    }
}
