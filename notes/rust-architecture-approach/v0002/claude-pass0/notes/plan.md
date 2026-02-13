# TermForge Implementation Plan

## Overview

Total estimated LOC: ~25,000 Rust LOC across 19 crates + 4 tools.
Timeline: 4 phases over ~12 weeks for a single focused engineer.

## Phase 0: Foundation (Current -- Complete)

**Status**: DONE. This scaffold.

| Deliverable | Status | LOC |
|-------------|--------|-----|
| Workspace Cargo.toml | Done | 120 |
| 19 crate stubs with real types | Done | ~3,500 |
| 4 tool stubs | Done | ~100 |
| CLAUDE.md, AGENTS.md, README.md | Done | ~500 |
| notes/architecture.md | Done | ~400 |
| notes/plan.md | Done | ~200 |
| `cargo check` passes | Done | -- |

**Phase 0 Total**: ~4,800 LOC scaffold

---

## Phase 1: Core Engine (Weeks 1-4)

Goal: Complete grid, parser, PTY, and kernel to the point where a single
pane can spawn a shell, accept input, and produce visible output.

### P1.1: Grid Completion (~800 LOC)
- [ ] Full scroll region implementation (chunk-boundary crossing)
- [ ] Reflow with content-aware line wrapping
- [ ] Alternate screen buffer (DECSET 1049)
- [ ] Line attributes (wrapped flag, shell prompt mark)
- [ ] Grid-level resize with scrollback recovery
- [ ] Benchmark: create/mutate/snapshot cycle

**Crate**: `mux-grid`
**Depends on**: mux-types, mux-grapheme-arena, bitvec

### P1.2: VT Parser Completion (~2,000 LOC)
- [ ] Full state machine (all 14 states)
- [ ] CSI dispatch for cursor movement (CUU, CUD, CUF, CUB, CUP, HPA, VPA)
- [ ] CSI dispatch for erase (ED, EL, ECH)
- [ ] CSI dispatch for scroll (SU, SD, DECSTBM)
- [ ] CSI dispatch for modes (DECRST, DECSET)
- [ ] CSI dispatch for SGR (all attributes, 256-colour, true colour)
- [ ] OSC dispatch (title, hyperlinks, clipboard)
- [ ] DCS passthrough
- [ ] UTF-8 multi-byte handling
- [ ] Fuzz target for arbitrary byte sequences

**Crate**: `mux-parser`
**Depends on**: mux-types

### P1.3: PTY Implementation (~600 LOC)
- [ ] `spawn_child()`: fork + setsid + peer setup + exec
- [ ] TIOCGPTPEER optimization (Linux)
- [ ] Non-blocking I/O setup
- [ ] SIGCHLD handler for zombie reaping
- [ ] Window size change (TIOCSWINSZ on resize)
- [ ] Integration test: spawn /bin/echo, read output

**Crate**: `mux-pty`
**Depends on**: mux-types, nix, libc

### P1.4: Kernel Core (~2,500 LOC)
- [ ] Session/Window/Pane CRUD operations
- [ ] PTY data processing (VT parser -> grid)
- [ ] Full CSI action mapping (cursor, erase, scroll, SGR)
- [ ] Key binding table and dispatch
- [ ] Prefix key handling (Ctrl-B + key)
- [ ] Command dispatch table (new-session, new-window, split-window, etc.)
- [ ] Client attach/detach
- [ ] Window/pane selection and navigation
- [ ] Environment variable inheritance
- [ ] Integration test: create session, send keys, verify grid content

**Crate**: `mux-kernel`
**Depends on**: mux-types, mux-grid, mux-grapheme-arena, mux-parser,
mux-options, mux-target, mux-format, mux-cmd-parse, mux-time, mux-snapshot, mux-orm

### P1.5: Protocol Core (~400 LOC)
- [ ] TF01 tokio codec (Encoder/Decoder)
- [ ] Frame serialization for all frame types
- [ ] imsg header parsing for compatibility
- [ ] Property tests for encode/decode round-trips

