# Configuration and Options — Key Bindings

## Key Tables

tmux maintains a set of named key tables. The important built-in ones are:

- `root` — keys that work when no prefix is active
- `prefix` — keys that work after you press the prefix (usually `C-b`)
- `copy-mode` and `copy-mode-vi` — active only while in copy mode (or other modes that switch to them)
- Any user-defined tables created with `bind-key -T newtable ...`

Each table is a hash of key → binding. A binding is either a command string or a "switch to table X" action (the classic prefix behaviour).

## How a Key Becomes a Command

1. The tty layer (`tty-keys.c`) decodes the bytes coming from the client's terminal into a `key_code` (including modifiers, mouse events, etc.).
2. The input path looks at the current key table for the client/pane.
3. If the key matches a binding that switches tables, the client's "next key table" is updated.
4. If it matches a command, the command is parsed and queued exactly like a command arriving over IPC.
5. If nothing matches in the current table, the key falls through to the root table (or is passed through to the pane in some cases).

This is why `bind-key -T root C-a send-prefix` and `bind-key C-a send-prefix` behave differently depending on whether you are already in the prefix table.

## Prefix Handling

The default `prefix` table is what makes tmux feel like a modal editor for a few seconds after you press the prefix. All the normal Emacs/readline keys in your shell continue to work because they are looked up in the `root` table until you hit the prefix.

You can have multiple prefixes, or use `bind-key -T root` for things that should always be available (common for `C-a C-a` "last window" patterns).

## Copy Mode Tables

When a pane enters copy mode (or any mode that calls `window_pane_set_mode` with a custom key table), the mode's `key` callback receives raw keys and decides what to do. Most modes simply forward to the current key table for that mode (`copy-mode` or `copy-mode-vi`).

This is why you can do `bind-key -T copy-mode-vi v begin-selection` and have it work only while in vi-style copy mode.

## Live Updates

`bind-key`, `unbind-key`, and `set -g prefix ...` all take effect immediately for new key events. Existing prefix states are not disturbed (if you are in the middle of a prefix sequence when you rebind the prefix, the old binding is still used for the next key).

This makes it very easy to experiment with key bindings in a running tmux without fear of locking yourself out.

## Relationship to Commands

Key bindings are ultimately just another way to produce `cmd_list`s. There is no deep difference between typing `tmux new-window` on the shell prompt inside a pane, sending it over the IPC `MSG_COMMAND` channel, or having it come from a key binding. They all end up as items on a command queue.

This uniformity is a large part of why tmux's scripting and key-binding stories feel coherent.