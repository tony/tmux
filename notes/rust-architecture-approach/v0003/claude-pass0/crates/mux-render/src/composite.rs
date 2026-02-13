//! CompositeGrid: the double-buffer target for rendering.
//!
//! Each render cycle writes to "next", diffs against "prev", emits
//! escape sequences, then swaps via `std::mem::swap`.

use mux_types::{Cell, PaneId};

/// Source of a cell in the composite grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellSource {
    /// Content from a specific pane.
    Pane(PaneId),
    /// Pane border separator.
    Border,
    /// Status line content.
    StatusLine,
    /// Unused/empty area.
    Empty,
}

/// A cell in the composite grid with source tracking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompositeCell {
    /// The terminal cell content.
    pub cell: Cell,
    /// Where this cell came from.
    pub source: CellSource,
}

impl CompositeCell {
    /// Create an empty composite cell.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            cell: Cell::empty(),
            source: CellSource::Empty,
        }
    }

    /// Create a border cell with a given character.
    #[must_use]
    pub fn border(c: char) -> Self {
        Self {
            cell: Cell::from_char(c),
            source: CellSource::Border,
        }
    }
}

impl Default for CompositeCell {
    fn default() -> Self {
        Self::empty()
    }
}

/// A 2D grid for compositing pane content, borders, and status.
#[derive(Debug, Clone)]
pub struct CompositeGrid {
    cells: Vec<CompositeCell>,
    cols: u16,
    rows: u16,
}

impl CompositeGrid {
    /// Create a new composite grid filled with empty cells.
    #[must_use]
    pub fn new(cols: u16, rows: u16) -> Self {
        let total = cols as usize * rows as usize;
        Self {
            cells: vec![CompositeCell::empty(); total],
            cols,
            rows,
        }
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

    /// Get a cell at (col, row).
    #[must_use]
    pub fn get(&self, col: u16, row: u16) -> Option<&CompositeCell> {
        if col < self.cols && row < self.rows {
            self.cells.get(row as usize * self.cols as usize + col as usize)
        } else {
            None
        }
    }

    /// Set a cell at (col, row).
    pub fn set(&mut self, col: u16, row: u16, cell: CompositeCell) {
        if col < self.cols && row < self.rows {
            let idx = row as usize * self.cols as usize + col as usize;
            if let Some(dest) = self.cells.get_mut(idx) {
                *dest = cell;
            }
        }
    }

    /// Clear all cells to empty.
    pub fn clear(&mut self) {
        for cell in &mut self.cells {
            *cell = CompositeCell::empty();
        }
    }

    /// Blit pane content into a rectangular region.
    pub fn blit_pane(
        &mut self,
        pane_id: PaneId,
        lines: &[&[Cell]],
        dest_x: u16,
        dest_y: u16,
        width: u16,
        height: u16,
    ) {
        for (dy, line) in lines.iter().enumerate().take(height as usize) {
            let row = dest_y + dy as u16;
            for (dx, cell) in line.iter().enumerate().take(width as usize) {
                self.set(
                    dest_x + dx as u16,
                    row,
                    CompositeCell {
                        cell: *cell,
                        source: CellSource::Pane(pane_id),
                    },
                );
            }
        }
    }

    /// Draw a horizontal border.
    pub fn draw_horizontal_border(&mut self, y: u16, x_start: u16, x_end: u16) {
        for x in x_start..x_end {
            self.set(x, y, CompositeCell::border('\u{2500}')); // ─
        }
    }

    /// Draw a vertical border.
    pub fn draw_vertical_border(&mut self, x: u16, y_start: u16, y_end: u16) {
        for y in y_start..y_end {
            self.set(x, y, CompositeCell::border('\u{2502}')); // │
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composite_grid_new() {
        let g = CompositeGrid::new(80, 24);
        assert_eq!(g.cols(), 80);
        assert_eq!(g.rows(), 24);
    }

    #[test]
    fn composite_grid_get_set() {
        let mut g = CompositeGrid::new(10, 10);
        let cell = CompositeCell::border('X');
        g.set(5, 5, cell.clone());
        let retrieved = g.get(5, 5);
        assert_eq!(retrieved, Some(&cell));
    }

    #[test]
    fn composite_grid_out_of_bounds() {
        let g = CompositeGrid::new(10, 10);
        assert!(g.get(10, 0).is_none());
        assert!(g.get(0, 10).is_none());
    }

    #[test]
    fn composite_grid_clear() {
        let mut g = CompositeGrid::new(10, 10);
        g.set(0, 0, CompositeCell::border('X'));
        g.clear();
        let cell = g.get(0, 0).cloned().unwrap_or_default();
        assert_eq!(cell.source, CellSource::Empty);
    }

    #[test]
    fn composite_cell_default() {
        let cell = CompositeCell::default();
        assert_eq!(cell.source, CellSource::Empty);
    }

    #[test]
    fn blit_pane() {
        let mut g = CompositeGrid::new(10, 5);
        let cells = vec![Cell::from_char('A'), Cell::from_char('B')];
        let lines: Vec<&[Cell]> = vec![&cells];
        g.blit_pane(PaneId::new(1), &lines, 0, 0, 2, 1);
        let c = g.get(0, 0).cloned().unwrap_or_default();
        assert_eq!(c.source, CellSource::Pane(PaneId::new(1)));
    }

    #[test]
    fn draw_borders() {
        let mut g = CompositeGrid::new(10, 10);
        g.draw_horizontal_border(5, 0, 10);
        let c = g.get(5, 5).cloned().unwrap_or_default();
        assert_eq!(c.source, CellSource::Border);
    }
}
