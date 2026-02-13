use mux_grid::Grid;
use mux_types::{Point, Cell};
use mux_proto::RenderOp;

pub struct CompositeGrid {
    pub buffer: Grid,
    pub width: u16,
    pub height: u16,
}

impl CompositeGrid {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            buffer: Grid::new(width, height),
            width,
            height,
        }
    }

    pub fn diff(&self, _other: &CompositeGrid) -> Vec<RenderOp> {
        let ops = Vec::new();
        // Compare self (previous) with other (next)
        // Implementation of double-buffer diffing
        // 1. Iterate cells
        // 2. If different, emit RenderOp
        // 3. Optimize (elision, etc)
        ops
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_composite_grid_new() {
        let cg = CompositeGrid::new(10, 5);
        assert_eq!(cg.width, 10);
        assert_eq!(cg.height, 5);
        assert_eq!(cg.buffer.size.width, 10);
    }

    #[test]
    fn test_diff_empty() {
        let cg1 = CompositeGrid::new(10, 10);
        let cg2 = CompositeGrid::new(10, 10);
        let ops = cg1.diff(&cg2);
        assert!(ops.is_empty());
    }

    #[test]
    fn test_diff_dimensions() {
        let cg = CompositeGrid::new(100, 100);
        assert_eq!(cg.buffer.rows.len(), 100);
    }
}
