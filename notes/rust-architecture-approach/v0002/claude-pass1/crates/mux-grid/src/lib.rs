//! # mux-grid
//!
//! Chunked COW terminal grid -- the v25 DEFINITIVE grid design (INV-127).
//!
//! ## Design
//! - `Vec<Arc<LineChunk>>` with `CHUNK_SIZE=256` lines per chunk.
//! - Single `line_dirty: BitVec` per grid.
//! - Clone cost: 16-32 KB per mutation with outstanding snapshot.

#![forbid(unsafe_code)]

use std::sync::Arc;

use bitvec::prelude::*;
use mux_grapheme_arena::GraphemeId;
use mux_types::{Cell, CellFlags, Colour, Size};

/// Number of lines per chunk (INV-127).
pub const CHUNK_SIZE: usize = 256;

/// A single line in the terminal grid.
#[derive(Debug, Clone)]
pub struct Line {
    cells: Vec<Cell>,
}

impl Line {
    /// Create a new blank line with the given width.
    #[must_use]
    pub fn new(width: u32) -> Self {
        Self {
            cells: vec![Cell::empty(); width as usize],
        }
    }

    /// Get a reference to a cell.
    #[must_use]
    pub fn cell(&self, x: u32) -> Option<&Cell> {
        self.cells.get(x as usize)
    }

    /// Get a mutable reference to a cell.
    pub fn cell_mut(&mut self, x: u32) -> Option<&mut Cell> {
        self.cells.get_mut(x as usize)
    }

    /// Number of cells.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.cells.len() as u32
    }

    /// Slice of all cells.
    #[must_use]
    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    /// Resize the line to a new width.
    pub fn resize(&mut self, new_width: u32) {
        self.cells.resize(new_width as usize, Cell::empty());
    }

    /// Clear all cells to empty.
    pub fn clear(&mut self) {
        for cell in &mut self.cells {
            *cell = Cell::empty();
        }
    }

    /// Whether all cells are empty.
    #[must_use]
    pub fn is_blank(&self) -> bool {
        self.cells.iter().all(Cell::is_empty)
    }

    /// Set a cell, performing wide-char co-invalidation.
    /// Verified against tmux screen-write.c:1928-1976.
    pub fn set_cell(&mut self, x: u32, cell: Cell) {
        let x = x as usize;
        let width = self.cells.len();
        if x >= width {
            return;
        }

        // If overwriting a padding cell, clear the wide char's primary cell
        if self.cells[x].flags.contains(CellFlags::PADDING) && x > 0 {
            self.cells[x - 1] = Cell::empty();
        }

        // If overwriting the primary cell of a wide char, clear the padding
        if x + 1 < width && self.cells[x + 1].flags.contains(CellFlags::PADDING) {
            self.cells[x + 1] = Cell::empty();
        }

        // If placing a wide char, set next cell as padding
        if cell.width == 2 && x + 1 < width {
            self.cells[x + 1] = Cell::padding();
        }

        self.cells[x] = cell;
    }
}

/// A chunk of `CHUNK_SIZE` lines stored behind an `Arc` for COW.
#[derive(Debug, Clone)]
pub struct LineChunk {
    pub lines: Vec<Line>,
}

impl LineChunk {
    /// Create a new chunk with blank lines of the given width.
    #[must_use]
    pub fn new_blank(width: u32, count: usize) -> Self {
        Self {
            lines: (0..count).map(|_| Line::new(width)).collect(),
        }
    }
}

/// A snapshot of the grid, created by `ChunkedGrid::snapshot()`.
#[derive(Debug, Clone)]
pub struct GridSnapshot {
    pub chunks: Vec<Arc<LineChunk>>,
    pub size: Size,
    pub hsize: u32,
    pub dirty: BitVec,
    pub revision: u64,
}

/// The chunked COW terminal grid (INV-127).
#[derive(Debug)]
pub struct ChunkedGrid {
    chunks: Vec<Arc<LineChunk>>,
    line_dirty: BitVec,
    sx: u32,
    sy: u32,
    hsize: u32,
    hlimit: u32,
    hscrolled: u32,
}

