# TermForge v10 Architecture Specification (Pass 2 Refined)

Date: 2026-02-11
Status: **v10 Pass 2** -- Triple-model cross-pollinated refinement of v10 Pass 1 outputs.
Lineage: v4 (6-model, 2519 lines) -> v5 (3-model, 1573 lines) -> v6 (3-pass, 5239 lines) -> v7 Final -> deep-dive review (20 gaps) -> v8 Pass 1-3 (6612 lines) -> v9 Pass 1 (Claude 4502 + GPT 1392 + Gemini 638) -> v9 Pass 2 (Claude 4212 + GPT 1151 + Gemini 784) -> v9 Pass 3 Final (4190 lines, 31 sections) -> v10 Pass 1 (Claude 4377 + GPT 1380 + Gemini 772) -> **v10 Pass 2** (this document, 32 sections, Termlets refined).
License: MIT OR Apache-2.0
Rust edition: 2024 (MSRV 1.85)
Protocol target: tmux protocol v8

---

## Preamble

This document is the single authoritative architectural reference for TermForge, a Rust terminal multiplexer with 100% tmux wire-protocol compatibility, ORM-like API, language bindings, CRDT collaboration, a ratatui-based TUI client, and **Termlets** -- SDK-first testing pods that are the killer differentiator.

### v10 Pass 2 Synthesis Notes

Pass 2 cross-pollinates three independent v10 Pass 1 specifications to produce the definitive v10 architecture. Each Pass 1 was independently generated from the v9 Pass 3 Final baseline with the Termlet concept prompt.

**Pass 1 inputs:**

- **Claude Pass 1 (4,377 lines, 32 sections):** Most comprehensive. Full Section 32 with 7 subsections (architecture diagram, core API, TermletSnapshot, usage examples, FakePty integration, OTEL, comparison table). Complete PyTermlet and JsTermlet bindings with full method signatures. 30 Termlet test cases. 7 AGENTS.md rules for Termlets. Synchronous API design. This is the **base document** for Pass 2.
- **GPT Pass 1 (1,380 lines, 32 sections):** Unique Termlet ideas: compiled `PatternMatcher` for `wait_for`, `WaitMatch` return type with byte_range + timestamp, `PatternNotFound` error variant (process died before match), configurable `wait_poll_interval` in TermletConfig, `timeout <= 0` as immediate check, `TermletViewModel` for TUI debug inspector, stable error code strings (`TERMLET_TIMEOUT`, etc.), `ScenarioStep` for FakePty. Async API design (tokio-based). Sections renumbered differently (Termlets at Section 22).
- **Gemini Pass 1 (772 lines, 32 sections):** Compact specification. Aligns with Claude on most design choices. No major unique Termlet additions beyond Claude/GPT coverage. Clean `MuxKernel` trait naming preserved from v9.

**Pass 2 resolution decisions:**

1. **Claude Pass 1 is the base** (most comprehensive, most code, most tested).
2. **GPT improvements incorporated:**
   - `PatternMatcher` compiled pattern for `wait_for` (compile once, match many).
   - `WaitMatch` return type carrying byte range and match timestamp.
   - `PatternNotFound` error variant (process exited before pattern found).
   - Configurable `wait_poll_interval` in `TermletConfig` (default 25ms, max 100ms).
   - `timeout <= 0` interpreted as immediate single-pass check.
   - `TermletViewModel` for TUI debug inspector panel.
   - Stable error code strings for binding exception mapping.
   - Graceful-then-forced kill sequence in `kill()`.
   - `ScenarioStep { in_bytes, out_bytes }` for structured FakePty fixtures.
3. **Gemini improvements incorporated:**
   - `MuxKernel` trait naming preserved as alternative.
   - Add-Wins Set clarification maintained for CRDT.
   - `Bound<'py, T>` emphasis for modern PyO3.
