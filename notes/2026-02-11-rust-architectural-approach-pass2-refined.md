# TermForge (Pass 2): Refined Rust Terminal Multiplexer Architecture

Date: 2026-02-11
Scope: This is a north-star architecture for a new Rust implementation that targets tmux compatibility.

## Pass 2 Delta Summary (What Was Fixed vs Pass 1)

### What Pass 1 Got Right

The Pass 1 synthesis converged on the correct macro-structure:

1. Functional core with `Event -> (State, Effects)` and an imperative runtime that interprets effects.
2. Generational IDs for entities and an explicit entity graph (SlotMap style).
3. Snapshots as the read path and a single-writer actor for state mutation.
4. tmux treated as a compatibility adapter (protocol + semantics), not the internal model.
5. Deterministic testing via fake PTY and hermetic runtime isolation.

### Weaknesses, Missing Steps, and Incorrect Assumptions in Pass 1

This repository is the upstream tmux C source tree, not a Rust workspace.

1. Referenced Rust crates and paths like `crates/mux-core/src/engine.rs` do not exist here today.
2. Several claims used "vibe-tmux" paths as if they were in this repo; they are not.
3. The VT100/ANSI/grid section was under-specified relative to its complexity.
4. The format string engine (`format.c`) was identified as large but the design was not pinned down enough to implement.
5. Key tables and copy mode were mentioned but not turned into concrete core state and reducer rules.
6. CRDT was described as universal without scoping which parts are actually safe/feasible to replicate first.
7. "WASM proof" was asserted without listing the specific crate boundaries and dependency constraints needed to make it practical.
8. Test strategy lacked explicit parity methodology against tmux (golden fixtures, reference runner, and failure triage).
9. SecondaryMap vs Vec adjacency was treated as a stylistic choice; it impacts borrowing, diffs, CRDT ops, and snapshotting.
10. "ORM" placement was debated but not settled into a stable layering contract that avoids a second business-logic engine.

### Grounding: What We Verified Exists in This Repo (tmux C Reference Points)

These are real files and symbols present in `/home/d/study/c/tmux` and should be treated as authoritative behavior references:

1. `format.c` contains `struct format_expand_state` and helpers like `format_expand1` and `format_replace`.
2. `window-copy.c` contains the copy-mode implementation entry points such as `window_copy_init`, `window_copy_command`, selection and search helpers.
3. `key-bindings.c`, `input-keys.c` and `input.c` exist and define tmux key handling and input parsing.
4. `grid.c`, `grid-view.c`, `grid-reader.c`, and `screen.c` exist and define tmux's grid/screen model.

Everything below that references Rust modules is a proposed workspace layout for a new Rust codebase, not something that exists in this repo yet.

---

## Table of Contents (23 Sections)

1. Project Name and Identity
2. Vision and Philosophy
3. North Star Acceptance Criteria
4. High-Level Architecture
5. Workspace Layout
6. Layering Contract
7. Entity Model
8. Event/Effect Engine
9. Protocol Codec
10. ORM-like Query API
11. Runtime Architecture
12. Language Bindings
13. CRDT Transaction Layer
14. tmux Version Management
15. Test Support Crate
16. Fake PTY Backend
17. Binding Test Frameworks
18. Test Framework and Harness Design
19. Visual Client (TUI)
20. DOs and DON'Ts
21. AGENTS.md Template
22. Phased Implementation Plan
23. Risks and Mitigations

---

## 1. Project Name and Identity

Name: `termforge`

Crate prefix: `mux-*` (kept for continuity with Pass 1 and grep friendliness)

Binaries:

1. `mux-server` (daemon)
2. `mux` (CLI client)
3. `mux-tui` (visual client)

License: MIT OR Apache-2.0 (dual)

Edition: Rust 2024, MSRV 1.85 (as in Pass 1)

---

## 2. Vision and Philosophy

TermForge is a multiplexer kernel and ecosystem that can:

