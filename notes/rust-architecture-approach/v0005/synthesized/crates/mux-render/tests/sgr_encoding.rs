//! Tests for SGR encoding and escape sequence generation.
#![allow(clippy::unwrap_used)]

use mux_render::{RenderBuffer, diff_buffers, encode_diff};
use mux_types::cell::Cell;
use mux_types::colour::Colour;
use mux_types::attrs::Attrs;
use mux_grapheme_arena::{GraphemeArena, GraphemeId};

#[test]
fn bold_cell_generates_diff() {
    let prev = RenderBuffer::new(80, 24);
    let mut next = RenderBuffer::new(80, 24);
    if let Some(cell) = next.cell_mut(0, 0) {
        cell.attrs = Attrs::BOLD;
        cell.grapheme = GraphemeId::from_raw(65);
    }
    let diff = diff_buffers(&prev, &next);
    let arena = GraphemeArena::new();
    let bytes = encode_diff(&diff, &arena);
    assert!(!bytes.is_empty());
}

#[test]
fn fg_colour_generates_diff() {
    let prev = RenderBuffer::new(80, 24);
    let mut next = RenderBuffer::new(80, 24);
    if let Some(cell) = next.cell_mut(0, 0) {
        cell.fg = Colour::Indexed(1);
        cell.grapheme = GraphemeId::from_raw(65);
    }
    let diff = diff_buffers(&prev, &next);
    let arena = GraphemeArena::new();
    let bytes = encode_diff(&diff, &arena);
    assert!(!bytes.is_empty());
}

#[test]
fn bg_colour_generates_diff() {
    let prev = RenderBuffer::new(80, 24);
    let mut next = RenderBuffer::new(80, 24);
    if let Some(cell) = next.cell_mut(0, 0) {
        cell.bg = Colour::Indexed(4);
        cell.grapheme = GraphemeId::from_raw(65);
    }
    let diff = diff_buffers(&prev, &next);
    let arena = GraphemeArena::new();
    let bytes = encode_diff(&diff, &arena);
    assert!(!bytes.is_empty());
}

#[test]
fn rgb_fg_generates_diff() {
    let prev = RenderBuffer::new(80, 24);
    let mut next = RenderBuffer::new(80, 24);
    if let Some(cell) = next.cell_mut(0, 0) {
        cell.fg = Colour::Rgb { r: 255, g: 0, b: 0 };
        cell.grapheme = GraphemeId::from_raw(65);
    }
    let diff = diff_buffers(&prev, &next);
    let arena = GraphemeArena::new();
    let bytes = encode_diff(&diff, &arena);
    assert!(!bytes.is_empty());
}

#[test]
fn cursor_movement_in_diff() {
    let prev = RenderBuffer::new(80, 24);
    let mut next = RenderBuffer::new(80, 24);
    if let Some(cell) = next.cell_mut(10, 5) {
        cell.grapheme = GraphemeId::from_raw(65);
    }
    let diff = diff_buffers(&prev, &next);
    let arena = GraphemeArena::new();
    let bytes = encode_diff(&diff, &arena);
    assert!(!bytes.is_empty());
}

#[test]
fn multiple_cells_diff() {
    let prev = RenderBuffer::new(80, 24);
    let mut next = RenderBuffer::new(80, 24);
    for col in 0..10u16 {
        if let Some(cell) = next.cell_mut(col, 0) {
            cell.grapheme = GraphemeId::from_raw(65 + col as u32);
        }
    }
    let diff = diff_buffers(&prev, &next);
    assert_eq!(diff.len(), 10);
}

#[test]
fn no_diff_same_buffer() {
    let buf = RenderBuffer::new(80, 24);
    let diff = diff_buffers(&buf, &buf);
    assert!(diff.is_empty());
}

#[test]
fn attr_change_only() {
    let mut prev = RenderBuffer::new(80, 24);
    let mut next = RenderBuffer::new(80, 24);
    if let Some(cell) = prev.cell_mut(0, 0) {
        cell.grapheme = GraphemeId::from_raw(65);
    }
    if let Some(cell) = next.cell_mut(0, 0) {
        cell.grapheme = GraphemeId::from_raw(65);
        cell.attrs = Attrs::BOLD;
    }
    let diff = diff_buffers(&prev, &next);
    assert!(!diff.is_empty());
}

#[test]
fn render_buffer_size() {
    let buf = RenderBuffer::new(120, 40);
    let size = buf.size();
    assert_eq!(size.cols, 120);
    assert_eq!(size.rows, 40);
}

#[test]
fn render_buffer_cell_access() {
    let mut buf = RenderBuffer::new(80, 24);
    if let Some(cell) = buf.cell_mut(5, 3) {
        cell.grapheme = GraphemeId::from_raw(88);
    }
    let cell = buf.cell(5, 3).unwrap();
    assert_eq!(cell.grapheme, GraphemeId::from_raw(88));
}

#[test]
fn encode_empty_diff() {
    let arena = GraphemeArena::new();
    let bytes = encode_diff(&[], &arena);
    assert!(bytes.is_empty());
}
