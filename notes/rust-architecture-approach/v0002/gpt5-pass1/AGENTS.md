# AGENTS.md

1. Read `CLAUDE.md` first; enforce invariants before adding features.
2. Keep kernel Sans-IO: no direct terminal, signal, process, socket side effects inside reducer.
3. Copy mode rules:
   - Support both `Vi` and `Emacs` key tables.
   - Keep explicit selection modes (`Character`, `Line`, `Block`).
   - Keep scrollback search deterministic and test-covered.
   - Bound copy buffer ring and avoid unbounded allocations.
4. Graphics protocol stance:
   - Sixel/iTerm2/Kitty payloads are passthrough-only in Pass 1.
   - Do not decode or rasterize graphics in kernel/render path yet.
   - Preserve byte transparency where tmux passthrough compatibility is required.
5. Rendering path rules:
   - Compose pane surfaces first, diff second, emit third.
   - Coalesce spans and respect per-frame output budget.
   - Prevent one flooding pane from starving others.
6. Signal handling rules:
   - `SIGWINCH` must flow through size query -> layout solve -> `TIOCSWINSZ` effects -> render.
   - `SIGCHLD` handling drains all exited children via non-blocking reap loop.
7. Run `cargo check --workspace` and `cargo test --workspace` before finalizing.