**Crate**: `mux-proto`
**Depends on**: mux-types, bytes, crc32fast

**Phase 1 Total**: ~6,300 LOC
**Phase 1 Gate**: `cargo test` passes; single-pane spawn+echo works in integration test.

---

## Phase 2: Multiplexing & Rendering (Weeks 5-8)

Goal: Multiple panes with layouts, rendering pipeline, and full tmux
command set for session/window/pane management.

### P2.1: Layout Engine (~800 LOC)
- [ ] Even-horizontal, even-vertical, main-horizontal, main-vertical, tiled
- [ ] Custom layout string parsing (tmux layout checksum format)
- [ ] Resize redistribution
- [ ] Pane splitting (horizontal and vertical)
- [ ] Pane swapping and rotation

**Crate**: `mux-kernel` (layout submodule)

### P2.2: Render Pipeline (~1,200 LOC)
- [ ] Full dirty-line rendering
- [ ] Attribute change coalescing
- [ ] Cursor movement optimization
- [ ] Border drawing (Unicode box characters)
- [ ] Status line rendering
- [ ] Pane title/border active indicators
- [ ] Double-buffering and output coalescing

**Crate**: `mux-render`
**Depends on**: mux-types, mux-grid, mux-grapheme-arena, mux-snapshot

### P2.3: Command System (~1,500 LOC)
- [ ] Full command table (50+ tmux commands)
- [ ] new-session, new-window, split-window, kill-pane, kill-window, kill-session
- [ ] select-window, select-pane, last-window, last-pane
- [ ] resize-pane, swap-pane, rotate-window
- [ ] set-option, show-options (all scopes)
- [ ] send-keys, send-prefix
- [ ] display-message, display-panes
- [ ] list-sessions, list-windows, list-panes, list-clients
- [ ] source-file (config loading)
- [ ] if-shell, run-shell
- [ ] bind-key, unbind-key

**Crate**: `mux-kernel` (command submodule), `mux-cmd-parse`

### P2.4: Mouse Support (~500 LOC)
- [ ] SGR mouse reporting
- [ ] Pane selection on click
- [ ] Border drag for resize
- [ ] Scroll wheel handling
- [ ] Mouse mode options (mouse on/off)

**Crate**: `mux-kernel` (mouse submodule)

### P2.5: Copy Mode (~600 LOC)
- [ ] Vi and emacs key bindings
- [ ] Selection marking (character, line, block)
- [ ] Copy to buffer
- [ ] Paste from buffer
- [ ] Incremental search in scrollback

**Crate**: `mux-kernel` (copy submodule)

**Phase 2 Total**: ~4,600 LOC
**Phase 2 Gate**: Multiple panes render correctly; tmux commands work;
copy-paste functional.

---

## Phase 3: Wire Protocol & IO (Weeks 9-10)

Goal: Full client-server communication over Unix sockets, supporting
both TF01 native protocol and tmux imsg compatibility.

### P3.1: IO Thread (~1,200 LOC)
- [ ] tokio event loop: PTY read/write, socket accept
- [ ] Channel bridge: KernelEvent/KernelEffect
- [ ] Signal handling (SIGWINCH, SIGCHLD, SIGTERM)
- [ ] Socket permission setup
- [ ] Client connection lifecycle

**New crate or in `mux-kernel`**: IO thread implementation

### P3.2: TF01 Client Protocol (~800 LOC)
- [ ] Client handshake (Identify/Ready)
- [ ] Input forwarding (KeyInput, MouseInput, Resize)
- [ ] Render frame reception and terminal output
- [ ] Ping/pong keepalive

### P3.3: tmux imsg Compatibility (~600 LOC)
- [ ] imsg message parsing for common message types
- [ ] Protocol translation layer (imsg -> KernelEvent)
- [ ] Response translation (KernelEffect -> imsg)
- [ ] attach-session, detach-session compatibility

### P3.4: Server Binary (~400 LOC)
- [ ] Main entry point: parse args, start threads, run event loop
- [ ] Socket path management
- [ ] Daemonization
- [ ] Config file loading

