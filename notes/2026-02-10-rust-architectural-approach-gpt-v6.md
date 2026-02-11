# TermForge Architectural Plan (v5, GPT-v6)

This v5 plan advances beyond v4 by turning previously abstract areas into implementation-ready specifications:

- A crate-by-crate error taxonomy with explicit propagation rules across the pure/impure boundary, including wire-protocol malformed-frame recovery and `ManagedMux`/`SocketActor` behavior.
- A configuration system deep dive grounded in existing `mux-conf` parsing, including a concrete tmux.conf grammar, option inheritance chain (server -> session -> window -> pane), and hot reload -> Events mechanics with `mux-command` integration.
- A concrete layout engine matching tmux's `layout_cell` tree model and layout string format (checksum + tree) as implemented in tmux `layout-custom.c`.
- A concrete OpenTelemetry integration plan: trace context propagation across server/client/bindings, span taxonomy, crossing pure/impure boundary via injected context, runtime enable/disable, exporters, and sampling.
- Language bindings (Python/Node/C++) with exact class shapes, async/GIL/event-loop handling, snapshot reads via `ArcSwap`, and error mapping/packaging.
- CRDT collaboration with explicit CRDT types, HLC details, OpLog persistence and sync protocol, conflict policies with examples, and how ops wrap core Events.
- Control mode protocol (`tmux -C`) as a structured, line-based stream with `%begin/%end/%error` blocks and notifications; treated as hints with binary protocol as authority.
- Concrete server lifecycle (startup/identify handshake/shutdown/crash recovery/lock file), security boundaries, and performance targets + benchmark infrastructure.

All file/line references below are to the upstream tmux C tree at `~/study/c/tmux/` and the Rust prototype at `~/work/rust/vibe-tmux/`.

---

## A. Concrete Error Handling Strategy

### A1. Error Philosophy (Layered, Typed, Recoverable)

v4 said "use `thiserror`"; v5 defines:

1. Layer 0 (PURE) crates: never use `anyhow`; define small, structured enums. Errors must be cloneable where they can appear in snapshots.
2. Layer 1 (IMPURE) crates: define IO/system errors (contain `io::Error`, `nix`, etc.), plus mapping from pure errors.
3. Layer 2/4 (Facade/Bindings): define stable public error types and map into language-native exceptions.

Untrusted input rule: anything from sockets/control mode/config files is untrusted. Decode/parse errors must be classified as:

- `Transient`: can keep connection and resync (rare; mostly "need more bytes").
- `ProtocolViolation`: drop that connection (binary protocol), but keep server alive.
- `UserError`: bad command/option, report to client, keep connection.
- `Bug/Invariants`: internal consistency issue; record + optionally crash depending on mode.

tmux reference: invalid binary message causes peer kill in `server-client.c` (`goto bad` -> `proc_kill_peer`) at `server-client.c:3377-3475`.

### A2. Crate-by-Crate Error Types (Exact)

This is the v5 baseline taxonomy. Names are illustrative but should be adopted literally to stabilize inter-crate mapping.

#### `mux-types` (PURE)

No fallible ops. Only `TryFrom` conversions for IDs if needed:

```rust
// crates/mux-types/src/error.rs
#![forbid(unsafe_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum IdError {
    #[error("invalid raw id {raw} for {kind}")]
    Invalid { raw: u64, kind: &'static str },
}
```

#### `mux-proto` (PURE)

Split errors into:

1. Framing errors (imsg header/length/type): always untrusted.
2. Payload errors (type-known but invalid payload).
3. Semantic errors (valid frame but violates handshake state).

The existing prototype already has a framing error enum in `~/work/rust/vibe-tmux/crates/mux-proto/src/error.rs:7-33` with length bounds (`MIN_IMSG_SIZE=16`, `MAX_IMSG_SIZE=16384`). Keep this but extend it:

```rust
// crates/mux-proto/src/error.rs
#![forbid(unsafe_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum FrameError {
    #[error("incomplete header")]
    Incomplete,
    #[error("length too small: {len}")]
    LengthTooSmall { len: u32 },
    #[error("length too large: {len}")]
    LengthTooLarge { len: u32 },
    #[error("unknown message type: {msg_type}")]
    UnknownMessageType { msg_type: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PayloadError {
    #[error("invalid payload for {msg:?}: {reason}")]
    Invalid { msg: MsgType, reason: &'static str },
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HandshakeError {
    #[error("identify burst out of order: got {got:?} after {after:?}")]
    IdentifyOutOfOrder { after: MsgType, got: MsgType },
    #[error("identify missing required field {field}")]
    IdentifyMissing { field: &'static str },
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProtoError {
    #[error(transparent)]
    Frame(#[from] FrameError),
    #[error(transparent)]
    Payload(#[from] PayloadError),
    #[error(transparent)]
    Handshake(#[from] HandshakeError),
}
```

#### Malformed-frame behavior (binary protocol)

`mux-proto` must make the recovery policy explicit at the codec boundary:

```rust
// crates/mux-proto/src/codec.rs
pub enum DecodeOutcome {
    NeedMore,
    Frame(ImsgFrame),
    ProtocolViolation(ProtoError),
}

pub struct ImsgCodec {
    buf: bytes::BytesMut,
}

impl ImsgCodec {
    pub fn feed(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    pub fn try_decode_one(&mut self) -> DecodeOutcome {
        match decode_one(&self.buf) {
            Ok(Some((frame, used))) => {
                self.buf.advance(used);
                DecodeOutcome::Frame(frame)
            }
            Ok(None) => DecodeOutcome::NeedMore,
            Err(err) => {
                // Drop all buffered bytes for this connection so we don't loop.
                self.buf.clear();
                DecodeOutcome::ProtocolViolation(err)
            }
        }
    }
}
```

Policy:

- `NeedMore`: keep reading.
- `Frame`: pass to state machine.
- `ProtocolViolation`: close connection; record reason and client metadata.

#### `mux-conf` (PURE)

The prototype already defines `LexError` and `ParseError` as typed `thiserror` enums in:

- `~/work/rust/vibe-tmux/crates/mux-conf/src/lexer.rs:79-93`
- `~/work/rust/vibe-tmux/crates/mux-conf/src/parser.rs:16-30`

v5 adds an eval/exec error used when mapping config AST to core Events:

```rust
// crates/mux-conf/src/error.rs
#![forbid(unsafe_code)]

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConfError {
    #[error(transparent)]
    Lex(#[from] lexer::LexError),
    #[error(transparent)]
    Parse(#[from] parser::ParseError),
    #[error("unsupported config construct: {what} at byte {pos}")]
    Unsupported { what: &'static str, pos: usize },
}
```

#### `mux-core` (PURE)

Core event application should be "mostly infallible"; if it can fail, failures must be domain errors, not `io`.

```rust
// crates/mux-core/src/error.rs
#![forbid(unsafe_code)]

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CoreError {
    #[error("not found: {kind} {id:?}")]
    NotFound { kind: &'static str, id: u64 },
    #[error("invalid target: {reason}")]
    InvalidTarget { reason: &'static str },
    #[error("option type mismatch for {name}")]
    OptionTypeMismatch { name: String },
    #[error("layout error: {0}")]
    Layout(#[from] LayoutError),
    #[error("invariant violated: {0}")]
    Invariant(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LayoutError {
    #[error("min size violation for pane {pane_id:?}")]
    MinSizeViolation { pane_id: PaneId },
    #[error("invalid layout string: {reason}")]
    InvalidLayoutString { reason: &'static str },
    #[error("checksum mismatch")]
    ChecksumMismatch,
}
```

