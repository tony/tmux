# Command System — Entries and Parser

## The cmd_entry Table

Every command is declared with a static `const struct cmd_entry` (https://github.com/tmux/tmux/blob/master/cmd.c). The struct contains:
- name and alias;
- argument specification (`args_template`);
- flag definitions (`cmd_entry_flag` for `-t`, `-s`, `-c` etc.);
- usage string;
- the `exec` function pointer.

At startup the table (an array of pointers to these structs) is simply iterated to build the help text and the fast lookup used by the parser and `list-commands`.

Because the entry lives next to the implementation in the same `.c` file, adding a feature to an existing command usually means touching only one file.

## 65-File Phenomenon

`fd --glob 'cmd-*.c' ... | wc -l` on the inspected tree yields 65 implementation files. This is not accidental architecture; it is the direct consequence of the "one command, one file, static entry" rule. It has two observable effects:
- Extremely easy to discover the implementation of any documented command.
- The binary is larger than a hypothetical "command table + shared interpreter" design would be, but the cognitive overhead of working on any single command is minimal.

## cmd-parse.y

The bison grammar (https://github.com/tmux/tmux/blob/master/cmd-parse.y) is used in three distinct contexts:
1. Parsing `~/.tmux.conf` and `-f` files at server start / `source-file`.
2. Parsing the argument to `if-shell`, `run-shell -b`, `display-menu`, etc.
3. Parsing commands typed at the `command-prompt` or sent via `send-keys -X`.

The parser produces a `cmd_list` (a TAILQ of `cmd` nodes, each carrying a `cmd_entry*` and an `args` tree). The same list structure is later executed by the queue machinery.

## Argument Parsing

`args.c` and the template strings in each `cmd_entry` drive a small declarative parser. `-F` flags cause format expansion of the following argument before the command ever sees it. This is why `new-window -n '#{host}'` works without the command author writing any format code.

The combination of a simple static entry + a reusable argument parser + a single execution queue is what lets tmux present a remarkably consistent and discoverable command language while still supporting the advanced control-flow features (`if-shell`, `wait-for`, command grouping) that power complex configurations and plugins.