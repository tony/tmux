Completed. I wrote the fresh v10 spec to:

`notes/2026-02-10-rust-architectural-approach-gpt5-v10.md`

Validation checks passed:
- 32 numbered sections present (v9’s 31 + new Section 22 Termlets)
- Includes detailed Termlet architecture:
  - Rust API design
  - FakePty vs real PTY modes
  - `wait_for` polling + pattern matching semantics
  - Snapshot capture/comparison flow
  - Python `PyTermlet` and Node `JsTermlet` APIs
  - pytest/vitest fixture design
  - timeout/error semantics
  - explicit Pane/Grid/PtyBackend relationships
- Each numbered section includes design decisions, Rust example, test strategy, and AGENTS.md rules.

No source files were modified; only `notes/` was written.
ing pods built on pane+grid+pty
- Deterministic FakePty mode for tests
- Real PTY mode for integration
- Snapshot-first ergonomics for Rust/Python/Node

### Re-verified references used in this pass

- `~/study/c/tmux/tmux-protocol.h` (`PROTOCOL_VERSION 8`)
- `~/study/c/tmux/client.c` (`flock(LOCK_EX|LOCK_NB)`)
- `~/study/c/tmux/layout.c` (round-robin resize behavior)
- `~/study/c/tmux/options.c` (`FALLTHROUGH` option scope rules)
- `~/work/python/libtmux/src/libtmux/_internal/query_list.py` (12 operators, kwargs filtering, callable matcher)
- `~/work/rust/vibe-tmux/crates/mux-test-support/src/path_guard.rs` (3-layer socket safety)
- `~/work/rust/vibe-tmux/tools/tmux-builder/src/lib.rs` (BLAKE3 cache keys + lock discipline)
- `~/work/rust/vibe-tmux/crates/mux-otel/src/otel.rs` (dual provider OTEL)

### Settled Decisions (v10)

| # | Decision | Rationale |
|---|---|---|
| S1 | New Rust multiplexer with tmux compatibility mode | Preserve compatibility and architecture freedom |
| S2 | Protocol compatibility locked to tmux protocol v8 | Source parity with upstream header |
| S3 | Pure kernel + impure runtime separation | Determinism and testability |
| S4 | SlotMap IDs + flat layout arena | Stable entity identity and efficient traversal |
| S5 | Round-robin resize semantics | Behavioral parity with tmux layout logic |
| S6 | Protocol violation terminates connection | Match tmux strictness |
| S7 | Config loads after identify burst | Parity with server bootstrap timing |
| S8 | FakePty scenario replay is required | Deterministic tests |
| S9 | OTEL dual provider model | Avoid client/server tracing contention |
| S10 | **Termlet is a pane-backed SDK handle** | Killer feature without protocol breakage |

---

## Table of Contents

