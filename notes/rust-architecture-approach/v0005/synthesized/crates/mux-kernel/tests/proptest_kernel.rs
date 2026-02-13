//! Property-based tests for mux-kernel.

use proptest::prelude::*;
use mux_kernel::{Kernel, KernelEvent, KernelEffect};
use mux_kernel::layout::{LayoutNode, Orientation, distribute_evenly, even_horizontal, even_vertical};
use mux_types::geometry::Size;
use mux_types::id::PaneId;
use mux_time::Clock;

proptest! {
    #[test]
    fn kernel_create_session_returns_effects(name in "[a-zA-Z]{1,20}") {
        let mut kernel = Kernel::new(Clock::manual());
        let effects = kernel.process_event(KernelEvent::CreateSession {
            name: name,
            size: Size::new(80, 24),
        });
        // Should produce at least SessionCreated + SpawnChild
        prop_assert!(effects.len() >= 2);
        let has_session_created = effects.iter().any(|e| matches!(e, KernelEffect::SessionCreated { .. }));
        prop_assert!(has_session_created);
    }

    #[test]
    fn kernel_session_count(count in 1..5usize) {
        let mut kernel = Kernel::new(Clock::manual());
        for i in 0..count {
            kernel.process_event(KernelEvent::CreateSession {
                name: format!("s{i}"),
                size: Size::new(80, 24),
            });
        }
        prop_assert_eq!(kernel.session_count(), count);
    }

    #[test]
    fn kernel_pty_output_renders(data in proptest::collection::vec(0x20..=0x7Eu8, 1..100)) {
        let mut kernel = Kernel::new(Clock::manual());
        let effects = kernel.process_event(KernelEvent::CreateSession {
            name: "test".to_string(),
            size: Size::new(80, 24),
        });
        let pane_id = effects.iter().find_map(|e| match e {
            KernelEffect::SessionCreated { pane_id, .. } => Some(*pane_id),
            _ => None,
        }).unwrap_or(PaneId(0));
        let effects = kernel.process_event(KernelEvent::PtyOutput {
            pane_id,
            data,
        });
        // Should produce Render effect
        let has_render = effects.iter().any(|e| matches!(e, KernelEffect::Render));
        prop_assert!(has_render);
    }

    #[test]
    fn layout_distribute_evenly_sums(total in 1..500u16, count in 1..20usize) {
        let sizes = distribute_evenly(total, count);
        let sum: u16 = sizes.iter().sum();
        prop_assert_eq!(sum, total);
    }

    #[test]
    fn layout_distribute_evenly_balanced(total in 10..500u16, count in 1..20usize) {
        let sizes = distribute_evenly(total, count);
        if sizes.len() >= 2 {
            let min = *sizes.iter().min().unwrap_or(&0);
            let max = *sizes.iter().max().unwrap_or(&0);
            prop_assert!(max - min <= 1, "sizes should differ by at most 1");
        }
    }

    #[test]
    fn layout_node_leaf_has_pane(id in 1..100u64) {
        let node = LayoutNode::Leaf {
            pane_id: PaneId(id), x: 0, y: 0, cols: 80, rows: 24,
        };
        prop_assert_eq!(node.pane_ids(), vec![PaneId(id)]);
    }

    #[test]
    fn layout_even_horizontal_pane_count(pane_count in 1usize..10) {
        let panes: Vec<PaneId> = (0..pane_count).map(|i| PaneId(i as u64 + 1)).collect();
        let layout = even_horizontal(&panes, 80, 24);
        prop_assert_eq!(layout.pane_count(), pane_count);
    }

    #[test]
    fn even_horizontal_widths_sum(
        pane_count in 1usize..10,
        cols in 20u16..300,
        rows in 5u16..100,
    ) {
        let panes: Vec<PaneId> = (0..pane_count).map(|i| PaneId(i as u64 + 1)).collect();
        let layout = even_horizontal(&panes, cols, rows);
        if let LayoutNode::Split { children, .. } = &layout {
            let total_width: u16 = children.iter().map(|c| c.size().0).sum();
            prop_assert_eq!(total_width, cols);
        }
    }

    #[test]
    fn even_vertical_heights_sum(
        pane_count in 1usize..10,
        cols in 20u16..300,
        rows in 5u16..100,
    ) {
        let panes: Vec<PaneId> = (0..pane_count).map(|i| PaneId(i as u64 + 1)).collect();
        let layout = even_vertical(&panes, cols, rows);
        if let LayoutNode::Split { children, .. } = &layout {
            let total_height: u16 = children.iter().map(|c| c.size().1).sum();
            prop_assert_eq!(total_height, rows);
        }
    }
}

#[test]
fn kernel_destroy_session() {
    let mut kernel = Kernel::new(Clock::manual());
    let effects = kernel.process_event(KernelEvent::CreateSession {
        name: "temp".to_string(),
        size: Size::new(80, 24),
    });
    let session_id = effects.iter().find_map(|e| match e {
        KernelEffect::SessionCreated { session_id, .. } => Some(*session_id),
        _ => None,
    }).unwrap_or(mux_types::id::SessionId(0));
    kernel.process_event(KernelEvent::DestroySession { session_id });
    assert_eq!(kernel.session_count(), 0);
}

#[test]
fn kernel_pane_exit() {
    let mut kernel = Kernel::new(Clock::manual());
    let effects = kernel.process_event(KernelEvent::CreateSession {
        name: "test".to_string(),
        size: Size::new(80, 24),
    });
    let pane_id = effects.iter().find_map(|e| match e {
        KernelEffect::SessionCreated { pane_id, .. } => Some(*pane_id),
        _ => None,
    }).unwrap_or(PaneId(0));
    let effects = kernel.process_event(KernelEvent::PaneExited {
        pane_id,
        code: 0,
    });
    let has_closed = effects.iter().any(|e| matches!(e, KernelEffect::PaneClosed { .. }));
    assert!(has_closed);
}

#[test]
fn kernel_rename_session() {
    let mut kernel = Kernel::new(Clock::manual());
    let effects = kernel.process_event(KernelEvent::CreateSession {
        name: "old".to_string(),
        size: Size::new(80, 24),
    });
    let session_id = effects.iter().find_map(|e| match e {
        KernelEffect::SessionCreated { session_id, .. } => Some(*session_id),
        _ => None,
    }).unwrap_or(mux_types::id::SessionId(0));
    kernel.process_event(KernelEvent::RenameSession {
        session_id,
        name: "new".to_string(),
    });
    assert!(kernel.session_by_name("new").is_some());
}

#[test]
fn layout_distribute_single() {
    let sizes = distribute_evenly(80, 1);
    assert_eq!(sizes, vec![80]);
}

#[test]
fn layout_distribute_two() {
    let sizes = distribute_evenly(80, 2);
    assert_eq!(sizes[0] + sizes[1], 80);
    assert!((sizes[0] as i32 - sizes[1] as i32).abs() <= 1);
}
