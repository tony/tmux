//! Cell-by-cell diff between two CompositeGrids.
//!
//! The diff algorithm detects changed cells and produces `CellUpdate` records.
//! Wide-char invalidation ensures padding cells are emitted with their parent.

use crate::composite::CompositeGrid;
use mux_types::Cell;

/// A single cell change from the diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellUpdate {
    /// Column position.
    pub col: u16,
    /// Row position.
    pub row: u16,
    /// The new cell content.
    pub cell: Cell,
}

/// Compute the diff between two composite grids.
///
/// Returns a list of cell updates needed to transform `prev` into `next`.
/// Handles wide-char invalidation: if a wide char cell changes, its padding
/// cell is also included.
#[must_use]
pub fn diff_grids(prev: &CompositeGrid, next: &CompositeGrid) -> Vec<CellUpdate> {
    let mut updates = Vec::new();
    let cols = prev.cols().min(next.cols());
    let rows = prev.rows().min(next.rows());

    for row in 0..rows {
        let mut skip_padding = false;
        for col in 0..cols {
            if skip_padding {
                skip_padding = false;
                continue;
            }

            let prev_cell = prev.get(col, row);
            let next_cell = next.get(col, row);

            match (prev_cell, next_cell) {
                (Some(p), Some(n)) if p != n => {
                    updates.push(CellUpdate {
                        col,
                        row,
                        cell: n.cell,
                    });
                    // Wide-char invalidation: if this is a wide char,
                    // also emit the padding cell.
                    if n.cell.is_wide() && col + 1 < cols {
                        if let Some(padding) = next.get(col + 1, row) {
                            updates.push(CellUpdate {
                                col: col + 1,
                                row,
                                cell: padding.cell,
                            });
                            skip_padding = true;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    updates
}

/// Check if a CUP (cursor position) escape is needed between two updates.
/// Returns `true` if the cursor won't naturally be at the right position.
#[must_use]
pub fn needs_cup(prev: &CellUpdate, next: &CellUpdate) -> bool {
    // The cursor auto-advances after printing. If the next update is
    // exactly one column to the right on the same row, no CUP needed.
    if prev.row == next.row {
        let expected_col = prev.col + if prev.cell.is_wide() { 2 } else { 1 };
        return next.col != expected_col;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composite::CompositeCell;

    #[test]
    fn diff_identical_grids() {
        let g = CompositeGrid::new(10, 5);
        let updates = diff_grids(&g, &g);
        assert!(updates.is_empty());
    }

    #[test]
    fn diff_single_change() {
        let mut prev = CompositeGrid::new(10, 5);
        let mut next = CompositeGrid::new(10, 5);
        next.set(3, 2, CompositeCell::border('X'));
        let updates = diff_grids(&prev, &next);
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].col, 3);
        assert_eq!(updates[0].row, 2);
    }

    #[test]
    fn diff_multiple_changes() {
        let prev = CompositeGrid::new(10, 5);
        let mut next = CompositeGrid::new(10, 5);
        next.set(0, 0, CompositeCell::border('A'));
        next.set(5, 3, CompositeCell::border('B'));
        let updates = diff_grids(&prev, &next);
        assert_eq!(updates.len(), 2);
    }

    #[test]
    fn cup_not_needed_for_adjacent() {
        let u1 = CellUpdate { col: 5, row: 0, cell: Cell::from_char('A') };
        let u2 = CellUpdate { col: 6, row: 0, cell: Cell::from_char('B') };
        assert!(!needs_cup(&u1, &u2));
    }

    #[test]
    fn cup_needed_for_gap() {
        let u1 = CellUpdate { col: 5, row: 0, cell: Cell::from_char('A') };
        let u2 = CellUpdate { col: 8, row: 0, cell: Cell::from_char('B') };
        assert!(needs_cup(&u1, &u2));
    }

    #[test]
    fn cup_needed_for_different_row() {
        let u1 = CellUpdate { col: 5, row: 0, cell: Cell::from_char('A') };
        let u2 = CellUpdate { col: 5, row: 1, cell: Cell::from_char('B') };
        assert!(needs_cup(&u1, &u2));
    }
}
