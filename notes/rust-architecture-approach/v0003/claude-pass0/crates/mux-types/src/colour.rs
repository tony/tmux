//! Terminal colour representation.
//!
//! Supports Default, Indexed (0-255), and 24-bit RGB. Pack/unpack provides
//! compact u32 storage for grid cells.

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
            Self::Rgb { r, g, b } => {
                0x0200_0000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
            }
        }
    }

    /// Unpack from the u32 representation.
    #[must_use]
    pub const fn unpack(v: u32) -> Self {
        match v >> 24 {
            1 => Self::Indexed(v as u8),
            2 => Self::Rgb {
                r: (v >> 16) as u8,
                g: (v >> 8) as u8,
                b: v as u8,
            },
            _ => Self::Default,
        }
    }

    /// Whether this is the default colour.
    #[must_use]
    pub const fn is_default(&self) -> bool {
        matches!(self, Self::Default)
    }

    /// Convert indexed colour 0-7 to its bright variant (8-15).
    #[must_use]
    pub const fn to_bright(self) -> Self {
        match self {
            Self::Indexed(n) if n < 8 => Self::Indexed(n + 8),
            other => other,
        }
    }
}

impl Default for Colour {
    fn default() -> Self {
        Self::Default
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colour_default() {
        assert_eq!(Colour::default(), Colour::Default);
        assert!(Colour::Default.is_default());
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
        let c = Colour::Rgb {
            r: 0xFF,
            g: 0x80,
            b: 0x00,
        };
        assert_eq!(Colour::unpack(c.pack()), c);
    }

    #[test]
    fn colour_pack_roundtrip_rgb_black() {
        let c = Colour::Rgb { r: 0, g: 0, b: 0 };
        assert_eq!(Colour::unpack(c.pack()), c);
    }

    #[test]
    fn colour_pack_roundtrip_rgb_white() {
        let c = Colour::Rgb {
            r: 255,
            g: 255,
            b: 255,
        };
        assert_eq!(Colour::unpack(c.pack()), c);
    }

    #[test]
    fn colour_to_bright() {
        assert_eq!(Colour::Indexed(0).to_bright(), Colour::Indexed(8));
        assert_eq!(Colour::Indexed(7).to_bright(), Colour::Indexed(15));
        // Already bright: no change
        assert_eq!(Colour::Indexed(8).to_bright(), Colour::Indexed(8));
        // Non-indexed: no change
        assert_eq!(Colour::Default.to_bright(), Colour::Default);
    }

    #[test]
    fn colour_is_not_default() {
        assert!(!Colour::Indexed(0).is_default());
        assert!(!Colour::Rgb { r: 0, g: 0, b: 0 }.is_default());
    }
}
