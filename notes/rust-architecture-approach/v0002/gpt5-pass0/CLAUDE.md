# CLAUDE.md

## Coding Conventions
- Rust 2024, MSRV 1.85, `cargo fmt` + `cargo clippy --workspace --all-targets` before merge.
- Library crates use `thiserror`; binaries/tools may use `anyhow`.
- No `unwrap()`/`expect()` in library code.
- `unsafe` is forbidden outside `mux-pty`.
- Keep APIs deterministic and testable with fake backends.

## Commit and PR Conventions
- Commit prefix: `crate-name: short imperative summary`.
- Every PR includes: architecture impact, invariants affected, tests added.
- For protocol changes, include frame-level compatibility tests.

## Test Patterns
- Unit tests for pure parsing/layout logic.
- Integration tests under `mux-test-support` with isolated socket paths.
- Snapshot tests use `insta`; baseline updates require reviewer approval.

## Type Invariants
- `CellFlags::PADDING` is `0x04` and must mirror tmux `GRID_FLAG_PADDING`.
- Kernel mutates state on a single thread only.
- Grid revision is global per-grid and monotonic.
- `TF01` header is always 16 bytes.
- Channel capacities must be configurable from `KernelConfig`.

## What Not To Do
- Do not hardcode test sockets under `/tmp/tmux-<uid>/default`; always randomize.
- Do not introduce GPU render abstractions.
- Do not bypass RAII cleanup in test infrastructure.
- Do not couple ORM traversal APIs directly to transport-specific details.

## References
- `notes/architecture.md`
- `notes/plan.md`
- tmux source (`GRID_FLAG_PADDING`, render and process management files)
