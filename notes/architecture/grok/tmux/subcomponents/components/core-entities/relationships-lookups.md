# Core Entities — Relationships and Lookups

## The winlink Pivot

Because a window can be linked into multiple sessions, the only reliable way to answer "which sessions see this window?" is to walk `window->last_panes` or the `wentry` TAILQ on each winlink that points at it. Conversely, "which windows does this session contain?" is answered by walking the session's `windows` RB tree (ordered by index) or its `lastw` stack.

`curw` on the session and `active` on the window are the two "focus" pointers that almost every command and redraw path consult first.

## Navigation Idioms Used by Commands

- `cmd_find_current` / `cmd_find_target` (cmd-find.c) start from the client's current `session` / `curw` and apply modifiers (`-t`, `-s`, index offsets, "last", "next", "previous").
- MRU stacks (`lastw`, `last_panes`) are rotated on focus changes so that `last-window` / `last-pane` are O(1) list rotations rather than searches.
- The `WINDOW_SIZE_LARGEST` / `SMALLEST` / `MANUAL` / `LATEST` logic in `resize.c` and `layout*` walks the winlink list of a window to decide what size the window should actually be rendered at.

## Alert Propagation

Bell / activity / silence flags live on the `winlink` (not the window), because the same window may be "alerting" in one session's status line but silent in another. `alerts.c` walks the `wentry` list when a pane produces output or a bell, setting the appropriate bit on each affected winlink and notifying the owning sessions.

## Focus and Zoom

`PANE_ZOOMED` and `PANE_FOCUSED` bits on `window_pane` are set when the layout engine applies a zoomed layout cell. Only one pane per window can be zoomed at a time; the saved layout root is kept so that `resize-pane -Z` can toggle back without losing the previous split ratios.

## Cross-Component Effects

Any mutation of the graph (new session, link-window, kill-pane, move-window, rotate-window, etc.) must:
1. Update the relevant RB/TAILQ links.
2. Mark panes/windows for redraw (`PANE_REDRAW`, window size change).
3. Possibly renumber indexes and fix `curw` / `active` pointers.
4. Notify any clients that are viewing the affected session (via control-mode or status redraw).

Because the graph is small and mutated only on the server event loop, there are no locks; the single-threaded `server_loop` + command queue draining provides the necessary serialization.

This design keeps the entity model extremely simple while still supporting the "one window, many views" and "many sessions, shared windows" use cases that distinguish tmux from simpler multiplexers.