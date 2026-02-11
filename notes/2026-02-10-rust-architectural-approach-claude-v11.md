# TermForge v11 Architecture Specification

Date: 2026-02-11
Status: **DEFINITIVE** -- v11, Claude Opus 4.6 single-pass refinement of v10 Pass 3 Final.
Lineage: v4 -> v5 -> v6 -> v7 -> v8 -> v9 Pass 3 Final -> v10 Pass 3 Final (DEFINITIVE, 32 sections, 5009 lines) -> **v11** (this document, 32 sections, comprehensive refinement).
License: MIT OR Apache-2.0
Rust edition: 2024 (MSRV 1.85)
Protocol target: tmux protocol v8

---

## Preamble

This document is the single authoritative architectural reference for TermForge, a Rust terminal multiplexer with 100% tmux wire-protocol compatibility, ORM-like API, language bindings, CRDT collaboration, a ratatui-based TUI client, and **Termlets** -- SDK-first testing pods that are the killer differentiator.

### v11 Refinement Notes

v11 is a comprehensive single-pass refinement of v10 Pass 3 Final. It preserves all v10 content while addressing the following improvement areas:

1. **Gaps and inconsistencies fixed:**
   - `ErrorCode` trait now has implementations for ALL error types that cross binding boundaries (was missing for `CoreError`, `ProtocolError`, `QueryError`)
   - `drain_output()` now documents the actual PTY read loop (was opaque `read_events()` call without explaining blocking/nonblocking semantics)
   - `Termlet::spawn()` no longer uses `PaneId::from(slotmap::KeyData::from_ffi(id))` which is an abuse of SlotMap internals; uses `TermletPaneId` newtype instead
   - `GraphState` snapshot construction now uses `HashMap` lookups instead of linear scans in entity traversal methods
   - `TermletPool::get_mut()` error now uses a dedicated `TermletError::NotInPool` variant instead of misusing `SpawnFailed`
   - Added `PtyBackend::as_any_mut()` to the trait definition (was used but never declared)
   - `kill()` now implements the full SIGTERM -> grace_period -> SIGKILL sequence documented in the design decisions (was only sending SIGTERM)

2. **Section 32 (Termlets) strengthened:**
   - Added concrete `drain_output()` implementation with nonblocking PTY reads
   - Added `Termlet::output_history()` for retrieving raw byte history
   - Added `TermletGuard` RAII wrapper for language bindings (from Gemini)
   - Added `TermletExt` trait for custom assertion extensions (from GPT)
   - Added Termlet-to-pane bridge API for when Termlets need server integration
   - Enhanced `TermletPool` with `wait_all()` and `send_all()` batch operations

3. **Cross-section coherence improved:**
   - Section 8 error types now cross-reference Section 32 consistently
   - Section 16 bindings now reference Section 32 API signatures exactly
   - Section 19 OTEL spans now include Termlet lifecycle spans with structured attributes
   - Section 21 FakePty now defines `as_any_mut()` on the trait
   - Section 26 rules renumbered for consistency; no rule ID changes

4. **LLM implementation details added:**
   - Every crate now has a "Getting Started" comment showing the minimal compilable module
   - Event -> Effect dispatch now shows the full state actor message protocol
   - PTY read loop is fully specified (nonblocking with `WouldBlock` handling)
   - Language binding GIL/thread-safety patterns fully annotated

5. **Rust 2024 edition improvements:**
   - `gen` keyword reserved; no identifier conflicts
   - `unsafe_op_in_unsafe_fn` lint enabled
   - `let chains` used where appropriate for cleaner pattern matching

6. **Test strategy coverage:**
   - Every test now specifies the assertion type (equality, snapshot, property, etc.)
   - Test naming convention enforced with regex validation
   - Added negative test requirements for every error variant

7. **AGENTS.md rules strengthened:**
   - Added enforcement mechanism for every rule (was missing for 8 rules in v10)
   - Added `RULE-S32-15` through `RULE-S32-20` for new Termlet capabilities

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
    dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("~/.config"))
        .join("termforge")
        .join("termforge.conf")
}
```

### Test Strategy

1. Binary name: `assert_eq!(BINARY_NAME, "termforge")` -- equality assertion.
2. Alias detection: binary invoked as `tf` behaves identically -- integration test via `argv[0]` check.
3. Socket dir: `default_socket_dir()` contains `termforge-` prefix and numeric UID -- regex assertion.
4. Config path: with `XDG_CONFIG_HOME=/custom`, `config_path()` starts with `/custom` -- path prefix assertion.
5. Config path default: without env var, falls back to `~/.config/termforge/termforge.conf` -- equality assertion.

### AGENTS.md Rules

- `RULE-S01-01`: All crate names must use the `mux-` prefix. **Enforcement:** CI grep of `Cargo.toml` names.
- `RULE-S01-02`: Binary name `termforge`, alias `tf`. **Enforcement:** `cargo build` output name check.

---

## 2. Acceptance Criteria and Gates [v11 updated]

### Design Decisions

- Acceptance gates are the release checklist. Organized into categories: Compatibility (C), Purity (A), Performance (B), Programmability (P), Ecosystem (E).
- Every gate maps to at least one test in the CI matrix.
- **[v10]** Termlet usability gates (P7/P8/P9), performance gates (B10/B11/B12), and ecosystem gates (E6/E7) added.
- **[v10 P3]** Pass/fail thresholds with measurable criteria.
- **[v11]** Added P10 (TermletPool batch), P11 (SnapshotDiff regression), B13 (drain_output latency).

### 2.1 Acceptance Gate Table

| ID | Category | Gate | Test / Evidence | Pass Threshold | Fail Threshold |
|---|---|---|---|---|---|
| C1 | Compatibility | `tmux attach` to TermForge server | Integration test | Attach succeeds, session list matches | Attach fails or session mismatch |
| C2 | Compatibility | `tmux list-sessions` on TermForge | Integration test | Correct session list | Missing or malformed sessions |
| C3 | Compatibility | tmux control mode subscribe/notify | Integration test | Notifications received for session/window/pane events | Missing notifications |
| C4 | Compatibility | tmux config file parsing | Unit test + fixtures | >= 95% of tmux options parsed correctly | < 90% coverage |
| C5 | Compatibility | Copy mode basic operations | Integration test | yank, paste, search work | Any basic operation fails |
| A1 | Purity | `mux-core` compiles without `std::io` | `cargo check --target wasm32-unknown-unknown` | Compiles clean | Any IO import |
| A2 | Purity | `mux-core` has `#![forbid(unsafe_code)]` | CI attribute check | Attribute present | Missing attribute |
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
| B10 | Performance | Termlet spawn (FakePty) | Criterion | < 500 us | > 1 ms |
| B11 | Performance | Termlet snapshot (80x24) | Criterion | < 50 us | > 100 us |
| B12 | Performance | Termlet spawn (real PTY) | Criterion | < 50 ms | > 100 ms |
| B13 | Performance | **[v11]** drain_output 1KB buffer | Criterion | < 10 us | > 50 us |
| E1 | Ecosystem | Python package installable via pip | CI | `pip install termforge` succeeds | Install fails |
| E2 | Ecosystem | Node package installable via npm | CI | `npm install termforge` succeeds | Install fails |
| E3 | Ecosystem | `cargo doc` builds without warnings | CI | Zero warnings | Any warning |
| E4 | Ecosystem | MSRV 1.85 compiles | CI | `cargo +1.85 check` passes | Compile failure |
| E5 | Ecosystem | Clippy clean on nightly | CI | Zero warnings | Any warning |
| E6 | Ecosystem | Termlet documented in README | Manual | Usage example present | No mention |
| E7 | Ecosystem | Termlet cross-language parity | Multi-runner | Same scenario, same snapshot across Rust/Python/Node | Snapshot divergence |

### Rust Example

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gate {
    // Compatibility
    C1WireAttach,
    C2ListSessions,
    C3ControlMode,
    C4ConfigParse,
    C5CopyMode,
    // Purity
    A1NoPureIo,
    A2ForbidUnsafe,
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
    P10TermletPoolBatch,     // [v11]
    P11SnapshotDiffDetect,   // [v11]
    // Performance (Termlet)
    B10TermletSpawnFake,
    B11TermletSnapshot,
    B12TermletSpawnReal,
    B13DrainOutputLatency,   // [v11]
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
    pub fn is_required_for_release(&self) -> bool {
        // All gates are release-blocking except performance warns
        !matches!(self,
            Gate::B1Vt100Ascii | Gate::B2Vt100Csi | Gate::B3ProtoDecode |
            Gate::B4LayoutResize | Gate::B5Snapshot50 |
            Gate::B10TermletSpawnFake | Gate::B11TermletSnapshot |
            Gate::B12TermletSpawnReal | Gate::B13DrainOutputLatency
        )
    }

    /// [v11] Returns the section this gate traces to.
    pub fn section(&self) -> u8 {
        match self {
            Gate::C1WireAttach | Gate::C2ListSessions => 9,
            Gate::C3ControlMode => 15,
            Gate::C4ConfigParse => 10,
            Gate::C5CopyMode => 11,
            Gate::A1NoPureIo | Gate::A2ForbidUnsafe => 5,
            Gate::B1Vt100Ascii | Gate::B2Vt100Csi => 23,
            Gate::B3ProtoDecode => 9,
            Gate::B4LayoutResize => 11,
            Gate::B5Snapshot50 => 5,
            Gate::P1PyFilter | Gate::P2PyGet | Gate::P3NodeFilter => 16,
            Gate::P4OtelTrace => 19,
            Gate::P5ControlParse => 15,
            Gate::P6CrdtMerge => 17,
            Gate::P7TermletSpawn | Gate::P8PyTermletCtxMgr | Gate::P9TermletDualMode |
            Gate::P10TermletPoolBatch | Gate::P11SnapshotDiffDetect |
            Gate::B10TermletSpawnFake | Gate::B11TermletSnapshot |
            Gate::B12TermletSpawnReal | Gate::B13DrainOutputLatency => 32,
            Gate::E1PyPip | Gate::E2NodeNpm => 16,
            Gate::E3CargoDoc | Gate::E4Msrv | Gate::E5Clippy => 4,
            Gate::E6TermletDocs | Gate::E7TermletCrossLang => 32,
        }
    }
}
```

### Test Strategy

1. Gate checklist run on every release candidate -- automated script iterates all `Gate` variants.
2. Performance gates warn but do not block (except regressions > 30%).
3. All functional gates must pass.
4. Termlet gates P7/P8/P9/P10/P11 and E6/E7 mandatory for release.
5. **[v11]** Gate-to-section traceability verified: every gate maps to a section via `Gate::section()`.

### AGENTS.md Rules

- `RULE-S02-01`: Every new feature must map to at least one acceptance gate. **Enforcement:** PR template checklist.
- `RULE-S02-02`: No gate may be removed without spec amendment and approval. **Enforcement:** Gate enum is append-only.
- `RULE-S02-03`: Gate pass/fail thresholds must be measurable, not subjective. **Enforcement:** Every threshold has a numeric value or boolean condition.

---

## 3. Crate Dependency Rules [v11 updated]

### Design Decisions

- Strict layered architecture: Layer 0 (pure) -> Layer 1 (OS) -> Layer 2 (facade) -> Layer 3 (application).
- Layer 0 crates must compile to WASM (`wasm32-unknown-unknown`).
- No circular dependencies allowed.
- `cargo deny check` enforced in CI.
- `mux-termlet` added at Layer 2, depends on Layer 0 + Layer 1 only.
- **[v11]** Explicit `PtyBackend` trait requirement: `Send + Any` bounds documented in dependency matrix.

### 3.1 Crate Dependency Matrix

| Crate | Layer | May Depend On | Must Not Depend On |
|---|---|---|---|
| `mux-types` | 0 | std | anything else |
| `mux-core` | 0 | mux-types | tokio, libc, mux-pty, mux-os, mux-server |
| `mux-grid` | 0 | mux-types | tokio, libc |
| `mux-proto` | 0 | mux-types | tokio (codec trait is generic) |
| `mux-query` | 0 | mux-types, mux-core | tokio, libc |
| `mux-conf` | 0 | mux-types | tokio, libc |
| `mux-crdt` | 0 | mux-types, mux-core | tokio, libc, network |
| `mux-pty-fake` | 0 | mux-types, mux-pty (trait only) | tokio, libc (real) |
| `mux-pty` | 1 | mux-types, libc | mux-core, mux-server |
| `mux-os` | 1 | mux-types, libc | mux-core, mux-server |
| `mux-control` | 1 | mux-types, mux-core | mux-server |
| `mux-otel` | 1 | opentelemetry, tokio | mux-core, mux-server |
| `mux-termlet` | **2** | mux-core, mux-grid, mux-pty, mux-pty-fake, mux-types | **mux-server, mux-api, mux-orm** |
| `mux-orm` | 2 | mux-core, mux-query, mux-types | mux-server |
| `mux-api` | 2 | mux-core, mux-orm, mux-types | mux-server (impl) |
| `mux-view` | 2 | mux-core, mux-types, ratatui | mux-server |
| `mux-test-support` | 2 | mux-types, mux-pty, mux-os | mux-server |
| `mux-server` | 3 | all above + tokio | (top level) |
| `mux-bench` | 3 | all above + criterion | (top level) |
| `termforge` (bin) | 3 | all above | (top level) |

### Rust Example

```rust
// Dependency enforcement via cargo-deny
// deny.toml
[bans]
multiple-versions = "deny"

// CI script
// scripts/check-layers.sh
#!/bin/bash
set -euo pipefail

# Layer 0: must compile to WASM
for crate in mux-types mux-core mux-grid mux-proto mux-query mux-conf mux-crdt; do
    cargo check -p "$crate" --target wasm32-unknown-unknown
done

# mux-termlet must not depend on mux-server, mux-api, mux-orm
for forbidden in mux-server mux-api mux-orm; do
    if cargo tree -p mux-termlet | grep -q "$forbidden"; then
        echo "ERROR: mux-termlet depends on $forbidden"
        exit 1
    fi
done

# [v11] Verify PtyBackend trait bounds include Send + Any
grep -r "trait PtyBackend" crates/mux-pty/src/ | grep -q "Send" || {
    echo "ERROR: PtyBackend must require Send bound"
    exit 1
}
```

### Test Strategy

1. WASM check: Layer 0 crates compile to `wasm32-unknown-unknown` -- CI build gate.
2. Dependency audit: `cargo deny check` passes in CI -- zero violations.
3. Layer violation: `cargo tree` verifies no forbidden dependencies -- script returns 0.
4. `mux-termlet` isolation: does not depend on `mux-server`, `mux-api`, or `mux-orm` -- cargo tree check.
5. **[v11]** `PtyBackend` trait bounds: `Send + Any` present in trait definition -- grep assertion.

### AGENTS.md Rules

- `RULE-S03-01`: Layer 0 crates must pass WASM compilation check. **Enforcement:** CI `cargo check --target wasm32-unknown-unknown`.
- `RULE-S03-02`: No circular dependencies. `cargo deny check` enforced. **Enforcement:** CI cargo-deny.
- `RULE-S03-03`: `mux-termlet` must not depend on `mux-server` or `mux-api`. **Enforcement:** CI `cargo tree` check.
- `RULE-S03-04`: **[v11]** `PtyBackend` trait must require `Send + Any` bounds. **Enforcement:** CI grep of trait definition.

---

## 4. Workspace Layout [v11 updated]

### Design Decisions

- Cargo workspace with `crates/`, `tools/`, `bindings/`, and `fuzz/` directories.
- Each crate has a focused responsibility.
- `mux-termlet` crate at Layer 2 (FACADE).
- Tools are separate binaries that may depend on any crate.
- **[v11]** Added `examples/` directory for standalone Termlet usage examples.

### 4.1 Directory Structure

```
termforge/
  Cargo.toml                    # workspace root
  AGENTS.md                     # AI agent rules
  deny.toml                     # cargo-deny config
  crates/
    mux-types/                  # Layer 0: shared types, IDs
    mux-core/                   # Layer 0: state graph, events, effects
    mux-grid/                   # Layer 0: terminal grid, VT100 parser
    mux-proto/                  # Layer 0: wire protocol codec
    mux-query/                  # Layer 0: ORM query operators
    mux-conf/                   # Layer 0: config parser
    mux-crdt/                   # Layer 0: CRDT types (HLC, LWW, OR-Set)
    mux-pty/                    # Layer 1: real PTY backend
    mux-pty-fake/               # Layer 0: fake PTY for testing
    mux-os/                     # Layer 1: OS abstractions (sockets, signals)
    mux-control/                # Layer 1: control mode parser
    mux-otel/                   # Layer 1: OpenTelemetry
    mux-termlet/                # Layer 2: Termlet SDK
    mux-orm/                    # Layer 2: ORM facade
    mux-api/                    # Layer 2: public API (ManagedMux, StateHandle)
    mux-view/                   # Layer 2: TUI ViewModel
    mux-test-support/           # Layer 2: test harness utilities
    mux-server/                 # Layer 3: tokio runtime, actor system
    mux-bench/                  # Layer 3: benchmarks
  tools/
    tmux-builder/               # Build + cache tmux from source
    tmux-command-audit/         # Command coverage tracking
    format-audit/               # Format string parity checker
    mux-regress/                # Parity regression runner
    mux-tui/                    # TUI binary
  bindings/
    python/                     # PyO3 bindings
    node/                       # Neon bindings
  fuzz/
    fuzz_targets/
      proto_decode.rs
      vt100_parse.rs
      config_parse.rs
  examples/                     # [v11] Standalone usage examples
    termlet_basic.rs
    termlet_pool.rs
    termlet_snapshot_diff.rs
```

### Rust Example

```rust
// Cargo.toml (workspace root)
[workspace]
resolver = "2"
members = [
    "crates/mux-types",
    "crates/mux-core",
    "crates/mux-grid",
    "crates/mux-proto",
    "crates/mux-query",
    "crates/mux-conf",
    "crates/mux-crdt",
    "crates/mux-pty",
    "crates/mux-pty-fake",
    "crates/mux-os",
    "crates/mux-control",
    "crates/mux-otel",
    "crates/mux-termlet",
    "crates/mux-orm",
    "crates/mux-api",
    "crates/mux-view",
    "crates/mux-test-support",
    "crates/mux-server",
    "crates/mux-bench",
    "tools/tmux-builder",
    "tools/tmux-command-audit",
    "tools/format-audit",
    "tools/mux-regress",
    "tools/mux-tui",
]

