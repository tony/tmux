# TermForge v11 Architecture Specification (Gemini v11 Refined)

Date: 2026-02-11
Status: **DEFINITIVE** -- v11 Refined, extending v10 Pass 3 Final.
Lineage: v10 Pass 3 Final -> **v11 Refined** (Gemini).
License: MIT OR Apache-2.0
Rust edition: 2024 (MSRV 1.85)
Protocol target: tmux protocol v8

---

## Preamble

This document is the single authoritative architectural reference for TermForge, a Rust terminal multiplexer with 100% tmux wire-protocol compatibility, ORM-like API, language bindings, CRDT collaboration, a ratatui-based TUI client, and **Termlets** -- SDK-first testing pods that are the killer differentiator.

### v11 Refined Synthesis Notes

This v11 specification builds upon the "v10 Pass 3 Final" definitive spec, addressing remaining subtle gaps and enhancing the Termlet ecosystem for even greater robustness and developer ergonomics.

**v11 Key Improvements:**
1.  **Termlet Environment Inheritance:** Added `inherit_env` to `TermletConfig` (Section 32.2) to support opting into the parent process environment, crucial for certain integration tests.
2.  **SnapshotDiff Context:** Enhanced `SnapshotDiff` (Section 32.11) to support configurable context lines, making visual regression failures easier to debug.
3.  **Async Termlet Pool:** Added `TermletPool::spawn_async` and `get_async` (Section 32.13) to fully support the `async` feature in batch testing scenarios.
4.  **Non-blocking IO Clarification:** Explicitly mandated non-blocking behavior for `RealPtyBackend::read_events` (Section 32) to prevent `wait_for` hangs.
5.  **Resize Safety:** Refined `Termlet::resize` to drain output before *and* after resize to capture immediate SIGWINCH responses.
6.  **LLM Implementation Hints:** Added Section 32.14 providing specific guidance for LLM agents implementing the Termlet system.

---

## 1. Project Identity

### Design Decisions

- Name: TermForge
- CLI binary: `termforge` (primary), `tf` (alias)
- Library name: `termforge` (Python/Node packages)
- Crate prefix: `mux-*` (all Rust crates)
- Config file: `~/.config/termforge/termforge.conf`
- Default socket dir: `$TMPDIR/termforge-$UID/`
- Env prefix: `TERMFORGE_*`
- Alternative names considered: MuxForge (too generic), RustMux (too literal), TermCore (too generic).
- The name must convey: terminal-specific, built/engineered, Rust-powered.
- Compatibility marketing: "100% tmux wire-protocol compatible."

### Rust Example

```rust
pub const PROJECT_NAME: &str = "TermForge";
pub const BINARY_NAME: &str = "termforge";
pub const BINARY_ALIAS: &str = "tf";
pub const SOCKET_PREFIX: &str = "termforge";

pub fn default_socket_dir() -> std::path::PathBuf {
    let uid = unsafe { libc::getuid() };
    std::env::temp_dir().join(format!("{}-{}", SOCKET_PREFIX, uid))
}

pub fn config_path() -> std::path::PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("~/.config"))
        .join("termforge")
        .join("termforge.conf")
}
```

### Test Strategy

1. Binary name: compiled binary is named `termforge`.
2. Alias: `tf` resolves to same binary via `argv[0]` check or symlink.
3. Socket dir: default uses `$TMPDIR/termforge-$UID/`.
4. Config path: respects `XDG_CONFIG_HOME` if set.

### AGENTS.md Rules

- `RULE-S01-01`: All crate names must use the `mux-` prefix.
- `RULE-S01-02`: Binary name `termforge`, alias `tf`.

---

## 2. Acceptance Criteria and Gates

### Design Decisions

- Acceptance gates are the release checklist. Organized into categories: Compatibility (C), Purity (A), Performance (B), Programmability (P), Ecosystem (E).
- Every gate maps to at least one test in the CI matrix.
- Termlet usability gates (P7/P8/P9), performance gates (B10/B11/B12), and ecosystem gates (E6/E7) added.
- Pass/fail thresholds with measurable criteria.

### 2.1 Acceptance Gate Table

| ID | Category | Gate | Test / Evidence | Pass Threshold | Fail Threshold |
|---|---|---|---|---|---|
| C1 | Compatibility | `tmux attach` to TermForge server | Integration test | Attach succeeds, session list matches | Attach fails or session mismatch |
| C2 | Compatibility | `tmux list-sessions` on TermForge | Integration test | Correct session list | Missing or malformed sessions |
| C3 | Compatibility | tmux control mode subscribe/notify | Integration test | Notifications received for session/window/pane events | Missing notifications |
| C4 | Compatibility | tmux config file parsing | Unit test + fixtures | >= 95% of tmux options parsed correctly | < 90% coverage |
| C5 | Compatibility | Copy mode basic operations | Integration test | yank, paste, search work | Any basic operation fails |
| A1 | Purity | `mux-core` compiles without `std::io` | `cargo check --target wasm32-unknown-unknown` | Compiles clean | Any IO import |
| A2 | Purity | `mux-core` has `#![forbid(unsafe_code)]` | CI attribute check | Attribute present | Missing attribute |
| B1 | Performance | VT100 ASCII throughput | Criterion benchmark | > 300 MB/s | < 250 MB/s |
| B2 | Performance | VT100 CSI throughput | Criterion benchmark | > 100 MB/s | < 80 MB/s |
| B3 | Performance | Proto decode throughput | Criterion benchmark | > 500 MB/s | < 400 MB/s |
| B4 | Performance | Layout resize 20 panes | Criterion benchmark | < 50 us | > 100 us |
| B5 | Performance | Snapshot 50 panes | Criterion benchmark | < 1 ms | > 2 ms |
| P1 | Programmability | Python `server.sessions.filter()` | pytest | Returns correct QueryList | Wrong results or exception |
| P2 | Programmability | Python `session.windows.get()` | pytest | Returns single window or raises | Silent failure |
| P3 | Programmability | Node `server.sessions()` | vitest | Returns filtered array | Wrong results or exception |
| P4 | Programmability | OTEL trace propagation | Integration test | Spans exported with correct parent IDs | Missing or broken span chain |
| P5 | Programmability | Control mode notification parsing | Unit test | All notification types parsed | Any type fails |
| P6 | Programmability | CRDT merge convergence | Property test | Replicas converge | Divergence detected |
| P7 | Programmability | Termlet spawn + send_keys + snapshot | Rust unit test | Snapshot contains sent text | Missing text or panic |
| P8 | Programmability | PyTermlet context manager | pytest | Auto-kills on exit, no leaked processes | Process leak or exception |
| P9 | Programmability | Termlet dual-mode (real + fake) | Parametrized test | Both modes produce consistent snapshots for same input | Divergence between modes |
| B10 | Performance | Termlet spawn (FakePty) | Criterion | < 500 us | > 1 ms |
| B11 | Performance | Termlet snapshot (80x24) | Criterion | < 50 us | > 100 us |
| B12 | Performance | Termlet spawn (real PTY) | Criterion | < 50 ms | > 100 ms |
| E1 | Ecosystem | Python package installable via pip | CI | `pip install termforge` succeeds | Install fails |
| E2 | Ecosystem | Node package installable via npm | CI | `npm install termforge` succeeds | Install fails |
| E3 | Ecosystem | `cargo doc` builds without warnings | CI | Zero warnings | Any warning |
| E4 | Ecosystem | MSRV 1.85 compiles | CI | `cargo +1.85 check` passes | Compile failure |
| E5 | Ecosystem | Clippy clean on nightly | CI | Zero warnings | Any warning |
| E6 | Ecosystem | Termlet documented in README | Manual | Usage example present | No mention |
| E7 | Ecosystem | Termlet cross-language parity | Multi-runner | Same scenario, same snapshot across Rust/Python/Node | Snapshot divergence |

### Rust Example

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gate {
    // Compatibility
    C1WireAttach,
    C2ListSessions,
    C3ControlMode,
    C4ConfigParse,
    C5CopyMode,
    // Purity
    A1NoPureIo,
    A2ForbidUnsafe,
    // Performance
    B1Vt100Ascii,
    B2Vt100Csi,
    B3ProtoDecode,
    B4LayoutResize,
    B5Snapshot50,
    // Programmability
    P1PyFilter,
    P2PyGet,
    P3NodeFilter,
    P4OtelTrace,
    P5ControlParse,
    P6CrdtMerge,
    P7TermletSpawn,
    P8PyTermletCtxMgr,
    P9TermletDualMode,
    // Performance (Termlet)
    B10TermletSpawnFake,
    B11TermletSnapshot,
    B12TermletSpawnReal,
    // Ecosystem
    E1PyPip,
    E2NodeNpm,
    E3CargoDoc,
    E4Msrv,
    E5Clippy,
    E6TermletDocs,
    E7TermletCrossLang,
}

impl Gate {
    pub fn is_required_for_release(&self) -> bool {
        // All gates are release-blocking except performance warns
        !matches!(self,
            Gate::B1Vt100Ascii | Gate::B2Vt100Csi | Gate::B3ProtoDecode |
            Gate::B4LayoutResize | Gate::B5Snapshot50 |
            Gate::B10TermletSpawnFake | Gate::B11TermletSnapshot | Gate::B12TermletSpawnReal
        )
    }
}
```

### Test Strategy

1. Gate checklist run on every release candidate.
2. Performance gates warn but do not block (except regressions > 30%).
3. All functional gates must pass.
4. Termlet gates P7/P8/P9 and E6/E7 mandatory for release.

### AGENTS.md Rules

- `RULE-S02-01`: Every new feature must map to at least one acceptance gate.
- `RULE-S02-02`: No gate may be removed without spec amendment and approval.
- `RULE-S02-03`: Gate pass/fail thresholds must be measurable, not subjective.

---

## 3. Crate Dependency Rules

### Design Decisions

- Strict layered architecture: Layer 0 (pure) -> Layer 1 (OS) -> Layer 2 (facade) -> Layer 3 (application).
- Layer 0 crates must compile to WASM (`wasm32-unknown-unknown`).
- No circular dependencies allowed.
- `cargo deny check` enforced in CI.
- `mux-termlet` added at Layer 2, depends on Layer 0 + Layer 1 only.

### 3.1 Crate Dependency Matrix

| Crate | Layer | May Depend On | Must Not Depend On |
|---|---|---|---|
| `mux-types` | 0 | std | anything else |
| `mux-core` | 0 | mux-types | tokio, libc, mux-pty, mux-os, mux-server |
| `mux-grid` | 0 | mux-types | tokio, libc |
| `mux-proto` | 0 | mux-types | tokio (codec trait is generic) |
| `mux-query` | 0 | mux-types, mux-core | tokio, libc |
| `mux-conf` | 0 | mux-types | tokio, libc |
| `mux-crdt` | 0 | mux-types, mux-core | tokio, libc, network |
| `mux-pty-fake` | 0 | mux-types, mux-pty (trait only) | tokio, libc (real) |
| `mux-pty` | 1 | mux-types, libc | mux-core, mux-server |
| `mux-os` | 1 | mux-types, libc | mux-core, mux-server |
| `mux-control` | 1 | mux-types, mux-core | mux-server |
| `mux-otel` | 1 | opentelemetry, tokio | mux-core, mux-server |
| `mux-termlet` | **2** | mux-core, mux-grid, mux-pty, mux-pty-fake, mux-types, tokio (optional) | **mux-server, mux-api, mux-orm** |
| `mux-orm` | 2 | mux-core, mux-query, mux-types | mux-server |
| `mux-api` | 2 | mux-core, mux-orm, mux-types | mux-server (impl) |
| `mux-view` | 2 | mux-core, mux-types, ratatui | mux-server |
| `mux-test-support` | 2 | mux-types, mux-pty, mux-os | mux-server |
| `mux-server` | 3 | all above + tokio | (top level) |
| `mux-bench` | 3 | all above + criterion | (top level) |
| `termforge` (bin) | 3 | all above | (top level) |

### Rust Example

```rust
// Dependency enforcement via cargo-deny
// deny.toml
[bans]
multiple-versions = "deny"

// CI script
// scripts/check-layers.sh
#!/bin/bash
set -euo pipefail

# Layer 0: must compile to WASM
for crate in mux-types mux-core mux-grid mux-proto mux-query mux-conf mux-crdt; do
    cargo check -p "$crate" --target wasm32-unknown-unknown
done

# mux-termlet must not depend on mux-server, mux-api, mux-orm
for forbidden in mux-server mux-api mux-orm; do
    if cargo tree -p mux-termlet | grep -q "$forbidden"; then
        echo "ERROR: mux-termlet depends on $forbidden"
        exit 1
    fi
done
```

### Test Strategy

1. WASM check: Layer 0 crates compile to `wasm32-unknown-unknown`.
2. Dependency audit: `cargo deny check` passes in CI.
3. Layer violation: `cargo tree` verifies no forbidden dependencies.
4. `mux-termlet` isolation: does not depend on `mux-server`, `mux-api`, or `mux-orm`.

### AGENTS.md Rules

- `RULE-S03-01`: Layer 0 crates must pass WASM compilation check.
- `RULE-S03-02`: No circular dependencies. `cargo deny check` enforced.
- `RULE-S03-03`: `mux-termlet` must not depend on `mux-server` or `mux-api`.

---

## 4. Workspace Layout

### Design Decisions

- Cargo workspace with `crates/`, `tools/`, `bindings/`, and `fuzz/` directories.
- Each crate has a focused responsibility.
- `mux-termlet` crate added at Layer 2 (FACADE).
- Tools are separate binaries that may depend on any crate.

### 4.1 Directory Structure

```
termforge/
  Cargo.toml                    # workspace root
  AGENTS.md                     # AI agent rules
  deny.toml                     # cargo-deny config
  crates/
    mux-types/                  # Layer 0: shared types, IDs
    mux-core/                   # Layer 0: state graph, events, effects
    mux-grid/                   # Layer 0: terminal grid, VT100 parser
    mux-proto/                  # Layer 0: wire protocol codec
    mux-query/                  # Layer 0: ORM query operators
    mux-conf/                   # Layer 0: config parser
    mux-crdt/                   # Layer 0: CRDT types (HLC, LWW, OR-Set)
    mux-pty/                    # Layer 1: real PTY backend
    mux-pty-fake/               # Layer 0: fake PTY for testing
    mux-os/                     # Layer 1: OS abstractions (sockets, signals)
    mux-control/                # Layer 1: control mode parser
    mux-otel/                   # Layer 1: OpenTelemetry
    mux-termlet/                # Layer 2: Termlet SDK
    mux-orm/                    # Layer 2: ORM facade
    mux-api/                    # Layer 2: public API (ManagedMux, StateHandle)
    mux-view/                   # Layer 2: TUI ViewModel
    mux-test-support/           # Layer 2: test harness utilities
    mux-server/                 # Layer 3: tokio runtime, actor system
    mux-bench/                  # Layer 3: benchmarks
  tools/
    tmux-builder/               # Build + cache tmux from source
    tmux-command-audit/         # Command coverage tracking
    format-audit/               # Format string parity checker
    mux-regress/                # Parity regression runner
    mux-tui/                    # TUI binary
  bindings/
    python/                     # PyO3 bindings
    node/                       # Neon bindings
  fuzz/
    fuzz_targets/
      proto_decode.rs
      vt100_parse.rs
      config_parse.rs
```

### Rust Example

```rust
// Cargo.toml (workspace root)
[workspace]
resolver = "2"
members = [
    "crates/mux-types",
    "crates/mux-core",
    "crates/mux-grid",
    "crates/mux-proto",
    "crates/mux-query",
    "crates/mux-conf",
    "crates/mux-crdt",
    "crates/mux-pty",
    "crates/mux-pty-fake",
    "crates/mux-os",
    "crates/mux-control",
    "crates/mux-otel",
    "crates/mux-termlet",
    "crates/mux-orm",
    "crates/mux-api",
    "crates/mux-view",
    "crates/mux-test-support",
    "crates/mux-server",
    "crates/mux-bench",
    "tools/tmux-builder",
    "tools/tmux-command-audit",
    "tools/format-audit",
    "tools/mux-regress",
    "tools/mux-tui",
]

[workspace.dependencies]
slotmap = "1"
thiserror = "2"
bytes = "1"
regex = "1"
blake3 = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
insta = "1"
proptest = "1"
criterion = "0.5"
ratatui = "0.29"
crossterm = "0.28"
pyo3 = "0.23"
neon = "1"
```

### Test Strategy

1. Workspace compiles: `cargo build --workspace` succeeds.
2. Individual crate builds: each crate compiles independently.
3. `mux-termlet` compiles and links against mux-core, mux-grid, mux-pty, mux-pty-fake.
4. No duplicate workspace dependencies.

### AGENTS.md Rules

- `RULE-S04-01`: Every new crate must be added to workspace members.
- `RULE-S04-02`: Shared dependencies must use `workspace.dependencies`.
- `RULE-S04-03`: `mux-termlet` lives at Layer 2, depends on Layer 0 + Layer 1 only.

---

## 5. Core State Graph

### Design Decisions

- `ServerGraph` is the single mutable state container. Only the state actor may mutate it.
- `GraphState` is an immutable snapshot shared via `ArcSwap` for zero-contention reads.
- Entity IDs via `slotmap::new_key_type!` for type-safe, generationally-indexed references.
- Reverse maps (pane-to-window, window-to-session) computed during snapshot construction.
- `CoreCtx` provides injectable time and randomness for deterministic testing.
- `apply_event()` is the single entry point for all state mutations.
- `mux-core` is pure: `#![forbid(unsafe_code)]`, no IO, no tokio, no libc.
- `MuxKernel` trait as alternative naming for the core dispatch interface. The `ServerGraph` struct implements `MuxKernel`.

### Rust Example

