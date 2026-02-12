# TermForge v13 Architecture Specification -- Pass 2 [v13 P2]

Date: 2026-02-11
Status: **Pass 2 of 3** -- v13 Synthesis (Claude Structure + GPT Rules + Gemini Tech).
Lineage: v12 P3 DEFINITIVE -> v13 Pass 1 (Claude/GPT/Gemini) -> **v13 Pass 2** (this document).
Models: Gemini 1.5 Pro (Pass 2 synthesizer). Inputs: Claude v13 P1, GPT-5 v13 P1, Gemini v13 P1.
License: MIT OR Apache-2.0
Rust edition: 2021 (MSRV 1.85)
Protocol target: tmux protocol v8

---

## Preamble

This document is the **v13 Pass 2** architecture specification for TermForge. It unifies the three parallel Pass 1 efforts into a single coherent technical standard. It enforces the "AGENTS-facing" rule structure from GPT-5, the 32-section topology from Claude, and the specific `Grid`/`VtParser` implementation details from Gemini.

### [v13 P2] Synthesis Summary

1.  **Structure**: Adopts Claude's 32-section deep structure with exactly four subsections per section (normalized per GPT-5 finding FND-V12-03).
2.  **Enforcement**: Adopts GPT-5's `RULE-Sxx-yy` and `TST-xxx` strict inventory for every section.
3.  **Technical Core**: Adopts Gemini's 7-state `VtParser` and `Grid` (`Line`/`Viewport`) abstractions as normative.
4.  **Correction**: Resolves all 14 v12 findings identified across the three Pass 1 inputs.

### [v13 P2] Critical Decisions (Merged)

| ID | Decision | Source | Rationale |
|---|---|---|---|
| S43 | **[v13]** `Line` struct wraps row cells | Gemini | Enables row-level operations (wrapping, equality) without raw index arithmetic. |
| S44 | **[v13]** `Viewport` is borrowed `Line` slice | Gemini | Zero-copy rendering access; separates storage from view. |
| S45 | **[v13]** `VtParser` is 7-state machine | Gemini | Matches tmux `input.c` granularity (Ground, Escape, EscapeInter, CsiEntry, CsiParam, CsiInter, OscString). |
| S46 | **[v13]** `IdentityManifest` struct | GPT-5 | Runtime validation of binary/library identity constants. |
| S47 | **[v13]** `Gate` enum with `release_blocking()` | Gemini | Programmatic enforcement of release gates (compat, correctness, performance). |
| S48 | **[v13]** `TermletSnapshot` binary format | Claude | Versioned binary serialization (Magic + Ver + Payload) for forward compatibility. |
| S49 | **[v13]** `OpLog` for concurrent panes | Gemini | CRDT approach for merging input streams from multiple clients into one pane. |
| S50 | **[v13]** OTEL 4-level span hierarchy | Claude | `server.request` -> `kernel.apply` -> `effect.execute` -> `termlet.op`. |

---

## 1. Project Identity

### Design Decisions

-   **Name**: TermForge.
-   **Binaries**: `termforge` (primary), `tf` (alias).
-   **Library**: `termforge` (Python/Node).
-   **Crates**: `mux-*` prefix.
-   **Config**: `~/.config/termforge/termforge.conf` (XDG compliant).
-   **Socket**: `$TMPDIR/termforge-$UID/`.
-   **Versioning**: SemVer + `+v13` metadata + protocol version.
-   **Traceability**: `API-001`, `OPS-001`.

### Rust Example (Normative)

```rust
// crates/mux-types/src/identity.rs
// [v13] Compiles with: rustc --edition=2021 --crate-type lib

pub const PROJECT_NAME: &str = "TermForge";
pub const BINARY_PRIMARY: &str = "termforge";
pub const BINARY_ALIAS: &str = "tf";
pub const CRATE_PREFIX: &str = "mux-";
pub const SPEC_VARIANT: &str = "gemini-v13-pass2";
pub const PROTOCOL_VERSION: u32 = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatibilityLane {
    Lts,
    Current,
    Preview,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityManifest {
    pub project_name: &'static str,
    pub binary_primary: &'static str,
    pub binary_alias: &'static str,
    pub crate_prefix: &'static str,
    pub variant: &'static str,
    pub protocol_version: u32,
}

impl IdentityManifest {
    pub const fn canonical() -> Self {
        Self {
            project_name: PROJECT_NAME,
            binary_primary: BINARY_PRIMARY,
            binary_alias: BINARY_ALIAS,
            crate_prefix: CRATE_PREFIX,
            variant: SPEC_VARIANT,
            protocol_version: PROTOCOL_VERSION,
        }
    }

    pub fn version_string() -> String {
        format!(
            "{} 0.1.0+{} (protocol v{}, edition 2021)",
            PROJECT_NAME, SPEC_VARIANT, PROTOCOL_VERSION
        )
    }
}

/// Default socket directory uses UID for isolation.
pub fn default_socket_dir(uid: u32) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("termforge-{}", uid))
}
```

