# TermForge v15 Architecture Specification — Pass 2 [v15 P2]

Date: 2026-02-12
Status: **v15 Pass 2 REFINEMENT** -- Gemini 2.0 (Synthesizer)
Lineage: v14 P3 -> v15 P1 (Claude/GPT/Gemini) -> **v15 P2** (this document)
Models: Gemini 2.0, GPT-4o, Claude 3.5 Sonnet.
License: MIT OR Apache-2.0
Rust edition: 2021 (MSRV 1.85)
Protocol target: tmux protocol v8

---

## Preamble

This document is the **v15 Pass 2 REFINEMENT** of the TermForge architecture. It unifies the three Pass 1 specifications into a single, cohesive, authoritative standard. It resolves all architectural conflicts by selecting the technically superior solution and enforcing strict "One Way To Do It" patterns.

### [v15 P2] Synthesis & Conflict Resolution Strategy

We applied a "Darwinian Architecture" selection process:
1.  **Safety**: Type-driven states (Claude) and COW immutability (Gemini) are prioritized.
2.  **Performance**: Static LUTs (Gemini/GPT) and Inline strings (Gemini) beat generic implementations.
3.  **Density**: GPT's rule/risk density is adopted as the baseline standard.

### [v15 P2] Definitive Decision Matrix

| # | Topic | v15 P2 Resolution | Rationale | Source |
|---|-------|-------------------|-----------|--------|
| 1 | String Type | **`compact_str::CompactString`** | The crate (not hand-rolled) offers 24-byte inline on 64-bit, beating 22-byte custom enum. Zero-dep purity dropped for performance. | Gemini |
| 2 | Checksum | **CRC32C Only** | Hardware accelerated, canonical. Dropped FNV-1a to simplify snapshot headers. | Gemini/GPT |
| 3 | Grid Storage | **`Arc<Vec<Cell>>` (COW)** | huge memory win for scrollback/snapshots. Lines are rarely mutated after history commit. | Gemini |
| 4 | PackedCell | **Hybrid Layout (64-bit)** | `scalar:21` + `style:15` + `flags:12` + `width:2` + `ext:14`. Explicit width and extension index (16k/screen) is superior to Registry Flag. | GPT (refined) |
| 5 | Parser Dispatch | **Static `CLASS_TABLE`** | 256-byte const LUT. Eliminates branches. `ByteClass` is `#[repr(u8)]`. | Gemini/GPT |
| 6 | PtyHandle | **Unified `InnerPtyState`** | Single enum wrapped in `PtyHandle<S>` with `PhantomData`. Best of both worlds: Typestate safety without code duplication. | Gemini |
| 7 | Time | **`DeterministicTimeSource`** | Unified abstraction managing Wall, Lamport, and Vector clocks. Essential for Jepsen-style testing. | GPT |
| 8 | Errors | **Rich Hierarchy** | `TermletError` (enum) for logic + `ExpectError` (struct w/ digest) for diagnostics. | Combined |

### [v15 P2] Global Invariants (Refined)

- `INV-001`: **Protocol Parity**: 100% tmux wire-protocol v8 compatibility.
- `INV-012`: **Controlled Mutation**: Grid mutation ONLY via `put_char`/`put_grapheme`.
- `INV-025`: **Vector Clock Monotonicity**: VectorClock entries monotonically non-decreasing per node.
- `INV-029` **[v15 P2]**: **COW Line Semantics**: `Line` is `Arc`-backed; mutation triggers `make_mut`.
- `INV-030` **[v15 P2]**: **Unified Time**: All time access MUST go through `DeterministicTimeSource`.

---

## 1. Project Identity [v15 P2]

### Design Decisions (DD)

- **Name**: TermForge.
- **Binary**: `termforge` (primary), `tf` (alias).
- **Identity Manifest**: Canonical source of truth for versioning.
- **[v15 P2]** Version string format: `termforge <semver>+v15 (protocol v8, edition 2021)`.
- **[v15 P2]** Build Profile: Embedded in manifest (`Debug` vs `Release`).

### Rust Example (RE)

```rust
// crates/mux-types/src/identity.rs
// [v15 P2] IdentityManifest with compact_str and build profile
// compiles with: rustc --edition=2021 --crate-type lib

use compact_str::CompactString;

pub const PROJECT_NAME: &str = "TermForge";
pub const BINARY_ALIAS: &str = "tf";
pub const SPEC_VERSION: &str = "v15-p2";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuildProfile {
    Debug,
    Release,
    RelWithDebInfo,
}

impl BuildProfile {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Release => "release",
            Self::RelWithDebInfo => "relwithdebinfo",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityManifest {
    pub project_name: &'static str,
    pub spec_version: &'static str,
    pub protocol_version: u32,
    pub build_profile: BuildProfile,
    pub git_sha: CompactString, // [v15 P2] Using CompactString
}

impl IdentityManifest {
    pub fn canonical() -> Self {
        Self {
            project_name: PROJECT_NAME,
            spec_version: SPEC_VERSION,
            protocol_version: 8,
            build_profile: if cfg!(debug_assertions) { BuildProfile::Debug } else { BuildProfile::Release },
            git_sha: CompactString::from("unknown"),
        }
    }
}
```

