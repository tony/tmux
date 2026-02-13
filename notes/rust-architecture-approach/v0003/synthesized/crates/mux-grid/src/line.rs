//! Arc-COW line with cells and dirty flag.
//!
//! Lines use `Arc` for copy-on-write semantics. When a snapshot is taken,
//! the Arc refcount increments cheaply. Only when a mutation occurs does
//! the line get cloned (via `Arc::make_mut` semantics, implemented manually
//! to avoid unsafe).

use std::sync::Arc;
use mux_types::Cell;

/// Inner line data behind the Arc.
#[derive(Debug, Clone)]
struct LineInner {
    cells: Vec<Cell>,
    dirty: bool,
}

/// A terminal line with Arc-COW semantics and dirty tracking.
#[derive(Debug, Clone)]
pub struct Line {
    inner: Arc<LineInner>,
}

impl Line {
    /// Create a new empty line of the given width.
    #[must_use]
    pub fn new(width: u16) -> Self {
        Self {
            inner: Arc::new(LineInner {
                cells: vec![Cell::empty(); width as usize],
                dirty: true,
            }),
        }
    }

    /// Width of the line in cells.
    #[must_use]
    pub fn width(&self) -> u16 {
        self.inner.cells.len() as u16
    }

    /// Get a cell by column index.
    #[must_use]
    pub fn get_cell(&self, col: usize) -> Option<&Cell> {
        self.inner.cells.get(col)
    }

    /// Set a cell, triggering COW clone if shared.
    pub fn set_cell(&mut self, col: usize, cell: Cell) {
        let inner = Arc::make_mut(&mut self.inner);
        if let Some(c) = inner.cells.get_mut(col) {
            *c = cell;
            inner.dirty = true;
        }
    }

    /// Get mutable access to all cells.
    pub fn cells_mut(&mut self) -> &mut [Cell] {
        let inner = Arc::make_mut(&mut self.inner);
        inner.dirty = true;
        &mut inner.cells
    }

    /// Read-only access to cells.
    #[must_use]
    pub fn cells(&self) -> &[Cell] {
        &self.inner.cells
    }

    /// Whether this line has been modified since last clear.
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.inner.dirty
    }

    /// Clear the dirty flag.
    pub fn clear_dirty(&mut self) {
        let inner = Arc::make_mut(&mut self.inner);
        inner.dirty = false;
    }

    /// Clear the line (fill with empty cells).
    pub fn clear(&mut self) {
        let inner = Arc::make_mut(&mut self.inner);
        for cell in &mut inner.cells {
            *cell = Cell::empty();
        }
        inner.dirty = true;
    }

    /// Resize the line to a new width.
    pub fn resize(&mut self, new_width: u16) {
        let inner = Arc::make_mut(&mut self.inner);
        inner.cells.resize(new_width as usize, Cell::empty());
        inner.dirty = true;
    }

    /// Get text content as a trimmed string.
    #[must_use]
    pub fn text_content(&self) -> String {
        let s: String = self.inner.cells.iter().map(|c| c.grapheme).collect();
        s.trim_end().to_owned()
    }

    /// The Arc reference count (useful for verifying COW behavior).
    #[must_use]
    pub fn ref_count(&self) -> usize {
        Arc::strong_count(&self.inner)
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
        let line = Line::new(10);
        assert!(line.is_dirty());
    }

    #[test]
    fn set_cell_marks_dirty() {
        let mut line = Line::new(10);
        line.clear_dirty();
        assert!(!line.is_dirty());
        line.set_cell(0, Cell::from_char('A'));
        assert!(line.is_dirty());
    }

    #[test]
    fn text_content() {
        let mut line = Line::new(10);
        line.set_cell(0, Cell::from_char('H'));
        line.set_cell(1, Cell::from_char('i'));
        assert_eq!(line.text_content(), "Hi");
    }

    #[test]
    fn cow_clone_increments_refcount() {
        let line = Line::new(10);
        let _clone = line.clone();
        assert_eq!(line.ref_count(), 2);
    }

    #[test]
    fn cow_mutation_separates() {
        let line = Line::new(10);
        let mut clone = line.clone();
        assert_eq!(line.ref_count(), 2);
        // Mutation triggers COW separation
        clone.set_cell(0, Cell::from_char('X'));
        assert_eq!(line.ref_count(), 1);
        assert_eq!(clone.ref_count(), 1);
    }

    #[test]
    fn clear_fills_empty() {
        let mut line = Line::new(5);
        line.set_cell(0, Cell::from_char('A'));
        line.clear();
        assert_eq!(line.text_content(), "");
    }

    #[test]
    fn resize_wider() {
        let mut line = Line::new(5);
        line.resize(10);
        assert_eq!(line.width(), 10);
    }

    #[test]
    fn resize_narrower() {
        let mut line = Line::new(10);
        line.resize(5);
        assert_eq!(line.width(), 5);
    }
}
