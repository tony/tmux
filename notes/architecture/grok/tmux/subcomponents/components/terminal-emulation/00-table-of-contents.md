# Terminal Emulation — Overview

The subsystem that turns bytes coming out of a child pty into a grid that can be rendered, searched, or copied.

Primary files: https://github.com/tmux/tmux/blob/master/input.c, grid*.c, screen*.c, utf8*.c (plus image*.c when sixel is enabled).

## Core Pipeline (per pane)

```
pty master fd (window_pane.fd)
        │
        ▼
input_ctx (one per pane) — state machine in input.c
        │  (produces grid cells, cursor moves, attributes, etc.)
        ▼
grid + screen (the model)
        │
        ▼
screen-write / screen-redraw  (when a client needs to see it)
        │
        ▼
tty output (the client's terminal)
```

## The Parser

`input.c` implements a table-driven parser based on Paul Williams' description (https://vt100.net/emu/dec_ansi_parser). It has explicit states for ground, escape, CSI, DCS, OSC, APC, and a "consume ST" helper. C0 controls (0x18, 0x1a, 0x1b) are handled from *any* state and force a return to ground. UTF-8 is decoded before the state machine sees the bytes; combining characters are normalized in the grid layer.

tmux extensions visible in the parser:
- OSC can be terminated by BEL (0x07) as well as ST.
- APC is treated similarly to OSC (used by some terminals for title hacks).
- A special "rename string" state for the ancient ESC k … ESC \ window rename sequence.
- DCS passthrough for Sixel and other device control strings (when enabled).

## Grid vs Screen

- `grid` (https://github.com/tmux/tmux/blob/master/grid.c) is the sparse, history-aware storage. Lines are allocated on demand; cells are `grid_cell` + extended entries for wide characters / hyperlinks / images.
- `screen` adds the viewport (cursor position, scroll region, margins, selection, titles, hyperlinks list) and the "visible" snapshot that redraw uses.

`grid-view.c` and `grid-reader.c` provide the read-only iterators used by copy mode and capture-pane.

## Related Components

- **Pane Lifecycle**: each `window_pane` owns exactly one `input_ctx` and one `screen`.
- **TTY Rendering**: screen-write operations eventually produce the bytes that `tty.c` writes to client fds.
- **Interactive Modes**: copy mode walks the grid history directly; it never goes back through the parser.

## Topics in This Component

- `input-state-machine.md` — the 15–17 states, transition tables, C0 anywhere rules, and how OSC/DCS/APC payloads are accumulated.
- `grid-screen-model.md` — cell storage, combining characters, selection, hyperlink and image metadata, the difference between the history grid and the on-screen screen.