# TermForge: Rust-First Terminal Multiplexer Architecture Specification

**Version:** 8.0
**Date:** 2026-02-11
**Status:** Complete standalone spec (supersedes v7)
**Delta from v7:** Fixes all 20 gaps identified in deep-dive review (4 critical, 11 high, 5 medium)

---

## Preamble

This document is the **authoritative architecture specification** for TermForge, a Rust-first terminal multiplexer with first-class Python and Node.js bindings. It is a complete, standalone document -- not a diff or patch against any prior version.

### v8 Change Summary (from v7)

| # | Section | Fix | Priority |
|---|---------|-----|----------|
| 1 | 16 | Add `PyQueryList` class with kwargs filter API | Critical |
| 2 | 16 | Add Node `QueryList` equivalent | Critical |
| 3 | 20 | Add file-based locking for parallel build safety | Critical |
| 4 | 21 | Add three-layer socket validation (not-default, within-tempdir, not-tmux-env) | Critical |
| 5 | 12 | Add `Nin`, `IRegex` operators | High |
| 6 | 19 | Dual-provider architecture (OTEL_PROVIDER + CLIENT_PROVIDER) | High |
| 7 | 19 | Thread-local TRACE_HEADERS_STACK with TraceHeadersGuard | High |
| 8 | 20 | BLAKE3 cache key computation | High |
| 9 | 20 | Atomic build publication | High |
| 10 | 20 | Comprehensive env var interface | High |
| 11 | 21 | Permission hardening (0700) | High |
| 12 | 21 | `-f /dev/null` config isolation | High |
| 13 | 21 | Socket readiness check | High |
| 14 | 21 | Two MuxServerTestServer modes (in-process + subprocess) | High |
| 15 | 12 | `get(default=...)` parameter | Medium |
| 16 | 19 | TOML config priority chain | Medium |
| 17 | 19 | Composite propagator | Medium |
| 18 | 19 | Env var enable toggle | Medium |
| 19 | 21 | Clipboard isolation | Medium |
| 20 | 12 | Callable matcher support in `filter()` | Low |

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

### 1.1 Mission Statement

TermForge is a Rust-first terminal multiplexer that replaces tmux while providing:
- **Protocol compatibility** with tmux's control mode
- **First-class Python and Node.js bindings** via PyO3 and Neon
- **ORM-like query API** mirroring libtmux's `QueryList` with kwargs-style filters
- **CRDT-based state** for eventual multi-server replication
- **OpenTelemetry observability** from day one

### 1.2 Guiding Principles

1. **Rust core, polyglot surface** -- All business logic in Rust; bindings are thin FFI wrappers.
2. **libtmux API fidelity** -- Python users should feel at home: `server.sessions.filter(name="work")` works.
3. **No unsafe in application code** -- `#![forbid(unsafe_code)]` in all crates except `tf-pty`.
4. **Observable by default** -- Every public function entry/exit is traceable via OpenTelemetry.
5. **Test isolation is non-negotiable** -- Tests never touch the user's real tmux session.
6. **Build safety** -- Parallel test runs cannot corrupt shared caches; file locks + atomic publish.
7. **Zero-copy where possible** -- `bytes::Bytes` for protocol buffers, zero-copy parsing.

### 1.3 Prior Art

| Project | What We Take | What We Improve |
|---------|-------------|-----------------|
| tmux | Protocol, keybindings, layout algorithm | Type safety, observability, bindings |
| libtmux | QueryList, ORM traversal, kwargs filter | Native Rust speed, compile-time checks |
| vibe-tmux | Socket safety, BLAKE3 cache, dual OTEL provider | Unified architecture instead of separate tools |
| Zellij | Plugin model, WASM | Tighter control-mode compat |

### AGENTS.md Rules (Section 1)

```
RULE S01-VISION: Every PR description must reference which guiding principle it advances.
RULE S01-UNSAFE: #![forbid(unsafe_code)] in all crates except tf-pty. CI enforces.
RULE S01-COMPAT: tmux control-mode protocol changes require RFC with migration plan.
```

### Test Strategy (Section 1)

- Compile-time: `#![forbid(unsafe_code)]` enforced by CI lint job
- Unit: Philosophy principles validated by architectural decision records (ADRs)
- Integration: tmux control-mode compatibility suite (Section 22)

---

## 2. North Star Acceptance Criteria

### 2.1 Functional Criteria

| ID | Criterion | Validation |
|----|-----------|------------|
| NS-01 | `tmux -CC` sessions work identically against TermForge server | Protocol fuzz test suite |
| NS-02 | `server.sessions.filter(name="work")` works in Python | PyQueryList integration test |
| NS-03 | `server.sessions({ name: "work" })` works in Node.js | JsQueryList integration test |
| NS-04 | Layout algorithm matches tmux pixel-for-pixel | Layout snapshot tests |
| NS-05 | Hot config reload without session loss | Config reload integration test |
| NS-06 | OTEL traces flow from Python -> Rust client -> Rust server | End-to-end trace validation |
| NS-07 | 10,000 panes with <16ms frame time | Benchmark suite |
| NS-08 | Parallel `cargo test` never corrupts shared state | File-lock stress test |

### 2.2 Non-Functional Criteria

| ID | Criterion | Target |
|----|-----------|--------|
| NF-01 | Server startup latency | <50ms |
| NF-02 | Control mode round-trip | <5ms |
| NF-03 | Memory per idle pane | <8KB |
| NF-04 | Binary size (release, stripped) | <10MB |
| NF-05 | Test suite wall-clock time | <120s on 4-core |

### AGENTS.md Rules (Section 2)

```
RULE S02-CRITERIA: Every milestone must demonstrate progress toward at least 2 NS-* criteria.
RULE S02-PERF: Performance regressions beyond 10% on any NF-* metric block merge.
```

### Test Strategy (Section 2)

- CI benchmark job runs on every PR, compares against baseline
- NS-* criteria have dedicated integration test tags: `#[cfg(test_ns01)]` etc.

---

## 3. High-Level Architecture

### 3.1 System Diagram

```
+-----------------------------------------------------------+
|                    Client Tier                              |
|  +----------+  +----------+  +----------+                  |
|  |  TUI     |  |  Python  |  |  Node.js |                  |
|  |(ratatui) |  | (PyO3)   |  | (Neon)   |                  |
|  |          |  |PyQueryList|  |JsQueryList|                 |
|  +----+-----+  +----+-----+  +----+-----+                  |
|       |              |              |                        |
|       +--------------+--------------+                        |
|                      | Control Protocol                      |
+----------------------+--------------------------------------+
|                 Server Tier                                  |
|  +-------------------------------------------------------+  |
|  |            Protocol Codec (tf-proto)                   |  |
|  +-------------------------------------------------------+  |
|  |         Event/Effect Engine (tf-event)                 |  |
|  +-------------------------------------------------------+  |
|  |      Entity Model + CRDT State (tf-model)              |  |
|  |   ServerGraph -> Session -> Window -> Pane             |  |
|  +-------------------------------------------------------+  |
|  |         ORM Query Layer (tf-query)                     |  |
|  |    QueryList<T> with 17 operators + callable           |  |
|  +-------------------------------------------------------+  |
|  |    Layout Engine (tf-layout) | Config (tf-config)      |  |
|  +-------------------------------------------------------+  |
|  |              PTY Backend (tf-pty)                       |  |
|  +-------------------------------------------------------+  |
+-----------------------------------------------------------+
|                 Cross-Cutting                               |
|  +-------------+  +--------------+  +---------------+      |
|  |  tf-otel    |  | tf-version   |  | tf-test       |      |
|  | Dual OTEL   |  | BLAKE3 cache |  | 3-layer guard |      |
|  | provider    |  | file locks   |  | fake PTY      |      |
|  +-------------+  +--------------+  +---------------+      |
+-----------------------------------------------------------+
```

### 3.2 Data Flow

1. Client sends command via control protocol (Unix socket or TCP)
2. `tf-proto` decodes into typed `Command` enum
3. `tf-event` processes command, produces `Effect` list
4. Effects mutate `ServerGraph` (CRDT-backed entity model)
5. Changed entities trigger notifications back through protocol
6. Query layer provides `QueryList<T>` views over entity collections

### 3.3 Crate Dependency DAG

```
tf-pty (unsafe allowed)
  ^
tf-model (entity model, CRDT)
  ^
tf-query (ORM query layer)
  ^
tf-event (event/effect engine)
  ^
tf-proto (protocol codec)
  ^
tf-config (configuration)
  ^
tf-layout (layout engine)
  ^
tf-server (server binary)
  ^
tf-client (client library)
  ^
+-- tf-tui (TUI client)
+-- tf-py (Python bindings)
+-- tf-node (Node bindings)

Cross-cutting:
tf-otel (telemetry) -- used by tf-server, tf-client, tf-tui
tf-version (tmux version mgmt) -- used by tf-test
tf-test (test support) -- used by all crate tests
```

### AGENTS.md Rules (Section 3)

```
RULE S03-DAG: No circular dependencies between crates. CI runs cargo-deny to enforce.
RULE S03-CROSS: Cross-cutting crates (tf-otel, tf-test) may be used by any crate
    but must not depend on application crates.
RULE S03-LAYER: Each layer only depends on layers below it. tf-proto never imports tf-event.
```

### Test Strategy (Section 3)

- `cargo deny check` verifies acyclic dependency graph
- Architecture tests use `cargo-modules` to verify layer boundaries

---

## 4. Workspace Layout

### 4.1 Directory Structure

```
termforge/
+-- Cargo.toml                    # workspace root
+-- AGENTS.md                     # AI agent rules
+-- deny.toml                     # cargo-deny config
+-- .termforge/
|   +-- otel.toml                 # project OTEL config
|   +-- otel.local.toml           # local override (gitignored)
+-- crates/
|   +-- tf-model/                 # Entity model + CRDT
|   |   +-- src/
|   |   |   +-- lib.rs
|   |   |   +-- server_graph.rs
|   |   |   +-- session.rs
|   |   |   +-- window.rs
|   |   |   +-- pane.rs
|   |   |   +-- ids.rs
|   |   +-- Cargo.toml
|   +-- tf-query/                 # ORM query layer
|   |   +-- src/
|   |   |   +-- lib.rs
|   |   |   +-- query_list.rs     # QueryList<T> with 17 operators
|   |   |   +-- ops.rs            # QueryOp enum
|   |   |   +-- spec.rs           # QuerySpec builder
|   |   +-- Cargo.toml
|   +-- tf-event/                 # Event/effect engine
|   +-- tf-proto/                 # Protocol codec
|   +-- tf-config/                # Configuration
|   +-- tf-layout/                # Layout engine
|   +-- tf-pty/                   # PTY backend (unsafe)
|   +-- tf-otel/                  # OpenTelemetry (dual provider)
|   |   +-- src/
|   |   |   +-- lib.rs            # TraceHeaders, TraceHeadersGuard
|   |   |   +-- otel.rs           # OTEL_PROVIDER + CLIENT_PROVIDER
|   |   |   +-- config.rs         # TOML config with priority chain
|   |   |   +-- stub.rs           # No-op when otel feature disabled
|   |   +-- Cargo.toml
|   +-- tf-version/               # tmux version management
|   |   +-- src/
|   |   |   +-- lib.rs            # ensure_tmux_binary, BLAKE3 cache
|   |   |   +-- cache.rs          # Cache key computation
|   |   |   +-- lock.rs           # File-based locking
|   |   |   +-- build.rs          # Atomic build publication
|   |   +-- Cargo.toml
|   +-- tf-test/                  # Test support
|   |   +-- src/
|   |   |   +-- lib.rs
|   |   |   +-- path_guard.rs     # Three-layer socket validation
|   |   |   +-- tmux_server.rs    # TmuxTestServer (subprocess)
|   |   |   +-- mux_server.rs     # MuxServerTestServer (dual mode)
|   |   |   +-- fake_pty.rs       # FakePtyBackend
|   |   +-- Cargo.toml
|   +-- tf-server/                # Server binary
|   +-- tf-client/                # Client library
+-- bindings/
|   +-- python/
|   |   +-- src/
|   |   |   +-- lib.rs
|   |   |   +-- py_server.rs
|   |   |   +-- py_session.rs
|   |   |   +-- py_window.rs
|   |   |   +-- py_pane.rs
|   |   |   +-- py_query_list.rs  # PyQueryList with kwargs filter
|   |   +-- pyproject.toml
|   |   +-- Cargo.toml
|   +-- node/
|       +-- src/
|       |   +-- lib.rs
|       |   +-- js_server.rs
|       |   +-- js_query_list.rs  # JsQueryList with object filter
|       +-- package.json
|       +-- Cargo.toml
+-- tools/
|   +-- tf-builder/               # tmux-builder equivalent
|       +-- src/
|           +-- lib.rs            # BLAKE3 cache, file locks, atomic publish
+-- tests/
    +-- integration/
    +-- protocol/
    +-- e2e/
```

### 4.2 Feature Flags

```toml
# Workspace Cargo.toml
[workspace]
resolver = "2"
members = ["crates/*", "bindings/*", "tools/*"]

[workspace.dependencies]
blake3 = "1"
fs2 = "0.4"
bytes = "1"
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
opentelemetry = "0.28"
opentelemetry-otlp = { version = "0.28", features = ["tonic", "http-proto"] }
opentelemetry-sdk = { version = "0.28", features = ["rt-tokio"] }
tracing-opentelemetry = "0.28"
toml = "0.8"
serde = { version = "1", features = ["derive"] }
tempfile = "3"
nix = { version = "0.29", features = ["user", "fs"] }
pyo3 = "0.23"
neon = "1"
regex = "1"
```

### AGENTS.md Rules (Section 4)

```
RULE S04-LAYOUT: New crates must go in crates/, bindings/, or tools/. Never at workspace root.
RULE S04-FEATURES: Feature flags must be documented in the crate's README with use-case.
RULE S04-DEPS: New dependencies require justification comment in PR description.
```

### Test Strategy (Section 4)

- CI validates workspace builds with `--all-features` and `--no-default-features`
- `cargo deny check bans` prevents duplicate transitive deps

---

## 5. Layering Contract

### 5.1 Layer Definitions

| Layer | Crate | Allowed Dependencies |
|-------|-------|---------------------|
| L0: Primitives | tf-model | std, serde, bytes |
| L1: Query | tf-query | L0 |
| L2: Effects | tf-event | L0, L1 |
| L3: Protocol | tf-proto | L0, L2 |
| L4: Config | tf-config | L0, toml, serde |
| L5: Layout | tf-layout | L0 |
| L6: PTY | tf-pty | L0, nix, libc |
| L7: Server | tf-server | L0-L6 |
| L8: Client | tf-client | L0, L1, L3 |
| L9: Bindings | tf-py, tf-node | L0, L1, L8 |
| X0: Telemetry | tf-otel | tracing, opentelemetry |
| X1: Version | tf-version | blake3, fs2 |
| X2: Test | tf-test | tempfile, nix, L0 |

### 5.2 Forbidden Dependencies

```rust
// tf-proto must NEVER import:
// - tf-event (protocol codec is pure parsing)
// - tf-server (no circular deps)
// - tf-py, tf-node (bindings are leaf nodes)

// tf-model must NEVER import:
// - tf-query (model is queried, not the reverse)
// - tf-event (model is mutated by effects, not the reverse)
// - Any async runtime (model is sync)
```

### 5.3 Cross-Cutting Crate Rules

Cross-cutting crates (tf-otel, tf-test, tf-version) may be used by any application crate but:
- Must not depend on any application crate (L0-L9)
- Must be feature-gated when adding weight (e.g., `tf-otel` behind `otel` feature)
- Must compile with `--no-default-features` for minimal builds

### AGENTS.md Rules (Section 5)

```
RULE S05-LAYER: PRs that add a dependency from a lower layer to a higher layer are auto-rejected.
RULE S05-CROSS: Cross-cutting crates must compile independently: cargo check -p <crate>.
RULE S05-SYNC: tf-model must remain Send + Sync with no async runtime dependency.
```

### Test Strategy (Section 5)

```rust
#[test]
fn layering_contract_enforced() {
    // Parse Cargo.toml dependency graph and assert no upward deps
    let metadata = cargo_metadata::MetadataCommand::new().exec().unwrap();
    for pkg in &metadata.packages {
        if pkg.name == "tf-proto" {
            for dep in &pkg.dependencies {
                assert_ne!(dep.name, "tf-event",
                    "tf-proto must not depend on tf-event");
            }
        }
    }
}
```

---

## 6. Entity Model

### 6.1 Identity Types

```rust
// crates/tf-model/src/ids.rs

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

/// Monotonically increasing server-scoped ID.
macro_rules! define_id {
    ($name:ident, $prefix:literal) => {
        #[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(u64);

        impl $name {
            pub fn new(raw: u64) -> Self { Self(raw) }
            pub fn raw(self) -> u64 { self.0 }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}{}", $prefix, self.0)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({})", stringify!($name), self.0)
            }
        }
    };
}

define_id!(SessionId, "$");
define_id!(WindowId, "@");
define_id!(PaneId, "%");
define_id!(ClientId, "/dev/");

/// Thread-safe ID allocator.
pub struct IdAllocator {
    next: AtomicU64,
}

impl IdAllocator {
    pub const fn new(start: u64) -> Self {
        Self { next: AtomicU64::new(start) }
    }

    pub fn next_session(&self) -> SessionId {
        SessionId(self.next.fetch_add(1, Ordering::Relaxed))
    }

    pub fn next_window(&self) -> WindowId {
        WindowId(self.next.fetch_add(1, Ordering::Relaxed))
    }

    pub fn next_pane(&self) -> PaneId {
        PaneId(self.next.fetch_add(1, Ordering::Relaxed))
    }
}
```

### 6.2 Entity Structs

```rust
// crates/tf-model/src/session.rs

use crate::ids::{SessionId, WindowId, PaneId};
use std::collections::BTreeMap;
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct Session {
    pub id: SessionId,
    pub name: String,
    pub created: SystemTime,
    pub attached: bool,
    pub windows: BTreeMap<WindowId, Window>,
    pub active_window: Option<WindowId>,
    pub last_activity: SystemTime,
    /// CRDT version vector
    pub version: u64,
}

#[derive(Debug, Clone)]
pub struct Window {
    pub id: WindowId,
    pub name: String,
    pub index: u32,
    pub panes: BTreeMap<PaneId, Pane>,
    pub active_pane: Option<PaneId>,
    pub layout: LayoutTree,
    pub created: SystemTime,
}

#[derive(Debug, Clone)]
pub struct Pane {
    pub id: PaneId,
    pub title: String,
    pub width: u16,
    pub height: u16,
    pub pid: Option<u32>,
    pub command: String,
    pub active: bool,
    pub dead: bool,
    pub created: SystemTime,
    pub mode: PaneMode,
}

#[derive(Debug, Clone, Default)]
pub enum PaneMode {
    #[default]
    Normal,
    Copy,
    Command,
}
```

### 6.3 ServerGraph

```rust
// crates/tf-model/src/server_graph.rs

use crate::ids::*;
use crate::session::Session;
use std::collections::BTreeMap;

/// The root of the entity hierarchy.
/// All state is accessible via traversal from ServerGraph.
///
/// Inspired by libtmux's Server -> Session -> Window -> Pane hierarchy.
#[derive(Debug, Clone)]
pub struct ServerGraph {
    pub sessions: BTreeMap<SessionId, Session>,
    pub clients: BTreeMap<ClientId, ClientInfo>,
    pub id_allocator: IdAllocator,
    /// Global server options (analogous to tmux `set-option -g`).
    pub options: ServerOptions,
}

impl ServerGraph {
    pub fn new() -> Self {
        Self {
            sessions: BTreeMap::new(),
            clients: BTreeMap::new(),
            id_allocator: IdAllocator::new(0),
            options: ServerOptions::default(),
        }
    }

    /// Iterate all sessions.
    pub fn sessions(&self) -> impl Iterator<Item = &Session> {
        self.sessions.values()
    }

    /// Iterate all windows across all sessions.
    pub fn windows(&self) -> impl Iterator<Item = &Window> {
        self.sessions.values().flat_map(|s| s.windows.values())
    }

    /// Iterate all panes across all sessions and windows.
    pub fn panes(&self) -> impl Iterator<Item = &Pane> {
        self.windows().flat_map(|w| w.panes.values())
    }

    /// Find a session by name.
    pub fn session_by_name(&self, name: &str) -> Option<&Session> {
        self.sessions.values().find(|s| s.name == name)
    }
}
```

### 6.4 Queryable Trait

```rust
// crates/tf-model/src/lib.rs

/// Marker trait for entities that can be queried via QueryList.
/// Each queryable entity defines its field names for filter operations.
pub trait Queryable {
    /// Return the value for a given field name.
    /// Returns None if the field doesn't exist.
    fn field_value(&self, field: &str) -> Option<FieldValue>;

    /// List all queryable field names.
    fn field_names() -> &'static [&'static str];
}

/// Dynamic field value for query comparison.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    String(String),
    U64(u64),
    U32(u32),
    U16(u16),
    Bool(bool),
    None,
}

impl Queryable for Session {
    fn field_value(&self, field: &str) -> Option<FieldValue> {
        match field {
            "id" => Some(FieldValue::U64(self.id.raw())),
            "name" => Some(FieldValue::String(self.name.clone())),
            "attached" => Some(FieldValue::Bool(self.attached)),
            _ => None,
        }
    }

    fn field_names() -> &'static [&'static str] {
        &["id", "name", "attached"]
    }
}
```

### AGENTS.md Rules (Section 6)

```
RULE S06-ID: All entity IDs must use the define_id! macro. No raw u64 in public APIs.
RULE S06-QUERYABLE: Every new entity struct must implement Queryable with all public fields.
RULE S06-VERSION: Every entity mutation must increment the entity's version field.
RULE S06-BTREE: Use BTreeMap (not HashMap) for deterministic iteration order.
```

### Test Strategy (Section 6)

```rust
#[test]
fn server_graph_traversal() {
    let mut graph = ServerGraph::new();
    let sid = graph.id_allocator.next_session();
    graph.sessions.insert(sid, Session { id: sid, name: "test".into(), /* ... */ });
    assert_eq!(graph.sessions().count(), 1);
    assert_eq!(graph.windows().count(), 0);
}

#[test]
fn queryable_field_access() {
    let session = Session { name: "work".into(), /* ... */ };
    assert_eq!(
        session.field_value("name"),
        Some(FieldValue::String("work".into()))
    );
    assert!(session.field_value("nonexistent").is_none());
}
```

---

## 7. Event/Effect Engine

### 7.1 Command Enum

```rust
// crates/tf-event/src/command.rs

use tf_model::ids::*;

/// Commands from clients to server.
#[derive(Debug, Clone)]
pub enum Command {
    NewSession { name: String },
    KillSession { target: SessionId },
    NewWindow { session: SessionId, name: String },
    KillWindow { target: WindowId },
    SplitPane { target: PaneId, direction: SplitDirection, size: Option<PaneSize> },
    SelectPane { target: PaneId },
    SelectWindow { target: WindowId },
    ResizePane { target: PaneId, width: u16, height: u16 },
    SendKeys { target: PaneId, keys: Vec<Key> },
    ListSessions,
    ListWindows { session: SessionId },
    ListPanes { window: WindowId },
    SetOption { scope: OptionScope, key: String, value: String },
    KillServer,
}

#[derive(Debug, Clone)]
pub enum SplitDirection { Horizontal, Vertical }

#[derive(Debug, Clone)]
pub enum PaneSize {
    Cells(u16),
    Percent(u8),
}

#[derive(Debug, Clone)]
pub enum OptionScope { Global, Session(SessionId), Window(WindowId) }
```

### 7.2 Effect Enum

```rust
// crates/tf-event/src/effect.rs

use tf_model::ids::*;
use tf_model::session::{Session, Window, Pane};

/// Effects produced by processing commands.
/// Effects are the ONLY way to mutate ServerGraph.
#[derive(Debug, Clone)]
pub enum Effect {
    SessionCreated(Session),
    SessionDestroyed(SessionId),
    WindowCreated { session: SessionId, window: Window },
    WindowDestroyed(WindowId),
    PaneCreated { window: WindowId, pane: Pane },
    PaneDestroyed(PaneId),
    PaneResized { pane: PaneId, width: u16, height: u16 },
    LayoutChanged { window: WindowId, layout: LayoutTree },
    OptionChanged { scope: OptionScope, key: String, value: String },
    ClientNotification(ClientId, Notification),
    PtyWrite { pane: PaneId, data: bytes::Bytes },
    ServerShutdown,
}

/// Notifications sent to clients.
#[derive(Debug, Clone)]
pub enum Notification {
    SessionChanged { session: SessionId },
    WindowRenamed { window: WindowId, name: String },
    PaneOutput { pane: PaneId, data: bytes::Bytes },
    LayoutChange { window: WindowId },
    Exit { reason: String },
}
```

