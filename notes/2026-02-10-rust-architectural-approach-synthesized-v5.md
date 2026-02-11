# TermForge v5 Final Specification

Date: 2026-02-10
Pass: 3 of 3 (Final synthesis of Claude/Gemini/GPT Pass 2 outputs)
License: MIT OR Apache-2.0
Rust edition: 2024 (MSRV 1.85)

---

## Grounding Statement

This is the definitive v5 architecture document for TermForge, a Rust terminal multiplexer with 100% tmux wire-protocol compatibility. It synthesizes three independent Pass 2 refinements (Claude 1632 lines, Gemini 184 lines, GPT 190 lines), verified against tmux C source and the vibe-tmux Rust prototype.

**Repository locations:**
- tmux C source: `~/study/c/tmux/`
- Rust prototype: `~/work/rust/vibe-tmux/`
- Python libtmux: `~/work/python/libtmux/`

**Settled v4 decisions (not re-argued):**
1. Vec-inside-entity (not SecondaryMap)
2. WASM compilability as CI purity gate
3. `mux-types` as separate leaf crate
4. `mux-orm` as separate crate from `mux-api`
5. FakePty ScenarioRecorder with JSON
6. Defer `im-rs`; use `Arc<Grid>` with `Arc::make_mut`
7. Custom VT100 parser matching tmux `input.c` (17 states)
8. Format string engine as pure function
9. Key binding system with table-based dispatch
10. Copy mode as per-pane `CopyModeState`

---

## Table of Contents

