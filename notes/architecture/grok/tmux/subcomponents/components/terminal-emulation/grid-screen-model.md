# Terminal Emulation — Grid and Screen Model

## grid.c — Persistent History Storage

A `grid` is an array of `grid_line` pointers. Each line contains:
- a `grid_cell_entry` array (small cells are stored inline; large ones or those with hyperlinks/images point into separate `grid_extd_entry` and `grid_cell` pools);
- a `cellsize` and `exdsize` for the used portion;
- a `flags` word (wrapped, extended, etc.).

Lines are allocated on demand and never shrink. The history limit (`history-limit` option) is enforced by dropping old lines from the front when the total exceeds the configured value. This is why `capture-pane -S -` can retrieve more lines than are currently visible.

`grid_view_*` functions provide a viewport onto a sub-range of the grid (used heavily by copy mode and the redraw engine).

## screen.c — The "Live" Viewport

A `screen` adds:
- cursor position (`cx`, `cy`);
- scroll region (`rupper`, `rlower`);
- margins, origin mode, insert/replace mode;
- current SGR attributes (the `grid_cell` that will be used for the next write);
- selection (`screen_sel`);
- list of titles (for the pane title stack);
- the `hyperlinks` tree (URI + id pairs referenced by cells);
- progress bar and other transient UI state.

`screen_write_*` functions (screen-write.c) are the only mutators; they translate high-level operations (putc, cursor move, erase, etc.) into grid mutations while respecting the current mode bits.

## Redraw Triggers

A pane sets `PANE_REDRAW` (or the window marks a full redraw) when:
- the parser wrote visible cells;
- the cursor moved;
- a bell or visual bell occurred;
- a mode change (mouse, alternate screen, etc.) that affects what the client should see;
- the layout or pane size changed.

The redraw engine in `screen-redraw.c` then walks the client list and, for each client that can see the pane, emits the minimal terminal sequence delta (or a full repaint in control mode).

## Selection, Hyperlinks, Images

Selections are stored as character ranges on the screen; they are converted to paste-buffer data only when the user explicitly copies. Hyperlinks (OSC 8) and sixel images are reference-counted in side tables so that cells can cheaply point at them without duplicating the URI or pixel data on every cell.

This separation of the persistent grid (history + cells) from the transient screen (cursor, modes, selection) is what allows tmux to support unlimited scrollback, alternate screens, and copy mode without ever re-parsing the original byte stream.