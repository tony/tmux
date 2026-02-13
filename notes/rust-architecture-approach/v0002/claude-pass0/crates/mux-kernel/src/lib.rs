//! # mux-kernel
//!
//! The single-threaded kernel that owns all mutable state.
//!
//! ## Architecture (v25 DEFINITIVE)
//! - Runs on `std::thread` (not tokio) -- the only thread that mutates state.
//! - Receives events from IO thread via bounded channels.
//! - Sends effects (snapshots, commands) to IO and Render threads.
//! - Sans-IO design: all IO goes through channels, never direct syscalls.
//!
//! ## Entity Model
//! - Sessions, windows, panes stored in `SlotMap`s with typed keys.
//! - Each pane owns a `ChunkedGrid` and `GraphemeArena`.
//! - Global revision counter for render optimization.
//!
//! ## Key Subsystems (7,600 LOC target)
//! - Key binding dispatch (~500 LOC)
//! - Environment inheritance (~150 LOC)
//! - Buffer management (~200 LOC)
//! - Mouse dispatch (~500 LOC)
//! - Alert system (~150 LOC)
//! - Scroll/reflow (~600 LOC)
//! - Layout engine (~800 LOC)
//! - Command execution (~1,000 LOC)

#![forbid(unsafe_code)]

use std::collections::HashMap;

use mux_grapheme_arena::GraphemeArena;
use mux_grid::ChunkedGrid;
use mux_options::OptionTable;
use mux_time::Clock;
use mux_types::{
    ClientId, KeyEvent, MouseEvent, PaneId, SessionId, Size, WindowId,
};

/// Events received by the kernel from the IO thread.
#[derive(Debug)]
pub enum KernelEvent {
    /// PTY data from a pane.
    PtyData { pane: PaneId, data: Vec<u8> },
    /// Key input from a client.
    KeyInput { client: ClientId, event: KeyEvent },
    /// Mouse input from a client.
    MouseInput { client: ClientId, event: MouseEvent },
    /// Client connected.
    ClientConnect { client: ClientId, size: Size },
    /// Client disconnected.
    ClientDisconnect { client: ClientId },
    /// Client resized.
    ClientResize { client: ClientId, size: Size },
    /// Command request (from client or config).
    Command { client: ClientId, command: String },
    /// Timer tick.
    Tick,
    /// Shutdown request.
    Shutdown,
}

/// Effects emitted by the kernel to IO/Render threads.
#[derive(Debug)]
pub enum KernelEffect {
    /// Write data to a pane's PTY.
    PtyWrite { pane: PaneId, data: Vec<u8> },
    /// Resize a pane's PTY.
    PtyResize { pane: PaneId, size: Size },
    /// Spawn a new PTY process.
    PtySpawn {
        pane: PaneId,
        program: String,
        args: Vec<String>,
        env: Vec<(String, String)>,
        size: Size,
    },
    /// Close a pane's PTY.
    PtyClose { pane: PaneId },
    /// Send a render snapshot to the render thread.
    RenderSnapshot {
        revision: u64,
        // Snapshot data would go here; using revision as handle.
    },
    /// Send output to a client.
    ClientOutput { client: ClientId, data: Vec<u8> },
    /// Detach a client.
    ClientDetach { client: ClientId },
    /// Command response.
    CommandResponse {
        client: ClientId,
        output: String,
        success: bool,
    },
    /// Request shutdown.
    Shutdown,
}

/// A session in the entity model.
#[derive(Debug)]
pub struct Session {
    pub id: SessionId,
    pub name: String,
    pub windows: Vec<WindowId>,
    pub active_window: Option<WindowId>,
    pub options: OptionTable,
    pub env: HashMap<String, String>,
    pub created_at: i64,
}

/// A window in the entity model.
#[derive(Debug)]
pub struct Window {
    pub id: WindowId,
    pub session: SessionId,
    pub name: String,
    pub index: u32,
    pub panes: Vec<PaneId>,
    pub active_pane: Option<PaneId>,
    pub options: OptionTable,
    /// Layout type for this window.
    pub layout: LayoutKind,
}

