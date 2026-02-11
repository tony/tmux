# Rust Multiplexer With Perfect tmux Compatibility: Architectural North Star (2026-02-10)

This document is a **north-star architecture** for a from-scratch Rust terminal multiplexer that:

- Is a **tmux-compatible peer** (real tmux clients can attach; it can attach to real tmux servers).
- Keeps **full architectural freedom** in the core (not a line-by-line tmux rewrite).
- Preserves the **vibe-tmux guardrails** (stream-first, fixtures-backed protocol claims, pure core, unsafe quarantine, no panics in hot paths).
- Supports three equally-first-class consumption styles:
  - **Core multiplexer library** (embed/extend in Rust)
  - **ORM-like object graph** (libtmux-shaped)
  - **Language bindings** (Python/Node/C++/C)
- Can be used as a **test framework** (hermetic servers/sockets/PTY fakes).
- Has a state model that can support **transactions and CRDT replication**.

Grounding references read for this design:

- `~/work/rust/vibe-tmux/AGENTS.md` (non-negotiables: stream-first, pure core, unsafe quarantine, testing doctrine)
- `~/work/rust/vibe-tmux/ARCHITECTURE.md` (as-built façade: `ManagedMux`/`SocketActor`, view store + RefreshHints)
- `~/work/rust/vibe-tmux/notes/ideas.md` and `~/work/rust/vibe-tmux/notes/plan.md` (object graph direction, tmux control-notify taxonomy, parity gates)
- tmux source:
  - `~/study/c/tmux/tmux-protocol.h` (protocol version + message types)
  - `~/study/c/tmux/control-notify.c` (control-mode notification taxonomy; `%layout-change` carries formatted layout data)
  - `~/study/c/tmux/client.c`, `~/study/c/tmux/server.c` (socket semantics, daemonize/accept loops)
- Zellij server patterns:
  - `~/study/rust/zellij/zellij-server/src/lib.rs` and `.../thread_bus.rs` (typed instruction enums; channel fan-in via `Select`; error-context propagation)
- Ratatui workspace modularization:
  - `~/study/rust/ratatui/ARCHITECTURE.md` (stable core crate + convenience re-export crate)
- libtmux ORM shape:
  - `~/work/python/libtmux/src/libtmux/server.py` and `.../_internal/query_list.py` (`Server/Session/Window/Pane`, `.cmd()` escape hatch, `QueryList.filter()/get()` operator semantics)

---

## 0. The Big Decision

### Choose: “new kernel + tmux compatibility profile”

Build a **new multiplexer kernel** (cleanly layered, transactional, extensible) and implement tmux compatibility as:

- A **tmux protocol adapter** (`imsg` binary codec and server/client peers)
- A **tmux semantics adapter** (commands/options/format/escape/key tables)

This is the same strategic choice vibe-tmux makes: tmux is the **compatibility surface**, not the internal architecture.

### Why not “rebuild tmux first”

A direct rewrite tends to:

- Freeze core abstractions around tmux’s internal structure
- Make CRDT/transactions “bolt-ons” instead of first-class
- Conflate protocol/semantics/runtime concerns

Instead, keep tmux behavior and protocol as the externally validated acceptance criteria.

---

## 1. Design Principles (Non-Negotiables)

These are the rules that should exist in the new repo’s `AGENTS.md` and be enforced culturally and via tests/lints.

### 1.1 Stream-first responsiveness

- **No steady-state polling** in UI/bindings.
- Updates propagate via **typed streams** (channels/broadcast/async streams).
- One persistent connection per socket per backend; no reconnect-per-tick.
- Latency target: **10-20ms** end-to-end for state propagation.

### 1.2 Purity boundary

`mux-core` is **pure and deterministic**:

- No tokio/async
- No IO/filesystem
- No libc/nix
- No unsafe
- No time/rng without injection

Core is “event reducer + effect planner”. Runtime executes effects.

### 1.3 Protocol authority model

- **Binary protocol is authoritative** for structured state.
- Control-mode (`tmux -C`) is **hints only** (dirty/refresh scheduling), except where tmux explicitly emits formatted data like `%layout-change`.
- Shelling out to `tmux` is an **escape hatch only**.

