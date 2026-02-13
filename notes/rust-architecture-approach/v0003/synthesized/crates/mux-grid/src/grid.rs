//! The ChunkedGrid -- main terminal grid implementation.
//!
//! Provides a terminal grid with:
//! - Active screen (bottom `sy` lines)
//! - Scrollback buffer (bounded by `hlimit`)
//! - Arc-COW line sharing for efficient snapshots
//! - Dirty tracking for incremental rendering

use crate::line::Line;
use crate::region::ScrollRegion;

/// A terminal grid with chunked storage and scrollback.
#[derive(Debug, Clone)]
pub struct ChunkedGrid {
    /// All lines: scrollback + active screen.
    lines: Vec<Line>,
    /// Screen width in columns.
    sx: u16,
    /// Screen height in rows.
    sy: u16,
    /// Maximum scrollback lines.
    hlimit: u32,
    /// Current scroll region.
    scroll_region: ScrollRegion,
    /// Cursor column (0-based).
    cursor_x: u16,
    /// Cursor row (0-based, relative to active screen top).
    cursor_y: u16,
}

impl ChunkedGrid {
    /// Create a new grid with the given dimensions and scrollback limit.
    #[must_use]
    pub fn new(sx: u16, sy: u16, hlimit: u32) -> Self {
        let lines = (0..sy).map(|_| Line::new(sx)).collect();
        Self {
            lines,
            sx,
            sy,
            hlimit,
            scroll_region: ScrollRegion::full(sy),
            cursor_x: 0,
            cursor_y: 0,
        }
    }

    /// Screen width.
    #[must_use]
    pub const fn sx(&self) -> u16 {
        self.sx
    }

    /// Screen height.
    #[must_use]
    pub const fn sy(&self) -> u16 {
        self.sy
    }

    /// Maximum scrollback lines.
    #[must_use]
    pub const fn hlimit(&self) -> u32 {
        self.hlimit
    }

    /// Total lines (scrollback + active screen).
    #[must_use]
    pub fn total_lines(&self) -> usize {
        self.lines.len()
    }

    /// Number of scrollback lines (above active screen).
    #[must_use]
    pub fn scrollback_len(&self) -> usize {
        self.lines.len().saturating_sub(self.sy as usize)
    }

    /// Current cursor position.
    #[must_use]
    pub const fn cursor(&self) -> (u16, u16) {
        (self.cursor_x, self.cursor_y)
    }

    /// Set cursor position, clamped to screen bounds.
    pub fn set_cursor(&mut self, x: u16, y: u16) {
        self.cursor_x = x.min(self.sx.saturating_sub(1));
        self.cursor_y = y.min(self.sy.saturating_sub(1));
    }

    /// Current scroll region.
    #[must_use]
    pub const fn scroll_region(&self) -> ScrollRegion {
        self.scroll_region
    }

    /// Set scroll region (DECSTBM).
    pub fn set_scroll_region(&mut self, top: u16, bottom: u16) {
        if let Some(region) = ScrollRegion::custom(top, bottom, self.sy) {
            self.scroll_region = region;
        }
    }

    /// Reset scroll region to full screen.
    pub fn reset_scroll_region(&mut self) {
        self.scroll_region = ScrollRegion::full(self.sy);
    }

    /// Index of the first active-screen line in the `lines` vec.
    fn active_start(&self) -> usize {
        self.lines.len().saturating_sub(self.sy as usize)
    }

    /// Get an active-screen line by row (0-based).
    #[must_use]
    pub fn active_line(&self, row: u16) -> Option<&Line> {
        let idx = self.active_start() + row as usize;
        self.lines.get(idx)
    }

    /// Get a mutable active-screen line by row (0-based).
    pub fn active_line_mut(&mut self, row: u16) -> Option<&mut Line> {
        let idx = self.active_start() + row as usize;
        self.lines.get_mut(idx)
    }

    /// Write a character at the cursor position and advance cursor.
    pub fn write_char(&mut self, c: char) {
        let row = self.cursor_y;
        let col = self.cursor_x;
        if let Some(line) = self.active_line_mut(row) {
            let cell = mux_types::Cell::from_char(c);
            line.set_cell(col as usize, cell);
        }
        self.cursor_x += 1;
        if self.cursor_x >= self.sx {
            self.cursor_x = 0;
            self.line_feed();
        }
    }

    /// Move to the next line (LF). Scrolls if at bottom of scroll region.
    pub fn line_feed(&mut self) {
        if self.cursor_y == self.scroll_region.bottom {
            self.scroll_up(1);
        } else if self.cursor_y < self.sy.saturating_sub(1) {
            self.cursor_y += 1;
        }
    }

