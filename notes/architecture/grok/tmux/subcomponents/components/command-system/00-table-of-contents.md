# Command System — Overview

tmux is, above all, a command interpreter. The overwhelming majority of its source (65 files under `dist_tmux_SOURCES` in https://github.com/tmux/tmux/blob/master/Makefile.am) consists of individual command implementations.

## Scale and Shape

- `cmd-*.c` — one file per command (or closely related family).
- `cmd.c` — the master `cmd_entry` table, parsing glue, and `cmd_list` management.
- `cmd-queue.c` — the execution engine (`cmdq_item`, groups, waiting, callbacks).
- `cmd-parse.y` — the bison grammar used both for `~/.tmux.conf` and for the `tmux` command-line / `send-keys` / `if-shell` etc.
- `cmd-find.c` — target resolution (`-t` / `-s` / current / last / next etc.).

## Command Lifecycle (simplified)

1. Client sends `MSG_COMMAND` (or initial command at identify time).
2. Server parses the string again via `cmd_parse_from_*` (defense in depth).
3. A `cmd_list` is created and wrapped in a `cmdq_item` (or items).
4. The item is placed on the client's `cmdq_list` (or the global queue for server-wide commands).
5. `server_loop` calls `cmdq_next` which executes the next ready item.
6. Most commands finish synchronously and call their `cmdq_cb` (or the default completion).
7. Commands that need to wait (if-shell, run-shell, prompt, etc.) set the `CMDQ_WAITING` flag and install a callback that will be invoked later by a job, a timer, or user input.

## Why So Many Files?

Each command is intentionally self-contained. A developer can add a new command by dropping a new `cmd-foo.c` that defines a `const struct cmd_entry cmd_foo_entry = { ... }` and implementing the handler. The build system simply lists it in `Makefile.am`; no central registry beyond the table in `cmd.c` is required.

This is why `cmd-list-commands` can enumerate every command with its flags, arguments, and help text — the data is all statically declared next to the implementation.

## Related Components

- **Client-Server IPC**: commands arrive and results leave via the peer.
- **Core Entities**: the great majority of command handlers exist only to mutate or query that graph.
- **Configuration and Options**: many commands are thin wrappers around `options_set_*` or `key_bindings_*`.

## Topics in This Component

- `queue-dispatch.md` — `cmdq_item` states, the `CMDQ_*` flags, group handling, `cmdq_next`, and how waiting + callbacks let `if-shell` and `run-shell` pause execution without blocking the server loop.
- `entries-parser.md` — the `cmd_entry` struct, argument parsing, the 65-file phenomenon, and how `cmd-parse.y` is used for both config files and runtime command strings.