### 1.4 Unsafe quarantine

- Only `mux-os` may contain handwritten `unsafe`.
- If using `cxx`, allow only cxx-generated unsafe glue in a dedicated crate (eg `mux-cxx`) and forbid handwritten unsafe there.
- Every unsafe block documents invariants with `// SAFETY: ...` and has a focused test.

### 1.5 Decoder hardening

- No “cast bytes to struct” default; parse explicitly.
- Validate lengths before reading; cap allocations.
- Malformed frames are protocol violations: close cleanly, log context.

### 1.6 No panics in hot paths

No `unwrap()`/`expect()` in:

- Protocol decode/encode
- Core reducer
- Server/PTY/event loops

---

## 2. The Layer Cake

A correct architecture here is mostly about **one-way dependencies** and **stable seams**.

### 2.1 Layers

1. **Core kernel (pure):** state graph, typed IDs, reducer, transactions, effect descriptions.
2. **Compatibility profiles (pure + thin runtime glue):** tmux semantics (commands/options/format/keys) + tmux protocol codec.
3. **Runtime/IO:** tokio tasks/threads, sockets, PTYs, SCM_RIGHTS, timers, signals.
4. **Façade API:** managed handles, view store, streams, query engine.
5. **ORM API:** libtmux-shaped object model built on top of views + exec.
6. **Bindings:** thin wrappers over façade + ORM; no business logic in wrappers.

### 2.2 Dependency rule

- Everything points inward.
- Outer layers may depend on inner layers; inner layers never depend on outer.

---

## 3. Proposed Cargo Workspace Layout

The goal is to make “the right thing” the easiest thing.

```text
mux/                        (workspace)
  crates/
    mux-core/               pure kernel: graph + reducer + transactions + effects
    mux-core-types/         pure: typed IDs, newtypes, shared primitives
    mux-core-format/        pure: internal format engine abstractions (not tmux yet)

    mux-compat-tmux/        pure: tmux semantic profile (cmd/options/format/keys mapping)
    mux-proto-tmux/         pure: tmux imsg codec + control-mode parser

    mux-view/               pure: immutable snapshots + traversal helpers
    mux-query/              pure: query DSL + matcher (libtmux QueryList semantics)

    mux-runtime/            tokio + orchestration; actors; connection lifecycle
    mux-os/                 PTY/termios/SCM_RIGHTS + minimal unsafe

    mux-server/             server implementation (tmux peer + native API)
    mux-client/             client implementation (to tmux or to our server)

    mux-api/                ManagedMux/SocketActor facade; view store; hint streams
    mux-orm/                libtmux-shaped API (Server/Session/Window/Pane)

    mux-crdt/               CRDT primitives + op log encoding + merge policies
    mux-repl/               replication transport (optional): websocket/quic/gossip

    mux-ffi/                stable C ABI (optional; minimal)
    mux-cxx/                C++ bridge (cxx), no handwritten unsafe
    mux-python/             pyo3 module; re-export mux-api + mux-orm
    mux-node/               napi/neon module; re-export mux-api + mux-orm

  tools/
    tmux-sniff/             fixtures capture (must preserve SCM_RIGHTS)
    tmux-lint/              static checks vs tmux headers/sources
    regress-audit/          track tmux regress scripts parity
    format-audit/           keep format variables aligned
    command-audit/          keep command coverage aligned
    mux-doctor/             diagnostics

  apps/
    mux/                    CLI (tmux-compatible flags + native commands)
    mux-tui/                ratatui UI (event-driven)

  bindings/
    python/                 packaging (uv/maturin)
    node/                   packaging (pnpm)
    cpp/                    wrapper + examples
```

Notes:

- Keep **pure crates** small and stable (ratatui-style): `mux-core-types`, `mux-view`, `mux-query`.
- Keep **compatibility** separate (`mux-compat-tmux`, `mux-proto-tmux`) so other profiles (eg “screen/tmate-like” or a future native wire protocol) can coexist.
- Keep **bindings** as packaging + thin glue; the logic lives in Rust crates.

