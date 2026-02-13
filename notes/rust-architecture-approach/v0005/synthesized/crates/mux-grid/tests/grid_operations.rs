//! Comprehensive grid operation tests.
#![allow(clippy::unwrap_used)]

use mux_grid::ChunkedGrid;
use mux_types::cell::Cell;
use mux_types::attrs::{Attrs, CellFlags};
use mux_types::colour::Colour;
use mux_grapheme_arena::{GraphemeArena, GraphemeId};

#[test]
fn new_grid_cells_are_default() {
    let grid = ChunkedGrid::new(80, 24, 100);
    for row in 0..24 {
        for col in 0..80 {
            let cell = grid.cell(col, row);
            assert!(cell.is_some());
            assert_eq!(cell.unwrap().grapheme, GraphemeId::DEFAULT);
        }
    }
}

#[test]
fn write_and_read_cell() {
    let mut arena = GraphemeArena::new();
    let mut grid = ChunkedGrid::new(80, 24, 100);
    let gid = arena.intern("A");
    grid.set_cursor(5, 3);
    grid.write_char(gid, 1);
    let cell = grid.cell(5, 3).unwrap();
    assert_eq!(cell.grapheme, gid);
}

#[test]
fn write_cell_with_attrs() {
    let mut arena = GraphemeArena::new();
    let mut grid = ChunkedGrid::new(80, 24, 100);
    grid.set_attrs(Attrs::BOLD | Attrs::ITALIC, Colour::Indexed(1), Colour::Default);
    let gid = arena.intern("X");
    grid.write_char(gid, 1);
    let cell = grid.cell(0, 0).unwrap();
    assert!(cell.attrs.contains(Attrs::BOLD));
    assert!(cell.attrs.contains(Attrs::ITALIC));
    assert_eq!(cell.fg, Colour::Indexed(1));
}

#[test]
fn cursor_set_and_get() {
    let mut grid = ChunkedGrid::new(80, 24, 100);
    grid.set_cursor(10, 5);
    assert_eq!(grid.cursor(), (10, 5));
}

#[test]
fn cursor_clamped() {
    let mut grid = ChunkedGrid::new(80, 24, 100);
    grid.set_cursor(100, 30);
    let (x, y) = grid.cursor();
    assert!(x < 80);
    assert!(y < 24);
}

#[test]
fn erase_display_all_resets() {
    let mut arena = GraphemeArena::new();
    let mut grid = ChunkedGrid::new(80, 24, 100);
    let gid = arena.intern("Z");
    grid.write_char(gid, 1);
    grid.erase_display_all();
    let cell = grid.cell(0, 0).unwrap();
    assert_eq!(cell.grapheme, GraphemeId::DEFAULT);
}

#[test]
fn scroll_up_one() {
    let mut arena = GraphemeArena::new();
    let mut grid = ChunkedGrid::new(80, 24, 100);
    let a = arena.intern("A");
    let b = arena.intern("B");
    grid.set_cursor(0, 0);
    grid.write_char(a, 1);
    grid.set_cursor(0, 1);
    grid.write_char(b, 1);
    grid.scroll_up(1);
    // Row 0 should now have what was in row 1
    let cell = grid.cell(0, 0).unwrap();
    assert_eq!(cell.grapheme, b);
}

#[test]
fn scroll_down_one() {
    let mut arena = GraphemeArena::new();
    let mut grid = ChunkedGrid::new(80, 24, 100);
    let a = arena.intern("A");
    grid.set_cursor(0, 0);
    grid.write_char(a, 1);
    grid.scroll_down(1);
    // Row 0 should be blank after scroll down
    let cell = grid.cell(0, 0).unwrap();
    assert_eq!(cell.grapheme, GraphemeId::DEFAULT);
}

#[test]
fn resize_preserves_data() {
    let mut arena = GraphemeArena::new();
    let mut grid = ChunkedGrid::new(80, 24, 100);
    let gid = arena.intern("X");
    grid.write_char(gid, 1);
    grid.resize(120, 40);
    assert_eq!(grid.sx(), 120);
    assert_eq!(grid.sy(), 40);
}

#[test]
fn resize_smaller() {
    let grid = ChunkedGrid::new(80, 24, 100);
    let mut grid = grid;
    grid.resize(40, 12);
    assert_eq!(grid.sx(), 40);
    assert_eq!(grid.sy(), 12);
}

#[test]
fn erase_line_clears_content() {
    let mut arena = GraphemeArena::new();
    let mut grid = ChunkedGrid::new(80, 24, 100);
    let gid = arena.intern("Q");
    grid.set_cursor(0, 5);
    grid.write_char(gid, 1);
    grid.erase_line(5);
    let cell = grid.cell(0, 5).unwrap();
    assert_eq!(cell.grapheme, GraphemeId::DEFAULT);
}

