//! Terminal size and geometry types.

/// Terminal size in columns and rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Size {
    pub cols: u32,
    pub rows: u32,
}

impl Size {
    /// Create a new size.
    #[must_use]
    pub const fn new(cols: u32, rows: u32) -> Self {
        Self { cols, rows }
    }

    /// Total cell count.
    #[must_use]
    pub const fn area(&self) -> u64 {
        self.cols as u64 * self.rows as u64
    }

    /// Whether this size fits the minimum pane dimensions (2x1).
    #[must_use]
    pub const fn meets_minimum(&self) -> bool {
        self.cols >= 2 && self.rows >= 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_constructor() {
        let s = Size::new(80, 24);
        assert_eq!(s.cols, 80);
        assert_eq!(s.rows, 24);
    }

    #[test]
    fn size_area() {
        let s = Size::new(80, 24);
        assert_eq!(s.area(), 1920);
    }

    #[test]
    fn size_meets_minimum() {
        assert!(Size::new(2, 1).meets_minimum());
        assert!(Size::new(80, 24).meets_minimum());
        assert!(!Size::new(1, 1).meets_minimum());
        assert!(!Size::new(2, 0).meets_minimum());
    }
}
