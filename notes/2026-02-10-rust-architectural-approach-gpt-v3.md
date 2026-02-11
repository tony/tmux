# Rust tmux-Compatible Mux in Rust: Layered Architecture North Star (v3)

Date: 2026-02-10

This is a plan-only architecture document for a from-scratch Rust terminal multiplexer that:

- Is a **tmux-compatible peer** (real tmux clients can attach; it can attach to real tmux servers).
- Has full architectural freedom in the kernel (not a 1:1 port), but can run in a strict **tmux compatibility mode**.
- Can be used as a **library**, an **ORM-like API** (libtmux-shaped), a **test framework**, and via **language bindings**.
- Has first-class support for **transactions** and a path to **CRDT replication**.
- Matches the practical constraints and “stream-first” reality proven by `~/work/rust/vibe-tmux/`.

This proposal is grounded in concrete patterns in these repos (read while drafting):

- vibe-tmux
  - Guardrails and acceptance constraints: `~/work/rust/vibe-tmux/AGENTS.md`
  - As-built architecture and crate split: `~/work/rust/vibe-tmux/ARCHITECTURE.md`, `~/work/rust/vibe-tmux/Cargo.toml`
  - Stream-first socket actor: `~/work/rust/vibe-tmux/crates/mux-api/src/managed.rs`
  - Lock-free snapshot reads: `~/work/rust/vibe-tmux/crates/mux-refresh/src/state.rs`
  - Tmux imsg codec and identify burst details: `~/work/rust/vibe-tmux/crates/mux-proto/src/codec.rs`, `~/work/rust/vibe-tmux/crates/mux-proto/src/identify.rs`
  - Rust QueryList semantics: `~/work/rust/vibe-tmux/crates/mux-query/src/lib.rs`
- upstream tmux (C)
  - Protocol message types/version: `~/study/c/tmux/tmux-protocol.h`
  - Client connect + server-start locking: `~/study/c/tmux/client.c` (see `client_connect(...)`, `client_get_lock(...)`)
  - Server socket creation/accept loop entry points: `~/study/c/tmux/server.c` (see `server_create_socket(...)`, `server_start(...)`)
  - Command registry shape: `~/study/c/tmux/cmd.c` (`cmd_table[]`)
  - Config grammar and parse-time format expansion: `~/study/c/tmux/cmd-parse.y`
  - Control-mode notification semantics (hints, plus `%layout-change` data): `~/study/c/tmux/control-notify.c`
  - Format expansion engine structure: `~/study/c/tmux/format.c`
- zellij (Rust mux)
  - Typed instruction enums + thread bus fan-in: `~/study/rust/zellij/zellij-server/src/lib.rs`, `~/study/rust/zellij/zellij-server/src/thread_bus.rs`
- ratatui (workspace modularization)
  - Main crate re-export strategy: `~/study/rust/ratatui/ARCHITECTURE.md`
  - Stable core traits and double-buffering mindset: `~/study/rust/ratatui/ratatui-core/src/backend.rs`, `~/study/rust/ratatui/ratatui-core/src/terminal.rs`
- libtmux (Python ORM)
  - Object model and `.cmd()` scoping: `~/work/python/libtmux/src/libtmux/server.py`, `~/work/python/libtmux/src/libtmux/session.py`, `~/work/python/libtmux/src/libtmux/window.py`, `~/work/python/libtmux/src/libtmux/pane.py`
  - QueryList lookup operators: `~/work/python/libtmux/src/libtmux/_internal/query_list.py`
  - Hermetic pytest fixture patterns and cleanup discipline: `~/work/python/libtmux/src/libtmux/pytest_plugin.py`

---

## 1. Big Decision: New Kernel + tmux Compatibility Profile

### 1.1 Choose: “new mux kernel with perfect tmux profile”

Build a **new multiplexer kernel** (transactional, extensible, testable), and implement tmux as a **compatibility profile**:

- Protocol compatibility (imsg + SCM_RIGHTS + identify burst + message ordering).
- Semantic compatibility (commands/options/format/key tables/config parsing/behavioral quirks).

This matches vibe-tmux’s proven stance that tmux is the external contract, not the internal model. See `~/work/rust/vibe-tmux/ARCHITECTURE.md` (“tmux-compatible peer”, “binary protocol is authoritative”, “control mode hints only”).

### 1.2 Why not “rewrite tmux in Rust first”

A direct port tends to:

