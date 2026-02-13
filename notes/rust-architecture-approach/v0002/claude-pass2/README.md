# TermForge

A Rust terminal multiplexer SDK with 100% tmux v3.5a wire-protocol compatibility.

## What is TermForge?

TermForge is an SDK-first terminal multiplexer. It provides:

- **Embeddable API** via `mux-api` for terminal emulators, IDEs, CI tools, and test harnesses.
- **tmux compatibility** for the 59 built-in commands, wire protocol v8, and `.tmux.conf` parsing.
- **Language bindings** planned for Python (PyO3) and Node.js (napi-rs) with QuerySet-style traversal.
- **Termlets** -- lightweight testing pods that wrap a PTY + VT parser + shell for snapshot testing.
- **CRDT support** for collaborative terminal sessions across federated servers.

## Architecture

```
+-------------------------------------------------------+
| L6: Tools                                              |
| tmux-builder | tmux-vm | tmux-sniff | mux-doctor       |
+-------------------------------------------------------+
| L5: External Surface                                   |
| mux-ffi | mux-otel                                     |
+-------------------------------------------------------+
| L4: Integration                                        |
| mux-api | mux-test-support | mux-termlet | mux-config  |
+-------------------------------------------------------+
| L3: Business Logic                                     |
| mux-kernel | mux-render | mux-orm | mux-snapshot |     |
| mux-pty                                                |
+-------------------------------------------------------+
| L2: Data Structures                                    |
| mux-grid | mux-parser | mux-proto | mux-options |      |
| mux-target | mux-format | mux-cmd-parse                |
+-------------------------------------------------------+
| L1: Core Types                                         |
| mux-types                                              |
+-------------------------------------------------------+
| L0: Leaf (zero internal deps)                          |
| mux-grapheme-arena | mux-time                          |
+-------------------------------------------------------+
```

Each layer may only depend on layers below it. This prevents circular
dependencies and ensures clear module boundaries.

## Three-Thread Model

```
IO Thread (tokio)  <--crossbeam-->  Kernel Thread (std)  --snapshot-->  Render Thread
  PTY FDs                           ALL mutable state                  CompositeGrid
  Unix socket                       Entity model                       Dirty-line diff
  signal-hook                       Layout engine                      Output coalesce
                                    Copy mode                          Flood fairness
```

- **IO Thread** (tokio): Async I/O for PTY FDs, Unix sockets, and signal pipes. Translates I/O events into `KernelEvent`s and executes `KernelEffect`s.
- **Kernel Thread** (std::thread): Owns all mutable state. Pure `process_event() -> Vec<KernelEffect>` reducer. No I/O, no async, fully deterministic.
- **Render Thread** (tokio task): Receives COW grid snapshots, composites visible panes via double-buffer diff, emits minimal escape sequences.

## Key Design Principles

1. **SDK-first**: `mux-api` is the primary entry point for embedding.
2. **Sans-IO kernel**: Deterministic, testable, no I/O in the kernel thread.
3. **Arc-COW grids**: Grid lines use `Arc<Vec<Cell>>` for zero-copy snapshots.
4. **Double-buffer diff rendering**: Only emit escape sequences for changed cells.
5. **Flood fairness**: Per-pane row quota with stride rotation prevents render starvation.
6. **Zero panic**: Clippy denies `unwrap_used`, `expect_used`, `panic`. FFI uses `catch_unwind`.
7. **Unsafe quarantine**: Only `mux-pty` (PTY syscalls) and `mux-ffi` (C ABI boundary).

## Workspace Layout

