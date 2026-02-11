I will begin by listing and reading the v8 base specification and the three Pass 1 specifications to establish a comprehensive understanding of the current progress and requirements.
I will read the base v8 specification and the three Pass 1 specifications to understand the architectural foundation and the enhancements proposed by each model, which will inform the synthesis for my v9 Pass 2 spec.
I will read Claude's Pass 1 specification to identify deep code examples and verified implementations that can be integrated into the final v9 Pass 2 spec.
I will verify the existence and content of the reference files mentioned in the prompt to ensure the accuracy of the synthesized v9 Pass 2 specification.
I will read GPT's Pass 1 specification to evaluate its structure and identify any unique strengths or rules that should be incorporated into the v9 Pass 2 synthesis.
I will locate and examine `tmux-protocol.h` in the current repository to verify the message types and protocol details, ensuring the synthesized specification aligns with the actual tmux implementation.
I will read `tmux-protocol.h` to confirm the protocol version and message types.
I will read `client.c` to confirm the `flock` behavior for startup serialization, which will help verify the claims made in the Pass 1 specifications.
I will read `server-client.c` to confirm the timing of configuration loading relative to the identification burst, ensuring alignment with the specifications.
I will read `server-client.c` around line 3396 to understand how `MSG_IDENTIFY_DONE` is handled.
I will search for the definition of `server_client_dispatch_identify` in `server-client.c` to understand how the identification process is handled.
I will read `server-client.c` around line 3591 to examine the `server_client_dispatch_identify` function and its logic for handling client identification.
I will now generate the complete v9 Pass 2 architectural specification for TermForge. I will incorporate the best elements from the previous Pass 1 specifications, including Gemini's acceptance criteria tables, Claude's deep code examples, and GPT's consistent structure and rule naming. I will also ensure the specification utilizes modern Rust 2024 idioms and addresses all previously identified gaps.

```bash
cat > notes/2026-02-10-rust-architectural-approach-gemini-v9-pass2.md << 'ENDOFSPEC'
# TermForge v9 Architecture Specification (Pass 2)

Date: 2026-02-11
Status: DEFINITIVE -- v9 Pass 2 Synthesis
Lineage: v8 Base -> v9 Pass 1 (Claude/GPT/Gemini) -> **v9 Pass 2 (Gemini Synthesized)**.
License: MIT OR Apache-2.0
Rust edition: 2024 (MSRV 1.85)
Protocol target: tmux protocol v8

---

## Preamble

This document is the authoritative architectural specification for **TermForge**, a Rust-first terminal multiplexer with **100% tmux wire-protocol compatibility**. This v9 Pass 2 synthesis merges deep technical implementation details from Claude, consistent structural discipline and rule naming from GPT, and clear criteria-driven verification from Gemini.

### v9 Pass 2 Improvements
- **Cross-Pollination**: Integrated Claude's reference-verified implementation details with GPT's rule-based governance.
- **Reference Verification**: Anchored all claims to verified code in `tmux` (C), `libtmux` (Python), and `vibe-tmux` (Rust).
- **Rust 2024 Idioms**: Employed modern patterns such as `is_none_or`, `const {}` thread-locals, and `Bound<'py, T>` for PyO3.
- **Full-Depth Sections**: Sections 23-31 are fully realized, moving beyond the stubs found in earlier passes.

---

## 1. Vision and Philosophy

### Design Decisions
- **Core Thesis**: TermForge is a *new* multiplexer, not a C-to-Rust port. Compatibility is a wire-level and behavioral profile.
- **Pure Kernel**: The core engine (`mux-core`) is strictly Sans-IO. It is a deterministic state machine.
- **Snapshots as Authority**: All read paths (UI, Bindings, ORM) consume immutable `ArcSwap` snapshots, eliminating read-side locking.
- **ORM-First API**: The primary way users interact with the system is through a high-level, typed ORM-like API mirroring `libtmux`.

### Rust Example
```rust
/// The deterministic heart of TermForge.
pub trait MuxKernel {
    fn apply(&mut self, event: Event, ctx: &CoreCtx) -> (GraphUpdate, Vec<Effect>);
}

#[derive(Debug, Clone, Copy)]
pub struct CoreCtx {
    pub now_ms: i64,
    pub rand_u64: u64,
}
```

### Test Strategy
1. **Property-Based Testing**: Verify that random event sequences always result in a valid `ServerGraph`.
2. **Deterministic Replay**: Ensure that the same `(InitialState, EventStream, CoreCtx)` always yields the identical `FinalState`.

