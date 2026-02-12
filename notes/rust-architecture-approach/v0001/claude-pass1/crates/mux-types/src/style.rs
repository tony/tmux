//! Terminal cell style.
//!
//! Styles are packed into a compact representation for cache efficiency.
//!
//! ## Invariants
//! - INV-009: Style is bitfield-packed.
//! - INV-004: All public types derive Debug.

/// Terminal color representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Color {
    /// Default terminal color.
    Default,
    /// Standard 16-color palette index (0-15) or 256-color index (0-255).
    Indexed(u8),
    /// 24-bit RGB color.
    Rgb(u8, u8, u8),
}

impl Color {
    /// Pack into a u32 for compact storage.
    /// Default = 0x0000_0000
    /// Indexed = 0x0100_00nn
    /// RGB     = 0x02RR_GGBB
    #[must_use]
    pub fn pack(self) -> u32 {
        match self {
            Color::Default => 0,
            Color::Indexed(n) => 0x0100_0000 | (n as u32),
            Color::Rgb(r, g, b) => 0x0200_0000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32),
        }
    }

    /// Unpack from the u32 representation.
    #[must_use]
    pub fn unpack(v: u32) -> Self {
        match v >> 24 {
            0 => Color::Default,
            1 => Color::Indexed(v as u8),
            2 => Color::Rgb((v >> 16) as u8, (v >> 8) as u8, v as u8),
            _ => Color::Default,
        }
    }
}

/// Text attribute flags, stored as a u16 bitfield.
/// INV-009: Style is bitfield-packed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Attrs(u16);

impl Attrs {
    pub const BOLD: u16       = 1 << 0;
    pub const DIM: u16        = 1 << 1;
    pub const ITALIC: u16     = 1 << 2;
    pub const UNDERLINE: u16  = 1 << 3;
    pub const BLINK: u16      = 1 << 4;
    pub const REVERSE: u16    = 1 << 5;
    pub const HIDDEN: u16     = 1 << 6;
    pub const STRIKETHROUGH: u16 = 1 << 7;

    #[must_use]
    pub fn new() -> Self { Self(0) }

    #[must_use]
    pub fn has(self, flag: u16) -> bool { self.0 & flag != 0 }

    pub fn set(&mut self, flag: u16) { self.0 |= flag; }

    pub fn clear(&mut self, flag: u16) { self.0 &= !flag; }

    #[must_use]
    pub fn bits(self) -> u16 { self.0 }

    #[must_use]
    pub fn from_bits(bits: u16) -> Self { Self(bits) }
}

/// The visual style of a terminal cell.
/// INV-004: Derives Debug.
/// INV-009: Uses packed bitfield for attributes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Style {
    pub fg: Color,
    pub bg: Color,
    pub attrs: Attrs,
}

impl Style {
    /// Create a new style with specific colors and no attributes.
    #[must_use]
    pub fn new(fg: Color, bg: Color) -> Self {
        Self { fg, bg, attrs: Attrs::new() }
    }

    /// Pack the style into a u15 index for PackedCell storage.
    /// This is a simplified hash-based approach; a production implementation
    /// would use a style palette with interning.
    #[must_use]
    pub fn pack_index(&self) -> u16 {
        let attr_part = (self.attrs.bits() & 0xFF) as u16;
        let fg_hint = match self.fg {
            Color::Default => 0u16,
            Color::Indexed(n) => n as u16 & 0x0F,
            Color::Rgb(r, _, _) => (r as u16 >> 4) & 0x0F,
        };
        let bg_hint = match self.bg {
            Color::Default => 0u16,
            Color::Indexed(n) => n as u16 & 0x0F,
            Color::Rgb(r, _, _) => (r as u16 >> 4) & 0x0F,
        };
        ((attr_part << 8) | (fg_hint << 4) | bg_hint) & 0x7FFF
    }
}

impl Default for Style {
    fn default() -> Self {
        Self {
            fg: Color::Default,
            bg: Color::Default,
            attrs: Attrs::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_style() {
        let s = Style::default();
        assert_eq!(s.fg, Color::Default);
        assert_eq!(s.bg, Color::Default);
        assert_eq!(s.attrs.bits(), 0);
    }

    #[test]
    fn test_color_pack_round_trip_default() {
        assert_eq!(Color::unpack(Color::Default.pack()), Color::Default);
    }

    #[test]
    fn test_color_pack_round_trip_indexed() {
        for n in [0u8, 7, 15, 128, 255] {
            let c = Color::Indexed(n);
            assert_eq!(Color::unpack(c.pack()), c);
        }
    }

    #[test]
    fn test_color_pack_round_trip_rgb() {
        let c = Color::Rgb(0xFF, 0x80, 0x00);
        assert_eq!(Color::unpack(c.pack()), c);
    }

    #[test]
    fn test_color_pack_round_trip_rgb_black() {
        let c = Color::Rgb(0, 0, 0);
        assert_eq!(Color::unpack(c.pack()), c);
    }

    #[test]
    fn test_attrs_set_and_check() {
        let mut a = Attrs::new();
        a.set(Attrs::BOLD);
        a.set(Attrs::ITALIC);
        assert!(a.has(Attrs::BOLD));
        assert!(a.has(Attrs::ITALIC));
        assert!(!a.has(Attrs::UNDERLINE));
    }

    #[test]
    fn test_attrs_clear() {
        let mut a = Attrs::new();
        a.set(Attrs::BOLD);
        a.clear(Attrs::BOLD);
        assert!(!a.has(Attrs::BOLD));
    }

    #[test]
    fn test_attrs_from_bits_round_trip() {
        let mut a = Attrs::new();
        a.set(Attrs::BOLD);
        a.set(Attrs::STRIKETHROUGH);
        let bits = a.bits();
        let b = Attrs::from_bits(bits);
        assert_eq!(a, b);
    }

    #[test]
    fn test_style_pack_index_fits_15_bits() {
        let s = Style::new(Color::Rgb(255, 128, 0), Color::Indexed(3));
        let idx = s.pack_index();
        assert!(idx < (1 << 15), "style index must fit in 15 bits, got {idx}");
    }

    #[test]
    fn test_style_default_pack_index_is_zero() {
        let s = Style::default();
        assert_eq!(s.pack_index(), 0);
    }
}
