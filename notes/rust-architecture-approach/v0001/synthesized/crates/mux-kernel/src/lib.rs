//! # mux-kernel
//!
//! Sans-IO kernel: Event/Effect reducer and ServerGraph for TermForge.
//!
//! ## Key Design
//! - Pure reducer: `Kernel::step(event) -> Vec<Effect>` with no I/O.
//! - `Event` enum: all external inputs to the multiplexer.
//! - `Effect` enum: all outputs / side-effect requests.
//! - `ServerGraph`: typed arena/slot-map for sessions, windows, panes.
//! - No `std::io` or `std::net` usage (L0 crate).
//!
//! ## Invariants
//! - INV-027: step() is a pure function (no side effects).
//! - INV-033: Deterministic under fixed seed and clock injection.

#![forbid(unsafe_code)]

use std::collections::HashMap;

/// Lightweight no-op OTEL span handle used at cross-crate boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OTelSpan {
    name: &'static str,
}

impl OTelSpan {
    /// Span name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        self.name
    }

    /// No-op enter helper for call-site compatibility.
    #[must_use]
    pub const fn entered(self) -> Self {
        self
    }
}

/// Create a no-op OTEL span.
#[must_use]
pub const fn otel_boundary_span(name: &'static str) -> OTelSpan {
    OTelSpan { name }
}

/// Create a boundary span with a concise macro form.
#[macro_export]
macro_rules! otel_span {
    ($name:expr) => {
        $crate::otel_boundary_span($name)
    };
}

/// Enter an OTEL scope for the current lexical block (no-op).
#[macro_export]
macro_rules! otel_scope {
    ($name:expr) => {
        let _span = $crate::otel_span!($name).entered();
    };
}

/// Ergonomic re-exports for downstream crates.
pub mod prelude {
    pub use super::{
        otel_boundary_span, Effect, Event, Generation, GenSlotMap, Kernel, LogLevel, OTelSpan,
        Pane, ServerGraph, Session, SlotKey, Window,
    };
}

// --- Slot Map / GenSlotMap ---

/// Generation counter for ABA prevention in slot-based addressing.
pub type Generation = u64;

/// A slot key combining an index and generation for ABA-safe addressing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SlotKey {
    pub index: u32,
    pub generation: Generation,
}

impl SlotKey {
    #[must_use]
    pub fn new(index: u32, generation: Generation) -> Self {
        Self { index, generation }
    }
}

/// A single slot in the GenSlotMap.
#[derive(Debug, Clone)]
enum Slot<T> {
    Occupied { value: T, generation: Generation },
    Vacant { generation: Generation },
}

/// A generational slot map for O(1) access with ABA prevention.
///
/// Used by ServerGraph to store sessions, windows, and panes.
#[derive(Debug, Clone)]
pub struct GenSlotMap<T> {
    slots: Vec<Slot<T>>,
    free_list: Vec<u32>,
    count: usize,
}