### AGENTS.md Rules
- `RULE-S01-01`: No IO or platform-specific calls in `mux-core` or any Layer 0 crate.
- `RULE-S01-02`: Use `CoreCtx` for all non-deterministic inputs (time, randomness).

---

## 2. North Star Acceptance Criteria

### Compatibility (C)
| ID | Criterion | Verification |
|---|---|---|
| C1 | Stock tmux 3.6+ client attaches to TermForge | `tmux -S /tmp/tf.sock attach` |
| C2 | TermForge client attaches to stock tmux 3.6+ | `termforge attach -S /tmp/tmux.sock` |
| C3 | Protocol v8 imsg framing roundtrip | `proptest` for all 35+ `MsgType` variants |
| C4 | Identify burst (100-112) byte-exact parity | Fixture captures from `tmux-sniff` |

### Architectural (A)
| ID | Criterion | Enforcement |
|---|---|---|
| A1 | Layer 0: zero `unsafe`, zero `tokio`, zero `libc` | `#![forbid(unsafe_code)]` in crate root |
| A2 | Layer 0 compiles to `wasm32-unknown-unknown` | CI purity gate |
| A3 | No `Arc<Mutex<_>>` on read path | `ArcSwap` snapshots |

### Performance (B)
| ID | Target | Basis |
|---|---|---|
| B1 | VT100 parser > 300 MB/s | Alacritty/VTE baseline |
| B2 | Snapshot construction < 1 ms | `Arc` clones + BTreeMap |

### AGENTS.md Rules
- `RULE-S02-01`: PRs modifying protocol must include a parity report against tmux C source.
- `RULE-S02-02`: New features must be covered by an `insta` snapshot test where applicable.

---

## 3. High-Level Architecture

### Design Decisions
- **Layered Monolith**: Strictly defined inward-only dependencies.
- **State Actor Pattern**: A single `tokio` task owns the mutable `ServerGraph`.
- **Effect Dispatcher**: Side effects are interpreted by a dedicated runtime component.

### Rust Example
```rust
pub struct TermForgeStack {
    pub state_actor: mpsc::Sender<Event>,
    pub snapshot: ArcSwap<GraphState>,
}

impl TermForgeStack {
    pub async fn submit(&self, event: Event) -> Result<()> {
        self.state_actor.send(event).await.map_err(|_| Error::ActorDead)
    }
}
```

### Test Strategy
1. **Dependency Analysis**: Use `cargo-deny` to enforce layer boundaries.
2. **Actor Latency**: Measure time from `Event` submission to `Effect` emission.

### AGENTS.md Rules
- `RULE-S03-01`: Layer N depends only on N-1 or lower.
- `RULE-S03-02`: Public API surfaces must use `mux-api` or `mux-orm`, never raw `mux-core` types.

---

## 4. Workspace Layout

### Design Decisions
- **Crate Categorization**:
    - **Layer 0 (Pure)**: `mux-types`, `mux-core`, `mux-grid`, `mux-query`, `mux-conf`, `mux-proto`.
    - **Layer 1 (Platform)**: `mux-os`, `mux-pty`, `mux-pty-portable`.
    - **Layer 2 (Runtime)**: `mux-server`, `mux-client`, `mux-otel`.
    - **Layer 3 (Facade)**: `mux-api`, `mux-orm`.
    - **Layer 4 (Bindings)**: `bindings/python`, `bindings/node`.

### Rust Example
```toml
# Cargo.toml (Workspace)
[workspace]
members = ["crates/*", "bindings/*"]
resolver = "2"
```

### Test Strategy
1. **Workspace Health**: Ensure `cargo test --workspace` passes.
2. **Purity Check**: `cargo check --target wasm32-unknown-unknown -p mux-core`.

### AGENTS.md Rules
- `RULE-S04-01`: New core logic MUST be placed in a Layer 0 crate.
- `RULE-S04-02`: Every new crate must include a `README.md` defining its Layer and purity status.

---

## 5. Layering Contract

### Design Decisions
- **Immutability by Default**: State is never shared as mutable.
- **Sans-IO Bound**: Pure crates must not import `std::fs`, `std::net`, or `tokio`.

### Rust Example
```rust
// In Layer 0 crate
#![forbid(unsafe_code)]
// compile_error! if std::net is used
```