1. Run as a server that speaks tmux protocols well enough to be a drop-in replacement.
2. Embed in-process as a library to manage sessions programmatically.
3. Expose a stable, typed API to Rust and thin bindings to Python/Node/C++.
4. Offer deterministic, replayable state transitions for correctness and long-term maintainability.

Non-goals:

1. A line-by-line tmux C port.
2. Reproducing tmux internals (rbtrees, TAILQ, libevent) in Rust.
3. Solving distributed CRDT for all state on day 1.

Core mantra: "Pure deterministic kernel; imperative runtime; adapters at the edge."

---

## 3. North Star Acceptance Criteria

### Compatibility

1. A stock `tmux` client can attach to `mux-server` and perform basic session/window/pane operations.
2. `mux` can attach to a stock tmux server and query/control it for a defined subset (useful as a parity tool).
3. `.tmux.conf` parsing supports the subset needed for common workflows and can be extended without re-architecting.

### Architectural Integrity

1. `mux-core` is deterministic and IO-free: no sockets, no files, no env, no clocks unless injected, no `unsafe`, no async runtime.
2. All mutation goes through the reducer: no hidden state mutation in runtime, protocol handlers, or bindings.
3. Reads are via immutable snapshots, not live locks.
4. Unsafe is quarantined into a small set of crates with explicit `// SAFETY:` comments and tests.

### Testing

1. The suite is hermetic: parallel runs do not collide on sockets, temp dirs, or environment.
2. Grid and ANSI parsing are fuzzed and run against vttest-like suites.
3. Format engine is parity-tested against real tmux `format.c` behavior for a large corpus.

---

## 4. High-Level Architecture

### Components

1. Pure Kernel: entity graph + reducer + transactions.
2. Pure Terminal Model: grid, styles, and a VT/ANSI performer that mutates the grid.
3. Pure Compatibility: tmux config semantics, key tables, and format expansion, all as pure logic.
4. Pure Protocol: codecs for tmux's server-client framing and control-mode parsing.
5. Runtime: sockets, PTY/process management, effect execution, snapshot publishing.
6. Facade: managed server handle, object-model API, and language binding entry points.

### Event/Effect Boundary

The core never "does" IO. It emits effects and the runtime interprets them.

Example effect classes:

1. Spawn process/PTY.
2. Write bytes to a PTY.
3. Send protocol frames to a client.
4. Start or cancel a timer.
5. Read filesystem config (as a request effect; runtime returns data as an event).

---

## 5. Workspace Layout

This is the proposed Rust workspace, not present in this tmux repo today.

```text
termforge/
  Cargo.toml
  AGENTS.md
  ARCHITECTURE.md
  crates/
    mux-types/            # PURE: IDs + small value types shared across pure crates
    mux-core/             # PURE: ServerGraph + Events + Effects + reducer + transactions
    mux-grid/             # PURE: Grid model + style + VT/ANSI performer (uses vte parser)
    mux-format/           # PURE: format AST/bytecode + evaluator traits (tmux semantics adapter lives here)
    mux-keys/             # PURE: key chords, key tables, key resolution state machine
    mux-conf/             # PURE: tmux-ish config grammar and lowering to core/key/format structures
    mux-proto/            # PURE: tmux-ish protocol framing + codec + control-mode parser
    mux-query/            # PURE: query DSL + traversal helpers over snapshots
    mux-view/             # PURE: snapshot types + derivations for UI and bindings
    mux-crdt/             # PURE: op log schema + merge, scoped to replicable state
    mux-os/               # IMPURE: platform adapters, unsafe quarantine (fd, pty, signals, scm_rights)
    mux-pty/              # IMPURE: real PTY + process lifecycle
    mux-pty-fake/         # PURE: fake PTY implementation + scenario playback
    mux-pty-recorder/     # IMPURE tool/lib: record real PTY sessions into scenarios
    mux-runtime/          # IMPURE: tokio/threads actors; effect executor; snapshot publisher
    mux-api/              # IMPURE facade: connect/host, submit commands/events, subscribe to snapshots
    mux-orm/              # IMPURE convenience object API on top of mux-api + mux-query
  tools/
    tmux-vm/
    tmux-builder/
    mux-regress/          # parity runner that uses tmux binaries + fixtures
  bindings/
    python/
    node/
    cpp/
  fuzz/
```

