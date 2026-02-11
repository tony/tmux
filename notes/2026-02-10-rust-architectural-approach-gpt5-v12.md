# TermForge Rust Architecture v12 (Codex)

## 1. Project Identity
### Design Decisions
- [v12] TermForge is a Rust-first, SDK-first terminal multiplexer with a hard requirement of 100% tmux wire-protocol compatibility for supported tmux versions (see Section 20).
- [v12] v11 weakness identified: protocol-compatibility language was broad; v12 narrows this to measurable byte-level parity for command replies, control-mode streams, and error surfaces.
- [v12] Product boundary: TermForge provides a compatibility server, an orthogonal object SDK, language bindings, collaboration primitives, and a termlet runtime; it does not embed non-terminal app frameworks.
- [v12] License is dual permissive only: `MIT OR Apache-2.0` across all first-party crates and SDK artifacts.
- [v12] Guiding constraints: deterministic core, replayable state transitions, stable identifiers, and explicit version-scoped feature gates.

### Rust Example
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LicenseMode {
    MitOrApache20,
}

#[derive(Debug, Clone)]
pub struct ProductIdentity {
    pub name: &'static str,
    pub protocol_target: &'static str,
    pub license: LicenseMode,
}

impl ProductIdentity {
    pub const fn termforge() -> Self {
        Self {
            name: "TermForge",
            protocol_target: "tmux-wire-compat",
            license: LicenseMode::MitOrApache20,
        }
    }
}
```

### Test Strategy
- [v12] Unit: assert identity constants, license mode, and feature-gate defaults.
- [v12] Conformance: protocol handshake golden tests against tmux fixtures per version lane.
- [v12] Governance: CI blocks release if any crate metadata declares a non-permissive license.

### AGENTS.md Rules
- `RULE-S01-01`: Any PR changing compatibility scope must update Section 20 matrices. Enforcement: CI checks changed files include both architecture spec and version matrix.
- `RULE-S01-02`: Any new crate must declare `MIT OR Apache-2.0`. Enforcement: metadata linter fails on mismatch.

## 2. Acceptance Criteria and Gates
### Design Decisions
- [v12] v11 weakness identified: acceptance gates mixed design and implementation detail; v12 defines four explicit gate classes: `compat`, `correctness`, `performance`, `operability`.
- [v12] Gate thresholds are release-blocking and versioned; no soft warnings for parity regressions.
- [v12] Compatibility gate: byte-accurate output and exit semantics for canonical tmux command corpus.
- [v12] Correctness gate: deterministic replay for state logs and CRDT merge determinism.
- [v12] Performance gate: p95 command latency budget and bounded memory growth under pane churn.
- [v12] Operability gate: OpenTelemetry spans and metrics emitted from all runtime services.

### Rust Example
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateClass {
    Compat,
    Correctness,
    Performance,
    Operability,
}

#[derive(Debug, Clone)]
pub struct GateResult {
    pub class: GateClass,
    pub passed: bool,
    pub detail: String,
}

pub fn all_pass(results: &[GateResult]) -> bool {
    results.iter().all(|r| r.passed)
}
```

### Test Strategy
- [v12] Gate tests run in separate CI jobs and publish machine-readable summaries.
- [v12] A release tag is blocked unless all gate classes pass for each supported tmux version lane.
- [v12] Regression test inserts one failing result to verify release-block behavior.

### AGENTS.md Rules
- `RULE-S02-01`: Every milestone PR must list impacted gate classes. Enforcement: PR template validator.
- `RULE-S02-02`: Compatibility failures are never `allow_failure`. Enforcement: CI policy script.

## 3. Crate Dependency Rules
### Design Decisions
- [v12] v11 weakness identified: some dependency boundaries were descriptive only; v12 defines deny-by-default dependency graph checks.
- [v12] Core crates (`mux-core`, `mux-domain`) are `std`-only unless explicitly waived.
- [v12] `mux-server` may depend on async/runtime crates; SDK crates may not depend on server runtime internals.
- [v12] FFI crates depend on SDK abstractions, never on transport internals.
- [v12] Each crate publishes `public_api.md` generated in CI to detect accidental API growth.

### Rust Example
```rust
#[derive(Debug, Clone)]
pub struct CratePolicy {
    pub crate_name: &'static str,
    pub allowed_deps: &'static [&'static str],
}

pub fn dep_allowed(policy: &CratePolicy, dep: &str) -> bool {
    policy.allowed_deps.iter().any(|d| *d == dep)
}
```

### Test Strategy
- [v12] Static graph lint verifies forbidden edges.
- [v12] Public API snapshot diff fails when unapproved symbols are added.
- [v12] Dependency audit checks dual-license compatibility and security advisories.

### AGENTS.md Rules
- `RULE-S03-01`: New dependencies require rationale in `docs/deps/<crate>.md`. Enforcement: changed `Cargo.toml` without rationale fails.
- `RULE-S03-02`: Cross-layer dependency violations block merge. Enforcement: `cargo deny` + custom graph checker.

## 4. Workspace Layout
### Design Decisions
- [v12] v11 weakness identified: layout guidance lacked ownership boundaries; v12 assigns directory-level ownership and lifecycle.
- [v12] Workspace top-level groups: `core/`, `server/`, `sdk/`, `bindings/`, `collab/`, `termlet/`, `tools/`, `tests/`, `docs/`.
- [v12] Generated code is isolated under `generated/` with no manual edits.
- [v12] Shared test fixtures are immutable once published for a version lane.
- [v12] Tooling crates (`mux-builder`, `mux-vm`, `mux-test-support`) are versioned with runtime code to prevent drift.

### Rust Example
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceGroup {
    Core,
    Server,
    Sdk,
    Bindings,
    Collab,
    Termlet,
    Tools,
    Tests,
    Docs,
}

