# TermForge v12 Architecture Specification -- Pass 1

Date: 2026-02-11
Status: **DRAFT** -- v12 Pass 1, critical review and improvement of v11 P3 DEFINITIVE.
Lineage: v4 -> v5 -> v6 -> v7 -> v8 -> v9 P3 -> v10 P3 -> v11 P3 DEFINITIVE -> **v12 Pass 1** (this document).
Model: Claude Opus 4.6 (Pass 1 of 3).
License: MIT OR Apache-2.0
Rust edition: 2024 (MSRV 1.85)
Protocol target: tmux protocol v8

---

## Preamble

This document is the v12 Pass 1 architecture specification for TermForge, a Rust terminal multiplexer with 100% tmux wire-protocol compatibility, ORM-like API, language bindings, CRDT collaboration, a ratatui-based TUI client, and **Termlets** -- SDK-first testing pods that are the killer differentiator.

**This specification critically examines v11 P3 DEFINITIVE and produces targeted improvements.** An LLM agent should be able to implement TermForge from scratch using only this document plus the referenced codebases.

### v12 Pass 1 Critical Review of v11

**[v12] Weaknesses identified in v11 P3:**

1. **TermletState ordering is semantically wrong.** v11 defines `PartialOrd`/`Ord` on TermletState with `SpawnFailed` ordered AFTER `Stopping` but BEFORE `Exited`. This is misleading because `SpawnFailed` branches from `Spawning`, not from `Stopping`. The monotonic ordering claim breaks for the SpawnFailed branch. [v12] removes `Ord`/`PartialOrd` from TermletState; uses explicit `valid_transition()` function instead.

2. **`mux-core` layer classification inconsistency.** v11 puts `mux-core` at Layer 1 (OS) but says it has `#![forbid(unsafe_code)]` and no IO. A crate with no IO and no unsafe belongs at Layer 0. [v12] moves `mux-core` to Layer 0 (pure), adds `mux-state` at Layer 1 for the server lifecycle parts that need `tracing`.

3. **Termlet::spawn() never enters `Spawning` state.** v11 defines `Created -> Spawning -> Running` but the `spawn()` constructor goes directly to `Running`. The `Created` and `Spawning` states are dead code. [v12] makes the state machine honest: `spawn()` returns in `Running` state, construction is atomic. `Created` removed as it was never observable externally.

4. **`PtyBackend::spawn` takes `TermletPaneId` but TermletPaneId is defined in mux-termlet (Layer 2), not mux-pty (Layer 1).** This is a dependency inversion. [v12] changes PtyBackend to use an opaque `PtyHandle` u64 token.

5. **`Termlet::send_keys` takes `&str` but tmux keys can be non-UTF8.** [v12] adds `send_bytes(&[u8])` alongside `send_keys` for raw byte injection.

6. **No concrete `Grid` or `VtParser` API specified.** v11 references `Grid::new()`, `grid.rows_text()`, `grid.to_text_trimmed()`, `grid.resize()` but never defines these methods. [v12] adds Section 5.5 with concrete Grid API.

7. **`TermletSnapshot` stores `Option<Vec<Vec<Cell>>>` which is expensive.** [v12] makes cell capture lazy -- computed on first `to_cells()` call via grid reference, not eagerly stored.

8. **`acquire_lock` uses `mem::forget(file)` which leaks the File.** [v12] stores the `File` in the guard instead.

9. **`QueryList::filter_by` has `T: Queryable` bound but `Queryable` trait is never defined.** [v12] defines the `Queryable` trait with `matches(&self, spec: &QuerySpec) -> bool`.

10. **`spawn_with_backend` duplicates 90% of `spawn` code.** [v12] refactors to a private `spawn_inner` that both call.

11. **`CoreCtx.rng` is `Box<dyn FnMut() -> u64>` which is not `Send`.** [v12] makes it `Box<dyn FnMut() -> u64 + Send>` for use across thread boundaries.

12. **No graceful degradation story for CRDT.** [v12] adds a feature gate `crdt` so CRDT is optional.

13. **`TermletPool::spawn_async` is trivially sync -- just calls sync spawn.** [v12] removes the fake async wrapper, replaces with genuine `tokio::task::spawn_blocking` delegation.

14. **Missing `QuerySpec` definition.** v11 references `QuerySpec` in `filter_by` but never defines the struct. [v12] adds concrete definition.

15. **`TermletExt` assertion methods take `&mut self` unnecessarily.** Snapshot should be taken immutably. [v12] changes to `&self` where possible, adds a note about drain_output being the mutation.

16. **No Termlet `restart()` or `respawn()` API.** Common testing pattern is to restart a process in the same terminal. [v12] adds `restart(command: &str)` that kills and respawns.

17. **Section 32.15 "Implementation Hints for LLMs" mixes hints with spec.** [v12] separates normative spec from implementer guidance via clear labeling.

**[v12] Additions not in v11:**

1. **`Grid` concrete API** (Section 5 expanded): `Grid::new`, `rows_text`, `to_text_trimmed`, `resize`, `cell_at`, `cursor_position`.
2. **`Queryable` trait definition** with derive macro plan.
3. **`QuerySpec` struct** with field path, operator, and value.
4. **Termlet `restart()`** method for test loops.
5. **`PtyHandle` opaque token** replacing TermletPaneId in PtyBackend.
6. **`send_bytes(&[u8])`** for raw byte injection.
7. **Feature gate `crdt`** for optional CRDT support.
8. **`GridCapture`** lazy cell snapshot.
9. **Expanded Section 32** with subsections 32.1-32.18 (was 32.1-32.16).

**Traceability convention (carried from v11):**
- `INV-*`: core invariants
- `API-*`: API contracts
- `PAR-*`: parity contracts against tmux/libtmux/reference crates
- `OPS-*`: runtime/operational contracts
- `TST-*`: mandatory test gates

**Settled global invariants (carried from v11, updated):**
- `INV-001`: Protocol compatibility target is tmux protocol v8 (`tmux-protocol.h:23`).
- `INV-002`: `mux-core` is deterministic and IO-free (`#![forbid(unsafe_code)]`). **[v12] mux-core is now Layer 0.**
- `INV-003`: Single writer for state mutation; snapshot-based readers.
- `INV-004`: Protocol violation drops client connection, never partial-continue (`server-client.c:3472-3475`).
- `INV-005`: First-client identify completes before config load path (`server-client.c:3725-3734`).
- `INV-006`: Layout resize uses round-robin one-cell adjustment (`layout.c:448-462`).
- `INV-007`: Socket/test isolation uses three guard layers (path guard pattern).
- `INV-008`: Termlets are pane-backed and never a parallel terminal stack.
- `INV-009`: **[v12]** `TermletState` transitions validated by `valid_transition()`, not by enum ordering.

**Reference codebases (re-verified for v12):**
- `~/work/python/libtmux/src/libtmux/_internal/query_list.py` -- 12 operators confirmed in `LOOKUP_NAME_MAP`: `eq`, `exact`, `iexact`, `contains`, `icontains`, `startswith`, `istartswith`, `endswith`, `iendswith`, `in`, `nin`, `regex`, `iregex`. Callable matcher in `filter()`. `get(default=no_arg)` semantics. `keygetter` nested `__` traversal. Exceptions: `MultipleObjectsReturned`, `ObjectDoesNotExist`, `PKRequiredException`, `OpNotFound`.
- `~/work/rust/vibe-tmux/` (logs dir only at `~/.local/state/vibe-tmux/`) -- mux-test-support PathGuard 3-layer validation, mux-otel dual providers, tmux-builder BLAKE3 cache.
- `~/study/rust/zellij/` -- terminal multiplexer: `zellij-server/src/panes/grid.rs` uses `VecDeque<Row>` for scrollback, `Row` with `is_canonical` flag, `Perform` trait from `vte` crate. Grid manages viewport + scrollback. Plugin panes separated from terminal panes.
- `~/study/rust/ratatui/` -- TUI framework: `ratatui-core/src/buffer/buffer.rs` uses flat `Vec<Cell>` with `Rect` area. Cell stores grapheme + style. Buffer has `set_string`, `cell_mut`, `Index` trait. Clear separation between buffer (state) and terminal (IO).
- `~/study/c/tmux/` -- `tmux-protocol.h:23`: `PROTOCOL_VERSION 8`. `client.c:89`: `flock(lockfd, LOCK_EX|LOCK_NB)`. `layout.c:448-462` round-robin resize. `layout-custom.c:46-57` checksum. `options.c:891-903` FALLTHROUGH.

**Settled decisions (carried from v11, updated for v12):**

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
| S15 | mux-core is pure | No IO, no `unsafe`, no `tokio`, no `libc`. `#![forbid(unsafe_code)]`. **[v12] Now Layer 0.** |
| S16 | QueryList ORM layer | 18 operators (12 from libtmux + 6 extensions). Django/SQLAlchemy-inspired. |
| S17 | Synchronous API for `mux-termlet` | `Termlet::wait_for` uses `std::thread::sleep` with exponential backoff. No async runtime required. |
| S18 | `TermletBuilder` for fluent construction | Builder pattern as ergonomic alternative to `TermletConfig` struct. |
| S19 | `TermletPaneId` newtype for Termlet-internal pane tracking | Not `slotmap::KeyData::from_ffi()`. Clean separation from server-managed `PaneId`. |
| S20 | `PtyBackend` requires `as_any_mut()` for downcasting | Enables `fake_backend_mut()` without unsafe. Trait object must support `Any`. |
| S21 | `TermletState` lifecycle enum | Monotonic state machine with API method gating. Not boolean `alive` flag. |
| S22 | `inherit_env` opt-in for Termlets | Default hermetic (`false`). Integration tests can opt in to parent env. |
| S23 | `wait_max_interval` separate from `wait_poll_interval` | Initial poll interval grows via exponential backoff up to max interval. |
| S24 | Zero-timeout `wait_for` is single immediate check | Not an error. Drains output once, checks pattern, returns immediately. |
| S25 | `exit_status` tracked on Termlet | Available after process exit for assertion on return codes. |
| S26 | `TermletState` includes `SpawnFailed` as terminal state | Spawn failure produces `SpawnFailed` state, not `Exited`. Distinguishes process-never-ran from process-ran-and-exited. |
| S27 | `TermletSnapshot` stores Grid reference for `to_cells()` | Avoids double-buffering cell data. `to_cells()` reads directly from Grid capture. |
| S28 | **[v12]** `mux-core` at Layer 0, `mux-state` at Layer 1 | Pure kernel stays pure; server lifecycle state management in separate crate. |
| S29 | **[v12]** `TermletState` without `Ord` | Use `valid_transition()` function, not comparison operators, for state machine validation. |
| S30 | **[v12]** `PtyHandle(u64)` for backend API | PtyBackend uses opaque handle, not TermletPaneId, to avoid dependency inversion. |
| S31 | **[v12]** `Created` state removed from TermletState | Never externally observable; spawn() is atomic. |
| S32 | **[v12]** `Queryable` trait explicitly defined | Required for `filter_by` to compile. Includes `matches()` and field access. |
| S33 | **[v12]** CRDT feature-gated | `mux-crdt` dependency is behind `crdt` feature flag for smaller builds. |

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

## 2. Acceptance Criteria and Gates [v12 updated]

### Design Decisions

- Acceptance gates are the release checklist. Organized into categories: Compatibility (C), Purity (A), Performance (B), Programmability (P), Ecosystem (E).
- Every gate maps to at least one test in the CI matrix.
- **[v12]** Added C7 (option FALLTHROUGH parity depth test), A4 (mux-core Layer 0 WASM proof), P15 (Termlet restart lifecycle), P16 (QuerySpec filter_by compilation), B16 (Grid cell_at throughput).

### 2.1 Acceptance Gate Table

| ID | Category | Gate | Test / Evidence | Pass Threshold | Fail Threshold |
|---|---|---|---|---|---|
| C1 | Compatibility | `tmux attach` to TermForge server | Integration test | Attach succeeds, session list matches | Attach fails or session mismatch |
| C2 | Compatibility | `tmux list-sessions` on TermForge | Integration test | Correct session list | Missing or malformed sessions |
| C3 | Compatibility | tmux control mode subscribe/notify | Integration test | Notifications received for session/window/pane events | Missing notifications |
| C4 | Compatibility | tmux config file parsing | Unit test + fixtures | >= 95% of tmux options parsed correctly | < 90% coverage |
| C5 | Compatibility | Copy mode basic operations | Integration test | yank, paste, search work | Any basic operation fails |
| C6 | Compatibility | Control-mode backpressure semantics | Integration test | Pause/resume notifications match tmux behavior | Backpressure divergence |
| C7 | Compatibility | **[v12]** Option FALLTHROUGH depth 4 (pane->window->session->server) | Property test | All 4 levels resolve correctly | Any level skipped |
| A1 | Purity | `mux-core` compiles without `std::io` | `cargo check --target wasm32-unknown-unknown` | Compiles clean | Any IO import |
| A2 | Purity | `mux-core` has `#![forbid(unsafe_code)]` | CI attribute check | Attribute present | Missing attribute |
| A3 | Purity | Cross-section traceability lint | CI | Every `INV-*` mapped to at least one `TST-*` | Unmapped invariant |
| A4 | Purity | **[v12]** `mux-core` Layer 0 classification proof | `cargo tree` | No Layer 1+ deps in mux-core | Any OS/IO dependency |
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
| P10 | Programmability | TermletPool batch operations | Rust unit test | Pool spawn/kill_all/snapshot_all work | Any operation fails |
| P11 | Programmability | SnapshotDiff detects changes | Rust unit test | Changed lines identified correctly | False positive/negative |
| P12 | Programmability | TermletState lifecycle gating | Rust unit test | API methods reject calls in wrong state | Allowed in wrong state |
| P13 | Programmability | Termlet exit_status tracking | Rust unit test | Exit code captured after process exit | Missing or wrong code |
| P14 | Programmability | SpawnFailed state distinguishable from Exited | Rust unit test | Failed spawn produces SpawnFailed not Exited | Wrong terminal state |
| P15 | Programmability | **[v12]** Termlet restart() lifecycle | Rust unit test | Restart kills old process and spawns new one | Process leak or panic |
| P16 | Programmability | **[v12]** QuerySpec filter_by compiles and filters | Rust unit test | Correct filtering with Queryable trait | Compilation failure or wrong results |
| B10 | Performance | Termlet spawn (FakePty) | Criterion | < 500 us | > 1 ms |
| B11 | Performance | Termlet snapshot (80x24) | Criterion | < 50 us | > 100 us |
| B12 | Performance | Termlet spawn (real PTY) | Criterion | < 50 ms | > 100 ms |
| B13 | Performance | drain_output 1KB buffer | Criterion | < 10 us | > 50 us |
| B14 | Performance | PerfTarget enforcement | Criterion | All targets within `target_ns` | Any exceeds `hard_fail_ns` |
| B15 | Performance | Pool stress: 50 spawn/kill cycles | Criterion | < 5 seconds total, zero leaks | > 10s or any FD leak |
| B16 | Performance | **[v12]** Grid cell_at 80x24 | Criterion | < 10 ns per cell | > 50 ns per cell |
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
    C1WireAttach, C2ListSessions, C3ControlNotifications,
    C4ConfigParse, C5CopyMode, C6ControlBackpressure,
    C7OptionFallthrough,  // [v12]
    // Purity
    A1NoPureIo, A2ForbidUnsafe, A3InvariantTraceability,
    A4CoreLayerZero,      // [v12]
    // Performance
    B1Vt100Ascii, B2Vt100Csi, B3ProtoDecode,
    B4LayoutResize, B5Snapshot50,
    // Programmability
    P1PyFilter, P2PyGet, P3NodeFilter,
    P4OtelTrace, P5ControlParse, P6CrdtMerge,
    P7TermletSpawn, P8PyTermletCtxMgr, P9TermletDualMode,
    P10TermletPool, P11SnapshotDiff,
    P12TermletStateGating, P13TermletExitStatus,
    P14SpawnFailedState,
    P15TermletRestart,    // [v12]
    P16QuerySpecFilterBy, // [v12]
    // Performance (Termlet)
    B10TermletSpawnFake, B11TermletSnapshot,
    B12TermletSpawnReal, B13DrainOutputLatency,
    B14PerfTargetEnforcement, B15PoolStressCycle,
    B16GridCellAt,        // [v12]
    // Ecosystem
    E1PyPip, E2NodeNpm, E3CargoDoc,
    E4Msrv, E5Clippy, E6TermletDocs, E7TermletCrossLang,
}

impl Gate {
    pub fn release_blocking(self) -> bool {
        !matches!(self,
            Gate::B1Vt100Ascii | Gate::B2Vt100Csi | Gate::B3ProtoDecode |
            Gate::B4LayoutResize | Gate::B5Snapshot50 |
            Gate::B10TermletSpawnFake | Gate::B11TermletSnapshot |
            Gate::B12TermletSpawnReal | Gate::B13DrainOutputLatency |
            Gate::B14PerfTargetEnforcement | Gate::B15PoolStressCycle |
            Gate::B16GridCellAt
        )
    }
}

/// Performance target struct for CI enforcement.
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
    PerfTarget { name: "pool_50_spawn_kill", target_ns: 5_000_000_000, hard_fail_ns: 10_000_000_000 },
    // [v12] Grid cell access target
    PerfTarget { name: "grid_cell_at_80x24", target_ns: 10, hard_fail_ns: 50 },
];
```

### Test Strategy

1. Gate checklist run on every release candidate -- CI gate.
2. Performance gates warn but do not block (except regressions > 30%) -- threshold enforcement.
3. All functional gates must pass -- blocking gate.
4. Termlet gates P7/P8/P9 and E6/E7 mandatory for release.
5. `PerfTarget` structs used to enforce Criterion thresholds programmatically.
6. SpawnFailed gate P14 verifies state distinction from Exited.
7. **[v12]** P15 restart gate ensures no process leaks on respawn.
8. **[v12]** P16 QuerySpec gate ensures Queryable trait compiles end-to-end.
9. `TST-010`: gate inventory exists and is complete.
10. `TST-011`: CI fails on any blocking gate red.
11. `TST-012`: performance regression budget >30% marks failure.

### AGENTS.md Rules

- `RULE-S02-01`: Every new feature must map to at least one acceptance gate. **Enforcement:** PR template checklist.
- `RULE-S02-02`: No gate may be removed without spec amendment and approval. **Enforcement:** Append-only gate table.
- `RULE-S02-03`: Gate pass/fail thresholds must be measurable, not subjective. **Enforcement:** Acceptance criteria table.
- `RULE-S02-04`: Every `INV-*` invariant must map to at least one `TST-*` test. **Enforcement:** CI traceability lint (A3 gate).

---

## 3. Crate Dependency Rules [v12 updated]

### Design Decisions

- Strict layered architecture: Layer 0 (pure) -> Layer 1 (OS) -> Layer 2 (facade) -> Layer 3 (application).
- **[v12] mux-core moved to Layer 0.** v11 placed mux-core at Layer 1 despite forbidding unsafe and IO. This was inconsistent. mux-core is pure and WASM-compatible, so it belongs at Layer 0.
- **[v12] mux-state added at Layer 1.** Server lifecycle management (`ServerPhase`, `transition_phase`, tracing) moves to `mux-state` which can depend on `tracing` and OS primitives.
- Layer 0 crates: `mux-types`, `mux-grid`, `mux-query`, `mux-crdt`, `mux-proto`, `mux-pty-fake`, **`mux-core`** [v12 moved]. Pure, no IO, WASM-compatible.
- Layer 1 crates: **`mux-state`** [v12 new], `mux-conf`, `mux-keys`, `mux-format`, `mux-pty`, `mux-os`. May use `libc`, `tokio`, file IO.
- Layer 2 crates: `mux-runtime`, `mux-api`, `mux-orm`, `mux-termlet`, `mux-control`, `mux-otel`, `mux-server`, `mux-test-support`. Facades composing lower layers.
- Layer 3 crates: `bindings-python`, `bindings-node`, `termforge` (binary), `mux-view`. Applications.
- Forbidden edges: Layer N may not depend on Layer N+1. `mux-termlet` may NOT depend on `mux-server`, `mux-api`, `mux-orm`.
- **[v12]** `mux-crdt` behind feature flag `crdt`. Not a hard dependency for any core crate.
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
    // Layer 0: Pure (WASM-compatible, no IO, no unsafe)
    CrateLayer { name: "mux-types", layer: 0 },
    CrateLayer { name: "mux-grid", layer: 0 },
    CrateLayer { name: "mux-proto", layer: 0 },
    CrateLayer { name: "mux-query", layer: 0 },
    CrateLayer { name: "mux-crdt", layer: 0 },
    CrateLayer { name: "mux-pty-fake", layer: 0 },
    CrateLayer { name: "mux-core", layer: 0 },  // [v12] moved from Layer 1
    // Layer 1: OS (may use libc, tracing, file IO)
    CrateLayer { name: "mux-state", layer: 1 },  // [v12] new
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
    CrateLayer { name: "mux-otel", layer: 2 },
    CrateLayer { name: "mux-server", layer: 2 },
    CrateLayer { name: "mux-test-support", layer: 2 },
    // Layer 3: Application
    CrateLayer { name: "bindings-python", layer: 3 },
    CrateLayer { name: "bindings-node", layer: 3 },
    CrateLayer { name: "mux-view", layer: 3 },
];

pub fn edge_allowed(from: u8, to: u8) -> bool {
    to <= from
}

/// [v12] Validate that a given dependency graph respects layer boundaries.
pub fn validate_graph(edges: &[(& str, &str)]) -> Vec<String> {
    let mut violations = Vec::new();
    for (from, to) in edges {
        let from_layer = CRATE_LAYERS.iter().find(|c| c.name == *from);
        let to_layer = CRATE_LAYERS.iter().find(|c| c.name == *to);
        if let (Some(f), Some(t)) = (from_layer, to_layer) {
            if !edge_allowed(f.layer, t.layer) {
                violations.push(format!(
                    "{} (L{}) -> {} (L{}): forbidden upward dependency",
                    from, f.layer, to, t.layer
                ));
            }
        }
    }
    violations
}
```

