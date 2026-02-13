//! The Kernel struct and process_event reducer.

use crate::entity::{Client, Pane, Session, Window};
use crate::event::{KernelEffect, KernelEvent};
use mux_cmd_parse;
use mux_time::Clock;
use mux_types::{ClientId, PaneId, SessionId, Size, WindowId};

/// The TermForge kernel: single-threaded state machine.
///
/// All mutable multiplexer state lives here. The IO thread sends events
/// via channel; the kernel processes them synchronously and returns effects
/// for the IO thread to execute.
///
/// This is the Sans-IO pattern: the kernel never does I/O directly.
pub struct Kernel {
    clock: Clock,
    sessions: Vec<Session>,
    windows: Vec<Window>,
    panes: Vec<Pane>,
    clients: Vec<Client>,
    next_session_id: u64,
    next_window_id: u64,
    next_pane_id: u64,
    next_client_id: u64,
    default_size: Size,
    history_limit: u32,
}

impl Kernel {
    /// Create a new kernel with the given clock.
    #[must_use]
    pub fn new(clock: Clock) -> Self {
        Self {
            clock,
            sessions: Vec::new(),
            windows: Vec::new(),
            panes: Vec::new(),
            clients: Vec::new(),
            next_session_id: 1,
            next_window_id: 1,
            next_pane_id: 1,
            next_client_id: 1,
            default_size: Size::new(80, 24),
            history_limit: 10_000,
        }
    }

    /// Number of sessions.
    #[must_use]
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Number of windows.
    #[must_use]
    pub fn window_count(&self) -> usize {
        self.windows.len()
    }

    /// Number of panes.
    #[must_use]
    pub fn pane_count(&self) -> usize {
        self.panes.len()
    }

    /// Get a reference to the clock.
    #[must_use]
    pub const fn clock(&self) -> &Clock {
        &self.clock
    }

    /// Process a single event, returning a list of effects.
    ///
    /// This is the core reducer: `(State, Event) -> (State', Vec<Effect>)`.
    pub fn process_event(&mut self, event: KernelEvent) -> Vec<KernelEffect> {
        match event {
            KernelEvent::Command { client, command } => {
                self.handle_command(client, &command)
            }
            KernelEvent::PtyOutput { pane, data } => {
                self.handle_pty_output(pane, &data)
            }
            KernelEvent::ClientResize { client, size } => {
                self.handle_client_resize(client, size)
            }
            KernelEvent::PaneExited { pane, exit_code } => {
                self.handle_pane_exited(pane, exit_code)
            }
            KernelEvent::ClientConnected { client, size } => {
                self.handle_client_connected(client, size)
            }
            KernelEvent::ClientDisconnected { client } => {
                self.handle_client_disconnected(client)
            }
            KernelEvent::Tick => {
                Vec::new()
            }
        }
    }

    fn handle_command(&mut self, client: ClientId, command: &str) -> Vec<KernelEffect> {
        let parsed = match mux_cmd_parse::parse_command(command) {
            Ok(cmd) => cmd,
            Err(e) => {
                return vec![KernelEffect::CommandResponse {
                    client,
                    success: false,
                    output: e.to_string(),
                }];
            }
        };

        match parsed.name.as_str() {
            "new-session" => self.cmd_new_session(client, &parsed),
            "new-window" => self.cmd_new_window(client, &parsed),
            "split-window" => self.cmd_split_window(client, &parsed),
            "kill-session" => self.cmd_kill_session(client, &parsed),
            "kill-pane" => self.cmd_kill_pane(client, &parsed),
            _ => {
                vec![KernelEffect::CommandResponse {
                    client,
                    success: false,
                    output: format!("unknown command: {}", parsed.name),
                }]
            }
        }
    }

    fn cmd_new_session(
        &mut self,
        client: ClientId,
        parsed: &mux_cmd_parse::Command,
    ) -> Vec<KernelEffect> {
        let mut name = format!("session-{}", self.next_session_id);

        // Extract -s flag for session name
        for arg in &parsed.args {
            if let mux_cmd_parse::Argument::FlagValue('s', val) = arg {
                name = val.clone();
            }
        }

        let session_id = SessionId(self.next_session_id);
        self.next_session_id += 1;

        let wall = self.clock.now_wall().as_millis();
        let mut session = Session::new(session_id, name.clone(), wall);

        // Create a default window and pane
        let window_id = WindowId(self.next_window_id);
        self.next_window_id += 1;
        let pane_id = PaneId(self.next_pane_id);
        self.next_pane_id += 1;

        let mut window = Window::new(window_id, session_id, name.clone(), 0, self.default_size);
        let pane = Pane::new(pane_id, window_id, self.default_size, self.history_limit);

        window.panes.push(pane_id);
        window.active_pane = Some(pane_id);
        session.windows.push(window_id);
        session.active_window = Some(window_id);

        self.sessions.push(session);
        self.windows.push(window);
        self.panes.push(pane);

        let mut effects = vec![KernelEffect::CommandResponse {
            client,
            success: true,
            output: format!("created session {name}"),
        }];

        // Check if -d flag is NOT present (attach)
        let detached = parsed.args.iter().any(|a| matches!(a, mux_cmd_parse::Argument::Flag('d')));
        if !detached {
            effects.push(KernelEffect::SpawnChild {
                pane: pane_id,
                shell: "/bin/sh".into(),
                size: self.default_size,
            });
        }

        effects
    }

