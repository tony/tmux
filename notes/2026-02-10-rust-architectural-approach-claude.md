---

# **Oku** -- A Next-Generation Rust Terminal Multiplexer Library

*Architecture & North-Star Design Document*

---

## 1. Project Name and Identity

**Name: `oku`** (Japanese: screen/curtain; short, `cargo`-friendly, unclaimed on crates.io-style namespaces)

**Tagline:** A layered, tmux-compatible terminal multiplexer library with CRDT state, ORM-like querying, and first-class language bindings.

**Design philosophy:**
- **Functional core, imperative shell** -- a pure, deterministic state machine at the center; IO, async, and platform concerns pushed to the edges.
- **tmux is the spec** -- 100% wire-compatible with real tmux clients and servers; the C tmux source (`tmux.h`, `tmux-protocol.h`, `control-notify.c`, `format.c`) is the source of truth for protocol and behavioral claims.
- **Library first, binary second** -- the primary artifact is a set of composable Rust crates, not a monolithic binary. Binaries and TUIs are thin consumers of the library.

**Lineage:** This design inherits hard-won lessons from the **vibe-tmux** project (stream-first architecture, purity boundary, protocol fixtures, AGENTS.md guardrails) and incorporates architectural patterns from **zellij** (typed instruction channels, thread bus), **ratatui** (layered core/widgets/backend crates), **tokio** (feature-gated layering), and **libtmux** (ORM-like QueryList, Django-style lookups).

---

## 2. Crate / Workspace Structure

```
oku/
├── crates/
│   ├── oku-core/            # PURE: state machine, graph, events, effects
│   ├── oku-proto/           # PURE: tmux binary protocol codec (imsg)
│   ├── oku-control/         # PURE: tmux control-mode parser
│   ├── oku-query/           # PURE: ORM-like query DSL & execution
│   ├── oku-crdt/            # PURE: CRDT state types & merge logic
│   ├── oku-format/          # PURE: tmux format string expansion (#{ })
│   ├── oku-command/         # PURE: command parsing, registry, args
│   ├── oku-layout/          # PURE: layout tree algorithms
│   ├── oku-vt/              # PURE: VT100/xterm terminal emulator
│   ├── oku-os/              # IMPURE: unsafe OS glue (PTY, SCM_RIGHTS, FD)
│   ├── oku-runtime/         # IMPURE: async event loop, actor system
│   ├── oku-server/          # IMPURE: tmux-compatible server
│   ├── oku-client/          # IMPURE: tmux-compatible client
│   ├── oku-api/             # FACADE: re-exports, ManagedMux, ServerHandle
│   ├── oku-test/            # TEST: terminal app test harness
│   └── oku-plugin/          # IMPURE: plugin host (WASI + native)
│
├── bindings/
│   ├── python/              # PyO3 + maturin (via uv)
│   ├── node/                # NAPI-RS
│   ├── cxx/                 # cxx bridge
│   └── c/                   # Stable C ABI (header-only shim)
│
├── tools/
│   ├── oku-tui/             # Ratatui-based TUI manager
│   ├── oku-sniff/           # Protocol sniffer/proxy
│   ├── oku-doctor/          # Environment diagnostics
│   └── oku-audit/           # tmux compatibility audits
│
├── Cargo.toml               # Workspace root
├── AGENTS.md                 # LLM/human guardrails
├── ARCHITECTURE.md           # This document (abbreviated)
└── notes/
    ├── plan.md              # Active P0 roadmap
    ├── completed.md         # Archived completed work
    └── ideas.md             # Future/experimental ideas
```

### Crate Dependency DAG

```
                    ┌──────────┐
                    │ oku-core │  (PURE: no IO, no async, no unsafe)
                    └────┬─────┘
           ┌─────────────┼─────────────┬──────────────┐
           │             │             │              │
     ┌─────▼─────┐ ┌────▼────┐ ┌──────▼─────┐ ┌─────▼──────┐
     │ oku-proto │ │oku-query│ │ oku-crdt   │ │oku-command │
     │  (PURE)   │ │ (PURE)  │ │  (PURE)    │ │  (PURE)    │
     └─────┬─────┘ └────┬────┘ └──────┬─────┘ └─────┬──────┘
           │             │             │              │
     ┌─────▼─────┐      │             │              │
     │oku-control│      │             │        ┌─────▼──────┐
     │  (PURE)   │      │             │        │oku-format  │
     └─────┬─────┘      │             │        │  (PURE)    │
           │             │             │        └─────┬──────┘
           │             │             │              │
     ┌─────▼─────────────▼─────────────▼──────────────▼──────┐
     │                    oku-api (FACADE)                    │
     │   re-exports + ManagedMux + ServerHandle + QuerySet   │
     └───────────────────────┬───────────────────────────────┘
                             │
          ┌──────────────────┼──────────────────┐
          │                  │                  │
    ┌─────▼─────┐    ┌──────▼──────┐    ┌──────▼──────┐
    │oku-runtime│    │oku-server   │    │oku-client   │
    │ (IMPURE)  │    │ (IMPURE)    │    │ (IMPURE)    │
    └─────┬─────┘    └──────┬──────┘    └──────┬──────┘
          │                  │                  │
          └─────────┬────────┘                  │
                    │                           │
              ┌─────▼─────┐                     │
              │  oku-os   │◄────────────────────┘
              │ (UNSAFE)  │
              └───────────┘
```

### Detailed Crate Specifications

| Crate | Purity | Dependencies (internal) | Key Types | Responsibility |
|-------|--------|------------------------|-----------|----------------|
| `oku-core` | **PURE** | none | `ServerGraph`, `Session`, `Window`, `Pane`, `Client`, `Job`, `Event`, `Effect`, typed IDs (`SessionId`, `WindowId`, `PaneId`, `ClientId`, `JobId`) | Authoritative state machine. Input: `Event`. Output: `(ServerGraph, Vec<Effect>)`. No IO, no async, no unsafe, no time/rng without injection. |
| `oku-proto` | **PURE** | `oku-core` (types only) | `ImsgHdr`, `ImsgCodec`, `MsgType`, identify builders | tmux binary protocol (imsg) codec. Explicit field parsing, length validation, allocation caps. |
| `oku-control` | **PURE** | `oku-core` (types only) | `ControlNotification`, `ControlParser` | tmux control-mode (`tmux -C`) stream parser. Maps `%%*` notifications to typed events. |
| `oku-query` | **PURE** | `oku-core` | `QuerySet<T>`, `Filter`, `Lookup`, `Selector` | ORM-like query DSL. Django/libtmux-style lookups (`name__icontains`, `startswith`, `regex`, `in`, etc.). Executes filtering in Rust over `ServerGraph` / view snapshots. |
| `oku-crdt` | **PURE** | `oku-core` | `CrdtGraph`, `OpLog`, `VectorClock`, `ActorId`, `Op` | CRDT state types and merge logic for collaborative multiplexing. Wraps `ServerGraph` mutations as CRDT operations. |
| `oku-command` | **PURE** | `oku-core` | `CommandRegistry`, `CommandEntry`, `Args`, `CmdFindState` | Command parsing, registry, argument validation. Mirrors tmux `cmd_entry` with `args_parse` templates. |
| `oku-format` | **PURE** | `oku-core`, `oku-command` | `FormatTree`, `FormatExpander` | tmux format string expansion (`#{session_name}`, conditionals, etc.). Backed by tmux `format.c` callback semantics. |
| `oku-layout` | **PURE** | `oku-core` | `LayoutTree`, `LayoutCell`, `LayoutDirection` | Layout algorithms (tiled, even-horizontal, even-vertical, main-horizontal, main-vertical, custom). tmux layout string parsing/serialization. |
| `oku-vt` | **PURE** | none | `Terminal`, `Screen`, `Grid`, `Cell`, `InputParser` | VT100/xterm terminal emulator (state machine). Processes escape sequences, maintains screen buffer. No IO. |
| `oku-os` | **IMPURE** (unsafe quarantine) | `oku-core` (types) | `PtyMaster`, `PtySlave`, `ScmRights`, `FdGuard` | OS-level glue: PTY allocation, SCM_RIGHTS FD passing, signal handling, close-on-exec. **Only crate with handwritten `unsafe`.** |
| `oku-runtime` | **IMPURE** | `oku-core`, `oku-proto`, `oku-control`, `oku-os` | `ActorSystem`, `SocketActor`, `RefreshDriver`, `EventLoop` | Async event loop, actor system, channel fan-in. Converts `Effect`s into IO operations and feeds resulting events back into `oku-core`. |
| `oku-server` | **IMPURE** | `oku-core`, `oku-proto`, `oku-command`, `oku-runtime`, `oku-os` | `TmuxServer` | tmux-compatible server. Accepts real tmux clients via Unix socket + imsg protocol. |
| `oku-client` | **IMPURE** | `oku-proto`, `oku-control`, `oku-runtime`, `oku-os` | `TmuxClient`, `SyncMuxClient` | tmux-compatible client. Connects to real tmux servers via imsg. |
| `oku-api` | **FACADE** | all pure + `oku-runtime`, `oku-server`, `oku-client` | `ManagedMux`, `ServerHandle`, `ServerFacade`, `ClientContext`, `View`, `ViewList` | Single public entry point. Re-exports the query DSL, view model, and managed connection facade. |
| `oku-test` | **TEST** | `oku-api`, `oku-vt` | `TestHarness`, `TestSession`, `ScreenAssert` | Terminal application test framework. Headless session management, process spawning, screen capture, assertion DSL. |
| `oku-plugin` | **IMPURE** | `oku-core`, `oku-api` | `PluginHost`, `PluginManifest`, `WasiPlugin`, `NativePlugin` | Plugin/extension system. WASI for sandboxed plugins, native for performance-critical plugins. |

