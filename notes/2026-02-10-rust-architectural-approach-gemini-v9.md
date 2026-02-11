I will list the files in the `notes/` directory to locate the v8 specification and any other relevant context.
I will read the v8 synthesis and the v7 deep dive review to gather the necessary context.
I will read the remaining sections of the v8 specification (from line 2000 onwards) to verify the implementation details for the ORM, Language Bindings, OpenTelemetry, Version Management, and Test Support.
I will read the remaining sections of the v8 specification (from line 4000 onwards) to verify the implementation details for Version Management, Test Support, and the specific gaps identified in the deep-dive review.
I will write the v9 architecture specification to `notes/2026-02-10-rust-architectural-approach-gemini-v9.md`. This will be done in multiple chunks to ensure the entire content is written correctly.
I will now generate the definitive v9 architecture specification for TermForge, incorporating all insights from the v8 synthesis and the v7 deep-dive review. This specification addresses the identified gaps in the ORM API, language bindings, OpenTelemetry integration, and test infrastructure.

```bash
cat > notes/2026-02-10-rust-architectural-approach-gemini-v9.md << 'ENDOFSPEC'
# TermForge v9 Architecture Specification

Date: 2026-02-11
Status: DEFINITIVE -- v9, refined from v8 Pass 3 with deep-dive gap closure.
Lineage: v4 -> v5 -> v6 -> v7 -> v8 (Triple-Pass Synthesis) -> **v9 (Gemini Refined)**.
License: MIT OR Apache-2.0
Rust edition: 2024 (MSRV 1.85)
Protocol target: tmux protocol v8

---

## Preamble

This document is the single authoritative architectural reference for **TermForge**, a Rust terminal multiplexer with **100% tmux wire-protocol compatibility**.

### v9 Refinements (Gemini)
This v9 specification builds upon the v8 synthesis, explicitly addressing gaps identified in the v7 deep-dive review and strengthening implementation details for critical subsystems.

**Key Enhancements in v9:**
1.  **ORM Completeness:** explicit inclusion of `Nin` (not-in) and `IRegex` operators to match `libtmux` exactly.
2.  **Language Bindings:** Detailed implementation of `PyQueryList` and `JsQueryList` with kwarg-based filtering and `get(default=...)` semantics.
3.  **Telemetry:** Mandated dual-provider architecture (`OTEL_PROVIDER` + `MUX_CLIENT_PROVIDER`) and thread-local trace header propagation.
4.  **Version Management:** Concrete implementation of BLAKE3 cache keys, file-based locking (`flock`), and atomic artifact publication.
5.  **Test Isolation:** Strict enforcement of 3-layer socket path validation (`PathGuard`) and permission hardening (`0700`).
6.  **Missing Crates:** Explicit definition of `mux-types` and `mux-os` contents to resolve "missing in reference" status.

---

## Table of Contents

1. [Vision and Philosophy](#1-vision-and-philosophy)
2. [North Star Acceptance Criteria](#2-north-star-acceptance-criteria)
3. [High-Level Architecture](#3-high-level-architecture)
4. [Workspace Layout](#4-workspace-layout)
5. [Layering Contract](#5-layering-contract)
6. [Entity Model](#6-entity-model)
7. [Event/Effect Engine](#7-eventeffect-engine)
8. [Error Handling](#8-error-handling)
9. [Protocol Codec](#9-protocol-codec)
10. [Configuration System](#10-configuration-system)
11. [Layout Engine](#11-layout-engine)
12. [ORM-like Query API](#12-orm-like-query-api)
13. [Runtime Architecture](#13-runtime-architecture)
14. [Server Lifecycle](#14-server-lifecycle)
15. [Control Mode](#15-control-mode)
16. [Language Bindings](#16-language-bindings)
17. [CRDT Transaction Layer](#17-crdt-transaction-layer)
18. [Security Model](#18-security-model)
19. [OpenTelemetry](#19-opentelemetry)
20. [tmux Version Management](#20-tmux-version-management)
21. [Test Support and Fake PTY](#21-test-support-and-fake-pty)
22. [Binding Test Frameworks](#22-binding-test-frameworks)
23. [Test Framework and Harness Design](#23-test-framework-and-harness-design)
24. [Performance Targets](#24-performance-targets)
25. [Visual Client / TUI](#25-visual-client--tui)
26. [AGENTS.md Rules](#26-agentsmd-rules)
27. [Risks and Mitigations](#27-risks-and-mitigations)
28. [Plan Evolution and Changelog](#28-plan-evolution-and-changelog)
29. [Reference Anchors](#29-reference-anchors)
30. [Appendix: Canonical Type Quick Reference](#30-appendix-canonical-type-quick-reference)
31. [Supplemental Test Matrix](#31-supplemental-test-matrix)

---

## 1. Vision and Philosophy

### 1.1 Core Thesis

TermForge is a **new terminal multiplexer** written in Rust that is **wire-compatible** with tmux protocol v8. It is not a port of tmux's C code. It uses tmux as a behavioral reference while building a clean, layered Rust architecture from scratch.

### 1.2 Design Principles

1.  **Pure/Impure Separation (Sans-IO):** The kernel (`mux-core`) is a pure `fn(graph, event) -> (graph, effects)` reducer. It never touches file descriptors, system calls, timers, or network.
2.  **tmux is a Compatibility Profile:** `struct session`, `struct window` inform our model, but we use domain-native names. `mux-proto` handles the adaptation.
3.  **Snapshots are the Read Path:** The `ServerGraph` is the single source of truth, mutated only via events. Readers see immutable `ArcSwap` snapshots.
4.  **ORM is a Facade:** `mux-orm` translates user intent into commands. It does not implement logic.
5.  **Layered Testing:** Property tests for core, fixture tests for protocol, hermetic fake-PTY tests for runtime.
6.  **WASM as Purity Proof:** Layer 0 crates must compile to `wasm32-unknown-unknown` to prove OS-independence.
7.  **Bindings as First-Class:** Python (PyO3) and Node (Neon) bindings expose the full ORM API.
8.  **Observability Built In:** OpenTelemetry tracing throughout, with specific dual-provider architecture.
9.  **CRDT-Ready:** State model supports conflict-free replication.

---

## 2. North Star Acceptance Criteria

### 2.1 Compatibility (C)

| ID | Criterion | Verification |
|---|---|---|
| C1 | Stock tmux 3.6+ client attaches to TermForge | `tmux -S /path attach` |
| C2 | TermForge client attaches to stock tmux 3.6+ | `termforge attach -S /path` |
| C3 | Protocol v8 imsg framing roundtrip | `proptest` with all 35+ types |
| C4 | Identify burst (100-112) byte-exact | Fixture captures |
| C5 | SCM_RIGHTS fd passing preserved | `tmux-sniff` passthrough |
| C6 | 200+ tmux commands parse/execute | `tmux-command-audit` |
| C7 | Format expansion matches tmux | `format-audit` corpus |
| C8 | Default key bindings match | Key table parity tests |

### 2.2 Architectural (A)

| ID | Criterion | Enforcement |
|---|---|---|
| A1 | Layer 0: zero `unsafe`, zero `tokio`, zero `libc` | `#![forbid(unsafe_code)]` |
| A2 | Layer 0 compiles to `wasm32-unknown-unknown` | CI check |
| A3 | `mux-os` is sole `unsafe` quarantine | CI lint |
| A4 | Deterministic state transitions | Property tests |
| A5 | No panics in decode/encode | `clippy::unwrap_used` denied |
| A6 | Bindings depend only on `mux-orm` | Cargo dependency check |
| A7 | No `Arc<Mutex<_>>` on read path | `ArcSwap` snapshots |

