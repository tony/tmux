# TermForge v11 Architecture Specification -- Pass 2 Synthesis

Date: 2026-02-11
Status: **DEFINITIVE** -- v11 Pass 2, three-model cross-pollination synthesis.
Lineage: v4 -> v5 -> v6 -> v7 -> v8 -> v9 Pass 3 Final -> v10 Pass 3 Final -> v11 Pass 1 (3-model) -> **v11 Pass 2** (this document).
Models: Claude Opus 4.6 (base), GPT-5 Codex, Gemini (cross-pollinated).
License: MIT OR Apache-2.0
Rust edition: 2024 (MSRV 1.85)
Protocol target: tmux protocol v8

---

## Preamble

This document is the single authoritative architectural reference for TermForge, a Rust terminal multiplexer with 100% tmux wire-protocol compatibility, ORM-like API, language bindings, CRDT collaboration, a ratatui-based TUI client, and **Termlets** -- SDK-first testing pods that are the killer differentiator.

### v11 Pass 2 Cross-Pollination Notes

v11 Pass 2 synthesizes three independent v11 Pass 1 specifications (Claude 5,198 lines, GPT 2,229 lines, Gemini 4,135 lines) into a unified document. Each model brought unique strengths:

**From Claude (base):**
- Most comprehensive implementation examples with full function bodies
- `TermletPaneId` newtype with `AtomicU64` counter
- `ErrorCode` trait universalized across `CoreError`, `ProtocolError`, `QueryError`
- `GraphState` backed by `HashMap` for O(1) lookups (not linear scan)
- `ApplyOutcome.warnings` field for non-fatal issues
- `MAX_PAYLOAD_SIZE` guard in protocol decoder
- `output_history()` returning `&[u8]` reference
- `TermletExt` trait for custom assertion extensions
- `TermletPoolViewModel` for TUI multi-Termlet inspector
- `EventMsg` with optional oneshot reply channel
- `OpLog::compact_before()` for CRDT log compaction
- 62 Termlet test cases, 58 AGENTS.md rules, 43 risks

**From GPT (unique contributions merged):**
- **[v11 P2]** `TermletState` enum -- explicit lifecycle state machine (`Created -> Spawning -> Running -> Stopping -> Exited`) with monotonic transitions and API method gating
- **[v11 P2]** `INV-*` / `API-*` / `PAR-*` / `OPS-*` / `TST-*` traceability ID convention
- **[v11 P2]** `C6` gate for control-mode backpressure semantics parity
- **[v11 P2]** `A3` gate for cross-section traceability lint
- **[v11 P2]** Zero-timeout `wait_for` means single immediate check (not error)
- **[v11 P2]** `wait_max_interval` config field separate from `wait_poll_interval`
- **[v11 P2]** `ServerPhase` enum for server lifecycle phases
- **[v11 P2]** `BindingQueryList<T>` trait for language binding query wrappers
- **[v11 P2]** `CrossLangCase` struct for shared cross-language test fixtures
- **[v11 P2]** `normalize_output` / `ParityResult` for tmux parity testing
- **[v11 P2]** `PerfTarget` struct with `target_ns` / `hard_fail_ns`
- **[v11 P2]** `TermletError::InvalidRegex` as separate variant
- **[v11 P2]** `exit_status` tracking on `Termlet` struct
- **[v11 P2]** `to_cells()` for style-aware snapshot assertions
- **[v11 P2]** Remaining-time clamping in `wait_for`: `sleep_for.min(remaining)`

**From Gemini (unique contributions merged):**
- **[v11 P2]** `inherit_env` field on `TermletConfig` (default `false` for hermetic, opt-in for integration tests)
- **[v11 P2]** `SnapshotDiff::compare` with `context_lines` parameter for better debug output
- **[v11 P2]** Async pool methods (`spawn_async`, `get_async`) feature-gated under `async`
- **[v11 P2]** Resize drains output before AND after backend resize to capture SIGWINCH response
- **[v11 P2]** Section 32.15 "Implementation Hints for LLMs" -- guidance for LLM coding agents
- **[v11 P2]** `to_styled()` method on `TermletSnapshot` for ANSI-colored output
- **[v11 P2]** `RULE-S32-15` for non-blocking IO requirement on `RealPtyBackend`

**Conflict resolutions:**
1. **GraphState storage**: Claude's `HashMap` chosen over Gemini's `Vec<(Id, T)>` -- O(1) vs O(n) lookup.
2. **TermletError variants**: Merged to 8 variants (Claude's 7 + GPT's `InvalidRegex`); added `NotInPool` from Claude.
3. **PtyBackend bounds**: Claude's `Send + Any` with `as_any_mut()` required; Gemini's `Send`-only rejected as insufficient for `fake_backend_mut()`.
4. **Termlet lifecycle**: GPT's explicit `TermletState` enum adopted (stronger than Claude's boolean `alive`/Gemini's boolean `exited`).
5. **Termlet::spawn signature**: Claude's ergonomic `spawn(command, config)` retained with GPT's DI pattern available via `spawn_with_backend()`.
6. **TermletSnapshot internal storage**: GPT's `Vec<String>` lines adopted for efficient line-level operations and `to_cells()` support.
7. **wait_for remaining time**: GPT's remaining-time clamping `sleep_for.min(remaining)` adopted -- prevents oversleep past deadline.
8. **SnapshotDiff API**: Gemini's `context_lines` parameter merged with GPT's `before_size`/`after_size` fields for complete diff context.

**Traceability convention (adopted from GPT):**
- `INV-*`: core invariants
- `API-*`: API contracts
- `PAR-*`: parity contracts against tmux/libtmux/reference crates
- `OPS-*`: runtime/operational contracts
- `TST-*`: mandatory test gates

**Settled global invariants:**
- `INV-001`: Protocol compatibility target is tmux protocol v8 (`tmux-protocol.h:23`).
- `INV-002`: `mux-core` is deterministic and IO-free (`#![forbid(unsafe_code)]`).
- `INV-003`: Single writer for state mutation; snapshot-based readers.
- `INV-004`: Protocol violation drops client connection, never partial-continue (`server-client.c:3472-3475`).
- `INV-005`: First-client identify completes before config load path (`server-client.c:3725-3734`).
- `INV-006`: Layout resize uses round-robin one-cell adjustment (`layout.c:448-462`).
- `INV-007`: Socket/test isolation uses three guard layers (path guard pattern).
- `INV-008`: Termlets are pane-backed and never a parallel terminal stack.

**Reference codebases (re-verified for v11):**
- `~/work/python/libtmux/src/libtmux/_internal/query_list.py` -- 12 operators confirmed in `LOOKUP_NAME_MAP` (lines 298-312): `eq` (alias for exact), `exact`, `iexact`, `contains`, `icontains`, `startswith`, `istartswith`, `endswith`, `iendswith`, `in`, `nin`, `regex`, `iregex`. Callable matcher in `filter()` (line 535). `get(default=no_arg)` semantics (lines 550-569). `keygetter` nested `__` traversal (lines 45-113). Exceptions: `MultipleObjectsReturned`, `ObjectDoesNotExist`, `PKRequiredException`, `OpNotFound`.
- `~/work/rust/vibe-tmux/crates/mux-test-support/src/path_guard.rs` -- 3-layer socket validation confirmed: `ensure_not_default_socket_name` (line 16), `ensure_socket_within_tempdir` (line 30), `ensure_socket_not_tmux_env` (line 45). `tmux_socket_path` format (line 76): `${TMUX_TMPDIR}/tmux-<uid>/<name>`. Tests at lines 80-106.
- `~/work/rust/vibe-tmux/tools/tmux-builder/src/lib.rs` -- BLAKE3 `flags_fingerprint` confirmed (line 511): null-separated flag bytes hashed via `blake3::hash`, truncated to 12 hex chars at call site (`&cfg[..12]` at line 543). `compute_cache_key` format (line 521): `tmux-{version}__{host}__{os}-{os_ver}__cfg{cfg}__mk{mk}__tb{tool}`. `LockGuard` with `Drop` for `fs2::FileExt::unlock` (lines 247-256). `sibling_tmp_dir` with `pid+nanos` (line 409). Atomic `std::fs::rename` publication (line 389). `validate_tmux_binary` via `tmux -V` (line 549).
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
| S12 | `thiserror` for library errors | `anyhow` only in binaries and tests. |
| S13 | ArcSwap for snapshot reads | Not `Arc<Mutex<_>>` or channels. |
| S14 | Event -> Effect architecture | Events are inbound mutations; Effects are outbound side-effects. Single-writer actor. |
| S15 | mux-core is pure | No IO, no `unsafe`, no `tokio`, no `libc`. `#![forbid(unsafe_code)]`. |
| S16 | QueryList ORM layer | 18 operators (12 from libtmux + 6 extensions). Django/SQLAlchemy-inspired. |
| S17 | Synchronous API for `mux-termlet` | `Termlet::wait_for` uses `std::thread::sleep` with exponential backoff. No async runtime required. |
| S18 | `TermletBuilder` for fluent construction | Builder pattern as ergonomic alternative to `TermletConfig` struct. |
| S19 | **[v11]** `TermletPaneId` newtype for Termlet-internal pane tracking | Not `slotmap::KeyData::from_ffi()`. Clean separation from server-managed `PaneId`. |
| S20 | **[v11]** `PtyBackend` requires `as_any_mut()` for downcasting | Enables `fake_backend_mut()` without unsafe. Trait object must support `Any`. |
| S21 | **[v11 P2]** `TermletState` lifecycle enum | Monotonic state machine with API method gating. Not boolean `alive` flag. |
| S22 | **[v11 P2]** `inherit_env` opt-in for Termlets | Default hermetic (`false`). Integration tests can opt in to parent env. |
| S23 | **[v11 P2]** `wait_max_interval` separate from `wait_poll_interval` | Initial poll interval grows via exponential backoff up to max interval. |
| S24 | **[v11 P2]** Zero-timeout `wait_for` is single immediate check | Not an error. Drains output once, checks pattern, returns immediately. |
| S25 | **[v11 P2]** `exit_status` tracked on Termlet | Available after process exit for assertion on return codes. |

---

## 1. Project Identity

### Design Decisions

- Name: TermForge
- CLI binary: `termforge` (primary), `tf` (alias)
- Library name: `termforge` (Python/Node packages)
- Crate prefix: `mux-*` (all Rust crates)
- Config file: `~/.config/termforge/termforge.conf`
- Default socket dir: `$TMPDIR/termforge-$UID/`
- Env prefix: `TERMFORGE_*`
- Alternative names considered: MuxForge (too generic), RustMux (too literal), TermCore (too generic).
- The name must convey: terminal-specific, built/engineered, Rust-powered.
- Compatibility marketing: "100% tmux wire-protocol compatible."
- Traceability: `API-001`, `OPS-001`.

### Rust Example

```rust
// crates/mux-types/src/identity.rs
pub const PROJECT_NAME: &str = "TermForge";
pub const BINARY_NAME: &str = "termforge";
pub const BINARY_ALIAS: &str = "tf";
pub const SOCKET_PREFIX: &str = "termforge";

/// Default socket directory: $TMPDIR/termforge-$UID/
///
/// # Safety
/// Uses `libc::getuid()` which is safe on all Unix platforms but requires libc.
/// This function lives in `mux-os`, not `mux-core` (which is pure).
pub fn default_socket_dir() -> std::path::PathBuf {
    let uid = unsafe { libc::getuid() };
    std::env::temp_dir().join(format!("{SOCKET_PREFIX}-{uid}"))
}

/// Config path respecting XDG_CONFIG_HOME.
pub fn config_path() -> std::path::PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".config")))
        .unwrap_or_else(|| std::path::PathBuf::from(".config"));
    base.join("termforge").join("termforge.conf")
}
```

### Test Strategy

1. Binary name: `assert_eq!(BINARY_NAME, "termforge")` -- equality assertion.
2. Alias detection: binary invoked as `tf` behaves identically -- integration test via `argv[0]` check.
3. Socket dir: `default_socket_dir()` contains `termforge-` prefix and numeric UID -- regex assertion.
4. Config path: with `XDG_CONFIG_HOME=/custom`, `config_path()` starts with `/custom` -- path prefix assertion.
5. Config path default: without env var, falls back to `~/.config/termforge/termforge.conf` -- equality assertion.
6. `TST-001`: config path resolution with and without `XDG_CONFIG_HOME`.
7. `TST-002`: default socket root format includes uid.
8. `TST-003`: binary naming sanity checks in packaging tests.

### AGENTS.md Rules

- `RULE-S01-01`: All crate names must use the `mux-` prefix. **Enforcement:** CI grep of `Cargo.toml` names.
- `RULE-S01-02`: Binary name `termforge`, alias `tf`. **Enforcement:** `cargo build` output name check.

---

## 2. Acceptance Criteria and Gates [v11 P2 updated]

### Design Decisions

- Acceptance gates are the release checklist. Organized into categories: Compatibility (C), Purity (A), Performance (B), Programmability (P), Ecosystem (E).
- Every gate maps to at least one test in the CI matrix.
- **[v10]** Termlet usability gates (P7/P8/P9), performance gates (B10/B11/B12), and ecosystem gates (E6/E7) added.
- **[v10 P3]** Pass/fail thresholds with measurable criteria.
- **[v11]** Added P10 (TermletPool batch), P11 (SnapshotDiff regression), B13 (drain_output latency).
- **[v11 P2]** Added C6 (control-mode backpressure parity from GPT), A3 (cross-section traceability lint from GPT), P12 (TermletState lifecycle gating), P13 (exit_status tracking), B14 (Termlet spawn latency budget with PerfTarget).

### 2.1 Acceptance Gate Table

| ID | Category | Gate | Test / Evidence | Pass Threshold | Fail Threshold |
|---|---|---|---|---|---|
| C1 | Compatibility | `tmux attach` to TermForge server | Integration test | Attach succeeds, session list matches | Attach fails or session mismatch |
| C2 | Compatibility | `tmux list-sessions` on TermForge | Integration test | Correct session list | Missing or malformed sessions |
| C3 | Compatibility | tmux control mode subscribe/notify | Integration test | Notifications received for session/window/pane events | Missing notifications |
| C4 | Compatibility | tmux config file parsing | Unit test + fixtures | >= 95% of tmux options parsed correctly | < 90% coverage |
| C5 | Compatibility | Copy mode basic operations | Integration test | yank, paste, search work | Any basic operation fails |
| C6 | Compatibility | **[v11 P2]** Control-mode backpressure semantics | Integration test | Pause/resume notifications match tmux behavior | Backpressure divergence |
| A1 | Purity | `mux-core` compiles without `std::io` | `cargo check --target wasm32-unknown-unknown` | Compiles clean | Any IO import |
| A2 | Purity | `mux-core` has `#![forbid(unsafe_code)]` | CI attribute check | Attribute present | Missing attribute |
| A3 | Purity | **[v11 P2]** Cross-section traceability lint | CI | Every `INV-*` mapped to at least one `TST-*` | Unmapped invariant |
| B1 | Performance | VT100 ASCII throughput | Criterion benchmark | > 300 MB/s | < 250 MB/s |
| B2 | Performance | VT100 CSI throughput | Criterion benchmark | > 100 MB/s | < 80 MB/s |
| B3 | Performance | Proto decode throughput | Criterion benchmark | > 500 MB/s | < 400 MB/s |
| B4 | Performance | Layout resize 20 panes | Criterion benchmark | < 50 us | > 100 us |
| B5 | Performance | Snapshot 50 panes | Criterion benchmark | < 1 ms | > 2 ms |
| P1 | Programmability | Python `server.sessions.filter()` | pytest | Returns correct QueryList | Wrong results or exception |
| P2 | Programmability | Python `session.windows.get()` | pytest | Returns single window or raises | Silent failure |
| P3 | Programmability | Node `server.sessions()` | vitest | Returns filtered array | Wrong results or exception |
| P4 | Programmability | OTEL trace propagation | Integration test | Spans exported with correct parent IDs | Missing or broken span chain |
| P5 | Programmability | Control mode notification parsing | Unit test | All notification types parsed | Any type fails |
| P6 | Programmability | CRDT merge convergence | Property test | Replicas converge | Divergence detected |
| P7 | Programmability | Termlet spawn + send_keys + snapshot | Rust unit test | Snapshot contains sent text | Missing text or panic |
| P8 | Programmability | PyTermlet context manager | pytest | Auto-kills on exit, no leaked processes | Process leak or exception |
| P9 | Programmability | Termlet dual-mode (real + fake) | Parametrized test | Both modes produce consistent snapshots for same input | Divergence between modes |
| P10 | Programmability | **[v11]** TermletPool batch operations | Rust unit test | Pool spawn/kill_all/snapshot_all work | Any operation fails |
| P11 | Programmability | **[v11]** SnapshotDiff detects changes | Rust unit test | Changed lines identified correctly | False positive/negative |
| P12 | Programmability | **[v11 P2]** TermletState lifecycle gating | Rust unit test | API methods reject calls in wrong state | Allowed in wrong state |
| P13 | Programmability | **[v11 P2]** Termlet exit_status tracking | Rust unit test | Exit code captured after process exit | Missing or wrong code |
| B10 | Performance | Termlet spawn (FakePty) | Criterion | < 500 us | > 1 ms |
| B11 | Performance | Termlet snapshot (80x24) | Criterion | < 50 us | > 100 us |
| B12 | Performance | Termlet spawn (real PTY) | Criterion | < 50 ms | > 100 ms |
| B13 | Performance | **[v11]** drain_output 1KB buffer | Criterion | < 10 us | > 50 us |
| B14 | Performance | **[v11 P2]** PerfTarget enforcement | Criterion | All targets within `target_ns` | Any exceeds `hard_fail_ns` |
| E1 | Ecosystem | Python package installable via pip | CI | `pip install termforge` succeeds | Install fails |
| E2 | Ecosystem | Node package installable via npm | CI | `npm install termforge` succeeds | Install fails |
| E3 | Ecosystem | `cargo doc` builds without warnings | CI | Zero warnings | Any warning |
| E4 | Ecosystem | MSRV 1.85 compiles | CI | `cargo +1.85 check` passes | Compile failure |
| E5 | Ecosystem | Clippy clean on nightly | CI | Zero warnings | Any warning |
| E6 | Ecosystem | Termlet documented in README | Manual | Usage example present | No mention |
| E7 | Ecosystem | Termlet cross-language parity | Multi-runner | Same scenario, same snapshot across Rust/Python/Node | Snapshot divergence |

### Rust Example

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Gate {
    // Compatibility
    C1WireAttach,
    C2ListSessions,
    C3ControlNotifications,
    C4ConfigParse,
    C5CopyMode,
    C6ControlBackpressure,        // [v11 P2] from GPT
    // Purity
    A1NoPureIo,
    A2ForbidUnsafe,
    A3InvariantTraceability,      // [v11 P2] from GPT
    // Performance
    B1Vt100Ascii,
    B2Vt100Csi,
    B3ProtoDecode,
    B4LayoutResize,
    B5Snapshot50,
    // Programmability
    P1PyFilter,
    P2PyGet,
    P3NodeFilter,
    P4OtelTrace,
    P5ControlParse,
    P6CrdtMerge,
    P7TermletSpawn,
    P8PyTermletCtxMgr,
    P9TermletDualMode,
    P10TermletPool,
    P11SnapshotDiff,
    P12TermletStateGating,        // [v11 P2]
    P13TermletExitStatus,         // [v11 P2]
    // Performance (Termlet)
    B10TermletSpawnFake,
    B11TermletSnapshot,
    B12TermletSpawnReal,
    B13DrainOutputLatency,
    B14PerfTargetEnforcement,     // [v11 P2]
    // Ecosystem
    E1PyPip,
    E2NodeNpm,
    E3CargoDoc,
    E4Msrv,
    E5Clippy,
    E6TermletDocs,
    E7TermletCrossLang,
}

impl Gate {
    pub fn release_blocking(self) -> bool {
        !matches!(self,
            Gate::B1Vt100Ascii | Gate::B2Vt100Csi | Gate::B3ProtoDecode |
            Gate::B4LayoutResize | Gate::B5Snapshot50 |
            Gate::B10TermletSpawnFake | Gate::B11TermletSnapshot |
            Gate::B12TermletSpawnReal | Gate::B13DrainOutputLatency |
            Gate::B14PerfTargetEnforcement
        )
    }
}

/// [v11 P2] Performance target struct from GPT for CI enforcement.
#[derive(Debug, Clone)]
pub struct PerfTarget {
    pub name: &'static str,
    pub target_ns: u64,
    pub hard_fail_ns: u64,
}

