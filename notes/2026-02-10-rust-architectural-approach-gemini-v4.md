To ensure the comprehensive architectural vision is captured accurately and effectively, I will generate the complete architectural document as requested. This document will serve as the "North Star" for the project, detailing every aspect from the high-level philosophy to specific implementation details of the Rust crates and testing strategies.

Here is the architectural design document for **Vibe**.

***

# Vibe: The Rust Terminal Multiplexer Ecosystem

## 1. Project Name and Identity

*   **Name:** Vibe
*   **Crate Prefix:** `vibe-` (e.g., `vibe-core`, `vibe-server`, `vibe-api`)
*   **Tagline:** "A purely deterministic terminal multiplexer ecosystem, speaking fluent tmux."

**Vibe** is not just a port of tmux to Rust. It is a ground-up re-architecture of the terminal multiplexer concept, built on a strictly pure, deterministic core, wrapped in a modern asynchronous runtime, and exposing a rich ORM-like API to multiple languages. It guarantees 100% wire-protocol compatibility with tmux, allowing it to act as a drop-in server replacement or a high-level client library.

## 2. Vision and Philosophy

The legacy of tmux is immense, but its architecture (C, libevent, single-threaded, monolithic state/IO mixing) limits extensibility and safety. Vibe separates the *logic* of multiplexing from the *runtime* of the OS.

*   **Independence:** We do not link against `libtmux` or `tmux` C code. We implement the protocol (Protocol v8/v9 via `imsg`) from scratch.
*   **Determinism:** The `vibe-core` crate is a pure function: `(State, Event) -> (NewState, Vec<Effect>)`. It has no IO, no sockets, no clocks, no randomness. This allows for perfect replayability and property-based testing.
*   **Ecosystem First:** Vibe is designed as a library first, a server second. The `vibe-api` facade provides a powerful "DOM for the terminal" that Python, Node, and C++ can script against naturally.

## 3. North Star Acceptance Criteria

1.  **Protocol Compatibility:** A stock `tmux` client (v3.3a+) must be able to attach to a `vibe-server` instance and perform all basic window/pane operations without realizing it's not talking to a C tmux server.
2.  **Architectural Purity:** `vibe-core` must compile to `wasm32-unknown-unknown` (proving no OS deps).
3.  **Hermetic Testing:** We must run the full test suite in parallel without port conflicts, file collisions, or global state leaks.
4.  **Language Parity:** A Python script using `vibe-binding-python` must be able to perform the same complex session orchestration as a Rust native binary.
5.  **Drop-in Replacement:** Alias `tmux` to `vibe` in a user's shell, and their daily workflow (split, attach, detach, resize) remains uninterrupted.

## 4. High-Level Architecture

```mermaid
graph TD
    subgraph "Bindings (Layer 3)"
        Py[Python Binding]
        Node[Node Binding]
        Cpp[C++ Binding]
    end

    subgraph "Facade (Layer 2)"
        API[vibe-api]
        ORM[ORM / Query Layer]
        API --> ORM
    end

    subgraph "Runtime (Layer 1)"
        Runtime[vibe-server / vibe-runtime]
        Socket[SocketActor]
        Refresh[RefreshDriver]
        PTY[PTY Backend (Real/Fake)]
        Runtime --> Socket
        Runtime --> Refresh
        Runtime --> PTY
    end

    subgraph "Pure Core (Layer 0)"
        Core[vibe-core]
        Model[Entity Graph (SlotMap)]
        Proto[vibe-protocol (imsg)]
        Logic[Event/Effect Engine]
        Core --> Model
        Core --> Proto
        Core --> Logic
    end

    Py --> API
    Node --> API
    Cpp --> API
    API --> Runtime
    Runtime --> Core
```

## 5. Workspace Layout

The repository is a Cargo workspace organized for strict layer separation.

