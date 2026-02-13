# TermForge Architecture -- v0002 Pass 1

## 1. Overview

TermForge is a Rust terminal multiplexer SDK with 100% tmux wire-protocol v8
compatibility. It is designed as an embeddable library with a clear SDK entry
point (`mux-api`), not merely as a standalone binary.

### 1.1 Design Principles

1. **SDK-first**: The primary consumer is a Rust developer embedding TermForge
   as a library. The CLI tools (`tmux-builder`, `tmux-vm`, etc.) are built atop
   the SDK, not the other way around.
2. **Sans-IO kernel**: The kernel owns all mutable state, processes events, and
   emits effects. It performs no I/O directly. This enables deterministic testing.
3. **3-thread model**: Kernel (std::thread), IO (tokio), Render (tokio task).
4. **tmux compatibility**: Wire protocol, command set, target syntax, format
   strings, and option names are compatible with tmux v3.5+.
5. **Zero unsafe outside quarantine**: Only `mux-pty` and `mux-ffi` contain
   `unsafe` code.

### 1.2 Thread Architecture

```
                          +------------------+
                          |   IO Thread      |
                          |   (tokio)        |
                          |                  |
  Clients <--unix sock--> |  PTY FDs         |
  SIGWINCH -------------> |  signal-hook     |
  SIGCHLD  -------------> |  child reaper    |
                          +--------+---------+
                                   |
                      crossbeam bounded channels
                      (control: 512, data: 65536)
                                   |
                          +--------v---------+
                          |  Kernel Thread   |
                          |  (std::thread)   |
                          |                  |
                          |  ALL mutable     |
                          |  state lives     |
                          |  here            |
                          +--------+---------+
                                   |
                      crossbeam bounded channel
                      (render: 256 snapshots)
                                   |
                          +--------v---------+
                          |  Render Thread   |
                          |  (tokio task)    |
                          |                  |
                          |  Composition     |
                          |  Diff pipeline   |
                          |  Output coalesce |
                          +---------+--------+
                                    |
                              write(2) to
                              client FDs
```

### 1.3 Channel Specifications (DECIDED, from GPT)

| Channel | Direction | Bound | Message Size | Backpressure |
|---------|-----------|-------|-------------|--------------|
| control | IO -> Kernel | 512 | ~64 bytes | sender blocks |
| data | IO -> Kernel | 1024 | up to 64 KiB | sender blocks |
| render | Kernel -> Render | 256 | snapshot refs | oldest dropped |
| effect | Kernel -> IO | 1024 | ~128 bytes | sender blocks |

All channels use `crossbeam-channel` bounded MPSC. The kernel never blocks on
sends; if the render channel is full, the oldest snapshot is dropped (the render
thread will catch up at the next snapshot). Control and effect channels block the
sender, providing natural backpressure.

## 2. Crate Dependency Layers

```
L0 (no deps):     mux-grapheme-arena, mux-time
L1 (L0 only):     mux-types
L2 (L0+L1):       mux-grid, mux-parser, mux-proto, mux-options, mux-target,
                   mux-format, mux-config
L3 (L0-L2):       mux-snapshot, mux-cmd-parse, mux-orm, mux-render, mux-pty
L4 (L0-L3):       mux-kernel, mux-termlet
L5 (L0-L4):       mux-api, mux-test-support, mux-ffi, mux-otel
L6 (tools):       tmux-builder, tmux-vm, tmux-sniff, mux-doctor
```

## 3. Composition and Diff Rendering Pipeline (Gap #1 -- DECIDED)

### 3.1 Problem Statement

A terminal multiplexer must compose N pane grids into one output stream for M
clients. Each client sees borders, status line, and the active window's panes.
The performance-critical path is: "given that pane P changed line Y, emit the
minimal escape sequences to update the client's terminal."

### 3.2 Decision: Own Dirty-Line Diff with Double-Buffered CompositeGrid

**Rejected alternatives:**
- `vt100::contents_diff()`: Requires feeding raw bytes through a third-party VT
  parser. TermForge already parses VT; round-tripping through vt100 duplicates
  work and adds a coupling point. Zellij chose vt100 because it delegates VT
  parsing entirely; TermForge owns its parser.
