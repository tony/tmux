# CLAUDE.md

## Project

TermForge is a Rust multiplexer SDK targeting tmux v8 protocol behavior.
This scaffold focuses on deterministic Sans-IO kernel + diff renderer architecture.

## Coding Rules

- Rust 2021, MSRV 1.75.0.
- All crates keep `mux-` prefix.
- `#![forbid(unsafe_code)]` outside `mux-pty`.
- Public APIs avoid panics and hidden side effects.
- Workspace clippy lints are set to deny `unwrap_used`, `expect_used`, `panic`.

## Comment Guidance

- Explain **why**, not **what**.
- Use comments for invariants, non-obvious algorithm choices, compatibility constraints, and failure-mode handling.
- Do not narrate straightforward assignments or control flow.
- When documenting tmux-compat decisions, include explicit behavior contract and tradeoff.

## Layout Invariants

- Geometry is represented by a tree (`LayoutNode`) and solved into pane `Rect`s.
- Sum of child widths/heights must exactly match parent size after redistribution.
- Minimum pane size is never below 1 cell in either axis.
- Resize redistribution is deterministic for equal inputs.
- Built-ins are always available: `even-horizontal`, `even-vertical`, `main-horizontal`, `main-vertical`, `tiled`.
- Custom layout strings must pass checksum validation before acceptance.

## Signal/Process Invariants

- `SIGWINCH` handling starts with terminal size query and ends in render.
- `SIGCHLD` handling always drains `waitpid(-1, WNOHANG)` until empty.
- Kernel remains Sans-IO: signal handlers enqueue events; reducers emit effects.

## Copy Mode Invariants

- Key table is explicit (`Vi` or `Emacs`) and mode-local.
- Selection mode is explicit (`Character`, `Line`, `Block`).
- Search cursor movement is deterministic and testable.
- Buffer ring is bounded.

