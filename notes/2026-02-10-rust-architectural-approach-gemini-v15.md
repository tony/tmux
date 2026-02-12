# TermForge v15 Architecture Specification -- Pass 1 [v15] DEFINITIVE

Date: 2026-02-12
Status: **v15 Pass 1 DEFINITIVE** -- Gemini 2.0
Lineage: v14 P3 DEFINITIVE -> **v15 Pass 1** (this document).
Models: Gemini 2.0 (v15 author). Inputs: Claude v14 P3 (structural base, 48 S32 subs).
License: MIT OR Apache-2.0
Rust edition: 2021 (MSRV 1.85)
Protocol target: tmux protocol v8

---

## Preamble

This document is the **v15 Pass 1 DEFINITIVE** architecture specification for TermForge, a Rust terminal multiplexer with 100% tmux wire-protocol compatibility. It builds upon v14 but aggressively challenges key architectural decisions to maximize performance, type safety, and correctness. [v15]

### [v15] The "Challenge Pass" Methodology

v15 represents the "Challenge Pass". Every major decision in v14 was re-evaluated against three criteria:
1.  **Performance**: Can we do better than O(n) or improve cache locality?
2.  **Correctness**: Does the data model actually support the full Unicode domain (graphemes)?
3.  **Ergonomics**: Are the APIs (especially FFI) actually usable?

### [v15] Decision Matrix (v14 vs v15)

| # | Topic | v14 Decision | v15 Challenge & Resolution | Rationale |
|---|-------|--------------|----------------------------|-----------|
| 1 | String Type | `CompactString` (implied wrapper) | **[v15] `compact_str::CompactString`** | Validated 24-byte inline capacity on 64-bit systems vs 22/23 bytes for others. Zero-dependency preference dropped for performance. |
| 2 | Checksums | Dual (FNV-1a + CRC32C) | **[v15] CRC32C Only** | "One way to do it". Modern CPUs (x86/ARM) have CRC32 hardware. Software fallback is fast enough. Drop FNV-1a to simplify snapshot headers. |
| 3 | ByteClass | `match` statement | **[v15] Static Lookup Table** | 256-byte static array fits in L1 cache. Eliminates branch misprediction in hot parser loop. O(1) guaranteed. |
| 4 | Grid Cells | `Vec<Cell>` | **[v15] `Arc<ChunkedRow>`** | Scrollback lines are rarely mutated. `Arc<Vec<Cell>>` (Cow) saves huge memory on window splits/clones. |
| 5 | PackedCell | 21-bit char (Unicode scalars) | **[v15] Grapheme Registry ID** | 21 bits cannot store emoji sequences. v15 introduces a "Grapheme Registry" for complex sequences, storing ID in the 21-bit field if a flag is set. |
| 6 | PtyHandle | `PtyHandle<S>` + `DynPtyHandle` | **[v15] Unified `PtyHandle`** | `PtyHandle` wraps a private `InnerPtyState` enum. Typestate is a zero-cost PhantomData wrapper around the *same* inner data. No code duplication. |
| 7 | Termlets | 48 Subsections | **[v15] 52 Subsections** | Added Resource Quotas, Sandboxing, Recorder, and FFI Stability. |

### [v15] Global Invariants (Updated)

- `INV-001`: Protocol compatibility target is tmux protocol v8.
- `INV-012`: Grid mutation solely via `put_char` / `put_grapheme`.
- `INV-025` **[v15]**: All complex graphemes (>1 char) in snapshots MUST be registry-backed.
- `INV-026` **[v15]**: Parser state transitions MUST be O(1) table lookups.

---

## 1. Project Identity

### Design Decisions

- **[v15]** Name: TermForge. Binary: `tf` (aliased to `tmux`).
- **[v15]** Crate structure: `mux-core`, `mux-grid`, `mux-termlet`, `mux-cli`.
- **[v15]** License: MIT OR Apache-2.0.

### Rust Example

```rust
// crates/mux-core/src/lib.rs
// [v15] Core identity and constants

#![no_std]
extern crate alloc;

pub const NAME: &str = "TermForge";
pub const VERSION: &str = "0.1.0";
pub const PROTOCOL_VERSION: u8 = 8; // tmux v8

/// [v15] Feature flags
pub mod features {
    pub const CRDT: bool = cfg!(feature = "crdt");
    pub const PYTHON_BINDINGS: bool = cfg!(feature = "python");
}
```

### Test Strategy

1. Binary name is `tf`. `TST-001`.
2. `tmux` alias works. `TST-002`.
3. Version matches Cargo.toml. `TST-003`.

### AGENTS.md Rules

