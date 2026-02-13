//! Chunk-based line storage.
//!
//! Lines are grouped in chunks of 64 for cache-friendly access and
//! efficient scrollback trimming (drop entire chunks at once).

use crate::line::Line;

/// Number of lines per chunk. 64 provides a good balance between
/// cache locality and trimming granularity.
pub const CHUNK_SIZE: usize = 64;

/// A chunk of lines in the grid.
#[derive(Debug, Clone)]
pub struct Chunk {
    /// Lines in this chunk. May be fewer than `CHUNK_SIZE` for the last chunk.
    lines: Vec<Line>,
}

impl Chunk {
    /// Create a new chunk with the given lines.
    #[must_use]
    pub fn new(lines: Vec<Line>) -> Self {
        Self { lines }
    }

    /// Create a new empty chunk with `count` lines of the given width.
    #[must_use]
    pub fn with_empty_lines(count: usize, width: u16) -> Self {
        let lines = (0..count).map(|_| Line::new(width)).collect();
        Self { lines }
    }

    /// Number of lines in this chunk.
    #[must_use]
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// Whether this chunk is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Whether this chunk is full (CHUNK_SIZE lines).
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.lines.len() >= CHUNK_SIZE
    }

    /// Get a line by index within this chunk.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&Line> {
        self.lines.get(index)
    }

    /// Get a mutable reference to a line.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Line> {
        self.lines.get_mut(index)
    }

    /// Push a line into this chunk.
    pub fn push(&mut self, line: Line) {
        self.lines.push(line);
    }

    /// Iterate over lines.
    pub fn iter(&self) -> impl Iterator<Item = &Line> {
        self.lines.iter()
    }

    /// Iterate mutably over lines.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Line> {
        self.lines.iter_mut()
    }

    /// Remove and return the first `n` lines.
    pub fn drain_front(&mut self, n: usize) -> Vec<Line> {
        let n = n.min(self.lines.len());
        self.lines.drain(..n).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_size_constant() {
        assert_eq!(CHUNK_SIZE, 64);
    }

    #[test]
    fn empty_chunk() {
        let chunk = Chunk::new(Vec::new());
        assert!(chunk.is_empty());
        assert_eq!(chunk.len(), 0);
    }

    #[test]
    fn chunk_with_lines() {
        let chunk = Chunk::with_empty_lines(10, 80);
        assert_eq!(chunk.len(), 10);
        assert!(!chunk.is_full());
    }

    #[test]
    fn chunk_full() {
        let chunk = Chunk::with_empty_lines(CHUNK_SIZE, 80);
        assert!(chunk.is_full());
    }

    #[test]
    fn chunk_get() {
        let chunk = Chunk::with_empty_lines(5, 80);
        assert!(chunk.get(0).is_some());
        assert!(chunk.get(4).is_some());
        assert!(chunk.get(5).is_none());
    }

    #[test]
    fn chunk_drain_front() {
        let mut chunk = Chunk::with_empty_lines(10, 80);
        let drained = chunk.drain_front(3);
        assert_eq!(drained.len(), 3);
        assert_eq!(chunk.len(), 7);
    }

    #[test]
    fn chunk_drain_more_than_available() {
        let mut chunk = Chunk::with_empty_lines(5, 80);
        let drained = chunk.drain_front(10);
        assert_eq!(drained.len(), 5);
        assert!(chunk.is_empty());
    }
}