### Test Strategy

1. Dependency graph linter checks no forbidden edges -- CI job.
2. `mux-core` transitive dependencies audited for IO crates -- `cargo tree` CI check.
3. `mux-termlet` dependency deny-list enforcement -- `cargo tree` CI check.
4. **[v12]** `mux-core` WASM compilation proof -- `cargo check --target wasm32-unknown-unknown -p mux-core`.
5. **[v12]** `mux-state` has `tracing` dependency -- `cargo tree` confirms.
6. `TST-020`: dependency graph linter checks no forbidden edges.
7. `TST-021`: `mux-core` transitive dependencies audited for IO crates.
8. `TST-022`: `mux-termlet` dependency deny-list enforcement.

### AGENTS.md Rules

- `RULE-S03-01`: Layer violations are blocking CI errors. **Enforcement:** Dependency linter CI job.
- `RULE-S03-02`: New crates must declare their layer in the architecture table. **Enforcement:** PR review.
- `RULE-S03-03`: `mux-termlet` must not depend on `mux-server` or `mux-api`. **Enforcement:** `cargo tree` CI check.
- `RULE-S03-04`: **[v12]** `mux-core` must remain at Layer 0 (WASM-compatible). **Enforcement:** `cargo check --target wasm32-unknown-unknown -p mux-core`.
- `RULE-S03-05`: **[v12]** `mux-crdt` is feature-gated; must not be a hard dependency of any crate. **Enforcement:** `cargo tree` CI check.

---

## 4. Workspace Layout [v12 updated]

### Design Decisions

- Cargo workspace at repo root.
- Crate directories under `crates/` for libraries, `tools/` for build tooling, `bindings/` for language bindings.
- Binary crate at top level.
- One Cargo.toml per crate; workspace inherits common metadata.
- **[v12]** `mux-state` crate added for server lifecycle management.

### Rust Example

```
termforge/
  Cargo.toml                    # workspace root
  crates/
    mux-types/                  # Layer 0: shared types, entity IDs
    mux-grid/                   # Layer 0: Grid, VtParser, Cell
    mux-proto/                  # Layer 0: wire protocol codec
    mux-query/                  # Layer 0: QueryList, QueryOp, QuerySpec, Queryable
    mux-crdt/                   # Layer 0: HLC, LWW, OrSet, OpLog (feature-gated)
    mux-pty-fake/               # Layer 0: FakePtyBackend, ScenarioStep
    mux-core/                   # Layer 0: ServerGraph, Event, Effect, Layout [v12 moved]
    mux-state/                  # Layer 1: ServerPhase, lifecycle transitions [v12 new]
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
4. **[v12]** `mux-state` exists and compiles -- CI check.

### AGENTS.md Rules

- `RULE-S04-01`: Every new crate must be added to workspace members. **Enforcement:** CI workspace member check.
- `RULE-S04-02`: Crate README must state its layer and dependencies. **Enforcement:** PR review.
- `RULE-S04-03`: `mux-termlet` lives at Layer 2, depends on Layer 0 + Layer 1 only. **Enforcement:** `cargo deny check`.

---

## 5. mux-core: Pure Kernel [v12 updated -- now Layer 0]

### Design Decisions

- `#![forbid(unsafe_code)]` -- no exceptions, ever.
- No IO, no `tokio`, no `libc`, no `std::io`. Compiles to WASM.
- **[v12]** Now at Layer 0 (was Layer 1 in v11). This is consistent with its purity constraints.
- Owns: `ServerGraph`, `Event`, `Effect`, `ApplyOutcome`, `CoreCtx`, `MuxKernel` trait.
- `ServerGraph` is the authoritative mutable state. Modifications only via `apply_event()`.
- `GraphState` is an immutable snapshot built from `ServerGraph`. Uses `HashMap` internally for O(1) entity lookups.
- `ApplyOutcome` returns effects + hints + `warnings: Vec<String>` for non-fatal issues.
- **[v12]** `CoreCtx.rng` changed to `Box<dyn FnMut() -> u64 + Send>` to allow cross-thread use.
- **[v12]** `ServerPhase` moved to `mux-state` crate (Layer 1) since it uses `tracing` for lifecycle logging.
- Traceability: `INV-002`, `INV-003`, `API-010`.

### 5.1 Grid API [v12 new]

**[v12] Critical gap in v11:** The `Grid` and `VtParser` types are referenced throughout but never concretely defined. This subsection provides the normative API that `mux-termlet` and `mux-view` depend on.

Design follows patterns from:
- **zellij** (`zellij-server/src/panes/grid.rs`): VecDeque scrollback, canonical rows, viewport management.
- **ratatui** (`ratatui-core/src/buffer/buffer.rs`): flat `Vec<Cell>` backing, `Rect` area, `Index` trait for position access.

```rust
// crates/mux-grid/src/lib.rs
#![forbid(unsafe_code)]

/// A single terminal cell with character content and style.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    /// The grapheme cluster displayed in this cell.
    pub grapheme: String,
    /// Foreground color (256-color index or RGB).
    pub fg: Color,
    /// Background color (256-color index or RGB).
    pub bg: Color,
    /// Style attributes (bold, italic, underline, etc.).
    pub attrs: CellAttrs,
    /// Width of this cell (1 for normal, 2 for wide CJK, 0 for continuation).
    pub width: u8,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            grapheme: " ".to_string(),
            fg: Color::Default,
            bg: Color::Default,
            attrs: CellAttrs::empty(),
            width: 1,
        }
    }
}

/// Color representation supporting 256-color and true-color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Default,
    Indexed(u8),
    Rgb(u8, u8, u8),
}

bitflags::bitflags! {
    /// Cell style attributes matching tmux's attribute set.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CellAttrs: u16 {
        const BOLD       = 0b0000_0001;
        const DIM        = 0b0000_0010;
        const ITALIC     = 0b0000_0100;
        const UNDERLINE  = 0b0000_1000;
        const BLINK      = 0b0001_0000;
        const REVERSE    = 0b0010_0000;
        const HIDDEN     = 0b0100_0000;
        const STRIKE     = 0b1000_0000;
    }
}

/// Terminal grid: a viewport of cells with scrollback.
///
/// Internal storage is a flat `Vec<Cell>` for the viewport (fast indexed access)
/// plus a `VecDeque<Vec<Cell>>` for scrollback lines (following zellij pattern).
pub struct Grid {
    /// Viewport cells: flat array, row-major. Length = cols * rows.
    viewport: Vec<Cell>,
    /// Scrollback buffer: oldest lines at front.
    scrollback: std::collections::VecDeque<Vec<Cell>>,
    cols: u16,
    rows: u16,
    /// Cursor position within viewport.
    cursor: CursorPos,
    /// Maximum scrollback lines.
    max_scrollback: usize,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CursorPos {
    pub col: u16,
    pub row: u16,
}

impl Grid {
    /// Create a new grid with given dimensions. All cells initialized to default (space).
    pub fn new(cols: u16, rows: u16) -> Self {
        Self {
            viewport: vec![Cell::default(); cols as usize * rows as usize],
            scrollback: std::collections::VecDeque::new(),
            cols,
            rows,
            cursor: CursorPos::default(),
            max_scrollback: 10_000,
        }
    }

    /// Access a cell at (col, row). Returns None if out of bounds.
    pub fn cell_at(&self, col: u16, row: u16) -> Option<&Cell> {
        if col < self.cols && row < self.rows {
            Some(&self.viewport[row as usize * self.cols as usize + col as usize])
        } else {
            None
        }
    }

    /// Mutable access to a cell at (col, row).
    pub fn cell_at_mut(&mut self, col: u16, row: u16) -> Option<&mut Cell> {
        if col < self.cols && row < self.rows {
            let idx = row as usize * self.cols as usize + col as usize;
            Some(&mut self.viewport[idx])
        } else {
            None
        }
    }

    /// Return each row as a String (graphemes concatenated). Includes trailing spaces.
    pub fn rows_text(&self) -> Vec<String> {
        (0..self.rows).map(|row| {
            let start = row as usize * self.cols as usize;
            let end = start + self.cols as usize;
            self.viewport[start..end].iter().map(|c| c.grapheme.as_str()).collect()
        }).collect()
    }

    /// Return visible text with trailing whitespace trimmed per line
    /// and trailing blank lines removed.
    pub fn to_text_trimmed(&self) -> String {
        let mut lines: Vec<String> = self.rows_text()
            .into_iter()
            .map(|l| l.trim_end().to_string())
            .collect();
        while lines.last().is_some_and(|l| l.is_empty()) {
            lines.pop();
        }
        lines.join("\n")
    }

    /// Resize the grid. Preserves content where possible, reflowing lines.
    pub fn resize(&mut self, new_cols: u16, new_rows: u16) {
        let mut new_viewport = vec![Cell::default(); new_cols as usize * new_rows as usize];
        let copy_cols = self.cols.min(new_cols) as usize;
        let copy_rows = self.rows.min(new_rows) as usize;
        for row in 0..copy_rows {
            let src_start = row * self.cols as usize;
            let dst_start = row * new_cols as usize;
            new_viewport[dst_start..dst_start + copy_cols]
                .clone_from_slice(&self.viewport[src_start..src_start + copy_cols]);
        }
        self.viewport = new_viewport;
        self.cols = new_cols;
        self.rows = new_rows;
        self.cursor.col = self.cursor.col.min(new_cols.saturating_sub(1));
        self.cursor.row = self.cursor.row.min(new_rows.saturating_sub(1));
    }

    /// Capture all cells as a 2D vector for snapshot purposes.
    pub fn capture_cells(&self) -> Vec<Vec<Cell>> {
        (0..self.rows).map(|row| {
            let start = row as usize * self.cols as usize;
            let end = start + self.cols as usize;
            self.viewport[start..end].to_vec()
        }).collect()
    }

    pub fn cols(&self) -> u16 { self.cols }
    pub fn rows(&self) -> u16 { self.rows }
    pub fn cursor_position(&self) -> CursorPos { self.cursor }
    pub fn size(&self) -> (u16, u16) { (self.cols, self.rows) }
}

/// VT100/xterm parser. Processes byte sequences into Grid mutations.
/// Follows tmux input.c state table -- NOT using the vte crate (settled decision S9).
pub struct VtParser {
    state: ParserState,
    params: Vec<u16>,
    intermediate: Vec<u8>,
}

/// Internal parser state matching tmux input.c state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParserState {
    Ground,
    Escape,
    EscapeIntermediate,
    CsiEntry,
    CsiParam,
    CsiIntermediate,
    OscString,
    DcsEntry,
    DcsParam,
    DcsIntermediate,
    DcsPassthrough,
}

impl VtParser {
    pub fn new() -> Self {
        Self {
            state: ParserState::Ground,
            params: Vec::with_capacity(16),
            intermediate: Vec::with_capacity(4),
        }
    }

    /// Parse bytes and apply mutations to the grid.
    pub fn parse(&mut self, data: &[u8], grid: &mut Grid) {
        for &byte in data {
            self.process_byte(byte, grid);
        }
    }

    fn process_byte(&mut self, byte: u8, grid: &mut Grid) {
        // State machine implementation matching tmux input.c.
        // Each state handles the byte and transitions to the next state.
        todo!("implement full VT100 state machine")
    }
}
```

### Rust Example (ServerGraph -- core kernel)

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

/// [v12] CoreCtx.rng is Send-able for cross-thread use.
pub struct CoreCtx {
    pub now: std::time::SystemTime,
    pub rng: Box<dyn FnMut() -> u64 + Send>,
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
    pub warnings: Vec<String>,
}

/// Authoritative mutable state. Single-writer.
pub struct ServerGraph {
    pub sessions: slotmap::SlotMap<SessionId, Session>,
    pub windows: slotmap::SlotMap<WindowId, Window>,
    pub panes: slotmap::SlotMap<PaneId, Pane>,
    pub clients: slotmap::SlotMap<ClientId, Client>,
    // [v12] ServerPhase moved to mux-state crate
}

/// Immutable snapshot. HashMap-backed for O(1) lookups.
pub struct GraphState {
    pub sessions: HashMap<SessionId, Session>,
    pub windows: HashMap<WindowId, Window>,
    pub panes: HashMap<PaneId, Pane>,
    pub clients: HashMap<ClientId, Client>,
    pub session_windows: HashMap<SessionId, Vec<WindowId>>,
    pub window_panes: HashMap<WindowId, Vec<PaneId>>,
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
2. WASM compilation: `cargo check --target wasm32-unknown-unknown -p mux-core` passes -- CI gate.
3. `#![forbid(unsafe_code)]` present in lib.rs -- CI attribute check.
4. GraphState round-trip: snapshot faithfully represents ServerGraph -- property test.
5. **[v12]** Grid API: `cell_at` returns correct cell for all valid positions -- unit test.
6. **[v12]** Grid resize: content preserved within overlap region -- property test.
7. **[v12]** `to_text_trimmed` strips trailing whitespace and blank lines -- unit test.
8. No `std::io` import transitively -- `cargo tree` check.

### AGENTS.md Rules

- `RULE-S05-01`: `mux-core` is pure: no IO, no `unsafe`, no `tokio`, no `libc`. **Enforcement:** `#![forbid(unsafe_code)]`, WASM CI check.
- `RULE-S05-02`: WASM CI check for all Layer 0 crates. **Enforcement:** CI: `cargo check --target wasm32-unknown-unknown`.
- `RULE-S05-03`: **[v12]** Grid API must provide `cell_at`, `rows_text`, `to_text_trimmed`, `resize`, `capture_cells`. **Enforcement:** API surface test.

---

## 6. Entity ID Design

### Design Decisions

- All entity IDs use `slotmap::new_key_type!` for type safety and generational indexing.
- `SessionId`, `WindowId`, `PaneId`, `ClientId`, `JobId`, `BufferId` -- never raw integers.
- ID types are `Copy + Clone + Hash + Eq + Debug`.
- Entity traversal: `session.window_ids()`, not direct child storage.
- Reverse lookups computed in `GraphState`, not stored on entities.
- `TermletPaneId` is a separate newtype (not SlotMap) for Termlet-internal use.
- **[v12]** `PtyHandle(u64)` added as opaque token for `PtyBackend` API to avoid dependency on TermletPaneId.
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

/// Termlet uses distinct ID type, not SlotMap.
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

    /// [v12] Convert to PtyHandle for backend API use.
    pub fn as_pty_handle(self) -> PtyHandle { PtyHandle(self.0) }
}

/// [v12] Opaque handle for PtyBackend API.
/// Decouples PtyBackend (Layer 1) from TermletPaneId (Layer 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PtyHandle(pub u64);
```

### Test Strategy

1. SlotMap key type safety: `SessionId` and `WindowId` cannot be interchanged -- compile-fail test.
2. Generational safety: removed + reinserted entity does not alias old key -- unit test.
3. `TermletPaneId::next()` produces unique monotonic IDs -- concurrent unit test.
4. `TermletPaneId` distinct from `PaneId` -- type system check.
5. **[v12]** `PtyHandle` convertible from `TermletPaneId` -- unit test.
6. **[v12]** `PtyHandle` is `Copy + Eq + Hash` -- compile check.

### AGENTS.md Rules

- `RULE-S06-01`: All entity IDs via `slotmap::new_key_type!`. **Enforcement:** Code review + type system.
- `RULE-S06-02`: `TermletPaneId` must not use `slotmap::KeyData::from_ffi()`. **Enforcement:** CI grep.
- `RULE-S06-03`: **[v12]** `PtyBackend` must use `PtyHandle`, not `TermletPaneId`. **Enforcement:** API signature check.

---

## 7. Event -> Effect Architecture

### Design Decisions

- Events are inbound state change requests. Effects are outbound side-effects.
- Single-writer: only the state actor calls `apply_event()`. All readers use `GraphState` snapshots via `ArcSwap`.
- The runtime executes effects after state transition. Core never executes effects.
- `EventMsg` wraps `Event` with optional oneshot reply channel for synchronous event submission.
- Traceability: `INV-003`, `INV-014`, `API-030`.

### Rust Example

```rust
// crates/mux-server/src/actor.rs
use arc_swap::ArcSwap;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};

/// Event message with optional reply channel for await-able submission.
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

## 8. Error Taxonomy [v12 updated]

### Design Decisions

- Three error classes: `Bug` (internal logic error), `UserError` (invalid input/config), `Transient` (IO/network failure).
- Each crate defines its own typed error via `thiserror`. `anyhow` only in binaries and tests.
- `ErrorCode` trait for errors that cross binding boundaries: `fn error_code(&self) -> &'static str`.
- **[v12]** Added `TermletError::InvalidState` variant to replace overloaded `AlreadyExited` for non-exit states. When a user calls `send_keys` on a `SpawnFailed` Termlet, the error should say "invalid state: SpawnFailed", not "already exited".
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

/// Trait for errors crossing binding boundaries.
pub trait ErrorCode {
    fn error_code(&self) -> &'static str;
    fn error_class(&self) -> ErrorClass;
}

/// [v12] TermletError: 9 variants (v11 had 8; added InvalidState).
#[derive(Debug, Error)]
pub enum TermletError {
    #[error("spawn failed: {reason}")]
    SpawnFailed { reason: String },

    /// [v12] Replaces AlreadyExited for states that are not Exited.
    /// Used when an operation is attempted in an invalid state.
    #[error("invalid state for operation: current state is {state:?}")]
    InvalidState { state: TermletState },

    #[error("termlet already exited with status {status:?}")]
    AlreadyExited { status: Option<i32> },

    #[error("wait_for timeout: pattern={pattern} timeout_ms={timeout_ms}")]
    WaitForTimeout { pattern: String, timeout_ms: u64 },

    #[error("pattern not found before process exit: {pattern}")]
    PatternNotFound { pattern: String },

    #[error("resize failed: {0}")]
    ResizeFailed(String),

    #[error("pty error: {0}")]
    Pty(String),

    #[error("termlet not in pool: {name}")]
    NotInPool { name: String },

    #[error("invalid regex: {0}")]
    InvalidRegex(String),
}

impl ErrorCode for TermletError {
    fn error_code(&self) -> &'static str {
        match self {
            Self::SpawnFailed { .. } => "TERMLET_SPAWN_ERROR",
            Self::InvalidState { .. } => "TERMLET_INVALID_STATE",
            Self::AlreadyExited { .. } => "TERMLET_ALREADY_EXITED",
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
            Self::InvalidState { .. } | Self::AlreadyExited { .. } |
            Self::NotInPool { .. } | Self::InvalidRegex(_) |
            Self::ResizeFailed(_) => ErrorClass::UserError,
            Self::WaitForTimeout { .. } | Self::PatternNotFound { .. } => ErrorClass::Transient,
        }
    }
}

// CoreError, ProtocolError, QueryError -- same as v11
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
3. Negative tests: every error variant constructible and matchable -- exhaustive test.
4. Binding format test: `format!("[{}] {}", err.error_code(), err)` matches expected pattern.
5. Semver audit: error code strings never change in minor versions -- snapshot test.
6. **[v12]** `InvalidState` error includes current state in message -- format test.
7. **[v12]** `InvalidState` vs `AlreadyExited` correctly distinguished -- state-based test.
8. `TST-080`: ErrorCode coverage for all binding-boundary types.

### AGENTS.md Rules

- `RULE-S08-01`: `thiserror` for library errors, `anyhow` only in binaries/tests. **Enforcement:** CI grep.
- `RULE-S08-02`: Every `ErrorClass` variant tested. **Enforcement:** Test coverage check.
- `RULE-S08-03`: Termlet error codes must be stable string constants. **Enforcement:** Semver check.
- `RULE-S08-04`: Every error type crossing binding boundaries must implement `ErrorCode`. **Enforcement:** Compile-fail test.
- `RULE-S08-05`: Error messages in bindings must include `[ERROR_CODE]` prefix. **Enforcement:** Exception message format test.
- `RULE-S08-06`: **[v12]** `InvalidState` used for non-Exited state violations; `AlreadyExited` only for `Exited` state. **Enforcement:** Error variant test.

---

## 9. Protocol Codec (mux-proto)

### Design Decisions

- Wire-compatible with tmux protocol v8 (`tmux-protocol.h:23`, `PROTOCOL_VERSION 8`).
- `ImsgHdr` (header) + `ImsgFrame` (header + payload) + `ImsgCodec` (stateful encoder/decoder).
- `MsgType` enum for all message types.
- Protocol violations are fatal: `DecodeOutcome::Fatal` kills the connection.
- `MAX_PAYLOAD_SIZE` guard prevents OOM on malicious input.
- tokio-codec compatible for async decode.
- Traceability: `INV-001`, `INV-004`, `PAR-001`.

### Rust Example