    fn cmd_new_window(
        &mut self,
        client: ClientId,
        _parsed: &mux_cmd_parse::Command,
    ) -> Vec<KernelEffect> {
        if let Some(session) = self.sessions.first_mut() {
            let window_id = WindowId(self.next_window_id);
            self.next_window_id += 1;
            let pane_id = PaneId(self.next_pane_id);
            self.next_pane_id += 1;

            let index = session.windows.len() as u32;
            let mut window = Window::new(window_id, session.id, format!("window-{index}"), index, self.default_size);
            let pane = Pane::new(pane_id, window_id, self.default_size, self.history_limit);

            window.panes.push(pane_id);
            window.active_pane = Some(pane_id);
            session.windows.push(window_id);
            session.active_window = Some(window_id);

            self.windows.push(window);
            self.panes.push(pane);

            vec![KernelEffect::CommandResponse {
                client, success: true, output: "window created".into(),
            }]
        } else {
            vec![KernelEffect::CommandResponse {
                client, success: false, output: "no session".into(),
            }]
        }
    }

    fn cmd_split_window(
        &mut self,
        client: ClientId,
        _parsed: &mux_cmd_parse::Command,
    ) -> Vec<KernelEffect> {
        if let Some(window) = self.windows.first_mut() {
            let pane_id = PaneId(self.next_pane_id);
            self.next_pane_id += 1;
            let pane = Pane::new(pane_id, window.id, self.default_size, self.history_limit);
            window.panes.push(pane_id);
            self.panes.push(pane);

            vec![KernelEffect::CommandResponse {
                client, success: true, output: "pane created".into(),
            }]
        } else {
            vec![KernelEffect::CommandResponse {
                client, success: false, output: "no window".into(),
            }]
        }
    }

    fn cmd_kill_session(
        &mut self,
        client: ClientId,
        parsed: &mux_cmd_parse::Command,
    ) -> Vec<KernelEffect> {
        let mut target_name = None;
        for arg in &parsed.args {
            if let mux_cmd_parse::Argument::FlagValue('t', val) = arg {
                target_name = Some(val.clone());
            }
        }

        let name = target_name.unwrap_or_default();
        let idx = self.sessions.iter().position(|s| s.name == name);
        if let Some(idx) = idx {
            let session = self.sessions.remove(idx);
            // Remove associated windows and panes
            for wid in &session.windows {
                if let Some(widx) = self.windows.iter().position(|w| w.id == *wid) {
                    let window = self.windows.remove(widx);
                    for pid in &window.panes {
                        if let Some(pidx) = self.panes.iter().position(|p| p.id == *pid) {
                            self.panes.remove(pidx);
                        }
                    }
                }
            }
            vec![KernelEffect::CommandResponse {
                client, success: true, output: format!("killed session {name}"),
            }]
        } else {
            vec![KernelEffect::CommandResponse {
                client, success: false, output: format!("session not found: {name}"),
            }]
        }
    }

    fn cmd_kill_pane(
        &mut self,
        client: ClientId,
        _parsed: &mux_cmd_parse::Command,
    ) -> Vec<KernelEffect> {
        if let Some(pane) = self.panes.last() {
            let pane_id = pane.id;
            let window_id = pane.window_id;
            self.panes.retain(|p| p.id != pane_id);
            if let Some(window) = self.windows.iter_mut().find(|w| w.id == window_id) {
                window.panes.retain(|p| *p != pane_id);
            }
            vec![KernelEffect::CommandResponse {
                client, success: true, output: "pane killed".into(),
            }]
        } else {
            vec![KernelEffect::CommandResponse {
                client, success: false, output: "no panes".into(),
            }]
        }
    }

    fn handle_pty_output(&mut self, pane_id: PaneId, data: &[u8]) -> Vec<KernelEffect> {
        if let Some(pane) = self.panes.iter_mut().find(|p| p.id == pane_id) {
            let actions = pane.parser.feed(data);
            for action in &actions {
                if let mux_parser::VtAction::Print(ch) = action {
                    let gid = pane.arena.intern(&ch.to_string());
                    let cell = mux_types::Cell {
                        grapheme: gid,
                        width: 1,
                        ..mux_types::Cell::empty()
                    };
                    if let Some(line) = pane.grid.active_line(pane.cursor_y) {
                        let _ = line;
                    }
                    pane.cursor_x += 1;
                    if pane.cursor_x >= pane.size.cols {
                        pane.cursor_x = 0;
                        pane.cursor_y += 1;
                    }
                    let _ = cell;
                }
            }
            pane.grid.mark_dirty(pane.cursor_y);
        }
        Vec::new()
    }

    fn handle_client_resize(&mut self, client_id: ClientId, size: Size) -> Vec<KernelEffect> {
        if let Some(client) = self.clients.iter_mut().find(|c| c.id == client_id) {
            client.size = size;
        }
        vec![KernelEffect::Render { client: client_id }]
    }

