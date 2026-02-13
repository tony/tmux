# TermForge v0002 Architecture (Pass 1)

This document records decided architecture for Pass 1. No TBD markers are left for the critical gaps from synthesis.

## 1) Composition and Diff Rendering Strategy (DECIDED)

### Decision

TermForge uses an **own dirty-line compositor + diff emitter** in `mux-render`.

- Input: pane surfaces (`PaneSurface`) with geometry, z-index, dirty row mask.
- Stage A: compose all panes into a frame buffer (`Vec<Line>`).
- Stage B: dirty-line guided diff against previous frame (`line_diff_spans`).
- Stage C: span coalescing (`coalesce_spans`) and ANSI emission.
- Stage D: enforce output byte budget and fairness quota.

We intentionally do not depend on `vt100::contents_diff()` or `termwiz::Change` for this hot path in Pass 1.

### Why this approach

- Multiplexer composition is pane-aware and geometry-aware; a black-box terminal diff API does not encode starvation/fairness policy.
- We need explicit per-pane row quota to prevent flood starvation.
- We need deterministic budget enforcement for control-mode and SDK state sync.

### Frame Rate Policy

- Target: `60 FPS` default (`FramePolicy::target_fps`).
- Frame interval gate: skip render if called before next deadline unless forced.
- Budget: `64 KiB` max output per frame (`max_batch_bytes`).

### Flood Handling and Coalescing

- Per pane, at most `pane_row_quota` dirty rows are emitted in one frame.
- Remaining rows are deferred and rotated using stride offset per pane.
- Adjacent diff spans separated by <=2 columns are coalesced.

### Code-backed API

```rust
pub struct PaneSurface {
    pub pane_id: u64,
    pub z_index: u16,
    pub rect: Rect,
    pub lines: Vec<Line>,
    pub dirty_rows: Vec<bool>,
    pub epoch: u64,
}

pub struct Compositor {
    previous: Vec<Line>,
    policy: FramePolicy,
    next_deadline_ms: u64,
}

pub fn compose_frame(&mut self, now_ms: u64, panes: &[PaneSurface], force: bool)
    -> Option<FrameOutput>;
```

### Composition Pipeline Pseudocode

```rust
fn compose_frame(now, panes):
  if now < deadline and !force: return None
  framebuffer = blank(rows, cols)
  for pane in sort_by_zindex(panes):
    mask = quota_mask(pane.dirty_rows)
    for each dirty row allowed by mask:
      blit pane row into framebuffer at pane.rect
  ops = []
  for row in 0..rows:
    spans = coalesce(diff(previous[row], framebuffer[row]))
    for span in spans:
      ops.push(encode_ansi_span(row, span, framebuffer[row]))
      if bytes(ops) >= max_batch_bytes: break
  previous = framebuffer
  return ops
```

## 2) Layout Engine (DECIDED)

### Decision

`mux-kernel` owns layout as a deterministic geometry tree.

- Data structure: `LayoutNode` tree (`Leaf`, `Split`).
- Solver: recursive `solve_layout(node, rect) -> HashMap<Pane, Rect>`.
- Built-ins: `EvenHorizontal`, `EvenVertical`, `MainHorizontal`, `MainVertical`, `Tiled`.
- Custom layout strings: checksum-validated `"hh,payload"` format.

### Data Structures

```rust
pub enum LayoutNode {
    Leaf { pane: SlotKey, min_size: u16 },
    Split {
        axis: Axis,
        children: Vec<LayoutNode>,
        weights: Vec<u16>,
        main_index: Option<usize>,
    },
}

pub struct WindowLayout {
    pub root: LayoutNode,
    pub encoded: String,
}
```

### Resize Redistribution Algorithm

- Compute mandatory minima per child.
- If sum(minima) < total, distribute remaining cells proportionally to weights.
- Distribute remainder cells in stable round-robin order.
- Always preserve exact parent size (no gap, no overlap).

## 3) Raw Mode Lifecycle (DECIDED)

### Decision

`mux-api` owns raw mode policy as explicit contract:

- `ManageRaw`: SDK owns enter/restore.
- `AssumeExternalRaw`: caller owns lifecycle.
- `ProbeAndManage`: SDK probes and only toggles when needed.

### State Machine

```rust
TerminalModeState::Canonical -> RawOwned | RawExternal -> Canonical
```

This avoids double-raw toggles for embedded usage and prevents clobbering caller-owned termios.

## 4) SIGWINCH Propagation Flow (DECIDED)

### Sequence (Invariant)

```text
Outer terminal resize
 -> OS delivers SIGWINCH to server process
 -> signal layer enqueues KernelEvent::SigWinch
 -> kernel emits KernelEffect::QueryTerminalSize
 -> runtime queries actual size (TIOCGWINSZ)
 -> runtime sends KernelEvent::Resize{rows, cols}
 -> kernel recomputes window layouts (solve_layout)
 -> kernel emits ResizePty per pane geometry (TIOCSWINSZ request)
 -> runtime performs ioctl(TIOCSWINSZ) on each pane PTY
 -> kernel emits RenderNow
 -> render thread composes/diffs and flushes frame
```

This flow is treated as protocol-level behavior, not an optional implementation detail.

## 5) Zombie Reaping and SIGCHLD (DECIDED)

### Decision

