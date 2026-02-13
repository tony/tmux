//! 64-line chunks for scrollback management.
//!
//! Each chunk holds up to `CHUNK_SIZE` (64) lines. When lines scroll off
//! the top of the active screen, they are pushed into scrollback chunks.

use crate::line::Line;

/// Number of lines per chunk.
pub const CHUNK_SIZE: usize = 64;

/// A chunk of lines within the grid's scrollback or active area.
#[derive(Debug, Clone)]
pub struct Chunk {
    /// Lines in this chunk (up to CHUNK_SIZE).
    pub lines: Vec<Line>,
}

impl Chunk {
    /// Create a new empty chunk.
    #[must_use]
    pub fn new() -> Self {
        Self {
            lines: Vec::with_capacity(CHUNK_SIZE),
        }
    }

    /// Create a chunk filled with blank lines.
    #[must_use]
    pub fn blank(width: u32, count: usize) -> Self {
        Self {
            lines: (0..count).map(|_| Line::new(width)).collect(),
        }
    }

    /// Push a line into this chunk. Returns false if full.
    pub fn push(&mut self, line: Line) -> bool {
        if self.lines.len() >= CHUNK_SIZE {
            return false;
        }
        self.lines.push(line);
        true
    }

    /// Number of lines in this chunk.
    #[must_use]
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// Whether the chunk is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Whether the chunk is full.
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.lines.len() >= CHUNK_SIZE
    }
}

impl Default for Chunk {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_chunk_empty() {
        let c = Chunk::new();
        assert!(c.is_empty());
        assert!(!c.is_full());
    }

    #[test]
    fn blank_chunk() {
        let c = Chunk::blank(80, 24);
        assert_eq!(c.len(), 24);
    }

    #[test]
    fn push_and_full() {
        let mut c = Chunk::new();
        for _ in 0..CHUNK_SIZE {
            assert!(c.push(Line::new(80)));
        }
        assert!(c.is_full());
        assert!(!c.push(Line::new(80)));
    }
}