pub const PERF_TARGETS: &[PerfTarget] = &[
    PerfTarget { name: "termlet_spawn_fake", target_ns: 500_000, hard_fail_ns: 1_000_000 },
    PerfTarget { name: "termlet_snapshot_80x24", target_ns: 50_000, hard_fail_ns: 100_000 },
    PerfTarget { name: "termlet_spawn_real", target_ns: 50_000_000, hard_fail_ns: 100_000_000 },
    PerfTarget { name: "drain_output_1kb", target_ns: 10_000, hard_fail_ns: 50_000 },
];
```

### Test Strategy

1. Gate checklist run on every release candidate -- CI gate.
2. Performance gates warn but do not block (except regressions > 30%) -- threshold enforcement.
3. All functional gates must pass -- blocking gate.
4. Termlet gates P7/P8/P9 and E6/E7 mandatory for release.
5. **[v11 P2]** `PerfTarget` structs used to enforce Criterion thresholds programmatically.
6. `TST-010`: gate inventory exists and is complete.
7. `TST-011`: CI fails on any blocking gate red.
8. `TST-012`: performance regression budget >30% marks failure.

### AGENTS.md Rules

- `RULE-S02-01`: Every new feature must map to at least one acceptance gate. **Enforcement:** PR template checklist.
- `RULE-S02-02`: No gate may be removed without spec amendment and approval. **Enforcement:** Append-only gate table.
- `RULE-S02-03`: Gate pass/fail thresholds must be measurable, not subjective. **Enforcement:** Acceptance criteria table.
- `RULE-S02-04`: **[v11 P2]** Every `INV-*` invariant must map to at least one `TST-*` test. **Enforcement:** CI traceability lint (A3 gate).

---

## 3. Crate Dependency Rules

### Design Decisions

- Strict layered architecture: Layer 0 (pure) -> Layer 1 (OS) -> Layer 2 (facade) -> Layer 3 (application).
- Layer 0 crates: `mux-types`, `mux-grid`, `mux-query`, `mux-crdt`, `mux-proto`, `mux-pty-fake`. Pure, no IO, WASM-compatible.
- Layer 1 crates: `mux-core`, `mux-conf`, `mux-keys`, `mux-format`, `mux-pty`, `mux-os`. May use `libc`, `tokio`, file IO.
- Layer 2 crates: `mux-runtime`, `mux-api`, `mux-orm`, `mux-termlet`, `mux-control`. Facades composing lower layers.
- Layer 3 crates: `bindings-python`, `bindings-node`, `termforge` (binary), `mux-view`. Applications.
- Forbidden edges: Layer N may not depend on Layer N+1. `mux-termlet` may NOT depend on `mux-server`, `mux-api`, `mux-orm`.
- Traceability: `INV-002`, `API-003`.

### Rust Example

```rust
/// Declarative dependency policy used by architecture tests.
#[derive(Debug)]
pub struct CrateLayer {
    pub name: &'static str,
    pub layer: u8,
}

pub const CRATE_LAYERS: &[CrateLayer] = &[
    // Layer 0: Pure
    CrateLayer { name: "mux-types", layer: 0 },
    CrateLayer { name: "mux-grid", layer: 0 },
    CrateLayer { name: "mux-proto", layer: 0 },
    CrateLayer { name: "mux-query", layer: 0 },
    CrateLayer { name: "mux-crdt", layer: 0 },
    CrateLayer { name: "mux-pty-fake", layer: 0 },
    // Layer 1: OS
    CrateLayer { name: "mux-core", layer: 1 },
    CrateLayer { name: "mux-conf", layer: 1 },
    CrateLayer { name: "mux-keys", layer: 1 },
    CrateLayer { name: "mux-format", layer: 1 },
    CrateLayer { name: "mux-pty", layer: 1 },
    CrateLayer { name: "mux-os", layer: 1 },
    // Layer 2: Facade
    CrateLayer { name: "mux-runtime", layer: 2 },
    CrateLayer { name: "mux-api", layer: 2 },
    CrateLayer { name: "mux-orm", layer: 2 },
    CrateLayer { name: "mux-termlet", layer: 2 },
    CrateLayer { name: "mux-control", layer: 2 },
    // Layer 3: Application
    CrateLayer { name: "bindings-python", layer: 3 },
    CrateLayer { name: "bindings-node", layer: 3 },
    CrateLayer { name: "mux-view", layer: 3 },
];

pub fn edge_allowed(from: u8, to: u8) -> bool {
    to <= from
}
```

### Test Strategy

1. Dependency graph linter checks no forbidden edges -- CI job.
2. `mux-core` transitive dependencies audited for IO crates -- `cargo tree` CI check.
3. `mux-termlet` dependency deny-list enforcement -- `cargo tree` CI check.
4. `TST-020`: dependency graph linter checks no forbidden edges.
5. `TST-021`: `mux-core` transitive dependencies audited for IO crates.
6. `TST-022`: `mux-termlet` dependency deny-list enforcement.

### AGENTS.md Rules

- `RULE-S03-01`: Layer violations are blocking CI errors. **Enforcement:** Dependency linter CI job.
- `RULE-S03-02`: New crates must declare their layer in the architecture table. **Enforcement:** PR review.
- `RULE-S03-03`: `mux-termlet` must not depend on `mux-server` or `mux-api`. **Enforcement:** `cargo tree` CI check.

---

## 4. Workspace Layout

### Design Decisions

- Cargo workspace at repo root.
- Crate directories under `crates/` for libraries, `tools/` for build tooling, `bindings/` for language bindings.
- Binary crate at top level.
- One Cargo.toml per crate; workspace inherits common metadata.

### Rust Example

```
termforge/
  Cargo.toml                    # workspace root
  crates/
    mux-types/                  # Layer 0: shared types, entity IDs
    mux-grid/                   # Layer 0: Grid, VtParser
    mux-proto/                  # Layer 0: wire protocol codec
    mux-query/                  # Layer 0: QueryList, QueryOp, operators
    mux-crdt/                   # Layer 0: HLC, LWW, OrSet, OpLog
    mux-pty-fake/               # Layer 0: FakePtyBackend, ScenarioStep
    mux-core/                   # Layer 1: ServerGraph, Event, Effect, Layout
    mux-conf/                   # Layer 1: config parser
    mux-keys/                   # Layer 1: key binding table
    mux-format/                 # Layer 1: format string expansion
    mux-pty/                    # Layer 1: PtyBackend trait, RealPtyBackend
    mux-os/                     # Layer 1: socket, flock, signals
    mux-runtime/                # Layer 2: tokio event loop, state actor
    mux-api/                    # Layer 2: StateHandle, ManagedMux
    mux-orm/                    # Layer 2: ORM traversal (Session.windows())
    mux-termlet/                # Layer 2: Termlet, TermletBuilder, TermletPool
    mux-control/                # Layer 2: control mode client
    mux-otel/                   # Layer 2: OTEL providers
    mux-server/                 # Layer 2: server binary logic
    mux-test-support/           # Layer 2: PathGuard, test fixtures
    mux-view/                   # Layer 3: ViewModel, TUI rendering
    mux-bench/                  # benchmarks
  bindings/
    python/                     # Layer 3: PyO3 bindings
    node/                       # Layer 3: Neon bindings
  tools/
    tmux-builder/               # tmux source builder + cache
    mux-vm/                     # tmux version manager
    mux-regress/                # parity test runner
    format-audit/               # format string parity checker
    tmux-command-audit/         # command coverage tracker
  src/
    main.rs                     # termforge binary entry point
  tests/
    integration/                # integration test suites
    parity/                     # tmux parity test fixtures
    cross-lang/                 # cross-language consistency tests
```

### Test Strategy

1. `cargo workspace` builds all crates without error -- CI job.
2. Workspace member count matches architecture table -- count check.
3. No orphan crates (outside workspace) -- CI grep.

### AGENTS.md Rules

- `RULE-S04-01`: Every new crate must be added to workspace members. **Enforcement:** CI workspace member check.
- `RULE-S04-02`: Crate README must state its layer and dependencies. **Enforcement:** PR review.
- `RULE-S04-03`: `mux-termlet` lives at Layer 2, depends on Layer 0 + Layer 1 only. **Enforcement:** `cargo deny check`.

---

## 5. mux-core: Pure Kernel

### Design Decisions

- `#![forbid(unsafe_code)]` -- no exceptions, ever.
- No IO, no `tokio`, no `libc`, no `std::io`. Compiles to WASM.
- Owns: `ServerGraph`, `Event`, `Effect`, `ApplyOutcome`, `CoreCtx`, `MuxKernel` trait.
- `ServerGraph` is the authoritative mutable state. Modifications only via `apply_event()`.
- `GraphState` is an immutable snapshot built from `ServerGraph`. **[v11]** Uses `HashMap` internally for O(1) entity lookups (not `Vec<(Id, T)>`).
- `ApplyOutcome` returns effects + hints. **[v11]** Also returns `warnings: Vec<String>` for non-fatal issues.
- `CoreCtx` injects clock + RNG for deterministic replay/testing.
- `MuxKernel` trait is the core dispatch interface for event processing.
- **[v11 P2]** `ServerPhase` enum from GPT adopted for explicit server lifecycle tracking: `Bootstrap -> SocketReady -> AwaitIdentify -> ConfigLoading -> Running -> ShuttingDown -> Stopped`.
- Traceability: `INV-002`, `INV-003`, `API-010`.

### Rust Example

```rust
// crates/mux-core/src/lib.rs
#![forbid(unsafe_code)]

use std::collections::HashMap;

slotmap::new_key_type! {
    pub struct SessionId;
    pub struct WindowId;
    pub struct PaneId;
    pub struct ClientId;
    pub struct JobId;
    pub struct BufferId;
}

/// [v11 P2] Server lifecycle phases from GPT.
/// Monotonic progression; each phase implies all prior phases completed.
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

pub struct CoreCtx {
    pub now: std::time::SystemTime,
    pub rng: Box<dyn FnMut() -> u64>,
}

pub enum Event {
    CreateSession { name: String },
    NewWindow { session: SessionId, name: String },
    SplitPane { window: WindowId, direction: Direction },
    KillPane { pane: PaneId },
    ResizeLayout { window: WindowId, cols: u16, rows: u16 },
    ClientIdentify { client: ClientId, features: u32 },
    // ... all tmux-compatible events
}

pub enum Effect {
    WritePty { pane: PaneId, data: Vec<u8> },
    SendMessage { client: ClientId, msg: Vec<u8> },
    SpawnProcess { pane: PaneId, cmd: Vec<String> },
    CloseClient { client: ClientId },
    // ... all side-effects
}

pub enum Direction { Horizontal, Vertical }

/// Outcome of apply_event: produced effects, hints for callers, and warnings.
pub struct ApplyOutcome {
    pub effects: Vec<Effect>,
    pub hints: Vec<String>,
    /// [v11] Non-fatal issues (e.g., missing entities referenced by event).
    pub warnings: Vec<String>,
}

/// Authoritative mutable state. Single-writer.
pub struct ServerGraph {
    pub sessions: slotmap::SlotMap<SessionId, Session>,
    pub windows: slotmap::SlotMap<WindowId, Window>,
    pub panes: slotmap::SlotMap<PaneId, Pane>,
    pub clients: slotmap::SlotMap<ClientId, Client>,
    pub phase: ServerPhase, // [v11 P2]
    // ...
}

/// Immutable snapshot. HashMap-backed for O(1) lookups.
pub struct GraphState {
    pub sessions: HashMap<SessionId, Session>,
    pub windows: HashMap<WindowId, Window>,
    pub panes: HashMap<PaneId, Pane>,
    pub clients: HashMap<ClientId, Client>,
    pub session_windows: HashMap<SessionId, Vec<WindowId>>,
    pub window_panes: HashMap<WindowId, Vec<PaneId>>,
    pub phase: ServerPhase, // [v11 P2]
}

pub trait MuxKernel {
    fn apply_event(&mut self, ctx: &mut CoreCtx, event: Event) -> ApplyOutcome;
    fn snapshot(&self) -> GraphState;
}

impl MuxKernel for ServerGraph {
    fn apply_event(&mut self, ctx: &mut CoreCtx, event: Event) -> ApplyOutcome {
        // Pure deterministic logic. No IO.
        todo!()
    }

    fn snapshot(&self) -> GraphState {
        // Build HashMap-backed snapshot from SlotMap data.
        todo!()
    }
}
```

### Test Strategy

1. Event -> Effect determinism: same events + same `CoreCtx` always produce same effects -- property test.
2. WASM compilation: `cargo check --target wasm32-unknown-unknown` passes -- CI gate.
3. `#![forbid(unsafe_code)]` present in lib.rs -- CI attribute check.
4. GraphState round-trip: snapshot faithfully represents ServerGraph -- property test.
5. **[v11 P2]** ServerPhase monotonic progression -- unit test that phase only moves forward.
6. **[v11]** ApplyOutcome.warnings populated for non-fatal issues -- unit test.
7. No `std::io` import transitively -- `cargo tree` check.

### AGENTS.md Rules

- `RULE-S05-01`: `mux-core` is pure: no IO, no `unsafe`, no `tokio`, no `libc`. **Enforcement:** `#![forbid(unsafe_code)]`, WASM CI check.
- `RULE-S05-02`: WASM CI check for all Layer 0 crates. **Enforcement:** CI: `cargo check --target wasm32-unknown-unknown`.
- `RULE-S05-03`: **[v11 P2]** `ServerPhase` transitions must be monotonic. **Enforcement:** Debug assertion in phase setter.

---

## 6. Entity ID Design

### Design Decisions

- All entity IDs use `slotmap::new_key_type!` for type safety and generational indexing.
- `SessionId`, `WindowId`, `PaneId`, `ClientId`, `JobId`, `BufferId` -- never raw integers.
- ID types are `Copy + Clone + Hash + Eq + Debug`.
- Entity traversal: `session.window_ids()`, not direct child storage.
- Reverse lookups computed in `GraphState`, not stored on entities.
- **[v11]** `TermletPaneId` is a separate newtype (not SlotMap) for Termlet-internal use.
- Traceability: `INV-003`, `API-020`.

### Rust Example

```rust
// crates/mux-types/src/ids.rs
slotmap::new_key_type! {
    pub struct SessionId;
    pub struct WindowId;
    pub struct PaneId;
    pub struct ClientId;
    pub struct JobId;
    pub struct BufferId;
}

/// [v11] Termlet uses distinct ID type, not SlotMap.
/// AtomicU64 counter prevents collision with server PaneId space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TermletPaneId(u64);

static TERMLET_ID_COUNTER: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(1);

impl TermletPaneId {
    pub fn next() -> Self {
        Self(TERMLET_ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
    }

    pub fn raw(self) -> u64 { self.0 }
}
```

### Test Strategy

1. SlotMap key type safety: `SessionId` and `WindowId` cannot be interchanged -- compile-fail test.
2. Generational safety: removed + reinserted entity does not alias old key -- unit test.
3. `TermletPaneId::next()` produces unique monotonic IDs -- concurrent unit test.
4. `TermletPaneId` distinct from `PaneId` -- type system check.

### AGENTS.md Rules

- `RULE-S06-01`: All entity IDs via `slotmap::new_key_type!`. **Enforcement:** Code review + type system.
- `RULE-S06-02`: **[v11]** `TermletPaneId` must not use `slotmap::KeyData::from_ffi()`. **Enforcement:** CI grep.

---

## 7. Event -> Effect Architecture

### Design Decisions

- Events are inbound state change requests. Effects are outbound side-effects.
- Single-writer: only the state actor calls `apply_event()`. All readers use `GraphState` snapshots via `ArcSwap`.
- The runtime executes effects after state transition. Core never executes effects.
- **[v11]** `EventMsg` wraps `Event` with optional oneshot reply channel for synchronous event submission.
- Traceability: `INV-003`, `INV-014`, `API-030`.

### Rust Example

```rust
// crates/mux-server/src/actor.rs
use arc_swap::ArcSwap;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};

/// [v11] Event message with optional reply channel for await-able submission.
pub struct EventMsg {
    pub event: Event,
    pub reply: Option<oneshot::Sender<ApplyOutcome>>,
}

pub struct StateActor {
    graph: ServerGraph,
    snapshot: Arc<ArcSwap<GraphState>>,
    rx: mpsc::Receiver<EventMsg>,
}

impl StateActor {
    pub async fn run(mut self) {
        let mut ctx = CoreCtx::production();
        while let Some(msg) = self.rx.recv().await {
            let outcome = self.graph.apply_event(&mut ctx, msg.event);

            // Publish new snapshot for all readers
            let snap = self.graph.snapshot();
            self.snapshot.store(Arc::new(snap));

            // Execute effects via runtime
            for effect in &outcome.effects {
                self.execute_effect(effect).await;
            }

            // Reply if caller is waiting
            if let Some(reply) = msg.reply {
                let _ = reply.send(outcome);
            }
        }
    }

    async fn execute_effect(&self, effect: &Effect) {
        // The runtime -- not the core -- performs IO.
        todo!()
    }
}
```

### Test Strategy

1. Round-trip: event submitted, effect executed, snapshot updated -- integration test.
2. Concurrent snapshot reads during event processing do not block -- concurrency test.
3. Reply channel receives outcome synchronously -- unit test.
4. Effects are only executed by runtime, never by core -- architecture test.

### AGENTS.md Rules

- `RULE-S07-01`: Core emits effects; only the runtime executes effects. **Enforcement:** Code review + typed `Effect` enum.
- `RULE-S07-02`: All state transitions go through `apply_event()`. **Enforcement:** CI grep for direct graph mutation.

---

## 8. Error Taxonomy [v11 P2 updated]

### Design Decisions

- Three error classes: `Bug` (internal logic error), `UserError` (invalid input/config), `Transient` (IO/network failure).
- Each crate defines its own typed error via `thiserror`. `anyhow` only in binaries and tests.
- **[v11]** `ErrorCode` trait for errors that cross binding boundaries: `fn error_code(&self) -> &'static str`. Implemented on `TermletError`, `CoreError`, `ProtocolError`, `QueryError`.
- **[v11 P2]** `TermletError` expanded to 8 variants (Claude's 7 + GPT's `InvalidRegex`).
- **[v11 P2]** Binding error messages include `[ERROR_CODE]` prefix for structured parsing by language-side exception handlers.
- Traceability: `API-040`, `INV-004`.

### Rust Example

```rust
// crates/mux-types/src/error.rs
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorClass {
    Bug,
    UserError,
    Transient,
}

/// [v11] Trait for errors crossing binding boundaries.
/// Error code strings must be stable across semver-minor versions.
pub trait ErrorCode {
    fn error_code(&self) -> &'static str;
    fn error_class(&self) -> ErrorClass;
}

// --- TermletError (8 variants, merged from all models) ---
#[derive(Debug, Error)]
pub enum TermletError {
    #[error("spawn failed: {reason}")]
    SpawnFailed { reason: String },

    #[error("termlet already exited")]
    AlreadyExited,

    #[error("wait_for timeout: pattern={pattern} timeout_ms={timeout_ms}")]
    WaitForTimeout { pattern: String, timeout_ms: u64 },

    #[error("pattern not found before process exit: {pattern}")]
    PatternNotFound { pattern: String },

    #[error("resize failed: {0}")]
    ResizeFailed(String),

    #[error("pty error: {0}")]
    Pty(String),

    /// [v11] Termlet not found in pool.
    #[error("termlet not in pool: {name}")]
    NotInPool { name: String },

    /// [v11 P2] Invalid regex pattern (from GPT). Separate from SpawnFailed for precise error handling.
    #[error("invalid regex: {0}")]
    InvalidRegex(String),
}

impl ErrorCode for TermletError {
    fn error_code(&self) -> &'static str {
        match self {
            Self::SpawnFailed { .. } => "TERMLET_SPAWN_ERROR",
            Self::AlreadyExited => "TERMLET_ALREADY_EXITED",
            Self::WaitForTimeout { .. } => "TERMLET_TIMEOUT",
            Self::PatternNotFound { .. } => "TERMLET_PATTERN_NOT_FOUND",
            Self::ResizeFailed(_) => "TERMLET_RESIZE_ERROR",
            Self::Pty(_) => "TERMLET_PTY_ERROR",
            Self::NotInPool { .. } => "TERMLET_NOT_IN_POOL",
            Self::InvalidRegex(_) => "TERMLET_INVALID_REGEX",
        }
    }

    fn error_class(&self) -> ErrorClass {
        match self {
            Self::SpawnFailed { .. } | Self::Pty(_) => ErrorClass::Transient,
            Self::AlreadyExited | Self::NotInPool { .. } |
            Self::InvalidRegex(_) | Self::ResizeFailed(_) => ErrorClass::UserError,
            Self::WaitForTimeout { .. } | Self::PatternNotFound { .. } => ErrorClass::Transient,
        }
    }
}

// [v11] ErrorCode for CoreError, ProtocolError, QueryError
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("invalid session: {0:?}")]
    InvalidSession(SessionId),
    #[error("invalid window: {0:?}")]
    InvalidWindow(WindowId),
    #[error("invalid pane: {0:?}")]
    InvalidPane(PaneId),
    #[error("state invariant violation: {0}")]
    InvariantViolation(String),
}

impl ErrorCode for CoreError {
    fn error_code(&self) -> &'static str {
        match self {
            Self::InvalidSession(_) => "CORE_INVALID_SESSION",
            Self::InvalidWindow(_) => "CORE_INVALID_WINDOW",
            Self::InvalidPane(_) => "CORE_INVALID_PANE",
            Self::InvariantViolation(_) => "CORE_INVARIANT_VIOLATION",
        }
    }
    fn error_class(&self) -> ErrorClass { ErrorClass::Bug }
}

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("invalid message type: {0}")]
    InvalidMessageType(u32),
    #[error("payload too large: {size} > {max}")]
    PayloadTooLarge { size: usize, max: usize },
    #[error("truncated frame")]
    TruncatedFrame,
}

impl ErrorCode for ProtocolError {
    fn error_code(&self) -> &'static str {
        match self {
            Self::InvalidMessageType(_) => "PROTO_INVALID_MSG_TYPE",
            Self::PayloadTooLarge { .. } => "PROTO_PAYLOAD_TOO_LARGE",
            Self::TruncatedFrame => "PROTO_TRUNCATED_FRAME",
        }
    }
    fn error_class(&self) -> ErrorClass { ErrorClass::Bug }
}

#[derive(Debug, Error)]
pub enum QueryError {
    #[error("unknown operator: {0}")]
    UnknownOperator(String),
    #[error("field not found: {0}")]
    FieldNotFound(String),
    #[error("multiple objects returned")]
    MultipleObjectsReturned,
    #[error("object does not exist")]
    ObjectDoesNotExist,
}

impl ErrorCode for QueryError {
    fn error_code(&self) -> &'static str {
        match self {
            Self::UnknownOperator(_) => "QUERY_UNKNOWN_OPERATOR",
            Self::FieldNotFound(_) => "QUERY_FIELD_NOT_FOUND",
            Self::MultipleObjectsReturned => "QUERY_MULTIPLE_OBJECTS",
            Self::ObjectDoesNotExist => "QUERY_NOT_FOUND",
        }
    }
    fn error_class(&self) -> ErrorClass { ErrorClass::UserError }
}
```