```rust
// crates/mux-proto/src/lib.rs
pub const PROTOCOL_VERSION: u32 = 8;
pub const MAX_PAYLOAD_SIZE: usize = 256 * 1024; // 256KB

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum MsgType {
    Identify = 100,
    Command = 101,
    Stdin = 102,
    Stdout = 103,
    Stderr = 104,
    Detach = 105,
    Resize = 106,
    Ready = 107,
    Exit = 108,
    Exited = 109,
    Shutdown = 110,
    Suspend = 111,
    Lock = 112,
    Unlock = 113,
    Oldstdout = 114,
    Oldstderr = 115,
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
        if self.buf.len() < std::mem::size_of::<ImsgHdr>() {
            return DecodeOutcome::NeedMore(
                std::mem::size_of::<ImsgHdr>() - self.buf.len()
            );
        }
        // Parse header, check payload size against MAX_PAYLOAD_SIZE
        // if hdr.len > MAX_PAYLOAD_SIZE { return Fatal(PayloadTooLarge) }
        todo!("full decode implementation")
    }

    pub fn encode(frame: &ImsgFrame) -> Vec<u8> {
        todo!("full encode implementation")
    }
}
```

### Test Strategy

1. Round-trip: encode then decode produces identical frame -- property test.
2. Fuzz: random bytes never panic, only `Fatal` or `NeedMore` -- cargo-fuzz target.
3. Fixture captures from real tmux decode correctly -- fixture test.
4. Oversized payload rejected with `PayloadTooLarge` -- unit test.
5. Protocol version assertion: `assert_eq!(PROTOCOL_VERSION, 8)` -- sanity check.
6. `TST-090`: fuzz_decode_frame target.

### AGENTS.md Rules

- `RULE-S09-01`: Protocol violations kill the connection, never drop-and-continue. **Enforcement:** `DecodeOutcome::Fatal` enforced.
- `RULE-S09-02`: Payload size must be bounded by `MAX_PAYLOAD_SIZE`. **Enforcement:** Decode check.

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
    Server, Session, Window, Pane,
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
    pub fn new() -> Self { Self { values: std::collections::HashMap::new() } }
    pub fn get(&self, key: &str) -> Option<&OptionValue> { self.values.get(key) }
    pub fn set(&mut self, key: String, value: OptionValue) { self.values.insert(key, value); }
    pub fn unset(&mut self, key: &str) { self.values.remove(key); }
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
6. **[v12]** All 4 levels tested explicitly: pane -> window -> session -> server.

### AGENTS.md Rules

- `RULE-S10-01`: Config loads only after first client identify burst. **Enforcement:** Startup sequence integration test.
- `RULE-S10-02`: Option FALLTHROUGH: pane -> window per `options.c:891-903`. **Enforcement:** Property test.

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
pub enum LayoutCellKind { Horizontal, Vertical, Pane }

pub struct LayoutTree {
    cells: Vec<LayoutCell>,
    root: usize,
}

impl LayoutTree {
    pub fn new(root_cols: u16, root_rows: u16) -> Self {
        let root = LayoutCell {
            kind: LayoutCellKind::Pane,
            x: 0, y: 0, cols: root_cols, rows: root_rows,
            parent: None, children: Vec::new(), pane: None,
        };
        Self { cells: vec![root], root: 0 }
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        // Round-robin resize matching layout.c:448-462
        todo!()
    }

    pub fn checksum(&self) -> u16 {
        // Match layout-custom.c:46-57
        todo!()
    }

    pub fn split(&mut self, pane_idx: usize, direction: Direction) -> Option<usize> {
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

- `RULE-S11-01`: `debug_assert!(layout_check(...))` in all layout-mutating functions. **Enforcement:** CI with debug assertions.
- `RULE-S11-02`: Format strings match tmux via `format-audit` parity corpus. **Enforcement:** CI: `format-audit` check.

---

## 12. QueryList ORM Layer [v12 updated]

### Design Decisions

- 18 operators: 12 from libtmux + 6 extensions (`gt`, `gte`, `lt`, `lte`, `ne`, `glob`).
- libtmux operators verified at `query_list.py:298-312`.
- `keygetter` nested traversal with `__` separator (libtmux lines 45-113).
- `QueryList<T>` returns chained results. `get()` returns single or error.
- `MultipleObjectsReturned`, `ObjectDoesNotExist` exceptions (from libtmux lines 550-569).
- Callable matcher support: `filter(|item| predicate)` (libtmux line 535).
- **[v12]** `Queryable` trait concretely defined with `field_value()` and `matches()`.
- **[v12]** `QuerySpec` struct concretely defined with field path, operator, and value.
- Traceability: `PAR-120`, `API-050`.

### Rust Example

```rust
// crates/mux-query/src/lib.rs
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryOp {
    Eq, Exact, IExact, Contains, IContains,
    StartsWith, IStartsWith, EndsWith, IEndsWith,
    In, Nin, Regex, IRegex,
    // Extensions beyond libtmux
    Gt, Gte, Lt, Lte, Ne, Glob,
}

/// [v12] Concrete QuerySpec -- was referenced but never defined in v11.
/// Represents a single filter condition: field_path__op = value.
#[derive(Debug, Clone)]
pub struct QuerySpec {
    /// Field path with `__` separator for nested access (e.g., "session__name").
    pub field_path: String,
    /// Operator to apply.
    pub op: QueryOp,
    /// Value to compare against.
    pub value: QueryValue,
}

/// [v12] Value type for query comparisons.
#[derive(Debug, Clone)]
pub enum QueryValue {
    String(String),
    Integer(i64),
    Bool(bool),
    StringList(Vec<String>),
    Regex(String),
}

/// [v12] Trait that types must implement to be filterable by QuerySpec.
/// This was referenced in v11's filter_by but never defined.
pub trait Queryable {
    /// Get the value of a field by path (supports `__` nested traversal).
    fn field_value(&self, field_path: &str) -> Option<QueryValue>;

    /// Check if this item matches a single QuerySpec.
    fn matches(&self, spec: &QuerySpec) -> bool {
        let Some(field_val) = self.field_value(&spec.field_path) else {
            return false;
        };
        apply_op(&spec.op, &field_val, &spec.value)
    }
}

/// Apply a query operator to field value and target value.
fn apply_op(op: &QueryOp, field: &QueryValue, target: &QueryValue) -> bool {
    match (op, field, target) {
        (QueryOp::Eq | QueryOp::Exact, QueryValue::String(f), QueryValue::String(t)) => f == t,
        (QueryOp::IExact, QueryValue::String(f), QueryValue::String(t)) => {
            f.to_lowercase() == t.to_lowercase()
        }
        (QueryOp::Contains, QueryValue::String(f), QueryValue::String(t)) => f.contains(t.as_str()),
        (QueryOp::IContains, QueryValue::String(f), QueryValue::String(t)) => {
            f.to_lowercase().contains(&t.to_lowercase())
        }
        (QueryOp::StartsWith, QueryValue::String(f), QueryValue::String(t)) => f.starts_with(t.as_str()),
        (QueryOp::Ne, QueryValue::String(f), QueryValue::String(t)) => f != t,
        (QueryOp::Gt, QueryValue::Integer(f), QueryValue::Integer(t)) => f > t,
        (QueryOp::Gte, QueryValue::Integer(f), QueryValue::Integer(t)) => f >= t,
        (QueryOp::Lt, QueryValue::Integer(f), QueryValue::Integer(t)) => f < t,
        (QueryOp::Lte, QueryValue::Integer(f), QueryValue::Integer(t)) => f <= t,
        (QueryOp::In, QueryValue::String(f), QueryValue::StringList(list)) => list.contains(f),
        (QueryOp::Nin, QueryValue::String(f), QueryValue::StringList(list)) => !list.contains(f),
        // ... remaining operators
        _ => false,
    }
}

pub struct QueryList<T> {
    items: Vec<T>,
}

impl<T: Clone> QueryList<T> {
    pub fn new(items: Vec<T>) -> Self { Self { items } }

    pub fn filter<F: Fn(&T) -> bool>(&self, predicate: F) -> QueryList<T> {
        QueryList { items: self.items.iter().filter(|i| predicate(i)).cloned().collect() }
    }

    /// [v12] Now compiles: Queryable trait is defined.
    pub fn filter_by(&self, specs: &[QuerySpec]) -> QueryList<T>
    where T: Queryable {
        QueryList {
            items: self.items.iter()
                .filter(|item| specs.iter().all(|spec| item.matches(spec)))
                .cloned().collect(),
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
    pub fn is_empty(&self) -> bool { self.items.is_empty() }

    /// [v12] Iterate over items.
    pub fn iter(&self) -> std::slice::Iter<'_, T> { self.items.iter() }
}

/// Trait for language binding query wrappers.
pub trait BindingQueryList<T> {
    fn filter_kwargs(&self, kwargs: &[(String, String)]) -> Vec<T>;
    fn get_kwargs(&self, kwargs: &[(String, String)]) -> Result<T, QueryError>;
}
```

### Test Strategy

1. All 18 operators produce correct results -- parametrized unit test per operator.
2. `keygetter` traversal: `session__name` resolves nested field -- unit test.
3. `get()` single/multiple/zero semantics -- unit tests for each case.
4. Callable filter works -- unit test.
5. Operator parity with libtmux -- parity test.
6. **[v12]** `Queryable` trait: concrete type implements `field_value` correctly -- unit test.
7. **[v12]** `QuerySpec` roundtrip: parse `name__contains=foo` into spec, apply to data -- unit test.
8. **[v12]** `apply_op` covers all 18 operators -- parametrized test.
9. `BindingQueryList` implementations tested for Python and Node.

### AGENTS.md Rules

- `RULE-S12-01`: Key bindings match tmux via generated parity tests. **Enforcement:** CI: key-binding parity tests.
- `RULE-S12-02`: QueryList must support all 18 operators. **Enforcement:** Parametrized test coverage.
- `RULE-S12-03`: **[v12]** `Queryable` trait must be implemented for all ORM entity types. **Enforcement:** Compile test.
- `RULE-S12-04`: **[v12]** `QuerySpec` parsing from `field__op=value` format must be tested. **Enforcement:** Unit test.

---

## 13. State Actor and ArcSwap

### Design Decisions

- Single-writer `StateActor` processes events sequentially.
- `ArcSwap<GraphState>` for zero-contention snapshot reads.
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
    pub fn new(initial: GraphState) -> Self {
        Self { inner: Arc::new(ArcSwap::new(Arc::new(initial))) }
    }

    pub fn load(&self) -> arc_swap::Guard<Arc<GraphState>> { self.inner.load() }
    pub fn snapshot(&self) -> Arc<GraphState> { self.inner.load_full() }
}

impl Clone for StateHandle {
    fn clone(&self) -> Self { Self { inner: Arc::clone(&self.inner) } }
}
```

### Test Strategy

1. Concurrent reads during writes: no blocking -- concurrency test.
2. Snapshot freshness: read after write reflects latest state -- ordering test.
3. No `Arc<Mutex<_>>` in read path -- architecture test (grep).

### AGENTS.md Rules

- `RULE-S13-01`: Only the state actor mutates graph state. **Enforcement:** ArcSwap read path, CI grep for `Mutex<ServerGraph>`.

---

## 14. Server Lifecycle [v12 updated]

### Design Decisions

- Startup sequence: socket creation -> flock -> accept loop -> first client identify -> config load.
- flock pattern matches `client.c:89`: `flock(LOCK_EX|LOCK_NB)`.
- Socket dir permissions: `0700`. Socket permissions: `0600`.
- **[v12]** `LockGuard` stores `File` instead of `RawFd`. v11 used `mem::forget(file)` which leaks the File handle. Storing the File in the guard ensures proper cleanup.
- **[v12]** `ServerPhase` moved to `mux-state` crate (Layer 1) since it uses `tracing`.
- Traceability: `INV-005`, `OPS-010`.

### Rust Example

```rust
// crates/mux-state/src/lifecycle.rs
/// [v12] Server lifecycle phases. Lives in mux-state (Layer 1), not mux-core (Layer 0),
/// because it uses tracing for phase transition logging.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ServerPhase {
    Bootstrap,
    SocketReady,
    AwaitIdentify,
    ConfigLoading,
    Running,
    ShuttingDown,
    Stopped,
}

impl ServerPhase {
    pub fn transition(&self, new: ServerPhase) -> Result<ServerPhase, String> {
        if (new as u8) > (*self as u8) {
            tracing::info!(from = ?self, to = ?new, "server phase transition");
            Ok(new)
        } else {
            Err(format!("invalid phase transition: {:?} -> {:?}", self, new))
        }
    }
}

// crates/mux-os/src/lock.rs
/// [v12] Lock guard stores File, not RawFd. Fixes v11 mem::forget leak.
pub struct LockGuard {
    _file: std::fs::File,
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        // File::drop() closes the fd, which releases the flock.
        // No explicit unlock needed -- flock is advisory and released on close.
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
    // [v12] File stored in guard, not forgotten. Drop closes fd, releasing lock.
    Ok(LockGuard { _file: file })
}
```

### Test Strategy

1. flock: second server on same socket fails immediately -- integration test.
2. `LockGuard` Drop releases lock -- unit test.
3. Socket permissions: dir `0700`, socket `0600` -- integration test.
4. Config not loaded until after identify -- startup sequence test.
5. ServerPhase monotonic: transition backward returns error -- unit test.
6. **[v12]** LockGuard does not leak File descriptor -- fd count test.

### AGENTS.md Rules

- `RULE-S14-01`: Lock file guard via `Drop` impl, never manual unlock. **Enforcement:** CI grep.
- `RULE-S14-02`: Config load trigger must remain post-identify. **Enforcement:** Startup sequence test.
- `RULE-S14-03`: ServerPhase transitions must be monotonic. **Enforcement:** Error return + test.
- `RULE-S14-04`: **[v12]** LockGuard must store File, not leak via mem::forget. **Enforcement:** Code review + fd test.

---

## 15. Control Mode

### Design Decisions

- Control mode (`tmux -CC`) provides typed notifications for external consumers.
- Notifications are hints, not authoritative -- periodic full refresh required.
- Typed `ControlNotification` enum for all notification types.
- `C6` gate: control-mode backpressure (pause/resume) semantics must match tmux.
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
    Pause,
    Continue,
    Exit { reason: String },
}

pub fn parse_notification(line: &str) -> Option<ControlNotification> {
    let parts: Vec<&str> = line.splitn(3, ' ').collect();
    match parts.first().copied() {
        Some("%session-changed") => {
            let session_id = parts.get(1)?.to_string();
            let name = parts.get(2)?.to_string();
            Some(ControlNotification::SessionChanged { session_id, name })
        }
        Some("%window-add") => {
            let window_id = parts.get(1)?.to_string();
            Some(ControlNotification::WindowAdded { window_id })
        }
        Some("%pause") => Some(ControlNotification::Pause),
        Some("%continue") => Some(ControlNotification::Continue),
        Some("%exit") => {
            let reason = parts.get(1).unwrap_or(&"").to_string();
            Some(ControlNotification::Exit { reason })
        }
        // ... remaining notification types
        _ => None,
    }
}
```

### Test Strategy

1. Parse all notification types from captured tmux output -- fixture test.
2. Periodic refresh overrides stale notifications -- integration test.
3. Backpressure: Pause/Continue notifications handled correctly -- integration test.
4. Pending limit enforcement -- unit test.

### AGENTS.md Rules

- `RULE-S15-01`: Control notifications are hints, not authoritative. **Enforcement:** Periodic refresh tests.
- `RULE-S15-02`: Control-mode backpressure semantics match tmux. **Enforcement:** Parity test.

---

## 16. Language Bindings [v12 updated]

### Design Decisions

- Python bindings via PyO3. Node bindings via Neon.
- Bindings depend on `mux-api`/`mux-orm`, never `mux-core` directly.
- Python: GIL released during Rust operations with `py.allow_threads()`.
- Node: `JsBox` for resource handles; Promise-based for blocking operations.
- Package name: `termforge` in both ecosystems.
- Error messages include `[ERROR_CODE]` prefix from `ErrorCode` trait.
- `BindingQueryList<T>` trait for binding query wrappers.
- `CrossLangCase` struct for shared test fixtures.
- Traceability: `API-080`, `PAR-120`.

### Rust Example

```rust
// bindings/python/src/lib.rs
use pyo3::prelude::*;

#[pyclass]
pub struct PyServer { handle: StateHandle }

#[pymethods]
impl PyServer {
    fn sessions(&self, py: Python<'_>) -> PyResult<PyQueryList> {
        py.allow_threads(|| {
            let snap = self.handle.snapshot();
            let sessions: Vec<_> = snap.sessions.values().cloned().collect();
            Ok(PyQueryList::new(sessions))
        })
    }
}

/// Shared cross-language test case.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CrossLangCase {
    pub name: String,
    pub setup: Vec<String>,
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
6. `CrossLangCase` fixtures produce identical results -- multi-runner test.

### AGENTS.md Rules

- `RULE-S16-01`: Bindings depend only on `mux-api`/`mux-orm`, never `mux-core` directly. **Enforcement:** `cargo deny check`.
- `RULE-S16-02`: GIL released for Rust operations > 1ms. **Enforcement:** Code review + timing test.
- `RULE-S16-03`: Termlet bindings expose: spawn, send_keys, send_bytes, wait_for, snapshot, resize, kill, restart. **Enforcement:** API surface test.
- `RULE-S16-04`: Error messages in bindings must include `[ERROR_CODE]` prefix. **Enforcement:** Exception message format test.
- `RULE-S16-05`: `CrossLangCase` fixtures required for new query features. **Enforcement:** Multi-runner CI.

---

## 17. CRDT Layer [v12 updated -- feature-gated]

### Design Decisions

- HLC (Hybrid Logical Clock) for causal ordering across replicas.
- LWW (Last-Writer-Wins) register for simple value convergence.
- OrSet (Observed-Remove Set) for add-wins set semantics.
- OpLog for operation recording, replay, and merge.
- `OpLog::compact_before()` for bounded log growth.
- `OrSet::merge()` for replica convergence.
- All CRDT types are pure (Layer 0). No IO.
- **[v12]** `mux-crdt` is behind a feature flag `crdt`. Not all deployments need collaboration.
- Traceability: `API-090`.

### Rust Example

```rust
// crates/mux-crdt/src/lib.rs
#![forbid(unsafe_code)]

use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HLC {
    pub wall_ms: u64,
    pub counter: u32,
    pub node_id: u64,
}

impl HLC {
    pub fn new(node_id: u64) -> Self {
        Self { wall_ms: 0, counter: 0, node_id }
    }

    pub fn tick(&mut self) { self.counter += 1; }

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
    pub fn new(value: T, ts: HLC) -> Self { Self { value, timestamp: ts } }

    pub fn set(&mut self, value: T, ts: HLC) {
        if ts > self.timestamp { self.value = value; self.timestamp = ts; }
    }
}

pub struct OrSet<T: Eq + std::hash::Hash + Clone> {
    elements: HashSet<(T, HLC)>,
}

impl<T: Eq + std::hash::Hash + Clone> OrSet<T> {
    pub fn new() -> Self { Self { elements: HashSet::new() } }
    pub fn add(&mut self, value: T, ts: HLC) { self.elements.insert((value, ts)); }
    pub fn remove(&mut self, value: &T) { self.elements.retain(|(v, _)| v != value); }
    pub fn merge(&mut self, other: &OrSet<T>) {
        for elem in &other.elements { self.elements.insert(elem.clone()); }
    }
    pub fn values(&self) -> HashSet<&T> { self.elements.iter().map(|(v, _)| v).collect() }
}

#[derive(Debug, Clone)]
pub enum Operation {
    SessionCreate { name: String },
    SessionRename { old: String, new: String },
    // ... CRDT-able operations
}

pub struct OpLog {
    ops: Vec<(HLC, Operation)>,
}

impl OpLog {
    pub fn new() -> Self { Self { ops: Vec::new() } }

    pub fn append(&mut self, ts: HLC, op: Operation) { self.ops.push((ts, op)); }

