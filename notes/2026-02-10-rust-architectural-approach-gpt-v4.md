# Rust Mux With Perfect tmux Compatibility: North-Star Architecture (Greenfield)

This is a “begin anew” layout that keeps the **vibe-tmux** layering discipline (pure core + stream-first IO facade + unsafe quarantine + hermetic harnessing), while adding architectural room for:
- a **new native multiplexer engine**
- a **100% tmux-compatible mode** implemented as an adapter on top of that engine
- an **ORM-like object graph API** (Rust + Python/Node bindings) with libtmux-level traversal and filtering
- **CRDT/transactional replication** (server-to-server, local-first, collaborative control)
- strong **testability** (PTY snapshots, protocol fixtures, versioned tmux matrix)

It is intentionally written as acceptance criteria for an LLM or team to execute against.

---

## 0) Decide: “New Mux + tmux Compatibility Adapter” (Recommended)

### Why not “rebuild tmux first”
Rebuilding tmux *as-is* tends to ossify the internal design around C-era constraints. You already proved in vibe-tmux that **tmux compatibility is best treated as a protocol/semantics target**, not an implementation template.

### What to do instead
Build a **new engine** with:
- a pure deterministic core state machine (like `crates/mux-core` in vibe-tmux)
- a runtime that executes `Effect`s (PTY/sockets/jobs)
- a tmux-compat “front door” that translates tmux protocol + commands into core `Event`s, and core state into tmux-visible answers

This mirrors vibe-tmux’s successful separation:
- Stream-first facade + stored view + hint stream (`crates/mux-api/src/managed.rs`)
- Pure core event/effect engine (`crates/mux-core/src/event.rs`, `crates/mux-core/src/effect.rs`, `crates/mux-core/src/engine.rs`)
- Binding-friendly handles (`crates/mux-core/src/handle.rs`) and facade (`crates/mux-core/src/facade.rs`)
- Lock-free-ish snapshot reads via `StateHandle<T>` (`crates/mux-refresh/src/state.rs`)
- Hermetic tmux test harness (`crates/mux-test-support/src/tmux.rs`, `crates/mux-test-support/src/path_guard.rs`)

---

## 1) Non-Negotiable Repo Rules (AGENTS.md North Star)

Carry forward vibe-tmux’s guardrails and make them repo-root laws.

### Safety
- Never touch the developer’s real tmux server.
- Always isolate sockets and tmpdirs (see vibe’s `TmuxTestServer` using `TMUX_TMPDIR` and refusing `default`: `crates/mux-test-support/src/tmux.rs`, `crates/mux-test-support/src/path_guard.rs`).

### Stream-first responsiveness
- No tick-polling as the primary update mechanism.
- Update propagation target ~10–20ms.
- “Persistent connection per socket per backend; never reconnect per tick.”
- Model: the vibe-tmux `RefreshDriver` fan-in loop (`crates/mux-api/src/managed.rs`) is the baseline architecture: event channel + control recv thread + wake timer thread + store updates + broadcast hints.

### Purity boundary
- `mux-core` is pure:
  - no tokio, no IO, no nix/libc, no filesystem
  - no unsafe
  - any time/rng must be injected
- Runtime executes effects; core only emits them.

### Unsafe quarantine
- Only `mux-os` may contain handwritten `unsafe`.
- If you need codegen unsafe (e.g. cxx bridge), it must be isolated and documented (vibe-tmux does this with `mux-cxx` policy in `AGENTS.md`).

### Protocol authority
- Any tmux protocol claim must be backed by:
  - upstream tmux source (`~/study/c/tmux`), or
  - captured fixtures that preserve SCM_RIGHTS FDs (vibe has `tools/tmux-sniff`)

### No panic/unwrap in hot paths
- Same lint posture as vibe’s workspace lints (deny `unwrap`, `expect`, `panic`) in root `Cargo.toml`.

---

## 2) Project Layout (Workspace)

Use a Rust workspace similar to vibe-tmux’s proven split, but with explicit “native engine” vs “tmux compatibility” seams and a first-class ORM API crate.

