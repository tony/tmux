//! Comprehensive tests for the layout engine.

use mux_kernel::layout::{LayoutNode, Orientation, distribute_evenly, even_horizontal, even_vertical};
use mux_types::id::PaneId;

#[test]
fn leaf_node_has_pane() {
    let node = LayoutNode::Leaf {
        pane_id: PaneId(1),
        x: 0, y: 0, cols: 80, rows: 24,
    };
    let ids = node.pane_ids();
    assert_eq!(ids, vec![PaneId(1)]);
    assert_eq!(node.pane_count(), 1);
}

#[test]
fn horizontal_split_via_even_horizontal() {
    let panes = vec![PaneId(1), PaneId(2)];
    let layout = even_horizontal(&panes, 80, 24);
    assert_eq!(layout.pane_count(), 2);
    let ids = layout.pane_ids();
    assert!(ids.contains(&PaneId(1)));
    assert!(ids.contains(&PaneId(2)));
}

#[test]
fn vertical_split_via_even_vertical() {
    let panes = vec![PaneId(1), PaneId(2)];
    let layout = even_vertical(&panes, 80, 24);
    assert_eq!(layout.pane_count(), 2);
}

#[test]
fn three_way_horizontal() {
    let panes = vec![PaneId(1), PaneId(2), PaneId(3)];
    let layout = even_horizontal(&panes, 80, 24);
    assert_eq!(layout.pane_count(), 3);
}

#[test]
fn nested_layout() {
    // Build vertical split on left, single pane on right
    let left_panes = vec![PaneId(1), PaneId(2)];
    let left = even_vertical(&left_panes, 40, 24);
    let right = LayoutNode::Leaf {
        pane_id: PaneId(3),
        x: 40, y: 0, cols: 40, rows: 24,
    };
    let root = LayoutNode::Split {
        orientation: Orientation::Vertical,
        x: 0, y: 0, cols: 80, rows: 24,
        children: vec![left, right],
    };
    assert_eq!(root.pane_count(), 3);
}

#[test]
fn distribute_evenly_one() {
    assert_eq!(distribute_evenly(100, 1), vec![100]);
}

#[test]
fn distribute_evenly_two() {
    let result = distribute_evenly(100, 2);
    assert_eq!(result[0] + result[1], 100);
    assert!((result[0] as i32 - result[1] as i32).abs() <= 1);
}

#[test]
fn distribute_evenly_three() {
    let result = distribute_evenly(100, 3);
    assert_eq!(result.iter().sum::<u16>(), 100);
    let min = *result.iter().min().unwrap_or(&0);
    let max = *result.iter().max().unwrap_or(&0);
    assert!(max - min <= 1);
}

#[test]
fn distribute_evenly_exact_division() {
    let result = distribute_evenly(90, 3);
    assert_eq!(result, vec![30, 30, 30]);
}

#[test]
fn distribute_evenly_remainder() {
    let result = distribute_evenly(91, 3);
    assert_eq!(result.iter().sum::<u16>(), 91);
}

#[test]
fn distribute_evenly_many() {
    let result = distribute_evenly(200, 10);
    assert_eq!(result.iter().sum::<u16>(), 200);
    assert_eq!(result.len(), 10);
}

#[test]
fn distribute_evenly_small() {
    let result = distribute_evenly(5, 3);
    assert_eq!(result.iter().sum::<u16>(), 5);
}

#[test]
fn distribute_evenly_zero_count() {
    let result = distribute_evenly(80, 0);
    assert!(result.is_empty());
}

#[test]
fn layout_node_size() {
    let node = LayoutNode::Leaf {
        pane_id: PaneId(1),
        x: 0, y: 0, cols: 80, rows: 24,
    };
    assert_eq!(node.size(), (80, 24));
}

#[test]
fn layout_node_position() {
    let node = LayoutNode::Leaf {
        pane_id: PaneId(1),
        x: 10, y: 5, cols: 80, rows: 24,
    };
    assert_eq!(node.position(), (10, 5));
}

#[test]
fn layout_pane_ids_collected() {
    let panes = vec![PaneId(1), PaneId(2), PaneId(3)];
    let layout = even_horizontal(&panes, 120, 40);
    let ids = layout.pane_ids();
    assert_eq!(ids.len(), 3);
    assert!(ids.contains(&PaneId(1)));
    assert!(ids.contains(&PaneId(2)));
    assert!(ids.contains(&PaneId(3)));
}

#[test]
fn layout_single_pane() {
    let panes = vec![PaneId(1)];
    let layout = even_horizontal(&panes, 80, 24);
    assert_eq!(layout.pane_count(), 1);
    assert_eq!(layout.size(), (80, 24));
}

#[test]
fn layout_dimensions_sum_horizontal() {
    let panes = vec![PaneId(1), PaneId(2), PaneId(3), PaneId(4)];
    let layout = even_horizontal(&panes, 100, 30);
    if let LayoutNode::Split { children, .. } = &layout {
        let total_width: u16 = children.iter().map(|c| c.size().0).sum();
        assert_eq!(total_width, 100);
    }
}

#[test]
fn layout_dimensions_sum_vertical() {
    let panes = vec![PaneId(1), PaneId(2), PaneId(3)];
    let layout = even_vertical(&panes, 80, 24);
    if let LayoutNode::Split { children, .. } = &layout {
        let total_height: u16 = children.iter().map(|c| c.size().1).sum();
        assert_eq!(total_height, 24);
    }
}

#[test]
fn orientation_types() {
    assert_ne!(Orientation::Horizontal, Orientation::Vertical);
}

#[test]
fn layout_debug_format() {
    let node = LayoutNode::Leaf {
        pane_id: PaneId(1),
        x: 0, y: 0, cols: 80, rows: 24,
    };
    let dbg = format!("{node:?}");
    assert!(dbg.contains("Leaf"));
}

#[test]
fn layout_clone() {
    let node = LayoutNode::Leaf {
        pane_id: PaneId(1),
        x: 0, y: 0, cols: 80, rows: 24,
    };
    let cloned = node.clone();
    assert_eq!(cloned.pane_ids(), node.pane_ids());
}