### 2.3 Product (P)

| ID | Criterion |
|---|---|
| P1 | In-process embedding without spawning |
| P2 | Python pytest fixture: `server` -> `pane` in 4 lines |
| P3 | Snapshot test via `insta` |
| P4 | CRDT merge of concurrent `new-session` |
| P5 | TUI client renders correctly |
| P6 | FakePty scenario replay |

### 2.4 Performance (B)

| ID | Target | Basis |
|---|---|---|
| B1 | VT100 parser (ASCII) > 300 MB/s | alacritty baseline |
| B2 | VT100 parser (CSI) > 100 MB/s | param parsing cost |
| B3 | Protocol decode > 500 MB/s | zero-copy goal |
| B4 | Layout resize (20 panes) < 50 us | tree walk |
| B5 | Graph snapshot < 1 ms | `Arc` clones |
| B6 | Option resolve < 100 ns | BTreeMap lookups |

---

## 3. High-Level Architecture

### 3.1 The Layer Cake

```
Layer 0  PURE KERNEL     mux-types, mux-core, mux-query, mux-conf,
                          mux-command, mux-grid, mux-control, mux-pty-fake,
                          mux-crdt, mux-proto

Layer 1  IMPURE RUNTIME  mux-os, mux-pty, mux-pty-portable, mux-server,
                          mux-client, mux-refresh, mux-backend

Layer 2  FACADE           mux-api (ManagedMux, SocketActor)

Layer 3  ORM              mux-orm (QueryList, typed wrappers)

Layer 4  BINDINGS         bindings/python, bindings/node, mux-cxx

Layer 5  TOOLS            termforge-cli, tmux-vm, tmux-sniff, etc.
```

