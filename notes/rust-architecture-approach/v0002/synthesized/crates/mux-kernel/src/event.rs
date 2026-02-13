//! Kernel event and effect types for the Sans-IO reducer pattern.

use mux_types::{ClientId, PaneId, Size};

/// Events fed into the kernel's [`process_event`] reducer.
#[derive(Debug, Clone)]
pub enum KernelEvent {
    /// A command string from a client (e.g. "new-session -d -s foo").
    Command { client: ClientId, command: String },
    /// Data arrived from a pane's PTY.
    PtyOutput { pane: PaneId, data: Vec<u8> },
    /// A client was resized.
    ClientResize { client: ClientId, size: Size },
    /// A pane's child process exited.
    PaneExited { pane: PaneId, exit_code: i32 },
    /// A client connected.
    ClientConnected { client: ClientId, size: Size },
    /// A client disconnected.
    ClientDisconnected { client: ClientId },
    /// Timer tick (for escape-time, status updates, etc.).
    Tick,
}

/// Effects emitted by the kernel's reducer for the IO layer to execute.
#[derive(Debug, Clone)]
pub enum KernelEffect {
    /// Write bytes to a pane's PTY.
    WritePty { pane: PaneId, data: Vec<u8> },
    /// Resize a pane's PTY.
    ResizePty { pane: PaneId, size: Size },
    /// Spawn a new child process on a PTY.
    SpawnChild { pane: PaneId, shell: String, size: Size },
    /// Kill a pane's child process.
    KillChild { pane: PaneId, signal: i32 },
    /// Render the specified client's terminal.
    Render { client: ClientId },
    /// Close a client connection.
    CloseClient { client: ClientId },
    /// Command response to send back to the client.
    CommandResponse { client: ClientId, success: bool, output: String },
    /// Server shutdown requested.
    Shutdown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_debug_format() {
        let e = KernelEvent::Command {
            client: ClientId(1),
            command: "test".into(),
        };
        let s = format!("{e:?}");
        assert!(s.contains("Command"));
    }

    #[test]
    fn effect_debug_format() {
        let e = KernelEffect::Render { client: ClientId(1) };
        let s = format!("{e:?}");
        assert!(s.contains("Render"));
    }
}