---

## 4. Core Kernel: State, Transactions, Effects

### 4.1 The core data model

Core owns a normalized graph:

- `ServerGraph` (or `World`) owns collections:
  - Sessions
  - Windows
  - Panes
  - Clients
  - Jobs
  - Buffers
  - Options / environment

Use typed IDs everywhere:

- `SessionId`, `WindowId`, `PaneId`, `ClientId`, `JobId`, `BufferId`

### 4.2 Reducer: `Event -> (State', Effects)`

Core is a pure reducer:

```rust
pub fn apply_event(state: &mut ServerGraph, event: Event, ctx: &CoreCtx) -> Vec<Effect>;
```

- `Event` is a fact about something that happened or was requested.
- `Effect` is a description of IO to perform (spawn process, write socket frame, resize PTY, persist snapshot, emit notification).

### 4.3 Transactions

Transactions are first-class batches:

- `Tx { tx_id, actor, timestamp?, causal_context?, ops: Vec<Event> }`
- Core applies a tx atomically and emits a tx-scoped effect plan.

Transaction semantics:

- Either all events apply, or the tx fails with typed reasons.
- Effects can be grouped by target (client socket, pty, filesystem) to reduce chatter.

### 4.4 Client-relative semantics

Many tmux semantics are client-relative (eg “current session”, “-t target pane”, “last pane”).

Model this explicitly:

- `ClientContext { client_id, attached_session, key_table, terminal_features, ... }`
- Commands are evaluated with a `ClientContext` (or a `CmdqItem` equivalent) to produce events.

### 4.5 Determinism hooks

To make the kernel usable as a test framework:

- Inject `Now` and `Random`.
- Inject a `ProcessSpawner` trait that can be faked.
- Inject `Terminfo/Capabilities` as data, not a global.

---

## 5. tmux Compatibility Profile

Treat tmux support as a profile that maps external tmux surfaces onto core concepts.

### 5.1 Protocol compatibility (`mux-proto-tmux`)

Responsibilities:

- Binary protocol (`imsg`) frame codec + message structs (grounded in `tmux-protocol.h`).
- Ancillary FD passing (SCM_RIGHTS) and strict FD ownership rules.
- Control-mode parser for `%%...` notifications.

Non-responsibilities:

- No state management.
- No “helpful behavior”. Decode what tmux does; if unclear, add fixtures.

### 5.2 Semantic compatibility (`mux-compat-tmux`)

Responsibilities:

- Command parser and AST aligned to tmux (`cmd-parse.y` semantics).
- Option tables and types aligned to tmux (`options-table.c`).
- Format variables aligned to tmux (`format.c`).
- Key table semantics aligned to tmux.

The profile produces core transactions:

```rust
pub fn tmux_command_to_tx(cmd: TmuxCmdAst, ctx: &ClientContext, state: &ServerGraph) -> Tx;
```

### 5.3 Compatibility mode vs native mode

Two front doors:

- `mux` in **tmux-compat mode**: accepts tmux flags, protocol, and semantics.
- `mux` in **native mode**: exposes additional APIs (transactions, replication, richer events).

But both drive the same kernel.

---

## 6. Runtime: Actors and Stores

### 6.1 Per-socket actor model (vibe-tmux proven)

Adopt vibe-tmux’s proven approach:

- One `SocketActor` / `ManagedMux` per socket.
- Persistent connections.
- A lock-free view store, plus a `RefreshHint` stream.

Key rule:

- Consumers do not “ask the server for updates” on a loop.
- They **subscribe** to hints and then read the latest snapshot.

### 6.2 View store design

Store snapshots as immutable views:

- `StateHandle<Option<View>>` (or ArcSwap)
- Reads are lock-free or effectively lock-free.

### 6.3 Event loop shape (Zellij-validated)

Use channel fan-in:

- Control notify recv -> `SocketEvent::Control`
- Explicit refresh requests -> `SocketEvent::Refresh`
- Wake/debounce -> `SocketEvent::Wake`
- Shutdown -> `SocketEvent::Shutdown`