### 7.3 Effect Processor

```rust
// crates/tf-event/src/processor.rs

use tf_model::ServerGraph;

pub struct EffectProcessor;

impl EffectProcessor {
    /// Process a command against the current state, returning effects.
    /// This is a pure function: no I/O, no side effects.
    pub fn process(graph: &ServerGraph, cmd: Command) -> Vec<Effect> {
        match cmd {
            Command::NewSession { name } => {
                let id = graph.id_allocator.next_session();
                let session = Session {
                    id,
                    name,
                    created: SystemTime::now(),
                    attached: false,
                    windows: BTreeMap::new(),
                    active_window: None,
                    last_activity: SystemTime::now(),
                    version: 0,
                };
                vec![Effect::SessionCreated(session)]
            }
            Command::KillSession { target } => {
                if graph.sessions.contains_key(&target) {
                    vec![Effect::SessionDestroyed(target)]
                } else {
                    vec![] // Idempotent: no-op if already gone
                }
            }
            _ => vec![],
        }
    }

    /// Apply effects to mutate the server graph.
    pub fn apply(graph: &mut ServerGraph, effects: &[Effect]) {
        for effect in effects {
            match effect {
                Effect::SessionCreated(session) => {
                    graph.sessions.insert(session.id, session.clone());
                }
                Effect::SessionDestroyed(id) => {
                    graph.sessions.remove(id);
                }
                _ => {}
            }
        }
    }
}
```

### AGENTS.md Rules (Section 7)

```
RULE S07-PURE: EffectProcessor::process() must be a pure function. No I/O, no mutable state.
RULE S07-EFFECT: ServerGraph may ONLY be mutated via Effect application. Direct mutation is forbidden.
RULE S07-IDEMPOTENT: All commands must be safe to replay. No "already exists" panics.
```

### Test Strategy (Section 7)

```rust
#[test]
fn process_is_pure() {
    let graph = ServerGraph::new();
    let cmd = Command::NewSession { name: "test".into() };
    let effects1 = EffectProcessor::process(&graph, cmd.clone());
    let effects2 = EffectProcessor::process(&graph, cmd);
    assert_eq!(effects1.len(), effects2.len());
}

#[test]
fn kill_nonexistent_session_is_noop() {
    let graph = ServerGraph::new();
    let effects = EffectProcessor::process(&graph, Command::KillSession {
        target: SessionId::new(999),
    });
    assert!(effects.is_empty());
}
```

---

## 8. Error Handling

### 8.1 Error Hierarchy

```rust
// crates/tf-model/src/error.rs

use thiserror::Error;
use crate::ids::*;

/// Top-level error for the TermForge server.
#[derive(Debug, Error)]
pub enum TermForgeError {
    #[error("protocol error: {0}")]
    Protocol(#[from] ProtocolError),

    #[error("query error: {0}")]
    Query(#[from] QueryError),

    #[error("config error: {0}")]
    Config(#[from] ConfigError),

    #[error("pty error: {0}")]
    Pty(#[from] PtyError),

    #[error("layout error: {0}")]
    Layout(#[from] LayoutError),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Query-specific errors, matching libtmux's exception model.
/// libtmux raises ObjectDoesNotExist and MultipleObjectsReturned from .get().
#[derive(Debug, Error)]
pub enum QueryError {
    #[error("object does not exist: {filter}")]
    ObjectDoesNotExist { filter: String },

    #[error("multiple objects returned for filter: {filter} (got {count})")]
    MultipleObjectsReturned { filter: String, count: usize },

    #[error("unknown field: {field} on {entity}")]
    UnknownField { field: String, entity: String },

    #[error("invalid operator: {op} for field type {field_type}")]
    InvalidOperator { op: String, field_type: String },
}

/// Protocol errors.
#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("unknown command: {0}")]
    UnknownCommand(String),

    #[error("invalid argument: {0}")]
    InvalidArgument(String),

    #[error("session not found: {0}")]
    SessionNotFound(SessionId),

    #[error("window not found: {0}")]
    WindowNotFound(WindowId),

    #[error("pane not found: {0}")]
    PaneNotFound(PaneId),
}
```

### 8.2 Error Conversion Rules

1. **Never panic in library code** -- all fallible operations return `Result`.
2. **Use `thiserror`** for library errors, `anyhow` only in binary crates.
3. **Map errors at boundaries** -- `tf-proto` converts `QueryError` to protocol error responses.
4. **Preserve context** -- use `.context()` from `anyhow` in binary crates for error chains.

### 8.3 Error in Bindings

```rust
// Python: convert QueryError to specific Python exceptions
impl From<QueryError> for PyErr {
    fn from(err: QueryError) -> PyErr {
        match err {
            QueryError::ObjectDoesNotExist { .. } => {
                pyo3::exceptions::PyLookupError::new_err(err.to_string())
            }
            QueryError::MultipleObjectsReturned { .. } => {
                pyo3::exceptions::PyValueError::new_err(err.to_string())
            }
            _ => pyo3::exceptions::PyRuntimeError::new_err(err.to_string()),
        }
    }
}
```

### AGENTS.md Rules (Section 8)

```
RULE S08-NOPANIC: #[cfg_attr(test, allow(clippy::unwrap_used))] only in test code.
    Production code uses Result.
RULE S08-THISERROR: Library crates use thiserror. Binary crates may use anyhow.
RULE S08-CONTEXT: Every ? in binary crates must have .context() or .with_context().
```

### Test Strategy (Section 8)

```rust
#[test]
fn query_error_maps_to_python_exception() {
    let err = QueryError::ObjectDoesNotExist {
        filter: "name=nonexistent".into(),
    };
    let py_err: PyErr = err.into();
    Python::with_gil(|py| {
        assert!(py_err.is_instance_of::<pyo3::exceptions::PyLookupError>(py));
    });
}
```

---

## 9. Protocol Codec

### 9.1 Wire Format

TermForge uses the tmux control mode protocol for backward compatibility. The codec
handles both tmux-compatible text framing and a future binary framing for internal use.

```rust
// crates/tf-proto/src/codec.rs

use bytes::{Buf, BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

/// tmux control-mode line codec.
/// Each message is a UTF-8 line terminated by \n.
/// Notifications are prefixed with % (e.g., %session-changed).
pub struct ControlCodec {
    max_line_length: usize,
}

impl ControlCodec {
    pub fn new() -> Self {
        Self { max_line_length: 1024 * 1024 } // 1MB max line
    }
}

impl Decoder for ControlCodec {
    type Item = ControlMessage;
    type Error = ProtocolError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if let Some(pos) = src.iter().position(|&b| b == b'\n') {
            if pos > self.max_line_length {
                return Err(ProtocolError::InvalidArgument(
                    "line too long".into()
                ));
            }
            let line = src.split_to(pos + 1);
            let text = std::str::from_utf8(&line[..pos])
                .map_err(|_| ProtocolError::InvalidArgument("invalid UTF-8".into()))?;
            Ok(Some(ControlMessage::parse(text)?))
        } else {
            Ok(None) // Need more data
        }
    }
}

impl Encoder<ControlResponse> for ControlCodec {
    type Error = ProtocolError;

    fn encode(&mut self, item: ControlResponse, dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.put_slice(item.to_wire().as_bytes());
        dst.put_u8(b'\n');
        Ok(())
    }
}
```

### 9.2 Message Types

```rust
// crates/tf-proto/src/message.rs

/// Parsed control-mode message.
#[derive(Debug, Clone)]
pub enum ControlMessage {
    /// Client command (e.g., "new-session -s work")
    Command(String, Vec<String>),
    /// Begin block
    Begin { timestamp: u64, number: u64, flags: u32 },
    /// End block
    End { timestamp: u64, number: u64, flags: u32 },
    /// Output line within a begin/end block
    OutputLine(String),
}

impl ControlMessage {
    pub fn parse(line: &str) -> Result<Self, ProtocolError> {
        if line.starts_with("%begin") {
            Self::parse_begin(line)
        } else if line.starts_with("%end") {
            Self::parse_end(line)
        } else {
            Self::parse_command(line)
        }
    }
}

/// Response sent back to client.
#[derive(Debug, Clone)]
pub enum ControlResponse {
    Begin { timestamp: u64, number: u64, flags: u32 },
    End { timestamp: u64, number: u64, flags: u32 },
    Output(String),
    Notification(ControlNotification),
    Error(String),
}

/// Control-mode notifications (%-prefixed).
#[derive(Debug, Clone)]
pub enum ControlNotification {
    SessionChanged { session_id: u64, name: String },
    SessionRenamed { name: String },
    SessionWindowChanged { session_id: u64, window_id: u64 },
    WindowAdd { window_id: u64 },
    WindowClose { window_id: u64 },
    WindowRenamed { window_id: u64, name: String },
    Output { pane_id: u64, data: bytes::Bytes },
    PaneModeChanged { pane_id: u64 },
    LayoutChange { window_id: u64, layout: String },
    ClientDetached { client: String },
    ClientSessionChanged { client: String, session_id: u64 },
    Exit { reason: String },
}
```

### 9.3 Protocol Version Negotiation

```rust
/// Version capabilities for protocol negotiation.
pub struct ProtocolVersion {
    pub major: u16,
    pub minor: u16,
}

impl ProtocolVersion {
    pub const CURRENT: Self = Self { major: 1, minor: 0 };

    pub fn is_compatible(&self, other: &Self) -> bool {
        self.major == other.major && self.minor >= other.minor
    }
}
```

### AGENTS.md Rules (Section 9)

```
RULE S09-COMPAT: Protocol changes must maintain backward compat with tmux control mode.
RULE S09-ZEROCOPY: Use bytes::Bytes for output data. Never allocate String for PTY output.
RULE S09-FUZZ: Every parser must have a fuzz target in tests/fuzz/.
```

### Test Strategy (Section 9)

```rust
#[test]
fn round_trip_command() {
    let msg = ControlMessage::Command("new-session".into(), vec!["-s".into(), "work".into()]);
    let wire = msg.to_wire();
    let parsed = ControlMessage::parse(&wire).unwrap();
    assert_eq!(msg, parsed);
}

#[test]
fn notification_parsing() {
    let line = "%session-changed $1 work";
    let msg = ControlMessage::parse(line).unwrap();
    // Verify correct parsing of notification
}

// Fuzz target
#[cfg(fuzzing)]
fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = ControlMessage::parse(s);
    }
});
```

---

## 10. Configuration System

### 10.1 Config Model

```rust
// crates/tf-config/src/lib.rs

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Hierarchical configuration matching tmux's option scoping.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TermForgeConfig {
    /// Global server options.
    pub server: ServerConfig,
    /// Default session options.
    pub session: SessionConfig,
    /// Default window options.
    pub window: WindowConfig,
    /// Key bindings.
    pub keys: KeyConfig,
    /// Visual/style settings.
    pub style: StyleConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub default_shell: String,
    pub escape_time: u64,        // milliseconds
    pub focus_events: bool,
    pub history_limit: usize,
    pub set_clipboard: ClipboardMode,
    pub terminal_overrides: Vec<String>,
    pub default_terminal: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClipboardMode {
    On,
    Off,
    External,
}
```

### 10.2 Config Loading Priority

```rust
/// Configuration loading follows tmux precedent with priority chain:
///
/// 1. Built-in defaults (compiled in)
/// 2. System config: /etc/termforge.toml
/// 3. User config: ~/.config/termforge/config.toml
/// 4. Project config: .termforge/config.toml
/// 5. Local override: .termforge/config.local.toml (gitignored)
/// 6. CLI flags (highest priority)
///
/// This mirrors vibe-tmux's TOML config priority chain pattern.
pub fn load_config(cli_overrides: Option<&Path>) -> Result<TermForgeConfig, ConfigError> {
    let mut config = TermForgeConfig::default();

    // Layer 1: Built-in defaults (already in config via Default)

    // Layer 2: System config
    if let Some(loaded) = load_toml_file(Path::new("/etc/termforge.toml"))? {
        config.merge(loaded);
    }

    // Layer 3: User config
    if let Some(config_dir) = dirs::config_dir() {
        let user_config = config_dir.join("termforge").join("config.toml");
        if let Some(loaded) = load_toml_file(&user_config)? {
            config.merge(loaded);
        }
    }

    // Layer 4: Project config
    if let Some(loaded) = load_toml_file(Path::new(".termforge/config.toml"))? {
        config.merge(loaded);
    }

    // Layer 5: Local override
    if let Some(loaded) = load_toml_file(Path::new(".termforge/config.local.toml"))? {
        config.merge(loaded);
    }

    // Layer 6: CLI overrides
    if let Some(path) = cli_overrides {
        if let Some(loaded) = load_toml_file(path)? {
            config.merge(loaded);
        }
    }

    Ok(config)
}

fn load_toml_file(path: &Path) -> Result<Option<TermForgeConfig>, ConfigError> {
    match std::fs::read_to_string(path) {
        Ok(contents) => {
            let config: TermForgeConfig = toml::from_str(&contents)
                .map_err(|e| ConfigError::ParseError {
                    path: path.to_path_buf(),
                    source: e.to_string(),
                })?;
            Ok(Some(config))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(ConfigError::IoError {
            path: path.to_path_buf(),
            source: e,
        }),
    }
}
```

### 10.3 Hot Reload

```rust
/// Watch config files for changes and signal reload.
pub struct ConfigWatcher {
    watcher: notify::RecommendedWatcher,
    config_paths: Vec<PathBuf>,
}

impl ConfigWatcher {
    pub fn new(config_paths: Vec<PathBuf>, tx: tokio::sync::mpsc::Sender<ConfigEvent>) -> Result<Self, ConfigError> {
        let watcher = notify::recommended_watcher(move |res: Result<notify::Event, _>| {
            if let Ok(event) = res {
                if event.kind.is_modify() {
                    let _ = tx.blocking_send(ConfigEvent::FileChanged(
                        event.paths.first().cloned().unwrap_or_default()
                    ));
                }
            }
        })?;
        Ok(Self { watcher, config_paths })
    }
}
```

### AGENTS.md Rules (Section 10)

```
RULE S10-TOML: All config files use TOML format. No YAML, no JSON config.
RULE S10-MERGE: Config merging uses "last wins" for scalars, append for deny lists.
RULE S10-LOCAL: .termforge/config.local.toml must be in .gitignore template.
```

### Test Strategy (Section 10)

```rust
#[test]
fn default_config_is_valid() {
    let config = TermForgeConfig::default();
    assert!(!config.server.default_shell.is_empty());
    assert!(config.server.history_limit > 0);
}

#[test]
fn config_merge_last_wins() {
    let mut base = TermForgeConfig::default();
    let override_config = TermForgeConfig {
        server: ServerConfig { history_limit: 50000, ..base.server.clone() },
        ..base.clone()
    };
    base.merge(override_config);
    assert_eq!(base.server.history_limit, 50000);
}

#[test]
fn missing_config_file_returns_none() {
    let result = load_toml_file(Path::new("/nonexistent/config.toml")).unwrap();
    assert!(result.is_none());
}
```

---

## 11. Layout Engine

### 11.1 Layout Tree

```rust
// crates/tf-layout/src/lib.rs

use tf_model::ids::PaneId;

/// Layout tree matching tmux's layout algorithm.
/// tmux uses a binary tree where each node is either a horizontal or vertical split.
#[derive(Debug, Clone, PartialEq)]
pub enum LayoutTree {
    /// Leaf node: a single pane.
    Pane {
        id: PaneId,
        x: u16,
        y: u16,
        width: u16,
        height: u16,
    },
    /// Internal node: horizontal split (panes stacked vertically, divider is horizontal).
    Horizontal {
        x: u16,
        y: u16,
        width: u16,
        height: u16,
        children: Vec<LayoutTree>,
    },
    /// Internal node: vertical split (panes side by side, divider is vertical).
    Vertical {
        x: u16,
        y: u16,
        width: u16,
        height: u16,
        children: Vec<LayoutTree>,
    },
}

impl LayoutTree {
    /// Recompute layout for the given total width and height.
    pub fn relayout(&mut self, x: u16, y: u16, width: u16, height: u16) {
        match self {
            LayoutTree::Pane { x: px, y: py, width: pw, height: ph, .. } => {
                *px = x; *py = y; *pw = width; *ph = height;
            }
            LayoutTree::Horizontal { children, .. } => {
                let n = children.len() as u16;
                if n == 0 { return; }
                let per_child = height / n;
                let remainder = height % n;
                let mut cy = y;
                for (i, child) in children.iter_mut().enumerate() {
                    let h = per_child + if (i as u16) < remainder { 1 } else { 0 };
                    child.relayout(x, cy, width, h);
                    cy += h;
                }
            }
            LayoutTree::Vertical { children, .. } => {
                let n = children.len() as u16;
                if n == 0 { return; }
                let per_child = width / n;
                let remainder = width % n;
                let mut cx = x;
                for (i, child) in children.iter_mut().enumerate() {
                    let w = per_child + if (i as u16) < remainder { 1 } else { 0 };
                    child.relayout(cx, y, w, height);
                    cx += w;
                }
            }
        }
    }

    /// Serialize to tmux layout string format.
    /// Example: "34x17,0,0{17x17,0,0,0,17x17,18,0,1}"
    pub fn to_tmux_layout_string(&self) -> String {
        // Implementation mirrors tmux's layout_dump()
        todo!()
    }

    /// Parse from tmux layout string.
    pub fn from_tmux_layout_string(s: &str) -> Result<Self, LayoutError> {
        todo!()
    }

    /// Collect all pane IDs in this tree.
    pub fn pane_ids(&self) -> Vec<PaneId> {
        let mut result = Vec::new();
        self.collect_pane_ids(&mut result);
        result
    }

    fn collect_pane_ids(&self, out: &mut Vec<PaneId>) {
        match self {
            LayoutTree::Pane { id, .. } => out.push(*id),
            LayoutTree::Horizontal { children, .. }
            | LayoutTree::Vertical { children, .. } => {
                for child in children {
                    child.collect_pane_ids(out);
                }
            }
        }
    }
}
```

### 11.2 Layout Algorithms

```rust
/// Standard tmux layout presets.
#[derive(Debug, Clone, Copy)]
pub enum LayoutPreset {
    EvenHorizontal,
    EvenVertical,
    MainHorizontal,
    MainVertical,
    Tiled,
}

impl LayoutPreset {
    pub fn apply(&self, pane_ids: &[PaneId], width: u16, height: u16) -> LayoutTree {
        match self {
            LayoutPreset::EvenHorizontal => {
                LayoutTree::Horizontal {
                    x: 0, y: 0, width, height,
                    children: pane_ids.iter().map(|&id| LayoutTree::Pane {
                        id, x: 0, y: 0, width: 0, height: 0,
                    }).collect(),
                }
            }
            // ... other presets
            _ => todo!(),
        }
    }
}
```

### AGENTS.md Rules (Section 11)

```
RULE S11-PIXEL: Layout must match tmux output pixel-for-pixel. Use snapshot tests.
RULE S11-ROUNDTRIP: layout_string -> LayoutTree -> layout_string must be identity.
RULE S11-MINSIZE: Minimum pane size is 1x1. Layout must never produce 0-dimension panes.
```

### Test Strategy (Section 11)

```rust
#[test]
fn relayout_distributes_evenly() {
    let mut tree = LayoutTree::Vertical {
        x: 0, y: 0, width: 80, height: 24,
        children: vec![
            LayoutTree::Pane { id: PaneId::new(0), x: 0, y: 0, width: 0, height: 0 },
            LayoutTree::Pane { id: PaneId::new(1), x: 0, y: 0, width: 0, height: 0 },
        ],
    };
    tree.relayout(0, 0, 80, 24);
    // Each pane gets 40 columns
    if let LayoutTree::Vertical { children, .. } = &tree {
        if let LayoutTree::Pane { width, .. } = &children[0] {
            assert_eq!(*width, 40);
        }
    }
}

#[test]
fn tmux_layout_string_roundtrip() {
    let input = "34x17,0,0{17x17,0,0,0,17x17,18,0,1}";
    let tree = LayoutTree::from_tmux_layout_string(input).unwrap();
    let output = tree.to_tmux_layout_string();
    assert_eq!(input, output);
}
```

---

## 12. ORM-like Query API

