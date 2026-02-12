# TermForge Architecture Reference

## Overview

TermForge is a Rust terminal multiplexer designed for 100% tmux wire-protocol v8 compatibility
while providing a clean, layered architecture suitable for ORM-like APIs, CRDT collaboration,
language bindings, and Termlet-based testing.

## Crate Dependency Layers

```
L0 (no internal deps)       L1 (depends on L0)          L2 (depends on L0+L1)
+-----------------+         +-----------------+          +-----------------+
| mux-types       |<--------| mux-grid        |<---------| mux-termlet     |
| mux-parser      |         | mux-snapshot    |          +-----------------+
| mux-grapheme-   |<--------| mux-pty         |
|   arena         |         | mux-proto       |
| mux-kernel      |<-----+  | mux-time        |
| mux-orm         |      |  +-----------------+
| mux-test-support|      |
+-----------------+      +-- depends on mux-types,
                             mux-grid, mux-time
```

## L0 Crates (Zero Internal Dependencies)

### mux-types
Core type definitions shared across the entire workspace.

- **Cell**: A single terminal cell holding a `CompactString` grapheme and a `Style`.
  - Width is explicit (0, 1, or 2) via `CellWidth` enum (INV-029).
  - No inference at render time.

- **PackedCell**: 64-bit packed representation.
  - Layout: scalar:21 + style:15 + flags:12 + width:2 + ext:14 = 64 bits (S94).
  - Verified by compile-time const assertion.
  - `pack()` and `unpack()` are infallible round-trip operations.

- **Line**: A row of cells backed by `Arc<Vec<Cell>>`.
  - COW semantics: clone-on-write for scrollback sharing (S96, INV-032).
  - `to_text()` for snapshot comparison.

- **Style**: Visual style with Color enum (Default/Indexed/Rgb), Attrs u16 bitfield (INV-009).
  - `pack_index()` produces a 15-bit compact index for PackedCell storage.

- **TermletError**: Exactly 16 error variants (S98) with error codes 8001-8016.
  - Implements `std::error::Error + Send + Sync + 'static` (INV-005).
  - `is_terminal()` distinguishes recoverable from fatal errors.

- **IdentityManifest**: Project identity and feature metadata (S01).

### mux-parser
VT100/VT220 escape sequence parser.

- **14-state state machine**: Ground, Escape, EscapeIntermediate, CsiEntry, CsiParam,
  CsiIntermediate, CsiIgnore, DcsEntry, DcsParam, DcsIntermediate, DcsPassthrough,
  DcsIgnore, OscString, SosPmApcString.

- **CLASS_TABLE[256]**: Static lookup table for byte classification. Built at compile time
  via `const fn build_table()` in the `byte_class` module.

- **ByteClass**: 7 variants including DcsEntry (INV-026). The `classify()` function uses
  the precomputed table for O(1) lookup. A `classify_match()` oracle exists for testing.

- **Action enum**: Print, Execute, Hook, Put, Unhook, OscStart, OscPut, OscEnd, Param,
  Collect, EscDispatch, CsiDispatch, Clear, Ignore.

### mux-grapheme-arena
Arena allocator for extended grapheme clusters.

- 14-bit ext index space (0x0001..0x3FFF). Index 0x0000 is reserved (INV-030).
- Deduplication: identical graphemes share the same slot.
- `encode()`/`decode()` for binary serialization with length-prefixed entries.

### mux-kernel
Sans-IO kernel: pure reducer pattern.

- **GenSlotMap<T>**: Generational slot map for O(1) access with ABA prevention.
  - Each slot has a generation counter. Stale keys are safely rejected.

- **ServerGraph**: Hierarchical session -> window -> pane structure.
  - Three GenSlotMaps: sessions, windows, panes.
  - `create_session()`, `create_window()`, `create_pane()` maintain parent-child links.

- **Event enum**: All external inputs (ClientInput, PtyOutput, PtyExited, Tick, Resize,
  CreateSession, CreateWindow, CreatePane).

- **Effect enum**: All output requests (WritePty, SendClient, SpawnPty, ResizePty,
  ClosePty, Log, SessionCreated, WindowCreated, PaneCreated).

- **Kernel**: `step(event) -> Vec<Effect>` pure function (INV-027).
  - No `std::io`, no `std::net`, no side effects.
  - Deterministic under fixed clock injection (INV-033).

### mux-orm
Django-inspired ORM-like query interface.

- **QueryList<T>**: Collection wrapper with `filter()`, `get()`, `first()`, `count()`.
- **Queryable trait**: Types that can be queried by name/id.
- Designed for traversing the ServerGraph: `server.sessions().filter(|s| ...)`.