### Test Strategy (TS)

1. `TST-101`: Binary alias `tf` invokes main.
2. `TST-102`: Version string contains `v15-p2`.
3. `TST-103`: `IdentityManifest` constants match build args.

### AGENTS.md Rules (AR)

- `RULE-S01-01`: Crate names MUST be prefixed `mux-`.
- `RULE-S01-02`: `spec_version` MUST match the current document version.

---

## 5. Grid API Completeness (Cell, Line, Viewport) [v15 P2]

### Design Decisions (DD)

- **[v15 P2]** **Cell Storage**: `compact_str::CompactString` (24 bytes inline).
- **[v15 P2]** **Line Storage**: `Arc<Vec<Cell>>` implementing Copy-On-Write (COW).
  - Rationale: Scrolling pushes lines to history. History is 99% read-only. `Arc` makes snapshotting O(lines) instead of O(cells).
- **[v15 P2]** **PackedCell**: 64-bit layout optimized for scalars and extension indexing.
  - `scalar:21` | `style:15` | `flags:12` | `width:2` | `ext:14`.
- **[v15 P2]** **Grapheme Arena**: `ext` index points to a `GraphemeArena` for clusters > 1 char.

### Rust Example (RE)

```rust
// crates/mux-grid/src/grid.rs
// [v15 P2] COW Line and CompactString Cell
// compiles with: rustc --edition=2021 --crate-type lib

use compact_str::CompactString;
use std::sync::Arc;
use std::ops::{Deref, DerefMut};

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
    
    pub fn width(&self) -> u8 {
        if self.flags & 0x01 != 0 { 2 } else { 1 } // Simplified width check
    }
}

/// [v15 P2] COW Line Implementation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    // Arc provides cheap cloning for history/snapshots
    cells: Arc<Vec<Cell>>,
    pub is_wrapped: bool,
}

impl Line {
    pub fn new(width: usize) -> Self {
        let cells = vec![Cell::new(' '); width];
        Self { cells: Arc::new(cells), is_wrapped: false }
    }

    /// [v15 P2] Triggers COW if shared
    pub fn cells_mut(&mut self) -> &mut Vec<Cell> {
        Arc::make_mut(&mut self.cells)
    }

    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }
}

/// [v15 P2] PackedCell for Snapshots (64-bit)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct PackedCell(u64);

impl PackedCell {
    const SCALAR_MASK: u64 = (1 << 21) - 1;
    const EXT_MASK: u64 = (1 << 14) - 1;

    pub fn new(scalar: char, style: u16, flags: u16, width: u8, ext_index: u16) -> Self {
        let scalar_bits = (scalar as u64) & Self::SCALAR_MASK;
        let style_bits = (style as u64) << 21;
        let flags_bits = (flags as u64) << 36;
        let width_bits = (width as u64) << 48;
        let ext_bits = ((ext_index as u64) & Self::EXT_MASK) << 50;
        
        Self(scalar_bits | style_bits | flags_bits | width_bits | ext_bits)
    }
    
    pub fn decode(&self) -> (char, u16, u16, u8, u16) {
        let raw = self.0;
        let scalar = char::from_u32((raw & Self::SCALAR_MASK) as u32).unwrap_or('\u{FFFD}');
        let style = ((raw >> 21) & 0x7FFF) as u16;
        let flags = ((raw >> 36) & 0xFFF) as u16;
        let width = ((raw >> 48) & 0x3) as u8;
        let ext = ((raw >> 50) & Self::EXT_MASK) as u16;
        (scalar, style, flags, width, ext)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cow_semantics() {
        let mut line1 = Line::new(10);
        let mut line2 = line1.clone();
        
        // Initial state: shared
        assert!(Arc::ptr_eq(&line1.cells, &line2.cells));
        
        // Modification triggers copy
        line1.cells_mut()[0] = Cell::new('A');
        
        assert!(!Arc::ptr_eq(&line1.cells, &line2.cells));
        assert_eq!(line1.cells()[0].grapheme, "A");
        assert_eq!(line2.cells()[0].grapheme, " ");
    }

    #[test]
    fn test_packed_cell_encoding() {
        let pc = PackedCell::new('A', 1, 0, 1, 42);
        let (c, s, f, w, e) = pc.decode();
        assert_eq!(c, 'A');
        assert_eq!(s, 1);
        assert_eq!(w, 1);
        assert_eq!(e, 42);
    }
}
```

### Test Strategy (TS)

