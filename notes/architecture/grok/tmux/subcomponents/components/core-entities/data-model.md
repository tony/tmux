# Core Entities — Data Model

All definitions live in https://github.com/tmux/tmux/blob/master/tmux.h.

## struct client (L1997)

```c
const char      *name;
struct tmuxpeer *peer;
struct cmdq_list *queue;
struct client_windows windows;   /* per-client view of winlinks */
...
struct environ  *environ;
pid_t            pid;
int              fd;             /* socket to server */
int              out_fd;         /* where server writes output */
```

A client that has completed identification has `CLIENT_IDENTIFIED`. Suspended clients keep the struct but are removed from normal redraw paths.

## struct session (L1498)

```c
u_int            id;
char            *name;
struct winlink  *curw;
struct winlink_stack lastw;
struct winlinks  windows;        /* RB tree by index */
struct options  *options;
struct environ  *environ;
u_int            attached;       /* count of clients on this session */
```

`attached` is incremented on attach and decremented on detach; it is the value tested by the `exit-unattached` logic.

## struct winlink (L1403)

```c
int              idx;
struct session  *session;
struct window   *window;
int              flags;          /* WINLINK_BELL | ACTIVITY | SILENCE | VISITED */
RB_ENTRY(winlink) entry;
TAILQ_ENTRY(winlink) wentry;     /* on window's list */
TAILQ_ENTRY(winlink) sentry;     /* on session's list */
```

This is the only struct that participates in two different RB/TAILQ collections simultaneously.

## struct window (L1345)

```c
u_int                 id;
struct window_pane   *active;
struct window_panes   panes;     /* TAILQ */
struct window_panes   last_panes;
struct layout_cell   *layout_root;
struct layout_cell   *saved_layout_root;
u_int                 sx, sy;    /* current size */
```

`lastlayout` remembers which preset (`even-horizontal` etc.) was last applied so `select-layout -E` can re-apply it.

## struct window_pane (L1248)

```c
u_int             id;
struct window    *window;
struct layout_cell *layout_cell;
struct layout_cell *saved_layout_cell;
u_int             sx, sy, xoff, yoff;
int               flags;         /* PANE_* bits */
pid_t             pid;
int               fd;            /* pty master */
char              tty[TTY_NAME_MAX];
struct screen     screen;
struct input_ctx *ictx;
TAILQ_HEAD(, window_mode_entry) modes;
```

The `PANE_*` flag bits (REDRAW, DROP, FOCUSED, ZOOMED, FLOATING, INPUTOFF, EXITED, …) are the primary mechanism that drives the redraw and layout engines.

## struct layout_cell (L1461)

```c
enum layout_type  type;          /* LAYOUT_LEFTRIGHT or TOPBOTTOM */
struct layout_cell *parent, *children;
u_int             sx, sy;
u_int             xoff, yoff;
```

The tree is always a full binary tree; leaves point back to their `window_pane`. Resize walks the tree and recomputes child sizes according to the split ratios stored in the cells.

## Lifetime Notes

- Panes are destroyed when their child process exits or via `kill-pane`; the `window_pane` struct is freed only after the layout cell is unlinked and any pending redraw has completed.
- Windows survive as long as at least one session references them via a winlink.
- Sessions are destroyed when the last client detaches and `exit-unattached` permits it, or explicitly via `kill-session`.

These six structs plus the three global roots (`sessions`, `windows`, `clients`) are the entire authoritative state of a running tmux server. Everything else (jobs, formats, key tables, options) is either derived or auxiliary.