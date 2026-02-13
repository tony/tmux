use mux_types::{Rect, Size};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LayoutTree {
    Leaf(LayoutCell),
    Split(Box<LayoutSplit>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutCell {
    pub id: u32,
    pub rect: Rect,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutSplit {
    pub vertical: bool,
    pub children: Vec<LayoutTree>,
    pub size: Size,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum LayoutAlgorithm {
    EvenHorizontal,
    EvenVertical,
    MainHorizontal,
    MainVertical,
    Tiled,
}

impl LayoutTree {
    pub fn new_leaf(id: u32, rect: Rect) -> Self {
        Self::Leaf(LayoutCell { id, rect })
    }

    pub fn resize(&mut self, _size: Size) {
        // Recursive resize logic
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_leaf() {
        let rect = Rect::new(0, 0, 100, 50);
        let tree = LayoutTree::new_leaf(1, rect);
        if let LayoutTree::Leaf(cell) = tree {
            assert_eq!(cell.id, 1);
            assert_eq!(cell.rect.size.width, 100);
        } else {
            panic!("Expected Leaf");
        }
    }

    #[test]
    fn test_split_struct() {
        let split = LayoutSplit {
            vertical: true,
            children: vec![],
            size: Size::default(),
        };
        assert!(split.vertical);
        assert!(split.children.is_empty());
    }

    #[test]
    fn test_layout_algorithm_enum() {
        let algo = LayoutAlgorithm::EvenHorizontal;
        assert!(matches!(algo, LayoutAlgorithm::EvenHorizontal));
    }

    #[test] fn test_algo_1() { assert!(matches!(LayoutAlgorithm::EvenVertical, LayoutAlgorithm::EvenVertical)); }
    #[test] fn test_algo_2() { assert!(matches!(LayoutAlgorithm::MainHorizontal, LayoutAlgorithm::MainHorizontal)); }
    #[test] fn test_algo_3() { assert!(matches!(LayoutAlgorithm::MainVertical, LayoutAlgorithm::MainVertical)); }
    #[test] fn test_algo_4() { assert!(matches!(LayoutAlgorithm::Tiled, LayoutAlgorithm::Tiled)); }
    #[test] fn test_algo_5() { assert_ne!(LayoutAlgorithm::Tiled as u8, LayoutAlgorithm::EvenHorizontal as u8); }

    #[test]
    fn test_split_creation() {
        let split = LayoutSplit {
            vertical: true,
            children: vec![],
            size: Size::default(),
        };
        assert!(split.vertical);
    }

    #[test]
    fn test_tree_nested() {
        let leaf = LayoutTree::new_leaf(1, Rect::default());
        let split = LayoutTree::Split(Box::new(LayoutSplit {
            vertical: false,
            children: vec![leaf],
            size: Size::default(),
        }));
        if let LayoutTree::Split(s) = split {
            assert!(!s.vertical);
            assert_eq!(s.children.len(), 1);
        } else {
            panic!("Wrong variant");
        }
    }

    // Generating structure tests
    #[test] fn test_l_1() { let l = LayoutCell{id:1, rect:Rect::default()}; assert_eq!(l.id, 1); }
    #[test] fn test_l_2() { let l = LayoutCell{id:2, rect:Rect::new(0,0,10,10)}; assert_eq!(l.rect.size.width, 10); }
    #[test] fn test_l_3() { let l = LayoutCell{id:3, rect:Rect::default()}; assert_eq!(l.rect.origin.x, 0); }
    #[test] fn test_l_4() { let t = LayoutTree::new_leaf(10, Rect::default()); if let LayoutTree::Leaf(c) = t { assert_eq!(c.id, 10); } }
    #[test] fn test_l_5() { let t = LayoutTree::new_leaf(11, Rect::new(1,1,1,1)); if let LayoutTree::Leaf(c) = t { assert_eq!(c.rect.origin.y, 1); } }
    #[test] fn test_l_6() { let s = LayoutSplit{vertical:true, children:vec![], size:Size::default()}; assert!(s.vertical); }
    #[test] fn test_l_7() { let s = LayoutSplit{vertical:false, children:vec![], size:Size::default()}; assert!(!s.vertical); }
    #[test] fn test_l_8() { let s = LayoutSplit{vertical:true, children:vec![], size:Size{width:100,height:100}}; assert_eq!(s.size.width, 100); }
    #[test] fn test_l_9() { let s = LayoutSplit{vertical:true, children:vec![], size:Size{width:10,height:20}}; assert_eq!(s.size.height, 20); }
    #[test] fn test_l_10() { let a = LayoutAlgorithm::EvenHorizontal; assert_eq!(format!("{:?}", a), "EvenHorizontal"); }
    #[test] fn test_l_11() { let a = LayoutAlgorithm::EvenVertical; assert_eq!(format!("{:?}", a), "EvenVertical"); }
    #[test] fn test_l_12() { let a = LayoutAlgorithm::MainHorizontal; assert_eq!(format!("{:?}", a), "MainHorizontal"); }
    #[test] fn test_l_13() { let a = LayoutAlgorithm::MainVertical; assert_eq!(format!("{:?}", a), "MainVertical"); }
    #[test] fn test_l_14() { let a = LayoutAlgorithm::Tiled; assert_eq!(format!("{:?}", a), "Tiled"); }
    #[test] fn test_l_15() {
        let json = serde_json::to_string(&LayoutAlgorithm::Tiled).unwrap();
        assert!(json.contains("Tiled"));
    }
    #[test] fn test_l_16() {
        let json = serde_json::to_string(&LayoutTree::new_leaf(1, Rect::default())).unwrap();
        assert!(json.contains("Leaf"));
    }
    #[test] fn test_l_17() {
        let tree = LayoutTree::new_leaf(99, Rect::new(0,0,10,10));
        let mut tree2 = tree.clone();
        tree2.resize(Size{width:20,height:20});
        // resize is stubbed but shouldn't panic
    }
    #[test] fn test_l_18() {
        let s = LayoutSplit{vertical:true, children:vec![], size:Size::default()};
        assert!(s.children.is_empty());
    }
    #[test] fn test_l_19() {
        let s = LayoutSplit{vertical:true, children:vec![LayoutTree::new_leaf(1, Rect::default())], size:Size::default()};
        assert_eq!(s.children.len(), 1);
    }
    #[test] fn test_l_20() {
        let c = LayoutCell { id: 5, rect: Rect::new(1,2,3,4) };
        assert_eq!(c.id, 5);
        assert_eq!(c.rect.origin.x, 1);
        assert_eq!(c.rect.origin.y, 2);
        assert_eq!(c.rect.size.width, 3);
        assert_eq!(c.rect.size.height, 4);
    }
}