1. [Vision and Philosophy](#1-vision-and-philosophy)
2. [North Star Acceptance Criteria](#2-north-star-acceptance-criteria)
3. [High-Level Architecture](#3-high-level-architecture)
4. [Workspace Layout](#4-workspace-layout)
5. [Layering Contract](#5-layering-contract)
6. [Entity Model](#6-entity-model)
7. [Event/Effect Engine](#7-eventeffect-engine)
8. [Error Handling](#8-error-handling)
9. [Protocol Codec](#9-protocol-codec)
10. [Configuration System](#10-configuration-system)
11. [Layout Engine](#11-layout-engine)
12. [ORM-like Query API](#12-orm-like-query-api)
13. [Runtime Architecture](#13-runtime-architecture)
14. [Server Lifecycle](#14-server-lifecycle)
15. [Control Mode](#15-control-mode)
16. [Language Bindings](#16-language-bindings)
17. [CRDT Transaction Layer](#17-crdt-transaction-layer)
18. [Security Model](#18-security-model)
19. [OpenTelemetry](#19-opentelemetry)
20. [tmux Version Management](#20-tmux-version-management)
21. [Test Support and Fake PTY](#21-test-support-and-fake-pty)
22. [Termlets (SDK-first Testing Pods)](#22-termlets-sdk-first-testing-pods)
23. [Binding Test Frameworks](#23-binding-test-frameworks)
24. [Test Framework and Harness Design](#24-test-framework-and-harness-design)
25. [Performance Targets](#25-performance-targets)
26. [Visual Client / TUI](#26-visual-client--tui)
27. [AGENTS.md Rules](#27-agentsmd-rules)
28. [Risks and Mitigations](#28-risks-and-mitigations)
29. [Plan Evolution and Changelog](#29-plan-evolution-and-changelog)
30. [Reference Anchors](#30-reference-anchors)
31. [Appendix: Canonical Type Quick Reference](#31-appendix-canonical-type-quick-reference)
32. [Supplemental Test Matrix](#32-supplemental-test-matrix)

---

## 1. Vision and Philosophy

### Design Decisions

- TermForge is a new multiplexer kernel; tmux is a compatibility profile, not internal architecture.
- Core contract remains pure reducer semantics: `Event + CoreCtx -> ApplyOutcome`.
- Read path is immutable snapshots (`ArcSwap`), write path is single-writer engine.
- Termlets are a first-class SDK surface for subprocess-like testing without abandoning pane semantics.

### Rust Example

```rust
pub trait MuxKernel {
    fn apply(&mut self, event: Event, ctx: CoreCtx) -> ApplyOutcome;
}

#[derive(Clone, Copy, Debug)]
pub struct CoreCtx {
    pub now_ms: i64,
    pub rand_u64: u64,
}
```

### Test Strategy

1. Determinism replay for identical event streams.
2. Interop smoke: stock tmux client attaches to TermForge.
3. Termlet smoke: spawn, send, wait, snapshot, kill.

### AGENTS.md Rules

- `RULE-1-01`: No OS IO in layer-0 crates.
- `RULE-1-02`: Compatibility claims require an anchor in Section 30.

---

## 2. North Star Acceptance Criteria

### Design Decisions

- Criteria split into Compatibility, Architecture, Product, Performance, Ecosystem.
- Termlet-specific criteria are now required release gates.

### Rust Example

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gate {
    C1WireAttach,
    C4IdentifyBurst,
    A1PureKernel,
    P7TermletRust,
    P8TermletPython,
    P9TermletNode,
    B10TermletSpawnLatency,
}
```

### Test Strategy

1. Gate matrix includes tmux protocol parity and Termlet API parity.
2. CI fails on any mandatory gate regression.

### AGENTS.md Rules

- `RULE-2-01`: New features must map to explicit gate IDs.
- `RULE-2-02`: Do not relax compatibility gates without written waiver.

---

## 3. High-Level Architecture

### Design Decisions

- Layer 0: pure kernel + parser + grid + query + CRDT.
- Layer 1: runtime, sockets, PTY, process control.
- Layer 2: `mux-api` plus new `mux-termlet` facade crate.
- Layer 3: ORM.
- Layer 4: bindings.

### Rust Example

```rust
pub struct ManagedMux {
    rt: Arc<RuntimeHandle>,
}

pub struct Termlet {
    pane_id: PaneId,
    backend: Arc<dyn PtyBackend>,
    state: Arc<TermletState>,
}
```

### Test Strategy

1. Layering tests: forbidden dependency edges via cargo-deny/custom script.
2. API tests: `Termlet` reachable without importing runtime internals.

### AGENTS.md Rules

- `RULE-3-01`: `mux-termlet` may depend on `mux-core`, `mux-grid`, `mux-pty`, `mux-api`; never bindings.
- `RULE-3-02`: Keep wiring unidirectional toward core.

---

## 4. Workspace Layout

### Design Decisions

- Add new crate: `crates/mux-termlet`.
- Add binding submodules: `bindings/python/src/termlet.rs`, `bindings/node/src/termlet.rs`.
- Add fixture helpers for pytest/vitest.

### Rust Example

```text
crates/
  mux-types/
  mux-core/
  mux-grid/
  mux-proto/
  mux-pty/
  mux-pty-fake/
  mux-api/
  mux-orm/
  mux-termlet/      # NEW
bindings/
  python/
  node/
```

### Test Strategy

1. Workspace lint verifies crate boundaries.
2. Binding packages expose `Termlet` symbols.

### AGENTS.md Rules

- `RULE-4-01`: Only add files under `notes/` during spec iterations.
- `RULE-4-02`: Keep crate names `mux-*` for grep consistency.

---

## 5. Layering Contract

### Design Decisions

- Layer 0 remains wasm-checkable and unsafe-free.
- `mux-termlet` is Layer 2 facade: ergonomic API, no protocol parser ownership.
- PTY backends implement trait object used by runtime and termlet.

### Rust Example

```rust
pub trait PtyBackend: Send + Sync {
    fn write(&self, pane: PaneId, bytes: &[u8]) -> Result<(), PtyError>;
    fn resize(&self, pane: PaneId, cols: u16, rows: u16) -> Result<(), PtyError>;
    fn kill(&self, pane: PaneId) -> Result<(), PtyError>;
}
```

### Test Strategy

1. `cargo check --target wasm32-unknown-unknown` for pure crates.
2. `unsafe` grep must only match `mux-os` and controlled FFI sites.

### AGENTS.md Rules

- `RULE-5-01`: Do not leak `tokio` types into pure crates.
- `RULE-5-02`: `mux-termlet` must not reimplement protocol decoding.

---

## 6. Entity Model

### Design Decisions

- Session/Window/Pane remain canonical runtime graph entities.
- Termlet is modeled as a facade over `PaneId` plus metadata.
- Optional `TermletId` exists for SDK registries; maps 1:1 to pane lifecycle.

### Rust Example

```rust
slotmap::new_key_type! {
    pub struct PaneId;
    pub struct TermletId;
}

pub struct TermletMeta {
    pub termlet_id: TermletId,
    pub pane_id: PaneId,
    pub name: Option<String>,
    pub mode: TermletMode,
}
```

### Test Strategy

1. Ensure destroying pane destroys mapped termlet.
2. Ensure no orphan `TermletId` in registry snapshots.

### AGENTS.md Rules

- `RULE-6-01`: Pane is source-of-truth lifecycle object.
- `RULE-6-02`: Never duplicate grid state in termlet metadata.

---

## 7. Event/Effect Engine

### Design Decisions

- New events: `TermletSpawn`, `TermletSendKeys`, `TermletResize`, `TermletKill`.
- Effects remain runtime-owned: spawn PTY, write PTY, signal process.
- Engine emits deterministic `TermletOutputAppended` based on parser updates.

### Rust Example

```rust
pub enum Event {
    TermletSpawn { cfg: TermletSpawnCfg },
    TermletSendKeys { pane: PaneId, bytes: Vec<u8> },
    TermletResize { pane: PaneId, cols: u16, rows: u16 },
    TermletKill { pane: PaneId },
}

pub enum Effect {
    SpawnPty { pane: PaneId, cmd: Vec<String>, env: Vec<(String, String)> },
    WritePty { pane: PaneId, bytes: Vec<u8> },
    ResizePty { pane: PaneId, cols: u16, rows: u16 },
    KillPty { pane: PaneId },
}
```

### Test Strategy

1. Reducer snapshot tests assert exact emitted effects.
2. Fuzz event ordering around resize+output races.

### AGENTS.md Rules

- `RULE-7-01`: Reducer never blocks.
- `RULE-7-02`: Effect emission must be idempotency-aware for retries.

---

## 8. Error Handling

### Design Decisions

- Unified typed errors with classes: user, transient, protocol, internal.
- Termlet adds explicit timeout and pattern-not-found errors.
- Binding layers preserve machine-readable error codes.

### Rust Example

```rust
#[derive(thiserror::Error, Debug)]
pub enum TermletError {
    #[error("spawn failed: {0}")]
    Spawn(String),
    #[error("timeout waiting for pattern")]
    Timeout,
    #[error("pattern not found before process exit")]
    PatternNotFound,
    #[error("pty backend error: {0}")]
    Pty(String),
}
```

### Test Strategy

1. Timeout tests with deterministic fake clock.
2. Exit-before-match tests produce `PatternNotFound`.

### AGENTS.md Rules

- `RULE-8-01`: No panics in decode, runtime loop, or binding boundary.
- `RULE-8-02`: Errors returned to bindings must include stable `code`.

---

## 9. Protocol Codec

### Design Decisions

- No protocol changes for Termlets; compatibility remains strict tmux v8.
- Termlet functionality is SDK-side composition over existing pane commands.
- Codec remains strict on malformed headers and payload lengths.

### Rust Example

```rust
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ImsgHdr {
    pub typ: u32,
    pub len: u32,
    pub peerid: u32,
    pub pid: u32,
}
```

### Test Strategy

1. Golden fixtures for identify burst 100-112.
2. Roundtrip property tests across all `MsgType` variants.

### AGENTS.md Rules

- `RULE-9-01`: Protocol violations close connection.
- `RULE-9-02`: Never add Termlet-specific wire frames.

---

## 10. Configuration System

### Design Decisions

- Config parser stays tmux-compatible for core options.
- Add optional TermForge namespace for SDK defaults: `termforge.termlet.*`.
- Defaults apply only when SDK caller omits explicit config.

### Rust Example

```rust
pub struct TermletDefaults {
    pub cols: u16,
    pub rows: u16,
    pub default_shell: String,
    pub wait_poll_ms: u64,
}
```

### Test Strategy

1. Parse compatibility tests against tmux config corpus.
2. Termlet defaults precedence tests: explicit > project > user > built-in.

### AGENTS.md Rules

- `RULE-10-01`: Never let new config keys break tmux parsing.
- `RULE-10-02`: Keep `termforge.*` keys isolated from tmux namespace.

---

## 11. Layout Engine

### Design Decisions

- Preserve tmux layout semantics and checksum algorithm.
- Termlet resize is pane resize with same round-robin behavior in split contexts.
- Standalone termlet windows use direct grid resize.

### Rust Example

```rust
pub fn resize_termlet(graph: &mut ServerGraph, pane: PaneId, cols: u16, rows: u16) {
    graph.resize_pane_tmux_compatible(pane, cols, rows);
}
```

### Test Strategy

1. Verify layout checksum parity vs tmux strings.
2. Verify `tput cols` reflects `Termlet::resize` target.

### AGENTS.md Rules

- `RULE-11-01`: Keep round-robin semantics for shared layouts.
- `RULE-11-02`: Never shortcut through non-parity resizing logic.

---

## 12. ORM-like Query API

### Design Decisions

- Preserve 12+ operator compatibility (`exact`, `contains`, `nin`, `iregex`, etc.).
- Support kwargs translation for Python and object-map filters for Node.
- Termlet handles are not ORM entities by default; panes remain query root.

### Rust Example

```rust
pub enum QueryOp {
    Exact, IExact, Contains, IContains, StartsWith, IStartsWith,
    EndsWith, IEndsWith, In, Nin, Regex, IRegex,
}
```

### Test Strategy

1. Parity tests against libtmux operator behavior.
2. Nested `__` field traversal tests.

### AGENTS.md Rules

- `RULE-12-01`: Add operators only with binding parity tests.
- `RULE-12-02`: Keep Rust query engine as single implementation source.

---

## 13. Runtime Architecture

### Design Decisions

- Single writer actor owns graph mutation.
- PTY reader tasks feed parser and emit output events.
- Termlet waits subscribe to snapshot/output stream; no busy spinning on locks.

### Rust Example

```rust
pub struct RuntimeHandle {
    tx: tokio::sync::mpsc::Sender<Event>,
    snapshots: tokio::sync::watch::Receiver<Arc<Snapshot>>,
}
```

### Test Strategy

1. Backpressure test on high output throughput.
2. Runtime shutdown test with active termlets.

### AGENTS.md Rules

- `RULE-13-01`: Runtime tasks must be cancellable.
- `RULE-13-02`: No direct graph writes outside writer actor.

---

## 14. Server Lifecycle

### Design Decisions

- Bootstrap: lock socket, bind, protocol handshake, post-identify config load.
- Maintain termlet registry inside server context.
- On shutdown, kill termlet-backed panes cleanly with timeout fallback.

### Rust Example

```rust
pub async fn shutdown(server: &Server) -> Result<(), ShutdownError> {
    server.termlets.kill_all(Duration::from_secs(2)).await?;
    server.runtime.stop().await
}
```

### Test Strategy

1. Simulate abrupt client disconnect during termlet activity.
2. Ensure no zombie PTYs after shutdown.

### AGENTS.md Rules

- `RULE-14-01`: Shutdown path must be idempotent.
- `RULE-14-02`: Termlet cleanup errors are reported, not ignored.

---

## 15. Control Mode

### Design Decisions

- Control mode parser remains tmux-compatible.
- Termlet operations can be exposed as client-side helpers that emit existing commands.
- Optional internal notifications can be prefixed and never sent to tmux clients.

### Rust Example

```rust
pub enum ControlNotification {
    TmuxLine(String),
    InternalTermlet { pane: PaneId, event: String },
}
```

### Test Strategy

1. Parse `%output` and `%exit` parity tests.
2. Ensure internal notifications are filtered on wire paths.

### AGENTS.md Rules

- `RULE-15-01`: Do not alter tmux control grammar.
- `RULE-15-02`: Internal events must remain opt-in.

---

## 16. Language Bindings

### Design Decisions

- Rust API is source-of-truth.
- Python (PyO3) and Node (Neon) expose equivalent feature surface.
- Bindings avoid reimplementing matching/wait logic.

### Rust Example

```rust
pub struct BindingFacade {
    api: ManagedMux,
}

impl BindingFacade {
    pub async fn termlet_spawn(&self, req: TermletSpawnRequest) -> Result<TermletHandle, TermletError> {
        self.api.termlet_spawn(req).await
    }
}
```

### Test Strategy

1. Cross-language contract tests on spawn/send/wait/snapshot/resize/kill.
2. Error code parity tests.

### AGENTS.md Rules

- `RULE-16-01`: Bindings must call Rust core for business logic.
- `RULE-16-02`: API names may be idiomatic per language, semantics must match.

---

## 17. CRDT Transaction Layer

### Design Decisions

- CRDT covers durable collaborative entities (sessions/windows/options/buffers).
- Termlets are ephemeral runtime constructs and excluded from replicated state.
- Termlet actions may emit audit events, not CRDT ops.

### Rust Example

```rust
pub enum ReplicatedEntity {
    Session(SessionId),
    Window(WindowId),
    Pane(PaneId),
}

pub fn is_replicated_termlet(_: TermletId) -> bool { false }
```

### Test Strategy

1. Merge tests verify no termlet IDs appear in op logs.
2. Concurrent window/pane edits merge without termlet leakage.

### AGENTS.md Rules

- `RULE-17-01`: Keep CRDT schema minimal and durable-only.
- `RULE-17-02`: Never require termlet state for convergence.

---

## 18. Security Model

### Design Decisions

- Enforce socket path safety and ownership checks.
- Termlet spawn environment defaults to safe baseline and explicit allowlist overrides.
- Optional process sandbox hooks in runtime layer.

### Rust Example

```rust
pub struct SpawnPolicy {
    pub allowed_bins: Vec<String>,
    pub env_allowlist: Vec<String>,
    pub cwd_allow_prefixes: Vec<PathBuf>,
}
```

### Test Strategy

1. Reject invalid socket names and unsafe paths.
2. Reject disallowed termlet command binaries.

### AGENTS.md Rules

- `RULE-18-01`: Never trust binding input for spawn policy.
- `RULE-18-02`: Security checks must run before PTY creation.

---

## 19. OpenTelemetry

### Design Decisions

- Keep dual provider architecture (global + mux-client provider).
- Trace termlet lifecycle spans: `termlet.spawn`, `termlet.send_keys`, `termlet.wait_for`, `termlet.snapshot`, `termlet.kill`.
- Carry trace context through bindings.

### Rust Example

```rust
pub fn in_termlet_span<T>(name: &'static str, f: impl FnOnce() -> T) -> T {
    let span = tracing::info_span!("termlet", op = name);
    let _guard = span.enter();
    f()
}
```

### Test Strategy

1. Span existence tests in Rust/Python/Node flows.
2. Force-flush tests for clean process shutdown.

### AGENTS.md Rules

- `RULE-19-01`: OTEL must never block hot paths on exporter latency.
- `RULE-19-02`: Keep low-cardinality span attributes.

---

## 20. tmux Version Management

### Design Decisions

- Keep BLAKE3 cache keys and lock-safe builder workflow.
- Maintain compatibility matrix across supported tmux versions.
- Run termlet integration suites against at least two tmux builds.

### Rust Example

```rust
pub fn compute_cache_key(version: &str, host: &str, flags_fp: &str) -> String {
    format!("tmux-{version}__{host}__cfg{flags_fp}")
}
```

### Test Strategy

1. Matrix build tests for selected tmux versions.
2. Validate termlet smoke works regardless of backend tmux version.

### AGENTS.md Rules

- `RULE-20-01`: All CI tmux artifacts must be content-addressed.
- `RULE-20-02`: File locks required around shared cache writes.

---

## 21. Test Support and Fake PTY

### Design Decisions

- Keep deterministic fake PTY scenario recorder/replayer.
- Provide common helpers for socket isolation and tempdir policy.
- Termlet test mode defaults to fake backend unless explicitly live.

### Rust Example

```rust
pub enum TermletBackendMode {
    Fake,
    Real,
}

pub struct ScenarioStep {
    pub in_bytes: Vec<u8>,
    pub out_bytes: Vec<u8>,
}
```

### Test Strategy

1. Replay equality tests: same script => same grid snapshots.
2. Mixed mode tests: fake unit + real integration.

### AGENTS.md Rules

- `RULE-21-01`: Unit tests never depend on local shell behavior.
- `RULE-21-02`: Fake scenarios must be versioned fixtures.

---

## 22. Termlets (SDK-first Testing Pods)

### 22.1 Design Goals

- Provide a minimal, fast, ergonomic subprocess-like API.
- Preserve tmux compatibility by implementing on top of pane+grid+pty.
- Offer deterministic and live execution modes.
- Make snapshot testing first-class in Rust, Python, and Node.

### 22.2 Relationship to Existing Types

Termlet is a facade over existing runtime objects:
- **Pane**: process attachment and lifecycle anchor
- **Grid**: visual buffer used for snapshotting and `wait_for`
- **PtyBackend**: real or fake IO/control backend

```rust
pub struct Termlet {
    pane_id: PaneId,
    runtime: Arc<RuntimeHandle>,
    backend: Arc<dyn PtyBackend>,
    matcher: PatternMatcher,
    defaults: TermletDefaults,
}
```

### 22.3 Rust API Design

```rust
#[derive(Debug, Clone)]
pub struct TermletConfig {
    pub cols: u16,
    pub rows: u16,
    pub env: Vec<(String, String)>,
    pub cwd: Option<std::path::PathBuf>,
    pub backend_mode: TermletBackendMode,
    pub wait_poll_interval: std::time::Duration,
}

impl Default for TermletConfig {
    fn default() -> Self {
        Self {
            cols: 80,
            rows: 24,
            env: vec![("TERM".into(), "xterm-256color".into())],
            cwd: None,
            backend_mode: TermletBackendMode::Fake,
            wait_poll_interval: std::time::Duration::from_millis(25),
        }
    }
}

impl Termlet {
    pub async fn spawn<S: AsRef<str>>(cmd: S, cfg: TermletConfig) -> Result<Self, TermletError>;
    pub async fn send_keys<S: AsRef<str>>(&self, keys: S) -> Result<(), TermletError>;
    pub async fn wait_for<S: AsRef<str>>(&self, pattern: S, timeout: std::time::Duration) -> Result<WaitMatch, TermletError>;
    pub fn snapshot(&self) -> GridSnapshot;
    pub async fn resize(&self, cols: u16, rows: u16) -> Result<(), TermletError>;
    pub async fn is_alive(&self) -> Result<bool, TermletError>;
    pub async fn kill(&self) -> Result<(), TermletError>;
}

impl Drop for Termlet {
    fn drop(&mut self) {
        // Best-effort cleanup; explicit kill() remains preferred.
    }
}
```

### 22.4 FakePty vs Real PTY Modes

- `Fake`: deterministic scenario-driven backend; no OS shell dependency; ideal for unit tests.
- `Real`: uses real PTY and shell/process; ideal for integration and parity.

```rust
pub fn select_backend(mode: TermletBackendMode, rt: &RuntimeHandle) -> Arc<dyn PtyBackend> {
    match mode {
        TermletBackendMode::Fake => Arc::new(rt.fake_pty_backend()),
        TermletBackendMode::Real => Arc::new(rt.real_pty_backend()),
    }
}
```

### 22.5 `wait_for` Implementation (polling + pattern matching)

- Poll snapshot stream at configured interval.
- Compile matcher once (`plain`, `regex`, case-insensitive options).
- Return as soon as pattern appears in current visible grid text.
- If process exits before match, return `PatternNotFound`.
- If timeout expires, return `Timeout`.

```rust
pub async fn wait_for<S: AsRef<str>>(
    &self,
    pattern: S,
    timeout: Duration,
) -> Result<WaitMatch, TermletError> {
    let deadline = tokio::time::Instant::now() + timeout;
    let compiled = self.matcher.compile(pattern.as_ref())?;

    loop {
        let snap = self.snapshot();
        if let Some(m) = compiled.find(&snap.to_text()) {
            return Ok(WaitMatch { byte_range: m, at: std::time::SystemTime::now() });
        }

        if !self.is_alive().await? {
            return Err(TermletError::PatternNotFound);
        }

        let now = tokio::time::Instant::now();
        if now >= deadline {
            return Err(TermletError::Timeout);
        }

        let sleep_for = std::cmp::min(self.defaults.wait_poll_interval, deadline - now);
        tokio::time::sleep(sleep_for).await;
    }
}
```

### 22.6 Snapshot Capture and Comparison

- `GridSnapshot` is immutable capture of current pane grid.
- Exporters: `to_text()`, `to_ansi()`, structured cells.
- Rust uses `insta` snapshots; Python/Node expose text for framework-native snapshots.

```rust
#[derive(Clone, Debug)]
pub struct GridSnapshot {
    pub cols: u16,
    pub rows: u16,
    pub cells: Vec<Cell>,
}

impl GridSnapshot {
    pub fn to_text(&self) -> String { /* render rows */ String::new() }
}

#[test]
fn termlet_snapshot_insta() {
    let snap = GridSnapshot { cols: 80, rows: 24, cells: vec![] };
    insta::assert_snapshot!(snap.to_text());
}
```

### 22.7 Python Binding API (`PyTermlet`)

```rust
#[pyclass(name = "Termlet")]
pub struct PyTermlet {
    inner: Arc<Termlet>,
}

#[pymethods]
impl PyTermlet {
    #[staticmethod]
    fn spawn(cmd: String, cols: Option<u16>, rows: Option<u16>) -> PyResult<Self> { /* ... */ }
    fn send_keys(&self, keys: String) -> PyResult<()> { /* ... */ }
    fn wait_for(&self, pattern: String, timeout: f64) -> PyResult<()> { /* ... */ }
    fn resize(&self, cols: u16, rows: u16) -> PyResult<()> { /* ... */ }
    fn is_alive(&self) -> PyResult<bool> { /* ... */ }
    fn snapshot(&self) -> PyResult<PyGridSnapshot> { /* ... */ }
    fn kill(&self) -> PyResult<()> { /* ... */ }
}
```

Python user API target:

```python
with Termlet.spawn("bash", cols=80, rows=24) as t:
    t.send_keys("echo hello\n")
    t.wait_for("hello", timeout=5)
    assert "hello" in t.snapshot().text()
```

### 22.8 Node Binding API (`JsTermlet`)

```rust
pub struct JsTermlet {
    inner: Arc<Termlet>,
}

impl JsTermlet {
    pub async fn spawn(cmd: String, opts: JsTermletOpts) -> Result<Self, JsError>;
    pub async fn send_keys(&self, keys: String) -> Result<(), JsError>;
    pub async fn wait_for(&self, pattern: String, timeout_ms: u64) -> Result<(), JsError>;
    pub async fn resize(&self, cols: u16, rows: u16) -> Result<(), JsError>;
    pub fn snapshot(&self) -> JsGridSnapshot;
    pub async fn is_alive(&self) -> Result<bool, JsError>;
    pub async fn kill(&self) -> Result<(), JsError>;
}
```

Node user API target:

```javascript
const t = await Termlet.spawn('bash', { cols: 80, rows: 24 });
await t.sendKeys('echo hello\n');
await t.waitFor('hello', { timeout: 5000 });
expect(t.snapshot().text()).toContain('hello');
await t.kill();
```

### 22.9 Fixture Design (pytest + vitest)

- `pytest` plugin provides `termlet` fixture with context-managed cleanup.
- `vitest` helper provides `useTermlet()` that auto-kills in `finally`.
- Both support fake/live backend modes.

```python
@pytest.fixture
def termlet():
    with Termlet.spawn("bash", backend="fake") as t:
        yield t
```

```javascript
export async function useTermlet(opts = {}) {
  const t = await Termlet.spawn(opts.cmd ?? 'bash', opts);
  return {
    termlet: t,
    async close() { await t.kill(); },
  };
}
```

### 22.10 Error Handling and Timeout Semantics

- `timeout <= 0` is immediate check (single snapshot pass).
- `wait_for` timeout includes polling overhead.
- Distinguish `Timeout` vs `PatternNotFound` (process ended first).
- Binding exceptions map to stable codes:
  - `TERMLET_TIMEOUT`
  - `TERMLET_PATTERN_NOT_FOUND`
  - `TERMLET_PTY_ERROR`
  - `TERMLET_SPAWN_ERROR`

### 22.11 Process Management Semantics

- `send_keys` writes raw bytes to pane PTY input.
- Background jobs (`&`) are shell semantics; termlet just transports IO.
- `is_alive` checks controlling process status.
- `kill` sends graceful terminate then forced kill on timeout.

```rust
pub async fn kill(&self) -> Result<(), TermletError> {
    self.runtime.terminate(self.pane_id).await?;
    if self.is_alive().await? {
        self.runtime.kill_force(self.pane_id).await?;
    }
    Ok(())
}
```

### 22.12 Termlet Test Strategy

1. Spawn/send/wait/snapshot/resize/kill happy path in fake and real modes.
2. Timeout and process-exit race coverage.
3. Snapshot stability checks across Rust/Python/Node wrappers.
4. Fixture cleanup guarantees under failure paths.
5. Concurrency: many short-lived termlets in parallel.

### 22.13 AGENTS.md Rules

- `RULE-22-01`: Termlet remains pane-backed; never invent parallel terminal core.
- `RULE-22-02`: `wait_for` must be deterministic in fake mode.
- `RULE-22-03`: Keep Rust/Python/Node API semantics aligned.
- `RULE-22-04`: Snapshot output must be stable and whitespace-preserving.

---

## 23. Binding Test Frameworks

### Design Decisions

- Provide first-class pytest and vitest plugins.
- Include backend parameterization: fake/live.
- Include snapshot adapters for each framework.

### Rust Example

```rust
pub struct BindingTestConfig {
    pub backend: TermletBackendMode,
    pub timeout_ms: u64,
}
```

### Test Strategy

1. Pytest matrix on Linux/macOS with fake/live.
2. Vitest matrix with Node LTS versions.

### AGENTS.md Rules

- `RULE-23-01`: Bindings must run against same Rust core artifact.
- `RULE-23-02`: Any fixture API changes require migration notes.

---

## 24. Test Framework and Harness Design

### Design Decisions

- Three-tier harness: unit (pure), integration (runtime), compatibility (real tmux).
- Add termlet cross-language snapshot corpus.
- Include flaky-test quarantine and replay artifacts.

### Rust Example

```rust
pub enum HarnessTier {
    Unit,
    Integration,
    Compatibility,
}
```

### Test Strategy

1. Keep deterministic unit suite always-on in PR.
2. Run compatibility matrix nightly and on release branches.

### AGENTS.md Rules

- `RULE-24-01`: Every bugfix must include a regression test.
- `RULE-24-02`: Flaky tests must emit replay logs before quarantine.

---

## 25. Performance Targets

### Design Decisions

- Retain v9 targets and add termlet SLOs.
- Focus on spawn latency and wait loop efficiency.

### Rust Example

```rust
pub struct PerfTargets {
    pub vt_parse_mb_s: u32,
    pub protocol_decode_mb_s: u32,
    pub termlet_spawn_ms_p50: u32,
    pub termlet_wait_poll_us: u32,
}
```

### Test Strategy

1. Criterion benchmarks for core parsers.
2. Termlet benchmarks in fake and real modes.

### AGENTS.md Rules

- `RULE-25-01`: Performance regressions >30% block release.
- `RULE-25-02`: Benchmark scripts must pin CPU governor where possible.

---

## 26. Visual Client / TUI

### Design Decisions

- ratatui remains main visual client.
- Add optional Termlet inspector panel (grid + process status) for debug mode.
- UI remains consumer of snapshots; never mutable state owner.

### Rust Example

```rust
pub struct TermletViewModel {
    pub pane_id: PaneId,
    pub alive: bool,
    pub grid_preview: String,
}
```

### Test Strategy

1. Golden render tests for inspector panel.
2. Keyboard navigation tests with synthetic events.

### AGENTS.md Rules

- `RULE-26-01`: TUI must not bypass runtime APIs.
- `RULE-26-02`: Debug panels are non-blocking and optional.

---

## 27. AGENTS.md Rules

### Design Decisions

- Keep actionable engineering rules in numbered format.
- Add explicit Termlet rules across architecture, testing, bindings.

### Rust Example

```rust
pub struct AgentRule {
    pub id: &'static str,
    pub text: &'static str,
}
```

### Test Strategy

1. Lint-like docs checks for required rule IDs.
2. Review checklist includes rule references in PR template.

### AGENTS.md Rules

- `RULE-27-01`: Cite rule IDs in architecture-impacting PRs.
- `RULE-27-02`: Rule updates require changelog entry.

---

## 28. Risks and Mitigations

### Design Decisions

- Primary risks: protocol drift, parser edge cases, binding drift, flaky live PTY tests.
- New risk: Termlet API semantics diverge across languages.

### Rust Example

```rust
pub enum Risk {
    ProtocolDrift,
    BindingSemanticDrift,
    TermletFlakiness,
}
```

### Test Strategy

1. Track risk owners and mitigations in CI dashboard.
2. Add canary tests for high-risk categories.

### AGENTS.md Rules

- `RULE-28-01`: Every high risk needs owner + date + mitigation.
- `RULE-28-02`: Unowned high risks block release.

---

## 29. Plan Evolution and Changelog

### Design Decisions

- Preserve v9 structure and add Section 22 for Termlets.
- Renumber prior sections 22-31 to 23-32.
- Keep compatibility commitments unchanged.

### Rust Example

```rust
pub struct ChangelogEntry {
    pub version: &'static str,
    pub highlights: Vec<&'static str>,
}
```

### Test Strategy

1. Validate section count and numbering in docs CI.
2. Validate every section has test strategy and AGENTS rules blocks.

### AGENTS.md Rules

- `RULE-29-01`: Structural doc changes require migration summary.
- `RULE-29-02`: Numbered section drift is CI-failing.

---

## 30. Reference Anchors

### Design Decisions

- All compatibility-sensitive decisions map to concrete upstream references.
- Keep anchors short, stable, and grep-friendly.

### Rust Example

```rust
pub struct Anchor {
    pub id: &'static str,
    pub path: &'static str,
    pub note: &'static str,
}
```

### Test Strategy

1. Anchor checker verifies file paths exist in configured reference repos.
2. Spot-check line drift during release prep.

### AGENTS.md Rules

- `RULE-30-01`: Do not merge parity claims without anchor IDs.
- `RULE-30-02`: If line numbers drift, refresh anchor notes.

### Anchor Table (Core)

| Anchor | File | Note |
|---|---|---|
| A-PROTO-8 | `~/study/c/tmux/tmux-protocol.h` | Protocol version constant |
| A-FLOCK | `~/study/c/tmux/client.c` | socket lock behavior |
| A-LAYOUT-RR | `~/study/c/tmux/layout.c` | round-robin resize |
| A-OPT-FALLTHROUGH | `~/study/c/tmux/options.c` | option scope fallback |
| A-QUERY-OPS | `~/work/python/libtmux/src/libtmux/_internal/query_list.py` | filter operators |
| A-PATH-GUARD | `~/work/rust/vibe-tmux/crates/mux-test-support/src/path_guard.rs` | socket safety checks |
| A-BUILDER-CACHE | `~/work/rust/vibe-tmux/tools/tmux-builder/src/lib.rs` | BLAKE3 cache key + lock |
| A-OTEL-DUAL | `~/work/rust/vibe-tmux/crates/mux-otel/src/otel.rs` | dual provider model |

---

## 31. Appendix: Canonical Type Quick Reference

### Design Decisions

- Keep a compact canonical index for frequently used architecture types.
- Add termlet-facing canonical types.

### Rust Example

```rust
pub type GridText = String;

pub struct WaitMatch {
    pub byte_range: std::ops::Range<usize>,
    pub at: std::time::SystemTime,
}
```

### Test Strategy

1. Doc examples compile-test under `cargo test --doc`.
2. Public API diff checks for semver awareness.

### AGENTS.md Rules

- `RULE-31-01`: Appendix types must mirror real crate names.
- `RULE-31-02`: Remove stale aliases during major revisions.

### Canonical Index

- `SessionId`, `WindowId`, `PaneId`, `ClientId`, `TermletId`
- `Event`, `Effect`, `ApplyOutcome`, `CoreCtx`
- `Grid`, `GridSnapshot`, `Cell`, `Style`
- `Termlet`, `TermletConfig`, `TermletBackendMode`, `WaitMatch`
- `QueryOp`, `QuerySpec`, `QueryList<T>`
- `OtelProvider`, `TraceHeaders`, `MuxClientSpanGuard`

---

## 32. Supplemental Test Matrix

### Design Decisions

- Matrix covers unit/integration/compatibility across Rust/Python/Node.
- Includes fake/live termlet modes and tmux-version axes.

### Rust Example

```rust
pub struct MatrixRow {
    pub suite: &'static str,
    pub backend: &'static str,
    pub language: &'static str,
    pub required: bool,
}
```

### Test Strategy

1. PR required rows: Rust unit fake, Rust integration fake, Python fake, Node fake.
2. Nightly required rows: all live PTY rows + multi-version tmux compatibility.

### AGENTS.md Rules

- `RULE-32-01`: Required rows cannot be skipped without waiver.
- `RULE-32-02`: Nightly failures in compatibility rows create release blockers.

### Matrix

| Suite | Backend | Language | Cadence | Required |
|---|---|---|---|---|
| Core reducer property tests | fake | Rust | PR | Yes |
| Protocol fixture roundtrip | fake | Rust | PR | Yes |
| Termlet API smoke | fake | Rust | PR | Yes |
| Termlet snapshot assertions | fake | Rust | PR | Yes |
| ORM parity operators | fake | Rust/Python | PR | Yes |
| PyTermlet fixture tests | fake | Python | PR | Yes |
| JsTermlet fixture tests | fake | Node | PR | Yes |
| Termlet live shell smoke | real | Rust | Nightly | Yes |
| PyTermlet live shell smoke | real | Python | Nightly | Yes |
| JsTermlet live shell smoke | real | Node | Nightly | Yes |
| tmux attach interop (C1/C2) | real | Rust CLI | Nightly | Yes |
| Multi-version tmux compatibility | real | Rust | Nightly | Yes |

---

## v10 Final Consistency Checklist

- 32 numbered sections present (v9 31 + new Termlet section).
- Each section includes design decisions, Rust example, test strategy, and AGENTS.md rules.
- Termlet section includes Rust/Python/Node APIs, fake/live backends, `wait_for`, snapshots, fixtures, errors, and Pane/Grid/Pty relationship.
- Wire protocol compatibility commitments preserved.

