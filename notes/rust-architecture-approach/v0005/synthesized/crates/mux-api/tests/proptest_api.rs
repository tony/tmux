//! Property-based tests for mux-api.

use proptest::prelude::*;
use mux_api::ServerBuilder;
use mux_kernel::{KernelEvent, KernelEffect};
use mux_types::geometry::Size;
use mux_types::id::SessionId;

proptest! {
    #[test]
    fn server_builder_session_names(name in "[a-zA-Z][a-zA-Z0-9_]{0,20}") {
        let server = ServerBuilder::new()
            .session_name(&name)
            .build();
        prop_assert!(server.is_ok());
        if let Ok(srv) = server {
            prop_assert!(srv.kernel().session_by_name(&name).is_some());
        }
    }

    #[test]
    fn server_builder_valid_sizes(cols in 2u16..300, rows in 1u16..100) {
        let server = ServerBuilder::new()
            .size(Size::new(cols, rows))
            .build();
        prop_assert!(server.is_ok());
        if let Ok(srv) = server {
            prop_assert_eq!(srv.size(), Size::new(cols, rows));
        }
    }

    #[test]
    fn create_sessions_via_kernel(count in 1..5usize) {
        let mut server = ServerBuilder::new()
            .build()
            .unwrap_or_else(|_| std::process::abort());
        for i in 0..count {
            server.kernel_mut().process_event(KernelEvent::CreateSession {
                name: format!("session-{i}"),
                size: Size::new(80, 24),
            });
        }
        // 1 default + count new
        prop_assert_eq!(server.session_count(), 1 + count);
    }

    #[test]
    fn feed_data_to_primary_pane(data in proptest::collection::vec(proptest::num::u8::ANY, 0..200)) {
        let mut server = ServerBuilder::new()
            .build()
            .unwrap_or_else(|_| std::process::abort());
        if let Some(pane_id) = server.primary_pane() {
            server.feed_output(pane_id, &data);
        }
    }
}

#[test]
fn server_default_session() {
    let server = ServerBuilder::new()
        .build()
        .unwrap_or_else(|_| std::process::abort());
    assert_eq!(server.session_count(), 1);
}

#[test]
fn server_destroy_session_via_kernel() {
    let mut server = ServerBuilder::new()
        .build()
        .unwrap_or_else(|_| std::process::abort());
    let effects = server.kernel_mut().process_event(KernelEvent::CreateSession {
        name: "ephemeral".into(),
        size: Size::new(80, 24),
    });
    let sid = effects.iter().find_map(|e| match e {
        KernelEffect::SessionCreated { session_id, .. } => Some(*session_id),
        _ => None,
    }).unwrap_or(SessionId(0));
    server.kernel_mut().process_event(KernelEvent::DestroySession { session_id: sid });
    assert_eq!(server.session_count(), 1); // Only default remains
}

#[test]
fn server_new_window() {
    let mut server = ServerBuilder::new()
        .build()
        .unwrap_or_else(|_| std::process::abort());
    if let Some(session_id) = server.primary_session() {
        let effects = server.new_window(session_id, "win2");
        assert!(!effects.is_empty());
    }
}

#[test]
fn server_rename_session() {
    let mut server = ServerBuilder::new()
        .session_name("old")
        .build()
        .unwrap_or_else(|_| std::process::abort());
    if let Some(session_id) = server.primary_session() {
        server.kernel_mut().process_event(KernelEvent::RenameSession {
            session_id,
            name: "new".into(),
        });
        assert!(server.kernel().session_by_name("new").is_some());
    }
}