`apply_event()` returns `Result<Outcome, CoreError>`.

#### `mux-os` / `mux-pty` / `mux-server` (IMPURE)

Examples:

```rust
// crates/mux-os/src/error.rs
#[derive(Debug, thiserror::Error)]
pub enum OsError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("fd passing rejected: {reason}")]
    FdPassingRejected { reason: &'static str },
}

// crates/mux-pty/src/lib.rs (prototype already has this)
// ~/work/rust/vibe-tmux/crates/mux-pty/src/lib.rs:107-112 defines PtyBackendError with Io(io::Error).

// crates/mux-server/src/error.rs
#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error(transparent)]
    Os(#[from] mux_os::OsError),
    #[error(transparent)]
    Proto(#[from] mux_proto::ProtoError),
    #[error(transparent)]
    Core(#[from] mux_core::CoreError),
    #[error("client {client:?} protocol violation: {reason}")]
    ClientProtocolViolation { client: ClientId, reason: String },
}
```

#### `mux-api` (FACADE)

Expose a stable `ApiError`:

```rust
// crates/mux-api/src/error.rs
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("not connected")]
    NotConnected,
    #[error("backend: {0}")]
    Backend(#[from] BackendError),
    #[error("protocol: {0}")]
    Protocol(#[from] mux_proto::ProtoError),
    #[error("timeout")]
    Timeout,
}
```

The prototype `ManagedMuxError` exists in `~/work/rust/vibe-tmux/crates/mux-api/src/managed.rs:54-62` and maps to backend errors at `~/work/rust/vibe-tmux/crates/mux-api/src/managed.rs:64-72`. v5 formalizes recoverability:

```rust
impl ApiError {
    pub fn is_transient(&self) -> bool {
        matches!(self, ApiError::Timeout)
    }
}
```

### A3. Propagation Across the Pure/Impure Boundary

Rule: only the runtime dispatch loop can observe IO/system errors. The pure core only observes them as events.

Pattern:

1. Core emits `Effect::SpawnPane { ... }`.
2. Runtime executes; if it fails, runtime pushes an Event back, never panics.

```rust
// crates/mux-core/src/effect.rs (PURE)
pub enum Effect {
    SpawnPane { pane: PaneId, cmd: Vec<String>, cwd: Option<String> },
    ResizePty { pane: PaneId, cols: u16, rows: u16 },
    KillPane { pane: PaneId, signal: i32 },
}

// crates/mux-core/src/event.rs (PURE)
pub enum Event {
    // ...
    EffectFailed {
        effect_id: u64,
        kind: EffectKind,
        pane: Option<PaneId>,
        // must be stable/serializable; no io::Error here
        message: String,
    },
}
```

Fatal vs non-fatal:

- Per-pane IO failures: non-fatal, become `Event::PaneExited` / `Event::EffectFailed`.
- Protocol violation: fatal for that connection, not server.
- Corrupted persisted state on boot: fatal unless `--recover` mode.

### A4. Codec Encountering Malformed Frames

Binary protocol behavior:

- If `FrameError::Incomplete`: keep reading.
- If `LengthTooLarge`: treat as attack; close connection immediately.
- If `UnknownMessageType`: close connection, log `msg_type`.
- If payload invalid: close connection (cannot trust resync without framing).

Control mode behavior (line protocol):

- A malformed `%begin/%end/%error` line: emit `ControlParseError`, mark control channel unhealthy, trigger full refresh drift.
  - Prototype behavior: control mode parse errors are written to control client as `parse error: ...` bracketed by `%begin` and `%error` guards in `control.c:520-533`.

### A5. Recovery in `SocketActor` / `ManagedMux` Facade

v5 adopts the drift/refresh model used in the prototype `SocketActor`:

- Control stream disconnect triggers `mark_drift()` and full refresh at `~/work/rust/vibe-tmux/crates/mux-api/src/managed.rs:760-765`.
- Any `cmd()` executed without control mode active marks drift and schedules full refresh at `~/work/rust/vibe-tmux/crates/mux-api/src/managed.rs:611-616`.
- Trace context propagation across threads is done by capturing headers and pushing them in each thread at `~/work/rust/vibe-tmux/crates/mux-api/src/managed.rs:160-220`.

Concrete `record_view_error` behavior exists at `~/work/rust/vibe-tmux/crates/mux-api/src/managed.rs:736-749`: it stores a last-error string only if it changed, avoiding spam.

v5 adds: classify errors and backoff policy:

```rust
// crates/mux-api/src/managed.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Health {
    Healthy,
    Degraded,   // control mode down, polling refresh
    Unreachable // socket down
}

impl SocketActor {
    fn on_backend_error(&mut self, err: &BackendError) {
        // 1. update error store
        // 2. degrade health
        // 3. schedule refresh with exponential backoff
    }
}
```

---

## B. Configuration System Deep Dive

v4 introduced `mux-conf` and "tmux.conf subset"; v5 specifies:

1. Exact grammar for the subset (and a path to full grammar).
2. Option inheritance chain and how it's represented in snapshots.
3. Hot reload eventization.
4. Relationship between `mux-conf` (syntax), `mux-command` (command semantics), and `mux-core` (events).

### B1. `.tmux.conf` Syntax: Supported Subset and Grammar

Use the existing `mux-conf` lexer/parser design (prototype):

- AST types in `~/work/rust/vibe-tmux/crates/mux-conf/src/ast.rs:7-174` include `StatementKind::{Command, Conditional}` and command `ArgKind::{Literal, Variable, Block}`.
- Lexer supports quotes, vars, comments, line continuation; errors are typed (`LexError`) at `~/work/rust/vibe-tmux/crates/mux-conf/src/lexer.rs:79-93`.
- Parser supports `%if/%elif/%else/%endif` conditionals (see `StatementKind::Conditional` in `~/work/rust/vibe-tmux/crates/mux-conf/src/ast.rs:80-87`).

Initial subset (Phase 1 for TermForge):

- `set-option` / `set` (server/session/window/pane; support `-g`, `-w`, `-p`, `-s`, `-u`, `-a`)
- `setw` alias for window options
- `bind-key` / `bind`
- `unbind-key` / `unbind`
- `source-file` / `source` (support `-q`, `-F`, glob expansion)
- `%if/%elif/%else/%endif` with condition format expansion (format truthiness)
- Comments, semicolons, newlines, `{ ... }` blocks

EBNF for the supported subset (matches the AST model):

```ebnf
config     := { stmt } EOF ;
stmt       := command_stmt | conditional_stmt ;
command_stmt := command { separator command } [separator] ;
separator  := ";" | NEWLINE ;

command    := word { arg } ;
arg        := word | quoted | variable | block ;
block      := "{" { stmt } "}" ;

conditional_stmt :=
    "%if" { arg } NEWLINE
        { stmt }
    { "%elif" { arg } NEWLINE { stmt } }
    [ "%else" NEWLINE { stmt } ]
    "%endif" ;
```

### B2. Option Inheritance Chain (Server -> Session -> Window -> Pane)

tmux reference behavior:

- `struct options` has a `parent` pointer and `options_get()` walks parents until it finds a matching entry (`options.c:228-240`).
- Scope resolution for `set-option` consults option table and flags in `options_scope_from_name()` (`options.c:850+`; see `options.c:873-880` where `OPTIONS_TABLE_SERVER` uses `global_options`, etc.).

TermForge representation (PURE):

