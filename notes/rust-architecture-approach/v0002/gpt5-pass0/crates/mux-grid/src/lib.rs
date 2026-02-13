use std::sync::Arc;

use bitvec::vec::BitVec;
use mux_types::{Cell, Point, Size};
use thiserror::Error;

pub const CHUNK_SIZE: usize = 256;

#[derive(Debug, Clone)]
pub struct LineChunk {
    pub cells: Vec<Cell>,
}

#[derive(Debug, Error)]
pub enum GridError {
    #[error("point out of bounds: ({x},{y}) for {cols}x{rows}")]
    OutOfBounds {
        x: u16,
        y: u16,
        cols: u16,
        rows: u16,
    },
}

#[derive(Debug, Clone)]
pub struct Grid {
    size: Size,
    lines: Vec<Arc<LineChunk>>,
    line_dirty: BitVec,
    revision: u64,
}

impl Grid {
    #[must_use]
    pub fn new(size: Size) -> Self {
        let cols = usize::from(size.cols);
        let rows = usize::from(size.rows);
        let line = Arc::new(LineChunk {
            cells: vec![Cell::default(); cols],
        });
        Self {
            size,
            lines: (0..rows).map(|_| line.clone()).collect(),
            line_dirty: BitVec::repeat(false, rows),
            revision: 0,
        }
    }

    #[must_use]
    pub fn size(&self) -> Size {
        self.size
    }

    #[must_use]
    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn put_cell(&mut self, at: Point, cell: Cell) -> Result<(), GridError> {
        self.validate(at)?;
        let y = usize::from(at.y);
        let x = usize::from(at.x);
        let line = Arc::make_mut(&mut self.lines[y]);
        line.cells[x] = cell;
        self.line_dirty.set(y, true);
        self.revision = self.revision.saturating_add(1);
        Ok(())
    }

    pub fn line_cells(&self, y: u16) -> Option<Vec<Cell>> {
        self.lines.get(usize::from(y)).map(|line| line.cells.clone())
    }

    #[must_use]
    pub fn is_line_dirty(&self, y: u16) -> bool {
        self.line_dirty
            .get(usize::from(y))
            .map(|bit| *bit)
            .unwrap_or(false)
    }

    pub fn clear_dirty(&mut self) {
        self.line_dirty.fill(false);
    }

    fn validate(&self, at: Point) -> Result<(), GridError> {
        if at.x < self.size.cols && at.y < self.size.rows {
            Ok(())
        } else {
            Err(GridError::OutOfBounds {
                x: at.x,
                y: at.y,
                cols: self.size.cols,
                rows: self.size.rows,
            })
        }
    }
}
