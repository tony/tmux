# TermForge Architecture -- 13-Layer Comprehensive Design

## Layer 1: Identity & Project Structure

### Project Identity
- Name: TermForge
- License: MIT OR Apache-2.0
- Rust edition: 2024, MSRV: 1.85
- Workspace: 19 crates + 4 tools + 2 binding stubs

### Crate Naming Convention
All crates use the `mux-` prefix. Crate names map to Rust module names
by replacing hyphens with underscores (e.g., `mux-grid` -> `mux_grid`).

### Dependency Layer Hierarchy
```
L0: mux-grapheme-arena, mux-time              (zero internal deps)
L1: mux-types                                  (depends on L0)
L2: mux-grid, mux-parser, mux-proto,          (depends on L0+L1)
    mux-options, mux-target, mux-format
L3: mux-snapshot, mux-cmd-parse, mux-orm,     (depends on L0-L2)
    mux-render, mux-pty
L4: mux-kernel, mux-termlet                   (depends on L0-L3)
L5: mux-test-support, mux-ffi, mux-otel       (depends on L0-L4)
L6: tools/tmux-builder, tools/tmux-vm,        (binary targets)
    tools/tmux-sniff, tools/mux-doctor
```

Circular dependencies are forbidden. The layer system is enforced by
the workspace dependency declarations.

---

## Layer 2: Cell & Type Model

### Cell Representation
The `Cell` struct in `mux-types` represents a single terminal cell:

```rust
pub struct Cell {
    pub grapheme: GraphemeId,  // Arena-allocated UTF-8 grapheme
    pub width: u8,             // Display width: 0=padding, 1=normal, 2=CJK
    pub flags: CellFlags,      // Bitflags matching tmux GRID_FLAG_*
    pub attrs: Attrs,          // Text attributes (bold, italic, etc.)
    pub fg: Colour,            // Foreground colour
    pub bg: Colour,            // Background colour
    pub us: Colour,            // Underline colour (OSC 4)
    pub link: u32,             // Hyperlink ID (OSC 8)
}
```

### CellFlags (INV-119)
Bit values match tmux's `GRID_FLAG_*` from `tmux.h` exactly:

| Flag     | Value | tmux.h line | Occurrences |
|----------|-------|-------------|-------------|
| PADDING  | 0x04  | 744         | 42 in 9 files |
| EXTENDED | 0x08  | 745         | used for multi-codepoint graphemes |
| SELECTED | 0x10  | 746         | copy mode selection |
| CLEARED  | 0x40  | 748         | explicitly cleared cells |
| TAB      | 0x80  | 749         | tab character markers |

Note: FG256 (0x01), BG256 (0x02), and NOPALETTE (0x20) are encoding flags
in tmux's compact representation. TermForge uses the `Colour` enum instead,
making these flags unnecessary.

### Colour Model
```rust
pub enum Colour {
    Default,              // Terminal default fg/bg
    Indexed(u8),          // 256-colour palette (0-255)
    Rgb { r: u8, g: u8, b: u8 },  // True colour
}
```

### Grapheme Arena
Multi-codepoint grapheme clusters (emoji, combining characters) are stored
in a deduplicating arena. Single-codepoint graphemes are stored inline
in `GraphemeId::Inline(char)` with zero arena overhead.

---

## Layer 3: Grid Design (INV-127)

### Chunked COW Grid
The v25 DEFINITIVE grid design:

```
ChunkedGrid
  chunks: Vec<Arc<LineChunk>>    // COW backbone
  line_dirty: BitVec             // per-line dirty tracking
  sx, sy, hsize, hlimit         // dimensions

LineChunk
  lines: Vec<Line>               // CHUNK_SIZE=256 lines per chunk

Line
  cells: Vec<Cell>               // one cell per column
```

### COW Mechanism
- `line(&self, y)`: read-only, no COW, no dirty mark.
- `line_mut(&mut self, y)`: calls `Arc::make_mut(chunk)`, marks line dirty.
- `snapshot()`: clones `Vec<Arc<LineChunk>>` backbone. Shared chunks.
- Cost: ~16-32 KB per mutation with outstanding snapshot (vs 3.2 MB monolithic).

