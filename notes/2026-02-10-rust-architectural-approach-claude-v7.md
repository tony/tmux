# TermForge Architecture Specification v7 (Claude Pass 1)

> **Status:** v7 Pass 1 — Refinement of v6 Final with deepened OTEL SDK types, verified PyO3/Neon binding APIs, expanded Version Management, Binding Tests, and AGENTS.md Rules.
>
> **Lineage:** v4 (2519 lines) -> v5 (1573 lines) -> v6 Pass 1 (merged) -> v6 Pass 2 (expanded Security, OTEL, TUI) -> v6 Pass 3 Final (audit, GPT test matrix) -> **v7 Pass 1** (this document).
>
> **New in v7:** (1) OTEL section verified against actual `opentelemetry-rust` SDK types (`SdkTracerProvider`, `SpanData`, `BatchSpanProcessor`, `SamplingDecision`). (2) Language Bindings verified against actual PyO3 0.26 API (`Bound<'py, T>`, `Python::detach`, `#[pyfunction]`) and Neon API (`FunctionContext`, `Channel`, `#[neon::main]`). (3) Pure logic separation pattern from learning-rust-nodejs incorporated. (4) Version Management expanded with concrete CLI workflows and build orchestration. (5) Binding Test Frameworks expanded with real PyO3/Neon fixture code. (6) AGENTS.md Rules expanded with rationale and enforcement per rule.

---

## Preamble: Settled Decisions

These decisions are final. Do not revisit them.

| # | Decision | Rationale |
|---|---|---|
| S1 | **New multiplexer, tmux-compatible** | Architectural freedom while maintaining wire compatibility. Not a C-to-Rust port. |
| S2 | **Protocol version 8** | Wire-compatible with tmux protocol v8 (`tmux-protocol.h:23`). |
| S3 | **SlotMap entity IDs** | `slotmap::new_key_type!` for all entity IDs (Session, Window, Pane, Client, Job, Buffer). |
| S4 | **Flat arena LayoutTree** | `Vec<LayoutCell>` with index-based parent/children. Not recursive nesting. |
| S5 | **Round-robin resize** | One cell at a time, matching `layout.c:448-462`. Not proportional. |
| S6 | **flock locking** | `flock(LOCK_EX|LOCK_NB)` per `client.c:77-101`. Not PID-based. |
| S7 | **Config after identify** | Load config only after first client completes identify burst (`server-client.c:3725-3734`). |
| S8 | **Protocol violation kills connection** | Matching `server-client.c:3472-3475`. Never drop and continue. |
| S9 | **Custom VT100 parser** | Match tmux's `input.c` state table exactly. Do not use the `vte` crate. |
| S10 | **Option FALLTHROUGH** | WindowPane falls through to Window scope, matching `options.c:891-903`. |

---

## 1. Vision and Philosophy

TermForge is a **new terminal multiplexer** written in Rust that is **wire-compatible** with tmux protocol v8. It is not a port of tmux's C code. It uses tmux as a behavioral reference while building a clean, layered Rust architecture from scratch.

### 1.1 Design Principles

1. **tmux compatibility as a constraint, not a goal.** The goal is a better multiplexer. Compatibility ensures existing workflows are preserved.
2. **Pure core, impure shell.** Business logic is deterministic and testable without IO. Side effects are expressed as `Effect` values dispatched by the runtime.
3. **Layered architecture.** Layer 0 (pure, no IO, WASM-compatible) -> Layer 1 (async runtime) -> Layer 2 (OS/FFI). Dependencies flow inward only.
4. **ORM-like expressiveness.** Traversing the object graph should feel like libtmux: `server.sessions().filter(name="work").windows().first().panes()`.
5. **Test-first sustainability.** Every subsystem is designed for snapshot testing, scenario replay, and property testing. Fake PTY backends enable fully deterministic integration tests.
6. **Language bindings as first-class citizens.** Python (PyO3), Node (Neon/NAPI-RS), and C++ (cxx) bindings expose the same ORM API, with snapshot testing support in pytest and vitest.
7. **Observability built in.** OpenTelemetry tracing from server to client to language bindings, using the `tracing` crate in pure code and OTEL SDK exporters at the runtime boundary.
8. **CRDT-ready state model.** The entity graph supports conflict-free replication for distributed/federated scenarios.

### 1.2 What TermForge Is Not

- Not a port of tmux C code to Rust.
- Not a wrapper around tmux (like libtmux). It is a standalone server.
- Not limited to tmux's feature set. The architecture supports extensions (CRDT sync, rich TUI, programmatic control) that tmux cannot.

---

## 2. North Star Acceptance Criteria

### 2.1 Compatibility (C)

| ID | Criterion | Verification |
|---|---|---|
| C1 | A stock tmux client can attach to a TermForge server | Integration test: `tmux -S /path attach` |
| C2 | A TermForge client can attach to a stock tmux server | Integration test: `termforge attach -S /path` |
| C3 | All 200+ tmux commands parse and execute | `tmux-command-audit` tool with coverage report |
| C4 | Identify burst (types 100-112) roundtrips byte-exact | Fixture test with real tmux captures |
| C5 | Layout strings roundtrip with matching checksums | Property test + parity test against tmux |
| C6 | Control mode notifications parse all known types | Parser test + parity test via `tmux -C` |
| C7 | Option scope resolution matches tmux exactly | Exhaustive matrix test for all (scope, flag, target) combinations |
| C8 | Default key bindings match tmux across all 4 key tables | Generated from `key-bindings.c`, diff-tested |

### 2.2 Performance (P)

| ID | Criterion | Target |
|---|---|---|
| P1 | VT100 parser throughput (plain ASCII) | > 300 MB/s |
| P2 | VT100 parser throughput (CSI heavy) | > 100 MB/s |
| P3 | Protocol frame decode throughput | > 500 MB/s |
| P4 | Layout resize (20 panes) | < 50 us |
| P5 | Format expand (status line) | < 50 us |
| P6 | Graph snapshot build | < 1 ms |

### 2.3 Quality (Q)

| ID | Criterion | Verification |
|---|---|---|
| Q1 | Layer 0 compiles to `wasm32-unknown-unknown` | CI gate: `cargo check --target wasm32-unknown-unknown` |
| Q2 | Zero `unsafe` in Layer 0 crates | `#![forbid(unsafe_code)]` in every Layer 0 crate |
| Q3 | All `unsafe` in `mux-os` with `// SAFETY:` comments | Clippy lint + CI check |
| Q4 | 24-hour fuzz run produces zero panics | `cargo-fuzz` targets for parser, codec, format engine |
| Q5 | Scenario replay produces identical grid snapshots | `insta` snapshot tests with recorded PTY sessions |

### 2.4 Ecosystem (E)

| ID | Criterion | Verification |
|---|---|---|
| E1 | Python bindings: `pip install termforge` | maturin build + PyPI publish |
| E2 | Node bindings: `npm install termforge` | NAPI-RS build + npm publish |
| E3 | pytest fixture chain: server -> session -> window -> pane | pytest plugin with hermetic server |
| E4 | vitest fixture chain: same pattern in TypeScript | vitest setup with hermetic server |
| E5 | Snapshot testing from Python/Node against PTY output | `insta`-compatible snapshots via bindings |

---

## 3. High-Level Architecture

```
+-----------------------------------------------------------------------+
|                         Language Bindings                               |
|  Python (PyO3)  |  Node (Neon)  |  C++ (cxx)  |  Rust (direct)        |
+--------+--------+-------+-------+------+------+-----------+-----------+
         |                |              |                    |
+--------v----------------v--------------v--------------------v---------+
|                           mux-orm                                      |
|  OrmServer, OrmSession, OrmWindow, OrmPane                            |
|  QueryList<T>, .filter(), .get()                                       |
+--------+-------------------------------------------------------------+
         |
+--------v-------------------------------------------------------------+
|                           mux-api                                      |
|  ManagedMux, StateHandle<GraphState>, Intent methods                   |
|  ArcSwap snapshot publish/subscribe boundary                           |
+---------+------------------------------------------------------------+
          |
+---------v------------------------------------------------------------+
|                      mux-core (Layer 0, PURE)                         |
|  ServerGraph, Event, Effect, apply_event()                             |
|  LayoutTree, OptionStore, FormatEngine, KeyTables, CopyMode            |
|  #![forbid(unsafe_code)]  -- compiles to wasm32-unknown-unknown        |
+----------+-----------------------------------------------------------+
           |
+----------v-----------------------------------------------------------+
|                      mux-server (Layer 1)                             |
|  StateActor, EffectDispatcher, AcceptLoop                              |
|  tokio runtime, mpsc channels                                          |
+----------+-----------------------------------------------------------+
           |
+----------v-----------------------------------------------------------+
|                      mux-os (Layer 2)                                 |
|  PTY spawn, flock, SCM_RIGHTS, socket permissions                      |
|  All unsafe code lives here with // SAFETY: comments                   |
+----------------------------------------------------------------------+
```

### 3.1 Dependency Direction

Dependencies flow strictly inward:
- Layer 2 depends on Layer 1 and Layer 0.
- Layer 1 depends on Layer 0.
- Layer 0 depends on nothing outside the workspace except leaf crates (`slotmap`, `smallvec`, `thiserror`, `serde`, `tracing`).
- Bindings depend on `mux-orm` -> `mux-api` -> `mux-core`. Never on runtime internals.

### 3.2 State Ownership

The `ServerGraph` in `mux-core` is the single source of truth. All state mutations flow through `apply_event()`. The runtime publishes immutable snapshots via `ArcSwap` for lock-free reads by the TUI, bindings, and API layer.

---

## 4. Workspace Layout

```
termforge/
  Cargo.toml                          # workspace root
  CLAUDE.md                           # project conventions
  AGENTS.md                           # LLM agent rules (Section 26)

  crates/
    mux-types/                        # Leaf crate: shared ID types, PaneSize, Rect
    mux-core/                         # Layer 0: ServerGraph, Event, Effect, engine
    mux-grid/                         # Layer 0: terminal grid, VT100 parser
    mux-query/                        # Layer 0: QueryList, QueryOp, Queryable trait
    mux-crdt/                         # Layer 0: HLC, DVV, LwwRegister, OrSet
    mux-proto/                        # Layer 0/1: imsg codec, MsgType, frame types
    mux-conf/                         # Layer 0: config parser, option table
    mux-control/                      # Layer 0/1: control mode parser, typed notifications
    mux-view/                         # Layer 0/1: ViewModel construction (pure)
    mux-orm/                          # Layer 1: ORM wrappers over mux-api
    mux-api/                          # Layer 1: ManagedMux, StateHandle, intents
    mux-server/                       # Layer 1: state actor, effect dispatcher, accept loop
    mux-client/                       # Layer 1: client-side connection, control mode
    mux-backend/                      # Layer 1: backend abstraction (TermForge or tmux)
    mux-refresh/                      # Layer 1: periodic state sync from server
    mux-telemetry/                    # Layer 1: OTEL bridge (tracing -> opentelemetry)
    mux-os/                           # Layer 2: PTY, flock, SCM_RIGHTS, sockets
    mux-pty/                          # Layer 1/2: PtyBackend trait + real implementation
    mux-pty-fake/                     # Layer 0: FakePtyBackend, ScenarioRecorder/Replayer
    mux-pty-portable/                 # Layer 2: portable PTY (openpty, forkpty)
    mux-pty-diagnostics/              # Layer 1: PTY diagnostics and debugging
    mux-test-support/                 # Test: TmuxTestServer, PathGuard, hermetic isolation
    mux-command/                      # Layer 0/1: command table, parser, dispatcher

  tools/
    mux-tui/                          # TUI client binary (ratatui-based)
    tmux-vm/                          # tmux version manager
    tmux-builder/                     # tmux source builder
    tmux-worktrees/                   # tmux git worktree manager
    mux-regress/                      # Parity regression runner
    mux-bench/                        # Benchmark suite (criterion)
    tmux-sniff/                       # Protocol sniffer/recorder

  bindings/
    python/                           # PyO3 + maturin
      src/                            # Rust binding glue
      python/termforge/               # Python package
        pytest_plugin.py              # pytest fixtures
    node/                             # Neon / NAPI-RS
      native/src/                     # Rust binding glue
        lib.rs                        # Neon entry point
        logic.rs                      # Pure Rust logic (no Neon types)
      __test__/                       # vitest tests
        setup.ts                      # vitest fixtures
    cxx/                              # C++ bridge (cxx crate)

  fixtures/
    protocol/                         # Real tmux protocol captures
    scenarios/                        # Recorded PTY sessions (JSON)
    configs/                          # Sample tmux.conf files (100+)
    layouts/                          # Layout string test vectors
    grids/                            # Expected grid snapshots
```

### 4.1 Crate Dependency Graph (simplified)

```
mux-types (leaf)
    |
    +-- mux-core ── mux-grid ── mux-query ── mux-crdt ── mux-conf
    |       |
    |       +-- mux-proto ── mux-control ── mux-command
    |       |
    |       +-- mux-view
    |
    +-- mux-api ── mux-orm
    |       |
    |       +-- mux-backend ── mux-refresh
    |
    +-- mux-server ── mux-telemetry
    |
    +-- mux-os ── mux-pty ── mux-pty-portable
    |
    +-- mux-pty-fake (test only)
    |
    +-- mux-test-support (test only)
```

---

## 5. Layering Contract

### 5.1 Layer 0: Pure Logic

**Crates:** `mux-types`, `mux-core`, `mux-grid`, `mux-query`, `mux-crdt`, `mux-conf`, `mux-pty-fake`, `mux-view` (pure subset).

**Rules:**
- `#![forbid(unsafe_code)]` in every crate.
- No `tokio`, no `async`, no `std::net`, no `std::fs`, no `std::process`.
- Must compile to `wasm32-unknown-unknown` (CI gate).
- Time injected via events (`Tick { now_millis }`, `created_at` fields). Never call `SystemTime::now()`.
- Random values injected via `CoreCtx { rand_u64 }`. Never call `rand::random()`.
- All state mutations return `Outcome { effects: Vec<Effect> }`. No side effects.

**Allowed dependencies:** `slotmap`, `smallvec`, `thiserror`, `serde`, `tracing`, `bitflags`, `unicode-width`, `bstr`.

### 5.2 Layer 1: Async Runtime

**Crates:** `mux-api`, `mux-orm`, `mux-server`, `mux-client`, `mux-backend`, `mux-refresh`, `mux-telemetry`, `mux-proto` (codec with bytes/tokio-util), `mux-control`, `mux-command`, `mux-pty`.

**Rules:**
- May use `tokio`, `async`/`await`, `mpsc`, `ArcSwap`.
- Must not use `unsafe` (delegate to Layer 2).
- Protocol types (`MsgType`, `ImsgFrame`) stay in Layer 1. Core uses domain-native types only.

### 5.3 Layer 2: OS/FFI

**Crates:** `mux-os`, `mux-pty-portable`.

**Rules:**
- All `unsafe` code lives here.
- Every `unsafe` block has a `// SAFETY:` comment explaining the invariant.
- Exposes safe Rust APIs to Layer 1.

### 5.4 Enforcement

| Check | Mechanism | CI Gate |
|---|---|---|
| Layer 0 purity | `#![forbid(unsafe_code)]` | Compiler error |
| WASM compilability | `cargo check -p mux-core --target wasm32-unknown-unknown` | CI step |
| No OTEL in core | `cargo tree -p mux-core \| grep opentelemetry` returns empty | CI step |
| No tokio in core | `cargo tree -p mux-core \| grep tokio` returns empty | CI step |
| Unsafe documentation | Custom clippy lint for `// SAFETY:` | CI step |
| Dependency direction | `cargo metadata` cycle check script | CI step |

---

## 6. Entity Model

### 6.1 ID Types (mux-types)

```rust
// crates/mux-types/src/ids.rs

use slotmap::new_key_type;

new_key_type! {
    pub struct SessionId;
    pub struct WindowId;
    pub struct PaneId;
    pub struct ClientId;
    pub struct JobId;
    pub struct BufferId;
}
```

### 6.2 Core Value Types (mux-types)

```rust
// crates/mux-types/src/geometry.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct PaneSize {
    pub cols: u16,
    pub rows: u16,
}

impl PaneSize {
    pub const DEFAULT: Self = Self { cols: 80, rows: 24 };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}
```

### 6.3 Entity Structs (mux-core)

```rust
// crates/mux-core/src/session.rs

#[derive(Debug, Clone)]
pub struct Session {
    pub name: String,
    pub cwd: String,
    pub windows: Vec<WindowId>,
    pub active_window: Option<WindowId>,
    pub last_window: Option<WindowId>,
    pub created_at: i64,
    pub last_attached_at: i64,
    pub destroying: bool,
    pub options: OptionStore,
}

impl Session {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            cwd: String::new(),
            windows: Vec::new(),
            active_window: None,
            last_window: None,
            created_at: 0,
            last_attached_at: 0,
            destroying: false,
            options: OptionStore::default(),
        }
    }

    pub fn add_window(&mut self, window_id: WindowId) {
        if self.active_window.is_none() {
            self.active_window = Some(window_id);
        }
        self.windows.push(window_id);
    }
}
```

```rust
// crates/mux-core/src/window.rs

#[derive(Debug, Clone)]
pub struct Window {
    pub name: String,
    pub panes: Vec<PaneId>,
    pub active_pane: Option<PaneId>,
    pub last_active_pane: Option<PaneId>,
    pub layout_root: LayoutTree,
    pub options: OptionStore,
}

impl Window {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            panes: Vec::new(),
            active_pane: None,
            last_active_pane: None,
            layout_root: LayoutTree::default(),
            options: OptionStore::default(),
        }
    }

    pub fn add_pane(&mut self, pane_id: PaneId) {
        if self.active_pane.is_none() {
            self.active_pane = Some(pane_id);
        }
        self.panes.push(pane_id);
    }
}
```

```rust
// crates/mux-core/src/pane.rs
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Pane {
    pub size: PaneSize,
    pub bounds: Option<Rect>,
    pub exited: bool,
    pub exit_status: Option<i32>,
    pub title: String,
    pub grid: Arc<Grid>,
    pub start_command: Option<String>,
    pub start_path: Option<String>,
}
```

```rust
// crates/mux-core/src/client.rs

#[derive(Debug, Clone)]
pub struct Client {
    pub name: String,
    pub session: Option<SessionId>,
    pub last_session: Option<SessionId>,
    pub tty_name: String,
    pub term_type: String,
}
```

```rust
// crates/mux-core/src/job.rs

#[derive(Debug, Clone)]
pub struct Job {
    pub command: String,
    pub exit_status: Option<i32>,
}
```

### 6.4 ServerGraph

```rust
// crates/mux-core/src/graph.rs

use slotmap::SlotMap;

#[derive(Debug, Default, Clone)]
pub struct ServerGraph {
    pub sessions: SlotMap<SessionId, Session>,
    pub windows: SlotMap<WindowId, Window>,
    pub panes: SlotMap<PaneId, Pane>,
    pub clients: SlotMap<ClientId, Client>,
    pub jobs: SlotMap<JobId, Job>,
    pub buffers: SlotMap<BufferId, Buffer>,
    pub key_tables: KeyTableSet,
    pub global_options: OptionStore,
    pub server_options: OptionStore,
}
```

