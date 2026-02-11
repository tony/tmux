Created the Pass 2 spec at:

`notes/2026-02-10-rust-architectural-approach-gpt5-v9-pass2.md`

What I validated:
- 31 numbered sections exist.
- Every section includes all required subsections: `Design Decisions`, `Rust Example`, `Test Strategy`, and `AGENTS.md Rules`.
- Pass 1 weaknesses are explicitly captured and resolved in Section 28.
- Reference claims are anchored to verified symbols/files (tmux C, libtmux, vibe-tmux OTEL/builder/path-guard).
- Only `notes/` was written; no source files were modified.
erified implementation patterns), GPT structure (uniform section contract + rule naming), and Gemini acceptance-table clarity.
- Correct Pass 1 weaknesses (stubs, placeholder APIs, and unverified assumptions).
- Keep all compatibility-critical behavior anchored to verified upstream references.

Verified reference anchors used in this pass:
- `~/work/python/libtmux/src/libtmux/_internal/query_list.py`
- `~/work/rust/vibe-tmux/crates/mux-test-support/src/path_guard.rs`
- `~/work/rust/vibe-tmux/tools/tmux-builder/src/lib.rs`
- `~/work/rust/vibe-tmux/crates/mux-otel/src/otel.rs`
- `~/study/c/tmux/tmux-protocol.h`, `~/study/c/tmux/client.c`, `~/study/c/tmux/layout.c`

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
26. AGENTS.md Rules (Global)
27. Risks and Mitigations
28. Plan Evolution and Pass 2 Changelog
29. Reference Anchors
30. Appendix: Canonical Type Quick Reference
31. Supplemental Test Matrix

---

## 1. Vision and Philosophy
### Design Decisions
- TermForge is a new Rust architecture, not a C port.
- tmux is a compatibility target at wire + behavior boundaries.
- Core state transitions are deterministic and replayable.

### Rust Example
```rust
pub trait Reducer {
    fn apply(&mut self, event: Event, ctx: CoreCtx) -> ApplyOutcome;
}

#[derive(Clone, Copy, Debug)]
pub struct CoreCtx {
    pub now_ms: u64,
    pub entropy: u64,
}
```

### Test Strategy
- Golden replay hash test for deterministic outcomes.
- Cross-run reproducibility with fixed context seed.
- Basic attach/create-window parity smoke against tmux.

### AGENTS.md Rules
- `RULE-S1-01`: Never perform IO inside reducer code.
- `RULE-S1-02`: Compatibility claims require an anchor in Section 29.

---

## 2. North Star Acceptance Criteria
### Design Decisions
Criteria are grouped by Compatibility (C), Architecture (A), Product (P), and Performance (B).

### Rust Example
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gate {
    C1WireAttach,
    C2CrossAttach,
    C3ProtocolRoundtrip,
    C4IdentifyBurst,
    C5ScmRights,
    A1PureCore,
    A2WasmPurity,
    P1Embedding,
    B1ParserThroughput,
}