```rust
// crates/mux-core/src/lib.rs
#![forbid(unsafe_code)]

use slotmap::SlotMap;

slotmap::new_key_type! {
    pub struct SessionId;
    pub struct WindowId;
    pub struct PaneId;
    pub struct ClientId;
    pub struct JobId;
    pub struct BufferId;
}

pub struct ServerGraph {
    pub sessions: SlotMap<SessionId, Session>,
    pub windows: SlotMap<WindowId, Window>,
    pub panes: SlotMap<PaneId, Pane>,
    pub clients: SlotMap<ClientId, Client>,
    pub jobs: SlotMap<JobId, Job>,
    pub buffers: SlotMap<BufferId, Buffer>,
    pub server_options: OptionSet,
    pub global_options: OptionSet,
}

pub struct Session {
    pub name: String,
    pub windows: Vec<WindowId>,
    pub active_window: Option<WindowId>,
    pub options: OptionSet,
    pub created: i64,
}

pub struct Window {
    pub name: String,
    pub panes: Vec<PaneId>,
    pub active_pane: Option<PaneId>,
    pub layout: LayoutTree,
    pub options: OptionSet,
}

pub struct Pane {
    pub command: Vec<String>,
    pub cwd: String,
    pub size: PaneSize,
    pub options: OptionSet,
}

#[derive(Debug, Clone, Copy)]
pub struct PaneSize {
    pub sx: u16,
    pub sy: u16,
}

/// Immutable snapshot for zero-contention reads via ArcSwap.
#[derive(Clone)]
pub struct GraphState {
    pub sessions: Vec<(SessionId, Session)>,
    pub windows: Vec<(WindowId, Window)>,
    pub panes: Vec<(PaneId, Pane)>,
    pub clients: Vec<(ClientId, Client)>,
    // Reverse maps computed during snapshot construction
    pub pane_to_window: std::collections::HashMap<PaneId, WindowId>,
    pub window_to_session: std::collections::HashMap<WindowId, SessionId>,
}

/// Injectable context for deterministic testing.
pub struct CoreCtx {
    pub now: Box<dyn Fn() -> i64>,
    pub random: Box<dyn Fn() -> u64>,
}

/// MuxKernel trait -- alternative naming for core dispatch.
/// ServerGraph implements this trait.
pub trait MuxKernel {
    fn apply_event(&mut self, event: Event, ctx: &CoreCtx) -> ApplyOutcome;
    fn snapshot(&self) -> GraphState;
}

impl MuxKernel for ServerGraph {
    fn apply_event(&mut self, event: Event, ctx: &CoreCtx) -> ApplyOutcome {
        // Single entry point for all state mutations.
        // Returns effects to be executed by the runtime.
        match event {
            Event::CreateSession { name } => {
                let id = self.sessions.insert(Session {
                    name,
                    windows: vec![],
                    active_window: None,
                    options: OptionSet::new(),
                    created: (ctx.now)(),
                });
                ApplyOutcome {
                    effects: vec![Effect::Notify(Notification::SessionCreated(id))],
                    ..Default::default()
                }
            }
            // ... other event handlers
            _ => ApplyOutcome::default(),
        }
    }

    fn snapshot(&self) -> GraphState {
        // Construct immutable snapshot with reverse maps.
        let mut pane_to_window = std::collections::HashMap::new();
        let mut window_to_session = std::collections::HashMap::new();
        for (sid, session) in &self.sessions {
            for &wid in &session.windows {
                window_to_session.insert(wid, sid);
                if let Some(window) = self.windows.get(wid) {
                    for &pid in &window.panes {
                        pane_to_window.insert(pid, wid);
                    }
                }
            }
        }
        GraphState {
            sessions: self.sessions.iter().map(|(k, v)| (k, v.clone())).collect(),
            windows: self.windows.iter().map(|(k, v)| (k, v.clone())).collect(),
            panes: self.panes.iter().map(|(k, v)| (k, v.clone())).collect(),
            clients: self.clients.iter().map(|(k, v)| (k, v.clone())).collect(),
            pane_to_window,
            window_to_session,
        }
    }
}
```

### Test Strategy

1. Create session: `apply_event(CreateSession)` adds session to graph.
2. Destroy session: `apply_event(DestroySession)` removes session and its windows/panes.
3. Snapshot isolation: mutating graph after snapshot does not affect snapshot.
4. Reverse maps: `pane_to_window` and `window_to_session` correct after snapshot.
5. ID generation: SessionId, WindowId, PaneId are unique across insertions.
6. CoreCtx injection: deterministic time/random via custom closures.
7. Pure crate: `cargo check --target wasm32-unknown-unknown` passes for `mux-core`.
8. MuxKernel trait: `ServerGraph` implements `MuxKernel` correctly.

### AGENTS.md Rules

- `RULE-S05-01`: `mux-core` is pure: no IO, no `unsafe`, no `tokio`, no `libc`. Enforced via `#![forbid(unsafe_code)]` and WASM CI check.
- `RULE-S05-02`: WASM CI check for all Layer 0 crates.

---

## 6. Entity Relationships

### Design Decisions

- `Vec<ChildId>` inside parent entity for parent-to-children relationships.
- Reverse maps (`pane_to_window`, `window_to_session`) computed during `GraphState` snapshot construction.
- No `SecondaryMap`. Reverse lookups are derived, not stored.
- `active_window` and `active_pane` stored as `Option<Id>` on the parent.
- Client-to-session attachment stored on the `Client` struct.

### Rust Example

```rust
// Entity relationship traversal
impl GraphState {
    pub fn session_windows(&self, sid: SessionId) -> Vec<&Window> {
        self.sessions.iter()
            .find(|(id, _)| *id == sid)
            .map(|(_, s)| {
                s.windows.iter()
                    .filter_map(|wid| self.windows.iter().find(|(id, _)| id == wid).map(|(_, w)| w))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn pane_session(&self, pid: PaneId) -> Option<SessionId> {
        self.pane_to_window.get(&pid)
            .and_then(|wid| self.window_to_session.get(wid))
            .copied()
    }

    pub fn window_panes(&self, wid: WindowId) -> Vec<&Pane> {
        self.windows.iter()
            .find(|(id, _)| *id == wid)
            .map(|(_, w)| {
                w.panes.iter()
                    .filter_map(|pid| self.panes.iter().find(|(id, _)| id == pid).map(|(_, p)| p))
                    .collect()
            })
            .unwrap_or_default()
    }
}
```

### Test Strategy

1. Session has windows: creating windows adds to session's `windows` vec.
2. Window has panes: creating panes adds to window's `panes` vec.
3. Reverse map correctness: `pane_to_window` matches actual graph structure.
4. Active tracking: `active_window` and `active_pane` updated on focus changes.
5. Orphan prevention: destroying a session destroys all its windows and panes.
6. Client attachment: client tracks its attached session.

### AGENTS.md Rules

- `RULE-S06-01`: All entity IDs via `slotmap::new_key_type!`. No raw integers for entity references.

---

## 7. Event and Effect Enums

### Design Decisions

- Events are inbound state change requests. Effects are outbound side-effect requests.
- The core emits effects; only the runtime executes them.
- All state transitions go through `apply_event()`.
- `ApplyOutcome` carries effects and optional hints for the runtime.
- Events and Effects are exhaustive enums -- no extensible variant pattern.

### Rust Example

```rust
// crates/mux-core/src/event.rs
#[derive(Debug, Clone)]
pub enum Event {
    CreateSession { name: String },
    DestroySession { id: SessionId },
    CreateWindow { session: SessionId, name: String },
    DestroyWindow { id: WindowId },
    CreatePane { window: WindowId, command: Vec<String>, cwd: String },
    DestroyPane { id: PaneId },
    ResizePane { id: PaneId, size: PaneSize },
    AttachClient { client: ClientId, session: SessionId },
    DetachClient { client: ClientId },
    SetOption { scope: OptionScope, key: String, value: OptionValue },
    UnsetOption { scope: OptionScope, key: String },
    PaneOutput { pane: PaneId, data: Vec<u8> },
    PaneExited { pane: PaneId, status: i32 },
    ClientIdentified { client: ClientId },
    LayoutResize { window: WindowId, delta: i32, horizontal: bool },
}

// crates/mux-core/src/effect.rs
#[derive(Debug, Clone)]
pub enum Effect {
    SpawnProcess { pane: PaneId, command: Vec<String>, cwd: String, size: PaneSize },
    KillProcess { pane: PaneId, signal: i32 },
    WritePty { pane: PaneId, data: Vec<u8> },
    ResizePty { pane: PaneId, size: PaneSize },
    Notify(Notification),
    LoadConfig { path: std::path::PathBuf },
    SendControlNotification { client: ClientId, notification: ControlNotification },
}

#[derive(Debug, Clone)]
pub enum Notification {
    SessionCreated(SessionId),
    SessionDestroyed(SessionId),
    WindowCreated(WindowId),
    WindowDestroyed(WindowId),
    PaneCreated(PaneId),
    PaneDestroyed(PaneId),
    LayoutChanged(WindowId),
    ClientAttached(ClientId),
    ClientDetached(ClientId),
}

#[derive(Debug, Clone, Default)]
pub struct ApplyOutcome {
    pub effects: Vec<Effect>,
    pub hints: Vec<String>,
}
```

### Test Strategy

1. CreateSession -> SessionCreated notification effect.
2. DestroySession -> cascading DestroyWindow + DestroyPane events.
3. PaneOutput -> no effects (grid update is internal).
4. ResizePane -> ResizePty effect.
5. LayoutResize -> multiple ResizePty effects for affected panes.
6. AttachClient -> ClientAttached notification + LoadConfig on first client.
7. SetOption -> no effects (pure state mutation).
8. Effect serialization: all Effect variants are Debug + Clone.

### AGENTS.md Rules

- `RULE-S07-01`: Core emits effects; only the runtime executes effects.
- `RULE-S07-02`: All state transitions go through `apply_event()`. No direct graph mutation.

---

## 8. Error Handling

### Design Decisions

- `thiserror` for all library crate errors.
- `anyhow` only in binaries and tests.
- Every error has an `ErrorClass` for categorization.
- ErrorClass drives retry/recovery logic in the runtime.
- `TermletError` added with 6 variants including `PatternNotFound` and `AlreadyKilled`.
- Stable error code strings for binding interop (`TERMLET_TIMEOUT`, etc.).
- `ErrorCode` trait for consistent string code extraction across all error types.

### Rust Example

```rust
// crates/mux-core/src/error.rs
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorClass {
    UserError,
    Bug,
    Transient,
    ProtocolViolation,
}

pub trait Classified {
    fn class(&self) -> ErrorClass;
}

/// ErrorCode trait for stable string extraction across error types.
pub trait ErrorCode {
    fn error_code(&self) -> &'static str;
}

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("session not found: {id:?}")]
    SessionNotFound { id: SessionId },
    #[error("window not found: {id:?}")]
    WindowNotFound { id: WindowId },
    #[error("pane not found: {id:?}")]
    PaneNotFound { id: PaneId },
    #[error("duplicate session name: {name}")]
    DuplicateSessionName { name: String },
    #[error("layout invariant violated: {reason}")]
    LayoutBug { reason: String },
}

impl Classified for CoreError {
    fn class(&self) -> ErrorClass {
        match self {
            Self::SessionNotFound { .. } |
            Self::WindowNotFound { .. } |
            Self::PaneNotFound { .. } |
            Self::DuplicateSessionName { .. } => ErrorClass::UserError,
            Self::LayoutBug { .. } => ErrorClass::Bug,
        }
    }
}

// crates/mux-proto/src/error.rs
#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("unknown message type: {msg_type}")]
    UnknownMsgType { msg_type: u32 },
    #[error("payload too short: expected {expected}, got {actual}")]
    PayloadTooShort { expected: usize, actual: usize },
    #[error("invalid UTF-8 in payload")]
    InvalidUtf8,
}

impl Classified for ProtocolError {
    fn class(&self) -> ErrorClass {
        ErrorClass::ProtocolViolation
    }
}

// crates/mux-termlet/src/error.rs
#[derive(Debug, Error)]
pub enum TermletError {
    #[error("spawn failed: {reason}")]
    SpawnFailed { reason: String },

    #[error("termlet already killed")]
    AlreadyKilled,

    #[error("wait_for timed out: pattern={pattern:?} timeout={timeout_ms}ms")]
    WaitForTimeout { pattern: String, timeout_ms: u64 },

    /// Process exited before pattern appeared.
    /// Distinct from WaitForTimeout: the process died, not the clock.
    #[error("pattern not found (process exited): pattern={pattern:?}")]
    PatternNotFound { pattern: String },

    #[error("resize failed: {0}")]
    ResizeFailed(String),

    #[error("PTY error: {0}")]
    Pty(#[from] PtyError),
}

impl Classified for TermletError {
    fn class(&self) -> ErrorClass {
        match self {
            Self::SpawnFailed { .. } => ErrorClass::Transient,
            Self::AlreadyKilled => ErrorClass::UserError,
            Self::WaitForTimeout { .. } => ErrorClass::Transient,
            Self::PatternNotFound { .. } => ErrorClass::UserError,
            Self::ResizeFailed(_) => ErrorClass::Transient,
            Self::Pty(_) => ErrorClass::Transient,
        }
    }
}

/// Stable error code strings for binding interop.
impl ErrorCode for TermletError {
    fn error_code(&self) -> &'static str {
        match self {
            Self::SpawnFailed { .. } => "TERMLET_SPAWN_ERROR",
            Self::AlreadyKilled => "TERMLET_ALREADY_KILLED",
            Self::WaitForTimeout { .. } => "TERMLET_TIMEOUT",
            Self::PatternNotFound { .. } => "TERMLET_PATTERN_NOT_FOUND",
            Self::ResizeFailed(_) => "TERMLET_RESIZE_ERROR",
            Self::Pty(_) => "TERMLET_PTY_ERROR",
        }
    }
}
```

### Test Strategy

1. Error classification: every variant maps to correct ErrorClass.
2. Display format: error messages are human-readable.
3. From conversion: `PtyError` converts to `TermletError::Pty`.
4. TermletError variants: all 6 variants constructed and displayed.
5. Error codes: every variant has a stable string code.
6. PatternNotFound vs WaitForTimeout: distinct error paths tested.
7. ErrorCode trait: implemented for all error types that cross binding boundaries.

### AGENTS.md Rules

- `RULE-S08-01`: `thiserror` for library errors, `anyhow` only in binaries/tests.
- `RULE-S08-02`: Every `ErrorClass` variant tested.
- `RULE-S08-03`: Termlet error codes must be stable string constants.

---

## 9. Wire Protocol

### Design Decisions

- tmux protocol v8 (`tmux-protocol.h:23`).
- `ImsgHdr` (12 bytes) + payload. `IMSG_HDR_SIZE = 12`.
- `ImsgCodec` implements tokio-codec-compatible encode/decode (but the trait is generic, not tokio-specific).
- `DecodeOutcome` distinguishes Ok, NeedMore, and Fatal.
- Protocol violations kill the connection (never drop-and-continue), matching `server-client.c:3472-3475`.
- Codec is stateful: tracks partial frame state across calls.

### Rust Example

```rust
// crates/mux-proto/src/codec.rs
use bytes::{Buf, BufMut, BytesMut};

pub const IMSG_HDR_SIZE: usize = 12;

#[derive(Debug, Clone)]
pub struct ImsgHdr {
    pub msg_type: u32,
    pub len: u32,
    pub peerid: u16,
    pub pid: u16,
    pub has_fd: bool,
}

#[derive(Debug, Clone)]
pub struct ImsgFrame {
    pub header: ImsgHdr,
    pub payload: bytes::Bytes,
}

pub enum DecodeOutcome {
    Ok(ImsgFrame),
    NeedMore,
    Fatal(ProtocolError),
}

pub struct ImsgCodec {
    state: CodecState,
}

enum CodecState {
    Header,
    Payload { header: ImsgHdr, remaining: usize },
}

impl ImsgCodec {
    pub fn new() -> Self {
        Self { state: CodecState::Header }
    }

    pub fn decode(&mut self, buf: &mut BytesMut) -> DecodeOutcome {
        loop {
            match &self.state {
                CodecState::Header => {
                    if buf.len() < IMSG_HDR_SIZE {
                        return DecodeOutcome::NeedMore;
                    }
                    let msg_type = buf.get_u32_le();
                    let len = buf.get_u32_le();
                    let peerid = buf.get_u16_le();
                    let pid_and_fd = buf.get_u16_le();
                    let pid = pid_and_fd & 0x7FFF;
                    let has_fd = (pid_and_fd & 0x8000) != 0;

                    if (len as usize) < IMSG_HDR_SIZE {
                        return DecodeOutcome::Fatal(ProtocolError::PayloadTooShort {
                            expected: IMSG_HDR_SIZE,
                            actual: len as usize,
                        });
                    }

                    let payload_len = len as usize - IMSG_HDR_SIZE;
                    let header = ImsgHdr { msg_type, len, peerid, pid, has_fd };
                    self.state = CodecState::Payload { header, remaining: payload_len };
                }
                CodecState::Payload { remaining, .. } => {
                    if buf.len() < *remaining {
                        return DecodeOutcome::NeedMore;
                    }
                    let remaining = *remaining;
                    if let CodecState::Payload { header, .. } =
                        std::mem::replace(&mut self.state, CodecState::Header)
                    {
                        let payload = buf.split_to(remaining).freeze();
                        return DecodeOutcome::Ok(ImsgFrame { header, payload });
                    }
                }
            }
        }
    }

    pub fn encode(&self, frame: &ImsgFrame, buf: &mut BytesMut) {
        buf.put_u32_le(frame.header.msg_type);
        buf.put_u32_le(frame.header.len);
        buf.put_u16_le(frame.header.peerid);
        let pid_and_fd = frame.header.pid | if frame.header.has_fd { 0x8000 } else { 0 };
        buf.put_u16_le(pid_and_fd);
        buf.put_slice(&frame.payload);
    }
}
```

### Test Strategy

1. Roundtrip: encode -> decode produces identical frame.
2. Partial header: 6 bytes -> NeedMore.
3. Partial payload: header complete but payload incomplete -> NeedMore.
4. Invalid length: len < IMSG_HDR_SIZE -> Fatal.
5. All message types: MsgType enum covers known tmux message types.
6. FD flag: has_fd bit correctly set/cleared in pid field.
7. proptest: random payloads roundtrip.
8. Fuzz: random bytes do not panic.
9. Multi-frame: buffer with two frames decodes both.

### AGENTS.md Rules

- `RULE-S09-01`: Protocol violations kill the connection, never drop-and-continue.

---

## 10. Configuration Engine

### Design Decisions

- Config loads only after first client identify burst (`server-client.c:3725-3734`).
- `OptionSet` is a key-value store with typed values.
- Option resolution chain: Pane -> Window -> Session -> Server (FALLTHROUGH per `options.c:891-903`).
- `unset` removes override, falls through to parent scope (matching `options.c:1269-1285`).
- Config parser handles tmux-compatible syntax: `set-option`, `bind-key`, `source-file`, etc.
- Four option scopes: Server, Session, Window, Pane (WindowPane falls through to Window).

### Rust Example

```rust
// crates/mux-types/src/options.rs
#[derive(Debug, Clone, PartialEq)]
pub enum OptionValue {
    String(String),
    Int(i64),
    Bool(bool),
    Color(Color),
    Style(Style),
    Array(Vec<String>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionScope {
    Server,
    Session,
    Window,
    Pane, // WindowPane -- falls through to Window
}

#[derive(Debug, Clone, Default)]
pub struct OptionSet {
    values: std::collections::HashMap<String, OptionValue>,
}

impl OptionSet {
    pub fn get(&self, key: &str) -> Option<&OptionValue> {
        self.values.get(key)
    }

    pub fn set(&mut self, key: String, value: OptionValue) {
        self.values.insert(key, value);
    }

    pub fn unset(&mut self, key: &str) -> bool {
        self.values.remove(key).is_some()
    }
}

// Option resolution with FALLTHROUGH (options.c:891-903)
pub fn resolve_option(
    key: &str,
    pane_opts: &OptionSet,
    window_opts: &OptionSet,
    session_opts: &OptionSet,
    server_opts: &OptionSet,
) -> Option<OptionValue> {
    pane_opts.get(key)
        .or_else(|| window_opts.get(key))   // Pane FALLTHROUGH to Window
        .or_else(|| session_opts.get(key))
        .or_else(|| server_opts.get(key))
        .cloned()
}

// crates/mux-conf/src/parser.rs
pub struct ConfigParser;

impl ConfigParser {
    pub fn parse(input: &str) -> Result<Vec<ConfigDirective>, ConfigError> {
        let mut directives = Vec::new();
        for (line_no, line) in input.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let directive = Self::parse_line(line, line_no + 1)?;
            directives.push(directive);
        }
        Ok(directives)
    }

    fn parse_line(line: &str, line_no: usize) -> Result<ConfigDirective, ConfigError> {
        let parts: Vec<&str> = line.splitn(3, ' ').collect();
        match parts.first().copied() {
            Some("set-option" | "set") => Self::parse_set_option(&parts[1..], line_no),
            Some("bind-key" | "bind") => Self::parse_bind_key(&parts[1..], line_no),
            Some("unbind-key" | "unbind") => Self::parse_unbind_key(&parts[1..], line_no),
            Some("source-file" | "source") => Self::parse_source_file(&parts[1..], line_no),
            Some(cmd) => Err(ConfigError::UnknownCommand {
                command: cmd.to_string(), line: line_no,
            }),
            None => Err(ConfigError::EmptyLine { line: line_no }),
        }
    }
}
```

