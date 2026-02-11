# TermForge v12 Architecture Specification -- Pass 2 (Synthesized)

Date: 2026-02-11
Status: DRAFT -- v12 Pass 2 of 3
Lineage: v11 Pass 3 -> v12 Pass 1 (Claude/GPT5/Gemini) -> v12 Pass 2 (this document)
Rust edition: 2024 (MSRV 1.85)
License: MIT OR Apache-2.0
Protocol target: tmux protocol v8

## 1. Project Identity
### Design Decisions
- [v12 P2] Product identity is fixed: TermForge, binary `termforge`, alias `tf`, crate prefix `mux-`.
- [v12 P2] Compatibility claims are narrowed to measurable protocol lanes (defined in Section 20).
- [v12 P2] Project scope includes server, SDK, bindings, CRDT (feature-gated), and Termlets.

### Rust Example
```rust
pub const PROJECT_NAME: &str = "TermForge";
pub const BINARY_NAME: &str = "termforge";
pub const BINARY_ALIAS: &str = "tf";
pub const CRATE_PREFIX: &str = "mux-";
```

### Test Strategy
- Unit checks for constants and alias behavior.
- Package metadata check for dual-license conformance.
- CLI smoke test verifies `termforge --version` and `tf --version` parity.

### AGENTS.md Rules
- `RULE-S01-01`: Crates must use `mux-` prefix. Enforcement: CI metadata lint.
- `RULE-S01-02`: Binary names are `termforge` and `tf` only. Enforcement: release artifact name check.

## 2. Acceptance Criteria and Gates
### Design Decisions
- [v12 P2] Gate classes are `compat`, `correctness`, `performance`, `operability`.
- [v12 P2] Compatibility and correctness gates are hard-blocking for release.
- [v12 P2] Each gate must map to explicit tests and thresholds.

### Rust Example
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateClass { Compat, Correctness, Performance, Operability }

#[derive(Debug, Clone)]
pub struct GateResult {
    pub class: GateClass,
    pub pass: bool,
}

pub fn all_required_pass(results: &[GateResult]) -> bool {
    results.iter().all(|r| r.pass || matches!(r.class, GateClass::Performance))
}
```

### Test Strategy
- Gate manifest test ensures no unmapped gate IDs.
- Release pipeline validates hard-block classes pass on all supported lanes.
- Negative CI test intentionally injects one failed compatibility gate.

### AGENTS.md Rules
- `RULE-S02-01`: Every feature maps to one or more gates. Enforcement: gate-map linter.
- `RULE-S02-02`: Compat/correctness failures cannot be `allow_failure`. Enforcement: CI policy script.

## 3. Crate Dependency Rules
### Design Decisions
- [v12 P2] `mux-core` is Layer 0 (pure) and cannot depend on OS/runtime crates.
- [v12 P2] `mux-termlet` is Layer 2 facade and cannot depend on server/API internals.
- [v12 P2] CRDT remains optional via feature flag.

### Rust Example
```rust
#[derive(Debug, Clone)]
pub struct DepRule {
    pub from: &'static str,
    pub forbid: &'static [&'static str],
}

pub const TERMLET_FORBID: DepRule = DepRule {
    from: "mux-termlet",
    forbid: &["mux-server", "mux-api", "mux-orm"],
};
```

### Test Strategy
- Dependency graph check (`cargo tree` + custom edge checker).
- WASM compile check for all Layer 0 crates.
- Feature matrix compile with and without `crdt`.

### AGENTS.md Rules
- `RULE-S03-01`: Layer violations are merge blockers. Enforcement: dependency graph CI.
- `RULE-S03-02`: `mux-crdt` must never be a mandatory dependency. Enforcement: feature-gate compile matrix.

## 4. Workspace Layout
### Design Decisions
- [v12 P2] Workspace uses stable top-level groups: `crates/`, `bindings/`, `tools/`, `tests/`, `examples/`, `docs/`.
- [v12 P2] Generated artifacts live under `generated/` and are tool-owned.
- [v12 P2] `examples/termlet-recipes/` is required to keep API usage executable.

### Rust Example
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceGroup { Crates, Bindings, Tools, Tests, Examples, Docs, Generated }
```

### Test Strategy
- Repository shape test ensures required directories exist.
- Generated-file mutation guard test.
- `cargo check --examples` required in PR CI.

### AGENTS.md Rules
- `RULE-S04-01`: New top-level directories need architecture approval. Enforcement: path policy check.
- `RULE-S04-02`: All examples must compile. Enforcement: examples CI job.

## 5. mux-core: Pure Kernel
### Design Decisions
- [v12 P2] Reducer model is pure `(state, event, ctx) -> (state, effects)`.
- [v12 P2] No wall-clock, RNG, env, or I/O reads inside reducer code.
- [v12 P2] Replay hash must be stable across runs.

### Rust Example
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreState { pub sessions: u64 }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreEvent { NewSession }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect { PersistSnapshot }

