# TermForge Architecture -- v0003

## Table of Contents

1. [Project Identity](#1-project-identity)
2. [Core Architecture (3-Thread Model)](#2-core-architecture-3-thread-model)
3. [Crate Dependency Layers (L0-L6)](#3-crate-dependency-layers-l0-l6)
4. [Grid Design](#4-grid-design)
5. [VT Parser Architecture](#5-vt-parser-architecture)
6. [Render Pipeline](#6-render-pipeline)
7. [Flood Fairness](#7-flood-fairness)
8. [Channel Architecture](#8-channel-architecture)
9. [Wire Protocol](#9-wire-protocol)
10. [Session/Window/Pane Object Model](#10-sessionwindowpane-object-model)
11. [Layout Engine](#11-layout-engine)
12. [Copy Mode](#12-copy-mode)
13. [Options System](#13-options-system)
14. [Format String Expansion](#14-format-string-expansion)
15. [Command Parse Pipeline](#15-command-parse-pipeline)
16. [ORM Layer (mux-orm)](#16-orm-layer-mux-orm)
17. [Termlet API](#17-termlet-api)
18. [Test Support](#18-test-support)
19. [OpenTelemetry Integration](#19-opentelemetry-integration)
20. [FFI Surface](#20-ffi-surface)
21. [Gap Analysis and Resolutions](#21-gap-analysis-and-resolutions)
22. [Terminal Mode Lifecycle](#22-terminal-mode-lifecycle)
23. [PTY Allocation Strategy](#23-pty-allocation-strategy)
24. [Graphics Passthrough Policy](#24-graphics-passthrough-policy)
25. [SDK Ergonomics (mux-api)](#25-sdk-ergonomics-mux-api)
26. [Tool Crates](#26-tool-crates)

---

## 1. Project Identity

TermForge is a Rust-native terminal multiplexer SDK targeting tmux v3.5a
protocol behavior. It is designed for three audiences:

1. **Application Embedders** who want to add terminal multiplexing
   capabilities to their Rust applications (terminal emulators, IDEs,
   CI orchestration tools, devtools).

2. **Library Authors** who want to build test harnesses for terminal
   programs using deterministic, in-process "termlet" pods.

3. **tmux Users** who want a modern, embeddable replacement with the
   same command set and wire protocol compatibility.

### Core Principles

- **Sans-IO Kernel**: All mutable multiplexer state lives in a
  single-threaded kernel that processes events and emits effects.
  The kernel never performs I/O directly.

- **Double-Buffer Diff Rendering**: The renderer maintains two
  CompositeGrids (prev/next). Each render cycle writes to "next",
  diffs against "prev", and emits only the minimal escape sequences
  needed to update the terminal.

- **Arc-COW Grid Lines**: Terminal grid lines use `Arc<Vec<Cell>>`
  for copy-on-write semantics. Snapshots for the render pipeline
  are created by cloning Arc pointers, not copying cell data.

### Technical Identity

| Property | Value |
|----------|-------|
| Edition | Rust 2024 |
| MSRV | 1.85 |
| License | MIT OR Apache-2.0 |
| Crate prefix | `mux-` |
| Safety | `#![forbid(unsafe_code)]` except mux-pty, mux-ffi |
| Panic policy | clippy denies `unwrap_used`, `expect_used`, `panic` |
| Error handling | `thiserror` for libraries, `anyhow` for binaries |
| Testing | 500+ tests, proptest for invariant verification |
| Target | tmux v3.5a protocol compatibility |

### v0003 Scaffold Metrics

- 23 library crates + 4 tool binaries
- 500+ passing tests
- Multi-file modules for 5 large crates
- All 12 architecture gaps addressed
- Rust 2024 edition with MSRV 1.85
- Comprehensive clippy lint configuration

### Reserved Keywords in Rust 2024

The identifier `gen` is reserved in Rust 2024. All code uses `id_gen`
instead. The `#[unsafe(no_mangle)]` syntax is required for FFI exports,
replacing the older `#[no_mangle]` attribute.

---

## 2. Core Architecture (3-Thread Model)

TermForge uses exactly three threads for its runtime:

### Kernel Thread (std::thread)

The kernel thread owns the single `Kernel` instance and runs a tight
event-processing loop:

```
loop {
    let event = event_rx.recv()?;
    let effects = kernel.process_event(event);
    for effect in effects {
        effect_tx.send(effect)?;
    }
}
```

The kernel thread:
- Is a plain `std::thread`, NOT a tokio task.
- Owns ALL mutable multiplexer state.
- Never performs I/O, file access, or network operations.
- Has no dependency on any async runtime.
- Is fully deterministic when using `Clock::manual()`.

### IO Thread (tokio)

The IO thread runs the tokio runtime and handles all external I/O:

- **Unix socket accept loop**: Listens for client connections.
- **PTY I/O**: Reads output from child processes, writes input to them.
- **Signal bridge**: Converts Unix signals to KernelEvents via pipe.
- **Client connections**: Reads commands, writes rendered output.
- **Effect execution**: Translates KernelEffects into real I/O operations.

### Render Thread (tokio task)

The render thread runs as a tokio task within the IO thread's runtime:

- Receives GridSnapshots from the kernel (via Arc-COW, zero-copy).
- Builds the CompositeGrid by blitting visible panes.
- Runs the diff algorithm against the previous frame.
- Emits minimal escape sequences via SgrEncoder.
- Writes rendered bytes to client connections.

### Thread Communication Diagram

```
+-------------------------+      +--------------------------+
| accept loop             | ---> | control lane (bound 512) |
+-------------------------+      +--------------------------+
            |                                  |
            v                                  v
+-------------------------+      +--------------------------+
| signal bridge loop      | ---> | effect lane (bound 1024) |
+-------------------------+      +--------------------------+
            |                                  |
            v                                  v
+-------------------------+      +--------------------------+
| PTY I/O loop            | ---> | data lane (1024 x 64KiB) |
+-------------------------+      +--------------------------+
                                               |
                                               v
                                   +--------------------------+
                                   | render lane (bound 256)  |
                                   +--------------------------+
```

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

### Why Not a Single Tokio Runtime?

The kernel is a pure state machine with no I/O. Running it on a
dedicated OS thread (not a tokio task) ensures:

- It is never preempted by other async tasks.
- It has predictable, low-jitter latency.
- It cannot accidentally perform I/O.
- It can be tested without any async runtime.

All communication is via crossbeam bounded channels.

---

## 3. Crate Dependency Layers (L0-L6)

The workspace is organized into 7 layers (L0-L6). Each layer may only
depend on layers below it, preventing circular dependencies.

### Layer 0 -- Leaf Crates (no internal dependencies)

| Crate | Purpose | Tests |
|-------|---------|-------|
| mux-grapheme-arena | Arena allocator for extended grapheme clusters | 12 |
| mux-time | Injectable Clock for deterministic testing | 17 |

These crates have zero internal dependencies and form the foundation.

### Layer 1 -- Core Types

| Crate | Purpose | Modules |
|-------|---------|---------|
| mux-types | Cell, CellFlags, Colour, Attrs, entity IDs, Size, errors | cell.rs, colour.rs, attrs.rs, key.rs, id.rs, geometry.rs, error.rs |

mux-types depends only on mux-grapheme-arena and mux-time.

### Layer 2 -- Data Structures

| Crate | Purpose | Modules |
|-------|---------|---------|
| mux-grid | Chunked COW grid with dirty tracking | line.rs, chunk.rs, region.rs, grid.rs |
| mux-parser | VT state machine (Paul Williams) | state_machine.rs, csi.rs, input.rs |
| mux-options | Typed layered option tables | lib.rs |
| mux-target | tmux target syntax parser | lib.rs |
| mux-format | Format string #{} evaluator | lib.rs |
| mux-cmd-parse | Command/config file parser | lib.rs |
| mux-proto | TF01 wire protocol | lib.rs |
| mux-crdt | Vector clock + LWW CRDT | lib.rs |
| mux-fdpass | SCM_RIGHTS fd passing envelope | lib.rs |

L2 crates depend on L0 and L1. They are pure data structures with
no business logic.

### Layer 3 -- Business Logic

| Crate | Purpose | Modules |
|-------|---------|---------|
| mux-snapshot | Grid snapshot and text extraction | lib.rs |
| mux-kernel | Sans-IO kernel: entities, layout, copy mode | session.rs, window.rs, pane.rs, layout.rs, copy_mode.rs, event.rs |
| mux-render | CompositeGrid, diff, SGR, flood fairness | composite.rs, diff.rs, sgr.rs, flood.rs |
| mux-orm | ORM-like QuerySet traversal API | lib.rs |
| mux-pty | PTY allocation, raw mode guard (unsafe) | lib.rs |

L3 crates implement the core multiplexer logic. mux-pty is the first
crate allowed to use unsafe code.

### Layer 4 -- Integration

| Crate | Purpose |
|-------|---------|
| mux-termlet | Testing pods (PTY + VT + shell) |
| mux-config | TOML configuration system |
| mux-api | SDK entry point (MuxServer builder) |
| mux-test-support | Shared test harness and fixtures |

L4 crates wire multiple L2/L3 crates together.

### Layer 5 -- External Surface

| Crate | Purpose |
|-------|---------|
| mux-ffi | C FFI shim with catch_unwind (unsafe) |
| mux-otel | OpenTelemetry tracing integration |

### Layer 6 -- Tools

| Tool | Purpose |
|------|---------|
| tmux-builder | YAML/TOML workspace layout tool |
| tmux-vm | Headless multiplexer for CI |
| tmux-sniff | Wire protocol inspector |
| mux-doctor | Diagnostic environment checker |

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

### Cross-Layer Dependency Rule

A crate at layer N may depend on any crate at layers 0..N-1. It MUST
NOT depend on a crate at layer N or higher. This rule is enforced by
the Architect Agent and validated during code review.

The only exception is dev-dependencies (used for tests), which may
reference crates from higher layers. For example, mux-types may use
mux-test-support as a dev-dependency.

---

## 4. Grid Design

### Arc-COW Line Architecture

Each `Line` stores its cells as `Arc<Vec<Cell>>`. This design enables:

- **Snapshot isolation**: Creating a grid snapshot for the renderer only
  increments reference counts on the Arc pointers.
- **Lazy copy**: Lines are only physically copied when mutated via
  `Arc::make_mut`, which clones the inner Vec only if `strong_count > 1`.
- **Verifiable**: `Line::ref_count()` exposes the Arc strong count for
  testing.
- **Deterministic**: Property tests verify that mutations on a COW clone
  do not affect the original line.

### CellFlags (INV-119)

Cell flags are bit-compatible with tmux's `GRID_FLAG_*` constants from
`tmux.h:742-749`:

```
PADDING  = 0x04  (GRID_FLAG_PADDING)
EXTENDED = 0x08  (GRID_FLAG_EXTENDED)
SELECTED = 0x10  (GRID_FLAG_SELECTED)
CLEARED  = 0x40  (GRID_FLAG_CLEARED)
TAB      = 0x80  (GRID_FLAG_TAB)
```

These values are verified by tests and must not be changed without
cross-referencing the tmux C source at `/home/d/study/c/tmux/tmux.h`.

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

### Chunked Storage

Lines are stored in chunks of 64 (`CHUNK_SIZE`). Benefits:

- Reduced allocation overhead compared to per-line allocation.
- Cache-friendly sequential access within a chunk.
- Scrollback trimming removes entire chunks at once.
- The chunk index calculation is simple: `line_index / CHUNK_SIZE`.

### Dirty Tracking

Each line has a `dirty: bool` flag. The diff algorithm only processes
dirty lines, reducing render work for static content. `clear_all_dirty()`
is called after each render cycle. `dirty_rows()` returns the indices
of modified rows.

### Damage Tracking for Rendering

When the grid is mutated (character write, scroll, erase), the affected
lines are marked dirty. The render pipeline only processes dirty regions:

1. `write_char()` marks the current line dirty.
2. `line_feed()` marks both the source and destination lines dirty.
3. `scroll_up()`/`scroll_down()` marks the entire scroll region dirty.
4. `erase_line()`/`erase_display()` marks affected lines dirty.

### Scrollback Management

- Active screen: the bottom `sy` lines.
- Scrollback: everything above the active screen, bounded by `hlimit`.
- When `total_lines > sy + hlimit`, excess lines are trimmed from front.
- Full-screen scroll up moves the top line to scrollback naturally.
- Sub-region scroll (DECSTBM) removes/inserts within the scroll region.

### Wide Character Handling

When a cell has `width > 1` (CJK character), the cell at position x+1
is marked with `CellFlags::PADDING` (0x04). During rendering, padding
cells are always re-emitted with their parent wide character to prevent
visual corruption.

---

## 5. VT Parser Architecture

### Paul Williams State Machine

The parser implements the 14-state Paul Williams VT state machine,
matching the design described in "A parser for DEC's ANSI-compatible
video terminals":

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

- `state_machine.rs`: State enum, `transition()` function,
  `TransitionAction` enum with all valid state-to-state transitions.
- `csi.rs`: `CsiParams` with parameter accumulation (up to 16 params),
  private markers, and intermediate character tracking.
- `input.rs`: `VtParser` entry point, `VtAction` output enum, and
  UTF-8 multi-byte sequence handling.

### Action Types

| Action | Description |
|--------|-------------|
| Print(char) | Write character at cursor position |
| Execute(u8) | Process C0 control (BEL, BS, CR, LF, etc.) |
| CsiDispatch(CsiParams) | CSI sequence complete |
| EscDispatch | ESC sequence complete |
| OscDispatch(Vec<u8>) | OSC string complete |
| DcsHook(CsiParams) | DCS sequence started |
| DcsPut(u8) | DCS data byte |
| DcsUnhook | DCS sequence complete |

### UTF-8 Support

The parser handles multi-byte UTF-8 sequences in the Ground state:

1. Lead bytes (0xC0-0xF7) start a new multi-byte sequence.
2. Continuation bytes (0x80-0xBF) are accumulated.
3. When the sequence is complete, the codepoint is emitted as Print.
4. Invalid continuation bytes reset the accumulator and re-process.

### CSI Parameter Limits

CSI sequences support up to 16 parameters (`MAX_CSI_PARAMS`). The
`CsiParams` struct provides `get(index, default)` for ergonomic access
with default values. Parameter values saturate at `u16::MAX` to prevent
overflow from malicious input.

### Why Not crossterm/vt100/termwiz

The VT parser is custom-built for tmux behavioral compatibility:

- tmux-specific escape sequence handling and quirks.
- Precise state machine behavior matching tmux's `input.c`.
- No unnecessary abstractions that hide platform details.
- Direct control over DCS passthrough for graphics policy enforcement.

### Fuzz Testing

The parser is tested with proptest using arbitrary byte sequences.
The invariant is: the parser must never panic, regardless of input.
Any byte sequence is valid input (it may produce no actions, but
must not crash).

---

## 6. Render Pipeline

### Double-Buffer Architecture

The renderer maintains two `CompositeGrid` instances:

1. **prev**: The state currently displayed on the terminal.
2. **next**: The state being assembled for the next frame.

Each render cycle:
1. Clear "next".
2. Blit each visible pane's grid snapshot into "next" (respecting
   flood fairness quota).
3. Draw borders and status line into "next".
4. Compute `diff_grids(prev, next)` -> `Vec<CellUpdate>`.
5. Emit escape sequences via `SgrEncoder` (SGR + CUP optimization).
6. Swap "prev" and "next" (`std::mem::swap`).

### Cell Source Tracking

Each cell in the CompositeGrid tracks its source:

- `Pane(PaneId)`: Content from a specific pane.
- `Border`: Pane separator character.
- `StatusLine`: Status bar content.
- `Empty`: Unused area.

This enables click-to-pane mapping and selective invalidation.

### SGR Optimization

The `SgrEncoder` maintains current attribute state (attrs, fg, bg).
SGR sequences are only emitted when the new cell's attributes differ
from the current state:

- When attributes are only added, individual SGR codes are emitted.
- When attributes are removed, a full SGR reset (ESC[0m) is emitted
  followed by re-setting the new attributes.
- This avoids individual attribute-off sequences which are not
  universally supported across terminal emulators.

### CUP Optimization

Cursor position (CUP) sequences are elided when the next cell is at
`cursor_x + cell_width, cursor_y` -- the terminal auto-advances the
cursor after character output. The `needs_cup()` function determines
whether a CUP is required between consecutive cell updates.

### Wide Character Handling in Diff

When a cell has `width > 1` (CJK character), the diff algorithm marks
the following padding cells as invalidated, ensuring they are re-emitted
to prevent visual corruption.

### Colour Encoding in SGR

Colours are encoded in SGR parameters using the standard ranges:

- Basic 8 colours: params 30-37 (fg) / 40-47 (bg)
- Bright 8 colours: params 90-97 (fg) / 100-107 (bg)
- 256-colour palette: params 38;5;n (fg) / 48;5;n (bg)
- True colour RGB: params 38;2;r;g;b (fg) / 48;2;r;g;b (bg)

The encoder tracks current colour state and only emits changes.

---

## 7. Flood Fairness

### Problem

When a pane floods output (e.g., `yes`, `cat /dev/urandom | xxd`),
the renderer must not spend all its time rendering that one pane while
other panes become unresponsive.

### Solution: Per-Pane Row Quota with Stride Rotation

1. **Budget**: Each render cycle has a total row budget (configurable,
   default 256).

2. **Quota**: Each visible pane receives `budget / num_visible_panes`
   rows. Remainder rows are distributed to the first panes.

3. **Stride**: Panes are rendered in rotated order. The `stride_offset`
   advances by 1 each cycle, preventing positional bias:
   - Cycle 0: [P1, P2, P3]
   - Cycle 1: [P2, P3, P1]
   - Cycle 2: [P3, P1, P2]

4. **Deferral**: If a pane's dirty rows exceed its quota, excess rows
   are deferred via `defer_rows()`. Deferred rows are tracked per-pane
   and gradually consumed as the pane quiets down.

### Fairness Guarantee

No pane can monopolize more than `1/N` of the render budget per cycle,
where N is the number of visible panes. The stride rotation ensures
that position-based advantages are eliminated over time.

### Verified by Testing

Fairness is verified by unit tests that simulate flood scenarios with
multiple panes, checking that:
- Total rendered rows never exceed the budget.
- Each pane receives approximately equal quota.
- Stride rotation prevents starvation over multiple cycles.

---

## 8. Channel Architecture

### All Channels Are Bounded

Every channel in TermForge has a fixed bound. There are no unbounded
channels anywhere in the codebase. This is an architectural invariant
enforced by code review and testing.

### Channel Bounds

| Channel | Bound | Direction | Content |
|---------|-------|-----------|---------|
| control | 512 | Client -> Kernel | Commands, resize events |
| data | 1024 x 64KiB | IO -> Kernel | PTY output frames |
| render | 256 | Kernel -> Render | Grid snapshots |
| effect | 1024 | Kernel -> IO | Effects to execute |

### Backpressure Strategy

When a channel is full:

- **control**: IO thread blocks. Commands are never dropped.
- **data**: IO thread drops the oldest PTY data frame (not commands).
  Data loss is acceptable here -- the terminal will self-correct on
  the next full redraw.
- **render**: Kernel skips the render request. The render thread will
  catch up on the next cycle.
- **effect**: This should never happen (effects are small and fast).
  If it does, the kernel blocks, which is a bug indicator.

### Channel Health Monitoring

Channel queue depths are tracked via the mux-otel integration:
- Sustained high-water marks indicate a slow consumer.
- Empty channels indicate an idle system.
- The diagnostic tool (mux-doctor) can report channel saturation.

---

## 9. Wire Protocol

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

Every frame payload is validated with CRC32 (using `crc32fast`).
Corrupted frames are rejected with `ProtoError::CrcMismatch`.

### Protocol Auto-Detection

The `is_tf01_frame()` function checks the first 4 bytes:
- If `0x54464F31` (TFO1): use TF01 protocol.
- Otherwise: fall back to tmux imsg format.

### tmux v8 Compatibility

TermForge supports both TF01 (native Rust-to-Rust protocol using
bincode via serde) and tmux's native wire format. The protocol layer
is a thin translation -- both share the same kernel command model.

---

## 10. Session/Window/Pane Object Model

### Entity Hierarchy

```
Server
  +-- Session (SessionId)
  |     +-- Window (WindowId)
  |     |     +-- Pane (PaneId)
  |     |     |     +-- ChunkedGrid
  |     |     |     +-- VtParser
  |     |     |     +-- GraphemeArena
  |     |     |     +-- exited: bool
  |     |     +-- LayoutNode
  |     +-- active_window_idx
  +-- Client (ClientId)
        +-- attached Session
        +-- terminal Size
```

### ID Generation

Entity IDs are monotonically increasing u64 values via `IdGenerator`.
They are never reused, simplifying garbage collection and preventing
use-after-free patterns. The generator starts at 1 (not 0) so that
0 can serve as a sentinel value.

### Session Management

- Sessions contain an ordered list of windows and an active window index.
- Session names must be unique within a server.
- Sessions can be created, renamed, and destroyed.
- When the last window in a session is closed, the session is destroyed.

### Window Management

- Windows contain an ordered list of panes and a layout tree.
- Windows belong to exactly one session.
- Window names can be set explicitly or auto-generated from the active
  pane's command.
- Windows can be moved between sessions.

### Pane Management

- Each pane owns a ChunkedGrid, a VtParser, and a GraphemeArena.
- Panes track their exit status (running or exited with status code).
- Each pane has a PaneId used for all cross-referencing.
- PTY output flows: IO thread -> KernelEvent -> pane.process_output().

### Option Inheritance

Options are resolved via layered lookup:
1. Pane-level option table
2. Window-level option table
3. Session-level option table
4. Server-level option table
5. Built-in defaults (`builtin_defaults()`)

---

## 11. Layout Engine

### Binary Tree Model

The layout engine uses a binary tree of `LayoutNode`s. Each node is either:

- `Leaf`: A leaf pane with position and dimensions.
- `Split`: A container with orientation (Horizontal/Vertical),
  position, dimensions, and child nodes.

This matches tmux's `layout_cell` tree from `layout.c`.

### Built-in Algorithms

| Algorithm | Behavior | Reference |
|-----------|----------|-----------|
| even-horizontal | Equal-width vertical splits | `layout-set.c` |
| even-vertical | Equal-height horizontal splits | `layout-set.c` |
| main-horizontal | Main pane on top, rest below | `layout-set.c` |
| main-vertical | Main pane on left, rest right | `layout-set.c` |
| tiled | Grid arrangement (sqrt-based) | `layout-set.c` |

### Dimension Redistribution

When the terminal is resized, the layout engine redistributes space:

1. Compute base size: `parent_dimension / num_children`.
2. Compute remainder: `parent_dimension % num_children`.
3. Distribute remainder pixels left-to-right (first children get +1).
4. The sum of all child dimensions exactly equals the parent size.

This invariant is verified by proptest: for any number of panes (1..20)
and any parent dimension (20..500), the sum of child sizes must equal
the parent.

### Minimum Pane Size

No pane can be smaller than 2 cols x 1 row (`Size::MIN_PANE`). The
`meets_minimum()` and `clamp_to_minimum()` methods enforce this.

### Custom Layout Strings

tmux supports custom layout strings with a checksum format:
```
89eb,120x40,0,0{60x40,0,0,0,59x40,61,0,1}
```

The `validate_layout_checksum()` function validates the 4-hex-digit
checksum before applying the layout.

---

## 12. Copy Mode

### State Machine

Copy mode is entered per-pane via the `copy-mode` command. The state
tracks:

- **Key table**: Vi or Emacs (explicit, never guessed).
- **Selection mode**: None, Character, Line, or Block.
- **Cursor position**: Independent of the pane's normal cursor.
- **Scroll offset**: How far into scrollback the view is.
- **Search state**: Term, direction (forward/backward).
- **Buffer ring**: Bounded list of copied text (default max 50).

### Selection Modes

| Mode | Vi Key | Emacs Key | Behavior |
|------|--------|-----------|----------|
| None | (default) | (default) | No selection active |
| Character | Space | Ctrl-Space | Select individual characters |
| Line | V | (not default) | Select entire lines |
| Block | Ctrl-V | (not default) | Rectangle selection |

### Cursor Movement

Cursor movement in copy mode is decoupled from the grid cursor:

- `cursor_up(n)`: Moves up, scrolling into scrollback if needed.
- `cursor_down(n, height)`: Moves down, reducing scroll offset.
- `cursor_left()`: Saturating subtraction, stops at column 0.
- `cursor_right(width)`: Bounded by screen width.

### Buffer Ring

Copied text is pushed to a bounded ring buffer (`BufferRing`).
When the ring reaches `max_entries` (default 50), the oldest entry
is evicted. Access by index (0 = most recent) allows `paste-buffer`
to retrieve any entry.

---

## 13. Options System

### Architecture

The options system uses typed option tables with layered inheritance.
Each entity (server, session, window, pane) has its own OptionTable.

### Option Value Types

```rust
pub enum OptionValue {
    String(String),
    Int(i64),
    Bool(bool),
    Colour(Colour),
    Style(Style),
}
```

### Resolution Order

Options are resolved from most specific to least specific:
1. Pane-level
2. Window-level
3. Session-level
4. Server-level
5. Built-in defaults

The `resolve_option()` function walks the chain and returns the first
match. If no entity-specific override exists, the built-in default
is returned.

### Built-in Defaults

Defaults are defined in `builtin_defaults()` and include:
- `default-terminal` = "tmux-256color"
- `escape-time` = 500 (milliseconds)
- `history-limit` = 2000
- `focus-events` = false
- `mouse` = false
- `allow-passthrough` = false (INV-220)
- `status` = true
- `base-index` = 0

### User-Defined Options

User options prefixed with `@` (e.g., `@my-option`) are stored in the
option table like any other option but have no built-in default.

---

## 14. Format String Expansion

### Syntax

tmux format strings use `#{}` delimiters:

```
#{session_name}: #{window_index}.#{pane_index} [#{pane_width}x#{pane_height}]
```

### Supported Features

- **Variables**: `#{session_name}`, `#{window_index}`, `#{pane_pid}`, etc.
- **Conditionals**: `#{?test,true_val,false_val}`
- **Comparison**: `#{==:#{left},#{right}}`
- **Substitution**: `#{s/pattern/replacement/:value}`
- **Literals**: `##{` for a literal `#{`

### Implementation

The format evaluator in mux-format:

1. Tokenizes the format string into literal and expression segments.
2. Evaluates each expression against the current context (session,
   window, pane, client).
3. Handles nested expressions recursively.
4. Returns the concatenated result.

### Error Handling

Invalid format strings return the literal text unchanged. This matches
tmux's behavior of silently passing through unrecognized format tokens.

---

## 15. Command Parse Pipeline

### Parsing Stages

tmux commands flow through three stages:

1. **Lexing**: Split raw input into tokens (respecting quotes, escapes,
   semicolons for command chaining).
2. **Parsing**: Match tokens against the command table to produce a
   structured `ParsedCommand`.
3. **Dispatch**: Execute the parsed command against the kernel.

### .tmux.conf Compatibility

The parser handles `.tmux.conf` files:

- `#` comment lines (ignored)
- Single and double quoted strings
- Backslash continuation lines (\ at end of line)
- Semicolon command chaining (`cmd1 ; cmd2`)
- Backslash escaping within double quotes
- Conditional execution (`if-shell`)

### Command Table

Each command entry defines:

- Command name and aliases
- Minimum and maximum argument counts
- Accepted flags with argument requirements
- Target syntax (session, window, pane, client)

### Error Recovery

Parse errors produce structured diagnostics with:

- The position of the error in the input
- The expected tokens at that position
- A suggestion for common typos

---

## 16. ORM Layer (mux-orm)

### Purpose

mux-orm provides an ORM-like query interface for traversing the
kernel's entity graph. This is a read-only view that does not
require mutable access to the kernel.

### ServerView

```rust
pub struct ServerView {
    sessions: Vec<SessionView>,
}

impl ServerView {
    pub fn sessions(&self) -> &[SessionView];
    pub fn session_by_name(&self, name: &str) -> Option<&SessionView>;
    pub fn all_panes(&self) -> Vec<&PaneView>;
    pub fn pane_count(&self) -> usize;
}
```

### QuerySet-Like API

```rust
let dev_sessions = view.sessions()
    .iter()
    .filter(|s| s.name.contains("dev"));

let large_panes = view.all_panes()
    .into_iter()
    .filter(|p| p.cols > 80);
```

### Window and Pane Views

Window and pane views provide read-only access to entity state:

```rust
pub struct PaneView {
    pub id: PaneId,
    pub cols: u16,
    pub rows: u16,
    pub active: bool,
    pub exited: bool,
}
```

---

## 17. Termlet API

### Purpose

A "termlet" is a lightweight, in-process terminal emulator for testing.
It combines a VT parser and a grid, allowing tests to:

1. Feed bytes (simulating PTY output).
2. Read screen content (capturing the visual state).
3. Verify terminal behavior deterministically.

### Lifecycle

```
Created -> spawn(cmd) -> Running -> background() -> Background
   |                        |            |
   |                        +---- foreground() --+
   |                        |                    |
   +--- feed(data) -> Running    Running <-------+
                        |
                   shutdown() -> Shutdown
```

### API Surface

| Method | Description |
|--------|-------------|
| `Termlet::new(size)` | Create with given dimensions |
| `Termlet::with_clock(size, clock)` | Create with explicit clock |
| `spawn(cmd)` | Mark as running, record command |
| `feed(data)` | Feed raw bytes through VT parser to grid |
| `capture()` | Snapshot active screen text |
| `line_text(row)` | Get text of a single line |
| `screen_text()` | Get all active screen text |
| `cursor()` | Current cursor position |
| `size()` | Grid dimensions |
| `resize(cols, rows)` | Resize the terminal |
| `send_keys(keys)` | Feed keystroke bytes |
| `wait_for(pattern, timeout)` | Wait for content to appear |
| `background()` | Send to background |
| `foreground()` | Bring back to foreground |
| `shutdown()` | Graceful cleanup |
| `state()` | Current lifecycle state |
| `command()` | The spawned command |

### Testing Example

```rust
let mut t = Termlet::new(Size::new(80, 24));
t.feed(b"Hello, World!\r\nSecond line");
assert_eq!(t.line_text(0), "Hello, World!");
assert_eq!(t.line_text(1), "Second line");
assert_eq!(t.cursor(), (11, 1));
```

---

## 18. Test Support

### TestServer

The `TestServer` from mux-test-support provides a pre-configured
kernel with a manual clock for integration tests:

```rust
let server = TestServer::new();
let session_id = server.create_session("test");
server.feed_pane(pane_id, b"output");
let text = server.capture_pane(pane_id);
```

### Socket Isolation

All test sockets use `isolated_socket_path()` which generates paths
under `/tmp/mux-test-<PID>-<UUID>/`. This ensures:

- Tests never touch the default tmux socket directory.
- Concurrent test runs do not conflict.
- Socket files are cleaned up automatically.

### CleanupGuard

The `CleanupGuard` RAII type removes socket files and directories on
drop. Combined with TestServer, this ensures cleanup even on panic.

### Deterministic Time

All tests use `Clock::manual()` for deterministic time. This eliminates:

- Wall-clock dependent flakiness.
- Race conditions in timeout tests.
- Non-reproducible failures.

### Version-Aware Fixtures

Tests can be annotated with version requirements:

```rust
#[test]
fn test_feature_requiring_3_3a() {
    let server = TestServer::new();
    if server.tmux_version() < Version::new(3, 3, "a") {
        return; // Skip on older versions
    }
    // ... test body
}
```

### Test Naming Conventions

- `unique_session_name()`: Generates UUID-based session names.
- `unique_socket_path()`: Generates isolated socket paths.
- Session names include the test function name for debugging.

---

## 19. OpenTelemetry Integration

### Architecture

mux-otel provides optional OpenTelemetry integration:

- Disabled by default (zero overhead when off).
- Compile-time feature gate for conditional inclusion.
- Context propagation across thread boundaries.
- Trace ID correlation across client/server boundary.

### Span Types

| Span | Scope | Attributes |
|------|-------|------------|
| `kernel.process_event` | Single event processing | event_type |
| `render.cycle` | Full render cycle | pane_count, dirty_rows |
| `session.create` | Session creation | session_name |
| `pane.output` | PTY output processing | pane_id, bytes |
| `command.dispatch` | Command execution | command_name |

### Initialization

```rust
let config = OtelConfig::builder()
    .service_name("termforge")
    .endpoint("http://localhost:4317")
    .build();
init_otel(&config)?;
```

### Shutdown

`shutdown_otel()` flushes all pending spans before process exit.
This is called during the ordered shutdown sequence.

---

## 20. FFI Surface

### C API Surface

```c
FfiGrid* grid_create(unsigned sx, unsigned sy, unsigned hlimit);
void grid_destroy(FfiGrid* ptr);
unsigned grid_sx(const FfiGrid* ptr);
unsigned grid_sy(const FfiGrid* ptr);
int is_tf01_frame(const uint8_t* data, unsigned len);
const char* termforge_version(void);
```

### Safety Model (INV-117)

1. All functions check for null pointers before dereferencing.
2. All functions use `catch_unwind` to prevent panic propagation.
3. All functions return null/0 on error instead of panicking.
4. The `FfiGrid` struct is opaque to C code.
5. `#[unsafe(no_mangle)]` used for Rust 2024 compliance.

### Panic Recovery

The FFI boundary uses `std::panic::catch_unwind` to prevent Rust
panics from unwinding through C code (which is undefined behavior).
The workspace-level `panic = "unwind"` setting in all profiles ensures
catch_unwind works correctly.

### Workspace-Level Panic Prevention

- Clippy denies `unwrap_used`, `expect_used`, `panic` in all crates.
- `unwrap_or`, `unwrap_or_else`, `unwrap_or_default`, or `let-else`
  are used instead.
- `panic = "unwind"` is set in dev, test, and release profiles.

---

## 21. Gap Analysis and Resolutions

### Gap-1: TIOCGPTPEER

**Problem**: PTY allocation via traditional openpty() has TOCTOU race.
**Resolution**: `PtyAllocStrategy` enum in mux-pty with `Traditional`
(openpty) and `Tiocgptpeer` (Linux 4.13+) variants. Runtime detection
selects the appropriate strategy.

### Gap-2: Raw Mode Lifecycle

**Problem**: Terminal mode management is complex with multiple embedding
scenarios.
**Resolution**: `RawModeGuard` RAII type in mux-pty with
`TerminalModePolicy` enum (Managed, External, SaveRestore). Guard
restores terminal state on drop, even during panic.

### Gap-3: SIGWINCH Propagation Flow

**Problem**: The exact signal -> query -> propagate -> render sequence
was unspecified.
**Resolution**: Documented 9-step sequence:
1. signal-hook registers pipe-based SIGWINCH handler.
2. IO thread reads from signal pipe via tokio AsyncFd.
3. IO thread queries terminal size via ioctl(TIOCGWINSZ).
4. IO thread sends KernelEvent::ClientResize.
5. Kernel updates client size, recalculates layouts.
6. Kernel emits KernelEffect::ResizePty for each pane.
7. Kernel emits KernelEffect::Render.
8. IO thread executes ResizePty via ioctl(TIOCSWINSZ).
9. Render thread produces new frame.

### Gap-4: crossterm Rejection Rationale

**Problem**: crossterm is the "obvious" choice but wrong for a multiplexer.
**Resolution**: Documented in mux-pty crate docs. Reasons:
- Platform detail abstraction hides needed ioctls.
- Event model mismatch (single terminal vs. multiplexer).
- Direct ioctl requirement for TIOCGWINSZ, TIOCGPTPEER.
- Custom parser for tmux behavioral compatibility.

### Gap-5: signal-hook vs tokio signals

**Problem**: Signal handling strategy was unspecified.
**Resolution**: signal-hook chosen for:
- Runtime-agnostic pipe-based delivery.
- AsyncFd integration with tokio.
- Alignment with Sans-IO kernel model.
- Correct async-signal-safety handling.

### Gap-6: Composition/Diff Rendering

**Problem**: How rendering actually works was not specified.
**Resolution**: Full terminal-to-terminal render pipeline in mux-render:
- composite.rs: double-buffer composition.
- diff.rs: cell diff with wide-char invalidation.
- sgr.rs: attribute encoding optimization.
- flood.rs: fairness quotas with stride rotation.

### Gap-7: Layout Engine

**Problem**: Layout algorithms were stub-only.
**Resolution**: Implemented in mux-kernel/layout.rs with 5 built-in
algorithms plus custom layout string parsing with checksum validation.
Verified by proptest for dimension sums.

### Gap-8: Zombie Reaping

**Problem**: SIGCHLD handling was not specified.
**Resolution**: SIGCHLD handler drains `waitpid(-1, WNOHANG)` in a
loop until no more children. This handles multiple children exiting
between signal deliveries.

### Gap-9: Job Control Passthrough

**Problem**: Signal forwarding to children was ambiguous.
**Resolution**: Signal non-interference invariant: TermForge does NOT
intercept SIGTSTP/SIGCONT/SIGTTIN/SIGTTOU. Child process groups manage
their own job control independently.

### Gap-10: SCM_RIGHTS fd Passing

**Problem**: PTY master handoff mechanism was unspecified.
**Resolution**: mux-fdpass provides FdEnvelope with 16-fd limit,
validation, and FdType classification. Actual sendmsg/recvmsg in
mux-pty (unsafe quarantine). Not used in TF01 mode.

### Gap-11: SDK Ergonomics

**Problem**: API design for embedders was unclear.
**Resolution**: mux-api provides `MuxServer::builder()` pattern with
fluent configuration and `Clock::manual()` injection for testing.
SessionBuilder with window/pane chaining.

### Gap-12: Graphics Passthrough Stance

**Problem**: Sixel/iTerm2/Kitty support strategy was unspecified.
**Resolution**: INV-220 enforced. `allow_passthrough` defaults to
`false`. DCS sequences from graphics protocols are consumed and
discarded when disabled. Acknowledged as future consideration.

---

## 22. Terminal Mode Lifecycle

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

### Policy Behavior

- **Managed**: TermForge calls `cfmakeraw()` and `tcsetattr()`.
  On drop, the saved `termios` is restored.
- **External**: TermForge assumes the caller manages raw mode.
  No terminal state changes are made.
- **SaveRestore**: TermForge queries the current terminal state,
  saves it, enters raw mode, and restores on drop.

---

## 23. PTY Allocation Strategy

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
1. Try TIOCGPTPEER on a test PTY.
2. If ENOTTY or EINVAL, fall back to traditional.
3. Cache the strategy for all subsequent allocations.

### Security Benefits

TIOCGPTPEER eliminates the TOCTOU race between `ptsname()` and
`open()` where a malicious process could replace the slave device.
This is particularly important in multi-tenant environments.

---

## 24. Graphics Passthrough Policy

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

## 25. SDK Ergonomics (mux-api)

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
    .default_size(Size::new(120, 40))
    .build()?;
```

### Fluent Session Building

```rust
let session_id = server
    .session("dev")
    .window("editor")
    .pane(PaneConfig::new().command("vim"))
    .window("terminal")
    .pane(PaneConfig::default())
    .build()?;
```

### Simple API

```rust
let session_id = server.new_session("main")?;
```

### Configuration Validation

The builder validates configuration at build time:
- Empty socket paths are rejected.
- Invalid terminal modes are caught.
- Clock injection ensures test determinism.

### Error Handling

All API methods return `Result<T, ApiError>`. Error variants include:
- `ApiError::Config`: Invalid configuration.
- `ApiError::Kernel`: Internal kernel error.
- `ApiError::NotRunning`: Server not started.
- `ApiError::SessionNotFound`: Named session does not exist.
- `ApiError::InvalidArgument`: Bad parameter value.

---

## 26. Tool Crates

### tmux-builder

A workspace layout tool that reads YAML/TOML layout definitions and
constructs tmux sessions:

- Parse layout files with session/window/pane definitions.
- Apply layouts to a TermForge or tmux server.
- Validate layout files before applying.
- Supports version-specific features via tmux version detection.

### tmux-vm

A headless multiplexer for CI environments:

- Manage multiple tmux installations.
- Download and compile tmux from source at any version.
- Run specific versions: `tmux-vm run 3.3a -- list-sessions`.
- Cache compiled binaries for fast switching.

### tmux-sniff

A wire protocol inspector:

- Capture live protocol traffic from Unix sockets.
- Hex dump and structured decode of TF01 frames.
- Filter by message type, session, or pane.
- Replay from capture files.

### mux-doctor

A diagnostic environment checker:

- Check TERM, LANG/LC_* environment variables.
- Check terminal capabilities via terminfo.
- Check tmux socket accessibility and running servers.
- Check PTY availability (try allocating one).
- Check signal-hook compatibility.
- Output structured JSON or human-readable reports.

---

## Appendix A: LOC Estimates

| Crate | Scaffold LOC | Full Target LOC | Phase |
|-------|-------------|----------------|-------|
| mux-grapheme-arena | 180 | 250 | P0 |
| mux-time | 310 | 400 | P0 |
| mux-types | 700 | 1,000 | P0 |
| mux-grid | 830 | 2,000 | P0 |
| mux-parser | 680 | 3,000 | P1 |
| mux-options | 230 | 600 | P1 |
| mux-target | 200 | 500 | P1 |
| mux-format | 140 | 800 | P1 |
| mux-cmd-parse | 190 | 1,200 | P1 |
| mux-proto | 320 | 1,500 | P1 |
| mux-crdt | 340 | 800 | P2 |
| mux-fdpass | 165 | 400 | P2 |
| mux-snapshot | 200 | 500 | P1 |
| mux-kernel | 1,050 | 8,000 | P1-P2 |
| mux-render | 620 | 3,000 | P1 |
| mux-orm | 140 | 500 | P2 |
| mux-pty | 170 | 1,000 | P2 |
| mux-termlet | 400 | 800 | P1 |
| mux-config | 340 | 600 | P1 |
| mux-api | 540 | 1,000 | P1 |
| mux-test-support | 380 | 700 | P1 |
| mux-ffi | 270 | 500 | P2 |
| mux-otel | 310 | 400 | P2 |
| tmux-builder | 320 | 2,000 | P3 |
| tmux-vm | 260 | 1,500 | P3 |
| tmux-sniff | 250 | 800 | P3 |
| mux-doctor | 360 | 600 | P3 |
| **Total** | **~9,900** | **~33,350** | |

## Appendix B: Risk Register

| ID | Risk | Probability | Impact | Mitigation |
|----|------|-------------|--------|------------|
| R1 | VT parser completeness | High | Medium | Fuzz with arbitrary bytes, golden tests |
| R2 | Grid reflow correctness | High | Medium | Port tmux reflow tests, proptest |
| R3 | tmux wire protocol drift | Medium | High | Multi-version golden tests via tmux-vm |
| R4 | Language binding churn | Medium | Medium | Pin PyO3/napi-rs versions, thin FFI wrapper |
| R5 | Channel deadlock under load | Medium | High | Dedicated backpressure, timeout sends |
| R6 | SCM_RIGHTS portability | Low | Medium | Platform-specific cfg, CI matrix |
| R7 | Layout engine complexity | High | High | Port tmux's tested algorithms directly |
| R8 | Copy mode edge cases | Medium | Medium | Use tmux test suite as oracle |
| R9 | Graphics passthrough security | Medium | High | Disabled by default, audit DCS sequences |
| R10 | Performance regression | Medium | High | Benchmark suite, regression CI |
| R11 | Wide char reflow | High | High | Proptest with CJK character generators |
| R12 | CRDT convergence bugs | Medium | High | Proptest with concurrent mutations |

## Appendix C: Decision Log

### D1: Own VT Parser vs. vte/vt100/termwiz
**Decision**: Own parser (mux-parser).
**Rationale**: tmux behavioral compatibility requires matching tmux's
specific VT interpretation. The vte crate is push-based and would
require significant wrapping to match tmux's processing model.

### D2: Own Grid vs. alacritty_terminal
**Decision**: Own grid (mux-grid).
**Rationale**: alacritty's grid is optimized for GPU rendering.
A multiplexer needs Arc-COW lines for cheap snapshots.

### D3: signal-hook vs. tokio signals
**Decision**: signal-hook for pipe-based signal bridge.
**Rationale**: The kernel thread is std::thread, not tokio. signal-hook
provides runtime-agnostic pipe-based delivery.

### D4: crossterm Rejection
**Decision**: Do not use crossterm.
**Rationale**: crossterm abstracts away platform-specific ioctls that
a multiplexer needs direct access to.

### D5: Sans-IO Kernel Threading
**Decision**: Single-threaded kernel on std::thread.
**Rationale**: No I/O, no preemption, predictable latency, deterministic
testing with Clock::manual().

### D6: Dual Protocol
**Decision**: TF01 (bincode) for native, tmux format for compatibility.
**Rationale**: Type-safe Rust-to-Rust via bincode, tmux wire compat
via translation layer. Both share the kernel command model.
