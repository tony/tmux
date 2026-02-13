//! CompositeGrid double-buffer with cell source tracking.

use mux_types::{Cell, PaneId};

/// Where a composite cell came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellSource {
    Pane(PaneId),
    Border,
    StatusLine,
    Empty,
}

impl Default for CellSource {
    fn default() -> Self { Self::Empty }
}

/// A cell in the composite grid with source tracking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompositeCell {
    pub cell: Cell,
    pub source: CellSource,
}

impl Default for CompositeCell {
    fn default() -> Self {
        Self { cell: Cell::empty(), source: CellSource::Empty }
    }
}

/// A flat buffer representing the entire client terminal.
///
/// Indexed as `cells[y * width + x]`. Two of these are maintained
/// as a double buffer for diff rendering.
#[derive(Debug, Clone)]
pub struct CompositeGrid {
    cells: Vec<CompositeCell>,
    width: u32,
    height: u32,
}

impl CompositeGrid {
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            cells: vec![CompositeCell::default(); (width as usize) * (height as usize)],
            width,
            height,
        }
    }

    #[must_use]
    pub const fn width(&self) -> u32 { self.width }

    #[must_use]
    pub const fn height(&self) -> u32 { self.height }

    #[must_use]
    pub fn get(&self, x: u32, y: u32) -> Option<&CompositeCell> {
        if x < self.width && y < self.height {
            self.cells.get((y as usize) * (self.width as usize) + (x as usize))
        } else {
            None
        }
    }

    pub fn set(&mut self, x: u32, y: u32, cell: CompositeCell) {
        if x < self.width && y < self.height {
            let idx = (y as usize) * (self.width as usize) + (x as usize);
            if let Some(slot) = self.cells.get_mut(idx) {
                *slot = cell;
            }
        }
    }

    pub fn clear(&mut self) {
        for cell in &mut self.cells { *cell = CompositeCell::default(); }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.cells = vec![CompositeCell::default(); (width as usize) * (height as usize)];
    }

    #[must_use]
    pub fn cells(&self) -> &[CompositeCell] { &self.cells }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mux_grapheme_arena::GraphemeId;

    #[test]
    fn composite_grid_new() {
        let g = CompositeGrid::new(80, 24);
        assert_eq!(g.width(), 80);
        assert_eq!(g.height(), 24);
        assert_eq!(g.cells().len(), 80 * 24);
    }

    #[test]
    fn composite_grid_set_get() {
        let mut g = CompositeGrid::new(10, 5);
        let cell = CompositeCell {
            cell: Cell { grapheme: GraphemeId::from_char('X'), ..Cell::empty() },
            source: CellSource::Pane(PaneId(1)),
        };
        g.set(3, 2, cell);
        assert_eq!(g.get(3, 2).map(|c| c.source), Some(CellSource::Pane(PaneId(1))));
    }

    #[test]
    fn composite_grid_out_of_bounds() {
        let g = CompositeGrid::new(10, 5);
        assert!(g.get(10, 0).is_none());
        assert!(g.get(0, 5).is_none());
    }

    #[test]
    fn composite_grid_clear() {
        let mut g = CompositeGrid::new(10, 5);
        g.set(0, 0, CompositeCell {
            cell: Cell { grapheme: GraphemeId::from_char('X'), ..Cell::empty() },
            source: CellSource::Pane(PaneId(1)),
        });
        g.clear();
        assert_eq!(g.get(0, 0).map(|c| c.source), Some(CellSource::Empty));
    }

    #[test]
    fn composite_grid_resize() {
        let mut g = CompositeGrid::new(10, 5);
        g.resize(20, 10);
        assert_eq!(g.width(), 20);
        assert_eq!(g.height(), 10);
        assert_eq!(g.cells().len(), 200);
    }
}