```text
/
├── Cargo.toml                  # Workspace definition
├── bindings/
│   ├── vibe-binding-python/    # PyO3 + maturin
│   ├── vibe-binding-node/      # NAPI-RS
│   └── vibe-binding-cpp/       # cxx bridge
├── crates/
│   ├── vibe-core/              # LAYER 0: Pure state machine, Entity Graph
│   ├── vibe-protocol/          # LAYER 0: imsg serialization, constants
│   ├── vibe-types/             # LAYER 0: Shared Enums (Layout, ResizeDir)
│   ├── vibe-query/             # LAYER 0: Query DSL (Django-style filters)
│   ├── vibe-crdt/              # LAYER 0: Conflict-free Replicated Data Types
│   ├── vibe-style/             # LAYER 0: Style parsing, palettes
│   ├── vibe-api/               # LAYER 2: Facade, ManagedMux, ORM public API
│   ├── vibe-server/            # LAYER 1: Tokio runtime, SocketActor
│   ├── vibe-client/            # LAYER 1: Client logic (attaching to socket)
│   ├── vibe-pty/               # LAYER 1: PTY abstraction (Portable & Fake)
│   └── vibe-test-support/      # TEST: Hermetic server, PathGuards
├── tools/
│   ├── tmux-vm/                # Version Manager (download/compile tmux)
│   ├── tmux-builder/           # Builder logic for C tmux
│   ├── tmux-sniff/             # Protocol analyzer/proxy
│   ├── vibe-doctor/            # Environment health check
│   └── vibe-tui/               # Rust-native Visual Client (ratatui)
└── xtask/                      # Dev scripts (codegen, tasks)
```

## 6. Layering Contract

We enforce these rules via `cargo-deny` or strict dependency checks:

*   **Rule A (The Core Void):** `vibe-core` MUST NOT depend on `tokio`, `std::net`, `std::fs`, or `libc`. It is strictly `no_std` compatible (conceptually).
*   **Rule B (Unidirectional Flow):** Bindings depend on `vibe-api`. `vibe-api` depends on `vibe-server`. `vibe-server` depends on `vibe-core`. NEVER the reverse.
*   **Rule C (The unsafe Quarantine):** All `unsafe` code related to PTYs, Signals, or raw FDs must reside in `vibe-pty` or `vibe-os` (if needed). These crates must have comprehensive SAFETY comments.
*   **Rule D (Snapshot Reads):** The Facade (`vibe-api`) reads state via immutable snapshots (`Arc<State>`) derived from the Core. It never reads "live" variables. Writes are sent as Events to the Runtime Actor.

## 7. Entity Model (`vibe-core`)

We avoid self-referential structs (`Rc<RefCell<...>>`) in favor of a relational model using `SlotMap`.

```rust
// vibe-core/src/model.rs
use slotmap::{new_key_type, SlotMap};

new_key_type! {
    pub struct SessionId;
    pub struct WindowId;
    pub struct PaneId;
    pub struct ClientId;
}

#[derive(Clone, Debug)]
pub struct State {
    pub sessions: SlotMap<SessionId, Session>,
    pub windows: SlotMap<WindowId, Window>,
    pub panes: SlotMap<PaneId, Pane>,
    pub clients: SlotMap<ClientId, Client>,
    // Adjacency lists for hierarchy
    pub session_windows: SecondaryMap<SessionId, Vec<WindowId>>,
    pub window_panes: SecondaryMap<WindowId, Vec<PaneId>>,
}

pub struct Session {
    pub name: String,
    pub created_at: u64, // Logical timestamp
}

pub struct Window {
    pub name: String,
    pub layout: LayoutNode,
}

pub struct Pane {
    pub width: u16,
    pub height: u16,
    pub grid: Grid, // Terminal grid state
}
```

## 8. Event/Effect Engine (`vibe-core`)

The heartbeat of the system.

```rust
// vibe-core/src/lib.rs

#[derive(Debug, Clone)]
pub enum Event {
    // Input
    ClientInput { client_id: ClientId, bytes: Vec<u8> },
    WindowResize { window_id: WindowId, size: Size },
    // Command
    Command(CommandEnum), // e.g., NewWindow, SplitPane
    // Timer
    Tick(u64),
}

#[derive(Debug, Clone)]
pub enum Effect {
    Render { client_id: ClientId, diff: String },
    WritePty { pane_id: PaneId, bytes: Vec<u8> },
    SpawnPty { command: String, env: HashMap<String, String> },
    Log(String),
}

pub fn apply_event(state: &mut State, event: Event) -> Vec<Effect> {
    // Pure logic: update State, generate Effects
    match event {
        Event::Command(Cmd::SplitPane { window_id, .. }) => {
            // ... Logic to update layout ...
            vec![Effect::SpawnPty { ... }]
        }
        _ => vec![]
    }
}
```

## 9. Protocol Codec (`vibe-protocol`)

Implements the `imsg` protocol used by OpenBSD/tmux.