### Test Strategy

1. Every error variant has `error_code()` returning stable string -- unit test.
2. Error code strings are unique across all types -- uniqueness test.
3. **[v11 P2]** Negative tests: every error variant constructible and matchable -- exhaustive test.
4. Binding format test: `format!("[{}] {}", err.error_code(), err)` matches expected pattern.
5. Semver audit: error code strings never change in minor versions -- snapshot test.
6. `TST-080`: ErrorCode coverage for all binding-boundary types.

### AGENTS.md Rules

- `RULE-S08-01`: `thiserror` for library errors, `anyhow` only in binaries/tests. **Enforcement:** CI: `grep anyhow crates/*/src/ | grep -v test`.
- `RULE-S08-02`: Every `ErrorClass` variant tested. **Enforcement:** Test coverage check.
- `RULE-S08-03`: Termlet error codes must be stable string constants. **Enforcement:** Semver check.
- `RULE-S08-04`: **[v11]** Every error type crossing binding boundaries must implement `ErrorCode`. **Enforcement:** Compile-fail test.
- `RULE-S08-05`: **[v11 P2]** Error messages in bindings must include `[ERROR_CODE]` prefix. **Enforcement:** Exception message format test.

---

## 9. Protocol Codec (mux-proto)

### Design Decisions

- Wire-compatible with tmux protocol v8 (`tmux-protocol.h:23`, `PROTOCOL_VERSION 8`).
- `ImsgHdr` (header) + `ImsgFrame` (header + payload) + `ImsgCodec` (stateful encoder/decoder).
- `MsgType` enum for all message types.
- Protocol violations are fatal: `DecodeOutcome::Fatal` kills the connection.
- **[v11]** `MAX_PAYLOAD_SIZE` guard prevents OOM on malicious input.
- tokio-codec compatible for async decode.
- Traceability: `INV-001`, `INV-004`, `PAR-001`.

### Rust Example

```rust
// crates/mux-proto/src/lib.rs
pub const PROTOCOL_VERSION: u32 = 8;
/// [v11] Maximum payload size to prevent OOM. tmux uses ~64KB practical maximum.
pub const MAX_PAYLOAD_SIZE: usize = 256 * 1024; // 256KB

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MsgType {
    Identify,
    Command,
    Stdin,
    Stdout,
    Stderr,
    Detach,
    Resize,
    Ready,
    Exit,
    Exited,
    Shutdown,
    Suspend,
    Lock,
    Unlock,
    Oldstdout,
    Oldstderr,
    // ... all tmux message types
}

#[derive(Debug)]
pub struct ImsgHdr {
    pub msg_type: u32,
    pub len: u32,
    pub peer_id: u32,
    pub flags: u16,
}

#[derive(Debug)]
pub struct ImsgFrame {
    pub hdr: ImsgHdr,
    pub payload: Vec<u8>,
}

pub enum DecodeOutcome {
    Frame(ImsgFrame),
    NeedMore(usize),
    Fatal(ProtocolError),
}

pub struct ImsgCodec {
    buf: Vec<u8>,
}

impl ImsgCodec {
    pub fn new() -> Self { Self { buf: Vec::new() } }

    pub fn decode(&mut self, input: &[u8]) -> DecodeOutcome {
        self.buf.extend_from_slice(input);
        // ... decode logic ...
        // [v11] Guard against oversized payloads
        // if hdr.len as usize > MAX_PAYLOAD_SIZE {
        //     return DecodeOutcome::Fatal(ProtocolError::PayloadTooLarge {
        //         size: hdr.len as usize, max: MAX_PAYLOAD_SIZE,
        //     });
        // }
        todo!()
    }

    pub fn encode(frame: &ImsgFrame) -> Vec<u8> {
        todo!()
    }
}
```

### Test Strategy

1. Round-trip: encode then decode produces identical frame -- property test.
2. Fuzz: random bytes never panic, only `Fatal` or `NeedMore` -- cargo-fuzz target.
3. Fixture captures from real tmux decode correctly -- fixture test.
4. **[v11]** Oversized payload rejected with `PayloadTooLarge` -- unit test.
5. Protocol version assertion: `assert_eq!(PROTOCOL_VERSION, 8)` -- sanity check.
6. `TST-090`: fuzz_decode_frame target.

### AGENTS.md Rules

- `RULE-S09-01`: Protocol violations kill the connection, never drop-and-continue. **Enforcement:** `DecodeOutcome::Fatal` enforced.
- `RULE-S09-02`: **[v11]** Payload size must be bounded by `MAX_PAYLOAD_SIZE`. **Enforcement:** Decode check.

---

## 10. Config and Option System

### Design Decisions

- Config file syntax matches tmux (`set-option`, `bind-key`, etc.).
- Config loaded only after first client identify burst completes.
- Option scoping: Server -> Session -> Window -> Pane with FALLTHROUGH per `options.c:891-903`.
- `OptionSet` stores typed key-value pairs. `OptionScope` enum for resolution chain.
- Unset propagates to parent scope, matching `options.c:1269-1285`.
- Traceability: `INV-005`, `PAR-004`, `PAR-005`.

### Rust Example

```rust
// crates/mux-types/src/options.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionScope {
    Server,
    Session,
    Window,
    Pane,
}

#[derive(Debug, Clone)]
pub enum OptionValue {
    String(String),
    Number(i64),
    Bool(bool),
    Colour(u32),
    Style(StyleSpec),
    Array(Vec<String>),
}

pub struct OptionSet {
    values: std::collections::HashMap<String, OptionValue>,
}

impl OptionSet {
    pub fn get(&self, key: &str) -> Option<&OptionValue> {
        self.values.get(key)
    }

    pub fn set(&mut self, key: String, value: OptionValue) {
        self.values.insert(key, value);
    }

    pub fn unset(&mut self, key: &str) {
        self.values.remove(key);
    }
}

/// Option resolution with FALLTHROUGH from pane -> window -> session -> server.
pub fn resolve_option<'a>(
    key: &str,
    pane_opts: Option<&'a OptionSet>,
    window_opts: Option<&'a OptionSet>,
    session_opts: Option<&'a OptionSet>,
    server_opts: &'a OptionSet,
) -> Option<&'a OptionValue> {
    pane_opts.and_then(|o| o.get(key))
        .or_else(|| window_opts.and_then(|o| o.get(key)))
        .or_else(|| session_opts.and_then(|o| o.get(key)))
        .or_else(|| server_opts.get(key))
}
```

### Test Strategy

1. FALLTHROUGH: pane option overrides window option -- unit test.
2. Unset cascades to parent scope -- unit test.
3. Config parse: `set-option -g status-style "bg=red"` produces correct OptionValue -- fixture test.
4. Config load timing: config only loads after first client identify -- integration test.
5. Property test: random option hierarchies resolve correctly -- proptest.

### AGENTS.md Rules

- `RULE-S10-01`: Config loads only after first client identify burst. **Enforcement:** Startup sequence integration test.
- `RULE-S10-02`: Option FALLTHROUGH: pane -> window per `options.c:891-903`. **Enforcement:** Property test with random option hierarchies.

---

## 11. Layout Engine

### Design Decisions

- Arena-based `LayoutTree` using `Vec<LayoutCell>` with index-based parent/children.
- `LayoutCell` stores position, size, type (horizontal/vertical/pane), and pane reference.
- Resize uses round-robin one-cell-at-a-time, matching `layout.c:448-462`.
- Split enforces minimum dimensions per `tmux.h:100` (`PANE_MINIMUM`).
- Layout checksum matches `layout-custom.c:46-57`.
- `debug_assert!(layout_check(...))` after every mutation.
- Format strings match tmux via `format-audit` parity corpus.

### Rust Example

```rust
// crates/mux-core/src/layout.rs
#[derive(Debug, Clone)]
pub struct LayoutCell {
    pub kind: LayoutCellKind,
    pub x: u16, pub y: u16,
    pub cols: u16, pub rows: u16,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub pane: Option<PaneId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutCellKind {
    Horizontal,
    Vertical,
    Pane,
}

pub struct LayoutTree {
    cells: Vec<LayoutCell>,
    root: usize,
}

impl LayoutTree {
    pub fn resize(&mut self, cols: u16, rows: u16) {
        // Round-robin resize matching layout.c:448-462
        // One cell at a time, not proportional
        todo!()
    }

    pub fn checksum(&self) -> u16 {
        // Match layout-custom.c:46-57
        todo!()
    }

    pub fn split(&mut self, pane: usize, direction: Direction) -> Option<usize> {
        // Enforce PANE_MINIMUM from tmux.h:100
        todo!()
    }

    pub fn layout_check(&self) -> bool {
        // Validate: no overlap, no gap, sum of children == parent
        todo!()
    }
}
```

### Test Strategy

1. Round-robin resize: 20-pane layout matches tmux capture -- fixture test.
2. Checksum: computed checksum matches tmux for captured layouts -- byte-exact test.
3. Split minimum: split below `PANE_MINIMUM` fails gracefully -- unit test.
4. `debug_assert!(layout_check(...))` fires on invalid mutation -- debug assertion test.
5. Layout validation: no overlap, no gap -- property test.

### AGENTS.md Rules

- `RULE-S11-01`: `debug_assert!(layout_check(...))` in all layout-mutating functions. **Enforcement:** CI with `RUSTFLAGS="-C debug-assertions=on"`.
- `RULE-S11-02`: Format strings match tmux via `format-audit` parity corpus. **Enforcement:** CI: `format-audit` check.

---

## 12. QueryList ORM Layer

### Design Decisions

- 18 operators: 12 from libtmux + 6 extensions (`gt`, `gte`, `lt`, `lte`, `ne`, `glob`).
- libtmux operators verified at `query_list.py:298-312`: `eq`, `exact`, `iexact`, `contains`, `icontains`, `startswith`, `istartswith`, `endswith`, `iendswith`, `in`, `nin`, `regex`, `iregex`.
- `keygetter` nested traversal with `__` separator (libtmux lines 45-113).
- `QueryList<T>` returns chained results. `get()` returns single or error.
- `MultipleObjectsReturned`, `ObjectDoesNotExist` exceptions (from libtmux lines 550-569).
- Callable matcher support: `filter(|item| predicate)` (libtmux line 535).
- **[v11 P2]** `BindingQueryList<T>` trait from GPT for language binding query wrappers.
- Traceability: `PAR-120`, `API-050`.

### Rust Example

```rust
// crates/mux-query/src/lib.rs
#[derive(Debug, Clone)]
pub enum QueryOp {
    Eq, Exact, IExact,
    Contains, IContains,
    StartsWith, IStartsWith,
    EndsWith, IEndsWith,
    In, Nin,
    Regex, IRegex,
    // Extensions beyond libtmux
    Gt, Gte, Lt, Lte, Ne, Glob,
}

#[derive(Debug, Clone)]
pub enum QueryValue {
    String(String),
    Int(i64),
    Bool(bool),
    List(Vec<QueryValue>),
    None,
}

pub struct QuerySpec {
    pub field: String,
    pub op: QueryOp,
    pub value: QueryValue,
}

pub struct QueryList<T> {
    items: Vec<T>,
}

impl<T: Clone> QueryList<T> {
    pub fn filter<F: Fn(&T) -> bool>(&self, predicate: F) -> QueryList<T> {
        QueryList {
            items: self.items.iter().filter(|i| predicate(i)).cloned().collect(),
        }
    }

    pub fn filter_by(&self, specs: &[QuerySpec]) -> QueryList<T>
    where T: Queryable {
        QueryList {
            items: self.items.iter()
                .filter(|item| specs.iter().all(|spec| item.matches(spec)))
                .cloned()
                .collect(),
        }
    }

    pub fn get(&self) -> Result<&T, QueryError> {
        match self.items.len() {
            0 => Err(QueryError::ObjectDoesNotExist),
            1 => Ok(&self.items[0]),
            _ => Err(QueryError::MultipleObjectsReturned),
        }
    }

    pub fn first(&self) -> Option<&T> { self.items.first() }
    pub fn count(&self) -> usize { self.items.len() }
}

pub trait Queryable {
    fn matches(&self, spec: &QuerySpec) -> bool;
    fn field_value(&self, field: &str) -> Option<QueryValue>;
}

/// [v11 P2] Trait for language binding query wrappers (from GPT).
/// Bindings implement this to provide language-native query APIs.
pub trait BindingQueryList<T> {
    fn filter_kwargs(&self, kwargs: &[(String, String)]) -> Vec<T>;
    fn get_kwargs(&self, kwargs: &[(String, String)]) -> Result<T, QueryError>;
}
```

### Test Strategy

1. All 18 operators produce correct results -- parametrized unit test per operator.
2. `keygetter` traversal: `session__name` resolves nested field -- unit test.
3. `get()` single: returns item. `get()` multiple: returns `MultipleObjectsReturned`. `get()` zero: returns `ObjectDoesNotExist`.
4. Callable filter: `filter(|s| s.name.starts_with("test"))` works -- unit test.
5. Operator parity with libtmux: test same inputs produce same results -- parity test.
6. **[v11 P2]** `BindingQueryList` implementations tested for Python and Node.

### AGENTS.md Rules

- `RULE-S12-01`: Key bindings match tmux via generated parity tests. **Enforcement:** CI: key-binding parity tests.
- `RULE-S12-02`: QueryList must support all 18 operators. **Enforcement:** Parametrized test coverage.

---

## 13. State Actor and ArcSwap

### Design Decisions

- Single-writer `StateActor` processes events sequentially. All mutations go through the actor.
- `ArcSwap<GraphState>` for zero-contention snapshot reads by clients, bindings, TUI.
- No `Arc<Mutex<_>>` on the read path. Ever.
- Snapshot published after every event application.
- `StateHandle` wraps `Arc<ArcSwap<GraphState>>` for ergonomic access.
- Traceability: `INV-003`, `API-060`.

### Rust Example

```rust
// crates/mux-api/src/handle.rs
use arc_swap::ArcSwap;
use std::sync::Arc;

pub struct StateHandle {
    inner: Arc<ArcSwap<GraphState>>,
}

impl StateHandle {
    pub fn load(&self) -> arc_swap::Guard<Arc<GraphState>> {
        self.inner.load()
    }

    /// Get a clone of the current snapshot.
    pub fn snapshot(&self) -> Arc<GraphState> {
        self.inner.load_full()
    }
}
```

### Test Strategy

1. Concurrent reads during writes: no blocking -- concurrency test.
2. Snapshot freshness: read after write reflects latest state -- ordering test.
3. No `Arc<Mutex<_>>` in read path -- architecture test (grep).

### AGENTS.md Rules

- `RULE-S13-01`: Only the state actor mutates graph state. **Enforcement:** ArcSwap read path, CI grep for `Mutex<ServerGraph>`.

---

## 14. Server Lifecycle [v11 P2 updated]

### Design Decisions

- Startup sequence: socket creation -> flock -> accept loop -> first client identify -> config load.
- flock pattern matches `client.c:89`: `flock(LOCK_EX|LOCK_NB)`.
- Socket dir permissions: `0700`. Socket permissions: `0600`.
- `LockGuard` with `Drop` for automatic cleanup (never manual `unlock()`).
- **[v11 P2]** `ServerPhase` enum tracks lifecycle state explicitly (from GPT): `Bootstrap -> SocketReady -> AwaitIdentify -> ConfigLoading -> Running -> ShuttingDown -> Stopped`.
- Traceability: `INV-005`, `OPS-010`.

### Rust Example

```rust
// crates/mux-server/src/lifecycle.rs

/// [v11 P2] Server lifecycle phases.
/// Transitions are monotonic and logged via OTEL spans.
impl ServerGraph {
    pub fn transition_phase(&mut self, new_phase: ServerPhase) {
        debug_assert!(
            new_phase as u8 > self.phase as u8,
            "ServerPhase must only advance: {:?} -> {:?}", self.phase, new_phase
        );
        tracing::info!(from = ?self.phase, to = ?new_phase, "server phase transition");
        self.phase = new_phase;
    }
}

pub struct LockGuard {
    fd: std::os::unix::io::RawFd,
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        unsafe { libc::flock(self.fd, libc::LOCK_UN); }
    }
}

pub fn acquire_lock(path: &std::path::Path) -> std::io::Result<LockGuard> {
    use std::os::unix::io::AsRawFd;
    let file = std::fs::File::create(path)?;
    let fd = file.as_raw_fd();
    let ret = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };
    if ret != 0 {
        return Err(std::io::Error::last_os_error());
    }
    std::mem::forget(file); // Fd owned by guard
    Ok(LockGuard { fd })
}
```

### Test Strategy

1. flock: second server on same socket fails immediately -- integration test.
2. `LockGuard` Drop releases lock -- unit test.
3. Socket permissions: dir `0700`, socket `0600` -- integration test.
4. Config not loaded until after identify -- startup sequence test.
5. **[v11 P2]** ServerPhase monotonic: transition backward panics in debug -- unit test.

### AGENTS.md Rules

- `RULE-S14-01`: Lock file guard via `Drop` impl, never manual unlock. **Enforcement:** CI grep: no manual `unlock()` calls.
- `RULE-S14-02`: Config load trigger must remain post-identify. **Enforcement:** Startup sequence test.
- `RULE-S14-03`: **[v11 P2]** ServerPhase transitions must be monotonic. **Enforcement:** Debug assertion.

---

## 15. Control Mode

### Design Decisions

- Control mode (`tmux -CC`) provides typed notifications for external consumers.
- Notifications are hints, not authoritative -- periodic full refresh required.
- Typed `ControlNotification` enum for all notification types.
- Control mode pending limit matches `control.c:450-461`.
- Extended output format matches `control.c:620-623`.
- Startup handshake matches `control.c:758-796`.
- **[v11 P2]** `C6` gate: control-mode backpressure (pause/resume) semantics must match tmux.
- Traceability: `PAR-010`, `API-070`.

### Rust Example

```rust
// crates/mux-control/src/notification.rs
#[derive(Debug, Clone)]
pub enum ControlNotification {
    SessionChanged { session_id: String, name: String },
    SessionRenamed { session_id: String, name: String },
    WindowAdded { window_id: String },
    WindowClosed { window_id: String },
    WindowRenamed { window_id: String, name: String },
    PaneOutput { pane_id: String, data: Vec<u8> },
    PaneModeChanged { pane_id: String },
    LayoutChanged { window_id: String, layout: String },
    ClientSessionChanged { client_id: String, session: String },
    /// [v11 P2] Pause notification for backpressure from GPT.
    Pause,
    Continue,
    Exit { reason: String },
}

pub fn parse_notification(line: &str) -> Option<ControlNotification> {
    // Parse tmux control mode notification format
    // %session-changed $id $name
    // %window-add @id
    // etc.
    todo!()
}
```

### Test Strategy

1. Parse all notification types from captured tmux output -- fixture test.
2. Periodic refresh overrides stale notifications -- integration test.
3. **[v11 P2]** Backpressure: Pause/Continue notifications handled correctly -- integration test.
4. Pending limit enforcement -- unit test.

### AGENTS.md Rules

- `RULE-S15-01`: Control notifications are hints, not authoritative. **Enforcement:** Periodic refresh tests.
- `RULE-S15-02`: **[v11 P2]** Control-mode backpressure semantics match tmux `control.c:450-461`. **Enforcement:** Parity test.

---

## 16. Language Bindings [v11 P2 updated]

### Design Decisions

