//! Chunked COW terminal grid with dirty tracking and scrollback.
//!
//! Each line stores its cells as `Arc<Vec<Cell>>` for copy-on-write semantics.
//! Snapshots for the render pipeline clone Arc pointers, not cell data.
//! Lines are stored in chunks of 64 for cache-friendly access.

#![forbid(unsafe_code)]

use std::sync::Arc;

use mux_grapheme_arena::{GraphemeArena, GraphemeId};
use mux_types::attrs::{Attrs, CellFlags};
use mux_types::cell::Cell;
use mux_types::colour::Colour;
use mux_types::geometry::Size;

/// Number of lines per chunk for allocation efficiency.
/// Used in future chunked allocation optimization.
const _CHUNK_SIZE: usize = 64;

/// A single line in the grid, stored as `Arc<Vec<Cell>>` for COW semantics.
#[derive(Debug, Clone)]
pub struct Line {
    cells: Arc<Vec<Cell>>,
    dirty: bool,
}

impl Line {
    /// Create a new line of the given width filled with default cells.
    pub fn new(width: u16) -> Self {
        Self {
            cells: Arc::new(vec![Cell::default(); width as usize]),
            dirty: true,
        }
    }

    /// Width of this line in cells.
    pub fn width(&self) -> u16 {
        self.cells.len() as u16
    }

    /// Get a cell by column index.
    pub fn cell(&self, col: u16) -> Option<&Cell> {
        self.cells.get(col as usize)
    }

    /// Get mutable access to a cell, performing COW clone if shared.
    pub fn cell_mut(&mut self, col: u16) -> Option<&mut Cell> {
        self.dirty = true;
        Arc::make_mut(&mut self.cells).get_mut(col as usize)
    }

    /// Returns the Arc strong reference count (for COW testing).
    pub fn ref_count(&self) -> usize {
        Arc::strong_count(&self.cells)
    }

    /// Is this line marked dirty (modified since last render)?
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Clear the dirty flag.
    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    /// Mark this line as dirty.
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Get the text content of this line as a String.
    pub fn text(&self, arena: &GraphemeArena) -> String {
        self.cells
            .iter()
            .filter(|c| !c.is_padding())
            .map(|c| arena.resolve(c.grapheme))
            .collect::<String>()
            .trim_end()
            .to_owned()
    }

    /// Fill the entire line with default cells.
    pub fn clear(&mut self) {
        let width = self.cells.len();
        self.cells = Arc::new(vec![Cell::default(); width]);
        self.dirty = true;
    }

    /// Resize this line to a new width.
    pub fn resize(&mut self, new_width: u16) {
        let mut cells = (*self.cells).clone();
        cells.resize(new_width as usize, Cell::default());
        self.cells = Arc::new(cells);
        self.dirty = true;
    }
}

/// A chunked terminal grid with scrollback support.
///
/// The grid has an active screen region (bottom `sy` lines) and a scrollback
/// region (everything above, bounded by `hlimit`).
#[derive(Debug, Clone)]
pub struct ChunkedGrid {
    /// All lines (scrollback + active screen).
    lines: Vec<Line>,
    /// Screen width in columns.
    sx: u16,
    /// Screen height in rows.
    sy: u16,
    /// Maximum scrollback lines.
    hlimit: u32,
    /// Cursor column position.
    cursor_x: u16,
    /// Cursor row position (relative to active screen).
    cursor_y: u16,
    /// Current SGR attributes for new characters.
    current_attrs: Attrs,
    /// Current foreground colour.
    current_fg: Colour,
    /// Current background colour.
    current_bg: Colour,
    /// Scroll region top (inclusive, 0-based).
    scroll_top: u16,
    /// Scroll region bottom (inclusive, 0-based).
    scroll_bottom: u16,
}

