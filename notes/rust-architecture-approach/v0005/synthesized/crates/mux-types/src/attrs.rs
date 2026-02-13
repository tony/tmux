//! Cell attributes and flags.
//!
//! CellFlags are bit-compatible with tmux's GRID_FLAG_* constants (INV-119).
//! Attrs represent SGR text attributes (bold, italic, etc.) as a bitfield.

use bitflags::bitflags;

bitflags! {
    /// Cell flags matching tmux's `GRID_FLAG_*` constants from tmux.h:742-749.
    ///
    /// INV-119: These values MUST NOT be changed without cross-referencing
    /// the tmux C source at `/home/d/study/c/tmux/tmux.h`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct CellFlags: u8 {
        /// Cell is a padding cell for a wide character (GRID_FLAG_PADDING = 0x04).
        const PADDING  = 0x04;
        /// Cell uses extended attributes (GRID_FLAG_EXTENDED = 0x08).
        const EXTENDED = 0x08;
        /// Cell is selected in copy mode (GRID_FLAG_SELECTED = 0x10).
        const SELECTED = 0x10;
        /// Cell has been explicitly cleared (GRID_FLAG_CLEARED = 0x40).
        const CLEARED  = 0x40;
        /// Cell is a tab character (GRID_FLAG_TAB = 0x80).
        const TAB      = 0x80;
    }
}

bitflags! {
    /// SGR text attributes applied to a cell.
    ///
    /// These map to the standard SGR parameters defined in ECMA-48.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Attrs: u16 {
        /// SGR 1: Bold or increased intensity.
        const BOLD          = 0x0001;
        /// SGR 2: Dim or decreased intensity.
        const DIM           = 0x0002;
        /// SGR 3: Italic.
        const ITALIC        = 0x0004;
        /// SGR 4: Underline.
        const UNDERSCORE    = 0x0008;
        /// SGR 5: Slow blink.
        const BLINK         = 0x0010;
        /// SGR 7: Reverse video.
        const REVERSE       = 0x0020;
        /// SGR 8: Hidden/invisible.
        const HIDDEN        = 0x0040;
        /// SGR 9: Strikethrough.
        const STRIKETHROUGH = 0x0080;
        /// SGR 21: Doubly underlined.
        const DOUBLE_UNDERSCORE = 0x0100;
        /// SGR 53: Overline.
        const OVERLINE      = 0x0200;
        /// Curly underline (non-standard, supported by many terminals).
        const CURLY_UNDERSCORE = 0x0400;
    }
}

impl Attrs {
    /// Pack into a u16 for compact storage.
    pub const fn pack(self) -> u16 {
        self.bits()
    }

    /// Unpack from a u16.
    pub fn unpack(bits: u16) -> Self {
        // Mask off any unknown bits for forward compatibility
        Self::from_bits_truncate(bits)
    }

    /// Returns true if no attributes are set.
    pub const fn is_empty_attrs(self) -> bool {
        self.bits() == 0
    }

    /// Returns the SGR parameter codes needed to enable these attributes.
    pub fn sgr_params(self) -> Vec<u8> {
        let mut params = Vec::new();
        if self.contains(Self::BOLD) {
            params.push(1);
        }
        if self.contains(Self::DIM) {
            params.push(2);
        }
        if self.contains(Self::ITALIC) {
            params.push(3);
        }
        if self.contains(Self::UNDERSCORE) {
            params.push(4);
        }
        if self.contains(Self::BLINK) {
            params.push(5);
        }
        if self.contains(Self::REVERSE) {
            params.push(7);
        }
        if self.contains(Self::HIDDEN) {
            params.push(8);
        }
        if self.contains(Self::STRIKETHROUGH) {
            params.push(9);
        }
        if self.contains(Self::DOUBLE_UNDERSCORE) {
            params.push(21);
        }
        if self.contains(Self::OVERLINE) {
            params.push(53);
        }
        params
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // INV-119 verification tests
    #[test]
    fn cell_flag_padding_matches_tmux() {
        assert_eq!(CellFlags::PADDING.bits(), 0x04);
    }

    #[test]
    fn cell_flag_extended_matches_tmux() {
        assert_eq!(CellFlags::EXTENDED.bits(), 0x08);
    }

    #[test]
    fn cell_flag_selected_matches_tmux() {
        assert_eq!(CellFlags::SELECTED.bits(), 0x10);
    }

    #[test]
    fn cell_flag_cleared_matches_tmux() {
        assert_eq!(CellFlags::CLEARED.bits(), 0x40);
    }

    #[test]
    fn cell_flag_tab_matches_tmux() {
        assert_eq!(CellFlags::TAB.bits(), 0x80);
    }

    #[test]
    fn attrs_bold() {
        let a = Attrs::BOLD;
        assert!(a.contains(Attrs::BOLD));
        assert!(!a.contains(Attrs::DIM));
    }

    #[test]
    fn attrs_combined() {
        let a = Attrs::BOLD | Attrs::ITALIC | Attrs::UNDERSCORE;
        assert!(a.contains(Attrs::BOLD));
        assert!(a.contains(Attrs::ITALIC));
        assert!(a.contains(Attrs::UNDERSCORE));
        assert!(!a.contains(Attrs::REVERSE));
    }

    #[test]
    fn attrs_pack_unpack_roundtrip() {
        let a = Attrs::BOLD | Attrs::STRIKETHROUGH | Attrs::OVERLINE;
        assert_eq!(Attrs::unpack(a.pack()), a);
    }

    #[test]
    fn attrs_empty() {
        let a = Attrs::empty();
        assert!(a.is_empty_attrs());
    }

    #[test]
    fn attrs_sgr_params_bold_italic() {
        let a = Attrs::BOLD | Attrs::ITALIC;
        let params = a.sgr_params();
        assert!(params.contains(&1));
        assert!(params.contains(&3));
        assert!(!params.contains(&7));
    }

    #[test]
    fn cell_flags_default_empty() {
        let f = CellFlags::default();
        assert!(f.is_empty());
    }

    #[test]
    fn cell_flags_combined() {
        let f = CellFlags::PADDING | CellFlags::TAB;
        assert!(f.contains(CellFlags::PADDING));
        assert!(f.contains(CellFlags::TAB));
        assert!(!f.contains(CellFlags::SELECTED));
    }

    #[test]
    fn attrs_unpack_unknown_bits_truncated() {
        // Bits beyond the known set should be stripped
        let packed = 0xFFFF;
        let a = Attrs::unpack(packed);
        // Should only contain known bits
        assert_eq!(a.pack() & 0xF800, 0);
    }

    mod prop {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn attrs_pack_unpack_roundtrip_prop(bits in 0u16..0x0800) {
                let a = Attrs::from_bits_truncate(bits);
                prop_assert_eq!(Attrs::unpack(a.pack()), a);
            }

            #[test]
            fn cell_flags_from_bits_roundtrip(bits in 0u8..=0xFF) {
                let f = CellFlags::from_bits_truncate(bits);
                let f2 = CellFlags::from_bits_truncate(f.bits());
                prop_assert_eq!(f, f2);
            }
        }
    }
}
