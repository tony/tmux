//! Terminal grid with explicit cursor and bounded scrollback.
//!
//! Key invariants:
//! - INV-007: zero-based `(row, col)` coordinates.
//! - INV-012: mutation occurs via `put_char` and `put_grapheme` entry points.

use std::collections::VecDeque;

use compact_str::CompactString;
use mux_types::cell::Cell;
use mux_types::line::Line;
use mux_types::style::Style;

use crate::scrollback::Scrollback;

/// Default scrollback line cap.
pub const DEFAULT_SCROLLBACK_LIMIT: usize = 10_000;

/// Live terminal grid.
#[derive(Debug, Clone)]
pub struct Grid {
    lines: VecDeque<Line>,
    scrollback: Scrollback,
    rows: usize,
    cols: usize,
    cursor_row: usize,
    cursor_col: usize,
    current_style: Style,
    cow_trigger_count: u64,
}

impl Grid {
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

    #[must_use]
    pub fn rows(&self) -> usize {
        self.rows
    }

    #[must_use]
    pub fn cols(&self) -> usize {
        self.cols
    }

    #[must_use]
    pub fn cursor(&self) -> (usize, usize) {
        (self.cursor_row, self.cursor_col)
    }

    pub fn set_cursor(&mut self, row: usize, col: usize) {
        self.cursor_row = row.min(self.rows.saturating_sub(1));
        self.cursor_col = col.min(self.cols.saturating_sub(1));
    }

    pub fn set_style(&mut self, style: Style) {
        self.current_style = style;
    }

    pub fn set_scrollback_limit(&mut self, limit: usize) {
        self.scrollback.set_limit(limit);
    }

    #[must_use]
    pub fn line(&self, row: usize) -> Option<&Line> {
        self.lines.get(row)
    }

    #[must_use]
    pub fn cell(&self, row: usize, col: usize) -> Option<&Cell> {
        self.lines.get(row).and_then(|line| line.get(col))
    }

    #[must_use]
    pub fn scrollback_len(&self) -> usize {
        self.scrollback.len()
    }

    #[must_use]
    pub fn cow_trigger_count(&self) -> u64 {
        self.cow_trigger_count
    }

    /// INV-012 primary mutation API.
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

            if width == 2 && self.cursor_col + 1 < self.cols {
                let spacer = Cell::new(CompactString::const_new(" "), self.current_style, 0);
                line.set(self.cursor_col + 1, spacer);
            }
        }

        self.advance_cursor(width);
    }

    /// INV-012 primary mutation API.
    pub fn put_grapheme(&mut self, grapheme: &str, width: u8) {
        if self.cursor_row >= self.rows || self.cursor_col >= self.cols {
            return;
        }

        let cell = Cell::new(CompactString::from(grapheme), self.current_style, width);

        if let Some(line) = self.lines.get_mut(self.cursor_row) {
            if line.is_shared() {
                self.cow_trigger_count += 1;
            }
            line.set(self.cursor_col, cell);
        }

        self.advance_cursor(width);
    }

    pub fn scroll_up(&mut self, count: usize) {
        for _ in 0..count {
            if let Some(line) = self.lines.pop_front() {
                self.scrollback.push_line(line);
            }
            self.lines.push_back(Line::new(self.cols));
        }
    }

    pub fn scroll_down(&mut self, count: usize) {
        for _ in 0..count {
            if let Some(line) = self.scrollback.pop_latest() {
                self.lines.push_front(line);
                self.lines.pop_back();
            }
        }
    }

    pub fn clear(&mut self) {
        for line in &mut self.lines {
            line.clear();
        }
        self.cursor_row = 0;
        self.cursor_col = 0;
    }

    pub fn resize(&mut self, new_rows: usize, new_cols: usize) {
        for line in &mut self.lines {
            line.resize(new_cols);
        }

        while self.lines.len() < new_rows {
            self.lines.push_back(Line::new(new_cols));
        }

        while self.lines.len() > new_rows {
            if let Some(line) = self.lines.pop_front() {
                self.scrollback.push_line(line);
            }
        }

        self.rows = new_rows;
        self.cols = new_cols;
        self.cursor_row = self.cursor_row.min(new_rows.saturating_sub(1));
        self.cursor_col = self.cursor_col.min(new_cols.saturating_sub(1));
    }

    #[must_use]
    pub fn to_text(&self) -> Vec<String> {
        self.lines
            .iter()
            .map(|line| {
                line.cells()
                    .iter()
                    .map(|cell| cell.grapheme())
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect()
    }

    fn advance_cursor(&mut self, width: u8) {
        self.cursor_col += width as usize;
        if self.cursor_col >= self.cols {
            self.cursor_col = 0;
            self.cursor_row += 1;
            if self.cursor_row >= self.rows {
                self.scroll_up(1);
                self.cursor_row = self.rows.saturating_sub(1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_grid_has_expected_dimensions() {
        let g = Grid::new(24, 80);
        assert_eq!(g.rows(), 24);
        assert_eq!(g.cols(), 80);
        assert_eq!(g.cursor(), (0, 0));
    }

    #[test]
    fn put_char_advances_cursor() {
        let mut g = Grid::new(24, 80);
        g.put_char('A');
        assert_eq!(g.cursor(), (0, 1));
        assert_eq!(g.cell(0, 0).map(|c| c.grapheme()), Some("A"));
    }

    #[test]
    fn wide_char_uses_width_two_and_places_spacer() {
        let mut g = Grid::new(1, 4);
        g.put_char('\u{4E16}');
        assert_eq!(g.cursor(), (0, 2));
        assert_eq!(g.cell(0, 1).map(|c| c.width()), Some(0));
    }

    #[test]
    fn scroll_up_moves_content_to_scrollback() {
        let mut g = Grid::new(3, 4);
        g.put_char('X');
        g.scroll_up(1);
        assert_eq!(g.scrollback_len(), 1);
        assert!(g.line(0).is_some());
    }

    #[test]
    fn scroll_down_restores_latest_scrollback_line() {
        let mut g = Grid::new(2, 3);
        g.put_char('a');
        g.scroll_up(1);
        assert_eq!(g.scrollback_len(), 1);
        g.scroll_down(1);
        assert_eq!(g.cell(0, 0).map(|c| c.grapheme()), Some("a"));
    }

    #[test]
    fn resize_changes_shape() {
        let mut g = Grid::new(24, 80);
        g.resize(10, 40);
        assert_eq!(g.rows(), 10);
        assert_eq!(g.cols(), 40);
    }

    #[test]
    fn clear_resets_cursor_and_content() {
        let mut g = Grid::new(5, 10);
        g.put_char('X');
        g.clear();
        assert_eq!(g.cursor(), (0, 0));
        assert!(g.line(0).map(|l| l.is_blank()).unwrap_or(false));
    }

    #[test]
    fn cursor_set_is_clamped_to_bounds() {
        let mut g = Grid::new(2, 2);
        g.set_cursor(99, 88);
        assert_eq!(g.cursor(), (1, 1));
    }

    #[test]
    fn to_text_trims_line_end_whitespace() {
        let mut g = Grid::new(1, 4);
        g.put_char('x');
        assert_eq!(g.to_text(), vec!["x".to_string()]);
    }

    #[test]
    fn scrollback_limit_is_configurable() {
        let mut g = Grid::new(2, 2);
        g.set_scrollback_limit(1);
        g.scroll_up(1);
        g.scroll_up(1);
        assert_eq!(g.scrollback_len(), 1);
    }
}
