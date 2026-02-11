# Rust Multiplexer With Perfect tmux Compatibility: Architectural North Star (2026-02-10)

This document proposes a *from-scratch* Rust terminal multiplexer architecture that:

- Achieves **100% tmux protocol/behavior compatibility** via a dedicated compatibility layer.
- Preserves **full architectural freedom** in the core (not a line-by-line tmux rewrite).
- Supports **vibe-tmux feature parity** and its hard constraints (stream-first, pure core, fixtures-backed protocol claims, unsafe quarantine, etc.).
- Exposes a **beautiful Rust API** that can be used:
  - as a “normal” multiplexer library (embed/extend)
  - as a tmux-compatible client/server peer
  - as an **ORM-like object graph** (libtmux-shaped)
  - as a **test framework** (isolated servers/sockets/fixtures)
  - as a platform for **CRDT / transactional state replication**
  - via **language bindings** without leaking internal plumbing

Grounding references used while drafting:

- `~/work/rust/vibe-tmux/AGENTS.md` (guardrails + purity boundary + stream-first + testing doctrine)
- `~/work/rust/vibe-tmux/ARCHITECTURE.md` (as-built crate layout and façade design)
- `~/work/rust/vibe-tmux/notes/ideas.md` (object graph/API ergonomics; control notifications; Zellij actor notes)
- `~/work/rust/vibe-tmux/notes/plan.md` (parity gates and audit workflows)
- `~/study/c/tmux/` source:
  - `tmux-protocol.h` (protocol constants)
  - `control-notify.c` (control notification taxonomy; `%layout-change` carries formatted data)
  - `cmd-parse.y`, `options-table.c`, `format.c`, `client.c` (config grammar, option tables, format engine, client connect semantics)
- `~/study/rust/zellij/zellij-server/src/lib.rs` and `.../thread_bus.rs` patterns (typed instruction enums, channel fan-in, error context)
- `~/study/rust/ratatui/ARCHITECTURE.md` (workspace modularization + re-export “main crate” strategy)
- `~/work/python/libtmux/`:
  - `Server/Session/Window/Pane` hierarchy
  - `.cmd()` escape hatch
  - `QueryList.filter()/get()` semantics + pytest plugin for hermetic cleanup

---

## 1. Core Decision: New Multiplexer + tmux Compatibility Profile

### Pick: “New core, perfect tmux profile”

Build a **new multiplexer kernel** with clean layering and modern concerns (transactions, CRDT, bindings), and treat tmux as a **compatibility profile**:

- tmux is not the internal model.
- tmux is a *projection*:
  - protocol (imsg + SCM_RIGHTS)
  - command language/config grammar
  - option schema/types/defaults
  - format string expansion semantics
  - control-mode notifications (hints)
  - behavioral quirks

This matches what vibe-tmux already validated in practice:

- binary protocol is authoritative; control mode is hints.
- stream-first façade with stored view snapshots is the right integration boundary.

### Why not “rebuild tmux first”

A faithful port tends to preserve:

- global state and incidental coupling
- hard-to-test time/IO-dependent behavior
- FFI-like boundaries inside Rust (unsafe creep)

Instead:

- keep the deterministic state machine pure
- isolate IO/protocol/PTY/platform code
- drive compatibility through fixtures + parity harnesses

---

## 2. Non-Negotiables (AGENTS.md Rules for the New Repo)

These are acceptance constraints. Put them in the new repo’s `AGENTS.md` and enforce via lints/CI.

### 2.1 Safety and correctness

- **Never kill real tmux**: all destructive tests use explicit temp sockets (eg `/tmp/<project>-*`).
- **Source-of-truth**: every protocol/behavior claim must be backed by:
  - tmux source/headers, or
  - captured fixtures that preserve `SCM_RIGHTS` file descriptors.
- **Protocol parsing**:
  - explicit parsing, validate lengths, cap allocations
  - malformed frames close the connection cleanly
- **FD passing**:
  - ancillary data is forwarded, not dropped
  - received FDs are bound to the correct frame
  - strict ownership and close-on-drop
- **No panics/unwraps in hot paths**:
  - protocol decode/encode
  - server loops
  - PTY loops

### 2.2 Architectural boundaries

- `mux-core` is **PURE**:
  - no tokio, no async
  - no nix/libc
  - no filesystem/sockets/PTY
  - no unsafe
  - no time/rng without injection
