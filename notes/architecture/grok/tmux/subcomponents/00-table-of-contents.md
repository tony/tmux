# tmux Subcomponents

All C references are to the public repository at https://github.com/tmux/tmux.

- Stable tag used for released symbols: `3.6b`
- Development tree used for the source inspection: `master` (corresponding to the `AC_INIT([tmux], next-3.7)` checkout at the time of analysis)

## System Overview

tmux is a single-binary (see `Makefile.am:85` `dist_tmux_SOURCES`), libevent-driven terminal multiplexer. Every invocation enters through `main()` in `tmux.c` and immediately becomes a client (`client_main`). The first client that cannot connect to the per-label socket takes a lock and calls `server_start`, which performs `proc_fork_and_daemon` and becomes the long-lived server process.

The server owns the global state (sessions RB tree, windows RB tree, clients TAILQ). Clients communicate with the server exclusively over Unix-domain sockets using the imsg-based protocol defined in `tmux-protocol.h` (current `PROTOCOL_VERSION 8`).

There are no internal shared libraries and no separate daemons; the "components" below are the major subsystems visible in the flat `.c` + `compat/` layout.

## The Eight Primary Subcomponents

1. **Client-Server IPC** (`components/client-server-ipc/`)  
   Process lifecycle, `tmuxproc`/`tmuxpeer`, imsg transport, `MSG_*` message types, identification handshake, and server exit conditions. Entry points: `client_main`, `server_start`, `proc_loop`.

2. **Core Entities** (`components/core-entities/`)  
   The runtime object graph: `client`, `session`, `winlink`, `window`, `window_pane`, `layout_cell`. `winlink` is the only many-to-many bridge. All lookup collections (RB trees for id/name, TAILQs for ordering and MRU stacks) are defined in `tmux.h`.

3. **Terminal Emulation** (`components/terminal-emulation/`)  
   The path from pty-master bytes to screen content. `input.c` implements a table-driven DEC ANSI parser (Paul Williams model + tmux extensions for OSC, APC, DCS, UTF-8). `grid.c`, `screen.c`, and UTF-8/combining layers sit beneath it. Sixel support is conditional.

4. **Command System** (`components/command-system/`)  
   tmux's primary user interface. 65 files matching `cmd-*.c` plus `cmd.c`, `cmd-queue.c`, and `cmd-parse.y`. Commands are parsed, turned into `cmdq_item`s, and executed on per-client or global queues that support grouping and asynchronous waiting.

5. **TTY Rendering and Status** (`components/tty-rendering-and-status/`)  
   Output side and UI furniture. `tty*.c` family (draw, terminfo, features, keys, ACS), `screen-write.c`/`screen-redraw.c`, the `#{...}` format mini-language in `format.c`, status line, popups, and menus.

6. **Pane Lifecycle and Jobs** (`components/pane-lifecycle-and-jobs/`)  
   Creation and child-process management. `spawn.c` (the `fdforkpty` / `getptmfd` path), `job.c` (bufferevent-wrapped async children used by `run-shell`, `pipe-pane`, control mode), pane pid/fd storage, and the per-pane mode stack.

7. **Interactive Modes** (`components/interactive-modes/`)  
   Temporary replacement of a pane's normal terminal view. `window-copy.c` (the dominant copy/search/scroll mode), `mode-tree.c` (generic chooser engine), and the six special `window-*.c` panes (buffer, client, clock, customize, tree). Triggered by `choose-*` and `display-*` commands.

8. **Configuration and Options** (`components/configuration-options/`)  
   Three scoped option trees (`global_options`, `global_s_options`, `global_w_options`), table-driven option definitions, `cfg.c` loading (system + user files, `-f` overrides), environment handling, and key-binding tables.

Build system, portability shims (`compat/` with 42 files), and platform `osdep-*.c` selection are described in the methodology note below and referenced from the IPC and jobs pages rather than given a ninth top-level component.

## Methodology

Findings were obtained exclusively by source inspection of a clean checkout using `fd`, `rg`, and `ag` (no `grep` or `find`):

- File classification and counts: `fd --glob` and `Makefile.am:85` `dist_tmux_SOURCES`.
- Struct definitions and ownership: direct extraction from `tmux.h`.
- Control flow: targeted reads of `tmux.c:556` (always `client_main`), `client.c:232`, `server.c:176` (`server_start`), `proc.c:36` (`tmuxproc`), `input.c` state tables, `cmd-queue.c`, `spawn.c:386` (`fdforkpty`), etc.
- Cross-cutting numbers (65 command files, 8+ window modes, etc.) were verified against the build list at the time of inspection.

No runtime debugging or strace sessions were used; every claim below is traceable to a specific file and, via the GitHub links, to a public commit.

## Source References

- Repository: https://github.com/tmux/tmux
- Primary stable tag for this document: `3.6b`
- Development tip matching the inspected tree: `master`
- Example canonical links appear throughout the component pages (e.g. https://github.com/tmux/tmux/blob/3.6b/tmux.h, https://github.com/tmux/tmux/blob/master/server.c).

All local paths, PII, dates, and brittle line numbers have been omitted per the documentation standards applied to this research tree.

## Navigation

- Start with any component's `00-table-of-contents.md` for a one-page summary + links to its two focused topic pages.
- Cross-links between components appear in the "Related Components" sections.
- For the Rust-oriented re-architecture proposals that motivated this breakdown, see the sibling `rust-architecture-approach/v0005/synthesized/crates/` tree (mux-kernel, mux-grid, mux-pty, mux-proto, mux-format, etc.).

---

*This page and all pages under `components/` were generated as a single attributable contribution under the `architecture/grok/` namespace.*