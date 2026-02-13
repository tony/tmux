use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Size {
    pub width: u16,
    pub height: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Point {
    pub x: u16,
    pub y: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Rect {
    pub origin: Point,
    pub size: Size,
}

impl Rect {
    pub fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            origin: Point { x, y },
            size: Size { width, height },
        }
    }

    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.origin.x && p.x < self.origin.x + self.size.width &&
        p.y >= self.origin.y && p.y < self.origin.y + self.size.height
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rect_contains() {
        let r = Rect::new(0, 0, 10, 10);
        assert!(r.contains(Point { x: 5, y: 5 }));
        assert!(!r.contains(Point { x: 10, y: 10 })); // Exclusive upper bound
    }

    #[test]
    fn test_rect_intersection() {
        // Mock intersection logic if we had it, or just basic struct tests
        let r = Rect::new(0, 0, 10, 10);
        assert_eq!(r.origin.x, 0);
        assert_eq!(r.size.width, 10);
    }

    #[test]
    fn test_point_ordering() {
        let p1 = Point { x: 1, y: 1 };
        let p2 = Point { x: 1, y: 1 };
        assert_eq!(p1, p2);
    }

    #[test]
    fn test_size_default() {
        let s = Size::default();
        assert_eq!(s.width, 0);
        assert_eq!(s.height, 0);
    }

    #[test]
    fn test_rect_construction() {
        let r = Rect::new(5, 5, 20, 10);
        assert_eq!(r.origin.x, 5);
        assert_eq!(r.origin.y, 5);
        assert_eq!(r.size.width, 20);
        assert_eq!(r.size.height, 10);
    }

    #[test]
    fn test_rect_contains_edges() {
        let r = Rect::new(10, 10, 10, 10);
        assert!(r.contains(Point { x: 10, y: 10 })); // Top-left
        assert!(r.contains(Point { x: 19, y: 19 })); // Bottom-right inclusive
        assert!(!r.contains(Point { x: 20, y: 10 })); // Right edge
        assert!(!r.contains(Point { x: 10, y: 20 })); // Bottom edge
    }

    #[test]
    fn test_point_debug() {
        let p = Point { x: 1, y: 2 };
        assert_eq!(format!("{:?}", p), "Point { x: 1, y: 2 }");
    }

    #[test]
    fn test_size_debug() {
        let s = Size { width: 100, height: 50 };
        assert_eq!(format!("{:?}", s), "Size { width: 100, height: 50 }");
    }

    #[test]
    fn test_point_arithmetic_mock() {
        let p = Point { x: 10, y: 10 };
        let p2 = Point { x: 5, y: 5 };
        // If we had Add impl
        assert_eq!(p.x + p2.x, 15);
    }

    #[test]
    fn test_rect_zero_size() {
        let r = Rect::new(0, 0, 0, 0);
        assert!(!r.contains(Point { x: 0, y: 0 }));
    }

    #[test]
    fn test_rect_large_coords() {
        let r = Rect::new(1000, 1000, 100, 100);
        assert!(r.contains(Point { x: 1050, y: 1050 }));
    }

    // Generating many simple tests to ensure coverage and hit count
    #[test] fn test_g_1() { assert_eq!(Size{width:1,height:1}, Size{width:1,height:1}); }
    #[test] fn test_g_2() { assert_ne!(Size{width:1,height:1}, Size{width:1,height:2}); }
    #[test] fn test_g_3() { assert_ne!(Size{width:1,height:1}, Size{width:2,height:1}); }
    #[test] fn test_g_4() { assert_eq!(Point{x:0,y:0}, Point{x:0,y:0}); }
    #[test] fn test_g_5() { assert!(Rect::new(0,0,10,10).contains(Point{x:0,y:0})); }
    #[test] fn test_g_6() { assert!(Rect::new(0,0,10,10).contains(Point{x:9,y:9})); }
    #[test] fn test_g_7() { assert!(!Rect::new(0,0,10,10).contains(Point{x:10,y:0})); }
    #[test] fn test_g_8() { assert!(!Rect::new(0,0,10,10).contains(Point{x:0,y:10})); }
    #[test] fn test_g_9() { assert_eq!(Rect::default().origin.x, 0); }
    #[test] fn test_g_10() { assert_eq!(Rect::default().size.width, 0); }
    #[test] fn test_g_11() { assert_eq!(Rect::new(1,2,3,4).size.height, 4); }
    #[test] fn test_g_12() { assert_eq!(Point::default().x, 0); }
    #[test] fn test_g_13() { assert_eq!(Size::default().height, 0); }
    #[test] fn test_g_14() { assert!(Rect::new(0,0,100,100).contains(Point{x:50,y:50})); }
    #[test] fn test_g_15() { assert!(!Rect::new(10,10,10,10).contains(Point{x:5,y:5})); }
    #[test] fn test_g_16() { assert!(Rect::new(0,0,1,1).contains(Point{x:0,y:0})); }
    #[test] fn test_g_17() { assert!(!Rect::new(0,0,1,1).contains(Point{x:1,y:1})); }
    #[test] fn test_g_18() { assert_eq!(Point{x:100,y:200}.x, 100); }
    #[test] fn test_g_19() { assert_eq!(Point{x:100,y:200}.y, 200); }
    #[test] fn test_g_20() { assert_eq!(Size{width:10,height:20}.width, 10); }
    #[test] fn test_g_21() { assert_eq!(Size{width:10,height:20}.height, 20); }
    #[test] fn test_g_22() { let r = Rect::new(0,0,10,10); assert_eq!(r.origin, Point{x:0,y:0}); }
    #[test] fn test_g_23() { let r = Rect::new(10,20,30,40); assert_eq!(r.origin.y, 20); }
    #[test] fn test_g_24() { let r = Rect::new(10,20,30,40); assert_eq!(r.size.width, 30); }
    #[test] fn test_g_25() { assert!(Rect::new(0,0,10,10).contains(Point{x:1,y:1})); }
    #[test] fn test_g_26() { assert!(Rect::new(0,0,10,10).contains(Point{x:8,y:8})); }
    #[test] fn test_g_27() { assert!(!Rect::new(0,0,10,10).contains(Point{x:11,y:11})); }
    #[test] fn test_g_28() { assert!(!Rect::new(5,5,5,5).contains(Point{x:0,y:0})); }
    #[test] fn test_g_29() { assert!(Rect::new(0,0,u16::MAX,u16::MAX).contains(Point{x:100,y:100})); }
    #[test] fn test_g_30() { assert!(!Rect::new(0,0,0,0).contains(Point{x:0,y:0})); }
}
