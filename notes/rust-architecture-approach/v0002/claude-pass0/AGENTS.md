# AGENTS.md -- Agent Rules and Quality Gates

## Overview

This document defines the rules, agent roles, and quality gates for
TermForge development. It is inspired by vibe-tmux's AGENTS.md and
adapted for the TermForge SDK architecture.

## Agent Roles

### Architect Agent
- Owns `notes/architecture.md`, `notes/plan.md`, `CLAUDE.md`, `AGENTS.md`.
- Ensures all crates follow the dependency layer hierarchy (L0-L6).
- Reviews breaking changes to public APIs.
- Validates that new crates do not create circular dependencies.

### Implementer Agent
- Implements crate functionality per the `notes/plan.md` phase schedule.
- Follows the coding standards in `CLAUDE.md`.
- Every public function gets a doc comment and at least one unit test.
- Uses `todo!()` for deferred implementations, NEVER `unimplemented!()`.

### Reviewer Agent
- Reviews all PRs for compliance with invariants and coding standards.
- Checks that `#![forbid(unsafe_code)]` is present in all crates except `mux-pty`/`mux-ffi`.
- Verifies no `unwrap()` or `expect()` in library code.
- Ensures tests exist for all new public APIs.
- Validates that error types use `thiserror` and implement `std::error::Error`.

### Test Agent
- Maintains `mux-test-support` fixtures and helpers.
- Ensures snapshot tests are regenerated after grid/render changes.
- Runs property tests (proptest) for round-trip operations.
- Monitors test coverage (target: 80%+ line coverage for core crates).

## Quality Gates

### Gate 1: Compilation (BLOCKING)
- `cargo check` must pass with zero errors.
- `cargo clippy -- -D warnings` must pass.
- All workspace lints in `Cargo.toml` must be satisfied.

### Gate 2: Tests (BLOCKING)
- `cargo test` must pass with zero failures.
- New code must include tests (reviewer enforced).
- Snapshot tests must be up-to-date (`cargo insta test`).

### Gate 3: Safety (BLOCKING)
- `#![forbid(unsafe_code)]` in all crates except `mux-pty` and `mux-ffi`.
- All `unsafe` blocks in `mux-pty`/`mux-ffi` have `// SAFETY:` comments.
- `catch_unwind` at every FFI boundary (`mux-ffi`).
- No `unwrap()` or `expect()` outside `#[cfg(test)]` modules.

### Gate 4: Documentation (NON-BLOCKING but expected)
- All public types and functions have doc comments.
- Module-level doc comments explain the crate's purpose and design.
- Architecture decisions reference spec identifiers (INV-nnn, RULE-nnn).

### Gate 5: Performance (NON-BLOCKING, measured)
- Grid operations benchmarked with Criterion.
- VT parser throughput measured (target: >100 MB/s).
- COW snapshot cost measured (target: <100us for 10K-line grid).
- Render pipeline latency measured (target: <16ms for 200x50 terminal).

## Invariant Rules

All invariants from the v25 DEFINITIVE spec are enforced. Key ones:

### INV-117: Panic Recovery at FFI Boundary
Every `extern "C"` function in `mux-ffi` MUST be wrapped in
`std::panic::catch_unwind`. Panics must not unwind into C.

### INV-119: Cell Flags Match tmux Exactly
`CellFlags` bit values MUST match tmux's `GRID_FLAG_*` from `tmux.h`.
See `mux-types/src/lib.rs` for the verified values with tmux.h line references.

### INV-127: Chunked COW Grid
- `Vec<Arc<LineChunk>>` with `CHUNK_SIZE=256`.
- Single `line_dirty: BitVec` (NOT per-chunk dirty tracking).
- `Arc::make_mut` for COW on `line_mut()`.
- `snapshot()` clones the backbone `Vec<Arc<>>`, sharing chunks.

### INV-130: TF01 Protocol Discrimination
TF01 magic byte `0x54` exceeds all tmux message types (max 307).
Discrimination is safe in 1 byte; 4 bytes used for defense in depth.

## Workflow Rules

### Adding a New Crate
1. Create `crates/<name>/Cargo.toml` and `crates/<name>/src/lib.rs`.
2. Add to `[workspace.members]` in root `Cargo.toml`.
3. Add `<name> = { path = "crates/<name>" }` to `[workspace.dependencies]`.
4. Add `#![forbid(unsafe_code)]` unless the crate requires unsafe.
5. Update `CLAUDE.md` dependency layer diagram.
6. Run `cargo check` to verify no circular dependencies.

### Adding a New Tool
1. Create `tools/<name>/Cargo.toml` with `[[bin]]` section.
2. Create `tools/<name>/src/main.rs` and optionally `src/lib.rs`.
3. Add to `[workspace.members]` in root `Cargo.toml`.
4. Tool crates should use `#![forbid(unsafe_code)]`.

### Modifying Public APIs
1. Update the type signatures.
2. Update all callers.
3. Update tests.
4. Update doc comments.
5. If breaking: document in commit message.
6. `cargo check && cargo test && cargo clippy -- -D warnings`.

## Test Strategy

### Unit Tests
- Inline `#[cfg(test)] mod tests` in each source file.
- Every public function has at least one test.
- Use `assert_eq!` with descriptive messages.
- Use `matches!()` for enum variant assertions.

### Snapshot Tests
- Use `insta` for golden-file testing of grid content.
- Snapshot files live in `<crate>/src/snapshots/`.
- Regenerate with `cargo insta test` after intentional changes.

### Property Tests
- Use `proptest` for round-trip testing (parse/serialize).
- Use `proptest` for grid resize/reflow invariants.
- Strategies defined in `mux-test-support`.

### Integration Tests
- Test the full kernel event loop with `mux-test-support::TestServer`.
- Test PTY interactions with `mux-termlet`.
- Test wire protocol with `mux-proto` encode/decode round-trips.

### Fuzz Tests
- VT parser: fuzz with arbitrary byte sequences.
- Protocol decoder: fuzz with arbitrary frames.
- Command parser: fuzz with arbitrary strings.
- Located in `fuzz/` at workspace root (cargo-fuzz).