### 6.5 Snapshot (GraphState)

The `graph_state()` method produces an immutable snapshot with precomputed reverse lookups. This is the read-side data structure published via `ArcSwap`.

Reference: `~/work/rust/vibe-tmux/crates/mux-core/src/graph.rs:215-295` computes `window_to_session` and `pane_to_window` reverse maps during snapshot construction.

```rust
// crates/mux-core/src/state.rs

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct GraphState {
    pub sessions: Vec<SessionState>,
    pub windows: Vec<WindowState>,
    pub panes: Vec<PaneState>,
    pub clients: Vec<ClientState>,
    pub jobs: Vec<JobState>,
}

#[derive(Debug, Clone)]
pub struct SessionState {
    pub id: SessionId,
    pub tmux_id: Option<u32>,
    pub name: String,
    pub windows: Vec<WindowId>,
    pub active_window: Option<WindowId>,
    pub last_window: Option<WindowId>,
    pub destroying: bool,
}

#[derive(Debug, Clone)]
pub struct WindowState {
    pub id: WindowId,
    pub tmux_id: Option<u32>,
    pub session_id: Option<SessionId>,
    pub name: String,
    pub panes: Vec<PaneId>,
    pub active_pane: Option<PaneId>,
    pub last_active_pane: Option<PaneId>,
}

#[derive(Debug, Clone)]
pub struct PaneState {
    pub id: PaneId,
    pub tmux_id: Option<u32>,
    pub window_id: Option<WindowId>,
    pub size: PaneSize,
    pub bounds: Option<Rect>,
    pub exited: bool,
    pub exit_status: Option<i32>,
    pub start_command: Option<String>,
    pub start_path: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ClientState {
    pub id: ClientId,
    pub tmux_id: Option<u32>,
    pub name: String,
    pub session: Option<SessionId>,
    pub last_session: Option<SessionId>,
}

#[derive(Debug, Clone)]
pub struct JobState {
    pub id: JobId,
    pub tmux_id: Option<u32>,
    pub command: String,
    pub exit_status: Option<i32>,
}
```

### 6.6 ClientContext

```rust
// crates/mux-core/src/context.rs

#[derive(Debug, Clone, Default)]
pub struct ClientContext {
    pub client_id: Option<ClientId>,
    pub session_id: Option<SessionId>,
    pub window_id: Option<WindowId>,
    pub pane_id: Option<PaneId>,
}

impl ClientContext {
    pub fn with_client(client_id: ClientId) -> Self {
        Self {
            client_id: Some(client_id),
            ..Default::default()
        }
    }
}
```

### 6.7 Entity Model Test Strategy

1. **Graph links:** Create session -> window -> pane chain, verify `active_window` and `active_pane` set correctly.
2. **Client attach:** Attach client to session, verify `client.session` set.
3. **Client context resolution:** Full chain: client -> session -> window -> pane via `client_context()`.
4. **Graph state snapshot:** Create all entity types, verify `graph_state()` contains correct counts and IDs.
5. **Reverse lookup:** Verify `window_state.session_id` computed correctly in snapshot.
6. **Multiple sessions:** 3 sessions with overlapping window counts, snapshot preserves all.

---

## 7. Event/Effect Engine

### 7.1 Events

```rust
// crates/mux-core/src/event.rs

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Event {
    // Session lifecycle
    CreateSession { name: String, cwd: Option<String>, created_at: i64 },
    DestroySession { session_id: SessionId },
    RenameSession { session_id: SessionId, name: String },

    // Window lifecycle
    CreateWindow { session_id: SessionId, name: String },
    DestroyWindow { window_id: WindowId },
    RenameWindow { window_id: WindowId, name: String },

    // Pane lifecycle
    CreatePane { window_id: WindowId, size: PaneSize, command: Vec<String>, cwd: Option<String> },
    DestroyPane { pane_id: PaneId },
    SplitWindow { window_id: WindowId, direction: SplitDirection, size: PaneSize,
                  command: Vec<String>, cwd: Option<String> },

    // Pane I/O
    PaneOutput { pane_id: PaneId, data: Vec<u8> },
    PaneExited { pane_id: PaneId, exit_status: i32 },
    ResizePane { pane_id: PaneId, size: PaneSize },

    // Client
    ClientAttach { client_id: ClientId, session_id: SessionId },
    ClientDetach { client_id: ClientId },

    // Input
    Key { client_id: ClientId, key: KeyCode },

    // Commands
    Command { name: String, args: Vec<String>, client_id: Option<ClientId> },

    // Configuration
    SetOption { scope: OptionScope, key: String, value: OptionValue },
    UnsetOption { scope: OptionScope, key: String },

    // Time
    Tick { now_millis: i64 },

    // Error feedback from runtime
    EffectFailed { token: EffectToken, reason: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}
```

### 7.2 Effects

```rust
// crates/mux-core/src/effect.rs

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Effect {
    // PTY operations (dispatched by runtime to mux-os)
    SpawnPane { pane_id: PaneId, command: Vec<String>, cwd: Option<String>, size: PaneSize },
    KillPane { pane_id: PaneId, signal: i32 },
    WritePane { pane_id: PaneId, data: Vec<u8> },
    ResizePty { pane_id: PaneId, size: PaneSize },

    // Client communication
    SendToClient { client_id: ClientId, frame: FrameData },
    NotifyClient { client_id: ClientId, message: String },
    DisconnectClient { client_id: ClientId },
    ErrorReply { client_id: ClientId, message: String },

    // Rendering
    Redraw { scope: RedrawScope },

    // Server lifecycle
    Shutdown,
    PublishSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedrawScope {
    All,
    Client(ClientId),
    Pane(PaneId),
    StatusLine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EffectToken(pub u64);
```

### 7.3 Core Context and Outcome

```rust
// crates/mux-core/src/engine.rs

/// Injected context for deterministic event processing.
pub struct CoreCtx {
    pub now_ms: i64,
    pub rand_u64: u64,
}

/// Result of processing a single event.
pub struct Outcome {
    pub effects: Vec<Effect>,
}

/// The pure event processor. All state mutations happen here.
pub fn apply_event(graph: &mut ServerGraph, event: Event, ctx: &CoreCtx) -> Outcome {
    match event {
        Event::CreateSession { name, cwd, created_at } => {
            let mut session = Session::new(name);
            session.created_at = created_at;
            if let Some(cwd) = cwd {
                session.cwd = cwd;
            }
            let session_id = graph.sessions.insert(session);
            Outcome {
                effects: vec![Effect::PublishSnapshot],
            }
        }
        Event::CreatePane { window_id, size, command, cwd } => {
            let pane_id = graph.panes.insert(Pane::new(size));
            if let Some(window) = graph.windows.get_mut(window_id) {
                window.add_pane(pane_id);
            }
            Outcome {
                effects: vec![
                    Effect::SpawnPane { pane_id, command, cwd, size },
                    Effect::PublishSnapshot,
                ],
            }
        }
        // ... remaining event handlers
        _ => Outcome { effects: vec![] },
    }
}
```

### 7.4 Event/Effect Test Strategy

1. **CreateSession:** Apply event, verify session in graph, verify `PublishSnapshot` effect.
2. **CreatePane:** Apply event, verify pane in graph and linked to window, verify `SpawnPane` effect.
3. **Determinism:** Same event + same `CoreCtx` on two identical graphs -> identical outcomes.
4. **EffectFailed feedback:** `EffectFailed` event produces `ErrorReply` effect.
5. **Destroy cascade:** Destroy session with 3 windows and 5 panes -> correct `KillPane` effects.
6. **No side effects:** `apply_event` never calls OS functions (enforced by Layer 0 purity).

---

## 8. Error Handling

### 8.1 Error Classification

```rust
// crates/mux-types/src/error.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorClass {
    /// Temporary failure (timeout, EAGAIN). Retry is safe.
    Transient,
    /// Binary protocol violation. Connection must be killed.
    ProtocolViolation,
    /// User-caused error (bad command, invalid argument). Report and continue.
    UserError,
    /// Internal logic error. This is a bug.
    Bug,
}

pub trait Classified {
    fn class(&self) -> ErrorClass;
    fn is_fatal(&self) -> bool {
        matches!(self.class(), ErrorClass::ProtocolViolation | ErrorClass::Bug)
    }
}
```

### 8.2 Per-Crate Error Types

Every library crate defines a public `Error` enum deriving `thiserror::Error`. `anyhow` is allowed only in binary crates (`mux-tui`, `tmux-vm`, etc.) and test code.

```rust
// Example: crates/mux-proto/src/error.rs

#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("frame too large: {len} bytes (max {max})")]
    FrameTooLarge { len: usize, max: usize },

    #[error("unknown message type: {msg_type}")]
    UnknownMessageType { msg_type: u32 },

    #[error("incomplete header: need {need} bytes, have {have}")]
    IncompleteHeader { need: usize, have: usize },

    #[error("invalid payload for message type {msg_type}: {reason}")]
    InvalidPayload { msg_type: u32, reason: String },
}

impl Classified for ProtocolError {
    fn class(&self) -> ErrorClass {
        match self {
            Self::IncompleteHeader { .. } => ErrorClass::Transient,
            _ => ErrorClass::ProtocolViolation,
        }
    }
}
```

### 8.3 Decode Outcome

```rust
// crates/mux-proto/src/decode.rs

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeOutcome {
    /// Need more bytes to complete a frame.
    NeedMore,
    /// Successfully decoded a frame.
    Frame(ImsgFrame),
    /// Protocol violation. Connection must be killed.
    ProtocolViolation(ProtocolError),
}
```

### 8.4 Pure Error Boundary

`io::Error` and other platform types must not cross the pure/impure boundary. The runtime converts IO failures to `Event::EffectFailed { token, reason }` before submitting to the state actor.

```rust
// At the runtime boundary:
match pty_pool.spawn(pane_id, &command, cwd, size).await {
    Ok(handle) => { /* store handle */ }
    Err(e) => {
        let _ = event_tx.send(Event::EffectFailed {
            token: effect_token,
            reason: e.to_string(),
        }).await;
    }
}
```

### 8.5 Error Handling Test Strategy

1. **Classification:** Each error type classifies correctly (`Transient`, `ProtocolViolation`, etc.).
2. **Fatal check:** `is_fatal()` returns `true` for `ProtocolViolation` and `Bug`.
3. **Decode outcome:** Incomplete data -> `NeedMore`. Complete valid frame -> `Frame`. Invalid length -> `ProtocolViolation`.
4. **Error boundary:** `io::Error` at runtime boundary -> `EffectFailed` event -> `ErrorReply` effect.
5. **No unwrap:** `grep -r 'unwrap()' crates/mux-core/src/` returns zero hits (CI check).

---

## 9. Protocol Codec

### 9.1 Wire Format

tmux uses the OpenBSD `imsg` protocol: a 16-byte header followed by a variable-length payload. Reference: `tmux-protocol.h:23` defines `PROTOCOL_VERSION 8`.

```rust
// crates/mux-proto/src/frame.rs

use bytes::BytesMut;

pub const IMSG_HEADER_SIZE: usize = 16;
pub const IMSG_MAX_SIZE: usize = 16384;

#[derive(Debug, Clone)]
pub struct ImsgHdr {
    pub msg_type: u32,
    pub len: u16,
    pub peerid: u16,
    pub pid: u32,
    pub has_fd: bool,
}

#[derive(Debug, Clone)]
pub struct ImsgFrame {
    pub header: ImsgHdr,
    pub payload: BytesMut,
}

impl ImsgHdr {
    #[must_use]
    pub fn payload_len(&self) -> usize {
        (self.len as usize).saturating_sub(IMSG_HEADER_SIZE)
    }
}
```

### 9.2 Message Types

Reference: `tmux-protocol.h:29-41` defines the identify burst types (MSG_IDENTIFY_FLAGS=100 through MSG_IDENTIFY_TERMINFO=112).

```rust
// crates/mux-proto/src/msg_type.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[repr(u32)]
pub enum MsgType {
    // Client-server commands
    Version = 12,
    Identify = 100,
    Command = 200,
    // ... all message types

    // Identify burst (100-112)
    IdentifyFlags = 100,
    IdentifyTerm = 101,
    IdentifyTtyname = 102,
    IdentifyOldcwd = 103,
    IdentifyStdin = 104,
    IdentifyEnviron = 105,
    IdentifyDone = 106,
    IdentifyClientpid = 107,
    IdentifyCwd = 108,
    IdentifyFeatures = 109,
    IdentifyLongflags = 110,
    IdentifyStdout = 111,
    IdentifyTerminfo = 112,
}
```

### 9.3 Codec (tokio-util)

```rust
// crates/mux-proto/src/codec.rs

use bytes::{Buf, BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

pub struct ImsgCodec;

impl ImsgCodec {
    pub fn new() -> Self { Self }
}

impl Decoder for ImsgCodec {
    type Item = ImsgFrame;
    type Error = ProtocolError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < IMSG_HEADER_SIZE {
            return Ok(None); // NeedMore
        }

        let len = u16::from_le_bytes([src[4], src[5]]) as usize;

        if len < IMSG_HEADER_SIZE {
            return Err(ProtocolError::FrameTooLarge { len, max: IMSG_MAX_SIZE });
        }
        if len > IMSG_MAX_SIZE {
            return Err(ProtocolError::FrameTooLarge { len, max: IMSG_MAX_SIZE });
        }
        if src.len() < len {
            src.reserve(len - src.len());
            return Ok(None); // NeedMore
        }

        let header_bytes = src.split_to(IMSG_HEADER_SIZE);
        let payload_len = len - IMSG_HEADER_SIZE;
        let payload = src.split_to(payload_len);

        let header = ImsgHdr {
            msg_type: u32::from_le_bytes([
                header_bytes[0], header_bytes[1], header_bytes[2], header_bytes[3],
            ]),
            len: len as u16,
            peerid: u16::from_le_bytes([header_bytes[6], header_bytes[7]]),
            pid: u32::from_le_bytes([
                header_bytes[8], header_bytes[9], header_bytes[10], header_bytes[11],
            ]),
            has_fd: header_bytes[12] != 0,
        };

        Ok(Some(ImsgFrame { header, payload }))
    }
}

impl Encoder<ImsgFrame> for ImsgCodec {
    type Error = ProtocolError;

    fn encode(&mut self, item: ImsgFrame, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let total_len = IMSG_HEADER_SIZE + item.payload.len();
        dst.reserve(total_len);
        dst.put_u32_le(item.header.msg_type);
        dst.put_u16_le(total_len as u16);
        dst.put_u16_le(item.header.peerid);
        dst.put_u32_le(item.header.pid);
        dst.put_u8(if item.header.has_fd { 1 } else { 0 });
        dst.put_bytes(0, 3); // padding
        dst.extend_from_slice(&item.payload);
        Ok(())
    }
}
```

### 9.4 Identify Burst Collector

```rust
// crates/mux-proto/src/identify.rs

/// Collect identify burst messages (types 100-112) into a complete client identity.
pub struct IdentifyCollector {
    pub flags: Option<u64>,
    pub term: Option<String>,
    pub ttyname: Option<String>,
    pub cwd: Option<String>,
    pub environ: Vec<String>,
    pub features: Option<u64>,
    pub client_pid: Option<u32>,
    pub stdin_fd: Option<i32>,
    pub stdout_fd: Option<i32>,
    pub done: bool,
}

impl IdentifyCollector {
    pub fn new() -> Self {
        Self {
            flags: None, term: None, ttyname: None, cwd: None,
            environ: Vec::new(), features: None, client_pid: None,
            stdin_fd: None, stdout_fd: None, done: false,
        }
    }

    /// Process an identify message. Returns true when IdentifyDone is received.
    pub fn feed(&mut self, msg_type: MsgType, payload: &[u8]) -> Result<bool, ProtocolError> {
        if self.done {
            return Err(ProtocolError::DuplicateIdentify);
        }
        match msg_type {
            MsgType::IdentifyFlags => {
                if payload.len() >= 8 {
                    self.flags = Some(u64::from_le_bytes(payload[..8].try_into().unwrap()));
                }
            }
            MsgType::IdentifyTerm => {
                self.term = Some(parse_nul_string(payload));
            }
            MsgType::IdentifyTtyname => {
                self.ttyname = Some(parse_nul_string(payload));
            }
            MsgType::IdentifyCwd => {
                self.cwd = Some(parse_nul_string(payload));
            }
            MsgType::IdentifyEnviron => {
                self.environ.push(parse_nul_string(payload));
            }
            MsgType::IdentifyDone => {
                self.done = true;
                return Ok(true);
            }
            _ => { /* other identify types: store as needed */ }
        }
        Ok(false)
    }
}

fn parse_nul_string(data: &[u8]) -> String {
    let end = data.iter().position(|&b| b == 0).unwrap_or(data.len());
    String::from_utf8_lossy(&data[..end]).into_owned()
}
```

### 9.5 Protocol Codec Test Strategy

1. **Roundtrip:** Encode frame, decode frame, verify identical.
2. **Incomplete header:** Feed 10 bytes, verify `NeedMore`.
3. **Incomplete payload:** Feed header + partial payload, verify `NeedMore`.
4. **Frame too large:** Feed header with `len > MAX_IMSGSIZE`, verify `ProtocolViolation`.
5. **Frame too small:** Feed header with `len < 16`, verify `ProtocolViolation`.
6. **Identify collector:** Feed all 13 identify types in order, verify `done == true` after IdentifyDone.
7. **Duplicate identify:** Feed IdentifyDone twice, verify error.
8. **Real tmux capture:** Replay `fixtures/protocol/identify-burst.bin`, verify all fields parsed.
9. **Fuzz:** `cargo-fuzz` target for `ImsgCodec::decode` with random bytes.

---

## 10. Configuration System

### 10.1 Option Table

```rust
// crates/mux-conf/src/option_table.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OptionScope {
    Server,
    Session,
    Window,
    Pane,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OptionValue {
    String(String),
    Number(i64),
    Flag(bool),
    Style(String),
    Array(Vec<String>),
}

#[derive(Debug, Clone, Default)]
pub struct OptionStore {
    local: BTreeMap<String, OptionValue>,
}

impl OptionStore {
    pub fn get(&self, key: &str) -> Option<&OptionValue> {
        self.local.get(key)
    }

    pub fn set(&mut self, key: &str, value: OptionValue) {
        self.local.insert(key.to_string(), value);
    }

    pub fn unset(&mut self, key: &str) -> Option<OptionValue> {
        self.local.remove(key)
    }
}
```

### 10.2 Option Table Entry