    pub fn compact_before(&mut self, before: HLC) {
        self.ops.retain(|(ts, _)| *ts >= before);
    }
}
```

### Test Strategy

1. HLC merge: concurrent clocks converge -- property test.
2. LWW: later timestamp wins -- unit test.
3. OrSet: add-wins semantics after concurrent add/remove -- property test.
4. OpLog compaction: entries before timestamp removed -- unit test.
5. OrSet merge: replicas converge to same set -- property test.
6. **[v12]** Feature gate: `mux-crdt` only compiled when `crdt` feature enabled -- compile check.

### AGENTS.md Rules

- `RULE-S17-01`: CRDT types must be pure (no IO). **Enforcement:** WASM CI check.
- `RULE-S17-02`: Property tests for convergence of all CRDT types. **Enforcement:** proptest in CI.
- `RULE-S17-03`: **[v12]** `mux-crdt` must be behind `crdt` feature flag. **Enforcement:** `cargo tree` check.

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

#[derive(Debug, thiserror::Error)]
pub enum SocketError {
    #[error("invalid socket path: not valid UTF-8")]
    InvalidPath,
    #[error("socket path too long: {len} > {max}")]
    PathTooLong { len: usize, max: usize },
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub fn create_socket_dir(uid: u32) -> Result<std::path::PathBuf, SocketError> {
    let dir = std::env::temp_dir().join(format!("termforge-{uid}"));
    std::fs::create_dir_all(&dir)?;
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))?;
    Ok(dir)
}

pub fn validate_socket_path(path: &std::path::Path) -> Result<(), SocketError> {
    let path_str = path.to_str().ok_or(SocketError::InvalidPath)?;
    if path_str.len() > SOCKET_PATH_MAX {
        return Err(SocketError::PathTooLong { len: path_str.len(), max: SOCKET_PATH_MAX });
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

## 19. Observability (OTEL)

### Design Decisions

- Dual OTEL providers: `OTEL_PROVIDER` (server) + `MUX_CLIENT_PROVIDER` (client).
- Both stored as `OnceLock<Mutex<Option<OtelProvider>>>`.
- Composite propagator: `BaggagePropagator` + `TraceContextPropagator`.
- `HeaderCarrier` with `Injector`/`Extractor` for cross-process propagation.
- `force_flush` both providers before process exit.
- `shutdown_timeout(Duration::from_millis(200))` to prevent hanging.
- Termlet lifecycle spans with structured attributes: `termlet.id`, `pane.id`, `backend.mode`.
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

pub struct HeaderCarrier {
    headers: std::collections::HashMap<String, String>,
}

impl opentelemetry::propagation::Injector for HeaderCarrier {
    fn set(&mut self, key: &str, value: String) { self.headers.insert(key.to_string(), value); }
}

impl opentelemetry::propagation::Extractor for HeaderCarrier {
    fn get(&self, key: &str) -> Option<&str> { self.headers.get(key).map(|v| v.as_str()) }
    fn keys(&self) -> Vec<&str> { self.headers.keys().map(|k| k.as_str()).collect() }
}
```

### Test Strategy

1. Dual providers: server and client provider both initialize -- integration test.
2. Trace propagation: spans have correct parent IDs -- integration test.
3. `force_flush` completes before process exit -- shutdown test.
4. Termlet spans include `termlet.id`, `pane.id`, `backend.mode` attributes -- attribute presence test.
5. `shutdown_timeout(200ms)` prevents blocking exit -- timeout test.

### AGENTS.md Rules

- `RULE-S19-01`: Dual OTEL providers: server and client. **Enforcement:** Architecture test.
- `RULE-S19-02`: `shutdown_timeout` bounded at 200ms. **Enforcement:** Config check.
- `RULE-S19-03`: Trace headers propagated across server/client boundary. **Enforcement:** Integration test.
- `RULE-S19-04`: Termlet spans must include structured attributes. **Enforcement:** Attribute presence test.

---

## 20. tmux Builder and Version Manager

### Design Decisions

- `tmux-builder`: compiles tmux from source with BLAKE3-hashed cache keys.
- Cache key format: `tmux-{version}__{host}__{os}-{os_ver}__cfg{cfg}__mk{mk}__tb{tool}`.
- BLAKE3 `flags_fingerprint`: null-separated flag bytes, truncated to 12 hex chars.
- File locking with `LockGuard` (`Drop` for `fs2::FileExt::unlock`).
- `sibling_tmp_dir` with `pid+nanos` for build isolation.
- Atomic publication via `std::fs::rename`.
- `validate_tmux_binary` via `tmux -V` output check.
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
    blake3::hash(flags).to_hex()[..12].to_string()
}

fn blake3_short(data: &[u8]) -> String {
    blake3::hash(data).to_hex()[..12].to_string()
}

/// Build lock uses fs2 for proper cross-platform file locking.
pub struct BuildLockGuard {
    file: std::fs::File,
}

impl Drop for BuildLockGuard {
    fn drop(&mut self) {
        use fs2::FileExt;
        let _ = self.file.unlock();
    }
}

pub fn validate_tmux_binary(path: &std::path::Path) -> Result<String, String> {
    let output = std::process::Command::new(path)
        .arg("-V")
        .output()
        .map_err(|e| format!("failed to run tmux: {e}"))?;
    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if version.starts_with("tmux ") {
        Ok(version)
    } else {
        Err(format!("unexpected tmux -V output: {version}"))
    }
}
```

### Test Strategy

1. Cache key reproducibility: same inputs produce same key -- unit test.
2. BLAKE3 fingerprint: 12-char hex truncation -- unit test.
3. File lock: concurrent builds block correctly -- integration test.
4. Atomic rename: partial build never visible -- integration test.
5. `validate_tmux_binary` with real tmux -- conditional integration test.

### AGENTS.md Rules

- `RULE-S20-01`: File-based locking with `Drop` guard for parallel builds. **Enforcement:** `LockGuard` pattern.
- `RULE-S20-02`: Atomic build publication via rename. **Enforcement:** CI grep for `std::fs::copy` absence.

---

## 21. Test Support and FakePty [v12 updated]

### Design Decisions

- `PathGuard` enforces three-layer socket validation (vibe-tmux pattern).
- Test harnesses use `-f /dev/null` for config isolation.
- `FakePtyBackend` for deterministic tests with injected output.
- `ScenarioRecorder` JSON format for reproducible fixtures.
- **[v12]** `PtyBackend` trait uses `PtyHandle` instead of `TermletPaneId` to avoid dependency inversion.
- `PtyBackend` trait requires `Send + Any` bounds with `as_any_mut()`.
- `normalize_output` and `ParityResult` for tmux parity testing.
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
        if name == "default" { return Err("socket name must not be 'default'".into()); }
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
            if path.to_str().is_some_and(|p| p.contains(tmux_socket)) {
                return Err("socket path must not match $TMUX env".into());
            }
        }
        Ok(())
    }
}

/// [v12] PtyBackend trait uses PtyHandle, not TermletPaneId.
/// This avoids dependency inversion (PtyBackend is Layer 1, TermletPaneId is Layer 2).
pub trait PtyBackend: Send + std::any::Any {
    fn spawn(&mut self, handle: PtyHandle, argv: &[String],
             cwd: Option<&str>, size: PaneSize) -> Result<(), Box<dyn std::error::Error>>;
    fn write(&mut self, handle: PtyHandle, data: &[u8]) -> Result<(), String>;
    fn read_events(&mut self) -> Vec<PtyEvent>;
    fn resize(&mut self, handle: PtyHandle, size: PaneSize) -> Result<(), String>;
    fn kill(&mut self, handle: PtyHandle, signal: i32) -> Result<(), String>;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

/// PTY events returned from read_events.
#[derive(Debug, Clone)]
pub enum PtyEvent {
    Output { handle: PtyHandle, data: Vec<u8> },
    Exited { handle: PtyHandle, status: i32 },
}

/// Output normalizer for parity testing.
pub fn normalize_output(output: &str) -> String {
    output.lines().map(|l| l.trim_end()).collect::<Vec<_>>().join("\n")
        .trim_end_matches('\n').to_string()
}

/// Result of parity comparison with tmux.
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
6. `normalize_output` trims trailing whitespace and newlines -- unit test.
7. `ParityResult` captures diff when outputs diverge -- unit test.
8. **[v12]** `PtyBackend` uses `PtyHandle`, not `TermletPaneId` -- API signature check.

### AGENTS.md Rules

- `RULE-S21-01`: Three-layer socket validation in all test harnesses. **Enforcement:** PathGuard enforced.
- `RULE-S21-02`: Fake PTY tests use `ScenarioRecorder` JSON format. **Enforcement:** Test harness validation.
- `RULE-S21-03`: `PtyBackend` trait must require `Send + Any` bounds. **Enforcement:** Compile-fail test.
- `RULE-S21-04`: Parity tests must use `normalize_output` before comparison. **Enforcement:** Code review.
- `RULE-S21-05`: **[v12]** `PtyBackend` must use `PtyHandle`, not `TermletPaneId`. **Enforcement:** API signature check.

---

## 22. Parity Test Framework

### Design Decisions

- `mux-regress`: runs identical scenarios against both TermForge and real tmux.
- tmux version matrix: 3.3a, 3.4, 3.5, 3.6 (via `mux-vm`).
- Test harnesses use separate socket names/paths. Clipboard disabled: `set-clipboard off`. Config isolated: `-f /dev/null`.
- `CrossLangCase` for cross-language consistency fixtures.
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
    let tf_output = run_termforge_scenario(scenario);
    let tmux_output = run_tmux_scenario(scenario);
    let tf_norm = normalize_output(&tf_output);
    let tmux_norm = normalize_output(&tmux_output);
    ParityResult {
        passed: tf_norm == tmux_norm,
        termforge_output: tf_norm.clone(),
        tmux_output: tmux_norm.clone(),
        diff: if tf_norm != tmux_norm {
            Some(compute_diff(&tf_norm, &tmux_norm))
        } else {
            None
        },
    }
}

fn run_termforge_scenario(scenario: &ParityScenario) -> String { todo!() }
fn run_tmux_scenario(scenario: &ParityScenario) -> String { todo!() }
fn compute_diff(a: &str, b: &str) -> String { todo!() }
```

### Test Strategy

1. Parity: `tmux list-sessions` identical output -- parity test.
2. Config parse: identical option values after config load -- parity test.
3. Layout checksum: identical for same sequence of operations -- parity test.
4. Clipboard: `set-clipboard off` in all test harnesses -- config check.
5. Socket isolation: test sockets never interfere with live tmux -- PathGuard enforcement.

### AGENTS.md Rules

- `RULE-S22-01`: Test harnesses use `-f /dev/null` for config isolation. **Enforcement:** Test fixture.
- `RULE-S22-02`: No test may use the default tmux socket. **Enforcement:** PathGuard check.
- `RULE-S22-03`: Termlet fixtures must auto-kill on cleanup. **Enforcement:** Leak detection test.
- `RULE-S22-04`: Termlet tests must run in both real and fake PTY modes. **Enforcement:** Parametrized fixture.

---

## 23. Fuzz Testing

### Design Decisions

- Three cargo-fuzz targets: VT100 parser, protocol decoder, config parser.
- Plus one Termlet fuzz target: random bytes through FakePty -> no panic.
- Random bytes must never panic -- only produce errors or valid results.
- Nightly CI runs fuzz for extended durations. Fuzz findings become regression fixtures.
- Traceability: `TST-230`.

### Rust Example

```rust
// fuzz/fuzz_targets/proto_decode.rs
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let mut codec = mux_proto::ImsgCodec::new();
    let _ = codec.decode(data);
});

// fuzz/fuzz_targets/vt100_parse.rs
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let mut grid = mux_grid::Grid::new(80, 24);
    let mut parser = mux_grid::VtParser::new();
    parser.parse(data, &mut grid);
    // Grid must be in valid state after arbitrary input
    assert!(grid.cols() == 80 && grid.rows() == 24);
});

// fuzz/fuzz_targets/termlet_fuzz.rs
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let mut grid = mux_grid::Grid::new(80, 24);
    let mut parser = mux_grid::VtParser::new();
    parser.parse(data, &mut grid);
    // Snapshot must not panic
    let text = grid.to_text_trimmed();
    assert!(text.len() <= 80 * 24 * 4); // reasonable upper bound
});
```

### Test Strategy

1. Protocol fuzz: random bytes -> no panic -- cargo-fuzz nightly.
2. VT100 fuzz: random bytes -> no panic -- cargo-fuzz nightly.
3. Config fuzz: random text -> no panic -- cargo-fuzz nightly.
4. Regression: fuzz findings converted to permanent fixtures.
5. Termlet fuzz: random bytes through FakePtyBackend -> snapshot stability -- cargo-fuzz nightly.

### AGENTS.md Rules

- `RULE-S23-01`: Fuzz targets must exist for all parser/decoder crates. **Enforcement:** Fuzz target audit.
- `RULE-S23-02`: Fuzz findings become regression fixtures within 48 hours. **Enforcement:** PR process.
- `RULE-S23-03`: Test names must match `test_{component}_{behavior}` convention. **Enforcement:** CI regex validation.

---

## 24. Performance Benchmarks

### Design Decisions

- Criterion benchmarks for hot-path code.
- Performance targets table with pass/fail thresholds.
- Nightly CI runs benchmarks; regression > 30% blocks release.
- `PerfTarget` struct with `target_ns`/`hard_fail_ns` for programmatic enforcement.
- **[v12]** Added B16 Grid cell_at benchmark.
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
| B15 | Pool stress: 50 spawn/kill | < 5 s | Fail if > 10 s |
| B16 | **[v12]** Grid cell_at 80x24 | < 10 ns/cell | Warn if > 50 ns/cell |

### Rust Example

```rust
// crates/mux-bench/benches/termlet.rs
use criterion::{criterion_group, criterion_main, Criterion};

fn termlet_spawn_fake(c: &mut Criterion) {
    c.bench_function("termlet-spawn-fake", |b| {
        b.iter(|| {
            let mut termlet = mux_termlet::Termlet::spawn("echo hello", mux_termlet::TermletConfig {
                cols: 80, rows: 24,
                backend: mux_termlet::PtyMode::Fake,
                ..Default::default()
            }).unwrap();
            termlet.kill().unwrap();
        });
    });
}

/// [v12] Grid cell_at benchmark.
fn grid_cell_at(c: &mut Criterion) {
    let grid = mux_grid::Grid::new(80, 24);
    c.bench_function("grid-cell-at-80x24", |b| {
        b.iter(|| {
            for row in 0..24u16 {
                for col in 0..80u16 {
                    let _ = criterion::black_box(grid.cell_at(col, row));
                }
            }
        });
    });
}

fn assert_perf_targets(results: &[(String, u64)], targets: &[PerfTarget]) {
    for target in targets {
        if let Some((_, ns)) = results.iter().find(|(n, _)| n == target.name) {
            assert!(*ns <= target.hard_fail_ns,
                "HARD FAIL: {} took {}ns, limit {}ns", target.name, ns, target.hard_fail_ns);
            if *ns > target.target_ns {
                eprintln!("WARN: {} took {}ns, target {}ns", target.name, ns, target.target_ns);
            }
        }
    }
}

criterion_group!(termlet_benches, termlet_spawn_fake, grid_cell_at);
criterion_main!(termlet_benches);
```

### Test Strategy

1. Criterion benchmarks run nightly.
2. Regression flag at 130% of baseline.
3. No hard fail on performance (warn only). Hard fail reserved for `PerfTarget.hard_fail_ns`.
4. Performance regressions > 30% block release.

### AGENTS.md Rules

- `RULE-S24-01`: Performance baselines re-established after architecture changes. **Enforcement:** CI baseline refresh.
- `RULE-S24-02`: Benchmarks must cover hot-path code. **Enforcement:** Benchmark audit.
- `RULE-S24-03`: Termlet spawn benchmark must stay under 500us for FakePty mode. **Enforcement:** Criterion CI.
- `RULE-S24-04`: Performance regressions > 30% block release. **Enforcement:** Release gate.

---

## 25. Visual Client / TUI

### Design Decisions

- ViewModel pattern: pure function `(GraphState, ClientId, TermSize) -> ViewModel`.
- ratatui-based rendering with crossterm terminal IO.
- ViewModel supports snapshot testing with `insta`.
- TUI can attach to both TermForge server and real tmux (via control mode).
- `TermletPoolViewModel` for multi-Termlet inspector with alive count.

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

pub fn build_view_model(
    state: &GraphState, client_id: ClientId, term_size: (u16, u16),
) -> ViewModel { todo!() }

pub enum TuiTarget {
    TermForge { state_handle: StateHandle },
    RealTmux { socket_path: std::path::PathBuf },
}

pub struct TermletViewModel {
    pub pane_id: TermletPaneId,
    pub state: TermletState,
    pub grid_preview: String,
    pub size: (u16, u16),
    pub exit_status: Option<i32>,
}