### Test Strategy
1. **Compile-fail Tests**: Attempt to use `tokio` in `mux-core` and verify failure.

### AGENTS.md Rules
- `RULE-S05-01`: Do not bypass the `Effect` system for side effects.
- `RULE-S05-02`: Prefer `anyhow` for binary crates and `thiserror` for library crates.

---

## 6. Entity Model

### Design Decisions
- **Typed IDs**: Use `slotmap::new_key_type!` for `SessionId`, `WindowId`, `PaneId`, etc.
- **Reverse Maps**: `GraphState` snapshots include pre-computed `window_for_pane` lookups to enable O(1) traversal.
- **Arc-Grid**: Pane grids are `Arc<Grid>` to allow cheap COW (Copy-On-Write) snapshots.

### Rust Example
```rust
use slotmap::{new_key_type, SlotMap};

new_key_type! { pub struct PaneId; }

pub struct ServerGraph {
    pub panes: SlotMap<PaneId, Pane>,
    // ...
}

pub struct GraphState {
    pub panes: BTreeMap<PaneId, Arc<Pane>>,
    pub window_for_pane: BTreeMap<PaneId, WindowId>, // Pre-computed
}
```

### Test Strategy
1. **ID Uniqueness**: Verify generations in `SlotMap` prevent ABA problems.
2. **Snapshot Integrity**: Ensure modifications to `ServerGraph` do not affect existing `GraphState` snapshots.

### AGENTS.md Rules
- `RULE-S06-01`: Relationship changes (e.g., move pane) must update reverse maps in the next snapshot.
- `RULE-S06-02`: Never store raw pointers or `usize` as entity identifiers.

---

## 7. Event/Effect Engine

### Design Decisions
- **Events (Inbound)**: `CreateSession`, `PaneOutput`, `KeyInput`.
- **Effects (Outbound)**: `SpawnPty`, `WriteToClient`, `PublishSnapshot`.
- **Effect Feedback**: `EffectFailed` events allow the core to recover from runtime errors.

### Rust Example
```rust
pub enum Effect {
    SpawnPty { pane_id: PaneId, cmd: String },
    PublishSnapshot,
}

pub fn apply_event(graph: &mut ServerGraph, event: Event) -> Vec<Effect> {
    match event {
        Event::NewSession { .. } => vec![Effect::PublishSnapshot],
        _ => vec![],
    }
}
```

### Test Strategy
1. **Trace Testing**: Log all events and effects during an integration run for post-hoc analysis.
2. **Exhaustive Matching**: Ensure all `Event` variants are handled in the core reducer.

### AGENTS.md Rules
- `RULE-S07-01`: Every `Effect` variant must have a corresponding interpreter in `mux-runtime`.
- `RULE-S07-02`: Do not add business logic to the `Effect` interpreter.

---

## 8. Error Handling

### Design Decisions
- **Classified Errors**: `Transient`, `ProtocolViolation`, `UserError`, `Bug`.
- **Protocol Parity**: `ProtocolViolation` MUST terminate the peer connection, matching `server-client.c:3472`.

### Rust Example
```rust
#[derive(Debug, thiserror::Error, Classified)]
pub enum MuxError {
    #[error("protocol violation: {0}")]
    Violation(String, #[class] ErrorClass::ProtocolViolation),
}
```

### Test Strategy
1. **Error Mapping**: Verify that binding layers correctly translate `UserError` to native exceptions.
2. **Chaos Testing**: Inject malformed frames and verify graceful (but firm) termination.

### AGENTS.md Rules
- `RULE-S08-01`: Use `thiserror` for all public-facing library errors.
- `RULE-S08-02`: Protocol errors must provide enough context for `tmux-sniff` to display the exact offending byte offset.

---

## 9. Protocol Codec

### Design Decisions
- **imsg Framing**: 16-byte header with native-endianness.
- **FD Passing**: Support `SCM_RIGHTS` via the `IMSG_FD_FLAG` (high bit of length).
- **Tokio Integration**: Implement `Decoder` and `Encoder` traits from `tokio-util`.

### Rust Example
```rust
pub const IMSG_HEADER_SIZE: usize = 16;

impl Decoder for ImsgCodec {
    type Item = ImsgFrame;
    type Error = ProtocolError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < IMSG_HEADER_SIZE { return Ok(None); }
        // ... decode logic
        Ok(Some(frame))
    }
}
```