This section defines the complete query layer, inspired by libtmux's `QueryList` from
`_internal/query_list.py`. **v8 additions**: `Nin`, `IRegex` operators (gap #5),
`get(default=...)` parameter (gap #15), callable matcher support (gap #20).

### 12.1 QueryOp Enum -- 17 Operators

```rust
// crates/tf-query/src/ops.rs

/// Query operators matching libtmux's LOOKUP_NAME_MAP plus Rust-specific extensions.
///
/// libtmux operators: eq, exact, iexact, contains, icontains, startswith,
///   istartswith, endswith, iendswith, in, nin, regex, iregex
/// Rust extensions: Gt, Gte, Lt, Lte, IsNone, IsSome
///
/// v8: Added Nin (not-in) and IRegex (case-insensitive regex) to match libtmux.
#[derive(Debug, Clone)]
pub enum QueryOp {
    // -- Exact matching --
    /// Exact equality (libtmux: `eq`, `exact`)
    Exact(FieldValue),
    /// Case-insensitive exact (libtmux: `iexact`)
    IExact(String),

    // -- Substring matching --
    /// Contains substring (libtmux: `contains`)
    Contains(String),
    /// Case-insensitive contains (libtmux: `icontains`)
    IContains(String),

    // -- Prefix/suffix matching --
    /// Starts with prefix (libtmux: `startswith`)
    StartsWith(String),
    /// Case-insensitive starts with (libtmux: `istartswith`)
    IStartsWith(String),
    /// Ends with suffix (libtmux: `endswith`)
    EndsWith(String),
    /// Case-insensitive ends with (libtmux: `iendswith`)
    IEndsWith(String),

    // -- Set membership --
    /// Value is in the given set (libtmux: `in`)
    In(Vec<FieldValue>),
    /// Value is NOT in the given set (libtmux: `nin`) -- v8 addition
    Nin(Vec<FieldValue>),

    // -- Regular expressions --
    /// Regex match (libtmux: `regex`)
    Regex(String),
    /// Case-insensitive regex match (libtmux: `iregex`) -- v8 addition
    IRegex(String),

    // -- Comparison (Rust extensions, not in libtmux) --
    /// Greater than
    Gt(FieldValue),
    /// Greater than or equal
    Gte(FieldValue),
    /// Less than
    Lt(FieldValue),
    /// Less than or equal
    Lte(FieldValue),

    // -- Nullability --
    /// Field is None/null
    IsNone,
    /// Field is Some/non-null
    IsSome,
}
```

### 12.2 QuerySpec Builder

```rust
// crates/tf-query/src/spec.rs

/// A single field filter: field name + operator.
#[derive(Debug, Clone)]
pub struct QuerySpec {
    pub field: String,
    pub op: QueryOp,
}

/// Builder for constructing queries fluently.
pub struct QueryBuilder {
    field: String,
}

impl QueryBuilder {
    pub fn new(field: &str) -> Self {
        Self { field: field.to_string() }
    }
}

/// Start building a query for a field.
pub fn q(field: &str) -> QueryBuilder {
    QueryBuilder::new(field)
}

impl QueryBuilder {
    pub fn exact(self, value: impl Into<FieldValue>) -> QuerySpec {
        QuerySpec { field: self.field, op: QueryOp::Exact(value.into()) }
    }
    pub fn iexact(self, value: &str) -> QuerySpec {
        QuerySpec { field: self.field, op: QueryOp::IExact(value.to_string()) }
    }
    pub fn contains(self, value: &str) -> QuerySpec {
        QuerySpec { field: self.field, op: QueryOp::Contains(value.to_string()) }
    }
    pub fn icontains(self, value: &str) -> QuerySpec {
        QuerySpec { field: self.field, op: QueryOp::IContains(value.to_string()) }
    }
    pub fn starts_with(self, value: &str) -> QuerySpec {
        QuerySpec { field: self.field, op: QueryOp::StartsWith(value.to_string()) }
    }
    pub fn istarts_with(self, value: &str) -> QuerySpec {
        QuerySpec { field: self.field, op: QueryOp::IStartsWith(value.to_string()) }
    }
    pub fn ends_with(self, value: &str) -> QuerySpec {
        QuerySpec { field: self.field, op: QueryOp::EndsWith(value.to_string()) }
    }
    pub fn iends_with(self, value: &str) -> QuerySpec {
        QuerySpec { field: self.field, op: QueryOp::IEndsWith(value.to_string()) }
    }
    pub fn is_in(self, values: Vec<FieldValue>) -> QuerySpec {
        QuerySpec { field: self.field, op: QueryOp::In(values) }
    }
    /// v8 addition: not-in operator matching libtmux's `nin`.
    pub fn not_in(self, values: Vec<FieldValue>) -> QuerySpec {
        QuerySpec { field: self.field, op: QueryOp::Nin(values) }
    }
    pub fn regex(self, pattern: &str) -> QuerySpec {
        QuerySpec { field: self.field, op: QueryOp::Regex(pattern.to_string()) }
    }
    /// v8 addition: case-insensitive regex matching libtmux's `iregex`.
    pub fn iregex(self, pattern: &str) -> QuerySpec {
        QuerySpec { field: self.field, op: QueryOp::IRegex(pattern.to_string()) }
    }
    pub fn gt(self, value: impl Into<FieldValue>) -> QuerySpec {
        QuerySpec { field: self.field, op: QueryOp::Gt(value.into()) }
    }
    pub fn gte(self, value: impl Into<FieldValue>) -> QuerySpec {
        QuerySpec { field: self.field, op: QueryOp::Gte(value.into()) }
    }
    pub fn lt(self, value: impl Into<FieldValue>) -> QuerySpec {
        QuerySpec { field: self.field, op: QueryOp::Lt(value.into()) }
    }
    pub fn lte(self, value: impl Into<FieldValue>) -> QuerySpec {
        QuerySpec { field: self.field, op: QueryOp::Lte(value.into()) }
    }
    pub fn is_none(self) -> QuerySpec {
        QuerySpec { field: self.field, op: QueryOp::IsNone }
    }
    pub fn is_some(self) -> QuerySpec {
        QuerySpec { field: self.field, op: QueryOp::IsSome }
    }
}

/// Parse a kwargs-style filter key into field name and operator.
/// Mirrors libtmux's `__` separator convention.
///
/// Examples:
///   "name" -> ("name", Exact)
///   "name__startswith" -> ("name", StartsWith)
///   "name__icontains" -> ("name", IContains)
///   "name__nin" -> ("name", Nin)
///   "name__iregex" -> ("name", IRegex)
pub fn parse_kwargs_key(key: &str) -> (&str, &str) {
    match key.rsplit_once("__") {
        Some((field, op)) => (field, op),
        None => (key, "exact"),
    }
}

/// Convert a kwargs operator name to QueryOp.
/// This is used by Python/Node bindings to convert kwargs to QuerySpec.
pub fn op_from_name(name: &str, value: FieldValue) -> Result<QueryOp, QueryError> {
    match name {
        "exact" | "eq" => Ok(QueryOp::Exact(value)),
        "iexact" => match value {
            FieldValue::String(s) => Ok(QueryOp::IExact(s)),
            _ => Err(QueryError::InvalidOperator {
                op: "iexact".into(), field_type: "non-string".into()
            }),
        },
        "contains" => match value {
            FieldValue::String(s) => Ok(QueryOp::Contains(s)),
            _ => Err(QueryError::InvalidOperator {
                op: "contains".into(), field_type: "non-string".into()
            }),
        },
        "icontains" => match value {
            FieldValue::String(s) => Ok(QueryOp::IContains(s)),
            _ => Err(QueryError::InvalidOperator {
                op: "icontains".into(), field_type: "non-string".into()
            }),
        },
        "startswith" => match value {
            FieldValue::String(s) => Ok(QueryOp::StartsWith(s)),
            _ => Err(QueryError::InvalidOperator {
                op: "startswith".into(), field_type: "non-string".into()
            }),
        },
        "istartswith" => match value {
            FieldValue::String(s) => Ok(QueryOp::IStartsWith(s)),
            _ => Err(QueryError::InvalidOperator {
                op: "istartswith".into(), field_type: "non-string".into()
            }),
        },
        "endswith" => match value {
            FieldValue::String(s) => Ok(QueryOp::EndsWith(s)),
            _ => Err(QueryError::InvalidOperator {
                op: "endswith".into(), field_type: "non-string".into()
            }),
        },
        "iendswith" => match value {
            FieldValue::String(s) => Ok(QueryOp::IEndsWith(s)),
            _ => Err(QueryError::InvalidOperator {
                op: "iendswith".into(), field_type: "non-string".into()
            }),
        },
        "in" => Ok(QueryOp::In(vec![value])),
        "nin" => Ok(QueryOp::Nin(vec![value])),
        "regex" => match value {
            FieldValue::String(s) => Ok(QueryOp::Regex(s)),
            _ => Err(QueryError::InvalidOperator {
                op: "regex".into(), field_type: "non-string".into()
            }),
        },
        "iregex" => match value {
            FieldValue::String(s) => Ok(QueryOp::IRegex(s)),
            _ => Err(QueryError::InvalidOperator {
                op: "iregex".into(), field_type: "non-string".into()
            }),
        },
        _ => Err(QueryError::InvalidOperator {
            op: name.into(), field_type: "unknown".into()
        }),
    }
}
```

### 12.3 QueryList -- Core Collection Type

```rust
// crates/tf-query/src/query_list.rs

use crate::ops::QueryOp;
use crate::spec::QuerySpec;
use tf_model::{FieldValue, Queryable, QueryError};
use regex::Regex;

/// A queryable, filterable collection of items.
/// This is the Rust equivalent of libtmux's QueryList class.
///
/// Supports:
/// - `filter()` with QuerySpec or callable
/// - `get()` with optional default value (v8 addition)
/// - All 17 operators including Nin and IRegex (v8 additions)
/// - Iterator, indexing, and length
pub struct QueryList<T: Queryable> {
    items: Vec<T>,
}

impl<T: Queryable + Clone> QueryList<T> {
    pub fn new(items: Vec<T>) -> Self {
        Self { items }
    }

    pub fn items(&self) -> &[T] {
        &self.items
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Filter items by one or more QuerySpecs (AND semantics).
    /// Returns a new QueryList containing only matching items.
    pub fn filter(&self, specs: &[QuerySpec]) -> Self {
        let filtered = self.items.iter()
            .filter(|item| specs.iter().all(|spec| matches_spec(*item, spec)))
            .cloned()
            .collect();
        Self { items: filtered }
    }

    /// Filter items using a callable predicate.
    /// Mirrors libtmux's `filter(lambda s: ...)` support.
    /// v8 addition: callable matcher support.
    pub fn filter_fn(&self, predicate: impl Fn(&T) -> bool) -> Self {
        let filtered = self.items.iter()
            .filter(|item| predicate(item))
            .cloned()
            .collect();
        Self { items: filtered }
    }

    /// Get exactly one item matching the specs.
    /// Returns ObjectDoesNotExist if no match, MultipleObjectsReturned if >1.
    /// Mirrors libtmux's `.get()` behavior.
    pub fn get(&self, specs: &[QuerySpec]) -> Result<&T, QueryError> {
        let matches: Vec<_> = self.items.iter()
            .filter(|item| specs.iter().all(|spec| matches_spec(*item, spec)))
            .collect();
        match matches.len() {
            0 => Err(QueryError::ObjectDoesNotExist {
                filter: format!("{:?}", specs),
            }),
            1 => Ok(matches[0]),
            n => Err(QueryError::MultipleObjectsReturned {
                filter: format!("{:?}", specs),
                count: n,
            }),
        }
    }

    /// Get exactly one item, or return default if not found.
    /// Returns MultipleObjectsReturned if >1 match (never silent).
    /// v8 addition: `default` parameter matching libtmux's `get(default=None)`.
    pub fn get_or_default<'a>(
        &'a self,
        specs: &[QuerySpec],
        default: &'a T,
    ) -> Result<&'a T, QueryError> {
        match self.get(specs) {
            Ok(item) => Ok(item),
            Err(QueryError::ObjectDoesNotExist { .. }) => Ok(default),
            Err(e) => Err(e), // MultipleObjectsReturned still errors
        }
    }

    /// Get the first matching item, or None.
    pub fn first(&self, specs: &[QuerySpec]) -> Option<&T> {
        self.items.iter()
            .find(|item| specs.iter().all(|spec| matches_spec(*item, spec)))
    }

    /// Index into the list.
    pub fn get_index(&self, index: usize) -> Option<&T> {
        self.items.get(index)
    }

    /// Iterate over all items.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.items.iter()
    }
}

impl<T: Queryable + Clone> IntoIterator for QueryList<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

/// Evaluate whether an item matches a single QuerySpec.
fn matches_spec<T: Queryable>(item: &T, spec: &QuerySpec) -> bool {
    let field_val = match item.field_value(&spec.field) {
        Some(v) => v,
        None => return matches!(spec.op, QueryOp::IsNone),
    };

    match &spec.op {
        QueryOp::Exact(expected) => field_val == *expected,
        QueryOp::IExact(expected) => match &field_val {
            FieldValue::String(s) => s.to_lowercase() == expected.to_lowercase(),
            _ => false,
        },
        QueryOp::Contains(needle) => match &field_val {
            FieldValue::String(s) => s.contains(needle.as_str()),
            _ => false,
        },
        QueryOp::IContains(needle) => match &field_val {
            FieldValue::String(s) => s.to_lowercase().contains(&needle.to_lowercase()),
            _ => false,
        },
        QueryOp::StartsWith(prefix) => match &field_val {
            FieldValue::String(s) => s.starts_with(prefix.as_str()),
            _ => false,
        },
        QueryOp::IStartsWith(prefix) => match &field_val {
            FieldValue::String(s) => s.to_lowercase().starts_with(&prefix.to_lowercase()),
            _ => false,
        },
        QueryOp::EndsWith(suffix) => match &field_val {
            FieldValue::String(s) => s.ends_with(suffix.as_str()),
            _ => false,
        },
        QueryOp::IEndsWith(suffix) => match &field_val {
            FieldValue::String(s) => s.to_lowercase().ends_with(&suffix.to_lowercase()),
            _ => false,
        },
        QueryOp::In(values) => values.contains(&field_val),
        // v8 addition: Nin operator
        QueryOp::Nin(values) => !values.contains(&field_val),
        QueryOp::Regex(pattern) => match &field_val {
            FieldValue::String(s) => {
                Regex::new(pattern).map_or(false, |re| re.is_match(s))
            }
            _ => false,
        },
        // v8 addition: IRegex operator
        QueryOp::IRegex(pattern) => match &field_val {
            FieldValue::String(s) => {
                // Prepend (?i) for case-insensitive matching
                let ci_pattern = format!("(?i){}", pattern);
                Regex::new(&ci_pattern).map_or(false, |re| re.is_match(s))
            }
            _ => false,
        },
        QueryOp::Gt(expected) => field_val_cmp(&field_val, expected) == Some(std::cmp::Ordering::Greater),
        QueryOp::Gte(expected) => matches!(field_val_cmp(&field_val, expected), Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)),
        QueryOp::Lt(expected) => field_val_cmp(&field_val, expected) == Some(std::cmp::Ordering::Less),
        QueryOp::Lte(expected) => matches!(field_val_cmp(&field_val, expected), Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)),
        QueryOp::IsNone => field_val == FieldValue::None,
        QueryOp::IsSome => field_val != FieldValue::None,
    }
}

/// Compare two FieldValues for ordering.
fn field_val_cmp(a: &FieldValue, b: &FieldValue) -> Option<std::cmp::Ordering> {
    match (a, b) {
        (FieldValue::U64(a), FieldValue::U64(b)) => Some(a.cmp(b)),
        (FieldValue::U32(a), FieldValue::U32(b)) => Some(a.cmp(b)),
        (FieldValue::U16(a), FieldValue::U16(b)) => Some(a.cmp(b)),
        (FieldValue::String(a), FieldValue::String(b)) => Some(a.cmp(b)),
        _ => None,
    }
}
```

### 12.4 Kwargs Parser for Bindings

```rust
// crates/tf-query/src/kwargs.rs

/// Parse kwargs-style filters into QuerySpecs.
/// This enables `server.sessions.filter(name="work")` in Python
/// and `server.sessions({ name: "work" })` in Node.js.
///
/// Format: field__operator=value
///   "name" -> field="name", op=Exact
///   "name__startswith" -> field="name", op=StartsWith
///   "name__iregex" -> field="name", op=IRegex (v8)
///   "name__nin" -> field="name", op=Nin (v8)
pub fn kwargs_to_specs(
    kwargs: &[(String, FieldValue)],
) -> Result<Vec<QuerySpec>, QueryError> {
    kwargs.iter().map(|(key, value)| {
        let (field, op_name) = crate::spec::parse_kwargs_key(key);
        let op = crate::spec::op_from_name(op_name, value.clone())?;
        Ok(QuerySpec {
            field: field.to_string(),
            op,
        })
    }).collect()
}
```

### AGENTS.md Rules (Section 12)

```
RULE S12-OPERATOR: All 17 operators must be tested. Adding a new operator requires
    tests for both matching and non-matching cases.
RULE S12-KWARGS: Python kwargs filter must use __ separator convention from libtmux.
RULE S12-GET: get() must raise ObjectDoesNotExist (never return None silently) unless
    default is explicitly provided via get_or_default().
RULE S12-REGEX: Regex compilation errors must be caught and returned as InvalidOperator,
    never panic.
```

### Test Strategy (Section 12)

```rust
#[test]
fn filter_exact() {
    let sessions = QueryList::new(vec![
        Session { name: "work".into(), ..default() },
        Session { name: "home".into(), ..default() },
    ]);
    let result = sessions.filter(&[q("name").exact(FieldValue::String("work".into()))]);
    assert_eq!(result.len(), 1);
    assert_eq!(result.items()[0].name, "work");
}

#[test]
fn filter_nin_operator() {
    // v8: test Nin operator
    let sessions = QueryList::new(vec![
        Session { name: "work".into(), ..default() },
        Session { name: "home".into(), ..default() },
        Session { name: "play".into(), ..default() },
    ]);
    let result = sessions.filter(&[q("name").not_in(vec![
        FieldValue::String("work".into()),
        FieldValue::String("home".into()),
    ])]);
    assert_eq!(result.len(), 1);
    assert_eq!(result.items()[0].name, "play");
}

#[test]
fn filter_iregex_operator() {
    // v8: test IRegex operator
    let sessions = QueryList::new(vec![
        Session { name: "WorkSession".into(), ..default() },
        Session { name: "homework".into(), ..default() },
    ]);
    let result = sessions.filter(&[q("name").iregex("^work")]);
    assert_eq!(result.len(), 2); // Both match case-insensitively
}

#[test]
fn get_returns_object_does_not_exist() {
    let sessions = QueryList::<Session>::new(vec![]);
    let result = sessions.get(&[q("name").exact(FieldValue::String("nope".into()))]);
    assert!(matches!(result, Err(QueryError::ObjectDoesNotExist { .. })));
}

#[test]
fn get_returns_multiple_objects_returned() {
    let sessions = QueryList::new(vec![
        Session { name: "work".into(), ..default() },
        Session { name: "work".into(), ..default() },
    ]);
    let result = sessions.get(&[q("name").exact(FieldValue::String("work".into()))]);
    assert!(matches!(result, Err(QueryError::MultipleObjectsReturned { count: 2, .. })));
}

#[test]
fn get_or_default_returns_default_on_missing() {
    // v8: test get(default=...) parameter
    let sessions = QueryList::<Session>::new(vec![]);
    let fallback = Session { name: "default".into(), ..default() };
    let result = sessions.get_or_default(
        &[q("name").exact(FieldValue::String("nope".into()))],
        &fallback,
    ).unwrap();
    assert_eq!(result.name, "default");
}

#[test]
fn get_or_default_still_errors_on_multiple() {
    // v8: get(default=...) does NOT suppress MultipleObjectsReturned
    let sessions = QueryList::new(vec![
        Session { name: "x".into(), ..default() },
        Session { name: "x".into(), ..default() },
    ]);
    let fallback = Session { name: "default".into(), ..default() };
    let result = sessions.get_or_default(
        &[q("name").exact(FieldValue::String("x".into()))],
        &fallback,
    );
    assert!(matches!(result, Err(QueryError::MultipleObjectsReturned { .. })));
}

#[test]
fn filter_fn_callable_matcher() {
    // v8: callable matcher support
    let sessions = QueryList::new(vec![
        Session { name: "work-main".into(), ..default() },
        Session { name: "work-dev".into(), ..default() },
        Session { name: "personal".into(), ..default() },
    ]);
    let result = sessions.filter_fn(|s| s.name.starts_with("work"));
    assert_eq!(result.len(), 2);
}

#[test]
fn kwargs_parsing() {
    let kwargs = vec![
        ("name".to_string(), FieldValue::String("work".into())),
        ("name__startswith".to_string(), FieldValue::String("wo".into())),
    ];
    let specs = kwargs_to_specs(&kwargs).unwrap();
    assert_eq!(specs.len(), 2);
    assert!(matches!(specs[0].op, QueryOp::Exact(_)));
    assert!(matches!(specs[1].op, QueryOp::StartsWith(_)));
}
```

---

## 13. Runtime Architecture

### 13.1 Async Runtime

```rust
// crates/tf-server/src/runtime.rs

/// TermForge server uses a multi-threaded Tokio runtime.
/// The main components are:
///
/// 1. Socket listener task -- accepts new client connections
/// 2. Per-client codec task -- reads/writes control protocol
/// 3. Event loop task -- processes commands, applies effects
/// 4. PTY I/O tasks -- reads output from child processes
/// 5. OTEL export task -- batches and ships telemetry (separate runtime)
///
/// The event loop is single-threaded (serializes state mutations).
/// PTY I/O and client codec tasks run on the multi-threaded pool.

pub struct ServerRuntime {
    rt: tokio::runtime::Runtime,
    graph: Arc<RwLock<ServerGraph>>,
    event_tx: mpsc::Sender<Command>,
    shutdown: CancellationToken,
}

impl ServerRuntime {
    pub fn new() -> Result<Self, TermForgeError> {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("termforge-worker")
            .build()?;

        let graph = Arc::new(RwLock::new(ServerGraph::new()));
        let (event_tx, event_rx) = mpsc::channel(4096);
        let shutdown = CancellationToken::new();

        Ok(Self { rt, graph, event_tx, shutdown })
    }

    pub fn run(&self, socket_path: &Path) -> Result<(), TermForgeError> {
        self.rt.block_on(async {
            let listener = UnixListener::bind(socket_path)?;

            // Spawn event loop
            let graph = self.graph.clone();
            let shutdown = self.shutdown.clone();
            tokio::spawn(event_loop(graph, self.event_rx, shutdown.clone()));

            // Accept loop
            loop {
                tokio::select! {
                    result = listener.accept() => {
                        let (stream, _) = result?;
                        let tx = self.event_tx.clone();
                        let graph = self.graph.clone();
                        tokio::spawn(handle_client(stream, tx, graph));
                    }
                    _ = shutdown.cancelled() => break,
                }
            }
            Ok(())
        })
    }
}
```

### 13.2 Event Loop

```rust
async fn event_loop(
    graph: Arc<RwLock<ServerGraph>>,
    mut rx: mpsc::Receiver<Command>,
    shutdown: CancellationToken,
) {
    loop {
        tokio::select! {
            Some(cmd) = rx.recv() => {
                let mut g = graph.write().await;
                let effects = EffectProcessor::process(&g, cmd);
                EffectProcessor::apply(&mut g, &effects);
                // Broadcast notifications to connected clients
                for effect in &effects {
                    if let Effect::ClientNotification(client_id, notification) = effect {
                        // Send to client's channel
                    }
                    if matches!(effect, Effect::ServerShutdown) {
                        shutdown.cancel();
                        return;
                    }
                }
            }
            _ = shutdown.cancelled() => return,
        }
    }
}
```

### AGENTS.md Rules (Section 13)

```
RULE S13-EVENTLOOP: The event loop is single-threaded. Never spawn blocking work on it.
RULE S13-LOCK: graph.write() must be held for <1ms. Move heavy work to spawn_blocking.
RULE S13-CANCEL: All async tasks must respect CancellationToken for graceful shutdown.
```

### Test Strategy (Section 13)

```rust
#[tokio::test]
async fn event_loop_processes_commands() {
    let graph = Arc::new(RwLock::new(ServerGraph::new()));
    let (tx, rx) = mpsc::channel(16);
    let shutdown = CancellationToken::new();
    let g = graph.clone();
    let s = shutdown.clone();
    let handle = tokio::spawn(event_loop(g, rx, s));

    tx.send(Command::NewSession { name: "test".into() }).await.unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(graph.read().await.sessions().count(), 1);

    shutdown.cancel();
    handle.await.unwrap();
}
```

---

## 14. Server Lifecycle

### 14.1 Startup Sequence

```
1. Parse CLI arguments
2. Load config (priority chain: defaults < system < user < project < local < CLI)
3. Initialize telemetry (tf-otel dual provider)
4. Create socket (check for existing server)
5. Set socket permissions (0700)
6. Start event loop
7. Start socket listener
8. Log startup complete with OTEL span
```

### 14.2 Shutdown Sequence

```
1. Receive kill-server command or SIGTERM
2. Cancel all async tasks via CancellationToken
3. Wait for in-flight commands to complete (bounded timeout: 5s)
4. Flush OTEL providers (force_flush on both OTEL_PROVIDER and CLIENT_PROVIDER)
5. Close all client connections
6. Remove socket file
7. Exit process
```

### 14.3 Socket Management

```rust
pub fn create_server_socket(path: &Path) -> Result<UnixListener, TermForgeError> {
    // Remove stale socket if exists
    if path.exists() {
        // Check if another server is running
        match UnixStream::connect(path) {
            Ok(_) => return Err(TermForgeError::Protocol(
                ProtocolError::InvalidArgument("server already running".into())
            )),
            Err(_) => {
                // Stale socket, remove it
                std::fs::remove_file(path)?;
            }
        }
    }

    // Create parent directory with 0700 permissions
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))?;
        }
    }

    let listener = UnixListener::bind(path)?;
    Ok(listener)
}
```

### AGENTS.md Rules (Section 14)

```
RULE S14-SOCKET: Socket directories must have 0700 permissions. No world-readable sockets.
RULE S14-STALE: Always check for stale sockets before binding. Never silently overwrite.
RULE S14-SHUTDOWN: Shutdown must complete within 10s. After that, force-exit.
```

### Test Strategy (Section 14)

```rust
#[test]
fn stale_socket_is_cleaned_up() {
    let dir = tempfile::tempdir().unwrap();
    let socket_path = dir.path().join("test.sock");
    // Create a stale socket file
    std::fs::write(&socket_path, "").unwrap();
    // Should succeed (stale socket removed)
    let listener = create_server_socket(&socket_path).unwrap();
    drop(listener);
}

#[test]
fn active_server_prevents_start() {
    let dir = tempfile::tempdir().unwrap();
    let socket_path = dir.path().join("test.sock");
    let _listener1 = UnixListener::bind(&socket_path).unwrap();
    // Second bind should fail with "server already running"
    let result = create_server_socket(&socket_path);
    assert!(result.is_err());
}
```

---

## 15. Control Mode

### 15.1 Control Mode Protocol

TermForge implements tmux's `-CC` control mode protocol for compatibility with
existing tmux clients (iTerm2, tmux-client libraries).

```rust
/// Control mode session state.
pub struct ControlSession {
    client_id: ClientId,
    codec: ControlCodec,
    /// Pending begin/end blocks for command responses.
    pending_commands: VecDeque<PendingCommand>,
    /// Rate limiter for notifications.
    rate_limiter: RateLimiter,
}

struct PendingCommand {
    number: u64,
    timestamp: u64,
}

impl ControlSession {
    pub fn new(client_id: ClientId) -> Self {
        Self {
            client_id,
            codec: ControlCodec::new(),
            pending_commands: VecDeque::new(),
            rate_limiter: RateLimiter::new(1000, Duration::from_secs(1)), // 1000 msgs/sec
        }
    }

    /// Format a command response as begin/end block.
    pub fn format_response(&mut self, output: &str) -> Vec<ControlResponse> {
        let number = self.pending_commands.len() as u64;
        let timestamp = unix_timestamp();
        vec![
            ControlResponse::Begin { timestamp, number, flags: 0 },
            ControlResponse::Output(output.to_string()),
            ControlResponse::End { timestamp, number, flags: 0 },
        ]
    }
}
```

### 15.2 Notification Throttling

```rust
/// Rate limiter for control mode notifications.
/// Prevents a runaway pane from flooding the control client.
pub struct RateLimiter {
    max_per_window: usize,
    window: Duration,
    tokens: usize,
    last_refill: Instant,
}

impl RateLimiter {
    pub fn new(max_per_window: usize, window: Duration) -> Self {
        Self {
            max_per_window,
            window,
            tokens: max_per_window,
            last_refill: Instant::now(),
        }
    }

    pub fn try_acquire(&mut self) -> bool {
        self.refill();
        if self.tokens > 0 {
            self.tokens -= 1;
            true
        } else {
            false
        }
    }

    fn refill(&mut self) {
        let elapsed = self.last_refill.elapsed();
        if elapsed >= self.window {
            self.tokens = self.max_per_window;
            self.last_refill = Instant::now();
        }
    }
}
```

### AGENTS.md Rules (Section 15)

```
RULE S15-COMPAT: Control mode output must be byte-identical to tmux for the same input.
RULE S15-RATE: Default rate limit is 1000 notifications/second per client. Configurable.
RULE S15-BLOCK: Begin/end blocks must always be paired. Track with pending_commands.
```

### Test Strategy (Section 15)

```rust
#[test]
fn begin_end_blocks_are_paired() {
    let mut session = ControlSession::new(ClientId::new(0));
    let responses = session.format_response("output line");
    assert!(matches!(responses[0], ControlResponse::Begin { .. }));
    assert!(matches!(responses[2], ControlResponse::End { .. }));
}

#[test]
fn rate_limiter_throttles() {
    let mut limiter = RateLimiter::new(2, Duration::from_secs(1));
    assert!(limiter.try_acquire()); // 1 of 2
    assert!(limiter.try_acquire()); // 2 of 2
    assert!(!limiter.try_acquire()); // exhausted
}
```

---

## 16. Language Bindings

This section defines the Python (PyO3) and Node.js (Neon) binding layers.
**v8 critical fixes**: `PyQueryList` class (gap #1) and `JsQueryList` equivalent (gap #2).

### 16.1 Python Binding Architecture

The Python binding exposes the full Server -> Session -> Window -> Pane hierarchy
with libtmux-compatible API:

```python
# Target Python API (what users see)
from termforge import Server

server = Server()

# QueryList-style access (v8: PyQueryList)
sessions = server.sessions                         # PyQueryList
work = server.sessions.filter(name="work")         # PyQueryList (filtered)
work = server.sessions.filter(name__startswith="wo")  # kwargs with __ operator
session = server.sessions.get(name="work")         # single item or raises
session = server.sessions.get(name="nope", default=None)  # v8: default parameter

# Traversal
for session in server.sessions:
    for window in session.windows:
        for pane in window.panes:
            print(pane.capture_pane())

# Callable filter (v8)
active = server.sessions.filter(lambda s: s.attached)
```

### 16.2 PyQueryList Class

```rust
// bindings/python/src/py_query_list.rs

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use tf_query::{QueryList, QuerySpec, kwargs_to_specs};
use tf_model::{FieldValue, Queryable, QueryError};

/// Python wrapper around Rust QueryList<T>.
/// Provides libtmux-compatible filter/get API with kwargs support.
///
/// v8 critical fix: This class was missing from v7 spec.
#[pyclass]
pub struct PyQueryList {
    /// The inner items, stored as Python objects for flexibility.
    items: Vec<PyObject>,
    /// The Rust QueryList for efficient filtering.
    query_list: QueryList<DynQueryable>,
}

/// Dynamic wrapper that implements Queryable for Python binding objects.
#[derive(Clone)]
struct DynQueryable {
    fields: std::collections::BTreeMap<String, FieldValue>,
    py_obj: PyObject,
}

impl Queryable for DynQueryable {
    fn field_value(&self, field: &str) -> Option<FieldValue> {
        self.fields.get(field).cloned()
    }

    fn field_names() -> &'static [&'static str] {
        // Dynamic; field_names are per-entity-type
        &[]
    }
}

#[pymethods]
impl PyQueryList {
    /// Filter items by keyword arguments.
    ///
    /// Supports libtmux's __ operator syntax:
    ///   .filter(name="work")              # exact match
    ///   .filter(name__startswith="wo")     # prefix match
    ///   .filter(name__icontains="WORK")    # case-insensitive contains
    ///   .filter(name__regex="^work.*$")    # regex match
    ///   .filter(name__nin=["a", "b"])      # not-in (v8)
    ///   .filter(name__iregex="^work")      # case-insensitive regex (v8)
    ///
    /// Also accepts a callable as first positional argument:
    ///   .filter(lambda s: s.name.startswith("work"))
    #[pyo3(signature = (*args, **kwargs))]
    fn filter(
        &self,
        py: Python<'_>,
        args: &pyo3::types::PyTuple,
        kwargs: Option<&PyDict>,
    ) -> PyResult<PyQueryList> {
        // Case 1: callable filter (v8 addition)
        if args.len() == 1 {
            let callable = args.get_item(0)?;
            if callable.is_callable() {
                let filtered: Vec<_> = self.items.iter()
                    .filter(|item| {
                        callable.call1((item.clone_ref(py),))
                            .and_then(|r| r.is_truthy())
                            .unwrap_or(false)
                    })
                    .cloned()
                    .collect();
                return Ok(PyQueryList::from_py_objects(py, filtered)?);
            }
        }

        // Case 2: kwargs filter
        let Some(kwargs) = kwargs else {
            return Ok(self.clone());
        };

        let specs = pydict_to_specs(py, kwargs)?;
        let filtered_rust = self.query_list.filter(&specs);
        let filtered_py: Vec<PyObject> = filtered_rust.iter()
            .map(|item| item.py_obj.clone())
            .collect();

        Ok(PyQueryList {
            items: filtered_py,
            query_list: filtered_rust,
        })
    }

    /// Get exactly one item matching kwargs.
    /// Raises LookupError if not found (unless default is provided).
    /// Raises ValueError if multiple items match.
    ///
    /// v8 fix: supports `default` parameter matching libtmux's get(default=None).
    #[pyo3(signature = (**kwargs))]
    fn get(
        &self,
        py: Python<'_>,
        kwargs: Option<&PyDict>,
    ) -> PyResult<PyObject> {
        let kwargs = kwargs.ok_or_else(|| {
            pyo3::exceptions::PyTypeError::new_err("get() requires keyword arguments")
        })?;

        // Extract 'default' parameter if present (v8 addition)
        let default_val = kwargs.get_item("default")
            .ok()
            .flatten()
            .map(|v| v.to_object(py));

        // Build specs from remaining kwargs (exclude 'default')
        let specs = pydict_to_specs_excluding(py, kwargs, &["default"])?;

        match self.query_list.get(&specs) {
            Ok(item) => Ok(item.py_obj.clone_ref(py)),
            Err(QueryError::ObjectDoesNotExist { .. }) => {
                if let Some(default) = default_val {
                    Ok(default)
                } else {
                    Err(pyo3::exceptions::PyLookupError::new_err(
                        "object does not exist"
                    ))
                }
            }
            Err(QueryError::MultipleObjectsReturned { count, .. }) => {
                Err(pyo3::exceptions::PyValueError::new_err(
                    format!("get() returned {} objects, expected 1", count)
                ))
            }
            Err(e) => Err(pyo3::exceptions::PyRuntimeError::new_err(e.to_string())),
        }
    }

    fn __len__(&self) -> usize {
        self.items.len()
    }

    fn __iter__(&self, py: Python<'_>) -> PyResult<PyObject> {
        let list = PyList::new(py, &self.items)?;
        list.call_method0("__iter__")
    }

    fn __getitem__(&self, index: isize) -> PyResult<PyObject> {
        let idx = if index < 0 {
            (self.items.len() as isize + index) as usize
        } else {
            index as usize
        };
        self.items.get(idx)
            .cloned()
            .ok_or_else(|| pyo3::exceptions::PyIndexError::new_err("index out of range"))
    }

    fn __repr__(&self) -> String {
        format!("QueryList(len={})", self.items.len())
    }

    fn __bool__(&self) -> bool {
        !self.items.is_empty()
    }
}

/// Convert a Python dict to QuerySpecs using __ syntax.
fn pydict_to_specs(py: Python<'_>, dict: &PyDict) -> PyResult<Vec<QuerySpec>> {
    pydict_to_specs_excluding(py, dict, &[])
}

fn pydict_to_specs_excluding(
    py: Python<'_>,
    dict: &PyDict,
    exclude: &[&str],
) -> PyResult<Vec<QuerySpec>> {
    let mut pairs = Vec::new();
    for (key, value) in dict.iter() {
        let key_str: String = key.extract()?;
        if exclude.contains(&key_str.as_str()) {
            continue;
        }
        let field_val = py_to_field_value(py, &value)?;
        pairs.push((key_str, field_val));
    }
    kwargs_to_specs(&pairs).map_err(|e| {
        pyo3::exceptions::PyRuntimeError::new_err(e.to_string())
    })
}

fn py_to_field_value(py: Python<'_>, obj: &Bound<'_, PyAny>) -> PyResult<FieldValue> {
    if let Ok(s) = obj.extract::<String>() {
        Ok(FieldValue::String(s))
    } else if let Ok(n) = obj.extract::<u64>() {
        Ok(FieldValue::U64(n))
    } else if let Ok(b) = obj.extract::<bool>() {
        Ok(FieldValue::Bool(b))
    } else if obj.is_none() {
        Ok(FieldValue::None)
    } else {
        Ok(FieldValue::String(obj.str()?.to_string()))
    }
}
```

### 16.3 PyServer with QueryList Properties

```rust
// bindings/python/src/py_server.rs

#[pyclass]
pub struct PyServer {
    inner: Arc<RwLock<ServerGraph>>,
}

#[pymethods]
impl PyServer {
    #[new]
    fn new(socket_path: Option<&str>) -> PyResult<Self> {
        // Connect to existing server or start embedded
        todo!()
    }

    /// Returns a PyQueryList of all sessions.
    /// Enables: server.sessions.filter(name="work")
    #[getter]
    fn sessions(&self, py: Python<'_>) -> PyResult<PyQueryList> {
        let graph = self.inner.read()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        let sessions: Vec<PyObject> = graph.sessions()
            .map(|s| PySession::from(s).into_py(py))
            .collect();
        PyQueryList::from_py_objects(py, sessions)
    }

    /// Returns a PyQueryList of all windows across all sessions.
    #[getter]
    fn windows(&self, py: Python<'_>) -> PyResult<PyQueryList> {
        let graph = self.inner.read()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        let windows: Vec<PyObject> = graph.windows()
            .map(|w| PyWindow::from(w).into_py(py))
            .collect();
        PyQueryList::from_py_objects(py, windows)
    }

    /// Returns a PyQueryList of all panes across all sessions.
    #[getter]
    fn panes(&self, py: Python<'_>) -> PyResult<PyQueryList> {
        let graph = self.inner.read()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        let panes: Vec<PyObject> = graph.panes()
            .map(|p| PyPane::from(p).into_py(py))
            .collect();
        PyQueryList::from_py_objects(py, panes)
    }
}

#[pyclass]
pub struct PySession {
    inner: Session,
}

#[pymethods]
impl PySession {
    #[getter]
    fn id(&self) -> u64 { self.inner.id.raw() }

    #[getter]
    fn name(&self) -> &str { &self.inner.name }

    #[getter]
    fn attached(&self) -> bool { self.inner.attached }

    /// Returns a PyQueryList of windows in this session.
    #[getter]
    fn windows(&self, py: Python<'_>) -> PyResult<PyQueryList> {
        let windows: Vec<PyObject> = self.inner.windows.values()
            .map(|w| PyWindow::from(w).into_py(py))
            .collect();
        PyQueryList::from_py_objects(py, windows)
    }
}
```

### 16.4 Node.js Binding with JsQueryList

```rust
// bindings/node/src/js_query_list.rs

use neon::prelude::*;
use tf_query::{QueryList, QuerySpec, kwargs_to_specs};
use tf_model::{FieldValue, Queryable, QueryError};

/// Node.js wrapper around Rust QueryList<T>.
/// Provides object-based filter API: server.sessions({ name: "work" })
///
/// v8 critical fix: This was missing from v7 spec.
///
/// Usage in JavaScript:
///   const sessions = server.sessions();           // JsQueryList
///   const work = sessions.filter({ name: "work" }); // filtered
///   const s = sessions.get({ name: "work" });     // single item or throws
///   const s = sessions.get({ name: "x", default: null }); // v8: default
///   sessions.filter({ name__startswith: "wo" });  // __ operator syntax
///   sessions.filter({ name__nin: ["a", "b"] });   // v8: Nin operator
///   sessions.filter({ name__iregex: "^work" });   // v8: IRegex operator
pub struct JsQueryList {
    items: Vec<Root<JsObject>>,
    query_list: QueryList<DynQueryable>,
}

impl Finalize for JsQueryList {}

impl JsQueryList {
    /// Create filter method for Node.js.
    pub fn js_filter(mut cx: FunctionContext) -> JsResult<JsArray> {
        let this = cx.this::<JsBox<JsQueryList>>()?;

        // Accept either a filter object or a callback function
        let arg = cx.argument::<JsValue>(0)?;

        if let Ok(callback) = arg.downcast::<JsFunction, _>(&mut cx) {
            // Callable filter (v8 addition)
            let filtered: Vec<_> = this.items.iter()
                .filter(|item| {
                    let obj = item.to_inner(&mut cx);
                    callback.call_with(&cx)
                        .arg(obj)
                        .apply::<JsBoolean, _>(&mut cx)
                        .map(|b| b.value(&mut cx))
                        .unwrap_or(false)
                })
                .cloned()
                .collect();
            let arr = JsArray::new(&mut cx, filtered.len());
            for (i, item) in filtered.iter().enumerate() {
                let obj = item.to_inner(&mut cx);
                arr.set(&mut cx, i as u32, obj)?;
            }
            Ok(arr)
        } else if let Ok(filter_obj) = arg.downcast::<JsObject, _>(&mut cx) {
            // Object-based filter
            let specs = js_obj_to_specs(&mut cx, &filter_obj)?;
            let filtered = this.query_list.filter(&specs);
            let arr = JsArray::new(&mut cx, filtered.len());
            for (i, item) in filtered.iter().enumerate() {
                let obj = item.py_obj.to_inner(&mut cx);
                arr.set(&mut cx, i as u32, obj)?;
            }
            Ok(arr)
        } else {
            cx.throw_type_error("filter() expects an object or function argument")
        }
    }

    /// Get method: returns exactly one match or throws.
    /// Supports `default` key in the filter object (v8 addition).
    pub fn js_get(mut cx: FunctionContext) -> JsResult<JsValue> {
        let this = cx.this::<JsBox<JsQueryList>>()?;
        let filter_obj = cx.argument::<JsObject>(0)?;

        // Extract 'default' if present (v8 addition)
        let default_val = filter_obj.get_opt::<JsValue, _, _>(&mut cx, "default")?;

        let specs = js_obj_to_specs_excluding(&mut cx, &filter_obj, &["default"])?;

        match this.query_list.get(&specs) {
            Ok(item) => {
                let obj = item.py_obj.to_inner(&mut cx);
                Ok(obj.upcast())
            }
            Err(QueryError::ObjectDoesNotExist { .. }) => {
                if let Some(default) = default_val {
                    Ok(default)
                } else {
                    cx.throw_error("object does not exist")
                }
            }
            Err(QueryError::MultipleObjectsReturned { count, .. }) => {
                cx.throw_error(format!("get() returned {} objects, expected 1", count))
            }
            Err(e) => cx.throw_error(e.to_string()),
        }
    }
}

fn js_obj_to_specs(
    cx: &mut FunctionContext,
    obj: &Handle<JsObject>,
) -> NeonResult<Vec<QuerySpec>> {
    js_obj_to_specs_excluding(cx, obj, &[])
}

fn js_obj_to_specs_excluding(
    cx: &mut FunctionContext,
    obj: &Handle<JsObject>,
    exclude: &[&str],
) -> NeonResult<Vec<QuerySpec>> {
    let prop_names = obj.get_own_property_names(cx)?;
    let mut pairs = Vec::new();
    for i in 0..prop_names.len(cx) {
        let key: Handle<JsString> = prop_names.get(cx, i)?;
        let key_str = key.value(cx);
        if exclude.contains(&key_str.as_str()) {
            continue;
        }
        let value: Handle<JsValue> = obj.get(cx, key)?;
        let field_val = js_to_field_value(cx, &value)?;
        pairs.push((key_str, field_val));
    }
    kwargs_to_specs(&pairs).or_else(|e| cx.throw_error(e.to_string()))
}

fn js_to_field_value(cx: &mut FunctionContext, val: &Handle<JsValue>) -> NeonResult<FieldValue> {
    if let Ok(s) = val.downcast::<JsString, _>(cx) {
        Ok(FieldValue::String(s.value(cx)))
    } else if let Ok(n) = val.downcast::<JsNumber, _>(cx) {
        Ok(FieldValue::U64(n.value(cx) as u64))
    } else if let Ok(b) = val.downcast::<JsBoolean, _>(cx) {
        Ok(FieldValue::Bool(b.value(cx)))
    } else if val.is_a::<JsNull, _>(cx) || val.is_a::<JsUndefined, _>(cx) {
        Ok(FieldValue::None)
    } else {
        Ok(FieldValue::String(format!("{:?}", val)))
    }
}
```

### 16.5 Node.js Server with QueryList

```rust
// bindings/node/src/js_server.rs

pub fn js_server_sessions(mut cx: FunctionContext) -> JsResult<JsBox<JsQueryList>> {
    let server = cx.this::<JsBox<JsServer>>()?;
    let graph = server.inner.read().unwrap();
    // Build JsQueryList from sessions
    let query_list = build_js_query_list(&mut cx, graph.sessions())?;
    Ok(cx.boxed(query_list))
}

// Register methods:
// server.sessions()         -> JsQueryList
// server.sessions().filter  -> filtered JsQueryList
// server.sessions().get     -> single item
```

### AGENTS.md Rules (Section 16)

```
RULE S16-PYQUERY: server.sessions must return PyQueryList, not list. Filtering happens in Rust.
RULE S16-KWARGS: Python kwargs must use __ operator syntax. "name__startswith" not "name_startswith".
RULE S16-DEFAULT: get(default=X) must be supported. get() without default raises on 0 results.
RULE S16-JSQUERY: Node.js sessions() must return JsQueryList with .filter() and .get() methods.
RULE S16-CALLABLE: Both PyQueryList.filter() and JsQueryList.filter() must accept callables.
```

### Test Strategy (Section 16)

```python
# tests/python/test_query_list.py

def test_filter_by_name(server):
    """Test basic kwargs filter."""
    server.new_session("work")
    server.new_session("home")
    result = server.sessions.filter(name="work")
    assert len(result) == 1
    assert result[0].name == "work"

def test_filter_startswith(server):
    """Test __ operator syntax."""
    server.new_session("work-main")
    server.new_session("work-dev")
    server.new_session("personal")
    result = server.sessions.filter(name__startswith="work")
    assert len(result) == 2

def test_filter_nin(server):
    """v8: Test Nin operator via binding."""
    server.new_session("alpha")
    server.new_session("beta")
    server.new_session("gamma")
    result = server.sessions.filter(name__nin=["alpha", "beta"])
    assert len(result) == 1
    assert result[0].name == "gamma"

def test_filter_iregex(server):
    """v8: Test IRegex operator via binding."""
    server.new_session("WorkMain")
    server.new_session("homework")
    result = server.sessions.filter(name__iregex="^work")
    assert len(result) == 2

def test_get_with_default(server):
    """v8: Test get(default=...) parameter."""
    result = server.sessions.get(name="nonexistent", default=None)
    assert result is None

def test_get_raises_on_missing(server):
    """Test get() raises LookupError without default."""
    with pytest.raises(LookupError):
        server.sessions.get(name="nonexistent")

def test_get_raises_on_multiple(server):
    """Test get() raises ValueError on multiple matches."""
    server.new_session("dup")
    server.new_session("dup")
    with pytest.raises(ValueError):
        server.sessions.get(name="dup")

def test_filter_callable(server):
    """v8: Test callable filter."""
    server.new_session("work-main")
    server.new_session("personal")
    result = server.sessions.filter(lambda s: s.name.startswith("work"))
    assert len(result) == 1

def test_query_list_iteration(server):
    """Test PyQueryList supports iteration."""
    server.new_session("a")
    server.new_session("b")
    names = [s.name for s in server.sessions]
    assert "a" in names and "b" in names

def test_query_list_len(server):
    """Test PyQueryList supports len()."""
    server.new_session("test")
    assert len(server.sessions) >= 1

def test_query_list_indexing(server):
    """Test PyQueryList supports [] indexing."""
    server.new_session("first")
    assert server.sessions[0] is not None
```

```javascript
// tests/node/test_query_list.js

test('filter by name', () => {
    server.newSession('work');
    server.newSession('home');
    const result = server.sessions().filter({ name: 'work' });
    expect(result).toHaveLength(1);
    expect(result[0].name).toBe('work');
});

test('filter with __ operator', () => {
    const result = server.sessions().filter({ name__startswith: 'wo' });
    expect(result.length).toBeGreaterThanOrEqual(1);
});

test('get with default (v8)', () => {
    const result = server.sessions().get({ name: 'nonexistent', default: null });
    expect(result).toBeNull();
});

test('filter callable (v8)', () => {
    const result = server.sessions().filter(s => s.name.startsWith('work'));
    expect(result.length).toBeGreaterThanOrEqual(1);
});
```

---

## 17. CRDT Transaction Layer

### 17.1 Version Vectors

```rust
// crates/tf-model/src/crdt.rs

use std::collections::BTreeMap;

/// Unique replica identifier (for future multi-server support).
pub type ReplicaId = u64;

/// Lamport-style version vector for conflict detection.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct VersionVector {
    clocks: BTreeMap<ReplicaId, u64>,
}

impl VersionVector {
    pub fn new() -> Self {
        Self { clocks: BTreeMap::new() }
    }

    pub fn increment(&mut self, replica: ReplicaId) -> u64 {
        let counter = self.clocks.entry(replica).or_insert(0);
        *counter += 1;
        *counter
    }

    pub fn get(&self, replica: ReplicaId) -> u64 {
        self.clocks.get(&replica).copied().unwrap_or(0)
    }

    /// Returns true if self happened-before or is concurrent with other.
    pub fn dominates(&self, other: &Self) -> bool {
        other.clocks.iter().all(|(replica, &clock)| {
            self.get(*replica) >= clock
        })
    }

    /// Merge two version vectors (element-wise max).
    pub fn merge(&mut self, other: &Self) {
        for (&replica, &clock) in &other.clocks {
            let entry = self.clocks.entry(replica).or_insert(0);
            *entry = (*entry).max(clock);
        }
    }
}
```

### 17.2 Transaction Log

```rust
/// A transaction wraps one or more effects as an atomic unit.
#[derive(Debug, Clone)]
pub struct Transaction {
    pub id: u64,
    pub replica: ReplicaId,
    pub version: VersionVector,
    pub effects: Vec<Effect>,
    pub timestamp: SystemTime,
}

/// Append-only transaction log for durability and replication.
pub struct TransactionLog {
    entries: Vec<Transaction>,
    current_version: VersionVector,
    replica_id: ReplicaId,
}

impl TransactionLog {
    pub fn new(replica_id: ReplicaId) -> Self {
        Self {
            entries: Vec::new(),
            current_version: VersionVector::new(),
            replica_id,
        }
    }

    pub fn commit(&mut self, effects: Vec<Effect>) -> &Transaction {
        let version_num = self.current_version.increment(self.replica_id);
        let tx = Transaction {
            id: version_num,
            replica: self.replica_id,
            version: self.current_version.clone(),
            effects,
            timestamp: SystemTime::now(),
        };
        self.entries.push(tx);
        self.entries.last().unwrap()
    }

    /// Replay transactions since a given version vector.
    pub fn since(&self, since: &VersionVector) -> &[Transaction] {
        // Find first transaction not dominated by `since`
        let start = self.entries.iter()
            .position(|tx| !since.dominates(&tx.version))
            .unwrap_or(self.entries.len());
        &self.entries[start..]
    }
}
```

### AGENTS.md Rules (Section 17)

```
RULE S17-CRDT: All state mutations go through the transaction log. No direct graph mutation.
RULE S17-REPLAY: Transactions must be replayable (deterministic effects).
RULE S17-VERSION: Version vectors use BTreeMap for deterministic serialization.
```

### Test Strategy (Section 17)

```rust
#[test]
fn version_vector_dominance() {
    let mut vv1 = VersionVector::new();
    vv1.increment(1);
    vv1.increment(1);

    let mut vv2 = VersionVector::new();
    vv2.increment(1);

    assert!(vv1.dominates(&vv2));
    assert!(!vv2.dominates(&vv1));
}

#[test]
fn transaction_log_commit_and_since() {
    let mut log = TransactionLog::new(1);
    log.commit(vec![Effect::SessionCreated(/* ... */)]);
    log.commit(vec![Effect::SessionDestroyed(SessionId::new(0))]);

    let empty_vv = VersionVector::new();
    assert_eq!(log.since(&empty_vv).len(), 2);
}
```

---

## 18. Security Model

### 18.1 Socket Security

```rust
/// Socket permission model:
/// - Socket directory: 0700 (user-private)
/// - Socket file: 0600 (user-private)
/// - No world-readable/writable sockets
/// - Validate caller UID matches socket owner

pub fn validate_socket_permissions(path: &Path) -> Result<(), SecurityError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let meta = std::fs::metadata(path)?;
        let mode = meta.mode() & 0o777;
        if mode & 0o077 != 0 {
            return Err(SecurityError::InsecurePermissions {
                path: path.to_path_buf(),
                mode,
            });
        }
    }
    Ok(())
}
```

### 18.2 Input Validation

```rust
/// All client input must be validated before processing.
pub fn validate_session_name(name: &str) -> Result<(), SecurityError> {
    if name.is_empty() {
        return Err(SecurityError::InvalidInput("session name cannot be empty".into()));
    }
    if name.len() > 256 {
        return Err(SecurityError::InvalidInput("session name too long".into()));
    }
    if name.contains('\0') {
        return Err(SecurityError::InvalidInput("session name contains null byte".into()));
    }
    // Reject path traversal attempts
    if name.contains('/') || name.contains("..") {
        return Err(SecurityError::InvalidInput("session name contains path characters".into()));
    }
    Ok(())
}
```

### 18.3 Rate Limiting

```rust
/// Per-client rate limiting to prevent denial of service.
pub struct ClientRateLimiter {
    /// Commands per second limit.
    pub commands_per_sec: u32,
    /// Maximum pending commands.
    pub max_pending: usize,
    /// Maximum output buffer size (bytes).
    pub max_output_buffer: usize,
}

impl Default for ClientRateLimiter {
    fn default() -> Self {
        Self {
            commands_per_sec: 1000,
            max_pending: 256,
            max_output_buffer: 16 * 1024 * 1024, // 16MB
        }
    }
}
```

### AGENTS.md Rules (Section 18)

```
RULE S18-PERMS: Socket files must be 0600, socket directories 0700. Test in CI.
RULE S18-INPUT: All string input from clients must pass validation before use.
RULE S18-RATE: Every client connection has a rate limiter. No exceptions.
RULE S18-NULLBYTE: Reject any input containing null bytes.
```

### Test Strategy (Section 18)

```rust
#[test]
fn reject_path_traversal_in_session_name() {
    assert!(validate_session_name("../../etc/passwd").is_err());
    assert!(validate_session_name("normal-name").is_ok());
}

#[test]
fn reject_null_bytes() {
    assert!(validate_session_name("name\0evil").is_err());
}

#[test]
fn socket_permissions_enforced() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.sock");
    std::fs::write(&path, "").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o777)).unwrap();
        assert!(validate_socket_permissions(&path).is_err());
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert!(validate_socket_permissions(&path).is_ok());
    }
}
```

---

## 19. OpenTelemetry

This section defines the telemetry architecture. **v8 fixes**: dual-provider
architecture (gap #6), thread-local TRACE_HEADERS_STACK (gap #7), TOML config
priority chain (gap #16), composite propagator (gap #17), env var enable toggle (gap #18).

The design is directly modeled on vibe-tmux's `mux-otel` crate which has proven these
patterns in production.

### 19.1 Dual Provider Architecture (v8)

TermForge uses **two separate OTEL providers**, mirroring vibe-tmux's `otel.rs`:

```rust
// crates/tf-otel/src/otel.rs

use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use opentelemetry::Context;
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_sdk::logs::SdkLoggerProvider;

/// Global OTEL provider for the server/TUI process.
/// Initialized once via init_tracing().
static OTEL_PROVIDER: OnceLock<Mutex<Option<OtelProvider>>> = OnceLock::new();

/// Separate OTEL provider for client-side spans.
/// Initialized on-demand when enter_client_span() is first called.
/// This prevents client initialization from blocking server startup.
static CLIENT_PROVIDER: OnceLock<Mutex<Option<OtelProvider>>> = OnceLock::new();

/// An OTEL provider holding tracer, logger, and optional dedicated runtime.
pub struct OtelProvider {
    logger: Option<SdkLoggerProvider>,
    tracer_provider: Option<SdkTracerProvider>,
    tracer: Option<opentelemetry_sdk::trace::Tracer>,
    /// Dedicated Tokio runtime for OTEL export (when not in an existing runtime).
    runtime: Option<tokio::runtime::Runtime>,
}

impl OtelProvider {
    /// Build a provider for a given service.
    ///
    /// `install_global`: if true, sets the global tracer/propagator.
    /// Server uses `install_global=true`, client uses `install_global=false`.
    pub fn build(
        service_name: &str,
        version: &str,
        install_global: bool,
    ) -> anyhow::Result<Option<Self>> {
        if !otel_enabled() {
            return Ok(None);
        }

        let resource = opentelemetry_sdk::Resource::builder()
            .with_attribute(opentelemetry::KeyValue::new(
                "service.name", service_name.to_string()
            ))
            .with_attribute(opentelemetry::KeyValue::new(
                "service.version", version.to_string()
            ))
            .with_attribute(opentelemetry::KeyValue::new(
                "service.namespace", "termforge".to_string()
            ))
            .build();

        let enable_traces = env_flag("TERMFORGE_OTEL_TRACES").unwrap_or(true);
        let enable_logs = env_flag("TERMFORGE_OTEL_LOGS").unwrap_or(true);

        // Build dedicated runtime if needed (when no Tokio runtime exists)
        let in_tokio = tokio::runtime::Handle::try_current().is_ok();
        let runtime = if !in_tokio {
            Some(tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .thread_name("tf-otel")
                .build()?)
        } else {
            None
        };
        let _guard = runtime.as_ref().map(tokio::runtime::Runtime::enter);

        let tracer_provider = if enable_traces {
            let exporter = build_span_exporter()?;
            Some(SdkTracerProvider::builder()
                .with_resource(resource.clone())
                .with_batch_exporter(exporter)
                .build())
        } else {
            None
        };

        let tracer = tracer_provider.as_ref()
            .map(|p| p.tracer(service_name.to_string()));

        if install_global {
            if let Some(ref provider) = tracer_provider {
                opentelemetry::global::set_tracer_provider(provider.clone());
                opentelemetry::global::set_text_map_propagator(composite_propagator());
            }
            if tracer.is_some() {
                attach_traceparent_from_env();
            }
        }

        let logger = if enable_logs {
            let exporter = build_log_exporter()?;
            Some(SdkLoggerProvider::builder()
                .with_resource(resource)
                .with_batch_exporter(exporter)
                .build())
        } else {
            None
        };

        Ok(Some(Self { logger, tracer_provider, tracer, runtime }))
    }
}

impl Drop for OtelProvider {
    fn drop(&mut self) {
        if let Some(logger) = &self.logger {
            let _ = logger.shutdown();
        }
        if let Some(provider) = &self.tracer_provider {
            let _ = provider.shutdown();
        }
        if let Some(runtime) = self.runtime.take() {
            runtime.shutdown_timeout(Duration::from_millis(200));
        }
    }
}
```

### 19.2 Thread-Local Trace Headers Stack (v8)

```rust
// crates/tf-otel/src/lib.rs

use std::cell::RefCell;

/// Trace context propagation headers.
#[derive(Debug, Clone, Default)]
pub struct TraceHeaders {
    pub traceparent: String,
    pub tracestate: Option<String>,
    pub baggage: Option<String>,
}

/// Thread-local stack of trace headers for nested span support.
/// Each thread maintains its own stack to prevent cross-thread interference.
///
/// Cross-thread propagation uses capture-before-spawn + push-in-child pattern:
///   1. Parent captures: `let headers = current_trace_headers()`
///   2. Parent spawns child thread with `headers`
///   3. Child pushes: `let _guard = push_trace_headers(headers)`
///
/// This is modeled on vibe-tmux's TRACE_HEADERS_STACK.
thread_local! {
    pub(crate) static TRACE_HEADERS_STACK: RefCell<Vec<TraceHeaders>> =
        const { RefCell::new(Vec::new()) };
}

/// RAII guard that pops trace headers when dropped.
/// Restores the stack to its depth at guard creation time.
pub struct TraceHeadersGuard {
    depth: usize,
    restore: bool,
}

impl Drop for TraceHeadersGuard {
    fn drop(&mut self) {
        if !self.restore {
            return;
        }
        TRACE_HEADERS_STACK.with(|stack| {
            stack.borrow_mut().truncate(self.depth);
        });
    }
}

/// Push trace headers onto the thread-local stack.
/// Returns a guard that pops them on drop.
pub fn push_trace_headers(headers: TraceHeaders) -> TraceHeadersGuard {
    TRACE_HEADERS_STACK.with(|stack| {
        let mut stack = stack.borrow_mut();
        let depth = stack.len();
        stack.push(headers);
        TraceHeadersGuard { depth, restore: true }
    })
}

/// Get current trace headers (top of stack, or from process headers, or from env).
pub fn current_trace_headers() -> Option<TraceHeaders> {
    // Priority: thread-local stack > process headers > env
    let from_stack = TRACE_HEADERS_STACK.with(|stack| stack.borrow().last().cloned());
    if from_stack.is_some() {
        return from_stack;
    }
    current_process_trace_headers()
}

/// Attach TRACEPARENT from environment (used at server startup).
pub fn attach_traceparent_from_env() {
    if let Ok(traceparent) = std::env::var("TRACEPARENT") {
        let tracestate = std::env::var("TRACESTATE").ok();
        let baggage = std::env::var("BAGGAGE").ok();
        set_trace_context(&traceparent, tracestate.as_deref(), baggage.as_deref());
    }
}
```

### 19.3 Composite Propagator (v8)

```rust
// crates/tf-otel/src/otel.rs

use opentelemetry::propagation::TextMapCompositePropagator;
use opentelemetry_sdk::propagation::{BaggagePropagator, TraceContextPropagator};

/// Build a composite propagator that handles both W3C TraceContext and Baggage.
/// This ensures trace context AND baggage are propagated across process boundaries.
///
/// v8 addition: vibe-tmux chains BaggagePropagator + TraceContextPropagator.
fn composite_propagator() -> TextMapCompositePropagator {
    let propagators: Vec<Box<dyn opentelemetry::propagation::TextMapPropagator + Send + Sync>> = vec![
        Box::new(BaggagePropagator::new()),
        Box::new(TraceContextPropagator::new()),
    ];
    TextMapCompositePropagator::new(propagators)
}
```

### 19.4 Client Span API

```rust
/// Enter a client-side OTEL span.
/// Uses the CLIENT_PROVIDER (initialized on-demand) to avoid blocking server startup.
///
/// The returned guard:
/// 1. Ends the span on drop
/// 2. Pops trace headers from the thread-local stack
/// 3. Optionally force-flushes (in sync mode)
pub fn enter_client_span(name: &'static str) -> Option<ClientSpanGuard> {
    let slot = CLIENT_PROVIDER.get_or_init(|| Mutex::new(None));
    let tracer = {
        let mut guard = slot.lock().unwrap_or_else(|e| e.into_inner());
        if guard.is_none() {
            *guard = OtelProvider::build("tf-client", env!("CARGO_PKG_VERSION"), false)
                .ok()
                .flatten();
        }
        guard.as_ref().and_then(|p| p.tracer.clone())
    }?;

    let parent = current_otel_context().unwrap_or_else(Context::current);
    let builder = tracer.span_builder(name)
        .with_kind(opentelemetry::trace::SpanKind::Client);
    let span = tracer.build_with_context(builder, &parent);
    let context = Context::current_with_span(span);
    let trace_guard = trace_headers_from_context(&context).map(push_trace_headers);

    Some(ClientSpanGuard {
        context,
        _trace_guard: trace_guard,
    })
}

pub struct ClientSpanGuard {
    context: Context,
    _trace_guard: Option<TraceHeadersGuard>,
}

impl Drop for ClientSpanGuard {
    fn drop(&mut self) {
        self.context.span().end();
        // Force flush in sync mode for short-lived processes
        if env_flag("TERMFORGE_OTEL_SYNC") == Some(true) {
            if let Some(slot) = CLIENT_PROVIDER.get() {
                if let Ok(guard) = slot.lock() {
                    if let Some(ref provider) = *guard {
                        if let Some(ref tp) = provider.tracer_provider {
                            let _ = tp.force_flush();
                        }
                    }
                }
            }
        }
    }
}
```

### 19.5 TOML Config with Priority Chain (v8)

```rust
// crates/tf-otel/src/config.rs

use serde::Deserialize;
use std::sync::OnceLock;

static CONFIG: OnceLock<OtelConfig> = OnceLock::new();

/// OTEL configuration loaded from TOML files with priority chain.
///
/// Priority (highest wins):
/// 1. TERMFORGE_OTEL_CONFIG env var (path to TOML file)
/// 2. .termforge/otel.local.toml (local override, gitignored)
/// 3. .termforge/otel.toml (project config)
/// 4. ~/.config/termforge/otel.toml (user config)
/// 5. Built-in defaults
///
/// v8 addition: modeled on vibe-tmux's config.rs priority chain.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct OtelConfig {
    pub export_filter: ExportFilterConfig,
}

/// Controls which tracing spans are exported to the OTEL collector.
/// Uses prefix/exact matching with allow/deny lists.
///
/// Example TOML:
/// ```toml
/// [export_filter]
/// allow_prefixes = ["tf_", "termforge"]
/// allow_exact = ["otel"]
/// deny_prefixes = ["tf_internal_"]
/// deny_exact = []
/// ```
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ExportFilterConfig {
    pub allow_prefixes: Vec<String>,
    pub allow_exact: Vec<String>,
    pub deny_prefixes: Vec<String>,
    pub deny_exact: Vec<String>,
}

impl Default for ExportFilterConfig {
    fn default() -> Self {
        Self {
            allow_prefixes: vec!["tf_".to_string(), "termforge".to_string()],
            allow_exact: vec!["otel".to_string()],
            deny_prefixes: Vec::new(),
            deny_exact: Vec::new(),
        }
    }
}

impl ExportFilterConfig {
    /// Check if a target should be exported.
    /// Deny rules take precedence over allow rules.
    pub fn should_export(&self, target: &str) -> bool {
        // Deny takes precedence
        if self.deny_exact.iter().any(|t| t == target) {
            return false;
        }
        if self.deny_prefixes.iter().any(|p| target.starts_with(p)) {
            return false;
        }
        // Check allow
        if self.allow_exact.iter().any(|t| t == target) {
            return true;
        }
        if self.allow_prefixes.iter().any(|p| target.starts_with(p)) {
            return true;
        }
        false
    }
}

impl OtelConfig {
    pub fn load() -> Self {
        let mut config = Self::default();

        // Layer 1: User config
        if let Some(dirs) = directories::ProjectDirs::from("", "", "termforge") {
            let path = dirs.config_dir().join("otel.toml");
            if let Some(loaded) = Self::load_file(&path) {
                config.merge(loaded);
            }
        }

        // Layer 2: Project config
        if let Some(loaded) = Self::load_file(std::path::Path::new(".termforge/otel.toml")) {
            config.merge(loaded);
        }

        // Layer 3: Local override
        if let Some(loaded) = Self::load_file(std::path::Path::new(".termforge/otel.local.toml")) {
            config.merge(loaded);
        }

        // Layer 4: Env var override
        if let Ok(path) = std::env::var("TERMFORGE_OTEL_CONFIG") {
            if let Some(loaded) = Self::load_file(std::path::Path::new(&path)) {
                config.merge(loaded);
            }
        }

        config
    }

    fn load_file(path: &std::path::Path) -> Option<Self> {
        let contents = std::fs::read_to_string(path).ok()?;
        toml::from_str(&contents).ok()
    }

    fn merge(&mut self, other: Self) {
        // Allow lists: replace if non-empty
        if !other.export_filter.allow_prefixes.is_empty() {
            self.export_filter.allow_prefixes = other.export_filter.allow_prefixes;
        }
        if !other.export_filter.allow_exact.is_empty() {
            self.export_filter.allow_exact = other.export_filter.allow_exact;
        }
        // Deny lists: additive (accumulate denials)
        self.export_filter.deny_prefixes.extend(other.export_filter.deny_prefixes);
        self.export_filter.deny_exact.extend(other.export_filter.deny_exact);
    }
}

pub fn get_config() -> &'static OtelConfig {
    CONFIG.get_or_init(OtelConfig::load)
}
```

### 19.6 Enable/Disable Toggle (v8)

```rust
// crates/tf-otel/src/lib.rs

use std::sync::atomic::{AtomicBool, Ordering};

static OTEL_ENABLED_OVERRIDE: AtomicBool = AtomicBool::new(false);

/// Programmatically enable OTEL export.
/// Call before any providers are initialized.
pub fn set_otel_enabled(enabled: bool) {
    OTEL_ENABLED_OVERRIDE.store(enabled, Ordering::SeqCst);
}

/// Check if OTEL is enabled.
/// Priority: programmatic override > TERMFORGE_OTEL env var > OTEL endpoint env vars.
///
/// v8 addition: env var activation pattern from vibe-tmux.
pub fn otel_enabled() -> bool {
    if OTEL_ENABLED_OVERRIDE.load(Ordering::SeqCst) {
        return true;
    }
    if let Some(enabled) = env_flag("TERMFORGE_OTEL") {
        return enabled;
    }
    std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").is_ok()
        || std::env::var("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT").is_ok()
        || std::env::var("OTEL_EXPORTER_OTLP_LOGS_ENDPOINT").is_ok()
}

/// Parse a boolean-like env var: "1"/"true" -> Some(true), "0"/"false" -> Some(false).
pub fn env_flag(name: &str) -> Option<bool> {
    let raw = std::env::var(name).ok()?;
    let value = raw.trim();
    if value.is_empty() { return None; }
    if value.eq_ignore_ascii_case("1") || value.eq_ignore_ascii_case("true") {
        return Some(true);
    }
    if value.eq_ignore_ascii_case("0") || value.eq_ignore_ascii_case("false") {
        return Some(false);
    }
    None
}
```

### 19.7 Init and Shutdown

```rust
/// Initialize the full tracing + OTEL stack.
/// Safe to call multiple times (OnceLock guards).
pub fn init_tracing(config: TracingConfig<'_>) {
    TRACING_INIT.get_or_init(|| {
        let otel_on = otel_enabled();
        let provider = OtelProvider::build(config.service_name, config.service_version, true)
            .ok()
            .flatten();

        let _ = tracing_subscriber::registry()
            .with(build_filter(config.default_directive, otel_on))
            .with(if config.enable_fmt {
                Some(build_fmt_layer(config.span_events, config.fmt_path))
            } else {
                None
            })
            .with(provider.as_ref().and_then(|p| p.logger_layer()))
            .with(provider.as_ref().and_then(|p| p.tracing_layer()))
            .try_init();

        let _ = OTEL_PROVIDER.set(Mutex::new(provider));
    });
}

/// Flush and shut down both OTEL providers.
/// Call at process exit for clean telemetry delivery.
pub fn shutdown() -> bool {
    let ok = force_flush();

    for slot in [OTEL_PROVIDER.get(), CLIENT_PROVIDER.get()].into_iter().flatten() {
        let provider = {
            let mut guard = slot.lock().unwrap_or_else(|e| e.into_inner());
            guard.take()
        };
        drop(provider); // triggers OtelProvider::drop -> shutdown
    }

    ok
}

/// Force-flush both providers without shutting down.
pub fn force_flush() -> bool {
    let mut ok = true;
    for slot in [OTEL_PROVIDER.get(), CLIENT_PROVIDER.get()].into_iter().flatten() {
        let guard = slot.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(ref provider) = *guard {
            if let Some(ref tp) = provider.tracer_provider {
                ok &= tp.force_flush().is_ok();
            }
            if let Some(ref lp) = provider.logger {
                ok &= lp.force_flush().is_ok();
            }
        }
    }
    ok
}
```

### 19.8 Binding-Side Trace Propagation

```rust
// How Python sets trace context for Rust:
//
// Python:
//   import os
//   os.environ["TRACEPARENT"] = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01"
//   server = termforge.Server()  # Rust picks up TRACEPARENT from env
//
// Or programmatically:
//   termforge.set_traceparent("00-...")
//
// Rust binding:
#[pyfunction]
fn set_traceparent(traceparent: &str, tracestate: Option<&str>) -> bool {
    tf_otel::set_trace_context(traceparent, tracestate, None)
}
```

### AGENTS.md Rules (Section 19)

```
RULE S19-DUAL: Server uses OTEL_PROVIDER (install_global=true).
    Client uses CLIENT_PROVIDER (install_global=false). Never share.
RULE S19-THREADLOCAL: Trace context propagation uses TRACE_HEADERS_STACK.
    Never use std::env::set_var for trace context in library code.
RULE S19-TOML: OTEL config uses TOML with priority chain. .termforge/otel.local.toml
    must be in .gitignore.
RULE S19-COMPOSITE: Propagator must be composite (Baggage + TraceContext).
RULE S19-ENABLE: OTEL activation checks: programmatic > TERMFORGE_OTEL > OTEL endpoint vars.
RULE S19-FLUSH: Always call tf_otel::shutdown() before process exit.
RULE S19-FILTER: Export filter uses prefix/exact deny/allow from config, not glob patterns.
```

### Test Strategy (Section 19)

```rust
#[test]
fn trace_headers_are_thread_local() {
    // Thread-local isolation: child thread doesn't see parent's headers
    let traceparent = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";
    set_trace_context(traceparent, None, None);

    let handle = std::thread::spawn(current_trace_headers);
    let headers = handle.join().unwrap();
    assert!(headers.is_none(), "child thread should not see parent's headers");

    clear_trace_context();
}

#[test]
fn trace_headers_propagate_via_capture_push() {
    // Cross-thread propagation pattern
    let traceparent = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";
    set_trace_context(traceparent, None, None);
    let captured = current_trace_headers().unwrap();

    let handle = std::thread::spawn(move || {
        let _guard = push_trace_headers(captured);
        current_trace_headers()
    });

    let headers = handle.join().unwrap();
    assert!(headers.is_some());
    assert_eq!(headers.unwrap().traceparent, traceparent);
    clear_trace_context();
}

#[test]
fn nested_spans_restore_correctly() {
    let trace1 = "00-11111111111111111111111111111111-1111111111111111-01";
    let trace2 = "00-22222222222222222222222222222222-2222222222222222-01";

    let h1 = TraceHeaders { traceparent: trace1.into(), tracestate: None, baggage: None };
    let guard1 = push_trace_headers(h1);
    assert_eq!(current_trace_headers().unwrap().traceparent, trace1);

    {
        let h2 = TraceHeaders { traceparent: trace2.into(), tracestate: None, baggage: None };
        let _guard2 = push_trace_headers(h2);
        assert_eq!(current_trace_headers().unwrap().traceparent, trace2);
    } // guard2 dropped -> restores to trace1

    assert_eq!(current_trace_headers().unwrap().traceparent, trace1);
    drop(guard1);
    assert!(current_trace_headers().is_none());
}

#[test]
fn export_filter_allows_tf_modules() {
    let filter = ExportFilterConfig::default();
    assert!(filter.should_export("tf_server"));
    assert!(filter.should_export("tf_query"));
    assert!(filter.should_export("termforge::core"));
    assert!(filter.should_export("otel"));
    assert!(!filter.should_export("h2"));
    assert!(!filter.should_export("tonic"));
    assert!(!filter.should_export("hyper"));
}

#[test]
fn deny_takes_precedence_over_allow() {
    let filter = ExportFilterConfig {
        allow_prefixes: vec!["tf_".into()],
        allow_exact: vec!["otel".into()],
        deny_prefixes: vec!["tf_internal_".into()],
        deny_exact: vec!["otel".into()],
    };
    assert!(filter.should_export("tf_server"));
    assert!(!filter.should_export("tf_internal_debug"));
    assert!(!filter.should_export("otel")); // deny_exact wins
}

#[test]
fn otel_enabled_checks_env_vars() {
    // This test verifies the priority chain logic
    // (actual env manipulation happens in integration tests)
    assert!(!OTEL_ENABLED_OVERRIDE.load(Ordering::SeqCst));
    set_otel_enabled(true);
    assert!(otel_enabled());
    set_otel_enabled(false);
}

#[test]
fn config_priority_chain() {
    let config = OtelConfig::default();
    assert!(config.export_filter.should_export("tf_server"));

    // Verify merge behavior
    let mut base = OtelConfig::default();
    let override_config = OtelConfig {
        export_filter: ExportFilterConfig {
            allow_prefixes: vec!["custom_".into()],
            ..Default::default()
        },
    };
    base.merge(override_config);
    assert!(base.export_filter.should_export("custom_module"));
    assert!(!base.export_filter.should_export("tf_server")); // replaced, not appended
}
```

---

## 20. tmux Version Management

This section defines how TermForge builds and caches tmux binaries for testing.
**v8 fixes**: BLAKE3 cache key (gap #8), atomic build publication (gap #9),
comprehensive env var interface (gap #10), file-based locking (gap #3).

All patterns are modeled on vibe-tmux's `tmux-builder` tool.

### 20.1 Cache Key Computation with BLAKE3 (v8)

```rust
// crates/tf-version/src/cache.rs

/// Compute a deterministic cache key for a tmux build.
///
/// Format: tmux-<version>__<host>__<os>-<os_ver>__cfg<hash>__mk<hash>__tb<tool_ver>
///
/// The configure_flags and make_flags are hashed with BLAKE3 to produce
/// a stable fingerprint. This means different flag sets produce different
/// cache directories, preventing cross-contamination.
///
/// v8 addition: BLAKE3 hashing modeled on vibe-tmux's compute_cache_key().
pub fn compute_cache_key(tag: &str, options: &EnsureOptions) -> String {
    let version = normalize_tag(tag);
    let host = host_triple_guess();
    let os = detect_os_id().unwrap_or_else(|| "unknown".into());
    let os_ver = detect_os_version().unwrap_or_else(|| "unknown".into());

    let cfg_hash = flags_fingerprint(&options.configure_flags);
    let mk_hash = flags_fingerprint(&options.make_flags);
    let tool_ver = env!("CARGO_PKG_VERSION");

    format!(
        "tmux-{version}__{host}__{os}-{os_ver}__cfg{cfg}__mk{mk}__tb{tool}",
        version = sanitize_token(&version),
        host = sanitize_token(&host),
        os = sanitize_token(&os),
        os_ver = sanitize_token(&os_ver),
        cfg = &cfg_hash[..12],
        mk = &mk_hash[..12],
        tool = sanitize_token(tool_ver),
    )
}

/// Hash a list of flags with BLAKE3, returning the hex digest.
fn flags_fingerprint(flags: &[String]) -> String {
    let mut data = Vec::new();
    for flag in flags {
        data.extend_from_slice(flag.as_bytes());
        data.push(0); // null separator between flags
    }
    let hash = blake3::hash(&data);
    hex_32(*hash.as_bytes())
}

fn hex_32(bytes: [u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0xf) as usize] as char);
    }
    out
}

fn sanitize_token(value: &str) -> String {
    value.chars().map(|ch| match ch {
        'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '-' | '_' => ch,
        _ => '_',
    }).collect()
}
```

### 20.2 File-Based Locking (v8 Critical Fix)

```rust
// crates/tf-version/src/lock.rs

use std::fs::OpenOptions;
use std::path::Path;
use anyhow::{Context, Result};
use fs2::FileExt;

/// RAII lock guard that releases the file lock on drop.
///
/// v8 critical addition: file-based locking for parallel build safety.
/// Modeled on vibe-tmux's LockGuard + lock_cache_key() + lock_repo_clone().
///
/// Two lock levels:
/// 1. repo_clone lock: prevents parallel `git clone` of the shared repo cache
/// 2. cache_key lock: prevents concurrent builds of the same version/flags combo
#[derive(Debug)]
struct LockGuard {
    file: std::fs::File,
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

/// Acquire an exclusive file lock for a build directory.
/// Creates `<dir>.lock` as a sibling of the target directory.
///
/// This prevents parallel `cargo test` runs from corrupting builds.
fn lock_cache_key(dir: &Path) -> Result<LockGuard> {
    let lock_path = lock_path_for_dir(dir)?;
    if let Some(parent) = lock_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create {}", parent.display()))?;
    }
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&lock_path)
        .with_context(|| format!("open lock {}", lock_path.display()))?;
    file.lock_exclusive()
        .with_context(|| format!("lock {}", lock_path.display()))?;
    Ok(LockGuard { file })
}

