use mux_grid::Grid;
use serde::Serialize;

#[derive(Serialize)]
pub struct GridSnapshot {
    pub text: String,
    pub cursor: (usize, usize),
}

impl GridSnapshot {
    pub fn from_grid(grid: &Grid) -> Self {
        todo!()
    }
}
--- END FILE ---