- Python bindings via PyO3. Node bindings via Neon.
- Bindings depend on `mux-api`/`mux-orm`, never `mux-core` directly.
- Python: GIL released during Rust operations with `py.allow_threads()`.
- Node: `JsBox` for resource handles; Promise-based for blocking operations.
- Package name: `termforge` in both ecosystems.
- **[v11]** Error messages include `[ERROR_CODE]` prefix from `ErrorCode` trait.
- **[v11 P2]** `BindingQueryList<T>` trait from GPT for binding query wrappers.
- **[v11 P2]** `CrossLangCase` struct from GPT for shared test fixtures.
- Traceability: `API-080`, `PAR-120`.

### Rust Example

```rust
// bindings/python/src/lib.rs
use pyo3::prelude::*;

#[pyclass]
pub struct PyServer {
    handle: StateHandle,
}

#[pymethods]
impl PyServer {
    /// server.sessions.filter(name="test")
    fn sessions(&self, py: Python<'_>) -> PyResult<PyQueryList> {
        py.allow_threads(|| {
            let snap = self.handle.snapshot();
            let sessions: Vec<_> = snap.sessions.values().cloned().collect();
            Ok(PyQueryList::new(sessions))
        })
    }
}

#[pyclass]
pub struct PyQueryList {
    items: Vec<Session>,
}

#[pymethods]
impl PyQueryList {
    /// filter(**kwargs) -> QueryList
    fn filter(&self, kwargs: &PyDict) -> PyResult<PyQueryList> {
        // Parse kwargs into QuerySpec, apply filter
        todo!()
    }

    /// get(**kwargs) -> Session (raises if 0 or 2+)
    fn get(&self, kwargs: &PyDict) -> PyResult<PySession> {
        // [v11] Error message includes [ERROR_CODE] prefix
        todo!()
    }
}

/// [v11 P2] Shared cross-language test case from GPT.
/// Used to verify identical behavior across Rust, Python, Node.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CrossLangCase {
    pub name: String,
    pub setup: Vec<String>,       // commands to run
    pub filter_kwargs: Vec<(String, String)>,
    pub expected_count: usize,
    pub expected_names: Vec<String>,
}
```

### Test Strategy

1. Python `filter(**kwargs)` matches Rust `QueryList.filter_by()` -- parity test.
2. Python `get()` raises correct exceptions -- pytest negative test.
3. Node `server.sessions()` returns filtered array -- vitest.
4. GIL released during Rust operations -- threading test.
5. Error messages include `[ERROR_CODE]` prefix -- exception message format test.
6. **[v11 P2]** `CrossLangCase` fixtures produce identical results in Rust/Python/Node -- multi-runner test.

### AGENTS.md Rules

- `RULE-S16-01`: Bindings depend only on `mux-api`/`mux-orm`, never `mux-core` directly. **Enforcement:** `cargo deny check`.
- `RULE-S16-02`: GIL released for Rust operations > 1ms. **Enforcement:** Code review + timing test.
- `RULE-S16-03`: Termlet bindings expose: spawn, send_keys, wait_for, snapshot, resize, kill. **Enforcement:** API surface test.
- `RULE-S16-04`: **[v11]** Error messages in bindings must include `[ERROR_CODE]` prefix. **Enforcement:** Exception message format test.
- `RULE-S16-05`: **[v11 P2]** `CrossLangCase` fixtures required for new query features. **Enforcement:** Multi-runner CI.

---

## 17. CRDT Layer

### Design Decisions

- HLC (Hybrid Logical Clock) for causal ordering across replicas.
- LWW (Last-Writer-Wins) register for simple value convergence.
- OrSet (Observed-Remove Set) for add-wins set semantics.
- OpLog for operation recording, replay, and merge.
- **[v11]** `OpLog::compact_before()` for bounded log growth.
- **[v11]** `OrSet::merge()` for replica convergence.
- All CRDT types are pure (Layer 0). No IO.
- Traceability: `API-090`.

### Rust Example

```rust
// crates/mux-crdt/src/lib.rs
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HLC {
    pub wall_ms: u64,
    pub counter: u32,
    pub node_id: u64,
}

impl HLC {
    pub fn tick(&mut self) {
        self.counter += 1;
    }

    pub fn merge(&mut self, other: &HLC) {
        if other.wall_ms > self.wall_ms {
            self.wall_ms = other.wall_ms;
            self.counter = other.counter + 1;
        } else if other.wall_ms == self.wall_ms && other.counter >= self.counter {
            self.counter = other.counter + 1;
        } else {
            self.counter += 1;
        }
    }
}

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
}

pub struct OrSet<T: Eq + std::hash::Hash + Clone> {
    elements: HashSet<(T, HLC)>,
}

impl<T: Eq + std::hash::Hash + Clone> OrSet<T> {
    pub fn add(&mut self, value: T, ts: HLC) {
        self.elements.insert((value, ts));
    }

    pub fn remove(&mut self, value: &T) {
        self.elements.retain(|(v, _)| v != value);
    }

    /// [v11] Merge from another replica.
    pub fn merge(&mut self, other: &OrSet<T>) {
        for elem in &other.elements {
            self.elements.insert(elem.clone());
        }
    }

    pub fn values(&self) -> HashSet<&T> {
        self.elements.iter().map(|(v, _)| v).collect()
    }
}

pub struct OpLog {
    ops: Vec<(HLC, Operation)>,
}

impl OpLog {
    /// [v11] Compact operations before the given timestamp.
    pub fn compact_before(&mut self, before: HLC) {
        self.ops.retain(|(ts, _)| *ts >= before);
    }
}
```

### Test Strategy

1. HLC merge: concurrent clocks converge -- property test.
2. LWW: later timestamp wins -- unit test.
3. OrSet: add-wins semantics after concurrent add/remove -- property test.
4. **[v11]** OpLog compaction: entries before timestamp removed -- unit test.
5. **[v11]** OrSet merge: replicas converge to same set -- property test.

### AGENTS.md Rules

- `RULE-S17-01`: CRDT types must be pure (no IO). **Enforcement:** WASM CI check.
- `RULE-S17-02`: Property tests for convergence of all CRDT types. **Enforcement:** proptest in CI.

---

## 18. Socket and Permissions

### Design Decisions

- Socket directory: `$TMPDIR/termforge-$UID/`.
- Socket permissions: dir `0700`, socket `0600`.
- Path length check before creation (Unix socket limit: 108 bytes).
- `PathGuard` from vibe-tmux enforces three layers of socket validation in tests.
- Traceability: `INV-007`, `OPS-020`.

### Rust Example

```rust
// crates/mux-os/src/socket.rs
pub const SOCKET_PATH_MAX: usize = 108;

pub fn create_socket_dir(uid: u32) -> std::io::Result<std::path::PathBuf> {
    let dir = std::env::temp_dir().join(format!("termforge-{uid}"));
    std::fs::create_dir_all(&dir)?;
    // Set permissions to 0700
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))?;
    Ok(dir)
}

pub fn validate_socket_path(path: &std::path::Path) -> Result<(), SocketError> {
    let path_str = path.to_str().ok_or(SocketError::InvalidPath)?;
    if path_str.len() > SOCKET_PATH_MAX {
        return Err(SocketError::PathTooLong {
            len: path_str.len(),
            max: SOCKET_PATH_MAX,
        });
    }
    Ok(())
}
```

### Test Strategy

1. Socket dir permissions: `0700` verified -- integration test.
2. Socket permissions: `0600` verified -- integration test.
3. Path length: > 108 bytes rejected -- unit test.
4. PathGuard three-layer validation in all test harnesses -- fixture enforcement.

### AGENTS.md Rules

- `RULE-S18-01`: Permission hardening: socket dir `0700`, socket `0600`. **Enforcement:** Test assertions.
- `RULE-S18-02`: Socket path must be validated before creation. **Enforcement:** Path length check.

---

## 19. Observability (OTEL) [v11 updated]

### Design Decisions

- Dual OTEL providers: `OTEL_PROVIDER` (server) + `MUX_CLIENT_PROVIDER` (client).
- Both stored as `OnceLock<Mutex<Option<OtelProvider>>>` (vibe-tmux `mux-otel/src/otel.rs:58-59`).
- Composite propagator: `BaggagePropagator` + `TraceContextPropagator` (lines 771-777).
- `HeaderCarrier` with `Injector`/`Extractor` for cross-process propagation (lines 998-1028).
- `force_flush` both providers before process exit.
- `shutdown_timeout(Duration::from_millis(200))` to prevent hanging (line 275).
- **[v11]** Termlet lifecycle spans with structured attributes: `termlet.id`, `pane.id`, `backend.mode`.
- Traceability: `OPS-030`, `INV-007`.

### Rust Example

```rust
// crates/mux-otel/src/otel.rs
use std::sync::{Mutex, OnceLock};

pub struct OtelProvider {
    pub logger: Option<opentelemetry_sdk::logs::SdkLoggerProvider>,
    pub tracer_provider: Option<opentelemetry_sdk::trace::SdkTracerProvider>,
    pub tracer: Option<opentelemetry::global::BoxedTracer>,
    pub runtime: Option<tokio::runtime::Runtime>,
}

static OTEL_PROVIDER: OnceLock<Mutex<Option<OtelProvider>>> = OnceLock::new();
static MUX_CLIENT_PROVIDER: OnceLock<Mutex<Option<OtelProvider>>> = OnceLock::new();

pub struct TraceHeaders {
    pub traceparent: String,
    pub tracestate: String,
    pub baggage: String,
}

pub struct HeaderCarrier {
    headers: std::collections::HashMap<String, String>,
}

impl opentelemetry::propagation::Injector for HeaderCarrier {
    fn set(&mut self, key: &str, value: String) {
        self.headers.insert(key.to_string(), value);
    }
}

impl opentelemetry::propagation::Extractor for HeaderCarrier {
    fn get(&self, key: &str) -> Option<&str> {
        self.headers.get(key).map(|v| v.as_str())
    }
    fn keys(&self) -> Vec<&str> {
        self.headers.keys().map(|k| k.as_str()).collect()
    }
}
```

### Test Strategy

1. Dual providers: server and client provider both initialize -- integration test.
2. Trace propagation: spans have correct parent IDs across server/client -- integration test.
3. `force_flush` completes before process exit -- shutdown test.
4. **[v11]** Termlet spans include `termlet.id`, `pane.id`, `backend.mode` attributes -- attribute presence test.
5. `shutdown_timeout(200ms)` prevents blocking exit -- timeout test.

### AGENTS.md Rules

- `RULE-S19-01`: Dual OTEL providers: server and client. **Enforcement:** Architecture test.
- `RULE-S19-02`: `shutdown_timeout` bounded at 200ms. **Enforcement:** Config check.
- `RULE-S19-03`: Trace headers propagated across server/client boundary. **Enforcement:** Integration test.
- `RULE-S19-04`: **[v11]** Termlet spans must include structured attributes. **Enforcement:** Attribute presence test.

---

## 20. tmux Builder and Version Manager

### Design Decisions

- `tmux-builder`: compiles tmux from source with BLAKE3-hashed cache keys.
- Cache key format: `tmux-{version}__{host}__{os}-{os_ver}__cfg{cfg}__mk{mk}__tb{tool}` (line 521).
- BLAKE3 `flags_fingerprint`: null-separated flag bytes, truncated to 12 hex chars (lines 511, 543).
- File locking with `LockGuard` (`Drop` for `fs2::FileExt::unlock`, lines 247-256).
- `sibling_tmp_dir` with `pid+nanos` for build isolation (line 409).
- Atomic publication via `std::fs::rename` (line 389).
- `validate_tmux_binary` via `tmux -V` output check (line 549).
- `mux-vm`: version manager for easy access to tmux binaries at various versions.
- Traceability: `OPS-040`.

### Rust Example

```rust
// tools/tmux-builder/src/lib.rs
pub fn compute_cache_key(
    version: &str, host: &str, os: &str, os_ver: &str,
    flags: &[u8], make_flags: &str, toolchain: &str,
) -> String {
    let cfg = flags_fingerprint(flags);
    let mk = blake3_short(make_flags.as_bytes());
    let tool = blake3_short(toolchain.as_bytes());
    format!("tmux-{version}__{host}__{os}-{os_ver}__cfg{cfg}__mk{mk}__tb{tool}")
}

fn flags_fingerprint(flags: &[u8]) -> String {
    let hash = blake3::hash(flags);
    hash.to_hex()[..12].to_string()
}

fn blake3_short(data: &[u8]) -> String {
    blake3::hash(data).to_hex()[..12].to_string()
}

pub struct LockGuard {
    file: std::fs::File,
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        use fs2::FileExt;
        let _ = self.file.unlock();
    }
}
```

### Test Strategy

1. Cache key reproducibility: same inputs produce same key -- unit test.
2. BLAKE3 fingerprint: 12-char hex truncation -- unit test.
3. File lock: concurrent builds block correctly -- integration test.
4. Atomic rename: partial build never visible -- integration test.
5. `validate_tmux_binary` with real tmux -- integration test (conditional on tmux availability).

### AGENTS.md Rules

- `RULE-S20-01`: File-based locking with `Drop` guard for parallel builds. **Enforcement:** `LockGuard` pattern enforced.
- `RULE-S20-02`: Atomic build publication via rename. **Enforcement:** CI grep for `std::fs::copy` absence.

---

## 21. Test Support and FakePty [v11 P2 updated]

### Design Decisions

- `PathGuard` enforces three-layer socket validation (vibe-tmux pattern):
  1. `ensure_not_default_socket_name` -- never use "default".
  2. `ensure_socket_within_tempdir` -- socket must be under temp dir.
  3. `ensure_socket_not_tmux_env` -- socket must not match `$TMUX` env.
- Test harnesses use `-f /dev/null` for config isolation.
- `FakePtyBackend` for deterministic tests with injected output.
- `ScenarioRecorder` JSON format for reproducible fixtures.
- `PtyBackend` trait requires `Send + Any` bounds with `as_any_mut()`.
- **[v11 P2]** `normalize_output` and `ParityResult` from GPT for tmux parity testing.
- Traceability: `INV-007`, `TST-200`.

### Rust Example

```rust
// crates/mux-test-support/src/path_guard.rs
pub struct PathGuard;

impl PathGuard {
    pub fn validate(socket_name: &str, socket_path: &std::path::Path) -> Result<(), String> {
        Self::ensure_not_default_socket_name(socket_name)?;
        Self::ensure_socket_within_tempdir(socket_path)?;
        Self::ensure_socket_not_tmux_env(socket_path)?;
        Ok(())
    }

    fn ensure_not_default_socket_name(name: &str) -> Result<(), String> {
        if name == "default" {
            return Err("socket name must not be 'default'".into());
        }
        Ok(())
    }

    fn ensure_socket_within_tempdir(path: &std::path::Path) -> Result<(), String> {
        let temp = std::env::temp_dir();
        if !path.starts_with(&temp) {
            return Err(format!("socket path {path:?} not under temp dir {temp:?}"));
        }
        Ok(())
    }

    fn ensure_socket_not_tmux_env(path: &std::path::Path) -> Result<(), String> {
        if let Ok(tmux_env) = std::env::var("TMUX") {
            let tmux_socket = tmux_env.split(',').next().unwrap_or("");
            if path.to_str().map_or(false, |p| p.contains(tmux_socket)) {
                return Err("socket path must not match $TMUX env".into());
            }
        }
        Ok(())
    }
}

// crates/mux-pty/src/backend.rs
/// PtyBackend trait. [v11] Requires Send + Any for downcasting.
pub trait PtyBackend: Send + std::any::Any {
    fn spawn(&mut self, pane_id: PaneId, argv: &[String],
             cwd: Option<&str>, size: PaneSize) -> Result<(), Box<dyn std::error::Error>>;
    fn write(&mut self, pane_id: PaneId, data: &[u8]) -> Result<(), String>;
    fn read_events(&mut self) -> Vec<PtyEvent>;
    fn resize(&mut self, pane_id: PaneId, size: PaneSize) -> Result<(), String>;
    fn kill(&mut self, pane_id: PaneId, signal: i32) -> Result<(), String>;
    /// [v11] Required for downcasting to FakePtyBackend.
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

/// [v11 P2] Output normalizer for parity testing (from GPT).
pub fn normalize_output(output: &str) -> String {
    output
        .lines()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
        .trim_end_matches('\n')
        .to_string()
}

/// [v11 P2] Result of parity comparison with tmux (from GPT).
#[derive(Debug)]
pub struct ParityResult {
    pub passed: bool,
    pub termforge_output: String,
    pub tmux_output: String,
    pub diff: Option<String>,
}
```

### Test Strategy

1. PathGuard: "default" socket name rejected -- unit test.
2. PathGuard: socket outside tempdir rejected -- unit test.
3. PathGuard: socket matching `$TMUX` rejected -- unit test.
4. FakePty: deterministic replay produces same grid state -- unit test.
5. `as_any_mut()` downcast to `FakePtyBackend` succeeds -- unit test.
6. **[v11 P2]** `normalize_output` trims trailing whitespace and trailing newlines -- unit test.
7. **[v11 P2]** `ParityResult` captures diff when outputs diverge -- unit test.

### AGENTS.md Rules

- `RULE-S21-01`: Three-layer socket validation in all test harnesses. **Enforcement:** PathGuard enforced in test fixtures.
- `RULE-S21-02`: Fake PTY tests use `ScenarioRecorder` JSON format. **Enforcement:** Test harness validation.
- `RULE-S21-03`: **[v11]** `PtyBackend` trait must require `Send + Any` bounds. **Enforcement:** Compile-fail test.
- `RULE-S21-04`: **[v11 P2]** Parity tests must use `normalize_output` before comparison. **Enforcement:** Code review.

---

## 22. Parity Test Framework

### Design Decisions

- `mux-regress`: runs identical scenarios against both TermForge and real tmux.
- Captures output from both and compares using normalized text.
- tmux version matrix: 3.3a, 3.4, 3.5, 3.6 (via `mux-vm`).
- Test harnesses use separate socket names/paths -- never conflicts with live tmux.
- Clipboard disabled in all test harnesses: `set-clipboard off`.
- Config isolated: `-f /dev/null`.
- **[v11 P2]** `CrossLangCase` from GPT for cross-language consistency fixtures.
- Traceability: `PAR-*`, `TST-220`.

### Rust Example

```rust
// tools/mux-regress/src/lib.rs
pub struct ParityScenario {
    pub name: String,
    pub commands: Vec<String>,
    pub expected_output: String,
    pub tmux_version: Option<String>,
}

pub fn run_parity(scenario: &ParityScenario) -> ParityResult {
    // 1. Run against TermForge with isolated socket
    let tf_output = run_termforge_scenario(scenario);
    // 2. Run against real tmux with isolated socket
    let tmux_output = run_tmux_scenario(scenario);
    // 3. Compare normalized output
    let tf_norm = normalize_output(&tf_output);
    let tmux_norm = normalize_output(&tmux_output);
    ParityResult {
        passed: tf_norm == tmux_norm,
        termforge_output: tf_norm.clone(),
        tmux_output: tmux_norm.clone(),
        diff: if tf_norm != tmux_norm {
            Some(compute_diff(&tf_norm, &tmux_norm))
        } else { None },
    }
}
```

### Test Strategy

1. Parity: `tmux list-sessions` identical output -- parity test.
2. Config parse: identical option values after config load -- parity test.
3. Layout checksum: identical for same sequence of operations -- parity test.
4. Clipboard: `set-clipboard off` in all test harnesses -- config check.
5. Socket isolation: test sockets never interfere with live tmux -- PathGuard enforcement.

### AGENTS.md Rules

- `RULE-S22-01`: Test harnesses use `-f /dev/null` for config isolation. **Enforcement:** Test fixture enforcement.
- `RULE-S22-02`: No test may use the default tmux socket. **Enforcement:** PathGuard three-layer check.
- `RULE-S22-03`: Termlet fixtures must auto-kill on cleanup. **Enforcement:** Leak detection test.
- `RULE-S22-04`: Termlet tests must run in both real and fake PTY modes. **Enforcement:** Parametrized fixture.

---

## 23. Fuzz Testing

### Design Decisions

- Three cargo-fuzz targets: VT100 parser, protocol decoder, config parser.
- Random bytes must never panic -- only produce errors or valid results.
- Nightly CI runs fuzz for extended durations.
- Fuzz findings become regression fixtures.
- Traceability: `TST-230`.

### Rust Example

```rust
// fuzz/fuzz_targets/proto_decode.rs
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let mut codec = mux_proto::ImsgCodec::new();
    // Must not panic, only Fatal or NeedMore
    let _ = codec.decode(data);
});

// fuzz/fuzz_targets/vt100_parse.rs
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let mut grid = mux_grid::Grid::new(80, 24);
    let mut parser = mux_grid::VtParser::new();
    // Must not panic
    parser.parse(data, &mut grid);
});
```

### Test Strategy

1. Protocol fuzz: random bytes -> no panic -- cargo-fuzz nightly.
2. VT100 fuzz: random bytes -> no panic -- cargo-fuzz nightly.
3. Config fuzz: random text -> no panic -- cargo-fuzz nightly.
4. Regression: fuzz findings converted to permanent fixtures.