Driver blocks on `recv()` (no polling).

### 6.4 Threading model

Pick one of two patterns depending on subsystem:

- Zellij-style multiple threads with typed instructions per thread (screen/pty/server/background jobs)
- Tokio task model with `tokio::select!` for multiplexing

Do not mix the two within the same tight loop; choose one per component.

---

## 7. Public Rust API: Two Facades + One ORM

### 7.1 Facade 1: “Managed core access” (`mux-api`)

A single primary entry point that hides plumbing:

- `ManagedMux` (per socket) and `MuxFleet` (multi-socket discovery)
- `subscribe()` returns `RefreshHint` stream
- `view()` returns the latest snapshot
- `exec(cmd)` executes commands/transactions

Public names must not leak internal details:

- No `Frame`, `Store`, `Json` in API type names.

### 7.2 Facade 2: “Server-embedded kernel”

When embedding the server in-process (test framework or embedded app):

- `KernelHandle` with `apply_tx()` and `subscribe_events()`
- A runtime executes effects and feeds results back as events.

### 7.3 ORM layer (`mux-orm`): libtmux-shaped

Expose:

- `Server`, `Session`, `Window`, `Pane` objects (cheap handles referencing IDs + a `ManagedMux`)
- `.cmd(...)` escape hatch on every object
- Query traversal:
  - `server.sessions()` -> `QuerySet<Session>`
  - `session.windows()` -> `QuerySet<Window>`
  - `window.panes()` -> `QuerySet<Pane>`

ORM rule:

- Query evaluation happens in Rust on the snapshot view (like vibe-tmux’s guidance).
- Bindings must not re-implement matching logic.

### 7.4 Query semantics (libtmux parity)

Implement a small query DSL compatible with libtmux operators:

- `field=value`
- `field__contains`, `__icontains`, `__startswith`, `__istartswith`, `__endswith`, `__iendswith`
- `__in`, `__nin`, `__regex` (if needed)

Provide:

- `filter(...) -> QuerySet<T>`
- `get(...) -> Result<T, DoesNotExist|MultipleObjectsReturned>`
- `first()/one_or_none()` helpers for ergonomics

---

## 8. CRDT + Replication: Make It a First-Class Lane

The trick is to keep the kernel deterministic while allowing distributed edits.

### 8.1 Two viable models

#### Model A: Event-sourced + CRDT log

- Kernel applies transactions to state.
- Each tx is appended to an op log.
- Replication merges logs using CRDT metadata (vector clocks / dotted version vectors).

This is a good fit when:

- You want full auditability.
- You can define merge semantics for concurrent txs.

#### Model B: CRDT state as primary

- The “state” itself is CRDT structures.
- Views are derived from CRDT state.

This is harder for tmux compatibility because tmux has many implicit, client-relative rules.

### Recommended: Model A

Keep the kernel reducer the authority and replicate **transactions** with causal metadata.

### 8.2 Transaction identity and causality

Define:

- `TxId = (ReplicaId, Counter)`
- `CausalContext = { seen: VersionVector }`

Rules:

- Local tx increments counter.
- Remote tx is applied when dependencies are satisfied (or applied with a “pending” buffer if you choose).

### 8.3 What is safe to make commutative

Not everything in a multiplexer is commutative. Split state into:

- **Replicated configuration/state**: options, layouts, metadata, named sessions/windows, key tables.
- **Non-replicated ephemeral runtime**: PTY file descriptors, process PIDs, local client TTY geometry.

Only replicate the first class.

### 8.4 Conflict resolution policy

Conflicts should be explicit and testable:

- Session/window/pane naming: LWW register or “rename is last-writer-wins per object”.
- Collections (sessions/windows/panes): OR-Set semantics.
- Layout: treat as derived from a replicated layout tree, not from pixel geometry.

### 8.5 Wire protocol

Do not reuse tmux protocol for replication.

Provide a separate native API:

- `mux-repl` message: `ApplyTx(TxEnvelope)`
- `TxEnvelope` includes `tx`, `causal_context`, and optional signatures.

