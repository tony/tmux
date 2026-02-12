# TermForge v13 Architecture Specification -- Pass 1 [v13 P1]

Date: 2026-02-11
Status: **DRAFT** -- v13 Pass 1 of 3.
Lineage: v12 P3 (Definitive) -> v13 Pass 1 (Gemini)
Models: Gemini (v13 Pass 1)
License: MIT OR Apache-2.0
Rust edition: 2024 (MSRV 1.85)
Protocol target: tmux protocol v8

---

## Preamble

This document is the **v13 Pass 1** architecture specification for TermForge. It builds upon the v12 P3 DEFINITIVE spec, deepening key technical areas that were under-specified, specifically the Grid data structures, VtParser state machine, and Cross-Language Binding hierarchy.

### [v13 P1] Key Additions

1.  **[v13] Grid API Deepening**: Introduced `Line` abstraction for row-specific metadata (wrap status) and `Viewport` for partial rendering.
2.  **[v13] VtParser State Machine**: Defined the 7-state machine and action dispatch table structure (S5).
3.  **[v13] PtyHandle Lifecycle**: Explicit resource cleanup protocols to prevent handle leaks (S6).
4.  **[v13] Termlet Snapshot Binary Format**: Added `to_binary()` using `bincode` for efficient storage and diffing (S32).
5.  **[v13] Binding Hierarchy**: Explicit class hierarchy for PyO3 (`PySession`, `PyWindow`, `PyPane`, `PyTermlet`) (S16).
6.  **[v13] CRDT for Panes**: Clarified `OpLog` usage for concurrent pane input stream merging (S17).

---

## 1. Project Identity

### Design Decisions

-   Name: TermForge
-   CLI binary: `termforge` (primary), `tf` (alias)
-   Library name: `termforge` (Python/Node packages)
-   Crate prefix: `mux-*` (all Rust crates)
-   Config file: `~/.config/termforge/termforge.conf`
-   Default socket dir: `$TMPDIR/termforge-$UID/`
-   Env prefix: `TERMFORGE_*`
-   Traceability: `API-001`, `OPS-001`.

### Rust Example

```rust
// crates/mux-types/src/identity.rs
pub const PROJECT_NAME: &str = "TermForge";
pub const BINARY_NAME: &str = "termforge";
pub const BINARY_ALIAS: &str = "tf";
pub const SOCKET_PREFIX: &str = "termforge";

/// Default socket directory: $TMPDIR/termforge-$UID/
pub fn default_socket_dir() -> std::path::PathBuf {
    let uid = unsafe { libc::getuid() };
    std::env::temp_dir().join(format!("{SOCKET_PREFIX}-{uid}"))
}

/// Config path respecting XDG_CONFIG_HOME.
pub fn config_path() -> std::path::PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".config")))
        .unwrap_or_else(|| std::path::PathBuf::from(".config"));
    base.join("termforge").join("termforge.conf")
}

#[derive(Debug, Clone)]
pub struct ProductIdentity {
    pub name: &'static str,
    pub protocol_target: &'static str,
    pub license: &'static str,
}

impl ProductIdentity {
    pub const fn termforge() -> Self {
        Self {
            name: "TermForge",
            protocol_target: "tmux-wire-compat-v8",
            license: "MIT OR Apache-2.0",
        }
    }
}
```

### Test Strategy

1.  Binary name: `assert_eq!(BINARY_NAME, "termforge")`.
2.  Alias detection: `argv[0]` check in integration test.
3.  Socket dir: Regex check for `termforge-\d+`.
4.  Config path: `XDG_CONFIG_HOME` overrides default.
5.  `TST-001`: Config path resolution.
6.  `TST-002`: Default socket root format.

### AGENTS.md Rules

-   `RULE-S01-01`: All crate names must use the `mux-` prefix.
-   `RULE-S01-02`: Binary name `termforge`, alias `tf`.
-   `RULE-S01-03`: Compatibility scope changes update Section 20 matrices.

---

## 2. Acceptance Criteria and Gates

### Design Decisions

-   Four gate classes: `compat`, `correctness`, `performance`, `operability`.
-   **[v13]** Added B18 (Line wrap churn) and A6 (VtParser state transitions).

### 2.1 Acceptance Gate Table