```
termforge/
  Cargo.toml            # Workspace manifest (21 crates + 4 tools)
  CLAUDE.md             # Coding conventions and invariants
  AGENTS.md             # AI agent rules
  README.md             # This file
  notes/
    architecture.md     # Full 20-section architecture document (900+ lines)
    plan.md             # Implementation plan with phases and LOC estimates
  crates/
    mux-grapheme-arena/ # L0: Arena allocator for extended grapheme clusters
    mux-time/           # L0: Injectable Clock (System/Manual) for deterministic testing
    mux-types/          # L1: Cell, CellFlags, Colour, Attrs, entity IDs, Size, errors
    mux-grid/           # L2: Chunked COW grid with dirty tracking (64-line chunks)
    mux-parser/         # L2: VT state machine (14 Paul Williams states)
    mux-proto/          # L2: TF01 wire protocol with CRC32 validation
    mux-options/        # L2: Typed option system with layered inheritance
    mux-target/         # L2: tmux target syntax parser (session:window.pane)
    mux-format/         # L2: Format string #{} parser and evaluator
    mux-cmd-parse/      # L2: Hand-written command tokenizer/parser
    mux-snapshot/       # L3: Grid snapshot serialization and text extraction
    mux-kernel/         # L3: Sans-IO kernel (entities, layout, copy mode, dispatch)
    mux-render/         # L3: CompositeGrid + diff + flood fairness + output
    mux-orm/            # L3: ORM-like QuerySet traversal API
    mux-pty/            # L3: PTY allocation + process management (unsafe quarantine)
    mux-termlet/        # L4: Testing pods (PTY + VT + shell)
    mux-config/         # L4: TOML configuration system
    mux-api/            # L4: SDK entry point (MuxServer builder)
    mux-test-support/   # L4: Shared test harness and fixtures
    mux-ffi/            # L5: C FFI shim with catch_unwind (unsafe quarantine)
    mux-otel/           # L5: OpenTelemetry tracing integration
  tools/
    tmux-builder/       # L6: YAML/TOML workspace layout builder
    tmux-vm/            # L6: tmux version manager for compatibility testing
    tmux-sniff/         # L6: Wire protocol inspector / sniffer
    mux-doctor/         # L6: Diagnostic environment checker
  bindings/             # (excluded from workspace, future)
    node/               # Node.js bindings via napi-rs
    python/             # Python bindings via PyO3
```

## Multi-File Module Pattern

Larger crates use GPT's multi-file module pattern for maintainability:

| Crate | Modules |
|-------|---------|
| mux-types | `cell.rs`, `style.rs`, `identity.rs`, `error.rs`, `input.rs`, `geometry.rs` |
| mux-parser | `state.rs`, `action.rs`, `parser.rs` |
| mux-grid | `line.rs`, `chunk.rs`, `grid.rs`, `snapshot.rs` |
| mux-kernel | `entity.rs`, `layout.rs`, `event.rs`, `dispatch.rs`, `copy_mode.rs`, `kernel.rs` |
| mux-render | `composite.rs`, `diff.rs`, `output.rs`, `flood.rs`, `blit.rs` |

Smaller crates use a single `lib.rs` file.

## Quick Start

```bash
# Type-check all 21 crates + 4 tools
cargo check

# Run all tests (192 tests)
cargo test

# Run tests for a single crate
cargo test -p mux-grid

# Lint all crates
cargo clippy -- -D warnings

# Generate documentation
cargo doc --no-deps
```

## Test Count

The scaffold includes **192 passing tests** across all 21 crates, exceeding the
150-test target. Key coverage:

- **mux-types**: 33 tests (cell flags, colour pack/unpack, identity, geometry, input)
- **mux-parser**: 13 tests (CSI SGR, cursor movement, OSC, ESC, DCS, C0 controls)
- **mux-grid**: 19 tests (line COW, chunk operations, grid scrollback, snapshots)
- **mux-kernel**: 32 tests (entities, layout algorithms, events, dispatch, copy mode, kernel reducer)
- **mux-render**: 17 tests (composite grid, diff, output SGR/CUP, flood fairness, blit)
- Plus unit tests in all other crates.

## Architecture Invariants

| ID | Invariant | Verified In |
|----|-----------|-------------|
| INV-117 | `catch_unwind` at FFI boundary | mux-ffi |
| INV-119 | CellFlags bit values match tmux GRID_FLAG_* (tmux.h:742-749) | mux-types/cell.rs |
| INV-200 | SIGWINCH via signal-hook pipe | architecture.md S13 |
| INV-211 | waitpid loop drains all children | architecture.md S13 |
| INV-220 | Graphics passthrough disabled by default | CLAUDE.md |
| INV-230 | No handlers for SIGTSTP/SIGCONT/SIGPIPE | architecture.md S13 |

## Technology Stack

| Category | Choice | Rationale |
|----------|--------|-----------|
| Async runtime | tokio | IO thread needs full async (PTY, sockets, signals) |
| Kernel-IO bridge | crossbeam-channel | Lock-free bounded channels between std::thread and tokio |
| Signals | signal-hook | Pipe-based handlers, works with tokio via AsyncFd |
| Error handling | thiserror (lib), anyhow (bin) | Standard Rust error pattern |
| Terminal I/O | nix + libc | Direct syscall control, no crossterm dependency |
| VT parsing | Custom (Paul Williams) | Full control over state machine, tmux behavior matching |
| Serialization | serde + toml + bincode | Config (toml), internal wire (bincode), state (serde_json) |
| Testing | insta + proptest + tempfile | Snapshot regression, property tests, temp directories |
| Observability | opentelemetry + OTLP | Cross-service tracing for server, client, language bindings |

## MSRV

Rust 2024 edition, MSRV 1.85.

## License

MIT OR Apache-2.0
