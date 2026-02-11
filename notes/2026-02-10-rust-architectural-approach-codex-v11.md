# TermForge v11 Architecture Specification (Codex Variant)

Date: 2026-02-11  
Status: Definitive v11 refresh (post-v10 hardening)  
Lineage: v4 -> v5 -> v6 -> v7 -> v8 -> v9 -> v10 -> v11  
License: MIT OR Apache-2.0  
Rust edition: 2024 (MSRV 1.85)  
Protocol target: tmux protocol v8

---

## Preamble

This is a complete standalone architecture for TermForge: a new Rust multiplexer that is wire-protocol compatible with tmux, not a port. v11 strengthens traceability, closes residual implementation ambiguity, and deepens Section 32 (Termlets) so an LLM can implement directly.

Traceability convention used throughout:
- `INV-*`: core invariants
- `API-*`: API contracts
- `PAR-*`: parity contracts against tmux/libtmux/reference crates
- `OPS-*`: runtime/operational contracts
- `TST-*`: mandatory test gates

Settled global invariants:
- `INV-001`: Protocol compatibility target is tmux protocol v8 (`tmux-protocol.h:23`).
- `INV-002`: `mux-core` is deterministic and IO-free (`#![forbid(unsafe_code)]`).
- `INV-003`: Single writer for state mutation; snapshot-based readers.
- `INV-004`: Protocol violation drops client connection, never partial-continue (`server-client.c:3472-3475`).
- `INV-005`: First-client identify completes before config load path (`server-client.c:3725-3734`).
- `INV-006`: Layout resize uses round-robin one-cell adjustment (`layout.c:448-462`).
- `INV-007`: Socket/test isolation uses three guard layers (path guard pattern).
- `INV-008`: Termlets are pane-backed and never a parallel terminal stack.

---

## 1. Project Identity

### Design Decisions

- Product name: `TermForge`.
- CLI executable: `termforge` with optional short alias `tf`.
- Rust crates use `mux-*` prefix for grep-ability and layering clarity.
- Public binding package name is unified: `termforge` for Python and Node.
- Config path: `${XDG_CONFIG_HOME:-~/.config}/termforge/termforge.conf`.
- Default socket root: `${TMPDIR}/termforge-<uid>/`.
- Traceability: `API-001`, `OPS-001`.

### Rust Example

```rust
use std::path::PathBuf;

pub const PROJECT_NAME: &str = "TermForge";
pub const BIN_NAME: &str = "termforge";
pub const BIN_ALIAS: &str = "tf";

pub fn config_path() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"));
    base.join("termforge").join("termforge.conf")
}

pub fn default_socket_root(uid: u32) -> PathBuf {
    std::env::temp_dir().join(format!("termforge-{uid}"))
}
```

### Test Strategy

1. `TST-001`: config path resolution with and without `XDG_CONFIG_HOME`.
2. `TST-002`: default socket root format includes uid.
3. `TST-003`: binary naming sanity checks in packaging tests.

### AGENTS.md Rules

- `RULE-S01-01`: New crates must keep `mux-*` prefix.
- `RULE-S01-02`: External docs/examples must use `termforge` binary name.

---

## 2. Acceptance Criteria and Gates

### Design Decisions

Release is blocked on compatibility and correctness gates; performance gates are regression-guarded. All gates map to concrete tests.

Gate groups:
- Compatibility: `C*`
- Purity/architecture: `A*`
- Performance: `B*`
- Bindings/API: `P*`
- Ecosystem/release: `E*`

Key v11 additions:
- `C6`: control-mode backpressure semantics parity.
- `P10`: Termlet background task lifecycle contract.
- `A3`: cross-section traceability lint (every invariant mapped to tests).

### Rust Example

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Gate {
    C1WireAttach,
    C2ListSessions,
    C3ControlNotifications,
    C4ConfigLoadOrder,
    C5OptionScopeFallback,
    C6ControlBackpressure,
    A1CoreNoUnsafe,
    A2CoreNoIo,
    A3InvariantTraceability,
    B1ProtoDecode,
    B2VtThroughput,
    B3LayoutResize,
    B4TermletSpawnFake,
    B5TermletSpawnReal,
    P1PyQueryKwargs,
    P2NodeQueryParity,
    P3CrdtConvergence,
    P4OtelPropagation,
    P5FakePtyDeterminism,
    P6TermletSnapshotParity,
    P7TermletWaitFor,
    P8TermletPool,
    P9TermletDiff,
    P10TermletBackgroundTasks,
    E1Docs,
    E2Msrv,
    E3License,
}

impl Gate {
    pub fn release_blocking(self) -> bool {
        !matches!(self, Gate::B1ProtoDecode | Gate::B2VtThroughput | Gate::B3LayoutResize | Gate::B4TermletSpawnFake | Gate::B5TermletSpawnReal)
    }
}
```

### Test Strategy

1. `TST-010`: gate inventory exists and is complete.
2. `TST-011`: CI fails on any blocking gate red.
3. `TST-012`: performance regression budget >30% marks failure.

### AGENTS.md Rules

- `RULE-S02-01`: Each new feature must add at least one gate.
- `RULE-S02-02`: No release tag unless all blocking gates pass.

---

## 3. Crate Dependency Rules

### Design Decisions

Layering is strict and one-way:
- Layer 0 Pure primitives: `mux-types`, `mux-grid`, `mux-query`, `mux-crdt`, `mux-proto`.
- Layer 1 Core semantics: `mux-core`, `mux-conf`, `mux-keys`, `mux-format`.
- Layer 2 Adapters/facades: `mux-runtime`, `mux-api`, `mux-orm`, `mux-termlet`.
- Layer 3 Integrations: bindings, tui, tools.

Forbidden edges are explicit (`A2`). `mux-termlet` may depend on `mux-grid` + `mux-pty` abstractions but not `mux-server`.

### Rust Example

```rust
/// Declarative dependency policy used by architecture tests.
#[derive(Debug)]
pub struct CrateLayer {
    pub name: &'static str,
    pub layer: u8,
}

pub const CRATE_LAYERS: &[CrateLayer] = &[
    CrateLayer { name: "mux-types", layer: 0 },
    CrateLayer { name: "mux-grid", layer: 0 },
    CrateLayer { name: "mux-proto", layer: 0 },
    CrateLayer { name: "mux-query", layer: 0 },
    CrateLayer { name: "mux-crdt", layer: 0 },
    CrateLayer { name: "mux-core", layer: 1 },
    CrateLayer { name: "mux-runtime", layer: 2 },
    CrateLayer { name: "mux-api", layer: 2 },
    CrateLayer { name: "mux-termlet", layer: 2 },
    CrateLayer { name: "bindings-python", layer: 3 },
    CrateLayer { name: "bindings-node", layer: 3 },
];

pub fn edge_allowed(from: u8, to: u8) -> bool {
    to <= from
}
```

### Test Strategy

1. `TST-020`: dependency graph linter checks no forbidden edges.
2. `TST-021`: `mux-core` transitive dependencies audited for IO crates.
3. `TST-022`: `mux-termlet` dependency deny-list enforcement.

### AGENTS.md Rules

- `RULE-S03-01`: New crate proposals must declare layer and inbound/outbound edges.
- `RULE-S03-02`: Architecture tests are mandatory for dependency exceptions.

---

## 4. Workspace Layout

### Design Decisions

Workspace is implementation-oriented and test-first. v11 keeps explicit place for parity runners and binding fixtures.

### Rust Example

```rust
pub const WORKSPACE_LAYOUT: &str = r#"
termforge/
  crates/
    mux-types/
    mux-core/
    mux-grid/
    mux-proto/
    mux-query/
    mux-crdt/
    mux-keys/
    mux-format/
    mux-conf/
    mux-runtime/
    mux-api/
    mux-orm/
    mux-pty/
    mux-pty-fake/
    mux-termlet/
    mux-otel/
    mux-test-support/
    mux-view/
  bindings/
    python/
    node/
  tools/
    mux-regress/
    tmux-builder/