- Cement tmux’s incidental coupling into your Rust APIs.
- Make transactions/CRDTs invasive.
- Push platform/protocol concerns into the core.

Instead: keep a deterministic kernel and drive tmux correctness via fixtures + parity harnesses.

---

## 2. North Star Acceptance Criteria

These are the acceptance tests (technical and architectural) that must stay true as the project evolves.

### 2.1 Compatibility acceptance

1. Real tmux clients can attach to our server.
2. Our client can attach to real tmux servers.
3. Protocol parsing/encoding is fixture-backed; no “invented” framing.
4. Control mode is used only as hints (except where tmux actually sends structured data like `%layout-change` in `~/study/c/tmux/control-notify.c`).
5. Command behavior is audit-checked against upstream tmux’s command registry (`~/study/c/tmux/cmd.c`) and regress scripts.

### 2.2 Architectural acceptance

1. **Pure core**: the kernel crate has no tokio/async, no IO, no `nix`/`libc`, no unsafe (vibe-tmux rule in `~/work/rust/vibe-tmux/AGENTS.md`).
2. **Stream-first**: UI/bindings update via streams; no steady-state polling; one persistent connection per socket/backend (vibe-tmux `AGENTS.md` + `SocketActor` in `crates/mux-api/src/managed.rs`).
3. **Unsafe quarantine**: only OS boundary crates contain handwritten `unsafe` (vibe-tmux `AGENTS.md`).
4. **No panics in hot paths**: protocol decode/encode, server loops, PTY loops (vibe-tmux workspace lints in `~/work/rust/vibe-tmux/Cargo.toml`).
5. **Public API hygiene**: bindings do not expose “frame/store/json/planner” in public names; they expose an object graph + exec facade.

### 2.3 Product acceptance

1. Core usage: embedding the mux kernel in-process is supported.
2. ORM usage: “libtmux-but-Rust” usage is ergonomic and layered.
3. Bindings usage: Python/Node/C++ wrappers can be thin and still powerful.
4. Test usage: hermetic servers/sockets/PTY fakes enable deterministic tests.
5. CRDT usage: transactions can be replicated with merge semantics without retrofitting the entire kernel.

---

## 3. Repository Shape and One-Way Dependency Rules

### 3.1 Layers (“the layer cake”)

1. **Kernel (pure):** state graph, typed IDs, reducer, transactions, effect descriptions.
2. **Views + Query (pure):** immutable snapshots and traversal; query DSL matching libtmux semantics.
3. **Compatibility profiles (mostly pure):** tmux semantics and protocol codec.
4. **Runtime (impure):** tokio/tasks/threads, sockets, PTY, SCM_RIGHTS, timers.
5. **Facade API:** managed per-socket actor, view store, refresh hints.
6. **ORM API:** libtmux-shaped object graph layered on top of views + exec.
7. **Bindings:** thin wrappers; no matching logic in foreign languages.
8. **Tools:** sniffers, audits, doctor.

### 3.2 Dependency rule

- Everything points inward.
- Pure crates never depend on runtime/OS/bindings.
- Compatibility adapters depend on kernel+views; kernel never depends on compat.

This is the same principle as vibe-tmux’s “PURE crates” and ratatui’s stable `ratatui-core` + re-export `ratatui` convenience crate (`~/study/rust/ratatui/ARCHITECTURE.md`).

---

## 4. Proposed Cargo Workspace Layout (New Repo)

The goal is to make “doing the right thing” structurally easy.

```
<repo>/
  AGENTS.md
  ARCHITECTURE.md
  Cargo.toml
  crates/
    mux-kernel/              # PURE: ServerGraph + Event->Effect + transactions
    mux-types/               # PURE: typed IDs + newtypes + primitives

    mux-view/                # PURE: snapshots + traversal (ServerHandle-like)
    mux-query/               # PURE: QuerySpec/QueryOp, libtmux-like operators

    mux-compat-tmux/         # PURE-ish: tmux semantics mapping (commands/options/format/keys)
    mux-proto-tmux/          # PURE: imsg codec + control-mode parser

    mux-runtime/             # IMPURE: actor runtime + orchestration
    mux-os/                  # IMPURE: PTY/SCM_RIGHTS + minimal unsafe

    mux-server/              # IMPURE: tmux-compatible server peer
    mux-client/              # IMPURE: tmux-compatible client peer

    mux-api/                 # FACADE: ManagedMux/SocketActor + view store + streams
    mux-orm/                 # FACADE: Server/Session/Window/Pane Rust ORM

    mux-crdt/                # PURE: op log + merge policies + CRDT adapters
    mux-replication/         # IMPURE: transports (optional), auth, persistence

    mux-test-support/        # TEST: fakes, helpers, parity harness

  tools/
    tmux-sniff/              # fixture capture (must preserve SCM_RIGHTS)
    format-audit/
    regress-audit/
    command-audit/
    mux-doctor/

  apps/
    mux/                     # CLI
    mux-tui/                 # ratatui UI

  bindings/
    python/
    node/
    cpp/
```

