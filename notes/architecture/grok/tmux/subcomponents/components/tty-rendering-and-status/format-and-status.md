# TTY Rendering and Status — Format Language and Status Line

## The #{...} Format Mini-Language

`format.c` and `format-draw.c` implement tmux's ubiquitous string interpolation language.

A `format_tree` is built with a set of "format jobs" and a context (usually a client + session + window + pane). Then `format_expand` (or the faster `format_single`) walks the string, expanding:
- Simple variables (`#{session_name}`, `#{pane_current_path}`)
- Conditionals (`#{?cond,true,false}`)
- Modifiers (`#{=10:session_name}`, `#{s/regex/repl/}`, `#{||:a,b}`, etc.)
- Nested formats and custom user variables

Format expansion happens on demand (status line, pane titles, display-message, if-shell arguments with `-F`, etc.). It is deliberately pure given its input context — the same string with the same context always produces the same result.

## Status Line

The status line (https://github.com/tmux/tmux/blob/master/status.c) is configured by:
- `status` (on/off)
- `status-position` (top/bottom)
- `status-format` (array of format strings, one per line)
- Many `status-style-*` and `window-status-*` options

At redraw time the status code:
1. Builds a fresh `format_tree` for the client.
2. Expands each line of `status-format`.
3. Paints the result using `screen_write_ctx` into a small screen that sits above or below the main window area.
4. The tty layer then treats the status area like any other part of the client's visible screen for delta calculation.

Because status lines are formats, they can contain live data (load average, git branch via user scripts in `status-right`, battery via `run-shell` + variables, etc.) and update on a timer or on certain events.

## Popups and Menus

`popup.c` and `menu.c` (plus `cmd-display-menu.c`, `cmd-display-popup.c`) use the same screen_write + format machinery, but they render into temporary overlay screens that the redraw engine composites on top of the normal window content.

They support their own nested key bindings and can be driven entirely from formats and commands without any C code changes for new menu items.

## Style and Attributes

`style.c` and `attributes.c` provide the bridge between user-visible style options (e.g. `status-style bg=green,fg=black,bold`) and the `grid_cell` attributes that the rest of the system uses.

They also handle the "default" inheritance rules and the various `pane-border-style`, `message-style`, `mode-style` etc. options.

## Why This Design Matters

The format language + status + popup system is one of the main reasons tmux feels so configurable without requiring users to write C or even Lua. Almost every piece of text the user sees (except the raw terminal content inside panes) flows through the same small, well-tested expansion engine. This is also why adding new variables is usually just a matter of adding an entry in the format table rather than touching dozens of call sites.