### Top-level
```
mux/
├── AGENTS.md
├── ARCHITECTURE.md
├── Cargo.toml               # workspace, lints, shared deps
├── justfile                 # build/test/audit entry points
├── crates/
├── tools/
├── bindings/
└── tests/                   # end-to-end + golden suites
```

### Core and model (pure)
1. `crates/mux-core/` (PURE)
   - Domain graph + typed IDs + state machine.
   - Inputs: `Event`
   - Outputs: `Effect`
   - Provide:
     - `Event` and `Effect` (pattern: `crates/mux-core/src/event.rs`, `crates/mux-core/src/effect.rs` in vibe-tmux)
     - `ServerGraph` (or `EngineGraph`) as authoritative state
     - `apply_event` engine (pattern: `crates/mux-core/src/engine.rs` in vibe-tmux)
     - `ServerHandle` traversal (pattern: `crates/mux-core/src/handle.rs` in vibe-tmux)
     - `ServerFacade` command-ish facade (pattern: `crates/mux-core/src/facade.rs` in vibe-tmux)

2. `crates/mux-txn/` (PURE)
   - Transaction boundary + oplog types, independent of transport.
   - Produces *deterministic* sequences of core `Event`s (or a higher-level `EngineOp` that deterministically lowers into `Event`s).
   - Pluggable conflict policy hooks, but deterministic.

3. `crates/mux-crdt/` (PURE)
   - CRDT merge + causality metadata (Lamport/Hybrid Logical Clock), id allocation scheme, and op merge rules.
   - Outputs: ordered/causally-resolved `mux-txn` transactions.

### tmux compatibility (mostly pure, IO-free where possible)
4. `crates/mux-tmux-proto/` (PURE)
   - Binary protocol codec/parsers, control-mode parser.
   - Fixtures + chunking robustness tests (like vibe’s `mux-proto` emphasis in `AGENTS.md`).

5. `crates/mux-tmux-compat/` (PURE or “almost pure”)
   - Translator layer:
     - tmux commands/proto → core `Event`/transactions
     - core state → tmux-visible responses (format expansions, lists, ids)
   - Key point: keep IO and sockets out. This is “pure semantics mapping”.

### Runtime / IO / processes
6. `crates/mux-runtime/`
   - Tokio/threads orchestration; executes `Effect`s.
   - Actor model: per-server and per-client tasks; fan-in channels (Zellij-style), but maintain vibe’s “block on streams”.

7. `crates/mux-store/` (PURE)
   - Snapshot state store + serialized mutation queue.
   - Start from vibe’s `StateStore<T>`, `StateHandle<T>`, `MutationQueue<T>` design (`crates/mux-refresh/src/state.rs`).
   - Provide “views” for consumers (UI/bindings) with lock-free reads.

8. `crates/mux-api/`
   - Stream-first facade and “managed” per-socket/per-instance actor.
   - Baseline: vibe’s `ManagedMux = SocketActor` + `RefreshDriver` (`crates/mux-api/src/managed.rs`).
   - Must expose:
     - `subscribe()` stream of `RefreshHint`
     - read-only snapshot view getter
     - `cmd()` and strongly-typed operations
     - “client-context” execution (mirrors `cmd_for_client` pattern in vibe’s `SocketActor`)

9. `crates/mux-pty/`, `crates/mux-pty-os/`, `crates/mux-pty-portable/`, `crates/mux-pty-fake/`
   - Follow vibe’s multi-backend PTY pattern (see workspace members in vibe’s root `Cargo.toml`).
   - `mux-pty-fake` is critical for deterministic snapshot tests.

10. `crates/mux-os/`
   - All syscalls, SCM_RIGHTS, ioctl, signal handling, fd hygiene.
   - Only place for handwritten unsafe.