| ID | Class | Category | Gate | Test / Evidence | Threshold |
|---|---|---|---|---|---|
| C1 | compat | Compatibility | `tmux attach` | Integration test | Attach succeeds |
| A1 | correctness | Purity | `mux-core` no IO | `wasm32` check | Clean compile |
| A6 | correctness | Purity | **[v13]** VtParser state coverage | Fuzz coverage | 100% state reachability |
| B1 | performance | Perf | VT100 ASCII | Benchmark | > 300 MB/s |
| B16 | performance | Perf | Grid cell_at | Benchmark | < 10 ns/cell |
| B18 | performance | Perf | **[v13]** Line wrap reflow | Benchmark | < 50 us/screen |
| P1 | operability | Prog | Python filter | pytest | Correct QueryList |
| P18 | operability | Prog | TermletLike trait | Compile test | Trait compliance |
| E1 | ecosystem | Eco | pip install | CI | Success |

### Rust Example

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GateClass {
    Compat, Correctness, Performance, Operability,
}

#[derive(Debug, Clone, Copy)]
pub enum Gate {
    C1WireAttach,
    A1NoPureIo, A6VtParserStates,
    B1Vt100Ascii, B16GridCellAt, B18LineWrapReflow,
    P1PyFilter, P18TermletLike,
    E1PyPip,
}

impl Gate {
    pub fn release_blocking(self) -> bool {
        match self {
            Gate::B1Vt100Ascii | Gate::B16GridCellAt | Gate::B18LineWrapReflow => false,
            _ => true,
        }
    }
}
```

### Test Strategy

1.  Blocking gates fail the release pipeline.
2.  Performance gates warn on regression > 30%.
3.  `TST-010`: Gate inventory check.

### AGENTS.md Rules

-   `RULE-S02-01`: Every feature maps to a gate.
-   `RULE-S02-02`: No gate removal without spec amendment.
-   `RULE-S02-06`: Expired quarantines block release.

---

## 3. Crate Dependency Rules

### Design Decisions

-   Layer 0: `mux-types`, `mux-grid`, `mux-proto`, `mux-query`, `mux-crdt`, `mux-core`, `mux-pty-fake`.
-   Layer 1: `mux-state`, `mux-conf`, `mux-keys`, `mux-pty`, `mux-os`.
-   Layer 2: `mux-termlet`, `mux-server`, `mux-api`, `mux-orm`.
-   Layer 3: Bindings, `termforge` binary.
-   **[v13]** `mux-grid` moved to Layer 0 explicitly (was implicitly there).

### Rust Example

```rust
pub struct CrateLayer { pub name: &'static str, pub layer: u8 }

pub const CRATE_LAYERS: &[CrateLayer] = &[
    CrateLayer { name: "mux-types", layer: 0 },
    CrateLayer { name: "mux-grid", layer: 0 },
    CrateLayer { name: "mux-core", layer: 0 },
    CrateLayer { name: "mux-termlet", layer: 2 },
];
```

### Test Strategy

1.  Dependency graph linter.
2.  `mux-core` WASM proof.
3.  `TST-020`: Forbidden edge check.

### AGENTS.md Rules

-   `RULE-S03-01`: Layer violations are blocking errors.
-   `RULE-S03-04`: `mux-core` stays Layer 0.

---

## 4. Workspace Layout

### Design Decisions

-   Standard Cargo workspace.
-   `crates/` for core, `bindings/` for FFI, `tools/` for dev tools.

### Rust Example

```text
termforge/
  Cargo.toml
  crates/
    mux-grid/       # [v13] Grid, Line, VtParser
    mux-core/       # ServerGraph
    mux-termlet/    # Termlet SDK
  bindings/
    python/
    node/
```

### AGENTS.md Rules

-   `RULE-S04-01`: All crates in workspace.
-   `RULE-S04-03`: `mux-termlet` depends on L0+L1 only.

---

## 5. mux-core: Pure Kernel & Grid [v13 Updated]

### Design Decisions

-   **[v13] Grid API**: `Grid` is composed of `Vec<Line>`.
-   **[v13] Line**: Contains `Vec<Cell>` and metadata (`wrapped`).
-   **[v13] VtParser**: Deterministic state machine with explicit `Action` table.
-   Layer 0 purity maintained.

### 5.1 Grid API [v13 Deepened]

```rust
// crates/mux-grid/src/lib.rs
#![forbid(unsafe_code)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    pub grapheme: String,
    pub fg: Color,
    pub bg: Color,
    pub attrs: CellAttrs,
    pub width: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub cells: Vec<Cell>,
    pub is_wrapped: bool,
}

impl Line {
    pub fn new(width: usize) -> Self {
        Self { cells: vec![Cell::default(); width], is_wrapped: false }
    }
}

