# Configuration and Options — Scopes and Loading

## The Option Trees in Memory

At runtime there are three "global" roots created early in `tmux.c`:

```c
global_options   = options_create(NULL);   /* server scope */
global_s_options = options_create(NULL);   /* session defaults */
global_w_options = options_create(NULL);   /* window defaults */
```

Every `session` and every `window` then gets its own `options` tree created with one of the global trees as parent. Lookups walk the parent chain until they find a value or hit the ultimate default in the table.

Panes currently share their window's options for most things (the model has been slowly moving toward more pane options over the years).

This design gives classic "set -g" vs "set" semantics with very little code at the use sites — they just ask the object for the option and the inheritance happens automatically.

## Loading Order (cfg.c)

When a server starts (or `start-server` / `source-file` is executed):

1. The compiled-in default path is expanded (from `TMUX_CONF` in the configure step).
2. Each file in the list is attempted in order.
3. `-f` on the command line replaces the entire list (or can be given multiple times).
4. Files are parsed with `load_cfg` / the command parser.
5. Commands that are not "immediate" (most `set` / `bind` / `source` etc.) are queued and executed in the normal command machinery after the server is fully up.

This ordering matters for things like:
- `set -g default-shell` affecting later `new-session` commands in the same file.
- `source-file` inside a config file (recursive sourcing).
- The `TMUX` environment variable being set before user config runs.

## Quiet and "Not Found" Handling

`source-file -q` suppresses "file not found" errors (useful in default configs that try `~/.tmux.conf` and XDG locations).

`source-file -n` does a dry run (parse only).

These flags are frequently used in system-wide or distribution configs so that a missing user file does not produce scary error messages on every new tmux invocation.

## Live Option Changes

When you do `set-option -g status-position top` in a running server:
- The value is written into the appropriate options tree.
- The option table's change callback (if any) is invoked.
- For status-related options this usually triggers a redraw of all clients attached to sessions that inherit that option.
- Some options (e.g. `mouse`, `xterm-keys`, `focus-events`) also cause the server to push updated `MSG_IDENTIFY_FEATURES` or similar state to existing clients.

This is why you can reconfigure key bindings or the status line and see the effect immediately without restarting anything.