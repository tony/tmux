//! Scroll region management.
//!
//! Defines the scrollable region within the active screen (DECSTBM).
//! The scroll region is used by scroll up/down operations and line insertion/deletion.

/// A scroll region defined by top and bottom margins (0-based, inclusive).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollRegion {
    /// Top margin (0-based, inclusive).
    pub top: u16,
    /// Bottom margin (0-based, inclusive).
    pub bottom: u16,
}

impl ScrollRegion {
    /// Create a scroll region covering the full screen height.
    #[must_use]
    pub const fn full(screen_height: u16) -> Self {
        Self {
            top: 0,
            bottom: if screen_height > 0 {
                screen_height - 1
            } else {
                0
            },
        }
    }

    /// Create a custom scroll region.
    ///
    /// # Errors
    ///
    /// Returns `None` if top > bottom or bottom >= screen_height.
    #[must_use]
    pub const fn custom(top: u16, bottom: u16, screen_height: u16) -> Option<Self> {
        if top > bottom || bottom >= screen_height {
            None
        } else {
            Some(Self { top, bottom })
        }
    }

    /// Number of rows in this scroll region.
    #[must_use]
    pub const fn height(&self) -> u16 {
        self.bottom - self.top + 1
    }

    /// Whether a row index falls within this scroll region.
    #[must_use]
    pub const fn contains_row(&self, row: u16) -> bool {
        row >= self.top && row <= self.bottom
    }

    /// Whether this region covers the entire screen.
    #[must_use]
    pub const fn is_full_screen(&self, screen_height: u16) -> bool {
        self.top == 0 && self.bottom + 1 == screen_height
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_region() {
        let r = ScrollRegion::full(24);
        assert_eq!(r.top, 0);
        assert_eq!(r.bottom, 23);
        assert_eq!(r.height(), 24);
    }

    #[test]
    fn custom_region() {
        let r = ScrollRegion::custom(5, 15, 24);
        assert!(r.is_some());
        let r = r.unwrap_or(ScrollRegion::full(24));
        assert_eq!(r.height(), 11);
    }

    #[test]
    fn invalid_region_top_gt_bottom() {
        assert!(ScrollRegion::custom(10, 5, 24).is_none());
    }

    #[test]
    fn invalid_region_bottom_ge_height() {
        assert!(ScrollRegion::custom(0, 24, 24).is_none());
    }

    #[test]
    fn contains_row() {
        let r = ScrollRegion::full(24);
        assert!(r.contains_row(0));
        assert!(r.contains_row(23));
        assert!(!r.contains_row(24));
    }

    #[test]
    fn is_full_screen() {
        assert!(ScrollRegion::full(24).is_full_screen(24));
        let custom = ScrollRegion::custom(1, 23, 24);
        assert!(
            !custom
                .unwrap_or(ScrollRegion::full(24))
                .is_full_screen(24)
        );
    }

    #[test]
    fn full_region_zero_height() {
        let r = ScrollRegion::full(0);
        assert_eq!(r.top, 0);
        assert_eq!(r.bottom, 0);
    }
}
