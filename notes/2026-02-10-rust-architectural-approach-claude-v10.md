# TermForge v10 Architecture Specification (Pass 1)

Date: 2026-02-11
Status: **v10 Pass 1** -- Termlets integration into v9 Pass 3 Final.
Lineage: v4 (6-model, 2519 lines) -> v5 (3-model, 1573 lines) -> v6 (3-pass, 5239 lines) -> v7 Final -> deep-dive review (20 gaps) -> v8 Pass 1-3 (6612 lines) -> v9 Pass 1 (Claude 4502 + GPT 1392 + Gemini 638) -> v9 Pass 2 (Claude 4212 + GPT 1151 + Gemini 784) -> v9 Pass 3 Final (4190 lines, 31 sections) -> **v10 Pass 1** (this document, 32 sections, Termlets).
License: MIT OR Apache-2.0
Rust edition: 2024 (MSRV 1.85)
Protocol target: tmux protocol v8

---

## Preamble

This document is the single authoritative architectural reference for TermForge, a Rust terminal multiplexer with 100% tmux wire-protocol compatibility, ORM-like API, language bindings, CRDT collaboration, a ratatui-based TUI client, and **Termlets** -- SDK-first testing pods that are the killer differentiator.

### v10 Pass 1 Synthesis Notes

v10 introduces the **Termlet** concept: simplified, embeddable, language-binding-native testing pods that wrap tmux panes into ergonomic handles. Termlets are the bridge between TermForge's multiplexer kernel and the testing ecosystem -- making it trivially easy to spawn a shell, send commands, capture output, and snapshot-test the result from Rust, Python, or Node.js.

**Major additions in v10:**
1. **Section 32: Termlets** -- full specification of the Termlet concept, API, architecture, and test strategy.
2. **Section 4 updated** -- `mux-termlet` crate added to workspace layout at Layer 2.
3. **Section 16 updated** -- `PyTermlet` and `JsTermlet` binding handles.
4. **Section 22 updated** -- pytest `termlet` fixture, vitest `useTermlet()` helper.
5. **Section 31 updated** -- Termlet test categories in the test matrix.

**Base document:** v9 Pass 3 Final (4,190 lines, 31 sections). All v9 content is preserved; sections updated for Termlet integration are marked with `[v10]`.

**Reference codebases (re-verified for v10):**
- `~/work/python/libtmux/src/libtmux/_internal/query_list.py` -- 12 operators confirmed in `LOOKUP_NAME_MAP` (lines 298-312): `eq` (alias for exact), `exact`, `iexact`, `contains`, `icontains`, `startswith`, `istartswith`, `endswith`, `iendswith`, `in`, `nin`, `regex`, `iregex`. Callable matcher in `filter()` (line 535). `get(default=no_arg)` semantics (lines 550-569). `keygetter` nested `__` traversal (lines 45-113). Exceptions: `MultipleObjectsReturned`, `ObjectDoesNotExist`, `PKRequiredException`, `OpNotFound`.
- `~/work/rust/vibe-tmux/crates/mux-test-support/src/path_guard.rs` -- 3-layer socket validation confirmed: `ensure_not_default_socket_name` (line 16), `ensure_socket_within_tempdir` (line 30), `ensure_socket_not_tmux_env` (line 45). `tmux_socket_path` format (line 76): `${TMUX_TMPDIR}/tmux-<uid>/<name>`. Tests at lines 80-106.
- `~/work/rust/vibe-tmux/tools/tmux-builder/src/lib.rs` -- BLAKE3 `flags_fingerprint` confirmed (line 511): null-separated flag bytes hashed via `blake3::hash`, truncated to 12 hex chars. `compute_cache_key` format (line 521): `tmux-{version}__{host}__{os}-{os_ver}__cfg{cfg}__mk{mk}__tb{tool}`. `LockGuard` with `Drop` for `fs2::FileExt::unlock` (lines 247-256). `sibling_tmp_dir` with `pid+nanos` (line 409). Atomic `std::fs::rename` publication (line 389). `validate_tmux_binary` via `tmux -V` (line 549).
- `~/work/rust/vibe-tmux/crates/mux-otel/src/otel.rs` -- Dual providers confirmed: `OTEL_PROVIDER` + `MUX_CLIENT_PROVIDER` as `OnceLock<Mutex<Option<OtelProvider>>>` (lines 58-59). `OtelProvider` struct with `logger: Option<SdkLoggerProvider>`, `tracer_provider: Option<SdkTracerProvider>`, `tracer: Option<Tracer>`, `runtime: Option<tokio::runtime::Runtime>` (lines 65-71). `enter_mux_client_span` lazy init (line 470). Composite propagator: `BaggagePropagator` + `TraceContextPropagator` (lines 771-777). `HeaderCarrier` with `Injector`/`Extractor` (lines 998-1028). `force_flush` both providers (line 533). `shutdown` takes providers via `.take()` (line 576). `runtime.shutdown_timeout(Duration::from_millis(200))` (line 275). Thread-local `TRACE_HEADERS_STACK` (not shown in otel.rs but referenced via imports from crate root). `MuxClientSpanGuard` with `Drop` that ends span + force-flushes (line 450).
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

---

## Table of Contents

