//! Property-based tests for mux-render.
#![allow(clippy::unwrap_used)]

use proptest::prelude::*;
use mux_render::{RenderBuffer, diff_buffers, encode_diff, CompositeGrid, FloodFairness};
use mux_types::cell::Cell;
use mux_types::colour::Colour;
use mux_types::attrs::Attrs;
use mux_grapheme_arena::{GraphemeArena, GraphemeId};

proptest! {
    #[test]
    fn identical_buffers_no_diff(cols in 1..50u16, rows in 1..20u16) {
        let a = RenderBuffer::new(cols, rows);
        let b = RenderBuffer::new(cols, rows);
        let diff = diff_buffers(&a, &b);
        prop_assert!(diff.is_empty());
    }

    #[test]
    fn single_cell_diff(col in 0..79u16, row in 0..23u16) {
        let prev = RenderBuffer::new(80, 24);
        let mut next = RenderBuffer::new(80, 24);
        if let Some(cell) = next.cell_mut(col, row) {
            cell.grapheme = GraphemeId::from_raw(42);
        }
        let diff = diff_buffers(&prev, &next);
        prop_assert!(!diff.is_empty());
    }

    #[test]
    fn diff_only_changed_cells(
        changes in proptest::collection::vec((0..79u16, 0..23u16), 1..20),
    ) {
        let prev = RenderBuffer::new(80, 24);
        let mut next = RenderBuffer::new(80, 24);
        for &(col, row) in &changes {
            if let Some(cell) = next.cell_mut(col, row) {
                cell.grapheme = GraphemeId::from_raw(col as u32 + 1);
            }
        }
        let diff = diff_buffers(&prev, &next);
        prop_assert!(diff.len() <= changes.len());
    }

    #[test]
    fn encode_diff_produces_bytes(col in 0..79u16, row in 0..23u16) {
        let prev = RenderBuffer::new(80, 24);
        let mut next = RenderBuffer::new(80, 24);
        if let Some(cell) = next.cell_mut(col, row) {
            cell.grapheme = GraphemeId::from_raw(65);
        }
        let diff = diff_buffers(&prev, &next);
        let arena = GraphemeArena::new();
        let bytes = encode_diff(&diff, &arena);
        prop_assert!(!bytes.is_empty());
    }

    #[test]
    fn flood_fairness_limits_rows(quota in 1..20usize) {
        let mut ff = FloodFairness::new(quota);
        let dirty: Vec<u16> = (0..50).collect();
        let selected = ff.select_rows(&dirty, 50);
        prop_assert!(selected.len() <= quota);
    }

    #[test]
    fn composite_grid_swap_alternates(_dummy in 0..5u8) {
        let mut cg = CompositeGrid::new(80, 24);
        cg.swap();
        cg.swap();
    }

    #[test]
    fn render_buffer_cell_roundtrip(col in 0..79u16, row in 0..23u16, gid in 1..100u32) {
        let mut buf = RenderBuffer::new(80, 24);
        if let Some(cell) = buf.cell_mut(col, row) {
            cell.grapheme = GraphemeId::from_raw(gid);
        }
        let cell = buf.cell(col, row);
        prop_assert!(cell.is_some());
        prop_assert_eq!(cell.unwrap().grapheme, GraphemeId::from_raw(gid));
    }
}

#[test]
fn render_buffer_default_cells() {
    let buf = RenderBuffer::new(80, 24);
    let cell = buf.cell(0, 0).unwrap();
    assert_eq!(cell.grapheme, GraphemeId::DEFAULT);
}

#[test]
fn diff_with_colour_change() {
    let prev = RenderBuffer::new(80, 24);
    let mut next = RenderBuffer::new(80, 24);
    if let Some(cell) = next.cell_mut(0, 0) {
        cell.fg = Colour::Indexed(1);
    }
    let diff = diff_buffers(&prev, &next);
    assert!(!diff.is_empty());
}

#[test]
fn diff_with_attrs_change() {
    let prev = RenderBuffer::new(80, 24);
    let mut next = RenderBuffer::new(80, 24);
    if let Some(cell) = next.cell_mut(0, 0) {
        cell.attrs = Attrs::BOLD;
    }
    let diff = diff_buffers(&prev, &next);
    assert!(!diff.is_empty());
}

#[test]
fn encode_diff_empty() {
    let arena = GraphemeArena::new();
    let bytes = encode_diff(&[], &arena);
    assert!(bytes.is_empty());
}

#[test]
fn composite_grid_diff_empty_initially() {
    let cg = CompositeGrid::new(80, 24);
    let diff = cg.diff();
    assert!(diff.is_empty());
}