/// Acquire an exclusive lock for the repo clone operation.
/// Prevents parallel tests from racing on `git clone`.
fn lock_repo_clone(clone_dest: &Path) -> Result<Option<LockGuard>> {
    let parent = clone_dest.parent().unwrap_or_else(|| Path::new("."));
    let base = clone_dest.file_name()
        .and_then(|n| n.to_str())
        .context("invalid clone dest name")?;
    let lock_path = parent.join(format!("{base}.lock"));
    if let Some(parent) = lock_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&lock_path)?;
    file.lock_exclusive()?;
    Ok(Some(LockGuard { file }))
}

fn lock_path_for_dir(dir: &Path) -> Result<std::path::PathBuf> {
    let parent = dir.parent()
        .with_context(|| format!("missing parent for {}", dir.display()))?;
    let base = dir.file_name()
        .and_then(|n| n.to_str())
        .context("invalid dir name")?;
    Ok(parent.join(format!("{base}.lock")))
}
```

### 20.3 Atomic Build Publication (v8)

```rust
// crates/tf-version/src/build.rs

/// Build tmux into a temporary directory and atomically rename to final location.
///
/// v8 addition: atomic publish pattern from vibe-tmux.
///
/// Process:
/// 1. Build into `.tmp-<name>-<pid>-<nanos>` (sibling of final dir)
/// 2. Validate the built binary (`tmux -V`)
/// 3. Write manifest.json with build metadata
/// 4. Atomic rename: `.tmp-...` -> final directory
///
/// If the rename fails (e.g., another process won the race), the
/// file lock prevents this scenario entirely.
pub fn ensure_tmux_binary(options: &EnsureOptions) -> Result<EnsureOutput> {
    // Lock 1: repo clone lock (prevents parallel git clone races)
    let _clone_lock = lock_repo_clone(&options.clone_dest)?;

    let tag = resolve_tag(&options.version)?;
    let cache_key = compute_cache_key(&tag, options);

    let final_dir = options.prefix_dir(&cache_key);
    let tmux_binary_final = final_dir.join("bin").join("tmux");
    let manifest_final = final_dir.join("manifest.json");

    // Lock 2: per-cache-key lock (prevents concurrent build of same version)
    let _build_lock = lock_cache_key(&final_dir)?;

    // Fast path: already built
    if tmux_binary_final.exists() && !options.rebuild {
        return Ok(EnsureOutput {
            tmux_binary: tmux_binary_final,
            manifest: manifest_final,
        });
    }

    // Build into temp directory
    let tmp_dir = sibling_tmp_dir(&final_dir)?;
    if tmp_dir.exists() {
        std::fs::remove_dir_all(&tmp_dir)?;
    }

    build_tmux_into(&options, &tmp_dir)?;
    validate_tmux_binary(&tmp_dir.join("bin").join("tmux"))?;
    write_manifest(&tmp_dir.join("manifest.json"), &cache_key, &tag)?;

    // Atomic publish: rename temp dir to final dir
    if final_dir.exists() {
        std::fs::remove_dir_all(&final_dir)?;
    }
    if let Some(parent) = final_dir.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::rename(&tmp_dir, &final_dir)
        .with_context(|| format!("atomic rename {} -> {}",
            tmp_dir.display(), final_dir.display()))?;

    Ok(EnsureOutput {
        tmux_binary: tmux_binary_final,
        manifest: manifest_final,
    })
}