### AGENTS.md Rules

- `RULE-S23-01`: Fuzz targets must exist for all parser/decoder crates. **Enforcement:** Fuzz target audit.
- `RULE-S23-02`: Fuzz findings become regression fixtures within 48 hours. **Enforcement:** PR process.
- `RULE-S23-03`: **[v11]** Test names must match `test_{component}_{behavior}` convention. **Enforcement:** CI regex validation.

---

## 24. Performance Benchmarks [v11 P2 updated]

### Design Decisions

- Criterion benchmarks for hot-path code.
- Performance targets table with pass/fail thresholds.
- Nightly CI runs benchmarks; regression > 30% blocks release.
- Baselines from 3 consecutive median runs.
- **[v11 P2]** `PerfTarget` struct from GPT with `target_ns`/`hard_fail_ns` for programmatic enforcement.
- Traceability: `OPS-050`.

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
| B10 | Termlet spawn (FakePty) | < 500 us | Warn if > 1 ms |
| B11 | Termlet snapshot (80x24) | < 50 us | Warn if > 100 us |
| B12 | Termlet spawn (real PTY) | < 50 ms | Warn if > 100 ms |
| B13 | drain_output 1KB buffer | < 10 us | Warn if > 50 us |

### Rust Example

```rust
// crates/mux-bench/benches/termlet.rs
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

/// [v11 P2] PerfTarget-based assertion (from GPT).
fn assert_perf_targets(results: &[(String, u64)], targets: &[PerfTarget]) {
    for target in targets {
        if let Some((_, ns)) = results.iter().find(|(n, _)| n == target.name) {
            assert!(
                *ns <= target.hard_fail_ns,
                "HARD FAIL: {} took {}ns, limit {}ns",
                target.name, ns, target.hard_fail_ns
            );
            if *ns > target.target_ns {
                eprintln!("WARN: {} took {}ns, target {}ns", target.name, ns, target.target_ns);
            }
        }
    }
}

criterion_group!(termlet_benches, termlet_spawn_fake, termlet_snapshot);
criterion_main!(termlet_benches);
```

### Test Strategy

1. Criterion benchmarks run nightly.
2. Regression flag at 130% of baseline.
3. Baseline established from 3 consecutive median runs.
4. No hard fail on performance (warn only). Hard fail reserved for `PerfTarget.hard_fail_ns`.
5. Termlet benchmarks included in nightly run.
6. Benchmark scripts pin CPU governor where possible.
7. Performance regressions > 30% block release.

### AGENTS.md Rules

- `RULE-S24-01`: Performance baselines re-established after architecture changes. **Enforcement:** CI baseline refresh.
- `RULE-S24-02`: Benchmarks must cover hot-path code (VT100, proto, layout). **Enforcement:** Benchmark audit.
- `RULE-S24-03`: Termlet spawn benchmark must stay under 500us for FakePty mode. **Enforcement:** Criterion CI.
- `RULE-S24-04`: Performance regressions > 30% block release. **Enforcement:** Release gate.

---

## 25. Visual Client / TUI

### Design Decisions

- ViewModel pattern: pure function `(GraphState, ClientId, TermSize) -> ViewModel`.
- ratatui-based rendering with crossterm terminal IO.
- ViewModel supports snapshot testing with `insta`.
- TUI can attach to both TermForge server (via StateHandle) and real tmux (via control mode).
- `TermletViewModel` for debug inspector panel shows Termlet grid preview and process status.
- **[v11]** `TermletPoolViewModel` for multi-Termlet inspector with alive count.

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
    todo!()
}

/// TUI can target TermForge server or real tmux.
pub enum TuiTarget {
    TermForge { state_handle: StateHandle },
    RealTmux { socket_path: std::path::PathBuf },
}

/// Debug inspector panel for Termlet state.
pub struct TermletViewModel {
    pub pane_id: TermletPaneId,
    pub alive: bool,
    pub state: TermletState,         // [v11 P2] from GPT
    pub grid_preview: String,
    pub size: (u16, u16),
    pub exit_status: Option<i32>,    // [v11 P2] from GPT
}

/// [v11] Multi-Termlet inspector panel.
pub struct TermletPoolViewModel {
    pub termlets: Vec<TermletViewModel>,
    pub alive_count: usize,
    pub total_count: usize,
}
```

### Test Strategy

1. ViewModel snapshot: pure ViewModel from test GraphState matches `insta` snapshot -- snapshot comparison.
2. Status line format: format expansion produces correct status text -- string equality.
3. Pane layout: ViewModel pane positions match layout tree -- coordinate comparison.
4. Key conversion: crossterm key events map to mux-core Key variants -- variant match.
5. Border rendering: correct border characters at pane boundaries -- character equality.
6. Zoomed pane: single pane fills entire area when zoomed -- dimension check.
7. Resize: terminal resize updates ViewModel dimensions -- dimension comparison.
8. Attach to real tmux: control mode notifications render correctly -- snapshot test.
9. TermletViewModel: grid preview matches Termlet snapshot -- string equality.
10. **[v11]** TermletPoolViewModel: alive_count matches actual alive Termlets -- count equality.
11. **[v11 P2]** TermletViewModel includes `state` and `exit_status` -- field presence test.

### AGENTS.md Rules

- `RULE-S25-01`: TUI reads snapshots only, never mutates graph directly. **Enforcement:** No `&mut ServerGraph` in view code.
- `RULE-S25-02`: UI event handlers emit commands/events, not direct state mutations. **Enforcement:** Code review.
- `RULE-S25-03`: Debug panels are non-blocking and optional. **Enforcement:** Feature flag for debug panels.

---

## 26. AGENTS.md Rules [v11 P2 updated]

### Design Decisions

This section consolidates all per-section rules into a global reference with numbered IDs and enforcement mechanisms. The rule naming convention follows `RULE-Snn-xx` pattern for traceability. **[v11]** All rules now have explicit enforcement mechanisms (was missing for 8 rules in v10). **[v11 P2]** Added rules for `TermletState`, `ServerPhase`, `CrossLangCase`, `normalize_output`, `inherit_env`, and GPT traceability lint. Total: 66 rules (was 58 in v11, 47 in Gemini v11, 10 in GPT v11).

### 26.1 Master Rule Table

| # | Rule ID | Rule | Enforcement |
|---|---|---|---|
| 1 | RULE-S05-01 | `mux-core` is pure: no IO, no `unsafe`, no `tokio`, no `libc` | `#![forbid(unsafe_code)]`, WASM CI check |
| 2 | RULE-S07-01 | Core emits effects; only the runtime executes effects | Code review + typed `Effect` enum |
| 3 | RULE-S07-02 | All state transitions go through `apply_event()` | CI grep for direct graph mutation |
| 4 | RULE-S13-01 | Only the state actor mutates graph state | ArcSwap read path, CI grep for `Mutex<ServerGraph>` |
| 5 | RULE-S08-01 | `thiserror` for library errors, `anyhow` only in binaries/tests | CI: `grep anyhow crates/*/src/ \| grep -v test` |
| 6 | RULE-S08-02 | Every `ErrorClass` variant tested | Test coverage check |
| 7 | RULE-S09-01 | Protocol violations kill the connection, never drop-and-continue | `DecodeOutcome::Fatal` enforced |
| 8 | RULE-S10-01 | Config loads only after first client identify burst | Startup sequence integration test |
| 9 | RULE-S10-02 | Option FALLTHROUGH: pane -> window per `options.c:891-903` | Property test with random option hierarchies |
| 10 | RULE-S11-01 | `debug_assert!(layout_check(...))` in all layout-mutating functions | CI with `RUSTFLAGS="-C debug-assertions=on"` |
| 11 | RULE-S11-02 | Format strings match tmux via `format-audit` parity corpus | CI: `format-audit` check |
| 12 | RULE-S12-01 | Key bindings match tmux via generated parity tests | CI: key-binding parity tests |
| 13 | RULE-S06-01 | All entity IDs via `slotmap::new_key_type!` | Code review + type system |
| 14 | RULE-S16-01 | Bindings depend only on `mux-api`/`mux-orm`, never `mux-core` directly | `cargo deny check` |
| 15 | RULE-S21-02 | Fake PTY tests use `ScenarioRecorder` JSON format | Test harness validation |
| 16 | RULE-S15-01 | Control notifications are hints, not authoritative | Periodic refresh tests |
| 17 | RULE-S05-02 | WASM CI check for all Layer 0 crates | CI: `cargo check --target wasm32-unknown-unknown` |
| 18 | RULE-S21-01 | Three-layer socket validation in all test harnesses | PathGuard enforced in test fixtures |
| 19 | RULE-S19-01 | Dual OTEL providers: server and client | Architecture test |
| 20 | RULE-S20-01 | File-based locking with `Drop` guard for parallel builds | `LockGuard` pattern enforced |
| 21 | RULE-S20-02 | Atomic build publication via rename | CI grep for `std::fs::copy` absence |
| 22 | RULE-S14-01 | Lock file guard via `Drop` impl, never manual unlock | CI grep: no manual `unlock()` calls |
| 23 | RULE-S14-02 | Config load trigger must remain post-identify | Startup sequence test |
| 24 | RULE-S22-01 | Test harnesses use `-f /dev/null` for config isolation | Test fixture enforcement |
| 25 | RULE-S18-01 | Permission hardening: socket dir `0700`, socket `0600` | Test assertions |
| 26 | RULE-S03-03 | `mux-termlet` must not depend on `mux-server` or `mux-api` | `cargo tree` CI check |
| 27 | RULE-S04-03 | `mux-termlet` lives at Layer 2, depends on Layer 0 + Layer 1 only | `cargo deny check` |
| 28 | RULE-S16-03 | Termlet bindings expose: spawn, send_keys, wait_for, snapshot, resize, kill | API surface test |
| 29 | RULE-S22-03 | Termlet fixtures must auto-kill on cleanup | Leak detection test |
| 30 | RULE-S22-04 | Termlet tests must run in both real and fake PTY modes | Parametrized fixture |
| 31 | RULE-S32-01 | Termlet snapshot text must be trailing-whitespace-trimmed | Snapshot format test |
| 32 | RULE-S32-02 | Termlet kill must be idempotent | Double-kill test |
| 33 | RULE-S32-03 | Termlet Drop must send SIGKILL to prevent PTY process leaks | Drop impl audit |
| 34 | RULE-S32-04 | FakePtyBackend tests must not depend on timing | Determinism check |
| 35 | RULE-S32-05 | Termlet bindings must implement context manager / cleanup pattern | Binding API test |
| 36 | RULE-S32-06 | `mux-termlet` must not depend on `mux-server`, `mux-api`, or `mux-orm` | `cargo tree` CI |
| 37 | RULE-S32-07 | Every Termlet API method tested in Rust + at least one binding | Test coverage |
| 38 | RULE-S08-03 | Termlet error codes must be stable string constants | Semver check |
| 39 | RULE-S32-08 | `wait_for` must compile pattern once, not per poll iteration | PatternMatcher audit |
| 40 | RULE-S32-09 | Termlet remains pane-backed; never invent parallel terminal core | Architecture review |
| 41 | RULE-S32-10 | Cross-language snapshot consistency tests must exist for every release | Multi-runner CI |
| 42 | RULE-S02-03 | Gate pass/fail thresholds must be measurable, not subjective | Acceptance criteria table |
| 43 | RULE-S32-11 | `TermletBuilder` and `TermletConfig` must produce identical Termlets | Unit test |
| 44 | RULE-S32-12 | `SnapshotDiff` must not allocate on identical snapshots | Benchmark |
| 45 | RULE-S08-04 | **[v11]** Every error type crossing binding boundaries must implement `ErrorCode` | Compile-fail test |
| 46 | RULE-S09-02 | **[v11]** Payload size must be bounded by `MAX_PAYLOAD_SIZE` | Decode check |
| 47 | RULE-S16-04 | **[v11]** Error messages in bindings must include `[ERROR_CODE]` prefix | Exception message format test |
| 48 | RULE-S19-04 | **[v11]** Termlet spans must include structured attributes | Attribute presence test |
| 49 | RULE-S21-03 | **[v11]** `PtyBackend` trait must require `Send + Any` bounds | Compile-fail test |
| 50 | RULE-S23-03 | **[v11]** Test names must match `test_{component}_{behavior}` convention | CI regex validation |
| 51 | RULE-S32-13 | **[v11]** `drain_output` must use nonblocking reads with `WouldBlock` handling | Code audit |
| 52 | RULE-S32-14 | **[v11]** `TermletPool` must use `HashMap<String, Termlet>` for named access | Type check |
| 53 | RULE-S32-15 | **[v11]** `Termlet::kill()` must implement full SIGTERM -> grace_period -> SIGKILL | Sequence test |
| 54 | RULE-S32-16 | **[v11]** `TermletPaneId` must be distinct from `PaneId` | Type system check |
| 55 | RULE-S32-17 | **[v11]** `output_history()` must return immutable reference, never clone | API signature check |
| 56 | RULE-S32-18 | **[v11]** `TermletExt` trait must not add state mutation methods | Trait method audit |
| 57 | RULE-S32-19 | **[v11]** `TermletPool::wait_all()` timeout must be per-Termlet, not total | Implementation check |
| 58 | RULE-S32-20 | **[v11]** `SnapshotDiff::compute()` returns empty diff for identical inputs | Zero-allocation test |
| 59 | RULE-S32-21 | **[v11 P2]** `TermletState` transitions must be monotonic | State transition test |
| 60 | RULE-S32-22 | **[v11 P2]** API methods must gate on `TermletState` (e.g., `send_keys` only in `Running`) | Method gating test |
| 61 | RULE-S32-23 | **[v11 P2]** `exit_status` must be populated when process exits | Exit status tracking test |
| 62 | RULE-S32-24 | **[v11 P2]** `inherit_env` default must be `false` for hermetic tests | Default config test |
| 63 | RULE-S32-25 | **[v11 P2]** Zero-timeout `wait_for` performs single check, not error | Zero-timeout semantics test |
| 64 | RULE-S14-03 | **[v11 P2]** `ServerPhase` transitions must be monotonic | Debug assertion + test |
| 65 | RULE-S02-04 | **[v11 P2]** Every `INV-*` invariant must map to at least one `TST-*` test | CI traceability lint |
| 66 | RULE-S32-26 | **[v11 P2]** `RealPtyBackend` must use non-blocking IO for event reading | Architecture review + unit test |

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
| Termlet without `kill()` call | Leaked PTY processes | Context manager / `Drop` impl |
| `std::thread::sleep` in `wait_for` loop body | Busy-wait wastes CPU | Exponential backoff with configurable interval |
| Re-compiling pattern on each `wait_for` poll | Unnecessary allocation | `PatternMatcher::compile()` once |
| **[v11]** `SlotMap::KeyData::from_ffi()` for Termlet pane IDs | Abuse of SlotMap internals | `TermletPaneId` newtype |
| **[v11]** Blocking PTY reads in `drain_output` | Hangs on empty PTY | Nonblocking read with `WouldBlock` |
| **[v11]** `clone()` in `output_history()` | Unnecessary allocation for read-only access | Return `&[u8]` reference |
| **[v11 P2]** Boolean `alive`/`exited` for Termlet state | Insufficient for lifecycle gating | `TermletState` enum |
| **[v11 P2]** Passing `inherit_env: true` as default | Breaks test hermeticity | Default `false`, opt-in only |

### Test Strategy

1. Static rule checks in CI (grep/cargo-deny/compile-fail suites) -- CI job.
2. Rule coverage report links each rule to at least one automated test -- report generation.
3. Release gate fails when critical rules are unverified -- release script.
4. Anti-pattern grep checks run on every PR -- CI job.
5. **[v11]** All 66 rules have explicit enforcement mechanism -- manual audit.

### AGENTS.md Rules

- `RULE-S26-01`: Every architecture section must define enforceable rules. **Enforcement:** Section review checklist.
- `RULE-S26-02`: Rule IDs are immutable once published. **Enforcement:** Append-only rule table.

---

## 27. Risks and Mitigations [v11 P2 updated]

### Design Decisions

Risk register covers protocol drift, behavioral divergence, binding API stability, test isolation, observability, dependency management, Termlet-specific risks, and **[v11]** drain_output, TermletPaneId, pool batch operations. **[v11 P2]** Added risks for TermletState lifecycle complexity, inherit_env leakage, wait_max_interval tuning, exit_status race conditions, and ServerPhase misuse. Each risk has an impact level, probability, and specific mitigation.

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
| R25 | CRDT OpLog unbounded growth | Medium | Low | Periodic compaction via `compact_before()` |
| R26 | Python GIL blocking during Rust operations | Medium | Medium | Release GIL with `py.allow_threads()` |
| R27 | Node.js async operation cancellation | Low | Medium | Neon task cancellation handling |
| R28 | tmux protocol version negotiation | Medium | Low | Version field in identify burst |
| R29 | WASM CI gate false positive from dev dependencies | Low | Medium | Feature-gated dev-deps |
| R30 | OTEL tokio runtime lifecycle blocking process exit | Medium | Medium | Bounded `shutdown_timeout(200ms)` |
| R31 | Termlet PTY process leak on crash/panic | High | Medium | `Drop` impl that kills PTY, RAII guard |
| R32 | FakePty Termlet diverges from real PTY Termlet | Medium | High | Dual-mode parametrized tests, fixture regen |
| R33 | Termlet `wait_for` busy-loop exhausts CPU | Medium | Low | Exponential backoff with configurable poll interval |
| R34 | Binding Termlet GC timing causes delayed kill | Medium | Medium | Explicit `kill()` in fixture teardown |
| R35 | Termlet grid reflow on resize loses scroll history | Low | Medium | Document limitation; snapshot before resize |
| R36 | Termlet API semantics diverge across languages | Medium | Medium | Cross-language consistency tests |
| R37 | Compiled PatternMatcher cache grows unbounded | Low | Low | LRU eviction or per-Termlet scope |
| R38 | TermletBuilder defaults diverge from TermletConfig defaults | Low | Low | Single source of defaults, unit test parity |
| R39 | SnapshotDiff false positives from trailing whitespace | Low | Medium | Normalize whitespace before diff |
| R40 | **[v11]** drain_output blocks on real PTY if no data available | Medium | Medium | Nonblocking reads with `WouldBlock` handling |
| R41 | **[v11]** TermletPaneId collides with server PaneId space | Low | Very Low | Distinct type systems prevent mixing |
| R42 | **[v11]** TermletPool.wait_all() timeout cascading | Medium | Low | Per-Termlet timeout, not total |
| R43 | **[v11]** ErrorCode string changes break binding consumers | High | Low | Semver-major for code string changes |
| R44 | **[v11 P2]** TermletState lifecycle complexity causes API confusion | Medium | Medium | Clear documentation, state transition diagram, compile-time gating where possible |
| R45 | **[v11 P2]** `inherit_env: true` leaks host secrets into test environment | High | Low | Default `false`, code review for opt-in uses |
| R46 | **[v11 P2]** `wait_max_interval` too high causes slow test feedback | Low | Medium | Default 100ms, document tuning guidance |
| R47 | **[v11 P2]** `exit_status` race: process exits between state check and read | Low | Low | `drain_output()` captures exit event atomically |
| R48 | **[v11 P2]** `ServerPhase` enum extended without updating state machine | Medium | Low | Exhaustive match enforced by `#[non_exhaustive]` absence + clippy |

### Test Strategy

1. Risk-to-test mapping must be complete and machine-checked -- mapping file.
2. High-impact risks require at least one integration/parity test -- coverage check.
3. Quarterly risk review updates with regression evidence -- review schedule.
4. Termlet-specific risks R31-R39 tested via dedicated Termlet test suite -- test list.
5. **[v11]** Risks R40-R43 tested via specific unit tests -- test names.
6. **[v11 P2]** Risks R44-R48 tested via lifecycle and config tests.

### AGENTS.md Rules

- `RULE-S27-01`: New high-risk features must add mitigation tests before merge. **Enforcement:** PR template checklist.
- `RULE-S27-02`: Risk register IDs remain stable for auditability. **Enforcement:** Append-only risk table.

---

## 28. Plan Evolution and Changelog

### Design Decisions

This section documents the evolution from v4 through v11 Pass 2, providing traceability for architectural decisions.

### 28.1 Version History

| Version | Date | Lines | Sections | Key Changes |
|---|---|---|---|---|
| v4 | 2026-02-10 | 2519 | 26 | 6-model synthesis |
| v5 | 2026-02-10 | 1573 | 27 | 3-model refinement |
| v6 | 2026-02-10 | 5239 | 31 | 3-pass synthesis, comprehensive code examples |
| v7 | 2026-02-10 | ~5000 | 31 | Cross-model final synthesis |
| v8 Pass 3 | 2026-02-11 | 6612 | 31 | Triple-pass final synthesis |
| v9 Pass 3 Final | 2026-02-11 | 4190 | 31 | Definitive synthesis, north star |
| v10 Pass 3 Final | 2026-02-11 | ~5009 | 32 | DEFINITIVE: Termlets, TermletBuilder, TermletPool, SnapshotDiff |
| v11 | 2026-02-11 | ~5200 | 32 | DEFINITIVE: ErrorCode universalized, drain_output, TermletPaneId, kill sequence, 58 rules |
| **v11 Pass 2** | **2026-02-11** | **~5500+** | **32** | **Three-model cross-pollination. TermletState lifecycle, ServerPhase, inherit_env, wait_max_interval, exit_status, zero-timeout, CrossLangCase, normalize_output, PerfTarget, to_cells(), 66 rules, 48 risks** |

