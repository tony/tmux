//! Integration tests: grid snapshot across crate boundaries.
#![allow(clippy::unwrap_used)]

use mux_grid::ChunkedGrid;
use mux_grapheme_arena::GraphemeArena;

#[test]
fn snapshot_cow_isolation() {
    let mut arena = GraphemeArena::new();
    let mut grid = ChunkedGrid::new(80, 24, 100);
    let id = arena.intern("A");
    grid.write_char(id, 1);

    let snap = grid.snapshot();
    let snap_text = snap[0].text(&arena);

    // Modify grid
    grid.set_cursor(0, 0);
    let id2 = arena.intern("B");
    grid.write_char(id2, 1);

    // Snapshot unchanged
    assert_eq!(snap[0].text(&arena), snap_text);
    assert_eq!(grid.line_text(0, &arena), "B");
}

#[test]
fn grid_resize_and_snapshot() {
    let mut arena = GraphemeArena::new();
    let mut grid = ChunkedGrid::new(80, 24, 100);
    let id = arena.intern("X");
    grid.write_char(id, 1);
    grid.resize(120, 40);
    let snap = grid.snapshot();
    assert_eq!(snap.len(), 40);
}

#[test]
fn wide_char_snapshot() {
    let mut arena = GraphemeArena::new();
    let mut grid = ChunkedGrid::new(80, 24, 100);
    let id = arena.intern("\u{4e16}");
    grid.write_char(id, 2);
    let snap = grid.snapshot();
    assert!(snap[0].cell(1).map(mux_types::cell::Cell::is_padding).unwrap_or(false));
}
