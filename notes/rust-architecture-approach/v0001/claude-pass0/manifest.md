# TermForge v0001/claude-pass0 -- File Manifest

All files created in this scaffold. Every `.rs` file compiles with
`rustc --edition=2021 --crate-type lib` and all tests pass.

## Root Project Files

| File | Description |
|------|-------------|
| `Cargo.toml` | Workspace root: 11 crates, workspace dependencies (compact_str, crc32fast, serde) |
| `README.md` | Project overview: crate map, key decisions, build instructions |
| `AGENTS.md` | All 405+ rules from v15 organized by section (S01-S32), with enforcement methods |
| `CLAUDE.md` | Project conventions for Claude Code: coding standards, file organization, key invariants |
| `manifest.md` | This file: listing of every file created |

## Documentation

| File | Description |
|------|-------------|
| `notes/architecture.md` | Condensed architecture reference: key decisions table, 34 invariants, 100 settled decisions, crate DAG, PackedCell layout, ByteClass variants, snapshot header, error codes, wire protocol |
| `notes/plan.md` | Implementation roadmap in 5 phases (20 weeks): Foundation, Grid+Snapshot, PTY+Protocol, Termlet+ORM, Polish+Compatibility |

## Crate: mux-types (L0 -- Core Types)

| File | Description |
|------|-------------|
| `crates/mux-types/Cargo.toml` | Crate manifest with compact_str dependency |
| `crates/mux-types/src/lib.rs` | Root module: re-exports Cell, PackedCell, Line, Style, IdentityManifest, TermletError |
| `crates/mux-types/src/cell.rs` | Cell type: grapheme (CompactString), style, width (0/1/2), is_wide_char heuristic. 5 tests |
| `crates/mux-types/src/packed_cell.rs` | PackedCell: 64-bit packed cell (scalar:21+style:15+flags:12+width:2+ext:14). LE encode/decode. 5 tests |
| `crates/mux-types/src/line.rs` | Line: Arc<Vec<Cell>> COW semantics, set/clear/resize, ref_count tracking. 5 tests |
| `crates/mux-types/src/style.rs` | Style: Color (Default/Indexed/Rgb), Attrs bitfield, pack/unpack. 5 tests |
| `crates/mux-types/src/identity.rs` | IdentityManifest: project name, version, spec version, build profile. 5 tests |
| `crates/mux-types/src/error.rs` | TermletError: 16 variants with unique u16 error codes, Display with [Ecode] prefix. 5 tests |

## Crate: mux-parser (L0 -- VT Parser)

| File | Description |
|------|-------------|
| `crates/mux-parser/Cargo.toml` | Crate manifest (no external deps) |
| `crates/mux-parser/src/lib.rs` | CLASS_TABLE[256] static LUT, ByteClass (7 variants), classify_match oracle, Parser state machine with step(), full CSI/ESC/OSC/DCS handling. 5 tests |

## Crate: mux-grid (L0 -- Terminal Grid)

| File | Description |
|------|-------------|
| `crates/mux-grid/Cargo.toml` | Crate manifest: depends on mux-types, mux-grapheme-arena, compact_str |
| `crates/mux-grid/src/lib.rs` | Grid: VecDeque<Line>, put_char/put_grapheme, scroll_up/down, COW tracking, resize, cursor management, viewport. 6 tests |

## Crate: mux-snapshot (L0 -- Snapshot Format)

| File | Description |
|------|-------------|
| `crates/mux-snapshot/Cargo.toml` | Crate manifest: depends on mux-types, mux-grapheme-arena, crc32fast |
| `crates/mux-snapshot/src/lib.rs` | SnapshotHeader (31 bytes), CRC32C checksum, PackedCell encode/decode, full snapshot encode/decode with arena trailer. 5 tests |

## Crate: mux-grapheme-arena (L0 -- Grapheme Storage)

| File | Description |
|------|-------------|
| `crates/mux-grapheme-arena/Cargo.toml` | Crate manifest (no external deps) |
| `crates/mux-grapheme-arena/src/lib.rs` | GraphemeArena: 14-bit ext index, deduplication, binary encode/decode, capacity limit. 5 tests |

## Crate: mux-pty (L1 -- PTY Lifecycle)

| File | Description |
|------|-------------|
| `crates/mux-pty/Cargo.toml` | Crate manifest: depends on mux-types |
| `crates/mux-pty/src/lib.rs` | InnerPtyState (7 states), PtyHandle<S> typestate (Created->Running->Killing->Closed), DynPtyHandle with runtime tracking, generation counter, TryFrom conversion. 5 tests |

## Crate: mux-proto (L0 -- Wire Protocol)

| File | Description |
|------|-------------|
| `crates/mux-proto/Cargo.toml` | Crate manifest (no external deps) |
| `crates/mux-proto/src/lib.rs` | FrameType (10 variants), Frame TLV encode/decode, 64 KiB size limit, handshake validation, protocol version check. 5 tests |

## Crate: mux-time (L1 -- Deterministic Time)

| File | Description |
|------|-------------|
| `crates/mux-time/Cargo.toml` | Crate manifest: depends on mux-types |
| `crates/mux-time/src/lib.rs` | LamportClock, VectorClock (merge/dominates/concurrent), DeterministicTimeSource (wall+Lamport+vector), MonotonicGuard. 6 tests |

## Crate: mux-termlet (L2 -- Termlet Runtime)

| File | Description |
|------|-------------|
| `crates/mux-termlet/Cargo.toml` | Crate manifest: depends on mux-types, mux-grid, mux-pty, mux-time |
| `crates/mux-termlet/src/lib.rs` | ResourceQuota, TermletConfig with validation, TermletBuilder, Termlet with send_keys/wait_for/resize/kill, Drop cleanup. 5 tests |

## Crate: mux-orm (L2 -- ORM Query Interface)

| File | Description |
|------|-------------|
| `crates/mux-orm/Cargo.toml` | Crate manifest: depends on mux-types |
| `crates/mux-orm/src/lib.rs` | QueryError (ObjectDoesNotExist, MultipleObjectsReturned), Queryable trait, QueryList with filter/get/get_by_id/get_by_name. 5 tests |

## Crate: mux-test-support (L2 -- Test Harness)

| File | Description |
|------|-------------|
| `crates/mux-test-support/Cargo.toml` | Crate manifest: depends on mux-types |
| `crates/mux-test-support/src/lib.rs` | TestGuard (RAII cleanup), test_socket_name (isolation), DiffResult/DiffReport (differential testing), GridSnapshot (golden files), section coverage validation. 7 tests |

## Statistics

| Metric | Count |
|--------|-------|
| Total files | 30 |
| Rust source files (.rs) | 18 |
| Cargo.toml files | 12 |
| Documentation files (.md) | 6 |
| Total Rust crates | 11 |
| Total tests | 89 |
| Tests passing | 89 (100%) |
| Lines of Rust code | ~3,500 |
| v15 invariants with tests | 21 of 34 |
| v15 settled decisions referenced | ~25 of 100 |