pub fn group_path(group: WorkspaceGroup) -> &'static str {
    match group {
        WorkspaceGroup::Core => "core",
        WorkspaceGroup::Server => "server",
        WorkspaceGroup::Sdk => "sdk",
        WorkspaceGroup::Bindings => "bindings",
        WorkspaceGroup::Collab => "collab",
        WorkspaceGroup::Termlet => "termlet",
        WorkspaceGroup::Tools => "tools",
        WorkspaceGroup::Tests => "tests",
        WorkspaceGroup::Docs => "docs",
    }
}
```

### Test Strategy
- [v12] Repo integrity test ensures required top-level directories exist.
- [v12] Ownership map lint enforces CODEOWNERS alignment with workspace groups.
- [v12] Generated directories are checked for reproducibility.

### AGENTS.md Rules
- `RULE-S04-01`: New top-level directories need architecture approval. Enforcement: path-change guard in CI.
- `RULE-S04-02`: Files under `generated/` are tool-owned only. Enforcement: manual edits to generated files fail.

## 5. mux-core: Pure Kernel
### Design Decisions
- [v12] v11 weakness identified: purity requirement was implied; v12 states `mux-core` is side-effect free and deterministic.
- [v12] Kernel input: prior state + command/event; output: next state + effect intents.
- [v12] No I/O, wall-clock, randomness, or env reads in kernel paths.
- [v12] Effects are typed intents executed by adapters outside core.
- [v12] Replay logs are canonicalized to allow deterministic audit and bisect.

### Rust Example
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    pub sessions: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    NewSession,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectIntent {
    PersistSnapshot,
}

pub fn reduce(state: &State, cmd: Command) -> (State, Vec<EffectIntent>) {
    match cmd {
        Command::NewSession => (
            State {
                sessions: state.sessions + 1,
            },
            vec![EffectIntent::PersistSnapshot],
        ),
    }
}
```

### Test Strategy
- [v12] Property tests: reducer determinism for equivalent input streams.
- [v12] Replay tests: serialized command log yields stable state hash.
- [v12] Mutation tests ensure no hidden side effects are introduced.

### AGENTS.md Rules
- `RULE-S05-01`: `mux-core` cannot depend on async/network/fs crates. Enforcement: dependency linter.
- `RULE-S05-02`: Reducer changes must include replay fixture updates. Enforcement: CI checks fixture delta.

## 6. Entity ID Design
### Design Decisions
- [v12] v11 weakness identified: ID format rationale was too open-ended; v12 standardizes monotonic local IDs plus globally unique replica prefixes for collaboration.
- [v12] IDs are opaque in public APIs and are not parsed by consumers.
- [v12] String representations are stable for wire compatibility and snapshot readability.
- [v12] Internal numeric compact form is allowed only behind conversion APIs.
- [v12] Collision policy: impossible by construction for a `(replica_id, counter)` pair.

### Rust Example
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReplicaId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityId {
    pub replica: ReplicaId,
    pub counter: u64,
}

impl EntityId {
    pub fn as_string(self) -> String {
        format!("{}:{}", self.replica.0, self.counter)
    }
}
```

### Test Strategy
- [v12] Unit: parse/format roundtrip for textual IDs.
- [v12] Stress: generate IDs concurrently per replica and assert uniqueness.
- [v12] Compatibility: ensure wire serialization remains backward-compatible.

### AGENTS.md Rules
- `RULE-S06-01`: Public APIs must accept opaque ID types, not raw strings. Enforcement: API lint.
- `RULE-S06-02`: Any ID format change requires migration docs and fixture updates. Enforcement: compatibility job.

## 7. Event -> Effect Architecture
### Design Decisions
- [v12] v11 weakness identified: event/effect causality was underspecified; v12 requires explicit causation IDs linking command, events, and effects.
- [v12] Events are immutable facts; effects are executable intents with retry policy metadata.
- [v12] Effect execution must be idempotent by effect key.
- [v12] Event ordering is total per session and causal across sessions when CRDT sync is active.
- [v12] Failed effects produce diagnostic events for observability and replay.

### Rust Example
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CausationId(pub u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub causation: CausationId,
    pub name: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Effect {
    pub causation: CausationId,
    pub key: String,
}

pub fn dedupe_effects(effects: &[Effect]) -> usize {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    for e in effects {
        set.insert(e.key.clone());
    }
    set.len()
}
```

### Test Strategy
- [v12] Validate each effect has a matching causation chain from a command.
- [v12] Inject retries and verify idempotency behavior.
- [v12] Replay diagnostic events to reproduce failed runs.

### AGENTS.md Rules
- `RULE-S07-01`: New effect types must define idempotency key derivation. Enforcement: compile-time trait bound + tests.
- `RULE-S07-02`: Diagnostic events are mandatory on terminal effect failure. Enforcement: integration test.

## 8. Error Taxonomy
### Design Decisions
- [v12] v11 weakness identified: error classes were broad; v12 separates `user`, `compat`, `internal`, `transient`, and `policy` errors.
- [v12] Public SDK errors are stable enums with machine-readable codes.
- [v12] Wire-compat errors preserve tmux-like messaging where required while retaining structured internals.
- [v12] Internal errors always include causation and span context.
- [v12] Error redaction policy prevents secret leakage in logs and binding exceptions.