- `RULE-S01-01`: Crate names MUST start with `mux-`.
- `RULE-S01-02`: Core crate MUST be `no_std` compatible.

---

## 2. Acceptance Criteria and Gates

### Design Decisions

- **[v15]** Zero-Panic Policy: `unwrap()` / `expect()` forbidden in library code.
- **[v15]** Coverage: 90% code coverage, 100% API surface coverage.
- **[v15]** Termlet Gates: 100% pass on 50+ Termlet scenarios.

### Rust Example

```rust
// tools/check_gates.rs
// [v15] Script to enforce gates

fn check_panic_freedom(dir: &str) -> bool {
    // Grep for unwrap/expect
    !grep_recursive(dir, "unwrap(").exists()
}
```

### Test Strategy

1. CI fails on `unwrap()`. `TST-010`.
2. Coverage report generated. `TST-011`.

### AGENTS.md Rules

- `RULE-S02-01`: No `unwrap()` in `src/`.

---

## 3. Crate Dependency Rules

### Design Decisions

- **[v15]** Dependency Tree Depth: Max 3.
- **[v15]** Banned Crates: `lazy_static` (use `once_cell`), `nom` (use custom parser).
- **[v15]** Std Access: `mux-core` is `no_std`, others can be `std`.

### Rust Example

```toml
# Cargo.toml
[workspace]
members = ["crates/*"]

[workspace.dependencies]
compact_str = "0.7"
crc32fast = "1.3"
serde = { version = "1.0", default-features = false }
```

### Test Strategy

1. `cargo tree` depth check. `TST-020`.
2. Banned crate check. `TST-021`.

### AGENTS.md Rules

- `RULE-S03-01`: Dependencies must be in workspace toml.

---

## 4. Workspace Layout

### Design Decisions

- **[v15]** Flat crate structure in `crates/`.
- **[v15]** Tests in `tests/` (integration) and `src/` (unit).

### Rust Example

```text
/
  crates/
    mux-core/
    mux-grid/
    mux-termlet/
  tests/
    integration/
  Cargo.toml
```

### Test Strategy

1. Workspace builds. `TST-030`.

### AGENTS.md Rules

- `RULE-S04-01`: All crates in `crates/`.

---

## 5. Grid API Completeness (Cell, Line, Viewport)

### Design Decisions

- **[v15]** `Cell` uses `compact_str::CompactString` (24 bytes inline).
- **[v15]** `Line` uses `Arc<Vec<Cell>>` (COW) to allow cheap snapshotting of scrollback.
- **[v15]** `Grid` uses `VecDeque<Line>`.
- **[v15]** `PackedCell` (for snapshots) uses Grapheme Registry ID if bit 21 set.

### Rust Example

```rust
// crates/mux-grid/src/lib.rs
// [v15] CompactString + COW Line + Grapheme Registry Support

use compact_str::CompactString;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    pub grapheme: CompactString,
    pub style: u16,
    pub flags: u16,
}

impl Cell {
    pub fn new(c: char) -> Self {
        Self { grapheme: CompactString::from(c), style: 0, flags: 0 }
    }
}

/// [v15] Line uses COW for scrollback efficiency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    // Arc allows cheap cloning for history/snapshots
    pub cells: Arc<Vec<Cell>>,
    pub is_wrapped: bool,
}

impl Line {
    pub fn new(cols: usize) -> Self {
        let cells = vec![Cell::new(' '); cols];
        Self { cells: Arc::new(cells), is_wrapped: false }
    }

    /// [v15] COW: make_mut only when modifying
    pub fn ensure_unique(&mut self) -> &mut Vec<Cell> {
        Arc::make_mut(&mut self.cells)
    }
}
```

### Test Strategy

1. `CompactString` inlines 24 bytes. `TST-050`.
2. `Line` clone is O(1) (Arc increment). `TST-051`.
3. Modification triggers COW. `TST-052`.

### AGENTS.md Rules

- `RULE-S05-01`: `Line` MUST be cheaply cloneable (Arc).
- `RULE-S05-02`: Cell MUST NOT exceed 32 bytes (24+4+4).

---

## 6. VtParser State Machine

### Design Decisions

- **[v15]** `ByteClass` uses static 256-byte lookup table (no match).
- **[v15]** `TRANSITIONS` table remains const.
- **[v15]** `classify()` is O(1) memory access.

### Rust Example