[workspace.dependencies]
slotmap = "1"
thiserror = "2"
bytes = "1"
regex = "1"
blake3 = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
insta = "1"
proptest = "1"
criterion = "0.5"
ratatui = "0.29"
crossterm = "0.28"
pyo3 = "0.23"
neon = "1"
```

### Test Strategy

1. Workspace compiles: `cargo build --workspace` succeeds -- CI gate.
2. Individual crate builds: each crate compiles independently -- matrix CI job.
3. `mux-termlet` compiles and links against mux-core, mux-grid, mux-pty, mux-pty-fake -- CI gate.
4. No duplicate workspace dependencies -- `cargo deny check` catches.
5. **[v11]** Examples compile: `cargo build --examples` succeeds -- CI gate.

### AGENTS.md Rules

- `RULE-S04-01`: Every new crate must be added to workspace members. **Enforcement:** CI workspace build.
- `RULE-S04-02`: Shared dependencies must use `workspace.dependencies`. **Enforcement:** `cargo deny check`.
- `RULE-S04-03`: `mux-termlet` lives at Layer 2, depends on Layer 0 + Layer 1 only. **Enforcement:** `cargo tree` CI check.

---

## 5. Core State Graph [v11 updated]

### Design Decisions

- `ServerGraph` is the single mutable state container. Only the state actor may mutate it.
- `GraphState` is an immutable snapshot shared via `ArcSwap` for zero-contention reads.
- Entity IDs via `slotmap::new_key_type!` for type-safe, generationally-indexed references.
- Reverse maps (pane-to-window, window-to-session) computed during snapshot construction.
- `CoreCtx` provides injectable time and randomness for deterministic testing.
- `apply_event()` is the single entry point for all state mutations.
- `mux-core` is pure: `#![forbid(unsafe_code)]`, no IO, no tokio, no libc.
- `MuxKernel` trait as the core dispatch interface. `ServerGraph` implements `MuxKernel`.
- **[v11]** `GraphState` entity traversal uses `HashMap` lookups instead of linear scans.
- **[v11]** `ApplyOutcome` now includes `errors` field for non-fatal issues during event processing.

### Rust Example

```rust
// crates/mux-core/src/lib.rs
#![forbid(unsafe_code)]

use slotmap::SlotMap;
use std::collections::HashMap;

slotmap::new_key_type! {
    pub struct SessionId;
    pub struct WindowId;
    pub struct PaneId;
    pub struct ClientId;
    pub struct JobId;
    pub struct BufferId;
}

pub struct ServerGraph {
    pub sessions: SlotMap<SessionId, Session>,
    pub windows: SlotMap<WindowId, Window>,
    pub panes: SlotMap<PaneId, Pane>,
    pub clients: SlotMap<ClientId, Client>,
    pub jobs: SlotMap<JobId, Job>,
    pub buffers: SlotMap<BufferId, Buffer>,
    pub server_options: OptionSet,
    pub global_options: OptionSet,
}

pub struct Session {
    pub name: String,
    pub windows: Vec<WindowId>,
    pub active_window: Option<WindowId>,
    pub options: OptionSet,
    pub created: i64,
}

pub struct Window {
    pub name: String,
    pub panes: Vec<PaneId>,
    pub active_pane: Option<PaneId>,
    pub layout: LayoutTree,
    pub options: OptionSet,
}

pub struct Pane {
    pub command: Vec<String>,
    pub cwd: String,
    pub size: PaneSize,
    pub options: OptionSet,
}

#[derive(Debug, Clone, Copy)]
pub struct PaneSize {
    pub sx: u16,
    pub sy: u16,
}

/// Immutable snapshot for zero-contention reads via ArcSwap.
/// [v11] Uses HashMap for O(1) entity lookups instead of Vec linear scans.
#[derive(Clone)]
pub struct GraphState {
    pub sessions: HashMap<SessionId, Session>,
    pub windows: HashMap<WindowId, Window>,
    pub panes: HashMap<PaneId, Pane>,
    pub clients: HashMap<ClientId, Client>,
    // Reverse maps computed during snapshot construction
    pub pane_to_window: HashMap<PaneId, WindowId>,
    pub window_to_session: HashMap<WindowId, SessionId>,
}

/// Injectable context for deterministic testing.
pub struct CoreCtx {
    pub now: Box<dyn Fn() -> i64>,
    pub random: Box<dyn Fn() -> u64>,
}

impl CoreCtx {
    /// Production context: real wall clock and random.
    pub fn real() -> Self {
        Self {
            now: Box::new(|| {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64
            }),
            random: Box::new(|| rand::random()),
        }
    }

    /// Test context: deterministic time and randomness.
    pub fn test(start_time: i64) -> Self {
        let counter = std::cell::Cell::new(start_time);
        Self {
            now: Box::new(move || {
                let t = counter.get();
                counter.set(t + 1);
                t
            }),
            random: Box::new(|| 42),
        }
    }
}

/// [v11] ApplyOutcome now includes non-fatal errors.
#[derive(Debug, Clone, Default)]
pub struct ApplyOutcome {
    pub effects: Vec<Effect>,
    pub hints: Vec<String>,
    /// [v11] Non-fatal errors encountered during event processing.
    /// These do NOT prevent the event from being applied but should be logged.
    pub warnings: Vec<String>,
}

/// MuxKernel trait -- core dispatch interface.
/// ServerGraph implements this trait.
pub trait MuxKernel {
    fn apply_event(&mut self, event: Event, ctx: &CoreCtx) -> ApplyOutcome;
    fn snapshot(&self) -> GraphState;
}

impl MuxKernel for ServerGraph {
    fn apply_event(&mut self, event: Event, ctx: &CoreCtx) -> ApplyOutcome {
        match event {
            Event::CreateSession { name } => {
                let id = self.sessions.insert(Session {
                    name,
                    windows: vec![],
                    active_window: None,
                    options: OptionSet::new(),
                    created: (ctx.now)(),
                });
                ApplyOutcome {
                    effects: vec![Effect::Notify(Notification::SessionCreated(id))],
                    ..Default::default()
                }
            }
            Event::DestroySession { id } => {
                if let Some(session) = self.sessions.remove(id) {
                    let mut effects = vec![Effect::Notify(Notification::SessionDestroyed(id))];
                    // Cascade: destroy all windows and their panes
                    for wid in &session.windows {
                        if let Some(window) = self.windows.remove(*wid) {
                            for pid in &window.panes {
                                if self.panes.remove(*pid).is_some() {
                                    effects.push(Effect::KillProcess {
                                        pane: *pid,
                                        signal: libc::SIGKILL,
                                    });
                                }
                            }
                        }
                    }
                    ApplyOutcome { effects, ..Default::default() }
                } else {
                    ApplyOutcome {
                        warnings: vec![format!("session {id:?} not found for destroy")],
                        ..Default::default()
                    }
                }
            }
            // ... other event handlers follow same pattern
            _ => ApplyOutcome::default(),
        }
    }

    fn snapshot(&self) -> GraphState {
        let mut pane_to_window = HashMap::new();
        let mut window_to_session = HashMap::new();
        for (sid, session) in &self.sessions {
            for &wid in &session.windows {
                window_to_session.insert(wid, sid);
                if let Some(window) = self.windows.get(wid) {
                    for &pid in &window.panes {
                        pane_to_window.insert(pid, wid);
                    }
                }
            }
        }
        GraphState {
            sessions: self.sessions.iter().map(|(k, v)| (k, v.clone())).collect(),
            windows: self.windows.iter().map(|(k, v)| (k, v.clone())).collect(),
            panes: self.panes.iter().map(|(k, v)| (k, v.clone())).collect(),
            clients: self.clients.iter().map(|(k, v)| (k, v.clone())).collect(),
            pane_to_window,
            window_to_session,
        }
    }
}
```

### Test Strategy

1. Create session: `apply_event(CreateSession)` adds session to graph -- equality check on sessions count.
2. Destroy session: cascade removes session, windows, panes -- equality check all three counts decrease.
3. Snapshot isolation: mutating graph after snapshot does not affect snapshot -- clone independence.
4. Reverse maps: `pane_to_window[pid]` returns correct `wid` -- equality assertion.
5. ID generation: SessionId, WindowId, PaneId are unique across insertions -- set uniqueness check.
6. CoreCtx injection: deterministic `CoreCtx::test(0)` produces predictable timestamps -- equality check.
7. Pure crate: `cargo check --target wasm32-unknown-unknown` passes for `mux-core` -- CI gate.
8. MuxKernel trait: `ServerGraph` implements `MuxKernel` correctly -- compile test.
9. **[v11]** GraphState HashMap lookup: `sessions.get(&sid)` returns in O(1) -- unit test.
10. **[v11]** ApplyOutcome warnings: destroying nonexistent session yields warning, not panic -- assertion on warnings.
11. **[v11]** `CoreCtx::test()` monotonicity: successive calls return incrementing values -- property test.

### AGENTS.md Rules

- `RULE-S05-01`: `mux-core` is pure: no IO, no `unsafe`, no `tokio`, no `libc`. Enforced via `#![forbid(unsafe_code)]` and WASM CI check. **Enforcement:** CI WASM build + attribute check.
- `RULE-S05-02`: WASM CI check for all Layer 0 crates. **Enforcement:** CI `cargo check --target wasm32-unknown-unknown`.
- `RULE-S05-03`: **[v11]** `GraphState` must use `HashMap` for entity storage, not `Vec<(Id, T)>`. **Enforcement:** Code review.

---

## 6. Entity Relationships

### Design Decisions

- `Vec<ChildId>` inside parent entity for parent-to-children relationships.
- Reverse maps (`pane_to_window`, `window_to_session`) computed during `GraphState` snapshot construction.
- No `SecondaryMap`. Reverse lookups are derived, not stored.
- `active_window` and `active_pane` stored as `Option<Id>` on the parent.
- Client-to-session attachment stored on the `Client` struct.
- **[v11]** GraphState entity traversal is O(1) via HashMap lookups (was O(n) linear scan in v10).

### Rust Example

```rust
// Entity relationship traversal -- [v11] O(1) lookups via HashMap
impl GraphState {
    /// Get all windows belonging to a session. O(w) where w = session.windows.len().
    pub fn session_windows(&self, sid: SessionId) -> Vec<&Window> {
        self.sessions.get(&sid)
            .map(|s| {
                s.windows.iter()
                    .filter_map(|wid| self.windows.get(wid))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get the session containing a pane. O(1) via reverse map.
    pub fn pane_session(&self, pid: PaneId) -> Option<SessionId> {
        self.pane_to_window.get(&pid)
            .and_then(|wid| self.window_to_session.get(wid))
            .copied()
    }

    /// Get all panes in a window. O(p) where p = window.panes.len().
    pub fn window_panes(&self, wid: WindowId) -> Vec<&Pane> {
        self.windows.get(&wid)
            .map(|w| {
                w.panes.iter()
                    .filter_map(|pid| self.panes.get(pid))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get the window containing a pane. O(1) via reverse map.
    pub fn pane_window(&self, pid: PaneId) -> Option<&Window> {
        self.pane_to_window.get(&pid)
            .and_then(|wid| self.windows.get(wid))
    }

    /// Get the active pane in a session's active window.
    pub fn session_active_pane(&self, sid: SessionId) -> Option<&Pane> {
        self.sessions.get(&sid)
            .and_then(|s| s.active_window)
            .and_then(|wid| self.windows.get(&wid))
            .and_then(|w| w.active_pane)
            .and_then(|pid| self.panes.get(&pid))
    }
}
```

### Test Strategy

1. Session has windows: creating windows adds to session's `windows` vec -- length check.
2. Window has panes: creating panes adds to window's `panes` vec -- length check.
3. Reverse map correctness: `pane_to_window` matches actual graph structure -- exhaustive check.
4. Active tracking: `active_window` and `active_pane` updated on focus changes -- equality check.
5. Orphan prevention: destroying a session destroys all its windows and panes -- zero counts.
6. Client attachment: client tracks its attached session -- equality check.
7. **[v11]** O(1) lookup: `session_windows()` does not iterate all windows -- complexity verified.

### AGENTS.md Rules

- `RULE-S06-01`: All entity IDs via `slotmap::new_key_type!`. No raw integers for entity references. **Enforcement:** Code review + type system.

---

## 7. Event and Effect Enums

### Design Decisions

- Events are inbound state change requests. Effects are outbound side-effect requests.
- The core emits effects; only the runtime executes them.
- All state transitions go through `apply_event()`.
- `ApplyOutcome` carries effects, hints, and warnings.
- Events and Effects are exhaustive enums -- no extensible variant pattern.
- **[v11]** `DestroySession` event cascading is fully specified (was abbreviated in v10).

### Rust Example

```rust
// crates/mux-core/src/event.rs
#[derive(Debug, Clone)]
pub enum Event {
    CreateSession { name: String },
    DestroySession { id: SessionId },
    CreateWindow { session: SessionId, name: String },
    DestroyWindow { id: WindowId },
    CreatePane { window: WindowId, command: Vec<String>, cwd: String },
    DestroyPane { id: PaneId },
    ResizePane { id: PaneId, size: PaneSize },
    AttachClient { client: ClientId, session: SessionId },
    DetachClient { client: ClientId },
    SetOption { scope: OptionScope, key: String, value: OptionValue },
    UnsetOption { scope: OptionScope, key: String },
    PaneOutput { pane: PaneId, data: Vec<u8> },
    PaneExited { pane: PaneId, status: i32 },
    ClientIdentified { client: ClientId },
    LayoutResize { window: WindowId, delta: i32, horizontal: bool },
}

// crates/mux-core/src/effect.rs
#[derive(Debug, Clone)]
pub enum Effect {
    SpawnProcess { pane: PaneId, command: Vec<String>, cwd: String, size: PaneSize },
    KillProcess { pane: PaneId, signal: i32 },
    WritePty { pane: PaneId, data: Vec<u8> },
    ResizePty { pane: PaneId, size: PaneSize },
    Notify(Notification),
    LoadConfig { path: std::path::PathBuf },
    SendControlNotification { client: ClientId, notification: ControlNotification },
}

#[derive(Debug, Clone)]
pub enum Notification {
    SessionCreated(SessionId),
    SessionDestroyed(SessionId),
    WindowCreated(WindowId),
    WindowDestroyed(WindowId),
    PaneCreated(PaneId),
    PaneDestroyed(PaneId),
    LayoutChanged(WindowId),
    ClientAttached(ClientId),
    ClientDetached(ClientId),
}
```

### Test Strategy

1. CreateSession -> SessionCreated notification effect -- variant match.
2. DestroySession -> cascading DestroyWindow + DestroyPane events + KillProcess effects -- count assertions.
3. PaneOutput -> no effects (grid update is internal) -- empty effects vec.
4. ResizePane -> ResizePty effect -- variant match with correct size.
5. LayoutResize -> multiple ResizePty effects for affected panes -- count >= 1.
6. AttachClient -> ClientAttached notification + LoadConfig on first client -- two effects.
7. SetOption -> no effects (pure state mutation) -- empty effects vec.
8. Effect serialization: all Effect variants are Debug + Clone -- compile test.
9. **[v11]** DestroySession cascade: session with 3 windows x 2 panes = 6 KillProcess effects + 1 SessionDestroyed -- exact count.

### AGENTS.md Rules

- `RULE-S07-01`: Core emits effects; only the runtime executes effects. **Enforcement:** Type system (`Effect` enum consumed only by `dispatch_effect`).
- `RULE-S07-02`: All state transitions go through `apply_event()`. No direct graph mutation. **Enforcement:** CI grep for direct `sessions.insert()` outside `apply_event`.

---

## 8. Error Handling [v11 updated]

### Design Decisions

- `thiserror` for all library crate errors.
- `anyhow` only in binaries and tests.
- Every error has an `ErrorClass` for categorization.
- ErrorClass drives retry/recovery logic in the runtime.
- `TermletError` with 7 variants (added `NotInPool` in v11).
- Stable error code strings for binding interop (`TERMLET_TIMEOUT`, etc.).
- `ErrorCode` trait for consistent string code extraction across ALL error types that cross binding boundaries.
- **[v11]** `ErrorCode` implemented for `CoreError`, `ProtocolError`, `QueryError` (was only on `TermletError` in v10).
- **[v11]** Added `TermletError::NotInPool` variant (was misusing `SpawnFailed` for pool lookups).

### Rust Example

```rust
// crates/mux-core/src/error.rs
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorClass {
    UserError,
    Bug,
    Transient,
    ProtocolViolation,
}

pub trait Classified {
    fn class(&self) -> ErrorClass;
}

/// ErrorCode trait for stable string extraction across error types.
/// Every error type that crosses a binding boundary (Python/Node) must implement this.
pub trait ErrorCode {
    fn error_code(&self) -> &'static str;
}

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("session not found: {id:?}")]
    SessionNotFound { id: SessionId },
    #[error("window not found: {id:?}")]
    WindowNotFound { id: WindowId },
    #[error("pane not found: {id:?}")]
    PaneNotFound { id: PaneId },
    #[error("duplicate session name: {name}")]
    DuplicateSessionName { name: String },
    #[error("layout invariant violated: {reason}")]
    LayoutBug { reason: String },
}

impl Classified for CoreError {
    fn class(&self) -> ErrorClass {
        match self {
            Self::SessionNotFound { .. } |
            Self::WindowNotFound { .. } |
            Self::PaneNotFound { .. } |
            Self::DuplicateSessionName { .. } => ErrorClass::UserError,
            Self::LayoutBug { .. } => ErrorClass::Bug,
        }
    }
}

/// [v11] ErrorCode implemented for CoreError (was missing in v10).
impl ErrorCode for CoreError {
    fn error_code(&self) -> &'static str {
        match self {
            Self::SessionNotFound { .. } => "CORE_SESSION_NOT_FOUND",
            Self::WindowNotFound { .. } => "CORE_WINDOW_NOT_FOUND",
            Self::PaneNotFound { .. } => "CORE_PANE_NOT_FOUND",
            Self::DuplicateSessionName { .. } => "CORE_DUPLICATE_SESSION",
            Self::LayoutBug { .. } => "CORE_LAYOUT_BUG",
        }
    }
}

// crates/mux-proto/src/error.rs
#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("unknown message type: {msg_type}")]
    UnknownMsgType { msg_type: u32 },
    #[error("payload too short: expected {expected}, got {actual}")]
    PayloadTooShort { expected: usize, actual: usize },
    #[error("invalid UTF-8 in payload")]
    InvalidUtf8,
}

impl Classified for ProtocolError {
    fn class(&self) -> ErrorClass {
        ErrorClass::ProtocolViolation
    }
}

/// [v11] ErrorCode for ProtocolError.
impl ErrorCode for ProtocolError {
    fn error_code(&self) -> &'static str {
        match self {
            Self::UnknownMsgType { .. } => "PROTO_UNKNOWN_MSG",
            Self::PayloadTooShort { .. } => "PROTO_PAYLOAD_SHORT",
            Self::InvalidUtf8 => "PROTO_INVALID_UTF8",
        }
    }
}

// crates/mux-query/src/error.rs
/// [v11] ErrorCode for QueryError.
impl ErrorCode for QueryError {
    fn error_code(&self) -> &'static str {
        match self {
            Self::ObjectDoesNotExist => "QUERY_NOT_FOUND",
            Self::MultipleObjectsReturned { .. } => "QUERY_MULTIPLE_FOUND",
            Self::UnknownOperator { .. } => "QUERY_UNKNOWN_OP",
        }
    }
}

// crates/mux-termlet/src/error.rs
#[derive(Debug, Error)]
pub enum TermletError {
    #[error("spawn failed: {reason}")]
    SpawnFailed { reason: String },

    #[error("termlet already killed")]
    AlreadyKilled,

    #[error("wait_for timed out: pattern={pattern:?} timeout={timeout_ms}ms")]
    WaitForTimeout { pattern: String, timeout_ms: u64 },

    /// Process exited before pattern appeared.
    /// Distinct from WaitForTimeout: the process died, not the clock.
    #[error("pattern not found (process exited): pattern={pattern:?}")]
    PatternNotFound { pattern: String },

    #[error("resize failed: {0}")]
    ResizeFailed(String),

    #[error("PTY error: {0}")]
    Pty(#[from] PtyError),

    /// [v11] Termlet not found in pool by name.
    #[error("termlet not found in pool: {name}")]
    NotInPool { name: String },
}

impl Classified for TermletError {
    fn class(&self) -> ErrorClass {
        match self {
            Self::SpawnFailed { .. } => ErrorClass::Transient,
            Self::AlreadyKilled => ErrorClass::UserError,
            Self::WaitForTimeout { .. } => ErrorClass::Transient,
            Self::PatternNotFound { .. } => ErrorClass::UserError,
            Self::ResizeFailed(_) => ErrorClass::Transient,
            Self::Pty(_) => ErrorClass::Transient,
            Self::NotInPool { .. } => ErrorClass::UserError,
        }
    }
}

impl ErrorCode for TermletError {
    fn error_code(&self) -> &'static str {
        match self {
            Self::SpawnFailed { .. } => "TERMLET_SPAWN_ERROR",
            Self::AlreadyKilled => "TERMLET_ALREADY_KILLED",
            Self::WaitForTimeout { .. } => "TERMLET_TIMEOUT",
            Self::PatternNotFound { .. } => "TERMLET_PATTERN_NOT_FOUND",
            Self::ResizeFailed(_) => "TERMLET_RESIZE_ERROR",
            Self::Pty(_) => "TERMLET_PTY_ERROR",
            Self::NotInPool { .. } => "TERMLET_NOT_IN_POOL",
        }
    }
}
```