### 28.2 v11 Pass 2 Changelog

**Cross-pollinated from GPT:**
1. `TermletState` enum for explicit lifecycle state machine with monotonic transitions.
2. API method gating on state (`send_keys` only in `Running`, etc.).
3. `ServerPhase` enum for server lifecycle tracking.
4. `C6` gate for control-mode backpressure semantics parity.
5. `A3` gate for cross-section traceability lint.
6. `INV-*`/`API-*`/`PAR-*`/`OPS-*`/`TST-*` traceability ID convention.
7. Zero-timeout `wait_for` means single immediate check.
8. `wait_max_interval` separate from `wait_poll_interval`.
9. `exit_status` tracking on Termlet struct.
10. `to_cells()` for style-aware snapshot assertions.
11. `TermletError::InvalidRegex` as separate variant.
12. Remaining-time clamping: `sleep_for.min(remaining)`.
13. `PerfTarget` struct with `target_ns`/`hard_fail_ns`.
14. `CrossLangCase` for shared cross-language test fixtures.
15. `normalize_output`/`ParityResult` for parity testing.
16. `BindingQueryList<T>` trait for binding query wrappers.

**Cross-pollinated from Gemini:**
17. `inherit_env` field on `TermletConfig` (default `false`).
18. `SnapshotDiff::compare` with `context_lines` parameter.
19. Async pool methods (`spawn_async`, `get_async`).
20. Resize drains output before AND after backend resize.
21. Section 32.15 "Implementation Hints for LLMs".
22. `to_styled()` on `TermletSnapshot`.
23. `RULE-S32-26` for non-blocking IO requirement.

**Strengthened from Claude base:**
24. All 66 rules have explicit enforcement (was 58 rules in v11 Pass 1).
25. 48 risks (was 43 in v11 Pass 1).
26. `TermletError` expanded to 8 variants (was 7).
27. Conflict resolutions documented in Preamble.

### Rust Example

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecVersion {
    V4, V5, V6, V7, V8, V9Pass3Final,
    V10Pass1, V10Pass2, V10Pass3Final,
    V11, V11Pass2,
}

pub struct ChangelogItem {
    pub from: SpecVersion,
    pub to: SpecVersion,
    pub summary: &'static str,
}

pub const V11P2_CHANGES: &[ChangelogItem] = &[
    ChangelogItem { from: SpecVersion::V11, to: SpecVersion::V11Pass2,
        summary: "v11 P2: Three-model cross-pollination. TermletState lifecycle, \
            ServerPhase, inherit_env, wait_max_interval, exit_status, zero-timeout, \
            CrossLangCase, normalize_output, PerfTarget, 66 rules, 48 risks." },
];
```

### Test Strategy

1. Changelog assertions reference concrete tests and anchors -- mapping check.
2. No unresolved placeholders allowed in release spec -- grep for `todo!()`.
3. Spec self-check script validates section count and required subsections -- script.
4. Pass evolution table cross-references all unique ideas to sections -- table completeness.
5. **[v11 P2]** All 27 changelog items are traceable to specific sections -- traceability matrix.

### AGENTS.md Rules

- `RULE-S28-01`: Each version bump must list behavior-impacting changes. **Enforcement:** Changelog section review.
- `RULE-S28-02`: No TODO placeholders in finalized spec versions. **Enforcement:** CI grep.

---

## 29. Reference Anchors [v11 re-verified]

### Design Decisions

All compatibility claims in this specification are anchored to verified source code. Anchors are organized by codebase and include specific file paths and line numbers where available. All anchors were re-verified during v11.

### 29.1 tmux Source Code References

| File | Line(s) | Topic | Verified v11 |
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

### 29.2 Reference Codebase Verification

| Codebase | File | Claims Verified |
|---|---|---|
| libtmux | `_internal/query_list.py` | 12 operators in `LOOKUP_NAME_MAP` (lines 298-312), `eq` alias, `keygetter` nested traversal (lines 45-113), callable matcher in `filter()` (line 535), `get(default=no_arg)` semantics (lines 550-569), `MultipleObjectsReturned` / `ObjectDoesNotExist` exceptions |
| vibe-tmux | `mux-test-support/src/path_guard.rs` | 3-layer socket validation: `ensure_not_default_socket_name` (line 16), `ensure_socket_within_tempdir` (line 30), `ensure_socket_not_tmux_env` (line 45), `tmux_socket_path` format (line 76) |
| vibe-tmux | `tools/tmux-builder/src/lib.rs` | BLAKE3 `flags_fingerprint` with null separator (line 511), `hex_32` full 64-char hex (line 501), truncated to 12 chars (`&cfg[..12]` at line 543), `LockGuard` with `Drop` (lines 247-256), `sibling_tmp_dir` with pid+nanos (line 409), atomic `std::fs::rename` (line 389), `validate_tmux_binary` (line 549) |
| vibe-tmux | `crates/mux-otel/src/otel.rs` | Dual providers `OTEL_PROVIDER` / `MUX_CLIENT_PROVIDER` as `OnceLock<Mutex<Option<OtelProvider>>>` (lines 58-59), `OtelProvider` struct (lines 65-71), `enter_mux_client_span` lazy init (line 470), composite propagator (lines 771-777), `HeaderCarrier` (lines 998-1028), `force_flush` (line 533), `shutdown` with `.take()` (line 576), `runtime.shutdown_timeout(200ms)` (line 275) |

### Rust Example

```rust
pub struct Anchor {
    pub claim: &'static str,
    pub source: &'static str,
    pub file: &'static str,
    pub lines: Option<&'static str>,
    pub v11_verified: bool,
}

pub const ANCHORS: &[Anchor] = &[
    Anchor { claim: "protocol version 8",
        source: "tmux", file: "tmux-protocol.h", lines: Some("23"),
        v11_verified: true },
    Anchor { claim: "flock startup lock",
        source: "tmux", file: "client.c", lines: Some("77-101"),
        v11_verified: true },
    Anchor { claim: "12 query operators + keygetter",
        source: "libtmux", file: "query_list.py", lines: Some("298-312"),
        v11_verified: true },
    Anchor { claim: "3-layer socket validation",
        source: "vibe-tmux", file: "path_guard.rs", lines: Some("16,30,45"),
        v11_verified: true },
    Anchor { claim: "BLAKE3 cache key + file locking + 12-char truncation",
        source: "vibe-tmux", file: "tmux-builder/src/lib.rs", lines: Some("511,521,543"),
        v11_verified: true },
    Anchor { claim: "dual OTEL providers",
        source: "vibe-tmux", file: "mux-otel/src/otel.rs", lines: Some("58-59"),
        v11_verified: true },
];
```

### Test Strategy

1. Anchor checker script confirms referenced files exist -- script output.
2. Drift checker flags missing/renamed files -- CI job.
3. Parity suite maps failing behavior to anchor category -- test categorization.

### AGENTS.md Rules

- `RULE-S29-01`: Major compatibility claims require source anchor entries. **Enforcement:** Anchor presence check.
- `RULE-S29-02`: Remove stale anchors during refactors. **Enforcement:** CI drift checker.

---

## 30. Appendix: Canonical Type Quick Reference [v11 P2 updated]

### Design Decisions

This appendix provides a consolidated reference for all canonical type names used across the specification. **[v11]** Added `TermletPaneId`, `TermletExt`, `TermletPoolViewModel`, `EventMsg` types. **[v11 P2]** Added `TermletState`, `ServerPhase`, `PerfTarget`, `CrossLangCase`, `ParityResult`, `BindingQueryList`.

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

// [v11] Termlet uses distinct ID type, not SlotMap
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TermletPaneId(u64);

impl TermletPaneId {
    pub fn next() -> Self { /* AtomicU64 counter */ }
}
```

### 30.2 Core Types

| Type | Crate | Purpose |
|---|---|---|
| `ServerGraph` | mux-core | Authoritative mutable state |
| `GraphState` | mux-core | Immutable snapshot with reverse maps (HashMap-backed) |
| `Event` | mux-core | Inbound state change request |
| `Effect` | mux-core | Outbound side-effect request |
| `CoreCtx` | mux-core | Injected time + randomness |
| `ApplyOutcome` | mux-core | Return from apply_event (effects + hints + warnings) |
| `MuxKernel` | mux-core | Core dispatch trait |
| `ServerPhase` | mux-core | **[v11 P2]** Server lifecycle phase enum |
| `Grid` | mux-grid | Terminal cell grid |
| `VtParser` | mux-grid | VT100 state machine |
| `ImsgHdr` | mux-proto | Wire protocol header |
| `ImsgFrame` | mux-proto | Header + payload |
| `ImsgCodec` | mux-proto | Stateful encoder/decoder |
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
| `BindingQueryList<T>` | mux-query | **[v11 P2]** Binding query wrapper trait |
| `HLC` | mux-crdt | Hybrid logical clock |
| `LWWRegister<T>` | mux-crdt | Last-writer-wins register |
| `OrSet<T>` | mux-crdt | Observed-remove set (add-wins) |
| `OpLog` | mux-crdt | Operation log for merge/replay |
| `ControlNotification` | mux-control | Typed control mode notification |
| `PtyBackend` | mux-pty | PTY abstraction trait (Send + Any) |
| `FakePtyBackend` | mux-pty-fake | Test PTY implementation |
| `StateHandle` | mux-api | ArcSwap-based read handle |
| `ManagedMux` | mux-api | Server lifecycle manager |
| `TermForgeStack` | mux-api | Composition for in-process embedding |
| `EventMsg` | mux-server | **[v11]** Event with optional reply channel |
| `OtelProvider` | mux-otel | OTEL trace + log provider |
| `TraceHeaders` | mux-otel | traceparent + tracestate + baggage |
| `TraceHeadersGuard` | mux-otel | RAII guard for thread-local headers |
| `PathGuard` | mux-test-support | Socket isolation for tests |
| `LockGuard` | tools/tmux-builder | File lock with Drop |
| `ViewModel` | mux-view | Pure view model for TUI |
| `TermletPoolViewModel` | mux-view | **[v11]** Multi-Termlet inspector |
| `PerfTarget` | mux-bench | **[v11 P2]** Performance target with thresholds |
| `CrossLangCase` | mux-test-support | **[v11 P2]** Shared cross-language test fixture |
| `ParityResult` | mux-test-support | **[v11 P2]** Parity comparison result |
| **`Termlet`** | **mux-termlet** | SDK-first testing pod handle |
| **`TermletConfig`** | **mux-termlet** | Configuration for Termlet spawn |
| **`TermletState`** | **mux-termlet** | **[v11 P2]** Lifecycle state enum |
| **`TermletSnapshot`** | **mux-termlet** | Captured grid state |
| **`PtyMode`** | **mux-termlet** | Real vs Fake PTY backend selection |
| **`WaitMatch`** | **mux-termlet** | Match result with byte range + timestamp |
| **`PatternMatcher`** | **mux-termlet** | Compiled pattern for efficient polling |
| **`ScenarioStep`** | **mux-pty-fake** | Structured FakePty fixture step |
| **`TermletBuilder`** | **mux-termlet** | Fluent construction for Termlets |
| **`TermletPool`** | **mux-termlet** | Batch Termlet management for test suites |
| **`SnapshotDiff`** | **mux-termlet** | Visual regression diff between snapshots |
| **`TermletPaneId`** | **mux-termlet** | **[v11]** Distinct pane ID for Termlets |
| **`TermletExt`** | **mux-termlet** | **[v11]** Custom assertion extension trait |
| **`TermletViewModel`** | **mux-view** | Debug inspector panel model |

### 30.3 Error Types

| Type | Crate | ErrorClass | ErrorCode impl? |
|---|---|---|---|
| `ProtocolError` | mux-proto | ProtocolViolation | **[v11]** Yes |
| `CoreError` | mux-core | UserError / Bug | **[v11]** Yes |
| `ConfigError` | mux-conf | UserError | No (internal only) |
| `LayoutError` | mux-core | Bug | No (internal only) |
| `QueryError` | mux-query | UserError | **[v11]** Yes |
| `PtyError` | mux-pty | Transient | No (wrapped by TermletError) |
| `SocketError` | mux-os | Transient / UserError | No (internal only) |
| `LockError` | mux-os | Transient | No (internal only) |
| `ControlParseError` | mux-control | UserError | No (internal only) |
| **`TermletError`** | **mux-termlet** | **Transient / UserError** | **Yes (8 variants)** |

### Test Strategy

1. Type API compile checks in integration crates -- compile test.
2. Semver surface snapshot for public crates -- snapshot test.
3. Doc tests for all appendix examples -- `cargo doc --test`.
4. **[v11]** ErrorCode column accuracy: every "Yes" has a corresponding impl -- compile test.
5. **[v11 P2]** All new types (`TermletState`, `ServerPhase`, `PerfTarget`, etc.) present in exports -- API surface test.

### AGENTS.md Rules

- `RULE-S30-01`: Do not rename canonical types without migration notes. **Enforcement:** Semver check.
- `RULE-S30-02`: Appendix must match actual exported APIs. **Enforcement:** API surface test.

---

## 31. Supplemental Test Matrix [v11 P2 updated]

### Design Decisions

Test matrix extends section-local tests with release gates. Organized by test category, runner, and CI enforcement level. **[v11 P2]** Added lifecycle, exit_status, inherit_env, zero-timeout, and ServerPhase test rows.

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
| **Termlet unit (Rust)** | ~35 | `cargo test` | Yes |
| **Termlet integration** | ~25 | `cargo test` | Yes |
| **Termlet Python** | ~15 | `pytest` | Yes |
| **Termlet Node** | ~10 | `vitest` | Yes |
| **Termlet performance** | ~3 | `criterion` | Yes (warn only) |
| **Cross-language consistency** | ~10 | `pytest` + `vitest` + `cargo test` | Yes |
| **TermletBuilder parity** | ~5 | `cargo test` | Yes |
| **SnapshotDiff regression** | ~10 | `cargo test` | Yes |

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
  test_termlet_spawn_fake
  test_termlet_spawn_real
  test_termlet_send_keys_echo
  test_termlet_wait_for_timeout
  test_termlet_wait_for_zero_timeout     # [v11 P2]
  test_termlet_wait_for_pattern_not_found
  test_termlet_state_lifecycle           # [v11 P2]
  test_termlet_exit_status              # [v11 P2]
  test_termlet_inherit_env              # [v11 P2]
  test_termlet_snapshot_text
  test_termlet_snapshot_to_cells        # [v11 P2]
  test_termlet_resize_reflow
  test_termlet_kill_idempotent
  test_termlet_drop_kills_pty
  test_termlet_pattern_matcher_compile
  test_termlet_builder_parity
  test_termlet_pool_batch
  test_termlet_snapshot_diff
  test_termlet_snapshot_diff_context    # [v11 P2]
  test_binding_py_termlet_fixture
  test_binding_node_termlet_helper
  test_cross_lang_termlet_snapshot
  test_server_phase_monotonic           # [v11 P2]
```

### 31.4 Coverage Requirements

| Crate | Target | Enforcement |
|---|---|---|
| mux-termlet | 85% | CI gate |
| mux-core | 80% | CI gate |
| bindings/python | 80% | CI gate |
| bindings/node | 70% | CI gate |

### 31.5 Release Gate Matrix

| Gate ID | Gate | Test Type | Required? |
|---|---|---|---|
| M-09-01 | Protocol fuzz | cargo-fuzz | Nightly |
| M-12-01 | Query kwargs parity | unit test | Yes |
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
| M-32-01 | Termlet spawn (fake+real) | unit test | Yes |
| M-32-02 | Termlet send_keys + snapshot | unit test | Yes |
| M-32-03 | Termlet resize + reflow | unit test | Yes |
| M-32-04 | PyTermlet fixture chain | pytest | Yes |
| M-32-05 | JsTermlet helper | vitest | Yes |
| M-32-06 | Cross-language snapshot consistency | multi-runner | Yes |
| M-32-07 | PatternMatcher compile efficiency | unit test | Yes |
| M-32-08 | TermletBuilder produces same Termlet as TermletConfig | unit test | Yes |
| M-32-09 | SnapshotDiff detects known differences | unit test | Yes |
| M-32-10 | **[v11]** Error code stability across all types | unit test | Yes |
| M-32-11 | **[v11]** drain_output nonblocking behavior | unit test | Yes |
| M-32-12 | **[v11]** TermletPool batch operations | unit test | Yes |
| M-32-13 | **[v11]** kill() full SIGTERM->SIGKILL sequence | unit test | Yes |
| M-32-14 | **[v11 P2]** TermletState lifecycle transitions | unit test | Yes |
| M-32-15 | **[v11 P2]** exit_status captured on process exit | unit test | Yes |
| M-32-16 | **[v11 P2]** Zero-timeout wait_for semantics | unit test | Yes |
| M-32-17 | **[v11 P2]** ServerPhase monotonic progression | unit test | Yes |
| M-32-18 | **[v11 P2]** CrossLangCase fixture parity | multi-runner | Yes |

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
    // Termlet gates
    MatrixRow { id: "M-32-01", section: 32, test_name: "termlet_spawn_dual_mode", required: true },
    MatrixRow { id: "M-32-02", section: 32, test_name: "termlet_send_snapshot", required: true },
    MatrixRow { id: "M-32-03", section: 32, test_name: "termlet_resize_reflow", required: true },
    MatrixRow { id: "M-32-04", section: 32, test_name: "py_termlet_fixture", required: true },
    MatrixRow { id: "M-32-05", section: 32, test_name: "js_termlet_helper", required: true },
    MatrixRow { id: "M-32-06", section: 32, test_name: "cross_lang_snapshot", required: true },
    // [v11 P2] New gates
    MatrixRow { id: "M-32-14", section: 32, test_name: "termlet_state_lifecycle", required: true },
    MatrixRow { id: "M-32-15", section: 32, test_name: "termlet_exit_status", required: true },
    MatrixRow { id: "M-32-16", section: 32, test_name: "termlet_zero_timeout", required: true },
    MatrixRow { id: "M-32-17", section: 5, test_name: "server_phase_monotonic", required: true },
    MatrixRow { id: "M-32-18", section: 16, test_name: "cross_lang_case_parity", required: true },
];
```

### Test Strategy

1. CI requires 100% pass of mandatory matrix rows -- CI gate.
2. Nightly includes extended parity + fuzz durations -- nightly CI config.
3. Release requires zero unresolved mandatory rows -- release gate script.
4. Termlet gates M-32-01 through M-32-18 are mandatory for release -- gate list.

### AGENTS.md Rules

- `RULE-S31-01`: New architecture features must add at least one matrix row. **Enforcement:** PR template.
- `RULE-S31-02`: Release tagging is blocked until mandatory rows are green. **Enforcement:** Release script.

---

## 32. Termlets [v10 new, v11 DEFINITIVE, v11 P2 three-model synthesis]

### Design Decisions

Termlets are the killer feature differentiating TermForge from other terminal multiplexers and testing tools. They are **SDK-first testing pods** that wrap tmux panes into simplified, embeddable, language-binding-native handles.

**Core philosophy:** A Termlet is to a tmux pane what a Docker container is to a VM -- same capability, simpler interface, faster lifecycle, purpose-built for programmatic use.

**Key design decisions:**

1. **Testing pods**: A Termlet wraps a Pane + Grid + PtyBackend into a single ergonomic handle. Unlike a full multiplexer session, a Termlet does not require a `ServerGraph`, a `StateActor`, a socket, or a running server. It is self-contained.

2. **SDK-first**: The API is designed for programmatic use, not interactive terminal use. Every operation returns a `Result`. Every state is queryable. Every output is capturable.

3. **Visual area captured**: Each Termlet owns a `Grid` that accumulates VT100 output from the underlying PTY. The grid can be snapshotted at any time, producing a text representation suitable for assertion or insta snapshot testing.

4. **Resizable**: Programmatic resize via `resize(cols, rows)` triggers PTY `SIGWINCH` and grid reflow. **[v11 P2]** Resize drains output before AND after backend resize to capture immediate SIGWINCH response (from Gemini).

5. **Interactive**: A Termlet holds a shell session (or any command). It accepts input via `send_keys()` and produces output that flows through the VT100 parser into the grid.

6. **Available everywhere**: Rust core (`mux_termlet::Termlet`), Python (`termforge.Termlet` via PyO3), Node.js (`termforge.useTermlet()` via Neon). Same semantics, language-native ergonomics.

7. **Backward compatible**: A Termlet IS a pane under the hood. The `Grid`, `VtParser` -- all reused from `mux-grid`. The Termlet just provides a simpler creation and interaction API.