### Purity Guarantees

**PURE crates MUST NOT:**
- Import `tokio`, `async-std`, or any async runtime
- Use `async`/`.await` keywords
- Import `nix`, `libc`, or any OS-specific crate
- Access filesystem, sockets, PTYs, or network
- Contain `unsafe` blocks
- Use `std::time::Instant/SystemTime` without injection
- Use `rand` without injection
- Panic in any reachable code path (`unwrap`, `expect`, `panic!` are forbidden)

**Enforcement:** CI runs `cargo check` with a custom lint pass that verifies PURE crates have no banned imports. The workspace `Cargo.toml` uses `[workspace.lints]` to deny `unwrap_used`, `expect_used`, and `panic` via clippy.

---

## 3. Core State Model

### 3.1 The ServerGraph

The heart of oku is `ServerGraph`, a pure, deterministic, in-memory representation of all multiplexer state. It mirrors tmux's global state exactly:

```rust
// oku-core/src/graph.rs

/// Authoritative state graph for the multiplexer.
/// This is the single source of truth. All mutations go through Event processing.
#[derive(Debug, Clone, PartialEq)]
pub struct ServerGraph {
    pub(crate) sessions: SlotMap<SessionId, Session>,
    pub(crate) windows: SlotMap<WindowId, Window>,
    pub(crate) panes: SlotMap<PaneId, Pane>,
    pub(crate) clients: SlotMap<ClientId, Client>,
    pub(crate) jobs: SlotMap<JobId, Job>,
    pub(crate) buffers: SlotMap<BufferId, PasteBuffer>,
    pub(crate) session_groups: SlotMap<SessionGroupId, SessionGroup>,
    pub(crate) options: OptionStore,  // global, session, window, pane options
    pub(crate) environ: EnvironStore, // environment variables
    pub(crate) key_tables: KeyTableStore,
    pub(crate) hooks: HookStore,
    pub(crate) alerts: AlertState,
    pub(crate) next_session_id: u32,  // mirrors tmux's next_session_id
    pub(crate) next_window_id: u32,
    pub(crate) next_pane_id: u32,
    pub(crate) message_log: VecDeque<MessageEntry>,
    pub(crate) server_flags: ServerFlags,
}
```

### 3.2 Typed IDs

Every entity uses a `slotmap::new_key_type!` for type-safe, generational indices. For interop with tmux, a `TmuxId` enum provides bidirectional mapping:

```rust
// Internal IDs (generational, type-safe)
slotmap::new_key_type! {
    pub struct SessionId;
    pub struct WindowId;
    pub struct PaneId;
    pub struct ClientId;
    pub struct JobId;
    pub struct BufferId;
    pub struct SessionGroupId;
}

// External tmux IDs (for protocol interop)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TmuxId {
    Session(u32),    // $N
    Window(u32),     // @N
    Pane(u32),       // %N
}

impl TmuxId {
    pub fn parse(s: &str) -> Result<Self, ParseError> { /* ... */ }
}

impl fmt::Display for TmuxId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Session(n) => write!(f, "${n}"),
            Self::Window(n) => write!(f, "@{n}"),
            Self::Pane(n) => write!(f, "%{n}"),
        }
    }
}
```

### 3.3 Entity Structures

Modeled directly from tmux's `struct session`, `struct window`, `struct window_pane`, `struct client`:

```rust
// oku-core/src/session.rs
#[derive(Debug, Clone, PartialEq)]
pub struct Session {
    pub name: String,
    pub cwd: PathBuf,
    pub creation_time: Timestamp,    // injected, not SystemTime
    pub last_attached: Timestamp,
    pub activity_time: Timestamp,
    pub windows: Vec<WindowLink>,    // ordered winlinks with indices
    pub active_window: Option<usize>, // index into windows
    pub last_windows: Vec<usize>,     // stack of last-visited
    pub flags: SessionFlags,
    pub options: SessionOptionOverrides,
    pub environ: HashMap<String, Option<String>>,
    pub group_id: Option<SessionGroupId>,
    pub tmux_id: Option<u32>,        // $N from tmux
    pub attached_count: u32,
}

// oku-core/src/window.rs
#[derive(Debug, Clone, PartialEq)]
pub struct Window {
    pub name: String,
    pub activity_time: Timestamp,
    pub panes: Vec<PaneId>,
    pub active_pane: Option<PaneId>,
    pub last_panes: Vec<PaneId>,
    pub layout_root: LayoutTree,
    pub size: Size,
    pub flags: WindowFlags,
    pub options: WindowOptionOverrides,
    pub tmux_id: Option<u32>,        // @N from tmux
}

// oku-core/src/pane.rs
#[derive(Debug, Clone, PartialEq)]
pub struct Pane {
    pub size: PaneSize,
    pub offset: Position,
    pub command: Vec<String>,
    pub shell: Option<String>,
    pub cwd: Option<PathBuf>,
    pub pid: Option<u32>,
    pub tty: Option<String>,
    pub flags: PaneFlags,
    pub exit_status: Option<i32>,
    pub dead_time: Option<Timestamp>,
    pub options: PaneOptionOverrides,
    pub tmux_id: Option<u32>,        // %N from tmux
    pub mode_stack: Vec<PaneMode>,
}

// oku-core/src/client.rs
#[derive(Debug, Clone, PartialEq)]
pub struct Client {
    pub name: String,
    pub session_id: Option<SessionId>,
    pub last_session_id: Option<SessionId>,
    pub tty_name: String,
    pub term: String,
    pub size: Size,
    pub flags: ClientFlags,
    pub key_table: String,
    pub identify: IdentifyState,
    pub uid: Option<u32>,
    pub pid: Option<u32>,
}
```

### 3.4 Event/Effect Engine

The core is a **pure function**: `(ServerGraph, Event) -> (ServerGraph, Vec<Effect>)`.

```rust
// oku-core/src/engine.rs

/// Process a single event against the graph, returning updated state and effects.
pub fn process_event(
    graph: &mut ServerGraph,
    event: Event,
    clock: &dyn Clock,  // injected time source
) -> Vec<Effect> {
    match event {
        Event::Command { name, args, context } => {
            dispatch_command(graph, &name, &args, context, clock)
        }
        Event::CreateSession { name } => { /* ... */ }
        Event::PtyOutput { pane_id, data } => { /* ... */ }
        Event::PaneExited { pane_id, status } => { /* ... */ }
        Event::ClientIdentify { client_id, info } => { /* ... */ }
        Event::Resize { client_id, size } => { /* ... */ }
        Event::Key { client_id, key } => { /* ... */ }
        Event::Timer { kind } => { /* ... */ }
        Event::CrdtOp { op } => {
            // Apply a CRDT operation from a remote actor
            apply_crdt_op(graph, op)
        }
        // ... full taxonomy matching tmux's event surface
    }
}
```