Decision (Open Question 3, 4): We split `mux-types` out, and we keep ORM as a separate crate (`mux-orm`) so `mux-api` stays a narrow "connection + submit + subscribe" facade.

---

## 6. Layering Contract

### Pure crates

`mux-types`, `mux-core`, `mux-grid`, `mux-format`, `mux-keys`, `mux-conf`, `mux-proto`, `mux-query`, `mux-view`, `mux-crdt`, `mux-pty-fake`

Rules:

1. No filesystem, sockets, PTY, environment reads, randomness, or time access unless injected explicitly.
2. No async runtime dependencies.
3. No `unsafe`.

### Impure crates

`mux-os`, `mux-pty`, `mux-pty-recorder`, `mux-runtime`, `mux-api`, `mux-orm`, binaries, tools, bindings.

Rules:

1. `unsafe` is limited to `mux-os` and small FFI boundaries.
2. Protocol and config adapters may be impure only if they call into OS; the semantics remain pure.

### WASM Purity Proof (Open Question 2)

We enforce `wasm32-unknown-unknown` compilation for specific pure crates in CI:

1. `mux-types`
2. `mux-core`
3. `mux-grid`
4. `mux-format`
5. `mux-keys`

This is a "purity tripwire", not a product requirement. It catches accidental OS dependencies early without forcing the entire system into `no_std`.

---

## 7. Entity Model

### Storage

We use generational IDs and SlotMap-style storage in `mux-core`.

```rust
// crates/mux-types/src/ids.rs
use slotmap::new_key_type;

new_key_type! { pub struct SessionId; }
new_key_type! { pub struct WindowId; }
new_key_type! { pub struct PaneId; }
new_key_type! { pub struct ClientId; }
new_key_type! { pub struct BufferId; }
new_key_type! { pub struct KeyTableId; }
```

### Decision: Adjacency Storage (Open Question 1)

We adopt a hybrid that makes diffs, CRDT ops, and borrowing predictable:

1. Entities store intrinsic fields only (name, options, mode state).
2. Graph relationships are stored in dedicated `Edges` maps using `SecondaryMap` keyed by the parent ID.
3. The adjacency list value is `SmallVec` for common small fanout, upgrading to heap Vec as needed.

```rust
// crates/mux-core/src/graph.rs
use slotmap::{SlotMap, SecondaryMap};
use smallvec::SmallVec;
use mux_types::{SessionId, WindowId, PaneId, ClientId, BufferId, KeyTableId};

pub type ChildList<T> = SmallVec<[T; 4]>;

pub struct ServerGraph {
    pub sessions: SlotMap<SessionId, Session>,
    pub windows: SlotMap<WindowId, Window>,
    pub panes: SlotMap<PaneId, Pane>,
    pub clients: SlotMap<ClientId, Client>,
    pub buffers: SlotMap<BufferId, PasteBuffer>,
    pub key_tables: SlotMap<KeyTableId, KeyTable>,

    pub session_windows: SecondaryMap<SessionId, ChildList<WindowId>>,
    pub window_panes: SecondaryMap<WindowId, ChildList<PaneId>>,
}
```

Why this beats "Vec inside entity" for the core:

1. It avoids relationship duplication inside entities.
2. It makes relationship diffs and CRDT ops explicit and uniform.
3. It reduces borrow complexity in the reducer because relationships live in distinct maps.
4. Snapshots can serialize `Edges` independently of entity structs for stable output.

### Decision: Persistent Data Structures (Open Question 6)

Decision: we do not use `im` persistent collections in the kernel initially.

1. The core graph is mutated in place by the single-writer actor, so persistent structures primarily help readers, not writers.
2. Readers already consume immutable `Arc<Snapshot>` values published via `ArcSwap`.
3. Persistent structures increase API complexity and tend to leak into public types if used pervasively.

