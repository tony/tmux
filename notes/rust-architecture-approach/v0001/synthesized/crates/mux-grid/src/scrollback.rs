//! Scrollback storage with bounded retention.
//!
//! Lines are Arc-backed COW via `Line` (S96, INV-032).
//! Separated from grid.rs per GPT decomposition pattern.

use std::collections::VecDeque;
use mux_types::Line;

/// Scrollback buffer with bounded history.
///
/// S96: Lines use Arc<Vec<Cell>> for zero-copy sharing.
/// INV-032: Scrollback lines use Arc.
#[derive(Debug, Clone)]
pub struct Scrollback {
    lines: VecDeque<Line>,
    limit: usize,
}

impl Scrollback {
    /// Create a new scrollback buffer with the given maximum line count.
    #[must_use]
    pub fn new(limit: usize) -> Self {
        Self {
            lines: VecDeque::new(),
            limit,
        }
    }

    /// Push a line into scrollback.
    /// If over limit, the oldest line is discarded.
    pub fn push(&mut self, line: Line) {
        self.lines.push_back(line);
        if self.lines.len() > self.limit {
            self.lines.pop_front();
        }
    }

    /// Pop the most recent line from scrollback (for scroll_down).
    pub fn pop(&mut self) -> Option<Line> {
        self.lines.pop_back()
    }

    /// Number of lines in scrollback.
    #[must_use]
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// True if scrollback is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// The maximum number of lines.
    #[must_use]
    pub fn limit(&self) -> usize {
        self.limit
    }

    /// Set a new limit, trimming excess lines.
    pub fn set_limit(&mut self, limit: usize) {
        self.limit = limit;
        while self.lines.len() > self.limit {
            self.lines.pop_front();
        }
    }

    /// Access scrollback lines as a slice (oldest first).
    pub fn lines(&self) -> impl Iterator<Item = &Line> {
        self.lines.iter()
    }

    /// Search scrollback for lines containing `needle`.
    /// Returns indices (0 = oldest) of matching lines.
    #[must_use]
    pub fn search(&self, needle: &str) -> Vec<usize> {
        self.lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line.to_text().contains(needle))
            .map(|(idx, _)| idx)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mux_types::{Cell, Line};
    use mux_types::style::Style;

    fn line_with_text(s: &str) -> Line {
        let cells: Vec<Cell> = s.chars().map(|c| Cell::from_char(c, Style::default())).collect();
        Line::from_cells(cells)
    }

    #[test]
    fn test_scrollback_empty() {
        let sb = Scrollback::new(10);
        assert!(sb.is_empty());
        assert_eq!(sb.len(), 0);
    }

    #[test]
    fn test_scrollback_push_and_len() {
        let mut sb = Scrollback::new(10);
        sb.push(Line::new(80));
        sb.push(Line::new(80));
        assert_eq!(sb.len(), 2);
    }

    #[test]
    fn test_scrollback_limit_enforced() {
        let mut sb = Scrollback::new(2);
        sb.push(line_with_text("a"));
        sb.push(line_with_text("b"));
        sb.push(line_with_text("c"));
        assert_eq!(sb.len(), 2);
        // Oldest "a" was dropped, "b" and "c" remain
        let lines: Vec<_> = sb.lines().collect();
        assert_eq!(lines[0].to_text(), "b");
        assert_eq!(lines[1].to_text(), "c");
    }

    #[test]
    fn test_scrollback_pop() {
        let mut sb = Scrollback::new(10);
        sb.push(line_with_text("x"));
        sb.push(line_with_text("y"));
        let popped = sb.pop();
        assert!(popped.is_some());
        assert_eq!(popped.unwrap().to_text(), "y");
        assert_eq!(sb.len(), 1);
    }

    #[test]
    fn test_scrollback_set_limit() {
        let mut sb = Scrollback::new(10);
        for i in 0..10 {
            sb.push(line_with_text(&format!("{i}")));
        }
        assert_eq!(sb.len(), 10);
        sb.set_limit(3);
        assert_eq!(sb.len(), 3);
        assert_eq!(sb.limit(), 3);
    }

    #[test]
    fn test_scrollback_search() {
        let mut sb = Scrollback::new(10);
        sb.push(line_with_text("hello world"));
        sb.push(line_with_text("foo bar"));
        sb.push(line_with_text("hello again"));
        let matches = sb.search("hello");
        assert_eq!(matches, vec![0, 2]);
    }

    #[test]
    fn test_scrollback_search_no_match() {
        let mut sb = Scrollback::new(10);
        sb.push(line_with_text("hello"));
        let matches = sb.search("xyz");
        assert!(matches.is_empty());
    }

    #[test]
    fn test_scrollback_search_empty_needle_matches_all() {
        let mut sb = Scrollback::new(10);
        sb.push(line_with_text("alpha"));
        sb.push(line_with_text("beta"));
        sb.push(line_with_text("gamma"));
        assert_eq!(sb.search(""), vec![0, 1, 2]);
    }

    #[test]
    fn test_scrollback_search_case_sensitive() {
        let mut sb = Scrollback::new(10);
        sb.push(line_with_text("Hello"));
        sb.push(line_with_text("hello"));
        assert_eq!(sb.search("Hello"), vec![0]);
        assert_eq!(sb.search("hello"), vec![1]);
    }

    #[test]
    fn test_scrollback_search_after_limit_trim() {
        let mut sb = Scrollback::new(2);
        sb.push(line_with_text("old"));
        sb.push(line_with_text("middle"));
        sb.push(line_with_text("new"));
        assert!(sb.search("old").is_empty());
        assert_eq!(sb.search("middle"), vec![0]);
        assert_eq!(sb.search("new"), vec![1]);
    }

    #[test]
    fn test_scrollback_search_order_is_oldest_first() {
        let mut sb = Scrollback::new(10);
        sb.push(line_with_text("needle-a"));
        sb.push(line_with_text("needle-b"));
        sb.push(line_with_text("needle-c"));
        assert_eq!(sb.search("needle"), vec![0, 1, 2]);
    }
}
