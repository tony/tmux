# TermForge v0003

A Rust terminal multiplexer SDK targeting tmux v3.5a protocol behavior.

## Architecture

```
+-------------------+     crossbeam     +-------------------+
|                   |     channels      |                   |
|   IO Thread       | <===============> |   Kernel Thread   |
|   (tokio)         |  KernelEvent      |   (std::thread)   |
|                   |  KernelEffect     |                   |
+-------------------+                   +-------------------+
         |                                       |
         |                              GridSnapshot (COW)
         |                                       |
         |                             +-------------------+
         +----------------------------> |  Render Thread    |
              rendered bytes            |  (tokio task)     |
                                        +-------------------+
```

## Key Design Principles

1. **Sans-IO Kernel**: Single-threaded, deterministic, no I/O
2. **Double-Buffer Diff Rendering**: Minimal escape sequences
3. **Arc-COW Grid Lines**: Cheap snapshots for render pipeline
4. **Pipe-Based Signals**: No heavy work in signal handlers

## Workspace: 23 Crates + 4 Tools

| Layer | Crates |
|-------|--------|
| L0 Leaf | mux-grapheme-arena, mux-time |
| L1 Types | mux-types |
| L2 Data | mux-grid, mux-parser, mux-options, mux-target, mux-format, mux-cmd-parse, mux-proto, mux-crdt, mux-fdpass |
| L3 Logic | mux-snapshot, mux-kernel, mux-render, mux-orm, mux-pty |
| L4 Integration | mux-termlet, mux-config, mux-api, mux-test-support |
| L5 Surface | mux-ffi, mux-otel |
| L6 Tools | tmux-builder, tmux-vm, tmux-sniff, mux-doctor |

## Quick Start

```bash
cargo check                    # Type-check all crates
cargo test --workspace         # Run 367 tests
cargo test -p mux-kernel       # Run kernel tests only
```

## v0003 Improvements over v0002

- **367 tests** (up from 202 in v0002)
- Multi-file modules for mux-types, mux-parser, mux-grid, mux-kernel, mux-render
- All 12 architecture gaps addressed
- RawModeGuard RAII for terminal mode lifecycle
- Layout engine with 5 built-in algorithms + checksum validation
- Comprehensive SGR encoder with optimization
- Flood fairness scheduler with stride rotation
- Copy mode state machine with buffer ring
- Full VT parser with UTF-8 support
- SDK builder pattern with INV-220 enforcement