/// Window layout types matching tmux.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutKind {
    EvenHorizontal,
    EvenVertical,
    MainHorizontal,
    MainVertical,
    Tiled,
    Custom,
}

impl Default for LayoutKind {
    fn default() -> Self {
        Self::EvenVertical
    }
}

/// A pane in the entity model.
#[derive(Debug)]
pub struct Pane {
    pub id: PaneId,
    pub window: WindowId,
    pub grid: ChunkedGrid,
    pub arena: GraphemeArena,
    pub size: Size,
    pub pid: Option<u32>,
    pub title: String,
    pub options: OptionTable,
    /// Cursor position (row, col) within the grid viewport.
    pub cursor_row: u32,
    pub cursor_col: u32,
    /// Whether the cursor is visible.
    pub cursor_visible: bool,
}

/// A connected client.
#[derive(Debug)]
pub struct Client {
    pub id: ClientId,
    pub session: Option<SessionId>,
    pub name: String,
    pub size: Size,
}

/// Key binding definition.
#[derive(Debug, Clone)]
pub struct KeyBinding {
    /// The key that triggers this binding.
    pub key: KeyEvent,
    /// Whether this is a root table binding (no prefix needed).
    pub is_root: bool,
    /// Command to execute.
    pub command: String,
}

/// The kernel: single-threaded owner of all mutable state.
///
/// ## Threading Model
/// The kernel runs on its own `std::thread`. It receives events from
/// the IO thread via bounded channels and emits effects back.
/// No async, no locks, no shared mutable state.
pub struct Kernel {
    /// All sessions.
    sessions: HashMap<SessionId, Session>,
    /// All windows.
    windows: HashMap<WindowId, Window>,
    /// All panes.
    panes: HashMap<PaneId, Pane>,
    /// All clients.
    clients: HashMap<ClientId, Client>,
    /// Global revision counter.
    revision: u64,
    /// Server-level options.
    server_options: OptionTable,
    /// Server-level environment.
    server_env: HashMap<String, String>,
    /// Key bindings.
    key_bindings: Vec<KeyBinding>,
    /// Paste buffers.
    buffers: Vec<String>,
    /// Clock (injectable for testing).
    clock: Clock,
    /// ID counters.
    next_session_id: u64,
    next_window_id: u64,
    next_pane_id: u64,
    next_client_id: u64,
}

impl Kernel {
    /// Create a new kernel with the given clock.
    #[must_use]
    pub fn new(clock: Clock) -> Self {
        Self {
            sessions: HashMap::new(),
            windows: HashMap::new(),
            panes: HashMap::new(),
            clients: HashMap::new(),
            revision: 0,
            server_options: OptionTable::new(),
            server_env: HashMap::new(),
            key_bindings: Vec::new(),
            buffers: Vec::new(),
            clock,
            next_session_id: 1,
            next_window_id: 1,
            next_pane_id: 1,
            next_client_id: 1,
        }
    }

    /// Process a single event, returning effects.
    pub fn process_event(&mut self, event: KernelEvent) -> Vec<KernelEffect> {
        match event {
            KernelEvent::PtyData { pane, data } => self.handle_pty_data(pane, data),
            KernelEvent::KeyInput { client, event } => self.handle_key_input(client, event),
            KernelEvent::MouseInput { client, event } => self.handle_mouse_input(client, event),
            KernelEvent::ClientConnect { client, size } => self.handle_client_connect(client, size),
            KernelEvent::ClientDisconnect { client } => self.handle_client_disconnect(client),
            KernelEvent::ClientResize { client, size } => self.handle_client_resize(client, size),
            KernelEvent::Command { client, command } => self.handle_command(client, &command),
            KernelEvent::Tick => self.handle_tick(),
            KernelEvent::Shutdown => vec![KernelEffect::Shutdown],
        }
    }

    /// Current revision.
    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    /// Increment and return the new revision.
    fn next_revision(&mut self) -> u64 {
        self.revision += 1;
        self.revision
    }