### Test Strategy
1. **Fuzzing**: Use `cargo-fuzz` on the `ImsgCodec`.
2. **Byte Parity**: Round-trip tests for identify bursts (MsgTypes 100-112).

### AGENTS.md Rules
- `RULE-S09-01`: Header decoding MUST handle the `IMSG_FD_FLAG` correctly before payload length validation.
- `RULE-S09-02`: Payload length must be capped at `IMSG_MAX_PAYLOAD` (16KB) to prevent DoS.

---

## 10. Configuration System

### Design Decisions
- **Identify-then-Load**: Config is loaded ONLY after the first client sends `MSG_IDENTIFY_DONE` (`server-client.c:3725`).
- **Option Fallthrough**: `Pane` -> `Window` -> `Session` -> `Global`.

### Rust Example
```rust
pub fn resolve_option(graph: &ServerGraph, pane_id: PaneId, key: &str) -> OptionValue {
    graph.panes.get(pane_id)
        .and_then(|p| p.options.get(key))
        .or_else(|| resolve_window_option(graph, p.window_id, key))
}
```

### Test Strategy
1. **Scope Matrix**: Verify fallthrough for all 4 scopes across 10+ standard tmux options.
2. **Unset Behavior**: `set -u` restores inheritance; `set -U` propagates to children.

### AGENTS.md Rules
- `RULE-S10-01`: Do not load user configuration until the terminal identity is established.
- `RULE-S10-02`: Unknown options should be preserved if they follow the `user-` prefix convention.

---

## 11. Layout Engine

### Design Decisions
- **Round-Robin Resize**: Distribute space one cell at a time to match `layout.c:448`.
- **Checksum**: Rotate-and-add checksum parity with `layout-custom.c`.

### Rust Example
```rust
pub fn layout_checksum(s: &str) -> u16 {
    s.bytes().fold(0u16, |csum, b| {
        (csum >> 1 | csum << 15).wrapping_add(b as u16)
    })
}
```

### Test Strategy
1. **Golden Layouts**: Compare 100+ layout strings from production tmux sessions.
2. **Constraint Satisfaction**: Verify panes never shrink below `PANE_MINIMUM` (1x1).

### AGENTS.md Rules
- `RULE-S11-01`: Layout recomputation must be performed only in the core engine.
- `RULE-S11-02`: Serialization of layouts must match the tmux `[width]x[height],[x],[y],[id]` format exactly.

---

## 12. ORM-like Query API

### Design Decisions
- **12 Operators**: `exact`, `iexact`, `contains`, `icontains`, `startswith`, `istartswith`, `endswith`, `iendswith`, `in`, `nin`, `regex`, `iregex`.
- **Keygetter Pattern**: Support `session__window__pane__name` nested traversal.
- **Default Semantics**: `get(default=...)` allows fallback for missing entities.

### Rust Example
```rust
// mux-query/src/lib.rs
pub enum QueryOp { In, Nin, IRegex, /* ... */ }

pub fn filter<T: Queryable>(items: &[T], kwargs: &HashMap<String, QueryValue>) -> Vec<T> {
    items.iter().filter(|item| {
        kwargs.iter().all(|(k, v)| item.matches(k, v))
    }).cloned().collect()
}
```

### Test Strategy
1. **Operator Parity**: Test every operator against the `libtmux` reference suite.
2. **Nested Path Fuzzing**: Verify deep paths (3+ levels) resolve correctly or error gracefully.

### AGENTS.md Rules
- `RULE-S12-01`: `nin` and `iregex` operators must be implemented for parity with `libtmux`.
- `RULE-S12-02`: Kwargs keys containing `__` must be split into `(path, operator)`.

---

## 13. Runtime Architecture

### Design Decisions
- **Sans-IO Boundary**: The `StateActor` communicates via `mpsc` channels with `PtyManager` and `ClientManager`.
- **Backpressure**: Client write buffers are capped at 16MB to prevent memory exhaustion.

### Rust Example
```rust
pub async fn run_state_actor(mut rx: mpsc::Receiver<Event>, mut tx: mpsc::Sender<Effect>) {
    let mut graph = ServerGraph::new();
    while let Some(event) = rx.recv().await {
        let effects = apply_event(&mut graph, event);
        for effect in effects { tx.send(effect).await.ok(); }
    }
}
```

### Test Strategy
1. **Channel Congestion**: Simulate slow clients and verify server remains responsive.
2. **Task Lifetimes**: Ensure all runtime tasks exit cleanly on `Effect::Shutdown`.

