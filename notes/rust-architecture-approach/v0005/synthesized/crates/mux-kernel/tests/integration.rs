//! Integration tests: kernel + grid + parser pipeline.

use mux_kernel::{Kernel, KernelEvent, KernelEffect};
use mux_time::Clock;
use mux_types::geometry::Size;

#[test]
fn kernel_processes_vt_output() {
    let mut kernel = Kernel::new(Clock::manual());
    let effects = kernel.process_event(KernelEvent::CreateSession {
        name: "test".into(),
        size: Size::new(80, 24),
    });
    let pane_id = match &effects[0] {
        KernelEffect::SessionCreated { pane_id, .. } => *pane_id,
        _ => return,
    };
    kernel.process_event(KernelEvent::PtyOutput {
        pane_id,
        data: b"Hello World".to_vec(),
    });
    let grid = kernel.pane_grid(pane_id);
    assert!(grid.is_some());
}

#[test]
fn kernel_csi_cursor_positioning() {
    let mut kernel = Kernel::new(Clock::manual());
    let effects = kernel.process_event(KernelEvent::CreateSession {
        name: "test".into(),
        size: Size::new(80, 24),
    });
    let pane_id = match &effects[0] {
        KernelEffect::SessionCreated { pane_id, .. } => *pane_id,
        _ => return,
    };
    kernel.process_event(KernelEvent::PtyOutput {
        pane_id,
        data: b"\x1b[5;10H".to_vec(),
    });
    let cursor = kernel.pane_grid(pane_id).map(mux_grid::ChunkedGrid::cursor);
    assert_eq!(cursor, Some((9, 4)));
}

#[test]
fn kernel_multiple_panes() {
    let mut kernel = Kernel::new(Clock::manual());
    let effects = kernel.process_event(KernelEvent::CreateSession {
        name: "test".into(),
        size: Size::new(80, 24),
    });
    let (session_id, window_id) = match &effects[0] {
        KernelEffect::SessionCreated { session_id, window_id, .. } => (*session_id, *window_id),
        _ => return,
    };
    kernel.process_event(KernelEvent::SplitPane {
        window_id,
        size: Size::new(40, 24),
        vertical: true,
    });
    assert_eq!(kernel.pane_count(), 2);
}

#[test]
fn kernel_erase_display() {
    let mut kernel = Kernel::new(Clock::manual());
    let effects = kernel.process_event(KernelEvent::CreateSession {
        name: "test".into(),
        size: Size::new(80, 24),
    });
    let pane_id = match &effects[0] {
        KernelEffect::SessionCreated { pane_id, .. } => *pane_id,
        _ => return,
    };
    kernel.process_event(KernelEvent::PtyOutput {
        pane_id,
        data: b"ABCDE\x1b[2J".to_vec(),
    });
}

#[test]
fn kernel_session_lifecycle() {
    let mut kernel = Kernel::new(Clock::manual());
    let effects = kernel.process_event(KernelEvent::CreateSession {
        name: "lifecycle".into(),
        size: Size::new(80, 24),
    });
    assert_eq!(kernel.session_count(), 1);
    let session_id = match &effects[0] {
        KernelEffect::SessionCreated { session_id, .. } => *session_id,
        _ => return,
    };
    kernel.process_event(KernelEvent::DestroySession { session_id });
    assert_eq!(kernel.session_count(), 0);
}
