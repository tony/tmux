//! # mux-render
//!
//! Composition and rendering pipeline for terminal multiplexer output.
//!
//! This is NOT a GPU renderer. It composes multiple pane grids into a single
//! terminal output stream (escape sequences for cursor movement, style changes,
//! and character output).
//!
//! ## Render Pipeline (6,206 C LOC in tmux -> ~2,800 Rust LOC)
//! 1. Collect dirty panes from kernel snapshot
//! 2. Compute layout geometry (borders, status line)
//! 3. For each dirty cell, emit minimal escape sequences
//! 4. Optimize cursor movement (skip unchanged regions)
//! 5. Coalesce output into a single write buffer

#![forbid(unsafe_code)]

use mux_grapheme_arena::GraphemeArena;
use mux_grid::GridSnapshot;
use mux_types::{Attrs, Cell, CellFlags, Colour, Size};

/// A positioned pane for rendering.
#[derive(Debug, Clone)]
pub struct PaneGeometry {
    /// Pane position in the composite terminal (col, row).
    pub x: u32,
    pub y: u32,
    /// Pane dimensions.
    pub size: Size,
    /// Whether this pane is active.
    pub active: bool,
    /// Whether borders should be drawn around this pane.
    pub has_border: bool,
}

/// The render output buffer.
#[derive(Debug)]
pub struct RenderOutput {
    /// Accumulated escape sequences and text.
    buf: Vec<u8>,
    /// Current cursor position.
    cursor_x: u32,
    cursor_y: u32,
    /// Current attributes.
    current_attrs: Attrs,
    current_fg: Colour,
    current_bg: Colour,
}

impl RenderOutput {
    /// Create a new render output buffer.
    #[must_use]
    pub fn new() -> Self {
        Self {
            buf: Vec::with_capacity(64 * 1024),
            cursor_x: 0,
            cursor_y: 0,
            current_attrs: Attrs::empty(),
            current_fg: Colour::Default,
            current_bg: Colour::Default,
        }
    }

    /// Move cursor to absolute position.
    pub fn move_to(&mut self, x: u32, y: u32) {
        if self.cursor_x != x || self.cursor_y != y {
            // CSI row;col H (1-based)
            let seq = format!("\x1b[{};{}H", y + 1, x + 1);
            self.buf.extend_from_slice(seq.as_bytes());
            self.cursor_x = x;
            self.cursor_y = y;
        }
    }

    /// Set text attributes, emitting SGR sequences only for changes.
    pub fn set_attrs(&mut self, attrs: Attrs, fg: Colour, bg: Colour) {
        if attrs != self.current_attrs || fg != self.current_fg || bg != self.current_bg {
            // Reset first, then apply
            self.buf.extend_from_slice(b"\x1b[0m");
            self.current_attrs = Attrs::empty();
            self.current_fg = Colour::Default;
            self.current_bg = Colour::Default;

            // Apply attributes
            if attrs.contains(Attrs::BOLD) {
                self.buf.extend_from_slice(b"\x1b[1m");
            }
            if attrs.contains(Attrs::DIM) {
                self.buf.extend_from_slice(b"\x1b[2m");
            }
            if attrs.contains(Attrs::ITALIC) {
                self.buf.extend_from_slice(b"\x1b[3m");
            }
            if attrs.contains(Attrs::UNDERSCORE) {
                self.buf.extend_from_slice(b"\x1b[4m");
            }
            if attrs.contains(Attrs::REVERSE) {
                self.buf.extend_from_slice(b"\x1b[7m");
            }
            if attrs.contains(Attrs::STRIKETHROUGH) {
                self.buf.extend_from_slice(b"\x1b[9m");
            }

            self.emit_colour(fg, true);
            self.emit_colour(bg, false);

            self.current_attrs = attrs;
            self.current_fg = fg;
            self.current_bg = bg;
        }
    }

    fn emit_colour(&mut self, colour: Colour, is_fg: bool) {
        let prefix = if is_fg { 38 } else { 48 };
        match colour {
            Colour::Default => {}
            Colour::Indexed(idx) => {
                let seq = format!("\x1b[{prefix};5;{idx}m");
                self.buf.extend_from_slice(seq.as_bytes());
            }
            Colour::Rgb { r, g, b } => {
                let seq = format!("\x1b[{prefix};2;{r};{g};{b}m");
                self.buf.extend_from_slice(seq.as_bytes());
            }
        }
    }

