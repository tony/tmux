pub mod row;
pub mod scrollback;

use crate::row::Row;
use mux_types::{Cell, Point, Size};

pub struct Grid {
    pub size: Size,
    pub rows: Vec<Row>,
    pub cursor: Point,
    pub scrollback: scrollback::ScrollbackBuffer,
}

impl Grid {
    pub fn new(width: u16, height: u16) -> Self {
        let mut rows = Vec::with_capacity(height as usize);
        for _ in 0..height {
            rows.push(Row::new(width));
        }
        Self {
            size: Size { width, height },
            rows,
            cursor: Point::default(),
            scrollback: scrollback::ScrollbackBuffer::new(1000),
        }
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        // Reflow logic would go here
        self.size = Size { width, height };
        self.rows.resize_with(height as usize, || Row::new(width));
        for row in &mut self.rows {
            row.resize(width);
        }
    }

    pub fn set_cell(&mut self, x: u16, y: u16, cell: Cell) {
        if y >= self.size.height || x >= self.size.width {
            return;
        }
        self.rows[y as usize].cells[x as usize] = cell;
    }

    pub fn get_cell(&self, x: u16, y: u16) -> Option<&Cell> {
        self.rows.get(y as usize)?.cells.get(x as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_creation() {
        let g = Grid::new(10, 5);
        assert_eq!(g.size.width, 10);
        assert_eq!(g.size.height, 5);
        assert_eq!(g.rows.len(), 5);
        assert_eq!(g.rows[0].cells.len(), 10);
    }

    #[test]
    fn test_set_get_cell() {
        let mut g = Grid::new(10, 5);
        let mut c = Cell::default();
        c.char = 'X';
        g.set_cell(2, 2, c);
        assert_eq!(g.get_cell(2, 2).unwrap().char, 'X');
        assert_eq!(g.get_cell(0, 0).unwrap().char, ' ');
    }

    #[test]
    fn test_out_of_bounds() {
        let mut g = Grid::new(5, 5);
        let mut c = Cell::default();
        c.char = 'Y';
        g.set_cell(10, 10, c); // Should not panic
        assert!(g.get_cell(10, 10).is_none());
    }

    #[test]
    fn test_resize() {
        let mut g = Grid::new(5, 5);
        g.resize(10, 10);
        assert_eq!(g.size.width, 10);
        assert_eq!(g.size.height, 10);
        assert_eq!(g.rows.len(), 10);
        assert_eq!(g.rows[0].cells.len(), 10);
    }

    #[test]
    fn test_resize_shrink() {
        let mut g = Grid::new(10, 10);
        g.resize(5, 5);
        assert_eq!(g.rows.len(), 5);
        assert_eq!(g.rows[0].cells.len(), 5);
    }

    #[test]
    fn test_cursor_default() {
        let g = Grid::new(10, 10);
        assert_eq!(g.cursor.x, 0);
        assert_eq!(g.cursor.y, 0);
    }
}
