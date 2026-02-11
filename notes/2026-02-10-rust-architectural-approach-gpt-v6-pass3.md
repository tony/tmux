# TermForge v5 Final Specification

Date: 2026-02-11
Pass: 3 (Final Synthesis)
Status: Definitive

## Preamble (license, MSRV, edition)
- License: `MIT OR Apache-2.0`
- Rust Edition: `2024`
- MSRV: `1.85`
- Protocol target: tmux wire protocol v8 (`tmux-protocol.h:23`)
- Compatibility policy: tmux wire behavior is normative. Internal enhancements are allowed only when they do not alter wire-visible behavior.

Implementation roots:
- tmux source of truth: `~/study/c/tmux/`
- Rust reference implementation context: `~/work/rust/vibe-tmux/`
- Python migration reference: `~/work/python/libtmux/src/libtmux/server.py`

Settled v4 decisions carried forward unchanged:
1. Vec-inside-entity (not `SecondaryMap`)
2. WASM compilability as CI purity gate
3. `mux-types` as separate leaf crate
4. `mux-orm` as separate crate from `mux-api`
5. FakePty `ScenarioRecorder` with JSON fixtures
6. Defer `im-rs`; use `Arc<Grid>` + `Arc::make_mut`
7. Custom VT100 parser matching tmux `input.c` state machine
8. Format string engine as pure function
9. Key binding system with table-based dispatch
10. Copy mode as per-pane `CopyModeState`

---

## 0. Open Questions Resolved (Q1-Q7 definitive answers)

### Q1. Layout checksum
Decision: tmux exact rotate-and-add checksum for wire layout strings.
- tmux algorithm: `layout-custom.c:47-57`
- wire dump prefix formatting: `layout-custom.c:69`
- wire parse verification: `layout-custom.c:165-173`

Internal-only persistence may add envelope hash:
```rust
pub struct LayoutEnvelopeV2 {
    pub version: u16,
    pub tmux_layout: String,   // exact "hhhh,<layout>" payload source
    pub blake3_128: [u8; 16],  // internal integrity only
}
```
Constraint: `blake3_128` must never appear on tmux wire paths.

### Q2. Option unset semantics
Decision:
- non-global scope + `set -u`: remove local override, restore inheritance.
- global scope + `set -u`: reset to compiled default.
- `set -U`: also unset pane-local values for target window.

Evidence:
- parent lookup chain: `options.c:228-241`
- unset/default behavior: `options.c:1270-1285`
- command handling of `-u/-U`: `cmd-set-option.c:168-187`
- scope resolution behavior: `options.c:851-918`

### Q3. Control mode authentication
Decision: no auth layer beyond socket permissions + ACL. Optional ingress rate limiting in TermForge.
- socket creation permissions boundary: `server.c:107-139`
- ACL join gate: `server.c:396-400`, `server-acl.c:164-179`
- control mode initialization has no auth challenge: `control.c:759-796`

### Q4. CRDT clocks
Decision: HLC (fixed 20-byte encoding) + DVV with membership-aware compaction.
- overflow handling: `checked_add`; on overflow force millis advance.
- memory bound: DVV cardinality bounded by active node set.

```rust
#[repr(C)]
pub struct HlcTimestamp {
    pub millis: u64,
    pub counter: u32,
    pub node_id: u64,
}
```

### Q5. Benchmarks
Decision:
- no authoritative baseline exists today.
- conservative initial targets.
- Criterion benches must be wired correctly (`[[bench]] harness = false`).
- CI regression gate: 130% threshold, nightly-only gate until baseline stabilizes.

Evidence of current gap in reference workspace:
- benchmark file exists: `~/work/rust/vibe-tmux/crates/mux-refresh/benches/latency.rs`
- no `[[bench]]` entry in crate manifest: `~/work/rust/vibe-tmux/crates/mux-refresh/Cargo.toml:1-18`

### Q6. Python async
Decision:
1. Phase 1: sync API with GIL release around blocking Rust calls.
2. Phase 2: `asyncio.to_thread` wrappers.
3. Phase 3: native streaming via `pyo3-async-runtimes` for subscriptions only.