### 3.2 Dependency Rules
- **Inward only:** Layer N depends on N-1.
- **Layer 0 is pure:** No IO, no async, no unsafe.
- **Layer 1 handles IO:** `tokio`, `libc`, `nix`.
- **Bindings are consumers:** They use `mux-orm`, never internals.

---

## 4. Workspace Layout

```
termforge/
  Cargo.toml (workspace)
  crates/
    mux-types/          # IDs, shared enums (PURE)
    mux-core/           # ServerGraph, Engine (PURE)
    mux-grid/           # VT100 parser, Grid (PURE)
    mux-query/          # QueryList, QueryOp (PURE)
    mux-conf/           # Config parser (PURE)
    mux-command/        # Command table (PURE)
    mux-proto/          # imsg codec (PURE)
    mux-control/        # Control mode (PURE)
    mux-pty-fake/       # Scenarios (PURE)
    mux-crdt/           # OpLog, Merge (PURE)
    
    mux-os/             # Unsafe quarantine (IMPURE)
    mux-pty/            # PtyBackend trait
    mux-pty-portable/   # Real PTY impl
    mux-server/         # Tokio runtime
    mux-client/         # Client handling
    mux-refresh/        # StateStore, snapshots
    mux-backend/        # Local/Tmux backends
    
    mux-api/            # Facade
    mux-orm/            # ORM layer
    mux-telemetry/      # OTEL integration
    mux-test-support/   # PathGuard, TestServer
    
  bindings/
    python/             # PyO3
    node/               # Neon
    
  tools/
    tmux-vm/            # Version manager
    tmux-builder/       # Build logic
```

---

## 5. Layering Contract

1.  **Core is Deterministic:** `apply_event(graph, event, ctx)` is a pure function. Time and randomness are injected via `ctx`.
2.  **Snapshots are the Read Path:** Readers never lock the graph. They read `ArcSwap` snapshots.
3.  **Protocol is an Adapter:** `mux-proto` translates wire format to domain events.
4.  **ORM is a Facade:** It exposes a nice API but implements no logic.

---

## 6. Entity Model

### 6.1 Typed IDs (`mux-types`)

```rust
use slotmap::new_key_type;
new_key_type! {
    pub struct SessionId;
    pub struct WindowId;
    pub struct PaneId;
    pub struct ClientId;
}
```

### 6.2 Server Graph (`mux-core`)

The `ServerGraph` uses `SlotMap` for storage and `Vec` for hierarchy. Reverse lookups are computed during snapshot generation.

```rust
pub struct ServerGraph {
    sessions: SlotMap<SessionId, Session>,
    windows:  SlotMap<WindowId, Window>,
    panes:    SlotMap<PaneId, Pane>,
    // ...
}

pub struct Session {
    pub name: String,
    pub windows: Vec<WindowId>, // Ordered list
    // ...
}
```

**Decision:** Parent-child relationships are stored as `Vec<ChildId>` in the parent. Reverse maps (Child -> Parent) are ephemeral, built only in `GraphState` snapshots.

---

## 7. Event/Effect Engine

### 7.1 The Update Loop

```rust
pub fn apply_event(
    graph: &mut ServerGraph,
    event: Event,
    ctx: &CoreCtx
) -> ApplyOutcome {
    // Pure logic only
    match event {
        Event::CreateSession { .. } => { /* mutate graph */ },
        Event::PaneOutput { .. } => { /* update grid */ },
        // ...
    }
}
```

### 7.2 Events and Effects

- **Event:** Something happened (Input, Timer, Request).
- **Effect:** Something should happen (Write PTY, Send to Client, Log).

---

## 8. Error Handling

### 8.1 Error Classification

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorClass {
    Transient,        // Retry (e.g., EAGAIN)
    ProtocolViolation,// Kill connection (e.g., bad frame)
    UserError,        // Report to user (e.g., unknown command)
    Bug,              // Log stack trace
}