Notes:

- Mirror vibe-tmux’s practical crate split (see its workspace members in `~/work/rust/vibe-tmux/Cargo.toml`).
- Keep “stable-core” crates as small as possible: `mux-types`, `mux-kernel`, `mux-view`, `mux-query`, `mux-proto-tmux`.

---

## 5. Kernel Design (Pure): Events, Effects, Transactions

### 5.1 State model: `ServerGraph`

Kernel owns the authoritative state graph for:

- servers, sessions, windows, panes
- clients
- jobs
- buffers
- options and environment
- key tables and hooks

Typed IDs are mandatory (vibe-tmux expects typed IDs; see its `AGENTS.md` “Core model expectations”).

### 5.2 Reducer: `Event -> Effects`

Kernel is an event reducer producing effect descriptions (functional core, imperative shell).

- `Event` is an intent or fact.
- `Effect` is a planned side effect the runtime will perform.

Example categories (not exhaustive):

- spawn/kill processes
- create/resize PTYs
- write protocol frames
- update subscriptions / emit notifications
- persist snapshot

### 5.3 Transactions as first-class

Transactions are the unit of composability.

Design constraints:

- A transaction can be derived from:
  - a tmux command (`new-session`, `split-window`, etc.)
  - a “native API” call
  - a replicated CRDT operation batch
- A transaction has:
  - deterministic validation
  - deterministic state changes
  - a deterministic effect list

Kernel API sketch:

```rust
pub struct TxnId(u64);

pub struct Transaction {
  pub id: TxnId,
  pub ops: Vec<KernelOp>,
  pub preconditions: Vec<Precondition>,
}

pub fn apply_transaction(
  state: &mut ServerGraph,
  txn: Transaction,
  ctx: &KernelCtx,
) -> Result<Vec<Effect>, KernelError>;
```

Preconditions are how you later support optimistic concurrency and CRDT conflict policies.

---

## 6. tmux Compatibility Profile

Treat tmux as a compatibility profile spanning:

- binary protocol (imsg)
- control mode notifications (hints)
- commands/options/format/config grammar

### 6.1 Binary protocol (imsg)

Source of truth for message types/version is `~/study/c/tmux/tmux-protocol.h` (`PROTOCOL_VERSION`, `enum msgtype`).

Implementation should follow vibe-tmux’s hardened codec pattern:

- Explicit header/payload parsing with partial read support (`ImsgCodec` in `~/work/rust/vibe-tmux/crates/mux-proto/src/codec.rs`).
- Guardrails:
  - validate lengths
  - cap allocations
  - treat malformed input as protocol violation and close cleanly

### 6.2 Identify burst

The handshake ordering and details matter, and should be asserted via fixtures and cross-checked with tmux source.

- tmux emits a specific burst from `client.c:client_write_identify` (see vibe-tmux’s commentary in `~/work/rust/vibe-tmux/crates/mux-proto/src/identify.rs`).
- The identify burst is where SCM_RIGHTS fd passing semantics appear (stdin/stdout messages).

Acceptance: your tmux profile has a unit test that replays the identify burst expected by real tmux clients and validates decode/encode, plus a real tmux attach test.

### 6.3 Control mode notifications are hints

tmux control notifications are emitted as textual `%%...` lines, and they exist to help clients decide what to refresh.

- `~/study/c/tmux/control-notify.c` shows `%layout-change` includes formatted data (`format_single(...)`) and is emitted when the window is in the client’s session and has a layout root.
- Most other notifications are just IDs (`%window-add @id`, `%pane-mode-changed %id`, etc.).

Design:

- Parse control notifications into a typed enum.
- Map them to refresh scopes.
- Never treat control mode as authoritative for full state (align with vibe-tmux `ARCHITECTURE.md`).

### 6.4 Commands/config grammar/format engine

tmux command semantics are a large surface; layering matters.