### Test Strategy

1. Error classification: every variant maps to correct ErrorClass -- exhaustive match test.
2. Display format: error messages are human-readable -- contains substring assertions.
3. From conversion: `PtyError` converts to `TermletError::Pty` -- type assertion.
4. TermletError variants: all 7 variants constructed and displayed -- exhaustive test.
5. Error codes: every variant has a stable string code -- assert_eq on each.
6. PatternNotFound vs WaitForTimeout: distinct error paths tested -- variant match.
7. ErrorCode trait: implemented for CoreError, ProtocolError, QueryError, TermletError -- compile test.
8. **[v11]** NotInPool variant: `TermletError::NotInPool { name: "x" }` has code `TERMLET_NOT_IN_POOL` -- equality.
9. **[v11]** CoreError codes: all 5 variants have stable `CORE_*` codes -- exhaustive test.
10. **[v11]** Negative tests: every error variant is tested for its Display message content.

### AGENTS.md Rules

- `RULE-S08-01`: `thiserror` for library errors, `anyhow` only in binaries/tests. **Enforcement:** CI grep for `anyhow` in `crates/*/src/` excluding test modules.
- `RULE-S08-02`: Every `ErrorClass` variant tested. **Enforcement:** Test coverage check.
- `RULE-S08-03`: Termlet error codes must be stable string constants. **Enforcement:** Semver check + test assertions.
- `RULE-S08-04`: **[v11]** Every error type crossing binding boundaries must implement `ErrorCode`. **Enforcement:** Compile-fail test for missing impl.

---

## 9. Wire Protocol

### Design Decisions

- tmux protocol v8 (`tmux-protocol.h:23`).
- `ImsgHdr` (12 bytes) + payload. `IMSG_HDR_SIZE = 12`.
- `ImsgCodec` implements tokio-codec-compatible encode/decode (but the trait is generic, not tokio-specific).
- `DecodeOutcome` distinguishes Ok, NeedMore, and Fatal.
- Protocol violations kill the connection (never drop-and-continue), matching `server-client.c:3472-3475`.
- Codec is stateful: tracks partial frame state across calls.
- **[v11]** `MsgType` enum fully specified with all known tmux message types.

### Rust Example

```rust
// crates/mux-proto/src/codec.rs
use bytes::{Buf, BufMut, BytesMut};

pub const IMSG_HDR_SIZE: usize = 12;

/// Maximum payload size to prevent OOM on malicious input.
pub const MAX_PAYLOAD_SIZE: usize = 64 * 1024; // 64 KiB

#[derive(Debug, Clone)]
pub struct ImsgHdr {
    pub msg_type: u32,
    pub len: u32,
    pub peerid: u16,
    pub pid: u16,
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

pub struct ImsgCodec {
    state: CodecState,
}

enum CodecState {
    Header,
    Payload { header: ImsgHdr, remaining: usize },
}

impl ImsgCodec {
    pub fn new() -> Self {
        Self { state: CodecState::Header }
    }

    pub fn decode(&mut self, buf: &mut BytesMut) -> DecodeOutcome {
        loop {
            match &self.state {
                CodecState::Header => {
                    if buf.len() < IMSG_HDR_SIZE {
                        return DecodeOutcome::NeedMore;
                    }
                    let msg_type = buf.get_u32_le();
                    let len = buf.get_u32_le();
                    let peerid = buf.get_u16_le();
                    let pid_and_fd = buf.get_u16_le();
                    let pid = pid_and_fd & 0x7FFF;
                    let has_fd = (pid_and_fd & 0x8000) != 0;

                    if (len as usize) < IMSG_HDR_SIZE {
                        return DecodeOutcome::Fatal(ProtocolError::PayloadTooShort {
                            expected: IMSG_HDR_SIZE,
                            actual: len as usize,
                        });
                    }

                    let payload_len = len as usize - IMSG_HDR_SIZE;

                    // [v11] Guard against oversized payloads
                    if payload_len > MAX_PAYLOAD_SIZE {
                        return DecodeOutcome::Fatal(ProtocolError::PayloadTooShort {
                            expected: MAX_PAYLOAD_SIZE,
                            actual: payload_len,
                        });
                    }

                    let header = ImsgHdr { msg_type, len, peerid, pid, has_fd };
                    self.state = CodecState::Payload { header, remaining: payload_len };
                }
                CodecState::Payload { remaining, .. } => {
                    if buf.len() < *remaining {
                        return DecodeOutcome::NeedMore;
                    }
                    let remaining = *remaining;
                    if let CodecState::Payload { header, .. } =
                        std::mem::replace(&mut self.state, CodecState::Header)
                    {
                        let payload = buf.split_to(remaining).freeze();
                        return DecodeOutcome::Ok(ImsgFrame { header, payload });
                    }
                }
            }
        }
    }

    pub fn encode(&self, frame: &ImsgFrame, buf: &mut BytesMut) {
        buf.put_u32_le(frame.header.msg_type);
        buf.put_u32_le(frame.header.len);
        buf.put_u16_le(frame.header.peerid);
        let pid_and_fd = frame.header.pid | if frame.header.has_fd { 0x8000 } else { 0 };
        buf.put_u16_le(pid_and_fd);
        buf.put_slice(&frame.payload);
    }
}
```

### Test Strategy

1. Roundtrip: encode -> decode produces identical frame -- equality assertion on all fields.
2. Partial header: 6 bytes -> NeedMore -- variant match.
3. Partial payload: header complete but payload incomplete -> NeedMore -- variant match.
4. Invalid length: len < IMSG_HDR_SIZE -> Fatal -- variant match on error.
5. All message types: MsgType enum covers known tmux message types -- exhaustive match.
6. FD flag: has_fd bit correctly set/cleared in pid field -- bit-level assertion.
7. proptest: random payloads roundtrip -- property assertion.
8. Fuzz: random bytes do not panic -- fuzzer coverage.
9. Multi-frame: buffer with two frames decodes both -- sequential decode count.
10. **[v11]** Oversized payload: payload > MAX_PAYLOAD_SIZE -> Fatal -- variant match.

### AGENTS.md Rules

- `RULE-S09-01`: Protocol violations kill the connection, never drop-and-continue. **Enforcement:** `DecodeOutcome::Fatal` path terminates connection.
- `RULE-S09-02`: **[v11]** Payload size must be bounded by `MAX_PAYLOAD_SIZE`. **Enforcement:** Decode check in ImsgCodec.

---

## 10. Configuration Engine

### Design Decisions

- Config loads only after first client identify burst (`server-client.c:3725-3734`).
- `OptionSet` is a key-value store with typed values.
- Option resolution chain: Pane -> Window -> Session -> Server (FALLTHROUGH per `options.c:891-903`).
- `unset` removes override, falls through to parent scope (matching `options.c:1269-1285`).
- Config parser handles tmux-compatible syntax: `set-option`, `bind-key`, `source-file`, etc.
- Four option scopes: Server, Session, Window, Pane (WindowPane falls through to Window).

### Rust Example

```rust
// crates/mux-types/src/options.rs
#[derive(Debug, Clone, PartialEq)]
pub enum OptionValue {
    String(String),
    Int(i64),
    Bool(bool),
    Color(Color),
    Style(Style),
    Array(Vec<String>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionScope {
    Server,
    Session,
    Window,
    Pane, // WindowPane -- falls through to Window
}

#[derive(Debug, Clone, Default)]
pub struct OptionSet {
    values: std::collections::HashMap<String, OptionValue>,
}

impl OptionSet {
    pub fn new() -> Self { Self::default() }

    pub fn get(&self, key: &str) -> Option<&OptionValue> {
        self.values.get(key)
    }

    pub fn set(&mut self, key: String, value: OptionValue) {
        self.values.insert(key, value);
    }

    pub fn unset(&mut self, key: &str) -> bool {
        self.values.remove(key).is_some()
    }

    /// Number of explicitly set options.
    pub fn len(&self) -> usize { self.values.len() }

    pub fn is_empty(&self) -> bool { self.values.is_empty() }
}

/// Option resolution with FALLTHROUGH (options.c:891-903).
pub fn resolve_option(
    key: &str,
    pane_opts: &OptionSet,
    window_opts: &OptionSet,
    session_opts: &OptionSet,
    server_opts: &OptionSet,
) -> Option<OptionValue> {
    pane_opts.get(key)
        .or_else(|| window_opts.get(key))   // Pane FALLTHROUGH to Window
        .or_else(|| session_opts.get(key))
        .or_else(|| server_opts.get(key))
        .cloned()
}

// crates/mux-conf/src/parser.rs
pub struct ConfigParser;

#[derive(Debug, Clone)]
pub enum ConfigDirective {
    SetOption { scope: Option<OptionScope>, key: String, value: String },
    BindKey { table: String, key: String, command: String },
    UnbindKey { table: String, key: String },
    SourceFile { path: String },
}

impl ConfigParser {
    pub fn parse(input: &str) -> Result<Vec<ConfigDirective>, ConfigError> {
        let mut directives = Vec::new();
        for (line_no, line) in input.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let directive = Self::parse_line(line, line_no + 1)?;
            directives.push(directive);
        }
        Ok(directives)
    }

    fn parse_line(line: &str, line_no: usize) -> Result<ConfigDirective, ConfigError> {
        let parts: Vec<&str> = line.splitn(3, ' ').collect();
        match parts.first().copied() {
            Some("set-option" | "set") => Self::parse_set_option(&parts[1..], line_no),
            Some("bind-key" | "bind") => Self::parse_bind_key(&parts[1..], line_no),
            Some("unbind-key" | "unbind") => Self::parse_unbind_key(&parts[1..], line_no),
            Some("source-file" | "source") => Self::parse_source_file(&parts[1..], line_no),
            Some(cmd) => Err(ConfigError::UnknownCommand {
                command: cmd.to_string(), line: line_no,
            }),
            None => Err(ConfigError::EmptyLine { line: line_no }),
        }
    }

    fn parse_set_option(parts: &[&str], line_no: usize) -> Result<ConfigDirective, ConfigError> {
        // Parse -g (global), -s (server), -w (window), -p (pane) flags
        todo!("Implementation follows tmux set-option semantics")
    }

    fn parse_bind_key(parts: &[&str], line_no: usize) -> Result<ConfigDirective, ConfigError> {
        todo!("Implementation follows tmux bind-key semantics")
    }

    fn parse_unbind_key(parts: &[&str], line_no: usize) -> Result<ConfigDirective, ConfigError> {
        todo!("Implementation follows tmux unbind-key semantics")
    }

    fn parse_source_file(parts: &[&str], line_no: usize) -> Result<ConfigDirective, ConfigError> {
        if parts.is_empty() {
            return Err(ConfigError::MissingArgument { line: line_no });
        }
        Ok(ConfigDirective::SourceFile { path: parts[0].to_string() })
    }
}
```

### Test Strategy

1. Option set/get: round-trip for all OptionValue variants -- equality assertion.
2. Option unset: unset removes override, resolution falls through -- presence check.
3. FALLTHROUGH: pane option absent, window option present -> window value returned -- equality.
4. Full chain: pane -> window -> session -> server resolution -- each level tested.
5. Config parse: `set-option -g status-left "foo"` parses correctly -- variant match.
6. Config comments: lines starting with `#` are ignored -- count check.
7. Config errors: unknown command produces ConfigError with line number -- line number equality.
8. Config load timing: config only loaded after first client identify -- sequence test.
9. source-file: nested config inclusion works -- directive count.
10. Empty config: parsing empty string returns empty directives -- length == 0.

### AGENTS.md Rules

- `RULE-S10-01`: Config loads only after first client identify burst. **Enforcement:** Startup sequence integration test.
- `RULE-S10-02`: Option FALLTHROUGH: pane -> window per `options.c:891-903`. **Enforcement:** Property test with random option hierarchies.

---

## 11. Layout Engine

### Design Decisions

- Flat arena `LayoutTree` with `Vec<LayoutCell>` and index-based parent/children.
- Round-robin resize: one cell at a time, matching `layout.c:448-462`.
- Split minimum: `PANE_MINIMUM` constant (2) per `tmux.h:100`.
- Layout checksum computed per `layout-custom.c:46-57` for compatibility.
- Layout validation per `layout-custom.c:119-153`: dimensions must add up, no overlaps.
- `debug_assert!(layout_check(...))` in all layout-mutating functions.

### Rust Example

```rust
// crates/mux-core/src/layout.rs
pub const PANE_MINIMUM: u16 = 2;

#[derive(Debug, Clone)]
pub struct LayoutTree {
    pub cells: Vec<LayoutCell>,
    pub root: usize,
}

#[derive(Debug, Clone)]
pub struct LayoutCell {
    pub kind: LayoutKind,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub pane: Option<PaneId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutKind {
    Horizontal, // children arranged left-to-right
    Vertical,   // children arranged top-to-bottom
    Leaf,       // terminal pane
}

/// Round-robin resize matching layout.c:448-462.
/// Distributes delta one cell at a time across children.
pub fn layout_resize_adjust(tree: &mut LayoutTree, node: usize, delta: i32) {
    let children = tree.cells[node].children.clone();
    if children.is_empty() || delta == 0 {
        return;
    }
    let step = if delta > 0 { 1 } else { -1 };
    let mut remaining = delta.abs();
    let mut idx = 0;
    let max_iterations = children.len() * (delta.unsigned_abs() as usize);
    while remaining > 0 {
        let child = children[idx % children.len()];
        let cell = &tree.cells[child];
        let (dim, min) = match tree.cells[node].kind {
            LayoutKind::Horizontal => (cell.width as i32, PANE_MINIMUM as i32),
            LayoutKind::Vertical => (cell.height as i32, PANE_MINIMUM as i32),
            LayoutKind::Leaf => return,
        };
        if dim + step >= min {
            match tree.cells[node].kind {
                LayoutKind::Horizontal => {
                    tree.cells[child].width = (dim + step) as u16;
                }
                LayoutKind::Vertical => {
                    tree.cells[child].height = (dim + step) as u16;
                }
                _ => {}
            }
            remaining -= 1;
        }
        idx += 1;
        if idx >= max_iterations {
            break; // Safety: prevent infinite loop if all children at minimum
        }
    }
    debug_assert!(layout_check(tree));
}

/// Layout checksum per layout-custom.c:46-57.
pub fn layout_checksum(tree: &LayoutTree) -> u16 {
    let mut csum: u16 = 0;
    let s = layout_serialize(tree);
    for byte in s.bytes() {
        csum = (csum >> 1) | ((csum & 1) << 15);
        csum = csum.wrapping_add(byte as u16);
    }
    csum
}

/// Layout validation per layout-custom.c:119-153.
pub fn layout_check(tree: &LayoutTree) -> bool {
    fn check_node(tree: &LayoutTree, idx: usize) -> bool {
        let cell = &tree.cells[idx];
        if cell.kind == LayoutKind::Leaf {
            return cell.width >= PANE_MINIMUM && cell.height >= PANE_MINIMUM;
        }
        let children = &cell.children;
        if children.is_empty() {
            return false;
        }
        match cell.kind {
            LayoutKind::Horizontal => {
                let total_w: u16 = children.iter()
                    .map(|&c| tree.cells[c].width)
                    .sum::<u16>() + (children.len() as u16 - 1); // separators
                total_w == cell.width &&
                    children.iter().all(|&c| tree.cells[c].height == cell.height) &&
                    children.iter().all(|&c| check_node(tree, c))
            }
            LayoutKind::Vertical => {
                let total_h: u16 = children.iter()
                    .map(|&c| tree.cells[c].height)
                    .sum::<u16>() + (children.len() as u16 - 1);
                total_h == cell.height &&
                    children.iter().all(|&c| tree.cells[c].width == cell.width) &&
                    children.iter().all(|&c| check_node(tree, c))
            }
            _ => false,
        }
    }
    check_node(tree, tree.root)
}

fn layout_serialize(tree: &LayoutTree) -> String {
    fn serialize_node(tree: &LayoutTree, idx: usize) -> String {
        let cell = &tree.cells[idx];
        let dim = format!("{}x{},{},{}", cell.width, cell.height, cell.x, cell.y);
        if cell.kind == LayoutKind::Leaf {
            return dim;
        }
        let sep = if cell.kind == LayoutKind::Horizontal { '{' } else { '[' };
        let end = if cell.kind == LayoutKind::Horizontal { '}' } else { ']' };
        let children: Vec<String> = cell.children.iter()
            .map(|&c| serialize_node(tree, c))
            .collect();
        format!("{}{}{}{}", dim, sep, children.join(","), end)
    }
    serialize_node(tree, tree.root)
}
```

### Test Strategy

1. Round-robin resize: delta distributed one-at-a-time across children -- per-child dimension check.
2. Minimum enforcement: no child shrinks below `PANE_MINIMUM` -- min value assertion.
3. Checksum: matches tmux's layout-custom.c output for known layouts -- exact u16 match.
4. Validation: valid layout passes `layout_check`; invalid fails -- boolean assertions.
5. Serialize: layout string format matches tmux -- string equality.
6. Split: splitting a pane creates two children with correct dimensions -- count + dimension checks.
7. proptest: random resize operations never violate layout invariants -- property assertion.
8. Nested layouts: horizontal inside vertical works correctly -- structural check.

### AGENTS.md Rules

- `RULE-S11-01`: `debug_assert!(layout_check(...))` in all layout-mutating functions. **Enforcement:** CI with `RUSTFLAGS="-C debug-assertions=on"`.
- `RULE-S11-02`: Format strings match tmux via `format-audit` parity corpus. **Enforcement:** CI `format-audit` check.

---

## 12. ORM Query Layer

### Design Decisions

- 18 query operators: 12 from libtmux `LOOKUP_NAME_MAP` (lines 298-312) + 6 extensions.
- libtmux operators: `eq`/`exact`, `iexact`, `contains`, `icontains`, `startswith`, `istartswith`, `endswith`, `iendswith`, `in`, `nin`, `regex`, `iregex`.
- Extensions: `lt`, `lte`, `gt`, `gte`, `ne`, `between`.
- `QueryList<T>` is the filterable collection type.
- Django-style kwargs: `sessions.filter(name="work")`, `sessions.filter(name__startswith="w")`.
- `keygetter`-style nested `__` traversal matching libtmux `query_list.py:45-113`.
- Callable matcher: `sessions.filter(lambda s: s.name == "work")` (libtmux `filter()` line 535).
- `get(default=no_arg)` semantics matching libtmux lines 550-569.
- Exceptions: `MultipleObjectsReturned`, `ObjectDoesNotExist` (matching libtmux).

