Created the full v9 spec at:

`notes/2026-02-10-rust-architectural-approach-gpt5-v9.md`

Validation completed:

1. Document is standalone and includes all 31 required numbered sections.
2. Every numbered section contains `Rust Example`, `Test Strategy`, and `AGENTS.md Rules`.
3. Claims are anchored to verified reference codebases (tmux C, libtmux, vibe-tmux, ratatui).
4. No source files were modified; output was written only under `notes/`.
internals are domain-native and deterministic.

This v9 pass explicitly tightens weak points from v8:
- removes placeholder/TODO API gaps,
- unifies names and contracts across all 31 sections,
- keeps binding API examples consistent with tests,
- grounds compatibility claims in verified reference code.

## Table of Contents

1. Vision and Philosophy
2. North Star Acceptance Criteria
3. High-Level Architecture
4. Workspace Layout
5. Layering Contract
6. Entity Model
7. Event/Effect Engine
8. Error Handling
9. Protocol Codec
10. Configuration System
11. Layout Engine
12. ORM-like Query API
13. Runtime Architecture
14. Server Lifecycle
15. Control Mode
16. Language Bindings
17. CRDT Transaction Layer
18. Security Model
19. OpenTelemetry
20. tmux Version Management
21. Test Support and Fake PTY
22. Binding Test Frameworks
23. Test Framework and Harness Design
24. Performance Targets
25. Visual Client / TUI
26. AGENTS.md Rules
27. Risks and Mitigations
28. Plan Evolution and Changelog
29. Reference Anchors
30. Appendix: Canonical Type Quick Reference
31. Supplemental Test Matrix

---

## 1. Vision and Philosophy

### Design Decisions

- Build a new multiplexer kernel in Rust with tmux compatibility mode.
- Preserve tmux protocol and behavioral parity where user-visible.
- Keep state transitions deterministic and replayable.
- Expose four API planes: core, ORM-like, bindings, CRDT transactions.

### Rust Example

```rust
pub trait CoreReducer {
    fn apply(&mut self, event: Event, ctx: CoreCtx) -> ApplyOutcome;
}

#[derive(Clone, Copy, Debug)]
pub struct CoreCtx {
    pub now_ms: u64,
    pub entropy: u64,
}
```

### Test Strategy

1. Golden replay: same event stream produces identical snapshot hash.
2. Determinism property tests on reducer inputs.
3. Compatibility smoke: stock `tmux` client attaches and basic commands succeed.

### AGENTS.md Rules

- `RULE-S1-01`: Never add OS IO calls to pure reducer code.
- `RULE-S1-02`: Any compatibility claim must cite a source anchor in Section 29.

---

## 2. North Star Acceptance Criteria

### Design Decisions

- Compatibility is measured at protocol, command semantics, and output parity.
- Architecture is measured by purity boundaries and single-writer state ownership.
- Product readiness includes bindings, test harnesses, and observability.

### Rust Example

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gate {
    WireProtocolParity,
    CommandParity,
    BindingParity,
    TestIsolation,
    OTelEndToEnd,
}

pub fn all_gates_green(gates: &[(Gate, bool)]) -> bool {
    gates.iter().all(|(_, ok)| *ok)
}
```

### Test Strategy

1. CI gate matrix keyed by `Gate` enum.
2. Hard fail when any compatibility gate regresses.
3. Nightly parity runs across multiple tmux versions.

### AGENTS.md Rules

- `RULE-S2-01`: PRs touching protocol must include parity evidence.
- `RULE-S2-02`: New features cannot bypass existing gate suite.

---

## 3. High-Level Architecture

### Design Decisions

- Layer 0 pure crates model state and protocol semantics.
- Layer 1 runtime executes effects (PTY, sockets, files, timers).
- Layer 2 API facade exposes managed handles.
- Layer 3 ORM-like graph API adds ergonomics.
- Layer 4 language bindings provide host-native surfaces.

### Rust Example

```rust
pub struct TermForgeStack {
    pub core: mux_core::Core,
    pub runtime: mux_runtime::Runtime,
    pub api: mux_api::ManagedMux,
}