pub struct Grid {
    lines: std::collections::VecDeque<Line>,
    cols: u16,
    rows: u16,
    cursor: CursorPos,
    scrollback_limit: usize,
}

impl Grid {
    pub fn new(cols: u16, rows: u16) -> Self {
        let mut lines = std::collections::VecDeque::with_capacity(rows as usize);
        for _ in 0..rows { lines.push_back(Line::new(cols as usize)); }
        Self {
            lines, cols, rows,
            cursor: CursorPos::default(),
            scrollback_limit: 10_000,
        }
    }

    pub fn line_at(&self, row: u16) -> Option<&Line> {
        self.lines.get(row as usize)
    }

    pub fn cell_at(&self, col: u16, row: u16) -> Option<&Cell> {
        self.lines.get(row as usize)?.cells.get(col as usize)
    }
}
```

### 5.2 VtParser State Machine [v13 Deepened]

```rust
// crates/mux-grid/src/parser.rs

#[derive(Debug, Clone, Copy)]
pub enum State {
    Ground, Escape, EscapeInt, CsiEntry, CsiParam, CsiInt,
    DcsEntry, DcsParam, DcsInt, DcsPass, OscString, SosString, PmString, ApcString,
}

#[derive(Debug, Clone, Copy)]
pub enum Action {
    Ignore, Print, Execute, Clear, Collect, Param, PrintOsc, Hook, Put, Unhook, OscStart, OscPut, OscEnd,
    CsiDispatch, EscDispatch,
}

// [v13] Explicit state table structure (simplified for example).
// Real implementation would be a [State][u8] -> (State, Action) lookup.
pub struct VtParser {
    state: State,
    params: Vec<u16>,
    osc_buffer: Vec<u8>,
}

impl VtParser {
    pub fn process(&mut self, byte: u8, grid: &mut Grid) {
        let (next_state, action) = self.lookup(self.state, byte);
        self.perform_action(action, byte, grid);
        self.state = next_state;
    }

    fn lookup(&self, state: State, byte: u8) -> (State, Action) {
        // [v13] Implementation of the tmux input.c state transition table
        match (state, byte) {
            (State::Ground, 0x1B) => (State::Escape, Action::Clear),
            (State::Ground, b) if b >= 0x20 => (State::Ground, Action::Print),
            // ... complete table ...
            _ => (State::Ground, Action::Ignore),
        }
    }

    fn perform_action(&mut self, action: Action, byte: u8, grid: &mut Grid) {
        match action {
            Action::Print => { /* grid.put_char(...) */ },
            Action::CsiDispatch => { /* dispatch based on intermediate + final */ },
            _ => {},
        }
    }
}
```

### Test Strategy

1.  `TST-050`: Grid resize reflow preserves wrapped lines.
2.  `TST-051`: VtParser state transition fuzzing.
3.  `TST-052`: Verify `Line` wrapping flag behavior.

### AGENTS.md Rules

-   `RULE-S05-01`: `mux-core` is pure.
-   `RULE-S05-05`: **[v13]** VtParser must implement all 14 standard states.

---

## 6. Entity Model

### Design Decisions

-   Entity IDs via `slotmap`.
-   **[v13]** `PtyHandle` cleanup: explicit `Drop` behavior or owner-checked destruction to prevent FD leaks.

### Rust Example

```rust
// crates/mux-types/src/entities.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PtyHandle(pub u64);

// crates/mux-pty/src/lib.rs
pub struct PtyRegistry {
    handles: std::collections::HashMap<PtyHandle, std::fs::File>,
}

impl PtyRegistry {
    pub fn close(&mut self, handle: PtyHandle) {
        if let Some(file) = self.handles.remove(&handle) {
            // File drops here, closing FD
            drop(file);
        }
    }
}
```

### AGENTS.md Rules

-   `RULE-S06-05`: **[v13]** `PtyHandle` must be explicitly closed via registry.

---

## 7. Event-Effect Architecture

### Design Decisions

-   Core emits effects; runtime executes.
-   `CausationId` links events to effects.

### AGENTS.md Rules

-   `RULE-S07-01`: Core never does IO.

---

## 8. Error Taxonomy

### Design Decisions

-   `thiserror` for libs.
-   `ErrorCode` trait for bindings.

---

## 9. Wire Protocol

### Design Decisions

-   tmux v8 compatible.

---

## 10. Configuration

### Design Decisions

-   FALLTHROUGH resolution.

---

## 11. Layout Engine

### Design Decisions

-   Flat `Vec<LayoutCell>`.

---

## 12. ORM and QueryList

### Design Decisions

-   Django-style filters.

---

## 13. State Actor

### Design Decisions

-   `ArcSwap` for snapshots.

---

## 14. Server Lifecycle

### Design Decisions

-   Flock locking.

---

## 15. Control Mode

### Design Decisions

-   Typed notifications.

---

## 16. Language Bindings [v13 Deepened]

### Design Decisions

-   **[v13]** Explicit class hierarchy for PyO3.
-   `PyTermlet` wraps `TermletPaneId`.

### Rust Example

```rust
// bindings/python/src/lib.rs
#[pyclass]
pub struct PySession { id: SessionId, handle: StateHandle }