```rust
// crates/mux-conf/src/option_table.rs

#[derive(Debug, Clone)]
pub struct OptionTableEntry {
    pub name: &'static str,
    pub scope: OptionScope,
    pub kind: OptionKind,
    pub default: OptionValue,
    /// If true, this option falls through from WindowPane to Window scope.
    /// Reference: options.c:891-903
    pub fallthrough: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionKind {
    String,
    Number { min: i64, max: i64 },
    Flag,
    Style,
    Array,
    Choice(&'static [&'static str]),
}
```

### 10.3 Scope Resolution

Reference: `options.c:228-241` walks the parent chain. `options.c:891-903` implements FALLTHROUGH from WindowPane to Window.

```rust
// crates/mux-conf/src/resolve.rs

/// Resolve an option value using scope chain with FALLTHROUGH.
///
/// Resolution order (from `options_get` at options.c:228-241):
/// 1. Check the specific entity's local store.
/// 2. Walk up to parent scope (Session -> Server for session options, etc.).
/// 3. For WindowPane scope with `fallthrough: true`, check Window scope first
///    before falling back to Session (options.c:891-903).
/// 4. Return compiled default.
pub fn resolve_option(
    graph: &ServerGraph,
    table_entry: &OptionTableEntry,
    target: TargetId,
) -> OptionValue {
    match target {
        TargetId::Pane(pane_id) => {
            // Check pane-local
            if let Some(pane) = graph.panes.get(pane_id) {
                if let Some(val) = pane.options.get(table_entry.name) {
                    return val.clone();
                }
            }
            // FALLTHROUGH: check window scope (options.c:903)
            if table_entry.fallthrough {
                if let Some(window_id) = find_window_for_pane(graph, pane_id) {
                    if let Some(window) = graph.windows.get(window_id) {
                        if let Some(val) = window.options.get(table_entry.name) {
                            return val.clone();
                        }
                    }
                }
            }
            // Walk up to session, then global
            // ... parent chain walk
            table_entry.default.clone()
        }
        TargetId::Window(window_id) => {
            if let Some(window) = graph.windows.get(window_id) {
                if let Some(val) = window.options.get(table_entry.name) {
                    return val.clone();
                }
            }
            // Walk up to session, then global
            table_entry.default.clone()
        }
        TargetId::Session(session_id) => {
            if let Some(session) = graph.sessions.get(session_id) {
                if let Some(val) = session.options.get(table_entry.name) {
                    return val.clone();
                }
            }
            // Walk up to global
            if let Some(val) = graph.global_options.get(table_entry.name) {
                return val.clone();
            }
            table_entry.default.clone()
        }
        TargetId::Server => {
            if let Some(val) = graph.server_options.get(table_entry.name) {
                return val.clone();
            }
            table_entry.default.clone()
        }
    }
}
```

### 10.4 Unset Semantics

Reference: `options.c:1269-1285` (`options_remove_or_default`).

- `set -u <option>`: Remove local override. Inheritance restored for non-global. Reset to compiled default for global.
- `set -U <option>`: Additionally clears pane-local values (cascade unset).
- Array index unset: `set -u status-format[2]` removes index 2, shifts remaining. Matches `options.c:1282`.

### 10.5 Config-as-Events

Config file parsing produces `Vec<Event>` submitted through the state actor. Direct graph mutation from config parsing is forbidden.

```rust
// crates/mux-conf/src/parser.rs

pub fn parse_config(source: &str) -> Result<Vec<Event>, Vec<ConfigError>> {
    let mut events = Vec::new();
    let mut errors = Vec::new();

    for (line_num, line) in source.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        match parse_command_line(line) {
            Ok(event) => events.push(event),
            Err(e) => errors.push(ConfigError {
                line: line_num + 1,
                message: e.to_string(),
                source_line: line.to_string(),
            }),
        }
    }

    if errors.is_empty() {
        Ok(events)
    } else {
        Err(errors)
    }
}
```

### 10.6 Configuration Test Strategy

1. **Scope resolution chain:** Set at pane -> window -> session -> global. Verify pane reads pane-local.
2. **FALLTHROUGH:** Set option at window scope with `fallthrough: true`, resolve from pane, verify window value returned.
3. **Unset restore:** Set at pane, unset, verify inherits from window.
4. **Global unset:** Set global, unset, verify compiled default.
5. **Array index unset:** Set 3-element array, unset index 1, verify shift.
6. **Config-as-events:** Parse 3-line config, verify 3 `SetOption` events produced.
7. **Config error reporting:** Parse invalid config line, verify `ConfigError` with line number.
8. **Real config corpus:** Parse 100+ real `tmux.conf` files from `fixtures/configs/`, zero panics.

---

## 11. Layout Engine

### 11.1 Flat Arena (Settled)

The layout tree uses a flat `Vec<LayoutCell>` arena with index-based parent/children references. This matches tmux's linked-list tree structure while using Rust-idiomatic arena allocation.

```rust
// crates/mux-core/src/layout.rs

pub const PANE_MINIMUM: u16 = 1; // Reference: tmux.h:100

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutType {
    WindowPane,
    LeftRight,
    TopBottom,
}

#[derive(Debug, Clone)]
pub struct LayoutCell {
    pub cell_type: LayoutType,
    pub sx: u16,
    pub sy: u16,
    pub xoff: u16,
    pub yoff: u16,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub pane_id: Option<PaneId>,
}

#[derive(Debug, Clone, Default)]
pub struct LayoutTree {
    pub cells: Vec<LayoutCell>,
}
```

### 11.2 Layout Checksum

Reference: `layout-custom.c:46-57`. The checksum uses rotate-right by 1 bit then add each byte. Must match tmux exactly for wire compatibility.

```rust
// crates/mux-core/src/layout.rs

/// Compute the tmux layout checksum.
/// Algorithm: rotate right by 1, then add each byte.
/// Reference: layout-custom.c:46-57
#[must_use]
pub fn layout_checksum(layout_str: &[u8]) -> u16 {
    let mut csum: u16 = 0;
    for &byte in layout_str {
        csum = (csum >> 1) | ((csum & 1) << 15);
        csum = csum.wrapping_add(byte as u16);
    }
    csum
}
```

### 11.3 Layout Dump and Parse

```rust
// crates/mux-core/src/layout.rs

impl LayoutTree {
    /// Dump layout to tmux format string (without checksum prefix).
    #[must_use]
    pub fn dump(&self) -> String {
        if self.cells.is_empty() {
            return String::new();
        }
        self.dump_cell(0)
    }

    fn dump_cell(&self, idx: usize) -> String {
        let cell = &self.cells[idx];
        let size = format!("{}x{},{},{}", cell.sx, cell.sy, cell.xoff, cell.yoff);

        if cell.children.is_empty() {
            // Leaf: append pane index
            let pane_idx = self.leaf_index(idx);
            return format!("{},{}", size, pane_idx);
        }

        let sep = match cell.cell_type {
            LayoutType::LeftRight => '{',
            LayoutType::TopBottom => '[',
            _ => unreachable!("non-leaf must be LeftRight or TopBottom"),
        };
        let end = match cell.cell_type {
            LayoutType::LeftRight => '}',
            LayoutType::TopBottom => ']',
            _ => unreachable!(),
        };

        let children: Vec<String> = cell.children.iter()
            .map(|&c| self.dump_cell(c))
            .collect();

        format!("{}{}{}{}", size, sep, children.join(","), end)
    }

    /// Parse a tmux layout string (without checksum prefix).
    pub fn parse(layout_str: &str) -> Result<Self, LayoutError> {
        let mut tree = LayoutTree { cells: Vec::new() };
        let mut chars = layout_str.chars().peekable();
        tree.parse_cell(&mut chars, None)?;
        Ok(tree)
    }
}
```

### 11.4 Layout Validation

Reference: `layout-custom.c:119-153`. Validates that cell sizes are consistent with children sizes plus borders.

```rust
// crates/mux-core/src/layout.rs

impl LayoutTree {
    /// Validate layout consistency.
    /// Reference: layout-custom.c:119-153
    /// Each parent's size must equal the sum of children sizes plus borders.
    /// Border = +1 per child - 1 total (for separator lines).
    #[must_use]
    pub fn layout_check(&self) -> bool {
        if self.cells.is_empty() {
            return true;
        }
        self.check_cell(0)
    }

    fn check_cell(&self, idx: usize) -> bool {
        let cell = &self.cells[idx];
        if cell.children.is_empty() {
            // Leaf: valid if size >= PANE_MINIMUM
            return cell.sx >= PANE_MINIMUM && cell.sy >= PANE_MINIMUM;
        }

        // Recursive check children
        for &child_idx in &cell.children {
            if !self.check_cell(child_idx) {
                return false;
            }
        }

        let n_children = cell.children.len() as u16;
        let borders = n_children.saturating_sub(1); // separator lines between children

        match cell.cell_type {
            LayoutType::LeftRight => {
                // Sum of children widths + borders must equal parent width
                let child_width_sum: u16 = cell.children.iter()
                    .map(|&c| self.cells[c].sx)
                    .sum();
                if child_width_sum + borders != cell.sx {
                    return false;
                }
                // All children must have same height as parent
                cell.children.iter()
                    .all(|&c| self.cells[c].sy == cell.sy)
            }
            LayoutType::TopBottom => {
                // Sum of children heights + borders must equal parent height
                let child_height_sum: u16 = cell.children.iter()
                    .map(|&c| self.cells[c].sy)
                    .sum();
                if child_height_sum + borders != cell.sy {
                    return false;
                }
                // All children must have same width as parent
                cell.children.iter()
                    .all(|&c| self.cells[c].sx == cell.sx)
            }
            LayoutType::WindowPane => {
                // Leaf should not have children
                false
            }
        }
    }
}
```

### 11.5 Layout Resize

Reference: `layout.c:448-462` -- round-robin distribution, one cell at a time.

```rust
// crates/mux-core/src/layout.rs

impl LayoutTree {
    /// Resize the layout tree to new dimensions.
    /// Uses round-robin distribution matching layout.c:448-462.
    /// Never fails; clamps to minimum per layout_resize_check().
    pub fn resize(&mut self, new_width: u16, new_height: u16) {
        if self.cells.is_empty() {
            return;
        }

        let old_width = self.cells[0].sx;
        let old_height = self.cells[0].sy;

        if new_width != old_width {
            self.resize_dimension(0, new_width, true);
        }
        if new_height != old_height {
            self.resize_dimension(0, new_height, false);
        }
    }

    fn resize_dimension(&mut self, idx: usize, new_size: u16, horizontal: bool) {
        let cell = &self.cells[idx];
        let is_split_direction = if horizontal {
            cell.cell_type == LayoutType::LeftRight
        } else {
            cell.cell_type == LayoutType::TopBottom
        };

        if cell.children.is_empty() {
            // Leaf: directly resize
            if horizontal {
                self.cells[idx].sx = new_size.max(PANE_MINIMUM);
            } else {
                self.cells[idx].sy = new_size.max(PANE_MINIMUM);
            }
            return;
        }

        if is_split_direction {
            // Distribute delta across children, round-robin, one cell at a time.
            // Reference: layout.c:448-462
            let children = self.cells[idx].children.clone();
            let old_size: u16 = children.iter()
                .map(|&c| if horizontal { self.cells[c].sx } else { self.cells[c].sy })
                .sum();
            let borders = children.len().saturating_sub(1) as u16;
            let available = new_size.saturating_sub(borders);

            // Compute minimum sizes per child
            let min_per_child = children.iter()
                .map(|&c| self.layout_resize_check(c, horizontal))
                .collect::<Vec<_>>();

            let total_min: u16 = min_per_child.iter().sum();
            let target = available.max(total_min);

            // Start from minimum, distribute remainder round-robin
            let mut sizes: Vec<u16> = min_per_child.clone();
            let mut remaining = target.saturating_sub(total_min);
            let mut round_robin_idx = 0;

            while remaining > 0 {
                sizes[round_robin_idx] += 1;
                remaining -= 1;
                round_robin_idx = (round_robin_idx + 1) % children.len();
            }

            // Apply sizes to children
            let mut offset = if horizontal { self.cells[idx].xoff } else { self.cells[idx].yoff };
            for (i, &child_idx) in children.iter().enumerate() {
                self.resize_dimension(child_idx, sizes[i], horizontal);
                if horizontal {
                    self.cells[child_idx].xoff = offset;
                    offset += sizes[i] + 1; // +1 for border
                } else {
                    self.cells[child_idx].yoff = offset;
                    offset += sizes[i] + 1;
                }
            }
        } else {
            // Perpendicular: resize all children to new_size
            let children = self.cells[idx].children.clone();
            for &child_idx in &children {
                self.resize_dimension(child_idx, new_size, horizontal);
            }
        }

        // Update parent size
        if horizontal {
            self.cells[idx].sx = new_size;
        } else {
            self.cells[idx].sy = new_size;
        }
    }

    /// Compute minimum size for a subtree.
    /// Reference: layout.c:366-415
    fn layout_resize_check(&self, idx: usize, horizontal: bool) -> u16 {
        let cell = &self.cells[idx];
        if cell.children.is_empty() {
            return PANE_MINIMUM;
        }

        let is_split_direction = if horizontal {
            cell.cell_type == LayoutType::LeftRight
        } else {
            cell.cell_type == LayoutType::TopBottom
        };

        if is_split_direction {
            let borders = cell.children.len().saturating_sub(1) as u16;
            let child_min: u16 = cell.children.iter()
                .map(|&c| self.layout_resize_check(c, horizontal))
                .sum();
            child_min + borders
        } else {
            cell.children.iter()
                .map(|&c| self.layout_resize_check(c, horizontal))
                .max()
                .unwrap_or(PANE_MINIMUM)
        }
    }
}
```

### 11.6 Arena Compaction

```rust
// crates/mux-core/src/layout.rs

impl LayoutTree {
    /// Remove unreachable cells from the arena.
    /// Call after destroy operations to prevent unbounded arena growth.
    pub fn compact(&mut self) {
        if self.cells.is_empty() {
            return;
        }
        let mut reachable = vec![false; self.cells.len()];
        self.mark_reachable(0, &mut reachable);

        let mut mapping = vec![0usize; self.cells.len()];
        let mut new_cells = Vec::new();
        for (old_idx, cell) in self.cells.iter().enumerate() {
            if reachable[old_idx] {
                mapping[old_idx] = new_cells.len();
                new_cells.push(cell.clone());
            }
        }
        for cell in &mut new_cells {
            cell.parent = cell.parent.map(|p| mapping[p]);
            cell.children = cell.children.iter()
                .map(|&c| mapping[c])
                .collect();
        }
        self.cells = new_cells;
    }

    fn mark_reachable(&self, idx: usize, reachable: &mut Vec<bool>) {
        reachable[idx] = true;
        for &child in &self.cells[idx].children {
            self.mark_reachable(child, reachable);
        }
    }
}
```

### 11.7 Layout Presets

tmux provides 7 built-in layout presets (even-horizontal, even-vertical, main-horizontal, main-vertical, tiled, etc.) via `layout-set.c`. Each preset is implemented as a function that arranges N panes into a `LayoutTree`.

```rust
pub enum LayoutPreset {
    EvenHorizontal,
    EvenVertical,
    MainHorizontal,
    MainVertical,
    Tiled,
    MainHorizontalMirrored,
    MainVerticalMirrored,
}

impl LayoutPreset {
    /// Arrange N panes into a layout tree.
    pub fn arrange(
        &self,
        panes: &[PaneId],
        width: u16,
        height: u16,
        main_size: Option<u16>,
    ) -> LayoutTree {
        // ... preset-specific layout algorithm
        todo!()
    }
}
```

### 11.8 Split Constraints

Reference: `layout.c:937-950` -- split minimum check: `PANE_MINIMUM * 2 + 1` (two panes plus a border).

```rust
pub fn can_split(cell: &LayoutCell, direction: SplitDirection) -> bool {
    let available = match direction {
        SplitDirection::Horizontal => cell.sx,
        SplitDirection::Vertical => cell.sy,
    };
    available >= PANE_MINIMUM * 2 + 1
}
```

### 11.9 Layout Test Strategy

1. **Roundtrip property:** `layout_parse(layout_dump(tree)) == tree`.
2. **Checksum parity:** Compare against tmux-generated layout strings.
3. **layout_check invariant:** After every mutation (split, resize, destroy, compact), `layout_check(tree)` returns true. Add as `debug_assert!` in all mutating methods.
4. **Round-robin distribution:** Split 3 panes at 100 cols, resize to 103, verify one cell at a time gets +1.
5. **Pane assignment parity:** Parse tmux layout string with N cells, verify depth-first assignment.
6. **Single pane:** dump/parse roundtrip.
7. **Maximum depth:** 20 levels of nesting.
8. **Resize to 1x1:** all panes at `PANE_MINIMUM`.
9. **Split at minimum:** returns `LayoutError::TooSmall` (matches `layout.c:937-944`).
10. **Destroy last child:** parent collapses.
11. **Compact after 10 destroy ops:** no dead cells.
12. **Preset parity:** For each of 7 presets with N=1..20 panes, compare layout dump against tmux `select-layout` output.

---

## 12. ORM-like Query API

### Architecture (cross-ref: Section 6 Entity Model, Section 16 Bindings)

```
mux-types   (IDs, shared structs)
    |
mux-query   (QueryList, QueryOp, Queryable trait)  -- PURE
    |
mux-core    (ServerGraph, engine)
    |
mux-api     (ManagedMux, StateHandle, intents)
    |
mux-orm     (OrmServer, OrmSession, OrmWindow, OrmPane)
    |
bindings    (Python, Node, C++)
```

### QueryOp and QueryList (pure, in mux-query)

```rust
// crates/mux-query/src/ops.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryOp {
    Exact, Contains, IContains, StartsWith, IStartsWith,
    EndsWith, IEndsWith, Regex, In, Gt, Gte, Lt, Lte,
    IsNone, IsSome,
}

#[derive(Debug, Clone)]
pub enum QueryValue {
    String(String), Int(i64), Bool(bool),
    StringList(Vec<String>), Regex(String), None,
}

#[derive(Debug, Clone)]
pub struct QuerySpec {
    pub field: String,
    pub op: QueryOp,
    pub value: QueryValue,
}
```

```rust
// crates/mux-query/src/list.rs
pub struct QueryList<T> {
    items: Vec<T>,
}

impl<T: Queryable> QueryList<T> {
    pub fn new(items: Vec<T>) -> Self { Self { items } }
    pub fn filter(&self, specs: &[QuerySpec]) -> Self { /* AND semantics */ todo!() }
    pub fn get(&self, specs: &[QuerySpec]) -> Result<&T, QueryError> { /* exactly one */ todo!() }
    pub fn first(&self) -> Option<&T> { self.items.first() }
    pub fn len(&self) -> usize { self.items.len() }
    pub fn is_empty(&self) -> bool { self.items.is_empty() }
}
```

### ORM Wrapper (mux-orm)