### Rust Example
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorClass {
    User,
    Compat,
    Internal,
    Transient,
    Policy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TfError {
    pub class: ErrorClass,
    pub code: &'static str,
    pub message: String,
}

impl TfError {
    pub fn new(class: ErrorClass, code: &'static str, message: impl Into<String>) -> Self {
        Self { class, code, message: message.into() }
    }
}
```

### Test Strategy
- [v12] Snapshot error rendering for CLI, Rust SDK, Python, and Node surfaces.
- [v12] Redaction tests ensure secrets are not emitted.
- [v12] Compatibility tests map internal classes to tmux-compatible messages.

### AGENTS.md Rules
- `RULE-S08-01`: New error codes require docs and bindings mapping updates. Enforcement: schema diff checker.
- `RULE-S08-02`: Secret-bearing fields must implement redaction. Enforcement: log-scrub tests.

## 9. Protocol Codec (mux-proto)
### Design Decisions
- [v12] v11 weakness identified: codec behavior under malformed frames was vague; v12 defines strict framing, bounded lengths, and deterministic recoverability.
- [v12] `mux-proto` is versioned with explicit feature tables per tmux target version.
- [v12] Unknown commands follow tmux-compatible rejection semantics.
- [v12] Codec APIs are pure parse/encode transforms; transport concerns are external.
- [v12] Byte-level golden fixtures are canonical artifacts in repo.

### Rust Example
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub payload: Vec<u8>,
}

pub fn encode(frame: &Frame) -> Vec<u8> {
    let len = frame.payload.len() as u32;
    let mut out = len.to_be_bytes().to_vec();
    out.extend_from_slice(&frame.payload);
    out
}

pub fn decode(input: &[u8]) -> Option<Frame> {
    if input.len() < 4 {
        return None;
    }
    let len = u32::from_be_bytes([input[0], input[1], input[2], input[3]]) as usize;
    if input.len() != 4 + len {
        return None;
    }
    Some(Frame {
        payload: input[4..].to_vec(),
    })
}
```

### Test Strategy
- [v12] Golden tests compare encoded bytes against fixture corpus.
- [v12] Fuzz tests target parser panic safety and frame boundary handling.
- [v12] Differential tests compare responses with tmux across supported versions.

### AGENTS.md Rules
- `RULE-S09-01`: Protocol changes require fixture updates and tmux diff report. Enforcement: CI gate.
- `RULE-S09-02`: Parser must reject oversized frames using configured bounds. Enforcement: bounds test suite.

## 10. Config and Option System
### Design Decisions
- [v12] v11 weakness identified: config source precedence was unclear; v12 fixes order as `CLI > env > project file > defaults`.
- [v12] Option keys preserve tmux compatibility names where user-facing.
- [v12] Typed config schema is generated once and reused by Rust SDK and bindings.
- [v12] Unknown options are rejected or ignored based on explicit compatibility mode.
- [v12] Runtime config changes emit events for auditability.

### Rust Example
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub socket_name: String,
    pub compat_mode: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            socket_name: "termforge-default".to_string(),
            compat_mode: true,
        }
    }
}

pub fn merge(cli: Option<Config>, env: Option<Config>, file: Option<Config>) -> Config {
    cli.or(env).or(file).unwrap_or_default()
}
```

### Test Strategy
- [v12] Matrix tests for precedence order.
- [v12] Schema snapshot tests for Rust/Python/Node consistency.
- [v12] Dynamic update tests ensure event emission on mutation.

### AGENTS.md Rules
- `RULE-S10-01`: New config fields require schema version bump. Enforcement: schema version checker.
- `RULE-S10-02`: Precedence changes need migration notes. Enforcement: doc+code parity check.

## 11. Layout Engine
### Design Decisions
- [v12] v11 weakness identified: resize edge behavior under nested splits was underspecified; v12 defines deterministic split conflict resolution.
- [v12] Layout solver uses integer-cell arithmetic only.
- [v12] Min/max pane constraints are strict and surfaced as typed errors.
- [v12] Layout state is serializable and replayable.
- [v12] Algorithm is deterministic regardless of hash map iteration order.

### Rust Example
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}

pub fn split_horizontal(area: Rect) -> (Rect, Rect) {
    let top_h = area.h / 2;
    let bottom_h = area.h - top_h;
    (
        Rect { h: top_h, ..area },
        Rect {
            y: area.y + top_h,
            h: bottom_h,
            ..area
        },
    )
}
```

### Test Strategy
- [v12] Property tests verify area conservation after repeated splits/merges.
- [v12] Regression fixtures capture known tmux resize semantics.
- [v12] Fuzz tests for random resize sequences.

### AGENTS.md Rules
- `RULE-S11-01`: Layout math must use integer cells only. Enforcement: lint disallowing float ops in layout crate.
- `RULE-S11-02`: Every layout bug fix adds a replay fixture. Enforcement: bug label requires fixture file.

## 12. QueryList ORM Layer
### Design Decisions
- [v12] v11 weakness identified: ORM API ergonomics were broad but not fully orthogonal; v12 introduces immutable query builders with stable predicates.
- [v12] QueryList API mirrors libtmux expressiveness: `filter`, `exclude`, `order_by`, `first`, `one`, `get`, `pluck`.
- [v12] Query evaluation is lazy until terminal methods are invoked.
- [v12] Predicates are serializable for remote execution and language bindings.
- [v12] Nullability semantics are explicit across Rust/Python/Node.

### Rust Example
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub id: u64,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct QueryList<T> {
    items: Vec<T>,
}

impl<T: Clone> QueryList<T> {
    pub fn new(items: Vec<T>) -> Self {
        Self { items }
    }

    pub fn filter<F>(&self, mut predicate: F) -> Self
    where
        F: FnMut(&T) -> bool,
    {
        let items = self.items.iter().filter(|x| predicate(x)).cloned().collect();
        Self { items }
    }

    pub fn first(&self) -> Option<T> {
        self.items.first().cloned()
    }
}
```

### Test Strategy
- [v12] Cross-language API parity tests for query semantics.
- [v12] Snapshot tests for generated binding docs and examples.
- [v12] Performance tests for query chaining on large object graphs.

### AGENTS.md Rules
- `RULE-S12-01`: Query methods must be orthogonal and chain-safe. Enforcement: API contract tests.
- `RULE-S12-02`: Any query semantic change requires Rust/Python/Node snapshot updates. Enforcement: binding parity CI.

## 13. State Actor and ArcSwap
### Design Decisions
- [v12] v11 weakness identified: concurrent read/write strategy lacked failure policy; v12 formalizes single-writer actor with lock-free snapshot reads.
- [v12] Writer actor applies commands serially and publishes immutable snapshots.
- [v12] Reader paths never block on writer except at explicit consistency barriers.
- [v12] Snapshot publication includes monotonically increasing revision IDs.
- [v12] Backpressure policy is explicit: bounded command queue with overload error class.

### Rust Example
```rust
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub revision: u64,
}

#[derive(Debug, Clone)]
pub struct SnapshotStore {
    inner: Arc<RwLock<Snapshot>>,
}