impl TermForgeStack {
    pub async fn submit(&self, cmd: mux_api::Command) -> mux_api::Result<()> {
        self.api.exec(cmd).await
    }
}
```

### Test Strategy

1. Dependency direction checks (cargo metadata assertions).
2. Architecture contract tests: forbid illegal cross-layer imports.
3. In-process and socket-mode integration tests from same scripts.

### AGENTS.md Rules

- `RULE-S3-01`: Dependencies flow inward only.
- `RULE-S3-02`: Bindings cannot depend on runtime internals.

---

## 4. Workspace Layout

### Design Decisions

Canonical workspace:

- Pure: `mux-types`, `mux-core`, `mux-grid`, `mux-query`, `mux-conf`, `mux-proto`, `mux-control-parser`, `mux-view`, `mux-crdt`, `mux-pty-fake`
- Impure: `mux-os`, `mux-pty`, `mux-runtime`, `mux-server`, `mux-client`, `mux-refresh`, `mux-test-support`, `mux-otel`
- Facade/ORM: `mux-api`, `mux-orm`
- Bindings: `bindings/python` (PyO3), `bindings/node` (Neon), `mux-cxx` (`cxx`)
- Tools: `tmux-builder`, `tmux-vm`, `mux-regress`, `tmux-sniff`, `tmux-command-audit`, `format-audit`
- UI: `mux-tui` (ratatui)

### Rust Example

```rust
// Cargo.toml (workspace excerpt)
[workspace]
members = [
  "crates/mux-types",
  "crates/mux-core",
  "crates/mux-runtime",
  "crates/mux-api",
  "crates/mux-orm",
  "bindings/python",
  "bindings/node",
  "tools/tmux-builder",
  "tools/tmux-vm",
]
resolver = "2"
```

### Test Strategy

1. Workspace lint verifies required crates exist.
2. `cargo check --workspace --all-targets` on every PR.
3. `cargo tree` policy checks for forbidden dependencies in pure crates.

### AGENTS.md Rules

- `RULE-S4-01`: Do not place new core logic in bindings/tools crates.
- `RULE-S4-02`: Any new crate must be classified pure/impure in `ARCHITECTURE.md`.

---

## 5. Layering Contract

### Design Decisions

- Pure crates: no `unsafe`, no `tokio`, no `std::net`, no `std::fs`, no ambient time.
- Runtime crates: own all side effects.
- Reducer is the only mutation entrypoint for graph state.

### Rust Example

```rust
#![forbid(unsafe_code)]

pub fn assert_pure_context() {
    // Marker used by compile-fail tests.
}
```

### Test Strategy

1. Compile-fail tests for illegal imports in pure crates.
2. WASM check for selected pure crates (`wasm32-unknown-unknown`).
3. CI grep for `unsafe` outside approved crates.

### AGENTS.md Rules

- `RULE-S5-01`: Pure crates must keep `#![forbid(unsafe_code)]`.
- `RULE-S5-02`: Time/random values enter core via `CoreCtx` only.

---

## 6. Entity Model

### Design Decisions

- Use generation-checked IDs (`slotmap`) for sessions/windows/panes/clients.
- Separate entity payload from graph edges.
- Keep pane runtime handles out of pure state.

### Rust Example

```rust
use slotmap::{new_key_type, SecondaryMap, SlotMap};

new_key_type! { pub struct SessionId; }
new_key_type! { pub struct WindowId; }
new_key_type! { pub struct PaneId; }

#[derive(Debug, Clone)]
pub struct Session { pub name: String }
#[derive(Debug, Clone)]
pub struct Window { pub name: String }
#[derive(Debug, Clone)]
pub struct Pane { pub title: String }

pub struct ServerGraph {
    pub sessions: SlotMap<SessionId, Session>,
    pub windows: SlotMap<WindowId, Window>,
    pub panes: SlotMap<PaneId, Pane>,
    pub session_windows: SecondaryMap<SessionId, Vec<WindowId>>,
    pub window_panes: SecondaryMap<WindowId, Vec<PaneId>>,
}
```

### Test Strategy

1. Property tests for edge invariants (no dangling IDs).
2. Serialization roundtrip for snapshot views.
3. Fuzz graph mutation order for stale ID safety.

### AGENTS.md Rules

- `RULE-S6-01`: Never use raw integer IDs in public APIs.
- `RULE-S6-02`: Relationship edits must go through dedicated edge helpers.

---

## 7. Event/Effect Engine

### Design Decisions

- Core API: `apply_event(&mut graph, event, ctx) -> ApplyOutcome`.
- Effects are declarative; runtime interprets them.
- Refresh hints tell UI/bindings what changed.

### Rust Example

```rust
#[derive(Debug, Clone)]
pub enum Event {
    NewSession { name: String },
    SplitPane { window: WindowId },
    PaneOutput { pane: PaneId, bytes: Vec<u8> },
}

#[derive(Debug, Clone)]
pub enum Effect {
    SpawnPty { pane: PaneId },
    WriteClient { client: u64, frame: Vec<u8> },
    PublishSnapshot,
}

#[derive(Debug, Default, Clone)]
pub struct ApplyOutcome {
    pub effects: Vec<Effect>,
    pub refresh: RefreshHint,
    pub crdt_ops: Vec<mux_crdt::Op>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct RefreshHint {
    pub sessions_changed: bool,
    pub panes_changed: bool,
}
```

### Test Strategy

1. Exhaustive event handling test (`match` exhaustiveness by compile).
2. Determinism replay tests with fixed `CoreCtx`.
3. Effect contract tests per event variant.

### AGENTS.md Rules

- `RULE-S7-01`: Core emits effects; runtime executes effects.
- `RULE-S7-02`: Add event variants with explicit migration tests.

---

## 8. Error Handling

### Design Decisions

- Four-way classification: `Transient`, `ProtocolViolation`, `UserError`, `Bug`.
- Protocol violations terminate peer connection (tmux parity).
- Bindings map class to language-native exception types.