- Unsafe is quarantined:
  - only in `mux-os` (plus cxx-generated glue if you choose that route)
  - every unsafe block has documented invariants + a focused test

### 2.3 Responsiveness (stream-first)

- Prefer **streams over polling**.
- Event loops block on channels/streams and wake immediately.
- **Latency target** for state propagation: 10–20ms.
- Persistent connections per socket/backend; no per-tick reconnect/identify.

### 2.4 Public API hygiene

- Bindings and top-level APIs expose a clean object graph and command exec.
- Do not leak internal names like “frame/store/json/planner” into user-facing APIs.
- Query/filter logic runs in Rust (bindings pass criteria; they do not reimplement matching).

---

## 3. Architectural Shape (Three Planes)

Design the codebase as three planes with explicit boundaries.

### Plane A: Deterministic Kernel (pure)

- Canonical state graph
- Deterministic reduction: `Event -> (State, Effects)`
- Transaction boundaries and op-log
- Query / view materialization

### Plane B: Runtime + IO (impure)

- PTYs, processes, sockets, SCM_RIGHTS
- Async runtimes (tokio)
- Watchers (socket discovery)
- Timers, debouncers
- Telemetry and tracing

### Plane C: Facades + Integrations

- Managed per-socket actor façade (`ManagedMux`-style)
- Tmux compatibility mode (client + server)
- ORM-like object graph (Rust + bindings)
- Test harness and fixtures

The “north star” is: **Plane C depends on Plane B depends on Plane A, never the reverse**.

---

## 4. Proposed Workspace Layout (Crates)

Use a Cargo workspace like vibe-tmux and Ratatui: many small crates + one “main crate” that re-exports stable APIs.

### 4.1 Top-level

```
<repo>/
  AGENTS.md
  ARCHITECTURE.md
  Cargo.toml                 # workspace
  crates/
  tools/
  bindings/
  notes/
  tests/
```

### 4.2 Pure crates (Plane A)

```
crates/
  mux-core/                   # PURE kernel: graph, events/effects, reducer, op-log
  mux-tx/                     # PURE transactions + CRDT-friendly op model (optional; may live in mux-core)
  mux-format/                 # PURE format engine (tmux semantics as profile module)
  mux-options/                # PURE options schema/types/defaults; tmux option table imported here
  mux-cmdlang/                # PURE cmd/config grammar AST + parser (tmux command language)
  mux-query/                  # PURE QuerySpec/QueryValue/Queryable (vibe-tmux-like)
  mux-view/                   # PURE view materialization + traversal helpers
  mux-proto-tmux/             # PURE tmux protocol types: imsg framing, payload parse/encode, fixtures
  mux-compat-tmux/            # PURE-ish adapters: tmux cmd AST -> core events; core state -> tmux views
```

Notes:

- In vibe-tmux, `mux-proto`, `mux-query`, `mux-view`, `mux-core` are pure. Keep that.
- tmux’s `format.c` and `options-table.c` imply big semantic surfaces. Isolate them in pure crates with heavy tests.

### 4.3 Runtime crates (Plane B)

```
crates/
  mux-runtime/                # tokio runtime services, actors, common IO patterns
  mux-backend/                # backends: tmux-imsg, tmux-control (hints), native
  mux-client-tmux/            # connect to real tmux servers (imsg)
  mux-server-tmux/            # accept real tmux clients (imsg)
  mux-refresh/                # refresh planning, hint streams, state store (vibe-tmux-like)
  mux-os/                     # unsafe + platform glue: unix sockets, SCM_RIGHTS, PTY, close_fds
  mux-telemetry/              # tracing + OTEL helpers (optional but recommended)
```

Notes:

- Keep `mux-os` as the only unsafe crate.
- `mux-refresh` is where stream-first “push hints + stored snapshots” lives.

### 4.4 Facade + API crates (Plane C)

```
crates/
  mux-api/                    # the primary public Rust API: ManagedMux + typed handles
  mux-orm/                    # optional higher-level “ORM-like” API over mux-api views
  mux/                        # top-level re-export crate (like ratatui): stable user entry point
```

Notes:

- `mux-api` should be the “one crate most users depend on”.
- `mux` can re-export `mux-api` plus curated types from pure crates.

### 4.5 Tools (developer + parity)

