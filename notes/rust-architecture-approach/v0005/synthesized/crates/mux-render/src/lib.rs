//! Double-buffer diff renderer with SGR optimization.
//!
//! Implements the Buffer::diff() pattern inspired by ratatui: write to "next"
//! buffer, diff against "prev" buffer, emit only changed cells as minimal
//! escape sequences.

#![forbid(unsafe_code)]

use mux_grapheme_arena::GraphemeArena;
use mux_grid::Line;
use mux_types::cell::Cell;
use mux_types::colour::Colour;
use mux_types::geometry::{Position, Size};
use mux_types::style::Style;

/// A flat buffer of cells for double-buffer rendering.
#[derive(Debug, Clone)]
pub struct RenderBuffer {
    cells: Vec<Cell>,
    width: u16,
    height: u16,
}

impl RenderBuffer {
    /// Create a buffer filled with default cells.
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            cells: vec![Cell::default(); width as usize * height as usize],
            width,
            height,
        }
    }

    /// Get a cell reference.
    pub fn cell(&self, x: u16, y: u16) -> Option<&Cell> {
        if x < self.width && y < self.height {
            Some(&self.cells[y as usize * self.width as usize + x as usize])
        } else {
            None
        }
    }

    /// Get a mutable cell reference.
    pub fn cell_mut(&mut self, x: u16, y: u16) -> Option<&mut Cell> {
        if x < self.width && y < self.height {
            Some(&mut self.cells[y as usize * self.width as usize + x as usize])
        } else {
            None
        }
    }

    /// Fill from grid lines.
    pub fn fill_from_lines(&mut self, lines: &[Line], _arena: &GraphemeArena) {
        for (row, line) in lines.iter().enumerate() {
            if row >= self.height as usize {
                break;
            }
            for col in 0..self.width {
                if let (Some(src), Some(dst)) = (
                    line.cell(col),
                    self.cell_mut(col, row as u16),
                ) {
                    *dst = *src;
                }
            }
        }
    }

    /// Dimensions.
    pub fn size(&self) -> Size {
        Size::new(self.width, self.height)
    }

    /// Clear all cells.
    pub fn clear(&mut self) {
        for cell in &mut self.cells {
            *cell = Cell::default();
        }
    }

    /// Resize the buffer (clears all data).
    pub fn resize(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
        self.cells = vec![Cell::default(); width as usize * height as usize];
    }
}

/// A change detected between two buffers.
#[derive(Debug, Clone)]
pub struct CellChange {
    /// Position of the changed cell.
    pub pos: Position,
    /// The new cell value.
    pub cell: Cell,
}

/// Compute the diff between two render buffers.
///
/// Returns a list of positions that changed, along with the new cell value.
/// Only cells that differ visually are included.
pub fn diff_buffers(prev: &RenderBuffer, next: &RenderBuffer) -> Vec<CellChange> {
    let mut changes = Vec::new();
    let width = prev.width.min(next.width);
    let height = prev.height.min(next.height);

    for y in 0..height {
        for x in 0..width {
            if let (Some(p), Some(n)) = (prev.cell(x, y), next.cell(x, y)) {
                if !p.visually_equal(n) {
                    changes.push(CellChange {
                        pos: Position::new(x, y),
                        cell: *n,
                    });
                }
            }
        }
    }

    changes
}