**Events** represent input intents (what happened):
- Commands (parsed tmux commands)
- Client lifecycle (identify, attach, detach, resize)
- PTY lifecycle (output, exit, error)
- Key/mouse input
- Timer ticks
- CRDT operations (remote state merge)
- Plugin messages

**Effects** represent output actions (what to do):
- SpawnProcess, KillProcess, WriteToProcess
- SendToClient (protocol frames)
- Redraw (screen regions)
- NotifyClient, NotifyHook
- StartJob, StopJob
- CrdtBroadcast (replicate operation)
- PluginCallback

---

## 4. Protocol Layer

### 4.1 tmux Binary Protocol (imsg)

Based on direct study of `tmux-protocol.h` and `tmux.h`:

```
Protocol version: 8

Frame format (native endian):
┌──────────────────────────────────────┐
│ ImsgHdr (16 bytes)                   │
│   type:   u32  (MsgType enum)       │
│   len:    u32  (total including hdr) │
│   flags:  u32                        │
│   peerid: u32                        │
├──────────────────────────────────────┤
│ Payload (len - 16 bytes)             │
│   Varies by message type             │
│   High bit of len → SCM_RIGHTS FD   │
└──────────────────────────────────────┘
```

Message types (from `tmux-protocol.h`):

```rust
// oku-proto/src/msg.rs
#[repr(u32)]
pub enum MsgType {
    Version = 12,
    
    // Identify burst (client → server)
    IdentifyFlags = 100,
    IdentifyTerm = 101,
    IdentifyTtyname = 102,
    // IdentifyOldCwd = 103,  // unused
    IdentifyStdin = 104,
    IdentifyEnviron = 105,
    IdentifyDone = 106,
    IdentifyClientPid = 107,
    IdentifyCwd = 108,
    IdentifyFeatures = 109,
    IdentifyStdout = 110,
    IdentifyLongFlags = 111,
    IdentifyTerminfo = 112,
    
    // Commands (bidirectional)
    Command = 200,
    Detach = 201,
    DetachKill = 202,
    Exit = 203,
    Exited = 204,
    Exiting = 205,
    Lock = 206,
    Ready = 207,
    Resize = 208,
    Shell = 209,
    Shutdown = 210,
    Suspend = 214,
    Unlock = 215,
    Wakeup = 216,
    Exec = 217,
    Flags = 218,
    
    // File transfer streams
    ReadOpen = 300,
    Read = 301,
    ReadDone = 302,
    WriteOpen = 303,
    Write = 304,
    WriteReady = 305,
    WriteClose = 306,
    ReadCancel = 307,
}
```

### 4.2 Protocol Parsing Rules

Inherited from vibe-tmux AGENTS.md and hardened:

1. **Explicit field parsing** -- never "cast bytes to struct"
2. **Length validation** -- verify `16 <= len <= 16384` before reading payload
3. **Allocation caps** -- no unbounded allocations from protocol data
4. **SCM_RIGHTS** -- high bit of `len` signals FD(s); bind FDs to correct frame; close on drop
5. **Two-phase decode** -- header first, then payload (handles partial reads)
6. **Native endian** -- tmux uses host byte order (not network order)
7. **Malformed frames** -- close connection cleanly, log with context
8. **Identify burst ordering** -- must match `client.c:client_write_identify`: LongFlags, LongFlags(client_flags), Term, Features, Ttyname, Cwd, Terminfo*, Stdin, Stdout, Clientpid, Environ*, Done

### 4.3 Control Mode Parser

Parses `tmux -C` output (text-based notifications used as **hints only**):

```rust
// oku-control/src/notification.rs
pub enum ControlNotification {
    SessionsChanged,
    WindowAdd { window_id: u32 },
    WindowClose { window_id: u32 },
    WindowRenamed { window_id: u32, name: String },
    UnlinkedWindowAdd { window_id: u32 },
    UnlinkedWindowClose { window_id: u32 },
    UnlinkedWindowRenamed { window_id: u32, name: String },
    LayoutChange { window_id: u32, layout: String, flags: String },
    PaneModeChanged { pane_id: u32 },
    ClientSessionChanged { client: String, session_id: u32, name: String },
    WindowPaneChanged { window_id: u32, pane_id: u32 },
    SessionChanged { session_id: u32, name: String },
    SessionRenamed { name: String },
    SessionWindowChanged { session_id: u32, window_id: u32 },
    PasteBufferChanged { name: String },
    PasteBufferDeleted { name: String },
    Output { pane_id: u32, data: String },
    Pause { timeout: u32 },
    Continue,
}
```

**Critical design principle:** Control mode is **hints only** -- it tells us *what changed*, but the binary protocol (imsg) remains authoritative for the actual data. This is validated against `~/study/c/tmux/control-notify.c`.

---

## 5. Runtime Architecture

### 5.1 Actor System

Inspired by zellij's typed instruction channels and vibe-tmux's `SocketActor`:

```
┌─────────────────────────────────────────────────────────┐
│                    oku-runtime                          │
│                                                         │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐             │
│  │  Server   │  │   PTY    │  │  Screen  │             │
│  │  Actor    │  │  Actor   │  │  Actor   │             │
│  │          │  │          │  │          │             │
│  │ typed    │  │ typed    │  │ typed    │             │
│  │ channel  │  │ channel  │  │ channel  │             │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘             │
│       │              │              │                   │
│  ┌────▼──────────────▼──────────────▼──────┐           │
│  │           Channel Fan-In Bus             │           │
│  │    (crossbeam Select / tokio::select!)   │           │
│  └────────────────────┬────────────────────┘           │
│                       │                                 │
│  ┌────────────────────▼────────────────────┐           │
│  │            Event Loop                    │           │
│  │  recv() → process_event() → Effects     │           │
│  │  Effects → dispatch to actors            │           │
│  └──────────────────────────────────────────┘           │
│                                                         │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐             │
│  │  Plugin  │  │ Background│  │  Route   │             │
│  │  Actor   │  │  Jobs     │  │  (per    │             │
│  │          │  │  Actor    │  │  client) │             │
│  └──────────┘  └──────────┘  └──────────┘             │
└─────────────────────────────────────────────────────────┘
```

### 5.2 Threading Model

Following zellij's proven pattern of 6-7 purpose-specific threads:

| Thread | Responsibility | Channel Type |
|--------|---------------|--------------|
| **Server** | Main event loop, `process_event()`, effect dispatch | Bounded mpsc (fan-in) |
| **PTY** | PTY allocation, process spawning, fd management | Bounded mpsc |
| **PTY Writer** | Writing to PTY fds (separated for backpressure) | Bounded mpsc |
| **Screen** | Screen/grid updates, rendering coordination | Bounded mpsc + unbounded |
| **Plugin** | WASI/native plugin hosting | Bounded mpsc |
| **Background Jobs** | Long-running commands, format jobs | Bounded mpsc |
| **Route (per-client)** | Client IPC gateway, isolation | Bounded mpsc |

### 5.3 Stream-First Design

Inherited from vibe-tmux as a non-negotiable:

- **Typed streams over polling** -- event loops `recv()` on channels, waking immediately on new data
- **RefreshHints** -- coalesced push notifications tell consumers to re-read the store
- **Lock-free reads** -- `ArcSwap<Option<View>>` for UI/bindings hot path (no lock contention)
- **Persistent connections** -- one connection per socket per backend; never reconnect/identify per tick
- **Latency target** -- UI and binding updates propagate in 10-20ms
- **Debounce** -- `RefreshPlanner` merges pending scopes (10-30ms coalesce window)

### 5.4 Event Fan-In Pattern

```rust
// oku-runtime/src/event_loop.rs
enum RuntimeEvent {
    // From client route threads
    ClientMessage { client_id: ClientId, msg: ClientToServerMsg },
    // From PTY actor
    PtyOutput { pane_id: PaneId, data: Vec<u8> },
    PtyExit { pane_id: PaneId, status: i32 },
    // From control mode parser
    ControlHint { notification: ControlNotification },
    // From plugin host
    PluginMessage { plugin_id: PluginId, msg: PluginToServerMsg },
    // From timer system
    Timer { kind: TimerKind },
    // From CRDT sync
    CrdtSync { actor_id: ActorId, ops: Vec<Op> },
    // Shutdown
    Shutdown,
}

// Main loop (blocking recv, no polling)
loop {
    let event = event_rx.recv()?;  // blocks until event arrives
    
    // Drain any queued events for batching
    let mut batch = vec![event];
    while let Ok(e) = event_rx.try_recv() {
        batch.push(e);
    }
    
    // Process batch through pure core
    for event in batch {
        let core_event = translate_runtime_event(event);
        let effects = oku_core::process_event(&mut graph, core_event, &clock);
        dispatch_effects(effects, &actors);
    }
}
```