### ORM / query / bindings-friendly API
11. `crates/mux-orm/`
   - This is the “libtmux-in-Rust” layer: expressive traversal + filtering over a live object graph.
   - It should be **thin** over `mux-api`:
     - read through snapshot handle (`StateHandle<Option<View>>`-style)
     - write through façade commands / typed operations
   - Provide:
     - `Server`, `Session`, `Window`, `Pane`, `Client`, `Job` wrappers
     - relations returning queryable collections
     - `.cmd(...)` escape hatch (like libtmux’s `.cmd()` on every object)
     - `QuerySet<T>` with `.filter(...)` / `.get(...)` semantics

   Grounding in libtmux patterns:
   - libtmux relations return `QueryList` with `.filter()`/`.get()` (`src/libtmux/session.py`, `src/libtmux/_internal/query_list.py`)
   - hydration uses a central “format string for all fields” approach (`src/libtmux/neo.py`)
   - objects have context manager cleanup patterns (`src/libtmux/server.py`, `src/libtmux/session.py`)

   Rust-side equivalent:
   - Don’t replicate libtmux’s “fetch objs by formatting all fields” internally; you already have structured views. But mirror its **API feel**.

12. `crates/mux-query/` (PURE)
   - Query DSL and operators.
   - vibe-tmux already enforces “QuerySet execution in Rust” in `AGENTS.md`.

### Server/client binaries + UI
13. `crates/muxd/` (server binary)
   - Native mux server.
   - Can optionally start in “tmux-compat server mode” that speaks tmux protocol and semantics via `mux-tmux-compat`.

14. `crates/mux/` (client binary)
   - CLI for both:
     - attaching to native mux
     - connecting to real tmux servers (like vibe-tmux does as a tmux peer)

15. `tools/mux-tui/`
   - Ratatui UI (or custom rendering later).
   - Must consume `RefreshHint` streams and snapshot reads, like vibe’s mux-tui architecture describes in `ARCHITECTURE.md` and implements via a blocking event channel.

### Bindings
16. `bindings/python/`
   - pyo3 + maturin (match vibe-tmux binding direction).
   - Python objects mirror libtmux: `Server`, `Session`, `Window`, `Pane`.
   - Include pytest plugin fixtures like libtmux has (`src/libtmux/pytest_plugin.py`).

17. `bindings/node/`
   - neon or napi-rs, but keep the same binding guardrails as vibe: do not expose internal frames/stores in public names.
   - Provide async iterators for refresh hints (avoid blocking event loop).

18. Optional: `bindings/cpp/` via cxx (vibe-tmux uses `mux-cxx` + wrapper).

### Version management + audits (tools)
Carry forward vibe-tmux’s tmux version manager and audit utilities:
- `tools/tmux-builder/` (see `tools/tmux-builder/src/lib.rs`)
- `tools/tmux-vm/` (see `tools/tmux-vm/src/main.rs` and `tools/tmux-vm/AGENTS.md`)
- `tools/tmux-worktrees/` (already present in vibe workspace)
- `tools/tmux-sniff/` (fixtures with SCM_RIGHTS preserved)
- `tools/format-audit/`, `tools/regress-audit/`, `tools/tmux-command-audit/`

These are not “nice to have”; they are what keeps tmux compatibility from rotting.

---

## 3) Layering Contract (Hard Boundaries)

### Boundary A: Core is deterministic
- `mux-core` must be runnable in a pure unit test: feed events, assert effects/state.
- No “ask the OS” inside core.
- If core needs time, inject a `Clock` trait passed in via event payloads or engine context.

### Boundary B: Store snapshots are the read path
UI and bindings read *only* via stored views/snapshots, woken by hint streams.

This is exactly vibe’s model:
- snapshot: `StateHandle<Option<View>>` with `try_with_read` (`crates/mux-api/src/managed.rs`)
- hint stream: `broadcast::Sender<RefreshHint>` and `subscribe()` (`crates/mux-api/src/managed.rs`)
- runtime updates store via serialized mutation queue (`crates/mux-refresh/src/state.rs`)

### Boundary C: tmux compatibility is an adapter
- No tmux protocol details in core names/types.
- tmux IDs and quirks live in `mux-tmux-compat` and `mux-tmux-proto`.
- Keep the “authority model” from vibe:
  - binary protocol is authoritative for structured data
  - control mode is hints only (except where proven otherwise via tmux source/fixtures)

### Boundary D: ORM API is a façade, not the engine
The ORM layer must not become “a second engine”:
- It should translate “user intent” into `cmd()` / typed ops / transactions.
- It should not implement business logic that belongs in core.