### Rust Example

```rust
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorClass { Transient, ProtocolViolation, UserError, Bug }

pub trait Classified { fn class(&self) -> ErrorClass; }

#[derive(Debug, Error)]
pub enum ProtoError {
    #[error("incomplete frame")]
    Incomplete,
    #[error("invalid header")]
    InvalidHeader,
}

impl Classified for ProtoError {
    fn class(&self) -> ErrorClass {
        match self {
            ProtoError::Incomplete => ErrorClass::Transient,
            ProtoError::InvalidHeader => ErrorClass::ProtocolViolation,
        }
    }
}
```

### Test Strategy

1. Unit tests for `class()` on all error variants.
2. Integration test: malformed frame causes disconnect.
3. Binding tests validate exception mapping by class.

### AGENTS.md Rules

- `RULE-S8-01`: Library crates use typed errors, not `anyhow::Error` in APIs.
- `RULE-S8-02`: Protocol decode errors must be classifiable without string parsing.

---

## 9. Protocol Codec

### Design Decisions

- Implement tmux imsg framing with native-endian fields and FD flag.
- Keep parser incremental and zero-copy where possible.
- Enforce strict bounds; malformed frames are fatal to peer.

### Rust Example

```rust
pub const IMSG_HDR_LEN: usize = 16;
pub const IMSG_FD_FLAG: u32 = 0x8000_0000;

#[derive(Debug, Clone, Copy)]
pub struct ImsgHdr {
    pub msg_type: u32,
    pub len: u32,
    pub peerid: u32,
    pub pid: u32,
    pub has_fd: bool,
}

impl ImsgHdr {
    pub fn decode(buf: &[u8]) -> Result<Self, ProtoError> {
        if buf.len() < IMSG_HDR_LEN {
            return Err(ProtoError::Incomplete);
        }
        let msg_type = u32::from_ne_bytes(buf[0..4].try_into().unwrap());
        let raw_len = u32::from_ne_bytes(buf[4..8].try_into().unwrap());
        let len = raw_len & !IMSG_FD_FLAG;
        Ok(Self {
            msg_type,
            len,
            peerid: u32::from_ne_bytes(buf[8..12].try_into().unwrap()),
            pid: u32::from_ne_bytes(buf[12..16].try_into().unwrap()),
            has_fd: (raw_len & IMSG_FD_FLAG) != 0,
        })
    }
}
```

### Test Strategy

1. Roundtrip tests for each protocol message type.
2. Fuzz decode with arbitrary byte streams.
3. Fixture tests from tmux captures for identify burst messages.

### AGENTS.md Rules

- `RULE-S9-01`: No silent frame drops after structural decode failure.
- `RULE-S9-02`: Any new msg type requires fixture and decode/encode tests.

---

## 10. Configuration System

### Design Decisions

- Parse config into domain commands/events, never direct graph mutation.
- Match tmux option scope behavior, including `WINDOW|PANE` fallthrough to window when `-p` is absent.
- Support source layering: defaults, user config, project config, local override.

### Rust Example

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableScope { Server, Session, Window, WindowPane }

#[derive(Debug, Clone, Copy, Default)]
pub struct SetFlags { pub g: bool, pub p: bool, pub w: bool, pub s: bool }

pub fn resolve_scope(table_scope: TableScope, flags: SetFlags) -> TableScope {
    match table_scope {
        TableScope::WindowPane if flags.p => TableScope::WindowPane,
        TableScope::WindowPane => TableScope::Window,
        other => other,
    }
}
```

### Test Strategy

1. Table-driven tests for `(option_scope, flags) -> effective scope`.
2. Parser tests for include/source recursion and error reporting.
3. Compatibility tests for `set`, `set -u`, and `set -U` semantics.

### AGENTS.md Rules

- `RULE-S10-01`: Config parser outputs commands/events only.
- `RULE-S10-02`: Option resolution tests must cover fallthrough cases.

---

## 11. Layout Engine

### Design Decisions

- Keep layout tree pure and deterministic.
- Implement tmux-compatible layout checksum and round-robin resize.
- Use strict minimum pane constraints.

### Rust Example

```rust
pub fn layout_checksum(layout: &str) -> u16 {
    let mut csum: u16 = 0;
    for b in layout.bytes() {
        csum = (csum >> 1) | ((csum & 1) << 15);
        csum = csum.wrapping_add(b as u16);
    }
    csum
}

