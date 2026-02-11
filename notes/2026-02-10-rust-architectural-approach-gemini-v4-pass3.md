# RUST TERMINAL MULTIPLEXER (`rmux`) ARCHITECTURE
## DEFINITIVE SYNTHESIS (PASS 3/3)

This document is the "North Star" architectural specification for `rmux`, a modern, memory-safe, and architecturally strict reimplementation of terminal multiplexing core logic. It aggregates all key decisions regarding entity modeling, WASM purity, protocol layering, and testing infrastructure.

---

### 1. Project Name and Identity
*   **Codename**: `rmux`
*   **Identity**: A "library-first" multiplexer. Unlike `tmux` which is an application that happens to be scriptable, `rmux` is a high-performance Rust library for terminal multiplexing that happens to ship with a CLI binary.
*   **Core Differentiator**: Strict separation of IO (Runtime) and Logic (Core). The Core is a deterministic finite state machine compilable to WASM.

### 2. Vision and Philosophy
*   **Safety**: Eliminate class-of-bugs related to use-after-free and buffer overflows common in C parsers.
*   **Determinism**: The `mux-core` crate must be purely functional. Given State S and Event E, the Result (S', Effects) is always identical.
*   **Parity**: Pixel-perfect VT100/xterm emulation matching `tmux` quirks where necessary for compatibility, but architected better.
*   **Programmability**: First-class bindings (Python, TypeScript) that are not just string-wrappers around a CLI, but typed interfaces over an IPC protocol.

### 3. North Star Acceptance Criteria
1.  **WASM Compilation**: `cd mux-core && cargo check --target wasm32-unknown-unknown` passes in CI. This guarantees no `std::io` or OS threading leakage.
2.  **The "Vibe" Test**: A recorded 1-hour active `tmux` session (input bytes + window resizes) replayed through `rmux` yields the exact same grid state hashes.
3.  **Headless Operation**: Full functionality available without a TUI, strictly via API/Protocol.
4.  **Zero-Copy Logic**: Cloning the state for history/undo uses Copy-on-Write (Arc) structures, not deep copies.

### 4. High-Level Architecture

```mermaid
graph TD
    subgraph "Host Environment (Runtime)"
        A[CLI/TUI Client] -->|Unix Socket| B(Socket Actor)
        C[Python Script] -->|Unix Socket| B
        D[PTY Backend] <-->|Bytes| E[IO Pump]
        F[Signal Handler] --> E
    end

    subgraph "Mux Runtime (Async Rust)"
        B --> G[Command Channel]
        E --> H[Input Channel]
        I[State Manager]
        G --> I
        H --> I
    end

    subgraph "Mux Core (Pure WASM)"
        I -->|Apply Event| J[State Machine]
        J -->|Return Effects| I
    end

    subgraph "Storage / Persistence"
        J --> K[Entity Graph (SlotMap)]
        J --> L[Grid (Arc COW)]
    end
```

### 5. Workspace Layout

The repository is a Cargo Workspace with strict dependency rules.

```text
/
├── Cargo.toml              # Workspace root
├── ag-mux.md               # Agent guidance
├── mux-types/              # [Leaf] Primitives, IDs, Enums. No logic.
├── mux-protocol/           # [Dep: types] IMSG codec, serde defs.
├── mux-core/               # [Dep: protocol] Pure logic, State, VT Parser. NO IO.
├── mux-orm/                # [Dep: protocol] High-level query API for clients.
├── mux-runtime/            # [Dep: core] Tokio, PTY handling, Signals, Sockets.
├── mux-server/             # [Dep: runtime] The actual binary entry point.
├── mux-client/             # [Dep: protocol, orm] CLI binary (thin wrapper).
├── mux-term/               # [Dep: types] Standalone terminal emulator widget (ratatui).
├── bindings/
│   ├── python/             # PyO3 bindings wrapping mux-client logic
│   └── node/               # NAPI-RS bindings
└── tools/
    ├── fake-pty/           # Testing harness for PTYs
    └── tmux-vm/            # Version manager for legacy tmux binaries
```

### 6. Layering Contract & Enforcement

| Crate | Allowed | Banned | Enforcement |
| :--- | :--- | :--- | :--- |
| `mux-types` | `serde`, `std::fmt` | Logic, IO, Async | Code Review |
| `mux-core` | `log`, `serde`, `bitflags` | `std::net`, `std::fs`, `tokio`, `std::thread` | **CI: wasm32-unknown-unknown** |
| `mux-protocol`| `serde`, `bytes` | Logic | Code Review |
| `mux-runtime`| `tokio`, `nix`, `libc` | UI Rendering logic (keep in client) | Structure |

### 7. Entity Model

We use `slotmap` for O(1) lookups and stable generation-counted IDs to prevent ABA problems.

```rust
// mux-types/src/ids.rs
use slotmap::new_key_type;

new_key_type! { pub struct SessionId; }
new_key_type! { pub struct WindowId; }
new_key_type! { pub struct PaneId; }

// mux-core/src/state.rs
pub struct MuxState {
    pub sessions: SlotMap<SessionId, Session>,
    pub windows: SlotMap<WindowId, Window>,
    pub panes: SlotMap<PaneId, Pane>,
    // Inverted indices (maintained by logic)
    pub window_to_session: SecondaryMap<WindowId, SessionId>,
}

// mux-core/src/entities.rs
pub struct Session {
    pub name: String,
    pub windows: Vec<WindowId>, // Ordered list
    pub active_window: WindowId,
}

pub struct Window {
    pub name: String,
    pub layout: LayoutTree, // Recursive split structure
    pub active_pane: PaneId,
    pub panes: Vec<PaneId>, // Flat list for quick iteration
}

pub struct Pane {
    pub grid: Arc<Grid>, // CoW for history
    pub cursor: Cursor,
    pub width: u16,
    pub height: u16,
    pub mode: PaneMode, // Normal, Copy, etc.
}
```

### 8. Event/Effect Engine

The core is a reducer: `fn apply(state, event) -> (State, Vec<Effect>)`.

```rust
// mux-core/src/event.rs
pub enum InputEvent {
    ClientInput { client_id: u64, bytes: Vec<u8> },
    PtyOutput { pane_id: PaneId, bytes: Vec<u8> },
    Command { cmd: CommandEnum },
    TimerTick,
}

// mux-core/src/effect.rs
pub enum SideEffect {
    WriteToClient { client_id: u64, data: Vec<u8> },
    WriteToPty { pane_id: PaneId, data: Vec<u8> },
    ResizePty { pane_id: PaneId, size: PaneSize },
    Bell,
    Exit,
}

impl MuxState {
    pub fn apply(&mut self, event: InputEvent) -> Vec<SideEffect> {
        match event {
            InputEvent::PtyOutput { pane_id, bytes } => {
                let pane = self.panes.get_mut(pane_id)?;
                let mut parser = VtParser::new();
                let dirty = parser.process(&mut pane.grid, &bytes);
                // Return redraw effect if grid changed
                if dirty { vec![SideEffect::NotifyUpdate(pane_id)] } else { vec![] }
            }
            // ...
        }
    }
}
```

### 9. Protocol Codec

We use a binary protocol similar to `tmux`'s `imsg` but defined with Rust types.

```rust
// mux-protocol/src/msg.rs
#[derive(Serialize, Deserialize)]
pub enum MsgType {
    Command(Command),
    Print(String),
    Error(String),
    Exit,
    // Two-phase data transfer
    Identify { protocol_version: u32, pid: u32 },
}

// mux-protocol/src/codec.rs
// Implements tokio_util::codec::Encoder/Decoder
// Frame format: [Length: u32][Type: u16][Payload: Bytes]
```

### 10. ORM-like Query API (`mux-orm`)

Provides a fluent interface for clients (CLI, Bindings) to interact with the server.

```rust
// mux-orm/src/lib.rs
pub struct Client { ... }

impl Client {
    pub fn session(&self, target: &str) -> SessionQuery { ... }
}

impl SessionQuery {
    pub fn list_windows(&self) -> Result<Vec<WindowInfo>>;
    pub fn new_window(&self, name: &str) -> Result<WindowId>;
    pub fn kill(&self) -> Result<()>;
}

// Usage
// let client = Client::connect("/tmp/tmux-1000/default").await?;
// client.session("0").window("active").split_pane().await?;
```

### 11. Runtime Architecture

The Runtime bridges the `async` world with the synchronous `Core`.

```rust
// mux-runtime/src/server.rs
struct Server {
    core: Arc<Mutex<MuxState>>, // Or RWLock
    pty_manager: PtyManager,
    clients: ClientManager,
}

async fn main_loop(mut rx: mpsc::Receiver<InputEvent>) {
    let mut state = MuxState::new();
    
    while let Some(event) = rx.recv().await {
        let effects = state.apply(event);
        for effect in effects {
            handle_effect(effect).await;
        }
    }
}
```

### 12. Language Bindings

Bindings are **thin clients**. They do not embed the server (unless explicitly designed for embedded usage). They use `mux-orm` and the IPC socket.

-   **Python (`pyo3`)**: Exposes `rmux.Server`, `rmux.Session` classes that send commands to the socket.
-   **Node (`napi-rs`)**: TypeScript definitions generated from the Rust ORM structs.

### 13. CRDT Transaction Layer (Future Proofing)

To support roaming/distributed sessions, state mutation is modeled as CRDT operations.

```rust
// mux-core/src/crdt.rs
#[derive(Serialize, Deserialize)]
pub enum CrdtOp {
    SessionCreate { id: SessionId, name: String, timestamp: Hlc },
    WindowLink { session: SessionId, window: WindowId, timestamp: Hlc },
    GridWrite { pane: PaneId, x: u16, y: u16, cell: Cell, timestamp: Hlc },
}
```
*Note: This is abstract for Phase 1, but `apply` functions should be structured to allow this.*

### 14. tmux Version Management

Located in `tools/tmux-vm`. A Rust binary that:
1.  Downloads `tmux` source tarballs (versions 2.0 to 3.5).
2.  Compiles them into isolated paths.
3.  Used by the test suite to verify "Is this behavior standard?"

### 15. Test Support Crate

`mux-test-support`:
-   **Hermetic Server**: Spawns `mux-server` with a temporary socket and config file.
-   **PathGuard**: RAII wrapper for temp directories.

### 16. Fake PTY & Scenario Recorder

Crucial for deterministic debugging.

```rust
// mux-runtime/src/pty.rs
trait PtyBackend {
    fn resize(&self, w: u16, h: u16);
    fn write(&self, data: &[u8]);
    fn read(&self, buf: &mut [u8]) -> usize;
}

// tools/recorder/
struct ScenarioRecorder<T: PtyBackend> {
    inner: T,
    log: File, // Writes JSON: { "ts": 100, "dir": "Out", "data": "hex" }
}
```

### 17. Binding Test Frameworks

-   **Python**: `pytest` plugin that spins up a `mux-server` per test function.
-   **Node**: `vitest` with `beforeAll` hooks to spawn the server.

### 18. Test Framework Design

1.  **Unit**: `cargo test` in `mux-core` (State transitions).
2.  **Property**: `proptest` generating random ANSI sequences to crash the parser.
3.  **Integration**: `mux-runtime` tests using FakePTY.
4.  **Parity**: Compare `rmux` grid state vs `tmux` grid state for same input.

### 19. Visual Client (TUI)

Uses `ratatui`.
-   **Pipeline**: `State` -> `Snapshot` -> `Ratatui Widgets`.
-   **Status Line**: A renderer that parses format strings and produces styled text.

### 20. DOs and DON'Ts

**DO:**
*   **Use `#[must_use]`** on all `Effect` return values.
*   **Use `tracing`** with structural logging instead of `println!`.
*   **Separate `Layout`** logic (tiling algorithm) from `Window` entity.
*   **Use Newtypes** (`Row(u16)`, `Col(u16)`) to avoid coordinate confusion.

**DON'T:**
*   **No `unsafe`** in `mux-core`.
*   **No `std::process::Command`** inside `mux-core`.
*   **Don't assume UTF-8** in PTY streams until parsed (they are byte streams).
*   **Don't block_on** in async code.

---

### Special Subsections

#### VT100 Parser Design
We do *not* use the `vte` crate because `tmux` has non-standard extensions and specific quirk handling required for compatibility. We implement a state machine based on Paul Williams' parser.

```rust
// mux-core/src/vt/machine.rs
enum State {
    Ground,
    Escape,
    EscapeIntermediate,
    CsiEntry,
    CsiParam,
    CsiIntermediate,
    CsiIgnore,
    DcsEntry,
    // ...
}

pub fn advance(state: State, byte: u8) -> (State, Action);
```

#### Format String Engine
`mux-core/src/format.rs`. A recursive descent parser for `#{session_name}` style formats.

```rust
enum FormatNode {
    Literal(String),
    Variable { name: String, modifiers: Vec<Modifier> },
    Conditional { var: String, true_branch: Vec<FormatNode>, false_branch: Vec<FormatNode> },
}
```

#### Key Binding System
`mux-core/src/key_table.rs`.

```rust
struct KeyTable {
    bindings: HashMap<Key, Command>,
}
struct KeyBindings {
    tables: HashMap<String, KeyTable>, // "root", "prefix", "copy-mode-vi"
}
```

#### Copy Mode
Implemented as an overlay on the Pane.

```rust
enum PaneMode {
    Normal,
    Copy {
        scroll_offset: usize,
        selection: Option<Selection>,
        search_query: Option<String>,
        style: CopyModeStyle, // Vi / Emacs
    }
}
```

---

### 21. AGENTS.md Template

Create `AGENTS.md` in the root.

```markdown
# Agent Protocol for rmux

## Context
You are working on `rmux`, a high-performance, strict-architecture Rust terminal multiplexer.

## Architectural Enforcements
1. **Core Purity**: NEVER add `std::io`, `std::net`, or `tokio` to `mux-core`. If you need time, pass it as an argument.
2. **Entity Access**: Always use `SlotMap` IDs. Never hold raw references to Entities.
3. **Locking**: `mux-core` is single-threaded. `mux-runtime` handles locking. Do not add Mutexes inside Entities.

## Common Tasks
- **Adding a Command**:
    1. Define in `mux-protocol/src/commands.rs`.
    2. Add parsing in `mux-client/src/parse.rs`.
    3. Add handling in `mux-core/src/lib.rs` (apply method).
- **Fixing VT Parsing**:
    1. Add a reproduction case in `mux-core/tests/vt_cases.rs`.
    2. Modify `mux-core/src/vt/`.
    3. Verify against `tools/tmux-vm` if behavior is ambiguous.

## Tool Usage
- Use `cargo check -p mux-core --target wasm32-unknown-unknown` to verify purity.
- Use `cargo test --workspace` before submitting.
```

### 22. Phased Implementation Plan

*   **Phase 1: The Core (Weeks 1-4)**
    *   Setup Workspace.
    *   Implement `SlotMap` entity graph.
    *   Implement basic VT parser (text, colors, cursor move).
    *   Implement Grid data structure (fixed size).
    *   *Milestone*: Headless session that accepts bytes and updates a grid.

*   **Phase 2: The Runtime (Weeks 5-8)**
    *   `tokio` server setup.
    *   PTY spawning (using `portable-pty` initially).
    *   Socket listeners.
    *   *Milestone*: `cargo run` starts a server, `nc` can send commands.

*   **Phase 3: The Protocol & Client (Weeks 9-10)**
    *   Implement `imsg` codec.
    *   Build `rmux` CLI.
    *   *Milestone*: `rmux new-session` works.

*   **Phase 4: Interactive TUI (Weeks 11-14)**
    *   `ratatui` integration.
    *   Rendering logic.
    *   Input handling loop.
    *   *Milestone*: `rmux attach` yields a usable terminal.

*   **Phase 5: Parity & Polish (Weeks 15-20)**
    *   Complex VT sequences (scrolling regions, insert/delete).
    *   Config parsing (`tmux.conf` compat).
    *   Copy mode.

*   **Phase 6: Bindings (Weeks 21+)**
    *   Python/Node bindings.

### 23. Risks and Mitigations

| Risk | Mitigation |
| :--- | :--- |
| **Performance (Grid)** | PTY throughput can be high. Use `Arc` CoW for history. Batch updates to clients (max 60fps). |
| **VT Complexity** | Terminal escape codes are notorious. Rely on `vte` reference tests + `tmux` source code analysis. |
| **Async Deadlocks** | Keep `mux-core` synchronous. Only `mux-runtime` holds locks. Minimize lock granularity. |
| **Unicode Width** | Use `unicode-width` crate, but allow overriding for ambiguous characters (East Asian Width). |
| **Platform Support** | Linux first. MacOS second. Windows (WSL) third. Abstract PTYs via trait. |
