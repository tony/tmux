//! Text attribute bitflags (bold, italic, underline variants, etc.).

use bitflags::bitflags;

bitflags! {
    /// Text rendering attributes matching tmux's GRID_ATTR_* values.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Attrs: u16 {
        const BOLD          = 0x0001;
        const DIM           = 0x0002;
        const ITALIC        = 0x0004;
        const UNDERSCORE    = 0x0008;
        const BLINK         = 0x0010;
        const REVERSE       = 0x0020;
        const HIDDEN        = 0x0040;
        const STRIKETHROUGH = 0x0080;
        const OVERLINE      = 0x0100;
        const CURLY_UNDERLINE = 0x0200;
        const DOTTED_UNDERLINE = 0x0400;
        const DASHED_UNDERLINE = 0x0800;
    }
}

impl Default for Attrs {
    fn default() -> Self {
        Self::empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_empty() {
        assert!(Attrs::default().is_empty());
    }

    #[test]
    fn bold_flag() {
        let a = Attrs::BOLD;
        assert!(a.contains(Attrs::BOLD));
        assert!(!a.contains(Attrs::ITALIC));
    }

    #[test]
    fn combined_flags() {
        let a = Attrs::BOLD | Attrs::ITALIC | Attrs::UNDERSCORE;
        assert!(a.contains(Attrs::BOLD));
        assert!(a.contains(Attrs::ITALIC));
        assert!(a.contains(Attrs::UNDERSCORE));
        assert!(!a.contains(Attrs::DIM));
    }

    #[test]
    fn flag_values_are_powers_of_two() {
        // Verify each flag is a distinct bit
        assert_eq!(Attrs::BOLD.bits(), 0x0001);
        assert_eq!(Attrs::DIM.bits(), 0x0002);
        assert_eq!(Attrs::ITALIC.bits(), 0x0004);
        assert_eq!(Attrs::UNDERSCORE.bits(), 0x0008);
        assert_eq!(Attrs::BLINK.bits(), 0x0010);
        assert_eq!(Attrs::REVERSE.bits(), 0x0020);
        assert_eq!(Attrs::HIDDEN.bits(), 0x0040);
        assert_eq!(Attrs::STRIKETHROUGH.bits(), 0x0080);
    }

    #[test]
    fn remove_flag() {
        let mut a = Attrs::BOLD | Attrs::ITALIC;
        a.remove(Attrs::BOLD);
        assert!(!a.contains(Attrs::BOLD));
        assert!(a.contains(Attrs::ITALIC));
    }
}
