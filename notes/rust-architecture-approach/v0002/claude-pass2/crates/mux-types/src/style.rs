//! Terminal cell style: colours and text attributes.
//!
//! ## Design
//! - [`Colour`] supports Default, Indexed (0-255), and 24-bit RGB.
//! - [`Attrs`] is a bitflags type covering all standard terminal attributes
//!   including the extended underline styles (double, curly, dotted, dashed).

use bitflags::bitflags;

/// Terminal colour representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Colour {
    /// Default terminal foreground/background.
    Default,
    /// 256-colour palette index (0-255).
    Indexed(u8),
    /// True-colour RGB.
    Rgb { r: u8, g: u8, b: u8 },
}

impl Colour {
    /// Pack colour into a u32 for compact storage.
    /// Default = 0x0000_0000, Indexed = 0x0100_00nn, RGB = 0x02RR_GGBB.
    #[must_use]
    pub const fn pack(self) -> u32 {
        match self {
            Self::Default => 0,
            Self::Indexed(n) => 0x0100_0000 | (n as u32),
            Self::Rgb { r, g, b } => 0x0200_0000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32),
        }
    }

    /// Unpack from the u32 representation.
    #[must_use]
    pub const fn unpack(v: u32) -> Self {
        match v >> 24 {
            1 => Self::Indexed(v as u8),
            2 => Self::Rgb { r: (v >> 16) as u8, g: (v >> 8) as u8, b: v as u8 },
            _ => Self::Default,
        }
    }
}

impl Default for Colour {
    fn default() -> Self {
        Self::Default
    }
}

bitflags! {
    /// Text attributes for terminal cells.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Attrs: u16 {
        const BOLD          = 0x0001;
        const DIM           = 0x0002;
        const ITALIC        = 0x0004;
        const UNDERSCORE    = 0x0008;
        const BLINK         = 0x0010;
        const REVERSE       = 0x0020;
        const HIDDEN        = 0x0040;
        const STRIKETHROUGH = 0x0080;
        const DOUBLE_UNDER  = 0x0100;
        const CURLY_UNDER   = 0x0200;
        const DOTTED_UNDER  = 0x0400;
        const DASHED_UNDER  = 0x0800;
        const OVERLINE      = 0x1000;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colour_default() {
        assert_eq!(Colour::default(), Colour::Default);
    }

    #[test]
    fn colour_pack_roundtrip_default() {
        assert_eq!(Colour::unpack(Colour::Default.pack()), Colour::Default);
    }

    #[test]
    fn colour_pack_roundtrip_indexed() {
        for n in [0u8, 7, 15, 128, 255] {
            let c = Colour::Indexed(n);
            assert_eq!(Colour::unpack(c.pack()), c);
        }
    }

    #[test]
    fn colour_pack_roundtrip_rgb() {
        let c = Colour::Rgb { r: 0xFF, g: 0x80, b: 0x00 };
        assert_eq!(Colour::unpack(c.pack()), c);
    }

    #[test]
    fn colour_pack_roundtrip_rgb_black() {
        let c = Colour::Rgb { r: 0, g: 0, b: 0 };
        assert_eq!(Colour::unpack(c.pack()), c);
    }

    #[test]
    fn attrs_default_empty() {
        assert_eq!(Attrs::default(), Attrs::empty());
    }

    #[test]
    fn attrs_bold_italic_combine() {
        let a = Attrs::BOLD | Attrs::ITALIC;
        assert!(a.contains(Attrs::BOLD));
        assert!(a.contains(Attrs::ITALIC));
        assert!(!a.contains(Attrs::DIM));
    }

    #[test]
    fn attrs_underline_variants() {
        // Ensure all underline variants have distinct bit values
        let variants = [
            Attrs::UNDERSCORE, Attrs::DOUBLE_UNDER,
            Attrs::CURLY_UNDER, Attrs::DOTTED_UNDER, Attrs::DASHED_UNDER,
        ];
        for (i, a) in variants.iter().enumerate() {
            for (j, b) in variants.iter().enumerate() {
                if i != j {
                    assert_ne!(a.bits(), b.bits());
                }
            }
        }
    }
}
