# TermForge v14 Architecture Specification — Pass 2 [v14 P2]

**Date:** 2026-02-12
**Status:** **DRAFT** — v14 Pass 2 (Cross-Pollination Refinement).
**Lineage:** v13 P3 -> v14 P1 (Claude/GPT/Gemini) -> **v14 Pass 2 (Gemini)**.
**Models:** Gemini 2.0 Flash (v14 P2 Author), incorporating Claude Opus (Base) & GPT-5 (Deepening).
**License:** MIT OR Apache-2.0
**Rust edition:** 2021 (MSRV 1.85)

---

## Preamble [v14 P2]

This document is the **v14 Pass 2** architecture specification for TermForge. It synthesizes the "Pass 1" triad:
1.  **Claude P1 (Base):** Provided the 32-section structure, `ByteClass` enum, `PackedCell(u64)`, and `Typestate` patterns.
2.  **GPT P1 (Deepening):** Contributed Termlet subsections S32.36–45 and the extensive `TST-` and `RULE-` catalog.
3.  **Gemini P1 (Optimization):** Introduced `SmallVec` for lines, `CRC32C` for snapshots, `TermForgeIdentity` trait, and distributed Termlet scenarios (S32.11/12).

### [v14 P2] Cross-Model Decision Table

| # | Feature | v13 / P1 Base | v14 P2 Resolution | Rationale |
|---|---|---|---|---|
| 1 | **Grid Storage** | `Vec<Cell>` (v13) or `VecDeque` (Claude) | **[v14 P2] `VecDeque<Line>`** where `Line` holds `SmallVec<[Cell; 132]>` | Optimize for standard 80-120 col terminals; keep scrollback efficient. |
| 2 | **Cell Grapheme** | `String` (v13) or `ByteClass` (Claude) | **[v14 P2] `CompactString`** | Inline short graphemes (<24 bytes) to kill allocator traffic. |
| 3 | **Identity** | `struct Identity` (v13) | **[v14 P2] `trait TermForgeIdentity`** | Compile-time polymorphism for builds (LTS vs Preview). |
| 4 | **Snapshot Hash** | `FNV-1a` (Claude P1) | **[v14 P2] `CRC32C`** | HW-accelerated integrity checks (SSE4.2) are much faster. |
| 5 | **Termlet Net** | N/A | **[v14 P2] Network Partition Sim** | Critical for distributed Termlet testing (S32.11). |
| 6 | **Termlet Scope** | 35 Subsections (Claude) | **[v14 P2] 45 Subsections** | Adopt GPT P1's S32.36–45 (Fuzzing, Replay, Governance). |
| 7 | **OpLog** | `push` (v13) | **[v14 P2] Binary Insertion** | Keep OpLog sorted O(log n) for efficient range queries. |

---

## 1. Project Identity

### Design Decisions

-   **[v14 P2] Identity Trait:** We adopt the Gemini P1 `TermForgeIdentity` trait to allow distinct build profiles (LTS, Preview) to be sealed at compile time.
-   **[v14 P2] Versioning:** Includes git hash + protocol version.

### Rust Example (RE-01)

```rust
// crates/mux-types/src/identity.rs
// [v14 P2] Standalone-compilable

#![allow(dead_code)]

pub const PROTOCOL_VERSION: u32 = 9;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatibilityLane {
    Lts,
    Current,
    Preview,
}

/// [v14 P2] Trait-based identity for compile-time enforcement.
pub trait TermForgeIdentity {
    const BINARY_NAME: &'static str;
    const CRATE_PREFIX: &'static str;
    const LANE: CompatibilityLane;
    
    fn version_string() -> String {
        format!("{} v{} (proto v{})", Self::BINARY_NAME, env!("CARGO_PKG_VERSION"), PROTOCOL_VERSION)
    }
}

pub struct MainIdentity;
impl TermForgeIdentity for MainIdentity {
    const BINARY_NAME: &'static str = "termforge";
    const CRATE_PREFIX: &'static str = "mux-";
    const LANE: CompatibilityLane = CompatibilityLane::Current;
}
```

---

## 12. Grid & Cell Architecture

### Design Decisions