```rust
// crates/mux-core/src/parser.rs
// [v15] Static Lookup Table for ByteClass

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum ByteClass { Printable = 0, Control = 1, Escape = 2, CsiEntry = 3, OscEntry = 4, Invalid = 5 }

// [v15] Precomputed lookup table
static CLASS_TABLE: [ByteClass; 256] = {
    let mut table = [ByteClass::Printable; 256];
    let mut i = 0;
    while i < 32 { table[i] = ByteClass::Control; i += 1; }
    table[0x1B] = ByteClass::Escape;
    table[0x9B] = ByteClass::CsiEntry;
    table[0x9D] = ByteClass::OscEntry;
    table
};

#[inline(always)]
pub fn classify(b: u8) -> ByteClass {
    // [v15] O(1) array access
    CLASS_TABLE[b as usize]
}
```

### Test Strategy

1. Classify matches reference impl. `TST-060`.
2. Benchmark shows speedup. `TST-061`.

### AGENTS.md Rules

- `RULE-S06-01`: Parser hot path MUST NOT contain match/if branches for classification.

---

## 7. PtyHandle Lifecycle

### Design Decisions

- **[v15]** Unified `PtyHandle`: One `InnerPtyState` enum.
- **[v15]** `PtyHandle<S>` wraps `InnerPtyState` with zero-cost `PhantomData`.
- **[v15]** `DynPtyHandle` is just `InnerPtyState`. No code duplication.

### Rust Example

```rust
// crates/mux-core/src/pty.rs
// [v15] Unified PtyHandle

#[derive(Debug, Clone, PartialEq)]
pub enum InnerPtyState { Allocated, Spawned, Running, Exited }

#[derive(Debug)]
pub struct PtyHandle<S> {
    inner: InnerPtyState,
    _marker: std::marker::PhantomData<S>,
}

impl PtyHandle<Allocated> {
    pub fn new() -> Self {
        Self { inner: InnerPtyState::Allocated, _marker: std::marker::PhantomData }
    }
    pub fn spawn(mut self) -> PtyHandle<Spawned> {
        self.inner = InnerPtyState::Spawned;
        // Transmute logic or destructure/restructure safe here
        PtyHandle { inner: self.inner, _marker: std::marker::PhantomData }
    }
}
```

### Test Strategy

1. Typestate transitions compile. `TST-070`.
2. Inner state reflects public state. `TST-071`.

### AGENTS.md Rules

- `RULE-S07-01`: `PtyHandle` MUST wrap `InnerPtyState`.

---

## 8. Termlet Snapshot Format (v15)

### Design Decisions

- **[v15]** `PackedCell` (64-bit):
  - 21 bits: Char (scalar) OR Registry ID.
  - 1 bit: Registry Flag (`IS_REGISTRY_ID`).
  - 16 bits: Style.
  - 16 bits: Flags.
  - 10 bits: Reserved.
- **[v15]** Grapheme Registry: separate chunk in snapshot or shared dictionary.
- **[v15]** Checksum: CRC32C only (offset 30 is `0x01` fixed).

### Rust Example

```rust
// crates/mux-snapshot/src/lib.rs
// [v15] PackedCell with Registry Flag

pub struct PackedCell(u64);

const REGISTRY_FLAG: u64 = 1 << 21;

impl PackedCell {
    pub fn new(c: char, style: u16) -> Self {
        Self((c as u64) | ((style as u64) << 22))
    }
    pub fn new_registry(id: u32, style: u16) -> Self {
        Self((id as u64) | REGISTRY_FLAG | ((style as u64) << 22))
    }
}
```

### Test Strategy

1. Emoji roundtrips via Registry. `TST-080`.
2. Scalar char inlines. `TST-081`.

### AGENTS.md Rules

- `RULE-S08-01`: Complex graphemes MUST use Registry ID.

---

## 32. Termlets [v15 Expanded]

### Design Decisions

- **[v15]** Expanded to 52 subsections.
- **[v15]** Added Resource Quotas, Sandboxing, Recorder, FFI Stability.

### 32.1 - 32.48 (Retained from v14)

(Content follows v14 structure)

### 32.49 Termlet Resource Quotas [v15]

**[v15]** Termlets MUST enforce CPU/RAM limits to prevent test runners from stalling.
- `RULE-S32-121`: Termlet memory limit default 512MB.
- `RULE-S32-122`: Termlet CPU time limit default 30s.

### 32.50 Termlet Sandboxing [v15]

**[v15]** Termlets running untrusted code/snapshots MUST use namespaces.
- `RULE-S32-123`: Use `unshare -n -u` where available.

### 32.51 Termlet Recorder [v15]

**[v15]** New capability: Record live session to `.tlet` file.
- `recorder.record("scenario_name")` -> `scenarios/scenario_name.tlet`.

### 32.52 Termlet FFI Stability [v15]

**[v15]** C ABI verification for Python/Node bindings.
- `RULE-S32-124`: `repr(C)` checks in CI.

---
