//! Kernel event and effect types.
//!
//! Events flow into the kernel from the IO thread. Effects flow out of
//! the kernel to be executed by the IO and render threads.

use mux_types::{PaneId, ClientId, Size};

/// Events that the kernel can process.
#[derive(Debug, Clone)]
pub enum KernelEvent {
    /// A tmux command string from a client.
    Command {
        client_id: ClientId,
        command: String,
    },
    /// Data from a pane's child process PTY.
    PtyOutput {
        pane_id: PaneId,
        data: Vec<u8>,
    },
    /// A client's terminal was resized.
    ClientResize {
        client_id: ClientId,
        size: Size,
    },
    /// A pane's child process exited.
    PaneExited {
        pane_id: PaneId,
        status: i32,
    },
    /// A new client connected.
    ClientConnected {
        client_id: ClientId,
        size: Size,
    },
    /// A client disconnected.
    ClientDisconnected {
        client_id: ClientId,
    },
    /// Periodic tick for escape-time, status updates, etc.
    Tick,
}

/// Effects that the kernel requests.
#[derive(Debug, Clone)]
pub enum KernelEffect {
    /// Write bytes to a pane's PTY master.
    WritePty {
        pane_id: PaneId,
        data: Vec<u8>,
    },
    /// Resize a pane's PTY window.
    ResizePty {
        pane_id: PaneId,
        size: Size,
    },
    /// Fork + exec a new child process.
    SpawnChild {
        pane_id: PaneId,
        shell: String,
    },
    /// Send a signal to a child process.
    KillChild {
        pane_id: PaneId,
        signal: i32,
    },
    /// Trigger a render cycle.
    Render,
    /// Close a client connection.
    CloseClient {
        client_id: ClientId,
    },
    /// Send a command response to a client.
    CommandResponse {
        client_id: ClientId,
        response: String,
    },
    /// Initiate server shutdown.
    Shutdown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_command() {
        let event = KernelEvent::Command {
            client_id: ClientId::new(1),
            command: "new-session".into(),
        };
        assert!(matches!(event, KernelEvent::Command { .. }));
    }

    #[test]
    fn effect_shutdown() {
        let effect = KernelEffect::Shutdown;
        assert!(matches!(effect, KernelEffect::Shutdown));
    }

    #[test]
    fn effect_write_pty() {
        let effect = KernelEffect::WritePty {
            pane_id: PaneId::new(1),
            data: b"hello".to_vec(),
        };
        assert!(matches!(effect, KernelEffect::WritePty { .. }));
    }

    #[test]
    fn event_clone() {
        let event = KernelEvent::Tick;
        let cloned = event.clone();
        assert!(matches!(cloned, KernelEvent::Tick));
    }
}