/// Generate a unique temp directory name as sibling of the final directory.
fn sibling_tmp_dir(final_dir: &Path) -> Result<std::path::PathBuf> {
    let parent = final_dir.parent().context("missing parent dir")?;
    let base = final_dir.file_name()
        .and_then(|n| n.to_str())
        .context("invalid dir name")?;
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    Ok(parent.join(format!(".tmp-{base}-{pid}-{nanos}")))
}
```

### 20.4 Environment Variable Interface (v8)

```rust
/// Comprehensive environment variable interface for CI and automation.
///
/// v8 addition: full env var table modeled on vibe-tmux.
///
/// | Variable | Purpose | Default |
/// |---|---|---|
/// | TERMFORGE_TMUX_VERSION | Desired tmux version | latest |
/// | TERMFORGE_AUTO_BUILD | Auto-build from source if not cached | false |
/// | TERMFORGE_OFFLINE | Reject network operations | false |
/// | TERMFORGE_CACHE_DIR | Custom cache directory | ~/.cache/termforge |
/// | TERMFORGE_REPO | Explicit local repo path | (clone from GitHub) |
/// | TERMFORGE_BUILD_JOBS | Parallel build jobs | nproc |
/// | TERMFORGE_CONFIGURE_FLAGS | Extra configure flags | (empty) |
/// | TERMFORGE_MAKE_FLAGS | Extra make flags | (empty) |
/// | TMUX_BIN | Explicit tmux binary path | (search PATH) |
pub fn resolve_tmux_binary() -> Result<PathBuf> {
    // Priority 1: Explicit binary path
    if let Ok(bin) = std::env::var("TMUX_BIN") {
        let path = PathBuf::from(bin);
        if path.exists() {
            return Ok(path);
        }
        anyhow::bail!("TMUX_BIN points to nonexistent file: {}", path.display());
    }

    // Priority 2: Auto-build
    if env_flag("TERMFORGE_AUTO_BUILD") == Some(true) {
        let version = std::env::var("TERMFORGE_TMUX_VERSION")
            .unwrap_or_else(|_| "3.5a".into());
        let options = EnsureOptions::from_env(&version)?;
        let output = ensure_tmux_binary(&options)?;
        return Ok(output.tmux_binary);
    }

    // Priority 3: PATH lookup
    which::which("tmux").context("tmux not found in PATH")
}

