# Interactive Modes — Chooser and Special Modes

## The Generic mode_tree Engine

`mode_tree.c` provides a reusable, keyboard-driven list + preview UI used by almost all the "choose" commands.

It handles:
- Vi / Emacs navigation and search within the list
- Collapsible tree nodes (for choose-tree)
- Preview pane on the right (or bottom)
- Custom key bindings per domain
- Sorting and filtering

Individual domains only have to supply:
- A list of items (with id, name, text, tags, etc.)
- Callbacks for drawing the preview
- Callbacks for what happens on "choose" (Enter) or custom keys

This is why `choose-buffer`, `choose-client`, `choose-tree`, `list-keys` in tree form, etc. all feel consistent and why adding a new chooser is relatively cheap.

## The window-*.c Special Panes

These are the concrete implementations that plug into (or sometimes bypass) mode_tree:

- `window-buffer.c` — paste buffers
- `window-client.c` — attached clients
- `window-clock.c` — the big digital clock (very small, doesn't use mode_tree)
- `window-customize.c` — the interactive options editor (`customize-mode`)
- `window-tree.c` — the powerful session/window/pane tree used by `choose-tree`

Each registers its own `window_mode` and is entered via the corresponding command.

## display-panes

`display-panes` (bound to `prefix q` by default) is a special non-list mode.

It overlays large numbers on every pane in the current window (or all windows with `-a`). The user can then type a number (or click) to select that pane.

Implementation is deliberately simple and self-contained in `cmd-display-panes.c` — it just needs to know the layout geometry and emit big characters in the right places. It does not use mode_tree.

## Why These Modes Exist

They solve the "I need to do something with a thing that is not the current pane" problem without forcing the user to remember cryptic commands or type long names.

Instead of `tmux swap-pane -s 3 -t 1`, most users do `prefix q`, look at the big numbers, type the source and target.

The tree modes give a visual, searchable, filterable view of the entire session/window/pane namespace that is hard to replicate with pure command-line tools.

## Relationship to Copy Mode

Copy mode and the chooser modes share the same `window_mode` infrastructure and the same screen_write painting path, but they are semantically different:
- Copy mode is about *inspecting history inside one pane*.
- Chooser modes are about *navigating and acting on tmux objects* (sessions, windows, panes, buffers, clients).

Both temporarily replace the normal pty-driven content of a pane with tmux-controlled content, which is why they are grouped together as "interactive modes".