impl ChunkedGrid {
    /// Create a new grid with the given dimensions.
    pub fn new(sx: u16, sy: u16, hlimit: u32) -> Self {
        let mut lines = Vec::with_capacity(sy as usize);
        for _ in 0..sy {
            lines.push(Line::new(sx));
        }
        Self {
            lines,
            sx,
            sy,
            hlimit,
            cursor_x: 0,
            cursor_y: 0,
            current_attrs: Attrs::empty(),
            current_fg: Colour::Default,
            current_bg: Colour::Default,
            scroll_top: 0,
            scroll_bottom: sy.saturating_sub(1),
        }
    }

    /// Screen width.
    pub fn sx(&self) -> u16 {
        self.sx
    }

    /// Screen height.
    pub fn sy(&self) -> u16 {
        self.sy
    }

    /// Maximum scrollback limit.
    pub fn hlimit(&self) -> u32 {
        self.hlimit
    }

    /// Total number of lines (scrollback + active).
    pub fn total_lines(&self) -> usize {
        self.lines.len()
    }

    /// Number of scrollback lines.
    pub fn scrollback_lines(&self) -> usize {
        self.lines.len().saturating_sub(self.sy as usize)
    }

    /// Cursor position as (x, y).
    pub fn cursor(&self) -> (u16, u16) {
        (self.cursor_x, self.cursor_y)
    }

    /// Set cursor position.
    pub fn set_cursor(&mut self, x: u16, y: u16) {
        self.cursor_x = x.min(self.sx.saturating_sub(1));
        self.cursor_y = y.min(self.sy.saturating_sub(1));
    }

    /// Set current SGR attributes for subsequent writes.
    pub fn set_attrs(&mut self, attrs: Attrs, fg: Colour, bg: Colour) {
        self.current_attrs = attrs;
        self.current_fg = fg;
        self.current_bg = bg;
    }

    /// Reset current SGR attributes to defaults.
    pub fn reset_attrs(&mut self) {
        self.current_attrs = Attrs::empty();
        self.current_fg = Colour::Default;
        self.current_bg = Colour::Default;
    }

    /// Get a reference to the active screen line at row index.
    fn active_line(&self, row: u16) -> Option<&Line> {
        let offset = self.scrollback_lines();
        self.lines.get(offset + row as usize)
    }

    /// Get a mutable reference to the active screen line at row index.
    fn active_line_mut(&mut self, row: u16) -> Option<&mut Line> {
        let offset = self.scrollback_lines();
        self.lines.get_mut(offset + row as usize)
    }

    /// Write a character at the current cursor position.
    pub fn write_char(&mut self, grapheme: GraphemeId, width: u8) {
        if self.cursor_x >= self.sx {
            // Auto-wrap: move to next line
            self.cursor_x = 0;
            self.line_feed();
        }

        // Copy values before mutable borrow of line
        let cx = self.cursor_x;
        let sx = self.sx;
        let attrs = self.current_attrs;
        let fg = self.current_fg;
        let bg = self.current_bg;
        let cy = self.cursor_y;

        if let Some(line) = self.active_line_mut(cy) {
            if let Some(cell) = line.cell_mut(cx) {
                cell.grapheme = grapheme;
                cell.width = width;
                cell.attrs = attrs;
                cell.fg = fg;
                cell.bg = bg;
                cell.flags = CellFlags::empty();
            }
            // Set padding cell for wide characters
            if width > 1 && cx + 1 < sx {
                if let Some(pad) = line.cell_mut(cx + 1) {
                    *pad = Cell::default();
                    pad.flags = CellFlags::PADDING;
                    pad.width = 0;
                }
            }
        }

        self.cursor_x += u16::from(width);
    }

    /// Line feed: move cursor down, scrolling if at bottom of scroll region.
    pub fn line_feed(&mut self) {
        if self.cursor_y == self.scroll_bottom {
            self.scroll_up(1);
        } else if self.cursor_y < self.sy.saturating_sub(1) {
            self.cursor_y += 1;
        }
    }

    /// Carriage return: move cursor to column 0.
    pub fn carriage_return(&mut self) {
        self.cursor_x = 0;
    }