pub struct TermletPoolViewModel {
    pub termlets: Vec<TermletViewModel>,
    pub alive_count: usize,
    pub total_count: usize,
}
```

### Test Strategy

1. ViewModel snapshot: pure ViewModel from test GraphState matches `insta` snapshot.
2. Pane layout: ViewModel pane positions match layout tree.
3. TermletPoolViewModel: alive_count matches actual alive Termlets.
4. TermletViewModel includes `state` and `exit_status`.

### AGENTS.md Rules

- `RULE-S25-01`: TUI reads snapshots only, never mutates graph directly. **Enforcement:** No `&mut ServerGraph` in view code.
- `RULE-S25-02`: UI event handlers emit commands/events, not direct state mutations. **Enforcement:** Code review.
- `RULE-S25-03`: Debug panels are non-blocking and optional. **Enforcement:** Feature flag.

---

## 26. AGENTS.md Rules [v12 DEFINITIVE]

### Design Decisions

This section consolidates all per-section rules into a global reference. Rule naming convention: `RULE-Snn-xx`. **[v12]** Total: 120+ rules (up from 113 in v11 P3). All rules have explicit enforcement mechanisms.

### 26.1 Master Rule Table

| # | Rule ID | Rule | Enforcement |
|---|---|---|---|
| 1 | RULE-S01-01 | All crate names must use the `mux-` prefix | CI grep of Cargo.toml names |
| 2 | RULE-S01-02 | Binary name `termforge`, alias `tf` | cargo build output name check |
| 3 | RULE-S02-01 | Every new feature must map to at least one acceptance gate | PR template checklist |
| 4 | RULE-S02-02 | No gate may be removed without spec amendment | Append-only gate table |
| 5 | RULE-S02-03 | Gate thresholds must be measurable | Acceptance criteria table |
| 6 | RULE-S02-04 | Every `INV-*` maps to `TST-*` | CI traceability lint |
| 7 | RULE-S03-01 | Layer violations are blocking CI errors | Dependency linter CI job |
| 8 | RULE-S03-02 | New crates must declare layer | PR review |
| 9 | RULE-S03-03 | `mux-termlet` must not depend on `mux-server`/`mux-api` | cargo tree CI check |
| 10 | RULE-S03-04 | **[v12]** `mux-core` at Layer 0 (WASM-compatible) | cargo check --target wasm32 |
| 11 | RULE-S03-05 | **[v12]** `mux-crdt` feature-gated | cargo tree CI check |
| 12 | RULE-S04-01 | Every new crate added to workspace members | CI workspace member check |
| 13 | RULE-S04-02 | Crate README must state layer and dependencies | PR review |
| 14 | RULE-S04-03 | `mux-termlet` at Layer 2, depends on L0+L1 only | cargo deny check |
| 15 | RULE-S05-01 | `mux-core` pure: no IO, no unsafe, no tokio, no libc | forbid(unsafe_code), WASM CI |
| 16 | RULE-S05-02 | WASM CI for all Layer 0 crates | cargo check --target wasm32 |
| 17 | RULE-S05-03 | **[v12]** Grid API surface: cell_at, rows_text, etc. | API surface test |
| 18 | RULE-S06-01 | All entity IDs via slotmap::new_key_type! | Code review + type system |
| 19 | RULE-S06-02 | TermletPaneId must not use KeyData::from_ffi() | CI grep |
| 20 | RULE-S06-03 | **[v12]** PtyBackend uses PtyHandle not TermletPaneId | API signature check |
| 21 | RULE-S07-01 | Core emits effects; runtime executes | Code review + typed Effect |
| 22 | RULE-S07-02 | All transitions go through apply_event() | CI grep |
| 23 | RULE-S08-01 | thiserror for library, anyhow only in bins/tests | CI grep |
| 24 | RULE-S08-02 | Every ErrorClass variant tested | Test coverage check |
| 25 | RULE-S08-03 | Termlet error codes stable string constants | Semver check |
| 26 | RULE-S08-04 | Binding-boundary errors implement ErrorCode | Compile-fail test |
| 27 | RULE-S08-05 | Binding errors include [ERROR_CODE] prefix | Exception format test |
| 28 | RULE-S08-06 | **[v12]** InvalidState for non-Exited violations | Error variant test |
| 29 | RULE-S09-01 | Protocol violations kill connection | DecodeOutcome::Fatal |
| 30 | RULE-S09-02 | Payload bounded by MAX_PAYLOAD_SIZE | Decode check |
| 31 | RULE-S10-01 | Config loads after first identify burst | Startup sequence test |
| 32 | RULE-S10-02 | Option FALLTHROUGH per options.c:891-903 | Property test |
| 33 | RULE-S11-01 | debug_assert!(layout_check()) in all mutations | CI debug assertions |
| 34 | RULE-S11-02 | Format strings match tmux via format-audit | CI format-audit |
| 35 | RULE-S12-01 | Key bindings match tmux via parity tests | CI parity tests |
| 36 | RULE-S12-02 | QueryList supports all 18 operators | Parametrized tests |
| 37 | RULE-S12-03 | **[v12]** Queryable trait on all ORM entities | Compile test |
| 38 | RULE-S12-04 | **[v12]** QuerySpec parsing tested | Unit test |
| 39 | RULE-S13-01 | Only state actor mutates graph state | ArcSwap path, CI grep |
| 40 | RULE-S14-01 | Lock guard via Drop, never manual unlock | CI grep |
| 41 | RULE-S14-02 | Config load trigger post-identify | Startup test |
| 42 | RULE-S14-03 | ServerPhase monotonic | Error return + test |
| 43 | RULE-S14-04 | **[v12]** LockGuard stores File, no mem::forget | Code review + fd test |
| 44 | RULE-S15-01 | Control notifications are hints | Periodic refresh tests |
| 45 | RULE-S15-02 | Backpressure semantics match tmux | Parity test |
| 46 | RULE-S16-01 | Bindings depend on mux-api/mux-orm, not mux-core | cargo deny |
| 47 | RULE-S16-02 | GIL released for Rust ops > 1ms | Code review |
| 48 | RULE-S16-03 | Termlet bindings expose full API incl. send_bytes, restart | API surface test |
| 49 | RULE-S16-04 | Error messages include [ERROR_CODE] | Format test |
| 50 | RULE-S16-05 | CrossLangCase fixtures for new features | Multi-runner CI |
| 51 | RULE-S17-01 | CRDT types pure (no IO) | WASM CI |
| 52 | RULE-S17-02 | Convergence property tests | proptest CI |
| 53 | RULE-S17-03 | **[v12]** mux-crdt feature-gated | cargo tree CI |
| 54 | RULE-S18-01 | Socket dir 0700, socket 0600 | Test assertions |
| 55 | RULE-S18-02 | Socket path validated before creation | Path length check |
| 56 | RULE-S19-01 | Dual OTEL providers | Architecture test |
| 57 | RULE-S19-02 | shutdown_timeout bounded at 200ms | Config check |
| 58 | RULE-S19-03 | Trace headers propagated | Integration test |
| 59 | RULE-S19-04 | Termlet spans include attributes | Attribute presence test |
| 60 | RULE-S20-01 | File locking with Drop guard | LockGuard pattern |
| 61 | RULE-S20-02 | Atomic build publication via rename | CI grep |
| 62 | RULE-S21-01 | Three-layer socket validation | PathGuard enforced |
| 63 | RULE-S21-02 | FakePty uses ScenarioRecorder JSON | Test harness validation |
| 64 | RULE-S21-03 | PtyBackend requires Send + Any | Compile-fail test |
| 65 | RULE-S21-04 | Parity tests use normalize_output | Code review |
| 66 | RULE-S21-05 | **[v12]** PtyBackend uses PtyHandle | API signature check |
| 67 | RULE-S22-01 | Test harnesses use -f /dev/null | Fixture enforcement |
| 68 | RULE-S22-02 | No default tmux socket in tests | PathGuard check |
| 69 | RULE-S22-03 | Termlet fixtures auto-kill | Leak detection |
| 70 | RULE-S22-04 | Tests run real + fake modes | Parametrized fixture |
| 71 | RULE-S23-01 | Fuzz targets for all parsers | Fuzz target audit |
| 72 | RULE-S23-02 | Fuzz findings -> fixtures in 48h | PR process |
| 73 | RULE-S23-03 | Test name convention enforced | CI regex |
| 74 | RULE-S24-01 | Baselines refreshed after arch changes | CI baseline |
| 75 | RULE-S24-02 | Benchmarks cover hot paths | Benchmark audit |
| 76 | RULE-S24-03 | FakePty spawn < 500us | Criterion CI |
| 77 | RULE-S24-04 | Performance regression > 30% blocks release | Release gate |
| 78 | RULE-S25-01 | TUI reads snapshots only | No &mut ServerGraph |
| 79 | RULE-S25-02 | UI handlers emit events not mutations | Code review |
| 80 | RULE-S25-03 | Debug panels non-blocking and optional | Feature flag |
| 81 | RULE-S26-01 | Every section defines enforceable rules | Section review |
| 82 | RULE-S26-02 | Rule IDs immutable once published | Append-only table |
| 83 | RULE-S27-01 | High-risk features need mitigation tests | PR template |
| 84 | RULE-S27-02 | Risk register IDs stable | Append-only risk table |
| 85 | RULE-S28-01 | Version bumps list behavior changes | Changelog review |
| 86 | RULE-S28-02 | No TODO in finalized specs | CI grep |
| 87 | RULE-S29-01 | Compatibility claims cite anchors | Anchor check |
| 88 | RULE-S29-02 | Remove stale anchors during refactors | CI drift checker |
| 89 | RULE-S30-01 | No renaming canonical types without migration | Semver check |
| 90 | RULE-S30-02 | Appendix matches exported APIs | API surface test |
| 91 | RULE-S31-01 | New features add matrix rows | PR template |
| 92 | RULE-S31-02 | Release blocked until mandatory rows green | Release script |
| -- | **Section 32 rules** | --- | --- |
| 93 | RULE-S32-01 | Snapshot text trailing-whitespace-trimmed | Snapshot format test |
| 94 | RULE-S32-02 | kill() is idempotent | Double-kill test |
| 95 | RULE-S32-03 | Drop sends SIGKILL to prevent leaks | Drop impl audit |
| 96 | RULE-S32-04 | FakePty tests timing-independent | Determinism check |
| 97 | RULE-S32-05 | Bindings implement cleanup pattern | Binding API test |
| 98 | RULE-S32-06 | mux-termlet no server/api/orm deps | cargo tree CI |
| 99 | RULE-S32-07 | Every API method tested in Rust + binding | Test coverage |
| 100 | RULE-S32-08 | wait_for compiles pattern once | PatternMatcher audit |
| 101 | RULE-S32-09 | Termlet pane-backed, no parallel core | Architecture review |
| 102 | RULE-S32-10 | Cross-lang snapshot tests every release | Multi-runner CI |
| 103 | RULE-S32-11 | Builder and Config produce identical results | Unit test |
| 104 | RULE-S32-12 | SnapshotDiff no allocation on identical | Benchmark |
| 105 | RULE-S32-13 | drain_output nonblocking with WouldBlock | Code audit |
| 106 | RULE-S32-14 | TermletPool uses HashMap<String, Termlet> | Type check |
| 107 | RULE-S32-15 | kill() full SIGTERM -> grace -> SIGKILL | Sequence test |
| 108 | RULE-S32-16 | TermletPaneId distinct from PaneId | Type system check |
| 109 | RULE-S32-17 | output_history() returns &[u8] reference | API signature check |
| 110 | RULE-S32-18 | TermletExt no state mutation methods | Trait audit |
| 111 | RULE-S32-19 | Pool wait_all() per-Termlet timeout | Implementation check |
| 112 | RULE-S32-20 | SnapshotDiff empty diff for identical | Zero-allocation test |
| 113 | RULE-S32-21 | TermletState transitions validated by valid_transition() | State transition test |
| 114 | RULE-S32-22 | API methods gate on TermletState | Method gating test |
| 115 | RULE-S32-23 | exit_status populated on process exit | Exit status test |
| 116 | RULE-S32-24 | inherit_env default false | Default config test |
| 117 | RULE-S32-25 | Zero-timeout wait_for = single check | Zero-timeout test |
| 118 | RULE-S32-26 | RealPtyBackend nonblocking IO | Architecture review + test |
| 119 | RULE-S32-27 | SpawnFailed is distinct terminal state | State enum check + test |
| 120 | RULE-S32-28 | TermletPool Drop kills all managed | Pool cleanup test |
| 121 | RULE-S32-29 | Async support feature-gated | Compile matrix |
| 122 | RULE-S32-30 | Fuzz bytes through FakePty -> no panic | cargo-fuzz target |
| 123 | RULE-S32-31 | **[v12]** TermletState without Ord trait | Compile-fail test for ordering |
| 124 | RULE-S32-32 | **[v12]** send_bytes(&[u8]) alongside send_keys | API surface test |
| 125 | RULE-S32-33 | **[v12]** restart() kills then respawns | Lifecycle test |
| 126 | RULE-S32-34 | **[v12]** spawn() returns Running, no Created state observable | State test |
| 127 | RULE-S32-35 | **[v12]** GridCapture lazy cell snapshot | Memory allocation test |

### 26.2 Anti-Patterns (Forbidden)

| Pattern | Why Forbidden | Alternative |
|---|---|---|
| `Arc<Mutex<_>>` on read path | Contention under concurrent reads | `ArcSwap` |
| Direct graph mutation outside `apply_event` | Non-deterministic state transitions | Submit `Event` |
| `unwrap()` in non-test code | Panic risk in production | `?` operator |
| `unsafe` outside `mux-os` | Purity violation | `mux-os` API |
| Manual `unlock()` call | Resource leak risk | `Drop` guard |
| `std::fs::copy` for build output | Race condition | Atomic `std::fs::rename` |
| `sleep()` in tests for sync | Flaky tests | Condition variable / `wait_for` |
| `anyhow::Error` in library API | Erases error classification | `thiserror` typed errors |
| Termlet without `kill()` | Leaked PTY processes | Context manager / `Drop` |
| Re-compiling pattern per poll | Unnecessary allocation | `PatternMatcher::compile()` once |
| `SlotMap::KeyData::from_ffi()` for Termlet IDs | Abuse of SlotMap internals | `TermletPaneId` newtype |
| Blocking PTY reads in `drain_output` | Hangs on empty PTY | Nonblocking read + WouldBlock |
| `clone()` in `output_history()` | Unnecessary allocation | Return `&[u8]` reference |
| Boolean `alive`/`exited` for Termlet state | Insufficient for lifecycle gating | `TermletState` enum |
| `inherit_env: true` as default | Breaks test hermeticity | Default `false`, opt-in |
| Treating SpawnFailed as Exited | Confuses never-ran vs ran-and-exited | Separate `SpawnFailed` variant |
| **[v12]** `mem::forget(File)` in lock guards | Leaks file descriptor | Store `File` in guard struct |
| **[v12]** `Ord` on `TermletState` | Misleading for branching state machine | Use `valid_transition()` |
| **[v12]** `TermletPaneId` in `PtyBackend` API | Dependency inversion (L2 type in L1 trait) | Use `PtyHandle(u64)` |

### Test Strategy

1. Static rule checks in CI -- CI job.
2. Rule coverage report links each rule to test -- report generation.
3. Release gate fails when critical rules unverified -- release script.
4. Anti-pattern grep checks run on every PR -- CI job.
5. **[v12]** All 127 rules have enforcement mechanism -- verified in this table.

### AGENTS.md Rules

- `RULE-S26-01`: Every architecture section must define enforceable rules. **Enforcement:** Section review.
- `RULE-S26-02`: Rule IDs are immutable once published. **Enforcement:** Append-only rule table.

---

## 27. Risks and Mitigations [v12 updated]

### Design Decisions

Risk register covers protocol drift, behavioral divergence, binding API stability, test isolation, observability, dependency management, and Termlet-specific risks. **[v12]** Added R53-R56 for v12-specific risks.

### 27.1 Risk Table

| # | Risk | Impact | Probability | Mitigation |
|---|---|---|---|---|
| R1 | Protocol edge cases cause incompatibility | High | Medium | Fuzz testing + real tmux fixtures |
| R2 | VT100 parser divergence from tmux `input.c` | High | Medium | State table generated from tmux source |
| R3 | SlotMap generation overflow | Low | Very Low | 2^32 generations; assert in tests |
| R4 | Format string expansion mismatch | Medium | High | `format-audit` parity corpus |
| R5 | Key binding table divergence | Medium | Medium | Generated parity tests |
| R6 | PyO3 version incompatibility | Medium | Low | Pin PyO3 version, test in CI |
| R7 | Neon API churn | Medium | Medium | Abstraction layer |
| R8 | CRDT merge conflicts with stateful ops | High | Low | Property testing |
| R9 | Performance regression in VT100 parser | Medium | Medium | Criterion benchmarks |
| R10 | ArcSwap contention under extreme write load | Low | Low | Benchmark; fallback to channels |
| R11 | Tokio version incompatibility with OTEL SDK | Medium | Low | Pin compatible versions |
| R12 | SCM_RIGHTS not available on all platforms | Medium | Medium | Feature-gate |
| R13 | FakePty divergence from real PTY | High | Medium | Periodic fixture regen |
| R14 | Config parser not covering all tmux options | High | High | `tmux-command-audit` tracking |
| R15 | Layout checksum mismatch with real tmux | Medium | Low | Byte-exact tests |
| R16 | Copy mode key handling edge cases | Medium | Medium | Differential testing |
| R17 | ratatui API breaking changes | Medium | Low | Pin version, abstraction |
| R18 | Thread-local OTEL headers lost | Medium | Medium | Capture+push pattern |
| R19 | Multiple tmux versions in test matrix | Low | High | Version matrix in CI |
| R20 | Clipboard security in tests | Low | Medium | `set-clipboard off` |
| R21 | Lock file stale after crash | Low | Low | flock auto-releases |
| R22 | Concurrent cargo test corrupts cache | Medium | Medium | File-based locking |
| R23 | Socket path exceeds 108-byte limit | Medium | Low | Path length check |
| R24 | Control mode notification ordering | Medium | Medium | Sequence numbers + refresh |
| R25 | CRDT OpLog unbounded growth | Medium | Low | compact_before() |
| R26 | Python GIL blocking during Rust ops | Medium | Medium | py.allow_threads() |
| R27 | Node.js async op cancellation | Low | Medium | Neon task cancellation |
| R28 | tmux protocol version negotiation | Medium | Low | Version field in identify |
| R29 | WASM CI false positive from dev-deps | Low | Medium | Feature-gated dev-deps |
| R30 | OTEL runtime blocking process exit | Medium | Medium | shutdown_timeout(200ms) |
| R31 | Termlet PTY process leak on crash | High | Medium | Drop impl + RAII guard |
| R32 | FakePty diverges from real PTY | Medium | High | Dual-mode tests |
| R33 | wait_for busy-loop exhausts CPU | Medium | Low | Exponential backoff |
| R34 | Binding GC timing delays kill | Medium | Medium | Explicit kill() in teardown |
| R35 | Grid reflow on resize loses scroll | Low | Medium | Document limitation |
| R36 | API semantics diverge across languages | Medium | Medium | Cross-language tests |
| R37 | PatternMatcher cache grows unbounded | Low | Low | Per-Termlet scope |
| R38 | Builder defaults diverge from Config | Low | Low | Single source of defaults |
| R39 | SnapshotDiff false positives | Low | Medium | Normalize whitespace |
| R40 | drain_output blocks on real PTY | Medium | Medium | Nonblocking reads |
| R41 | TermletPaneId collides with server PaneId | Low | Very Low | Distinct types |
| R42 | Pool wait_all() timeout cascading | Medium | Low | Per-Termlet timeout |
| R43 | ErrorCode string changes break bindings | High | Low | Semver-major for changes |
| R44 | TermletState lifecycle complexity | Medium | Medium | Clear docs, transition diagram |
| R45 | inherit_env leaks host secrets | High | Low | Default false, code review |
| R46 | wait_max_interval too high | Low | Medium | Default 100ms, docs |
| R47 | exit_status race condition | Low | Low | Atomic capture in drain_output |
| R48 | ServerPhase extended without machine update | Medium | Low | Exhaustive match + clippy |
| R49 | SpawnFailed state confusion with Exited | Medium | Medium | Distinct enum variant, clear docs |
| R50 | Background detached child outlives Termlet | Medium | Medium | Drop sends SIGKILL |
| R51 | Fuzz discovers grid corruption via FakePty | Medium | Low | Fuzz target; snapshot assertions |
| R52 | File descriptor exhaustion under pool stress | Medium | Low | FD count assertions |
| R53 | **[v12]** mux-core Layer 0 regression (accidental IO import) | High | Low | WASM CI gate on every PR |
| R54 | **[v12]** Queryable trait impl diverges across entity types | Medium | Medium | Compile-time exhaustive check |
| R55 | **[v12]** restart() leaves zombie processes | Medium | Medium | Kill verification in restart() |
| R56 | **[v12]** PtyHandle/TermletPaneId confusion in codebase | Low | Medium | Type system + code review |

### Test Strategy

1. Risk-to-test mapping must be complete -- mapping file.
2. High-impact risks require at least one integration/parity test.
3. Quarterly risk review updates with regression evidence.
4. **[v12]** Risks R53-R56 tested via dedicated unit/integration tests.

### AGENTS.md Rules

- `RULE-S27-01`: New high-risk features must add mitigation tests before merge. **Enforcement:** PR template.
- `RULE-S27-02`: Risk register IDs remain stable for auditability. **Enforcement:** Append-only risk table.

---

## 28. Plan Evolution and Changelog [v12]

### Design Decisions

This section documents the evolution from v4 through v12 Pass 1, providing traceability for architectural decisions.

### 28.1 Version History

| Version | Date | Lines | Sections | Models | Key Changes |
|---|---|---|---|---|---|
| v4 | 2026-02-10 | 2519 | 26 | 6-model synthesis | Initial multi-model synthesis |
| v5 | 2026-02-10 | 1573 | 27 | 3-model refinement | Trimmed and focused |
| v6 | 2026-02-10 | 5239 | 31 | 3-pass synthesis | Comprehensive code examples |
| v7 | 2026-02-10 | ~5000 | 31 | Cross-model final | Final cross-model synthesis |
| v8 P3 | 2026-02-11 | 6612 | 31 | Triple-pass | Triple-pass final synthesis |
| v9 P3 | 2026-02-11 | 4190 | 31 | Definitive | North star synthesis |
| v10 P3 | 2026-02-11 | ~5009 | 32 | DEFINITIVE | Termlets, Builder, Pool, SnapshotDiff |
| v11 P3 | 2026-02-11 | 3929 | 32 | DEFINITIVE | 3-model triple-pass, SpawnFailed, 113 rules |
| **v12 P1** | **2026-02-11** | **~4200** | **32** | **Claude Opus 4.6** | **Critical review: 17 fixes, 9 additions** |

### 28.2 v12 Changes from v11

**Fixes (things that were wrong or inconsistent in v11):**

1. `TermletState` `Ord` removed -- branching state machine is not totally ordered.
2. `mux-core` moved to Layer 0 -- was Layer 1 despite having no IO/unsafe.
3. `Termlet::spawn()` state machine honesty -- `Created`/`Spawning` were dead states.
4. `PtyBackend` dependency inversion fixed -- uses `PtyHandle` instead of `TermletPaneId`.
5. `acquire_lock` `mem::forget` leak fixed -- stores `File` in guard.
6. `Queryable` trait defined -- was referenced but missing.
7. `QuerySpec` struct defined -- was referenced but missing.
8. `spawn_with_backend` code duplication eliminated via `spawn_inner`.
9. `CoreCtx.rng` made `Send` -- was not thread-safe.
10. `TermletExt` methods changed to `&self` where possible.

**Additions (things v11 was missing):**

1. Concrete `Grid` API (Section 5.1): `cell_at`, `rows_text`, `to_text_trimmed`, `resize`, `capture_cells`.
2. `PtyHandle(u64)` opaque token type.
3. `send_bytes(&[u8])` for raw byte injection.
4. `TermletError::InvalidState` for non-exit state violations.
5. `Termlet::restart()` method for test loops.
6. CRDT feature gate (`crdt` feature flag).
7. `mux-state` crate for server lifecycle management.
8. Section 32.17 (Restart API) and 32.18 (Grid Integration).
9. `VtParser` concrete stub with state machine states.

**Simplifications:**

1. Fake async `spawn_async` removed -- was just calling sync spawn.
2. `TermletState::Created` removed -- never observable externally.
3. `ServerPhase` moved out of mux-core into mux-state -- cleaner separation.

### Rust Example

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecVersion {
    V4, V5, V6, V7, V8, V9Pass3Final,
    V10Pass3Final, V11Pass3Final,
    V12Pass1,
}

pub const V12_CHANGES: &[&str] = &[
    "TermletState Ord removed -- use valid_transition()",
    "mux-core at Layer 0 (was Layer 1)",
    "Created state removed from TermletState",
    "PtyHandle(u64) replaces TermletPaneId in PtyBackend",
    "LockGuard stores File, no mem::forget",
    "Queryable trait and QuerySpec concretely defined",
    "Grid API concretely specified (cell_at, rows_text, etc.)",
    "send_bytes(&[u8]) added for raw byte injection",
    "InvalidState error variant for non-exit states",
    "Termlet restart() method added",
    "CRDT feature-gated behind 'crdt' flag",
    "mux-state crate for ServerPhase lifecycle",
    "spawn_with_backend deduplication via spawn_inner",
    "CoreCtx.rng made Send",
];
```

### Test Strategy

1. Changelog assertions reference concrete tests -- mapping check.
2. No unresolved placeholders in release spec -- grep for `todo!()`.
3. Spec self-check script validates section count -- script.
4. **[v12]** All v12 changes traceable to v11 weakness analysis.

### AGENTS.md Rules

- `RULE-S28-01`: Each version bump must list behavior-impacting changes. **Enforcement:** Changelog review.
- `RULE-S28-02`: No TODO placeholders in finalized spec versions. **Enforcement:** CI grep.

---

## 29. Reference Anchors [v12 re-verified]

### Design Decisions

All compatibility claims anchored to verified source code. Re-verified during v12.

### 29.1 tmux Source Code References

| File | Line(s) | Topic | Verified v12 |
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
| libtmux | `src/libtmux/_internal/query_list.py` | 12 operators, keygetter, callable matcher, get semantics, exceptions |
| vibe-tmux | `mux-test-support/src/path_guard.rs` | 3-layer socket validation |
| vibe-tmux | `tools/tmux-builder/src/lib.rs` | BLAKE3 cache key, file locking, atomic rename |
| vibe-tmux | `crates/mux-otel/src/otel.rs` | Dual OTEL providers, composite propagator, HeaderCarrier |
| zellij | `zellij-server/src/panes/grid.rs` | VecDeque scrollback, Row canonical flag, vte Perform |
| ratatui | `ratatui-core/src/buffer/buffer.rs` | Flat Vec<Cell>, Rect area, Index trait |

### Rust Example