If performance measurements later show snapshot building dominates CPU, we will introduce persistence only in derived view layers:

1. Use `im` or `rpds` inside `mux-view` for derived caches, not in `mux-core` entity storage.
2. Keep `mux-core` entity structs and adjacency maps on standard `SlotMap` + `SecondaryMap` + `SmallVec`.

### Pane Modes and Copy Mode State

```rust
// crates/mux-core/src/pane.rs
pub enum PaneMode {
    Normal,
    Copy(CopyState),
    View,
}

pub struct CopyState {
    pub cursor_row: u32,
    pub cursor_col: u32,
    pub scroll_offset: u32,
    pub selection: Option<Selection>,
    pub search: Option<SearchState>,
    pub key_mode: CopyKeyMode,
}
```

This maps directly to tmux behavior in `window-copy.c`, but in a pure, serializable form.

---

## 8. Event/Effect Engine

### Reducer Signature

We commit to a reducer that is deterministic and test-friendly.

```rust
// crates/mux-core/src/engine.rs
use mux_types::*;

#[derive(Clone, Debug)]
pub struct ApplyOutcome {
    pub effects: Vec<Effect>,
    pub refresh: RefreshHint,
    pub created: CreatedIds,
    pub crdt_ops: Vec<mux_crdt::Op>,
}

pub fn apply_event(g: &mut ServerGraph, e: Event, ctx: &CoreCtx) -> Result<ApplyOutcome, CoreError> {
    // No IO. All nondeterminism flows through ctx.
    todo!()
}
```

### Determinism Controls

`CoreCtx` is explicit injection for "needs time" or "needs randomness" cases.

```rust
// crates/mux-core/src/ctx.rs
#[derive(Clone, Copy, Debug)]
pub struct CoreCtx {
    pub now_ms: u64,
    pub rand_u64: u64,
}
```

The runtime is responsible for supplying `CoreCtx` and for turning timeouts into events.

### Key Processing as Pure State Machine

Key events do not directly trigger "commands". They go through key table resolution to yield semantic core events.

```rust
// crates/mux-core/src/event.rs
pub enum Event {
    ClientInputKey { client: ClientId, key: mux_keys::KeyChord },
    PtyOutput { pane: PaneId, bytes: Vec<u8> },
    TimerFired { token: TimerToken },
    // Domain intents:
    NewSession { name: String },
    SplitPane { window: WindowId, target: PaneId, dir: SplitDir },
    EnterCopyMode { pane: PaneId, mode: CopyKeyMode },
    CopyCommit { pane: PaneId, buffer: Option<String> },
}
```

Effects include:

```rust
// crates/mux-core/src/effect.rs
pub enum Effect {
    SpawnPanePty { pane: PaneId, cmd: Vec<String>, env: Vec<(String, String)> },
    PtyWrite { pane: PaneId, bytes: Vec<u8> },
    ClientSend { client: ClientId, frame: mux_proto::Frame },
    StartTimer { token: TimerToken, after_ms: u64 },
    CancelTimer { token: TimerToken },
    PublishSnapshot,
}
```

### Key Binding System (Open Question 9)

Decision: key handling is split into two layers:

1. `mux-keys` defines `KeyChord`, `KeyTable`, and a pure resolver that maps `(KeyState, KeyChord) -> KeyOutcome`.
2. `mux-core` owns per-client `KeyState` (active tables, prefix latch, mode-specific overrides) and converts `KeyOutcome` into domain `Event`s.

This matches tmux's conceptual model in `key-bindings.c` and mode-specific tables in `input-keys.c`, but keeps tmux naming out of core domain events.

