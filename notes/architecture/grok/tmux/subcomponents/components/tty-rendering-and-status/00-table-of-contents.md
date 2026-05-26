# TTY Rendering and Status — Overview

This component handles everything that turns internal state into bytes sent to client terminals, plus the status line, popups, and menus that make up tmux's own UI chrome.

Primary files (https://github.com/tmux/tmux):
- `tty*.c` family
- `screen-write.c`, `screen-redraw.c`
- `format.c`, `format-draw.c`
- `status.c`
- `popup.c`, `menu.c`
- `style.c`, `attributes.c`

## Role in the System

After the input parser and grid have updated a pane's content, and after the command system or layout engine has changed focus or sizes, something must produce the escape sequences that the client's terminal actually displays. This component owns that output path plus the "tmux-owned" UI elements (status line, popups, choose trees when they are not using a full window mode).

## Key Abstractions

- `tty` + `tty_term` — per-client output state, terminfo cache, feature detection, the actual write path.
- `screen_write_ctx` — the high-level drawing API used by everything that wants to paint (panes, status, popups, copy mode).
- `format_tree` — the `#{...}` mini-language evaluator.
- Status line, popup, and menu subsystems that compose the above.

## Data Flow (Simplified)

1. Something marks a client or pane as needing redraw (PANE_REDRAW, window size change, status update, etc.).
2. `server_client_loop` / redraw logic calls into `screen_redraw_*` or direct `screen_write_*` operations.
3. These operations build a list of `screen_write_citem` / `screen_write_cline` changes.
4. The tty layer turns the resulting grid deltas into the smallest possible terminal sequence (or full repaint for control clients) and writes it to the client's `out_fd`.
5. Format strings (status, pane titles, etc.) are expanded on demand using the current `format_tree` context.

## Related Components

- **Terminal Emulation**: the grid/screen model that the redraw engine reads from.
- **Core Entities**: size changes, focus changes, and alert flags on panes/windows/sessions drive most redraws.
- **Command System**: many commands (display-message, popup, etc.) ultimately paint through this layer.
- **Interactive Modes**: copy mode and choosers use screen_write_ctx heavily while active.

## Topics in This Component

- `output-pipeline.md` — the tty layer, screen-write / screen-redraw, how deltas become bytes on the wire, and the difference between normal and control clients.
- `format-and-status.md` — the `#{...}` format language, status line configuration and drawing, popups, menus, and how options + formats interact.