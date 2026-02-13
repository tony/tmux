//! Scroll region management (DECSTBM).

/// Scroll region defining the top and bottom boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollRegion {
    /// Top row of the scroll region (0-based, inclusive).
    pub top: u16,
    /// Bottom row of the scroll region (0-based, inclusive).
    pub bottom: u16,
}

impl ScrollRegion {
    /// Full-screen scroll region.
    #[must_use]
    pub const fn full(sy: u16) -> Self {
        Self {
            top: 0,
            bottom: sy.saturating_sub(1),
        }
    }

    /// Custom scroll region with validation.
    /// Returns `None` if the region is invalid.
    #[must_use]
    pub const fn custom(top: u16, bottom: u16, sy: u16) -> Option<Self> {
        if top < bottom && bottom < sy {
            Some(Self { top, bottom })
        } else {
            None
        }
    }

    /// Whether this region covers the full screen.
    #[must_use]
    pub const fn is_full_screen(&self, sy: u16) -> bool {
        self.top == 0 && self.bottom == sy.saturating_sub(1)
    }

    /// Height of the scroll region in rows.
    #[must_use]
    pub const fn height(&self) -> u16 {
        self.bottom - self.top + 1
    }

    /// Whether a given row is inside this scroll region.
    #[must_use]
    pub const fn contains(&self, row: u16) -> bool {
        row >= self.top && row <= self.bottom
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
        assert!(r.is_full_screen(24));
    }

    #[test]
    fn custom_region() {
        let r = ScrollRegion::custom(5, 15, 24);
        assert!(r.is_some());
        let r = r.unwrap_or_else(|| ScrollRegion::full(24));
        assert_eq!(r.top, 5);
        assert_eq!(r.bottom, 15);
        assert!(!r.is_full_screen(24));
    }

    #[test]
    fn invalid_region_top_ge_bottom() {
        assert!(ScrollRegion::custom(10, 5, 24).is_none());
    }

    #[test]
    fn invalid_region_bottom_ge_sy() {
        assert!(ScrollRegion::custom(0, 24, 24).is_none());
    }

    #[test]
    fn region_height() {
        let r = ScrollRegion::full(24);
        assert_eq!(r.height(), 24);
        let r = ScrollRegion::custom(5, 15, 24).unwrap_or_else(|| ScrollRegion::full(24));
        assert_eq!(r.height(), 11);
    }

    #[test]
    fn region_contains_top_and_bottom() {
        let r = ScrollRegion::custom(3, 10, 24).unwrap_or_else(|| ScrollRegion::full(24));
        assert!(r.contains(3));
        assert!(r.contains(10));
        assert!(!r.contains(2));
        assert!(!r.contains(11));
    }

    #[test]
    fn adjacent_line_region() {
        // Smallest valid custom region: two adjacent rows
        let r = ScrollRegion::custom(5, 6, 24);
        assert!(r.is_some());
        let r = r.unwrap_or_else(|| ScrollRegion::full(24));
        assert_eq!(r.height(), 2);
    }
}
