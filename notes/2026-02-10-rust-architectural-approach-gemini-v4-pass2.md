# Termforge Architecture Reference

## 1. Project Name and Identity
*   **Name:** `termforge`
*   **Binary Names:**
    *   `mux-server`: The background daemon managing sessions and PTYs.
    *   `mux`: The CLI client for sending commands and attaching.
    *   `mux-tui`: A specialized visual client (like a native UI for the multiplexer).
*   **Crate Prefix:** `mux-*` (e.g., `mux-core`, `mux-server`).
*   **License:** MIT OR Apache-2.0.

## 2. Vision and Philosophy
Termforge is a "Pure Core" terminal multiplexer. It separates the logical state of terminal windows (the "Model") from the side effects of I/O (PTYs, Sockets, Rendering).
*   **Deterministic:** The core logic is a pure function `(State, Event) -> (State, Effects)`.
*   **Recoverable:** The entire state can be snapshotted and restored.
*   **Embeddable:** The core can run in WASM, allowing for browser-based playback or "terminals in terminals" without spawning processes.
*   **Type-Safe:** Entities (Sessions, Windows, Panes) are referenced by strongly-typed IDs, preventing invalid access.

## 3. North Star Acceptance Criteria
1.  **WASM Compilation:** `mux-core` must compile to `wasm32-unknown-unknown`.
2.  **Deterministic Replay:** A recorded session of user inputs must produce the exact same pixel-grid state when replayed through the core, guaranteed by CI.
3.  **Crash Recovery:** If `mux-server` crashes, the PTYs remain alive (via a separate holder process or careful file descriptor passing) and the server can restart, re-attach, and reconstruct the state.
4.  **Zero-Flicker Reloads:** Configuration reloads happen atomically in the `State`.

## 4. High-Level Architecture
```mermaid
graph TD
    Client[mux CLI] -->|IPC/Proto| Server[mux-server]
    Server -->|IO/PTY| PTYs[System PTYs]
    
    subgraph "mux-server Process"
        Net[Network Layer] --> Runtime
        Runtime[Runtime Loop] -->|Event| Core[mux-core (Pure)]
        Core -->|Effect| Runtime
        Runtime -->|Write| PTYs
        Runtime -->|Draw| Client
    end
```

## 5. Workspace Layout
```text
termforge/
├── Cargo.toml          # Workspace root
├── mux-types/          # Shared IDs, Enums, Primitives (low deps)
├── mux-protocol/       # Wire format (Client <-> Server)
├── mux-core/           # Pure logic, Grid, State, Reducer (WASM compatible)
├── mux-api/            # High-level Query/ORM and Facade
├── mux-server/         # Daemon, Tokio, PTY management, Sockets
├── mux-client/         # CLI implementation
├── mux-tui/            # Visual client implementation
└── mux-regress/        # Compatibility and regression suite
```

## 6. Layering Contract
1.  **mux-types:** Zero logic. Only `struct`s and `enum`s. `no_std` compatible.
2.  **mux-core:** Depends on `mux-types`. Pure logic only. No `std::net`, `std::fs`, or `tokio`. Allocator allowed.
3.  **mux-api:** Depends on `mux-core`. Provides developer-friendly wrappers (Iterators, Queries).
4.  **mux-server:** The "Impure Shell". Manages `tokio` runtime, file descriptors, signals. Injects events into `mux-core`.

## 7. Entity Model
We use a relational model backed by `SlotMap` for O(1) access and stable IDs.

**Location:** `mux-core/src/model.rs`
```rust
// In mux-types
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct SessionId(pub u64);
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct WindowId(pub u64);
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct PaneId(pub u64);

// In mux-core
pub struct State {
    pub sessions: SlotMap<SessionId, Session>,
    pub windows: SlotMap<WindowId, Window>,
    pub panes: SlotMap<PaneId, Pane>,
    // Many-to-Many relationships or active pointers
    pub active_session: Option<SessionId>,
}

pub struct Session {
    pub id: SessionId,
    pub name: String,
    // Ordered list of windows in this session
    pub windows: Vec<WindowId>, 
    pub active_window: Option<WindowId>,
}

pub struct Window {
    pub id: WindowId,
    pub name: String,
    pub layout: LayoutTree, // Binary tree of PaneIds
    pub active_pane: Option<PaneId>,
}

pub struct Pane {
    pub id: PaneId,
    pub grid: Grid, // The terminal content
    pub mode: PaneMode, // Normal, Copy, View
}
```

