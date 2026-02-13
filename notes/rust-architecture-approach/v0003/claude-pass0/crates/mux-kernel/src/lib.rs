//! # mux-kernel
//!
//! Single-threaded Sans-IO kernel for TermForge.
//!
//! The kernel owns all mutable state and processes events via a pure reducer
//! function: `process_event(event) -> Vec<Effect>`. The kernel never performs I/O.
//!
//! ## Module Organization
//! - [`session`]: Session entity and management.
//! - [`window`]: Window entity and management.
//! - [`pane`]: Pane entity with grid and parser.
//! - [`layout`]: Binary-tree layout engine (5 built-in + custom string parsing).
//! - [`copy_mode`]: Vi/Emacs copy mode state machine.
//! - [`event`]: KernelEvent / KernelEffect types.

#![forbid(unsafe_code)]

pub mod session;
pub mod window;
pub mod pane;
pub mod layout;
pub mod copy_mode;
pub mod event;

pub use event::{KernelEvent, KernelEffect};

use mux_types::{SessionId, WindowId, PaneId, ClientId, Size};
use mux_types::id::IdGenerator;
use mux_time::Clock;

/// The Sans-IO kernel. Owns all multiplexer state.
///
/// The kernel is strictly single-threaded. All mutations happen through
/// `process_event()`, and all I/O requirements are communicated through
/// the returned `Vec<KernelEffect>`.
pub struct Kernel {
    /// ID generator for all entities.
    id_gen: IdGenerator,
    /// Active sessions.
    sessions: Vec<session::Session>,
    /// Active clients.
    clients: Vec<Client>,
    /// Injectable clock for deterministic testing.
    clock: Clock,
    /// Server-level options.
    server_options: mux_options::OptionTable,
}

/// A connected client.
#[derive(Debug)]
pub struct Client {
    /// Client identifier.
    pub id: ClientId,
    /// Attached session (if any).
    pub attached_session: Option<SessionId>,
    /// Client terminal size.
    pub size: Size,
}

impl Kernel {
    /// Create a new kernel with the given clock.
    #[must_use]
    pub fn new(clock: Clock) -> Self {
        let mut server_options = mux_options::builtin_defaults();
        // INV-220: graphics passthrough disabled by default
        server_options.set(
            "allow-passthrough",
            mux_options::OptionValue::Bool(false),
        );

        Self {
            id_gen: IdGenerator::new(),
            sessions: Vec::new(),
            clients: Vec::new(),
            clock,
            server_options,
        }
    }

    /// Process a kernel event and return effects.
    ///
    /// This is the core reducer function. It is pure: no I/O, no side effects
    /// beyond mutating `self`.
    pub fn process_event(&mut self, event: KernelEvent) -> Vec<KernelEffect> {
        match event {
            KernelEvent::Command { client_id, command } => {
                self.handle_command(client_id, &command)
            }
            KernelEvent::PtyOutput { pane_id, data } => {
                self.handle_pty_output(pane_id, &data)
            }
            KernelEvent::ClientResize { client_id, size } => {
                self.handle_client_resize(client_id, size)
            }
            KernelEvent::PaneExited { pane_id, status } => {
                self.handle_pane_exited(pane_id, status)
            }
            KernelEvent::ClientConnected { client_id, size } => {
                self.handle_client_connected(client_id, size)
            }
            KernelEvent::ClientDisconnected { client_id } => {
                self.handle_client_disconnected(client_id)
            }
            KernelEvent::Tick => {
                self.handle_tick()
            }
        }
    }

    /// Create a new session and return its ID.
    pub fn create_session(&mut self, name: &str, size: Size) -> (SessionId, Vec<KernelEffect>) {
        let session_id = self.id_gen.next_session();
        let window_id = self.id_gen.next_window();
        let pane_id = self.id_gen.next_pane();

        let pane = pane::Pane::new(pane_id, size);
        let window = window::Window::new(window_id, name, pane);
        let session = session::Session::new(session_id, name.to_owned(), window);
        self.sessions.push(session);

        let effects = vec![
            KernelEffect::SpawnChild {
                pane_id,
                shell: "/bin/sh".into(),
            },
        ];
        (session_id, effects)
    }

    /// Find a session by name.
    #[must_use]
    pub fn find_session(&self, name: &str) -> Option<&session::Session> {
        self.sessions.iter().find(|s| s.name == name)
    }

    /// Number of active sessions.
    #[must_use]
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Number of connected clients.
    #[must_use]
    pub fn client_count(&self) -> usize {
        self.clients.len()
    }

    /// Get the clock.
    #[must_use]
    pub fn clock(&self) -> &Clock {
        &self.clock
    }

    fn handle_command(&mut self, client_id: ClientId, command: &str) -> Vec<KernelEffect> {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return vec![KernelEffect::CommandResponse {
                client_id,
                response: "empty command".into(),
            }];
        }