### Q7. Layout minimum behavior
Decision: resize clamps to minimum and never fails; distribution is round-robin one-cell-at-a-time.
- minimum constant: `tmux.h:99-103` (`PANE_MINIMUM = 1`)
- round-robin distribution: `layout.c:422-463` (especially `448-462`)
- clamp behavior in top-level resize: `layout.c:535-578`

Returned API:
```rust
pub struct ResizeResult {
    pub requested_sx: u16,
    pub requested_sy: u16,
    pub applied_sx: u16,
    pub applied_sy: u16,
}
```

---

## A. Error Handling

### A.1 Taxonomy (canonical)
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorClass {
    Transient,
    ProtocolViolation,
    UserError,
    Bug,
}

pub trait Classified {
    fn class(&self) -> ErrorClass;
}
```

Recovery matrix:
- `Transient`: retry/wait.
- `ProtocolViolation`: kill connection.
- `UserError`: send structured error reply.
- `Bug`: log, crash in debug, isolate in release when possible.

### A.2 Decode boundary contract
```rust
pub enum DecodeOutcome<T, E> {
    NeedMoreBytes,     // Transient
    Message(T),
    Fatal(E),          // ProtocolViolation
}

pub fn decode_frame(src: &mut bytes::BytesMut) -> DecodeOutcome<Frame, ProtoError>;
```

Required behavior:
- do not drop malformed frame and continue.
- terminate peer on protocol violation.

tmux parity anchor:
- invalid client message path kills peer: `server-client.c:3397-3399`, `server-client.c:3472-3475`
- identify protocol violation (`already identified`) returns error: `server-client.c:3599-3600`

### A.3 Per-crate error skeleton
- `crates/mux-proto/src/error.rs`: framing + payload validity.
- `crates/mux-conf/src/error.rs`: parse/eval user errors.
- `crates/mux-server/src/error.rs`: lifecycle/runtime faults.
- `crates/mux-bindings-*/src/error.rs`: language surface mapping.

```rust
#[derive(thiserror::Error, Debug)]
pub enum ProtoError {
    #[error("incomplete frame")]
    Incomplete,
    #[error("length overflow: {len}")]
    LengthOverflow { len: u32 },
    #[error("unknown msg type: {msg_type}")]
    UnknownMsgType { msg_type: u32 },
    #[error("invalid payload: {reason}")]
    InvalidPayload { reason: &'static str },
}

impl Classified for ProtoError {
    fn class(&self) -> ErrorClass {
        match self {
            Self::Incomplete => ErrorClass::Transient,
            Self::LengthOverflow { .. } | Self::UnknownMsgType { .. } | Self::InvalidPayload { .. } => ErrorClass::ProtocolViolation,
        }
    }
}
```

### A.4 Tests
- `crates/mux-proto/tests/decode_incomplete.rs`: `NeedMoreBytes` only.
- `crates/mux-proto/tests/decode_protocol_violation.rs`: bad length/type => `ProtocolViolation` and server disconnect.
- `crates/mux-server/tests/peer_kill_on_bad_frame.rs`: assert peer termination.
- `crates/mux-conf/tests/user_error_reply.rs`: invalid command replies `%error`/error frame, no crash.

---

## B. Configuration

### B.1 Scope resolution and inheritance chain
Type model:
```rust
pub enum OptionScope { Server, SessionGlobal, SessionLocal, WindowGlobal, WindowLocal, PaneLocal }

pub struct OptionStore {
    pub local: std::collections::BTreeMap<String, OptionValue>,
    pub parent: Option<OptionStoreId>,
}

pub fn resolve_option(graph: &Graph, store: OptionStoreId, key: &str) -> Option<&OptionValue>;
```

tmux parity anchors:
- parent traversal semantics: `options.c:228-241`
- name+flag scope resolution: `options.c:851-918`

### B.2 Unset semantics
```rust
pub enum UnsetMode { LocalOnly, LocalAndPanes }

pub fn unset_option(
    graph: &mut Graph,
    scope: OptionScope,
    target: TargetId,
    key: &str,
    mode: UnsetMode,
) -> Result<(), ConfigError>;
```

Rules:
- local scope unset deletes local key.
- global scope unset resets to default table value.
- array index unset follows tmux indexed semantics.

tmux anchors:
- remove/default split: `options.c:1270-1285`
- `-U` pane cascade behavior path: `cmd-set-option.c:168-179`

### B.3 Config load timing and reload
Startup load timing MUST match tmux:
- load only after first client finishes identify: `server-client.c:3725-3734`
- `start_cfg` blocking logic for initial client queue: `cfg.c:65-93`
- control-mode config error emission `%config-error`: `cfg.c:227-233`, `cfg.c:251-254`

API:
```rust
pub enum ConfigReloadMode { Initial, Manual }
pub fn load_config(ctx: &mut ServerCtx, mode: ConfigReloadMode) -> Result<Vec<Event>, ConfigError>;
```

### B.4 Tests
- scope matrix tests for `-g/-s/-w/-p/-t`.
- inheritance restore test after `set -u`.
- global default restore test.
- `set -U` cascade test across panes.
- startup timing integration test: commands from first client execute after config queue barrier.

---

## C. Layout Engine

### C.1 Data model (flat arena)
```rust
pub type CellId = u32;

pub enum LayoutKind { WindowPane, LeftRight, TopBottom }

pub struct LayoutCell {
    pub kind: LayoutKind,
    pub parent: Option<CellId>,
    pub children: Vec<CellId>,
    pub sx: u16,
    pub sy: u16,
    pub xoff: u16,
    pub yoff: u16,
    pub pane_id: Option<u32>,
}

pub struct LayoutTree {
    pub cells: Vec<LayoutCell>,
    pub root: CellId,
}
```

### C.2 Checksum and wire parse/dump
```rust
pub fn layout_checksum(layout: &[u8]) -> u16;
pub fn layout_dump(tree: &LayoutTree) -> Result<String, LayoutError>;     // "hhhh,<payload>"
pub fn layout_parse(s: &str) -> Result<LayoutTree, LayoutError>;
```

tmux anchors:
- checksum algorithm: `layout-custom.c:47-57`
- string format: `layout-custom.c:69`
- parse checksum verify: `layout-custom.c:165-173`
- structure validity check: `layout-custom.c:121-153`

### C.3 Resize behavior (definitive)
```rust
pub fn resize(tree: &mut LayoutTree, request_sx: u16, request_sy: u16) -> ResizeResult;
```

Hard requirements:
- clamp to minimum possible, never return failure.
- distribution in same direction is one-cell round-robin across children, repeated until delta exhausted.

tmux anchors:
- adjust algorithm: `layout.c:422-463`
- top-level clamp logic: `layout.c:535-578`
- minimum pane size constant: `tmux.h:99-103`

### C.4 `layout_check` and compact
```rust
pub fn layout_check(tree: &LayoutTree) -> bool;
pub fn compact(tree: &mut LayoutTree);
```

- call `debug_assert!(layout_check(tree))` after every mutating op.
- compact removes unreachable cells after destroy/merge churn.

### C.5 Presets
Preset names must align with tmux names:
- `even-horizontal`, `even-vertical`, `main-horizontal`, `main-horizontal-mirrored`, `main-vertical`, `main-vertical-mirrored`, `tiled` (`layout-set.c:42-49`)

### C.6 Tests
- dump/parse roundtrip corpus.
- checksum parity against tmux-produced strings.
- resize parity snapshots (including min clamp).
- explicit round-robin distribution tests.
- preset parity tests against tmux outputs.

---

## D. OpenTelemetry

### D.1 Span namespace
All spans use `termforge.*`:
- `termforge.server.accept`
- `termforge.server.client.identify`
- `termforge.proto.decode`
- `termforge.layout.resize`
- `termforge.control.parse`
- `termforge.config.load`

### D.2 Propagation
```rust
pub trait TraceCarrier {
    fn inject(&self, map: &mut std::collections::BTreeMap<String, String>);
    fn extract(map: &std::collections::BTreeMap<String, String>) -> Self;
}
```

- propagate across spawned tasks, IPC boundaries, and binding callbacks.
- keep `mux-core` pure: no OTEL dependency in core crate.

### D.3 Filters
Default export denylist prefixes:
- `h2`, `hyper`, `tonic`, `mio`

### D.4 Tests
- structured span presence per critical op.
- parent-child linkage across API->server calls.
- purity test: `mux-core` build graph excludes OTEL crates.

---

## E. Language Bindings

### E.1 Python / Node / C++ surface
Crates:
- `bindings/python/src/lib.rs`
- `bindings/node/src/lib.rs`
- `bindings/cpp/mux.hpp`

Error mapping:
- `Transient` -> timeout/temporary exceptions
- `ProtocolViolation` -> connection/protocol exceptions
- `UserError` -> value/argument exceptions
- `Bug` -> runtime/internal exceptions

### E.2 Async phases (final)
1. sync calls with GIL release.
2. `asyncio.to_thread` convenience wrappers.
3. native async stream subscriptions only.

### E.3 Migration from `libtmux`
Preserve common `Server` ergonomics where feasible:
- `Server.cmd(...)` pattern (`server.py:240-312`)
- `Server.sessions` property (`server.py:56`, `server.py:593+`)
- session/window/pane traversal style

Compatibility wrapper type:
```rust
pub struct PyServerCompat {
    inner: OrmServer,
}

impl PyServerCompat {
    pub fn cmd(&self, cmd: &str, args: Vec<String>) -> PyResult<PyCmdResult>;
    pub fn sessions(&self) -> PyResult<Vec<PySession>>;
}
```

### E.4 Tests
- language-level exception mapping matrix.
- concurrency test with GIL release.
- subscription stream test for phase 3.
- compatibility smoke tests mirroring `libtmux` usage flows.

---

## F. CRDT

### F.1 Clock + DVV structures
```rust
pub struct NodeId([u8; 8]);

pub struct Dvv {
    pub entries: std::collections::BTreeMap<NodeId, u64>,
}

pub struct LwwRegister<T> {
    pub value: T,
    pub ts: HlcTimestamp,
}

pub struct OrSet<T> {
    adds: std::collections::BTreeMap<T, Dvv>,
    removes: std::collections::BTreeMap<T, Dvv>,
}
```

### F.2 OpLog
```rust
pub struct OpId {
    pub node: NodeId,
    pub counter: u64,
}

pub struct OpLogEntry<Op> {
    pub id: OpId,
    pub hlc: HlcTimestamp,
    pub op: Op,
}

pub struct OpLog<Op> {
    pub entries: Vec<OpLogEntry<Op>>,
}
```

### F.3 Compaction
- DVV compacts to active membership set.
- prune tombstones older than watermark + fully observed by all active peers.

### F.4 Overflow rule
- `checked_add` for HLC counter.
- on overflow: increment millis and reset counter.

### F.5 Tests
- convergence, commutativity, idempotency.
- HLC monotonicity + overflow path.
- DVV compaction boundedness by active membership count.

---

## G. Control Mode

### G.1 Wire semantics
- identify message types: `tmux-protocol.h:29-41`
- accepted identify dispatch set: `server-client.c:3385-3397`
- identify completion and control startup: `server-client.c:3689-3713`
- `%begin/%error/%end` guard generation: `cmd-queue.c:619`, `cmd-queue.c:677`, `cmd-queue.c:679`, `cmd-queue.c:825-833`
- parse-error guard path: `control.c:527-530`

### G.2 Typed notifications and migration
Current reference state uses generic notification shape in `vibe-tmux`:
- `RefreshEvent::ControlNotification { name, raw }` in `~/work/rust/vibe-tmux/crates/mux-refresh/src/event.rs:9-11`

Final v5 target uses typed enum with raw fallback:
```rust
pub enum ControlNotification {
    PaneModeChanged { pane_id: u32 },
    LayoutChange { window_id: u32, layout: String, visible_layout: String, raw_flags: String },
    WindowAdd { window_id: u32 },
    WindowClose { window_id: u32 },
    SessionChanged { session_id: u32, name: String },
    Output { pane_id: u32, data: Vec<u8> },
    ExtendedOutput { pane_id: u32, age_ms: u64, data: Vec<u8> },
    Begin { epoch: i64, number: u32, flags: i32 },
    End { epoch: i64, number: u32, flags: i32 },
    Error { epoch: i64, number: u32, flags: i32 },
    Unknown { name: String, raw: String },
}
```

Migration contract:
- parse typed first.
- fallback to `Unknown` for unrecognized `%...` lines.
- no panic on malformed input.

### G.3 Rate limiting
```rust
pub struct ControlIngressLimit {
    pub max_lines_per_sec: u32,
    pub burst: u32,
}
```

This is additive hardening; not a tmux compatibility claim.

### G.4 Tests
- parser golden tests for every known notification.
- guard block parser tests for `%begin/%end/%error`.
- malformed line fuzz tests.
- output backpressure tests.

---

## H. Server Lifecycle

### H.1 Lock file and startup discipline
Definitive rule: lock via `flock(LOCK_EX|LOCK_NB)`, not PID ownership.

tmux anchor:
- lock open+flock behavior: `client.c:72-100`

Rust API:
```rust
pub struct LockGuard {
    file: std::fs::File,
    path: std::path::PathBuf,
}

pub fn acquire_startup_lock(socket_path: &std::path::Path) -> Result<LockGuard, StartupError>;
```

### H.2 13-step startup sequence
1. Resolve socket path.
2. Acquire `.lock` with `flock`.
3. Initialize runtime/process state.
4. Init key/input caches.
5. Init global trees/queues.
6. Create/listen UNIX socket.
7. Register accept handler.
8. Initialize ACL defaults.
9. Accept first client.
10. Process identify burst.
11. Mark client identified.
12. If first client and cfg not finished: run config load.
13. Enter steady-state loop.

Anchors:
- server startup core: `server.c:176-255`
- accept + ACL join: `server.c:369-401`
- identify handling completion: `server-client.c:3591-3735`
- identify send ordering from client side: `client.c:450-494`

### H.3 Identify burst ordering (must match)
From tmux client emission order (`client.c:460-494`):
1. `MSG_IDENTIFY_LONGFLAGS` (server flags)
2. `MSG_IDENTIFY_LONGFLAGS` (client flags)
3. `MSG_IDENTIFY_TERM`
4. `MSG_IDENTIFY_FEATURES`
5. `MSG_IDENTIFY_TTYNAME`
6. `MSG_IDENTIFY_CWD`
7. repeated `MSG_IDENTIFY_TERMINFO`
8. `MSG_IDENTIFY_STDIN` (SCM_RIGHTS fd)
9. `MSG_IDENTIFY_STDOUT` (SCM_RIGHTS fd)
10. `MSG_IDENTIFY_CLIENTPID`
11. repeated `MSG_IDENTIFY_ENVIRON`
12. `MSG_IDENTIFY_DONE`

### H.4 Shutdown
- graceful path drains queues and closes clients.
- fatal protocol violations are per-client kills.
- lock released by fd close automatically.

### H.5 Tests
- startup contention lock test.
- crash-restart lock release test.
- identify ordering parity fixture tests.
- config timing test (load only after identify done).

---

## I. Security

### I.1 Validation layers
1. framing (`mux-proto`): header length/type/payload caps
2. command parse (`mux-conf`/control parser)
3. domain validation (`mux-orm`/target resolution)
4. effect boundary (`mux-server`/`mux-os`)

### I.2 Socket and ACL boundary
tmux compatibility anchors:
- socket creation and umask boundary: `server.c:107-139`
- accept-time ACL enforcement: `server.c:396-400`, `server-acl.c:164-179`

### I.3 SCM_RIGHTS constraints
- accept fds only during identify frame types expecting them.
- set `CLOEXEC` immediately.
- reject/close unexpected ancillary fds.

tmux anchor for fd intake in identify:
- `MSG_IDENTIFY_STDIN`/`STDOUT`: `server-client.c:3660-3670`

### I.4 Max clients policy
tmux itself mainly relies on OS FD limits and backoff on `EMFILE/ENFILE` (`server.c:384-387`).
TermForge adds explicit soft cap:
```rust
pub struct ClientLimits {
    pub max_clients: usize,
    pub max_control_pending_bytes: usize,
}
```

### I.5 Tests
- malformed frame fuzz tests.
- ACL deny integration test.
- SCM_RIGHTS out-of-phase rejection test.
- client-cap enforcement test.

---

## J. Performance

### J.1 Bench suite layout
- `crates/mux-proto/benches/*`: decode/encode throughput and latency.
- `crates/mux-layout/benches/*`: resize/preset complexity.
- `crates/mux-server/benches/*`: control ingestion and command dispatch.
- `crates/mux-bindings-python/benches/*`: FFI overhead.

Manifest requirement per bench crate:
```toml
[[bench]]
name = "latency"
harness = false
```

### J.2 Initial targets (conservative)
- codec decode p50 <= 2 us/frame on CI x86_64 baseline machine.
- layout resize p50 <= 50 us for 64-pane synthetic tree.
- control parse p50 <= 10 us/line.

### J.3 CI gating
- collect nightly benchmark artifacts.
- fail nightly if >130% regression against moving baseline.
- non-nightly runs only publish trend, no hard fail.

### J.4 Tests
- Criterion + Iai-callgrind for hotspots.
- fixture-based regression microbench tests.
- per-PR trend comment in CI.

---

## K. AGENTS.md Rules (all 18 rules + DO/DON'T)

### K.1 Rules (final)
1. Every library crate exposes typed `Error` enums; no `anyhow` in library APIs.
2. Every error implements `Classified` with `ErrorClass`.
3. Any protocol violation kills that connection immediately.
4. Pure/impure boundary must convert OS errors to stable domain errors.
5. Config execution produces events; no direct graph mutation from parser.
6. Option scope resolution must match tmux table+flag semantics.
7. Unset semantics: non-global remove override; global reset default; `-U` cascades panes.
8. Layout dump/parse/checksum must match tmux wire format exactly.
9. Layout resize never fails; clamp to minimum.
10. Layout resize distribution is round-robin one-cell-at-a-time, not proportional.
11. After every layout mutation, `layout_check` must hold.
12. Telemetry span names use `termforge.` prefix.
13. Trace context must propagate across spawned tasks and IPC boundaries.
14. Treat all ingress (socket/config/FFI/control) as untrusted.
15. SCM_RIGHTS accepted only during identify; set CLOEXEC immediately.
16. Control notifications are hints, never sole authority.
17. Server lock file uses `flock(LOCK_EX|LOCK_NB)`.
18. Config loading starts only after first client identify completes.

### K.2 DO
- DO mirror tmux checksum algorithm exactly.
- DO disconnect on malformed protocol frames.
- DO use round-robin resize distribution.
- DO emit typed control notifications with unknown fallback.
- DO maintain explicit source file:line anchors for compatibility behavior.

### K.3 DON'T
- DON'T use proportional resize.
- DON'T use PID-based lock semantics.
- DON'T drop bad frames and continue on same peer.
- DON'T load config before first identify completion.
- DON'T treat control notifications as authoritative state.

---

## L. Risks (17 risks with mitigations + reference verification table)

### L.1 Risk register
| ID | Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|---|
| R1 | Layout wire format divergence | Low | High | Snapshot parity vs tmux layout dump/parse corpus |
| R2 | `flock` behavior differences across OS | Medium | Medium | Linux/macOS CI coverage + docs |
| R3 | HLC counter overflow under extreme same-ms updates | Very Low | Medium | `checked_add` + forced millis advance |
| R4 | Control output backpressure memory growth | Medium | High | Per-client pending byte cap + pause/drop policy |
| R5 | Python GIL contention | Medium | Medium | Release GIL around blocking calls |
| R6 | Scope resolution complexity regressions | High | Medium | exhaustive scope/flag/target matrix tests |
| R7 | Arena dead-cell accumulation | Medium | Low | periodic compaction |
| R8 | Resize algorithm accidental proportional drift | Medium | High | dedicated round-robin parity tests |
| R9 | CRDT sync attack surface in future multi-host mode | Low | High | phase-gate network sync behind auth/TLS |
| R10 | Config parser divergence vs tmux edge grammar | Medium | Medium | corpus replay of real tmux configs |
| R11 | 16-bit checksum collisions (internal persistence misuse) | Low | Low | BLAKE3 envelope for persistence only |
| R12 | FD edge cases with extra SCM_RIGHTS descriptors | Low | Medium | close all unexpected FDs deterministically |
| R13 | Array index unset divergence | Medium | Medium | mirror tmux index behavior + parity tests |
| R14 | Missing `mux-types` crate delays layering | Certain | Medium | create as first implementation task |
| R15 | Criterion harness not wired means false confidence | Certain | Low | enforce `[[bench]] harness=false` + CI assertion |
| R16 | Control typed-notification migration breaks consumers | Medium | Medium | staged migration with raw fallback |
| R17 | Protocol limits too permissive (allocation risk) | Medium | High | strict max frame length + fuzzing |

### L.2 Reference verification table
| Claim | Status | Evidence |
|---|---|---|
| Layout checksum rotate-add | Verified | `layout-custom.c:47-57` |
| Layout parser validates checksum | Verified | `layout-custom.c:165-173` |
| Round-robin resize distribution | Verified | `layout.c:448-462` |
| Resize clamp semantics | Verified | `layout.c:535-578` |
| Minimum pane constant is 1 | Verified | `tmux.h:99-103` |
| `set -u` remove-or-default behavior | Verified | `options.c:1270-1285` |
| Option inheritance parent walk | Verified | `options.c:228-241` |
| `-U` cascades pane-local unsets | Verified | `cmd-set-option.c:168-179` |
| Control has no extra auth handshake | Verified | `control.c:759-796` |
| Socket+ACL are access boundary | Verified | `server.c:107-139`, `server.c:396-400`, `server-acl.c:164-179` |
| Protocol violation kills peer | Verified | `server-client.c:3472-3475` |
| Identify re-entry is protocol error | Verified | `server-client.c:3599-3600` |
| Config loads after first identify completion | Verified | `server-client.c:3725-3734` |
| Locking uses flock | Verified | `client.c:72-100` |
| Guard lines `%begin/%end/%error` emitted | Verified | `cmd-queue.c:619`, `cmd-queue.c:677`, `cmd-queue.c:679`, `cmd-queue.c:825-833` |
| `mux-types` crate currently absent in vibe-tmux | Verified | `~/work/rust/vibe-tmux/Cargo.toml:3-37` + crate list |
| Current control notification in vibe-tmux is generic name/raw | Verified | `~/work/rust/vibe-tmux/crates/mux-refresh/src/event.rs:9-11` |

---

## Summary of v5 changes from v4
- Formalized `ErrorClass` and enforced recovery contract (`ProtocolViolation` => disconnect).
- Locked layout wire compatibility to tmux checksum + parser behavior.
- Corrected resize algorithm to strict round-robin one-cell distribution.
- Finalized option unset semantics with global/default and `-U` pane cascade.
- Finalized control-mode security posture (socket+ACL only) plus optional ingress limiter.
- Finalized lifecycle sequence: `flock` lock and config load after first identify completion.
- Added CRDT clock safety rules (HLC overflow handling + DVV compaction bounds).
- Added binding rollout phases and explicit `libtmux` migration surface.
- Added benchmark wiring requirements and realistic CI regression policy.
- Added 18 AGENTS rules, explicit DO/DON'T list, and 17-risk register with mitigations.
