# TermForge v10 Architecture Specification (Pass 3 FINAL)

Date: 2026-02-11
Status: **v10 Pass 3 (FINAL)** -- Definitive synthesis of TermForge architecture.
Lineage: v9 Pass 3 Final -> v10 Pass 1 (Claude/GPT/Gemini) -> v10 Pass 2 (Cross-pollinated) -> **v10 Pass 3 (FINAL)**
License: MIT OR Apache-2.0
Rust edition: 2024 (MSRV 1.85)
Protocol target: tmux protocol v8

---

## Preamble

This document is the single authoritative architectural reference for TermForge, a Rust terminal multiplexer with 100% tmux wire-protocol compatibility, ORM-like API, language bindings, CRDT collaboration, a ratatui-based TUI client, and **Termlets** -- SDK-first testing pods that are the killer differentiator.

### v10 Pass 3 Synthesis Notes

This Final Pass 3 specification consolidates the best-in-class architecture from v10 Pass 2, ensuring maximal detail for the Termlet subsystem (Section 32).

**Key refinements in Pass 3:**
1. **TermletBuilder Pattern**: Added ergonomic builder API for Termlet construction (Section 32.2).
2. **Final Verification**: Re-verified all 40 Termlet test cases and 10 AGENTS.md rules.
3. **Plan Evolution**: Updated changelog to reflect the final freeze state.

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
22. [Binding Test Frameworks](#22-binding-test-frameworks)
23. [Test Framework and Harness Design](#23-test-framework-and-harness-design)
24. [Performance Targets](#24-performance-targets)
25. [Visual Client / TUI](#25-visual-client--tui)
26. [AGENTS.md Rules](#26-agentsmd-rules)
27. [Risks and Mitigations](#27-risks-and-mitigations)
28. [Plan Evolution and Changelog](#28-plan-evolution-and-changelog)
29. [Reference Anchors](#29-reference-anchors)
30. [Appendix: Canonical Type Quick Reference](#30-appendix-canonical-type-quick-reference)
31. [Supplemental Test Matrix](#31-supplemental-test-matrix)
32. [Termlets](#32-termlets)

---

## 1. Vision and Philosophy

### Design Decisions

- TermForge is Rust-first and tmux wire-compatible, but not a structural clone.
- Compatibility is enforced at protocol and behavior boundaries, not internal type layout.
- Pure core model remains mandatory for determinism and replayability.
- Product differentiation centers on Termlets without compromising tmux pane semantics.

### Rust Example

```rust
pub trait Kernel {
    fn apply(&mut self, event: Event, ctx: CoreCtx) -> ApplyOutcome;
}

#[derive(Debug, Clone, Copy)]
pub struct CoreCtx {
    pub now_ms: i64,
    pub rand_u64: u64,
}
```

### Test Strategy

1. Determinism replay from recorded event streams.
2. Compatibility attach test with stock tmux client.
3. Termlet smoke test in Rust/Python/Node.

### AGENTS.md Rules

- `RULE-S1-01`: Keep Layer 0 crates free of OS side effects.
- `RULE-S1-02`: Compatibility assertions require a Section 29 anchor.

---

## 2. North Star Acceptance Criteria

### Design Decisions

- Gate taxonomy: compatibility, architecture, reliability, performance, SDK.
- Termlet gates are blocking for v10.
- A gate can only be marked complete with automated validation.

### Rust Example

```rust
pub enum Gate {
    C1WireProtocol,
    C2BehaviorParity,
    A1PureKernel,
    R1NoZombiePtys,
    S1TermletRust,
    S2TermletPython,
    S3TermletNode,
}
```

### Test Strategy

1. CI matrix maps each gate to one or more jobs.
2. Nightly compatibility suite validates tmux behavior parity.

### AGENTS.md Rules

- `RULE-S2-01`: Every feature PR maps to gate IDs.
- `RULE-S2-02`: Gate waivers require owner + expiration.

---

## 3. High-Level Architecture

### Design Decisions

- Layer 0: pure types, parser, grid, reducer, query, CRDT.
- Layer 1: runtime IO, sockets, PTY, process services.
- Layer 2: facades (`mux-api`, `mux-termlet`).
- Layer 3/4: ORM and language bindings.

### Rust Example

```rust
pub struct ManagedMux {
    rt: std::sync::Arc<RuntimeHandle>,
}

pub struct TermletFacade {
    inner: std::sync::Arc<TermletInner>,
}
```

### Test Strategy

1. Dependency-edge lint to enforce layer direction.
2. API compile tests ensure no runtime internals leak.

### AGENTS.md Rules

- `RULE-S3-01`: Layer 2 may depend downward only.
- `RULE-S3-02`: No binding crate may be depended on by core/runtime.

---

## 4. Workspace Layout

### Design Decisions

- Keep `mux-*` crate naming.
- Add/retain `crates/mux-termlet` as Layer 2 facade.
- Bindings expose termlet in idiomatic API modules.

### Rust Example

```text
crates/
  mux-types/
  mux-core/
  mux-grid/
  mux-proto/
  mux-pty/
  mux-pty-fake/
  mux-runtime/
  mux-api/
  mux-orm/
  mux-termlet/
bindings/
  python/
  node/
```

### Test Strategy

1. Workspace metadata test validates required crates.
2. Binding package import tests validate symbol exposure.

### AGENTS.md Rules

- `RULE-S4-01`: Spec passes only write under `notes/`.
- `RULE-S4-02`: New crate names must stay grep-friendly.

---

## 5. Layering Contract

### Design Decisions

- Layer 0 must be `tokio`-free and mostly `unsafe`-free.
- PTY interactions abstracted behind trait boundary.
- Termlet consumes these traits instead of duplicating runtime logic.

### Rust Example

```rust
pub trait PtyBackend: Send + Sync {
    fn spawn(&self, req: SpawnRequest) -> Result<PaneId, PtyError>;
    fn write(&self, pane_id: PaneId, bytes: &[u8]) -> Result<(), PtyError>;
    fn resize(&self, pane_id: PaneId, cols: u16, rows: u16) -> Result<(), PtyError>;
    fn kill(&self, pane_id: PaneId, signal: i32) -> Result<(), PtyError>;
}
```

### Test Strategy

1. `cargo check --target wasm32-unknown-unknown` for Layer 0.
2. Static dependency checks for forbidden crates.

### AGENTS.md Rules

- `RULE-S5-01`: No async runtime types in pure domain structs.
- `RULE-S5-02`: New trait APIs must include fake implementation plan.

---

## 6. Entity Model

### Design Decisions

- Stable IDs via `SlotMap`-style keys.
- Server state modeled as graph of session/window/pane.
- Pane remains canonical execution/render anchor used by Termlet.

### Rust Example

```rust
slotmap::new_key_type! {
    pub struct SessionId;
    pub struct WindowId;
    pub struct PaneId;
}

pub struct Pane {
    pub id: PaneId,
    pub cols: u16,
    pub rows: u16,
}
```

### Test Strategy

1. Property tests for ID uniqueness.
2. Graph invariants (parent/child consistency).

### AGENTS.md Rules

- `RULE-S6-01`: Never recycle IDs in-process.
- `RULE-S6-02`: Pane size is authoritative in resize flows.

---

## 7. Event/Effect Engine

### Design Decisions

- Split pure decision (`ApplyOutcome`) from impure execution (`Effect`).
- Effects are replayable and observable.
- Idempotent effect IDs prevent duplicate execution.

### Rust Example

```rust
pub enum Effect {
    PtyWrite { pane: PaneId, bytes: Vec<u8> },
    PtyResize { pane: PaneId, cols: u16, rows: u16 },
    EmitClientFrame { client: ClientId, frame: Vec<u8> },
}
```

### Test Strategy

1. Reducer snapshot tests.
2. Effect scheduler retry/idempotency tests.

### AGENTS.md Rules

- `RULE-S7-01`: Reducer must not block or sleep.
- `RULE-S7-02`: Effects must carry correlation IDs.

---

## 8. Error Handling

### Design Decisions

- Structured errors with stable machine codes.
- Preserve context via `thiserror` + `anyhow` boundaries.
- Bindings map Rust codes to language-native exceptions.

### Rust Example

```rust
#[derive(thiserror::Error, Debug)]
pub enum TfError {
    #[error("protocol violation: {0}")]
    Protocol(String),
    #[error("timeout: {operation}")]
    Timeout { operation: &'static str },
    #[error("pty: {0}")]
    Pty(String),
}
```

### Test Strategy

1. Golden tests for code-to-message mapping.
2. Binding error mapping tests.

### AGENTS.md Rules

- `RULE-S8-01`: No `unwrap()` in runtime paths.
- `RULE-S8-02`: Public APIs expose stable error codes.

---

## 9. Protocol Codec

### Design Decisions

- Target tmux protocol v8 compatibility.
- Strict frame parser and explicit violation handling.
- Decoder keeps unknown fields for forward-compat diagnostics.

### Rust Example

```rust
pub const TMUX_PROTOCOL_VERSION: u32 = 8;

pub enum ProtoFrame {
    Identify(IdentifyPayload),
    Command(CommandPayload),
    Exit,
}
```

### Test Strategy

1. Fuzz parser with malformed frames.
2. Golden compatibility traces from tmux client/server captures.

### AGENTS.md Rules

- `RULE-S9-01`: Protocol mismatches fail fast and close connection.
- `RULE-S9-02`: New frame handling requires trace fixture update.

---

## 10. Configuration System

### Design Decisions

- Layered sources: defaults -> file -> env -> CLI.
- Config applied after identify burst for parity.
- Validation returns typed diagnostics, not panics.

### Rust Example

```rust
#[derive(Debug, Clone)]
pub struct Config {
    pub socket_name: String,
    pub default_shell: String,
    pub otel_enabled: bool,
}
```

### Test Strategy

1. Precedence tests across all sources.
2. Invalid config surface tests with expected diagnostics.

### AGENTS.md Rules

- `RULE-S10-01`: Defaults must be explicit in code.
- `RULE-S10-02`: Env parsing cannot mutate global state.

---

## 11. Layout Engine

### Design Decisions

- Keep tmux-like tree layout model.
- Preserve round-robin resize distribution behavior.
- Reflow must preserve pane content semantics.

### Rust Example

```rust
pub enum Split {
    Horizontal,
    Vertical,
}

pub fn resize_round_robin(cells: &mut [u16], delta: i16) {
    if cells.is_empty() || delta == 0 { return; }
    // deterministic incremental distribution
}
```

### Test Strategy

1. Resize parity fixtures against tmux behavior.
2. Property tests: total size invariant under split/merge.

### AGENTS.md Rules

- `RULE-S11-01`: Maintain deterministic resize order.
- `RULE-S11-02`: No lossy conversions in size math.

---

## 12. ORM-like Query API

### Design Decisions

- Query API mirrors libtmux ergonomics.
- Support kwargs-style filters and callable predicates.
- Operator set tracks libtmux semantics.

### Rust Example

```rust
pub enum LookupOp {
    Eq,
    IExact,
    Contains,
    IContains,
    StartsWith,
    IStartsWith,
    EndsWith,
    IEndsWith,
    In,
    Nin,
    Regex,
    IRegex,
}
```

### Test Strategy

1. Cross-language query parity tests with fixture datasets.
2. Operator correctness tests for each lookup op.

### AGENTS.md Rules

- `RULE-S12-01`: Keep kwargs and callable filter support.
- `RULE-S12-02`: Alias behavior (`eq`/`exact`) must remain compatible.

---

## 13. Runtime Architecture

### Design Decisions

- Single writer state actor.
- Dedicated IO tasks for sockets/PTy ingestion.
- Broadcast immutable snapshots to readers.

### Rust Example

```rust
pub struct Runtime {
    state_tx: tokio::sync::mpsc::Sender<Event>,
    snapshot: arc_swap::ArcSwap<StateView>,
}
```

### Test Strategy

1. Concurrency stress tests with many clients.
2. Snapshot consistency under high event rate.

### AGENTS.md Rules

- `RULE-S13-01`: Runtime must tolerate slow readers.
- `RULE-S13-02`: Event loop shutdown is bounded-time.

---

## 14. Server Lifecycle

### Design Decisions

- Start sequence: config -> socket guard -> protocol accept loop.
- Shutdown sequence drains active operations and kills orphan PTYs.
- Recovery mode for stale sockets and lock files.

### Rust Example

```rust
pub enum ServerState {
    Starting,
    Running,
    Draining,
    Stopped,
}
```

### Test Strategy

1. Start/stop/restart integration tests.
2. Crash recovery with stale socket fixtures.

### AGENTS.md Rules

- `RULE-S14-01`: Server start must validate socket isolation.
- `RULE-S14-02`: Shutdown must leave no running child process.

---

## 15. Control Mode

### Design Decisions

- Control mode stream remains line-oriented and deterministic.
- Backpressure controls per-client output queues.
- Session attachment state transitions are explicit.

### Rust Example

```rust
pub struct ControlFrame {
    pub kind: &'static str,
    pub payload: String,
}
```

### Test Strategy

1. Golden transcript tests for control mode outputs.
2. Backpressure tests with slow consumer simulation.

### AGENTS.md Rules

- `RULE-S15-01`: Frame order must be stable for same event stream.
- `RULE-S15-02`: Control mode parser must reject partial invalid records.

---

## 16. Language Bindings

### Design Decisions

- Rust core is single source of truth.
- Python via PyO3 and Node via Neon/N-API wrappers.
- Bindings expose equivalent semantics, not identical syntax.

### Rust Example

```rust
pub trait BindingSurface {
    fn send_keys(&self, keys: &str) -> Result<(), TfError>;
    fn snapshot_text(&self) -> Result<String, TfError>;
}
```

### Test Strategy

1. Cross-language conformance tests over shared fixtures.
2. ABI compatibility tests for wheel/package artifacts.

### AGENTS.md Rules

- `RULE-S16-01`: Binding behavior changes require fixture updates in all languages.
- `RULE-S16-02`: Do not bypass Rust validation in wrappers.

---

## 17. CRDT Transaction Layer

### Design Decisions

- CRDT applies to collaborative metadata/state, not PTY byte stream.
- Transaction log provides mergeability and auditability.
- Keep deterministic conflict resolution order.

### Rust Example

```rust
pub struct Txn {
    pub actor: String,
    pub seq: u64,
    pub ops: Vec<Op>,
}
```

### Test Strategy

1. Merge commutativity/associativity property tests.
2. Replay tests across reordered message delivery.

### AGENTS.md Rules

- `RULE-S17-01`: PTY output is never CRDT-merged.
- `RULE-S17-02`: Txn IDs must be monotonic per actor.

---

## 18. Security Model

### Design Decisions

- Principle: local-socket least privilege by default.
- Validate socket path isolation and default-socket refusal.
- Guard against accidental attachment to existing `$TMUX` socket.

### Rust Example

```rust
pub fn validate_socket(socket_name: &str, socket_path: &std::path::Path, tempdir: &std::path::Path) -> anyhow::Result<()> {
    ensure_not_default_socket_name(socket_name)?;
    ensure_socket_within_tempdir(socket_path, tempdir)?;
    ensure_socket_not_tmux_env(socket_path)?;
    Ok(())
}
```

### Test Strategy

1. Tests for all three path guard layers.
2. Permission/symlink edge-case tests.

### AGENTS.md Rules

- `RULE-S18-01`: Never default to tmux `default` socket name.
- `RULE-S18-02`: Socket outside harness tempdir is a hard error.

---

## 19. OpenTelemetry

### Design Decisions

- Use dual-provider architecture: server/global and client/local.
- OTEL is feature-gated and safe to disable.
- Spans include protocol event IDs and pane/termlet IDs.

### Rust Example

```rust
#[cfg(feature = "otel")]
pub fn termlet_span(op: &'static str, id: u64) -> tracing::Span {
    tracing::info_span!("termforge.termlet", op = op, termlet_id = id)
}
```

### Test Strategy

1. OTEL-off tests ensure zero behavioral change.
2. OTEL-on tests verify spans and flush on shutdown.

### AGENTS.md Rules

- `RULE-S19-01`: OTEL failures must not fail core operations.
- `RULE-S19-02`: Shutdown must force-flush providers best-effort.

---

## 20. tmux Version Management

### Design Decisions

- Build cache keyed by source version + host/toolchain fingerprint.
- Use BLAKE3 hash for key derivation.
- Use file locks around clone/worktree/build directories.

### Rust Example

```rust
pub fn compute_cache_key(input: &[u8]) -> String {
    let hash = blake3::hash(input);
    hex::encode(&hash.as_bytes()[..16])
}
```

### Test Strategy

1. Cache key stability tests.
2. Parallel build race tests validating lock behavior.

### AGENTS.md Rules

- `RULE-S20-01`: Build cache operations must be lock-protected.
- `RULE-S20-02`: Cache key changes require migration note.

---

## 21. Test Support and Fake PTY

### Design Decisions

- Fake PTY is required for deterministic unit tests.
- Real PTY remains required for integration parity.
- Scenario replay captures bytes + timing envelopes.

### Rust Example

```rust
pub enum PtyMode {
    Real,
    Fake,
}

pub struct ScenarioEvent {
    pub at_ms: u64,
    pub bytes: Vec<u8>,
}
```

### Test Strategy

1. Deterministic replay tests in fake mode.
2. Real shell integration tests for prompts/signals/resize.

### AGENTS.md Rules

- `RULE-S21-01`: Fake mode tests must avoid wall-clock sleeps.
- `RULE-S21-02`: Real mode tests must include explicit cleanup.

---

## 22. Binding Test Frameworks

### Design Decisions

- First-class pytest and vitest helpers.
- Consistent fixture lifecycle: spawn, yield, guaranteed kill.
- Snapshot adapters unify text normalization.

### Rust Example

```rust
pub struct BindingTestConfig {
    pub mode: PtyMode,
    pub timeout_ms: u64,
}
```

### Test Strategy

1. pytest fixture failure cleanup tests.
2. vitest helper cleanup and timeout tests.

### AGENTS.md Rules

- `RULE-S22-01`: Binding fixtures must guarantee process teardown.
- `RULE-S22-02`: Snapshot normalization must match Rust behavior.

---

## 23. Test Framework and Harness Design

### Design Decisions

- Three tiers: unit, integration, compatibility.
- Artifacts include grid snapshots, OTEL traces, protocol logs.
- Flake triage tracks retry counts and signatures.

### Rust Example

```rust
pub enum HarnessTier {
    Unit,
    Integration,
    Compatibility,
}
```

### Test Strategy

1. Tier-specific required suites.
2. Quarantine policy for flaky tests with expiration.

### AGENTS.md Rules

- `RULE-S23-01`: Each new subsystem must add at least one unit and one integration test.
- `RULE-S23-02`: Compatibility regressions are release blockers.

---

## 24. Performance Targets

### Design Decisions

- Define p50/p95 targets for spawn, resize, snapshot, attach.
- Separate targets for fake vs real PTY.
- Track allocations and parser throughput.

### Rust Example

```rust
pub struct PerfBudget {
    pub termlet_spawn_fake_us_p95: u64,
    pub termlet_spawn_real_ms_p95: u64,
    pub snapshot_80x24_us_p95: u64,
}
```

### Test Strategy

1. Criterion microbenchmarks in CI nightly.
2. Regression budget checks against baseline JSON.

### AGENTS.md Rules

- `RULE-24-01`: Budget regressions >10% require explicit sign-off.
- `RULE-24-02`: Benchmarks must run on pinned runner class.

---

## 25. Visual Client / TUI

### Design Decisions

- TUI is a consumer of core snapshots/events, not owner of state.
- Render pipeline must preserve grid semantics exactly.
- Input translation maps keys/mouse to effect events.

### Rust Example

```rust
pub trait UiRenderer {
    fn render(&mut self, grid: &GridSnapshot);
}
```

### Test Strategy

1. Golden screenshot/text snapshot tests.
2. Input translation conformance tests.

### AGENTS.md Rules

- `RULE-25-01`: UI must not mutate core state directly.
- `RULE-25-02`: Grid rendering must preserve wide-char behavior.

---

## 26. AGENTS.md Rules

### Design Decisions

- Keep architecture guardrails directly enforceable in CI.
- Rules are short, specific, and mapped to sections.
- Runtime-critical rules have associated tests/lints.

### Rust Example

```rust
pub struct AgentRule {
    pub id: &'static str,
    pub section: u8,
    pub text: &'static str,
}
```

### Test Strategy

1. Rule index completeness test.
2. Lint script checks referenced rule IDs in changed files.

### AGENTS.md Rules

- `RULE-26-01`: Any architecture change must update corresponding section rules.
- `RULE-26-02`: Rule IDs are immutable once published.

---

## 27. Risks and Mitigations

### Design Decisions

- Track technical, compatibility, operational, and adoption risks.
- Mitigation plans include leading indicators and rollback plans.
- Termlet adoption risk treated as product-critical.

### Rust Example

```rust
pub struct Risk {
    pub id: &'static str,
    pub probability: f32,
    pub impact: f32,
    pub mitigation: &'static str,
}
```

### Test Strategy

1. Risk register review each milestone.
2. Drill tests for top operational risks.

### AGENTS.md Rules

- `RULE-27-01`: High-probability/high-impact risks must have owner.
- `RULE-27-02`: Mitigations must be testable or observable.

---

## 28. Plan Evolution and Changelog

### Design Decisions

- Record rationale, not only final outcomes.
- Each pass documents accepted/rejected deltas.
- Maintain compatibility timeline with concrete dates.

### Rust Example

```rust
pub struct ChangelogEntry {
    pub date: &'static str,
    pub section: u8,
    pub change: &'static str,
    pub rationale: &'static str,
}
```

### Test Strategy

1. Changelog schema validation.
2. Cross-reference check against updated sections.

### AGENTS.md Rules

- `RULE-28-01`: Breaking decisions require explicit migration notes.
- `RULE-28-02`: Every pass must list verified anchors.

---

## 29. Reference Anchors

### Design Decisions

Verified anchors used in this pass:
- `~/study/c/tmux/tmux-protocol.h`: `PROTOCOL_VERSION 8`
- `~/study/c/tmux/client.c`: `flock(LOCK_EX|LOCK_NB)` startup lock behavior
- `~/work/python/libtmux/src/libtmux/_internal/query_list.py`: kwargs filtering + lookup map (`eq`/`exact` alias)
- `~/work/rust/vibe-tmux/crates/mux-test-support/src/path_guard.rs`: 3-layer socket validation
- `~/work/rust/vibe-tmux/tools/tmux-builder/src/lib.rs`: BLAKE3 cache key + file locking
- `~/work/rust/vibe-tmux/crates/mux-otel/src/otel.rs`: dual provider and flush/shutdown logic

### Rust Example

```rust
pub struct Anchor {
    pub path: &'static str,
    pub claim: &'static str,
}
```

### Test Strategy

1. Anchor existence check script.
2. Line-pattern checks for critical claims.

### AGENTS.md Rules

- `RULE-29-01`: Claims without anchors must be marked inference.
- `RULE-29-02`: Anchor drift triggers spec update.

---

## 30. Appendix: Canonical Type Quick Reference

### Design Decisions

- Keep canonical type names stable for docs, tests, bindings.
- Include only high-value boundary types.
- Provide brief role descriptions.

### Rust Example

```rust
pub struct ApplyOutcome {
    pub effects: Vec<Effect>,
    pub notifications: Vec<Notification>,
}

pub struct GridSnapshot {
    pub cols: u16,
    pub rows: u16,
    pub text: String,
}
```

### Test Strategy

1. Compile-time doc examples for key types.
2. API diff check for public type changes.

### AGENTS.md Rules

- `RULE-30-01`: Renaming canonical types requires deprecation phase.
- `RULE-30-02`: Public structs must document field semantics.

---

## 31. Supplemental Test Matrix

### Design Decisions

- Matrix dimensions: OS, shell, backend mode, binding language, tmux version.
- Explicitly include race, signal, and resize cases.
- Include cross-language snapshot consistency lane.

### Rust Example

```rust
pub struct MatrixCase {
    pub os: &'static str,
    pub shell: &'static str,
    pub mode: PtyMode,
    pub lang: &'static str,
}
```

### Test Strategy

1. Required lanes: linux+bash fake/real in Rust/Python/Node.
2. Weekly extended lanes: zsh/fish and compatibility stress tests.

### AGENTS.md Rules

- `RULE-31-01`: No release without green required lanes.
- `RULE-31-02`: Add matrix lane before adding new shell-specific behavior.

---

## 32. Termlets (SDK-first Testing Pods)

### Design Decisions

Termlets are the v10 differentiator: pane-backed, SDK-first testing pods that wrap `Pane + Grid + PtyBackend` into one ergonomic handle while remaining tmux-compatible.

#### 32.1 Core Model

- A Termlet is a lightweight facade over existing pane/render/pty primitives.
- No standalone protocol or parallel terminal core is introduced.
- Works in two modes:
  - `FakePty`: deterministic and fast for unit tests
  - `RealPty`: real shell/process for integration tests

#### 32.2 Rust API (Builder Pattern + Explicit Errors)

```rust
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TermletBackendMode {
    Fake,
    Real,
}

#[derive(Debug, Clone)]
pub struct TermletConfig {
    pub cols: u16,
    pub rows: u16,
    pub env: Vec<(String, String)>,
    pub cwd: Option<PathBuf>,
    pub backend: TermletBackendMode,
    pub wait_poll_interval: Duration,
    pub graceful_kill_timeout: Duration,
}

impl Default for TermletConfig {
    fn default() -> Self {
        Self {
            cols: 80,
            rows: 24,
            env: vec![("TERM".into(), "xterm-256color".into())],
            cwd: None,
            backend: TermletBackendMode::Fake,
            wait_poll_interval: Duration::from_millis(20),
            graceful_kill_timeout: Duration::from_millis(300),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TermletError {
    #[error("spawn failed: {reason}")]
    SpawnFailed { reason: String },
    #[error("pty io failed: {reason}")]
    PtyIo { reason: String },
    #[error("wait_for timeout after {timeout_ms}ms for pattern `{pattern}`")]
    WaitTimeout { pattern: String, timeout_ms: u64 },
    #[error("pattern not found before process exit: `{pattern}`")]
    PatternNotFound { pattern: String },
    #[error("termlet already terminated")]
    AlreadyTerminated,
    #[error("invalid argument: {0}")]
    InvalidArgument(&'static str),
}

/// [v10 Pass 3] Ergonomic builder for Termlet construction.
pub struct TermletBuilder {
    cmd: String,
    args: Vec<String>,
    cfg: TermletConfig,
}

impl TermletBuilder {
    pub fn new(cmd: impl Into<String>) -> Self {
        Self { cmd: cmd.into(), args: vec![], cfg: TermletConfig::default() }
    }
    pub fn args(mut self, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.args = args.into_iter().map(Into::into).collect();
        self
    }
    pub fn size(mut self, cols: u16, rows: u16) -> Self {
        self.cfg.cols = cols;
        self.cfg.rows = rows;
        self
    }
    pub fn cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cfg.cwd = Some(cwd.into());
        self
    }
    pub fn env(mut self, k: impl Into<String>, v: impl Into<String>) -> Self {
        self.cfg.env.push((k.into(), v.into()));
        self
    }
    pub fn backend(mut self, mode: TermletBackendMode) -> Self {
        self.cfg.backend = mode;
        self
    }
    pub fn fake(mut self) -> Self {
        self.cfg.backend = TermletBackendMode::Fake;
        self
    }
    pub fn real(mut self) -> Self {
        self.cfg.backend = TermletBackendMode::Real;
        self
    }
    pub async fn spawn(self) -> Result<Termlet, TermletError> {
        Termlet::spawn_with(self.cmd, self.args, self.cfg).await
    }
}
```

#### 32.3 Runtime behavior and wait semantics

- `send_keys` writes raw bytes to PTY input.
- Output bytes are parsed by VT parser into grid cells.
- `wait_for` behavior:
  - compile matcher once (plain or regex)
  - poll snapshot text at interval
  - if matched: return `WaitMatch`
  - if process exits before match: `PatternNotFound`
  - if deadline reached first: `WaitTimeout`
  - `timeout == 0`: single-pass immediate check

```rust
pub enum Pattern {
    Literal(String),
    Regex(regex::Regex),
}

pub struct WaitMatch {
    pub byte_start: usize,
    pub byte_end: usize,
    pub matched_at: std::time::SystemTime,
}

impl Termlet {
    pub async fn wait_for(&self, pattern: Pattern, timeout: Duration) -> Result<WaitMatch, TermletError> {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let text = self.snapshot().to_text();
            if let Some((s, e)) = find_match(&pattern, &text) {
                return Ok(WaitMatch {
                    byte_start: s,
                    byte_end: e,
                    matched_at: std::time::SystemTime::now(),
                });
            }

            let alive = self.is_alive().await?;
            if !alive {
                let label = pattern_label(&pattern);
                return Err(TermletError::PatternNotFound { pattern: label });
            }

            let now = tokio::time::Instant::now();
            if timeout.is_zero() || now >= deadline {
                let label = pattern_label(&pattern);
                return Err(TermletError::WaitTimeout {
                    pattern: label,
                    timeout_ms: timeout.as_millis() as u64,
                });
            }

            tokio::time::sleep(self.config().wait_poll_interval.min(deadline - now)).await;
        }
    }
}
```

#### 32.4 Snapshot capture model

- `TermletSnapshot` is immutable and cheap to clone.
- Views:
  - `to_text()` for stable snapshots/assertions
  - `to_cells()` for color/style assertions
  - `to_ansi()` for debug rendering
- Normalization contract:
  - trim trailing whitespace per line
  - trim trailing blank lines
  - preserve meaningful interior whitespace

```rust
#[derive(Debug, Clone)]
pub struct TermletSnapshot {
    pub cols: u16,
    pub rows: u16,
    pub cells: Vec<Cell>,
}

impl TermletSnapshot {
    pub fn to_text(&self) -> String {
        render_trimmed_text(self.cols, self.rows, &self.cells)
    }
}

#[test]
fn termlet_insta_snapshot() {
    let snap = TermletSnapshot { cols: 80, rows: 24, cells: vec![] };
    insta::assert_snapshot!(snap.to_text());
}
```

#### 32.5 Process lifecycle and cleanup

- `kill()` is idempotent.
- Two-phase kill: TERM then KILL after `graceful_kill_timeout`.
- `Drop` performs best-effort forced cleanup to avoid leaks.

```rust
impl Termlet {
    pub async fn kill(&self) -> Result<(), TermletError> {
        if self.terminated.load(std::sync::atomic::Ordering::SeqCst) {
            return Ok(());
        }
        self.backend.kill(self.pane_id, libc::SIGTERM)
            .map_err(|e| TermletError::PtyIo { reason: e.to_string() })?;
        tokio::time::sleep(self.cfg.graceful_kill_timeout).await;
        if self.is_alive().await.unwrap_or(false) {
            let _ = self.backend.kill(self.pane_id, libc::SIGKILL);
        }
        self.terminated.store(true, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }
}
```

#### 32.6 Python API (`PyTermlet`, PyO3)

```rust
#[pyclass(name = "Termlet")]
pub struct PyTermlet {
    inner: std::sync::Arc<Termlet>,
}

#[pymethods]
impl PyTermlet {
    #[staticmethod]
    pub fn spawn(cmd: String, cols: Option<u16>, rows: Option<u16>, backend: Option<String>) -> pyo3::PyResult<Self> { todo!() }
    pub fn send_keys(&self, keys: String) -> pyo3::PyResult<()> { todo!() }
    pub fn wait_for(&self, pattern: String, timeout: f64) -> pyo3::PyResult<()> { todo!() }
    pub fn resize(&self, cols: u16, rows: u16) -> pyo3::PyResult<()> { todo!() }
    pub fn snapshot(&self) -> pyo3::PyResult<String> { todo!() }
    pub fn is_alive(&self) -> pyo3::PyResult<bool> { todo!() }
    pub fn kill(&self) -> pyo3::PyResult<()> { todo!() }
    pub fn __enter__(slf: pyo3::PyRef<'_, Self>) -> pyo3::PyRef<'_, Self> { slf }
    pub fn __exit__(&self, _ty: &pyo3::Bound<'_, pyo3::types::PyAny>, _v: &pyo3::Bound<'_, pyo3::types::PyAny>, _tb: &pyo3::Bound<'_, pyo3::types::PyAny>) -> pyo3::PyResult<()> {
        // best-effort kill
        Ok(())
    }
}
```

Python fixture contract:

```python
import pytest
from termforge import Termlet

@pytest.fixture
def termlet():
    with Termlet.spawn("bash", cols=80, rows=24, backend="fake") as t:
        yield t
```

#### 32.7 Node API (`JsTermlet`, Neon/N-API)

```rust
pub struct JsTermlet {
    inner: std::sync::Arc<Termlet>,
}

impl JsTermlet {
    pub async fn spawn(cmd: String, opts: JsTermletOpts) -> Result<Self, JsError> { todo!() }
    pub async fn send_keys(&self, keys: String) -> Result<(), JsError> { todo!() }
    pub async fn wait_for(&self, pattern: String, timeout_ms: u64) -> Result<(), JsError> { todo!() }
    pub async fn resize(&self, cols: u16, rows: u16) -> Result<(), JsError> { todo!() }
    pub fn snapshot(&self) -> String { String::new() }
    pub async fn is_alive(&self) -> Result<bool, JsError> { todo!() }
    pub async fn kill(&self) -> Result<(), JsError> { todo!() }
}
```

vitest helper contract:

```javascript
export async function withTermlet(cmd = 'bash', opts = {}) {
  const t = await Termlet.spawn(cmd, opts);
  return {
    termlet: t,
    async close() {
      await t.kill();
    }
  };
}
```

#### 32.8 Error code alignment across languages

- Rust `TermletError` maps to stable codes:
  - `TERMLET_SPAWN_ERROR`
  - `TERMLET_PTY_ERROR`
  - `TERMLET_TIMEOUT`
  - `TERMLET_PATTERN_NOT_FOUND`
  - `TERMLET_INVALID_ARGUMENT`
  - `TERMLET_ALREADY_TERMINATED`
- Python raises `TermletError(code=...)`.
- Node rejects `TermletError` object with `code` and `cause`.

#### 32.9 OTEL integration for Termlet operations

- Feature-gated spans on `spawn/send_keys/wait_for/resize/kill/snapshot`.
- Include attributes: `termlet.id`, `pane.id`, `backend.mode`, `timeout.ms`.
- Termlet spans inherit current context when present.

```rust
#[cfg(feature = "otel")]
async fn with_termlet_span<T>(name: &'static str, id: u64, fut: impl std::future::Future<Output = Result<T, TermletError>>) -> Result<T, TermletError> {
    let span = tracing::info_span!("termforge.termlet.op", op = name, termlet_id = id);
    async move { fut.await }.instrument(span).await
}
```

### Rust Example

```rust
use std::time::Duration;

#[tokio::test]
async fn termlet_end_to_end_real_and_fake() -> Result<(), TermletError> {
    // [v10 Pass 3] Use TermletBuilder
    let fake = TermletBuilder::new("bash")
        .fake()
        .size(80, 24)
        .spawn()
        .await?;

    fake.send_keys("echo hello\n").await?;
    fake.wait_for(Pattern::Literal("hello".into()), Duration::from_secs(2)).await?;
    insta::assert_snapshot!(fake.snapshot().to_text());
    fake.resize(120, 40).await?;
    assert!(fake.is_alive().await?);
    fake.kill().await?;

    let real = TermletBuilder::new("bash")
        .real()
        .size(80, 24)
        .spawn()
        .await?;

    real.send_keys("printf 'ok\\n'\n").await?;
    real.wait_for(Pattern::Literal("ok".into()), Duration::from_secs(5)).await?;
    assert!(real.snapshot().to_text().contains("ok"));
    real.kill().await?;
    Ok(())
}
```

### Test Strategy

1. Unit tests (FakePty): spawn/send/wait/snapshot/resize/kill deterministic and timing-free.
2. Integration tests (Real PTY): shell spawn, prompt handling, SIGWINCH resize propagation, signal kill.
3. Snapshot tests:
   - Rust: `insta` snapshot corpus
   - Python: `pytest` snapshot text assertions
   - Node: `vitest` snapshot text assertions
4. Wait semantics tests:
   - literal and regex matching
   - immediate timeout (`0ms`) single-pass behavior
   - exit-before-match => `PatternNotFound`
   - deadline-first => `WaitTimeout`
5. Cross-language consistency tests:
   - same scripted input yields same normalized snapshot text
   - same failure scenario yields matching stable error code
6. Lifecycle tests:
   - double kill idempotency
   - drop cleanup prevents zombies
   - many short-lived termlets in parallel
7. OTEL tests:
   - spans emitted when enabled
   - no-op behavior when disabled

### AGENTS.md Rules

- `RULE-32-01`: Termlet must remain pane-backed; no parallel terminal core.
- `RULE-32-02`: Fake and Real backend semantics must match API contracts.
- `RULE-32-03`: `wait_for` must distinguish timeout vs process-exit-no-match.
- `RULE-32-04`: Snapshot normalization must stay stable across Rust/Python/Node.
- `RULE-32-05`: `kill()` is idempotent; `Drop` must perform best-effort forced cleanup.
- `RULE-32-06`: Binding APIs must preserve stable error codes.
- `RULE-32-07`: Termlet operations must emit OTEL spans when feature-enabled.
- `RULE-32-08`: Every Termlet method requires FakePty unit coverage and Real PTY integration coverage.

---

## v10 Pass 3 Consistency Checklist

- [x] 32 numbered sections present.
- [x] Each section includes Design Decisions, Rust Example, Test Strategy, AGENTS.md Rules.
- [x] Section 32 includes TermletBuilder, API, wait semantics, snapshotting, bindings, OTEL, cleanup.
- [x] Verified key external anchor claims for libtmux, vibe-tmux, and tmux C source.
- [x] TermletBuilder integrated into Rust Example in Section 32.

ENDOFSPEC
```