### Test Strategy

1. Option set/get: round-trip for all OptionValue variants.
2. Option unset: unset removes override, resolution falls through.
3. FALLTHROUGH: pane option absent, window option present -> window value returned.
4. Full chain: pane -> window -> session -> server resolution.
5. Config parse: `set-option -g status-left "foo"` parses correctly.
6. Config comments: lines starting with `#` are ignored.
7. Config errors: unknown command produces ConfigError with line number.
8. Config load timing: config only loaded after first client identify.
9. source-file: nested config inclusion works.
10. Empty config: parsing empty string returns empty directives.

### AGENTS.md Rules

- `RULE-S10-01`: Config loads only after first client identify burst.
- `RULE-S10-02`: Option FALLTHROUGH: pane -> window per `options.c:891-903`.

---

## 11. Layout Engine

### Design Decisions

- Flat arena `LayoutTree` with `Vec<LayoutCell>` and index-based parent/children.
- Round-robin resize: one cell at a time, matching `layout.c:448-462`.
- Split minimum: `PANE_MINIMUM` constant (2) per `tmux.h:100`.
- Layout checksum computed per `layout-custom.c:46-57` for compatibility.
- Layout validation per `layout-custom.c:119-153`: dimensions must add up, no overlaps.
- `debug_assert!(layout_check(...))` in all layout-mutating functions.

### Rust Example

```rust
// crates/mux-core/src/layout.rs
pub const PANE_MINIMUM: u16 = 2;

#[derive(Debug, Clone)]
pub struct LayoutTree {
    pub cells: Vec<LayoutCell>,
    pub root: usize,
}

#[derive(Debug, Clone)]
pub struct LayoutCell {
    pub kind: LayoutKind,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub pane: Option<PaneId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutKind {
    Horizontal, // children arranged left-to-right
    Vertical,   // children arranged top-to-bottom
    Leaf,       // terminal pane
}

/// Round-robin resize matching layout.c:448-462.
/// Distributes delta one cell at a time across children.
pub fn layout_resize_adjust(tree: &mut LayoutTree, node: usize, delta: i32) {
    let children = tree.cells[node].children.clone();
    if children.is_empty() || delta == 0 {
        return;
    }
    let step = if delta > 0 { 1 } else { -1 };
    let mut remaining = delta.abs();
    let mut idx = 0;
    while remaining > 0 {
        let child = children[idx % children.len()];
        let cell = &tree.cells[child];
        let (dim, min) = match tree.cells[node].kind {
            LayoutKind::Horizontal => (cell.width as i32, PANE_MINIMUM as i32),
            LayoutKind::Vertical => (cell.height as i32, PANE_MINIMUM as i32),
            LayoutKind::Leaf => return,
        };
        if dim + step >= min {
            match tree.cells[node].kind {
                LayoutKind::Horizontal => {
                    tree.cells[child].width = (dim + step) as u16;
                }
                LayoutKind::Vertical => {
                    tree.cells[child].height = (dim + step) as u16;
                }
                _ => {}
            }
            remaining -= 1;
        }
        idx += 1;
        if idx >= children.len() * (delta.unsigned_abs() as usize) {
            break; // Safety: prevent infinite loop if all children at minimum
        }
    }
    debug_assert!(layout_check(tree));
}

/// Layout checksum per layout-custom.c:46-57.
pub fn layout_checksum(tree: &LayoutTree) -> u16 {
    let mut csum: u16 = 0;
    let s = layout_serialize(tree);
    for byte in s.bytes() {
        csum = (csum >> 1) | ((csum & 1) << 15);
        csum = csum.wrapping_add(byte as u16);
    }
    csum
}

/// Layout validation per layout-custom.c:119-153.
pub fn layout_check(tree: &LayoutTree) -> bool {
    fn check_node(tree: &LayoutTree, idx: usize) -> bool {
        let cell = &tree.cells[idx];
        if cell.kind == LayoutKind::Leaf {
            return cell.width >= PANE_MINIMUM && cell.height >= PANE_MINIMUM;
        }
        let children = &cell.children;
        if children.is_empty() {
            return false;
        }
        match cell.kind {
            LayoutKind::Horizontal => {
                let total_w: u16 = children.iter()
                    .map(|&c| tree.cells[c].width)
                    .sum::<u16>() + (children.len() as u16 - 1); // separators
                total_w == cell.width &&
                    children.iter().all(|&c| tree.cells[c].height == cell.height) &&
                    children.iter().all(|&c| check_node(tree, c))
            }
            LayoutKind::Vertical => {
                let total_h: u16 = children.iter()
                    .map(|&c| tree.cells[c].height)
                    .sum::<u16>() + (children.len() as u16 - 1);
                total_h == cell.height &&
                    children.iter().all(|&c| tree.cells[c].width == cell.width) &&
                    children.iter().all(|&c| check_node(tree, c))
            }
            _ => false,
        }
    }
    check_node(tree, tree.root)
}

fn layout_serialize(tree: &LayoutTree) -> String {
    fn serialize_node(tree: &LayoutTree, idx: usize) -> String {
        let cell = &tree.cells[idx];
        let dim = format!("{}x{},{},{}", cell.width, cell.height, cell.x, cell.y);
        if cell.kind == LayoutKind::Leaf {
            return dim;
        }
        let sep = if cell.kind == LayoutKind::Horizontal { '{' } else { '[' };
        let end = if cell.kind == LayoutKind::Horizontal { '}' } else { ']' };
        let children: Vec<String> = cell.children.iter()
            .map(|&c| serialize_node(tree, c))
            .collect();
        format!("{}{}{}{}", dim, sep, children.join(","), end)
    }
    serialize_node(tree, tree.root)
}
```

### Test Strategy

1. Round-robin resize: delta distributed one-at-a-time across children.
2. Minimum enforcement: no child shrinks below `PANE_MINIMUM`.
3. Checksum: matches tmux's layout-custom.c output for known layouts.
4. Validation: valid layout passes `layout_check`; invalid fails.
5. Serialize: layout string format matches tmux.
6. Split: splitting a pane creates two children with correct dimensions.
7. proptest: random resize operations never violate layout invariants.
8. Nested layouts: horizontal inside vertical works correctly.

### AGENTS.md Rules

- `RULE-S11-01`: `debug_assert!(layout_check(...))` in all layout-mutating functions.
- `RULE-S11-02`: Format strings match tmux via `format-audit` parity corpus.

---

## 12. ORM Query Layer

### Design Decisions

- 18 query operators: 12 from libtmux `LOOKUP_NAME_MAP` + 6 extensions.
- libtmux operators: `eq`/`exact`, `iexact`, `contains`, `icontains`, `startswith`, `istartswith`, `endswith`, `iendswith`, `in`, `nin`, `regex`, `iregex`.
- Extensions: `lt`, `lte`, `gt`, `gte`, `ne`, `between`.
- `QueryList<T>` is the filterable collection type.
- Django-style kwargs: `sessions.filter(name="work")`.
- `keygetter`-style nested `__` traversal matching libtmux `query_list.py:45-113`.
- Callable matcher: `sessions.filter(lambda s: s.name == "work")`.
- `get(default=no_arg)` semantics.
- Exceptions: `MultipleObjectsReturned`, `ObjectDoesNotExist`.

### Rust Example

```rust
// crates/mux-query/src/lib.rs
#[derive(Debug, Clone)]
pub enum QueryOp {
    Eq, Exact, IExact,
    Contains, IContains,
    StartsWith, IStartsWith,
    EndsWith, IEndsWith,
    In, NotIn,
    Regex, IRegex,
    // Extensions beyond libtmux
    Lt, Lte, Gt, Gte,
    Ne,
    Between,
}

impl QueryOp {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "eq" | "exact" => Some(Self::Eq),
            "iexact" => Some(Self::IExact),
            "contains" => Some(Self::Contains),
            "icontains" => Some(Self::IContains),
            "startswith" => Some(Self::StartsWith),
            "istartswith" => Some(Self::IStartsWith),
            "endswith" => Some(Self::EndsWith),
            "iendswith" => Some(Self::IEndsWith),
            "in" => Some(Self::In),
            "nin" => Some(Self::NotIn),
            "regex" => Some(Self::Regex),
            "iregex" => Some(Self::IRegex),
            "lt" => Some(Self::Lt),
            "lte" => Some(Self::Lte),
            "gt" => Some(Self::Gt),
            "gte" => Some(Self::Gte),
            "ne" => Some(Self::Ne),
            "between" => Some(Self::Between),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum QueryValue {
    String(String),
    Int(i64),
    Bool(bool),
    List(Vec<QueryValue>),
    None,
}

#[derive(Debug, Clone)]
pub struct QuerySpec {
    pub field: String,
    pub op: QueryOp,
    pub value: QueryValue,
}

/// Parse "field__op" into (field_path, op).
/// Supports nested traversal: "foods__fruit__in" -> (["foods", "fruit"], In)
pub fn parse_query_key(key: &str) -> (Vec<&str>, QueryOp) {
    let parts: Vec<&str> = key.split("__").collect();
    if parts.len() >= 2 {
        if let Some(op) = QueryOp::from_name(parts.last().unwrap()) {
            return (parts[..parts.len()-1].to_vec(), op);
        }
    }
    (parts, QueryOp::Eq) // default to Eq when no operator suffix
}

pub struct QueryList<T> {
    items: Vec<T>,
}

impl<T> QueryList<T> {
    pub fn new(items: Vec<T>) -> Self { Self { items } }

    pub fn filter<F: Fn(&T) -> bool>(&self, predicate: F) -> QueryList<&T> {
        QueryList {
            items: self.items.iter().filter(|item| predicate(item)).collect(),
        }
    }

    pub fn get<F: Fn(&T) -> bool>(&self, predicate: F) -> Result<&T, QueryError> {
        let matches: Vec<&T> = self.items.iter().filter(|item| predicate(item)).collect();
        match matches.len() {
            0 => Err(QueryError::ObjectDoesNotExist),
            1 => Ok(matches[0]),
            _ => Err(QueryError::MultipleObjectsReturned { count: matches.len() }),
        }
    }
}

#[derive(Debug, Error)]
pub enum QueryError {
    #[error("object does not exist")]
    ObjectDoesNotExist,
    #[error("multiple objects returned ({count})")]
    MultipleObjectsReturned { count: usize },
    #[error("unknown operator: {op}")]
    UnknownOperator { op: String },
}
```

### Test Strategy

1. All 18 operators: each operator tested with matching and non-matching data.
2. `eq`/`exact` alias: both resolve to the same operator.
3. Case-insensitive: `icontains`, `iexact`, `istartswith`, `iendswith`, `iregex`.
4. Nested traversal: `"foods__fruit__in"` parses to `(["foods", "fruit"], In)`.
5. Default operator: `"name"` (no `__`) defaults to `Eq`.
6. QueryList filter: returns matching items.
7. QueryList get: returns single item or error.
8. MultipleObjectsReturned: multiple matches raises error.
9. ObjectDoesNotExist: no matches raises error.
10. Regex: `regex` and `iregex` operators use `re.search` semantics.
11. Between: `between` with `[low, high]` range.
12. Callable matcher: function predicate in `filter()`.

### AGENTS.md Rules

- `RULE-S12-01`: Key bindings match tmux via generated parity tests.

---

## 13. Concurrency Model

### Design Decisions

- Single-writer state actor: only one task mutates `ServerGraph`.
- `ArcSwap<GraphState>` for zero-contention snapshot reads.
- No `Arc<Mutex<_>>` on read paths.
- Tokio channels for event submission to the state actor.
- `StateHandle` wraps `ArcSwap` for ergonomic reads from multiple tasks.
- Actor processes events sequentially, publishes snapshots after each batch.

### Rust Example

```rust
// crates/mux-api/src/state_handle.rs
use arc_swap::ArcSwap;
use std::sync::Arc;

pub struct StateHandle {
    inner: Arc<ArcSwap<GraphState>>,
}

impl StateHandle {
    pub fn new(initial: GraphState) -> Self {
        Self {
            inner: Arc::new(ArcSwap::new(Arc::new(initial))),
        }
    }

    /// Zero-contention snapshot read.
    pub fn load(&self) -> arc_swap::Guard<Arc<GraphState>> {
        self.inner.load()
    }

    /// Publish a new snapshot (called by state actor only).
    pub(crate) fn store(&self, state: GraphState) {
        self.inner.store(Arc::new(state));
    }
}

impl Clone for StateHandle {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

// crates/mux-server/src/state_actor.rs
pub struct StateActor {
    graph: ServerGraph,
    handle: StateHandle,
    event_rx: tokio::sync::mpsc::Receiver<Event>,
    ctx: CoreCtx,
}

impl StateActor {
    pub async fn run(mut self) {
        while let Some(event) = self.event_rx.recv().await {
            let outcome = self.graph.apply_event(event, &self.ctx);
            // Publish updated snapshot
            self.handle.store(self.graph.snapshot());
            // Dispatch effects to runtime
            for effect in outcome.effects {
                self.dispatch_effect(effect).await;
            }
        }
    }

    async fn dispatch_effect(&self, effect: Effect) {
        match effect {
            Effect::SpawnProcess { pane, command, cwd, size } => {
                // Delegate to PTY manager
            }
            Effect::Notify(notification) => {
                // Broadcast to control mode clients
            }
            _ => {}
        }
    }
}
```

### Test Strategy

1. Single writer: concurrent event submissions are serialized by actor.
2. Snapshot reads: multiple readers see consistent state without blocking.
3. ArcSwap load: returns most recent snapshot.
4. Actor shutdown: dropping event sender cleanly shuts down actor.
5. Effect dispatch: effects from apply_event are dispatched correctly.
6. No mutex contention: benchmark confirms no lock contention on reads.

### AGENTS.md Rules

- `RULE-S13-01`: Only the state actor mutates graph state. No `Arc<Mutex<_>>` on read paths.

---

## 14. Server Lifecycle

### Design Decisions

- `flock(LOCK_EX|LOCK_NB)` per `client.c:77-101`.
- Lock file guard via `Drop` impl, never manual unlock.
- Server starts on first client connect if not already running.
- Config load triggers after first client completes identify burst.
- Graceful shutdown: drain pending operations, close sockets, exit.
- `ManagedMux` combines server lifecycle with `StateHandle`.

### Rust Example

```rust
// crates/mux-api/src/managed.rs
pub struct ManagedMux {
    state_handle: StateHandle,
    event_tx: tokio::sync::mpsc::Sender<Event>,
    shutdown_tx: tokio::sync::broadcast::Sender<()>,
}

impl ManagedMux {
    pub fn state(&self) -> StateHandle {
        self.state_handle.clone()
    }

    pub fn submit_event(&self, event: Event) -> Result<(), CoreError> {
        self.event_tx.try_send(event)
            .map_err(|_| CoreError::SessionNotFound { id: SessionId::default() })
    }

    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(());
    }
}

/// TermForgeStack: composition root for in-process embedding.
pub struct TermForgeStack {
    pub managed: ManagedMux,
    _actor_handle: tokio::task::JoinHandle<()>,
}

impl TermForgeStack {
    pub async fn start() -> Self {
        let graph = ServerGraph::default();
        let handle = StateHandle::new(graph.snapshot());
        let (event_tx, event_rx) = tokio::sync::mpsc::channel(1024);
        let (shutdown_tx, _shutdown_rx) = tokio::sync::broadcast::channel(1);

        let actor = StateActor {
            graph,
            handle: handle.clone(),
            event_rx,
            ctx: CoreCtx::real(),
        };

        let actor_handle = tokio::spawn(actor.run());

        Self {
            managed: ManagedMux { state_handle: handle, event_tx, shutdown_tx },
            _actor_handle: actor_handle,
        }
    }
}

// crates/mux-os/src/lock.rs
pub struct LockGuard {
    file: std::fs::File,
}

impl LockGuard {
    pub fn try_lock(path: &std::path::Path) -> Result<Self, LockError> {
        use fs2::FileExt;
        let file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(path)?;
        file.try_lock_exclusive()
            .map_err(|_| LockError::AlreadyLocked(path.to_owned()))?;
        Ok(Self { file })
    }
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        use fs2::FileExt;
        let _ = self.file.unlock();
    }
}
```

### Test Strategy

1. Lock acquisition: `try_lock` succeeds on free path.
2. Lock contention: second `try_lock` on same path returns `AlreadyLocked`.
3. Lock release on drop: dropping `LockGuard` releases lock.
4. Config load timing: config loaded only after first client identify.
5. ManagedMux lifecycle: start -> submit events -> shutdown.
6. TermForgeStack: in-process embedding starts and stops cleanly.

### AGENTS.md Rules

- `RULE-S14-01`: Lock file guard via `Drop` impl, never manual unlock.
- `RULE-S14-02`: Config load trigger must remain post-identify.

---

## 15. Control Mode

### Design Decisions

- Control mode notifications are hints, not authoritative state.
- Periodic refresh reconciles control-mode-derived state with actual state.
- Extended output format per `control.c:620-623`.
- Pending limit enforcement per `control.c:450-461`.
- Control mode startup sequence per `control.c:758-796`.

### Rust Example

```rust
// crates/mux-control/src/notification.rs
#[derive(Debug, Clone)]
pub enum ControlNotification {
    SessionChanged { session_id: String, session_name: String },
    SessionCreated { session_id: String, session_name: String },
    SessionRenamed { session_id: String, new_name: String },
    SessionClosed { session_id: String },
    WindowAdd { window_id: String },
    WindowClose { window_id: String },
    WindowRenamed { window_id: String, new_name: String },
    WindowPaneChanged { window_id: String, pane_id: String },
    LayoutChange { window_id: String, layout: String },
    PaneOutput { pane_id: String, data: String },
    PaneModeChanged { pane_id: String, mode: String },
    ClientDetached { client_name: String, reason: String },
    ClientSessionChanged { client_name: String, session_id: String },
    Output { pane_id: String, data: String },
}

pub fn parse_notification(line: &str) -> Result<ControlNotification, ControlParseError> {
    let (prefix, rest) = line.split_once(' ')
        .ok_or(ControlParseError::MalformedLine(line.to_string()))?;

    match prefix {
        "%session-changed" => {
            let (id, name) = rest.split_once(' ')
                .ok_or(ControlParseError::MalformedLine(line.to_string()))?;
            Ok(ControlNotification::SessionChanged {
                session_id: id.to_string(),
                session_name: name.to_string(),
            })
        }
        "%window-add" => Ok(ControlNotification::WindowAdd {
            window_id: rest.to_string(),
        }),
        "%layout-change" => {
            let (wid, layout) = rest.split_once(' ')
                .ok_or(ControlParseError::MalformedLine(line.to_string()))?;
            Ok(ControlNotification::LayoutChange {
                window_id: wid.to_string(),
                layout: layout.to_string(),
            })
        }
        _ => Err(ControlParseError::UnknownNotification(prefix.to_string())),
    }
}
```