pub fn gates_green(values: &[(Gate, bool)]) -> bool {
    values.iter().all(|(_, ok)| *ok)
}
```

### Acceptance Tables
| C-ID | Criterion | Verification |
|---|---|---|
| C1 | Stock tmux client attaches to TermForge | Integration attach suite |
| C2 | TermForge client attaches to stock tmux | Cross-attach suite |
| C3 | Protocol v8 roundtrip | Property tests over headers/payloads |
| C4 | Identify burst 100..112 fidelity | Captured fixture parity |
| C5 | SCM_RIGHTS correctness | FD lifecycle tests |

| A-ID | Criterion | Enforcement |
|---|---|---|
| A1 | Pure crates contain no `unsafe` | lint + CI grep |
| A2 | Pure crates compile to `wasm32-unknown-unknown` | dedicated CI job |
| A3 | `mux-os` is unsafe quarantine | workspace policy |

| P-ID | Criterion | Verification |
|---|---|---|
| P1 | In-process embedding API | integration tests |
| P2 | Python/Node query parity | binding contract tests |
| P3 | Fake PTY replay | deterministic scenario harness |

| B-ID | Target | Gate |
|---|---|---|
| B1 | VT parser ASCII > 300 MB/s | benchmark regression gate |
| B2 | Protocol decode > 500 MB/s | benchmark regression gate |
| B3 | Layout resize < 50 us (20 panes) | microbench gate |

### Test Strategy
- CI gate matrix must include all C/A/P/B rows.
- Nightly tmux-version matrix (3.3a..3.6a + rolling).
- Failed gate blocks merge unless explicitly waived.

### AGENTS.md Rules
- `RULE-S2-01`: Do not merge with red C/A gates.
- `RULE-S2-02`: New feature PRs must map to at least one gate row.

---

## 3. High-Level Architecture
### Design Decisions
- Layer 0: pure domain + protocol logic.
- Layer 1: runtime/effects.
- Layer 2: API façade.
- Layer 3: ORM conveniences.
- Layer 4: bindings.

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
- Cargo dependency-direction policy test.
- Forbidden import scan (`tokio` in pure crates).
- End-to-end parity test from same script in in-proc and socket modes.

### AGENTS.md Rules
- `RULE-S3-01`: Dependencies flow inward only.
- `RULE-S3-02`: Bindings cannot depend on runtime internals.

---

## 4. Workspace Layout
### Design Decisions
- Keep pure crates small and leaf-oriented.
- Separate `mux-types` and `mux-os` explicitly (Pass 1 ambiguity removed).

### Rust Example
```toml
[workspace]
resolver = "2"
members = [
  "crates/mux-types",
  "crates/mux-core",
  "crates/mux-query",
  "crates/mux-proto",
  "crates/mux-runtime",
  "crates/mux-api",
  "crates/mux-orm",
  "crates/mux-otel",
  "bindings/python",
  "bindings/node",
  "tools/tmux-builder",
]
```

### Test Strategy
- Workspace manifest sanity check.
- Required crate presence check (`mux-types`, `mux-os`).
- `cargo check --workspace --all-targets` on every PR.

### AGENTS.md Rules
- `RULE-S4-01`: New crates must declare pure/impure class.
- `RULE-S4-02`: Core logic is prohibited in tools/bindings crates.

---

## 5. Layering Contract
### Design Decisions
- Pure core: no OS calls, no async runtime, no unsafe.
- WASM compile success is treated as purity proof.

### Rust Example
```rust
#![forbid(unsafe_code)]

pub fn pure_transition(state: &State, event: &Event, ctx: CoreCtx) -> State {
    let mut next = state.clone();
    next.apply(event, ctx);
    next
}
```

### Test Strategy
- `cargo check -p mux-core --target wasm32-unknown-unknown`.
- Static scan for forbidden crates in pure layer.
- Determinism property tests on transition function.

### AGENTS.md Rules
- `RULE-S5-01`: Any pure-layer `unsafe` usage is a hard stop.
- `RULE-S5-02`: New pure APIs must be context-injected (time/randomness via params).

---

## 6. Entity Model
### Design Decisions
- SlotMap typed IDs for stable handles.
- Parent->children stored in entities; reverse maps in snapshots.

### Rust Example
```rust
use slotmap::{SlotMap, new_key_type};

new_key_type! { pub struct SessionId; pub struct WindowId; pub struct PaneId; }

pub struct ServerGraph {
    pub sessions: SlotMap<SessionId, Session>,
    pub windows: SlotMap<WindowId, Window>,
    pub panes: SlotMap<PaneId, Pane>,
}

pub struct GraphState {
    pub graph: std::sync::Arc<ServerGraph>,
    pub session_for_window: std::collections::BTreeMap<WindowId, SessionId>,
    pub window_for_pane: std::collections::BTreeMap<PaneId, WindowId>,
}
```

### Test Strategy
- ID generation and stale-handle safety tests.
- Snapshot reverse-map consistency tests.
- Session/window/pane topology invariant tests.

### AGENTS.md Rules
- `RULE-S6-01`: Do not use integer IDs directly in public APIs.
- `RULE-S6-02`: Reverse maps belong in snapshot layer, not mutable graph.

---

## 7. Event/Effect Engine
### Design Decisions
- Reducer returns effects; runtime interprets effects.
- Effect failures re-enter graph as explicit events.

### Rust Example
```rust
pub enum Effect {
    SpawnPty { pane_id: PaneId },
    SendClientFrame { client_id: u64, bytes: Vec<u8> },
}

pub enum Event {
    Command(Command),
    PtyOutput { pane_id: PaneId, data: Vec<u8> },
    EffectFailed { effect: Effect, reason: String },
}

pub struct ApplyOutcome {
    pub effects: Vec<Effect>,
}
```

### Test Strategy
- Event ordering under concurrent producers.
- Effect failure feedback loop correctness.
- Duplicate event idempotence where required.

### AGENTS.md Rules
- `RULE-S7-01`: Reducer cannot execute effects directly.
- `RULE-S7-02`: Every effect failure must become an explicit event.

---

## 8. Error Handling
### Design Decisions
- Classify errors by actionability.
- Protocol violations are fatal to the peer connection.

### Rust Example
```rust
#[derive(Debug, thiserror::Error)]
pub enum ProtocolViolation {
    #[error("frame too short")]
    FrameTooShort,
    #[error("unknown msg type {0}")]
    UnknownMsgType(u32),
    #[error("payload mismatch")]
    PayloadMismatch,
}