impl SnapshotStore {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(Snapshot { revision: 0 })),
        }
    }

    pub fn read(&self) -> Snapshot {
        self.inner.read().expect("lock poisoned").clone()
    }

    pub fn publish(&self, next_revision: u64) {
        let mut guard = self.inner.write().expect("lock poisoned");
        *guard = Snapshot { revision: next_revision };
    }
}
```

### Test Strategy
- [v12] Concurrency tests for monotonic revision visibility.
- [v12] Overload tests for bounded queue policy.
- [v12] Replay tests verify identical revision stream under same command input.

### AGENTS.md Rules
- `RULE-S13-01`: State mutation is actor-owned only. Enforcement: module visibility restrictions.
- `RULE-S13-02`: Snapshot reads must not mutate state. Enforcement: clippy + review checklist.

## 14. Server Lifecycle
### Design Decisions
- [v12] v11 weakness identified: startup/shutdown phases were listed but not state-machined; v12 introduces explicit lifecycle states.
- [v12] States: `Booting -> Running -> Draining -> Stopped` with legal transitions only.
- [v12] Startup validates socket path, permissions, and compatibility mode before accepting clients.
- [v12] Draining rejects new sessions and completes in-flight operations with deadline.
- [v12] Crash recovery replays durable log and publishes recovery event summary.

### Rust Example
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lifecycle {
    Booting,
    Running,
    Draining,
    Stopped,
}

pub fn transition(from: Lifecycle, to: Lifecycle) -> bool {
    matches!(
        (from, to),
        (Lifecycle::Booting, Lifecycle::Running)
            | (Lifecycle::Running, Lifecycle::Draining)
            | (Lifecycle::Draining, Lifecycle::Stopped)
    )
}
```

### Test Strategy
- [v12] State-machine tests for legal/illegal transitions.
- [v12] Integration tests for graceful shutdown with active panes.
- [v12] Recovery tests for log replay from mid-flight failures.

### AGENTS.md Rules
- `RULE-S14-01`: Lifecycle transitions must be validated centrally. Enforcement: transition API only.
- `RULE-S14-02`: Shutdown deadlines are mandatory config. Enforcement: startup validation test.

## 15. Control Mode
### Design Decisions
- [v12] v11 weakness identified: control-mode stream guarantees were not strict; v12 defines frame ordering, flushing, and heartbeat semantics.
- [v12] Control clients receive ordered frames per connection.
- [v12] Heartbeats are optional but standardized when enabled.
- [v12] Partial writes and disconnects are explicit error classes with retry guidance.
- [v12] Backward compatibility is maintained via version-capability negotiation.

### Rust Example
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlFrame {
    pub seq: u64,
    pub body: String,
}

pub fn is_in_order(frames: &[ControlFrame]) -> bool {
    frames
        .windows(2)
        .all(|w| w[0].seq < w[1].seq)
}
```

### Test Strategy
- [v12] Stream ordering tests under heavy event load.
- [v12] Disconnect/reconnect tests for frame boundary correctness.
- [v12] Negotiation tests for older capability profiles.

### AGENTS.md Rules
- `RULE-S15-01`: Control frames must carry monotonic sequence numbers. Enforcement: integration assertions.
- `RULE-S15-02`: Protocol negotiation tables must be versioned. Enforcement: schema diff check.

## 16. Language Bindings
### Design Decisions
- [v12] v11 weakness identified: binding behavior for errors and iterators was not strict enough; v12 sets a generated FFI schema and shared semantics package.
- [v12] Python binding uses PyO3; Node binding uses Neon.
- [v12] Bindings expose QueryList-like APIs with lazy semantics preserved.
- [v12] Error mapping is lossless for code/class and best-effort for source spans.
- [v12] ABI surface is minimized and versioned independently from internal crates.

### Rust Example
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingError {
    pub code: &'static str,
    pub message: String,
}

pub trait BindingAdapter {
    fn map_error(&self, code: &'static str, message: &str) -> BindingError;
}

pub struct DefaultAdapter;

impl BindingAdapter for DefaultAdapter {
    fn map_error(&self, code: &'static str, message: &str) -> BindingError {
        BindingError {
            code,
            message: message.to_string(),
        }
    }
}
```

### Test Strategy
- [v12] Rust/Python/Node conformance suite runs shared query and error fixtures.
- [v12] ABI compatibility tests for minor-version upgrades.
- [v12] Packaging tests verify wheel/npm artifacts bundle correct native binaries.

### AGENTS.md Rules
- `RULE-S16-01`: Bindings must consume generated schema, not handwritten drifted structs. Enforcement: generated-file hash check.
- `RULE-S16-02`: Binding releases require parity suite pass on all supported OS targets. Enforcement: release workflow gate.

## 17. CRDT Layer
### Design Decisions
- [v12] v11 weakness identified: transaction semantics across replicas were partially specified; v12 defines operation IDs, Lamport clocks, and merge preconditions.
- [v12] Local operations are applied optimistically with durable op-log append.
- [v12] Merge is associative, commutative, and idempotent for all CRDT entities.
- [v12] Conflicts that cannot be auto-merged are materialized as first-class review tasks, not silent drops.
- [v12] CRDT layer is optional at runtime but always compiled and test-covered.

### Rust Example
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Lamport(pub u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Op {
    pub actor: u32,
    pub seq: u64,
    pub clock: Lamport,
}

pub fn next_clock(local: Lamport, observed: Lamport) -> Lamport {
    Lamport(local.0.max(observed.0) + 1)
}
```

### Test Strategy
- [v12] Property tests validate CRDT algebraic laws.
- [v12] Jepsen-style simulation tests for partitions and concurrent edits.
- [v12] Replay tests verify deterministic final state across merge orders.

### AGENTS.md Rules
- `RULE-S17-01`: CRDT entities must document merge law proofs or counterexample tests. Enforcement: docs+test checker.
- `RULE-S17-02`: Non-mergeable conflicts must emit review tasks. Enforcement: conflict simulation tests.

## 18. Socket and Permissions
### Design Decisions
- [v12] v11 weakness identified: socket isolation guidance existed but enforcement was weak; v12 makes non-default isolated sockets mandatory for all tests.
- [v12] Test runs must set `socket_name` and `socket_path` outside tmux defaults.
- [v12] Socket paths are ephemeral, unique per test process, and cleaned in teardown.
- [v12] Runtime permission checks enforce user-only access unless explicit override.
- [v12] Cleanup executes on normal exit and panic paths.

### Rust Example
```rust
use std::path::PathBuf;

pub fn test_socket_path(test_id: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("termforge-{}-sock", test_id));
    p
}