"#;
```

### Test Strategy

1. `TST-030`: workspace manifest includes expected crates.
2. `TST-031`: `cargo check --workspace` on stable and MSRV.
3. `TST-032`: smoke examples compile in docs for each top-level crate.

### AGENTS.md Rules

- `RULE-S04-01`: Keep test-support and parity tools under `tools/` or dedicated crates.
- `RULE-S04-02`: Avoid creating mixed-purpose crates.

---

## 5. Core State Graph

### Design Decisions

`ServerGraph` is canonical mutable state; `GraphSnapshot` is immutable read model.
- Entity IDs use slotmap keys.
- Parent->children relations stored directly on owning entities for write simplicity.
- Reverse lookups generated in snapshots.
- Mutation only through reducer (`INV-003`).

### Rust Example

```rust
use slotmap::{new_key_type, SlotMap};

new_key_type! { pub struct SessionId; }
new_key_type! { pub struct WindowId; }
new_key_type! { pub struct PaneId; }

#[derive(Debug, Clone)]
pub struct Session {
    pub name: String,
    pub windows: Vec<WindowId>,
}

#[derive(Debug, Clone)]
pub struct Window {
    pub name: String,
    pub panes: Vec<PaneId>,
}

#[derive(Debug, Clone)]
pub struct Pane {
    pub title: String,
}

#[derive(Default)]
pub struct ServerGraph {
    pub sessions: SlotMap<SessionId, Session>,
    pub windows: SlotMap<WindowId, Window>,
    pub panes: SlotMap<PaneId, Pane>,
}
```

### Test Strategy

1. `TST-040`: parent-child relationship integrity under create/delete.
2. `TST-041`: snapshot reverse map correctness.
3. `TST-042`: fuzz graph mutation sequences for dangling references.

### AGENTS.md Rules

- `RULE-S05-01`: Graph writes must occur only inside reducer/event apply path.
- `RULE-S05-02`: Snapshot generation must never mutate graph.

---

## 6. Entity Relationships

### Design Decisions

Relationship invariants:
- `INV-101`: each pane belongs to exactly one window.
- `INV-102`: each window belongs to exactly one session.
- `INV-103`: order is explicit via vectors and must be stable.

### Rust Example

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationshipError(&'static str);

pub fn validate_session_window_relations(
    graph: &ServerGraph,
) -> Result<(), RelationshipError> {
    for (_sid, session) in &graph.sessions {
        for wid in &session.windows {
            if !graph.windows.contains_key(*wid) {
                return Err(RelationshipError("session references missing window"));
            }
        }
    }

    for (_wid, window) in &graph.windows {
        for pid in &window.panes {
            if !graph.panes.contains_key(*pid) {
                return Err(RelationshipError("window references missing pane"));
            }
        }
    }
    Ok(())
}
```

### Test Strategy

1. `TST-050`: relationship validator catches orphan IDs.
2. `TST-051`: reorder operations preserve membership.
3. `TST-052`: proptest random edit sequences keep invariants.

### AGENTS.md Rules

- `RULE-S06-01`: any event adding/removing entities must update parent vectors atomically.
- `RULE-S06-02`: invariant failures are bugs, not user errors.

---

## 7. Event and Effect Enums

### Design Decisions

Event-effect split is strict:
- `Event`: state mutation intent.
- `Effect`: side-effect request for runtime executor.
- Reducer returns deterministic `ApplyOutcome { effects, hints }`.

### Rust Example

```rust
#[derive(Debug, Clone)]
pub enum Event {
    SessionCreate { name: String },
    WindowCreate { session_id: SessionId, name: String },
    PaneSendKeys { pane_id: PaneId, keys: Vec<u8> },
}

#[derive(Debug, Clone)]
pub enum Effect {
    PtyWrite { pane_id: PaneId, bytes: Vec<u8> },
    NotifyControl { line: String },
    PersistSnapshot,
}

#[derive(Debug, Default)]
pub struct ApplyOutcome {
    pub effects: Vec<Effect>,
    pub hints: Vec<&'static str>,
}
```

### Test Strategy

1. `TST-060`: each event has deterministic effect list for same input.
2. `TST-061`: no IO operations inside reducer path.
3. `TST-062`: effect interpreter retries policy covered by integration tests.

### AGENTS.md Rules

- `RULE-S07-01`: do not emit side effects directly from command parsing layers.
- `RULE-S07-02`: all new effect variants require runtime handling and tests.

---

## 8. Error Handling

### Design Decisions

- Library crates use `thiserror`; binaries/tools may use `anyhow`.
- Error classes: user, transient, bug.
- Binding layers preserve stable error codes.

