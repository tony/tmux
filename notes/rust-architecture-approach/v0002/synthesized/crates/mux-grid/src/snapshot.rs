//! Immutable grid snapshot for the render pipeline.

use crate::chunk::Chunk;
use mux_types::Size;

/// An immutable snapshot of a grid taken for the render pipeline.
///
/// Because lines use Arc-based COW, creating a snapshot is cheap
/// (just cloning Arc pointers and the dirty bitmap).
#[derive(Debug, Clone)]
pub struct GridSnapshot {
    /// The chunks (cloned from the grid's chunk list).
    pub chunks: Vec<Chunk>,
    /// Dirty flags per active-screen row.
    pub dirty: Vec<bool>,
    /// Grid dimensions at snapshot time.
    pub size: Size,
    /// Revision counter at snapshot time.
    pub revision: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::ChunkedGrid;

    #[test]
    fn snapshot_captures_current_state() {
        let g = ChunkedGrid::new(80, 24, 1000);
        let snap = g.snapshot(0);
        assert_eq!(snap.size, Size::new(80, 24));
        assert_eq!(snap.revision, 0);
    }
}