```rust
// crates/mux-keys/src/lib.rs
use mux_types::KeyTableId;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct KeyChord {
    pub key: Key,
    pub mods: Mods,
}

#[derive(Clone, Debug)]
pub struct KeyTable {
    pub name: String,
    pub bindings: std::collections::HashMap<KeyChord, KeyAction>,
}

#[derive(Clone, Debug)]
pub enum KeyAction {
    EmitCoreEvent(mux_core::Event),
    EnterTable(KeyTableId),
    PopTable,
    Noop,
}

#[derive(Clone, Debug)]
pub struct KeyState {
    pub active_table_stack: Vec<KeyTableId>,
    pub prefix_armed: bool,
    pub prefix_timeout_token: Option<mux_core::TimerToken>,
}

pub fn resolve(tables: &std::collections::HashMap<KeyTableId, KeyTable>, st: &KeyState, k: KeyChord) -> KeyAction {
    // Pure table lookup + stack semantics.
    todo!()
}
```

Prefix behavior:

1. `ClientInputKey` for prefix key sets `prefix_armed = true` and emits `Effect::StartTimer` for prefix timeout.
2. Next key is resolved against the prefix table; if matched, emit the mapped `Event`.
3. On `TimerFired` for the prefix token, clear `prefix_armed`.

Tests:

1. Deterministic unit tests in `mux-keys` for table resolution and prefix stack semantics.
2. Integration tests in `mux-core` ensuring `StartTimer` and `TimerFired` drive prefix expiry deterministically.

### Copy Mode Architecture (Open Question 10)

Decision: copy mode is a pure per-pane state machine owned by `mux-core`, backed by `mux-grid` for read-only access to the pane's grid snapshot.

Key points:

1. Copy mode does not mutate the underlying PTY or process; it mutates `CopyState` only.
2. Selection extraction produces bytes deterministically from the current `Grid` and `CopyState`.
3. Copy-mode search is pure and uses deterministic regex engines (recommend `regex-automata`) with bounded backtracking behavior.

Core reducer responsibilities:

1. Entering copy mode sets `pane.mode = PaneMode::Copy(CopyState { ... })`.
2. Cursor movement events update `CopyState.cursor_*`.
3. Selection events update `CopyState.selection`.
4. Copy commit emits an effect to update a paste buffer entity and optionally to forward to OS clipboard (runtime effect).

```rust
// crates/mux-core/src/copy.rs
pub enum CopyEvent {
    MoveUp,
    MoveDown,
    BeginSelection,
    ClearSelection,
    SearchForward { needle: String },
    SearchBackward { needle: String },
    CopySelection { buffer_name: Option<String> },
}

pub fn apply_copy_event(pane: &mut Pane, grid: &mux_grid::Grid, ev: CopyEvent) -> Result<Vec<Effect>, CoreError> {
    // Pure, deterministic copy-mode updates and effects.
    todo!()
}
```

Mapping keys to copy events:

1. `mux-conf` loads tmux `bind-key -T copy-mode-vi ...` into `mux-keys` tables.
2. Key bindings in copy mode resolve to `Event::CopyMode { pane, ev: CopyEvent }` variants.

Tests:

1. Snapshot tests for selection extraction over a fixed grid.
2. Property tests: selection bounds never exceed grid dimensions; search is idempotent on repeated "find next" at end-of-buffer.
3. Parity tests: compare `capture-pane` outputs and selection behavior against tmux `window-copy.c` for a corpus of scripts.

---

## 9. Protocol Codec

Decision: keep `mux-proto` independent of `mux-core`.

1. `mux-proto` defines framing types and encode/decode.
2. Runtime translates between protocol messages and core events.
3. Control mode parsing is treated as a hint stream, not authoritative state.

```rust
// crates/mux-proto/src/lib.rs
pub struct Frame {
    pub ty: FrameType,
    pub payload: Vec<u8>,
}

pub trait Codec {
    fn decode(&mut self, bytes: &[u8]) -> Result<Vec<Frame>, ProtoError>;
    fn encode(&self, f: &Frame, out: &mut Vec<u8>);
}
```

Fuzz targets:

1. Frame decoder (invalid lengths, truncation, oversized payloads).
2. Control-mode line parser.

---

## 10. ORM-like Query API

Decision (Open Question 4): `mux-query` is pure and works on snapshots. `mux-orm` is a convenience wrapper on top of `mux-api` + `mux-query`.