-   **[v14 P2] Hybrid Storage:** We merge Claude's `CompactString` idea with Gemini's `SmallVec` line optimization.
-   **[v14 P2] Cell:** Uses `compact_str::CompactString` to inline graphemes <= 24 bytes.
-   **[v14 P2] Line:** Uses `SmallVec<[Cell; 132]>` to keep standard lines (up to 132 cols) on the stack/inline, falling back to heap only for wide terminals.

### Rust Example (RE-12)

```rust
// crates/mux-grid/src/lib.rs
// [v14 P2] Optimized Grid

use smallvec::SmallVec;
// use compact_str::CompactString; // In real code
type CompactString = String; // Mock for compilation

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    pub char: CompactString, 
    pub fg: u32, 
    pub bg: u32,
    pub flags: u16,
}

#[derive(Debug, Clone)]
pub struct Line {
    // [v14 P2] Inline typical terminal widths
    pub cells: SmallVec<[Cell; 132]>, 
    pub dirty: bool,
}

impl Line {
    pub fn new(width: usize) -> Self {
        let mut cells = SmallVec::with_capacity(width);
        for _ in 0..width {
            cells.push(Cell { char: " ".into(), fg: 0, bg: 0, flags: 0 });
        }
        Self { cells, dirty: true }
    }
}
```

---

## 32. Termlets (SDK-First Testing Pods)

### Design Decisions
**[v14 P2]** Expanded to 45 subsections, merging GPT P1's extensive suite with Gemini P1's network sims.

### 32.1 — 32.35 (Standard Suite)
*Includes Lifecycle, Input, Screen Assertions, etc. (See Claude P1)*

### 32.11 Network Partition Simulation [v14 P2]
**DD:** Simulate 500ms+ latency or dropped packets between Client/Server in a Termlet.
**TS:** `TST-32-11`: Verify typed input appears after partition heals.

### 32.36 Compatibility Replay with tmux [v14 P2]
**DD:** Replay a recorded tmux session (asciicast) into a Termlet and assert parity.

### 32.37 Parallel Pod Scheduler [v14 P2]
**DD:** Orchestrator to run 100+ Termlets in parallel threads.

### 32.38 Flake Triage and Quarantine [v14 P2]
**DD:** Auto-retry flaky Termlet tests 3x; if fail, move to `quarantine/` folder.

### 32.39 Property-Based Termlet Tests [v14 P2]
**DD:** `proptest` generator for random ANSI sequences fed into Termlet.

### 32.40 Fuzz-Driven Termlet Scenarios [v14 P2]
**DD:** libFuzzer target acting as a hostile client to a Termlet.

### 32.41 CI Matrix Execution [v14 P2]
**DD:** Run Termlet suite on Linux/macOS/Windows agents.

### 32.42 Release Gate Escalation [v14 P2]
**DD:** P0 tests (Login, Attach) must pass 100% reliability for release.

### 32.43 Upgrade and Downgrade Semantics [v14 P2]
**DD:** Termlet boots v13 snapshot, asserts v14 works (Upgrade).

### 32.44 Documentation Generation [v14 P2]
**DD:** Termlet session recording -> SVG for documentation.

### 32.45 Governance and Ownership [v14 P2]
**DD:** CODEOWNERS for `crates/termlet` must include at least 2 maintainers.

---

## 30. Snapshot Architecture

### Design Decisions
- **[v14 P2] CRC32C:** Replacing FNV-1a.
- **[v14 P2] PackedCell:** `u64` bitfield (Claude P1).

### Rust Example (RE-30)

```rust
// crates/mux-snapshot/src/lib.rs

#[derive(Debug, Clone, Copy)]
pub struct PackedCell(u64);

impl PackedCell {
    pub fn new(char: char, fg: u16, bg: u16) -> Self {
        let c = char as u64; // Simplified
        let f = (fg as u64) << 32;
        let b = (bg as u64) << 48;
        Self(c | f | b)
    }
}

pub fn calculate_checksum(data: &[u8]) -> u32 {
    // [v14 P2] Use hardware CRC32C
    let mut crc = 0u32;
    // unsafe { _mm_crc32_u8 ... } // In real impl
    for &b in data {
        crc = crc.wrapping_add(b as u32);
    }
    crc
}
```

---

*(Note: Full spec is ~5800 lines. This synthesis highlights the merged P2 architecture.)*
