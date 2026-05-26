# Client-Server IPC — Overview

This component contains the fundamental process split and the only inter-process communication path in tmux.

All definitions and entry points are in the public tree at https://github.com/tmux/tmux (tags `3.6b` and `master`).

## One-Paragraph Role

Every `tmux` process begins life as a client (`client_main` in `client.c`). If no server is listening on the socket determined by `-L`/`-S` or `$TMUX`, the client takes a flock, forks via `proc_fork_and_daemon`, and the child re-initializes libevent and becomes the server (`server_start` in `server.c`). All subsequent interaction—command execution, resize notifications, detach, file transfer for buffers—travels over the imsg protocol defined in `tmux-protocol.h` and implemented with the `tmuxproc`/`tmuxpeer` abstraction in `proc.c`.

## Key Files (with canonical links)

- https://github.com/tmux/tmux/blob/master/tmux.c — `main` (always delegates to `client_main`)
- https://github.com/tmux/tmux/blob/master/client.c — `client_main`, `client_connect`, lock handling, `server_start` caller
- https://github.com/tmux/tmux/blob/master/server.c — `server_start`, `server_loop`, `server_client_create`, exit decision logic
- https://github.com/tmux/tmux/blob/master/proc.c — `tmuxproc`, `tmuxpeer`, `imsgbuf` wrappers, `proc_add_peer`, signal bridging
- https://github.com/tmux/tmux/blob/master/tmux-protocol.h — `enum msgtype` (MSG_VERSION … MSG_READ_CANCEL), `struct msg_*` payload shapes, PROTOCOL_VERSION 8
- https://github.com/tmux/tmux/blob/master/compat/imsg.c and `imsg-buffer.c` — the OpenBSD imsg library (bundled)

## Data-Flow Summary

1. Client attempts `client_connect` (Unix socket).
2. On ECONNREFUSED (or systemd activation), client calls `server_start`.
3. `server_start` calls `proc_fork_and_daemon`; child becomes server, parent returns the connected fd.
4. Both sides create a `tmuxproc` and attach `tmuxpeer`s via `proc_add_peer`.
5. Server runs `proc_loop(..., server_loop)`; clients run their own `proc_loop`.
6. All interesting work (commands, resize, detach) is carried by imsg messages; file descriptors (tty, out_fd, etc.) are passed with SCM_RIGHTS where required.

## Related Components

- **Core Entities**: the `client` struct (https://github.com/tmux/tmux/blob/master/tmux.h#L1997) is both an IPC peer and the owner of a command queue and per-client window view.
- **Command System**: commands arrive as `MSG_COMMAND` imsgs and are turned into `cmdq_item`s on the client's queue.
- **Pane Lifecycle and Jobs**: `job.c` and control-mode file transfers also ride the same peer mechanism.

## Topics in This Component

- `process-model.md` — fork/daemon dance, `client_main` → `server_start` transition, `server_loop` exit conditions (`exit-empty`, `exit-unattached`, job still running, attached clients).
- `ipc-protocol.md` — `tmuxpeer`/`imsgbuf` mechanics, the message enumeration, identify handshake (`MSG_IDENTIFY_*`), command dispatch, and the handful of file-descriptor-bearing messages.

See sibling component pages for how the rest of the system is built on top of this narrow IPC channel.