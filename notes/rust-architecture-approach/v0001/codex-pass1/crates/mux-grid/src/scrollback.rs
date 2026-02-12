use std::collections::VecDeque;

use mux_types::line::Line;

/// Bounded scrollback storage with line-level sharing.
///
/// INV-032: scrollback lines are line objects backed by `Arc<Vec<Cell>>`.
#[derive(Debug, Clone)]
pub struct Scrollback {
    lines: VecDeque<Line>,
    limit: usize,
}

impl Scrollback {
    #[must_use]
    pub fn new(limit: usize) -> Self {
        Self {
            lines: VecDeque::new(),
            limit,
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    #[must_use]
    pub fn limit(&self) -> usize {
        self.limit
    }

    pub fn set_limit(&mut self, limit: usize) {
        self.limit = limit;
        while self.lines.len() > self.limit {
            self.lines.pop_front();
        }
    }

    pub fn push_line(&mut self, line: Line) {
        self.lines.push_back(line);
        while self.lines.len() > self.limit {
            self.lines.pop_front();
        }
    }

    #[must_use]
    pub fn pop_latest(&mut self) -> Option<Line> {
        self.lines.pop_back()
    }

    #[must_use]
    pub fn get(&self, index: usize) -> Option<&Line> {
        self.lines.get(index)
    }

    #[must_use]
    pub fn search(&self, needle: &str) -> Vec<usize> {
        self.lines
            .iter()
            .enumerate()
            .filter_map(|(idx, line)| {
                let text: String = line.cells().iter().map(|cell| cell.grapheme()).collect();
                text.contains(needle).then_some(idx)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mux_types::cell::Cell;
    use mux_types::style::Style;

    #[test]
    fn bounded_history_discards_oldest() {
        let mut sb = Scrollback::new(2);
        let mut a = Line::new(1);
        a.set(0, Cell::from_char('a', Style::default()));
        let mut b = Line::new(1);
        b.set(0, Cell::from_char('b', Style::default()));
        let mut c = Line::new(1);
        c.set(0, Cell::from_char('c', Style::default()));

        sb.push_line(a);
        sb.push_line(b);
        sb.push_line(c);

        assert_eq!(sb.len(), 2);
        assert_eq!(sb.get(0).and_then(|l| l.get(0)).map(|c| c.grapheme()), Some("b"));
    }

    #[test]
    fn pop_latest_returns_newest_line() {
        let mut sb = Scrollback::new(4);
        let mut line = Line::new(1);
        line.set(0, Cell::from_char('x', Style::default()));
        sb.push_line(line);

        let popped = sb.pop_latest().unwrap();
        assert_eq!(popped.get(0).map(|c| c.grapheme()), Some("x"));
        assert!(sb.is_empty());
    }

    #[test]
    fn search_returns_matching_indices() {
        let mut sb = Scrollback::new(8);
        let mut hello = Line::new(5);
        for (idx, ch) in "hello".chars().enumerate() {
            hello.set(idx, Cell::from_char(ch, Style::default()));
        }
        let mut world = Line::new(5);
        for (idx, ch) in "world".chars().enumerate() {
            world.set(idx, Cell::from_char(ch, Style::default()));
        }

        sb.push_line(hello);
        sb.push_line(world);

        assert_eq!(sb.search("wor"), vec![1]);
        assert_eq!(sb.search("absent"), Vec::<usize>::new());
    }

    #[test]
    fn set_limit_trims_existing_lines() {
        let mut sb = Scrollback::new(4);
        sb.push_line(Line::new(1));
        sb.push_line(Line::new(1));
        sb.push_line(Line::new(1));

        sb.set_limit(2);
        assert_eq!(sb.limit(), 2);
        assert_eq!(sb.len(), 2);
    }
}
