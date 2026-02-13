//! Comprehensive server operation tests.

use mux_api::{ServerBuilder, Server};
use mux_kernel::{KernelEvent, KernelEffect};
use mux_types::geometry::Size;
use mux_types::id::SessionId;
use mux_options::OptionValue;

#[test]
fn server_builder_default() {
    let server = ServerBuilder::new().build().unwrap_or_else(|_| std::process::abort());
    assert_eq!(server.session_count(), 1); // Builder creates a default session
}

#[test]
fn server_builder_with_session_name() {
    let server = ServerBuilder::new()
        .session_name("test-session")
        .build()
        .unwrap_or_else(|_| std::process::abort());
    assert!(server.kernel().session_by_name("test-session").is_some());
}

#[test]
fn server_has_primary_session() {
    let server = ServerBuilder::new()
        .build()
        .unwrap_or_else(|_| std::process::abort());
    assert!(server.primary_session().is_some());
    assert_eq!(server.session_count(), 1);
}

#[test]
fn server_has_primary_pane() {
    let server = ServerBuilder::new()
        .build()
        .unwrap_or_else(|_| std::process::abort());
    assert!(server.primary_pane().is_some());
    assert_eq!(server.pane_count(), 1);
}

#[test]
fn create_additional_session_via_kernel() {
    let mut server = ServerBuilder::new()
        .build()
        .unwrap_or_else(|_| std::process::abort());
    let effects = server.kernel_mut().process_event(KernelEvent::CreateSession {
        name: "dev".into(),
        size: Size::new(80, 24),
    });
    assert!(effects.iter().any(|e| matches!(e, KernelEffect::SessionCreated { .. })));
    assert_eq!(server.session_count(), 2);
}

#[test]
fn create_multiple_sessions_via_kernel() {
    let mut server = ServerBuilder::new()
        .build()
        .unwrap_or_else(|_| std::process::abort());
    for name in ["dev", "prod", "staging"] {
        server.kernel_mut().process_event(KernelEvent::CreateSession {
            name: name.into(),
            size: Size::new(80, 24),
        });
    }
    assert_eq!(server.session_count(), 4); // 1 default + 3 new
}

#[test]
fn destroy_session_via_kernel() {
    let mut server = ServerBuilder::new()
        .build()
        .unwrap_or_else(|_| std::process::abort());
    let effects = server.kernel_mut().process_event(KernelEvent::CreateSession {
        name: "temp".into(),
        size: Size::new(80, 24),
    });
    let session_id = effects.iter().find_map(|e| match e {
        KernelEffect::SessionCreated { session_id, .. } => Some(*session_id),
        _ => None,
    }).unwrap_or(SessionId(0));
    server.kernel_mut().process_event(KernelEvent::DestroySession { session_id });
    assert_eq!(server.session_count(), 1); // Only default session remains
}

#[test]
fn create_window() {
    let mut server = ServerBuilder::new()
        .build()
        .unwrap_or_else(|_| std::process::abort());
    if let Some(session_id) = server.primary_session() {
        let effects = server.new_window(session_id, "editor");
        assert!(!effects.is_empty());
    }
}

#[test]
fn feed_pane_data() {
    let mut server = ServerBuilder::new()
        .build()
        .unwrap_or_else(|_| std::process::abort());
    if let Some(pane_id) = server.primary_pane() {
        server.feed_output(pane_id, b"Hello");
    }
}

#[test]
fn rename_session_via_kernel() {
    let mut server = ServerBuilder::new()
        .session_name("old-name")
        .build()
        .unwrap_or_else(|_| std::process::abort());
    if let Some(session_id) = server.primary_session() {
        server.kernel_mut().process_event(KernelEvent::RenameSession {
            session_id,
            name: "new-name".into(),
        });
        assert!(server.kernel().session_by_name("new-name").is_some());
    }
}

#[test]
fn server_with_custom_size() {
    let server = ServerBuilder::new()
        .size(Size::new(120, 40))
        .build()
        .unwrap_or_else(|_| std::process::abort());
    assert_eq!(server.size(), Size::new(120, 40));
}

#[test]
fn server_with_small_size() {
    let server = ServerBuilder::new()
        .size(Size::new(10, 5))
        .build()
        .unwrap_or_else(|_| std::process::abort());
    assert_eq!(server.size(), Size::new(10, 5));
}

#[test]
fn server_with_large_size() {
    let server = ServerBuilder::new()
        .size(Size::new(300, 100))
        .build()
        .unwrap_or_else(|_| std::process::abort());
    assert_eq!(server.size(), Size::new(300, 100));
}

#[test]
fn feed_empty_data() {
    let mut server = ServerBuilder::new()
        .build()
        .unwrap_or_else(|_| std::process::abort());
    if let Some(pane_id) = server.primary_pane() {
        server.feed_output(pane_id, b"");
    }
}

#[test]
fn feed_escape_sequences() {
    let mut server = ServerBuilder::new()
        .build()
        .unwrap_or_else(|_| std::process::abort());
    if let Some(pane_id) = server.primary_pane() {
        server.feed_output(pane_id, b"\x1b[1;31mRed\x1b[0m");
    }
}

#[test]
fn feed_large_data() {
    let mut server = ServerBuilder::new()
        .build()
        .unwrap_or_else(|_| std::process::abort());
    if let Some(pane_id) = server.primary_pane() {
        let data: Vec<u8> = (0..10000).map(|i| (b'A' + (i % 26) as u8)).collect();
        server.feed_output(pane_id, &data);
    }
}

#[test]
fn server_kernel_access() {
    let server = ServerBuilder::new()
        .build()
        .unwrap_or_else(|_| std::process::abort());
    assert!(server.kernel().session_count() > 0);
}

#[test]
fn server_is_started() {
    let server = ServerBuilder::new()
        .build()
        .unwrap_or_else(|_| std::process::abort());
    assert!(server.is_started());
}

#[test]
fn passthrough_default_off() {
    let server = ServerBuilder::new()
        .build()
        .unwrap_or_else(|_| std::process::abort());
    assert!(!server.allow_passthrough());
}

#[test]
fn server_composite_access() {
    let server = ServerBuilder::new()
        .build()
        .unwrap_or_else(|_| std::process::abort());
    let _composite = server.composite();
}

#[test]
fn server_builder_method() {
    let server = Server::builder().build();
    assert!(server.is_ok());
}

#[test]
fn builder_with_history_limit() {
    let server = ServerBuilder::new()
        .history_limit(5000)
        .build()
        .unwrap_or_else(|_| std::process::abort());
    let val = server.kernel().server_options
        .get("history-limit")
        .and_then(OptionValue::as_int);
    assert_eq!(val, Some(5000));
}

#[test]
fn builder_with_clock() {
    use mux_time::Clock;
    let server = ServerBuilder::new()
        .clock(Clock::manual())
        .build()
        .unwrap_or_else(|_| std::process::abort());
    assert!(server.is_started());
}

#[test]
fn builder_load_config() {
    let result = ServerBuilder::new()
        .load_config("set -g mouse on\n");
    assert!(result.is_ok());
}
