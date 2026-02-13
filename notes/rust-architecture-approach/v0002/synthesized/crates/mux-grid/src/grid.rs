//! The ChunkedGrid combining active screen + scrollback.

use crate::chunk::{Chunk, CHUNK_SIZE};
use crate::line::Line;
use crate::snapshot::GridSnapshot;
use mux_types::Size;

/// A chunked terminal grid with scrollback.
///
/// The grid is organized into chunks of 64 lines. The active screen
/// occupies the last `sy` lines; everything above is scrollback.
/// Scrollback is bounded by `hlimit` -- excess chunks are discarded
/// from the front.
#[derive(Debug, Clone)]
pub struct ChunkedGrid {
    /// Grid column width.
    sx: u32,
    /// Grid row height (active screen lines).
    sy: u32,
    /// Scrollback history limit (max lines).
    hlimit: u32,
    /// Line storage in chunks.
    chunks: Vec<Chunk>,
    /// Dirty flags per active screen row.
    dirty: Vec<bool>,
    /// Monotonic revision counter (incremented on any mutation).
    revision: u64,
}

impl ChunkedGrid {
    /// Create a new grid with the given dimensions and scrollback limit.
    #[must_use]
    pub fn new(sx: u32, sy: u32, hlimit: u32) -> Self {
        let active_chunk = Chunk::blank(sx, sy as usize);
        Self {
            sx, sy, hlimit,
            chunks: vec![active_chunk],
            dirty: vec![true; sy as usize],
            revision: 0,
        }
    }

    /// Grid column width.
    #[must_use]
    pub const fn sx(&self) -> u32 {
        self.sx
    }

    /// Grid row height (active screen).
    #[must_use]
    pub const fn sy(&self) -> u32 {
        self.sy
    }

    /// Current revision counter.
    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    /// Get the total number of lines (scrollback + active).
    #[must_use]
    pub fn total_lines(&self) -> usize {
        self.chunks.iter().map(Chunk::len).sum()
    }

    /// Take an immutable snapshot of the grid for the render pipeline.
    ///
    /// Uses Arc-based COW: snapshot creation is O(n_chunks), not O(n_cells).
    #[must_use]
    pub fn snapshot(&self, _scroll_offset: u32) -> GridSnapshot {
        GridSnapshot {
            chunks: self.chunks.clone(),
            dirty: self.dirty.clone(),
            size: Size::new(self.sx, self.sy),
            revision: self.revision,
        }
    }

    /// Mark a row as dirty.
    pub fn mark_dirty(&mut self, row: u32) {
        if let Some(d) = self.dirty.get_mut(row as usize) {
            *d = true;
        }
        self.revision += 1;
    }

    /// Mark all rows as dirty.
    pub fn mark_all_dirty(&mut self) {
        self.dirty.fill(true);
        self.revision += 1;
    }

    /// Clear all dirty flags.
    pub fn clear_dirty(&mut self) {
        self.dirty.fill(false);
    }

    /// Get a line from the active screen (0 = top of visible area).
    #[must_use]
    pub fn active_line(&self, row: u32) -> Option<&Line> {
        let total = self.total_lines();
        let abs_row = total.saturating_sub(self.sy as usize) + row as usize;
        self.line_at(abs_row)
    }

    /// Get a line by absolute index (0 = first scrollback line).
    #[must_use]
    pub fn line_at(&self, abs_row: usize) -> Option<&Line> {
        let mut remaining = abs_row;
        for chunk in &self.chunks {
            if remaining < chunk.len() {
                return chunk.lines.get(remaining);
            }
            remaining -= chunk.len();
        }
        None
    }

    /// Reflow the grid to a new column width.
    ///
    /// Full reflow implementation would re-wrap all lines. This scaffold
    /// resizes each line and marks all dirty.
    pub fn reflow(&mut self, new_sx: u32) {
        self.sx = new_sx;
        for chunk in &mut self.chunks {
            for line in &mut chunk.lines {
                line.resize(new_sx);
            }
        }
        self.mark_all_dirty();
    }

    /// Scroll the active region: move the top line to scrollback,
    /// add a new blank line at the bottom.
    pub fn scroll_up(&mut self) {
        if self.total_lines() < 1 {
            return;
        }
        // The top line of the active region goes to scrollback
        // (it's already in the chunk; we just add a new blank line)
        let last_chunk = self.chunks.last_mut();
        if let Some(chunk) = last_chunk {
            chunk.lines.push(Line::new(self.sx));
        }
        // Trim excess scrollback
        self.trim_scrollback();
        self.mark_all_dirty();
    }

    /// Trim scrollback to stay within hlimit.
    fn trim_scrollback(&mut self) {
        let max_total = self.sy as usize + self.hlimit as usize;
        while self.total_lines() > max_total {
            if let Some(first) = self.chunks.first_mut() {
                if !first.lines.is_empty() {
                    first.lines.remove(0);
                }
                if first.is_empty() {
                    self.chunks.remove(0);
                }
            } else {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mux_grapheme_arena::GraphemeId;
    use mux_types::Cell;

    #[test]
    fn new_grid_dimensions() {
        let g = ChunkedGrid::new(80, 24, 10_000);
        assert_eq!(g.sx(), 80);
        assert_eq!(g.sy(), 24);
        assert_eq!(g.total_lines(), 24);
    }

    #[test]
    fn snapshot_preserves_size() {
        let g = ChunkedGrid::new(80, 24, 10_000);
        let snap = g.snapshot(0);
        assert_eq!(snap.size, Size::new(80, 24));
    }

    #[test]
    fn dirty_tracking() {
        let mut g = ChunkedGrid::new(80, 24, 10_000);
        g.clear_dirty();
        assert!(g.dirty.iter().all(|d| !d));
        g.mark_dirty(5);
        assert!(g.dirty[5]);
    }

    #[test]
    fn mark_all_dirty() {
        let mut g = ChunkedGrid::new(80, 24, 10_000);
        g.clear_dirty();
        g.mark_all_dirty();
        assert!(g.dirty.iter().all(|d| *d));
    }

    #[test]
    fn active_line_access() {
        let g = ChunkedGrid::new(80, 24, 10_000);
        assert!(g.active_line(0).is_some());
        assert!(g.active_line(23).is_some());
        assert!(g.active_line(24).is_none());
    }

    #[test]
    fn reflow_changes_width() {
        let mut g = ChunkedGrid::new(80, 24, 10_000);
        g.reflow(120);
        assert_eq!(g.sx(), 120);
    }

    #[test]
    fn scroll_up_adds_line() {
        let mut g = ChunkedGrid::new(80, 24, 10_000);
        let before = g.total_lines();
        g.scroll_up();
        assert_eq!(g.total_lines(), before + 1);
    }

    #[test]
    fn revision_increments() {
        let mut g = ChunkedGrid::new(80, 24, 10_000);
        let r0 = g.revision();
        g.mark_dirty(0);
        assert!(g.revision() > r0);
    }
}