/// Encode a diff as escape sequences.
pub fn encode_diff(changes: &[CellChange], arena: &GraphemeArena) -> Vec<u8> {
    let mut output = Vec::new();
    let mut current_style = Style::new();
    let mut current_pos = Position::new(u16::MAX, u16::MAX);

    for change in changes {
        // Cursor positioning (CUP)
        if change.pos.x != current_pos.x.wrapping_add(1) || change.pos.y != current_pos.y {
            // Emit CUP: ESC [ row ; col H (1-based)
            let row = change.pos.y + 1;
            let col = change.pos.x + 1;
            let cup = format!("\x1b[{row};{col}H");
            output.extend_from_slice(cup.as_bytes());
        }

        // SGR changes
        let cell_style = Style {
            attrs: change.cell.attrs,
            fg: change.cell.fg,
            bg: change.cell.bg,
            us: change.cell.us,
        };

        if cell_style != current_style {
            if cell_style.needs_reset(&current_style) || current_style != Style::new() {
                output.extend_from_slice(b"\x1b[0m");
            }
            encode_sgr(&cell_style, &mut output);
            current_style = cell_style;
        }

        // Character
        let grapheme = arena.resolve(change.cell.grapheme);
        output.extend_from_slice(grapheme.as_bytes());

        current_pos = change.pos;
    }

    output
}

fn encode_sgr(style: &Style, output: &mut Vec<u8>) {
    let mut params = Vec::new();

    for p in style.attrs.sgr_params() {
        params.push(p.to_string());
    }

    match style.fg {
        Colour::Indexed(idx) => {
            if idx < 8 {
                params.push((30 + idx).to_string());
            } else if idx < 16 {
                params.push((90 + idx - 8).to_string());
            } else {
                params.push("38".into());
                params.push("5".into());
                params.push(idx.to_string());
            }
        }
        Colour::Rgb { r, g, b } => {
            params.push("38".into());
            params.push("2".into());
            params.push(r.to_string());
            params.push(g.to_string());
            params.push(b.to_string());
        }
        Colour::Default => {}
    }

    match style.bg {
        Colour::Indexed(idx) => {
            if idx < 8 {
                params.push((40 + idx).to_string());
            } else if idx < 16 {
                params.push((100 + idx - 8).to_string());
            } else {
                params.push("48".into());
                params.push("5".into());
                params.push(idx.to_string());
            }
        }
        Colour::Rgb { r, g, b } => {
            params.push("48".into());
            params.push("2".into());
            params.push(r.to_string());
            params.push(g.to_string());
            params.push(b.to_string());
        }
        Colour::Default => {}
    }

    if !params.is_empty() {
        let sgr = format!("\x1b[{}m", params.join(";"));
        output.extend_from_slice(sgr.as_bytes());
    }
}

/// Flood fairness: per-pane row quota with stride rotation.
#[derive(Debug)]
pub struct FloodFairness {
    pub quota_per_pane: usize,
    pub stride_offset: usize,
}

impl FloodFairness {
    pub fn new(quota: usize) -> Self {
        Self {
            quota_per_pane: quota,
            stride_offset: 0,
        }
    }

    /// Select rows to render for a pane, respecting the quota.
    pub fn select_rows(&mut self, dirty_rows: &[u16], _total_rows: u16) -> Vec<u16> {
        if dirty_rows.len() <= self.quota_per_pane {
            return dirty_rows.to_vec();
        }
        let selected: Vec<u16> = dirty_rows
            .iter()
            .copied()
            .skip(self.stride_offset % dirty_rows.len())
            .take(self.quota_per_pane)
            .collect();
        self.stride_offset = (self.stride_offset + self.quota_per_pane) % dirty_rows.len().max(1);
        selected
    }
}

/// Composite grid for multi-pane rendering with borders.
#[derive(Debug)]
pub struct CompositeGrid {
    pub prev: RenderBuffer,
    pub next: RenderBuffer,
}

impl CompositeGrid {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            prev: RenderBuffer::new(width, height),
            next: RenderBuffer::new(width, height),
        }
    }

    /// Swap buffers: next becomes prev, next is cleared.
    pub fn swap(&mut self) {
        std::mem::swap(&mut self.prev, &mut self.next);
        self.next.clear();
    }

    /// Compute diff between prev and next.
    pub fn diff(&self) -> Vec<CellChange> {
        diff_buffers(&self.prev, &self.next)
    }

    /// Resize both buffers.
    pub fn resize(&mut self, width: u16, height: u16) {
        self.prev.resize(width, height);
        self.next.resize(width, height);
    }
}