```rust
// crates/mux-orm/src/server.rs
pub struct OrmServer {
    api: ManagedMux,
}

impl OrmServer {
    pub fn sessions(&self) -> QueryList<OrmSession> {
        let snapshot = self.api.snapshot();
        QueryList::new(
            snapshot.sessions.iter()
                .map(|s| OrmSession::from_snapshot(s, &self.api))
                .collect()
        )
    }

    pub fn cmd(&self, cmd: &str) -> Result<String, OrmError> {
        self.api.cmd(cmd)
    }
}

// Usage:
// let server = OrmServer::connect_tmux(socket_path)?;
// let session = server.sessions().filter(&[q("name").exact("work")]).get()?;
// session.windows().first().unwrap().panes().first().unwrap().send_keys("cargo test\n")?;
```

### ORM Query Test Strategy

1. **Filter exact:** Create 3 sessions, filter by name, verify 1 result.
2. **Filter contains:** `name__contains="wo"` matches "work" and "workflow".
3. **Filter chain:** Multiple specs are AND-combined.
4. **Empty result:** Filter returns empty QueryList, `.get()` returns `Err`.
5. **Property test:** Filter is idempotent: `filter(f).filter(f) == filter(f)`.
6. **Binding roundtrip:** Python `server.sessions.filter(name="work")` resolves correctly.

---

## 13. Runtime Architecture

### Thread Model

```
+--------------------+     +-------------------+
|  Accept Thread     |     |  PTY Reader Pool  |
|  (tokio task)      |     |  (1 task/pane)    |
|  UnixListener      |     |  reads PTY fd     |
+--------+-----------+     +--------+----------+
         |                          |
         | new client conn          | PaneOutput events
         v                          v
+---------------------------------------------------+
|              State Actor (single task)              |
|                                                     |
|  loop {                                             |
|      event = recv(event_rx);                        |
|      outcome = apply_event(&mut graph, event, &ctx);|
|      for effect in outcome.effects {                |
|          dispatch_effect(effect, &tx_pool);          |
|      }                                              |
|      publish_snapshot(&graph, &state_store);         |
|  }                                                  |
+---------------------------------------------------+
```

### State Actor

```rust
// crates/mux-server/src/state.rs
pub struct StateActor {
    graph: ServerGraph,
    state_store: StateStore<GraphState>,
    event_rx: mpsc::Receiver<Event>,
    effect_tx: mpsc::Sender<Effect>,
}

impl StateActor {
    pub async fn run(mut self) {
        while let Some(event) = self.event_rx.recv().await {
            let ctx = CoreCtx {
                now_ms: now_millis(),
                rand_u64: rand::random(),
            };
            let outcome = apply_event(&mut self.graph, event, &ctx);
            for effect in outcome.effects {
                let _ = self.effect_tx.send(effect).await;
            }
            self.state_store.publish(self.graph.graph_state());
        }
    }
}
```

### Effect Dispatcher

```rust
// crates/mux-server/src/dispatch.rs
pub async fn dispatch_effect(
    effect: Effect,
    pty_pool: &PtyPool,
    client_pool: &ClientPool,
) {
    match effect {
        Effect::SpawnPane { pane_id, command, cwd, size } => {
            pty_pool.spawn(pane_id, &command, cwd.as_deref(), size).await;
        }
        Effect::WritePane { pane_id, data } => {
            pty_pool.write(pane_id, &data).await;
        }
        Effect::ResizePty { pane_id, size } => {
            pty_pool.resize(pane_id, size).await;
        }
        Effect::SendToClient { client_id, frame } => {
            client_pool.send(client_id, frame).await;
        }
        Effect::DisconnectClient { client_id } => {
            client_pool.disconnect(client_id).await;
        }
        Effect::Shutdown => {
            // Initiate graceful shutdown
        }
        _ => {}
    }
}
```

### Runtime Test Strategy

1. **State actor determinism:** Send same events to two actors with same `CoreCtx`, verify identical snapshots.
2. **Effect dispatch:** Send `Effect::SpawnPane`, verify PTY pool called.
3. **Snapshot publication:** After event processing, verify `StateHandle` reflects new state.
4. **Graceful shutdown:** Send `Effect::Shutdown`, verify all clients notified.
5. **Backpressure:** Fill effect channel, verify state actor does not block on snapshot publish.

---

## 14. Server Lifecycle

### 14.1 Lock File: flock (Not PID-based)

**Critical correction from v5:** tmux uses `flock(LOCK_EX|LOCK_NB)`, not PID-based locking. Reference: `client.c:77-101`:

```c
// client.c:78-101
static int
client_get_lock(char *lockfile)
{
    int lockfd;
    if ((lockfd = open(lockfile, O_WRONLY|O_CREAT, 0600)) == -1)
        return (-1);
    if (flock(lockfd, LOCK_EX|LOCK_NB) == -1) {
        if (errno != EAGAIN)
            return (lockfd);
        while (flock(lockfd, LOCK_EX) == -1 && errno == EINTR)
            /* nothing */;
        close(lockfd);
        return (-2);
    }
    return (lockfd);
}
```

**Advantages of flock over PID:**
- Automatically released on process death (kernel cleanup).
- No PID reuse risk.
- No TOCTOU race between check and remove.

```rust
// crates/mux-os/src/lock.rs

pub struct LockFile {
    _file: std::fs::File,  // held open to keep flock
    path: PathBuf,
}

pub fn acquire_lock_file(socket_path: &Path) -> Result<LockFile, ServerError> {
    let lock_path = socket_path.with_extension("lock");

    let file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(|e| ServerError::Startup {
            reason: format!("cannot open lock file {lock_path:?}: {e}"),
        })?;

    // SAFETY: flock is safe on a valid file descriptor.
    // We hold the File open for the lifetime of LockFile.
    let fd = file.as_raw_fd();
    let ret = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };
    if ret != 0 {
        let err = std::io::Error::last_os_error();
        if err.kind() == std::io::ErrorKind::WouldBlock {
            return Err(ServerError::Startup {
                reason: format!("another server holds the lock on {lock_path:?}"),
            });
        }
        return Err(ServerError::Startup {
            reason: format!("flock failed on {lock_path:?}: {err}"),
        });
    }

    // Write PID for diagnostics only (not for locking)
    use std::io::Write;
    let mut f = &file;
    let _ = f.write_all(format!("{}\n", std::process::id()).as_bytes());

    Ok(LockFile { _file: file, path: lock_path })
}

impl Drop for LockFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}
```

### 14.2 Startup Sequence (13 Steps)

1. Parse CLI arguments.
2. Resolve socket path.
3. Acquire lock file (flock).
4. Create Unix domain socket with restricted permissions.
5. Daemonize (if not foreground mode).
6. Initialize tracing/telemetry.
7. Initialize event loop (tokio runtime).
8. Spawn socket acceptor task.
9. **Wait for first client connection.**
10. **Complete first client identify burst.**
11. **Load configuration files** (matches `server-client.c:3725-3734`).
12. Report config errors to first client.
13. Enter main event loop.

Steps 9-12 are ordered to match tmux exactly: config is not loaded until the first client identifies. This ensures config errors are reported to the attaching client.

### 14.3 Shutdown Sequence

1. Server receives shutdown signal (SIGTERM or `kill-server` command).
2. Send exit notification to all connected clients.
3. Send exit notification to all control-mode clients.
4. Close all PTYs (kill child processes).
5. Close all client connections.
6. Drop `LockFile` (releases flock, removes lock file).
7. Remove socket file.
8. Exit process.

### 14.4 Server Lifecycle Test Strategy

1. **Lock contention:** Start server A, then B on same socket. B fails with "another server holds the lock".
2. **Crash recovery:** Start, `kill -9`, start again. Succeeds (flock released by kernel).
3. **Config timing:** Start server, connect client, verify config loaded after identify completes.
4. **Shutdown broadcast:** 3 clients connected, shutdown, all receive exit notification.
5. **Signal handling:** SIGTERM triggers graceful shutdown sequence.
6. **Identify burst:** Fixture tests for types 100-112, verify ordering parity with `client_send_identify` at `client.c:450-495`.

---

## 15. Control Mode (cross-ref: Section 18 Security for rate limiting, Section 30 Appendix for ControlNotification)

### 15.1 Current State

**Verified:** `~/work/rust/vibe-tmux/crates/mux-client/src/control.rs:30-36` defines a generic struct:

```rust
pub struct ControlNotification {
    pub name: String,  // notification name without '%'
    pub raw: String,   // raw line
}
```

This must be incrementally migrated to a typed enum.

### 15.2 Typed Notification Enum

```rust
// crates/mux-control/src/notification.rs

/// Typed notification variants.
/// Reference: control-notify.c for all notification formats.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlNotification {
    SessionsChanged,
    SessionChanged { session_id: u32, name: String },
    SessionRenamed { session_id: u32, name: String },
    SessionWindowChanged { session_id: u32, window_id: u32 },
    WindowAdd { window_id: u32 },
    WindowClose { window_id: u32 },
    WindowRenamed { window_id: u32, name: String },
    WindowPaneChanged { window_id: u32, pane_id: u32 },
    Output { pane_id: u32, data: Vec<u8> },
    /// Extended output with age timestamp.
    /// Reference: control.c:620-623
    ExtendedOutput { pane_id: u32, age: u64, data: Vec<u8> },
    LayoutChange { window_id: u32, layout: String },
    PaneModeChanged { pane_id: u32 },
    ClientSessionChanged { client: String, session_id: u32 },
    ClientDetached { client: String },
    Pause { pane_id: u32 },
    Continue { pane_id: u32 },
    Exit { reason: Option<String> },
}
```

### 15.3 Migration Parser

```rust
pub fn parse_control_line(line: &str) -> ControlEvent {
    match parse_notification(line) {
        Ok(typed) => ControlEvent::Typed(typed),
        Err(ControlParseError::UnknownNotification(tag)) => {
            ControlEvent::Generic { tag, raw: line.to_string() }
        }
        Err(ControlParseError::NotANotification) => {
            ControlEvent::OutputLine(line.to_string())
        }
        Err(e) => ControlEvent::ParseError(e),
    }
}

pub enum ControlEvent {
    Typed(ControlNotification),
    Generic { tag: String, raw: String },
    OutputLine(String),
    ParseError(ControlParseError),
}
```

### 15.4 `%begin/%end/%error` Block Framing

Control mode wraps command output in guard blocks:

```
%begin <time> <number> <flags>
<output lines>
%end <time> <number> <flags>
```

Or on error:

```
%begin <time> <number> <flags>
<error message>
%error <time> <number> <flags>
```

### 15.5 Rate Limiting

Reference: Control backpressure at `control.c:450-461` (output-side only). TermForge adds input-side rate limiting:

```rust
pub struct ControlRateLimiter {
    /// Max commands per second per control-mode client.
    pub max_commands_per_sec: u32,
    /// Max pending output bytes before backpressure.
    pub max_pending_bytes: usize,
}

impl Default for ControlRateLimiter {
    fn default() -> Self {
        Self {
            max_commands_per_sec: 1000,
            max_pending_bytes: 16 * 1024 * 1024, // 16 MB
        }
    }
}
```

### 15.6 Hints vs Authority

Control notifications are hints, never authoritative. The binary protocol is the source of truth. Dropped notifications are corrected by periodic refresh cycles. The `mux-refresh` crate periodically syncs full state from the server to correct any drift.

### 15.7 Control Mode Test Strategy

1. **Parser roundtrip:** Construct line per notification type, parse, verify variant.
2. **Unknown notification:** `%foo-bar` produces `ControlEvent::Generic`.
3. **Guard protocol:** `%begin/%end/%error` block framing.
4. **Malformed lines:** Truncated, empty, binary data -> `ControlParseError`.
5. **Parity test:** Connect via `tmux -C`, trigger each notification, feed to parser.
6. **Hints-vs-authority:** Dropped notification corrected by periodic refresh.
7. **Migration test:** `ControlEvent::Generic` fallback works for un-typed notifications.
8. **Extended output:** Parse `%extended-output %5 12345 : hello`, verify fields.

---

## 16. Language Bindings

### 16.1 Thin Wrapper Philosophy

Bindings expose:
1. Object graph traversal: `Server.sessions`, `Session.windows`, `Window.panes`
2. Command execution: `server.cmd("new-session -d -s work")`
3. Query API: `server.sessions.filter(name__startswith="wo")`
4. Lifecycle management: `ManagedMux`, refresh subscriptions

Bindings depend on `mux-orm` (which depends on `mux-api`), never runtime internals.

### 16.2 Pure Logic Separation Pattern

Verified in `~/study/rust/learning-rust-nodejs/native/src/`: the project separates pure Rust logic (`logic.rs`) from Neon binding glue (`lib.rs`). TermForge adopts this pattern for all bindings:

```
bindings/node/native/src/
    lib.rs      # Neon glue: FunctionContext, JsResult, Channel
    logic.rs    # Pure Rust: no Neon types, testable without Node.js/V8
```

This ensures binding logic is testable with standard `#[test]` without requiring a language runtime.

### 16.3 Error Mapping (cross-ref: Section 8 Error Handling)

Map `ErrorClass` to native exception types:

| ErrorClass | Python | Node.js | C++ |
|---|---|---|---|
| Transient | `TimeoutError` | `Error` with `ETIMEOUT` | `std::runtime_error` |
| ProtocolViolation | `ConnectionError` | `Error` with `EPROTO` | `std::runtime_error` |
| UserError | `ValueError` / `KeyError` | `Error` with `EINVAL` | `std::invalid_argument` |
| Bug | `RuntimeError` | `Error` with `EINTERNAL` | `std::logic_error` |

### 16.4 Python (PyO3 + maturin)

Verified against actual PyO3 0.26 API at `~/study/rust-python/pyo3/`:
- `Bound<'py, T>` smart pointer (`src/instance.rs`): the primary way to hold Python objects with a lifetime tied to the GIL.
- `#[pyfunction]` and `#[pymodule]` macros for defining Python-callable functions and modules.
- `wrap_pyfunction!` macro for registering functions in modules.
- **`Python::detach`** (formerly `allow_threads` before PyO3 0.26, verified at `guide/src/migration.md:300-312`): releases the GIL so other Python threads can run while Rust code executes blocking operations.

```rust
// bindings/python/src/lib.rs

use pyo3::prelude::*;

#[pyclass(name = "Server")]
struct PyServer {
    inner: OrmServer,
}

#[pymethods]
impl PyServer {
    #[new]
    fn new(socket_name: Option<&str>, socket_path: Option<&str>) -> PyResult<Self> {
        todo!()
    }

    /// Execute a tmux command.
    /// Releases the GIL during blocking IO via Python::detach (PyO3 0.26+).
    /// Previously known as `allow_threads` (renamed in PyO3 0.26).
    fn cmd(&self, py: Python<'_>, cmd: &str) -> PyResult<String> {
        let inner = self.inner.clone();
        // Python::detach releases the GIL so other Python threads can proceed
        // while the Rust code performs blocking IO on the Unix socket.
        py.detach(move || {
            inner.cmd(cmd).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string())
            })
        })
    }

    #[getter]
    fn sessions(&self) -> PyResult<Vec<PySession>> { todo!() }
}

#[pymodule]
fn termforge(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyServer>()?;
    m.add_class::<PySession>()?;
    m.add_class::<PyWindow>()?;
    m.add_class::<PyPane>()?;
    Ok(())
}
```

### 16.5 Python Async Phases

**Phase 1 (sync):** All blocking methods release GIL via `py.detach()`. This matches libtmux's sync model (`src/libtmux/options.py:701-787`).

**Phase 2 (Python-side async):** `asyncio.to_thread()` wrappers in Python, zero Rust changes.

```python
# Phase 2: Zero Rust changes
async def async_cmd(server: Server, cmd: str) -> str:
    return await asyncio.to_thread(server.cmd, cmd)
```

**Phase 3 (native streaming):** `pyo3-async-runtimes` only for event subscription, bridging `broadcast::Receiver` to `asyncio.Queue`.

```rust
// Phase 3: Native streaming only
#[pymethod]
fn subscribe(&self, py: Python<'_>) -> PyResult<PyObject> {
    let asyncio = py.import("asyncio")?;
    let queue = asyncio.call_method0("Queue")?;
    // Rust thread bridges broadcast::Receiver -> asyncio.Queue
    Ok(queue.to_object(py))
}
```

### 16.6 Node (Neon)

Verified against actual Neon API at `~/study/rust-node/neon/crates/neon/src/`:
- `FunctionContext` (`context/mod.rs`): provides access to JS arguments, return values, and memory management with lifetimes.
- `JsResult<JsString>` for fallible operations returning JS types.
- `Channel` (`event/channel.rs`): schedules Rust closures onto the JS main thread from background threads. Uses `ThreadsafeFunction` internally.
- `#[neon::main]` attribute macro for the module entry point (replacing the older `register_module!` macro).

```rust
// bindings/node/native/src/lib.rs

use neon::prelude::*;

fn js_server_cmd(mut cx: FunctionContext) -> JsResult<JsString> {
    let cmd = cx.argument::<JsString>(0)?.value(&mut cx);
    // Delegate to pure logic (no Neon types in logic.rs)
    let result = logic::execute_command(&cmd)
        .or_else(|e| cx.throw_error(e.to_string()))?;
    Ok(cx.string(result))
}

/// Async command using Neon Channel for background work.
/// The Channel schedules a callback on the JS main thread when the Rust
/// background thread completes, avoiding blocking the Node.js event loop.
fn js_server_cmd_async(mut cx: FunctionContext) -> JsResult<JsPromise> {
    let cmd = cx.argument::<JsString>(0)?.value(&mut cx);
    let channel = cx.channel();
    let (deferred, promise) = cx.promise();

    std::thread::spawn(move || {
        let result = logic::execute_command(&cmd);
        deferred.settle_with(&channel, move |mut cx| {
            match result {
                Ok(output) => Ok(cx.string(output)),
                Err(e) => cx.throw_error(e.to_string()),
            }
        });
    });

    Ok(promise)
}

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    cx.export_function("serverCmd", js_server_cmd)?;
    cx.export_function("serverCmdAsync", js_server_cmd_async)?;
    Ok(())
}
```

```rust
// bindings/node/native/src/logic.rs
// Pure Rust logic -- no Neon types. Testable with standard #[test].

pub fn execute_command(cmd: &str) -> Result<String, Box<dyn std::error::Error>> {
    // ... delegate to mux-orm
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_command_validation() {
        // Can test without Node.js runtime
        assert!(execute_command("").is_err());
    }
}
```

### 16.7 C++ (cxx)

Uses the `cxx` bridge crate. Exposes the same ORM API through C++ shared pointers.

### 16.8 Migration from libtmux

Expose property names matching libtmux conventions:

```python
server = termforge.Server()
server.sessions      # property, returns list
server.cmd("ls")     # method, returns string
```

### 16.9 Bindings Test Strategy

