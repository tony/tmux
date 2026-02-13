//! RenderOutput: escape sequence builder with SGR/CUP optimization.

use crate::diff::CellUpdate;
use mux_grapheme_arena::GraphemeArena;
use mux_types::{Attrs, Colour};

/// The render output buffer with SGR/CUP optimization.
#[derive(Debug)]
pub struct RenderOutput {
    buf: Vec<u8>,
    cursor_x: u32,
    cursor_y: u32,
    current_attrs: Attrs,
    current_fg: Colour,
    current_bg: Colour,
}

impl RenderOutput {
    #[must_use]
    pub fn new() -> Self {
        Self {
            buf: Vec::with_capacity(64 * 1024),
            cursor_x: u32::MAX, // force first move
            cursor_y: u32::MAX,
            current_attrs: Attrs::empty(),
            current_fg: Colour::Default,
            current_bg: Colour::Default,
        }
    }

    /// Emit escape sequences for a list of cell updates.
    pub fn emit_updates(&mut self, updates: &[CellUpdate], arena: &GraphemeArena) {
        for update in updates {
            let need_move = update.x != self.cursor_x || update.y != self.cursor_y;
            if need_move {
                self.move_to(update.x, update.y);
            }
            self.set_attrs(update.cell.attrs, update.cell.fg, update.cell.bg);

            if update.cell.is_padding() {
                // Skip padding cells -- cursor auto-advances from wide char
            } else {
                let mut char_buf = String::new();
                arena.resolve_to_buf(update.cell.grapheme, &mut char_buf);
                if char_buf.is_empty() {
                    self.buf.push(b' ');
                    self.cursor_x += 1;
                } else {
                    self.buf.extend_from_slice(char_buf.as_bytes());
                    self.cursor_x += u32::from(update.cell.width.max(1));
                }
            }
        }
    }

    pub fn move_to(&mut self, x: u32, y: u32) {
        if self.cursor_x != x || self.cursor_y != y {
            let seq = format!("\x1b[{};{}H", y + 1, x + 1);
            self.buf.extend_from_slice(seq.as_bytes());
            self.cursor_x = x;
            self.cursor_y = y;
        }
    }

    pub fn set_attrs(&mut self, attrs: Attrs, fg: Colour, bg: Colour) {
        if attrs != self.current_attrs || fg != self.current_fg || bg != self.current_bg {
            self.buf.extend_from_slice(b"\x1b[0m");
            if attrs.contains(Attrs::BOLD) { self.buf.extend_from_slice(b"\x1b[1m"); }
            if attrs.contains(Attrs::DIM) { self.buf.extend_from_slice(b"\x1b[2m"); }
            if attrs.contains(Attrs::ITALIC) { self.buf.extend_from_slice(b"\x1b[3m"); }
            if attrs.contains(Attrs::UNDERSCORE) { self.buf.extend_from_slice(b"\x1b[4m"); }
            if attrs.contains(Attrs::REVERSE) { self.buf.extend_from_slice(b"\x1b[7m"); }
            if attrs.contains(Attrs::STRIKETHROUGH) { self.buf.extend_from_slice(b"\x1b[9m"); }
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

    pub fn hide_cursor(&mut self) { self.buf.extend_from_slice(b"\x1b[?25l"); }

    pub fn show_cursor(&mut self, x: u32, y: u32) {
        self.move_to(x, y);
        self.buf.extend_from_slice(b"\x1b[?25h");
    }

    pub fn reset_attrs(&mut self) {
        self.buf.extend_from_slice(b"\x1b[0m");
        self.current_attrs = Attrs::empty();
        self.current_fg = Colour::Default;
        self.current_bg = Colour::Default;
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8] { &self.buf }

    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> { self.buf }

    #[must_use]
    pub fn len(&self) -> usize { self.buf.len() }

    #[must_use]
    pub fn is_empty(&self) -> bool { self.buf.is_empty() }
}

impl Default for RenderOutput {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_output_cursor_movement() {
        let mut output = RenderOutput::new();
        output.move_to(5, 10);
        let s = std::str::from_utf8(output.as_bytes()).unwrap_or("");
        assert!(s.contains("\x1b[11;6H")); // 1-based
    }

    #[test]
    fn render_output_attrs() {
        let mut output = RenderOutput::new();
        output.set_attrs(Attrs::BOLD, Colour::Default, Colour::Default);
        let s = String::from_utf8_lossy(output.as_bytes());
        assert!(s.contains("\x1b[1m"));
    }

    #[test]
    fn render_output_cursor_elision() {
        let mut output = RenderOutput::new();
        let arena = GraphemeArena::new();
        let updates = vec![
            CellUpdate { x: 0, y: 0, cell: mux_types::Cell { grapheme: mux_grapheme_arena::GraphemeId::from_char('A'), ..mux_types::Cell::empty() } },
            CellUpdate { x: 1, y: 0, cell: mux_types::Cell { grapheme: mux_grapheme_arena::GraphemeId::from_char('B'), ..mux_types::Cell::empty() } },
        ];
        output.emit_updates(&updates, &arena);
        let s = String::from_utf8_lossy(output.as_bytes());
        assert!(s.contains('A'));
        assert!(s.contains('B'));
    }
}
