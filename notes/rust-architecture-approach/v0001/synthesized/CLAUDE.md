# CLAUDE.md -- Project Conventions

## Scope

This scaffold is the v16 definitive Rust architecture baseline for TermForge.

## Hard Rules

- Rust 2021, MSRV 1.75.0.
- Crate names use `mux-` prefix (INV-002).
- `unsafe` is forbidden in every crate except `mux-pty` (INV-003).
- All public types derive `Debug` (INV-004).
- All error types implement `std::error::Error + Send + Sync + 'static` (INV-005).
- Public `Result`-returning APIs are `#[must_use]`.
- No panics in library code paths; `unwrap/expect` are test-only.
- Parser hot path uses `CLASS_TABLE[256]`; `classify_match()` remains the oracle.
- Snapshot encode uses CRC32C.
- `PackedCell` keeps `21+15+12+2+14 == 64` with compile-time and runtime checks.
- Vector-clock logic is feature-gated behind `crdt`; lamport-only fallback exists without it.

## Preferred Patterns

- Expose stable API surfaces from `lib.rs` via `pub use` or `prelude` modules.
- Keep reducer logic in `mux-kernel` pure and deterministic.
- Keep serialization little-endian and deterministic.
- Keep tests close to implementation and name by behavior/invariant.

## What Not To Do

- Do not add `SmallVec` for core line/cell storage.
- Do not move I/O concerns into `mux-kernel`.
- Do not remove `#[cfg(feature = "crdt")]` gates around vector-clock fields/methods.
- Do not change `PackedCell` bit layout without updating invariants/tests/docs.

## Primary References

- `AGENTS.md`: full 405+ rule catalog with enforcement methods.
- `notes/architecture.md`: condensed architecture and invariants.
- `notes/plan.md`: 10-phase implementation roadmap.