---

## 4) Public API Shapes (Acceptance Criteria)

### Rust: three-tier API (choose your sharpness)

1) **Core embedding** (testing, research)
```rust
use mux_core::{ServerFacade, Event};

let mut f = ServerFacade::new();
let out = f.apply(Event::CreateSession { name: "s".into() });
```
Grounding: vibe’s `ServerFacade` and `Event`/`Effect` (`crates/mux-core/src/facade.rs`, `crates/mux-core/src/event.rs`).

2) **Managed runtime** (production, UI, bindings)
- Equivalent to vibe’s `ManagedMux`/`SocketActor` + `RefreshDriver` (`crates/mux-api/src/managed.rs`).
- Must expose:
  - `handle().subscribe()` refresh hints
  - snapshot reads
  - command exec and client-context exec (`cmd_for_client` pattern in vibe’s `SocketActor`)

3) **ORM** (beautiful object graph)
- Objects hold a reference to a shared “live handle” and an ID.
- Relations return query sets:
  - `server.sessions() -> Sessions`
  - `session.windows() -> Windows`
  - `window.panes() -> Panes`
- Query sets support:
  - `filter(...)` (pure filtering executed in Rust over snapshot)
  - `get(...)` (0/1 semantics like libtmux’s `QueryList.get()`)
  - iteration, `first()`, `one()`, etc.
Grounding: libtmux `QueryList` (`src/libtmux/_internal/query_list.py`) and relations (`src/libtmux/session.py`).

### Python bindings
Target libtmux parity *in feel*:
- `server.sessions.filter(session_name__startswith="foo")`
- `session.windows.get(window_name="bar")`
- `pane.send_keys("echo hi")`, `pane.capture_pane()`
- Context managers for cleanup (libtmux does this on `Server` and `Session`: `src/libtmux/server.py`, `src/libtmux/session.py`)

But internally:
- reads come from Rust snapshot store
- writes go through Rust command/typed ops
- pytest plugin provides isolated mux servers and isolated tmux servers, similar to vibe’s test harness constraints (`crates/mux-test-support/src/tmux.rs`)

### Node bindings
- Provide:
  - object graph + query filtering
  - async iterator for refresh hints
  - PTY snapshot helpers for vitest
- Must not block the JS event loop; if you need “next hint”, it’s async.

---

## 5) Transactions + CRDT (How it Fits Without Corrupting Layers)

### Goals
- Allow programmatic, testable multi-step mutations:
  - “create session + create window + split pane + send keys”
  - commit as one transaction for atomicity and replication

### Minimal architecture
- `mux-txn` defines:
  - `Txn { ops: Vec<Op> }`
  - `Op` is a semantic intent (create pane, move focus, set option)
  - deterministic lowering: `Txn -> Vec<Event>`
- `mux-crdt` defines:
  - `CrdtEnvelope { site_id, hlc, txn }`
  - merge rules return a stable, deterministic ordering of txns
- `mux-runtime` applies transactions:
  - converts to events
  - core returns effects
  - runtime executes effects
- `mux-api` publishes:
  - `begin_txn()` / `commit_txn()` returning a `TxnId` and emitting a refresh hint scope

### tmux compatibility constraint
tmux clients will never “send CRDT”. They send commands/protocol frames. The tmux adapter must be able to:
- treat each tmux command as either:
  - a single transaction, or
  - a stream of events (if tmux semantics require it)
- preserve tmux’s observed ordering and outputs (validated via real tmux tests)

---

## 6) Testing Strategy (Must Be First-Class)

### Pure unit tests
- `mux-core`: event → state/effects determinism.
- `mux-txn`/`mux-crdt`: merge determinism, replay, id allocation.

### Protocol tests
- `mux-tmux-proto`:
  - fixture-driven tests (including SCM_RIGHTS)
  - chunking segmentation fuzz/proptest (vibe requires this in `AGENTS.md`)