---

## 6. API Layers

### 6.1 Layer Stack

```
┌──────────────────────────────────────────────┐
│  Layer 5: Language Bindings                   │
│  Python/Node/C++/C wrappers                  │
├──────────────────────────────────────────────┤
│  Layer 4: ORM-like Query DSL                  │
│  QuerySet, Filter, Lookup, typed selectors   │
├──────────────────────────────────────────────┤
│  Layer 3: Managed Facade (oku-api)           │
│  ManagedMux, ServerHandle, ServerFacade      │
├──────────────────────────────────────────────┤
│  Layer 2: Runtime (oku-runtime)              │
│  ActorSystem, EventLoop, RefreshDriver       │
├──────────────────────────────────────────────┤
│  Layer 1: Pure Core (oku-core)               │
│  ServerGraph, Event, Effect, process_event() │
└──────────────────────────────────────────────┘
```

### 6.2 Layer 1: Pure Core API

```rust
use oku_core::{ServerGraph, Event, Effect, engine::process_event};

let mut graph = ServerGraph::new();
let clock = oku_core::clock::FakeClock::new(0);

let effects = process_event(&mut graph, Event::CreateSession {
    name: "dev".into(),
}, &clock);

assert_eq!(graph.sessions().count(), 1);
assert!(effects.iter().any(|e| matches!(e, Effect::SpawnPane { .. })));
```

### 6.3 Layer 3: Managed Facade API

```rust
use oku_api::{ManagedMux, Backend};

// Connect to a real tmux server
let mux = ManagedMux::tmux(Some("/tmp/tmux-1000/default"), None, None)?;

// Or run an in-process server
let mux = ManagedMux::local()?;

// Get a view snapshot (lock-free read)
let view = mux.view()?;

// Navigate the object graph
for session in view.sessions() {
    println!("Session: {} ({})", session.name(), session.id());
    for window in session.windows() {
        println!("  Window: {}", window.name());
        for pane in window.panes() {
            println!("    Pane: {} ({}x{})", pane.id(), pane.size().cols, pane.size().rows);
        }
    }
}

// Execute commands
mux.cmd("split-window -h")?;
mux.cmd("send-keys 'echo hello' Enter")?;

// Subscribe to updates (stream-first)
let mut hints = mux.subscribe();
while let Ok(hint) = hints.recv().await {
    let view = mux.view()?;
    // React to changes
}
```

### 6.4 Layer 4: ORM-like Query DSL

Modeled after libtmux's `QueryList` with Django-style lookups:

```rust
use oku_api::prelude::*;
use oku_query::{session, window, pane};

let mux = ManagedMux::tmux(None, None, None)?;
let view = mux.view()?;

// Filter sessions
let dev_sessions = view.sessions()
    .filter(session::name().icontains("dev"));

// Chain queries
let editor_window = dev_sessions
    .get_one()?
    .windows()
    .filter(window::name().startswith("editor"))
    .get_one()?;

// Get active pane
let active_pane = editor_window.panes()
    .filter(pane::active().is_true())
    .get_one()?;

// Complex queries
let large_panes = view.panes()
    .filter(pane::width().gt(80))
    .filter(pane::height().gt(24))
    .filter(pane::current_command().regex("vim|nvim"));

// Count and existence
assert!(view.sessions().filter(session::name().exact("main")).exists());
assert_eq!(view.windows().count(), 5);
```