pub struct EnsureOptions {
    pub version: String,
    pub clone_dest: PathBuf,
    pub rebuild: bool,
    pub jobs: Option<usize>,
    pub configure_flags: Vec<String>,
    pub make_flags: Vec<String>,
}

impl EnsureOptions {
    pub fn from_env(version: &str) -> Result<Self> {
        let cache_dir = std::env::var("TERMFORGE_CACHE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                dirs::cache_dir()
                    .unwrap_or_else(|| PathBuf::from("/tmp"))
                    .join("termforge")
            });

        let jobs = std::env::var("TERMFORGE_BUILD_JOBS")
            .ok()
            .and_then(|s| s.parse().ok());

        let configure_flags = std::env::var("TERMFORGE_CONFIGURE_FLAGS")
            .ok()
            .map(|s| s.split_whitespace().map(String::from).collect())
            .unwrap_or_default();

        let make_flags = std::env::var("TERMFORGE_MAKE_FLAGS")
            .ok()
            .map(|s| s.split_whitespace().map(String::from).collect())
            .unwrap_or_default();

        Ok(Self {
            version: version.into(),
            clone_dest: cache_dir.join("tmux.git"),
            rebuild: false,
            jobs,
            configure_flags,
            make_flags,
        })
    }

    pub fn prefix_dir(&self, cache_key: &str) -> PathBuf {
        let cache_dir = std::env::var("TERMFORGE_CACHE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                dirs::cache_dir()
                    .unwrap_or_else(|| PathBuf::from("/tmp"))
                    .join("termforge")
            });
        cache_dir.join(format!("prefix-{cache_key}"))
    }
}
```

### 20.5 Version Satisfies Logic

```rust
/// Check if a tmux version string satisfies a required version.
/// Uses prefix matching: "3.4a" satisfies "3.4".
pub fn version_satisfies(actual: &str, required: &str) -> bool {
    let actual = actual.trim().strip_prefix("tmux ").unwrap_or(actual);
    let required = required.trim();
    actual.starts_with(required)
}
```

### AGENTS.md Rules (Section 20)

```
RULE S20-BLAKE3: Cache keys must use BLAKE3 for flag fingerprinting. No SHA-256.
RULE S20-LOCK: All shared cache operations must acquire file locks before proceeding.
    Two lock levels: repo clone + cache key.
RULE S20-ATOMIC: Build artifacts must be atomically published via rename.
    Never write directly to the final directory.
RULE S20-ENV: All build configuration must be settable via env vars for CI.
RULE S20-OFFLINE: When TERMFORGE_OFFLINE=1, reject any network operation with clear error.
```

### Test Strategy (Section 20)

```rust
#[test]
fn cache_key_is_deterministic() {
    let opts = EnsureOptions {
        version: "3.4".into(),
        configure_flags: vec!["--enable-debug".into()],
        make_flags: vec![],
        ..default_test_opts()
    };
    let key1 = compute_cache_key("3.4", &opts);
    let key2 = compute_cache_key("3.4", &opts);
    assert_eq!(key1, key2, "same inputs must produce same cache key");
}

#[test]
fn different_flags_different_cache_key() {
    let opts1 = EnsureOptions {
        configure_flags: vec!["--enable-debug".into()],
        ..default_test_opts()
    };
    let opts2 = EnsureOptions {
        configure_flags: vec!["--disable-debug".into()],
        ..default_test_opts()
    };
    let key1 = compute_cache_key("3.4", &opts1);
    let key2 = compute_cache_key("3.4", &opts2);
    assert_ne!(key1, key2, "different flags must produce different cache keys");
}

#[test]
fn file_lock_prevents_concurrent_access() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("build");
    std::fs::create_dir_all(&target).unwrap();

    let lock1 = lock_cache_key(&target).unwrap();
    // Second lock on same path would block (test with try_lock)
    let lock_path = lock_path_for_dir(&target).unwrap();
    let file2 = OpenOptions::new()
        .create(true).read(true).write(true)
        .open(&lock_path).unwrap();
    assert!(file2.try_lock_exclusive().is_err(),
        "should fail because lock1 holds it");
    drop(lock1);
    assert!(file2.try_lock_exclusive().is_ok(),
        "should succeed after lock1 is dropped");
}

#[test]
fn version_satisfies_prefix_matching() {
    assert!(version_satisfies("tmux 3.4a", "3.4"));
    assert!(version_satisfies("3.4a", "3.4"));
    assert!(version_satisfies("3.4", "3.4"));
    assert!(!version_satisfies("3.3", "3.4"));
}

#[test]
fn sibling_tmp_dir_is_unique() {
    let dir = Path::new("/tmp/termforge/prefix-test");
    let tmp1 = sibling_tmp_dir(dir).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(1));
    let tmp2 = sibling_tmp_dir(dir).unwrap();
    assert_ne!(tmp1, tmp2, "temp dirs must be unique");
}
```

---

## 21. Test Support and Fake PTY

This section defines the test harness infrastructure. **v8 fixes**: three-layer socket
validation (gap #4), permission hardening (gap #11), `-f /dev/null` config isolation
(gap #12), socket readiness check (gap #13), dual MuxServerTestServer modes (gap #14),
clipboard isolation (gap #19).

All patterns modeled on vibe-tmux's `mux-test-support` crate.

### 21.1 Three-Layer Socket Validation (v8 Critical Fix)

```rust
// crates/tf-test/src/path_guard.rs

use std::path::{Path, PathBuf};
use anyhow::{Result, bail};

/// Three-layer socket validation ensures tests NEVER touch the user's real tmux.
///
/// v8 critical addition: modeled on vibe-tmux's path_guard.rs.
///
/// Layer 1: Refuse default socket name
/// Layer 2: Socket must be within test temp directory
/// Layer 3: Socket must not match $TMUX environment variable

/// Layer 1: Refuse to use tmux's default socket name.
pub fn ensure_not_default_socket_name(socket_name: &str) -> Result<()> {
    if socket_name.trim().is_empty() {
        bail!("socket name must be non-empty");
    }
    if socket_name == "default" {
        bail!("refusing to use tmux default socket name: default");
    }
    Ok(())
}

/// Layer 2: Ensure the socket is within the harness temp directory.
pub fn ensure_socket_within_tempdir(socket_path: &Path, tempdir: &Path) -> Result<()> {
    if !socket_path.starts_with(tempdir) {
        bail!(
            "refusing to operate on socket outside harness temp dir: socket={} tempdir={}",
            socket_path.display(),
            tempdir.display()
        );
    }
    Ok(())
}

/// Layer 3: Refuse to touch the socket referenced by $TMUX.
pub fn ensure_socket_not_tmux_env(socket_path: &Path) -> Result<()> {
    let tmux_env = std::env::var("TMUX").ok();
    ensure_socket_not_tmux_env_value(socket_path, tmux_env.as_deref())
}

/// Testable version that accepts $TMUX value as parameter.
pub fn ensure_socket_not_tmux_env_value(
    socket_path: &Path,
    tmux_env: Option<&str>,
) -> Result<()> {
    let Some(tmux) = tmux_env else { return Ok(()); };
    let tmux_socket = tmux.split(',').next().unwrap_or("");
    if tmux_socket.is_empty() { return Ok(()); }
    if socket_path.to_string_lossy() == tmux_socket {
        bail!(
            "refusing to operate on socket matching TMUX env socket: {}",
            socket_path.display()
        );
    }
    Ok(())
}

/// Run all three validation layers.
pub fn validate_test_socket(
    socket_name: &str,
    socket_path: &Path,
    tempdir: &Path,
) -> Result<()> {
    ensure_not_default_socket_name(socket_name)?;
    ensure_socket_within_tempdir(socket_path, tempdir)?;
    ensure_socket_not_tmux_env(socket_path)?;
    Ok(())
}

/// Resolve the tmux socket path from TMUX_TMPDIR and socket name.
/// Format: $TMUX_TMPDIR/tmux-<uid>/<socket_name>
pub fn tmux_socket_path(tmux_tmpdir: &Path, socket_name: &str, uid: u32) -> PathBuf {
    tmux_tmpdir.join(format!("tmux-{uid}")).join(socket_name)
}
```

### 21.2 Permission Hardening (v8)

```rust
// crates/tf-test/src/tmux_server.rs (helper)

/// Set directory permissions to 0700 (user-private).
/// tmux rejects socket directories with "unsafe permissions".
///
/// v8 addition: permission hardening from vibe-tmux.
#[cfg(unix)]
fn chmod_0700(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)
        .with_context(|| format!("stat {}", path.display()))?
        .permissions();
    perms.set_mode(0o700);
    std::fs::set_permissions(path, perms)
        .with_context(|| format!("chmod 0700 {}", path.display()))?;
    Ok(())
}
```

### 21.3 TmuxTestServer (Subprocess Mode)

```rust
// crates/tf-test/src/tmux_server.rs

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use anyhow::{Context, Result, bail};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(2);
const SHUTDOWN_TIMEOUT: Duration = Duration::from_millis(800);

/// Generate a unique socket name that can never be "default".
fn unique_socket_name(prefix: &str) -> String {
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{prefix}-{pid}-{nanos}")
}

/// Wait for a Unix socket to become connectable.
///
/// v8 addition: socket readiness check from vibe-tmux.
#[cfg(unix)]
fn wait_for_socket(path: &Path, timeout: Duration) -> bool {
    use std::os::unix::net::UnixStream;
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if path.exists() && UnixStream::connect(path).is_ok() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    path.exists() && UnixStream::connect(path).is_ok()
}

/// Real tmux test harness with full isolation.
///
/// Isolation guarantees:
/// - Unique socket name (never "default")
/// - Per-test TMUX_TMPDIR temp directory
/// - `-f /dev/null` config isolation (v8)
/// - `env_remove("TMUX")` prevents session nesting
/// - Permission hardening: 0700 on socket dir (v8)
/// - Socket readiness check before returning (v8)
/// - Clipboard isolation: set-clipboard off (v8)
/// - Three-layer socket validation on shutdown (v8)
#[derive(Debug)]
pub struct TmuxTestServer {
    tmux_bin: PathBuf,
    tmux_tmpdir: tempfile::TempDir,
    socket_name: String,
    socket_path: PathBuf,
    session_name: String,
}

impl TmuxTestServer {
    pub fn start() -> Result<Self> {
        Self::start_with_session("termforge-test")
    }

    pub fn start_with_session(session_name: &str) -> Result<Self> {
        let socket_name = unique_socket_name("termforge-test");
        // Layer 1: validate socket name
        crate::path_guard::ensure_not_default_socket_name(&socket_name)?;

        let tmux_bin = resolve_tmux_binary()?;
        let tmux_tmpdir = tempfile::tempdir().context("create tmux temp dir")?;
        let uid = nix::unistd::Uid::effective().as_raw();
        let socket_path = crate::path_guard::tmux_socket_path(
            tmux_tmpdir.path(), &socket_name, uid
        );

        // Create socket directory with 0700 permissions (v8)
        if let Some(parent) = socket_path.parent() {
            std::fs::create_dir_all(parent)?;
            #[cfg(unix)]
            {
                chmod_0700(tmux_tmpdir.path())?;
                chmod_0700(parent)?;
            }
        }

        // Start tmux with -f /dev/null for config isolation (v8)
        let output = std::process::Command::new(&tmux_bin)
            .arg("-L").arg(&socket_name)
            .arg("-f").arg("/dev/null")       // v8: config isolation
            .args(["new-session", "-d", "-s", session_name])
            .env("TMUX_TMPDIR", tmux_tmpdir.path())
            .env_remove("TMUX")               // prevent session nesting
            .output()
            .with_context(|| format!("spawn tmux at {}", tmux_bin.display()))?;

        if !output.status.success() {
            bail!("tmux start failed: {}",
                String::from_utf8_lossy(&output.stderr).trim());
        }

        // v8: Wait for socket to become ready (connectable)
        if !wait_for_socket(&socket_path, DEFAULT_TIMEOUT) {
            bail!("tmux socket did not become ready at {}",
                socket_path.display());
        }

        // v8: Clipboard isolation -- disable set-clipboard to avoid
        // mutating developer's clipboard via OSC 52
        let clip_output = std::process::Command::new(&tmux_bin)
            .arg("-L").arg(&socket_name)
            .arg("-f").arg("/dev/null")
            .args(["set-option", "-g", "set-clipboard", "off"])
            .env("TMUX_TMPDIR", tmux_tmpdir.path())
            .env_remove("TMUX")
            .output()?;

        if !clip_output.status.success() {
            // Allow opt-in via TERMFORGE_ALLOW_SYSTEM_CLIPBOARD=1
            if std::env::var("TERMFORGE_ALLOW_SYSTEM_CLIPBOARD")
                .as_deref() != Ok("1")
            {
                bail!("failed to disable tmux set-clipboard");
            }
        }

        Ok(Self {
            tmux_bin,
            tmux_tmpdir,
            socket_name,
            socket_path,
            session_name: session_name.into(),
        })
    }