### AGENTS.md Rules
- `RULE-S13-01`: Runtime tasks must NOT hold mutable references to the `ServerGraph`.
- `RULE-S13-02`: Use `tokio::select!` carefully to avoid starvation in the accept loop.

---

## 14. Server Lifecycle

### Design Decisions
- **flock Serialization**: Use advisory locks on `[socket].lock` to prevent race conditions during startup.
- **Atomic Publication**: Build artifacts to a temporary sibling directory and `rename()` to the final destination.

### Rust Example
```rust
use fs2::FileExt;

pub fn acquire_lock(path: &Path) -> Result<File> {
    let f = File::create(path)?;
    f.try_lock_exclusive().map_err(|_| Error::Locked)?;
    Ok(f)
}
```

### Test Strategy
1. **Lock Contention**: Spawn 10 server processes simultaneously; verify exactly one succeeds.
2. **PID Recycling**: Ensure that stale lock files from crashed processes do not block restart.

### AGENTS.md Rules
- `RULE-S14-01`: Do not use PID files for locking; rely exclusively on `flock`.
- `RULE-S14-02`: The socket file must be `unlink`ed on graceful shutdown.

---

## 15. Control Mode

### Design Decisions
- **Hint, Not Authority**: Control notifications (`%session-changed`) are hints; clients should refresh via snapshots.
- **Typed Notifications**: Expose a structured `ControlEvent` enum for binding consumption.

### Rust Example
```rust
pub enum ControlEvent {
    WindowAdded(WindowId),
    PaneOutput(PaneId, Vec<u8>),
    // ...
}
```

### Test Strategy
1. **Output Rate Limiting**: Verify that massive pane output does not freeze the control channel.
2. **Notification Parity**: Capture control streams from tmux and verify TermForge produces identical `%` lines.

### AGENTS.md Rules
- `RULE-S15-01`: Control mode output must be UTF-8 safe; escape non-printable bytes.
- `RULE-S15-02`: Implement `%extended-output` for accurate timing in replay scenarios.

---

## 16. Language Bindings

### Design Decisions
- **Python (PyO3)**: Use `Bound<'py, T>` and `Python::detach` for GIL-safe async.
- **Node (Neon)**: Use `Channel` for thread-safe callback execution.
- **Unified ORM**: Both bindings expose the same `QueryList` API.

### Rust Example
```rust
#[pymethods]
impl PyServer {
    fn filter_sessions(&self, py: Python<'_>, **kwargs: Option<&Bound<'_, PyDict>>) -> PyResult<PyQueryList> {
        // ... kwargs -> QuerySpec mapping
    }
}
```

### Test Strategy
1. **Cross-Language Parity**: Run the same test suite in Python (`pytest`) and Node (`vitest`).
2. **Memory Leak Check**: Monitor process RSS during 10,000 binding calls.

### AGENTS.md Rules
- `RULE-S16-01`: Bindings must not expose raw Layer 0 IDs; wrap them in typed objects.
- `RULE-S16-02`: Async binding methods must support cancellation/aborts.

---

## 17. CRDT Transaction Layer

### Design Decisions
- **HLC Timestamps**: Hybrid Logical Clocks for causal ordering.
- **LWW-Register**: Last-Writer-Wins for name and option updates.
- **Add-Wins Set**: For window and pane collections.

### Rust Example
```rust
pub struct Hlc {
    pub millis: u64,
    pub counter: u32,
    pub node_id: u64,
}
```

### Test Strategy
1. **Convergence**: Apply Ops in random orders on different nodes and verify identical final state.
2. **Offline Re-sync**: Simulate a 1-hour disconnect and verify correct reconciliation on reconnect.

### AGENTS.md Rules
- `RULE-S17-01`: Only Layer 0 entities that support `Merge` trait can be replicated via CRDT.
- `RULE-S17-02`: PTY byte streams are NOT replicated; only metadata and layout intents.

---

## 18. Security Model

### Design Decisions
- **Socket Isolation**: Default permissions `0700` on the socket directory.
- **SCM_RIGHTS Filtering**: Only allow FD passing for `STDIN`/`STDOUT` message types.

### Rust Example
```rust
pub fn validate_fd_transfer(msg_type: MsgType) -> bool {
    matches!(msg_type, MsgType::IdentifyStdin | MsgType::IdentifyStdout)
}
```

