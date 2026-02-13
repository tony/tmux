use mux_grid::Grid;

pub struct Renderer {
    last_grid: Grid,
}

impl Renderer {
    pub fn new(width: usize, height: usize) -> Self {
        todo!()
    }

    pub fn diff_and_render(&mut self, current: &Grid) -> Vec<u8> {
        // Emit VT sequences to update terminal from last_grid to current
        todo!()
    }
}
--- END FILE ---
