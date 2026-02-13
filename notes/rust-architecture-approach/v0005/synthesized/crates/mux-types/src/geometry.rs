//! Geometry types: Size, Position, Rect.
//!
//! These types represent terminal dimensions, cursor positions, and rectangular
//! regions. All sizes and positions are in character cells, not pixels.

/// Terminal dimensions in character cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Size {
    /// Number of columns.
    pub cols: u16,
    /// Number of rows.
    pub rows: u16,
}

impl Size {
    /// Minimum pane size: 2 columns by 1 row.
    pub const MIN_PANE: Self = Self { cols: 2, rows: 1 };

    /// Create a new size.
    pub const fn new(cols: u16, rows: u16) -> Self {
        Self { cols, rows }
    }

    /// Total number of cells (cols * rows).
    pub const fn area(self) -> u32 {
        self.cols as u32 * self.rows as u32
    }

    /// Returns true if this size meets the minimum pane dimensions.
    pub const fn meets_minimum(self) -> bool {
        self.cols >= Self::MIN_PANE.cols && self.rows >= Self::MIN_PANE.rows
    }

    /// Clamp to minimum pane size.
    pub fn clamp_to_minimum(self) -> Self {
        Self {
            cols: self.cols.max(Self::MIN_PANE.cols),
            rows: self.rows.max(Self::MIN_PANE.rows),
        }
    }

    /// Returns true if both dimensions are zero.
    pub const fn is_zero(self) -> bool {
        self.cols == 0 || self.rows == 0
    }

    /// Saturating subtraction for border calculations.
    pub fn saturating_sub(self, cols: u16, rows: u16) -> Self {
        Self {
            cols: self.cols.saturating_sub(cols),
            rows: self.rows.saturating_sub(rows),
        }
    }
}

/// A position in the terminal grid (column, row).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Position {
    /// Column (x coordinate), 0-based.
    pub x: u16,
    /// Row (y coordinate), 0-based.
    pub y: u16,
}

impl Position {
    /// Create a new position.
    pub const fn new(x: u16, y: u16) -> Self {
        Self { x, y }
    }

    /// Origin position (0, 0).
    pub const ORIGIN: Self = Self { x: 0, y: 0 };

    /// Returns true if this position is within the given size bounds.
    pub const fn within(self, size: Size) -> bool {
        self.x < size.cols && self.y < size.rows
    }

    /// Linear index for flat buffer access.
    pub const fn linear_index(self, cols: u16) -> usize {
        self.y as usize * cols as usize + self.x as usize
    }
}

/// A rectangular region in the terminal grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Rect {
    /// Top-left position.
    pub origin: Position,
    /// Dimensions.
    pub size: Size,
}

impl Rect {
    /// Create a new rect.
    pub const fn new(x: u16, y: u16, cols: u16, rows: u16) -> Self {
        Self {
            origin: Position::new(x, y),
            size: Size::new(cols, rows),
        }
    }

    /// Right edge (exclusive).
    pub const fn right(self) -> u16 {
        self.origin.x + self.size.cols
    }

    /// Bottom edge (exclusive).
    pub const fn bottom(self) -> u16 {
        self.origin.y + self.size.rows
    }

    /// Returns true if the given position is inside this rect.
    pub const fn contains(self, pos: Position) -> bool {
        pos.x >= self.origin.x
            && pos.x < self.right()
            && pos.y >= self.origin.y
            && pos.y < self.bottom()
    }

    /// Total area.
    pub const fn area(self) -> u32 {
        self.size.area()
    }

    /// Returns true if this rect is empty (zero area).
    pub const fn is_empty(self) -> bool {
        self.size.is_zero()
    }

    /// Intersection of two rects.
    pub fn intersect(self, other: Self) -> Option<Self> {
        let x1 = self.origin.x.max(other.origin.x);
        let y1 = self.origin.y.max(other.origin.y);
        let x2 = self.right().min(other.right());
        let y2 = self.bottom().min(other.bottom());
        if x1 < x2 && y1 < y2 {
            Some(Self::new(x1, y1, x2 - x1, y2 - y1))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_area() {
        assert_eq!(Size::new(80, 24).area(), 1920);
    }

    #[test]
    fn size_meets_minimum() {
        assert!(Size::new(2, 1).meets_minimum());
        assert!(!Size::new(1, 1).meets_minimum());
        assert!(!Size::new(2, 0).meets_minimum());
    }

    #[test]
    fn size_clamp() {
        let s = Size::new(1, 0).clamp_to_minimum();
        assert_eq!(s, Size::new(2, 1));
    }

    #[test]
    fn size_is_zero() {
        assert!(Size::new(0, 5).is_zero());
        assert!(!Size::new(5, 5).is_zero());
    }

    #[test]
    fn size_saturating_sub() {
        let s = Size::new(10, 5).saturating_sub(3, 2);
        assert_eq!(s, Size::new(7, 3));
    }

    #[test]
    fn size_saturating_sub_underflow() {
        let s = Size::new(2, 1).saturating_sub(5, 5);
        assert_eq!(s, Size::new(0, 0));
    }

    #[test]
    fn position_within() {
        let size = Size::new(80, 24);
        assert!(Position::new(0, 0).within(size));
        assert!(Position::new(79, 23).within(size));
        assert!(!Position::new(80, 0).within(size));
    }

    #[test]
    fn position_linear_index() {
        assert_eq!(Position::new(5, 2).linear_index(80), 165);
    }

    #[test]
    fn rect_contains() {
        let r = Rect::new(10, 10, 20, 10);
        assert!(r.contains(Position::new(15, 15)));
        assert!(!r.contains(Position::new(5, 5)));
        assert!(!r.contains(Position::new(30, 15)));
    }

    #[test]
    fn rect_right_bottom() {
        let r = Rect::new(10, 20, 30, 40);
        assert_eq!(r.right(), 40);
        assert_eq!(r.bottom(), 60);
    }

    #[test]
    fn rect_intersect() {
        let a = Rect::new(0, 0, 10, 10);
        let b = Rect::new(5, 5, 10, 10);
        let c = a.intersect(b);
        assert_eq!(c, Some(Rect::new(5, 5, 5, 5)));
    }

    #[test]
    fn rect_no_intersect() {
        let a = Rect::new(0, 0, 5, 5);
        let b = Rect::new(10, 10, 5, 5);
        assert!(a.intersect(b).is_none());
    }

    #[test]
    fn rect_area() {
        assert_eq!(Rect::new(0, 0, 10, 5).area(), 50);
    }

    #[test]
    fn rect_is_empty() {
        assert!(Rect::new(0, 0, 0, 5).is_empty());
        assert!(!Rect::new(0, 0, 1, 1).is_empty());
    }

    mod prop {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn size_area_product(cols in 0u16..1000, rows in 0u16..1000) {
                let s = Size::new(cols, rows);
                prop_assert_eq!(s.area(), cols as u32 * rows as u32);
            }

            #[test]
            fn rect_contains_within_bounds(
                x in 0u16..50, y in 0u16..50,
                w in 1u16..50, h in 1u16..50,
                px in 0u16..100, py in 0u16..100,
            ) {
                let r = Rect::new(x, y, w, h);
                let pos = Position::new(px, py);
                let expected = px >= x && px < x + w && py >= y && py < y + h;
                prop_assert_eq!(r.contains(pos), expected);
            }

            #[test]
            fn position_linear_index_prop(x in 0u16..80, y in 0u16..24) {
                let idx = Position::new(x, y).linear_index(80);
                prop_assert_eq!(idx, y as usize * 80 + x as usize);
            }
        }
    }
}
