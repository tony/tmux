//! KernelEvent / KernelEffect types for the Sans-IO architecture.

use mux_types::{ClientId, PaneId, Size};

/// Events consumed by the kernel.
#[derive(Debug, Clone)]
pub enum KernelEvent {
    /// A client sent a command.
    Command { client_id: ClientId, command: String },
    /// PTY output from a pane.
    PtyOutput { pane_id: PaneId, data: Vec<u8> },
    /// Client terminal resized.
    ClientResize { client_id: ClientId, size: Size },
    /// A pane's child process exited.
    PaneExited { pane_id: PaneId, status: i32 },
    /// A new client connected.
    ClientConnected { client_id: ClientId, size: Size },
    /// A client disconnected.
    ClientDisconnected { client_id: ClientId },
    /// Periodic tick for maintenance.
    Tick,
}

/// Effects produced by the kernel (to be handled by the IO layer).
#[derive(Debug, Clone)]
pub enum KernelEffect {
    /// Spawn a child process in a pane.
    SpawnChild { pane_id: PaneId, shell: String },
    /// Send data to a pane's PTY.
    PtyWrite { pane_id: PaneId, data: Vec<u8> },
    /// Trigger a render cycle.
    Render,
    /// Send a command response to a client.
    CommandResponse { client_id: ClientId, response: String },
    /// Shut down the server.
    Shutdown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_variants() {
        let _cmd = KernelEvent::Command {
            client_id: ClientId::new(1),
            command: "test".into(),
        };
        let _tick = KernelEvent::Tick;
    }

    #[test]
    fn effect_variants() {
        let _spawn = KernelEffect::SpawnChild {
            pane_id: PaneId::new(1),
            shell: "/bin/sh".into(),
        };
        let _shutdown = KernelEffect::Shutdown;
    }
}
