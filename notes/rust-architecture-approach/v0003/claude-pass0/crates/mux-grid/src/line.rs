//! Arc-COW line implementation.
//!
//! Each line stores its cells as `Arc<Vec<Cell>>`. Snapshots only increment
//! reference counts. Lines are physically copied only on mutation when
//! `Arc::strong_count > 1`.

use std::sync::Arc;
use mux_types::Cell;

/// A single terminal line with Arc-based COW semantics.
#[derive(Debug, Clone)]
pub struct Line {
    /// Cells stored behind Arc for COW sharing with snapshots.
    cells: Arc<Vec<Cell>>,
    /// Whether this line has been modified since the last render.
    dirty: bool,
}

impl Line {
    /// Create a new line with the given width, filled with empty cells.
    #[must_use]
    pub fn new(width: u16) -> Self {
        Self {
            cells: Arc::new(vec![Cell::empty(); width as usize]),
            dirty: true,
        }
    }

    /// Number of cells in this line.
    #[must_use]
    pub fn width(&self) -> usize {
        self.cells.len()
    }

    /// Read-only access to cells (no COW copy triggered).
    #[must_use]
    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    /// Get a single cell by column index.
    #[must_use]
    pub fn cell(&self, col: usize) -> Option<&Cell> {
        self.cells.get(col)
    }

    /// Mutable access to cells. Triggers a COW copy if the Arc is shared.
    pub fn cells_mut(&mut self) -> &mut Vec<Cell> {
        self.dirty = true;
        Arc::make_mut(&mut self.cells)
    }

    /// Set a single cell, marking the line as dirty.
    ///
    /// # Errors
    ///
    /// Returns `None` if `col` is out of bounds.
    pub fn set_cell(&mut self, col: usize, cell: Cell) -> Option<()> {
        if col >= self.cells.len() {
            return None;
        }
        self.dirty = true;
        Arc::make_mut(&mut self.cells)[col] = cell;
        Some(())
    }

    /// Whether this line has been modified since last `clear_dirty()`.
    #[must_use]
    pub const fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Mark this line as clean (after rendering).
    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    /// Mark this line as dirty (forcing re-render).
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Resize this line to a new width, filling new cells with empty.
    pub fn resize(&mut self, new_width: u16) {
        let new_width = new_width as usize;
        let cells = Arc::make_mut(&mut self.cells);
        cells.resize(new_width, Cell::empty());
        self.dirty = true;
    }

    /// Clear the line (fill with empty cells), preserving width.
    pub fn clear(&mut self) {
        let width = self.cells.len();
        let cells = Arc::make_mut(&mut self.cells);
        for cell in cells.iter_mut() {
            *cell = Cell::empty();
        }
        let _ = width; // used for clarity
        self.dirty = true;
    }

    /// Extract text content from this line (for copy mode / snapshots).
    #[must_use]
    pub fn text_content(&self) -> String {
        let mut s = String::new();
        for cell in self.cells.iter() {
            match cell.grapheme {
                mux_grapheme_arena::GraphemeId::Empty => s.push(' '),
                mux_grapheme_arena::GraphemeId::Inline(c) => s.push(c),
                mux_grapheme_arena::GraphemeId::Interned(_) => s.push('?'),
            }
        }
        // Trim trailing spaces.
        let trimmed = s.trim_end();
        trimmed.to_owned()
    }

    /// Number of Arc strong references (useful for testing COW behavior).
    #[must_use]
    pub fn ref_count(&self) -> usize {
        Arc::strong_count(&self.cells)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_line_width() {
        let line = Line::new(80);
        assert_eq!(line.width(), 80);
    }

    #[test]
    fn new_line_is_dirty() {
        let line = Line::new(80);
        assert!(line.is_dirty());
    }

    #[test]
    fn clear_dirty() {
        let mut line = Line::new(80);
        line.clear_dirty();
        assert!(!line.is_dirty());
    }

    #[test]
    fn set_cell_marks_dirty() {
        let mut line = Line::new(80);
        line.clear_dirty();
        line.set_cell(0, Cell::from_char('A'));
        assert!(line.is_dirty());
    }

    #[test]
    fn set_cell_out_of_bounds() {
        let mut line = Line::new(10);
        assert!(line.set_cell(10, Cell::empty()).is_none());
    }

    #[test]
    fn cow_clone_shares_arc() {
        let line = Line::new(80);
        let clone = line.clone();
        assert_eq!(line.ref_count(), 2);
        assert_eq!(clone.ref_count(), 2);
    }

    #[test]
    fn cow_mutation_detaches() {
        let line = Line::new(80);
        let mut clone = line.clone();
        assert_eq!(line.ref_count(), 2);
        clone.set_cell(0, Cell::from_char('X'));
        assert_eq!(line.ref_count(), 1);
        assert_eq!(clone.ref_count(), 1);
    }

    #[test]
    fn resize_wider() {
        let mut line = Line::new(10);
        line.resize(20);
        assert_eq!(line.width(), 20);
    }

    #[test]
    fn resize_narrower() {
        let mut line = Line::new(20);
        line.resize(10);
        assert_eq!(line.width(), 10);
    }

    #[test]
    fn clear_resets_cells() {
        let mut line = Line::new(10);
        line.set_cell(0, Cell::from_char('A'));
        line.clear();
        assert!(line.cell(0).map_or(false, |c| c.is_empty()));
    }

    #[test]
    fn text_content_basic() {
        let mut line = Line::new(5);
        line.set_cell(0, Cell::from_char('H'));
        line.set_cell(1, Cell::from_char('i'));
        assert_eq!(line.text_content(), "Hi");
    }

    #[test]
    fn text_content_trims_trailing() {
        let mut line = Line::new(10);
        line.set_cell(0, Cell::from_char('A'));
        // Rest are empty (spaces)
        assert_eq!(line.text_content(), "A");
    }
}