    /// Scroll the active screen up by `n` lines within the scroll region.
    pub fn scroll_up(&mut self, n: u16) {
        let offset = self.scrollback_lines();
        for _ in 0..n {
            let remove_idx = offset + self.scroll_top as usize;
            if remove_idx < self.lines.len() {
                // Move line to scrollback (it stays in lines array, just shifts)
                let old_line = self.lines.remove(remove_idx);
                // Insert at scrollback boundary if scroll_top == 0
                if self.scroll_top == 0 {
                    self.lines
                        .insert(offset, old_line);
                }
                // Insert new blank line at scroll_bottom
                let insert_idx = offset + self.scroll_bottom as usize;
                let insert_idx = insert_idx.min(self.lines.len());
                self.lines.insert(insert_idx, Line::new(self.sx));
            }
        }
        // Trim scrollback
        self.trim_scrollback();
        // Mark all lines in scroll region dirty
        let offset = self.scrollback_lines();
        for row in self.scroll_top..=self.scroll_bottom {
            if let Some(line) = self.lines.get_mut(offset + row as usize) {
                line.mark_dirty();
            }
        }
    }

    /// Scroll down by `n` lines within the scroll region.
    pub fn scroll_down(&mut self, n: u16) {
        let offset = self.scrollback_lines();
        for _ in 0..n {
            let remove_idx = offset + self.scroll_bottom as usize;
            if remove_idx < self.lines.len() {
                self.lines.remove(remove_idx);
            }
            let insert_idx = offset + self.scroll_top as usize;
            let insert_idx = insert_idx.min(self.lines.len());
            self.lines.insert(insert_idx, Line::new(self.sx));
        }
        // Mark dirty
        let offset = self.scrollback_lines();
        for row in self.scroll_top..=self.scroll_bottom {
            if let Some(line) = self.lines.get_mut(offset + row as usize) {
                line.mark_dirty();
            }
        }
    }

    /// Trim scrollback to hlimit.
    fn trim_scrollback(&mut self) {
        let max_total = self.sy as usize + self.hlimit as usize;
        while self.lines.len() > max_total {
            self.lines.remove(0);
        }
    }

    /// Erase from cursor to end of line.
    pub fn erase_line_right(&mut self) {
        let cx = self.cursor_x;
        let cy = self.cursor_y;
        if let Some(line) = self.active_line_mut(cy) {
            for col in cx..line.width() {
                if let Some(cell) = line.cell_mut(col) {
                    cell.clear();
                }
            }
        }
    }

    /// Erase entire line.
    pub fn erase_line(&mut self, row: u16) {
        if let Some(line) = self.active_line_mut(row) {
            line.clear();
        }
    }

    /// Erase from cursor to end of display.
    pub fn erase_display_below(&mut self) {
        self.erase_line_right();
        for row in (self.cursor_y + 1)..self.sy {
            self.erase_line(row);
        }
    }

    /// Erase entire display.
    pub fn erase_display_all(&mut self) {
        for row in 0..self.sy {
            self.erase_line(row);
        }
    }

    /// Set the scroll region (DECSTBM).
    pub fn set_scroll_region(&mut self, top: u16, bottom: u16) {
        let top = top.min(self.sy.saturating_sub(1));
        let bottom = bottom.min(self.sy.saturating_sub(1));
        if top < bottom {
            self.scroll_top = top;
            self.scroll_bottom = bottom;
        }
    }

    /// Get text content of an active screen row.
    pub fn line_text(&self, row: u16, arena: &GraphemeArena) -> String {
        self.active_line(row)
            .map(|l| l.text(arena))
            .unwrap_or_default()
    }

    /// Get all active screen text.
    pub fn screen_text(&self, arena: &GraphemeArena) -> Vec<String> {
        (0..self.sy).map(|r| self.line_text(r, arena)).collect()
    }

    /// Get dirty row indices.
    pub fn dirty_rows(&self) -> Vec<u16> {
        let offset = self.scrollback_lines();
        (0..self.sy)
            .filter(|&r| {
                self.lines
                    .get(offset + r as usize)
                    .map(Line::is_dirty)
                    .unwrap_or(false)
            })
            .collect()
    }

