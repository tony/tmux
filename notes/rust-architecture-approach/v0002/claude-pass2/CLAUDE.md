# CLAUDE.md

## Project

TermForge is a Rust terminal multiplexer SDK targeting tmux v3.5a protocol behavior.
The scaffold uses a deterministic Sans-IO kernel + double-buffer diff renderer architecture.

## MSRV

Rust 2024 edition, MSRV 1.85.

## Coding Rules

- All crates use `mux-` prefix.
- `#![forbid(unsafe_code)]` everywhere except `mux-pty` and `mux-ffi`.
- Public APIs never panic and never have hidden side effects.
- Workspace clippy lints deny `unwrap_used`, `expect_used`, `panic`.
- Use `thiserror` for library errors; `anyhow` only in binaries.
- Prefer `const fn` where possible.
- All public items must have `#[must_use]` where applicable.
- Functions returning `Result` must have `# Errors` doc section.

## Comment Guidance

- Explain **why**, not **what**.
- Use comments for invariants, non-obvious algorithm choices, compatibility constraints.
- Do not narrate straightforward assignments or control flow.
- When documenting tmux-compat decisions, include explicit behavior contract and tradeoff.
- Reference tmux source file and line for invariant verification (e.g. "tmux.h:742").

## Architecture Invariants

- The Kernel is strictly Sans-IO: signal handlers enqueue events; reducers emit effects.
- The Kernel runs on a single thread; all mutable state lives there.
- Grid lines use Arc-based COW for snapshot isolation.
- CellFlags bit values match tmux's GRID_FLAG_* exactly (INV-119, tmux.h:742-749).
- Graphics passthrough is disabled by default (INV-220).
- Panic recovery uses catch_unwind at FFI boundaries (INV-117).

## Layout Invariants

- Sum of child widths/heights must exactly match parent size after redistribution.
- Minimum pane size is never below 2 cols x 1 row.
- Built-ins always available: even-horizontal, even-vertical, main-horizontal, main-vertical, tiled.
- Custom layout strings must pass checksum validation before acceptance.

## Signal/Process Invariants

- SIGWINCH handling starts with terminal size query and ends in render.
- SIGCHLD handling always drains waitpid(-1, WNOHANG) until empty.
- Kernel remains Sans-IO: signal handlers enqueue events; reducers emit effects.

## Copy Mode Invariants

- Key table is explicit (Vi or Emacs) and mode-local.
- Selection mode is explicit (Character, Line, Block).
- Search cursor movement is deterministic and testable.
- Buffer ring is bounded (max 50 entries by default).

## Render Pipeline Invariants

- Flood fairness: per-pane row quota with stride rotation prevents starvation.
- CompositeGrid double-buffer: write to "next", diff against "prev".
- Wide-char invalidation: padding cells emitted with their parent wide char.
- SGR optimization: only emit attributes when they change.
- CUP optimization: skip cursor move for adjacent cells.

## CRDT Strategy

- Vector clock dominance for causal ordering of collaborative edits.
- Last-Writer-Wins (LWW) register for individual cell conflicts.
- Convergence verified via proptest with concurrent mutation generators.

## Testing Rules

- Every crate has `#[cfg(test)] mod tests` with at least 3 unit tests.
- Use `Clock::manual()` for deterministic time in all tests.
- Use `TestServer` from mux-test-support for integration tests.
- Property tests (proptest) for layout dimension sums and colour pack/unpack.