### Hermetic integration tests
Adopt vibe’s harness model as a baseline:
- `TmuxTestServer`:
  - never uses `default` socket name
  - always sets `TMUX_TMPDIR`
  - always `-f /dev/null`
  - always `env_remove("TMUX")`
  - only kills its own socket (`crates/mux-test-support/src/tmux.rs`)
- Unified “test against tmux or against muxd” abstraction (vibe has `TestServer`: `crates/mux-test-support/src/server.rs`)

### PTY snapshot tests
- Provide a fake PTY backend (`mux-pty-fake`) so:
  - deterministic rendering snapshots
  - deterministic input/output scripts
- Also support OS PTY strict diagnostics (vibe has `pty-test`, `pty-report`, `pty-clean` in workspace members).

### Version matrix
Use a tmux version manager like vibe’s:
- `tmux-vm exec <ver> -- <cmd>` must:
  - set `TMUX_BIN` and prepend `PATH` (see `tools/tmux-vm/AGENTS.md`)
- `tmux-vm regress <ver>` must:
  - set `TEST_TMUX`
  - isolate sockets via `TMUX_TMPDIR` (same file)
- Run:
  - parity suite against multiple tmux versions (3.2a+ if you target libtmux range; vibe currently references 3.4+ heavily)

### Bindings tests
- Python: pytest fixtures spin up muxd in isolated socket/tmpdir; snapshot PTY output.
- Node: vitest does the same; test helpers must guarantee cleanup and isolation.

---

## 7) Bootstrap/Version Management (tmux source + bins)

Make “tmux version availability” a first-class development dependency, as vibe already did.

### Required tools to port/reuse conceptually
- `tools/tmux-builder` for building a given tag into a prefix (see `tools/tmux-builder/src/lib.rs`)
- `tools/tmux-vm` for selecting a version and running commands with `TMUX_BIN`/`PATH` set (see `tools/tmux-vm/src/main.rs`, `tools/tmux-vm/AGENTS.md`)
- `tools/tmux-worktrees` for managing worktrees/tag resolution (present in vibe workspace)
- `tools/tmux-sniff` for capturing fixtures including FD passing

### Required acceptance behaviors
- Offline reuse of cached builds
- Hermetic tests never require system tmux (but can use it if present)
- Ability to run regress scripts safely (socket isolation)

---

## 8) Implementation Steps (In Order)

This order is optimized to keep semantics testable at each step.

1. Repo bootstrap
- Create workspace skeleton and root laws:
  - `AGENTS.md` (copy vibe-tmux’s rule set as baseline)
  - `ARCHITECTURE.md` (this doc distilled)
  - root `Cargo.toml` with lints matching vibe’s (deny unwrap/expect/panic)
  - `justfile` with `check`, `test`, `fmt`, `clippy`, `parity`, `tmux-vm` wrappers

2. Core engine first
- Implement `mux-core` with:
  - `Event`, `Effect`, `ServerGraph`, `apply_event`
  - `ServerHandle` traversal and `ServerFacade` (grounding: vibe `crates/mux-core/src/handle.rs`, `crates/mux-core/src/facade.rs`)

3. Store + managed facade early
- Implement `mux-store` from vibe `StateHandle`/`MutationQueue` (`crates/mux-refresh/src/state.rs`)
- Implement `mux-api` “managed actor” patterned after vibe’s `SocketActor`/`RefreshDriver` (`crates/mux-api/src/managed.rs`)
  - Even if initially it manages an in-process engine (no sockets yet), preserve the same interface:
    - snapshot + broadcast hints + request_refresh

4. PTY abstraction + fake backend
- Implement `mux-pty` trait
- Implement `mux-pty-fake` immediately so snapshot tests can start before OS PTY works

5. Native runtime and server
- `mux-runtime` executes core effects; `muxd` exposes a control transport (native protocol)
- Add hermetic integration tests using fake PTY

6. tmux protocol codec + fixture capture tooling
- Implement `mux-tmux-proto`
- Add `tools/tmux-sniff` equivalent early so protocol truth is captured, not guessed

7. tmux compatibility adapter
- `mux-tmux-compat` translating:
  - tmux commands/proto → core ops/events
  - core state → tmux answers
- Add parity tests: “real tmux client attaches to muxd in tmux-compat mode”