    /// Clear all dirty flags.
    pub fn clear_all_dirty(&mut self) {
        for line in &mut self.lines {
            line.clear_dirty();
        }
    }

    /// Create a snapshot of the active screen for rendering (Arc clones only).
    pub fn snapshot(&self) -> Vec<Line> {
        let offset = self.scrollback_lines();
        self.lines[offset..].to_vec()
    }

    /// Resize the grid.
    pub fn resize(&mut self, new_sx: u16, new_sy: u16) {
        // Resize existing lines to new width
        for line in &mut self.lines {
            if line.width() != new_sx {
                line.resize(new_sx);
            }
        }
        self.sx = new_sx;

        // Adjust number of active lines
        let current_active = self.lines.len().min(self.sy as usize);
        if (new_sy as usize) > current_active {
            // Add blank lines
            for _ in current_active..(new_sy as usize) {
                self.lines.push(Line::new(new_sx));
            }
        }

        self.sy = new_sy;
        self.scroll_top = 0;
        self.scroll_bottom = new_sy.saturating_sub(1);
        self.cursor_x = self.cursor_x.min(new_sx.saturating_sub(1));
        self.cursor_y = self.cursor_y.min(new_sy.saturating_sub(1));
    }

    /// Get a cell from the active screen.
    pub fn cell(&self, col: u16, row: u16) -> Option<&Cell> {
        self.active_line(row).and_then(|l| l.cell(col))
    }

