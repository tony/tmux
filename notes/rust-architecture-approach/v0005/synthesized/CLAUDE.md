# CLAUDE.md -- TermForge v0005 Project Conventions

## Scope

This is the v0005 definitive Rust architecture scaffold for TermForge, a
terminal multiplexer SDK built from scratch in Rust with full tmux
compatibility. The codebase implements 23 library crates across 6 layers
(L0-L5), plus 4 tool binaries (L6).

## Project Identity

- **Name**: TermForge
- **License**: MIT OR Apache-2.0
- **Rust Edition**: 2024
- **MSRV**: 1.85 (required for edition 2024)
- **Binary prefix**: `mux-` for all library crates
- **Repository**: `https://github.com/termforge/termforge`

## Workspace Structure

```
crates/          # All library crates (23 total)
tools/           # CLI tool binaries (4 total)
bindings/node/   # Node.js language bindings (excluded from workspace)
bindings/python/ # Python language bindings (excluded from workspace)
notes/           # Architecture documentation
```

### Crate Layers

| Layer | Crates | Description |
|-------|--------|-------------|
| L0 | mux-grapheme-arena, mux-time | Leaf crates, no internal deps |
| L1 | mux-types | Core type definitions |
| L2 | mux-grid, mux-parser, mux-options, mux-target, mux-format, mux-cmd-parse, mux-proto, mux-crdt, mux-fdpass | Data structures and protocols |
| L3 | mux-snapshot, mux-kernel, mux-render, mux-orm, mux-pty | Business logic |
| L4 | mux-termlet, mux-config, mux-api, mux-test-support | Integration |
| L5 | mux-ffi, mux-otel | External surface / bindings |
| L6 | tmux-builder, tmux-vm, tmux-sniff, mux-doctor | CLI tools |

## Hard Rules

### Safety

- `unsafe` is forbidden in every crate except `mux-pty` and `mux-ffi` (INV-003).
  All other crates use `#![forbid(unsafe_code)]`.
- Rust 2024 edition requires `#[unsafe(no_mangle)]` syntax for FFI exports.
- `gen` is a reserved keyword in edition 2024; use `generate` or `gen_id` instead.
- `panic = "unwind"` is required in all profiles for `catch_unwind` panic
  recovery at FFI boundaries (INV-117).

### Error Handling

- All error types implement `std::error::Error + Send + Sync + 'static` (INV-005).
- No panicking in library code paths (INV-006). `unwrap/expect` are test-only.
- Public `Result`-returning APIs should be `#[must_use]`.
- Use `thiserror` for error derives in library crates.
- Use `anyhow` only in tool binaries and test support.

### Type System

- All public types derive `Debug` (INV-004).
- Grid coordinates are zero-based (INV-007).
- Cell grapheme values must be valid UTF-8 (INV-008).
- Cell width field must be 0, 1, or 2 (INV-029).
- CellFlags bit values must match tmux's GRID_FLAG_* exactly (INV-119).
- Graphics passthrough defaults to false (INV-220).

### Architecture

- Kernel logic (`mux-kernel`) must be pure and deterministic -- no I/O, no
  global state. The kernel processes `KernelEvent` and returns `KernelEffect`.
- Grid lines use `Arc<Vec<Cell>>` for copy-on-write snapshot isolation (INV-032).
- Double-buffer diff rendering: write to "next" buffer, diff against "prev",
  emit minimal escape sequences to terminal.
- VT parser uses Paul Williams' 14-state machine with `CLASS_TABLE[256]` lookup.
- CRDT operations are feature-gated behind `crdt` feature flag.

### Linting

- Clippy is configured at `warn` level for `all`, `pedantic`, `nursery`, `cargo`.
- `unwrap_used`, `expect_used`, `panic` are denied via clippy.
- `uninlined_format_args`, `redundant_clone`, `redundant_closure` are denied.
- Many pedantic lints are allowed for practical reasons (see Cargo.toml).

### Testing

- Every crate has inline `#[cfg(test)]` modules.
- Integration tests go in `crates/<name>/tests/`.
- Use `proptest` for property-based testing of all encoding round-trips,
  algebraic properties (commutativity, idempotency, associativity), and
  boundary conditions.
- Use `insta` with YAML for snapshot testing.
- Test socket paths must be unique and outside the default tmux socket
  directory. Always clean up.
- `mux-test-support` provides `TestServer`, `CleanupGuard`, and
  `isolated_socket_path()` for test harnesses.

