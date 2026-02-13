//! Property-based tests for mux-grid.
#![allow(clippy::unwrap_used)]

use proptest::prelude::*;
use mux_grid::ChunkedGrid;
use mux_types::cell::Cell;
use mux_types::attrs::CellFlags;
use mux_grapheme_arena::{GraphemeArena, GraphemeId};

proptest! {
    #[test]
    fn grid_dimensions_preserved(cols in 1..200u16, rows in 1..100u16) {
        let grid = ChunkedGrid::new(cols, rows, 100);
        prop_assert_eq!(grid.sx(), cols);
        prop_assert_eq!(grid.sy(), rows);
    }

    #[test]
    fn grid_resize_updates_dimensions(
        cols in 1..100u16, rows in 1..50u16,
        new_cols in 1..200u16, new_rows in 1..100u16,
    ) {
        let mut grid = ChunkedGrid::new(cols, rows, 100);
        grid.resize(new_cols, new_rows);
        prop_assert_eq!(grid.sx(), new_cols);
        prop_assert_eq!(grid.sy(), new_rows);
    }

    #[test]
    fn grid_write_read_roundtrip(col in 0..79u16, row in 0..23u16) {
        let mut arena = GraphemeArena::new();
        let mut grid = ChunkedGrid::new(80, 24, 100);
        let gid = arena.intern("X");
        grid.set_cursor(col, row);
        grid.write_char(gid, 1);
        let cell = grid.cell(col, row);
        prop_assert!(cell.is_some());
        prop_assert_eq!(cell.unwrap().grapheme, gid);
    }

    #[test]
    fn grid_cursor_bounds(col in 0..80u16, row in 0..24u16) {
        let mut grid = ChunkedGrid::new(80, 24, 100);
        grid.set_cursor(col.min(79), row.min(23));
        let (cx, cy) = grid.cursor();
        prop_assert!(cx < 80);
        prop_assert!(cy < 24);
    }

    #[test]
    fn grid_erase_all_resets(cols in 1..50u16, rows in 1..50u16) {
        let mut arena = GraphemeArena::new();
        let mut grid = ChunkedGrid::new(cols, rows, 100);
        let gid = arena.intern("A");
        grid.write_char(gid, 1);
        grid.erase_display_all();
        let cell = grid.cell(0, 0);
        prop_assert!(cell.is_some());
        prop_assert_eq!(cell.unwrap().grapheme, GraphemeId::DEFAULT);
    }

    #[test]
    fn grid_snapshot_matches_grid(cols in 1..50u16, rows in 1..20u16) {
        let grid = ChunkedGrid::new(cols, rows, 100);
        let snap = grid.snapshot();
        prop_assert_eq!(snap.len(), rows as usize);
    }

    #[test]
    fn grid_scroll_up_preserves_dimensions(scroll in 1..10u16) {
        let mut grid = ChunkedGrid::new(80, 24, 100);
        let actual = scroll.min(24);
        grid.scroll_up(actual);
        // After scroll, grid should still have 24 rows height
        prop_assert_eq!(grid.sy(), 24);
    }

    #[test]
    fn grid_dirty_tracking(col in 0..79u16, row in 0..23u16) {
        let mut arena = GraphemeArena::new();
        let mut grid = ChunkedGrid::new(80, 24, 100);
        grid.clear_all_dirty();
        grid.set_cursor(col, row);
        grid.write_char(arena.intern("D"), 1);
        let dirty = grid.dirty_rows();
        prop_assert!(!dirty.is_empty());
    }
}

#[test]
fn grid_erase_line() {
    let mut arena = GraphemeArena::new();
    let mut grid = ChunkedGrid::new(80, 24, 100);
    let gid = arena.intern("X");
    grid.set_cursor(0, 0);
    grid.write_char(gid, 1);
    grid.set_cursor(1, 0);
    grid.write_char(gid, 1);
    grid.erase_line(0);
    assert_eq!(grid.cell(0, 0).unwrap().grapheme, GraphemeId::DEFAULT);
    assert_eq!(grid.cell(1, 0).unwrap().grapheme, GraphemeId::DEFAULT);
}

#[test]
fn grid_erase_line_right() {
    let mut arena = GraphemeArena::new();
    let mut grid = ChunkedGrid::new(80, 24, 100);
    let gid = arena.intern("Z");
    for i in 0..10u16 {
        grid.set_cursor(i, 0);
        grid.write_char(gid, 1);
    }
    grid.set_cursor(5, 0);
    grid.erase_line_right();
    // Cell 5 should be cleared
    assert_eq!(grid.cell(5, 0).unwrap().grapheme, GraphemeId::DEFAULT);
    // Cell 4 should be preserved
    assert_eq!(grid.cell(4, 0).unwrap().grapheme, gid);
}

#[test]
fn grid_set_scroll_region() {
    let mut grid = ChunkedGrid::new(80, 24, 100);
    grid.set_scroll_region(5, 20);
    // Should not panic
}

#[test]
fn grid_minimum_size() {
    let grid = ChunkedGrid::new(1, 1, 0);
    assert_eq!(grid.sx(), 1);
    assert_eq!(grid.sy(), 1);
    let cell = grid.cell(0, 0);
    assert!(cell.is_some());
    assert_eq!(cell.unwrap().grapheme, GraphemeId::DEFAULT);
}
