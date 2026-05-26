# Pane Lifecycle — Spawn and PTY

## Master FD Acquisition

`getptmfd` (osdep-*.c or the generic path) opens `/dev/ptmx` (or equivalent) and returns a master fd that will be used for *all* subsequent pty allocations in this server process. Keeping one master fd avoids per-pane open races and lets the server set ownership/permissions once.

## fdforkpty and Child Setup

In `spawn_pane` (https://github.com/tmux/tmux/blob/master/spawn.c:386):

```c
new_wp->pid = fdforkpty(ptm_fd, &new_wp->fd, new_wp->tty, NULL, &ws);
```

`fdforkpty` (compat/fdforkpty.c or platform forkpty-*.c) does the `forkpty` dance and returns the master side to the parent while the child is on the slave.

In the child (before exec):
- `setsid`, `ioctl(TIOCSCTTY)`
- termios and winsize are applied
- environment and cwd are set from the spawn context
- the command (or default shell + login flag) is `exec`ed

If exec fails the child writes an error message to the slave and exits; the parent sees the exit and marks the pane `PANE_EXITED`.

## Pane Lifecycle After Spawn

- The master fd is made non-blocking and fed to the input event loop (or a bufferevent).
- Data arriving on it goes to `input_ctx` → grid updates.
- The pid is waited via the server's `SIGCHLD` handler (`server_child_exited`).
- On exit the pane is removed from the layout, the window may be destroyed if it was the last pane, and any `remain-on-exit` or `pane-died` hooks fire.

## Respawn

`respawn-pane` / `respawn-window` reuse the existing `window_pane` struct (and its screen/history where possible), kill the old pid if still running, and go through the spawn path again with a fresh command. The layout cell and client views are preserved; only the child process is replaced. This is why you can restart a shell inside a window without losing scrollback or the pane's position in the layout.

The pty master fd is closed and a new one allocated for the respawned child; the old fd is the one that produced the "pane died" notification.