## Preferred Patterns

- Expose stable API surfaces from `lib.rs` via `pub use` or `prelude` modules.
- Keep reducer logic in `mux-kernel` pure and deterministic.
- Keep serialization little-endian and deterministic (INV-034).
- Keep tests close to implementation and name by behavior/invariant.
- Use `slotmap` for entity storage (sessions, windows, panes).
- Use `indexmap` where insertion order must be preserved.
- Use builder pattern for complex construction (ServerBuilder, TermletBuilder).
- Use `crossbeam-channel` for the kernel-IO bridge (bounded channels).

## Channel Bounds

| Channel | Bound |
|---------|-------|
| control | 512 |
| data | 1024 |
| render | 256 |
| effect | 1024 |

## What Not To Do

- Do not add `SmallVec` for core line/cell storage (INV-024).
- Do not move I/O concerns into `mux-kernel`.
- Do not remove `#[cfg(feature = "crdt")]` gates around vector-clock fields.
- Do not change `PackedCell` bit layout without updating invariants/tests/docs.
- Do not use `unwrap()` or `expect()` in library code (use `unwrap_or`,
  `unwrap_or_default`, `unwrap_or_else`, or `?` operator).
- Do not use `gen` as an identifier (reserved in edition 2024).
- Do not create tmux sockets in the default path during tests.
- Do not use `SmallVec` or `tinyvec` for cell storage.

## Wire Protocol (TF01)

```
Magic(4) + type(2) + flags(2) + payload_len(4) + crc32(4) + payload(N)
```

Total header: 16 bytes before payload.

## Terminal Mode Policy (Gap-2 Resolution)

Three modes for managing the host terminal's raw mode state:

| Mode | Description |
|------|-------------|
| Managed | TermForge owns raw mode transitions via `RawModeGuard` RAII |
| External | Caller manages raw mode; TermForge does not touch termios |
| SaveRestore | TermForge saves/restores termios on entry/exit |

## Flood Fairness

Per-pane row quota with stride rotation prevents a single high-output pane
from starving others during rendering.

## Primary References

- `notes/architecture.md`: Full architecture document with all design decisions.
- `notes/plan.md`: Implementation roadmap with phases and milestones.
- `AGENTS.md`: Rule catalog with invariants and enforcement methods.

## Gap Resolution Summary

The v0005 scaffold addresses all 9 identified gaps from the synthesis audit:

| Gap | Resolution | Crate |
|-----|-----------|-------|
| Gap-1: SIGWINCH propagation | Defined signal flow in mux-pty | mux-pty |
| Gap-2: Raw mode lifecycle | TerminalMode enum (Managed/External/SaveRestore) | mux-pty |
| Gap-3: Rendering composition | Double-buffer diff with RenderBuffer | mux-render |
| Gap-4: Layout engine | Binary tree LayoutNode with distribute_evenly | mux-kernel |
| Gap-5: Threading model | Tokio async with sans-IO kernel | mux-kernel, mux-api |
| Gap-6: Zombie reaping | SIGCHLD handler via signal-hook | mux-pty |
| Gap-7: SCM_RIGHTS | FdEnvelope with 16-fd cap | mux-fdpass |
| Gap-8: SDK ergonomics | ServerBuilder + fluent API | mux-api |
| Gap-9: Error recovery | Typed errors with PtyError, KernelError, etc. | all crates |

## Dependency Rationale

| Crate | Why |
|-------|-----|
| `signal-hook` | Ecosystem-standard signal handling (127M downloads) |
| `crossbeam-channel` | Bounded MPSC for kernel-IO bridge |
| `nix` | POSIX syscall wrappers for PTY, signals, termios |
| `winnow` | Parser combinator for command parsing |
| `crc32fast` | CRC32 for wire protocol frame checksums |
| `slotmap` | Generational arena for entity management |
| `indexmap` | Insertion-ordered maps for option tables |
| `bitflags` | Type-safe bitfield operations for Attrs/CellFlags |
| `unicode-width` | East Asian Width property for cell width calculation |
| `unicode-segmentation` | Grapheme cluster boundaries |
| `thiserror` | Derive macro for Error trait implementations |
| `tokio` | Async runtime with signal support |
| `proptest` | Property-based testing framework |
| `insta` | Snapshot testing with YAML support |

## Crate Development Checklist

When implementing or modifying a crate, verify these items:

### Before Starting

