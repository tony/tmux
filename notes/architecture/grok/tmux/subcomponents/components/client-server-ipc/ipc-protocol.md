# Client-Server IPC — Protocol and Handshake

## The Peer Abstraction

`struct tmuxproc` (https://github.com/tmux/tmux/blob/master/proc.c:36) owns a `TAILQ_HEAD(, tmuxpeer) peers`.

`struct tmuxpeer` (proc.c:54) wraps:
- an `imsgbuf`
- a libevent `event`
- a `dispatchcb` that receives fully reassembled `struct imsg`

`proc_add_peer` wires the fd into the event loop and version-checks the first message.

## Message Types (tmux-protocol.h)

The enumeration (https://github.com/tmux/tmux/blob/master/tmux-protocol.h) is divided into bands:

- 100–199: identification (`MSG_IDENTIFY_FLAGS`, `TERM`, `TTYNAME`, `ENVIRON`, `CWD`, `FEATURES`, `TERMINFO`, `DONE`, …)
- 200–299: control (`MSG_COMMAND`, `DETACH`, `DETACHKILL`, `EXIT`, `RESIZE`, `SUSPEND`, `LOCK`, `SHELL`, `SHUTDOWN`, …)
- 300–399: file transfer streams (`MSG_READ_OPEN`/`READ`/`READ_DONE`/`READ_CANCEL`, same for `WRITE_*`)

Payload structs follow each message (e.g. `struct msg_command { int argc; }` followed by packed argv strings and environment).

`PROTOCOL_VERSION` is currently 8; any change to message layout or new required identify fields bumps it and forces an error on the peer.

## Client Identification Handshake

After the peer is created the client immediately sends a burst of `MSG_IDENTIFY_*` messages (term name, features, cwd, environment, stdin/stdout fds via SCM_RIGHTS when needed, client pid, etc.) and finishes with `MSG_IDENTIFY_DONE`.

The server side (`server_client_dispatch_identify` etc.) accumulates the information into the freshly created `struct client`, sets `CLIENT_IDENTIFIED`, and only then begins processing `MSG_COMMAND` messages for that client.

## Command Execution over IPC

A client that has a command to run (or the initial command from `tmux new …`) sends `MSG_COMMAND` (or `MSG_SHELL` for the implicit login shell case).

The server receives it on the peer's dispatch callback, parses it again (for safety), creates a `cmdq_item`, and places it on the client's `cmdq_list`. `server_loop` later drains it via `cmdq_next(c)`.

Results (output, exit status, "detached" messages, etc.) travel back either as imsg replies or by the server writing directly to the client's `out_fd` (the original stdout of the client process).

## File Descriptor Passing

Several messages carry file descriptors:
- Client stdin/stdout at identify time (for `-CC` control mode and normal attach)
- `MSG_READ_OPEN` / `MSG_WRITE_OPEN` for `load-buffer`/`save-buffer` and control-mode file copies
- The initial pty master for a new pane is never passed over IPC; it is created inside the server after the `new-window` / `split-window` command has been executed locally.

`proc_get_peer_uid` is used for the `server-access` ACL checks.

## Version and Capability Negotiation

The very first imsg on a new peer is `MSG_VERSION`. If the peer's version does not match, the connection is dropped before any identify data is processed. Feature bits (256-color, sixel, clipboard, etc.) are conveyed in the identify phase rather than in the protocol version number, allowing a newer client to talk to an older server (with graceful degradation).

## Summary

The IPC layer is deliberately narrow: a typed message bus with SCM_RIGHTS for the handful of fds that must cross the privilege or process boundary. All higher-level concepts (sessions, panes, commands, formats) live entirely on the server side after the identify handshake completes. This is why killing the server process (`kill-server`) instantly terminates every client, and why a client can attach, detach, and re-attach to the same live server without losing state.