- `~/study/c/tmux/cmd.c` has the central `cmd_table[]` registry of commands.
- `~/study/c/tmux/cmd-parse.y` demonstrates config parsing is a real grammar including conditionals and parse-time format expansion.
- `~/study/c/tmux/format.c` shows format expansion is an engine with callbacks and job support.

Proposal:

- `mux-compat-tmux` owns:
  - command registry and argument spec
  - option schema mapping
  - format variable mapping
  - key table mapping
  - config grammar and parse-time expansion rules

Kernel does not know about tmux tokens like `$1`, `@3`, `%7`; the compat layer translates those into typed IDs and operations.

---

## 7. Runtime Architecture (Stream-First, Persistent Connections)

### 7.1 Per-socket actor facade (vibe-tmux proven pattern)

Use the per-socket actor model from vibe-tmux as the default.

- `SocketActor` / `ManagedMux` in `~/work/rust/vibe-tmux/crates/mux-api/src/managed.rs`.
- Event fan-in via a single channel receiving typed `SocketEvent` variants.
- Three cooperating threads/tasks:
  - control receive loop
  - wake/debounce timer
  - driver loop that blocks on `recv()` (no polling)

Acceptance: one persistent connection per socket per backend. No reconnect-per-tick.

### 7.2 Lock-free snapshot store for view reads

UI/bindings should never block on state reads.

- `StateHandle` / `StateStore` / `MutationQueue` in `~/work/rust/vibe-tmux/crates/mux-refresh/src/state.rs` uses a double-buffer snapshot model and serialized writes.

Adopt the same design principle:

- writes are serialized
- reads are lock-free snapshots
- updates publish `RefreshHint`s via broadcast

### 7.3 Typed instruction buses (zellij pattern)

Zellij’s server side uses typed instruction enums and a thread bus with channel fan-in:

- `ServerInstruction` in `~/study/rust/zellij/zellij-server/src/lib.rs`
- `Bus::recv()` fan-in via `Select` in `~/study/rust/zellij/zellij-server/src/thread_bus.rs`

Use this pattern for internal runtime subsystems (PTY loop, server loop, command exec loop, replication loop) when multiple inputs must be multiplexed.

---

## 8. Public Rust API: Facade + ORM (Properly Layered)

Two public faces:

1. **Facade API**: stable, stream-first, “managed connection” entry point.
2. **ORM API**: object graph and QuerySet API built on top of the facade.

### 8.1 Facade API (what most Rust users use)

Design it like vibe-tmux’s `mux-api` crate (`~/work/rust/vibe-tmux/crates/mux-api/src/lib.rs` and `managed.rs`):

- `ManagedMux` / `SocketHandle` for:
  - subscribing to refresh hints
  - obtaining current view snapshot
  - executing commands/transactions

API shape (sketch):

```rust
pub struct ManagedMux;

impl ManagedMux {
  pub fn connect_temporary(socket: &Path) -> Result<Self>;
  pub fn handle(&self) -> MuxHandle;
}

pub struct MuxHandle {
  pub fn subscribe(&self) -> impl Stream<Item = RefreshHint>;
  pub fn snapshot(&self) -> Option<ServerSnapshot>;
  pub fn exec(&self, cmd: TmuxCommand) -> Result<CommandOutput>;
  pub fn begin_txn(&self) -> TxnBuilder;
}
```

### 8.2 ORM API (libtmux, but in Rust)

Target user experience: strongly typed traversal that feels like:

```rust
let server = orm::Server::connect(socket)?;
let session = server.sessions().get(Query::eq("name", "dev"))?;
let pane = session.active_window()?.active_pane()?;

pane.send_keys("ls", Enter)?;

let panes = server.panes().filter(q!("pane_title__icontains" = "ssh"));
```

Grounding:

- libtmux object graph and `.cmd()` scoping:
  - `Server.cmd(...)` in `~/work/python/libtmux/src/libtmux/server.py`
  - `Session.cmd(...)` in `~/work/python/libtmux/src/libtmux/session.py`
  - `Window.cmd(...)` in `~/work/python/libtmux/src/libtmux/window.py`
  - `Pane.cmd(...)` in `~/work/python/libtmux/src/libtmux/pane.py`
- Query operators:
  - libtmux `QueryList.filter()` lookups in `~/work/python/libtmux/src/libtmux/_internal/query_list.py`
  - vibe-tmux’s Rust query ops in `~/work/rust/vibe-tmux/crates/mux-query/src/lib.rs` (`QueryOp::{Exact,IContains,Regex,...}`)

