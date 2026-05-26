# Command System — Queue and Dispatch

## cmdq_item and cmdq_list

Defined in https://github.com/tmux/tmux/blob/master/cmd-queue.c.

A `cmdq_item` carries:
- the `cmd_list` to execute (or a single callback for async continuations);
- source and target `cmd_find_state` (the resolved `-s` / `-t` contexts);
- a `group` id (commands with the same group number are serialized);
- flags (`CMDQ_FIRED`, `CMDQ_WAITING`);
- an optional `cb` + `data` for the continuation after an async operation.

Items live on `TAILQ` lists owned either by a `client` or by the global server queue.

## Execution Rules in server_loop

```c
do {
    items = cmdq_next(NULL);           /* global queue */
    TAILQ_FOREACH(c, &clients, entry)
        if (identified(c))
            items += cmdq_next(c);     /* per-client queues */
} while (items != 0);
```

`cmdq_next` returns the number of items it managed to start (or continue). The loop drains everything that is ready before the server does any other work (redraw, job I/O, etc.). This gives the illusion of atomic command sequences even though the server is single-threaded and event-driven.

## Waiting and Callbacks

Commands such as `if-shell`, `run-shell`, `command-prompt`, `confirm-before`, and `wait-for` set `CMDQ_WAITING` on their item and return `CMD_RETURN_WAIT`. They install a callback (often via a `job` or a `wait-for` channel). When the external event fires, the callback is invoked; it may append more items to the same queue and then clears the waiting flag. The next `cmdq_next` will pick up the continuation.

This is how `if-shell "sleep 2" "echo done"` can pause a command sequence for two seconds without freezing every other client attached to the server.

## Groups

The `-g` / group mechanism (used internally by `source-file` with multiple commands, or by users via `;` vs `&&` style parsing) ensures that a set of commands are treated as a single unit for the purpose of "last window" updates, focus changes, and error handling. Items in the same group share the same `group` number and are executed sequentially even if other items from other groups are present on the queue.

## Error and Exit Handling

A command returning `CMD_RETURN_ERROR` or `CMD_RETURN_STOP` aborts the remainder of its `cmd_list`. The queue machinery propagates the error to the client (or to the `if-shell` caller) via the normal reply path. This is why `tmux new-session 'bad-command' || echo failed` works from the shell.

The design deliberately keeps the command queue small, deterministic, and fully observable from the event loop — the single most important reason tmux can remain responsive while running long-lived background jobs or interactive prompts.