### Dirty Tracking
Single `BitVec` for all lines. No per-chunk dirty tracking (over-engineering
eliminated in v25 DEFINITIVE). Clear dirty bits after render consumes snapshot.

### Wide Character Co-invalidation
When writing to a cell that is a PADDING cell (right half of wide char),
the primary cell (left half) is cleared. When writing to the primary cell
of a wide char, the padding cell is cleared. This matches tmux's
`screen-write.c:1928-1976` behavior.

### Scroll Regions
Scroll region operations are chunk-boundary-aware. Lines are moved within
and across chunks via `Arc::make_mut` COW.

### Reflow
On resize, full reconstruction: collect all lines, reflow to new width,
rebuild chunk vector. All lines marked dirty after reflow.

---

## Layer 4: VT Parser

### State Machine
Hand-written state machine following the Paul Williams VT parser model.
Not using the `vte` or `termwiz` crate -- full control for tmux-compatible
escape sequence handling.

States: Ground, Escape, EscapeIntermediate, CsiEntry, CsiParam,
CsiIntermediate, CsiIgnore, DcsEntry, DcsParam, DcsIntermediate,
DcsPassthrough, DcsIgnore, OscString, SosString.

### Actions
- `Print(char)`: output character at cursor position
- `Execute(u8)`: C0/C1 control character
- `CsiDispatch`: CSI sequence with params and final byte
- `EscDispatch`: ESC sequence
- `OscDispatch`: OSC string
- `DcsHook`/`DcsPut`/`DcsUnhook`: DCS passthrough

### Input Parsing
Terminal input byte sequences are parsed into `KeyEvent` and `MouseEvent`.
SGR mouse encoding is supported (CSI < Pb;Px;Py M/m).

---

## Layer 5: Wire Protocol

### TF01 Native Protocol
```
Offset  Size  Field         Description
0       4     magic         "TF01" (0x54, 0x46, 0x30, 0x31)
4       2     version       Protocol version (network byte order)
6       2     flags         Reserved (must be 0 for v1)
8       4     frame_length  Total frame length including header
12      4     frame_type    Frame type identifier
16      ...   payload       frame_length - 16 bytes
```

### tmux imsg Compatibility
The TF01 protocol coexists with tmux's imsg protocol on the same Unix socket.
Discrimination: TF01 magic byte 0x54 (84) exceeds all tmux message types
(12-307, max LE first byte 0x33=51). Safe in 1 byte; 4 used for depth.

### Frame Types
Ping/Pong, Identify/Ready, KeyInput, MouseInput, Resize, RenderFrame,
Command/CommandResponse, GridData, StateUpdate, Error.

---

## Layer 6: Kernel Architecture

### Threading Model
```
IO Thread (tokio)
  |
  | KernelEvent channel (bounded)
  v
Kernel Thread (std::thread)
  |
  | KernelEffect channel (bounded)
  v
IO Thread (tokio) + Render Thread
```

The kernel is single-threaded. It owns ALL mutable state:
- Sessions, Windows, Panes (in HashMaps with typed ID keys)
- Each Pane owns a ChunkedGrid and GraphemeArena
- Server/Session/Window/Pane OptionTables
- Key bindings, paste buffers, environment
- Global revision counter

### Sans-IO Design
The kernel never performs I/O directly. It receives `KernelEvent`s and
emits `KernelEffect`s. This enables deterministic testing: inject events,
assert effects, without real PTYs or sockets.

### Event Loop
```rust
loop {
    let event = channel.recv();
    let effects = kernel.process_event(event);
    for effect in effects {
        effect_channel.send(effect);
    }
}
```

### Command Execution
Commands are parsed by `mux-cmd-parse`, resolved by `mux-target`,
and executed by the kernel's command dispatch table. Command execution
is synchronous within the kernel thread.

---

## Layer 7: Option System

### Scoped Options
Four scope levels matching tmux: Server, Session, Window, Pane.
Lookup walks the inheritance chain: pane -> window -> session -> server -> default.

### Option Types
String, Number, Boolean, Colour, Style, Array.

### Format Strings
The `mux-format` crate handles `#{...}` variable expansion,
conditional formats `#{?...,...,...}`, and format aliases (`#S`, `#W`, etc.).