```
tools/
  tmux-sniff/                 # capture fixtures with SCM_RIGHTS preserved
  format-audit/               # diff tmux format vars vs our implementation
  command-audit/              # diff tmux commands/options vs our registry
  regress-audit/              # track/align tmux regress scripts
  mux-doctor/                 # diagnostics: sockets, perms, env, PTY leak checks
  mux-tui/                    # optional: ratatui front-end (debug UI)
  mux-parity-e2e/             # hermetic parity runner (real tmux vs us)
```

This is effectively what vibe-tmux evolved into; keep it. It’s how you prevent compatibility drift.

### 4.6 Bindings

```
bindings/
  python/                     # pyo3 + maturin; exposes ORM-like API
  node/                       # napi-rs/neon; exposes ORM-like API
  cpp/                        # cxx bridge; modern RAII wrapper
  c-abi/                      # optional; only if you truly need stable ABI
```

If you follow vibe-tmux: drop C ABI, keep cxx for C++.

---

## 5. Key Internal Abstractions

### 5.1 Kernel: Event, Effect, Operation Log

Maintain vibe-tmux’s central purity boundary:

- Input: `Event` (user intent, protocol input, timers as injected events)
- Output: `Effect` (requests for runtime to do IO)

Extend it for transactions/CRDT:

- Introduce an internal **operation** model that is:
  - deterministic
  - serializable
  - stable enough for replication

Suggested model:

- `Op`: smallest state mutation (eg `CreatePane`, `SetActiveWindow`, `SetOption`, `AppendBuffer`)
- `Tx`: a list of `Op` plus metadata
  - `tx_id` (unique)
  - `actor_id` (replica identity)
  - `deps` or causal context (Lamport/Vector clock or dotted version vectors)
- Reducer accepts `Tx` (or `Event` that compiles to `Tx`) and returns:
  - new state
  - emitted `Effect`s
  - emitted `Tx` (for observers/replication)

Why: tmux compatibility wants imperative commands, but CRDT wants stable ops.

### 5.2 Views as the read path

Follow vibe-tmux:

- capture a **view snapshot** (GraphState + indexes)
- serve reads from that snapshot
- wake consumers via **RefreshHint** stream

This is the correct boundary for:

- UI
- bindings
- ORM
- tests

### 5.3 Per-socket actor: `ManagedMux`

Keep the proven facade:

- one long-lived actor per socket
- persistent backend connection(s)
- lock-free view store (readers never block)
- `broadcast` refresh hints

Pattern matches:

- vibe-tmux’s `SocketActor` + `RefreshDriver`
- Zellij’s typed instruction buses and channel fan-in

### 5.4 Compatibility profiles

Treat tmux as one profile among others:

- `Profile`: trait describing semantic defaults/quirks (options, key tables, format vars)
- `tmux::Profile`: implements tmux semantics
- Future: “native profile” (more ergonomic defaults)

Key constraint: the profile must live in pure crates (`mux-options`, `mux-format`, `mux-cmdlang`, `mux-compat-tmux`).

---

## 6. Tmux Compatibility Surface (What Must Be First-Class)

### 6.1 Binary protocol (imsg framing)

From `tmux-protocol.h` and vibe-tmux `mux-proto`:

- native-endian, fixed 16-byte header
- length is total bytes (header + payload) capped (tmux uses 16384)
- FD passing indicated by `IMSG_FD_FLAG` and delivered via `SCM_RIGHTS`

Acceptance criteria:

- fixtures captured by a sniffer/proxy that preserves ancillary data
- robust against chunking/segmentation

### 6.2 Identify burst ordering

Tmux client sends an identify burst (see `client.c` + vibe-tmux notes). Your implementation must:

- match ordering and required frames
- match expectations around stdin/stdout FD markers

### 6.3 Control mode notifications (hints)

From `control-notify.c`:

- Most notifications are IDs-only
- `%layout-change` is special: it contains formatted layout data

Policy (validated by vibe-tmux):

- Control mode is **hints-only** (authoritative state comes from imsg/protocol snapshots)
- Use notifications to compute `RefreshSet` scopes and debounce refresh capture

### 6.4 Command language / config grammar

From `cmd-parse.y`:

- tmux config language includes conditionals and format expansion in parsing

Recommendation:

- Implement a pure AST + parser in `mux-cmdlang`.
- Keep format expansion in `mux-format` and inject it where tmux requires it.

