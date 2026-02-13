# TermForge Architecture -- v0002 Pass 2 (FINAL)

## Table of Contents

1. [System Overview](#1-system-overview)
2. [Crate DAG and Layering](#2-crate-dag-and-layering)
3. [Rendering Pipeline](#3-rendering-pipeline)
4. [Sans-IO Kernel](#4-sans-io-kernel)
5. [Entity Model](#5-entity-model)
6. [Layout Engine](#6-layout-engine)
7. [VT Parser](#7-vt-parser)
8. [Grid Architecture](#8-grid-architecture)
9. [Copy Mode](#9-copy-mode)
10. [Protocol Layer](#10-protocol-layer)
11. [Configuration System](#11-configuration-system)
12. [Threading Model](#12-threading-model)
13. [Signal Handling](#13-signal-handling)
14. [CRDT Strategy](#14-crdt-strategy)
15. [SCM_RIGHTS fd Passing](#15-scm_rights-fd-passing)
16. [Panic Crash Frame and Ordered Shutdown](#16-panic-crash-frame-and-ordered-shutdown)
17. [Flood Fairness Model](#17-flood-fairness-model)
18. [Testing Strategy](#18-testing-strategy)
19. [SDK API Design](#19-sdk-api-design)
20. [FFI Layer](#20-ffi-layer)

---

## 1. System Overview

TermForge is a Rust-native terminal multiplexer SDK that provides tmux v3.5a
protocol compatibility while offering a modern, embeddable API. The core
architecture follows three principles:

1. **Sans-IO Kernel**: All mutable multiplexer state lives in a single-threaded
   kernel that processes events and emits effects. The kernel never performs I/O.
   This makes it fully deterministic and testable.

2. **Double-Buffer Diff Rendering**: The renderer maintains two CompositeGrids
   (prev/next). Each render cycle writes the new state to "next", diffs against
   "prev", and emits only the minimal escape sequences needed to update the
   terminal.

3. **Arc-COW Grid Lines**: Terminal grid lines use `Arc<Vec<Cell>>` for
   copy-on-write semantics. Snapshots for the render pipeline are created by
   cloning Arc pointers, not by copying cell data.

### High-Level Data Flow

```
  PTY read -> IO Thread -> KernelEvent -> Kernel.process_event()
                                              |
                                        Vec<KernelEffect>
                                              |
                              +---------------+---------------+
                              |               |               |
                         WritePty        SpawnChild         Render
                              |               |               |
                         IO Thread       IO Thread      Render Thread
                                                             |
                                                     GridSnapshot (COW)
                                                             |
                                                     CompositeGrid
                                                             |
                                                        diff(prev, next)
                                                             |
                                                     RenderOutput -> TTY write
```

### Design Goals

- **Embeddable**: The `mux-api` crate provides a builder-pattern SDK that can
  be embedded in any Rust application (terminal emulators, IDEs, CI tools).
- **tmux Compatible**: Target 100% command compatibility with tmux v3.5a for
  the 59 built-in commands, plus wire protocol support for existing clients.
- **Zero Panic**: All public APIs return Result. Panic is denied at the clippy
  level. FFI boundaries use catch_unwind.
- **Observable**: Optional OpenTelemetry integration via the mux-otel crate.

---

## 2. Crate DAG and Layering

The workspace is organized into 6 layers. Each layer may only depend on
layers below it. This prevents circular dependencies and ensures clear
module boundaries.

### Layer 0 -- Leaf Crates (no internal dependencies)

| Crate | Purpose |
|-------|---------|
| mux-grapheme-arena | Arena allocator for extended grapheme clusters |
| mux-time | Injectable Clock for deterministic testing |

### Layer 1 -- Core Types

| Crate | Purpose | Key Types |
|-------|---------|-----------|
| mux-types | Cell, CellFlags, Colour, Attrs, entity IDs, Size, errors | Cell, PaneId, Size |

### Layer 2 -- Data Structures

| Crate | Purpose | Dependencies |
|-------|---------|-------------|
| mux-grid | Chunked COW grid with dirty tracking | mux-types, mux-grapheme-arena |
| mux-parser | VT state machine (Paul Williams) | mux-types, mux-grapheme-arena |
| mux-options | Typed layered option tables | mux-types |
| mux-target | tmux target syntax parser | (thiserror only) |
| mux-format | Format string #{} evaluator | (thiserror only) |
| mux-cmd-parse | Command/config file parser | (thiserror only) |
| mux-proto | TF01 wire protocol | (bytes, crc32fast, thiserror) |

### Layer 3 -- Business Logic

| Crate | Purpose |
|-------|---------|
| mux-snapshot | Grid snapshot serialization and text extraction |
| mux-kernel | Sans-IO kernel: entities, layout, copy mode, dispatch |
| mux-render | CompositeGrid, diff, output, flood fairness |
| mux-orm | ORM-like QuerySet traversal API |
| mux-pty | PTY allocation, raw mode guard (unsafe quarantine) |

### Layer 4 -- Integration

| Crate | Purpose |
|-------|---------|
| mux-termlet | Testing pods (PTY + VT + shell) |
| mux-config | TOML configuration system |
| mux-api | SDK entry point (MuxServer builder) |
| mux-test-support | Shared test harness and fixtures |

### Layer 5 -- External Surface

| Crate | Purpose |
|-------|---------|
| mux-ffi | C FFI shim with catch_unwind (unsafe quarantine) |
| mux-otel | OpenTelemetry tracing integration |

### Layer 6 -- Tools

| Tool | Purpose |
|------|---------|
| tmux-builder | YAML/TOML workspace layout tool |
| tmux-vm | Headless multiplexer for CI |
| tmux-sniff | Wire protocol inspector |
| mux-doctor | Diagnostic environment checker |

### Full Dependency Graph

```
L0: mux-grapheme-arena  mux-time
     |                    |
L1:  mux-types -----+----+
     |               |
L2:  mux-grid     mux-parser  mux-options  mux-target  mux-format  mux-cmd-parse  mux-proto
     |               |            |            |           |            |
L3:  mux-snapshot  mux-kernel <--+-----------+------------+------------+
     |               |
     mux-render    mux-orm    mux-pty
     |               |          |
L4:  mux-termlet  mux-config  mux-api  mux-test-support
     |               |          |          |
L5:  mux-ffi      mux-otel
     |
L6:  tmux-builder  tmux-vm  tmux-sniff  mux-doctor
```

---

## 3. Rendering Pipeline

### Double-Buffer Architecture

The renderer maintains two `CompositeGrid` instances:

1. **prev**: The state currently displayed on the terminal.
2. **next**: The state being assembled for the next frame.

Each render cycle:
1. Clear "next".
2. Blit each visible pane's grid snapshot into "next" (with flood fairness).
3. Draw borders and status line into "next".
4. Compute `diff(prev, next)` -> `Vec<CellUpdate>`.
5. Emit escape sequences via `RenderOutput`.
6. Swap "prev" and "next" (`std::mem::swap`).

### Cell Source Tracking

Each cell in the CompositeGrid knows its source:
- `Pane(PaneId)`: Content from a specific pane.
- `Border`: Pane separator character.
- `StatusLine`: Status bar content.
- `Empty`: Unused area.

This enables click-to-pane mapping and selective invalidation.

### SGR Optimization

The `RenderOutput` maintains current attribute state (`attrs`, `fg`, `bg`).
SGR sequences are only emitted when the new cell's attributes differ from
the current state.

### CUP Optimization

Cursor position (CUP) sequences are elided when the next cell is at
`cursor_x + cell_width, cursor_y` -- the terminal auto-advances the
cursor after character output.

### Wide Character Handling

When a cell has `width > 1` (CJK character), the diff algorithm marks
the following padding cells as invalidated, ensuring they are re-emitted
to prevent visual corruption.

---

## 4. Sans-IO Kernel

The kernel is the heart of TermForge. It owns all mutable state and
processes events via a pure reducer function:

```
fn process_event(&mut self, event: KernelEvent) -> Vec<KernelEffect>
```

### Event Types

| Event | Source | Description |
|-------|--------|-------------|
| Command | Client | A tmux command string ("new-session -d -s foo") |
| PtyOutput | IO thread | Data from a pane's child process |
| ClientResize | IO thread | Client terminal size changed |
| PaneExited | IO thread | Child process exited |
| ClientConnected | IO thread | New client attached |
| ClientDisconnected | IO thread | Client detached |
| Tick | Timer | Periodic timer for escape-time, status updates |

### Effect Types

| Effect | Executor | Description |
|--------|----------|-------------|
| WritePty | IO thread | Write bytes to a pane's PTY master |
| ResizePty | IO thread | Set pane PTY window size |
| SpawnChild | IO thread | Fork + exec a new child process |
| KillChild | IO thread | Send signal to child process |
| Render | Render thread | Trigger render for a client |
| CloseClient | IO thread | Close a client connection |
| CommandResponse | IO thread | Send command result to client |
| Shutdown | IO thread | Initiate server shutdown |

### Deterministic Testing

Because the kernel is Sans-IO:
- Time is injected via `Clock::manual()`.
- All I/O effects are inspectable as data.
- Tests can replay event sequences and verify exact effect outputs.
- No flaky tests from timing, network, or filesystem.

---

## 5. Entity Model

### Hierarchy

```
Server
  +-- Session (SessionId)
  |     +-- Window (WindowId)
  |     |     +-- Pane (PaneId)
  |     |     |     +-- ChunkedGrid
  |     |     |     +-- VtParser
  |     |     |     +-- GraphemeArena
  |     |     |     +-- OptionTable
  |     |     +-- LayoutNode
  |     |     +-- OptionTable
  |     +-- OptionTable
  +-- Client (ClientId)
        +-- attached Session
        +-- terminal Size
```

### ID Generation

Entity IDs are monotonically increasing u64 values. They are never reused.
This simplifies garbage collection and prevents use-after-free bugs.

### Option Inheritance

Options are resolved via layered lookup:
1. Pane-level option table
2. Window-level option table
3. Session-level option table
4. Server-level option table
5. Built-in defaults

---

## 6. Layout Engine

### Binary Tree Model

The layout engine uses a binary tree of `LayoutNode`s. Each node is either:
- `Pane`: A leaf node containing a single pane.
- `LeftRight`: A horizontal split container.
- `TopBottom`: A vertical split container.

This matches tmux's `layout_cell` tree from `layout.c`.

### Built-in Algorithms

| Algorithm | Behavior |
|-----------|----------|
| even-horizontal | Equal-width vertical splits |
| even-vertical | Equal-height horizontal splits |
| main-horizontal | Main pane on top, rest split below |
| main-vertical | Main pane on left, rest split right |
| tiled | Grid arrangement (sqrt-based) |

### Dimension Redistribution

When the terminal is resized, the layout engine redistributes space:
1. Compute the ratio of each child to the old parent size.
2. Apply the ratio to the new parent size.
3. Distribute remainder pixels left-to-right / top-to-bottom.
4. Ensure no pane is below minimum size (2 cols x 1 row).
5. The sum of all child dimensions exactly equals the parent size.

### Custom Layout Strings

tmux supports custom layout strings with a checksum format:
```
89eb,120x40,0,0{60x40,0,0,0,59x40,61,0,1}
```

The parser validates the 4-hex-digit checksum before applying the layout.

---

## 7. VT Parser

### Paul Williams State Machine

The parser implements the 14-state Paul Williams VT state machine:

| State | Description |
|-------|-------------|
| Ground | Normal text processing |
| Escape | ESC received |
| EscapeIntermediate | ESC + intermediate chars |
| CsiEntry | CSI (ESC [) received |
| CsiParam | CSI parameter accumulation |
| CsiIntermediate | CSI intermediate chars |
| CsiIgnore | Malformed CSI, consuming to final |
| OscString | OSC (ESC ]) string data |
| DcsEntry | DCS (ESC P) received |
| DcsParam | DCS parameter accumulation |
| DcsIntermediate | DCS intermediate chars |
| DcsPassthrough | DCS data bytes |
| DcsIgnore | Malformed DCS |
| SosPmApcString | SOS/PM/APC (consumed and discarded) |

### Action Types

| Action | Description |
|--------|-------------|
| Print(char) | Write character at cursor |
| Execute(u8) | Process C0 control (BEL, BS, CR, LF, etc.) |
| CsiDispatch | CSI sequence complete (SGR, CUP, ED, EL, etc.) |
| EscDispatch | ESC sequence complete (DECSC, DECRC, etc.) |
| OscDispatch | OSC string complete (title, clipboard, etc.) |
| DcsHook | DCS sequence started |
| DcsPut | DCS data byte |
| DcsUnhook | DCS sequence complete |

### CSI Sequence Coverage

The full implementation targets ~40 CSI sequences matching tmux's input.c:
- Cursor movement: CUP, CUU, CUD, CUF, CUB, CNL, CPL, CHA, VPA, HPA
- Editing: ICH, DCH, IL, DL, ECH, ED, EL
- Scrolling: SU, SD, DECSTBM
- Modes: SM, RM, DECSM, DECRM
- Attributes: SGR (including 256-color and true-color)
- Device status: DSR, DA, DECRPM
- Tab: HTS, TBC

---

## 8. Grid Architecture

### Chunked Storage

Lines are stored in chunks of 64 (`CHUNK_SIZE`). This provides:
- Cache-friendly sequential access within a chunk.
- Efficient scrollback trimming (drop entire chunks).
- Good balance between overhead and granularity.

### Arc-based COW

Each `Line` stores its cells as `Arc<Vec<Cell>>`. Benefits:
- **Snapshot isolation**: Creating a grid snapshot for the renderer only
  increments reference counts on the Arc pointers.
- **Lazy copy**: Lines are only physically copied when mutated via
  `Arc::make_mut`, which clones the inner Vec only if `strong_count > 1`.

### Dirty Tracking

A `Vec<bool>` tracks which active-screen rows have been modified since
the last render. The diff algorithm only processes dirty lines, reducing
render work for static content.

### Scrollback Management

- Active screen: the bottom `sy` lines.
- Scrollback: everything above the active screen, bounded by `hlimit`.
- When `total_lines > sy + hlimit`, excess lines are trimmed from the front.

### Cell Structure

```rust
pub struct Cell {
    grapheme: GraphemeId,   // Arena-allocated grapheme cluster
    width: u8,              // 0=padding, 1=normal, 2=CJK
    flags: CellFlags,       // PADDING, EXTENDED, SELECTED, CLEARED, TAB
    attrs: Attrs,           // BOLD, DIM, ITALIC, UNDERSCORE, BLINK, REVERSE, etc.
    fg: Colour,             // Default | Indexed(u8) | Rgb{r,g,b}
    bg: Colour,
    us: Colour,             // Underline colour
    link: u32,              // Hyperlink ID (OSC 8)
}
```

### CellFlags Bit Values (INV-119)

Verified against tmux source `tmux.h:742-749`:
```
PADDING  = 0x04  (GRID_FLAG_PADDING)
EXTENDED = 0x08  (GRID_FLAG_EXTENDED)
SELECTED = 0x10  (GRID_FLAG_SELECTED)
CLEARED  = 0x40  (GRID_FLAG_CLEARED)
TAB      = 0x80  (GRID_FLAG_TAB)
```

---

## 9. Copy Mode

### State Machine

Copy mode is entered per-pane via the `copy-mode` command. The state
machine tracks:

- **Key table**: Vi or Emacs (explicit, never guessed).
- **Selection mode**: None, Character, Line, or Block.
- **Cursor position**: Independent of the pane's normal cursor.
- **Scroll offset**: How far into scrollback the view is.
- **Search state**: Term, direction (forward/backward).
- **Buffer ring**: Bounded list of copied text (default max 50).

### Selection Modes

| Mode | Vi Key | Emacs Key | Behavior |
|------|--------|-----------|----------|
| Character | Space | Ctrl-Space | Select individual characters |
| Line | V | (not default) | Select entire lines |
| Block | Ctrl-V | (not default) | Rectangle selection |

### Buffer Ring

Copied text is pushed to a bounded ring buffer. When the ring reaches
`max_buffers` (default 50), the oldest entry is evicted.

`paste-buffer` retrieves the most recent entry. Named buffers are
supported for explicit targeting.

---

## 10. Protocol Layer

### TF01 Frame Format

```
Offset  Size   Field
[0..4]  u32    magic: 0x54464F31 ("TFO1")
[4..6]  u16    type (LE)
[6..8]  u16    flags (LE)
[8..12] u32    payload_len (LE)
[12..16] u32   CRC32 of payload
[16..N] [u8]   payload bytes
```

### Frame Types

| Type | Value | Direction | Description |
|------|-------|-----------|-------------|
| Hello | 0x0001 | C->S | Client hello with capabilities |
| Data | 0x0002 | S->C | Rendered output data |
| Resize | 0x0003 | C->S | Client terminal size change |
| Command | 0x0004 | C->S | tmux command string |
| CommandResponse | 0x0005 | S->C | Command result |
| KeyInput | 0x0006 | C->S | Key/mouse input event |
| Shutdown | 0x0007 | Both | Graceful shutdown request |
| Error | 0x00FF | S->C | Protocol-level error |

### CRC32 Validation

Every frame payload is validated with CRC32 (using the `crc32fast` crate).
This catches:
- Transmission corruption.
- Truncated writes.
- Buffer overruns from incorrect length fields.

### tmux imsg Compatibility

For backward compatibility with existing tmux clients, the server can
detect incoming connections by checking the first 4 bytes:
- If `0x54464F31` (TFO1): use TF01 protocol.
- Otherwise: fall back to tmux imsg format.

---

## 11. Configuration System

### File Hierarchy (highest to lowest priority)

1. Programmatic: `MuxServer::builder().history_limit(50_000)`
2. User config: `$XDG_CONFIG_HOME/termforge/config.toml`
3. System config: `/etc/termforge/config.toml`
4. tmux compat: `~/.tmux.conf` (parsed via mux-cmd-parse)

### TOML Schema

```toml
[server]
socket_path = "/tmp/termforge.sock"
default_shell = "/bin/zsh"
history_limit = 50000
escape_time = 100
focus_events = true
allow_passthrough = false
default_terminal = "tmux-256color"
mouse = true

[session]
base_index = 1
renumber_windows = true
status = true
status_position = "top"
status_left = "#S"
status_right = "%H:%M %d-%b-%y"

[[keybindings]]
key = "C-a"
root = false
command = "send-prefix"
```

### .tmux.conf Compatibility

The `mux-cmd-parse` crate parses `.tmux.conf` files into command sequences.
Features supported:
- `#` comment lines.
- Quoted strings (single and double).
- Backslash continuation.
- Semicolon command chaining.

---

## 12. Threading Model

### Three-Thread Architecture

```
+------------------+     crossbeam     +------------------+
|                  |     channels      |                  |
|   IO Thread      | <===============> |   Kernel Thread  |
|   (tokio)        |  KernelEvent      |   (std::thread)  |
|                  |  KernelEffect     |                  |
+------------------+                   +------------------+
         |                                       |
         |                              GridSnapshot (COW)
         |                                       |
         |                             +------------------+
         +----------------------------> |  Render Thread   |
              rendered bytes            |  (tokio task)    |
                                        +------------------+
```

### IO Thread (tokio)

- Owns all file descriptors (PTY masters, Unix socket, signals).
- Drives async I/O via tokio::io.
- Translates I/O events into KernelEvents.
- Executes KernelEffects (WritePty, SpawnChild, etc.).

### Kernel Thread (std::thread)

- Owns the single Kernel instance.
- Runs a tight loop: receive event, process, send effects.
- No I/O, no blocking, no async.
- Fully deterministic when using Clock::manual().

### Render Thread (tokio task)

- Receives GridSnapshots from the kernel (via Arc-COW).
- Builds CompositeGrid, runs diff, emits escape sequences.
- Writes rendered output to client connections.

### Channel Design

- `crossbeam-channel` bounded channels between IO and Kernel.
- Channel capacity: 256 events, 64 effects.
- Backpressure: IO thread blocks on send if kernel falls behind.

---

## 13. Signal Handling

### SIGWINCH (Terminal Resize)

1. `signal-hook` registers a pipe-based SIGWINCH handler.
2. IO thread reads from signal pipe via tokio AsyncFd.
3. IO thread queries terminal size via `ioctl(TIOCGWINSZ)`.
4. IO thread sends `KernelEvent::ClientResize`.
5. Kernel updates client size, recalculates layouts, emits `KernelEffect::Render`.
6. Render thread produces new frame with updated dimensions.

### SIGCHLD (Child Process Exit)

1. `signal-hook` registers pipe-based SIGCHLD handler.
2. IO thread reads from signal pipe.
3. IO thread drains `waitpid(-1, WNOHANG)` in a loop until no more children.
4. For each reaped child, sends `KernelEvent::PaneExited`.
5. Kernel may close the pane, respawn, or keep as zombie.

### SIGTERM / SIGINT (Graceful Shutdown)

1. Signal handler sets an atomic flag.
2. IO thread detects flag on next poll cycle.
3. IO thread sends `KernelEvent::Command { command: "kill-server" }`.
4. Kernel processes shutdown, emitting effects to close clients.
5. IO thread executes CloseClient effects and exits.

### Design Rationale

Signal handlers only write to pipes. All logic is in the IO thread
and kernel. This avoids async-signal-safety issues and keeps the kernel
Sans-IO.

---

## 14. CRDT Strategy

### Motivation

Collaborative terminal sessions (multiple users editing the same pane
simultaneously) require conflict resolution. TermForge uses a CRDT
(Conflict-free Replicated Data Type) approach for eventual convergence.

### Vector Clock Dominance

Each server in a multi-server federation maintains a vector clock.
Events are ordered by vector clock dominance:
- Event A dominates Event B if A's vector clock is >= B's in all dimensions
  and strictly > in at least one.
- If neither dominates, the events are concurrent.

### Last-Writer-Wins (LWW) Register

For individual cell conflicts (two users write to the same grid position):
- Each cell write carries a timestamp and origin server ID.
- When concurrent writes are detected, the one with the highest timestamp
  wins. Ties are broken by server ID (total order).

### Convergence Guarantee

All replicas that see the same set of events will converge to the same
state, regardless of the order in which events are received.

### Testing Strategy

Property-based tests (proptest) generate random concurrent mutation
sequences and verify that all replicas converge to identical grids.

---

## 15. SCM_RIGHTS fd Passing

### Architecture

For tmux compatibility mode, the server and client may need to pass
file descriptors (PTY peers, stdin/stdout) between processes. This
uses Unix domain socket ancillary data.

### Implementation

```rust
// Server side: pass a PTY peer fd to a newly connecting client.
use nix::sys::socket::{sendmsg, ControlMessage, MsgFlags};
use std::os::unix::io::RawFd;

fn send_fd(socket: RawFd, fd_to_send: RawFd, data: &[u8]) {
    let cmsg = [ControlMessage::ScmRights(&[fd_to_send])];
    let iov = [std::io::IoSlice::new(data)];
    sendmsg::<()>(socket, &iov, &cmsg, MsgFlags::empty(), None).unwrap();
}
```

### Use Cases

1. **Client attach**: When a client attaches to an existing session,
   the server may pass the PTY master fd to the client for direct I/O
   (bypassing the server for data-plane performance).

2. **Process migration**: Moving a pane between server instances requires
   passing the PTY master fd via SCM_RIGHTS over a Unix socket.

### Security

- Only Unix domain sockets support SCM_RIGHTS (not TCP).
- The receiving process must validate the fd type (stat + fstat).
- File descriptor limits are checked before accepting passed fds.

### TF01 Mode

SCM_RIGHTS is NOT used in TF01 mode. The TF01 protocol handles all
data passing through the wire protocol frames.

---

## 16. Panic Crash Frame and Ordered Shutdown

### Panic Recovery (INV-117)

The FFI boundary (`mux-ffi`) uses `std::panic::catch_unwind` to prevent
Rust panics from unwinding through C code (which is undefined behavior).

```rust
#[no_mangle]
pub extern "C" fn grid_create(sx: u32, sy: u32, hlimit: u32) -> *mut FfiGrid {
    std::panic::catch_unwind(|| {
        Box::into_raw(Box::new(FfiGrid { inner: ChunkedGrid::new(sx, sy, hlimit) }))
    })
    .unwrap_or(std::ptr::null_mut())
}
```

### Workspace-Level Panic Prevention

- `profile.release.panic = "unwind"`: Required for catch_unwind.
- Clippy denies `unwrap_used`, `expect_used`, `panic` in all crates.
- `Option::unwrap_or`, `Result::unwrap_or_else`, or `let-else` are used
  instead of unwrap/expect.

### Crash Frame Sequence

When a panic occurs despite preventions:

1. **catch_unwind fires** (FFI boundary or IO thread top-level).
2. **Log the panic** via tracing::error with backtrace.
3. **Begin ordered shutdown**:
   a. Drain pending KernelEffects.
   b. Send SIGHUP to all child processes.
   c. Wait up to 5 seconds for children to exit.
   d. Close all client connections with error notification.
   e. Remove Unix socket file.
   f. Flush tracing spans to OTLP endpoint.
   g. Exit with code 128 + signal number.

### Ordered Shutdown (Normal)

Normal shutdown (from `kill-server` command or SIGTERM):

1. Kernel emits `KernelEffect::Shutdown`.
2. IO thread stops accepting new connections.
3. IO thread sends detach notifications to all clients.
4. IO thread waits for in-flight writes to complete (1s timeout).
5. IO thread sends SIGHUP to all child processes.
6. IO thread waits for children (WNOHANG loop, 5s timeout).
7. IO thread removes Unix socket file.
8. IO thread joins kernel thread.
9. Tracing shutdown (flush spans).
10. Process exits with code 0.

---

## 17. Flood Fairness Model

### Problem

When a pane floods output (e.g., `yes`, `cat /dev/urandom | xxd`), the
renderer must not spend all its time rendering that one pane while other
panes become unresponsive.

### Solution: Per-Pane Row Quota with Stride Rotation

1. **Budget**: Each render cycle has a total row budget
   (default: `terminal_height * 2`).

2. **Quota**: Each visible pane receives `budget / num_visible_panes` rows.

3. **Stride**: Panes are rendered in rotated order. The first pane to render
   advances by 1 each cycle, preventing positional bias. For 3 panes:
   - Cycle 0: [P1, P2, P3]
   - Cycle 1: [P2, P3, P1]
   - Cycle 2: [P3, P1, P2]

4. **Deferral**: If a pane's dirty rows exceed its quota, excess rows are
   deferred to the next cycle. Deferred rows are tracked per-pane and
   gradually consumed as the pane quiets down.

### Fairness Guarantee

No pane can monopolize more than `1/N` of the render budget per cycle,
where N is the number of visible panes. The stride rotation ensures
that position-based advantages (e.g., always being rendered first) are
eliminated over time.

---

## 18. Testing Strategy

### Test Layers

| Layer | Scope | Count Target | Framework |
|-------|-------|-------------|-----------|
| Unit | Individual functions and types | 150+ | #[test] |
| Property | Invariant verification | 20+ | proptest |
| Integration | Multi-crate interactions | 30+ | mux-test-support |
| Snapshot | Regression detection | 10+ | insta |
| Fuzz | Input robustness | 5+ | cargo-fuzz |

### Current Test Count: 192

All 192 tests pass with `cargo test`.

### Deterministic Testing

- `Clock::manual()` eliminates time-dependent flakiness.
- `TestServer` provides a pre-configured kernel for integration tests.
- `unique_session_name()` generates UUID-based names to prevent collision.

### Property Tests

Key properties verified with proptest:
1. Colour pack/unpack is a perfect roundtrip.
2. Layout dimension sums exactly match parent size.
3. Grid snapshot + mutation does not affect snapshot (COW isolation).
4. VT parser round-trips through all states without hanging.

### Snapshot Tests

insta snapshots capture:
1. Grid text output after VT sequence processing.
2. Rendered escape sequences for known input grids.
3. Configuration parsing results.

---

## 19. SDK API Design

### Builder Pattern

```rust
let server = MuxServer::builder()
    .terminal_mode(TerminalModePolicy::Managed)
    .socket_path("/tmp/termforge.sock")
    .history_limit(50_000)
    .default_shell("/bin/zsh")
    .escape_time(100)
    .focus_events(true)
    .build()?;
```

### Terminal Mode Policy

| Mode | Description | Use Case |
|------|-------------|----------|
| Managed | TermForge calls cfmakeraw/tcsetattr | Standalone server |
| External | Caller manages raw mode | Embedded in terminal emulator |
| SaveRestore | TermForge queries and saves/restores | Unknown embedding context |

### Handle API

```rust
let session = server.new_session("main")?;
let window = session.active_window()?;
let pane = window.active_pane()?;
pane.send_keys("echo hello\n")?;
```

### ORM-like Traversal

```rust
let view = server.server_view();
let main = view.session_by_name("main");
let windows = view.windows().filter(|w| w.session_id == main.id);
```

---

## 20. FFI Layer

### C API Surface

```c
// Grid lifecycle
FfiGrid* grid_create(unsigned sx, unsigned sy, unsigned hlimit);
void grid_destroy(FfiGrid* ptr);
unsigned grid_sx(const FfiGrid* ptr);
unsigned grid_sy(const FfiGrid* ptr);

// Protocol detection
int is_tf01_frame(const uint8_t* data, unsigned len);

// Version
const char* termforge_version(void);
```

### Safety Model

1. All functions check for null pointers.
2. All functions use `catch_unwind` to prevent panic propagation.
3. All functions return null/0 on error instead of panicking.
4. The `FfiGrid` struct is opaque to C code.

### Memory Ownership

- `grid_create` returns a heap-allocated pointer. Caller MUST call
  `grid_destroy` to free it.
- `termforge_version` returns a static string pointer. Caller MUST NOT free it.
- `is_tf01_frame` borrows the data pointer. No ownership transfer.

---

## Appendix A: LOC Estimates

| Crate | Estimated LOC | Phase |
|-------|--------------|-------|
| mux-grapheme-arena | 250 | P0 |
| mux-time | 200 | P0 |
| mux-types | 600 | P0 |
| mux-grid | 1,200 | P0 |
| mux-parser | 2,500 | P1 |
| mux-proto | 1,500 | P1 |
| mux-options | 600 | P1 |
| mux-target | 400 | P0 |
| mux-format | 800 | P1 |
| mux-config | 500 | P1 |
| mux-snapshot | 400 | P0 |
| mux-cmd-parse | 1,200 | P1 |
| mux-orm | 400 | P1 |
| mux-render | 3,000 | P1 |
| mux-pty | 1,000 | P1 |
| mux-kernel | 8,000 | P1-P2 |
| mux-termlet | 600 | P1 |
| mux-api | 800 | P1 |
| mux-test-support | 500 | P1 |
| mux-ffi | 400 | P2 |
| mux-otel | 300 | P2 |
| tmux-builder | 2,000 | P2 |
| tmux-vm | 1,500 | P2 |
| tmux-sniff | 800 | P2 |
| mux-doctor | 500 | P2 |
| Integration tests | 2,000 | P1-P2 |
| **Total** | **~31,450** | |

## Appendix B: Risk Register

| Risk | Impact | Mitigation |
|------|--------|------------|
| Layout engine complexity | High | Port tmux's tested algorithms directly |
| Copy mode edge cases | Medium | Use tmux's test suite as oracle |
| Graphics passthrough security | High | Disabled by default, audit DCS sequences |
| Performance regression | High | Benchmark suite from P1, regression CI |
| signal-hook + tokio interaction | Medium | Pipe-based integration, not raw handlers |
| Wide char reflow correctness | High | Proptest with CJK character generators |
| Custom layout string parsing | Medium | Fuzz with tmux-generated layout strings |
| CRDT convergence bugs | High | Proptest with concurrent mutation generators |
| SCM_RIGHTS fd validation | Medium | fstat + type check on received fds |
| Panic in FFI boundary | Critical | catch_unwind + null return on all FFI fns |