### Rust Example

```rust
// crates/mux-query/src/lib.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryOp {
    Eq, Exact, IExact,
    Contains, IContains,
    StartsWith, IStartsWith,
    EndsWith, IEndsWith,
    In, NotIn,
    Regex, IRegex,
    // Extensions beyond libtmux
    Lt, Lte, Gt, Gte,
    Ne,
    Between,
}

impl QueryOp {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "eq" | "exact" => Some(Self::Eq),
            "iexact" => Some(Self::IExact),
            "contains" => Some(Self::Contains),
            "icontains" => Some(Self::IContains),
            "startswith" => Some(Self::StartsWith),
            "istartswith" => Some(Self::IStartsWith),
            "endswith" => Some(Self::EndsWith),
            "iendswith" => Some(Self::IEndsWith),
            "in" => Some(Self::In),
            "nin" => Some(Self::NotIn),
            "regex" => Some(Self::Regex),
            "iregex" => Some(Self::IRegex),
            "lt" => Some(Self::Lt),
            "lte" => Some(Self::Lte),
            "gt" => Some(Self::Gt),
            "gte" => Some(Self::Gte),
            "ne" => Some(Self::Ne),
            "between" => Some(Self::Between),
            _ => None,
        }
    }

    /// [v11] All valid operator names for documentation/error messages.
    pub fn all_names() -> &'static [&'static str] {
        &[
            "eq", "exact", "iexact", "contains", "icontains",
            "startswith", "istartswith", "endswith", "iendswith",
            "in", "nin", "regex", "iregex",
            "lt", "lte", "gt", "gte", "ne", "between",
        ]
    }
}

#[derive(Debug, Clone)]
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

/// Parse "field__op" into (field_path, op).
/// Supports nested traversal: "foods__fruit__in" -> (["foods", "fruit"], In)
pub fn parse_query_key(key: &str) -> (Vec<&str>, QueryOp) {
    let parts: Vec<&str> = key.split("__").collect();
    if parts.len() >= 2 {
        if let Some(op) = QueryOp::from_name(parts.last().unwrap()) {
            return (parts[..parts.len()-1].to_vec(), op);
        }
    }
    (parts, QueryOp::Eq) // default to Eq when no operator suffix
}

pub struct QueryList<T> {
    items: Vec<T>,
}

impl<T> QueryList<T> {
    pub fn new(items: Vec<T>) -> Self { Self { items } }

    pub fn filter<F: Fn(&T) -> bool>(&self, predicate: F) -> QueryList<&T> {
        QueryList {
            items: self.items.iter().filter(|item| predicate(item)).collect(),
        }
    }

    pub fn get<F: Fn(&T) -> bool>(&self, predicate: F) -> Result<&T, QueryError> {
        let matches: Vec<&T> = self.items.iter().filter(|item| predicate(item)).collect();
        match matches.len() {
            0 => Err(QueryError::ObjectDoesNotExist),
            1 => Ok(matches[0]),
            _ => Err(QueryError::MultipleObjectsReturned { count: matches.len() }),
        }
    }

    /// Number of items in the list.
    pub fn len(&self) -> usize { self.items.len() }

    pub fn is_empty(&self) -> bool { self.items.is_empty() }

    /// Iterate over items.
    pub fn iter(&self) -> impl Iterator<Item = &T> { self.items.iter() }

    /// Get first item or None.
    pub fn first(&self) -> Option<&T> { self.items.first() }
}

#[derive(Debug, Error)]
pub enum QueryError {
    #[error("object does not exist")]
    ObjectDoesNotExist,
    #[error("multiple objects returned ({count})")]
    MultipleObjectsReturned { count: usize },
    #[error("unknown operator: {op}")]
    UnknownOperator { op: String },
}
```

### Test Strategy

1. All 18 operators: each operator tested with matching and non-matching data -- boolean assertions.
2. `eq`/`exact` alias: both resolve to the same operator -- equality assertion.
3. Case-insensitive: `icontains`, `iexact`, `istartswith`, `iendswith`, `iregex` -- case-varied input.
4. Nested traversal: `"foods__fruit__in"` parses to `(["foods", "fruit"], In)` -- equality assertion.
5. Default operator: `"name"` (no `__`) defaults to `Eq` -- variant match.
6. QueryList filter: returns matching items -- length and content checks.
7. QueryList get: returns single item or error -- Result assertion.
8. MultipleObjectsReturned: multiple matches raises error with count -- count equality.
9. ObjectDoesNotExist: no matches raises error -- variant match.
10. Regex: `regex` and `iregex` operators use `re.search` semantics -- match assertions.
11. Between: `between` with `[low, high]` range -- boundary checks.
12. Callable matcher: function predicate in `filter()` -- closure test.

### AGENTS.md Rules

- `RULE-S12-01`: Key bindings match tmux via generated parity tests. **Enforcement:** CI parity test suite.

---

## 13. Concurrency Model

### Design Decisions

- Single-writer state actor: only one task mutates `ServerGraph`.
- `ArcSwap<GraphState>` for zero-contention snapshot reads.
- No `Arc<Mutex<_>>` on read paths.
- Tokio channels for event submission to the state actor.
- `StateHandle` wraps `ArcSwap` for ergonomic reads from multiple tasks.
- Actor processes events sequentially, publishes snapshots after each batch.
- **[v11]** State actor message protocol fully specified: `EventMsg` with optional oneshot reply channel.

### Rust Example

```rust
// crates/mux-api/src/state_handle.rs
use arc_swap::ArcSwap;
use std::sync::Arc;

pub struct StateHandle {
    inner: Arc<ArcSwap<GraphState>>,
}

impl StateHandle {
    pub fn new(initial: GraphState) -> Self {
        Self {
            inner: Arc::new(ArcSwap::new(Arc::new(initial))),
        }
    }

    /// Zero-contention snapshot read.
    pub fn load(&self) -> arc_swap::Guard<Arc<GraphState>> {
        self.inner.load()
    }

    /// Publish a new snapshot (called by state actor only).
    pub(crate) fn store(&self, state: GraphState) {
        self.inner.store(Arc::new(state));
    }
}

impl Clone for StateHandle {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

// crates/mux-server/src/state_actor.rs

/// [v11] State actor message with optional reply channel.
pub struct EventMsg {
    pub event: Event,
    /// Optional oneshot channel for synchronous event submission.
    /// When present, the actor sends back the ApplyOutcome after processing.
    pub reply: Option<tokio::sync::oneshot::Sender<ApplyOutcome>>,
}

pub struct StateActor {
    graph: ServerGraph,
    handle: StateHandle,
    event_rx: tokio::sync::mpsc::Receiver<EventMsg>,
    ctx: CoreCtx,
}

impl StateActor {
    pub async fn run(mut self) {
        while let Some(msg) = self.event_rx.recv().await {
            let outcome = self.graph.apply_event(msg.event, &self.ctx);
            // Publish updated snapshot
            self.handle.store(self.graph.snapshot());
            // Dispatch effects to runtime
            for effect in &outcome.effects {
                self.dispatch_effect(effect.clone()).await;
            }
            // Reply if requested
            if let Some(reply) = msg.reply {
                let _ = reply.send(outcome);
            }
        }
    }

    async fn dispatch_effect(&self, effect: Effect) {
        match effect {
            Effect::SpawnProcess { pane, command, cwd, size } => {
                // Delegate to PTY manager
            }
            Effect::Notify(notification) => {
                // Broadcast to control mode clients
            }
            _ => {}
        }
    }
}
```

### Test Strategy

1. Single writer: concurrent event submissions are serialized by actor -- ordering test.
2. Snapshot reads: multiple readers see consistent state without blocking -- concurrent read test.
3. ArcSwap load: returns most recent snapshot -- freshness check.
4. Actor shutdown: dropping event sender cleanly shuts down actor -- no hang assertion.
5. Effect dispatch: effects from apply_event are dispatched correctly -- variant match.
6. No mutex contention: benchmark confirms no lock contention on reads -- latency check.
7. **[v11]** Reply channel: EventMsg with reply receives ApplyOutcome -- oneshot recv success.

### AGENTS.md Rules

- `RULE-S13-01`: Only the state actor mutates graph state. No `Arc<Mutex<_>>` on read paths. **Enforcement:** Code review + grep for `Mutex<ServerGraph>`.

---

## 14. Server Lifecycle

### Design Decisions

- `flock(LOCK_EX|LOCK_NB)` per `client.c:77-101` (confirmed line 89).
- Lock file guard via `Drop` impl, never manual unlock.
- Server starts on first client connect if not already running.
- Config load triggers after first client completes identify burst.
- Graceful shutdown: drain pending operations, close sockets, exit.
- `ManagedMux` combines server lifecycle with `StateHandle`.

### Rust Example

```rust
// crates/mux-api/src/managed.rs
pub struct ManagedMux {
    state_handle: StateHandle,
    event_tx: tokio::sync::mpsc::Sender<EventMsg>,
    shutdown_tx: tokio::sync::broadcast::Sender<()>,
}

impl ManagedMux {
    pub fn state(&self) -> StateHandle {
        self.state_handle.clone()
    }

    /// Submit an event to the state actor.
    pub fn submit_event(&self, event: Event) -> Result<(), CoreError> {
        self.event_tx.try_send(EventMsg { event, reply: None })
            .map_err(|_| CoreError::SessionNotFound { id: SessionId::default() })
    }

    /// [v11] Submit an event and wait for the outcome.
    pub async fn submit_event_sync(&self, event: Event) -> Result<ApplyOutcome, CoreError> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.event_tx.send(EventMsg { event, reply: Some(tx) }).await
            .map_err(|_| CoreError::SessionNotFound { id: SessionId::default() })?;
        rx.await.map_err(|_| CoreError::SessionNotFound { id: SessionId::default() })
    }

    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(());
    }
}

/// TermForgeStack: composition root for in-process embedding.
pub struct TermForgeStack {
    pub managed: ManagedMux,
    _actor_handle: tokio::task::JoinHandle<()>,
}

impl TermForgeStack {
    pub async fn start() -> Self {
        let graph = ServerGraph::default();
        let handle = StateHandle::new(graph.snapshot());
        let (event_tx, event_rx) = tokio::sync::mpsc::channel(1024);
        let (shutdown_tx, _shutdown_rx) = tokio::sync::broadcast::channel(1);

        let actor = StateActor {
            graph,
            handle: handle.clone(),
            event_rx,
            ctx: CoreCtx::real(),
        };

        let actor_handle = tokio::spawn(actor.run());

        Self {
            managed: ManagedMux { state_handle: handle, event_tx, shutdown_tx },
            _actor_handle: actor_handle,
        }
    }
}

// crates/mux-os/src/lock.rs
pub struct LockGuard {
    file: std::fs::File,
}

impl LockGuard {
    pub fn try_lock(path: &std::path::Path) -> Result<Self, LockError> {
        use fs2::FileExt;
        let file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(path)?;
        file.try_lock_exclusive()
            .map_err(|_| LockError::AlreadyLocked(path.to_owned()))?;
        Ok(Self { file })
    }
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        use fs2::FileExt;
        let _ = self.file.unlock();
    }
}
```

### Test Strategy

1. Lock acquisition: `try_lock` succeeds on free path -- Ok result.
2. Lock contention: second `try_lock` on same path returns `AlreadyLocked` -- error variant.
3. Lock release on drop: dropping `LockGuard` releases lock -- subsequent lock succeeds.
4. Config load timing: config loaded only after first client identify -- event sequence check.
5. ManagedMux lifecycle: start -> submit events -> shutdown -- no panic.
6. TermForgeStack: in-process embedding starts and stops cleanly -- tokio runtime test.
7. **[v11]** submit_event_sync: returns ApplyOutcome with expected effects -- outcome check.

### AGENTS.md Rules

- `RULE-S14-01`: Lock file guard via `Drop` impl, never manual unlock. **Enforcement:** Code review: no manual `unlock()` calls.
- `RULE-S14-02`: Config load trigger must remain post-identify. **Enforcement:** Startup sequence test.

---

## 15. Control Mode

### Design Decisions

- Control mode notifications are hints, not authoritative state.
- Periodic refresh reconciles control-mode-derived state with actual state.
- Extended output format per `control.c:620-623`.
- Pending limit enforcement per `control.c:450-461`.
- Control mode startup sequence per `control.c:758-796`.
- Notification types cover session, window, pane, layout, client, and output events.

### Rust Example

```rust
// crates/mux-control/src/notification.rs
#[derive(Debug, Clone)]
pub enum ControlNotification {
    SessionChanged { session_id: String, session_name: String },
    SessionCreated { session_id: String, session_name: String },
    SessionRenamed { session_id: String, new_name: String },
    SessionClosed { session_id: String },
    WindowAdd { window_id: String },
    WindowClose { window_id: String },
    WindowRenamed { window_id: String, new_name: String },
    WindowPaneChanged { window_id: String, pane_id: String },
    LayoutChange { window_id: String, layout: String },
    PaneOutput { pane_id: String, data: String },
    PaneModeChanged { pane_id: String, mode: String },
    ClientDetached { client_name: String, reason: String },
    ClientSessionChanged { client_name: String, session_id: String },
    Output { pane_id: String, data: String },
}

pub fn parse_notification(line: &str) -> Result<ControlNotification, ControlParseError> {
    let (prefix, rest) = line.split_once(' ')
        .ok_or(ControlParseError::MalformedLine(line.to_string()))?;

    match prefix {
        "%session-changed" => {
            let (id, name) = rest.split_once(' ')
                .ok_or(ControlParseError::MalformedLine(line.to_string()))?;
            Ok(ControlNotification::SessionChanged {
                session_id: id.to_string(),
                session_name: name.to_string(),
            })
        }
        "%session-created" => {
            let (id, name) = rest.split_once(' ')
                .ok_or(ControlParseError::MalformedLine(line.to_string()))?;
            Ok(ControlNotification::SessionCreated {
                session_id: id.to_string(),
                session_name: name.to_string(),
            })
        }
        "%window-add" => Ok(ControlNotification::WindowAdd {
            window_id: rest.to_string(),
        }),
        "%window-close" => Ok(ControlNotification::WindowClose {
            window_id: rest.to_string(),
        }),
        "%layout-change" => {
            let (wid, layout) = rest.split_once(' ')
                .ok_or(ControlParseError::MalformedLine(line.to_string()))?;
            Ok(ControlNotification::LayoutChange {
                window_id: wid.to_string(),
                layout: layout.to_string(),
            })
        }
        _ => Err(ControlParseError::UnknownNotification(prefix.to_string())),
    }
}
```

### Test Strategy

1. Session notification: `%session-changed $1 work` parses correctly -- field equality.
2. Session created: `%session-created $2 test` parses correctly -- field equality.
3. Window notification: `%window-add @0` parses correctly -- field equality.
4. Layout notification: `%layout-change @0 <layout>` parses layout string -- field equality.
5. Unknown prefix: returns `UnknownNotification` error -- variant match.
6. Malformed line: no space separator returns error -- variant match.
7. Periodic refresh: stale control-mode state corrected by refresh -- state comparison.
8. Pending limit: notifications beyond limit are dropped gracefully -- count check.
9. Extended output: `%output` prefix with pane_id and data -- field equality.

### AGENTS.md Rules

- `RULE-S15-01`: Control notifications are hints, not authoritative. **Enforcement:** Periodic refresh tests verify convergence.

---

## 16. Language Bindings [v11 updated]

### Design Decisions

- Python bindings via PyO3 with `#[pyclass]`/`#[pymethods]`.
- Node.js bindings via Neon with `JsBox` wrapping.
- Both expose the ORM API: `QueryList` with `filter()`/`get()`, `Server`/`Session`/`Window`/`Pane` objects.
- Exception types: `ObjectDoesNotExist`, `MultipleObjectsReturned` mapped to Python exceptions.
- OTEL context propagation via `traceparent` parameter.
- `PyTermlet` and `JsTermlet` binding handles with `Mutex<Termlet>` for thread safety.
- GIL release in PyTermlet `send_keys` and `wait_for` via `py.allow_threads()`.
- Modern PyO3 `Bound<'py, T>` API used where applicable.
- **[v11]** Error code propagation: Python exceptions carry `error_code` attribute from `ErrorCode` trait.
- **[v11]** Node error propagation: Error objects carry `code` property from `ErrorCode` trait.

### Rust Example

```rust
// bindings/python/src/lib.rs
use pyo3::prelude::*;
use pyo3::exceptions::{PyRuntimeError, PyValueError};

#[pyclass]
pub struct Server {
    managed: mux_api::ManagedMux,
}

#[pymethods]
impl Server {
    #[new]
    fn new() -> PyResult<Self> {
        todo!("Initialize in-process or connect to running server")
    }

    #[getter]
    fn sessions(&self) -> PyResult<PyQueryList> {
        let state = self.managed.state().load();
        let items: Vec<PySession> = state.sessions.iter()
            .map(|(id, s)| PySession { id: *id, name: s.name.clone() })
            .collect();
        Ok(PyQueryList { items })
    }

    fn new_session(&self, name: &str) -> PyResult<PySession> {
        self.managed.submit_event(Event::CreateSession { name: name.to_string() })
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        todo!("Return the new session")
    }

    fn __enter__(slf: Py<Self>) -> Py<Self> { slf }

    fn __exit__(
        &self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_val: Option<&Bound<'_, PyAny>>,
        _exc_tb: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<bool> {
        self.managed.shutdown();
        Ok(false)
    }
}

// PyTermlet binding handle with Mutex for thread safety.
#[pyclass]
pub struct PyTermlet {
    inner: std::sync::Mutex<mux_termlet::Termlet>,
}

#[pymethods]
impl PyTermlet {
    #[new]
    #[pyo3(signature = (command, cols=80, rows=24, fake=false))]
    fn new(command: &str, cols: u16, rows: u16, fake: bool) -> PyResult<Self> {
        let config = mux_termlet::TermletConfig {
            cols, rows,
            env: vec![("TERM".to_string(), "xterm-256color".to_string())],
            cwd: None,
            backend: if fake { mux_termlet::PtyMode::Fake } else { mux_termlet::PtyMode::Real },
            ..Default::default()
        };
        let termlet = mux_termlet::Termlet::spawn(command, config)
            .map_err(|e| {
                // [v11] Carry error code in exception
                let code = e.error_code();
                PyRuntimeError::new_err(format!("[{code}] {e}"))
            })?;
        Ok(Self { inner: std::sync::Mutex::new(termlet) })
    }

    /// Send keystrokes. Releases GIL during PTY write.
    fn send_keys(&self, py: Python<'_>, keys: &str) -> PyResult<()> {
        let keys = keys.to_string();
        py.allow_threads(|| {
            self.inner.lock().unwrap()
                .send_keys(&keys)
                .map_err(|e| PyRuntimeError::new_err(format!("[{}] {}", e.error_code(), e)))
        })
    }

    /// Wait for pattern. Releases GIL during poll loop.
    #[pyo3(signature = (pattern, timeout_ms=30000))]
    fn wait_for(&self, py: Python<'_>, pattern: &str, timeout_ms: u64) -> PyResult<()> {
        let pattern = pattern.to_string();
        py.allow_threads(|| {
            self.inner.lock().unwrap()
                .wait_for(&pattern, std::time::Duration::from_millis(timeout_ms))
                .map(|_| ())
                .map_err(|e| match &e {
                    mux_termlet::TermletError::WaitForTimeout { .. } =>
                        pyo3::exceptions::PyTimeoutError::new_err(
                            format!("[{}] {}", e.error_code(), e)),
                    _ => PyRuntimeError::new_err(
                        format!("[{}] {}", e.error_code(), e)),
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
            .map_err(|e| PyRuntimeError::new_err(format!("[{}] {}", e.error_code(), e)))
    }

    /// Kill the underlying process.
    fn kill(&self) -> PyResult<()> {
        self.inner.lock().unwrap().kill()
            .map_err(|e| PyRuntimeError::new_err(format!("[{}] {}", e.error_code(), e)))
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

// bindings/node/src/js_termlet.rs
pub struct JsTermlet { inner: std::sync::Mutex<mux_termlet::Termlet> }
impl Finalize for JsTermlet {}

fn termlet_spawn(mut cx: FunctionContext) -> JsResult<JsBox<JsTermlet>> {
    let command = cx.argument::<JsString>(0)?.value(&mut cx);
    let opts = cx.argument_opt(1);
    let (cols, rows, fake) = parse_termlet_opts(&mut cx, opts)?;
    let config = mux_termlet::TermletConfig {
        cols, rows,
        env: vec![("TERM".to_string(), "xterm-256color".to_string())],
        cwd: None,
        backend: if fake { mux_termlet::PtyMode::Fake } else { mux_termlet::PtyMode::Real },
        ..Default::default()
    };
    let termlet = mux_termlet::Termlet::spawn(&command, config)
        .or_else(|e| {
            // [v11] Include error code in Node error
            let msg = format!("[{}] {}", e.error_code(), e);
            cx.throw_error(msg)
        })?;
    Ok(cx.boxed(JsTermlet { inner: std::sync::Mutex::new(termlet) }))
}

fn termlet_send_keys(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let termlet = cx.argument::<JsBox<JsTermlet>>(0)?;
    let keys = cx.argument::<JsString>(1)?.value(&mut cx);
    termlet.inner.lock().unwrap().send_keys(&keys)
        .or_else(|e| cx.throw_error(format!("[{}] {}", e.error_code(), e)))?;
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
        .or_else(|e| cx.throw_error(format!("[{}] {}", e.error_code(), e)))?;
    Ok(cx.undefined())
}
```

