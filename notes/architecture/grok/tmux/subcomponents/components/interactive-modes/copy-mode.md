# Interactive Modes — Copy Mode

## Entry and State

`window_copy_init` (https://github.com/tmux/tmux/blob/master/window-copy.c) allocates a private `struct window_copy_mode_data` and a temporary `screen` that will be shown instead of the pane's normal grid.

The mode is pushed onto the pane's mode stack with `window_pane_set_mode`. From that moment until the mode is popped, all keys sent to the pane go to `window_copy_key` and the pane's visible screen comes from the mode's screen, not the real history.

## Key Handling

Copy mode maintains two key tables (`mode-key-vi` and `mode-key-emacs`, configurable via options).

On each key it looks up the binding and dispatches to one of a large set of `window_copy_*` functions:
- Movement: `cursor_up`, `cursor_down`, `page_up`, `page_down`, `history_top`, `goto_line`, etc.
- Search: `search_forward`, `search_backward`, `search_again`
- Selection: `start_selection`, `rectangle_toggle`, `cursor_jump`
- Actions: `copy_pipe`, `copy_selection`, `cancel`

Many of these are also exposed as `send-keys -X` commands so users (and plugins) can bind keys outside copy mode or create custom workflows.

## Selection Model

Selection is stored as start and end positions in the mode's coordinate space (which maps onto the underlying grid history plus the visible screen).

There are three styles:
- character (default)
- line
- rectangle (block)

When the user copies, the selected region is turned into a string (respecting rectangle mode) and placed into a paste buffer (or piped to a command). The selection is then cleared unless the user asked to keep it.

## Search

Search is implemented by walking the grid history lines (using the grid reader/view APIs) looking for the pattern. Results are highlighted by temporarily setting the `GRID_ATTR_SEARCH` attribute on matching cells.

The search string and direction are part of the mode state, so `n` / `N` (or the Emacs equivalents) work naturally.

## Integration with the Rest of tmux

- While in copy mode the pane still receives output from its child process; the grid keeps growing, but the user is looking at a frozen snapshot controlled by the mode.
- `copy-mode -u` enters the mode and immediately pages up — the common "I just ran a long command and want to see the output" gesture.
- Mouse selection in the pane (when `mouse` option is on) can automatically enter copy mode and start a selection.
- Exiting the mode (q, Escape, or Enter after a copy in some configurations) pops the mode entry and returns the pane to normal terminal behavior.

Copy mode is deliberately the only interactive mode that most users ever see, which is why it receives so much attention in the key binding tables and why its implementation is by far the largest of the mode files.