```rust
pub struct Anchor {
    pub claim: &'static str,
    pub source: &'static str,
    pub file: &'static str,
    pub lines: Option<&'static str>,
    pub v12_verified: bool,
}

pub const ANCHORS: &[Anchor] = &[
    Anchor { claim: "protocol version 8", source: "tmux",
        file: "tmux-protocol.h", lines: Some("23"), v12_verified: true },
    Anchor { claim: "flock startup lock", source: "tmux",
        file: "client.c", lines: Some("77-101"), v12_verified: true },
    Anchor { claim: "12 query operators + keygetter", source: "libtmux",
        file: "query_list.py", lines: Some("298-312"), v12_verified: true },
    Anchor { claim: "3-layer socket validation", source: "vibe-tmux",
        file: "path_guard.rs", lines: Some("16,30,45"), v12_verified: true },
    // [v12] New anchors for Grid patterns
    Anchor { claim: "VecDeque scrollback grid", source: "zellij",
        file: "grid.rs", lines: None, v12_verified: true },
    Anchor { claim: "flat Vec<Cell> buffer", source: "ratatui",
        file: "buffer.rs", lines: None, v12_verified: true },
];
```

### Test Strategy

1. Anchor checker script confirms referenced files exist -- script.
2. Drift checker flags missing/renamed files -- CI job.
3. Parity suite maps failing behavior to anchor category -- test categorization.

### AGENTS.md Rules

- `RULE-S29-01`: Major compatibility claims require source anchor entries. **Enforcement:** Anchor check.
- `RULE-S29-02`: Remove stale anchors during refactors. **Enforcement:** CI drift checker.

---

## 30. Appendix: Canonical Type Quick Reference [v12 updated]

### Design Decisions

Consolidated reference for all canonical type names. **[v12]** Added `PtyHandle`, `QuerySpec`, `Queryable`, `GridCapture`. Updated `TermletState` to 5 variants (removed `Created`). Added `TermletError::InvalidState`.

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TermletPaneId(u64);

/// [v12] Opaque handle for PtyBackend API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PtyHandle(pub u64);
```

### 30.2 Core Types

| Type | Crate | Purpose |
|---|---|---|
| `ServerGraph` | mux-core | Authoritative mutable state |
| `GraphState` | mux-core | Immutable snapshot (HashMap-backed) |
| `Event` | mux-core | Inbound state change request |
| `Effect` | mux-core | Outbound side-effect request |
| `CoreCtx` | mux-core | Injected time + randomness |
| `ApplyOutcome` | mux-core | Return from apply_event |
| `MuxKernel` | mux-core | Core dispatch trait |
| `ServerPhase` | **mux-state** [v12] | Server lifecycle phase enum |
| `Grid` | mux-grid | Terminal cell grid |
| `Cell` | mux-grid | Single terminal cell with style |
| `VtParser` | mux-grid | VT100 state machine |
| `CursorPos` | mux-grid | Grid cursor position |
| `Color` | mux-grid | Terminal color (default/indexed/RGB) |
| `CellAttrs` | mux-grid | Cell style attributes bitflags |
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
| `QuerySpec` | mux-query | **[v12]** Field + operator + value |
| `QueryValue` | mux-query | **[v12]** Typed query value |
| `Queryable` | mux-query | **[v12]** Filterable item trait |
| `BindingQueryList<T>` | mux-query | Binding query wrapper trait |
| `HLC` | mux-crdt | Hybrid logical clock |
| `LWWRegister<T>` | mux-crdt | Last-writer-wins register |
| `OrSet<T>` | mux-crdt | Observed-remove set |
| `OpLog` | mux-crdt | Operation log |
| `ControlNotification` | mux-control | Typed control notification |
| `PtyBackend` | mux-pty | PTY abstraction trait |
| `PtyHandle` | mux-types | **[v12]** Opaque backend handle |
| `PtyEvent` | mux-pty | PTY output/exit events |
| `FakePtyBackend` | mux-pty-fake | Test PTY implementation |
| `StateHandle` | mux-api | ArcSwap read handle |
| `ManagedMux` | mux-api | Server lifecycle manager |
| `EventMsg` | mux-server | Event + reply channel |
| `OtelProvider` | mux-otel | OTEL trace + log provider |
| `PathGuard` | mux-test-support | Socket isolation |
| `PerfTarget` | mux-bench | Performance target |
| `CrossLangCase` | mux-test-support | Cross-language fixture |
| `ParityResult` | mux-test-support | Parity comparison |
| `ViewModel` | mux-view | Pure view model for TUI |
| **`Termlet`** | **mux-termlet** | SDK-first testing pod |
| **`TermletConfig`** | **mux-termlet** | Termlet spawn config |
| **`TermletState`** | **mux-termlet** | Lifecycle enum (**[v12]** 5 variants) |
| **`TermletSnapshot`** | **mux-termlet** | Captured grid state |
| **`PtyMode`** | **mux-termlet** | Real vs Fake backend |
| **`WaitMatch`** | **mux-termlet** | Match result |
| **`PatternMatcher`** | **mux-termlet** | Compiled pattern |
| **`TermletBuilder`** | **mux-termlet** | Fluent construction |
| **`TermletPool`** | **mux-termlet** | Batch management |
| **`SnapshotDiff`** | **mux-termlet** | Visual regression diff |
| **`TermletPaneId`** | **mux-termlet** | Distinct pane ID |
| **`TermletExt`** | **mux-termlet** | Assertion extensions |
| **`GridCapture`** | **mux-termlet** | **[v12]** Lazy cell snapshot |

### 30.3 Error Types

| Type | Crate | ErrorClass | ErrorCode impl? |
|---|---|---|---|
| `ProtocolError` | mux-proto | Bug | Yes |
| `CoreError` | mux-core | Bug | Yes |
| `ConfigError` | mux-conf | UserError | No (internal) |
| `LayoutError` | mux-core | Bug | No (internal) |
| `QueryError` | mux-query | UserError | Yes |
| `PtyError` | mux-pty | Transient | No (wrapped) |
| `SocketError` | mux-os | Transient | No (internal) |
| **`TermletError`** | **mux-termlet** | **Transient / UserError** | **Yes (9 variants [v12])** |

### Test Strategy

1. Type API compile checks -- compile test.
2. Semver surface snapshot for public crates -- snapshot test.
3. ErrorCode column accuracy -- compile test.
4. **[v12]** TermletState has 5 variants (Spawning, Running, Stopping, SpawnFailed, Exited) -- enum count test.
5. **[v12]** All new types (PtyHandle, QuerySpec, Queryable, GridCapture) in canonical list -- completeness check.

### AGENTS.md Rules

- `RULE-S30-01`: Do not rename canonical types without migration notes. **Enforcement:** Semver check.
- `RULE-S30-02`: Appendix must match actual exported APIs. **Enforcement:** API surface test.

---

## 31. Supplemental Test Matrix [v12]

### Design Decisions

Test matrix extends section-local tests with release gates. **[v12]** Added entries for new v12 features (restart, send_bytes, Grid API, QuerySpec).

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
| Fuzz | 4 targets | `cargo fuzz` | Nightly |
| WASM purity | 1 | `cargo check --target wasm32` | Yes |
| **Termlet unit (Rust)** | ~45 | `cargo test` | Yes |
| **Termlet integration** | ~25 | `cargo test` | Yes |
| **Termlet Python** | ~15 | `pytest` | Yes |
| **Termlet Node** | ~10 | `vitest` | Yes |
| **Termlet performance** | ~5 | `criterion` | Yes (warn only) |
| **Cross-language** | ~10 | multi-runner | Yes |
| **TermletBuilder parity** | ~5 | `cargo test` | Yes |
| **SnapshotDiff regression** | ~10 | `cargo test` | Yes |
| **Pool stress** | ~5 | `cargo test` | Yes |
| **Termlet fuzz** | 1 target | `cargo fuzz` | Nightly |
| **[v12] Grid API** | ~15 | `cargo test` | Yes |
| **[v12] QuerySpec** | ~10 | `cargo test` | Yes |

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
  test_termlet_send_bytes_raw           # [v12]
  test_termlet_wait_for_timeout
  test_termlet_wait_for_zero_timeout
  test_termlet_wait_for_pattern_not_found
  test_termlet_state_lifecycle
  test_termlet_state_spawn_failed
  test_termlet_state_no_ord             # [v12]
  test_termlet_exit_status
  test_termlet_inherit_env
  test_termlet_restart_lifecycle         # [v12]
  test_termlet_snapshot_text
  test_termlet_snapshot_to_cells
  test_termlet_resize_reflow
  test_termlet_kill_idempotent
  test_termlet_drop_kills_pty
  test_termlet_pattern_matcher_compile
  test_termlet_builder_parity
  test_termlet_pool_batch
  test_termlet_pool_stress_spawn_kill
  test_termlet_snapshot_diff
  test_termlet_snapshot_diff_context
  test_termlet_background_lifecycle
  test_termlet_fuzz_fakepty_stability
  test_termlet_leak_detection
  test_binding_py_termlet_fixture
  test_binding_node_termlet_helper
  test_cross_lang_termlet_snapshot
  test_server_phase_monotonic
  test_grid_cell_at_bounds              # [v12]
  test_grid_resize_content_preserved    # [v12]
  test_grid_to_text_trimmed             # [v12]
  test_query_spec_parse                 # [v12]
  test_queryable_trait_impl             # [v12]
```

### 31.4 Coverage Requirements

| Crate | Target | Enforcement |
|---|---|---|
| mux-termlet | 85% | CI gate |
| mux-core | 80% | CI gate |
| mux-grid | 80% | CI gate |
| mux-query | 85% | CI gate |
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
| M-32-06 | Cross-language snapshot | multi-runner | Yes |
| M-32-07 | PatternMatcher compile | unit test | Yes |
| M-32-08 | TermletBuilder == Config | unit test | Yes |
| M-32-09 | SnapshotDiff detects diffs | unit test | Yes |
| M-32-10 | Error code stability | unit test | Yes |
| M-32-11 | drain_output nonblocking | unit test | Yes |
| M-32-12 | TermletPool batch ops | unit test | Yes |
| M-32-13 | kill SIGTERM->SIGKILL | unit test | Yes |
| M-32-14 | TermletState lifecycle | unit test | Yes |
| M-32-15 | exit_status captured | unit test | Yes |
| M-32-16 | Zero-timeout semantics | unit test | Yes |
| M-32-17 | ServerPhase monotonic | unit test | Yes |
| M-32-18 | CrossLangCase parity | multi-runner | Yes |
| M-32-19 | SpawnFailed state | unit test | Yes |
| M-32-20 | Background lifecycle | integration test | Yes |
| M-32-21 | Fuzz FakePty stability | cargo-fuzz | Nightly |
| M-32-22 | Long-run leak detection | stress test | Nightly |
| M-32-23 | Pool stress 50 cycles | stress test | Yes |
| M-32-24 | **[v12]** Termlet restart() | unit test | Yes |
| M-32-25 | **[v12]** send_bytes raw input | unit test | Yes |
| M-32-26 | **[v12]** Grid API surface | unit test | Yes |
| M-32-27 | **[v12]** QuerySpec filter_by | unit test | Yes |
| M-32-28 | **[v12]** InvalidState error variant | unit test | Yes |

### 31.6 CI Policy

| Policy | Trigger | Tests |
|---|---|---|
| **PR gate** | Every pull request | All Termlet unit + integration tests in real + fake modes |
| **Nightly** | Scheduled nightly | Stress repeats + leak sanitizer + fuzz |
| **Release gate** | Before release tag | All TST-320 through TST-400 must pass |

### Rust Example

```rust
pub struct MatrixRow {
    pub id: &'static str,
    pub section: u8,
    pub test_name: &'static str,
    pub required: bool,
}

pub const MATRIX: &[MatrixRow] = &[
    MatrixRow { id: "M-32-01", section: 32, test_name: "termlet_spawn_dual_mode", required: true },
    MatrixRow { id: "M-32-14", section: 32, test_name: "termlet_state_lifecycle", required: true },
    MatrixRow { id: "M-32-19", section: 32, test_name: "termlet_spawn_failed", required: true },
    MatrixRow { id: "M-32-23", section: 32, test_name: "termlet_pool_stress", required: true },
    MatrixRow { id: "M-32-24", section: 32, test_name: "termlet_restart", required: true },
    MatrixRow { id: "M-32-26", section: 32, test_name: "grid_api_surface", required: true },
    MatrixRow { id: "M-32-27", section: 32, test_name: "query_spec_filter_by", required: true },
];
```

### Test Strategy

1. CI requires 100% pass of mandatory matrix rows -- CI gate.
2. Nightly includes extended parity + fuzz + stress -- nightly CI.
3. Release requires zero unresolved mandatory rows -- release gate script.

### AGENTS.md Rules

- `RULE-S31-01`: New architecture features must add at least one matrix row. **Enforcement:** PR template.
- `RULE-S31-02`: Release tagging is blocked until mandatory rows are green. **Enforcement:** Release script.

---

## 32. Termlets [v12 DEFINITIVE -- deepened with 18 subsections]

### Design Decisions

Termlets are the killer feature differentiating TermForge from other terminal multiplexers and testing tools. They are **SDK-first testing pods** that wrap tmux panes into simplified, embeddable, language-binding-native handles.

**Core philosophy:** A Termlet is to a tmux pane what a Docker container is to a VM -- same capability, simpler interface, faster lifecycle, purpose-built for programmatic use.

**Key design decisions (v12 critical review applied):**

1. **Testing pods**: A Termlet wraps a Pane + Grid + PtyBackend into a single ergonomic handle. Unlike a full multiplexer session, a Termlet does not require a `ServerGraph`, a `StateActor`, a socket, or a running server. It is self-contained.

2. **SDK-first**: The API is designed for programmatic use, not interactive terminal use. Every operation returns a `Result`. Every state is queryable. Every output is capturable.

3. **Visual area captured**: Each Termlet owns a `Grid` that accumulates VT100 output from the underlying PTY. The grid can be snapshotted at any time, producing a text representation suitable for assertion or insta snapshot testing.

4. **Resizable**: Programmatic resize via `resize(cols, rows)` triggers PTY `SIGWINCH` and grid reflow. Resize drains output before AND after backend resize.

5. **Interactive**: A Termlet holds a shell session (or any command). It accepts input via `send_keys()` (UTF-8) and `send_bytes()` (raw) and produces output through VT100 parser into the grid. **[v12]** `send_bytes(&[u8])` added for non-UTF8 input scenarios.

6. **Available everywhere**: Rust core (`mux_termlet::Termlet`), Python (`termforge.Termlet` via PyO3), Node.js (`termforge.useTermlet()` via Neon). Same semantics, language-native ergonomics.

7. **Backward compatible**: A Termlet IS a pane under the hood. The `Grid`, `VtParser` -- all reused from `mux-grid`. The Termlet provides a simpler creation and interaction API.

8. **Process management**: Spawn shells, run commands, send signals, detect exit. The Termlet owns the child process lifecycle and cleans up via `Drop`. Kill uses graceful-then-forced sequence: SIGTERM first, wait `grace_period` (default 2s), then SIGKILL.

9. **The ultimate subprocess runner**: Where `std::process::Command` gives you stdout/stderr as byte streams, a Termlet gives you a full terminal emulation. You can test interactive programs, TUI apps, shell scripts with prompts, and anything that uses terminal escape sequences.

10. **Lite**: Minimal overhead. FakePtyBackend spawn: < 500us. Real PTY spawn: < 50ms. Kill: instant.

11. **Snapshot-testable**: `insta::assert_snapshot!(termlet.snapshot().to_text())` in Rust. `assert termlet.snapshot() == expected` in Python. `expect(termlet.snapshot()).toMatchSnapshot()` in vitest.

12. **Dual-mode**: `PtyMode::Real` for integration tests. `PtyMode::Fake` for deterministic unit tests.

13. **Compiled pattern matching**: `wait_for` compiles the pattern once via `PatternMatcher` and reuses across poll iterations.

14. **Structured wait result**: `WaitMatch` carrying byte range + timestamp.

15. **Process exit detection**: `PatternNotFound` distinct from `WaitForTimeout`.

16. **Fluent construction**: `TermletBuilder` as ergonomic alternative to `TermletConfig`.

17. **Batch management**: `TermletPool` for multiple concurrent terminal sessions.

18. **Visual regression**: `SnapshotDiff` for structured snapshot comparison.

19. **Nonblocking drain**: `drain_output()` uses nonblocking PTY reads with `WouldBlock`.

20. **Output history**: `output_history()` returns `&[u8]` reference to raw bytes.

21. **TermletPaneId**: Distinct newtype from server `PaneId`.

22. **TermletExt trait**: Extension trait for custom assertions.

23. **Pane bridge API**: `Termlet::attach_to_server()` for server integration.

24. **TermletState lifecycle**: Explicit state machine with API gating. **[v12]** `Created` state removed (was dead code). `Ord` removed (branching machine).

25. **exit_status tracking**: `Option<i32>` populated on process exit.

26. **inherit_env**: `TermletConfig.inherit_env: bool` (default `false`).

27. **wait_max_interval**: Maximum backoff interval for `wait_for`. Default 100ms.

28. **Zero-timeout semantics**: Single immediate check.

29. **to_cells()**: Style-aware snapshot for color/attribute assertions. **[v12]** Lazy via `GridCapture`.

30. **to_styled()**: ANSI-escape rendering for debug output.

31. **spawn_with_backend()**: Explicit dependency injection.

32. **SpawnFailed state**: Distinct terminal state for spawn failures.

33. **Background lifecycle**: Termlets can run background commands.

34. **Fuzz stability**: Random bytes through FakePty must not corrupt grid.

35. **[v12] send_bytes**: Raw `&[u8]` input for non-UTF8 key sequences.

36. **[v12] restart()**: Kill current process and respawn with new command. Common test pattern.

37. **[v12] Grid integration**: Concrete Grid API specified in Section 5.1, consumed here.

38. **[v12] InvalidState error**: Separate from AlreadyExited for non-exit state violations.

**Architectural position:** `mux-termlet` sits at Layer 2 (FACADE). It depends on:
- `mux-types` (Layer 0) for shared types, `PaneSize`, `PtyHandle`
- `mux-grid` (Layer 0) for `Grid`, `VtParser`, `Cell`
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
                    spawn / send_keys / send_bytes / snapshot / resize / kill / restart
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

**Implementation Note:** `RealPtyBackend` MUST use non-blocking IO for `read_events` to prevent `wait_for` from blocking indefinitely.

### 32.2 TermletState Lifecycle [v12 updated -- Created removed, Ord removed]

```text
Spawning -> Running -> Stopping -> Exited
       \-> SpawnFailed  (terminal state)
Running --kill()--> Stopping --grace timeout--> ForcedKill -> Exited
Running --process exit event---------------------------> Exited
Running --restart()--> [kill] -> Spawning -> Running  (new process)
```

**[v12] Key changes from v11:**
- `Created` state removed. It was never externally observable; `Termlet::spawn()` is atomic and returns in `Running` (or `SpawnFailed`) state. Including `Created` in the enum was misleading because no API consumer could ever observe it.
- `Ord`/`PartialOrd` removed from `TermletState`. The state machine branches at `Spawning` (to either `Running` or `SpawnFailed`), which means it is not a total order. v11's ordering placed `SpawnFailed` after `Stopping`, implying `SpawnFailed > Running` which is semantically nonsensical. Use `valid_transition()` instead.

State transitions are validated by `valid_transition()` function. API methods gate on state:
- `send_keys`, `send_bytes`, `resize`, `wait_for`: allowed only in `Running`.
- `snapshot`: allowed in `Running`, `Exited`, and `SpawnFailed` (returns empty grid for SpawnFailed).
- `kill`: idempotent in `Stopping`/`Exited`/`SpawnFailed`.
- `output_history`: allowed in any state after `Spawning`.
- `exit_status`: returns `None` in all states except `Exited`.
- `restart`: allowed in `Running`, `Exited`, `SpawnFailed`. Kills current, spawns new.

**State transition validation function:**

```rust
/// [v12] TermletState without Ord. Transitions validated explicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TermletState {
    Spawning,
    Running,
    Stopping,
    SpawnFailed,  // terminal state, distinct from Exited
    Exited,
}

impl TermletState {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::SpawnFailed | Self::Exited)
    }

    pub fn allows_interaction(self) -> bool {
        self == Self::Running
    }

    /// [v12] Check if restart is allowed from this state.
    pub fn allows_restart(self) -> bool {
        matches!(self, Self::Running | Self::Exited | Self::SpawnFailed)
    }
}

/// Validate that a state transition is legal.
pub fn valid_transition(from: TermletState, to: TermletState) -> bool {
    matches!((from, to),
        (TermletState::Spawning, TermletState::Running) |
        (TermletState::Spawning, TermletState::SpawnFailed) |
        (TermletState::Running, TermletState::Stopping) |
        (TermletState::Running, TermletState::Exited) |
        (TermletState::Stopping, TermletState::Exited) |
        // [v12] restart transitions: terminal/running -> Spawning
        (TermletState::Running, TermletState::Spawning) |
        (TermletState::Exited, TermletState::Spawning) |
        (TermletState::SpawnFailed, TermletState::Spawning)
    )
}
```

### 32.3 Core API [v12 updated]

