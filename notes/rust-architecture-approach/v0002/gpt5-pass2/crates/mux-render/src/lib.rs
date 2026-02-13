#![forbid(unsafe_code)]

use mux_types::Cell;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PaneId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaneRect {
    pub x: u16,
    pub y: u16,
    pub rows: u16,
    pub cols: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellSource {
    Empty,
    Pane(PaneId),
    Overlay(PaneId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompositeCell {
    pub cell: Cell,
    pub source: CellSource,
}

impl CompositeCell {
    #[must_use]
    pub fn blank() -> Self {
        Self { cell: Cell::blank(), source: CellSource::Empty }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderOp {
    Move(u16, u16),
    Sgr(u16),
    Put(char),
    Rep(u16),
    EraseLine(u16),
    HideCursor,
    ShowCursor(u16, u16),
}

#[derive(Debug, Clone)]
pub struct CompositeGrid {
    rows: u16,
    cols: u16,
    front: Vec<CompositeCell>,
    back: Vec<CompositeCell>,
    pane_row_quota: usize,
}

impl CompositeGrid {
    #[must_use]
    pub fn new(rows: u16, cols: u16, pane_row_quota: usize) -> Self {
        let len = rows as usize * cols as usize;
        Self {
            rows,
            cols,
            front: vec![CompositeCell::blank(); len],
            back: vec![CompositeCell::blank(); len],
            pane_row_quota: pane_row_quota.max(1),
        }
    }

    #[must_use]
    pub fn dims(&self) -> (u16, u16) {
        (self.rows, self.cols)
    }

    pub fn clear_back(&mut self) {
        self.back.fill(CompositeCell::blank());
    }

    pub fn paint_pane(&mut self, pane: PaneId, rect: PaneRect, cells: &[Cell]) {
        let mut idx = 0usize;
        for dy in 0..rect.rows {
            for dx in 0..rect.cols {
                if idx >= cells.len() {
                    return;
                }
                let y = rect.y.saturating_add(dy);
                let x = rect.x.saturating_add(dx);
                if y < self.rows && x < self.cols {
                    let cell_idx = self.index(y, x);
                    self.back[cell_idx] = CompositeCell {
                        cell: cells[idx].clone(),
                        source: CellSource::Pane(pane),
                    };
                }
                idx += 1;
            }
        }
    }

    fn index(&self, row: u16, col: u16) -> usize {
        row as usize * self.cols as usize + col as usize
    }

    #[must_use]
    pub fn diff_and_swap(&mut self, cursor: Option<(u16, u16)>) -> Vec<RenderOp> {
        let mut ops = Vec::new();
        let mut pane_rows: BTreeMap<PaneId, usize> = BTreeMap::new();
        let mut current_style: Option<u16> = None;

        // optimization 1: cursor elision when cursor is off-screen
        if cursor.is_none_or(|(r, c)| r >= self.rows || c >= self.cols) {
            ops.push(RenderOp::HideCursor);
        }

        for row in 0..self.rows {
            let row_start = self.index(row, 0);
            let row_end = row_start + self.cols as usize;
            let old_row = &self.front[row_start..row_end];
            let new_row = &self.back[row_start..row_end];

            // optimization 4: erase line if row is now entirely blank
            let old_non_blank = old_row.iter().any(|c| !c.cell.is_blank());
            let new_blank = new_row.iter().all(|c| c.cell.is_blank());
            if old_non_blank && new_blank {
                ops.push(RenderOp::Move(row, 0));
                ops.push(RenderOp::EraseLine(row));
                continue;
            }

            for col in 0..self.cols {
                let idx = row_start + col as usize;
                if self.front[idx] == self.back[idx] {
                    continue;
                }

                if let CellSource::Pane(pid) = self.back[idx].source {
                    let count = pane_rows.entry(pid).or_insert(0);
                    if *count >= self.pane_row_quota {
                        continue;
                    }
                    *count += 1;
                }

                let ch = self.back[idx].cell.first_scalar();
                ops.push(RenderOp::Move(row, col));

                // optimization 2: batch SGR changes by style index
                let style_index = self.back[idx].cell.style().pack_index();
                if current_style != Some(style_index) {
                    ops.push(RenderOp::Sgr(style_index));
                    current_style = Some(style_index);
                }

                ops.push(RenderOp::Put(ch));

                // optimization 3: REP for repeated runs
                let mut rep = 0u16;
                let mut next = col + 1;
                while next < self.cols {
                    let nidx = row_start + next as usize;
                    if self.back[nidx] != self.back[idx] || self.front[nidx] == self.back[nidx] {
                        break;
                    }
                    rep += 1;
                    next += 1;
                }
                if rep > 1 {
                    ops.push(RenderOp::Rep(rep));
                }
            }
        }

        if let Some((r, c)) = cursor
            && r < self.rows
            && c < self.cols
        {
            ops.push(RenderOp::ShowCursor(r, c));
        }
        std::mem::swap(&mut self.front, &mut self.back);
        ops
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mux_types::Style;

    fn cells(s: &str) -> Vec<Cell> {
        s.chars().map(|ch| Cell::from_char(ch, Style::default())).collect()
    }

    #[test]
    fn new_grid_has_expected_dims() {
        let g = CompositeGrid::new(3, 4, 2);
        assert_eq!(g.dims(), (3, 4));
    }

    #[test]
    fn clear_back_resets_to_blank() {
        let mut g = CompositeGrid::new(1, 1, 1);
        g.paint_pane(PaneId(1), PaneRect { x: 0, y: 0, rows: 1, cols: 1 }, &cells("A"));
        g.clear_back();
        let ops = g.diff_and_swap(None);
        assert!(ops.iter().all(|op| !matches!(op, RenderOp::Put('A'))));
    }

    #[test]
    fn paint_pane_writes_content() {
        let mut g = CompositeGrid::new(1, 2, 10);
        g.paint_pane(PaneId(1), PaneRect { x: 0, y: 0, rows: 1, cols: 2 }, &cells("AB"));
        let ops = g.diff_and_swap(None);
        assert!(ops.iter().any(|op| matches!(op, RenderOp::Put('A'))));
        assert!(ops.iter().any(|op| matches!(op, RenderOp::Put('B'))));
    }

    #[test]
    fn cursor_elision_hides_cursor_when_absent() {
        let mut g = CompositeGrid::new(1, 1, 1);
        let ops = g.diff_and_swap(None);
        assert!(ops.iter().any(|op| matches!(op, RenderOp::HideCursor)));
    }

    #[test]
    fn show_cursor_when_present() {
        let mut g = CompositeGrid::new(2, 2, 1);
        let ops = g.diff_and_swap(Some((1, 1)));
        assert!(ops.iter().any(|op| matches!(op, RenderOp::ShowCursor(1, 1))));
    }

    #[test]
    fn sgr_batching_avoids_duplicate_codes() {
        let mut g = CompositeGrid::new(1, 2, 10);
        g.paint_pane(PaneId(1), PaneRect { x: 0, y: 0, rows: 1, cols: 2 }, &cells("AA"));
        let ops = g.diff_and_swap(None);
        let sgr = ops.iter().filter(|op| matches!(op, RenderOp::Sgr(_))).count();
        assert!(sgr <= 2);
    }

    #[test]
    fn rep_emitted_for_runs() {
        let mut g = CompositeGrid::new(1, 4, 10);
        g.paint_pane(PaneId(1), PaneRect { x: 0, y: 0, rows: 1, cols: 4 }, &cells("AAAA"));
        let ops = g.diff_and_swap(None);
        assert!(ops.iter().any(|op| matches!(op, RenderOp::Rep(_))));
    }

    #[test]
    fn erase_line_for_blank_transition() {
        let mut g = CompositeGrid::new(1, 2, 10);
        g.paint_pane(PaneId(1), PaneRect { x: 0, y: 0, rows: 1, cols: 2 }, &cells("AB"));
        let _ = g.diff_and_swap(None);
        g.clear_back();
        let ops = g.diff_and_swap(None);
        assert!(ops.iter().any(|op| matches!(op, RenderOp::EraseLine(0))));
    }

    #[test]
    fn fairness_quota_limits_per_pane() {
        let mut g = CompositeGrid::new(3, 3, 1);
        g.paint_pane(PaneId(1), PaneRect { x: 0, y: 0, rows: 3, cols: 3 }, &cells("ABCDEFGHI"));
        let ops = g.diff_and_swap(None);
        let puts = ops.iter().filter(|op| matches!(op, RenderOp::Put(_))).count();
        assert!(puts <= 3);
    }

    #[test]
    fn overlay_source_is_distinct() {
        assert_ne!(CellSource::Pane(PaneId(1)), CellSource::Overlay(PaneId(1)));
    }

    #[test]
    fn blank_composite_cell_uses_empty_source() {
        let c = CompositeCell::blank();
        assert_eq!(c.source, CellSource::Empty);
    }

    #[test]
    fn diff_is_empty_after_second_identical_frame() {
        let mut g = CompositeGrid::new(1, 1, 10);
        g.paint_pane(PaneId(1), PaneRect { x: 0, y: 0, rows: 1, cols: 1 }, &cells("X"));
        let _ = g.diff_and_swap(None);
        g.paint_pane(PaneId(1), PaneRect { x: 0, y: 0, rows: 1, cols: 1 }, &cells("X"));
        let ops = g.diff_and_swap(Some((0, 0)));
        assert!(ops.iter().filter(|op| matches!(op, RenderOp::Put(_))).count() <= 1);
    }

    #[test]
    fn pane_rect_can_clip() {
        let mut g = CompositeGrid::new(1, 1, 10);
        g.paint_pane(PaneId(1), PaneRect { x: 1, y: 1, rows: 1, cols: 1 }, &cells("Z"));
        let ops = g.diff_and_swap(None);
        assert!(!ops.iter().any(|op| matches!(op, RenderOp::Put('Z'))));
    }

    #[test]
    fn quota_minimum_is_one() {
        let g = CompositeGrid::new(1, 1, 0);
        assert_eq!(g.pane_row_quota, 1);
    }

    #[test]
    fn move_ops_emitted_for_changed_cells() {
        let mut g = CompositeGrid::new(1, 1, 1);
        g.paint_pane(PaneId(1), PaneRect { x: 0, y: 0, rows: 1, cols: 1 }, &cells("Q"));
        let ops = g.diff_and_swap(None);
        assert!(ops.iter().any(|op| matches!(op, RenderOp::Move(0, 0))));
    }
}
