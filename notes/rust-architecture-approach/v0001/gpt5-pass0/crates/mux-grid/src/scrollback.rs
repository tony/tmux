use mux_types::Line;

/// Scrollback storage with bounded retention. Lines are Arc-backed COW via `Line` (S96).
#[derive(Debug, Clone)]
pub struct Scrollback {
    max_lines: usize,
    lines: Vec<Line>,
}

impl Scrollback {
    pub fn new(max_lines: usize) -> Self {
        Self {
            max_lines,
            lines: Vec::new(),
        }
    }

    pub fn push_line(&mut self, line: Line) {
        self.lines.push(line);
        if self.lines.len() > self.max_lines {
            let drop_n = self.lines.len() - self.max_lines;
            self.lines.drain(0..drop_n);
        }
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn lines(&self) -> &[Line] {
        &self.lines
    }

    pub fn search(&self, needle: &str) -> Vec<usize> {
        self.lines
            .iter()
            .enumerate()
            .filter_map(|(idx, line)| {
                let txt: String = line
                    .as_slice()
                    .iter()
                    .map(|c| c.grapheme.as_str())
                    .collect::<Vec<_>>()
                    .join("");
                txt.contains(needle).then_some(idx)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mux_types::{Cell, Line};

    #[test]
    fn keeps_only_bounded_history() {
        let mut sb = Scrollback::new(2);
        sb.push_line(Line::new(vec![Cell::from_char('a')]));
        sb.push_line(Line::new(vec![Cell::from_char('b')]));
        sb.push_line(Line::new(vec![Cell::from_char('c')]));
        assert_eq!(sb.len(), 2);
        assert_eq!(sb.lines()[0].as_slice()[0].grapheme, "b");
    }

    #[test]
    fn search_finds_matching_lines() {
        let mut sb = Scrollback::new(8);
        sb.push_line(Line::new(vec![Cell::new("hello", 0, 0, mux_types::CellWidth::One)]));
        sb.push_line(Line::new(vec![Cell::new("world", 0, 0, mux_types::CellWidth::One)]));
        assert_eq!(sb.search("wor"), vec![1]);
    }

    #[test]
    fn empty_scrollback() {
        let sb = Scrollback::new(1);
        assert!(sb.is_empty());
        assert!(sb.search("x").is_empty());
    }
}
