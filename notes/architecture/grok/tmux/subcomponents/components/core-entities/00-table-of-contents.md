# Core Entities — Overview

The live object graph that the server maintains for every attached client.

All struct definitions are in https://github.com/tmux/tmux/blob/master/tmux.h.

## The Central Six Structs

- `client` — an attached (or control-mode) front-end.
- `session` — a named, persistent collection of windows.
- `winlink` — the only many-to-many relationship (a window can appear at different indexes in multiple sessions).
- `window` — a container of panes plus one active layout tree.
- `window_pane` — one terminal instance (pty, grid, input parser, mode stack).
- `layout_cell` — the binary tree node used for pane splits and sizing.

## Why winlink Exists

A direct `session → window` pointer would prevent the same window from being present in two sessions at different indexes or with different "last visited" stacks. `winlink` carries the per-session index, alert flags, and the two TAILQ entries (`wentry` for the window's list of sessions, `sentry` for the session's list of windows) plus the RB entry for fast lookup by index inside a session.

## Collection Patterns (tmux.h)

- `RB_HEAD(sessions, session)` + `RB_HEAD(windows, window)` — global id-based lookup.
- `RB_HEAD(winlinks, winlink)` — per-session lookup by index.
- Multiple `TAILQ_HEAD` stacks for MRU order (`lastw`, `last_panes`).
- `TAILQ` for ordered lists that must support O(1) splice or insertion at ends (`panes` inside a window).

## Related Components

- **Client-Server IPC**: a `client` owns the `tmuxpeer` and the `cmdq_list` that feeds commands into the entity graph.
- **Command System**: most commands resolve a target via `cmd_find_state` that walks the above collections.
- **Pane Lifecycle and Jobs**: `window_pane` owns the pty fd/pid and the head of its mode stack.
- **Terminal Emulation**: each `window_pane` owns exactly one `input_ctx` and one `screen`/`grid`.

## Topics in This Component

- `data-model.md` — full field-level view of the six structs, their back-pointers, and the flag bits that drive redraw and exit behavior.
- `relationships-lookups.md` — how `curw`/`lastw`, the two directions on `winlink`, active pane, and layout root are used at runtime for navigation, focus, and resize.