### 6.5 Options table + types

From `options-table.c`:

- options have types, scopes, ranges, defaults, and choice lists

Recommendation:

- Generate a Rust options schema from tmux source (tooling) and compile it into `mux-options`.
- The *data* (tables) can be generated; the semantics (inheritance, scopes) should be pure and well-tested.

### 6.6 Format engine

From `format.c`:

- extensive format modifiers and recursion limits
- jobs are part of formatting (but those “jobs” are runtime-driven)

Recommendation:

- `mux-format` is pure and exposes:
  - `FormatTemplate` parsing
  - `format_expand(ctx, template)`
  - a well-defined “job resolver” interface that is injected (to keep pure)
- `mux-runtime` provides the actual job execution and caches results.

---

## 7. Public Rust API: Layered but Beautiful

Expose two tiers: a low-level façade (stable, explicit) and an ORM-like layer (ergonomic, optional).

### 7.1 Tier 1: `mux-api` (stable façade)

Key types:

- `ManagedMux` (per-socket actor)
- `ManagedMuxHandle` (cloneable, read-only view + subscriptions)
- `ServerFacade` (command execution and transactions)
- `ClientContext` (client-relative semantics)

Read path:

- `handle.view() -> Option<View>` (snapshot)
- `handle.subscribe() -> Receiver<RefreshHint>`

Write path:

- `facade.exec(cmd: Cmd) -> Result<CmdResult>`
- `facade.tx(|t| { ... }) -> Result<TxResult>`

### 7.2 Tier 2: `mux-orm` (libtmux-shaped)

Goal: Rust should feel like libtmux:

- `Server.sessions().filter(...).get(...)`
- `session.windows()`
- `pane.send_keys(...)`
- universal `.cmd(...)` escape hatch

Design:

- ORM objects are thin handles (socket + entity id + cached view reference)
- *All reads* come from a `View` snapshot
- ORM exposes:
  - `QuerySet<T>` wrappers around `mux-query::QuerySpec`
  - `get()/filter()/first()/one_or_none()`

API sketch (Rust):

```rust
use mux::{Server, Query};

let server = Server::connect_default()?; // wraps ManagedMux internally

let session = server
    .sessions()
    .filter(Query::eq("session_name", "demo"))
    .one()?;

let window = session.windows().filter(Query::startswith("window_name", "api")).one()?;
let pane = window.active_pane().unwrap();

pane.send_keys("echo hello", true)?;
let out = pane.capture()?;

// Escape hatch for perfect parity.
server.cmd(["display-message", "-p", "#{pane_id}"])?;
```

Rules:

- ORM must never block on IO in getters.
- ORM must never “refresh by polling” automatically; it consumes hints.

### 7.3 Transactions and CRDT-friendly APIs

Expose transactions as first-class, without requiring CRDT use:

- `server.tx(|tx| { tx.new_window(...); tx.split_pane(...); })`
- `tx.commit() -> TxId`
- Optional:
  - `server.subscribe_ops() -> Stream<Tx>`
  - `server.apply_remote_tx(tx) -> Result<()>`

Even if your first version uses simple linear tx ordering, keep the API shape.

---

## 8. Language Bindings Strategy

Bindings should be thin and map to the same conceptual model.

### 8.1 The stable binding contract

Expose only:

- `ManagedMux` / `Server` connection objects
- view snapshot accessors
- object graph traversal
- query/filter ops as data (`QuerySpec`)
- exec APIs (`cmd`, `exec`, `tx`)

Do not expose:

- internal store/frames/planners
- raw channels
- OS handles

### 8.2 Python (pyo3) and the test framework angle

Copy libtmux’s proven ergonomics:

- context managers for cleanup
- pytest plugin for isolated sockets and server lifetimes

In Rust terms:

- bindings crate depends on `mux-api` + `mux-view` + `mux-query`
- embed a small “test server” harness in Rust and expose it for pytest fixtures

### 8.3 Node

Node should be async-friendly:

- hint subscription surfaces as async iterator or callback stream
- exec surfaces return Promises

Hard rule: no blocking calls on the JS event loop.

### 8.4 C++

If you want C++:

- prefer `cxx` bridge and a modern wrapper
- accept rebuild-per-version; do not promise ABI stability

---

## 9. Testing and Compatibility Gates (Acceptance Criteria)

### 9.1 Pure crates

