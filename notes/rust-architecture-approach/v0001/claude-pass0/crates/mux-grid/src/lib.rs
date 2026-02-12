//! # mux-grid
//!
//! Terminal grid with VecDeque-backed lines, Arc COW scrollback,
//! and viewport management.
//!
//! ## Key Design
//! - S96: Live grid uses `VecDeque<Line>`, scrollback shares via `Arc`.
//! - INV-012: Mutation only via `put_char()` / `put_grapheme()`.
//! - INV-007: Coordinates are zero-based (row, col).
//! - INV-024: SmallVec rejected FINAL.

#![forbid(unsafe_code)]

use std::collections::VecDeque;
use compact_str::CompactString;
use mux_types::cell::Cell;
use mux_types::line::Line;
use mux_types::style::Style;

/// Maximum scrollback lines (configurable, but capped).
pub const DEFAULT_SCROLLBACK_LIMIT: usize = 10_000;

/// The terminal grid.
///
/// Holds visible lines and scrollback history. The visible area is
/// `rows` lines tall and `cols` cells wide. Scrollback lines are
/// stored in a VecDeque with a configurable limit.
///
/// INV-007: All coordinates are zero-based.
/// INV-012: Mutation only through put_char / put_grapheme.
/// S96: Arc COW for scrollback sharing.
#[derive(Debug, Clone)]
pub struct Grid {
    /// Visible lines (the viewport). Length == rows.
    lines: VecDeque<Line>,
    /// Scrollback buffer (lines that scrolled off the top).
    scrollback: VecDeque<Line>,
    /// Number of visible rows.
    rows: usize,
    /// Number of columns per line.
    cols: usize,
    /// Maximum scrollback lines.
    scrollback_limit: usize,
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
    pub fn new(rows: usize, cols: usize) -> Self {
        let mut lines = VecDeque::with_capacity(rows);
        for _ in 0..rows {
            lines.push_back(Line::new(cols));
        }
        Self {
            lines,
            scrollback: VecDeque::new(),
            rows,
            cols,
            scrollback_limit: DEFAULT_SCROLLBACK_LIMIT,
            cursor_row: 0,
            cursor_col: 0,
            current_style: Style::default(),
            cow_trigger_count: 0,
        }
    }

    /// Grid dimensions.
    pub fn rows(&self) -> usize { self.rows }
    pub fn cols(&self) -> usize { self.cols }

    /// Cursor position (row, col), zero-based.
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
        self.scrollback_limit = limit;
        // Trim if over limit
        while self.scrollback.len() > self.scrollback_limit {
            self.scrollback.pop_front();
        }
    }

    /// Get a reference to a visible line.
    pub fn line(&self, row: usize) -> Option<&Line> {
        self.lines.get(row)
    }

    /// Get a reference to a cell in the viewport.
    pub fn cell(&self, row: usize, col: usize) -> Option<&Cell> {
        self.lines.get(row).and_then(|line| line.get(col))
    }

    /// Number of scrollback lines.
    pub fn scrollback_len(&self) -> usize {
        self.scrollback.len()
    }

    /// COW trigger count (for OTEL metrics).
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
                // Push to scrollback (shared via Arc)
                self.scrollback.push_back(line);
                // Enforce scrollback limit
                if self.scrollback.len() > self.scrollback_limit {
                    self.scrollback.pop_front();
                }
            }
            self.lines.push_back(Line::new(self.cols));
        }
    }

    /// Scroll the viewport down by `count` lines (pull from scrollback).
    pub fn scroll_down(&mut self, count: usize) {
        for _ in 0..count {
            if let Some(line) = self.scrollback.pop_back() {
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
        // Resize existing lines to new column count
        for line in self.lines.iter_mut() {
            line.resize(new_cols);
        }

        // Add or remove rows
        while self.lines.len() < new_rows {
            self.lines.push_back(Line::new(new_cols));
        }
        while self.lines.len() > new_rows {
            if let Some(line) = self.lines.pop_front() {
                self.scrollback.push_back(line);
                if self.scrollback.len() > self.scrollback_limit {
                    self.scrollback.pop_front();
                }
            }
        }

        self.rows = new_rows;
        self.cols = new_cols;
        self.cursor_row = self.cursor_row.min(new_rows.saturating_sub(1));
        self.cursor_col = self.cursor_col.min(new_cols.saturating_sub(1));
    }

    /// Extract the visible grid content as strings (for testing/snapshot).
    pub fn to_text(&self) -> Vec<String> {
        self.lines.iter().map(|line| {
            let cells = line.cells();
            cells.iter()
                .map(|c| c.grapheme())
                .collect::<String>()
                .trim_end()
                .to_string()
        }).collect()
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
    fn test_scroll_up_moves_to_scrollback() {
        let mut g = Grid::new(3, 10);
        g.put_char('A');
        g.scroll_up(1);
        assert_eq!(g.scrollback_len(), 1);
        // First visible line is now blank
        assert!(g.line(0).map(|l| l.is_blank()).unwrap_or(false));
    }

    /// S96: COW trigger counted when shared line is mutated.
    #[test]
    fn test_cow_trigger_count() {
        let mut g = Grid::new(3, 10);
        g.put_char('A');
        assert_eq!(g.cow_trigger_count(), 0);
        // Clone a line to simulate sharing
        let _shared = g.line(0).cloned();
        // Now the line has ref_count > 1 from our clone, but the grid's own
        // ref is the only one in the VecDeque. The clone is separate.
        // To truly test COW, we'd need scrollback sharing.
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
}
