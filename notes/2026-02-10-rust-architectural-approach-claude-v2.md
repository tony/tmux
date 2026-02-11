I now have a thorough understanding of all the reference codebases. Let me compile the comprehensive architectural plan.

---

# TermForge: A Layered, CRDT-Ready Terminal Multiplexer with tmux Compatibility

## Comprehensive Implementation Plan

---

## 1. Project Name and Identity

**Name:** `termforge`

**Rationale:** The name captures the dual nature of the project -- it is both a terminal multiplexer and a forge (foundation) upon which libraries, test frameworks, bindings, and distributed collaborative sessions can be built. It avoids the "tmux" substring to signal architectural independence while the `tmux-compat` feature layer delivers full behavioral compatibility.

**Tagline:** "A deterministic, layered terminal multiplexer core with tmux wire-compatibility, ORM-style APIs, CRDT collaboration, and multi-language bindings."

---

## 2. Crate/Workspace Structure with Purity Classifications

### Workspace Layout

```
termforge/
├── Cargo.toml                          # workspace root
├── AGENTS.md                           # human + LLM guardrails
├── ARCHITECTURE.md                     # living architecture doc
│
├── crates/
│   │
│   │  ── LAYER 0: PURE (no IO, no async, no unsafe, no time) ──
│   │
│   ├── tf-types/                       # PURE: foundational types + typed IDs
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── id.rs                   # SessionId, WindowId, PaneId, ClientId, JobId, BufferId
│   │       ├── geometry.rs             # Rect, Size, Position, SplitDirection
│   │       ├── key.rs                  # KeyCode, KeyModifiers (from tmux.h KEYC_*)
│   │       ├── style.rs               # Color, Style, Attributes (from tmux grid_cell)
│   │       └── error.rs               # CoreError enum (pure, no IO errors)
│   │
│   ├── tf-query/                       # PURE: query DSL + operators
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── ops.rs                  # QueryOp enum (Exact, IContains, Regex, etc.)
│   │       ├── value.rs               # QueryValue (Null, Bool, I64, String, List)
│   │       ├── spec.rs                # QuerySpec, QueryTerm
│   │       ├── list.rs                # QueryList<T: Queryable> with filter/get/get_one
│   │       └── derive.rs              # #[derive(Queryable)] proc macro support
│   │
│   ├── tf-config/                      # PURE: config AST, lexer, parser
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── ast.rs                  # Config AST nodes
│   │       ├── lexer.rs               # Tokenizer
│   │       ├── parser.rs              # Config file parser (tmux.conf compatible)
│   │       └── options.rs             # Option table + defaults (from tmux options-table.c)
│   │
│   ├── tf-core/                        # PURE: deterministic state machine
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── graph.rs               # ServerGraph (SlotMap-backed entity store)
│   │       ├── session.rs             # Session entity
│   │       ├── window.rs              # Window entity
│   │       ├── pane.rs                # Pane entity
│   │       ├── client.rs              # Client entity
│   │       ├── job.rs                 # Job entity
│   │       ├── buffer.rs              # Paste buffer entity
│   │       ├── hook.rs                # Hook storage + dispatch model
│   │       ├── key_table.rs           # Key table + binding model
│   │       ├── event.rs               # Event enum (input intents)
│   │       ├── effect.rs              # Effect enum (runtime side effects)
│   │       ├── engine.rs              # apply_event(graph, event) -> ApplyOutcome
│   │       ├── facade.rs              # ServerFacade (command string -> event)
│   │       ├── handle.rs              # ServerHandle (read-only graph traversal)
│   │       ├── context.rs             # ClientContext (client-relative semantics)
│   │       ├── state.rs               # GraphState, SessionState, etc. (serializable snapshots)
│   │       ├── format.rs              # Format string expansion engine (from tmux format.c)
│   │       ├── layout.rs              # Layout tree + algorithms
│   │       ├── copy_mode.rs           # Copy mode state machine
│   │       └── command/               # Per-command handlers (pure)
│   │           ├── mod.rs
│   │           ├── new_session.rs
│   │           ├── new_window.rs
│   │           ├── split_window.rs
│   │           ├── select_pane.rs
│   │           ├── send_keys.rs
│   │           ├── display_message.rs
│   │           ├── set_option.rs
│   │           └── ...                # one file per tmux command
│   │
│   ├── tf-view/                        # PURE: view model + traversal helpers
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── view.rs               # View, ViewIndex, ViewMeta
│   │       ├── view_id.rs            # ViewId (Local | Tmux)
│   │       ├── refs.rs               # SessionRef, WindowRef, PaneRef (lifetime-bounded)
│   │       ├── query.rs              # query::session::*, query::window::*, query::pane::*
│   │       └── diff.rs               # View diffing for CRDT and incremental updates
│   │
│   ├── tf-grid/                        # PURE: terminal grid + ANSI/VT state machine
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── grid.rs               # Grid (row/column cell storage)
│   │       ├── cell.rs               # Cell (character + style)
│   │       ├── row.rs                # Row (line storage, wrapping)
│   │       ├── parser.rs             # ANSI/VT escape sequence parser
│   │       ├── hyperlinks.rs         # Hyperlink tracking (OSC 8)
│   │       └── scrollback.rs         # Scrollback buffer management
│   │
│   │  ── LAYER 1: PROTOCOL (PURE except for byte handling) ──
│   │
│   ├── tf-proto/                       # PURE*: tmux binary protocol codec
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── frame.rs              # ImsgHdr, ImsgFrame (from tmux-protocol.h)
│   │       ├── codec.rs              # ImsgCodec (tokio_util::Decoder/Encoder)
│   │       ├── msg.rs                # MsgType enum (MSG_COMMAND, MSG_IDENTIFY_*, etc.)
│   │       ├── identify.rs           # IdentifyBurstBuilder
│   │       ├── payload.rs            # Payload parsing helpers
│   │       ├── error.rs              # ProtocolError
│   │       └── fixture.rs            # Test fixtures from real tmux captures
│   │
│   ├── tf-control/                     # PURE: control-mode line parser
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── notification.rs       # ControlNotification enum (from control-notify.c)
│   │       ├── parser.rs             # %%notification line parser
│   │       └── backpressure.rs       # Pause/continue/watermark model
│   │
│   │  ── LAYER 2: RUNTIME (async, IO, unsafe quarantine) ──
│   │
│   ├── tf-os/                          # UNSAFE: OS primitives quarantine
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── pty.rs                # PTY creation + management
│   │       ├── scm_rights.rs         # SCM_RIGHTS FD passing
│   │       ├── socket.rs             # Unix socket creation + permissions
│   │       ├── signal.rs             # Signal handling
│   │       └── fd.rs                 # FD lifecycle + close-on-drop
│   │
│   ├── tf-pty/                         # PTY trait abstraction
│   │   └── src/
│   │       ├── lib.rs                # PtyBackend trait
│   │       ├── os.rs                 # Real OS PTY backend
│   │       ├── portable.rs           # portable-pty backend
│   │       └── fake.rs               # Fake PTY for testing
│   │
│   ├── tf-refresh/                     # Refresh planner + state store
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── planner.rs            # RefreshPlanner (debounce + scope coalescing)
│   │       ├── state.rs              # StateHandle<T> (ArcSwap-backed lock-free store)
│   │       ├── event.rs              # RefreshEvent parsing
│   │       └── subscription.rs       # RefreshHint + broadcast
│   │
│   ├── tf-client/                      # tmux protocol client
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── connection.rs         # Protocol connection management
│   │       ├── control.rs            # Control-mode session management
│   │       ├── sync.rs               # SyncClient (blocking wrapper)
│   │       └── socket.rs             # Socket discovery + resolution
│   │
│   ├── tf-backend/                     # Backend trait + implementations
│   │   └── src/
│   │       ├── lib.rs                # MuxBackend enum (Local | Tmux)
│   │       ├── local.rs              # In-process backend (uses tf-core directly)
│   │       ├── tmux.rs               # Real tmux backend (protocol + control + binary)
│   │       ├── connection.rs         # ConnectionManager (lifecycle)
│   │       └── view_capture.rs       # State snapshot via list-* commands
│   │
│   ├── tf-server/                      # tmux-compatible server binary
│   │   └── src/
│   │       ├── main.rs
│   │       ├── state.rs              # Runtime state wrapping tf-core
│   │       ├── commands.rs           # Command dispatch
│   │       ├── framing.rs            # Protocol frame handler
│   │       ├── config.rs             # Format context + option semantics
│   │       ├── keybinding.rs         # Key dispatch
│   │       ├── status.rs             # Status line rendering
│   │       ├── borders.rs            # Pane border rendering
│   │       ├── copy_mode.rs          # Copy mode runtime
│   │       ├── layout.rs             # Layout algorithms
│   │       ├── attached.rs           # Attached client loop
│   │       ├── pty_reader.rs         # PTY output reader
│   │       └── test_backend.rs       # Configurable backend for testing
│   │
│   │  ── LAYER 3: FACADE (unified API surface) ──
│   │
│   ├── tf-api/                         # Primary public facade crate
│   │   └── src/
│   │       ├── lib.rs                # Re-exports: ManagedMux, View, query::*, etc.
│   │       ├── managed.rs            # ManagedMux (SocketActor) + RefreshDriver
│   │       ├── lifecycle.rs          # LifecycleManager (server start/stop)
│   │       ├── targets.rs            # Target resolution (session/window/pane)
│   │       └── orm.rs                # ORM-like API: server.sessions().filter().get_one()
│   │
│   │  ── LAYER 4: CRDT (collaborative features) ──
│   │
│   ├── tf-crdt/                        # CRDT state synchronization
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── document.rs           # CrdtDocument (graph state as CRDT)
│   │       ├── operations.rs         # CrdtOp enum (insert/delete/update/move)
│   │       ├── clock.rs              # HybridLogicalClock
│   │       ├── merge.rs              # Merge resolution strategies
│   │       ├── transport.rs          # CrdtTransport trait (for network layer)
│   │       └── snapshot.rs           # Snapshot + delta encoding
│   │
│   │  ── LAYER 5: TELEMETRY ──
│   │
│   ├── tf-telemetry/                   # Tracing + OTEL infrastructure
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── tracing.rs            # tracing subscriber setup
│   │       ├── otel.rs               # OpenTelemetry integration
│   │       └── config.rs             # Telemetry config (env vars, TOML)
│   │
│   │  ── TEST SUPPORT ──
│   │
│   ├── tf-test-support/                # Shared test utilities
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── harness.rs            # Test harness (temp sockets, isolated env)
│   │       ├── pty_strict.rs         # PTY leak detection
│   │       └── assertions.rs         # Domain-specific assertions
│   │
│   └── tf-test-framework/              # Test framework for terminal apps
│       └── src/
│           ├── lib.rs
│           ├── session_builder.rs    # Declarative test session setup
│           ├── expectations.rs       # Output matching (exact, regex, contains)
│           ├── timing.rs             # Wait conditions + timeouts
│           ├── capture.rs            # Pane capture utilities
│           └── fixtures.rs           # Fixture recording + replay
│
├── tools/
│   ├── tf-tui/                         # Ratatui-based TUI client
│   ├── tf-sniff/                       # Protocol sniffer/proxy
│   ├── tf-doctor/                      # Diagnostic tool
│   ├── format-audit/                   # tmux format.c alignment checker
│   ├── command-audit/                  # tmux command coverage tracker
│   └── regress-audit/                  # tmux regress test alignment
│
├── bindings/
│   ├── python/                         # PyO3 + maturin
│   │   ├── Cargo.toml
│   │   ├── pyproject.toml
│   │   └── src/
│   │       ├── lib.rs                # #[pymodule] termforge
│   │       ├── server.rs             # PyServer wrapping ManagedMux
│   │       ├── session.rs            # PySession
│   │       ├── window.rs             # PyWindow
│   │       ├── pane.rs               # PyPane
│   │       ├── query.rs              # PyQueryList + Django-style lookups
│   │       └── exceptions.rs         # ObjectDoesNotExist, MultipleObjectsReturned
│   │
│   ├── node/                           # Neon bindings
│   │   ├── Cargo.toml
│   │   ├── package.json
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── server.rs
│   │       ├── session.rs
│   │       ├── window.rs
│   │       ├── pane.rs
│   │       └── query.rs
│   │
│   └── cxx/                            # cxx bridge
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs                # #[cxx::bridge]
│           └── wrapper.hpp           # Thin C++ adapter
│
└── tests/
    ├── integration/                    # Cross-crate integration tests
    ├── parity/                         # tmux behavioral parity tests
    └── e2e/                            # End-to-end real-tmux tests
```