### Snapshot Model

`mux-view` publishes a stable snapshot for reads:

```rust
// crates/mux-view/src/snapshot.rs
use std::sync::Arc;
use mux_types::*;

pub struct Snapshot {
    pub sessions: Vec<SessionView>,
    pub windows: Vec<WindowView>,
    pub panes: Vec<PaneView>,
}

pub type SnapshotArc = Arc<Snapshot>;
```

### Query DSL

```rust
// crates/mux-query/src/lib.rs
use mux_view::Snapshot;
use mux_types::*;

pub struct Query<'a> { snap: &'a Snapshot }

impl<'a> Query<'a> {
    pub fn sessions(&self) -> impl Iterator<Item = &'a mux_view::SessionView> { todo!() }
    pub fn panes_in_window(&self, w: WindowId) -> impl Iterator<Item = &'a mux_view::PaneView> { todo!() }
}
```

### ORM Wrapper

```rust
// crates/mux-orm/src/lib.rs
use mux_api::ServerHandle;
use mux_types::*;

pub struct Session<'a> { server: &'a ServerHandle, id: SessionId }

impl<'a> Session<'a> {
    pub async fn rename(&self, name: &str) -> Result<(), mux_api::ApiError> {
        self.server.exec(mux_api::Command::RenameSession { session: self.id, name: name.into() }).await
    }
}
```

---

## 11. Runtime Architecture

Runtime uses an actor-style single writer for the graph:

1. A `StateActor` owns `ServerGraph` and applies events.
2. A `PtyActor` per pane owns the real PTY and streams output as events.
3. A `ClientActor` per client owns protocol IO and streams messages as events.
4. A `SnapshotPublisher` holds an `ArcSwap<SnapshotArc>` and publishes refresh notifications.

Decision: use `ArcSwap` only at snapshot boundary, not inside core state.

```rust
// crates/mux-runtime/src/state_actor.rs
pub struct StateActor {
    graph: mux_core::ServerGraph,
    // ...
}
```

---

## 12. Language Bindings

Bindings are thin and should not implement logic.

1. Python uses PyO3 to call into `mux-api`.
2. Node uses NAPI-RS to call into `mux-api`.
3. C++ uses `cxx` to call into a small wrapper over `mux-api`.

Binding surface area:

1. Connect/host.
2. Submit commands/events.
3. Subscribe to snapshot stream.
4. Convert snapshots to foreign-friendly DTOs.

---

## 13. CRDT Transaction Layer

Decision (scope): CRDT is applied to a subset of state first.

Replicable first:

1. Paste buffers.
2. Options/config key-value maps.
3. Session/window/pane metadata and layout intents.

Not replicated first:

1. Raw PTY stream output.
2. Per-client ephemeral UI state (cursor shape, focus, render timing).

Model:

1. Reducer emits `crdt_ops` alongside effects, derived from applied events.
2. CRDT layer can accept remote ops and lower them into core events or apply them into a replicated overlay that yields core events.

```rust
// crates/mux-crdt/src/op.rs
pub enum Op {
    SetOption { scope: Scope, key: String, value: String, ts: LamportTs },
    BufferPut { buffer: mux_types::BufferId, bytes: Vec<u8>, ts: LamportTs },
}
```

Property tests:

1. Convergence for commutative op sets.
2. Idempotence (apply same op twice).

---

## 14. tmux Version Management

We keep `tmux-vm` and `tmux-builder` as tooling to compile multiple tmux versions and run parity tests.

Parity runner responsibilities:

1. Build tmux versions into isolated prefixes.
2. Run the same scenario against tmux and TermForge.
3. Collect logs, protocol traces, and format outputs.

---

## 15. Test Support Crate

`mux-test-support` provides:

1. Tempdir helpers.
2. Unique socket paths.
3. Path guards that reject dangerous paths.
4. Helpers to spawn a managed server and wait for readiness.

---

## 16. Fake PTY Backend

### FakePty Interface