### Test Strategy

1. Session notification: `%session-changed $1 work` parses correctly.
2. Window notification: `%window-add @0` parses correctly.
3. Layout notification: `%layout-change @0 <layout>` parses layout string.
4. Unknown prefix: returns `UnknownNotification` error.
5. Malformed line: no space separator returns error.
6. Periodic refresh: stale control-mode state corrected by refresh.
7. Pending limit: notifications beyond limit are dropped gracefully.
8. Extended output: `%output` prefix with pane_id and data.

### AGENTS.md Rules

- `RULE-S15-01`: Control notifications are hints, not authoritative.

---

## 16. Language Bindings

### Design Decisions

- Python bindings via PyO3 with `#[pyclass]`/`#[pymethods]`.
- Node.js bindings via Neon with `JsBox` wrapping.
- Both expose the ORM API: `QueryList` with `filter()`/`get()`, `Server`/`Session`/`Window`/`Pane` objects.
- Exception types: `ObjectDoesNotExist`, `MultipleObjectsReturned` mapped to Python exceptions.
- OTEL context propagation via `traceparent` parameter.
- `PyTermlet` and `JsTermlet` binding handles with `Mutex<Termlet>` for thread safety.
- GIL release in PyTermlet `send_keys` and `wait_for` via `py.allow_threads()`.
- Modern PyO3 `Bound<'py, T>` API used where applicable.

### Rust Example

```rust
// bindings/python/src/lib.rs
use pyo3::prelude::*;
use pyo3::exceptions::{PyRuntimeError, PyValueError};

#[pyclass]
pub struct Server {
    managed: mux_api::ManagedMux,
}

#[pymethods]
impl Server {
    #[new]
    fn new() -> PyResult<Self> {
        todo!("Initialize in-process or connect to running server")
    }

    #[getter]
    fn sessions(&self) -> PyResult<PyQueryList> {
        let state = self.managed.state().load();
        let items: Vec<PySession> = state.sessions.iter()
            .map(|(id, s)| PySession { id, name: s.name.clone() })
            .collect();
        Ok(PyQueryList { items })
    }

    fn new_session(&self, name: &str) -> PyResult<PySession> {
        self.managed.submit_event(Event::CreateSession { name: name.to_string() })
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        // Return the new session
        todo!()
    }

    fn __enter__(slf: Py<Self>) -> Py<Self> { slf }

    fn __exit__(
        &self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_val: Option<&Bound<'_, PyAny>>,
        _exc_tb: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<bool> {
        self.managed.shutdown();
        Ok(false)
    }
}

// PyTermlet binding handle with Mutex for thread safety.
#[pyclass]
pub struct PyTermlet {
    inner: std::sync::Mutex<mux_termlet::Termlet>,
}

#[pymethods]
impl PyTermlet {
    #[new]
    #[pyo3(signature = (command, cols=80, rows=24, fake=false))]
    fn new(command: &str, cols: u16, rows: u16, fake: bool) -> PyResult<Self> {
        let config = mux_termlet::TermletConfig {
            cols, rows,
            env: vec![],
            cwd: None,
            backend: if fake { mux_termlet::PtyMode::Fake } else { mux_termlet::PtyMode::Real },
            inherit_env: false, // Default to hermetic
            ..Default::default()
        };
        let termlet = mux_termlet::Termlet::spawn(command, config)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        Ok(Self { inner: std::sync::Mutex::new(termlet) })
    }

    /// Send keystrokes. Releases GIL during PTY write.
    fn send_keys(&self, py: Python<'_>, keys: &str) -> PyResult<()> {
        let keys = keys.to_string();
        py.allow_threads(|| {
            self.inner.lock().unwrap()
                .send_keys(&keys)
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))
        })
    }

    /// Wait for pattern. Releases GIL during poll loop.
    #[pyo3(signature = (pattern, timeout_ms=30000))]
    fn wait_for(&self, py: Python<'_>, pattern: &str, timeout_ms: u64) -> PyResult<()> {
        let pattern = pattern.to_string();
        py.allow_threads(|| {
            self.inner.lock().unwrap()
                .wait_for(&pattern, std::time::Duration::from_millis(timeout_ms))
                .map(|_| ())
                .map_err(|e| match e {
                    mux_termlet::TermletError::WaitForTimeout { .. } =>
                        pyo3::exceptions::PyTimeoutError::new_err(e.to_string()),
                    _ => PyRuntimeError::new_err(e.to_string()),
                })
        })
    }

    /// Capture a text snapshot of the current grid state.
    fn snapshot(&self) -> PyResult<String> {
        Ok(self.inner.lock().unwrap().snapshot().to_text())
    }

    /// Resize the Termlet grid.
    fn resize(&self, cols: u16, rows: u16) -> PyResult<()> {
        self.inner.lock().unwrap().resize(cols, rows)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Kill the underlying process.
    fn kill(&self) -> PyResult<()> {
        self.inner.lock().unwrap().kill()
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Check if the underlying process is still alive.
    #[getter]
    fn is_alive(&self) -> bool {
        self.inner.lock().unwrap().is_alive()
    }

    /// Get the current grid dimensions.
    #[getter]
    fn size(&self) -> (u16, u16) {
        let s = self.inner.lock().unwrap().size();
        (s.sx, s.sy)
    }

    /// Context manager entry.
    fn __enter__(slf: Py<Self>) -> Py<Self> { slf }

    /// Context manager exit -- kills the Termlet.
    fn __exit__(
        &self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_val: Option<&Bound<'_, PyAny>>,
        _exc_tb: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<bool> {
        let _ = self.kill();
        Ok(false)
    }

    fn __repr__(&self) -> String {
        let guard = self.inner.lock().unwrap();
        let s = guard.size();
        format!("Termlet({}x{}, alive={})", s.sx, s.sy, guard.is_alive())
    }
}

// bindings/node/src/js_termlet.rs
pub struct JsTermlet { inner: std::sync::Mutex<mux_termlet::Termlet> }
impl Finalize for JsTermlet {}

fn termlet_spawn(mut cx: FunctionContext) -> JsResult<JsBox<JsTermlet>> {
    let command = cx.argument::<JsString>(0)?.value(&mut cx);
    let opts = cx.argument_opt(1);
    let (cols, rows, fake) = parse_termlet_opts(&mut cx, opts)?;
    let config = mux_termlet::TermletConfig {
        cols, rows,
        env: vec![],
        cwd: None,
        backend: if fake { mux_termlet::PtyMode::Fake } else { mux_termlet::PtyMode::Real },
        inherit_env: false,
        ..Default::default()
    };
    let termlet = mux_termlet::Termlet::spawn(&command, config)
        .or_else(|e| cx.throw_error(e.to_string()))?;
    Ok(cx.boxed(JsTermlet { inner: std::sync::Mutex::new(termlet) }))
}

fn termlet_send_keys(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let termlet = cx.argument::<JsBox<JsTermlet>>(0)?;
    let keys = cx.argument::<JsString>(1)?.value(&mut cx);
    termlet.inner.lock().unwrap().send_keys(&keys)
        .or_else(|e| cx.throw_error(e.to_string()))?;
    Ok(cx.undefined())
}

fn termlet_snapshot(mut cx: FunctionContext) -> JsResult<JsString> {
    let termlet = cx.argument::<JsBox<JsTermlet>>(0)?;
    let text = termlet.inner.lock().unwrap().snapshot().to_text();
    Ok(cx.string(text))
}

fn termlet_kill(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let termlet = cx.argument::<JsBox<JsTermlet>>(0)?;
    termlet.inner.lock().unwrap().kill()
        .or_else(|e| cx.throw_error(e.to_string()))?;
    Ok(cx.undefined())
}
```

### Test Strategy

1. PyQueryList filter kwargs: `sessions.filter(name="work")` returns matching.
2. PyQueryList filter operator: `sessions.filter(name__startswith="w")` works.
3. PyQueryList callable: `sessions.filter(lambda s: s.name == "work")` works.
4. PyQueryList get default: `sessions.get(name="missing", default=None)` returns None.
5. PyQueryList get error: multiple matches raises `MultipleObjectsReturned`.
6. PyQueryList get not found: no matches raises `ObjectDoesNotExist`.
7. PyQueryList iteration: `for s in server.sessions` iterates all.
8. PyQueryList indexing: `server.sessions[0]` and `server.sessions[-1]` work.
9. Context manager: `with Server() as s` starts and kills server.
10. OTEL propagation: traceparent passed from Python into Rust bindings.
11. Node sessions: `server.sessions({ name: "work" })` filters correctly.
12. Node async: `await server.newSession("test")` resolves/rejects correctly.
13. Error mapping: Rust `CoreError::SessionNotFound` maps to Python `ObjectDoesNotExist`.
14. PyTermlet spawn: `Termlet("bash")` starts a shell.
15. PyTermlet send_keys: `termlet.send_keys("echo hello\n")` sends keystrokes.
16. PyTermlet snapshot: `termlet.snapshot()` returns grid text.
17. PyTermlet context manager: `with Termlet("bash") as t:` auto-kills on exit.
18. PyTermlet fake mode: `Termlet("bash", fake=True)` uses FakePtyBackend.
19. PyTermlet wait_for timeout: raises `TimeoutError` (not generic RuntimeError).
20. JsTermlet spawn: `termletSpawn("bash")` returns JsBox.
21. JsTermlet snapshot: `termletSnapshot(t)` returns string.
22. PyTermlet GIL release: `send_keys` and `wait_for` release GIL via `py.allow_threads()`.

### AGENTS.md Rules

- `RULE-S16-01`: Bindings must mirror ORM API. No extra mutation surfaces.
- `RULE-S16-02`: Expose OTEL context propagation via traceparent parameter.
- `RULE-S16-03`: Termlet bindings must expose: spawn, send_keys, wait_for, snapshot, resize, kill.
- `RULE-S16-04`: Termlet binding handles must use `Mutex<Termlet>` for thread safety.

---

## 17. CRDT Transaction Layer

### Design Decisions

- Support eventual-consistency replication between TermForge instances.
- Hybrid Logical Clock (HLC) for causal ordering.
- LWW (Last Writer Wins) registers for scalar fields (session name, options).
- OR-Set / Add-Wins Set for collections (windows in session, panes in window).
- OpLog stores operations for merge/replay.
- CRDT layer is opt-in. Single-server mode uses direct events without HLC overhead.

### Rust Example

```rust
// crates/mux-crdt/src/hlc.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct HLC {
    pub wall_ms: i64,
    pub counter: u32,
    pub node_id: u64,
}

impl HLC {
    pub fn now(prev: &HLC, wall_ms: i64, node_id: u64) -> HLC {
        let wall = wall_ms.max(prev.wall_ms);
        let counter = if wall == prev.wall_ms { prev.counter + 1 } else { 0 };
        HLC { wall_ms: wall, counter, node_id }
    }

    pub fn merge(local: &HLC, remote: &HLC, wall_ms: i64, node_id: u64) -> HLC {
        let wall = wall_ms.max(local.wall_ms).max(remote.wall_ms);
        let counter = if wall == local.wall_ms && wall == remote.wall_ms {
            local.counter.max(remote.counter) + 1
        } else if wall == local.wall_ms { local.counter + 1 }
          else if wall == remote.wall_ms { remote.counter + 1 }
          else { 0 };
        HLC { wall_ms: wall, counter, node_id }
    }
}

// crates/mux-crdt/src/lww.rs
#[derive(Debug, Clone)]
pub struct LWWRegister<T> {
    pub value: T,
    pub timestamp: HLC,
}

impl<T: Clone> LWWRegister<T> {
    pub fn set(&mut self, value: T, ts: HLC) {
        if ts > self.timestamp {
            self.value = value;
            self.timestamp = ts;
        }
    }

    pub fn merge(&mut self, other: &LWWRegister<T>) {
        if other.timestamp > self.timestamp {
            self.value = other.value.clone();
            self.timestamp = other.timestamp;
        }
    }
}

// crates/mux-crdt/src/or_set.rs
/// Observed-Remove Set (Add-Wins semantics):
/// Concurrent add + remove of the same element results in element being present.
#[derive(Debug, Clone)]
pub struct OrSet<T: Eq + std::hash::Hash> {
    elements: HashMap<T, HashSet<(u64, HLC)>>,
    tombstones: HashMap<T, HashSet<(u64, HLC)>>,
}

impl<T: Eq + std::hash::Hash + Clone> OrSet<T> {
    pub fn add(&mut self, element: T, node_id: u64, ts: HLC) {
        self.elements.entry(element).or_default().insert((node_id, ts));
    }

    pub fn remove(&mut self, element: &T, _node_id: u64, _ts: HLC) {
        if let Some(tags) = self.elements.get(element) {
            self.tombstones.entry(element.clone()).or_default()
                .extend(tags.iter().cloned());
        }
    }

    pub fn contains(&self, element: &T) -> bool {
        let adds = self.elements.get(element).map_or(0, |s| s.len());
        let removes = self.tombstones.get(element).map_or(0, |s| s.len());
        adds > removes
    }
}

// crates/mux-crdt/src/oplog.rs
#[derive(Debug, Clone)]
pub struct OpLog {
    ops: Vec<(HLC, Event)>,
}

impl OpLog {
    pub fn append(&mut self, ts: HLC, event: Event) {
        self.ops.push((ts, event));
    }

    pub fn since(&self, ts: HLC) -> impl Iterator<Item = &(HLC, Event)> {
        self.ops.iter().filter(move |(op_ts, _)| *op_ts > ts)
    }

    pub fn merge(&mut self, remote: &OpLog) {
        for (ts, event) in &remote.ops {
            if !self.ops.iter().any(|(t, _)| t == ts) {
                self.ops.push((*ts, event.clone()));
            }
        }
        self.ops.sort_by_key(|(ts, _)| *ts);
    }
}
```

### Test Strategy

1. HLC monotonicity: after merge, HLC is strictly greater than both inputs.
2. LWW convergence: two replicas with concurrent writes converge to same value.
3. OR-Set add-remove: add + remove + re-add -> element present.
4. OR-Set concurrent add: two nodes add different elements, merge has both.
5. OpLog merge idempotent: merging same log twice does not duplicate ops.
6. Deterministic replay: OpLog replayed in HLC order produces deterministic graph.

### AGENTS.md Rules

- `RULE-S17-01`: CRDT layer is additive, not required for single-server mode.
- `RULE-S17-02`: No network code in `mux-crdt` crate.

---

## 18. Security Model

### Design Decisions

- Socket permissions: directory 0700, socket 0600, created with `umask(0o177)`.
- Symlink check on entire socket path (every ancestor).
- SCM_RIGHTS for fd passing between client and server.
- No SUID/SGID bits on any binary.
- All processes run as the calling user.

### Rust Example

```rust
// crates/mux-os/src/socket.rs
pub fn create_socket(path: &std::path::Path) -> Result<std::os::unix::net::UnixListener, SocketError> {
    let dir = path.parent().ok_or(SocketError::InvalidPath)?;
    std::fs::create_dir_all(dir)?;

    let metadata = std::fs::metadata(dir)?;
    let mut perms = metadata.permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o700);
    std::fs::set_permissions(dir, perms)?;

    for ancestor in path.ancestors() {
        if ancestor.is_symlink() {
            return Err(SocketError::SymlinkInPath(ancestor.to_owned()));
        }
    }

    let old_umask = unsafe { libc::umask(0o177) };
    let result = std::os::unix::net::UnixListener::bind(path);
    unsafe { libc::umask(old_umask); }
    result.map_err(SocketError::Bind)
}
```

### Test Strategy

1. Socket permissions: verify socket created with mode 0600.
2. Directory permissions: verify socket dir has mode 0700.
3. Symlink rejection: socket path with symlink fails creation.
4. FD passing: SCM_RIGHTS roundtrip preserves fd.
5. umask restoration: after socket creation, umask is restored to original value.

### AGENTS.md Rules

- `RULE-S18-01`: All `unsafe` socket/fd code lives in `mux-os` only.
- `RULE-S18-02`: Socket path length checked against 108-byte limit before creation.

---

## 19. OpenTelemetry

### Design Decisions

Architecture directly modeled on vibe-tmux `mux-otel` crate.

1. **Dual provider**: `OTEL_PROVIDER` (server/TUI) and `MUX_CLIENT_PROVIDER` (client).
2. **Thread-local** `TRACE_HEADERS_STACK` with `TraceHeadersGuard`.
3. **Process-level** `PROCESS_TRACE_HEADERS` for cross-thread header sharing.
4. **Composite propagator**: `BaggagePropagator` + `TraceContextPropagator`.
5. **OnceLock<Mutex<Option<OtelProvider>>>** for safe one-time init.
6. **Env-var activation**: `TERMFORGE_OTEL=1` or `OTEL_EXPORTER_OTLP_ENDPOINT` set.
7. **Bounded shutdown**: `runtime.shutdown_timeout(200ms)`.

### Rust Example