1. **Python:** pytest with hermetic server per test.
2. **Node:** vitest with same pattern.
3. **GIL test:** Blocking call in one thread, concurrent Python thread confirms GIL released (via `Python::detach`).
4. **Error mapping:** Trigger each error class, verify correct exception type.
5. **Memory leak:** `tracemalloc` for 1000 create/destroy cycles.
6. **Concurrency:** 10 threads calling `server.sessions` -- no deadlocks.
7. **asyncio.to_thread:** Verify `await asyncio.to_thread(server.cmd, "list-sessions")` works.
8. **Pure logic test:** `logic.rs` tests run without Node.js runtime.

---

## 17. CRDT Transaction Layer (cross-ref: Section 30 Appendix for HlcTimestamp/NodeId canonical types)

### 17.1 Scope (settled)

CRDT is applied to a **subset** of state first:

**Replicable first:**
1. Paste buffers
2. Options/config key-value maps
3. Session/window/pane metadata and layout intents

**Not replicated first:**
1. Raw PTY stream output
2. Per-client ephemeral UI state (cursor shape, focus, render timing)

### 17.2 HLC Timestamps

All three v5 models agreed on Hybrid Logical Clocks. Fixed-size `(u64 millis, u32 counter, NodeId)` = 20 bytes, providing causal ordering sufficient for LWW-Register and OR-Set CRDTs.

```rust
// crates/mux-crdt/src/clock.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
         serde::Serialize, serde::Deserialize)]
pub struct HlcTimestamp {
    pub millis: u64,
    pub counter: u32,
    pub node_id: NodeId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
         serde::Serialize, serde::Deserialize)]
pub struct NodeId(pub u64);

pub struct HybridClock {
    pub last: HlcTimestamp,
    pub node_id: NodeId,
}

impl HybridClock {
    pub fn now(&mut self, wall_ms: u64) -> HlcTimestamp {
        let millis = wall_ms.max(self.last.millis);
        let counter = if millis == self.last.millis {
            self.last.counter + 1
        } else {
            0
        };
        self.last = HlcTimestamp { millis, counter, node_id: self.node_id };
        self.last
    }

    pub fn receive(&mut self, remote: HlcTimestamp, wall_ms: u64) -> HlcTimestamp {
        let millis = wall_ms.max(self.last.millis).max(remote.millis);
        let counter = if millis == self.last.millis && millis == remote.millis {
            match self.last.counter.max(remote.counter).checked_add(1) {
                Some(c) => c,
                None => {
                    return self.force_advance(millis + 1);
                }
            }
        } else if millis == self.last.millis {
            self.last.counter.saturating_add(1)
        } else if millis == remote.millis {
            remote.counter.saturating_add(1)
        } else {
            0
        };
        self.last = HlcTimestamp { millis, counter, node_id: self.node_id };
        self.last
    }

    fn force_advance(&mut self, millis: u64) -> HlcTimestamp {
        self.last = HlcTimestamp { millis, counter: 0, node_id: self.node_id };
        self.last
    }
}
```

### 17.3 Dotted Version Vectors

```rust
// crates/mux-crdt/src/dvv.rs
pub struct Dvv {
    pub summary: SmallVec<[(NodeId, u64); 8]>,
    pub dot: HlcTimestamp,
}

pub fn compact_dvv(dvv: &mut Dvv, active_nodes: &[NodeId]) {
    dvv.summary.retain(|(node, _)| active_nodes.contains(node));
}
```

### 17.4 CRDT Types

```rust
// crates/mux-crdt/src/register.rs
pub struct LwwRegister<T> {
    pub value: T,
    pub timestamp: HlcTimestamp,
}

impl<T> LwwRegister<T> {
    pub fn merge(&mut self, other: LwwRegister<T>) {
        if other.timestamp > self.timestamp {
            self.value = other.value;
            self.timestamp = other.timestamp;
        }
    }
}
```

```rust
// crates/mux-crdt/src/set.rs
pub struct OrSet<T: Eq + Hash> {
    pub entries: HashMap<T, SmallVec<[HlcTimestamp; 2]>>,
}

impl<T: Eq + Hash + Clone> OrSet<T> {
    pub fn add(&mut self, value: T, ts: HlcTimestamp) {
        self.entries.entry(value).or_default().push(ts);
    }

    pub fn remove(&mut self, value: &T, observed: &[HlcTimestamp]) {
        if let Some(dots) = self.entries.get_mut(value) {
            dots.retain(|ts| !observed.contains(ts));
            if dots.is_empty() {
                self.entries.remove(value);
            }
        }
    }

    pub fn contains(&self, value: &T) -> bool {
        self.entries.contains_key(value)
    }
}
```

### 17.5 Operation Log

```rust
// crates/mux-crdt/src/oplog.rs
pub enum CrdtOp {
    SetOption { scope: Scope, key: String, value: String, ts: HlcTimestamp },
    BufferPut { buffer: BufferId, bytes: Vec<u8>, ts: HlcTimestamp },
    EntityCreate { kind: EntityKind, id: u64, ts: HlcTimestamp },
    EntityDelete { kind: EntityKind, id: u64, ts: HlcTimestamp },
}

pub struct OpLog {
    pub ops: Vec<CrdtOp>,
    pub version: Dvv,
}

impl OpLog {
    pub fn compact(&mut self, active_nodes: &[NodeId]) {
        compact_dvv(&mut self.version, active_nodes);
    }
}
```

### 17.6 Merge Strategy

Last-Writer-Wins by default, with kill-wins-over-write for topology conflicts.

### 17.7 CRDT Test Strategy

1. **HLC monotonicity:** 10K calls to `now()` -> strictly increasing.
2. **HLC convergence:** Two clocks, 1000 interleaved ops, causally consistent.
3. **LWW determinism:** Same timestamp, different `node_id`s -> higher `node_id` wins.
4. **OR-Set commutativity:** `merge(a, b) == merge(b, a)`.
5. **OR-Set add-remove-add:** Re-add with higher timestamp restores element.
6. **OpLog idempotency:** Merging same ops twice -> identical state.
7. **Counter overflow:** Simulate and verify forced millis advance.
8. **DVV compaction:** 10 nodes, deactivate 5, compact -> 5 entries.

---

## 18. Security Model

### 18.1 Input Validation Layers (cross-ref: Section 8 Error Handling)

```
Layer  Input Source         Validator            Error Class          Action
-----  ------------         ---------            -----------          ------
  0    Socket bytes         ImsgCodec            Transient            wait
                                                 ProtocolViolation    kill conn
  1    ImsgFrame            PayloadParser        ProtocolViolation    kill conn
  2    TypedPayload         IdentifyCollector    ProtocolViolation    kill conn
  3    Command args         CommandDispatch      UserError            error reply
  4    Config file text     mux-conf lexer       UserError            error event
  5    Control mode lines   ControlParser        UserError            %error reply
  6    Binding FFI args     mux-orm validators   UserError            exception
  7    CRDT ops (sync)      CrdtValidator        ProtocolViolation    drop peer
```

**Layer 0 -- Socket bytes:** `ImsgCodec` validates `len >= 16` and `len <= MAX_IMSGSIZE` (16384). Invalid lengths produce `ProtocolViolation` which kills the connection (matching `server-client.c:3472-3475`).

**Layer 1 -- ImsgFrame:** `PayloadParser` validates `msg_type` against `MsgType` enum.

**Layer 2 -- TypedPayload:** `IdentifyCollector` validates identify burst (types 100-112), rejects duplicates (`server-client.c:3599-3600`).

**Layer 3 -- Command args:** `CommandDispatch` validates command names and argument counts.

**Layer 4 -- Config file text:** `mux-conf` lexer validates syntax.

**Layer 5 -- Control mode lines:** `ControlParser` validates notification format.

**Layer 6 -- Binding FFI args:** `mux-orm` validators check Python/Node/C++ arguments at Rust boundary.

**Layer 7 -- CRDT ops:** `CrdtValidator` checks HLC timestamps and DVV consistency.

### 18.2 Socket Security

Reference: `server.c:126-129`:

```rust
// crates/mux-os/src/socket.rs
pub fn create_socket(path: &Path, is_default: bool) -> Result<UnixListener, ServerError> {
    let mask = if is_default {
        libc::S_IXUSR | libc::S_IXGRP | libc::S_IRWXO
    } else {
        libc::S_IXUSR | libc::S_IRWXG | libc::S_IRWXO
    };
    // SAFETY: umask is thread-safe on single-threaded startup
    let old_mask = unsafe { libc::umask(mask as libc::mode_t) };
    let listener = UnixListener::bind(path);
    unsafe { libc::umask(old_mask) };
    listener.map_err(|e| ServerError::Startup {
        reason: format!("cannot bind socket {path:?}: {e}"),
    })
}
```

#### Socket Directory Validation

```rust
pub fn validate_socket_dir(dir: &Path) -> Result<(), ServerError> {
    let meta = std::fs::symlink_metadata(dir)?;
    if meta.file_type().is_symlink() {
        return Err(ServerError::Startup {
            reason: format!("socket directory {dir:?} is a symlink"),
        });
    }
    let mode = meta.permissions().mode();
    if mode & 0o002 != 0 && mode & 0o1000 == 0 {
        return Err(ServerError::Startup {
            reason: format!("socket directory {dir:?} is world-writable without sticky bit"),
        });
    }
    if meta.uid() != unsafe { libc::getuid() } {
        return Err(ServerError::Startup {
            reason: format!("socket directory {dir:?} owned by uid {}, expected {}",
                meta.uid(), unsafe { libc::getuid() }),
        });
    }
    Ok(())
}
```

### 18.3 SCM_RIGHTS and File Descriptor Security

FDs via SCM_RIGHTS accepted only during identify handshake. After `CLIENT_IDENTIFIED`, ancillary FDs closed immediately. CLOEXEC set on all received FDs. Verified: `~/work/rust/vibe-tmux/crates/mux-os/src/scm_rights.rs:141-149`.

```rust
// crates/mux-os/src/fd.rs
pub fn set_cloexec(fd: RawFd) -> Result<(), io::Error> {
    // SAFETY: fcntl with F_GETFD/F_SETFD is safe on valid FDs
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
    if flags < 0 { return Err(io::Error::last_os_error()); }
    let ret = unsafe { libc::fcntl(fd, libc::F_SETFD, flags | libc::FD_CLOEXEC) };
    if ret < 0 { return Err(io::Error::last_os_error()); }
    Ok(())
}
```

### 18.4 Resource Limits

```rust
pub struct SecurityLimits {
    pub max_clients: usize,               // 256
    pub max_control_pending: usize,        // 16 MB
    pub max_imsg_payload: usize,           // 16368
    pub max_control_commands_per_sec: u32, // 1000
    pub max_sessions: usize,               // 1000
    pub max_windows_per_session: usize,    // 1000
    pub max_scrollback_lines: u32,         // 50_000
}
```

### 18.5 Security Invariants Summary

1. Socket permissions match `server.c:126-129`.
2. No symlink following in socket directory.
3. Directory ownership check.
4. World-writable directory rejection (without sticky bit).
5. Max 256 clients (configurable).
6. Control mode 16 MB output limit.
7. SCM_RIGHTS phase restriction.
8. CLOEXEC on all received FDs.
9. Close unexpected extra FDs.
10. flock locking (no PID trust).
11. Config after identify.

### 18.6 Security Test Strategy

1. **Socket permissions:** Default socket 0660, named socket 0600.
2. **Directory symlink/ownership/world-writable:** Verify rejection.
3. **SCM_RIGHTS valid/non-TTY/out-of-phase/multiple FDs.**
4. **CLOEXEC verification.**
5. **Max clients/control backpressure/resource exhaustion.**
6. **Fuzz:** Random bytes to all parsers.
7. **Identify replay:** Second burst rejected.

---

## 19. OpenTelemetry

### 19.1 Architecture Overview

- **Layer 0:** `tracing` crate only. No `opentelemetry` dependency.
- **Layer 1 (`mux-telemetry`):** Bridges `tracing` to OTEL exporters.

```
mux-core (tracing spans)  -->  mux-telemetry (OTEL SDK)  -->  OTLP/gRPC
                                                          -->  Stdout
                                                          -->  No-op
```

### 19.2 Verified OTEL SDK Types

Verified against `~/study/otel/opentelemetry-rust/`:

**API layer** (`opentelemetry/src/trace/`):
- `TracerProvider` trait: factory for `Tracer` instances.
- `Tracer` trait: `start()`, `start_with_context()`, `in_span()`.
- `Span` trait: `add_event()`, `record_error()`, `is_recording()`, `set_status()`.

**SDK layer** (`opentelemetry-sdk/src/trace/`):
- `SdkTracerProvider` (`provider.rs`): holds processors, config, `is_shutdown: AtomicBool`. Default sampler: `ParentBased(AlwaysOn)`.
- SDK `Span` (`span.rs`): wraps `SpanData` with `parent_span_id`, `span_kind`, `name`, `start_time`, `end_time`, `attributes: Vec<KeyValue>`, `events`, `links`, `status`.
- `SpanExporter` trait (`export.rs`): `export(batch: Vec<SpanData>)` async, `shutdown()`, `force_flush()`.
- `BatchSpanProcessor` (`span_processor.rs`): `OTEL_BSP_SCHEDULE_DELAY_DEFAULT = 5000ms`, `OTEL_BSP_MAX_QUEUE_SIZE_DEFAULT = 2048`.
- `SamplingDecision` enum (`sampler.rs`): `Drop`, `RecordOnly`, `RecordAndSample`.

### 19.3 Span Naming Convention

Pattern: `termforge.<subsystem>.<operation>`.

```rust
pub mod span_names {
    pub const SERVER_ACCEPT: &str = "termforge.server.accept";
    pub const SERVER_STARTUP: &str = "termforge.server.startup";
    pub const ENGINE_APPLY: &str = "termforge.core.apply_event";
    pub const PTY_SPAWN: &str = "termforge.pty.spawn";
    pub const PTY_READ: &str = "termforge.pty.read";
    pub const CODEC_DECODE: &str = "termforge.proto.decode";
    pub const LAYOUT_RESIZE: &str = "termforge.layout.resize";
    pub const CONTROL_PARSE: &str = "termforge.control.parse";
    pub const CRDT_MERGE: &str = "termforge.crdt.merge";
    pub const ORM_QUERY: &str = "termforge.orm.query";
    pub const BINDINGS_PYTHON: &str = "termforge.bindings.python";
    pub const BINDINGS_NODE: &str = "termforge.bindings.node";
    // ... full list in crate
}
```

### 19.4 Sampling Strategy

Maps to SDK's `SamplingDecision`:

```rust
pub enum SamplingStrategy {
    AlwaysOn,                          // SamplingDecision::RecordAndSample
    AlwaysOff,                         // SamplingDecision::Drop
    RatioBased { ratio: f64 },         // TraceIdRatioBased
    ParentBased { root_ratio: f64 },   // ParentBased(TraceIdRatioBased)
}

impl Default for SamplingStrategy {
    fn default() -> Self {
        Self::ParentBased { root_ratio: 0.01 }
    }
}
```

### 19.5 Telemetry Initialization

```rust
pub struct TelemetryConfig {
    pub exporter: Exporter,
    pub sampling: SamplingStrategy,
    pub filter: ExportFilterConfig,
    pub service_name: String,
    pub batch_queue_size: usize,           // default 2048
    pub batch_schedule_delay_ms: u64,      // default 5000
}

pub struct TelemetryGuard {
    _provider: Option<opentelemetry_sdk::trace::SdkTracerProvider>,
}

impl Drop for TelemetryGuard {
    fn drop(&mut self) {
        if let Some(provider) = self._provider.take() {
            let _ = provider.shutdown();
        }
    }
}
```

### 19.6 Context Propagation

- **Cross-task:** `tracing::info_span!` + `span.enter()` in spawned tasks.
- **Cross-process (bindings):** W3C Trace Context headers via Unix socket. Python uses `Python::detach`.
- **CRDT sync:** `SyncMessage.trace_parent: Option<String>`.

### 19.7 Pure Boundary Enforcement

`mux-core` depends only on `tracing`. Enforced by:
1. `Cargo.toml` dependency check.
2. CI: `cargo tree -p mux-core | grep opentelemetry` returns empty.
3. WASM gate catches accidental imports.

### 19.8 Telemetry Test Strategy

1. **Stdout export:** Verify `termforge.` spans in JSON output.
2. **Span parent-child:** Correct hierarchy.
3. **Pure boundary:** No opentelemetry in mux-core dep tree.
4. **Context propagation (Python/Node):** Verify cross-process trace linking.
5. **Filter exclusion:** h2/tonic spans excluded.
6. **Sampling:** AlwaysOff exports nothing, AlwaysOn exports all.
7. **Guard flush:** Drop guard -> pending spans flushed.
8. **Batch config:** Default queue 2048, delay 5000ms.

---

## 20. tmux Version Management

### 20.1 tmux-vm: Version Manager CLI

```
USAGE:
    tmux-vm <COMMAND>

COMMANDS:
    ensure <VERSION>       Ensure a tmux version is built and cached
    list                   List installed versions
    path <VERSION>         Print path to binary
    remove <VERSION>       Remove a cached version
    clean                  Remove all cached versions
    versions               List known versions (git tags)
```

#### Concrete CLI Workflows

```bash
$ tmux-vm ensure 3.6a
Checking cache... not found
Fetching tmux source at tag 3.6a...
Building tmux 3.6a (configure + make)...
Installed to ~/.local/share/tmux-vm/versions/3.6a/bin/tmux

$ tmux-vm path 3.6a
/home/user/.local/share/tmux-vm/versions/3.6a/bin/tmux

$ tmux-vm list
3.5a   ~/.local/share/tmux-vm/versions/3.5a/bin/tmux
3.6a   ~/.local/share/tmux-vm/versions/3.6a/bin/tmux
```

#### Implementation

```rust
// tools/tmux-vm/src/main.rs
use clap::Parser;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    Ensure { version: String },
    Exec { version: String, #[arg(last = true)] args: Vec<String> },
    List,
    Path { version: String },
    Remove { version: String },
    Clean,
    Versions,
}

/// Cache: ~/.local/share/tmux-vm/versions/<ver>/bin/tmux
fn cache_dir() -> PathBuf {
    dirs::data_local_dir().unwrap_or_else(|| PathBuf::from(".")).join("tmux-vm")
}
```

### 20.2 tmux-builder: Source Builder

```rust
// tools/tmux-builder/src/lib.rs
pub struct BuildConfig {
    pub git_ref: String,
    pub source_dir: PathBuf,
    pub install_prefix: PathBuf,
    pub configure_flags: Vec<String>,
    pub jobs: usize,
}

pub fn build_tmux(config: &BuildConfig) -> Result<PathBuf, BuildError> {
    // 1. git checkout <git_ref>
    // 2. sh autogen.sh (if needed)
    // 3. ./configure --prefix=<install_prefix>
    // 4. make -j<jobs>
    // 5. make install
    todo!()
}
```

### 20.3 tmux-worktrees

```bash
$ tmux-worktrees create 3.5a   # create worktree at tag 3.5a
$ tmux-worktrees list           # list worktrees
$ tmux-worktrees remove 3.5a
```

### 20.4 Parity Runner (mux-regress)