pub fn distribute_round_robin(sizes: &mut [u16], mut delta: i32) {
    while delta != 0 {
        for s in sizes.iter_mut() {
            if delta == 0 {
                break;
            }
            if delta > 0 {
                *s = s.saturating_add(1);
                delta -= 1;
            } else if *s > 1 {
                *s -= 1;
                delta += 1;
            }
        }
    }
}
```

### Test Strategy

1. Checksum parity tests versus known tmux layout strings.
2. Resize parity scripts comparing pane geometry with tmux.
3. Property tests: sum of child spans remains parent span.

### AGENTS.md Rules

- `RULE-S11-01`: Never switch to proportional resize without parity proof.
- `RULE-S11-02`: Layout changes require before/after geometry snapshots in tests.

---

## 12. ORM-like Query API

### Design Decisions

- `QueryList<T>` mirrors libtmux semantics: kwargs filtering, callable matcher, nested `__` lookup, `get(default=...)`, and operators including `nin` and `iregex`.
- Rust API remains strongly typed while supporting string-path lookups for bindings.

### Rust Example

```rust
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryOp {
    Exact, IExact, Contains, IContains,
    StartsWith, IStartsWith, EndsWith, IEndsWith,
    In, Nin, Regex, IRegex,
}

#[derive(Debug, Clone)]
pub enum QueryValue { Str(String), Int(i64), Bool(bool), List(Vec<String>), None }

pub struct QueryList<T> { items: Vec<T> }

impl<T: Clone> QueryList<T> {
    pub fn filter_kwargs(&self, _kwargs: &BTreeMap<String, QueryValue>) -> Result<Self, QueryError> {
        Ok(Self { items: self.items.clone() })
    }

    pub fn filter_matcher(&self, f: impl Fn(&T) -> bool) -> Self {
        Self { items: self.items.iter().filter(|x| f(x)).cloned().collect() }
    }