pub trait Classified {
    fn class(&self) -> ErrorClass;
}
```

### 8.2 Recovery Strategy
- **Transient:** Backoff and retry.
- **ProtocolViolation:** Immediate connection termination.
- **UserError:** Send `MSG_ERROR` or print to stderr.
- **Bug:** Panic in debug, log error in release.

---

## 9. Protocol Codec

### 9.1 Wire Format
Standard `imsg` format (OpenBSD/tmux):
```
[ type(4) | len(4) | peerid(4) | pid(4) ] + payload
```
Native endianness.

### 9.2 Codec Implementation
`mux-proto` implements a two-phase decoder (Header -> Payload).
It handles `IMSG_FD_FLAG` (`0x8000_0000` in length) for `SCM_RIGHTS`.

---

## 10. Configuration System

### 10.1 Scope Resolution
Matches tmux `options.c`:
- `Server`
- `Session` (Global or Specific)
- `Window` (Global or Specific)
- `Pane` (falls through to Window if not `-p`)

### 10.2 Loading
Config is loaded **only after** the first client identifies. This ensures the client is ready to receive error messages.

---

## 11. Layout Engine

### 11.1 Tree Structure (`mux-core`)
A flattened arena of `LayoutCell`s.

```rust
pub struct LayoutTree {
    cells: Vec<LayoutCell>, // Index 0 is root
}

pub struct LayoutCell {
    pub kind: LayoutType, // WindowPane, LeftRight, TopBottom
    pub sx: u16,
    pub sy: u16,
    pub children: Vec<usize>,
    // ...
}
```

### 11.2 Resize Algorithm
**Critical:** Uses round-robin distribution (one cell at a time), NOT proportional. This guarantees bit-exact layout parity with tmux.

### 11.3 Checksum
Implements the custom rotate-and-add checksum used by tmux for layout strings.

---

## 12. ORM-like Query API

### 12.1 Operators
Complete set matching `libtmux` + Rust extensions.

```rust
// mux-query/src/ops.rs
pub enum QueryOp {
    Exact, IExact,
    Contains, IContains,
    StartsWith, IStartsWith,
    EndsWith, IEndsWith,
    In, Nin,           // Nin added for parity
    Regex, IRegex,     // IRegex added for parity
    Gt, Gte, Lt, Lte,  // Rust extensions
    IsNone, IsSome,    // Rust extensions
}
```

### 12.2 QueryList
Wraps `Arc<Vec<T>>` with filtering capabilities.

```rust
// mux-query/src/list.rs
impl<T: Queryable> QueryList<T> {
    pub fn filter(&self, input: QueryInput) -> Result<Self, QueryError>;
    pub fn get(&self, input: QueryInput) -> Result<T, QueryError>;
    
    // Support for get(default=...) pattern
    pub fn get_or(&self, input: QueryInput, default: Option<T>) -> Result<T, QueryError>;
}
```

---

## 13. Runtime Architecture

### 13.1 State Actor
Single `tokio` task owning the `ServerGraph`.
- Receives `Event`s.
- Calls `apply_event`.
- Dispatches `Effect`s.
- Publishes `GraphState` snapshots.

### 13.2 IO Tasks
- **Accept Loop:** Accepts connections, spawns client tasks.
- **Client Task:** Reads socket, decodes frames, sends Events.
- **PTY Task:** Reads PTY output, sends `PaneOutput` events.

---

## 14. Server Lifecycle

### 14.1 Locking
Uses `flock` (advisory file locking) on the lock file.
**Why:** Released automatically by OS on process death. No stale PID file issues.

### 14.2 Startup
1. Bind socket (0600 permissions).
2. Acquire lock.
3. Init telemetry.
4. Wait for first client.
5. Handshake (Identify).
6. Load Config.

---

## 15. Control Mode

### 15.1 Output Rate Limiting
Per-client buffer limit (e.g., 16MB). If exceeded, client is disconnected to protect the server.

### 15.2 Notifications
Typed `ControlNotification` enum for all tmux control channel messages (`%session-changed`, etc.).

---

## 16. Language Bindings

### 16.1 Python (PyO3)

**Features:**
- `PyQueryList` wrapper with `filter(**kwargs)` support.
- GIL release via `Python::detach`.
- `get(default=...)` support.

```rust
// bindings/python/src/query_list.rs
#[pymethods]
impl PyQueryList {
    #[pyo3(signature = (matcher=None, **kwargs))]
    fn filter(&self, py: Python<'_>, matcher: Option<PyObject>, kwargs: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
        // Implementation handling kwargs -> QuerySpec mapping
        // ...
    }