### Rust Example

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("not found: {0}")]
    NotFound(&'static str),
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    #[error("invariant violation: {0}")]
    InvariantViolation(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorClass {
    User,
    Transient,
    Bug,
}
```

### Test Strategy

1. `TST-070`: mapping from internal errors to binding exceptions is stable.
2. `TST-071`: invariant violations panic only in debug; classified as bug in release path.
3. `TST-072`: error code snapshots for Python/Node APIs.

### AGENTS.md Rules

- `RULE-S08-01`: never expose Rust backtraces as primary user message in bindings.
- `RULE-S08-02`: add explicit error code for every new public error variant.

---

## 9. Wire Protocol

### Design Decisions

- Protocol compatibility target is tmux v8 (`INV-001`).
- Decoder is strict and connection-fatal on unknown/invalid message (`INV-004`).
- Use framed codec with bounded payload size.

### Rust Example

```rust
use bytes::{Buf, BufMut, BytesMut};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImsgHdr {
    pub msg_type: u32,
    pub len: u32,
}

pub fn decode_header(src: &mut BytesMut) -> Result<Option<ImsgHdr>, &'static str> {
    const HDR_LEN: usize = 8;
    if src.len() < HDR_LEN {
        return Ok(None);
    }
    let mut peek = &src[..HDR_LEN];
    let msg_type = peek.get_u32_le();
    let len = peek.get_u32_le();
    if len < HDR_LEN as u32 || len > 16 * 1024 * 1024 {
        return Err("invalid frame length");
    }
    src.advance(HDR_LEN);
    Ok(Some(ImsgHdr { msg_type, len }))
}

pub fn encode_header(dst: &mut BytesMut, hdr: ImsgHdr) {
    dst.put_u32_le(hdr.msg_type);
    dst.put_u32_le(hdr.len);
}
```

### Test Strategy

1. `TST-090`: roundtrip tests for all supported msg types.
2. `TST-091`: fuzz decoder with malformed lengths/types.
3. `TST-092`: protocol violation forces disconnect integration test.

### AGENTS.md Rules

- `RULE-S09-01`: protocol parser changes require compatibility fixture updates.
- `RULE-S09-02`: never ignore unknown message types in server mode.

---

## 10. Configuration Engine

### Design Decisions

- Config load starts after first client identify flow (`INV-005`).
- Parser lowers to typed commands/events; execution happens in reducer.
- Supports partial tmux config with explicit unsupported diagnostics.

### Rust Example

```rust
#[derive(Debug, Clone)]
pub enum ConfStmt {
    SetOption { scope: String, key: String, value: String },
    BindKey { table: String, key: String, command: String },
}

pub fn parse_line(line: &str) -> Option<ConfStmt> {
    let line = line.trim();
    if line.starts_with("set -g ") {
        let rest = line.trim_start_matches("set -g ");
        let mut parts = rest.splitn(2, ' ');
        let key = parts.next()?.to_string();
        let value = parts.next().unwrap_or_default().to_string();
        return Some(ConfStmt::SetOption {
            scope: "global".to_string(),
            key,
            value,
        });
    }
    None
}
```

### Test Strategy

1. `TST-100`: parse fixtures from representative `.tmux.conf` fragments.
2. `TST-101`: unsupported directives produce stable diagnostics.
3. `TST-102`: first-client load ordering integration test.

### AGENTS.md Rules

- `RULE-S10-01`: config parser must be pure.
- `RULE-S10-02`: config execution emits events; no direct state mutation.

---

## 11. Layout Engine

### Design Decisions

- Arena-based tree with index references.
- Resize algorithm follows tmux round-robin cell-at-a-time (`INV-006`).
- Layout checksum algorithm mirrors tmux `layout-custom.c` behavior for parity checks.

### Rust Example

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitAxis {
    Horizontal,
    Vertical,
}

pub fn round_robin_resize(child_sizes: &mut [u16], mut delta: i32) {
    if child_sizes.is_empty() || delta == 0 {
        return;
    }
    while delta != 0 {
        for size in child_sizes.iter_mut() {
            if delta == 0 {
                break;
            }
            if delta > 0 {
                *size = size.saturating_add(1);
                delta -= 1;
            } else if *size > 1 {
                *size -= 1;
                delta += 1;
            }
        }
    }
}

pub fn layout_checksum(layout: &str) -> u16 {
    let mut csum: u16 = 0;
    for b in layout.bytes() {
        csum = (csum >> 1) | ((csum & 1) << 15);
        csum = csum.wrapping_add(u16::from(b));
    }
    csum
}
```

### Test Strategy

1. `TST-110`: golden tests against tmux resize fixtures.
2. `TST-111`: checksum parity vectors with tmux outputs.
3. `TST-112`: proptest for resize invariants (sum/limits).

### AGENTS.md Rules

- `RULE-S11-01`: never replace round-robin with proportional resize in compatibility mode.
- `RULE-S11-02`: checksum algorithm changes require parity proof.

---

## 12. ORM Query Layer

### Design Decisions

`QueryList` semantics align with libtmux and add explicit extensions.

Supported operators:
- libtmux parity: `eq`, `exact`, `iexact`, `contains`, `icontains`, `startswith`, `istartswith`, `endswith`, `iendswith`, `in`, `nin`, `regex`, `iregex` (`PAR-120`).
- extensions: numeric comparisons and optionals may exist only in Rust core query DSL, not required in Python parity mode.

Required behaviors:
- kwargs filtering with `field__op` syntax.
- nested `__` traversal keygetter semantics.
- callable matcher support.
- `.get(default=...)` behavior with `ObjectDoesNotExist`/`MultipleObjectsReturned` mapping.

### Rust Example

```rust
use regex::Regex;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryOp {
    Eq,
    IExact,
    Contains,
    IContains,
    StartsWith,
    IStartsWith,
    EndsWith,
    IEndsWith,
    In,
    NotIn,
    Regex,
    IRegex,
}

pub fn parse_lookup(raw: &str) -> (&str, QueryOp) {
    let mut parts = raw.rsplitn(2, "__");
    let tail = parts.next().unwrap_or(raw);
    let head = parts.next().unwrap_or(raw);
    let op = match tail {
        "eq" | "exact" => QueryOp::Eq,
        "iexact" => QueryOp::IExact,
        "contains" => QueryOp::Contains,
        "icontains" => QueryOp::IContains,
        "startswith" => QueryOp::StartsWith,
        "istartswith" => QueryOp::IStartsWith,
        "endswith" => QueryOp::EndsWith,
        "iendswith" => QueryOp::IEndsWith,
        "in" => QueryOp::In,
        "nin" => QueryOp::NotIn,
        "regex" => QueryOp::Regex,
        "iregex" => QueryOp::IRegex,
        _ => return (raw, QueryOp::Eq),
    };
    (head, op)
}

pub fn apply_regex(value: &str, pat: &str, ci: bool) -> bool {
    let expr = if ci { format!("(?i){pat}") } else { pat.to_string() };
    Regex::new(&expr).map(|re| re.is_match(value)).unwrap_or(false)
}

pub fn nested_get<'a>(obj: &'a HashMap<String, String>, key: &str) -> Option<&'a str> {
    // Placeholder for uniform keygetter shape in examples.
    obj.get(key).map(String::as_str)
}
```

### Test Strategy

1. `TST-120`: operator matrix parity against libtmux operator set.
2. `TST-121`: nested key traversal tests with dict/object-like fixtures.
3. `TST-122`: `.get(default=...)` and exception parity tests in Python binding.

### AGENTS.md Rules

- `RULE-S12-01`: Python bindings must expose kwargs and callable matcher paths.
- `RULE-S12-02`: Any operator addition must define parity vs extension status.

---

## 13. Concurrency Model

### Design Decisions

- Single-writer actor owns mutable graph.
- Readers consume immutable snapshots via `ArcSwap`.
- Runtime tasks communicate by bounded channels with explicit backpressure strategy.

### Rust Example

```rust
use arc_swap::ArcSwap;
use std::sync::Arc;

#[derive(Debug, Default)]
pub struct GraphSnapshot {
    pub generation: u64,
}

#[derive(Default)]
pub struct SnapshotStore {
    inner: ArcSwap<GraphSnapshot>,
}

impl SnapshotStore {
    pub fn new() -> Self {
        Self {
            inner: ArcSwap::new(Arc::new(GraphSnapshot::default())),
        }
    }

    pub fn publish(&self, snap: GraphSnapshot) {
        self.inner.store(Arc::new(snap));
    }

    pub fn load(&self) -> Arc<GraphSnapshot> {
        self.inner.load_full()
    }
}
```

### Test Strategy

1. `TST-130`: no reader lock contention under write load.
2. `TST-131`: bounded channel overflow behavior for control mode.
3. `TST-132`: loom-style model checks for actor message ordering.

### AGENTS.md Rules

- `RULE-S13-01`: keep mutable graph in one task/thread.
- `RULE-S13-02`: never expose mutable references across API boundaries.

---

## 14. Server Lifecycle

### Design Decisions

Lifecycle phases:
1. bootstrap
2. socket bind/lock
3. client identify handshake
4. config load gate
5. steady-state
6. graceful shutdown

Locking compatibility uses `flock(LOCK_EX|LOCK_NB)` semantics (`client.c:89`).

### Rust Example

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerPhase {
    Bootstrap,
    SocketReady,
    AwaitIdentify,
    ConfigLoading,
    Running,
    ShuttingDown,
    Stopped,
}

#[derive(Debug)]
pub struct ServerLifecycle {
    pub phase: ServerPhase,
}

impl ServerLifecycle {
    pub fn transition(&mut self, next: ServerPhase) {
        self.phase = next;
    }
}
```

### Test Strategy

1. `TST-140`: ordered phase transition tests.
2. `TST-141`: lock contention behavior (second process waits/retries).
3. `TST-142`: shutdown drains pending outbound frames.

### AGENTS.md Rules

- `RULE-S14-01`: lifecycle transitions must be observable in tracing/logs.
- `RULE-S14-02`: shutdown path must be idempotent.

---

## 15. Control Mode

### Design Decisions

- Support `tmux -C` style stream semantics.
- Notifications are typed internally, string-rendered at boundary.
- Backpressure: if client is too far behind, apply pause-after semantics or disconnect (`control.c:450-461`).

### Rust Example

```rust
#[derive(Debug, Clone)]
pub enum ControlNotification {
    SessionChanged(String),
    WindowChanged(String),
    PaneOutput { pane_id: u32, data: String },
    Pause(u32),
}

pub fn render_notification(n: &ControlNotification) -> String {
    match n {
        ControlNotification::SessionChanged(s) => format!("%session-changed {s}"),
        ControlNotification::WindowChanged(w) => format!("%window-changed {w}"),
        ControlNotification::PaneOutput { pane_id, data } => {
            format!("%output %{} {}", pane_id, data)
        }
        ControlNotification::Pause(pane_id) => format!("%pause %{}", pane_id),
    }
}
```

### Test Strategy

1. `TST-150`: parse/render roundtrip of notifications.
2. `TST-151`: backlog thresholds produce pause/disconnect behavior.
3. `TST-152`: extended output formatting parity fixtures.

### AGENTS.md Rules

- `RULE-S15-01`: control-mode output must be fully deterministic for same event stream.
- `RULE-S15-02`: backlog policy changes require perf + parity evidence.

---

## 16. Language Bindings

### Design Decisions

Bindings expose equivalent high-level APIs:
- Rust: direct typed API.
- Python (PyO3): libtmux-like object model and query kwargs semantics.
- Node (Neon): parity API with plain JS objects and async-friendly methods.

v11 closes v10 ambiguity by defining binding-native `QueryList` wrappers explicitly.

### Rust Example

```rust
pub trait BindingQueryList<T> {
    fn len(&self) -> usize;
    fn filter_kwargs(&self, kwargs: &[(&str, &str)]) -> Vec<T>;
    fn get_kwargs(&self, kwargs: &[(&str, &str)]) -> Result<T, &'static str>;
}

#[derive(Debug, Clone)]
pub struct SessionView {
    pub name: String,
}

pub struct PyQueryList<T> {
    pub items: Vec<T>,
}

impl<T: Clone> BindingQueryList<T> for PyQueryList<T> {
    fn len(&self) -> usize {
        self.items.len()
    }

    fn filter_kwargs(&self, _kwargs: &[(&str, &str)]) -> Vec<T> {
        self.items.clone()
    }

    fn get_kwargs(&self, _kwargs: &[(&str, &str)]) -> Result<T, &'static str> {
        self.items.first().cloned().ok_or("ObjectDoesNotExist")
    }
}
```

### Test Strategy

1. `TST-160`: Python `QueryList.filter(**kwargs)` parity tests.
2. `TST-161`: Python `.get(default=...)` behavior with exact exception mapping.
3. `TST-162`: Node query wrapper parity with same filter fixtures.

### AGENTS.md Rules

- `RULE-S16-01`: all public binding methods require cross-language parity tests.
- `RULE-S16-02`: binding API additions must be reflected in Rust facade contracts.

---

## 17. CRDT Transaction Layer

### Design Decisions

CRDT is scoped, not universal:
- Replicate collaborative metadata first (marks, annotations, commands), not raw terminal byte streams.
- Use HLC timestamps and op-log for deterministic merge.
- Keep pane output/event logs out of CRDT consensus path.

### Rust Example

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Hlc {
    pub wall_millis: u64,
    pub counter: u32,
    pub node: u32,
}

#[derive(Debug, Clone)]
pub enum CrdtOp {
    SetTitle { pane: PaneId, title: String, ts: Hlc },
    AddTag { pane: PaneId, tag: String, ts: Hlc },
    RemoveTag { pane: PaneId, tag: String, ts: Hlc },
}

pub fn lww_merge<T: Clone>(a: (Hlc, T), b: (Hlc, T)) -> (Hlc, T) {
    if b.0 >= a.0 { b } else { a }
}
```

