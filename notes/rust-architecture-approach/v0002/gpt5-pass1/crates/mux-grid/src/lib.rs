//! # mux-grid
//!
//! Terminal grid with VecDeque-backed lines, Arc COW scrollback,
//! and viewport management.
//!
//! ## Module Decomposition (GPT pattern)
//! - [`scrollback`]: Bounded scrollback buffer with search.
//! - Root: Grid struct with put_char/put_grapheme mutation.
//!
//! ## Key Design
//! - S96: Live grid uses `VecDeque<Line>`, scrollback shares via `Arc`.
//! - INV-012: Mutation only via `put_char()` / `put_grapheme()`.
//! - INV-007: Coordinates are zero-based (row, col).
//! - INV-024: SmallVec rejected FINAL.

#![forbid(unsafe_code)]

pub mod scrollback;

// GPT-style pub use re-exports
pub use scrollback::Scrollback;

use std::collections::VecDeque;
use compact_str::CompactString;
use mux_types::cell::Cell;
use mux_types::line::Line;
use mux_types::style::Style;

/// Maximum scrollback lines (configurable, but capped).
pub const DEFAULT_SCROLLBACK_LIMIT: usize = 10_000;

/// The terminal grid.
///
/// INV-007: All coordinates are zero-based.
/// INV-012: Mutation only through put_char / put_grapheme.
/// S96: Arc COW for scrollback sharing.
#[derive(Debug, Clone)]
pub struct Grid {
    /// Visible lines (the viewport). Length == rows.
    lines: VecDeque<Line>,
    /// Scrollback buffer.
    scrollback: Scrollback,
    /// Number of visible rows.
    rows: usize,
    /// Number of columns per line.
    cols: usize,
    /// Cursor row (zero-based, within viewport).
    cursor_row: usize,
    /// Cursor column (zero-based).
    cursor_col: usize,
    /// Current style for new characters.
    current_style: Style,
    /// Count of COW triggers (for OTEL metrics).
    cow_trigger_count: u64,
}

impl Grid {
    /// Create a new grid with the given dimensions.
    #[must_use]
    pub fn new(rows: usize, cols: usize) -> Self {
        let mut lines = VecDeque::with_capacity(rows);
        for _ in 0..rows {
            lines.push_back(Line::new(cols));
        }
        Self {
            lines,
            scrollback: Scrollback::new(DEFAULT_SCROLLBACK_LIMIT),
            rows,
            cols,
            cursor_row: 0,
            cursor_col: 0,
            current_style: Style::default(),
            cow_trigger_count: 0,
        }
    }

    /// Grid dimensions.
    #[must_use]
    pub fn rows(&self) -> usize { self.rows }
    #[must_use]
    pub fn cols(&self) -> usize { self.cols }

    /// Cursor position (row, col), zero-based.
    #[must_use]
    pub fn cursor(&self) -> (usize, usize) {
        (self.cursor_row, self.cursor_col)
    }

    /// Set the cursor position. Clamped to grid bounds.
    pub fn set_cursor(&mut self, row: usize, col: usize) {
        self.cursor_row = row.min(self.rows.saturating_sub(1));
        self.cursor_col = col.min(self.cols.saturating_sub(1));
    }

    /// Set the current style for new characters.
    pub fn set_style(&mut self, style: Style) {
        self.current_style = style;
    }

    /// Set the scrollback limit.
    pub fn set_scrollback_limit(&mut self, limit: usize) {
        self.scrollback.set_limit(limit);
    }

    /// Get a reference to a visible line.
    #[must_use]
    pub fn line(&self, row: usize) -> Option<&Line> {
        self.lines.get(row)
    }

    /// Get a reference to a cell in the viewport.
    #[must_use]
    pub fn cell(&self, row: usize, col: usize) -> Option<&Cell> {
        self.lines.get(row).and_then(|line| line.get(col))
    }

    /// Access the scrollback buffer.
    #[must_use]
    pub fn scrollback(&self) -> &Scrollback {
        &self.scrollback
    }

    /// Number of scrollback lines.
    #[must_use]
    pub fn scrollback_len(&self) -> usize {
        self.scrollback.len()
    }

    /// COW trigger count (for OTEL metrics).
    #[must_use]
    pub fn cow_trigger_count(&self) -> u64 {
        self.cow_trigger_count
    }

    /// INV-012: Put a single character at the cursor position.
    ///
    /// This is the primary mutation method. The character is placed at
    /// (cursor_row, cursor_col) with the current style, then the cursor
    /// advances. If the cursor reaches the right edge, it wraps.
    pub fn put_char(&mut self, ch: char) {
        if self.cursor_row >= self.rows || self.cursor_col >= self.cols {
            return;
        }

        let cell = Cell::from_char(ch, self.current_style);
        let width = cell.width();

        if let Some(line) = self.lines.get_mut(self.cursor_row) {
            if line.is_shared() {
                self.cow_trigger_count += 1;
            }
            line.set(self.cursor_col, cell);

            // For wide characters, blank the next cell
            if width == 2 && self.cursor_col + 1 < self.cols {
                let spacer = Cell::new(
                    CompactString::const_new(" "),
                    self.current_style,
                    0, // width 0 = spacer for wide char
                );
                line.set(self.cursor_col + 1, spacer);
            }
        }

        // Advance cursor
        self.cursor_col += width as usize;
        if self.cursor_col >= self.cols {
            self.cursor_col = 0;
            self.cursor_row += 1;
            if self.cursor_row >= self.rows {
                self.scroll_up(1);
                self.cursor_row = self.rows - 1;
            }
        }
    }

