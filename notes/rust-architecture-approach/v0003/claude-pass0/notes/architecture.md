# TermForge Architecture -- v0003

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
21. [Gap Analysis and Resolutions](#21-gap-analysis-and-resolutions)
22. [Terminal Mode Lifecycle](#22-terminal-mode-lifecycle)
23. [PTY Allocation Strategy](#23-pty-allocation-strategy)
24. [Graphics Passthrough Policy](#24-graphics-passthrough-policy)

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
   terminal. This is terminal-to-terminal rendering (not GPU-based).

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
  level. FFI boundaries use catch_unwind (INV-117).
- **Observable**: Optional OpenTelemetry integration via the mux-otel crate.
- **Deterministic**: The kernel is fully testable with Clock::manual().

### v0003 Scaffold Metrics

- 23 library crates + 4 tool binaries
- 367 passing tests (up from 202 in v0002)
- Multi-file modules for 5 large crates
- All 12 architecture gaps addressed
- Rust 2024 edition, MSRV 1.85

---

## 2. Crate DAG and Layering

The workspace is organized into 7 layers (L0-L6). Each layer may only depend on
layers below it. This prevents circular dependencies and ensures clear
module boundaries.

### Layer 0 -- Leaf Crates (no internal dependencies)

| Crate | Purpose | Tests |
|-------|---------|-------|
| mux-grapheme-arena | Arena allocator for extended grapheme clusters | 11 |
| mux-time | Injectable Clock for deterministic testing | 15 |

### Layer 1 -- Core Types

| Crate | Purpose | Modules | Tests |
|-------|---------|---------|-------|
| mux-types | Cell, CellFlags, Colour, Attrs, entity IDs, Size, errors | cell.rs, colour.rs, attrs.rs, key.rs, id.rs, geometry.rs, error.rs | 54 |

### Layer 2 -- Data Structures

| Crate | Purpose | Modules | Tests |
|-------|---------|---------|-------|
| mux-grid | Chunked COW grid with dirty tracking | line.rs, chunk.rs, region.rs, grid.rs | 43 |
| mux-parser | VT state machine (Paul Williams) | state_machine.rs, csi.rs, input.rs | 34 |
| mux-options | Typed layered option tables | lib.rs | 10 |
| mux-target | tmux target syntax parser | lib.rs | 11 |
| mux-format | Format string #{} evaluator | lib.rs | 11 |
| mux-cmd-parse | Command/config file parser | lib.rs | 10 |
| mux-proto | TF01 wire protocol | lib.rs | 10 |
| mux-crdt | Vector clock + LWW CRDT | lib.rs | 11 |
| mux-fdpass | SCM_RIGHTS fd passing envelope | lib.rs | 6 |

### Layer 3 -- Business Logic

| Crate | Purpose | Modules | Tests |
|-------|---------|---------|-------|
| mux-snapshot | Grid snapshot and text extraction | lib.rs | 6 |
| mux-kernel | Sans-IO kernel: entities, layout, copy mode | session.rs, window.rs, pane.rs, layout.rs, copy_mode.rs, event.rs | 53 |
| mux-render | CompositeGrid, diff, SGR, flood fairness | composite.rs, diff.rs, sgr.rs, flood.rs | 29 |
| mux-orm | ORM-like QuerySet traversal API | lib.rs | 6 |
| mux-pty | PTY allocation, raw mode guard (unsafe) | lib.rs | 5 |

### Layer 4 -- Integration

| Crate | Purpose | Tests |
|-------|---------|-------|
| mux-termlet | Testing pods (PTY + VT + shell) | 5 |
| mux-config | TOML configuration system | 5 |
| mux-api | SDK entry point (MuxServer builder) | 5 |
| mux-test-support | Shared test harness and fixtures | 4 |

### Layer 5 -- External Surface

| Crate | Purpose | Tests |
|-------|---------|-------|
| mux-ffi | C FFI shim with catch_unwind (unsafe) | 6 |
| mux-otel | OpenTelemetry tracing integration | 3 |

### Layer 6 -- Tools

| Tool | Purpose | Tests |
|------|---------|-------|
| tmux-builder | YAML/TOML workspace layout tool | 3 |
| tmux-vm | Headless multiplexer for CI | 3 |
| tmux-sniff | Wire protocol inspector | 3 |
| mux-doctor | Diagnostic environment checker | 4 |

### Dependency Graph (ASCII)

```
L0: mux-grapheme-arena  mux-time
     |                    |
L1:  mux-types -----+----+
     |               |
L2:  mux-grid     mux-parser  mux-options  mux-target  mux-format  mux-cmd-parse  mux-proto  mux-crdt  mux-fdpass
     |               |            |
L3:  mux-snapshot  mux-kernel <--+
     |               |
     mux-render    mux-orm    mux-pty
     |               |          |
L4:  mux-termlet  mux-config  mux-api  mux-test-support
     |               |          |
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
2. Blit each visible pane's grid snapshot into "next" (with flood fairness quota).
3. Draw borders and status line into "next".
4. Compute `diff_grids(prev, next)` -> `Vec<CellUpdate>`.
5. Emit escape sequences via `SgrEncoder` (SGR optimization + CUP optimization).
6. Swap "prev" and "next" (`std::mem::swap`).

### Cell Source Tracking

Each cell in the CompositeGrid knows its source:
- `Pane(PaneId)`: Content from a specific pane.
- `Border`: Pane separator character.
- `StatusLine`: Status bar content.
- `Empty`: Unused area.

This enables click-to-pane mapping and selective invalidation.

### SGR Optimization

The `SgrEncoder` maintains current attribute state (`attrs`, `fg`, `bg`).
SGR sequences are only emitted when the new cell's attributes differ from
the current state. When attributes are removed, a full SGR reset (ESC[0m)
is emitted followed by re-setting the new attributes. This avoids the need
for individual attribute-off sequences which are not universally supported.

### CUP Optimization

Cursor position (CUP) sequences are elided when the next cell is at
`cursor_x + cell_width, cursor_y` -- the terminal auto-advances the
cursor after character output. The `needs_cup()` function determines
whether a CUP is required between consecutive cell updates.

### Wide Character Handling

When a cell has `width > 1` (CJK character), the diff algorithm marks
the following padding cells as invalidated, ensuring they are re-emitted
to prevent visual corruption. The `CellFlags::PADDING` flag (INV-119: 0x04)
marks these padding cells.

### Colour Encoding

Colours are encoded in SGR parameters using the standard ranges:
- Basic 8 colours: params 30-37 (fg) / 40-47 (bg)
- Bright 8 colours: params 90-97 (fg) / 100-107 (bg)
- 256-colour palette: params 38;5;n (fg) / 48;5;n (bg)
- True colour RGB: params 38;2;r;g;b (fg) / 48;2;r;g;b (bg)

---

## 4. Sans-IO Kernel

The kernel is the heart of TermForge. It owns all mutable state and
processes events via a pure reducer function:

```rust
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

The `TestServer` from mux-test-support provides a pre-configured kernel
with a manual clock for integration tests.

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
  |     |     +-- LayoutNode
  |     +-- active_window_idx
  +-- Client (ClientId)
        +-- attached Session
        +-- terminal Size
```

### ID Generation

Entity IDs are monotonically increasing u64 values via `IdGenerator`.
They are never reused, which simplifies garbage collection and prevents
use-after-free patterns. The generator starts at 1 (not 0) so that
0 can serve as a sentinel value.

### Option Inheritance

Options are resolved via layered lookup using `resolve_option()`:
1. Pane-level option table
2. Window-level option table
3. Session-level option table
4. Server-level option table
5. Built-in defaults (`builtin_defaults()`)

---

## 6. Layout Engine

### Binary Tree Model

The layout engine uses a binary tree of `LayoutNode`s. Each node is either:
- `Pane(PaneId)`: A leaf node containing a single pane.
- `LeftRight`: A horizontal split container with children and sizes.
- `TopBottom`: A vertical split container with children and sizes.

This matches tmux's `layout_cell` tree from `layout.c`.

### Built-in Algorithms

| Algorithm | Behavior | Implementation |
|-----------|----------|----------------|
| even-horizontal | Equal-width vertical splits | `even_horizontal()` |
| even-vertical | Equal-height horizontal splits | `even_vertical()` |
| main-horizontal | Main pane on top, rest split below | `main_horizontal()` |
| main-vertical | Main pane on left, rest split right | `main_vertical()` |
| tiled | Grid arrangement (sqrt-based) | `tiled()` |

### Dimension Redistribution

When the terminal is resized, the layout engine redistributes space:
1. Compute base size: `parent_dimension / num_children`.
2. Compute remainder: `parent_dimension % num_children`.
3. Distribute remainder pixels left-to-right (first children get +1).
4. The sum of all child dimensions exactly equals the parent size.
5. This is verified by tests: `sum_child_sizes(&sizes) == parent_dimension`.

### Minimum Pane Size

No pane can be smaller than 2 cols x 1 row (`Size::MIN_PANE`). The
`meets_minimum()` and `clamp_to_minimum()` methods enforce this.

### Custom Layout Strings

tmux supports custom layout strings with a checksum format:
```
89eb,120x40,0,0{60x40,0,0,0,59x40,61,0,1}
```

The `validate_layout_checksum()` function validates the 4-hex-digit
checksum before applying the layout. The checksum algorithm is a
rotating 16-bit sum matching tmux's `layout_checksum()` in `layout.c`.

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

### Multi-File Organization

- `state_machine.rs`: State enum, `transition()` function, `TransitionAction` enum.
- `csi.rs`: `CsiParams` with parameter accumulation, private markers, intermediates.
- `input.rs`: `VtParser` entry point, `VtAction` enum, UTF-8 handling.

### Action Types

| Action | Description |
|--------|-------------|
| Print(char) | Write character at cursor |
| Execute(u8) | Process C0 control (BEL, BS, CR, LF, etc.) |
| CsiDispatch(CsiParams) | CSI sequence complete (SGR, CUP, ED, EL, etc.) |
| EscDispatch | ESC sequence complete (DECSC, DECRC, etc.) |
| OscDispatch(Vec<u8>) | OSC string complete (title, clipboard, etc.) |
| DcsHook(CsiParams) | DCS sequence started |
| DcsPut(u8) | DCS data byte |
| DcsUnhook | DCS sequence complete |

### UTF-8 Support

The parser handles multi-byte UTF-8 sequences in the Ground state by
accumulating bytes until a complete codepoint is formed. Invalid
continuation bytes cause the accumulator to reset and the byte to be
processed as a new lead byte.

### CSI Parameter Limits

CSI sequences support up to 16 parameters (`MAX_CSI_PARAMS`). The
`CsiParams` struct provides `get(index, default)` for ergonomic access
with default values. Parameter values saturate at `u16::MAX` to prevent
overflow from malicious input.

### Why Not crossterm/vt100/termwiz

The VT parser is custom-built for tmux behavioral compatibility:
- tmux-specific escape sequence handling and quirks
- Precise state machine behavior matching tmux's `input.c`
- No unnecessary abstractions that hide platform details
- Direct control over DCS passthrough (for graphics policy enforcement)

---

## 8. Grid Architecture

### Multi-File Organization

- `line.rs`: Arc-COW line with cells and dirty flag.
- `chunk.rs`: Chunk-based storage (64 lines per chunk constant).
- `region.rs`: Scroll region management (DECSTBM).
- `grid.rs`: The `ChunkedGrid` entry point with full API.

### Arc-based COW

Each `Line` stores its cells as `Arc<Vec<Cell>>`. Benefits:
- **Snapshot isolation**: Creating a grid snapshot for the renderer only
  increments reference counts on the Arc pointers.
- **Lazy copy**: Lines are only physically copied when mutated via
  `Arc::make_mut`, which clones the inner Vec only if `strong_count > 1`.
- **Verifiable**: `Line::ref_count()` exposes the Arc strong count for testing.

### Dirty Tracking

Each line has a `dirty: bool` flag. The diff algorithm only processes
dirty lines, reducing render work for static content. `clear_all_dirty()`
is called after each render cycle. `dirty_rows()` returns the indices
of modified rows.

### Scrollback Management

- Active screen: the bottom `sy` lines.
- Scrollback: everything above the active screen, bounded by `hlimit`.
- When `total_lines > sy + hlimit`, excess lines are trimmed from the front.
- Full-screen scroll up moves the top line to scrollback naturally.
- Sub-region scroll (DECSTBM) removes/inserts within the scroll region only.

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

### Cursor Movement

Cursor movement in copy mode is decoupled from the grid cursor:
- `cursor_up(n)`: Moves up, scrolling into scrollback if needed.
- `cursor_down(n, height)`: Moves down, reducing scroll offset if at bottom.
- `cursor_left()`: Saturating subtraction.
- `cursor_right(width)`: Bounded by screen width.

### Buffer Ring

Copied text is pushed to a bounded ring buffer (`BufferRing`).
When the ring reaches `max_entries` (default 50), the oldest entry
is evicted. Access by index (0 = most recent) allows `paste-buffer`
to retrieve any entry.

### Rectangle Selection

Block selection uses `extract_rect()` from `GridSnapshot` to capture
a rectangular region of text. This is independent of line wrapping.

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
Corrupted frames are rejected with `ProtoError::CrcMismatch`.

### Protocol Auto-Detection

The `is_tf01_frame()` function checks the first 4 bytes:
- If `0x54464F31` (TFO1): use TF01 protocol.
- Otherwise: fall back to tmux imsg format.

This is also exposed via the FFI layer for C consumers.

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
allow_passthrough = false   # INV-220
default_terminal = "tmux-256color"
mouse = true

[session]
base_index = 1
renumber_windows = true
status = true
status_position = "top"
```

### .tmux.conf Compatibility

The `mux-cmd-parse` crate parses `.tmux.conf` files into command sequences:
- `#` comment lines
- Quoted strings (single and double)
- Backslash continuation
- Semicolon command chaining
- Backslash escaping within double quotes

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

All channels are bounded (crossbeam-channel):
- control: 512
- data: 1024 frames x 64 KiB
- render: 256
- effect: 1024

Backpressure: IO thread blocks on send if kernel falls behind.

---

## 13. Signal Handling

### SIGWINCH (Terminal Resize) -- Gap-3

The exact signal -> query -> propagate -> render sequence:

1. `signal-hook` registers a pipe-based SIGWINCH handler (Gap-5).
2. IO thread reads from signal pipe via tokio AsyncFd.
3. IO thread queries terminal size via `ioctl(TIOCGWINSZ)`.
4. IO thread sends `KernelEvent::ClientResize { client_id, size }`.
5. Kernel updates client size, recalculates layouts for the attached session.
6. Kernel emits `KernelEffect::ResizePty` for each affected pane.
7. Kernel emits `KernelEffect::Render`.
8. IO thread executes `ResizePty` effects via `ioctl(TIOCSWINSZ)`.
9. Render thread produces new frame with updated dimensions.

### SIGCHLD (Child Process Exit) -- Gap-8

1. `signal-hook` registers pipe-based SIGCHLD handler.
2. IO thread reads from signal pipe.
3. IO thread drains `waitpid(-1, WNOHANG)` **in a loop** until no more children.
   This is critical because multiple children may exit between signal deliveries.
4. For each reaped child, sends `KernelEvent::PaneExited { pane_id, status }`.
5. Kernel may close the pane, respawn, or keep as zombie.

### SIGTERM / SIGINT (Graceful Shutdown)

1. Signal handler sets an atomic flag (via signal-hook).
2. IO thread detects flag on next poll cycle.
3. IO thread sends `KernelEvent::Command { command: "kill-server" }`.
4. Kernel processes shutdown, emitting `KernelEffect::Shutdown`.
5. IO thread executes ordered shutdown sequence.

### Job Control Passthrough -- Gap-9

Signal non-interference invariant: TermForge does NOT intercept or forward
SIGTSTP, SIGCONT, SIGTTIN, SIGTTOU to child processes. Child process groups
manage their own job control independently. The terminal multiplexer only
passes data through the PTY, not signals.

### Why signal-hook, Not tokio Signals -- Gap-5

signal-hook is used instead of tokio's signal module because:
- Pipe-based signal delivery is runtime-agnostic (works with std + tokio).
- The pipe fd can be registered with `tokio::io::AsyncFd` for async notification.
- This aligns with the Sans-IO kernel model where signals become events.
- signal-hook handles the async-signal-safety constraints correctly.

---

## 14. CRDT Strategy

### Motivation

Collaborative terminal sessions (multiple users editing the same pane
simultaneously) require conflict resolution. TermForge uses a CRDT
approach for eventual convergence.

### Vector Clock Dominance

Each server in a multi-server federation maintains a vector clock.
Events are ordered by vector clock dominance:
- Event A dominates Event B if A's vector clock is >= B's in all dimensions
  and strictly > in at least one.
- If neither dominates, the events are concurrent.
- Equal clocks indicate the same logical time (not concurrent).

### Last-Writer-Wins (LWW) Register

For individual cell conflicts (two users write to the same grid position):
- Each cell write carries a timestamp and origin replica ID.
- When concurrent writes are detected, the one with the highest timestamp
  wins. Ties are broken by replica ID (total order).

### Convergence Guarantee

All replicas that see the same set of events will converge to the same
state, regardless of the order in which events are received. This is
verified by the `lww_convergence` test which merges writes in both orders.

---

## 15. SCM_RIGHTS fd Passing -- Gap-10

### Architecture

For tmux compatibility mode, the server and client may need to pass
file descriptors (PTY peers, stdin/stdout) between processes via Unix
domain socket ancillary data.

### Envelope Model

The `FdEnvelope` in mux-fdpass provides:
- Maximum 16 file descriptors per message (`FD_LIMIT`).
- Validation: negative fd values are rejected.
- Optional data payload alongside the fds.

### Fd Type Validation

After receiving fds via SCM_RIGHTS, the `FdType` enum classifies them:
- `PtyMaster`: PTY master fd
- `PtySlave`: PTY slave/peer fd
- `RegularFile`: Regular file
- `Socket`: Unix domain socket
- `Unknown`: Unrecognized type

### TF01 Mode

SCM_RIGHTS is NOT used in TF01 mode. The TF01 protocol handles all
data passing through wire protocol frames.

---

## 16. Panic Crash Frame and Ordered Shutdown

### Panic Recovery (INV-117)

The FFI boundary (`mux-ffi`) uses `std::panic::catch_unwind` to prevent
Rust panics from unwinding through C code (which is undefined behavior).

All FFI functions:
1. Use `#[unsafe(no_mangle)]` (Rust 2024 syntax).
2. Wrap their body in `catch_unwind`.
3. Return null/0 on panic.
4. Check for null pointers before dereferencing.

### Workspace-Level Panic Prevention

- `profile.*.panic = "unwind"`: Required for catch_unwind in all profiles.
- Clippy denies `unwrap_used`, `expect_used`, `panic` in all crates.
- `unwrap_or`, `unwrap_or_else`, `unwrap_or_default`, or `let-else` used instead.

### Ordered Shutdown Sequence

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

1. **Budget**: Each render cycle has a total row budget (configurable).

2. **Quota**: Each visible pane receives `budget / num_visible_panes` rows.
   Remainder rows are distributed to the first panes.

3. **Stride**: Panes are rendered in rotated order. The `stride_offset`
   advances by 1 each cycle, preventing positional bias. For 3 panes:
   - Cycle 0: [P1, P2, P3]
   - Cycle 1: [P2, P3, P1]
   - Cycle 2: [P3, P1, P2]

4. **Deferral**: If a pane's dirty rows exceed its quota, excess rows are
   deferred via `defer_rows()`. Deferred rows are tracked per-pane and
   gradually consumed as the pane quiets down.

### Fairness Guarantee

No pane can monopolize more than `1/N` of the render budget per cycle,
where N is the number of visible panes. The stride rotation ensures
that position-based advantages are eliminated over time.

---

## 18. Testing Strategy

### Test Layers

| Layer | Scope | Count | Framework |
|-------|-------|-------|-----------|
| Unit | Individual functions and types | 300+ | #[test] |
| Property | Invariant verification | 10+ | proptest (dev-dep) |
| Integration | Multi-crate interactions | 20+ | mux-test-support |
| Golden | Regression detection (planned) | -- | insta (available) |

### Current Test Count: 367

All 367 tests pass with `cargo test --workspace`.

### Test Distribution by Crate

| Crate | Tests |
|-------|-------|
| mux-types | 54 |
| mux-kernel | 53 |
| mux-grid | 43 |
| mux-parser | 34 |
| mux-render | 29 |
| mux-time | 15 |
| mux-grapheme-arena | 11 |
| mux-target | 11 |
| mux-format | 11 |
| mux-crdt | 11 |
| mux-options | 10 |
| mux-cmd-parse | 10 |
| mux-proto | 10 |
| Other crates | 65 |

### Deterministic Testing

- `Clock::manual()` eliminates time-dependent flakiness.
- `TestServer` provides a pre-configured kernel for integration tests.
- `unique_session_name()` generates UUID-based names to prevent collision.

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
    .allow_passthrough(false)   // INV-220: default
    .clock(Clock::manual())     // For testing
    .build()?;
```

### Terminal Mode Policy (Gap-2)

| Mode | Description | Use Case |
|------|-------------|----------|
| Managed | TermForge calls cfmakeraw/tcsetattr via RawModeGuard | Standalone server |
| External | Caller manages raw mode | Embedded in terminal emulator |
| SaveRestore | TermForge queries and saves/restores | Unknown embedding context |

### Handle API

```rust
let session_id = server.new_session("main")?;
```

### ORM-like Traversal

```rust
let view = ServerView { sessions: ... };
let main = view.session_by_name("main");
let all_panes = view.all_panes();
```

---

## 20. FFI Layer

### C API Surface

```c
FfiGrid* grid_create(unsigned sx, unsigned sy, unsigned hlimit);
void grid_destroy(FfiGrid* ptr);
unsigned grid_sx(const FfiGrid* ptr);
unsigned grid_sy(const FfiGrid* ptr);
int is_tf01_frame(const uint8_t* data, unsigned len);
const char* termforge_version(void);
```

### Safety Model

1. All functions check for null pointers.
2. All functions use `catch_unwind` to prevent panic propagation (INV-117).
3. All functions return null/0 on error instead of panicking.
4. The `FfiGrid` struct is opaque to C code.
5. `#[unsafe(no_mangle)]` used for Rust 2024 compliance.

---

## 21. Gap Analysis and Resolutions

### Gap-1: TIOCGPTPEER

**Resolution**: Documented in mux-pty. `PtyAllocStrategy` enum defines
`Traditional` (openpty) and `Tiocgptpeer` (Linux 4.13+) variants.
Runtime detection selects the appropriate strategy.

### Gap-2: Raw Mode Lifecycle

**Resolution**: `RawModeGuard` RAII type in mux-pty with `TerminalModePolicy`
enum (Managed, External, SaveRestore). Guard restores terminal on drop.

### Gap-3: SIGWINCH Propagation Flow

**Resolution**: Documented in Section 13 with the exact 9-step sequence:
signal -> pipe -> query TIOCGWINSZ -> KernelEvent -> layout recalc ->
ResizePty effects -> Render effect -> render thread produces frame.

### Gap-4: crossterm Rejection Rationale

**Resolution**: Documented in mux-pty crate docs. Reasons: platform detail
abstraction, event model mismatch, direct ioctl requirement, custom parser.

### Gap-5: signal-hook vs tokio signals

**Resolution**: Documented in mux-pty. signal-hook chosen for:
runtime-agnostic pipe-based delivery, AsyncFd integration, Sans-IO alignment.

### Gap-6: Composition/Diff Rendering

**Resolution**: Full terminal-to-terminal render pipeline in mux-render with
4 modules: composite.rs (double-buffer), diff.rs (cell diff + wide-char),
sgr.rs (attribute encoding), flood.rs (fairness).

### Gap-7: Layout Engine

**Resolution**: Implemented in mux-kernel/layout.rs with 5 built-in algorithms
(even-horizontal, even-vertical, main-horizontal, main-vertical, tiled) plus
custom layout string parsing with checksum validation.

### Gap-8: Zombie Reaping

**Resolution**: SIGCHLD handler drains `waitpid(-1, WNOHANG)` in a loop
as documented in Section 13. The loop continues until no more children.

### Gap-9: Job Control Passthrough

**Resolution**: Signal non-interference invariant documented. TermForge
does not intercept SIGTSTP/SIGCONT/SIGTTIN/SIGTTOU for child processes.

### Gap-10: SCM_RIGHTS fd Passing

**Resolution**: mux-fdpass provides `FdEnvelope` with 16-fd limit,
validation, and `FdType` classification. Actual sendmsg/recvmsg in mux-pty.

### Gap-11: SDK Ergonomics

**Resolution**: mux-api provides `MuxServer::builder()` pattern with
fluent configuration and `Clock::manual()` injection for testing.

### Gap-12: Graphics Passthrough Stance

**Resolution**: INV-220 enforced. `allow_passthrough` defaults to `false`.
Sixel/iTerm2/Kitty acknowledged as future considerations in mux-api docs.
DCS sequences consumed and discarded when passthrough is disabled.

---

## 22. Terminal Mode Lifecycle (Gap-2 Deep Dive)

### RawModeGuard RAII

The `RawModeGuard` ensures terminal state is always restored:

```rust
{
    let guard = RawModeGuard::new(TerminalModePolicy::Managed)?;
    // Terminal is now in raw mode
    run_server()?;
    // guard.drop() called here -> restores terminal
}
```

Even if `run_server()` panics (caught by catch_unwind), the guard's
`Drop` implementation restores the original terminal state.

### Policy Selection

| Scenario | Recommended Policy |
|----------|--------------------|
| Standalone tmux replacement | Managed |
| Embedded in Alacritty/WezTerm | External |
| IDE integration (unknown context) | SaveRestore |
| CI/headless (no terminal) | External |

---

## 23. PTY Allocation Strategy (Gap-1 Deep Dive)

### Traditional Path

```
posix_openpt(O_RDWR | O_NOCTTY) -> master_fd
grantpt(master_fd)
unlockpt(master_fd)
ptsname(master_fd) -> slave_path    // TOCTOU race possible
open(slave_path) -> slave_fd
```

### TIOCGPTPEER Path (Linux 4.13+)

```
posix_openpt(O_RDWR | O_NOCTTY) -> master_fd
grantpt(master_fd)
unlockpt(master_fd)
ioctl(master_fd, TIOCGPTPEER, O_RDWR | O_NOCTTY) -> slave_fd  // race-free
```

### Runtime Detection

The PTY allocator detects kernel support at startup:
1. Try `TIOCGPTPEER` on a test PTY.
2. If `ENOTTY` or `EINVAL`, fall back to traditional.
3. Cache the strategy for all subsequent allocations.

---

## 24. Graphics Passthrough Policy (Gap-12 Deep Dive)

### Supported Protocols (Future)

| Protocol | Escape Prefix | Status |
|----------|--------------|--------|
| Sixel | DCS q | Future |
| iTerm2 | OSC 1337 | Future |
| Kitty | APC G | Future |

### Default Behavior (INV-220)

When `allow-passthrough` is `false` (default):
- DCS sequences matching graphics protocols are consumed and discarded.
- OSC sequences with unknown numbers are discarded.
- APC sequences are always discarded (SosPmApcString state).

### Opt-in

Users must explicitly enable passthrough:
```toml
[server]
allow_passthrough = true
```
Or via the builder:
```rust
MuxServer::builder().allow_passthrough(true)
```

### Security Rationale

Graphics passthrough can be used to inject arbitrary terminal sequences
into the parent terminal, potentially escaping the multiplexer sandbox.
Disabled by default per tmux's own security stance.

---

## Appendix A: LOC Estimates

| Crate | Scaffold LOC | Full Target LOC | Phase |
|-------|-------------|----------------|-------|
| mux-grapheme-arena | 170 | 250 | P0 |
| mux-time | 200 | 300 | P0 |
| mux-types | 500 | 800 | P0 |
| mux-grid | 400 | 1,500 | P0 |
| mux-parser | 500 | 3,000 | P1 |
| mux-proto | 200 | 1,500 | P1 |
| mux-options | 150 | 600 | P1 |
| mux-target | 200 | 500 | P0 |
| mux-format | 200 | 800 | P1 |
| mux-config | 150 | 500 | P1 |
| mux-snapshot | 120 | 400 | P0 |
| mux-cmd-parse | 200 | 1,200 | P1 |
| mux-orm | 100 | 400 | P1 |
| mux-render | 350 | 3,000 | P1 |
| mux-pty | 120 | 1,000 | P1 |
| mux-kernel | 600 | 8,000 | P1-P2 |
| mux-termlet | 80 | 600 | P1 |
| mux-api | 150 | 800 | P1 |
| mux-test-support | 60 | 500 | P1 |
| mux-ffi | 120 | 400 | P2 |
| mux-otel | 50 | 300 | P2 |
| mux-crdt | 150 | 800 | P2 |
| mux-fdpass | 80 | 400 | P2 |
| tmux-builder | 50 | 2,000 | P2 |
| tmux-vm | 50 | 1,500 | P2 |
| tmux-sniff | 50 | 800 | P2 |
| mux-doctor | 70 | 500 | P2 |
| **Total** | **~4,570** | **~31,550** | |

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
| Rust 2024 reserved keywords | Low | Avoid `gen` identifier; use `id_gen` |
| TIOCGPTPEER availability | Low | Runtime detection with traditional fallback |