#[derive(Debug, thiserror::Error)]
pub enum TfError {
    #[error(transparent)]
    Violation(#[from] ProtocolViolation),
    #[error("domain error: {0}")]
    Domain(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}
```

### Test Strategy
- Decode fuzz tests with no panics.
- Fatal disconnect behavior for protocol violations.
- Binding error mapping tests (Python/Node surface).

### AGENTS.md Rules
- `RULE-S8-01`: Never swallow protocol violations.
- `RULE-S8-02`: Public APIs return typed errors, not bare strings.

---

## 9. Protocol Codec
### Design Decisions
- Implement tmux imsg framing with strict header checks.
- Support tokio `Decoder/Encoder` integration.

### Rust Example
```rust
use tokio_util::codec::{Decoder, Encoder};

#[derive(Debug, Clone, Copy)]
pub struct ImsgHdr {
    pub typ: u32,
    pub len: u32,
    pub peerid: u32,
    pub pid: u32,
}

pub struct ImsgCodec;

impl Decoder for ImsgCodec {
    type Item = Vec<u8>;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut bytes::BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < 16 { return Ok(None); }
        let len = u32::from_ne_bytes(src[4..8].try_into().unwrap());
        let frame_len = (len & 0x7fff_ffff) as usize;
        if frame_len < 16 {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "short frame"));
        }
        if src.len() < frame_len { return Ok(None); }
        Ok(Some(src.split_to(frame_len).to_vec()))
    }
}
```

### Test Strategy
- Header roundtrip tests for all message types.
- Identify burst fixture comparison (100..112 sequence).
- Backpressure and partial-frame decode tests.

### AGENTS.md Rules
- `RULE-S9-01`: Never accept frame length < header size.
- `RULE-S9-02`: Unknown message types are protocol violations unless explicitly negotiated.

---

## 10. Configuration System
### Design Decisions
- Parse config to typed command stream.
- Apply after identify completion (tmux ordering parity).

### Rust Example
```rust
#[derive(Debug, Clone)]
pub struct ConfigCommand {
    pub name: String,
    pub args: Vec<String>,
}

pub fn parse_config(input: &str) -> Result<Vec<ConfigCommand>, TfError> {
    input
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
        .map(|line| {
            let mut parts = line.split_whitespace();
            let Some(name) = parts.next() else {
                return Err(TfError::Domain("empty command".into()));
            };
            Ok(ConfigCommand { name: name.into(), args: parts.map(str::to_owned).collect() })
        })
        .collect()
}
```

### Test Strategy
- Parser fixtures with comments/quoting cases.
- Ordering test: identify then config.
- Option scope fallback tests (server/session/window/pane).

### AGENTS.md Rules
- `RULE-S10-01`: Config parser is pure and side-effect free.
- `RULE-S10-02`: Config reload emits events, never mutates state out-of-band.

---

## 11. Layout Engine
### Design Decisions
- Flat arena layout tree.
- Round-robin resize distribution (matching `layout_resize_adjust`).

### Rust Example
```rust
pub const PANE_MINIMUM: u16 = 1;

pub fn layout_checksum(layout: &[u8]) -> u16 {
    let mut csum = 0u16;
    for &b in layout {
        csum = (csum >> 1) | ((csum & 1) << 15);
        csum = csum.wrapping_add(u16::from(b));
    }
    csum
}