pub fn outside_tmux_default(path: &str) -> bool {
    !path.contains("/tmp/tmux-")
}
```

### Test Strategy
- [v12] Integration tests assert socket path uniqueness and cleanup.
- [v12] Permission tests verify reject-by-default on unsafe modes.
- [v12] Panic-path tests ensure cleanup hooks still run.

### AGENTS.md Rules
- `RULE-S18-01`: Tests must never use default tmux socket path. Enforcement: runtime assert in test harness.
- `RULE-S18-02`: Socket artifacts must be removed post-test. Enforcement: filesystem leak check.

## 19. Observability (OTEL)
### Design Decisions
- [v12] v11 weakness identified: OTEL adoption was broad but not end-to-end; v12 requires trace continuity across core server, builders, bindings, and termlets.
- [v12] Every request flow has root span + causation attributes.
- [v12] Metrics set includes command latency, queue depth, protocol errors, CRDT merge lag, and termlet run outcomes.
- [v12] Sampling defaults to parent-based with override for parity test runs.
- [v12] Logs and traces share correlation IDs.

### Rust Example
```rust
#[derive(Debug, Clone)]
pub struct SpanData {
    pub trace_id: u128,
    pub name: &'static str,
}

pub fn child_span(parent: &SpanData, name: &'static str) -> SpanData {
    SpanData {
        trace_id: parent.trace_id,
        name,
    }
}
```

### Test Strategy
- [v12] Integration tests assert required span attributes are present.
- [v12] Metrics snapshot tests validate instrument names and units.
- [v12] End-to-end tests verify correlation ID continuity through bindings.

### AGENTS.md Rules
- `RULE-S19-01`: New RPC/command paths require trace instrumentation. Enforcement: instrumentation coverage test.
- `RULE-S19-02`: Metric names are append-only unless deprecated via migration doc. Enforcement: metric registry diff.

## 20. tmux Builder and Version Manager
### Design Decisions
- [v12] v11 weakness identified: version lane policy lacked a strict support contract; v12 defines `LTS`, `Current`, and `Preview` lanes.
- [v12] `mux-builder` obtains and builds tmux versions reproducibly.
- [v12] `mux-vm` orchestrates test matrices against built tmux binaries.
- [v12] Compatibility reports are artifacted by commit SHA and version lane.
- [v12] Unsupported versions fail fast with actionable diagnostics.

### Rust Example
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionLane {
    Lts,
    Current,
    Preview,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TmuxTarget {
    pub version: &'static str,
    pub lane: VersionLane,
}
```

### Test Strategy
- [v12] Builder tests verify deterministic source resolution.
- [v12] VM tests run parity suites across all lane targets.
- [v12] Report schema tests ensure historical comparability.

### AGENTS.md Rules
- `RULE-S20-01`: Any compatibility claim must reference lane + version. Enforcement: docs checker.
- `RULE-S20-02`: Parity CI must run at least one version per lane. Enforcement: workflow policy.

## 21. Test Support and FakePty
### Design Decisions
- [v12] v11 weakness identified: fake PTY fidelity criteria were ambiguous; v12 defines required behaviors (echo, resize, signal, UTF-8 boundary handling).
- [v12] `mux-test-support` provides deterministic fake PTY and real PTY adapters.
- [v12] Fake PTY has scripted timing control to eliminate flake.
- [v12] Real PTY smoke tests remain mandatory for integration confidence.
- [v12] Output capture supports byte and cell-grid modes.

### Rust Example
```rust
#[derive(Debug, Clone)]
pub struct FakePty {
    pub cols: u16,
    pub rows: u16,
    pub buffer: Vec<u8>,
}

impl FakePty {
    pub fn new(cols: u16, rows: u16) -> Self {
        Self { cols, rows, buffer: Vec::new() }
    }

    pub fn write(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.cols = cols;
        self.rows = rows;
    }
}
```

### Test Strategy
- [v12] Contract tests compare fake vs real PTY behavior on canonical scripts.
- [v12] Timing-control tests validate deterministic scheduling.
- [v12] Unicode tests cover multi-byte boundaries and combining characters.

### AGENTS.md Rules
- `RULE-S21-01`: Fake PTY changes require parity diff against real PTY corpus. Enforcement: corpus comparison test.
- `RULE-S21-02`: New terminal behavior features need both fake and real PTY tests. Enforcement: test coverage label gate.

## 22. Parity Test Framework
### Design Decisions
- [v12] v11 weakness identified: parity scope and verdict logic were too coarse; v12 defines command-level verdicts with explainable diffs.
- [v12] Framework runs same scenario against tmux and TermForge.
- [v12] Verdicts classify as `match`, `acceptable-drift`, or `fail`, where drift requires documented waiver.
- [v12] Drift waivers are version-scoped and expire automatically.
- [v12] Artifacts include byte streams, state snapshots, and timing summaries.

### Rust Example
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Match,
    AcceptableDrift,
    Fail,
}

pub fn verdict(expected: &[u8], actual: &[u8]) -> Verdict {
    if expected == actual {
        Verdict::Match
    } else {
        Verdict::Fail
    }
}
```

### Test Strategy
- [v12] Scenario corpus includes session/window/pane lifecycle and control-mode streams.
- [v12] Waiver expiry tests ensure temporary drifts cannot linger.
- [v12] CI publishes per-command verdict summaries.

### AGENTS.md Rules
- `RULE-S22-01`: New drift waiver requires expiry date and owner. Enforcement: waiver schema validator.
- `RULE-S22-02`: `fail` verdicts in supported lanes block merge. Enforcement: parity gate.

## 23. Fuzz Testing
### Design Decisions
- [v12] v11 weakness identified: fuzz targets were listed but triage process was unclear; v12 defines reproducible crash minimization workflow.
- [v12] Fuzz targets: codec, control parser, layout transitions, CRDT op ingestion.
- [v12] Corpus seeds include real-world captures and generated edge cases.
- [v12] Every crash case gets minimized and added to regression corpus.
- [v12] Fuzzing is continuous on main and nightly deeper campaigns.

### Rust Example
```rust
pub fn safe_index(input: &[u8], idx: usize) -> Option<u8> {
    input.get(idx).copied()
}