    /// Allocate a new session ID.
    fn alloc_session_id(&mut self) -> SessionId {
        let id = SessionId(self.next_session_id);
        self.next_session_id += 1;
        id
    }

    /// Allocate a new window ID.
    fn alloc_window_id(&mut self) -> WindowId {
        let id = WindowId(self.next_window_id);
        self.next_window_id += 1;
        id
    }

    /// Allocate a new pane ID.
    fn alloc_pane_id(&mut self) -> PaneId {
        let id = PaneId(self.next_pane_id);
        self.next_pane_id += 1;
        id
    }

    /// Number of sessions.
    #[must_use]
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Number of panes.
    #[must_use]
    pub fn pane_count(&self) -> usize {
        self.panes.len()
    }

    // --- Event handlers ---

    fn handle_pty_data(&mut self, pane_id: PaneId, data: Vec<u8>) -> Vec<KernelEffect> {
        if let Some(pane) = self.panes.get_mut(&pane_id) {
            // Feed data through VT parser -> grid mutations
            let mut parser = mux_parser::VtParser::new();
            let actions = parser.feed(&data);
            for action in actions {
                match action {
                    mux_parser::VtAction::Print(ch) => {
                        let grapheme_id = pane.arena.intern(&ch.to_string());
                        let width = if ch.is_ascii() { 1 } else { 1 }; // simplified
                        let cell = mux_types::Cell {
                            grapheme: grapheme_id,
                            width,
                            ..mux_types::Cell::empty()
                        };
                        if let Some(line) = pane.grid.line_mut(pane.cursor_row) {
                            line.set_cell(pane.cursor_col, cell);
                        }
                        pane.cursor_col += u32::from(width);
                        if pane.cursor_col >= pane.size.cols {
                            pane.cursor_col = 0;
                            pane.cursor_row += 1;
                        }
                    }
                    mux_parser::VtAction::Execute(0x0A) => {
                        // Line feed
                        pane.cursor_row += 1;
                        if pane.cursor_row >= pane.size.rows {
                            pane.grid.scroll_region(0, pane.size.rows - 1, mux_types::Colour::Default);
                            pane.cursor_row = pane.size.rows - 1;
                        }
                    }
                    mux_parser::VtAction::Execute(0x0D) => {
                        // Carriage return
                        pane.cursor_col = 0;
                    }
                    _ => {
                        // Other actions handled in full implementation
                    }
                }
            }
            self.next_revision();
        }
        Vec::new()
    }

    fn handle_key_input(&mut self, _client: ClientId, _event: KeyEvent) -> Vec<KernelEffect> {
        // Key binding lookup and dispatch
        // Full implementation in P1 (~500 LOC)
        Vec::new()
    }

    fn handle_mouse_input(&mut self, _client: ClientId, _event: MouseEvent) -> Vec<KernelEffect> {
        // Mouse dispatch: pane selection, scroll, drag
        // Full implementation in P2a (~500 LOC)
        Vec::new()
    }

    fn handle_client_connect(&mut self, client_id: ClientId, size: Size) -> Vec<KernelEffect> {
        self.clients.insert(client_id, Client {
            id: client_id,
            session: None,
            name: format!("client-{}", client_id.0),
            size,
        });
        Vec::new()
    }

    fn handle_client_disconnect(&mut self, client_id: ClientId) -> Vec<KernelEffect> {
        self.clients.remove(&client_id);
        Vec::new()
    }

    fn handle_client_resize(&mut self, client_id: ClientId, size: Size) -> Vec<KernelEffect> {
        if let Some(client) = self.clients.get_mut(&client_id) {
            client.size = size;
        }
        // Recalculate window sizes for clients attached to same session
        Vec::new()
    }

    fn handle_command(&mut self, client: ClientId, command: &str) -> Vec<KernelEffect> {
        match mux_cmd_parse::parse_command(command) {
            Ok(cmd) => {
                let result = self.execute_command(&cmd);
                let success = result.is_ok();
                let output = result.unwrap_or_else(|e| e);
                vec![KernelEffect::CommandResponse {
                    client,
                    output,
                    success,
                }]
            }
            Err(e) => {
                vec![KernelEffect::CommandResponse {
                    client,
                    output: e.to_string(),
                    success: false,
                }]
            }
        }
    }

