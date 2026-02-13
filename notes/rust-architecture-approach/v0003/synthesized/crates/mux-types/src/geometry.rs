//! Size, Rect, and Position types for terminal geometry.

/// Terminal dimensions (columns x rows).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Size {
    /// Number of columns.
    pub cols: u16,
    /// Number of rows.
    pub rows: u16,
}

impl Size {
    /// Minimum pane size: 2 columns x 1 row.
    pub const MIN_PANE: Self = Self { cols: 2, rows: 1 };

    /// Create a new size.
    #[must_use]
    pub const fn new(cols: u16, rows: u16) -> Self {
        Self { cols, rows }
    }

    /// Total cell count.
    #[must_use]
    pub const fn area(&self) -> u32 {
        self.cols as u32 * self.rows as u32
    }

    /// Check whether this size meets the minimum pane requirements.
    #[must_use]
    pub const fn meets_minimum(&self) -> bool {
        self.cols >= Self::MIN_PANE.cols && self.rows >= Self::MIN_PANE.rows
    }
}

/// A rectangle in terminal coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rect {
    /// Left column (0-based).
    pub x: u16,
    /// Top row (0-based).
    pub y: u16,
    /// Width in columns.
    pub width: u16,
    /// Height in rows.
    pub height: u16,
}

impl Rect {
    /// Create a new rectangle.
    #[must_use]
    pub const fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Right edge (exclusive).
    #[must_use]
    pub const fn right(&self) -> u16 {
        self.x + self.width
    }

    /// Bottom edge (exclusive).
    #[must_use]
    pub const fn bottom(&self) -> u16 {
        self.y + self.height
    }

    /// Whether a point is inside this rect.
    #[must_use]
    pub const fn contains(&self, col: u16, row: u16) -> bool {
        col >= self.x && col < self.right() && row >= self.y && row < self.bottom()
    }
}

/// A position in the terminal grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    /// Column (0-based).
    pub col: u16,
    /// Row (0-based).
    pub row: u16,
}

impl Position {
    /// Create a new position.
    #[must_use]
    pub const fn new(col: u16, row: u16) -> Self {
        Self { col, row }
    }

    /// The origin position (0, 0).
    pub const ORIGIN: Self = Self { col: 0, row: 0 };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_area() {
        assert_eq!(Size::new(80, 24).area(), 1920);
    }

    #[test]
    fn rect_contains() {
        let r = Rect::new(10, 5, 20, 10);
        assert!(r.contains(10, 5));
        assert!(r.contains(29, 14));
        assert!(!r.contains(30, 5));
        assert!(!r.contains(10, 15));
    }

    #[test]
    fn rect_edges() {
        let r = Rect::new(5, 3, 10, 8);
        assert_eq!(r.right(), 15);
        assert_eq!(r.bottom(), 11);
    }

    #[test]
    fn position_origin() {
        assert_eq!(Position::ORIGIN.col, 0);
        assert_eq!(Position::ORIGIN.row, 0);
    }

    #[test]
    fn size_equality() {
        assert_eq!(Size::new(80, 24), Size::new(80, 24));
        assert_ne!(Size::new(80, 24), Size::new(120, 40));
    }

    #[test]
    fn rect_zero_size() {
        let r = Rect::new(0, 0, 0, 0);
        assert_eq!(r.width, 0);
        assert!(!r.contains(0, 0));
    }

    #[test]
    fn size_min_pane() {
        let min = Size::MIN_PANE;
        assert!(min.cols >= 2);
        assert!(min.rows >= 1);
    }
}
