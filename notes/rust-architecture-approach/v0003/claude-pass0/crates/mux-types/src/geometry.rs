//! Geometry types for terminal dimensions.
//!
//! Provides [`Size`], [`Rect`], and [`Position`] types used throughout
//! the layout engine and rendering pipeline.

/// Terminal dimensions (width x height).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Size {
    /// Number of columns.
    pub cols: u16,
    /// Number of rows.
    pub rows: u16,
}

impl Size {
    /// Minimum pane size: 2 cols x 1 row (layout invariant).
    pub const MIN_PANE: Self = Self { cols: 2, rows: 1 };

    /// Create a new size.
    #[must_use]
    pub const fn new(cols: u16, rows: u16) -> Self {
        Self { cols, rows }
    }

    /// Total number of cells (cols * rows).
    #[must_use]
    pub const fn area(&self) -> u32 {
        self.cols as u32 * self.rows as u32
    }

    /// Whether this size meets the minimum pane requirement.
    #[must_use]
    pub const fn meets_minimum(&self) -> bool {
        self.cols >= Self::MIN_PANE.cols && self.rows >= Self::MIN_PANE.rows
    }

    /// Clamp to at least the minimum pane size.
    #[must_use]
    pub const fn clamp_to_minimum(self) -> Self {
        Self {
            cols: if self.cols < Self::MIN_PANE.cols {
                Self::MIN_PANE.cols
            } else {
                self.cols
            },
            rows: if self.rows < Self::MIN_PANE.rows {
                Self::MIN_PANE.rows
            } else {
                self.rows
            },
        }
    }
}

/// A position on screen (column, row), 0-based.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    /// Column (0-based).
    pub x: u16,
    /// Row (0-based).
    pub y: u16,
}

impl Position {
    /// Origin (0, 0).
    pub const ORIGIN: Self = Self { x: 0, y: 0 };

    /// Create a new position.
    #[must_use]
    pub const fn new(x: u16, y: u16) -> Self {
        Self { x, y }
    }
}

/// A rectangle defined by position and size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rect {
    /// Top-left corner.
    pub origin: Position,
    /// Dimensions.
    pub size: Size,
}

impl Rect {
    /// Create a rect from position and size.
    #[must_use]
    pub const fn new(x: u16, y: u16, cols: u16, rows: u16) -> Self {
        Self {
            origin: Position::new(x, y),
            size: Size::new(cols, rows),
        }
    }

    /// Right edge (exclusive).
    #[must_use]
    pub const fn right(&self) -> u16 {
        self.origin.x + self.size.cols
    }

    /// Bottom edge (exclusive).
    #[must_use]
    pub const fn bottom(&self) -> u16 {
        self.origin.y + self.size.rows
    }

    /// Whether a position is inside this rectangle.
    #[must_use]
    pub const fn contains(&self, pos: Position) -> bool {
        pos.x >= self.origin.x
            && pos.x < self.right()
            && pos.y >= self.origin.y
            && pos.y < self.bottom()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn size_clamp_to_minimum() {
        assert_eq!(Size::new(0, 0).clamp_to_minimum(), Size::new(2, 1));
        assert_eq!(Size::new(1, 0).clamp_to_minimum(), Size::new(2, 1));
        assert_eq!(Size::new(80, 24).clamp_to_minimum(), Size::new(80, 24));
    }

    #[test]
    fn rect_contains() {
        let r = Rect::new(10, 5, 20, 10);
        assert!(r.contains(Position::new(10, 5)));
        assert!(r.contains(Position::new(29, 14)));
        assert!(!r.contains(Position::new(30, 5)));
        assert!(!r.contains(Position::new(10, 15)));
        assert!(!r.contains(Position::new(9, 5)));
    }

    #[test]
    fn rect_edges() {
        let r = Rect::new(5, 10, 20, 15);
        assert_eq!(r.right(), 25);
        assert_eq!(r.bottom(), 25);
    }

    #[test]
    fn position_origin() {
        assert_eq!(Position::ORIGIN, Position::new(0, 0));
    }

    #[test]
    fn size_min_pane() {
        assert_eq!(Size::MIN_PANE.cols, 2);
        assert_eq!(Size::MIN_PANE.rows, 1);
    }
}