### Purity Classification Table

| Crate | Purity | Async | Unsafe | IO | Dependencies |
|-------|--------|-------|--------|----|--------------|
| `tf-types` | PURE | No | No | No | None (maybe `bitflags`) |
| `tf-query` | PURE | No | No | No | `regex` |
| `tf-config` | PURE | No | No | No | `nom` |
| `tf-core` | PURE | No | No | No | `slotmap`, `indexmap`, `smallvec`, `tf-types`, `tf-query`, `tf-config` |
| `tf-view` | PURE | No | No | No | `tf-core`, `tf-query` |
| `tf-grid` | PURE | No | No | No | `tf-types` |
| `tf-proto` | PURE* | No | No | No | `bytes`, `tokio-util` (codec trait only), `nom` |
| `tf-control` | PURE | No | No | No | `nom` |
| `tf-crdt` | PURE | No | No | No | `tf-types`, `tf-core` |
| `tf-os` | IMPURE | No | **YES** | YES | `nix`, `libc` |
| `tf-pty` | IMPURE | No | Via tf-os | YES | `tf-os`, `portable-pty` |
| `tf-refresh` | IMPURE | Yes | No | No | `tokio`, `tf-view` |
| `tf-client` | IMPURE | Yes | No | YES | `tokio`, `tf-proto`, `tf-os` |
| `tf-backend` | IMPURE | Yes | No | YES | `tf-client`, `tf-core`, `tf-view` |
| `tf-server` | IMPURE | Yes | Via tf-os | YES | `tf-core`, `tf-proto`, `tf-os`, `tf-pty` |
| `tf-api` | FACADE | Yes | No | No | `tf-backend`, `tf-view`, `tf-refresh` |
| `tf-telemetry` | IMPURE | No | No | YES | `tracing`, `opentelemetry` |

---

## 3. Core State Model

### Authoritative Graph (from vibe-tmux `ServerGraph` in `/home/d/work/rust/vibe-tmux/crates/mux-core/src/graph.rs`)

```rust
// tf-core/src/graph.rs
use slotmap::SlotMap;

pub struct ServerGraph {
    sessions: SlotMap<SessionId, Session>,
    windows: SlotMap<WindowId, Window>,
    panes: SlotMap<PaneId, Pane>,
    clients: SlotMap<ClientId, Client>,
    jobs: SlotMap<JobId, Job>,
    buffers: SlotMap<BufferId, PasteBuffer>,  // NEW: paste buffers
    hooks: HookTable,                          // NEW: hook storage
    key_tables: KeyTableSet,                   // NEW: key bindings
    options: OptionsStore,                     // NEW: hierarchical options
}
```

**Key difference from vibe-tmux:** Move `buffers`, `hooks`, `key_tables`, and `options` INTO the graph as first-class entities rather than handling them in `mux-server`. This keeps the pure core as the single source of truth, simplifying CRDT synchronization and testing.

### Entity Model (aligned with tmux `tmux.h` struct hierarchy at `/home/d/study/c/tmux/tmux.h`)