pub fn parse_u16_pair(input: &[u8]) -> Option<(u16, u16)> {
    if input.len() < 4 {
        return None;
    }
    let a = u16::from_be_bytes([input[0], input[1]]);
    let b = u16::from_be_bytes([input[2], input[3]]);
    Some((a, b))
}
```

### Test Strategy
- [v12] Per-target coverage trend monitoring and minimum thresholds.
- [v12] Crash replay tests run in normal CI.
- [v12] Seed corpus integrity checks prevent accidental deletion.

### AGENTS.md Rules
- `RULE-S23-01`: New parser modules require fuzz target registration. Enforcement: module-to-target mapping check.
- `RULE-S23-02`: Fuzz-found bugs must include minimized reproducer before close. Enforcement: issue workflow bot.

## 24. Performance Benchmarks
### Design Decisions
- [v12] v11 weakness identified: benchmark definitions mixed micro and macro metrics; v12 separates them with distinct SLAs.
- [v12] Microbenchmarks track parser, query filtering, layout operations.
- [v12] Macrobenchmarks track startup, session churn, pane broadcast, and control-mode throughput.
- [v12] Benchmarks run in controlled environments with fixed CPU governor where available.
- [v12] Regressions above threshold require either fix or approved waiver.

### Rust Example
```rust
use std::time::{Duration, Instant};

pub fn measure<F: FnOnce()>(f: F) -> Duration {
    let start = Instant::now();
    f();
    start.elapsed()
}
```

### Test Strategy
- [v12] Baseline snapshots are stored per hardware class.
- [v12] CI compares current runs against rolling median.
- [v12] Noise-control tests validate benchmark harness stability.

### AGENTS.md Rules
- `RULE-S24-01`: Benchmark harness changes require baseline revalidation. Enforcement: benchmark metadata check.
- `RULE-S24-02`: Performance waivers must include expiration and owner. Enforcement: waiver schema check.

## 25. Visual Client / TUI
### Design Decisions
- [v12] v11 weakness identified: TUI contract with core state snapshots was loosely described; v12 defines a unidirectional render data model.
- [v12] TUI reads immutable snapshots and emits user intents only.
- [v12] Rendering pipeline is deterministic from `(snapshot, viewport, theme)`.
- [v12] Mouse and keyboard mappings are configurable with compatibility defaults.
- [v12] Client supports headless render mode for screenshot-based snapshot tests.

### Rust Example
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewModel {
    pub title: String,
    pub lines: Vec<String>,
}

pub fn render(vm: &ViewModel) -> String {
    let mut out = String::new();
    out.push_str(&vm.title);
    out.push('\n');
    for line in &vm.lines {
        out.push_str(line);
        out.push('\n');
    }
    out
}
```

### Test Strategy
- [v12] Snapshot tests for canonical view states.
- [v12] Input mapping tests for keyboard/mouse compatibility behavior.
- [v12] Headless rendering tests for CI determinism.

### AGENTS.md Rules
- `RULE-S25-01`: Render output must be deterministic for fixed inputs. Enforcement: snapshot stability check.
- `RULE-S25-02`: New input mappings require docs and tests. Enforcement: mapping registry test.

## 26. AGENTS.md Rules
### Design Decisions
- [v12] v11 weakness identified: agent instructions were comprehensive but not sufficiently enforceable; v12 ties rules to executable checks where possible.
- [v12] Rules are scoped by section and use `RULE-Snn-xx` naming.
- [v12] Every section includes at least two enforceable rules.
- [v12] Non-enforceable process rules are minimized and explicitly marked manual-review.
- [v12] Rule registry is generated for discoverability and conflict checks.

### Rust Example
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    pub id: &'static str,
    pub enforceable: bool,
}

pub fn duplicate_rule_ids(rules: &[Rule]) -> bool {
    use std::collections::HashSet;
    let mut seen = HashSet::new();
    for rule in rules {
        if !seen.insert(rule.id) {
            return true;
        }
    }
    false
}
```

### Test Strategy
- [v12] Rule ID uniqueness tests over full architecture spec.
- [v12] Enforcement-tag tests ensure each rule declares mechanism.
- [v12] Manual-review rules tracked with explicit owners.

### AGENTS.md Rules
- `RULE-S26-01`: All rule IDs must match `RULE-Snn-xx`. Enforcement: regex linter.
- `RULE-S26-02`: Duplicate rule IDs are forbidden. Enforcement: rule registry generator test.

## 27. Risks and Mitigations
### Design Decisions
- [v12] v11 weakness identified: risk list lacked trigger metrics; v12 attaches measurable signals and fallback actions.
- [v12] Top risks: protocol drift, binding divergence, CRDT correctness regressions, and test flakiness.
- [v12] Each risk has owner, detection metric, threshold, and rollback path.
- [v12] Risk review cadence is per release lane.
- [v12] Critical risk breach blocks promotion from preview to current lane.

### Rust Example
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RiskItem {
    pub key: &'static str,
    pub threshold: u64,
    pub observed: u64,
}

pub fn breached(item: &RiskItem) -> bool {
    item.observed > item.threshold
}
```

### Test Strategy
- [v12] Synthetic risk drills validate alerting and rollback scripts.
- [v12] CI checks require non-empty risk metadata for major features.
- [v12] Release tests verify lane promotion blocks when thresholds are breached.

### AGENTS.md Rules
- `RULE-S27-01`: Major features must register risk items before merge. Enforcement: feature flag metadata lint.
- `RULE-S27-02`: Risk thresholds require owner assignment. Enforcement: release checklist validator.

## 28. Plan Evolution and Changelog
### Design Decisions
- [v12] v11 weakness identified: plan evolution mixed rationale and chronology; v12 separates decision records from chronological changelog entries.
- [v12] Architectural changes require concise ADR entries linked from changelog.
- [v12] Changelog entries are machine-parseable and include gate impact tags.
- [v12] Removed complexity from v11 is explicitly recorded to prevent reintroduction.
- [v12] v12 marks all deltas from v11 with `[v12]` tags.