- `termwiz::Surface` + `Change` log: Designed for single-terminal applications
  (e.g., WezTerm's GUI), not server-side composition of multiple grids.
  Change log accumulates unboundedly without periodic draining.
- ratatui `Buffer::diff()`: Closest match. Flat `Vec<Cell>` with position-based
  diff. However, ratatui's model assumes the application owns the entire screen.
  TermForge needs per-pane dirty tracking that propagates to per-line invalidation
  in the composite buffer.

**Chosen approach**: A `CompositeGrid` that mirrors the client's terminal. Each
render cycle:

1. For each pane, check the grid's `line_dirty` BitVec.
2. For dirty pane lines, blit cells into the corresponding region of the
   CompositeGrid, translating pane-local coords to absolute screen coords.
3. Diff the CompositeGrid against the previous frame's CompositeGrid
   (double-buffered swap).
4. Emit SGR + CUP sequences only for changed cells.
5. Coalesce adjacent changed cells into contiguous write runs to minimize
   escape sequence overhead.

### 3.3 Data Structures

```rust
/// A flat buffer representing the entire client terminal.
/// Indexed as `cells[y * width + x]`.
pub struct CompositeGrid {
    cells: Vec<CompositeCell>,
    width: u32,
    height: u32,
}

/// A cell in the composite grid. Tracks both content and the source pane.
pub struct CompositeCell {
    cell: Cell,
    source: CellSource,
}

/// Where a composite cell came from.
pub enum CellSource {
    Pane(PaneId),
    Border,
    StatusLine,
    Empty,
}
```

### 3.4 Diff Algorithm

```rust
pub fn diff<'a>(
    prev: &CompositeGrid,
    next: &'a CompositeGrid,
) -> Vec<CellUpdate<'a>> {
    let mut updates = Vec::new();
    let mut invalidated = 0usize;
    for i in 0..next.cells.len() {
        if next.cells[i].cell != prev.cells[i].cell || invalidated > 0 {
            let x = (i as u32) % next.width;
            let y = (i as u32) / next.width;
            updates.push(CellUpdate { x, y, cell: &next.cells[i].cell });
            // Handle wide char invalidation (ratatui pattern)
            let w = unicode_width(next.cells[i].cell.grapheme);
            if w > 1 { invalidated = w - 1; } else { invalidated = 0; }
        } else if invalidated > 0 {
            invalidated -= 1;
        }
    }
    updates
}
```

### 3.5 Frame Rate Policy

- Target: 60 Hz (16.6 ms per frame).
- Implementation: Render thread runs a `tokio::time::interval(Duration::from_millis(16))`.
- If no snapshots arrive, the render thread sleeps.
- If snapshots arrive faster than 60 Hz, intermediate snapshots are dropped.
- Output flooding mitigation: if a single pane produces > 64 KiB/frame of output,
  the data channel backpressure naturally throttles the IO thread's read loop.
- Per-client output coalescing: changes are batched into a single `write(2)` call.

### 3.6 Output Coalescing Strategy

The renderer emits escape sequences into a `Vec<u8>` buffer, then flushes with a
single write. Optimizations:

1. **Cursor movement elision**: If the next changed cell is adjacent to the
   previous one, no CUP sequence is emitted (the cursor auto-advances).
2. **SGR batching**: SGR attributes are only re-emitted when they differ from the
   current state (stateful tracking).
3. **Repeat character optimization**: If a character repeats N times with the same
   attributes, use REP (CSI Ps b) when the terminal supports it.
4. **Erase optimization**: Long runs of empty cells use EL (CSI K) instead of
   writing spaces.

## 4. Layout Engine (Gap #2 -- DECIDED)

### 4.1 tmux Layout Model

tmux uses a tree of `layout_cell` nodes. Each node is one of:
- `LAYOUT_LEFTRIGHT`: horizontal container, children are arranged left-to-right
- `LAYOUT_TOPBOTTOM`: vertical container, children are arranged top-to-bottom
- `LAYOUT_WINDOWPANE`: leaf node containing a window pane

Each node stores: `sx` (width), `sy` (height), `xoff` (x offset), `yoff`
(y offset), plus a pointer to the parent and a list of children.

tmux has 7 built-in layout algorithms (layout-set.c):
- `even-horizontal`: equal-width columns
- `even-vertical`: equal-height rows
- `main-horizontal`: one large pane on top, rest below
- `main-horizontal-mirrored`: one large pane on bottom, rest above
- `main-vertical`: one large pane on left, rest on right
- `main-vertical-mirrored`: one large pane on right, rest on left
- `tiled`: grid arrangement

Plus custom layout strings with checksum validation (layout-custom.c).

### 4.2 TermForge Layout Data Structures

```rust
/// Layout cell type, matching tmux's layout_type enum.
pub enum LayoutType {
    LeftRight,
    TopBottom,
    Pane,
}

/// A node in the layout tree.
pub struct LayoutCell {
    pub cell_type: LayoutType,
    pub sx: u32,
    pub sy: u32,
    pub xoff: u32,
    pub yoff: u32,
    pub pane: Option<PaneId>,
    pub children: Vec<LayoutCell>,
}
```

### 4.3 Layout Algorithms

The `LayoutKind` enum drives layout computation:

```rust
pub enum LayoutKind {
    EvenHorizontal,
    EvenVertical,
    MainHorizontal,
    MainHorizontalMirrored,
    MainVertical,
    MainVerticalMirrored,
    Tiled,
    Custom(String),  // layout string with checksum
}
```

**Even layout algorithm** (from tmux layout-set.c):
```rust
fn layout_even(panes: &[PaneId], total: u32, direction: LayoutType) -> LayoutCell {
    let n = panes.len() as u32;
    let each = total / n;
    let extra = total % n;
    // Each pane gets `each` size, first `extra` panes get +1.
    // Between panes: 1-cell separator (border).
    // Available = total - (n - 1) separators.
    // ...
}
```

**Tiled algorithm** (from tmux layout-set.c):
```
Compute rows = ceil(sqrt(n)), cols = ceil(n / rows).
Distribute panes in row-major order.
Last row may have fewer panes -- those panes get wider.
```

### 4.4 Resize Redistribution

When the window resizes (SIGWINCH propagation), the layout tree must be
recalculated. tmux's algorithm:

1. Compute the delta: `delta = new_size - old_size`.
2. Walk the tree depth-first.
3. At each container node, distribute the delta proportionally among children.
4. Minimum pane size: 2x1 (1 cell + 1 border).
5. If the delta is too negative (shrinking), some panes may be hidden (tmux
   does not hide, it enforces minimum sizes and returns an error if impossible).

### 4.5 Custom Layout Strings

tmux encodes layouts as strings with checksums, e.g.:
`a]54,202x50,0,0{101x50,0,0,0,100x50,102,0,1}`

TermForge parses these strings to reconstruct layout trees (in mux-cmd-parse)
and serializes them for `list-windows` output compatibility.

## 5. Raw Mode Lifecycle (Gap #3 -- DECIDED)

### 5.1 The Problem

Terminal raw mode (disabling line buffering, echo, and signal generation) must
be managed carefully. Two scenarios:

1. **Standalone server**: TermForge enters raw mode when a client connects,
   restores it when the client detaches. The IO thread owns the transition.
2. **Embedded SDK**: The caller may already be in raw mode (e.g., a TUI app
   embedding a terminal pane). TermForge must not fight for terminal ownership.

### 5.2 Decision: TerminalMode Policy Enum

```rust
/// Who owns the raw mode transition.
pub enum TerminalModePolicy {
    /// TermForge manages raw mode (standalone server mode).
    /// Calls cfmakeraw/tcsetattr on attach, restores on detach.
    Managed,
    /// Caller manages raw mode (embedded SDK mode).
    /// TermForge assumes the terminal is already in the correct state.
    External,
    /// TermForge queries the current state on attach and restores it on detach.
    /// Safe default for unknown embedding contexts.
    SaveRestore,
}
```

The `MuxServerBuilder` in `mux-api` accepts this policy:
```rust
MuxServer::builder()
    .terminal_mode(TerminalModePolicy::Managed)
    .build()?;
```

### 5.3 Implementation

`mux-pty` provides `RawModeGuard`:
```rust
pub struct RawModeGuard {
    fd: RawFd,
    original: termios::Termios,
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        // Restore original terminal settings (best-effort, ignore errors).
        let _ = termios::tcsetattr(self.fd, termios::SetArg::TCSANOW, &self.original);
    }
}
```

## 6. SIGWINCH Propagation Flow (Gap #4 -- DECIDED)

### 6.1 Sequence Diagram

```
Outer Terminal resizes
       |
       v
SIGWINCH delivered to TermForge server process
       |
       v
signal-hook pipe_fd becomes readable (IO thread)
       |
       v
IO thread: ioctl(TIOCGWINSZ) on controlling TTY
       |
       v
IO thread sends KernelEvent::ClientResize { client, new_size }
       |
       v
Kernel thread processes event:
  1. Update client.size
  2. Find session attached to client
  3. Recalculate window size (smallest-client policy)
  4. For each pane in the window:
     a. Recalculate layout (LayoutCell tree)
     b. Update pane.size
     c. Emit KernelEffect::PtyResize { pane, new_size }
     d. Call grid.reflow(new_width)
     e. Mark all lines dirty
  5. Bump global revision
  6. Emit KernelEffect::RenderSnapshot
       |
       v
IO thread receives PtyResize effects:
  For each pane: ioctl(TIOCSWINSZ, &winsize) on PTY master FD
       |
       v
Child process receives SIGWINCH (kernel delivers automatically
  when TIOCSWINSZ changes the PTY size)
       |
       v
Render thread receives snapshot, re-renders full screen
```

### 6.2 Invariants

- **INV-200**: SIGWINCH is handled via `signal-hook` pipe (never raw signal
  handlers). The IO thread registers `signal_hook::low_level::pipe::register(SIGWINCH)`.
- **INV-201**: TIOCGWINSZ is called on the *controlling TTY* (not the PTY master).
  For remote clients, the size comes from the Resize frame in TF01 protocol.
- **INV-202**: Window size follows smallest-client policy (same as tmux default).
  All clients attached to the same session see the same window size.
- **INV-203**: Grid reflow is a full reconstruction (not incremental). All lines
  are marked dirty after reflow.

## 7. Zombie Reaping and SIGCHLD (Gap #5 -- DECIDED)

### 7.1 Strategy

TermForge uses `signal-hook` to register a pipe for SIGCHLD. The IO thread polls
this pipe and calls `waitpid(-1, WNOHANG)` in a loop to reap all terminated
children.

```rust
// In IO thread initialization:
let sigchld_fd = signal_hook::low_level::pipe::register(
    signal_hook::consts::SIGCHLD,
    write_end,
)?;

// In IO thread event loop:
if sigchld_pipe_readable {
    loop {
        match nix::sys::wait::waitpid(
            Pid::from_raw(-1),
            Some(WaitPidFlag::WNOHANG),
        ) {
            Ok(WaitStatus::Exited(pid, status)) => {
                tx.send(KernelEvent::ChildExited { pid, status })?;
            }
            Ok(WaitStatus::Signaled(pid, signal, _)) => {
                tx.send(KernelEvent::ChildExited {
                    pid: pid.as_raw() as u32,
                    status: 128 + signal as i32,
                })?;
            }
            Ok(WaitStatus::StillAlive) | Err(_) => break,
            _ => continue,
        }
    }
}
```

### 7.2 Process Group Management

Each pane's child process is placed in its own process group via `setsid()` during
spawn. This ensures:

1. SIGCHLD is delivered to the TermForge server for the direct child.
2. Grandchildren are adopted by init/systemd when the direct child exits.
3. Job control signals (SIGTSTP, SIGCONT) are scoped to the pane's process group.
4. TermForge does NOT intercept signals destined for child process groups.

### 7.3 Invariants

- **INV-210**: SIGCHLD is handled via `signal-hook` pipe, never a raw handler.
- **INV-211**: `waitpid(-1, WNOHANG)` is called in a loop (not just once) to
  handle simultaneous child exits.
- **INV-212**: The kernel thread does NOT call `waitpid`. Only the IO thread reaps.
- **INV-213**: When a pane's child exits, the kernel marks the pane as "dead" but
  does not immediately destroy it (tmux behavior: dead panes remain visible until
  explicitly closed or respawned).

## 8. crossterm vs nix/rustix Decision (Gap #6 -- DECIDED)

### 8.1 Decision: nix + libc (NOT crossterm)

**Rationale**: crossterm is designed for applications that *use* a terminal (TUI
apps). A terminal multiplexer *is* a terminal -- it sits between the real terminal
and the child processes. crossterm's abstractions (Event, execute!, queue!) are for
the application layer, not the infrastructure layer.

Specific reasons:

1. **PTY management**: crossterm has no PTY support. TermForge needs
   `posix_openpt`, `grantpt`, `unlockpt`, `TIOCGPTPEER`, `TIOCSWINSZ` -- all raw
   ioctls that crossterm does not expose.

2. **Multi-client**: crossterm assumes a single terminal. TermForge serves N
   clients simultaneously, each with its own FD and size. crossterm's global
   `terminal::size()` and `enable_raw_mode()` are per-process, not per-FD.

3. **VT output generation**: crossterm writes escape sequences for a single screen.
   TermForge composes N pane grids into client-specific output. The composition
   pipeline generates raw escape sequences directly -- crossterm's command queue
   would add abstraction overhead without benefit.

4. **Signal handling**: crossterm uses its own signal handling for resize events.
   TermForge uses signal-hook for SIGWINCH and SIGCHLD, which would conflict.

5. **Input parsing**: crossterm uses its own input parser. TermForge has a custom
   VT parser (mux-parser) that handles tmux-specific sequences including DCS
   passthrough, which crossterm does not support.

### 8.2 What nix/libc provides

- `nix::pty`: posix_openpt, grantpt, unlockpt, ptsname_r
- `nix::sys::termios`: cfmakeraw, tcsetattr, tcgetattr
- `nix::sys::signal`: kill, sigaction (via signal-hook)
- `libc`: TIOCSWINSZ, TIOCGWINSZ, TIOCGPTPEER, SCM_RIGHTS
- `nix::sys::wait`: waitpid
- `nix::unistd`: fork, setsid, dup2, execvp

## 9. vt100/termwiz Evaluation (Gap #7 -- DECIDED)

### 9.1 Decision: Own VT Parser (NOT vt100 or termwiz)

**vt100 crate** (used by Zellij):
- Pros: Battle-tested, has `contents_diff()` for rendering.
- Cons: (a) Zellij delegates VT parsing entirely to vt100 -- TermForge cannot do
  this because it needs control over DCS passthrough for tmux protocol
  compatibility. (b) vt100 does not expose grapheme arena; it stores cell content
  as Rust strings, which is 3x the memory of TermForge's `GraphemeId` approach.
  (c) vt100's diff operates on text content, not on styled cells -- it cannot
  detect attribute-only changes (bold added to existing text).

**termwiz crate** (used by WezTerm):
- Pros: Rich Surface + Change log model, good for state sync.
- Cons: (a) Designed for WezTerm's GUI rendering pipeline, not server-side
  terminal composition. (b) Change log accumulates unboundedly; needs periodic
  drain. (c) Large dependency (pulls in GUI-related code).

**Own VT parser (mux-parser)**:
- Full control over tmux-specific sequences (DCS passthrough, OSC 8 hyperlinks,
  Kitty keyboard protocol).
- Paul Williams state machine with exact state transitions.
- Zero-copy: operates on byte slices, produces `VtAction` enum values.
- Tight integration with `GraphemeArena` for memory-efficient cell storage.
- Can be tested deterministically without terminal I/O.

### 9.2 Parsing Architecture

```
PTY output bytes --> VtParser (state machine) --> VtAction enum
                                                      |
                                                      v
                                              Kernel event handler:
                                              - Print -> grid cell mutation
                                              - CsiDispatch -> cursor/scroll/SGR
                                              - OscDispatch -> title/hyperlink
                                              - DcsHook/Put/Unhook -> passthrough
```

## 10. Copy Mode (Gap #8 -- DECIDED)

### 10.1 Design

Copy mode is a pane-level state machine that intercepts key input before it
reaches the PTY. When active, the pane displays a scrollback view with cursor
navigation, text selection, and search.

### 10.2 State Machine

```rust
/// Copy mode state, matching tmux window-copy.c's window_copy_mode_data.
pub struct CopyModeState {
    /// Scroll offset: number of lines scrolled up from the bottom.
    pub oy: u32,
    /// Cursor position within the scrollback view.
    pub cx: u32,
    pub cy: u32,
    /// Selection anchor (start of selection).
    pub sel: Option<SelectionAnchor>,
    /// Selection mode.
    pub sel_mode: SelectionMode,
    /// Whether rectangle (block) selection is active.
    pub rect_select: bool,
    /// Key binding mode (vi or emacs).
    pub mode_keys: ModeKeys,
    /// Search state.
    pub search: Option<SearchState>,
    /// Mark position (for mark-and-jump).
    pub mark: Option<(u32, u32)>,
    /// Last cursor x position (for vertical movement).
    pub last_cx: u32,
}

pub struct SelectionAnchor {
    pub x: u32,
    pub y: u32,
}

pub enum SelectionMode {
    Char,
    Word,
    Line,
}

pub enum ModeKeys {
    Vi,
    Emacs,
}

pub struct SearchState {
    pub pattern: String,
    pub direction: SearchDirection,
    pub is_regex: bool,
    pub marks: Vec<(u32, u32, u32)>,  // (line, start_col, end_col)
    pub current_match: usize,
}

pub enum SearchDirection {
    Forward,
    Backward,
}
```

### 10.3 Copy Mode Key Tables

Vi mode key bindings (subset, matching tmux defaults):
| Key | Action |
|-----|--------|
| q | Exit copy mode |
| h/j/k/l | Cursor movement |
| 0/$ | Line start/end |
| w/b/e | Word movement |
| Space | Begin selection |
| Enter | Copy selection and exit |
| / | Search forward |
| ? | Search backward |
| n/N | Next/previous search match |
| v | Toggle rectangle selection |
| V | Line selection |
| g/G | Top/bottom of scrollback |
| Ctrl-u/d | Half-page up/down |

### 10.4 Integration with Kernel

Copy mode is entered via a `KernelEvent::Command` ("copy-mode") or by scrolling
up with the mouse wheel. The kernel sets `pane.copy_mode = Some(CopyModeState)`.
While copy mode is active:

1. Key events for this pane are dispatched to the copy mode handler, not the PTY.
2. The render thread uses `pane.copy_mode.oy` to offset the viewport into history.
3. Search highlights are rendered as inverted cells.
4. The selection is rendered with the `SELECTED` cell flag.

When the user copies (Enter in vi mode), the selected text is placed into the
paste buffer stack. Copy mode is exited and the pane returns to live view.

## 11. Graphics Protocol Stance (Gap #9 -- DECIDED)

### 11.1 Decision: Passthrough with Future Native Support

**Phase 1 (P2)**: DCS/OSC passthrough. Graphics escape sequences from the child
process are forwarded to the client terminal without interpretation. This matches
tmux's `allow-passthrough` option behavior.

Implementation:
- The VT parser recognizes DCS and OSC sequences and emits them as
  `VtAction::DcsHook`/`DcsUnhook` and `VtAction::OscDispatch`.
- The kernel stores passthrough sequences in a per-pane buffer.
- The render thread emits them in the correct screen region using DCS
  tmux passthrough wrapping if the client supports it.

**Phase 2 (P3)**: Native sixel/Kitty support. Image data is stored in the grid as
special cells referencing an image ID. The render thread translates to the client's
supported protocol (sixel for xterm, Kitty protocol for Kitty, iTerm2 for
iTerm2/WezTerm).

### 11.2 Protocols

| Protocol | Detection | Status |
|----------|-----------|--------|
| Sixel | DA2 response, DECRQM(80) | P2: passthrough, P3: native |
| Kitty graphics | `_Gi=31,s=1,v=1,a=q` probe | P2: passthrough, P3: native |
| iTerm2 inline | `TERM_PROGRAM=iTerm.app` | P2: passthrough |
| tmux passthrough | `\ePtmux;\e...\e\\` | P1: wrap/unwrap support |

### 11.3 Invariant

- **INV-220**: Graphics passthrough is disabled by default. Enabled via option
  `set -g allow-passthrough on` (matching tmux behavior). This is a security
  consideration: arbitrary DCS sequences can be used for terminal injection.

## 12. signal-hook vs tokio Signals (Gap #10 -- DECIDED)

### 12.1 Decision: signal-hook for SIGCHLD + SIGWINCH, tokio for SIGTERM/SIGINT

**Rationale**:
- `signal-hook` (127M downloads) provides `low_level::pipe::register()` which
  writes a byte to a pipe on signal delivery. This integrates cleanly with the
  tokio event loop (register the pipe read end as an `AsyncFd`).
- tokio's built-in signal support uses `signal_hook_registry` internally anyway.
- SIGCHLD requires `waitpid(-1, WNOHANG)` in a loop, which is more naturally
  expressed with signal-hook's pipe approach than tokio's stream.
- SIGWINCH requires `TIOCGWINSZ` ioctl immediately after delivery, not just
  notification.
- SIGTERM and SIGINT are simple shutdown signals -- tokio's `signal::ctrl_c()`
  is fine for these.

### 12.2 Registration

```rust
// In IO thread setup:
use signal_hook::consts::signal::*;

let (sigwinch_read, sigwinch_write) = nix::unistd::pipe()?;
signal_hook::low_level::pipe::register(SIGWINCH, sigwinch_write)?;

let (sigchld_read, sigchld_write) = nix::unistd::pipe()?;
signal_hook::low_level::pipe::register(SIGCHLD, sigchld_write)?;

// Wrap read ends as tokio AsyncFd for integration with the event loop.
```

### 12.3 Signal Non-Interference Invariant

- **INV-230**: TermForge MUST NOT install signal handlers for SIGTSTP, SIGCONT,
  SIGPIPE, SIGURG, or SIGUSR1/SIGUSR2. These signals may be used by child
  processes and must pass through unmodified.
- **INV-231**: TermForge MUST set `SA_RESTART` for all signal handlers to avoid
  interrupted syscall errors (EINTR) in child processes.

## 13. SCM_RIGHTS File Descriptor Passing (from GPT)

### 13.1 Purpose

tmux uses `SCM_RIGHTS` over Unix domain sockets to pass PTY file descriptors
from the server to the client process. This enables the client process to
directly write to the PTY without proxying through the server.

### 13.2 TermForge Approach

In TF01 protocol mode, fd passing is not used -- the server proxies all I/O.
For tmux compatibility mode (imsg protocol), `SCM_RIGHTS` is implemented in
`mux-proto` using `nix::sys::socket::sendmsg/recvmsg` with `ControlMessage::ScmRights`.

```rust
use nix::sys::socket::{sendmsg, recvmsg, ControlMessage, MsgFlags};

pub fn send_fd(socket: RawFd, fd: RawFd) -> nix::Result<()> {
    let cmsg = [ControlMessage::ScmRights(&[fd])];
    let iov = [std::io::IoSlice::new(b"\0")]; // dummy byte
    sendmsg::<()>(socket, &iov, &cmsg, MsgFlags::empty(), None)?;
    Ok(())
}
```

## 14. CRDT for Collaborative Sessions (from GPT)

### 14.1 Decision: Deferred to P3

CRDTs (Conflict-free Replicated Data Types) enable collaborative editing of the
session tree (multiple users attaching to the same session and making structural
changes simultaneously). This is a P3 feature.

### 14.2 Architecture Sketch

The entity model (sessions, windows, panes, options) can be modeled as a CRDT
using Lamport timestamps for conflict resolution:

- Each entity change carries a vector clock `(client_id, sequence)`.
- Concurrent creates are resolved by client_id ordering.
- Concurrent deletes win over creates (last-writer-wins register).
- Option changes use LWW-Register semantics.

This is NOT implemented in P1/P2 -- the single-server model with mutex-free
kernel makes CRDT unnecessary until multi-server federation is added.

## 15. Configuration System (from GPT)

### 15.1 Decision: TOML Config with Hot-Reload

TermForge supports two config file formats:
- `.tmux.conf`: tmux-compatible command sequence (parsed by mux-cmd-parse).
- `termforge.toml`: TOML-based config for SDK embedding.

### 15.2 Config Hierarchy

```
$XDG_CONFIG_HOME/termforge/config.toml   (user config)
/etc/termforge/config.toml               (system config)
~/.tmux.conf                             (tmux compat, lower priority)
Programmatic via MuxServer::builder()    (highest priority)
```

### 15.3 Hot-Reload

Config changes are watched via `notify` crate (P2). On change:
1. Parse the new config.
2. Validate all options.
3. If validation passes, atomically swap the config.
4. If validation fails, reject and log a warning (atomic rejection).
5. Emit a `KernelEvent::ConfigReloaded` so the kernel can apply changes.

### 15.4 TOML Schema (mux-config crate)

```rust
#[derive(Debug, Deserialize)]
pub struct TermForgeConfig {
    pub server: ServerConfig,
    pub session: SessionConfig,
    pub keybindings: Vec<KeyBindingConfig>,
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub socket_path: Option<PathBuf>,
    pub default_shell: String,
    pub history_limit: u32,
    pub escape_time: u32,       // milliseconds
    pub focus_events: bool,
    pub allow_passthrough: bool,
}
```

## 16. Panic Recovery Architecture (from Gemini)

### 16.1 Strategy

If any thread panics:

1. The panic is caught by `std::panic::set_hook()`.
2. A crash report is logged (backtrace, last known state).
3. If the kernel thread panics, it sends a "crash report" frame to all connected
   clients before shutting down. This frame contains:
   - A human-readable error message.
   - The backtrace (if `RUST_BACKTRACE=1`).
4. The server exits with code 1.

### 16.2 FFI Boundary

At the `mux-ffi` boundary, all extern "C" functions use `catch_unwind` to prevent
Rust panics from unwinding across the C ABI (which is undefined behavior).

```rust
#[unsafe(no_mangle)]
pub extern "C" fn grid_create(sx: c_uint, sy: c_uint, hlimit: c_uint) -> *mut FfiGrid {
    std::panic::catch_unwind(|| {
        // ... Rust code ...
    }).unwrap_or(std::ptr::null_mut())
}
```

## 17. Async Boundary for Language Bindings (from GPT)

### 17.1 Design

Language bindings (Python via PyO3, Node.js via napi-rs) wrap `mux-api`. The
async boundary is:

- **Python**: PyO3 `asyncio` integration. `MuxServer` exposes `async def` methods
  that internally bridge to the Rust tokio runtime via `pyo3_asyncio`.
- **Node.js**: napi-rs with `AsyncTask` trait. The tokio runtime runs on a
  background thread; napi callbacks are dispatched to the Node.js event loop.

Both bindings use `mux-api`'s synchronous `MuxServer` handle, which internally
communicates with the kernel via crossbeam channels. The bindings add async
wrappers on top.

## 18. Layer Diagram (Complete)

```
+---------------------------------------------------------------------------+
|  L6: Tools                                                                 |
|  tmux-builder | tmux-vm | tmux-sniff | mux-doctor                         |
+---------------------------------------------------------------------------+
|  L5: SDK + Integration                                                     |
|  mux-api | mux-test-support | mux-ffi | mux-otel                          |
+---------------------------------------------------------------------------+
|  L4: Kernel + Termlet                                                      |
|  mux-kernel (7,600 LOC target)                                             |
|    - Entity model (Session, Window, Pane, Client)                          |
|    - Layout engine (tree, 7 algorithms)                                    |
|    - Copy mode state machine                                               |
|    - Key binding dispatch                                                  |
|    - Mouse dispatch                                                        |
|    - Buffer management                                                     |
|    - Command execution (~140 commands)                                     |
|  mux-termlet (test pods)                                                   |
+---------------------------------------------------------------------------+
|  L3: Snapshot + Parsing + Render + ORM + PTY                               |
|  mux-snapshot | mux-cmd-parse | mux-orm | mux-render | mux-pty             |
+---------------------------------------------------------------------------+
|  L2: Grid + Parser + Protocol + Options + Target + Format + Config         |
|  mux-grid | mux-parser | mux-proto | mux-options | mux-target |            |
|  mux-format | mux-config                                                   |
+---------------------------------------------------------------------------+
|  L1: Types                                                                 |
|  mux-types (Cell, CellFlags, Colour, Attrs, IDs, Size, Errors)            |
+---------------------------------------------------------------------------+
|  L0: Leaf crates (zero internal deps)                                      |
|  mux-grapheme-arena | mux-time                                             |
+---------------------------------------------------------------------------+
```