- Signal layer converts SIGCHLD to `KernelEvent::SigChild`.
- Runtime owns reaping syscall loop:
  - `while waitpid(-1, WNOHANG) > 0 { ... }`
- Kernel receives `PtyExited` events and updates graph/copy-mode state.

### Non-interference invariant

- Child panes are isolated process groups/sessions.
- Reaper only consumes direct child state transitions from pane supervision.
- Grandchildren remain governed by pane shell/session leader semantics unless explicitly adopted.

## 6) crossterm vs nix/rustix (DECIDED)

### Decision

Pass 1 selects **direct unix syscalls via `nix`/`rustix` style architecture** (implemented incrementally, currently modeled in stubs), not `crossterm`.

### Rationale

- Multiplexers require PTY/session/ioctl/fd-passing control that sits below high-level event abstractions.
- We need precise control for tmux-compat behavior around signals, process groups, and wire control mode.
- `crossterm` is excellent for apps; TermForge is an infrastructure/runtime layer.

## 7) vt100/termwiz Evaluation (DECIDED)

### Decision

Pass 1 keeps an **in-house parser+render pipeline** and does not adopt `vt100`/`termwiz` as the primary engine.

### Rationale

- `vt100::contents_diff()` is strong for single-surface diffing, but TermForge requires multi-pane composition fairness and protocol-aware batching.
- `termwiz` `Surface/Change` is compelling for rich terminal abstraction, but adds an additional model layer we would still have to map into tmux-compatible pane semantics.
- We still keep the door open for benchmark-backed interop adapters in later passes.

## 8) Copy Mode (DECIDED)

### Decision

`mux-kernel` contains explicit copy-mode model:

- Key tables: `Vi`, `Emacs`.
- Selection modes: `Character`, `Line`, `Block`.
- Search model: direction + case mode + needle.
- Copy buffer ring: bounded queue (64 entries).

### Code-backed State

```rust
pub struct CopyModeState {
    pub enabled: bool,
    pub key_table: KeyTable,
    pub cursor: CopyCursor,
    pub scroll_offset: usize,
    pub selection: Option<CopySelection>,
    pub search: Option<SearchQuery>,
    pub buffer_ring: VecDeque<String>,
}
```

### Behavioral Notes

- Movement clamps to bounds.
- Yank operation writes selection into ring buffer.
- Search updates cursor deterministically.

## 9) Graphics Protocol Stance (DECIDED)

### Decision

Pass 1 stance is **passthrough-only** for graphics escape payloads.

- Sixel: passthrough supported.
- iTerm2 inline image escapes: passthrough supported.
- Kitty graphics protocol: passthrough supported.
- No decode, raster cache, or image composition in renderer yet.

### Rationale

- tmux compatibility requires payload transparency first.
- Full graphics virtualization is high complexity and not needed for protocol parity baseline.
- Keeps render path focused on text-cell latency.

## 10) signal-hook vs tokio signals (DECIDED)

### Decision

Hybrid split by responsibility:

- **`signal-hook` style sync handlers** for robust low-level POSIX signal capture/masking behavior.
- **tokio channels/tasks** for async propagation to I/O and render loops.

### Rationale

- Low-level signal semantics (masking/restart behavior) belong to dedicated signal tooling.
- Runtime scheduling, backpressure, and cross-thread fanout fit tokio/crossbeam model.

## Additional Adopted Structures

### `mux-api` crate

Added as SDK front-door with builder and embedding lifecycle controls.

### Bounded lane defaults

`LaneCapacity` default:

- control: `512`
- data: `1024` frames x `64 KiB`
- render: `256`

### crossbeam-channel bridge

Used for deterministic bounded queues between API/runtime boundaries.

### Config and hot reload

`TermForgeConfig` includes hot-reload path and enable switch.
Format stance for runtime config: TOML source, serde-backed typed config in memory.

### Panic recovery architecture

Kernel supports explicit `Crash` event -> ordered effects:

1. `CrashReportFrame` to clients
2. `Shutdown`

This preserves post-mortem context before process exit.

### CRDT section

CRDT clocks remain in `mux-time` (vector clock under feature gate). Collaborative conflict resolution strategy:

- causal ordering by vector clock dominance,
- tie-break by `(node_id, lamport, local_counter)` stable tuple,
- merge by last-writer-wins within causality class.

### SCM_RIGHTS FD passing

Architecture includes unix-domain fd handoff path for tmux-compatible PTY transfer flows.
Runtime layer owns `SCM_RIGHTS` framing and capability validation.

### Async boundary for language bindings

`mux-api::BindingBridge` defines boundary styles:

- blocking,
- callback,
- async-poll tokenization.

This keeps Python/Node wrappers from leaking kernel internals.

## Crate Responsibilities (Pass 1)

- `mux-kernel`: layout tree, copy mode, signal event/effect contracts.
- `mux-render`: composition, diff, batching, flood fairness.
- `mux-api`: embedding policy, terminal mode lifecycle, config hydration.
- Existing crates from prior scaffold remain at or above previous completeness.

## Validation Status

- Workspace includes the new crates and dependencies.
- Critical gaps from synthesis are all mapped to explicit decisions and code types.
- Unit tests include render/layout/copy-mode and lifecycle policy coverage in addition to pre-existing suite.
