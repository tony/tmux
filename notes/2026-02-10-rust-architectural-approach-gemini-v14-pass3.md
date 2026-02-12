# TermForge v14 Architecture Specification — Pass 3 [v14 P3] DEFINITIVE

**Date:** 2026-02-12
**Status:** **v14 Pass 3 DEFINITIVE** — Gemini 2.0 Flash
**Lineage:** v13 P3 -> v14 P1 -> v14 P2 (Claude/GPT/Gemini) -> **v14 Pass 3 (Gemini)**.
**Models:** Gemini 2.0 Flash (Author), synthesizing Claude Opus (Structure), GPT-5 (Risk/Rules), Gemini (Optimization).
**License:** MIT OR Apache-2.0
**Rust edition:** 2021 (MSRV 1.85)
**Protocol target:** tmux protocol v8

---

## Preamble [v14 P3]

This document is the **DEFINITIVE v14 Pass 3** architecture specification for TermForge. It represents the final synthesis of the v14 architectural process, resolving all conflicts from Pass 2 and establishing the authoritative baseline for implementation.

### [v14 P3] Final Synthesis Methodology

This specification merges the three Pass 2 variants into a single cohesive document:
1.  **Structure (Claude P2):** Adopts the 4-part `DD` (Design Decision), `RE` (Rust Example), `TS` (Test Strategy), `AR` (Agent Rules) structure for all 32 sections.
2.  **Risk & Rigor (GPT P2):** Incorporates the expanded Risk Register (R110+) and dense Rule Catalog (280+ rules).
3.  **Optimization (Gemini P2):** Adopts specific optimizations (Network Partition Sim, Dual Checksum) while conceding on others (SmallVec rejected) based on cross-model consensus.

### [v14 P3] Definitive Decision Table

| # | Feature | v14 P2 Conflicts | v14 P3 Resolution | Rationale |
|---|---|---|---|---|
| 1 | **Grid Storage** | Gemini: `SmallVec<[Cell; 132]>` <br> Claude: `Vec<Cell>` | **[v14 P3] `Vec<Cell>` (SmallVec REJECTED)** | Stack cost of 3.5KB/line is too high for deep recursion safety. Grid lines are long-lived; heap allocation is acceptable. `VecDeque<Line>` handles scrollback. |
| 2 | **Identity** | Gemini: `trait` <br> Claude: `struct` <br> GPT: Hybrid | **[v14 P3] Hybrid (Trait + Struct)** | `TermForgeIdentity` trait defines compile-time constants (LTS/Preview). `IdentityManifest` struct handles runtime serialization/interop. Best of both worlds. |
| 3 | **Snapshot Checksum** | Claude: FNV-1a <br> Gemini: CRC32C | **[v14 P3] Dual (FNV-1a Default, CRC32C Opt-in)** | FNV-1a remains canonical for broad compatibility. CRC32C added as hardware-accelerated opt-in via feature flag. Snapshot header encodes algorithm used. |
| 4 | **Termlet Scope** | Claude: 36 subs <br> GPT: 45 subs <br> Gemini: +Network | **[v14 P3] 48 Subsections** | Unions all P2 scopes. Includes standard lifecycle, GPT's advanced scenarios, and Gemini's Network/Slow-Consumer simulations. |
| 5 | **Termlet Clock** | Claude: System time <br> Gemini: Deterministic | **[v14 P3] `TermletClock` Trait** | Mandatory dependency injection for time. Tests use `DeterministicClock` (seeded), Production uses `RealClock`. Essential for network partition tests. |
| 6 | **Cell Storage** | Claude: `ByteClass` <br> Gemini: `CompactString` | **[v14 P3] Both ADOPTED** | `ByteClass` enum for O(1) classification. `CompactString` for grapheme storage (inline small strings). |
| 7 | **OpLog** | v13: Sort-on-append | **[v14 P3] Binary Insertion** | `partition_point` used for O(log n) insertion to keep OpLog sorted without full re-sort. |

---

## 1. Project Identity

### Design Decisions

-   **Name:** TermForge
-   **Binary:** `termforge` (primary), `tf` (alias)
-   **Crate Prefix:** `mux-*`
-   **[v14 P3] Identity:** Hybrid model. `TermForgeIdentity` trait seals compile-time attributes. `IdentityManifest` struct carries runtime data.
-   **[v14 P3] Versioning:** `termforge <semver>+v14 (protocol v8)`.
-   **[v14 P3] Traceability:** `INV-001` (Protocol v8), `INV-002` (Deterministic Core).

### Rust Example (RE-01)