    fn handle_tick(&mut self) -> Vec<KernelEffect> {
        // Periodic tasks: alert checking, status line refresh
        Vec::new()
    }

    /// Execute a parsed command.
    fn execute_command(&mut self, cmd: &mux_cmd_parse::Command) -> Result<String, String> {
        match cmd.name.as_str() {
            "new-session" => {
                let session_id = self.alloc_session_id();
                let window_id = self.alloc_window_id();
                let pane_id = self.alloc_pane_id();

                let name = cmd.args.iter().find_map(|a| {
                    if let mux_cmd_parse::Argument::FlagValue('s', v) = a {
                        Some(v.clone())
                    } else {
                        None
                    }
                }).unwrap_or_else(|| format!("{}", session_id.0));

                let size = Size::new(80, 24);

                // Create pane
                self.panes.insert(pane_id, Pane {
                    id: pane_id,
                    window: window_id,
                    grid: ChunkedGrid::new(size.cols, size.rows, 10_000),
                    arena: GraphemeArena::new(),
                    size,
                    pid: None,
                    title: String::new(),
                    options: OptionTable::new(),
                    cursor_row: 0,
                    cursor_col: 0,
                    cursor_visible: true,
                });

                // Create window
                self.windows.insert(window_id, Window {
                    id: window_id,
                    session: session_id,
                    name: "bash".into(),
                    index: 0,
                    panes: vec![pane_id],
                    active_pane: Some(pane_id),
                    options: OptionTable::new(),
                    layout: LayoutKind::default(),
                });

                // Create session
                self.sessions.insert(session_id, Session {
                    id: session_id,
                    name: name.clone(),
                    windows: vec![window_id],
                    active_window: Some(window_id),
                    options: OptionTable::new(),
                    env: HashMap::new(),
                    created_at: self.clock.now_wall().as_millis(),
                });

                Ok(format!("${}", session_id.0))
            }
            "list-sessions" => {
                let lines: Vec<String> = self.sessions.values()
                    .map(|s| format!("{}: {} windows", s.name, s.windows.len()))
                    .collect();
                Ok(lines.join("\n"))
            }
            _ => Err(format!("unknown command: {}", cmd.name)),
        }
    }
}

impl std::fmt::Debug for Kernel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Kernel")
            .field("sessions", &self.sessions.len())
            .field("windows", &self.windows.len())
            .field("panes", &self.panes.len())
            .field("clients", &self.clients.len())
            .field("revision", &self.revision)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mux_time::Clock;

    #[test]
    fn kernel_creates_session() {
        let mut kernel = Kernel::new(Clock::manual(0, 0));
        let effects = kernel.handle_command(
            ClientId(1),
            "new-session -d -s test",
        );
        assert_eq!(kernel.session_count(), 1);
        assert!(!effects.is_empty());
    }

    #[test]
    fn kernel_lists_sessions() {
        let mut kernel = Kernel::new(Clock::manual(0, 0));
        let _ = kernel.handle_command(ClientId(1), "new-session -d -s demo");
        let effects = kernel.handle_command(ClientId(1), "list-sessions");
        assert!(effects.iter().any(|e| matches!(e,
            KernelEffect::CommandResponse { success: true, .. }
        )));
    }

    #[test]
    fn kernel_revision_increments() {
        let mut kernel = Kernel::new(Clock::manual(0, 0));
        assert_eq!(kernel.revision(), 0);
        kernel.next_revision();
        assert_eq!(kernel.revision(), 1);
    }

    #[test]
    fn kernel_client_connect_disconnect() {
        let mut kernel = Kernel::new(Clock::manual(0, 0));
        let _ = kernel.handle_client_connect(ClientId(1), Size::new(80, 24));
        assert_eq!(kernel.clients.len(), 1);
        let _ = kernel.handle_client_disconnect(ClientId(1));
        assert_eq!(kernel.clients.len(), 0);
    }
}