```rust
// crates/mux-otel/src/provider.rs
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_sdk::logs::SdkLoggerProvider;

pub struct OtelProvider {
    logger: Option<SdkLoggerProvider>,
    tracer_provider: Option<SdkTracerProvider>,
    tracer: Option<opentelemetry_sdk::trace::Tracer>,
    runtime: Option<tokio::runtime::Runtime>,
}

impl Drop for OtelProvider {
    fn drop(&mut self) {
        if let Some(logger) = &self.logger { let _ = logger.shutdown(); }
        if let Some(provider) = &self.tracer_provider { let _ = provider.shutdown(); }
        if let Some(runtime) = self.runtime.take() {
            runtime.shutdown_timeout(Duration::from_millis(200));
        }
    }
}

// Dual provider statics
static OTEL_PROVIDER: OnceLock<Mutex<Option<OtelProvider>>> = OnceLock::new();
static MUX_CLIENT_PROVIDER: OnceLock<Mutex<Option<OtelProvider>>> = OnceLock::new();

// Thread-local trace headers stack
thread_local! {
    static TRACE_HEADERS_STACK: RefCell<Vec<TraceHeaders>> = const { RefCell::new(Vec::new()) };
}

#[derive(Debug, Clone)]
pub struct TraceHeaders {
    pub traceparent: String,
    pub tracestate: Option<String>,
    pub baggage: Option<String>,
}

pub struct TraceHeadersGuard { _private: () }

impl Drop for TraceHeadersGuard {
    fn drop(&mut self) {
        TRACE_HEADERS_STACK.with(|stack| { stack.borrow_mut().pop(); });
    }
}

pub fn push_trace_headers(headers: TraceHeaders) -> TraceHeadersGuard {
    TRACE_HEADERS_STACK.with(|stack| { stack.borrow_mut().push(headers); });
    TraceHeadersGuard { _private: () }
}

// Composite propagator: Baggage + TraceContext
fn composite_propagator() -> TextMapCompositePropagator {
    let propagators: Vec<Box<dyn TextMapPropagator + Send + Sync>> = vec![
        Box::new(BaggagePropagator::new()),
        Box::new(TraceContextPropagator::new()),
    ];
    TextMapCompositePropagator::new(propagators)
}

// HeaderCarrier for inject/extract
#[derive(Default)]
struct HeaderCarrier(HashMap<String, String>);

impl opentelemetry::propagation::Injector for HeaderCarrier {
    fn set(&mut self, key: &str, value: String) {
        self.0.insert(key.to_ascii_lowercase(), value);
    }
}

impl opentelemetry::propagation::Extractor for HeaderCarrier {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(&key.to_ascii_lowercase()).map(String::as_str)
    }
    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(String::as_str).collect()
    }
}

// Force flush both providers
pub fn force_flush() -> bool {
    let mut ok = true;
    for lock in [&OTEL_PROVIDER, &MUX_CLIENT_PROVIDER] {
        if let Some(slot) = lock.get() {
            let guard = slot.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(provider) = guard.as_ref() {
                ok &= provider.tracer_provider.as_ref()
                    .is_none_or(|tp| tp.force_flush().is_ok());
                ok &= provider.logger.as_ref()
                    .is_none_or(|lp| lp.force_flush().is_ok());
            }
        }
    }
    ok
}

pub fn shutdown() -> bool {
    let ok = force_flush();
    for lock in [&OTEL_PROVIDER, &MUX_CLIENT_PROVIDER] {
        if let Some(slot) = lock.get() {
            let provider = {
                let mut guard = slot.lock().unwrap_or_else(|e| e.into_inner());
                guard.take()
            };
            drop(provider);
        }
    }
    ok
}
```

### 19.1 Span Naming Convention

| Span Name | Kind | Source |
|---|---|---|
| `termforge.server.start` | Internal | Server startup |
| `termforge.server.event` | Internal | Event processing |
| `termforge.server.effect` | Internal | Effect dispatch |
| `termforge.client.connect` | Client | Client connection |
| `termforge.client.command` | Client | Command execution |
| `termforge.binding.call` | Client | Python/Node binding call |
| `termforge.proto.decode` | Internal | Protocol decode |
| `termforge.proto.encode` | Internal | Protocol encode |
| `termforge.termlet.spawn` | Internal | Termlet spawn |
| `termforge.termlet.interact` | Internal | Termlet send_keys/wait_for |
| `termforge.termlet.snapshot` | Internal | Termlet snapshot capture |

### Test Strategy

1. Dual provider init: both `OTEL_PROVIDER` and `MUX_CLIENT_PROVIDER` initialized correctly.
2. Thread-local isolation: child thread does not see parent's trace headers.
3. Guard stack semantics: push A, push B, drop B, current = A.
4. Composite propagator: inject + extract roundtrips traceparent + tracestate + baggage.
5. Env enable: `TERMFORGE_OTEL=1` enables; unset disables.
6. Shutdown idempotent: calling `shutdown()` twice does not panic.
7. Force flush both: `force_flush()` flushes both providers.
8. Export filter: `mux_*` allowed, `h2`/`tonic` denied.
9. Client span lazy init: first `enter_mux_client_span` initializes `MUX_CLIENT_PROVIDER`.
10. Runtime timeout: OtelProvider drop completes within 200ms (no hang).
11. HeaderCarrier roundtrip: inject into carrier, extract from carrier, headers match.
12. Protocol resolution: gRPC inferred for port 4317; HTTP for port 4318.

### AGENTS.md Rules

- `RULE-S19-01`: OTEL is always behind env-flag guard.
- `RULE-S19-02`: Force flush both providers before process exit.
- `RULE-S19-03`: OTEL must never block Termlet hot paths on exporter latency.

---

## 20. tmux Version Management

### Design Decisions

Build and cache versioned tmux binaries from source for integration testing.

- BLAKE3 hash for cache key computation with `flags_fingerprint`.
- Full 64-char hex via `hex_32`, truncated to 12 chars at call site.
- File-based locking with `fs2::FileExt` and `Drop`-based `LockGuard`.
- Atomic build publication via `sibling_tmp_dir` + `std::fs::rename`.
- Binary validation via `tmux -V`.
- HOME-based fallback for repo clone destination.

### Rust Example

```rust
// tools/tmux-builder/src/cache.rs
use blake3;

fn hex_32(bytes: [u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0xf) as usize] as char);
    }
    out
}

fn flags_fingerprint(flags: &[String]) -> String {
    let mut data = Vec::new();
    for flag in flags {
        data.extend_from_slice(flag.as_bytes());
        data.push(0); // null separator between flags
    }
    let hash = blake3::hash(&data);
    hex_32(*hash.as_bytes())
    // NOTE: caller truncates to 12 chars: &result[..12]
}

pub fn compute_cache_key(
    tag: &str,
    host: &str,
    os_id: &str,
    os_version: &str,
    configure_flags: &[String],
    make_flags: &[String],
    builder_version: &str,
) -> String {
    let version = tag.strip_prefix("tmux-").unwrap_or(tag);
    let cfg = flags_fingerprint(configure_flags);
    let mk = flags_fingerprint(make_flags);
    format!(
        "tmux-{version}__{host}__{os}-{os_ver}__cfg{cfg}__mk{mk}__tb{tool}",
        version = sanitize_token(version),
        host = sanitize_token(host),
        os = sanitize_token(os_id),
        os_ver = sanitize_token(os_version),
        cfg = &cfg[..12],
        mk = &mk[..12],
        tool = sanitize_token(builder_version),
    )
}

// tools/tmux-builder/src/lock.rs
#[derive(Debug)]
struct LockGuard {
    file: std::fs::File,
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

fn validate_tmux_binary(path: &std::path::Path) -> Result<String, anyhow::Error> {
    let output = std::process::Command::new(path).arg("-V").output()?;
    if !output.status.success() {
        anyhow::bail!("tmux -V failed at {}", path.display());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
```

### Test Strategy

1. Cache key determinism: same inputs produce same cache key.
2. Cache key uniqueness: different configure_flags produce different key.
3. BLAKE3 fingerprint: empty flags produce consistent hash.
4. Lock exclusivity: two threads cannot hold same lock_cache_key simultaneously.
5. Lock auto-release: LockGuard drop releases file lock.
6. Atomic publish: concurrent readers never see partial build.
7. Binary validation: valid tmux binary returns version string.
8. Binary validation failure: invalid binary produces descriptive error.
9. Fingerprint truncation: 12-char prefix is unique across typical flag sets.

### AGENTS.md Rules

- `RULE-S20-01`: Build cache keys must be deterministic and collision-free.
- `RULE-S20-02`: Atomic rename for build publication; never multi-step copy.

---

## 21. Test Support and Fake PTY

### Design Decisions

- Three-layer socket validation verified against vibe-tmux `path_guard.rs`.
- PathGuard with permission hardening (0700 on socket directory).
- FakePtyBackend with ScenarioRecorder for deterministic replay.
- `ScenarioStep` with `in_bytes`/`out_bytes` for structured FakePty fixtures.
- Dual-mode test harness: in-process (FakePty) and subprocess (real PTY).
- Subprocess shutdown: SIGTERM first, then SIGKILL escalation.
- Config isolation: every test invocation uses `-f /dev/null`.
- Clipboard isolation: `set-clipboard off` in all test harnesses.

### Rust Example

```rust
// crates/mux-test-support/src/path_guard.rs
use anyhow::{Result, bail};
use std::path::{Path, PathBuf};

/// Layer 1: Refuse to use tmux default socket name.
pub fn ensure_not_default_socket_name(socket_name: &str) -> Result<()> {
    if socket_name.trim().is_empty() {
        bail!("socket name must be non-empty");
    }
    if socket_name == "default" {
        bail!("refusing to use tmux default socket name: default");
    }
    Ok(())
}

/// Layer 2: Socket must be within the test harness temp directory.
pub fn ensure_socket_within_tempdir(socket_path: &Path, tempdir: &Path) -> Result<()> {
    if !socket_path.starts_with(tempdir) {
        bail!("refusing to operate on socket outside harness temp dir: \
            socket={} tempdir={}", socket_path.display(), tempdir.display());
    }
    Ok(())
}

/// Layer 3: Socket must not match the $TMUX env var.
pub fn ensure_socket_not_tmux_env(socket_path: &Path) -> Result<()> {
    let tmux = std::env::var("TMUX").ok();
    ensure_socket_not_tmux_env_value(socket_path, tmux.as_deref())
}

// crates/mux-pty/src/backend.rs
pub trait PtyBackend: Send {
    fn spawn(&mut self, pane_id: PaneId, command: &[String],
        cwd: Option<&str>, size: PaneSize) -> Result<(), PtyError>;
    fn write(&mut self, pane_id: PaneId, data: &[u8]) -> Result<(), PtyError>;
    fn resize(&mut self, pane_id: PaneId, size: PaneSize) -> Result<(), PtyError>;
    fn kill(&mut self, pane_id: PaneId, signal: i32) -> Result<(), PtyError>;
    fn read_events(&mut self) -> Vec<PtyEvent>;
}

// crates/mux-pty-fake/src/lib.rs
pub struct FakePtyBackend {
    panes: HashMap<PaneId, FakePane>,
    pending_events: Vec<PtyEvent>,
}

impl FakePtyBackend {
    pub fn inject_output(&mut self, pane_id: PaneId, data: Vec<u8>) {
        self.pending_events.push(PtyEvent::Output { pane_id, data });
    }
}

impl PtyBackend for FakePtyBackend {
    fn spawn(&mut self, pane_id: PaneId, _command: &[String],
        _cwd: Option<&str>, size: PaneSize) -> Result<(), PtyError> {
        self.panes.insert(pane_id, FakePane {
            size, input_queue: Vec::new(), output_queue: Vec::new(), exited: false,
        });
        Ok(())
    }

    fn write(&mut self, pane_id: PaneId, data: &[u8]) -> Result<(), PtyError> {
        if let Some(pane) = self.panes.get_mut(&pane_id) {
            pane.input_queue.push(data.to_vec());
        }
        Ok(())
    }

    fn resize(&mut self, pane_id: PaneId, size: PaneSize) -> Result<(), PtyError> {
        if let Some(pane) = self.panes.get_mut(&pane_id) { pane.size = size; }
        Ok(())
    }

    fn kill(&mut self, pane_id: PaneId, _signal: i32) -> Result<(), PtyError> {
        if let Some(pane) = self.panes.get_mut(&pane_id) {
            pane.exited = true;
            self.pending_events.push(PtyEvent::Exited { pane_id, status: 0 });
        }
        Ok(())
    }

    fn read_events(&mut self) -> Vec<PtyEvent> {
        std::mem::take(&mut self.pending_events)
    }
}

/// Structured scenario step for FakePty fixtures.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScenarioStep {
    pub in_bytes: Vec<u8>,
    pub out_bytes: Vec<u8>,
}

/// A scenario is a sequence of steps for deterministic replay.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Scenario {
    pub steps: Vec<ScenarioStep>,
    pub initial_size: PaneSize,
}
```

### Test Strategy

1. Default socket rejected: `ensure_not_default_socket_name("default")` fails.
2. Empty socket rejected: `ensure_not_default_socket_name("")` fails.
3. Socket within tempdir: path inside tempdir passes; path outside fails.
4. Socket not TMUX env: socket matching $TMUX rejected.
5. PathGuard cleanup: temp dir removed on drop.
6. Permission hardening: socket dir has mode 0700.
7. FakePty inject output: injected data appears in `read_events()`.
8. FakePty spawn/kill lifecycle: spawn -> write -> kill -> Exited event.
9. Scenario roundtrip: record -> save -> load -> replay produces same grid.
10. ScenarioStep serialization: JSON roundtrip preserves bytes.

### AGENTS.md Rules

- `RULE-S21-01`: Tests must not operate on live user tmux sessions.
- `RULE-S21-02`: PathGuard three-layer validation required for all test fixtures.
- `RULE-S21-03`: FakePty scenarios must be versioned JSON fixtures.

---

## 22. Binding Test Frameworks

### Design Decisions

- Python tests via pytest with fixture chain: `server` -> `session` -> `window` -> `pane`.
- Node tests via vitest with equivalent fixture chain.
- Dual-mode parametrized tests run every test against both in-process and subprocess servers.
- OTEL integration tests verify traceparent propagation from Python.
- Custom exception types (`ObjectDoesNotExist`, `MultipleObjectsReturned`) tested in both languages.
- Termlet fixtures: pytest `termlet` fixture spawns a Termlet with automatic cleanup. vitest `useTermlet()` helper does the same.
- `TERM_FILTER` environment variable for CI test filtering.

### Rust Example (Python)

```python
# bindings/python/tests/conftest.py
import pytest
from termforge import Server, Termlet

@pytest.fixture
def server():
    """In-process TermForge server with automatic cleanup."""
    with Server() as s:
        yield s

@pytest.fixture
def session(server):
    """Server with one session."""
    return server.new_session("test")

@pytest.fixture
def window(session):
    """Session with one window."""
    return session.windows[0]

# Termlet fixtures
@pytest.fixture
def termlet():
    """Spawn a Termlet running bash with automatic cleanup."""
    with Termlet("bash", cols=80, rows=24) as t:
        yield t

@pytest.fixture
def fake_termlet():
    """Spawn a Termlet with FakePtyBackend for deterministic tests."""
    with Termlet("bash", cols=80, rows=24, fake=True) as t:
        yield t

@pytest.fixture(params=["real", "fake"])
def any_termlet(request):
    """Parametrized fixture: same test runs with real and fake PTY."""
    fake = request.param == "fake"
    with Termlet("bash", cols=80, rows=24, fake=fake) as t:
        yield t
```

### Rust Example (Python Tests)

```python
# bindings/python/tests/test_query.py
def test_filter_sessions(server):
    server.new_session("alpha")
    server.new_session("beta")
    assert len(server.sessions.filter(session_name="alpha")) == 1
    assert len(server.sessions.filter(session_name__startswith="a")) == 1

def test_get_session(server):
    server.new_session("work")
    s = server.sessions.get(session_name="work")
    assert s.session_name == "work"

def test_get_not_found(server):
    from termforge import ObjectDoesNotExist
    with pytest.raises(ObjectDoesNotExist):
        server.sessions.get(session_name="missing")

# bindings/python/tests/test_termlet.py
def test_termlet_echo(termlet):
    """Basic send-keys and snapshot test."""
    termlet.send_keys("echo hello\n")
    termlet.wait_for("hello", timeout_ms=5000)
    snapshot = termlet.snapshot()
    assert "hello" in snapshot

def test_termlet_resize(termlet):
    """Verify programmatic resize."""
    assert termlet.size == (80, 24)
    termlet.resize(120, 40)
    assert termlet.size == (120, 40)

def test_termlet_context_manager():
    """Context manager auto-kills on exit."""
    from termforge import Termlet
    with Termlet("bash") as t:
        t.send_keys("echo managed\n")
        t.wait_for("managed")
    # t is killed here -- no leaked processes

def test_termlet_wait_for_timeout():
    """wait_for raises TimeoutError on timeout."""
    from termforge import Termlet
    with Termlet("bash") as t:
        with pytest.raises(TimeoutError):
            t.wait_for("this_will_never_appear", timeout_ms=100)
```

### Rust Example (Node.js)

```javascript
// bindings/node/tests/termlet.test.js
import { describe, it, expect } from 'vitest';
import { useTermlet } from 'termforge';

describe('Termlet', () => {
    it('spawns and captures output', () => {
        const t = useTermlet('bash', { cols: 80, rows: 24 });
        try {
            t.sendKeys('echo hello\n');
            t.waitFor('hello', 5000);
            const snapshot = t.snapshot();
            expect(snapshot).toContain('hello');
        } finally {
            t.kill();
        }
    });

    it('supports fake mode for deterministic tests', () => {
        const t = useTermlet('bash', { cols: 80, rows: 24, fake: true });
        try {
            t.sendKeys('echo test\n');
            const snapshot = t.snapshot();
            expect(typeof snapshot).toBe('string');
        } finally {
            t.kill();
        }
    });
});
```

### Test Strategy

1. pytest fixtures: `server`, `session`, `window` chain correctly.
2. Dual-mode parametrize: same test runs in-process and subprocess.
3. OTEL propagation: `TRACEPARENT` env var reaches Rust.
4. Node filter: `sessions({ name: "work" })` returns matching.
5. Error types: Python `ObjectDoesNotExist` and `MultipleObjectsReturned` raised correctly.
6. Context manager: `with Server()` cleans up on exit.
7. pytest `termlet` fixture: spawns, yields, auto-kills.
8. pytest `fake_termlet` fixture: uses FakePtyBackend.
9. pytest `any_termlet` parametrize: same test, real and fake.
10. vitest `useTermlet()`: spawns, returns handle with kill cleanup.
11. Termlet send_keys + snapshot roundtrip: both languages.
12. Termlet wait_for timeout: raises `TimeoutError` in Python.
13. Termlet context manager: no leaked processes.
14. `TERM_FILTER` env var selects test subset in CI.

### AGENTS.md Rules

- `RULE-S22-01`: Binding tests must use the same ORM semantics as core tests.
- `RULE-S22-02`: Dual-mode fixture required for all integration tests.
- `RULE-S22-03`: Termlet fixtures must auto-kill on cleanup. No leaked PTY processes.
- `RULE-S22-04`: Termlet tests must run in both real and fake PTY modes where applicable.

---

## 23. Test Framework and Harness Design

### Design Decisions

- Custom VT100 parser testing against `input.c` state table.
- Grid snapshot testing via `insta`.
- Format engine parity testing via `format-audit` corpus.
- Key binding parity testing via generated tests from `key-bindings.c`.
- Property testing with `proptest` for protocol roundtrip and layout resize.
- Fuzz targets for proto decode, VT100 parse, and config parse.
- Three-tier harness: unit (pure), integration (runtime), compatibility (real tmux).
- Flaky-test quarantine requires replay log before quarantine.

### Rust Example