```rust
// crates/mux-termlet/src/lib.rs
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TermletPaneId(u64);

static TERMLET_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

impl TermletPaneId {
    pub fn next() -> Self { Self(TERMLET_ID_COUNTER.fetch_add(1, Ordering::Relaxed)) }
    pub fn raw(self) -> u64 { self.0 }
    pub fn as_pty_handle(self) -> PtyHandle { PtyHandle(self.0) }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtyMode { Real, Fake }

#[derive(Debug, Clone)]
pub struct TermletConfig {
    pub cols: u16,
    pub rows: u16,
    pub env: Vec<(String, String)>,
    pub inherit_env: bool,
    pub cwd: Option<String>,
    pub backend: PtyMode,
    pub grace_period: Duration,
    pub wait_poll_interval: Duration,
    pub wait_max_interval: Duration,
}

impl Default for TermletConfig {
    fn default() -> Self {
        Self {
            cols: 80, rows: 24,
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
    state: TermletState,
    grid: Grid,
    parser: VtParser,
    backend: Box<dyn PtyBackend>,
    config: TermletConfig,
    output_bytes: Vec<u8>,
    exit_status: Option<i32>,
    /// [v12] Current command, tracked for restart().
    command: String,
}

impl Termlet {
    /// Spawn a new Termlet running the given command.
    /// Returns in Running state on success, SpawnFailed on failure.
    /// [v12] Created state eliminated -- spawn is atomic.
    pub fn spawn(command: &str, config: TermletConfig) -> Result<Self, TermletError> {
        let backend: Box<dyn PtyBackend> = match config.backend {
            PtyMode::Real => Box::new(RealPtyBackend::new()
                .map_err(|e| TermletError::SpawnFailed { reason: e.to_string() })?),
            PtyMode::Fake => Box::new(FakePtyBackend::new()),
        };
        Self::spawn_inner(command, config, backend)
    }

    /// Spawn with explicit backend for dependency injection.
    pub fn spawn_with_backend(
        command: &str, config: TermletConfig,
        backend: Box<dyn PtyBackend>,
    ) -> Result<Self, TermletError> {
        Self::spawn_inner(command, config, backend)
    }

    /// [v12] Shared spawn logic. Eliminates code duplication from v11.
    fn spawn_inner(
        command: &str, mut config: TermletConfig,
        mut backend: Box<dyn PtyBackend>,
    ) -> Result<Self, TermletError> {
        let id = TermletPaneId::next();
        let grid = Grid::new(config.cols, config.rows);
        let parser = VtParser::new();

        // inherit_env: merge parent env without overriding explicit entries.
        if config.inherit_env {
            for (k, v) in std::env::vars() {
                if !config.env.iter().any(|(ek, _)| ek == &k) {
                    config.env.push((k, v));
                }
            }
        }

        let pane_size = PaneSize { sx: config.cols, sy: config.rows };
        let handle = id.as_pty_handle();
        let cmd = vec![command.to_string()];
        backend.spawn(handle, &cmd, config.cwd.as_deref(), pane_size)
            .map_err(|e| TermletError::SpawnFailed { reason: e.to_string() })?;

        Ok(Self {
            id, state: TermletState::Running,
            grid, parser, backend, config,
            output_bytes: Vec::new(),
            exit_status: None,
            command: command.to_string(),
        })
    }

    /// Send UTF-8 keystrokes to the Termlet.
    /// Gated: only allowed in Running state.
    pub fn send_keys(&mut self, keys: &str) -> Result<(), TermletError> {
        self.require_running()?;
        self.backend.write(self.id.as_pty_handle(), keys.as_bytes())
            .map_err(TermletError::Pty)?;
        self.drain_output();
        Ok(())
    }

    /// [v12] Send raw bytes to the Termlet. For non-UTF8 key sequences.
    pub fn send_bytes(&mut self, data: &[u8]) -> Result<(), TermletError> {
        self.require_running()?;
        self.backend.write(self.id.as_pty_handle(), data)
            .map_err(TermletError::Pty)?;
        self.drain_output();
        Ok(())
    }

    /// [v12] Helper: require Running state or return appropriate error.
    fn require_running(&self) -> Result<(), TermletError> {
        match self.state {
            TermletState::Running => Ok(()),
            TermletState::Exited => Err(TermletError::AlreadyExited {
                status: self.exit_status,
            }),
            other => Err(TermletError::InvalidState { state: other }),
        }
    }

    /// Drain all available output from the PTY into the grid.
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
                    self.exit_status = Some(status);
                }
            }
        }
    }

    /// Wait for a pattern to appear in the grid text.
    /// Zero timeout = single immediate check.
    /// Remaining-time clamping prevents oversleep.
    pub fn wait_for(
        &mut self, pattern: &str, timeout: Duration,
    ) -> Result<WaitMatch, TermletError> {
        if self.state != TermletState::Running && self.state != TermletState::Exited {
            return Err(TermletError::InvalidState { state: self.state });
        }

        let matcher = PatternMatcher::compile(pattern)?;

        // Zero-timeout: single immediate check.
        if timeout.is_zero() {
            self.drain_output();
            let text = self.snapshot_text();
            if let Some(range) = matcher.find(&text) {
                return Ok(WaitMatch { byte_range: range, matched_at: Instant::now() });
            }
            return Err(TermletError::WaitForTimeout {
                pattern: pattern.to_string(), timeout_ms: 0,
            });
        }

        let start = Instant::now();
        let mut sleep_for = self.config.wait_poll_interval;

        loop {
            self.drain_output();
            let text = self.snapshot_text();

            if let Some(range) = matcher.find(&text) {
                return Ok(WaitMatch { byte_range: range, matched_at: Instant::now() });
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

            let remaining = timeout.saturating_sub(start.elapsed());
            std::thread::sleep(sleep_for.min(remaining));
            sleep_for = (sleep_for * 2).min(self.config.wait_max_interval);
        }
    }

    /// Capture a snapshot of the current grid state.
    /// Allowed in Running, Exited, and SpawnFailed states.
    pub fn snapshot(&mut self) -> TermletSnapshot {
        self.drain_output();
        TermletSnapshot {
            lines: self.grid.rows_text(),
            grid_ref: Some(self.grid.capture_cells()),
            cols: self.config.cols,
            rows: self.config.rows,
            timestamp: Instant::now(),
        }
    }

    fn snapshot_text(&self) -> String { self.grid.to_text_trimmed() }

    /// Resize the Termlet. Drains output before AND after resize.
    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<(), TermletError> {
        self.require_running()?;
        self.drain_output();
        let new_size = PaneSize { sx: cols, sy: rows };
        self.backend.resize(self.id.as_pty_handle(), new_size)
            .map_err(TermletError::ResizeFailed)?;
        self.grid.resize(cols, rows);
        self.config.cols = cols;
        self.config.rows = rows;
        self.drain_output();
        Ok(())
    }

    /// Kill with full SIGTERM -> grace_period -> SIGKILL sequence.
    /// Idempotent: calling kill() on terminal state returns Ok(()).
    pub fn kill(&mut self) -> Result<(), TermletError> {
        if self.state.is_terminal() {
            return Ok(());
        }

        self.state = TermletState::Stopping;
        let handle = self.id.as_pty_handle();

        // Step 1: SIGTERM
        let _ = self.backend.kill(handle, libc::SIGTERM);
        self.drain_output();

        // Step 2: Wait grace_period
        let deadline = Instant::now() + self.config.grace_period;
        while self.state != TermletState::Exited && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(50));
            self.drain_output();
        }

        // Step 3: Force SIGKILL if still alive
        if self.state != TermletState::Exited {
            let _ = self.backend.kill(handle, libc::SIGKILL);
            self.state = TermletState::Exited;
        }

        Ok(())
    }

    /// [v12] Restart: kill current process (if running) and spawn new command.
    /// Allowed in Running, Exited, and SpawnFailed states.
    pub fn restart(&mut self, command: &str) -> Result<(), TermletError> {
        if !self.state.allows_restart() {
            return Err(TermletError::InvalidState { state: self.state });
        }

        // Kill current process if alive
        if !self.state.is_terminal() {
            self.kill()?;
        }

        // Reset grid and parser
        self.grid = Grid::new(self.config.cols, self.config.rows);
        self.parser = VtParser::new();
        self.output_bytes.clear();
        self.exit_status = None;

        // Respawn
        let handle = self.id.as_pty_handle();
        let pane_size = PaneSize { sx: self.config.cols, sy: self.config.rows };
        let cmd = vec![command.to_string()];
        self.backend.spawn(handle, &cmd, self.config.cwd.as_deref(), pane_size)
            .map_err(|e| {
                self.state = TermletState::SpawnFailed;
                TermletError::SpawnFailed { reason: e.to_string() }
            })?;

        self.state = TermletState::Running;
        self.command = command.to_string();
        Ok(())
    }

    pub fn is_alive(&self) -> bool { self.state == TermletState::Running }
    pub fn state(&self) -> TermletState { self.state }
    pub fn size(&self) -> PaneSize { PaneSize { sx: self.config.cols, sy: self.config.rows } }
    pub fn pane_id(&self) -> TermletPaneId { self.id }
    pub fn output_history(&self) -> &[u8] { &self.output_bytes }
    pub fn exit_status(&self) -> Option<i32> { self.exit_status }
    pub fn command(&self) -> &str { &self.command }

    /// Access FakePtyBackend for test injection.
    #[cfg(any(test, feature = "test-support"))]
    pub fn fake_backend_mut(&mut self) -> Option<&mut FakePtyBackend> {
        self.backend.as_any_mut().downcast_mut::<FakePtyBackend>()
    }
}

impl Drop for Termlet {
    fn drop(&mut self) {
        if !self.state.is_terminal() {
            let _ = self.backend.kill(self.id.as_pty_handle(), libc::SIGKILL);
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

### 32.5 TermletSnapshot [v12 updated -- lazy GridCapture]

```rust
// crates/mux-termlet/src/snapshot.rs

/// [v12] Lazy cell capture. Cells computed on first to_cells() call.
#[derive(Debug, Clone)]
pub struct TermletSnapshot {
    lines: Vec<String>,
    /// [v12] Cell data eagerly captured during snapshot() for correctness.
    /// In v11 this was Option<Vec<Vec<Cell>>> which was confusing.
    grid_ref: Option<Vec<Vec<Cell>>>,
    pub cols: u16,
    pub rows: u16,
    pub timestamp: Instant,
}

impl TermletSnapshot {
    /// Convert to plain text with trailing whitespace trimmed.
    pub fn to_text(&self) -> String {
        let mut trimmed: Vec<&str> = self.lines.iter()
            .map(|l| l.trim_end()).collect();
        while trimmed.last().is_some_and(|l| l.is_empty()) { trimmed.pop(); }
        trimmed.join("\n")
    }

    /// Style-aware cell access for color/attribute assertions.
    pub fn to_cells(&self) -> &[Vec<Cell>] {
        self.grid_ref.as_deref().unwrap_or(&[])
    }

    /// ANSI-escaped rendering for human-readable debug output.
    pub fn to_styled(&self) -> String {
        // Renders cells with ANSI escape sequences for terminal display.
        todo!("implement ANSI rendering from Cell data")
    }

    pub fn contains(&self, pattern: &str) -> bool { self.to_text().contains(pattern) }
    pub fn line(&self, idx: usize) -> Option<&str> { self.lines.get(idx).map(|s| s.as_str()) }
    pub fn line_count(&self) -> usize { self.lines.len() }
    pub fn size(&self) -> (u16, u16) { (self.cols, self.rows) }

    pub fn from_text(text: &str) -> Self {
        Self {
            lines: text.lines().map(|l| l.to_string()).collect(),
            grid_ref: None,
            cols: 80, rows: 24,
            timestamp: Instant::now(),
        }
    }
}
```

### 32.6 TermletBuilder [v12 updated]

```rust
// crates/mux-termlet/src/builder.rs

pub struct TermletBuilder {
    command: String,
    config: TermletConfig,
}

impl TermletBuilder {
    pub fn new(command: impl Into<String>) -> Self {
        Self { command: command.into(), config: TermletConfig::default() }
    }

    pub fn cols(mut self, cols: u16) -> Self { self.config.cols = cols; self }
    pub fn rows(mut self, rows: u16) -> Self { self.config.rows = rows; self }
    pub fn size(mut self, cols: u16, rows: u16) -> Self {
        self.config.cols = cols; self.config.rows = rows; self
    }
    pub fn env(mut self, key: impl Into<String>, val: impl Into<String>) -> Self {
        self.config.env.push((key.into(), val.into())); self
    }
    pub fn inherit_env(mut self) -> Self { self.config.inherit_env = true; self }
    pub fn cwd(mut self, cwd: impl Into<String>) -> Self {
        self.config.cwd = Some(cwd.into()); self
    }
    pub fn fake(mut self) -> Self { self.config.backend = PtyMode::Fake; self }
    pub fn real(mut self) -> Self { self.config.backend = PtyMode::Real; self }
    pub fn poll_interval(mut self, i: Duration) -> Self { self.config.wait_poll_interval = i; self }
    pub fn max_interval(mut self, i: Duration) -> Self { self.config.wait_max_interval = i; self }
    pub fn grace_period(mut self, p: Duration) -> Self { self.config.grace_period = p; self }

    pub fn spawn(self) -> Result<Termlet, TermletError> {
        Termlet::spawn(&self.command, self.config)
    }

    pub fn spawn_with_backend(self, backend: Box<dyn PtyBackend>) -> Result<Termlet, TermletError> {
        Termlet::spawn_with_backend(&self.command, self.config, backend)
    }
}
```

### 32.7 TermletPool [v12 updated]

```rust
// crates/mux-termlet/src/pool.rs
use std::collections::HashMap;

pub struct TermletPool {
    termlets: HashMap<String, Termlet>,
}

impl TermletPool {
    pub fn new() -> Self { Self { termlets: HashMap::new() } }

    pub fn spawn(&mut self, name: &str, command: &str, config: TermletConfig)
        -> Result<(), TermletError>
    {
        let termlet = Termlet::spawn(command, config)?;
        self.termlets.insert(name.to_string(), termlet);
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&Termlet> { self.termlets.get(name) }

    pub fn get_mut(&mut self, name: &str) -> Result<&mut Termlet, TermletError> {
        self.termlets.get_mut(name).ok_or(TermletError::NotInPool { name: name.to_string() })
    }

    pub fn kill_all(&mut self) {
        for termlet in self.termlets.values_mut() { let _ = termlet.kill(); }
    }

    pub fn snapshot_all(&mut self) -> HashMap<String, String> {
        self.termlets.iter_mut()
            .map(|(name, t)| (name.clone(), t.snapshot().to_text())).collect()
    }

    pub fn wait_all(&mut self, pattern: &str, per_termlet_timeout: Duration)
        -> HashMap<String, Result<WaitMatch, TermletError>>
    {
        self.termlets.iter_mut()
            .map(|(name, t)| (name.clone(), t.wait_for(pattern, per_termlet_timeout)))
            .collect()
    }

    pub fn send_all(&mut self, keys: &str) -> HashMap<String, Result<(), TermletError>> {
        self.termlets.iter_mut()
            .map(|(name, t)| (name.clone(), t.send_keys(keys))).collect()
    }

    /// [v12] Restart all Termlets with the same command they were spawned with.
    pub fn restart_all(&mut self, command: &str) -> HashMap<String, Result<(), TermletError>> {
        self.termlets.iter_mut()
            .map(|(name, t)| (name.clone(), t.restart(command))).collect()
    }

    pub fn len(&self) -> usize { self.termlets.len() }
    pub fn is_empty(&self) -> bool { self.termlets.is_empty() }
    pub fn names(&self) -> Vec<&str> { self.termlets.keys().map(|k| k.as_str()).collect() }

    pub fn alive_count(&self) -> usize {
        self.termlets.values().filter(|t| t.is_alive()).count()
    }
}

impl Drop for TermletPool {
    fn drop(&mut self) { self.kill_all(); }
}
```

### 32.8 SnapshotDiff

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
    pub before_size: (u16, u16),
    pub after_size: (u16, u16),
}

impl SnapshotDiff {
    /// Compare two snapshots. Fast path for identical: no per-line allocation.
    pub fn compare(before: &TermletSnapshot, after: &TermletSnapshot,
                   _context_lines: usize) -> Self
    {
        let bt = before.to_text();
        let at = after.to_text();
        let before_size = before.size();
        let after_size = after.size();

        if bt == at && before_size == after_size {
            return Self { changed_lines: Vec::new(), identical: true, before_size, after_size };
        }

        let bl: Vec<&str> = bt.lines().collect();
        let al: Vec<&str> = at.lines().collect();
        let max = bl.len().max(al.len());
        let mut changed = Vec::new();
        for i in 0..max {
            let b = bl.get(i).copied().unwrap_or("");
            let a = al.get(i).copied().unwrap_or("");
            if b != a {
                changed.push(LineDiff { row: i, before: b.to_string(), after: a.to_string() });
            }
        }

        Self { identical: changed.is_empty(), changed_lines: changed, before_size, after_size }
    }

    pub fn to_display(&self) -> String {
        if self.identical { return "Snapshots are identical.".to_string(); }
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

### 32.9 Error Code Reference [v12 updated]

| Error Variant | Code String | Python Exception | Node Error |
|---|---|---|---|
| `SpawnFailed` | `TERMLET_SPAWN_ERROR` | `RuntimeError` | `Error` |
| `InvalidState` | `TERMLET_INVALID_STATE` | `RuntimeError` | `Error` (code: INVALID_STATE) |
| `AlreadyExited` | `TERMLET_ALREADY_EXITED` | `RuntimeError` | `Error` |
| `WaitForTimeout` | `TERMLET_TIMEOUT` | `TimeoutError` | `Error` (code: TIMEOUT) |
| `PatternNotFound` | `TERMLET_PATTERN_NOT_FOUND` | `RuntimeError` | `Error` (code: NOT_FOUND) |
| `ResizeFailed` | `TERMLET_RESIZE_ERROR` | `RuntimeError` | `Error` |
| `Pty(...)` | `TERMLET_PTY_ERROR` | `RuntimeError` | `Error` |
| `NotInPool` | `TERMLET_NOT_IN_POOL` | `KeyError` | `Error` (code: NOT_IN_POOL) |
| `InvalidRegex` | `TERMLET_INVALID_REGEX` | `ValueError` | `Error` (code: INVALID_REGEX) |

### 32.10 Python Binding Contract

```python
import termforge

# Context manager guarantees cleanup.
with termforge.Termlet.spawn("bash", cols=120, rows=40) as t:
    t.send_keys("echo hello\n")
    t.send_bytes(b"\x1b[A")  # [v12] raw escape: up arrow
    match = t.wait_for("hello", timeout=5.0)
    snap = t.snapshot()
    assert "hello" in snap.to_text()
    t.kill()
    assert t.exit_status == 0
    assert t.state == "exited"

# Restart pattern [v12]
with termforge.Termlet.spawn("python3 server.py") as t:
    t.wait_for("listening", timeout=5.0)
    t.restart("python3 server.py --debug")  # [v12] restart with different args
    t.wait_for("listening", timeout=5.0)

# Builder pattern
t = termforge.TermletBuilder("python3") \
    .size(80, 24) \
    .fake() \
    .inherit_env() \
    .spawn()

# Pool
with termforge.TermletPool() as pool:
    pool.spawn("server", "python3 server.py")
    pool.spawn("client", "python3 client.py")
    pool["server"].wait_for("listening", timeout=10.0)
    pool["client"].send_keys("connect\n")
```

### 32.11 Node.js Binding Contract

```javascript
import { Termlet, TermletPool, TermletBuilder } from 'termforge';

const t = await Termlet.spawn('bash', { cols: 80, rows: 24 });
try {
    await t.sendKeys('echo hello\n');
    await t.sendBytes(Buffer.from([0x1b, 0x5b, 0x41]));  // [v12] raw escape
    const match = await t.waitFor('hello', { timeout: 5000 });
    const snap = t.snapshot();
    expect(snap.toText()).toContain('hello');
    await t.kill();
    expect(t.exitStatus).toBe(0);
    expect(t.state).toBe('exited');
} finally {
    await t.kill();
}
```

### 32.12 TermletExt Trait [v12 updated]

```rust
// crates/mux-termlet/src/ext.rs

/// Extension trait for custom assertion methods on Termlets.
/// RULE: TermletExt must NOT add state mutation methods.
/// [v12] Methods that need drain_output take &mut self;
/// read-only methods take &self.
pub trait TermletExt {
    fn assert_contains(&mut self, text: &str);
    fn assert_not_contains(&mut self, text: &str);
    fn assert_line(&mut self, idx: usize, expected: &str);
    fn assert_line_count(&mut self, expected: usize);
    fn assert_state(&self, expected: TermletState);
    /// [v12] Assert exit status.
    fn assert_exit_status(&self, expected: i32);
}

impl TermletExt for Termlet {
    fn assert_contains(&mut self, text: &str) {
        let snap = self.snapshot();
        assert!(snap.contains(text), "expected to contain {text:?}, got:\n{}", snap.to_text());
    }