### Test Strategy

1. `TST-170`: two/three replica convergence tests.
2. `TST-171`: commutativity/associativity property tests on op logs.
3. `TST-172`: replay determinism tests across randomized op order.

### AGENTS.md Rules

- `RULE-S17-01`: CRDT scope changes must document conflict policy.
- `RULE-S17-02`: every new replicated field needs merge-law tests.

---

## 18. Security Model

### Design Decisions

- Unix-socket ACLs and path ownership checks.
- No shell interpolation in command execution APIs; explicit argv required where possible.
- Binding input validation on untrusted strings.
- Test harness isolation must prevent accidental attachment to user tmux.

### Rust Example

```rust
use std::path::Path;

pub fn validate_socket_permissions(meta_mode: u32) -> Result<(), &'static str> {
    // require user-only rwx on socket directory
    if meta_mode & 0o077 != 0 {
        return Err("socket dir permissions too broad");
    }
    Ok(())
}

pub fn reject_default_socket_name(name: &str) -> Result<(), &'static str> {
    if name.trim().is_empty() || name == "default" {
        return Err("unsafe socket name");
    }
    Ok(())
}

pub fn ensure_within(base: &Path, candidate: &Path) -> Result<(), &'static str> {
    if !candidate.starts_with(base) {
        return Err("socket path escapes sandbox");
    }
    Ok(())
}
```

### Test Strategy

1. `TST-180`: permission-mode enforcement tests.
2. `TST-181`: path traversal rejection tests.
3. `TST-182`: reject default socket name and `$TMUX` collisions.

### AGENTS.md Rules

- `RULE-S18-01`: all filesystem paths must be validated before destructive operations.
- `RULE-S18-02`: never run shell via concatenated strings from user input.

---

## 19. OpenTelemetry

### Design Decisions

v11 aligns with dual-provider architecture verified in `mux-otel` reference:
- global provider (`OTEL_PROVIDER`) for server/tui
- lazy client provider (`MUX_CLIENT_PROVIDER`) for per-client spans

Context propagation:
- composite propagator: baggage + tracecontext
- thread-local header stack with RAII guard
- optional process-level override headers for cross-thread handoff

### Rust Example

```rust
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Default)]
pub struct OtelProviderHandle {
    pub enabled: bool,
}

static OTEL_PROVIDER: OnceLock<Mutex<Option<OtelProviderHandle>>> = OnceLock::new();
static MUX_CLIENT_PROVIDER: OnceLock<Mutex<Option<OtelProviderHandle>>> = OnceLock::new();

pub fn init_global_otel() {
    let slot = OTEL_PROVIDER.get_or_init(|| Mutex::new(None));
    let mut guard = slot.lock().expect("poisoned");
    if guard.is_none() {
        *guard = Some(OtelProviderHandle { enabled: true });
    }
}

pub fn enter_mux_client_span() {
    let slot = MUX_CLIENT_PROVIDER.get_or_init(|| Mutex::new(None));
    let mut guard = slot.lock().expect("poisoned");
    if guard.is_none() {
        *guard = Some(OtelProviderHandle { enabled: true });
    }
}

pub fn shutdown_otel() {
    if let Some(slot) = OTEL_PROVIDER.get() {
        let _ = slot.lock().map(|mut g| g.take());
    }
    if let Some(slot) = MUX_CLIENT_PROVIDER.get() {
        let _ = slot.lock().map(|mut g| g.take());
    }
}
```

### Test Strategy

1. `TST-190`: dual-provider lazy-init behavior.
2. `TST-191`: context propagation with traceparent+baggage across binding/server boundaries.
3. `TST-192`: shutdown flush + drop semantics for short-lived processes.

### AGENTS.md Rules

- `RULE-S19-01`: OTEL integration must be feature-gated and safe when disabled.
- `RULE-S19-02`: trace context handoff paths require integration tests per binding.

---

## 20. tmux Version Management

### Design Decisions

v11 codifies reference-proven safety model:
- BLAKE3 fingerprints for configure/make flags.
- cache key includes version/host/os/hash/tool version.
- file locks for clone + per-cache-key build.
- temp sibling build dir with atomic rename publication.
- runtime validation via `tmux -V`.

### Rust Example

```rust
pub fn flags_fingerprint(flags: &[String]) -> String {
    let mut bytes = Vec::new();
    for f in flags {
        bytes.extend_from_slice(f.as_bytes());
        bytes.push(0);
    }
    let hash = blake3::hash(&bytes);
    hex::encode(hash.as_bytes())
}

pub fn compute_cache_key(version: &str, host: &str, os: &str, os_ver: &str, cfg: &[String], mk: &[String], tool_ver: &str) -> String {
    let cfg_hash = flags_fingerprint(cfg);
    let mk_hash = flags_fingerprint(mk);
    format!(
        "tmux-{version}__{host}__{os}-{os_ver}__cfg{}__mk{}__tb{tool_ver}",
        &cfg_hash[..12],
        &mk_hash[..12]
    )
}
```

### Test Strategy

1. `TST-200`: cache-key stability tests.
2. `TST-201`: lock contention tests under parallel builders.
3. `TST-202`: atomic publication crash-safety simulation.

### AGENTS.md Rules

- `RULE-S20-01`: build outputs must never be published non-atomically.
- `RULE-S20-02`: builder changes require offline-mode and lock behavior tests.

---

## 21. Test Support and Fake PTY

### Design Decisions

`mux-test-support` provides hermetic integration scaffolding.
- Three-layer socket validation (`PAR-210`): non-default name, within tempdir, not equal `$TMUX` socket.
- deterministic socket path builder under temp root.
- `-f /dev/null` config isolation by default.
- readiness probe waits for socket accept/connect.
- clipboard isolation toggles (`set-clipboard off`) by default.
- both in-process and subprocess harnesses are explicit modes.