8. **Process management**: Spawn shells, run commands, send signals, detect exit. The Termlet owns the child process lifecycle and cleans up via `Drop`. Kill uses graceful-then-forced sequence: SIGTERM first, wait `grace_period` (default 2s), then SIGKILL. **[v11]** `kill()` implements the full SIGTERM -> grace_period -> SIGKILL sequence.

9. **The ultimate subprocess runner**: Where `std::process::Command` gives you stdout/stderr as byte streams, a Termlet gives you a full terminal emulation. You can test interactive programs, TUI apps, shell scripts with prompts, and anything that uses terminal escape sequences.

10. **Lite**: Minimal overhead. Creating a Termlet with FakePtyBackend takes < 500us. Creating one with a real PTY takes < 50ms. Destroying one is instant (kill signal + drop).

11. **Snapshot-testable**: `insta::assert_snapshot!(termlet.snapshot().to_text())` in Rust. `assert termlet.snapshot() == expected` in Python. `expect(termlet.snapshot()).toMatchSnapshot()` in vitest.

12. **Dual-mode**: `PtyMode::Real` for integration tests against real shells. `PtyMode::Fake` for deterministic unit tests with injected output.

13. **Compiled pattern matching**: `wait_for` compiles the pattern once via `PatternMatcher` and reuses the compiled form across poll iterations, avoiding per-iteration allocation. Supports plain string and regex modes.

14. **Structured wait result**: `wait_for` returns `WaitMatch` carrying the byte range of the match and the timestamp, enabling precise assertions.

15. **Process exit detection**: `PatternNotFound` error variant signals that the child process exited before the pattern appeared, distinct from `WaitForTimeout` which means the deadline elapsed.

16. **Fluent construction**: `TermletBuilder` provides a builder pattern alternative to `TermletConfig` for ergonomic Termlet construction.

17. **Batch management**: `TermletPool` manages multiple Termlets for test suites that need several concurrent terminal sessions.

18. **Visual regression**: `SnapshotDiff` provides structured comparison between two snapshots for visual regression testing.

19. **[v11] Nonblocking drain**: `drain_output()` uses nonblocking PTY reads with `WouldBlock` handling to pull available output without blocking.

20. **[v11] Output history**: `output_history()` returns an immutable `&[u8]` reference to all raw bytes received from the PTY, enabling assertions on raw escape sequences.

21. **[v11] TermletPaneId**: Distinct newtype from server-managed `PaneId`. Prevents accidental mixing of Termlet-internal pane IDs with server entity IDs.

22. **[v11] TermletExt trait**: Extension trait for custom assertion methods that can be added by downstream crates without modifying `mux-termlet`.

23. **[v11] Pane bridge API**: `Termlet::attach_to_server()` allows bridging a standalone Termlet into a server-managed pane when integration is needed.

24. **[v11 P2] TermletState lifecycle**: Explicit state machine `Created -> Spawning -> Running -> Stopping -> Exited` with monotonic transitions and API method gating (from GPT).

25. **[v11 P2] exit_status tracking**: `Option<i32>` on Termlet struct, populated when process exits, available for assertion on return codes (from GPT).

26. **[v11 P2] inherit_env**: `TermletConfig.inherit_env: bool` (default `false` for hermetic tests, opt-in for integration) (from Gemini).

27. **[v11 P2] wait_max_interval**: Separate config for maximum backoff interval in `wait_for` (from GPT). Default 100ms.

28. **[v11 P2] Zero-timeout semantics**: `wait_for` with zero timeout performs single immediate check, returns match or error (from GPT).

29. **[v11 P2] to_cells()**: Style-aware snapshot method for asserting on terminal colors and attributes (from GPT).

30. **[v11 P2] to_styled()**: ANSI-escape rendering of snapshot for human-readable debug output (from Gemini).

31. **[v11 P2] spawn_with_backend()**: Alternative spawn method that takes backend as parameter for explicit dependency injection (inspired by GPT).

**Architectural position:** `mux-termlet` sits at Layer 2 (FACADE). It depends on:
- `mux-types` (Layer 0) for shared types, `PaneSize`
- `mux-grid` (Layer 0) for `Grid`, `VtParser`
- `mux-pty` (Layer 1) for `PtyBackend` trait and real implementation
- `mux-pty-fake` (Layer 0) for `FakePtyBackend`

It does NOT depend on: `mux-server`, `mux-api`, `mux-orm`.

### 32.1 Architecture Diagram

```
                    +------------------+
                    |   User Code      |
                    | (Rust / Py / JS) |
                    +--------+---------+
                             |
                    spawn / send_keys / snapshot / resize / kill
                             |
                    +--------v---------+
                    |     Termlet      |  <-- mux-termlet crate (Layer 2)
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

**Implementation Note:** `RealPtyBackend` MUST use non-blocking IO for `read_events` to prevent `wait_for` (which loops and drains output) from blocking indefinitely if the PTY has no output available.

### 32.2 TermletState Lifecycle [v11 P2 new from GPT]

```text
Created -> Spawning -> Running -> Stopping -> Exited
                 \-> SpawnFailed
Running --kill()--> Stopping --grace timeout--> ForcedKill -> Exited
Running --process exit event---------------------------> Exited
```

State transitions are **monotonic** -- a Termlet can never move backward. API methods gate on state:
- `send_keys`, `resize`, `wait_for` allowed only in `Running`.
- `snapshot` allowed in `Running` and `Exited`.
- `kill` idempotent in `Stopping`/`Exited`.
- `output_history` allowed in any state after `Spawning`.

### 32.3 Core API

```rust
// crates/mux-termlet/src/lib.rs
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicU64, Ordering};

/// [v11] Distinct ID type for Termlet-managed panes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TermletPaneId(u64);

static TERMLET_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

impl TermletPaneId {
    pub fn next() -> Self {
        Self(TERMLET_ID_COUNTER.fetch_add(1, Ordering::Relaxed))
    }
    pub fn raw(self) -> u64 { self.0 }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtyMode { Real, Fake }

/// [v11 P2] Explicit lifecycle state machine from GPT.
/// Monotonic: states only advance forward.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TermletState {
    Created,
    Spawning,
    Running,
    Stopping,
    Exited,
}

#[derive(Debug, Clone)]
pub struct TermletConfig {
    pub cols: u16,
    pub rows: u16,
    pub env: Vec<(String, String)>,
    /// [v11 P2] Whether to inherit parent process environment (from Gemini).
    /// Default: false (hermetic).
    pub inherit_env: bool,
    pub cwd: Option<String>,
    pub backend: PtyMode,
    /// Maximum time to wait between SIGTERM and SIGKILL during kill().
    pub grace_period: Duration,
    /// Initial interval between poll iterations in wait_for.
    pub wait_poll_interval: Duration,
    /// [v11 P2] Maximum backoff interval for wait_for (from GPT).
    pub wait_max_interval: Duration,
}

impl Default for TermletConfig {
    fn default() -> Self {
        Self {
            cols: 80,
            rows: 24,
            env: vec![("TERM".to_string(), "xterm-256color".to_string())],
            inherit_env: false,
            cwd: None,
            backend: PtyMode::Real,
            grace_period: Duration::from_secs(2),
            wait_poll_interval: Duration::from_millis(25),
            wait_max_interval: Duration::from_millis(100),
        }
    }
}

pub struct Termlet {
    id: TermletPaneId,
    state: TermletState,          // [v11 P2] replaces boolean `alive`
    grid: Grid,
    parser: VtParser,
    backend: Box<dyn PtyBackend>,
    config: TermletConfig,
    /// [v11] Raw byte accumulator for output_history().
    output_bytes: Vec<u8>,
    /// [v11 P2] Exit code from process, populated on exit.
    exit_status: Option<i32>,
}

impl Termlet {
    /// Spawn a new Termlet running the given command.
    pub fn spawn(command: &str, mut config: TermletConfig) -> Result<Self, TermletError> {
        let id = TermletPaneId::next();
        let grid = Grid::new(config.cols, config.rows);
        let parser = VtParser::new();

        let mut backend: Box<dyn PtyBackend> = match config.backend {
            PtyMode::Real => Box::new(RealPtyBackend::new()
                .map_err(|e| TermletError::SpawnFailed { reason: e.to_string() })?),
            PtyMode::Fake => Box::new(FakePtyBackend::new()),
        };

        // [v11 P2] inherit_env: merge parent env (excluding overrides) from Gemini.
        if config.inherit_env {
            for (k, v) in std::env::vars() {
                if !config.env.iter().any(|(ek, _)| ek == &k) {
                    config.env.push((k, v));
                }
            }
        }

        let pane_size = PaneSize { sx: config.cols, sy: config.rows };
        let cmd = vec![command.to_string()];
        backend.spawn(
            id, &cmd, config.cwd.as_deref(), pane_size,
        ).map_err(|e| TermletError::SpawnFailed { reason: e.to_string() })?;

        Ok(Self {
            id, state: TermletState::Running,
            grid, parser, backend, config,
            output_bytes: Vec::new(),
            exit_status: None,
        })
    }

    /// [v11 P2] Spawn with explicit backend for dependency injection (inspired by GPT).
    pub fn spawn_with_backend(
        command: &str,
        mut config: TermletConfig,
        mut backend: Box<dyn PtyBackend>,
    ) -> Result<Self, TermletError> {
        let id = TermletPaneId::next();
        let grid = Grid::new(config.cols, config.rows);
        let parser = VtParser::new();

        if config.inherit_env {
            for (k, v) in std::env::vars() {
                if !config.env.iter().any(|(ek, _)| ek == &k) {
                    config.env.push((k, v));
                }
            }
        }

        let pane_size = PaneSize { sx: config.cols, sy: config.rows };
        let cmd = vec![command.to_string()];
        backend.spawn(
            id, &cmd, config.cwd.as_deref(), pane_size,
        ).map_err(|e| TermletError::SpawnFailed { reason: e.to_string() })?;

        Ok(Self {
            id, state: TermletState::Running,
            grid, parser, backend, config,
            output_bytes: Vec::new(),
            exit_status: None,
        })
    }

    /// Send keystrokes to the Termlet.
    /// [v11 P2] Gated: only allowed in Running state.
    pub fn send_keys(&mut self, keys: &str) -> Result<(), TermletError> {
        if self.state != TermletState::Running {
            return Err(TermletError::AlreadyExited);
        }
        self.backend.write(self.id, keys.as_bytes())
            .map_err(|e| TermletError::Pty(e))?;
        self.drain_output();
        Ok(())
    }

    /// [v11] Drain all available output from the PTY into the grid.
    /// Uses nonblocking reads: returns when no more data is immediately available.
    pub fn drain_output(&mut self) {
        let events = self.backend.read_events();
        for event in events {
            match event {
                PtyEvent::Output { data, .. } => {
                    self.output_bytes.extend_from_slice(&data);
                    self.parser.parse(&data, &mut self.grid);
                }
                PtyEvent::Exited { status, .. } => {
                    self.state = TermletState::Exited;
                    self.exit_status = Some(status); // [v11 P2]
                }
            }
        }
    }

    /// Wait for a pattern to appear in the grid text.
    /// [v11 P2] Zero timeout = single immediate check (from GPT).
    /// [v11 P2] Remaining-time clamping prevents oversleep (from GPT).
    pub fn wait_for(
        &mut self,
        pattern: &str,
        timeout: Duration,
    ) -> Result<WaitMatch, TermletError> {
        if self.state != TermletState::Running && self.state != TermletState::Exited {
            return Err(TermletError::AlreadyExited);
        }

        let matcher = PatternMatcher::compile(pattern)?;

        // [v11 P2] Zero-timeout: single immediate check (from GPT).
        if timeout.is_zero() {
            self.drain_output();
            let text = self.snapshot_text();
            if let Some(range) = matcher.find(&text) {
                return Ok(WaitMatch {
                    byte_range: range,
                    matched_at: Instant::now(),
                });
            }
            return Err(TermletError::WaitForTimeout {
                pattern: pattern.to_string(),
                timeout_ms: 0,
            });
        }

        let start = Instant::now();
        let mut sleep_for = self.config.wait_poll_interval;

        loop {
            self.drain_output();
            let text = self.snapshot_text();

            if let Some(range) = matcher.find(&text) {
                return Ok(WaitMatch {
                    byte_range: range,
                    matched_at: Instant::now(),
                });
            }

            if self.state == TermletState::Exited {
                return Err(TermletError::PatternNotFound {
                    pattern: pattern.to_string(),
                });
            }

            if start.elapsed() >= timeout {
                return Err(TermletError::WaitForTimeout {
                    pattern: pattern.to_string(),
                    timeout_ms: timeout.as_millis() as u64,
                });
            }

            // [v11 P2] Clamp sleep to remaining time to avoid oversleep (from GPT).
            let remaining = timeout.saturating_sub(start.elapsed());
            std::thread::sleep(sleep_for.min(remaining));
            // Exponential backoff capped at wait_max_interval.
            sleep_for = (sleep_for * 2).min(self.config.wait_max_interval);
        }
    }

    /// Capture a snapshot of the current grid state.
    /// Allowed in Running and Exited states.
    pub fn snapshot(&mut self) -> TermletSnapshot {
        self.drain_output();
        TermletSnapshot {
            lines: self.grid.rows_text(),
            cols: self.config.cols,
            rows: self.config.rows,
            timestamp: Instant::now(),
        }
    }

    /// Internal helper: get text without constructing full snapshot.
    fn snapshot_text(&self) -> String {
        self.grid.to_text_trimmed()
    }

    /// Resize the Termlet.
    /// [v11 P2] Drains output before AND after resize (from Gemini).
    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<(), TermletError> {
        if self.state != TermletState::Running {
            return Err(TermletError::AlreadyExited);
        }

        // Drain before resize to process any pending output
        self.drain_output();

        let new_size = PaneSize { sx: cols, sy: rows };
        self.backend.resize(self.id, new_size)
            .map_err(|e| TermletError::ResizeFailed(e))?;
        self.grid.resize(cols, rows);
        self.config.cols = cols;
        self.config.rows = rows;

        // Drain after resize to catch immediate SIGWINCH response
        self.drain_output();
        Ok(())
    }

    /// [v11] Kill with full SIGTERM -> grace_period -> SIGKILL sequence.
    /// Idempotent: calling kill() on already-exited Termlet returns Ok(()).
    pub fn kill(&mut self) -> Result<(), TermletError> {
        if self.state == TermletState::Exited {
            return Ok(()); // Idempotent
        }

        self.state = TermletState::Stopping;

        // Step 1: Send SIGTERM
        let _ = self.backend.kill(self.id, libc::SIGTERM);
        self.drain_output();

        // Step 2: Wait grace_period for natural exit
        let deadline = Instant::now() + self.config.grace_period;
        while self.state != TermletState::Exited && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(50));
            self.drain_output();
        }

        // Step 3: Force SIGKILL if still alive
        if self.state != TermletState::Exited {
            let _ = self.backend.kill(self.id, libc::SIGKILL);
            self.state = TermletState::Exited;
        }

        Ok(())
    }

    pub fn is_alive(&self) -> bool { self.state == TermletState::Running }
    pub fn state(&self) -> TermletState { self.state }
    pub fn size(&self) -> PaneSize { PaneSize { sx: self.config.cols, sy: self.config.rows } }
    pub fn pane_id(&self) -> TermletPaneId { self.id }

    /// [v11] Get immutable reference to all raw bytes received from PTY.
    pub fn output_history(&self) -> &[u8] { &self.output_bytes }

    /// [v11 P2] Get exit status of the underlying process, if exited.
    pub fn exit_status(&self) -> Option<i32> { self.exit_status }

    /// [v11] Access FakePtyBackend for test injection.
    #[cfg(any(test, feature = "test-support"))]
    pub fn fake_backend_mut(&mut self) -> Option<&mut FakePtyBackend> {
        self.backend.as_any_mut().downcast_mut::<FakePtyBackend>()
    }
}

impl Drop for Termlet {
    fn drop(&mut self) {
        if self.state != TermletState::Exited {
            // Emergency kill: SIGKILL immediately to prevent process leak
            let _ = self.backend.kill(self.id, libc::SIGKILL);
            self.state = TermletState::Exited;
        }
    }
}
```

### 32.4 PatternMatcher

```rust
// crates/mux-termlet/src/pattern.rs
use regex::Regex;
use std::ops::Range;

#[derive(Debug, Clone)]
pub enum PatternMatcher {
    Plain(String),
    Regex(Regex),
}

impl PatternMatcher {
    /// Compile pattern once. `re:` prefix triggers regex mode.
    /// [v11 P2] Returns InvalidRegex (separate variant) on bad regex.
    pub fn compile(raw: &str) -> Result<Self, TermletError> {
        if let Some(rest) = raw.strip_prefix("re:") {
            let re = Regex::new(rest)
                .map_err(|e| TermletError::InvalidRegex(e.to_string()))?;
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
```

### 32.5 TermletSnapshot [v11 P2 updated]

```rust
// crates/mux-termlet/src/snapshot.rs

/// Captured grid state at a point in time.
/// [v11 P2] Uses Vec<String> lines internally (from GPT) for efficient line ops.
#[derive(Debug, Clone)]
pub struct TermletSnapshot {
    lines: Vec<String>,
    pub cols: u16,
    pub rows: u16,
    pub timestamp: Instant,
}

impl TermletSnapshot {
    /// Convert to plain text with trailing whitespace trimmed.
    pub fn to_text(&self) -> String {
        let mut trimmed: Vec<&str> = self.lines.iter()
            .map(|l| l.trim_end())
            .collect();
        while trimmed.last().is_some_and(|l| l.is_empty()) {
            trimmed.pop();
        }
        trimmed.join("\n")
    }

    /// [v11 P2] Style-aware cell access for color/attribute assertions (from GPT).
    pub fn to_cells(&self) -> Vec<Vec<Cell>> {
        // Returns grid cells with style information preserved.
        // Implementation reads from Grid directly.
        todo!()
    }

    /// [v11 P2] ANSI-escaped rendering for human-readable debug output (from Gemini).
    pub fn to_styled(&self) -> String {
        // Renders cells with ANSI escape sequences for terminal display.
        todo!()
    }

    /// Check if snapshot contains a substring.
    pub fn contains(&self, pattern: &str) -> bool {
        self.to_text().contains(pattern)
    }

    /// Get a specific line by index (0-based).
    pub fn line(&self, idx: usize) -> Option<&str> {
        self.lines.get(idx).map(|s| s.as_str())
    }

    /// Get line count.
    pub fn line_count(&self) -> usize { self.lines.len() }

    /// Get dimensions.
    pub fn size(&self) -> (u16, u16) { (self.cols, self.rows) }

    /// Construct from raw text (for testing SnapshotDiff).
    pub fn from_text(text: &str) -> Self {
        Self {
            lines: text.lines().map(|l| l.to_string()).collect(),
            cols: 80,
            rows: 24,
            timestamp: Instant::now(),
        }
    }
}
```

### 32.6 TermletBuilder [v11 P2 updated]

```rust
// crates/mux-termlet/src/builder.rs

pub struct TermletBuilder {
    command: String,
    config: TermletConfig,
}

impl TermletBuilder {
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            config: TermletConfig::default(),
        }
    }

    pub fn cols(mut self, cols: u16) -> Self { self.config.cols = cols; self }
    pub fn rows(mut self, rows: u16) -> Self { self.config.rows = rows; self }
    pub fn size(mut self, cols: u16, rows: u16) -> Self {
        self.config.cols = cols;
        self.config.rows = rows;
        self
    }
    pub fn env(mut self, key: impl Into<String>, val: impl Into<String>) -> Self {
        self.config.env.push((key.into(), val.into()));
        self
    }
    /// [v11 P2] Enable environment inheritance (from Gemini).
    pub fn inherit_env(mut self) -> Self {
        self.config.inherit_env = true;
        self
    }
    pub fn cwd(mut self, cwd: impl Into<String>) -> Self {
        self.config.cwd = Some(cwd.into());
        self
    }
    pub fn fake(mut self) -> Self { self.config.backend = PtyMode::Fake; self }
    pub fn real(mut self) -> Self { self.config.backend = PtyMode::Real; self }
    pub fn poll_interval(mut self, interval: Duration) -> Self {
        self.config.wait_poll_interval = interval;
        self
    }
    /// [v11 P2] Set maximum backoff interval (from GPT).
    pub fn max_interval(mut self, interval: Duration) -> Self {
        self.config.wait_max_interval = interval;
        self
    }
    pub fn grace_period(mut self, period: Duration) -> Self {
        self.config.grace_period = period;
        self
    }

    pub fn spawn(self) -> Result<Termlet, TermletError> {
        Termlet::spawn(&self.command, self.config)
    }

    /// [v11 P2] Spawn with explicit backend (inspired by GPT DI pattern).
    pub fn spawn_with_backend(self, backend: Box<dyn PtyBackend>) -> Result<Termlet, TermletError> {
        Termlet::spawn_with_backend(&self.command, self.config, backend)
    }
}
```

### 32.7 TermletPool [v11 P2 updated]

```rust
// crates/mux-termlet/src/pool.rs
use std::collections::HashMap;

pub struct TermletPool {
    termlets: HashMap<String, Termlet>,
}

impl TermletPool {
    pub fn new() -> Self {
        Self { termlets: HashMap::new() }
    }