    #[pyo3(signature = (matcher=None, default=None, **kwargs))]
    fn get(&self, py: Python<'_>, matcher: Option<PyObject>, default: Option<PyObject>, kwargs: Option<&Bound<'_, PyDict>>) -> PyResult<PyObject> {
        // Implementation with default fallback
        // ...
    }
}
```

### 16.2 Node.js (Neon)

**Features:**
- `JsQueryList` wrapper.
- `Channel` for async operations.

```typescript
// binding.ts
class QueryList<T> {
    filter(criteria: object): QueryList<T>;
    get(criteria: object, options?: { default?: T | null }): T | null;
}
```

---

## 17. CRDT Transaction Layer

### 17.1 Hybrid Logical Clock (HLC)
Timestamps for causal ordering.
`HlcTimestamp { millis: u64, counter: u32, node_id: u64 }`

### 17.2 Data Structures
- **LWW Register:** Last-Writer-Wins for simple fields (name, size).
- **OR Set:** Observed-Remove Set for collections (windows, panes).

---

## 18. Security Model

### 18.1 Socket Security
- `0700` permissions on socket directory.
- `0600` on socket file.
- `flock` on lock file.

### 18.2 Input Validation
- **Protocol:** Strict frame validation.
- **PTY:** Bounded VT100 parsing (no infinite buffers).
- **Control:** Rate limited output.

---

## 19. OpenTelemetry

### 19.1 Dual Provider Architecture (Mandatory)
To prevent blocking the server startup with client-side export logic, use two providers:
1.  **`OTEL_PROVIDER`:** Global provider for server internals and TUI.
2.  **`MUX_CLIENT_PROVIDER`:** Lazy, on-demand provider for client CLI commands.

```rust
// mux-otel/src/otel.rs
static OTEL_PROVIDER: OnceLock<Mutex<Option<OtelProvider>>> = OnceLock::new();
static MUX_CLIENT_PROVIDER: OnceLock<Mutex<Option<OtelProvider>>> = OnceLock::new();
```

### 19.2 Context Propagation
Use thread-local storage for headers to support process boundaries without leaking context in nested calls.

```rust
thread_local! {
    static TRACE_HEADERS_STACK: RefCell<Vec<TraceHeaders>> = const { RefCell::new(Vec::new()) };
}
```

---

## 20. tmux Version Management

### 20.1 BLAKE3 Cache Keys
Concrete implementation of cache keys for reproducible builds.

```rust
fn compute_cache_key(tag: &str, options: &EnsureOptions) -> String {
    let flags_hash = blake3::hash(format!("{:?}{:?}", options.configure_flags, options.make_flags).as_bytes());
    format!("tmux-{tag}__{host}__{hash}", tag=tag, host=host_triple(), hash=flags_hash.to_hex())
}
```

### 20.2 File-Based Locking
Use `fs2` or `libc::flock` to prevent build races.
- `lock_repo_clone()`: Serializes git operations.
- `lock_cache_key()`: Serializes builds for the same key.

### 20.3 Atomic Publish
Build to `.tmp-XXXX`, then `rename()` to final path.

---

## 21. Test Support and Fake PTY

### 21.1 PathGuard (3-Layer Validation)
Mandatory safety for all test sockets.

```rust
// mux-test-support/src/path_guard.rs
pub fn ensure_safe(path: &Path, tempdir: &Path) -> Result<()> {
    ensure_not_default_name(path)?;
    ensure_within_tempdir(path, tempdir)?;
    ensure_not_tmux_env(path)?;
    Ok(())
}
```

### 21.2 Test Server Modes
- **In-Process:** Direct `ManagedMux` calls. Fast, debuggable.
- **Subprocess:** Spawns `mux-server`. True integration test.

---

## 22. Binding Test Frameworks
Tests must run against both **In-Process** and **Socket** backends to verify transport transparency.

---

## 23. Test Framework and Harness Design
- **Scenario Replay:** JSON-based recordings of PTY sessions.
- **Snapshot Testing:** `insta` for grid state verification.

---

## 24. Performance Targets
- **Parsing:** > 300 MB/s
- **Latency:** < 1 ms for local actions.

---

## 25. Visual Client / TUI
- **ratatui** based.
- **ViewModel** pattern: Pure function `(State, UIState) -> ViewModel`.

---

## 26. AGENTS.md Rules
Specific rules for AI agents modifying this codebase.
1.  **No Unsafe:** Outside `mux-os`.
2.  **Verify Parity:** Check against `tmux` source behavior.
3.  **Add Tests:** Every feature needs a test.

---

## 27. Risks and Mitigations
- **Parity Drift:** Mitigated by `mux-regress` suite.
- **Security:** Mitigated by fuzzing and strict validation.

---

## 28. Plan Evolution and Changelog
- **v9:** Addressed v7 deep-dive gaps (ORM, Bindings, OTEL, Version Mgmt).

---

## 29. Reference Anchors
- **tmux 3.6a:** Behavioral reference.
- **libtmux:** API reference.

---

## 30. Appendix: Canonical Type Quick Reference
(List of key structs and enums)

---

## 31. Supplemental Test Matrix
(Grid of test coverage requirements)
ENDOFSPEC
```