pub fn can_split(span: u16) -> bool {
    span >= PANE_MINIMUM * 2 + 1
}
```

### Test Strategy
- Layout parse/dump roundtrip.
- Resize parity scripts against tmux for representative trees.
- Invariant checks (`layout_check`) after each mutation.

### AGENTS.md Rules
- `RULE-S11-01`: Do not replace round-robin with proportional math without proof.
- `RULE-S11-02`: Mutating layout code requires invariant assertions in tests.

---

## 12. ORM-like Query API
### Design Decisions
- Match libtmux QueryList semantics: kwargs, callable filter, nested path, `get(default=...)`.
- Include verified operators: exact/iexact/contains/icontains/startswith/istartswith/endswith/iendswith/in/nin/regex/iregex (+ `eq` alias).

### Rust Example
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryOp {
    Exact, IExact, Contains, IContains,
    StartsWith, IStartsWith, EndsWith, IEndsWith,
    In, Nin, Regex, IRegex,
}

pub fn parse_kwargs_key(key: &str) -> (String, QueryOp) {
    if let Some((prefix, suffix)) = key.rsplit_once("__") {
        let op = match suffix {
            "eq" | "exact" => Some(QueryOp::Exact),
            "iexact" => Some(QueryOp::IExact),
            "contains" => Some(QueryOp::Contains),
            "icontains" => Some(QueryOp::IContains),
            "startswith" => Some(QueryOp::StartsWith),
            "istartswith" => Some(QueryOp::IStartsWith),
            "endswith" => Some(QueryOp::EndsWith),
            "iendswith" => Some(QueryOp::IEndsWith),
            "in" => Some(QueryOp::In),
            "nin" => Some(QueryOp::Nin),
            "regex" => Some(QueryOp::Regex),
            "iregex" => Some(QueryOp::IRegex),
            _ => None,
        };
        if let Some(op) = op { return (prefix.to_string(), op); }
    }
    (key.to_string(), QueryOp::Exact)
}
```

### Test Strategy
- Operator parity test list mirrors `LOOKUP_NAME_MAP` entries.
- Nested traversal tests (`food__fruit__in` style).
- `get(default=...)`, `ObjectDoesNotExist`, and `MultipleObjectsReturned` behavior tests.

### AGENTS.md Rules
- `RULE-S12-01`: Every documented operator must have explicit tests.
- `RULE-S12-02`: Bindings must expose the same kwargs semantics as Rust ORM.

---

## 13. Runtime Architecture
### Design Decisions
- Single writer actor for mutable graph.
- Snapshot store for lock-free reads.

### Rust Example
```rust
pub struct StateActor {
    graph: ServerGraph,
    rx: tokio::sync::mpsc::Receiver<Event>,
    effect_tx: tokio::sync::mpsc::Sender<Effect>,
}

impl StateActor {
    pub async fn run(mut self) {
        while let Some(event) = self.rx.recv().await {
            let outcome = apply_event(&mut self.graph, event, CoreCtx { now_ms: 0, entropy: 0 });
            for effect in outcome.effects {
                let _ = self.effect_tx.send(effect).await;
            }
        }
    }
}
```

### Test Strategy
- Actor serialization tests.
- Snapshot freshness tests per processed event.
- Graceful shutdown and drain tests.

### AGENTS.md Rules
- `RULE-S13-01`: Graph mutation is single-writer only.
- `RULE-S13-02`: Runtime tasks communicate by events/effects, not shared mutation.

---

## 14. Server Lifecycle
### Design Decisions
- Use `flock(LOCK_EX|LOCK_NB)` semantics for startup ownership.
- Defer config until identify phase completes.

### Rust Example
```rust
pub fn acquire_server_lock(path: &std::path::Path) -> anyhow::Result<std::fs::File> {
    use fs2::FileExt;
    let file = std::fs::OpenOptions::new().create(true).read(true).write(true).open(path)?;
    file.try_lock_exclusive()?;
    Ok(file)
}
```

### Test Strategy
- Lock contention tests (second server fails fast).
- Crash-release behavior tests.
- Identify-before-config ordering tests.

### AGENTS.md Rules
- `RULE-S14-01`: Do not introduce PID-file lock fallback.
- `RULE-S14-02`: Startup sequence changes need race tests.

---

## 15. Control Mode
### Design Decisions
- Parse `%` notifications to typed variants with generic fallback.
- Control notifications are hints; snapshots remain authority.

### Rust Example
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlNotification {
    SessionsChanged,
    SessionChanged { id: u32, name: String },
    ExtendedOutput { pane: u32, age_ms: u64, data: Vec<u8> },
    Generic { raw: String },
}

pub fn parse_control_line(line: &str) -> ControlNotification {
    if line.starts_with("%sessions-changed") {
        ControlNotification::SessionsChanged
    } else {
        ControlNotification::Generic { raw: line.to_string() }
    }
}
```

### Test Strategy
- Real-notification fixture parse tests.
- Unknown-notification robustness tests.
- Backpressure limit tests for per-client pending bytes.

### AGENTS.md Rules
- `RULE-S15-01`: Unknown control lines must never panic parser.
- `RULE-S15-02`: Control channel cannot bypass reconciliation path.

---

## 16. Language Bindings
### Design Decisions
- Python (PyO3) and Node (Neon) expose equivalent ORM surfaces.
- Keep binding-specific logic thin; core semantics remain in Rust crates.

### Rust Example
```rust
#[cfg(feature = "python")]
#[pyo3::pyclass]
pub struct PyServer {
    inner: mux_orm::OrmServer,
}