impl ChunkedGrid {
    /// Create a new grid with the given dimensions.
    #[must_use]
    pub fn new(sx: u32, sy: u32, hlimit: u32) -> Self {
        let total_lines = sy as usize;
        let num_chunks = (total_lines + CHUNK_SIZE - 1) / CHUNK_SIZE;

        let mut chunks = Vec::with_capacity(num_chunks);
        let mut remaining = total_lines;
        for _ in 0..num_chunks {
            let count = remaining.min(CHUNK_SIZE);
            chunks.push(Arc::new(LineChunk::new_blank(sx, count)));
            remaining -= count;
        }

        Self {
            chunks,
            line_dirty: bitvec![0; total_lines],
            sx,
            sy,
            hsize: 0,
            hlimit,
            hscrolled: 0,
        }
    }

    #[must_use]
    pub const fn sx(&self) -> u32 { self.sx }

    #[must_use]
    pub const fn sy(&self) -> u32 { self.sy }

    #[must_use]
    pub const fn hsize(&self) -> u32 { self.hsize }

    #[must_use]
    pub const fn hlimit(&self) -> u32 { self.hlimit }

    #[must_use]
    pub fn total_lines(&self) -> usize {
        self.chunks.iter().map(|c| c.lines.len()).sum()
    }

    /// Read-only access to a line. No COW triggered.
    #[must_use]
    pub fn line(&self, y: u32) -> Option<&Line> {
        let y = y as usize;
        let (chunk_idx, line_idx) = self.line_coords(y)?;
        self.chunks
            .get(chunk_idx)
            .and_then(|chunk| chunk.lines.get(line_idx))
    }

    /// Mutable access to a line. Triggers COW if chunk is shared.
    pub fn line_mut(&mut self, y: u32) -> Option<&mut Line> {
        let y_usize = y as usize;
        let (chunk_idx, line_idx) = self.line_coords(y_usize)?;

        if y_usize < self.line_dirty.len() {
            self.line_dirty.set(y_usize, true);
        }

        let chunk = self.chunks.get_mut(chunk_idx)?;
        let chunk_inner = Arc::make_mut(chunk);
        chunk_inner.lines.get_mut(line_idx)
    }

    /// Create a snapshot sharing chunk storage via `Arc`.
    #[must_use]
    pub fn snapshot(&self, revision: u64) -> GridSnapshot {
        GridSnapshot {
            chunks: self.chunks.clone(),
            size: Size::new(self.sx, self.sy),
            hsize: self.hsize,
            dirty: self.line_dirty.clone(),
            revision,
        }
    }

    /// Clear all dirty bits.
    pub fn clear_dirty(&mut self) {
        self.line_dirty.fill(false);
    }

    /// Check if a line is dirty.
    #[must_use]
    pub fn is_dirty(&self, y: u32) -> bool {
        self.line_dirty
            .get(y as usize)
            .as_deref()
            .copied()
            .unwrap_or(false)
    }

    /// Mark all lines as dirty (used after reflow and full redraw).
    pub fn mark_all_dirty(&mut self) {
        self.line_dirty.fill(true);
    }

    /// Scroll a region of the viewport (INV-127 chunk-boundary-aware).
    pub fn scroll_region(&mut self, upper: u32, lower: u32, _bg: Colour) {
        if upper >= lower || lower >= self.sy {
            return;
        }

        for y in upper..=lower {
            let y_abs = self.hsize + y;
            if (y_abs as usize) < self.line_dirty.len() {
                self.line_dirty.set(y_abs as usize, true);
            }
        }

        for y in upper..lower {
            let src_y = self.hsize + y + 1;
            let dst_y = self.hsize + y;
            let src_line = self.line(src_y).cloned();
            if let Some(line) = src_line {
                if let Some(dst) = self.line_mut(dst_y) {
                    *dst = line;
                }
            }
        }

        let bottom_y = self.hsize + lower;
        if let Some(line) = self.line_mut(bottom_y) {
            line.clear();
        }
    }

    /// Reflow the grid to a new width (INV-203: marks all dirty).
    pub fn reflow(&mut self, new_sx: u32) {
        let total = self.total_lines();
        let mut new_lines: Vec<Line> = Vec::with_capacity(total);

        for chunk in &self.chunks {
            for line in &chunk.lines {
                let mut new_line = line.clone();
                new_line.resize(new_sx);
                new_lines.push(new_line);
            }
        }

        let num_chunks = (new_lines.len() + CHUNK_SIZE - 1) / CHUNK_SIZE;
        let mut new_chunks = Vec::with_capacity(num_chunks);

        for chunk_lines in new_lines.chunks(CHUNK_SIZE) {
            new_chunks.push(Arc::new(LineChunk {
                lines: chunk_lines.to_vec(),
            }));
        }

        self.chunks = new_chunks;
        self.sx = new_sx;
        self.line_dirty = bitvec![1; total];
    }