    pub fn socket_name(&self) -> &str { &self.socket_name }
    pub fn socket_path(&self) -> &Path { &self.socket_path }
    pub fn tmux_bin(&self) -> &Path { &self.tmux_bin }
    pub fn tmux_tmpdir(&self) -> &Path { self.tmux_tmpdir.path() }
    pub fn session_name(&self) -> &str { &self.session_name }

    /// Run a tmux command against this isolated server.
    /// Always uses -f /dev/null for config isolation (v8).
    pub fn tmux(&self, args: &[&str]) -> Result<std::process::Output> {
        std::process::Command::new(&self.tmux_bin)
            .arg("-L").arg(&self.socket_name)
            .arg("-f").arg("/dev/null")       // v8: always config-isolated
            .args(args)
            .env("TMUX_TMPDIR", self.tmux_tmpdir.path())
            .env_remove("TMUX")
            .output()
            .with_context(|| format!("run tmux at {}", self.tmux_bin.display()))
    }

    /// Check if the test server is still running.
    pub fn is_alive(&self) -> bool {
        #[cfg(unix)]
        {
            use std::os::unix::net::UnixStream;
            if !self.socket_path.exists() { return false; }
            if UnixStream::connect(&self.socket_path).is_err() { return false; }
        }
        matches!(
            self.tmux(&["display-message", "-p", "#{pid}"]),
            Ok(out) if out.status.success()
        )
    }

    /// Shutdown with three-layer validation (v8).
    pub fn shutdown(&mut self) -> Result<()> {
        let tmux_env = std::env::var("TMUX").ok();
        self.shutdown_with_tmux_env(tmux_env.as_deref())
    }

    pub fn shutdown_with_tmux_env(&mut self, tmux_env: Option<&str>) -> Result<()> {
        // v8: Three-layer validation before kill-server
        crate::path_guard::ensure_not_default_socket_name(&self.socket_name)?;
        crate::path_guard::ensure_socket_within_tempdir(
            &self.socket_path, self.tmux_tmpdir.path())?;
        crate::path_guard::ensure_socket_not_tmux_env_value(
            &self.socket_path, tmux_env)?;

        let output = std::process::Command::new(&self.tmux_bin)
            .arg("-L").arg(&self.socket_name)
            .arg("-f").arg("/dev/null")
            .arg("kill-server")
            .env("TMUX_TMPDIR", self.tmux_tmpdir.path())
            .env_remove("TMUX")
            .output()?;

        if !output.status.success() {
            bail!("tmux kill-server failed: {}",
                String::from_utf8_lossy(&output.stderr).trim());
        }

        // Wait for socket to stop accepting connections
        #[cfg(unix)]
        {
            use std::os::unix::net::UnixStream;
            let deadline = Instant::now() + SHUTDOWN_TIMEOUT;
            while Instant::now() < deadline {
                if UnixStream::connect(&self.socket_path).is_err() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        }

        let _ = std::fs::remove_file(&self.socket_path);
        Ok(())
    }
}

impl Drop for TmuxTestServer {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}
```

### 21.4 MuxServerTestServer -- Dual Mode (v8)

```rust
// crates/tf-test/src/mux_server.rs

/// Test server for TermForge's own server (not tmux).
///
/// v8 addition: supports two modes:
/// 1. In-process: ServerGraph + FakePtyBackend (fast unit tests)
/// 2. Subprocess: spawns tf-server binary (integration/e2e tests)
///
/// Both modes enforce the same three-layer socket validation.
pub enum MuxServerTestServer {
    /// In-process mode: runs server logic directly, no subprocess.
    InProcess {
        graph: Arc<RwLock<ServerGraph>>,
        event_tx: mpsc::Sender<Command>,
        shutdown: CancellationToken,
        _task: tokio::task::JoinHandle<()>,
    },
    /// Subprocess mode: spawns the tf-server binary.
    Subprocess {
        child: std::process::Child,
        socket_path: PathBuf,
        tmpdir: tempfile::TempDir,
    },
}

impl MuxServerTestServer {
    /// Create an in-process test server (fast, no real PTY).
    pub async fn in_process() -> Result<Self> {
        let graph = Arc::new(RwLock::new(ServerGraph::new()));
        let (event_tx, event_rx) = mpsc::channel(4096);
        let shutdown = CancellationToken::new();

        let g = graph.clone();
        let s = shutdown.clone();
        let task = tokio::spawn(async move {
            event_loop(g, event_rx, s).await;
        });

        Ok(MuxServerTestServer::InProcess {
            graph,
            event_tx,
            shutdown,
            _task: task,
        })
    }

    /// Create a subprocess test server (real binary, real socket).
    pub fn subprocess() -> Result<Self> {
        let tmpdir = tempfile::tempdir()?;
        let socket_name = unique_socket_name("tf-test");
        crate::path_guard::ensure_not_default_socket_name(&socket_name)?;

        let uid = nix::unistd::Uid::effective().as_raw();
        let socket_path = crate::path_guard::tmux_socket_path(
            tmpdir.path(), &socket_name, uid
        );

        if let Some(parent) = socket_path.parent() {
            std::fs::create_dir_all(parent)?;
            #[cfg(unix)]
            chmod_0700(parent)?;
        }

        let tf_server_bin = std::env::var("TERMFORGE_SERVER_BIN")
            .unwrap_or_else(|_| "tf-server".into());

        let child = std::process::Command::new(&tf_server_bin)
            .arg("--socket").arg(&socket_path)
            .env_remove("TMUX")
            .spawn()
            .with_context(|| format!("spawn {}", tf_server_bin))?;

        // Wait for socket readiness
        #[cfg(unix)]
        if !wait_for_socket(&socket_path, DEFAULT_TIMEOUT) {
            bail!("tf-server socket did not become ready");
        }

        Ok(MuxServerTestServer::Subprocess {
            child,
            socket_path,
            tmpdir,
        })
    }

    pub fn socket_path(&self) -> Option<&Path> {
        match self {
            Self::InProcess { .. } => None,
            Self::Subprocess { socket_path, .. } => Some(socket_path),
        }
    }
}

impl Drop for MuxServerTestServer {
    fn drop(&mut self) {
        match self {
            Self::InProcess { shutdown, .. } => {
                shutdown.cancel();
            }
            Self::Subprocess { child, socket_path, tmpdir } => {
                // SIGTERM first, then SIGKILL after timeout
                #[cfg(unix)]
                {
                    use nix::sys::signal::{Signal, kill};
                    use nix::unistd::Pid;
                    let pid = Pid::from_raw(child.id() as i32);
                    let _ = kill(pid, Signal::SIGTERM);
                    std::thread::sleep(Duration::from_millis(200));
                    let _ = kill(pid, Signal::SIGKILL);
                }
                let _ = child.wait();
                let _ = std::fs::remove_file(socket_path);
            }
        }
    }
}
```

### 21.5 Fake PTY Backend

```rust
// crates/tf-test/src/fake_pty.rs

use bytes::Bytes;
use std::collections::VecDeque;

/// Fake PTY backend for testing without real terminal I/O.
/// Allows injecting output and capturing input.
pub struct FakePtyBackend {
    /// Output that the fake PTY will produce when read.
    pending_output: VecDeque<Bytes>,
    /// Input captured by the fake PTY (what the test "typed").
    captured_input: Vec<Bytes>,
    /// Whether the PTY is "alive" (child process running).
    alive: bool,
    /// Exit code when the fake process terminates.
    exit_code: Option<i32>,
    pub width: u16,
    pub height: u16,
}

impl FakePtyBackend {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            pending_output: VecDeque::new(),
            captured_input: Vec::new(),
            alive: true,
            exit_code: None,
            width,
            height,
        }
    }

    /// Inject output that the "process" will produce.
    pub fn inject_output(&mut self, data: impl Into<Bytes>) {
        self.pending_output.push_back(data.into());
    }

    /// Read output from the fake PTY.
    pub fn read_output(&mut self) -> Option<Bytes> {
        self.pending_output.pop_front()
    }

    /// Write input to the fake PTY (simulates typing).
    pub fn write_input(&mut self, data: impl Into<Bytes>) {
        self.captured_input.push(data.into());
    }

    /// Get all input that was written to this PTY.
    pub fn get_captured_input(&self) -> &[Bytes] {
        &self.captured_input
    }

    /// Simulate process exit.
    pub fn exit(&mut self, code: i32) {
        self.alive = false;
        self.exit_code = Some(code);
    }

    pub fn is_alive(&self) -> bool {
        self.alive
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
    }
}
```

### AGENTS.md Rules (Section 21)

```
RULE S21-THREELAYER: Every test socket operation must pass all three validation layers.
    No exceptions. No shortcuts.
RULE S21-DEVNULL: Every tmux invocation in tests must include -f /dev/null.
    This prevents user's ~/.tmux.conf from interfering with tests.
RULE S21-PERMS: Socket directories in tests must be chmod 0700 before use.
RULE S21-READINESS: After starting a test server, always wait for socket readiness
    before running commands. Use wait_for_socket() with timeout.
RULE S21-CLIPBOARD: Tests must disable set-clipboard unless TERMFORGE_ALLOW_SYSTEM_CLIPBOARD=1.
RULE S21-DROP: TmuxTestServer::drop must call shutdown(). Never leak test servers.
RULE S21-DUALMODE: Integration tests use MuxServerTestServer::subprocess().
    Unit tests use MuxServerTestServer::in_process().
```

### Test Strategy (Section 21)

```rust
#[test]
fn default_socket_name_is_rejected() {
    assert!(ensure_not_default_socket_name("default").is_err());
    assert!(ensure_not_default_socket_name("").is_err());
    assert!(ensure_not_default_socket_name("test").is_ok());
}

#[test]
fn socket_path_must_be_within_tempdir() {
    let tempdir = PathBuf::from("/tmp/termforge-test");
    let ok = tempdir.join("tmux-1000").join("name");
    let bad = PathBuf::from("/tmp/tmux-1000/default");
    assert!(ensure_socket_within_tempdir(&ok, &tempdir).is_ok());
    assert!(ensure_socket_within_tempdir(&bad, &tempdir).is_err());
}

#[test]
fn tmux_env_guard_prevents_touching_active_session() {
    let socket = PathBuf::from("/tmp/tmux-1000/default");
    let tmux_env = "/tmp/tmux-1000/default,12345,0";
    assert!(ensure_socket_not_tmux_env_value(&socket, Some(tmux_env)).is_err());
}

#[test]
fn tmux_env_guard_allows_different_socket() {
    let socket = PathBuf::from("/tmp/termforge-test/tmux-1000/test-socket");
    let tmux_env = "/tmp/tmux-1000/default,12345,0";
    assert!(ensure_socket_not_tmux_env_value(&socket, Some(tmux_env)).is_ok());
}

#[test]
fn tmux_socket_path_format() {
    let base = PathBuf::from("/tmp/termforge-test");
    let path = tmux_socket_path(&base, "mysocket", 1000);
    assert_eq!(path, base.join("tmux-1000").join("mysocket"));
}

#[test]
fn unique_socket_names_are_never_default() {
    for _ in 0..100 {
        let name = unique_socket_name("tf-test");
        assert_ne!(name, "default");
        assert!(name.starts_with("tf-test-"));
    }
}

#[test]
fn fake_pty_inject_and_read() {
    let mut pty = FakePtyBackend::new(80, 24);
    pty.inject_output(b"hello world".as_slice());
    let output = pty.read_output().unwrap();
    assert_eq!(&output[..], b"hello world");
}

#[test]
fn fake_pty_captures_input() {
    let mut pty = FakePtyBackend::new(80, 24);
    pty.write_input(b"ls\n".as_slice());
    assert_eq!(pty.get_captured_input().len(), 1);
}

#[test]
fn tmux_test_server_integration() -> Result<()> {
    // Skip if tmux not available
    if which::which("tmux").is_err() {
        return Ok(());
    }
    let server = TmuxTestServer::start()?;
    assert!(server.is_alive());
    let output = server.tmux(&["list-sessions"])?;
    assert!(output.status.success());
    Ok(())
}
```

---

## 22. Binding Test Frameworks

### 22.1 Python Test Infrastructure

```python
# tests/python/conftest.py

import pytest
import termforge

@pytest.fixture
def server():
    """Start an isolated TermForge server for testing."""
    srv = termforge.Server(socket_path=None)  # auto-create isolated socket
    yield srv
    srv.kill_server()

@pytest.fixture
def tmux_server():
    """Start an isolated real tmux server for compatibility testing."""
    srv = termforge.TmuxTestServer()
    yield srv
    srv.shutdown()
```

### 22.2 Python Binding Tests

```python
# tests/python/test_server.py

def test_create_session(server):
    session = server.new_session("work")
    assert session.name == "work"

def test_list_sessions(server):
    server.new_session("alpha")
    server.new_session("beta")
    sessions = server.sessions
    assert len(sessions) >= 2

# v8: Tests using PyQueryList
def test_filter_sessions(server):
    server.new_session("alpha")
    server.new_session("beta")
    result = server.sessions.filter(name="beta")
    assert len(result) == 1
    assert result[0].name == "beta"

def test_filter_startswith(server):
    server.new_session("work-main")
    server.new_session("work-dev")
    server.new_session("personal")
    result = server.sessions.filter(name__startswith="work")
    assert len(result) == 2

def test_get_session(server):
    server.new_session("unique")
    session = server.sessions.get(name="unique")
    assert session.name == "unique"

def test_get_with_default(server):
    """v8: Test get(default=...) parameter via binding."""
    result = server.sessions.get(name="nonexistent", default=None)
    assert result is None

def test_window_traversal(server):
    session = server.new_session("test")
    window = session.new_window("editor")
    assert len(session.windows) >= 1
    assert any(w.name == "editor" for w in session.windows)

def test_pane_traversal(server):
    session = server.new_session("test")
    window = session.windows[0]
    pane = window.split_window()
    assert len(window.panes) >= 2

def test_nested_filter(server):
    """Filter windows across all sessions."""
    server.new_session("work")
    server.new_session("home")
    windows = server.windows.filter(name__contains="default")
    # Default windows from both sessions
    assert len(windows) >= 2
```

### 22.3 Python OTEL Integration Test

```python
# tests/python/test_otel.py

def test_trace_propagation(server):
    """v8: Verify trace context flows from Python to Rust."""
    import os
    traceparent = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01"
    termforge.set_traceparent(traceparent)
    session = server.new_session("traced")
    # Verify the span was created under the expected trace
    assert session is not None
```

### 22.4 Node.js Test Infrastructure

```javascript
// tests/node/setup.js

const termforge = require('termforge');

let server;

beforeAll(() => {
    server = new termforge.Server();
});

afterAll(() => {
    server.killServer();
});

module.exports = { getServer: () => server };
```

### 22.5 Node.js Binding Tests

```javascript
// tests/node/test_server.test.js

const { getServer } = require('./setup');

test('create session', () => {
    const server = getServer();
    const session = server.newSession('work');
    expect(session.name).toBe('work');
});

// v8: Tests using JsQueryList
test('filter sessions by name', () => {
    const server = getServer();
    server.newSession('alpha');
    server.newSession('beta');
    const result = server.sessions().filter({ name: 'beta' });
    expect(result).toHaveLength(1);
    expect(result[0].name).toBe('beta');
});

test('filter with __ operator', () => {
    const server = getServer();
    server.newSession('work-main');
    server.newSession('work-dev');
    const result = server.sessions().filter({ name__startswith: 'work' });
    expect(result.length).toBeGreaterThanOrEqual(2);
});

test('get session', () => {
    const server = getServer();
    server.newSession('unique');
    const session = server.sessions().get({ name: 'unique' });
    expect(session.name).toBe('unique');
});

test('get with default (v8)', () => {
    const server = getServer();
    const result = server.sessions().get({ name: 'nonexistent', default: null });
    expect(result).toBeNull();
});

test('window traversal', () => {
    const server = getServer();
    const session = server.newSession('traverse');
    expect(session.windows().length).toBeGreaterThanOrEqual(1);
});
```

### 22.6 tmux Compatibility Tests

```python
# tests/python/test_tmux_compat.py

"""
These tests verify TermForge produces identical output to tmux
for the same control-mode commands.
"""

def test_list_sessions_format(tmux_server, tf_server):
    """Control-mode list-sessions output must match."""
    tmux_out = tmux_server.cmd("list-sessions", "-F", "#{session_name}")
    tf_out = tf_server.cmd("list-sessions", "-F", "#{session_name}")
    assert tmux_out == tf_out

def test_layout_string_format(tmux_server, tf_server):
    """Layout strings must match tmux format."""
    # Create identical layouts in both
    for srv in [tmux_server, tf_server]:
        srv.cmd("split-window", "-h")
        srv.cmd("split-window", "-v")
    tmux_layout = tmux_server.cmd("display-message", "-p", "#{window_layout}")
    tf_layout = tf_server.cmd("display-message", "-p", "#{window_layout}")
    assert tmux_layout == tf_layout
```

### AGENTS.md Rules (Section 22)

```
RULE S22-FIXTURE: All test servers must use fixtures that auto-cleanup on test end.
RULE S22-COMPAT: tmux compatibility tests must run against both tmux and TermForge.
RULE S22-QUERYLIST: Binding tests must exercise PyQueryList/JsQueryList, not raw lists.
RULE S22-OTEL: At least one test per binding must verify trace propagation.
```

### Test Strategy (Section 22)

- Python tests run via `pytest` with `maturin develop` build step
- Node.js tests run via `jest` with `cargo-cp-artifact` build step
- Compatibility tests use parametric fixtures for tmux vs TermForge

---

## 23. Test Framework and Harness Design

### 23.1 Test Categories

| Category | Runner | Socket | PTY | Speed |
|----------|--------|--------|-----|-------|
| Unit | `cargo test` | None | FakePty | <1ms/test |
| Integration | `cargo test --features integration` | Real | Real | <5s/test |
| Protocol | `cargo test --features protocol` | Real | FakePty | <1s/test |
| Binding | `pytest` / `jest` | Real | Real | <5s/test |
| E2E | Custom harness | Real | Real | <30s/test |
| Fuzz | `cargo fuzz` | None | None | Continuous |

### 23.2 Test Isolation Guarantees

Every test that interacts with a server socket must:

1. **Use TmuxTestServer or MuxServerTestServer** -- never raw socket operations
2. **Pass three-layer validation** -- automatic via harness constructors
3. **Use tempfile::tempdir()** -- automatic cleanup on test end
4. **Use `-f /dev/null`** -- no user config interference
5. **Use `env_remove("TMUX")`** -- no session nesting
6. **Set `set-clipboard off`** -- no clipboard pollution

### 23.3 Parallel Test Safety

```rust
// Tests run in parallel by default (cargo test). Safety guaranteed by:
//
// 1. Each test gets its own tempdir (TmuxTestServer::start)
// 2. Socket names include PID + nanosecond timestamp (unique_socket_name)
// 3. Build cache uses file locks (lock_cache_key)
// 4. Repo clone uses file locks (lock_repo_clone)
// 5. Atomic publish prevents partial builds from being visible

#[test]
fn parallel_safety_stress() {
    // Run 10 test servers concurrently
    let handles: Vec<_> = (0..10).map(|i| {
        std::thread::spawn(move || {
            let server = TmuxTestServer::start_with_session(
                &format!("parallel-{i}")
            ).unwrap();
            assert!(server.is_alive());
            let output = server.tmux(&["list-sessions"]).unwrap();
            assert!(output.status.success());
        })
    }).collect();

    for handle in handles {
        handle.join().unwrap();
    }
}
```

### 23.4 CI Pipeline

```yaml
# .github/workflows/ci.yml
jobs:
  test:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest]
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Unit tests
        run: cargo test --workspace
      - name: Integration tests
        run: cargo test --workspace --features integration
        env:
          TERMFORGE_AUTO_BUILD: "1"
          TERMFORGE_TMUX_VERSION: "3.5a"
      - name: Python binding tests
        run: |
          cd bindings/python
          maturin develop
          pytest tests/
      - name: Node binding tests
        run: |
          cd bindings/node
          npm install
          npm test

  lint:
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt
      - run: cargo fmt --all --check
      - run: cargo clippy --workspace --all-features -- -D warnings
      - run: cargo deny check

  fuzz:
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@nightly
      - run: cargo install cargo-fuzz
      - run: cargo fuzz run protocol_parser -- -max_total_time=300
```

### AGENTS.md Rules (Section 23)

```
RULE S23-CATEGORY: Every test file must be in the correct category directory.
RULE S23-PARALLEL: Tests must be parallel-safe. No global state, no shared sockets.
RULE S23-CI: All test categories must run in CI. No "manual only" tests.
RULE S23-TIMEOUT: Individual tests must complete within 30 seconds. Use #[timeout(30000)].
```

### Test Strategy (Section 23)

- Meta-tests verify the test framework itself works
- CI runs all categories in matrix across OS + Rust version

---

## 24. Performance Targets

### 24.1 Benchmarks

```rust
// benches/throughput.rs

use criterion::{criterion_group, criterion_main, Criterion};

fn bench_query_filter(c: &mut Criterion) {
    let sessions: Vec<Session> = (0..10000)
        .map(|i| Session { name: format!("session-{i}"), ..default() })
        .collect();
    let query_list = QueryList::new(sessions);
    let spec = q("name").starts_with("session-999");

    c.bench_function("filter_10k_sessions", |b| {
        b.iter(|| query_list.filter(&[spec.clone()]))
    });
}

fn bench_effect_processing(c: &mut Criterion) {
    let mut graph = ServerGraph::new();
    // Pre-populate with 1000 sessions
    for i in 0..1000 {
        let sid = graph.id_allocator.next_session();
        graph.sessions.insert(sid, Session {
            id: sid,
            name: format!("session-{i}"),
            ..default()
        });
    }

    c.bench_function("process_new_session", |b| {
        b.iter(|| {
            EffectProcessor::process(&graph, Command::NewSession {
                name: "bench".into()
            })
        })
    });
}

fn bench_layout_relayout(c: &mut Criterion) {
    let pane_ids: Vec<PaneId> = (0..100).map(PaneId::new).collect();
    let mut tree = LayoutPreset::Tiled.apply(&pane_ids, 200, 60);

    c.bench_function("relayout_100_panes", |b| {
        b.iter(|| tree.relayout(0, 0, 200, 60))
    });
}

criterion_group!(benches, bench_query_filter, bench_effect_processing, bench_layout_relayout);
criterion_main!(benches);
```

### 24.2 Performance Budgets

| Operation | Target | Measurement |
|-----------|--------|-------------|
| QueryList filter (10K items) | <1ms | `criterion` bench |
| Effect processing (single) | <10us | `criterion` bench |
| Layout relayout (100 panes) | <100us | `criterion` bench |
| Protocol parse (1KB message) | <5us | `criterion` bench |
| Server startup to socket ready | <50ms | Integration test |
| Control mode round-trip | <5ms | Integration test |
| Memory per idle pane | <8KB | `jemalloc-ctl` measurement |
| 10K pane frame render | <16ms | TUI benchmark |

### 24.3 Memory Profiling

```rust
#[test]
fn memory_per_pane() {
    let before = allocated_bytes();
    let mut graph = ServerGraph::new();
    let sid = graph.id_allocator.next_session();
    let session = Session { id: sid, name: "bench".into(), ..default() };
    graph.sessions.insert(sid, session);

    // Add 1000 panes
    for _ in 0..1000 {
        let wid = graph.id_allocator.next_window();
        let pid = graph.id_allocator.next_pane();
        // ... add window with pane
    }

    let after = allocated_bytes();
    let per_pane = (after - before) / 1000;
    assert!(per_pane < 8192, "memory per pane: {} bytes (limit: 8192)", per_pane);
}
```

### AGENTS.md Rules (Section 24)

```
RULE S24-BENCH: Every PR touching hot paths must include benchmark results.
RULE S24-REGRESSION: Performance regressions >10% block merge. Use criterion comparisons.
RULE S24-MEMORY: Memory per pane must stay under 8KB. Test in CI.
```

### Test Strategy (Section 24)

- `criterion` benchmarks run on every PR via CI
- Memory tests use custom allocator tracking
- Baseline stored in `benches/baseline.json`

---

## 25. Visual Client / TUI

### 25.1 TUI Architecture

```rust
// crates/tf-tui/src/app.rs

use ratatui::prelude::*;

pub struct App {
    client: tf_client::Client,
    /// Current view state.
    view: ViewState,
    /// Pane content buffers.
    pane_buffers: BTreeMap<PaneId, PaneBuffer>,
}