    fn handle_pane_exited(&mut self, pane_id: PaneId, _exit_code: i32) -> Vec<KernelEffect> {
        if let Some(pane) = self.panes.iter_mut().find(|p| p.id == pane_id) {
            pane.pid = None;
        }
        Vec::new()
    }

    fn handle_client_connected(&mut self, client_id: ClientId, size: Size) -> Vec<KernelEffect> {
        let client = Client::new(client_id, format!("client-{}", client_id.0), size);
        self.clients.push(client);
        vec![KernelEffect::Render { client: client_id }]
    }

    fn handle_client_disconnected(&mut self, client_id: ClientId) -> Vec<KernelEffect> {
        self.clients.retain(|c| c.id != client_id);
        // Detach from sessions
        for session in &mut self.sessions {
            session.attached_clients.retain(|c| *c != client_id);
        }
        Vec::new()
    }
}

impl std::fmt::Debug for Kernel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Kernel")
            .field("sessions", &self.sessions.len())
            .field("windows", &self.windows.len())
            .field("panes", &self.panes.len())
            .field("clients", &self.clients.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_kernel() -> Kernel {
        Kernel::new(Clock::manual(0, 1_000_000_000))
    }

    #[test]
    fn new_kernel_empty() {
        let k = test_kernel();
        assert_eq!(k.session_count(), 0);
        assert_eq!(k.window_count(), 0);
        assert_eq!(k.pane_count(), 0);
    }

    #[test]
    fn create_session() {
        let mut k = test_kernel();
        let effects = k.process_event(KernelEvent::Command {
            client: ClientId(0),
            command: "new-session -d -s test".into(),
        });
        assert_eq!(k.session_count(), 1);
        assert_eq!(k.window_count(), 1);
        assert_eq!(k.pane_count(), 1);
        assert!(effects.iter().any(|e| matches!(e, KernelEffect::CommandResponse { success: true, .. })));
    }

    #[test]
    fn create_multiple_sessions() {
        let mut k = test_kernel();
        for name in &["s1", "s2", "s3"] {
            k.process_event(KernelEvent::Command {
                client: ClientId(0),
                command: format!("new-session -d -s {name}"),
            });
        }
        assert_eq!(k.session_count(), 3);
    }

    #[test]
    fn kill_session() {
        let mut k = test_kernel();
        k.process_event(KernelEvent::Command {
            client: ClientId(0),
            command: "new-session -d -s test".into(),
        });
        assert_eq!(k.session_count(), 1);
        k.process_event(KernelEvent::Command {
            client: ClientId(0),
            command: "kill-session -t test".into(),
        });
        assert_eq!(k.session_count(), 0);
    }

    #[test]
    fn unknown_command() {
        let mut k = test_kernel();
        let effects = k.process_event(KernelEvent::Command {
            client: ClientId(0),
            command: "frobnicate".into(),
        });
        assert!(effects.iter().any(|e| matches!(e, KernelEffect::CommandResponse { success: false, .. })));
    }

    #[test]
    fn new_window() {
        let mut k = test_kernel();
        k.process_event(KernelEvent::Command {
            client: ClientId(0),
            command: "new-session -d -s test".into(),
        });
        k.process_event(KernelEvent::Command {
            client: ClientId(0),
            command: "new-window".into(),
        });
        assert_eq!(k.window_count(), 2);
    }

    #[test]
    fn split_window() {
        let mut k = test_kernel();
        k.process_event(KernelEvent::Command {
            client: ClientId(0),
            command: "new-session -d -s test".into(),
        });
        k.process_event(KernelEvent::Command {
            client: ClientId(0),
            command: "split-window".into(),
        });
        assert_eq!(k.pane_count(), 2);
    }

    #[test]
    fn client_connect_disconnect() {
        let mut k = test_kernel();
        k.process_event(KernelEvent::ClientConnected {
            client: ClientId(1), size: Size::new(80, 24),
        });
        assert_eq!(k.clients.len(), 1);
        k.process_event(KernelEvent::ClientDisconnected {
            client: ClientId(1),
        });
        assert_eq!(k.clients.len(), 0);
    }

    #[test]
    fn client_resize() {
        let mut k = test_kernel();
        k.process_event(KernelEvent::ClientConnected {
            client: ClientId(1), size: Size::new(80, 24),
        });
        let effects = k.process_event(KernelEvent::ClientResize {
            client: ClientId(1), size: Size::new(120, 40),
        });
        assert!(effects.iter().any(|e| matches!(e, KernelEffect::Render { .. })));
    }

    #[test]
    fn pane_exited() {
        let mut k = test_kernel();
        k.process_event(KernelEvent::Command {
            client: ClientId(0),
            command: "new-session -d -s test".into(),
        });
        k.process_event(KernelEvent::PaneExited {
            pane: PaneId(1), exit_code: 0,
        });
    }

    #[test]
    fn tick_is_noop() {
        let mut k = test_kernel();
        let effects = k.process_event(KernelEvent::Tick);
        assert!(effects.is_empty());
    }
}