#[cfg(feature = "python")]
#[pyo3::pymethods]
impl PyServer {
    fn cmd(&self, py: pyo3::Python<'_>, cmd: &str) -> pyo3::PyResult<String> {
        let inner = self.inner.clone();
        let cmd = cmd.to_owned();
        py.detach(move || inner.cmd(&cmd).map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string())))
    }
}
```

### Test Strategy
- PyQueryList/JsQueryList API parity tests.
- Error mapping tests for not-found/multiple-results semantics.
- Async boundary tests for Neon `deferred.settle_with`.

### AGENTS.md Rules
- `RULE-S16-01`: Binding methods must map to existing ORM APIs only.
- `RULE-S16-02`: Python and Node docs must describe equivalent filter/get behavior.

---

## 17. CRDT Transaction Layer
### Design Decisions
- HLC timestamps for causality.
- OR-set/LWW composition for replicated entities.

### Rust Example
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Hlc {
    pub wall_ms: i64,
    pub counter: u32,
    pub node_id: u64,
}

impl Hlc {
    pub fn merge(local: Self, remote: Self, wall_ms: i64, node_id: u64) -> Self {
        let wall = wall_ms.max(local.wall_ms).max(remote.wall_ms);
        let counter = if wall == local.wall_ms && wall == remote.wall_ms {
            local.counter.max(remote.counter) + 1
        } else if wall == local.wall_ms {
            local.counter + 1
        } else if wall == remote.wall_ms {
            remote.counter + 1
        } else {
            0
        };
        Self { wall_ms: wall, counter, node_id }
    }
}
```

### Test Strategy
- Commutative/associative/idempotent merge tests.
- Concurrent create/delete convergence tests.
- Partition/rejoin scenario simulation.

### AGENTS.md Rules
- `RULE-S17-01`: CRDT ops must be deterministic under replay.
- `RULE-S17-02`: Merge code requires algebraic property tests.

---

## 18. Security Model
### Design Decisions
- Unix socket permissions + directory hardening are first boundary.
- SCM_RIGHTS accepted only in expected protocol phases.

### Rust Example
```rust
pub fn bind_secure_socket(path: &std::path::Path) -> Result<std::os::unix::net::UnixListener, std::io::Error> {
    #[cfg(unix)]
    let old_umask = unsafe { libc::umask(0o177) };
    let result = std::os::unix::net::UnixListener::bind(path);
    #[cfg(unix)]
    unsafe { libc::umask(old_umask); }
    result
}
```

### Test Strategy
- Socket mode and dir permission checks.
- Symlink/path traversal rejection tests.
- SCM_RIGHTS accept/reject phase tests with FD cleanup assertions.

### AGENTS.md Rules
- `RULE-S18-01`: Keep all handwritten unsafe in `mux-os` only.
- `RULE-S18-02`: Every security boundary change needs adversarial tests.

---

## 19. OpenTelemetry
### Design Decisions
- Dual providers (`OTEL_PROVIDER`, `MUX_CLIENT_PROVIDER`) with lazy client init.
- Thread-local trace header stack + RAII guards.
- Composite propagator (baggage + trace context).

### Rust Example
```rust
use std::sync::{Mutex, OnceLock};

static OTEL_PROVIDER: OnceLock<Mutex<Option<OtelProvider>>> = OnceLock::new();
static MUX_CLIENT_PROVIDER: OnceLock<Mutex<Option<OtelProvider>>> = OnceLock::new();

pub fn force_flush_all() -> bool {
    let mut ok = true;
    for slot in [OTEL_PROVIDER.get(), MUX_CLIENT_PROVIDER.get()].into_iter().flatten() {
        let guard = slot.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(provider) = guard.as_ref() {
            ok &= provider
                .tracer_provider
                .as_ref()
                .is_none_or(|tp| tp.force_flush().is_ok());
        }
    }
    ok
}
```

### Test Strategy
- Dual provider lifecycle tests (`init`/`flush`/`shutdown`).
- Header inject/extract roundtrip tests.
- Runtime timeout-on-drop tests for short-lived binaries.

### AGENTS.md Rules
- `RULE-S19-01`: Client provider must remain lazy; do not initialize on startup.
- `RULE-S19-02`: Shutdown path must flush both providers before drop.

---

## 20. tmux Version Management
### Design Decisions
- BLAKE3 cache key over version + host + flags.
- File lock guards for clone and cache-key build.
- Atomic publish via sibling temp dir + rename.

