# Interactive Modes — Overview

When a pane is not behaving like a normal terminal, it is usually because an "interactive mode" has taken over its screen area.

Primary implementation files (https://github.com/tmux/tmux):
- `window-copy.c` (the most important)
- `mode-tree.c`
- `window-*.c` (buffer, client, clock, customize, tree)
- `cmd-choose-tree.c`, `cmd-display-panes.c`, `cmd-copy-mode.c`

## The window_mode Abstraction

Every interactive mode implements the small `struct window_mode` interface (https://github.com/tmux/tmux/blob/master/tmux.h):

```c
struct window_mode {
    const char *name;
    struct screen *(*init)(...);
    void (*free)(...);
    void (*resize)(...);
    void (*key)(...);
    ...
};
```

A `window_mode_entry` is pushed onto a per-pane `TAILQ` of modes. The topmost mode receives all keys sent to that pane and owns the screen that is displayed instead of the normal grid.

This stack design allows temporary modes (search inside copy mode, etc.) and clean nesting.

## The Big One: Copy Mode

`window_copy_mode` (window-copy.c) is the workhorse. It provides:
- Vi and Emacs key tables
- Scrollback navigation
- Search (plain and regex, forward/back)
- Selection (character / line / block)
- Pipe / copy to paste buffer or command
- Jump to previous output, etc.

It is entered via `copy-mode`, `copy-mode -u`, or various mouse actions, and is the primary way users interact with history.

## Chooser / Tree Modes

Several commands replace the pane content with a navigable list:
- `choose-tree`, `choose-window`, `choose-session`
- `list-buffers`, `choose-buffer`
- `list-clients`, etc.

These are all built on the generic `mode_tree.c` engine, which provides a consistent vi/emacs navigation UI, filtering, sorting, and preview pane. The individual `window-*.c` files supply the data source and the actions for each domain.

`display-panes` (the big numbered overlay you get with `prefix q`) is a simpler, special-purpose mode implemented directly in `cmd-display-panes.c`.

## Why Modes Live on Panes

Modes are attached to `window_pane`, not to `client` or `session`. This means:
- Different clients attached to the same session can be in different modes (or no mode) on the same pane.
- When you switch away from a window and come back, the mode state (scroll position, selection, search string) is still there for that pane.
- A pane can have a mode active even if no client is currently looking at it.

## Related Components

- **Command System**: all the `copy-mode`, `choose-*`, `display-*` commands live here.
- **Core Entities**: modes read the grid history and can affect which pane/window is active.
- **TTY Rendering**: modes paint exclusively through `screen_write_ctx`; they never touch the real pty input path.
- **Terminal Emulation**: copy mode is the main consumer of the full history grid.

## Topics in This Component

- `copy-mode.md` — the window_copy implementation, its key tables, selection model, search, and how it exports data to paste buffers.
- `chooser-modes.md` — the mode_tree generic engine and how the various `window-*.c` special panes (buffer, client, clock, customize, tree) plug into it, plus display-panes.