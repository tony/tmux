//! # mux-snapshot
//!
//! Arc-COW grid snapshot for render thread isolation.
//! Snapshots are cheap to create (Arc clone) and provide immutable
//! access to the grid state at a point in time.
//!
//! L3 logic crate.

#![forbid(unsafe_code)]

use mux_grid::{ChunkedGrid, Line};

/// A point-in-time snapshot of a grid's active screen.
#[derive(Debug, Clone)]
pub struct GridSnapshot {
    /// The frozen lines (Arc-COW shared with the live grid).
    lines: Vec<Line>,
    /// Grid width at snapshot time.
    sx: u16,
    /// Grid height at snapshot time.
    sy: u16,
    /// Snapshot sequence number for ordering.
    sequence: u64,
}

impl GridSnapshot {
    /// Create a snapshot from a grid.
    #[must_use]
    pub fn capture(grid: &ChunkedGrid, sequence: u64) -> Self {
        Self {
            lines: grid.snapshot_active(),
            sx: grid.sx(),
            sy: grid.sy(),
            sequence,
        }
    }

    /// Grid width.
    #[must_use]
    pub const fn sx(&self) -> u16 {
        self.sx
    }

    /// Grid height.
    #[must_use]
    pub const fn sy(&self) -> u16 {
        self.sy
    }

    /// Snapshot sequence number.
    #[must_use]
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }

    /// Get a line by row.
    #[must_use]
    pub fn line(&self, row: u16) -> Option<&Line> {
        self.lines.get(row as usize)
    }

    /// Text content of all lines.
    #[must_use]
    pub fn text(&self) -> Vec<String> {
        self.lines.iter().map(Line::text_content).collect()
    }

    /// Number of lines in the snapshot.
    #[must_use]
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_empty_grid() {
        let g = ChunkedGrid::new(80, 24, 0);
        let snap = GridSnapshot::capture(&g, 1);
        assert_eq!(snap.sx(), 80);
        assert_eq!(snap.sy(), 24);
        assert_eq!(snap.sequence(), 1);
        assert_eq!(snap.line_count(), 24);
    }

    #[test]
    fn snapshot_preserves_content() {
        let mut g = ChunkedGrid::new(80, 24, 0);
        g.write_char('A');
        g.write_char('B');
        let snap = GridSnapshot::capture(&g, 1);
        let text = snap.text();
        assert!(text[0].starts_with("AB"));
    }

    #[test]
    fn snapshot_isolation() {
        let mut g = ChunkedGrid::new(80, 24, 0);
        g.write_char('X');
        let snap = GridSnapshot::capture(&g, 1);
        // Mutate grid after snapshot
        g.clear_screen();
        // Snapshot still has original content
        let text = snap.text();
        assert!(text[0].contains('X'));
    }

    #[test]
    fn snapshot_line_access() {
        let g = ChunkedGrid::new(80, 24, 0);
        let snap = GridSnapshot::capture(&g, 1);
        assert!(snap.line(0).is_some());
        assert!(snap.line(23).is_some());
        assert!(snap.line(24).is_none());
    }

    #[test]
    fn snapshot_sequence_ordering() {
        let g = ChunkedGrid::new(80, 24, 0);
        let s1 = GridSnapshot::capture(&g, 1);
        let s2 = GridSnapshot::capture(&g, 2);
        assert!(s1.sequence() < s2.sequence());
    }

    #[test]
    fn snapshot_text_empty_lines() {
        let g = ChunkedGrid::new(10, 3, 0);
        let snap = GridSnapshot::capture(&g, 0);
        let text = snap.text();
        assert_eq!(text.len(), 3);
        // Empty lines should be empty strings.
        for line in &text {
            assert!(line.is_empty() || line.chars().all(|c| c == ' ' || c == '\0'));
        }
    }

    #[test]
    fn snapshot_clone() {
        let mut g = ChunkedGrid::new(80, 24, 0);
        g.write_char('Z');
        let snap = GridSnapshot::capture(&g, 5);
        let snap2 = snap.clone();
        assert_eq!(snap.sequence(), snap2.sequence());
        assert_eq!(snap.sx(), snap2.sx());
        assert_eq!(snap.text(), snap2.text());
    }

    #[test]
    fn snapshot_small_grid() {
        let mut g = ChunkedGrid::new(5, 2, 0);
        g.write_char('H');
        g.write_char('i');
        let snap = GridSnapshot::capture(&g, 1);
        assert_eq!(snap.sx(), 5);
        assert_eq!(snap.sy(), 2);
        assert!(snap.text()[0].starts_with("Hi"));
    }

    #[test]
    fn snapshot_after_line_feed() {
        let mut g = ChunkedGrid::new(80, 24, 0);
        g.write_char('A');
        // LF only advances cursor_y; CR (set_cursor) resets cursor_x.
        g.line_feed();
        g.set_cursor(0, 1);
        g.write_char('B');
        let snap = GridSnapshot::capture(&g, 1);
        let text = snap.text();
        assert!(text[0].starts_with('A'));
        assert!(text[1].starts_with('B'));
    }

    #[test]
    fn snapshot_after_resize() {
        let mut g = ChunkedGrid::new(80, 24, 0);
        g.write_char('R');
        g.resize(40, 12);
        let snap = GridSnapshot::capture(&g, 1);
        assert_eq!(snap.sx(), 40);
        assert_eq!(snap.sy(), 12);
        assert_eq!(snap.line_count(), 12);
    }

    #[test]
    fn snapshot_line_out_of_bounds() {
        let g = ChunkedGrid::new(10, 3, 0);
        let snap = GridSnapshot::capture(&g, 0);
        assert!(snap.line(3).is_none());
        assert!(snap.line(100).is_none());
    }

    #[test]
    fn snapshot_text_empty_grid() {
        let g = ChunkedGrid::new(5, 2, 0);
        let snap = GridSnapshot::capture(&g, 0);
        let text = snap.text();
        assert_eq!(text.len(), 2);
        // All lines should be spaces only (trimmed to empty or whitespace)
        for line in &text {
            assert!(line.chars().all(|c| c == ' '));
        }
    }

    #[test]
    fn snapshot_sequence_tracks() {
        let g = ChunkedGrid::new(10, 3, 0);
        let snap1 = GridSnapshot::capture(&g, 1);
        let snap2 = GridSnapshot::capture(&g, 2);
        assert_eq!(snap1.sequence(), 1);
        assert_eq!(snap2.sequence(), 2);
    }
}