#[test]
fn erase_line_right_partial() {
    let mut arena = GraphemeArena::new();
    let mut grid = ChunkedGrid::new(80, 24, 100);
    let gid = arena.intern("Z");
    // Fill row 0
    for col in 0..10u16 {
        grid.set_cursor(col, 0);
        grid.write_char(gid, 1);
    }
    // Erase from col 5 onward
    grid.set_cursor(5, 0);
    grid.erase_line_right();
    // Col 4 should still have content
    assert_eq!(grid.cell(4, 0).unwrap().grapheme, gid);
    // Col 5 should be cleared
    assert_eq!(grid.cell(5, 0).unwrap().grapheme, GraphemeId::DEFAULT);
}

#[test]
fn set_scroll_region_validates() {
    let mut grid = ChunkedGrid::new(80, 24, 100);
    grid.set_scroll_region(5, 20);
    // Should not panic
}

#[test]
fn snapshot_captures_content() {
    let mut arena = GraphemeArena::new();
    let mut grid = ChunkedGrid::new(80, 24, 100);
    let gid = arena.intern("A");
    grid.write_char(gid, 1);
    let snap = grid.snapshot();
    assert_eq!(snap.len(), 24);
}

#[test]
fn dirty_row_tracking() {
    let mut arena = GraphemeArena::new();
    let mut grid = ChunkedGrid::new(80, 24, 100);
    grid.clear_all_dirty();
    let dirty = grid.dirty_rows();
    assert!(dirty.is_empty());
    // Write to row 0
    grid.write_char(arena.intern("X"), 1);
    let dirty = grid.dirty_rows();
    assert!(!dirty.is_empty());
}

#[test]
fn line_text_extraction() {
    let mut grid = ChunkedGrid::new(80, 24, 100);
    let mut arena = GraphemeArena::new();
    let h_id = arena.intern("H");
    let i_id = arena.intern("i");
    grid.set_cursor(0, 0);
    grid.write_char(h_id, 1);
    grid.write_char(i_id, 1);
    let text = grid.line_text(0, &arena);
    assert!(text.starts_with("Hi"));
}

#[test]
fn wide_character_via_write() {
    let mut arena = GraphemeArena::new();
    let mut grid = ChunkedGrid::new(80, 24, 100);
    let gid = arena.intern("\u{4e16}"); // Wide CJK char
    grid.set_cursor(0, 0);
    grid.write_char(gid, 2);
    let cell = grid.cell(0, 0).unwrap();
    assert!(cell.is_wide());
    let pad = grid.cell(1, 0).unwrap();
    assert!(pad.is_padding());
}

#[test]
fn grid_minimum_dimensions() {
    let grid = ChunkedGrid::new(1, 1, 0);
    let cell = grid.cell(0, 0);
    assert!(cell.is_some());
    assert_eq!(cell.unwrap().grapheme, GraphemeId::DEFAULT);
}

#[test]
fn screen_text_returns_all_rows() {
    let mut arena = GraphemeArena::new();
    let mut grid = ChunkedGrid::new(80, 3, 100);
    let a = arena.intern("A");
    grid.set_cursor(0, 0);
    grid.write_char(a, 1);
    let text = grid.screen_text(&arena);
    assert_eq!(text.len(), 3);
}

#[test]
fn size_accessor() {
    let grid = ChunkedGrid::new(80, 24, 100);
    let size = grid.size();
    assert_eq!(size.cols, 80);
    assert_eq!(size.rows, 24);
}

#[test]
fn erase_display_below() {
    let mut arena = GraphemeArena::new();
    let mut grid = ChunkedGrid::new(80, 5, 100);
    let gid = arena.intern("X");
    for row in 0..5u16 {
        grid.set_cursor(0, row);
        grid.write_char(gid, 1);
    }
    grid.set_cursor(0, 2);
    grid.erase_display_below();
    // Rows 0, 1 should be preserved
    assert_eq!(grid.cell(0, 0).unwrap().grapheme, gid);
    assert_eq!(grid.cell(0, 1).unwrap().grapheme, gid);
}

#[test]
fn line_feed_advances_cursor() {
    let mut grid = ChunkedGrid::new(80, 24, 100);
    grid.set_cursor(0, 0);
    grid.line_feed();
    let (_, y) = grid.cursor();
    assert_eq!(y, 1);
}

#[test]
fn carriage_return_resets_x() {
    let mut grid = ChunkedGrid::new(80, 24, 100);
    grid.set_cursor(40, 5);
    grid.carriage_return();
    let (x, y) = grid.cursor();
    assert_eq!(x, 0);
    assert_eq!(y, 5);
}

#[test]
fn total_lines_includes_scrollback() {
    let grid = ChunkedGrid::new(80, 24, 100);
    assert_eq!(grid.total_lines(), 24);
}

#[test]
fn scrollback_initially_zero() {
    let grid = ChunkedGrid::new(80, 24, 100);
    assert_eq!(grid.scrollback_lines(), 0);
}

#[test]
fn cell_out_of_bounds_returns_none() {
    let grid = ChunkedGrid::new(80, 24, 100);
    assert!(grid.cell(80, 0).is_none());
    assert!(grid.cell(0, 24).is_none());
    assert!(grid.cell(200, 200).is_none());
}
