//! # mux-snapshot
//!
//! Grid snapshot serialization, text extraction, and comparison.

#![forbid(unsafe_code)]

use mux_grapheme_arena::GraphemeArena;
use mux_grid::GridSnapshot;
use mux_types::Size;

/// A captured terminal snapshot in text form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextSnapshot {
    /// Lines of text, one per row.
    pub lines: Vec<String>,
    /// Grid dimensions.
    pub size: Size,
    /// Revision number.
    pub revision: u64,
}

impl TextSnapshot {
    /// Convert the snapshot to a single string.
    #[must_use]
    pub fn to_text(&self) -> String {
        self.lines.join("\n")
    }
}

impl std::fmt::Display for TextSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, line) in self.lines.iter().enumerate() {
            if i > 0 { writeln!(f)?; }
            write!(f, "{line}")?;
        }
        Ok(())
    }
}

/// Extract text content from a grid snapshot.
#[must_use]
pub fn extract_text(snapshot: &GridSnapshot, arena: &GraphemeArena) -> TextSnapshot {
    let mut lines = Vec::new();
    for chunk in &snapshot.chunks {
        for line in &chunk.lines {
            let mut text = String::with_capacity(snapshot.size.cols as usize);
            for cell in line.cells() {
                let mut buf = String::new();
                arena.resolve_to_buf(cell.grapheme, &mut buf);
                if buf.is_empty() && !cell.is_padding() {
                    text.push(' ');
                } else {
                    text.push_str(&buf);
                }
            }
            lines.push(text.trim_end().to_owned());
        }
    }
    TextSnapshot { lines, size: snapshot.size, revision: snapshot.revision }
}

/// Compute diff between two text snapshots.
#[must_use]
pub fn diff_snapshots(old: &TextSnapshot, new: &TextSnapshot) -> Vec<(usize, String, String)> {
    let max_lines = old.lines.len().max(new.lines.len());
    let mut diffs = Vec::new();
    for i in 0..max_lines {
        let old_line = old.lines.get(i).map_or("", String::as_str);
        let new_line = new.lines.get(i).map_or("", String::as_str);
        if old_line != new_line {
            diffs.push((i, old_line.to_owned(), new_line.to_owned()));
        }
    }
    diffs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_snapshot_display() {
        let snap = TextSnapshot {
            lines: vec!["hello".into(), "world".into()],
            size: Size::new(80, 24),
            revision: 1,
        };
        assert_eq!(snap.to_text(), "hello\nworld");
    }

    #[test]
    fn diff_identical() {
        let snap = TextSnapshot { lines: vec!["hello".into()], size: Size::new(80, 24), revision: 1 };
        assert!(diff_snapshots(&snap, &snap).is_empty());
    }

    #[test]
    fn diff_different() {
        let old = TextSnapshot { lines: vec!["hello".into()], size: Size::new(80, 24), revision: 1 };
        let new = TextSnapshot { lines: vec!["world".into()], size: Size::new(80, 24), revision: 2 };
        let diffs = diff_snapshots(&old, &new);
        assert_eq!(diffs.len(), 1);
    }
}