### Rust Example

```rust
use std::path::{Path, PathBuf};

pub fn ensure_not_default_socket_name(socket_name: &str) -> Result<(), &'static str> {
    if socket_name.trim().is_empty() || socket_name == "default" {
        return Err("refusing unsafe socket name");
    }
    Ok(())
}

pub fn ensure_socket_within_tempdir(socket_path: &Path, tempdir: &Path) -> Result<(), &'static str> {
    if !socket_path.starts_with(tempdir) {
        return Err("socket outside tempdir");
    }
    Ok(())
}

pub fn ensure_socket_not_tmux_env(socket_path: &Path, tmux_env: Option<&str>) -> Result<(), &'static str> {
    let Some(raw) = tmux_env else { return Ok(()); };
    let active = raw.split(',').next().unwrap_or("");
    if active == socket_path.to_string_lossy() {
        return Err("socket matches TMUX env");
    }
    Ok(())
}

pub fn tmux_socket_path(tmux_tmpdir: &Path, socket_name: &str, uid: u32) -> PathBuf {
    tmux_tmpdir.join(format!("tmux-{uid}")).join(socket_name)
}
```

### Test Strategy

1. `TST-210`: all three path-guard checks.
2. `TST-211`: fake PTY deterministic replay with exact snapshot match.
3. `TST-212`: subprocess harness readiness + shutdown escalation tests.

### AGENTS.md Rules

- `RULE-S21-01`: test harnesses must refuse unsafe socket targets.
- `RULE-S21-02`: fake PTY tests cannot rely on wall-clock timing.

---

## 22. Binding Test Frameworks

### Design Decisions

- Python: `pytest` + fixtures + snapshot assertions.
- Node: `vitest` + setup/teardown helpers.
- Shared cross-language fixture corpus to compare outputs.
- All binding tests run against both fake and real PTY when viable.

### Rust Example

```rust
#[derive(Debug, Clone)]
pub struct CrossLangCase {
    pub name: &'static str,
    pub command: &'static str,
    pub input: &'static str,
    pub expect_contains: &'static str,
}

pub const CROSS_LANG_CASES: &[CrossLangCase] = &[
    CrossLangCase {
        name: "echo-basic",
        command: "bash",
        input: "echo hello\\n",
        expect_contains: "hello",
    },
];
```

### Test Strategy

1. `TST-220`: Python fixtures enforce auto-cleanup context manager behavior.
2. `TST-221`: Node helpers enforce cleanup in `finally`.
3. `TST-222`: cross-language fixture outputs match for canonical cases.

### AGENTS.md Rules

- `RULE-S22-01`: new binding APIs require tests in that binding and one parity test.
- `RULE-S22-02`: CI matrix must include supported Python/Node versions.

---

## 23. Test Framework and Harness Design

### Design Decisions

- test taxonomy: unit, property, snapshot, parity, integration, fuzz, perf.
- parity harness (`mux-regress`) can run scenario against tmux and termforge and compare normalized outputs.
- normalization rules are explicit and versioned.

### Rust Example

```rust
#[derive(Debug, Clone)]
pub struct ParityResult {
    pub case: String,
    pub tmux_output: String,
    pub termforge_output: String,
}

pub fn normalize_output(raw: &str) -> String {
    raw.replace("\r\n", "\n").trim_end().to_string()
}

pub fn parity_equal(a: &str, b: &str) -> bool {
    normalize_output(a) == normalize_output(b)
}
```

### Test Strategy

1. `TST-230`: normalization is stable and documented.
2. `TST-231`: failing parity tests emit concise diffs with fixture names.
3. `TST-232`: parity suite runs across tmux versions matrix.

### AGENTS.md Rules

- `RULE-S23-01`: parity failures must include raw and normalized artifacts.
- `RULE-S23-02`: avoid adding normalization rules that mask semantic mismatches.

---

## 24. Performance Targets

### Design Decisions

Baselines are explicit and tied to gate budget.
- decoder throughput
- vt parser throughput
- layout resize latency
- termlet spawn/snapshot latency

Performance regressions are triaged by reproducible benchmark harness.

### Rust Example

```rust
#[derive(Debug, Clone, Copy)]
pub struct PerfTarget {
    pub name: &'static str,
    pub target_ns: u64,
    pub hard_fail_ns: u64,
}

pub const PERF_TARGETS: &[PerfTarget] = &[
    PerfTarget { name: "layout_resize_20", target_ns: 50_000, hard_fail_ns: 100_000 },
    PerfTarget { name: "termlet_snapshot_80x24", target_ns: 50_000, hard_fail_ns: 100_000 },
];
```

### Test Strategy

1. `TST-240`: criterion baselines captured and versioned.
2. `TST-241`: regression alarms when >30%.
3. `TST-242`: benchmark fixtures isolate noisy external dependencies.

### AGENTS.md Rules

- `RULE-S24-01`: performance changes require benchmark evidence.
- `RULE-S24-02`: disable debug assertions for release benchmark jobs.

---

## 25. Visual Client / TUI

### Design Decisions

- ratatui client is a view over snapshots; no direct mutable graph access.
- command input emits API events, not direct runtime mutations.
- rendering pipeline is incremental and resilient to dropped frames.

### Rust Example

```rust
#[derive(Debug, Clone)]
pub struct ViewModel {
    pub sessions: Vec<String>,
    pub selected_session: usize,
    pub panes: Vec<String>,
}

pub fn render_title(vm: &ViewModel) -> String {
    format!(
        "TermForge | sessions={} | selected={}",
        vm.sessions.len(),
        vm.selected_session
    )
}
```

### Test Strategy

1. `TST-250`: snapshot tests for view model to frame rendering.
2. `TST-251`: keyboard navigation and command dispatch tests.
3. `TST-252`: tracing instrumentation around input->event->render loop.

### AGENTS.md Rules

- `RULE-S25-01`: TUI must depend on view model crate, not core mutability internals.
- `RULE-S25-02`: every new keybinding needs integration tests.

---

## 26. AGENTS.md Rules (Global)

### Design Decisions

This section defines enforced contributor behavior for AI/human agents.
- preserve layer boundaries
- prove parity claims with anchors
- avoid undocumented “equivalent” shortcuts in compatibility paths
- require traceability IDs in substantial changes

### Rust Example

```rust
#[derive(Debug, Clone)]
pub struct Rule {
    pub id: &'static str,
    pub text: &'static str,
}

pub const GLOBAL_RULES: &[Rule] = &[
    Rule { id: "AG-001", text: "No mutation outside reducer" },
    Rule { id: "AG-002", text: "All compatibility claims require source anchors" },
    Rule { id: "AG-003", text: "Termlet remains pane-backed" },
    Rule { id: "AG-004", text: "Binding parity changes require cross-language tests" },
];
```

### Test Strategy

1. `TST-260`: architecture lint checks rule coverage in PR metadata.
2. `TST-261`: CI checks traceability IDs appear in changed docs/spec diffs.
3. `TST-262`: release checklist includes AG rule audit.

### AGENTS.md Rules

- `RULE-S26-01`: rules in this section supersede per-section heuristics on conflict.
- `RULE-S26-02`: exceptions require explicit written rationale and expiry date.

---

## 27. Risks and Mitigations

### Design Decisions

Top risks:
- protocol edge-case drift
- layout behavior mismatches under complex trees
- binding divergence from Rust semantics
- flaky PTY integration tests
- OTEL overhead under heavy workloads

Mitigation style: each risk maps to one owner, one test gate, one rollback path.

### Rust Example

```rust
#[derive(Debug, Clone)]
pub struct RiskItem {
    pub id: &'static str,
    pub description: &'static str,
    pub gate: Gate,
    pub mitigation: &'static str,
}

pub const RISKS: &[RiskItem] = &[
    RiskItem {
        id: "R-01",
        description: "protocol drift",
        gate: Gate::C1WireAttach,
        mitigation: "run parity fixtures on each protocol change",
    },
];
```

