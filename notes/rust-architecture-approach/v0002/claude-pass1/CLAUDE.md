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

## Comment Guidance (from Gemini)

- **Explain WHY, not WHAT**. The code shows what; comments should explain the
  reasoning behind non-obvious decisions.
- Reference tmux source locations: `// matches tmux screen-write.c:1928-1976`.
- Reference invariants: `// INV-127: COW chunk boundary`.
- Reference spec items: `// RULE-S33-87: hand-written parser`.

## Architecture (3 Threads)

```
IO Thread (tokio) <--crossbeam channels--> Kernel Thread (std) --snapshot--> Render Thread
```

- **IO Thread**: tokio async runtime. Owns PTY FDs, Unix socket, signal handling.
  Uses signal-hook pipes for SIGWINCH and SIGCHLD.
- **Kernel Thread**: `std::thread`. Owns ALL mutable state. Single-threaded, no locks.
  Processes `KernelEvent`s, emits `KernelEffect`s. Sans-IO.
- **Render Thread**: Composes grid snapshots into terminal escape sequences.
  Uses CompositeGrid double-buffer diff.

## Channel Bounds

- Control (IO -> Kernel): 512 messages
- Data (IO -> Kernel): 1024 * 64 KiB
- Render (Kernel -> Render): 256 snapshots
- Effect (Kernel -> IO): 1024 messages

## Crate Dependency Layers

```
L0 (no deps):     mux-grapheme-arena, mux-time
L1 (L0 only):     mux-types
L2 (L0+L1):       mux-grid, mux-parser, mux-proto, mux-options, mux-target,
                   mux-format, mux-config
L3 (L0-L2):       mux-snapshot, mux-cmd-parse, mux-orm, mux-render, mux-pty
L4 (L0-L3):       mux-kernel, mux-termlet
L5 (L0-L4):       mux-api, mux-test-support, mux-ffi, mux-otel
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

- INV-117: `catch_unwind` at FFI boundary.
- INV-119: CellFlags bit values match tmux's `GRID_FLAG_*` exactly.
- INV-127: Chunked grid: `Vec<Arc<LineChunk>>` with `CHUNK_SIZE=256`.
- INV-200: SIGWINCH via signal-hook pipe, not raw handler.
- INV-201: TIOCGWINSZ on controlling TTY, not PTY master.
- INV-202: Window size follows smallest-client policy.
- INV-203: Grid reflow marks all lines dirty.
- INV-210: SIGCHLD via signal-hook pipe.
- INV-211: waitpid loop (not single call).
- INV-212: Only IO thread reaps children.
- INV-213: Dead panes remain visible until explicitly closed.
- INV-220: Graphics passthrough disabled by default.
- INV-230: No signal handlers for SIGTSTP/SIGCONT/SIGPIPE.
- INV-231: SA_RESTART for all signal handlers.

## Layout Engine Invariants

- Layout tree uses `LayoutCell` nodes with `LayoutType` enum.
- Minimum pane size: 2 columns x 1 row.
- 7 built-in layout algorithms + custom layout strings.
- Border separator is 1 cell wide/tall.
- Resize redistribution is proportional.

## What NOT to Do

- Do NOT use `unsafe` outside `mux-pty` and `mux-ffi`.
- Do NOT use `unwrap()` or `expect()` in library code.
- Do NOT use `SmallVec` for line cells.
- Do NOT use `async` in the kernel thread -- it is `std::thread` only.
- Do NOT share mutable state between threads. The kernel owns all mutable state.
- Do NOT modify grid cells directly; always go through `line_mut()`.
- Do NOT store scrollback as `Vec<Cell>`; use `Arc<LineChunk>` COW.
- Do NOT use `#[no_mangle]` directly -- use `#[unsafe(no_mangle)]` (edition 2024).
- Do NOT use crossterm. TermForge uses nix + libc for terminal I/O.
- Do NOT use vt100 or termwiz for VT parsing. TermForge has its own parser.
- Do NOT install signal handlers for SIGTSTP, SIGCONT, SIGPIPE, SIGUSR1/2.
- Do NOT enable graphics passthrough by default (security).