impl<T> GenSlotMap<T> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            free_list: Vec::new(),
            count: 0,
        }
    }

    /// Insert a value, returning its slot key.
    pub fn insert(&mut self, value: T) -> SlotKey {
        if let Some(index) = self.free_list.pop() {
            let slot = &mut self.slots[index as usize];
            let gen = match slot {
                Slot::Vacant { generation } => *generation + 1,
                Slot::Occupied { generation, .. } => *generation + 1,
            };
            *slot = Slot::Occupied { value, generation: gen };
            self.count += 1;
            SlotKey::new(index, gen)
        } else {
            let index = self.slots.len() as u32;
            let gen = 0;
            self.slots.push(Slot::Occupied { value, generation: gen });
            self.count += 1;
            SlotKey::new(index, gen)
        }
    }

    /// Get a reference to the value at the given key.
    #[must_use]
    pub fn get(&self, key: SlotKey) -> Option<&T> {
        match self.slots.get(key.index as usize)? {
            Slot::Occupied { value, generation } if *generation == key.generation => Some(value),
            _ => None,
        }
    }

    /// Get a mutable reference to the value at the given key.
    #[must_use]
    pub fn get_mut(&mut self, key: SlotKey) -> Option<&mut T> {
        match self.slots.get_mut(key.index as usize)? {
            Slot::Occupied { value, generation } if *generation == key.generation => Some(value),
            _ => None,
        }
    }

    /// Remove the value at the given key, returning it.
    pub fn remove(&mut self, key: SlotKey) -> Option<T> {
        let slot = self.slots.get_mut(key.index as usize)?;
        match slot {
            Slot::Occupied { generation, .. } if *generation == key.generation => {
                let gen = *generation;
                let old = std::mem::replace(slot, Slot::Vacant { generation: gen });
                self.free_list.push(key.index);
                self.count -= 1;
                match old {
                    Slot::Occupied { value, .. } => Some(value),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Number of occupied slots.
    #[must_use]
    pub fn len(&self) -> usize {
        self.count
    }

    /// True if no occupied slots.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Check if a key is valid (occupied with correct generation).
    #[must_use]
    pub fn contains_key(&self, key: SlotKey) -> bool {
        self.get(key).is_some()
    }
}

impl<T> Default for GenSlotMap<T> {
    fn default() -> Self {
        Self::new()
    }
}

// --- ServerGraph ---

/// A session in the server graph.
#[derive(Debug, Clone)]
pub struct Session {
    pub name: String,
    pub windows: Vec<SlotKey>,
}

/// A window in the server graph.
#[derive(Debug, Clone)]
pub struct Window {
    pub name: String,
    pub session: SlotKey,
    pub panes: Vec<SlotKey>,
}

/// A pane in the server graph.
#[derive(Debug, Clone)]
pub struct Pane {
    pub window: SlotKey,
    pub rows: u16,
    pub cols: u16,
}

/// The ServerGraph: hierarchical session -> window -> pane structure.
///
/// Uses GenSlotMap for O(1) access with generation-based ABA prevention.
#[derive(Debug, Clone)]
pub struct ServerGraph {
    pub sessions: GenSlotMap<Session>,
    pub windows: GenSlotMap<Window>,
    pub panes: GenSlotMap<Pane>,
}

impl ServerGraph {
    #[must_use]
    pub fn new() -> Self {
        Self {
            sessions: GenSlotMap::new(),
            windows: GenSlotMap::new(),
            panes: GenSlotMap::new(),
        }
    }

    /// Create a new session, returning its key.
    pub fn create_session(&mut self, name: &str) -> SlotKey {
        self.sessions.insert(Session {
            name: name.to_string(),
            windows: Vec::new(),
        })
    }

    /// Create a new window in a session, returning its key.
    pub fn create_window(&mut self, session_key: SlotKey, name: &str) -> Option<SlotKey> {
        let win_key = self.windows.insert(Window {
            name: name.to_string(),
            session: session_key,
            panes: Vec::new(),
        });
        let session = self.sessions.get_mut(session_key)?;
        session.windows.push(win_key);
        Some(win_key)
    }

    /// Create a new pane in a window, returning its key.
    pub fn create_pane(&mut self, window_key: SlotKey, rows: u16, cols: u16) -> Option<SlotKey> {
        let pane_key = self.panes.insert(Pane {
            window: window_key,
            rows,
            cols,
        });
        let window = self.windows.get_mut(window_key)?;
        window.panes.push(pane_key);
        Some(pane_key)
    }

    /// Destroy a pane and detach it from its parent window.
    pub fn destroy_pane(&mut self, pane_key: SlotKey) -> Option<Pane> {
        let pane = self.panes.remove(pane_key)?;
        if let Some(window) = self.windows.get_mut(pane.window) {
            window.panes.retain(|&k| k != pane_key);
        }
        Some(pane)
    }

    /// Destroy a window and all of its panes.
    pub fn destroy_window(&mut self, window_key: SlotKey) -> Option<Window> {
        let window = self.windows.get(window_key)?.clone();
        for pane_key in window.panes.clone() {
            let _ = self.destroy_pane(pane_key);
        }
        if let Some(session) = self.sessions.get_mut(window.session) {
            session.windows.retain(|&k| k != window_key);
        }
        self.windows.remove(window_key)
    }

    /// Destroy a session and recursively destroy all windows and panes below it.
    pub fn destroy_session(&mut self, session_key: SlotKey) -> Option<Session> {
        let session = self.sessions.get(session_key)?.clone();
        for window_key in session.windows.clone() {
            let _ = self.destroy_window(window_key);
        }
        self.sessions.remove(session_key)
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
}

impl Default for ServerGraph {
    fn default() -> Self {
        Self::new()
    }
}

// --- Event / Effect ---

/// All external inputs to the multiplexer kernel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// Input bytes from a client connection.
    ClientInput { client_id: u32, data: Vec<u8> },
    /// Output bytes from a PTY child process.
    PtyOutput { pane: SlotKey, data: Vec<u8> },
    /// A PTY child process exited.
    PtyExited { pane: SlotKey, exit_code: i32 },
    /// Timer tick (for status bar refresh, etc.).
    Tick { time_ms: u64 },
    /// Client requested a resize.
    Resize { pane: SlotKey, rows: u16, cols: u16 },
    /// Client requested a new session.
    CreateSession { name: String },
    /// Client requested a new window.
    CreateWindow { session: SlotKey, name: String },
    /// Client requested a new pane.
    CreatePane { window: SlotKey, rows: u16, cols: u16 },
    /// Client requested pane destruction.
    DestroyPane { pane: SlotKey },
    /// Client requested window destruction.
    DestroyWindow { window: SlotKey },
    /// Client requested session destruction.
    DestroySession { session: SlotKey },
}

/// All outputs / side-effect requests from the kernel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    /// Write bytes to a PTY master FD.
    WritePty { pane: SlotKey, data: Vec<u8> },
    /// Send bytes to a client connection.
    SendClient { client_id: u32, data: Vec<u8> },
    /// Spawn a new PTY process.
    SpawnPty { pane: SlotKey, command: String },
    /// Resize a PTY.
    ResizePty { pane: SlotKey, rows: u16, cols: u16 },
    /// Close/kill a PTY.
    ClosePty { pane: SlotKey },
    /// Log a message (for OTEL integration).
    Log { level: LogLevel, message: String },
    /// Session was created.
    SessionCreated { key: SlotKey, name: String },
    /// Window was created.
    WindowCreated { key: SlotKey, session: SlotKey, name: String },
    /// Pane was created.
    PaneCreated { key: SlotKey, window: SlotKey },
    /// Pane was destroyed.
    PaneDestroyed { key: SlotKey, window: SlotKey },
    /// Window was destroyed.
    WindowDestroyed { key: SlotKey, session: SlotKey },
    /// Session was destroyed.
    SessionDestroyed { key: SlotKey, name: String },
}

/// Log levels for Effect::Log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// The Sans-IO kernel: a pure reducer that maps Events to Effects.
///
/// INV-027: step() is pure (no I/O, no side effects).
/// INV-033: Deterministic under fixed clock.
#[derive(Debug, Clone)]
pub struct Kernel {
    graph: ServerGraph,
    /// Monotonic tick counter.
    tick_count: u64,
    /// Pane -> accumulated output for grid processing (reserved for future use).
    #[allow(dead_code)]
    pane_buffers: HashMap<u32, Vec<u8>>,
}

impl Kernel {
    #[must_use]
    pub fn new() -> Self {
        Self {
            graph: ServerGraph::new(),
            tick_count: 0,
            pane_buffers: HashMap::new(),
        }
    }

    /// Access the server graph.
    #[must_use]
    pub fn graph(&self) -> &ServerGraph {
        &self.graph
    }

    /// Mutable access to the server graph.
    pub fn graph_mut(&mut self) -> &mut ServerGraph {
        &mut self.graph
    }

    /// The current tick count.
    #[must_use]
    pub fn tick_count(&self) -> u64 {
        self.tick_count
    }

    /// Merge a remote Lamport/vector clock update (CRDT-enabled build).
    #[cfg(feature = "crdt")]
    #[must_use]
    pub fn merge_remote_clock(
        &self,
        time_source: &mut mux_time::DeterministicTimeSource,
        remote_lamport: u64,
        remote_vector: &mux_time::VectorClock,
    ) -> Vec<Effect> {
        time_source.receive(remote_lamport, remote_vector);
        vec![Effect::Log {
            level: LogLevel::Debug,
            message: "merged remote vector clock".to_string(),
        }]
    }

    /// Merge a remote Lamport update (non-CRDT build).
    #[cfg(not(feature = "crdt"))]
    #[must_use]
    pub fn merge_remote_clock(
        &self,
        time_source: &mut mux_time::DeterministicTimeSource,
        remote_lamport: u64,
    ) -> Vec<Effect> {
        time_source.receive_lamport(remote_lamport);
        vec![Effect::Log {
            level: LogLevel::Debug,
            message: "merged remote lamport clock".to_string(),
        }]
    }

    /// INV-027: Pure step function. No I/O.
    ///
    /// Takes an Event, updates internal state, and returns a Vec of Effects
    /// that the I/O layer should execute.
    #[must_use]
    pub fn step(&mut self, event: Event) -> Vec<Effect> {
        otel_scope!("mux-kernel::Kernel::step");

        match event {
            Event::ClientInput { client_id, data } => {
                // In a full implementation, this would parse the data
                // and route to the appropriate pane.
                vec![Effect::Log {
                    level: LogLevel::Debug,
                    message: format!("client {client_id} sent {} bytes", data.len()),
                }]
            }
            Event::PtyOutput { pane, data } => {
                // In a full implementation, this would feed data to the
                // parser/grid for the pane, then render to clients.
                let _ = pane;
                vec![Effect::Log {
                    level: LogLevel::Debug,
                    message: format!("pty output: {} bytes", data.len()),
                }]
            }
            Event::PtyExited { pane, exit_code } => {
                vec![
                    Effect::ClosePty { pane },
                    Effect::Log {
                        level: LogLevel::Info,
                        message: format!("pane exited with code {exit_code}"),
                    },
                ]
            }
            Event::Tick { time_ms } => {
                self.tick_count += 1;
                let _ = time_ms;
                vec![] // Status bar refresh would go here
            }
            Event::Resize { pane, rows, cols } => {
                vec![Effect::ResizePty { pane, rows, cols }]
            }
            Event::CreateSession { name } => {
                let key = self.graph.create_session(&name);
                vec![Effect::SessionCreated { key, name }]
            }
            Event::CreateWindow { session, name } => {
                if let Some(key) = self.graph.create_window(session, &name) {
                    vec![Effect::WindowCreated { key, session, name }]
                } else {
                    vec![Effect::Log {
                        level: LogLevel::Error,
                        message: "create_window: invalid session key".to_string(),
                    }]
                }
            }
            Event::CreatePane { window, rows, cols } => {
                if let Some(key) = self.graph.create_pane(window, rows, cols) {
                    vec![
                        Effect::PaneCreated { key, window },
                        Effect::SpawnPty {
                            pane: key,
                            command: "/bin/sh".to_string(),
                        },
                    ]
                } else {
                    vec![Effect::Log {
                        level: LogLevel::Error,
                        message: "create_pane: invalid window key".to_string(),
                    }]
                }
            }
            Event::DestroyPane { pane } => {
                if let Some(removed) = self.graph.destroy_pane(pane) {
                    vec![
                        Effect::PaneDestroyed {
                            key: pane,
                            window: removed.window,
                        },
                        Effect::ClosePty { pane },
                    ]
                } else {
                    vec![Effect::Log {
                        level: LogLevel::Error,
                        message: "destroy_pane: invalid pane key".to_string(),
                    }]
                }
            }
            Event::DestroyWindow { window } => {
                if let Some(removed) = self.graph.destroy_window(window) {
                    vec![Effect::WindowDestroyed {
                        key: window,
                        session: removed.session,
                    }]
                } else {
                    vec![Effect::Log {
                        level: LogLevel::Error,
                        message: "destroy_window: invalid window key".to_string(),
                    }]
                }
            }
            Event::DestroySession { session } => {
                if let Some(removed) = self.graph.destroy_session(session) {
                    vec![Effect::SessionDestroyed {
                        key: session,
                        name: removed.name,
                    }]
                } else {
                    vec![Effect::Log {
                        level: LogLevel::Error,
                        message: "destroy_session: invalid session key".to_string(),
                    }]
                }
            }
        }
    }
}

impl Default for Kernel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- GenSlotMap tests ---

    #[test]
    fn test_slot_map_insert_and_get() {
        let mut map = GenSlotMap::new();
        let key = map.insert("hello");
        assert_eq!(map.get(key), Some(&"hello"));
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn test_slot_map_remove() {
        let mut map = GenSlotMap::new();
        let key = map.insert(42);
        assert_eq!(map.remove(key), Some(42));
        assert!(map.get(key).is_none());
        assert_eq!(map.len(), 0);
    }

    #[test]
    fn test_slot_map_generation_prevents_aba() {
        let mut map = GenSlotMap::new();
        let key1 = map.insert("first");
        map.remove(key1);
        let key2 = map.insert("second");
        // key1 and key2 share the same index but different generations
        assert_eq!(key1.index, key2.index);
        assert_ne!(key1.generation, key2.generation);
        // Old key no longer valid
        assert!(map.get(key1).is_none());
        assert_eq!(map.get(key2), Some(&"second"));
    }

    #[test]
    fn test_slot_map_contains_key() {
        let mut map = GenSlotMap::new();
        let key = map.insert(1);
        assert!(map.contains_key(key));
        map.remove(key);
        assert!(!map.contains_key(key));
    }

    #[test]
    fn test_slot_map_get_mut() {
        let mut map = GenSlotMap::new();
        let key = map.insert(10);
        if let Some(v) = map.get_mut(key) {
            *v = 20;
        }
        assert_eq!(map.get(key), Some(&20));
    }

    #[test]
    fn test_slot_map_empty() {
        let map: GenSlotMap<i32> = GenSlotMap::new();
        assert!(map.is_empty());
        assert_eq!(map.len(), 0);
    }

    #[test]
    fn test_slot_map_multiple_inserts() {
        let mut map = GenSlotMap::new();
        let k1 = map.insert("a");
        let k2 = map.insert("b");
        let k3 = map.insert("c");
        assert_eq!(map.len(), 3);
        assert_eq!(map.get(k1), Some(&"a"));
        assert_eq!(map.get(k2), Some(&"b"));
        assert_eq!(map.get(k3), Some(&"c"));
    }

    // --- ServerGraph tests ---

    #[test]
    fn test_server_graph_create_session() {
        let mut graph = ServerGraph::new();
        let sk = graph.create_session("main");
        assert_eq!(graph.session_count(), 1);
        assert_eq!(graph.sessions.get(sk).unwrap().name, "main");
    }

    #[test]
    fn test_server_graph_create_window() {
        let mut graph = ServerGraph::new();
        let sk = graph.create_session("main");
        let wk = graph.create_window(sk, "editor").unwrap();
        assert_eq!(graph.window_count(), 1);
        assert_eq!(graph.windows.get(wk).unwrap().name, "editor");
        // Session has the window
        assert_eq!(graph.sessions.get(sk).unwrap().windows.len(), 1);
    }

    #[test]
    fn test_server_graph_create_pane() {
        let mut graph = ServerGraph::new();
        let sk = graph.create_session("main");
        let wk = graph.create_window(sk, "editor").unwrap();
        let pk = graph.create_pane(wk, 24, 80).unwrap();
        assert_eq!(graph.pane_count(), 1);
        let pane = graph.panes.get(pk).unwrap();
        assert_eq!(pane.rows, 24);
        assert_eq!(pane.cols, 80);
    }

    #[test]
    fn test_server_graph_invalid_session_key() {
        let mut graph = ServerGraph::new();
        let bad_key = SlotKey::new(99, 0);
        assert!(graph.create_window(bad_key, "test").is_none());
    }

    // --- Kernel tests ---

    #[test]
    fn test_kernel_create_session() {
        let mut kernel = Kernel::new();
        let effects = kernel.step(Event::CreateSession { name: "main".into() });
        assert_eq!(effects.len(), 1);
        assert!(matches!(&effects[0], Effect::SessionCreated { name, .. } if name == "main"));
        assert_eq!(kernel.graph().session_count(), 1);
    }

    #[test]
    fn test_kernel_create_window_and_pane() {
        let mut kernel = Kernel::new();
        let effects = kernel.step(Event::CreateSession { name: "s1".into() });
        let session_key = match &effects[0] {
            Effect::SessionCreated { key, .. } => *key,
            _ => panic!("expected SessionCreated"),
        };

        let effects = kernel.step(Event::CreateWindow { session: session_key, name: "w1".into() });
        assert_eq!(effects.len(), 1);
        let window_key = match &effects[0] {
            Effect::WindowCreated { key, .. } => *key,
            _ => panic!("expected WindowCreated"),
        };

        let effects = kernel.step(Event::CreatePane { window: window_key, rows: 24, cols: 80 });
        assert_eq!(effects.len(), 2);
        assert!(matches!(&effects[0], Effect::PaneCreated { .. }));
        assert!(matches!(&effects[1], Effect::SpawnPty { .. }));
    }

    #[test]
    fn test_kernel_tick() {
        let mut kernel = Kernel::new();
        assert_eq!(kernel.tick_count(), 0);
        let _ = kernel.step(Event::Tick { time_ms: 1000 });
        assert_eq!(kernel.tick_count(), 1);
        let _ = kernel.step(Event::Tick { time_ms: 2000 });
        assert_eq!(kernel.tick_count(), 2);
    }

    #[test]
    fn test_kernel_pty_exited() {
        let pane_key = SlotKey::new(0, 0);
        let mut kernel = Kernel::new();
        let effects = kernel.step(Event::PtyExited { pane: pane_key, exit_code: 0 });
        assert_eq!(effects.len(), 2);
        assert!(matches!(&effects[0], Effect::ClosePty { .. }));
        assert!(matches!(&effects[1], Effect::Log { level: LogLevel::Info, .. }));
    }

    #[test]
    fn test_kernel_resize() {
        let pane_key = SlotKey::new(0, 0);
        let mut kernel = Kernel::new();
        let effects = kernel.step(Event::Resize { pane: pane_key, rows: 40, cols: 120 });
        assert_eq!(effects.len(), 1);
        assert!(matches!(&effects[0], Effect::ResizePty { rows: 40, cols: 120, .. }));
    }

    #[test]
    fn test_kernel_client_input() {
        let mut kernel = Kernel::new();
        let effects = kernel.step(Event::ClientInput { client_id: 1, data: vec![0x41] });
        assert_eq!(effects.len(), 1);
        assert!(matches!(&effects[0], Effect::Log { level: LogLevel::Debug, .. }));
    }

    #[test]
    fn test_kernel_create_window_invalid_session() {
        let mut kernel = Kernel::new();
        let bad_key = SlotKey::new(99, 0);
        let effects = kernel.step(Event::CreateWindow { session: bad_key, name: "w1".into() });
        assert_eq!(effects.len(), 1);
        assert!(matches!(&effects[0], Effect::Log { level: LogLevel::Error, .. }));
    }

    #[test]
    fn test_kernel_default() {
        let kernel = Kernel::default();
        assert_eq!(kernel.tick_count(), 0);
        assert_eq!(kernel.graph().session_count(), 0);
    }

    #[test]
    fn test_server_graph_destroy_pane() {
        let mut graph = ServerGraph::new();
        let session = graph.create_session("s");
        let window = graph.create_window(session, "w").unwrap();
        let pane = graph.create_pane(window, 24, 80).unwrap();
        let removed = graph.destroy_pane(pane).expect("pane exists");
        assert_eq!(removed.window, window);
        assert_eq!(graph.pane_count(), 0);
        assert!(graph.windows.get(window).unwrap().panes.is_empty());
    }

    #[test]
    fn test_server_graph_destroy_window_cascades_panes() {
        let mut graph = ServerGraph::new();
        let session = graph.create_session("s");
        let window = graph.create_window(session, "w").unwrap();
        graph.create_pane(window, 24, 80).unwrap();
        graph.create_pane(window, 40, 120).unwrap();

        let removed = graph.destroy_window(window).expect("window exists");
        assert_eq!(removed.name, "w");
        assert_eq!(graph.window_count(), 0);
        assert_eq!(graph.pane_count(), 0);
        assert!(graph.sessions.get(session).unwrap().windows.is_empty());
    }

    #[test]
    fn test_server_graph_destroy_session_cascades_all_descendants() {
        let mut graph = ServerGraph::new();
        let session = graph.create_session("s");
        let window_a = graph.create_window(session, "a").unwrap();
        let window_b = graph.create_window(session, "b").unwrap();
        graph.create_pane(window_a, 24, 80).unwrap();
        graph.create_pane(window_b, 24, 80).unwrap();

        let removed = graph.destroy_session(session).expect("session exists");
        assert_eq!(removed.name, "s");
        assert_eq!(graph.session_count(), 0);
        assert_eq!(graph.window_count(), 0);
        assert_eq!(graph.pane_count(), 0);
    }

    #[test]
    fn test_server_graph_destroy_missing_items() {
        let mut graph = ServerGraph::new();
        let bad = SlotKey::new(55, 0);
        assert!(graph.destroy_pane(bad).is_none());
        assert!(graph.destroy_window(bad).is_none());
        assert!(graph.destroy_session(bad).is_none());
    }

    #[test]
    fn test_kernel_destroy_pane_effects() {
        let mut kernel = Kernel::new();
        let session = match kernel.step(Event::CreateSession { name: "s".into() })[0] {
            Effect::SessionCreated { key, .. } => key,
            _ => panic!("expected session"),
        };
        let window = match kernel.step(Event::CreateWindow {
            session,
            name: "w".into(),
        })[0]
        {
            Effect::WindowCreated { key, .. } => key,
            _ => panic!("expected window"),
        };
        let pane = match kernel.step(Event::CreatePane {
            window,
            rows: 24,
            cols: 80,
        })[0]
        {
            Effect::PaneCreated { key, .. } => key,
            _ => panic!("expected pane"),
        };

        let effects = kernel.step(Event::DestroyPane { pane });
        assert!(matches!(
            effects.as_slice(),
            [Effect::PaneDestroyed { key, .. }, Effect::ClosePty { pane: key2 }] if key == key2
        ));
    }

    #[test]
    fn test_kernel_destroy_window_effects() {
        let mut kernel = Kernel::new();
        let session = match kernel.step(Event::CreateSession { name: "s".into() })[0] {
            Effect::SessionCreated { key, .. } => key,
            _ => panic!("expected session"),
        };
        let window = match kernel.step(Event::CreateWindow {
            session,
            name: "w".into(),
        })[0]
        {
            Effect::WindowCreated { key, .. } => key,
            _ => panic!("expected window"),
        };
        let _ = kernel.step(Event::CreatePane {
            window,
            rows: 24,
            cols: 80,
        });

        let effects = kernel.step(Event::DestroyWindow { window });
        assert!(matches!(
            effects.as_slice(),
            [Effect::WindowDestroyed { key, session: s }] if *key == window && *s == session
        ));
        assert_eq!(kernel.graph().pane_count(), 0);
    }

    #[test]
    fn test_kernel_destroy_session_effects() {
        let mut kernel = Kernel::new();
        let session = match kernel.step(Event::CreateSession { name: "s".into() })[0] {
            Effect::SessionCreated { key, .. } => key,
            _ => panic!("expected session"),
        };
        let effects = kernel.step(Event::DestroySession { session });
        assert!(matches!(
            effects.as_slice(),
            [Effect::SessionDestroyed { key, name }] if *key == session && name == "s"
        ));
        assert_eq!(kernel.graph().session_count(), 0);
    }

    #[test]
    fn test_kernel_destroy_invalid_keys_log_error() {
        let mut kernel = Kernel::new();
        let bad = SlotKey::new(900, 0);
        let pane_effects = kernel.step(Event::DestroyPane { pane: bad });
        assert!(matches!(
            pane_effects.as_slice(),
            [Effect::Log {
                level: LogLevel::Error,
                ..
            }]
        ));
        let window_effects = kernel.step(Event::DestroyWindow { window: bad });
        assert!(matches!(
            window_effects.as_slice(),
            [Effect::Log {
                level: LogLevel::Error,
                ..
            }]
        ));
        let session_effects = kernel.step(Event::DestroySession { session: bad });
        assert!(matches!(
            session_effects.as_slice(),
            [Effect::Log {
                level: LogLevel::Error,
                ..
            }]
        ));
    }

    #[cfg(not(feature = "crdt"))]
    #[test]
    fn test_kernel_merge_remote_clock_lamport_only() {
        let kernel = Kernel::new();
        let mut ts = mux_time::DeterministicTimeSource::deterministic(1, 1_000);
        ts.tick();
        let effects = kernel.merge_remote_clock(&mut ts, 10);
        assert_eq!(ts.lamport_time(), 11);
        assert!(matches!(
            effects.as_slice(),
            [Effect::Log {
                level: LogLevel::Debug,
                ..
            }]
        ));
    }

    #[cfg(feature = "crdt")]
    #[test]
    fn test_kernel_merge_remote_clock_crdt() {
        let kernel = Kernel::new();
        let mut ts = mux_time::DeterministicTimeSource::deterministic(1, 1_000);
        ts.tick();
        let mut remote = mux_time::VectorClock::new();
        remote.increment(2);
        remote.increment(2);
        let effects = kernel.merge_remote_clock(&mut ts, 10, &remote);
        assert_eq!(ts.lamport_time(), 11);
        assert_eq!(ts.vector_clock().get(2), 2);
        assert!(matches!(
            effects.as_slice(),
            [Effect::Log {
                level: LogLevel::Debug,
                ..
            }]
        ));
    }

    #[test]
    fn test_otel_span_noop_helpers() {
        let span = otel_boundary_span("mux-kernel::test");
        assert_eq!(span.name(), "mux-kernel::test");
        let _ = span.entered();
        let _ = otel_span!("mux-kernel::macro");
        otel_scope!("mux-kernel::scope");
    }
}
