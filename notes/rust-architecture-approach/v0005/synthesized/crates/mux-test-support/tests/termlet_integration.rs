//! Integration tests: TestServer lifecycle.

use mux_test_support::{TestServer, isolated_socket_path, CleanupGuard};
use mux_types::geometry::Size;

#[test]
fn test_server_full_lifecycle() {
    let mut ts = TestServer::new();
    ts.feed(b"Step 1\r\n");
    ts.feed(b"Step 2\r\n");
    ts.feed(b"Step 3");
    assert_eq!(ts.pane_text(0), "Step 1");
    assert_eq!(ts.pane_text(1), "Step 2");
    assert_eq!(ts.pane_text(2), "Step 3");
}

#[test]
fn test_server_csi_sequences() {
    let mut ts = TestServer::new();
    ts.feed(b"\x1b[3;5HX");
    let (x, y) = ts.cursor();
    assert_eq!(x, 5);
    assert_eq!(y, 2);
}

#[test]
fn isolated_paths_unique() {
    let a = isolated_socket_path();
    let b = isolated_socket_path();
    assert_ne!(a, b);
}

#[test]
fn cleanup_guard_works() {
    let path = isolated_socket_path();
    std::fs::write(&path, "test").unwrap_or(());
    let guard = CleanupGuard::new(path.clone());
    drop(guard);
    assert!(!path.exists());
}

#[test]
fn test_server_with_size() {
    let ts = TestServer::with_size(Size::new(120, 40));
    assert!(ts.primary_pane.is_some());
}