### Rust Example
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangelogEntry {
    pub version: &'static str,
    pub summary: String,
    pub gate_tags: Vec<&'static str>,
}

pub fn has_gate_tag(entry: &ChangelogEntry, tag: &str) -> bool {
    entry.gate_tags.iter().any(|t| *t == tag)
}
```

### Test Strategy
- [v12] Lint validates changelog entry schema.
- [v12] ADR links are checked for existence.
- [v12] Diff tests verify removal rationale present for deleted subsystems.

### AGENTS.md Rules
- `RULE-S28-01`: Breaking architectural changes require ADR + changelog entry. Enforcement: PR policy check.
- `RULE-S28-02`: Gate impact tags are mandatory on release notes. Enforcement: changelog linter.

## 29. Reference Anchors
### Design Decisions
- [v12] v11 weakness identified: anchors existed but were not audited for staleness; v12 requires automated cross-reference verification.
- [v12] Each section has a stable anchor key `Snn`.
- [v12] Cross-references must target existing sections only.
- [v12] Version-specific references must include lane/version qualifiers.
- [v12] Reference graph is exported for tooling and docs.

### Rust Example
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Anchor {
    pub key: String,
}

pub fn valid_anchor(key: &str) -> bool {
    let bytes = key.as_bytes();
    bytes.len() == 3 && bytes[0] == b'S' && bytes[1].is_ascii_digit() && bytes[2].is_ascii_digit()
}
```

### Test Strategy
- [v12] Link checker validates intra-doc references.
- [v12] Anchor uniqueness test across full document.
- [v12] Version qualifier tests on compatibility references.

### AGENTS.md Rules
- `RULE-S29-01`: Section references must use valid `Snn` anchors. Enforcement: docs lint.
- `RULE-S29-02`: Broken cross-references block merge. Enforcement: link check CI.

## 30. Appendix: Canonical Type Quick Reference
### Design Decisions
- [v12] v11 weakness identified: canonical types were scattered and occasionally inconsistent; v12 centralizes authoritative type definitions and aliases.
- [v12] Public canonical types are minimal and stable.
- [v12] Internal aliases are allowed but must map back to canonical definitions.
- [v12] FFI schemas reference canonical types directly.
- [v12] Type deprecations are phased and documented.

### Rust Example
```rust
pub type SessionId = u64;
pub type WindowId = u64;
pub type PaneId = u64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalIds {
    pub session: SessionId,
    pub window: WindowId,
    pub pane: PaneId,
}
```

### Test Strategy
- [v12] Type alias consistency tests across crates and bindings.
- [v12] API snapshot tests catch accidental type widening/narrowing.
- [v12] Deprecation tests verify warnings and migration paths.

### AGENTS.md Rules
- `RULE-S30-01`: Public IDs must map to canonical type aliases. Enforcement: API lint.
- `RULE-S30-02`: Type changes require binding schema regeneration. Enforcement: schema hash check.

## 31. Supplemental Test Matrix
### Design Decisions
- [v12] v11 weakness identified: matrix dimensions were broad but sparse; v12 defines mandatory dimensions and minimum coverage counts.
- [v12] Dimensions: OS, architecture, tmux version lane, shell, locale, terminal type, and binding runtime.
- [v12] Matrix includes Rust unit/integration, Python pytest snapshots, Node vitest snapshots.
- [v12] Flaky tests are quarantined with expiry and owner.
- [v12] Minimum matrix must run on every PR; expanded matrix runs nightly.

### Rust Example
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatrixCase {
    pub os: &'static str,
    pub shell: &'static str,
    pub lane: &'static str,
}