pub fn apply_event(s: &CoreState, e: CoreEvent) -> (CoreState, Vec<Effect>) {
    match e {
        CoreEvent::NewSession => (CoreState { sessions: s.sessions + 1 }, vec![Effect::PersistSnapshot]),
    }
}
```

### Test Strategy
- Property tests for reducer determinism.
- Replay test compares final state hash for the same event stream.
- Static lint forbids disallowed imports in Layer 0 crates.

### AGENTS.md Rules
- `RULE-S05-01`: `mux-core` is side-effect free. Enforcement: banned-import lint + review gate.
- `RULE-S05-02`: Reducer changes require replay fixture updates. Enforcement: fixture-delta CI check.

## 6. Entity ID Design
### Design Decisions
- [v12 P2] Public APIs use opaque newtypes, not raw strings.
- [v12 P2] Server entities use slotmap IDs; Termlets use monotonic `TermletPaneId`.
- [v12 P2] `PtyHandle(u64)` decouples pty layer from termlet type definitions.

### Rust Example
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TermletPaneId(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PtyHandle(pub u64);

impl TermletPaneId {
    pub fn as_pty_handle(self) -> PtyHandle { PtyHandle(self.0) }
}
```

### Test Strategy
- Compile tests for opaque ID boundaries.
- Concurrency test for monotonic uniqueness.
- API lint ensures no `String`/`&str` raw IDs in public signatures.

### AGENTS.md Rules
- `RULE-S06-01`: Public API IDs must be opaque types. Enforcement: API signature lint.
- `RULE-S06-02`: Pty layer accepts `PtyHandle`, not `TermletPaneId`. Enforcement: signature test.

## 7. Event -> Effect Architecture
### Design Decisions
- [v12 P2] Events mutate state; effects perform side effects.
- [v12 P2] Effects include idempotency key and causation metadata.
- [v12 P2] Failed effects emit diagnostic events.