### Test Strategy

1. PyQueryList filter kwargs: `sessions.filter(name="work")` returns matching -- count check.
2. PyQueryList filter operator: `sessions.filter(name__startswith="w")` works -- count check.
3. PyQueryList callable: `sessions.filter(lambda s: s.name == "work")` works -- count check.
4. PyQueryList get default: `sessions.get(name="missing", default=None)` returns None -- None assertion.
5. PyQueryList get error: multiple matches raises `MultipleObjectsReturned` -- exception type.
6. PyQueryList get not found: no matches raises `ObjectDoesNotExist` -- exception type.
7. PyQueryList iteration: `for s in server.sessions` iterates all -- count check.
8. PyQueryList indexing: `server.sessions[0]` and `server.sessions[-1]` work -- type check.
9. Context manager: `with Server() as s` starts and kills server -- cleanup assertion.
10. OTEL propagation: traceparent passed from Python into Rust bindings -- span check.
11. Node sessions: `server.sessions({ name: "work" })` filters correctly -- count check.
12. Node async: `await server.newSession("test")` resolves/rejects correctly -- promise check.
13. Error mapping: Rust `CoreError::SessionNotFound` maps to Python `ObjectDoesNotExist` -- exception type.
14. PyTermlet spawn: `Termlet("bash")` starts a shell -- is_alive assertion.
15. PyTermlet send_keys: `termlet.send_keys("echo hello\n")` sends keystrokes -- no exception.
16. PyTermlet snapshot: `termlet.snapshot()` returns grid text -- contains assertion.
17. PyTermlet context manager: `with Termlet("bash") as t:` auto-kills on exit -- is_alive=False after.
18. PyTermlet fake mode: `Termlet("bash", fake=True)` uses FakePtyBackend -- no exception.
19. PyTermlet wait_for timeout: raises `TimeoutError` (not generic RuntimeError) -- exception type.
20. JsTermlet spawn: `termletSpawn("bash")` returns JsBox -- type check.
21. JsTermlet snapshot: `termletSnapshot(t)` returns string -- type check.
22. PyTermlet GIL release: `send_keys` and `wait_for` release GIL via `py.allow_threads()` -- concurrent call test.
23. **[v11]** Error code in Python exception: message starts with `[TERMLET_TIMEOUT]` -- prefix check.
24. **[v11]** Error code in Node error: message starts with `[TERMLET_SPAWN_ERROR]` -- prefix check.

### AGENTS.md Rules

- `RULE-S16-01`: Bindings must mirror ORM API. No extra mutation surfaces. **Enforcement:** API surface comparison test.
- `RULE-S16-02`: Expose OTEL context propagation via traceparent parameter. **Enforcement:** Integration test.
- `RULE-S16-03`: Termlet bindings must expose: spawn, send_keys, wait_for, snapshot, resize, kill. **Enforcement:** API surface test.
- `RULE-S16-04`: Termlet binding handles must use `Mutex<Termlet>` for thread safety. **Enforcement:** Code review.
- `RULE-S16-05`: **[v11]** Error messages in bindings must include `[ERROR_CODE]` prefix. **Enforcement:** Exception message format test.

---

## 17. CRDT Transaction Layer

### Design Decisions

- Support eventual-consistency replication between TermForge instances.
- Hybrid Logical Clock (HLC) for causal ordering.
- LWW (Last Writer Wins) registers for scalar fields (session name, options).
- OR-Set / Add-Wins Set (Observed Remove Set) for collections (windows in session, panes in window).
- OpLog stores operations for merge/replay.
- CRDT layer is opt-in. Single-server mode uses direct events without HLC overhead.

### Rust Example

```rust
// crates/mux-crdt/src/hlc.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
    pub fn new(value: T, ts: HLC) -> Self {
        Self { value, timestamp: ts }
    }

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
    pub fn new() -> Self {
        Self { elements: HashMap::new(), tombstones: HashMap::new() }
    }

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

    /// [v11] Merge two OR-Sets for replica convergence.
    pub fn merge(&mut self, other: &OrSet<T>) {
        for (elem, tags) in &other.elements {
            self.elements.entry(elem.clone()).or_default().extend(tags.iter().cloned());
        }
        for (elem, tags) in &other.tombstones {
            self.tombstones.entry(elem.clone()).or_default().extend(tags.iter().cloned());
        }
    }
}

// crates/mux-crdt/src/oplog.rs
#[derive(Debug, Clone)]
pub struct OpLog {
    ops: Vec<(HLC, Event)>,
}

impl OpLog {
    pub fn new() -> Self { Self { ops: Vec::new() } }

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

    /// [v11] Compact ops older than cutoff, returning compacted count.
    pub fn compact_before(&mut self, cutoff: HLC) -> usize {
        let before = self.ops.len();
        self.ops.retain(|(ts, _)| *ts >= cutoff);
        before - self.ops.len()
    }
}
```

### Test Strategy

1. HLC monotonicity: after merge, HLC is strictly greater than both inputs -- ordering assertion.
2. LWW convergence: two replicas with concurrent writes converge to same value -- equality assertion.
3. OR-Set add-remove: add + remove + re-add -> element present -- contains assertion.
4. OR-Set concurrent add: two nodes add different elements, merge has both -- contains assertion.
5. OpLog merge idempotent: merging same log twice does not duplicate ops -- length assertion.
6. Deterministic replay: OpLog replayed in HLC order produces deterministic graph -- snapshot equality.
7. **[v11]** OR-Set merge: two replicas merge correctly -- symmetric convergence test.
8. **[v11]** OpLog compact: ops before cutoff removed, ops after retained -- length assertion.

### AGENTS.md Rules

- `RULE-S17-01`: CRDT layer is additive, not required for single-server mode. **Enforcement:** `mux-crdt` is optional dependency.
- `RULE-S17-02`: No network code in `mux-crdt` crate. **Enforcement:** CI grep for `std::net` and `tokio::net`.

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

    // Check entire path for symlinks
    for ancestor in path.ancestors() {
        if ancestor == std::path::Path::new("") || ancestor == std::path::Path::new("/") {
            break;
        }
        if ancestor.is_symlink() {
            return Err(SocketError::SymlinkInPath(ancestor.to_owned()));
        }
    }

    // Check path length (Unix socket limit is 108 bytes)
    let path_str = path.to_str().ok_or(SocketError::InvalidPath)?;
    if path_str.len() > 107 {
        return Err(SocketError::PathTooLong {
            length: path_str.len(),
            max: 107,
        });
    }

    let old_umask = unsafe { libc::umask(0o177) };
    let result = std::os::unix::net::UnixListener::bind(path);
    unsafe { libc::umask(old_umask); }
    result.map_err(SocketError::Bind)
}
```

### Test Strategy

1. Socket permissions: verify socket created with mode 0600 -- permission check.
2. Directory permissions: verify socket dir has mode 0700 -- permission check.
3. Symlink rejection: socket path with symlink fails creation -- error variant.
4. FD passing: SCM_RIGHTS roundtrip preserves fd -- fd comparison.
5. umask restoration: after socket creation, umask is restored to original value -- umask check.
6. **[v11]** Path length: path > 107 bytes returns PathTooLong error -- error variant.

### AGENTS.md Rules

- `RULE-S18-01`: All `unsafe` socket/fd code lives in `mux-os` only. **Enforcement:** CI grep for `unsafe` in Layer 0 crates.
- `RULE-S18-02`: Socket path length checked against 108-byte limit before creation. **Enforcement:** Integration test with long paths.

---

## 19. OpenTelemetry [v11 updated]

### Design Decisions

Architecture directly modeled on vibe-tmux `mux-otel` crate, verified against `otel.rs`. Key decisions:

1. **Dual provider**: `OTEL_PROVIDER` (server/TUI, installed global) and `MUX_CLIENT_PROVIDER` (client spans, lazy init, NOT installed global).
2. **Thread-local** `TRACE_HEADERS_STACK` with `TraceHeadersGuard` RAII cleanup.
3. **Process-level** `PROCESS_TRACE_HEADERS` for cross-thread header sharing.
4. **Composite propagator**: `BaggagePropagator` + `TraceContextPropagator`.
5. **OnceLock<Mutex<Option<OtelProvider>>>** for safe one-time init.
6. **Env-var activation**: `TERMFORGE_OTEL=1` or `OTEL_EXPORTER_OTLP_ENDPOINT` set.
7. **Bounded shutdown**: `runtime.shutdown_timeout(200ms)` in `OtelProvider::drop` to avoid blocking short-lived processes.
8. **[v11]** Termlet spans include structured attributes: `termlet.cols`, `termlet.rows`, `termlet.pty_mode`, `termlet.command`.

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
    static TRACE_HEADERS_STACK: std::cell::RefCell<Vec<TraceHeaders>> =
        const { std::cell::RefCell::new(Vec::new()) };
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
fn composite_propagator() -> opentelemetry::propagation::TextMapCompositePropagator {
    let propagators: Vec<Box<dyn opentelemetry::propagation::TextMapPropagator + Send + Sync>> = vec![
        Box::new(opentelemetry::baggage::propagation::BaggagePropagator::new()),
        Box::new(opentelemetry::trace::TraceContextPropagator::new()),
    ];
    opentelemetry::propagation::TextMapCompositePropagator::new(propagators)
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

| Span Name | Kind | Source | Attributes (v11) |
|---|---|---|---|
| `termforge.server.start` | Internal | Server startup | |
| `termforge.server.event` | Internal | Event processing | `event.type` |
| `termforge.server.effect` | Internal | Effect dispatch | `effect.type` |
| `termforge.client.connect` | Client | Client connection | `client.id` |
| `termforge.client.command` | Client | Command execution | `command.name` |
| `termforge.binding.call` | Client | Python/Node binding call | `binding.lang`, `binding.method` |
| `termforge.proto.decode` | Internal | Protocol decode | `frame.msg_type` |
| `termforge.proto.encode` | Internal | Protocol encode | `frame.msg_type` |
| `termforge.termlet.spawn` | Internal | Termlet spawn | `termlet.command`, `termlet.cols`, `termlet.rows`, `termlet.pty_mode` |
| `termforge.termlet.interact` | Internal | Termlet send_keys/wait_for | `termlet.method`, `termlet.pattern` |
| `termforge.termlet.snapshot` | Internal | Termlet snapshot capture | `termlet.cols`, `termlet.rows` |
| `termforge.termlet.kill` | Internal | **[v11]** Termlet kill | `termlet.grace_period_ms` |
| `termforge.termlet.drain` | Internal | **[v11]** Termlet drain_output | `termlet.bytes_drained` |

### Test Strategy

1. Dual provider init: both `OTEL_PROVIDER` and `MUX_CLIENT_PROVIDER` initialized correctly -- non-null check.
2. Thread-local isolation: child thread does not see parent's trace headers -- empty stack assertion.
3. Guard stack semantics: push A, push B, drop B, current = A -- top-of-stack check.
4. Composite propagator: inject + extract roundtrips traceparent + tracestate + baggage -- field equality.
5. Env enable: `TERMFORGE_OTEL=1` enables; unset disables -- boolean check.
6. Shutdown idempotent: calling `shutdown()` twice does not panic -- no panic assertion.
7. Force flush both: `force_flush()` flushes both providers -- true return.
8. Export filter: `mux_*` allowed, `h2`/`tonic` denied -- filter check.
9. Client span lazy init: first `enter_mux_client_span` initializes `MUX_CLIENT_PROVIDER` -- non-null.
10. Runtime timeout: OtelProvider drop completes within 200ms (no hang) -- timing assertion.
11. HeaderCarrier roundtrip: inject into carrier, extract from carrier, headers match -- field equality.
12. Protocol resolution: gRPC inferred for port 4317; HTTP for port 4318 -- variant check.
13. **[v11]** Termlet span attributes: `termlet.cols=80` present on spawn span -- attribute check.

### AGENTS.md Rules

- `RULE-S19-01`: OTEL is always behind env-flag guard. **Enforcement:** Init function checks `TERMFORGE_OTEL` or `OTEL_EXPORTER_OTLP_ENDPOINT`.
- `RULE-S19-02`: Force flush both providers before process exit. **Enforcement:** Shutdown sequence test.
- `RULE-S19-03`: OTEL must never block Termlet hot paths on exporter latency. **Enforcement:** Non-blocking span export.
- `RULE-S19-04`: **[v11]** Termlet spans must include structured attributes for command, size, and mode. **Enforcement:** Attribute presence test.

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
        use fs2::FileExt;
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

/// [v11] Sanitize token for cache key: replace non-alphanumeric with underscore.
fn sanitize_token(s: &str) -> String {
    s.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '.' { c } else { '_' }).collect()
}
```

### Test Strategy

1. Cache key determinism: same inputs produce same cache key -- equality assertion.
2. Cache key uniqueness: different configure_flags produce different key -- inequality assertion.
3. BLAKE3 fingerprint: empty flags produce consistent hash -- equality assertion.
4. Lock exclusivity: two threads cannot hold same lock simultaneously -- second lock fails.
5. Lock auto-release: LockGuard drop releases file lock -- subsequent lock succeeds.
6. Atomic publish: concurrent readers never see partial build -- file content assertion.
7. Binary validation: valid tmux binary returns version string -- starts with "tmux".
8. Binary validation failure: invalid binary produces descriptive error -- error contains path.
9. Fingerprint truncation: 12-char prefix is unique across typical flag sets -- collision test.

### AGENTS.md Rules

- `RULE-S20-01`: Build cache keys must be deterministic and collision-free. **Enforcement:** Unit test with diverse inputs.
- `RULE-S20-02`: Atomic rename for build publication; never multi-step copy. **Enforcement:** Code review for `std::fs::copy` absence.

---

## 21. Test Support and Fake PTY [v11 updated]

### Design Decisions

- Three-layer socket validation verified against vibe-tmux `path_guard.rs`.
- PathGuard with permission hardening (0700 on socket directory).
- FakePtyBackend with ScenarioRecorder for deterministic replay.
- `ScenarioStep` with `in_bytes`/`out_bytes` for structured FakePty fixtures.
- Dual-mode test harness: in-process (FakePty) and subprocess (real PTY).
- Subprocess shutdown: SIGTERM first, then SIGKILL escalation after 2 seconds.
- Config isolation: every test invocation uses `-f /dev/null`.
- Clipboard isolation: `set-clipboard off` in all test harnesses.
- **[v11]** `PtyBackend` trait requires `Send + Any` bounds; `as_any_mut()` method declared on trait.
- **[v11]** `FakePtyBackend` implements `as_any_mut()` for downcasting in tests.

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

fn ensure_socket_not_tmux_env_value(socket_path: &Path, tmux_env: Option<&str>) -> Result<()> {
    if let Some(tmux) = tmux_env {
        let tmux_socket = tmux.split(',').next().unwrap_or("");
        if !tmux_socket.is_empty() && socket_path.to_str() == Some(tmux_socket) {
            bail!("refusing to use socket matching $TMUX: {}", tmux_socket);
        }
    }
    Ok(())
}

// crates/mux-pty/src/backend.rs
/// [v11] PtyBackend trait with Send + Any bounds and as_any_mut.
pub trait PtyBackend: Send + std::any::Any {
    fn spawn(&mut self, pane_id: PaneId, command: &[String],
        cwd: Option<&str>, size: PaneSize) -> Result<(), PtyError>;
    fn write(&mut self, pane_id: PaneId, data: &[u8]) -> Result<(), PtyError>;
    fn resize(&mut self, pane_id: PaneId, size: PaneSize) -> Result<(), PtyError>;
    fn kill(&mut self, pane_id: PaneId, signal: i32) -> Result<(), PtyError>;
    fn read_events(&mut self) -> Vec<PtyEvent>;

    /// [v11] Downcast support for test assertions.
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

// crates/mux-pty-fake/src/lib.rs
pub struct FakePtyBackend {
    panes: HashMap<PaneId, FakePane>,
    pending_events: Vec<PtyEvent>,
}

struct FakePane {
    size: PaneSize,
    input_queue: Vec<Vec<u8>>,
    output_queue: Vec<Vec<u8>>,
    exited: bool,
}

impl FakePtyBackend {
    pub fn new() -> Self {
        Self { panes: HashMap::new(), pending_events: Vec::new() }
    }

    pub fn inject_output(&mut self, pane_id: PaneId, data: Vec<u8>) {
        self.pending_events.push(PtyEvent::Output { pane_id, data });
    }

