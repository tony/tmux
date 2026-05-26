# Pane Lifecycle — Jobs and Asynchronous Execution

## struct job

```c
struct job {
    enum { JOB_RUNNING, JOB_DEAD, JOB_CLOSED } state;
    int              flags;
    pid_t            pid;
    int              fd;               /* usually the child's stdout */
    struct bufferevent *event;         /* libevent wrapper */
    job_update_cb    updatecb;
    job_complete_cb  completecb;
    void            *data;
    ...
};
```

Created by `job_start` or the higher-level `job_run` helpers used by commands.

## Bufferevent Integration

The child's stdout (and optionally stderr) are turned into a `bufferevent` with `BEV_OPT_DEFER_CALLBACKS`. Read callbacks accumulate data into a `cmdq_item`'s buffer or a paste buffer. Error/EOF callbacks mark the job `JOB_DEAD` and schedule the completion callback.

Because everything is event-driven, a long-running `run-shell 'sleep 3600'` consumes zero CPU on the server while it waits.

## Callback Lifetime and Queue Resumption

The completion callback is almost always responsible for:
- appending more `cmdq_item`s to the queue that was waiting;
- clearing the `CMDQ_WAITING` flag on the original item;
- cleaning up any temporary state (temporary buffers, client pause flags, etc.).

`job_free` is called only after the completion callback has returned and no other references remain. This guarantees that a command that did `if-shell "sleep 5" "echo hi"` will have its continuation executed exactly once, after the five seconds have elapsed, even if the client that issued the command has since detached.

## Distinction from Pane Children

A pane's shell is a direct child whose output is fed through the full terminal emulation pipeline (input parser → grid). A job's output is treated as raw bytes for a buffer, a notification, or a one-shot command continuation. The two mechanisms share the same underlying `fork` + `wait` + event primitives but serve completely different purposes in the command and UI model.

Jobs are the reason tmux can offer `run-shell`, `if-shell`, control-mode file I/O, and background hooks without ever blocking the single server loop that all clients depend on.