pub enum ViewState {
    Normal,
    CommandMode { input: String },
    CopyMode { pane: PaneId, cursor: (u16, u16) },
    PaneSelection,
}

impl App {
    pub fn new(socket_path: &Path) -> Result<Self, TermForgeError> {
        let client = tf_client::Client::connect(socket_path)?;
        Ok(Self {
            client,
            view: ViewState::Normal,
            pane_buffers: BTreeMap::new(),
        })
    }

    pub fn render(&self, frame: &mut Frame) {
        // Render based on current layout from server
        let layout = self.client.current_layout();
        self.render_layout(frame, &layout, frame.area());
    }

    fn render_layout(&self, frame: &mut Frame, layout: &LayoutTree, area: Rect) {
        match layout {
            LayoutTree::Pane { id, .. } => {
                if let Some(buffer) = self.pane_buffers.get(id) {
                    frame.render_widget(PaneWidget::new(buffer), area);
                }
            }
            LayoutTree::Horizontal { children, .. } => {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(
                        children.iter().map(|_| Constraint::Ratio(1, children.len() as u32))
                    )
                    .split(area);
                for (i, child) in children.iter().enumerate() {
                    self.render_layout(frame, child, chunks[i]);
                }
            }
            LayoutTree::Vertical { children, .. } => {
                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(
                        children.iter().map(|_| Constraint::Ratio(1, children.len() as u32))
                    )
                    .split(area);
                for (i, child) in children.iter().enumerate() {
                    self.render_layout(frame, child, chunks[i]);
                }
            }
        }
    }
}
```

### 25.2 Pane Buffer

```rust
/// Buffer holding the visual content of a pane.
/// Uses a ring buffer for scrollback.
pub struct PaneBuffer {
    lines: VecDeque<Line<'static>>,
    max_scrollback: usize,
    cursor: (u16, u16),
}

impl PaneBuffer {
    pub fn new(max_scrollback: usize) -> Self {
        Self {
            lines: VecDeque::new(),
            max_scrollback,
            cursor: (0, 0),
        }
    }

    pub fn push_line(&mut self, line: Line<'static>) {
        self.lines.push_back(line);
        while self.lines.len() > self.max_scrollback {
            self.lines.pop_front();
        }
    }
}
```

### AGENTS.md Rules (Section 25)

```
RULE S25-RATATUI: TUI uses ratatui. No custom terminal drawing.
RULE S25-FRAMERATE: Target 60fps for interactive use. Skip frames if behind.
RULE S25-SCROLLBACK: Default scrollback is 10000 lines. Configurable via config.
```

### Test Strategy (Section 25)

```rust
#[test]
fn pane_buffer_scrollback_limit() {
    let mut buf = PaneBuffer::new(100);
    for i in 0..200 {
        buf.push_line(Line::raw(format!("line {i}")));
    }
    assert_eq!(buf.lines.len(), 100);
}

// Snapshot tests for TUI rendering
#[test]
fn render_single_pane() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let app = App::new_test();
    terminal.draw(|f| app.render(f)).unwrap();
    // Compare against snapshot
    insta::assert_snapshot!(terminal.backend().to_string());
}
```

---

## 26. AGENTS.md Rules

### 26.1 Complete AGENTS.md

```markdown
# AGENTS.md -- TermForge AI Agent Rules

## Code Quality
RULE S01-UNSAFE: #![forbid(unsafe_code)] in all crates except tf-pty. CI enforces.
RULE S08-NOPANIC: Production code must use Result, never unwrap/expect.
RULE S08-THISERROR: Library crates use thiserror. Binary crates may use anyhow.
RULE S08-CONTEXT: Every ? in binary crates must have .context().

## Architecture
RULE S03-DAG: No circular dependencies. CI runs cargo-deny.
RULE S03-LAYER: Each layer only depends on layers below it.
RULE S05-SYNC: tf-model must remain Send + Sync.
RULE S06-ID: Entity IDs use define_id! macro. No raw u64 in public APIs.
RULE S06-BTREE: Use BTreeMap for deterministic iteration.
RULE S07-PURE: EffectProcessor::process() is pure. No I/O.
RULE S07-EFFECT: ServerGraph mutated ONLY via Effect application.

## Query API
RULE S12-OPERATOR: All 17 operators must be tested (including Nin, IRegex).
RULE S12-KWARGS: Python kwargs use __ separator (name__startswith).
RULE S12-GET: get() raises ObjectDoesNotExist unless default is provided.

## Bindings
RULE S16-PYQUERY: server.sessions returns PyQueryList, not list.
RULE S16-KWARGS: Python kwargs use __ operator syntax.
RULE S16-DEFAULT: get(default=X) supported. get() without default raises.
RULE S16-JSQUERY: Node sessions() returns JsQueryList.
RULE S16-CALLABLE: filter() accepts callables in both Python and Node.

## Telemetry
RULE S19-DUAL: Server=OTEL_PROVIDER, Client=CLIENT_PROVIDER. Never share.
RULE S19-THREADLOCAL: Trace context via TRACE_HEADERS_STACK, not env vars.
RULE S19-TOML: OTEL config uses TOML with priority chain.
RULE S19-COMPOSITE: Propagator = Baggage + TraceContext.
RULE S19-ENABLE: Check: programmatic > TERMFORGE_OTEL > OTEL endpoint vars.
RULE S19-FLUSH: Call tf_otel::shutdown() before process exit.

## Build System
RULE S20-BLAKE3: Cache keys use BLAKE3 for flag fingerprinting.
RULE S20-LOCK: File locks required for all shared cache operations.
RULE S20-ATOMIC: Build artifacts atomically published via rename.
RULE S20-ENV: All build config settable via env vars.

## Test Isolation
RULE S21-THREELAYER: Three-layer socket validation. No exceptions.
RULE S21-DEVNULL: Every test tmux invocation uses -f /dev/null.
RULE S21-PERMS: Socket dirs chmod 0700 before use.
RULE S21-READINESS: Wait for socket readiness after server start.
RULE S21-CLIPBOARD: Disable set-clipboard unless opted in.
RULE S21-DROP: TmuxTestServer::drop calls shutdown().

## Protocol
RULE S09-COMPAT: Protocol byte-identical to tmux control mode.
RULE S09-ZEROCOPY: bytes::Bytes for output data.
RULE S15-RATE: 1000 notifications/second rate limit per client.

## Config
RULE S10-TOML: All config in TOML. No YAML, no JSON.

## Performance
RULE S24-REGRESSION: >10% perf regression blocks merge.
RULE S24-MEMORY: Memory per pane under 8KB.

## Security
RULE S18-PERMS: Socket files 0600, directories 0700.
RULE S18-INPUT: All client input validated before use.
RULE S18-NULLBYTE: Reject input containing null bytes.

## General
RULE S01-VISION: PRs must reference guiding principles.
RULE S01-COMPAT: Protocol changes require RFC.
RULE S04-DEPS: New dependencies require justification.
RULE S22-FIXTURE: Test servers use auto-cleanup fixtures.
RULE S23-PARALLEL: Tests must be parallel-safe.
```

---

## 27. Risks and Mitigations

### 27.1 Risk Register

| ID | Risk | Impact | Likelihood | Mitigation |
|----|------|--------|-----------|------------|
| R01 | tmux protocol changes break compat | High | Medium | Pin to specific tmux versions; protocol fuzz tests |
| R02 | PyO3/Neon API instability | Medium | Medium | Pin binding versions; integration test matrix |
| R03 | OTEL overhead in hot paths | High | Low | Feature-gated; export filter prevents transport noise |
| R04 | CRDT complexity delays delivery | High | Medium | Phase CRDT behind feature flag; single-server first |
| R05 | Unix socket path length limits | Medium | Low | Short temp dir paths; validate at creation |
| R06 | File lock deadlocks in CI | Medium | Low | Timeout on lock acquisition; lock ordering protocol |
| R07 | BLAKE3 dependency size | Low | Low | It is a small, well-maintained crate |
| R08 | Binding test flakiness | Medium | High | Retry logic; socket readiness checks; generous timeouts |
| R09 | Clipboard pollution in tests | Medium | Medium | set-clipboard off by default; env var escape hatch |
| R10 | Cross-platform PTY differences | High | Medium | FakePty for unit tests; real PTY only in integration |

### 27.2 Phasing Strategy

**Phase 1: Foundation** (Months 1-3)
- tf-model, tf-query, tf-event (core logic)
- tf-test (harness with three-layer validation)
- tf-otel (dual provider)
- Unit and integration test suites

**Phase 2: Protocol** (Months 3-5)
- tf-proto (control mode codec)
- tf-server (basic server)
- tf-client (client library)
- Protocol compatibility tests

**Phase 3: Bindings** (Months 5-7)
- tf-py with PyQueryList
- tf-node with JsQueryList
- Binding test suites

**Phase 4: TUI + Polish** (Months 7-9)
- tf-tui (ratatui visual client)
- Performance optimization
- Documentation
- CRDT feature flag

### AGENTS.md Rules (Section 27)

```
RULE S27-RISK: New features that introduce a new risk must update the risk register.
RULE S27-PHASE: Features must be delivered in phase order. No phase 3 without phase 2.
```

### Test Strategy (Section 27)

- Risk R01: Protocol fuzz tests run nightly
- Risk R06: Lock timeout test with parallel stress
- Risk R08: Retry wrapper for flaky socket operations

---

## 28. Plan Evolution and Changelog

### 28.1 Version History

| Version | Date | Author | Summary |
|---------|------|--------|---------|
| v1 | 2026-02-10 | GPT | Initial architecture |
| v2 | 2026-02-10 | Claude | Refinements |
| v3 | 2026-02-10 | Gemini | Alternative perspective |
| v4 | 2026-02-10 | GPT | Iteration |
| v5 | 2026-02-10 | Synthesized | Multi-model synthesis |
| v6 | 2026-02-10 | GPT | Extended |
| v7 | 2026-02-10 | Synthesized | 31-section complete spec |
| v8 | 2026-02-11 | Claude | 20 gap fixes from deep-dive review |

### 28.2 v8 Detailed Changelog

**Critical Fixes:**
1. Section 16: Added `PyQueryList` class wrapping Rust `QueryList<T>` with kwargs filter API
   (`server.sessions.filter(name="work")`), `__` operator syntax, callable support,
   `get(default=...)`, `__len__`, `__iter__`, `__getitem__`.
2. Section 16: Added `JsQueryList` with equivalent JavaScript API using object filters
   and callback support.
3. Section 20: Added file-based locking (`lock_cache_key` + `lock_repo_clone`) using
   `fs2::FileExt` for parallel build safety. Two lock levels prevent both repo clone
   races and concurrent build corruption.
4. Section 21: Added three-layer socket validation (`ensure_not_default_socket_name`,
   `ensure_socket_within_tempdir`, `ensure_socket_not_tmux_env`) modeled on vibe-tmux's
   `path_guard.rs`.

**High Fixes:**
5. Section 12: Added `Nin` (not-in) and `IRegex` (case-insensitive regex) operators
   to `QueryOp` enum, matching libtmux's complete operator set.
6. Section 19: Added dual-provider architecture (`OTEL_PROVIDER` + `CLIENT_PROVIDER`)
   with `OnceLock<Mutex<Option<OtelProvider>>>` pattern from vibe-tmux.
7. Section 19: Added thread-local `TRACE_HEADERS_STACK` with `TraceHeadersGuard` RAII
   pattern for nested span support and cross-thread capture-push propagation.
8. Section 20: Added BLAKE3 cache key computation with `flags_fingerprint()` for
   deterministic, collision-resistant build cache keys.
9. Section 20: Added atomic build publication via temp dir + rename pattern from
   vibe-tmux's `sibling_tmp_dir()` + `std::fs::rename()`.
10. Section 20: Added comprehensive env var interface (TERMFORGE_TMUX_VERSION,
    TERMFORGE_AUTO_BUILD, TERMFORGE_OFFLINE, etc.) for CI automation.
11. Section 21: Added `chmod_0700()` permission hardening for socket directories.
12. Section 21: Added `-f /dev/null` to every test tmux invocation for config isolation.
13. Section 21: Added `wait_for_socket()` readiness check using `UnixStream::connect()`.
14. Section 21: Added `MuxServerTestServer` enum with `InProcess` and `Subprocess` modes.

**Medium Fixes:**
15. Section 12: Added `get_or_default()` method matching libtmux's `get(default=None)`.
16. Section 19: Added TOML config priority chain (built-in < user < project < local < env).
17. Section 19: Added composite propagator (`BaggagePropagator` + `TraceContextPropagator`).
18. Section 19: Added env var enable toggle (`TERMFORGE_OTEL`, `set_otel_enabled()`).
19. Section 21: Added clipboard isolation (`set-clipboard off`) with
    `TERMFORGE_ALLOW_SYSTEM_CLIPBOARD` escape hatch.

**Low Fixes:**
20. Section 12: Added `filter_fn()` for callable matcher support matching libtmux's
    `filter(lambda s: ...)`.

### 28.3 Evolution Protocol

1. **Propose** -- Open a GitHub issue with the proposed change
2. **Review** -- Deep-dive review against reference codebases
3. **Spec** -- Update this document with concrete code
4. **Implement** -- PR with tests
5. **Validate** -- CI green + benchmark comparison

### AGENTS.md Rules (Section 28)

```
RULE S28-CHANGELOG: Every spec change must update the changelog with justification.
RULE S28-VERSION: Spec versions increment monotonically. Never reuse a version number.
```

---

## 29. Reference Anchors

### 29.1 Reference Codebases

| Codebase | Path | Used For |
|----------|------|----------|
| libtmux QueryList | `~/work/python/libtmux/src/libtmux/_internal/query_list.py` | 12 operators, kwargs filter, callable matcher |
| libtmux Server | `~/work/python/libtmux/src/libtmux/server.py` | Server->Session->Window->Pane traversal |
| vibe-tmux path_guard | `~/work/rust/vibe-tmux/crates/mux-test-support/src/path_guard.rs` | Three-layer socket validation |
| vibe-tmux TmuxTestServer | `~/work/rust/vibe-tmux/crates/mux-test-support/src/tmux.rs` | Test isolation, permission hardening |
| vibe-tmux tmux-builder | `~/work/rust/vibe-tmux/tools/tmux-builder/src/lib.rs` | BLAKE3 cache, file locking, atomic publish |
| vibe-tmux mux-otel | `~/work/rust/vibe-tmux/crates/mux-otel/src/otel.rs` | Dual provider, trace headers stack |
| vibe-tmux otel config | `~/work/rust/vibe-tmux/crates/mux-otel/src/config.rs` | TOML priority chain, export filter |
| vibe-tmux otel lib | `~/work/rust/vibe-tmux/crates/mux-otel/src/lib.rs` | TraceHeaders, TraceHeadersGuard |

### 29.2 Cross-Reference Map

| Concept | Spec Section | Reference Implementation |
|---------|-------------|------------------------|
| QueryList<T> | 12 | libtmux `query_list.py` |
| QueryOp (17 operators) | 12.1 | libtmux LOOKUP_NAME_MAP |
| kwargs_to_specs | 12.4 | libtmux `filter(**kwargs)` |
| PyQueryList | 16.2 | libtmux Server.sessions |
| JsQueryList | 16.4 | (new design) |
| OtelProvider (dual) | 19.1 | vibe-tmux `OTEL_PROVIDER` + `MUX_CLIENT_PROVIDER` |
| TRACE_HEADERS_STACK | 19.2 | vibe-tmux `lib.rs` thread_local |
| TraceHeadersGuard | 19.2 | vibe-tmux `TraceHeadersGuard` |
| composite_propagator | 19.3 | vibe-tmux `composite_propagator()` |
| ExportFilterConfig | 19.5 | vibe-tmux `config.rs` |
| otel_enabled() | 19.6 | vibe-tmux `otel_enabled()` |
| compute_cache_key | 20.1 | vibe-tmux `compute_cache_key()` |
| flags_fingerprint (BLAKE3) | 20.1 | vibe-tmux `flags_fingerprint()` |
| lock_cache_key | 20.2 | vibe-tmux `lock_cache_key()` |
| lock_repo_clone | 20.2 | vibe-tmux `lock_repo_clone()` |
| sibling_tmp_dir | 20.3 | vibe-tmux `sibling_tmp_dir()` |
| ensure_not_default_socket_name | 21.1 | vibe-tmux `path_guard.rs` |
| ensure_socket_within_tempdir | 21.1 | vibe-tmux `path_guard.rs` |
| ensure_socket_not_tmux_env | 21.1 | vibe-tmux `path_guard.rs` |
| chmod_0700 | 21.2 | vibe-tmux `tmux.rs` |
| wait_for_socket | 21.3 | vibe-tmux `wait_for_socket()` |
| TmuxTestServer | 21.3 | vibe-tmux `TmuxTestServer` |
| clipboard isolation | 21.3 | vibe-tmux `set-clipboard off` |

### AGENTS.md Rules (Section 29)

```
RULE S29-REFERENCE: When implementing a pattern from a reference codebase, cite it
    in the code comment with the source file path.
RULE S29-CROSSREF: Keep the cross-reference map updated when adding new patterns.
```

---

## 30. Appendix: Canonical Type Quick Reference

### 30.1 Core Types

```rust
// Identity types
pub struct SessionId(u64);
pub struct WindowId(u64);
pub struct PaneId(u64);
pub struct ClientId(u64);

// Entity types
pub struct Session { id: SessionId, name: String, windows: BTreeMap<WindowId, Window>, ... }
pub struct Window { id: WindowId, name: String, panes: BTreeMap<PaneId, Pane>, ... }
pub struct Pane { id: PaneId, title: String, width: u16, height: u16, ... }

// Query types
pub enum QueryOp { Exact, IExact, Contains, IContains, StartsWith, IStartsWith,
    EndsWith, IEndsWith, In, Nin, Regex, IRegex, Gt, Gte, Lt, Lte, IsNone, IsSome }
pub struct QuerySpec { field: String, op: QueryOp }
pub struct QueryList<T: Queryable> { items: Vec<T> }
pub enum FieldValue { String, U64, U32, U16, Bool, None }
pub enum QueryError { ObjectDoesNotExist, MultipleObjectsReturned, UnknownField, InvalidOperator }

// Event types
pub enum Command { NewSession, KillSession, NewWindow, ... }
pub enum Effect { SessionCreated, SessionDestroyed, WindowCreated, ... }
pub enum Notification { SessionChanged, WindowRenamed, PaneOutput, ... }

// Protocol types
pub enum ControlMessage { Command, Begin, End, OutputLine }
pub enum ControlResponse { Begin, End, Output, Notification, Error }
pub enum ControlNotification { SessionChanged, WindowAdd, Output, ... }

// Layout types
pub enum LayoutTree { Pane { id, x, y, w, h }, Horizontal { children }, Vertical { children } }
pub enum LayoutPreset { EvenHorizontal, EvenVertical, MainHorizontal, MainVertical, Tiled }

// CRDT types
pub struct VersionVector { clocks: BTreeMap<ReplicaId, u64> }
pub struct Transaction { id: u64, replica: ReplicaId, version: VersionVector, effects: Vec<Effect> }

// Config types
pub struct TermForgeConfig { server: ServerConfig, session: SessionConfig, ... }
pub struct OtelConfig { export_filter: ExportFilterConfig }
pub struct ExportFilterConfig { allow_prefixes, allow_exact, deny_prefixes, deny_exact }

// Telemetry types
pub struct TraceHeaders { traceparent: String, tracestate: Option<String>, baggage: Option<String> }
pub struct TraceHeadersGuard { depth: usize, restore: bool }
pub struct OtelProvider { logger, tracer_provider, tracer, runtime }
pub struct ClientSpanGuard { context: Context, _trace_guard: Option<TraceHeadersGuard> }

// Test types
pub struct TmuxTestServer { tmux_bin, tmux_tmpdir, socket_name, socket_path, session_name }
pub enum MuxServerTestServer { InProcess { graph, event_tx, ... }, Subprocess { child, socket_path, ... } }
pub struct FakePtyBackend { pending_output, captured_input, alive, width, height }
pub struct LockGuard { file: std::fs::File }

// Binding types
pub struct PyQueryList { items: Vec<PyObject>, query_list: QueryList<DynQueryable> }
pub struct JsQueryList { items: Vec<Root<JsObject>>, query_list: QueryList<DynQueryable> }
pub struct PyServer { inner: Arc<RwLock<ServerGraph>> }
pub struct PySession { inner: Session }
```

### 30.2 Trait Summary

```rust
pub trait Queryable {
    fn field_value(&self, field: &str) -> Option<FieldValue>;
    fn field_names() -> &'static [&'static str];
}
// Implemented by: Session, Window, Pane
```

### 30.3 Static Summary

```rust
// Telemetry statics
static OTEL_PROVIDER: OnceLock<Mutex<Option<OtelProvider>>>;
static CLIENT_PROVIDER: OnceLock<Mutex<Option<OtelProvider>>>;
static OTEL_ENABLED_OVERRIDE: AtomicBool;
static TRACING_INIT: OnceLock<()>;
static CONFIG: OnceLock<OtelConfig>;
thread_local! { static TRACE_HEADERS_STACK: RefCell<Vec<TraceHeaders>>; }
```

---

## 31. Supplemental Test Matrix

### 31.1 Complete Test Matrix

| Section | Test Type | Count | Key Assertions |
|---------|-----------|-------|----------------|
| 1 | Compile | 1 | `#![forbid(unsafe_code)]` in all non-PTY crates |
| 5 | Structural | 1 | No upward dependencies in DAG |
| 6 | Unit | 2 | ServerGraph traversal, Queryable field access |
| 7 | Unit | 2 | Process purity, idempotent kill |
| 8 | Unit | 1 | QueryError maps to Python LookupError |
| 9 | Unit+Fuzz | 3 | Round-trip, notification parse, fuzz target |
| 10 | Unit | 3 | Default valid, merge last-wins, missing file |
| 11 | Unit+Snapshot | 2 | Even distribution, layout string round-trip |
| 12 | Unit | 8 | Exact, Nin, IRegex, get, get_or_default, callable, kwargs |
| 14 | Integration | 2 | Stale socket cleanup, active server prevention |
| 15 | Unit | 2 | Begin/end pairing, rate limiter |
| 16 | Integration | 11 | PyQueryList filter/get/iter/index, JsQueryList filter/get |
| 17 | Unit | 2 | Version vector dominance, transaction log |
| 18 | Unit | 3 | Path traversal, null bytes, socket perms |
| 19 | Unit | 6 | Thread-local isolation, capture-push, nested restore, export filter, deny precedence, env toggle |
| 20 | Unit | 5 | Cache key determinism, different flags, file lock, version satisfies, sibling tmp unique |
| 21 | Unit+Integration | 8 | Default name rejected, tempdir guard, tmux env guard, socket path format, unique names, fake PTY, server integration |
| 22 | Integration | 12 | Python filter/get/default/callable/traversal, Node filter/get/default, compat |
| 23 | Integration | 1 | Parallel safety stress (10 concurrent servers) |
| 24 | Benchmark | 3 | Query filter 10K, effect processing, layout relayout |
| 25 | Unit+Snapshot | 2 | Scrollback limit, render snapshot |

### 31.2 Coverage Targets

| Crate | Target | Method |
|-------|--------|--------|
| tf-model | 90% line | `cargo tarpaulin` |
| tf-query | 95% line | `cargo tarpaulin` (critical path) |
| tf-event | 90% line | `cargo tarpaulin` |
| tf-proto | 85% line + fuzz | `cargo tarpaulin` + `cargo fuzz` |
| tf-otel | 80% line | `cargo tarpaulin` (hard to test OTEL infra) |
| tf-version | 85% line | `cargo tarpaulin` |
| tf-test | 90% line | `cargo tarpaulin` |
| tf-py | 80% line | `pytest --cov` |
| tf-node | 80% line | `jest --coverage` |

### 31.3 Test Environment Matrix

| Dimension | Values |
|-----------|--------|
| OS | Ubuntu 24.04, macOS 14 |
| Rust | stable, beta, nightly |
| tmux | 3.3a, 3.4, 3.5a |
| Python | 3.10, 3.11, 3.12, 3.13 |
| Node.js | 18, 20, 22 |

---

*End of TermForge v8 Architecture Specification*
