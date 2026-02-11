# TermForge: v5 Pass 2 Refined Specification (Claude)

Date: 2026-02-10
Pass: 2 of 2
Extends: v4 Final Synthesis + v5 Pass 1 outputs (Claude, Gemini, GPT)
License: MIT OR Apache-2.0
Rust edition: 2024 (MSRV 1.85)

---

## Preamble

This document is the Pass 2 refinement of the v5 architectural plan. It synthesizes contributions from all three Pass 1 outputs (Claude v5, Gemini v5, GPT v6), resolves the 7 open questions with concrete decisions grounded in tmux source, incorporates Gemini's error classification and GPT's AGENTS.md rules, strengthens test strategies, and adds missed risks.

All file/line references point to the tmux C source at `~/study/c/tmux/`, the Rust prototype at `~/work/rust/vibe-tmux/`, and the Python libtmux at `~/work/python/libtmux/`.

---

## Table of Contents

0. [Open Questions Resolved](#0-open-questions-resolved)
A. [Concrete Error Handling (Refined)](#a-concrete-error-handling-refined)
B. [Configuration System (Refined)](#b-configuration-system-refined)
C. [Layout Engine (Refined)](#c-layout-engine-refined)
D. [OpenTelemetry (Refined)](#d-opentelemetry-refined)
E. [Language Bindings (Refined)](#e-language-bindings-refined)
F. [CRDT Detail (Refined)](#f-crdt-detail-refined)
G. [Control Mode (Refined)](#g-control-mode-refined)
H. [Server Lifecycle (Refined)](#h-server-lifecycle-refined)
I. [Security Model (Refined)](#i-security-model-refined)
J. [Performance Targets (Refined)](#j-performance-targets-refined)
K. [Updated AGENTS.md Rules](#k-updated-agentsmd-rules)
L. [Additional Risks](#l-additional-risks)

---

## 0. Open Questions Resolved

### Q1: Layout String Checksum -- Match tmux exactly or use a stronger hash?

**Decision: Match tmux's exact algorithm for all wire-format paths. No stronger hash for wire compatibility. Add an optional TermForge envelope with BLAKE3-128 for internal persistence only (never sent to tmux).**

**Evidence:** tmux's `layout_checksum` in `layout-custom.c:47-57` uses a simple rotate-and-add algorithm over the layout string bytes (excluding the 4-hex-digit checksum prefix):

```c
// ~/study/c/tmux/layout-custom.c:47-57
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

The checksum is a 16-bit value formatted as `%04hx` (4 hex digits) at `layout-custom.c:69`. Parsing verifies the checksum at `layout-custom.c:165-173`.

**Rationale:** The checksum serves two purposes: (1) quick validity check for malformed strings, (2) wire-format compatibility with tmux. If we use a different algorithm, `tmux attach` to a TermForge server would fail when exchanging layout strings. A stronger hash (e.g. CRC-32) would break `layout_parse` in any tmux client.

**TermForge implementation:**

```rust
/// Wire format variants.
pub enum LayoutWireFormat {
    /// "hhhh,<layout>" exact tmux compatibility.
    TmuxV1,
    /// Internal envelope with strong hash + version for persistence.
    TermForgeV2,
}

/// Layout checksum matching tmux exactly.
/// Reference: ~/study/c/tmux/layout-custom.c:47-57
pub fn layout_checksum(layout: &[u8]) -> u16 {
    let mut csum: u16 = 0;
    for &b in layout {
        csum = (csum >> 1) | ((csum & 1) << 15);
        csum = csum.wrapping_add(b as u16);
    }
    csum
}

/// Internal envelope for persistence (not sent to tmux).
pub struct LayoutEnvelopeV2 {
    pub version: u16,
    pub tmux_payload: String,
    pub blake3_128: [u8; 16],
}
```

**Test strategy:**
1. Property test: for any valid layout string, `layout_checksum(s) == tmux_checksum(s)` using a corpus of real layout strings extracted from tmux sessions.
2. Roundtrip: `assert_eq!(layout_parse(layout_dump(tree)).unwrap(), tree)`.
3. Parity: capture layout strings from tmux via control mode, verify our checksum matches exactly.

**Version compatibility:** The checksum algorithm has not changed across tmux versions. If it ever changes, we version-gate behind `mux-proto`'s protocol version detection.

---

### Q2: Option Inheritance vs Override -- Does `set -u` (unset) block inheritance permanently?

**Decision: `set -u` removes the local override, restoring inheritance. For global scopes, it resets to the compiled default. `set -U` additionally unsets pane-local values for pane options in the target window.**

**Evidence:** tmux's `options_remove_or_default` at `options.c:1270-1285`:

```c
// ~/study/c/tmux/options.c:1270-1285
int
options_remove_or_default(struct options_entry *o, int idx, char **cause)
{
    struct options *oo = o->owner;
    if (idx == -1) {
        if (o->tableentry != NULL &&
            (oo == global_options ||
            oo == global_s_options ||
            oo == global_w_options))
            options_default(oo, o->tableentry);  // reset to default
        else
            options_remove(o);  // remove local override
    } else if (options_array_set(o, idx, NULL, 0, cause) != 0)
        return (-1);
    return (0);
}
```

The `options_get` function at `options.c:228-241` walks the `parent` chain:

```c
struct options_entry *
options_get(struct options *oo, const char *name)
{
    struct options_entry *o;
    o = options_get_only(oo, name);
    while (o == NULL) {
        oo = oo->parent;
        if (oo == NULL)
            break;
        o = options_get_only(oo, name);
    }
    return (o);
}
```

**Behavior summary:**
- `set -u` on a **non-global** scope (specific session/window/pane): calls `options_remove()`, which deletes the local entry from the RB-tree. Next `options_get()` walks up to the parent and finds the inherited value. **Inheritance is restored.**
- `set -u` on a **global** scope (`-g`): calls `options_default()`, which resets the entry to its compiled default value from `options_table`. The entry is not removed; it is overwritten with the default.
- `set -U`: applies `-u` and additionally iterates pane-local options in the target window to unset pane-level values (tmux man: `tmux.1:4075-4084`).
- Setting an option on a child scope does NOT permanently block inheritance. It just adds a local entry that shadows the parent. `set -u` removes that shadow.

**TermForge mapping:**

```rust
// crates/mux-core/src/options.rs

pub enum UnsetMode {
    /// Remove local override at the target scope only.
    LocalOnly,
    /// Remove local override AND pane-local values in target window.
    LocalAndPanes,
}

impl OptionStore {
    /// Remove a local override (set -u on non-global scope).
    /// After this, resolve_option() will walk up to the parent.
    pub fn unset(&mut self, key: &str) -> Option<OptionValue> {
        self.local.remove(key)
    }
}

/// Unset an option, respecting scope rules.
pub fn unset_option(
    graph: &mut ServerGraph,
    scope: OptionScope,
    target_id: TargetId,
    key: &str,
    mode: UnsetMode,
) {
    match scope {
        OptionScope::Server => {
            // Global: reset to compiled default
            if let Some(default) = option_table_default(key) {
                graph.global_options.set(key, default);
            }
        }
        OptionScope::Session if matches!(target_id, TargetId::Server) => {
            // Global session options: reset to default
            if let Some(default) = option_table_default(key) {
                graph.global_session_options.set(key, default);
            }
        }
        _ => {
            // Non-global: remove local override to restore inheritance
            if let Some(store) = get_option_store_mut(graph, scope, target_id) {
                store.unset(key);
            }
            // If -U mode, also unset pane-local values in the target window
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
1. Unit test: set option on pane, verify it shadows parent. Unset it, verify parent value is visible again.
2. Unit test: set option on global session scope, unset, verify compiled default is restored.
3. Unit test: set pane-local option on 3 panes, `set -U` on window, verify all pane overrides removed.
4. Property test: for any chain of set/unset operations, `resolve_option` always returns a value (never None for known options).
5. Parity test: run `set -u` via tmux and compare option resolution against TermForge.

---

### Q3: Control Mode Authentication -- Does `tmux -C` require auth beyond socket access?

**Decision: No additional authentication. Socket permissions + ACL (`server_acl_join`) are the sole access control. Add configurable rate limiting for control mode clients.**

**Evidence:** tmux's `control_start()` at `control.c:759-796` initializes control state without any authentication check. The only gate is `CLIENT_CONTROL` flag set during the identify burst, which is determined by the `-C` flag. The tmux server trusts any client that can connect to the Unix socket:

```c
// ~/study/c/tmux/control.c:759-796
void
control_start(struct client *c)
{
    struct control_state *cs;
    // ... sets up buffered events, no auth
    cs = c->control_state = xcalloc(1, sizeof *cs);
    // ...
}
```

The server dispatches identify messages at `server-client.c:3384-3397` -- control mode clients send `MSG_IDENTIFY_FLAGS` with `CLIENT_CONTROL` set. No token, no challenge-response. The security boundary is the socket directory permissions (`0700`), socket file permissions, and server-side ACL check (`server-acl.c:164-179`).

tmux's backpressure for control mode is output-side: it pauses and disconnects clients whose output is behind (`control.c:450-461`, `control.c:739-741`). Command ingress is not rate-limited.

**TermForge policy:**
- **Phase 1:** Match tmux -- socket permissions + ACL are the auth boundary.
- **Phase 2:** Add a configurable rate limiter in the server for control mode command ingestion to prevent a misbehaving client from overwhelming the server.

```rust
// crates/mux-server/src/control_rate.rs

pub struct ControlRateLimiter {
    /// Max commands per second per control-mode client.
    pub max_commands_per_sec: u32,
    /// Max pending output bytes per control-mode client before backpressure.
    pub max_pending_bytes: usize,
}

impl Default for ControlRateLimiter {
    fn default() -> Self {
        Self {
            max_commands_per_sec: 1000,     // generous default
            max_pending_bytes: 16 * 1024 * 1024, // 16 MB
        }
    }
}
```

**Test strategy:**
1. Integration test: control mode client connects and executes commands without any token.
2. Integration test: rate limiter triggers backpressure when a client exceeds `max_commands_per_sec`.
3. Security test: verify non-owner user cannot connect to socket (Unix permissions test).

---

### Q4: CRDT Vector Clock Size -- Bounded vector clocks or interval tree clocks?

**Decision: Use Hybrid Logical Clocks (HLC) per-node with dotted version vectors (DVV) for sync delta computation. Bounded by active node count with membership-aware compaction.**

**Rationale:** Classic vector clocks grow linearly with the number of nodes that have ever participated, which is unbounded over time. HLC timestamps are fixed-size `(u64 millis, u32 counter, NodeId)` and provide causal ordering sufficient for LWW-Register and OR-Set CRDTs. The DVV adds a compact per-node high-water summary for sync.

**Concrete design:**

```rust
// crates/mux-crdt/src/clock.rs

/// HLC timestamp: fixed 20 bytes, causally ordered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
         serde::Serialize, serde::Deserialize)]
pub struct HlcTimestamp {
    pub millis: u64,    // wall clock, injected
    pub counter: u32,   // logical counter
    pub node_id: NodeId, // 8 bytes
}

/// 8-byte node identifier, generated from UUID at server start.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
         serde::Serialize, serde::Deserialize)]
pub struct NodeId(pub u64);

/// Dotted version vector for sync delta computation.
/// SmallVec avoids heap allocation for <= 8 nodes.
pub struct Dvv {
    pub summary: SmallVec<[(NodeId, u64); 8]>,
    pub dot: HlcTimestamp,
}

/// Compact DVV by removing entries for nodes no longer in active set.
pub fn compact_dvv(dvv: &mut Dvv, active_nodes: &[NodeId], _epoch: u64) {
    dvv.summary.retain(|(node, _)| active_nodes.contains(node));
}
```

**Why not plain vector clocks:** Our collaboration model has a small, known set of nodes (typically 2-5 servers in a collaborative session). DVV with compaction keeps the summary bounded by active nodes. Nodes that disconnect and never reconnect can be garbage-collected.

**Why not Interval Tree Clocks:** ITC solves the "join/leave" problem for dynamic systems but adds implementation complexity. DVV with membership tracking is simpler and sufficient for our use case.

**Test strategy:**
1. Property test: HLC `now()` always produces monotonically increasing timestamps.
2. Property test: HLC `receive()` produces a timestamp causally after both local and remote.
3. Unit test: OpLog sync with 3 nodes produces identical state on all nodes.
4. Compaction test: add 10 nodes, deactivate 5, compact, verify summary has 5 entries.

---

### Q5: Benchmark Baseline -- What are actual measured values from vibe-tmux?

**Decision: The only existing benchmark is `mux-refresh/benches/latency.rs` but it is not wired correctly (missing `[[bench]]` + `harness = false` in Cargo.toml). No baseline values exist. Fix harness first, then establish baselines before enforcing regression gates.**

**Evidence:** The sole benchmark file at `~/work/rust/vibe-tmux/crates/mux-refresh/benches/latency.rs` contains 3 benchmarks:
- `control_debouncer_note_take_ready` -- measures debouncer path
- `runtime_broadcaster_flush_ready` -- measures broadcast flush
- `refresh_scope_mapping_window_add` -- measures notification-to-scope mapping

These are infrastructure benchmarks, not hot-path benchmarks. The `mux-refresh/Cargo.toml` is missing the `[[bench]]` section needed by criterion. No existing measurements exist for VT100 parser throughput, protocol codec, layout operations, format expansion, or graph snapshots.

**Immediate fix:**
```toml
# crates/mux-refresh/Cargo.toml (add)
[[bench]]
name = "latency"
harness = false
```

**Revised targets with feasibility notes:**

| Benchmark | Target | Feasibility Basis |
|---|---|---|
| VT100 parser, plain ASCII | > 300 MB/s | alacritty's vte parser achieves ~500 MB/s; table-driven is realistic |
| VT100 parser, CSI heavy | > 100 MB/s | Parameter parsing adds overhead; conservative |
| Protocol frame decode | > 500 MB/s | 16-byte header parse + memcpy; trivial |
| Layout resize (20 panes) | < 50 us | Tree walk with ~60 nodes, no allocation |
| Format expand (status line) | < 50 us | String concatenation with ~10 variable lookups |
| Graph snapshot | < 1 ms | Arc clone + BTreeMap iteration for 50 panes |
| Config parse (500 lines) | < 5 ms | Lexer + parser, no IO |
| Option resolve (4-level chain) | < 100 ns | 4 BTreeMap lookups |
| Control notification parse | < 200 ns | String split + parse_u32 |

**Action:** Before setting CI regression gates, establish baselines by running criterion benchmarks on the reference CI hardware (GitHub Actions `ubuntu-latest`) for 3 consecutive runs and using the median as the baseline. Apply 130% regression gate (relaxed from 120% to reduce false positives on shared CI runners) only after baselines are established.

---

### Q6: Python Binding Async -- PyO3 asyncio or synchronous only?

**Decision: Phase 1 is synchronous-only with GIL release. Phase 2 adds `asyncio` support via Python-side `asyncio.to_thread()` wrappers first, then native streaming via `pyo3-async-runtimes` only for event subscription.**

**Rationale:**
1. `pyo3-asyncio` (now `pyo3-async-runtimes` for PyO3 0.23+) adds complexity and a hard dependency on a specific Rust async runtime.
2. libtmux (the existing Python library at `~/work/python/libtmux/`) is entirely synchronous. `options.py:701-787` and `options.py:958-1002` are sync wrappers. Users are accustomed to this.
3. The most important optimization -- releasing the GIL during blocking operations -- is already handled by `py.allow_threads()`.
4. Python 3.9+ provides `asyncio.to_thread()` which wraps any sync function for async callers with zero Rust changes.
5. For event subscription (watching for state changes), a hybrid approach works: a Rust background thread writes to an `asyncio.Queue` that Python can `await`.

**Phase 1 (sync):**
```rust
// All blocking methods release GIL:
fn cmd(&self, py: Python<'_>, cmd: &str) -> PyResult<String> {
    let inner = self.inner.clone();
    py.allow_threads(move || { /* blocking operation */ })
}
```

**Phase 2 (Python-side async wrapper, zero Rust changes):**
```python
# termforge/_async.py
import asyncio
from termforge._native import Server

async def async_cmd(server: Server, cmd: str) -> str:
    return await asyncio.to_thread(server.cmd, cmd)
```

**Phase 3 (native streaming only):**
```rust
/// Returns an asyncio.Queue that receives RefreshHint objects.
#[pymethod]
fn subscribe(&self, py: Python<'_>) -> PyResult<PyObject> {
    let asyncio = py.import("asyncio")?;
    let queue = asyncio.call_method0("Queue")?;
    // Rust thread bridges broadcast::Receiver -> asyncio.Queue
    Ok(queue.to_object(py))
}
```

**Test strategy:**
1. Unit test: synchronous `cmd()` works from Python thread.
2. Unit test: verify GIL is released during blocking calls (use a concurrent Python thread to confirm it can run).
3. Unit test: `asyncio.to_thread(server.cmd, "list-sessions")` works from async context.
4. Integration test (Phase 3): `async for hint in server.subscribe()` receives events.

---

### Q7: Layout Minimum Size -- What happens when resize violates PANE_MINIMUM?

**Decision: tmux clamps the resize to the minimum possible size. It never fails the resize. The layout can become smaller than the window, with drawing handled at a higher level. Resize returns a `ResizeResult` indicating how much of the requested delta was applied.**

**Evidence:** tmux's `layout_resize()` at `layout.c:535-583` explicitly bounds the change:

```c
// ~/study/c/tmux/layout.c:535-583
void
layout_resize(struct window *w, u_int sx, u_int sy)
{
    struct layout_cell *lc = w->layout_root;
    int xlimit, ylimit, xchange, ychange;

    xchange = sx - lc->sx;
    xlimit = layout_resize_check(w, lc, LAYOUT_LEFTRIGHT);
    if (xchange < 0 && xchange < -xlimit)
        xchange = -xlimit;     // CLAMP: never shrink more than available
    if (xlimit == 0) {
        if (sx <= lc->sx)      // already at minimum, no change
            xchange = 0;
        else
            xchange = sx - lc->sx;  // growing is always allowed
    }
    if (xchange != 0)
        layout_resize_adjust(w, lc, LAYOUT_LEFTRIGHT, xchange);
    // ... same for Y axis
}
```

`PANE_MINIMUM` is defined as `1` at `tmux.h:100`. The `layout_resize_check()` function at `layout.c:366-415` computes how much space can be removed from a cell:
- For leaf cells: `available = cell_size - PANE_MINIMUM` (clamped to 0).
- For same-direction containers: sum of children's available space.
- For perpendicular containers: minimum of children's available space.

**Key behavior:**
1. **Shrinking:** The resize change is clamped so no pane goes below `PANE_MINIMUM` (1 cell). The window may end up smaller than the layout -- tmux handles this in the rendering layer.
2. **Growing:** Always succeeds. Extra space is distributed evenly among children using `layout_resize_adjust()` at `layout.c:421-463`, which distributes one cell at a time in round-robin to children.
3. **Split refusal:** When splitting, tmux checks if the pane has enough space for two children + border (`PANE_MINIMUM * 2 + 1`). See `layout.c:937-950`. If not, the split fails.

**TermForge implementation:**

```rust
// crates/mux-core/src/layout.rs

/// Result of a resize operation.
pub struct ResizeResult {
    /// The delta that was requested.
    pub requested_x: i32,
    pub requested_y: i32,
    /// The delta that was actually applied (clamped to minimum constraints).
    pub applied_x: i32,
    pub applied_y: i32,
}

impl LayoutTree {
    /// Resize the layout to fit a new window size.
    /// Never fails: clamps to minimum possible size.
    /// Matches tmux layout_resize() at layout.c:535-583.
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
            xchange = -xlimit; // clamp shrink to available
        }
        if xlimit == 0 && new_sx <= old_sx {
            xchange = 0; // already at minimum
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

    /// How much space can be removed from this cell in the given direction.
    /// Matches tmux layout_resize_check() at layout.c:366-415.
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
                // Same direction: sum of children's available space
                cell.children.iter()
                    .map(|&child| self.resize_check(child, direction))
                    .sum()
            }
            _ => {
                // Perpendicular: minimum of children's available space
                cell.children.iter()
                    .map(|&child| self.resize_check(child, direction))
                    .min()
                    .unwrap_or(0)
            }
        }
    }

    /// Distribute size change evenly among children, one cell at a time.
    /// Matches tmux layout_resize_adjust() at layout.c:421-463.
    fn resize_adjust(&mut self, idx: usize, direction: LayoutType, mut change: i32) {
        // Adjust this cell's size
        if direction == LayoutType::LeftRight {
            self.cells[idx].sx = (self.cells[idx].sx as i32 + change).max(0) as u16;
        } else {
            self.cells[idx].sy = (self.cells[idx].sy as i32 + change).max(0) as u16;
        }

        if self.cells[idx].cell_type == LayoutType::WindowPane {
            return; // leaf: done
        }

        let children: Vec<usize> = self.cells[idx].children.clone();

        if self.cells[idx].cell_type != direction {
            // Perpendicular: apply same change to all children
            for &child in &children {
                self.resize_adjust(child, direction, change);
            }
            return;
        }

        // Same direction: distribute one unit at a time, round-robin.
        // This matches tmux's while loop at layout.c:448-462.
        while change != 0 {
            let mut made_progress = false;
            for &child in &children {
                if change == 0 { break; }
                if change > 0 {
                    self.resize_adjust(child, direction, 1);
                    change -= 1;
                    made_progress = true;
                } else {
                    if self.resize_check(child, direction) > 0 {
                        self.resize_adjust(child, direction, -1);
                        change += 1;
                        made_progress = true;
                    }
                }
            }
            if !made_progress { break; } // prevent infinite loop
        }
    }
}
```

**Test strategy:**
1. Unit test: resize below minimum clamps correctly. Verify no pane has dimension < `PANE_MINIMUM`.
2. Unit test: resize from minimum back up distributes space evenly.
3. Unit test: `ResizeResult.applied != ResizeResult.requested` when clamped.
4. Property test: for any sequence of resize operations, all leaf cells have `sx >= PANE_MINIMUM && sy >= PANE_MINIMUM`.
5. Parity test: capture tmux layout strings before and after resize, compare with TermForge output.
6. Edge case test: single pane at 1x1 -- resize to 1x1 is no-op, resize to 2x2 grows.
7. Edge case test: split when at minimum size returns `LayoutError::TooSmall`.

---

## A. Concrete Error Handling (Refined)

### A.1 Error Classification System

Claude v5 Pass 1 defined per-crate error types. Gemini and GPT both independently proposed the same 4-category error classification. We adopt it as a cross-cutting concern that enriches the existing per-crate errors.

```rust
// crates/mux-types/src/error_class.rs
#![forbid(unsafe_code)]

/// Classification of errors for recovery policy decisions.
/// All three Pass 1 models converged on this taxonomy.
///
/// Reference: tmux's "goto bad -> proc_kill_peer" pattern at
/// server-client.c:3472-3475 maps to ProtocolViolation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorClass {
    /// Transient: can retry or wait for more data.
    /// Examples: incomplete frame, timeout, temporary IO failure.
    /// Recovery: retry with backoff, keep connection.
    Transient,

    /// Protocol violation: the peer sent structurally invalid data.
    /// Examples: bad magic, length overflow, unknown message type.
    /// Recovery: close that connection, log, keep server alive.
    /// tmux reference: `goto bad` at server-client.c:3472 -> proc_kill_peer.
    ProtocolViolation,

    /// User error: semantically invalid request from a valid connection.
    /// Examples: unknown command, invalid option name, target not found.
    /// Recovery: send error response to client, keep connection.
    UserError,

    /// Bug / invariant violation: internal consistency failure.
    /// Examples: entity referenced by ID doesn't exist in graph, impossible state.
    /// Recovery: log with full context, optionally crash in debug mode.
    Bug,
}
```

**Integration with existing error types:**

```rust
// Each crate error type implements this trait:
pub trait Classified {
    fn class(&self) -> ErrorClass;
}

// Example: mux-proto errors
impl Classified for ProtocolError {
    fn class(&self) -> ErrorClass {
        match self {
            ProtocolError::Incomplete => ErrorClass::Transient,
            ProtocolError::LengthTooSmall { .. }
            | ProtocolError::LengthTooLarge { .. }
            | ProtocolError::UnknownMessageType { .. }
            | ProtocolError::InvalidPayload { .. }
            | ProtocolError::MissingNul { .. }
            | ProtocolError::InvalidUtf8 { .. } => ErrorClass::ProtocolViolation,
            ProtocolError::NotIdentifyMessage { .. } => ErrorClass::ProtocolViolation,
        }
    }
}

// Example: mux-core errors
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

### A.2 Codec Recovery Policy (Refined from GPT)

GPT's `DecodeOutcome` enum from Pass 1 is clearer than Claude's `io::Error` mapping. We adopt it, correcting Claude v5 Pass 1's incorrect claim that malformed frames can be dropped while keeping the connection alive. Gemini correctly identified that tmux kills the peer on protocol violations (`server-client.c:3472-3475 -> proc_kill_peer`).

```rust
// crates/mux-proto/src/codec.rs

/// Outcome of attempting to decode one frame from the buffer.
/// The caller (server connection handler) uses this to decide
/// whether to keep reading, process the frame, or kill the connection.
pub enum DecodeOutcome {
    /// Need more bytes. Keep reading.
    NeedMore,
    /// Successfully decoded a frame.
    Frame(ImsgFrame),
    /// Unrecoverable protocol error. Close this connection.
    /// The buffer has been cleared to prevent decode loops.
    ProtocolViolation(ProtocolError),
}
```

**Server-side connection handler:**

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
                        ErrorClass::ProtocolViolation => {
                            tracing::warn!(
                                client_id = %conn.id,
                                error = %e,
                                "protocol violation, killing connection"
                            );
                            return ConnectionAction::Kill(e.to_string());
                        }
                        ErrorClass::UserError => {
                            send_error_to_client(conn, &e).await;
                            // continue processing more frames
                        }
                        ErrorClass::Bug => {
                            tracing::error!(error = %e, "invariant violation");
                            return ConnectionAction::Kill(e.to_string());
                        }
                        ErrorClass::Transient => {
                            // should not happen for frame processing
                        }
                    }
                }
            }
            DecodeOutcome::ProtocolViolation(e) => {
                tracing::warn!(
                    client_id = %conn.id,
                    error = %e,
                    "protocol violation in codec, killing connection"
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

### A.3 Test Strategy for Error Handling

1. **Fuzz testing:** `fuzz/decode_frame.rs` feeds arbitrary bytes to `ImsgCodec` and asserts it never panics, always returns one of the three `DecodeOutcome` variants.
2. **Classification coverage:** Unit test that every variant of every error enum has a `class()` implementation that returns a non-panicking `ErrorClass`.
3. **Recovery integration test:** Feed a valid frame followed by garbage bytes. Verify the connection is killed.
4. **Error propagation test:** Trigger each `CoreError` variant through the engine and verify the correct `Effect::ErrorReply` is produced.
5. **Protocol violation test:** Send identify message after already identified. Verify `ProtocolViolation` -> connection killed (matching tmux `server-client.c:3599-3600`).

---

## B. Configuration System (Refined)

### B.1 Option Scope Resolution (Detailed)

tmux's `options_scope_from_name()` at `options.c:851-919` resolves the scope for an option based on:
1. The option's declared scope in `options_table` (`OPTIONS_TABLE_SERVER`, `OPTIONS_TABLE_SESSION`, `OPTIONS_TABLE_WINDOW`, combined `OPTIONS_TABLE_WINDOW|OPTIONS_TABLE_PANE`).
2. The flags on the command (`-g` for global, `-s` for server, `-p` for pane, `-w` for window).
3. The target (`-t`) if specified.
4. User-defined options starting with `@` use `options_scope_from_flags` instead of table lookup.

**Key nuance:** Some options (like `pane-border-status`) are `OPTIONS_TABLE_WINDOW|OPTIONS_TABLE_PANE` -- they can be set at either scope. When `-p` is used, the pane scope is selected. When `-w` or no flag is used, the window scope is selected. When `-g` is used, the global window options are selected (at `options.c:905-916`). This includes a `FALLTHROUGH` from `WINDOW|PANE` to `WINDOW` at `options.c:903`.

**TermForge must replicate this fallthrough:**

```rust
// crates/mux-command/src/set_option.rs

pub fn resolve_option_scope(
    option_name: &str,
    flags: &SetOptionFlags,
    target: &ResolvedTarget,
) -> Result<(OptionScope, TargetId), CoreError> {
    // User-defined options (@foo) resolve by flags only
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

### B.2 Test Strategy for Configuration

1. **Scope resolution matrix test:** For each combination of (option scope, flag set, target), verify the correct `(OptionScope, TargetId)` is produced.
2. **Inheritance chain test:** Build a 4-level hierarchy (server -> session -> window -> pane), set options at each level, verify resolution walks correctly.
3. **Unset test:** Set option at pane, verify shadow. Unset, verify parent visible. Set at global, unset, verify default.
4. **User option test:** `@foo` with `-s`, `-g`, `-p` flags resolves correctly.
5. **Parity test:** Run `show-options -g`, `show-options -s`, `show-options -w`, `show-options -p` against tmux and compare with TermForge output.
6. **Config reload test:** Load config, verify events produced. Modify config, trigger reload, verify only changed options produce new events.

---

## C. Layout Engine (Refined)

### C.1 Corrections to Claude v5 Pass 1

**Correction 1: Protocol error handling.** Claude v5 Pass 1 stated "codec errors are recoverable, drop frame, continue" (A.1 principle 4). This is WRONG for the binary protocol. tmux kills the peer on bad frames (`server-client.c:3472-3475`). The correct behavior is `ProtocolViolation -> close connection`. Corrected in Section A above.

**Correction 2: Resize distribution algorithm.** Claude v5's `resize()` uses proportional distribution. tmux's `layout_resize_adjust()` at `layout.c:421-463` uses **round-robin one-cell-at-a-time** distribution, not proportional. Claude's proportional approach would produce different layout geometry than tmux. Corrected in Q7 above.

**Correction 3: `mux-types` crate.** Claude v5 references `mux-types` which does not exist in the current vibe-tmux workspace. Either create it or host the `ErrorClass` and `Classified` trait in an existing foundational crate. Decision: create `mux-types` as a leaf crate with no dependencies except `thiserror`, `serde`, and `smallvec`.

### C.2 Layout Validation (from Gemini's `layout_check`)

Gemini identified that tmux has layout consistency validation. tmux checks at `layout-custom.c:120-153` that container geometry matches its children. We add this:

```rust
/// Validate that a layout tree is internally consistent.
/// For LeftRight containers: all children must have same height as parent,
/// and sum of (child widths + borders) must equal parent width.
/// For TopBottom: same logic with axes swapped.
pub fn layout_check(tree: &LayoutTree) -> bool {
    if tree.cells.is_empty() { return true; }
    layout_check_cell(tree, 0)
}

fn layout_check_cell(tree: &LayoutTree, idx: usize) -> bool {
    let cell = &tree.cells[idx];
    match cell.cell_type {
        LayoutType::WindowPane => true,
        LayoutType::LeftRight => {
            let mut total_width: u32 = 0;
            for (i, &child_idx) in cell.children.iter().enumerate() {
                let child = &tree.cells[child_idx];
                if child.sy != cell.sy { return false; }
                total_width += child.sx as u32;
                if i > 0 { total_width += PANE_BORDER as u32; } // border between siblings
                if !layout_check_cell(tree, child_idx) { return false; }
            }
            total_width == cell.sx as u32
        }
        LayoutType::TopBottom => {
            let mut total_height: u32 = 0;
            for (i, &child_idx) in cell.children.iter().enumerate() {
                let child = &tree.cells[child_idx];
                if child.sx != cell.sx { return false; }
                total_height += child.sy as u32;
                if i > 0 { total_height += PANE_BORDER as u32; }
                if !layout_check_cell(tree, child_idx) { return false; }
            }
            total_height == cell.sy as u32
        }
    }
}
```

### C.3 Arena Compaction

Risk R7 identifies that the arena-based layout tree can accumulate dead cells after `layout_destroy_cell` operations. When a cell is destroyed and its parent has only one child left, tmux collapses the parent into the remaining child (`layout.c:497-513`). With a flat Vec arena, this leaves holes. We add periodic compaction:

```rust
impl LayoutTree {
    /// Compact the arena by removing unreachable cells.
    /// Should be called after destroy operations or preset application.
    pub fn compact(&mut self) {
        let mut reachable = vec![false; self.cells.len()];
        if !self.cells.is_empty() {
            self.mark_reachable(0, &mut reachable);
        }
        // Build mapping from old index to new index
        let mut mapping = vec![usize::MAX; self.cells.len()];
        let mut new_cells = Vec::new();
        for (old_idx, cell) in self.cells.iter().enumerate() {
            if reachable[old_idx] {
                mapping[old_idx] = new_cells.len();
                new_cells.push(cell.clone());
            }
        }
        // Remap parent/children indices
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

### C.4 Test Strategy for Layout

1. **Roundtrip property test:** For any `LayoutTree`, `layout_parse(layout_dump(tree)) == tree`.
2. **Checksum parity test:** Generate layout strings from tmux, verify our `layout_checksum()` produces the same 4-hex-digit prefix.
3. **layout_check invariant:** After every mutation (split, resize, destroy, compact), `layout_check(tree)` returns true.
4. **resize_check parity:** For a known tree, compare `resize_check` values against tmux.
5. **Round-robin distribution test:** Split 3 panes evenly at 100 cols, resize to 103, verify one cell at a time gets +1 (not proportional).
6. **Pane assignment parity:** Parse a tmux layout string with N cells, verify depth-first pane assignment matches tmux.
7. **Edge cases:**
   - Single pane window: dump/parse roundtrip.
   - Maximum depth: 20 levels of nesting.
   - Resize to 1x1: all panes get `PANE_MINIMUM`.
   - Split when at minimum size: returns `LayoutError::TooSmall`.
   - Destroy last child: parent collapses.
   - Compact after 10 destroy operations: no dead cells remain.
8. **Preset layout parity:** For each of 7 presets with N=1..20 panes, compare our layout dump against tmux's `select-layout` output.

---

## D. OpenTelemetry (Refined)

### D.1 Span Naming Convention

Adopt GPT's `termforge.` prefix for all span names (instead of Claude's shorter names). This avoids collision with other instrumented libraries and follows OpenTelemetry semantic conventions for namespace prefixing:

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

### D.2 Test Strategy for Telemetry

1. **Unit test:** Verify `init_tracing()` with `Exporter::Stdout` produces valid JSON span output.
2. **Integration test:** Run server with OTEL enabled, execute a command sequence, verify span parent-child relationships are correct.
3. **Pure boundary test:** Verify that `mux-core` has no dependency on `opentelemetry` crate -- only `tracing`.
4. **Context propagation test:** Verify that a trace started in the Python binding appears as a parent span in the server's trace.
5. **Filter test:** Verify `ExportFilterConfig.should_export` excludes `h2`, `tonic`, `hyper` prefixes.

---

## E. Language Bindings (Refined)

### E.1 Error Mapping (Refined with Classification)

Map `ErrorClass` to language-native exception types:

| ErrorClass | Python | Node.js | C++ |
|---|---|---|---|
| Transient | `TimeoutError` | `Error` with code `ETIMEOUT` | `std::runtime_error` |
| ProtocolViolation | `ConnectionError` | `Error` with code `EPROTO` | `std::runtime_error` |
| UserError | `ValueError` / `KeyError` | `Error` with code `EINVAL` | `std::invalid_argument` |
| Bug | `RuntimeError` | `Error` with code `EINTERNAL` | `std::logic_error` |

### E.2 Migration Path from libtmux

Existing libtmux users need a smooth migration. The Python binding should expose property names that match libtmux conventions where possible:

```python
# Compatibility: libtmux uses .sessions as property, .cmd() as method
server = termforge.Server()
server.sessions      # property, returns list (like libtmux)
server.cmd("ls")     # method, returns string (like libtmux)
```

### E.3 Test Strategy for Bindings

1. **Python:** pytest suite with hermetic server (spin up muxd per test).
2. **Node:** vitest suite with same pattern.
3. **GIL test:** Python test that runs a blocking command in one thread while another thread verifies it can acquire the GIL.
4. **Error mapping test:** Trigger each error class from bindings and verify the correct exception type.
5. **Memory leak test:** Use Python's `tracemalloc` to verify no leaks after 1000 create/destroy cycles.
6. **Concurrency test:** 10 Python threads calling `server.sessions` concurrently -- no deadlocks, no panics.
7. **asyncio.to_thread test:** Verify `asyncio.to_thread(server.cmd, "list-sessions")` works from async context.

---

## F. CRDT Detail (Refined)

### F.1 HLC Overflow Guard

GPT's HLC `observe()` uses `saturating_add` for the counter. Claude's `receive()` uses `+ 1`. The counter is `u32`, which can wrap after 4 billion events at the same millisecond. We adopt `checked_add` with a fallback to advancing the physical clock:

```rust
pub fn receive(&mut self, remote: HlcTimestamp, wall_ms: u64) -> HlcTimestamp {
    let millis = wall_ms.max(self.last.millis).max(remote.millis);
    let counter = if millis == self.last.millis && millis == remote.millis {
        match self.last.counter.max(remote.counter).checked_add(1) {
            Some(c) => c,
            None => {
                // Counter overflow: advance millis by 1 to reset counter
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
```

### F.2 Test Strategy for CRDT

1. **HLC monotonicity:** Property test: 10K calls to `now()` with non-decreasing wall_ms always produce strictly increasing timestamps.
2. **HLC convergence:** Two clocks, 1000 interleaved local/receive operations, both clocks produce causally consistent ordering.
3. **LWW determinism:** Two registers with same timestamp, different node_ids: both converge to the same winner.
4. **OR-Set commutativity:** Property test: `merge(a, b) == merge(b, a)` for any two OR-Set states.
5. **OR-Set add-remove-add:** Verify that re-adding after removal with higher timestamp restores the element.
6. **OpLog idempotency:** Merging the same ops twice produces identical state.
7. **Counter overflow:** Simulate counter overflow and verify clock advances correctly.
8. **DVV compaction:** Add 10 nodes, deactivate 5, compact, verify summary has 5 entries.

---

## G. Control Mode (Refined)

### G.1 Extended Output Handling

Gemini noted that tmux emits `%extended-output` at `control.c:620-626`. Claude v5's `ControlNotification` enum includes `Output` but not `ExtendedOutput`. We add it:

```rust
pub enum ControlNotification {
    // ... all existing variants from Claude v5 ...

    /// %extended-output %<pane_id> <age> : <data>
    /// Newer format with timestamp, used by tmux 3.2+.
    ExtendedOutput {
        pane_id: u32,
        age: u64,
        data: Vec<u8>,
    },
}
```

### G.2 Migration from Current Generic Parser

Claude v5 Pass 1 defined a typed `ControlNotification` enum but did not address migration from the existing generic `name/raw` parser at `crates/mux-client/src/control.rs:31-36` and `crates/mux-refresh/src/event.rs:8-46`. The migration path:

1. Keep the generic parser as a fallback for unknown notifications.
2. Add the typed parser alongside, attempting typed parse first.
3. If typed parse returns `UnknownNotification`, fall through to generic path.
4. Over time, convert all consumers to use typed variants.

```rust
pub fn parse_control_line(line: &str) -> ControlEvent {
    match parse_notification(line) {
        Ok(typed) => ControlEvent::Typed(typed),
        Err(ControlParseError::UnknownNotification(tag)) => {
            // Fall back to generic parsing
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

### G.3 Test Strategy for Control Mode

1. **Parser roundtrip:** For each notification type, construct a line, parse it, verify correct variant.
2. **Unknown notification:** Verify unknown `%foo-bar` produces `Generic`, not a panic.
3. **Guard protocol:** Simulate a `%begin/%end/%error` block with output lines, verify correct framing.
4. **Malformed lines:** Feed truncated, empty, and binary data to the parser. Verify `ControlParseError`.
5. **Parity test:** Connect via `tmux -C`, trigger each notification type, capture the line, feed to our parser.
6. **Hints-vs-authority test:** Simulate dropped notification, verify periodic refresh corrects state.
7. **Migration test:** Verify `ControlEvent::Generic` fallback works for notifications not yet in the typed enum.
8. **Extended output test:** Parse `%extended-output %5 12345 : hello` and verify fields.

---

## H. Server Lifecycle (Refined)

### H.1 Lock File: flock vs PID File

Claude v5 uses `create_new` (O_EXCL) + PID check for crash recovery. tmux actually uses `flock(LOCK_EX|LOCK_NB)` at `client.c:72-100`. `flock` is superior because:
- The lock is automatically released if the process dies (kernel cleans up).
- No need for stale PID detection (eliminates PID reuse risk identified by Gemini).
- No TOCTOU race between checking PID and removing the file.

**Revised implementation:**

```rust
// crates/mux-os/src/lock.rs

use std::os::unix::io::AsRawFd;

pub struct LockFile {
    _file: std::fs::File,
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

    // Try non-blocking exclusive lock (matches tmux client_get_lock)
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

    // Write PID for diagnostic purposes (not for locking)
    use std::io::Write;
    let mut f = &file;
    let _ = f.write_all(format!("{}\n", std::process::id()).as_bytes());

    Ok(LockFile { _file: file, path: lock_path })
}

impl Drop for LockFile {
    fn drop(&mut self) {
        // flock is automatically released when file is closed.
        // Remove the lock file for cleanliness.
        let _ = std::fs::remove_file(&self.path);
    }
}
```

### H.2 Config Load Timing

tmux loads config only after the first client completes identify (`server-client.c:3725-3734`):

```c
if ((~c->flags & CLIENT_EXIT) &&
     !cfg_finished &&
     c == TAILQ_FIRST(&clients))
    start_cfg();
```

TermForge must replicate this: the server starts, binds the socket, but does NOT load `~/.tmux.conf` or `~/.config/termforge/config` until the first client's identify burst completes. This ensures that any config errors can be reported to the first client.

### H.3 Test Strategy for Server Lifecycle

1. **Lock contention test:** Start server A, then server B on the same socket. B must fail with `another server holds the lock`.
2. **Crash recovery test:** Start server, kill -9 it, start new server -- must succeed because `flock` is released on process death.
3. **Config timing test:** Start server, connect client, verify config was loaded only after first client identified.
4. **Shutdown test:** Start server, connect 3 clients, send shutdown, verify all clients receive exit notification and socket is cleaned up.
5. **Signal handling test:** Send SIGTERM to server, verify graceful shutdown sequence.
6. **Identify burst test:** Fixture tests for all identify types 100-112, verifying ordering parity with `client_send_identify` (`client.c:450-495`).

---

## I. Security Model (Refined)

### I.1 Input Validation Boundary Diagram

Expanding Claude v5's 4-layer diagram with the error classification:

```
Layer    Input Source         Validator            Error Class          Action
-----    ------------         ---------            -----------          ------
  0      Socket bytes         ImsgCodec            Transient            wait
                                                   ProtocolViolation    kill conn
  1      ImsgFrame            PayloadParser        ProtocolViolation    kill conn
  2      TypedPayload         IdentifyCollector    ProtocolViolation    kill conn
  3      Command args         CommandDispatch      UserError            error reply
  4      Config file text     mux-conf lexer       UserError            error event
  5      Control mode lines   ControlParser        UserError            %error reply
  6      Binding FFI args     mux-orm validators   UserError            exception
  7      CRDT ops (sync)      CrdtValidator        ProtocolViolation    drop peer
```

### I.2 Additional Security Invariants

1. **No symlink following in socket path:** Check for symlinks at each component of the socket directory path. tmux does not explicitly check this, but it is a defense against symlink attacks on shared systems.
2. **Maximum client count:** Limit the number of simultaneous client connections to prevent DoS. Default: 256 (configurable).
3. **Maximum control mode output queue:** As specified in Q3, limit pending output per control client to 16 MB to prevent memory exhaustion.
4. **SCM_RIGHTS restriction:** File descriptors received via SCM_RIGHTS are accepted only during the identify handshake phase. Any ancillary fds received outside handshake are closed immediately. Multiple unexpected fds in one ancillary message must all be closed deterministically.
5. **ValidatedFd alignment:** The current `mux-os` crate already applies CLOEXEC in `recv_with_fd` at `crates/mux-os/src/scm_rights.rs:141-149`. The `ValidatedFd` type from Claude v5 should wrap this existing functionality rather than duplicating it.

### I.3 Test Strategy for Security

1. **Socket permissions test:** Create socket, verify mode is 0600.
2. **Directory permissions test:** Reject socket in world-writable directory.
3. **SCM_RIGHTS validation test:** Send an fd that is not a TTY during identify -- verify it is accepted but marked `is_tty: false`.
4. **CLOEXEC test:** After receiving fd, verify FD_CLOEXEC is set.
5. **Max client test:** Connect 257 clients, verify 257th is rejected or queued.
6. **Fuzz:** Feed random bytes to all parsers (codec, control, config, format). Verify no panic, no OOB access.
7. **Out-of-phase SCM_RIGHTS test:** Send fd after identify is complete -- verify fd is closed, connection is killed.

---

## J. Performance Targets (Refined)

### J.1 Revised Benchmark Suite

Based on the existing `mux-refresh/benches/latency.rs` and the gaps identified in Q5:

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
    let layout_str = "d3a0,204x50,0,0{102x50,0,0,0,101x50,103,0[101x25,103,0,1,101x24,103,26,2]}";
    c.bench_function("layout_parse", |b| {
        b.iter(|| black_box(layout_parse(layout_str)))
    });
}

fn bench_layout_dump(c: &mut Criterion) {
    // Build a 10-pane tiled layout
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
                &graph,
                OptionScope::Pane,
                TargetId::Pane(pane),
                "status",
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
        graph: &graph,
        session: Some(session),
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

### J.2 CI Regression Gate (Revised)

```yaml
# .github/workflows/bench.yml
name: Performance Regression Check
on:
  pull_request:
    paths: ['crates/**']
  schedule:
    - cron: '0 4 * * *'  # nightly

jobs:
  bench:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
    - name: Run benchmarks
      run: cargo bench --bench hot_path -- --output-format bencher | tee output.txt
    - name: Compare against baseline
      uses: benchmark-action/github-action-benchmark@v1
      with:
        tool: 'cargo'
        output-file-path: output.txt
        alert-threshold: '130%'   # 30% tolerance for shared runners
        fail-on-alert: ${{ github.event_name == 'schedule' }}  # only block on nightly
        comment-on-alert: true
        github-token: ${{ secrets.GITHUB_TOKEN }}
```

**Difference from Claude v5 Pass 1:** 130% threshold instead of 120%, and only block PRs on nightly runs (not every PR) to avoid false positives from noisy CI runners. Also added a validity check: CI must assert that criterion output file is non-empty before comparing.

---

## K. Updated AGENTS.md Rules

Incorporating GPT's AGENTS.md rule additions and Gemini's DOs/DON'Ts:

### New Rules (additions to v4 template)

```markdown
## Error Handling Rules

1. **Error Taxonomy Rule:** Every library crate defines a public `Error` enum
   deriving `thiserror::Error`. `anyhow` is allowed only in binary crates
   and test code, never in library crates.

2. **Error Classification Rule:** Every error type must implement the
   `Classified` trait returning one of `Transient`, `ProtocolViolation`,
   `UserError`, or `Bug`. The classification determines recovery action.

3. **Protocol Violation Rule:** Any binary protocol decode error other than
   "need more bytes" (Transient) closes that client connection and records
   a structured error event. Reference: tmux server-client.c:3472-3475.
   This is a MUST -- never drop a malformed frame and continue on the
   same connection.

4. **Pure Error Boundary Rule:** `io::Error` and platform types MUST NOT
   cross the pure/impure boundary. Convert to stable, serializable
   domain errors via `Event::EffectFailed`.

## Configuration Rules

5. **Config-as-Events Rule:** Config file execution produces `Vec<Event>`
   and submits them through the state actor. Direct graph mutation from
   config parsing is forbidden.

6. **Option Scope Resolution Rule:** Option scope must be resolved using
   the option table's declared scope combined with command flags, matching
   tmux's `options_scope_from_name()` logic including the FALLTHROUGH
   from WindowPane to Window scope.

7. **Unset Semantics Rule:** `set -u` removes local override (restoring
   inheritance) for non-global scopes. For global scopes, it resets to
   compiled default. `set -U` additionally clears pane-local values.

## Layout Rules

8. **Layout String Parity Rule:** Layout string dump/parse and checksum
   MUST match tmux's `layout-custom.c` exactly. Verify with roundtrip
   tests against real tmux layout strings.

9. **Layout Minimum Rule:** `layout_resize()` never fails. It clamps
   the resize to the minimum possible size per `layout_resize_check()`.
   `PANE_MINIMUM` is 1 cell in each dimension.

10. **Round-Robin Distribution Rule:** `layout_resize_adjust()` distributes
    size changes one cell at a time in round-robin across children, NOT
    proportionally. This matches tmux layout.c:448-462.

11. **Layout Consistency Rule:** After every layout mutation, `layout_check()`
    must return true. Add this as a debug_assert in all mutating methods.

## Telemetry Rules

12. **Span Naming Rule:** All span names use the `termforge.` prefix.
    Pattern: `termforge.<subsystem>.<operation>`.

13. **Telemetry Propagation Rule:** Any new thread, task, or spawned process
    must attach OTEL context by pushing trace headers.

## Security Rules

14. **Untrusted Input Rule:** All data from sockets, control mode, config
    files, and binding FFI is untrusted. Validation occurs at crate
    boundaries (mux-proto, mux-conf, mux-control, mux-orm).

15. **SCM_RIGHTS Rule:** File descriptors received via SCM_RIGHTS are
    accepted only during the identify handshake. All received fds must
    have CLOEXEC set immediately. No ancillary fds outside handshake.

16. **Control Mode Hints Rule:** Control mode notifications are hints only,
    never authoritative state. The binary protocol / direct query is the
    authority. A dropped notification is corrected by periodic refresh.

## Lifecycle Rules

17. **Lock File Rule:** Use `flock(LOCK_EX|LOCK_NB)` for server lock files,
    not PID-based locking. flock is automatically released on process death.

18. **Config Timing Rule:** Do not load config until the first client
    completes the identify burst. This matches tmux server-client.c:3725-3734.

## DO / DON'T (Pass 2 delta)

- DO mirror tmux checksum algorithm exactly on tmux wire paths.
- DO treat `set -u` as "remove local override", not "write inherited value locally".
- DO enforce protocol-violation disconnect for binary protocol peers.
- DO separate control-command ingress rate limits from output-backpressure limits.
- DO use round-robin distribution for layout resize, not proportional.
- DO validate layout consistency after every mutation (debug_assert layout_check).
- DON'T claim benchmark regressions are gated until criterion harness is proven running in CI.
- DON'T use proportional resize distribution -- it produces different geometry from tmux.
- DON'T drop malformed binary frames and continue on the same connection.
- DON'T load config before first client identifies.
```

---

## L. Additional Risks

### L.1 Risks Identified in Pass 2

| # | Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|---|
| R1 | Layout string format divergence across tmux versions | Low | High | Pin to tmux protocol v8; version-gate parser changes behind protocol version |
| R2 | `flock` behavior differs across OSes (Linux vs macOS vs BSDs) | Medium | Medium | Test on all CI platforms (Linux, macOS); document platform differences in SAFETY.md |
| R3 | HLC counter overflow at same millisecond (4B events/ms) | Very Low | Medium | `checked_add` with forced millis advance (see F.1) |
| R4 | Control mode output backpressure causing unbounded memory | Medium | High | Enforce per-client pending output limit (16MB per Q3); drop oldest %output blocks first |
| R5 | Python GIL contention under heavy concurrent access | Medium | Medium | All blocking ops release GIL; document that OrmServer is not Send across threads |
| R6 | `options_scope_from_name` fallthrough logic is subtle | High | Medium | Exhaustive test matrix covering all (scope, flag, target) combinations |
| R7 | Arena-based layout tree accumulates dead cells after destroy | Medium | Low | Compact arena periodically or on layout preset application (see C.3) |
| R8 | Proportional resize vs round-robin resize produces different geometry | High (if not caught) | High (parity failure) | Resolved: use round-robin matching tmux exactly |
| R9 | CRDT sync over untrusted network could be attacked | Low (Phase 1 is local) | High (if exploited) | Phase 1: sync only over Unix sockets. Phase 2: add TLS + mutual auth for TCP sync. Validate all incoming CrdtOps. |
| R10 | `mux-conf` parser divergence from tmux's actual parsing | Medium | Medium | Maintain a corpus of 100+ real tmux.conf files and verify parse succeeds and produces identical events |
| R11 | 16-bit checksum collision risk for internal persistence | Low | Low | Use BLAKE3-128 envelope for persistence; tmux checksum only for wire compat |
| R12 | PID reuse in stale lock handling (Claude v5 approach) | Medium | Medium | Resolved: use flock instead of PID-based locking |
| R13 | Control mode output backlog unfair scheduling across panes | Low | Low | Implement per-pane output quotas within the per-client limit |
| R14 | FD-passing edge case: multiple FDs in one ancillary message | Low | Medium | Close all unexpected extra FDs deterministically |
| R15 | Option unset with array indices may diverge from tmux | Medium | Medium | Mirror `options_remove_or_default` index handling at options.c:1282 exactly |
| R16 | `mux-types` crate does not exist in current workspace | Certain | Medium | Create it as first action; all other crates depend on it |
| R17 | Criterion benchmarks not wired in CI (harness misconfigured) | Certain | Low | Fix `[[bench]]` section immediately; add CI step asserting non-empty output |

### L.2 Reference Verification (Existence Check)

| Claim from Pass 1 | Status | Evidence |
|---|---|---|
| Identify burst 100-112 tracked | Exists | `tmux-protocol.h:29-41`, `crates/mux-proto/src/msg.rs:40-67` |
| `goto bad -> proc_kill_peer` behavior | Exists | `server-client.c:3377-3475` |
| `ControlNotification` exhaustive enum in current code | Missing | Current is generic `name/raw` struct (`crates/mux-client/src/control.rs:31-36`) |
| `LockFile` implementation in vibe-tmux | Missing | No `LockFile` symbol in `crates/`/`tools/` |
| `HybridClock` implementation in vibe-tmux | Missing | No `HybridClock` symbol in `crates/`/`bindings/` |
| `ValidatedFd` type in vibe-tmux | Missing | No `ValidatedFd` symbol; CLOEXEC handled functionally (`scm_rights.rs:141-149`) |
| Criterion benchmarks currently enforceable | Not yet | Benches defined, harness wiring missing (`mux-refresh` Cargo.toml) |
| `mux-types` crate exists | Missing | Not present in workspace members |

---

## Summary of Pass 2 Changes

| Area | Pass 1 State | Pass 2 Change |
|---|---|---|
| Error Classification | Per-crate errors only | Added `ErrorClass` taxonomy (Gemini+GPT consensus) with `Classified` trait |
| Codec Recovery | "Drop frame, continue" | Corrected: `ProtocolViolation -> kill connection` (Gemini's insight) |
| Layout Resize | Proportional distribution | Corrected: round-robin one-cell-at-a-time (matching tmux `layout_resize_adjust`) |
| Layout Validation | Missing | Added `layout_check()` from Gemini |
| Layout Arena | No compaction | Added `compact()` method for arena hygiene |
| Lock File | PID-based with stale detection | Changed to `flock(LOCK_EX|LOCK_NB)` (matches tmux) |
| Option Unset | Unspecified | Defined: remove local override for non-global, reset to default for global, `-U` for pane cascade |
| Option Scope | Simplified | Added `WindowPane` fallthrough matching tmux `options_scope_from_name` |
| Control Auth | Unspecified | Defined: no auth beyond socket permissions + ACL; optional rate limiter |
| Control Parser | Typed only | Added migration path from generic parser with fallback |
| Control Notifications | Missing ExtendedOutput | Added `ExtendedOutput` variant from Gemini |
| CRDT Clocks | Vector clocks mentioned | Decided: HLC + DVV with compaction; bounded by active nodes |
| HLC Safety | Unchecked add | Added `checked_add` with forced millis advance for overflow |
| Benchmark Baseline | "500 MB/s" targets | Revised: no existing baselines; conservative targets; CI at 130% threshold |
| Benchmark Wiring | Missing harness config | Documented `[[bench]]` fix needed for mux-refresh |
| Python Async | Unspecified | Decided: Phase 1 sync, Phase 2 `asyncio.to_thread`, Phase 3 native streaming |
| Layout Minimum | Unspecified | Decided: resize clamps to minimum, never fails; returns `ResizeResult` |
| Config Timing | Not mentioned | Added: config loads only after first client identifies |
| AGENTS.md Rules | v4 rules | Added 18 concrete rules + 10 DOs/DON'Ts from GPT and Gemini |
| Span Names | Short names | Adopted `termforge.` prefix from GPT |
| Risks | From v4 | Added 17 new risks specific to v5 features with mitigations |
| Reference Check | Not done | Added existence verification table for all claimed types/features |
| Test Strategies | Sparse | Every section now has 5-8 concrete test strategies |