```rust
// tf-types/src/id.rs
slotmap::new_key_type! {
    pub struct SessionId;
    pub struct WindowId;
    pub struct PaneId;
    pub struct ClientId;
    pub struct JobId;
    pub struct BufferId;
}

// tf-core/src/session.rs (matches tmux `struct session`)
pub struct Session {
    pub name: String,
    pub windows: Vec<WindowId>,       // ordered (winlink list)
    pub active_window: Option<WindowId>,
    pub last_window: Option<WindowId>,
    pub created: Option<i64>,
    pub destroying: bool,
    pub group: Option<String>,        // session groups
    pub options: OptionScope,
}

// tf-core/src/pane.rs (matches tmux `struct window_pane`)
pub struct Pane {
    pub size: PaneSize,
    pub bounds: Option<Rect>,
    pub grid: Option<PaneId>,         // Links to tf-grid grid (kept external)
    pub pid: Option<u32>,
    pub tty: Option<String>,
    pub start_command: String,
    pub start_path: String,
    pub exited: bool,
    pub exit_status: Option<i32>,
    pub mode: PaneMode,               // Normal, Copy, View
}
```

### Event/Effect Engine (from vibe-tmux pattern in `/home/d/work/rust/vibe-tmux/crates/mux-core/src/engine.rs`)

```rust
// tf-core/src/engine.rs
pub fn apply_event(graph: &mut ServerGraph, event: Event) -> ApplyOutcome {
    match event {
        Event::Command { name, args } => command::dispatch(graph, &name, &args),
        Event::CreateSession { name } => { /* ... */ },
        Event::Key { client_id, key } => key_dispatch(graph, client_id, key),
        // ... complete event taxonomy
    }
}

pub struct ApplyOutcome {
    pub effects: Vec<Effect>,
    pub created_session: Option<SessionId>,
    pub created_window: Option<WindowId>,
    pub created_pane: Option<PaneId>,
    pub notifications: Vec<Notification>,  // NEW: control-mode notifications to emit
    pub crdt_ops: Vec<CrdtOp>,            // NEW: CRDT operations for sync
}
```

---

## 4. Protocol Layer

### tmux Binary Protocol (from `/home/d/study/c/tmux/tmux-protocol.h`)

The protocol layer reproduces tmux's imsg framing exactly:

```rust
// tf-proto/src/frame.rs
pub struct ImsgHdr {
    pub msg_type: u32,     // enum msgtype from tmux-protocol.h
    pub peerid: u32,
    pub pid: u32,
    pub len: u32,          // includes header; high bit = IMSG_FD_FLAG
}

// tf-proto/src/msg.rs (matches tmux-protocol.h enum msgtype)
pub enum MsgType {
    Version = 12,
    IdentifyFlags = 100,
    IdentifyTerm = 101,
    IdentifyTtyname = 102,
    // ... complete enumeration from tmux-protocol.h
    Command = 200,
    Detach = 201,
    // ... all 40+ message types
    ReadOpen = 300,
    Read = 301,
    // ... stream messages
}
```

### Control-mode Protocol (from `/home/d/study/c/tmux/control-notify.c`)

```rust
// tf-control/src/notification.rs
pub enum ControlNotification {
    SessionsChanged,
    SessionRenamed { session_id: u32, name: String },
    SessionWindowChanged { session_id: u32, window_id: u32 },
    WindowAdd { window_id: u32 },
    WindowClose { window_id: u32 },
    WindowRenamed { window_id: u32, name: String },
    WindowPaneChanged { window_id: u32, pane_id: u32 },
    LayoutChange { window_id: u32, layout: String, visible_layout: String, flags: String },
    PaneModeChanged { pane_id: u32 },
    ClientSessionChanged { client: String, session_id: u32, name: String },
    ClientDetached { client: String },
    PasteBufferChanged { name: String },
    PasteBufferDeleted { name: String },
    // Unlinked variants
    UnlinkedWindowAdd { window_id: u32 },
    UnlinkedWindowClose { window_id: u32 },
    UnlinkedWindowRenamed { window_id: u32, name: String },
}
```

---

## 5. Runtime Architecture

### Actor Model (inspired by Zellij's thread bus from `/home/d/study/rust/zellij/zellij-server/src/thread_bus.rs`)

```
                                    ┌─────────────┐
                                    │  tf-server   │
                                    │  (main loop) │
                                    └──────┬───────┘
                                           │
              ┌────────────────────────────┼────────────────────────────┐
              │                            │                            │
     ┌────────▼────────┐         ┌────────▼────────┐         ┌───────▼────────┐
     │  PTY Actor       │         │  Screen Actor   │         │  Plugin Actor  │
     │  (read/write     │         │  (layout, grid, │         │  (WASM runtime)│
     │   pty streams)   │         │   rendering)    │         │                │
     └─────────────────┘         └─────────────────┘         └────────────────┘
              │                            │                            │
              └────────────────────────────┼────────────────────────────┘
                                           │
                                    ┌──────▼───────┐
                                    │ SocketActor   │  (per-socket, from vibe-tmux ManagedMux)
                                    │ event channel │
                                    │ fan-in recv() │
                                    └──────────────┘
```

The key pattern from Zellij (`ThreadSenders` in `thread_bus.rs`) is typed instruction enums per actor:

```rust
// tf-server instruction enums (Zellij pattern)
pub enum PtyInstruction {
    SpawnTerminal { pane_id: PaneId, command: Vec<String>, cwd: Option<String> },
    WriteToPty { pane_id: PaneId, data: Vec<u8> },
    CloseTerminal { pane_id: PaneId },
}

pub enum ScreenInstruction {
    Render,
    Resize { cols: u16, rows: u16 },
    NewPane { pane_id: PaneId },
    ClosePane { pane_id: PaneId },
}
```

### SocketActor (from vibe-tmux `/home/d/work/rust/vibe-tmux/crates/mux-api/src/managed.rs`)

```rust
// tf-api/src/managed.rs
pub struct ManagedMux {
    backend: MuxBackend,
    state: StateHandle<Option<View>>,     // ArcSwap lock-free store
    updates: broadcast::Sender<RefreshHint>,
    event_tx: Sender<SocketEvent>,
}

enum SocketEvent {
    Wake,
    Control(ControlNotification),
    Refresh(RefreshSet),
    CrdtSync(CrdtOp),                    // NEW: CRDT operation from remote
    Shutdown,
}
```

---

## 6. API Layers

### Layer 0: Core API (pure, deterministic)

```rust
use tf_core::{ServerGraph, ServerFacade, Event, Effect, ApplyOutcome};

let mut facade = ServerFacade::new();
let outcome = facade.exec("new-session -s demo")?;
let outcome = facade.exec("split-window -h")?;
let state = facade.snapshot();  // GraphState
```

### Layer 1: View API (pure traversal, inspired by libtmux `QueryList` from `/home/d/work/python/libtmux/src/libtmux/_internal/query_list.py`)

```rust
use tf_view::{View, ViewIndex};
use tf_view::query::{session, window, pane};

let view = View::from_state(state);
let s = view.sessions().filter(session::name().icontains("demo")).get_one()?;
let w = s.windows().filter(window::name().startswith("editor")).get_one()?;
let p = w.panes().filter(pane::active().is_true()).get_one()?;
```

### Layer 2: Managed API (stream-first, push updates)

```rust
use tf_api::{ManagedMux, RefreshDriver, RefreshHint};

let managed = ManagedMux::tmux(Some("/tmp/tmux-1000/default"), None, None)?;
let handle = managed.handle();
let mut rx = handle.subscribe();

// Stream-first: block on channel, wake on new data
while let Ok(hint) = rx.recv().await {
    let view = handle.view().expect("view available after hint");
    // React to changes
}
```

### Layer 3: ORM-like API (the "libtmux for Rust")

