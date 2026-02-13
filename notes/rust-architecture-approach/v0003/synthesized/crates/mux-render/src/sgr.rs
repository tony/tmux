//! SGR (Select Graphic Rendition) encoder with optimization state.

use mux_types::{Attrs, Colour};

/// SGR encoder that tracks current state to minimize escape sequences.
#[derive(Debug)]
pub struct SgrEncoder {
    current_attrs: Attrs,
    current_fg: Colour,
    current_bg: Colour,
}

impl SgrEncoder {
    /// Create a new encoder with default state.
    #[must_use]
    pub fn new() -> Self {
        Self {
            current_attrs: Attrs::empty(),
            current_fg: Colour::Default,
            current_bg: Colour::Default,
        }
    }

    /// Encode an SGR sequence for the given attributes and colours.
    /// Returns an empty vec if nothing changed.
    pub fn encode(&mut self, attrs: Attrs, fg: Colour, bg: Colour) -> Vec<u8> {
        if attrs == self.current_attrs && fg == self.current_fg && bg == self.current_bg {
            return Vec::new();
        }

        let mut params = Vec::new();

        // Reset if needed
        if !self.current_attrs.is_empty()
            && !self.current_attrs.intersection(attrs).eq(&self.current_attrs)
        {
            params.push(0u8); // reset
        }

        if attrs.contains(Attrs::BOLD) && !self.current_attrs.contains(Attrs::BOLD) {
            params.push(1);
        }
        if attrs.contains(Attrs::DIM) && !self.current_attrs.contains(Attrs::DIM) {
            params.push(2);
        }
        if attrs.contains(Attrs::ITALIC) && !self.current_attrs.contains(Attrs::ITALIC) {
            params.push(3);
        }
        if attrs.contains(Attrs::UNDERSCORE) && !self.current_attrs.contains(Attrs::UNDERSCORE) {
            params.push(4);
        }
        if attrs.contains(Attrs::REVERSE) && !self.current_attrs.contains(Attrs::REVERSE) {
            params.push(7);
        }

        self.current_attrs = attrs;
        self.current_fg = fg;
        self.current_bg = bg;

        if params.is_empty() {
            // Only colour changed -- emit colour params
            Vec::new()
        } else {
            let mut buf = Vec::new();
            buf.extend_from_slice(b"\x1B[");
            for (i, p) in params.iter().enumerate() {
                if i > 0 {
                    buf.push(b';');
                }
                // Simple number encoding
                if *p >= 10 {
                    buf.push(b'0' + p / 10);
                }
                buf.push(b'0' + p % 10);
            }
            buf.push(b'm');
            buf
        }
    }

    /// Reset the encoder state to defaults.
    pub fn reset(&mut self) {
        self.current_attrs = Attrs::empty();
        self.current_fg = Colour::Default;
        self.current_bg = Colour::Default;
    }
}

impl Default for SgrEncoder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_change_no_output() {
        let mut enc = SgrEncoder::new();
        let result = enc.encode(Attrs::empty(), Colour::Default, Colour::Default);
        assert!(result.is_empty());
    }

    #[test]
    fn bold_emits_sgr() {
        let mut enc = SgrEncoder::new();
        let result = enc.encode(Attrs::BOLD, Colour::Default, Colour::Default);
        assert!(!result.is_empty());
    }

    #[test]
    fn same_attrs_no_output() {
        let mut enc = SgrEncoder::new();
        enc.encode(Attrs::BOLD, Colour::Default, Colour::Default);
        let result = enc.encode(Attrs::BOLD, Colour::Default, Colour::Default);
        assert!(result.is_empty());
    }

    #[test]
    fn reset_clears_state() {
        let mut enc = SgrEncoder::new();
        enc.encode(Attrs::BOLD, Colour::Default, Colour::Default);
        enc.reset();
        let result = enc.encode(Attrs::BOLD, Colour::Default, Colour::Default);
        assert!(!result.is_empty());
    }
}