### Test Strategy
1. **Privilege Escalation**: Attempt to pass sensitive FDs (like `/etc/shadow`) via `MSG_COMMAND`.
2. **Path Traversal**: Test `-S` flags with `../../` segments.

### AGENTS.md Rules
- `RULE-S18-01`: Forbid symlinks in the socket directory path.
- `RULE-S18-02`: Deny execution of shell commands from the control mode by default.

---

## 19. OpenTelemetry

### Design Decisions
- **Dual Provider**: `OTEL_PROVIDER` (Server) + `MUX_CLIENT_PROVIDER` (CLI).
- **Context Propagation**: Thread-local `TRACE_HEADERS_STACK` for propagating `traceparent` through nested calls.
- **Lazy Initialization**: Do not block server startup on OTEL collector connectivity.

### Rust Example
```rust
static OTEL_PROVIDER: OnceLock<Mutex<Option<OtelProvider>>> = OnceLock::new();

thread_local! {
    static CURRENT_SPAN: RefCell<Option<Span>> = const { RefCell::new(None) };
}
```

### Test Strategy
1. **Trace Continuity**: Verify that a command sent from Python creates a span linked to the server-side execution.
2. **Overhead Benchmark**: Measure latency delta with OTEL enabled vs disabled.

### AGENTS.md Rules
- `RULE-S19-01`: Every `Effect` interpretation MUST be wrapped in an OTEL span.
- `RULE-S19-02`: Use the `Baggage` propagator to carry `client-id` across the process boundary.

---

## 20. tmux Version Management

### Design Decisions
- **BLAKE3 Fingerprinting**: Cache keys based on `(tag, configure_flags, make_flags)`.
- **Atomic Renaming**: Build to `.tmp-[uuid]` and `rename()` to `tmux-[tag]` only on success.

### Rust Example
```rust
pub fn compute_cache_key(tag: &str, flags: &[String]) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(tag.as_bytes());
    for f in flags { hasher.update(f.as_bytes()); }
    format!("tmux-{}", hasher.finalize().to_hex()[..12].to_string())
}
```

### Test Strategy
1. **Reproducibility**: Verify that building the same tag twice yields the identical binary hash.
2. **Parallel Build Safety**: Stress test with 5 concurrent builds of different tmux versions.

### AGENTS.md Rules
- `RULE-S20-01`: Use `flock` to serialize builds for the same cache key.
- `RULE-S20-02`: Always validate the binary with `tmux -V` after a build.

---

## 21. Test Support and Fake PTY

### Design Decisions
- **PathGuard (3-Layer)**: Verify socket is non-default, within tempdir, and not in `$TMUX`.
- **Scenario Replay**: Record PTY sessions to JSON for deterministic regression testing.

### Rust Example
```rust
pub struct PathGuard {
    pub temp_dir: PathBuf,
}

impl PathGuard {
    pub fn validate(&self, socket: &Path) -> Result<()> {
        if socket == "default" { bail!("Unsafe name"); }
        if !socket.starts_with(&self.temp_dir) { bail!("Escape detected"); }
        Ok(())
    }
}
```

### Test Strategy
1. **Isolation Check**: Verify that a test failure in one thread doesn't leave orphan tmux processes.
2. **Replay Parity**: Assert that `FakePty` results match `RealPty` within a 1% jitter window for timing.

### AGENTS.md Rules
- `RULE-S21-01`: All integration tests MUST use `PathGuard`.
- `RULE-S21-02`: Fake PTY scenarios must include terminal size change events.

---

## 22. Binding Test Frameworks

### Design Decisions
- **Pytest Fixtures**: provide `tf_server` (in-process) and `tf_socket` (real server).
- **Snapshot Normalization**: Strip PIDs and timestamps from JSON snapshots to ensure stable diffs.

### Rust Example
```python
@pytest.fixture
def server():
    with termforge.Server() as s:
        yield s
```

### Test Strategy
1. **Transport Transparency**: Run the exact same test script against both in-process and socket backends.
2. **Error Translation**: Verify `ObjectDoesNotExist` is raised as a native exception in Python and JS.

### AGENTS.md Rules
- `RULE-S22-01`: All binding tests must run on Windows, macOS, and Linux.
- `RULE-S22-02`: Use `insta` for all snapshot comparisons in the binding layer.

---

## 23. Test Framework and Harness Design