```rust
use tf_api::orm::{Server, Session, Window, Pane};

let server = Server::connect("/tmp/tmux-1000/default")?;

// Navigate like libtmux: server.sessions -> session.windows -> window.panes
for session in server.sessions().iter() {
    println!("Session: {}", session.name());
    for window in session.windows().iter() {
        println!("  Window: {}", window.name());
        for pane in window.panes().iter() {
            println!("    Pane: {} ({})", pane.id(), pane.current_command());
        }
    }
}

// Django-style filtering (matches libtmux.QueryList)
let session = server.sessions().get(name__icontains="dev")?;
let window = session.windows().get(name__startswith="edit")?;

// Command execution
server.cmd("split-window -h")?;
session.cmd("rename-session new-name")?;
window.cmd("select-pane -t 0")?;
pane.send_keys("echo hello", enter=true)?;
```

### Layer 4: Test Framework API

```rust
use tf_test_framework::{TestSession, expect};

#[test]
fn test_split_window() {
    let session = TestSession::builder()
        .window("main", |w| {
            w.pane("editor", "vim file.rs");
            w.pane("terminal", "bash");
        })
        .build()?;

    session.cmd("split-window -h")?;

    expect!(session.active_window().panes().len() == 3);
    expect!(session.active_pane().capture_text()).contains("$");
}
```

---

## 7. Bindings Architecture

### Python Bindings (PyO3, matching libtmux `Server`/`Session`/`Window`/`Pane` from `/home/d/work/python/libtmux/src/libtmux/server.py`)

```python
# Python API mirrors libtmux exactly
from termforge import Server

server = Server(socket_name="default")

# QueryList with Django-style lookups (from libtmux._internal.query_list)
session = server.sessions.get(session_name="demo")
window = session.windows.filter(window_name__startswith="edit")[0]
pane = window.panes.get(pane_active="1")

# Command execution
server.cmd("new-window", "-n", "logs")
pane.send_keys("tail -f /var/log/syslog")

# Context manager for test isolation (like libtmux)
with Server(socket_name_factory=lambda: f"test-{uuid4()}") as server:
    session = server.new_session(session_name="test")
    # Server killed on exit
```

### Node Bindings (Neon)

```javascript
const { ManagedMux } = require('termforge');

const server = new ManagedMux();
const sessions = server.sessions();
const session = sessions.filter({ name__icontains: 'demo' }).getOne();
const windows = session.windows();
```

### C++ Bindings (cxx, from vibe-tmux pattern)

```cpp
#include "termforge.hpp"

auto server = termforge::ManagedMux::connect("/tmp/tmux-1000/default");
auto view = server.view();
auto session = view.sessions()
    .filter(termforge::query::session::name().icontains("demo"))
    .get_one();
```

### Binding Implementation Strategy

