//! Integration tests: QuerySet traversal on entity graph.

use mux_orm::*;
use mux_types::id::{SessionId, WindowId, PaneId};

fn build_test_graph() -> (Vec<SessionInfo>, Vec<WindowInfo>, Vec<PaneInfo>) {
    let sessions = vec![
        SessionInfo { id: SessionId(1), name: "dev".into(), window_count: 2, attached: true },
        SessionInfo { id: SessionId(2), name: "prod".into(), window_count: 1, attached: false },
    ];
    let windows = vec![
        WindowInfo { id: WindowId(1), session_id: SessionId(1), name: "editor".into(), index: 0, pane_count: 2, active: true },
        WindowInfo { id: WindowId(2), session_id: SessionId(1), name: "shell".into(), index: 1, pane_count: 1, active: false },
        WindowInfo { id: WindowId(3), session_id: SessionId(2), name: "main".into(), index: 0, pane_count: 1, active: true },
    ];
    let panes = vec![
        PaneInfo { id: PaneId(1), window_id: WindowId(1), index: 0, active: true, width: 80, height: 24 },
        PaneInfo { id: PaneId(2), window_id: WindowId(1), index: 1, active: false, width: 80, height: 24 },
        PaneInfo { id: PaneId(3), window_id: WindowId(2), index: 0, active: true, width: 80, height: 24 },
    ];
    (sessions, windows, panes)
}

#[test]
fn session_to_window_traversal() {
    let (sessions, windows, _) = build_test_graph();
    let session_qs = SessionQuerySet::new(sessions).filter_by_name("dev");
    assert_eq!(session_qs.count(), 1);
    let window_qs = WindowQuerySet::new(windows)
        .filter(Filter::NameContains("editor".into()));
    assert_eq!(window_qs.count(), 1);
}

#[test]
fn window_to_pane_traversal() {
    let (_, _, panes) = build_test_graph();
    let qs = PaneQuerySet::new(panes).filter_active();
    assert_eq!(qs.count(), 2);
}

#[test]
fn chained_filter_composition() {
    let (sessions, _, _) = build_test_graph();
    let qs = SessionQuerySet::new(sessions)
        .filter(Filter::IsAttached(true))
        .filter(Filter::NameContains("dev".into()));
    assert_eq!(qs.count(), 1);
}