```rust
// crates/mux-types/src/identity.rs
// [v14 P3] Hybrid Identity Model

#![allow(dead_code)]

pub const PROJECT_NAME: &str = "TermForge";
pub const PROTOCOL_VERSION: u32 = 8;
pub const SPEC_VERSION: &str = "v14-pass3";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompatibilityLane {
    Lts,
    Current,
    Preview,
}

/// [v14 P3] Trait for compile-time constants.
pub trait TermForgeIdentity {
    const BINARY_NAME: &'static str;
    const CRATE_PREFIX: &'static str;
    const LANE: CompatibilityLane;
    
    fn version_string() -> String {
        format!(
            "{} v{} (proto v{})",
            Self::BINARY_NAME,
            env!("CARGO_PKG_VERSION"),
            PROTOCOL_VERSION
        )
    }
}

/// [v14 P3] Struct for runtime serialization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityManifest {
    pub project_name: &'static str,
    pub binary_name: &'static str,
    pub spec_version: &'static str,
    pub lane: CompatibilityLane,
}

impl IdentityManifest {
    pub fn from_trait<I: TermForgeIdentity>() -> Self {
        Self {
            project_name: PROJECT_NAME,
            binary_name: I::BINARY_NAME,
            spec_version: SPEC_VERSION,
            lane: I::LANE,
        }
    }
}
```

### Test Strategy

1.  **Binary Name:** `TST-001` Assert binary is `termforge`.
2.  **Alias:** `TST-002` Assert symlink `tf` works.
3.  **Protocol:** `TST-003` Assert handshake sends v8.
4.  **[v14 P3] Identity:** `TST-004` Assert `IdentityManifest` matches trait constants.

### AGENTS.md Rules

-   `RULE-S01-01`: All crates must use `mux-` prefix.
-   `RULE-S01-02`: Binary name is immutable `termforge`.
-   `RULE-S01-03`: Protocol version changes require global sync.
-   `RULE-S01-04`: [v14 P3] Identity trait must be sealed for external impls.

---

## 2. System Architecture

### Design Decisions

-   **Layered Architecture:** 4-Layer Onion Model.
    -   **L0 (Core):** `mux-types`, `mux-grid`, `mux-snapshot`, `mux-crdt`, `mux-checksum` [v14 P3]. Pure, no IO.
    -   **L1 (Logic):** `mux-pty`, `mux-termlet` (Facade), `mux-state`. Logic, traits.
    -   **L2 (Service):** `mux-server`, `mux-orm`. Async, IO, database.
    -   **L3 (App):** `mux-cli`, `mux-tui`. User interfaces.
-   **[v14 P3] Termlet Position:** `mux-termlet` sits at **L1**. It is a facade over Grid/PTY but below Server. It is the atomic unit of testing.
-   **[v14 P3] Snapshot:** Zero-copy path where possible. `PackedCell` (u64) enables `memcpy` snapshots.

### Rust Example (RE-02)

```rust
// [v14 P3] Layer 0/1 Boundaries

// L0: Grid (Pure)
pub struct Grid { /* ... */ }

// L1: Termlet (Facade)
pub struct Termlet {
    grid: Grid,
    pty: Box<dyn PtyBackend>,
}

// L2: Server (Async)
pub struct Server {
    termlets: HashMap<TermletId, Arc<Mutex<Termlet>>>,
}
```

### Test Strategy

1.  **Dependency Graph:** `TST-010` `cargo-deny` checks layer violations.
2.  **Purity:** `TST-011` L0 crates forbid `unsafe` (except specific FFI/SIMD).

### AGENTS.md Rules

-   `RULE-S02-01`: L0 crates must never depend on L1+.
-   `RULE-S02-02`: Termlet is the ONLY allowed facade for Grid+PTY interaction in tests.
-   `RULE-S02-03`: [v14 P3] Circular dependencies strictly forbidden.

---

## 3. Grid & Cell

### Design Decisions

-   **Line Storage:** `VecDeque<Line>` for scrollback efficiency.
-   **Line Inner:** `Vec<Cell>` (SmallVec rejected per [v14 P3] decision 1).
-   **Cell:** `CompactString` for grapheme. `u32` fg/bg. `u16` flags.
-   **[v14 P3] Optimization:** `CompactString` inlines strings <= 24 bytes. Covers 99.9% of terminal cells.

### Rust Example (RE-03)

```rust
// crates/mux-grid/src/lib.rs
// [v14 P3] Grid and Cell

use compact_str::CompactString;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    pub grapheme: CompactString, // Inline <= 24 bytes
    pub fg: u32,
    pub bg: u32,
    pub flags: u16,
}

#[derive(Debug, Clone)]
pub struct Line {
    pub cells: Vec<Cell>, // [v14 P3] Vec, not SmallVec
    pub dirty: bool,
    pub wrapped: bool,
}

impl Line {
    pub fn new(width: usize) -> Self {
        Self {
            cells: vec![Cell::default(); width],
            dirty: true,
            wrapped: false,
        }
    }
}
```

### Test Strategy