```rust
// crates/mux-core/src/options.rs
#![forbid(unsafe_code)]

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OptionValue {
    String(String),
    Number(i64),
    Bool(bool),
    Choice(&'static str),
    // arrays are required for options like status-format[n]
    Array(BTreeMap<u32, OptionValue>),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OptionMap {
    entries: BTreeMap<String, OptionValue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionScope {
    Server,
    Session(SessionId),
    Window(WindowId),
    Pane(PaneId),
}

pub struct OptionResolver<'a> {
    server: &'a OptionMap,
    session: Option<&'a OptionMap>,
    window: Option<&'a OptionMap>,
    pane: Option<&'a OptionMap>,
}

impl<'a> OptionResolver<'a> {
    pub fn get(&self, key: &str) -> Option<&'a OptionValue> {
        // precedence: pane -> window -> session -> server
        self.pane.and_then(|m| m.entries.get(key))
            .or_else(|| self.window.and_then(|m| m.entries.get(key)))
            .or_else(|| self.session.and_then(|m| m.entries.get(key)))
            .or_else(|| self.server.entries.get(key))
    }
}
```

### B3. Hot Reload Mechanics (Config Changes as Events)

tmux reference:

- config load is started after the first client identify completes at `server-client.c:3725-3734` (`start_cfg()`), and config parse errors are reported via `%config-error` in control mode (`cfg.c:229-254`).

TermForge design:

1. `mux-conf` parses into AST with spans.
2. `mux-command` resolves commands to core Events (plus some Effects like `run-shell`).
3. Config application is just a batch of Events run through the state actor.
4. Hot reload is initiated by:
   - `source-file` command (runtime)
   - File watcher (optional, behind feature flag)

Core Events:

```rust
// crates/mux-core/src/event.rs
pub enum Event {
    // ...
    ConfigReloadRequested { path: String, client: Option<ClientId> },
    ConfigReloadStarted { path: String },
    ConfigReloadCompleted { path: String, applied: u32, errors: u32 },
    ConfigError { path: String, line: u32, message: String },
    OptionSet { scope: OptionScope, key: String, value: OptionValue },
    OptionUnset { scope: OptionScope, key: String },
    KeyBindingSet { table: String, key: KeyCode, binding: KeyBinding },
    KeyBindingUnset { table: String, key: KeyCode },
}
```

Reference: tmux config load runs in global queue and blocks initial client so its command runs after config load (`cfg.c:60-93`).

Prototype reference: statement span prefixing for config errors is in `~/work/rust/vibe-tmux/crates/mux-server/src/config.rs:120-170`.

### B4. `mux-conf` vs `mux-command`

Hard boundary:

- `mux-conf` owns syntax and AST only.
- `mux-command` owns tmux command semantics and argument parsing.

Concretely:

```rust
// crates/mux-command/src/lib.rs
pub struct CommandCompiler;

impl CommandCompiler {
    pub fn compile_statement(stmt: &mux_conf::ast::Statement) -> Result<Vec<mux_core::Event>, CommandError>;
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CommandError {
    #[error("unknown command {name}")]
    UnknownCommand { name: String },
    #[error("invalid args for {name}: {reason}")]
    InvalidArgs { name: String, reason: &'static str },
    #[error("unsupported feature: {what}")]
    Unsupported { what: &'static str },
}
```

---

## C. Layout Engine Design

v4 said "LayoutTree + tiling algorithms"; v5 makes it concrete and matches tmux's `layout_cell`.

tmux references:

- Layout string serialization/parsing is in `layout-custom.c`:
  - checksum algorithm: `layout_checksum` rotates and adds bytes at `layout-custom.c:45-56`
  - dump format: `"%04hx,%s"` at `layout-custom.c:59-70`
  - append format: leaf includes pane id at `layout-custom.c:85-91`
  - parse verifies checksum at `layout-custom.c:164-173`
  - parse "close bottom right if fewer panes than cells" at `layout-custom.c:186-201`

### C1. Data Structures

We need:

- A stable node ID type (typed IDs per v4 decision #3).
- A tree of nodes with parent pointer and ordered children.
- Node geometry (sx/sy/xoff/yoff) matching tmux.
- Leaf nodes referencing `PaneId`.

```rust
// crates/mux-core/src/layout.rs
#![forbid(unsafe_code)]

use mux_types::{PaneId, WindowId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutType {
    LeftRight,  // tmux LAYOUT_LEFTRIGHT uses '{' ... '}' in layout strings
    TopBottom,  // tmux LAYOUT_TOPBOTTOM uses '[' ... ']'
    WindowPane, // leaf
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutCell {
    pub ty: LayoutType,
    pub parent: Option<LayoutCellId>,
    pub children: Vec<LayoutCellId>,
    pub rect: Rect,
    pub pane: Option<PaneId>, // Some for leaf, None for internal
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutTree {
    pub root: LayoutCellId,
    pub cells: Vec<LayoutCell>,
}

impl LayoutTree {
    pub fn cell(&self, id: LayoutCellId) -> &LayoutCell { &self.cells[id.0 as usize] }
    pub fn cell_mut(&mut self, id: LayoutCellId) -> &mut LayoutCell { &mut self.cells[id.0 as usize] }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LayoutCellId(pub u32);
```

### C2. Minimum Pane Size Constraints

tmux enforces minimum sizes during resize. TermForge defines:

- `min_pane_w`, `min_pane_h` from options.
- A border consumes 1 cell between siblings (tmux adds `+1` frequently; see `layout-custom.c:135-149` checking `+1` border math).

Core API:

```rust
pub struct LayoutConstraints {
    pub min_w: u16,
    pub min_h: u16,
    pub border: u16, // 1 for tmux split border, 0 for overlay layouts
}

pub fn validate_layout(tree: &LayoutTree, constraints: &LayoutConstraints) -> Result<(), LayoutError>;
```

### C3. Layout Algorithms (Concrete)

We implement tmux-like layout commands. Each algorithm produces a new `LayoutTree` for a given window size and pane list order.

```rust
pub enum LayoutPreset {
    Tiled,
    EvenHorizontal,
    EvenVertical,
    MainHorizontal { main_size: Option<u16> }, // rows
    MainVertical { main_size: Option<u16> },   // cols
}

pub fn build_layout_preset(
    panes: &[PaneId],
    window_size: (u16, u16),
    preset: LayoutPreset,
    constraints: &LayoutConstraints,
) -> Result<LayoutTree, LayoutError>;
```

Algorithm specs (implementation-level):

General rules:

1. The `panes: &[PaneId]` order is authoritative for which panes become "main" vs "stack", and for stable tiling. This should match tmux's pane ordering within a window (in tmux C, panes are in a `TAILQ` on `struct window`; see how layout assignment walks panes in order in `layout-custom.c:243-278`).
2. Borders: every split inserts a 1-cell border between siblings. In geometry, this means for `n` siblings along an axis, total occupied size is `sum(child_sizes) + (n-1)*border`.
3. Remainders: when distributing widths/heights, leftover cells are assigned to earlier siblings to preserve determinism.
4. Minimum sizes: if a computed child size would violate `constraints.min_w/min_h`, the algorithm must reduce the number of rows/cols (for `Tiled`) or clamp `main_size` and stack sizes (for `Main*`).

#### EvenHorizontal

Tree:

- root: `TopBottom`
- children: `WindowPane` leaves

Geometry:

- `root.rect = (0,0,window_w,window_h)`
- available height for children: `window_h - (n-1)*border`
- each child height: `base = available / n`, `rem = available % n`, child_i gets `base + (i < rem ? 1 : 0)`
- offsets: `y = sum(prev_h + border)`

#### EvenVertical

Same as above but:

- root `LeftRight`
- distribute widths
- offsets on x

#### Tiled

Goal: produce a near-square grid.

1. Choose `cols = ceil(sqrt(n))`, `rows = ceil(n/cols)`.
2. Validate `min_w/min_h`: if `window_w` or `window_h` can't fit, reduce `cols` or `rows` until feasible.
3. Build a two-level tree:
   - root `TopBottom` with `rows` children
   - each row child is a `LeftRight` node with up to `cols` leaf panes
4. Distribute heights across rows and widths across columns using the even distribution rule.
5. Fill panes row-major.

#### MainVertical

Tree:

- root `LeftRight` with 2 children:
  - left child: the "main" leaf (single pane) or a `TopBottom` stack if `main_count > 1` (Phase 2)
  - right child: `TopBottom` stack of remaining panes (or leaf if 1)

Geometry:

- `main_w` defaults to option `main-pane-width` (if modeled) else `window_w/2`.
- Clamp `main_w` to `[min_w, window_w - min_w - border]` if stack exists.
- Stack width is remainder.
- Heights in stacks distributed evenly.

MainHorizontal mirrors this with axes swapped (`TopBottom` root).

### C4. Resize Propagation Through the Tree (tmux parity)

tmux resize logic is nontrivial and encoded in `layout.c` via:

- `layout_resize_check` and `layout_resize_adjust` (`layout.c:367-466`)
- `layout_resize_pane` choosing which sibling to resize when the target is last (`layout.c:665-668`)

TermForge must implement the same semantics:

1. When resizing a pane along axis X in a `LeftRight` parent, transfer width from an adjacent sibling; if the pane is last, use previous sibling (tmux uses `TAILQ_PREV` when last).
2. When the desired change exceeds what adjacent siblings can donate while respecting minima, walk outward across siblings (tmux iterates with `TAILQ_PREV` in `layout_resize_pane_grow` at `layout.c:693-699`).
3. After adjustments, recompute offsets and fix pane geometry (tmux calls `layout_fix_offsets` and `layout_fix_panes` after resizes; see `layout.c:641-642`).

### C4. Resize Propagation Through the Tree

We match tmux's "available delta propagation" approach (see `layout.c:535-582` and `layout_resize_check`/`layout_resize_adjust` in `layout.c:367+`):

Core API:

```rust
pub enum ResizeAxis { X, Y }

pub fn resize_window(
    tree: &mut LayoutTree,
    new_size: (u16, u16),
    constraints: &LayoutConstraints,
) -> Result<(), LayoutError>;

pub fn resize_pane(
    tree: &mut LayoutTree,
    pane: PaneId,
    axis: ResizeAxis,
    change: i16,      // positive grow, negative shrink
    constraints: &LayoutConstraints,
) -> Result<(), LayoutError>;
```

### C5. Layout Serialization: tmux Layout String Format

tmux format (from `layout-custom.c`):

- Entire string: `"{checksum4hex},{tree}"` where checksum is computed over `tree` (not including checksum prefix) at `layout-custom.c:69-70`.
- Each node begins with: `"{sx}x{sy},{xoff},{yoff}"` and if leaf, `",{pane_id}"` at `layout-custom.c:85-91`.
- Internal nodes wrap children:
  - `LeftRight`: `...{ child1,child2,... }` using `{` `}` at `layout-custom.c:98-111`
  - `TopBottom`: `...[ child1,child2,... ]` using `[` `]` at `layout-custom.c:101-111`

Rust API:

```rust
pub fn dump_layout_tmux(tree: &LayoutTree) -> String;
pub fn parse_layout_tmux(input: &str) -> Result<LayoutTree, LayoutError>;
```

Checksum implementation must match tmux exactly (`layout-custom.c:51-56`):

```rust
fn tmux_layout_checksum(bytes: &[u8]) -> u16 {
    let mut csum: u16 = 0;
    for &b in bytes {
        csum = (csum >> 1) + ((csum & 1) << 15);
        csum = csum.wrapping_add(b as u16);
    }
    csum
}
```

Applying a parsed layout string to an existing window (pane assignment) must match tmux:

- tmux assigns panes in TAILQ order by walking the layout tree depth-first at `layout-custom.c:243-278`.
- If fewer panes than cells, tmux deletes bottom-right cells repeatedly at `layout-custom.c:186-201`.

TermForge should expose:

```rust
// crates/mux-core/src/layout_apply.rs
pub fn apply_layout_to_window(
    tree: &LayoutTree,
    panes_in_order: &[PaneId],
) -> Result<LayoutTree, LayoutError>;
```

This function returns a new `LayoutTree` where leaves have `pane: Some(pane_id)` assigned in order.

---

## D. OpenTelemetry Integration

v4 said "mux-telemetry"; v5 specifies span taxonomy and concrete context propagation across server/client/bindings.

Prototype references:

- The `mux-otel` crate defines a `TraceCtx` with `traceparent/tracestate/baggage` and a thread-local stack (`TRACE_HEADERS_STACK`) at `~/work/rust/vibe-tmux/crates/mux-otel/src/lib.rs:31-41`, plus `push_trace_headers()` at `~/work/rust/vibe-tmux/crates/mux-otel/src/lib.rs:114-119`.
- `SocketActor` captures current headers before spawning threads and pushes them per-thread at `~/work/rust/vibe-tmux/crates/mux-api/src/managed.rs:160-220`.
- Helper process spawns propagate W3C headers via env vars at `~/work/rust/vibe-tmux/crates/mux-server/src/telemetry_env.rs:49-57`.

### D1. Trace Context Propagation Across Server -> Client -> Bindings

Concrete propagation mechanisms:

- In-process Rust: use `tracing` spans; capture `TraceCtx` and use `push_trace_headers`.
- Cross-process: set `TRACEPARENT`, `TRACESTATE`, `BAGGAGE` env vars for helper spawns.
- Cross-language bindings:
  - Python: accept optional `traceparent/tracestate/baggage` strings on API calls; push into `mux-otel` stack for duration of call.
  - Node: accept optional headers or use OpenTelemetry JS API to inject; pass down via NAPI.
  - C++: expose setters/getters for headers; optionally integrate with OpenTelemetry C++ SDK (future).

### D2. Span Design (Operations and Attributes)

Canonical span names:

- `termforge.server.accept` (attrs: `socket_path`, `fd`)
- `termforge.server.client.handshake` (attrs: `client_pid`, `ttyname`, `term`, `features`)
- `termforge.server.actor.tick` (attrs: `queue_len`, `effects_len`)
- `termforge.core.apply_event` (attrs: `event.kind`, `event.scope`)
- `termforge.runtime.dispatch_effect` (attrs: `effect.kind`, `pane_id`)
- `termforge.pty.read` / `termforge.pty.write` (attrs: `pane_id`, `nbytes`)
- `termforge.grid.parse` (attrs: `pane_id`, `nbytes`, `state`)
- `termforge.control.parse` (attrs: `line_len`, `state`)
- `termforge.bindings.python.call` (attrs: `method`, `gil_ms`)

### D3. Crossing the Pure/Impure Boundary (Injected Context)

Core is pure but can still carry trace metadata via `CoreCtx`:

```rust
// crates/mux-core/src/context.rs (PURE)
#[derive(Debug, Clone, Default)]
pub struct CoreTraceCtx {
    pub traceparent: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CoreCtx {
    pub now_ms: u64,
    pub trace: CoreTraceCtx,
}
```

### D4. Runtime Enable/Disable, Exporters, Sampling

Minimum config surface:

- Env: `TERMFORGE_OTEL=1`
- Env: `OTEL_EXPORTER_OTLP_ENDPOINT` (fallback detection like prototype `otel_enabled()` in `~/work/rust/vibe-tmux/crates/mux-otel/src/lib.rs:62-88`)
- File: `.termforge/otel.toml`

Concrete crate API (align with the prototype `mux-otel` approach but keep TermForge naming stable):

```rust
// crates/mux-telemetry/src/lib.rs
#![forbid(unsafe_code)]

#[derive(Debug, Clone)]
pub struct TelemetryConfig {
    pub enabled: bool,
    pub service_name: String,
    pub service_version: String,
    pub sampling: Sampling,
    pub exporter: Exporter,
    pub export_filter: ExportFilter,
}

#[derive(Debug, Clone)]
pub enum Sampling {
    AlwaysOn,
    AlwaysOff,
    TraceIdRatio(f64),
}

#[derive(Debug, Clone)]
pub enum Exporter {
    OtlpHttp { endpoint: String },
    OtlpGrpc { endpoint: String },
    Stdout,
    None,
}

#[derive(Debug, Clone)]
pub struct ExportFilter {
    pub allow_targets: Vec<String>, // prefix matches
}

pub fn init_tracing(cfg: &TelemetryConfig) -> Result<(), TelemetryError>;

#[derive(Debug, thiserror::Error)]
pub enum TelemetryError {
    #[error("invalid config: {0}")]
    InvalidConfig(String),
    #[error("init failed: {0}")]
    InitFailed(String),
}
```

Trace context helpers:

```rust
// crates/mux-telemetry/src/context.rs
#[derive(Debug, Clone, Default)]
pub struct TraceCtx {
    pub traceparent: String,
    pub tracestate: Option<String>,
    pub baggage: Option<String>,
}

pub type TraceHeaders = TraceCtx;

pub fn current_trace_headers() -> Option<TraceHeaders>;
pub fn push_trace_headers(headers: TraceHeaders) -> TraceHeadersGuard;

pub struct TraceHeadersGuard { /* pops thread-local stack on drop */ }
```

This mirrors the prototype's header stack at `~/work/rust/vibe-tmux/crates/mux-otel/src/lib.rs:31-41` and `push_trace_headers` at `~/work/rust/vibe-tmux/crates/mux-otel/src/lib.rs:114-119`.

Runtime enable/disable semantics:

- If `enabled=false`: install only fmt/log layers; no OTLP exporter.
- If enabled: set up OTLP exporter and ensure any helper spawns propagate `TRACEPARENT` env (use the pattern in `~/work/rust/vibe-tmux/crates/mux-server/src/telemetry_env.rs:49-57`).

Sampling semantics:

- AlwaysOn: export all TermForge spans.
- TraceIdRatio: deterministic sampling by trace id hash.

Span attachment points (must exist in code, not just docs):

```rust
// crates/mux-server/src/actor.rs
let span = tracing::info_span!(
    "termforge.server.actor.tick",
    queue_len = events_len,
    effects_len = effects_len
);
let _g = span.enter();
```

---

## E. Language Bindings Detail

v4 had thin wrapper philosophy; v5 specifies concrete class hierarchies, async/event-loop integration, snapshot reads, and error mapping.

Bindings depend only on `mux-orm` + `mux-api` (and `mux-types` for typed ids).

### E1. Snapshot Read Model (ArcSwap Load -> Typed Wrapper)

Bindings should never hold locks into the server. They read snapshots published via `ArcSwap`.

Wrapper objects hold:

- `Arc<ManagedMuxHandle>` (for issuing commands and subscribing)
- `Arc<SnapshotStore<View>>` (for reads)
- typed id (`SessionId`, `WindowId`, `PaneId`)

Reads are point-in-time: each method loads a snapshot and operates on it; no stale references.

Concrete wrapper pattern (Rust side, shared across bindings):

```rust
// crates/mux-orm/src/wrappers.rs
use std::sync::Arc;

pub struct OrmRoot {
    pub handle: mux_api::ManagedMuxHandle,
}

pub struct OrmServer {
    root: Arc<OrmRoot>,
}

#[derive(Clone)]
pub struct OrmSession {
    root: Arc<OrmRoot>,
    id: mux_types::SessionId,
}

#[derive(Clone)]
pub struct OrmWindow {
    root: Arc<OrmRoot>,
    id: mux_types::WindowId,
}

#[derive(Clone)]
pub struct OrmPane {
    root: Arc<OrmRoot>,
    id: mux_types::PaneId,
}

impl OrmServer {
    pub fn sessions(&self) -> Vec<OrmSession> {
        let view = self.root.handle.view().unwrap_or_default(); // bindings must map None to empty
        view.sessions().map(|id| OrmSession { root: Arc::clone(&self.root), id }).collect()
    }
}
```

Snapshot reads from bindings must not clone large state:

- `ArcSwap::load_full()` gives `Arc<View>`; wrappers borrow from that snapshot for the duration of the call.
- For language object methods that need to return large data (e.g. `capture_pane()`), return a String/bytes and let the caller own it.

### E2. Python (PyO3): Class Hierarchy, GIL, Async

Class hierarchy (mirrors `libtmux` ORM style in `~/work/python/libtmux/src/libtmux/server.py:49`, `pane.py:48`, etc):

- `termforge.ManagedMux`
- `termforge.Server`
- `termforge.Session`
- `termforge.Window`
- `termforge.Pane`
- `termforge.QueryList[T]`

GIL rules:

- Any call that can block must release GIL via `py.allow_threads(|| ...)`.

Concrete PyO3 surface:

```rust
// bindings/python/src/lib.rs
use pyo3::prelude::*;

#[pyclass(module = "termforge")]
pub struct ManagedMux {
    inner: std::sync::Arc<std::sync::Mutex<mux_api::ManagedMux>>,
    driver: std::sync::Arc<std::sync::Mutex<Option<mux_api::SocketDriver>>>,
}

#[pymethods]
impl ManagedMux {
    #[new]
    fn new(socket_path: Option<String>) -> PyResult<Self> {
        let mux = if let Some(path) = socket_path {
            mux_api::ManagedMux::tmux_with_options(mux_api::TmuxBackendOptions::with_socket(path))
                .map_err(py_err)?
        } else {
            mux_api::ManagedMux::tmux_default().map_err(py_err)?
        };
        Ok(Self {
            inner: std::sync::Arc::new(std::sync::Mutex::new(mux)),
            driver: std::sync::Arc::new(std::sync::Mutex::new(None)),
        })
    }

    fn start(&self, py: Python<'_>) -> PyResult<()> {
        let inner = std::sync::Arc::clone(&self.inner);
        py.allow_threads(|| {
            let mut guard = self.driver.lock().unwrap();
            if guard.is_none() {
                *guard = Some(mux_api::RefreshDriver::spawn(inner));
            }
            Ok(())
        })
    }

    fn handle(&self) -> PyResult<ManagedMuxHandle> {
        let guard = self.inner.lock().unwrap();
        Ok(ManagedMuxHandle { inner: guard.handle() })
    }
}

#[pyclass(module = "termforge")]
pub struct ManagedMuxHandle {
    inner: mux_api::ManagedMuxHandle,
}

#[pymethods]
impl ManagedMuxHandle {
    fn last_error(&self) -> Option<String> { self.inner.last_error() }

    fn cmd(&self, py: Python<'_>, cmd: String) -> PyResult<String> {
        py.allow_threads(|| {
            // Prefer a non-mutable cmd path in mux-api; if not, wrap in a Mutex at OrmRoot.
            Err(pyo3::exceptions::PyNotImplementedError::new_err("cmd on handle TBD"))
        })
    }
}

fn py_err<E: std::fmt::Display>(e: E) -> pyo3::PyErr {
    pyo3::exceptions::PyRuntimeError::new_err(e.to_string())
}
```

Async subscription:

- Provide `handle.subscribe()` returning an async iterator that yields refresh hints.
- Implementation strategy: spawn a Rust thread that reads from `broadcast::Receiver` and pushes into an `asyncio.Queue` via `pyo3` call scheduling.

Error mapping:

- Map `BackendError::NoServerRunning` to `termforge.exc.NoServerRunning`.
- Map `CoreError::NotFound` to `termforge.exc.NotFound`.
- Map `Timeout` to `TimeoutError`.

Packaging:

- Use `maturin` and publish wheels. Expose a `pyproject.toml` with `pyo3` extension.

### E3. Node (NAPI-RS): Class Design, Event Loop, TS Types

Event loop integration:

- Use `napi::ThreadsafeFunction` to deliver refresh hints to JS without blocking.

Packaging:

- Use `pnpm` + `napi-rs` build; prebuild binaries per target; publish scoped package.

Concrete NAPI surface:

```rust
// bindings/node/src/lib.rs
use napi::bindgen_prelude::*;

#[napi]
pub struct ManagedMux {
    inner: std::sync::Arc<std::sync::Mutex<mux_api::ManagedMux>>,
    driver: std::sync::Arc<std::sync::Mutex<Option<mux_api::SocketDriver>>>,
}

#[napi]
impl ManagedMux {
    #[napi(constructor)]
    pub fn new(socket_path: Option<String>) -> napi::Result<Self> { /* ... */ }

    #[napi]
    pub fn start(&self) -> napi::Result<()> { /* spawn driver */ }

    #[napi]
    pub fn handle(&self) -> napi::Result<ManagedMuxHandle> { /* clone handle */ }
}

#[napi]
pub struct ManagedMuxHandle {
    inner: mux_api::ManagedMuxHandle,
}

#[napi]
impl ManagedMuxHandle {
    #[napi]
    pub fn view_json(&self) -> napi::Result<String> { /* snapshot -> serde_json */ }

    #[napi]
    pub fn on_refresh(&self, cb: napi::JsFunction) -> napi::Result<JsObject> {
        // Create ThreadsafeFunction and forward broadcast events.
        Ok(cb.env().create_object()?)
    }
}
```

TypeScript generation:

- Export `.d.ts` for `ManagedMux`, `ManagedMuxHandle`, entity wrappers, and event types.

### E4. C++ (cxx): Bridge Design and Ownership

Ownership:

- C++ holds `std::shared_ptr` wrappers around Rust `Arc` objects.
- Snapshot objects are immutable views backed by `Arc<View>`.

Concrete cxx ownership rules:

- Functions returning snapshots return `cxx::SharedPtr<View>` (View is opaque C++ type whose internals are `Arc<View>`).
- Entity wrappers store `SharedPtr<View>` + typed id; methods require passing the snapshot to avoid hidden global state.

Example:

```rust
// crates/mux-cxx/src/lib.rs
#[cxx::bridge]
mod ffi {
    extern "Rust" {
        type Managed;
        type View;
        type Session;

        fn managed_new(socket_path: String) -> Result<Box<Managed>>;
        fn managed_view(m: &mut Managed) -> Result<std::shared_ptr<View>>;

        fn view_sessions(v: &View) -> Vec<Session>;
        fn session_name(s: &Session) -> String;
    }
}
```

Error mapping:

- `Result<T>` in cxx becomes exceptions; error strings must include a stable prefix, e.g. `TF_BACKEND:...`, `TF_NOTFOUND:...`.

---

## F. CRDT Design Detail

v4's CRDT section defined an HLC struct and a few ops; v5 specifies concrete CRDT types, HLC algorithm, OpLog persistence and sync, and conflict policies with examples.

### F1. Concrete CRDT Types

Options: LWW-Register per key.

Buffers: Phase 1 LWW blob per buffer name; Phase 2 RGA.

Topology: OR-Set with tombstones; ids must be globally unique (node id namespace).

Layout intents: LWW on tmux layout string per window.

Concrete op model (builds on the v4 sketch at `notes/2026-02-10-rust-architectural-approach-synthesized-v4.md:1353-1368` but makes the domains explicit):

```rust
// crates/mux-crdt/src/op.rs
#![forbid(unsafe_code)]

use mux_core::options::OptionValue;
use mux_types::{PaneId, WindowId, SessionId};

pub type BufferName = String;
pub type OptionKey = String;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CrdtOp {
    // Options
    OptionSet {
        scope: CrdtScope,
        key: OptionKey,
        value: OptionValue,
        ts: HybridTimestamp,
    },
    OptionUnset {
        scope: CrdtScope,
        key: OptionKey,
        ts: HybridTimestamp,
    },

    // Buffers
    BufferPut {
        name: BufferName,
        bytes: Vec<u8>,
        ts: HybridTimestamp,
    },
    BufferDelete {
        name: BufferName,
        ts: HybridTimestamp,
    },

    // Topology metadata (replicable subset)
    SessionRename { session: SessionId, name: String, ts: HybridTimestamp },
    WindowRename { window: WindowId, name: String, ts: HybridTimestamp },
    PaneRename { pane: PaneId, name: String, ts: HybridTimestamp },

    // Layout intent
    WindowLayoutSet { window: WindowId, layout_tmux: String, ts: HybridTimestamp },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CrdtScope {
    Server,
    Session(SessionId),
    Window(WindowId),
    Pane(PaneId),
}
```

CRDT state (in-memory):

```rust
// crates/mux-crdt/src/state.rs
use std::collections::BTreeMap;

pub struct CrdtState {
    pub options: BTreeMap<(CrdtScope, OptionKey), Lww<OptionValue>>,
    pub buffers: BTreeMap<BufferName, Lww<Vec<u8>>>,
    pub window_layouts: BTreeMap<WindowId, Lww<String>>,
    pub tombstones: Tombstones,
}

pub struct Tombstones {
    // future: ORSWOT / causal contexts
    pub deleted_buffers: BTreeMap<BufferName, HybridTimestamp>,
}
```

### F2. Hybrid Logical Clock (HLC) Implementation

```rust
// crates/mux-crdt/src/clock.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub u128);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HybridTimestamp {
    pub millis: u64,
    pub counter: u32,
    pub node_id: NodeId,
}
```

HLC update rules (implementation-ready; required for convergence):

```rust
pub struct Hlc {
    node_id: NodeId,
    last: HybridTimestamp,
}

impl Hlc {
    pub fn new(node_id: NodeId) -> Self {
        Self {
            node_id,
            last: HybridTimestamp { millis: 0, counter: 0, node_id },
        }
    }

    pub fn now(&mut self, physical_ms: u64) -> HybridTimestamp {
        if physical_ms > self.last.millis {
            self.last = HybridTimestamp { millis: physical_ms, counter: 0, node_id: self.node_id };
        } else {
            self.last.counter = self.last.counter.wrapping_add(1);
        }
        self.last
    }

    pub fn observe(&mut self, remote: HybridTimestamp, physical_ms: u64) -> HybridTimestamp {
        let max_ms = physical_ms.max(self.last.millis).max(remote.millis);
        let next_counter = if max_ms == self.last.millis && max_ms == remote.millis {
            self.last.counter.max(remote.counter).saturating_add(1)
        } else if max_ms == self.last.millis {
            self.last.counter.saturating_add(1)
        } else if max_ms == remote.millis {
            remote.counter.saturating_add(1)
        } else {
            0
        };
        self.last = HybridTimestamp { millis: max_ms, counter: next_counter, node_id: self.node_id };
        self.last
    }
}
```

### F3. OpLog Storage and Sync Protocol

Storage:

- append-only log + checkpoint + metadata.

Sync protocol messages:

- `Hello`, `Have`, `Need`, `Ops`, `Ack`, `Checkpoint`.

Concrete wire framing for collab sync (separate TCP port or WebSocket; does not impact tmux protocol compatibility):

```rust
// crates/mux-crdt-net/src/wire.rs
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SyncMsg {
    Hello { node: NodeId, last_seq: u64 },
    Have { ranges: Vec<(u64, u64)> },
    Need { ranges: Vec<(u64, u64)> },
    Ops { seq_start: u64, ops: Vec<CrdtOp> },
    Ack { upto: u64 },
    Checkpoint { upto: u64, bytes: Vec<u8> },
    Error { message: String },
}

pub fn encode_len_delimited(msg: &SyncMsg, out: &mut Vec<u8>);
pub fn decode_len_delimited(buf: &[u8]) -> Result<(SyncMsg, usize), NetError>;
```

Durability knobs:

- `fsync_every_n_ops` (default 1 in tests, maybe 32 in production)
- `checkpoint_every_n_ops` (default 10000)

### F4. Conflict Policies (Concrete)

1. LWW tie-break: `(millis, counter, node_id)` lexicographic.
2. Delete-wins over write for the same logical key:
   - if `BufferDelete.ts >= BufferPut.ts`, buffer absent.
3. Layout referencing missing panes:
   - `apply_layout_to_window` drops missing leaves and rebalances (tmux closes extra cells at `layout-custom.c:186-201`).

### F4. CRDT Ops Wrapping Core Events

Rule: CRDT ops produce core Events; core remains the deterministic unit.

Concrete conversion boundary:

```rust
// crates/mux-crdt/src/into_events.rs
use mux_core::event::Event;

pub fn op_to_events(op: &CrdtOp) -> Vec<Event> {
    match op {
        CrdtOp::OptionSet { scope, key, value, .. } => vec![Event::OptionSet {
            scope: map_scope(*scope),
            key: key.clone(),
            value: value.clone(),
        }],
        CrdtOp::WindowLayoutSet { window, layout_tmux, .. } => vec![Event::WindowLayoutSet {
            window: *window,
            layout_tmux: layout_tmux.clone(),
        }],
        _ => Vec::new(),
    }
}
```

The server actor applies these Events through `apply_event()` to keep one mutation path.

---

## G. Control Mode Protocol

Control mode is the `tmux -C` line protocol. TermForge supports it as hints; binary protocol is authority.

### G1. Exact `%begin/%end/%error` Guard Protocol

tmux sends guard lines in control mode via `cmdq_guard()`:

- Format: `%%%s %ld %u %d` at `cmd-queue.c:823-833`.
- Guards are emitted around command execution at:
  - begin: `cmd-queue.c:618-620`
  - error: `cmd-queue.c:667-678`
  - end: `cmd-queue.c:679-680`

Prototype parser: `~/work/rust/vibe-tmux/crates/mux-client/src/control.rs:205-265`.

### G2. `%output` / `%extended-output` Stream

tmux emits pane output in control mode using:

- `%extended-output ...` at `control.c:620-626`
- `%output ...` at `control.c:625-636`

These are never authoritative PTY output for TermForge; they are hints for refresh.

---

## H. Server Lifecycle

v5 pins down lifecycle steps using tmux behavior.

### H1. Server Startup Sequence

tmux reference:

- socket creation uses `umask` to restrict permissions at `server.c:126-136`.
- accept loop via libevent at `server.c:367-427`.
- signals trigger shutdown at `server.c:429-460`.

TermForge server startup spec (Rust, `mux-server`):

```rust
// crates/mux-server/src/main.rs
pub struct ServerConfig {
    pub socket_path: std::path::PathBuf,
    pub enable_control_mode: bool,
    pub enable_collab: bool,
    pub recover: bool,
}

pub fn run_server(cfg: ServerConfig) -> Result<(), mux_server::ServerError>;
```

Startup steps (must be executed in this order for parity + safety):

1. Resolve and validate socket directory (create if missing): ensure directory mode `0700`, ensure owned by current uid, reject symlinks/world-writable dirs.
2. Acquire lock file `<socket>.lock` using `flock`, mirroring tmux `client_get_lock()` behavior (`client.c:72-100`).
3. Create Unix socket: use restrictive `umask` like tmux (`server.c:126-136`), then bind/listen and set nonblocking.
4. Initialize: `ServerGraph` + default options + default key tables, telemetry, and CRDT node id + oplog (if enabled).
5. Start accept loop and state actor.
6. Do not load config until first client completes identify (tmux triggers `start_cfg()` at `server-client.c:3725-3734`).

### H2. Lock File / Single-Instance Enforcement

tmux reference:

- client uses `<socket>.lock` with `open(...,0600)` and `flock(LOCK_EX|LOCK_NB)` at `client.c:72-100`, retry logic at `client.c:121-170`.

TermForge should mirror this exactly.

Concrete Rust API:

```rust
// crates/mux-os/src/lock.rs
pub enum LockAcquire {
    Acquired(ServerLock),
    BusyRetry, // someone else is starting server; wait+retry
}

pub fn try_acquire_lock(lock_path: &Path) -> Result<LockAcquire, OsError>;
```

### H3. Client Connection Handshake (Identify Burst)

tmux reference for identify messages:

- handled in `server_client_dispatch_identify()` at `server-client.c:3590-3735`.
- identify message types listed at `server-client.c:3384-3397`.

Handshake state machine (binary protocol, `mux-proto` + `mux-server`):

States:

1. `Connected`
2. `Identifying { collector }`
3. `Ready { client_id }`
4. `Closing { reason }`

Rules derived from tmux:

- Reject any command frames before identify completes.
- Accept identify frames:
  - TERM (`MSG_IDENTIFY_TERM`) must be NUL-terminated; tmux checks `data[datalen-1]=='\\0'` at `server-client.c:3629-3634`.
  - CWD is accepted only if `access(X_OK)==0` else fallback to HOME or `/` at `server-client.c:3649-3658`.
  - STDIN/STDOUT carry an fd via `imsg_get_fd` at `server-client.c:3660-3671`.
- On `MSG_IDENTIFY_DONE`:
  - set `CLIENT_IDENTIFIED`
  - name client as ttyname if present else `client-<pid>` at `server-client.c:3698-3702`
  - if control mode: start it at `server-client.c:3710-3713`

TermForge Rust types:

```rust
// crates/mux-server/src/conn.rs
pub struct ClientConn {
    pub id: mux_types::ClientId,
    pub codec: mux_proto::ImsgCodec,
    pub state: ConnState,
}

pub enum ConnState {
    Identifying(mux_proto::IdentifyCollector),
    Ready(ReadyConn),
    Closing { reason: String },
}
```

Malformed frames:

- Any `ProtoError::Frame` or `ProtoError::Payload` is a `ProtocolViolation` -> close this connection, like tmux kills the peer at `server-client.c:3472-3475`.

### H4. Graceful Shutdown

tmux reference:

- `server_send_exit()` kills clients and destroys sessions at `server.c:308-329`.
- loop exits only when conditions satisfied at `server.c:281-305`.

TermForge shutdown spec:

1. Stop accepting new clients.
2. For each connected client: send an exiting notification and close after flushing pending output.
3. Drain effect workers with a timeout; then kill remaining PTY child processes if configured.
4. Persist state (best-effort): options, topology metadata, layouts, buffers, key tables, and CRDT checkpoint if enabled.
5. Close socket and remove lock file.

---

## I. Security Model

### I1. Socket Permissions

tmux reference:

- `umask` at `server.c:126-136`.
- execute bits toggled at `server.c:331-364`.

TermForge policy:

- Socket directory: `0700`.
- Socket file: `0600` by default.
- Optional: mimic tmux's execute-bit toggling to indicate attached sessions (tmux does this in `server_update_socket()` at `server.c:331-364`).

Concrete implementation (`mux-os`):

```rust
// crates/mux-os/src/socket.rs
pub fn create_unix_listener(path: &Path) -> Result<std::os::unix::net::UnixListener, OsError>;
pub fn set_socket_mode(path: &Path, mode: u32) -> Result<(), OsError>;
pub fn validate_socket_dir(dir: &Path) -> Result<(), OsError>;
```

Validation rules:

- Reject socket dir if owned by different uid.
- Reject if permissions allow group/other write, unless sticky bit semantics are explicitly supported.
- Reject symlink components in socket path.

### I2. SCM_RIGHTS FD Passing

tmux uses fd passing for stdin/stdout identify (`imsg_get_fd`) at `server-client.c:3663-3671`.

Policy:

- Only accept fds for expected identify messages.
- Validate fd type and set CLOEXEC.

Concrete validation:

```rust
// crates/mux-os/src/fd.rs
pub fn set_cloexec(fd: std::os::unix::io::RawFd) -> Result<(), OsError>;

pub enum FdKind { Tty, Pty, RegularFile, Unknown }
pub fn classify_fd(fd: std::os::unix::io::RawFd) -> Result<FdKind, OsError>;
```

Security invariants:

- Never accept more than one STDIN/STDOUT fd per connection.
- Never accept ancillary fds outside handshake; treat as protocol violation.

### I3. Input Validation Boundaries

Untrusted inputs:

- binary protocol frames (network)
- control mode lines (text stream)
- config files (filesystem)
- binding calls (FFI)

Boundaries:

- `mux-proto` must enforce max frame size (prototype uses `MAX_IMSG_SIZE=16384` at `~/work/rust/vibe-tmux/crates/mux-proto/src/error.rs:10-11`).
- `mux-command` validates args; core still checks existence and returns `CoreError::NotFound`.

### I4. Sandboxing Pane Processes (Optional, Planned)

Keep the API ready even if unimplemented initially:

```rust
// crates/mux-os/src/spawn.rs
pub struct SandboxOptions {
    pub enabled: bool,
    pub allow_network: bool,
    pub fs_roots: Vec<PathBuf>,
}
```

---

## J. Performance Targets

### J1. Latency Budgets

Targets (P50 / P99):

- Input -> visible update: 4ms / 16ms
- PTY output -> grid update: 2ms / 8ms
- Command -> ack/refresh: 5ms / 25ms

### J2. Memory Budget per Pane

- Grid + scrollback: <= 10 MB typical.

### J3. VT100 Parser Throughput

- sustain 50 MB/s parsing throughput in release mode.

### J4. Benchmark Infrastructure

Use `criterion` for microbenchmarks (parser, layout rebuild, format expansion).

Add scenario replay benchmarks using recorded PTY scenarios.

Concrete workspace additions:

- `crates/mux-bench/` (IMPURE, uses criterion)
- `crates/mux-bench/benches/vt100.rs`
- `crates/mux-bench/benches/layout.rs`
- `crates/mux-bench/benches/format.rs`

Benchmark harness API (scenario replay):

```rust
// crates/mux-bench/src/scenario.rs
pub struct ScenarioBenchResult {
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub events_applied: u64,
    pub elapsed_ms: u64,
}

pub fn run_scenario(path: &Path) -> Result<ScenarioBenchResult, BenchError>;

#[derive(Debug, thiserror::Error)]
pub enum BenchError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse: {0}")]
    Parse(String),
}
```

CI gates:

- Add a non-flaky perf regression gate only for gross regressions (e.g. vt100 throughput must not drop more than 30% vs baseline) and run it nightly.

Metrics to record in benches:

- allocations/op (when built with allocator instrumentation)
- bytes/sec for parser
- layout build time vs N panes

---

## Updated DOs/DON'Ts (Additions Beyond v4)

### DOs (add)

1. DO define a crate-local `Error` enum for every crate; `anyhow` is allowed only in binaries/tests, not libraries.
2. DO classify untrusted-input failures as `NeedMore` vs `ProtocolViolation` and document the recovery action (close connection vs resync).
3. DO ensure config reload produces core Events rather than mutating state directly.
4. DO match tmux layout string checksum and grammar exactly (see `layout-custom.c:45-117`).
5. DO propagate OTEL headers across threads and helper process spawns (pattern: `~/work/rust/vibe-tmux/crates/mux-api/src/managed.rs:160-220`, `~/work/rust/vibe-tmux/crates/mux-server/src/telemetry_env.rs:49-57`).

### DON'Ts (add)

1. DON'T treat control mode output as authoritative state; it is hints only.
2. DON'T leak `io::Error` or platform types across the pure boundary; convert to stable events.
3. DON'T accept SCM_RIGHTS fds except in expected identify frames; validate fd types.

---

## Updated AGENTS.md Rules (Additions to v4 Template)

Add these rules to the v4 `AGENTS.md` template section:

1. Error Taxonomy Rule: every crate defines a public `Error` enum and `Result<T>` alias; no `anyhow` in library crates.
2. Protocol Violation Rule: any binary decode error other than "need more bytes" closes the connection and records a structured error event.
3. Config-as-Events Rule: config execution produces `Vec<Event>` and submits them through the actor.
4. Layout String Parity Rule: layout dump/parse must match `layout-custom.c`.
5. Telemetry Propagation Rule: any new thread/task must attach OTEL context by pushing trace headers (see `~/work/rust/vibe-tmux/crates/mux-api/src/managed.rs:160-220`).
6. Security Boundary Rule: treat sockets/control/config/bindings as untrusted; validate at boundaries.

---

## Additional Architectural Insights from Reference Codebases

1. tmux layout strings are defined in `layout-custom.c`, not `layout.c`. Checksum and encoding are explicit (`layout-custom.c:45-117`).
2. tmux config load is triggered only after first client finishes identify (`server-client.c:3725-3734`).
3. Control mode is a guard-delimited protocol (`cmd-queue.c:618-680`, formatting at `cmd-queue.c:823-833`) and matches the prototype parser (`~/work/rust/vibe-tmux/crates/mux-client/src/control.rs:205-265`).
4. Options in tmux are a parent chain (`options.c:228-240`), not a merged map.
5. OTEL propagation patterns in the Rust prototype are already aligned with TermForge needs (`~/work/rust/vibe-tmux/crates/mux-otel/src/lib.rs:31-41`, `~/work/rust/vibe-tmux/crates/mux-api/src/managed.rs:160-220`).