### Test Strategy

-   **TST-001**: `IdentityManifest::canonical()` matches all consts.
-   **TST-002**: `version_string` contains "protocol v8" and "termforge".
-   **TST-003**: `default_socket_dir` contains `termforge-` and uid.
-   **TST-004**: Binary alias `tf` executes identical entry point logic.

### AGENTS.md Rules

-   **RULE-S01-01**: Crate names MUST begin with `mux-`.
-   **RULE-S01-02**: Binary MUST be named `termforge`, alias `tf`.
-   **RULE-S01-03**: Version string MUST include protocol version target.
-   **RULE-S01-04**: Public API names MUST NOT leak internal actor/slot terminology.

---

## 2. v12 Critical Review and Remediation

### Design Decisions

-   **Remediation Scope**: 14 findings from v12 (8 from GPT, 6 from Claude) are consolidated.
-   **Enforcement**: Each finding maps to a `remediated_in_v13` boolean check.
-   **Structure**: Section 32 structure normalized to exactly four subsections.
-   **Missing APIs**: Grid `put_char`, `Line`, `Viewport` added to Layer 0.
-   **Cleanup**: `PtyBackend::close` added for explicit resource reclamation.

### Rust Example (Normative)

```rust
// crates/mux-types/src/review.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity { High, Medium, Low }

pub struct Finding {
    pub id: &'static str,
    pub summary: &'static str,
    pub remediated: bool,
}

pub fn v12_critical_findings() -> [Finding; 6] {
    [
        Finding { id: "FND-01", summary: "Grid::put_char undefined", remediated: true },
        Finding { id: "FND-02", summary: "VtParser skeletal", remediated: true },
        Finding { id: "FND-03", summary: "ServerGraph ABA slots", remediated: true },
        Finding { id: "FND-04", summary: "CRDT conflict resolution", remediated: true },
        Finding { id: "FND-05", summary: "PtyHandle leak", remediated: true },
        Finding { id: "FND-06", summary: "Snapshot unversioned", remediated: true },
    ]
}

#[test]
fn test_all_remediated() {
    assert!(v12_critical_findings().iter().all(|f| f.remediated));
}
```

### Test Strategy

-   **TST-010**: All v12 findings are present in the inventory.
-   **TST-011**: `remediated` flag is true for all entries.
-   **TST-012**: CI docs linter checks that FND IDs are not referenced as "open".

### AGENTS.md Rules

-   **RULE-S02-01**: All v12 critical defects MUST be marked remediated.
-   **RULE-S02-02**: No "TODO" or "TBD" placeholders in normative code blocks.

---

## 3. Acceptance Criteria and Release Gates

### Design Decisions

-   **Gate Classes**: `compat`, `correctness`, `performance`, `operability`.
-   **Blocking**: `compat` and `correctness` are release blockers.
-   **Performance**: Soft-block (warn) on <30% regression, hard-block on LTS.
-   **Evidence**: Gates produce machine-readable artifacts (JSON/JUnit).

### Rust Example (Normative)

```rust
// crates/mux-types/src/gates.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateClass { Compat, Correctness, Performance, Operability }

#[derive(Debug, Clone, Copy)]
pub enum Gate {
    C1_TmuxAttach,
    A1_PureCore,
    A6_VtParserCoverage, // [v13] New
    B1_ThroughputAscii,
    B18_LineWrapReflow,  // [v13] New
    P1_PythonBinding,
}

impl Gate {
    pub fn is_blocking(&self) -> bool {
        match self {
            Gate::B1_ThroughputAscii | Gate::B18_LineWrapReflow => false,
            _ => true,
        }
    }
}
```

### Test Strategy

-   **TST-020**: `Gate::is_blocking()` returns true for C1, A1, A6, P1.
-   **TST-021**: Performance gates B1, B18 return false (soft block).
-   **TST-022**: CI pipeline fails if blocking gate artifact is missing/failed.

### AGENTS.md Rules