    /// Scroll the scroll region up by `count` lines.
    pub fn scroll_up(&mut self, count: u16) {
        let start = self.active_start();
        let region_top = start + self.scroll_region.top as usize;
        let region_bottom = start + self.scroll_region.bottom as usize;

        for _ in 0..count {
            if region_top <= region_bottom && region_bottom < self.lines.len() {
                if self.scroll_region.is_full_screen(self.sy) {
                    self.lines.push(Line::new(self.sx));
                } else if region_top < self.lines.len() {
                    self.lines.remove(region_top);
                    let insert_at = region_top
                        + (self.scroll_region.bottom - self.scroll_region.top) as usize;
                    let insert_at = insert_at.min(self.lines.len());
                    self.lines.insert(insert_at, Line::new(self.sx));
                }
            }
        }
        self.trim_scrollback();
    }

    /// Scroll the scroll region down by `count` lines.
    pub fn scroll_down(&mut self, count: u16) {
        let start = self.active_start();
        let region_top = start + self.scroll_region.top as usize;
        let region_bottom = start + self.scroll_region.bottom as usize;

        for _ in 0..count {
            if region_top <= region_bottom && region_bottom < self.lines.len() {
                self.lines.remove(region_bottom);
                self.lines.insert(region_top, Line::new(self.sx));
            }
        }
    }

    /// Trim scrollback to the configured limit.
    fn trim_scrollback(&mut self) {
        let max_total = self.sy as usize + self.hlimit as usize;
        if self.lines.len() > max_total {
            let excess = self.lines.len() - max_total;
            self.lines.drain(..excess);
        }
    }

    /// Clear the entire active screen.
    pub fn clear_screen(&mut self) {
        let start = self.active_start();
        for line in &mut self.lines[start..] {
            line.clear();
        }
    }

    /// Clear from cursor to end of line.
    pub fn clear_to_eol(&mut self) {
        let row = self.cursor_y;
        let col = self.cursor_x as usize;
        if let Some(line) = self.active_line_mut(row) {
            let cells = line.cells_mut();
            for cell in &mut cells[col..] {
                *cell = mux_types::Cell::empty();
            }
        }
    }

    /// Resize the grid to new dimensions.
    pub fn resize(&mut self, new_sx: u16, new_sy: u16) {
        for line in &mut self.lines {
            line.resize(new_sx);
        }
        let current_active = self.sy as usize;
        let new_active = new_sy as usize;
        if new_active > current_active {
            let to_add = new_active - current_active;
            for _ in 0..to_add {
                self.lines.push(Line::new(new_sx));
            }
        }
        self.sx = new_sx;
        self.sy = new_sy;
        self.scroll_region = ScrollRegion::full(new_sy);
        self.cursor_x = self.cursor_x.min(new_sx.saturating_sub(1));
        self.cursor_y = self.cursor_y.min(new_sy.saturating_sub(1));
        self.trim_scrollback();
    }

    /// Mark all active lines as clean.
    pub fn clear_all_dirty(&mut self) {
        let start = self.active_start();
        for line in &mut self.lines[start..] {
            line.clear_dirty();
        }
    }

    /// Collect dirty row indices for the active screen.
    #[must_use]
    pub fn dirty_rows(&self) -> Vec<u16> {
        let start = self.active_start();
        self.lines[start..]
            .iter()
            .enumerate()
            .filter(|(_, line)| line.is_dirty())
            .map(|(i, _)| i as u16)
            .collect()
    }

    /// Extract text content of the active screen as lines.
    #[must_use]
    pub fn active_text(&self) -> Vec<String> {
        let start = self.active_start();
        self.lines[start..]
            .iter()
            .map(Line::text_content)
            .collect()
    }

