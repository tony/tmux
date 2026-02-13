//! Property-based tests for mux-termlet.

use proptest::prelude::*;
use mux_termlet::{Termlet, TermletState, TermletError};
use mux_types::geometry::Size;

proptest! {
    #[test]
    fn termlet_creation_size(cols in 1..300u16, rows in 1..100u16) {
        let t = Termlet::new(Size::new(cols, rows));
        prop_assert_eq!(t.size(), Size::new(cols, rows));
        prop_assert_eq!(t.state(), TermletState::Created);
    }

    #[test]
    fn termlet_start_transitions(name in "[a-zA-Z]{1,20}") {
        let mut t = Termlet::new(Size::new(80, 24));
        let result = t.start(&name);
        prop_assert!(result.is_ok());
        prop_assert_eq!(t.state(), TermletState::Running);
    }

    #[test]
    fn termlet_feed_data(data in proptest::collection::vec(0x20..=0x7Eu8, 0..200)) {
        let mut t = Termlet::new(Size::new(80, 24));
        t.start("test").unwrap_or(());
        let result = t.feed(&data);
        prop_assert!(result.is_ok());
    }

    #[test]
    fn termlet_resize(
        cols in 1..200u16, rows in 1..50u16,
        new_cols in 1..200u16, new_rows in 1..50u16,
    ) {
        let mut t = Termlet::new(Size::new(cols, rows));
        t.start("test").unwrap_or(());
        t.resize(Size::new(new_cols, new_rows));
        prop_assert_eq!(t.size(), Size::new(new_cols, new_rows));
    }

    #[test]
    fn termlet_lifecycle_transitions(name in "[a-zA-Z]{1,10}") {
        let mut t = Termlet::new(Size::new(80, 24));
        prop_assert_eq!(t.state(), TermletState::Created);
        t.start(&name).unwrap_or(());
        prop_assert_eq!(t.state(), TermletState::Running);
        t.stop();
        prop_assert_eq!(t.state(), TermletState::Stopped);
    }

    #[test]
    fn termlet_multiple_render_cycles(count in 1..10u32) {
        let mut t = Termlet::new(Size::new(80, 24));
        t.start("test").unwrap_or(());
        for i in 0..count {
            let data = format!("line {i}\r\n");
            t.feed(data.as_bytes()).unwrap_or(());
            let _ = t.render();
        }
    }
}

#[test]
fn termlet_feed_before_start_errors() {
    let mut t = Termlet::new(Size::new(80, 24));
    assert!(t.feed(b"data").is_err());
}

#[test]
fn termlet_primary_pane_after_start() {
    let mut t = Termlet::new(Size::new(80, 24));
    t.start("test").unwrap_or(());
    assert!(t.primary_pane().is_some());
}

#[test]
fn termlet_kernel_access_after_start() {
    let mut t = Termlet::new(Size::new(80, 24));
    t.start("test").unwrap_or(());
    assert!(t.kernel().session_count() > 0);
}

#[test]
fn termlet_render_after_feed() {
    let mut t = Termlet::new(Size::new(80, 24));
    t.start("test").unwrap_or(());
    t.feed(b"Hello, World!").unwrap_or(());
    let output = t.render();
    assert!(!output.is_empty());
}

#[test]
fn termlet_render_without_feed() {
    let mut t = Termlet::new(Size::new(80, 24));
    t.start("test").unwrap_or(());
    let _ = t.render();
}
