# Pane Lifecycle and Jobs — Overview

How panes (and their child processes) are created, how background work is scheduled, and how the server stays responsive while children run.

Key files: https://github.com/tmux/tmux/blob/master/spawn.c, job.c, window.c (pane management parts).

## Pane Creation Path

`spawn_window` / `spawn_pane` (spawn.c) are called by `new-window`, `split-window`, `respawn-*`, and the initial window creation at session birth.

They:
1. Resolve cwd, environment, command, and size from the `spawn_context`.
2. Allocate the `window_pane` struct and its `screen` / `input_ctx`.
3. Call `fdforkpty` (or platform `forkpty`) on the master pty fd obtained via `getptmfd`.
4. In the child: set up the slave tty, `exec` the shell/command.
5. In the parent: store pid + fd on the pane, insert the pane into the window's TAILQ and the layout tree, and start reading from the pty master via libevent (or bufferevent in some paths).

The pty master fd stays inside the server for the lifetime of the pane; it is never passed over IPC.

## Jobs (asynchronous child processes)

`struct job` (job.c) is a separate mechanism from panes:
- Used by `run-shell`, `if-shell`, `pipe-pane`, control-mode file copies, and some `display-*` commands.
- Created with `job_run` or `job_start`.
- The child's stdout/stderr are attached to libevent `bufferevent`s.
- When the child exits, a callback (often supplied by the originating command) is invoked.
- The callback can append more items to a command queue, write to a paste buffer, send a control-mode notification, etc.

Jobs are reference-counted and cleaned up by `job_died` / `job_free`. `job_kill_all` is called on server shutdown.

## Relationship to the Rest of the System

- A pane's "normal" child is *not* a `job`; it is owned directly by the `window_pane` and its data path goes through the input parser.
- `pipe-pane` is the bridge: it creates a job whose output is also fed into the pane's grid (or vice-versa).
- `if-shell` and `run-shell` are the main users of the "pause the command queue until this job finishes" pattern.

## Topics in This Component

- `spawn-pty.md` — `getptmfd`, `fdforkpty` wrapper, the exact child setup sequence, what happens on respawn, and how the pane fd is turned into input events.
- `jobs-async.md` — the `job` struct, bufferevent integration, callback lifetime, and how jobs let long-running work coexist with the single-threaded server loop and command queues.