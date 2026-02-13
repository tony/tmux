use mux_types::{Cell, Style};
use std::sync::Arc;

const CHUNK_SIZE: usize = 256;

#[derive(Debug, Clone)]
pub struct LineChunk {
    pub lines: Vec<Vec<Cell>>,
}

#[derive(Debug, Clone)]
pub struct Grid {
    pub width: usize,
    pub height: usize,
    pub chunks: Vec<Arc<LineChunk>>,
    pub cursor_x: usize,
    pub cursor_y: usize,
    pub scroll_top: usize,
    pub scroll_bottom: usize,
}

impl Grid {
    pub fn new(width: usize, height: usize) -> Self {
        todo!()
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        todo!()
    }

    pub fn set_cell(&mut self, x: usize, y: usize, cell: Cell) {
        todo!()
    }

    pub fn get_cell(&self, x: usize, y: usize) -> &Cell {
        todo!()
    }

    pub fn clear(&mut self, style: Style) {
        todo!()
    }
}
--- END FILE ---