```rust
// tools/mux-regress/src/lib.rs
pub struct ParityScenario {
    pub name: String,
    pub commands: Vec<String>,
    pub expected_format_outputs: Vec<(String, String)>,
}

pub fn run_parity(
    scenario: &ParityScenario,
    tmux_binary: &Path,
    termforge_binary: &Path,
) -> Result<ParityResult, ParityError> {
    // 1. Start tmux server, run commands, collect output
    // 2. Start TermForge server, run same commands, collect output
    // 3. Diff
    todo!()
}
```

### 20.5 Multi-Version Test Matrix

Run parity against 3.5a, 3.6, 3.6a simultaneously.

### 20.6 Version Management Test Strategy

1. **tmux-vm ensure:** Build 3.6a, verify binary runs.
2. **tmux-vm path:** Correct cache location.
3. **tmux-vm list:** Both versions listed.
4. **tmux-vm remove:** Binary removed.
5. **Parity runner:** Zero differences on supported commands.
6. **Multi-version:** Tests across 3.5a, 3.6, 3.6a.
7. **Worktree isolation:** Independent builds.
8. **Cache persistence:** Version survives process restart.
9. **Build failure:** Non-existent version -> clear error.

---

## 21. Test Support and Fake PTY

### 21.1 TmuxTestServer

```rust
pub struct TmuxTestServer {
    socket_path: PathBuf,
    _guard: PathGuard,
    process: Option<Child>,
}

impl Drop for TmuxTestServer {
    fn drop(&mut self) {
        if let Some(mut p) = self.process.take() {
            let _ = p.kill(); let _ = p.wait();
        }
    }
}
```

### 21.2 PathGuard

```rust
pub struct PathGuard { path: PathBuf }

impl PathGuard {
    pub fn new() -> Self {
        let path = std::env::temp_dir().join(format!("termforge-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&path).expect("create test dir");
        Self { path }
    }
}

impl Drop for PathGuard {
    fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.path); }
}
```

### 21.3 Hermetic Isolation Rules

1. Unique socket path per test.
2. Clear `TMUX`, `TMUX_TMPDIR`, `TMUX_PANE`.
3. `FakePtyBackend` never allocates real PTYs.
4. `PathGuard::drop()` cleanup even on panic.
5. Safe for parallel `cargo test -j N`.

### 21.4 PtyBackend Trait

```rust
pub trait PtyBackend: Send {
    fn spawn(&mut self, pane_id: PaneId, command: &[String],
             cwd: Option<&str>, size: PaneSize) -> Result<PtyHandle, PtyError>;
    fn write(&mut self, handle: &PtyHandle, data: &[u8]) -> Result<usize, PtyError>;
    fn read(&mut self, handle: &PtyHandle, buf: &mut [u8]) -> Result<usize, PtyError>;
    fn resize(&mut self, handle: &PtyHandle, size: PaneSize) -> Result<(), PtyError>;
    fn kill(&mut self, handle: &PtyHandle, signal: i32) -> Result<(), PtyError>;
    fn try_wait(&mut self, handle: &PtyHandle) -> Result<Option<i32>, PtyError>;
}
```

### 21.5 ScenarioRecorder and ScenarioReplayer

Record real PTY sessions, replay through pure core for deterministic tests.

```rust
pub struct PtyScenario {
    pub metadata: ScenarioMetadata,
    pub events: Vec<ScenarioEvent>,
}

pub enum ScenarioEventKind {
    Input(Vec<u8>),
    Output(Vec<u8>),
    Resize { cols: u16, rows: u16 },
    Exit { status: i32 },
}
```

**Workflow:** Record -> save to `fixtures/scenarios/` -> replay with `insta` snapshots.

### 21.6 Test Support Test Strategy

1. **TmuxTestServer lifecycle.**
2. **PathGuard cleanup.**
3. **Hermetic isolation.**
4. **ScenarioRecorder fidelity.**
5. **ScenarioReplayer determinism.**

---

## 22. Binding Test Frameworks

### 22.1 Python pytest Plugin

```python
# bindings/python/python/termforge/pytest_plugin.py
import pytest, os
from termforge import Server

@pytest.fixture
def server(tmp_path):
    """Hermetic TermForge server. Scrubs TMUX env vars."""
    for var in ("TMUX", "TMUX_TMPDIR", "TMUX_PANE"):
        os.environ.pop(var, None)
    socket_path = str(tmp_path / "test.sock")
    srv = Server(socket_path=socket_path)
    yield srv
    try: srv.kill_server()
    except: pass

@pytest.fixture
def session(server):
    server.cmd("new-session -d -s test")
    return server.sessions[0]

@pytest.fixture
def window(session):
    return session.windows[0]

@pytest.fixture
def pane(window):
    return window.panes[0]

@pytest.fixture
def server_with_tmux(tmp_path):
    """Server backed by real tmux binary (for parity tests)."""
    import subprocess
    tmux_bin = subprocess.check_output(
        ["tmux-vm", "path", "3.6a"], text=True
    ).strip()
    socket_path = str(tmp_path / "test.sock")
    srv = Server(socket_path=socket_path, tmux_bin=tmux_bin)
    yield srv
    try: srv.kill_server()
    except: pass
```

#### PyO3 Rust-side Fixture Support

Uses `Bound<'py, T>` smart pointer and `Python::detach` (PyO3 0.26+):

```rust
// bindings/python/src/fixtures.rs
use pyo3::prelude::*;

#[pyfunction]
pub fn snapshot_pane_grid(py: Python<'_>, pane: &PyPane) -> PyResult<String> {
    let inner = pane.inner.clone();
    // Python::detach releases GIL during grid read
    py.detach(move || {
        inner.grid_text().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string())
        })
    })
}
```

### 22.2 Python Snapshot Testing

```python
def test_new_session_grid(server, tmp_path):
    server.cmd("new-session -d -s snap -x 80 -y 24")
    pane = server.sessions[0].windows[0].panes[0]
    pane.send_keys("echo 'Hello, TermForge!'", enter=True)
    import time; time.sleep(0.5)
    grid = pane.capture_pane()
    assert "Hello, TermForge!" in grid

def test_concurrent_sessions(server):
    for name in ("alpha", "beta", "gamma"):
        server.cmd(f"new-session -d -s {name}")
    assert len(server.sessions) == 3
    filtered = server.sessions.filter(name="beta")
    assert len(filtered) == 1

def test_gil_release(server):
    """Verify GIL released via Python::detach during blocking ops."""
    import threading
    results = []
    def background(): results.append("done")
    server.cmd("new-session -d -s test")
    t = threading.Thread(target=background)
    t.start()
    server.cmd("list-sessions")  # releases GIL via detach
    t.join(timeout=5.0)
    assert "done" in results
```

### 22.3 Node vitest Plugin

Uses Neon `Channel` for async and `FunctionContext` for arg handling:

```typescript
// bindings/node/__test__/setup.ts
import { mkdtempSync, rmSync } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';
import { Server } from 'termforge';

export function createTestServer() {
  const dir = mkdtempSync(join(tmpdir(), 'termforge-test-'));
  const socketPath = join(dir, 'test.sock');
  for (const key of ['TMUX', 'TMUX_TMPDIR', 'TMUX_PANE'])
    delete process.env[key];
  const server = new Server({ socketPath });
  return {
    server, socketPath,
    cleanup: () => {
      try { server.killServer(); } catch {}
      rmSync(dir, { recursive: true, force: true });
    },
  };
}
```

```typescript
// bindings/node/__test__/server.test.ts
import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { createTestServer } from './setup';

describe('Server', () => {
  let ctx;
  beforeEach(() => { ctx = createTestServer(); });
  afterEach(() => { ctx.cleanup(); });

  it('creates and filters sessions', () => {
    ctx.server.cmd('new-session -d -s work');
    ctx.server.cmd('new-session -d -s play');
    const filtered = ctx.server.sessions({ name: 'work' });
    expect(filtered).toHaveLength(1);
    expect(filtered[0].name).toBe('work');
  });

  it('async commands via Neon Channel', async () => {
    ctx.server.cmd('new-session -d -s async-test');
    const result = await ctx.server.cmdAsync('list-sessions');
    expect(result).toContain('async-test');
  });
});
```

#### Neon Rust-side Pure Logic Tests

```rust
// bindings/node/native/src/logic.rs
// Pattern from ~/study/rust/learning-rust-nodejs/native/src/logic.rs

pub fn validate_command(cmd: &str) -> Result<(), String> {
    if cmd.is_empty() { return Err("command cannot be empty".into()); }
    if cmd.contains('\0') { return Err("command cannot contain null bytes".into()); }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_validate_empty() { assert!(validate_command("").is_err()); }
    #[test]
    fn test_validate_valid() { assert!(validate_command("list-sessions").is_ok()); }
}
```

### 22.4 Binding Test Framework Test Strategy

1. **pytest fixture chain:** `server` -> `session` -> `window` -> `pane`.
2. **vitest fixture chain:** Same in TypeScript.
3. **Parallel safety:** 10 pytest workers, no collisions.
4. **Cleanup on failure:** Server killed even on exception.
5. **GIL release:** Concurrent thread confirms `Python::detach` works.
6. **Neon Channel async:** Promise resolves correctly.
7. **Pure logic tests:** `logic.rs` without Node.js.
8. **Snapshot testing (Python/Node):** Grid content assertions.
9. **Error propagation:** Invalid command -> correct exception type.
10. **Memory leak (Python):** `tracemalloc` 1000 cycles.
11. **Parity fixture:** `server_with_tmux` uses real tmux.

---

## 23. Test Framework and Harness Design

### 23.1 Test Taxonomy

| Category | Crate(s) | Strategy | Dependencies |
|---|---|---|---|
| Unit (pure) | mux-core, mux-query, mux-grid | `#[test]` + proptest + insta | None |
| Unit (codec) | mux-proto | Fixture roundtrip + fuzz | Fixture files |
| Scenario replay | mux-grid, mux-core | ScenarioReplayer + insta | Scenario fixtures |
| Integration (local) | mux-backend, mux-api | FakePty + MuxServerTestServer | mux-pty-fake |
| Integration (tmux) | mux-backend, mux-api | TmuxTestServer | Real tmux binary |
| Parity | mux-regress | Same ops on both servers, diff output | Real tmux + our server |
| E2E | mux-tui, bindings | Full stack with real/fake PTY | Full runtime |
| Fuzz | mux-proto, mux-grid | libfuzzer via cargo-fuzz | None |
| Property | mux-core, mux-crdt | proptest strategies | None |

### 23.2 Grid Snapshot Testing

```rust
#[test]
fn grid_after_hello_world() {
    let mut grid = Grid::new(80, 24);
    let mut parser = Parser::new();
    parser.parse(b"Hello, World!\r\nSecond line\r\n", &mut grid);
    insta::assert_snapshot!(grid.to_string());
}
```

### 23.3 Scenario Replay Testing

```rust
#[test]
fn replay_ls_scenario() {
    let scenario = PtyScenario::load("fixtures/scenarios/ls-command.json").unwrap();
    let mut graph = ServerGraph::new();
    let session = graph.create_session("test");
    let window = graph.create_window("w1");
    graph.add_window_to_session(session, window);
    let pane_id = graph.create_pane(PaneSize { cols: 80, rows: 24 });
    graph.add_pane_to_window(window, pane_id);

    let mut replayer = ScenarioReplayer::new(scenario);
    let snapshots = replayer.replay_full(&mut graph, pane_id, &[500, 1000, 2000]);

    for (ts, grid_text) in &snapshots {
        insta::assert_snapshot!(format!("ls_at_{ts}ms"), grid_text);
    }
}
```

### 23.4 Format Engine Parity Testing

```rust
#[test]
fn format_parity_session_name() {
    let tmux_output = tmux_display_message("#{session_name}", &test_server);
    let our_output = format_expand("#{session_name}", &format_ctx);
    assert_eq!(tmux_output, our_output);
}
```

### 23.5 VT100 Parser Design (cross-ref: Section 7 PaneOutput)

#### Why Custom (Not vte Crate)

tmux's `input.c` implements a specific variant of the Paul Williams VT100 state machine with tmux-specific extensions:
- 7-bit only (no 8-bit C1 controls)
- UTF-8 support via a separate top-bit-set handler
- OSC terminated by BEL (0x07) as well as ST
- APC state for title setting
- A special "rename_string" state for ESC-k...ESC-\ sequence
- DCS escape handling

#### 17 Parser States

Verified against `input.c` lines 369-386:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParserState {
    Ground, EscEnter, EscIntermediate,
    CsiEnter, CsiParameter, CsiIntermediate, CsiIgnore,
    DcsEnter, DcsParameter, DcsIntermediate, DcsHandler, DcsEscape, DcsIgnore,
    OscString, ApcString, RenameString, ConsumeSt,
}
```

#### Parser Context

Matches tmux's `struct input_ctx` (verified at `input.c` lines 99-147):

```rust
#[derive(Debug, Clone)]
pub struct Parser {
    state: ParserState,
    interm_buf: [u8; 4],
    interm_len: usize,
    param_buf: [u8; 64],
    param_len: usize,
    params: [Param; 24],
    param_count: usize,
    input_buf: Vec<u8>,
    input_end: InputEndType,
    utf8_state: Utf8State,
    cell: CellState,
    saved_cell: CellState,
    saved_cx: u32,
    saved_cy: u32,
    ch: u8,
}
```

#### CSI Commands (39 types)

Verified against `input.c` lines 257-344:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CsiCommand {
    Cbt, Cnl, Cpl, Cub, Cud, Cuf, Cup, Cuu,
    Da, DaTwo, Dch, Decscusr, Decstbm, Dl,
    Dsr, DsrPrivate, Ech, Ed, El, Hpa, Ich, Il,
    ModOff, ModSet, QueryPrivate, Rcp, Rep,
    Rm, RmPrivate, Scp, Sd, Sgr,
    Sm, SmGraphics, SmPrivate, Su, Tbc, Vpa, Winops, Xda,
}
```

### 23.6 Format String Engine

tmux's `format.c` (5600+ lines) implements variable expansion, conditionals, comparisons, loops, modifiers, arithmetic, and string operations.

```rust
pub struct FormatContext<'a> {
    graph: &'a ServerGraph,
    client: Option<ClientId>,
    session: Option<SessionId>,
    window: Option<WindowId>,
    pane: Option<PaneId>,
    runtime_vars: &'a HashMap<String, String>,
}

pub fn format_expand(template: &str, ctx: &FormatContext<'_>) -> String {
    let mut es = ExpandState::new(ctx, 0);
    expand_inner(&mut es, template)
}
```

### 23.7 Key Binding System

```rust
pub type KeyCode = u64;

#[derive(Debug, Clone)]
pub struct KeyBinding {
    pub key: KeyCode,
    pub commands: Vec<ParsedCommand>,
    pub note: Option<String>,
    pub flags: KeyBindingFlags,
}

#[derive(Debug, Clone)]
pub struct KeyTable {
    pub name: String,
    pub bindings: BTreeMap<KeyCode, KeyBinding>,
    pub default_bindings: BTreeMap<KeyCode, KeyBinding>,
}

#[derive(Debug, Clone, Default)]
pub struct KeyTableSet {
    tables: BTreeMap<String, KeyTable>,
}
```

### 23.8 Copy Mode

```rust
#[derive(Debug, Clone)]
pub struct CopyModeState {
    pub screen: Grid,
    pub oy: u32,
    pub cx: u32, pub cy: u32,
    pub selection: Option<Selection>,
    pub search: Option<SearchState>,
    pub mode_keys: ModeKeys,
}

pub fn apply_copy_mode_action(
    state: &mut CopyModeState,
    action: CopyModeAction,
    grid: &Grid,
) -> Vec<CopyModeEffect> {
    match action {
        CopyModeAction::CursorUp => {
            if state.cy > 0 { state.cy -= 1; }
            else if state.oy < grid.scrollback_lines() { state.oy += 1; }
            vec![CopyModeEffect::Redraw]
        }
        CopyModeAction::CopySelectionAndCancel => {
            let text = extract_selection_text(state, grid);
            vec![CopyModeEffect::SetBuffer(text), CopyModeEffect::ExitCopyMode]
        }
        CopyModeAction::Cancel => vec![CopyModeEffect::ExitCopyMode],
        _ => vec![],
    }
}
```

### 23.9 Test Harness Test Strategy

1. **Grid snapshots:** Feed known VT100 sequences, assert with `insta`.
2. **Scenario replay:** Replay recorded session, compare grid at checkpoints.
3. **Format parity:** 200+ format strings compared between tmux and TermForge.
4. **Key binding parity:** Default tables match tmux across all 4 key tables.
5. **Copy mode navigation:** Property test: cursor never exceeds grid bounds.
6. **Copy mode selection:** Snapshot test for selection extraction.
7. **Fuzz:** VT100 parser fuzz target, format parser fuzz target.

---

## 24. Performance Targets

### 24.1 Targets Table

| Benchmark | Target | Basis |
|---|---|---|
| VT100 parser, plain ASCII | > 300 MB/s | alacritty vte ~500 MB/s |
| VT100 parser, CSI heavy | > 100 MB/s | Parameter parsing overhead |
| Protocol frame decode | > 500 MB/s | 16-byte header + memcpy |
| Layout resize (20 panes) | < 50 us | Tree walk ~60 nodes |
| Format expand (status line) | < 50 us | ~10 variable lookups |
| Graph snapshot | < 1 ms | Arc clone + BTreeMap for 50 panes |
| Config parse (500 lines) | < 5 ms | Lexer + parser |
| Option resolve (4-level chain) | < 100 ns | 4 BTreeMap lookups |
| Control notification parse | < 200 ns | String split + parse |

**Important:** No existing baselines exist. Fix `mux-refresh/Cargo.toml` harness first, establish baselines, then enforce regression gates.

```toml
# Add to crates/mux-refresh/Cargo.toml
[[bench]]
name = "latency"
harness = false
```

### 24.2 Benchmark Suite

```rust
// crates/mux-bench/benches/hot_path.rs
use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId, black_box};

fn bench_layout_checksum(c: &mut Criterion) {
    let layout = "204x50,0,0{102x50,0,0,0,101x50,103,0[101x25,103,0,1,101x24,103,26,2]}";
    c.bench_function("layout_checksum", |b| {
        b.iter(|| black_box(layout_checksum(layout.as_bytes())))
    });
}

fn bench_layout_resize(c: &mut Criterion) {
    let mut group = c.benchmark_group("layout_resize");
    for n_panes in [4, 10, 20, 50] {
        group.bench_with_input(BenchmarkId::from_parameter(n_panes), &n_panes, |b, &n| {
            let panes: Vec<PaneId> = (0..n).collect();
            let mut tree = LayoutPreset::Tiled.arrange(&panes, 200, 60, None);
            b.iter(|| { tree.resize(180, 50); tree.resize(200, 60); });
        });
    }
    group.finish();
}

fn bench_control_parse(c: &mut Criterion) {
    let lines = vec![
        "%sessions-changed", "%session-changed $0 work",
        "%window-add @1", "%output %0 hello world",
        "%layout-change @0 d3a0,204x50,0,0{102x50,0,0,0,101x50,103,0}",
    ];
    c.bench_function("control_parse_5_notifications", |b| {
        b.iter(|| { for line in &lines { black_box(parse_notification(line)); } })
    });
}

criterion_group!(benches, bench_layout_checksum, bench_layout_resize, bench_control_parse);
criterion_main!(benches);
```

