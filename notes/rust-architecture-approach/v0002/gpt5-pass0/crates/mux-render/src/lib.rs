use mux_grid::Grid;
use mux_types::{Cell, Point};

#[derive(Debug, Clone)]
pub struct CellDiff {
    pub at: Point,
    pub cell: Cell,
}

#[derive(Debug, Clone, Default)]
pub struct RenderBatch {
    pub revision: u64,
    pub diffs: Vec<CellDiff>,
}

pub trait Renderer: Send + Sync {
    fn render(&self, previous: Option<&Grid>, next: &Grid) -> RenderBatch;
}

#[derive(Debug, Default)]
pub struct FullFrameRenderer;

impl Renderer for FullFrameRenderer {
    fn render(&self, _previous: Option<&Grid>, next: &Grid) -> RenderBatch {
        let size = next.size();
        let mut diffs = Vec::new();
        for y in 0..size.rows {
            if let Some(line) = next.line_cells(y) {
                for (x, cell) in line.into_iter().enumerate() {
                    diffs.push(CellDiff {
                        at: Point {
                            x: x as u16,
                            y,
                        },
                        cell,
                    });
                }
            }
        }

        RenderBatch {
            revision: next.revision(),
            diffs,
        }
    }
}