        match parts[0] {
            "new-session" => {
                let name = parts.get(2).copied().unwrap_or("default");
                let size = Size::new(80, 24);
                let (_session_id, mut effects) = self.create_session(name, size);
                effects.push(KernelEffect::CommandResponse {
                    client_id,
                    response: format!("session created: {name}"),
                });
                effects
            }
            "kill-server" => {
                vec![KernelEffect::Shutdown]
            }
            _ => {
                vec![KernelEffect::CommandResponse {
                    client_id,
                    response: format!("unknown command: {}", parts[0]),
                }]
            }
        }
    }

    fn handle_pty_output(&mut self, pane_id: PaneId, data: &[u8]) -> Vec<KernelEffect> {
        // Find the pane and feed data to its parser + grid.
        for session in &mut self.sessions {
            for window in &mut session.windows {
                for pane in &mut window.panes {
                    if pane.id == pane_id {
                        pane.process_output(data);
                        return vec![KernelEffect::Render];
                    }
                }
            }
        }
        Vec::new()
    }

    fn handle_client_resize(&mut self, client_id: ClientId, size: Size) -> Vec<KernelEffect> {
        for client in &mut self.clients {
            if client.id == client_id {
                client.size = size;
                break;
            }
        }
        // Trigger re-layout and render
        vec![KernelEffect::Render]
    }

    fn handle_pane_exited(&mut self, pane_id: PaneId, _status: i32) -> Vec<KernelEffect> {
        // Mark the pane as exited (simplified: just mark it)
        for session in &mut self.sessions {
            for window in &mut session.windows {
                for pane in &mut window.panes {
                    if pane.id == pane_id {
                        pane.exited = true;
                    }
                }
            }
        }
        Vec::new()
    }

    fn handle_client_connected(&mut self, client_id: ClientId, size: Size) -> Vec<KernelEffect> {
        self.clients.push(Client {
            id: client_id,
            attached_session: self.sessions.first().map(|s| s.id),
            size,
        });
        vec![KernelEffect::Render]
    }

    fn handle_client_disconnected(&mut self, client_id: ClientId) -> Vec<KernelEffect> {
        self.clients.retain(|c| c.id != client_id);
        Vec::new()
    }

    fn handle_tick(&mut self) -> Vec<KernelEffect> {
        // Periodic maintenance: escape-time checks, status updates, etc.
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kernel_new() {
        let kernel = Kernel::new(Clock::manual());
        assert_eq!(kernel.session_count(), 0);
        assert_eq!(kernel.client_count(), 0);
    }

    #[test]
    fn create_session() {
        let mut kernel = Kernel::new(Clock::manual());
        let (id, effects) = kernel.create_session("test", Size::new(80, 24));
        assert_eq!(kernel.session_count(), 1);
        assert!(id.raw() > 0);
        // Should have a SpawnChild effect
        assert!(effects.iter().any(|e| matches!(e, KernelEffect::SpawnChild { .. })));
    }

    #[test]
    fn find_session() {
        let mut kernel = Kernel::new(Clock::manual());
        kernel.create_session("main", Size::new(80, 24));
        assert!(kernel.find_session("main").is_some());
        assert!(kernel.find_session("nonexistent").is_none());
    }

    #[test]
    fn process_new_session_command() {
        let mut kernel = Kernel::new(Clock::manual());
        let effects = kernel.process_event(KernelEvent::Command {
            client_id: ClientId::new(1),
            command: "new-session -s mysession".into(),
        });
        assert_eq!(kernel.session_count(), 1);
        assert!(!effects.is_empty());
    }

    #[test]
    fn process_kill_server() {
        let mut kernel = Kernel::new(Clock::manual());
        let effects = kernel.process_event(KernelEvent::Command {
            client_id: ClientId::new(1),
            command: "kill-server".into(),
        });
        assert!(effects.iter().any(|e| matches!(e, KernelEffect::Shutdown)));
    }

    #[test]
    fn process_unknown_command() {
        let mut kernel = Kernel::new(Clock::manual());
        let effects = kernel.process_event(KernelEvent::Command {
            client_id: ClientId::new(1),
            command: "nonexistent-cmd".into(),
        });
        assert!(effects.iter().any(|e| matches!(e, KernelEffect::CommandResponse { .. })));
    }

    #[test]
    fn client_connect_disconnect() {
        let mut kernel = Kernel::new(Clock::manual());
        kernel.process_event(KernelEvent::ClientConnected {
            client_id: ClientId::new(1),
            size: Size::new(80, 24),
        });
        assert_eq!(kernel.client_count(), 1);
        kernel.process_event(KernelEvent::ClientDisconnected {
            client_id: ClientId::new(1),
        });
        assert_eq!(kernel.client_count(), 0);
    }

    #[test]
    fn pty_output_triggers_render() {
        let mut kernel = Kernel::new(Clock::manual());
        let (_, _) = kernel.create_session("test", Size::new(80, 24));
        // Find the pane ID (it's the third ID generated: session=1, window=2, pane=3)
        let pane_id = PaneId::new(3);
        let effects = kernel.process_event(KernelEvent::PtyOutput {
            pane_id,
            data: b"Hello".to_vec(),
        });
        assert!(effects.iter().any(|e| matches!(e, KernelEffect::Render)));
    }

    #[test]
    fn client_resize_triggers_render() {
        let mut kernel = Kernel::new(Clock::manual());
        kernel.process_event(KernelEvent::ClientConnected {
            client_id: ClientId::new(1),
            size: Size::new(80, 24),
        });
        let effects = kernel.process_event(KernelEvent::ClientResize {
            client_id: ClientId::new(1),
            size: Size::new(120, 40),
        });
        assert!(effects.iter().any(|e| matches!(e, KernelEffect::Render)));
    }

    #[test]
    fn tick_returns_no_effects() {
        let mut kernel = Kernel::new(Clock::manual());
        let effects = kernel.process_event(KernelEvent::Tick);
        assert!(effects.is_empty());
    }
}