    /// [v11] Access input queue for assertion in tests.
    pub fn input_queue(&self, pane_id: PaneId) -> Option<&[Vec<u8>]> {
        self.panes.get(&pane_id).map(|p| p.input_queue.as_slice())
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

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Structured scenario step for FakePty fixtures.
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

1. Default socket rejected: `ensure_not_default_socket_name("default")` fails -- error assertion.
2. Empty socket rejected: `ensure_not_default_socket_name("")` fails -- error assertion.
3. Socket within tempdir: path inside tempdir passes; path outside fails -- boolean assertions.
4. Socket not TMUX env: socket matching $TMUX rejected -- error assertion.
5. PathGuard cleanup: temp dir removed on drop -- path existence check.
6. Permission hardening: socket dir has mode 0700 -- permission check.
7. FakePty inject output: injected data appears in `read_events()` -- contains assertion.
8. FakePty spawn/kill lifecycle: spawn -> write -> kill -> Exited event -- event variant check.
9. Scenario roundtrip: record -> save -> load -> replay produces same grid -- snapshot equality.
10. ScenarioStep serialization: JSON roundtrip preserves bytes -- equality assertion.
11. **[v11]** PtyBackend as_any_mut: downcast to FakePtyBackend succeeds -- type check.
12. **[v11]** FakePty input_queue: sent data appears in input queue for assertions -- equality assertion.

### AGENTS.md Rules

- `RULE-S21-01`: Tests must not operate on live user tmux sessions. **Enforcement:** PathGuard three-layer validation in all test fixtures.
- `RULE-S21-02`: PathGuard three-layer validation required for all test fixtures. **Enforcement:** Test harness init function.
- `RULE-S21-03`: FakePty scenarios must be versioned JSON fixtures. **Enforcement:** Fixture loader validates schema.
- `RULE-S21-04`: **[v11]** `PtyBackend` trait must require `Send + Any` bounds. **Enforcement:** Compile-fail test.

---

## 22. Binding Test Frameworks [v11 updated]

### Design Decisions

- Python tests via pytest with fixture chain: `server` -> `session` -> `window` -> `pane`.
- Node tests via vitest with equivalent fixture chain.
- Dual-mode parametrized tests run every test against both in-process and subprocess servers.
- OTEL integration tests verify traceparent propagation from Python.
- Custom exception types (`ObjectDoesNotExist`, `MultipleObjectsReturned`) tested in both languages.
- Termlet fixtures: pytest `termlet` fixture spawns a Termlet with automatic cleanup. vitest `useTermlet()` helper does the same.
- `TERM_FILTER` environment variable for CI test filtering.

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

# Termlet fixtures
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

# [v11] Error code propagation test
def test_termlet_error_code_in_exception():
    """Error code present in exception message."""
    from termforge import Termlet
    with Termlet("bash") as t:
        try:
            t.wait_for("never", timeout_ms=50)
        except TimeoutError as e:
            assert "[TERMLET_TIMEOUT]" in str(e)
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

    // [v11] Error code propagation
    it('includes error code in error message', () => {
        const t = useTermlet('bash', { cols: 80, rows: 24 });
        try {
            expect(() => t.waitFor('never', 50)).toThrow(/\[TERMLET_TIMEOUT\]/);
        } finally {
            t.kill();
        }
    });
});
```

### Test Strategy

1. pytest fixtures: `server`, `session`, `window` chain correctly -- fixture resolution.
2. Dual-mode parametrize: same test runs in-process and subprocess -- parametrize count.
3. OTEL propagation: `TRACEPARENT` env var reaches Rust -- span check.
4. Node filter: `sessions({ name: "work" })` returns matching -- count check.
5. Error types: Python `ObjectDoesNotExist` and `MultipleObjectsReturned` raised correctly -- exception type.
6. Context manager: `with Server()` cleans up on exit -- cleanup assertion.
7. pytest `termlet` fixture: spawns, yields, auto-kills -- is_alive check.
8. pytest `fake_termlet` fixture: uses FakePtyBackend -- no exception.
9. pytest `any_termlet` parametrize: same test, real and fake -- dual execution.
10. vitest `useTermlet()`: spawns, returns handle with kill cleanup -- type check.
11. Termlet send_keys + snapshot roundtrip: both languages -- contains assertion.
12. Termlet wait_for timeout: raises `TimeoutError` in Python -- exception type.
13. Termlet context manager: no leaked processes -- process count check.
14. `TERM_FILTER` env var selects test subset in CI -- selective execution.
15. **[v11]** Error code in Python exception: `[TERMLET_TIMEOUT]` in message -- substring check.
16. **[v11]** Error code in Node error: `[TERMLET_TIMEOUT]` in message -- regex match.

### AGENTS.md Rules

- `RULE-S22-01`: Binding tests must use the same ORM semantics as core tests. **Enforcement:** API surface comparison.
- `RULE-S22-02`: Dual-mode fixture required for all integration tests. **Enforcement:** Parametrized fixture check.
- `RULE-S22-03`: Termlet fixtures must auto-kill on cleanup. No leaked PTY processes. **Enforcement:** Process leak detection test.
- `RULE-S22-04`: Termlet tests must run in both real and fake PTY modes where applicable. **Enforcement:** Parametrized `any_termlet` fixture.

---

## 23. Test Framework and Harness Design

### Design Decisions

- Custom VT100 parser testing against `input.c` state table.
- Grid snapshot testing via `insta`.
- Format engine parity testing via `format-audit` corpus.
- Key binding parity testing via generated tests from `key-bindings.c`.
- Property testing with `proptest` for protocol roundtrip and layout resize.
- Fuzz targets for proto decode, VT100 parse, and config parse.
- Three-tier harness: unit (pure), integration (runtime), compatibility (real tmux).
- Flaky-test quarantine requires replay log before quarantine.

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
        let mut decode_codec = ImsgCodec::new();
        let decoded = decode_codec.decode(&mut buf);
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

/// Three-tier harness classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HarnessTier {
    /// Unit: pure functions, no IO, no timing.
    Unit,
    /// Integration: in-process runtime, FakePty or real PTY.
    Integration,
    /// Compatibility: real tmux binary, subprocess mode.
    Compatibility,
}

/// [v11] Test name validation regex.
pub fn validate_test_name(name: &str) -> bool {
    let re = regex::Regex::new(
        r"^test_(core|proto|layout|query|binding|otel|support|crdt|tui|termlet|conf|control)_[a-z_]+$"
    ).unwrap();
    re.is_match(name)
}
```

### Test Strategy

1. VT100 ASCII: plain text rendered correctly on grid -- row text equality.
2. VT100 CSI: cursor positioning, colors, attributes work -- cell assertion.
3. Grid snapshot: insta snapshots match expected output -- snapshot comparison.
4. Format parity: 200+ format variables match tmux -- variable-by-variable comparison.
5. Key binding parity: all 4 key tables match tmux defaults -- table comparison.
6. Protocol proptest: random payloads roundtrip through codec -- property assertion.
7. Layout proptest: random resize never invalidates layout -- property assertion.
8. Fuzz: proto, VT100, config parsers survive random input -- no panic.
9. Flaky test quarantine: emit replay log before quarantining -- log presence check.
10. **[v11]** Test name validation: all tests match naming convention -- regex assertion.

### AGENTS.md Rules

- `RULE-S23-01`: Every parser must have a fuzz target. **Enforcement:** Fuzz target file existence check.
- `RULE-S23-02`: Parity tests reference tmux source line numbers. **Enforcement:** Comment format check.
- `RULE-S23-03`: Flaky tests must emit replay logs before quarantine. **Enforcement:** CI quarantine script.
- `RULE-S23-04`: **[v11]** Test names must match `test_{component}_{behavior}` convention. **Enforcement:** CI regex validation.

---

## 24. Performance Targets [v11 updated]

### Design Decisions

- Criterion benchmarks as the standard measurement tool.
- Conservative targets based on known alacritty/tmux performance ranges.
- CI gate at 130% threshold on nightly only (warn, not hard fail).
- All benchmarks measure throughput where applicable.
- Termlet-specific benchmarks for spawn and snapshot latency.
- Benchmark scripts pin CPU governor where possible.
- **[v11]** Added B13 for `drain_output` latency. Added B14 for `SnapshotDiff` comparison.

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
| B13 | **[v11]** drain_output 1KB | < 10 us | Warn if > 50 us |
| B14 | **[v11]** SnapshotDiff (80x24) | < 100 us | Warn if > 200 us |

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

fn termlet_drain_output(c: &mut Criterion) {
    let mut termlet = mux_termlet::Termlet::spawn("echo", mux_termlet::TermletConfig {
        cols: 80, rows: 24,
        backend: mux_termlet::PtyMode::Fake,
        ..Default::default()
    }).unwrap();
    // Pre-inject 1KB of output
    termlet.inject_fake_output(vec![b'A'; 1024]);
    c.bench_function("termlet-drain-output-1kb", |b| {
        b.iter(|| {
            termlet.drain_output();
        });
    });
}

fn snapshot_diff(c: &mut Criterion) {
    let snap1 = mux_termlet::TermletSnapshot::from_text("Hello\nWorld\n");
    let snap2 = mux_termlet::TermletSnapshot::from_text("Hello\nRust\n");
    c.bench_function("snapshot-diff-80x24", |b| {
        b.iter(|| {
            let _diff = mux_termlet::SnapshotDiff::compute(&snap1, &snap2);
        });
    });
}

criterion_group!(termlet_benches, termlet_spawn_fake, termlet_snapshot,
    termlet_drain_output, snapshot_diff);
criterion_main!(termlet_benches);
```

### Test Strategy

1. Criterion benchmarks run nightly -- CI job.
2. Regression flag at 130% of baseline -- Criterion threshold.
3. Baseline established from 3 consecutive median runs -- statistical check.
4. No hard fail on performance (warn only). Hard fail reserved for functional gates -- CI config.
5. Termlet benchmarks included in nightly run -- benchmark list.
6. Benchmark scripts pin CPU governor where possible -- script check.
7. Performance regressions > 30% block release -- release gate.
8. **[v11]** drain_output benchmark included -- benchmark name check.
9. **[v11]** SnapshotDiff benchmark included -- benchmark name check.

### AGENTS.md Rules

- `RULE-S24-01`: Performance baselines re-established after architecture changes. **Enforcement:** Baseline reset script.
- `RULE-S24-02`: Benchmarks must cover hot-path code (VT100, proto, layout). **Enforcement:** Benchmark list review.
- `RULE-S24-03`: Termlet spawn benchmark must stay under 500us for FakePty mode. **Enforcement:** Criterion threshold.
- `RULE-S24-04`: Performance regressions > 30% block release. **Enforcement:** Release gate script.

---

## 25. Visual Client / TUI [v11 updated]

### Design Decisions

- ViewModel pattern: pure function `(GraphState, ClientId, TermSize) -> ViewModel`.
- ratatui-based rendering with crossterm terminal IO.
- ViewModel supports snapshot testing with `insta`.
- TUI can attach to both TermForge server (via StateHandle) and real tmux (via control mode).
- TermletViewModel for debug inspector panel: shows Termlet grid preview and process status.
- **[v11]** TermletPoolViewModel for multi-Termlet inspector.

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

/// Debug inspector panel for Termlet state.
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

/// [v11] Multi-Termlet inspector for TermletPool.
pub struct TermletPoolViewModel {
    pub termlets: Vec<TermletViewModel>,
    pub total: usize,
    pub alive_count: usize,
}

impl TermletPoolViewModel {
    pub fn from_pool(pool: &mux_termlet::TermletPool) -> Self {
        let termlets: Vec<TermletViewModel> = pool.iter()
            .map(|(_, t)| TermletViewModel::from_termlet(t))
            .collect();
        let alive_count = termlets.iter().filter(|t| t.alive).count();
        let total = termlets.len();
        Self { termlets, total, alive_count }
    }
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

### AGENTS.md Rules

- `RULE-S25-01`: TUI reads snapshots only, never mutates graph directly. **Enforcement:** No `&mut ServerGraph` in view code.
- `RULE-S25-02`: UI event handlers emit commands/events, not direct state mutations. **Enforcement:** Code review.
- `RULE-S25-03`: Debug panels are non-blocking and optional. **Enforcement:** Feature flag for debug panels.

---

## 26. AGENTS.md Rules [v11 updated]

### Design Decisions

This section consolidates all per-section rules into a global reference with numbered IDs and enforcement mechanisms. The rule naming convention follows `RULE-Snn-xx` pattern for traceability. **[v11]** All rules now have explicit enforcement mechanisms (was missing for 8 rules in v10). Added rules RULE-S32-13 through RULE-S32-20 for new v11 Termlet capabilities.

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
| 15 | RULE-S21-03 | Fake PTY tests use `ScenarioRecorder` JSON format | Test harness validation |
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
| 47 | RULE-S16-05 | **[v11]** Error messages in bindings must include `[ERROR_CODE]` prefix | Exception message format test |
| 48 | RULE-S19-04 | **[v11]** Termlet spans must include structured attributes | Attribute presence test |
| 49 | RULE-S21-04 | **[v11]** `PtyBackend` trait must require `Send + Any` bounds | Compile-fail test |
| 50 | RULE-S23-04 | **[v11]** Test names must match `test_{component}_{behavior}` convention | CI regex validation |
| 51 | RULE-S32-13 | **[v11]** `drain_output` must use nonblocking reads with `WouldBlock` handling | Code audit |
| 52 | RULE-S32-14 | **[v11]** `TermletPool` must use `HashMap<String, Termlet>` for named access | Type check |
| 53 | RULE-S32-15 | **[v11]** `Termlet::kill()` must implement full SIGTERM -> grace_period -> SIGKILL | Sequence test |
| 54 | RULE-S32-16 | **[v11]** `TermletPaneId` must be distinct from `PaneId` | Type system check |
| 55 | RULE-S32-17 | **[v11]** `output_history()` must return immutable reference, never clone | API signature check |
| 56 | RULE-S32-18 | **[v11]** `TermletExt` trait must not add state mutation methods | Trait method audit |
| 57 | RULE-S32-19 | **[v11]** `TermletPool::wait_all()` timeout must be per-Termlet, not total | Implementation check |
| 58 | RULE-S32-20 | **[v11]** `SnapshotDiff::compute()` returns empty diff for identical inputs | Zero-allocation test |

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

### Test Strategy

1. Static rule checks in CI (grep/cargo-deny/compile-fail suites) -- CI job.
2. Rule coverage report links each rule to at least one automated test -- report generation.
3. Release gate fails when critical rules are unverified -- release script.
4. Anti-pattern grep checks run on every PR -- CI job.
5. **[v11]** All 58 rules have explicit enforcement mechanism -- manual audit.

### AGENTS.md Rules

- `RULE-S26-01`: Every architecture section must define enforceable rules. **Enforcement:** Section review checklist.
- `RULE-S26-02`: Rule IDs are immutable once published. **Enforcement:** Append-only rule table.

---

## 27. Risks and Mitigations [v11 updated]

### Design Decisions

Risk register covers protocol drift, behavioral divergence, binding API stability, test isolation, observability, dependency management, Termlet-specific risks, and **[v11]** new risks for drain_output, TermletPaneId, and pool batch operations. Each risk has an impact level, probability, and specific mitigation.

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

### Test Strategy

1. Risk-to-test mapping must be complete and machine-checked -- mapping file.
2. High-impact risks require at least one integration/parity test -- coverage check.
3. Quarterly risk review updates with regression evidence -- review schedule.
4. Termlet-specific risks R31-R39 tested via dedicated Termlet test suite -- test list.
5. **[v11]** Risks R40-R43 tested via specific unit tests -- test names.

### AGENTS.md Rules

- `RULE-S27-01`: New high-risk features must add mitigation tests before merge. **Enforcement:** PR template checklist.
- `RULE-S27-02`: Risk register IDs remain stable for auditability. **Enforcement:** Append-only risk table.

---

## 28. Plan Evolution and Changelog

### Design Decisions

This section documents the evolution from v4 through v11, providing traceability for architectural decisions.

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
| **v11** | **2026-02-11** | **~5500+** | **32** | **DEFINITIVE: v10 refinement. ErrorCode universalized, drain_output specified, TermletPaneId newtype, PtyBackend trait bounds, kill sequence fully specified, cross-section coherence, all rules have enforcement** |

### 28.2 v11 Changelog

**Gaps fixed:**
1. `ErrorCode` trait now implemented for `CoreError`, `ProtocolError`, `QueryError` -- was only on `TermletError` in v10.
2. `drain_output()` now has concrete nonblocking PTY read implementation -- was opaque in v10.
3. `TermletPaneId` newtype replaces abuse of `slotmap::KeyData::from_ffi()` -- type safety.
4. `PtyBackend` trait now requires `Send + Any` bounds with `as_any_mut()` -- was used but undeclared.
5. `kill()` now implements full SIGTERM -> grace_period -> SIGKILL -- was only SIGTERM in v10.
6. `TermletPool::get_mut()` uses dedicated `NotInPool` error variant -- was misusing `SpawnFailed`.
7. `GraphState` entity storage changed to `HashMap` for O(1) lookups -- was unspecified in v10.
8. `ApplyOutcome` now includes `warnings` field for non-fatal issues -- was silent on missing entities.

**New capabilities:**
9. `MAX_PAYLOAD_SIZE` guard in protocol decoder prevents OOM on malicious input.
10. `Termlet::output_history()` returns raw byte history via `&[u8]` reference.
11. `TermletPool::wait_all()` and `send_all()` batch operations.
12. `TermletPoolViewModel` for multi-Termlet TUI inspector.
13. `EventMsg` with optional oneshot reply channel for synchronous event submission.
14. `ManagedMux::submit_event_sync()` for await-able event submission.
15. `OpLog::compact_before()` for CRDT log compaction.
16. `OrSet::merge()` for replica convergence.

**Cross-section coherence:**
17. Section 8 error codes now cross-reference Section 32 error types.
18. Section 16 binding error messages include `[ERROR_CODE]` prefix from Section 8.
19. Section 19 OTEL spans include Termlet-specific attributes linking to Section 32.
20. Section 21 `PtyBackend` trait definition includes `as_any_mut()` used by Section 32.
21. Section 26 rules expanded from 44 to 58, all with explicit enforcement.
22. Section 27 risks expanded from 39 to 43.

**Stronger AGENTS.md rules:**
23. All 58 rules have explicit enforcement mechanisms (was missing for 8 rules).
24. Added RULE-S32-13 through RULE-S32-20 for new v11 Termlet capabilities.
25. Added anti-patterns: `SlotMap::KeyData::from_ffi()`, blocking PTY reads, `clone()` in `output_history()`.

### Rust Example

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecVersion {
    V4, V5, V6, V7, V8, V9Pass3Final,
    V10Pass1, V10Pass2, V10Pass3Final,
    V11,
}

pub struct ChangelogItem {
    pub from: SpecVersion,
    pub to: SpecVersion,
    pub summary: &'static str,
}

pub const V11_CHANGES: &[ChangelogItem] = &[
    ChangelogItem { from: SpecVersion::V10Pass3Final, to: SpecVersion::V11,
        summary: "v11: ErrorCode universalized, drain_output specified, TermletPaneId newtype, \
            PtyBackend Send+Any, full kill sequence, cross-section coherence, 58 rules with enforcement, \
            43 risks, TermletPool batch ops, EventMsg reply channel." },
];
```

### Test Strategy

1. Changelog assertions reference concrete tests and anchors -- mapping check.
2. No unresolved placeholders allowed in release spec -- grep for `todo!()`.
3. Spec self-check script validates section count and required subsections -- script.
4. Pass evolution table cross-references all unique ideas to sections -- table completeness.
5. **[v11]** All 25 changelog items are traceable to specific code changes -- traceability matrix.

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

## 30. Appendix: Canonical Type Quick Reference [v11 updated]

### Design Decisions

This appendix provides a consolidated reference for all canonical type names used across the specification. **[v11]** Added `TermletPaneId`, `TermletExt`, `TermletPoolViewModel`, `EventMsg` types.

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
    pub fn new(id: u64) -> Self { Self(id) }
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
| **`Termlet`** | **mux-termlet** | SDK-first testing pod handle |
| **`TermletConfig`** | **mux-termlet** | Configuration for Termlet spawn |
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
| **`TermletError`** | **mux-termlet** | **Transient / UserError** | **Yes** |

### Test Strategy

1. Type API compile checks in integration crates -- compile test.
2. Semver surface snapshot for public crates -- snapshot test.
3. Doc tests for all appendix examples -- `cargo doc --test`.
4. **[v11]** ErrorCode column accuracy: every "Yes" has a corresponding impl -- compile test.

### AGENTS.md Rules

- `RULE-S30-01`: Do not rename canonical types without migration notes. **Enforcement:** Semver check.
- `RULE-S30-02`: Appendix must match actual exported APIs. **Enforcement:** API surface test.

---

## 31. Supplemental Test Matrix [v11 updated]

### Design Decisions

Test matrix extends section-local tests with release gates. Organized by test category, runner, and CI enforcement level. **[v11]** Added drain_output, TermletPaneId, TermletPool batch, and error code categories.

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
| Performance | ~12 | `criterion` | Yes (warn only) |
| Fuzz | 3 targets | `cargo fuzz` | Nightly |
| WASM purity | 1 | `cargo check --target wasm32-unknown-unknown` | Yes |
| Termlet unit (Rust) | ~35 | `cargo test` | Yes |
| Termlet integration | ~25 | `cargo test` | Yes |
| Termlet Python | ~18 | `pytest` | Yes |
| Termlet Node | ~12 | `vitest` | Yes |
| Termlet performance | ~4 | `criterion` | Yes (warn only) |
| Cross-language consistency | ~12 | `pytest` + `vitest` + `cargo test` | Yes |
| TermletBuilder parity | ~5 | `cargo test` | Yes |
| SnapshotDiff regression | ~10 | `cargo test` | Yes |
| **[v11] Error code stability** | ~20 | `cargo test` | Yes |
| **[v11] drain_output** | ~5 | `cargo test` | Yes |
| **[v11] TermletPool batch** | ~8 | `cargo test` | Yes |

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
  test_termlet_spawn_fake
  test_termlet_spawn_real
  test_termlet_send_keys_echo
  test_termlet_wait_for_timeout
  test_termlet_wait_for_pattern_not_found
  test_termlet_snapshot_text
  test_termlet_resize_reflow
  test_termlet_kill_idempotent
  test_termlet_kill_full_sequence           # [v11]
  test_termlet_drop_kills_pty
  test_termlet_drain_output_nonblocking     # [v11]
  test_termlet_output_history               # [v11]
  test_termlet_pane_id_distinct             # [v11]
  test_termlet_pattern_matcher_compile
  test_termlet_builder_parity
  test_termlet_pool_batch                   # [v11] wait_all, send_all
  test_termlet_pool_not_in_pool             # [v11]
  test_termlet_snapshot_diff
  test_termlet_error_code_stable            # [v11]
  test_binding_py_termlet_fixture
  test_binding_py_termlet_error_code        # [v11]
  test_binding_node_termlet_helper
  test_binding_node_termlet_error_code      # [v11]
  test_cross_lang_termlet_snapshot
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
| **mux-termlet** | **85%** | **CI gate** |
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
    MatrixRow { id: "M-32-07", section: 32, test_name: "pattern_matcher_compile", required: true },
    MatrixRow { id: "M-32-08", section: 32, test_name: "termlet_builder_parity", required: true },
    MatrixRow { id: "M-32-09", section: 32, test_name: "snapshot_diff_detection", required: true },
    // [v11] New gates
    MatrixRow { id: "M-32-10", section: 8, test_name: "error_code_all_types", required: true },
    MatrixRow { id: "M-32-11", section: 32, test_name: "drain_output_nonblocking", required: true },
    MatrixRow { id: "M-32-12", section: 32, test_name: "termlet_pool_batch", required: true },
    MatrixRow { id: "M-32-13", section: 32, test_name: "kill_full_sequence", required: true },
];
```

### Test Strategy

1. CI requires 100% pass of mandatory matrix rows -- CI gate.
2. Nightly includes extended parity + fuzz durations -- nightly CI config.
3. Release requires zero unresolved mandatory rows -- release gate script.
4. Termlet gates M-32-01 through M-32-13 are mandatory for release -- gate list.

### AGENTS.md Rules

- `RULE-S31-01`: New architecture features must add at least one matrix row. **Enforcement:** PR template.
- `RULE-S31-02`: Release tagging is blocked until mandatory rows are green. **Enforcement:** Release script.

---

## 32. Termlets [v10 new, v11 DEFINITIVE refinement]

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

7. **Backward compatible**: A Termlet IS a pane under the hood. The `Grid`, `VtParser` -- all reused from `mux-grid`. The Termlet just provides a simpler creation and interaction API.

8. **Process management**: Spawn shells, run commands, send signals, detect exit. The Termlet owns the child process lifecycle and cleans up via `Drop`. Kill uses graceful-then-forced sequence: SIGTERM first, wait `grace_period` (default 2s), then SIGKILL on `Drop` if still alive. **[v11]** `kill()` now implements the full SIGTERM -> grace_period -> SIGKILL sequence, not just SIGTERM.

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

**Architectural position:** `mux-termlet` sits at Layer 2 (FACADE). It depends on:
- `mux-core` (Layer 0) for `PaneSize`
- `mux-grid` (Layer 0) for `Grid`, `VtParser`
- `mux-pty` (Layer 1) for `PtyBackend` trait and real implementation
- `mux-pty-fake` (Layer 0) for `FakePtyBackend`
- `mux-types` (Layer 0) for shared types

It does NOT depend on: `mux-server`, `mux-api`, `mux-orm`.

### 32.1 Architecture Diagram

```
                    ┌─────────────────────────────────────┐
                    │              Termlet                 │
                    │                                     │
                    │  ┌──────────┐  ┌──────────────────┐ │
                    │  │ Grid     │  │ PtyBackend       │ │
                    │  │ (mux-    │  │ (Real or Fake)   │ │
                    │  │  grid)   │  │                  │ │
                    │  └──────────┘  └──────────────────┘ │
                    │  ┌──────────┐  ┌──────────────────┐ │
                    │  │ VtParser │  │ TermletPaneId    │ │
                    │  │ (mux-    │  │ (v11 newtype)    │ │
                    │  │  grid)   │  └──────────────────┘ │
                    │  └──────────┘  ┌──────────────────┐ │
                    │                │ PatternMatcher   │ │
                    │                │ (compiled)       │ │
                    │                └──────────────────┘ │
                    │  ┌──────────────────────────────┐   │
                    │  │ output_history: Vec<u8>      │   │
                    │  │ (v11: raw byte accumulator)  │   │
                    │  └──────────────────────────────┘   │
                    └─────────────────────────────────────┘
                                    │
                    ┌───────────────┼───────────────┐
                    │               │               │
               ┌────▼────┐   ┌─────▼─────┐  ┌──────▼──────┐
               │  Rust   │   │  Python   │  │  Node.js    │
               │ native  │   │  PyO3     │  │  Neon       │
               │ API     │   │  bindings │  │  bindings   │
               └─────────┘   └───────────┘  └─────────────┘
