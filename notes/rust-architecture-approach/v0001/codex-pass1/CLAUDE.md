# CLAUDE.md -- Project Conventions for Claude Code

## Project: TermForge

TermForge is a Rust terminal multiplexer targeting 100% tmux wire-protocol v8 compatibility.
The architecture is specified in v15 Pass 3 DEFINITIVE (8,628 lines, 405 rules, 34 invariants,
100 settled decisions).

## Coding Standards

- Rust edition 2021, MSRV 1.75.0.
- All crates use `mux-` prefix (INV-002).
- No unsafe outside `mux-pty` (INV-003). All other crates use `#![forbid(unsafe_code)]`.
- All public types derive `Debug` (INV-004).
- All error types implement `std::error::Error + Send + Sync + 'static` (INV-005).
- No panicking in library code (INV-006). No `unwrap()`, `expect()`, `panic!()` outside tests.
- Use `compact_str::CompactString` for Cell graphemes (S91).
- Use `crc32fast` for CRC32C checksums (S93).
- Grid coordinates are zero-based (row, col) (INV-007).
- Grid mutation ONLY via `put_char()`/`put_grapheme()` (INV-012).
- All public APIs MUST be deterministic under fixed seed and clock injection (S100).

## File Organization

- Each crate lives in `crates/<crate-name>/`.
- Each crate has `src/lib.rs` as its root module.
- Additional modules are in `src/<module>.rs` and re-exported from `lib.rs`.
- Tests go in `#[cfg(test)] mod tests` at the bottom of each file.
- Integration tests go in `tests/` at the workspace root.
- Benchmarks go in `benchmarks/` using Criterion.
- Fuzz targets go in `fuzz/`.

## Commit Conventions

- Use conventional commits: `feat:`, `fix:`, `refactor:`, `test:`, `docs:`, `chore:`.
- Reference invariants and settled decisions: `feat(mux-grid): implement COW scrollback (S96, INV-032)`.
- Keep commits atomic: one logical change per commit.

## Test Patterns

- Every public function has at least one unit test.
- Tests use descriptive names: `test_packed_cell_round_trip`, `test_scroll_up_cow`.
- Tests reference TST IDs from the spec where applicable.
- Use `assert_eq!` with clear messages. Use `matches!` for enum variant checks.
- Property tests use `proptest` for round-trip operations.
- Fuzz tests use `cargo-fuzz` with persistent corpus.

## What NOT to Do

- Do NOT use `SmallVec` for line cells (INV-024, S75).
- Do NOT use `unsafe` outside `mux-pty`.
- Do NOT use `unwrap()` or `expect()` in library code.
- Do NOT use FNV-1a for encoding (S93, INV-031). CRC32C only for encode.
- Do NOT enable both `crdt` and `wasm` features simultaneously.
- Do NOT use `std::io` or `std::net` in L0 crates.
- Do NOT infer cell width at render time; width is explicit (0/1/2) (INV-029).
- Do NOT modify grid cells directly; always go through `put_char`/`put_grapheme` (INV-012).
- Do NOT store scrollback lines as `Vec<Cell>`; use `Arc<Vec<Cell>>` (INV-032).

## Key Type Invariants

- `PackedCell` is exactly 64 bits: scalar:21 + style:15 + flags:12 + width:2 + ext:14 (S94).
- `ByteClass` has exactly 7 variants including `DcsEntry` (INV-026).
- `TermletError` has exactly 16 variants (S98).
- `InnerPtyState` has exactly 7 states (S99).
- Snapshot header is exactly 31 bytes (INV-023).
- GraphemeArena ext index 0x0000 is reserved for "no extension" (INV-030).
- VectorClock entries are monotonically non-decreasing per node (INV-025).

## Architecture References

- Full spec: `notes/2026-02-10-rust-architectural-approach-synthesized-v15.md`
- Condensed reference: `notes/architecture.md`
- Implementation plan: `notes/plan.md`
- Rule catalog: `AGENTS.md`
