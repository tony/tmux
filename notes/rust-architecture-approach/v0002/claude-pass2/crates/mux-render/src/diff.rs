//! Cell-by-cell diff with wide-char invalidation (ratatui pattern).

use crate::composite::CompositeGrid;
use mux_types::Cell;

/// A single cell update from the diff algorithm.
#[derive(Debug, Clone)]
pub struct CellUpdate {
    pub x: u32,
    pub y: u32,
    pub cell: Cell,
}

/// Compute the diff between two composite grids.
///
/// Returns a list of cell updates. Wide-char invalidation ensures
/// that when a multi-width character is written, the following
/// padding cells are also emitted for correct display.
#[must_use]
pub fn diff(prev: &CompositeGrid, next: &CompositeGrid) -> Vec<CellUpdate> {
    let mut updates = Vec::new();
    let len = prev.cells().len().min(next.cells().len());
    let width = next.width() as usize;
    let mut invalidated: usize = 0;

    for i in 0..len {
        let prev_cell = &prev.cells()[i];
        let next_cell = &next.cells()[i];

        if next_cell.cell != prev_cell.cell || invalidated > 0 {
            let x = (i % width) as u32;
            let y = (i / width) as u32;
            updates.push(CellUpdate { x, y, cell: next_cell.cell });

            if next_cell.cell.width > 1 {
                invalidated = (next_cell.cell.width as usize).saturating_sub(1);
            } else if invalidated > 0 {
                invalidated -= 1;
            }
        } else if invalidated > 0 {
            invalidated -= 1;
        }
    }

    updates
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composite::{CompositeCell, CellSource};
    use mux_grapheme_arena::GraphemeId;
    use mux_types::PaneId;

    #[test]
    fn diff_identical_grids() {
        let a = CompositeGrid::new(80, 24);
        let b = CompositeGrid::new(80, 24);
        assert!(diff(&a, &b).is_empty());
    }

    #[test]
    fn diff_single_cell_change() {
        let a = CompositeGrid::new(10, 5);
        let mut b = CompositeGrid::new(10, 5);
        b.set(3, 2, CompositeCell {
            cell: Cell { grapheme: GraphemeId::from_char('A'), ..Cell::empty() },
            source: CellSource::Pane(PaneId(1)),
        });
        let updates = diff(&a, &b);
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].x, 3);
        assert_eq!(updates[0].y, 2);
    }

    #[test]
    fn diff_wide_char_invalidation() {
        let a = CompositeGrid::new(10, 1);
        let mut b = CompositeGrid::new(10, 1);
        b.set(3, 0, CompositeCell {
            cell: Cell {
                grapheme: GraphemeId::from_char('\u{4E16}'),
                width: 2,
                ..Cell::empty()
            },
            source: CellSource::Pane(PaneId(1)),
        });
        b.set(4, 0, CompositeCell {
            cell: Cell::padding(),
            source: CellSource::Pane(PaneId(1)),
        });
        let updates = diff(&a, &b);
        assert!(updates.len() >= 2);
    }
}