    fn line_coords(&self, y: usize) -> Option<(usize, usize)> {
        let mut offset = 0;
        for (i, chunk) in self.chunks.iter().enumerate() {
            let chunk_len = chunk.lines.len();
            if y < offset + chunk_len {
                return Some((i, y - offset));
            }
            offset += chunk_len;
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_grid_dimensions() {
        let g = ChunkedGrid::new(80, 24, 10_000);
        assert_eq!(g.sx(), 80);
        assert_eq!(g.sy(), 24);
        assert_eq!(g.total_lines(), 24);
    }

    #[test]
    fn line_read_only_no_cow() {
        let g = ChunkedGrid::new(80, 24, 10_000);
        let line = g.line(0);
        assert!(line.is_some());
        assert!(line.map_or(false, Line::is_blank));
    }

    #[test]
    fn line_mut_marks_dirty() {
        let mut g = ChunkedGrid::new(80, 24, 10_000);
        assert!(!g.is_dirty(0));
        let _line = g.line_mut(0);
        assert!(g.is_dirty(0));
    }

    #[test]
    fn snapshot_shares_chunks() {
        let mut g = ChunkedGrid::new(80, 24, 10_000);
        let snap = g.snapshot(1);
        assert_eq!(snap.chunks.len(), g.chunks.len());
        let _line = g.line_mut(0);
        assert!(snap.chunks[0].lines[0].is_blank());
    }

    #[test]
    fn cow_on_shared_chunk() {
        let mut g = ChunkedGrid::new(80, 24, 10_000);
        let snap = g.snapshot(1);
        assert_eq!(Arc::strong_count(&g.chunks[0]), 2);

        if let Some(line) = g.line_mut(0) {
            line.set_cell(0, Cell {
                grapheme: GraphemeId::from_char('X'),
                width: 1,
                flags: CellFlags::empty(),
                ..Cell::empty()
            });
        }

        assert_eq!(Arc::strong_count(&g.chunks[0]), 1);
        assert_eq!(Arc::strong_count(&snap.chunks[0]), 1);
    }

    #[test]
    fn wide_char_co_invalidation() {
        let mut line = Line::new(10);
        let wide = Cell {
            grapheme: GraphemeId::from_char('\u{4E16}'),
            width: 2,
            flags: CellFlags::empty(),
            ..Cell::empty()
        };
        line.set_cell(3, wide);
        assert!(line.cell(4).map_or(false, Cell::is_padding));

        line.set_cell(4, Cell {
            grapheme: GraphemeId::from_char('A'),
            width: 1,
            flags: CellFlags::empty(),
            ..Cell::empty()
        });
        assert!(line.cell(3).map_or(false, Cell::is_empty));
    }

    #[test]
    fn chunk_size_constant() {
        assert_eq!(CHUNK_SIZE, 256);
    }

    #[test]
    fn clear_dirty_resets_all() {
        let mut g = ChunkedGrid::new(80, 24, 10_000);
        let _line = g.line_mut(0);
        assert!(g.is_dirty(0));
        g.clear_dirty();
        assert!(!g.is_dirty(0));
    }

    #[test]
    fn mark_all_dirty() {
        let mut g = ChunkedGrid::new(80, 24, 10_000);
        assert!(!g.is_dirty(0));
        g.mark_all_dirty();
        assert!(g.is_dirty(0));
        assert!(g.is_dirty(23));
    }

    #[test]
    fn reflow_marks_all_dirty() {
        let mut g = ChunkedGrid::new(80, 24, 10_000);
        g.clear_dirty();
        g.reflow(120);
        assert_eq!(g.sx(), 120);
        // After reflow, all lines should be dirty (INV-203)
        assert!(g.is_dirty(0));
    }

    #[test]
    fn scroll_region_clears_bottom() {
        let mut g = ChunkedGrid::new(10, 5, 100);
        // Put content on line 0
        if let Some(line) = g.line_mut(0) {
            line.set_cell(0, Cell {
                grapheme: GraphemeId::from_char('A'),
                width: 1,
                flags: CellFlags::empty(),
                ..Cell::empty()
            });
        }
        g.clear_dirty();
        g.scroll_region(0, 4, Colour::Default);
        // Bottom line (4) should be blank after scroll
        assert!(g.line(4).map_or(false, Line::is_blank));
    }
}