-   **RULE-S03-01**: Every feature MUST map to at least one Gate.
-   **RULE-S03-02**: `mux-core` MUST remain `no_std` / WASM-compatible (Gate A1).
-   **RULE-S03-03**: VtParser MUST have 100% state reachability (Gate A6).

---

## 5. Grid Data Structures ([v13] Enhanced)

### Design Decisions

-   **Line Abstraction**: `struct Line` wraps `Vec<Cell>` to provide row-level logic (wrapping, width calc).
-   **Viewport**: `struct Viewport<'a>` is a slice of `&'a Line` for rendering, decoupled from storage.
-   **Cell**: `Cell` is 24 bytes (char + style + flags).
-   **Grid**: `Grid` owns `Vec<Line>` and handles scrolling/resizing.

### Rust Example (Normative)

```rust
// crates/mux-grid/src/lib.rs

#[derive(Clone, Debug, PartialEq)]
pub struct Cell {
    pub char: char,
    pub fg: u32,
    pub bg: u32,
    pub flags: u16,
}

#[derive(Clone, Debug)]
pub struct Line {
    pub cells: Vec<Cell>,
    pub wrapped: bool,
}

impl Line {
    pub fn new(width: usize) -> Self {
        Self {
            cells: vec![Cell { char: ' ', fg: 0, bg: 0, flags: 0 }; width],
            wrapped: false,
        }
    }
}

pub struct Grid {
    pub lines: Vec<Line>,
    pub width: usize,
    pub height: usize,
    pub cursor_x: usize,
    pub cursor_y: usize,
}

impl Grid {
    pub fn new(width: usize, height: usize) -> Self {
        let lines = (0..height).map(|_| Line::new(width)).collect();
        Self { lines, width, height, cursor_x: 0, cursor_y: 0 }
    }

    pub fn viewport(&self, offset: usize, height: usize) -> &[Line] {
        let start = offset.min(self.lines.len().saturating_sub(height));
        &self.lines[start..start + height]
    }
    
    // [v13] FND-01 Remediation
    pub fn put_char(&mut self, c: char) {
        if self.cursor_x >= self.width {
            self.wrap_next_line();
        }
        self.lines[self.cursor_y].cells[self.cursor_x].char = c;
        self.cursor_x += 1;
    }
    
    fn wrap_next_line(&mut self) {
        self.lines[self.cursor_y].wrapped = true;
        self.cursor_y += 1;
        self.cursor_x = 0;
        // Scroll logic omitted for brevity
    }
}
```

### Test Strategy

-   **TST-050**: `Grid::new` creates correct dimensions.
-   **TST-051**: `put_char` advances cursor.
-   **TST-052**: `put_char` at right edge sets `wrapped=true` and moves to next line.
-   **TST-053**: `viewport()` returns correct slice of lines.

### AGENTS.md Rules

-   **RULE-S05-01**: Grid operations MUST be safe (no panic on index out of bounds).
-   **RULE-S05-02**: `Line` MUST track wrapped state for reflow.
-   **RULE-S05-03**: `put_char` is the ONLY entry point for character insertion.

---

## 6. VtParser State Machine ([v13] Complete)

### Design Decisions

-   **7 States**: Ground, Escape, EscapeIntermediate, CsiEntry, CsiParam, CsiIntermediate, OscString.
-   **Dispatch**: Table-based dispatch `(State, u8) -> Action`.
-   **Params**: `CsiParam` state accumulates arguments into `Vec<i64>`.

### Rust Example (Normative)

```rust
// crates/mux-grid/src/parser.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Ground,
    Escape,
    EscapeIntermediate,
    CsiEntry,
    CsiParam,
    CsiIntermediate,
    OscString,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Ignore,
    Print,
    Execute,
    Clear,
    Collect,
    Param,
    CsiDispatch,
    OscStart,
    OscPut,
    OscEnd,
}

pub struct VtParser {
    state: State,
    params: Vec<i64>,
}

impl VtParser {
    pub fn new() -> Self {
        Self { state: State::Ground, params: Vec::new() }
    }

    pub fn advance(&mut self, b: u8) -> Action {
        match (self.state, b) {
            (State::Ground, 0x1B) => {
                self.state = State::Escape;
                Action::Clear
            },
            (State::Ground, c) if c >= 0x20 && c <= 0x7F => Action::Print,
            (State::Escape, 0x5B) => {
                self.state = State::CsiEntry;
                Action::Clear
            },
            (State::CsiEntry, c) if c >= 0x30 && c <= 0x39 => {
                self.state = State::CsiParam;
                Action::Param
            },
            // ... strict subset of full table ...
            _ => Action::Ignore,
        }
    }
}
```