### 24.3 CI Regression Gate

130% threshold, nightly only, assert non-empty output. Baselines from 3 consecutive median runs.

---

## 25. Visual Client / TUI

### 25.1 Architecture Overview

```
          +-------------------+
          |  Terminal (raw)   |
          |  crossterm events |
          +--------+----------+
                   |
          +--------v----------+
          |     mux-tui       |
          |  Event Loop       |
          |  Key Dispatch     |
          |  ViewModel Build  |
          |  ratatui Render   |
          +--------+----------+
                   |
          +--------v----------+
          |    mux-api        |
          |  StateHandle      |
          +--------+----------+
                   |
      +------------+-------------+
      |                          |
+-----v-------+    +------------v----+
| TermForge   |    | Real tmux       |
| server      |    | (via mux-client)|
+-------------+    +-----------------+
```

### 25.2 ratatui Rendering Pipeline

Follows `Terminal::draw` closure pattern (verified at `~/study/rust/ratatui/ratatui/src/lib.rs:192-196`). Uses `Buffer::diff` (verified at `ratatui-core/src/buffer/buffer.rs:492`).

```rust
// tools/mux-tui/src/app.rs
pub struct App {
    state_handle: StateHandle<GraphState>,
    api: ManagedMux,
    key_state: TuiKeyState,
    should_quit: bool,
}

impl App {
    pub fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
        loop {
            let snapshot = self.state_handle.load();
            let view_model = build_view_model(&snapshot, &self.key_state);
            terminal.draw(|frame| render_multiplexer(frame, &view_model))?;
            if crossterm::event::poll(Duration::from_millis(16))? {
                if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                    self.handle_key(key)?;
                }
            }
            if self.should_quit { break; }
        }
        Ok(())
    }
}
```

### 25.3 ViewModel Construction (Pure)

```rust
// crates/mux-view/src/view_model.rs
#[derive(Debug, Clone)]
pub struct MultiplexerView {
    pub panes: Vec<PaneView>,
    pub status_left: String,
    pub status_right: String,
    pub borders: Vec<BorderSegment>,
    pub active_pane_idx: usize,
    pub window_list: Vec<WindowListEntry>,
    pub mode_indicator: Option<String>,
}

pub fn build_view_model(
    snapshot: &GraphState,
    key_state: &TuiKeyState,
) -> MultiplexerView {
    // Pure transformation from GraphState to render-ready structures
    todo!()
}
```

### 25.4 Grid-to-Buffer Rendering

```rust
pub struct PaneWidget<'a> { pub view: &'a PaneView }

impl<'a> Widget for PaneWidget<'a> {
    fn render(self, area: ratatui::layout::Rect, buf: &mut Buffer) {
        let grid = &self.view.grid_snapshot;
        for row in 0..area.height.min(grid.rows()) as usize {
            for col in 0..area.width.min(grid.cols()) as usize {
                if let Some(cell) = grid.cell(row, col) {
                    let x = area.x + col as u16;
                    let y = area.y + row as u16;
                    let style = grid_style_to_ratatui(&cell.style);
                    let ch = if cell.ch == '\0' { ' ' } else { cell.ch };
                    buf[(x, y)].set_char(ch).set_style(style);
                }
            }
        }
    }
}
```

### 25.5 Key Dispatch and Rendering Loop

60fps target (16ms per frame). Event-driven + time-driven hybrid.

### 25.6 TUI Rules

- Never call core graph directly. All reads via `StateHandle`.
- All writes via `mux-api` events.
- Works against both TermForge and real tmux.
- ViewModel is a pure function (snapshot-testable).
- Grid-to-Buffer handles wide characters (CJK).
- Status line uses same `format_expand()`.

### 25.7 TUI Test Strategy

1. **Snapshot rendering:** Known grid -> ratatui `Buffer` -> `insta`.
2. **Status line:** `format_expand` correct for templates.
3. **Input mapping:** crossterm keys -> TermForge `KeyCode`.
4. **Layout rendering:** Multi-pane borders at correct positions.
5. **Wide character:** CJK spanning 2 columns.
6. **Color mapping:** 256 indexed + RGB.
7. **Attach to tmux/TermForge:** First frame without crash.
8. **ViewModel purity:** Same input -> same output (property test).
9. **Resize handling:** Terminal resize propagates correctly.

---

## 26. AGENTS.md Rules

### 26.1 Error Handling Rules

**Rule 1 -- Error Taxonomy.**
Every library crate defines a public `Error` enum deriving `thiserror::Error`. `anyhow` is allowed only in binary crates and test code.
**Rationale:** Typed errors enable callers to match on specific failure modes. `anyhow` erases types, which is acceptable at the top level but harmful in libraries.
**Enforcement:** `grep -r 'anyhow' crates/*/src/` CI check fails if any library crate uses `anyhow` outside `#[cfg(test)]`.

**Rule 2 -- Error Classification.**
Every error type implements `Classified` returning `Transient`, `ProtocolViolation`, `UserError`, or `Bug`.
**Rationale:** Classification drives recovery policy. `Transient` errors are retried; `ProtocolViolation` kills the connection; `UserError` is reported to the user; `Bug` is logged and reported.
**Enforcement:** `cargo clippy` custom lint checks that all public `Error` types implement `Classified`.

**Rule 3 -- Protocol Violation.**
Any binary protocol decode error other than "need more bytes" closes that client connection. Reference: `server-client.c:3472-3475`. Never drop a malformed frame and continue.
**Rationale:** Continuing after a protocol violation can lead to state corruption. tmux kills the peer on any decode error, and TermForge must match this behavior for security and compatibility.
**Enforcement:** Integration test: inject malformed frame, verify connection terminated within 100ms.

**Rule 4 -- Pure Error Boundary.**
`io::Error` and platform types must not cross the pure/impure boundary. Convert to stable domain errors via `Event::EffectFailed`.
**Rationale:** Leaking `io::Error` into Layer 0 prevents WASM compilation and makes the core untestable without mocking. Domain errors are stable, serializable, and testable.
**Enforcement:** `trybuild` test: attempt to use `io::Error` in `mux-core`, verify compile failure.

### 26.2 Configuration Rules

**Rule 5 -- Config-as-Events.**
Config file parsing produces `Vec<Event>` submitted through the state actor. Direct graph mutation from config parsing is forbidden.
**Rationale:** Single path for all state mutations ensures consistency, auditability, and makes config loading testable as pure event processing.
**Enforcement:** `mux-conf` has no dependency on `mux-core`'s graph types. Only `Event` types are shared.

**Rule 6 -- Option Scope Resolution.**
Resolve using option table scope + command flags, matching `options_scope_from_name()` including the FALLTHROUGH from WindowPane to Window (`options.c:903`).
**Rationale:** Exact compatibility with tmux's option resolution prevents user-visible behavioral differences.
**Enforcement:** Exhaustive test matrix covering all (scope, flag, target) combinations, compared against tmux output.

**Rule 7 -- Unset Semantics.**
`set -u` removes local override (inheritance restored) for non-global. Resets to compiled default for global. `set -U` additionally clears pane-local values.
**Rationale:** Matches `options_remove_or_default` at `options.c:1269-1285`. Incorrect unset semantics cause user confusion.
**Enforcement:** Test: set at pane, unset, verify inherited value; set at global, unset, verify default.

### 26.3 Layout Rules

**Rule 8 -- Layout String Parity.**
Layout dump/parse and checksum must match `layout-custom.c` exactly. Verify with roundtrip tests against real tmux layout strings.
**Rationale:** Layout strings are exchanged between clients and servers on the wire. Mismatched checksums cause layout corruption.
**Enforcement:** `rstest` with vectors from real tmux, verify checksum and roundtrip.

**Rule 9 -- Layout Minimum.**
`layout_resize()` never fails. Clamps to minimum per `layout_resize_check()`. `PANE_MINIMUM` is 1 cell (`tmux.h:100`).
**Rationale:** tmux never returns an error from resize. Returning an error would change client-observable behavior.
**Enforcement:** Property test: random resize dimensions always produce a valid layout (layout_check passes).

**Rule 10 -- Round-Robin Distribution.**
`layout_resize_adjust()` distributes one cell at a time in round-robin, NOT proportionally. Matches `layout.c:448-462`.
**Rationale:** Proportional distribution produces different pixel geometry than tmux, causing visual mismatches. Round-robin is the only correct approach for compatibility.
**Enforcement:** Test: 3 uneven panes at 100 cols, resize to 103, verify distribution is +1, +1, +1 (not +2, +1, +0).

**Rule 11 -- Layout Consistency.**
After every layout mutation, `layout_check()` must return true. Add as `debug_assert!` in all mutating methods.
**Rationale:** Layout inconsistency causes rendering artifacts and crashes. tmux checks this invariant after every mutation.
**Enforcement:** `debug_assert!(self.layout_check())` at the end of every mutating method. Fails loudly in debug builds.

### 26.4 Telemetry Rules

**Rule 12 -- Span Naming.**
All span names use `termforge.` prefix. Pattern: `termforge.<subsystem>.<operation>`.
**Rationale:** Prevents collision with third-party library spans (tokio, hyper, tonic). Makes filtering trivial.
**Enforcement:** `grep -r 'info_span!' crates/ | grep -v 'termforge\.'` returns empty (CI check).

**Rule 13 -- Telemetry Propagation.**
New threads, tasks, and spawned processes must attach OTEL context.
**Rationale:** Broken trace context creates orphaned spans that are impossible to correlate.
**Enforcement:** Integration test: spawn task, verify child span has correct parent.

### 26.5 Security Rules

**Rule 14 -- Untrusted Input.**
All data from sockets, control mode, config files, and binding FFI is untrusted. Validation at crate boundaries.
**Rationale:** Defense in depth. Even internal crate boundaries validate inputs to catch bugs early.
**Enforcement:** Each validation layer has dedicated fuzz targets (Section 18.6).

**Rule 15 -- SCM_RIGHTS.**
FDs via SCM_RIGHTS accepted only during identify handshake. CLOEXEC immediately. No ancillary fds outside handshake.
**Rationale:** Out-of-phase FDs could be used to inject file descriptors into the server, potentially enabling privilege escalation.
**Enforcement:** Integration test: send FD after identify, verify closed and connection killed.

**Rule 16 -- Control Mode Hints.**
Control notifications are hints, never authoritative. Binary protocol is authority. Dropped notifications corrected by periodic refresh.
**Rationale:** Control notifications can be lost (backpressure, network issues). Treating them as authoritative causes state drift.
**Enforcement:** Test: drop notification, verify periodic refresh corrects state within 1 cycle.

### 26.6 Lifecycle Rules

**Rule 17 -- Lock File.**
Use `flock(LOCK_EX|LOCK_NB)`, not PID-based locking. Automatically released on process death.
**Rationale:** PID-based locking has TOCTOU races and PID reuse risks. flock is kernel-managed and crash-safe.
**Enforcement:** Test: start server, `kill -9`, start again, succeeds.

**Rule 18 -- Config Timing.**
Do not load config until first client completes identify burst. Matches `server-client.c:3725-3734`.
**Rationale:** Config errors must be reported to the first client. Loading config before identify means errors have no destination.
**Enforcement:** Integration test: trace startup, verify config loaded after identify.

### 26.7 Binding Rules

**Rule 19 -- Pure Logic Separation.**
All binding crates separate pure Rust logic (`logic.rs`) from binding glue (`lib.rs`). Pure logic has no PyO3/Neon/cxx types.
**Rationale:** Pure logic is testable with standard `#[test]` without requiring a language runtime (Python, Node.js, V8).
**Enforcement:** `cfg(not(test))` guards on binding imports. `logic.rs` compiles without feature flags.

**Rule 20 -- GIL Release.**
All Python blocking operations release the GIL via `Python::detach` (PyO3 0.26+).
**Rationale:** Holding the GIL during IO blocks all other Python threads, making the library unusable in concurrent applications.
**Enforcement:** Test: concurrent Python thread confirms GIL released during blocking call.

**Rule 21 -- Neon Channel for Async.**
All Node.js async operations use Neon `Channel` for scheduling results back to the JS main thread.
**Rationale:** Neon `Channel` safely bridges Rust background threads to the V8 event loop without data races.
**Enforcement:** Test: async command returns Promise that resolves correctly.

### 26.8 DOs

1. **DO** use `#![forbid(unsafe_code)]` in every Layer 0 crate.
2. **DO** verify Layer 0 compiles to `wasm32-unknown-unknown` in CI.
3. **DO** use `slotmap::new_key_type!` for all entity IDs.
4. **DO** return `Result` from all fallible operations.
5. **DO** use `#[must_use]` on pure functions returning values.
6. **DO** use `#[non_exhaustive]` on public enums that may grow.
7. **DO** use `insta` for snapshot tests, `proptest` for property tests.
8. **DO** put all `unsafe` in `mux-os` with `// SAFETY:` comments.
9. **DO** use `tracing` for structured logging with span context.
10. **DO** test protocol parsing against real tmux captures.
11. **DO** use scenario recordings for VT100 parser regression testing.
12. **DO** match tmux's exact state table for the VT100 parser (17 states).
13. **DO** implement the format string engine as a pure function.
14. **DO** model key tables and copy mode as pure state in the core.
15. **DO** use `Arc<Grid>` with `Arc::make_mut` for copy-on-write grid snapshots.
16. **DO** compute reverse lookups during snapshot construction.
17. **DO** use `ArcSwap` for the snapshot publish/subscribe boundary.
18. **DO** inject time via events, never from OS clocks in core.
19. **DO** use `BTreeMap` for key tables (ordered iteration for `list-keys`).
20. **DO** store key tables globally in `ServerGraph`, not per-client.
21. **DO** mirror tmux checksum algorithm exactly on wire paths.
22. **DO** treat `set -u` as "remove local override".
23. **DO** enforce protocol-violation disconnect for binary protocol peers.
24. **DO** use round-robin distribution for layout resize.
25. **DO** validate layout consistency after every mutation.
26. **DO** cite tmux source line numbers for compatibility claims.
27. **DO** separate pure logic from binding glue in all binding crates.
28. **DO** release the GIL via `Python::detach` in all Python blocking operations.
29. **DO** use Neon `Channel` for all async Node.js operations.

### 26.9 DON'Ts

1. **DON'T** add `tokio`, `async`, or IO to Layer 0 crates.
2. **DON'T** use `unwrap()` or `expect()` in library code.
3. **DON'T** use `Arc<Mutex<_>>` on the read path. Use `ArcSwap`.
4. **DON'T** store back-pointers in the entity model.
5. **DON'T** put tmux protocol names in core types.
6. **DON'T** implement multiplexer logic in bindings or the ORM.
7. **DON'T** adopt `im-rs` until profiling shows `Clone` is a bottleneck.
8. **DON'T** use the `vte` crate.
9. **DON'T** test against the user's live tmux server.
10. **DON'T** let `mux-orm` become a second business-logic engine.
11. **DON'T** use `SecondaryMap` for parent-child relationships.
12. **DON'T** expose internal names in binding APIs.
13. **DON'T** use `std::thread::sleep` or `std::time::SystemTime` in pure crates.
14. **DON'T** skip `// SAFETY:` documentation on any `unsafe` block.
15. **DON'T** drop malformed binary frames and continue.
16. **DON'T** use proportional resize distribution.
17. **DON'T** load config before first client identifies.
18. **DON'T** claim benchmark regressions are gated until criterion harness is proven running.
19. **DON'T** use `py.allow_threads()` (renamed to `py.detach()` in PyO3 0.26).
20. **DON'T** put Neon/PyO3 types in `logic.rs` files.

---

## 27. Risks and Mitigations

### 27.1 Risk Register

| # | Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|---|
| R1 | VT100 parser parity divergence | Medium | High | Table-driven parser from tmux `input.c`; scenario recordings; vttest suite; 24h fuzz |
| R2 | Format engine completeness (200+ vars) | Medium | Medium | `format-audit` tool; start with 50 most common |
| R3 | Key binding compatibility (100+ bindings) | Medium | Medium | Default tables generated from `key-bindings.c` |
| R4 | Copy mode complexity (~7000 lines) | Medium | Medium | Pure state machine; snapshot tests; incremental |
| R5 | Layout string format divergence across versions | Low | High | Pin to protocol v8; version-gate parser changes |
| R6 | `flock` behavior differs across OSes | Medium | Medium | Test on all CI platforms |
| R7 | HLC counter overflow at same millisecond | Very Low | Medium | `checked_add` with forced millis advance |
| R8 | Control mode output backpressure | Medium | High | Per-client 16 MB limit |
| R9 | Python GIL contention | Medium | Medium | All ops release GIL via `Python::detach` |
| R10 | `options_scope_from_name` fallthrough logic | High | Medium | Exhaustive test matrix |
| R11 | Arena-based layout tree dead cells | Medium | Low | Compact periodically |
| R12 | Proportional resize mismatch | Resolved | High | Round-robin matching tmux |
| R13 | CRDT sync over untrusted network | Low (Phase 1) | High | Unix sockets only in Phase 1 |
| R14 | `mux-conf` parser divergence | Medium | Medium | Corpus of 100+ real tmux.conf |
| R15 | 16-bit checksum collision | Low | Low | BLAKE3-128 for persistence |
| R16 | PID reuse in stale lock handling | Resolved | Medium | flock |
| R17 | Control output backlog unfair scheduling | Low | Low | Per-pane quotas |
| R18 | Multiple FDs in one ancillary message | Low | Medium | Close extras deterministically |
| R19 | Array option index unset divergence | Medium | Medium | Mirror `options_remove_or_default` |
| R20 | `mux-types` crate not created | Certain | Medium | Create as first action |
| R21 | Criterion benchmarks not wired | Certain | Low | Fix `[[bench]]` section |
| R22 | WASM compilability blocks useful crates | Low | Medium | WASM check is `cargo check` only |
| R23 | Protocol undocumented behaviors | Medium | Medium | Real captures; `tmux-sniff` |
| R24 | SCM_RIGHTS platform complexity | Medium | Medium | Isolated in `mux-os` |
| R25 | Binding memory safety | Low | High | Thin wrappers; Miri testing |
| R26 | Large scrollback performance | Medium | Medium | `Arc<Grid>` with COW |
| R27 | Test isolation failures | Low | Medium | Unique temp dirs; env clearing |
| R28 | PyO3 API breakage (allow_threads -> detach) | Resolved | Medium | Pinned to PyO3 0.26+; use `detach` |
| R29 | Neon Channel lifetime safety | Low | Medium | Thin wrapper; deferred.settle_with pattern |

