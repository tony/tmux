# Terminal Emulation — Input State Machine

The parser lives in https://github.com/tmux/tmux/blob/master/input.c.

## State Table Organization

Each state is a `struct input_state` containing a name, optional enter/exit hooks, and a pointer to a table of `input_transition` entries (first/last byte range + handler + next state).

"Anywhere" transitions (0x18, 0x1a, 0x1b) are duplicated into every table so that CAN, SUB, and ESC always abort the current sequence and return to ground or esc_enter.

## Major States

- `ground` — normal printable characters and C0 controls that produce immediate grid output.
- `esc_enter` / `esc_intermediate` — single-character escapes and ESC I … sequences.
- `csi_enter` / `csi_parameter` / `csi_intermediate` / `csi_ignore` — the classic CSI [ params ; … finalbyte family (cursor moves, SGR, etc.).
- `dcs_*` family (enter, parameter, handler, escape, ignore) — device control strings. The handler state accumulates the payload until ST or a DCS-specific terminator; Sixel images are parsed here when enabled.
- `osc_string` — Operating System Command (title, clipboard, hyperlinks, etc.). Terminated by ST or BEL.
- `apc_string` and `rename_string` — two tmux-specific extensions treated like OSC.

## UTF-8 Handling

Bytes >= 0x80 are never fed to the state machine as C1 controls. `utf8.c` (and the combined-grapheme layer in `utf8-combined.c`) decodes them first; only well-formed codepoints reach the grid cell logic. Invalid sequences produce a replacement character and reset the UTF-8 accumulator.

## OSC / DCS Payload Rules Specific to tmux

- OSC 0–2 set the pane title (and therefore the window name unless the window has been manually renamed).
- OSC 8 is the hyperlink escape (stored in the `hyperlinks` tree on the screen).
- OSC 52 is clipboard access (guarded by `set-clipboard` option and the `allows` feature flag).
- DCS `q` … ST with sixel data is handed to `image-sixel.c` when the feature is compiled in.

## Error Recovery

Any transition that lands in an ignore state consumes bytes until a final character or ST is seen, then returns to ground. This prevents a single malformed sequence from permanently desynchronizing the parser — a deliberate robustness choice inherited from the original VT100 design.

The entire machine is deliberately *not* a push-down automaton; all state lives in the `input_ctx` struct (parameter buffer, intermediate bytes, current grid cell being built, etc.). This keeps the code simple enough to reason about while still supporting the full range of sequences that real applications emit.