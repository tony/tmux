//! Kernel events and effects.

use mux_types::id::{SessionId, WindowId, PaneId, ClientId};
use mux_types::geometry::Size;
use crate::KernelError;

/// Events sent TO the kernel from the IO thread.
#[derive(Debug, Clone)]
pub enum KernelEvent {
    CreateSession { name: String, size: Size },
    CreateWindow { session_id: SessionId, name: String },
    SplitPane { window_id: WindowId, size: Size, vertical: bool },
    PtyOutput { pane_id: PaneId, data: Vec<u8> },
    PaneExited { pane_id: PaneId, code: i32 },
    ClientResize { client_id: ClientId, size: Size },
    DestroySession { session_id: SessionId },
    DestroyPane { pane_id: PaneId },
    SelectWindow { session_id: SessionId, index: usize },
    SelectPane { window_id: WindowId, index: usize },
    RenameSession { session_id: SessionId, name: String },
}

/// Effects emitted BY the kernel for the IO thread to execute.
#[derive(Debug)]
pub enum KernelEffect {
    SessionCreated { session_id: SessionId, window_id: WindowId, pane_id: PaneId },
    WindowCreated { session_id: SessionId, window_id: WindowId, pane_id: PaneId },
    PaneCreated { window_id: WindowId, pane_id: PaneId },
    SpawnChild { pane_id: PaneId, size: Size },
    Render,
    PaneClosed { pane_id: PaneId, code: i32 },
    SessionDestroyed { session_id: SessionId },
    ResizePty { pane_id: PaneId, size: Size },
    WritePty { pane_id: PaneId, data: Vec<u8> },
    Error(KernelError),
}