**Supported lookup operators** (matching libtmux's `LOOKUP_NAME_MAP`):

| Operator | Rust | Python | Node | Description |
|----------|------|--------|------|-------------|
| `exact` | `.exact("foo")` | `name="foo"` or `name__exact="foo"` | `{ name: "foo" }` | Exact match |
| `iexact` | `.iexact("foo")` | `name__iexact="foo"` | `{ name__iexact: "foo" }` | Case-insensitive exact |
| `contains` | `.contains("fo")` | `name__contains="fo"` | `{ name__contains: "fo" }` | Substring match |
| `icontains` | `.icontains("fo")` | `name__icontains="fo"` | `{ name__icontains: "fo" }` | Case-insensitive substring |
| `startswith` | `.startswith("f")` | `name__startswith="f"` | `{ name__startswith: "f" }` | Prefix match |
| `istartswith` | `.istartswith("f")` | `name__istartswith="f"` | `{ name__istartswith: "f" }` | Case-insensitive prefix |
| `endswith` | `.endswith("o")` | `name__endswith="o"` | `{ name__endswith: "o" }` | Suffix match |
| `iendswith` | `.iendswith("o")` | `name__iendswith="o"` | `{ name__iendswith: "o" }` | Case-insensitive suffix |
| `in` | `.is_in(&["a","b"])` | `name__in=["a","b"]` | `{ name__in: ["a","b"] }` | Membership |
| `nin` | `.not_in(&["a","b"])` | `name__nin=["a","b"]` | `{ name__nin: ["a","b"] }` | Non-membership |
| `regex` | `.regex(r"^dev\d+")` | `name__regex=r"^dev\d+"` | `{ name__regex: "^dev\\d+" }` | Regex match |
| `iregex` | `.iregex(r"^dev")` | `name__iregex=r"^dev"` | `{ name__iregex: "^dev" }` | Case-insensitive regex |
| `gt`, `gte`, `lt`, `lte` | `.gt(80)` | `width__gt=80` | `{ width__gt: 80 }` | Numeric comparisons |
| `is_true`, `is_false` | `.is_true()` | `active="1"` | `{ active: true }` | Boolean checks |

**Implementation approach:** `oku-query` uses a `derive(Queryable)` macro to generate typed field selectors from entity structs, plus a `query_fields!` macro for composing filters. All filtering executes in Rust; bindings construct criteria in their language and pass them to Rust for execution.

---

## 7. Bindings Architecture

### 7.1 Principles

- **Matching runs in Rust** -- language wrappers construct filter criteria, but execution happens in `oku-query` (Rust)
- **Public APIs never mention JSON/frame/store** -- those are internal plumbing
- **Object graph mirrors tmux** -- `server.sessions[0].windows[0].panes[0]`
- **Commands mirror libtmux** -- `.cmd("split-window -h")` everywhere
- **Stream/iterator update surfaces** -- not sleep+poll loops

### 7.2 Python (PyO3 + maturin)

```python
from oku import Server, LocalServer

# Connect to real tmux
server = Server(socket_name="default")
# Or in-process
server = LocalServer()

# libtmux-compatible API
sessions = server.sessions.filter(name__icontains="dev")
session = sessions.get_one()

windows = session.windows
pane = windows[0].panes.get(pane_active="1")

# Commands
server.cmd("new-session", "-d", "-s", "test")
session.cmd("split-window", "-h")
pane.send_keys("echo hello", enter=True)

# Capture
output = pane.capture_pane()
assert "hello" in output

# Stream updates
for hint in server.subscribe():
    view = server.view()
    print(f"Updated: {hint}")
```

**Implementation:** PyO3 classes wrapping `oku-api` handles. `#[pyclass]` on `Session`, `Window`, `Pane`, `Client`. `QuerySet` exposed as Python iterator with `filter(**kwargs)` and `get_one()`. The Python QuerySet marshals `field__operator=value` kwargs into Rust `Filter` objects, then calls into `oku-query` for execution.

### 7.3 Node.js (NAPI-RS)

```javascript
const { Server, LocalServer } = require('@oku/bindings');

const server = new Server({ socketName: 'default' });
const view = server.view();

const session = view.sessions
    .filter({ name__icontains: 'dev' })
    .getOne();

const window = session.windows
    .filter({ name__startswith: 'editor' })
    .getOne();

// Commands
await server.cmd('split-window -h');

// Async stream updates
for await (const hint of server.subscribe()) {
    const view = server.view();
    console.log('Updated:', hint);
}
```

**Implementation:** NAPI-RS for zero-overhead Node bindings. Async iterators backed by `oku-api`'s broadcast channel. Filter objects constructed in JS, serialized to Rust `Filter` structs.

### 7.4 C++ (cxx bridge)

```cpp
#include <oku/oku.h>

auto server = oku::Server::tmux("/tmp/tmux-1000/default");
auto view = server->view();

auto sessions = view->sessions()
    .filter(oku::query::session::name().icontains("dev"));
auto session = sessions.get_one();

auto windows = session->windows()
    .filter(oku::query::window::name().startswith("editor"));

server->cmd("split-window -h");
```

**Implementation:** `cxx::bridge` in `oku-cxx` crate. Thin C++ wrapper in `bindings/cxx/oku.hpp`. No handwritten unsafe in the cxx crate (only cxx-generated glue). RAII guards for resource management.

### 7.5 C ABI (Stable)

```c
#include <oku/oku.h>

oku_server_t server = oku_server_tmux("/tmp/tmux-1000/default");
oku_view_t view = oku_server_view(server);

oku_query_t q = oku_query_new();
oku_query_add(q, "name__icontains", "dev");
oku_session_list_t sessions = oku_view_sessions_filter(view, q);

oku_server_cmd(server, "split-window -h");

oku_query_free(q);
oku_view_free(view);
oku_server_free(server);
```

**Implementation:** Stable C ABI with `u64` handles (not raw pointers). `0` is an invalid handle. All allocations managed by oku; caller uses `_free` functions. Header generated from Rust via `cbindgen`.

---

## 8. Test Framework Design

### 8.1 Purpose

`oku-test` turns the multiplexer core into a headless terminal application test harness. Developers can:

- Spawn processes in virtual panes
- Send keystrokes and text
- Capture screen state at any point
- Assert on screen contents, cursor position, cell attributes
- Run integration tests without a real terminal

### 8.2 API

```rust
use oku_test::{TestHarness, ScreenAssert};

#[test]
fn test_vim_opens_file() {
    let harness = TestHarness::new()
        .size(80, 24)
        .build();
    
    let session = harness.new_session("test");
    let pane = session.active_pane();
    
    // Spawn a process
    pane.send_keys("vim test.txt");
    pane.send_keys("Enter");
    
    // Wait for screen to settle
    pane.wait_for(|screen| screen.contains("test.txt"), Duration::from_secs(5))?;
    
    // Assert on screen state
    let screen = pane.capture();
    assert!(screen.contains("test.txt"));
    assert_eq!(screen.cursor_position(), (0, 0));
    
    // Assert on specific cells
    screen.assert_at(0, 0, ScreenAssert::char_is(' '));
    screen.assert_at(0, 23, ScreenAssert::contains("test.txt"));
    
    // Send vim commands
    pane.send_keys(":q!");
    pane.send_keys("Enter");
    
    pane.wait_for(|screen| screen.contains("$"), Duration::from_secs(2))?;
}
```

### 8.3 Screen Capture & Assertion DSL

```rust
// oku-test/src/screen_assert.rs

pub struct CapturedScreen {
    cells: Vec<Vec<Cell>>,
    cursor: Position,
    size: Size,
}

impl CapturedScreen {
    /// Full text content of the screen.
    pub fn text(&self) -> String { /* ... */ }
    
    /// Check if any line contains the given string.
    pub fn contains(&self, s: &str) -> bool { /* ... */ }
    
    /// Get text of a specific line.
    pub fn line(&self, row: usize) -> &str { /* ... */ }
    
    /// Get a specific cell.
    pub fn cell(&self, col: usize, row: usize) -> &Cell { /* ... */ }
    
    /// Cursor position.
    pub fn cursor_position(&self) -> (usize, usize) { /* ... */ }
    
    /// Diff two screens for test output.
    pub fn diff(&self, other: &CapturedScreen) -> ScreenDiff { /* ... */ }
    
    /// Snapshot for insta.
    pub fn snapshot(&self) -> String { /* ... */ }
}

/// Fluent assertion builder.
pub struct ScreenAssert;
impl ScreenAssert {
    pub fn char_is(c: char) -> CellAssertion { /* ... */ }
    pub fn contains(s: &str) -> LineAssertion { /* ... */ }
    pub fn fg_is(color: Color) -> CellAssertion { /* ... */ }
    pub fn bg_is(color: Color) -> CellAssertion { /* ... */ }
    pub fn bold() -> CellAssertion { /* ... */ }
    pub fn has_hyperlink(url: &str) -> CellAssertion { /* ... */ }
}
```

### 8.4 Integration with oku-vt

`oku-test` wraps `oku-vt` (the pure terminal emulator) to process escape sequences and maintain the virtual screen buffer. Because `oku-vt` is pure, tests are deterministic and fast.

### 8.5 Real tmux Validation Tests

In addition to headless testing, `oku-test` supports running the same test scenarios against a real tmux server to validate behavioral parity:

```rust
#[test]
#[ignore] // requires real tmux
fn parity_split_window() {
    let result_oku = run_with_oku(|harness| {
        harness.cmd("split-window -h");
        harness.view().panes().count()
    });
    
    let result_tmux = run_with_tmux(|server| {
        server.cmd("split-window -h");
        server.view().panes().count()
    });
    
    assert_eq!(result_oku, result_tmux);
}
```

---

## 9. CRDT Design

### 9.1 Motivation

Traditional tmux has a single-writer server. Oku extends this with CRDT support, enabling:

- **Collaborative sessions** -- multiple users editing the same session state concurrently
- **Offline-capable** -- clients can accumulate changes offline and merge on reconnect
- **Conflict-free** -- no coordination required for concurrent modifications
- **Audit trail** -- operation log enables undo/redo and state replay

### 9.2 Architecture

```
┌─────────────────────────────────────────────────┐
│                  oku-crdt                        │
│                                                  │
│  ┌────────────┐   ┌──────────────┐              │
│  │ VectorClock│   │  ActorId     │              │
│  │            │   │ (unique per  │              │
│  │ {actor:seq}│   │  connection) │              │
│  └────────────┘   └──────────────┘              │
│                                                  │
│  ┌────────────────────────────────────────┐     │
│  │  OpLog (append-only operation log)     │     │
│  │                                        │     │
│  │  Op {                                  │     │
│  │    actor: ActorId,                     │     │
│  │    seq: u64,                           │     │
│  │    deps: VectorClock,                  │     │
│  │    timestamp: Timestamp,               │     │
│  │    payload: OpPayload,                 │     │
│  │  }                                     │     │
│  └────────────────────────────────────────┘     │
│                                                  │
│  ┌────────────────────────────────────────┐     │
│  │  CrdtGraph                             │     │
│  │    Wraps ServerGraph + OpLog           │     │
│  │    apply_local(Event) → (Effects, Op)  │     │
│  │    apply_remote(Op) → Effects          │     │
│  │    merge(remote_ops: &[Op]) → Effects  │     │
│  └────────────────────────────────────────┘     │
└─────────────────────────────────────────────────┘
```

### 9.3 Operation Types

```rust
// oku-crdt/src/op.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OpPayload {
    // Session operations
    CreateSession { name: String },
    RenameSession { session_id: TmuxId, name: String },
    DestroySession { session_id: TmuxId },
    
    // Window operations
    CreateWindow { session_id: TmuxId, name: String, index: Option<u32> },
    RenameWindow { window_id: TmuxId, name: String },
    DestroyWindow { window_id: TmuxId },
    MoveWindow { window_id: TmuxId, target_session: TmuxId, index: u32 },
    
    // Pane operations
    SplitPane { window_id: TmuxId, direction: SplitDirection, size: PaneSize },
    ResizePane { pane_id: TmuxId, size: PaneSize },
    DestroyPane { pane_id: TmuxId },
    
    // Client operations
    AttachSession { client_id: TmuxId, session_id: TmuxId },
    SelectWindow { session_id: TmuxId, window_id: TmuxId },
    SelectPane { window_id: TmuxId, pane_id: TmuxId },
    
    // Option operations
    SetOption { scope: OptionScope, key: String, value: String },
    UnsetOption { scope: OptionScope, key: String },
    
    // Environment operations
    SetEnv { scope: EnvScope, key: String, value: Option<String> },
}
```

### 9.4 Conflict Resolution Strategy

**Last-Writer-Wins Register (LWW)** for scalar values:
- Session name, window name, option values
- Timestamp + actor ID for deterministic tie-breaking

**Add-Wins Set (AW-Set)** for collections:
- Windows in a session, panes in a window
- Concurrent create + destroy: create wins (matching tmux's "creation is intentional" semantics)

**Causal ordering** via vector clocks:
- Operations carry dependency vector clocks
- Remote operations are applied in causal order
- Concurrent operations are deterministically ordered by (timestamp, actor_id)

**Session/window/pane numbering:**
- CRDT operations reference entities by `TmuxId` (tmux's `$N`, `@N`, `%N`)
- Each actor maintains a local ID allocator that avoids conflicts via actor-prefixed numbering
- On merge, ID conflicts are resolved by the lower actor ID taking precedence

### 9.5 Sync Protocol

```
Actor A                    Actor B
   │                          │
   │── Op(create-session) ──→ │
   │                          │
   │←── Op(rename-session) ──│
   │                          │
   │── Op(create-window)  ──→ │
   │                          │
   │   (concurrent ops)       │
   │── Op(split-pane)     ──→ │
   │←── Op(split-pane)    ──│
   │                          │
   │   (both apply both;      │
   │    deterministic merge)  │
```

The sync protocol is **transport-agnostic** -- CRDT operations can be transmitted over:
- Direct TCP connections between oku servers
- WebSocket for browser-based collaboration
- A relay server for NAT traversal
- Even via tmux's own control mode (embedding ops in notifications)

---

## 10. Plugin / Extension System

### 10.1 Architecture

```
┌────────────────────────────────────────────┐
│              oku-plugin                     │
│                                            │
│  ┌──────────────────────────────────┐     │
│  │  Plugin Host                     │     │
│  │                                  │     │
│  │  ┌───────────┐  ┌───────────┐  │     │
│  │  │ WASI      │  │ Native    │  │     │
│  │  │ Sandbox   │  │ (dlopen)  │  │     │
│  │  │           │  │           │  │     │
│  │  │ .wasm     │  │ .so/.dylib│  │     │
│  │  └───────────┘  └───────────┘  │     │
│  │                                  │     │
│  │  Plugin API (trait):             │     │
│  │    on_event(&Event) → Vec<Op>   │     │
│  │    on_render(&View) → Overlay   │     │
│  │    on_key(&Key) → Action        │     │
│  │    on_timer() → Vec<Op>         │     │
│  └──────────────────────────────────┘     │
│                                            │
│  ┌──────────────────────────────────┐     │
│  │  Plugin Manifest (TOML)          │     │
│  │                                  │     │
│  │  [plugin]                        │     │
│  │  name = "git-status"             │     │
│  │  version = "0.1.0"              │     │
│  │  runtime = "wasi"               │     │
│  │  permissions = ["read-pane",     │     │
│  │                  "status-line"]  │     │
│  └──────────────────────────────────┘     │
└────────────────────────────────────────────┘
```

### 10.2 Plugin API

```rust
// oku-plugin/src/api.rs

/// The interface that plugins implement.
pub trait Plugin: Send + Sync {
    /// Called when a core event is processed.
    fn on_event(&mut self, event: &Event, view: &View) -> Vec<PluginOp> {
        Vec::new()
    }
    
    /// Called when rendering; can add overlays to the screen.
    fn on_render(&self, view: &View) -> Option<Overlay> {
        None
    }
    
    /// Called on key press; can intercept or transform keys.
    fn on_key(&mut self, key: &KeyEvent, view: &View) -> KeyAction {
        KeyAction::Pass
    }
    
    /// Called on timer tick.
    fn on_timer(&mut self, view: &View) -> Vec<PluginOp> {
        Vec::new()
    }
    
    /// Plugin metadata.
    fn manifest(&self) -> &PluginManifest;
}

/// Actions a plugin can request.
pub enum PluginOp {
    /// Execute a command.
    Command { name: String, args: Vec<String> },
    /// Send data to a pane.
    WriteToPane { pane_id: PaneId, data: Vec<u8> },
    /// Set a user option.
    SetOption { key: String, value: String },
    /// Add a status line component.
    SetStatusComponent { position: StatusPosition, content: String },
    /// Register a custom command.
    RegisterCommand { name: String },
    /// Schedule a timer callback.
    ScheduleTimer { duration: Duration },
}
```

### 10.3 Capabilities Beyond tmux

Enabled by the plugin system and layered architecture:

| Feature | How |
|---------|-----|
| **Scripting** | WASI plugins in any language that compiles to WASM |
| **Collaborative sessions** | CRDT sync + relay server plugin |
| **Git integration** | Status line plugin showing branch/status |
| **Session persistence** | Plugin that serializes/restores session layout |
| **Smart pane naming** | Plugin that renames panes based on running process |
| **Web UI** | Plugin that serves a WebSocket-based browser client |
| **AI assistant** | Plugin that feeds screen content to LLMs for suggestions |
| **Custom layouts** | Layout plugins that go beyond tmux's tiled layouts |
| **Pane previews** | Minimap/thumbnail preview of other panes |

---

## 11. DOs and DON'Ts

### DOs

1. **DO** treat the C tmux source as the authoritative specification for protocol behavior, frame layouts, control notifications, and format variables.
2. **DO** use typed IDs (`SessionId`, `WindowId`, `PaneId`) everywhere -- never raw integers for entity references.
3. **DO** keep `oku-core` pure: every function must be deterministic given the same inputs. Inject time via `Clock` trait, randomness via `Rng` trait.
4. **DO** use `Result<T, E>` for all fallible operations. Never `unwrap()` or `expect()` in protocol decode/encode, server loop, or PTY loop.
5. **DO** use explicit field parsing for protocol messages. Validate lengths before reading. Cap allocations.
6. **DO** use stream-first patterns: channels and `recv()` for event loops, not `sleep()` + poll.
7. **DO** write snapshot tests (insta) for protocol codec round-trips.
8. **DO** write parity tests that compare oku behavior against real tmux behavior.
9. **DO** document every `unsafe` block with `// SAFETY: ...` and a focused test.
10. **DO** keep binding APIs libtmux-shaped: `server.sessions`, `session.windows`, `window.panes`, `.cmd()`.
11. **DO** run all filtering/querying in Rust, even when called from bindings.
12. **DO** use `#[non_exhaustive]` on public enums to allow future extension.
13. **DO** use `ArcSwap` or similar lock-free structures for view store reads.
14. **DO** add protocol fixtures captured by the sniffer tool for every new message type.
15. **DO** test with random frame segmentation (chunking robustness).
16. **DO** close connections cleanly on protocol violations.
17. **DO** use hermetic test environments with temp dirs/sockets; never touch the user's real tmux socket.
18. **DO** prefer inline format args (`format!("{value}")` not `format!("{}", value)`).
19. **DO** prefer method references over redundant closures (`iter.map(str::to_string)`).

### DON'Ts

1. **DON'T** add tokio, async, IO, libc/nix, or unsafe to any PURE crate. This is a hard invariant.
2. **DON'T** use `#[repr(packed)]` -- it creates unaligned access UB.
3. **DON'T** use serde-based IPC for tmux compatibility surfaces. The tmux protocol is binary, not JSON.
4. **DON'T** use "cast bytes to struct" for protocol decoding. Always parse explicitly.
5. **DON'T** use `unwrap()`, `expect()`, or `panic!()` in hot paths. Use `Result` and propagate errors.
6. **DON'T** expose frame/store/JSON concepts in public binding APIs. Keep those as internal plumbing.
7. **DON'T** implement matching/filtering logic in language bindings. Always call into Rust.
8. **DON'T** add new crates without a clear purity classification and dependency rationale.
9. **DON'T** use sleep-based polling as the default data update path. It must be layered above stream-first.
10. **DON'T** create duplicate connections per socket. One persistent connection per socket per backend.
11. **DON'T** reconnect/re-identify per tick or per command. Connection persistence is required.
12. **DON'T** use overlapping strategies (e.g., two PTY libraries, two FD-passing approaches). Pick one.
13. **DON'T** run `kill-server` against the developer's real tmux socket. Use dedicated test sockets.
14. **DON'T** invent protocol message ordering, payload layouts, or handshake shortcuts. Every claim must be backed by tmux source or captured fixtures.
15. **DON'T** mutate process environment in tests. Use dependency injection.
16. **DON'T** skip pre-commit hooks (`--no-verify`).
17. **DON'T** add features that break tmux compatibility in the core protocol path. Extensions must be opt-in and layered above.

---

## 12. AGENTS.md Rules

The following is a complete `AGENTS.md` file for the oku project, designed to constrain both human and LLM contributors:

```markdown
# AGENTS.md (human + LLM guardrails)

## Rule 0: Never kill real tmux
- **Never** run `kill-server` against the developer's real tmux socket.
- Always use dedicated test sockets (e.g., `/tmp/oku-test-*`).
- Verify the socket path before issuing destructive commands.
- Keep the kill-server guard enabled (`OKU_GUARD_KILL_SERVER=1`).

## Mission
Drop-in tmux compatibility. A real tmux client must be able to attach
to an oku server, and an oku client must be able to attach to a real
tmux server. "Works" is not success unless real tmux agrees.

## Non-negotiables

### 0) Push-first responsiveness (streams over polling)
- Prefer typed streams (channels / async streams) over tick-based polling.
- Event loops block on streams and wake immediately on new data.
- Polling/sleep loops acceptable only for backoff/diagnostics.
- Latency target: UI/binding updates propagate in ~10-20ms.
- One persistent connection per socket per backend.

### 1) Source-of-truth policy
Protocol/behavior claims must be backed by:
- Upstream tmux headers/source (`~/study/c/tmux`), or
- Fixtures captured by `tools/oku-sniff`

No invented message ordering, payload layouts, or handshake shortcuts.

### 2) Purity boundary
PURE crates (oku-core, oku-proto, oku-control, oku-query, oku-crdt,
oku-format, oku-command, oku-layout, oku-vt):
- No tokio, no async/.await
- No nix/libc
- No filesystem, sockets, PTYs
- No unsafe
- No time/rng without explicit injection

Core API shape:
- Input: typed Event
- Output: typed Effect values (data), executed by runtime

### 3) Unsafe quarantine
Only `oku-os` may contain handwritten `unsafe`.
`bindings/cxx` may contain cxx-generated unsafe only.

Every unsafe block:
- Minimal boundary
- Documents invariants with `// SAFETY: ...`
- Has a focused test that would fail if invariants break

Never use `#[repr(packed)]`.

### 4) Protocol parsing policy
- Explicit field parsing (no "cast bytes to struct")
- Validate lengths before reading
- Cap allocations
- Malformed frames → close connection cleanly

### 5) SCM_RIGHTS FD passing policy
- Forward ancillary data, not only bytes
- Bind FD(s) to correct protocol frame
- Close on drop (no leaks)
- Validate expected FD counts (fixture/source-backed)

### 6) No panics in hot paths
No unwrap()/expect() in:
- Protocol decode/encode
- Server loop
- PTY loop

Use Result, close the client, log with context.

### 7) Testing policy

Protocol: fixtures + snapshot tests + chunking robustness
Core: unit tests for Event → Effect (deterministic, no IO)
Integration: hermetic tests with temp dirs/sockets
Real tmux validation: parity tests against real tmux
CRDT: property-based tests for merge commutativity/associativity

### 8) CRDT correctness
- All operations must be commutative (apply in any order, same result)
- Property-based tests with proptest for merge operations
- Vector clocks must be monotonically increasing per actor
- OpLog is append-only; no retroactive edits

### 9) Binding surface rules
- Public APIs never expose frame/store/JSON concepts
- Query criteria merge in language wrappers; matching runs in Rust
- Bindings surface tmux object graph: sessions/windows/panes/clients

## Dependency policy
- Minimal deps, tightly scoped to owning crate
- No overlapping stacks
- No serde-based IPC for tmux compatibility surfaces

## Required before commit
Run: `just check-llm`

## Git commit format
type(scope[detail]) concise description

why: Explanation.
what:
- Change 1
- Change 2
```

---

## 13. Implementation Phases

### Phase 0: Foundation (Weeks 1-4)

**Goal:** Workspace skeleton, pure core compiles, basic event/effect round-trip.

| Task | Crate | Deliverable |
|------|-------|-------------|
| Create workspace with all crate stubs | root | `Cargo.toml`, all `lib.rs` files |
| Define typed IDs | `oku-core` | `SessionId`, `WindowId`, `PaneId`, etc. |
| Implement `ServerGraph` | `oku-core` | SlotMap-based graph with CRUD |
| Implement `Event` and `Effect` enums | `oku-core` | Full taxonomy matching tmux commands |
| Implement `process_event()` for basic operations | `oku-core` | Create/destroy session/window/pane |
| Set up AGENTS.md and CI | root | Lint checks, purity verification |
| Set up workspace lints | root | deny unwrap/expect/panic |
| Write unit tests | `oku-core` | Event -> Effect deterministic tests |

**Acceptance:** `cargo test -p oku-core` passes. `cargo check` on pure crates with no async/IO/unsafe deps.

### Phase 1: Protocol & Terminal (Weeks 5-10)

**Goal:** Wire-compatible tmux protocol codec, terminal emulator.

| Task | Crate | Deliverable |
|------|-------|-------------|
| Implement imsg codec | `oku-proto` | `ImsgHdr`, `ImsgCodec`, `MsgType` |
| Implement identify burst builder/parser | `oku-proto` | Client/server identify sequence |
| Implement control-mode parser | `oku-control` | `ControlNotification` enum, line parser |
| Port protocol fixtures from vibe-tmux | `oku-proto` | Snapshot tests, chunking tests |
| Implement VT100 state machine | `oku-vt` | `Terminal`, `Screen`, `Grid`, `Cell` |
| Implement format string expander | `oku-format` | `#{session_name}`, conditionals |
| Implement command registry | `oku-command` | `CommandEntry`, `Args`, 40+ tmux commands |
| Implement layout algorithms | `oku-layout` | Tiled, even-h, even-v, main-h, main-v |
| Build protocol sniffer | `tools/oku-sniff` | Capture + replay tmux conversations |

**Acceptance:** Protocol codec round-trips all tmux message types. VT100 passes vttest basics. Sniffer can capture real tmux conversations.

### Phase 2: OS Layer & Runtime (Weeks 11-16)

**Goal:** PTY management, async event loop, basic server/client.

| Task | Crate | Deliverable |
|------|-------|-------------|
| Implement PTY allocation | `oku-os` | `PtyMaster`, `PtySlave`, fork+exec |
| Implement SCM_RIGHTS | `oku-os` | FD passing over Unix sockets |
| Implement signal handling | `oku-os` | SIGWINCH, SIGTERM, SIGCHLD |
| Build actor system | `oku-runtime` | `ActorSystem`, channel fan-in, event loop |
| Implement RefreshDriver | `oku-runtime` | Debounced refresh hints |
| Build basic tmux server | `oku-server` | Accept connections, identify, basic commands |
| Build basic tmux client | `oku-client` | Connect, identify, send commands |
| Implement lock-free view store | `oku-runtime` | `ArcSwap<Option<View>>` |

**Acceptance:** `oku-server` accepts a real `tmux attach` client. `oku-client` connects to a real tmux server and runs `list-sessions`. PTY tests pass with strict telemetry.

### Phase 3: API Facade & Query DSL (Weeks 17-22)

**Goal:** Public API, ORM-like queries, first bindings.

| Task | Crate | Deliverable |
|------|-------|-------------|
| Build `ManagedMux` facade | `oku-api` | Connect to tmux, local server, view() |
| Build `ServerHandle` read-only view | `oku-api` | Navigation: sessions/windows/panes |
| Build `ServerFacade` command interface | `oku-api` | `.cmd()`, `.exec()` |
| Implement QuerySet with all operators | `oku-query` | `filter()`, `get_one()`, `count()` |
| Implement `derive(Queryable)` macro | `oku-query` | Auto-generate typed selectors |
| Build Python bindings | `bindings/python` | PyO3 classes, QuerySet, subscribe |
| Build Node bindings | `bindings/node` | NAPI-RS classes, async iterator |
| Build C++ bindings | `bindings/cxx` | cxx bridge, wrapper header |
| Build C ABI | `bindings/c` | Stable handle-based API, cbindgen header |

**Acceptance:** `server.sessions.filter(name__icontains="dev").get_one()` works from Rust, Python, Node, C++, and C. Binding smoke tests pass.

### Phase 4: Test Framework & CRDT (Weeks 23-30)

**Goal:** Terminal test harness, collaborative state.

| Task | Crate | Deliverable |
|------|-------|-------------|
| Build test harness | `oku-test` | `TestHarness`, headless sessions |
| Implement screen capture/assertion | `oku-test` | `CapturedScreen`, `ScreenAssert` |
| Implement wait_for with timeout | `oku-test` | Polling with configurable timeout |
| Write parity test framework | `oku-test` | Run scenarios against oku + real tmux |
| Implement CRDT OpLog | `oku-crdt` | Append-only log, vector clocks |
| Implement CRDT merge | `oku-crdt` | LWW registers, AW-Sets |
| Property-based CRDT tests | `oku-crdt` | proptest for commutativity/associativity |
| Implement CRDT sync protocol | `oku-crdt` | Op exchange between actors |

**Acceptance:** Test harness can spawn vim, send keys, and assert on screen. CRDT merge of concurrent session modifications is deterministic. Property tests pass for 10000+ operations.

### Phase 5: Plugin System & Polish (Weeks 31-38)

**Goal:** Extension system, full tmux command coverage, production readiness.

| Task | Crate | Deliverable |
|------|-------|-------------|
| Build plugin host | `oku-plugin` | WASI sandbox, native dlopen |
| Define plugin API | `oku-plugin` | `Plugin` trait, `PluginOp` enum |
| Build TUI manager | `tools/oku-tui` | Ratatui-based session manager |
| Complete tmux command coverage | `oku-command` | All ~180 tmux commands |
| Build compatibility audit tools | `tools/oku-audit` | format-audit, command-audit, regress-audit |
| Full parity test suite | `oku-test` | Against tmux regress scripts |
| Performance benchmarks | root | Latency, throughput, memory |
| Documentation | root | API docs, architecture guide, tutorials |

**Acceptance:** All tmux regress scripts pass. Plugin API is stable. TUI can manage sessions. Benchmarks show <20ms update latency.

---

## 14. Acceptance Criteria

### Protocol Compatibility (Hard Requirements)

| Criterion | Measurement | Target |
|-----------|-------------|--------|
| Client interop | Real tmux client attaches to oku server | 100% success |
| Server interop | oku client attaches to real tmux server | 100% success |
| Identify burst | Protocol sniffer captures show byte-identical identify sequences | 100% match |
| Command coverage | `just command-audit` | 0 missing commands |
| Format variables | `just format-audit` | 0 missing variables |
| Regress scripts | `just regress-audit` | 100% tracked |
| Parity tests | `just parity-e2e` | >95% pass rate |

### Code Quality (Hard Requirements)

| Criterion | Measurement | Target |
|-----------|-------------|--------|
| Pure crate purity | CI lint for banned imports | 0 violations |
| No panics in hot paths | clippy deny unwrap/expect/panic | 0 violations |
| Unsafe quarantine | Only `oku-os` has handwritten unsafe | Verified by CI |
| Test coverage | `cargo llvm-cov` | >80% line coverage |
| `just check-llm` | Full lint + test suite | Green |
| Protocol fixtures | Every message type has a fixture | 100% coverage |

### Performance (Soft Targets)

| Criterion | Measurement | Target |
|-----------|-------------|--------|
| UI update latency | Event → screen update | <20ms |
| View snapshot read | Lock-free `ArcSwap` load | <1 microsecond |
| Event throughput | Events processed per second | >100,000/s |
| Memory per session | RSS for 1 session/1 window/1 pane | <5MB |
| Memory per 100 panes | RSS for 1 session/10 windows/100 panes | <50MB |
| CRDT merge latency | Merge 1000 operations | <10ms |

### API Ergonomics (Measured by Usage)

| Criterion | Measurement | Target |
|-----------|-------------|--------|
| Rust API lines to list sessions | LOC comparison with libtmux | Comparable or fewer |
| Python API compatibility | libtmux test suite passrate | >90% |
| Query DSL operators | Supported lookup operators | All 12+ from libtmux |
| Binding language count | Languages with working bindings | 4 (Python, Node, C++, C) |

### CRDT Correctness (Hard Requirements)

| Criterion | Measurement | Target |
|-----------|-------------|--------|
| Commutativity | `merge(a,b) == merge(b,a)` | proptest 10000 cases |
| Associativity | `merge(merge(a,b),c) == merge(a,merge(b,c))` | proptest 10000 cases |
| Idempotence | `merge(a,a) == a` | proptest 10000 cases |
| Convergence | All actors reach same state | proptest 1000 scenarios |
| Causal consistency | Ops respect vector clock ordering | 100% verified |

---

## Appendix A: Key Architectural Decisions and Rationale

### A.1 Why SlotMap for Entity Storage

tmux uses RB-trees (`RB_HEAD`) for sessions, windows, and a combination of TAILQ and RB for panes. SlotMap provides:
- O(1) lookup by ID (vs O(log n) for RB-tree)
- Generational indices prevent use-after-free (a common tmux bug class)
- Iterator order is insertion order (matching tmux's behavior for most use cases)
- Memory-efficient (dense array, no pointer chasing)

### A.2 Why Separate oku-vt from oku-core

tmux's terminal emulation (`input.c`, `screen.c`, `grid.c`) is deeply intertwined with its server logic. Separating the VT emulator enables:
- Reuse in the test framework without pulling in server dependencies
- Independent testing against vttest and other terminal conformance suites
- Potential use as a standalone terminal emulator library
- Cleaner mental model (state machine vs terminal display)

### A.3 Why CRDT Instead of OT for Collaborative State

Operational Transformation (OT) requires a central server for transformation functions. CRDTs provide:
- No central coordinator required (true peer-to-peer possible)
- Offline support (operations accumulate and merge on reconnect)
- Proven formal properties (commutativity, convergence)
- Good fit for the tree-structured tmux state model
- Well-studied in the Rust ecosystem (automerge, etc.)

### A.4 Why Both WASI and Native Plugins

WASI provides sandboxing (a plugin cannot crash the server or access the filesystem unless permitted). Native plugins provide performance for latency-critical extensions (e.g., custom rendering). The dual approach mirrors zellij's plugin model (WASM plugins for most use cases, with native escape hatches).

### A.5 Why cxx Instead of Raw C FFI for C++

From vibe-tmux's experience: cxx provides compile-time safety, RAII, and `Result`/`expected` patterns. The tradeoff (rebuild required, no ABI stability) is acceptable because the C++ bindings target developers who compile from source, not system package consumers. A separate stable C ABI (`bindings/c`) handles the ABI-stable use case.

### A.6 Why a Separate oku-query Crate

The query DSL is pure (no IO) and useful independently of the runtime. Separating it:
- Keeps `oku-core` focused on state mutation
- Allows bindings to use query logic without pulling in the full API
- Enables query compilation/optimization independently
- Matches the pattern from libtmux where `QueryList` is an internal utility

---

## Appendix B: Glossary

| Term | Definition |
|------|-----------|
| **Actor** | A thread with a typed instruction channel (zellij pattern) |
| **Effect** | A value describing a side effect to be performed by the runtime |
| **Event** | A value describing something that happened (input to the core) |
| **imsg** | tmux's binary message protocol over Unix sockets |
| **OpLog** | Append-only log of CRDT operations |
| **PURE** | A crate with no IO, async, unsafe, or platform-specific code |
| **RefreshHint** | A push notification that the view store has been updated |
| **SCM_RIGHTS** | Unix mechanism for passing file descriptors over sockets |
| **ServerGraph** | The authoritative in-memory state of all multiplexer entities |
| **TmuxId** | An external ID in tmux format ($N, @N, %N) |
| **View** | A read-only snapshot of the ServerGraph for UI/binding consumption |
| **VectorClock** | A map of {actor -> sequence_number} for causal ordering |

---

*This document is the north star for oku development. All implementation decisions should be traceable back to a section in this document. When in doubt, consult the tmux C source, the vibe-tmux AGENTS.md, and this architecture.*