1. Read this CLAUDE.md document.
2. Read AGENTS.md for applicable rules.
3. Identify which layer the crate belongs to.
4. Verify dependencies only flow downward (L(N) depends on L(0..N-1)).

### During Development

1. Add `#![forbid(unsafe_code)]` unless the crate is mux-pty or mux-ffi.
2. Use `thiserror` for all error types.
3. Derive `Debug` on all public types.
4. Use `unwrap_or`, `unwrap_or_default`, or `?` instead of `unwrap()`/`expect()`.
5. Do not use `gen` as an identifier.
6. Add doc comments to all public functions.
7. Write tests alongside implementation.

### Before Submitting

1. Run `cargo clippy --all-targets` and fix all warnings.
2. Run `cargo test` and verify all tests pass.
3. Verify test count did not decrease.
4. Check that new proptest coverage exists for any new encoding/decoding.
5. Verify socket paths in tests use `isolated_socket_path()`.

## Commit Message Format

```
<type>(<scope>): <short description>

<optional body>
```

Types: feat, fix, refactor, test, docs, chore, perf, ci
Scopes: crate names (mux-grid, mux-parser, etc.) or workspace

Examples:
```
feat(mux-grid): implement alternate screen buffer
fix(mux-parser): handle malformed CSI sequences without panic
test(mux-crdt): add proptest for merge associativity
docs(architecture): document SIGWINCH propagation chain
```

## File Naming Conventions

| File | Convention |
|------|-----------|
| Source | `src/lib.rs` for library crates, `src/main.rs` for binaries |
| Modules | `src/<name>.rs` for single-file modules |
| Tests | `tests/<name>.rs` for integration tests |
| Proptests | `tests/proptest_<name>.rs` |
| Benchmarks | `benches/<name>.rs` |
| Fuzz targets | `fuzz/fuzz_targets/<name>.rs` |

## Important Constants

| Constant | Value | Location |
|----------|-------|----------|
| MAX_FDS | 16 | mux-fdpass |
| FRAME_MAGIC | "TF01" | mux-proto |
| FRAME_HEADER_SIZE | 16 | mux-proto |
| DEFAULT_SCROLLBACK | 2000 | mux-grid |
| DEFAULT_ESCAPE_TIME | 500 | mux-config |
| CHANNEL_CONTROL | 512 | mux-api |
| CHANNEL_DATA | 1024 | mux-api |
| CHANNEL_RENDER | 256 | mux-api |
| CHANNEL_EFFECT | 1024 | mux-api |

## VT Parser States

The 14 states of the Paul Williams VT parser:

1. Ground -- normal text input
2. Escape -- after ESC byte
3. EscapeIntermediate -- after ESC + intermediate
4. CsiEntry -- after CSI introducer
5. CsiParam -- collecting CSI parameters
6. CsiIntermediate -- CSI intermediate bytes
7. CsiIgnore -- discarding malformed CSI
8. DcsEntry -- after DCS introducer
9. DcsParam -- collecting DCS parameters
10. DcsIntermediate -- DCS intermediate bytes
11. DcsPassthrough -- passing through DCS data
12. DcsIgnore -- discarding malformed DCS
13. OscString -- collecting OSC string
14. SosPmApcString -- collecting SOS/PM/APC

## Byte Classes

The CLASS_TABLE maps all 256 byte values to one of 7 classes:

| Class | Bytes | Description |
|-------|-------|-------------|
| Printable | 0x20-0x7E | Visible characters |
| Control | 0x00-0x1F (except 0x1B) | C0 control codes |
| Escape | 0x1B | Escape byte |
| CsiEntry | 0x9B | 8-bit CSI |
| DcsEntry | 0x90 | 8-bit DCS |
| OscEntry | 0x9D | 8-bit OSC |
| Utf8Lead | 0xC0-0xFD | UTF-8 leading bytes |

## Architecture Decision Records

When making a design decision that affects multiple crates:

1. Document the decision in notes/architecture.md.
2. Assign an SD-NN identifier.
3. Record the rationale.
4. List affected invariants.
5. Update AGENTS.md if new rules are needed.

## Performance Budget

| Operation | Budget |
|-----------|--------|
| VT parser per byte | <5ns |
| Grid cell write | <50ns |
| Snapshot capture (80x24) | <1us |
| Render diff (80x24) | <100us |
| Frame encode (1KB payload) | <5us |
| Arena intern | <100ns |
| Option resolve (4-scope chain) | <200ns |