    /// Write a character at the current position.
    pub fn put_char(&mut self, ch: char) {
        let mut buf = [0u8; 4];
        let encoded = ch.encode_utf8(&mut buf);
        self.buf.extend_from_slice(encoded.as_bytes());
        self.cursor_x += 1;
    }

    /// Write a string at the current position.
    pub fn put_str(&mut self, s: &str) {
        self.buf.extend_from_slice(s.as_bytes());
        self.cursor_x += s.len() as u32; // approximate
    }

    /// Clear the entire screen.
    pub fn clear_screen(&mut self) {
        self.buf.extend_from_slice(b"\x1b[2J");
    }

    /// Hide the cursor.
    pub fn hide_cursor(&mut self) {
        self.buf.extend_from_slice(b"\x1b[?25l");
    }

    /// Show the cursor.
    pub fn show_cursor(&mut self) {
        self.buf.extend_from_slice(b"\x1b[?25h");
    }

    /// Reset all attributes.
    pub fn reset_attrs(&mut self) {
        self.buf.extend_from_slice(b"\x1b[0m");
        self.current_attrs = Attrs::empty();
        self.current_fg = Colour::Default;
        self.current_bg = Colour::Default;
    }

    /// Get the accumulated output bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.buf
    }

    /// Take the output buffer.
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.buf
    }

    /// Current buffer size.
    #[must_use]
    pub fn len(&self) -> usize {
        self.buf.len()
    }

    /// Whether the buffer is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }
}

impl Default for RenderOutput {
    fn default() -> Self {
        Self::new()
    }
}

/// Draw a single pane's grid into the render output.
pub fn render_pane(
    output: &mut RenderOutput,
    snapshot: &GridSnapshot,
    arena: &GraphemeArena,
    geometry: &PaneGeometry,
) {
    for row in 0..geometry.size.rows {
        let grid_y = row;
        if let Some(line) = snapshot.chunks.iter().flat_map(|c| c.lines.iter()).nth(grid_y as usize) {
            // Only render dirty lines for efficiency
            if snapshot.dirty.get(grid_y as usize).as_deref().copied().unwrap_or(false) {
                output.move_to(geometry.x, geometry.y + row);

                for col in 0..geometry.size.cols {
                    if let Some(cell) = line.cell(col) {
                        if cell.is_padding() {
                            continue; // skip padding cells
                        }
                        output.set_attrs(cell.attrs, cell.fg, cell.bg);

                        let mut buf = String::new();
                        arena.resolve_to_buf(cell.grapheme, &mut buf);
                        if buf.is_empty() {
                            output.put_char(' ');
                        } else {
                            output.put_str(&buf);
                        }
                    } else {
                        output.put_char(' ');
                    }
                }
            }
        }
    }
}

/// Draw pane borders.
pub fn render_borders(
    output: &mut RenderOutput,
    panes: &[PaneGeometry],
    terminal_size: Size,
) {
    output.reset_attrs();
    for pane in panes {
        if !pane.has_border {
            continue;
        }
        // Draw horizontal borders
        if pane.y > 0 {
            output.move_to(pane.x, pane.y - 1);
            for _ in 0..pane.size.cols {
                output.put_char('\u{2500}'); // horizontal line
            }
        }
        // Draw vertical borders
        if pane.x > 0 {
            for row in 0..pane.size.rows {
                output.move_to(pane.x - 1, pane.y + row);
                output.put_char('\u{2502}'); // vertical line
            }
        }
    }
    let _ = terminal_size; // used for status line positioning
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_output_cursor_movement() {
        let mut output = RenderOutput::new();
        output.move_to(5, 10);
        let bytes = output.as_bytes();
        // Should contain CSI sequence for cursor positioning
        assert!(!bytes.is_empty());
        let s = std::str::from_utf8(bytes).unwrap_or("");
        assert!(s.contains("\x1b[11;6H")); // 1-based
    }

    #[test]
    fn render_output_attrs() {
        let mut output = RenderOutput::new();
        output.set_attrs(Attrs::BOLD, Colour::Default, Colour::Default);
        let s = String::from_utf8_lossy(output.as_bytes());
        assert!(s.contains("\x1b[1m")); // bold
    }

    #[test]
    fn render_output_put_char() {
        let mut output = RenderOutput::new();
        output.put_char('A');
        assert_eq!(output.as_bytes(), b"A");
    }
}