*   **Structure:** Header (type, len, flags, peerid, pid) + Body.
*   **MsgType:** Enum matching `MSG_IDENTIFY`, `MSG_COMMAND`, `MSG_EXIT`, etc.
*   **Two-Phase Decode:**
    1.  `FrameDecoder`: Reads raw bytes -> `RawFrame` (handling partial reads).
    2.  `MessageDecoder`: `RawFrame` -> `TypedMessage` (deserialization).
*   **Fixture Testing:** Real hex dumps from `tmux-sniff` are used as unit test inputs to verify binary compatibility.

## 10. ORM-like Query API (`vibe-api`)

Inspired by `libtmux`, providing a fluent interface to search the entity graph.

```rust
// vibe-api/src/query.rs

// Django-style lookups supported via traits
pub enum Lookup {
    Exact(String),
    Contains(String),
    StartsWith(String),
    Regex(String),
}

pub struct QueryList<T> {
    items: Vec<T>,
}

impl<T: Queryable> QueryList<T> {
    pub fn filter(&self, query: Query) -> QueryList<T> { ... }
    pub fn get(&self, query: Query) -> Option<T> { ... }
}

// Usage Example (Rust)
// let session = server.sessions()
//    .filter(Query::new("name__startswith", "dev"))
//    .get(Query::new("windows__count__gt", 1));
```

## 11. Runtime Architecture (`vibe-server`)

*   **SocketActor:** One actor per connected client/socket. Handles the TCP/UnixStream, reads `imsg`, sends `Event`s to the core, and processes `Effect`s back to the socket.
*   **StateActor:** Holds the canonical `State`. Serializes updates.
*   **RefreshDriver:** Manages the TTY update loop (dirty checking).
*   **ArcSwap:** The `StateActor` publishes a read-only `Arc<State>` to an `ArcSwap`. The Facade/ORM reads from this `ArcSwap` for instant, lock-free access to the latest state.

## 12. Language Bindings

We use a "Thin Wrapper" philosophy. The logic stays in Rust; bindings just expose the `vibe-api` types.

*   **Python (`vibe-binding-python`):** Uses `PyO3`. Exposes `Server`, `Session`, `Window`, `Pane` classes.
    *   Setup: `uv` for environment, `maturin` for build.
*   **Node (`vibe-binding-node`):** Uses `NAPI-RS`. Exposes async-friendly classes.
    *   Setup: `pnpm`, `napi-rs` CLI.
*   **C++ (`vibe-binding-cpp`):** Uses `cxx`. Exposes a C++ header for embedding Vibe in C++ apps.

## 13. CRDT Transaction Layer (`vibe-crdt`)

Optional layer for collaborative editing.

*   **CrdtOp:** `InsertChar(pos, char, lambda)`, `DeleteChar(pos, lambda)`.
*   **HybridLogicalClock:** For causality tracking.
*   **Merge Strategy:** Yjs/RGA style sequence CRDT.
*   This layer sits *above* `vibe-core`'s basic grid. The core grid is a projection of the CRDT log.

## 14. tmux Version Management

We cannot rely on the system `tmux`.

*   **tmux-vm:** CLI tool to download tmux tarballs, verify signatures, and manage versions in `~/.vibe/tmux-versions/`.
*   **tmux-builder:** Encapsulates `./configure && make` logic, handling dependencies (libevent, ncurses) locally if needed.
*   **tmux-worktrees:** Automates creating git worktrees for `tmux/tmux` repository to test against specific commits.

## 15. Test Support Crate (`vibe-test-support`)

Critical for hermetic testing.

*   **TmuxTestServer:** Spawns a real `tmux` server on a random socket socket (e.g., `/tmp/vibe-test-1234.sock`).
*   **MuxServerTestServer:** Spawns a `vibe-server` in-process or out-of-process on a random socket.
*   **PathGuard:** RAII struct that creates a temp dir and deletes it on drop.
*   **Isolation:** Sets `TMUX_TMPDIR`, `XDG_CONFIG_HOME` to temp paths.

## 16. Fake PTY Backend (`vibe-pty`)

For 100% deterministic tests, we do not want to spawn real OS processes (which are flaky and slow).

*   **FakePty:** A struct implementing `PtyTrait` backed by a `Vec<u8>` buffer.
*   **Scenario Recorder:** Can record a real session's input/output and replay it through `FakePty` to regression test the `vibe-core` parser.

## 17. Binding Test Frameworks

*   **Python:** `pytest` plugin providing `server`, `session`, `window`, `pane` fixtures.
    ```python
    def test_window_split(session):
        window = session.new_window(name="test")
        pane = window.split_window()
        assert len(window.panes) == 2
    ```
