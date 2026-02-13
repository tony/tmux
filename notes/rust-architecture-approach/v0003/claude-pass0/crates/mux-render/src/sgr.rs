//! SGR (Select Graphic Rendition) encoding and optimization.
//!
//! Only emits attribute changes when the new cell differs from the current
//! tracked state, minimizing bytes written to the terminal.

use mux_types::{Attrs, Colour};

/// Tracks current SGR state and encodes changes.
#[derive(Debug)]
pub struct SgrEncoder {
    /// Current foreground colour.
    pub fg: Colour,
    /// Current background colour.
    pub bg: Colour,
    /// Current text attributes.
    pub attrs: Attrs,
}

impl SgrEncoder {
    /// Create a new encoder with default (reset) state.
    #[must_use]
    pub fn new() -> Self {
        Self {
            fg: Colour::Default,
            bg: Colour::Default,
            attrs: Attrs::empty(),
        }
    }

    /// Encode the SGR sequence needed to transition from current state to the
    /// new cell's attributes. Returns an empty string if no change is needed.
    #[must_use]
    pub fn encode_transition(&mut self, new_attrs: Attrs, new_fg: Colour, new_bg: Colour) -> String {
        if self.attrs == new_attrs && self.fg == new_fg && self.bg == new_bg {
            return String::new();
        }

        let mut params = Vec::new();

        // Check if we need a full reset
        let removed_attrs = self.attrs.difference(new_attrs);
        if !removed_attrs.is_empty() {
            // It's simpler to reset and re-set than to individually unset
            params.push("0".to_owned());
            self.attrs = Attrs::empty();
            self.fg = Colour::Default;
            self.bg = Colour::Default;
        }

        // Set new attributes
        let attrs_to_set = new_attrs.difference(self.attrs);
        if attrs_to_set.contains(Attrs::BOLD) {
            params.push("1".to_owned());
        }
        if attrs_to_set.contains(Attrs::DIM) {
            params.push("2".to_owned());
        }
        if attrs_to_set.contains(Attrs::ITALIC) {
            params.push("3".to_owned());
        }
        if attrs_to_set.contains(Attrs::UNDERSCORE) {
            params.push("4".to_owned());
        }
        if attrs_to_set.contains(Attrs::BLINK) {
            params.push("5".to_owned());
        }
        if attrs_to_set.contains(Attrs::REVERSE) {
            params.push("7".to_owned());
        }
        if attrs_to_set.contains(Attrs::HIDDEN) {
            params.push("8".to_owned());
        }
        if attrs_to_set.contains(Attrs::STRIKETHROUGH) {
            params.push("9".to_owned());
        }

        // Foreground colour
        if self.fg != new_fg {
            encode_colour(&mut params, new_fg, false);
        }

        // Background colour
        if self.bg != new_bg {
            encode_colour(&mut params, new_bg, true);
        }

        self.attrs = new_attrs;
        self.fg = new_fg;
        self.bg = new_bg;

        if params.is_empty() {
            return String::new();
        }

        format!("\x1b[{}m", params.join(";"))
    }

    /// Reset to default state.
    pub fn reset(&mut self) {
        self.fg = Colour::Default;
        self.bg = Colour::Default;
        self.attrs = Attrs::empty();
    }
}

impl Default for SgrEncoder {
    fn default() -> Self {
        Self::new()
    }
}

/// Encode a colour into SGR parameters.
fn encode_colour(params: &mut Vec<String>, colour: Colour, background: bool) {
    let offset = if background { 40 } else { 30 };
    match colour {
        Colour::Default => {
            params.push(format!("{}", offset + 9)); // 39 or 49
        }
        Colour::Indexed(n) if n < 8 => {
            params.push(format!("{}", offset + u16::from(n)));
        }
        Colour::Indexed(n) if n < 16 => {
            params.push(format!("{}", offset + 60 + u16::from(n - 8)));
        }
        Colour::Indexed(n) => {
            let base = if background { 48 } else { 38 };
            params.push(format!("{base}"));
            params.push("5".to_owned());
            params.push(format!("{n}"));
        }
        Colour::Rgb { r, g, b } => {
            let base = if background { 48 } else { 38 };
            params.push(format!("{base}"));
            params.push("2".to_owned());
            params.push(format!("{r}"));
            params.push(format!("{g}"));
            params.push(format!("{b}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_change_no_output() {
        let mut enc = SgrEncoder::new();
        let result = enc.encode_transition(Attrs::empty(), Colour::Default, Colour::Default);
        assert!(result.is_empty());
    }

    #[test]
    fn bold_on() {
        let mut enc = SgrEncoder::new();
        let result = enc.encode_transition(Attrs::BOLD, Colour::Default, Colour::Default);
        assert!(result.contains("1"));
    }

    #[test]
    fn fg_indexed_color() {
        let mut enc = SgrEncoder::new();
        let result = enc.encode_transition(Attrs::empty(), Colour::Indexed(1), Colour::Default);
        assert!(result.contains("31")); // red foreground
    }

    #[test]
    fn bg_rgb_color() {
        let mut enc = SgrEncoder::new();
        let result = enc.encode_transition(
            Attrs::empty(),
            Colour::Default,
            Colour::Rgb { r: 255, g: 0, b: 128 },
        );
        assert!(result.contains("48"));
        assert!(result.contains("2"));
        assert!(result.contains("255"));
    }

    #[test]
    fn reset_on_attr_removal() {
        let mut enc = SgrEncoder::new();
        enc.encode_transition(Attrs::BOLD, Colour::Default, Colour::Default);
        let result = enc.encode_transition(Attrs::empty(), Colour::Default, Colour::Default);
        assert!(result.contains("0")); // reset
    }

    #[test]
    fn subsequent_same_no_output() {
        let mut enc = SgrEncoder::new();
        enc.encode_transition(Attrs::BOLD, Colour::Indexed(1), Colour::Default);
        let result = enc.encode_transition(Attrs::BOLD, Colour::Indexed(1), Colour::Default);
        assert!(result.is_empty());
    }

    #[test]
    fn encoder_reset() {
        let mut enc = SgrEncoder::new();
        enc.encode_transition(Attrs::BOLD, Colour::Indexed(1), Colour::Indexed(2));
        enc.reset();
        assert_eq!(enc.attrs, Attrs::empty());
        assert_eq!(enc.fg, Colour::Default);
    }

    #[test]
    fn bright_color_encoding() {
        let mut enc = SgrEncoder::new();
        let result = enc.encode_transition(Attrs::empty(), Colour::Indexed(9), Colour::Default);
        // Bright red should use 90-97 range
        assert!(result.contains("91"));
    }
}