    /// Create a snapshot: clone all active lines (Arc-COW, cheap).
    #[must_use]
    pub fn snapshot_active(&self) -> Vec<Line> {
        let start = self.active_start();
        self.lines[start..].to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_grid_dimensions() {
        let g = ChunkedGrid::new(80, 24, 1000);
        assert_eq!(g.sx(), 80);
        assert_eq!(g.sy(), 24);
        assert_eq!(g.hlimit(), 1000);
        assert_eq!(g.total_lines(), 24);
    }

    #[test]
    fn cursor_starts_at_origin() {
        let g = ChunkedGrid::new(80, 24, 1000);
        assert_eq!(g.cursor(), (0, 0));
    }

    #[test]
    fn set_cursor_clamps() {
        let mut g = ChunkedGrid::new(80, 24, 1000);
        g.set_cursor(100, 30);
        assert_eq!(g.cursor(), (79, 23));
    }

    #[test]
    fn write_char_advances_cursor() {
        let mut g = ChunkedGrid::new(80, 24, 1000);
        g.write_char('A');
        assert_eq!(g.cursor(), (1, 0));
    }

    #[test]
    fn write_char_wraps_at_eol() {
        let mut g = ChunkedGrid::new(5, 3, 0);
        for c in "Hello".chars() {
            g.write_char(c);
        }
        assert_eq!(g.cursor(), (0, 1));
    }

    #[test]
    fn scroll_up_adds_scrollback() {
        let mut g = ChunkedGrid::new(80, 24, 100);
        g.scroll_up(1);
        assert_eq!(g.scrollback_len(), 1);
        assert_eq!(g.total_lines(), 25);
    }

    #[test]
    fn trim_scrollback() {
        let mut g = ChunkedGrid::new(80, 24, 5);
        for _ in 0..10 {
            g.scroll_up(1);
        }
        assert!(g.scrollback_len() <= 5);
    }

    #[test]
    fn active_line_access() {
        let mut g = ChunkedGrid::new(80, 24, 0);
        g.write_char('X');
        let line = g.active_line(0);
        assert!(line.is_some());
        let text = line.map(Line::text_content).unwrap_or_default();
        assert!(text.starts_with('X'));
    }

    #[test]
    fn clear_screen() {
        let mut g = ChunkedGrid::new(80, 24, 0);
        g.write_char('A');
        g.clear_screen();
        let text = g.active_text();
        assert!(text.iter().all(String::is_empty));
    }

    #[test]
    fn clear_to_eol() {
        let mut g = ChunkedGrid::new(10, 1, 0);
        for c in "ABCDE".chars() {
            g.write_char(c);
        }
        g.set_cursor(2, 0);
        g.clear_to_eol();
        let text = g.active_text();
        assert_eq!(text[0], "AB");
    }

    #[test]
    fn resize_wider() {
        let mut g = ChunkedGrid::new(80, 24, 0);
        g.resize(120, 24);
        assert_eq!(g.sx(), 120);
        assert_eq!(g.active_line(0).map(Line::width).unwrap_or(0), 120);
    }

    #[test]
    fn resize_taller() {
        let mut g = ChunkedGrid::new(80, 24, 0);
        g.resize(80, 30);
        assert_eq!(g.sy(), 30);
        assert_eq!(g.total_lines(), 30);
    }

    #[test]
    fn dirty_rows_initially_all() {
        let g = ChunkedGrid::new(80, 24, 0);
        assert_eq!(g.dirty_rows().len(), 24);
    }

    #[test]
    fn clear_all_dirty() {
        let mut g = ChunkedGrid::new(80, 24, 0);
        g.clear_all_dirty();
        assert!(g.dirty_rows().is_empty());
    }

    #[test]
    fn snapshot_is_cow() {
        let mut g = ChunkedGrid::new(80, 24, 0);
        g.write_char('A');
        let snap = g.snapshot_active();
        // Snapshot shares Arc with grid
        assert!(snap[0].ref_count() >= 2);
    }

    #[test]
    fn scroll_region_custom() {
        let mut g = ChunkedGrid::new(80, 24, 0);
        g.set_scroll_region(5, 15);
        assert_eq!(g.scroll_region().top, 5);
        assert_eq!(g.scroll_region().bottom, 15);
    }

    #[test]
    fn scroll_region_reset() {
        let mut g = ChunkedGrid::new(80, 24, 0);
        g.set_scroll_region(5, 15);
        g.reset_scroll_region();
        assert!(g.scroll_region().is_full_screen(24));
    }

    #[test]
    fn scroll_down() {
        let mut g = ChunkedGrid::new(80, 24, 0);
        g.write_char('A');
        g.scroll_down(1);
        // First active line should now be empty (new line inserted at top)
        assert_eq!(g.active_line(0).map(Line::text_content).unwrap_or_default(), "");
    }

    #[test]
    fn scrollback_len_zero_without_scroll() {
        let g = ChunkedGrid::new(80, 24, 100);
        assert_eq!(g.scrollback_len(), 0);
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// Grid COW isolation: concurrent snapshots maintain isolation
        /// after mutation of the original grid.
        #[test]
        fn cow_snapshot_isolation(
            chars in proptest::collection::vec(proptest::char::range('A', 'Z'), 1..20)
        ) {
            let mut g = ChunkedGrid::new(80, 24, 0);
            for &c in &chars {
                g.write_char(c);
            }
            // Take snapshot
            let snap = g.snapshot_active();
            let snap_text: Vec<String> = snap.iter().map(Line::text_content).collect();
            // Mutate original
            g.clear_screen();
            // Snapshot should still have the original content
            let snap_after: Vec<String> = snap.iter().map(Line::text_content).collect();
            prop_assert_eq!(snap_text, snap_after);
        }
    }
}