    pub fn spawn(
        &mut self,
        name: &str,
        command: &str,
        config: TermletConfig,
    ) -> Result<(), TermletError> {
        let termlet = Termlet::spawn(command, config)?;
        self.termlets.insert(name.to_string(), termlet);
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&Termlet> {
        self.termlets.get(name)
    }

    pub fn get_mut(&mut self, name: &str) -> Result<&mut Termlet, TermletError> {
        self.termlets.get_mut(name).ok_or(TermletError::NotInPool {
            name: name.to_string(),
        })
    }

    pub fn kill_all(&mut self) {
        for termlet in self.termlets.values_mut() {
            let _ = termlet.kill();
        }
    }

    pub fn snapshot_all(&mut self) -> HashMap<String, String> {
        self.termlets.iter_mut()
            .map(|(name, termlet)| (name.clone(), termlet.snapshot().to_text()))
            .collect()
    }

    /// [v11] Wait for pattern in all Termlets. Per-Termlet timeout.
    pub fn wait_all(
        &mut self,
        pattern: &str,
        per_termlet_timeout: Duration,
    ) -> HashMap<String, Result<WaitMatch, TermletError>> {
        self.termlets.iter_mut()
            .map(|(name, termlet)| {
                let result = termlet.wait_for(pattern, per_termlet_timeout);
                (name.clone(), result)
            })
            .collect()
    }

    /// [v11] Send keys to all Termlets.
    pub fn send_all(&mut self, keys: &str) -> HashMap<String, Result<(), TermletError>> {
        self.termlets.iter_mut()
            .map(|(name, termlet)| {
                let result = termlet.send_keys(keys);
                (name.clone(), result)
            })
            .collect()
    }

    pub fn len(&self) -> usize { self.termlets.len() }
    pub fn is_empty(&self) -> bool { self.termlets.is_empty() }
    pub fn names(&self) -> Vec<&str> { self.termlets.keys().map(|k| k.as_str()).collect() }

    /// [v11 P2] Async version of spawn (from Gemini). Feature-gated.
    #[cfg(feature = "async")]
    pub async fn spawn_async(
        &mut self,
        name: &str,
        command: &str,
        config: TermletConfig,
    ) -> Result<(), TermletError> {
        self.spawn(name, command, config)
    }
}

impl Drop for TermletPool {
    fn drop(&mut self) { self.kill_all(); }
}
```

### 32.8 SnapshotDiff [v11 P2 updated]

```rust
// crates/mux-termlet/src/diff.rs

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
    /// [v11 P2] Size tracking from GPT.
    pub before_size: (u16, u16),
    pub after_size: (u16, u16),
}

impl SnapshotDiff {
    /// Compare two snapshots. Fast path for identical: no per-line allocation.
    /// [v11 P2] context_lines parameter from Gemini (for future display logic).
    pub fn compare(
        before: &TermletSnapshot,
        after: &TermletSnapshot,
        _context_lines: usize,
    ) -> Self {
        let bt = before.to_text();
        let at = after.to_text();
        let before_size = before.size();
        let after_size = after.size();

        if bt == at && before_size == after_size {
            return Self {
                changed_lines: Vec::new(),
                identical: true,
                before_size,
                after_size,
            };
        }

        let bl: Vec<&str> = bt.lines().collect();
        let al: Vec<&str> = at.lines().collect();
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
            identical: changed.is_empty(),
            changed_lines: changed,
            before_size,
            after_size,
        }
    }

    /// Human-readable diff display.
    pub fn to_display(&self) -> String {
        if self.identical {
            return "Snapshots are identical.".to_string();
        }
        let mut out = format!(
            "Snapshot diff: {} changed line(s) (before: {}x{}, after: {}x{})\n",
            self.changed_lines.len(),
            self.before_size.0, self.before_size.1,
            self.after_size.0, self.after_size.1,
        );
        for diff in &self.changed_lines {
            out.push_str(&format!("  row {}: -{:?} +{:?}\n", diff.row, diff.before, diff.after));
        }
        out
    }
}
```

### 32.9 Error Code Reference [v11 P2 updated]

| Error Variant | Code String | Python Exception | Node Error |
|---|---|---|---|
| `SpawnFailed` | `TERMLET_SPAWN_ERROR` | `RuntimeError` | `Error` |
| `AlreadyExited` | `TERMLET_ALREADY_EXITED` | `RuntimeError` | `Error` |
| `WaitForTimeout` | `TERMLET_TIMEOUT` | `TimeoutError` | `Error` (code: TIMEOUT) |
| `PatternNotFound` | `TERMLET_PATTERN_NOT_FOUND` | `RuntimeError` | `Error` (code: NOT_FOUND) |
| `ResizeFailed` | `TERMLET_RESIZE_ERROR` | `RuntimeError` | `Error` |
| `Pty(...)` | `TERMLET_PTY_ERROR` | `RuntimeError` | `Error` |
| `NotInPool` | `TERMLET_NOT_IN_POOL` | `KeyError` | `Error` (code: NOT_IN_POOL) |
| `InvalidRegex` | `TERMLET_INVALID_REGEX` | `ValueError` | `Error` (code: INVALID_REGEX) |

### 32.10 Python Binding Contract

```python
# Python API mirrors libtmux ergonomics and pytest friendliness.
import termforge

# Context manager guarantees cleanup.
with termforge.Termlet.spawn("bash", cols=120, rows=40) as t:
    t.send_keys("echo hello\n")
    match = t.wait_for("hello", timeout=5.0)
    snap = t.snapshot()
    assert "hello" in snap.to_text()
    # exit_status available after process exits
    t.kill()
    assert t.exit_status == 0  # [v11 P2]

# Builder pattern alternative
t = termforge.TermletBuilder("python3") \
    .size(80, 24) \
    .fake() \
    .inherit_env() \
    .spawn()

# Pool for multi-Termlet scenarios
with termforge.TermletPool() as pool:
    pool.spawn("server", "python3 server.py")
    pool.spawn("client", "python3 client.py")
    pool["server"].wait_for("listening", timeout=10.0)
    pool["client"].send_keys("connect\n")
```

### 32.11 Node.js Binding Contract

```javascript
// Node API uses Promise-based methods where blocking can occur.
import { Termlet, TermletPool, TermletBuilder } from 'termforge';

// Spawn and interact
const t = await Termlet.spawn('bash', { cols: 80, rows: 24 });
try {
    await t.sendKeys('echo hello\n');
    const match = await t.waitFor('hello', { timeout: 5000 });
    const snap = t.snapshot();
    expect(snap.toText()).toContain('hello');
    // [v11 P2] exit_status available after kill
    await t.kill();
    expect(t.exitStatus).toBe(0);
} finally {
    await t.kill();
}

// Pool pattern
const pool = new TermletPool();
try {
    await pool.spawn('server', 'node server.js');
    await pool.spawn('client', 'node client.js');
    await pool.get('server').waitFor('listening', { timeout: 10000 });
} finally {
    pool.killAll();
}
```

### 32.12 TermletExt Trait [v11 new]

```rust
// crates/mux-termlet/src/ext.rs

/// Extension trait for custom assertion methods on Termlets.
/// Downstream crates can implement this without modifying mux-termlet.
/// RULE: TermletExt must NOT add state mutation methods.
pub trait TermletExt {
    fn assert_contains(&mut self, text: &str);
    fn assert_not_contains(&mut self, text: &str);
    fn assert_line(&mut self, idx: usize, expected: &str);
    fn assert_line_count(&mut self, expected: usize);
}

impl TermletExt for Termlet {
    fn assert_contains(&mut self, text: &str) {
        let snap = self.snapshot();
        assert!(snap.contains(text), "expected snapshot to contain {text:?}, got:\n{}", snap.to_text());
    }

    fn assert_not_contains(&mut self, text: &str) {
        let snap = self.snapshot();
        assert!(!snap.contains(text), "expected snapshot to NOT contain {text:?}");
    }

    fn assert_line(&mut self, idx: usize, expected: &str) {
        let snap = self.snapshot();
        let actual = snap.line(idx).unwrap_or("");
        assert_eq!(actual.trim_end(), expected, "line {idx} mismatch");
    }

    fn assert_line_count(&mut self, expected: usize) {
        let snap = self.snapshot();
        assert_eq!(snap.line_count(), expected, "line count mismatch");
    }
}
```

### 32.13 Async Support

```rust
// crates/mux-termlet/src/async_support.rs
#[cfg(feature = "async")]
pub async fn wait_for_async(
    termlet: &mut Termlet,
    pattern: &str,
    timeout: Duration,
) -> Result<WaitMatch, TermletError> {
    let matcher = PatternMatcher::compile(pattern)?;
    let start = Instant::now();
    let mut sleep_for = termlet.config.wait_poll_interval;

    loop {
        termlet.drain_output();
        let text = termlet.snapshot_text();

        if let Some(range) = matcher.find(&text) {
            return Ok(WaitMatch {
                byte_range: range,
                matched_at: Instant::now(),
            });
        }

        if termlet.state() == TermletState::Exited {
            return Err(TermletError::PatternNotFound {
                pattern: pattern.to_string(),
            });
        }

        if start.elapsed() >= timeout {
            return Err(TermletError::WaitForTimeout {
                pattern: pattern.to_string(),
                timeout_ms: timeout.as_millis() as u64,
            });
        }

        let remaining = timeout.saturating_sub(start.elapsed());
        tokio::time::sleep(sleep_for.min(remaining)).await;
        sleep_for = (sleep_for * 2).min(termlet.config.wait_max_interval);
    }
}
```

### 32.14 OTEL in Termlets

If the `otel` feature is enabled, emit spans for all Termlet operations:

| Operation | Span Name | Attributes |
|---|---|---|
| spawn | `termlet.spawn` | `termlet.id`, `command`, `backend.mode`, `cols`, `rows` |
| send_keys | `termlet.send_keys` | `termlet.id`, `keys.len` |
| wait_for | `termlet.wait_for` | `termlet.id`, `pattern`, `timeout_ms`, `matched` |
| snapshot | `termlet.snapshot` | `termlet.id`, `cols`, `rows` |
| resize | `termlet.resize` | `termlet.id`, `old_cols`, `old_rows`, `new_cols`, `new_rows` |
| kill | `termlet.kill` | `termlet.id`, `grace_period_ms`, `forced` |

### 32.15 Implementation Hints for LLMs [v11 P2 from Gemini]

Specific guidance for AI coding agents implementing Termlets:

1. **Non-blocking IO:** When implementing `RealPtyBackend`, ensure the file descriptor is set to non-blocking mode (`O_NONBLOCK`). `read_events` should return immediately with whatever data is available, or an empty vector if none. Do NOT block waiting for data.

2. **Drain Loop:** The `wait_for` loop relies on `drain_output` to fetch data. If `drain_output` blocks, the timeout logic will fail. Always test `drain_output` returns promptly when no data is available.

3. **Pattern Compilation:** Use `regex::Regex::new` only inside `PatternMatcher::compile`. Do not re-compile inside the poll loop. The compiled matcher is reused across all iterations.

4. **Byte Ranges:** `WaitMatch.byte_range` indexes into the **UTF-8 string** returned by `grid.to_text()`, not the raw grid cells. The range refers to byte offsets in the trimmed text.

5. **Snapshots:** `grid.to_text()` must strip trailing whitespace from every line and strip trailing blank lines to ensure stable snapshots across resize operations (which might introduce padding spaces).

6. **TermletState:** State transitions are monotonic. Always check state before performing operations. The enum values are ordered: `Created < Spawning < Running < Stopping < Exited`.

7. **Exit Status:** Capture exit status from `PtyEvent::Exited` events during `drain_output`. Store in the Termlet struct. Check with `exit_status()` after confirming state is `Exited`.

8. **inherit_env:** When `inherit_env` is `true`, merge parent environment variables but do NOT override explicit entries in `config.env`. Explicit env takes precedence.

9. **Resize Safety:** Always call `drain_output()` before AND after `backend.resize()` to capture any output generated in response to `SIGWINCH`.

### Test Strategy

Mandatory Termlet tests (merged from all three models, 67 tests total):

1. Spawn in real and fake modes -- dual-mode parametrized test.
2. `send_keys` + `wait_for` plain string -- basic interaction test.
3. `wait_for` regex path + invalid regex error -- pattern variant test.
4. Timeout vs process-exited distinction -- error classification test.
5. **[v11 P2]** Zero-timeout immediate check semantics -- zero-timeout test.
6. Snapshot trimming contract -- trailing whitespace test.
7. Resize updates size and backend call -- resize test.
8. **[v11 P2]** Resize drains before and after -- resize drain test.
9. Kill idempotency -- double-kill test.
10. Drop cleanup (forced kill) -- drop leak test.
11. Fake backend deterministic replay -- determinism test.
12. Cross-language snapshot parity fixtures -- multi-runner test.
13. Python context manager no-leak -- pytest.
14. Node finally-cleanup no-leak -- vitest.
15. TermletBuilder parity with direct config -- builder/config equivalence test.
16. TermletPool `kill_all` and `Drop` behavior -- pool lifecycle test.
17. TermletPool `wait_all` per-Termlet timeout -- pool wait test.
18. TermletPool `send_all` -- pool broadcast test.
19. TermletPool `snapshot_all` -- pool snapshot test.
20. SnapshotDiff identical fast path (no allocation) -- benchmark.
21. SnapshotDiff changed-lines accuracy -- diff accuracy test.
22. **[v11 P2]** SnapshotDiff with context_lines -- display format test.
23. OTEL spans emitted when feature enabled -- attribute presence test.
24. Performance: FakePty spawn < 500us -- criterion benchmark.
25. Performance: snapshot 80x24 < 50us -- criterion benchmark.
26. Performance: real PTY spawn < 50ms -- criterion benchmark.
27. **[v11]** drain_output nonblocking behavior -- nonblocking test.
28. **[v11]** output_history returns raw bytes -- byte assertion test.
29. **[v11]** TermletPaneId uniqueness -- concurrent ID test.
30. **[v11]** TermletExt assertion helpers -- extension test.
31. **[v11]** kill SIGTERM -> grace -> SIGKILL sequence -- signal sequence test.
32. **[v11]** NotInPool error from pool.get_mut -- error variant test.
33. **[v11]** fake_backend_mut downcast -- downcasting test.
34. **[v11 P2]** TermletState lifecycle: Created -> Running -> Exited -- state transition test.
35. **[v11 P2]** TermletState gating: send_keys in Exited returns AlreadyExited -- method gating test.
36. **[v11 P2]** TermletState: snapshot allowed in Exited -- post-exit snapshot test.
37. **[v11 P2]** exit_status captured on process exit -- exit code test.
38. **[v11 P2]** exit_status is None before process exits -- pre-exit status test.
39. **[v11 P2]** inherit_env merges parent env without overriding explicit -- env merge test.
40. **[v11 P2]** inherit_env default is false -- hermetic default test.
41. **[v11 P2]** wait_max_interval caps backoff -- interval cap test.
42. **[v11 P2]** Remaining-time clamping in wait_for -- oversleep prevention test.
43. **[v11 P2]** to_cells() returns style-aware data -- cell assertion test.
44. **[v11 P2]** to_styled() produces ANSI output -- styled output test.
45. **[v11 P2]** spawn_with_backend() works with injected backend -- DI test.
46. **[v11 P2]** TermletBuilder.inherit_env() produces correct config -- builder field test.
47. **[v11 P2]** TermletBuilder.max_interval() produces correct config -- builder field test.
48. **[v11 P2]** InvalidRegex error distinct from SpawnFailed -- error variant test.
49. **[v11 P2]** Python exit_status accessible -- pytest.
50. **[v11 P2]** Node exitStatus accessible -- vitest.
51. **[v11 P2]** CrossLangCase shared fixture parity -- multi-runner test.
52-67. Negative tests for all 8 error variants (constructible, matchable, error_code stable) + additional binding parity and regression tests.

### AGENTS.md Rules

- `RULE-S32-01`: Termlet snapshot text must be trailing-whitespace-trimmed. **Enforcement:** Snapshot format test.
- `RULE-S32-02`: Termlet kill must be idempotent. **Enforcement:** Double-kill test.
- `RULE-S32-03`: Termlet Drop must send SIGKILL to prevent PTY process leaks. **Enforcement:** Drop impl audit.
- `RULE-S32-04`: FakePtyBackend tests must not depend on timing. **Enforcement:** Determinism check.
- `RULE-S32-05`: Termlet bindings must implement context manager / cleanup pattern. **Enforcement:** Binding API test.
- `RULE-S32-06`: `mux-termlet` must not depend on `mux-server`, `mux-api`, or `mux-orm`. **Enforcement:** `cargo tree` CI.
- `RULE-S32-07`: Every Termlet API method tested in Rust + at least one binding. **Enforcement:** Test coverage.
- `RULE-S32-08`: `wait_for` must compile pattern once, not per poll iteration. **Enforcement:** PatternMatcher audit.
- `RULE-S32-09`: Termlet remains pane-backed; never invent parallel terminal core. **Enforcement:** Architecture review.
- `RULE-S32-10`: Cross-language snapshot consistency tests must exist for every release. **Enforcement:** Multi-runner CI.
- `RULE-S32-11`: `TermletBuilder` and `TermletConfig` must produce identical Termlets. **Enforcement:** Unit test.
- `RULE-S32-12`: `SnapshotDiff` must not allocate on identical snapshots. **Enforcement:** Benchmark.
- `RULE-S32-13`: **[v11]** `drain_output` must use nonblocking reads with `WouldBlock` handling. **Enforcement:** Code audit.
- `RULE-S32-14`: **[v11]** `TermletPool` must use `HashMap<String, Termlet>` for named access. **Enforcement:** Type check.
- `RULE-S32-15`: **[v11]** `Termlet::kill()` must implement full SIGTERM -> grace_period -> SIGKILL. **Enforcement:** Sequence test.
- `RULE-S32-16`: **[v11]** `TermletPaneId` must be distinct from `PaneId`. **Enforcement:** Type system check.
- `RULE-S32-17`: **[v11]** `output_history()` must return immutable reference, never clone. **Enforcement:** API signature check.
- `RULE-S32-18`: **[v11]** `TermletExt` trait must not add state mutation methods. **Enforcement:** Trait method audit.
- `RULE-S32-19`: **[v11]** `TermletPool::wait_all()` timeout must be per-Termlet, not total. **Enforcement:** Implementation check.
- `RULE-S32-20`: **[v11]** `SnapshotDiff::compute()` returns empty diff for identical inputs. **Enforcement:** Zero-allocation test.
- `RULE-S32-21`: **[v11 P2]** `TermletState` transitions must be monotonic. **Enforcement:** State transition test.
- `RULE-S32-22`: **[v11 P2]** API methods must gate on `TermletState`. **Enforcement:** Method gating test.
- `RULE-S32-23`: **[v11 P2]** `exit_status` must be populated when process exits. **Enforcement:** Exit status test.
- `RULE-S32-24`: **[v11 P2]** `inherit_env` default must be `false`. **Enforcement:** Default config test.
- `RULE-S32-25`: **[v11 P2]** Zero-timeout `wait_for` performs single check. **Enforcement:** Zero-timeout test.
- `RULE-S32-26`: **[v11 P2]** `RealPtyBackend` must use non-blocking IO. **Enforcement:** Architecture review + unit test.

---

## v11 Pass 2 Final Consistency Checklist

- [x] 32 sections present (1-32).
- [x] Every section includes: Design Decisions, Rust Example, Test Strategy, AGENTS.md Rules.
- [x] Section 32 deepest with subsections 32.1-32.15.
- [x] All rules use `RULE-Snn-xx` naming convention with explicit enforcement.
- [x] Total rules: 66 (up from 58 in v11 Pass 1, 47 in Gemini v11, 10 in GPT v11).
- [x] Total risks: 48 (up from 43 in v11 Pass 1, 39 in Gemini v11).
- [x] Total Termlet test cases: 67 (up from 62 in Claude v11, 20 in GPT v11).
- [x] All `[v11 P2]` additions tagged.
- [x] Settled decisions table: 25 items (up from 20 in v11 Pass 1).
- [x] Global invariants: 8 `INV-*` items.
- [x] Cross-model conflict resolutions: 8 items documented in Preamble.
- [x] Cross-pollinated features: 16 from GPT, 7 from Gemini, fully integrated.
- [x] Claims mapped to reference anchors where compatibility-sensitive.
- [x] Section 32 includes lifecycle state machine, error taxonomy, binding contracts, OTEL spans, LLM implementation hints.
- [x] Rust examples written for Rust 2024-compatible style.
- [x] `TermletError` has 8 variants with stable `ErrorCode` strings.
- [x] `TermletState` enum adopted with monotonic transitions and API gating.
- [x] `ServerPhase` enum adopted for server lifecycle.
- [x] `PerfTarget` struct for programmatic performance enforcement.
- [x] `CrossLangCase` for shared cross-language test fixtures.
- [x] `BindingQueryList<T>` for binding query wrappers.

---

*End of TermForge v11 Pass 2 Specification.*
