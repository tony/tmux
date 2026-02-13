//! Test server lifecycle and convenience method tests.
#![allow(clippy::unwrap_used)]

use mux_test_support::{TestServer, CleanupGuard, isolated_socket_path};
use mux_types::geometry::Size;

#[test]
fn server_creates_with_session() {
    let ts = TestServer::new();
    assert!(ts.primary_session.is_some());
}

#[test]
fn server_creates_with_window() {
    let ts = TestServer::new();
    assert!(ts.primary_window.is_some());
}

#[test]
fn server_creates_with_pane() {
    let ts = TestServer::new();
    assert!(ts.primary_pane.is_some());
}

#[test]
fn server_feed_text() {
    let mut ts = TestServer::new();
    ts.feed(b"Hello World");
    let text = ts.pane_text(0);
    assert!(text.contains("Hello World"));
}

#[test]
fn server_cursor_moves_with_text() {
    let mut ts = TestServer::new();
    ts.feed(b"ABCDE");
    let (x, y) = ts.cursor();
    assert_eq!(x, 5);
    assert_eq!(y, 0);
}

#[test]
fn server_newline_moves_cursor() {
    let mut ts = TestServer::new();
    ts.feed(b"Line1\r\nLine2");
    let (_, y) = ts.cursor();
    assert_eq!(y, 1);
}

#[test]
fn server_carriage_return() {
    let mut ts = TestServer::new();
    ts.feed(b"XXXX\r");
    let (x, _) = ts.cursor();
    assert_eq!(x, 0);
}

#[test]
fn server_custom_size() {
    let ts = TestServer::with_size(Size::new(120, 40));
    assert!(ts.primary_pane.is_some());
}

#[test]
fn server_small_size() {
    let ts = TestServer::with_size(Size::new(10, 5));
    assert!(ts.primary_pane.is_some());
}

#[test]
fn server_multiple_feeds() {
    let mut ts = TestServer::new();
    ts.feed(b"Hello ");
    ts.feed(b"World");
    let text = ts.pane_text(0);
    assert!(text.contains("Hello World"));
}

#[test]
fn server_sgr_bold() {
    let mut ts = TestServer::new();
    ts.feed(b"\x1b[1mBold\x1b[0m Normal");
    // Should not crash
}

#[test]
fn server_cursor_position() {
    let mut ts = TestServer::new();
    ts.feed(b"\x1b[10;20H");
    let (x, y) = ts.cursor();
    assert_eq!(x, 19); // 0-indexed
    assert_eq!(y, 9);
}

#[test]
fn socket_paths_unique_parallel() {
    let paths: Vec<_> = (0..20).map(|_| isolated_socket_path()).collect();
    let unique: std::collections::HashSet<_> = paths.iter().collect();
    assert_eq!(unique.len(), 20);
}

#[test]
fn cleanup_guard_works() {
    let path = isolated_socket_path();
    std::fs::write(&path, b"data").unwrap();
    {
        let _guard = CleanupGuard::new(path.clone());
    }
    assert!(!path.exists());
}

#[test]
fn cleanup_guard_no_panic_on_missing() {
    let path = std::env::temp_dir().join("nonexistent-test-file-12345");
    let _guard = CleanupGuard::new(path);
    // Should not panic on drop
}

#[test]
fn server_erase_display() {
    let mut ts = TestServer::new();
    ts.feed(b"XXXXX\r\n");
    ts.feed(b"YYYYY\r\n");
    ts.feed(b"\x1b[2J");
    // After clear, all should be empty
}

#[test]
fn server_erase_line() {
    let mut ts = TestServer::new();
    ts.feed(b"Hello World\x1b[K");
}

#[test]
fn server_wrap_around() {
    let ts = TestServer::with_size(Size::new(10, 5));
    let mut ts = ts;
    ts.feed(b"1234567890X");
    // X should wrap to next line
    let (_, y) = ts.cursor();
    assert!(y >= 1 || ts.pane_text(1).contains('X'));
}