8. ORM layer
- Implement `mux-orm`:
  - object graph wrappers, relations, query DSL
  - `.cmd(...)` escape hatch
- Enforce: queries execute in Rust over snapshot (vibe `AGENTS.md`)

9. Bindings
- Python binding maps `mux-orm` objects closely to libtmux objects
- Node binding mirrors the same graph and offers async hint streams

10. CRDT/transactions
- Add `mux-txn` + `mux-crdt`
- Integrate into runtime and expose in API
- Add deterministic merge/replay tests and a replication harness

---

## 9) Files To Create/Modify (Greenfield Checklist)

Root:
- `AGENTS.md`
- `ARCHITECTURE.md`
- `Cargo.toml`
- `justfile`

Crates (initial):
- `crates/mux-core/Cargo.toml`, `crates/mux-core/src/{lib.rs,event.rs,effect.rs,graph.rs,engine.rs,handle.rs,facade.rs}`
- `crates/mux-store/...` (start from vibe’s `crates/mux-refresh/src/state.rs` design)
- `crates/mux-api/...` (start from vibe’s `crates/mux-api/src/managed.rs` surface)
- `crates/mux-pty/...`, `crates/mux-pty-fake/...`
- `crates/mux-os/...` (empty initially)
- `crates/mux-runtime/...`
- `crates/mux-orm/...`
- `crates/mux-tmux-proto/...`
- `crates/mux-tmux-compat/...`
- `crates/mux-txn/...`, `crates/mux-crdt/...`

Tools:
- `tools/tmux-builder/...` (or reuse conceptually from vibe: `tools/tmux-builder/src/lib.rs`)
- `tools/tmux-vm/...` (or reuse conceptually from vibe: `tools/tmux-vm/src/main.rs`)
- `tools/tmux-sniff/...`

Bindings:
- `bindings/python/...`
- `bindings/node/...`

Test support:
- `crates/mux-test-support/...` with:
  - socket guards and tmux harness patterned after vibe (`crates/mux-test-support/src/tmux.rs`, `crates/mux-test-support/src/path_guard.rs`)

---

## 10) Risks / Hard Problems (Call Them Early)

- **Over-generalizing core too early**: keep core “mux-native” and add tmux profile via adapter, not via generic abstractions everywhere.
- **tmux semantics breadth**: parity requires relentless fixture-driven and real-tmux testing; avoid “invented” protocol behavior.
- **CRDT complexity**: keep CRDT out of core; treat it as a transaction ordering/merge layer that produces deterministic engine inputs.
- **FFI lifetime and snapshot consistency**: bindings must not expose internal stores; they should hold IDs + shared handle and re-hydrate from snapshots on demand.
- **PTY determinism**: fake PTY backend is mandatory for snapshot testing; OS PTY correctness will otherwise stall progress.
- **Performance regressions**: lock-free read path and stream-first updates are critical; copy vibe’s approach rather than reinventing.

---

## 11) Concrete “Do / Do Not” List (LLM Acceptance Criteria)

**Do**
- Keep `mux-core` pure and deterministic (like vibe’s core).
- Implement stream-first managed facade with snapshot store + hint stream (like vibe’s `RefreshDriver`).
- Use hermetic socket/tmpdir harnessing (like vibe’s `TmuxTestServer`).
- Treat tmux control mode as hints, binary protocol as authority (vibe’s architecture stance).
- Make queries execute in Rust, not in JS/Python, matching vibe’s binding guardrails.

**Do Not**
- Do not let bindings expose internal “frame/view/store/json” concepts in public names (vibe `AGENTS.md` explicitly forbids this).
- Do not add polling-based refresh loops as the primary mechanism.
- Do not put unsafe outside `mux-os` (except tightly-scoped codegen glue if you choose cxx).
- Do not claim tmux protocol behavior without source/fixture proof.

---

If you want, I can follow up with a second doc that gets more concrete about:
1) the exact Rust `mux-orm` API (type signatures and example usage), and
2) the tmux-compat boundary: what lives in `mux-tmux-compat` vs `mux-tmux-proto` vs `mux-runtime`.
