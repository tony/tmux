//! Cell-by-cell diff between two CompositeGrids.

use crate::composite::CompositeGrid;
use mux_types::Cell;

/// A single cell change from the diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellUpdate {
    pub col: u16,
    pub row: u16,
    pub cell: Cell,
}

/// Compute the diff between two composite grids.
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
                        cell: n.cell.clone(),
                    });
                    if n.cell.is_wide() && col + 1 < cols {
                        if let Some(padding) = next.get(col + 1, row) {
                            updates.push(CellUpdate {
                                col: col + 1,
                                row,
                                cell: padding.cell.clone(),
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

/// Apply a list of cell updates to a composite grid, producing the "next" state.
/// This is the inverse of diff: `apply(prev, diff(prev, next)) == next` for changed cells.
pub fn apply_updates(grid: &mut CompositeGrid, updates: &[CellUpdate]) {
    for update in updates {
        grid.set(
            update.col,
            update.row,
            crate::composite::CompositeCell::content(update.cell.clone()),
        );
    }
}

/// Check if a CUP escape is needed between two updates.
#[must_use]
pub fn needs_cup(prev: &CellUpdate, next: &CellUpdate) -> bool {
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
        let prev = CompositeGrid::new(10, 5);
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
        let u1 = CellUpdate {
            col: 5,
            row: 0,
            cell: Cell::from_char('A'),
        };
        let u2 = CellUpdate {
            col: 6,
            row: 0,
            cell: Cell::from_char('B'),
        };
        assert!(!needs_cup(&u1, &u2));
    }

    #[test]
    fn cup_needed_for_gap() {
        let u1 = CellUpdate {
            col: 5,
            row: 0,
            cell: Cell::from_char('A'),
        };
        let u2 = CellUpdate {
            col: 8,
            row: 0,
            cell: Cell::from_char('B'),
        };
        assert!(needs_cup(&u1, &u2));
    }

    #[test]
    fn cup_needed_for_different_row() {
        let u1 = CellUpdate {
            col: 5,
            row: 0,
            cell: Cell::from_char('A'),
        };
        let u2 = CellUpdate {
            col: 5,
            row: 1,
            cell: Cell::from_char('B'),
        };
        assert!(needs_cup(&u1, &u2));
    }

    #[test]
    fn apply_updates_roundtrip() {
        let prev = CompositeGrid::new(10, 5);
        let mut next = CompositeGrid::new(10, 5);
        next.set(2, 1, CompositeCell::border('Z'));
        let updates = diff_grids(&prev, &next);
        let mut applied = prev;
        apply_updates(&mut applied, &updates);
        // The changed cell should match
        let cell = applied.get(2, 1);
        assert_eq!(cell.map(|c| c.cell.grapheme), Some('Z'));
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use crate::composite::CompositeCell;
    use proptest::prelude::*;

    proptest! {
        /// Render diff roundtrip: diff(prev, next) applied to prev matches next
        /// for all changed cells.
        #[test]
        fn diff_apply_roundtrip(
            changes in proptest::collection::vec(
                (0u16..10, 0u16..5, proptest::char::range('A', 'Z')),
                0..20
            )
        ) {
            let prev = CompositeGrid::new(10, 5);
            let mut next = CompositeGrid::new(10, 5);
            for &(col, row, ch) in &changes {
                next.set(col, row, CompositeCell::border(ch));
            }
            let updates = diff_grids(&prev, &next);
            let mut applied = prev;
            apply_updates(&mut applied, &updates);

            // Verify all changed cells match
            for &(col, row, _) in &changes {
                prop_assert_eq!(
                    applied.get(col, row).map(|c| c.cell.grapheme),
                    next.get(col, row).map(|c| c.cell.grapheme)
                );
            }
        }
    }
}
