//! # mux-snapshot
//!
//! Grid snapshot creation and text extraction.
//!
//! Snapshots are cheap (Arc-COW) and provide read-only access to grid state
//! for the render pipeline and copy mode.
//!
//! L3 logic crate.

#![forbid(unsafe_code)]

use mux_grid::{ChunkedGrid, Line};
use mux_types::PaneId;

/// A snapshot of a pane's grid state.
///
/// Created by cloning Arc pointers to grid lines (COW), so creation is O(rows)
/// Arc increments, not O(rows * cols) cell copies.
#[derive(Debug, Clone)]
pub struct GridSnapshot {
    /// The pane this snapshot belongs to.
    pub pane_id: PaneId,
    /// Screen width at snapshot time.
    pub sx: u16,
    /// Screen height at snapshot time.
    pub sy: u16,
    /// Active screen lines (Arc-shared with the grid).
    pub lines: Vec<Line>,
    /// Cursor position at snapshot time.
    pub cursor: (u16, u16),
}

impl GridSnapshot {
    /// Create a snapshot from a grid.
    #[must_use]
    pub fn from_grid(pane_id: PaneId, grid: &ChunkedGrid) -> Self {
        Self {
            pane_id,
            sx: grid.sx(),
            sy: grid.sy(),
            lines: grid.snapshot_active(),
            cursor: grid.cursor(),
        }
    }

    /// Extract plain text from the snapshot.
    #[must_use]
    pub fn text(&self) -> Vec<String> {
        self.lines.iter().map(Line::text_content).collect()
    }

    /// Extract a single line's text.
    #[must_use]
    pub fn line_text(&self, row: u16) -> Option<String> {
        self.lines.get(row as usize).map(Line::text_content)
    }

    /// Number of lines in the snapshot.
    #[must_use]
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Extract a rectangular region of text (for block-mode copy).
    #[must_use]
    pub fn extract_rect(&self, x1: u16, y1: u16, x2: u16, y2: u16) -> Vec<String> {
        let mut result = Vec::new();
        let x_start = x1.min(x2) as usize;
        let x_end = x1.max(x2) as usize;
        let y_start = y1.min(y2) as usize;
        let y_end = y1.max(y2) as usize;

        for row in y_start..=y_end {
            if let Some(line) = self.lines.get(row) {
                let cells = line.cells();
                let mut s = String::new();
                for col in x_start..=x_end {
                    if let Some(cell) = cells.get(col) {
                        match cell.grapheme {
                            mux_grapheme_arena::GraphemeId::Empty => s.push(' '),
                            mux_grapheme_arena::GraphemeId::Inline(c) => s.push(c),
                            mux_grapheme_arena::GraphemeId::Interned(_) => s.push('?'),
                        }
                    }
                }
                result.push(s.trim_end().to_owned());
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_grid() -> ChunkedGrid {
        let mut g = ChunkedGrid::new(10, 3, 0);
        for c in "Hello".chars() {
            g.write_char(c);
        }
        g
    }

    #[test]
    fn snapshot_from_grid() {
        let g = make_test_grid();
        let snap = GridSnapshot::from_grid(PaneId::new(1), &g);
        assert_eq!(snap.sx, 10);
        assert_eq!(snap.sy, 3);
        assert_eq!(snap.line_count(), 3);
    }

    #[test]
    fn snapshot_text() {
        let g = make_test_grid();
        let snap = GridSnapshot::from_grid(PaneId::new(1), &g);
        let text = snap.text();
        assert_eq!(text[0], "Hello");
    }

    #[test]
    fn snapshot_line_text() {
        let g = make_test_grid();
        let snap = GridSnapshot::from_grid(PaneId::new(1), &g);
        assert_eq!(snap.line_text(0), Some("Hello".to_owned()));
        assert_eq!(snap.line_text(1), Some(String::new()));
    }

    #[test]
    fn snapshot_is_cow() {
        let g = make_test_grid();
        let snap = GridSnapshot::from_grid(PaneId::new(1), &g);
        // Lines are Arc-shared
        assert!(snap.lines[0].ref_count() >= 2);
    }

    #[test]
    fn extract_rect() {
        let g = make_test_grid();
        let snap = GridSnapshot::from_grid(PaneId::new(1), &g);
        let rect = snap.extract_rect(0, 0, 4, 0);
        assert_eq!(rect, vec!["Hello"]);
    }

    #[test]
    fn extract_rect_partial() {
        let g = make_test_grid();
        let snap = GridSnapshot::from_grid(PaneId::new(1), &g);
        let rect = snap.extract_rect(1, 0, 3, 0);
        assert_eq!(rect, vec!["ell"]);
    }
}