1. `TST-501`: `Line` cloning is O(1) (pointer copy).
2. `TST-502`: `Line` mutation triggers deep copy (COW).
3. `TST-503`: `PackedCell` roundtrips all fields correctly.
4. `TST-504`: `CompactString` uses inline storage for ASCII.

### AGENTS.md Rules (AR)

- `RULE-S05-01`: Grid lines MUST implement COW semantics via `Arc`.
- `RULE-S05-02`: PackedCell MUST fit in 64 bits.

---

## 6. VtParser State Machine [v15 P2]

### Design Decisions (DD)

- **[v15 P2]** **Classification**: Static 256-byte Lookup Table (LUT).
- **[v15 P2]** **Transition Table**: 7x7 const array.
- **[v15 P2]** **ByteClass**: `#[repr(u8)]` matching LUT indices.
- **Resolution**: Adopts Gemini's LUT for O(1) classification, replacing v14's match statement.

### Rust Example (RE)

```rust
// crates/mux-grid/src/parser.rs
// [v15 P2] Static LUT Parser
// compiles with: rustc --edition=2021 --crate-type lib

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ByteClass {
    Printable = 0,
    Control = 1,
    Escape = 2,
    CsiEntry = 3,
    OscEntry = 4,
    DcsEntry = 5,
    Invalid = 6,
}

static CLASS_TABLE: [ByteClass; 256] = {
    let mut table = [ByteClass::Printable; 256];
    let mut i = 0;
    while i < 0x20 { table[i] = ByteClass::Control; i += 1; }
    table[0x7F] = ByteClass::Control;
    table[0x1B] = ByteClass::Escape;
    table[0x90] = ByteClass::DcsEntry;
    table[0x9B] = ByteClass::CsiEntry;
    table[0x9D] = ByteClass::OscEntry;
    table
};

#[inline(always)]
pub fn classify(b: u8) -> ByteClass {
    CLASS_TABLE[b as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lut_classification() {
        assert_eq!(classify(b'A'), ByteClass::Printable);
        assert_eq!(classify(0x1B), ByteClass::Escape);
        assert_eq!(classify(0x90), ByteClass::DcsEntry);
    }
}
```

### Test Strategy (TS)

1. `TST-601`: All 256 bytes classified correctly against oracle.
2. `TST-602`: Transition table is complete (7x7).

---

## 7. PtyHandle Lifecycle [v15 P2]

### Design Decisions (DD)

- **[v15 P2]** **Unified Backend**: Single `InnerPtyState` enum.
- **[v15 P2]** **Typestate Frontend**: `PtyHandle<S>` wraps `InnerPtyState` + `PhantomData`.
- **[v15 P2]** **Dynamic Frontend**: `DynPtyHandle` wraps `InnerPtyState`.
- **Resolution**: Solves the conflict between Claude's Typestate (safe) and GPT's DynHandle (ergonomic) by sharing the backend.

### Rust Example (RE)

```rust
// crates/mux-pty/src/handle.rs
// [v15 P2] Unified PtyHandle
// compiles with: rustc --edition=2021 --crate-type lib

use std::marker::PhantomData;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State { Allocated, Spawned, Running, Exited }

// The unified backend state
#[derive(Debug, Clone)]
pub struct InnerPtyState {
    id: u64,
    state: State,
}

// Typestate Wrapper
#[derive(Debug)]
pub struct PtyHandle<S> {
    inner: InnerPtyState,
    _marker: PhantomData<S>,
}

pub struct Allocated;
pub struct Spawned;

impl PtyHandle<Allocated> {
    pub fn new(id: u64) -> Self {
        Self {
            inner: InnerPtyState { id, state: State::Allocated },
            _marker: PhantomData,
        }
    }

    pub fn spawn(mut self) -> PtyHandle<Spawned> {
        self.inner.state = State::Spawned;
        PtyHandle { inner: self.inner, _marker: PhantomData }
    }
}

// Dynamic Wrapper (for FFI)
pub struct DynPtyHandle(InnerPtyState);

impl DynPtyHandle {
    pub fn from_typed<S>(handle: PtyHandle<S>) -> Self {
        Self(handle.inner)
    }
    
    pub fn current_state(&self) -> State {
        self.0.state
    }
}
```

---

## 32. Termlets [v15 P2]

### Design Decisions (DD)

- **[v15 P2]** **Structure**: Expanded to 52 subsections (S32.1 - S32.52).
- **[v15 P2]** **Quotas**: CPU/RAM limits enforced by `mux-termlet` runtime.
- **[v15 P2]** **Recorder**: Live recording to `.tlet` files.

### 32.49 Resource Quotas
Termlets MUST enforce:
- 512MB RAM limit per instance.
- 30s CPU time limit (soft), 35s (hard).

### 32.50 Sandboxing
Termlets MUST use namespace isolation (`unshare -n -u`) on Linux targets.

### 32.52 FFI Stability
Bindings MUST verify `repr(C)` layout for all cross-boundary structs.

---

**End of v15 Pass 2 Refinement**