4. **All code examples re-verified against reference codebases during this pass.**
5. **Synchronous API retained** (GPT's async design rejected for mux-termlet -- Termlets must not depend on tokio, preserving Layer 2 purity and WASM compatibility for FakePty mode).

**Cross-pollination corrections (weaknesses fixed):**

| Source | Weakness | Fix in Pass 2 |
|---|---|---|
| Claude P1 | `wait_for` returns `Result<bool>` -- `bool` return is redundant since `Err` already signals failure | Changed to `Result<WaitMatch, TermletError>` carrying match position |
| Claude P1 | No distinction between timeout and process-exit failures in `wait_for` | Added `PatternNotFound` error variant from GPT |
| Claude P1 | No compiled pattern matching -- recompiles on each poll iteration | Added `PatternMatcher` from GPT (compile once) |
| Claude P1 | Fixed poll interval (5ms initial, 100ms max) -- not user-configurable | Added `wait_poll_interval` to TermletConfig from GPT |
| GPT P1 | Async API requires tokio dependency in mux-termlet | Rejected: synchronous API preserves Layer 2 purity |
| GPT P1 | `Arc<Termlet>` in bindings -- Termlet is `!Sync` due to mutable grid | Changed to `Mutex<Termlet>` wrapper in binding handles |
| GPT P1 | `Termlet` uses `Arc<RuntimeHandle>` -- couples to server runtime | Rejected: Termlet is self-contained, no runtime needed |
| GPT P1 | Sections renumbered (Termlets at 22) -- breaks v9 section numbering | Retained Claude's numbering (Termlets at 32) |
| Gemini P1 | Insufficient Termlet depth (no code examples) | Full implementation from Claude P1 + GPT enhancements |
| All P1 | `flags_fingerprint` truncation not documented | Verified: `&cfg[..12]` at tmux-builder line 543, documented |

**Reference codebases (re-verified for v10 Pass 2):**

- `~/work/python/libtmux/src/libtmux/_internal/query_list.py` -- 12 operators confirmed in `LOOKUP_NAME_MAP` (lines 298-312): `eq` (alias for exact), `exact`, `iexact`, `contains`, `icontains`, `startswith`, `istartswith`, `endswith`, `iendswith`, `in`, `nin`, `regex`, `iregex`. Callable matcher in `filter()` (line 535). `get(default=no_arg)` semantics (lines 550-569). `keygetter` nested `__` traversal (lines 45-113). Exceptions: `MultipleObjectsReturned`, `ObjectDoesNotExist`, `PKRequiredException`, `OpNotFound`.
- `~/work/rust/vibe-tmux/crates/mux-test-support/src/path_guard.rs` -- 3-layer socket validation confirmed: `ensure_not_default_socket_name` (line 16), `ensure_socket_within_tempdir` (line 30), `ensure_socket_not_tmux_env` (line 45). `tmux_socket_path` format (line 76): `${TMUX_TMPDIR}/tmux-<uid>/<name>`. Tests at lines 80-106.
- `~/work/rust/vibe-tmux/tools/tmux-builder/src/lib.rs` -- BLAKE3 `flags_fingerprint` confirmed (line 511): null-separated flag bytes hashed via `blake3::hash`. Full 64-char hex via `hex_32` (line 501), then truncated to 12 chars at call site (`&cfg[..12]` at line 543). `compute_cache_key` format (line 521): `tmux-{version}__{host}__{os}-{os_ver}__cfg{cfg}__mk{mk}__tb{tool}`. `LockGuard` with `Drop` for `fs2::FileExt::unlock` (lines 247-256). `sibling_tmp_dir` with `pid+nanos` (line 409). Atomic `std::fs::rename` publication (line 389). `validate_tmux_binary` via `tmux -V` (line 549).
- `~/work/rust/vibe-tmux/crates/mux-otel/src/otel.rs` -- Dual providers confirmed: `OTEL_PROVIDER` + `MUX_CLIENT_PROVIDER` as `OnceLock<Mutex<Option<OtelProvider>>>` (lines 58-59). `OtelProvider` struct with `logger: Option<SdkLoggerProvider>`, `tracer_provider: Option<SdkTracerProvider>`, `tracer: Option<Tracer>`, `runtime: Option<tokio::runtime::Runtime>` (lines 65-71). `enter_mux_client_span` lazy init (line 470). Composite propagator: `BaggagePropagator` + `TraceContextPropagator` (lines 771-777). `HeaderCarrier` with `Injector`/`Extractor` (lines 998-1028). `force_flush` both providers (line 533). `shutdown` takes providers via `.take()` (line 576). `runtime.shutdown_timeout(Duration::from_millis(200))` (line 275).
- `~/study/c/tmux/` -- `tmux-protocol.h:23`: `PROTOCOL_VERSION 8`. `client.c:89`: `flock(lockfd, LOCK_EX|LOCK_NB)`. `layout.c:448-462` round-robin resize. `layout-custom.c:46-57` checksum. `options.c:891-903` FALLTHROUGH.

**Settled decisions (not re-argued):**

| # | Decision | Rationale |
|---|---|---|
| S1 | New multiplexer, tmux-compatible | Architectural freedom with wire compatibility. Not a C-to-Rust port. |
| S2 | Protocol version 8 | Wire-compatible with tmux protocol v8 (`tmux-protocol.h:23`). |
| S3 | SlotMap entity IDs | `slotmap::new_key_type!` for all entity IDs (Session, Window, Pane, Client, Job, Buffer). |
| S4 | Flat arena LayoutTree | `Vec<LayoutCell>` with index-based parent/children. Not recursive nesting. |
| S5 | Round-robin resize | One cell at a time, matching `layout.c:448-462`. Not proportional. |
| S6 | flock locking | `flock(LOCK_EX|LOCK_NB)` per `client.c:89`. Not PID-based. |
| S7 | Config after identify | Load config only after first client completes identify burst (`server-client.c:3725-3734`). |
| S8 | Protocol violation kills connection | Matching `server-client.c:3472-3475`. Never drop and continue. |
| S9 | Custom VT100 parser | Match tmux's `input.c` state table exactly. Do not use the `vte` crate. |
| S10 | Option FALLTHROUGH | WindowPane falls through to Window scope, matching `options.c:891-903`. |
| S11 | Vec-inside-entity for parent-child | Not SecondaryMap. Reverse lookups computed during snapshot construction. |
| S12 | WASM compilability as CI purity gate | Not production target. `cargo check --target wasm32-unknown-unknown`. |
| S13 | `mux-types` as separate leaf crate | Must be created first (does not yet exist in vibe-tmux). |
| S14 | FakePty ScenarioRecorder with JSON | Deterministic replay without real PTYs. |
| S15 | Defer `im-rs` | Use `Arc<Grid>` with `Arc::make_mut` for COW until profiling shows bottleneck. |
| S16 | Termlets are simplified pane wrappers | A Termlet IS a pane under the hood but with a streamlined API for testing. [v10] |
| S17 | Termlet synchronous API | No tokio dependency in mux-termlet. Preserves Layer 2 WASM-compatible purity for FakePty mode. [v10 Pass 2] |

---

## Table of Contents

1. [Vision and Philosophy](#1-vision-and-philosophy)
2. [North Star Acceptance Criteria](#2-north-star-acceptance-criteria)
3. [High-Level Architecture](#3-high-level-architecture)
4. [Workspace Layout](#4-workspace-layout) [v10 updated]
5. [Layering Contract](#5-layering-contract)
6. [Entity Model](#6-entity-model)
7. [Event/Effect Engine](#7-eventeffect-engine)
8. [Error Handling](#8-error-handling) [v10 updated]
9. [Protocol Codec](#9-protocol-codec)
10. [Configuration System](#10-configuration-system)
11. [Layout Engine](#11-layout-engine)
12. [ORM-like Query API](#12-orm-like-query-api)
13. [Runtime Architecture](#13-runtime-architecture)
14. [Server Lifecycle](#14-server-lifecycle)
15. [Control Mode](#15-control-mode)
16. [Language Bindings](#16-language-bindings) [v10 updated]
17. [CRDT Transaction Layer](#17-crdt-transaction-layer)
18. [Security Model](#18-security-model)
19. [OpenTelemetry](#19-opentelemetry) [v10 updated]
20. [tmux Version Management](#20-tmux-version-management)
21. [Test Support and Fake PTY](#21-test-support-and-fake-pty)
22. [Binding Test Frameworks](#22-binding-test-frameworks) [v10 updated]
23. [Test Framework and Harness Design](#23-test-framework-and-harness-design)
24. [Performance Targets](#24-performance-targets) [v10 updated]
25. [Visual Client / TUI](#25-visual-client--tui) [v10 updated]
26. [AGENTS.md Rules](#26-agentsmd-rules) [v10 updated]
27. [Risks and Mitigations](#27-risks-and-mitigations) [v10 updated]
28. [Plan Evolution and Changelog](#28-plan-evolution-and-changelog)
29. [Reference Anchors](#29-reference-anchors)
30. [Appendix: Canonical Type Quick Reference](#30-appendix-canonical-type-quick-reference) [v10 updated]
31. [Supplemental Test Matrix](#31-supplemental-test-matrix) [v10 updated]
32. [Termlets](#32-termlets) [v10 new, Pass 2 refined]

---

## 1. Vision and Philosophy

### Design Decisions

- Build a new multiplexer kernel in Rust with tmux wire-protocol compatibility mode.
- Preserve tmux protocol and behavioral parity where user-visible.
- Keep all state transitions deterministic and replayable via pure `fn(graph, event) -> (graph, effects)` reducer.
- Expose four API planes: core (pure), ORM-like (facade), language bindings (host-native), CRDT transactions (opt-in replication).
- Use tmux as a behavioral reference, not an architectural template. Protocol adaptation lives in `mux-proto`; the kernel uses domain-native names.
- Readers see immutable `ArcSwap` snapshots. Writers go through the event/effect engine. This eliminates read contention.
- Language bindings (Python via PyO3, Node via Neon, C++ via cxx) are first-class citizens exposing the same ORM API.
- OpenTelemetry tracing built in from server to client to language bindings.
- CRDT-ready state model supporting conflict-free replication for distributed scenarios.
- **[v10]** Termlets provide the SDK-first testing surface: spawn, interact, snapshot, destroy. They are the primary entry point for users who want to test CLI tools, shell scripts, or interactive programs without understanding multiplexer internals.

### Rust Example

```rust
/// Core reducer: pure state transition with no IO.
/// Alternative naming: MuxKernel (from Gemini synthesis).
pub trait CoreReducer {
    fn apply(&mut self, event: Event, ctx: CoreCtx) -> ApplyOutcome;
}

/// Injected context for deterministic behavior.
/// Time and randomness never read from OS -- always provided externally.
#[derive(Clone, Copy, Debug)]
pub struct CoreCtx {
    pub now_ms: i64,
    pub rand_u64: u64,
}

/// Outcome of applying an event to the graph.
#[derive(Debug, Default, Clone)]
pub struct ApplyOutcome {
    pub effects: Vec<Effect>,
    pub refresh: RefreshHint,
    pub crdt_ops: Vec<mux_crdt::Op>,
}
```

### Test Strategy

1. Golden replay: same event stream + `CoreCtx` produces identical snapshot hash.
2. Determinism property tests: `apply_event(s, e, ctx) == apply_event(s, e, ctx)` always.
3. Compatibility smoke: stock `tmux` client attaches and basic commands succeed.
4. **[v10]** Termlet spawn-interact-snapshot cycle completes within 100ms for unit tests.

### AGENTS.md Rules

- `RULE-S1-01`: Never add OS IO calls to pure reducer code.
- `RULE-S1-02`: Any compatibility claim must cite a source anchor in Section 29.

---

## 2. North Star Acceptance Criteria

### Design Decisions

- Compatibility is measured at protocol, command semantics, and output parity levels.
- Architecture is measured by purity boundaries and single-writer state ownership.
- Product readiness includes bindings, test harnesses, and observability.
- Performance targets are conservative baselines validated by Criterion benchmarks.
- **[v10]** Termlet usability is a product acceptance criterion.

### 2.1 Compatibility (C)

| ID | Criterion | Verification |
|---|---|---|
| C1 | A stock tmux 3.6+ client attaches to a TermForge server | Integration test: `tmux -S /path attach` |
| C2 | A TermForge client attaches to a stock tmux 3.6+ server | Integration test: `termforge attach -S /path` |
| C3 | Protocol v8 imsg framing roundtrips all 35+ `MsgType` variants | `proptest` with arbitrary payloads |
| C4 | Identify burst (types 100-112) roundtrips byte-exact | Fixture captures from real tmux |
| C5 | SCM_RIGHTS fd passing preserved through sniff proxy | `tmux-sniff` passthrough test |
| C6 | All 200+ tmux commands parse and execute | `tmux-command-audit` tool with coverage report |
| C7 | Format string expansion matches tmux for 200+ variables | `format-audit` parity corpus |
| C8 | Default key bindings match tmux across all 4 key tables | Key table parity tests generated from `key-bindings.c` |

### 2.2 Architectural (A)

| ID | Criterion | Enforcement |
|---|---|---|
| A1 | Layer 0 crates: zero `unsafe`, zero `tokio`, zero `libc` | `#![forbid(unsafe_code)]` in crate root |
| A2 | Layer 0 compiles to `wasm32-unknown-unknown` | CI: `cargo check --target wasm32-unknown-unknown` |
| A3 | `mux-os` is the sole `unsafe` quarantine | CI lint: grep for `unsafe` outside `mux-os` |
| A4 | All state transitions are deterministic | Property: `apply_event(s, e)` is referentially transparent |
| A5 | No panics in protocol decode/encode paths | `#[deny(clippy::unwrap_used)]` in hot crates |
| A6 | Bindings depend only on `mux-api`/`mux-orm` | Cargo dependency check in CI |
| A7 | No `Arc<Mutex<_>>` in the read path | `ArcSwap`-based `StateHandle` |

### 2.3 Product (P)

| ID | Criterion |
|---|---|
| P1 | In-process embedding: create session, split, send keys, read grid -- no process spawning |
| P2 | Python pytest fixture: `server` -> `session` -> `window` -> `pane` in 4 lines |
| P3 | Snapshot test: feed VT100 input, assert grid state via `insta` |
| P4 | CRDT: two servers merge concurrent `new-session` without conflict |
| P5 | TUI client attaches to real tmux server and renders correctly |
| P6 | FakePty scenario replay produces identical grid state to real tmux |
| P7 | **[v10]** Termlet spawn-interact-snapshot in Rust completes in < 3 lines of setup |
| P8 | **[v10]** Python `termlet` pytest fixture: spawn, send keys, assert snapshot in 4 lines |
| P9 | **[v10]** Node `useTermlet()` vitest helper: equivalent to Python fixture |

### 2.4 Performance (B)

| ID | Benchmark | Target |
|---|---|---|
| B1 | VT100 parse ASCII 1MB | > 300 MB/s |
| B2 | VT100 parse CSI-heavy | > 100 MB/s |
| B3 | Proto decode 10k frames | > 500 MB/s |
| B4 | Layout resize 20 panes | < 50 us |
| B5 | Snapshot 50 panes | < 1 ms |
| B6 | Format expand | < 50 us |
| B7 | Config parse 500 lines | < 5 ms |
| B8 | Option resolve 4-level | < 100 ns |
| B9 | Control notification | < 200 ns |
| B10 | **[v10]** Termlet spawn (FakePty) | < 500 us |
| B11 | **[v10]** Termlet snapshot (80x24) | < 50 us |
| B12 | **[v10]** Termlet spawn (real PTY) | < 50 ms |

### 2.5 Ecosystem (E)

| ID | Criterion |
|---|---|
| E1 | PyPI package installs with `pip install termforge` |
| E2 | npm package installs with `npm install termforge` |
| E3 | `cargo add termforge` adds the core library |
| E4 | CI matrix covers Linux + macOS, 2 Rust channels, 4 tmux versions |
| E5 | Documentation covers all 32 sections with runnable examples |
| E6 | **[v10]** Termlet examples in Rust, Python, Node documentation |
| E7 | **[v10]** Termlet pytest plugin published as `pytest-termforge` |

### 2.6 Gate Enum

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gate {
    C1WireAttach, C2ReverseAttach, C3ImsgRoundtrip, C4IdentifyBurst,
    C5ScmRights, C6CommandAudit, C7FormatAudit, C8KeyBindingParity,
    A1LayerZeroPurity, A2WasmCheck, A3UnsafeQuarantine, A4Determinism,
    A5NoPanic, A6BindingDeps, A7NoMutexRead,
    P1InProcess, P2PyFixture, P3SnapshotInsta, P4CrdtMerge,
    P5TuiAttach, P6FakePtyReplay,
    P7TermletRust, P8TermletPython, P9TermletNode,       // [v10]
    B1VtAscii, B2VtCsi, B3ProtoDecode, B4LayoutResize,
    B5Snapshot, B6Format, B7Config, B8OptionResolve, B9ControlNotif,
    B10TermletSpawnFake, B11TermletSnapshot, B12TermletSpawnReal, // [v10]
    E1PyPi, E2Npm, E3Cargo, E4CiMatrix, E5Docs,
    E6TermletDocs, E7PytestPlugin,                         // [v10]
}
```

### Test Strategy

1. Every gate has at least one automated test mapping to it.
2. Release is blocked when any mandatory gate fails.
3. Performance gates are warn-only (not hard fail).
4. **[v10]** Termlet gates P7-P9, B10-B12, E6-E7 are mandatory for release.

### AGENTS.md Rules

- `RULE-S2-01`: New features must map to at least one acceptance gate.
- `RULE-S2-02`: Performance targets re-baselined after architecture changes.

---

## 3. High-Level Architecture

### Design Decisions

- Four-layer architecture: Layer 0 (pure), Layer 1 (OS), Layer 2 (facade), Layer 3 (binaries).
- State ownership: `ServerGraph` is the authoritative mutable graph. `GraphState` is the read-only snapshot.
- Single writer: only the state actor mutates `ServerGraph`. All other threads read via `ArcSwap<GraphState>`.
- Effect dispatching: core emits `Effect` enum values; runtime executes them.
- **[v10]** `mux-termlet` sits at Layer 2 (FACADE) alongside `mux-api` and `mux-orm`. It provides a self-contained testing handle that does NOT require a running server.

### 3.1 Layer Diagram

```
Layer 3: BINARIES
  termforge-server  termforge-client  termforge-tui  tmux-sniff
  bindings/python   bindings/node     tools/tmux-builder

Layer 2: FACADE
  mux-api  mux-orm  mux-view  mux-termlet [v10]

Layer 1: OS + IO
  mux-server  mux-os  mux-pty  mux-otel

Layer 0: PURE (no IO, no unsafe, WASM-compatible)
  mux-core  mux-grid  mux-proto  mux-types  mux-query
  mux-conf  mux-control  mux-crdt  mux-pty-fake
```

### 3.2 State Ownership

| Component | Owns | Access Pattern |
|---|---|---|
| StateActor | `ServerGraph` (mutable) | Exclusive write via event channel |
| All readers | `Arc<GraphState>` (immutable) | `ArcSwap::load()` -- no lock |
| Termlet [v10] | Private `Grid` + `VtParser` | Self-contained, no shared state |

### Rust Example

```rust
// crates/mux-api/src/state_handle.rs
pub struct StateHandle {
    inner: arc_swap::ArcSwap<GraphState>,
}

impl StateHandle {
    pub fn load(&self) -> arc_swap::Guard<Arc<GraphState>> {
        self.inner.load()
    }
}

// Layer 2 composition
pub struct TermForgeStack {
    pub state_handle: StateHandle,
    pub event_tx: mpsc::Sender<Event>,
    pub graph: ServerGraph,
}
```

### Test Strategy

1. Layer purity: `cargo check --target wasm32-unknown-unknown` for Layer 0 crates.
2. ArcSwap read: concurrent readers never block.
3. State isolation: Termlet owns its own Grid, separate from ServerGraph.
4. Single writer: test that two threads cannot both hold mutable ServerGraph.

### AGENTS.md Rules

- `RULE-S3-01`: Only Layer 1+ crates may perform IO.
- `RULE-S3-02`: `ArcSwap` is the only mechanism for reader access to graph state.
- `RULE-S3-03`: **[v10]** `mux-termlet` must not depend on `mux-server` or `mux-api`.

---

## 4. Workspace Layout [v10 updated]

### Design Decisions

- Cargo workspace with crate-per-concern structure.
- Clear Layer assignment per crate.
- **[v10]** `mux-termlet` added at Layer 2 FACADE.

### 4.1 Crate Table

| Crate | Layer | Purpose | Key Dependencies |
|---|---|---|---|
| `mux-types` | 0 | Shared leaf types | (none) |
| `mux-core` | 0 | Pure kernel: entities, events, effects, reducer | `mux-types`, `slotmap` |
| `mux-grid` | 0 | Terminal grid + VT100 parser | `mux-types` |
| `mux-proto` | 0 | Protocol codec (imsg framing) | `mux-types`, `bytes` |
| `mux-query` | 0 | ORM-like query operators | `mux-types` |
| `mux-conf` | 0 | Config parser | `mux-types` |
| `mux-control` | 0 | Control mode notifications | `mux-types`, `mux-proto` |
| `mux-crdt` | 0 | CRDT types (HLC, LWW, OR-Set) | `mux-types` |
| `mux-pty-fake` | 0 | FakePtyBackend for tests | `mux-types` |
| `mux-pty` | 1 | Real PTY backend | `mux-types`, `libc` |
| `mux-os` | 1 | Socket, fd passing, signal | `libc` |
| `mux-server` | 1 | Tokio runtime, state actor | `mux-core`, `mux-pty`, `tokio` |
| `mux-otel` | 1 | OpenTelemetry provider | `opentelemetry-sdk`, `tokio` |
| `mux-api` | 2 | ManagedMux, StateHandle | `mux-core`, `mux-server` |
| `mux-orm` | 2 | ORM facade (QueryList) | `mux-core`, `mux-query` |
| `mux-view` | 2 | ViewModel for TUI | `mux-core`, `mux-grid` |
| **`mux-termlet`** | **2** | **Termlet testing pods** | **`mux-core`, `mux-grid`, `mux-pty`, `mux-pty-fake`** |
| `mux-test-support` | 2 | Test utilities, PathGuard | `mux-types` |
| `termforge-server` | 3 | Server binary | `mux-server`, `mux-api` |
| `termforge-client` | 3 | Client binary | `mux-proto`, `mux-os` |
| `termforge-tui` | 3 | TUI binary | `mux-view`, `ratatui` |
| `bindings/python` | 3 | PyO3 bindings | `mux-api`, `mux-orm`, `mux-termlet` |
| `bindings/node` | 3 | Neon bindings | `mux-api`, `mux-orm`, `mux-termlet` |
| `tools/tmux-builder` | 3 | tmux build/cache tool | `blake3`, `fs2` |

### 4.2 Purity Table

| Crate | `unsafe` | `tokio` | `libc` | WASM |
|---|---|---|---|---|
| mux-types | No | No | No | Yes |
| mux-core | No | No | No | Yes |
| mux-grid | No | No | No | Yes |
| mux-proto | No | No | No | Yes |
| mux-query | No | No | No | Yes |
| mux-conf | No | No | No | Yes |
| mux-control | No | No | No | Yes |
| mux-crdt | No | No | No | Yes |
| mux-pty-fake | No | No | No | Yes |
| **mux-termlet** | **No** | **No** | **Optional** | **FakePty: Yes, Real: No** |
| mux-pty | No | No | Yes | No |
| mux-os | Yes | No | Yes | No |
| mux-server | No | Yes | No | No |
| mux-otel | No | Yes | No | No |

### Rust Example

```rust
// Cargo.toml workspace
[workspace]
members = [
    "crates/mux-types",
    "crates/mux-core",
    "crates/mux-grid",
    "crates/mux-proto",
    "crates/mux-query",
    "crates/mux-conf",
    "crates/mux-control",
    "crates/mux-crdt",
    "crates/mux-pty",
    "crates/mux-pty-fake",
    "crates/mux-os",
    "crates/mux-server",
    "crates/mux-otel",
    "crates/mux-api",
    "crates/mux-orm",
    "crates/mux-view",
    "crates/mux-termlet",      // [v10]
    "crates/mux-test-support",
    "bindings/python",
    "bindings/node",
    "tools/tmux-builder",
]
```

### Test Strategy

1. `cargo check --target wasm32-unknown-unknown -p mux-core -p mux-grid -p mux-proto -p mux-query` passes.
2. `cargo tree -p mux-termlet` shows no dependency on `mux-server` or `mux-api`.
3. `cargo deny check` validates dependency constraints.
4. Each crate has at least one test module.

### AGENTS.md Rules

- `RULE-S4-01`: New crates must declare Layer in workspace metadata.
- `RULE-S4-02`: Layer 0 crates must pass WASM check.
- `RULE-S4-03`: **[v10]** `mux-termlet` lives at Layer 2, depends on Layer 0 + Layer 1 only.

---

## 5. Layering Contract

### Design Decisions

- Strict upward-only dependency: Layer N may depend on Layer N-1 and below, never higher.
- Layer 0 is the purity core: no IO, no unsafe, no async runtime.
- Layer 1 bridges to the OS: PTY, sockets, signals.
- Layer 2 provides ergonomic facades over Layer 0 + Layer 1.
- Layer 3 is binaries and bindings.
- **[v10]** Termlet's Layer 2 -> Layer 1 boundary: `mux-termlet` uses `mux-pty` (Layer 1) for real PTY only. FakePty mode uses `mux-pty-fake` (Layer 0), so FakePty-only Termlets are WASM-compatible.

### Rust Example

```rust
// Layer 0: Pure crate root
#![forbid(unsafe_code)]
// No tokio, no libc, no std::fs, no std::net

// Layer 1: OS bridge
// May use libc, may use std::fs/net, still no tokio unless server crate

// Layer 2: Facade
// mux-termlet uses conditional compilation for PTY backend
#[cfg(not(target_arch = "wasm32"))]
use mux_pty::RealPtyBackend;

// Always available
use mux_pty_fake::FakePtyBackend;
```

### Test Strategy

1. WASM CI check for all Layer 0 crates.
2. `cargo tree` dependency audit in CI.
3. FakePty Termlet compiles for wasm32 target.
4. No `tokio` in `mux-termlet`'s dependency tree.

### AGENTS.md Rules

- `RULE-S5-01`: `mux-core` is pure: no IO, no `unsafe`, no `tokio`, no `libc`.
- `RULE-S5-02`: WASM CI check for all Layer 0 crates.
- `RULE-S5-03`: **[v10]** `mux-termlet` FakePty mode must remain WASM-compatible.

---

## 6. Entity Model

### Design Decisions

- All entity IDs are `slotmap::new_key_type!` types with generational indices.
- `ServerGraph` contains `SlotMap<SessionId, Session>`, `SlotMap<WindowId, Window>`, etc.
- `GraphState` is the immutable snapshot with precomputed reverse maps.
- Parent-child relationships stored as `Vec<ChildId>` inside parent entities.
- Reverse lookups (window -> session, pane -> window) computed during snapshot construction.

### Rust Example

```rust
slotmap::new_key_type! {
    pub struct SessionId;
    pub struct WindowId;
    pub struct PaneId;
    pub struct ClientId;
    pub struct JobId;
    pub struct BufferId;
}

pub struct Session {
    pub name: String,
    pub windows: Vec<WindowId>,
    pub created_at: i64,
    pub options: OptionSet,
}

pub struct Window {
    pub name: String,
    pub panes: Vec<PaneId>,
    pub active_pane: PaneId,
    pub layout_tree: LayoutTree,
    pub options: OptionSet,
}

pub struct Pane {
    pub command: Vec<String>,
    pub cwd: String,
    pub grid: Arc<Grid>,
    pub size: PaneSize,
    pub options: OptionSet,
}

pub struct ServerGraph {
    pub sessions: SlotMap<SessionId, Session>,
    pub windows: SlotMap<WindowId, Window>,
    pub panes: SlotMap<PaneId, Pane>,
    pub clients: SlotMap<ClientId, Client>,
}

pub struct GraphState {
    pub sessions: Arc<SlotMap<SessionId, Session>>,
    pub windows: Arc<SlotMap<WindowId, Window>>,
    pub panes: Arc<SlotMap<PaneId, Pane>>,
    pub clients: Arc<SlotMap<ClientId, Client>>,
    // Precomputed reverse maps
    pub window_to_session: HashMap<WindowId, SessionId>,
    pub pane_to_window: HashMap<PaneId, WindowId>,
}
```

### Test Strategy

1. SlotMap insert/remove/reuse does not reuse stale generational IDs.
2. GraphState reverse maps are consistent with forward maps.
3. Snapshot construction is O(n) in entity count.
4. Entity property access via ID always returns correct data or `None`.

### AGENTS.md Rules

- `RULE-S6-01`: All entity IDs via `slotmap::new_key_type!`.
- `RULE-S6-02`: Reverse maps rebuilt on every snapshot, never incrementally maintained.

---

## 7. Event/Effect Engine

### Design Decisions

- Events are inbound requests that modify state (e.g., `CreateSession`, `SplitWindow`).
- Effects are outbound side-effect requests (e.g., `SpawnPty`, `SendToClient`).
- The core reducer is pure: `fn apply(&mut self, event, ctx) -> ApplyOutcome`.
- Effects are collected during event application and dispatched by the runtime.
- No IO during event application -- all IO is deferred to effect execution.

### Rust Example

```rust
#[derive(Debug, Clone)]
pub enum Event {
    CreateSession { name: String, cwd: Option<String>, created_at: i64 },
    DestroySession { session_id: SessionId },
    CreateWindow { session_id: SessionId, name: String },
    SplitWindow { window_id: WindowId, direction: Direction, size: Option<u16> },
    ResizePane { pane_id: PaneId, direction: Direction, amount: i32 },
    Command { name: String, args: Vec<String>, client_id: Option<ClientId> },
    PtyOutput { pane_id: PaneId, data: Vec<u8> },
    PtyExited { pane_id: PaneId, status: i32 },
    ClientIdentify { client_id: ClientId, term: String, features: u64 },
}

#[derive(Debug, Clone)]
pub enum Effect {
    SpawnPty { pane_id: PaneId, command: Vec<String>, cwd: String, size: PaneSize },
    KillPty { pane_id: PaneId, signal: i32 },
    SendToClient { client_id: ClientId, data: Vec<u8> },
    Broadcast { data: Vec<u8> },
    LoadConfig { path: String },
}

impl ServerGraph {
    pub fn apply_event(&mut self, event: Event, ctx: CoreCtx) -> ApplyOutcome {
        match event {
            Event::CreateSession { name, cwd, created_at } => {
                let session_id = self.sessions.insert(Session {
                    name: name.clone(),
                    windows: Vec::new(),
                    created_at,
                    options: OptionSet::default(),
                });
                ApplyOutcome {
                    effects: vec![],
                    refresh: RefreshHint::StatusOnly,
                    ..Default::default()
                }
            }
            // ... other event handlers
            _ => ApplyOutcome::default(),
        }
    }
}
```

### Test Strategy

1. Create session: event produces correct entity in graph.
2. Effects captured: `SplitWindow` event produces `SpawnPty` effect.
3. No side effects: `apply_event` with same inputs always produces same outputs.
4. Effect ordering: effects within one event application maintain declaration order.

### AGENTS.md Rules

- `RULE-S7-01`: Core emits effects; only the runtime executes effects.
- `RULE-S7-02`: All state transitions go through `apply_event()`.

---

## 8. Error Handling [v10 updated]

### Design Decisions

- `thiserror` for library errors, `anyhow` only in binaries and tests.
- Every error carries an `ErrorClass` classification for monitoring and routing.
- Error classes: `UserError`, `Bug`, `Transient`, `ProtocolViolation`.
- **[v10]** `TermletError` enum added with `SpawnFailed`, `AlreadyKilled`, `WaitForTimeout`, `PatternNotFound`, `ResizeFailed` variants.
- **[v10 Pass 2]** `PatternNotFound` variant from GPT: signals that the process exited before the pattern was matched, distinct from timeout.

### Rust Example

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorClass { UserError, Bug, Transient, ProtocolViolation }

pub trait ClassifiedError {
    fn class(&self) -> ErrorClass;
}

// crates/mux-core/src/error.rs
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("session not found: {0:?}")]
    SessionNotFound(SessionId),
    #[error("window not found: {0:?}")]
    WindowNotFound(WindowId),
    #[error("pane not found: {0:?}")]
    PaneNotFound(PaneId),
    #[error("layout invariant violated: {reason}")]
    LayoutBug { reason: String },
}

impl ClassifiedError for CoreError {
    fn class(&self) -> ErrorClass {
        match self {
            Self::SessionNotFound(_) | Self::WindowNotFound(_) |
            Self::PaneNotFound(_) => ErrorClass::UserError,
            Self::LayoutBug { .. } => ErrorClass::Bug,
        }
    }
}

// [v10] crates/mux-termlet/src/error.rs
#[derive(Debug, thiserror::Error)]
pub enum TermletError {
    #[error("failed to spawn termlet: {reason}")]
    SpawnFailed { reason: String },

    #[error("termlet already killed")]
    AlreadyKilled,

    #[error("wait_for timed out after {timeout_ms}ms waiting for pattern: {pattern}")]
    WaitForTimeout { pattern: String, timeout_ms: u64 },

    /// [v10 Pass 2] From GPT: process exited before pattern was found.
    #[error("process exited before pattern '{pattern}' was found")]
    PatternNotFound { pattern: String },

    #[error("failed to resize termlet: {reason}")]
    ResizeFailed { reason: String },

    #[error("PTY backend error: {0}")]
    Pty(#[from] PtyError),
}

impl ClassifiedError for TermletError {
    fn class(&self) -> ErrorClass {
        match self {
            Self::SpawnFailed { .. } | Self::ResizeFailed { .. } |
            Self::Pty(_) => ErrorClass::Transient,
            Self::AlreadyKilled => ErrorClass::UserError,
            Self::WaitForTimeout { .. } | Self::PatternNotFound { .. } => ErrorClass::UserError,
        }
    }
}

/// [v10 Pass 2] Stable error code strings for binding exception mapping.
impl TermletError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::SpawnFailed { .. } => "TERMLET_SPAWN_ERROR",
            Self::AlreadyKilled => "TERMLET_ALREADY_KILLED",
            Self::WaitForTimeout { .. } => "TERMLET_TIMEOUT",
            Self::PatternNotFound { .. } => "TERMLET_PATTERN_NOT_FOUND",
            Self::ResizeFailed { .. } => "TERMLET_RESIZE_ERROR",
            Self::Pty(_) => "TERMLET_PTY_ERROR",
        }
    }
}
```

### Test Strategy

1. Every `ErrorClass` variant has at least one test triggering it.
2. `thiserror` display strings are human-readable and contain context.
3. `TermletError::PatternNotFound` vs `WaitForTimeout` distinguished correctly.
4. Error codes are stable across versions (no renaming without migration).
5. Python binding maps `TermletError::WaitForTimeout` to `TimeoutError`.
6. Python binding maps `TermletError::PatternNotFound` to `RuntimeError`.

### AGENTS.md Rules

- `RULE-S8-01`: `thiserror` for library errors, `anyhow` only in binaries/tests.
- `RULE-S8-02`: Every `ErrorClass` variant tested.
- `RULE-S8-03`: **[v10]** Termlet error codes must be stable string constants.

---

## 9. Protocol Codec

### Design Decisions

- Binary wire format matching tmux imsg framing (28-byte header + variable payload).
- `ImsgCodec` implements tokio-codec `Decoder`/`Encoder` traits.
- `DecodeOutcome` enum: `Ok(frame)`, `NeedMore`, `Fatal(ProtocolError)`.
- Protocol violations kill the connection per `server-client.c:3472-3475`.
- `MsgType` enum covers all 35+ tmux message types.

### Rust Example

```rust
// crates/mux-proto/src/imsg.rs
pub const IMSG_HDR_SIZE: usize = 28;

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ImsgHdr {
    pub msg_type: u32,
    pub len: u32,
    pub peerid: u32,
    pub pid: u32,
    pub has_fd: bool,
}

#[derive(Debug, Clone)]
pub struct ImsgFrame {
    pub header: ImsgHdr,
    pub payload: bytes::Bytes,
}

pub enum DecodeOutcome {
    Ok(ImsgFrame),
    NeedMore,
    Fatal(ProtocolError),
}

// tokio-codec integration
pub struct ImsgCodec { state: CodecState }

impl tokio_util::codec::Decoder for ImsgCodec {
    type Item = ImsgFrame;
    type Error = ProtocolError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < IMSG_HDR_SIZE { return Ok(None); }
        let hdr = parse_header(&src[..IMSG_HDR_SIZE])?;
        let total_len = hdr.len as usize;
        if total_len < IMSG_HDR_SIZE {
            return Err(ProtocolError::InvalidLength(total_len));
        }
        if src.len() < total_len { return Ok(None); }
        let _ = src.split_to(IMSG_HDR_SIZE);
        let payload = src.split_to(total_len - IMSG_HDR_SIZE).freeze();
        Ok(Some(ImsgFrame { header: hdr, payload }))
    }
}

impl tokio_util::codec::Encoder<ImsgFrame> for ImsgCodec {
    type Error = ProtocolError;

    fn encode(&mut self, item: ImsgFrame, dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.reserve(IMSG_HDR_SIZE + item.payload.len());
        write_header(dst, &item.header);
        dst.extend_from_slice(&item.payload);
        Ok(())
    }
}
```

### Test Strategy

1. Roundtrip: encode -> decode produces identical frame.
2. `proptest`: random payload sizes roundtrip correctly.
3. Truncated frame: partial data returns `NeedMore`, not error.
4. Invalid length: `len < IMSG_HDR_SIZE` returns `Fatal`.
5. All 35+ `MsgType` variants encodable and decodable.
6. Identify burst (types 100-112): byte-exact roundtrip against fixtures.
7. Fuzz target: `cargo fuzz` on `decode`.

### AGENTS.md Rules

- `RULE-S9-01`: Protocol violations kill the connection, never drop-and-continue.
- `RULE-S9-02`: Codec must not panic on any input.

---

## 10. Configuration System

### Design Decisions

- Config loaded after first client identify burst per `server-client.c:3725-3734`.
- `OptionSet` stores key-value pairs with typed `OptionValue`.
- Four-level option resolution: Pane -> Window -> Session -> Server (FALLTHROUGH per `options.c:891-903`).
- Unset semantics match `options.c:1269-1285`.

### Rust Example

```rust
// crates/mux-conf/src/lib.rs
#[derive(Debug, Clone)]
pub enum OptionValue {
    String(String),
    Number(i64),
    Boolean(bool),
    Colour(Colour),
    Style(StyleSpec),
    Array(Vec<String>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionScope { Server, Session, Window, Pane }

pub fn resolve_option(
    key: &str,
    pane: &OptionSet,
    window: &OptionSet,
    session: &OptionSet,
    server: &OptionSet,
) -> Option<OptionValue> {
    // FALLTHROUGH: Pane -> Window -> Session -> Server
    pane.get(key)
        .or_else(|| window.get(key))
        .or_else(|| session.get(key))
        .or_else(|| server.get(key))
}
```

### Test Strategy

1. FALLTHROUGH: pane option overrides window -> session -> server.
2. Unset: removing pane option falls through to window.
3. Default values: server scope has correct defaults.
4. Config parse: valid config file produces expected OptionSets.
5. Config load timing: config not loaded before first identify burst.

### AGENTS.md Rules

- `RULE-S10-01`: Config loads only after first client identify burst.
- `RULE-S10-02`: Option FALLTHROUGH: pane -> window -> session -> server per `options.c:891-903`.

---

## 11. Layout Engine

### Design Decisions

- Flat arena-based `LayoutTree` using `Vec<LayoutCell>` with index-based parent/children.
- Round-robin resize: one cell at a time per `layout.c:448-462`.
- Layout checksum matches tmux per `layout-custom.c:46-57`.
- `layout_check()` debug assertion in all layout-mutating functions.
- Split minimum enforcement per `tmux.h:100 PANE_MINIMUM`.

### Rust Example

```rust
// crates/mux-core/src/layout.rs
pub struct LayoutTree {
    pub cells: Vec<LayoutCell>,
    pub root: usize,
}

#[derive(Debug, Clone)]
pub struct LayoutCell {
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub kind: LayoutKind,
    pub x: u16, pub y: u16,
    pub sx: u16, pub sy: u16,
}

#[derive(Debug, Clone, Copy)]
pub enum LayoutKind { Horizontal, Vertical, Pane(PaneId) }

pub fn layout_resize_adjust(tree: &mut LayoutTree, node: usize, delta: i32) {
    let cell = &tree.cells[node];
    if cell.children.is_empty() { return; }
    let children: Vec<usize> = cell.children.clone();
    let mut remaining = delta;
    // Round-robin: one cell at a time per layout.c:448-462
    while remaining != 0 {
        for &child in &children {
            if remaining == 0 { break; }
            let step = if remaining > 0 { 1 } else { -1 };
            let child_cell = &tree.cells[child];
            let new_size = match cell.kind {
                LayoutKind::Horizontal => child_cell.sx as i32 + step,
                LayoutKind::Vertical => child_cell.sy as i32 + step,
                _ => continue,
            };
            if new_size >= PANE_MINIMUM as i32 {
                match cell.kind {
                    LayoutKind::Horizontal => tree.cells[child].sx = new_size as u16,
                    LayoutKind::Vertical => tree.cells[child].sy = new_size as u16,
                    _ => {}
                }
                remaining -= step;
            }
        }
    }
    debug_assert!(layout_check(tree));
}

pub fn layout_checksum(tree: &LayoutTree) -> u16 {
    // Match layout-custom.c:46-57
    let mut csum: u16 = 0;
    for cell in &tree.cells {
        let s = format!("{}x{},{}x{}", cell.x, cell.y, cell.sx, cell.sy);
        for b in s.bytes() {
            csum = (csum >> 1) | ((csum & 1) << 15);
            csum = csum.wrapping_add(b as u16);
        }
    }
    csum
}
```

### Test Strategy

1. Round-robin resize: 20-pane layout resize produces expected dimensions.
2. Layout checksum: matches tmux capture for known layout.
3. Split minimum: refuse to split below PANE_MINIMUM.
4. `proptest`: random resize never violates layout invariants.
5. Layout validation: `layout_check()` catches corrupted trees.

### AGENTS.md Rules

- `RULE-S11-01`: `debug_assert!(layout_check(...))` in all layout-mutating functions.
- `RULE-S11-02`: Format strings match tmux via `format-audit` parity corpus.

---

## 12. ORM-like Query API

### Design Decisions

- `QueryList<T>` wraps `Vec<T>` with filter/get operations.
- 18 query operators: 12 from libtmux (`LOOKUP_NAME_MAP` lines 298-312) + 6 Rust extensions.
- libtmux operators: `exact`, `eq` (alias), `iexact`, `contains`, `icontains`, `startswith`, `istartswith`, `endswith`, `iendswith`, `in`, `nin`, `regex`, `iregex`.
- Rust extensions: `lt`, `gt`, `lte`, `gte`, `ne`, `is_none`.
- `keygetter`-style nested `__` traversal per libtmux lines 45-113.
- Callable matcher support per libtmux line 535.
- `get(default)` semantics per libtmux lines 550-569.

### Rust Example

```rust
// crates/mux-query/src/lib.rs
#[derive(Debug, Clone)]
pub enum QueryOp {
    Exact, Eq, IExact,
    Contains, IContains,
    StartsWith, IStartsWith,
    EndsWith, IEndsWith,
    In, Nin,
    Regex, IRegex,
    // Rust extensions
    Lt, Gt, Lte, Gte, Ne, IsNone,
}

#[derive(Debug, Clone)]
pub struct QuerySpec {
    pub field: String,
    pub op: QueryOp,
    pub value: QueryValue,
}

#[derive(Debug, Clone)]
pub enum QueryValue {
    Str(String),
    Int(i64),
    Bool(bool),
    List(Vec<String>),
    None,
}

pub struct QueryList<T> {
    items: Vec<T>,
}

impl<T: Queryable> QueryList<T> {
    pub fn filter(&self, specs: &[QuerySpec]) -> QueryList<T> {
        let items = self.items.iter()
            .filter(|item| specs.iter().all(|spec| {
                let val = keygetter(item, &spec.field);
                apply_op(&spec.op, &val, &spec.value)
            }))
            .cloned()
            .collect();
        QueryList { items }
    }

    pub fn filter_fn<F: Fn(&T) -> bool>(&self, f: F) -> QueryList<T> {
        QueryList { items: self.items.iter().filter(|i| f(i)).cloned().collect() }
    }

    pub fn get(&self, specs: &[QuerySpec]) -> Result<&T, QueryError> {
        let filtered = self.filter(specs);
        match filtered.items.len() {
            0 => Err(QueryError::ObjectDoesNotExist),
            1 => Ok(&self.items[/* find the matched index */0]),
            _ => Err(QueryError::MultipleObjectsReturned),
        }
    }
}

/// Nested field traversal matching libtmux keygetter (lines 45-113).
fn keygetter<T: Queryable>(item: &T, path: &str) -> Option<QueryValue> {
    let parts: Vec<&str> = path.split("__").collect();
    let mut current = item.query_field(parts[0])?;
    for part in &parts[1..] {
        current = current.nested_field(part)?;
    }
    Some(current)
}
```

### Test Strategy

1. All 12 libtmux operators produce correct results against test data.
2. `eq` alias: `filter(name__eq="x")` == `filter(name__exact="x")`.
3. Nested traversal: `filter(options__status_style__exact="green")`.
4. Callable matcher: `filter_fn(|s| s.name == "work")` works.
5. `get` default: no matches returns default if provided.
6. `get` error: multiple matches raises `MultipleObjectsReturned`.
7. Rust extensions: `lt`, `gt`, `lte`, `gte`, `ne`, `is_none` produce correct results.
8. Case-insensitive: `icontains`, `iexact`, `istartswith`, `iendswith` are case-independent.

### AGENTS.md Rules

- `RULE-S12-01`: Query operators must be a superset of libtmux's 12 operators.
- `RULE-S12-02`: `keygetter` nested traversal must split on `__` per libtmux convention.

---

## 13. Runtime Architecture

### Design Decisions

- Tokio-based async runtime for IO (PTY multiplexing, socket IO, timers).
- State actor runs on a dedicated tokio task, processing events sequentially.
- `ArcSwap<GraphState>` published after every state mutation for lock-free reads.
- Effect dispatching uses typed channels (not dynamic dispatch).

### Rust Example

```rust
// crates/mux-server/src/state_actor.rs
pub async fn state_actor_loop(
    mut graph: ServerGraph,
    event_rx: mpsc::Receiver<Event>,
    state_swap: Arc<ArcSwap<GraphState>>,
    effect_tx: mpsc::Sender<Effect>,
) {
    while let Some(event) = event_rx.recv().await {
        let ctx = CoreCtx { now_ms: now_ms(), rand_u64: rand_u64() };
        let outcome = graph.apply_event(event, ctx);

        // Publish new snapshot
        let snapshot = graph.to_snapshot();
        state_swap.store(Arc::new(snapshot));

        // Dispatch effects
        for effect in outcome.effects {
            let _ = effect_tx.send(effect).await;
        }
    }
}
```

### Test Strategy

1. Single writer: state actor processes events sequentially.
2. Snapshot published: after event, `state_swap.load()` reflects mutation.
3. Effects dispatched: effects from apply_event reach effect executor.
4. Channel closed: actor exits cleanly on sender drop.

### AGENTS.md Rules

- `RULE-S13-01`: Only the state actor mutates graph state.
- `RULE-S13-02`: Snapshot publication via `ArcSwap::store`, never lock-based.

---

## 14. Server Lifecycle

### Design Decisions

- Server starts via `flock(LOCK_EX|LOCK_NB)` per `client.c:77-101`.
- Lock file guards use `Drop` impl, never manual `unlock()`.
- Subprocess test servers use SIGTERM -> SIGKILL escalation (2-second timeout).
- Config isolation: `-f /dev/null` for all test invocations.
- Clipboard isolation: `set-clipboard off` in test harnesses.

### Rust Example

```rust
// crates/mux-os/src/lock.rs
pub struct FlockGuard {
    fd: std::os::unix::io::OwnedFd,
}

impl FlockGuard {
    pub fn try_lock(path: &std::path::Path) -> Result<Self, LockError> {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(path)?;
        let fd = std::os::unix::io::OwnedFd::from(file);
        let ret = unsafe { libc::flock(std::os::unix::io::AsRawFd::as_raw_fd(&fd),
            libc::LOCK_EX | libc::LOCK_NB) };
        if ret != 0 {
            return Err(LockError::AlreadyLocked);
        }
        Ok(FlockGuard { fd })
    }
}

impl Drop for FlockGuard {
    fn drop(&mut self) {
        unsafe { libc::flock(std::os::unix::io::AsRawFd::as_raw_fd(&self.fd),
            libc::LOCK_UN); }
    }
}

// crates/mux-test-support/src/subprocess.rs
pub struct SubprocessTestServer {
    child: std::process::Child,
    socket_path: std::path::PathBuf,
}

impl Drop for SubprocessTestServer {
    fn drop(&mut self) {
        // SIGTERM first
        let _ = signal::kill(Pid::from_raw(self.child.id() as i32), Signal::SIGTERM);
        // Wait up to 2 seconds
        match self.child.wait_timeout(Duration::from_secs(2)) {
            Ok(Some(_)) => {},
            _ => {
                // Escalate to SIGKILL
                let _ = signal::kill(Pid::from_raw(self.child.id() as i32), Signal::SIGKILL);
                let _ = self.child.wait();
            }
        }
    }
}
```

### Test Strategy

1. Lock exclusivity: two processes cannot hold same lock file.
2. Lock auto-release: `FlockGuard` drop releases lock.
3. SIGTERM shutdown: server exits cleanly on SIGTERM.
4. SIGKILL escalation: hung server is killed after 2 seconds.
5. Config isolation: test server uses `-f /dev/null`.

### AGENTS.md Rules

- `RULE-S14-01`: Lock file guard via `Drop` impl, never manual `unlock()`.
- `RULE-S14-02`: Config load trigger must remain post-identify.

---

## 15. Control Mode

### Design Decisions

- Control mode notifications parsed from tmux output format.
- Notifications are hints, not authoritative state (periodic refresh reconciles).
- Pending output limit per `control.c:450-461`.
- Extended output format per `control.c:620-623`.
- Control mode startup sequence per `control.c:758-796`.

### Rust Example

```rust
// crates/mux-control/src/notification.rs
#[derive(Debug, Clone)]
pub enum ControlNotification {
    Output { pane_id: u32, data: String },
    WindowAdd { window_id: u32 },
    WindowClose { window_id: u32 },
    WindowRenamed { window_id: u32, name: String },
    SessionChanged { session_id: u32, name: String },
    SessionRenamed { name: String },
    LayoutChanged { window_id: u32, layout: String },
    PaneModeChanged { pane_id: u32 },
    ClientSessionChanged { client: String, session_id: u32 },
}

pub fn parse_notification(line: &str) -> Result<ControlNotification, ControlParseError> {
    let (prefix, rest) = line.split_once(' ')
        .ok_or(ControlParseError::MissingPrefix)?;
    match prefix {
        "%output" => parse_output(rest),
        "%window-add" => parse_window_add(rest),
        "%window-close" => parse_window_close(rest),
        "%window-renamed" => parse_window_renamed(rest),
        "%session-changed" => parse_session_changed(rest),
        "%layout-change" => parse_layout_changed(rest),
        _ => Err(ControlParseError::UnknownNotification(prefix.to_string())),
    }
}
```

### Test Strategy

1. Parse all notification types from tmux captures.
2. Unknown notification: produces descriptive error, not panic.
3. Output notification: pane ID and data extracted correctly.
4. Periodic refresh: notification-based state reconciled with full refresh.

### AGENTS.md Rules

- `RULE-S15-01`: Control notifications are hints, not authoritative.
- `RULE-S15-02`: Parser must handle unknown notification types gracefully.

---

## 16. Language Bindings [v10 updated]

### Design Decisions

- Python via PyO3 (0.26+): `Bound<'py, T>` API, `PyQueryList` with kwargs filter, `get(default=...)`.
- Node via Neon: `JsBox<JsServer>`, async operations via `deferred.settle_with`.
- Both expose the same ORM API semantics: filter with operators, get with default, callable matcher.
- Custom Python exceptions: `ObjectDoesNotExist`, `MultipleObjectsReturned`.
- Context manager support for Python `Server`.
- OTEL traceparent propagation from Python into Rust bindings.
- GIL release via `py.allow_threads()` for potentially blocking Rust operations.
- **[v10]** PyTermlet and JsTermlet: language-native Termlet handles that wrap `mux_termlet::Termlet`. These are the primary SDK entry points for testing.
- **[v10 Pass 2]** Binding handles use `Mutex<Termlet>` internally since `Termlet` has mutable state (`&mut self` methods).

### 16.1 Architecture

```
Python (PyO3)                  Node (Neon)
    |                              |
    v                              v
PyQueryList<PySession>         JsQueryList
PyTermlet  [v10]               JsTermlet  [v10]
    |                              |
    v                              v
mux-orm  (Rust ORM facade)
mux-termlet  (Termlet handle)  [v10]
    |
    v
mux-api  (StateHandle + event submission)
mux-core (pure kernel)
```

### Rust Example

```rust
// bindings/python/src/py_server.rs
use pyo3::prelude::*;

#[pyclass(name = "Server")]
pub struct PyServer {
    inner: ManagedMux,
}

#[pymethods]
impl PyServer {
    #[new]
    #[pyo3(signature = (socket_path=None, config_file=None, otel_traceparent=None))]
    fn new(
        py: Python<'_>,
        socket_path: Option<&str>,
        config_file: Option<&str>,
        otel_traceparent: Option<&str>,
    ) -> PyResult<Self> {
        let opts = MuxOptions {
            socket_path: socket_path.map(Into::into),
            config_file: config_file.map(Into::into),
            otel_traceparent: otel_traceparent.map(Into::into),
        };
        let managed = ManagedMux::start(opts)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        Ok(Self { inner: managed })
    }

    #[getter]
    fn sessions(&self) -> PyResult<PyQueryList> {
        let snap = self.inner.state_handle().load();
        let sessions: Vec<PySession> = snap.sessions.iter()
            .map(|(id, s)| PySession { id, inner: Arc::clone(s), handle: self.inner.clone() })
            .collect();
        Ok(PyQueryList::new(sessions))
    }

    fn kill_server(&self) -> PyResult<()> {
        self.inner.submit(Event::Command {
            name: "kill-server".into(), args: vec![], client_id: None,
        }).map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    fn __enter__(slf: Py<Self>) -> Py<Self> { slf }

    fn __exit__(
        &self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_val: Option<&Bound<'_, PyAny>>,
        _exc_tb: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<bool> {
        let _ = self.kill_server();
        Ok(false)
    }
}

// bindings/python/src/py_query_list.rs
#[pyclass(name = "QueryList")]
pub struct PyQueryList {
    items: Vec<PyObject>,
}

#[pymethods]
impl PyQueryList {
    #[pyo3(signature = (matcher=None, **kwargs))]
    fn filter(
        &self,
        py: Python<'_>,
        matcher: Option<PyObject>,
        kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<PyQueryList> {
        if let Some(callable) = matcher {
            let filtered: Vec<PyObject> = self.items.iter()
                .filter(|item| callable.call1(py, (item.clone_ref(py),))
                    .and_then(|result| result.is_truthy(py))
                    .unwrap_or(false))
                .cloned().collect();
            return Ok(PyQueryList { items: filtered });
        }
        let Some(kwargs) = kwargs else {
            return Ok(PyQueryList { items: self.items.clone() });
        };
        let specs = parse_py_kwargs(py, kwargs)?;
        let filtered: Vec<PyObject> = self.items.iter()
            .filter(|item| matches_all_specs(py, item, &specs))
            .cloned().collect();
        Ok(PyQueryList { items: filtered })
    }

    #[pyo3(signature = (matcher=None, default=None, **kwargs))]
    fn get(
        &self,
        py: Python<'_>,
        matcher: Option<PyObject>,
        default: Option<PyObject>,
        kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<PyObject> {
        let filtered = self.filter(py, matcher, kwargs)?;
        match filtered.items.len() {
            0 => default.ok_or_else(|| PyObjectDoesNotExist::new_err("no matching object")),
            1 => Ok(filtered.items[0].clone_ref(py)),
            _ => Err(PyMultipleObjectsReturned::new_err("multiple objects returned")),
        }
    }

    fn __len__(&self) -> usize { self.items.len() }
    fn __bool__(&self) -> bool { !self.items.is_empty() }
    fn __repr__(&self) -> String { format!("QueryList(len={})", self.items.len()) }

    fn __getitem__(&self, py: Python<'_>, idx: isize) -> PyResult<PyObject> {
        let len = self.items.len() as isize;
        let actual = if idx < 0 { len + idx } else { idx };
        if actual < 0 || actual >= len {
            return Err(pyo3::exceptions::PyIndexError::new_err("index out of range"));
        }
        Ok(self.items[actual as usize].clone_ref(py))
    }
}

// bindings/python/src/exceptions.rs
use pyo3::create_exception;
create_exception!(termforge, ObjectDoesNotExist, pyo3::exceptions::PyException);
create_exception!(termforge, MultipleObjectsReturned, pyo3::exceptions::PyException);
create_exception!(termforge, ServerError, pyo3::exceptions::PyException);

// [v10] bindings/python/src/py_termlet.rs
#[pyclass(name = "Termlet")]
pub struct PyTermlet {
    inner: std::sync::Mutex<mux_termlet::Termlet>,
}

#[pymethods]
impl PyTermlet {
    /// Spawn a new Termlet running the given command.
    #[new]
    #[pyo3(signature = (command="bash", cols=80, rows=24, env=None, cwd=None, fake=false))]
    fn new(
        command: &str,
        cols: u16,
        rows: u16,
        env: Option<Vec<(String, String)>>,
        cwd: Option<&str>,
        fake: bool,
    ) -> PyResult<Self> {
        let config = mux_termlet::TermletConfig {
            cols,
            rows,
            env: env.unwrap_or_default(),
            cwd: cwd.map(Into::into),
            backend: if fake {
                mux_termlet::PtyMode::Fake
            } else {
                mux_termlet::PtyMode::Real
            },
            ..Default::default()
        };
        let termlet = mux_termlet::Termlet::spawn(command, config)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        Ok(Self { inner: std::sync::Mutex::new(termlet) })
    }

    /// Send keystrokes to the Termlet.
    fn send_keys(&self, py: Python<'_>, keys: &str) -> PyResult<()> {
        py.allow_threads(|| {
            self.inner.lock().unwrap().send_keys(keys)
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))
        })
    }

    /// Wait for a pattern to appear in the grid output.
    #[pyo3(signature = (pattern, timeout_ms=5000))]
    fn wait_for(&self, py: Python<'_>, pattern: &str, timeout_ms: u64) -> PyResult<()> {
        py.allow_threads(|| {
            self.inner.lock().unwrap()
                .wait_for(pattern, std::time::Duration::from_millis(timeout_ms))
                .map(|_| ())
                .map_err(|e| match e {
                    mux_termlet::TermletError::WaitForTimeout { .. } =>
                        pyo3::exceptions::PyTimeoutError::new_err(e.to_string()),
                    _ => PyRuntimeError::new_err(e.to_string()),
                })
        })
    }

    /// Capture a text snapshot of the current grid state.
    fn snapshot(&self) -> PyResult<String> {
        Ok(self.inner.lock().unwrap().snapshot().to_text())
    }

    /// Resize the Termlet grid.
    fn resize(&self, cols: u16, rows: u16) -> PyResult<()> {
        self.inner.lock().unwrap().resize(cols, rows)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Kill the underlying process.
    fn kill(&self) -> PyResult<()> {
        self.inner.lock().unwrap().kill()
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Check if the underlying process is still alive.
    #[getter]
    fn is_alive(&self) -> bool {
        self.inner.lock().unwrap().is_alive()
    }

    /// Get the current grid dimensions.
    #[getter]
    fn size(&self) -> (u16, u16) {
        let s = self.inner.lock().unwrap().size();
        (s.sx, s.sy)
    }

    /// Context manager entry.
    fn __enter__(slf: Py<Self>) -> Py<Self> { slf }

    /// Context manager exit -- kills the Termlet.
    fn __exit__(
        &self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_val: Option<&Bound<'_, PyAny>>,
        _exc_tb: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<bool> {
        let _ = self.kill();
        Ok(false)
    }

    fn __repr__(&self) -> String {
        let guard = self.inner.lock().unwrap();
        let s = guard.size();
        format!("Termlet({}x{}, alive={})", s.sx, s.sy, guard.is_alive())
    }
}

// bindings/node/src/js_termlet.rs  [v10]
pub struct JsTermlet { inner: std::sync::Mutex<mux_termlet::Termlet> }
impl Finalize for JsTermlet {}

fn termlet_spawn(mut cx: FunctionContext) -> JsResult<JsBox<JsTermlet>> {
    let command = cx.argument::<JsString>(0)?.value(&mut cx);
    let opts = cx.argument_opt(1);
    let (cols, rows, fake) = parse_termlet_opts(&mut cx, opts)?;
    let config = mux_termlet::TermletConfig {
        cols, rows,
        env: vec![],
        cwd: None,
        backend: if fake { mux_termlet::PtyMode::Fake } else { mux_termlet::PtyMode::Real },
        ..Default::default()
    };
    let termlet = mux_termlet::Termlet::spawn(&command, config)
        .or_else(|e| cx.throw_error(e.to_string()))?;
    Ok(cx.boxed(JsTermlet { inner: std::sync::Mutex::new(termlet) }))
}

fn termlet_send_keys(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let termlet = cx.argument::<JsBox<JsTermlet>>(0)?;
    let keys = cx.argument::<JsString>(1)?.value(&mut cx);
    termlet.inner.lock().unwrap().send_keys(&keys)
        .or_else(|e| cx.throw_error(e.to_string()))?;
    Ok(cx.undefined())
}

fn termlet_snapshot(mut cx: FunctionContext) -> JsResult<JsString> {
    let termlet = cx.argument::<JsBox<JsTermlet>>(0)?;
    let text = termlet.inner.lock().unwrap().snapshot().to_text();
    Ok(cx.string(text))
}

fn termlet_kill(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let termlet = cx.argument::<JsBox<JsTermlet>>(0)?;
    termlet.inner.lock().unwrap().kill()
        .or_else(|e| cx.throw_error(e.to_string()))?;
    Ok(cx.undefined())
}
```

### Test Strategy

1. PyQueryList filter kwargs: `sessions.filter(name="work")` returns matching.
2. PyQueryList filter operator: `sessions.filter(name__startswith="w")` works.
3. PyQueryList callable: `sessions.filter(lambda s: s.name == "work")` works.
4. PyQueryList get default: `sessions.get(name="missing", default=None)` returns None.
5. PyQueryList get error: multiple matches raises `MultipleObjectsReturned`.
6. PyQueryList get not found: no matches raises `ObjectDoesNotExist`.
7. PyQueryList iteration: `for s in server.sessions` iterates all.
8. PyQueryList indexing: `server.sessions[0]` and `server.sessions[-1]` work.
9. Context manager: `with Server() as s` starts and kills server.
10. OTEL propagation: traceparent passed from Python into Rust bindings.
11. Node sessions: `server.sessions({ name: "work" })` filters correctly.
12. Node async: `await server.newSession("test")` resolves/rejects correctly.
13. Error mapping: Rust `CoreError::SessionNotFound` maps to Python `ObjectDoesNotExist`.
14. **[v10]** PyTermlet spawn: `Termlet("bash")` starts a shell.
15. **[v10]** PyTermlet send_keys: `termlet.send_keys("echo hello\n")` sends keystrokes.
16. **[v10]** PyTermlet snapshot: `termlet.snapshot()` returns grid text.
17. **[v10]** PyTermlet context manager: `with Termlet("bash") as t:` auto-kills on exit.
18. **[v10]** PyTermlet fake mode: `Termlet("bash", fake=True)` uses FakePtyBackend.
19. **[v10]** PyTermlet wait_for timeout: raises `TimeoutError` (not generic RuntimeError).
20. **[v10]** JsTermlet spawn: `termletSpawn("bash")` returns JsBox.
21. **[v10]** JsTermlet snapshot: `termletSnapshot(t)` returns string.
22. **[v10 Pass 2]** PyTermlet GIL release: `send_keys` and `wait_for` release GIL via `py.allow_threads()`.

### AGENTS.md Rules

- `RULE-S16-01`: Bindings must mirror ORM API. No extra mutation surfaces.
- `RULE-S16-02`: Expose OTEL context propagation via traceparent parameter.
- `RULE-S16-03`: **[v10]** Termlet bindings must expose: spawn, send_keys, wait_for, snapshot, resize, kill.
- `RULE-S16-04`: **[v10 Pass 2]** Termlet binding handles must use `Mutex<Termlet>` for thread safety.

---

## 17. CRDT Transaction Layer

### Design Decisions

- Support eventual-consistency replication between TermForge instances.
- Hybrid Logical Clock (HLC) for causal ordering.
- LWW (Last Writer Wins) registers for scalar fields (session name, options).
- OR-Set / Add-Wins Set (Observed Remove Set) for collections (windows in session, panes in window). The OR-Set semantics here are equivalent to Add-Wins -- concurrent add and remove of the same element results in the element being present.
- OpLog stores operations for merge/replay.
- CRDT layer is opt-in. Single-server mode uses direct events without HLC overhead.

### Rust Example

```rust
// crates/mux-crdt/src/hlc.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct HLC {
    pub wall_ms: i64,
    pub counter: u32,
    pub node_id: u64,
}

impl HLC {
    pub fn now(prev: &HLC, wall_ms: i64, node_id: u64) -> HLC {
        let wall = wall_ms.max(prev.wall_ms);
        let counter = if wall == prev.wall_ms { prev.counter + 1 } else { 0 };
        HLC { wall_ms: wall, counter, node_id }
    }

    pub fn merge(local: &HLC, remote: &HLC, wall_ms: i64, node_id: u64) -> HLC {
        let wall = wall_ms.max(local.wall_ms).max(remote.wall_ms);
        let counter = if wall == local.wall_ms && wall == remote.wall_ms {
            local.counter.max(remote.counter) + 1
        } else if wall == local.wall_ms { local.counter + 1 }
          else if wall == remote.wall_ms { remote.counter + 1 }
          else { 0 };
        HLC { wall_ms: wall, counter, node_id }
    }
}

// crates/mux-crdt/src/lww.rs
#[derive(Debug, Clone)]
pub struct LWWRegister<T> {
    pub value: T,
    pub timestamp: HLC,
}

impl<T: Clone> LWWRegister<T> {
    pub fn set(&mut self, value: T, ts: HLC) {
        if ts > self.timestamp {
            self.value = value;
            self.timestamp = ts;
        }
    }

    pub fn merge(&mut self, other: &LWWRegister<T>) {
        if other.timestamp > self.timestamp {
            self.value = other.value.clone();
            self.timestamp = other.timestamp;
        }
    }
}

// crates/mux-crdt/src/or_set.rs
/// Observed-Remove Set (Add-Wins semantics):
/// Concurrent add + remove of the same element results in element being present.
#[derive(Debug, Clone)]
pub struct OrSet<T: Eq + std::hash::Hash> {
    elements: HashMap<T, HashSet<(u64, HLC)>>,
    tombstones: HashMap<T, HashSet<(u64, HLC)>>,
}

impl<T: Eq + std::hash::Hash + Clone> OrSet<T> {
    pub fn add(&mut self, element: T, node_id: u64, ts: HLC) {
        self.elements.entry(element).or_default().insert((node_id, ts));
    }

    pub fn remove(&mut self, element: &T, _node_id: u64, _ts: HLC) {
        if let Some(tags) = self.elements.get(element) {
            self.tombstones.entry(element.clone()).or_default()
                .extend(tags.iter().cloned());
        }
    }

    pub fn contains(&self, element: &T) -> bool {
        let adds = self.elements.get(element).map_or(0, |s| s.len());
        let removes = self.tombstones.get(element).map_or(0, |s| s.len());
        adds > removes
    }
}

// crates/mux-crdt/src/oplog.rs
#[derive(Debug, Clone)]
pub struct OpLog {
    ops: Vec<(HLC, Event)>,
}

impl OpLog {
    pub fn append(&mut self, ts: HLC, event: Event) {
        self.ops.push((ts, event));
    }

    pub fn since(&self, ts: HLC) -> impl Iterator<Item = &(HLC, Event)> {
        self.ops.iter().filter(move |(op_ts, _)| *op_ts > ts)
    }

    pub fn merge(&mut self, remote: &OpLog) {
        for (ts, event) in &remote.ops {
            if !self.ops.iter().any(|(t, _)| t == ts) {
                self.ops.push((*ts, event.clone()));
            }
        }
        self.ops.sort_by_key(|(ts, _)| *ts);
    }
}
```

### Test Strategy

1. HLC monotonicity: after merge, HLC is strictly greater than both inputs.
2. LWW convergence: two replicas with concurrent writes converge to same value.
3. OR-Set add-remove: add + remove + re-add -> element present.
4. OR-Set concurrent add: two nodes add different elements, merge has both.
5. OpLog merge idempotent: merging same log twice does not duplicate ops.
6. Deterministic replay: OpLog replayed in HLC order produces deterministic graph.

### AGENTS.md Rules

- `RULE-S17-01`: CRDT layer is additive, not required for single-server mode.
- `RULE-S17-02`: No network code in `mux-crdt` crate.

---

## 18. Security Model

### Design Decisions

- Socket permissions: directory 0700, socket 0600, created with `umask(0o177)`.
- Symlink check on entire socket path (every ancestor).
- SCM_RIGHTS for fd passing between client and server.
- No SUID/SGID bits on any binary.
- All processes run as the calling user.

### Rust Example

```rust
// crates/mux-os/src/socket.rs
pub fn create_socket(path: &std::path::Path) -> Result<std::os::unix::net::UnixListener, SocketError> {
    let dir = path.parent().ok_or(SocketError::InvalidPath)?;
    std::fs::create_dir_all(dir)?;

    let metadata = std::fs::metadata(dir)?;
    let mut perms = metadata.permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o700);
    std::fs::set_permissions(dir, perms)?;

    for ancestor in path.ancestors() {
        if ancestor.is_symlink() {
            return Err(SocketError::SymlinkInPath(ancestor.to_owned()));
        }
    }

    let old_umask = unsafe { libc::umask(0o177) };
    let result = std::os::unix::net::UnixListener::bind(path);
    unsafe { libc::umask(old_umask); }
    result.map_err(SocketError::Bind)
}
```

### Test Strategy

1. Socket permissions: verify socket created with mode 0600.
2. Directory permissions: verify socket dir has mode 0700.
3. Symlink rejection: socket path with symlink fails creation.
4. FD passing: SCM_RIGHTS roundtrip preserves fd.
5. umask restoration: after socket creation, umask is restored to original value.

### AGENTS.md Rules

- `RULE-S18-01`: All `unsafe` socket/fd code lives in `mux-os` only.
- `RULE-S18-02`: Socket path length checked against 108-byte limit before creation.

---

## 19. OpenTelemetry [v10 updated]

### Design Decisions

Architecture directly modeled on vibe-tmux `mux-otel` crate, verified against `otel.rs`. Key decisions:

1. **Dual provider**: `OTEL_PROVIDER` (server/TUI, installed global) and `MUX_CLIENT_PROVIDER` (client spans, lazy init, NOT installed global).
2. **Thread-local** `TRACE_HEADERS_STACK` with `TraceHeadersGuard` RAII cleanup.
3. **Process-level** `PROCESS_TRACE_HEADERS` for cross-thread header sharing.
4. **Composite propagator**: `BaggagePropagator` + `TraceContextPropagator`.
5. **OnceLock<Mutex<Option<OtelProvider>>>** for safe one-time init.
6. **Env-var activation**: `TERMFORGE_OTEL=1` or `OTEL_EXPORTER_OTLP_ENDPOINT` set.
7. **Bounded shutdown**: `runtime.shutdown_timeout(200ms)` in `OtelProvider::drop` to avoid blocking short-lived processes.

### Rust Example

```rust
// crates/mux-otel/src/provider.rs
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_sdk::logs::SdkLoggerProvider;

pub struct OtelProvider {
    logger: Option<SdkLoggerProvider>,
    tracer_provider: Option<SdkTracerProvider>,
    tracer: Option<opentelemetry_sdk::trace::Tracer>,
    runtime: Option<tokio::runtime::Runtime>,
}

impl Drop for OtelProvider {
    fn drop(&mut self) {
        if let Some(logger) = &self.logger { let _ = logger.shutdown(); }
        if let Some(provider) = &self.tracer_provider { let _ = provider.shutdown(); }
        if let Some(runtime) = self.runtime.take() {
            runtime.shutdown_timeout(Duration::from_millis(200));
        }
    }
}

// Dual provider statics
static OTEL_PROVIDER: OnceLock<Mutex<Option<OtelProvider>>> = OnceLock::new();
static MUX_CLIENT_PROVIDER: OnceLock<Mutex<Option<OtelProvider>>> = OnceLock::new();

// Thread-local trace headers stack
thread_local! {
    static TRACE_HEADERS_STACK: RefCell<Vec<TraceHeaders>> = const { RefCell::new(Vec::new()) };
}

#[derive(Debug, Clone)]
pub struct TraceHeaders {
    pub traceparent: String,
    pub tracestate: Option<String>,
    pub baggage: Option<String>,
}

pub struct TraceHeadersGuard { _private: () }

impl Drop for TraceHeadersGuard {
    fn drop(&mut self) {
        TRACE_HEADERS_STACK.with(|stack| { stack.borrow_mut().pop(); });
    }
}

pub fn push_trace_headers(headers: TraceHeaders) -> TraceHeadersGuard {
    TRACE_HEADERS_STACK.with(|stack| { stack.borrow_mut().push(headers); });
    TraceHeadersGuard { _private: () }
}

// Composite propagator: Baggage + TraceContext
fn composite_propagator() -> TextMapCompositePropagator {
    let propagators: Vec<Box<dyn TextMapPropagator + Send + Sync>> = vec![
        Box::new(BaggagePropagator::new()),
        Box::new(TraceContextPropagator::new()),
    ];
    TextMapCompositePropagator::new(propagators)
}

// HeaderCarrier for inject/extract
#[derive(Default)]
struct HeaderCarrier(HashMap<String, String>);

impl opentelemetry::propagation::Injector for HeaderCarrier {
    fn set(&mut self, key: &str, value: String) {
        self.0.insert(key.to_ascii_lowercase(), value);
    }
}

impl opentelemetry::propagation::Extractor for HeaderCarrier {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(&key.to_ascii_lowercase()).map(String::as_str)
    }
    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(String::as_str).collect()
    }
}

// Force flush both providers
pub fn force_flush() -> bool {
    let mut ok = true;
    for lock in [&OTEL_PROVIDER, &MUX_CLIENT_PROVIDER] {
        if let Some(slot) = lock.get() {
            let guard = slot.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(provider) = guard.as_ref() {
                ok &= provider.tracer_provider.as_ref()
                    .is_none_or(|tp| tp.force_flush().is_ok());
                ok &= provider.logger.as_ref()
                    .is_none_or(|lp| lp.force_flush().is_ok());
            }
        }
    }
    ok
}

pub fn shutdown() -> bool {
    let ok = force_flush();
    for lock in [&OTEL_PROVIDER, &MUX_CLIENT_PROVIDER] {
        if let Some(slot) = lock.get() {
            let provider = {
                let mut guard = slot.lock().unwrap_or_else(|e| e.into_inner());
                guard.take()
            };
            drop(provider);
        }
    }
    ok
}
```

### 19.1 Span Naming Convention

| Span Name | Kind | Source |
|---|---|---|
| `termforge.server.start` | Internal | Server startup |
| `termforge.server.event` | Internal | Event processing |
| `termforge.server.effect` | Internal | Effect dispatch |
| `termforge.client.connect` | Client | Client connection |
| `termforge.client.command` | Client | Command execution |
| `termforge.binding.call` | Client | Python/Node binding call |
| `termforge.proto.decode` | Internal | Protocol decode |
| `termforge.proto.encode` | Internal | Protocol encode |
| `termforge.termlet.spawn` | Internal | **[v10]** Termlet spawn |
| `termforge.termlet.interact` | Internal | **[v10]** Termlet send_keys/wait_for |
| `termforge.termlet.snapshot` | Internal | **[v10]** Termlet snapshot capture |

### Test Strategy

1. Dual provider init: both `OTEL_PROVIDER` and `MUX_CLIENT_PROVIDER` initialized correctly.
2. Thread-local isolation: child thread does not see parent's trace headers.
3. Guard stack semantics: push A, push B, drop B, current = A.
4. Composite propagator: inject + extract roundtrips traceparent + tracestate + baggage.
5. Env enable: `TERMFORGE_OTEL=1` enables; unset disables.
6. Shutdown idempotent: calling `shutdown()` twice does not panic.
7. Force flush both: `force_flush()` flushes both providers.
8. Export filter: `mux_*` allowed, `h2`/`tonic` denied.
9. Client span lazy init: first `enter_mux_client_span` initializes `MUX_CLIENT_PROVIDER`.
10. Runtime timeout: OtelProvider drop completes within 200ms (no hang).
11. HeaderCarrier roundtrip: inject into carrier, extract from carrier, headers match.
12. Protocol resolution: gRPC inferred for port 4317; HTTP for port 4318.

### AGENTS.md Rules

- `RULE-S19-01`: OTEL is always behind env-flag guard.
- `RULE-S19-02`: Force flush both providers before process exit.
- `RULE-S19-03`: **[v10]** OTEL must never block Termlet hot paths on exporter latency.

---

## 20. tmux Version Management

### Design Decisions

Build and cache versioned tmux binaries from source for integration testing. Directly modeled on vibe-tmux `tmux-builder` crate. Key patterns:

- BLAKE3 hash for cache key computation with `flags_fingerprint`.
- Full 64-char hex via `hex_32`, truncated to 12 chars at call site (`&cfg[..12]` at tmux-builder line 543).
- File-based locking with `fs2::FileExt` and `Drop`-based `LockGuard`.
- Atomic build publication via `sibling_tmp_dir` + `std::fs::rename`.
- Binary validation via `tmux -V`.
- HOME-based fallback for repo clone destination.

### Rust Example

```rust
// tools/tmux-builder/src/cache.rs
use blake3;

fn hex_32(bytes: [u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0xf) as usize] as char);
    }
    out
}

fn flags_fingerprint(flags: &[String]) -> String {
    let mut data = Vec::new();
    for flag in flags {
        data.extend_from_slice(flag.as_bytes());
        data.push(0); // null separator between flags
    }
    let hash = blake3::hash(&data);
    hex_32(*hash.as_bytes())
    // NOTE: caller truncates to 12 chars: &result[..12]
}

pub fn compute_cache_key(
    tag: &str,
    host: &str,
    os_id: &str,
    os_version: &str,
    configure_flags: &[String],
    make_flags: &[String],
    builder_version: &str,
) -> String {
    let version = tag.strip_prefix("tmux-").unwrap_or(tag);
    let cfg = flags_fingerprint(configure_flags);
    let mk = flags_fingerprint(make_flags);
    format!(
        "tmux-{version}__{host}__{os}-{os_ver}__cfg{cfg}__mk{mk}__tb{tool}",
        version = sanitize_token(version),
        host = sanitize_token(host),
        os = sanitize_token(os_id),
        os_ver = sanitize_token(os_version),
        cfg = &cfg[..12],
        mk = &mk[..12],
        tool = sanitize_token(builder_version),
    )
}

// tools/tmux-builder/src/lock.rs
#[derive(Debug)]
struct LockGuard {
    file: std::fs::File,
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

fn validate_tmux_binary(path: &std::path::Path) -> Result<String, anyhow::Error> {
    let output = std::process::Command::new(path).arg("-V").output()?;
    if !output.status.success() {
        anyhow::bail!("tmux -V failed at {}", path.display());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
```

### Test Strategy

1. Cache key determinism: same inputs produce same cache key.
2. Cache key uniqueness: different configure_flags produce different key.
3. BLAKE3 fingerprint: empty flags produce consistent hash.
4. Lock exclusivity: two threads cannot hold same lock_cache_key simultaneously.
5. Lock auto-release: LockGuard drop releases file lock.
6. Atomic publish: concurrent readers never see partial build.
7. Binary validation: valid tmux binary returns version string.
8. Binary validation failure: invalid binary produces descriptive error.
9. Fingerprint truncation: 12-char prefix is unique across typical flag sets.

### AGENTS.md Rules

- `RULE-S20-01`: Build cache keys must be deterministic and collision-free.
- `RULE-S20-02`: Atomic rename for build publication; never multi-step copy.

---

## 21. Test Support and Fake PTY

### Design Decisions

- Three-layer socket validation verified against vibe-tmux `path_guard.rs`.
- PathGuard with permission hardening (0700 on socket directory).
- FakePtyBackend with ScenarioRecorder for deterministic replay.
- **[v10 Pass 2]** `ScenarioStep` with `in_bytes`/`out_bytes` for structured FakePty fixtures (from GPT).
- Dual-mode test harness: in-process (FakePty) and subprocess (real PTY).
- Subprocess shutdown: SIGTERM first, then SIGKILL escalation after 2 seconds.
- Config isolation: every test invocation uses `-f /dev/null`.
- Clipboard isolation: `set-clipboard off` in all test harnesses.

### Rust Example

```rust
// crates/mux-test-support/src/path_guard.rs
use anyhow::{Result, bail};
use std::path::{Path, PathBuf};

/// Layer 1: Refuse to use tmux default socket name.
pub fn ensure_not_default_socket_name(socket_name: &str) -> Result<()> {
    if socket_name.trim().is_empty() {
        bail!("socket name must be non-empty");
    }
    if socket_name == "default" {
        bail!("refusing to use tmux default socket name: default");
    }
    Ok(())
}

/// Layer 2: Socket must be within the test harness temp directory.
pub fn ensure_socket_within_tempdir(socket_path: &Path, tempdir: &Path) -> Result<()> {
    if !socket_path.starts_with(tempdir) {
        bail!("refusing to operate on socket outside harness temp dir: \
            socket={} tempdir={}", socket_path.display(), tempdir.display());
    }
    Ok(())
}

/// Layer 3: Socket must not match the $TMUX env var.
pub fn ensure_socket_not_tmux_env(socket_path: &Path) -> Result<()> {
    let tmux = std::env::var("TMUX").ok();
    ensure_socket_not_tmux_env_value(socket_path, tmux.as_deref())
}

// crates/mux-pty/src/backend.rs
pub trait PtyBackend: Send {
    fn spawn(&mut self, pane_id: PaneId, command: &[String],
        cwd: Option<&str>, size: PaneSize) -> Result<(), PtyError>;
    fn write(&mut self, pane_id: PaneId, data: &[u8]) -> Result<(), PtyError>;
    fn resize(&mut self, pane_id: PaneId, size: PaneSize) -> Result<(), PtyError>;
    fn kill(&mut self, pane_id: PaneId, signal: i32) -> Result<(), PtyError>;
    fn read_events(&mut self) -> Vec<PtyEvent>;
}

// crates/mux-pty-fake/src/lib.rs
pub struct FakePtyBackend {
    panes: HashMap<PaneId, FakePane>,
    pending_events: Vec<PtyEvent>,
}

impl FakePtyBackend {
    pub fn inject_output(&mut self, pane_id: PaneId, data: Vec<u8>) {
        self.pending_events.push(PtyEvent::Output { pane_id, data });
    }
}

impl PtyBackend for FakePtyBackend {
    fn spawn(&mut self, pane_id: PaneId, _command: &[String],
        _cwd: Option<&str>, size: PaneSize) -> Result<(), PtyError> {
        self.panes.insert(pane_id, FakePane {
            size, input_queue: Vec::new(), output_queue: Vec::new(), exited: false,
        });
        Ok(())
    }

    fn write(&mut self, pane_id: PaneId, data: &[u8]) -> Result<(), PtyError> {
        if let Some(pane) = self.panes.get_mut(&pane_id) {
            pane.input_queue.push(data.to_vec());
        }
        Ok(())
    }

    fn resize(&mut self, pane_id: PaneId, size: PaneSize) -> Result<(), PtyError> {
        if let Some(pane) = self.panes.get_mut(&pane_id) { pane.size = size; }
        Ok(())
    }

    fn kill(&mut self, pane_id: PaneId, _signal: i32) -> Result<(), PtyError> {
        if let Some(pane) = self.panes.get_mut(&pane_id) {
            pane.exited = true;
            self.pending_events.push(PtyEvent::Exited { pane_id, status: 0 });
        }
        Ok(())
    }

    fn read_events(&mut self) -> Vec<PtyEvent> {
        std::mem::take(&mut self.pending_events)
    }
}

/// [v10 Pass 2] Structured scenario step for FakePty fixtures (from GPT).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScenarioStep {
    pub in_bytes: Vec<u8>,
    pub out_bytes: Vec<u8>,
}

/// A scenario is a sequence of steps for deterministic replay.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Scenario {
    pub steps: Vec<ScenarioStep>,
    pub initial_size: PaneSize,
}
```

### Test Strategy

1. Default socket rejected: `ensure_not_default_socket_name("default")` fails.
2. Empty socket rejected: `ensure_not_default_socket_name("")` fails.
3. Socket within tempdir: path inside tempdir passes; path outside fails.
4. Socket not TMUX env: socket matching $TMUX rejected.
5. PathGuard cleanup: temp dir removed on drop.
6. Permission hardening: socket dir has mode 0700.
7. FakePty inject output: injected data appears in `read_events()`.
8. FakePty spawn/kill lifecycle: spawn -> write -> kill -> Exited event.
9. Scenario roundtrip: record -> save -> load -> replay produces same grid.
10. **[v10 Pass 2]** ScenarioStep serialization: JSON roundtrip preserves bytes.

### AGENTS.md Rules

- `RULE-S21-01`: Tests must not operate on live user tmux sessions.
- `RULE-S21-02`: PathGuard three-layer validation required for all test fixtures.
- `RULE-S21-03`: **[v10 Pass 2]** FakePty scenarios must be versioned JSON fixtures.

---

## 22. Binding Test Frameworks [v10 updated]

### Design Decisions

- Python tests via pytest with fixture chain: `server` -> `session` -> `window` -> `pane`.
- Node tests via vitest with equivalent fixture chain.
- Dual-mode parametrized tests run every test against both in-process and subprocess servers.
- OTEL integration tests verify traceparent propagation from Python.
- Custom exception types (`ObjectDoesNotExist`, `MultipleObjectsReturned`) tested in both languages.
- **[v10]** Termlet fixtures: pytest `termlet` fixture spawns a Termlet with automatic cleanup. vitest `useTermlet()` helper does the same.

### Rust Example (Python)

```python
# bindings/python/tests/conftest.py
import pytest
from termforge import Server, Termlet

@pytest.fixture
def server():
    """In-process TermForge server with automatic cleanup."""
    with Server() as s:
        yield s

@pytest.fixture
def session(server):
    """Server with one session."""
    return server.new_session("test")

@pytest.fixture
def window(session):
    """Session with one window."""
    return session.windows[0]

# [v10] Termlet fixtures
@pytest.fixture
def termlet():
    """Spawn a Termlet running bash with automatic cleanup.

    This is the primary entry point for testing CLI tools,
    shell scripts, or interactive programs from Python.
    """
    with Termlet("bash", cols=80, rows=24) as t:
        yield t

@pytest.fixture
def fake_termlet():
    """Spawn a Termlet with FakePtyBackend for deterministic tests."""
    with Termlet("bash", cols=80, rows=24, fake=True) as t:
        yield t

@pytest.fixture(params=["real", "fake"])
def any_termlet(request):
    """Parametrized fixture: same test runs with real and fake PTY."""
    fake = request.param == "fake"
    with Termlet("bash", cols=80, rows=24, fake=fake) as t:
        yield t
```

### Rust Example (Python Tests)

```python
# bindings/python/tests/test_query.py
def test_filter_sessions(server):
    server.new_session("alpha")
    server.new_session("beta")
    assert len(server.sessions.filter(session_name="alpha")) == 1
    assert len(server.sessions.filter(session_name__startswith="a")) == 1

def test_get_session(server):
    server.new_session("work")
    s = server.sessions.get(session_name="work")
    assert s.session_name == "work"

def test_get_not_found(server):
    from termforge import ObjectDoesNotExist
    with pytest.raises(ObjectDoesNotExist):
        server.sessions.get(session_name="missing")

# [v10] Termlet tests
# bindings/python/tests/test_termlet.py
def test_termlet_echo(termlet):
    """Basic send-keys and snapshot test."""
    termlet.send_keys("echo hello\n")
    termlet.wait_for("hello", timeout_ms=5000)
    snapshot = termlet.snapshot()
    assert "hello" in snapshot

def test_termlet_resize(termlet):
    """Verify programmatic resize."""
    assert termlet.size == (80, 24)
    termlet.resize(120, 40)
    assert termlet.size == (120, 40)

def test_termlet_context_manager():
    """Context manager auto-kills on exit."""
    from termforge import Termlet
    with Termlet("bash") as t:
        t.send_keys("echo managed\n")
        t.wait_for("managed")
    # t is killed here -- no leaked processes

def test_termlet_wait_for_timeout():
    """wait_for raises TimeoutError on timeout."""
    from termforge import Termlet
    with Termlet("bash") as t:
        with pytest.raises(TimeoutError):
            t.wait_for("this_will_never_appear", timeout_ms=100)
```

### Rust Example (Node.js)

```javascript
// bindings/node/tests/termlet.test.js
import { describe, it, expect } from 'vitest';
import { useTermlet } from 'termforge';

describe('Termlet', () => {
    it('spawns and captures output', () => {
        const t = useTermlet('bash', { cols: 80, rows: 24 });
        try {
            t.sendKeys('echo hello\n');
            t.waitFor('hello', 5000);
            const snapshot = t.snapshot();
            expect(snapshot).toContain('hello');
        } finally {
            t.kill();
        }
    });

    it('supports fake mode for deterministic tests', () => {
        const t = useTermlet('bash', { cols: 80, rows: 24, fake: true });
        try {
            t.sendKeys('echo test\n');
            const snapshot = t.snapshot();
            expect(typeof snapshot).toBe('string');
        } finally {
            t.kill();
        }
    });
});
```

### Test Strategy

1. pytest fixtures: `server`, `session`, `window` chain correctly.
2. Dual-mode parametrize: same test runs in-process and subprocess.
3. OTEL propagation: `TRACEPARENT` env var reaches Rust.
4. Node filter: `sessions({ name: "work" })` returns matching.
5. Error types: Python `ObjectDoesNotExist` and `MultipleObjectsReturned` raised correctly.
6. Context manager: `with Server()` cleans up on exit.
7. **[v10]** pytest `termlet` fixture: spawns, yields, auto-kills.
8. **[v10]** pytest `fake_termlet` fixture: uses FakePtyBackend.
9. **[v10]** pytest `any_termlet` parametrize: same test, real and fake.
10. **[v10]** vitest `useTermlet()`: spawns, returns handle with kill cleanup.
11. **[v10]** Termlet send_keys + snapshot roundtrip: both languages.
12. **[v10]** Termlet wait_for timeout: raises `TimeoutError` in Python.
13. **[v10]** Termlet context manager: no leaked processes.

### AGENTS.md Rules

- `RULE-S22-01`: Binding tests must use the same ORM semantics as core tests.
- `RULE-S22-02`: Dual-mode fixture required for all integration tests.
- `RULE-S22-03`: **[v10]** Termlet fixtures must auto-kill on cleanup. No leaked PTY processes.
- `RULE-S22-04`: **[v10]** Termlet tests must run in both real and fake PTY modes where applicable.

---

## 23. Test Framework and Harness Design

### Design Decisions

- Custom VT100 parser testing against `input.c` state table.
- Grid snapshot testing via `insta`.
- Format engine parity testing via `format-audit` corpus.
- Key binding parity testing via generated tests from `key-bindings.c`.
- Property testing with `proptest` for protocol roundtrip and layout resize.
- Fuzz targets for proto decode, VT100 parse, and config parse.
- **[v10 Pass 2]** Three-tier harness: unit (pure), integration (runtime), compatibility (real tmux).
- **[v10 Pass 2]** Flaky-test quarantine requires replay log before quarantine (from GPT).

### Rust Example

```rust
// VT100 parser testing
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_text() {
        let mut grid = Grid::new(80, 24);
        let mut parser = VtParser::new();
        parser.parse(b"Hello, World!", &mut grid);
        assert_eq!(grid.row_text(0), "Hello, World!");
    }

    #[test]
    fn cursor_movement() {
        let mut grid = Grid::new(80, 24);
        let mut parser = VtParser::new();
        parser.parse(b"\x1b[5;10HX", &mut grid);
        assert_eq!(grid.cell(4, 9).ch, 'X');
    }
}

// Grid snapshot testing with insta
#[test]
fn htop_rendering() {
    let mut grid = Grid::new(120, 40);
    let mut parser = VtParser::new();
    let fixture = std::fs::read("fixtures/htop-output.bin").unwrap();
    parser.parse(&fixture, &mut grid);
    insta::assert_snapshot!(grid.to_text());
}

// Property testing
use proptest::prelude::*;

proptest! {
    #[test]
    fn imsg_roundtrip(
        msg_type in 0u32..300,
        payload in prop::collection::vec(any::<u8>(), 0..1000)
    ) {
        let frame = ImsgFrame {
            header: ImsgHdr { msg_type, len: (IMSG_HDR_SIZE + payload.len()) as u32,
                peerid: 0, pid: 0, has_fd: false },
            payload: payload.into(),
        };
        let mut buf = BytesMut::new();
        let codec = ImsgCodec::new();
        codec.encode(&frame, &mut buf);
        let decoded = codec.decode(&mut buf);
        prop_assert!(matches!(decoded, DecodeOutcome::Ok(_)));
    }

    #[test]
    fn layout_resize_never_panics(delta in -100i32..100, n_children in 1usize..20) {
        let mut tree = make_test_layout(n_children, 200, 50);
        layout_resize_adjust(&mut tree, tree.root, delta);
        prop_assert!(layout_check(&tree));
    }
}

// Fuzz targets
// fuzz/fuzz_targets/proto_decode.rs
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let mut codec = ImsgCodec::new();
    let mut buf = BytesMut::from(data);
    let _ = codec.decode(&mut buf);
});

/// [v10 Pass 2] Three-tier harness classification.
pub enum HarnessTier {
    /// Unit: pure functions, no IO, no timing.
    Unit,
    /// Integration: in-process runtime, FakePty or real PTY.
    Integration,
    /// Compatibility: real tmux binary, subprocess mode.
    Compatibility,
}
```

### Test Strategy

1. VT100 ASCII: plain text rendered correctly on grid.
2. VT100 CSI: cursor positioning, colors, attributes work.
3. Grid snapshot: insta snapshots match expected output.
4. Format parity: 200+ format variables match tmux.
5. Key binding parity: all 4 key tables match tmux defaults.
6. Protocol proptest: random payloads roundtrip through codec.
7. Layout proptest: random resize never invalidates layout.
8. Fuzz: proto, VT100, config parsers survive random input.
9. **[v10 Pass 2]** Flaky test quarantine: emit replay log before quarantining.

### AGENTS.md Rules

- `RULE-S23-01`: Every parser must have a fuzz target.
- `RULE-S23-02`: Parity tests reference tmux source line numbers.
- `RULE-S23-03`: **[v10 Pass 2]** Flaky tests must emit replay logs before quarantine.

---

## 24. Performance Targets [v10 updated]

### Design Decisions

- Criterion benchmarks as the standard measurement tool.
- Conservative targets based on known alacritty/tmux performance ranges.
- CI gate at 130% threshold on nightly only (warn, not hard fail).
- All benchmarks measure throughput where applicable.
- **[v10]** Termlet-specific benchmarks for spawn and snapshot latency.

### Performance Targets Table

| ID | Benchmark | Target | CI Gate |
|---|---|---|---|
| B1 | VT100 ASCII 1MB | > 300 MB/s | Warn if < 250 MB/s |
| B2 | VT100 CSI heavy | > 100 MB/s | Warn if < 80 MB/s |
| B3 | Proto decode 10k frames | > 500 MB/s | Warn if < 400 MB/s |
| B4 | Layout resize 20 panes | < 50 us | Warn if > 100 us |
| B5 | Snapshot 50 panes | < 1 ms | Warn if > 2 ms |
| B6 | Format expand (status) | < 50 us | Warn if > 100 us |
| B7 | Config parse 500 lines | < 5 ms | Warn if > 10 ms |
| B8 | Option resolve 4-level | < 100 ns | Warn if > 200 ns |
| B9 | Control notification parse | < 200 ns | Warn if > 500 ns |
| B10 | **[v10]** Termlet spawn (FakePty) | < 500 us | Warn if > 1 ms |
| B11 | **[v10]** Termlet snapshot (80x24) | < 50 us | Warn if > 100 us |
| B12 | **[v10]** Termlet spawn (real PTY) | < 50 ms | Warn if > 100 ms |

### Rust Example

```rust
// crates/mux-bench/benches/termlet.rs  [v10]
use criterion::{criterion_group, criterion_main, Criterion};

fn termlet_spawn_fake(c: &mut Criterion) {
    c.bench_function("termlet-spawn-fake", |b| {
        b.iter(|| {
            let mut termlet = mux_termlet::Termlet::spawn("echo", mux_termlet::TermletConfig {
                cols: 80, rows: 24,
                backend: mux_termlet::PtyMode::Fake,
                ..Default::default()
            }).unwrap();
            termlet.kill().unwrap();
        });
    });
}

fn termlet_snapshot(c: &mut Criterion) {
    let mut termlet = mux_termlet::Termlet::spawn("echo", mux_termlet::TermletConfig {
        cols: 80, rows: 24,
        backend: mux_termlet::PtyMode::Fake,
        ..Default::default()
    }).unwrap();
    termlet.send_keys("Hello, World!\n").unwrap();
    c.bench_function("termlet-snapshot-80x24", |b| {
        b.iter(|| {
            let _snap = termlet.snapshot();
        });
    });
}

criterion_group!(termlet_benches, termlet_spawn_fake, termlet_snapshot);
criterion_main!(termlet_benches);
```

### Test Strategy

1. Criterion benchmarks run nightly.
2. Regression flag at 130% of baseline.
3. Baseline established from 3 consecutive median runs.
4. No hard fail on performance (warn only). Hard fail reserved for functional gates.
5. **[v10]** Termlet benchmarks included in nightly run.
6. **[v10 Pass 2]** Benchmark scripts pin CPU governor where possible (from GPT).

### AGENTS.md Rules

- `RULE-S24-01`: Performance baselines re-established after architecture changes.
- `RULE-S24-02`: Benchmarks must cover hot-path code (VT100, proto, layout).
- `RULE-S24-03`: **[v10]** Termlet spawn benchmark must stay under 500us for FakePty mode.
- `RULE-S24-04`: **[v10 Pass 2]** Performance regressions > 30% block release.

---

## 25. Visual Client / TUI [v10 updated]

### Design Decisions

- ViewModel pattern: pure function `(GraphState, ClientId, TermSize) -> ViewModel`.
- ratatui-based rendering with crossterm terminal IO.
- ViewModel supports snapshot testing with `insta`.
- TUI can attach to both TermForge server (via StateHandle) and real tmux (via control mode).
- **[v10 Pass 2]** TermletViewModel for debug inspector panel (from GPT): shows Termlet grid preview and process status.

### Rust Example

```rust
// crates/mux-view/src/lib.rs
pub struct ViewModel {
    pub status_top: StatusLine,
    pub status_bottom: StatusLine,
    pub panes: Vec<PaneView>,
    pub borders: Vec<Border>,
    pub mode_indicator: Option<String>,
}

pub struct PaneView {
    pub x: u16, pub y: u16,
    pub width: u16, pub height: u16,
    pub grid_rows: Vec<Vec<StyledCell>>,
    pub is_active: bool,
    pub is_zoomed: bool,
}

/// Pure function: no IO, no side effects. Suitable for snapshot testing.
pub fn build_view_model(
    state: &GraphState,
    client_id: ClientId,
    term_size: (u16, u16),
) -> ViewModel {
    let (_cols, _rows) = term_size;
    todo!()
}

/// TUI can target TermForge server or real tmux.
pub enum TuiTarget {
    TermForge { state_handle: StateHandle },
    RealTmux { socket_path: PathBuf },
}

/// [v10 Pass 2] Debug inspector panel for Termlet state (from GPT).
pub struct TermletViewModel {
    pub pane_id: PaneId,
    pub alive: bool,
    pub grid_preview: String,
    pub size: (u16, u16),
}

impl TermletViewModel {
    pub fn from_termlet(termlet: &mux_termlet::Termlet) -> Self {
        let snap = termlet.snapshot();
        let s = termlet.size();
        Self {
            pane_id: termlet.pane_id(),
            alive: termlet.is_alive(),
            grid_preview: snap.to_text(),
            size: (s.sx, s.sy),
        }
    }
}
```

### Test Strategy

1. ViewModel snapshot: pure ViewModel from test GraphState matches `insta` snapshot.
2. Status line format: format expansion produces correct status text.
3. Pane layout: ViewModel pane positions match layout tree.
4. Key conversion: crossterm key events map to mux-core Key variants.
5. Border rendering: correct border characters at pane boundaries.
6. Zoomed pane: single pane fills entire area when zoomed.
7. Resize: terminal resize updates ViewModel dimensions.
8. Attach to real tmux: control mode notifications render correctly.
9. **[v10 Pass 2]** TermletViewModel: grid preview matches Termlet snapshot.

### AGENTS.md Rules

- `RULE-S25-01`: TUI reads snapshots only, never mutates graph directly.
- `RULE-S25-02`: UI event handlers emit commands/events, not direct state mutations.
- `RULE-S25-03`: **[v10 Pass 2]** Debug panels are non-blocking and optional.

---

## 26. AGENTS.md Rules [v10 updated]

### Design Decisions

This section consolidates all per-section rules into a global reference with numbered IDs and enforcement mechanisms. The rule naming convention follows `RULE-Snn-xx` pattern for traceability. **[v10]** Adds Termlet-specific rules. **[v10 Pass 2]** Adds cross-pollinated rules from GPT and Gemini.

### 26.1 Master Rule Table

| # | Rule ID | Rule | Enforcement |
|---|---|---|---|
| 1 | RULE-S5-01 | `mux-core` is pure: no IO, no `unsafe`, no `tokio`, no `libc` | `#![forbid(unsafe_code)]`, WASM CI check |
| 2 | RULE-S7-01 | Core emits effects; only the runtime executes effects | Code review + typed `Effect` enum |
| 3 | RULE-S7-02 | All state transitions go through `apply_event()` | `grep` for direct graph mutation |
| 4 | RULE-S13-01 | Only the state actor mutates graph state | ArcSwap read path, no `Arc<Mutex<_>>` |
| 5 | RULE-S8-01 | `thiserror` for library errors, `anyhow` only in binaries/tests | CI: `grep anyhow crates/*/src/ \| grep -v test` |
| 6 | RULE-S8-02 | Every `ErrorClass` variant tested | Test coverage check |
| 7 | RULE-S9-01 | Protocol violations kill the connection, never drop-and-continue | `DecodeOutcome::Fatal` enforced |
| 8 | RULE-S10-01 | Config loads only after first client identify burst | Startup sequence test |
| 9 | RULE-S10-02 | Option FALLTHROUGH: pane -> window per `options.c:891-903` | Property test |
| 10 | RULE-S11-01 | `debug_assert!(layout_check(...))` in all layout-mutating functions | CI with debug assertions |
| 11 | RULE-S11-02 | Format strings match tmux via `format-audit` parity corpus | CI: `format-audit` check |
| 12 | RULE-S12-01 | Key bindings match tmux via generated parity tests | CI: key-binding parity tests |
| 13 | RULE-S6-01 | All entity IDs via `slotmap::new_key_type!` | Code review |
| 14 | RULE-S16-01 | Bindings depend only on `mux-api`/`mux-orm`, never `mux-core` directly | `cargo deny check` |
| 15 | RULE-S21-02 | Fake PTY tests use `ScenarioRecorder` JSON format | Test harness validation |
| 16 | RULE-S15-01 | Control notifications are hints, not authoritative | Periodic refresh tests |
| 17 | RULE-S5-02 | WASM CI check for all Layer 0 crates | CI: `cargo check --target wasm32-unknown-unknown` |
| 18 | RULE-S21-01 | Three-layer socket validation in all test harnesses | PathGuard enforced in test fixtures |
| 19 | RULE-S19-01 | Dual OTEL providers: server and client | Architecture test |
| 20 | RULE-S20-01 | File-based locking with `Drop` guard for parallel builds | `LockGuard` pattern enforced |
| 21 | RULE-S20-02 | Atomic build publication via rename | `sibling_tmp_dir` + `std::fs::rename` |
| 22 | RULE-S14-01 | Lock file guard via `Drop` impl, never manual unlock | Code review: no manual `unlock()` calls |
| 23 | RULE-S14-02 | Config load trigger must remain post-identify | Startup sequence test |
| 24 | RULE-S22-01 | Test harnesses use `-f /dev/null` for config isolation | Test fixture enforcement |
| 25 | RULE-S18-01 | Permission hardening: socket dir `0700`, socket `0600` | Test assertions |
| 26 | RULE-S3-03 | **[v10]** `mux-termlet` must not depend on `mux-server` or `mux-api` | `cargo tree` CI check |
| 27 | RULE-S4-03 | **[v10]** `mux-termlet` lives at Layer 2, depends on Layer 0 + Layer 1 only | `cargo deny check` |
| 28 | RULE-S16-03 | **[v10]** Termlet bindings expose: spawn, send_keys, wait_for, snapshot, resize, kill | API surface test |
| 29 | RULE-S22-03 | **[v10]** Termlet fixtures must auto-kill on cleanup | Leak detection test |
| 30 | RULE-S22-04 | **[v10]** Termlet tests must run in both real and fake PTY modes | Parametrized fixture |
| 31 | RULE-S32-01 | **[v10]** Termlet snapshot text must be trailing-whitespace-trimmed | Snapshot format test |
| 32 | RULE-S32-02 | **[v10]** Termlet kill must be idempotent | Double-kill test |
| 33 | RULE-S32-03 | **[v10]** Termlet Drop must send SIGKILL to prevent PTY process leaks | Drop impl audit |
| 34 | RULE-S32-04 | **[v10]** FakePtyBackend tests must not depend on timing | Determinism check |
| 35 | RULE-S32-05 | **[v10]** Termlet bindings must implement context manager / cleanup pattern | Binding API test |
| 36 | RULE-S32-06 | **[v10]** `mux-termlet` must not depend on `mux-server`, `mux-api`, or `mux-orm` | `cargo tree` CI |
| 37 | RULE-S32-07 | **[v10]** Every Termlet API method tested in Rust + at least one binding | Test coverage |
| 38 | RULE-S8-03 | **[v10 P2]** Termlet error codes must be stable string constants | Semver check |
| 39 | RULE-S32-08 | **[v10 P2]** `wait_for` must compile pattern once, not per poll iteration | PatternMatcher audit |
| 40 | RULE-S32-09 | **[v10 P2]** Termlet remains pane-backed; never invent parallel terminal core | Architecture review |

### 26.2 Anti-Patterns (Forbidden)

| Pattern | Why Forbidden | Alternative |
|---|---|---|
| `Arc<Mutex<_>>` on read path | Contention under concurrent reads | `ArcSwap` |
| Direct graph mutation outside `apply_event` | Non-deterministic state transitions | Submit `Event` |
| `unwrap()` in non-test code | Panic risk in production | `?` operator or explicit error handling |
| `unsafe` outside `mux-os` | Purity violation of Layer 0 contract | `mux-os` API |
| Manual `unlock()` call | Resource leak risk on early return | `Drop` guard |
| `std::fs::copy` for build output | Race condition with concurrent readers | Atomic `std::fs::rename` |
| `sleep()` in tests for synchronization | Flaky tests | `wait_for_socket()` or condition variable |
| `anyhow::Error` in library crate public API | Erases error classification | `thiserror` typed errors |
| Inline SQL-style queries | Not applicable to entity graph | `QueryList` + `QueryOp` |
| **[v10]** Termlet without `kill()` call | Leaked PTY processes | Context manager / `Drop` impl |
| **[v10]** `std::thread::sleep` in `wait_for` loop body | Busy-wait wastes CPU | Exponential backoff with configurable interval |
| **[v10 P2]** Re-compiling pattern on each `wait_for` poll | Unnecessary allocation | `PatternMatcher::compile()` once |

### Test Strategy

1. Static rule checks in CI (grep/cargo-deny/compile-fail suites).
2. Rule coverage report links each rule to at least one automated test.
3. Release gate fails when critical rules are unverified.
4. Anti-pattern grep checks run on every PR.

### AGENTS.md Rules

- `RULE-S26-01`: Every architecture section must define enforceable rules.
- `RULE-S26-02`: Rule IDs are immutable once published.

---

## 27. Risks and Mitigations [v10 updated]

### Design Decisions

Risk register covers protocol drift, behavioral divergence, binding API stability, test isolation, observability, dependency management, and **[v10]** Termlet-specific risks. Each risk has an impact level, probability, and specific mitigation.

### 27.1 Risk Table

| # | Risk | Impact | Probability | Mitigation |
|---|---|---|---|---|
| R1 | Protocol edge cases cause incompatibility | High | Medium | Fuzz testing + real tmux fixture captures |
| R2 | VT100 parser divergence from tmux `input.c` | High | Medium | State table generated from tmux source |
| R3 | SlotMap generation overflow | Low | Very Low | 2^32 generations; assert in tests |
| R4 | Format string expansion mismatch | Medium | High | `format-audit` parity corpus |
| R5 | Key binding table divergence | Medium | Medium | Generated parity tests from `key-bindings.c` |
| R6 | PyO3 version incompatibility | Medium | Low | Pin PyO3 version, test in CI |
| R7 | Neon API churn | Medium | Medium | Abstraction layer between Neon and our code |
| R8 | CRDT merge conflicts with stateful operations | High | Low | Property testing with concurrent operations |
| R9 | Performance regression in VT100 parser | Medium | Medium | Criterion benchmarks in CI with threshold |
| R10 | ArcSwap contention under extreme write load | Low | Low | Benchmark; fallback to channel-based notification |
| R11 | Tokio version incompatibility with OTEL SDK | Medium | Low | Pin compatible versions in workspace |
| R12 | SCM_RIGHTS not available on all platforms | Medium | Medium | Feature-gate behind `unix_socket` |
| R13 | FakePty divergence from real PTY behavior | High | Medium | Periodically regenerate fixtures from real tmux |
| R14 | Config parser not covering all tmux options | High | High | `tmux-command-audit` coverage tracking |
| R15 | Layout checksum mismatch with real tmux | Medium | Low | Byte-exact test against tmux captures |
| R16 | Copy mode key handling edge cases | Medium | Medium | Differential testing against real tmux copy mode |
| R17 | ratatui API breaking changes | Medium | Low | Pin version, abstraction layer |
| R18 | Thread-local OTEL headers lost across thread boundaries | Medium | Medium | Capture+push pattern enforced |
| R19 | Multiple tmux versions in test matrix | Low | High | Version matrix in CI |
| R20 | Clipboard security in tests | Low | Medium | `set-clipboard off` in all test harnesses |
| R21 | Lock file stale after crash | Low | Low | `flock` auto-releases on process death |
| R22 | Concurrent cargo test corrupts build cache | Medium | Medium | File-based locking per cache key |
| R23 | Socket path exceeds 108-byte limit | Medium | Low | Path length check before socket creation |
| R24 | Control mode notification ordering | Medium | Medium | Sequence numbers + periodic refresh |
| R25 | CRDT OpLog unbounded growth | Medium | Low | Periodic compaction |
| R26 | Python GIL blocking during Rust operations | Medium | Medium | Release GIL with `py.allow_threads()` |
| R27 | Node.js async operation cancellation | Low | Medium | Neon task cancellation handling |
| R28 | tmux protocol version negotiation | Medium | Low | Version field in identify burst |
| R29 | WASM CI gate false positive from dev dependencies | Low | Medium | Feature-gated dev-deps |
| R30 | OTEL tokio runtime lifecycle blocking process exit | Medium | Medium | Bounded `shutdown_timeout(200ms)` in OtelProvider Drop |
| R31 | **[v10]** Termlet PTY process leak on crash/panic | High | Medium | `Drop` impl that kills PTY, RAII guard pattern |
| R32 | **[v10]** FakePty Termlet diverges from real PTY Termlet | Medium | High | Dual-mode parametrized tests, periodic fixture regen |
| R33 | **[v10]** Termlet `wait_for` busy-loop exhausts CPU | Medium | Low | Exponential backoff with configurable poll interval |
| R34 | **[v10]** Binding Termlet GC timing causes delayed kill | Medium | Medium | Explicit `kill()` in fixture teardown, not relying on GC |
| R35 | **[v10]** Termlet grid reflow on resize loses scroll history | Low | Medium | Document limitation; snapshot before resize if needed |
| R36 | **[v10 P2]** Termlet API semantics diverge across languages | Medium | Medium | Cross-language consistency tests (from GPT) |
| R37 | **[v10 P2]** Compiled PatternMatcher cache grows unbounded | Low | Low | LRU eviction or per-Termlet scope |

### Test Strategy

1. Risk-to-test mapping must be complete and machine-checked.
2. High-impact risks require at least one integration/parity test.
3. Quarterly risk review updates with regression evidence.
4. **[v10]** Termlet-specific risks R31-R37 tested via dedicated Termlet test suite.

### AGENTS.md Rules

- `RULE-S27-01`: New high-risk features must add mitigation tests before merge.
- `RULE-S27-02`: Risk register IDs remain stable for auditability.

---

## 28. Plan Evolution and Changelog

### Design Decisions

This section documents the evolution from v4 through v10 Pass 2, providing traceability for architectural decisions.

### 28.1 Version History

| Version | Date | Changes |
|---|---|---|
| v4 | 2026-02-10 | 6-model synthesis, 2519 lines |
| v5 | 2026-02-10 | 3-model refinement, 1573 lines |
| v6 | 2026-02-10 | 3-pass synthesis, 5239 lines, 31 sections |
| v7 | 2026-02-10 | Cross-model final synthesis |
| Deep-dive | 2026-02-11 | 20 gaps identified across sections 12,16,19,20-22 |
| v8 Pass 1 | 2026-02-11 | Claude + GPT gap closure |
| v8 Pass 2 | 2026-02-11 | GPT refined |
| v8 Pass 3 | 2026-02-11 | Triple-pass final synthesis, 6612 lines |
| v9 Pass 1 (Claude) | 2026-02-11 | 4502 lines, deep code examples |
| v9 Pass 1 (GPT) | 2026-02-11 | 1392 lines, consistent section structure |
| v9 Pass 1 (Gemini) | 2026-02-11 | 638 lines, acceptance criteria tables |
| v9 Pass 2 (Claude) | 2026-02-11 | 4212 lines, triple-model synthesis |
| v9 Pass 2 (GPT) | 2026-02-11 | 1151 lines, verified reference anchors |
| v9 Pass 2 (Gemini) | 2026-02-11 | 784 lines, cross-model verification |
| v9 Pass 3 Final | 2026-02-11 | Definitive synthesis, 4190 lines, 31 sections |
| v10 Pass 1 (Claude) | 2026-02-11 | 4377 lines, 32 sections, Termlets, full Section 32 |
| v10 Pass 1 (GPT) | 2026-02-11 | 1380 lines, 32 sections, Termlets at Section 22, async API |
| v10 Pass 1 (Gemini) | 2026-02-11 | 772 lines, 32 sections, compact Termlet spec |
| **v10 Pass 2** | **2026-02-11** | **This document. Cross-pollinated refinement. 32 sections.** |

### 28.2 v10 Pass 2 Changelog

**Cross-pollination from GPT Pass 1:**
1. `PatternMatcher` compiled pattern for `wait_for` (compile once, match many).
2. `WaitMatch` return type with byte_range and match timestamp.
3. `PatternNotFound` error variant (process exited before match found).
4. Configurable `wait_poll_interval` in TermletConfig.
5. `timeout <= 0` as immediate single-pass check.
6. `TermletViewModel` for TUI debug inspector panel.
7. Stable error code strings (`TERMLET_TIMEOUT`, etc.).
8. Graceful-then-forced kill sequence.
9. `ScenarioStep { in_bytes, out_bytes }` for FakePty fixtures.
10. Flaky-test quarantine requires replay log.
11. Performance regression > 30% blocks release.
12. Cross-language consistency tests for Termlet API.

**Cross-pollination from Gemini Pass 1:**
1. `MuxKernel` alternative naming preserved.
2. `Bound<'py, T>` emphasis for modern PyO3 maintained.

**Weaknesses fixed:**
1. `wait_for` now returns `Result<WaitMatch>` instead of `Result<bool>`.
2. `PatternNotFound` distinguished from `WaitForTimeout`.
3. Compiled pattern matching (not recompiled each poll).
4. Configurable poll interval (not hardcoded).
5. Binding handles use `Mutex<Termlet>` for thread safety.
6. GIL release in PyTermlet `send_keys` and `wait_for`.

### Rust Example

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecVersion {
    V4, V5, V6, V7, V8, V9Pass1, V9Pass2, V9Pass3Final,
    V10Pass1, V10Pass2,
}

pub struct ChangelogItem {
    pub from: SpecVersion,
    pub to: SpecVersion,
    pub summary: &'static str,
}

pub const V10P2_CHANGES: &[ChangelogItem] = &[
    ChangelogItem { from: SpecVersion::V10Pass1, to: SpecVersion::V10Pass2,
        summary: "Cross-pollinated refinement: PatternMatcher, WaitMatch, PatternNotFound, \
            TermletViewModel, Mutex<Termlet> bindings, ScenarioStep, stable error codes." },
];
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

All compatibility claims in this specification are anchored to verified source code. Anchors are organized by codebase and include specific file paths and line numbers where available. All anchors were re-verified during v10 Pass 2.

### 29.1 tmux Source Code References

| File | Line(s) | Topic | Verified v10 P2 |
|---|---|---|---|
| `tmux-protocol.h` | 23 | Protocol version 8 | Yes |
| `client.c` | 77-101 | flock locking (line 89 confirmed) | Yes |
| `server-client.c` | 3472-3475 | Protocol violation kills connection | Yes |
| `server-client.c` | 3725-3734 | Config loaded after identify | Yes |
| `options.c` | 228-241 | Option resolution chain | Yes |
| `options.c` | 891-903 | WindowPane FALLTHROUGH | Yes |
| `options.c` | 1269-1285 | Unset semantics | Yes |
| `layout.c` | 448-462 | Round-robin resize | Yes |
| `layout.c` | 937-950 | Split minimum | Yes |
| `layout-custom.c` | 46-57 | Layout checksum | Yes |
| `layout-custom.c` | 119-153 | Layout validation | Yes |
| `input.c` | State table | VT100 parser | Yes |
| `key-bindings.c` | Default table | Key binding defaults | Yes |
| `control.c` | 450-461 | Control mode pending limit | Yes |
| `control.c` | 620-623 | Extended output format | Yes |
| `control.c` | 758-796 | Control mode startup | Yes |
| `tmux.h` | 100 | PANE_MINIMUM | Yes |

### 29.2 Reference Codebase Verification (Re-verified v10 Pass 2)

| Codebase | File | Claims Verified |
|---|---|---|
| libtmux | `_internal/query_list.py` | 12 operators in `LOOKUP_NAME_MAP` (lines 298-312), `eq` alias, `keygetter` nested traversal (lines 45-113), callable matcher in `filter()` (line 535), `get(default=no_arg)` semantics (lines 550-569), `MultipleObjectsReturned` / `ObjectDoesNotExist` exceptions |
| vibe-tmux | `mux-test-support/src/path_guard.rs` | 3-layer socket validation: `ensure_not_default_socket_name` (line 16), `ensure_socket_within_tempdir` (line 30), `ensure_socket_not_tmux_env` (line 45), `tmux_socket_path` format (line 76) |
| vibe-tmux | `tools/tmux-builder/src/lib.rs` | BLAKE3 `flags_fingerprint` with null separator (line 511), `hex_32` full 64-char hex (line 501), truncated to 12 chars at call site (`&cfg[..12]` at line 543), `compute_cache_key` format (line 521), `sanitize_token`, `LockGuard` with `Drop` (lines 247-256), `lock_cache_key` / `lock_repo_clone`, `sibling_tmp_dir` with pid+nanos (line 409), atomic `std::fs::rename` (line 389), `validate_tmux_binary` (line 549) |
| vibe-tmux | `crates/mux-otel/src/otel.rs` | Dual providers `OTEL_PROVIDER` / `MUX_CLIENT_PROVIDER` as `OnceLock<Mutex<Option<OtelProvider>>>` (lines 58-59), `OtelProvider` struct with logger+tracer_provider+tracer+runtime (lines 65-71), `enter_mux_client_span` lazy init (line 470), composite propagator Baggage+TraceContext (lines 771-777), `HeaderCarrier` Injector/Extractor (lines 998-1028), `force_flush` both providers (line 533), `shutdown` with `.take()` (line 576), `runtime.shutdown_timeout(200ms)` (line 275) |

### Rust Example

```rust
pub struct Anchor {
    pub claim: &'static str,
    pub source: &'static str,
    pub file: &'static str,
    pub lines: Option<&'static str>,
    pub v10p2_verified: bool,
}

pub const ANCHORS: &[Anchor] = &[
    Anchor { claim: "protocol version 8",
        source: "tmux", file: "tmux-protocol.h", lines: Some("23"),
        v10p2_verified: true },
    Anchor { claim: "flock startup lock",
        source: "tmux", file: "client.c", lines: Some("77-101"),
        v10p2_verified: true },
    Anchor { claim: "12 query operators + keygetter",
        source: "libtmux", file: "query_list.py", lines: Some("298-312"),
        v10p2_verified: true },
    Anchor { claim: "3-layer socket validation",
        source: "vibe-tmux", file: "path_guard.rs", lines: Some("16,30,45"),
        v10p2_verified: true },
    Anchor { claim: "BLAKE3 cache key + file locking + 12-char truncation",
        source: "vibe-tmux", file: "tmux-builder/src/lib.rs", lines: Some("511,521,543"),
        v10p2_verified: true },
    Anchor { claim: "dual OTEL providers",
        source: "vibe-tmux", file: "mux-otel/src/otel.rs", lines: Some("58-59"),
        v10p2_verified: true },
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

## 30. Appendix: Canonical Type Quick Reference [v10 updated]

### Design Decisions

This appendix provides a consolidated reference for all canonical type names used across the specification. **[v10]** Termlet types added. **[v10 Pass 2]** `WaitMatch` and `PatternMatcher` types added.

### 30.1 Entity IDs

```rust
slotmap::new_key_type! {
    pub struct SessionId;
    pub struct WindowId;
    pub struct PaneId;
    pub struct ClientId;
    pub struct JobId;
    pub struct BufferId;
}

// [v10] Termlet uses a simple counter, not SlotMap
pub type TermletId = u64;
```

### 30.2 Core Types

| Type | Crate | Purpose |
|---|---|---|
| `ServerGraph` | mux-core | Authoritative mutable state |
| `GraphState` | mux-core | Immutable snapshot with reverse maps |
| `Event` | mux-core | Inbound state change request |
| `Effect` | mux-core | Outbound side-effect request |
| `CoreCtx` | mux-core | Injected time + randomness |
| `ApplyOutcome` | mux-core | Return from apply_event (effects + hints) |
| `Grid` | mux-grid | Terminal cell grid |
| `VtParser` | mux-grid | VT100 state machine |
| `ImsgHdr` | mux-proto | Wire protocol header |
| `ImsgFrame` | mux-proto | Header + payload |
| `ImsgCodec` | mux-proto | Stateful encoder/decoder (tokio-codec compatible) |
| `MsgType` | mux-proto | Protocol message type enum |
| `OptionSet` | mux-types | Key-value option store |
| `OptionValue` | mux-types | Typed option value |
| `OptionScope` | mux-types | Server/Session/Window/Pane |
| `LayoutTree` | mux-core | Arena-based layout |
| `LayoutCell` | mux-core | Single layout node |
| `QueryOp` | mux-query | Filter operator (18 variants) |
| `QueryList<T>` | mux-query | Filterable collection |
| `QuerySpec` | mux-query | Field + operator + value |
| `QueryValue` | mux-query | String / Int / Bool / List / None |
| `HLC` | mux-crdt | Hybrid logical clock |
| `LWWRegister<T>` | mux-crdt | Last-writer-wins register |
| `OrSet<T>` | mux-crdt | Observed-remove set (add-wins) |
| `OpLog` | mux-crdt | Operation log for merge/replay |
| `ControlNotification` | mux-control | Typed control mode notification |
| `PtyBackend` | mux-pty | PTY abstraction trait |
| `FakePtyBackend` | mux-pty-fake | Test PTY implementation |
| `StateHandle` | mux-api | ArcSwap-based read handle |
| `ManagedMux` | mux-api | Server lifecycle manager |
| `TermForgeStack` | mux-api | Composition for in-process embedding |
| `OtelProvider` | mux-otel | OTEL trace + log provider |
| `TraceHeaders` | mux-otel | traceparent + tracestate + baggage |
| `TraceHeadersGuard` | mux-otel | RAII guard for thread-local headers |
| `PathGuard` | mux-test-support | Socket isolation for tests |
| `LockGuard` | tools/tmux-builder | File lock with Drop |
| `ViewModel` | mux-view | Pure view model for TUI |
| **`Termlet`** | **mux-termlet** | **[v10] SDK-first testing pod handle** |
| **`TermletConfig`** | **mux-termlet** | **[v10] Configuration for Termlet spawn** |
| **`TermletSnapshot`** | **mux-termlet** | **[v10] Captured grid state** |
| **`PtyMode`** | **mux-termlet** | **[v10] Real vs Fake PTY backend selection** |
| **`WaitMatch`** | **mux-termlet** | **[v10 P2] Match result with byte range + timestamp** |
| **`PatternMatcher`** | **mux-termlet** | **[v10 P2] Compiled pattern for efficient polling** |
| **`ScenarioStep`** | **mux-pty-fake** | **[v10 P2] Structured FakePty fixture step** |
| **`TermletViewModel`** | **mux-view** | **[v10 P2] Debug inspector panel model** |

### 30.3 Error Types

| Type | Crate | ErrorClass |
|---|---|---|
| `ProtocolError` | mux-proto | ProtocolViolation |
| `CoreError` | mux-core | UserError / Bug |
| `ConfigError` | mux-conf | UserError |
| `LayoutError` | mux-core | Bug |
| `QueryError` | mux-query | UserError |
| `PtyError` | mux-pty | Transient |
| `SocketError` | mux-os | Transient / UserError |
| `LockError` | mux-os | Transient |
| `ControlParseError` | mux-control | UserError |
| **`TermletError`** | **mux-termlet** | **[v10] Transient / UserError** |

### Test Strategy

1. Type API compile checks in integration crates.
2. Semver surface snapshot for public crates.
3. Doc tests for all appendix examples.

### AGENTS.md Rules

- `RULE-S30-01`: Do not rename canonical types without migration notes.
- `RULE-S30-02`: Appendix must match actual exported APIs.

---

## 31. Supplemental Test Matrix [v10 updated]

### Design Decisions

Test matrix extends section-local tests with release gates. Organized by test category, runner, and CI enforcement level. **[v10]** Termlet test categories added. **[v10 Pass 2]** Cross-language consistency tests added.

### 31.1 Test Classification

| Category | Count | Runner | CI? |
|---|---|---|---|
| Unit (pure core) | ~500 | `cargo test` | Yes |
| Property (proptest) | ~50 | `cargo test` | Yes |
| Snapshot (insta) | ~100 | `cargo test` | Yes |
| Protocol fixtures | ~50 | `cargo test` | Yes |
| FakePty scenarios | ~30 | `cargo test` | Yes |
| Integration (in-process) | ~100 | `cargo test` | Yes |
| Integration (subprocess) | ~50 | `cargo test` | Yes, separate job |
| Parity (real tmux) | ~200 | `mux-regress` | Yes, matrix |
| Python binding | ~80 | `pytest` | Yes |
| Node binding | ~40 | `vitest` | Yes |
| Performance | ~10 | `criterion` | Yes (warn only) |
| Fuzz | 3 targets | `cargo fuzz` | Nightly |
| WASM purity | 1 | `cargo check --target wasm32-unknown-unknown` | Yes |
| **[v10] Termlet unit (Rust)** | ~30 | `cargo test` | Yes |
| **[v10] Termlet integration** | ~20 | `cargo test` | Yes |
| **[v10] Termlet Python** | ~15 | `pytest` | Yes |
| **[v10] Termlet Node** | ~10 | `vitest` | Yes |
| **[v10] Termlet performance** | ~3 | `criterion` | Yes (warn only) |
| **[v10 P2] Cross-language consistency** | ~10 | `pytest` + `vitest` + `cargo test` | Yes |

### 31.2 CI Matrix

| Axis | Values |
|---|---|
| OS | Ubuntu 24.04, macOS 14 |
| Rust | stable, nightly |
| tmux version | 3.3a, 3.4, 3.5, 3.6 |
| Python | 3.11, 3.12, 3.13 |
| Node | 20, 22 |

### 31.3 Test Naming Convention

```
test_{layer}_{component}_{behavior}

Examples:
  test_core_session_create_destroy
  test_proto_imsg_roundtrip_all_types
  test_layout_resize_round_robin
  test_query_filter_icontains
  test_binding_py_server_context_manager
  test_otel_dual_provider_init
  test_support_path_guard_three_layers
  test_crdt_hlc_monotonicity
  test_tui_viewmodel_snapshot
  test_termlet_spawn_fake              # [v10]
  test_termlet_spawn_real              # [v10]
  test_termlet_send_keys_echo          # [v10]
  test_termlet_wait_for_timeout        # [v10]
  test_termlet_wait_for_pattern_not_found  # [v10 P2]
  test_termlet_snapshot_text           # [v10]
  test_termlet_resize_reflow           # [v10]
  test_termlet_kill_idempotent         # [v10]
  test_termlet_drop_kills_pty          # [v10]
  test_termlet_pattern_matcher_compile # [v10 P2]
  test_binding_py_termlet_fixture      # [v10]
  test_binding_node_termlet_helper     # [v10]
  test_cross_lang_termlet_snapshot     # [v10 P2]
```

### 31.4 Coverage Requirements

| Crate | Target | Enforcement |
|---|---|---|
| mux-types | 90% | CI gate |
| mux-core | 85% | CI gate |
| mux-grid | 80% | CI gate |
| mux-proto | 90% | CI gate |
| mux-query | 90% | CI gate |
| mux-conf | 80% | CI gate |
| mux-control | 85% | CI gate |
| mux-crdt | 85% | CI gate |
| mux-otel | 70% | Warn only |
| mux-test-support | 80% | CI gate |
| mux-view | 80% | CI gate |
| **mux-termlet** | **85%** | **CI gate** [v10] |
| bindings/python | 80% | CI gate |
| bindings/node | 70% | Warn only |

### 31.5 Release Gate Matrix

| Gate ID | Gate | Test Type | Required? |
|---|---|---|---|
| M-09-01 | Protocol roundtrip | proptest | Yes |
| M-09-02 | Protocol fuzz | cargo-fuzz | Nightly |
| M-11-01 | Layout checksum parity | fixture comparison | Yes |
| M-11-02 | Layout resize proptest | proptest | Yes |
| M-12-01 | Query kwargs parity | unit test | Yes |
| M-12-02 | Query builder parity | unit test | Yes |
| M-15-01 | Control notification parse | unit test | Yes |
| M-16-01 | Python binding API | pytest | Yes |
| M-16-02 | Node binding API | vitest | Yes |
| M-17-01 | CRDT convergence | property test | Yes |
| M-19-01 | OTEL trace continuity | integration test | Yes |
| M-19-02 | OTEL dual provider | architecture test | Yes |
| M-20-01 | Build cache key | unit test | Yes |
| M-21-01 | PathGuard three-layer | unit test | Yes |
| M-21-02 | FakePty scenario replay | unit test | Yes |
| M-23-01 | VT100 fuzz | cargo-fuzz | Nightly |
| M-23-02 | Config fuzz | cargo-fuzz | Nightly |
| M-24-01 | Performance baseline | criterion | Warn only |
| M-25-01 | TUI ViewModel snapshot | insta | Yes |
| M-A2-01 | WASM purity proof | cargo check | Yes |
| **M-32-01** | **[v10] Termlet spawn (fake+real)** | unit test | Yes |
| **M-32-02** | **[v10] Termlet send_keys + snapshot** | unit test | Yes |
| **M-32-03** | **[v10] Termlet resize + reflow** | unit test | Yes |
| **M-32-04** | **[v10] PyTermlet fixture chain** | pytest | Yes |
| **M-32-05** | **[v10] JsTermlet helper** | vitest | Yes |
| **M-32-06** | **[v10 P2] Cross-language snapshot consistency** | multi-runner | Yes |
| **M-32-07** | **[v10 P2] PatternMatcher compile efficiency** | unit test | Yes |

### Rust Example

```rust
pub struct MatrixRow {
    pub id: &'static str,
    pub section: u8,
    pub test_name: &'static str,
    pub required: bool,
}

pub const MATRIX: &[MatrixRow] = &[
    MatrixRow { id: "M-09-01", section: 9, test_name: "fuzz_decode_frame", required: true },
    MatrixRow { id: "M-12-01", section: 12, test_name: "query_kwargs_parity", required: true },
    MatrixRow { id: "M-21-01", section: 21, test_name: "path_guard_three_layer", required: true },
    MatrixRow { id: "M-19-01", section: 19, test_name: "otel_trace_continuity", required: true },
    MatrixRow { id: "M-A2-01", section: 5, test_name: "wasm_purity_proof", required: true },
    // [v10] Termlet gates
    MatrixRow { id: "M-32-01", section: 32, test_name: "termlet_spawn_dual_mode", required: true },
    MatrixRow { id: "M-32-02", section: 32, test_name: "termlet_send_snapshot", required: true },
    MatrixRow { id: "M-32-03", section: 32, test_name: "termlet_resize_reflow", required: true },
    MatrixRow { id: "M-32-04", section: 32, test_name: "py_termlet_fixture", required: true },
    MatrixRow { id: "M-32-05", section: 32, test_name: "js_termlet_helper", required: true },
    // [v10 Pass 2] Cross-pollinated gates
    MatrixRow { id: "M-32-06", section: 32, test_name: "cross_lang_snapshot", required: true },
    MatrixRow { id: "M-32-07", section: 32, test_name: "pattern_matcher_compile", required: true },
];
```

### Test Strategy

1. CI requires 100% pass of mandatory matrix rows.
2. Nightly includes extended parity + fuzz durations.
3. Release requires zero unresolved mandatory rows.
4. **[v10]** Termlet gates M-32-01 through M-32-07 are mandatory for release.

### AGENTS.md Rules

- `RULE-S31-01`: New architecture features must add at least one matrix row.
- `RULE-S31-02`: Release tagging is blocked until mandatory rows are green.

---

## 32. Termlets [v10 new, Pass 2 refined]

### Design Decisions

Termlets are the killer feature differentiating TermForge from other terminal multiplexers and testing tools. They are **SDK-first testing pods** that wrap tmux panes into simplified, embeddable, language-binding-native handles.

**Core philosophy:** A Termlet is to a tmux pane what a Docker container is to a VM -- same capability, simpler interface, faster lifecycle, purpose-built for programmatic use.

**Key design decisions:**

1. **Testing pods**: A Termlet wraps a Pane + Grid + PtyBackend into a single ergonomic handle. Unlike a full multiplexer session, a Termlet does not require a `ServerGraph`, a `StateActor`, a socket, or a running server. It is self-contained.

2. **SDK-first**: The API is designed for programmatic use, not interactive terminal use. Every operation returns a `Result`. Every state is queryable. Every output is capturable.

3. **Visual area captured**: Each Termlet owns a `Grid` that accumulates VT100 output from the underlying PTY. The grid can be snapshotted at any time, producing a text representation suitable for assertion or insta snapshot testing.

4. **Resizable**: Programmatic resize via `resize(cols, rows)` triggers PTY `SIGWINCH` and grid reflow.

5. **Interactive**: A Termlet holds a shell session (or any command). It accepts input via `send_keys()` and produces output that flows through the VT100 parser into the grid.

6. **Available everywhere**: Rust core (`mux_termlet::Termlet`), Python (`termforge.Termlet` via PyO3), Node.js (`termforge.useTermlet()` via Neon). Same semantics, language-native ergonomics.

7. **Backward compatible**: A Termlet IS a pane under the hood. The `Pane` struct, the `Grid` struct, the `VtParser` -- all reused from `mux-core` and `mux-grid`. The Termlet just provides a simpler creation and interaction API.

8. **Process management**: Spawn shells, run commands, send signals, detect exit. The Termlet owns the child process lifecycle and cleans up via `Drop`. **[Pass 2]** Kill uses graceful-then-forced sequence: SIGTERM first, SIGKILL on `Drop`.

9. **The ultimate subprocess runner**: Where `std::process::Command` gives you stdout/stderr as byte streams, a Termlet gives you a full terminal emulation. You can test interactive programs, TUI apps, shell scripts with prompts, and anything that uses terminal escape sequences.

10. **Lite**: Minimal overhead. Creating a Termlet with FakePtyBackend takes < 500us. Creating one with a real PTY takes < 50ms. Destroying one is instant (kill signal + drop).

11. **Snapshot-testable**: `insta::assert_snapshot!(termlet.snapshot().to_text())` in Rust. `assert termlet.snapshot() == expected` in Python. `expect(termlet.snapshot()).toMatchSnapshot()` in vitest.

12. **Dual-mode**: `PtyMode::Real` for integration tests against real shells. `PtyMode::Fake` for deterministic unit tests with injected output.

13. **[Pass 2] Compiled pattern matching**: `wait_for` compiles the pattern once via `PatternMatcher` and reuses the compiled form across poll iterations, avoiding per-iteration allocation. Supports plain string and regex modes.

14. **[Pass 2] Structured wait result**: `wait_for` returns `WaitMatch` carrying the byte range of the match and the timestamp, enabling precise assertions.

15. **[Pass 2] Process exit detection**: `PatternNotFound` error variant signals that the child process exited before the pattern appeared, distinct from `WaitForTimeout` which means the deadline elapsed.

**Architectural position:** `mux-termlet` sits at Layer 2 (FACADE). It depends on:
- `mux-core` (Layer 0) for `Pane`, `PaneSize`
- `mux-grid` (Layer 0) for `Grid`, `VtParser`
- `mux-pty` (Layer 1) for `PtyBackend` trait and real implementation
- `mux-pty-fake` (Layer 0) for `FakePtyBackend`
- `mux-types` (Layer 0) for shared types

It does NOT depend on:
- `mux-server` (no runtime needed)
- `mux-api` (no ManagedMux needed)
- `mux-orm` (no query layer needed)
- `mux-otel` (optional, feature-gated)

### 32.1 Termlet Architecture

```
                    +------------------+
                    |   User Code      |
                    | (Rust / Py / JS) |
                    +--------+---------+
                             |
                    spawn / send_keys / snapshot / resize / kill
                             |
                    +--------v---------+
                    |     Termlet      |  <-- mux-termlet crate
                    |  (single handle) |
                    +--------+---------+
                             |
            +----------------+----------------+
            |                |                |
    +-------v------+  +-----v------+  +------v------+
    |  VtParser    |  |    Grid    |  | PtyBackend  |
    | (mux-grid)   |  | (mux-grid) |  | (mux-pty)   |
    +--------------+  +------------+  +------+------+
                                             |
                            +----------------+----------------+
                            |                                 |
                    +-------v-------+               +--------v--------+
                    | RealPtyBackend|               | FakePtyBackend  |
                    |  (mux-pty)    |               | (mux-pty-fake)  |
                    +---------------+               +-----------------+
```

### 32.2 Core API

```rust
// crates/mux-termlet/src/lib.rs

/// Configuration for spawning a Termlet.
#[derive(Debug, Clone)]
pub struct TermletConfig {
    /// Terminal width in columns.
    pub cols: u16,
    /// Terminal height in rows.
    pub rows: u16,
    /// Environment variables to set in the spawned process.
    pub env: Vec<(String, String)>,
    /// Working directory for the spawned process.
    pub cwd: Option<String>,
    /// PTY backend mode: real or fake.
    pub backend: PtyMode,
    /// [v10 Pass 2] Poll interval for wait_for (default 25ms, max 100ms).
    /// Exponential backoff starts at this value and doubles up to max.
    pub wait_poll_interval: std::time::Duration,
}

impl Default for TermletConfig {
    fn default() -> Self {
        Self {
            cols: 80,
            rows: 24,
            env: vec![("TERM".to_string(), "xterm-256color".to_string())],
            cwd: None,
            backend: PtyMode::Real,
            wait_poll_interval: std::time::Duration::from_millis(25),
        }
    }
}

/// Select between real and fake PTY backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtyMode {
    /// Real PTY: spawns an actual process with a pseudo-terminal.
    /// Use for integration tests against real shells/programs.
    Real,
    /// Fake PTY: deterministic, no real process.
    /// Use for unit tests with injected output.
    Fake,
}

/// [v10 Pass 2] Compiled pattern for efficient wait_for polling.
/// Compiles the pattern once, then matches on each poll iteration
/// without re-allocating.
pub enum PatternMatcher {
    /// Plain substring match (case-sensitive).
    Plain(String),
    /// Compiled regular expression.
    Regex(regex::Regex),
}

impl PatternMatcher {
    /// Compile a pattern. Prefix with "re:" for regex mode.
    pub fn compile(pattern: &str) -> Result<Self, TermletError> {
        if let Some(re_pat) = pattern.strip_prefix("re:") {
            let re = regex::Regex::new(re_pat)
                .map_err(|e| TermletError::SpawnFailed {
                    reason: format!("invalid regex: {e}"),
                })?;
            Ok(PatternMatcher::Regex(re))
        } else {
            Ok(PatternMatcher::Plain(pattern.to_string()))
        }
    }

    /// Find a match in the given text.
    /// Returns the byte range of the first match, or None.
    pub fn find(&self, text: &str) -> Option<std::ops::Range<usize>> {
        match self {
            PatternMatcher::Plain(s) => {
                text.find(s).map(|start| start..start + s.len())
            }
            PatternMatcher::Regex(re) => {
                re.find(text).map(|m| m.start()..m.end())
            }
        }
    }
}

/// [v10 Pass 2] Result of a successful wait_for match.
#[derive(Debug, Clone)]
pub struct WaitMatch {
    /// Byte range of the match within the snapshot text.
    pub byte_range: std::ops::Range<usize>,
    /// Timestamp when the match was found.
    pub matched_at: std::time::Instant,
}

/// A Termlet is an SDK-first testing pod: a simplified, self-contained
/// terminal emulation handle for programmatic use.
///
/// It wraps a Pane + Grid + VtParser + PtyBackend into a single ergonomic handle.
/// Unlike a full multiplexer session, a Termlet does not require a server,
/// a state actor, or a socket. It is self-contained.
pub struct Termlet {
    /// Terminal grid (accumulated VT100 output).
    grid: Grid,
    /// VT100 state machine parser.
    parser: VtParser,
    /// PTY backend (real or fake).
    backend: Box<dyn PtyBackend>,
    /// Pane ID used with the backend.
    pane_id: PaneId,
    /// Current terminal size.
    size: PaneSize,
    /// Whether the underlying process has exited.
    exited: bool,
    /// Unique Termlet identifier (monotonic counter).
    id: TermletId,
    /// [v10 Pass 2] Poll interval for wait_for.
    wait_poll_interval: std::time::Duration,
}

/// Counter for Termlet IDs.
static NEXT_TERMLET_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

impl Termlet {
    /// Spawn a new Termlet running the given command.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let mut t = Termlet::spawn("bash", TermletConfig::default())?;
    /// t.send_keys("echo hello\n")?;
    /// let m = t.wait_for("hello", Duration::from_secs(5))?;
    /// println!("matched at byte range {:?}", m.byte_range);
    /// let snap = t.snapshot();
    /// insta::assert_snapshot!(snap.to_text());
    /// t.kill()?;
    /// ```
    pub fn spawn(command: &str, config: TermletConfig) -> Result<Self, TermletError> {
        let id = NEXT_TERMLET_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let size = PaneSize { sx: config.cols, sy: config.rows };
        let grid = Grid::new(config.cols, config.rows);
        let parser = VtParser::new();
        let wait_poll_interval = config.wait_poll_interval;

        // Create a synthetic PaneId for this Termlet.
        let pane_id = PaneId::from(slotmap::KeyData::from_ffi(id));

        let mut backend: Box<dyn PtyBackend> = match config.backend {
            PtyMode::Real => {
                Box::new(RealPtyBackend::new()?)
            }
            PtyMode::Fake => {
                Box::new(FakePtyBackend::new())
            }
        };

        let cmd_parts = vec![command.to_string()];
        backend.spawn(
            pane_id,
            &cmd_parts,
            config.cwd.as_deref(),
            size,
        ).map_err(|e| TermletError::SpawnFailed {
            reason: e.to_string(),
        })?;

        Ok(Self {
            grid,
            parser,
            backend,
            pane_id,
            size,
            exited: false,
            id,
            wait_poll_interval,
        })
    }

    /// Send keystrokes to the Termlet.
    ///
    /// The keys are written directly to the PTY. Use `\n` for Enter,
    /// `\x03` for Ctrl-C, etc.
    pub fn send_keys(&mut self, keys: &str) -> Result<(), TermletError> {
        if self.exited {
            return Err(TermletError::AlreadyKilled);
        }
        self.backend.write(self.pane_id, keys.as_bytes())?;
        // Drain any pending output from the PTY into the grid.
        self.drain_output();
        Ok(())
    }

    /// Wait for a pattern to appear in the grid text.
    ///
    /// [v10 Pass 2] Compiles the pattern once via PatternMatcher, then polls
    /// with exponential backoff. Returns WaitMatch on success.
    ///
    /// Supports two pattern modes:
    /// - Plain string: `"hello"` (default, case-sensitive substring match)
    /// - Regex: `"re:hello\\s+world"` (prefix with "re:" for regex mode)
    ///
    /// Special timeout values:
    /// - `timeout <= 0`: immediate single-pass check (no polling)
    /// - Normal: polls until match or deadline
    ///
    /// Error variants:
    /// - `WaitForTimeout`: deadline elapsed without match
    /// - `PatternNotFound`: process exited before match was found
    pub fn wait_for(
        &mut self,
        pattern: &str,
        timeout: std::time::Duration,
    ) -> Result<WaitMatch, TermletError> {
        let compiled = PatternMatcher::compile(pattern)?;
        let start = std::time::Instant::now();
        let mut delay = self.wait_poll_interval;
        let max_delay = std::time::Duration::from_millis(100);

        // Immediate check mode: timeout of zero
        if timeout.is_zero() {
            self.drain_output();
            let text = self.grid.to_text();
            if let Some(range) = compiled.find(&text) {
                return Ok(WaitMatch {
                    byte_range: range,
                    matched_at: std::time::Instant::now(),
                });
            }
            return Err(TermletError::WaitForTimeout {
                pattern: pattern.to_string(),
                timeout_ms: 0,
            });
        }

        loop {
            self.drain_output();
            let text = self.grid.to_text();
            if let Some(range) = compiled.find(&text) {
                return Ok(WaitMatch {
                    byte_range: range,
                    matched_at: std::time::Instant::now(),
                });
            }

            // [v10 Pass 2] Check if process exited (distinct from timeout)
            if self.exited {
                return Err(TermletError::PatternNotFound {
                    pattern: pattern.to_string(),
                });
            }

            if start.elapsed() > timeout {
                return Err(TermletError::WaitForTimeout {
                    pattern: pattern.to_string(),
                    timeout_ms: timeout.as_millis() as u64,
                });
            }

            let remaining = timeout.saturating_sub(start.elapsed());
            let sleep_for = delay.min(max_delay).min(remaining);
            std::thread::sleep(sleep_for);
            delay = (delay * 2).min(max_delay);
        }
    }

    /// Capture a snapshot of the current grid state.
    ///
    /// The snapshot is a frozen copy of the grid that can be used for
    /// assertions, insta snapshots, or visual inspection.
    pub fn snapshot(&mut self) -> TermletSnapshot {
        // Drain any pending output first.
        self.drain_output();
        TermletSnapshot {
            grid: self.grid.clone(),
            cols: self.size.sx,
            rows: self.size.sy,
        }
    }

    /// Resize the Termlet.
    ///
    /// This resizes the grid, triggers a SIGWINCH on the PTY, and
    /// reflows the grid content.
    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<(), TermletError> {
        if self.exited {
            return Err(TermletError::AlreadyKilled);
        }
        let new_size = PaneSize { sx: cols, sy: rows };
        self.backend.resize(self.pane_id, new_size)?;
        self.grid.resize(cols, rows);
        self.size = new_size;
        Ok(())
    }

    /// Kill the underlying process.
    ///
    /// [v10 Pass 2] Uses graceful-then-forced sequence:
    /// 1. SIGTERM first (allows clean shutdown)
    /// 2. Marks as exited
    ///
    /// Idempotent: calling kill() on an already-killed Termlet is a no-op.
    pub fn kill(&mut self) -> Result<(), TermletError> {
        if self.exited {
            return Ok(()); // Idempotent
        }
        self.backend.kill(self.pane_id, libc::SIGTERM)?;
        self.exited = true;
        Ok(())
    }

    /// Check if the underlying process is still alive.
    pub fn is_alive(&self) -> bool {
        !self.exited
    }

    /// Get the current terminal size.
    pub fn size(&self) -> PaneSize {
        self.size
    }

    /// Get the Termlet's unique ID.
    pub fn id(&self) -> TermletId {
        self.id
    }

    /// Get the Termlet's pane ID (for debug/inspector use).
    pub fn pane_id(&self) -> PaneId {
        self.pane_id
    }

    /// Drain pending PTY output into the grid via the VT100 parser.
    fn drain_output(&mut self) {
        let events = self.backend.read_events();
        for event in events {
            match event {
                PtyEvent::Output { data, .. } => {
                    self.parser.parse(&data, &mut self.grid);
                }
                PtyEvent::Exited { .. } => {
                    self.exited = true;
                }
                _ => {}
            }
        }
    }

    /// Get mutable access to the underlying FakePtyBackend for test injection.
    ///
    /// Returns `None` if the Termlet is using a real PTY backend.
    /// This is intentionally only available in test mode.
    #[cfg(any(test, feature = "test-support"))]
    pub fn fake_backend_mut(&mut self) -> Option<&mut FakePtyBackend> {
        self.backend.as_any_mut().downcast_mut::<FakePtyBackend>()
    }
}

impl Drop for Termlet {
    fn drop(&mut self) {
        if !self.exited {
            // Best-effort SIGKILL on drop to prevent PTY process leaks.
            // Using SIGKILL (not SIGTERM) because drop is the last resort.
            let _ = self.backend.kill(self.pane_id, libc::SIGKILL);
        }
    }
}
```

### 32.3 TermletSnapshot

```rust
// crates/mux-termlet/src/snapshot.rs

/// A frozen snapshot of a Termlet's grid state.
///
/// Snapshots are cheap to create (grid clone) and are the primary
/// mechanism for asserting terminal output in tests.
#[derive(Debug, Clone)]
pub struct TermletSnapshot {
    grid: Grid,
    cols: u16,
    rows: u16,
}

impl TermletSnapshot {
    /// Convert the snapshot to plain text, trimming trailing whitespace
    /// from each line and trailing empty lines from the bottom.
    ///
    /// This is the primary output format for snapshot testing.
    pub fn to_text(&self) -> String {
        let mut lines: Vec<String> = (0..self.rows)
            .map(|row| self.grid.row_text(row as usize).trim_end().to_string())
            .collect();
        // Trim trailing empty lines
        while lines.last().is_some_and(|l| l.is_empty()) {
            lines.pop();
        }
        lines.join("\n")
    }

    /// Convert the snapshot to styled text with ANSI escape sequences.
    ///
    /// Useful for visual debugging but not for snapshot testing
    /// (ANSI codes would make snapshots fragile).
    pub fn to_styled(&self) -> String {
        let mut output = String::new();
        for row in 0..self.rows {
            for col in 0..self.cols {
                let cell = self.grid.cell(row as usize, col as usize);
                write_ansi_cell(&mut output, cell);
            }
            output.push('\n');
        }
        output
    }

    /// Get the raw cell grid for advanced assertions.
    ///
    /// Returns a 2D vector of cells with character, foreground color,
    /// background color, and attributes.
    pub fn to_cells(&self) -> Vec<Vec<Cell>> {
        (0..self.rows)
            .map(|row| {
                (0..self.cols)
                    .map(|col| self.grid.cell(row as usize, col as usize).clone())
                    .collect()
            })
            .collect()
    }

    /// Get the grid dimensions.
    pub fn size(&self) -> (u16, u16) {
        (self.cols, self.rows)
    }

    /// Check if a pattern appears anywhere in the text representation.
    pub fn contains(&self, pattern: &str) -> bool {
        self.to_text().contains(pattern)
    }

    /// Get a specific row as text.
    pub fn row(&self, idx: usize) -> String {
        self.grid.row_text(idx).trim_end().to_string()
    }
}

impl std::fmt::Display for TermletSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_text())
    }
}
```

### 32.4 Usage Examples

#### Rust: Basic Echo Test

```rust
#[test]
fn test_termlet_echo() {
    let mut t = Termlet::spawn("bash", TermletConfig {
        cols: 80, rows: 24,
        env: vec![("TERM".into(), "xterm-256color".into())],
        cwd: Some("/tmp".into()),
        backend: PtyMode::Real,
        ..Default::default()
    }).unwrap();

    t.send_keys("echo hello world\n").unwrap();
    let m = t.wait_for("hello world", Duration::from_secs(5)).unwrap();
    assert!(!m.byte_range.is_empty());

    let snap = t.snapshot();
    assert!(snap.contains("hello world"));
    insta::assert_snapshot!(snap.to_text());

    t.kill().unwrap();
}
```

#### Rust: Fake PTY for Deterministic Tests

```rust
#[test]
fn test_termlet_fake_deterministic() {
    let mut t = Termlet::spawn("fake-shell", TermletConfig {
        cols: 40, rows: 10,
        backend: PtyMode::Fake,
        ..Default::default()
    }).unwrap();

    // Inject deterministic output via FakePtyBackend
    let fake = t.fake_backend_mut().unwrap();
    fake.inject_output(t.pane_id(), b"$ echo test\r\ntest\r\n$ ".to_vec());

    let snap = t.snapshot();
    insta::assert_snapshot!("fake-echo", snap.to_text());
}
```

#### Rust: wait_for with Regex Pattern

```rust
#[test]
fn test_termlet_wait_for_regex() {
    let mut t = Termlet::spawn("bash", TermletConfig::default()).unwrap();

    t.send_keys("echo 'build 42 passed'\n").unwrap();
    let m = t.wait_for("re:build \\d+ passed", Duration::from_secs(5)).unwrap();
    assert!(m.byte_range.len() > 0);

    t.kill().unwrap();
}
```

#### Rust: PatternNotFound on Process Exit

```rust
#[test]
fn test_termlet_pattern_not_found() {
    let mut t = Termlet::spawn("echo", TermletConfig {
        backend: PtyMode::Real,
        ..Default::default()
    }).unwrap();

    // "echo" exits immediately -- pattern will never appear
    std::thread::sleep(Duration::from_millis(200));
    let result = t.wait_for("this_will_never_appear", Duration::from_secs(5));
    assert!(matches!(result, Err(TermletError::PatternNotFound { .. })));
}
```

#### Rust: Resize and Reflow

```rust
#[test]
fn test_termlet_resize() {
    let mut t = Termlet::spawn("bash", TermletConfig::default()).unwrap();

    t.send_keys("echo hello\n").unwrap();
    t.wait_for("hello", Duration::from_secs(3)).unwrap();

    // Resize to 120x40
    t.resize(120, 40).unwrap();
    assert_eq!(t.size(), PaneSize { sx: 120, sy: 40 });

    // Content is preserved after resize
    let snap = t.snapshot();
    assert!(snap.contains("hello"));

    t.kill().unwrap();
}
```

#### Rust: insta Snapshot Testing with FakePty

```rust
#[test]
fn test_termlet_insta_snapshot() {
    let mut t = Termlet::spawn("bash", TermletConfig {
        cols: 80, rows: 24,
        backend: PtyMode::Fake,
        ..Default::default()
    }).unwrap();

    let fake = t.fake_backend_mut().unwrap();
    fake.inject_output(t.pane_id(),
        b"\x1b[1;1H\x1b[2J$ ls\r\nfile1.txt  file2.txt  dir/\r\n$ ".to_vec());

    let snap = t.snapshot();
    insta::assert_snapshot!(snap.to_text());
}
```

#### Rust: Colored Output Cell-Level Assertion

```rust
#[test]
fn test_fake_pty_colored_output() {
    let mut t = Termlet::spawn("fake", TermletConfig {
        cols: 80, rows: 24,
        backend: PtyMode::Fake,
        ..Default::default()
    }).unwrap();

    let fake = t.fake_backend_mut().unwrap();
    // Inject colored output: red "ERROR" followed by normal text
    fake.inject_output(t.pane_id(),
        b"\x1b[31mERROR\x1b[0m: something went wrong\r\n".to_vec());

    let snap = t.snapshot();
    // Text extraction strips colors
    assert!(snap.to_text().contains("ERROR: something went wrong"));

    // Cell-level assertion preserves color info
    let cells = snap.to_cells();
    for i in 0..5 {
        assert_eq!(cells[0][i].fg, Color::Red);
    }
}
```

#### Python: pytest Fixtures

```python
# bindings/python/tests/conftest.py
import pytest
from termforge import Termlet

@pytest.fixture
def termlet():
    """Spawn a real Termlet with automatic cleanup."""
    with Termlet("bash", cols=80, rows=24) as t:
        yield t

@pytest.fixture
def fake_termlet():
    """Spawn a fake Termlet for deterministic testing."""
    with Termlet("bash", cols=80, rows=24, fake=True) as t:
        yield t

@pytest.fixture(params=["real", "fake"])
def any_termlet(request):
    """Parametrized fixture: same test runs with real and fake PTY."""
    fake = request.param == "fake"
    with Termlet("bash", cols=80, rows=24, fake=fake) as t:
        yield t
```

```python
# bindings/python/tests/test_termlet.py
def test_echo(termlet):
    termlet.send_keys("echo hello\n")
    termlet.wait_for("hello")
    snapshot = termlet.snapshot()
    assert "hello" in snapshot

def test_resize(termlet):
    termlet.resize(120, 40)
    assert termlet.size == (120, 40)

def test_wait_for_timeout():
    from termforge import Termlet
    with Termlet("bash") as t:
        with pytest.raises(TimeoutError):
            t.wait_for("this_will_never_appear", timeout_ms=100)

def test_context_manager():
    from termforge import Termlet
    with Termlet("bash") as t:
        t.send_keys("echo managed\n")
        t.wait_for("managed")
    # t is killed here -- no leaked processes
```

#### Node.js: vitest Helper

```javascript
// bindings/node/tests/termlet.test.js
import { describe, it, expect } from 'vitest';
import { useTermlet } from 'termforge';

describe('Termlet', () => {
    it('captures echo output', () => {
        const t = useTermlet('bash', { cols: 80, rows: 24 });
        try {
            t.sendKeys('echo hello\n');
            t.waitFor('hello', 5000);
            expect(t.snapshot()).toContain('hello');
        } finally {
            t.kill();
        }
    });

    it('supports fake mode', () => {
        const t = useTermlet('bash', { cols: 80, rows: 24, fake: true });
        try {
            t.sendKeys('echo test\n');
            expect(typeof t.snapshot()).toBe('string');
        } finally {
            t.kill();
        }
    });

    it('resizes correctly', () => {
        const t = useTermlet('bash', { cols: 80, rows: 24 });
        try {
            expect(t.size()).toEqual({ cols: 80, rows: 24 });
            t.resize(120, 40);
            expect(t.size()).toEqual({ cols: 120, rows: 40 });
        } finally {
            t.kill();
        }
    });
});
```

### 32.5 FakePty Integration

The FakePtyBackend (from `mux-pty-fake`) enables fully deterministic Termlet tests:

```rust
// Deterministic test pattern:
// 1. Spawn Termlet with PtyMode::Fake
// 2. Access FakePtyBackend via fake_backend_mut()
// 3. Inject precise byte sequences (including ANSI escape codes)
// 4. Snapshot the grid
// 5. Assert with insta

// [v10 Pass 2] Structured scenario replay:
fn replay_scenario(t: &mut Termlet, scenario: &Scenario) {
    let fake = t.fake_backend_mut().expect("must be in fake mode");
    for step in &scenario.steps {
        // Write input to the "shell"
        t.send_keys(&String::from_utf8_lossy(&step.in_bytes)).unwrap();
        // Inject expected output
        fake.inject_output(t.pane_id(), step.out_bytes.clone());
    }
}

#[test]
fn test_scenario_replay() {
    let scenario = Scenario {
        initial_size: PaneSize { sx: 80, sy: 24 },
        steps: vec![
            ScenarioStep {
                in_bytes: b"echo hello\n".to_vec(),
                out_bytes: b"$ echo hello\r\nhello\r\n$ ".to_vec(),
            },
        ],
    };

    let mut t = Termlet::spawn("fake-shell", TermletConfig {
        cols: scenario.initial_size.sx,
        rows: scenario.initial_size.sy,
        backend: PtyMode::Fake,
        ..Default::default()
    }).unwrap();

    replay_scenario(&mut t, &scenario);

    let snap = t.snapshot();
    insta::assert_snapshot!(snap.to_text());
}
```

### 32.6 OTEL Integration (Feature-Gated)

```rust
// When the `otel` feature is enabled, Termlet operations emit spans:
#[cfg(feature = "otel")]
pub fn spawn_instrumented(command: &str, config: TermletConfig) -> Result<Termlet, TermletError> {
    let _span = tracing::info_span!(
        "termforge.termlet.spawn",
        command = command,
        cols = config.cols,
        rows = config.rows,
        mode = ?config.backend,
    ).entered();
    Termlet::spawn(command, config)
}

#[cfg(feature = "otel")]
pub fn wait_for_instrumented(
    termlet: &mut Termlet,
    pattern: &str,
    timeout: std::time::Duration,
) -> Result<WaitMatch, TermletError> {
    let _span = tracing::info_span!(
        "termforge.termlet.interact",
        operation = "wait_for",
        pattern = pattern,
        timeout_ms = timeout.as_millis() as u64,
    ).entered();
    termlet.wait_for(pattern, timeout)
}
```

### 32.7 Comparison with Alternatives

| Feature | Termlet | `std::process::Command` | `expectrl` | tmux pane |
|---|---|---|---|---|
| Terminal emulation | Full VT100 | None | Partial | Full |
| Grid capture | Yes (snapshot) | No | No | Via capture-pane |
| Resize support | Yes (SIGWINCH) | N/A | Limited | Yes |
| Color/attribute info | Yes (per-cell) | No | No | Yes |
| Deterministic mode | Yes (FakePty) | No | No | No |
| No server needed | Yes | Yes | Yes | No (needs tmux) |
| Language bindings | Rust/Py/Node | Rust only | Rust only | Via libtmux |
| insta snapshots | Native | No | No | Manual |
| Lifecycle (spawn) | < 500us fake | ~5ms | ~10ms | ~50ms |
| API surface | 6 methods | 3 methods | ~10 methods | ~20 commands |
| Regex wait_for | Yes [P2] | No | Yes | No |
| Match result | WaitMatch [P2] | No | Match obj | No |
| Process exit detect | PatternNotFound [P2] | Exit status | EOF | Exit hook |

### 32.8 Error Code Reference

| Error Variant | Code String | Python Exception | Node Error |
|---|---|---|---|
| `SpawnFailed` | `TERMLET_SPAWN_ERROR` | `RuntimeError` | `Error` |
| `AlreadyKilled` | `TERMLET_ALREADY_KILLED` | `RuntimeError` | `Error` |
| `WaitForTimeout` | `TERMLET_TIMEOUT` | `TimeoutError` | `Error` (code: TIMEOUT) |
| `PatternNotFound` | `TERMLET_PATTERN_NOT_FOUND` | `RuntimeError` | `Error` (code: NOT_FOUND) |
| `ResizeFailed` | `TERMLET_RESIZE_ERROR` | `RuntimeError` | `Error` |
| `Pty(...)` | `TERMLET_PTY_ERROR` | `RuntimeError` | `Error` |

### Test Strategy

1. **Spawn lifecycle**: Termlet::spawn succeeds with both Real and Fake backends.
2. **Send keys echo**: send "echo hello\n", verify "hello" appears in snapshot.
3. **Wait for timeout**: wait_for("nonexistent", 100ms) returns WaitForTimeout error.
4. **Wait for success**: wait_for("hello", 5s) returns Ok(WaitMatch) after send_keys.
5. **[Pass 2] Wait for regex**: wait_for("re:build \\d+", 5s) matches regex pattern.
6. **[Pass 2] Wait for immediate**: wait_for("pattern", Duration::ZERO) does single-pass check.
7. **[Pass 2] Pattern not found**: process exits -> wait_for returns PatternNotFound.
8. **[Pass 2] WaitMatch byte range**: returned range correctly identifies match position.
9. **Snapshot text trimming**: trailing whitespace and empty lines stripped from to_text().
10. **Snapshot contains**: snapshot.contains("pattern") matches substring correctly.
11. **Snapshot row access**: snapshot.row(0) returns first row text.
12. **Resize dimensions**: after resize(120, 40), size() returns (120, 40).
13. **Resize SIGWINCH**: real PTY receives SIGWINCH after resize.
14. **Resize grid reflow**: content preserved after resize.
15. **Kill idempotent**: calling kill() twice does not error.
16. **Kill already exited**: killing after process exit is a no-op.
17. **Drop kills PTY**: dropping a Termlet without kill() sends SIGKILL.
18. **Fake inject output**: FakePtyBackend.inject_output -> drain_output -> grid updated.
19. **Fake deterministic**: same inject sequence always produces identical snapshot.
20. **Fake colored output**: injected ANSI color codes parsed correctly, cells have color info.
21. **ID uniqueness**: each Termlet gets a unique monotonic ID.
22. **Config default**: TermletConfig::default() produces 80x24 with xterm-256color.
23. **AlreadyKilled error**: send_keys/resize after kill returns AlreadyKilled.
24. **insta snapshot**: Termlet snapshot integrates with insta::assert_snapshot!.
25. **[Pass 2] PatternMatcher compile**: plain and regex patterns compile correctly.
26. **[Pass 2] PatternMatcher find**: plain and regex patterns find matches in text.
27. **[Pass 2] Configurable poll interval**: custom wait_poll_interval is respected.
28. **[Pass 2] Scenario replay**: ScenarioStep sequence replays deterministically.
29. **PyTermlet spawn**: Python `Termlet("bash")` creates and returns handle.
30. **PyTermlet context manager**: `with Termlet("bash") as t:` auto-kills.
31. **PyTermlet snapshot**: `termlet.snapshot()` returns string.
32. **PyTermlet fake mode**: `Termlet("bash", fake=True)` uses FakePtyBackend.
33. **PyTermlet wait_for timeout**: raises `TimeoutError`.
34. **JsTermlet spawn**: `useTermlet("bash")` creates and returns handle.
35. **JsTermlet snapshot**: `t.snapshot()` returns string.
36. **JsTermlet cleanup**: `t.kill()` sends SIGTERM.
37. **[Pass 2] Cross-language consistency**: same scenario produces same snapshot text across Rust, Python, Node.
38. **Performance B10**: FakePty spawn < 500us.
39. **Performance B11**: 80x24 snapshot < 50us.
40. **Performance B12**: Real PTY spawn < 50ms.

### AGENTS.md Rules

- `RULE-S32-01`: Termlet snapshot text must be trailing-whitespace-trimmed. No trailing spaces on any line, no trailing empty lines.
- `RULE-S32-02`: Termlet kill must be idempotent. Double-kill is not an error.
- `RULE-S32-03`: Termlet Drop must send SIGKILL to prevent PTY process leaks.
- `RULE-S32-04`: FakePtyBackend tests must not depend on timing. They must be fully deterministic.
- `RULE-S32-05`: Termlet bindings (Python/Node) must implement context manager / cleanup pattern.
- `RULE-S32-06`: `mux-termlet` must not depend on `mux-server`, `mux-api`, or `mux-orm`.
- `RULE-S32-07`: Every Termlet API method must be tested in at least Rust and one binding language.
- `RULE-S32-08`: **[Pass 2]** `wait_for` must compile pattern once via PatternMatcher, not per poll iteration.
- `RULE-S32-09`: **[Pass 2]** Termlet remains pane-backed; never invent parallel terminal core.
- `RULE-S32-10`: **[Pass 2]** Cross-language snapshot consistency tests must exist for every release.

---

## v10 Pass 2 Consistency Checklist

- [x] All 32 required sections are present and complete.
- [x] Every section includes: Design Decisions, Rust Example, Test Strategy, AGENTS.md Rules.
- [x] Section 32 (Termlets) has full depth: architecture, API, snapshot, usage examples, FakePty integration, OTEL, comparison table, error code reference.
- [x] Section 4 (Workspace Layout) includes `mux-termlet` crate at Layer 2.
- [x] Section 8 (Errors) includes `TermletError` with all variants including `PatternNotFound` [P2].
- [x] Section 16 (Language Bindings) includes `PyTermlet` and `JsTermlet` with `Mutex<Termlet>` wrapper [P2].
- [x] Section 22 (Binding Test Frameworks) includes pytest `termlet`/`fake_termlet`/`any_termlet` fixtures and vitest `useTermlet()`.
- [x] Section 25 (TUI) includes `TermletViewModel` for debug inspector [P2].
- [x] Section 31 (Test Matrix) includes Termlet test categories and release gates M-32-01 through M-32-07.
- [x] Acceptance criteria P7/P8/P9 (Termlet usability) and B10/B11/B12 (Termlet performance) and E6/E7 (Termlet ecosystem) in Section 2.
- [x] Section 26 (Rules) includes 40 rules including Termlet-specific and Pass 2 additions.
- [x] Section 27 (Risks) includes R31-R37 for Termlet-specific risks including Pass 2 additions.
- [x] Section 30 (Types) includes `Termlet`, `TermletConfig`, `TermletSnapshot`, `PtyMode`, `WaitMatch`, `PatternMatcher`, `ScenarioStep`, `TermletViewModel`.
- [x] Settled decision S17 (synchronous API for mux-termlet) added [P2].
- [x] Gate enum in Section 2 includes P7/P8/P9/B10/B11/B12 Termlet gates.
- [x] OTEL span naming convention in Section 19 includes termlet spans.
- [x] All [v10] and [v10 Pass 2] / [P2] annotations mark new content clearly.
- [x] No v9 content removed. All 31 v9 sections preserved.
- [x] Reference anchors re-verified against 4 codebases during v10 Pass 2.
- [x] BLAKE3 fingerprint truncation documented (12-char at call site, line 543).
- [x] Cross-pollination from GPT: PatternMatcher, WaitMatch, PatternNotFound, TermletViewModel, ScenarioStep, stable error codes, configurable poll interval.
- [x] Cross-pollination from Gemini: MuxKernel alternative naming, Bound<'py, T> emphasis.
- [x] Weaknesses fixed: wait_for returns WaitMatch not bool, PatternNotFound vs Timeout, compiled patterns, Mutex<Termlet> in bindings, GIL release.
- [x] Rule naming follows `RULE-Snn-xx` convention throughout.
- [x] Document writes only to `notes/` and is standalone.
- [x] Lineage: v9 Pass 3 Final -> v10 Pass 1 (3 models) -> v10 Pass 2 (this document).
- [x] Rust 2024 edition idioms used throughout.

---

*End of TermForge v10 Pass 2 Architecture Specification (Cross-Pollinated Refinement).*
