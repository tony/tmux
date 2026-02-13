//! Text attributes for terminal cells.
//!
//! Bitflags type covering all standard terminal attributes including
//! the extended underline styles (double, curly, dotted, dashed).

use bitflags::bitflags;

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

impl Attrs {
    /// Returns true if any underline variant is set.
    #[must_use]
    pub const fn has_any_underline(&self) -> bool {
        self.intersects(
            Self::UNDERSCORE
                .union(Self::DOUBLE_UNDER)
                .union(Self::CURLY_UNDER)
                .union(Self::DOTTED_UNDER)
                .union(Self::DASHED_UNDER),
        )
    }

    /// Clear all underline variants.
    #[must_use]
    pub const fn clear_underlines(self) -> Self {
        self.difference(
            Self::UNDERSCORE
                .union(Self::DOUBLE_UNDER)
                .union(Self::CURLY_UNDER)
                .union(Self::DOTTED_UNDER)
                .union(Self::DASHED_UNDER),
        )
    }

    /// The SGR parameter number for this attribute set's underline style.
    /// Returns 0 if no underline is set.
    #[must_use]
    pub const fn underline_sgr_param(&self) -> u8 {
        if self.contains(Self::CURLY_UNDER) {
            3 // CSI 4:3 m
        } else if self.contains(Self::DOUBLE_UNDER) {
            2 // CSI 4:2 m
        } else if self.contains(Self::DOTTED_UNDER) {
            4 // CSI 4:4 m
        } else if self.contains(Self::DASHED_UNDER) {
            5 // CSI 4:5 m
        } else if self.contains(Self::UNDERSCORE) {
            1 // CSI 4 m
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn attrs_underline_variants_distinct() {
        let variants = [
            Attrs::UNDERSCORE,
            Attrs::DOUBLE_UNDER,
            Attrs::CURLY_UNDER,
            Attrs::DOTTED_UNDER,
            Attrs::DASHED_UNDER,
        ];
        for (i, a) in variants.iter().enumerate() {
            for (j, b) in variants.iter().enumerate() {
                if i != j {
                    assert_ne!(a.bits(), b.bits());
                }
            }
        }
    }

    #[test]
    fn attrs_has_any_underline() {
        assert!(Attrs::UNDERSCORE.has_any_underline());
        assert!(Attrs::CURLY_UNDER.has_any_underline());
        assert!(!Attrs::BOLD.has_any_underline());
        assert!(!Attrs::empty().has_any_underline());
    }

    #[test]
    fn attrs_clear_underlines() {
        let a = Attrs::BOLD | Attrs::UNDERSCORE | Attrs::CURLY_UNDER;
        let cleared = a.clear_underlines();
        assert_eq!(cleared, Attrs::BOLD);
    }

    #[test]
    fn attrs_underline_sgr_params() {
        assert_eq!(Attrs::UNDERSCORE.underline_sgr_param(), 1);
        assert_eq!(Attrs::DOUBLE_UNDER.underline_sgr_param(), 2);
        assert_eq!(Attrs::CURLY_UNDER.underline_sgr_param(), 3);
        assert_eq!(Attrs::DOTTED_UNDER.underline_sgr_param(), 4);
        assert_eq!(Attrs::DASHED_UNDER.underline_sgr_param(), 5);
        assert_eq!(Attrs::empty().underline_sgr_param(), 0);
        assert_eq!(Attrs::BOLD.underline_sgr_param(), 0);
    }

    #[test]
    fn attrs_all_bits_non_overlapping() {
        let all = Attrs::all();
        assert_eq!(all.bits(), 0x1FFF);
    }
}