```rust
// VT100 parser testing
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_text() {
        let mut grid = Grid::new(80, 24);
        let mut parser = VtParser::new();
        parser.parse(b"Hello, World!", &mut grid);
        assert_eq!(grid.row_text(0), "Hello, World!");
    }

    #[test]
    fn cursor_movement() {
        let mut grid = Grid::new(80, 24);
        let mut parser = VtParser::new();
        parser.parse(b"\x1b[5;10HX", &mut grid);
        assert_eq!(grid.cell(4, 9).ch, 'X');
    }
}

// Grid snapshot testing with insta
#[test]
fn htop_rendering() {
    let mut grid = Grid::new(120, 40);
    let mut parser = VtParser::new();
    let fixture = std::fs::read("fixtures/htop-output.bin").unwrap();
    parser.parse(&fixture, &mut grid);
    insta::assert_snapshot!(grid.to_text());
}

// Property testing
use proptest::prelude::*;

proptest! {
    #[test]
    fn imsg_roundtrip(
        msg_type in 0u32..300,
        payload in prop::collection::vec(any::<u8>(), 0..1000)
    ) {
        let frame = ImsgFrame {
            header: ImsgHdr { msg_type, len: (IMSG_HDR_SIZE + payload.len()) as u32,
                peerid: 0, pid: 0, has_fd: false },
            payload: payload.into(),
        };
        let mut buf = BytesMut::new();
        let codec = ImsgCodec::new();
        codec.encode(&frame, &mut buf);
        let decoded = codec.decode(&mut buf);
        prop_assert!(matches!(decoded, DecodeOutcome::Ok(_)));
    }

    #[test]
    fn layout_resize_never_panics(delta in -100i32..100, n_children in 1usize..20) {
        let mut tree = make_test_layout(n_children, 200, 50);
        layout_resize_adjust(&mut tree, tree.root, delta);
        prop_assert!(layout_check(&tree));
    }
}

// Fuzz targets
// fuzz/fuzz_targets/proto_decode.rs
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let mut codec = ImsgCodec::new();
    let mut buf = BytesMut::from(data);
    let _ = codec.decode(&mut buf);
});

/// Three-tier harness classification.
pub enum HarnessTier {
    /// Unit: pure functions, no IO, no timing.
    Unit,
    /// Integration: in-process runtime, FakePty or real PTY.
    Integration,
    /// Compatibility: real tmux binary, subprocess mode.
    Compatibility,
}
```

### Test Strategy

1. VT100 ASCII: plain text rendered correctly on grid.
2. VT100 CSI: cursor positioning, colors, attributes work.
3. Grid snapshot: insta snapshots match expected output.
4. Format parity: 200+ format variables match tmux.
5. Key binding parity: all 4 key tables match tmux defaults.
6. Protocol proptest: random payloads roundtrip through codec.
7. Layout proptest: random resize never invalidates layout.
8. Fuzz: proto, VT100, config parsers survive random input.
9. Flaky test quarantine: emit replay log before quarantining.

### AGENTS.md Rules

- `RULE-S23-01`: Every parser must have a fuzz target.
- `RULE-S23-02`: Parity tests reference tmux source line numbers.
- `RULE-S23-03`: Flaky tests must emit replay logs before quarantine.

---

## 24. Performance Targets

### Design Decisions

- Criterion benchmarks as the standard measurement tool.
- Conservative targets based on known alacritty/tmux performance ranges.
- CI gate at 130% threshold on nightly only (warn, not hard fail).
- All benchmarks measure throughput where applicable.
- Termlet-specific benchmarks for spawn and snapshot latency.
- Benchmark scripts pin CPU governor where possible.

### Performance Targets Table

| ID | Benchmark | Target | CI Gate |
|---|---|---|---|
| B1 | VT100 ASCII 1MB | > 300 MB/s | Warn if < 250 MB/s |
| B2 | VT100 CSI heavy | > 100 MB/s | Warn if < 80 MB/s |
| B3 | Proto decode 10k frames | > 500 MB/s | Warn if < 400 MB/s |
| B4 | Layout resize 20 panes | < 50 us | Warn if > 100 us |
| B5 | Snapshot 50 panes | < 1 ms | Warn if > 2 ms |
| B6 | Format expand (status) | < 50 us | Warn if > 100 us |
| B7 | Config parse 500 lines | < 5 ms | Warn if > 10 ms |
| B8 | Option resolve 4-level | < 100 ns | Warn if > 200 ns |
| B9 | Control notification parse | < 200 ns | Warn if > 500 ns |
| B10 | Termlet spawn (FakePty) | < 500 us | Warn if > 1 ms |
| B11 | Termlet snapshot (80x24) | < 50 us | Warn if > 100 us |
| B12 | Termlet spawn (real PTY) | < 50 ms | Warn if > 100 ms |

### Rust Example

```rust
// crates/mux-bench/benches/termlet.rs
use criterion::{criterion_group, criterion_main, Criterion};

fn termlet_spawn_fake(c: &mut Criterion) {
    c.bench_function("termlet-spawn-fake", |b| {
        b.iter(|| {
            let mut termlet = mux_termlet::Termlet::spawn("echo", mux_termlet::TermletConfig {
                cols: 80, rows: 24,
                backend: mux_termlet::PtyMode::Fake,
                ..Default::default()
            }).unwrap();
            termlet.kill().unwrap();
        });
    });
}

fn termlet_snapshot(c: &mut Criterion) {
    let mut termlet = mux_termlet::Termlet::spawn("echo", mux_termlet::TermletConfig {
        cols: 80, rows: 24,
        backend: mux_termlet::PtyMode::Fake,
        ..Default::default()
    }).unwrap();
    termlet.send_keys("Hello, World!\n").unwrap();
    c.bench_function("termlet-snapshot-80x24", |b| {
        b.iter(|| {
            let _snap = termlet.snapshot();
        });
    });
}

criterion_group!(termlet_benches, termlet_spawn_fake, termlet_snapshot);
criterion_main!(termlet_benches);
```

### Test Strategy

1. Criterion benchmarks run nightly.
2. Regression flag at 130% of baseline.
3. Baseline established from 3 consecutive median runs.
4. No hard fail on performance (warn only). Hard fail reserved for functional gates.
5. Termlet benchmarks included in nightly run.
6. Benchmark scripts pin CPU governor where possible.
7. Performance regressions > 30% block release.

### AGENTS.md Rules

- `RULE-S24-01`: Performance baselines re-established after architecture changes.
- `RULE-S24-02`: Benchmarks must cover hot-path code (VT100, proto, layout).
- `RULE-S24-03`: Termlet spawn benchmark must stay under 500us for FakePty mode.
- `RULE-S24-04`: Performance regressions > 30% block release.

---

## 25. Visual Client / TUI

### Design Decisions

- ViewModel pattern: pure function `(GraphState, ClientId, TermSize) -> ViewModel`.
- ratatui-based rendering with crossterm terminal IO.
- ViewModel supports snapshot testing with `insta`.
- TUI can attach to both TermForge server (via StateHandle) and real tmux (via control mode).
- TermletViewModel for debug inspector panel shows Termlet grid preview and process status.

### Rust Example

```rust
// crates/mux-view/src/lib.rs
pub struct ViewModel {
    pub status_top: StatusLine,
    pub status_bottom: StatusLine,
    pub panes: Vec<PaneView>,
    pub borders: Vec<Border>,
    pub mode_indicator: Option<String>,
}

pub struct PaneView {
    pub x: u16, pub y: u16,
    pub width: u16, pub height: u16,
    pub grid_rows: Vec<Vec<StyledCell>>,
    pub is_active: bool,
    pub is_zoomed: bool,
}

/// Pure function: no IO, no side effects. Suitable for snapshot testing.
pub fn build_view_model(
    state: &GraphState,
    client_id: ClientId,
    term_size: (u16, u16),
) -> ViewModel {
    let (_cols, _rows) = term_size;
    todo!()
}

/// TUI can target TermForge server or real tmux.
pub enum TuiTarget {
    TermForge { state_handle: StateHandle },
    RealTmux { socket_path: PathBuf },
}

/// Debug inspector panel for Termlet state.
pub struct TermletViewModel {
    pub pane_id: PaneId,
    pub alive: bool,
    pub grid_preview: String,
    pub size: (u16, u16),
}

impl TermletViewModel {
    pub fn from_termlet(termlet: &mux_termlet::Termlet) -> Self {
        let snap = termlet.snapshot();
        let s = termlet.size();
        Self {
            pane_id: termlet.pane_id(),
            alive: termlet.is_alive(),
            grid_preview: snap.to_text(),
            size: (s.sx, s.sy),
        }
    }
}
```

### Test Strategy

1. ViewModel snapshot: pure ViewModel from test GraphState matches `insta` snapshot.
2. Status line format: format expansion produces correct status text.
3. Pane layout: ViewModel pane positions match layout tree.
4. Key conversion: crossterm key events map to mux-core Key variants.
5. Border rendering: correct border characters at pane boundaries.
6. Zoomed pane: single pane fills entire area when zoomed.
7. Resize: terminal resize updates ViewModel dimensions.
8. Attach to real tmux: control mode notifications render correctly.
9. TermletViewModel: grid preview matches Termlet snapshot.

### AGENTS.md Rules

- `RULE-S25-01`: TUI reads snapshots only, never mutates graph directly.
- `RULE-S25-02`: UI event handlers emit commands/events, not direct state mutations.
- `RULE-S25-03`: Debug panels are non-blocking and optional.

---

## 26. AGENTS.md Rules

### Design Decisions

This section consolidates all per-section rules into a global reference with numbered IDs and enforcement mechanisms. The rule naming convention follows `RULE-Snn-xx` pattern for traceability.

### 26.1 Master Rule Table

| # | Rule ID | Rule | Enforcement |
|---|---|---|---|
| 1 | RULE-S5-01 | `mux-core` is pure: no IO, no `unsafe`, no `tokio`, no `libc` | `#![forbid(unsafe_code)]`, WASM CI check |
| 2 | RULE-S7-01 | Core emits effects; only the runtime executes effects | Code review + typed `Effect` enum |
| 3 | RULE-S7-02 | All state transitions go through `apply_event()` | `grep` for direct graph mutation |
| 4 | RULE-S13-01 | Only the state actor mutates graph state | ArcSwap read path, no `Arc<Mutex<_>>` |
| 5 | RULE-S8-01 | `thiserror` for library errors, `anyhow` only in binaries/tests | CI: `grep anyhow crates/*/src/ \| grep -v test` |
| 6 | RULE-S8-02 | Every `ErrorClass` variant tested | Test coverage check |
| 7 | RULE-S9-01 | Protocol violations kill the connection, never drop-and-continue | `DecodeOutcome::Fatal` enforced |
| 8 | RULE-S10-01 | Config loads only after first client identify burst | Startup sequence test |
| 9 | RULE-S10-02 | Option FALLTHROUGH: pane -> window per `options.c:891-903` | Property test |
| 10 | RULE-S11-01 | `debug_assert!(layout_check(...))` in all layout-mutating functions | CI with debug assertions |
| 11 | RULE-S11-02 | Format strings match tmux via `format-audit` parity corpus | CI: `format-audit` check |
| 12 | RULE-S12-01 | Key bindings match tmux via generated parity tests | CI: key-binding parity tests |
| 13 | RULE-S6-01 | All entity IDs via `slotmap::new_key_type!` | Code review |
| 14 | RULE-S16-01 | Bindings depend only on `mux-api`/`mux-orm`, never `mux-core` directly | `cargo deny check` |
| 15 | RULE-S21-02 | Fake PTY tests use `ScenarioRecorder` JSON format | Test harness validation |
| 16 | RULE-S15-01 | Control notifications are hints, not authoritative | Periodic refresh tests |
| 17 | RULE-S5-02 | WASM CI check for all Layer 0 crates | CI: `cargo check --target wasm32-unknown-unknown` |
| 18 | RULE-S21-01 | Three-layer socket validation in all test harnesses | PathGuard enforced in test fixtures |
| 19 | RULE-S19-01 | Dual OTEL providers: server and client | Architecture test |
| 20 | RULE-S20-01 | File-based locking with `Drop` guard for parallel builds | `LockGuard` pattern enforced |
| 21 | RULE-S20-02 | Atomic build publication via rename | `sibling_tmp_dir` + `std::fs::rename` |
| 22 | RULE-S14-01 | Lock file guard via `Drop` impl, never manual unlock | Code review: no manual `unlock()` calls |
| 23 | RULE-S14-02 | Config load trigger must remain post-identify | Startup sequence test |
| 24 | RULE-S22-01 | Test harnesses use `-f /dev/null` for config isolation | Test fixture enforcement |
| 25 | RULE-S18-01 | Permission hardening: socket dir `0700`, socket `0600` | Test assertions |
| 26 | RULE-S3-03 | `mux-termlet` must not depend on `mux-server` or `mux-api` | `cargo tree` CI check |
| 27 | RULE-S4-03 | `mux-termlet` lives at Layer 2, depends on Layer 0 + Layer 1 only | `cargo deny check` |
| 28 | RULE-S16-03 | Termlet bindings expose: spawn, send_keys, wait_for, snapshot, resize, kill | API surface test |
| 29 | RULE-S22-03 | Termlet fixtures must auto-kill on cleanup | Leak detection test |
| 30 | RULE-S22-04 | Termlet tests must run in both real and fake PTY modes | Parametrized fixture |
| 31 | RULE-S32-01 | Termlet snapshot text must be trailing-whitespace-trimmed | Snapshot format test |
| 32 | RULE-S32-02 | Termlet kill must be idempotent | Double-kill test |
| 33 | RULE-S32-03 | Termlet Drop must send SIGKILL to prevent PTY process leaks | Drop impl audit |
| 34 | RULE-S32-04 | FakePtyBackend tests must not depend on timing | Determinism check |
| 35 | RULE-S32-05 | Termlet bindings must implement context manager / cleanup pattern | Binding API test |
| 36 | RULE-S32-06 | `mux-termlet` must not depend on `mux-server`, `mux-api`, or `mux-orm` | `cargo tree` CI |
| 37 | RULE-S32-07 | Every Termlet API method tested in Rust + at least one binding | Test coverage |
| 38 | RULE-S8-03 | Termlet error codes must be stable string constants | Semver check |
| 39 | RULE-S32-08 | `wait_for` must compile pattern once, not per poll iteration | PatternMatcher audit |
| 40 | RULE-S32-09 | Termlet remains pane-backed; never invent parallel terminal core | Architecture review |
| 41 | RULE-S32-10 | Cross-language snapshot consistency tests must exist for every release | Multi-runner CI |
| 42 | RULE-S02-03 | Gate pass/fail thresholds must be measurable, not subjective | Acceptance criteria table |
| 43 | RULE-S32-11 | `TermletBuilder` and `TermletConfig` must produce identical Termlets | Unit test |
| 44 | RULE-S32-12 | `SnapshotDiff` must not allocate on identical snapshots | Benchmark |
| 45 | RULE-S32-13 | **[v11]** `TermletPool` Drop must kill all managed Termlets | Unit test |
| 46 | RULE-S32-14 | **[v11]** `wait_for_async` is feature-gated under `async` | CI feature check |
| 47 | RULE-S32-15 | **[v11]** `RealPtyBackend` must use non-blocking IO for event reading | Architecture review |

### 26.2 Anti-Patterns (Forbidden)

| Pattern | Why Forbidden | Alternative |
|---|---|---|
| `Arc<Mutex<_>>` on read path | Contention under concurrent reads | `ArcSwap` |
| Direct graph mutation outside `apply_event` | Non-deterministic state transitions | Submit `Event` |
| `unwrap()` in non-test code | Panic risk in production | `?` operator or explicit error handling |
| `unsafe` outside `mux-os` | Purity violation of Layer 0 contract | `mux-os` API |
| Manual `unlock()` call | Resource leak risk on early return | `Drop` guard |
| `std::fs::copy` for build output | Race condition with concurrent readers | Atomic `std::fs::rename` |
| `sleep()` in tests for synchronization | Flaky tests | `wait_for_socket()` or condition variable |
| `anyhow::Error` in library crate public API | Erases error classification | `thiserror` typed errors |
| Inline SQL-style queries | Not applicable to entity graph | `QueryList` + `QueryOp` |
| Termlet without `kill()` call | Leaked PTY processes | Context manager / `Drop` impl |
| `std::thread::sleep` in `wait_for` loop body | Busy-wait wastes CPU | Exponential backoff with configurable interval |
| Re-compiling pattern on each `wait_for` poll | Unnecessary allocation | `PatternMatcher::compile()` once |

### Test Strategy

1. Static rule checks in CI (grep/cargo-deny/compile-fail suites).
2. Rule coverage report links each rule to at least one automated test.
3. Release gate fails when critical rules are unverified.
4. Anti-pattern grep checks run on every PR.

### AGENTS.md Rules

- `RULE-S26-01`: Every architecture section must define enforceable rules.
- `RULE-S26-02`: Rule IDs are immutable once published.

---

## 27. Risks and Mitigations

### Design Decisions

Risk register covers protocol drift, behavioral divergence, binding API stability, test isolation, observability, dependency management, and Termlet-specific risks.

### 27.1 Risk Table

| # | Risk | Impact | Probability | Mitigation |
|---|---|---|---|---|
| R1 | Protocol edge cases cause incompatibility | High | Medium | Fuzz testing + real tmux fixture captures |
| R2 | VT100 parser divergence from tmux `input.c` | High | Medium | State table generated from tmux source |
| R3 | SlotMap generation overflow | Low | Very Low | 2^32 generations; assert in tests |
| R4 | Format string expansion mismatch | Medium | High | `format-audit` parity corpus |
| R5 | Key binding table divergence | Medium | Medium | Generated parity tests from `key-bindings.c` |
| R6 | PyO3 version incompatibility | Medium | Low | Pin PyO3 version, test in CI |
| R7 | Neon API churn | Medium | Medium | Abstraction layer between Neon and our code |
| R8 | CRDT merge conflicts with stateful operations | High | Low | Property testing with concurrent operations |
| R9 | Performance regression in VT100 parser | Medium | Medium | Criterion benchmarks in CI with threshold |
| R10 | ArcSwap contention under extreme write load | Low | Low | Benchmark; fallback to channel-based notification |
| R11 | Tokio version incompatibility with OTEL SDK | Medium | Low | Pin compatible versions in workspace |
| R12 | SCM_RIGHTS not available on all platforms | Medium | Medium | Feature-gate behind `unix_socket` |
| R13 | FakePty divergence from real PTY behavior | High | Medium | Periodically regenerate fixtures from real tmux |
| R14 | Config parser not covering all tmux options | High | High | `tmux-command-audit` coverage tracking |
| R15 | Layout checksum mismatch with real tmux | Medium | Low | Byte-exact test against tmux captures |
| R16 | Copy mode key handling edge cases | Medium | Medium | Differential testing against real tmux copy mode |
| R17 | ratatui API breaking changes | Medium | Low | Pin version, abstraction layer |
| R18 | Thread-local OTEL headers lost across thread boundaries | Medium | Medium | Capture+push pattern enforced |
| R19 | Multiple tmux versions in test matrix | Low | High | Version matrix in CI |
| R20 | Clipboard security in tests | Low | Medium | `set-clipboard off` in all test harnesses |
| R21 | Lock file stale after crash | Low | Low | `flock` auto-releases on process death |
| R22 | Concurrent cargo test corrupts build cache | Medium | Medium | File-based locking per cache key |
| R23 | Socket path exceeds 108-byte limit | Medium | Low | Path length check before socket creation |
| R24 | Control mode notification ordering | Medium | Medium | Sequence numbers + periodic refresh |
| R25 | CRDT OpLog unbounded growth | Medium | Low | Periodic compaction |
| R26 | Python GIL blocking during Rust operations | Medium | Medium | Release GIL with `py.allow_threads()` |
| R27 | Node.js async operation cancellation | Low | Medium | Neon task cancellation handling |
| R28 | tmux protocol version negotiation | Medium | Low | Version field in identify burst |
| R29 | WASM CI gate false positive from dev dependencies | Low | Medium | Feature-gated dev-deps |
| R30 | OTEL tokio runtime lifecycle blocking process exit | Medium | Medium | Bounded `shutdown_timeout(200ms)` in OtelProvider Drop |
| R31 | Termlet PTY process leak on crash/panic | High | Medium | `Drop` impl that kills PTY, RAII guard pattern |
| R32 | FakePty Termlet diverges from real PTY Termlet | Medium | High | Dual-mode parametrized tests, periodic fixture regen |
| R33 | Termlet `wait_for` busy-loop exhausts CPU | Medium | Low | Exponential backoff with configurable poll interval |
| R34 | Binding Termlet GC timing causes delayed kill | Medium | Medium | Explicit `kill()` in fixture teardown, not relying on GC |
| R35 | Termlet grid reflow on resize loses scroll history | Low | Medium | Document limitation; snapshot before resize if needed |
| R36 | Termlet API semantics diverge across languages | Medium | Medium | Cross-language consistency tests |
| R37 | Compiled PatternMatcher cache grows unbounded | Low | Low | LRU eviction or per-Termlet scope |
| R38 | TermletBuilder defaults diverge from TermletConfig defaults | Low | Low | Single source of defaults, unit test parity |
| R39 | SnapshotDiff false positives from trailing whitespace | Low | Medium | Normalize whitespace before diff |

