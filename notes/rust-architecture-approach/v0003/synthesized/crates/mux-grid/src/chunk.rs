//! Chunk-based storage -- groups of 64 lines for cache-friendly access.

use crate::line::Line;

/// Lines per chunk. Chosen for L1 cache line alignment.
pub const CHUNK_SIZE: usize = 64;

/// A chunk of contiguous lines.
#[derive(Debug, Clone)]
pub struct Chunk {
    lines: Vec<Line>,
}

impl Chunk {
    /// Create a new chunk with `count` lines of the given width.
    #[must_use]
    pub fn new(count: usize, width: u16) -> Self {
        Self {
            lines: (0..count).map(|_| Line::new(width)).collect(),
        }
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

    /// Get a line by index.
    #[must_use]
    pub fn get(&self, idx: usize) -> Option<&Line> {
        self.lines.get(idx)
    }

    /// Get a mutable line by index.
    pub fn get_mut(&mut self, idx: usize) -> Option<&mut Line> {
        self.lines.get_mut(idx)
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
    fn new_chunk() {
        let chunk = Chunk::new(CHUNK_SIZE, 80);
        assert_eq!(chunk.len(), CHUNK_SIZE);
    }

    #[test]
    fn chunk_get() {
        let chunk = Chunk::new(10, 80);
        assert!(chunk.get(0).is_some());
        assert!(chunk.get(9).is_some());
        assert!(chunk.get(10).is_none());
    }

    #[test]
    fn empty_chunk() {
        let chunk = Chunk::new(0, 80);
        assert!(chunk.is_empty());
    }
}
