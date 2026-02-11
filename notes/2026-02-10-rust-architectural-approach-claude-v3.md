I now have comprehensive understanding of all the referenced codebases. Let me produce the architectural document.

# Rust Terminal Multiplexer: Comprehensive Architectural Plan

**Project codename: `mux-rs`**
**License: MIT OR Apache-2.0**
**Rust edition: 2024 (minimum 1.85)**
**Date: 2026-02-10**

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Workspace Structure and Purity Classifications](#2-workspace-structure-and-purity-classifications)
3. [Entity Model](#3-entity-model)
4. [Event/Effect Engine Design](#4-eventeffect-engine-design)
5. [Protocol Codec Design](#5-protocol-codec-design)
6. [ORM-like Query API](#6-orm-like-query-api)
7. [Language Bindings](#7-language-bindings)
8. [CRDT Transaction Layer](#8-crdt-transaction-layer)
9. [tmux Version Manager Tool](#9-tmux-version-manager-tool)
10. [Test Framework and Harness Design](#10-test-framework-and-harness-design)
11. [Binding Test Frameworks](#11-binding-test-frameworks)
12. [Visual Client (TUI)](#12-visual-client-tui)
13. [DOs and DON'Ts](#13-dos-and-donts)
14. [AGENTS.md Template](#14-agentsmd-template)
15. [Phased Implementation Plan](#15-phased-implementation-plan)
16. [Risks and Mitigations](#16-risks-and-mitigations)

---

## 1. Executive Summary

This document describes a Rust terminal multiplexer designed for 100% tmux protocol compatibility, an ORM-like API surface inspired by libtmux, language bindings for Python/Node/C++, CRDT-based collaborative state, a comprehensive test framework with PTY mocking and hermetic isolation, tooling for managing multiple tmux versions, and a Rust-native TUI client.

The architecture is directly grounded in:

- **tmux source** (`/home/d/study/c/tmux/`) -- `tmux.h` structs (`struct session`, `struct window`, `struct window_pane`, `struct client`), `tmux-protocol.h` message types (`enum msgtype`, `struct msg_command`, `PROTOCOL_VERSION 8`), `control-notify.c` notification taxonomy, `format.c` format expansion engine, `server.c` event loop, `client.c` identify handshake
- **vibe-tmux** (`/home/d/work/rust/vibe-tmux/`) -- proven architecture with `mux-core` (PURE), `mux-proto`, `mux-view`, `mux-query`, `mux-os` (unsafe quarantine), `mux-api` facade, `mux-server`, `mux-client`, `tmux-vm` (version manager), `tmux-builder` (source builder), `mux-test-support` (hermetic test harness with path guards)
- **Zellij** (`/home/d/study/rust/zellij/`) -- `SenderWithContext<T>` channel fan-in pattern, `ServerOsApi` trait for OS abstraction, per-client Route threads, typed instruction enums
- **ratatui** (`/home/d/study/rust/ratatui/`) -- `ratatui-core`/`ratatui-widgets`/backend splits, `Widget`/`StatefulWidget` traits, backend trait abstraction
- **libtmux** (`/home/d/work/python/libtmux/`) -- `Server`/`Session`/`Window`/`Pane` dataclass hierarchy, `QueryList` with Django-style lookups (`exact`, `icontains`, `startswith`, `regex`, etc.), `pytest_plugin.py` fixture pattern with `TmuxTestServer`, isolated socket names

---

## 2. Workspace Structure and Purity Classifications

The workspace follows the proven vibe-tmux layout (see `/home/d/work/rust/vibe-tmux/Cargo.toml`) with the addition of CRDT, binding test, and version management crates.

```
mux-rs/
├── Cargo.toml                    # workspace root
├── AGENTS.md                     # human + LLM guardrails
├── ARCHITECTURE.md               # snapshot for reviewers
├── notes/
│   ├── plan.md                   # active roadmap
│   ├── completed.md              # archive
│   └── ideas.md                  # future possibilities
│
├── crates/
│   ├── mux-core/                 # PURE: state machine, entity model, Event/Effect
│   ├── mux-proto/                # PURE: imsg codec, control-mode parser
│   ├── mux-view/                 # PURE: view traversal, ViewList, ViewRef
│   ├── mux-query/                # PURE: query DSL, Django-style operators
│   ├── mux-conf/                 # PURE: tmux.conf parser
│   ├── mux-command/              # PURE: command parsing + dispatch table
│   ├── mux-crdt/                 # PURE: CRDT operation log, merge semantics
│   ├── mux-pty/                  # trait: PTY abstraction
│   ├── mux-pty-fake/             # PURE: fake PTY for testing
│   ├── mux-pty-portable/         # IMPURE: portable-pty backend
│   ├── mux-os/                   # IMPURE + UNSAFE: OS glue, SCM_RIGHTS, FDs
│   ├── mux-client/               # IMPURE: imsg protocol client
│   ├── mux-backend/              # IMPURE: backend trait + implementations
│   ├── mux-refresh/              # IMPURE: async store + refresh plumbing
│   ├── mux-api/                  # FACADE: ManagedMux, ServerFacade, re-exports
│   ├── mux-server/               # IMPURE: tmux-compatible server binary
│   ├── mux-cxx/                  # IMPURE: cxx bridge glue
│   ├── mux-telemetry/            # IMPURE: tracing + OTEL
│   ├── mux-otel/                 # IMPURE: OpenTelemetry export
│   ├── mux-pty-diagnostics/      # IMPURE: PTY leak detection
│   └── mux-test-support/         # IMPURE: test harness, path guards, cleanup
│
├── tools/
│   ├── mux-tui/                  # Ratatui-based TUI client
│   ├── tmux-sniff/               # Protocol sniffer (preserves SCM_RIGHTS)
│   ├── tmux-builder/             # Build tmux from source (configure/make)
│   ├── tmux-vm/                  # Version manager (ensure/exec/regress)
│   ├── tmux-worktrees/           # Git worktree management for tmux repo
│   ├── tmux-lint/                # Protocol/behavior linter
│   ├── format-audit/             # Format variable alignment checker
│   ├── regress-audit/            # Regress test coverage tracker
│   ├── tmux-command-audit/       # Command coverage tracker
│   ├── pty-test/                 # PTY leak test runner
│   ├── pty-report/               # PTY diagnostic report analyzer
│   ├── pty-clean/                # Stale PTY cleaner
│   └── mux-doctor/               # System diagnostics tool
│
├── bindings/
│   ├── python/                   # PyO3 + maturin (built via uv)
│   │   ├── src/lib.rs
│   │   ├── python/mux_rs/
│   │   │   ├── __init__.py
│   │   │   ├── server.py         # ManagedMux wrapper
│   │   │   ├── session.py
│   │   │   ├── window.py
│   │   │   ├── pane.py
│   │   │   └── query.py          # QueryList implementation
│   │   └── tests/
│   │       ├── conftest.py       # pytest plugin (fixtures)
│   │       └── test_snapshot.py  # Snapshot test suite
│   ├── node/                     # NAPI-RS (built via pnpm)
│   │   ├── src/lib.rs
│   │   ├── index.d.ts
│   │   ├── __test__/
│   │   │   ├── setup.ts          # vitest fixtures
│   │   │   └── snapshot.test.ts  # Snapshot test suite
│   │   └── package.json
│   └── cpp/                      # cxx bridge + thin C++ wrapper
│       ├── mux.hpp
│       ├── tests/
│       └── CMakeLists.txt
│
└── fixtures/
    ├── protocol/                 # Captured imsg frames
    ├── control/                  # Captured control-mode sessions
    └── snapshots/                # insta snapshot files
```

### Purity Classification Rules

Following the vibe-tmux AGENTS.md non-negotiable (Section 2):

| Classification | Rules | Examples |
|---|---|---|
| **PURE** | No tokio, no async/.await, no nix/libc, no filesystem/sockets/PTYs, no unsafe, no time/rng without injection | `mux-core`, `mux-proto`, `mux-view`, `mux-query`, `mux-crdt`, `mux-conf`, `mux-command`, `mux-pty-fake` |
| **IMPURE** | May use tokio, async, nix, filesystem, sockets | `mux-client`, `mux-backend`, `mux-refresh`, `mux-server`, `mux-pty-portable` |
| **UNSAFE** | Quarantined unsafe blocks with `// SAFETY:` docs | Only `mux-os` (handwritten unsafe) and `mux-cxx` (cxx-generated only) |
| **FACADE** | Re-exports, no novel logic | `mux-api` |

---

## 3. Entity Model

Grounded directly in tmux.h structs from `/home/d/study/c/tmux/tmux.h`.

### 3.1 Core Entities (in `mux-core`)

```rust
// Typed IDs -- SlotMap-based for local, string-prefixed for tmux
// Matches tmux convention: $<num> for sessions, @<num> for windows, %<num> for panes
pub struct SessionId(slotmap::DefaultKey);
pub struct WindowId(slotmap::DefaultKey);
pub struct PaneId(slotmap::DefaultKey);
pub struct ClientId(slotmap::DefaultKey);
pub struct JobId(slotmap::DefaultKey);

// Dual ID for bridging local and tmux worlds
pub enum ViewId {
    Local(u64),
    Tmux(Arc<str>),  // "$1", "@3", "%5"
}
```

#### Session (from `struct session` at tmux.h:1428)

```rust
pub struct Session {
    pub id: SessionId,
    pub name: String,
    pub cwd: PathBuf,
    pub creation_time: Timestamp,
    pub last_attached_time: Timestamp,
    pub activity_time: Timestamp,
    pub windows: Vec<WindowId>,      // ordered winlinks (tmux: RB_HEAD winlinks)
    pub active_window: WindowId,     // tmux: curw
    pub last_windows: Vec<WindowId>, // tmux: lastw stack
    pub options: SessionOptions,
    pub environ: Environ,
    pub flags: SessionFlags,         // SESSION_ALERTED etc.
    pub attached: u32,               // tmux: attached count
    pub group: Option<SessionGroupId>,
}
```

#### Window (from `struct window` at tmux.h:1276)

```rust
pub struct Window {
    pub id: WindowId,
    pub name: String,
    pub panes: Vec<PaneId>,
    pub active_pane: PaneId,
    pub last_panes: Vec<PaneId>,
    pub layout_root: LayoutCell,
    pub size: Size,                  // sx, sy
    pub flags: WindowFlags,         // WINDOW_BELL, WINDOW_ZOOMED, etc.
    pub options: WindowOptions,
    pub activity_time: Timestamp,
}
```

#### Pane (from `struct window_pane` at tmux.h:1181)

```rust
pub struct Pane {
    pub id: PaneId,
    pub window: WindowId,
    pub size: Size,                  // sx, sy
    pub offset: Position,            // xoff, yoff
    pub flags: PaneFlags,           // PANE_EXITED, PANE_FOCUSED, etc.
    pub pid: Option<u32>,
    pub tty: String,
    pub shell: String,
    pub cwd: PathBuf,
    pub start_command: String,
    pub exit_status: Option<i32>,
    pub screen: Screen,             // grid + cursor + mode
    pub layout_cell: LayoutCellId,
    pub options: PaneOptions,
    pub mode_stack: Vec<WindowMode>, // tmux: TAILQ modes
}
```

#### Client (from `struct client` at tmux.h -- later in file)

```rust
pub struct Client {
    pub id: ClientId,
    pub name: String,
    pub session: Option<SessionId>,
    pub last_session: Option<SessionId>,
    pub tty: TtyState,
    pub flags: ClientFlags,         // CLIENT_CONTROL, CLIENT_READONLY, etc.
    pub key_table: String,
    pub environ: Environ,
}
```

#### Grid (from `struct grid` at tmux.h:836)

```rust
pub struct Grid {
    pub flags: GridFlags,           // GRID_HISTORY
    pub size: Size,                 // sx, sy
    pub history_scrolled: u32,      // hscrolled
    pub history_size: u32,          // hsize
    pub history_limit: u32,         // hlimit
    pub lines: Vec<GridLine>,
}

pub struct GridLine {
    pub cells: Vec<GridCell>,
    pub flags: LineFlags,
    pub time: Timestamp,
}
```

### 3.2 ServerGraph (Authoritative State)

```rust
// In mux-core: the single source of truth
pub struct ServerGraph {
    pub sessions: SlotMap<SessionId, Session>,
    pub windows: SlotMap<WindowId, Window>,
    pub panes: SlotMap<PaneId, Pane>,
    pub clients: SlotMap<ClientId, Client>,
    pub jobs: SlotMap<JobId, Job>,
    pub session_groups: Vec<SessionGroup>,
    pub options: GlobalOptions,
    pub buffers: Vec<PasteBuffer>,
    pub marked_pane: Option<MarkedPane>,  // tmux: server.c marked_pane
}
```

---

## 4. Event/Effect Engine Design

Following the vibe-tmux principle: **functional core, imperative shell** (AGENTS.md Section 2).

### 4.1 Event Types (inputs to the core)

```rust
pub enum Event {
    // Protocol events (from imsg client)
    ProtocolMessage(ImsgFrame),
    ProtocolDisconnect,

    // Control-mode hints (from tmux -C)
    ControlNotification(ControlNotif),

    // Commands (from clients, bindings, config)
    Command(CommandEvent),

    // PTY events
    PtyOutput { pane: PaneId, data: Bytes },
    PtyExit { pane: PaneId, status: i32 },

    // Client events
    ClientInput { client: ClientId, key: KeyEvent },
    ClientResize { client: ClientId, size: Size },
    ClientAttach { client: ClientId },
    ClientDetach { client: ClientId },

    // Timer events (injected by runtime)
    Tick(Timestamp),
    NameRefresh,

    // CRDT events (collaborative)
    CrdtMerge(OpLog),
}
```

### 4.2 Effect Types (outputs from the core)

```rust
pub enum Effect {
    // PTY control
    SpawnPty { pane: PaneId, command: String, cwd: PathBuf },
    WritePty { pane: PaneId, data: Bytes },
    ResizePty { pane: PaneId, size: Size },
    ClosePty { pane: PaneId },

    // Client communication
    SendToClient { client: ClientId, msg: ImsgFrame },
    RedrawClient { client: ClientId },
    NotifyControl { client: ClientId, notification: String },

    // Socket operations
    AcceptClient,
    DisconnectClient { client: ClientId, reason: String },

    // Server lifecycle
    Shutdown,

    // State change signals
    RefreshHint(RefreshSet),

    // CRDT operations
    CrdtEmit(Op),

    // Timer scheduling
    ScheduleTimer { id: TimerId, delay: Duration },

    // Hook dispatch
    RunHook { hook: HookName, args: Vec<String> },
}
```

### 4.3 Core Transition Function

```rust
// In mux-core -- PURE, deterministic, no IO
impl ServerGraph {
    pub fn apply(&mut self, event: Event) -> Vec<Effect> {
        match event {
            Event::Command(cmd) => self.dispatch_command(cmd),
            Event::PtyOutput { pane, data } => self.handle_pty_output(pane, data),
            Event::ClientInput { client, key } => self.handle_key(client, key),
            Event::CrdtMerge(log) => self.merge_crdt(log),
            // ...
        }
    }
}
```

### 4.4 Runtime Effect Executor

```rust
// In mux-server (IMPURE) -- executes effects against real IO
pub struct EffectExecutor {
    pty_manager: PtyManager,
    client_connections: HashMap<ClientId, ClientConnection>,
    socket_listener: UnixListener,
}

impl EffectExecutor {
    pub async fn execute(&mut self, effect: Effect, graph: &mut ServerGraph) {
        match effect {
            Effect::SpawnPty { pane, command, cwd } => {
                let pty = self.pty_manager.spawn(&command, &cwd).await;
                // Feed resulting event back into graph
            }
            Effect::SendToClient { client, msg } => {
                if let Some(conn) = self.client_connections.get_mut(&client) {
                    conn.send(msg).await;
                }
            }
            // ...
        }
    }
}
```

---

## 5. Protocol Codec Design

Grounded in `/home/d/study/c/tmux/tmux-protocol.h` and vibe-tmux's `mux-proto` crate.

### 5.1 imsg Framing

tmux uses OpenBSD's `imsg` framing. From `tmux-protocol.h`, `PROTOCOL_VERSION 8`.

```rust
// 16-byte header, native endian
pub struct ImsgHdr {
    pub type_: u32,      // enum msgtype value
    pub len: u32,        // total length including header; high bit = IMSG_FD_FLAG
    pub flags: u16,
    pub peerid: u16,
    pub pid: u32,
}

pub const IMSG_HEADER_SIZE: usize = 16;
pub const IMSG_FD_FLAG: u32 = 1 << 31;
pub const IMSG_MAX_LEN: usize = 16384;
pub const IMSG_MIN_LEN: usize = IMSG_HEADER_SIZE;
```

### 5.2 Message Types (from `enum msgtype` in tmux-protocol.h)

```rust
#[repr(u32)]
pub enum MsgType {
    Version = 12,

    // Identify sequence (client.c:client_send_identify)
    IdentifyFlags = 100,
    IdentifyTerm = 101,
    IdentifyTtyname = 102,
    IdentifyOldcwd = 103,  // unused
    IdentifyStdin = 104,
    IdentifyEnviron = 105,
    IdentifyDone = 106,
    IdentifyClientpid = 107,
    IdentifyCwd = 108,
    IdentifyFeatures = 109,
    IdentifyStdout = 110,
    IdentifyLongflags = 111,
    IdentifyTerminfo = 112,

    // Command/lifecycle
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

### 5.3 Codec Implementation

```rust
// Two-phase decode (following vibe-tmux mux-proto pattern)
pub struct ImsgCodec {
    pending_header: Option<ImsgHdr>,
}

impl Decoder for ImsgCodec {
    type Item = ImsgFrame;
    type Error = ProtocolError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<ImsgFrame>, ProtocolError> {
        // Phase 1: Read header if not pending
        if self.pending_header.is_none() {
            if src.len() < IMSG_HEADER_SIZE { return Ok(None); }
            let hdr = ImsgHdr::parse(&src[..IMSG_HEADER_SIZE])?;
            // Validate: length in [16..=16384]
            let payload_len = hdr.payload_len();
            if payload_len > IMSG_MAX_LEN - IMSG_HEADER_SIZE {
                return Err(ProtocolError::FrameTooLarge);
            }
            src.advance(IMSG_HEADER_SIZE);
            self.pending_header = Some(hdr);
        }

        // Phase 2: Read payload
        let hdr = self.pending_header.as_ref().unwrap();
        let payload_len = hdr.payload_len();
        if src.len() < payload_len { return Ok(None); }
        let payload = src.split_to(payload_len).freeze();
        let hdr = self.pending_header.take().unwrap();
        Ok(Some(ImsgFrame { hdr, payload }))
    }
}
```

### 5.4 Identify Sequence

Ordering must match `client.c:client_send_identify` (validated against tmux source):
LongFlags -> LongFlags -> Term -> Features -> Ttyname -> Cwd -> Terminfo* -> Stdin -> Stdout -> Clientpid -> Environ* -> Done

Note the double `MSG_IDENTIFY_LONGFLAGS` (first `flags`, then `client_flags`), as documented in vibe-tmux `notes/ideas.md`.

### 5.5 Control Mode Notifications

From `/home/d/study/c/tmux/control-notify.c`:

| Function | Notification String | Payload |
|---|---|---|
| `control_notify_pane_mode_changed` | `%pane-mode-changed %%%u` | pane_id |
| `control_notify_window_layout_changed` | `%layout-change #{window_id}...` | formatted layout data |
| `control_notify_window_pane_changed` | `%window-pane-changed @%u %%%u` | window_id, pane_id |
| `control_notify_window_unlinked` | `%window-close @%u` or `%unlinked-window-close @%u` | window_id |
| `control_notify_window_linked` | `%window-add @%u` or `%unlinked-window-add @%u` | window_id |
| `control_notify_window_renamed` | `%window-renamed @%u %s` | window_id, name |
| `control_notify_client_session_changed` | `%session-changed $%u %s` or `%client-session-changed %s $%u %s` | varies |
| `control_notify_client_detached` | `%client-detached %s` | client_name |
| `control_notify_session_renamed` | `%session-renamed $%u %s` | session_id, name |
| `control_notify_session_created` | `%sessions-changed` | none |
| `control_notify_session_closed` | `%sessions-changed` | none |
| `control_notify_session_window_changed` | `%session-window-changed $%u @%u` | session_id, window_id |
| `control_notify_paste_buffer_changed` | `%paste-buffer-changed %s` | name |
| `control_notify_paste_buffer_deleted` | `%paste-buffer-deleted %s` | name |

Key insight: `%layout-change` is the **only notification with structured data** (uses `format_single`). All others are ID-only hints. This validates the vibe-tmux authority model: control mode is hints only, imsg protocol is authoritative.

---

## 6. ORM-like Query API

Grounded in libtmux's `QueryList` (`/home/d/work/python/libtmux/src/libtmux/_internal/query_list.py`).

### 6.1 Query Operators (from libtmux's LOOKUP_NAME_MAP)

```rust
// In mux-query (PURE)
pub enum QueryOp {
    Exact,
    Iexact,
    Contains,
    Icontains,
    Startswith,
    Istartswith,
    Endswith,
    Iendswith,
    In,
    Nin,
    Regex,
    Iregex,
}
```

### 6.2 Typed Query Selectors

```rust
// Generated by query_fields! macro or derive(Queryable)
pub mod session {
    pub fn name() -> FieldSelector<String> { FieldSelector::new("name") }
    pub fn id() -> FieldSelector<SessionId> { FieldSelector::new("id") }
}

pub mod window {
    pub fn name() -> FieldSelector<String> { FieldSelector::new("name") }
    pub fn active() -> FieldSelector<bool> { FieldSelector::new("active") }
}

pub mod pane {
    pub fn active() -> FieldSelector<bool> { FieldSelector::new("active") }
    pub fn start_command() -> FieldSelector<String> { FieldSelector::new("start_command") }
    pub fn pid() -> FieldSelector<u32> { FieldSelector::new("pid") }
}
```

### 6.3 ViewList and QuerySet

```rust
pub struct ViewList<'a, T> {
    items: &'a [T],
    index: &'a ViewIndex,
}

impl<'a, T: Queryable> ViewList<'a, T> {
    pub fn filter(&self, predicate: impl QueryPredicate<T>) -> ViewList<'a, T>;
    pub fn get(&self, predicate: impl QueryPredicate<T>) -> Result<&T, QueryError>;
    pub fn get_one(&self) -> Result<&T, QueryError>;  // errors on 0 or 2+
    pub fn first(&self) -> Option<&T>;
    pub fn count(&self) -> usize;
}
```

### 6.4 Usage Across Languages

**Rust:**
```rust
let view = server.view()?;
let session = view.sessions()
    .get(session::name().iexact("demo"))?;
let pane = session.windows()
    .get(window::name().startswith("editor"))?
    .panes()
    .get(pane::active().is_true())?;
```

**Python (PyO3):**
```python
server = ManagedMux()
session = server.sessions.get(name__iexact="demo")
pane = session.windows.get(name__startswith="editor").panes.get(active=True)
```

**Node (NAPI-RS):**
```typescript
const server = new ManagedMux();
const session = server.sessions.filter({ name__icontains: "demo" }).getOne();
const pane = session.windows.filter({ name__startswith: "editor" }).getOne().panes.filter({ active: true }).getOne();
```

**C++ (cxx):**
```cpp
auto view = server.view();
auto session = view.sessions()
    .filter(mux::query::session::name().icontains("demo"))
    .get_one();
```

---

## 7. Language Bindings

### 7.1 Python (PyO3 + maturin)

Following the vibe-tmux pattern (`bindings/python/`) and libtmux's API shape.

```python
# bindings/python/python/mux_rs/server.py
class ManagedMux:
    """Rust-backed tmux server connection."""

    def __init__(self, socket_path=None, socket_name=None):
        self._inner = _mux_rs.ManagedMux(socket_path, socket_name)

    @property
    def sessions(self) -> QueryList[Session]:
        """Returns sessions (filtering runs in Rust)."""
        return QueryList([Session(s) for s in self._inner.sessions()])

    def cmd(self, command: str) -> CommandResult:
        return self._inner.cmd(command)

    def subscribe(self) -> Iterator[RefreshHint]:
        """Stream-first: yields RefreshHint on state changes."""
        return self._inner.subscribe()
```

Build: `cd bindings/python && uv run maturin develop`

### 7.2 Node (NAPI-RS)

```typescript
// bindings/node/index.d.ts
export class ManagedMux {
    constructor(socketPath?: string, socketName?: string);
    get sessions(): QueryList<Session>;
    cmd(command: string): CommandResult;
    subscribe(): AsyncIterableIterator<RefreshHint>;
}

export class QueryList<T> {
    filter(criteria: Record<string, any>): QueryList<T>;
    getOne(): T;
    first(): T | null;
    count(): number;
}
```

Build: `cd bindings/node && pnpm build`

### 7.3 C++ (cxx bridge)

Following vibe-tmux's `mux-cxx` pattern -- cxx bridge generates the unsafe glue, thin C++ wrapper in `bindings/cpp/mux.hpp`.

```cpp
// bindings/cpp/mux.hpp
namespace mux {
    class ManagedMux {
    public:
        explicit ManagedMux(std::optional<std::string> socket_path = std::nullopt);
        ViewHandle view() const;
        CommandResult cmd(std::string_view command);
    };

    class ViewHandle {
        QueryList<Session> sessions() const;
        QueryList<Window> windows() const;
        QueryList<Pane> panes() const;
    };
}
```

### 7.4 Binding Naming Policy

From vibe-tmux AGENTS.md: "Bindings must expose a clean tmux object graph (sessions/windows/panes/clients/jobs + commands). Do not expose frame, store, or JSON concepts in public method/type names."

---

## 8. CRDT Transaction Layer

### 8.1 Operation Log

```rust
// In mux-crdt (PURE)
pub struct OpLog {
    pub ops: Vec<Op>,
    pub site_id: SiteId,
    pub lamport_clock: u64,
}

pub struct Op {
    pub id: OpId,          // (site_id, lamport_clock)
    pub timestamp: Timestamp,
    pub kind: OpKind,
}

pub enum OpKind {
    CreateSession { name: String, options: SessionOptions },
    DestroySession { session: SessionId },
    CreateWindow { session: SessionId, name: String },
    DestroyWindow { window: WindowId },
    SplitPane { window: WindowId, direction: Direction },
    DestroyPane { pane: PaneId },
    RenameSession { session: SessionId, name: String },
    RenameWindow { window: WindowId, name: String },
    ResizePane { pane: PaneId, size: Size },
    SetOption { scope: OptionScope, key: String, value: OptionValue },
    SelectWindow { session: SessionId, window: WindowId },
    SelectPane { window: WindowId, pane: PaneId },
}
```

### 8.2 Merge Semantics

```rust
impl ServerGraph {
    pub fn merge_crdt(&mut self, remote_log: OpLog) -> Vec<Effect> {
        let mut effects = Vec::new();
        for op in remote_log.ops {
            if self.has_seen(&op.id) { continue; }
            // Last-writer-wins for conflicting renames
            // Set-union for session/window/pane creation
            // Causal ordering via Lamport timestamps
            let local_effects = self.apply_crdt_op(&op);
            effects.extend(local_effects);
            self.mark_seen(op.id);
        }
        effects
    }
}
```

### 8.3 Conflict Resolution

| Operation | Conflict Type | Resolution |
|---|---|---|
| Rename | Two sites rename same entity | Last-writer-wins (higher Lamport clock) |
| Create + Destroy | One site creates, another destroys | Destroy wins (tombstone) |
| Split + Destroy | Split a pane that was destroyed | Split is no-op (parent gone) |
| Resize | Two sites resize same pane | Last-writer-wins |
| Select | Two sites select different windows | Local selection wins (client-relative) |

---

## 9. tmux Version Manager Tool

Grounded in vibe-tmux's `tmux-vm` (`/home/d/work/rust/vibe-tmux/tools/tmux-vm/src/main.rs`) and `tmux-builder` (`/home/d/work/rust/vibe-tmux/tools/tmux-builder/src/lib.rs`).

### 9.1 tmux-builder (library crate)

Responsibilities:
- **OS detection**: `parse_os_release()` reads `/etc/os-release`, returns `OsRelease { id, version_id, pretty_name }`
- **Dependency planning**: `dep_plan()` returns distro-specific package lists (e.g., `build-essential`, `autoconf`, `libevent-dev`, `libncurses-dev`)
- **Build orchestration**: `build_tmux()` runs `autogen.sh` -> `configure` -> `make` with configurable flags
- **Install**: `install_tmux()` runs `make install` with optional `DESTDIR`
- **Cache key computation**: `compute_cache_key()` hashes version + host triple + OS + configure/make flags using blake3, producing deterministic cache paths like `tmux-3.4__x86_64-unknown-linux__ubuntu-24.04__cfg<hash>__mk<hash>__tb<version>`
- **Atomic publish**: Build into a `.tmp-*` sibling directory, rename atomically to final path
- **Concurrency safety**: File-based locks via `fs2::FileExt::lock_exclusive()` for both repo cloning and per-version builds

Key function: `ensure_tmux_binary()` -- the main entry point that clones/opens the tmux git repo, creates a worktree for the version tag, builds, installs, validates (`tmux -V`), and writes a `manifest.json`.

### 9.2 tmux-vm (CLI tool)

Subcommands:
- **`ls`**: List tmux versions from git tags (filters to version-like tags: `X.Y`, `X.Ya`)
- **`ensure`**: Build and cache a specific tmux version
- **`exec`**: Run a command with a versioned tmux in PATH (sets `PATH` and `TMUX_BIN`)
- **`regress`**: Run upstream tmux regress scripts against a versioned binary in an isolated `TMUX_TMPDIR`

Regress isolation pattern (from tmux-vm source):
```rust
cmd.env("TMUX_TMPDIR", tmp.path());
cmd.env("TMPDIR", tmp.path());
cmd.env("HOME", tmp.path());
cmd.env("XDG_CONFIG_HOME", tmp.path());
cmd.env("XDG_DATA_HOME", tmp.path());
```

### 9.3 tmux-worktrees (support crate)

Manages git worktrees for the tmux repository:
- `open_repo()` -- opens or clones the tmux git repository
- `collect_tags()` -- lists all tags from the repository
- `resolve_tag()` -- resolves a version string to a concrete tag
- `create_worktree()` -- creates a git worktree for a specific tag
- `expand_template()` -- expands `~` and `{tag}` placeholders in path templates

---

## 10. Test Framework and Harness Design

### 10.1 Test Categories

| Category | Location | Purity | Strategy |
|---|---|---|---|
| Core unit tests | `crates/mux-core/tests/` | PURE | Event -> Effect deterministic tests |
| Protocol tests | `crates/mux-proto/tests/` | PURE | Fixture-based decode/encode + chunking |
| Query tests | `crates/mux-query/tests/` | PURE | Operator coverage + edge cases |
| CRDT tests | `crates/mux-crdt/tests/` | PURE | Merge semantics + conflict resolution |
| Integration tests | `crates/mux-server/tests/` | IMPURE | Hermetic temp dirs + isolated sockets |
| Parity tests | `crates/mux-client/tests/parity.rs` | IMPURE | Side-by-side tmux vs mux-server |
| Snapshot tests | Throughout | PURE/IMPURE | insta snapshots for output/rendering |
| PTY tests | `tools/pty-test/` | IMPURE | Leak detection + strict diagnostics |
| Regress tests | `tools/tmux-vm regress` | IMPURE | Upstream tmux regress scripts |

### 10.2 PTY Mocking (mux-pty-fake)

```rust
// crates/mux-pty-fake/ (PURE -- no real PTYs)
pub struct FakePty {
    input_buffer: VecDeque<u8>,   // data written to PTY (from pane)
    output_buffer: VecDeque<u8>,  // data to read from PTY (simulated program)
    size: Size,
    closed: bool,
}

pub trait PtyBackend: Send {
    fn spawn(&self, cmd: &str, cwd: &Path, size: Size) -> Result<Box<dyn PtyHandle>>;
}

pub trait PtyHandle: Send {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>;
    fn write(&mut self, data: &[u8]) -> Result<usize>;
    fn resize(&mut self, size: Size) -> Result<()>;
    fn tty_name(&self) -> &str;
    fn pid(&self) -> Option<u32>;
    fn wait(&mut self) -> Result<ExitStatus>;
}

// For tests: FakePtyBackend returns FakePty instances with scripted output
pub struct FakePtyBackend {
    scripts: HashMap<String, Vec<FakePtyAction>>,
}

pub enum FakePtyAction {
    Output(Bytes),
    Delay(Duration),
    Exit(i32),
}
```

Test backend selection (following vibe-tmux pattern): `MUX_PTY_TEST_BACKEND=os|portable|fake`

### 10.3 Hermetic Test Server (from mux-test-support)

Directly based on `/home/d/work/rust/vibe-tmux/crates/mux-test-support/src/tmux.rs`:

```rust
pub struct TmuxTestServer {
    tmux_bin: PathBuf,
    tmux_tmpdir: tempfile::TempDir,     // auto-cleaned on drop
    socket_name: String,                 // never "default"
    socket_path: PathBuf,               // inside tmpdir
    session_name: String,
}

impl TmuxTestServer {
    pub fn start() -> Result<Self>;
    pub fn tmux(&self, args: &[&str]) -> Result<Output>;
    pub fn is_alive(&self) -> bool;
    pub fn shutdown(&mut self) -> Result<()>;
}

impl Drop for TmuxTestServer {
    fn drop(&mut self) { let _ = self.shutdown(); }
}
```

### 10.4 Path Guards (from mux-test-support/path_guard.rs)

Safety invariants that prevent accidental destruction of real tmux sessions:

```rust
pub fn ensure_not_default_socket_name(name: &str) -> Result<()>;
pub fn ensure_socket_within_tempdir(socket: &Path, tmpdir: &Path) -> Result<()>;
pub fn ensure_socket_not_tmux_env(socket: &Path) -> Result<()>;
pub fn ensure_socket_not_tmux_env_value(socket: &Path, tmux_env: Option<&str>) -> Result<()>;
```

### 10.5 Unified TestServer (from mux-test-support/server.rs)

```rust
pub enum TestServer {
    Tmux(TmuxTestServer),
    Mux(MuxServerTestServer),
}

impl TestServer {
    pub fn start_tmux() -> Result<Self>;
    pub fn start_mux() -> Result<Self>;
    pub fn tmux(&self, args: &[&str]) -> Result<Output>;  // unified interface
    pub fn shutdown(&mut self) -> Result<()>;
}
```

### 10.6 Snapshot Testing

Using `insta` with YAML serialization (following vibe-tmux `Cargo.toml` dependency):

```rust
#[test]
fn test_event_produces_expected_effects() {
    let mut graph = ServerGraph::default();
    // Setup initial state...
    let effects = graph.apply(Event::Command(CommandEvent {
        command: "split-window -h".to_string(),
        // ...
    }));
    insta::assert_yaml_snapshot!(effects);
}

#[test]
fn test_protocol_frame_decode() {
    let raw = include_bytes!("../fixtures/protocol/identify_burst.bin");
    let frames = decode_all(raw);
    insta::assert_yaml_snapshot!(frames);
}
```

### 10.7 Property-Based Testing

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn chunked_decode_matches_full_decode(
        frame in arb_imsg_frame(),
        split_points in prop::collection::vec(0..256usize, 1..10)
    ) {
        let full = decode_single(&frame.to_bytes());
        let chunked = decode_chunked(&frame.to_bytes(), &split_points);
        prop_assert_eq!(full, chunked);
    }
}
```

---

## 11. Binding Test Frameworks

### 11.1 Python pytest Plugin

Modeled on libtmux's `pytest_plugin.py` (`/home/d/work/python/libtmux/src/libtmux/pytest_plugin.py`):

```python
# bindings/python/python/mux_rs/pytest_plugin.py
import pytest
from mux_rs import ManagedMux

@pytest.fixture(scope="session")
def mux_tmpdir(tmp_path_factory):
    """Isolated TMUX_TMPDIR for the test session."""
    return tmp_path_factory.mktemp("mux-rs-test")

@pytest.fixture
def mux_server(request, mux_tmpdir):
    """Hermetic ManagedMux server for a single test."""
    import os, uuid
    socket_name = f"mux-rs-test-{uuid.uuid4().hex[:8]}"
    server = ManagedMux(socket_name=socket_name, tmpdir=str(mux_tmpdir))

    def cleanup():
        server.kill_server()  # safety: only kills our isolated socket

    request.addfinalizer(cleanup)
    return server

@pytest.fixture
def mux_session(mux_server):
    """Create a session on the test server."""
    return mux_server.new_session("pytest-session")
```

Snapshot tests using `pytest-snapshot` or `syrupy`:

```python
# bindings/python/tests/test_snapshot.py
def test_sessions_snapshot(mux_server, snapshot):
    session = mux_server.new_session("snap-test")
    window = session.new_window("editor")
    state = mux_server.state_snapshot()
    snapshot.assert_match(state.to_json(), "sessions_basic.json")

def test_split_pane_snapshot(mux_session, snapshot):
    pane = mux_session.active_window.split(direction="horizontal")
    state = mux_session.server.state_snapshot()
    snapshot.assert_match(state.to_json(), "split_pane.json")
```

### 11.2 Node vitest Plugin

```typescript
// bindings/node/__test__/setup.ts
import { beforeAll, afterAll, beforeEach, afterEach } from 'vitest';
import { ManagedMux, type Session } from '../index';
import { mkdtemp } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { randomBytes } from 'crypto';

let testTmpdir: string;
let testServer: ManagedMux;

export function useMuxServer() {
    beforeEach(async (ctx) => {
        testTmpdir = await mkdtemp(join(tmpdir(), 'mux-rs-test-'));
        const socketName = `mux-rs-test-${randomBytes(4).toString('hex')}`;
        testServer = new ManagedMux({ socketName, tmpdir: testTmpdir });
    });

    afterEach(async () => {
        testServer.killServer();
    });

    return () => testServer;
}
```

Snapshot tests using vitest's built-in snapshot:

```typescript
// bindings/node/__test__/snapshot.test.ts
import { describe, it, expect } from 'vitest';
import { useMuxServer } from './setup';

describe('ManagedMux snapshots', () => {
    const getServer = useMuxServer();

    it('creates session', () => {
        const server = getServer();
        const session = server.newSession('snap-test');
        const state = server.stateSnapshot();
        expect(state).toMatchSnapshot();
    });

    it('splits pane horizontally', () => {
        const server = getServer();
        const session = server.newSession('split-test');
        session.activeWindow.split({ direction: 'horizontal' });
        const state = server.stateSnapshot();
        expect(state).toMatchSnapshot();
    });
});
```

### 11.3 PTY Snapshot Comparisons

For both Python and Node, provide helpers to capture and compare PTY output:

```python
# Python
def test_pty_output_snapshot(mux_session, snapshot):
    pane = mux_session.active_pane
    pane.send_keys("echo hello")
    pane.wait_for_text("hello", timeout=2.0)
    capture = pane.capture_pane()
    snapshot.assert_match(capture, "echo_hello.txt")
```

```typescript
// Node
it('captures PTY output', async () => {
    const pane = session.activePan;
    await pane.sendKeys('echo hello');
    await pane.waitForText('hello', { timeout: 2000 });
    const capture = pane.capturePane();
    expect(capture).toMatchSnapshot();
});
```

---

## 12. Visual Client (TUI)

Based on vibe-tmux's `mux-tui` (`/home/d/work/rust/vibe-tmux/tools/mux-tui/`) and ratatui patterns.

### 12.1 Architecture

```
mux-tui/
├── src/
│   ├── main.rs         # Event loop: UiEvent channel fan-in
│   ├── app.rs          # App state + update logic
│   ├── ui.rs           # Ratatui render functions
│   ├── input.rs        # Key mapping + action dispatch
│   ├── tree.rs         # Session/window/pane tree builder
│   ├── config.rs       # TOML configuration
│   ├── backend.rs      # Socket discovery + connection
│   ├── logging.rs      # File-based logging
│   └── otel.rs         # Optional OpenTelemetry
```

### 12.2 Event-Driven Loop

Following the vibe-tmux pattern (stream-first, no polling for data):

```rust
enum UiEvent {
    Input(KeyEvent),
    Tick,                         // UI cadence only (10ms default)
    Draw,                         // Explicit redraw request
    ManagedInit(SocketId, Result<ManagedMuxHandle>),
    ManagedHint(SocketId, RefreshHint),
    SocketDiscovered(PathBuf),
    SocketRemoved(PathBuf),
}

fn main_loop(event_rx: Receiver<UiEvent>, terminal: &mut Terminal) {
    loop {
        let event = event_rx.recv()?;  // blocks on channel, not polling
        // Drain queued events
        let mut events = vec![event];
        while let Ok(e) = event_rx.try_recv() { events.push(e); }
        // Process all, then single draw
        for event in events { app.update(event); }
        terminal.draw(|f| app.render(f))?;
    }
}
```

### 12.3 Widget Structure

Using ratatui's `Widget`/`StatefulWidget` traits:

```rust
pub struct PaneWidget<'a> {
    grid: &'a Grid,
    focused: bool,
    border_style: Style,
}

impl Widget for PaneWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Render grid cells into ratatui buffer
        // Map tmux grid_cell attributes to ratatui Style
    }
}

pub struct StatusLineWidget<'a> {
    format: &'a str,
    context: &'a FormatContext,
}
```

### 12.4 tmux Compatibility

The TUI client must be interoperable: a real tmux client can attach to our server, and our TUI can attach to a real tmux server. Both paths use the same imsg protocol.

---

## 13. DOs and DON'Ts

### DOs

1. **DO** keep `mux-core` pure: no tokio, no async, no IO, no unsafe, no time/rng without injection.
2. **DO** use typed IDs (`SessionId`, `WindowId`, `PaneId`) with SlotMap, never raw `usize` or `u32`.
3. **DO** parse protocol frames explicitly (field-by-field, validate lengths before reading, cap allocations).
4. **DO** document every `unsafe` block with `// SAFETY:` explaining the invariant, and write a test that would fail if the invariant breaks.
5. **DO** quarantine all unsafe in `mux-os` (handwritten) or `mux-cxx` (cxx-generated only).
6. **DO** use `Result` everywhere in hot paths; close the client and log with context instead of panicking.
7. **DO** use hermetic test sockets with per-test `TMUX_TMPDIR`, never the developer's real tmux socket.
8. **DO** run path guards (`ensure_not_default_socket_name`, `ensure_socket_within_tempdir`, `ensure_socket_not_tmux_env`) before every destructive operation.
9. **DO** prefer typed streams (channels/async streams) over tick-based polling for all data paths.
10. **DO** treat control mode as hints only; the imsg binary protocol is the authority for structured data.
11. **DO** use `insta` for snapshot tests and `proptest` for property-based tests on protocol parsing.
12. **DO** keep the identify burst ordering aligned with `client.c:client_send_identify` (validate against tmux source).
13. **DO** use atomic directory renames for build artifacts (build in `.tmp-*`, rename to final).
14. **DO** use file-based locks (`fs2::FileExt`) for concurrent build/test safety.
15. **DO** run all query/filter logic in Rust; bindings merge criteria but never implement matching.
16. **DO** hide internal names (frame, store, JSON) from binding public APIs.
17. **DO** use `just check-llm` (or equivalent quality gate) before every commit.
18. **DO** keep the CRDT operation log append-only; resolve conflicts with deterministic rules.
19. **DO** close FDs on drop and validate expected FD counts for SCM_RIGHTS messages.
20. **DO** design all cleanup for test servers using RAII (`Drop` impls on `TmuxTestServer`, `MuxServerTestServer`).
21. **DO** use `env_remove("TMUX")` on every test subprocess to prevent inheritance from the developer's session.

### DON'Ts

1. **DON'T** ever run `kill-server` against the developer's real tmux socket (enforce with `VIBE_TMUX_GUARD_KILL_SERVER=1`).
2. **DON'T** use `unwrap()`, `expect()`, or `panic!()` in protocol decode/encode, server loop, or PTY loop.
3. **DON'T** use `#[repr(packed)]` -- explicit field parsing only.
4. **DON'T** cast bytes to struct for protocol decoding (parse explicitly, validate, then construct).
5. **DON'T** use serde-based IPC for tmux compatibility surfaces.
6. **DON'T** mutate process environment in tests -- use dependency injection or per-subprocess env.
7. **DON'T** add new crates for migration work; add modules within `mux-api` for connection/refresh/discover/store.
8. **DON'T** expose `View`, `ViewHandle`, `Frame`, `Store`, or `JSON` in binding public APIs.
9. **DON'T** implement filter/matching logic in binding language code (Python/Node/C++) -- always delegate to Rust.
10. **DON'T** use polling/sleep loops for steady-state UI or binding updates -- streams are the default, polling is fallback only.
11. **DON'T** reconnect/re-identify per tick -- maintain one persistent connection per socket per backend.
12. **DON'T** invent protocol message ordering, payload layouts, or handshake shortcuts not backed by tmux source or captured fixtures.
13. **DON'T** use the tmux socket name `"default"` in any test.
14. **DON'T** skip running `tmux -f /dev/null` in test harnesses (prevents user config interference).
15. **DON'T** create sessions that could leak into real tmux without cleanup-on-drop tracking.
16. **DON'T** use overlapping dependency stacks (one PTY strategy, one FD-passing strategy).
17. **DON'T** put async/.await in `mux-core`, `mux-proto`, `mux-view`, `mux-query`, or `mux-crdt`.
18. **DON'T** use `git add -A` or `git add .` -- stage specific files only.
19. **DON'T** run force push to main/master without explicit team approval.
20. **DON'T** use `#![allow(clippy::unwrap_used)]` outside of test code.
21. **DON'T** block the UI thread on IO (all IO must be async or on a worker thread).

---

## 14. AGENTS.md Template

```markdown
# AGENTS.md (human + LLM guardrails)

## Rule 0: Never kill real tmux
- **Never** run `kill-server` against the developer's real tmux socket.
- Always use dedicated test sockets (e.g., `/tmp/mux-rs-*`) and verify
  the socket path before issuing destructive commands.
- Keep the kill-server guard enabled (`MUX_RS_GUARD_KILL_SERVER=1`).

## Mission
Drop-in tmux compatibility. "Works" is not success unless a real tmux
client/server agrees.

## Reference repos
- `~/study/c/tmux` for upstream tmux behavior/source.
- `~/study/rust/{tokio,zellij,ratatui}` for async/actor/UI patterns.
- `~/work/python/libtmux` for control-mode lifecycle, filtering, and
  cleanup expectations in a libtmux-shaped API surface.

## Non-negotiables

### 0) Push-first responsiveness (streams over polling)
- Prefer typed streams over tick-based polling.
- Latency target: ~10-20ms for UI/binding propagation.
- One persistent connection per socket per backend.

### 1) Source-of-truth policy
Protocol/behavior claims must be backed by tmux source or captured fixtures.

### 2) Purity boundary
`mux-core` is PURE: no tokio, no async, no nix/libc, no filesystem, no unsafe.
Input: typed `Event`. Output: typed `Effect` values.

### 3) Unsafe quarantine
Only `mux-os` (handwritten) and `mux-cxx` (cxx-generated).
Every unsafe block: minimal boundary, documented invariants, focused test.
Never use `#[repr(packed)]`.

### 4) Protocol parsing policy
Explicit field parsing. Validate lengths. Cap allocations.
Malformed frames close the connection cleanly.

### 5) SCM_RIGHTS FD passing policy
Forward ancillary data. Bind FDs to frames. Close on drop. Validate counts.

### 6) No panics in hot paths
No `unwrap()`/`expect()` in protocol decode/encode, server loop, PTY loop.

### 7) Testing policy
- Protocol: fixtures + snapshot tests + chunking robustness
- Core: Event -> Effect, deterministic, no IO mocking
- Integration: hermetic temp dirs/sockets, never touch real tmux
- Parity: mux-server vs real tmux side-by-side
- Keep regress/format/command audits green

### 8) CRDT policy
- Operation log is append-only
- Merge is deterministic (same inputs -> same output)
- Conflict resolution: last-writer-wins for values, set-union for collections
- CRDT state is in `mux-core` (PURE); network transport is in runtime

## Dependency policy
Minimal deps, tightly scoped. No overlapping stacks.
No serde-based IPC for tmux compat surfaces.

## Coding conventions
- Inline format args (`format!("{value}")` not `format!("{}", value)`)
- Collapsible if statements
- Method references over redundant closures
- Prefer `pretty_assertions::assert_eq` for clearer test diffs
- Snapshot tests via `cargo insta`

## Git commit format
```
type(scope[detail]) concise description

why: Explanation.
what:
- Change 1
- Change 2
```

## Required before commit
`just check-llm`

## Planning files
- `notes/plan.md`: active roadmap (only remaining work)
- `notes/completed.md`: archived finished work
- `notes/ideas.md`: future possibilities (not committed to)
```

---

## 15. Phased Implementation Plan

### Phase 0: Foundation (Weeks 1-4)

**Goal:** Core state machine, protocol codec, basic server loop.

| Task | Crate | Purity | Acceptance Criteria |
|---|---|---|---|
| Entity model (Session, Window, Pane, Client) | `mux-core` | PURE | Typed IDs, SlotMap storage, `ServerGraph` compiles |
| Event/Effect engine | `mux-core` | PURE | `apply(Event) -> Vec<Effect>` works for create-session, split-window |
| imsg codec (header + payload decode/encode) | `mux-proto` | PURE | Decodes all fixtures from tmux capture; chunking proptest passes |
| Control-mode parser | `mux-proto` | PURE | Parses all 14 notification types from `control-notify.c` |
| Identify sequence builder | `mux-proto` | PURE | Matches `client.c:client_send_identify` ordering |
| PTY trait + fake implementation | `mux-pty`, `mux-pty-fake` | PURE | FakePty passes basic read/write/resize tests |
| OS glue (SCM_RIGHTS, FD passing) | `mux-os` | UNSAFE | Sends/receives FDs with validated counts; all unsafe documented |
| Test harness + path guards | `mux-test-support` | IMPURE | `TmuxTestServer::start()` works; path guards block default socket |

**Acceptance:** `cargo test` passes. No IO in `mux-core`. insta snapshots for protocol decode.

### Phase 1: Server + Client (Weeks 5-8)

**Goal:** A working tmux-compatible server that real tmux clients can attach to.

| Task | Crate | Acceptance Criteria |
|---|---|---|
| Server binary (accept connections, dispatch commands) | `mux-server` | `tmux -S /tmp/test attach` succeeds |
| Command table (30+ commands from `cmd.c`) | `mux-command` | `just command-audit` shows coverage |
| Config parser (`.tmux.conf` subset) | `mux-conf` | Parses `set -g`, `bind-key`, `set-option` |
| Client binary (connect, identify, run commands) | `mux-client` | `mux-client list-sessions` matches tmux output |
| Backend trait + protocol/control implementations | `mux-backend` | Protocol backend reads views; control backend receives hints |
| ManagedMux facade | `mux-api` | `ManagedMux.view()` returns populated `View` |

**Acceptance:** Parity test: `tmux new-session -d && tmux list-sessions` produces identical output on both tmux and mux-server.

### Phase 2: Query API + Bindings (Weeks 9-12)

**Goal:** ORM-like API and thin language bindings.

| Task | Crate | Acceptance Criteria |
|---|---|---|
| Query DSL (all 12 operators) | `mux-query` | Unit tests for every operator; `query_fields!` macro works |
| View model + traversal | `mux-view` | `view.sessions().get(name().iexact("x"))` works |
| Python binding (PyO3) | `bindings/python` | `server.sessions.get(name="test")` returns Session object |
| Node binding (NAPI-RS) | `bindings/node` | `server.sessions.filter({name__icontains:'test'}).getOne()` works |
| C++ binding (cxx bridge) | `bindings/cpp`, `mux-cxx` | `view.sessions().filter(...)` compiles and runs |
| Binding smoke tests | All bindings | `just bindings-smoke` passes |

**Acceptance:** libtmux-shaped API works in all three languages. `just bindings-smoke-diff` shows identical output.

### Phase 3: TUI Client + tmux VM (Weeks 13-16)

**Goal:** Rust-native TUI client and version management tooling.

| Task | Crate | Acceptance Criteria |
|---|---|---|
| mux-tui (ratatui event loop, pane rendering) | `tools/mux-tui` | Can attach to real tmux server and render panes |
| tmux-builder (build from source) | `tools/tmux-builder` | `tmux-builder ensure 3.4` produces working binary |
| tmux-vm (version manager) | `tools/tmux-vm` | `tmux-vm exec 3.4 -- tmux -V` prints "tmux 3.4" |
| tmux-vm regress runner | `tools/tmux-vm` | `tmux-vm regress 3.4` runs all upstream regress scripts |
| Format audit tool | `tools/format-audit` | Tracks format variable coverage against `format.c` |
| Regress audit tool | `tools/regress-audit` | Tracks regress script coverage |

**Acceptance:** `mux-tui` renders correctly when attached to tmux 3.4. `tmux-vm regress 3.4` produces a meaningful pass/fail report.

### Phase 4: CRDT + Test Frameworks (Weeks 17-20)

**Goal:** Collaborative state and comprehensive binding test infrastructure.

| Task | Crate | Acceptance Criteria |
|---|---|---|
| CRDT operation log + merge | `mux-crdt` | Two sites can create/rename sessions and merge deterministically |
| CRDT conflict resolution | `mux-crdt` | All conflict types in Section 8.3 pass |
| Python pytest plugin | `bindings/python` | `@pytest.fixture mux_server` works with hermetic cleanup |
| Python snapshot tests | `bindings/python` | 10+ snapshot tests pass using `syrupy` |
| Node vitest plugin | `bindings/node` | `useMuxServer()` fixture works with hermetic cleanup |
| Node snapshot tests | `bindings/node` | 10+ snapshot tests pass using vitest snapshots |
| OTEL trace chain verification | `mux-telemetry`, `mux-otel` | Zero missing-parent spans across all bindings |

**Acceptance:** `just test-all` green. CRDT merge passes property tests. Both binding test suites produce stable snapshots.

### Phase 5: Polish + Parity (Weeks 21-24)

**Goal:** High tmux parity, production-quality diagnostics.

| Task | Acceptance Criteria |
|---|---|
| Command coverage >90% | `just command-audit` shows <10% gap |
| Format variable coverage >80% | `just format-audit` shows <20% gap |
| Parity test suite: 90%+ pass rate | `just mux-parity-e2e` passes 90%+ |
| PTY diagnostics clean | `pty-test --strict` reports zero leaks |
| mux-doctor system check | Validates tmux binary, socket dirs, permissions, deps |
| Documentation | ARCHITECTURE.md current, binding docs complete |

**Acceptance:** Real tmux clients can attach to mux-server for daily use. All audit tools green.

---

## 16. Risks and Mitigations

| Risk | Impact | Likelihood | Mitigation |
|---|---|---|---|
| **Protocol version drift**: tmux changes imsg framing or message types | High | Medium | Pin to `PROTOCOL_VERSION 8`. Monitor tmux commits. Use `tmux-sniff` to capture new frames. Run parity tests against tmux HEAD weekly. |
| **SCM_RIGHTS portability**: Different platforms handle ancillary data differently | Medium | Medium | Quarantine in `mux-os`. Test on Linux + macOS. Use captured fixtures for offline testing. |
| **PTY leak accumulation**: Fake PTY or OS PTY handles not properly cleaned | High | Medium | `mux-pty-diagnostics` with strict mode. `pty-test --strict` in CI. RAII cleanup in all harnesses. `MUX_PTY_OWNER` tagging for attribution. |
| **Test isolation failure**: Test kills developer's real tmux | Critical | Low | Triple guard: path_guard functions, `env_remove("TMUX")`, unique socket names. Never use socket name "default". |
| **CRDT merge divergence**: Two sites reach different states | High | Medium | Property-based tests for merge commutativity (`merge(A,B) == merge(B,A)`). Deterministic conflict resolution rules. Snapshot tests for all conflict types. |
| **Binding API churn**: Rust API changes break Python/Node/C++ bindings | Medium | High | Facade crate (`mux-api`) as stable boundary. Binding tests in CI. Additive-only schema changes for state payloads. |
| **tmux-builder cross-platform**: Build toolchain differences across distros | Low | Medium | OS detection via `/etc/os-release`. Per-distro `dep_plan()`. Cache key includes host triple + OS version. |
| **Performance regression in event loop**: Polling creep replaces streams | Medium | Medium | Latency target: 10-20ms. Tracing diagnostics (`VIBE_TMUX_TRACE=1`). OTEL spans for refresh pipeline. "If refreshes exceed 20ms, treat as bug." |
| **Control-mode notification taxonomy changes**: tmux adds/removes notifications | Low | Low | `format-audit` and `regress-audit` tools detect drift. Map new notifications to `RefreshSet::full()` as safe default. |
| **Unicode/wide-char rendering bugs**: Grid cell width calculations diverge | Medium | Medium | Use `unicode-width` crate. Test against tmux's `utf8_test` regress script. Snapshot tests for CJK, emoji, combining characters. |

---

## Appendix A: Key File References

| File | Purpose | Key Insight |
|---|---|---|
| `/home/d/study/c/tmux/tmux.h` | Entity definitions | `struct session` (line 1428), `struct window` (1276), `struct window_pane` (1181), `struct grid` (836) |
| `/home/d/study/c/tmux/tmux-protocol.h` | Protocol messages | `PROTOCOL_VERSION 8`, `enum msgtype`, `struct msg_command` |
| `/home/d/study/c/tmux/control-notify.c` | Control notifications | 14 notification types, `%layout-change` is only one with data |
| `/home/d/study/c/tmux/format.c` | Format expansion | `format_job_tree`, `FORMAT_SESSIONS/WINDOWS/PANES` modifiers |
| `/home/d/study/c/tmux/server.c` | Server event loop | `server_loop()`, `server_accept()`, `marked_pane` global |
| `/home/d/study/c/tmux/client.c` | Client handshake | `client_send_identify()`, `client_dispatch()`, exit reasons |
| `/home/d/study/c/tmux/cmd.c` | Command table | 60+ `cmd_entry` declarations, dispatch structure |
| `/home/d/work/rust/vibe-tmux/AGENTS.md` | Guardrails | Purity boundary, unsafe quarantine, testing policy, commit format |
| `/home/d/work/rust/vibe-tmux/ARCHITECTURE.md` | Current architecture | Stream-first facade, SocketActor, lock-free store reads |
| `/home/d/work/rust/vibe-tmux/notes/ideas.md` | API design decisions | Option C (graph + facade + client context), QuerySet, ViewId |
| `/home/d/work/rust/vibe-tmux/tools/tmux-builder/src/lib.rs` | Build system | `ensure_tmux_binary()`, `compute_cache_key()`, atomic publish |
| `/home/d/work/rust/vibe-tmux/tools/tmux-vm/src/main.rs` | Version manager | `ls`, `ensure`, `exec`, `regress` subcommands |
| `/home/d/work/rust/vibe-tmux/crates/mux-test-support/src/tmux.rs` | Test harness | `TmuxTestServer`, hermetic sockets, RAII shutdown |
| `/home/d/work/rust/vibe-tmux/crates/mux-test-support/src/path_guard.rs` | Safety guards | Socket name/path validation, TMUX env checking |
| `/home/d/work/python/libtmux/src/libtmux/_internal/query_list.py` | Query operators | 12 lookup operators, `QueryList` class, `keygetter` for nested access |
| `/home/d/work/python/libtmux/src/libtmux/pytest_plugin.py` | Test fixtures | `server` fixture with isolated socket, `config_file` fixture |
| `/home/d/study/rust/ratatui/ARCHITECTURE.md` | Crate organization | `ratatui-core`/`ratatui-widgets`/backend splits, stability tiers |
| `/home/d/study/rust/zellij/zellij-server/src/lib.rs` | Server architecture | `start_server()`, channel fan-in, per-client Route threads |

## Appendix B: Workspace Dependency Graph

```
mux-core (PURE)
  |
  +-- mux-proto (PURE) --> mux-core
  +-- mux-view (PURE) --> mux-core
  +-- mux-query (PURE) --> mux-core, mux-view
  +-- mux-conf (PURE) --> mux-core
  +-- mux-command (PURE) --> mux-core
  +-- mux-crdt (PURE) --> mux-core
  +-- mux-pty (trait only)
  +-- mux-pty-fake (PURE) --> mux-pty
  |
  +-- mux-os (UNSAFE) --> mux-pty
  +-- mux-pty-portable (IMPURE) --> mux-pty
  +-- mux-client (IMPURE) --> mux-proto, mux-os
  +-- mux-backend (IMPURE) --> mux-client, mux-core
  +-- mux-refresh (IMPURE) --> mux-core, mux-view
  +-- mux-api (FACADE) --> mux-backend, mux-refresh, mux-view, mux-query
  +-- mux-server (IMPURE) --> mux-core, mux-proto, mux-os, mux-command, mux-conf
  |
  +-- mux-cxx (cxx) --> mux-api
  +-- mux-telemetry (IMPURE)
  +-- mux-otel (IMPURE) --> mux-telemetry
  +-- mux-pty-diagnostics (IMPURE) --> mux-pty
  +-- mux-test-support (IMPURE) --> mux-pty-diagnostics
  |
  +-- bindings/python --> mux-api (PyO3)
  +-- bindings/node --> mux-api (NAPI-RS)
  +-- bindings/cpp --> mux-cxx
  |
  +-- tools/mux-tui --> mux-api, ratatui
  +-- tools/tmux-builder (standalone)
  +-- tools/tmux-vm --> tmux-builder, tmux-worktrees
  +-- tools/tmux-sniff --> mux-proto, mux-os
```