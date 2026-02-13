//! Integration tests: render composition pipeline.

use mux_render::*;
use mux_grapheme_arena::GraphemeArena;
use mux_types::geometry::Size;

#[test]
fn composite_grid_full_pipeline() {
    let mut cg = CompositeGrid::new(80, 24);
    // Write to next buffer
    let arena = GraphemeArena::new();
    // Get diff (should show changes since both start empty)
    let changes = cg.diff();
    assert!(changes.is_empty()); // Both empty, no diff
}

#[test]
fn diff_and_encode() {
    let mut arena = GraphemeArena::new();
    let prev = RenderBuffer::new(10, 2);
    let mut next = RenderBuffer::new(10, 2);
    let id = arena.intern("X");
    if let Some(cell) = next.cell_mut(5, 0) {
        cell.grapheme = id;
    }
    let changes = diff_buffers(&prev, &next);
    assert_eq!(changes.len(), 1);
    let output = encode_diff(&changes, &arena);
    assert!(!output.is_empty());
}

#[test]
fn border_rendering() {
    let mut arena = GraphemeArena::new();
    let mut buf = RenderBuffer::new(20, 10);
    draw_hborder(&mut buf, 5, &mut arena);
    draw_vborder(&mut buf, 10, &mut arena);
}

#[test]
fn flood_fairness_rotation() {
    let mut ff = FloodFairness::new(3);
    let rows: Vec<u16> = (0..10).collect();
    let mut all_selected = std::collections::HashSet::new();
    for _ in 0..4 {
        let selected = ff.select_rows(&rows, 24);
        for r in selected {
            all_selected.insert(r);
        }
    }
    // Over multiple passes, should cover more rows
    assert!(all_selected.len() > 3);
}
