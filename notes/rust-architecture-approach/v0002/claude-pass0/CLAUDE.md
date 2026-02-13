# CLAUDE.md -- Project Conventions for Claude Code

## Project: TermForge

TermForge is a Rust terminal multiplexer SDK targeting 100% tmux wire-protocol
compatibility. The architecture follows the v25 DEFINITIVE specification.

## Build & Test

```bash
cargo check                    # Type-check all crates
cargo test                     # Run all tests
cargo test -p mux-grid         # Run tests for a single crate
cargo clippy -- -D warnings    # Lint all crates
cargo doc --no-deps            # Generate documentation
```

## Coding Standards

- Rust edition 2024, MSRV 1.85.
- All crates use `mux-` prefix.
- `#![forbid(unsafe_code)]` in all crates EXCEPT `mux-pty` and `mux-ffi`.
  `mux-pty` is the unsafe quarantine for PTY syscalls.
  `mux-ffi` is the C ABI boundary with `catch_unwind` panic recovery.
- All public types derive `Debug`.
- All error types implement `std::error::Error + Send + Sync + 'static` via `thiserror`.
- No panicking in library code. No `unwrap()`, `expect()`, `panic!()` outside tests.
  The workspace Cargo.toml enforces `clippy::unwrap_used = "deny"`.
- Grid coordinates are zero-based (row, col).
- Grid mutation ONLY via `line_mut()` which triggers COW + dirty marking.
- All public APIs MUST be deterministic under fixed seed and `Clock::manual()`.

## Architecture (3 Threads)

```
IO Thread (tokio) <--channels--> Kernel Thread (std) --snapshot--> Render Thread
```

- **IO Thread**: tokio async runtime. Owns PTY FDs, Unix socket, signal handling.
- **Kernel Thread**: `std::thread`. Owns ALL mutable state. Single-threaded, no locks.
  Processes `KernelEvent`s, emits `KernelEffect`s. Sans-IO.
- **Render Thread**: Composes grid snapshots into terminal escape sequences.

## Crate Dependency Layers

```
L0 (no deps):     mux-grapheme-arena, mux-time
L1 (L0 only):     mux-types
L2 (L0+L1):       mux-grid, mux-parser, mux-proto, mux-options, mux-target, mux-format
L3 (L0-L2):       mux-snapshot, mux-cmd-parse, mux-orm, mux-render, mux-pty
L4 (L0-L3):       mux-kernel, mux-termlet
L5 (L0-L4):       mux-test-support, mux-ffi, mux-otel
L6 (tools):       tmux-builder, tmux-vm, tmux-sniff, mux-doctor
```

## File Organization

- Each crate lives in `crates/<crate-name>/`.
- Each crate has `src/lib.rs` as its root module.
- Tools live in `tools/<tool-name>/` with both `src/lib.rs` and `src/main.rs`.
- Tests go in `#[cfg(test)] mod tests` at the bottom of each file.
- Integration tests go in `tests/` at the workspace root.

## Commit Conventions

- Use conventional commits: `feat:`, `fix:`, `refactor:`, `test:`, `docs:`, `chore:`.
- Reference spec items: `feat(mux-grid): implement COW scrollback (INV-127)`.
- Keep commits atomic: one logical change per commit.

## Test Patterns

- Every public function has at least one unit test.
- Tests use descriptive names: `test_cow_on_shared_chunk`, `test_wide_char_co_invalidation`.
- Use `insta` for snapshot testing (golden file comparison).
- Use `proptest` for round-trip and property testing.
- Use `mux-test-support` fixtures for integration tests.
- The `Clock::manual()` pattern enables deterministic time-dependent tests.

## Key Invariants

- Chunked grid: `Vec<Arc<LineChunk>>` with `CHUNK_SIZE=256` (INV-127).
- Single `line_dirty: BitVec` per grid (chunk_dirty DROPPED).
- Global revision counter in the kernel (not per-grid).
- CellFlags bit values match tmux's `GRID_FLAG_*` exactly (INV-119).
- TF01 protocol magic `0x54464F31` discriminates from tmux imsg in 1 byte.
- `catch_unwind` at FFI boundary prevents Rust panics from unwinding into C (INV-117).

## What NOT to Do

- Do NOT use `unsafe` outside `mux-pty` and `mux-ffi`.
- Do NOT use `unwrap()` or `expect()` in library code.
- Do NOT use `SmallVec` for line cells.
- Do NOT use `async` in the kernel thread -- it is `std::thread` only.
- Do NOT share mutable state between threads. The kernel owns all mutable state.
- Do NOT modify grid cells directly; always go through `line_mut()`.
- Do NOT store scrollback as `Vec<Cell>`; use `Arc<LineChunk>` COW.
- Do NOT use `#[no_mangle]` directly -- use `#[unsafe(no_mangle)]` (edition 2024).