### Rust Example
```rust
fn sanitize_token(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') { ch } else { '_' })
        .collect()
}

fn flags_fingerprint(flags: &[String]) -> String {
    let mut data = Vec::new();
    for flag in flags {
        data.extend_from_slice(flag.as_bytes());
        data.push(0);
    }
    let hash = blake3::hash(&data);
    let hex = hash.to_hex().to_string();
    hex[..12].to_string()
}
```

### Test Strategy
- Cache key determinism and uniqueness tests.
- Lock exclusivity and auto-release-on-drop tests.
- Atomic publish integrity tests under concurrent readers.

### AGENTS.md Rules
- `RULE-S20-01`: Build outputs must publish atomically, never by copy-overwrite.
- `RULE-S20-02`: Lock lifecycle must be RAII-based.

---

## 21. Test Support and Fake PTY
### Design Decisions
- Mandatory 3-layer socket path validation from `path_guard` pattern.
- Support both in-process and subprocess harness modes.

### Rust Example
```rust
pub fn ensure_safe_socket(socket_name: &str, socket_path: &std::path::Path, tempdir: &std::path::Path) -> anyhow::Result<()> {
    mux_test_support::path_guard::ensure_not_default_socket_name(socket_name)?;
    mux_test_support::path_guard::ensure_socket_within_tempdir(socket_path, tempdir)?;
    mux_test_support::path_guard::ensure_socket_not_tmux_env(socket_path)?;
    Ok(())
}
```

### Test Strategy
- Default socket-name rejection tests.
- `$TMUX` collision rejection tests.
- Fake PTY scenario record/replay equivalence tests.

### AGENTS.md Rules
- `RULE-S21-01`: Path guard checks are mandatory before destructive actions.
- `RULE-S21-02`: Test harness socket dirs must be permission-hardened.

---

## 22. Binding Test Frameworks
### Design Decisions
- pytest and vitest must run against both in-process and socket backends.
- Query semantics are a shared compatibility contract.

### Rust Example
```rust
pub enum BackendMode {
    InProcess,
    Socket,
}

pub fn backend_modes() -> [BackendMode; 2] {
    [BackendMode::InProcess, BackendMode::Socket]
}
```

### Test Strategy
- Parametrized dual-backend suites in Python and Node.
- Binding exception-class parity tests.
- Traceparent propagation tests from host runtime into Rust spans.

### AGENTS.md Rules
- `RULE-S22-01`: New binding tests must execute in both backend modes.
- `RULE-S22-02`: Binding fixture APIs must remain transport-transparent.

---

## 23. Test Framework and Harness Design
### Design Decisions
- Multi-layer test pyramid: property, fixture, integration, regress.
- Prefer deterministic fixtures + snapshot assertions for UX-critical output.

### Rust Example
```rust
#[test]
fn vt_snapshot() {
    let mut grid = mux_grid::Grid::new(80, 24);
    let mut parser = mux_grid::VtParser::new();
    parser.parse(b"hello\r\n", &mut grid);
    insta::assert_snapshot!(grid.to_text());
}
```

### Test Strategy
- VT parser fixtures + fuzzing.
- Protocol fixture replay with FD metadata where applicable.
- Regress runner across selected tmux versions.

### AGENTS.md Rules
- `RULE-S23-01`: Every protocol/layout fix needs a regression fixture.
- `RULE-S23-02`: Nondeterministic tests must be quarantined from merge gates.

---

## 24. Performance Targets
### Design Decisions
- Targets are explicit and benchmarked nightly.
- Regressions use relative gates to control noise.

### Rust Example
```rust
#[derive(Debug, Clone, Copy)]
pub struct PerfBudget {
    pub vt_ascii_mb_s: f64,
    pub proto_decode_mb_s: f64,
    pub layout_resize_us_p99: f64,
}

pub const PERF_BUDGET: PerfBudget = PerfBudget {
    vt_ascii_mb_s: 300.0,
    proto_decode_mb_s: 500.0,
    layout_resize_us_p99: 50.0,
};
```

### Test Strategy
- Criterion microbench suites with pinned CPU governor in CI runners.
- Rolling median over three runs.
- 130% regression alert threshold for nightly gates.

### AGENTS.md Rules
- `RULE-S24-01`: Performance claims require benchmark artifact links.
- `RULE-S24-02`: Hot-path allocations must be explicitly justified.

---

## 25. Visual Client / TUI
### Design Decisions
- ratatui rendering with pure `ViewModel` stage.
- TUI reads immutable snapshots only.