### Rust Example
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CausationId(pub u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event { pub causation: CausationId, pub kind: &'static str }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectIntent {
    pub causation: CausationId,
    pub idempotency_key: String,
    pub kind: &'static str,
}
```

### Test Strategy
- State machine tests ensure only events mutate state.
- Retry tests verify idempotency key suppresses duplicates.
- Failure injection test validates diagnostic-event emission.

### AGENTS.md Rules
- `RULE-S07-01`: Core emits intents only; adapters execute effects. Enforcement: crate-boundary lint.
- `RULE-S07-02`: New effect types must define idempotency derivation. Enforcement: trait-impl compile check.

## 8. Error Taxonomy
### Design Decisions
- [v12 P2] Library crates use `thiserror`; binaries/tests may use `anyhow`.
- [v12 P2] Error codes are stable and binding-safe.
- [v12 P2] Termlet errors distinguish timeout, pattern-not-found, wait-failed, and invalid-state.

### Rust Example
```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TermletError {
    #[error("[TF_TERMLET_TIMEOUT] pattern={pattern} timeout_ms={timeout_ms}")]
    Timeout { pattern: String, timeout_ms: u64 },
    #[error("[TF_TERMLET_NOT_FOUND] pattern={pattern}")]
    PatternNotFound { pattern: String },
    #[error("[TF_TERMLET_WAIT_FAILED] {reason}")]
    WaitFailed { reason: String },
    #[error("[TF_TERMLET_INVALID_STATE] {state}")]
    InvalidState { state: &'static str },
}
```

### Test Strategy
- Unit tests for each variant, message format, and stable code prefixes.
- Binding tests verify Python/Node exceptions preserve code prefix.
- Secret redaction tests for error payloads.

### AGENTS.md Rules
- `RULE-S08-01`: New error codes require docs + binding mappings. Enforcement: schema diff check.
- `RULE-S08-02`: Binding-visible messages must include `[CODE]` prefix. Enforcement: cross-language tests.

## 9. Protocol Codec (mux-proto)
### Design Decisions
- [v12 P2] Decoder is strict: malformed frames are fatal for that client.
- [v12 P2] Frame size bounds are explicit and configurable.
- [v12 P2] Protocol fixtures are lane-scoped by tmux version.

### Rust Example
```rust
pub const MAX_PAYLOAD_SIZE: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeOutcome {
    Message(Vec<u8>),
    NeedMore,
    Fatal(&'static str),
}

pub fn decode_frame(input: &[u8]) -> DecodeOutcome {
    if input.len() > MAX_PAYLOAD_SIZE {
        return DecodeOutcome::Fatal("payload_too_large");
    }
    DecodeOutcome::NeedMore
}
```

### Test Strategy
- Corpus tests for valid/invalid frame streams.
- Oversized payload tests verify immediate fatal result.
- Fixture diff tests against tmux lane captures.

### AGENTS.md Rules
- `RULE-S09-01`: Protocol violations terminate the client connection. Enforcement: integration test with malformed frames.
- `RULE-S09-02`: Payload size must be bounded. Enforcement: bounds test suite.

## 10. Config and Option System
### Design Decisions
- [v12 P2] Config load occurs after first client identify burst.
- [v12 P2] Option resolution uses deterministic precedence and explicit FALLTHROUGH behavior.
- [v12 P2] Schema changes require migration notes.

### Rust Example
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope { Pane, Window, Session, Server }

pub fn resolve_option(scopes: &[Scope]) -> Scope {
    // Simplified precedence: pane -> window -> session -> server.
    scopes.first().copied().unwrap_or(Scope::Server)
}
```

### Test Strategy
- Startup integration test verifies post-identify config loading.
- Property test for pane->window->session->server fallthrough resolution.
- Schema versioning tests for backward compatibility.

### AGENTS.md Rules
- `RULE-S10-01`: Config loads only after identify burst completion. Enforcement: startup-sequence test.
- `RULE-S10-02`: Option precedence changes require migration docs. Enforcement: docs+code parity check.

## 11. Layout Engine
### Design Decisions
- [v12 P2] Resize algorithm is deterministic and integer-cell based.
- [v12 P2] Behavior must be independent of hash iteration order.
- [v12 P2] Debug invariants are asserted in mutating paths.

### Rust Example
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaneRect { pub x: u16, pub y: u16, pub w: u16, pub h: u16 }

pub fn round_robin_shrink(widths: &mut [u16], mut delta: u16) {
    let mut i = 0usize;
    while delta > 0 && !widths.is_empty() {
        if widths[i] > 1 {
            widths[i] -= 1;
            delta -= 1;
        }
        i = (i + 1) % widths.len();
    }
}
```

### Test Strategy
- Deterministic replay tests for identical input layouts.
- Regression fixtures for nested split edge cases.
- Invariant assertions enabled in debug CI.

### AGENTS.md Rules
- `RULE-S11-01`: Layout math uses integer cell arithmetic only. Enforcement: lint disallowing float math.
- `RULE-S11-02`: Layout bug fixes require replay fixtures. Enforcement: bugfix policy check.

## 12. QueryList ORM Layer
### Design Decisions
- [v12 P2] `QueryList` is orthogonal and chain-safe across Rust/Python/Node.
- [v12 P2] `Queryable` and `QuerySpec` are normative types (fixing missing definitions).
- [v12 P2] Generic type safety includes explicit `PhantomData` where needed.

### Rust Example
```rust
use std::marker::PhantomData;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuerySpec {
    pub field: String,
    pub op: String,
    pub value: String,
}

pub trait Queryable {
    fn matches(&self, spec: &QuerySpec) -> bool;
}

pub struct QueryList<T> {
    items: Vec<T>,
    _marker: PhantomData<T>,
}
```

### Test Strategy
- Operator table tests for all supported ops.
- Chain-safety tests (`filter().exclude().order_by()...`) in all three language runtimes.
- Parser tests for `field__op=value` input forms.

### AGENTS.md Rules
- `RULE-S12-01`: Query APIs remain orthogonal and chain-safe. Enforcement: API contract tests.
- `RULE-S12-02`: `QuerySpec` parse semantics must be test-covered. Enforcement: parser unit tests.

## 13. State Actor and ArcSwap
### Design Decisions
- [v12 P2] Single actor is the sole mutable writer.
- [v12 P2] Readers consume immutable snapshots via `ArcSwap`.
- [v12 P2] Snapshot reads must never acquire mutable locks.

### Rust Example
```rust
use arc_swap::ArcSwap;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphState { pub rev: u64 }

pub struct StateHandle {
    inner: Arc<ArcSwap<GraphState>>,
}

impl StateHandle {
    pub fn snapshot(&self) -> Arc<GraphState> { self.inner.load_full() }
}
```

### Test Strategy
- Concurrency tests with frequent writes and parallel snapshot reads.
- Actor ownership compile tests (no mutable graph in read-only modules).
- Performance tests for read-path latency.

### AGENTS.md Rules
- `RULE-S13-01`: State mutation is actor-owned only. Enforcement: module visibility + lint.
- `RULE-S13-02`: Snapshot reads must be non-mutating. Enforcement: compile and clippy checks.

## 14. Server Lifecycle
### Design Decisions
- [v12 P2] Lifecycle transitions are explicit and monotonic.
- [v12 P2] Lock guard owns `File` handle (fixes leaked-fd pattern).
- [v12 P2] Shutdown deadlines are mandatory configuration.

### Rust Example
```rust
use std::fs::File;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerPhase { Booting, Ready, Draining, Stopped }

pub struct LockGuard { file: File }

impl Drop for LockGuard {
    fn drop(&mut self) {
        // File drop releases lock via OS semantics.
    }
}
```

### Test Strategy
- Transition table tests rejecting illegal phase jumps.
- FD leak tests around lock acquisition/release.
- Shutdown timeout integration tests.

### AGENTS.md Rules
- `RULE-S14-01`: Lifecycle transitions must be centralized and validated. Enforcement: transition API-only lint.
- `RULE-S14-02`: Lock guards must own file handles (no leaked lock fd). Enforcement: fd leak tests.

## 15. Control Mode
### Design Decisions
- [v12 P2] Control notifications are hints; authoritative state comes from refresh/query.
- [v12 P2] Frames carry monotonic sequence numbers.
- [v12 P2] Backpressure behavior follows tmux-compatible semantics.

### Rust Example
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlNotification {
    pub seq: u64,
    pub kind: String,
}

pub fn is_monotonic(prev: u64, next: u64) -> bool { next > prev }
```

### Test Strategy
- Sequence monotonicity tests under burst notifications.
- Backpressure tests for pause/resume and queue bounds.
- Compatibility tests against lane fixtures.

### AGENTS.md Rules
- `RULE-S15-01`: Control frames must include monotonic sequence IDs. Enforcement: integration assertions.
- `RULE-S15-02`: Backpressure semantics must match parity fixtures. Enforcement: control-mode parity tests.

## 16. Language Bindings
### Design Decisions
- [v12 P2] Bindings consume generated shared schema, not handwritten drifted structs.
- [v12 P2] Python and Node keep semantic ordering identical to Rust API.
- [v12 P2] Long-running Rust calls release host runtime locks appropriately.

### Rust Example
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingMethod {
    pub name: &'static str,
    pub stable_order: u32,
}

pub const CORE_BINDING_METHODS: &[BindingMethod] = &[
    BindingMethod { name: "spawn", stable_order: 1 },
    BindingMethod { name: "send_keys", stable_order: 2 },
    BindingMethod { name: "snapshot", stable_order: 3 },
];
```

### Test Strategy
- Schema hash checks between Rust/Python/Node generated artifacts.
- Cross-runtime contract tests for method names and error codes.
- Release matrix tests across supported OS lanes.

### AGENTS.md Rules
- `RULE-S16-01`: Bindings must be generated from shared schema. Enforcement: generated hash check.
- `RULE-S16-02`: Binding release requires parity suite pass on supported lanes. Enforcement: release workflow gate.

## 17. CRDT Layer
### Design Decisions
- [v12 P2] CRDT support is optional (`crdt` feature), but always tested in CI lanes.
- [v12 P2] Merge determinism is mandatory for same input op set regardless of order.
- [v12 P2] Conflict surfaces emit explicit review artifacts.

### Rust Example
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Lamport(pub u64);

pub fn next_clock(local: Lamport, observed: Lamport) -> Lamport {
    Lamport(local.0.max(observed.0) + 1)
}
```

### Test Strategy
- Property tests for convergence and commutativity constraints.
- Merge-order replay tests with deterministic final hash assertions.
- Partition/rejoin simulation tests with conflict artifact checks.

### AGENTS.md Rules
- `RULE-S17-01`: CRDT crates remain pure and deterministic. Enforcement: wasm + purity CI checks.
- `RULE-S17-02`: CRDT feature remains optional. Enforcement: build matrix with `--no-default-features`.

## 18. Socket and Permissions
### Design Decisions
- [v12 P2] Runtime sockets are created in private directories with strict permissions.
- [v12 P2] Paths are validated before creation and tested for cleanup.
- [v12 P2] Test harnesses never touch default tmux socket paths.

### Rust Example
```rust
use std::path::Path;

pub fn validate_socket_path(path: &Path) -> Result<(), &'static str> {
    let s = path.to_string_lossy();
    if s.is_empty() || s.len() > 100 { return Err("invalid_socket_path"); }
    Ok(())
}
```

### Test Strategy
- Permission tests for directory/socket mode bits.
- Runtime assert tests for non-default test socket usage.
- Leak checker tests verify socket file cleanup on success and failure.

### AGENTS.md Rules
- `RULE-S18-01`: Tests must not use default tmux socket path. Enforcement: harness runtime assert.
- `RULE-S18-02`: Socket artifacts must be removed post-test. Enforcement: filesystem leak checker.

## 19. Observability (OTEL)
### Design Decisions
- [v12 P2] Every RPC/command path is trace-instrumented.
- [v12 P2] Metric names are append-only unless explicitly deprecated with migration docs.
- [v12 P2] Termlet operations emit spans with correlation identifiers.

### Rust Example
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpanAttrs {
    pub op: &'static str,
    pub correlation_id: String,
}

pub fn required_attrs(op: &'static str, id: &str) -> SpanAttrs {
    SpanAttrs { op, correlation_id: id.to_string() }
}
```

### Test Strategy
- Instrumentation coverage tests for command handlers.
- Registry diff tests for metric-name compatibility.
- Span schema tests for required attributes on termlet operations.

### AGENTS.md Rules
- `RULE-S19-01`: New command paths require trace instrumentation. Enforcement: instrumentation coverage test.
- `RULE-S19-02`: Metric names are append-only unless migrated. Enforcement: metric-registry diff check.

## 20. tmux Builder and Version Manager
### Design Decisions
- [v12 P2] Compatibility claims are tied to explicit version lanes.
- [v12 P2] Build artifacts are content-addressed and atomically published.
- [v12 P2] Every compatibility claim must reference lane + version tuple.

### Rust Example
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatLane {
    pub lane: &'static str,
    pub tmux_version: &'static str,
}

pub const SUPPORTED_LANES: &[CompatLane] = &[
    CompatLane { lane: "L1", tmux_version: "3.2" },
    CompatLane { lane: "L2", tmux_version: "3.3" },
    CompatLane { lane: "L3", tmux_version: "3.4" },
];
```

### Test Strategy
- Lane matrix test ensures at least one lane run on every PR.
- Full lane sweep on nightly/release.
- Atomic publication test verifies no partial publish state.

### AGENTS.md Rules
- `RULE-S20-01`: Compatibility claims must include lane+version. Enforcement: docs claim linter.
- `RULE-S20-02`: Parity CI must run at least one version lane each PR. Enforcement: workflow policy.

## 21. Test Support and FakePty
### Design Decisions
- [v12 P2] Fake PTY is deterministic and corpus-driven.
- [v12 P2] Real and fake PTY behavior must stay aligned for supported scenarios.
- [v12 P2] Pty backend trait uses `PtyHandle` to avoid layer inversion.

### Rust Example
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PtyEvent {
    Output { handle: u64, data: Vec<u8> },
    Exited { handle: u64, status: i32 },
}

pub trait PtyBackend {
    fn read_events(&mut self) -> Vec<PtyEvent>;
}
```

### Test Strategy
- Corpus replay comparison fake vs real.
- Nonblocking read behavior tests for wait loop stability.
- API surface tests for handle type correctness.

### AGENTS.md Rules
- `RULE-S21-01`: Fake PTY changes require corpus parity diff vs real PTY. Enforcement: corpus comparison CI.
- `RULE-S21-02`: Pty backend APIs use `PtyHandle` and stay layer-safe. Enforcement: signature lint.

## 22. Parity Test Framework
### Design Decisions
- [v12 P2] Parity verdicts are explicit: `pass`, `waived`, `fail`.
- [v12 P2] Waivers require owner and expiry date.
- [v12 P2] Supported-lane failures block merge.

### Rust Example
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParityVerdict { Pass, Waived { owner: String, expiry: String }, Fail }

pub fn blocks_merge(v: &ParityVerdict, supported_lane: bool) -> bool {
    supported_lane && matches!(v, ParityVerdict::Fail)
}
```

### Test Strategy
- Verdict policy tests for fail/waive behavior.
- Waiver schema validation tests.
- Lane-specific block tests for supported matrix.

### AGENTS.md Rules
- `RULE-S22-01`: New waivers require owner+expiry. Enforcement: waiver schema validator.
- `RULE-S22-02`: `Fail` verdict on supported lane blocks merge. Enforcement: parity gate.

## 23. Fuzz Testing
### Design Decisions
- [v12 P2] Parser and codec crates require fuzz targets.
- [v12 P2] Termlet fuzz target exercises random byte and API sequences.
- [v12 P2] Fuzz findings must convert to minimized regression tests quickly.

### Rust Example
```rust
pub fn register_required_fuzz_targets() -> &'static [&'static str] {
    &["proto_decode", "vt_parser", "termlet_api_sequence"]
}
```

### Test Strategy
- Target registry test for required fuzz modules.
- Nightly fuzz runs with crash artifact persistence.
- Regression fixture test added for each fixed crash.

### AGENTS.md Rules
- `RULE-S23-01`: New parser modules require fuzz target registration. Enforcement: module-to-target mapping check.
- `RULE-S23-02`: Fuzz bugs must add minimized reproducer before closure. Enforcement: issue workflow bot + regression CI.

## 24. Performance Benchmarks
### Design Decisions
- [v12 P2] Benchmarks define `target` and `hard_fail` thresholds.
- [v12 P2] Re-baselining requires explicit metadata update.
- [v12 P2] Termlet predicate waits have dedicated overhead budget.

### Rust Example
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PerfTarget {
    pub target_ns: u64,
    pub hard_fail_ns: u64,
}

pub fn exceeds_hard_fail(observed_ns: u64, target: PerfTarget) -> bool {
    observed_ns > target.hard_fail_ns
}
```

### Test Strategy
- CI benchmark smoke checks for key hot paths.
- Weekly baseline run with drift report.
- Budget tests for termlet spawn, snapshot, wait predicate overhead.

### AGENTS.md Rules
- `RULE-S24-01`: Baseline changes require explicit revalidation metadata. Enforcement: benchmark metadata checker.
- `RULE-S24-02`: Hard-fail threshold breaches block release. Enforcement: release perf gate.

## 25. Visual Client / TUI
### Design Decisions
- [v12 P2] Render output is deterministic for fixed `(snapshot, viewport, theme)`.
- [v12 P2] View layer is read-only over state snapshots.
- [v12 P2] Input mapping changes require docs and tests.

### Rust Example
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderInput {
    pub snapshot_rev: u64,
    pub viewport: (u16, u16),
    pub theme: String,
}

pub fn render_key(input: &RenderInput) -> String {
    format!("{}:{}x{}:{}", input.snapshot_rev, input.viewport.0, input.viewport.1, input.theme)
}
```

### Test Strategy
- Snapshot stability tests for deterministic rendering.
- Read-only architecture checks in view modules.
- Input map registry tests for new key handling.

### AGENTS.md Rules
- `RULE-S25-01`: Render output must be deterministic for fixed inputs. Enforcement: snapshot stability tests.
- `RULE-S25-02`: View code must not mutate core graph state. Enforcement: lint for mutable graph access.

## 26. AGENTS.md Rules Registry
### Design Decisions
- [v12 P2] Rule IDs are canonicalized as `RULE-Snn-xx`.
- [v12 P2] Duplicate IDs are forbidden.
- [v12 P2] All rules require explicit enforcement text.

### Rust Example
```rust
use std::collections::HashSet;

pub fn validate_rule_ids(ids: &[&str]) -> bool {
    let mut seen = HashSet::new();
    ids.iter().all(|id| id.starts_with("RULE-S") && seen.insert(*id))
}
```

### Test Strategy
- Regex lint for naming format.
- Duplicate detection test across spec extraction.
- Enforcement-field presence test in each section block.

### AGENTS.md Rules
- `RULE-S26-01`: Rule IDs must match `RULE-Snn-xx`. Enforcement: regex linter.
- `RULE-S26-02`: Rule IDs must be unique and include enforcement. Enforcement: rule registry validator.

## 27. Risks and Mitigations
### Design Decisions
- [v12 P2] Risks are tracked with owner, threshold, and mitigation plan.
- [v12 P2] Major features require risk registration before merge.
- [v12 P2] Release gate validates unresolved high-risk items.

### Rust Example
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RiskItem {
    pub id: &'static str,
    pub owner: String,
    pub severity: u8,
    pub mitigated: bool,
}

pub fn blocks_release(item: &RiskItem) -> bool {
    item.severity >= 4 && !item.mitigated
}
```

### Test Strategy
- Risk schema validation tests.
- Pipeline check for missing owner/severity fields.
- Release preflight test for unresolved high severity risks.

### AGENTS.md Rules
- `RULE-S27-01`: Major features must register risk items before merge. Enforcement: feature metadata lint.
- `RULE-S27-02`: High-risk thresholds require explicit owner and mitigation. Enforcement: release checklist validator.

## 28. Plan Evolution and Changelog
### Design Decisions
- [v12 P2] Breaking architecture changes require ADR and changelog entries.
- [v12 P2] Release notes must include gate impact tags.
- [v12 P2] Spec evolution remains append-only with dated passes.

### Rust Example
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeEntry {
    pub date: &'static str,
    pub summary: String,
    pub breaking: bool,
    pub adr_ref: Option<String>,
}
```

### Test Strategy
- Changelog lint for required fields.
- ADR-link existence checks for breaking changes.
- Release notes parser checks gate impact tags.

### AGENTS.md Rules
- `RULE-S28-01`: Breaking architecture changes require ADR + changelog. Enforcement: PR policy check.
- `RULE-S28-02`: Release notes must include gate-impact tags. Enforcement: changelog linter.

## 29. Reference Anchors
### Design Decisions
- [v12 P2] Section references use stable `Snn` anchor semantics.
- [v12 P2] Cross-references must be link-check clean.
- [v12 P2] External source anchors are lane/version scoped.

### Rust Example
```rust
pub fn section_anchor(n: u8) -> String {
    format!("S{:02}", n)
}

pub fn is_valid_anchor(s: &str) -> bool {
    s.len() == 3 && s.starts_with('S') && s[1..].chars().all(|c| c.is_ascii_digit())
}
```

### Test Strategy
- Anchor format unit tests.
- Cross-reference integrity checks.
- Broken-link CI gate for spec docs.

### AGENTS.md Rules
- `RULE-S29-01`: Section references must use valid `Snn` anchors. Enforcement: docs lint.
- `RULE-S29-02`: Broken internal cross-references block merge. Enforcement: link-check CI.

## 30. Appendix: Canonical Type Quick Reference
### Design Decisions
- [v12 P2] Public IDs and handles have canonical aliases and crate ownership.
- [v12 P2] Binding-visible type changes require schema regeneration.
- [v12 P2] Type table is versioned with each pass.

### Rust Example
```rust
pub type SessionName = String;
pub type WindowName = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PaneId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TermletId(pub u64);
```

### Test Strategy
- Type alias consistency tests across crate boundaries.
- API surface diff tests for accidental widening.
- Binding schema regeneration checks for type changes.

### AGENTS.md Rules
- `RULE-S30-01`: Public types must map to canonical aliases/newtypes. Enforcement: API lint.
- `RULE-S30-02`: Type changes require binding schema regeneration. Enforcement: schema hash CI check.

## 31. Supplemental Test Matrix
### Design Decisions
- [v12 P2] Minimum PR matrix includes OS, shell, tmux lane, and one binding lane.
- [v12 P2] Full matrix runs nightly with expanded dimensions.
- [v12 P2] Quarantined tests require owner and expiry.

### Rust Example
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatrixCase {
    pub os: &'static str,
    pub shell: &'static str,
    pub lane: &'static str,
    pub binding: &'static str,
}

pub fn case_id(c: &MatrixCase) -> String {
    format!("{}:{}:{}:{}", c.os, c.shell, c.lane, c.binding)
}
```

### Test Strategy
- PR matrix coverage check for required dimensions.
- Nightly matrix completeness report.
- Quarantine manifest validation tests.

### AGENTS.md Rules
- `RULE-S31-01`: PR pipeline must execute minimum matrix set. Enforcement: required workflow checks.
- `RULE-S31-02`: Quarantined tests require owner+expiry. Enforcement: quarantine linter.

## 32. Termlets
### Design Decisions
#### 32.1 Product Positioning
- [v12 P2] Termlets remain the differentiator: programmable terminal pods for deterministic testing.
- [v12 P2] Conflict resolution: keep Claude/GPT model of pane-backed architecture, not a parallel terminal stack.
- [v12 P2] SDK parity is mandatory across Rust, Python, and Node.

#### 32.2 Core State Model
- [v12 P2] State machine is branch-safe and explicit: `Spawning -> Running -> Stopping -> Exited`, plus `Spawning -> SpawnFailed`.
- [v12 P2] Conflict resolution: `Created` is removed; `spawn()` returns `Running` or error-state result.
- [v12 P2] Conflict resolution: no `Ord/PartialOrd` for `TermletState`; use `valid_transition()` only.

#### 32.3 API Surface
- [v12 P2] Required APIs: `spawn`, `send_keys`, `send_bytes`, `wait_for`, `wait_for_condition`, `expect`, `snapshot`, `resize`, `kill`, `restart`.
- [v12 P2] `expect()` is a concise assertion wrapper for tests (Gemini contribution, retained with strict timeout/error messaging).
- [v12 P2] `ShellInteraction` trait is standardized for prompt detection and command-run loops.

#### 32.4 Async Model
- [v12 P2] Base `Termlet` remains sync and runtime-agnostic.
- [v12 P2] `AsyncTermlet` is feature-gated (`async`) and must not block runtime threads.
- [v12 P2] Async waits use `tokio::time::sleep` plus `spawn_blocking` where needed.

#### 32.5 Snapshot and Capture
- [v12 P2] Snapshot schema supports `text`, `cells`, and optional ANSI for debugging.
- [v12 P2] Cell-grid snapshot format is canonical for deterministic cross-language assertions.
- [v12 P2] Cross-language fixture schema is shared JSON with strict versioning.

#### 32.6 Isolation and Safety
- [v12 P2] Termlets must use isolated non-default socket paths.
- [v12 P2] Environment inheritance defaults to deny (`inherit_env=false`).
- [v12 P2] Kill semantics are SIGTERM -> grace timeout -> SIGKILL fallback and idempotent.

#### 32.7 CRDT and Collaboration
- [v12 P2] Collaborative termlet flows must replay deterministically under merge-order permutations.
- [v12 P2] Conflict artifacts are first-class outputs of collaboration tests.
- [v12 P2] CRDT collaboration test suites run when `crdt` feature is enabled.

### Rust Example
#### 32.8 Compile-Safe Core Types
```rust
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TermletState {
    Spawning,
    Running,
    Stopping,
    SpawnFailed,
    Exited,
}

impl TermletState {
    pub fn is_terminal(self) -> bool { matches!(self, Self::SpawnFailed | Self::Exited) }
}

pub fn valid_transition(from: TermletState, to: TermletState) -> bool {
    matches!((from, to),
        (TermletState::Spawning, TermletState::Running)
            | (TermletState::Spawning, TermletState::SpawnFailed)
            | (TermletState::Running, TermletState::Stopping)
            | (TermletState::Running, TermletState::Exited)
            | (TermletState::Stopping, TermletState::Exited)
            | (TermletState::Running, TermletState::Spawning)
            | (TermletState::Exited, TermletState::Spawning)
            | (TermletState::SpawnFailed, TermletState::Spawning)
    )
}

#[derive(Debug, Clone)]
pub struct WaitMatch {
    pub byte_start: usize,
    pub byte_end: usize,
    pub matched_at: Instant,
}
```

#### 32.9 Required APIs (`expect`, predicate wait, restart)
```rust
#[derive(Debug, Clone)]
pub struct TermletConfig {
    pub default_timeout: Duration,
    pub wait_poll_interval: Duration,
    pub wait_max_interval: Duration,
}

pub trait TermletLike {
    fn wait_for(&mut self, pattern: &str, timeout: Duration) -> Result<WaitMatch, String>;
    fn snapshot_text(&mut self) -> String;

    fn expect(&mut self, pattern: &str, timeout: Duration) {
        if let Err(e) = self.wait_for(pattern, timeout) {
            panic!("termlet expect failed: pattern={pattern:?} error={e}");
        }
    }

    fn wait_for_condition<F>(&mut self, mut pred: F, timeout: Duration) -> Result<(), String>
    where
        F: FnMut(&str) -> bool,
    {
        let start = Instant::now();
        let mut backoff = Duration::from_millis(25);
        while start.elapsed() < timeout {
            let text = self.snapshot_text();
            if pred(&text) {
                return Ok(());
            }
            std::thread::sleep(backoff.min(timeout.saturating_sub(start.elapsed())));
            backoff = (backoff * 2).min(Duration::from_millis(100));
        }
        Err("predicate_timeout".to_string())
    }
}
```

#### 32.10 Shell Interaction and Async Wrapper
```rust
pub trait ShellInteraction {
    fn wait_for_prompt(&mut self, timeout: Duration) -> Result<(), String>;
    fn run_command(&mut self, cmd: &str, timeout: Duration) -> Result<(), String>;
}

#[cfg(feature = "async")]
pub struct AsyncTermlet<T> {
    inner: T,
}

#[cfg(feature = "async")]
impl<T> AsyncTermlet<T>
where
    T: Send + 'static,
{
    pub async fn wait_tick(&self, d: Duration) {
        tokio::time::sleep(d).await;
    }
}
```

### Test Strategy
#### 32.11 Unit and Contract Tests
- [v12 P2] `TST-320`: spawn in fake and real modes.
- [v12 P2] `TST-321`: `send_keys` and `send_bytes` behavior, including non-UTF8 sequences.
- [v12 P2] `TST-322`: `wait_for` success/timeout/pattern-not-found distinctions.
- [v12 P2] `TST-323`: `expect()` failure message quality.
- [v12 P2] `TST-324`: `wait_for_condition` deterministic timeout behavior.
- [v12 P2] `TST-325`: `restart()` resets parser/grid/output history while preserving logical handle identity.

#### 32.12 State Machine and Error Tests
- [v12 P2] `TST-326`: no `Created` state externally observable.
- [v12 P2] `TST-327`: `TermletState` has no `Ord` implementation (compile-fail).
- [v12 P2] `TST-328`: illegal transitions rejected by `valid_transition()`.
- [v12 P2] `TST-329`: state-gated APIs return `InvalidState` vs `AlreadyExited` correctly.
- [v12 P2] `TST-330`: spawn failure enters `SpawnFailed` terminal state.

#### 32.13 Determinism and Snapshot Tests
- [v12 P2] `TST-331`: text snapshot normalization stability.
- [v12 P2] `TST-332`: cell-grid schema stability and expected cell counts.
- [v12 P2] `TST-333`: fake-vs-real PTY snapshot parity on shared fixtures.
- [v12 P2] `TST-334`: snapshot diff emits first-diff coordinates.

#### 32.14 Cross-Language Parity Tests
- [v12 P2] `TST-335`: Rust/Python/Node shared fixture parity.
- [v12 P2] `TST-336`: binding error code prefix parity.
- [v12 P2] `TST-337`: binding cleanup guarantees (`context manager`/`finally`) no leak.
- [v12 P2] `TST-338`: method order and naming parity contract test.

#### 32.15 Async and Shell Tests
- [v12 P2] `TST-339`: `AsyncTermlet` wait path does not block runtime thread.
- [v12 P2] `TST-340`: `ShellInteraction::wait_for_prompt` handles `$`, `#`, `>` prompt variants.
- [v12 P2] `TST-341`: `run_command` command->prompt roundtrip determinism.

#### 32.16 Fuzz, Stress, and Leak Tests
- [v12 P2] `TST-342`: termlet API-sequence fuzz target (spawn/send/resize/kill/restart permutations).
- [v12 P2] `TST-343`: random byte streams through fake backend must not panic.
- [v12 P2] `TST-344`: 50-cycle spawn/kill/restart FD and process leak checks.
- [v12 P2] `TST-345`: pool-level concurrent termlet lifecycle stress.

#### 32.17 Performance Budgets
- [v12 P2] `B10`: fake spawn target < 500us, hard-fail > 1ms.
- [v12 P2] `B11`: snapshot (80x24) target < 50us, hard-fail > 100us.
- [v12 P2] `B12`: real spawn target < 50ms, hard-fail > 100ms.
- [v12 P2] `B16`: `wait_for_condition` overhead target < 1ms/poll.

### AGENTS.md Rules
- `RULE-S32-01`: Termlets must use isolated non-default socket paths. Enforcement: harness runtime assert + leak checker.
- `RULE-S32-02`: `TermletState` transitions are validated by `valid_transition()` only. Enforcement: transition tests + compile-fail for `Ord`.
- `RULE-S32-03`: `Created` state is forbidden in public termlet lifecycle. Enforcement: API/state enum lint.
- `RULE-S32-04`: `send_bytes(&[u8])` must exist alongside `send_keys(&str)`. Enforcement: API surface test.
- `RULE-S32-05`: `expect()` must provide concise assertions with descriptive panic text. Enforcement: assertion contract tests.
- `RULE-S32-06`: `wait_for_condition` must support deterministic polling and bounded backoff. Enforcement: timing-behavior tests.
- `RULE-S32-07`: `AsyncTermlet` must not block runtime threads. Enforcement: async starvation test.
- `RULE-S32-08`: `ShellInteraction` prompt detection must support variable prompt suffixes. Enforcement: prompt-variant suite.
- `RULE-S32-09`: `restart()` must kill before respawn and clear transient buffers. Enforcement: lifecycle tests.
- `RULE-S32-10`: Snapshot schema must include deterministic cell-grid mode. Enforcement: snapshot schema validator.
- `RULE-S32-11`: Every termlet feature requires Rust + pytest + vitest coverage. Enforcement: multi-runtime CI gate.
- `RULE-S32-12`: Collaborative termlet flows must pass CRDT merge-order replay tests. Enforcement: merge simulation suite.

### Final Consistency Checklist
- [x] [v12 P2] Exactly 32 numbered sections are present (`## 1` through `## 32`).
- [x] [v12 P2] Every section contains the required four headings: `### Design Decisions`, `### Rust Example`, `### Test Strategy`, `### AGENTS.md Rules`.
- [x] [v12 P2] Section 32 is the deepest section and includes detailed `####` subsections.
- [x] [v12 P2] Rules follow `RULE-Snn-xx` naming and each has an explicit enforcement mechanism.
- [x] [v12 P2] Conflicts were resolved toward stronger architecture: Layer-0 `mux-core`, explicit termlet transitions, `PtyHandle` decoupling, and feature-gated async/crdt.
- [x] [v12 P2] Unique contributions integrated: compiled-example discipline (Gemini), gate/risk governance rigor (GPT5), critical v11 bug fixes and termlet lifecycle hardening (Claude).
- [x] [v12 P2] Section 32 test strategy is strengthened with unit, property, compile-fail, fuzz, stress, perf, async, shell, and cross-language parity coverage.
