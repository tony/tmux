# TermForge

A Rust terminal multiplexer SDK targeting 100% tmux wire-protocol compatibility.

## Architecture

```
                        TermForge Architecture
    ================================================================

    +-----------+     +-------------------+     +---------------+
    |  Clients  |     |    IO Thread      |     | Render Thread |
    | (tmux,    |<--->|    (tokio)        |<--->| (composition) |
    |  TF01,    |     |                   |     |               |
    |  Python,  |     | - PTY I/O         |     | - Grid snap   |
    |  Node.js) |     | - Unix socket     |     | - Dirty-line  |
    +-----------+     | - Signal handler  |     | - SGR output  |
                      | - TF01/imsg codec |     +-------+-------+
                      +--------+----------+             |
                               |                        |
                    KernelEvent|          GridSnapshot   |
                       channel |          (Arc<Chunk>)   |
                               v                        |
                      +--------+----------+             |
                      |   Kernel Thread   |<------------+
                      |   (std::thread)   |
                      |                   |
                      | SlotMap<Session>  |    +-----------------+
                      | SlotMap<Window>   |    | mux-termlet     |
                      | SlotMap<Pane>     |    | (test pods)     |
                      |   ChunkedGrid    |    |                 |
                      |   GraphemeArena   |    | PTY + VT parser |
                      |   VtParser        |    | + Grid + Arena  |
                      |   OptionTable     |    | = snapshot test |
                      +-------------------+    +-----------------+

    ================================================================
    Wire Protocol: TF01 (native) + tmux imsg (compat)
    Discrimination: magic byte 0x54 > all tmux types (max 307)
    ================================================================
```

## Workspace Layout

```
termforge/
  Cargo.toml              # Workspace root (19 crates + 4 tools)
  CLAUDE.md               # Coding conventions for Claude Code
  AGENTS.md               # Agent roles and quality gates
  README.md               # This file
  notes/
    architecture.md       # 13-layer comprehensive architecture
    plan.md               # Phased implementation plan with LOC estimates

  crates/                 # 19 library crates
    mux-grapheme-arena/   # L0: Arena for extended grapheme clusters
    mux-time/             # L0: Injectable clock for deterministic testing
    mux-types/            # L1: Core types (Cell, CellFlags, Colour, KeyCode, IDs)
    mux-grid/             # L2: Chunked COW grid with dirty-line tracking
    mux-parser/           # L2: VT parser and input decoder
    mux-proto/            # L2: TF01 wire protocol + tmux imsg compat
    mux-options/          # L2: Typed option system (set-option/show-options)
    mux-target/           # L2: Target syntax parser (session:window.pane)
    mux-format/           # L2: Format string parser (#{...} syntax)
    mux-snapshot/         # L3: Grid snapshot serialization
    mux-cmd-parse/        # L3: Command and config file parser
    mux-orm/              # L3: ORM traversal (Server->Session->Window->Pane)
    mux-render/           # L3: Composition/rendering pipeline
    mux-pty/              # L3: PTY allocation (unsafe quarantine)
    mux-kernel/           # L4: Single-threaded kernel (owns all state)
    mux-termlet/          # L4: Lightweight testing pods
    mux-test-support/     # L5: Test harness, fixtures, helpers
    mux-ffi/              # L5: C FFI shim (catch_unwind boundary)
    mux-otel/             # L5: OpenTelemetry integration

  tools/                  # 4 binary tools
    tmux-builder/         # Workspace layout builder (YAML/TOML -> sessions)
    tmux-vm/              # Headless tmux VM for CI testing
    tmux-sniff/           # Wire protocol sniffer/debugger
    mux-doctor/           # Diagnostic health checker

  bindings/               # Language bindings (excluded from workspace)
    python/               # PyO3 bindings (future)
    node/                 # napi-rs bindings (future)
```

## Quick Start

```bash
# Check that everything compiles
cargo check

# Run all tests
cargo test

# Run tests for a specific crate
cargo test -p mux-grid

# Run with clippy
cargo clippy -- -D warnings
```

## Design Principles

1. **SDK-first**: TermForge is a library (SDK) that happens to have a binary.
   Every capability is testable programmatically.

2. **Single-threaded kernel**: All mutable state lives in one thread.
   No locks, no races, no shared mutable state.

3. **Sans-IO kernel**: The kernel never does I/O directly. It receives events
   and emits effects through typed channels.

4. **Chunked COW grid**: `Vec<Arc<LineChunk>>` enables cheap snapshots for
   rendering while the kernel continues processing.

5. **tmux compatibility**: Wire-protocol compatible with tmux. The TF01 native
   protocol coexists with tmux imsg on the same Unix socket.

6. **Deterministic testing**: Injectable `Clock`, sans-IO design, and snapshot
   testing enable fully reproducible test runs.

## License

MIT OR Apache-2.0