Design rules:

- ORM objects are **views over snapshots** + a handle to the exec facade.
- ORM objects must be cheap to clone and do not own IO.
- Query matching logic lives in Rust (`mux-query`), not in Python/Node.

---

## 9. Language Bindings Strategy

Bindings must be thin and stable.

Rules (mirroring vibe-tmux `AGENTS.md`):

- Bindings expose the object graph + exec facade.
- Bindings must not expose internal store/frame types.
- QuerySet execution and operator semantics are implemented in Rust.

Targets:

- Python: pyo3 + maturin (packaging under `bindings/python/`)
- Node: napi-rs/neon (packaging under `bindings/node/`)
- C++: `cxx` bridge (allow only generated unsafe, like vibe-tmux’s policy in `~/work/rust/vibe-tmux/AGENTS.md`)
- Optional stable C ABI for non-C++ consumers

Testing binding behavior should be driven by:

- embedding into libtmux test suites (vibe-tmux already uses libtmux parity testing per `~/work/rust/vibe-tmux/notes/plan.md`), and
- hermetic binding integration tests.

---

## 10. CRDT + Replication Model (Without Poisoning the Kernel)

CRDT support should not “infect” all APIs, but it must be first-class.

### 10.1 Kernel-level requirement: transactional op log

Every state mutation must be representable as a deterministic op log entry.

- Locally: transactions produce `KernelOp` entries.
- Remotely: CRDT layer receives ops and attempts to merge.

### 10.2 CRDT layer responsibilities

`mux-crdt` owns:

- operation identity (actor id, op id)
- vector clock / causal metadata
- merge policies for conflicting ops
- snapshot materialization rules

### 10.3 Authority modes

Support multiple authority configurations:

1. **Local-authoritative tmux-compat mode**: single writer, tmux semantics, CRDT disabled.
2. **Replicated-authoritative mode**: kernel state derived from CRDT doc; tmux projection is a view.
3. **Hybrid**: tmux commands create transactions that are encoded as CRDT ops; remote merges apply them.

The CRDT transport itself lives outside the kernel (runtime plane).

---

## 11. Test Strategy (Hermetic + Real tmux Parity)

This is non-negotiable if you want “100% compatible” to mean something.

### 11.1 Unit tests (pure crates)

- Kernel: event/transaction semantics, invariants, deterministic reducer tests.
- Proto: codec tests, partial read tests, allocation cap tests.
- Query: operator semantics should match libtmux lookups (`exact`, `icontains`, `regex`, `in`, etc.).

### 11.2 Property tests

- Protocol decoding robustness (random chunking), like vibe-tmux’s “chunking robustness tests” doctrine in `~/work/rust/vibe-tmux/AGENTS.md`.
- Transaction invariants (no dangling references, IDs unique, etc.).

### 11.3 Integration tests (hermetic)

- Start a server on a temp socket.
- Use a fake PTY backend where possible.
- Assert that UI and bindings are stream-driven.

### 11.4 Real tmux parity tests

- Capture fixtures with an sniffer/proxy that preserves SCM_RIGHTS (vibe-tmux tool direction in `~/work/rust/vibe-tmux/Cargo.toml` includes `tools/tmux-sniff`).
- Run regress audits and command audits (vibe-tmux has `tools/regress-audit`, `tools/format-audit`, `tools/tmux-command-audit`).

### 11.5 ORM/bindings parity tests

- Embed Rust-backed bindings into libtmux and run libtmux pytest.
- Follow libtmux’s hermetic fixture discipline (`~/work/python/libtmux/src/libtmux/pytest_plugin.py`): unique socket names, cleanup finalizers.

---

## 12. LLM/Human Guardrails (AGENTS.md Template for the New Repo)

This section is intended to be copied into a new repo’s `AGENTS.md` so an LLM can implement safely.

### 12.1 DO

- Keep `mux-kernel` pure (no tokio/IO/unsafe).
- Add protocol fixtures when changing any protocol behavior.
- Treat upstream tmux C source as the authority for semantics.
- Use stream-first fan-in event loops; block on channels, not ticks.
- Quarantine unsafe in `mux-os`, document invariants with `// SAFETY:`.
- Keep public API names clean: object graph + commands/transactions.

### 12.2 DO NOT