Each binding follows the same internal pattern (established in vibe-tmux's `/home/d/work/rust/vibe-tmux/bindings/python/src/lib.rs` and `/home/d/work/rust/vibe-tmux/bindings/node/src/lib.rs`):

1. `ServerState` struct holds `Arc<Mutex<ManagedMux>>` + optional `RefreshDriver`
2. `with_lock()` method for synchronized access
3. OTEL span propagation across binding boundary
4. Query execution happens in Rust; bindings format criteria and call Rust query engine
5. View reads are lock-free via `StateHandle` (no blocking on readers)

---

## 8. Test Framework Design

### Layers (from vibe-tmux AGENTS.md testing policy at `/home/d/work/rust/vibe-tmux/AGENTS.md` lines 157-186)

1. **Pure core tests** (tf-core): deterministic Event -> Effect assertions, no IO mocking.
   Pattern from vibe-tmux: `crates/mux-core/src/engine.rs` tests.

2. **Protocol tests** (tf-proto): fixture-based snapshot tests + proptest chunking.
   Pattern from vibe-tmux: `crates/mux-proto/src/fixture.rs` + `insta` snapshots.

3. **Integration tests**: hermetic with temp dirs/sockets.
   Pattern from vibe-tmux: never touch real tmux socket.

4. **Parity tests**: our server vs real tmux with identical scripts.
   Pattern from vibe-tmux: `just mux-parity-e2e` (76/95 passing per `/home/d/work/rust/vibe-tmux/notes/plan.md`).

5. **Test framework** (tf-test-framework): reusable by external consumers.

```rust
// tf-test-framework/src/session_builder.rs
pub struct TestSession {
    server: ManagedMux,
    socket_path: PathBuf,
    cleanup: CleanupGuard,
}

impl TestSession {
    pub fn builder() -> TestSessionBuilder { /* ... */ }
    pub fn cmd(&self, command: &str) -> Result<CommandOutput> { /* ... */ }
    pub fn active_pane(&self) -> PaneHandle { /* ... */ }
    pub fn wait_for(&self, condition: impl Fn(&View) -> bool, timeout: Duration) -> Result<()> { /* ... */ }
    pub fn capture(&self, pane: PaneId) -> Vec<String> { /* ... */ }
}

// Declarative setup
pub struct TestSessionBuilder {
    windows: Vec<WindowSpec>,
    env: HashMap<String, String>,
    config: Option<String>,
}

impl TestSessionBuilder {
    pub fn window(mut self, name: &str, setup: impl FnOnce(&mut WindowSpec)) -> Self { /* ... */ }
    pub fn env(mut self, key: &str, value: &str) -> Self { /* ... */ }
    pub fn build(self) -> Result<TestSession> { /* ... */ }
}
```

### Regress Test Alignment (from vibe-tmux audit tools)

```
tools/
├── command-audit/   # Compares our command set against tmux binary output
├── format-audit/    # Compares format variables against tmux format.c
└── regress-audit/   # Maps tmux regress scripts to our parity tests
```

Each audit tool reads the tmux source at `~/study/c/tmux` and produces machine-readable output.

---

## 9. CRDT Design

### Architecture

The CRDT layer sits atop the pure core and treats `ServerGraph` mutations as a log of operations:

```rust
// tf-crdt/src/document.rs
pub struct CrdtDocument {
    local_id: ReplicaId,
    clock: HybridLogicalClock,
    operations: Vec<TimestampedOp>,
    graph: ServerGraph,                    // Local materialized view
}

// tf-crdt/src/operations.rs
pub enum CrdtOp {
    CreateSession { id: SessionId, name: String },
    RenameSession { id: SessionId, new_name: String },
    CreateWindow { session_id: SessionId, window_id: WindowId, name: String },
    CreatePane { window_id: WindowId, pane_id: PaneId, spec: PaneSpec },
    SelectWindow { session_id: SessionId, window_id: WindowId },
    SelectPane { window_id: WindowId, pane_id: PaneId },
    MoveWindow { from_session: SessionId, to_session: SessionId, window_id: WindowId },
    SetOption { scope: OptionScope, key: String, value: OptionValue },
    PaneOutput { pane_id: PaneId, data: Vec<u8>, offset: u64 },  // For shared panes
    // Tombstones for deletes
    DeleteSession { id: SessionId },
    DeleteWindow { id: WindowId },
    DeletePane { id: PaneId },
}

// tf-crdt/src/clock.rs
pub struct HybridLogicalClock {
    wall_time: u64,      // milliseconds
    logical: u32,        // tiebreaker
    replica_id: ReplicaId,
}
```

### Integration Point

The `ApplyOutcome` returned by `tf-core`'s engine includes a `crdt_ops` field. The runtime decides whether to broadcast these ops:

```rust
// In tf-server or tf-api
let outcome = apply_event(&mut graph, event);
for effect in &outcome.effects {
    execute_effect(effect);
}
if let Some(crdt) = crdt_layer.as_mut() {
    for op in &outcome.crdt_ops {
        crdt.broadcast(op).await;
    }
}
```

### Transport Layer

```rust
// tf-crdt/src/transport.rs
pub trait CrdtTransport: Send + Sync {
    fn broadcast(&self, op: &CrdtOp) -> Result<()>;
    fn receive(&self) -> Pin<Box<dyn Stream<Item = CrdtOp> + Send>>;
    fn sync_state(&self, peer: ReplicaId) -> Result<Vec<CrdtOp>>;
}
```

Implementations would include:
- **LocalTransport**: in-process (for testing)
- **UnixSocketTransport**: local machine sync
- **WebSocketTransport**: remote collaboration
- **GrpcTransport**: server-to-server federation

---

## 10. Plugin/Extension System

### Architecture (inspired by Zellij's WASM plugin system from `/home/d/study/rust/zellij/CLAUDE.md`)

```rust
// tf-core plugin trait (pure, no IO)
pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    fn on_event(&mut self, event: &Event, graph: &ServerGraph) -> Vec<PluginEffect>;
    fn on_render(&self, ctx: &RenderContext) -> Option<PluginOutput>;
}

pub enum PluginEffect {
    EmitEvent(Event),          // Feed back into the engine
    SetOption(String, String), // Modify options
    SendKeys(PaneId, Vec<u8>), // Write to a pane
    Notify(String),            // User notification
}
```

### Extension Points

1. **Command extensions**: Register new commands via `CommandRegistry`
2. **Format extensions**: Register new format variables
3. **Hook extensions**: Subscribe to hooks (matching tmux `set-hook`)
4. **Layout extensions**: Custom layout algorithms
5. **Backend extensions**: Custom backend implementations (SSH tunneled, cloud, etc.)

---

## 11. Files to Create (Complete Listing)

### Phase 1: Foundation (tf-types, tf-query, tf-config)

| File | Purpose |
|------|---------|
| `Cargo.toml` | Workspace root (34 members) |
| `AGENTS.md` | Ported from vibe-tmux AGENTS.md with termforge adjustments |
| `ARCHITECTURE.md` | Living doc (this plan in condensed form) |
| `crates/tf-types/Cargo.toml` | Pure types crate |
| `crates/tf-types/src/lib.rs` | Re-exports |
| `crates/tf-types/src/id.rs` | `SessionId`, `WindowId`, `PaneId`, `ClientId`, `JobId`, `BufferId` |
| `crates/tf-types/src/geometry.rs` | `Rect`, `Size`, `Position`, `SplitDirection` |
| `crates/tf-types/src/key.rs` | `KeyCode`, `KeyModifiers` (from tmux KEYC_* in tmux.h) |
| `crates/tf-types/src/style.rs` | `Color`, `Style`, `Attributes` |
| `crates/tf-types/src/error.rs` | `CoreError` |
| `crates/tf-query/Cargo.toml` | Pure query crate |
| `crates/tf-query/src/lib.rs` | QueryList, QuerySpec, QueryOp, Queryable derive |
| `crates/tf-query/src/ops.rs` | 12 query operators (from vibe-tmux mux-query) |
| `crates/tf-query/src/value.rs` | QueryValue variants |
| `crates/tf-query/src/spec.rs` | QuerySpec, QueryTerm, path parsing |
| `crates/tf-query/src/list.rs` | `QueryList<T>` with `.filter()`, `.get()`, `.get_one()` |
| `crates/tf-config/Cargo.toml` | Pure config crate |
| `crates/tf-config/src/lib.rs` | Config AST, lexer, parser |
| `crates/tf-config/src/ast.rs` | Config AST (from vibe-tmux mux-conf) |
| `crates/tf-config/src/lexer.rs` | Tokenizer |
| `crates/tf-config/src/parser.rs` | tmux.conf parser |
| `crates/tf-config/src/options.rs` | Option table + defaults |

### Phase 2: Core State Machine (tf-core, tf-view, tf-grid)

| File | Purpose |
|------|---------|
| `crates/tf-core/Cargo.toml` | Pure core crate |
| `crates/tf-core/src/lib.rs` | Re-exports (matches vibe-tmux mux-core/src/lib.rs pattern) |
| `crates/tf-core/src/graph.rs` | `ServerGraph` with all entity stores |
| `crates/tf-core/src/session.rs` | Session entity |
| `crates/tf-core/src/window.rs` | Window entity |
| `crates/tf-core/src/pane.rs` | Pane entity + PaneSize |
| `crates/tf-core/src/client.rs` | Client entity |
| `crates/tf-core/src/job.rs` | Job entity |
| `crates/tf-core/src/buffer.rs` | PasteBuffer entity (NEW) |
| `crates/tf-core/src/hook.rs` | HookTable + dispatch |
| `crates/tf-core/src/key_table.rs` | KeyTableSet + bindings |
| `crates/tf-core/src/event.rs` | Event enum (all input intents) |
| `crates/tf-core/src/effect.rs` | Effect enum (all runtime side effects) |
| `crates/tf-core/src/engine.rs` | `apply_event` function |
| `crates/tf-core/src/facade.rs` | `ServerFacade` |
| `crates/tf-core/src/handle.rs` | `ServerHandle` (read-only) |
| `crates/tf-core/src/context.rs` | `ClientContext` |
| `crates/tf-core/src/state.rs` | GraphState, SessionState, etc. |
| `crates/tf-core/src/format.rs` | Format expansion engine |
| `crates/tf-core/src/layout.rs` | Layout tree + algorithms |
| `crates/tf-core/src/copy_mode.rs` | Copy mode state machine |
| `crates/tf-core/src/command/mod.rs` | Command dispatch registry |
| `crates/tf-core/src/command/new_session.rs` | (one per command, ~80 files) |
| `crates/tf-view/Cargo.toml` | Pure view crate |
| `crates/tf-view/src/lib.rs` | View, ViewIndex, ViewMeta |
| `crates/tf-view/src/view.rs` | View construction + traversal |
| `crates/tf-view/src/view_id.rs` | ViewId (Local \| Tmux) |
| `crates/tf-view/src/refs.rs` | SessionRef, WindowRef, PaneRef |
| `crates/tf-view/src/query.rs` | Typed query field selectors |
| `crates/tf-view/src/diff.rs` | View diffing |
| `crates/tf-grid/Cargo.toml` | Pure grid crate |
| `crates/tf-grid/src/lib.rs` | Grid, Cell, Row |
| `crates/tf-grid/src/grid.rs` | Grid storage |
| `crates/tf-grid/src/cell.rs` | Cell (char + style) |
| `crates/tf-grid/src/row.rs` | Row (line wrapping) |
| `crates/tf-grid/src/parser.rs` | ANSI/VT parser |
| `crates/tf-grid/src/hyperlinks.rs` | OSC 8 hyperlinks |
| `crates/tf-grid/src/scrollback.rs` | Scrollback management |

### Phase 3-7: Protocol, Runtime, API, CRDT, Bindings, Tools

(Each phase adds ~15-40 files following the structure above. Total estimated files: ~250 source files.)

---

## 12. Implementation Sequence

### Phase 1: Foundation Types (Week 1-2)
**Dependencies:** None
**Acceptance criteria:**
- `tf-types` compiles with `#![forbid(unsafe_code)]`
- `tf-query` passes port of all vibe-tmux `mux-query` tests
- `tf-config` parses tmux.conf syntax from `~/study/c/tmux/regress/*.conf`

### Phase 2: Core State Machine (Week 3-6)
**Dependencies:** Phase 1
**Acceptance criteria:**
- `tf-core` passes all vibe-tmux `mux-core` tests (ported)
- Event -> Effect engine covers all tmux commands in vibe-tmux's command-audit
- `ServerGraph` invariants validated by proptest
- `#![forbid(unsafe_code)]` on tf-core

### Phase 3: Protocol + Control (Week 7-9)
**Dependencies:** Phase 1
**Acceptance criteria:**
- `tf-proto` decodes all vibe-tmux fixtures
- `tf-proto` passes proptest chunking robustness (random segmentation)
- `tf-control` parses all notifications from `control-notify.c`
- Snapshot tests via `insta`

### Phase 4: Runtime + Server (Week 10-14)
**Dependencies:** Phase 2, 3
**Acceptance criteria:**
- `tf-server` accepts connections from real `tmux` client
- `tf-client` connects to real `tmux` server
- Parity tests: `regress-audit` tracks 33+ tmux regress scripts
- PTY telemetry with strict mode (`MUX_PTY_STRICT=1`)

### Phase 5: API Facade + ORM (Week 15-17)
**Dependencies:** Phase 4
**Acceptance criteria:**
- `tf-api` `ManagedMux` provides lock-free view reads with <20ms latency
- ORM API (`server.sessions().filter().get_one()`) works for all entity types
- `just check-llm` equivalent passes

### Phase 6: CRDT Layer (Week 18-20)
**Dependencies:** Phase 2 (core only)
**Acceptance criteria:**
- CrdtDocument round-trips through merge/snapshot
- Two in-process replicas converge after concurrent mutations
- CRDT operations serializable via serde (for network transport)

### Phase 7: Bindings (Week 21-24)
**Dependencies:** Phase 5
**Acceptance criteria:**
- Python binding passes libtmux pytest smoke suite
- Node binding passes `just bindings-node-list-sessions` equivalent
- C++ binding compiles and links via cxx
- OTEL trace chain integrity across all bindings

### Phase 8: Test Framework (Week 25-26)
**Dependencies:** Phase 5
**Acceptance criteria:**
- `tf-test-framework` published as standalone crate
- Example tests for session/window/pane lifecycle
- Fixture recording + replay working

### Phase 9: TUI Client (Week 27-30)
**Dependencies:** Phase 5
**Acceptance criteria:**
- Matches vibe-tmux `mux-tui` feature set
- Event-driven (no polling for data)
- Ratatui snapshot tests passing

---

## 13. Architecture Decisions with Justification

### Decision 1: New multiplexer with tmux compatibility layer, not a tmux rebuild

**Justification:** tmux's C codebase (see `/home/d/study/c/tmux/tmux.h` -- 3000+ line header with global state) is deeply coupled. Rebuilding it line-by-line in Rust would inherit the architecture. vibe-tmux already demonstrates that a clean Rust architecture with a compatibility *layer* is the right approach (`ARCHITECTURE.md`: "vibe-tmux is a tmux-compatible peer... goal: full behavioral reproduction"). The new project extends this by making the core even more independent.

### Decision 2: Separate tf-grid from tf-core

**Justification:** vibe-tmux's `mux-core` does not contain grid/terminal emulation (it delegates to the server). Zellij (`zellij-server/src/panes/grid.rs`) has the grid tightly coupled to the server. By extracting tf-grid as a pure crate, we enable: (a) testing grid rendering without a server, (b) using the grid in non-tmux contexts (test framework, embedded terminals), (c) keeping tf-core focused on the session/window/pane graph.

### Decision 3: CRDT operations derived from core Event/Effect, not bolted on

**Justification:** If CRDT is a separate layer that observes `ApplyOutcome`, it gets operations for free. The core engine already returns `ApplyOutcome { effects, created_* }` (from `/home/d/work/rust/vibe-tmux/crates/mux-core/src/engine.rs`). Adding a `crdt_ops` field to `ApplyOutcome` keeps the CRDT integrated without polluting the pure core with networking.

### Decision 4: SlotMap for entity storage (matching vibe-tmux)

**Justification:** vibe-tmux uses `SlotMap<SessionId, Session>` (from `/home/d/work/rust/vibe-tmux/crates/mux-core/src/graph.rs`). SlotMap gives O(1) insert/remove/lookup with generational indices (preventing use-after-free bugs). This is superior to `HashMap<u64, _>` for an entity graph with frequent creation/destruction cycles.

### Decision 5: ArcSwap for lock-free view reads (matching vibe-tmux)

**Justification:** vibe-tmux's `StateHandle` uses ArcSwap for lock-free reads (from `ARCHITECTURE.md`: "Lock-free store reads via ArcSwap"). This is critical for the <20ms latency target -- UI/bindings should never block waiting for a write lock.

### Decision 6: One command per file in tf-core/src/command/

**Justification:** tmux has ~100 commands, each with distinct semantics. vibe-tmux puts commands in a single large `commands.rs` in mux-server (~2000+ lines). By splitting into individual files, we get: (a) isolated testing per command, (b) clear ownership for contributors, (c) easier command-audit alignment with tmux source.

### Decision 7: Stream-first, polling as fallback only

**Justification:** This is vibe-tmux's non-negotiable #0 (from AGENTS.md line 47): "Prefer typed streams over tick-based polling." Zellij validates this pattern with crossbeam `Select` channel fan-in. The <20ms latency target requires push semantics.

### Decision 8: Query execution in Rust, binding wrappers are thin

**Justification:** From vibe-tmux AGENTS.md line 68: "QuerySet execution: filtering and lookups run in Rust... JS/Python/C++/C2Y wrappers may merge criteria, but must not implement the matching logic." This ensures consistent behavior across all bindings and avoids reimplementing `QueryOp::IContains` in four languages.

---

## 14. Test Strategy

### Testing Pyramid

```
         /\
        /  \  E2E: real tmux <-> our server
       /    \  (50 parity tests, from vibe-tmux regress-audit)
      /------\
     /        \  Integration: hermetic, temp sockets
    /          \  (200+ tests, test harness)
   /------------\
  /              \  Unit: pure core Event -> Effect
 /                \  (500+ tests, proptest, insta snapshots)
/------------------\
```

### Test Categories and Patterns

**1. Pure core tests (tf-core)**
- Pattern: from vibe-tmux `crates/mux-core/src/engine.rs` tests
- Deterministic: no IO, no time, no randomness unless injected
- Assert on whole `ApplyOutcome` objects (from codex AGENTS.md: "Prefer asserting on whole objects")
- Proptest for graph invariants: "for any sequence of valid events, session_ids in window.session never reference a deleted session"

**2. Protocol snapshot tests (tf-proto)**
- Pattern: from vibe-tmux `crates/mux-proto/src/fixture.rs`
- Insta snapshots for decoded fixtures
- Proptest: random bytes into codec must not panic
- Chunking robustness: split valid frames at random boundaries

**3. Parity tests (against real tmux)**
- Pattern: from vibe-tmux `just mux-parity-e2e`
- Run identical script against our server AND real tmux
- Diff outputs
- `#[ignore]` by default (require `cargo test --ignored`)
- Currently 76/95 passing in vibe-tmux; target 90%+ in phase 4

**4. Binding tests**
- Python: pytest suite with isolated tmux servers (from libtmux `pytest_plugin.py`)
- Node: vitest suite (from vibe-tmux `bindings/node/` vitest setup)
- C++: Catch2 tests linked against cxx bridge

**5. Test framework tests** (meta-testing)
- The test framework itself tested via its own harness
- Fixture recording verified via replay

### Test Environment Safety (from vibe-tmux AGENTS.md Rule 0)
- NEVER touch the developer's real tmux socket
- All tests use `/tmp/termforge-test-*` sockets
- `TERMFORGE_GUARD_KILL_SERVER=1` enabled by default
- Socket path verified before any destructive command

---

## 15. Risks and Edge Cases

### Risk 1: tmux Protocol Fragility
**Problem:** tmux's imsg protocol (from `tmux-protocol.h`) has no stability guarantee. `PROTOCOL_VERSION 8` can change.
**Mitigation:** Protocol tests run against multiple tmux versions (3.3a, 3.4, 3.5, 3.6). Fixtures captured per version. Identify burst ordering validated against real captures (from vibe-tmux: "tmux `client_send_identify` actually emits two `MSG_IDENTIFY_LONGFLAGS` frames").

### Risk 2: SCM_RIGHTS FD Leaks
**Problem:** FD passing via Unix domain sockets can leak if not handled precisely.
**Mitigation:** From vibe-tmux AGENTS.md: "bind the FD(s) to the correct protocol frame, close on drop (no leaks), validate expected FD counts." `tf-os` tests use PTY strict mode with leak detection.

### Risk 3: CRDT Conflict Resolution in Layout
**Problem:** Two users simultaneously resizing panes creates conflicting layouts.
**Mitigation:** Layout CRDT uses last-writer-wins for geometry (wall clock + replica ID tiebreaker). Session/window/pane *existence* uses add-wins semantics (harder to accidentally delete). Layout is re-computed from the authoritative pane set after merge.

### Risk 4: WSL Socket Compatibility
**Problem:** WSL's 9p filesystem causes socket bind failures (documented in vibe-tmux notes).
**Mitigation:** From vibe-tmux AGENTS.md: "socket paths must be on the Linux filesystem (avoid `/mnt/*`), especially on WSL." Abstract socket escape hatch (`MUX_TEST_ABSTRACT_SOCKET=1`) for testing.

### Risk 5: Binding API Drift
**Problem:** Python, Node, and C++ bindings can diverge from each other and from the Rust API.
**Mitigation:** Audit tools compare binding surface against `tf-api` public types. Query ops documented in a shared `query_ops.txt` with guardrail tests (from vibe-tmux: "mux-query guardrail test enforces coverage").

### Risk 6: Grid/Terminal Emulation Correctness
**Problem:** ANSI/VT parsing is notoriously complex (thousands of edge cases).
**Mitigation:** Use established parser patterns from Zellij (`grid.rs`), test against vttest and tmux's own regress scripts. Keep tf-grid pure so it can be fuzz-tested independently.

### Risk 7: tokio Runtime Overhead in Sync Contexts
**Problem:** vibe-tmux discovered that creating full tokio runtimes per protocol command causes thread explosion (from `/home/d/work/rust/vibe-tmux/notes/research.md` line 67).
**Mitigation:** `tf-client` `SyncClient` uses `Builder::new_current_thread().enable_all()` exclusively. Batch snapshot operations share one runtime.

### Risk 8: Control-mode Backpressure
**Problem:** tmux pauses control clients at 8192 bytes (from `control-notify.c` analysis in vibe-tmux `notes/ideas.md`).
**Mitigation:** `tf-control` models pause/continue explicitly. `ControlDebouncer` coalesces notifications before forwarding.

---

## 16. DOs and DON'Ts

### DOs (20 rules)

1. **DO** keep `tf-core` pure: `#![forbid(unsafe_code)]`, no tokio, no IO, no time/rng without injection
2. **DO** use typed IDs (`SessionId`, `WindowId`, etc.) everywhere -- never raw integers across crate boundaries
3. **DO** validate protocol claims against tmux source or captured fixtures before implementing
4. **DO** use stream-first semantics (channels, async streams) for all real-time update paths
5. **DO** quarantine all `unsafe` in `tf-os` with `// SAFETY:` comments and focused tests
6. **DO** use `Result` everywhere in hot paths -- no `unwrap()`/`expect()` in protocol/server/PTY code
7. **DO** implement one command per file in `tf-core/src/command/` for isolation and testability
8. **DO** run query filtering in Rust -- bindings are thin wrappers that format criteria
9. **DO** use `ArcSwap`/lock-free reads for view store access from UI/bindings
10. **DO** expose CRDT operations as a derived product of `ApplyOutcome`, not a separate mutation path
11. **DO** use `insta` snapshot tests for protocol decoding, view rendering, and format expansion
12. **DO** use `proptest` for invariant checking on `ServerGraph` and protocol codec
13. **DO** maintain audit tools (`command-audit`, `format-audit`, `regress-audit`) against tmux source
14. **DO** use hermetic test sockets (`/tmp/termforge-test-*`) -- verify path before destructive commands
15. **DO** propagate OTEL trace context across all thread/binding/process boundaries
16. **DO** keep the binding API surface libtmux-shaped: `server.sessions -> session.windows -> window.panes`
17. **DO** document public API changes in `ARCHITECTURE.md` and `notes/plan.md`
18. **DO** use `just check-llm` equivalent before every commit
19. **DO** prefer inline format args (`format!("{value}")`) over positional
20. **DO** assert on whole objects in tests, not field-by-field

### DON'Ts (18 rules)

1. **DON'T** kill the developer's real tmux server (Rule 0 from vibe-tmux AGENTS.md)
2. **DON'T** add tokio/async to any PURE-classified crate
3. **DON'T** use `#[repr(packed)]` anywhere
4. **DON'T** cast bytes to struct for protocol parsing -- explicit field parsing only
5. **DON'T** use serde-based IPC for tmux compatibility surfaces
6. **DON'T** expose internal names (`Frame`, `Store`, `JSON`) in binding public APIs
7. **DON'T** implement query matching logic in binding languages (only in Rust)
8. **DON'T** use polling/sleep loops for steady-state UI/binding updates
9. **DON'T** reconnect/identify per tick -- one persistent connection per socket per backend
10. **DON'T** create new crates for incremental migration work -- add modules to existing crates
11. **DON'T** use `unwrap()` or `expect()` in protocol decode, server loop, or PTY loop
12. **DON'T** mutate process environment in tests -- use dependency injection
13. **DON'T** touch `/mnt/*` paths for socket creation on WSL
14. **DON'T** invent protocol message ordering or payload layouts -- everything must be tmux-source-backed
15. **DON'T** use `Runtime::new()` (multi-thread tokio) in synchronous client contexts
16. **DON'T** keep legacy/polling API paths alongside stream-first surfaces
17. **DON'T** add overlapping dependency stacks (one PTY strategy, one FD-passing strategy)
18. **DON'T** create documentation files proactively -- only when explicitly requested

---

## 17. AGENTS.md Rules (for the new project)

```markdown
# AGENTS.md (human + LLM guardrails)

## Rule 0: Never kill real tmux
- NEVER run `kill-server` against the developer's real tmux socket.
- Always use dedicated test sockets (`/tmp/termforge-test-*`).
- Keep the kill-server guard enabled (`TERMFORGE_GUARD_KILL_SERVER=1`).

## Mission
Layered terminal multiplexer core with perfect tmux compatibility, ORM-style APIs,
CRDT collaboration support, and multi-language bindings.

"Works" is not success unless a real tmux client/server agrees on the compatibility layer.

## Reference repos
- `~/study/c/tmux` — upstream tmux behavior/source (authoritative for protocol + semantics)
- `~/study/rust/zellij` — actor patterns, thread bus, plugin system
- `~/study/rust/tokio` — async runtime patterns
- `~/study/rust/ratatui` — TUI widget patterns, workspace structure
- `~/study/rust/codex` — AI tool architecture, coding conventions
- `~/work/python/libtmux` — ORM-like API shape, QueryList, pytest plugin
- `~/work/rust/vibe-tmux` — predecessor project (all lessons learned apply)

## Non-negotiables

### 0) Push-first responsiveness
- Typed streams over tick-based polling.
- Event loops block on channels and wake on new data.
- Latency target: <20ms for UI/binding propagation.

### 1) Purity boundary
`tf-core`, `tf-types`, `tf-query`, `tf-config`, `tf-view`, `tf-grid`,
`tf-proto`, `tf-control`, `tf-crdt` are PURE:
- no tokio, no async/.await
- no nix/libc
- no filesystem, sockets, PTYs
- no unsafe (except tf-proto may use bytes::Buf)
- no time/rng without explicit injection

### 2) Unsafe quarantine
Only `tf-os` may contain `unsafe`. Every block:
- minimal boundary
- documents invariants with `// SAFETY: ...`
- has a focused test

### 3) Protocol parsing
- Explicit field parsing (no cast bytes to struct)
- Validate lengths before reading
- Cap allocations
- Malformed frames = protocol violation = close connection cleanly

### 4) Source-of-truth
Protocol/behavior claims must be backed by tmux source or captured fixtures.
No invented message ordering, payload layouts, or flags.

### 5) No panics in hot paths
Use `Result`, close the client, log with context.
Deny: `unwrap_used`, `expect_used`, `panic` in Clippy.

### 6) Testing policy
- Core: deterministic Event -> Effect unit tests
- Protocol: fixture snapshots + proptest chunking
- Integration: hermetic, temp dirs/sockets
- Parity: our server vs real tmux with identical scripts
- Bindings: smoke tests per language

### 7) Dependency policy
- Minimal deps per crate
- No overlapping stacks
- No serde-based IPC for tmux surfaces

## Coding conventions (from codex-rs)
- Inline format args: `format!("{value}")` not `format!("{}", value)`
- Collapsible if statements
- Method references over closures
- Assert whole objects in tests
- Pretty assertions for diffs

## Git commit format
type(scope[detail]) concise description

why: Explanation.
what:
- Change 1
- Change 2
```

---

## 18. Implementation Phases with Acceptance Criteria (Detailed)

### Phase 1: Foundation Types (Weeks 1-2)

**Create:**
- `Cargo.toml` (workspace), `AGENTS.md`, `ARCHITECTURE.md`
- `crates/tf-types/` (all files)
- `crates/tf-query/` (all files)
- `crates/tf-config/` (all files)
- `crates/tf-test-support/` (basic harness)

**Acceptance:**
- [ ] `cargo test -p tf-types` passes (50+ unit tests for ID types, geometry, keys)
- [ ] `cargo test -p tf-query` passes (port all tests from vibe-tmux `mux-query`)
- [ ] `cargo test -p tf-config` parses 10+ tmux.conf files from `~/study/c/tmux/regress/`
- [ ] All three crates pass `#![forbid(unsafe_code)]`
- [ ] `cargo clippy --workspace` clean with workspace lints

### Phase 2: Core State Machine (Weeks 3-6)

**Create:**
- `crates/tf-core/` (all files including `command/` directory)
- `crates/tf-view/` (all files)

**Acceptance:**
- [ ] `ServerGraph` CRUD operations have 100% coverage
- [ ] `apply_event` covers all Event variants with Effect assertions
- [ ] `ServerFacade::exec()` parses and applies 40+ tmux commands
- [ ] `View` construction from `GraphState` round-trips correctly
- [ ] QueryList filtering works for sessions, windows, panes
- [ ] Proptest: "1000 random event sequences never panic or violate invariants"
- [ ] `format-audit` shows <10% gap against tmux `format.c` variables

### Phase 3: Protocol + Grid (Weeks 7-9)

**Create:**
- `crates/tf-proto/` (all files)
- `crates/tf-control/` (all files)
- `crates/tf-grid/` (all files)

**Acceptance:**
- [ ] `tf-proto` decodes all imsg types from tmux-protocol.h
- [ ] Identify burst builder matches tmux `client_send_identify` ordering
- [ ] Proptest: "random byte chunks through ImsgCodec never panic"
- [ ] `tf-control` parses all 14+ notification types from `control-notify.c`
- [ ] `tf-grid` renders basic ANSI sequences correctly (vttest subset)
- [ ] Insta snapshots for all protocol fixtures

### Phase 4: Runtime + Server (Weeks 10-14)

**Create:**
- `crates/tf-os/` (all files)
- `crates/tf-pty/` (all files)
- `crates/tf-refresh/` (all files)
- `crates/tf-client/` (all files)
- `crates/tf-backend/` (all files)
- `crates/tf-server/` (all files)
- `tools/tf-sniff/`
- `tools/tf-doctor/`
- `tools/command-audit/`
- `tools/format-audit/`
- `tools/regress-audit/`

**Acceptance:**
- [ ] `tf-server` binary starts and accepts connection from `tmux attach`
- [ ] `tf-client` connects to real `tmux` server and retrieves `list-sessions`
- [ ] PTY strict mode passes: no leaked PTYs after test suite
- [ ] Loopback test: `tf-client` <-> `tf-server` round-trip
- [ ] `regress-audit` tracks 30+ tmux regress scripts
- [ ] `command-audit` shows 80%+ command coverage

### Phase 5: API Facade + ORM (Weeks 15-17)

**Create:**
- `crates/tf-api/` (all files)
- `crates/tf-telemetry/` (all files)

**Acceptance:**
- [ ] `ManagedMux` provides lock-free view reads
- [ ] RefreshHint latency <20ms (measured with tracing)
- [ ] ORM API: `server.sessions().filter(name__icontains="demo").get_one()` works
- [ ] `just check-llm` equivalent passes
- [ ] OTEL trace chain verified (no orphan spans)

### Phase 6: CRDT Layer (Weeks 18-20)

**Create:**
- `crates/tf-crdt/` (all files)

**Acceptance:**
- [ ] `CrdtDocument` round-trips through serialize/deserialize
- [ ] Two replicas converge after 100 concurrent random mutations
- [ ] Snapshot + delta encoding produces compact representations
- [ ] No CRDT code in tf-core (clean separation)

### Phase 7: Bindings (Weeks 21-24)

**Create:**
- `bindings/python/` (all files)
- `bindings/node/` (all files)
- `bindings/cxx/` (all files)

**Acceptance:**
- [ ] Python: `import termforge; s = termforge.Server()` works
- [ ] Python: `server.sessions.filter(session_name__icontains="x")` returns correct results
- [ ] Python: libtmux pytest smoke suite passes (10+ tests)
- [ ] Node: `const s = new ManagedMux()` works
- [ ] C++: `auto s = termforge::ManagedMux::connect(path)` compiles and runs
- [ ] OTEL traces flow from bindings to server

### Phase 8: Test Framework (Weeks 25-26)

**Create:**
- `crates/tf-test-framework/` (all files)

**Acceptance:**
- [ ] `TestSession::builder().window("main").build()` creates isolated session
- [ ] `session.wait_for(|v| v.panes().len() == 2, Duration::from_secs(5))` works
- [ ] `session.capture(pane_id)` returns terminal content
- [ ] Fixture recording captures real tmux output for replay
- [ ] Published as standalone crate with documentation

### Phase 9: TUI Client (Weeks 27-30)

**Create:**
- `tools/tf-tui/` (all files)

**Acceptance:**
- [ ] Renders session/window/pane tree
- [ ] Event-driven main loop (no data polling)
- [ ] Ratatui snapshot tests for all views
- [ ] Keyboard navigation works
- [ ] Real tmux integration tests pass

---

## 19. Key Architectural Differences from vibe-tmux

| Aspect | vibe-tmux | termforge | Justification |
|--------|-----------|-----------|---------------|
| Grid/terminal emulation | In mux-server | Separate tf-grid crate (PURE) | Testable, reusable in test framework |
| Paste buffers | In mux-server | In tf-core ServerGraph | CRDT-compatible, testable |
| Hooks | In mux-server | In tf-core ServerGraph | CRDT-compatible, testable |
| Key tables | In mux-server | In tf-core ServerGraph | Deterministic dispatch testing |
| Command handlers | Single `commands.rs` | One file per command | Isolation, contributor clarity |
| CRDT | Not present | tf-crdt crate | Core design goal |
| Test framework | Internal test-support | Published tf-test-framework | External consumer value |
| Plugin system | Not present | Plugin trait in tf-core | Extensibility |
| Options | mux-conf + server config | tf-config + tf-core OptionsStore | Unified, testable |

---

## 20. Summary

This architecture takes the hard-won lessons from vibe-tmux (pure core, stream-first, lock-free reads, protocol fixture testing, parity audits, binding-first design) and extends them into a fully layered system that supports:

1. **tmux compatibility** through protocol, control-mode, and behavioral fidelity -- but as a *layer*, not as the fundamental architecture
2. **ORM-like usage** through `tf-view` + `tf-query` with Django-style lookups, directly inspired by libtmux's `QueryList`
3. **Multi-language bindings** through thin wrappers over `tf-api` with query execution in Rust
4. **CRDT collaboration** through operation-derived state sync on top of the pure core
5. **Test framework** as a publishable crate for testing any terminal application
6. **Plugin extensibility** through pure trait interfaces in the core

The key insight is: by making the core even purer than vibe-tmux (moving buffers, hooks, key tables, options into the deterministic graph), everything else -- CRDT sync, test replay, plugin effects, format expansion -- becomes composable and testable without IO or mocking.