- `mux-proto-tmux`:
  - fixture-based decode/encode tests
  - chunking robustness (property tests)
  - fd-flag invariants
- `mux-core`:
  - unit tests for reduction logic
  - determinism tests (same event stream => same state)
- `mux-format`, `mux-options`, `mux-cmdlang`:
  - golden tests derived from tmux output
  - fuzz where appropriate (parser)

### 9.2 Integration

- hermetic server/client tests using temp sockets and temp dirs
- regression scripts alignment (tmux regress suite tracked)

### 9.3 Real tmux parity

You need “parity runner” tests that compare:

- tmux vs us (server mode)
- tmux vs us (client mode)

Policy: “works with ourselves” is not success.

### 9.4 Tooling audits (prevent drift)

Codify vibe-tmux’s audit approach:

- `format-audit`: tmux `format.c` variables vs ours
- `command-audit`: tmux commands/options vs ours
- `regress-audit`: track tmux regress scripts and their status

### 9.5 CI gates

- `cargo test` for pure crates and key integration tests
- periodic parity runs (nightly or manual due to env requirements)
- lint gates:
  - deny `unwrap/expect/panic` in critical crates
  - deny unsafe outside `mux-os`

---

## 10. Dos and Don’ts for Implementation (LLM-Friendly)

These are explicit rules an LLM (or human) can follow without “guessing intent”.

### Do

- Keep `mux-core` pure and deterministic.
- Represent external inputs as typed `Event`s.
- Return side effects as typed `Effect`s.
- Store captured state in view snapshots; serve all read APIs from views.
- Push refresh via hint streams; do not poll.
- Back all protocol work with tmux source references or captured fixtures.
- Quarantine unsafe code; document invariants and add tests.
- Keep tmux compat as a profile/adapter layer.

### Don’t

- Don’t add tokio/async to the core.
- Don’t parse protocol by transmuting structs.
- Don’t reconnect/identify “per tick”.
- Don’t leak internal names like “frame/store/planner/json” into public APIs.
- Don’t implement query matching logic separately in each binding.
- Don’t let bindings invent their own refresh loops.
- Don’t mix tmux quirks into the generic multiplexer kernel unless the kernel truly needs them.

---

## 11. Milestone Breakdown (Practical Path)

### M0: Skeleton + guardrails

- workspace created with crate boundaries above
- lints + AGENTS.md rules enforced
- `tmux-sniff` and fixture loader exist early

### M1: tmux protocol peer (client side)

- `mux-proto-tmux` correct framing + identify burst
- `mux-client-tmux` can connect to real tmux and run list commands
- view capture pipeline working; hints optional

### M2: ManagedMux + ORM layer

- per-socket actor with hint stream + lock-free snapshot store
- Rust ORM-like traversal/query
- Python binding that looks like libtmux for common flows

### M3: tmux server mode

- accept tmux clients
- session/window/pane lifecycle parity grows via regress suite
- parity runner becomes the primary signal

### M4: Transactions + CRDT hooks

- Tx API stable
- op log emitted
- “apply remote tx” behind feature flag

---

## 12. Repository Documents to Include (so contributors stay aligned)

- `AGENTS.md`: the non-negotiables (this document’s section 2) plus “never kill real tmux” guard.
- `ARCHITECTURE.md`: as-built snapshot, updated when major changes land.
- `notes/compatibility.md`:
  - protocol authority model
  - which tmux versions are targeted
  - fixture capture process
- `notes/api-design.md`:
  - what is stable in `mux-api`
  - ORM conventions and escape hatches
- `notes/crdt.md`:
  - op log format
  - tx semantics
  - merge/conflict rules

---

## 13. What “Done” Looks Like (Acceptance Criteria)

A future LLM (or team) can declare success when:

- A real tmux client can attach to the Rust server and behave indistinguishably for a large subset of regress tests.
- The Rust client can connect to real tmux servers and capture state reliably with correct FD handling.
- The public Rust API offers:
  - stream-first updates
  - snapshot reads
  - command exec
  - transaction boundaries
  - ORM-like traversal + filtering
- Python/Node bindings expose the same object graph and query semantics as first-class concepts, plus a `.cmd()` escape hatch.
- Compatibility claims are continuously defended by:
  - fixtures
  - format/command/regress audits
  - parity harnesses
- Unsafe remains quarantined and auditable.