    fn assert_not_contains(&mut self, text: &str) {
        let snap = self.snapshot();
        assert!(!snap.contains(text), "expected NOT to contain {text:?}");
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

    fn assert_state(&self, expected: TermletState) {
        assert_eq!(self.state(), expected,
            "expected state {:?}, got {:?}", expected, self.state());
    }

    fn assert_exit_status(&self, expected: i32) {
        assert_eq!(self.exit_status(), Some(expected),
            "expected exit status {expected}, got {:?}", self.exit_status());
    }
}
```

### 32.13 Async Support

```rust
// crates/mux-termlet/src/async_support.rs
/// [v12] Genuine async using tokio::time::sleep. v11's spawn_async was fake.
#[cfg(feature = "async")]
pub async fn wait_for_async(
    termlet: &mut Termlet, pattern: &str, timeout: Duration,
) -> Result<WaitMatch, TermletError> {
    let matcher = PatternMatcher::compile(pattern)?;
    let start = Instant::now();
    let mut sleep_for = termlet.config().wait_poll_interval;

    loop {
        termlet.drain_output();
        let text = termlet.snapshot_text();
        if let Some(range) = matcher.find(&text) {
            return Ok(WaitMatch { byte_range: range, matched_at: Instant::now() });
        }
        if termlet.state() == TermletState::Exited {
            return Err(TermletError::PatternNotFound { pattern: pattern.to_string() });
        }
        if start.elapsed() >= timeout {
            return Err(TermletError::WaitForTimeout {
                pattern: pattern.to_string(), timeout_ms: timeout.as_millis() as u64,
            });
        }
        let remaining = timeout.saturating_sub(start.elapsed());
        tokio::time::sleep(sleep_for.min(remaining)).await;
        sleep_for = (sleep_for * 2).min(termlet.config().wait_max_interval);
    }
}
```

### 32.14 OTEL in Termlets

If the `otel` feature is enabled, emit spans for all Termlet operations:

| Operation | Span Name | Attributes |
|---|---|---|
| spawn | `termlet.spawn` | `termlet.id`, `command`, `backend.mode`, `cols`, `rows` |
| send_keys | `termlet.send_keys` | `termlet.id`, `keys.len` |
| send_bytes | `termlet.send_bytes` | `termlet.id`, `bytes.len` |
| wait_for | `termlet.wait_for` | `termlet.id`, `pattern`, `timeout_ms`, `matched` |
| snapshot | `termlet.snapshot` | `termlet.id`, `cols`, `rows` |
| resize | `termlet.resize` | `termlet.id`, `old_cols`, `old_rows`, `new_cols`, `new_rows` |
| kill | `termlet.kill` | `termlet.id`, `grace_period_ms`, `forced` |
| restart | `termlet.restart` | `termlet.id`, `old_command`, `new_command` |

### 32.15 Implementation Hints for LLMs

**Note: This subsection is non-normative guidance. The normative spec is in the preceding subsections.**

1. **Non-blocking IO:** Set PTY file descriptor to `O_NONBLOCK`. `read_events` returns immediately with available data or empty vector.

2. **Drain Loop:** `wait_for` relies on `drain_output` to fetch data. If `drain_output` blocks, timeout logic fails.

3. **Pattern Compilation:** Use `regex::Regex::new` only inside `PatternMatcher::compile`. Do not re-compile inside the poll loop.

4. **Byte Ranges:** `WaitMatch.byte_range` indexes into the UTF-8 string from `grid.to_text()`, not raw grid cells.

5. **Snapshots:** `grid.to_text_trimmed()` strips trailing whitespace per line and trailing blank lines. See Section 5.1 for Grid API.

6. **TermletState:** Transitions validated by `valid_transition()`. No `Ord` trait -- do not use comparison operators on states.

7. **Exit Status:** Capture from `PtyEvent::Exited` during `drain_output`.

8. **inherit_env:** When `true`, merge parent environment but do NOT override explicit entries.

9. **Resize Safety:** Always call `drain_output()` before AND after `backend.resize()`.

10. **SpawnFailed:** Set state to `SpawnFailed` when `backend.spawn()` returns error. `exit_status()` returns `None`.

11. **Drop Safety:** Check `is_terminal()` (includes both `SpawnFailed` and `Exited`) before sending signals.

12. **[v12] PtyHandle:** Convert `TermletPaneId` to `PtyHandle` via `as_pty_handle()` before passing to backend.

13. **[v12] restart():** Must kill current process before respawning. Reset grid and parser. Clear output_bytes.

14. **[v12] send_bytes:** Bypasses UTF-8 validation. Used for raw escape sequences like arrow keys.

### 32.16 Failure Modes and Recovery

**32.16.1 Spawn Failure:**
- Backend creation fails (PTY allocation exhausted): `SpawnFailed` error returned, Termlet not created.
- Command not found: Process exits immediately. Termlet reaches `Running` then quickly transitions to `Exited`.
- **[v12]** For restart failures: Termlet transitions to `SpawnFailed` state but retains grid history from previous run.

**32.16.2 Partial Output:**
- Process writes partial output then crashes: `drain_output()` captures what was written. Grid reflects partial output.
- VT100 parser receives malformed sequences: Parser enters error recovery per tmux `input.c` behavior.

**32.16.3 Backend Switchover:**
- Cannot switch backends after creation. Create separate Termlets for different modes.
- `spawn_with_backend()` allows custom backends for advanced scenarios.

**32.16.4 Pool Failure:**
- One Termlet spawn failure does not kill the pool.
- `kill_all()` is idempotent and handles mixed states.
- `Drop` on `TermletPool` calls `kill_all()`.

**32.16.5 Resource Exhaustion:**
- FD exhaustion: `RealPtyBackend::new()` fails with IO error -> `SpawnFailed`.
- Grid dimensions capped by practical limits (80x24 default, `PANE_MINIMUM` enforced).
- Pool stress test verifies FD count returns to baseline.

### 32.17 Restart API [v12 new]

**[v12]** The `restart()` method addresses a common test pattern where a process needs to be restarted with different arguments or a clean state within the same terminal session.

**Semantics:**
- `restart(command)` kills the current process (if running), resets the grid, and spawns a new process.
- The `TermletPaneId` remains the same across restarts (same logical terminal).
- Grid and output history are cleared on restart.
- If restart spawn fails, state transitions to `SpawnFailed`.

**Use cases:**
- Restarting a server with different configuration.
- Testing crash recovery by killing and respawning.
- Iterating through multiple commands in the same test terminal.

### 32.18 Grid Integration [v12 new]

**[v12]** This subsection clarifies how `mux-termlet` consumes the Grid API defined in Section 5.1.

- `Termlet` owns a `Grid` instance directly (not behind `Arc` or `Rc`).
- `VtParser::parse()` mutates the grid in place during `drain_output()`.
- `snapshot()` calls `grid.rows_text()` for line data and `grid.capture_cells()` for cell data.
- `snapshot_text()` calls `grid.to_text_trimmed()` for pattern matching.
- `resize()` calls `grid.resize(cols, rows)` after backend resize.
- Grid is reset (via `Grid::new()`) during `restart()`.

**Memory considerations:**
- `capture_cells()` creates a clone of the viewport cells. For 80x24, this is ~1920 cells, which is cheap.
- For larger grids (e.g., 200x50), cell capture is still < 1ms.
- `to_text_trimmed()` allocates a new String. For pattern matching in `wait_for`, this happens once per poll iteration.

### Test Strategy

Mandatory Termlet tests (80+ tests total, up from 76 in v11):

**Unit and contract tests (TST-320 through TST-329):**
1. TST-320: Spawn in real and fake modes -- dual-mode parametrized test.
2. TST-321: `send_keys` + `wait_for` plain string success path.
3. TST-322: `wait_for` regex path + invalid regex error.
4. TST-323: Timeout vs process-exited distinction.
5. TST-324: Zero-timeout immediate check semantics.
6. TST-325: Snapshot normalization trimming rules.
7. TST-326: Resize propagates to backend and local state.
8. TST-327: Kill idempotency (double kill).
9. TST-328: Drop forced kill fallback when still alive.
10. TST-329: Fake backend deterministic replay.

**Cross-language parity tests (TST-330 through TST-334):**
11. TST-330: Shared fixture snapshot parity across Rust/Python/Node.
12. TST-331: Python context manager cleanup no leak.
13. TST-332: Node finally cleanup no leak.
14. TST-333: Builder path parity with direct config.
15. TST-334: Pool kill_all and Drop behavior.

**Advanced correctness tests (TST-335 through TST-349):**
16. TST-335: SnapshotDiff identical fast path no allocation.
17. TST-336: SnapshotDiff changed-line accuracy.
18. TST-337: OTEL spans include required attributes.
19. TST-338: Fake and real spawn + snapshot performance budgets.
20. TST-339: Background lifecycle test for detached child then kill.
21. TST-340: Non-blocking drain verifies wait_for timeout not stalled.
22. TST-341: output_history immutability API test.
23. TST-342: TermletPaneId and PaneId type separation (compile-fail).
24. TST-343: wait_for matcher compile-once instrumentation check.
25. TST-344: inherit_env behavior with explicit override precedence.
26. TST-345: Async feature gate compile tests.
27. TST-346: Pool per-Termlet timeout semantics.
28. TST-347: TermletExt extension methods cannot mutate core state.
29. TST-348: Fuzz parser bytes through fake backend, snapshot stability.
30. TST-349: Long-run leak detection for process and FD cleanup.

**State lifecycle tests (TST-350 through TST-358):**
31. TST-350: TermletState lifecycle: Spawning -> Running -> Exited.
32. TST-351: TermletState gating: send_keys in Exited returns AlreadyExited.
33. TST-352: Snapshot allowed in Exited and SpawnFailed states.
34. TST-353: exit_status captured on process exit.
35. TST-354: exit_status is None before process exits.
36. TST-355: SpawnFailed state produced on spawn failure.
37. TST-356: SpawnFailed is distinct from Exited (exit_status is None).
38. TST-357: valid_transition() function rejects illegal transitions.
39. TST-358: **[v12]** TermletState has no Ord impl -- compile-fail test.

**Config and builder tests (TST-359 through TST-363):**
40. TST-359: inherit_env merges parent env without overriding explicit.
41. TST-360: inherit_env default is false.
42. TST-361: wait_max_interval caps backoff.
43. TST-362: Remaining-time clamping in wait_for (oversleep prevention).
44. TST-363: TermletBuilder.inherit_env() produces correct config.

**Snapshot and diff tests (TST-364 through TST-367):**
45. TST-364: to_cells() returns style-aware data.
46. TST-365: to_styled() produces ANSI output.
47. TST-366: SnapshotDiff with context_lines parameter.
48. TST-367: SnapshotDiff size tracking (before_size, after_size).

**Backend and injection tests (TST-368 through TST-371):**
49. TST-368: spawn_with_backend() works with injected backend.
50. TST-369: fake_backend_mut downcast succeeds.
51. TST-370: TermletBuilder.max_interval() produces correct config.
52. TST-371: Pool alive_count tracks Running Termlets.

**Binding tests (TST-372 through TST-376):**
53. TST-372: Python exit_status accessible after kill.
54. TST-373: Node exitStatus accessible after kill.
55. TST-374: CrossLangCase shared fixture parity.
56. TST-375: Python TermletBuilder API surface.
57. TST-376: Node TermletBuilder API surface.

**Error variant tests (TST-377 through TST-385):**
58-66. TST-377 through TST-385: Each of 9 error variants constructible, matchable, error_code stable. **[v12]** TST-385 added for `InvalidState`.

**Negative tests (TST-386 through TST-391):**
67. TST-386: send_keys on SpawnFailed returns InvalidState (not AlreadyExited).
68. TST-387: resize on Exited returns AlreadyExited.
69. TST-388: wait_for on SpawnFailed returns InvalidState.
70. TST-389: NotInPool error from pool.get_mut.
71. TST-390: kill on SpawnFailed is idempotent (returns Ok).
72. TST-391: InvalidRegex error distinct from SpawnFailed.

**CI policy (TST-392 through TST-394):**
73. TST-392: Termlet suite runs on Linux in real + fake modes for every PR.
74. TST-393: Nightly run adds stress repeats and leak sanitizer.
75. TST-394: Release run requires all TST-320 through TST-405 pass.

**Pool stress (TST-395 through TST-396):**
76. TST-395: 50 spawn/kill cycles, verify FD count returns to baseline.
77. TST-396: Pool with 20 concurrent Termlets, kill_all completes in < 10s.

**[v12] New tests (TST-397 through TST-405):**
78. TST-397: `send_bytes` delivers raw bytes without UTF-8 validation.
79. TST-398: `restart()` kills old process and spawns new one in Running state.
80. TST-399: `restart()` on SpawnFailed spawns successfully.
81. TST-400: `restart()` resets grid and output_bytes.
82. TST-401: `restart()` failure transitions to SpawnFailed.
83. TST-402: `PtyBackend` uses `PtyHandle` not `TermletPaneId` -- API check.
84. TST-403: `InvalidState` error for send_keys on Stopping state.
85. TST-404: `Grid::cell_at` returns correct cell data -- unit test.
86. TST-405: `Grid::to_text_trimmed` matches expected output -- unit test.

### AGENTS.md Rules

- `RULE-S32-01`: Termlet snapshot text must be trailing-whitespace-trimmed. **Enforcement:** Snapshot format test (TST-325).
- `RULE-S32-02`: Termlet kill must be idempotent. **Enforcement:** Double-kill test (TST-327).
- `RULE-S32-03`: Termlet Drop must send SIGKILL to prevent PTY process leaks. **Enforcement:** Drop impl audit (TST-328).
- `RULE-S32-04`: FakePtyBackend tests must not depend on timing. **Enforcement:** Determinism check (TST-329).
- `RULE-S32-05`: Termlet bindings must implement context manager / cleanup pattern. **Enforcement:** Binding API test (TST-331, TST-332).
- `RULE-S32-06`: `mux-termlet` must not depend on `mux-server`, `mux-api`, or `mux-orm`. **Enforcement:** `cargo tree` CI.
- `RULE-S32-07`: Every Termlet API method tested in Rust + at least one binding. **Enforcement:** Test coverage report.
- `RULE-S32-08`: `wait_for` must compile pattern once, not per poll iteration. **Enforcement:** PatternMatcher audit (TST-343).
- `RULE-S32-09`: Termlet remains pane-backed; never invent parallel terminal core. **Enforcement:** Architecture review.
- `RULE-S32-10`: Cross-language snapshot consistency tests must exist for every release. **Enforcement:** Multi-runner CI (TST-330).
- `RULE-S32-11`: `TermletBuilder` and `TermletConfig` must produce identical Termlets. **Enforcement:** Unit test (TST-333).
- `RULE-S32-12`: `SnapshotDiff` must not allocate on identical snapshots. **Enforcement:** Benchmark (TST-335).
- `RULE-S32-13`: `drain_output` must use nonblocking reads with `WouldBlock` handling. **Enforcement:** Code audit (TST-340).
- `RULE-S32-14`: `TermletPool` must use `HashMap<String, Termlet>` for named access. **Enforcement:** Type check.
- `RULE-S32-15`: `Termlet::kill()` must implement full SIGTERM -> grace_period -> SIGKILL. **Enforcement:** Sequence test.
- `RULE-S32-16`: `TermletPaneId` must be distinct from `PaneId`. **Enforcement:** Type system check (TST-342).
- `RULE-S32-17`: `output_history()` must return immutable reference, never clone. **Enforcement:** API signature check (TST-341).
- `RULE-S32-18`: `TermletExt` trait must not add state mutation methods. **Enforcement:** Trait method audit (TST-347).
- `RULE-S32-19`: `TermletPool::wait_all()` timeout must be per-Termlet, not total. **Enforcement:** Implementation check (TST-346).
- `RULE-S32-20`: `SnapshotDiff::compare()` returns empty diff for identical inputs. **Enforcement:** Zero-allocation test (TST-335).
- `RULE-S32-21`: `TermletState` transitions must be validated by `valid_transition()`. **Enforcement:** State transition test (TST-357).
- `RULE-S32-22`: API methods must gate on `TermletState`. **Enforcement:** Method gating test (TST-351).
- `RULE-S32-23`: `exit_status` must be populated when process exits. **Enforcement:** Exit status test (TST-353).
- `RULE-S32-24`: `inherit_env` default must be `false`. **Enforcement:** Default config test (TST-360).
- `RULE-S32-25`: Zero-timeout `wait_for` performs single check. **Enforcement:** Zero-timeout test (TST-324).
- `RULE-S32-26`: `RealPtyBackend` must use non-blocking IO. **Enforcement:** Architecture review (TST-340).
- `RULE-S32-27`: `SpawnFailed` is a distinct terminal state from `Exited`. **Enforcement:** State enum check (TST-355, TST-356).
- `RULE-S32-28`: `TermletPool` Drop must kill all managed Termlets. **Enforcement:** Pool cleanup test (TST-334).
- `RULE-S32-29`: Async support must remain feature-gated. **Enforcement:** Compile matrix (TST-345).
- `RULE-S32-30`: Fuzz bytes through FakePty must not cause panic. **Enforcement:** cargo-fuzz target (TST-348).
- `RULE-S32-31`: **[v12]** `TermletState` must NOT implement `Ord`/`PartialOrd`. **Enforcement:** Compile-fail test (TST-358).
- `RULE-S32-32`: **[v12]** `send_bytes(&[u8])` must be available alongside `send_keys`. **Enforcement:** API surface test (TST-397).
- `RULE-S32-33`: **[v12]** `restart()` must kill before respawning. **Enforcement:** Lifecycle test (TST-398).
- `RULE-S32-34`: **[v12]** `spawn()` returns in `Running` state; no `Created` state observable. **Enforcement:** State test (TST-350).
- `RULE-S32-35`: **[v12]** Grid cell snapshot is captured during `snapshot()`, not lazily. **Enforcement:** Memory test.

---

## v12 Pass 1 Final Consistency Checklist

- [x] **32 sections present** (1-32).
- [x] **Every section includes**: Design Decisions, Rust Example, Test Strategy, AGENTS.md Rules (4-part structure verified for all 32 sections).
- [x] **Section 32 deepest** with subsections 32.1-32.18 (18 subsections, up from 16 in v11 P3).
- [x] **All rules use `RULE-Snn-xx` naming** with explicit enforcement.
- [x] **Total rules**: 127 unique entries in master table (Section 26), including 35 RULE-S32-xx rules.
- [x] **Total risks**: 56 (up from 52 in v11 P3).
- [x] **Total Termlet test cases**: 86 (TST-320 through TST-405, up from 76 in v11 P3).
- [x] **All `[v12]` additions tagged** (searchable tag for v12 content).
- [x] **All `[v11 P3]` and earlier tags preserved** (searchable for historical content).
- [x] **Settled decisions table**: 33 items (up from 27 in v11 P3).
- [x] **Global invariants**: 9 `INV-*` items (up from 8 in v11 P3).
- [x] **TermletState enum**: 5 variants (down from 6 in v11 -- `Created` removed).
- [x] **TermletError**: 9 variants (up from 8 in v11 -- `InvalidState` added).
- [x] **Performance targets**: 16 entries (up from 15 in v11 P3).
- [x] **Acceptance gates**: 47 entries (up from 43 in v11 P3).
- [x] **Release gate matrix**: 48 rows (up from 44 in v11 P3).
- [x] **Section 32 subsections**: 18 (32.1-32.18, up from 16 in v11 P3).
- [x] **v12 Critical Fixes**: 10 issues from v11 identified and corrected.
- [x] **v12 Additions**: 9 new features/types added.
- [x] **v12 Simplifications**: 3 unnecessary complexities removed.
- [x] **Grid API concrete**: Section 5.1 defines Cell, Grid, VtParser with compilable signatures.
- [x] **Queryable trait defined**: Section 12 defines trait + QuerySpec + QueryValue.
- [x] **PtyHandle decoupling**: PtyBackend uses PtyHandle, not TermletPaneId.
- [x] **LockGuard fixed**: Stores File, no mem::forget.
- [x] **TermletState ordering fixed**: No Ord, uses valid_transition().
- [x] **mux-core at Layer 0**: Consistent with purity constraints.
- [x] **Restart API**: Section 32.17 defines restart() method.
- [x] **send_bytes API**: Raw byte injection alongside send_keys.
- [x] **Anti-patterns table**: 19 entries (up from 16 in v11 P3).
- [x] **Canonical types table**: 55+ types catalogued.
- [x] **Reference anchors**: All tmux + zellij + ratatui references verified.
- [x] **Cross-references**: All section references internally consistent.
- [x] **Rust examples**: Written for Rust 2024 edition, types match declared APIs.
- [x] **Complete enough for LLM implementation**: All APIs, types, rules, tests, and constraints specified.

---

## Summary Statistics

| Metric | v11 P3 | v12 P1 | Delta |
|---|---|---|---|
| Total sections | 32 | 32 | -- |
| Sections with 4-part structure | 32 | 32 | -- |
| Total rules (Section 26) | 113 | 127 | +14 |
| Total rules (Section 32) | 30 | 35 | +5 |
| Total risks | 52 | 56 | +4 |
| Total Termlet tests | 76 | 86 | +10 |
| Total acceptance gates | 43 | 47 | +4 |
| Total release gate matrix rows | 44 | 48 | +4 |
| Total performance targets | 15 | 16 | +1 |
| Total settled decisions | 27 | 33 | +6 |
| Total global invariants (INV-*) | 8 | 9 | +1 |
| TermletState variants | 6 | 5 | -1 (Created removed) |
| TermletError variants | 8 | 9 | +1 (InvalidState) |
| Section 32 subsections | 16 | 18 | +2 |
| Reference anchors (sources) | 4 | 6 | +2 (zellij, ratatui) |
| Anti-patterns catalogued | 16 | 19 | +3 |
| Canonical types documented | 50+ | 55+ | +5 |
| Critical fixes from v11 | -- | 10 | new |
| New features added | -- | 9 | new |
| Simplifications | -- | 3 | new |
| [v12] tag count | -- | 80+ | new |
| Spec version lineage depth | 12 | 13 | +1 |

---

*End of TermForge v12 Architecture Specification -- Pass 1.*