### Test Strategy

1. `TST-270`: risk register completeness check.
2. `TST-271`: every high-risk item has assigned gate and owner.
3. `TST-272`: quarterly risk review automation in release pipeline.

### AGENTS.md Rules

- `RULE-S27-01`: new high-risk features must enter risk register before merge.
- `RULE-S27-02`: unresolved high-risk items block GA releases.

---

## 28. Plan Evolution and Changelog

### Design Decisions

v11 changelog is architecture-oriented, not commit-oriented.
- records decision deltas and reason
- marks deprecated assumptions
- links each change to tests and reference anchors

### Rust Example

```rust
#[derive(Debug, Clone)]
pub struct ArchChange {
    pub version: &'static str,
    pub summary: &'static str,
    pub trace_ids: &'static [&'static str],
}

pub const EVOLUTION: &[ArchChange] = &[
    ArchChange {
        version: "v11",
        summary: "Termlet lifecycle and binding parity contracts hardened",
        trace_ids: &["INV-008", "PAR-120", "TST-320"],
    },
];
```

### Test Strategy

1. `TST-280`: changelog entries reference at least one test gate.
2. `TST-281`: deprecated contracts tracked until removed.
3. `TST-282`: CI checks monotonic version annotations.

### AGENTS.md Rules

- `RULE-S28-01`: architecture changes without changelog entry are rejected.
- `RULE-S28-02`: changelog must describe behavioral impact, not only code movement.

---

## 29. Reference Anchors

### Design Decisions

All major compatibility claims are mapped to source anchors.

tmux anchors:
- protocol v8: `tmux-protocol.h:23`
- flock lock pattern: `client.c:89`
- invalid message disconnect: `server-client.c:3472-3475`
- config load order: `server-client.c:3725-3734`
- option fallback fallthrough: `options.c:891-903`
- round-robin resize: `layout.c:448-462`
- layout checksum: `layout-custom.c:46-57`

reference crates:
- libtmux operator map + kwargs/callable/get default behavior
- vibe-tmux path guard three-layer checks
- vibe-tmux tmux-builder hash+lock+atomic publish
- vibe-tmux mux-otel dual-provider + propagator stack

### Rust Example

```rust
#[derive(Debug, Clone)]
pub struct Anchor {
    pub id: &'static str,
    pub claim: &'static str,
    pub path: &'static str,
    pub line: &'static str,
}

pub const ANCHORS: &[Anchor] = &[
    Anchor { id: "PAR-001", claim: "Protocol version 8", path: "tmux-protocol.h", line: "23" },
    Anchor { id: "PAR-002", claim: "flock lock", path: "client.c", line: "89" },
    Anchor { id: "PAR-003", claim: "disconnect on bad message", path: "server-client.c", line: "3472-3475" },
    Anchor { id: "PAR-004", claim: "layout round-robin", path: "layout.c", line: "448-462" },
    Anchor { id: "PAR-120", claim: "libtmux query operators", path: "libtmux/_internal/query_list.py", line: "298-312" },
];
```

### Test Strategy

1. `TST-290`: anchor path/line existence smoke check.
2. `TST-291`: stale anchor detection when files move.
3. `TST-292`: release audit ensures no unanchored compatibility claims.

### AGENTS.md Rules

- `RULE-S29-01`: cite at least one anchor for every tmux parity behavior.
- `RULE-S29-02`: do not keep dead anchors.

---

## 30. Appendix: Canonical Type Quick Reference

### Design Decisions

Canonical names are frozen by semver contract once public.
- IDs, snapshots, event/effect types, binding wrappers, termlet surface.

### Rust Example

```rust
pub type TermletId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaneSize {
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug, Clone)]
pub struct TermletSnapshotText(pub String);

#[derive(Debug, Clone)]
pub struct WaitMatch {
    pub start: usize,
    pub end: usize,
    pub matched_at_ms: u64,
}
```

### Test Strategy

1. `TST-300`: public API docs compile with canonical names.
2. `TST-301`: semver snapshot of public type signatures.
3. `TST-302`: deprecation annotations required before renames/removals.

### AGENTS.md Rules

- `RULE-S30-01`: type renames need migration notes and versioning plan.
- `RULE-S30-02`: appendix must mirror actual exported APIs.

---

## 31. Supplemental Test Matrix

### Design Decisions

Matrix defines mandatory rows per subsystem and environment axis.

Axes:
- OS: linux/macOS
- Rust toolchains: stable/nightly/MSRV
- tmux versions: 3.3a/3.4/3.5/3.6
- Python: 3.11-3.13
- Node: 20/22

### Rust Example

```rust
#[derive(Debug, Clone)]
pub struct MatrixRow {
    pub id: &'static str,
    pub gate: Gate,
    pub required: bool,
}

pub const MATRIX_ROWS: &[MatrixRow] = &[
    MatrixRow { id: "M-09-01", gate: Gate::C1WireAttach, required: true },
    MatrixRow { id: "M-12-01", gate: Gate::P1PyQueryKwargs, required: true },
    MatrixRow { id: "M-19-01", gate: Gate::P4OtelPropagation, required: true },
    MatrixRow { id: "M-21-01", gate: Gate::P5FakePtyDeterminism, required: true },
    MatrixRow { id: "M-32-01", gate: Gate::P6TermletSnapshotParity, required: true },
];
```

### Test Strategy

1. `TST-310`: all required rows must run in CI.
2. `TST-311`: matrix drift checker for missing jobs.
3. `TST-312`: flaky-test quarantine policy with expiration.

### AGENTS.md Rules

- `RULE-S31-01`: every new feature adds a matrix row.
- `RULE-S31-02`: required row failures block release.

---

## 32. Termlets

### Design Decisions

Termlets are SDK-first testing pods and the primary product differentiator.

#### 32.1 Architectural role

- `INV-008`: A Termlet is pane-backed. It reuses pane/grid/parser semantics; it is not a second emulator.
- Layering: `mux-termlet` depends on `mux-grid`, `mux-pty` traits, `mux-types`, optional `mux-otel`.
- Prohibited dependencies: `mux-server`, `mux-api`, `mux-orm`.
- Two backend modes:
  - `PtyMode::Real` for integration with real shell/process.
  - `PtyMode::Fake` for deterministic tests.

#### 32.2 Lifecycle state machine

Termlet lifecycle is explicit and testable:

```text
Created -> Spawning -> Running -> Stopping -> Exited
                 \-> SpawnFailed
Running --kill--> Stopping --grace timeout--> ForcedKill -> Exited
Running --process exit event---------------------------> Exited
```

State transitions are monotonic. API methods gate on state:
- `send_keys`, `resize`, `wait_for` allowed only in `Running`.
- `snapshot` allowed in `Running` and `Exited`.
- `kill` idempotent in `Stopping`/`Exited`.

#### 32.3 Data model

- `Termlet` owns:
  - parser
  - grid
  - backend handle
  - pane id
  - lifecycle state
  - config (poll interval, grace period)
  - process metadata (pid optional, exit status optional)
- Output ingestion path:
  1. backend read event
  2. parser apply to grid
  3. lifecycle updates from exit events

#### 32.4 API contracts (Rust)

Core synchronous API:
- `spawn(command, config) -> Result<Termlet>`
- `send_keys(&mut self, &str) -> Result<()>`
- `wait_for(&mut self, pattern, timeout) -> Result<WaitMatch>`
- `snapshot(&mut self) -> TermletSnapshot`
- `resize(&mut self, cols, rows) -> Result<()>`
- `kill(&mut self) -> Result<()>`
- `is_alive(&self) -> bool`

Optional APIs:
- `wait_for_async` behind `feature = "async"`.
- `spawn_with_builder` via `TermletBuilder`.
- `TermletPool` for multi-termlet orchestration.
- `SnapshotDiff` for visual regression.

#### 32.5 Wait semantics

`wait_for` details:
- pattern compiled once (`PatternMatcher`) before poll loop.
- supports plain and regex modes (`re:` prefix or enum API).
- timeout zero means single immediate check.
- returns `WaitMatch { byte_range, matched_at }`.
- distinct errors:
  - timeout elapsed
  - process exited before match
  - invalid regex