#[pyclass]
pub struct PyWindow { id: WindowId, handle: StateHandle }

#[pyclass]
pub struct PyPane { id: PaneId, handle: StateHandle }

#[pyclass]
pub struct PyTermlet { inner: std::sync::Arc<parking_lot::Mutex<Termlet>> }

#[pymethods]
impl PyTermlet {
    #[staticmethod]
    fn spawn(cmd: &str) -> PyResult<Self> {
        let config = TermletConfig::default();
        let t = Termlet::spawn(cmd, config).map_err(to_py_err)?;
        Ok(Self { inner: std::sync::Arc::new(parking_lot::Mutex::new(t)) })
    }
    
    fn wait_for(&self, pattern: &str, py: Python<'_>) -> PyResult<()> {
        py.allow_threads(|| {
            self.inner.lock().wait_for(pattern, Duration::from_secs(5)).map_err(to_py_err)?;
            Ok(())
        })
    }
}
```

### AGENTS.md Rules

-   `RULE-S16-06`: **[v13]** PyO3 classes must release GIL for blocking ops.

---

## 17. CRDT Layer [v13 Updated]

### Design Decisions

-   **[v13]** `OpLog` used for concurrent pane input merging (interleaved event stream).

### Rust Example

```rust
// crates/mux-crdt/src/lib.rs
pub enum InputOp {
    Key { code: Vec<u8> },
    Paste { text: String },
}

pub struct PaneInputLog {
    ops: Vec<(HLC, InputOp)>,
}

impl PaneInputLog {
    pub fn merge(&mut self, other: &PaneInputLog) {
        // Standard causal merge
    }
}
```

---

## 18. Socket and Permissions

### Design Decisions

-   `0700`/`0600` permissions.

---

## 19. Observability

### Design Decisions

-   Dual OTEL providers.

---

## 20. tmux Builder

### Design Decisions

-   BLAKE3 cache keys.

---

## 21. Test Support

### Design Decisions

-   `FakePtyBackend`.

---

## 22. Parity Framework

### Design Decisions

-   `mux-regress` runner.

---

## 23. Fuzzing

### Design Decisions

-   VtParser fuzzing.

---

## 24. Benchmarks

### Design Decisions

-   Criterion targets.

---

## 25. TUI

### Design Decisions

-   ViewModel pattern.

---

## 26. Rules

### Design Decisions

-   Consolidated table.

---

## 27. Risks

### Design Decisions

-   Risk coverage matrix.

---

## 28. Plan Evolution

### Design Decisions

-   v13 P1 delta tracking.

---

## 29. Anchors

### Design Decisions

-   tmux source links.

---

## 30. Type Reference

### Design Decisions

-   Canonical types.

---

## 31. Test Matrix

### Design Decisions

-   Release gates.

---

## 32. Termlets [v13 Deepened]

### Design Decisions

-   **[v13]** Binary Snapshot format.
-   `TermletLike` trait.

### 32.1 Snapshot Binary Format [v13 New]

```rust
// crates/mux-termlet/src/snapshot.rs
#[derive(serde::Serialize, serde::Deserialize)]
pub struct BinarySnapshot {
    pub version: u8,
    pub cols: u16,
    pub rows: u16,
    pub cells: Vec<u8>, // Compressed/packed cell data
}

impl TermletSnapshot {
    pub fn to_binary(&self) -> Vec<u8> {
        let bin = BinarySnapshot {
            version: 1,
            cols: self.cols,
            rows: self.rows,
            cells: bincode::serialize(&self.grid_ref).unwrap(),
        };
        bincode::serialize(&bin).unwrap()
    }
}
```

### AGENTS.md Rules

-   `RULE-S32-50`: **[v13]** Termlet snapshots must support binary serialization.

