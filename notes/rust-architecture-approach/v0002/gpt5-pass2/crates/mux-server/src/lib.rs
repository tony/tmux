#![forbid(unsafe_code)]

use mux_api::{CrashFrame, MuxBuilder, MuxServer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerPhase {
    Init,
    Running,
    Draining,
    Stopped,
}

#[derive(Debug)]
pub struct ServerRuntime {
    pub server: MuxServer,
    pub phase: ServerPhase,
}

impl ServerRuntime {
    #[must_use]
    pub fn new() -> Self {
        Self { server: MuxBuilder::new().build(), phase: ServerPhase::Init }
    }

    pub fn start(&mut self) {
        self.server.start();
        self.phase = ServerPhase::Running;
    }

    #[must_use]
    pub fn crash(&mut self, msg: &str) -> CrashFrame {
        self.phase = ServerPhase::Draining;
        self.server.stop();
        self.phase = ServerPhase::Stopped;
        MuxServer::panic_recovery_frame(msg)
    }
}

impl Default for ServerRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_starts() {
        let mut rt = ServerRuntime::new();
        rt.start();
        assert_eq!(rt.phase, ServerPhase::Running);
    }

    #[test]
    fn crash_moves_to_stopped() {
        let mut rt = ServerRuntime::new();
        rt.start();
        let frame = rt.crash("panic");
        assert_eq!(rt.phase, ServerPhase::Stopped);
        assert!(frame.panic_message.contains("panic"));
    }
}