## 8. Event/Effect Engine
The heart of the system.

**Location:** `mux-core/src/lib.rs`
```rust
pub enum Event {
    InputReceived { session_id: SessionId, data: Vec<u8> },
    PtyData { pane_id: PaneId, data: Vec<u8> },
    Command { cmd: CommandEnum },
    Resize { pane_id: PaneId, cols: usize, rows: usize },
    Tick,
}

pub enum Effect {
    WritePty { pane_id: PaneId, data: Vec<u8> },
    RenderClient { client_id: ClientId, diff: GridDiff },
    Exit,
}

pub fn update(state: &mut State, event: Event) -> Vec<Effect> {
    // Pure function logic
    match event {
        Event::PtyData { pane_id, data } => {
            if let Some(pane) = state.panes.get_mut(pane_id) {
                pane.grid.process(&data);
                // Return no side-effect, just state mutation
                // Or return Render effect if dirty
            }
            vec![] 
        }
        // ...
    }
}
```

## 9. Protocol Codec
We use a binary protocol for efficiency, defined in `mux-protocol`.

*   **Format:** `Length (u32) | MessageType (u8) | Payload (Bincode/Protobuf)`
*   **Crate:** `mux-protocol` defines the `enum ServerMessage` and `enum ClientMessage`.

## 10. ORM-like Query API
To simplify binding implementation and CLI queries.

**Location:** `mux-api/src/query.rs`
```rust
pub struct Query<'a> {
    state: &'a State,
}

impl<'a> Query<'a> {
    pub fn new(state: &'a State) -> Self { Self { state } }
    
    pub fn sessions(&self) -> impl Iterator<Item = SessionWrapper<'a>> { ... }
}

pub struct SessionWrapper<'a> {
    session: &'a Session,
    state: &'a State,
}

impl<'a> SessionWrapper<'a> {
    pub fn windows(&self) -> impl Iterator<Item = WindowWrapper<'a>> {
        self.session.windows.iter().filter_map(|id| self.state.windows.get(*id))
    }
}
```

## 11. Runtime Architecture (mux-server)
*   **Main Loop:** `tokio::select!` loop.
*   **Actor Model:**
    *   `PtyManager`: Owns `pty_master` FDs. Reads stdout, sends `Event::PtyData` to Core. Receives `Effect::WritePty`.
    *   `ClientListener`: Accepts TCP/Unix connections.
    *   `CoreLoop`: Holds `Arc<Mutex<State>>` (or `RwLock`). Processes events sequentially.

## 12. Language Bindings
Bindings interact with `mux-api` and `mux-protocol`.
*   **Python:** `pyo3` wrapping `mux-api` structs.
*   **Node:** `napi-rs`.
*   They communicate with a running server via RPC, OR if we want "embedded" usage (like a Python script creating a session in-memory), they link against `mux-core`.

## 13. CRDT Transaction Layer
*Optional for V1.*
If we support multi-server or collaborative editing, we use a CRDT for the text buffer (e.g., `diamond-types`). For V1, the `State` is the single source of truth (Authority) held by the Server.

## 14. tmux Version Management
We do not use `tmux` code. We replace it. However, we provide a `tmux` compatibility alias.
*   `mux-regress` will run `tmux` test suites against `termforge` to ensure flag compatibility.

## 15. Test Support Crate
`mux-test`: Contains `Scenario` builder, `MockPty`, and assertion helpers.
Used by `mux-core` integration tests.

## 16. Fake PTY Backend
Crucial for deterministic testing.

**Location:** `mux-core/src/pty.rs` (Trait definition)
```rust
pub trait PtyBackend {
    fn resize(&mut self, cols: usize, rows: usize);
    fn write(&mut self, data: &[u8]);
}
```
In `mux-server`, this is implemented by `portable-pty`.
In tests, it's `MockPty` which records writes to a buffer.