    /// Get the size as a Size struct.
    pub fn size(&self) -> Size {
        Size::new(self.sx, self.sy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_arena() -> GraphemeArena {
        GraphemeArena::new()
    }

    #[test]
    fn grid_creation() {
        let g = ChunkedGrid::new(80, 24, 2000);
        assert_eq!(g.sx(), 80);
        assert_eq!(g.sy(), 24);
        assert_eq!(g.total_lines(), 24);
    }

    #[test]
    fn write_char_and_read() {
        let mut arena = make_arena();
        let mut g = ChunkedGrid::new(80, 24, 100);
        let id = arena.intern("A");
        g.write_char(id, 1);
        assert_eq!(g.cursor(), (1, 0));
        assert_eq!(g.line_text(0, &arena), "A");
    }

    #[test]
    fn write_multiple_chars() {
        let mut arena = make_arena();
        let mut g = ChunkedGrid::new(80, 24, 100);
        for ch in "Hello".chars() {
            let id = arena.intern(&ch.to_string());
            g.write_char(id, 1);
        }
        assert_eq!(g.line_text(0, &arena), "Hello");
    }

    #[test]
    fn line_feed_moves_cursor() {
        let mut g = ChunkedGrid::new(80, 24, 100);
        g.line_feed();
        assert_eq!(g.cursor(), (0, 1));
    }

    #[test]
    fn carriage_return() {
        let mut arena = make_arena();
        let mut g = ChunkedGrid::new(80, 24, 100);
        let id = arena.intern("X");
        g.write_char(id, 1);
        g.carriage_return();
        assert_eq!(g.cursor(), (0, 0));
    }

    #[test]
    fn cow_snapshot_isolation() {
        let mut arena = make_arena();
        let mut g = ChunkedGrid::new(80, 24, 100);
        let id = arena.intern("B");
        g.write_char(id, 1);

        let snap = g.snapshot();
        // Modify original after snapshot
        let id2 = arena.intern("C");
        g.set_cursor(0, 0);
        g.write_char(id2, 1);

        // Snapshot should still show original
        assert_eq!(snap[0].text(&arena), "B");
        assert_eq!(g.line_text(0, &arena), "C");
    }

    #[test]
    fn cow_ref_count_increases_on_snapshot() {
        let g = ChunkedGrid::new(80, 24, 100);
        let snap = g.snapshot();
        // Active line and snapshot share the Arc
        assert!(snap[0].ref_count() >= 2);
    }

    #[test]
    fn dirty_tracking() {
        let mut g = ChunkedGrid::new(80, 24, 100);
        g.clear_all_dirty();
        assert!(g.dirty_rows().is_empty());

        let mut arena = make_arena();
        let id = arena.intern("X");
        g.write_char(id, 1);
        assert!(!g.dirty_rows().is_empty());
    }

    #[test]
    fn erase_line_right() {
        let mut arena = make_arena();
        let mut g = ChunkedGrid::new(80, 24, 100);
        for ch in "ABCDE".chars() {
            let id = arena.intern(&ch.to_string());
            g.write_char(id, 1);
        }
        g.set_cursor(2, 0);
        g.erase_line_right();
        assert_eq!(g.line_text(0, &arena), "AB");
    }

    #[test]
    fn erase_display_all() {
        let mut arena = make_arena();
        let mut g = ChunkedGrid::new(80, 24, 100);
        let id = arena.intern("X");
        g.write_char(id, 1);
        g.erase_display_all();
        assert_eq!(g.line_text(0, &arena), "");
    }

    #[test]
    fn scroll_region() {
        let mut g = ChunkedGrid::new(80, 24, 100);
        g.set_scroll_region(5, 10);
        // Scroll region should be set
        assert_eq!(g.scroll_top, 5);
        assert_eq!(g.scroll_bottom, 10);
    }

    #[test]
    fn resize_grid() {
        let mut g = ChunkedGrid::new(80, 24, 100);
        g.resize(120, 40);
        assert_eq!(g.sx(), 120);
        assert_eq!(g.sy(), 40);
    }

    #[test]
    fn resize_clamps_cursor() {
        let mut g = ChunkedGrid::new(80, 24, 100);
        g.set_cursor(79, 23);
        g.resize(40, 10);
        assert_eq!(g.cursor(), (39, 9));
    }

    #[test]
    fn wide_char_padding() {
        let mut arena = make_arena();
        let mut g = ChunkedGrid::new(80, 24, 100);
        let id = arena.intern("\u{4e16}"); // CJK
        g.write_char(id, 2);
        // Cursor should advance by 2
        assert_eq!(g.cursor(), (2, 0));
        // Second cell should be padding
        let cell = g.cell(1, 0);
        assert!(cell.is_some());
        assert!(cell.map(Cell::is_padding).unwrap_or(false));
    }

    #[test]
    fn scrollback_lines_count() {
        let mut g = ChunkedGrid::new(80, 5, 100);
        // Fill screen and trigger scrolls
        for _ in 0..10 {
            g.line_feed();
        }
        assert!(g.scrollback_lines() > 0);
    }

    #[test]
    fn screen_text() {
        let mut arena = make_arena();
        let mut g = ChunkedGrid::new(80, 3, 100);
        for (row, text) in ["abc", "def", "ghi"].iter().enumerate() {
            g.set_cursor(0, row as u16);
            for ch in text.chars() {
                let id = arena.intern(&ch.to_string());
                g.write_char(id, 1);
            }
        }
        let texts = g.screen_text(&arena);
        assert_eq!(texts[0], "abc");
        assert_eq!(texts[1], "def");
        assert_eq!(texts[2], "ghi");
    }

    #[test]
    fn erase_line() {
        let mut arena = make_arena();
        let mut g = ChunkedGrid::new(80, 24, 100);
        let id = arena.intern("X");
        g.write_char(id, 1);
        g.erase_line(0);
        assert_eq!(g.line_text(0, &arena), "");
    }

    #[test]
    fn scroll_down() {
        let mut arena = make_arena();
        let mut g = ChunkedGrid::new(80, 5, 100);
        let id = arena.intern("A");
        g.set_cursor(0, 0);
        g.write_char(id, 1);
        g.scroll_down(1);
        // First line should be blank after scroll down
        assert_eq!(g.line_text(0, &arena), "");
    }

    #[test]
    fn attrs_applied_to_written_cells() {
        let mut arena = make_arena();
        let mut g = ChunkedGrid::new(80, 24, 100);
        g.set_attrs(Attrs::BOLD, Colour::Indexed(1), Colour::Default);
        let id = arena.intern("X");
        g.write_char(id, 1);
        let cell = g.cell(0, 0);
        assert!(cell.is_some());
        let default_cell = Cell::default();
        let cell = cell.unwrap_or(&default_cell);
        assert!(cell.attrs.contains(Attrs::BOLD));
        assert_eq!(cell.fg, Colour::Indexed(1));
    }

    #[test]
    fn reset_attrs() {
        let mut g = ChunkedGrid::new(80, 24, 100);
        g.set_attrs(Attrs::BOLD, Colour::Indexed(1), Colour::Indexed(2));
        g.reset_attrs();
        assert_eq!(g.current_attrs, Attrs::empty());
        assert_eq!(g.current_fg, Colour::Default);
    }

    #[test]
    fn line_new_width() {
        let line = Line::new(80);
        assert_eq!(line.width(), 80);
    }

    #[test]
    fn line_resize() {
        let mut line = Line::new(80);
        line.resize(120);
        assert_eq!(line.width(), 120);
    }

    #[test]
    fn grid_size() {
        let g = ChunkedGrid::new(80, 24, 100);
        assert_eq!(g.size(), Size::new(80, 24));
    }

    #[test]
    fn hlimit() {
        let g = ChunkedGrid::new(80, 24, 5000);
        assert_eq!(g.hlimit(), 5000);
    }

    #[test]
    fn auto_wrap() {
        let mut arena = make_arena();
        let mut g = ChunkedGrid::new(5, 3, 100);
        for ch in "ABCDEF".chars() {
            let id = arena.intern(&ch.to_string());
            g.write_char(id, 1);
        }
        // F should be on second line
        assert_eq!(g.line_text(0, &arena), "ABCDE");
        assert_eq!(g.line_text(1, &arena), "F");
    }

    #[test]
    fn erase_display_below() {
        let mut arena = make_arena();
        let mut g = ChunkedGrid::new(80, 3, 100);
        for row in 0..3u16 {
            g.set_cursor(0, row);
            let id = arena.intern(&row.to_string());
            g.write_char(id, 1);
        }
        g.set_cursor(0, 1);
        g.erase_display_below();
        assert_eq!(g.line_text(0, &arena), "0");
        assert_eq!(g.line_text(1, &arena), "");
        assert_eq!(g.line_text(2, &arena), "");
    }

    mod prop {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn cow_isolation(writes in proptest::collection::vec(b'A'..=b'Z', 1..20)) {
                let mut arena = GraphemeArena::new();
                let mut g = ChunkedGrid::new(80, 24, 100);
                for &b in &writes {
                    let id = arena.intern(&(b as char).to_string());
                    g.write_char(id, 1);
                }
                let snap = g.snapshot();
                let snap_text = snap[0].text(&arena);

                // Mutate original
                g.set_cursor(0, 0);
                g.erase_line(0);

                // Snapshot preserved
                prop_assert_eq!(snap[0].text(&arena), snap_text);
            }

            #[test]
            fn resize_preserves_cursor_bounds(
                sx in 5u16..200, sy in 3u16..100,
                new_sx in 5u16..200, new_sy in 3u16..100,
            ) {
                let mut g = ChunkedGrid::new(sx, sy, 100);
                g.set_cursor(sx.saturating_sub(1), sy.saturating_sub(1));
                g.resize(new_sx, new_sy);
                let (cx, cy) = g.cursor();
                prop_assert!(cx < new_sx);
                prop_assert!(cy < new_sy);
            }

            #[test]
            fn scrollback_bounded(scroll_count in 1u16..50) {
                let hlimit = 10u32;
                let mut g = ChunkedGrid::new(80, 5, hlimit);
                for _ in 0..scroll_count {
                    g.line_feed();
                }
                prop_assert!(g.scrollback_lines() <= hlimit as usize);
            }

            #[test]
            fn dirty_cleared_resets(row_count in 1u16..20) {
                let sy = row_count.max(3);
                let mut g = ChunkedGrid::new(80, sy, 100);
                g.clear_all_dirty();
                prop_assert!(g.dirty_rows().is_empty());
            }
        }
    }
}