**Phase 3 Total**: ~3,000 LOC
**Phase 3 Gate**: `termforge` binary starts, accepts client connections,
renders terminal output.

---

## Phase 4: Polish & Ecosystem (Weeks 11-12)

Goal: Testing infrastructure, FFI, observability, and tool binaries.

### P4.1: Termlet Completion (~500 LOC)
- [ ] Full PTY lifecycle in termlet
- [ ] `send_keys()` implementation
- [ ] `wait_for()` with timeout
- [ ] `capture()` with style information
- [ ] Resize support

**Crate**: `mux-termlet`

### P4.2: Test Support Completion (~400 LOC)
- [ ] Full TestServer with session/window/pane creation
- [ ] Property test strategies for grid operations
- [ ] Snapshot comparison utilities
- [ ] CI pipeline configuration

**Crate**: `mux-test-support`

### P4.3: FFI Completion (~300 LOC)
- [ ] Grid manipulation functions
- [ ] Parser feed functions
- [ ] Command execution through FFI
- [ ] C header generation (cbindgen)

**Crate**: `mux-ffi`

### P4.4: OTEL Completion (~400 LOC)
- [ ] OTLP exporter setup
- [ ] Span hierarchy
- [ ] Metrics: grid mutation rate, COW triggers, render latency
- [ ] Configuration via `.termforge/otel.toml`

**Crate**: `mux-otel`

### P4.5: Tools (~600 LOC)
- [ ] tmux-builder: YAML/TOML workspace config -> session creation
- [ ] tmux-vm: headless kernel for CI testing
- [ ] tmux-sniff: protocol traffic dumper
- [ ] mux-doctor: diagnostic health checker

**Phase 4 Total**: ~2,200 LOC
**Phase 4 Gate**: All tests pass; tools functional; OTEL traces visible
in Jaeger/Grafana.

---

## LOC Summary

| Phase | Description | Estimated LOC |
|-------|-------------|---------------|
| P0    | Foundation (scaffold) | 4,800 |
| P1    | Core Engine | 6,300 |
| P2    | Multiplexing & Rendering | 4,600 |
| P3    | Wire Protocol & IO | 3,000 |
| P4    | Polish & Ecosystem | 2,200 |
| **Total** | | **20,900** |

Plus tests (~25-30% additional): ~5,000-6,000 LOC.
**Grand total**: ~25,000-27,000 LOC.

## LOC Per Crate (Estimated Final)

| Crate | Estimated LOC | Phase |
|-------|---------------|-------|
| mux-grapheme-arena | 200 | P0 |
| mux-time | 150 | P0 |
| mux-types | 350 | P0 |
| mux-grid | 1,200 | P0+P1 |
| mux-parser | 2,500 | P0+P1 |
| mux-proto | 800 | P0+P1+P3 |
| mux-options | 400 | P0+P2 |
| mux-target | 400 | P0 |
| mux-format | 500 | P0+P2 |
| mux-snapshot | 300 | P0 |
| mux-cmd-parse | 800 | P0+P2 |
| mux-orm | 400 | P0 |
| mux-render | 1,500 | P0+P2 |
| mux-pty | 800 | P0+P1 |
| mux-kernel | 7,600 | P0+P1+P2 |
| mux-termlet | 600 | P0+P4 |
| mux-test-support | 500 | P0+P4 |
| mux-ffi | 400 | P0+P4 |
| mux-otel | 500 | P0+P4 |
| Tools (4) | 1,000 | P4 |
| **Total** | **~20,900** | |

## Risk Register

| Risk | Mitigation |
|------|------------|
| VT parser complexity | Fuzz testing from day 1; reference tmux's input.c |
| tmux imsg compat | Protocol sniffer tool; record/replay testing |
| Layout engine edge cases | Property testing with random resize sequences |
| Wide char/emoji rendering | Comprehensive grapheme cluster test suite |
| Performance (render latency) | Criterion benchmarks; dirty-line optimization |
| PTY portability | nix crate abstractions; CI on Linux + macOS |