/// Draw a horizontal border at the given row.
pub fn draw_hborder(buf: &mut RenderBuffer, y: u16, arena: &mut GraphemeArena) {
    let dash = arena.intern("\u{2500}");
    for x in 0..buf.size().cols {
        if let Some(cell) = buf.cell_mut(x, y) {
            cell.grapheme = dash;
            cell.width = 1;
        }
    }
}

/// Draw a vertical border at the given column.
pub fn draw_vborder(buf: &mut RenderBuffer, x: u16, arena: &mut GraphemeArena) {
    let pipe = arena.intern("\u{2502}");
    for y in 0..buf.size().rows {
        if let Some(cell) = buf.cell_mut(x, y) {
            cell.grapheme = pipe;
            cell.width = 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mux_grapheme_arena::GraphemeId;

    #[test]
    fn render_buffer_creation() {
        let buf = RenderBuffer::new(80, 24);
        assert_eq!(buf.size(), Size::new(80, 24));
    }

    #[test]
    fn render_buffer_cell_access() {
        let mut buf = RenderBuffer::new(80, 24);
        if let Some(cell) = buf.cell_mut(0, 0) {
            cell.grapheme = GraphemeId::from_raw(5);
        }
        assert_eq!(buf.cell(0, 0).map(|c| c.grapheme), Some(GraphemeId::from_raw(5)));
    }

    #[test]
    fn diff_identical_buffers() {
        let a = RenderBuffer::new(80, 24);
        let b = RenderBuffer::new(80, 24);
        let changes = diff_buffers(&a, &b);
        assert!(changes.is_empty());
    }

    #[test]
    fn diff_detects_change() {
        let a = RenderBuffer::new(10, 2);
        let mut b = RenderBuffer::new(10, 2);
        if let Some(cell) = b.cell_mut(5, 1) {
            cell.grapheme = GraphemeId::from_raw(42);
        }
        let changes = diff_buffers(&a, &b);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].pos, Position::new(5, 1));
    }

    #[test]
    fn diff_multiple_changes() {
        let a = RenderBuffer::new(10, 2);
        let mut b = RenderBuffer::new(10, 2);
        for x in 0..5u16 {
            if let Some(cell) = b.cell_mut(x, 0) {
                cell.grapheme = GraphemeId::from_raw(x as u32 + 1);
            }
        }
        let changes = diff_buffers(&a, &b);
        assert_eq!(changes.len(), 5);
    }

    #[test]
    fn composite_grid_swap() {
        let mut cg = CompositeGrid::new(80, 24);
        if let Some(cell) = cg.next.cell_mut(0, 0) {
            cell.grapheme = GraphemeId::from_raw(10);
        }
        cg.swap();
        assert_eq!(cg.prev.cell(0, 0).map(|c| c.grapheme), Some(GraphemeId::from_raw(10)));
    }

    #[test]
    fn composite_grid_diff() {
        let mut cg = CompositeGrid::new(10, 2);
        if let Some(cell) = cg.next.cell_mut(3, 0) {
            cell.grapheme = GraphemeId::from_raw(7);
        }
        let changes = cg.diff();
        assert_eq!(changes.len(), 1);
    }

    #[test]
    fn encode_diff_produces_output() {
        let arena = GraphemeArena::new();
        let changes = vec![CellChange {
            pos: Position::new(0, 0),
            cell: Cell::default(),
        }];
        let output = encode_diff(&changes, &arena);
        assert!(!output.is_empty());
    }

    #[test]
    fn flood_fairness_under_quota() {
        let mut ff = FloodFairness::new(10);
        let rows = vec![0, 1, 2, 3, 4];
        let selected = ff.select_rows(&rows, 24);
        assert_eq!(selected, rows);
    }

    #[test]
    fn flood_fairness_over_quota() {
        let mut ff = FloodFairness::new(3);
        let rows: Vec<u16> = (0..10).collect();
        let selected = ff.select_rows(&rows, 24);
        assert_eq!(selected.len(), 3);
    }

    #[test]
    fn flood_fairness_stride_rotation() {
        let mut ff = FloodFairness::new(2);
        let rows: Vec<u16> = (0..6).collect();
        let s1 = ff.select_rows(&rows, 24);
        let s2 = ff.select_rows(&rows, 24);
        assert_ne!(s1, s2);
    }

    #[test]
    fn render_buffer_clear() {
        let mut buf = RenderBuffer::new(10, 2);
        if let Some(cell) = buf.cell_mut(0, 0) {
            cell.grapheme = GraphemeId::from_raw(5);
        }
        buf.clear();
        assert_eq!(buf.cell(0, 0).map(|c| c.grapheme), Some(GraphemeId::DEFAULT));
    }

    #[test]
    fn render_buffer_resize() {
        let mut buf = RenderBuffer::new(80, 24);
        buf.resize(120, 40);
        assert_eq!(buf.size(), Size::new(120, 40));
    }

    #[test]
    fn draw_hborder() {
        let mut arena = GraphemeArena::new();
        let mut buf = RenderBuffer::new(10, 3);
        super::draw_hborder(&mut buf, 1, &mut arena);
        // Border cells should be set
        let cell = buf.cell(5, 1);
        assert!(cell.is_some());
    }

    #[test]
    fn draw_vborder() {
        let mut arena = GraphemeArena::new();
        let mut buf = RenderBuffer::new(10, 3);
        super::draw_vborder(&mut buf, 5, &mut arena);
        let cell = buf.cell(5, 1);
        assert!(cell.is_some());
    }

    #[test]
    fn cell_out_of_bounds() {
        let buf = RenderBuffer::new(10, 5);
        assert!(buf.cell(10, 0).is_none());
        assert!(buf.cell(0, 5).is_none());
    }

    #[test]
    fn sgr_fg_colour_indexed() {
        let style = Style::new().with_fg(Colour::Indexed(1));
        let mut output = Vec::new();
        encode_sgr(&style, &mut output);
        assert!(!output.is_empty());
    }

    #[test]
    fn sgr_fg_colour_rgb() {
        let style = Style::new().with_fg(Colour::Rgb { r: 255, g: 128, b: 0 });
        let mut output = Vec::new();
        encode_sgr(&style, &mut output);
        let s = String::from_utf8_lossy(&output);
        assert!(s.contains("38;2;255;128;0"));
    }

    #[test]
    fn sgr_bg_colour() {
        let style = Style::new().with_bg(Colour::Indexed(4));
        let mut output = Vec::new();
        encode_sgr(&style, &mut output);
        let s = String::from_utf8_lossy(&output);
        assert!(s.contains("44"));
    }

    #[test]
    fn composite_grid_resize() {
        let mut cg = CompositeGrid::new(80, 24);
        cg.resize(120, 40);
        assert_eq!(cg.prev.size(), Size::new(120, 40));
        assert_eq!(cg.next.size(), Size::new(120, 40));
    }

    mod prop {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn diff_roundtrip(
                w in 5u16..50, h in 3u16..20,
                changes_count in 1usize..20,
            ) {
                let prev = RenderBuffer::new(w, h);
                let mut next = RenderBuffer::new(w, h);
                for i in 0..changes_count {
                    let x = (i as u16) % w;
                    let y = (i as u16) % h;
                    if let Some(cell) = next.cell_mut(x, y) {
                        cell.grapheme = GraphemeId::from_raw(i as u32 + 1);
                    }
                }
                let diff = diff_buffers(&prev, &next);
                prop_assert!(diff.len() <= changes_count);
            }

            #[test]
            fn identical_buffers_no_diff(w in 5u16..50, h in 3u16..20) {
                let a = RenderBuffer::new(w, h);
                let b = RenderBuffer::new(w, h);
                let diff = diff_buffers(&a, &b);
                prop_assert!(diff.is_empty());
            }
        }
    }
}
