//! Grid snapshot and text extraction.
//!
//! Provides snapshot capture and text extraction from grid lines.
//! Snapshots are zero-copy via Arc-COW line cloning.

#![forbid(unsafe_code)]

use mux_grapheme_arena::GraphemeArena;
use mux_grid::{ChunkedGrid, Line};
use mux_types::geometry::Size;

/// A snapshot of a pane's grid state at a point in time.
#[derive(Debug, Clone)]
pub struct GridSnapshot {
    /// Snapshot of active screen lines (Arc-cloned, not deep-copied).
    pub lines: Vec<Line>,
    /// Grid dimensions at snapshot time.
    pub size: Size,
    /// Cursor position at snapshot time.
    pub cursor: (u16, u16),
}

impl GridSnapshot {
    /// Create a snapshot from a grid.
    pub fn capture(grid: &ChunkedGrid) -> Self {
        Self {
            lines: grid.snapshot(),
            size: grid.size(),
            cursor: grid.cursor(),
        }
    }

    /// Extract all text content from the snapshot.
    pub fn text(&self, arena: &GraphemeArena) -> Vec<String> {
        self.lines.iter().map(|l| l.text(arena)).collect()
    }

    /// Extract text for a specific row.
    pub fn line_text(&self, row: u16, arena: &GraphemeArena) -> String {
        self.lines
            .get(row as usize)
            .map(|l| l.text(arena))
            .unwrap_or_default()
    }

    /// Number of rows in the snapshot.
    pub fn rows(&self) -> u16 {
        self.lines.len() as u16
    }

    /// Search for a pattern in the snapshot text.
    pub fn contains_text(&self, pattern: &str, arena: &GraphemeArena) -> bool {
        self.text(arena).iter().any(|line| line.contains(pattern))
    }

    /// Find the row index containing a pattern.
    pub fn find_text(&self, pattern: &str, arena: &GraphemeArena) -> Option<u16> {
        self.text(arena)
            .iter()
            .position(|line| line.contains(pattern))
            .map(|i| i as u16)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_snapshot() {
        let mut arena = GraphemeArena::new();
        let mut grid = ChunkedGrid::new(80, 24, 100);
        let id = arena.intern("X");
        grid.write_char(id, 1);

        let snap = GridSnapshot::capture(&grid);
        assert_eq!(snap.size, Size::new(80, 24));
        assert_eq!(snap.rows(), 24);
    }

    #[test]
    fn snapshot_text_extraction() {
        let mut arena = GraphemeArena::new();
        let mut grid = ChunkedGrid::new(80, 24, 100);
        for ch in "Hello".chars() {
            let id = arena.intern(&ch.to_string());
            grid.write_char(id, 1);
        }

        let snap = GridSnapshot::capture(&grid);
        assert_eq!(snap.line_text(0, &arena), "Hello");
    }

    #[test]
    fn snapshot_isolation() {
        let mut arena = GraphemeArena::new();
        let mut grid = ChunkedGrid::new(80, 24, 100);
        let id_a = arena.intern("A");
        grid.write_char(id_a, 1);

        let snap = GridSnapshot::capture(&grid);

        let id_b = arena.intern("B");
        grid.set_cursor(0, 0);
        grid.write_char(id_b, 1);

        assert_eq!(snap.line_text(0, &arena), "A");
        assert_eq!(grid.line_text(0, &arena), "B");
    }

    #[test]
    fn contains_text() {
        let mut arena = GraphemeArena::new();
        let mut grid = ChunkedGrid::new(80, 24, 100);
        for ch in "Compiling".chars() {
            let id = arena.intern(&ch.to_string());
            grid.write_char(id, 1);
        }
        let snap = GridSnapshot::capture(&grid);
        assert!(snap.contains_text("Compil", &arena));
        assert!(!snap.contains_text("Error", &arena));
    }

    #[test]
    fn find_text() {
        let mut arena = GraphemeArena::new();
        let mut grid = ChunkedGrid::new(80, 3, 100);
        grid.set_cursor(0, 1);
        for ch in "target".chars() {
            let id = arena.intern(&ch.to_string());
            grid.write_char(id, 1);
        }
        let snap = GridSnapshot::capture(&grid);
        assert_eq!(snap.find_text("target", &arena), Some(1));
    }

    #[test]
    fn cursor_preserved() {
        let mut grid = ChunkedGrid::new(80, 24, 100);
        grid.set_cursor(10, 5);
        let snap = GridSnapshot::capture(&grid);
        assert_eq!(snap.cursor, (10, 5));
    }

    #[test]
    fn empty_snapshot() {
        let grid = ChunkedGrid::new(80, 24, 100);
        let arena = GraphemeArena::new();
        let snap = GridSnapshot::capture(&grid);
        let text = snap.text(&arena);
        assert!(text.iter().all(String::is_empty));
    }

    #[test]
    fn snapshot_rows() {
        let grid = ChunkedGrid::new(80, 10, 100);
        let snap = GridSnapshot::capture(&grid);
        assert_eq!(snap.rows(), 10);
    }

    #[test]
    fn snapshot_clone() {
        let grid = ChunkedGrid::new(80, 24, 100);
        let snap = GridSnapshot::capture(&grid);
        let snap2 = snap.clone();
        assert_eq!(snap.rows(), snap2.rows());
        assert_eq!(snap.cursor, snap2.cursor);
        assert_eq!(snap.size, snap2.size);
    }

    #[test]
    fn find_text_not_found() {
        let grid = ChunkedGrid::new(80, 24, 100);
        let arena = GraphemeArena::new();
        let snap = GridSnapshot::capture(&grid);
        assert_eq!(snap.find_text("nonexistent", &arena), None);
    }

    #[test]
    fn line_text_out_of_bounds() {
        let grid = ChunkedGrid::new(80, 5, 100);
        let arena = GraphemeArena::new();
        let snap = GridSnapshot::capture(&grid);
        assert_eq!(snap.line_text(99, &arena), "");
    }

    #[test]
    fn text_all_rows() {
        let grid = ChunkedGrid::new(80, 3, 100);
        let arena = GraphemeArena::new();
        let snap = GridSnapshot::capture(&grid);
        let text = snap.text(&arena);
        assert_eq!(text.len(), 3);
    }

    #[test]
    fn multiple_lines_text() {
        let mut arena = GraphemeArena::new();
        let mut grid = ChunkedGrid::new(80, 5, 100);
        for ch in "Line1".chars() {
            grid.write_char(arena.intern(&ch.to_string()), 1);
        }
        grid.set_cursor(0, 1);
        for ch in "Line2".chars() {
            grid.write_char(arena.intern(&ch.to_string()), 1);
        }
        let snap = GridSnapshot::capture(&grid);
        assert!(snap.contains_text("Line1", &arena));
        assert!(snap.contains_text("Line2", &arena));
    }

    mod prop {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn snapshot_preserves_size(cols in 5u16..200, rows in 3u16..100) {
                let grid = ChunkedGrid::new(cols, rows, 100);
                let snap = GridSnapshot::capture(&grid);
                prop_assert_eq!(snap.size, Size::new(cols, rows));
            }
        }
    }
}