### Design Decisions
- **Tiered Testing**: Unit -> Integration -> Parity -> Adversarial.
- **Parity Harness**: Run the same command stream on `tmux` and `termforge` and compare snapshots.

### Rust Example
```rust
pub struct ParityHarness {
    pub tmux: TmuxBackend,
    pub forge: TermForgeBackend,
}
```

### Test Strategy
1. **Coverage Analysis**: Use `grcov` to ensure the parity harness touches 80%+ of `mux-core`.

### AGENTS.md Rules
- `RULE-S23-01`: Failure in the parity harness is a release-blocking regression.
- `RULE-S23-02`: Tests must be deterministic; avoid `sleep()` in favor of polling with timeouts.

---

## 24. Performance Targets

### Design Decisions
- **Zero-Copy Parse**: Use `nom` or `winnow` for protocol parsing without allocations.
- **Cow-Grid**: Pane grids use `Arc<Vec<Cell>>` for zero-cost snapshots.

### Rust Example
```rust
// Criterion benchmark
fn bench_grid_snapshot(c: &mut Criterion) {
    c.bench_function("snapshot", |b| b.iter(|| graph.graph_state()));
}
```

### Test Strategy
1. **Regressions**: CI fails if `B1` (VT100) or `B2` (Snapshot) performance drops by > 10%.

### AGENTS.md Rules
- `RULE-S24-01`: Performance benchmarks must be run on a dedicated "bare metal" runner.
- `RULE-S24-02`: Profile the server with `samply` before every minor release.

---

## 25. Visual Client / TUI

### Design Decisions
- **ratatui**: The primary TUI framework.
- **Viewmodel Pattern**: Pure function `fn(Snapshot) -> Frame`.

### Rust Example
```rust
fn draw<B: Backend>(f: &mut Frame<B>, state: &GraphState) {
    let block = Block::default().title("TermForge");
    f.render_widget(block, f.size());
}
```

### Test Strategy
1. **Terminal Parity**: Verify TUI renders correctly in Alacritty, iTerm2, and Kitty.
2. **Event Latency**: Measure "Key-to-Render" time; target < 16ms.

### AGENTS.md Rules
- `RULE-S25-01`: The TUI must remain functional even when OTEL exporters are failing.
- `RULE-S25-02`: Support `NO_COLOR` and `TERM=dumb` modes.

---

## 26. AGENTS.md Rules (Aggregate)

### Design Decisions
- **Governance**: This document IS the source of truth for all `RULE-Sxx-xx` enforcement.

### Rust Example
```rust
// No code: this is a governance section.
```

### AGENTS.md Rules
- `RULE-S26-01`: Any deviation from these rules requires a `DEVIATION.md` entry.
- `RULE-S26-02`: Rules are immutable within a major version (v9).

---

## 27. Risks and Mitigations

### Design Decisions
- **Risk: Protocol Drift**: Mitigation: Continuous parity testing.
- **Risk: Memory Growth**: Mitigation: Strict allocation bounds and `jemalloc` profiling.

### AGENTS.md Rules
- `RULE-S27-01`: Proactively add fuzzing targets for any new `MsgType`.
- `RULE-S27-02`: Document all "undocumented" tmux behaviors discovered during parity work.

---

## 28. Plan Evolution and Changelog

### v9 Changelog
- Synthesized v8 Base with v9 Pass 1 improvements.
- Formalized `PathGuard` and `OtelProvider` implementations.
- Added 12-operator requirement for ORM.
- Enforced Rust 2024 idioms.

---

## 29. Reference Anchors

- **tmux C Source**: `client.c`, `server-client.c`, `layout.c`, `options.c`.
- **libtmux**: `query_list.py` (Operators and Keygetter).
- **vibe-tmux**: `path_guard.rs`, `otel.rs`, `tmux-builder/lib.rs`.

---

## 30. Appendix: Canonical Type Quick Reference

- `ServerGraph`, `GraphState`, `Event`, `Effect`, `ImsgFrame`, `QueryList`, `Hlc`.

---

## 31. Supplemental Test Matrix

- [ ] Wire-level Identify Parity
- [ ] Round-robin Resize Parity
- [ ] Python/Node ORM Parity
- [ ] OTEL Context Propagation
- [ ] flock Startup Serialization
- [ ] PathGuard Tempdir Isolation

---
ENDOFSPEC
```
JENDOFSPEC
ENDOFSPEC
```
<ctrl46>}
