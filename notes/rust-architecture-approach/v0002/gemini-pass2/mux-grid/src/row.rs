use mux_types::Cell;

#[derive(Clone, Debug)]
pub struct Row {
    pub cells: Vec<Cell>,
    pub dirty: bool,
}

impl Row {
    pub fn new(width: u16) -> Self {
        Self {
            cells: vec![Cell::default(); width as usize],
            dirty: true,
        }
    }

    pub fn resize(&mut self, width: u16) {
        self.cells.resize(width as usize, Cell::default());
        self.dirty = true;
    }
}