#### 32.6 Resize semantics

- `resize(cols, rows)` updates backend + grid dimensions.
- Real PTY must receive resize ioctl/SIGWINCH equivalent.
- Grid reflow follows same renderer/parsing rules as pane path.

#### 32.7 Process management contracts

- `kill()` executes graceful->forced model:
  - send SIGTERM
  - wait `grace_period`
  - escalate SIGKILL if still running
- `Drop` must do best-effort forced kill when still alive.
- `kill()` must be idempotent.
- expose exit status if known for assertions.

#### 32.8 Snapshot contracts

`TermletSnapshot` requirements:
- immutable capture of current grid state.
- `to_text()` trims trailing spaces per line and trailing blank lines.
- `to_cells()` exposes style-aware assertions.
- snapshot text parity across languages for shared fixtures.

#### 32.9 Python binding contract

Python API mirrors libtmux ergonomics and pytest friendliness.
- construction: `Termlet.spawn(...)` or context manager.
- context manager guarantees cleanup.
- exceptions map to meaningful Python types (`TimeoutError`, `RuntimeError`, etc.).
- supports fake mode and snapshot text.

#### 32.10 Node binding contract

Node API uses Promise-based methods where blocking can occur.
- `await Termlet.spawn(...)`
- `await sendKeys`, `await waitFor`, `await kill`
- cleanup in `finally`
- snapshot can be sync if no I/O needed after drain.

#### 32.11 TermletPool contract

- named collection of termlets for integration scenarios.
- batch operations: `kill_all`, `snapshot_all`, `wait_all`.
- pool drop guarantees cleanup.
- failure in one member must not leak others.

#### 32.12 SnapshotDiff contract

- fast path for identical snapshots avoids per-line allocations.
- structured line diffs include row index and before/after text.
- used by CI to print concise visual regressions.

#### 32.13 OTEL in Termlets

If enabled, emit spans:
- `termlet.spawn`
- `termlet.send_keys`
- `termlet.wait_for`
- `termlet.snapshot`
- `termlet.resize`
- `termlet.kill`

Include identifiers: `termlet.id`, `pane.id`, backend mode.

#### 32.14 Failure taxonomy

Termlet errors (stable codes):
- `TERMLET_SPAWN_ERROR`
- `TERMLET_ALREADY_EXITED`
- `TERMLET_TIMEOUT`
- `TERMLET_PATTERN_NOT_FOUND`
- `TERMLET_INVALID_REGEX`
- `TERMLET_RESIZE_ERROR`
- `TERMLET_PTY_ERROR`

#### 32.15 Concrete implementation skeleton

### Rust Example