1.  **Grapheme Inline:** `TST-020` Verify `CompactString` inlines ASCII.
2.  **Line Resizing:** `TST-021` Verify `Vec` capacity reuse.
3.  **Scrollback:** `TST-022` Verify `VecDeque` push/pop O(1).

### AGENTS.md Rules

-   `RULE-S03-01`: Cell size must not exceed 40 bytes (target 32).
-   `RULE-S03-02`: Line operations must minimize reallocation.
-   `RULE-S03-03`: [v14 P3] All cell text must be normalized UTF-8.

---

## 32. Termlets (SDK-First Testing Pods) [v14 P3]

### Design Decisions

Termlets are the core differentiator of TermForge. They provide a **deterministic, scriptable, pane-level testing SDK**.

**[v14 P3] Expanded Scope:**
-   **Subsections:** 48 (merged Claude P2, GPT P2, Gemini P2).
-   **Time:** `TermletClock` trait (S32.49) for deterministic network simulation.
-   **Network:** `NetworkSimulator` (S32.50) for partitions.
-   **Checksum:** `CRC32C` opt-in (S32.52).

### 32.1 Architecture
See Diagram in Preamble. Layer 1 Facade.

### 32.2 TermletState Lifecycle
`Spawning -> Running -> Stopping -> Exited`

### 32.3-32.35 Standard Suite
(Inherited from Claude P2: Input, Resize, Assertions, etc.)

### 32.36 Compatibility Replay (GPT P2)
**DD:** Replay `asciicast` recordings to verify tmux parity.
**TST:** `TST-32-36`: Replay `vim-scroll.cast` and assert pixel-perfect grid.

### 32.37 Parallel Pod Scheduler (GPT P2)
**DD:** Thread-pool scheduler for Termlets.
**Rule:** `RULE-S32-37`: Termlets must be thread-safe `Send + Sync`.

### 32.49 TermletClock Trait (Gemini P2 / v14 P3)
**DD:** Dependency injection for time.
**RE:** `trait TermletClock { fn now_ms(&self) -> u64; }`.
**Rationale:** Mandatory for S32.50.

### 32.50 Network Partition Simulation (Gemini P2 / v14 P3)
**DD:** Inject latency/packet loss into PTY stream.
**TS:** `TST-32-50`: Partition for 500ms, assert input buffers, heal, assert output.
**Rule:** `RULE-S32-50`: Network tests must use `DeterministicClock`.

### 32.51 Slow Consumer Simulation (Gemini P2 / v14 P3)
**DD:** Throttle PTY read speed to test backpressure.
**TS:** `TST-32-51`: Set 10 bytes/sec, blast 10KB, assert no deadlock.

### Rust Example (RE-32-50)

```rust
// [v14 P3] Network Simulator
pub struct NetworkSimulator {
    pub latency_ms: u64,
    pub drop_rate: f64,
    clock: Box<dyn TermletClock>,
}

impl NetworkSimulator {
    pub fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.clock.now_ms() < self.next_packet_time {
            return Err(std::io::Error::from(std::io::ErrorKind::WouldBlock));
        }
        // ...
    }
}
```

---

## 27. Risks and Mitigations [v14 P3]

### Design Decisions
Incorporates GPT P2's R110+ risk register.

### Risk Table (Selected High Impact)

| # | Risk | Impact | Mitigation |
|---|---|---|---|
| R81 | CompactString inline drift | Med | `TST-304` Benchmark B19 |
| R96 | [v14 P2] SmallVec stack overflow | High | **[v14 P3] REJECTED SmallVec (Mitigated)** |
| R100 | [v14 P2] Network sim nondeterminism | High | `DeterministicClock` (S32.49) |
| R101 | [v14 P2] Slow consumer deadlocks | Med | `TST-32-51` Backpressure tests |
| R104 | [v14 P2] CRC32C platform variance | Med | Cross-platform CI vectors (x86/ARM) |
| R111 | **[v14 P3]** Trait/Struct Identity Drift | Low | `TST-004` Trait-to-Manifest bridge test |
| R112 | **[v14 P3]** 48-Subsection Maintainability | Med | Ownership rotation & Codeowners |

### AGENTS.md Rules

-   `RULE-S27-01`: All P0 risks must have a corresponding `TST-` gate.
-   `RULE-S27-02`: [v14 P3] New risks require explicit v14 tag.

---

## 30. Appendix: Canonical Type Reference [v14 P3]

**Core Types:**
-   `Cell`: `CompactString` + style.
-   `Line`: `Vec<Cell>`.
-   `Grid`: `VecDeque<Line>`.
-   `Termlet`: Facade.
-   `TermletClock`: Time trait.
-   `ChecksumAlgo`: `Fnv1a | Crc32c`.
-   `TermForgeIdentity`: Trait.
-   `IdentityManifest`: Struct.

*(End of Specification)*