- Do not add polling loops in UI/bindings for state.
- Do not “guess” protocol layouts; verify against `tmux-protocol.h` and fixtures.
- Do not expose internal “frame/store/json” types in public APIs.
- Do not use `unwrap`/`expect`/`panic` in protocol decode/encode, server loops, PTY loops.
- Do not let bindings implement QuerySet matching logic.

### 12.3 Compatibility rule

Any tmux-compat claim must cite:

- upstream tmux source/headers (eg `~/study/c/tmux/tmux-protocol.h`, `client.c`, `control-notify.c`), or
- captured fixtures (including SCM_RIGHTS FDs).

---

## 13. Implementation Steps (In Order)

This is the recommended build order for a greenfield repo.

1. `crates/mux-types/`
   - typed IDs, sizes, paths, newtypes
2. `crates/mux-kernel/`
   - `ServerGraph`, `Event`, `Effect`, basic invariants
   - transaction primitives and op log
3. `crates/mux-proto-tmux/`
   - imsg codec (follow vibe-tmux `ImsgCodec` pattern)
   - identify burst builder (follow vibe-tmux `identify.rs` commentary)
   - control-mode parser + notification enum
4. `crates/mux-view/`
   - snapshot types and traversal (read-only handle API)
5. `crates/mux-query/`
   - implement libtmux-like lookups (align with libtmux `QueryList` and vibe-tmux’s `mux-query`)
6. `crates/mux-os/`
   - PTY allocation + SCM_RIGHTS send/recv
   - strict fd ownership; tests
7. `crates/mux-runtime/`
   - actor system and per-socket actor
   - lock-free snapshot store + refresh hint broadcast (borrow vibe-tmux `StateHandle` approach)
8. `crates/mux-server/` + `crates/mux-client/`
   - tmux peer server/client
   - real tmux attach parity tests
9. `crates/mux-api/`
   - stable facade types; “the” crate most users depend on
10. `crates/mux-orm/`
   - Rust object model mirroring libtmux (`Server/Session/Window/Pane`) with `.cmd()` and `QuerySet`
11. `bindings/*`
   - thin wrappers and parity tests
12. `tools/*`
   - sniff/audit/doctor; wire into CI
13. `apps/*`
   - CLI and TUI are last; they should be thin consumers

---

## 14. Files to Modify / Create (Greenfield Checklist)

In a new repo, these are the first files you should create and evolve:

- `AGENTS.md` (copy guardrails; base it on `~/work/rust/vibe-tmux/AGENTS.md`)
- `ARCHITECTURE.md` (this doc condensed)
- `Cargo.toml` (workspace members, lints like vibe-tmux `~/work/rust/vibe-tmux/Cargo.toml`)

Then create crate skeletons:

- `crates/mux-types/src/lib.rs`
- `crates/mux-kernel/src/lib.rs`
- `crates/mux-proto-tmux/src/lib.rs`
- `crates/mux-view/src/lib.rs`
- `crates/mux-query/src/lib.rs`
- `crates/mux-os/src/lib.rs`
- `crates/mux-runtime/src/lib.rs`
- `crates/mux-server/src/lib.rs`
- `crates/mux-client/src/lib.rs`
- `crates/mux-api/src/lib.rs`
- `crates/mux-orm/src/lib.rs`
- `crates/mux-crdt/src/lib.rs`

Tools/apps/bindings follow.

---

## 15. Primary Risks and Mitigations

1. **Protocol edge cases and SCM_RIGHTS correctness**
   - Mitigation: fixtures and sniffer tooling early; strict fd lifecycle tests.
2. **Behavioral drift from tmux semantics**
   - Mitigation: command/format/regress audits, plus real tmux parity harness.
3. **UI-heavy features (copy-mode, menus, choose-tree)**
   - Mitigation: model them in kernel first; UI later; validate against tmux behavior.
4. **“Compatibility mode” leaks into the kernel**
   - Mitigation: keep tmux-specific tokens/types in `mux-compat-tmux`.
5. **CRDT complexity**
   - Mitigation: start with transactional op log + deterministic replay; keep CRDT optional and isolated.

---

## 16. Bottom Line

If you copy vibe-tmux’s proven constraints (pure core, stream-first actor, hardened protocol codec, lock-free view snapshots) and then layer a libtmux-shaped ORM on top of those stable seams, you can build a tmux-perfect multiplexer that also becomes a general-purpose terminal state engine with transactional/CRDT capabilities.

The critical part is not picking the “perfect” set of crates; it’s enforcing the one-way dependency rules and making correctness measurable via fixtures + parity harnesses from day one.