    /// INV-012: Put a grapheme cluster at the cursor position.
    pub fn put_grapheme(&mut self, grapheme: &str, width: u8) {
        if self.cursor_row >= self.rows || self.cursor_col >= self.cols {
            return;
        }

        let cell = Cell::new(
            CompactString::from(grapheme),
            self.current_style,
            width,
        );

        if let Some(line) = self.lines.get_mut(self.cursor_row) {
            if line.is_shared() {
                self.cow_trigger_count += 1;
            }
            line.set(self.cursor_col, cell);
        }

        self.cursor_col += width as usize;
        if self.cursor_col >= self.cols {
            self.cursor_col = 0;
            self.cursor_row += 1;
            if self.cursor_row >= self.rows {
                self.scroll_up(1);
                self.cursor_row = self.rows - 1;
            }
        }
    }

    /// Scroll the viewport up by `count` lines.
    ///
    /// Lines scrolled off the top go into scrollback (with Arc sharing).
    /// New blank lines appear at the bottom.
    pub fn scroll_up(&mut self, count: usize) {
        for _ in 0..count {
            if let Some(line) = self.lines.pop_front() {
                self.scrollback.push(line);
            }
            self.lines.push_back(Line::new(self.cols));
        }
    }

    /// Scroll the viewport down by `count` lines (pull from scrollback).
    pub fn scroll_down(&mut self, count: usize) {
        for _ in 0..count {
            if let Some(line) = self.scrollback.pop() {
                self.lines.push_front(line);
                self.lines.pop_back();
            }
        }
    }

    /// Clear the entire viewport.
    pub fn clear(&mut self) {
        for line in self.lines.iter_mut() {
            line.clear();
        }
        self.cursor_row = 0;
        self.cursor_col = 0;
    }

    /// Resize the grid to new dimensions.
    pub fn resize(&mut self, new_rows: usize, new_cols: usize) {
        for line in self.lines.iter_mut() {
            line.resize(new_cols);
        }
        while self.lines.len() < new_rows {
            self.lines.push_back(Line::new(new_cols));
        }
        while self.lines.len() > new_rows {
            if let Some(line) = self.lines.pop_front() {
                self.scrollback.push(line);
            }
        }
        self.rows = new_rows;
        self.cols = new_cols;
        self.cursor_row = self.cursor_row.min(new_rows.saturating_sub(1));
        self.cursor_col = self.cursor_col.min(new_cols.saturating_sub(1));
    }

    /// Extract the visible grid content as strings (for testing/snapshot).
    #[must_use]
    pub fn to_text(&self) -> Vec<String> {
        self.lines.iter().map(|line| line.to_text()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_grid_dimensions() {
        let g = Grid::new(24, 80);
        assert_eq!(g.rows(), 24);
        assert_eq!(g.cols(), 80);
        assert_eq!(g.cursor(), (0, 0));
    }

    #[test]
    fn test_put_char_advances_cursor() {
        let mut g = Grid::new(24, 80);
        g.put_char('A');
        assert_eq!(g.cursor(), (0, 1));
        assert_eq!(g.cell(0, 0).map(|c| c.grapheme()), Some("A"));
    }

    #[test]
    fn test_put_multiple_chars() {
        let mut g = Grid::new(24, 80);
        for ch in "Hello".chars() {
            g.put_char(ch);
        }
        assert_eq!(g.cursor(), (0, 5));
        assert_eq!(g.to_text()[0], "Hello");
    }

    #[test]
    fn test_scroll_up_moves_to_scrollback() {
        let mut g = Grid::new(3, 10);
        g.put_char('A');
        g.scroll_up(1);
        assert_eq!(g.scrollback_len(), 1);
        assert!(g.line(0).map(|l| l.is_blank()).unwrap_or(false));
    }

    #[test]
    fn test_scroll_down_retrieves_from_scrollback() {
        let mut g = Grid::new(3, 10);
        g.put_char('A');
        g.scroll_up(1);
        assert_eq!(g.scrollback_len(), 1);
        g.scroll_down(1);
        assert_eq!(g.scrollback_len(), 0);
        // The line with 'A' is back at the top
        assert_eq!(g.cell(0, 0).map(|c| c.grapheme()), Some("A"));
    }

    #[test]
    fn test_resize_grid() {
        let mut g = Grid::new(24, 80);
        g.resize(10, 40);
        assert_eq!(g.rows(), 10);
        assert_eq!(g.cols(), 40);
    }

    #[test]
    fn test_clear_grid() {
        let mut g = Grid::new(5, 10);
        g.put_char('X');
        g.clear();
        assert_eq!(g.cursor(), (0, 0));
        assert!(g.line(0).map(|l| l.is_blank()).unwrap_or(false));
    }

    #[test]
    fn test_cursor_clamped_to_bounds() {
        let mut g = Grid::new(5, 10);
        g.set_cursor(100, 200);
        assert_eq!(g.cursor(), (4, 9));
    }

    #[test]
    fn test_put_grapheme() {
        let mut g = Grid::new(5, 10);
        g.put_grapheme("AB", 1);
        assert_eq!(g.cell(0, 0).map(|c| c.grapheme()), Some("AB"));
    }
}
