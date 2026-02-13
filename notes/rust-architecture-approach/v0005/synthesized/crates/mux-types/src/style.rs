//! Style combining attributes and colours for rendering.

use crate::attrs::Attrs;
use crate::colour::Colour;

/// Combined style with text attributes and colours.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Style {
    /// Text attributes.
    pub attrs: Attrs,
    /// Foreground colour.
    pub fg: Colour,
    /// Background colour.
    pub bg: Colour,
    /// Underline colour.
    pub us: Colour,
}

impl Style {
    /// Create a new default style.
    pub const fn new() -> Self {
        Self {
            attrs: Attrs::empty(),
            fg: Colour::Default,
            bg: Colour::Default,
            us: Colour::Default,
        }
    }

    /// Returns true if this style has no attributes and default colours.
    pub fn is_default(&self) -> bool {
        self.attrs.is_empty_attrs()
            && self.fg.is_default()
            && self.bg.is_default()
            && self.us.is_default()
    }

    /// Create a style with bold attribute.
    pub const fn bold() -> Self {
        Self {
            attrs: Attrs::BOLD,
            fg: Colour::Default,
            bg: Colour::Default,
            us: Colour::Default,
        }
    }

    /// Set the foreground colour.
    pub const fn with_fg(mut self, fg: Colour) -> Self {
        self.fg = fg;
        self
    }

    /// Set the background colour.
    pub const fn with_bg(mut self, bg: Colour) -> Self {
        self.bg = bg;
        self
    }

    /// Returns true if this style differs from another in a way that requires
    /// SGR reset (attributes were removed, not just added).
    pub fn needs_reset(&self, prev: &Self) -> bool {
        // If prev had attributes that self does not, we need a reset
        let removed = prev.attrs & !self.attrs;
        !removed.is_empty_attrs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_style() {
        let s = Style::new();
        assert!(s.is_default());
    }

    #[test]
    fn bold_style() {
        let s = Style::bold();
        assert!(!s.is_default());
        assert!(s.attrs.contains(Attrs::BOLD));
    }

    #[test]
    fn with_fg() {
        let s = Style::new().with_fg(Colour::Indexed(1));
        assert!(!s.is_default());
        assert_eq!(s.fg, Colour::Indexed(1));
    }

    #[test]
    fn with_bg() {
        let s = Style::new().with_bg(Colour::Rgb { r: 255, g: 0, b: 0 });
        assert_eq!(
            s.bg,
            Colour::Rgb {
                r: 255,
                g: 0,
                b: 0
            }
        );
    }

    #[test]
    fn needs_reset_when_attrs_removed() {
        let prev = Style {
            attrs: Attrs::BOLD | Attrs::ITALIC,
            ..Style::new()
        };
        let next = Style {
            attrs: Attrs::BOLD,
            ..Style::new()
        };
        assert!(next.needs_reset(&prev));
    }

    #[test]
    fn no_reset_when_attrs_added() {
        let prev = Style {
            attrs: Attrs::BOLD,
            ..Style::new()
        };
        let next = Style {
            attrs: Attrs::BOLD | Attrs::ITALIC,
            ..Style::new()
        };
        assert!(!next.needs_reset(&prev));
    }

    #[test]
    fn no_reset_when_same_attrs() {
        let s = Style::bold();
        assert!(!s.needs_reset(&s));
    }

    #[test]
    fn needs_reset_all_removed() {
        let prev = Style {
            attrs: Attrs::BOLD | Attrs::ITALIC | Attrs::UNDERSCORE,
            ..Style::new()
        };
        let next = Style::new();
        assert!(next.needs_reset(&prev));
    }

    #[test]
    fn default_is_default() {
        let s = Style::default();
        assert!(s.is_default());
    }

    #[test]
    fn style_eq() {
        let a = Style::bold().with_fg(Colour::Indexed(1));
        let b = Style::bold().with_fg(Colour::Indexed(1));
        assert_eq!(a, b);
    }

    #[test]
    fn style_ne_different_fg() {
        let a = Style::new().with_fg(Colour::Indexed(1));
        let b = Style::new().with_fg(Colour::Indexed(2));
        assert_ne!(a, b);
    }

    #[test]
    fn style_hash_consistent() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let s1 = Style::bold();
        let s2 = Style::bold();
        let mut h1 = DefaultHasher::new();
        let mut h2 = DefaultHasher::new();
        s1.hash(&mut h1);
        s2.hash(&mut h2);
        assert_eq!(h1.finish(), h2.finish());
    }

    #[test]
    fn chain_with_fg_bg() {
        let s = Style::bold()
            .with_fg(Colour::Indexed(1))
            .with_bg(Colour::Indexed(2));
        assert!(s.attrs.contains(Attrs::BOLD));
        assert_eq!(s.fg, Colour::Indexed(1));
        assert_eq!(s.bg, Colour::Indexed(2));
    }

    #[test]
    fn is_default_with_only_us() {
        let mut s = Style::new();
        s.us = Colour::Indexed(5);
        assert!(!s.is_default());
    }
}