### Test Strategy

1. Risk-to-test mapping must be complete and machine-checked.
2. High-impact risks require at least one integration/parity test.
3. Quarterly risk review updates with regression evidence.
4. Termlet-specific risks R31-R37 tested via dedicated Termlet test suite.
5. Risks R38-R39 tested via builder/diff unit tests.

### AGENTS.md Rules

- `RULE-S27-01`: New high-risk features must add mitigation tests before merge.
- `RULE-S27-02`: Risk register IDs remain stable for auditability.

---

## 28. Plan Evolution and Changelog

### Design Decisions

This section documents the evolution from v4 through v11 Refined.

### 28.1 Version History

| Version | Date | Key Changes |
|---|---|---|
| v4-v9 | 2026-02-10 | Evolution from 6-model synthesis to definitive v9 spec. |
| v10 Pass 1 | 2026-02-11 | Introduction of Termlets (Section 32). |
| v10 Pass 2 | 2026-02-11 | Refinement with PatternMatcher, WaitMatch, structured errors, cross-lang parity. |
| v10 Pass 3 Final | 2026-02-11 | Definitive synthesis. TermletBuilder, TermletPool, SnapshotDiff, async support. |
| **v11 Refined** | **2026-02-11** | **v11 Refined (Gemini).** `inherit_env` for Termlets, `SnapshotDiff` context, async Pool methods, non-blocking IO clarification. |

### 28.2 Changelog v11 Refined (Gemini)

1.  **Termlet Environment Inheritance:** Added `inherit_env` to `TermletConfig` (Section 32.2). Default is `false` (hermetic), but allows opt-in for integration tests needing host env.
2.  **SnapshotDiff Context:** Added `context_lines` to `SnapshotDiff::compare` (Section 32.11) for better debug output.
3.  **Async Pool:** Added `spawn_async` and `get_async` to `TermletPool` (Section 32.13).
4.  **Non-blocking IO:** Clarified `RealPtyBackend` must use non-blocking IO (Section 32.1).
5.  **Resize Safety:** `Termlet::resize` drains output before and after backend resize (Section 32.2).
6.  **LLM Hints:** Added Section 32.14 "Implementation Hints for LLMs".

### Rust Example

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecVersion {
    V10Pass3Final, V11Refined,
}

pub struct ChangelogItem {
    pub from: SpecVersion,
    pub to: SpecVersion,
    pub summary: &'static str,
}

pub const V11_CHANGES: &[ChangelogItem] = &[
    ChangelogItem { from: SpecVersion::V10Pass3Final, to: SpecVersion::V11Refined,
        summary: "Refined Termlet env inheritance, SnapshotDiff context, \
            Async Pool support, and non-blocking IO requirements." },
];
```

### Test Strategy

1. Changelog assertions reference concrete tests and anchors.
2. No unresolved placeholders allowed in release spec.
3. Spec self-check script validates section count and required subsections.

### AGENTS.md Rules

- `RULE-S28-01`: Each version bump must list behavior-impacting changes.
- `RULE-S28-02`: No TODO placeholders in finalized spec versions.

---

## 29. Reference Anchors

### Design Decisions

All compatibility claims in this specification are anchored to verified source code.

### 29.1 tmux Source Code References

| File | Line(s) | Topic | Verified v10 P3 |
|---|---|---|---|
| `tmux-protocol.h` | 23 | Protocol version 8 | Yes |
| `client.c` | 77-101 | flock locking (line 89 confirmed) | Yes |
| `server-client.c` | 3472-3475 | Protocol violation kills connection | Yes |
| `server-client.c` | 3725-3734 | Config loaded after identify | Yes |
| `options.c` | 228-241 | Option resolution chain | Yes |
| `options.c` | 891-903 | WindowPane FALLTHROUGH | Yes |
| `options.c` | 1269-1285 | Unset semantics | Yes |
| `layout.c` | 448-462 | Round-robin resize | Yes |
| `layout.c` | 937-950 | Split minimum | Yes |
| `layout-custom.c` | 46-57 | Layout checksum | Yes |
| `layout-custom.c` | 119-153 | Layout validation | Yes |
| `input.c` | State table | VT100 parser | Yes |
| `key-bindings.c` | Default table | Key binding defaults | Yes |
| `control.c` | 450-461 | Control mode pending limit | Yes |
| `control.c` | 620-623 | Extended output format | Yes |
| `control.c` | 758-796 | Control mode startup | Yes |
| `tmux.h` | 100 | PANE_MINIMUM | Yes |

### 29.2 Reference Codebase Verification (Re-verified v10 Pass 3)

| Codebase | File | Claims Verified |
|---|---|---|
| libtmux | `_internal/query_list.py` | 12 operators, `keygetter`, exceptions |
| vibe-tmux | `path_guard.rs` | 3-layer socket validation |
| vibe-tmux | `tmux-builder/src/lib.rs` | BLAKE3 cache, file locking, 12-char trunc |
| vibe-tmux | `mux-otel/src/otel.rs` | Dual providers, composite propagator |

### Rust Example

```rust
pub struct Anchor {
    pub claim: &'static str,
    pub source: &'static str,
    pub file: &'static str,
    pub lines: Option<&'static str>,
    pub v10p3_verified: bool,
}

pub const ANCHORS: &[Anchor] = &[
    Anchor { claim: "protocol version 8",
        source: "tmux", file: "tmux-protocol.h", lines: Some("23"),
        v10p3_verified: true },
    Anchor { claim: "flock startup lock",
        source: "tmux", file: "client.c", lines: Some("77-101"),
        v10p3_verified: true },
];
```

### Test Strategy

1. Anchor checker script confirms referenced files exist.
2. Drift checker flags missing/renamed files.
3. Parity suite maps failing behavior to anchor category.

### AGENTS.md Rules

- `RULE-S29-01`: Major compatibility claims require source anchor entries.
- `RULE-S29-02`: Remove stale anchors during refactors.

---

## 30. Appendix: Canonical Type Quick Reference

### Design Decisions

This appendix provides a consolidated reference for all canonical type names used across the specification.

### 30.1 Entity IDs

```rust
slotmap::new_key_type! {
    pub struct SessionId;
    pub struct WindowId;
    pub struct PaneId;
    pub struct ClientId;
    pub struct JobId;
    pub struct BufferId;
}

// Termlet uses a simple counter, not SlotMap
pub type TermletId = u64;
```

### 30.2 Core Types

| Type | Crate | Purpose |
|---|---|---|
| `ServerGraph` | mux-core | Authoritative mutable state |
| `GraphState` | mux-core | Immutable snapshot with reverse maps |
| `Event` | mux-core | Inbound state change request |
| `Effect` | mux-core | Outbound side-effect request |
| `CoreCtx` | mux-core | Injected time + randomness |
| `ApplyOutcome` | mux-core | Return from apply_event (effects + hints) |
| `Grid` | mux-grid | Terminal cell grid |
| `VtParser` | mux-grid | VT100 state machine |
| `ImsgHdr` | mux-proto | Wire protocol header |
| `ImsgFrame` | mux-proto | Header + payload |
| `ImsgCodec` | mux-proto | Stateful encoder/decoder (tokio-codec compatible) |
| `MsgType` | mux-proto | Protocol message type enum |
| `OptionSet` | mux-types | Key-value option store |
| `OptionValue` | mux-types | Typed option value |
| `OptionScope` | mux-types | Server/Session/Window/Pane |
| `LayoutTree` | mux-core | Arena-based layout |
| `LayoutCell` | mux-core | Single layout node |
| `QueryOp` | mux-query | Filter operator (18 variants) |
| `QueryList<T>` | mux-query | Filterable collection |
| `QuerySpec` | mux-query | Field + operator + value |
| `QueryValue` | mux-query | String / Int / Bool / List / None |
| `HLC` | mux-crdt | Hybrid logical clock |
| `LWWRegister<T>` | mux-crdt | Last-writer-wins register |
| `OrSet<T>` | mux-crdt | Observed-remove set (add-wins) |
| `OpLog` | mux-crdt | Operation log for merge/replay |
| `ControlNotification` | mux-control | Typed control mode notification |
| `PtyBackend` | mux-pty | PTY abstraction trait |
| `FakePtyBackend` | mux-pty-fake | Test PTY implementation |
| `StateHandle` | mux-api | ArcSwap-based read handle |
| `ManagedMux` | mux-api | Server lifecycle manager |
| `TermForgeStack` | mux-api | Composition for in-process embedding |
| `OtelProvider` | mux-otel | OTEL trace + log provider |
| `TraceHeaders` | mux-otel | traceparent + tracestate + baggage |
| `TraceHeadersGuard` | mux-otel | RAII guard for thread-local headers |
| `PathGuard` | mux-test-support | Socket isolation for tests |
| `LockGuard` | tools/tmux-builder | File lock with Drop |
| `ViewModel` | mux-view | Pure view model for TUI |
| **`Termlet`** | **mux-termlet** | **SDK-first testing pod handle** |
| **`TermletConfig`** | **mux-termlet** | **Configuration for Termlet spawn** |
| **`TermletSnapshot`** | **mux-termlet** | **Captured grid state** |
| **`PtyMode`** | **mux-termlet** | **Real vs Fake PTY backend selection** |
| **`WaitMatch`** | **mux-termlet** | **Match result with byte range + timestamp** |
| **`PatternMatcher`** | **mux-termlet** | **Compiled pattern for efficient polling** |
| **`ScenarioStep`** | **mux-pty-fake** | **Structured FakePty fixture step** |
| **`TermletViewModel`** | **mux-view** | **Debug inspector panel model** |
| **`TermletBuilder`** | **mux-termlet** | **Fluent construction for Termlets** |
| **`TermletPool`** | **mux-termlet** | **Batch Termlet management for test suites** |
| **`SnapshotDiff`** | **mux-termlet** | **Visual regression diff between snapshots** |

### 30.3 Error Types

| Type | Crate | ErrorClass |
|---|---|---|
| `ProtocolError` | mux-proto | ProtocolViolation |
| `CoreError` | mux-core | UserError / Bug |
| `ConfigError` | mux-conf | UserError |
| `LayoutError` | mux-core | Bug |
| `QueryError` | mux-query | UserError |
| `PtyError` | mux-pty | Transient |
| `SocketError` | mux-os | Transient / UserError |
| `LockError` | mux-os | Transient |
| `ControlParseError` | mux-control | UserError |
| **`TermletError`** | **mux-termlet** | **Transient / UserError** |

### Test Strategy

1. Type API compile checks in integration crates.
2. Semver surface snapshot for public crates.
3. Doc tests for all appendix examples.

### AGENTS.md Rules

- `RULE-S30-01`: Do not rename canonical types without migration notes.
- `RULE-S30-02`: Appendix must match actual exported APIs.

---

## 31. Supplemental Test Matrix

### Design Decisions

Test matrix extends section-local tests with release gates.

### 31.1 Test Classification

| Category | Count | Runner | CI? |
|---|---|---|---|
| Unit (pure core) | ~500 | `cargo test` | Yes |
| Property (proptest) | ~50 | `cargo test` | Yes |
| Snapshot (insta) | ~100 | `cargo test` | Yes |
| Protocol fixtures | ~50 | `cargo test` | Yes |
| FakePty scenarios | ~30 | `cargo test` | Yes |
| Integration (in-process) | ~100 | `cargo test` | Yes |
| Integration (subprocess) | ~50 | `cargo test` | Yes, separate job |
| Parity (real tmux) | ~200 | `mux-regress` | Yes, matrix |
| Python binding | ~80 | `pytest` | Yes |
| Node binding | ~40 | `vitest` | Yes |
| Performance | ~10 | `criterion` | Yes (warn only) |
| Fuzz | 3 targets | `cargo fuzz` | Nightly |
| WASM purity | 1 | `cargo check --target wasm32-unknown-unknown` | Yes |
| **Termlet unit (Rust)** | ~30 | `cargo test` | Yes |
| **Termlet integration** | ~20 | `cargo test` | Yes |
| **Termlet Python** | ~15 | `pytest` | Yes |
| **Termlet Node** | ~10 | `vitest` | Yes |
| **Termlet performance** | ~3 | `criterion` | Yes (warn only) |
| **Cross-language consistency** | ~10 | `pytest` + `vitest` + `cargo test` | Yes |
| **TermletBuilder parity** | ~5 | `cargo test` | Yes |
| **SnapshotDiff regression** | ~10 | `cargo test` | Yes |

### 31.2 CI Matrix

| Axis | Values |
|---|---|
| OS | Ubuntu 24.04, macOS 14 |
| Rust | stable, nightly |
| tmux version | 3.3a, 3.4, 3.5, 3.6 |
| Python | 3.11, 3.12, 3.13 |
| Node | 20, 22 |

### 31.3 Test Naming Convention

```
test_{layer}_{component}_{behavior}

Examples:
  test_termlet_spawn_fake
  test_termlet_spawn_real
  test_termlet_send_keys_echo
  test_termlet_wait_for_timeout
  test_termlet_wait_for_pattern_not_found
  test_termlet_snapshot_text
  test_termlet_resize_reflow
  test_termlet_kill_idempotent
  test_termlet_drop_kills_pty
  test_termlet_pattern_matcher_compile
  test_termlet_builder_parity
  test_termlet_pool_batch
  test_termlet_snapshot_diff
  test_binding_py_termlet_fixture
  test_binding_node_termlet_helper
  test_cross_lang_termlet_snapshot
```

### 31.4 Coverage Requirements

| Crate | Target | Enforcement |
|---|---|---|
| mux-termlet | 85% | CI gate |
| bindings/python | 80% | CI gate |

### 31.5 Release Gate Matrix

| Gate ID | Gate | Test Type | Required? |
|---|---|---|---|
| M-32-01 | Termlet spawn (fake+real) | unit test | Yes |
| M-32-02 | Termlet send_keys + snapshot | unit test | Yes |
| M-32-03 | Termlet resize + reflow | unit test | Yes |
| M-32-04 | PyTermlet fixture chain | pytest | Yes |
| M-32-05 | JsTermlet helper | vitest | Yes |
| M-32-06 | Cross-language snapshot consistency | multi-runner | Yes |
| M-32-07 | PatternMatcher compile efficiency | unit test | Yes |
| M-32-08 | TermletBuilder produces same Termlet as TermletConfig | unit test | Yes |
| M-32-09 | SnapshotDiff detects known differences | unit test | Yes |

### Rust Example

```rust
pub struct MatrixRow {
    pub id: &'static str,
    pub section: u8,
    pub test_name: &'static str,
    pub required: bool,
}

pub const MATRIX: &[MatrixRow] = &[
    MatrixRow { id: "M-32-01", section: 32, test_name: "termlet_spawn_dual_mode", required: true },
];
```

### Test Strategy

1. CI requires 100% pass of mandatory matrix rows.
2. Nightly includes extended parity + fuzz durations.
3. Release requires zero unresolved mandatory rows.
4. Termlet gates M-32-01 through M-32-09 are mandatory for release.

### AGENTS.md Rules

- `RULE-S31-01`: New architecture features must add at least one matrix row.
- `RULE-S31-02`: Release tagging is blocked until mandatory rows are green.

---

## 32. Termlets [v11 Refined]

### Design Decisions

Termlets are the killer feature differentiating TermForge from other terminal multiplexers and testing tools. They are **SDK-first testing pods** that wrap tmux panes into simplified, embeddable, language-binding-native handles.

**Core philosophy:** A Termlet is to a tmux pane what a Docker container is to a VM -- same capability, simpler interface, faster lifecycle, purpose-built for programmatic use.

**Key design decisions:**

1. **Testing pods**: A Termlet wraps a Pane + Grid + PtyBackend into a single ergonomic handle.
2. **SDK-first**: The API is designed for programmatic use.
3. **Visual area captured**: Each Termlet owns a `Grid` that accumulates VT100 output.
4. **Resizable**: Programmatic resize via `resize(cols, rows)`.
5. **Interactive**: Accepts input via `send_keys()`.
6. **Available everywhere**: Rust core, Python, Node.js.
7. **Backward compatible**: A Termlet IS a pane under the hood.
8. **Process management**: Spawn shells, run commands, send signals, detect exit.
9. **The ultimate subprocess runner**: Full terminal emulation for interactive testing.
10. **Lite**: Minimal overhead.
11. **Snapshot-testable**: `insta::assert_snapshot!` compatible.
12. **Dual-mode**: `PtyMode::Real` and `PtyMode::Fake`.
13. **Compiled pattern matching**: `PatternMatcher` for efficient polling.
14. **Structured wait result**: `WaitMatch` carrying byte range and timestamp.
15. **Fluent construction**: `TermletBuilder`.
16. **Batch management**: `TermletPool`.
17. **Visual regression**: `SnapshotDiff`.

**Architectural position:** `mux-termlet` sits at Layer 2 (FACADE).

### 32.1 Termlet Architecture

```
                    +------------------+
                    |   User Code      |
                    | (Rust / Py / JS) |
                    +--------+---------+
                             |
                    spawn / send_keys / snapshot / resize / kill
                             |
                    +--------v---------+
                    |     Termlet      |  <-- mux-termlet crate
                    |  (single handle) |
                    +--------+---------+
                             |
            +----------------+----------------+
            |                |                |
    +-------v------+  +-----v------+  +------v------+
    |  VtParser    |  |    Grid    |  | PtyBackend  |
    | (mux-grid)   |  | (mux-grid) |  | (mux-pty)   |
    +--------------+  +------------+  +------+------+
                                             |
                            +----------------+----------------+
                            |                                 |
                    +-------v-------+               +--------v--------+
                    | RealPtyBackend|               | FakePtyBackend  |
                    |  (mux-pty)    |               | (mux-pty-fake)  |
                    +---------------+               +-----------------+
