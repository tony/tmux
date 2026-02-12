# TermForge v16 Pass 2 Refined Scaffold -- Manifest

## Summary

- **Crates**: 12
- **Source files**: 20 (.rs)
- **Config files**: 13 (Cargo.toml)
- **Documentation files**: 4 (CLAUDE.md, AGENTS.md, README.md, manifest.md)
- **Total Rust LOC**: 5,838
- **Total LOC (all files)**: ~6,100
- **Tests (default features)**: 184
- **Tests (with crdt feature)**: 191 (+7 VectorClock tests)
- **Failures**: 0

## Per-Crate Breakdown

| Crate | Level | Files | LOC | Tests | Feature-gated |
|-------|-------|-------|-----|-------|---------------|
| mux-types | L0 | 7 (lib.rs, cell.rs, packed_cell.rs, line.rs, style.rs, identity.rs, error.rs) | 1,269 | 54 | crdt (declared, reserved) |
| mux-parser | L0 | 2 (lib.rs, byte_class.rs) | 630 | 18 | -- |
| mux-grapheme-arena | L0 | 1 (lib.rs) | 264 | 8 | -- |
| mux-kernel | L0 | 1 (lib.rs) | 626 | 19 | -- |
| mux-orm | L0 | 1 (lib.rs) | 256 | 11 | -- |
| mux-test-support | L0 | 1 (lib.rs) | 379 | 14 | -- |
| mux-grid | L1 | 2 (lib.rs, scrollback.rs) | 512 | 16 | -- |
| mux-snapshot | L1 | 1 (lib.rs) | 392 | 9 | -- |
| mux-pty | L1 | 1 (lib.rs) | 403 | 10 | -- |
| mux-proto | L1 | 1 (lib.rs) | 328 | 10 | -- |
| mux-time | L1 | 1 (lib.rs) | 408 | 6 (13 with crdt) | crdt: VectorClock, receive() |
| mux-termlet | L2 | 1 (lib.rs) | 332 | 9 | -- |

## File Inventory

### Workspace Root

| File | Lines | Purpose |
|------|-------|---------|
| Cargo.toml | 42 | Workspace definition, shared deps |
| CLAUDE.md | 57 | Project conventions for Claude Code |
| AGENTS.md | 11 | Agent instructions |
| README.md | 26 | Crate map |
| manifest.md | -- | This file |

### crates/mux-types/

| File | Lines | Purpose |
|------|-------|---------|
| Cargo.toml | 13 | Depends on compact_str; features: crdt |
| src/lib.rs | 29 | Module declarations and pub use re-exports |
| src/cell.rs | 210 | Cell struct with CompactString grapheme |
| src/packed_cell.rs | 233 | 64-bit PackedCell with compile-time bit-sum assertion |
| src/line.rs | 225 | Arc COW Line with to_text() |
| src/style.rs | 202 | Color enum, Attrs bitfield, Style with pack_index() |
| src/identity.rs | 131 | IdentityManifest, BuildProfile |
| src/error.rs | 239 | TermletError (16 variants, error codes 8001-8016) |

### crates/mux-parser/

| File | Lines | Purpose |
|------|-------|---------|
| Cargo.toml | 7 | No dependencies |
| src/lib.rs | 470 | VT parser: 14 states, CLASS_TABLE[256], Action enum |
| src/byte_class.rs | 160 | ByteClass (7 variants incl. DcsEntry), const fn classify |

### crates/mux-grapheme-arena/

| File | Lines | Purpose |
|------|-------|---------|
| Cargo.toml | 7 | No dependencies |
| src/lib.rs | 264 | Arena with 14-bit ext index, dedup, encode/decode |

### crates/mux-kernel/

| File | Lines | Purpose |
|------|-------|---------|
| Cargo.toml | 12 | Depends on mux-types, mux-grid, mux-time |
| src/lib.rs | 626 | GenSlotMap, ServerGraph, Event/Effect enums, Kernel reducer |

### crates/mux-grid/

| File | Lines | Purpose |
|------|-------|---------|
| Cargo.toml | 12 | Depends on mux-types, mux-grapheme-arena |
| src/lib.rs | 346 | Grid with put_char/put_grapheme, scroll, resize |
| src/scrollback.rs | 166 | Scrollback buffer with VecDeque, search |

### crates/mux-snapshot/