### Rust Example
```rust
pub struct ViewModel {
    pub tabs: Vec<String>,
    pub panes: Vec<String>,
}

pub fn build_view_model(state: &mux_view::Snapshot, ui: &UiState) -> ViewModel {
    let tabs = state.sessions.values().map(|s| s.name.clone()).collect();
    let panes = ui.visible_panes.clone();
    ViewModel { tabs, panes }
}
```

### Test Strategy
- Golden render tests for key layouts.
- Resize behavior tests (mobile and desktop terminal sizes).
- Input latency tests under high-output scenarios.

### AGENTS.md Rules
- `RULE-S25-01`: UI rendering must remain side-effect free.
- `RULE-S25-02`: TUI changes require snapshot tests for core screens.

---

## 26. AGENTS.md Rules (Global)
### Design Decisions
- Consolidate section-level rules into enforceable cross-cutting policy.

### Rust Example
```rust
#[derive(Debug, Clone)]
pub struct AgentPolicy {
    pub id: &'static str,
    pub text: &'static str,
}

pub const GLOBAL_POLICIES: &[AgentPolicy] = &[
    AgentPolicy { id: "RULE-G-01", text: "No unsafe outside mux-os" },
    AgentPolicy { id: "RULE-G-02", text: "Compatibility claims need anchors" },
    AgentPolicy { id: "RULE-G-03", text: "Every behavior change must add tests" },
];
```

### Test Strategy
- Policy lints and docs checks in CI.
- PR template validation (rule mapping required).
- Random audit of merged PRs against declared rules.

### AGENTS.md Rules
- `RULE-S26-01`: Keep global policy IDs stable for tooling.
- `RULE-S26-02`: Section-local rules may strengthen but not weaken global rules.

---

## 27. Risks and Mitigations
### Design Decisions
- Prioritize high-impact risks: protocol drift, FD leaks, race conditions, binding divergence.

### Rust Example
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLevel { Low, Medium, High }

pub struct RiskItem {
    pub id: &'static str,
    pub summary: &'static str,
    pub level: RiskLevel,
    pub mitigation: &'static str,
}
```

### Test Strategy
- Risk-to-test traceability table maintained in CI artifact.
- Nightly chaos tests for lock/race and flaky-network scenarios.
- Security-focused fuzzing for decode paths.

### AGENTS.md Rules
- `RULE-S27-01`: High-risk items require at least one blocking gate.
- `RULE-S27-02`: New risk entries must include owner and verification plan.

---

## 28. Plan Evolution and Pass 2 Changelog
### Design Decisions
Pass 2 explicitly resolves Pass 1 weaknesses.

### Rust Example
```rust
#[derive(Debug, Clone)]
pub struct ChangelogEntry {
    pub id: &'static str,
    pub section: u8,
    pub change: &'static str,
    pub source: &'static str,
}
```

### Pass 2 Corrections Over Pass 1
| ID | Weakness Found in Pass 1 | Pass 2 Resolution |
|---|---|---|
| P2-01 | Gemini sections 26-31 were stub-level | Fully expanded sections 26-31 with code/tests/rules |
| P2-02 | GPT Pass 1 used minimal placeholder snippets in deep sections | Replaced with reference-grounded concrete patterns |
| P2-03 | Some Pass 1 docs mixed tentative env naming without normalization | Standardized TermForge naming and listed compatibility aliases |
| P2-04 | Inconsistent explanation of ORM operator parity | Explicitly aligned to verified libtmux operator set and alias `eq` |
| P2-05 | Incomplete OTEL lifecycle details in some variants | Added dual-provider init/flush/shutdown requirements |
| P2-06 | Limited lock/cache publication rigor in compact variants | Added RAII locking + atomic publish requirements |
| P2-07 | WASM purity requirement under-emphasized outside Gemini | Promoted wasm check to architecture acceptance gate |

### Test Strategy
- Changelog regression tests ensure corrected behaviors remain covered.
- Section completeness check (all 31 sections include required subsections).
- Anchor verification script against referenced symbols.

### AGENTS.md Rules
- `RULE-S28-01`: Any future pass must list concrete corrected weaknesses.
- `RULE-S28-02`: Do not mark a weakness resolved without a corresponding test gate.

---

## 29. Reference Anchors
### Design Decisions
- Every compatibility-critical claim must point to a verified symbol.

### Rust Example
```rust
pub struct Anchor {
    pub source: &'static str,
    pub symbol: &'static str,
    pub verified: bool,
}
```

### Verified Anchors
| Source File | Symbol(s) | Purpose |
|---|---|---|
| `~/study/c/tmux/tmux-protocol.h` | `PROTOCOL_VERSION`, `MSG_IDENTIFY_*` | wire protocol baseline |
| `~/study/c/tmux/client.c` | `flock(LOCK_EX|LOCK_NB)` usage | lock semantics parity |
| `~/study/c/tmux/layout.c` | `layout_resize_adjust` | round-robin resize parity |
| `~/work/python/libtmux/src/libtmux/_internal/query_list.py` | `LOOKUP_NAME_MAP`, `keygetter`, `filter`, `get` | ORM/query parity |
| `~/work/rust/vibe-tmux/crates/mux-test-support/src/path_guard.rs` | `ensure_not_default_socket_name`, `ensure_socket_within_tempdir`, `ensure_socket_not_tmux_env` | test socket safety |
| `~/work/rust/vibe-tmux/tools/tmux-builder/src/lib.rs` | `compute_cache_key`, `flags_fingerprint`, `lock_cache_key`, `sibling_tmp_dir` | version/cache/locking |
| `~/work/rust/vibe-tmux/crates/mux-otel/src/otel.rs` | `OTEL_PROVIDER`, `MUX_CLIENT_PROVIDER`, `enter_mux_client_span`, `resolve_signal_config` | telemetry architecture |

### Test Strategy
- Automated anchor existence checks via `rg` in CI.
- Failing anchor checks block compatibility claims.
- Manual audit for newly added anchor rows.

### AGENTS.md Rules
- `RULE-S29-01`: Anchor table updates are mandatory for new compat-critical claims.
- `RULE-S29-02`: Do not cite symbols not present in repository state.

---

## 30. Appendix: Canonical Type Quick Reference
### Design Decisions
- Keep naming consistent across core/runtime/bindings.

### Rust Example
```rust
pub type ClientId = u64;