pub fn case_key(c: &MatrixCase) -> String {
    format!("{}:{}:{}", c.os, c.shell, c.lane)
}
```

### Test Strategy
- [v12] PR matrix validates critical dimensions only.
- [v12] Nightly matrix validates full combinatorial coverage tiers.
- [v12] Snapshot artifact retention keeps failed outputs for triage.

### AGENTS.md Rules
- `RULE-S31-01`: PR pipeline must execute minimum matrix set. Enforcement: workflow required checks.
- `RULE-S31-02`: Quarantined tests need expiry date and owner. Enforcement: quarantine manifest linter.

## 32. Termlets
### Design Decisions
#### 32.1 Scope and Product Positioning
- [v12] Termlets are the killer feature: programmable test/execution pods that attach to TermForge and tmux-compatible semantics.
- [v12] v11 weakness identified: termlet boundaries were expansive but under-constrained; v12 defines strict pod lifecycle and API contracts.
- [v12] Termlets are SDK-first: identical orchestration primitives are available in Rust, Python, and Node.

#### 32.2 Core Abstractions
- [v12] `TermletTemplate`: immutable definition (image/runtime profile, shell bootstrap, env policy).
- [v12] `TermletInstance`: running pod with stable ID and event stream.
- [v12] `TermletSessionHandle`: high-level ORM entry for querying windows/panes and issuing interactions.
- [v12] `VisualCapture`: cell-grid capture plus optional ANSI byte stream.

#### 32.3 Lifecycle State Machine
- [v12] States: `Planned -> Provisioning -> Running -> Draining -> Stopped -> Collected`.
- [v12] Illegal transitions are rejected with `policy` errors.
- [v12] Teardown is mandatory and idempotent.

#### 32.4 Interaction Model
- [v12] APIs: `spawn_shell`, `send_keys`, `resize`, `capture_area`, `wait_for`, `expect_text`, `expect_cells`.
- [v12] Interaction commands are timestamped and causation-linked to captures.
- [v12] Wait semantics support deterministic polling windows to avoid flaky tests.

#### 32.5 Visual Area Capture
- [v12] Capture modes: `cells`, `ansi`, `both`.
- [v12] Cell captures include grapheme cluster metadata for robust Unicode assertions.
- [v12] Captures can be scoped to pane, window, or viewport rectangle.

#### 32.6 Snapshot Testing Across Languages
- [v12] Rust: snapshot crates with stable normalization filters.
- [v12] Python: `pytest` plugin with fixture-managed termlet lifecycle.
- [v12] Node: `vitest` helper with async lifecycle hooks.
- [v12] Snapshot format is shared JSON schema to enable cross-language diffs.

#### 32.7 Termlet + CRDT Collaboration
- [v12] Collaborative termlets can connect multiple writers with CRDT-backed intent streams.
- [v12] Merge conflicts materialize as explicit review artifacts in captures/logs.
- [v12] Deterministic merge replay is required for every collaborative scenario.

#### 32.8 Performance and Isolation
- [v12] Termlet pods must use isolated `socket_name` and `socket_path` outside defaults.
- [v12] Resource caps (CPU/memory/fd) are template-level policies.
- [v12] Test harness cleans all pod resources even on panic.

#### 32.9 Security and Policy
- [v12] Environment passthrough is deny-by-default; allowlists are explicit.
- [v12] File mounts use read-only default with explicit writable exceptions.
- [v12] Network access policy is profile-based and auditable.

#### 32.10 SDK Orthogonality Guarantees
- [v12] Every termlet operation composes independently with query APIs and CRDT transactions.
- [v12] API avoids hidden global context; handles carry explicit scope.
- [v12] Binding APIs preserve method names and semantic ordering.

#### 32.11 Observability
- [v12] Each termlet run has root trace and structured run summary.
- [v12] Capture, resize, and assertion operations emit spans with correlation IDs.
- [v12] Failure diagnostics bundle spans, logs, captures, and command transcript.

#### 32.12 Simplifications from v11
- [v12] Removed optional implicit lifecycle shortcuts that produced hidden state transitions.
- [v12] Removed multi-format snapshot drift by standardizing a single canonical schema.
- [v12] Reduced adapter indirection by keeping one execution trait per runtime profile.

### Rust Example
#### 32.13 Compile-Safe Termlet Skeleton
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TermletState {
    Planned,
    Provisioning,
    Running,
    Draining,
    Stopped,
    Collected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TermletId(pub String);

#[derive(Debug, Clone)]
pub struct TermletInstance {
    pub id: TermletId,
    pub state: TermletState,
    pub cols: u16,
    pub rows: u16,
}

impl TermletInstance {
    pub fn spawn(id: impl Into<String>, cols: u16, rows: u16) -> Self {
        Self {
            id: TermletId(id.into()),
            state: TermletState::Running,
            cols,
            rows,
        }
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.cols = cols;
        self.rows = rows;
    }

    pub fn drain(&mut self) {
        self.state = TermletState::Draining;
    }

    pub fn stop(&mut self) {
        self.state = TermletState::Stopped;
    }
}

pub fn legal_transition(from: TermletState, to: TermletState) -> bool {
    matches!(
        (from, to),
        (TermletState::Planned, TermletState::Provisioning)
            | (TermletState::Provisioning, TermletState::Running)
            | (TermletState::Running, TermletState::Draining)
            | (TermletState::Draining, TermletState::Stopped)
            | (TermletState::Stopped, TermletState::Collected)
    )
}
```

#### 32.14 Cross-Language Snapshot Schema Sketch
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    pub ch: String,
    pub fg: String,
    pub bg: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisualCapture {
    pub cols: u16,
    pub rows: u16,
    pub cells: Vec<Cell>,
}

pub fn expected_cell_count(c: &VisualCapture) -> usize {
    (c.cols as usize) * (c.rows as usize)
}
```

### Test Strategy
#### 32.15 Lifecycle and Policy Tests
- [v12] State-machine tests enforce legal transitions and idempotent teardown.
- [v12] Isolation tests validate unique non-default sockets and full cleanup.
- [v12] Resource policy tests verify caps and deny-by-default env/network rules.

#### 32.16 Interaction and Visual Validation
- [v12] Deterministic interaction tests run `spawn -> send_keys -> resize -> capture` pipelines.
- [v12] Unicode visual tests validate grapheme-aware captures.
- [v12] Retry-window tests ensure `wait_for` behavior avoids flaky timing.

#### 32.17 Cross-Language Snapshot Parity
- [v12] Rust, pytest, and vitest suites consume identical snapshot schema.
- [v12] Normalization filters are versioned and applied consistently.
- [v12] Snapshot drift reports include first-diff coordinates and ANSI context.

#### 32.18 Collaboration and CRDT Tests
- [v12] Multi-writer simulations verify deterministic merged captures.
- [v12] Partition/rejoin tests validate merge replay determinism.
- [v12] Conflict materialization tests ensure review artifacts are emitted.

### AGENTS.md Rules
- `RULE-S32-01`: Termlets must run with isolated `socket_name`/`socket_path` outside tmux defaults. Enforcement: harness runtime assert + leak checker.
- `RULE-S32-02`: Termlet APIs must remain orthogonal and chain-safe across Rust/Python/Node. Enforcement: shared API contract tests.
- `RULE-S32-03`: Every termlet feature needs Rust + pytest + vitest coverage with shared fixtures. Enforcement: multi-runtime CI gate.
- `RULE-S32-04`: Visual capture assertions must support cell-grid mode for deterministic snapshots. Enforcement: snapshot schema validator.
- `RULE-S32-05`: Collaborative termlet flows must pass CRDT deterministic replay tests. Enforcement: merge-order simulation suite.

## Final Consistency Checklist
- [v12] Exactly 32 numbered sections exist (`## 1` through `## 32`).
- [v12] Each numbered section contains exactly these four third-level headings: `### Design Decisions`, `### Rust Example`, `### Test Strategy`, `### AGENTS.md Rules`.
- [v12] Section 32 is the deepest section and includes additional `####` subsections.
- [v12] Rule naming uses `RULE-Snn-xx` with explicit enforcement language.
- [v12] All declared architectural changes from v11 are tagged with `[v12]`.
- [v12] Cross-reference approach is anchored via `Snn` section keys and link checks.
- [v12] Required user priorities are covered: tmux compatibility, ORM-like SDK, bindings, CRDT, OTEL, test isolation, mux-builder/mux-vm, termlets, snapshots, orthogonality, permissive licensing.