---

## Layer 8: Target Resolution

### Target Syntax
Full tmux target grammar: `[session:]window[.pane]`

Components support:
- IDs: `$N` (session), `@N` (window), `%N` (pane)
- Names and patterns (fnmatch)
- Exact match: `=name`
- Special tokens: `{last}`, `{next}`, `{previous}`, `{start}`, `{end}`
- Shorthand: `!`, `+`, `-`, `^`, `$`

---

## Layer 9: Rendering Pipeline

### Pipeline Stages
1. Kernel produces `GridSnapshot` (COW, zero-copy backbone)
2. Render thread receives snapshot
3. For each pane: iterate dirty lines, emit escape sequences
4. Compose borders, status line
5. Coalesce output into single write buffer
6. Send to client via IO thread

### Optimization
- Dirty-line tracking: only re-render changed lines
- Attribute coalescing: minimize SGR sequence changes
- Cursor movement optimization: relative moves when cheaper
- Double-buffering: render to buffer, then single write

---

## Layer 10: PTY Management

### Allocation
- `posix_openpt` + `grantpt` + `unlockpt` via nix/libc
- Linux optimization: `TIOCGPTPEER` ioctl
- Fallback: `ptsname_r` for peer path

### Child Process Lifecycle
- Fork + setsid + open peer + dup2 + exec
- SIGCHLD handling for zombie reaping
- Job control passthrough (SIGTSTP, SIGCONT)
- Process group management

### Safety
`mux-pty` is the ONLY crate allowed unsafe (besides `mux-ffi`).
All unsafe blocks have `// SAFETY:` documentation.

---

## Layer 11: Testing Architecture

### Termlet (Testing Pods)
Lightweight self-contained terminal: PTY + VT parser + Grid + Arena.
Designed for snapshot testing:
```rust
let termlet = Termlet::builder().size(80, 24).build()?;
termlet.send_keys("echo hello\n")?;
termlet.wait_for("hello", Duration::from_secs(5))?;
insta::assert_snapshot!(termlet.capture());
```

### Test Support Fixtures
Inspired by libtmux's pytest plugin:
- `TestServer`: isolated kernel with unique socket name
- `test_kernel()`: deterministic kernel with manual clock
- `test_kernel_with_session()`: kernel + pre-created session
- `assert_grid_text!()`: macro for grid content assertion
- `sizes::STANDARD`, `sizes::WIDE`, etc.: standard test sizes

### Testing Levels
1. **Unit**: `#[cfg(test)]` in each source file
2. **Snapshot**: `insta` golden-file tests for grid content
3. **Property**: `proptest` for round-trip and invariant testing
4. **Integration**: `mux-test-support` fixtures for kernel event loops
5. **Fuzz**: `cargo-fuzz` for parser and protocol robustness

---

## Layer 12: FFI & Bindings

### C FFI (mux-ffi)
- `extern "C"` functions for grid, protocol, version
- `catch_unwind` at every boundary (INV-117)
- Opaque handles (`*mut FfiGrid`) with create/destroy lifecycle
- Null pointer safety on all functions
- Static library + cdylib output

### Python Bindings (future)
PyO3-based bindings exposing Termlet, Grid, and Kernel APIs.
Located in `bindings/python/`, excluded from workspace build.

### Node.js Bindings (future)
napi-rs-based bindings for the same APIs.
Located in `bindings/node/`, excluded from workspace build.

---

## Layer 13: Observability

### OpenTelemetry (mux-otel)
- Feature-gated: `otel` feature enables OTLP export
- Without `otel`: tracing-subscriber fmt layer only
- Trace context propagation via TRACEPARENT/TRACESTATE/BAGGAGE env vars
- Export filter: only `mux_*` and `termforge*` spans

### Tracing Integration
- All crates use `tracing` for structured logging
- Span hierarchy: server -> session -> window -> pane
- Key events: PTY data, key input, command execution, render cycles
- Performance: grid mutation count, COW trigger count, render latency

### Diagnostic Tool (mux-doctor)
CLI tool that validates:
- PTY support and capabilities
- Terminal size detection
- OTEL connectivity
- Protocol version compatibility