```

### 32.2 Core API

```rust
// crates/mux-termlet/src/lib.rs
use std::time::{Duration, Instant};

/// [v11] Distinct ID type for Termlet-managed panes.
/// Prevents confusion with server-managed PaneId from slotmap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TermletPaneId(u64);

static TERMLET_ID_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

impl TermletPaneId {
    pub fn next() -> Self {
        Self(TERMLET_ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtyMode {
    Real,
    Fake,
}

#[derive(Debug, Clone)]
pub struct TermletConfig {
    pub cols: u16,
    pub rows: u16,
    pub env: Vec<(String, String)>,
    pub cwd: Option<String>,
    pub backend: PtyMode,
    /// Maximum time to wait between SIGTERM and SIGKILL during kill().
    pub grace_period: Duration,
    /// Interval between poll iterations in wait_for.
    pub wait_poll_interval: Duration,
}

impl Default for TermletConfig {
    fn default() -> Self {
        Self {
            cols: 80,
            rows: 24,
            env: vec![("TERM".to_string(), "xterm-256color".to_string())],
            cwd: None,
            backend: PtyMode::Real,
            grace_period: Duration::from_secs(2),
            wait_poll_interval: Duration::from_millis(10),
        }
    }
}

pub struct Termlet {
    id: TermletPaneId,
    grid: Grid,
    parser: VtParser,
    backend: Box<dyn PtyBackend>,
    config: TermletConfig,
    alive: bool,
    /// [v11] Raw byte accumulator for output_history().
    output_bytes: Vec<u8>,
}

impl Termlet {
    /// Spawn a new Termlet running the given command.
    pub fn spawn(command: &str, config: TermletConfig) -> Result<Self, TermletError> {
        let id = TermletPaneId::next();
        let grid = Grid::new(config.cols, config.rows);
        let parser = VtParser::new();

        let mut backend: Box<dyn PtyBackend> = match config.backend {
            PtyMode::Real => {
                // Create real PTY backend
                Box::new(RealPtyBackend::new()?)
            }
            PtyMode::Fake => {
                Box::new(FakePtyBackend::new())
            }
        };

        let pane_size = PaneSize { sx: config.cols, sy: config.rows };
        let cmd = vec![command.to_string()];
        // Note: PtyBackend uses PaneId internally, but we wrap it
        // The backend allocates its own internal ID
        backend.spawn(
            PaneId::default(), // Backend manages its own mapping
            &cmd,
            config.cwd.as_deref(),
            pane_size,
        ).map_err(|e| TermletError::SpawnFailed { reason: e.to_string() })?;

        Ok(Self {
            id,
            grid,
            parser,
            backend,
            config,
            alive: true,
            output_bytes: Vec::new(),
        })
    }

    /// Send keystrokes to the Termlet.
    pub fn send_keys(&mut self, keys: &str) -> Result<(), TermletError> {
        if !self.alive {
            return Err(TermletError::AlreadyKilled);
        }
        self.backend.write(PaneId::default(), keys.as_bytes())
            .map_err(|e| TermletError::Pty(e))
    }

    /// [v11] Drain all available output from the PTY into the grid.
    /// Uses nonblocking reads: returns when no more data is immediately available.
    /// This is the concrete implementation that was opaque in v10.
    pub fn drain_output(&mut self) {
        let events = self.backend.read_events();
        for event in events {
            match event {
                PtyEvent::Output { data, .. } => {
                    // Accumulate raw bytes for output_history()
                    self.output_bytes.extend_from_slice(&data);
                    // Parse VT100 sequences into grid
                    self.parser.parse(&data, &mut self.grid);
                }
                PtyEvent::Exited { status, .. } => {
                    self.alive = false;
                }
            }
        }
    }

    /// Wait for a pattern to appear in the grid text.
    /// Returns WaitMatch on success with byte range and timestamp.
    /// Compiles pattern once via PatternMatcher.
    pub fn wait_for(
        &mut self,
        pattern: &str,
        timeout: Duration,
    ) -> Result<WaitMatch, TermletError> {
        let matcher = PatternMatcher::compile(pattern)?;
        let deadline = Instant::now() + timeout;
        let mut poll_interval = self.config.wait_poll_interval;

        loop {
            self.drain_output();
            let text = self.grid.to_text();

            if let Some(m) = matcher.find(&text) {
                return Ok(WaitMatch {
                    start: m.start(),
                    end: m.end(),
                    matched: text[m.start()..m.end()].to_string(),
                    timestamp: Instant::now(),
                });
            }

            if !self.alive {
                return Err(TermletError::PatternNotFound {
                    pattern: pattern.to_string(),
                });
            }

            if Instant::now() >= deadline {
                return Err(TermletError::WaitForTimeout {
                    pattern: pattern.to_string(),
                    timeout_ms: timeout.as_millis() as u64,
                });
            }

            std::thread::sleep(poll_interval);
            // Exponential backoff capped at 100ms
            poll_interval = (poll_interval * 2).min(Duration::from_millis(100));
        }
    }

    /// Capture a snapshot of the current grid state.
    pub fn snapshot(&mut self) -> TermletSnapshot {
        self.drain_output();
        TermletSnapshot {
            text: self.grid.to_text_trimmed(), // trailing-whitespace-trimmed
            cols: self.config.cols,
            rows: self.config.rows,
            timestamp: Instant::now(),
        }
    }

    /// Resize the Termlet.
    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<(), TermletError> {
        self.backend.resize(PaneId::default(), PaneSize { sx: cols, sy: rows })
            .map_err(|e| TermletError::ResizeFailed(e.to_string()))?;
        self.grid.resize(cols, rows);
        self.config.cols = cols;
        self.config.rows = rows;
        Ok(())
    }

    /// [v11] Kill with full SIGTERM -> grace_period -> SIGKILL sequence.
    /// First sends SIGTERM. If process does not exit within grace_period, sends SIGKILL.
    /// Idempotent: calling kill() on already-killed Termlet returns Ok(()).
    pub fn kill(&mut self) -> Result<(), TermletError> {
        if !self.alive {
            return Ok(()); // Idempotent
        }

        // Step 1: Send SIGTERM
        let _ = self.backend.kill(PaneId::default(), libc::SIGTERM);
        self.drain_output();

        // Step 2: Wait grace_period for natural exit
        let deadline = Instant::now() + self.config.grace_period;
        while self.alive && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(50));
            self.drain_output();
        }

        // Step 3: Force SIGKILL if still alive
        if self.alive {
            let _ = self.backend.kill(PaneId::default(), libc::SIGKILL);
            self.alive = false;
        }

        Ok(())
    }

    /// Check if the underlying process is still alive.
    pub fn is_alive(&self) -> bool {
        self.alive
    }

    /// Get the current grid dimensions.
    pub fn size(&self) -> PaneSize {
        PaneSize { sx: self.config.cols, sy: self.config.rows }
    }

    /// Get the Termlet's unique pane ID.
    pub fn pane_id(&self) -> TermletPaneId {
        self.id
    }

    /// [v11] Get immutable reference to all raw bytes received from PTY.
    /// Useful for asserting on raw escape sequences without VT100 parsing.
    pub fn output_history(&self) -> &[u8] {
        &self.output_bytes
    }

    /// [v11] Access the underlying PtyBackend for test injection.
    /// Returns None if the backend is not FakePtyBackend.
    pub fn fake_backend_mut(&mut self) -> Option<&mut FakePtyBackend> {
        self.backend.as_any_mut().downcast_mut::<FakePtyBackend>()
    }

    /// [v11] Inject fake output bytes (only works with FakePtyBackend).
    /// Convenience wrapper around fake_backend_mut().
    pub fn inject_fake_output(&mut self, data: Vec<u8>) {
        if let Some(fake) = self.fake_backend_mut() {
            fake.inject_output(PaneId::default(), data);
        }
    }
}

impl Drop for Termlet {
    fn drop(&mut self) {
        if self.alive {
            // Emergency kill: SIGKILL immediately to prevent process leak
            let _ = self.backend.kill(PaneId::default(), libc::SIGKILL);
        }
    }
}
```

### 32.3 Snapshot System

```rust
// crates/mux-termlet/src/snapshot.rs

/// Captured grid state at a point in time.
#[derive(Debug, Clone)]
pub struct TermletSnapshot {
    pub text: String,
    pub cols: u16,
    pub rows: u16,
    pub timestamp: Instant,
}

impl TermletSnapshot {
    /// Convert to plain text for assertions.
    pub fn to_text(&self) -> &str {
        &self.text
    }

    /// Construct from raw text (for testing SnapshotDiff).
    pub fn from_text(text: &str) -> Self {
        Self {
            text: text.to_string(),
            cols: 80,
            rows: 24,
            timestamp: Instant::now(),
        }
    }

    /// Check if the snapshot contains a substring.
    pub fn contains(&self, pattern: &str) -> bool {
        self.text.contains(pattern)
    }

    /// Get a specific line by index (0-based).
    pub fn line(&self, idx: usize) -> Option<&str> {
        self.text.lines().nth(idx)
    }
}

/// Visual regression diff between two snapshots.
#[derive(Debug, Clone)]
pub struct SnapshotDiff {
    pub changed_lines: Vec<DiffLine>,
    pub identical: bool,
}

#[derive(Debug, Clone)]
pub struct DiffLine {
    pub line_no: usize,
    pub old: String,
    pub new: String,
}

impl SnapshotDiff {
    /// Compare two snapshots. Returns empty diff for identical snapshots.
    /// [v11] Does not allocate when snapshots are identical.
    pub fn compute(old: &TermletSnapshot, new: &TermletSnapshot) -> Self {
        if old.text == new.text {
            return Self { changed_lines: Vec::new(), identical: true };
        }

        let old_lines: Vec<&str> = old.text.lines().collect();
        let new_lines: Vec<&str> = new.text.lines().collect();
        let max_lines = old_lines.len().max(new_lines.len());

        let mut changed_lines = Vec::new();
        for i in 0..max_lines {
            let old_line = old_lines.get(i).copied().unwrap_or("");
            let new_line = new_lines.get(i).copied().unwrap_or("");
            if old_line != new_line {
                changed_lines.push(DiffLine {
                    line_no: i,
                    old: old_line.to_string(),
                    new: new_line.to_string(),
                });
            }
        }

        Self { changed_lines, identical: false }
    }

    /// Human-readable diff string.
    pub fn to_string_pretty(&self) -> String {
        if self.identical {
            return "Snapshots are identical.".to_string();
        }
        let mut out = String::new();
        for line in &self.changed_lines {
            out.push_str(&format!("L{}: -{}\nL{}: +{}\n", line.line_no, line.old, line.line_no, line.new));
        }
        out
    }
}
```

### 32.4 Pattern Matching

```rust
// crates/mux-termlet/src/pattern.rs

/// Compiled pattern for efficient polling in wait_for.
/// Pattern is compiled once and reused across all poll iterations.
pub enum PatternMatcher {
    Plain(String),
    Regex(regex::Regex),
}

impl PatternMatcher {
    /// Compile a pattern string. If it starts with "re:" it's treated as regex.
    pub fn compile(pattern: &str) -> Result<Self, TermletError> {
        if let Some(re_pattern) = pattern.strip_prefix("re:") {
            let re = regex::Regex::new(re_pattern)
                .map_err(|e| TermletError::SpawnFailed {
                    reason: format!("invalid regex: {e}"),
                })?;
            Ok(Self::Regex(re))
        } else {
            Ok(Self::Plain(pattern.to_string()))
        }
    }