```rust
use regex::Regex;
use std::ops::Range;
use std::time::{Duration, Instant};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtyMode {
    Real,
    Fake,
}

#[derive(Debug, Clone)]
pub struct TermletConfig {
    pub cols: u16,
    pub rows: u16,
    pub env: Vec<(String, String)>,
    pub cwd: Option<std::path::PathBuf>,
    pub backend: PtyMode,
    pub wait_poll_interval: Duration,
    pub wait_max_interval: Duration,
    pub grace_period: Duration,
}

impl Default for TermletConfig {
    fn default() -> Self {
        Self {
            cols: 80,
            rows: 24,
            env: vec![("TERM".to_string(), "xterm-256color".to_string())],
            cwd: None,
            backend: PtyMode::Real,
            wait_poll_interval: Duration::from_millis(25),
            wait_max_interval: Duration::from_millis(100),
            grace_period: Duration::from_secs(2),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TermletState {
    Created,
    Spawning,
    Running,
    Stopping,
    Exited,
}

#[derive(Debug, Clone)]
pub enum PtyEvent {
    Output(Vec<u8>),
    Exited(i32),
}

pub trait PtyBackend {
    fn spawn(
        &mut self,
        pane_id: u64,
        argv: &[String],
        cwd: Option<&std::path::Path>,
        cols: u16,
        rows: u16,
        env: &[(String, String)],
    ) -> Result<(), String>;
    fn write(&mut self, pane_id: u64, bytes: &[u8]) -> Result<(), String>;
    fn resize(&mut self, pane_id: u64, cols: u16, rows: u16) -> Result<(), String>;
    fn kill(&mut self, pane_id: u64, signal: i32) -> Result<(), String>;
    fn drain_events(&mut self, pane_id: u64) -> Vec<PtyEvent>;
}

#[derive(Debug, Clone)]
pub enum PatternMatcher {
    Plain(String),
    Regex(Regex),
}

impl PatternMatcher {
    pub fn compile(raw: &str) -> Result<Self, TermletError> {
        if let Some(rest) = raw.strip_prefix("re:") {
            let re = Regex::new(rest).map_err(|e| TermletError::InvalidRegex(e.to_string()))?;
            Ok(Self::Regex(re))
        } else {
            Ok(Self::Plain(raw.to_string()))
        }
    }

    pub fn find(&self, text: &str) -> Option<Range<usize>> {
        match self {
            Self::Plain(s) => text.find(s).map(|start| start..(start + s.len())),
            Self::Regex(re) => re.find(text).map(|m| m.start()..m.end()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct WaitMatch {
    pub byte_range: Range<usize>,
    pub matched_at: Instant,
}

#[derive(Debug, Clone)]
pub struct TermletSnapshot {
    lines: Vec<String>,
    pub cols: u16,
    pub rows: u16,
}

impl TermletSnapshot {
    pub fn to_text(&self) -> String {
        let mut lines = self
            .lines
            .iter()
            .map(|l| l.trim_end().to_string())
            .collect::<Vec<_>>();
        while lines.last().is_some_and(|l| l.is_empty()) {
            lines.pop();
        }
        lines.join("\n")
    }
}

#[derive(Debug, Error)]
pub enum TermletError {
    #[error("spawn failed: {0}")]
    Spawn(String),
    #[error("termlet already exited")]
    AlreadyExited,
    #[error("wait_for timeout pattern={pattern} timeout_ms={timeout_ms}")]
    Timeout { pattern: String, timeout_ms: u64 },
    #[error("pattern not found before process exit: {0}")]
    PatternNotFound(String),
    #[error("invalid regex: {0}")]
    InvalidRegex(String),
    #[error("pty error: {0}")]
    Pty(String),
}

pub struct Termlet {
    id: u64,
    pane_id: u64,
    state: TermletState,
    lines: Vec<String>,
    cols: u16,
    rows: u16,
    exit_status: Option<i32>,
    config: TermletConfig,
    backend: Box<dyn PtyBackend + Send>,
}

impl Termlet {
    pub fn spawn(
        id: u64,
        command: &str,
        config: TermletConfig,
        mut backend: Box<dyn PtyBackend + Send>,
    ) -> Result<Self, TermletError> {
        let pane_id = id;
        let argv = vec![command.to_string()];
        backend
            .spawn(
                pane_id,
                &argv,
                config.cwd.as_deref(),
                config.cols,
                config.rows,
                &config.env,
            )
            .map_err(TermletError::Spawn)?;

        Ok(Self {
            id,
            pane_id,
            state: TermletState::Running,
            lines: vec![String::new(); usize::from(config.rows)],
            cols: config.cols,
            rows: config.rows,
            exit_status: None,
            config,
            backend,
        })
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn is_alive(&self) -> bool {
        self.state == TermletState::Running
    }

    pub fn send_keys(&mut self, keys: &str) -> Result<(), TermletError> {
        if self.state != TermletState::Running {
            return Err(TermletError::AlreadyExited);
        }
        self.backend
            .write(self.pane_id, keys.as_bytes())
            .map_err(TermletError::Pty)?;
        self.drain_output();
        Ok(())
    }

    pub fn wait_for(&mut self, pattern: &str, timeout: Duration) -> Result<WaitMatch, TermletError> {
        let matcher = PatternMatcher::compile(pattern)?;
        let start = Instant::now();
        let mut sleep_for = self.config.wait_poll_interval;

        loop {
            self.drain_output();
            let text = self.snapshot().to_text();
            if let Some(byte_range) = matcher.find(&text) {
                return Ok(WaitMatch {
                    byte_range,
                    matched_at: Instant::now(),
                });
            }

            if self.state == TermletState::Exited {
                return Err(TermletError::PatternNotFound(pattern.to_string()));
            }

            if start.elapsed() >= timeout {
                return Err(TermletError::Timeout {
                    pattern: pattern.to_string(),
                    timeout_ms: timeout.as_millis() as u64,
                });
            }

            let remaining = timeout.saturating_sub(start.elapsed());
            std::thread::sleep(sleep_for.min(remaining));
            sleep_for = (sleep_for * 2).min(self.config.wait_max_interval);
        }
    }

    pub fn snapshot(&self) -> TermletSnapshot {
        TermletSnapshot {
            lines: self.lines.clone(),
            cols: self.cols,
            rows: self.rows,
        }
    }

    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<(), TermletError> {
        if self.state != TermletState::Running {
            return Err(TermletError::AlreadyExited);
        }
        self.backend
            .resize(self.pane_id, cols, rows)
            .map_err(TermletError::Pty)?;
        self.cols = cols;
        self.rows = rows;
        self.lines.resize(usize::from(rows), String::new());
        Ok(())
    }

    pub fn kill(&mut self) -> Result<(), TermletError> {
        if self.state == TermletState::Exited {
            return Ok(());
        }
        self.state = TermletState::Stopping;

        let _ = self.backend.kill(self.pane_id, libc::SIGTERM).map_err(TermletError::Pty);
        let deadline = Instant::now() + self.config.grace_period;
        while Instant::now() < deadline {
            self.drain_output();
            if self.state == TermletState::Exited {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(10));
        }

        let _ = self.backend.kill(self.pane_id, libc::SIGKILL).map_err(TermletError::Pty);
        self.state = TermletState::Exited;
        Ok(())
    }

    fn drain_output(&mut self) {
        for event in self.backend.drain_events(self.pane_id) {
            match event {
                PtyEvent::Output(data) => {
                    let chunk = String::from_utf8_lossy(&data);
                    if let Some(last) = self.lines.last_mut() {
                        last.push_str(&chunk);
                    }
                }
                PtyEvent::Exited(code) => {
                    self.exit_status = Some(code);
                    self.state = TermletState::Exited;
                }
            }
        }
    }
}

impl Drop for Termlet {
    fn drop(&mut self) {
        if self.state != TermletState::Exited {
            let _ = self.backend.kill(self.pane_id, libc::SIGKILL);
            self.state = TermletState::Exited;
        }
    }
}

pub struct TermletBuilder {
    command: String,
    cfg: TermletConfig,
}

impl TermletBuilder {
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            cfg: TermletConfig::default(),
        }
    }

    pub fn size(mut self, cols: u16, rows: u16) -> Self {
        self.cfg.cols = cols;
        self.cfg.rows = rows;
        self
    }

    pub fn cwd(mut self, cwd: impl Into<std::path::PathBuf>) -> Self {
        self.cfg.cwd = Some(cwd.into());
        self
    }

    pub fn env(mut self, key: impl Into<String>, val: impl Into<String>) -> Self {
        self.cfg.env.push((key.into(), val.into()));
        self
    }

    pub fn fake(mut self) -> Self {
        self.cfg.backend = PtyMode::Fake;
        self
    }

    pub fn grace_period(mut self, dur: Duration) -> Self {
        self.cfg.grace_period = dur;
        self
    }

    pub fn spawn(self, id: u64, backend: Box<dyn PtyBackend + Send>) -> Result<Termlet, TermletError> {
        Termlet::spawn(id, &self.command, self.cfg, backend)
    }
}

#[derive(Default)]
pub struct TermletPool {
    inner: std::collections::HashMap<String, Termlet>,
}

impl TermletPool {
    pub fn insert(&mut self, name: impl Into<String>, termlet: Termlet) {
        self.inner.insert(name.into(), termlet);
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut Termlet> {
        self.inner.get_mut(name)
    }

    pub fn kill_all(&mut self) {
        for t in self.inner.values_mut() {
            let _ = t.kill();
        }
    }
}

impl Drop for TermletPool {
    fn drop(&mut self) {
        self.kill_all();
    }
}

#[derive(Debug, Clone)]
pub struct LineDiff {
    pub row: usize,
    pub before: String,
    pub after: String,
}

#[derive(Debug, Clone)]
pub struct SnapshotDiff {
    pub changed_lines: Vec<LineDiff>,
    pub identical: bool,
}

impl SnapshotDiff {
    pub fn compare(before: &TermletSnapshot, after: &TermletSnapshot) -> Self {
        let bt = before.to_text();
        let at = after.to_text();
        if bt == at && before.cols == after.cols && before.rows == after.rows {
            return Self {
                changed_lines: Vec::new(),
                identical: true,
            };
        }

        let bl = bt.lines().collect::<Vec<_>>();
        let al = at.lines().collect::<Vec<_>>();
        let max = bl.len().max(al.len());
        let mut changed = Vec::new();
        for i in 0..max {
            let b = bl.get(i).copied().unwrap_or("");
            let a = al.get(i).copied().unwrap_or("");
            if b != a {
                changed.push(LineDiff {
                    row: i,
                    before: b.to_string(),
                    after: a.to_string(),
                });
            }
        }
        Self {
            changed_lines: changed,
            identical: false,
        }
    }
}
```

### Test Strategy

Mandatory Termlet tests:

1. `TST-320`: spawn in real and fake modes.
2. `TST-321`: send_keys + wait_for plain string.
3. `TST-322`: wait_for regex path + invalid regex error.
4. `TST-323`: timeout vs process-exited distinction.
5. `TST-324`: zero-timeout immediate check semantics.
6. `TST-325`: snapshot trimming contract.
7. `TST-326`: resize updates size and backend call.
8. `TST-327`: kill idempotency.
9. `TST-328`: drop cleanup (forced kill).
10. `TST-329`: fake backend deterministic replay.
11. `TST-330`: cross-language snapshot parity fixtures.
12. `TST-331`: Python context manager no-leak.
13. `TST-332`: Node finally-cleanup no-leak.
14. `TST-333`: TermletBuilder parity with direct config.
15. `TST-334`: TermletPool `kill_all` and `Drop` behavior.
16. `TST-335`: SnapshotDiff identical fast path.
17. `TST-336`: SnapshotDiff changed-lines accuracy.
18. `TST-337`: OTEL spans emitted when feature enabled.
19. `TST-338`: performance thresholds fake/real spawn + snapshot.
20. `TST-339`: background task lifecycle (spawn process, detach/kill, observe exit).

### AGENTS.md Rules

- `RULE-S32-01`: Termlet must remain pane-backed; no duplicate terminal core.
- `RULE-S32-02`: `kill()` is idempotent and `Drop` performs best-effort forced cleanup.
- `RULE-S32-03`: snapshot text normalization is stable and tested.
- `RULE-S32-04`: fake mode tests must be deterministic and timing-independent.
- `RULE-S32-05`: wait patterns must compile once per `wait_for` call.
- `RULE-S32-06`: distinct timeout vs exited-before-match errors are required.
- `RULE-S32-07`: Python and Node Termlet APIs must maintain semantic parity with Rust core.
- `RULE-S32-08`: all new Termlet features require cross-language fixture coverage.
- `RULE-S32-09`: `mux-termlet` may not depend on server/runtime orchestration crates.
- `RULE-S32-10`: TermletPool and SnapshotDiff are first-class tested APIs, not examples-only helpers.

---

## v11 Consistency Checklist

- [x] 32 sections present.
- [x] Every section includes: Design Decisions, Rust Example, Test Strategy, AGENTS.md Rules.
- [x] Claims mapped to reference anchors where compatibility-sensitive.
- [x] Section 32 strengthened with lifecycle state machine, contracts, errors, concrete Rust skeleton, and binding parity details.
- [x] Rust examples written for Rust 2024-compatible style.