pub enum Command {
    NewSession { name: String },
    SplitPane { pane_id: PaneId },
}

pub struct Snapshot {
    pub sessions: std::collections::BTreeMap<String, SessionView>,
}
```

### Quick Reference
| Domain | Types |
|---|---|
| Identity | `SessionId`, `WindowId`, `PaneId`, `ClientId` |
| State | `ServerGraph`, `GraphState`, `Snapshot` |
| Flow | `Event`, `Effect`, `ApplyOutcome` |
| Protocol | `ImsgHdr`, `MsgType`, `ProtocolViolation` |
| Query | `QueryList<T>`, `QueryOp`, `QuerySpec` |
| Telemetry | `OtelProvider`, `TraceHeaders`, `TraceHeadersGuard` |

### Test Strategy
- API doc tests for type examples.
- Cross-crate compile checks for public type stability.
- Binding generation tests for type exposure.

### AGENTS.md Rules
- `RULE-S30-01`: Public type renames require migration notes.
- `RULE-S30-02`: Avoid duplicate type aliases across layers.

---

## 31. Supplemental Test Matrix
### Design Decisions
- Combine strongest Pass 1 test ideas into one matrix.

### Rust Example
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestTier { Unit, Property, Integration, Parity, Fuzz, Perf }
```

### Test Matrix
| Area | Core Tests | Advanced Tests | Gate |
|---|---|---|---|
| Protocol | header/decode unit tests | fuzz + fixture parity | blocking |
| Layout | parse/dump + invariants | cross-tmux geometry diff | blocking |
| Query | operator + kwargs tests | binding parity matrix | blocking |
| Runtime | actor ordering tests | chaos/race tests | blocking |
| Security | permission/path tests | FD lifecycle adversarial tests | blocking |
| OTEL | provider lifecycle tests | propagation E2E | non-blocking nightly -> blocking on regressions |
| Version Mgmt | cache key/lock tests | concurrent build stress | blocking |
| TUI | view-model unit tests | snapshot + latency under load | non-blocking nightly |

### Test Strategy
- Tiered execution: unit/property on PR, parity/perf nightly, full weekly.
- Determinism requirement: flaky tests are quarantined and tracked.
- All rows must map to section rules.

### AGENTS.md Rules
- `RULE-S31-01`: Any new subsystem must add a matrix row before merge.
- `RULE-S31-02`: Failing blocking rows prevent release tags.

---

## Final Consistency Checklist
- [x] 31 sections present.
- [x] Every section includes Design Decisions, Rust Example, Test Strategy, AGENTS.md Rules.
- [x] Pass 1 weakness corrections are explicit.
- [x] Claims are anchored to verified symbols and files.
- [x] WASM purity proof requirement retained.
- [x] Output written only under `notes/`.