---

## 9. Language Bindings

### 9.1 Binding philosophy

- Bindings expose `ManagedMux` + ORM objects.
- Bindings must be stream-first and must not block their language runtime.

### 9.2 C ABI vs C++

If you need a stable ABI:

- Provide a tiny, stable C ABI in `mux-ffi` that exposes:
  - create/destroy handle
  - subscribe to hints (callback-based)
  - read snapshot (serialized or structured)
  - exec command/tx

For modern C++ DX:

- Prefer `cxx` bridge in `mux-cxx` and build a thin RAII wrapper.

### 9.3 Python

- Provide `pyo3` module that exposes:
  - `Server`/`Session`/`Window`/`Pane`
  - `server.cmd()`
  - `server.sessions.filter(...)`
  - `server.subscribe_hints()` yielding an iterator/async iterator

Python cleanup must be easy:

- Context manager (like libtmux): `with Server(...) as s: ...`
- Hermetic socket fixtures.

### 9.4 Node

- Provide `napi`/`neon` module.
- Expose hint stream as `AsyncIterator` (do not block event loop).

---

## 10. Compatibility Acceptance Criteria (What “100% compatible” means)

Compatibility is not vibes. It needs gates.

### 10.1 Protocol correctness gates

- Fixture capture tool that preserves SCM_RIGHTS FDs.
- Snapshot tests for decode/encode.
- Chunking robustness tests (random segmentation of frames).

### 10.2 Behavior parity gates

- Run upstream tmux regress scripts against:
  - real tmux (baseline)
  - our server (target)

Keep audits aligned:

- Command coverage vs tmux source
- Format variables vs tmux `format.c`
- Options vs tmux `options-table.c`

### 10.3 ORM parity gates

- Run libtmux’s test suite against the Rust backend (like vibe-tmux does).
- Enforce QueryList operator coverage parity.

---

## 11. Suggested `AGENTS.md` for the New Repo (LLM North-Star Rules)

Create `AGENTS.md` at the repo root with at least:

- Never kill the developer’s real tmux server.
- Use dedicated test sockets (eg `/tmp/mux-*`) and verify paths before destructive commands.
- Stream-first: typed streams over polling.
- Core is pure; runtime executes effects.
- Protocol claims backed by tmux source or fixtures.
- Unsafe quarantine rules.
- No panics in hot paths.
- Testing doctrine:
  - protocol fixtures + chunking tests
  - core reducer tests
  - hermetic integration tests
  - periodic real tmux parity runs

Also include a “public API naming rule”:

- Do not leak `View`, `Frame`, `Store`, `Json` in public names; keep those debug-only.

---

## 12. Implementation Roadmap (Phased, Test-First)

### Phase 0: Kernel and harness

- `mux-core-types` + `mux-core`
- Reducer + effect system
- In-process harness executing effects against fakes

### Phase 1: tmux protocol codec + sniff fixtures

- `mux-proto-tmux` codec
- Sniffer that captures frames + FDs
- Decoder robustness tests

### Phase 2: ManagedMux facade (vibe-tmux pattern)

- Per-socket actor
- Lock-free view store
- RefreshHint stream

### Phase 3: tmux-compatible server and client

- Real tmux client attaches to us
- We can attach to real tmux server

### Phase 4: tmux semantics profile

- Command parsing/dispatch
- Options/format parity audits

### Phase 5: ORM and bindings

- Rust ORM
- Python and Node
- C++ bridge

### Phase 6: Transactions + CRDT replication

- Tx envelopes + causal metadata
- Replication transport
- Conflict resolution test suite

---

## 13. Do / Do Not (Quick Checklist)

### Do

- Use `ManagedMux` + view snapshots as the primary read path.
- Put matching/filtering logic in Rust (shared by all bindings).
- Keep `mux-core` deterministic and IO-free.
- Add parity tests before “big refactors”.

### Do not

- Implement tmux behavior by guesswork.
- Add polling loops for “simplicity”.
- Leak internal store concepts into public APIs.
- Spread unsafe across crates.