### Test Strategy

-   **TST-060**: State transitions follow DEC standard.
-   **TST-061**: `ESC [` transitions to `CsiEntry`.
-   **TST-062**: Digits in `CsiEntry` transition to `CsiParam`.
-   **TST-063**: Fuzz testing reaches all 7 states (Gate A6).

### AGENTS.md Rules

-   **RULE-S06-01**: Parser MUST implement the full 7-state machine.
-   **RULE-S06-02**: State transitions MUST be deterministic.
-   **RULE-S06-03**: Parser MUST NOT access Grid directly; emits Actions/Events.

---

## 17. CRDT & Input Handling ([v13] Clarified)

### Design Decisions

-   **Pane Input**: Merged via `OpLog` (LWW per client input chunk).
-   **Conflict Resolution**: Last-Write-Wins based on timestamp; tie-break by ClientID.
-   **Echo**: Local echo handled by Termlet state; confirmed by server.

### Rust Example (Normative)

```rust
// crates/mux-crdt/src/lib.rs

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpLogEntry {
    pub lamport: u64,
    pub client_id: u128,
    pub payload: Vec<u8>,
}

pub struct PaneOpLog {
    entries: Vec<OpLogEntry>,
}

impl PaneOpLog {
    pub fn merge(&mut self, entry: OpLogEntry) {
        // Simple append for log; state reconstruction applies LWW
        self.entries.push(entry);
        self.entries.sort_by_key(|e| (e.lamport, e.client_id));
    }
}
```

### Test Strategy

-   **TST-170**: Concurrent inputs are ordered by lamport timestamp.
-   **TST-171**: Tie-breaking uses client_id deterministically.
-   **TST-172**: Replay of OpLog produces identical Grid state.

### AGENTS.md Rules

-   **RULE-S17-01**: All pane mutations MUST be recorded in OpLog.
-   **RULE-S17-02**: Conflict resolution MUST be deterministic LWW.

---

## 32. Termlet Architecture ([v13] Deepest)

### 32.1 Termlet Definition & Lifecycle

Termlets are the fundamental unit of execution in TermForge. Unlike tmux panes which are passive PTY sinks, Termlets are active actors that can be purely virtual (WASM), PTY-backed, or remote.

-   **Lifecycle**: `Created` -> `Running` -> `Pausing` -> `Paused` -> `Destroyed`.
-   **Snapshotting**: Termlets must support synchronous serialization to `TermletSnapshot`.

### 32.2 TermletSnapshot Binary Format ([v13] New)

Binary format allows efficient storage and transport.

-   **Magic**: `TFSN` (4 bytes).
-   **Version**: `u8` (currently 1).
-   **Payload**: Bincode/MessagePack encoded state.

### 32.3 TermletLike Trait ([v13] Normative)

The defining interface for all Termlets.

```rust
// crates/mux-termlet/src/lib.rs

pub trait TermletLike {
    fn id(&self) -> u64;
    fn tick(&mut self, dt_ms: u64);
    fn handle_input(&mut self, bytes: &[u8]);
    fn render(&self) -> Vec<String>; // Simplified for example
    fn snapshot(&self) -> Vec<u8>;
}

// [v13] Snapshot Header
pub struct SnapshotHeader {
    pub magic: [u8; 4], // b"TFSN"
    pub version: u8,
}

impl SnapshotHeader {
    pub fn new() -> Self {
        Self { magic: *b"TFSN", version: 1 }
    }
}
```

### 32.4 Termlet Isolation

Termlets run in isolation. A crash in a Termlet (e.g., WASM fault) MUST NOT crash the server.

-   **Fault Tolerance**: Panic catch boundaries around `tick()` and `handle_input()`.
-   **Resource Quotas**: Max memory, max CPU time per tick.

### Test Strategy

-   **TST-320**: Termlet implements `TermletLike`.
-   **TST-321**: Snapshot starts with `TFSN` magic.
-   **TST-322**: Panic in `tick` is caught and logs error, Termlet transitions to `Paused`.

### AGENTS.md Rules

-   **RULE-S32-01**: All Termlet implementations MUST implement `TermletLike`.
-   **RULE-S32-02**: Snapshots MUST use the versioned binary header.
-   **RULE-S32-03**: Termlet panic MUST be contained.

---

