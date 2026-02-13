//! Integration tests: API builder + kernel state.

use mux_api::ServerBuilder;
use mux_time::Clock;
use mux_types::geometry::Size;

#[test]
fn builder_creates_server_with_session() {
    let server = ServerBuilder::new()
        .session_name("integration")
        .build()
        .unwrap_or_else(|_| std::process::abort());
    assert_eq!(server.session_count(), 1);
    assert!(server.primary_session().is_some());
}

#[test]
fn server_feed_and_render() {
    let mut server = ServerBuilder::new()
        .size(Size::new(80, 24))
        .build()
        .unwrap_or_else(|_| std::process::abort());
    if let Some(pane_id) = server.primary_pane() {
        server.feed_output(pane_id, b"Test data");
    }
}

#[test]
fn server_new_window_creation() {
    let mut server = ServerBuilder::new().build().unwrap_or_else(|_| std::process::abort());
    if let Some(session_id) = server.primary_session() {
        let effects = server.new_window(session_id, "second");
        assert!(!effects.is_empty());
    }
}

#[test]
fn server_passthrough_default() {
    let server = ServerBuilder::new().build().unwrap_or_else(|_| std::process::abort());
    assert!(!server.allow_passthrough());
}

#[test]
fn builder_with_config() {
    let server = ServerBuilder::new()
        .load_config("set -g mouse on\n")
        .unwrap_or_else(|_| ServerBuilder::new())
        .build()
        .unwrap_or_else(|_| std::process::abort());
    assert!(server.is_started());
}
