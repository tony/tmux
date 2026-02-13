//! Copy mode and paste buffer ring property tests.
#![allow(clippy::unwrap_used)]

use proptest::prelude::*;
use mux_kernel::copy_mode::{CopyModeState, CopyKeyTable, SelectionMode, PasteBufferRing};
use mux_types::geometry::Position;

proptest! {
    #[test]
    fn enter_exit_idempotent(_dummy in 0..10u8) {
        let mut cm = CopyModeState::default();
        cm.enter();
        prop_assert!(cm.active);
        cm.exit();
        prop_assert!(!cm.active);
        cm.exit();
        prop_assert!(!cm.active);
    }

    #[test]
    fn cursor_movement_clamped(
        start_x in 0..100u16, start_y in 0..100u16,
        dx in 0..200u16, dy in 0..200u16,
        max_x in 0..200u16, max_y in 0..200u16,
    ) {
        let mut cm = CopyModeState::default();
        cm.enter();
        cm.cursor = Position { x: start_x, y: start_y };
        cm.cursor_right(dx, max_x);
        prop_assert!(cm.cursor.x <= max_x);
        cm.cursor_down(dy, max_y);
        prop_assert!(cm.cursor.y <= max_y);
    }

    #[test]
    fn cursor_left_never_negative(start_x in 0..100u16, amount in 0..200u16) {
        let mut cm = CopyModeState::default();
        cm.enter();
        cm.cursor = Position { x: start_x, y: 0 };
        cm.cursor_left(amount);
        // u16 can't be negative, but test saturating behavior
        prop_assert!(cm.cursor.x <= start_x);
    }

    #[test]
    fn cursor_up_never_negative(start_y in 0..100u16, amount in 0..200u16) {
        let mut cm = CopyModeState::default();
        cm.enter();
        cm.cursor = Position { x: 0, y: start_y };
        cm.cursor_up(amount);
        prop_assert!(cm.cursor.y <= start_y);
    }

    #[test]
    fn selection_range_ordered(
        ax in 0..80u16, ay in 0..24u16,
        cx in 0..80u16, cy in 0..24u16,
    ) {
        let mut cm = CopyModeState::default();
        cm.enter();
        cm.cursor = Position { x: ax, y: ay };
        cm.begin_selection();
        cm.cursor = Position { x: cx, y: cy };
        if let Some((start, end)) = cm.selection_range() {
            prop_assert!(start.y < end.y || (start.y == end.y && start.x <= end.x));
        }
    }

    #[test]
    fn paste_buffer_ring_bounded(max in 1..20usize, items in proptest::collection::vec("[a-z]{1,10}", 0..50)) {
        let mut ring = PasteBufferRing::new(max);
        for item in &items {
            ring.push(item.clone());
        }
        prop_assert!(ring.len() <= max);
    }

    #[test]
    fn paste_buffer_latest_is_last_pushed(items in proptest::collection::vec("[a-z]{1,10}", 1..20)) {
        let mut ring = PasteBufferRing::new(50);
        for item in &items {
            ring.push(item.clone());
        }
        prop_assert_eq!(ring.latest(), Some(items.last().unwrap().as_str()));
    }

    #[test]
    fn paste_buffer_get_zero_is_latest(items in proptest::collection::vec("[a-z]{1,10}", 1..20)) {
        let mut ring = PasteBufferRing::new(50);
        for item in &items {
            ring.push(item.clone());
        }
        prop_assert_eq!(ring.get(0), ring.latest());
    }
}