1. [Vision and Philosophy](#1-vision-and-philosophy)
2. [North Star Acceptance Criteria](#2-north-star-acceptance-criteria)
3. [High-Level Architecture](#3-high-level-architecture)
4. [Workspace Layout](#4-workspace-layout) [v10 updated]
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
16. [Language Bindings](#16-language-bindings) [v10 updated]
17. [CRDT Transaction Layer](#17-crdt-transaction-layer)
18. [Security Model](#18-security-model)
19. [OpenTelemetry](#19-opentelemetry)
20. [tmux Version Management](#20-tmux-version-management)
21. [Test Support and Fake PTY](#21-test-support-and-fake-pty)
22. [Binding Test Frameworks](#22-binding-test-frameworks) [v10 updated]
23. [Test Framework and Harness Design](#23-test-framework-and-harness-design)
24. [Performance Targets](#24-performance-targets)
25. [Visual Client / TUI](#25-visual-client--tui)
26. [AGENTS.md Rules](#26-agentsmd-rules)
27. [Risks and Mitigations](#27-risks-and-mitigations)
28. [Plan Evolution and Changelog](#28-plan-evolution-and-changelog)
29. [Reference Anchors](#29-reference-anchors)
30. [Appendix: Canonical Type Quick Reference](#30-appendix-canonical-type-quick-reference)
31. [Supplemental Test Matrix](#31-supplemental-test-matrix) [v10 updated]
32. [Termlets](#32-termlets) [v10 new]

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
- **[v10] Termlets provide the SDK-first testing surface: spawn, interact, snapshot, destroy.** They are the primary entry point for users who want to test CLI tools, shell scripts, or interactive programs without understanding multiplexer internals.

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

| ID | Target | Basis |
|---|---|---|
| B1 | VT100 parser > 300 MB/s (plain ASCII) | alacritty vte ~500 MB/s |
| B2 | VT100 parser > 100 MB/s (CSI heavy) | Parameter parsing overhead |
| B3 | Protocol decode > 500 MB/s | 16-byte header + memcpy |
| B4 | Layout resize < 50 us (20 panes) | Tree walk, no alloc |
| B5 | Graph snapshot < 1 ms | Arc clone + BTreeMap for 50 panes |
| B6 | Option resolve (4-level chain) < 100 ns | 4 BTreeMap lookups |
| B7 | Config parse (500 lines) < 5 ms | Lexer + parser, no IO |
| B8 | Format expand (status line) < 50 us | ~10 variable lookups |
| B9 | Control notification parse < 200 ns | String split + parse |
| B10 | **[v10]** Termlet spawn (FakePty) < 500 us | No real PTY overhead |
| B11 | **[v10]** Termlet snapshot (80x24 grid) < 50 us | Grid text extraction |

These are conservative targets validated after establishing baselines from 3 consecutive median runs. CI regression gate: 130% threshold on nightly only.

### 2.5 Ecosystem (E)

| ID | Criterion | Verification |
|---|---|---|
| E1 | Python bindings: `pip install termforge` | maturin build + PyPI publish |
| E2 | Node bindings: `npm install termforge` | Neon build + npm publish |
| E3 | pytest fixture chain: server -> session -> window -> pane | pytest plugin with hermetic server |
| E4 | vitest fixture chain: same pattern in TypeScript | vitest setup with hermetic server |
| E5 | Snapshot testing from Python/Node against PTY output | `insta`-compatible snapshots via bindings |
| E6 | **[v10]** pytest `termlet` fixture available out of the box | pytest plugin ships with termlet support |
| E7 | **[v10]** vitest `useTermlet()` available out of the box | npm package ships with termlet helper |

### Rust Example

```rust
/// CI gate enum -- every gate must be green before release.
/// Named variants from GPT synthesis for clarity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gate {
    C1WireAttach,
    C2CrossAttach,
    C3ProtocolRoundtrip,
    C4IdentifyBurst,
    C5ScmRights,
    C6CommandParity,
    C7FormatParity,
    C8KeyBindingParity,
    A1PureCore,
    A2WasmPurity,
    A3UnsafeQuarantine,
    A4Determinism,
    A5NoPanicsHotPath,
    A6BindingDeps,
    A7NoMutexReadPath,
    P1Embedding,
    P2PytestFixture,
    P3SnapshotTest,
    P4CrdtMerge,
    P5TuiRealTmux,
    P6FakePtyParity,
    P7TermletRustSpawn,        // [v10]
    P8TermletPytestFixture,    // [v10]
    P9TermletVitestHelper,     // [v10]
    B1ParserAscii,
    B2ParserCsi,
    B3ProtoDecode,
    B10TermletSpawnFake,       // [v10]
    B11TermletSnapshot,        // [v10]
    WasmPurityProof,
    OTelEndToEnd,
    TestIsolation,
    PerformanceBaseline,
}

pub fn all_gates_green(gates: &[(Gate, bool)]) -> bool {
    gates.iter().all(|(_, ok)| *ok)
}
```

### Test Strategy

1. CI gate matrix keyed by `Gate` enum.
2. Hard fail when any compatibility gate regresses.
3. Nightly parity runs across multiple tmux versions (3.3a, 3.4, 3.5, 3.6).
4. WASM purity proof: `cargo check --target wasm32-unknown-unknown` for all Layer 0 crates.
5. **[v10]** Termlet gates P7/P8/P9 run on every PR.

### AGENTS.md Rules

- `RULE-S2-01`: PRs touching protocol must include parity evidence.
- `RULE-S2-02`: New features cannot bypass existing gate suite.

---

## 3. High-Level Architecture

### Design Decisions

- Six-layer model: Pure Core (L0) -> Platform (L1) -> Runtime (L2) -> API Facade (L3) -> Bindings (L4) -> Tools/TUI (L5).
- Dependencies flow strictly inward. Layer N depends on N-1 but never on N+1.
- Layer 0 is pure: no IO, no async, no unsafe. Enforced by WASM CI gate.
- Layer 1 handles all platform concerns: PTY, sockets, signals, file locking.
- Bindings are consumers of `mux-orm` only, never runtime internals.
- Single state actor owns `ServerGraph`. Readers get `ArcSwap` snapshots.
- **[v10]** `mux-termlet` sits at Layer 2 (FACADE), composing Layer 0 (mux-core, mux-grid) and Layer 1 (mux-pty) into an ergonomic single-handle API. Termlets are a *consumer* of the kernel, not part of it.

### 3.1 The Layer Cake

```
Layer 0  PURE KERNEL     mux-types, mux-core, mux-query, mux-conf,
                          mux-command, mux-grid, mux-control, mux-pty-fake
                          mux-crdt, mux-proto

Layer 1  IMPURE RUNTIME  mux-os, mux-pty, mux-pty-portable, mux-client,
                          mux-server, mux-refresh, mux-backend

Layer 2  FACADE           mux-api (ManagedMux, SocketActor, intents)
                          mux-termlet (Termlet, TermletConfig)  [v10]

Layer 3  ORM              mux-orm (libtmux-style graph + QueryList sugar)

Layer 4  BINDINGS         bindings/python (PyO3), bindings/node (Neon),
                          crates/mux-cxx (cxx bridge)

Layer 5  TOOLS            termforge-cli, tmux-vm, tmux-sniff, tmux-builder,
                          mux-tui, mux-doctor, format-audit, mux-regress,
                          tmux-command-audit
```

### 3.2 State Ownership

| Actor | Owns | Accessed By |
|---|---|---|
| State Actor | `ServerGraph` (single writer) | Event submissions |
| `ArcSwap<GraphState>` | Immutable snapshot | All readers (UI, bindings, ORM) |
| Effect Dispatcher | Side-effect execution | State actor via effect queue |
| **[v10]** Termlet | Private `Pane` + `Grid` + `PtyBackend` | Termlet owner (single-threaded handle) |

### Rust Example

```rust
/// Full-stack composition for in-process embedding.
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

1. Dependency direction checks via `cargo metadata` assertions.
2. Architecture contract tests: forbid illegal cross-layer imports.
3. In-process and socket-mode integration tests from same scripts.
4. WASM purity proof for all Layer 0 crates.
5. **[v10]** `mux-termlet` depends only on Layer 0 + Layer 1 crates. Verified via `cargo tree`.

### AGENTS.md Rules

- `RULE-S3-01`: Dependencies flow inward only. Layer N never depends on N+1.
- `RULE-S3-02`: Bindings cannot depend on runtime internals.
- `RULE-S3-03`: **[v10]** `mux-termlet` must not depend on `mux-server` or `mux-api`. It composes lower layers directly.

---

## 4. Workspace Layout [v10 updated]

### Design Decisions

Monorepo with `mux-*` prefix matching existing vibe-tmux workspace convention. Pure crates in Layer 0, impure in Layer 1, facade/ORM in Layer 2-3, bindings in Layer 4, tools in Layer 5. **[v10]** `mux-termlet` added as a Layer 2 FACADE crate, sitting alongside `mux-api`.

### Rust Example

```
termforge/
  Cargo.toml                     # [workspace]
  crates/
    -- LAYER 0: PURE (no IO, no async, no unsafe, WASM-compatible) --
    mux-types/                   # Shared types: IDs, sizes, enums, ErrorClass
    mux-core/                    # ServerGraph, Event, Effect, Engine, entities
    mux-grid/                    # VT100 parser, Grid, Cell
    mux-query/                   # QueryList, QueryOp, QueryBuilder
    mux-conf/                    # tmux.conf parser -> Vec<Event>
    mux-command/                 # Command table + dispatch
    mux-proto/                   # imsg codec, MsgType, payload parsers
    mux-control/                 # Control mode parser (typed notifications)
    mux-crdt/                    # HLC, LWW, OrSet, OpLog
    mux-pty-fake/                # FakePtyBackend, ScenarioRecorder/Replayer
    mux-view/                    # ViewModel construction (pure)

    -- LAYER 1: IMPURE (IO, async, platform) --
    mux-os/                      # Unsafe quarantine: PTY, SCM_RIGHTS, signals
    mux-pty/                     # PtyBackend trait + real implementation
    mux-pty-portable/            # portable-pty backend
    mux-server/                  # Tokio runtime, socket accept loop
    mux-client/                  # Client connection, control mode
    mux-refresh/                 # StateStore + ArcSwap + RefreshPlanner
    mux-backend/                 # Backend trait: Local + Tmux
    mux-otel/                    # OpenTelemetry integration
    mux-test-support/            # TmuxTestServer, PathGuard, hermetic isolation

    -- LAYER 2: FACADE --
    mux-api/                     # ManagedMux, StateHandle, intent API
    mux-termlet/                 # [v10] Termlet: spawn, interact, snapshot, resize, kill

    -- LAYER 3: ORM --
    mux-orm/                     # OrmServer, OrmSession, OrmWindow, OrmPane

    -- SUPPORT --
    mux-cxx/                     # cxx bridge
    mux-bench/                   # Criterion benchmarks

  tools/
    tmux-sniff/                  # Protocol capture proxy (preserves SCM_RIGHTS)
    tmux-vm/                     # tmux version manager (ensure/exec)
    tmux-builder/                # Build tmux from source (BLAKE3 cache)
    tmux-worktrees/              # Git worktree manager for tmux source
    tmux-command-audit/          # Command coverage tracker
    format-audit/                # Format variable alignment checker
    mux-regress/                 # Parity harness (both servers)
    mux-tui/                     # Rust-native TUI client
    mux-doctor/                  # Diagnostic tool

  bindings/
    python/                      # PyO3 bindings + pytest plugin (includes PyTermlet)
    node/                        # Neon bindings + vitest (includes JsTermlet)

  fixtures/                      # Protocol captures, scenario recordings
  fuzz/                          # cargo-fuzz targets
  notes/                         # Architecture specs, reviews
```

### Purity Classification

| Crate | Layer | `#![forbid(unsafe_code)]` | WASM CI | Key Dependencies |
|---|---|---|---|---|
| mux-types | 0 | Yes | Yes | thiserror, serde, smallvec |
| mux-core | 0 | Yes | Yes | mux-types, slotmap |
| mux-grid | 0 | Yes | Yes | mux-types |
| mux-proto | 0 | Yes | Yes | mux-types, bytes |
| mux-conf | 0 | Yes | Yes | mux-types |
| mux-command | 0 | Yes | Yes | mux-types |
| mux-control | 0 | Yes | Yes | mux-types |
| mux-query | 0 | Yes | Yes | mux-types, regex |
| mux-crdt | 0 | Yes | Yes | mux-types |
| mux-pty-fake | 0 | Yes | Yes | mux-types, mux-pty (trait only) |
| mux-view | 0 | Yes | Yes | mux-types, mux-core |
| mux-os | 1 | No | No | libc, nix, tokio |
| mux-pty | 1 | No | No | mux-types, nix |
| mux-server | 1 | No | No | mux-core, mux-os, mux-proto, tokio |
| mux-client | 1 | No | No | mux-proto, mux-os, tokio |
| mux-otel | 1 | Yes | No | tracing, opentelemetry-sdk |
| mux-api | 2 | Yes | No | mux-types, mux-core, arc-swap |
| **mux-termlet** | **2** | **Yes** | **No** | **mux-core, mux-grid, mux-pty, mux-pty-fake** [v10] |
| mux-orm | 3 | Yes | No | mux-types, mux-query, mux-api |
| mux-tui | 5 | Yes | No | mux-api, mux-view, ratatui, crossterm |

### Test Strategy

1. Workspace lint verifies required crates exist.
2. `cargo check --workspace --all-targets` on every PR.
3. `cargo tree` policy checks for forbidden dependencies in pure crates.
4. Any new crate must be classified pure/impure in ARCHITECTURE.md.
5. **[v10]** `mux-termlet` dependency audit: must not depend on `mux-server`, `mux-api`, or `mux-orm`.

### AGENTS.md Rules

- `RULE-S4-01`: Do not place new core logic in bindings/tools crates.
- `RULE-S4-02`: Any new crate must be classified pure/impure with WASM check status.
- `RULE-S4-03`: **[v10]** `mux-termlet` lives at Layer 2. It depends on Layer 0 + Layer 1 only.

---

## 5. Layering Contract

### Design Decisions

- Pure crates: no `unsafe`, no `tokio`, no `std::net`, no `std::fs`, no ambient time or randomness.
- Runtime crates own all side effects.
- The reducer is the only mutation entrypoint for graph state.
- Time and randomness are injected via `CoreCtx`, never read from OS.
- Protocol is an adapter: `mux-proto` translates wire format to domain events.
- ORM is a facade: it exposes a nice API but implements no multiplexer logic.
- **[v10]** `mux-termlet` is a facade that composes Grid + Pane + PtyBackend into a single handle. It owns its own mini graph state (one pane, one grid) rather than going through the full ServerGraph. This is deliberate: Termlets are lightweight and do not require the full multiplexer machinery.

### Boundary Rules

| Boundary | Rule |
|---|---|
| Layer 0 -> Layer 1 | Never. Pure crates have no platform dependency. |
| Layer 1 -> Layer 0 | Allowed. Platform code reads types and invokes pure functions. |
| Layer 2 -> Layer 0 | Allowed. Runtime submits events and reads effects. |
| Layer 2 -> Layer 1 | **[v10]** Allowed. `mux-termlet` uses `mux-pty` for real PTY mode. |
| Layer 3 -> Layer 2 | Allowed. API wraps runtime. |
| Layer 4 -> Layer 3 | Allowed. Bindings call ORM/API only. |
| Layer 4 -> Layer 2 | **[v10]** Allowed for Termlet bindings. `PyTermlet` wraps `mux-termlet::Termlet`. |
| Layer 4 -> Layer 0 | Forbidden. Bindings must not bypass the API layer. |

### Rust Example

```rust
// IO Boundary Contract:
// Pure core communicates with the outside world exclusively through:
// - Inbound: Event variants submitted to apply_event()
// - Outbound: Effect variants returned from apply_event()
// - Time: Injected via Event::Tick { now_millis }, never from OS
// - Randomness: Injected via CoreCtx { rand_u64 }, never from OS RNG

// WRONG -- core asking the OS for time
pub fn create_session_bad(graph: &mut ServerGraph) -> SessionId {
    let name = format!("session-{}", std::time::SystemTime::now()
        .elapsed().unwrap().as_secs());
    graph.create_session(name)
}

// RIGHT -- core receiving information via events
pub fn apply_event(
    graph: &mut ServerGraph,
    event: Event,
    ctx: &CoreCtx,
) -> Vec<Effect> {
    match event {
        Event::CreateSession { name, created_at, .. } => {
            let _id = graph.sessions.insert(Session {
                name, created_at, ..Default::default()
            });
            vec![Effect::PublishSnapshot]
        }
        _ => vec![],
    }
}
```

### Test Strategy

1. Compile-fail tests for illegal imports in pure crates.
2. WASM check: `cargo check --target wasm32-unknown-unknown` for all Layer 0 crates.
3. CI grep for `unsafe` outside approved crates.
4. `cargo deny check` for forbidden dependency edges.

### AGENTS.md Rules

- `RULE-S5-01`: Pure crates must keep `#![forbid(unsafe_code)]`.
- `RULE-S5-02`: Time/random values enter core via `CoreCtx` only.

---

## 6. Entity Model

### Design Decisions

- `ServerGraph` is the single mutable authority. It contains `SlotMap<SessionId, Session>`, `SlotMap<WindowId, Window>`, etc.
- Parent-child relationships via `Vec` inside parent entity: `Session.windows: Vec<WindowId>`, `Window.panes: Vec<PaneId>`.
- No `SecondaryMap` for child->parent. Reverse lookups computed during snapshot construction.
- `GraphState` is the immutable snapshot with precomputed reverse maps (`window_for_pane`, `session_for_window`).
- Entity IDs are generational via `slotmap::new_key_type!`, eliminating use-after-free bugs.
- **[v10]** Termlets do NOT use entity IDs from the `ServerGraph`. A Termlet owns a standalone `Pane` + `Grid` + `PtyBackend` tuple. It has a `TermletId` (a simple `u64` counter) for identification in binding layers but does not participate in the SlotMap entity graph.

### Rust Example

```rust
// crates/mux-types/src/ids.rs
use slotmap::new_key_type;

new_key_type! {
    pub struct SessionId;
    pub struct WindowId;
    pub struct PaneId;
    pub struct ClientId;
    pub struct JobId;
    pub struct BufferId;
}

// crates/mux-core/src/graph.rs
use slotmap::SlotMap;

pub struct ServerGraph {
    pub sessions: SlotMap<SessionId, Session>,
    pub windows: SlotMap<WindowId, Window>,
    pub panes: SlotMap<PaneId, Pane>,
    pub clients: SlotMap<ClientId, Client>,
    pub global_options: OptionSet,
    pub next_session_number: u32,
}

#[derive(Debug, Clone)]
pub struct Session {
    pub name: String,
    pub windows: Vec<WindowId>,
    pub active_window: Option<WindowId>,
    pub created_at: i64,
    pub options: OptionSet,
}

#[derive(Debug, Clone)]
pub struct Window {
    pub name: String,
    pub panes: Vec<PaneId>,
    pub active_pane: Option<PaneId>,
    pub layout: LayoutTree,
    pub options: OptionSet,
}

#[derive(Debug, Clone)]
pub struct Pane {
    pub grid: Grid,
    pub parser: VtParser,
    pub pid: Option<u32>,
    pub cwd: String,
    pub command: String,
    pub size: PaneSize,
    pub options: OptionSet,
}

#[derive(Debug, Clone)]
pub struct Client {
    pub identify: IdentifyState,
    pub terminal_name: String,
    pub terminal_features: Vec<String>,
    pub session: Option<SessionId>,
}

#[derive(Debug, Clone, Copy)]
pub struct PaneSize { pub sx: u16, pub sy: u16 }

// Immutable snapshot with precomputed reverse maps.
pub struct GraphState {
    pub sessions: Vec<(SessionId, Session)>,
    pub windows: Vec<(WindowId, Window)>,
    pub panes: Vec<(PaneId, Pane)>,
    pub session_for_window: HashMap<WindowId, SessionId>,
    pub window_for_pane: HashMap<PaneId, WindowId>,
    pub global_options: OptionSet,
}

impl ServerGraph {
    pub fn graph_state(&self) -> GraphState {
        let mut session_for_window = HashMap::new();
        let mut window_for_pane = HashMap::new();
        for (session_id, session) in &self.sessions {
            for &window_id in &session.windows {
                session_for_window.insert(window_id, session_id);
                if let Some(window) = self.windows.get(window_id) {
                    for &pane_id in &window.panes {
                        window_for_pane.insert(pane_id, window_id);
                    }
                }
            }
        }
        GraphState {
            sessions: self.sessions.iter().map(|(id, s)| (id, s.clone())).collect(),
            windows: self.windows.iter().map(|(id, w)| (id, w.clone())).collect(),
            panes: self.panes.iter().map(|(id, p)| (id, p.clone())).collect(),
            session_for_window,
            window_for_pane,
            global_options: self.global_options.clone(),
        }
    }
}
```

### Test Strategy

1. Entity ID never reused after removal (slotmap generation check).
2. Reverse maps: `window_for_pane` includes all panes in all windows.
3. Empty graph snapshot is valid.
4. Session -> Window -> Pane hierarchy preserved after add/remove cycles.
5. GraphState reverse map consistency: every pane has a window, every window has a session.

### AGENTS.md Rules

- `RULE-S6-01`: All entity IDs via `slotmap::new_key_type!`.
- `RULE-S6-02`: No direct child->parent back-pointers. Use reverse maps in GraphState.

---

## 7. Event/Effect Engine

### Design Decisions

- Pure state reducer: `fn apply_event(graph, event, ctx) -> Vec<Effect>`.
- `Event` describes what happened. `Effect` describes what the runtime should do.
- All events are deterministic. Async operations communicate only through events.
- `Effect::PublishSnapshot` triggers `ArcSwap` update.
- `Event::EffectFailed` feeds failures back into the reducer for recovery.
- Graph updates are returned from the kernel trait as part of the outcome.

### Rust Example

```rust
// crates/mux-core/src/event.rs
#[derive(Debug, Clone)]
pub enum Event {
    CreateSession { name: String, cwd: Option<String>, created_at: i64 },
    DestroySession { session: SessionId },
    NewWindow { session: SessionId, name: String },
    SplitWindow { window: WindowId, direction: LayoutType, size: Option<u16> },
    PaneOutput { pane: PaneId, data: Vec<u8> },
    PaneExited { pane: PaneId, status: i32 },
    Resize { pane: PaneId, size: PaneSize },
    Key { client: ClientId, key: KeyEvent },
    SetOption { scope: OptionScope, target: TargetId, key: String, value: OptionValue },
    UnsetOption { scope: OptionScope, target: TargetId, key: String },
    ClientIdentify { client: ClientId, data: IdentifyData },
    ConfigLine { line: String },
    Command { name: String, args: Vec<String>, client_id: Option<ClientId> },
    Tick { now_millis: i64 },
    EffectFailed { effect_id: u64, error: String, class: ErrorClass },
}

#[derive(Debug, Clone)]
pub enum Effect {
    SpawnPty { pane: PaneId, command: Vec<String>, cwd: Option<String>, size: PaneSize },
    KillPty { pane: PaneId, signal: i32 },
    ResizePty { pane: PaneId, size: PaneSize },
    SendFrame { client_id: u64, frame: Vec<u8> },
    PublishSnapshot,
    WriteControlNotification { client: ClientId, notification: String },
    Shutdown,
    LoadConfig { path: String },
}

// crates/mux-core/src/engine.rs
pub fn apply_event(
    graph: &mut ServerGraph,
    event: Event,
    ctx: &CoreCtx,
) -> Vec<Effect> {
    match event {
        Event::CreateSession { name, created_at, .. } => {
            let session_id = graph.sessions.insert(Session {
                name: name.clone(),
                windows: Vec::new(),
                active_window: None,
                created_at,
                options: OptionSet::new(),
            });
            graph.next_session_number += 1;
            vec![Effect::PublishSnapshot]
        }
        Event::PaneOutput { pane, data } => {
            if let Some(p) = graph.panes.get_mut(pane) {
                p.parser.parse(&data, &mut p.grid);
            }
            vec![Effect::PublishSnapshot]
        }
        Event::EffectFailed { effect_id, error, class } => {
            tracing::warn!(effect_id, %error, ?class, "effect failed");
            vec![]
        }
        _ => vec![],
    }
}
```

### Test Strategy

1. CreateSession produces PublishSnapshot effect.
2. PaneOutput updates grid state.
3. EffectFailed logged but does not mutate graph.
4. Unknown events produce empty effect list.
5. Determinism: same event sequence produces identical graph.
6. Event replay from OpLog reconstructs identical state.

### AGENTS.md Rules

- `RULE-S7-01`: Core emits effects; only the runtime executes effects.
- `RULE-S7-02`: All state transitions go through `apply_event()`.

---

## 8. Error Handling

### Design Decisions

- `thiserror` for library errors in all pure crates.
- `anyhow` only in binary/tool crates and test harnesses.
- Error classification via `ErrorClass` enum: `Bug`, `UserError`, `Transient`, `ProtocolViolation`.
- Protocol violations kill the connection (matching `server-client.c:3472-3475`).
- `ProtocolError` has sub-variants for specific decode failures.
- Every `ErrorClass` variant must be covered by at least one test.
- **[v10]** `TermletError` added for Termlet-specific failures.

### Rust Example

```rust
// crates/mux-types/src/error.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorClass {
    Bug,                 // Internal logic error
    UserError,           // Bad config, bad command, etc.
    Transient,           // Temporary failure, retryable
    ProtocolViolation,   // Peer sent invalid data
}

pub trait ClassifiedError: std::error::Error {
    fn class(&self) -> ErrorClass;
}

// crates/mux-proto/src/error.rs
#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("frame too short: {len} bytes (minimum {IMSG_HDR_SIZE})")]
    FrameTooShort { len: usize },
    #[error("unknown message type: {msg_type}")]
    UnknownMsgType { msg_type: u32 },
    #[error("payload length mismatch: header says {header_len}, got {actual_len}")]
    PayloadMismatch { header_len: u32, actual_len: usize },
    #[error("payload exceeds maximum: {len} > {max}")]
    PayloadTooLarge { len: usize, max: usize },
    #[error("invalid identify burst: {reason}")]
    InvalidIdentify { reason: String },
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl ClassifiedError for ProtocolError {
    fn class(&self) -> ErrorClass {
        match self {
            Self::Io(_) => ErrorClass::Transient,
            _ => ErrorClass::ProtocolViolation,
        }
    }
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
    #[error("cannot split: dimension too small")]
    CannotSplit,
    #[error("invalid option key: {0}")]
    InvalidOption(String),
    #[error("layout invariant violated")]
    LayoutInvariant,
}

impl ClassifiedError for CoreError {
    fn class(&self) -> ErrorClass {
        match self {
            Self::SessionNotFound(_) | Self::WindowNotFound(_) | Self::PaneNotFound(_) =>
                ErrorClass::UserError,
            Self::CannotSplit | Self::InvalidOption(_) => ErrorClass::UserError,
            Self::LayoutInvariant => ErrorClass::Bug,
        }
    }
}

// [v10] crates/mux-termlet/src/error.rs
#[derive(Debug, thiserror::Error)]
pub enum TermletError {
    #[error("termlet spawn failed: {reason}")]
    SpawnFailed { reason: String },
    #[error("termlet already killed")]
    AlreadyKilled,
    #[error("wait_for timed out after {timeout_ms}ms waiting for {pattern:?}")]
    WaitForTimeout { pattern: String, timeout_ms: u64 },
    #[error("termlet resize failed: {reason}")]
    ResizeFailed { reason: String },
    #[error("PTY backend error: {0}")]
    Pty(#[from] PtyError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl ClassifiedError for TermletError {
    fn class(&self) -> ErrorClass {
        match self {
            Self::AlreadyKilled => ErrorClass::UserError,
            Self::WaitForTimeout { .. } => ErrorClass::Transient,
            Self::SpawnFailed { .. } | Self::ResizeFailed { .. } => ErrorClass::Transient,
            Self::Pty(_) | Self::Io(_) => ErrorClass::Transient,
        }
    }
}
```

### Test Strategy

1. Every `ErrorClass` variant is covered by at least one test.
2. Protocol violation triggers connection kill.
3. Transient error allows retry.
4. Bug error triggers internal alert.
5. Error display strings are stable (snapshot tested).
6. ProtocolError sub-variants tested individually.
7. **[v10]** TermletError variants: spawn failure, already killed, wait_for timeout, resize failure.

### AGENTS.md Rules

- `RULE-S8-01`: `thiserror` for library errors, `anyhow` only in binaries/tests.
- `RULE-S8-02`: Every `ErrorClass` variant must have test coverage.

---

## 9. Protocol Codec

### Design Decisions

- Protocol v8 (`tmux-protocol.h:23`).
- 16-byte `ImsgHdr` matching `imsg.h` exactly.
- Three-outcome decoder: `Ok(frame)`, `Incomplete`, `Fatal(error)`.
- FD flag stored in high bit of `msg_type` field.
- Maximum payload size enforced at decode time.
- Tokio-codec `Decoder`/`Encoder` integration for `Framed<UnixStream, ImsgCodec>`.

### Rust Example

```rust
// crates/mux-proto/src/imsg.rs
pub const IMSG_HDR_SIZE: usize = 16;

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
    Incomplete,
    Fatal(ProtocolError),
}

pub struct ImsgCodec { max_payload: usize }

impl ImsgCodec {
    pub fn new() -> Self { Self { max_payload: 64 * 1024 } }

    pub fn decode(&self, src: &mut bytes::BytesMut) -> DecodeOutcome {
        if src.len() < IMSG_HDR_SIZE { return DecodeOutcome::Incomplete; }
        let hdr = parse_header(&src[..IMSG_HDR_SIZE]);
        let total = hdr.len as usize;
        if total < IMSG_HDR_SIZE {
            return DecodeOutcome::Fatal(ProtocolError::FrameTooShort { len: total });
        }
        let payload_len = total - IMSG_HDR_SIZE;
        if payload_len > self.max_payload {
            return DecodeOutcome::Fatal(ProtocolError::PayloadTooLarge {
                len: payload_len, max: self.max_payload,
            });
        }
        if src.len() < total { return DecodeOutcome::Incomplete; }
        let frame_bytes = src.split_to(total);
        let payload = frame_bytes.slice(IMSG_HDR_SIZE..);
        DecodeOutcome::Ok(ImsgFrame { header: hdr, payload })
    }

    pub fn encode(&self, frame: &ImsgFrame, dst: &mut bytes::BytesMut) {
        write_header(&frame.header, dst);
        dst.extend_from_slice(&frame.payload);
    }
}

fn parse_header(bytes: &[u8]) -> ImsgHdr {
    let raw_type = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
    ImsgHdr {
        msg_type: raw_type & 0x7FFF_FFFF,
        len: u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
        peerid: u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
        pid: u32::from_le_bytes(bytes[12..16].try_into().unwrap()),
        has_fd: (raw_type & 0x8000_0000) != 0,
    }
}

fn write_header(hdr: &ImsgHdr, dst: &mut bytes::BytesMut) {
    use bytes::BufMut;
    let raw_type = if hdr.has_fd { hdr.msg_type | 0x8000_0000 } else { hdr.msg_type };
    dst.put_u32_le(raw_type);
    dst.put_u32_le(hdr.len);
    dst.put_u32_le(hdr.peerid);
    dst.put_u32_le(hdr.pid);
}

// Tokio codec integration for Framed<UnixStream, ImsgCodec>
impl tokio_util::codec::Decoder for ImsgCodec {
    type Item = ImsgFrame;
    type Error = ProtocolError;

    fn decode(&mut self, src: &mut bytes::BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match ImsgCodec::decode(self, src) {
            DecodeOutcome::Ok(frame) => Ok(Some(frame)),
            DecodeOutcome::Incomplete => Ok(None),
            DecodeOutcome::Fatal(e) => Err(e),
        }
    }
}

impl tokio_util::codec::Encoder<ImsgFrame> for ImsgCodec {
    type Error = ProtocolError;

    fn encode(&mut self, item: ImsgFrame, dst: &mut bytes::BytesMut) -> Result<(), Self::Error> {
        ImsgCodec::encode(self, &item, dst);
        Ok(())
    }
}
// Enables: Framed<UnixStream, ImsgCodec> for async read/write with backpressure.
```

### Test Strategy

1. Header roundtrip: encode + decode ImsgHdr preserves all fields.
2. FD flag encoding: `has_fd = true` sets high bit; decode recovers it.
3. Frame roundtrip: arbitrary payload roundtrips through encode/decode.
4. Incomplete: partial header -> Incomplete, partial payload -> Incomplete.
5. Fatal on short frame: `len < 16` -> Fatal(FrameTooShort).
6. Fatal on oversized: payload exceeding max -> Fatal(PayloadTooLarge).
7. Real captures: decode fixtures captured from real tmux via `tmux-sniff`.
8. Tokio Decoder/Encoder: Framed stream roundtrips frames correctly.
9. Fuzz: arbitrary byte streams survive decode without panic.

### AGENTS.md Rules

- `RULE-S9-01`: No silent frame drops after structural decode failure.
- `RULE-S9-02`: Any new msg type requires fixture and decode/encode tests.

---

## 10. Configuration System

### Design Decisions

- Parse config into domain commands/events, never direct graph mutation.
- Match tmux option scope behavior: Server, Session, Window/Pane with FALLTHROUGH (options.c:891-903).
- `set -u` removes local override (inheritance restored). `set -U` also clears pane-local values.
- Config is loaded only after first client identify burst completes (settled decision S7).

### Rust Example

```rust
// crates/mux-types/src/options.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionScope { Server, Session, Window, Pane }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableScope { Server, Session, WindowPane }

#[derive(Debug, Clone, Copy, Default)]
pub struct SetFlags { pub g: bool, pub p: bool, pub w: bool, pub s: bool }

/// Resolve effective scope with WindowPane FALLTHROUGH.
/// Matches options.c:891-903.
pub fn resolve_scope(table_scope: TableScope, flags: SetFlags) -> OptionScope {
    match table_scope {
        TableScope::WindowPane if flags.p => OptionScope::Pane,
        TableScope::WindowPane => OptionScope::Window,
        TableScope::Session => OptionScope::Session,
        TableScope::Server => OptionScope::Server,
    }
}

// crates/mux-core/src/options.rs
/// Resolve an option value by walking the scope chain.
/// Matches tmux options_get() at options.c:228-241.
pub fn resolve_option(
    graph: &ServerGraph,
    scope: OptionScope,
    target: TargetId,
    key: &str,
) -> Option<OptionValue> {
    match scope {
        OptionScope::Pane => {
            if let Some(pane) = graph.panes.get(target.as_pane()?) {
                if let Some(val) = pane.options.get(key) {
                    return Some(val.clone());
                }
            }
            // FALLTHROUGH to window scope (options.c:903)
            let window_id = graph.window_for_pane(target.as_pane()?)?;
            resolve_option(graph, OptionScope::Window, TargetId::Window(window_id), key)
        }
        OptionScope::Window => {
            if let Some(window) = graph.windows.get(target.as_window()?) {
                if let Some(val) = window.options.get(key) {
                    return Some(val.clone());
                }
            }
            graph.global_options.get(key).cloned()
        }
        OptionScope::Session => {
            if let Some(session) = graph.sessions.get(target.as_session()?) {
                if let Some(val) = session.options.get(key) {
                    return Some(val.clone());
                }
            }
            graph.global_options.get(key).cloned()
        }
        OptionScope::Server => {
            graph.global_options.get(key).cloned()
        }
    }
}

// crates/mux-conf/src/lib.rs
/// Parse a tmux-compatible config file into events.
/// mux-conf depends on mux-types only, never on mux-core graph types.
pub fn parse_config(source: &str) -> Result<Vec<Event>, ConfigError> {
    let mut events = Vec::new();
    for line in source.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        events.push(parse_config_line(line)?);
    }
    Ok(events)
}
```

### Test Strategy

1. Pane FALLTHROUGH: set window option, query from pane scope, gets window value.
2. Local override: set pane-local option, query from pane scope, gets pane value.
3. Unset restores inheritance: set pane-local, unset, query returns window value.
4. Global unset resets default: unset server option, returns compiled default.
5. Config as events: parse config file, verify produces SetOption events.
6. Scope resolution matrix: all (scope, flag, target) combinations tested.
7. Table-driven tests for `(option_scope, flags) -> effective scope`.

### AGENTS.md Rules

- `RULE-S10-01`: Config parser outputs commands/events only. No direct graph mutation.
- `RULE-S10-02`: Option resolution tests must cover fallthrough cases.

---

## 11. Layout Engine

### Design Decisions

- Flat arena `Vec<LayoutCell>` with index-based parent/children.
- Round-robin resize: one cell at a time, matching `layout.c:448-462` exactly.
- Layout checksum: rotate-right-13 + add per byte, matching `layout-custom.c:46-57`.
- Layout validation: border = +1 per child, -1 total, matching `layout-custom.c:119-153`.
- Split requires `PANE_MINIMUM * 2 + 1` in split direction (`layout.c:937-950`).
- `debug_assert!(layout_check(...))` in all layout-mutating functions.

### Rust Example

```rust
// crates/mux-core/src/layout.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutType { LeftRight, TopBottom, Leaf }

#[derive(Debug, Clone)]
pub struct LayoutCell {
    pub cell_type: LayoutType,
    pub sx: u16, pub sy: u16,
    pub xoff: u16, pub yoff: u16,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub pane_id: Option<PaneId>,
}

#[derive(Debug, Clone)]
pub struct LayoutTree {
    pub cells: Vec<LayoutCell>,
    pub root: usize,
}

/// Checksum matching layout-custom.c:46-57
pub fn layout_checksum(data: &[u8]) -> u16 {
    let mut csum: u16 = 0;
    for &byte in data {
        csum = (csum >> 1) | (csum << 15);
        csum = csum.wrapping_add(byte as u16);
    }
    csum
}

/// Validation matching layout-custom.c:119-153
pub fn layout_check(tree: &LayoutTree) -> bool {
    fn check_cell(tree: &LayoutTree, idx: usize) -> bool {
        let cell = &tree.cells[idx];
        match cell.cell_type {
            LayoutType::Leaf => true,
            LayoutType::LeftRight => {
                let mut total_w: u16 = 0;
                for (i, &child_idx) in cell.children.iter().enumerate() {
                    let child = &tree.cells[child_idx];
                    if child.sy != cell.sy { return false; }
                    if i > 0 { total_w += 1; } // border
                    total_w += child.sx;
                    if !check_cell(tree, child_idx) { return false; }
                }
                total_w == cell.sx
            }
            LayoutType::TopBottom => {
                let mut total_h: u16 = 0;
                for (i, &child_idx) in cell.children.iter().enumerate() {
                    let child = &tree.cells[child_idx];
                    if child.sx != cell.sx { return false; }
                    if i > 0 { total_h += 1; } // border
                    total_h += child.sy;
                    if !check_cell(tree, child_idx) { return false; }
                }
                total_h == cell.sy
            }
        }
    }
    check_cell(tree, tree.root)
}

/// Round-robin resize matching layout.c:448-462
pub fn layout_resize_adjust(tree: &mut LayoutTree, parent_idx: usize, delta: i32) {
    let children = tree.cells[parent_idx].children.clone();
    if children.is_empty() { return; }
    let is_lr = tree.cells[parent_idx].cell_type == LayoutType::LeftRight;
    let mut remaining = delta.unsigned_abs() as u16;
    let mut i = 0;
    while remaining > 0 {
        let child_idx = children[i % children.len()];
        let current = if is_lr {
            tree.cells[child_idx].sx
        } else {
            tree.cells[child_idx].sy
        };
        if delta > 0 {
            if is_lr { tree.cells[child_idx].sx += 1; }
            else { tree.cells[child_idx].sy += 1; }
            remaining -= 1;
        } else if current > 1 {
            if is_lr { tree.cells[child_idx].sx -= 1; }
            else { tree.cells[child_idx].sy -= 1; }
            remaining -= 1;
        }
        i += 1;
        if i >= children.len() * 2 && remaining == delta.unsigned_abs() as u16 {
            break; // safety valve
        }
    }
    recompute_offsets(tree, parent_idx);
}

pub const PANE_MINIMUM: u16 = 1;

pub fn can_split(cell: &LayoutCell, direction: LayoutType) -> bool {
    let available = match direction {
        LayoutType::LeftRight => cell.sx,
        LayoutType::TopBottom => cell.sy,
        LayoutType::Leaf => return false,
    };
    available >= PANE_MINIMUM * 2 + 1
}
```

### Test Strategy

1. Checksum parity: verify against real tmux for 10+ layout strings.
2. Roundtrip: parse layout string -> tree -> serialize -> identical string.
3. Validation pass/fail: correct tree passes, bad tree fails `layout_check()`.
4. Resize round-robin: 3 panes at 100 cols, resize to 103, verify +1,+1,+1.
5. Resize clamp: shrink below PANE_MINIMUM never produces zero-size pane.
6. Split minimum: `can_split` rejects when dimension < PANE_MINIMUM * 2 + 1.
7. Property test: random dimensions always produce valid layout.
8. Resize never panics: `layout_resize_adjust()` survives arbitrary delta.

### AGENTS.md Rules

- `RULE-S11-01`: Never switch to proportional resize without parity proof.
- `RULE-S11-02`: Layout changes require before/after geometry snapshots in tests.

---

## 12. ORM-like Query API

### Design Decisions

- `QueryList<T>` mirrors libtmux semantics exactly: kwargs filtering, callable matcher, nested `__` lookup, `get(default=...)`, and all 12 operators plus `eq` alias.
- Verified against libtmux `LOOKUP_NAME_MAP` at `query_list.py:298-312`.
- Rust extensions beyond libtmux: `Gt`, `Gte`, `Lt`, `Lte`, `IsNone`, `IsSome`.
- The `__` separator serves dual purpose: field path traversal AND operator suffix, matching libtmux's `keygetter` + `filter()` behavior.
- `QueryBuilder` provides a fluent Rust-native API alongside kwargs parsing for bindings.

### Rust Example

```rust
// crates/mux-query/src/op.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryOp {
    Exact,       // "exact" and "eq" (alias per libtmux LOOKUP_NAME_MAP)
    IExact,      // "iexact"
    Contains,    // "contains"
    IContains,   // "icontains"
    StartsWith,  // "startswith"
    IStartsWith, // "istartswith"
    EndsWith,    // "endswith"
    IEndsWith,   // "iendswith"
    In,          // "in"
    Nin,         // "nin"
    Regex,       // "regex"
    IRegex,      // "iregex"
    // Extensions beyond libtmux (reasonable for Rust callers):
    Gt, Gte, Lt, Lte,
    IsNone, IsSome,
}

impl QueryOp {
    pub fn parse_lookup(name: &str) -> Option<Self> {
        match name {
            "exact" | "eq" => Some(Self::Exact),
            "iexact" => Some(Self::IExact),
            "contains" => Some(Self::Contains),
            "icontains" => Some(Self::IContains),
            "startswith" => Some(Self::StartsWith),
            "istartswith" => Some(Self::IStartsWith),
            "endswith" => Some(Self::EndsWith),
            "iendswith" => Some(Self::IEndsWith),
            "in" => Some(Self::In),
            "nin" => Some(Self::Nin),
            "regex" => Some(Self::Regex),
            "iregex" => Some(Self::IRegex),
            "gt" => Some(Self::Gt),
            "gte" => Some(Self::Gte),
            "lt" => Some(Self::Lt),
            "lte" => Some(Self::Lte),
            "is_none" => Some(Self::IsNone),
            "is_some" => Some(Self::IsSome),
            _ => None,
        }
    }
}

// crates/mux-query/src/value.rs
#[derive(Debug, Clone, PartialEq)]
pub enum QueryValue {
    String(String),
    Int(i64),
    Bool(bool),
    List(Vec<QueryValue>),
    None,
}

#[derive(Debug, Clone)]
pub struct QuerySpec {
    pub field: String,
    pub op: QueryOp,
    pub value: QueryValue,
}

/// Parse "field__subfield__op" into (field_path, operator).
pub fn parse_kwargs_key(key: &str) -> (String, QueryOp) {
    if let Some((prefix, suffix)) = key.rsplit_once("__") {
        if let Some(op) = QueryOp::parse_lookup(suffix) {
            return (prefix.to_string(), op);
        }
    }
    (key.to_string(), QueryOp::Exact)
}

/// Extract a field value from an entity, supporting nested "__" paths.
pub trait FieldAccess {
    fn get_field(&self, path: &str) -> Option<QueryValue>;
}

// crates/mux-query/src/query_list.rs
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct QueryList<T: FieldAccess + Clone> {
    items: Arc<Vec<T>>,
}

pub enum QueryInput<'a, T> {
    Kwargs(&'a [(String, QueryValue)]),
    Predicate(&'a dyn Fn(&T) -> bool),
}

#[derive(Debug, thiserror::Error)]
pub enum QueryError {
    #[error("no object found matching the query")]
    ObjectDoesNotExist,
    #[error("multiple objects returned for single-object query")]
    MultipleObjectsReturned,
}

impl<T: FieldAccess + Clone> QueryList<T> {
    pub fn new(items: Vec<T>) -> Self {
        Self { items: Arc::new(items) }
    }

    pub fn filter(&self, input: QueryInput<'_, T>) -> QueryList<T> {
        let filtered: Vec<T> = match input {
            QueryInput::Kwargs(specs) => {
                self.items.iter().filter(|item| {
                    specs.iter().all(|(key, val)| {
                        let (field, op) = parse_kwargs_key(key);
                        match item.get_field(&field) {
                            Some(field_val) => matches_one(&field_val, op, val),
                            None => false,
                        }
                    })
                }).cloned().collect()
            }
            QueryInput::Predicate(pred) => {
                self.items.iter().filter(|item| pred(item)).cloned().collect()
            }
        };
        QueryList::new(filtered)
    }

    pub fn get(&self, input: QueryInput<'_, T>) -> Result<&T, QueryError> {
        let filtered = self.filter(input);
        match filtered.len() {
            0 => Err(QueryError::ObjectDoesNotExist),
            1 => Ok(&filtered.items[0]),
            _ => Err(QueryError::MultipleObjectsReturned),
        }
    }

    pub fn len(&self) -> usize { self.items.len() }
    pub fn is_empty(&self) -> bool { self.items.is_empty() }
    pub fn iter(&self) -> impl Iterator<Item = &T> { self.items.iter() }
}
```

### Test Strategy

1. Exact match: `filter(name="work")` returns exactly matching items.
2. IExact: `filter(name__iexact="WORK")` matches case-insensitively.
3. StartsWith: `filter(name__startswith="wo")` matches "work", "workflow".
4. Nin: `filter(name__nin=["alpha", "beta"])` excludes listed items.
5. IRegex: `filter(name__iregex="^work")` matches case-insensitively.
6. Callable matcher: `filter(|s| s.name.starts_with("wo"))` works.
7. get default: `get(name="missing", default=None)` returns None instead of error.
8. get multiple error: `get(name__startswith="w")` with 2+ matches returns MultipleObjectsReturned.
9. get not found error: `get(name="missing")` returns ObjectDoesNotExist.
10. eq alias: `parse_lookup("eq")` returns `Some(QueryOp::Exact)`.
11. QueryBuilder parity: Builder API and kwargs produce identical results.
12. Nested lookup: `food__fruit__in` pattern traverses fields via `__` separator.

### AGENTS.md Rules

- `RULE-S12-01`: `QueryList` APIs used in examples must exist in bindings.
- `RULE-S12-02`: No placeholder operators; all documented ops must be implemented.

---

## 13. Runtime Architecture

### Design Decisions

- Single state actor owns mutable `ServerGraph` in a dedicated `tokio` task.
- PTY/client tasks feed events into state actor via `mpsc` channel.
- Effect dispatcher executes side effects and feeds failures back as `Event::EffectFailed`.
- Snapshot publisher provides lock-free read handles via `ArcSwap`.

### Rust Example

```rust
// crates/mux-server/src/state_actor.rs
pub struct StateActor {
    graph: ServerGraph,
    event_rx: mpsc::Receiver<Event>,
    effect_tx: mpsc::Sender<Effect>,
    snapshot: arc_swap::ArcSwap<GraphState>,
}

impl StateActor {
    pub async fn run(&mut self) {
        while let Some(event) = self.event_rx.recv().await {
            let ctx = CoreCtx {
                now_ms: chrono::Utc::now().timestamp_millis(),
                rand_u64: rand::random(),
            };
            let effects = apply_event(&mut self.graph, event, &ctx);
            for effect in effects {
                if matches!(effect, Effect::PublishSnapshot) {
                    let snapshot = self.graph.graph_state();
                    self.snapshot.store(Arc::new(snapshot));
                } else {
                    let _ = self.effect_tx.send(effect).await;
                }
            }
        }
    }
}

// crates/mux-server/src/effect_dispatcher.rs
pub struct EffectDispatcher {
    effect_rx: mpsc::Receiver<Effect>,
    event_tx: mpsc::Sender<Event>,
    pty_backend: Box<dyn PtyBackend>,
}

impl EffectDispatcher {
    pub async fn run(&mut self) {
        while let Some(effect) = self.effect_rx.recv().await {
            if let Err(e) = self.dispatch(effect.clone()).await {
                let _ = self.event_tx.send(Event::EffectFailed {
                    effect_id: 0,
                    error: e.to_string(),
                    class: e.class(),
                }).await;
            }
        }
    }
}

// crates/mux-api/src/state_handle.rs
pub struct StateHandle {
    inner: Arc<ArcSwap<GraphState>>,
}

impl StateHandle {
    pub fn load(&self) -> Arc<GraphState> {
        self.inner.load_full()
    }
}
```

### Test Strategy

1. Event -> Effect roundtrip: send CreateSession, verify PublishSnapshot effect.
2. Snapshot published: after state change, `StateHandle::load()` returns updated state.
3. Effect dispatch: SpawnPane effect calls `PtyBackend::spawn`.
4. Effect failure feedback: failed SpawnPane produces `Event::EffectFailed`.
5. Concurrent reads: multiple threads read StateHandle without blocking.
6. Graceful shutdown tests across all runtime tasks.

### AGENTS.md Rules

- `RULE-S13-01`: Only the state actor mutates graph state.
- `RULE-S13-02`: Runtime tasks communicate via events/effects only.

---

## 14. Server Lifecycle

### Design Decisions

- Use `flock` lock file semantics (tmux parity, `client.c:77-101`).
- Load config after first client identify completes (settled decision S7, `server-client.c:3725-3734`).
- Support graceful and forced shutdown paths with SIGTERM-then-SIGKILL escalation.
- Lock file guard via `Drop` impl, never manual unlock.

### Rust Example

```rust
// crates/mux-os/src/lock.rs
pub struct LockFile {
    file: std::fs::File,
}

impl LockFile {
    pub fn try_acquire(path: &std::path::Path) -> Result<Self, LockError> {
        let file = std::fs::OpenOptions::new()
            .create(true).truncate(false)
            .read(true).write(true)
            .open(path).map_err(LockError::Io)?;
        file.try_lock_exclusive()
            .map_err(|_| LockError::AlreadyLocked)?;
        Ok(Self { file })
    }
}

impl Drop for LockFile {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}
```

### 14.1 13-Step Startup Sequence

1. Parse command-line arguments.
2. Determine socket path (from `-S` flag or default).
3. Validate socket directory (symlink check, permissions check).
4. Acquire lock file via `flock(LOCK_EX|LOCK_NB)`.
5. Create server socket with `umask(0o177)`.
6. Initialize `ServerGraph` with compiled defaults.
7. Initialize OTEL tracing (if enabled).
8. Start state actor and effect dispatcher.
9. Accept first client connection.
10. Process identify burst (types 100-112).
11. Load config file (now that terminal capabilities are known).
12. Start PTY processes for initial session.
13. Enter main event loop.

### 14.2 Shutdown Sequence

1. Set shutdown flag.
2. Send `Effect::Shutdown` to all connected clients.
3. Kill all PTY processes (SIGHUP, then SIGTERM after 5s).
4. Flush OTEL providers (both `OTEL_PROVIDER` and `MUX_CLIENT_PROVIDER`).
5. Remove socket file.
6. Release lock file (automatic via `Drop`).

### Test Strategy

1. Lock exclusivity: second server start fails with AlreadyLocked.
2. Lock release on crash: `kill -9` server, second start succeeds.
3. Config load timing: trace startup, verify config loaded AFTER identify burst.
4. Socket permissions: created socket has mode 0600.
5. Shutdown cleanup: socket and lock files removed after graceful shutdown.

### AGENTS.md Rules

- `RULE-S14-01`: No PID-file lock logic for startup ownership.
- `RULE-S14-02`: Config load trigger must remain post-identify.

---

## 15. Control Mode

### Design Decisions

- Control notifications are hints, never authoritative. Binary protocol is the authority.
- Parse control lines into typed `ControlNotification` variants with generic fallback.
- Enforce per-client backpressure limit of 16 MB (`control.c:450-461`).
- Dropped notifications corrected by periodic snapshot refresh.

### Rust Example

```rust
// crates/mux-control/src/notification.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlNotification {
    SessionsChanged,
    SessionChanged { session_id: u32, name: String },
    SessionRenamed { session_id: u32, name: String },
    WindowAdd { window_id: u32 },
    WindowClose { window_id: u32 },
    WindowRenamed { window_id: u32, name: String },
    PaneAdd { pane_id: u32 },
    PaneModeChanged { pane_id: u32 },
    Output { pane_id: u32, data: Vec<u8> },
    ExtendedOutput { pane_id: u32, age: u64, data: Vec<u8> },
    LayoutChange { window_id: u32, layout: String },
    ClientSessionChanged { client_id: String, session_id: u32 },
    Exit { reason: Option<String> },
}

pub fn parse_notification(line: &str) -> Result<ControlNotification, ControlParseError> {
    let line = line.trim_end_matches('\n');
    if let Some(_rest) = line.strip_prefix("%sessions-changed") {
        return Ok(ControlNotification::SessionsChanged);
    }
    if let Some(rest) = line.strip_prefix("%session-changed ") {
        let (id, name) = parse_session_changed(rest)?;
        return Ok(ControlNotification::SessionChanged { session_id: id, name });
    }
    if let Some(rest) = line.strip_prefix("%window-add ") {
        let id = parse_window_id(rest)?;
        return Ok(ControlNotification::WindowAdd { window_id: id });
    }
    if let Some(rest) = line.strip_prefix("%output ") {
        let (pane_id, data) = parse_output(rest)?;
        return Ok(ControlNotification::Output { pane_id, data });
    }
    if let Some(rest) = line.strip_prefix("%exit") {
        let reason = if rest.is_empty() { None }
            else { Some(rest.trim_start().to_string()) };
        return Ok(ControlNotification::Exit { reason });
    }
    Err(ControlParseError::UnknownNotification(line.to_string()))
}

pub struct ControlState {
    pub pending_bytes: usize,
    pub max_pending: usize, // 16 * 1024 * 1024
}

impl ControlState {
    pub fn can_accept(&self, data_len: usize) -> bool {
        self.pending_bytes + data_len <= self.max_pending
    }
}
```

### Test Strategy

1. Parse all notification types: verify each `%` prefix notification parses correctly.
2. Extended output format: verify `%extended-output` with age field.
3. Unknown notification: graceful error, not panic.
4. Backpressure: exceed 16 MB limit, verify client disconnected.
5. `%begin/%end/%error` block framing tests.
6. Periodic refresh: dropped notification corrected by next refresh cycle.
7. UTF-8 validation: invalid UTF-8 in control lines handled cleanly.

### AGENTS.md Rules

- `RULE-S15-01`: Unknown notifications must not crash parser.
- `RULE-S15-02`: Control mode cannot bypass normal state reconciliation.

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
- **[v10] PyTermlet and JsTermlet**: language-native Termlet handles that wrap `mux_termlet::Termlet`. These are the primary SDK entry points for testing.

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
    inner: mux_termlet::Termlet,
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
        };
        let termlet = mux_termlet::Termlet::spawn(command, config)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        Ok(Self { inner: termlet })
    }

    /// Send keystrokes to the Termlet.
    fn send_keys(&mut self, keys: &str) -> PyResult<()> {
        self.inner.send_keys(keys)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Wait for a pattern to appear in the grid output.
    #[pyo3(signature = (pattern, timeout_ms=5000))]
    fn wait_for(&mut self, pattern: &str, timeout_ms: u64) -> PyResult<bool> {
        self.inner.wait_for(pattern, std::time::Duration::from_millis(timeout_ms))
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Capture a text snapshot of the current grid state.
    fn snapshot(&self) -> PyResult<String> {
        Ok(self.inner.snapshot().to_text())
    }

    /// Resize the Termlet grid.
    fn resize(&mut self, cols: u16, rows: u16) -> PyResult<()> {
        self.inner.resize(cols, rows)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Kill the underlying process.
    fn kill(&mut self) -> PyResult<()> {
        self.inner.kill()
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Check if the underlying process is still alive.
    #[getter]
    fn is_alive(&self) -> bool {
        self.inner.is_alive()
    }

    /// Get the current grid dimensions.
    #[getter]
    fn size(&self) -> (u16, u16) {
        let s = self.inner.size();
        (s.sx, s.sy)
    }

    /// Context manager entry.
    fn __enter__(slf: Py<Self>) -> Py<Self> { slf }

    /// Context manager exit -- kills the Termlet.
    fn __exit__(
        &mut self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_val: Option<&Bound<'_, PyAny>>,
        _exc_tb: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<bool> {
        let _ = self.kill();
        Ok(false)
    }

    fn __repr__(&self) -> String {
        let (cols, rows) = self.size();
        format!("Termlet({}x{}, alive={})", cols, rows, self.is_alive())
    }
}

// bindings/node/src/lib.rs -- Neon bindings
use neon::prelude::*;

pub struct JsServer { inner: ManagedMux }
impl Finalize for JsServer {}

fn server_new(mut cx: FunctionContext) -> JsResult<JsBox<JsServer>> {
    let managed = ManagedMux::start(MuxOptions::default())
        .or_else(|e| cx.throw_error(e.to_string()))?;
    Ok(cx.boxed(JsServer { inner: managed }))
}

fn async_new_session(mut cx: FunctionContext) -> JsResult<JsPromise> {
    let server = cx.argument::<JsBox<JsServer>>(0)?;
    let name = cx.argument::<JsString>(1)?.value(&mut cx);
    let channel = cx.channel();
    let inner = server.inner.clone();
    let (deferred, promise) = cx.promise();
    std::thread::spawn(move || {
        let result = inner.submit(Event::CreateSession {
            name, cwd: None, created_at: 0,
        });
        deferred.settle_with(&channel, move |mut cx| {
            match result {
                Ok(()) => Ok(cx.undefined()),
                Err(e) => cx.throw_error(e.to_string()),
            }
        });
    });
    Ok(promise)
}

// [v10] bindings/node/src/js_termlet.rs
pub struct JsTermlet { inner: mux_termlet::Termlet }
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
    };
    let termlet = mux_termlet::Termlet::spawn(&command, config)
        .or_else(|e| cx.throw_error(e.to_string()))?;
    Ok(cx.boxed(JsTermlet { inner: termlet }))
}

fn termlet_send_keys(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let termlet = cx.argument::<JsBox<JsTermlet>>(0)?;
    let keys = cx.argument::<JsString>(1)?.value(&mut cx);
    termlet.inner.send_keys(&keys)
        .or_else(|e| cx.throw_error(e.to_string()))?;
    Ok(cx.undefined())
}

fn termlet_snapshot(mut cx: FunctionContext) -> JsResult<JsString> {
    let termlet = cx.argument::<JsBox<JsTermlet>>(0)?;
    let text = termlet.inner.snapshot().to_text();
    Ok(cx.string(text))
}

fn termlet_kill(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let termlet = cx.argument::<JsBox<JsTermlet>>(0)?;
    termlet.inner.kill()
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
19. **[v10]** JsTermlet spawn: `termletSpawn("bash")` returns JsBox.
20. **[v10]** JsTermlet snapshot: `termletSnapshot(t)` returns string.

### AGENTS.md Rules

- `RULE-S16-01`: Bindings must mirror ORM API. No extra mutation surfaces.
- `RULE-S16-02`: Expose OTEL context propagation via traceparent parameter.
- `RULE-S16-03`: **[v10]** Termlet bindings must expose: spawn, send_keys, wait_for, snapshot, resize, kill.

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
5. No SUID/SGID bits on any binary.
6. umask restoration: after socket creation, umask is restored to original value.

### AGENTS.md Rules

- `RULE-S18-01`: All `unsafe` socket/fd code lives in `mux-os` only.
- `RULE-S18-02`: Socket path length checked against 108-byte limit before creation.

---

## 19. OpenTelemetry

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

// Force flush and shutdown -- both providers
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

---

## 20. tmux Version Management

### Design Decisions

Build and cache versioned tmux binaries from source for integration testing. Directly modeled on vibe-tmux `tmux-builder` crate. Key patterns:

- BLAKE3 hash for cache key computation with `flags_fingerprint`.
- File-based locking with `fs2::FileExt` and `Drop`-based `LockGuard`.
- Atomic build publication via `sibling_tmp_dir` + `std::fs::rename`.
- Binary validation via `tmux -V`.
- HOME-based fallback for repo clone destination.

### Rust Example

```rust
// tools/tmux-builder/src/cache.rs
use blake3;

fn flags_fingerprint(flags: &[String]) -> String {
    let mut data = Vec::new();
    for flag in flags {
        data.extend_from_slice(flag.as_bytes());
        data.push(0); // null separator between flags
    }
    let hash = blake3::hash(&data);
    hex_32(*hash.as_bytes())[..12].to_string()
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

### AGENTS.md Rules

- `RULE-S20-01`: Build cache keys must be deterministic and collision-free.
- `RULE-S20-02`: Atomic rename for build publication; never multi-step copy.

---

## 21. Test Support and Fake PTY

### Design Decisions

- Three-layer socket validation verified against vibe-tmux `path_guard.rs`.
- PathGuard with permission hardening (0700 on socket directory).
- FakePtyBackend with ScenarioRecorder for deterministic replay.
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

### AGENTS.md Rules

- `RULE-S21-01`: Tests must not operate on live user tmux sessions.
- `RULE-S21-02`: PathGuard three-layer validation required for all test fixtures.

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
    assert len(server.sessions.filter(session_name__icontains="ALPH")) == 1

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

def test_termlet_snapshot_testing(fake_termlet):
    """Snapshot test with deterministic FakePty backend."""
    # In fake mode, we inject output directly for deterministic testing
    fake_termlet.send_keys("echo hello world\n")
    snapshot = fake_termlet.snapshot()
    # Would use a snapshot testing library in practice
    assert isinstance(snapshot, str)

def test_termlet_lifecycle(termlet):
    """Verify spawn -> interact -> kill lifecycle."""
    assert termlet.is_alive
    termlet.send_keys("exit\n")
    import time
    time.sleep(0.5)
    # After exit, the shell process terminates

def test_termlet_context_manager():
    """Context manager auto-kills on exit."""
    from termforge import Termlet
    with Termlet("bash") as t:
        t.send_keys("echo managed\n")
        t.wait_for("managed")
    # t is killed here -- no leaked processes

def test_termlet_wait_for_timeout():
    """wait_for raises on timeout."""
    from termforge import Termlet
    with Termlet("bash") as t:
        try:
            t.wait_for("this_will_never_appear", timeout_ms=100)
            assert False, "should have raised"
        except RuntimeError:
            pass  # Expected timeout error
```

### Rust Example (Python OTEL)

```python
# bindings/python/tests/test_otel.py
import os

def test_otel_propagation(server, monkeypatch):
    monkeypatch.setenv("TRACEPARENT",
        "00-abcdef1234567890abcdef1234567890-1234567890abcdef-01")
    session = server.new_session("otel-test")
    assert session is not None
```

### Rust Example (Node.js)

```javascript
// bindings/node/tests/server.test.js
import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { Server, useTermlet } from 'termforge';

describe('Server', () => {
    let server;
    beforeEach(() => { server = new Server(); });
    afterEach(() => { server.kill(); });

    it('creates sessions', async () => {
        await server.newSession('work');
        const sessions = server.sessions();
        expect(sessions).toHaveLength(1);
        expect(sessions[0].name).toBe('work');
    });

    it('filters sessions', async () => {
        await server.newSession('alpha');
        await server.newSession('beta');
        const result = server.sessions({ name__startswith: 'a' });
        expect(result).toHaveLength(1);
    });
});

// [v10] bindings/node/tests/termlet.test.js
describe('Termlet', () => {
    it('spawns and captures output', async () => {
        const t = useTermlet('bash', { cols: 80, rows: 24 });
        try {
            t.sendKeys('echo hello\n');
            await t.waitFor('hello', 5000);
            const snapshot = t.snapshot();
            expect(snapshot).toContain('hello');
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
7. Callable filter: lambda-based filter works in Python.
8. **[v10]** pytest `termlet` fixture: spawns, yields, auto-kills.
9. **[v10]** pytest `fake_termlet` fixture: uses FakePtyBackend.
10. **[v10]** pytest `any_termlet` parametrize: same test, real and fake.
11. **[v10]** vitest `useTermlet()`: spawns, returns handle with kill cleanup.
12. **[v10]** Termlet send_keys + snapshot roundtrip: both languages.
13. **[v10]** Termlet wait_for timeout: raises appropriate error.
14. **[v10]** Termlet resize: dimensions change and grid reflows.
15. **[v10]** Termlet context manager: no leaked processes.

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
```

### Test Strategy

1. VT100 ASCII: plain text rendered correctly on grid.
2. VT100 CSI: cursor positioning, colors, attributes work.
3. VT100 scrolling: content scrolls correctly.
4. Grid snapshot: insta snapshots match expected output.
5. Format parity: 200+ format variables match tmux.
6. Key binding parity: all 4 key tables match tmux defaults.
7. Copy mode vi: word/line/selection movement matches.
8. Protocol proptest: random payloads roundtrip through codec.
9. Layout proptest: random resize never invalidates layout.
10. Fuzz: proto, VT100, config parsers survive random input.

### AGENTS.md Rules

- `RULE-S23-01`: Every parser must have a fuzz target.
- `RULE-S23-02`: Parity tests reference tmux source line numbers.

---

## 24. Performance Targets

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
                env: vec![],
                cwd: None,
                backend: mux_termlet::PtyMode::Fake,
            }).unwrap();
            termlet.kill().unwrap();
        });
    });
}

fn termlet_snapshot(c: &mut Criterion) {
    let mut termlet = mux_termlet::Termlet::spawn("echo", mux_termlet::TermletConfig {
        cols: 80, rows: 24,
        env: vec![],
        cwd: None,
        backend: mux_termlet::PtyMode::Fake,
    }).unwrap();
    // Feed some data into the grid
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

### AGENTS.md Rules

- `RULE-S24-01`: Performance baselines re-established after architecture changes.
- `RULE-S24-02`: Benchmarks must cover hot-path code (VT100, proto, layout).
- `RULE-S24-03`: **[v10]** Termlet spawn benchmark must stay under 500us for FakePty mode.

---

## 25. Visual Client / TUI

### Design Decisions

- ViewModel pattern: pure function `(GraphState, ClientId, TermSize) -> ViewModel`.
- ratatui-based rendering with crossterm terminal IO.
- ViewModel supports snapshot testing with `insta`.
- TUI can attach to both TermForge server (via StateHandle) and real tmux (via control mode).

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

### AGENTS.md Rules

- `RULE-S25-01`: TUI reads snapshots only, never mutates graph directly.
- `RULE-S25-02`: UI event handlers emit commands/events, not direct state mutations.

---

## 26. AGENTS.md Rules

### Design Decisions

This section consolidates all per-section rules into a global reference with numbered IDs and enforcement mechanisms. The rule naming convention follows `RULE-Snn-xx` pattern for traceability. **[v10]** Adds Termlet-specific rules.

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
| **[v10]** `std::thread::sleep` in `wait_for` | Busy-wait wastes CPU | Poll with backoff + condition |

### Test Strategy

1. Static rule checks in CI (grep/cargo-deny/compile-fail suites).
2. Rule coverage report links each rule to at least one automated test.
3. Release gate fails when critical rules are unverified.
4. Anti-pattern grep checks run on every PR.

### AGENTS.md Rules

- `RULE-S26-01`: Every architecture section must define enforceable rules.
- `RULE-S26-02`: Rule IDs are immutable once published.

---

## 27. Risks and Mitigations

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

### Test Strategy

1. Risk-to-test mapping must be complete and machine-checked.
2. High-impact risks require at least one integration/parity test.
3. Quarterly risk review updates with regression evidence.
4. **[v10]** Termlet-specific risks R31-R35 tested via dedicated Termlet test suite.

### AGENTS.md Rules

- `RULE-S27-01`: New high-risk features must add mitigation tests before merge.
- `RULE-S27-02`: Risk register IDs remain stable for auditability.

---

## 28. Plan Evolution and Changelog

### Design Decisions

This section documents the evolution from v4 through v10 Pass 1, providing traceability for architectural decisions.

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
| v9 Pass 1 (Claude) | 2026-02-11 | 4502 lines, deep code examples, reference-verified |
| v9 Pass 1 (GPT) | 2026-02-11 | 1392 lines, consistent section structure, RULE-Snn-xx |
| v9 Pass 1 (Gemini) | 2026-02-11 | 638 lines, acceptance criteria tables, WASM emphasis |
| v9 Pass 2 (Claude) | 2026-02-11 | 4212 lines, triple-model synthesis |
| v9 Pass 2 (GPT) | 2026-02-11 | 1151 lines, verified reference anchors |
| v9 Pass 2 (Gemini) | 2026-02-11 | 784 lines, cross-model verification |
| v9 Pass 3 Final | 2026-02-11 | Definitive synthesis, 4190 lines, 31 sections |
| **v10 Pass 1** | **2026-02-11** | **This document. Termlets integration. 32 sections.** |

### 28.2 v10 Pass 1 Changelog

**Major addition: Section 32 (Termlets)**
1. Full Termlet concept definition: SDK-first testing pods wrapping Pane + Grid + PtyBackend.
2. `mux-termlet` crate specification at Layer 2 FACADE.
3. `Termlet` struct with spawn/send_keys/wait_for/snapshot/resize/kill API.
4. `TermletConfig` with cols/rows/env/cwd/backend mode.
5. `TermletSnapshot` with `to_text()`, `to_styled()`, `to_cells()` extraction methods.
6. Dual-mode: `PtyMode::Real` for integration tests, `PtyMode::Fake` for deterministic unit tests.
7. `Drop` impl for automatic PTY cleanup.
8. OTEL span support for Termlet operations.

**Section updates for Termlet integration:**
1. **Section 2 (Acceptance Criteria)**: Added P7/P8/P9 (Termlet usability), B10/B11/B12 (Termlet performance), E6/E7 (Termlet ecosystem).
2. **Section 3 (Architecture)**: `mux-termlet` placed at Layer 2 FACADE. State ownership table updated.
3. **Section 4 (Workspace Layout)**: `mux-termlet` crate added. Purity table updated with dependency list.
4. **Section 5 (Layering)**: Layer 2 -> Layer 1 boundary rule added for Termlet PTY access.
5. **Section 8 (Errors)**: `TermletError` enum added with SpawnFailed, AlreadyKilled, WaitForTimeout, ResizeFailed variants.
6. **Section 16 (Bindings)**: `PyTermlet` and `JsTermlet` binding handles with full API.
7. **Section 19 (OTEL)**: Termlet span names added to naming convention table.
8. **Section 22 (Binding Tests)**: pytest `termlet`/`fake_termlet`/`any_termlet` fixtures. vitest `useTermlet()` helper.
9. **Section 24 (Performance)**: B10/B11/B12 benchmarks for Termlet spawn and snapshot.
10. **Section 26 (Rules)**: Rules 26-32 added for Termlet-specific concerns.
11. **Section 27 (Risks)**: R31-R35 added for Termlet-specific risks.
12. **Section 30 (Types)**: `Termlet`, `TermletConfig`, `TermletSnapshot`, `TermletError` added to canonical type reference.
13. **Section 31 (Test Matrix)**: Termlet test categories added, release gate rows M-32-01 through M-32-05 added.

### 28.3 Preserved from v9 Pass 3 Final

All 31 v9 sections preserved unchanged except where explicitly updated for Termlet integration. No v9 content was removed. The following is the v9 Pass 3 synthesis changelog, preserved for completeness:

- All 31 sections with full 4-part structure (Design Decisions, Rust Example, Test Strategy, AGENTS.md Rules).
- Deep reference-verified code examples as the implementation authority.
- Full `OtelProvider` implementation with dual providers, thread-local headers, composite propagator.
- BLAKE3 cache key computation with `flags_fingerprint`, `sanitize_token`, `hex_32`.
- 3-layer socket validation (`PathGuard`) with permission hardening.
- Complete `QueryList` with all 12 libtmux operators + `eq` alias + 6 Rust extensions.
- 25 numbered rules (v9), 30 risks (v9), 20 release gate matrix entries (v9).
- Tokio-codec `Decoder`/`Encoder` integration for `ImsgCodec`.
- `SubprocessTestServer` with SIGTERM-then-SIGKILL shutdown escalation.

### Rust Example

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecVersion { V4, V5, V6, V7, V8, V9Pass1, V9Pass2, V9Pass3Final, V10Pass1 }

pub struct ChangelogItem {
    pub from: SpecVersion,
    pub to: SpecVersion,
    pub summary: &'static str,
}

pub const V10_CHANGES: &[ChangelogItem] = &[
    ChangelogItem { from: SpecVersion::V9Pass3Final, to: SpecVersion::V10Pass1,
        summary: "Termlets: SDK-first testing pods. Section 32. Updates to S4/S16/S22/S31." },
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

All compatibility claims in this specification are anchored to verified source code. Anchors are organized by codebase and include specific file paths and line numbers where available. All anchors were re-verified during v10 Pass 1.

### 29.1 tmux Source Code References

| File | Line(s) | Topic | Verified v10 |
|---|---|---|---|
| `tmux-protocol.h` | 23 | Protocol version 8 | Yes |
| `client.c` | 77-101 | flock locking | Yes (line 89 confirmed) |
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

### 29.2 Reference Codebase Verification (Re-verified v10)

| Codebase | File | Claims Verified |
|---|---|---|
| libtmux | `_internal/query_list.py` | 12 operators in `LOOKUP_NAME_MAP` (lines 298-312), `eq` alias, `keygetter` nested traversal (lines 45-113), callable matcher in `filter()` (line 535), `get(default=no_arg)` semantics (lines 550-569), `MultipleObjectsReturned` / `ObjectDoesNotExist` exceptions |
| vibe-tmux | `mux-test-support/src/path_guard.rs` | 3-layer socket validation: `ensure_not_default_socket_name` (line 16), `ensure_socket_within_tempdir` (line 30), `ensure_socket_not_tmux_env` (line 45), `tmux_socket_path` format (line 76) |
| vibe-tmux | `tools/tmux-builder/src/lib.rs` | BLAKE3 `flags_fingerprint` with null separator (line 511), `compute_cache_key` format (line 521), `sanitize_token`, `LockGuard` with `Drop` (lines 247-256), `lock_cache_key` / `lock_repo_clone`, `sibling_tmp_dir` with pid+nanos (line 409), atomic `std::fs::rename` (line 389), `validate_tmux_binary` (line 549) |
| vibe-tmux | `crates/mux-otel/src/otel.rs` | Dual providers `OTEL_PROVIDER` / `MUX_CLIENT_PROVIDER` as `OnceLock<Mutex<Option<OtelProvider>>>` (lines 58-59), `OtelProvider` struct with logger+tracer_provider+tracer+runtime (lines 65-71), `enter_mux_client_span` lazy init (line 470), composite propagator Baggage+TraceContext (lines 771-777), `HeaderCarrier` Injector/Extractor (lines 998-1028), `force_flush` both providers (line 533), `shutdown` with `.take()` (line 576), `runtime.shutdown_timeout(200ms)` (line 275) |

### Rust Example

```rust
pub struct Anchor {
    pub claim: &'static str,
    pub source: &'static str,
    pub file: &'static str,
    pub lines: Option<&'static str>,
    pub v10_verified: bool,
}

pub const ANCHORS: &[Anchor] = &[
    Anchor { claim: "protocol version 8",
        source: "tmux", file: "tmux-protocol.h", lines: Some("23"),
        v10_verified: true },
    Anchor { claim: "flock startup lock",
        source: "tmux", file: "client.c", lines: Some("77-101"),
        v10_verified: true },
    Anchor { claim: "12 query operators + keygetter",
        source: "libtmux", file: "query_list.py", lines: Some("298-312"),
        v10_verified: true },
    Anchor { claim: "3-layer socket validation",
        source: "vibe-tmux", file: "path_guard.rs", lines: Some("16,30,45"),
        v10_verified: true },
    Anchor { claim: "BLAKE3 cache key + file locking",
        source: "vibe-tmux", file: "tmux-builder/src/lib.rs", lines: Some("511,521"),
        v10_verified: true },
    Anchor { claim: "dual OTEL providers",
        source: "vibe-tmux", file: "mux-otel/src/otel.rs", lines: Some("58-59"),
        v10_verified: true },
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

This appendix provides a consolidated reference for all canonical type names used across the specification. **[v10]** Termlet types added.

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

Test matrix extends section-local tests with release gates. Organized by test category, runner, and CI enforcement level. **[v10]** Termlet test categories added.

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
  test_termlet_send_keys_echo         # [v10]
  test_termlet_wait_for_timeout       # [v10]
  test_termlet_snapshot_text          # [v10]
  test_termlet_resize_reflow          # [v10]
  test_termlet_kill_idempotent        # [v10]
  test_termlet_drop_kills_pty         # [v10]
  test_binding_py_termlet_fixture     # [v10]
  test_binding_node_termlet_helper    # [v10]
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
];
```

### Test Strategy

1. CI requires 100% pass of mandatory matrix rows.
2. Nightly includes extended parity + fuzz durations.
3. Release requires zero unresolved mandatory rows.
4. **[v10]** Termlet gates M-32-01 through M-32-05 are mandatory for release.

### AGENTS.md Rules

- `RULE-S31-01`: New architecture features must add at least one matrix row.
- `RULE-S31-02`: Release tagging is blocked until mandatory rows are green.

---

## 32. Termlets [v10 new]

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

8. **Process management**: Spawn shells, run commands, send signals, detect exit. The Termlet owns the child process lifecycle and cleans up via `Drop`.

9. **The ultimate subprocess runner**: Where `std::process::Command` gives you stdout/stderr as byte streams, a Termlet gives you a full terminal emulation. You can test interactive programs, TUI apps, shell scripts with prompts, and anything that uses terminal escape sequences.

10. **Lite**: Minimal overhead. Creating a Termlet with FakePtyBackend takes < 500us. Creating one with a real PTY takes < 50ms. Destroying one is instant (kill signal + drop).

11. **Snapshot-testable**: `insta::assert_snapshot!(termlet.snapshot().to_text())` in Rust. `assert termlet.snapshot() == expected` in Python. `expect(termlet.snapshot()).toMatchSnapshot()` in vitest.

12. **Dual-mode**: `PtyMode::Real` for integration tests against real shells. `PtyMode::Fake` for deterministic unit tests with injected output.

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
}

impl Default for TermletConfig {
    fn default() -> Self {
        Self {
            cols: 80,
            rows: 24,
            env: vec![("TERM".to_string(), "xterm-256color".to_string())],
            cwd: None,
            backend: PtyMode::Real,
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
}

/// Counter for Termlet IDs.
static NEXT_TERMLET_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

impl Termlet {
    /// Spawn a new Termlet running the given command.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let mut t = Termlet::spawn("bash", TermletConfig::default())?;
    /// t.send_keys("echo hello\n")?;
    /// t.wait_for("hello", Duration::from_secs(5))?;
    /// let snap = t.snapshot();
    /// insta::assert_snapshot!(snap.to_text());
    /// t.kill()?;
    /// ```
    pub fn spawn(command: &str, config: TermletConfig) -> Result<Self, TermletError> {
        let id = NEXT_TERMLET_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let size = PaneSize { sx: config.cols, sy: config.rows };
        let grid = Grid::new(config.cols, config.rows);
        let parser = VtParser::new();

        // Create a synthetic PaneId for this Termlet.
        // We use a standalone SlotMap just for ID generation.
        let pane_id = PaneId::from(slotmap::KeyData::from_ffi(id));

        let mut backend: Box<dyn PtyBackend> = match config.backend {
            PtyMode::Real => {
                Box::new(RealPtyBackend::new()?)
            }
            PtyMode::Fake => {
                Box::new(FakePtyBackend::new())
            }
        };

        // Build command with environment
        let mut cmd_parts = vec![command.to_string()];
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
    /// Polls the PTY output with exponential backoff until the pattern
    /// is found in the grid text or the timeout is reached.
    pub fn wait_for(
        &mut self,
        pattern: &str,
        timeout: std::time::Duration,
    ) -> Result<bool, TermletError> {
        let start = std::time::Instant::now();
        let mut delay = std::time::Duration::from_millis(5);
        let max_delay = std::time::Duration::from_millis(100);

        loop {
            self.drain_output();
            let text = self.grid.to_text();
            if text.contains(pattern) {
                return Ok(true);
            }
            if start.elapsed() > timeout {
                return Err(TermletError::WaitForTimeout {
                    pattern: pattern.to_string(),
                    timeout_ms: timeout.as_millis() as u64,
                });
            }
            std::thread::sleep(delay);
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
            // Best-effort kill on drop to prevent PTY process leaks.
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
                // Emit ANSI codes for foreground/background/attributes
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
    }).unwrap();

    t.send_keys("echo hello world\n").unwrap();
    t.wait_for("hello world", Duration::from_secs(5)).unwrap();

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
    fake.inject_output(t.pane_id, b"$ echo test\r\ntest\r\n$ ".to_vec());

    let snap = t.snapshot();
    insta::assert_snapshot!("fake-echo", snap.to_text());
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

#### Rust: insta Snapshot Testing

```rust
#[test]
fn test_termlet_insta_snapshot() {
    let mut t = Termlet::spawn("bash", TermletConfig {
        cols: 80, rows: 24,
        backend: PtyMode::Fake,
        ..Default::default()
    }).unwrap();

    let fake = t.fake_backend_mut().unwrap();
    fake.inject_output(t.pane_id, b"\x1b[1;1H\x1b[2J$ ls\r\nfile1.txt  file2.txt  dir/\r\n$ ".to_vec());

    let snap = t.snapshot();
    insta::assert_snapshot!(snap.to_text());
}
```

#### Python: pytest Fixture

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
```

#### Node.js: vitest Helper

```javascript
// bindings/node/tests/termlet.test.js
import { describe, it, expect } from 'vitest';
import { useTermlet } from 'termforge';

describe('Termlet', () => {
    it('captures echo output', async () => {
        const t = useTermlet('bash', { cols: 80, rows: 24 });
        try {
            t.sendKeys('echo hello\n');
            await t.waitFor('hello', 5000);
            expect(t.snapshot()).toContain('hello');
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

#[test]
fn test_fake_pty_colored_output() {
    let mut t = Termlet::spawn("fake", TermletConfig {
        cols: 80, rows: 24,
        backend: PtyMode::Fake,
        ..Default::default()
    }).unwrap();

    let fake = t.fake_backend_mut().unwrap();
    // Inject colored output: red "ERROR" followed by normal text
    fake.inject_output(t.pane_id,
        b"\x1b[31mERROR\x1b[0m: something went wrong\r\n".to_vec());

    let snap = t.snapshot();
    // Text extraction strips colors
    assert!(snap.to_text().contains("ERROR: something went wrong"));

    // Cell-level assertion preserves color info
    let cells = snap.to_cells();
    // First row, first 5 cells should be red
    for i in 0..5 {
        assert_eq!(cells[0][i].fg, Color::Red);
    }
}
```

### 32.6 OTEL Integration (Feature-Gated)

```rust
// When the `otel` feature is enabled, Termlet operations emit spans:
#[cfg(feature = "otel")]
impl Termlet {
    fn spawn_instrumented(command: &str, config: TermletConfig) -> Result<Self, TermletError> {
        let _span = tracing::info_span!(
            "termforge.termlet.spawn",
            command = command,
            cols = config.cols,
            rows = config.rows,
            mode = ?config.backend,
        ).entered();
        Self::spawn(command, config)
    }
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

### Test Strategy

1. **Spawn lifecycle**: Termlet::spawn succeeds with both Real and Fake backends.
2. **Send keys echo**: send "echo hello\n", verify "hello" appears in snapshot.
3. **Wait for timeout**: wait_for("nonexistent", 100ms) returns WaitForTimeout error.
4. **Wait for success**: wait_for("hello", 5s) returns Ok(true) after send_keys.
5. **Snapshot text trimming**: trailing whitespace and empty lines stripped from to_text().
6. **Snapshot contains**: snapshot.contains("pattern") matches substring correctly.
7. **Snapshot row access**: snapshot.row(0) returns first row text.
8. **Resize dimensions**: after resize(120, 40), size() returns (120, 40).
9. **Resize SIGWINCH**: real PTY receives SIGWINCH after resize.
10. **Resize grid reflow**: content preserved after resize.
11. **Kill idempotent**: calling kill() twice does not error.
12. **Kill already exited**: killing after process exit is a no-op.
13. **Drop kills PTY**: dropping a Termlet without kill() sends SIGKILL.
14. **Fake inject output**: FakePtyBackend.inject_output -> drain_output -> grid updated.
15. **Fake deterministic**: same inject sequence always produces identical snapshot.
16. **Fake colored output**: injected ANSI color codes parsed correctly, cells have color info.
17. **ID uniqueness**: each Termlet gets a unique monotonic ID.
18. **Config default**: TermletConfig::default() produces 80x24 with xterm-256color.
19. **AlreadyKilled error**: send_keys/resize after kill returns AlreadyKilled.
20. **insta snapshot**: Termlet snapshot integrates with insta::assert_snapshot!.
21. **PyTermlet spawn**: Python `Termlet("bash")` creates and returns handle.
22. **PyTermlet context manager**: `with Termlet("bash") as t:` auto-kills.
23. **PyTermlet snapshot**: `termlet.snapshot()` returns string.
24. **PyTermlet fake mode**: `Termlet("bash", fake=True)` uses FakePtyBackend.
25. **JsTermlet spawn**: `useTermlet("bash")` creates and returns handle.
26. **JsTermlet snapshot**: `t.snapshot()` returns string.
27. **JsTermlet cleanup**: `t.kill()` sends SIGTERM.
28. **Performance B10**: FakePty spawn < 500us.
29. **Performance B11**: 80x24 snapshot < 50us.
30. **Performance B12**: Real PTY spawn < 50ms.

### AGENTS.md Rules

- `RULE-S32-01`: Termlet snapshot text must be trailing-whitespace-trimmed. No trailing spaces on any line, no trailing empty lines.
- `RULE-S32-02`: Termlet kill must be idempotent. Double-kill is not an error.
- `RULE-S32-03`: Termlet Drop must send SIGKILL to prevent PTY process leaks.
- `RULE-S32-04`: FakePtyBackend tests must not depend on timing. They must be fully deterministic.
- `RULE-S32-05`: Termlet bindings (Python/Node) must implement context manager / cleanup pattern.
- `RULE-S32-06`: `mux-termlet` must not depend on `mux-server`, `mux-api`, or `mux-orm`.
- `RULE-S32-07`: Every Termlet API method must be tested in at least Rust and one binding language.

---

## v10 Pass 1 Consistency Checklist

- [x] All 32 required sections are present and complete.
- [x] Every section includes: Design Decisions, Rust Example, Test Strategy, AGENTS.md Rules.
- [x] Section 32 (Termlets) has full depth: architecture, API, snapshot, usage examples, FakePty integration, OTEL, comparison table.
- [x] Section 4 (Workspace Layout) includes `mux-termlet` crate at Layer 2.
- [x] Section 16 (Language Bindings) includes `PyTermlet` and `JsTermlet` with full API.
- [x] Section 22 (Binding Test Frameworks) includes pytest `termlet`/`fake_termlet`/`any_termlet` fixtures and vitest `useTermlet()`.
- [x] Section 31 (Test Matrix) includes Termlet test categories (unit, integration, Python, Node, performance) and release gates M-32-01 through M-32-05.
- [x] Acceptance criteria P7/P8/P9 (Termlet usability) and B10/B11/B12 (Termlet performance) and E6/E7 (Termlet ecosystem) added to Section 2.
- [x] Error handling Section 8 includes `TermletError` with all variants and `ClassifiedError` impl.
- [x] Section 26 (Rules) includes rules 26-32 for Termlet-specific concerns.
- [x] Section 27 (Risks) includes R31-R35 for Termlet-specific risks.
- [x] Section 30 (Types) includes `Termlet`, `TermletConfig`, `TermletSnapshot`, `PtyMode`, `TermletError`.
- [x] Settled decision S16 (Termlets are simplified pane wrappers) added.
- [x] Gate enum in Section 2 includes P7/P8/P9/B10/B11 Termlet gates.
- [x] OTEL span naming convention in Section 19 includes termlet spans.
- [x] All [v10] annotations mark new content clearly.
- [x] No v9 content removed. All 31 v9 sections preserved.
- [x] Reference anchors re-verified against 4 codebases during v10.
- [x] Rule naming follows `RULE-Snn-xx` convention throughout.
- [x] Document writes only to `notes/` and is standalone.
- [x] Lineage: v9 Pass 3 Final -> v10 Pass 1 (this document).
- [x] Rust 2024 edition idioms used throughout.

---

*End of TermForge v10 Pass 1 Architecture Specification.*