| File | Lines | Purpose |
|------|-------|---------|
| Cargo.toml | 12 | Depends on mux-types, crc32fast |
| src/lib.rs | 392 | 31-byte header, CRC32C encode/decode |

### crates/mux-pty/

| File | Lines | Purpose |
|------|-------|---------|
| Cargo.toml | 7 | No dependencies |
| src/lib.rs | 403 | Typestate PtyHandle (7 states), DynPtyHandle |

### crates/mux-proto/

| File | Lines | Purpose |
|------|-------|---------|
| Cargo.toml | 7 | No dependencies |
| src/lib.rs | 328 | Wire protocol v8, TLV frames, handshake |

### crates/mux-time/

| File | Lines | Purpose |
|------|-------|---------|
| Cargo.toml | 10 | Features: crdt |
| src/lib.rs | 408 | LamportClock, VectorClock (crdt-gated), DeterministicTimeSource, MonotonicGuard |

### crates/mux-termlet/

| File | Lines | Purpose |
|------|-------|---------|
| Cargo.toml | 10 | Depends on mux-types |
| src/lib.rs | 332 | TermletConfig, TermletBuilder, Termlet lifecycle |

### crates/mux-orm/

| File | Lines | Purpose |
|------|-------|---------|
| Cargo.toml | 7 | No dependencies |
| src/lib.rs | 256 | QueryList<T> with Queryable trait, filter/get/first |

### crates/mux-test-support/

| File | Lines | Purpose |
|------|-------|---------|
| Cargo.toml | 7 | No dependencies |
| src/lib.rs | 379 | DiffResult, DiffReport, GridSnapshot, TestGuard, socket isolation |

## Feature Gates

| Feature | Crates | What it gates |
|---------|--------|---------------|
| crdt | mux-types, mux-time | VectorClock struct + impl + Default, DeterministicTimeSource.vector field, receive() method, BTreeMap import, 7 tests |

## Invariants Covered

| INV | Description | Verified By |
|-----|-------------|-------------|
| INV-002 | All crates use mux- prefix | Cargo.toml workspace members |
| INV-003 | No unsafe outside mux-pty | `#![forbid(unsafe_code)]` in all other crates |
| INV-004 | Public types derive Debug | All public structs/enums |
| INV-005 | Errors implement Error + Send + Sync | Compile-time assert_send_sync in error.rs |
| INV-007 | Grid coordinates zero-based | Grid::new(), put_char() tests |
| INV-009 | Style is bitfield-packed | Attrs(u16), pack_index() |
| INV-012 | Grid mutation only via put_char/put_grapheme | Grid API design |
| INV-023 | Snapshot header is 31 bytes | Compile-time const assertion |
| INV-024 | No SmallVec for line cells | Vec<Cell> in Line |
| INV-025 | VectorClock monotonic per node | test_vector_clock_monotonic (crdt) |
| INV-026 | ByteClass has 7 variants incl DcsEntry | Compile-time variant count check |
| INV-027 | Kernel step() is pure | No std::io in mux-kernel |
| INV-029 | Cell width explicit (0/1/2) | CellWidth enum |
| INV-030 | Arena ext index 0 reserved | test_index_zero_is_invalid |
| INV-031 | CRC32C only for encode | crc32fast usage in mux-snapshot |
| INV-032 | Scrollback uses Arc<Vec<Cell>> | Line::cells is Arc |
| INV-033 | Deterministic under fixed clock | DeterministicTimeSource tests |

## Merged Contributions

### From Claude Pass 0
- Full 14-state VT parser state machine with CLASS_TABLE[256]
- Style module: Color enum, Attrs bitfield, Style with pack_index()
- Comprehensive test-support: DiffResult, DiffReport, GridSnapshot, TestGuard
- DeterministicTimeSource with LamportClock and VectorClock
- TermletError 16 variants with error codes

### From GPT Pass 0
- Module decomposition: byte_class.rs, scrollback.rs as separate modules
- pub use re-exports from lib.rs for ergonomic imports
- Compile-time const assertions for bit field sums (PackedCell)
- const fn classify_const and build_table in byte_class

### New in Pass 1 (Claude)
- mux-kernel crate: GenSlotMap, ServerGraph, Event/Effect enums, Kernel reducer (626 LOC, 19 tests)
- `#[cfg(feature = "crdt")]` gating on VectorClock and related code
- `#[must_use]` on all public Result-returning and pure accessor functions
- Test count raised from ~120 to 191
