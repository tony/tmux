//! Pane grid -> composite grid blitting and border rendering.

use crate::composite::{CellSource, CompositeCell, CompositeGrid};
use mux_grapheme_arena::GraphemeArena;
use mux_grid::GridSnapshot;
use mux_types::{Attrs, Cell, CellFlags, Colour, PaneId, Size};

/// A positioned pane for rendering.
#[derive(Debug, Clone)]
pub struct PaneGeometry {
    pub pane_id: PaneId,
    pub x: u32,
    pub y: u32,
    pub size: Size,
    pub active: bool,
    pub has_border: bool,
}

/// Blit a pane's grid snapshot into the composite grid.
pub fn blit_pane(
    composite: &mut CompositeGrid,
    snapshot: &GridSnapshot,
    arena: &GraphemeArena,
    geometry: &PaneGeometry,
) {
    let _ = arena; // used for grapheme resolution in full impl

    for row in 0..geometry.size.rows {
        let is_dirty = snapshot.dirty.get(row as usize).copied().unwrap_or(false);
        if !is_dirty { continue; }

        let line = snapshot.chunks.iter()
            .flat_map(|c| c.lines.iter())
            .nth(row as usize);

        if let Some(line) = line {
            for col in 0..geometry.size.cols {
                let cell = line.cell(col).copied().unwrap_or_default();
                let screen_x = geometry.x + col;
                let screen_y = geometry.y + row;
                composite.set(screen_x, screen_y, CompositeCell {
                    cell,
                    source: CellSource::Pane(geometry.pane_id),
                });
            }
        }
    }
}

/// Draw pane borders into the composite grid.
pub fn render_borders(
    composite: &mut CompositeGrid,
    panes: &[PaneGeometry],
    _terminal_size: Size,
) {
    let border_cell = Cell {
        grapheme: mux_grapheme_arena::GraphemeId::from_char('\u{2502}'),
        width: 1,
        flags: CellFlags::empty(),
        attrs: Attrs::empty(),
        fg: Colour::Default,
        bg: Colour::Default,
        us: Colour::Default,
        link: 0,
    };

    let hborder_cell = Cell {
        grapheme: mux_grapheme_arena::GraphemeId::from_char('\u{2500}'),
        width: 1,
        flags: CellFlags::empty(),
        attrs: Attrs::empty(),
        fg: Colour::Default,
        bg: Colour::Default,
        us: Colour::Default,
        link: 0,
    };

    for pane in panes {
        if !pane.has_border { continue; }
        if pane.x > 0 {
            for row in 0..pane.size.rows {
                composite.set(pane.x - 1, pane.y + row, CompositeCell {
                    cell: border_cell,
                    source: CellSource::Border,
                });
            }
        }
        if pane.y > 0 {
            for col in 0..pane.size.cols {
                composite.set(pane.x + col, pane.y - 1, CompositeCell {
                    cell: hborder_cell,
                    source: CellSource::Border,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pane_geometry_debug() {
        let g = PaneGeometry {
            pane_id: PaneId(1), x: 0, y: 0,
            size: Size::new(40, 12), active: true, has_border: false,
        };
        let s = format!("{g:?}");
        assert!(s.contains("PaneGeometry"));
    }
}