0. [Open Questions Resolved (Q1--Q7)](#0-open-questions-resolved)
A. [Error Handling](#a-error-handling)
B. [Configuration System](#b-configuration-system)
C. [Layout Engine](#c-layout-engine)
D. [OpenTelemetry](#d-opentelemetry)
E. [Language Bindings](#e-language-bindings)
F. [CRDT Detail](#f-crdt-detail)
G. [Control Mode](#g-control-mode)
H. [Server Lifecycle](#h-server-lifecycle)
I. [Security Model](#i-security-model)
J. [Performance Targets](#j-performance-targets)
K. [AGENTS.md Rules](#k-agentsmd-rules)
L. [Risks and Mitigations](#l-risks-and-mitigations)
[Summary of v5 Changes from v4](#summary-of-v5-changes-from-v4)

---

## 0. Open Questions Resolved

All three Pass 2 outputs converged on the same decisions for all seven questions. Where implementation details differed, the resolution is noted.

### Q1: Layout String Checksum

**Decision:** Match tmux's exact rotate-and-add algorithm for all wire-format paths. Add an optional BLAKE3-128 envelope for internal persistence only (never sent to tmux clients).

**Verified source:**

```c
// ~/study/c/tmux/layout-custom.c:46-57
static u_short
layout_checksum(const char *layout)
{
    u_short csum;
    csum = 0;
    for (; *layout != '\0'; layout++) {
        csum = (csum >> 1) + ((csum & 1) << 15);
        csum += *layout;
    }
    return (csum);
}
```

The checksum is emitted as `%04hx` at `layout-custom.c:69`. Parsing rejects mismatches at `layout-custom.c:165-173` (verified: `sscanf` extracts 4-hex prefix, compares against computed checksum of remaining string).

**Rust implementation:**

```rust
/// Layout wire format variants.
pub enum LayoutWireFormat {
    /// "hhhh,<layout>" -- exact tmux compatibility.
    TmuxV1,
    /// Internal envelope with BLAKE3-128 for persistence only.
    TermForgeV2,
}

/// Layout checksum matching tmux exactly.
/// Reference: layout-custom.c:46-57
pub fn layout_checksum(layout: &[u8]) -> u16 {
    let mut csum: u16 = 0;
    for &b in layout {
        csum = (csum >> 1) | ((csum & 1) << 15);
        csum = csum.wrapping_add(b as u16);
    }
    csum
}

/// Internal envelope for persistence (never sent to tmux).
pub struct LayoutEnvelopeV2 {
    pub version: u16,
    pub tmux_payload: String,
    pub blake3_128: [u8; 16],
}
```

**Test strategy:**
1. Property test: for any valid layout string, `layout_checksum(s) == tmux_checksum(s)` using a corpus extracted from real tmux sessions.
2. Roundtrip: `assert_eq!(layout_parse(layout_dump(tree)).unwrap(), tree)`.
3. Parity: capture layout strings from tmux via control mode, verify our checksum matches.

---

### Q2: Option Unset Semantics

**Decision:** `set -u` removes the local override, restoring inheritance. For global scopes, it resets to the compiled default. `set -U` additionally unsets pane-local values in the target window.

**Verified source:**

`options_remove_or_default` at `options.c:1269-1285`:
- For global options (`oo == global_options || global_s_options || global_w_options`): calls `options_default()` to reset to compiled default (line 1279).
- For non-global: calls `options_remove()` to delete the local entry (line 1281).
- Array index unset: delegates to `options_array_set(o, idx, NULL, 0, cause)` (line 1282).

`options_get` at `options.c:228-241` walks the `parent` chain. After `options_remove()`, the next `options_get()` finds the inherited value from the parent.

**Scope resolution** follows `options_scope_from_name()` at `options.c:850-919`, which includes a critical `FALLTHROUGH` at line 903: options declared as `OPTIONS_TABLE_WINDOW|OPTIONS_TABLE_PANE` fall through to `WINDOW` handling when `-p` is not specified.

**Rust implementation:**

```rust
pub enum UnsetMode {
    /// Remove local override at target scope only.
    LocalOnly,
    /// Remove local override AND pane-local values in target window.
    LocalAndPanes,
}

impl OptionStore {
    /// Remove a local override. After this, resolve walks to parent.
    pub fn unset(&mut self, key: &str) -> Option<OptionValue> {
        self.local.remove(key)
    }
}

pub fn unset_option(
    graph: &mut ServerGraph,
    scope: OptionScope,
    target_id: TargetId,
    key: &str,
    mode: UnsetMode,
) {
    match scope {
        OptionScope::Server => {
            if let Some(default) = option_table_default(key) {
                graph.global_options.set(key, default);
            }
        }
        OptionScope::Session if matches!(target_id, TargetId::Server) => {
            if let Some(default) = option_table_default(key) {
                graph.global_session_options.set(key, default);
            }
        }
        _ => {
            if let Some(store) = get_option_store_mut(graph, scope, target_id) {
                store.unset(key);
            }
            if mode == UnsetMode::LocalAndPanes {
                if let TargetId::Window(wid) = target_id {
                    for pane_id in graph.panes_in_window(wid) {
                        if let Some(pane) = graph.pane_mut(pane_id) {
                            pane.options.unset(key);
                        }
                    }
                }
            }
        }
    }
}
```

**Test strategy:**
1. Set option on pane, verify shadow. Unset, verify parent visible.
2. Set on global session scope, unset, verify compiled default restored.
3. Set pane-local option on 3 panes, `set -U` on window, verify all removed.
4. Property test: `resolve_option` always returns a value for known options.
5. Parity test: compare `set -u` behavior between tmux and TermForge.

---

### Q3: Control Mode Authentication

**Decision:** No additional authentication beyond socket permissions + ACL. Add configurable rate limiting for control mode command ingestion.

**Verified source:**

`control_start()` at `control.c:758-796` initializes control state with zero authentication. The security boundary is the Unix socket: creation at `server.c:105-134` applies `umask(S_IXUSR|S_IXGRP|S_IRWXO)` for default sockets (line 127), ACL check at `server-acl.c:164` via `server_acl_join()`. Backpressure is output-side only: `control.c:450-461` pauses or disconnects lagging control clients.

**Rust implementation:**

```rust
pub struct ControlRateLimiter {
    /// Max commands per second per control-mode client.
    pub max_commands_per_sec: u32,
    /// Max pending output bytes before backpressure.
    pub max_pending_bytes: usize,
}

impl Default for ControlRateLimiter {
    fn default() -> Self {
        Self {
            max_commands_per_sec: 1000,
            max_pending_bytes: 16 * 1024 * 1024, // 16 MB
        }
    }
}
```

**Test strategy:**
1. Control mode client connects and runs commands without any token.
2. Rate limiter triggers backpressure when client exceeds threshold.
3. Non-owner user cannot connect (Unix permissions test).

---

### Q4: CRDT Vector Clock Size

**Decision:** Hybrid Logical Clocks (HLC) per-node with dotted version vectors (DVV) for sync delta computation. Bounded by active node count with membership-aware compaction. Use `checked_add` for counter overflow safety.

**Rationale:** HLC timestamps are fixed-size `(u64 millis, u32 counter, NodeId)` = 20 bytes and provide causal ordering sufficient for LWW-Register and OR-Set CRDTs. DVV adds a compact per-node high-water summary for sync. All three models agreed that unbounded vector clocks are unacceptable and that DVV with compaction is the right trade-off.

```rust
/// HLC timestamp: fixed 20 bytes, causally ordered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
         serde::Serialize, serde::Deserialize)]
pub struct HlcTimestamp {
    pub millis: u64,
    pub counter: u32,
    pub node_id: NodeId,
}

/// 8-byte node identifier, generated from UUID at server start.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
         serde::Serialize, serde::Deserialize)]
pub struct NodeId(pub u64);

/// Dotted version vector for sync delta computation.
pub struct Dvv {
    pub summary: SmallVec<[(NodeId, u64); 8]>,
    pub dot: HlcTimestamp,
}

/// Compact DVV by removing entries for departed nodes.
pub fn compact_dvv(dvv: &mut Dvv, active_nodes: &[NodeId]) {
    dvv.summary.retain(|(node, _)| active_nodes.contains(node));
}
```

**Test strategy:**
1. Property test: `now()` always produces monotonically increasing timestamps.
2. Property test: `receive()` produces a timestamp causally after both local and remote.
3. Unit test: OpLog sync with 3 nodes produces identical state.
4. Compaction test: add 10 nodes, deactivate 5, compact, verify 5 entries remain.

---

### Q5: Benchmark Baseline

**Decision:** No existing baselines. The sole benchmark file at `crates/mux-refresh/benches/latency.rs` is not wired correctly -- `crates/mux-refresh/Cargo.toml` (19 lines total) lacks `[[bench]] harness = false`. Fix harness first, establish baselines, then enforce regression gates.

**Verified:** `mux-refresh/Cargo.toml` confirmed to have no `[[bench]]` section (lines 1-19 contain only `[package]`, `[dependencies]`, and `[dev-dependencies]`).

**Immediate fix:**
```toml
# Add to crates/mux-refresh/Cargo.toml
[[bench]]
name = "latency"
harness = false
```

**Conservative targets (to be validated against actual measurements):**

| Benchmark | Target | Basis |
|---|---|---|
| VT100 parser, plain ASCII | > 300 MB/s | alacritty vte achieves ~500 MB/s |
| VT100 parser, CSI heavy | > 100 MB/s | Parameter parsing overhead |
| Protocol frame decode | > 500 MB/s | 16-byte header + memcpy |
| Layout resize (20 panes) | < 50 us | Tree walk ~60 nodes, no alloc |
| Format expand (status line) | < 50 us | ~10 variable lookups |
| Graph snapshot | < 1 ms | Arc clone + BTreeMap for 50 panes |
| Config parse (500 lines) | < 5 ms | Lexer + parser, no IO |
| Option resolve (4-level chain) | < 100 ns | 4 BTreeMap lookups |
| Control notification parse | < 200 ns | String split + parse |

**CI regression gate:** 130% threshold on nightly only (not every PR). Must assert non-empty criterion output before comparing.

---

### Q6: Python Async Bindings

**Decision:** Three-phase approach.

- **Phase 1 (sync):** All blocking methods release GIL via `py.allow_threads()`. This matches libtmux's sync model (`src/libtmux/options.py:701-787`).
- **Phase 2 (Python-side async):** `asyncio.to_thread()` wrappers in Python, zero Rust changes.
- **Phase 3 (native streaming):** `pyo3-async-runtimes` only for event subscription, bridging `broadcast::Receiver` to `asyncio.Queue`.

**Verified:** vibe-tmux Python binding depends on `pyo3` only, no `pyo3-asyncio` (`bindings/python/Cargo.toml`).

```rust
// Phase 1: GIL release
fn cmd(&self, py: Python<'_>, cmd: &str) -> PyResult<String> {
    let inner = self.inner.clone();
    py.allow_threads(move || { /* blocking */ })
}
```

```python
# Phase 2: Zero Rust changes
async def async_cmd(server: Server, cmd: str) -> str:
    return await asyncio.to_thread(server.cmd, cmd)
```

```rust
// Phase 3: Native streaming only
#[pymethod]
fn subscribe(&self, py: Python<'_>) -> PyResult<PyObject> {
    let asyncio = py.import("asyncio")?;
    let queue = asyncio.call_method0("Queue")?;
    // Rust thread bridges broadcast::Receiver -> asyncio.Queue
    Ok(queue.to_object(py))
}
```

---

### Q7: Layout Minimum Size

**Decision:** Resize clamps to the minimum possible size. It never fails. `PANE_MINIMUM` is 1 cell (`tmux.h:100`). Distribution is round-robin one-cell-at-a-time, NOT proportional. Returns `ResizeResult` with requested vs applied delta.

**Verified source:**

`layout_resize()` at `layout.c:534-583`:
- Computes `xchange = sx - lc->sx` (line 553).
- Computes `xlimit = layout_resize_check(w, lc, LAYOUT_LEFTRIGHT)` (line 554).
- Clamps: `if (xchange < 0 && xchange < -xlimit) xchange = -xlimit` (lines 555-556).
- If already at minimum and not growing: `xchange = 0` (lines 557-559).

`layout_resize_check()` at `layout.c:366-415`:
- Leaf: `available = cell_size - minimum` clamped to 0 (lines 378-397).
- Same-direction container: sum of children's available (lines 398-402).
- Perpendicular container: minimum of children's available (lines 403-411).

`layout_resize_adjust()` at `layout.c:421-463`:
- Same-direction children: distributes one unit at a time in round-robin (lines 448-462).
- The while loop terminates because the caller clamps `change` to available space.
- Growing always succeeds (lines 452-455).
- Shrinking checks per-child availability (lines 457-460).

**Critical correction from Pass 2:** Claude v5 Pass 1 used proportional distribution. All three Pass 2 outputs corrected this to round-robin. Proportional distribution would produce different layout geometry from tmux, breaking visual parity. This is now definitive.

**Rust implementation:**

```rust
pub const PANE_MINIMUM: u16 = 1;

pub struct ResizeResult {
    pub requested_x: i32,
    pub requested_y: i32,
    pub applied_x: i32,
    pub applied_y: i32,
}

impl LayoutTree {
    /// Resize the layout to fit new window size. Never fails.
    /// Reference: layout.c:534-583
    pub fn resize(&mut self, new_sx: u16, new_sy: u16) -> ResizeResult {
        if self.cells.is_empty() {
            return ResizeResult {
                requested_x: 0, requested_y: 0,
                applied_x: 0, applied_y: 0,
            };
        }
        let root = &self.cells[0];
        let old_sx = root.sx;
        let old_sy = root.sy;

        // Horizontal
        let xlimit = self.resize_check(0, LayoutType::LeftRight) as i32;
        let requested_x = new_sx as i32 - old_sx as i32;
        let mut xchange = requested_x;
        if xchange < 0 && xchange < -xlimit {
            xchange = -xlimit;
        }
        if xlimit == 0 && new_sx <= old_sx {
            xchange = 0;
        }
        if xchange != 0 {
            self.resize_adjust(0, LayoutType::LeftRight, xchange);
        }

        // Vertical (same logic)
        let ylimit = self.resize_check(0, LayoutType::TopBottom) as i32;
        let requested_y = new_sy as i32 - old_sy as i32;
        let mut ychange = requested_y;
        if ychange < 0 && ychange < -ylimit {
            ychange = -ylimit;
        }
        if ylimit == 0 && new_sy <= old_sy {
            ychange = 0;
        }
        if ychange != 0 {
            self.resize_adjust(0, LayoutType::TopBottom, ychange);
        }

        self.fix_offsets();

        ResizeResult {
            requested_x, requested_y,
            applied_x: xchange, applied_y: ychange,
        }
    }

    /// How much space can be removed from cell in given direction.
    /// Reference: layout.c:366-415
    fn resize_check(&self, idx: usize, direction: LayoutType) -> u16 {
        let cell = &self.cells[idx];
        match cell.cell_type {
            LayoutType::WindowPane => {
                let size = if direction == LayoutType::LeftRight {
                    cell.sx
                } else {
                    cell.sy
                };
                size.saturating_sub(PANE_MINIMUM)
            }
            ty if ty == direction => {
                cell.children.iter()
                    .map(|&child| self.resize_check(child, direction))
                    .sum()
            }
            _ => {
                cell.children.iter()
                    .map(|&child| self.resize_check(child, direction))
                    .min()
                    .unwrap_or(0)
            }
        }
    }

    /// Distribute size change one cell at a time, round-robin.
    /// Reference: layout.c:421-463
    fn resize_adjust(&mut self, idx: usize, direction: LayoutType, mut change: i32) {
        if direction == LayoutType::LeftRight {
            self.cells[idx].sx = (self.cells[idx].sx as i32 + change).max(0) as u16;
        } else {
            self.cells[idx].sy = (self.cells[idx].sy as i32 + change).max(0) as u16;
        }

        if self.cells[idx].cell_type == LayoutType::WindowPane {
            return;
        }

        let children: Vec<usize> = self.cells[idx].children.clone();

        if self.cells[idx].cell_type != direction {
            for &child in &children {
                self.resize_adjust(child, direction, change);
            }
            return;
        }

        // Round-robin: one unit at a time per child.
        // tmux guarantees termination via caller clamping.
        // We add a safety guard for defensive coding.
        while change != 0 {
            let mut made_progress = false;
            for &child in &children {
                if change == 0 { break; }
                if change > 0 {
                    self.resize_adjust(child, direction, 1);
                    change -= 1;
                    made_progress = true;
                } else if self.resize_check(child, direction) > 0 {
                    self.resize_adjust(child, direction, -1);
                    change += 1;
                    made_progress = true;
                }
            }
            if !made_progress { break; }
        }
    }
}
```

**Note on safety guard:** tmux's while loop at `layout.c:448-462` has no explicit `made_progress` break because the caller guarantees `change <= available`. Our Rust version adds `if !made_progress { break; }` as defensive coding -- it should never trigger if `resize_check` is correct, but prevents infinite loops if a bug is introduced.

---

## A. Error Handling

### A.1 Error Classification System

All three Pass 2 outputs independently converged on the same four-category taxonomy. This is now definitive.

```rust
// crates/mux-types/src/error_class.rs
#![forbid(unsafe_code)]

/// Classification of errors for recovery policy decisions.
/// Reference: tmux "goto bad -> proc_kill_peer" at
/// server-client.c:3472-3475 maps to ProtocolViolation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorClass {
    /// Transient: retry or wait for more data.
    /// Examples: incomplete frame, timeout, temporary IO failure.
    /// Recovery: retry with backoff, keep connection.
    Transient,

    /// Protocol violation: peer sent structurally invalid data.
    /// Examples: bad magic, length overflow, unknown msg type.
    /// Recovery: kill connection, log, keep server alive.
    ProtocolViolation,

    /// User error: semantically invalid request from valid connection.
    /// Examples: unknown command, invalid option, target not found.
    /// Recovery: send error reply, keep connection.
    UserError,

    /// Bug / invariant violation: internal consistency failure.
    /// Examples: dangling entity ID, impossible state.
    /// Recovery: log with full context, crash in debug.
    Bug,
}
```

### A.2 Classified Trait

```rust
/// Every error type implements this for dispatch.
pub trait Classified {
    fn class(&self) -> ErrorClass;
}

// Per-crate implementations:
impl Classified for ProtocolError {
    fn class(&self) -> ErrorClass {
        match self {
            ProtocolError::Incomplete => ErrorClass::Transient,
            ProtocolError::LengthTooSmall { .. }
            | ProtocolError::LengthTooLarge { .. }
            | ProtocolError::UnknownMessageType { .. }
            | ProtocolError::InvalidPayload { .. }
            | ProtocolError::MissingNul { .. }
            | ProtocolError::InvalidUtf8 { .. }
            | ProtocolError::NotIdentifyMessage { .. } => ErrorClass::ProtocolViolation,
        }
    }
}

impl Classified for CoreError {
    fn class(&self) -> ErrorClass {
        match self {
            CoreError::Entity(_)
            | CoreError::InvalidCommand { .. }
            | CoreError::InvalidOption { .. }
            | CoreError::LayoutConstraint { .. }
            | CoreError::CopyMode { .. } => ErrorClass::UserError,
            CoreError::FormatExpansion { .. } => ErrorClass::Bug,
        }
    }
}
```

### A.3 Codec Recovery: DecodeOutcome

**Critical correction from Pass 2:** Claude v5 Pass 1 incorrectly stated that malformed frames can be dropped while keeping the connection alive. tmux kills the peer on protocol violations (`server-client.c:3472-3475`: `goto bad` -> `proc_kill_peer`). The correct behavior is: `ProtocolViolation` -> close connection.

GPT's `DecodeOutcome` enum is adopted as the codec return type:

```rust
// crates/mux-proto/src/codec.rs

/// Outcome of decoding one frame from the buffer.
pub enum DecodeOutcome {
    /// Need more bytes.
    NeedMore,
    /// Successfully decoded a frame.
    Frame(ImsgFrame),
    /// Unrecoverable protocol error. Close this connection.
    ProtocolViolation(ProtocolError),
}
```

### A.4 Connection Handler

```rust
// crates/mux-server/src/conn.rs

async fn handle_client_data(
    conn: &mut ClientConn,
    buf: &[u8],
) -> ConnectionAction {
    conn.codec.feed(buf);
    loop {
        match conn.codec.try_decode_one() {
            DecodeOutcome::NeedMore => return ConnectionAction::Continue,
            DecodeOutcome::Frame(frame) => {
                if let Err(e) = process_frame(conn, frame).await {
                    match e.class() {
                        ErrorClass::ProtocolViolation | ErrorClass::Bug => {
                            tracing::warn!(
                                client_id = %conn.id,
                                error = %e,
                                "killing connection: {:?}", e.class()
                            );
                            return ConnectionAction::Kill(e.to_string());
                        }
                        ErrorClass::UserError => {
                            send_error_to_client(conn, &e).await;
                        }
                        ErrorClass::Transient => { /* should not happen */ }
                    }
                }
            }
            DecodeOutcome::ProtocolViolation(e) => {
                tracing::warn!(
                    client_id = %conn.id, error = %e,
                    "codec protocol violation, killing connection"
                );
                return ConnectionAction::Kill(e.to_string());
            }
        }
    }
}

pub enum ConnectionAction {
    Continue,
    Kill(String),
}
```

### A.5 Error Handling Test Strategy

1. **Fuzz testing:** `fuzz/decode_frame.rs` feeds arbitrary bytes to `ImsgCodec`, asserts never panics, always returns one of three `DecodeOutcome` variants.
2. **Classification coverage:** Unit test that every variant of every error enum returns a valid `ErrorClass`.
3. **Recovery integration:** Feed valid frame then garbage bytes. Verify connection killed.
4. **Error propagation:** Trigger each `CoreError` variant, verify `Effect::ErrorReply`.
5. **Protocol violation:** Send identify message after already identified. Verify kill (matches `server-client.c:3597-3600`).

---

## B. Configuration System

### B.1 Option Scope Resolution

tmux's `options_scope_from_name()` at `options.c:850-919` resolves scope using:
1. The option's declared scope in `options_table` (`OPTIONS_TABLE_SERVER`, `OPTIONS_TABLE_SESSION`, `OPTIONS_TABLE_WINDOW`, combined `OPTIONS_TABLE_WINDOW|OPTIONS_TABLE_PANE`).
2. Command flags (`-g`, `-s`, `-p`, `-w`).
3. Target (`-t`) if specified.
4. User options (`@foo`) use `options_scope_from_flags` (line 863).

**Critical detail:** The `FALLTHROUGH` at `options.c:903` from `WINDOW|PANE` to `WINDOW`. When `-p` is specified, the pane scope is selected (lines 892-901). Otherwise, processing falls through to `WINDOW` (lines 904-916). This means `set -g pane-border-status` sets the global window option, not a pane option, because `-g` triggers line 905.

```rust
pub fn resolve_option_scope(
    option_name: &str,
    flags: &SetOptionFlags,
    target: &ResolvedTarget,
) -> Result<(OptionScope, TargetId), CoreError> {
    if option_name.starts_with('@') {
        return resolve_scope_from_flags(flags, target);
    }

    let table_entry = OPTION_TABLE.get(option_name)
        .ok_or_else(|| CoreError::InvalidOption {
            scope: OptionScope::Server,
            key: option_name.to_string(),
            reason: "unknown option".to_string(),
        })?;

    match table_entry.scope {
        TableScope::Server => Ok((OptionScope::Server, TargetId::Server)),
        TableScope::Session => {
            if flags.global {
                Ok((OptionScope::Session, TargetId::Server))
            } else {
                Ok((OptionScope::Session, TargetId::Session(target.session()?)))
            }
        }
        TableScope::Window => {
            if flags.global {
                Ok((OptionScope::Window, TargetId::Server))
            } else {
                Ok((OptionScope::Window, TargetId::Window(target.window()?)))
            }
        }
        TableScope::WindowPane => {
            // Matches tmux FALLTHROUGH at options.c:903
            if flags.pane {
                Ok((OptionScope::Pane, TargetId::Pane(target.pane()?)))
            } else if flags.global {
                Ok((OptionScope::Window, TargetId::Server))
            } else {
                Ok((OptionScope::Window, TargetId::Window(target.window()?)))
            }
        }
    }
}
```

### B.2 Config Load Timing

**Verified:** tmux loads config only after the first client completes identify. At `server-client.c:3725-3734`:

```c
if ((~c->flags & CLIENT_EXIT) &&
     !cfg_finished &&
     c == TAILQ_FIRST(&clients))
    start_cfg();
```

TermForge must replicate this: the server starts and binds the socket but does NOT load `~/.tmux.conf` or `~/.config/termforge/config` until the first client's identify burst completes. This ensures config errors are reported to the first client.

### B.3 Configuration Test Strategy

1. **Scope resolution matrix:** For each combination of (option scope, flag set, target), verify correct `(OptionScope, TargetId)`.
2. **Inheritance chain:** 4-level hierarchy (server -> session -> window -> pane), verify walk.
3. **Unset test:** Set/unset cycles per Q2.
4. **User option test:** `@foo` with `-s`, `-g`, `-p` flags.
5. **Parity test:** Compare `show-options -g/-s/-w/-p` between tmux and TermForge.
6. **Config reload test:** Load config, modify, reload, verify only changed options produce events.
7. **Config-as-Events:** Verify config parsing produces `Vec<Event>`, never mutates graph directly.

---

## C. Layout Engine

### C.1 Layout Validation (`layout_check`)

tmux validates layout consistency at `layout-custom.c:119-153`. For `LAYOUT_LEFTRIGHT` containers, it verifies that all children have the same height as the parent and that the sum of `(child_widths + borders)` equals the parent width. The border accounting adds 1 per child and subtracts 1 for the total (`n += lcchild->sx + 1` then checks `n - 1 != lc->sx`).

```rust
/// Validate layout tree internal consistency.
/// Reference: layout-custom.c:119-153
pub fn layout_check(tree: &LayoutTree) -> bool {
    if tree.cells.is_empty() { return true; }
    layout_check_cell(tree, 0)
}

fn layout_check_cell(tree: &LayoutTree, idx: usize) -> bool {
    let cell = &tree.cells[idx];
    match cell.cell_type {
        LayoutType::WindowPane => true,
        LayoutType::LeftRight => {
            let mut n: u32 = 0;
            for &child_idx in &cell.children {
                let child = &tree.cells[child_idx];
                if child.sy != cell.sy { return false; }
                if !layout_check_cell(tree, child_idx) { return false; }
                n += child.sx as u32 + 1; // +1 for border
            }
            // n - 1 accounts for no border before first child
            n.saturating_sub(1) == cell.sx as u32
        }
        LayoutType::TopBottom => {
            let mut n: u32 = 0;
            for &child_idx in &cell.children {
                let child = &tree.cells[child_idx];
                if child.sx != cell.sx { return false; }
                if !layout_check_cell(tree, child_idx) { return false; }
                n += child.sy as u32 + 1;
            }
            n.saturating_sub(1) == cell.sy as u32
        }
    }
}
```

### C.2 Arena Compaction

The flat Vec arena can accumulate dead cells after `layout_destroy_cell` operations. tmux collapses a parent into its sole remaining child (`layout.c:465-513`). With a flat Vec, destroyed cells leave holes. Periodic compaction removes unreachable cells:

```rust
impl LayoutTree {
    /// Compact arena by removing unreachable cells.
    pub fn compact(&mut self) {
        let mut reachable = vec![false; self.cells.len()];
        if !self.cells.is_empty() {
            self.mark_reachable(0, &mut reachable);
        }
        let mut mapping = vec![usize::MAX; self.cells.len()];
        let mut new_cells = Vec::new();
        for (old_idx, cell) in self.cells.iter().enumerate() {
            if reachable[old_idx] {
                mapping[old_idx] = new_cells.len();
                new_cells.push(cell.clone());
            }
        }
        for cell in &mut new_cells {
            cell.parent = cell.parent.map(|p| mapping[p]);
            cell.children = cell.children.iter()
                .map(|&c| mapping[c])
                .collect();
        }
        self.cells = new_cells;
    }

    fn mark_reachable(&self, idx: usize, reachable: &mut Vec<bool>) {
        reachable[idx] = true;
        for &child in &self.cells[idx].children {
            self.mark_reachable(child, reachable);
        }
    }
}
```

### C.3 Layout Test Strategy

1. **Roundtrip property:** `layout_parse(layout_dump(tree)) == tree`.
2. **Checksum parity:** Compare against tmux-generated layout strings.
3. **layout_check invariant:** After every mutation (split, resize, destroy, compact), `layout_check(tree)` returns true. Add as `debug_assert!` in all mutating methods.
4. **Round-robin distribution:** Split 3 panes at 100 cols, resize to 103, verify one cell at a time gets +1.
5. **Pane assignment parity:** Parse tmux layout string with N cells, verify depth-first assignment.
6. **Edge cases:**
   - Single pane: dump/parse roundtrip.
   - Maximum depth: 20 levels of nesting.
   - Resize to 1x1: all panes at `PANE_MINIMUM`.
   - Split at minimum: returns `LayoutError::TooSmall` (matches `layout.c:937-944`).
   - Destroy last child: parent collapses.
   - Compact after 10 destroy ops: no dead cells.
7. **Preset parity:** For each of 7 presets with N=1..20 panes, compare layout dump against tmux `select-layout` output.

---

## D. OpenTelemetry

### D.1 Span Naming Convention

Adopt `termforge.` prefix for all span names (GPT proposal, adopted by all three). This follows OpenTelemetry semantic conventions and avoids collision with third-party libraries.

```rust
pub mod span_names {
    pub const SERVER_ACCEPT: &str = "termforge.server.accept";
    pub const CLIENT_IDENTIFY: &str = "termforge.server.client.identify";
    pub const ENGINE_APPLY: &str = "termforge.core.apply_event";
    pub const EFFECT_DISPATCH: &str = "termforge.runtime.dispatch_effect";
    pub const PTY_READ: &str = "termforge.pty.read";
    pub const PTY_WRITE: &str = "termforge.pty.write";
    pub const CODEC_DECODE: &str = "termforge.proto.decode";
    pub const CODEC_ENCODE: &str = "termforge.proto.encode";
    pub const SNAPSHOT_BUILD: &str = "termforge.state.snapshot";
    pub const SOCKET_ACTOR_CYCLE: &str = "termforge.api.socket_actor.cycle";
    pub const CONFIG_RELOAD: &str = "termforge.config.reload";
    pub const LAYOUT_RESIZE: &str = "termforge.layout.resize";
    pub const FORMAT_EXPAND: &str = "termforge.format.expand";
    pub const CONTROL_PARSE: &str = "termforge.control.parse";
    pub const GRID_PARSE: &str = "termforge.grid.parse";
    pub const BINDINGS_PYTHON: &str = "termforge.bindings.python";
    pub const BINDINGS_NODE: &str = "termforge.bindings.node";
}
```

### D.2 Export Filter

```rust
pub struct ExportFilterConfig {
    /// Span name prefixes to exclude from export.
    pub excluded_prefixes: Vec<String>,
}

impl Default for ExportFilterConfig {
    fn default() -> Self {
        Self {
            excluded_prefixes: vec![
                "h2".into(), "tonic".into(), "hyper".into(),
                "tower".into(), "reqwest".into(),
            ],
        }
    }
}

impl ExportFilterConfig {
    pub fn should_export(&self, span_name: &str) -> bool {
        !self.excluded_prefixes.iter().any(|p| span_name.starts_with(p))
    }
}
```

### D.3 Telemetry Test Strategy

1. Verify `init_tracing()` with `Exporter::Stdout` produces valid JSON.
2. Integration: run server with OTEL, verify span parent-child relationships.
3. Pure boundary: verify `mux-core` depends only on `tracing`, not `opentelemetry`.
4. Context propagation: trace from Python binding appears as parent in server trace.
5. Filter test: `should_export` excludes `h2`, `tonic`, `hyper`.

---

## E. Language Bindings

### E.1 Error Mapping

Map `ErrorClass` to native exception types:

| ErrorClass | Python | Node.js | C++ |
|---|---|---|---|
| Transient | `TimeoutError` | `Error` with `ETIMEOUT` | `std::runtime_error` |
| ProtocolViolation | `ConnectionError` | `Error` with `EPROTO` | `std::runtime_error` |
| UserError | `ValueError` / `KeyError` | `Error` with `EINVAL` | `std::invalid_argument` |
| Bug | `RuntimeError` | `Error` with `EINTERNAL` | `std::logic_error` |

### E.2 Migration from libtmux

Expose property names matching libtmux conventions:

```python
server = termforge.Server()
server.sessions      # property, returns list
server.cmd("ls")     # method, returns string
```

### E.3 Bindings Test Strategy

1. **Python:** pytest with hermetic server per test.
2. **Node:** vitest with same pattern.
3. **GIL test:** blocking call in one thread, concurrent Python thread confirms GIL released.
4. **Error mapping:** trigger each error class, verify correct exception type.
5. **Memory leak:** `tracemalloc` for 1000 create/destroy cycles.
6. **Concurrency:** 10 threads calling `server.sessions` -- no deadlocks.
7. **asyncio.to_thread:** verify `await asyncio.to_thread(server.cmd, "list-sessions")` works.

---

## F. CRDT Detail

### F.1 HLC Overflow Guard

All three models agreed on HLC. GPT used `saturating_add`, Claude used `+ 1`. The synthesized approach uses `checked_add` with forced millis advance on overflow:

```rust
impl HybridClock {
    pub fn receive(&mut self, remote: HlcTimestamp, wall_ms: u64) -> HlcTimestamp {
        let millis = wall_ms.max(self.last.millis).max(remote.millis);
        let counter = if millis == self.last.millis && millis == remote.millis {
            match self.last.counter.max(remote.counter).checked_add(1) {
                Some(c) => c,
                None => {
                    // Counter overflow: force millis advance to reset counter.
                    return self.force_advance(millis + 1);
                }
            }
        } else if millis == self.last.millis {
            self.last.counter.saturating_add(1)
        } else if millis == remote.millis {
            remote.counter.saturating_add(1)
        } else {
            0
        };
        self.last = HlcTimestamp { millis, counter, node_id: self.node_id };
        self.last
    }

    fn force_advance(&mut self, millis: u64) -> HlcTimestamp {
        self.last = HlcTimestamp { millis, counter: 0, node_id: self.node_id };
        self.last
    }
}
```

### F.2 CRDT Types Summary

```rust
/// Last-Writer-Wins Register.
pub struct LwwRegister<T> {
    pub value: T,
    pub timestamp: HlcTimestamp,
}

/// Observed-Remove Set.
pub struct OrSet<T: Eq + Hash> {
    pub entries: HashMap<T, SmallVec<[HlcTimestamp; 2]>>,
}

/// Operation log for sync.
pub struct OpLog {
    pub ops: Vec<CrdtOp>,
    pub version: Dvv,
}
```

### F.3 CRDT Test Strategy

1. HLC monotonicity: 10K calls to `now()` with non-decreasing `wall_ms` -> strictly increasing.
2. HLC convergence: two clocks, 1000 interleaved ops, causally consistent.
3. LWW determinism: same timestamp, different node_ids -> deterministic winner.
4. OR-Set commutativity: `merge(a, b) == merge(b, a)`.
5. OR-Set add-remove-add: re-add with higher timestamp restores element.
6. OpLog idempotency: merging same ops twice -> identical state.
7. Counter overflow: simulate and verify forced millis advance.
8. DVV compaction: 10 nodes, deactivate 5, compact -> 5 entries.

---

## G. Control Mode

### G.1 Typed Notifications with Migration Path

**Current state (verified):** `crates/mux-client/src/control.rs:30-36` defines a generic struct:

```rust
pub struct ControlNotification {
    pub name: String,  // notification name without '%'
    pub raw: String,   // raw line
}
```

This must be incrementally migrated to a typed enum. The migration preserves backward compatibility:

```rust
/// Typed notification variants.
pub enum ControlNotification {
    SessionsChanged,
    SessionChanged { session_id: u32, name: String },
    SessionRenamed { session_id: u32, name: String },
    SessionWindowChanged { session_id: u32, window_id: u32 },
    WindowAdd { window_id: u32 },
    WindowClose { window_id: u32 },
    WindowRenamed { window_id: u32, name: String },
    WindowPaneChanged { window_id: u32, pane_id: u32 },
    Output { pane_id: u32, data: Vec<u8> },
    ExtendedOutput { pane_id: u32, age: u64, data: Vec<u8> },
    LayoutChange { window_id: u32, layout: String },
    PaneModeChanged { pane_id: u32 },
    ClientSessionChanged { client: String, session_id: u32 },
    ClientDetached { client: String },
    Pause { pane_id: u32 },
    Continue { pane_id: u32 },
    Exit { reason: Option<String> },
}
```

`%extended-output` (verified at `control.c:620-623`) is included as a distinct variant with age timestamp.

### G.2 Migration Parser

```rust
pub fn parse_control_line(line: &str) -> ControlEvent {
    match parse_notification(line) {
        Ok(typed) => ControlEvent::Typed(typed),
        Err(ControlParseError::UnknownNotification(tag)) => {
            ControlEvent::Generic { tag, raw: line.to_string() }
        }
        Err(ControlParseError::NotANotification) => {
            ControlEvent::OutputLine(line.to_string())
        }
        Err(e) => ControlEvent::ParseError(e),
    }
}

pub enum ControlEvent {
    Typed(ControlNotification),
    Generic { tag: String, raw: String },
    OutputLine(String),
    ParseError(ControlParseError),
}
```

### G.3 Control Mode Test Strategy

1. **Parser roundtrip:** construct line per notification type, parse, verify variant.
2. **Unknown notification:** `%foo-bar` produces `Generic`.
3. **Guard protocol:** `%begin/%end/%error` block framing.
4. **Malformed lines:** truncated, empty, binary -> `ControlParseError`.
5. **Parity test:** connect via `tmux -C`, trigger each notification, feed to parser.
6. **Hints-vs-authority:** dropped notification corrected by periodic refresh.
7. **Migration test:** `ControlEvent::Generic` fallback works for un-typed notifications.
8. **Extended output:** parse `%extended-output %5 12345 : hello`, verify fields.

---

## H. Server Lifecycle

### H.1 Lock File: flock (Not PID-based)

**Critical correction from Pass 2:** Claude v5 Pass 1 used `O_EXCL` + PID check. tmux uses `flock(LOCK_EX|LOCK_NB)` at `client.c:77-101`:

```c
// client.c:78-101
static int
client_get_lock(char *lockfile)
{
    int lockfd;
    if ((lockfd = open(lockfile, O_WRONLY|O_CREAT, 0600)) == -1)
        return (-1);
    if (flock(lockfd, LOCK_EX|LOCK_NB) == -1) {
        if (errno != EAGAIN)
            return (lockfd);
        while (flock(lockfd, LOCK_EX) == -1 && errno == EINTR)
            /* nothing */;
        close(lockfd);
        return (-2);
    }
    return (lockfd);
}
```

**Advantages of flock over PID:**
- Automatically released on process death (kernel cleanup).
- No PID reuse risk (eliminates Gemini's identified vulnerability).
- No TOCTOU race between check and remove.

```rust
// crates/mux-os/src/lock.rs

pub struct LockFile {
    _file: std::fs::File,  // held open to keep flock
    path: PathBuf,
}

pub fn acquire_lock_file(socket_path: &Path) -> Result<LockFile, ServerError> {
    let lock_path = socket_path.with_extension("lock");

    let file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(|e| ServerError::Startup {
            reason: format!("cannot open lock file {lock_path:?}: {e}"),
        })?;

    let fd = file.as_raw_fd();
    let ret = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };
    if ret != 0 {
        let err = std::io::Error::last_os_error();
        if err.kind() == std::io::ErrorKind::WouldBlock {
            return Err(ServerError::Startup {
                reason: format!("another server holds the lock on {lock_path:?}"),
            });
        }
        return Err(ServerError::Startup {
            reason: format!("flock failed on {lock_path:?}: {err}"),
        });
    }

    // Write PID for diagnostics only (not for locking)
    use std::io::Write;
    let mut f = &file;
    let _ = f.write_all(format!("{}\n", std::process::id()).as_bytes());

    Ok(LockFile { _file: file, path: lock_path })
}

impl Drop for LockFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}
```

### H.2 Startup Sequence (13 Steps)

1. Parse CLI arguments.
2. Resolve socket path.
3. Acquire lock file (flock).
4. Create Unix domain socket with restricted permissions.
5. Daemonize (if not foreground mode).
6. Initialize tracing/telemetry.
7. Initialize event loop (tokio runtime).
8. Spawn socket acceptor task.
9. **Wait for first client connection.**
10. **Complete first client identify burst.**
11. **Load configuration files** (matches `server-client.c:3725-3734`).
12. Report config errors to first client.
13. Enter main event loop.

Steps 9-12 are ordered to match tmux exactly: config is not loaded until the first client identifies.

### H.3 Server Lifecycle Test Strategy

1. **Lock contention:** Start A, then B on same socket. B fails.
2. **Crash recovery:** Start, kill -9, start again. Succeeds (flock released).
3. **Config timing:** Start server, connect client, verify config loaded after identify.
4. **Shutdown:** 3 clients connected, shutdown, all receive exit notification.
5. **Signal handling:** SIGTERM triggers graceful shutdown.
6. **Identify burst:** Fixture tests for types 100-112, verify ordering parity with `client_send_identify` at `client.c:450-495`.

---

## I. Security Model

### I.1 Input Validation Layers

```
Layer  Input Source         Validator            Error Class          Action
-----  ------------         ---------            -----------          ------
  0    Socket bytes         ImsgCodec            Transient            wait
                                                 ProtocolViolation    kill conn
  1    ImsgFrame            PayloadParser        ProtocolViolation    kill conn
  2    TypedPayload         IdentifyCollector    ProtocolViolation    kill conn
  3    Command args         CommandDispatch      UserError            error reply
  4    Config file text     mux-conf lexer       UserError            error event
  5    Control mode lines   ControlParser        UserError            %error reply
  6    Binding FFI args     mux-orm validators   UserError            exception
  7    CRDT ops (sync)      CrdtValidator        ProtocolViolation    drop peer
```

### I.2 Security Invariants

1. **Socket permissions:** Created with `umask(S_IXUSR|S_IXGRP|S_IRWXO)` for default sockets (matching `server.c:127`), restricting to owner + group for shared sockets.
2. **No symlink following:** Check for symlinks in socket directory path.
3. **Maximum clients:** 256 simultaneous connections (configurable).
4. **Control mode output limit:** 16 MB pending per control client (Q3).
5. **SCM_RIGHTS phase restriction:** FDs via SCM_RIGHTS accepted only during identify handshake. Out-of-phase FDs closed immediately.
6. **CLOEXEC:** All received FDs get CLOEXEC immediately. Verified existing: `crates/mux-os/src/scm_rights.rs:141-149` already sets CLOEXEC via `set_cloexec()`. The `ValidatedFd` type wraps this, not duplicates it.
7. **Multiple FD handling:** Multiple FDs in one ancillary message: close all extras deterministically (`extract_first_fd` in `scm_rights.rs` handles this).

### I.3 Security Test Strategy

1. Socket permissions: create, verify mode 0600.
2. Directory permissions: reject world-writable directory.
3. SCM_RIGHTS: send non-TTY fd during identify, verify accepted but `is_tty: false`.
4. CLOEXEC: after receiving fd, verify FD_CLOEXEC set.
5. Max client: connect 257, verify 257th rejected.
6. Fuzz: random bytes to all parsers. No panic, no OOB.
7. Out-of-phase SCM_RIGHTS: send fd after identify complete, verify closed and connection killed.

---

## J. Performance Targets

### J.1 Benchmark Suite

```rust
// crates/mux-bench/benches/hot_path.rs

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId, black_box};

fn bench_layout_checksum(c: &mut Criterion) {
    let layout = "204x50,0,0{102x50,0,0,0,101x50,103,0[101x25,103,0,1,101x24,103,26,2]}";
    c.bench_function("layout_checksum", |b| {
        b.iter(|| black_box(layout_checksum(layout.as_bytes())))
    });
}

fn bench_layout_parse(c: &mut Criterion) {
    let s = "d3a0,204x50,0,0{102x50,0,0,0,101x50,103,0[101x25,103,0,1,101x24,103,26,2]}";
    c.bench_function("layout_parse", |b| {
        b.iter(|| black_box(layout_parse(s)))
    });
}

fn bench_layout_dump(c: &mut Criterion) {
    let panes: Vec<PaneId> = (0..10).collect();
    let tree = LayoutPreset::Tiled.arrange(&panes, 200, 60, None);
    c.bench_function("layout_dump_10_panes", |b| {
        b.iter(|| black_box(layout_dump(&tree)))
    });
}

fn bench_layout_resize(c: &mut Criterion) {
    let mut group = c.benchmark_group("layout_resize");
    for n_panes in [4, 10, 20, 50] {
        group.bench_with_input(
            BenchmarkId::from_parameter(n_panes),
            &n_panes,
            |b, &n| {
                let panes: Vec<PaneId> = (0..n).collect();
                let mut tree = LayoutPreset::Tiled.arrange(&panes, 200, 60, None);
                b.iter(|| {
                    tree.resize(180, 50);
                    tree.resize(200, 60);
                });
            },
        );
    }
    group.finish();
}

fn bench_option_resolve(c: &mut Criterion) {
    let mut graph = ServerGraph::new();
    let session = graph.create_session("bench");
    let window = graph.create_window("w1");
    graph.add_window_to_session(session, window);
    let pane = graph.create_pane(PaneSize::DEFAULT);
    graph.add_pane_to_window(window, pane);
    graph.global_options.set("status", OptionValue::Flag(true));

    c.bench_function("option_resolve_4_levels", |b| {
        b.iter(|| {
            black_box(resolve_option(
                &graph, OptionScope::Pane,
                TargetId::Pane(pane), "status",
            ))
        })
    });
}

fn bench_control_parse(c: &mut Criterion) {
    let lines = vec![
        "%sessions-changed",
        "%session-changed $0 work",
        "%window-add @1",
        "%output %0 hello world",
        "%layout-change @0 d3a0,204x50,0,0{102x50,0,0,0,101x50,103,0}",
    ];
    c.bench_function("control_parse_5_notifications", |b| {
        b.iter(|| {
            for line in &lines {
                black_box(parse_notification(line));
            }
        })
    });
}

fn bench_format_expand(c: &mut Criterion) {
    let graph = ServerGraph::new();
    let session = graph.create_session("work");
    let ctx = FormatContext {
        graph: &graph, session: Some(session),
        window: None, pane: None, client: None,
        runtime_vars: &HashMap::new(),
    };
    c.bench_function("format_expand_status_line", |b| {
        let template = "[#S] #I:#W#{?window_flags,#{window_flags},}";
        b.iter(|| black_box(format_expand(template, &ctx)))
    });
}

fn bench_codec(c: &mut Criterion) {
    use bytes::BytesMut;
    let frame = ImsgFrame {
        header: ImsgHdr {
            msg_type: 200, len: 80, peerid: 0, pid: 12345, has_fd: false,
        },
        payload: BytesMut::from(&[0u8; 64][..]),
    };

    c.bench_function("codec_encode_frame", |b| {
        let mut buf = BytesMut::with_capacity(128);
        let mut codec = ImsgCodec::new();
        b.iter(|| {
            buf.clear();
            codec.encode(black_box(frame.clone()), &mut buf).unwrap();
        });
    });

    c.bench_function("codec_decode_frame", |b| {
        let mut codec = ImsgCodec::new();
        let mut encode_buf = BytesMut::with_capacity(128);
        codec.encode(frame.clone(), &mut encode_buf).unwrap();
        let raw = encode_buf.freeze();
        b.iter(|| {
            let mut buf = BytesMut::from(&raw[..]);
            black_box(codec.decode(&mut buf).unwrap());
        });
    });
}

criterion_group!(
    benches,
    bench_layout_checksum,
    bench_layout_parse,
    bench_layout_dump,
    bench_layout_resize,
    bench_option_resolve,
    bench_control_parse,
    bench_format_expand,
    bench_codec,
);
criterion_main!(benches);
```

### J.2 CI Regression Gate

```yaml
name: Performance Regression Check
on:
  pull_request:
    paths: ['crates/**']
  schedule:
    - cron: '0 4 * * *'

jobs:
  bench:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
    - name: Run benchmarks
      run: |
        cargo bench --bench hot_path -- --output-format bencher | tee output.txt
        test -s output.txt || exit 1
    - name: Compare against baseline
      uses: benchmark-action/github-action-benchmark@v1
      with:
        tool: 'cargo'
        output-file-path: output.txt
        alert-threshold: '130%'
        fail-on-alert: ${{ github.event_name == 'schedule' }}
        comment-on-alert: true
        github-token: ${{ secrets.GITHUB_TOKEN }}
```

**Key decisions:**
- 130% threshold (not 120%) to reduce false positives on shared CI runners.
- Only block on nightly runs, not every PR.
- Assert non-empty output before comparison.
- Baselines must be established from 3 consecutive median runs before enforcement.

---

## K. AGENTS.md Rules

### Error Handling Rules

**Rule 1 -- Error Taxonomy:** Every library crate defines a public `Error` enum deriving `thiserror::Error`. `anyhow` is allowed only in binary crates and test code.

**Rule 2 -- Error Classification:** Every error type implements `Classified` returning `Transient`, `ProtocolViolation`, `UserError`, or `Bug`.

**Rule 3 -- Protocol Violation:** Any binary protocol decode error other than "need more bytes" closes that client connection. Reference: `server-client.c:3472-3475`. Never drop a malformed frame and continue.

**Rule 4 -- Pure Error Boundary:** `io::Error` and platform types must not cross the pure/impure boundary. Convert to stable domain errors via `Event::EffectFailed`.

### Configuration Rules

**Rule 5 -- Config-as-Events:** Config file execution produces `Vec<Event>` submitted through the state actor. Direct graph mutation from config parsing is forbidden.

**Rule 6 -- Option Scope Resolution:** Resolve using option table scope + command flags, matching `options_scope_from_name()` including the FALLTHROUGH from WindowPane to Window (`options.c:903`).

**Rule 7 -- Unset Semantics:** `set -u` removes local override (inheritance restored) for non-global. Resets to compiled default for global. `set -U` additionally clears pane-local values.

### Layout Rules

**Rule 8 -- Layout String Parity:** Layout dump/parse and checksum must match `layout-custom.c` exactly. Verify with roundtrip tests against real tmux layout strings.

**Rule 9 -- Layout Minimum:** `layout_resize()` never fails. Clamps to minimum per `layout_resize_check()`. `PANE_MINIMUM` is 1 cell.

**Rule 10 -- Round-Robin Distribution:** `layout_resize_adjust()` distributes one cell at a time in round-robin, NOT proportionally. Matches `layout.c:448-462`.

**Rule 11 -- Layout Consistency:** After every layout mutation, `layout_check()` must return true. Add as `debug_assert!` in all mutating methods.

### Telemetry Rules

**Rule 12 -- Span Naming:** All span names use `termforge.` prefix. Pattern: `termforge.<subsystem>.<operation>`.

**Rule 13 -- Telemetry Propagation:** New threads, tasks, and spawned processes must attach OTEL context.

### Security Rules

**Rule 14 -- Untrusted Input:** All data from sockets, control mode, config files, and binding FFI is untrusted. Validation at crate boundaries.

**Rule 15 -- SCM_RIGHTS:** FDs via SCM_RIGHTS accepted only during identify handshake. CLOEXEC immediately. No ancillary fds outside handshake.

**Rule 16 -- Control Mode Hints:** Control notifications are hints, never authoritative. Binary protocol is authority. Dropped notifications corrected by periodic refresh.

### Lifecycle Rules

**Rule 17 -- Lock File:** Use `flock(LOCK_EX|LOCK_NB)`, not PID-based locking. Automatically released on process death.

**Rule 18 -- Config Timing:** Do not load config until first client completes identify burst. Matches `server-client.c:3725-3734`.

### DO / DON'T

**DO:**
- Mirror tmux checksum algorithm exactly on wire paths.
- Treat `set -u` as "remove local override", not "write inherited value locally".
- Enforce protocol-violation disconnect for binary protocol peers.
- Separate control-command ingress rate limits from output-backpressure limits.
- Use round-robin distribution for layout resize.
- Validate layout consistency after every mutation (`debug_assert! layout_check`).
- Cite tmux source line numbers for compatibility claims.
- Separate compatibility format from internal persistence format.
- Bind error taxonomy to existing crates.

**DON'T:**
- Claim benchmark regressions are gated until criterion harness is proven running.
- Use proportional resize distribution (produces different geometry from tmux).
- Drop malformed binary frames and continue on the same connection.
- Load config before first client identifies.
- Introduce phantom crates into acceptance criteria (GPT/Gemini correction).
- Conflate policy docs with implemented code paths (GPT correction).
- Treat control mode auth as a protocol concern; it is a socket/ACL concern.

---

## L. Risks and Mitigations

### L.1 Risk Register

| # | Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|---|
| R1 | Layout string format divergence across tmux versions | Low | High | Pin to tmux protocol v8; version-gate parser changes behind protocol version |
| R2 | `flock` behavior differs across OSes (Linux vs macOS vs BSDs) | Medium | Medium | Test on all CI platforms; document in SAFETY.md |
| R3 | HLC counter overflow at same millisecond | Very Low | Medium | `checked_add` with forced millis advance (Section F.1) |
| R4 | Control mode output backpressure causing unbounded memory | Medium | High | Per-client pending output limit (16 MB per Q3); drop oldest %output blocks first |
| R5 | Python GIL contention under heavy concurrent access | Medium | Medium | All blocking ops release GIL; document OrmServer threading model |
| R6 | `options_scope_from_name` fallthrough logic is subtle | High | Medium | Exhaustive test matrix for all (scope, flag, target) combinations |
| R7 | Arena-based layout tree accumulates dead cells | Medium | Low | Compact arena periodically or on preset application (Section C.2) |
| R8 | Proportional resize vs round-robin produces wrong geometry | Resolved | High | Use round-robin matching tmux exactly |
| R9 | CRDT sync over untrusted network | Low (Phase 1 local) | High | Phase 1: Unix sockets only. Phase 2: TLS + mutual auth. Validate all incoming CrdtOps |
| R10 | `mux-conf` parser divergence from tmux parsing | Medium | Medium | Corpus of 100+ real tmux.conf files; verify identical parse results |
| R11 | 16-bit checksum collision for internal persistence | Low | Low | BLAKE3-128 envelope for persistence; tmux checksum for wire only |
| R12 | PID reuse in stale lock handling | Resolved | Medium | Use flock instead of PID-based locking |
| R13 | Control output backlog unfair scheduling among panes | Low | Low | Per-pane output quotas within per-client limit |
| R14 | Multiple FDs in one ancillary message | Low | Medium | Close all unexpected extra FDs deterministically |
| R15 | Array option index unset may diverge from tmux | Medium | Medium | Mirror `options_remove_or_default` index handling at `options.c:1282` |
| R16 | `mux-types` crate does not exist in workspace | Certain | Medium | Create as first action; leaf crate with `thiserror`, `serde`, `smallvec` only |
| R17 | Criterion benchmarks not wired (harness misconfigured) | Certain | Low | Fix `[[bench]]` section; CI asserts non-empty output |

### L.2 Reference Verification Table

Every major claim was checked against actual source. Status as of verification:

| Claim | Status | Evidence |
|---|---|---|
| Identify burst types 100-112 | Verified | `tmux-protocol.h:29-41`, `crates/mux-proto/src/msg.rs:40-67` |
| `goto bad -> proc_kill_peer` | Verified | `server-client.c:3472-3475` |
| `layout_checksum` algorithm | Verified | `layout-custom.c:46-57` (rotate-right + add) |
| `layout_resize_adjust` round-robin | Verified | `layout.c:448-462` (while loop, one cell at a time) |
| `layout_resize_check` computation | Verified | `layout.c:366-415` (leaf/same/perp logic) |
| `layout_check` validation | Verified | `layout-custom.c:119-153` (border = +1 per child, -1 total) |
| `PANE_MINIMUM = 1` | Verified | `tmux.h:100` |
| `options_get` parent chain walk | Verified | `options.c:228-241` |
| `options_remove_or_default` | Verified | `options.c:1269-1285` |
| `options_scope_from_name` FALLTHROUGH | Verified | `options.c:891-903` (line 903: `/* FALLTHROUGH */`) |
| `client_get_lock` uses flock | Verified | `client.c:77-101` |
| Config load after first client | Verified | `server-client.c:3725-3734` |
| `control_start` no auth | Verified | `control.c:758-796` |
| `%extended-output` format | Verified | `control.c:620-623` |
| Control backpressure/disconnect | Verified | `control.c:450-461` |
| Socket permissions umask | Verified | `server.c:126-129` |
| `server_acl_join` ACL check | Verified | `server-acl.c:164` |
| SCM_RIGHTS CLOEXEC in vibe-tmux | Verified | `crates/mux-os/src/scm_rights.rs:141-149` |
| Current `ControlNotification` is generic | Verified | `crates/mux-client/src/control.rs:30-36` (name/raw struct) |
| `mux-types` crate exists | Missing | Not in workspace -- must be created |
| `LockFile` in vibe-tmux | Missing | No implementation yet |
| `HybridClock` in vibe-tmux | Missing | No implementation yet |
| `ValidatedFd` in vibe-tmux | Missing | Not a type; CLOEXEC handled functionally |
| Criterion harness working | Missing | `mux-refresh/Cargo.toml` lacks `[[bench]]` |
| Split minimum check | Verified | `layout.c:937-950` (`PANE_MINIMUM * 2 + 1`) |

---

## Summary of v5 Changes from v4

| Area | v4 State | v5 Change |
|---|---|---|
| Error Classification | Per-crate errors only | Added `ErrorClass` taxonomy with `Classified` trait (all 3 models converged) |
| Codec Recovery | Unspecified | `ProtocolViolation` -> kill connection (corrected from "drop frame") |
| Layout Resize | Unspecified | Round-robin one-cell-at-a-time (corrected from proportional) |
| Layout Validation | Missing | Added `layout_check()` from Gemini, verified against `layout-custom.c:119-153` |
| Layout Arena | No compaction | Added `compact()` method |
| Lock File | Unspecified | `flock(LOCK_EX|LOCK_NB)` (corrected from PID-based) |
| Option Unset | Unspecified | Remove local override / reset default / `-U` cascade -- fully specified |
| Option Scope | Simplified | Added `WindowPane` fallthrough matching `options.c:903` |
| Control Auth | Unspecified | Socket permissions + ACL only; optional rate limiter |
| Control Parser | Unspecified | Typed enum + migration path from generic struct |
| Control Notifications | Basic | Added `ExtendedOutput` variant |
| CRDT Clocks | Vector clocks mentioned | HLC + DVV with compaction; `checked_add` overflow safety |
| Benchmark Baseline | Claimed targets | No baselines exist; conservative targets; 130% CI gate |
| Benchmark Wiring | Missing harness | Documented `[[bench]]` fix for `mux-refresh` |
| Python Async | Unspecified | 3-phase: sync -> asyncio.to_thread -> native streaming |
| Layout Minimum | Unspecified | Clamp to minimum, never fail; `ResizeResult` |
| Config Timing | Unspecified | Load only after first client identifies |
| AGENTS.md | v4 rules | 18 concrete rules + DO/DON'T list |
| Span Names | Short names | `termforge.` prefix convention |
| Risks | v4 risks | 17 new risks with mitigations + resolution status |
| Reference Verification | Not done | Full existence check table with verified/missing status |
| Test Strategies | Sparse | Every section has 5-8 concrete test strategies |
| Source Verification | Assumed | All line references independently verified against source |