**Scenario Recorder:**
Records `(Time, Event)` tuples.
Serialized to YAML/JSON.
Replayer runs the `update` loop with these timed events and asserts the `State` hash or Grid checksum at the end.

## 17. Binding Test Frameworks
*   **Python:** `pytest` invoking the `mux` python module.
*   **JS:** `jest`.

## 18. Test Framework and Harness Design
*   **Unit Tests:** Standard `#[test]` in `mux-core`.
*   **Snapshot Tests:** `insta` for Grid state.
    ```rust
    let mut grid = Grid::new(80, 24);
    grid.process(b"Hello World");
    assert_snapshot!(grid.to_string());
    ```
*   **Fuzzing:** `cargo-fuzz` on the `Grid::process` method (VT100 parser) and `Protocol::decode`.

## 19. Visual Client (TUI)
A separate binary `mux-tui` that uses `ratatui`.
It connects to `mux-server`, receives the `Grid` diffs, and renders them.
It does *not* manage state. It is a dumb terminal.

## 20. DOs and DON'Ts
*   **DO** use `Result` for everything.
*   **DO** implement `Display` for IDs.
*   **DON'T** use `unwrap()` in `mux-core` or `mux-server`. Panic in daemon is unacceptable.
*   **DON'T** put `std::thread::sleep` in `mux-core`.
*   **DON'T** mix IO logic into `Session` methods.

## 21. AGENTS.md Template
(Omitted for brevity, standard Agent rules apply: "You are a Rust expert...")

## 22. Phased Implementation Plan
1.  **Phase 1: The Grid.** Implement `Grid`, `Cell`, `Line` and VT100 parser (`vte` wrapper). 90% test coverage.
2.  **Phase 2: The Core.** Implement `State`, `Session`, `Window`, `Pane` and the `Event` loop.
3.  **Phase 3: The Server.** Implement `tokio` runtime, PTY spawning, and Client IPC.
4.  **Phase 4: The CLI.** Implement basic `attach`, `ls`, `new-session`.
5.  **Phase 5: Key Bindings & Copy Mode.**

## 23. Risks and Mitigations
*   **Risk:** VT100 compatibility is a bottomless pit.
    *   *Mitigation:* Use `vte` crate. Do not write a parser. Focus on the *model* (Grid).
*   **Risk:** Performance with large scrollback.
    *   *Mitigation:* Use `Vec<Vec<Cell>>` with a circular buffer logic or `pagetable` approach for history. Benchmark early.
*   **Risk:** IO blocking the main loop.
    *   *Mitigation:* Strict actor model. `mux-core` never does IO.

## Detailed Refinements (from Open Questions)

### Grid / VT100 Implementation (`mux-term`)
We will use the `vte` crate.
```rust
struct TermGrid {
    lines: Vec<Line>,
    cursor: Cursor,
    // ...
}

impl vte::Perform for TermGrid {
    fn print(&mut self, c: char) { ... }
    fn execute(&mut self, byte: u8) { ... }
    // ...
}
```

### Format String Engine
We will use `nom`.
**Structure:**
```rust
enum FormatNode {
    Literal(String),
    Variable { name: String, limit: Option<usize> },
    Conditional { var: String, true_branch: Box<FormatNode>, false_branch: Box<FormatNode> },
}
```

### Key Binding System
```rust
// mux-core/src/input.rs
pub struct KeyBinding {
    pub key: Key, // Enum with modifiers
    pub command: CommandEnum,
}

pub struct KeyTable {
    pub bindings: HashMap<Key, KeyBinding>,
}

pub struct KeyManager {
    pub tables: HashMap<String, KeyTable>,
    pub current_table: String, // "root", "prefix"
}
```

### Copy Mode
```rust
// mux-core/src/pane.rs
pub trait PaneMode {
    fn handle_input(&mut self, key: Key, ctx: &mut PaneContext) -> ModeAction;
    fn render(&self, grid: &Grid) -> Grid; // Overlay
}

struct NormalMode; // Forwards to PTY
struct CopyMode {
    selection: Option<Selection>,
    scroll_offset: usize,
    search_query: Option<String>,
}
```

This architecture provides a robust, type-safe, and deterministic foundation for a modern terminal multiplexer.
