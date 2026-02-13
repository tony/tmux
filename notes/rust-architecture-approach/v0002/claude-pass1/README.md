# TermForge

A Rust terminal multiplexer SDK with 100% tmux wire-protocol v8 compatibility.

## Architecture

```
+-------------------------------------------------------+
| L6: Tools                                              |
| tmux-builder | tmux-vm | tmux-sniff | mux-doctor       |
+-------------------------------------------------------+
| L5: SDK + Integration                                  |
| mux-api | mux-test-support | mux-ffi | mux-otel        |
+-------------------------------------------------------+
| L4: Kernel + Termlet                                   |
| mux-kernel | mux-termlet                                |
+-------------------------------------------------------+
| L3: Snapshot + Parsing + Render + ORM + PTY             |
| mux-snapshot | mux-cmd-parse | mux-orm | mux-render |   |
| mux-pty                                                |
+-------------------------------------------------------+
| L2: Grid + Parser + Protocol + Config                  |
| mux-grid | mux-parser | mux-proto | mux-options |      |
| mux-target | mux-format | mux-config                   |
+-------------------------------------------------------+
| L1: Types                                              |
| mux-types                                              |
+-------------------------------------------------------+
| L0: Leaf (zero internal deps)                          |
| mux-grapheme-arena | mux-time                          |
+-------------------------------------------------------+
```

## Thread Model

```
IO Thread (tokio)  <--crossbeam-->  Kernel Thread (std)  --snapshot-->  Render Thread
  PTY FDs                           ALL mutable state                  CompositeGrid
  Unix socket                       Entity model                       Dirty-line diff
  signal-hook                       Layout engine                      Output coalesce
                                    Copy mode
```

## Workspace Layout

```
termforge/
  Cargo.toml          # Workspace manifest
  CLAUDE.md           # Coding conventions
  AGENTS.md           # Quality gates
  notes/
    architecture.md   # Full architecture document
    plan.md           # Implementation plan
  crates/
    mux-api/          # SDK entry-point (builder API)
    mux-types/        # Core types (Cell, Colour, Attrs, IDs)
    mux-grid/         # Chunked COW grid with dirty tracking
    mux-parser/       # VT state machine parser
    mux-proto/        # TF01 + tmux imsg wire protocol
    mux-kernel/       # Sans-IO kernel (entity model, layout, copy mode)
    mux-render/       # Composition pipeline with diff rendering
    mux-pty/          # PTY allocation + process management (unsafe quarantine)
    mux-config/       # TOML + tmux.conf config system
    mux-options/      # Typed option system with inheritance
    mux-format/       # Format string parser/evaluator
    mux-target/       # Target syntax parser (session:window.pane)
    mux-cmd-parse/    # Command parser
    mux-snapshot/     # Grid snapshot extraction
    mux-orm/          # ORM-like traversal API
    mux-termlet/      # Testing pods (PTY + VT + shell)
    mux-test-support/ # Test harness and fixtures
    mux-grapheme-arena/ # Grapheme cluster arena allocator
    mux-time/         # Injectable time source
    mux-ffi/          # C FFI shim
    mux-otel/         # OpenTelemetry integration
  tools/
    tmux-builder/     # Workspace layout builder
    tmux-vm/          # Headless test VM
    tmux-sniff/       # Wire protocol sniffer
    mux-doctor/       # Diagnostic tool
```

## Design Principles

1. **SDK-first**: `mux-api` is the primary entry point for embedding.
2. **Sans-IO kernel**: Deterministic, testable, no I/O in the kernel thread.
3. **3-thread model**: IO (tokio), Kernel (std::thread), Render (tokio task).
4. **tmux compatibility**: Wire protocol, commands, options, targets, formats.
5. **Zero unsafe outside quarantine**: Only `mux-pty` and `mux-ffi`.

## Quick Start

```bash
cargo check     # Type-check all 21 crates + 4 tools
cargo test      # Run all tests
```