```rust
// crates/mux-pty-fake/src/lib.rs
pub trait Pty {
    fn write(&mut self, bytes: &[u8]);
    fn read(&mut self) -> Vec<u8>;
    fn resize(&mut self, cols: u16, rows: u16);
}
```

### Scenario Format and Recorder (Open Question 5)

We define a deterministic scenario file as an ordered stream of operations:

```rust
// crates/mux-pty-fake/src/scenario.rs
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Scenario {
    pub meta: ScenarioMeta,
    pub ops: Vec<PtyOp>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PtyOp {
    InputFromMux { bytes_b64: String },
    OutputToMux { bytes_b64: String },
    Resize { cols: u16, rows: u16 },
    Tick { now_ms: u64 },
    Exit { code: i32 },
}
```

Recorder rules:

1. Recording is done by running a real PTY process and capturing bidirectional bytes, resize events, and a monotonic tick stream.
2. Playback in tests is fully deterministic: given the same mux input sequence, the fake returns the recorded output sequence.
3. The recorder is an impure tool in `mux-pty-recorder` that emits `Scenario` JSON or CBOR.

Why include `Tick`:

1. It pins down time-dependent behavior without allowing core access to clocks.
2. It allows testing of prefix timeouts and copy-mode search timing behavior deterministically.

---

## 17. Binding Test Frameworks

We test bindings at two layers:

1. Unit tests on conversion code (DTO correctness, error mapping).
2. Integration tests that start an in-process server and run scripted operations.

Python:

1. Run via `pytest` driving the binding and asserting snapshots.

Node:

1. Run via `node:test` or jest driving the binding and asserting snapshots.

---

## 18. Test Framework and Harness Design

### Test Pyramid

1. Pure unit tests: reducer, key resolution, format evaluation, grid performer.
2. Property tests: event sequences, CRDT convergence, grid invariants.
3. Snapshot tests: grid rendering, view snapshots, protocol frames.
4. Fuzzing: protocol decoder, ANSI parser input, format parser.
5. Parity tests: run same scenarios against tmux binaries.

### Strengthened Grid and Parser Strategy (Open Question 7)

Decision: `mux-grid` uses the `vte` crate only as a byte-to-action parser, not as the grid model. We implement our own `Performer` that mutates a pure `Grid` and `CursorState`.

Rationale:

1. `vte` gives a mature ANSI escape parser.
2. Our grid needs tmux-like features (scrollback, selection, hyperlinks) in a data model optimized for snapshots.
3. We keep the model pure and fuzzable.

Conformance strategy:

1. Run vttest-like suites against the performer.
2. Add "record and replay" scenarios from real apps (shell, vim, less) as FakePty scenarios.
3. Snapshot test the rendered grid after each op sequence.

### Strengthened Format Engine Strategy (Open Question 8)

Decision: `mux-format` is a pure parser + compiler + evaluator framework:

1. Parse tmux-style format strings to an AST.
2. Compile to a small bytecode for fast evaluation.
3. Evaluate with a trait-based variable resolver that reads from a snapshot and injected context.

```rust
// crates/mux-format/src/eval.rs
pub trait VarSource {
    fn get_str(&self, key: &str) -> Option<std::borrow::Cow<'_, str>>;
    fn get_num(&self, key: &str) -> Option<i64>;
    fn get_bool(&self, key: &str) -> Option<bool>;
}

pub fn eval(compiled: &Compiled, vars: &dyn VarSource) -> String { todo!() }
```

Parity strategy:

1. `mux-regress` runs `tmux display-message -p` with a corpus of format strings and captured contexts.
2. Compare outputs exactly and store deltas as fixtures.
3. Build a shrinker that finds the smallest failing format string for debugging.

---

## 19. Visual Client (TUI)

`mux-tui` uses snapshots only.

Rules:

1. It never calls into the core graph directly.
2. All writes are submitted as commands/events to `mux-api`.
3. Rendering is driven by `Snapshot` diffs and refresh hints.

---

## 20. DOs and DON'Ts

DO:

1. Keep `mux-core` deterministic and IO-free.
2. Push all IO to runtime via effects.
3. Use typed IDs everywhere, no raw integers in public APIs.
4. Snapshot as the read path; actor as the write path.
5. Quarantine unsafe into `mux-os` with explicit `// SAFETY:` comments.

DON'T:

1. Don’t let `mux-orm` become a second engine; it is a convenience facade.
2. Don’t embed tmux protocol names in core state types.
3. Don’t store `Arc<Mutex<_>>` in the UI read path.
4. Don’t implement ANSI parsing logic in multiple places; it lives in `mux-grid`.

---

## 21. AGENTS.md Template

```md
# AGENTS.md

## Layering Rules

1. `mux-core`, `mux-grid`, `mux-format`, `mux-keys`, `mux-proto`, `mux-query`, `mux-view`, `mux-types` are PURE.
2. PURE crates must not depend on tokio, nix, libc, std::fs, std::net, or any OS APIs.
3. `unsafe` code must live in `mux-os` (or narrowly in binding FFI shims) and must include `// SAFETY:` comments.

## Determinism Rules

1. All state mutation is through `apply_event`.
2. Any time-based behavior must be driven by explicit events (`TimerFired`) with timestamps injected from runtime.
3. No global RNG or `SystemTime` reads in pure code.

## Testing Rules

1. Prefer pure unit tests first.
2. Use FakePty scenarios for integration tests.
3. Add parity fixtures when implementing tmux compatibility behavior.
```

---

## 22. Phased Implementation Plan

Phase 0: Foundations

1. Create workspace crates for `mux-types`, `mux-core`, `mux-grid`, `mux-proto`, `mux-runtime`, `mux-api`.
2. Implement minimal entity graph and reducer with create session/window/pane and split.
3. Implement snapshot publishing and a basic CLI client.

Phase 1: Terminal and PTY

1. Implement `mux-grid` with `vte` performer and snapshot tests.
2. Implement real PTY in `mux-pty` and effect execution in runtime.
3. Implement FakePty and recorder tool.

Phase 2: tmux Compatibility

1. Implement format engine parity harness, then implement core set of format variables.
2. Implement key tables and prefix behavior with deterministic timer events.
3. Implement copy mode state machine and selection extraction.

Phase 3: Tooling and Bindings

1. Implement `mux-orm` convenience layer.
2. Add Python and Node bindings for connect/exec/snapshot.
3. Add `mux-regress` parity runner integrated with `tmux-vm`.

Phase 4: CRDT (Scoped)

1. Add op log for replicable state only.
2. Prove convergence properties in property tests.
3. Add optional network transport outside core.

---

## 23. Risks and Mitigations

1. VT/ANSI completeness risk: many apps rely on obscure modes.
Mitigation: start with `vte` parser; build conformance with vttest-like suite and scenario recorder; fuzz the performer.

2. tmux format engine complexity risk (`format.c` is large and has many edge cases).
Mitigation: implement `mux-format` as AST + bytecode; build a parity corpus runner early; shrink failing cases.

3. Key binding parity risk: tmux has many tables and mode interactions.
Mitigation: represent key tables explicitly in core; write parity fixtures for key resolution; ensure prefix timeouts are event-driven.

4. Copy mode complexity risk (`window-copy.c` has deep behavior).
Mitigation: implement CopyState in core; bind vi/emacs behaviors as key tables; snapshot tests of selection/search results.

5. Protocol ambiguity risk: tmux control mode is not authoritative for all state.
Mitigation: treat control mode as hints; prefer binary protocol framing where structured; record real traces.

6. CRDT scope creep risk.
Mitigation: replicate only safe subsets first; require convergence tests for any new op type.

7. Performance risk: naive snapshot building may be expensive.
Mitigation: keep core graph mutable; build snapshots with incremental caching; use `SmallVec` for adjacency; measure before pulling in persistent structures.

8. WASM compile gate risk: dependencies might block wasm builds.
Mitigation: apply wasm CI only to listed pure crates; keep dependencies minimal; feature gate anything that breaks wasm.