```

**Implementation Note:** `RealPtyBackend` must use non-blocking IO for `read_events` to prevent `wait_for` (which loops and drains output) from blocking indefinitely if the PTY has no output available.

### 32.2 Core API

```rust
// crates/mux-termlet/src/lib.rs

/// Configuration for spawning a Termlet.
#[derive(Debug, Clone)]
pub struct TermletConfig {
    /// Terminal width in columns.
    pub cols: u16,
    /// Terminal height in rows.
    pub rows: u16,
    /// Environment variables to set in the spawned process.
    pub env: Vec<(String, String)>,
    /// [v11] Whether to inherit the parent process environment.
    /// Default: false (hermetic).
    pub inherit_env: bool,
    /// Working directory for the spawned process.
    pub cwd: Option<String>,
    /// PTY backend mode: real or fake.
    pub backend: PtyMode,
    /// Poll interval for wait_for (default 25ms).
    pub wait_poll_interval: std::time::Duration,
    /// Grace period between SIGTERM and SIGKILL during shutdown.
    pub grace_period: std::time::Duration,
}

impl Default for TermletConfig {
    fn default() -> Self {
        Self {
            cols: 80,
            rows: 24,
            env: vec![("TERM".to_string(), "xterm-256color".to_string())],
            inherit_env: false,
            cwd: None,
            backend: PtyMode::Real,
            wait_poll_interval: std::time::Duration::from_millis(25),
            grace_period: std::time::Duration::from_secs(2),
        }
    }
}

/// Select between real and fake PTY backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtyMode {
    Real,
    Fake,
}

/// Compiled pattern for efficient wait_for polling.
pub enum PatternMatcher {
    Plain(String),
    Regex(regex::Regex),
}

impl PatternMatcher {
    pub fn compile(pattern: &str) -> Result<Self, TermletError> {
        if let Some(re_pat) = pattern.strip_prefix("re:") {
            let re = regex::Regex::new(re_pat)
                .map_err(|e| TermletError::SpawnFailed {
                    reason: format!("invalid regex: {e}"),
                })?;
            Ok(PatternMatcher::Regex(re))
        } else {
            Ok(PatternMatcher::Plain(pattern.to_string()))
        }
    }

    pub fn find(&self, text: &str) -> Option<std::ops::Range<usize>> {
        match self {
            PatternMatcher::Plain(s) => text.find(s).map(|start| start..start + s.len()),
            PatternMatcher::Regex(re) => re.find(text).map(|m| m.start()..m.end()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct WaitMatch {
    pub byte_range: std::ops::Range<usize>,
    pub matched_at: std::time::Instant,
}

pub struct Termlet {
    grid: Grid,
    parser: VtParser,
    backend: Box<dyn PtyBackend>,
    pane_id: PaneId,
    size: PaneSize,
    exited: bool,
    id: TermletId,
    wait_poll_interval: std::time::Duration,
    grace_period: std::time::Duration,
}

static NEXT_TERMLET_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

impl Termlet {
    pub fn spawn(command: &str, mut config: TermletConfig) -> Result<Self, TermletError> {
        let id = NEXT_TERMLET_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let size = PaneSize { sx: config.cols, sy: config.rows };
        let grid = Grid::new(config.cols, config.rows);
        let parser = VtParser::new();
        let wait_poll_interval = config.wait_poll_interval;
        let grace_period = config.grace_period;

        let pane_id = PaneId::from(slotmap::KeyData::from_ffi(id));

        let mut backend: Box<dyn PtyBackend> = match config.backend {
            PtyMode::Real => Box::new(RealPtyBackend::new()?),
            PtyMode::Fake => Box::new(FakePtyBackend::new()),
        };

        if config.inherit_env {
            for (k, v) in std::env::vars() {
                if !config.env.iter().any(|(ek, _)| ek == &k) {
                    config.env.push((k, v));
                }
            }
        }

        let cmd_parts = vec![command.to_string()];
        backend.spawn(
            pane_id,
            &cmd_parts,
            config.cwd.as_deref(),
            size,
        ).map_err(|e| TermletError::SpawnFailed {
            reason: e.to_string(),
        })?;

        Ok(Self {
            grid, parser, backend, pane_id, size,
            exited: false, id, wait_poll_interval, grace_period,
        })
    }

    pub fn send_keys(&mut self, keys: &str) -> Result<(), TermletError> {
        if self.exited {
            return Err(TermletError::AlreadyKilled);
        }
        self.backend.write(self.pane_id, keys.as_bytes())?;
        self.drain_output();
        Ok(())
    }

    pub fn wait_for(
        &mut self,
        pattern: &str,
        timeout: std::time::Duration,
    ) -> Result<WaitMatch, TermletError> {
        let compiled = PatternMatcher::compile(pattern)?;
        let start = std::time::Instant::now();
        let mut delay = self.wait_poll_interval;
        let max_delay = std::time::Duration::from_millis(100);

        if timeout.is_zero() {
            self.drain_output();
            let text = self.grid.to_text();
            if let Some(range) = compiled.find(&text) {
                return Ok(WaitMatch {
                    byte_range: range,
                    matched_at: std::time::Instant::now(),
                });
            }
            return Err(TermletError::WaitForTimeout {
                pattern: pattern.to_string(),
                timeout_ms: 0,
            });
        }

        loop {
            self.drain_output();
            let text = self.grid.to_text();
            if let Some(range) = compiled.find(&text) {
                return Ok(WaitMatch {
                    byte_range: range,
                    matched_at: std::time::Instant::now(),
                });
            }

            if self.exited {
                return Err(TermletError::PatternNotFound {
                    pattern: pattern.to_string(),
                });
            }

            if start.elapsed() > timeout {
                return Err(TermletError::WaitForTimeout {
                    pattern: pattern.to_string(),
                    timeout_ms: timeout.as_millis() as u64,
                });
            }

            let remaining = timeout.saturating_sub(start.elapsed());
            let sleep_for = delay.min(max_delay).min(remaining);
            std::thread::sleep(sleep_for);
            delay = (delay * 2).min(max_delay);
        }
    }

    pub fn snapshot(&mut self) -> TermletSnapshot {
        self.drain_output();
        TermletSnapshot {
            grid: self.grid.clone(),
            cols: self.size.sx,
            rows: self.size.sy,
        }
    }

    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<(), TermletError> {
        if self.exited {
            return Err(TermletError::AlreadyKilled);
        }
        // Drain before resize to process any pending output
        self.drain_output();
        
        let new_size = PaneSize { sx: cols, sy: rows };
        self.backend.resize(self.pane_id, new_size)?;
        self.grid.resize(cols, rows);
        self.size = new_size;
        
        // Drain after resize to catch immediate SIGWINCH response
        self.drain_output();
        Ok(())
    }

    pub fn kill(&mut self) -> Result<(), TermletError> {
        if self.exited {
            return Ok(()); // Idempotent
        }
        self.backend.kill(self.pane_id, libc::SIGTERM)?;
        self.exited = true;
        Ok(())
    }

    pub fn is_alive(&self) -> bool { !self.exited }

    pub fn size(&self) -> PaneSize { self.size }

    pub fn id(&self) -> TermletId { self.id }

    pub fn pane_id(&self) -> PaneId { self.pane_id }

    fn drain_output(&mut self) {
        let events = self.backend.read_events();
        for event in events {
            match event {
                PtyEvent::Output { data, .. } => {
                    self.parser.parse(&data, &mut self.grid);
                }
                PtyEvent::Exited { .. } => {
                    self.exited = true;
                }
                _ => {}
            }
        }
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn fake_backend_mut(&mut self) -> Option<&mut FakePtyBackend> {
        self.backend.as_any_mut().downcast_mut::<FakePtyBackend>()
    }
}

impl Drop for Termlet {
    fn drop(&mut self) {
        if !self.exited {
            let _ = self.backend.kill(self.pane_id, libc::SIGKILL);
        }
    }
}
```

### 32.3 TermletSnapshot

```rust
// crates/mux-termlet/src/snapshot.rs

#[derive(Debug, Clone)]
pub struct TermletSnapshot {
    grid: Grid,
    cols: u16,
    rows: u16,
}

impl TermletSnapshot {
    pub fn to_text(&self) -> String {
        let mut lines: Vec<String> = (0..self.rows)
            .map(|row| self.grid.row_text(row as usize).trim_end().to_string())
            .collect();
        while lines.last().is_some_and(|l| l.is_empty()) {
            lines.pop();
        }
        lines.join("\n")
    }

    pub fn to_styled(&self) -> String {
        let mut output = String::new();
        for row in 0..self.rows {
            for col in 0..self.cols {
                let cell = self.grid.cell(row as usize, col as usize);
                write_ansi_cell(&mut output, cell);
            }
            output.push('\n');
        }
        output
    }

    pub fn to_cells(&self) -> Vec<Vec<Cell>> {
        (0..self.rows)
            .map(|row| {
                (0..self.cols)
                    .map(|col| self.grid.cell(row as usize, col as usize).clone())
                    .collect()
            })
            .collect()
    }

    pub fn size(&self) -> (u16, u16) { (self.cols, self.rows) }

    pub fn contains(&self, pattern: &str) -> bool { self.to_text().contains(pattern) }

    pub fn row(&self, idx: usize) -> String {
        self.grid.row_text(idx).trim_end().to_string()
    }
}
```

### 32.8 Error Code Reference

| Error Variant | Code String | Python Exception | Node Error |
|---|---|---|---|
| `SpawnFailed` | `TERMLET_SPAWN_ERROR` | `RuntimeError` | `Error` |
| `AlreadyKilled` | `TERMLET_ALREADY_KILLED` | `RuntimeError` | `Error` |
| `WaitForTimeout` | `TERMLET_TIMEOUT` | `TimeoutError` | `Error` (code: TIMEOUT) |
| `PatternNotFound` | `TERMLET_PATTERN_NOT_FOUND` | `RuntimeError` | `Error` (code: NOT_FOUND) |
| `ResizeFailed` | `TERMLET_RESIZE_ERROR` | `RuntimeError` | `Error` |
| `Pty(...)` | `TERMLET_PTY_ERROR` | `RuntimeError` | `Error` |

### 32.9 TermletBuilder

Fluent builder pattern for ergonomic Termlet construction.

```rust
// crates/mux-termlet/src/builder.rs

pub struct TermletBuilder {
    command: String,
    config: TermletConfig,
}

impl TermletBuilder {
    pub fn new(command: &str) -> Self {
        Self {
            command: command.to_string(),
            config: TermletConfig::default(),
        }
    }

    pub fn cols(mut self, cols: u16) -> Self { self.config.cols = cols; self }
    pub fn rows(mut self, rows: u16) -> Self { self.config.rows = rows; self }
    pub fn size(mut self, cols: u16, rows: u16) -> Self {
        self.config.cols = cols;
        self.config.rows = rows;
        self
    }
    pub fn env(mut self, key: &str, value: &str) -> Self {
        self.config.env.push((key.to_string(), value.to_string()));
        self
    }
    /// [v11] Enable environment inheritance.
    pub fn inherit_env(mut self) -> Self {
        self.config.inherit_env = true;
        self
    }
    pub fn cwd(mut self, cwd: &str) -> Self { self.config.cwd = Some(cwd.to_string()); self }
    pub fn fake(mut self) -> Self { self.config.backend = PtyMode::Fake; self }
    pub fn real(mut self) -> Self { self.config.backend = PtyMode::Real; self }
    pub fn poll_interval(mut self, interval: std::time::Duration) -> Self {
        self.config.wait_poll_interval = interval;
        self
    }
    pub fn grace_period(mut self, period: std::time::Duration) -> Self {
        self.config.grace_period = period;
        self
    }

    pub fn spawn(self) -> Result<Termlet, TermletError> {
        Termlet::spawn(&self.command, self.config)
    }
}
```

### 32.10 TermletPool

Batch Termlet management for test suites.

```rust
// crates/mux-termlet/src/pool.rs

pub struct TermletPool {
    termlets: std::collections::HashMap<String, Termlet>,
}

impl TermletPool {
    pub fn new() -> Self {
        Self { termlets: std::collections::HashMap::new() }
    }

    pub fn spawn(
        &mut self,
        name: &str,
        command: &str,
        config: TermletConfig,
    ) -> Result<(), TermletError> {
        let termlet = Termlet::spawn(command, config)?;
        self.termlets.insert(name.to_string(), termlet);
        Ok(())
    }

    pub fn get_mut(&mut self, name: &str) -> Result<&mut Termlet, TermletError> {
        self.termlets.get_mut(name).ok_or(TermletError::SpawnFailed {
            reason: format!("termlet not found in pool: {name}"),
        })
    }

    pub fn get(&self, name: &str) -> Option<&Termlet> {
        self.termlets.get(name)
    }

    pub fn kill_all(&mut self) {
        for (_, termlet) in self.termlets.iter_mut() {
            let _ = termlet.kill();
        }
    }

    pub fn snapshot_all(&mut self) -> std::collections::HashMap<String, String> {
        self.termlets.iter_mut()
            .map(|(name, termlet)| (name.clone(), termlet.snapshot().to_text()))
            .collect()
    }

    pub fn len(&self) -> usize { self.termlets.len() }
    pub fn is_empty(&self) -> bool { self.termlets.is_empty() }
}

impl Drop for TermletPool {
    fn drop(&mut self) { self.kill_all(); }
}
```

### 32.11 SnapshotDiff [v11 Refined]

Structured comparison between two snapshots with optional context.

```rust
// crates/mux-termlet/src/diff.rs

#[derive(Debug, Clone)]
pub struct SnapshotDiff {
    pub changed_lines: Vec<LineDiff>,
    pub is_identical: bool,
    pub before_size: (u16, u16),
    pub after_size: (u16, u16),
}

#[derive(Debug, Clone)]
pub struct LineDiff {
    pub row: usize,
    pub before: String,
    pub after: String,
}

impl SnapshotDiff {
    /// Compare two snapshots and produce a diff.
    /// [v11] context_lines ignored for now in struct, but intended for future display logic.
    pub fn compare(before: &TermletSnapshot, after: &TermletSnapshot, _context_lines: usize) -> Self {
        let before_text = before.to_text();
        let after_text = after.to_text();

        if before_text == after_text && before.size() == after.size() {
            return Self {
                changed_lines: Vec::new(),
                is_identical: true,
                before_size: before.size(),
                after_size: after.size(),
            };
        }

        let before_lines: Vec<&str> = before_text.lines().collect();
        let after_lines: Vec<&str> = after_text.lines().collect();
        let max_rows = before_lines.len().max(after_lines.len());

        let mut changed_lines = Vec::new();
        for row in 0..max_rows {
            let b = before_lines.get(row).copied().unwrap_or("");
            let a = after_lines.get(row).copied().unwrap_or("");
            if b != a {
                changed_lines.push(LineDiff {
                    row,
                    before: b.to_string(),
                    after: a.to_string(),
                });
            }
        }

        Self {
            is_identical: changed_lines.is_empty(),
            changed_lines,
            before_size: before.size(),
            after_size: after.size(),
        }
    }

    pub fn to_display(&self) -> String {
        if self.is_identical {
            return "Snapshots are identical.".to_string();
        }
        let mut out = format!(
            "Snapshot diff: {} changed line(s) (before: {}x{}, after: {}x{})\n",
            self.changed_lines.len(),
            self.before_size.0, self.before_size.1,
            self.after_size.0, self.after_size.1,
        );
        for diff in &self.changed_lines {
            out.push_str(&format!("  row {}: -{:?} +{:?}\n", diff.row, diff.before, diff.after));
        }
        out
    }
}
```

### 32.12 Async Support

Optional async variant for binding event loops.

```rust
// crates/mux-termlet/src/async_support.rs
#[cfg(feature = "async")]
pub async fn wait_for_async(
    termlet: &mut Termlet,
    pattern: &str,
    timeout: std::time::Duration,
) -> Result<WaitMatch, TermletError> {
    // ... same as v10 ...
    todo!("Implementation uses tokio::time::sleep")
}
```

### 32.13 Async TermletPool [v11 New]

Async extensions for TermletPool.

```rust
// crates/mux-termlet/src/pool.rs
impl TermletPool {
    /// Async version of spawn.
    #[cfg(feature = "async")]
    pub async fn spawn_async(
        &mut self,
        name: &str,
        command: &str,
        config: TermletConfig,
    ) -> Result<(), TermletError> {
        // Since spawn is primarily synchronous (process creation),
        // this is mostly a wrapper, but allows for future async setup.
        self.spawn(name, command, config)
    }

    /// Async version of get (mutable).
    /// Useful if we want to add async locks in the future.
    #[cfg(feature = "async")]
    pub async fn get_async(&mut self, name: &str) -> Result<&mut Termlet, TermletError> {
        self.get_mut(name)
    }
}
```

### 32.14 Implementation Hints for LLMs [v11 New]

Specific guidance for AI coding agents implementing Termlets.

1.  **Non-blocking IO:** When implementing `RealPtyBackend`, ensure the file descriptor is set to non-blocking mode (`O_NONBLOCK`). `read_events` should return immediately with whatever data is available, or an empty vector if none. Do NOT block waiting for data.
2.  **Drain Loop:** The `wait_for` loop relies on `drain_output` to fetch data. If `drain_output` blocks, the timeout logic will fail.
3.  **Pattern Compilation:** Use `regex::Regex::new` only inside `PatternMatcher::compile`. Do not re-compile inside the loop.
4.  **Byte Ranges:** `WaitMatch.byte_range` indexes into the **UTF-8 string** returned by `grid.to_text()`, not the raw grid cells.
5.  **Snapshots:** `grid.to_text()` must strip trailing whitespace from every line to ensure stable snapshots across resize operations (which might introduce padding spaces).

### AGENTS.md Rules

- `RULE-S32-01`: Termlet snapshot text must be trailing-whitespace-trimmed.
- `RULE-S32-02`: Termlet kill must be idempotent.
- `RULE-S32-03`: Termlet Drop must send SIGKILL to prevent PTY process leaks.
- `RULE-S32-04`: FakePtyBackend tests must not depend on timing.
- `RULE-S32-05`: Termlet bindings must implement context manager / cleanup pattern.
- `RULE-S32-06`: `mux-termlet` must not depend on `mux-server`, `mux-api`, or `mux-orm`.
- `RULE-S32-07`: Every Termlet API method tested in Rust + at least one binding.
- `RULE-S32-08`: `wait_for` must compile pattern once, not per poll iteration.
- `RULE-S32-09`: Termlet remains pane-backed; never invent parallel terminal core.
- `RULE-S32-10`: Cross-language snapshot consistency tests must exist for every release.
- `RULE-S32-11`: `TermletBuilder` and `TermletConfig` must produce identical Termlets.
- `RULE-S32-12`: `SnapshotDiff` must not allocate on identical snapshots.
- `RULE-S32-13`: `TermletPool` Drop must kill all managed Termlets.
- `RULE-S32-14`: `wait_for_async` is feature-gated under `async`.
- `RULE-S32-15`: **[v11]** `RealPtyBackend` must use non-blocking IO for event reading.

---

*End of TermForge v11 Refined Architecture Specification.*
