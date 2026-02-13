//! Property-based tests for mux-snapshot.

use proptest::prelude::*;
use mux_snapshot::GridSnapshot;
use mux_grid::ChunkedGrid;
use mux_grapheme_arena::GraphemeArena;
use mux_types::geometry::Size;

proptest! {
    #[test]
    fn snapshot_preserves_dimensions(cols in 5u16..200, rows in 3u16..100) {
        let grid = ChunkedGrid::new(cols, rows, 100);
        let snap = GridSnapshot::capture(&grid);
        prop_assert_eq!(snap.size, Size::new(cols, rows));
        prop_assert_eq!(snap.rows(), rows);
    }

    #[test]
    fn snapshot_cursor_preserved(cx in 0..79u16, cy in 0..23u16) {
        let mut grid = ChunkedGrid::new(80, 24, 100);
        grid.set_cursor(cx, cy);
        let snap = GridSnapshot::capture(&grid);
        prop_assert_eq!(snap.cursor, (cx, cy));
    }

    #[test]
    fn snapshot_clone_independent(cols in 5u16..50, rows in 3u16..20) {
        let grid = ChunkedGrid::new(cols, rows, 100);
        let snap = GridSnapshot::capture(&grid);
        let snap2 = snap.clone();
        prop_assert_eq!(snap.size, snap2.size);
        prop_assert_eq!(snap.cursor, snap2.cursor);
        prop_assert_eq!(snap.rows(), snap2.rows());
    }

    #[test]
    fn snapshot_text_has_correct_row_count(rows in 1u16..20) {
        let grid = ChunkedGrid::new(80, rows, 100);
        let arena = GraphemeArena::new();
        let snap = GridSnapshot::capture(&grid);
        let text = snap.text(&arena);
        prop_assert_eq!(text.len() as u16, rows);
    }
}

#[test]
fn empty_snapshot() {
    let grid = ChunkedGrid::new(80, 24, 100);
    let arena = GraphemeArena::new();
    let snap = GridSnapshot::capture(&grid);
    assert_eq!(snap.rows(), 24);
    assert!(snap.text(&arena).iter().all(String::is_empty));
}

#[test]
fn snapshot_with_content() {
    let mut arena = GraphemeArena::new();
    let mut grid = ChunkedGrid::new(80, 24, 100);
    let id = arena.intern("A");
    grid.write_char(id, 1);
    let snap = GridSnapshot::capture(&grid);
    assert_eq!(snap.line_text(0, &arena), "A");
}

#[test]
fn snapshot_text_with_graphemes() {
    let mut arena = GraphemeArena::new();
    let mut grid = ChunkedGrid::new(80, 24, 100);
    for ch in "Hello".chars() {
        let id = arena.intern(&ch.to_string());
        grid.write_char(id, 1);
    }
    let snap = GridSnapshot::capture(&grid);
    assert_eq!(snap.line_text(0, &arena), "Hello");
}

#[test]
fn snapshot_isolation_from_grid() {
    let mut arena = GraphemeArena::new();
    let mut grid = ChunkedGrid::new(80, 24, 100);
    let id_a = arena.intern("A");
    grid.write_char(id_a, 1);

    let snap = GridSnapshot::capture(&grid);

    // Modify grid after snapshot
    let id_b = arena.intern("B");
    grid.set_cursor(0, 0);
    grid.write_char(id_b, 1);

    // Snapshot should be unchanged
    assert_eq!(snap.line_text(0, &arena), "A");
    assert_eq!(grid.line_text(0, &arena), "B");
}

#[test]
fn contains_text_found() {
    let mut arena = GraphemeArena::new();
    let mut grid = ChunkedGrid::new(80, 24, 100);
    for ch in "Compiling".chars() {
        let id = arena.intern(&ch.to_string());
        grid.write_char(id, 1);
    }
    let snap = GridSnapshot::capture(&grid);
    assert!(snap.contains_text("Compil", &arena));
}

#[test]
fn contains_text_not_found() {
    let grid = ChunkedGrid::new(80, 24, 100);
    let arena = GraphemeArena::new();
    let snap = GridSnapshot::capture(&grid);
    assert!(!snap.contains_text("Error", &arena));
}

#[test]
fn find_text_on_specific_row() {
    let mut arena = GraphemeArena::new();
    let mut grid = ChunkedGrid::new(80, 3, 100);
    grid.set_cursor(0, 1);
    for ch in "target".chars() {
        let id = arena.intern(&ch.to_string());
        grid.write_char(id, 1);
    }
    let snap = GridSnapshot::capture(&grid);
    assert_eq!(snap.find_text("target", &arena), Some(1));
}

#[test]
fn find_text_not_found() {
    let grid = ChunkedGrid::new(80, 24, 100);
    let arena = GraphemeArena::new();
    let snap = GridSnapshot::capture(&grid);
    assert_eq!(snap.find_text("nonexistent", &arena), None);
}

#[test]
fn line_text_out_of_bounds() {
    let grid = ChunkedGrid::new(80, 5, 100);
    let arena = GraphemeArena::new();
    let snap = GridSnapshot::capture(&grid);
    assert_eq!(snap.line_text(99, &arena), "");
}
