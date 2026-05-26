# TTY Rendering and Status — Output Pipeline

## The tty Layer

The `struct tty` (https://github.com/tmux/tmux/blob/master/tty.c and tty-*.c siblings) represents the output side for one client.

Key responsibilities:
- Terminfo / feature detection (`tty_term.c`, `tty-features.c`)
- Attribute and colour mapping (including 256-colour, RGB, italics, etc.)
- The actual low-level write path and cursor/scroll/erase optimization
- ACS (alternate character set) translation (`tty-acs.c`)
- Key input decoding on the way back in (`tty-keys.c`)

Each client has exactly one `tty` struct once it has identified its terminal.

## screen_write_ctx and screen_redraw

Most high-level painting goes through `screen_write_ctx` (https://github.com/tmux/tmux/blob/master/screen-write.c).

This context wraps a target `screen` (or sometimes a grid directly) and provides operations such as:
- `screen_write_putc`, `screen_write_nputs`
- Cursor movement, scrolling regions, clearing
- Cell attribute changes (via `grid_cell` / `style`)
- Copy / paste of regions

`screen_redraw.c` contains the higher-level logic that decides *what* to redraw for a client:
- Full window redraws
- Status line updates
- Border drawing for panes
- The "choose" overlays when a mode is active

It walks the layout tree, the current window's panes, and the client's view, emitting the minimal set of `screen_write_*` calls needed.

## How Deltas Become Bytes

The redraw engine ultimately produces a set of changed lines / cells. The tty layer then:
1. Compares against its idea of the client's current screen contents (tty has its own "backing" grid for the client).
2. Emits the shortest escape sequence that will make the client's terminal match (cursor addressing, SGR, scrolling, etc.).
3. For control-mode clients (`-CC`), it often emits structured `%output` or redraw notifications instead of raw bytes.

This is why attaching with `tmux attach -CC` gives you a very different (and much more efficient for machine consumption) output format.

## Redraw Triggers

Anything that can change what a client should see sets flags:
- `PANE_REDRAW` on a pane
- Window size or layout changes
- Status line options or content changes
- Alert bits on winlinks
- Client `pause_age` / suspended state

The server coalesces these and only runs the expensive redraw work when the client is ready (not suspended, not in the middle of a long command, etc.).

## Control vs Normal Clients

Normal clients get a faithful emulation of what a real terminal would see.

Control clients (and tmux's own popup/menu drawing) bypass much of the tty optimization and work directly with the higher-level screen/grid structures, which is why they can do precise mouse tracking and structured output that ordinary terminals cannot express.