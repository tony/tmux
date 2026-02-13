use mux_grid::Grid;
use mux_types::{Cell, Size};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub size: Size,
    pub revision: u64,
    pub lines: Vec<Vec<Cell>>,
}

impl Snapshot {
    #[must_use]
    pub fn from_grid(grid: &Grid) -> Self {
        let size = grid.size();
        let lines = (0..size.rows)
            .map(|row| grid.line_cells(row).unwrap_or_default())
            .collect();
        Self {
            size,
            revision: grid.revision(),
            lines,
        }
    }

    #[must_use]
    pub fn text(&self) -> String {
        let mut out = String::new();
        for line in &self.lines {
            let text = line.iter().map(|cell| cell.grapheme.as_str()).collect::<String>();
            out.push_str(text.trim_end());
            out.push('\n');
        }
        out
    }
}