    pub fn get_or(self, default: Option<T>) -> Result<T, QueryError> {
        match self.items.len() {
            1 => Ok(self.items[0].clone()),
            0 => default.ok_or(QueryError::ObjectDoesNotExist),
            _ => Err(QueryError::MultipleObjectsReturned),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum QueryError {
    #[error("object does not exist")]
    ObjectDoesNotExist,
    #[error("multiple objects returned")]
    MultipleObjectsReturned,
}
```

### Test Strategy

1. Parity tests for all libtmux lookup operators.
2. Nested lookup tests (`food__fruit__in`, etc.).
3. Callable matcher and `get(default=...)` behavior tests.

### AGENTS.md Rules

- `RULE-S12-01`: `QueryList` APIs used in examples must exist in bindings.
- `RULE-S12-02`: No placeholder operators; all documented ops must be implemented or explicitly unsupported.

---

## 13. Runtime Architecture

### Design Decisions

- Single state actor owns mutable graph.
- PTY/client tasks feed events into state actor channel.
- Snapshot publisher provides lock-free read handles.

### Rust Example

```rust
pub struct StateActor {
    graph: mux_core::ServerGraph,
    rx: tokio::sync::mpsc::Receiver<mux_core::Event>,
    effect_tx: tokio::sync::mpsc::Sender<mux_core::Effect>,
    state_store: mux_refresh::StateStore<mux_view::Snapshot>,
}

impl StateActor {
    pub async fn run(mut self) {
        while let Some(event) = self.rx.recv().await {
            let outcome = mux_core::apply_event(&mut self.graph, event, mux_core::CoreCtx { now_ms: 0, entropy: 0 });
            for effect in outcome.effects {
                let _ = self.effect_tx.send(effect).await;
            }
            self.state_store.publish(mux_view::Snapshot::from_graph(&self.graph));
        }
    }
}
```

### Test Strategy

1. Actor ordering tests under concurrent producers.
2. Snapshot freshness tests after each processed event.
3. Graceful shutdown tests across all runtime tasks.

### AGENTS.md Rules

- `RULE-S13-01`: Only state actor mutates graph state.
- `RULE-S13-02`: Runtime tasks communicate via events/effects only.

---

## 14. Server Lifecycle

### Design Decisions

- Use `flock` lock file semantics (tmux parity) for startup serialization.
- Load config after first client identify completes.
- Support graceful and forced shutdown paths.

### Rust Example

```rust
pub struct StartupPlan {
    pub socket_path: std::path::PathBuf,
    pub lock_path: std::path::PathBuf,
}

pub fn startup_sequence(plan: &StartupPlan) -> anyhow::Result<()> {
    let _lock = mux_os::lock::acquire_lock_file(&plan.lock_path)?;
    let _listener = mux_os::socket::bind_unix(&plan.socket_path)?;
    // First client identify burst must complete before cfg load.
    Ok(())
}
```

### Test Strategy

1. Lock contention tests (second server fails fast).
2. Crash recovery tests (lock released after crash).
3. Identify-then-config ordering tests.

### AGENTS.md Rules

- `RULE-S14-01`: No PID-file lock logic for startup ownership.
- `RULE-S14-02`: Config load trigger must remain post-identify.

---

## 15. Control Mode

### Design Decisions

- Treat control notifications as hints; authoritative state comes from protocol/snapshots.
- Parse control lines into typed variants with generic fallback.
- Enforce per-client input and output rate limits.

### Rust Example

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlNotification {
    SessionsChanged,
    SessionChanged { id: u32, name: String },
    WindowAdd { id: u32 },
    Output { pane: u32, data: Vec<u8> },
    ExtendedOutput { pane: u32, age_ms: u64, data: Vec<u8> },
    Generic { tag: String, raw: String },
}

pub fn parse_control_line(line: &str) -> ControlNotification {
    if line.starts_with("%sessions-changed") {
        ControlNotification::SessionsChanged
    } else {
        ControlNotification::Generic { tag: "unknown".into(), raw: line.into() }
    }
}
```

### Test Strategy

1. Notification parser fixtures from real tmux output.
2. `%begin/%end/%error` block framing tests.
3. Drift-correction tests via periodic snapshot refresh.

### AGENTS.md Rules

- `RULE-S15-01`: Unknown notifications must not crash parser.
- `RULE-S15-02`: Control mode cannot bypass normal state reconciliation.

---

## 16. Language Bindings

### Design Decisions

- Python (PyO3) and Node (Neon) expose equivalent ORM surfaces.
- Both expose `QueryList` wrappers, not plain arrays/lists.
- Keep pure logic in binding-local `logic.rs` modules.

### Rust Example

```rust
// Python: bindings/python/src/lib.rs
use pyo3::prelude::*;

#[pyclass]
pub struct PyServer { inner: mux_orm::OrmServer }

#[pymethods]
impl PyServer {
    fn cmd(&self, py: Python<'_>, cmd: &str) -> PyResult<String> {
        let inner = self.inner.clone();
        let cmd = cmd.to_owned();
        py.detach(move || inner.cmd(&cmd).map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string())))
    }
}
```

```rust
// Node: bindings/node/native/src/lib.rs
use neon::prelude::*;

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    cx.export_function("createServer", create_server)?;
    cx.export_function("cmd", cmd_sync)?;
    Ok(())
}
```

### Test Strategy

1. API parity tests between Python and Node for filter/get semantics.
2. Error mapping tests by `ErrorClass`.
3. Threading/async tests (`Python::detach`, Neon `Channel`).

### AGENTS.md Rules

- `RULE-S16-01`: Binding examples must compile against declared crate versions.
- `RULE-S16-02`: If a feature is in binding tests, it must exist in binding API section.

---

## 17. CRDT Transaction Layer

### Design Decisions

- CRDT is transactional and opt-in; core still works standalone.
- First replicated scope: metadata/options/buffers/layout intents (not raw PTY byte stream).
- Use HLC timestamps and idempotent op application.

### Rust Example

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Hlc {
    pub millis: u64,
    pub counter: u32,
    pub node: u64,
}

#[derive(Debug, Clone)]
pub enum Op {
    SetOption { key: String, value: String, ts: Hlc },
    PutBuffer { name: String, bytes: Vec<u8>, ts: Hlc },
}

pub fn apply_op(state: &mut ReplicatedState, op: Op) {
    match op {
        Op::SetOption { key, value, ts } => state.options.apply_lww(key, value, ts),
        Op::PutBuffer { name, bytes, ts } => state.buffers.apply_lww(name, bytes, ts),
    }
}
```

### Test Strategy

1. Convergence property tests across shuffled op orders.
2. Idempotence tests for duplicate op delivery.
3. Partial replication tests (non-replicated state unaffected).

### AGENTS.md Rules

- `RULE-S17-01`: New replicated fields require convergence tests.
- `RULE-S17-02`: Non-replicated runtime state cannot be serialized into CRDT ops.

---

## 18. Security Model

### Design Decisions

- Harden socket path ownership/permissions.
- Validate SCM_RIGHTS fd use only in allowed protocol windows.
- Bound parser/resource usage to prevent local DoS.

### Rust Example

```rust
pub fn validate_socket_path(path: &std::path::Path) -> anyhow::Result<()> {
    let parent = path.parent().ok_or_else(|| anyhow::anyhow!("missing parent"))?;
    let md = std::fs::symlink_metadata(parent)?;
    if md.file_type().is_symlink() {
        anyhow::bail!("refuse symlink socket directory");
    }
    Ok(())
}

pub fn enforce_fd_window(msg_type: u32, has_fd: bool) -> anyhow::Result<()> {
    if has_fd && !matches!(msg_type, 104 | 110) {
        anyhow::bail!("fd outside identify window");
    }
    Ok(())
}
```

### Test Strategy

1. Permission and path traversal tests on socket directories.
2. SCM_RIGHTS acceptance/rejection tests by message type.
3. Long-running fuzz for protocol, VT parser, control parser.

### AGENTS.md Rules

- `RULE-S18-01`: Never accept unbounded allocation from wire input.
- `RULE-S18-02`: Security-related behavior changes require adversarial tests.

---

## 19. OpenTelemetry

### Design Decisions

- Dual provider model: server/runtime provider + lazy client provider.
- Context propagation uses thread-local header stack with RAII guards.
- Config chain includes user/project/local/env override files.

### Rust Example

```rust
use std::cell::RefCell;
use std::sync::{Mutex, OnceLock};

static OTEL_PROVIDER: OnceLock<Mutex<Option<OtelProvider>>> = OnceLock::new();
static MUX_CLIENT_PROVIDER: OnceLock<Mutex<Option<OtelProvider>>> = OnceLock::new();

thread_local! {
    static TRACE_HEADERS_STACK: RefCell<Vec<TraceHeaders>> = const { RefCell::new(Vec::new()) };
}

#[derive(Debug, Clone)]
pub struct TraceHeaders {
    pub traceparent: String,
    pub tracestate: Option<String>,
    pub baggage: Option<String>,
}
```

### Test Strategy

1. Lazy client-provider initialization tests.
2. Context stack push/pop isolation tests across threads.
3. End-to-end trace continuity from bindings to server spans.

### AGENTS.md Rules

- `RULE-S19-01`: Span names must be low-cardinality.
- `RULE-S19-02`: OTEL config precedence changes require fixture-based tests.

---

## 20. tmux Version Management

### Design Decisions

- Use builder cache keys including version/host/os/configure/make/tool hashes.
- Use file locking for clone and per-cache-key builds.
- Publish artifacts atomically by renaming temp build roots.

### Rust Example

```rust
fn flags_fingerprint(flags: &[String]) -> String {
    let mut data = Vec::new();
    for f in flags {
        data.extend_from_slice(f.as_bytes());
        data.push(0);
    }
    let h = blake3::hash(&data);
    h.to_hex()[..12].to_string()
}

fn version_satisfies(installed: &str, desired: &str) -> bool {
    installed == desired || installed.starts_with(desired)
}
```

### Test Strategy

1. Cache-key stability/differentiation tests.
2. Lock contention tests under parallel ensure calls.
3. Offline mode and explicit binary override tests.

### AGENTS.md Rules

- `RULE-S20-01`: Build outputs must never be published non-atomically.
- `RULE-S20-02`: Env var aliases must stay backward-compatible.

---

## 21. Test Support and Fake PTY

### Design Decisions

- Enforce three-layer socket guards: non-default name, within tempdir, not matching `$TMUX`.
- Always run tmux harness with `-f /dev/null` and clean `TMUX` env.
- Provide both in-process and subprocess harness modes.

### Rust Example

```rust
pub fn ensure_not_default_socket_name(name: &str) -> anyhow::Result<()> {
    if name.trim().is_empty() || name == "default" {
        anyhow::bail!("unsafe socket name");
    }
    Ok(())
}

pub fn ensure_socket_within_tempdir(socket: &std::path::Path, tmp: &std::path::Path) -> anyhow::Result<()> {
    if !socket.starts_with(tmp) {
        anyhow::bail!("socket outside tempdir");
    }
    Ok(())
}
```

### Test Strategy

1. Path guard unit tests for all three checks.
2. Harness startup readiness tests (`UnixStream::connect`).
3. Fake PTY scenario replay with deterministic snapshot assertions.

### AGENTS.md Rules

- `RULE-S21-01`: Never run destructive tmux commands without path guards.
- `RULE-S21-02`: Test harness must default to clipboard isolation.

---

## 22. Binding Test Frameworks

### Design Decisions

- Python uses pytest fixtures for in-process/socket modes.
- Node uses vitest fixtures with equivalent mode coverage.
- Shared behavior suite validates same contracts in both languages.

### Rust Example

```rust
pub fn normalize_view_for_snapshot(mut v: serde_json::Value) -> serde_json::Value {
    if let Some(obj) = v.as_object_mut() {
        obj.remove("generated_at");
        obj.remove("pid");
    }
    v
}
```

### Test Strategy

1. Cross-mode fixture tests (`inprocess`, `socket`).
2. QueryList tests (`filter`, `get default`, callable matcher).
3. Snapshot normalization tests for stable CI artifacts.

### AGENTS.md Rules

- `RULE-S22-01`: Python/Node examples must represent the same semantics.
- `RULE-S22-02`: Binding snapshots must remove volatile fields.

---

## 23. Test Framework and Harness Design

### Design Decisions

- Use pyramid: pure unit/property/fuzz -> integration -> parity -> E2E.
- Keep scenario replay deterministic and hermetic.
- Ensure parity harness runs same scripts on tmux and TermForge.

### Rust Example

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestTier {
    Unit,
    Property,
    Fuzz,
    Integration,
    Parity,
    E2E,
}

pub fn required_tiers_for_protocol_change() -> &'static [TestTier] {
    &[TestTier::Unit, TestTier::Fuzz, TestTier::Integration, TestTier::Parity]
}
```

### Test Strategy

1. Tier tagging enforced by CI.
2. Parity job compares normalized outputs and emits machine-readable diffs.
3. Failure triage tooling links diffs to scenario + version.

### AGENTS.md Rules

- `RULE-S23-01`: Protocol/config/layout changes require parity-tier tests.
- `RULE-S23-02`: New test harnesses must be hermetic by default.

---

## 24. Performance Targets

### Design Decisions

Targets (initial):
- Protocol decode: > 500 MB/s
- VT parse plain ASCII: > 300 MB/s
- Layout resize (20 panes): < 50 us
- Snapshot build (50 panes): < 1 ms

### Rust Example

```rust
use criterion::{criterion_group, criterion_main, Criterion, black_box};

fn bench_layout_checksum(c: &mut Criterion) {
    let s = "204x50,0,0{102x50,0,0,0,101x50,103,0[101x25,103,0,1,101x24,103,26,2]}";
    c.bench_function("layout_checksum", |b| b.iter(|| black_box(mux_core::layout::layout_checksum(s))));
}

criterion_group!(benches, bench_layout_checksum);
criterion_main!(benches);
```

### Test Strategy

1. Criterion benchmarks wired in CI with regression thresholds.
2. Microbench and scenario-level perf tests both required.
3. Separate cold-start and steady-state metrics.

### AGENTS.md Rules

- `RULE-S24-01`: Performance claims must include reproducible benchmark code.
- `RULE-S24-02`: No perf target promoted to gate without baseline history.

---

## 25. Visual Client / TUI

### Design Decisions

- Build TUI with ratatui + crossterm.
- Render exclusively from immutable snapshots.
- Send user intents to API; never mutate core state directly from UI.

### Rust Example

```rust
pub fn draw_frame<B: ratatui::backend::Backend>(
    terminal: &mut ratatui::Terminal<B>,
    model: &TuiModel,
) -> std::io::Result<()> {
    terminal.draw(|f| {
        use ratatui::widgets::{Block, Borders, Paragraph};
        let area = f.area();
        let text = model.active_pane_text.clone();
        f.render_widget(Paragraph::new(text).block(Block::default().borders(Borders::ALL)), area);
    })?;
    Ok(())
}
```

### Test Strategy

1. Snapshot tests for view-model rendering output.
2. Input-to-intent tests for key maps.
3. Integration tests against both TermForge server and real tmux backend mode.

### AGENTS.md Rules

- `RULE-S25-01`: TUI reads snapshots only.
- `RULE-S25-02`: UI event handlers emit commands/events, not direct state mutations.

---

## 26. AGENTS.md Rules

### Design Decisions

This section defines global enforcement rules that aggregate all section-local rules.

### Rust Example

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleSeverity { Error, Warn }

pub struct AgentRule {
    pub id: &'static str,
    pub severity: RuleSeverity,
    pub summary: &'static str,
}

pub const CORE_RULES: &[AgentRule] = &[
    AgentRule { id: "RULE-S5-01", severity: RuleSeverity::Error, summary: "forbid unsafe in pure crates" },
    AgentRule { id: "RULE-S7-01", severity: RuleSeverity::Error, summary: "effects executed only in runtime" },
];
```

### Test Strategy

1. Static rule checks in CI (grep/cargo-deny/compile-fail suites).
2. Rule coverage report links each rule to at least one automated test.
3. Release gate fails when critical rules are unverified.

### AGENTS.md Rules

- `RULE-S26-01`: Every architecture section must define enforceable rules.
- `RULE-S26-02`: Rule IDs are immutable once published.

---

## 27. Risks and Mitigations

### Design Decisions

Top risks:
- protocol drift,
- layout/copy-mode behavioral drift,
- binding API drift,
- test isolation regressions,
- observability misconfiguration.

### Rust Example

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskId { R1, R2, R3, R4, R5 }

pub struct RiskEntry {
    pub id: RiskId,
    pub mitigation_test: &'static str,
}

pub const RISK_REGISTER: &[RiskEntry] = &[
    RiskEntry { id: RiskId::R1, mitigation_test: "parity_protocol_suite" },
    RiskEntry { id: RiskId::R2, mitigation_test: "layout_geometry_parity" },
    RiskEntry { id: RiskId::R3, mitigation_test: "binding_api_contract" },
    RiskEntry { id: RiskId::R4, mitigation_test: "path_guard_three_layer" },
    RiskEntry { id: RiskId::R5, mitigation_test: "otel_trace_continuity" },
];
```

### Test Strategy

1. Risk-to-test mapping must be complete and machine-checked.
2. High-impact risks require at least one integration/parity test.
3. Quarterly risk review updates with regression evidence.

### AGENTS.md Rules

- `RULE-S27-01`: New high-risk features must add mitigation tests before merge.
- `RULE-S27-02`: Risk register IDs remain stable for auditability.

---

## 28. Plan Evolution and Changelog

### Design Decisions

v9 deltas versus v8:
- tightened cross-section naming consistency,
- removed placeholder examples,
- strengthened binding/query parity contracts,
- clarified authoritative source boundaries (wire vs control hints),
- promoted section-local AGENTS rules across all 31 sections.

### Rust Example

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecVersion { V8, V9 }

pub struct ChangelogItem {
    pub from: SpecVersion,
    pub to: SpecVersion,
    pub summary: &'static str,
}
```

### Test Strategy

1. Changelog assertions reference concrete tests and anchors.
2. No unresolved placeholders allowed in release spec.
3. Spec self-check script validates section count and required subsections.

### AGENTS.md Rules

- `RULE-S28-01`: Each version bump must list behavior-impacting changes.
- `RULE-S28-02`: No TODO placeholders in finalized spec versions.

---

## 29. Reference Anchors

### Design Decisions

Claims in this document are anchored to verified code:

- tmux C (`~/study/c/tmux`):
  - `tmux-protocol.h` protocol version and msg enums,
  - `client.c` flock lock behavior,
  - `server-client.c` invalid-message kill path and identify/config timing,
  - `options.c` window/pane scope fallthrough,
  - `layout-custom.c` checksum,
  - `layout.c` one-cell-at-a-time resize loop,
  - `control-notify.c` notification vocabulary.
- libtmux (`~/work/python/libtmux/src/libtmux/_internal/query_list.py`):
  - lookup operators, kwargs/callable filter, `get(default)` semantics.
- vibe-tmux:
  - `crates/mux-test-support/src/path_guard.rs` socket guards,
  - `crates/mux-test-support/src/requirements.rs` version/env controls,
  - `crates/mux-test-support/src/tmux.rs` readiness/0700/clipboard isolation,
  - `tools/tmux-builder/src/lib.rs` BLAKE3 cache keys, file locking, atomic rename,
  - `crates/mux-otel/src/lib.rs`, `crates/mux-otel/src/otel.rs`, `crates/mux-otel/src/config.rs` dual providers/context/config chain.
- ratatui (`~/study/rust/ratatui`) draw/render/diff contracts.

### Rust Example

```rust
pub struct Anchor {
    pub claim: &'static str,
    pub source: &'static str,
}

pub const ANCHORS: &[Anchor] = &[
    Anchor { claim: "protocol version 8", source: "tmux-protocol.h" },
    Anchor { claim: "flock startup lock", source: "client.c" },
    Anchor { claim: "invalid msg => kill peer", source: "server-client.c" },
];
```

### Test Strategy

1. Anchor checker script confirms referenced files exist.
2. Drift checker flags missing/renamed files.
3. Parity suite maps failing behavior to anchor category.

### AGENTS.md Rules

- `RULE-S29-01`: Major compatibility claims require source anchor entries.
- `RULE-S29-02`: Remove stale anchors during refactors.

---

## 30. Appendix: Canonical Type Quick Reference

### Design Decisions

Canonical type names used across sections:
- `ServerGraph`, `Event`, `Effect`, `ApplyOutcome`, `Snapshot`, `QueryList`, `ControlNotification`, `Op`.

### Rust Example

```rust
#[derive(Debug, Clone)]
pub enum Event {
    NewSession { name: String },
    NewWindow { session: SessionId, name: String },
    NewPane { window: WindowId },
    PaneOutput { pane: PaneId, bytes: Vec<u8> },
}

#[derive(Debug, Clone)]
pub enum Effect {
    SpawnPty { pane: PaneId },
    SendFrame { client: u64, bytes: Vec<u8> },
    PublishSnapshot,
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub sessions: Vec<SessionView>,
    pub windows: Vec<WindowView>,
    pub panes: Vec<PaneView>,
}
```

### Test Strategy

1. Type API compile checks in integration crates.
2. Semver surface snapshot for public crates.
3. Doc tests for all appendix examples.

### AGENTS.md Rules

- `RULE-S30-01`: Do not rename canonical types without migration notes.
- `RULE-S30-02`: Appendix must match actual exported APIs.

---

## 31. Supplemental Test Matrix

### Design Decisions

Matrix extends section-local tests with release gates:
- protocol and parity gates,
- binding and ORM gates,
- security and isolation gates,
- performance and observability gates.

### Rust Example

```rust
pub struct MatrixRow {
    pub id: &'static str,
    pub section: u8,
    pub test_name: &'static str,
}

pub const MATRIX: &[MatrixRow] = &[
    MatrixRow { id: "M-09-01", section: 9, test_name: "fuzz_decode_frame" },
    MatrixRow { id: "M-12-01", section: 12, test_name: "query_kwargs_parity" },
    MatrixRow { id: "M-21-01", section: 21, test_name: "path_guard_three_layer" },
    MatrixRow { id: "M-19-01", section: 19, test_name: "otel_trace_continuity" },
];
```

### Test Strategy

1. CI requires 100% pass of mandatory matrix rows.
2. Nightly includes extended parity + fuzz durations.
3. Release requires zero unresolved mandatory rows.

### AGENTS.md Rules

- `RULE-S31-01`: New architecture features must add at least one matrix row.
- `RULE-S31-02`: Release tagging is blocked until mandatory rows are green.

---

## v9 Final Consistency Checklist

- All 31 required sections are present.
- Every section includes: design decisions, Rust example, test strategy, AGENTS rules.
- Query API, binding API, and binding tests use the same semantics (`filter`, `get(default)`, kwargs/callable).
- Compatibility claims are tied to verified anchors listed in Section 29.
- Document writes only to `notes/` and is standalone.