### 27.2 Reference Verification Table

| Claim | Status | Evidence |
|---|---|---|
| Identify burst types 100-112 | Verified | `tmux-protocol.h:29-41` |
| `goto bad -> proc_kill_peer` | Verified | `server-client.c:3472-3475` |
| `layout_checksum` algorithm | Verified | `layout-custom.c:46-57` |
| `layout_resize_adjust` round-robin | Verified | `layout.c:448-462` |
| `layout_resize_check` computation | Verified | `layout.c:366-415` |
| `layout_check` validation | Verified | `layout-custom.c:119-153` |
| `PANE_MINIMUM = 1` | Verified | `tmux.h:100` |
| `options_get` parent chain | Verified | `options.c:228-241` |
| `options_remove_or_default` | Verified | `options.c:1269-1285` |
| `options_scope_from_name` FALLTHROUGH | Verified | `options.c:891-903` |
| `client_get_lock` uses flock | Verified | `client.c:77-101` |
| Config load after first client | Verified | `server-client.c:3725-3734` |
| `control_start` no auth | Verified | `control.c:758-796` |
| `%extended-output` format | Verified | `control.c:620-623` |
| Control backpressure/disconnect | Verified | `control.c:450-461` |
| Socket permissions umask | Verified | `server.c:126-129` |
| VT100 parser 17 states | Verified | `input.c:369-386` |
| VT100 parser context struct | Verified | `input.c:99-147` |
| CSI command table | Verified | `input.c:257-344` |
| Protocol version 8 | Verified | `tmux-protocol.h:23` |
| Split minimum PANE_MINIMUM * 2 + 1 | Verified | `layout.c:937-950` |
| vibe-tmux graph_state reverse maps | Verified | `graph.rs:215-295` |
| vibe-tmux ControlNotification generic | Verified | `control.rs:28-36` |
| SCM_RIGHTS CLOEXEC in vibe-tmux | Verified | `scm_rights.rs:141-149` |
| ratatui Terminal::draw closure | Verified | `lib.rs:192-196` |
| ratatui Buffer::diff | Verified | `buffer.rs:492` |
| PyO3 `Bound<'py, T>` smart pointer | Verified | `pyo3/src/instance.rs` |
| PyO3 `Python::detach` (was `allow_threads`) | Verified | `pyo3/guide/src/migration.md:300-312` |
| Neon `FunctionContext` | Verified | `neon/crates/neon/src/context/mod.rs` |
| Neon `Channel` for async | Verified | `neon/crates/neon/src/event/channel.rs` |
| OTEL `SdkTracerProvider` | Verified | `opentelemetry-sdk/src/trace/provider.rs` |
| OTEL `SpanExporter` trait | Verified | `opentelemetry-sdk/src/trace/export.rs` |
| OTEL `BatchSpanProcessor` defaults | Verified | `opentelemetry-sdk/src/trace/span_processor.rs` |
| OTEL `SamplingDecision` enum | Verified | `opentelemetry-sdk/src/trace/sampler.rs` |
| `mux-types` crate exists | **Missing** | Not in vibe-tmux workspace |
| `LockFile` in vibe-tmux | **Missing** | No implementation yet |
| `HybridClock` in vibe-tmux | **Missing** | No implementation yet |
| Criterion harness working | **Missing** | Needs `[[bench]]` fix |

---

## 28. Summary and Changelog

### What This Document Is

This is the v7 Pass 1 architecture specification for TermForge. It refines v6 Final with:

1. **OTEL SDK verification:** Section 19 now references actual `opentelemetry-rust` SDK types (`SdkTracerProvider`, `SpanData`, `BatchSpanProcessor`, `SamplingDecision`), verified against source at `~/study/otel/opentelemetry-rust/`.

2. **PyO3 API update:** All Python binding code uses `Python::detach` (renamed from `allow_threads` in PyO3 0.26, verified at `guide/src/migration.md:300-312`). Uses `Bound<'py, T>` smart pointer pattern.

3. **Neon API verification:** Node.js binding code uses `FunctionContext`, `Channel`, and `#[neon::main]` attribute, verified against source at `~/study/rust-node/neon/`.

4. **Pure logic separation:** All binding crates adopt the `logic.rs` / `lib.rs` separation pattern from `~/study/rust/learning-rust-nodejs/`, enabling standard `#[test]` without language runtimes.

5. **Version Management expanded:** Section 20 now includes concrete CLI workflows, implementation details, and multi-version test matrix.

6. **Binding Tests expanded:** Section 22 now includes real PyO3/Neon fixture code, snapshot testing examples, and pure logic test examples.

7. **AGENTS.md Rules expanded:** Section 26 now includes rationale and enforcement mechanism for every rule, plus 3 new binding-specific rules (19-21).

### v6 -> v7 Changes

| Area | v6 State | v7 Change |
|---|---|---|
| OTEL SDK types | Generic sampling/export types | Verified against actual SDK: SdkTracerProvider, SpanExporter, BatchSpanProcessor (queue 2048, delay 5000ms), SamplingDecision (Drop/RecordOnly/RecordAndSample) |
| PyO3 API | `py.allow_threads()` | Updated to `py.detach()` (PyO3 0.26 rename, verified at migration.md:300-312) |
| PyO3 types | Generic | Uses `Bound<'py, T>`, `#[pyfunction]`, `wrap_pyfunction!` verified at pyo3/src/instance.rs |
| Neon API | NAPI-RS references | Updated to Neon with `FunctionContext`, `Channel`, `#[neon::main]` verified at neon source |
| Pure logic pattern | Not mentioned | New Rule 19: separate `logic.rs` from binding glue, pattern from learning-rust-nodejs |
| Version Management (S20) | ~35 lines | ~120 lines: concrete CLI, implementation, multi-version matrix |
| Binding Tests (S22) | ~63 lines | ~150 lines: real PyO3/Neon fixtures, snapshot tests, pure logic tests |
| AGENTS.md Rules (S26) | ~102 lines, no rationale | ~200 lines: rationale + enforcement per rule, 3 new binding rules |
| Risk register | 27 risks | 29 risks: added R28 (PyO3 API rename), R29 (Neon Channel safety) |
| Verification table | tmux + vibe-tmux | Added PyO3, Neon, OTEL SDK verification entries |

### Completeness Checklist

- [x] Preamble
- [x] 1. Vision and Philosophy
- [x] 2. North Star Acceptance Criteria
- [x] 3. High-Level Architecture
- [x] 4. Workspace Layout
- [x] 5. Layering Contract
- [x] 6. Entity Model
- [x] 7. Event/Effect Engine
- [x] 8. Error Handling
- [x] 9. Protocol Codec
- [x] 10. Configuration System
- [x] 11. Layout Engine
- [x] 12. ORM-like Query API
- [x] 13. Runtime Architecture
- [x] 14. Server Lifecycle
- [x] 15. Control Mode
- [x] 16. Language Bindings
- [x] 17. CRDT Transaction Layer
- [x] 18. Security Model
- [x] 19. OpenTelemetry
- [x] 20. tmux Version Management
- [x] 21. Test Support and Fake PTY
- [x] 22. Binding Test Frameworks
- [x] 23. Test Framework and Harness Design
- [x] 24. Performance Targets
- [x] 25. Visual Client / TUI
- [x] 26. AGENTS.md Rules
- [x] 27. Risks and Mitigations
- [x] 28. Summary and Changelog
- [x] 29. Reference Anchors
- [x] 30. Appendix: Canonical Type Quick Reference
- [x] 31. Supplemental Test Matrix

---

## 29. Reference Anchors

### tmux C source (`~/study/c/tmux/`)

- `tmux-protocol.h:23` -- PROTOCOL_VERSION 8
- `tmux-protocol.h:29-41` -- Identify burst types (100-112)
- `tmux.h:100` -- PANE_MINIMUM = 1
- `server.c:126-129` -- Socket permissions umask
- `server-client.c:3472-3475` -- goto bad -> proc_kill_peer
- `server-client.c:3590-3600` -- Reject post-identify (CLIENT_IDENTIFIED check)
- `server-client.c:3725-3734` -- Config load after first client
- `client.c:77-101` -- flock-based lock file
- `options.c:228-241` -- Option parent chain walk
- `options.c:891-903` -- FALLTHROUGH from WindowPane to Window
- `options.c:1269-1285` -- options_remove_or_default
- `control.c:450-461` -- Control backpressure/disconnect
- `control.c:620-623` -- %extended-output format
- `control.c:758-796` -- control_start (no auth)
- `layout-custom.c:46-57` -- Layout checksum algorithm
- `layout-custom.c:119-153` -- Layout validation (layout_check)
- `layout.c:366-415` -- layout_resize_check
- `layout.c:448-462` -- layout_resize_adjust (round-robin)
- `layout.c:937-950` -- Split minimum (PANE_MINIMUM * 2 + 1)
- `input.c:99-147` -- Parser context struct
- `input.c:257-344` -- CSI command table
- `input.c:369-386` -- 17 parser state forward declarations

### vibe-tmux prototype (`~/work/rust/vibe-tmux/crates/`)

- `mux-core/src/graph.rs:215-295` -- graph_state reverse maps
- `mux-client/src/control.rs:28-36` -- ControlNotification (generic struct)
- `mux-os/src/scm_rights.rs:141-149` -- CLOEXEC via set_cloexec()
- `mux-otel/src/otel.rs:283-321` -- OTEL init and subscriber wiring
- `mux-otel/src/config.rs:52-103` -- OTEL allow/deny config

### ratatui (`~/study/rust/ratatui/`)

- `ratatui/src/lib.rs:192-196` -- Terminal::draw closure pattern
- `ratatui-core/src/buffer/buffer.rs:492` -- Buffer::diff

### libtmux (`~/work/python/libtmux/`)

- `src/libtmux/server.py:18` -- Server class
- `src/libtmux/session.py:15` -- Session class
- `src/libtmux/window.py:16` -- Window class

### PyO3 (`~/study/rust-python/pyo3/`)

- `src/instance.rs` -- `Bound<'py, T>` smart pointer
- `src/marker.rs:558` -- `Python::detach` (formerly `allow_threads`)
- `guide/src/migration.md:300-312` -- PyO3 0.26 renames

### Neon (`~/study/rust-node/neon/crates/neon/src/`)

- `lib.rs` -- `#[neon::main]`, `FunctionContext`
- `context/mod.rs` -- `Context` trait, `ModuleContext`, `cx.argument()`
- `event/channel.rs` -- `Channel` for async scheduling

### OTEL Rust SDK (`~/study/otel/opentelemetry-rust/`)

- `opentelemetry/src/trace/tracer.rs` -- `Tracer` trait
- `opentelemetry/src/trace/span.rs` -- `Span` trait
- `opentelemetry-sdk/src/trace/provider.rs` -- `SdkTracerProvider`
- `opentelemetry-sdk/src/trace/span.rs` -- SDK `Span`, `SpanData`
- `opentelemetry-sdk/src/trace/export.rs` -- `SpanExporter` trait
- `opentelemetry-sdk/src/trace/span_processor.rs` -- `BatchSpanProcessor`
- `opentelemetry-sdk/src/trace/sampler.rs` -- `SamplingDecision`

### Learning projects

- `~/study/rust/learning-rust-python/src/lib.rs` -- PyO3 pattern: `#[pyfunction]`, `#[pymodule]`, `Bound<'_, PyModule>`
- `~/study/rust/learning-rust-nodejs/native/src/lib.rs` -- Neon pattern: `register_module!`, `cfg(not(test))` guards
- `~/study/rust/learning-rust-nodejs/native/src/logic.rs` -- Pure logic separation: testable without V8

---

## 30. Appendix: Canonical Type Quick Reference

### ErrorClass, Classified, DecodeOutcome

```rust
#![forbid(unsafe_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorClass { Transient, ProtocolViolation, UserError, Bug }

pub trait Classified {
    fn class(&self) -> ErrorClass;
    fn is_fatal(&self) -> bool {
        matches!(self.class(), ErrorClass::ProtocolViolation | ErrorClass::Bug)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeOutcome { NeedMore, Frame(ImsgFrame), ProtocolViolation(ProtocolError) }
```

### LayoutTree, LayoutCell (Flat Arena)

```rust
pub const PANE_MINIMUM: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutType { WindowPane, LeftRight, TopBottom }

#[derive(Debug, Clone)]
pub struct LayoutCell {
    pub cell_type: LayoutType,
    pub sx: u16, pub sy: u16,
    pub xoff: u16, pub yoff: u16,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub pane_id: Option<PaneId>,
}

#[derive(Debug, Clone)]
pub struct LayoutTree { pub cells: Vec<LayoutCell> }
```

### OptionStore

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OptionScope { Server, Session, Window, Pane }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OptionValue { String(String), Number(i64), Flag(bool), Style(String), Array(Vec<String>) }

#[derive(Debug, Clone, Default)]
pub struct OptionStore { local: BTreeMap<String, OptionValue> }
```

### HlcTimestamp

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
         serde::Serialize, serde::Deserialize)]
pub struct NodeId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
         serde::Serialize, serde::Deserialize)]
pub struct HlcTimestamp { pub millis: u64, pub counter: u32, pub node_id: NodeId }
```

### Entity Types

```rust
use slotmap::{SlotMap, new_key_type};

new_key_type! {
    pub struct SessionId; pub struct WindowId; pub struct PaneId;
    pub struct ClientId; pub struct JobId; pub struct BufferId;
}

#[derive(Debug, Default, Clone)]
pub struct ServerGraph {
    pub sessions: SlotMap<SessionId, Session>,
    pub windows: SlotMap<WindowId, Window>,
    pub panes: SlotMap<PaneId, Pane>,
    pub clients: SlotMap<ClientId, Client>,
    pub jobs: SlotMap<JobId, Job>,
    pub buffers: SlotMap<BufferId, Buffer>,
    pub key_tables: KeyTableSet,
}
```

### Event, Effect, QueryOp

```rust
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Event {
    CreateSession { name: String, cwd: Option<String>, created_at: i64 },
    DestroySession { session_id: SessionId },
    CreateWindow { session_id: SessionId, name: String },
    CreatePane { window_id: WindowId, size: PaneSize, command: Vec<String>, cwd: Option<String> },
    PaneOutput { pane_id: PaneId, data: Vec<u8> },
    PaneExited { pane_id: PaneId, exit_status: i32 },
    Key { client_id: ClientId, key: KeyCode },
    Command { name: String, args: Vec<String>, client_id: Option<ClientId> },
    SetOption { scope: OptionScope, key: String, value: OptionValue },
    Tick { now_millis: i64 },
    EffectFailed { token: EffectToken, reason: String },
    // ... additional variants
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Effect {
    SpawnPane { pane_id: PaneId, command: Vec<String>, cwd: Option<String>, size: PaneSize },
    KillPane { pane_id: PaneId, signal: i32 },
    WritePane { pane_id: PaneId, data: Vec<u8> },
    ResizePty { pane_id: PaneId, size: PaneSize },
    SendToClient { client_id: ClientId, frame: FrameData },
    DisconnectClient { client_id: ClientId },
    Shutdown, PublishSnapshot,
    // ... additional variants
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryOp {
    Exact, Contains, IContains, StartsWith, IStartsWith,
    EndsWith, IEndsWith, Regex, In, Gt, Gte, Lt, Lte, IsNone, IsSome,
}
```

---

## 31. Supplemental Test Matrix

Concrete tests for sections without dedicated test-strategy headings.

### 31.1 Vision (Section 1)

| Test | Target | Expected | Framework |
|---|---|---|---|
| `non_port_claim_guard` | Document text | No "tmux port" phrase | grep |
| `protocol_target` | Version line | PROTOCOL_VERSION 8 | unit |

### 31.2 North Star (Section 2)

| Test | Target | Expected | Framework |
|---|---|---|---|
| `C1_attach_real_tmux` | Client attach | Exit code 0 | integration |
| `C4_identify_burst` | Identify 100-112 | Byte-exact roundtrip | insta |

### 31.3 Architecture (Section 3)

| Test | Target | Expected | Framework |
|---|---|---|---|
| `layer_import_guard` | Layer 0 purity | `mux-core` -> `mux-os` compile failure | trybuild |
| `snapshot_copy` | Snapshot immutability | Mutate graph, snapshot unchanged | unit |

### 31.4 Workspace (Section 4)

| Test | Target | Expected | Framework |
|---|---|---|---|
| `workspace_membership` | Cargo.toml | All required crates present | unit |
| `mux_types_leaf` | Dependency graph | No inward workspace deps | cargo metadata |

### 31.5 Layering (Section 5)

| Test | Target | Expected | Framework |
|---|---|---|---|
| `pure_crate_no_io` | `mux-core` | No `std::net`/`std::fs` usage | clippy |
| `layer_cycle_check` | Dependency graph | Acyclic | script |

### 31.6 Performance (Section 24)

| Test | Target | Expected | Framework |
|---|---|---|---|
| `bench_harness_present` | Cargo.toml | `[[bench]]` exists | unit |
| `ci_130pct_gate` | Baseline comparison | Fail when > 130% | script |

### 31.7 AGENTS.md (Section 26)

| Test | Target | Expected | Framework |
|---|---|---|---|
| `rule3_protocol_kill` | Rule 3 | Client disconnected on malformed frame | integration |
| `rule8_layout_checksum` | Rule 8 | Checksum matches tmux exactly | rstest |
| `rule10_round_robin` | Rule 10 | Round-robin (not proportional) | unit |
| `rule18_config_timing` | Rule 18 | Config after identify | integration |
| `rule19_pure_logic` | Rule 19 | `logic.rs` compiles without binding features | trybuild |
| `rule20_gil_release` | Rule 20 | GIL released during blocking call | pytest |
| `rule21_neon_channel` | Rule 21 | Promise resolves via Channel | vitest |

### 31.8 Risks (Section 27)

| Test | Target | Expected | Framework |
|---|---|---|---|
| `risk_id_uniqueness` | Risk table | All IDs unique (R1-R29) | unit |
| `reference_table_validity` | Cited paths | All files exist | checker |

### 31.9 Reference Anchors (Section 29)

| Test | Target | Expected | Framework |
|---|---|---|---|
| `tmux_anchor_exists` | tmux source | All files/lines valid | checker |
| `vibe_anchor_exists` | vibe-tmux | All files/lines valid | checker |
| `pyo3_anchor_exists` | PyO3 source | Files exist | checker |
| `neon_anchor_exists` | Neon source | Files exist | checker |
| `otel_anchor_exists` | OTEL SDK | Files exist | checker |

### 31.10 Canonical Types (Section 30)

| Test | Target | Expected | Framework |
|---|---|---|---|
| `error_types_compile` | ErrorClass block | Compiles | trybuild |
| `flat_arena_no_recursion` | LayoutTree | No recursive `Vec<LayoutCell>` | syn |
| `crdt_types_ordering` | HlcTimestamp | `Ord` works correctly | unit |
| `event_effect_non_exhaustive` | Event/Effect | `#[non_exhaustive]` present | syn |
