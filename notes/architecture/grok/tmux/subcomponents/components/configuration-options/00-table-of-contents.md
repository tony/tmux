# Configuration and Options — Overview

tmux has a large, hierarchical, live configuration system that affects almost every other component.

Core files (https://github.com/tmux/tmux):
- `options.c`, `options-table.c`
- `cfg.c`
- `environ.c`
- `key-bindings.c`
- `style.c` (for style options)

## Three Scopes

tmux maintains three root option trees:

- `global_options` — server-wide (e.g. `exit-empty`, `history-limit` default, `default-terminal`)
- `global_s_options` — session defaults
- `global_w_options` — window defaults

When a new session or window is created, it gets a copy of the current global defaults for its scope, then any per-object overrides are applied. This gives the familiar "global → session → window → pane" inheritance for many settings.

## Options Table

`options-table.c` is a large static table that declares every known option:
- name
- scope (server / session / window / pane / ... )
- type (string, number, flag, choice, style, etc.)
- default value
- change callback (many options have side effects when changed)

This table is what makes `show-options -a`, `set-option -a`, tab-completion, and the man page stay in sync. Adding a new option is usually just adding one entry to this table plus the code that actually uses the value.

## Loading and cfg.c

`cfg.c` handles:
- The default search path (`TMUX_CONF` from configure, usually `/etc/tmux.conf` + `~/.tmux.conf` + XDG paths)
- `-f` overrides on the command line
- `source-file` (including `-nq` for "not found is ok" and `-q` for quiet)
- `start-server` vs normal client start ordering

Configuration is parsed with the same `cmd-parse.y` grammar used for runtime commands. This is why you can use variables, conditionals, and even some commands inside config files.

## Key Bindings

`key-bindings.c` manages the key tables (`root`, `prefix`, `copy-mode`, `copy-mode-vi`, and any user-defined tables).

Bindings can be:
- Direct commands
- "switch to another key table" (the prefix mechanism)
- Commands with arguments

The tables are consulted by the input path after the tty layer has decoded a key, and also by copy mode and the various interactive modes.

## Environment

`environ.c` provides a simple name → value map with the usual `put` / `unset` / `copy` operations. Every session and client has its own environment that is used when spawning new panes or jobs. Global environment is inherited at server start.

## Live Changes

Most options can be changed at runtime with `set-option` / `set-window-option` / `set -g`. Many have immediate visible effects (status line updates, key table changes, colour palette reloads, etc.) because the options table entries often register change callbacks that poke the relevant redraw or state machines.

This live configurability is one of the big reasons tmux feels responsive and scriptable compared with multiplexers that require a restart to pick up config changes.

## Related Components

- **Command System**: the entire `set-option`, `show-options`, `bind-key`, `source-file` family lives here.
- **Core Entities**: sessions and windows own their option trees; many layout / size behaviours read window options.
- **TTY Rendering**: status line, styles, and formats are all driven by options + the format engine.
- **Client-Server IPC**: when a client attaches it receives the current server + session + window option state as part of identify.

## Topics in This Component

- `scopes-and-loading.md` — the three global roots, per-object overrides, cfg.c loading order, and how `source-file` interacts with a running server.
- `key-bindings.md` — the key table system, prefix handling, how bindings are looked up, and the relationship between root / prefix / copy-mode tables.