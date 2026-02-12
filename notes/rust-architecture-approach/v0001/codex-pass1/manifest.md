# TermForge v0001/codex-pass1 -- File Manifest

Refined scaffold produced by cross-pollinating `claude-pass0` and `gpt5-pass0`.

## Workspace Summary

- Crates: 12 (`mux-kernel` added)
- Rust source files: 23
- Cargo manifests: 13 (workspace + 12 crates)
- Markdown docs: 6
- Unit tests: 131 (`cargo test --workspace`)

## Root Files

- `Cargo.toml`
- `Cargo.lock`
- `README.md`
- `AGENTS.md`
- `CLAUDE.md`
- `manifest.md`
- `notes/architecture.md`
- `notes/plan.md`

## Crates

- `crates/mux-types`
- `crates/mux-parser`
- `crates/mux-grid`
- `crates/mux-snapshot`
- `crates/mux-grapheme-arena`
- `crates/mux-proto`
- `crates/mux-time`
- `crates/mux-pty`
- `crates/mux-termlet`
- `crates/mux-orm`
- `crates/mux-test-support`
- `crates/mux-kernel`

## Notable Pass 1 Changes

- `mux-grid` split into `src/lib.rs`, `src/grid.rs`, `src/scrollback.rs` with `pub use` re-exports.
- `mux-parser` split into `src/lib.rs` (state machine) and `src/byte_class.rs` (CLASS_TABLE + oracle).
- `mux-snapshot` split into `src/lib.rs` and `src/packed_cell.rs`.
- `mux-types::PackedCell` has both compile-time bit-sum assertion and runtime size assertion.
- `#[cfg(feature = "crdt")]` stubs added in `mux-types` and `mux-kernel`.
- `mux-kernel` adds `Event`/`Effect`, `ServerGraph`, `GenSlotMap`, and OTEL span macros (`otel_span!`, `otel_scope!`).
- `#[must_use]` added on all public `Result`-returning functions across crates.
