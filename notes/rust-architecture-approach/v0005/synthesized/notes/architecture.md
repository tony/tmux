# TermForge v0005 Architecture Document

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Design Philosophy](#2-design-philosophy)
3. [Project Structure](#3-project-structure)
4. [Crate Dependency Graph](#4-crate-dependency-graph)
5. [Layer 0: Leaf Crates](#5-layer-0-leaf-crates)
6. [Layer 1: Core Types](#6-layer-1-core-types)
7. [Layer 2: Data Structures](#7-layer-2-data-structures)
8. [Layer 3: Business Logic](#8-layer-3-business-logic)
9. [Layer 4: Integration](#9-layer-4-integration)
10. [Layer 5: External Surface](#10-layer-5-external-surface)
11. [Layer 6: Tools](#11-layer-6-tools)
12. [Sans-IO Kernel Architecture](#12-sans-io-kernel-architecture)
13. [Grid and Cell Model](#13-grid-and-cell-model)
14. [VT Parser State Machine](#14-vt-parser-state-machine)
15. [Double-Buffer Diff Rendering](#15-double-buffer-diff-rendering)
16. [CRDT and Collaboration](#16-crdt-and-collaboration)
17. [Wire Protocol (TF01)](#17-wire-protocol-tf01)
18. [SCM_RIGHTS File Descriptor Passing](#18-scm_rights-file-descriptor-passing)
19. [ORM and QuerySet API](#19-orm-and-queryset-api)
20. [Termlet Subsystem](#20-termlet-subsystem)
21. [PTY Management](#21-pty-management)
22. [Configuration System](#22-configuration-system)
23. [Layout Engine](#23-layout-engine)
24. [Copy Mode](#24-copy-mode)
25. [Snapshot System](#25-snapshot-system)
26. [OpenTelemetry Observability](#26-opentelemetry-observability)
27. [FFI and Language Bindings](#27-ffi-and-language-bindings)
28. [Testing Strategy](#28-testing-strategy)
29. [Error Handling Strategy](#29-error-handling-strategy)
30. [Performance Considerations](#30-performance-considerations)
31. [Platform Abstraction](#31-platform-abstraction)
32. [Terminal Mode Policy](#32-terminal-mode-policy)
33. [Flood Fairness](#33-flood-fairness)
34. [Signal Handling](#34-signal-handling)
35. [Gap Resolutions](#35-gap-resolutions)
36. [Invariant Catalog](#36-invariant-catalog)
37. [Settled Decisions](#37-settled-decisions)
38. [Future Considerations](#38-future-considerations)

---

## 1. Executive Summary

TermForge is a terminal multiplexer SDK built from scratch in Rust. It is
designed as a layered architecture that provides:

- **Full tmux compatibility**: Wire protocol v8, command set, option names,
  and behavioral parity with the tmux terminal multiplexer.
- **SDK-first design**: An embeddable library API that allows programmatic
  control of terminal sessions, windows, panes, and PTY processes.
- **ORM-like traversal**: A QuerySet API inspired by Python's libtmux that
  enables fluent filtering and navigation of the session/window/pane graph.
- **Termlet testing pods**: Lightweight terminal instances for testing that
  provide visual area capture, resize, interaction, and shell session
  management across Rust, Node.js, and Python.
- **CRDT collaboration**: Optional vector-clock-based conflict resolution for
  multi-user terminal collaboration scenarios.
- **Language bindings**: FFI surface for C, with Python and Node.js bindings
  planned via PyO3 and napi-rs.
- **OpenTelemetry tracing**: Cross-crate and cross-service observability.

The architecture is organized into 7 layers (L0-L6) with 23 library crates
and 4 tool binaries. Dependencies flow strictly downward: a crate at layer N
may only depend on crates at layers 0 through N-1.

### Key Architectural Decisions

1. **Sans-IO kernel**: The core multiplexer logic is pure and deterministic,
   receiving typed events and producing typed effects with no I/O.
2. **Arc-based COW grid lines**: Grid lines use `Arc<Vec<Cell>>` for
   zero-copy snapshot isolation during rendering.
3. **Double-buffer diff rendering**: Frame output uses a double-buffer pattern
   where the renderer diffs the current frame against the previous one to
   emit minimal escape sequences.
4. **Paul Williams VT parser**: A 14-state machine with a 256-entry lookup
   table for fast byte classification.
5. **Binary tree layout**: Pane layout uses a binary tree with horizontal and
   vertical splits, supporting recursive nesting.

---

## 2. Design Philosophy

### 2.1 Layered Architecture

The crate dependency graph is strictly acyclic and layered. Each layer has a
clear responsibility:

- **L0 (Leaf)**: Zero internal dependencies. Can compile to WASM.
- **L1 (Types)**: Shared type definitions used by all other layers.
- **L2 (Data)**: Data structures and parsers that depend only on types.
- **L3 (Logic)**: Business logic that combines data structures with algorithms.
- **L4 (Integration)**: Crates that wire together L3 components into usable APIs.
- **L5 (Surface)**: External-facing crates (FFI, telemetry) that expose L4.
- **L6 (Tools)**: Binary executables.

This layering ensures that core logic (L0-L2) remains testable without any
runtime dependencies, and that the PTY/IO boundary (in mux-pty) is isolated
from pure logic.

### 2.2 Sans-IO Pattern

The central architectural pattern is sans-IO: the kernel processes events and
produces effects without performing any I/O directly. This is inspired by:

- **tmux's single-threaded event loop**: All state mutation in one place.
- **The Elm Architecture**: Pure update functions with effect descriptions.
- **WezTerm's Mux pattern**: Centralized state with trait-based extensibility.

The sans-IO kernel enables:
- Deterministic testing without mocking I/O.
- Replay and snapshot-based debugging.
- CRDT-compatible state transitions.
- Embedding in WASM or headless environments.

### 2.3 Composition Over Inheritance

Rust does not have inheritance. TermForge uses composition throughout:

- A `Pane` contains a `ChunkedGrid`, not inheriting from it.
- A `Server` contains a `Kernel`, not inheriting from it.
- A `Termlet` contains a `TestServer`, not inheriting from it.

Traits are used for abstraction at boundaries (e.g., `PtySpawner` trait in
mux-pty) but concrete types are preferred for internal wiring.

### 2.4 Test-First Design

Every architectural decision is validated by tests:

- **Property tests** verify algebraic properties (commutativity, idempotency).
- **Snapshot tests** capture complex output for regression detection.
- **Differential tests** compare behavior against the real tmux binary.
- **Fuzz tests** explore edge cases in parsers and codecs.

The target is 1,100+ tests across the workspace, with proptest coverage for
all encoding/decoding round-trips.

---

## 3. Project Structure

```
termforge/
  Cargo.toml                     # Workspace root
  CLAUDE.md                      # Project conventions
  AGENTS.md                      # Rule catalog and agent guidelines
  notes/
    architecture.md              # This document
    plan.md                      # Implementation roadmap
  crates/
    mux-grapheme-arena/          # L0: Grapheme string interning
    mux-time/                    # L0: Deterministic time, timer wheel
    mux-types/                   # L1: Core type definitions
      src/
        lib.rs                   # Re-exports
        attrs.rs                 # Terminal attributes (bold, italic, etc.)
        cell.rs                  # Grid cell with grapheme, colours, flags
        colour.rs                # Colour enum (Default, Indexed, Rgb)
        error.rs                 # Unified MuxError
        geometry.rs              # Size, Rect, Position
        id.rs                    # SessionId, WindowId, PaneId
        key.rs                   # Key enum with modifiers
        style.rs                 # StyleDelta for tracking changes
    mux-grid/                    # L2: Grid data structure
    mux-parser/                  # L2: VT escape sequence parser
      src/
        lib.rs                   # Re-exports
        state_machine.rs         # 14-state VT parser
        csi.rs                   # CSI sequence dispatcher
        input.rs                 # InputParser with grid integration
    mux-options/                 # L2: Option table with scoped resolution
    mux-target/                  # L2: tmux target syntax parser
    mux-format/                  # L2: tmux format string engine
    mux-cmd-parse/               # L2: Command tokenizer and parser
    mux-proto/                   # L2: Wire protocol frames
    mux-crdt/                    # L2: CRDT primitives
    mux-fdpass/                  # L2: SCM_RIGHTS fd passing envelope
    mux-snapshot/                # L3: Grid snapshot capture
    mux-kernel/                  # L3: Sans-IO multiplexer kernel
      src/
        lib.rs                   # Kernel struct and event dispatch
        event.rs                 # KernelEvent and KernelEffect enums
        layout.rs                # Binary tree layout engine
        copy_mode.rs             # Vi/Emacs copy mode state
        session.rs               # Session model (stub)
        window.rs                # Window model (stub)
        pane.rs                  # Pane model (stub)
    mux-render/                  # L3: Double-buffer diff renderer
    mux-orm/                     # L3: QuerySet API for entity traversal
    mux-pty/                     # L3: PTY allocation and management
    mux-termlet/                 # L4: Testing pods
    mux-config/                  # L4: Configuration loading
    mux-api/                     # L4: Public SDK API
    mux-test-support/            # L4: Test harness utilities
    mux-ffi/                     # L5: C FFI surface
    mux-otel/                    # L5: OpenTelemetry integration
  tools/
    tmux-builder/                # L6: Build tmux from source
    tmux-vm/                     # L6: tmux version manager
    tmux-sniff/                  # L6: Wire protocol sniffer
    mux-doctor/                  # L6: Diagnostic tool
  bindings/
    node/                        # Node.js bindings (excluded from workspace)
    python/                      # Python bindings (excluded from workspace)
```

---

## 4. Crate Dependency Graph

### Layer Diagram

```
L6: tmux-builder  tmux-vm  tmux-sniff  mux-doctor
     |              |         |           |
     v              v         v           v
L5: mux-ffi                         mux-otel
     |                                 |
     v                                 v
L4: mux-api    mux-config    mux-termlet    mux-test-support
     |  \          |            |   |             |
     |   \         |            |   |             |
     v    v        v            v   v             v
L3: mux-kernel  mux-render  mux-snapshot  mux-orm  mux-pty
     |   |  |      |            |           |        |
     |   |  |      |            |           |        |
     v   v  v      v            v           v        v
L2: mux-grid  mux-parser  mux-options  mux-target  mux-format
     |           |            |           |            |
     |   mux-cmd-parse  mux-proto  mux-crdt  mux-fdpass
     |           |            |        |          |
     v           v            v        v          v
L1: mux-types
     |
     v
L0: mux-grapheme-arena    mux-time
```

### Dependency Matrix

Each crate's direct dependencies (internal only):

| Crate | Depends On |
|-------|-----------|
| mux-grapheme-arena | (none) |
| mux-time | (none) |
| mux-types | mux-grapheme-arena |
| mux-grid | mux-types, mux-grapheme-arena |
| mux-parser | mux-types, mux-grapheme-arena, mux-grid |
| mux-options | mux-types |
| mux-target | (none -- uses thiserror only) |
| mux-format | mux-types |
| mux-cmd-parse | mux-types |
| mux-proto | mux-types |
| mux-crdt | mux-time |
| mux-fdpass | (none -- uses thiserror only) |
| mux-snapshot | mux-grid, mux-types, mux-grapheme-arena |
| mux-kernel | mux-types, mux-grid, mux-parser, mux-grapheme-arena, mux-time |
| mux-render | mux-types, mux-grapheme-arena |
| mux-orm | mux-types |
| mux-pty | mux-types |
| mux-termlet | mux-types, mux-kernel, mux-grid, mux-parser, mux-grapheme-arena, mux-render |
| mux-config | mux-types, mux-options, mux-cmd-parse |
| mux-api | mux-types, mux-kernel, mux-grid, mux-parser, mux-grapheme-arena |
| mux-test-support | mux-types, mux-api, mux-kernel, mux-grid, mux-parser, mux-grapheme-arena |
| mux-ffi | mux-api, mux-types |
| mux-otel | (none -- uses opentelemetry) |

---

## 5. Layer 0: Leaf Crates

### 5.1 mux-grapheme-arena

**Purpose**: String interning for grapheme clusters. Cells in the grid store
a `GraphemeId` (a 32-bit index) rather than a heap-allocated String. The arena
owns the actual UTF-8 strings and provides O(1) lookup by ID.

**Key Types**:
- `GraphemeArena`: The interning table. `intern(&str) -> GraphemeId`.
- `GraphemeId`: A `#[repr(transparent)]` newtype over `u32`. Has `DEFAULT`
  and `from_raw()` constructors.

**Design Rationale**: Terminal grids can have millions of cells. Storing a
String per cell would be prohibitively expensive. The arena deduplicates
identical graphemes (e.g., 'a' appears once regardless of how many cells
contain it) and provides cache-friendly sequential access.

**Invariants**:
- INV-030: Extension index 0x0000 means no extension.
- `GraphemeId::DEFAULT` is always valid and resolves to an empty string.
- `intern()` is idempotent: interning the same string twice returns the same ID.

**Testing**: 7 unit tests + 6 proptests covering intern roundtrip,
idempotency, distinct IDs for distinct strings, Unicode/CJK handling,
and bulk interning.

### 5.2 mux-time

**Purpose**: Deterministic time sources, timestamps, and a timer wheel for
scheduling deferred events (e.g., key repeat timeouts, status bar refresh).

**Key Types**:
- `MonotonicClock`: A clock abstraction that can be real or deterministic.
- `Timestamp`: A monotonic timestamp with arithmetic operations.
- `TimerWheel`: A hierarchical timer wheel with O(1) insert/cancel and
  amortized O(1) expiry.

**Design Rationale**: The kernel must be deterministic under replay. A real
clock would break determinism. The `MonotonicClock` can be replaced with a
deterministic source that advances only when explicitly stepped.

**Invariants**:
- INV-025: VectorClock monotonic per actor (when CRDT feature enabled).
- INV-033: Deterministic replay under fixed clock/seed.
- `Timestamp::ZERO` is the epoch.
- Timer wheel slots wrap around correctly.

**Testing**: 8 unit tests + 6 proptests covering clock advancement, timestamp
arithmetic, timer insertion/expiry/cancellation, and deterministic ordering.

---

## 6. Layer 1: Core Types

### 6.1 mux-types

**Purpose**: The foundational type library shared by all other crates. Contains
types for cells, colours, attributes, geometry, keys, IDs, errors, and styles.

**Modules**:

#### 6.1.1 attrs.rs -- Terminal Attributes

Defines `Attrs` as a `bitflags` type with the following flags:

| Flag | Bit | SGR Code |
|------|-----|----------|
| BOLD | 0x01 | 1 |
| DIM | 0x02 | 2 |
| ITALIC | 0x04 | 3 |
| UNDERLINE | 0x08 | 4 |
| BLINK | 0x10 | 5 |
| REVERSE | 0x20 | 7 |
| HIDDEN | 0x40 | 8 |
| STRIKETHROUGH | 0x80 | 9 |

Also defines `CellFlags` with:

| Flag | Bit | tmux GRID_FLAG_* |
|------|-----|------------------|
| WIDE | 0x01 | EXTENDED |
| PADDING | 0x02 | PADDING |
| WRAPPED | 0x04 | WRAPPED (INV-119) |
| SELECTED | 0x08 | SELECTED |
| SEARCHED | 0x10 | TAG |

**Invariant INV-119**: CellFlags bit values must match tmux's GRID_FLAG_*
constants exactly to ensure behavioral compatibility.

#### 6.1.2 cell.rs -- Grid Cell

The `Cell` struct represents a single character position in the grid:

```rust
pub struct Cell {
    pub grapheme: GraphemeId,
    pub width: u8,          // 0 (padding), 1 (normal), 2 (wide)
    pub fg: Colour,
    pub bg: Colour,
    pub attrs: Attrs,
    pub flags: CellFlags,
    pub link: u16,          // hyperlink ID (0 = none)
}
```

**Methods**:
- `Cell::default()`: Empty cell with GraphemeId::DEFAULT, width 1.
- `Cell::with_grapheme(id, width)`: Construct with specific grapheme.
- `Cell::is_wide()`: Check if width == 2.
- `Cell::is_padding()`: Check if PADDING flag set.
- `Cell::clear()`: Reset all fields to default.

**Invariant INV-029**: Width must be 0, 1, or 2.

#### 6.1.3 colour.rs -- Terminal Colours

```rust
pub enum Colour {
    Default,
    Indexed(u8),         // 0-255 palette
    Rgb { r: u8, g: u8, b: u8 },
}
```

Indexed colours 0-7 are standard, 8-15 are bright, 16-231 are the 6x6x6
cube, and 232-255 are the grayscale ramp.

#### 6.1.4 geometry.rs -- Spatial Types

- `Size { cols: u16, rows: u16 }`: Terminal dimensions.
- `Rect { x: u16, y: u16, width: u16, height: u16 }`: Rectangular region.
- `Position { x: u16, y: u16 }`: Grid position.

All geometry types are `Copy + Clone + Debug + PartialEq + Eq + Hash`.

#### 6.1.5 id.rs -- Entity Identifiers

```rust
pub struct SessionId(pub u64);
pub struct WindowId(pub u64);
pub struct PaneId(pub u64);
```

All ID types are:
- `Copy + Clone + Debug + PartialEq + Eq + Hash + PartialOrd + Ord`
- Generated by `IdGenerator` with monotonically increasing values.
- Display as `$N` format (e.g., `$0`, `$1`).

#### 6.1.6 key.rs -- Input Keys

```rust
pub enum Key {
    Char(char),
    Named(NamedKey),
    Function(u8),
}

pub enum NamedKey {
    Up, Down, Left, Right,
    Home, End, PageUp, PageDown,
    Insert, Delete, Backspace, Tab, Enter, Escape,
}

bitflags! {
    pub struct Modifiers: u8 {
        const SHIFT = 0x01;
        const ALT   = 0x02;
        const CTRL  = 0x04;
        const SUPER = 0x08;
    }
}
```

`KeyEvent` combines a `Key` with `Modifiers`.

#### 6.1.7 error.rs -- Unified Error Type

```rust
pub enum MuxError {
    Kernel(String),
    Config(String),
    Protocol(String),
    Api(String),
    InvalidArgument(String),
    CapacityExceeded { current: usize, limit: usize },
    Io(std::io::Error),
}
```

Implements `std::error::Error + Send + Sync + 'static` (INV-005).

#### 6.1.8 style.rs -- Style Delta Tracking

`StyleDelta` tracks changes between two style states:

```rust
pub struct StyleDelta {
    pub fg_changed: bool,
    pub bg_changed: bool,
    pub attrs_changed: bool,
    pub underline_colour_changed: bool,
    pub new_fg: Colour,
    pub new_bg: Colour,
    pub new_attrs: Attrs,
}
```

`needs_reset()` returns true when transitioning from complex to simple
attributes requires a full SGR reset rather than incremental updates.

---

## 7. Layer 2: Data Structures

### 7.1 mux-grid -- Grid Data Structure

**Purpose**: The core grid buffer that stores terminal cell content. Each
pane has its own grid. The grid supports scrollback history, cursor management,
scroll regions, and line operations.

**Key Type**: `ChunkedGrid`

```rust
pub struct ChunkedGrid {
    lines: Vec<Arc<Vec<Cell>>>,     // Active screen lines
    scrollback: VecDeque<Arc<Vec<Cell>>>,  // History buffer
    cols: u16,
    rows: u16,
    cursor_x: u16,
    cursor_y: u16,
    scroll_top: u16,
    scroll_bottom: u16,
    scrollback_limit: usize,
    dirty: bool,
}
```

**Arc-COW Strategy (SD-02, INV-032)**:

Grid lines are wrapped in `Arc<Vec<Cell>>`. When a snapshot is taken (for
rendering or copy mode), the Arc references are cloned -- not the underlying
vectors. Mutation via `make_line_mut()` uses `Arc::make_mut()` which
performs a copy-on-write only when there are other references:

```rust
fn make_line_mut(&mut self, row: u16) -> &mut Vec<Cell> {
    Arc::make_mut(&mut self.lines[row as usize])
}
```

This provides O(1) snapshot creation (just cloning Arc pointers) while
maintaining mutation safety.

**Operations**:

| Method | Description |
|--------|-------------|
| `new(cols, rows, scrollback_limit)` | Create grid with dimensions |
| `write_char(grapheme_id, width)` | Write character at cursor, advance |
| `put_cell(col, row, cell)` | Set cell at position |
| `cell(col, row) -> &Cell` | Read cell at position |
| `set_cursor(x, y)` | Set cursor position (clamped) |
| `cursor() -> (u16, u16)` | Get cursor position |
| `scroll_up(n)` | Scroll up n lines within scroll region |
| `scroll_down(n)` | Scroll down n lines within scroll region |
| `resize(cols, rows)` | Resize grid, preserving content |
| `clear_all()` | Clear all cells, reset cursor |
| `clear_line(row)` | Clear specific row |
| `erase_in_line(col, row, left)` | Erase left or right of cursor |
| `insert_lines(row, n)` | Insert n blank lines at row |
| `delete_lines(row, n)` | Delete n lines at row |
| `set_scroll_region(top, bottom)` | Set scroll region bounds |
| `reset_scroll_region()` | Reset to full screen |
| `snapshot() -> Vec<Arc<Vec<Cell>>>` | Arc-clone all lines |
| `line_text(row, &GraphemeArena)` | Extract row text |
| `is_dirty() / clear_dirty()` | Dirty tracking |

**Line Type**:

```rust
pub type Line = Arc<Vec<Cell>>;
```

Lines support a `text(&GraphemeArena)` method that extracts the visual text
by looking up each cell's `GraphemeId` in the arena.

### 7.2 mux-parser -- VT Escape Sequence Parser

**Purpose**: Parse the VT100/VT220/xterm escape sequence stream from PTY
output into structured actions that can be applied to a grid.

**Architecture**: Three-layer parser:

1. **State Machine** (`state_machine.rs`): The Paul Williams 14-state automaton
   that classifies bytes and produces raw actions.
2. **CSI Dispatcher** (`csi.rs`): Interprets CSI parameter sequences into
   typed cursor/erase/SGR/mode actions.
3. **Input Parser** (`input.rs`): Applies parsed actions to a `ChunkedGrid`,
   updating cells, cursor, attributes, and scroll state.

See [Section 14](#14-vt-parser-state-machine) for detailed state machine
documentation.

### 7.3 mux-options -- Option Table

**Purpose**: Hierarchical option storage with tmux-compatible scoping.

Options are stored per scope (server, session, window, pane) and resolved
by walking up the scope chain:

```
pane -> window -> session -> server -> default
```

**Key Types**:
- `OptionTable`: A scope-aware option store.
- `OptionValue`: Enum with Bool(bool), Int(i64), Str(String) variants.
- `OptionScope`: Server, Session, Window, Pane.

### 7.4 mux-target -- Target Syntax Parser

**Purpose**: Parse tmux target specifiers like `session:window.pane`.

Target syntax supports:
- Named targets: `dev:editor.0`
- Index targets: `0:1.2`
- Exact match: `=dev`
- ID targets: `$5`
- Relative: `+1`, `-2`
- Special: `!` (last), `+` (next), `-` (previous)

### 7.5 mux-format -- Format String Engine

**Purpose**: Evaluate tmux-compatible format strings like `#S`, `#W`, `#I`.

Supports:
- Variable substitution: `#{session_name}`, `#{window_index}`
- Conditionals: `#{?condition,true,false}`
- Literal text passthrough

### 7.6 mux-cmd-parse -- Command Parser

**Purpose**: Tokenize and parse tmux command strings.

Handles:
- Simple commands: `set -g mouse on`
- Quoted arguments: `bind r "source-file ~/.tmux.conf"`
- Semicolons for command chaining: `set mouse on ; set status on`
- Comments: `# this is a comment`
- Command aliases: `set-option` -> `set`

### 7.7 mux-proto -- Wire Protocol Frames

**Purpose**: Encode and decode TF01 wire protocol frames.

Frame format:
```
Offset  Size  Field
0       4     Magic ("TF01")
4       2     Frame type (u16 LE)
6       2     Flags (u16 LE)
8       4     Payload length (u32 LE)
12      4     CRC32 checksum (u32 LE)
16      N     Payload bytes
```

CRC32 covers: type + flags + payload (not magic or length).

### 7.8 mux-crdt -- CRDT Primitives

**Purpose**: Conflict-free replicated data types for collaborative editing.

**Key Types**:
- `VectorClock`: Maps actor IDs to logical timestamps. Supports `merge()`,
  `increment()`, and `happens_before()`.
- `LwwRegister<T>`: Last-writer-wins register with timestamp-based resolution.

**Properties verified by proptest**:
- `merge(a, b) == merge(b, a)` (commutativity)
- `merge(a, a) == a` (idempotency)
- `merge(merge(a, b), c) == merge(a, merge(b, c))` (associativity)

### 7.9 mux-fdpass -- File Descriptor Passing

**Purpose**: Data model for SCM_RIGHTS file descriptor passing over Unix
domain sockets.

See [Section 18](#18-scm_rights-file-descriptor-passing) for details.

---

## 8. Layer 3: Business Logic

### 8.1 mux-kernel -- Sans-IO Multiplexer Kernel

**Purpose**: The central state machine that manages all multiplexer state.
Processes typed events, produces typed effects. Contains no I/O code.

See [Section 12](#12-sans-io-kernel-architecture) for detailed documentation.

### 8.2 mux-render -- Double-Buffer Diff Renderer

**Purpose**: Compose pane grids into terminal output with minimal escape
sequences.

See [Section 15](#15-double-buffer-diff-rendering) for detailed documentation.

### 8.3 mux-snapshot -- Grid Snapshot

**Purpose**: Capture grid state at a point in time for rendering, copy mode,
or serialization.

**Key Type**: `GridSnapshot`

```rust
pub struct GridSnapshot {
    pub lines: Vec<Line>,   // Arc-cloned lines
    pub size: Size,
    pub cursor: (u16, u16),
}
```

**Methods**:
- `capture(&ChunkedGrid)`: Take snapshot via Arc cloning.
- `text(&GraphemeArena)`: Extract all row text.
- `line_text(row, &GraphemeArena)`: Extract specific row.
- `contains_text(pattern, &GraphemeArena)`: Search for pattern.
- `find_text(pattern, &GraphemeArena)`: Find row containing pattern.

### 8.4 mux-orm -- QuerySet API

**Purpose**: libtmux-inspired fluent filtering for session/window/pane
traversal.

See [Section 19](#19-orm-and-queryset-api) for detailed documentation.

### 8.5 mux-pty -- PTY Management

**Purpose**: PTY allocation, process spawning, signal handling, and raw mode
management. This is one of only two crates that may contain `unsafe` code.

See [Section 21](#21-pty-management) for detailed documentation.

---

## 9. Layer 4: Integration

### 9.1 mux-api -- Public SDK API

**Purpose**: The primary entry point for library users. Provides `ServerBuilder`
and `Server` types for creating and managing terminal sessions.

**Key Types**:
- `ServerBuilder`: Fluent builder for server configuration.
- `Server`: The running server with session/window/pane management.

```rust
let server = ServerBuilder::new()
    .name("my-server")
    .size(Size::new(80, 24))
    .build()?;

let sid = server.create_session("dev")?;
let wid = server.create_window(sid, "editor")?;
let pane_id = server.first_pane(sid)?;
server.feed_pane(pane_id, b"Hello World")?;
```

### 9.2 mux-config -- Configuration Loading

**Purpose**: Parse tmux-compatible configuration files and populate the
option table.

Supports:
- `set -g option value`: Global (server-scope) options.
- `set option value`: Session-scope options.
- `set-option` as alias for `set`.
- `bind key command`: Key bindings (counted but not yet executed).
- Comments, empty lines, mixed content.
- Boolean values: on/off/true/false.
- Integer values, string values.
- Option defaults (escape-time, history-limit, allow-passthrough).

### 9.3 mux-termlet -- Testing Pods

**Purpose**: Lightweight terminal instances for programmatic testing.

See [Section 20](#20-termlet-subsystem) for detailed documentation.

### 9.4 mux-test-support -- Test Harness

**Purpose**: Shared test infrastructure for all crates.

**Key Types**:
- `TestServer`: A pre-configured server with a default session, window, and
  pane. Supports `feed()`, `pane_text()`, `cursor()`.
- `CleanupGuard`: RAII guard that removes a file path on drop.
- `isolated_socket_path()`: Generate unique socket paths for test isolation.

```rust
let mut ts = TestServer::new();
ts.feed(b"Hello World");
assert!(ts.pane_text(0).contains("Hello World"));
assert_eq!(ts.cursor(), (11, 0));
```

---

## 10. Layer 5: External Surface

### 10.1 mux-ffi -- C FFI Surface

**Purpose**: Expose the TermForge API as C-compatible functions for language
bindings.

All FFI functions use `#[unsafe(no_mangle)]` (edition 2024 syntax) and
return error codes rather than panicking. Panic recovery uses `catch_unwind`
(INV-117, requires `panic = "unwind"` in all profiles).

**Functions**:
- `tf_server_new(cols, rows) -> *mut TfServer`
- `tf_server_free(server: *mut TfServer)`
- `tf_session_create(server, name) -> TfSessionId`

### 10.2 mux-otel -- OpenTelemetry Integration

**Purpose**: Cross-crate and cross-service observability via OpenTelemetry.

**Key Types**:
- `OtelSpan`: Wraps an OpenTelemetry span with convenience methods.
- `SpanKind`: Client, Server, Internal, Producer, Consumer.

Provides `add_attribute()`, `end()`, and duration tracking.

---

## 11. Layer 6: Tools

### 11.1 tmux-builder

Build tmux from source at a specified version. Used to create reference
binaries for differential testing.

### 11.2 tmux-vm

tmux version manager. Downloads, builds, and manages multiple tmux versions
for compatibility testing.

### 11.3 tmux-sniff

Wire protocol sniffer. Captures and decodes TF01 frames on Unix domain
sockets for debugging.

### 11.4 mux-doctor

Diagnostic tool. Checks system capabilities (PTY support, signal handling,
terminal features) and reports issues.

---

## 12. Sans-IO Kernel Architecture

### 12.1 Overview

The kernel is the heart of TermForge. It maintains all multiplexer state
(sessions, windows, panes, grids, options) and processes events without
performing any I/O directly.

```
                     +-----------+
  KernelEvent -----> |           | -----> KernelEffect
                     |  Kernel   |
  (typed input)      |           |  (typed output)
                     +-----------+
                          |
                    (pure state mutation)
```

### 12.2 Event Types

```rust
pub enum KernelEvent {
    PtyOutput { pane_id: PaneId, data: Vec<u8> },
    KeyInput { key: KeyEvent },
    Resize { size: Size },
    CreateSession { name: String },
    DestroySession { session_id: SessionId },
    CreateWindow { session_id: SessionId, name: String },
    FeedPane { pane_id: PaneId, data: Vec<u8> },
    // ... more events
}
```

### 12.3 Effect Types

```rust
pub enum KernelEffect {
    Render { pane_id: PaneId },
    SpawnProcess { pane_id: PaneId, command: String },
    SendPtyData { pane_id: PaneId, data: Vec<u8> },
    SessionCreated { session_id: SessionId },
    SessionDestroyed { session_id: SessionId },
    WindowCreated { window_id: WindowId },
    Error { error: MuxError },
    // ... more effects
}
```

### 12.4 State Management

The kernel stores entities in `SlotMap` collections:

```rust
pub struct Kernel {
    sessions: SlotMap<SessionSlotKey, KernelSession>,
    windows: SlotMap<WindowSlotKey, KernelWindow>,
    panes: SlotMap<PaneSlotKey, KernelPane>,
    next_session_id: u64,
    next_window_id: u64,
    next_pane_id: u64,
    grids: HashMap<PaneId, ChunkedGrid>,
    arena: GraphemeArena,
    parser: InputParser,
    // ...
}
```

SlotMap provides O(1) insertion, removal, and lookup with generational
safety -- a removed entity's slot cannot be accidentally reused.

### 12.5 Pane Data Flow

When PTY output arrives for a pane:

1. `KernelEvent::PtyOutput { pane_id, data }` is received.
2. The kernel looks up the pane's grid.
3. The VT parser processes each byte through the state machine.
4. Parser actions (Print, CSI, OSC, etc.) are applied to the grid.
5. The grid's dirty flag is set.
6. A `KernelEffect::Render { pane_id }` is produced.

### 12.6 Session/Window/Pane Hierarchy

```
Server
  +-- Session "dev"
  |     +-- Window "editor" (active)
  |     |     +-- Pane 0 (80x12) [vim]
  |     |     +-- Pane 1 (80x12) [shell]
  |     +-- Window "build"
  |           +-- Pane 0 (80x24) [cargo watch]
  +-- Session "staging"
        +-- Window "deploy"
              +-- Pane 0 (80x24) [ssh]
```

---

## 13. Grid and Cell Model

### 13.1 Memory Layout

Each cell is approximately 16 bytes:

| Field | Type | Size |
|-------|------|------|
| grapheme | GraphemeId (u32) | 4 bytes |
| width | u8 | 1 byte |
| fg | Colour (enum) | 4 bytes |
| bg | Colour (enum) | 4 bytes |
| attrs | Attrs (u8 bitflags) | 1 byte |
| flags | CellFlags (u8 bitflags) | 1 byte |
| link | u16 | 2 bytes |

A typical 80x24 grid has 1,920 cells = ~30 KB of cell data.

### 13.2 Wide Character Handling

CJK characters occupy two cells. When writing a wide character:

1. The first cell gets `width = 2` and `flags.insert(CellFlags::WIDE)`.
2. The second cell gets `width = 0` and `flags.insert(CellFlags::PADDING)`.

When overwriting a wide character, both cells must be cleared.

### 13.3 Scrollback Buffer

The scrollback is a `VecDeque<Arc<Vec<Cell>>>` with a configurable limit.
When a line scrolls off the top of the visible area, it is pushed to the
scrollback. When the scrollback exceeds the limit, the oldest line is dropped.

The Arc wrapper means scrollback lines shared with snapshots are not
duplicated in memory.

---

## 14. VT Parser State Machine

### 14.1 States

The parser has 14 states as defined by Paul Williams' VT parser model:

| State | Description |
|-------|-------------|
| Ground | Normal text input |
| Escape | After ESC (0x1B) |
| EscapeIntermediate | After ESC + intermediate byte |
| CsiEntry | After CSI (ESC[ or 0x9B) |
| CsiParam | Collecting CSI parameters |
| CsiIntermediate | CSI intermediate bytes |
| CsiIgnore | Discarding malformed CSI |
| DcsEntry | After DCS (0x90) |
| DcsParam | Collecting DCS parameters |
| DcsIntermediate | DCS intermediate bytes |
| DcsPassthrough | Passing through DCS data |
| DcsIgnore | Discarding malformed DCS |
| OscString | Collecting OSC string data |
| SosPmApcString | Collecting SOS/PM/APC data |

### 14.2 Byte Classification

The `CLASS_TABLE[256]` provides O(1) byte-to-class mapping:

| Class | Bytes | Description |
|-------|-------|-------------|
| Printable | 0x20-0x7E | Visible characters |
| Control | 0x00-0x1F (except ESC) | C0 control codes |
| Escape | 0x1B | Escape character |
| CsiEntry | 0x9B | 8-bit CSI introducer |
| DcsEntry | 0x90 | 8-bit DCS introducer (INV-026) |
| OscEntry | 0x9D | 8-bit OSC introducer |
| Utf8Lead | 0xC0-0xFD | UTF-8 leading bytes |

### 14.3 Transitions

The state machine processes one byte at a time:

```rust
pub fn step(&mut self, byte: u8) -> Action {
    let class = CLASS_TABLE[byte as usize];
    // Anywhere transitions first
    match byte {
        0x1B => { self.state = State::Escape; return Action::None; }
        0x18 | 0x1A => { self.state = State::Ground; return Action::Execute(byte); }
        _ => {}
    }
    // State-specific transitions
    match (self.state, class) {
        (State::Ground, ByteClass::Printable) => Action::Print(byte),
        (State::Ground, ByteClass::Control) => Action::Execute(byte),
        // ... hundreds more transitions
    }
}
```

### 14.4 CSI Dispatch

CSI sequences follow the pattern `ESC [ <params> <final>`:

| Final | Action | Description |
|-------|--------|-------------|
| A | CUU | Cursor up |
| B | CUD | Cursor down |
| C | CUF | Cursor forward |
| D | CUB | Cursor backward |
| H | CUP | Cursor position |
| J | ED | Erase in display |
| K | EL | Erase in line |
| L | IL | Insert lines |
| M | DL | Delete lines |
| P | DCH | Delete characters |
| S | SU | Scroll up |
| T | SD | Scroll down |
| m | SGR | Select graphic rendition |
| r | DECSTBM | Set scroll region |
| h | SM | Set mode |
| l | RM | Reset mode |

### 14.5 SGR Parameters

SGR (`ESC [ <params> m`) sets text attributes:

| Param | Effect |
|-------|--------|
| 0 | Reset all |
| 1 | Bold |
| 2 | Dim |
| 3 | Italic |
| 4 | Underline |
| 5 | Blink |
| 7 | Reverse |
| 8 | Hidden |
| 9 | Strikethrough |
| 22 | Normal intensity |
| 23 | No italic |
| 24 | No underline |
| 25 | No blink |
| 27 | No reverse |
| 28 | No hidden |
| 29 | No strikethrough |
| 30-37 | Foreground colour |
| 38;5;N | 256-colour foreground |
| 38;2;R;G;B | RGB foreground |
| 39 | Default foreground |
| 40-47 | Background colour |
| 48;5;N | 256-colour background |
| 48;2;R;G;B | RGB background |
| 49 | Default background |

---

## 15. Double-Buffer Diff Rendering

### 15.1 Concept

The renderer maintains two buffers:

```
Previous Frame  ----+
                    |----> diff_buffers() ----> DiffEntry[] ----> encode_diff() ----> bytes
Current Frame   ----+
```

Only cells that differ between frames generate escape sequences. This
minimizes terminal I/O and prevents flicker.

### 15.2 RenderBuffer

```rust
pub struct RenderBuffer {
    cells: Vec<Cell>,
    cols: u16,
    rows: u16,
}
```

Methods: `new()`, `set(col, row, cell)`, `get(col, row)`, `cols()`, `rows()`.

### 15.3 CompositeBuffer

For multi-pane rendering, `CompositeBuffer` maps pane buffers into a single
output buffer:

```rust
pub struct CompositeBuffer {
    buffer: RenderBuffer,
    panes: Vec<PaneRegion>,
}

pub struct PaneRegion {
    pub x: u16,
    pub y: u16,
    pub buffer: RenderBuffer,
}
```

### 15.4 Diff Algorithm

`diff_buffers(&prev, &next)` compares cell by cell:

```rust
pub struct DiffEntry {
    pub col: u16,
    pub row: u16,
    pub cell: Cell,
}
```

A cell is considered changed if any field differs: grapheme, width, fg, bg,
attrs, flags, or link.

### 15.5 Escape Sequence Encoding

`encode_diff(entries, &arena)` produces minimal bytes:

1. Sort entries by row, then column.
2. For each entry:
   a. If position changed, emit CUP: `ESC [ row+1 ; col+1 H`
   b. If attributes changed, emit SGR sequences.
   c. If foreground changed, emit fg colour escape.
   d. If background changed, emit bg colour escape.
   e. Emit the character (looked up from arena by GraphemeId).

### 15.6 Flood Fairness (SD-13)

When multiple panes have pending output, the renderer uses stride rotation
to prevent a single pane from monopolizing the output:

1. Each pane has a per-frame row quota.
2. Panes are served in rotation order.
3. If a pane exhausts its quota, it is skipped until the next frame.

---

## 16. CRDT and Collaboration

### 16.1 Vector Clock

```rust
pub struct VectorClock {
    clocks: HashMap<u64, u64>,
}
```

Operations:
- `increment(actor_id)`: Advance the clock for an actor.
- `merge(other)`: Take the component-wise maximum.
- `happens_before(other) -> bool`: Partial ordering.

### 16.2 Last-Writer-Wins Register

```rust
pub struct LwwRegister<T> {
    value: T,
    timestamp: u64,
    actor_id: u64,
}
```

On merge, the register with the higher timestamp wins. Ties are broken by
actor_id (higher wins).

### 16.3 Convergence Property

All CRDT operations are verified by proptest to converge: applying the same
set of operations in any order produces the same final state.

---

## 17. Wire Protocol (TF01)

### 17.1 Frame Format

```
+-------+------+-------+----------+--------+---------+
| Magic | Type | Flags | PayloadLen | CRC32  | Payload |
| 4B    | 2B   | 2B    | 4B        | 4B     | NB      |
+-------+------+-------+----------+--------+---------+
```

- **Magic**: `"TF01"` (4 bytes ASCII).
- **Type**: Frame type identifier (u16 LE).
- **Flags**: Bitfield for compression, encryption, etc. (u16 LE).
- **PayloadLen**: Length of payload in bytes (u32 LE).
- **CRC32**: CRC32 of (type + flags + payload) (u32 LE).
- **Payload**: Variable-length payload bytes.

### 17.2 Frame Types

| Type | Name | Description |
|------|------|-------------|
| 0x0001 | Data | PTY output data |
| 0x0002 | Input | Key/mouse input |
| 0x0003 | Resize | Terminal resize |
| 0x0004 | Control | Control command |
| 0x0005 | Ping | Keepalive |
| 0x0006 | Pong | Keepalive response |

### 17.3 tmux Wire Compatibility

The native TF01 protocol is used for Rust-to-Rust communication. For tmux
compatibility, a separate adapter translates between tmux's wire format and
TF01 frames.

---

## 18. SCM_RIGHTS File Descriptor Passing

### 18.1 Purpose

Unix domain sockets support passing file descriptors between processes via
`sendmsg`/`recvmsg` with `SCM_RIGHTS` control messages. This is how tmux
hands PTY master fds to clients.

### 18.2 FdEnvelope

```rust
pub struct FdEnvelope {
    entries: Vec<FdEntry>,  // max 16 (SD-11)
    payload: Vec<u8>,
}

pub struct FdEntry {
    raw_fd: i32,
    fd_type: FdType,  // PtyMaster, PtySlave, UnixSocket, Generic
    label: String,
}
```

### 18.3 Serialization

The envelope serializes to bytes for transport alongside the fd array:

```
[count: u8]
[entry 0: fd(4 LE) + type(1) + label_len(1) + label(N)]
[entry 1: ...]
...
[payload bytes]
```

### 18.4 Limits

- Maximum 16 fds per envelope (matches kernel SCM_RIGHTS limits).
- FdType tag values: Generic=0, PtyMaster=1, PtySlave=2, UnixSocket=3.
- Invalid tags produce `FdPassError::InvalidType`.

---

## 19. ORM and QuerySet API

### 19.1 Design

Inspired by Django's QuerySet and Python's libtmux, the ORM provides fluent
traversal of the session/window/pane graph:

```rust
let sessions = server.sessions()
    .filter(|s| s.name.contains("dev"))
    .first()?;

let panes = server.panes()
    .filter(|p| p.session_name == "dev")
    .filter(|p| p.width > 80)
    .count();
```

### 19.2 QuerySet Type

```rust
pub struct QuerySet<T> {
    items: Vec<T>,
}

impl<T> QuerySet<T> {
    pub fn filter<F: Fn(&T) -> bool>(self, f: F) -> Self;
    pub fn first(&self) -> Option<&T>;
    pub fn get(&self) -> Result<&T, QueryError>;
    pub fn count(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn iter(&self) -> impl Iterator<Item = &T>;
    pub fn contains<F: Fn(&T) -> bool>(&self, f: F) -> bool;
}
```

### 19.3 Error Types

```rust
pub enum QueryError {
    NotFound,
    MultipleResults(usize),
}
```

`get()` returns `NotFound` if the filtered set is empty, `MultipleResults(n)`
if it contains more than one item.

---

## 20. Termlet Subsystem

### 20.1 Concept

Termlets are lightweight, self-contained terminal instances designed for
programmatic testing. They are the successor to raw tmux panes in testing
scenarios:

- **Visual capture**: Take snapshots of terminal content at any time.
- **Resize**: Dynamically change dimensions.
- **Interact**: Send keystrokes, paste text, execute commands.
- **Control**: Start, stop, and monitor child processes.
- **Cross-language**: Available via Rust, Node.js, and Python bindings.

### 20.2 State Machine

```
Created --start()--> Running --stop()--> Stopped
                       |
                   feed()/resize()
```

### 20.3 TermletBuilder

```rust
let termlet = TermletBuilder::new()
    .size(Size::new(80, 24))
    .build()?;
```

### 20.4 API

| Method | State | Description |
|--------|-------|-------------|
| `start()` | Created -> Running | Initialize grid and parser |
| `feed(data)` | Running | Send bytes to VT parser |
| `resize(size)` | Running | Change terminal dimensions |
| `snapshot()` | Running | Capture current grid state |
| `pane_text(row)` | Running | Extract text from a row |
| `cursor()` | Running | Get cursor position |
| `stop()` | Running -> Stopped | Finalize termlet |

---

## 21. PTY Management

### 21.1 Architecture

`mux-pty` is the platform abstraction layer. It is one of only two crates
allowed to contain `unsafe` code (the other being `mux-ffi`).

### 21.2 PtyPair

```rust
pub struct PtyPair {
    pub master: RawFd,
    pub slave: RawFd,
}
```

### 21.3 SpawnContext

```rust
pub struct SpawnContext {
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub cwd: Option<PathBuf>,
    pub size: Size,
}
```

### 21.4 PtyError

```rust
pub enum PtyError {
    Io(std::io::Error),
    InvalidSize(String),
    SpawnFailed(String),
}
```

### 21.5 Signal Handling

`mux-pty` uses `signal-hook` for:
- SIGWINCH: Propagated to all active panes via TIOCSWINSZ.
- SIGCHLD: Reaps zombie children.
- SIGTERM/SIGINT: Graceful shutdown.

### 21.6 Raw Mode

The `RawModeGuard` RAII pattern saves and restores terminal state:

```rust
let _guard = RawModeGuard::enter()?;
// Terminal is in raw mode
// Guard restores on drop
```

Three modes are supported:
- **Managed**: TermForge owns the transition.
- **External**: Caller manages raw mode.
- **SaveRestore**: Save on entry, restore on exit.

---

## 22. Configuration System

### 22.1 Loading

Configuration is loaded from tmux-compatible files:

```
# ~/.tmux.conf
set -g mouse on
set -g history-limit 10000
set -g default-terminal "screen-256color"
bind r source-file ~/.tmux.conf
```

### 22.2 Option Resolution

Options are resolved by walking up the scope chain:

```
pane[key] -> window[key] -> session[key] -> server[key] -> default[key]
```

The most specific scope wins.

### 22.3 Defaults

| Option | Default | Type |
|--------|---------|------|
| escape-time | 500 | int |
| history-limit | 2000 | int |
| mouse | false | bool |
| status | true | bool |
| allow-passthrough | false (INV-220) | bool |

---

## 23. Layout Engine

### 23.1 Layout Tree

Pane layout is a binary tree:

```rust
pub enum LayoutNode {
    Leaf { pane_id: PaneId },
    HSplit { left: Box<LayoutNode>, right: Box<LayoutNode>, ratio: f32 },
    VSplit { top: Box<LayoutNode>, bottom: Box<LayoutNode>, ratio: f32 },
}
```

### 23.2 Operations

- `distribute_evenly(total, count)`: Split space equally with rounding.
- `find_pane(pane_id)`: Search the tree for a pane.
- `all_panes()`: Collect all pane IDs.

### 23.3 Resize Propagation

When the terminal is resized:
1. The root layout node receives the new dimensions.
2. Each split distributes space according to its ratio.
3. Leaf nodes produce TIOCSWINSZ ioctls for their panes.

---

## 24. Copy Mode

### 24.1 State

```rust
pub struct CopyModeState {
    pub active: bool,
    pub cursor_x: u16,
    pub cursor_y: u16,
    pub anchor_x: Option<u16>,
    pub anchor_y: Option<u16>,
    pub scroll_offset: u16,
    pub search_pattern: Option<String>,
    pub key_table: String,
}
```

### 24.2 Operations

- `enter()`: Activate copy mode.
- `exit()`: Deactivate and return to normal mode.
- `begin_selection()`: Set anchor at current cursor.
- `cursor_up/down/left/right()`: Move cursor (clamped to grid bounds).
- `selection_range()`: Get ordered (start, end) of selection.
- `search(pattern)`: Search in scrollback.

### 24.3 Paste Buffer Ring

```rust
pub struct PasteBufferRing {
    buffers: VecDeque<String>,
    capacity: usize,
}
```

FIFO eviction: when capacity is exceeded, the oldest buffer is dropped.

---

## 25. Snapshot System

### 25.1 Capture

Snapshots are taken by Arc-cloning grid lines:

```rust
let snap = GridSnapshot::capture(&grid);
// snap.lines are Arc references to the same data
// Modifying the grid does not affect the snapshot
```

### 25.2 Text Extraction

```rust
let text = snap.text(&arena);  // Vec<String>, one per row
let line = snap.line_text(0, &arena);  // Single row
let found = snap.contains_text("error", &arena);  // Search
```

### 25.3 Isolation Property

Snapshots are isolated from subsequent grid mutations:

```rust
let snap = GridSnapshot::capture(&grid);
grid.clear_all();  // Does not affect snap
assert!(!snap.lines.is_empty());
```

This is guaranteed by the Arc-COW strategy: the snapshot holds its own Arc
references, and grid mutations create new Arc instances via `Arc::make_mut()`.

---

## 26. OpenTelemetry Observability

### 26.1 Architecture

`mux-otel` provides a thin wrapper over OpenTelemetry for consistent span
creation across crates.

### 26.2 Span Types

| Kind | Use Case |
|------|----------|
| Internal | Intra-process operations |
| Client | Outgoing requests (e.g., to PTY) |
| Server | Incoming requests (e.g., from client) |
| Producer | Async message production |
| Consumer | Async message consumption |

### 26.3 Attributes

Spans carry structured attributes:

```rust
let mut span = OtelSpan::new("kernel.process_event", SpanKind::Internal);
span.add_attribute("event_type", "pty_output");
span.add_attribute("pane_id", "42");
// ... do work ...
span.end();
```

---

## 27. FFI and Language Bindings

### 27.1 C FFI

`mux-ffi` exports C-compatible functions:

```c
// Create a server
TfServer* server = tf_server_new(80, 24);

// Create a session
TfSessionId sid = tf_session_create(server, "dev");

// Cleanup
tf_server_free(server);
```

### 27.2 Panic Recovery

All FFI functions wrap their body in `catch_unwind`:

```rust
#[unsafe(no_mangle)]
pub extern "C" fn tf_server_new(cols: u16, rows: u16) -> *mut TfServer {
    std::panic::catch_unwind(|| {
        // ... create server ...
    }).unwrap_or(std::ptr::null_mut())
}
```

This requires `panic = "unwind"` in all Cargo profiles (INV-117).

### 27.3 Python Bindings (Planned)

Via PyO3, exposing:
- `Server`, `Session`, `Window`, `Pane` classes.
- `Termlet` for pytest fixtures.
- QuerySet API with Python iteration protocol.

### 27.4 Node.js Bindings (Planned)

Via napi-rs, exposing:
- Same API as Python bindings.
- Vitest integration for snapshot testing.
- Async operations via Node.js event loop.

---

## 28. Testing Strategy

### 28.1 Test Pyramid

```
         /  Differential  \     (vs. tmux binary)
        /   Integration    \    (cross-crate)
       /     Property       \   (proptest)
      /       Unit           \  (per-module)
     /________________________\
```

### 28.2 Test Types

| Type | Tool | Count Target |
|------|------|-------------|
| Unit tests | `#[test]` | 700+ |
| Property tests | `proptest!` | 130+ |
| Integration tests | `tests/*.rs` | 36+ |
| Snapshot tests | `insta` | TBD |
| Fuzz tests | `cargo-fuzz` | 5+ targets (future) |
| Differential | tmux binary | Per-command (future) |

### 28.3 Socket Isolation

Every test that creates sockets uses `isolated_socket_path()`:

```rust
pub fn isolated_socket_path() -> PathBuf {
    let id = uuid::Uuid::new_v4();
    std::env::temp_dir().join(format!("termforge-test-{id}"))
}
```

This ensures tests never conflict with:
- The user's live tmux server.
- Other tests running in parallel.
- Previous test runs that did not clean up.

### 28.4 Cleanup

`CleanupGuard` ensures resources are removed even on test failure:

```rust
let path = isolated_socket_path();
let _guard = CleanupGuard::new(path.clone());
// ... test ...
// path is removed when _guard drops
```

### 28.5 Property Test Coverage

Every crate with encoding/decoding has proptest coverage:

| Crate | Properties Tested |
|-------|------------------|
| mux-types | Colour pack/unpack, Attrs union/intersection, Geometry area/contains |
| mux-grid | Resize preserves content, cursor bounds, write/read roundtrip |
| mux-parser | State determinism, ESC/cancel anywhere, ground printable |
| mux-proto | Encode/decode roundtrip, CRC corruption detection |
| mux-crdt | Merge commutativity/idempotency/associativity |
| mux-render | Identical buffers no diff, single cell diff |
| mux-orm | Filter composition, chaining |
| mux-kernel | Create session effects, distribute_evenly |
| mux-termlet | Size, lifecycle, feed data |

---

## 29. Error Handling Strategy

### 29.1 Principles

1. **No panics in library code** (INV-006).
2. **All errors implement Error + Send + Sync** (INV-005).
3. **Use `thiserror`** for derive-based error implementations.
4. **Use `anyhow`** only in binaries and test support.
5. **Return Result** from all fallible operations.

### 29.2 Error Hierarchy

```
MuxError (mux-types)
  +-- Kernel(String)
  +-- Config(String)
  +-- Protocol(String)
  +-- Api(String)
  +-- InvalidArgument(String)
  +-- CapacityExceeded { current, limit }
  +-- Io(std::io::Error)

Per-crate errors:
  PtyError (mux-pty)
  FdPassError (mux-fdpass)
  TargetError (mux-target)
  ConfigError (mux-config)
  QueryError (mux-orm)
  FrameError (mux-proto)
```

### 29.3 Error Context

Errors carry enough context for debugging:

```rust
#[error("too many fds: {0} exceeds limit of {MAX_FDS}")]
TooManyFds(usize),
```

---

## 30. Performance Considerations

### 30.1 Hot Paths

| Path | Optimization |
|------|-------------|
| VT byte classification | CLASS_TABLE[256] lookup (O(1)) |
| Grid line cloning | Arc-COW (zero-copy on read) |
| Render diff | Cell-by-cell comparison |
| GraphemeId lookup | Arena index (O(1)) |
| Entity lookup | SlotMap (O(1) with generation check) |

### 30.2 Memory

| Structure | Per-Instance |
|-----------|-------------|
| Cell | ~16 bytes |
| Grid 80x24 | ~30 KB |
| Grid 200x50 | ~160 KB |
| Scrollback (10000 lines) | ~12 MB (80 cols) |
| GraphemeArena (1000 entries) | ~8 KB |

### 30.3 Channel Bounds

Bounded channels prevent unbounded memory growth:

| Channel | Bound | Rationale |
|---------|-------|-----------|
| control | 512 | Commands are small, infrequent |
| data | 1024 | PTY output can burst |
| render | 256 | Frame-rate limited |
| effect | 1024 | Effects may batch |

---

## 31. Platform Abstraction

### 31.1 Strategy

Platform-specific code is isolated in `mux-pty`. All other crates are
platform-independent.

### 31.2 Linux

- PTY allocation: `posix_openpt` + `grantpt` + `unlockpt` + `ptsname`.
- Signal handling: `signal-hook` with `tokio` integration.
- Resize: `TIOCSWINSZ` ioctl.
- Raw mode: `tcgetattr` / `tcsetattr` with `cfmakeraw`.

### 31.3 macOS

- Same POSIX PTY API.
- `cfmakeraw` is a glibc extension but available on macOS.
- No `TIOCGPTPEER` (use `ptsname` + `open`).

### 31.4 WASM

- No PTY support.
- Grid and parser crates work in WASM.
- Resize is a no-op.

---

## 32. Terminal Mode Policy

Three modes for managing the host terminal's raw mode state (Gap-2 resolution):

### 32.1 Managed Mode

TermForge owns raw mode transitions. Uses `RawModeGuard` RAII:

```rust
let _guard = RawModeGuard::enter()?;
```

The guard restores terminal state on drop.

### 32.2 External Mode

The caller has already set up raw mode. TermForge does not touch termios.
Used when embedding TermForge in another terminal application.

### 32.3 SaveRestore Mode

TermForge saves the current termios state on startup and restores it on
shutdown. This is the safest option for standalone use.

---

## 33. Flood Fairness

### 33.1 Problem

A single pane producing high-volume output (e.g., `cat /dev/urandom`) can
starve other panes of rendering time.

### 33.2 Solution (SD-13)

Per-pane row quota with stride rotation:

1. Each frame has a total row budget.
2. The budget is divided among visible panes.
3. Panes are served in rotation order (not always starting from pane 0).
4. A pane that exhausts its quota is deferred to the next frame.

### 33.3 Configuration

The row budget is configurable via the `render-budget` option (default: 256
rows per frame).

---

## 34. Signal Handling

### 34.1 SIGWINCH Propagation

When the outer terminal resizes:

1. `signal-hook` delivers SIGWINCH to the TermForge process.
2. The signal handler queries the new terminal size via `TIOCGWINSZ`.
3. A `KernelEvent::Resize` is dispatched to the kernel.
4. The kernel recalculates layout for all windows.
5. Each pane's grid is resized.
6. `TIOCSWINSZ` is sent to each pane's PTY slave.
7. `SIGWINCH` is sent to each pane's child process group.

### 34.2 SIGCHLD Handling

When a child process exits:

1. `signal-hook` delivers SIGCHLD.
2. The handler calls `waitpid(-1, WNOHANG)` in a loop to reap all zombies.
3. For each reaped child, a `KernelEvent::PaneExited` is dispatched.
4. The kernel marks the pane as dead and optionally respawns.

### 34.3 SIGTERM/SIGINT

Graceful shutdown:

1. Signal is caught by `signal-hook`.
2. A shutdown event is dispatched.
3. All PTYs are closed.
4. Terminal mode is restored (via `RawModeGuard` drop).

---

## 35. Gap Resolutions

### Gap-1: SIGWINCH Propagation

**Problem**: The v0004 spec did not define the signal flow when the outer
terminal resizes.

**Resolution**: Defined a 7-step propagation chain (see Section 34.1).
`signal-hook` delivers the signal, the handler queries the new size, and the
kernel propagates it to all panes.

### Gap-2: Raw Mode Lifecycle

**Problem**: Who owns the raw mode transition? What about library embedding?

**Resolution**: Three-mode policy (Managed/External/SaveRestore) with
`RawModeGuard` RAII (see Section 32).

### Gap-3: Rendering Composition

**Problem**: No explicit strategy for compositing multiple pane buffers into
terminal output.

**Resolution**: Double-buffer diff rendering with `RenderBuffer` and
`CompositeBuffer` (see Section 15). This is the correct approach for
terminal-to-terminal rendering (vs. GPU rendering).

### Gap-4: Layout Engine

**Problem**: No layout algorithm defined.

**Resolution**: Binary tree `LayoutNode` with HSplit/VSplit and recursive
`distribute_evenly()` (see Section 23).

### Gap-5: Threading Model

**Problem**: Not defined whether single-threaded, multi-threaded, or async.

**Resolution**: Tokio async with sans-IO kernel. The kernel itself is
single-threaded (no `Arc<Mutex<>>`), but I/O operations (PTY read/write,
client connections) are async tokio tasks that communicate with the kernel
via bounded channels.

### Gap-6: Zombie Reaping

**Problem**: Child process cleanup not defined.

**Resolution**: `signal-hook` SIGCHLD handler with `waitpid(-1, WNOHANG)`
loop in `mux-pty` (see Section 34.2).

### Gap-7: SCM_RIGHTS

**Problem**: File descriptor passing not supported.

**Resolution**: `FdEnvelope` in `mux-fdpass` with 16-fd cap and typed fd
classification (see Section 18).

### Gap-8: SDK Ergonomics

**Problem**: API was server-daemon oriented, not library-friendly.

**Resolution**: `ServerBuilder` fluent API in `mux-api` with `TestServer`
convenience wrapper in `mux-test-support` (see Section 9.1).

### Gap-9: Error Recovery

**Problem**: Runtime failure modes not handled.

**Resolution**: Typed errors in every crate, `catch_unwind` at FFI
boundaries, `CleanupGuard` for resource cleanup (see Section 29).

---

## 36. Invariant Catalog

See AGENTS.md for the complete invariant catalog (INV-001 through INV-034,
plus extended invariants INV-117, INV-119, INV-220).

---

## 37. Settled Decisions

| ID | Decision | Rationale |
|----|----------|-----------|
| SD-01 | GraphemeArena interning | Per-cell String too expensive for millions of cells |
| SD-02 | Arc<Vec<Cell>> for lines | Zero-copy snapshot isolation |
| SD-03 | CRC32C for checksums | Hardware-accelerable, fast |
| SD-04 | Paul Williams 14-state machine | Industry standard, proven correct |
| SD-05 | Sans-IO kernel | Deterministic, testable, replayable |
| SD-06 | Double-buffer diff rendering | Minimal escape sequences for terminal output |
| SD-07 | Binary tree layout | Recursive nesting, natural for split panes |
| SD-08 | SlotMap entity storage | O(1) with generational safety |
| SD-09 | crossbeam-channel | Bounded, high-performance MPSC |
| SD-10 | signal-hook for signals | Ecosystem standard (127M downloads) |
| SD-11 | 16-fd FdEnvelope cap | Matches kernel SCM_RIGHTS limits |
| SD-12 | RawModeGuard RAII | Automatic terminal state restore |
| SD-13 | Stride rotation fairness | Prevents pane output starvation |
| SD-14 | VectorClock behind feature gate | CRDT is optional overhead |
| SD-15 | winnow for commands | Zero-copy parser combinators |
| SD-16 | Edition 2024 | Latest stable edition, gen reserved |
| SD-17 | thiserror for errors | Derive-based, zero boilerplate |
| SD-18 | proptest for properties | Algebraic property verification |

---

## 38. Future Considerations

### 38.1 Not Yet Implemented

- **Full tmux command set**: Only `set`, `set-option`, and `bind` are parsed.
- **tmux wire protocol v8**: Frame model exists, full protocol not implemented.
- **Real PTY spawning**: `mux-pty` has the model but not the syscalls.
- **Session/window/pane model in kernel**: Currently stubs in separate files.
- **WASM target**: L0 crates are designed for it but not tested.
- **Benchmarks**: No criterion benchmarks yet.
- **Fuzz targets**: No cargo-fuzz targets yet.
- **Differential tests**: No tmux binary comparison yet.

### 38.2 Evaluated and Deferred

- **crossterm**: The dominant terminal I/O crate (73M downloads) was evaluated.
  TermForge uses `nix`/`signal-hook` directly for lower-level control needed
  by a multiplexer. crossterm is designed for application-level terminal I/O,
  not multiplexer internals.
- **termwiz Surface/Change log**: WezTerm's approach is well-suited for
  multiplexer state sync. Deferred to evaluate if the double-buffer diff
  approach is sufficient for TermForge's use cases.
- **vt100 crate**: Zellij's choice with built-in `contents_diff()`. The
  TermForge approach builds the diff engine into `mux-render` for more
  control over the rendering pipeline.
- **Sixel/iTerm2/Kitty graphics**: Acknowledged as future work. The DCS
  passthrough mechanism in the parser is the foundation.
- **WASM plugin model**: Zellij's WASM plugin panes are interesting but
  add significant complexity. Deferred.
- **Ion Shell**: Embeddable Rust shell evaluated as a potential base for
  the built-in shell. Deferred -- the built-in shell scope is not yet defined.

### 38.3 Known Limitations

- `mux-kernel/src/pane.rs`, `session.rs`, `window.rs` are stubs.
- The VT parser handles common sequences but not all xterm extensions.
- No actual PTY allocation (syscalls not implemented in mux-pty).
- No actual signal handling (signal-hook integration not wired).
- No actual async runtime (tokio not wired in mux-api).
