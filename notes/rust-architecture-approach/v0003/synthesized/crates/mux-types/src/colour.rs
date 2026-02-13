//! Colour representation with pack/unpack for compact 32-bit storage.
//!
//! Matches tmux's colour encoding: the top byte encodes the type,
//! the lower 24 bits encode the value (RGB or palette index).

/// Terminal colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Colour {
    /// Default terminal colour.
    #[default]
    Default,
    /// One of the 16 standard colours (0-15).
    Palette(u8),
    /// 256-colour palette (0-255).
    Palette256(u8),
    /// True colour RGB.
    Rgb { r: u8, g: u8, b: u8 },
}

impl Colour {
    /// Pack a colour into a 32-bit value for compact storage.
    /// Format: `[type:8][r_or_idx:8][g:8][b:8]`
    #[must_use]
    pub const fn pack(self) -> u32 {
        match self {
            Self::Default => 0,
            Self::Palette(idx) => 0x0100_0000 | idx as u32,
            Self::Palette256(idx) => 0x0200_0000 | idx as u32,
            Self::Rgb { r, g, b } => {
                0x0300_0000 | ((r as u32) << 16) | ((g as u32) << 8) | b as u32
            }
        }
    }

    /// Unpack a colour from a 32-bit packed value.
    #[must_use]
    pub const fn unpack(packed: u32) -> Self {
        let tag = (packed >> 24) & 0xFF;
        match tag {
            1 => Self::Palette((packed & 0xFF) as u8),
            2 => Self::Palette256((packed & 0xFF) as u8),
            3 => {
                let r = ((packed >> 16) & 0xFF) as u8;
                let g = ((packed >> 8) & 0xFF) as u8;
                let b = (packed & 0xFF) as u8;
                Self::Rgb { r, g, b }
            }
            // 0 and all unknown tags unpack to Default.
            _ => Self::Default,
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_pack_unpack() {
        let c = Colour::Default;
        assert_eq!(Colour::unpack(c.pack()), Colour::Default);
    }

    #[test]
    fn palette_pack_unpack() {
        let c = Colour::Palette(7);
        assert_eq!(Colour::unpack(c.pack()), c);
    }

    #[test]
    fn palette256_pack_unpack() {
        let c = Colour::Palette256(196);
        assert_eq!(Colour::unpack(c.pack()), c);
    }

    #[test]
    fn rgb_pack_unpack() {
        let c = Colour::Rgb {
            r: 255,
            g: 128,
            b: 0,
        };
        assert_eq!(Colour::unpack(c.pack()), c);
    }

    #[test]
    fn default_colour_packs_to_zero() {
        assert_eq!(Colour::Default.pack(), 0);
    }

    #[test]
    fn unknown_tag_unpacks_to_default() {
        assert_eq!(Colour::unpack(0xFF00_0000), Colour::Default);
    }

    #[test]
    fn all_palette_values_roundtrip() {
        for i in 0..=15u8 {
            let c = Colour::Palette(i);
            assert_eq!(Colour::unpack(c.pack()), c);
        }
    }

    #[test]
    fn rgb_black_and_white() {
        let black = Colour::Rgb { r: 0, g: 0, b: 0 };
        let white = Colour::Rgb {
            r: 255,
            g: 255,
            b: 255,
        };
        assert_eq!(Colour::unpack(black.pack()), black);
        assert_eq!(Colour::unpack(white.pack()), white);
        assert_ne!(black.pack(), white.pack());
    }

    #[test]
    fn default_colour_pack() {
        let c = Colour::Default;
        assert_eq!(Colour::unpack(c.pack()), c);
    }

    #[test]
    fn palette_colour_distinct() {
        let a = Colour::Palette(1);
        let b = Colour::Palette(7);
        assert_ne!(a.pack(), b.pack());
    }

    #[test]
    fn palette256_boundary() {
        let c0 = Colour::Palette256(0);
        let c255 = Colour::Palette256(255);
        assert_eq!(Colour::unpack(c0.pack()), c0);
        assert_eq!(Colour::unpack(c255.pack()), c255);
        assert_ne!(c0, c255);
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn colour_pack_unpack_roundtrip(r in 0u8..=255, g in 0u8..=255, b in 0u8..=255) {
            let c = Colour::Rgb { r, g, b };
            prop_assert_eq!(Colour::unpack(c.pack()), c);
        }

        #[test]
        fn palette256_roundtrip(idx in 0u8..=255) {
            let c = Colour::Palette256(idx);
            prop_assert_eq!(Colour::unpack(c.pack()), c);
        }
    }
}