### mux-test-support
Test harness utilities.

- **TestGuard**: RAII cleanup guard. Removes socket files on drop. Runs registered
  cleanup callbacks in reverse order.
- **test_socket_name()**: Generates unique socket paths under `/tmp/termforge-test-*`
  to avoid conflicts with live tmux servers.
- **DiffResult/DiffReport**: Differential testing against tmux. Compares byte-level
  output with first-diff-byte reporting and pass rate statistics.
- **GridSnapshot**: Captured grid state for snapshot comparison. `snapshot_diff()`
  produces human-readable diffs with row,col coordinates.

## L1 Crates

### mux-grid
Terminal grid with scrollback.

- **Grid**: rows x cols grid of Cells. Zero-based coordinates (INV-007).
  - Mutation only via `put_char()` / `put_grapheme()` (INV-012).
  - `scroll_up()` / `scroll_down()` manage scrollback buffer.
  - `resize()` handles terminal resize.
  - `clear()` resets all cells.

- **Scrollback**: Separate module using `VecDeque<Line>`.
  - Configurable limit (default 10,000 lines).
  - `search()` for text search across scrollback history.

### mux-snapshot
Binary snapshot format.

- **31-byte header** (INV-023): magic bytes, version, dimensions, cursor position,
  arena size, data length, CRC32C checksum.
- **CRC32C** via `crc32fast` (S93, INV-031).
- `encode()` / `decode()` with error handling for corrupt snapshots.

### mux-pty
PTY handle with typestate lifecycle.

- **7 states** (S99): Created, Opening, Open, Suspended, Resuming, Closing, Closed.
- **Typestate pattern**: Invalid transitions are compile-time errors (e.g., cannot
  write to a Closed PTY).
- **DynPtyHandle**: Runtime state machine for when static typing is impractical.
- `#![forbid(unsafe_code)]` in scaffold; production will use `unsafe` for PTY syscalls.

### mux-proto
Wire protocol v8 TLV frames.

- **FrameType enum**: 9 frame types (Handshake, HandshakeAck, Data, Resize, Command,
  CommandResponse, Heartbeat, HeartbeatAck, Error).
- **TLV encoding**: 1-byte type + 4-byte length + payload.
- `encode_frame()` / `decode_frame()` with length validation.

### mux-time
Deterministic time source.

- **LamportClock**: Logical clock with `tick()` and `receive()`.
- **VectorClock** (crdt feature): Causal ordering for distributed events (INV-025).
  - `increment()`, `merge()`, `dominates()`, `concurrent_with()`.
- **DeterministicTimeSource**: Combines wall clock + Lamport + optional VectorClock (S97).
  - `deterministic()` constructor for testing with predictable timestamps (INV-033).
- **MonotonicGuard**: Ensures timestamps never decrease.

## L2 Crates

### mux-termlet
Termlet testing runtime.

- **TermletConfig**: Shell command, dimensions, environment, timeout.
- **TermletBuilder**: Fluent builder pattern for constructing Termlets.
- **Termlet**: Lifecycle management (spawn, send_keys, capture, resize, kill).
- **TermletState**: Running, Paused, Exited.
- Will integrate with language bindings (Python/Node.js) for cross-language snapshot testing.

## Feature Gates

### crdt
Enables CRDT-related types for collaborative/distributed scenarios.

- `VectorClock` struct and impl in mux-time
- `DeterministicTimeSource.vector` field
- `DeterministicTimeSource.receive()` method (takes VectorClock parameter)
- When disabled, only `receive_lamport()` (Lamport-only merge) is available.

## Key Design Decisions

| ID | Decision | Rationale |
|----|----------|-----------|
| S75 | No SmallVec for cells | Simplicity; Vec<Cell> is fast enough with Arc COW |
| S82 | Typestate for PTY | Compile-time safety for lifecycle transitions |
| S91 | CompactString for graphemes | Small string optimization; no heap for ASCII |
| S93 | CRC32C for checksums | Hardware-accelerated on modern CPUs |
| S94 | PackedCell = 64 bits | Cache-line friendly; fits in a register |
| S95 | CLASS_TABLE[256] | O(1) byte classification |
| S96 | Arc COW scrollback | Share lines between grid and scrollback cheaply |
| S97 | Combined time source | Wall + Lamport + VectorClock in one struct |
| S98 | 16 TermletError variants | Covers all Termlet lifecycle failures |
| S99 | 7 PTY states | Minimal state machine for full lifecycle |
| S100 | Deterministic APIs | Reproducible behavior under test |