*   **Node:** `vitest` fixtures doing the same.

## 18. Test Framework and Harness Design

*   **Unit Tests:** Inside crates (`cargo test`). Focus on pure logic.
*   **Integration Tests:** In `tests/` folder of `vibe-server`. Boot full server, connect client, run scenario.
*   **Compliance Tests:** Run against BOTH `tmux` (real) and `vibe-server` to assert identical behavior.
*   **Snapshot Tests:** Use `insta` to snapshot the `Grid` state (the text on the screen) after operations.

## 19. Visual Client (TUI)

*   **Ratatui:** We will use `ratatui` for the client. It provides the layout and widget primitives we need.
*   **Renderer:** A custom renderer that takes `vibe-core`'s `Grid` and translates it to `ratatui`'s `Buffer`.
*   **Input:** Translates `crossterm` events into Vibe `Event`s.

## 20. DOs and DON'Ts

### DO
*   **DO** use `cargo-deny` to enforce crate boundaries.
*   **DO** use `slotmap` for all entity storage.
*   **DO** implement `Display` and `Debug` for all public types.
*   **DO** write a test case in `vibe-core` for every bug found.
*   **DO** use `tracing` for all logging.

### DON'T
*   **DON'T** use `unwrap()` in `vibe-server` or `vibe-core`. Use `Result`.
*   **DON'T** make blocking IO calls in `async` functions.
*   **DON'T** leak `tokio` types into `vibe-core`.
*   **DON'T** assume the user is on Linux (keep macOS/BSD in mind).
*   **DON'T** hardcode paths. Use `PathGuard` and configuration.

## 21. AGENTS.md Template

```markdown
# Vibe Agent Guidelines

You are an AI assistant working on the Vibe project.

## Context
Vibe is a Rust-based terminal multiplexer ecosystem compatible with tmux.
Current Architecture: Pure Core (vibe-core) -> Runtime (vibe-server) -> Facade (vibe-api).

## Rules
1. **Check Purity:** If editing `vibe-core`, NO IO, NO Async.
2. **Check Protocol:** If touching `vibe-protocol`, verify against `tmux-protocol.h`.
3. **Use Fixtures:** When writing tests, usage `vibe-test-support`. NEVER spawn direct processes.
4. **Update Bindings:** If you change `vibe-api`, update the Python/Node bindings.
5. **Run Snapshots:** Use `cargo insta review` to verify grid changes.

## Common Tasks
- **Add Command:** 1. Add to `vibe-core/commands.rs`, 2. Handle in `apply_event`, 3. Expose in `vibe-api`.
- **Fix Parser:** Add hex dump to `vibe-protocol/tests/fixtures`, then fix code.
```

## 22. Phased Implementation Plan

1.  **Phase 1: The Core (Weeks 1-2)**
    *   Set up workspace.
    *   Implement `vibe-core` Entity Graph (`SlotMap`).
    *   Implement `vibe-protocol` basic codec.
    *   Unit tests for core logic.

2.  **Phase 2: The Runtime & Server (Weeks 3-4)**
    *   Implement `SocketActor` in `vibe-server`.
    *   Connect `vibe-core` to `vibe-server`.
    *   Basic `imsg` handshake with a real `tmux` client.

3.  **Phase 3: The Facade & PTY (Weeks 5-6)**
    *   Implement `vibe-pty` (portable PTYs).
    *   Implement `vibe-api` facade.
    *   First "Hello World" in a pane.

4.  **Phase 4: Bindings & TUI (Weeks 7-8)**
    *   Generate Python bindings.
    *   Build basic `vibe-tui` client.
    *   Verify `libtmux` compatibility.

## 23. Risks and Mitigations

*   **Risk:** `imsg` protocol nuances are undocumented.
    *   *Mitigation:* Use `tmux-sniff` to record traffic from real `tmux` sessions and replay it in tests.
*   **Risk:** PTY behavior differences across OSs.
    *   *Mitigation:* Use `vibe-pty` abstraction and heavily rely on `portable-pty` crate patterns, validating with CI on Linux/Mac.
*   **Risk:** Performance of the Pure Core on large output.
    *   *Mitigation:* Use persistent data structures (or COW) if cloning `State` becomes too expensive, and optimize the `Grid` implementation (using `im-rs` or similar if needed).

***

This document serves as the foundational blueprint for Vibe. All contributions must align with these architectural constraints and vision.