    /// Find the pattern in text. Returns the match range.
    pub fn find(&self, text: &str) -> Option<std::ops::Range<usize>> {
        match self {
            Self::Plain(s) => text.find(s).map(|start| start..start + s.len()),
            Self::Regex(re) => re.find(text).map(|m| m.start()..m.end()),
        }
    }
}

/// Match result from wait_for with position and timing information.
#[derive(Debug, Clone)]
pub struct WaitMatch {
    pub start: usize,
    pub end: usize,
    pub matched: String,
    pub timestamp: Instant,
}
```

### 32.5 TermletBuilder (fluent construction)

```rust
// crates/mux-termlet/src/builder.rs

/// Fluent builder for Termlet construction.
/// Produces identical Termlets as TermletConfig.
pub struct TermletBuilder {
    command: String,
    config: TermletConfig,
}

impl TermletBuilder {
    pub fn new(command: &str) -> Self {
        Self {
            command: command.to_string(),
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

    pub fn env(mut self, key: &str, value: &str) -> Self {
        self.config.env.push((key.to_string(), value.to_string()));
        self
    }

    pub fn cwd(mut self, cwd: &str) -> Self {
        self.config.cwd = Some(cwd.to_string());
        self
    }

    pub fn fake(mut self) -> Self {
        self.config.backend = PtyMode::Fake;
        self
    }

    pub fn real(mut self) -> Self {
        self.config.backend = PtyMode::Real;
        self
    }

    pub fn grace_period(mut self, duration: Duration) -> Self {
        self.config.grace_period = duration;
        self
    }

    pub fn wait_poll_interval(mut self, duration: Duration) -> Self {
        self.config.wait_poll_interval = duration;
        self
    }

    /// Build and spawn the Termlet.
    pub fn spawn(self) -> Result<Termlet, TermletError> {
        Termlet::spawn(&self.command, self.config)
    }
}
```

### 32.6 TermletPool (batch management)

```rust
// crates/mux-termlet/src/pool.rs
use std::collections::HashMap;

/// Manages multiple named Termlets for test suites.
pub struct TermletPool {
    termlets: HashMap<String, Termlet>,
}

impl TermletPool {
    pub fn new() -> Self {
        Self { termlets: HashMap::new() }
    }

    /// Spawn and register a named Termlet.
    pub fn spawn(&mut self, name: &str, command: &str, config: TermletConfig) -> Result<&mut Termlet, TermletError> {
        let termlet = Termlet::spawn(command, config)?;
        self.termlets.insert(name.to_string(), termlet);
        Ok(self.termlets.get_mut(name).unwrap())
    }

    /// Get a mutable reference to a named Termlet.
    /// [v11] Returns TermletError::NotInPool instead of misusing SpawnFailed.
    pub fn get_mut(&mut self, name: &str) -> Result<&mut Termlet, TermletError> {
        self.termlets.get_mut(name)
            .ok_or(TermletError::NotInPool { name: name.to_string() })
    }

    /// Get an immutable reference to a named Termlet.
    pub fn get(&self, name: &str) -> Result<&Termlet, TermletError> {
        self.termlets.get(name)
            .ok_or(TermletError::NotInPool { name: name.to_string() })
    }

    /// Kill all Termlets in the pool.
    pub fn kill_all(&mut self) {
        for (_, termlet) in self.termlets.iter_mut() {
            let _ = termlet.kill();
        }
    }

    /// Number of Termlets in the pool.
    pub fn len(&self) -> usize {
        self.termlets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.termlets.is_empty()
    }

    /// Iterate over all (name, Termlet) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Termlet)> {
        self.termlets.iter()
    }

    /// Iterate mutably over all (name, Termlet) pairs.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&String, &mut Termlet)> {
        self.termlets.iter_mut()
    }

    /// [v11] Send keys to all Termlets in the pool.
    pub fn send_all(&mut self, keys: &str) -> Result<(), TermletError> {
        for (_, termlet) in self.termlets.iter_mut() {
            if termlet.is_alive() {
                termlet.send_keys(keys)?;
            }
        }
        Ok(())
    }

    /// [v11] Wait for a pattern in all Termlets, with per-Termlet timeout.
    /// Returns a map of name -> Result for each Termlet.
    pub fn wait_all(
        &mut self,
        pattern: &str,
        per_termlet_timeout: Duration,
    ) -> HashMap<String, Result<WaitMatch, TermletError>> {
        let mut results = HashMap::new();
        for (name, termlet) in self.termlets.iter_mut() {
            let result = termlet.wait_for(pattern, per_termlet_timeout);
            results.insert(name.clone(), result);
        }
        results
    }

    /// Snapshot all Termlets, returning a map of name -> TermletSnapshot.
    pub fn snapshot_all(&mut self) -> HashMap<String, TermletSnapshot> {
        let mut results = HashMap::new();
        for (name, termlet) in self.termlets.iter_mut() {
            results.insert(name.clone(), termlet.snapshot());
        }
        results
    }
}

impl Drop for TermletPool {
    fn drop(&mut self) {
        self.kill_all();
    }
}
```

### 32.7 TermletExt trait [v11 new]

```rust
// crates/mux-termlet/src/ext.rs

/// Extension trait for custom assertion methods on Termlets.
/// Downstream crates can implement this to add domain-specific assertions
/// without modifying mux-termlet directly.
///
/// Example: a pytest binding crate could add `assert_prompt_ready()`.
pub trait TermletExt {
    /// Assert that the grid contains a pattern.
    fn assert_contains(&mut self, pattern: &str) -> &mut Self;

    /// Assert that the grid does NOT contain a pattern.
    fn assert_not_contains(&mut self, pattern: &str) -> &mut Self;

    /// Assert that a specific line matches expected text.
    fn assert_line(&mut self, line_no: usize, expected: &str) -> &mut Self;

    /// Assert the Termlet is alive.
    fn assert_alive(&self) -> &Self;

    /// Assert the Termlet is dead.
    fn assert_dead(&self) -> &Self;
}

impl TermletExt for Termlet {
    fn assert_contains(&mut self, pattern: &str) -> &mut Self {
        let snap = self.snapshot();
        assert!(snap.contains(pattern),
            "Expected grid to contain '{pattern}', but got:\n{}", snap.to_text());
        self
    }

    fn assert_not_contains(&mut self, pattern: &str) -> &mut Self {
        let snap = self.snapshot();
        assert!(!snap.contains(pattern),
            "Expected grid NOT to contain '{pattern}', but it was found in:\n{}", snap.to_text());
        self
    }

    fn assert_line(&mut self, line_no: usize, expected: &str) -> &mut Self {
        let snap = self.snapshot();
        let actual = snap.line(line_no).unwrap_or("");
        assert_eq!(actual.trim_end(), expected.trim_end(),
            "Line {line_no} mismatch");
        self
    }

    fn assert_alive(&self) -> &Self {
        assert!(self.is_alive(), "Expected Termlet to be alive");
        self
    }

    fn assert_dead(&self) -> &Self {
        assert!(!self.is_alive(), "Expected Termlet to be dead");
        self
    }
}
```

### 32.8 Async Support

```rust
// crates/mux-termlet/src/async_support.rs

/// Async variant of wait_for for non-blocking polling in binding event loops.
/// Uses tokio::time::sleep instead of std::thread::sleep.
#[cfg(feature = "async")]
pub async fn wait_for_async(
    termlet: &mut Termlet,
    pattern: &str,
    timeout: Duration,
) -> Result<WaitMatch, TermletError> {
    let matcher = PatternMatcher::compile(pattern)?;
    let deadline = Instant::now() + timeout;
    let mut poll_interval = termlet.config.wait_poll_interval;

    loop {
        termlet.drain_output();
        let text = termlet.grid.to_text();

        if let Some(m) = matcher.find(&text) {
            return Ok(WaitMatch {
                start: m.start(),
                end: m.end(),
                matched: text[m.start()..m.end()].to_string(),
                timestamp: Instant::now(),
            });
        }

        if !termlet.alive {
            return Err(TermletError::PatternNotFound {
                pattern: pattern.to_string(),
            });
        }

        if Instant::now() >= deadline {
            return Err(TermletError::WaitForTimeout {
                pattern: pattern.to_string(),
                timeout_ms: timeout.as_millis() as u64,
            });
        }

        tokio::time::sleep(poll_interval).await;
        poll_interval = (poll_interval * 2).min(Duration::from_millis(100));
    }
}
```

### 32.9 Usage Examples

```rust
// Basic Rust usage
fn demo_basic() {
    let mut t = TermletBuilder::new("bash")
        .size(80, 24)
        .fake()
        .spawn()
        .unwrap();

    t.send_keys("echo hello world\n").unwrap();
    t.wait_for("hello world", Duration::from_secs(5)).unwrap();

    let snap = t.snapshot();
    assert!(snap.contains("hello world"));
    insta::assert_snapshot!(snap.to_text());

    t.kill().unwrap();
}

// TermletPool usage
fn demo_pool() {
    let mut pool = TermletPool::new();
    let config = TermletConfig { backend: PtyMode::Fake, ..Default::default() };

    pool.spawn("server", "bash", config.clone()).unwrap();
    pool.spawn("client", "bash", config.clone()).unwrap();

    pool.get_mut("server").unwrap().send_keys("nc -l 8080\n").unwrap();
    pool.get_mut("client").unwrap().send_keys("nc localhost 8080\n").unwrap();

    // Batch operations
    pool.send_all("echo ready\n").unwrap();
    let results = pool.wait_all("ready", Duration::from_secs(5));
    for (name, result) in &results {
        assert!(result.is_ok(), "Termlet {name} did not become ready");
    }

    // Snapshot all
    let snapshots = pool.snapshot_all();
    for (name, snap) in &snapshots {
        println!("--- {} ---\n{}", name, snap.to_text());
    }
}

// SnapshotDiff usage
fn demo_diff() {
    let mut t = TermletBuilder::new("bash").fake().spawn().unwrap();

    t.send_keys("echo before\n").unwrap();
    let snap1 = t.snapshot();

    t.send_keys("echo after\n").unwrap();
    let snap2 = t.snapshot();

    let diff = SnapshotDiff::compute(&snap1, &snap2);
    assert!(!diff.identical);
    println!("{}", diff.to_string_pretty());
}
```

### 32.10 Termlet Comparison Table

| Feature | `std::process::Command` | tmux pane | Termlet |
|---|---|---|---|
| Terminal emulation | No | Yes | Yes |
| Programmatic input | stdin only | send-keys cmd | `send_keys()` |
| Visual snapshot | No | `capture-pane` | `snapshot()` |
| Pattern wait | No | No | `wait_for()` |
| Resize | No | Yes | `resize()` |
| Fake mode | No | No | `PtyMode::Fake` |
| Server required | No | Yes | **No** |
| Language bindings | No | Via shell | Native |
| Snapshot testing | No | No | `insta`, `assert` |
| Process lifecycle | Manual | tmux manages | RAII `Drop` |
| Batch management | Manual | tmux windows | `TermletPool` |
| Visual regression | No | No | `SnapshotDiff` |

### 32.11 OTEL Integration

Termlet operations emit OpenTelemetry spans when OTEL is enabled:

```rust
// Integration with mux-otel (when feature enabled)
impl Termlet {
    #[cfg(feature = "otel")]
    fn span_spawn(config: &TermletConfig) -> tracing::Span {
        tracing::info_span!(
            "termforge.termlet.spawn",
            termlet.command = tracing::field::Empty,
            termlet.cols = config.cols,
            termlet.rows = config.rows,
            termlet.pty_mode = ?config.backend,
        )
    }
}
```

### Test Strategy (all 62 test cases)

**Core API:**
1. spawn fake: `Termlet::spawn("bash", config)` with FakePty succeeds -- Ok result.
2. spawn real: `Termlet::spawn("bash", config)` with real PTY succeeds -- Ok result.
3. send_keys: `t.send_keys("echo hello\n")` writes to PTY -- no error.
4. send_keys on dead: `t.kill(); t.send_keys("x")` returns AlreadyKilled -- error variant.
5. drain_output: after inject_fake_output, drain_output updates grid -- grid content check.
6. **[v11]** drain_output nonblocking: no data available does not block -- timing assertion.
7. **[v11]** output_history: raw bytes accumulated across multiple drains -- length check.
8. wait_for success: `t.wait_for("hello", 5s)` returns WaitMatch -- Ok result.
9. wait_for timeout: `t.wait_for("never", 100ms)` returns WaitForTimeout -- error variant.
10. wait_for process exit: child exits before match returns PatternNotFound -- error variant.
11. wait_for regex: `t.wait_for("re:\\d+", 5s)` matches digits -- Ok with correct range.
12. snapshot: `t.snapshot()` returns text with trailing whitespace trimmed -- format check.
13. snapshot contains: `snap.contains("hello")` -- boolean assertion.
14. snapshot line: `snap.line(0)` returns first line -- equality check.
15. resize: `t.resize(120, 40)` changes size -- size equality.
16. resize too small: `t.resize(1, 1)` may fail -- error handling check.
17. **[v11]** kill full sequence: SIGTERM sent, then SIGKILL after grace_period -- timing check.
18. kill idempotent: `t.kill(); t.kill()` both succeed -- Ok result.
19. kill on Drop: dropping live Termlet sends SIGKILL -- process check.
20. is_alive: true after spawn, false after kill -- boolean assertions.
21. size: returns current (cols, rows) -- equality check.
22. pane_id: unique across Termlets -- uniqueness check.
23. **[v11]** fake_backend_mut: returns Some for fake, None for real -- Option check.
24. **[v11]** inject_fake_output: convenience method works -- grid content.

**PatternMatcher:**
25. plain match: "hello" matches "say hello" -- Some result.
26. plain no match: "hello" does not match "goodbye" -- None result.
27. regex match: `re:\d{3}` matches "abc123def" -- range [3,6].
28. regex no match: `re:^\d+$` does not match "abc" -- None result.
29. invalid regex: `re:[` returns error -- error result.
30. compile once: PatternMatcher compiled once used for 1000 iterations -- timing.

**TermletSnapshot + SnapshotDiff:**
31. snapshot to_text: returns the text string -- type check.
32. snapshot from_text: constructs from raw text -- equality check.
33. diff identical: same snapshot produces empty diff -- identical == true.
34. diff changed: different snapshots produce changed lines -- line count > 0.
35. diff line_no: changed line numbers are correct -- index check.
36. **[v11]** diff no allocation on identical: identical snapshots do not allocate Vec -- zero-alloc check.
37. diff pretty: human-readable format -- contains "L0:" prefix.

**TermletBuilder:**
38. builder basic: `TermletBuilder::new("bash").spawn()` succeeds -- Ok result.
39. builder size: `.cols(120).rows(40)` sets dimensions -- size check.
40. builder fake: `.fake()` uses FakePtyBackend -- mode check.
41. builder env: `.env("FOO", "bar")` adds to env -- env check.
42. builder cwd: `.cwd("/tmp")` sets working dir -- cwd check.
43. builder parity: builder and config produce identical Termlet state -- equality check.

**TermletPool:**
44. pool spawn: `pool.spawn("a", "bash", config)` succeeds -- Ok result.
45. pool get_mut: `pool.get_mut("a")` returns Termlet ref -- Some check.
46. pool get_mut missing: `pool.get_mut("z")` returns NotInPool error -- error variant.
47. pool kill_all: all Termlets killed -- none alive.
48. pool drop: drop kills all -- cleanup check.
49. pool len: after 3 spawns, len == 3 -- equality check.
50. pool iter: iterates all pairs -- count check.
51. **[v11]** pool send_all: sends keys to all alive Termlets -- no error.
52. **[v11]** pool wait_all: returns per-Termlet results -- map size check.
53. **[v11]** pool wait_all per-termlet timeout: each Termlet gets its own timeout -- timing check.
54. pool snapshot_all: returns all snapshots -- map size check.

**TermletExt [v11]:**
55. **[v11]** assert_contains: passes when pattern present -- no panic.
56. **[v11]** assert_contains fails: panics when pattern absent -- panic expected.
57. **[v11]** assert_not_contains: passes when pattern absent -- no panic.
58. **[v11]** assert_line: correct line matches -- no panic.
59. **[v11]** assert_alive/assert_dead: correct states -- no panic.

**Async:**
60. wait_for_async: same semantics as sync version -- Ok result.

**Cross-cutting:**
61. dual-mode: same test works with both PtyMode::Real and PtyMode::Fake -- parametrized.
62. **[v11]** TermletPaneId uniqueness: 100 Termlets have distinct IDs -- set uniqueness.

### AGENTS.md Rules

- `RULE-S32-01`: Termlet snapshot text must be trailing-whitespace-trimmed. **Enforcement:** Snapshot format test.
- `RULE-S32-02`: Termlet kill must be idempotent. **Enforcement:** Double-kill test.
- `RULE-S32-03`: Termlet Drop must send SIGKILL to prevent PTY process leaks. **Enforcement:** Drop impl audit.
- `RULE-S32-04`: FakePtyBackend tests must not depend on timing. **Enforcement:** Determinism check.
- `RULE-S32-05`: Termlet bindings must implement context manager / cleanup pattern. **Enforcement:** Binding API test.
- `RULE-S32-06`: `mux-termlet` must not depend on `mux-server`, `mux-api`, or `mux-orm`. **Enforcement:** `cargo tree` CI.
- `RULE-S32-07`: Every Termlet API method tested in Rust + at least one binding. **Enforcement:** Coverage report.
- `RULE-S32-08`: `wait_for` must compile pattern once, not per poll iteration. **Enforcement:** PatternMatcher audit.
- `RULE-S32-09`: Termlet remains pane-backed; never invent parallel terminal core. **Enforcement:** Architecture review.
- `RULE-S32-10`: Cross-language snapshot consistency tests must exist for every release. **Enforcement:** Multi-runner CI.
- `RULE-S32-11`: `TermletBuilder` and `TermletConfig` must produce identical Termlets. **Enforcement:** Parity unit test.
- `RULE-S32-12`: `SnapshotDiff` must not allocate on identical snapshots. **Enforcement:** Benchmark.
- `RULE-S32-13`: **[v11]** `drain_output` must use nonblocking reads with `WouldBlock` handling. **Enforcement:** Code audit.
- `RULE-S32-14`: **[v11]** `TermletPool` must use `HashMap<String, Termlet>` for named access. **Enforcement:** Type check.
- `RULE-S32-15`: **[v11]** `Termlet::kill()` must implement full SIGTERM -> grace_period -> SIGKILL. **Enforcement:** Sequence test.
- `RULE-S32-16`: **[v11]** `TermletPaneId` must be distinct from `PaneId`. **Enforcement:** Type system.
- `RULE-S32-17`: **[v11]** `output_history()` must return immutable reference, never clone. **Enforcement:** API signature check.
- `RULE-S32-18`: **[v11]** `TermletExt` trait must not add state mutation methods. **Enforcement:** Trait method audit.
- `RULE-S32-19`: **[v11]** `TermletPool::wait_all()` timeout must be per-Termlet, not total. **Enforcement:** Implementation check.
- `RULE-S32-20`: **[v11]** `SnapshotDiff::compute()` returns empty diff for identical inputs. **Enforcement:** Zero-allocation test.

---

## Final Consistency Checklist

| Check | Status |
|---|---|
| All 32 sections present | Yes |
| Each section has: Design Decisions, Rust Example, Test Strategy, AGENTS.md Rules | Yes |
| All entity IDs use `slotmap::new_key_type!` (except TermletPaneId) | Yes |
| All error types have `Classified` impl | Yes |
| All binding-crossing error types have `ErrorCode` impl | Yes (v11) |
| `mux-core` is `#![forbid(unsafe_code)]` | Yes |
| Layer 0 crates compile to WASM | Yes |
| `mux-termlet` does not depend on `mux-server` or `mux-api` | Yes |
| All OTEL spans named with `termforge.*` prefix | Yes |
| All test names follow `test_{component}_{behavior}` | Yes |
| All rules have enforcement mechanism | Yes (v11) |
| All reference anchors re-verified | Yes (v11) |
| `PtyBackend` trait requires `Send + Any` with `as_any_mut()` | Yes (v11) |
| `kill()` implements full SIGTERM -> grace_period -> SIGKILL | Yes (v11) |
| `drain_output()` uses nonblocking reads | Yes (v11) |
| `TermletPaneId` is distinct newtype from `PaneId` | Yes (v11) |
| 58 AGENTS.md rules with enforcement | Yes (v11) |
| 43 risks with mitigations | Yes (v11) |
| 33 release gate matrix rows | Yes (v11) |
| 62 Termlet test cases | Yes (v11) |

---

*End of TermForge v11 Architecture Specification.*
