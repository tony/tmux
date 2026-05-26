# Client-Server IPC — Process Model

## Entry Point Uniformity

`main` in https://github.com/tmux/tmux/blob/master/tmux.c:556 does only locale, environment, option table defaults, and socket-path resolution, then unconditionally calls:

```c
exit(client_main(osdep_event_init(), argc, argv, flags, feat));
```

There is no "am I the server?" test at process start.

## Client Path (Normal Case)

1. `client_main` (https://github.com/tmux/tmux/blob/master/client.c:232) parses flags, creates a `tmuxproc` named "client".
2. Calls `client_connect` (or the systemd activation path).
3. On success, attaches a `tmuxpeer` with `client_dispatch` as callback and drops privileges (`pledge`).
4. Enters its own `proc_loop`.

If connect fails with ECONNREFUSED (or the `-N` / lock dance requires it), the client calls `server_start` itself.

## Server Start and Daemonization

`server_start` (https://github.com/tmux/tmux/blob/master/server.c:176):

```c
if (~flags & CLIENT_NOFORK) {
    if (proc_fork_and_daemon(&fd) != 0) { ... }   // parent returns
}
...
server_proc = proc_start("server");
...
proc_loop(server_proc, server_loop);
```

`proc_fork_and_daemon` (proc.c) performs the classic double-fork + setsid + redirect of stdio so the server is a true daemon even when the original client terminal goes away.

After the fork:
- Child re-initializes the libevent base (`event_reinit`).
- Creates the listening socket (`server_create_socket` or systemd socket).
- Calls `server_add_accept(0)`.
- The original connected client fd (if any) is turned into the first `struct client` via `server_client_create`.

## Server Main Loop and Exit Conditions

`server_loop` (https://github.com/tmux/tmux/blob/master/server.c:264) is called repeatedly from `proc_loop`:

```c
do {
    items = cmdq_next(NULL);
    TAILQ_FOREACH(c, &clients, entry)
        if (c->flags & CLIENT_IDENTIFIED)
            items += cmdq_next(c);
} while (items != 0);

server_client_loop();

if (!options_get_number(global_options, "exit-empty") && !server_exit)
    return (0);
if (!options_get_number(global_options, "exit-unattached") && !RB_EMPTY(&sessions))
    return (0);
... more guards for attached clients and running jobs ...
return (1);   /* tells proc_loop to exit */
```

The server therefore stays alive exactly as long as:
- there are attached clients, or
- sessions exist and `exit-unattached` is off, or
- `exit-empty` is off, or
- background jobs are still running, or
- `server_exit` has not been forced.

## Signals and Lifecycle Cleanup

Both client and server processes register signal handlers via `proc_set_signals`. The server translates `SIGCHLD` into job reaping and child-pane exit handling. On graceful shutdown `server_send_exit` walks the client list and forces `CLIENT_EXIT` so that the last client to drain its queue will cause the server process to terminate cleanly.

## Summary

The model is "first client wins the server role via a lock-protected fork." After that point there is a clean 1:N relationship: one server process, N client processes, all communication via the imsg peers established at connect/accept time. No other daemons or helper binaries are involved.