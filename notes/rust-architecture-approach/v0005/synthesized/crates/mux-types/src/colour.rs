//! Colour representation matching tmux's colour model.
//!
//! Supports default, 256-colour palette, and 24-bit true colour.
//! Pack/unpack roundtrip is verified by proptest.

/// Terminal colour value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Colour {
    /// Terminal default colour.
    #[default]
    Default,
    /// 256-colour palette index (0-255).
    Indexed(u8),
    /// 24-bit true colour.
    Rgb { r: u8, g: u8, b: u8 },
}

impl Colour {
    /// Pack colour into a u32 for compact storage.
    ///
    /// Layout: `[tag:8][r_or_idx:8][g:8][b:8]`
    /// - tag 0 = Default
    /// - tag 1 = Indexed (index in r_or_idx)
    /// - tag 2 = Rgb
    pub const fn pack(self) -> u32 {
        match self {
            Self::Default => 0,
            Self::Indexed(idx) => 0x0100_0000 | (idx as u32) << 16,
            Self::Rgb { r, g, b } => {
                0x0200_0000 | (r as u32) << 16 | (g as u32) << 8 | b as u32
            }
        }
    }

    /// Unpack a u32 back into a Colour.
    pub const fn unpack(v: u32) -> Self {
        let tag = (v >> 24) & 0xFF;
        match tag {
            1 => Self::Indexed(((v >> 16) & 0xFF) as u8),
            2 => Self::Rgb {
                r: ((v >> 16) & 0xFF) as u8,
                g: ((v >> 8) & 0xFF) as u8,
                b: (v & 0xFF) as u8,
            },
            _ => Self::Default,
        }
    }

    /// Returns true if this is the default colour.
    pub const fn is_default(self) -> bool {
        matches!(self, Self::Default)
    }

    /// Convert a basic ANSI colour number (0-7) to an indexed colour.
    pub const fn from_ansi(n: u8) -> Self {
        Self::Indexed(n)
    }

    /// Convert a bright ANSI colour number (0-7) to indexed (8-15).
    pub const fn from_ansi_bright(n: u8) -> Self {
        Self::Indexed(n + 8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_colour_pack_unpack() {
        assert_eq!(Colour::unpack(Colour::Default.pack()), Colour::Default);
    }

    #[test]
    fn indexed_colour_pack_unpack() {
        for i in 0..=255u8 {
            let c = Colour::Indexed(i);
            assert_eq!(Colour::unpack(c.pack()), c);
        }
    }

    #[test]
    fn rgb_colour_pack_unpack() {
        let c = Colour::Rgb {
            r: 0xAB,
            g: 0xCD,
            b: 0xEF,
        };
        assert_eq!(Colour::unpack(c.pack()), c);
    }

    #[test]
    fn is_default() {
        assert!(Colour::Default.is_default());
        assert!(!Colour::Indexed(1).is_default());
    }

    #[test]
    fn from_ansi() {
        assert_eq!(Colour::from_ansi(3), Colour::Indexed(3));
    }

    #[test]
    fn from_ansi_bright() {
        assert_eq!(Colour::from_ansi_bright(3), Colour::Indexed(11));
    }

    mod prop {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn rgb_roundtrip(r in 0u8..=255, g in 0u8..=255, b in 0u8..=255) {
                let c = Colour::Rgb { r, g, b };
                prop_assert_eq!(Colour::unpack(c.pack()), c);
            }

            #[test]
            fn indexed_roundtrip(idx in 0u8..=255) {
                let c = Colour::Indexed(idx);
                prop_assert_eq!(Colour::unpack(c.pack()), c);
            }
        }
    }
}
