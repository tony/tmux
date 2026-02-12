# TermForge Implementation Plan

## Current State (Pass 2 Scaffold)

The scaffold contains 12 crates with 184-191 passing tests (depending on feature flags).
All crates compile, all tests pass, and the architecture matches the v15 DEFINITIVE spec.

This document outlines the path from scaffold to production-ready implementation.

## Phase 1: Core Grid + Parser Integration

**Goal**: Wire the VT parser into the Grid so that byte sequences produce visible cells.

### Step 1.1: Parser-Grid Bridge
- **Files**: `crates/mux-grid/src/lib.rs`, new `crates/mux-grid/src/terminal.rs`
- **Changes**: Create a `Terminal` struct that owns a `Grid`, a `Parser`, and a `GraphemeArena`.
  Feed bytes through the parser; dispatch `Action` variants to grid mutations.
- **Tests**: Feed known VT sequences (e.g., `\x1b[2J` for clear, `\x1b[10;20H` for cursor
  movement) and verify grid state via `GridSnapshot`.
- **Depends on**: Nothing (L0 + L1 crates already exist).

### Step 1.2: Scrollback Integration
- **Files**: `crates/mux-grid/src/terminal.rs`, `crates/mux-grid/src/scrollback.rs`
- **Changes**: When the terminal scrolls, push lines into Scrollback via Arc COW.
  Verify that scrollback lines share Arc references with original grid lines.
- **Tests**: Fill a 24-line terminal with 50 lines of output, verify scrollback has 26 lines.

### Step 1.3: Wide Character + Grapheme Cluster Support
- **Files**: `crates/mux-grid/src/terminal.rs`, `crates/mux-grapheme-arena/src/lib.rs`
- **Changes**: Handle CJK wide characters (width=2), combining marks, and multi-codepoint
  grapheme clusters. Use GraphemeArena for clusters that exceed CompactString inline capacity.
- **Tests**: Feed CJK text, emoji with ZWJ sequences, verify cell widths and arena allocation.

## Phase 2: Kernel Wiring

**Goal**: Connect the Kernel's Event/Effect reducer to actual I/O.

### Step 2.1: I/O Runtime
- **Files**: New `crates/mux-runtime/src/lib.rs`
- **Changes**: Create an async runtime (tokio-based) that:
  1. Reads from PTY file descriptors -> `Event::PtyOutput`
  2. Reads from client sockets -> `Event::ClientInput`
  3. Calls `kernel.step(event)` -> gets `Vec<Effect>`
  4. Executes each Effect (write to PTY, send to client, spawn process, etc.)
- **Tests**: Mock PTY and client connections, verify round-trip.

### Step 2.2: Session/Window/Pane Lifecycle
- **Files**: `crates/mux-kernel/src/lib.rs`
- **Changes**: Flesh out the Kernel's `step()` for full lifecycle:
  - Attach/detach clients to sessions
  - Window navigation (next/prev/last)
  - Pane splitting (horizontal/vertical)
  - Pane focus management
- **Tests**: Create session -> window -> 3 panes, verify graph structure.

### Step 2.3: Status Line
- **Files**: `crates/mux-kernel/src/lib.rs`, new `crates/mux-kernel/src/status.rs`
- **Changes**: On `Event::Tick`, render a status line based on session/window state.
  Emit `Effect::SendClient` with the rendered status bytes.
- **Tests**: Verify status line content updates when windows are created/renamed.

## Phase 3: PTY + Process Management

**Goal**: Real PTY allocation and process spawning.

### Step 3.1: PTY Syscalls
- **Files**: `crates/mux-pty/src/lib.rs`
- **Changes**: Replace stub implementations with real `openpty()`, `forkpty()`, and
  `ioctl(TIOCSWINSZ)` calls. This is the ONLY crate that uses `unsafe`.
- **Tests**: Spawn `/bin/echo hello`, read output, verify bytes.

### Step 3.2: Typestate Enforcement
- **Files**: `crates/mux-pty/src/lib.rs`
- **Changes**: Implement actual state transitions (Created -> Opening -> Open, etc.)
  with real file descriptors.
- **Tests**: Verify that double-close returns error, write-after-close returns error.

## Phase 4: Wire Protocol

**Goal**: Full tmux wire protocol v8 compatibility.

### Step 4.1: Protocol Encoding/Decoding
- **Files**: `crates/mux-proto/src/lib.rs`
- **Changes**: Implement full TLV frame parsing with streaming support (partial frames).
  Handle backpressure and frame size limits.
- **Tests**: Fuzz the decoder with random bytes. Verify round-trip for all 9 frame types.

### Step 4.2: Control Mode
- **Files**: New `crates/mux-proto/src/control.rs`
- **Changes**: Implement tmux control mode (`-C` flag) protocol.
  Parse `%begin`, `%end`, `%output`, `%session-changed`, etc.
- **Tests**: Feed captured tmux control mode output, verify parsed events.

### Step 4.3: Compatibility Baseline
- **Files**: `crates/mux-test-support/src/lib.rs`
- **Changes**: Build differential tests that run the same commands against both
  TermForge and a real tmux binary, comparing output byte-by-byte.
- **Tests**: Use mux-vm to test against tmux 3.3a, 3.4, 3.5, and HEAD.

## Phase 5: Snapshot + ORM

**Goal**: Production-quality snapshot format and ORM traversal.

### Step 5.1: Snapshot Format
- **Files**: `crates/mux-snapshot/src/lib.rs`
- **Changes**: Implement full snapshot encode/decode with grid data, scrollback,
  cursor state, and GraphemeArena. Delta snapshots for efficient saves.
- **Tests**: Round-trip a full terminal state. Verify CRC32C catches corruption.

### Step 5.2: ORM Integration
- **Files**: `crates/mux-orm/src/lib.rs`, `crates/mux-kernel/src/lib.rs`
- **Changes**: Connect QueryList to ServerGraph. Implement:
  ```rust
  server.sessions().filter(|s| s.name.contains("dev"))
      .windows().filter(|w| w.is_active())
      .panes().first()
  ```
- **Tests**: Create complex graph, verify filter chains.

## Phase 6: Termlets + Testing SDK

**Goal**: Production Termlet runtime for testing.

### Step 6.1: Termlet Runtime
- **Files**: `crates/mux-termlet/src/lib.rs`
- **Changes**: Implement actual process spawning, PTY allocation, and grid capture.
  Termlets are lightweight panes with simplified lifecycle management.
- **Tests**: Spawn a Termlet running `echo hello`, capture grid, verify snapshot.

### Step 6.2: Snapshot Testing
- **Files**: New `crates/mux-termlet/src/snapshot.rs`
- **Changes**: Integrate with `insta` for snapshot testing. Capture grid state as
  text representation, compare against stored snapshots.
- **Tests**: Snapshot tests for common terminal sequences (clear, scroll, color).

## Phase 7: Language Bindings

**Goal**: Python and Node.js bindings with ORM-like traversal.

### Step 7.1: Python Bindings (PyO3)
- **Files**: New `bindings/python/` workspace member
- **Changes**: Expose Server, Session, Window, Pane, Termlet via PyO3.
  QueryList becomes a Python iterable with `.filter()` method.
  Snapshot comparison integrates with pytest-insta or similar.
- **Tests**: pytest tests that spawn Termlets and do snapshot comparison.

### Step 7.2: Node.js Bindings (napi-rs)
- **Files**: New `bindings/node/` workspace member
- **Changes**: Expose the same API surface via napi-rs.
  Integrate with vitest for snapshot testing.
- **Tests**: vitest tests with Termlet lifecycle.

## Phase 8: OpenTelemetry + Observability

**Goal**: Cross-service tracing.

### Step 8.1: OTEL Integration
- **Files**: New `crates/mux-otel/src/lib.rs`
- **Changes**: Map `Effect::Log` to OTEL spans/events. Propagate trace context
  through the wire protocol (add trace ID to handshake frames).
- **Tests**: Verify span creation for session lifecycle events.

## Phase 9: tmux Version Management

**Goal**: Build and manage tmux binaries for compatibility testing.

### Step 9.1: mux-vm (Version Manager)
- **Files**: New `crates/mux-vm/src/lib.rs`
- **Changes**: Download, compile, and manage tmux source at specific versions.
  Cache compiled binaries. Support version ranges (3.3a, 3.4, 3.5, HEAD).
- **Tests**: Build tmux 3.4, verify binary runs.

### Step 9.2: mux-builder
- **Files**: New `crates/mux-builder/src/lib.rs`
- **Changes**: Automate tmux compilation with custom configure flags.
  Support cross-compilation for CI environments.
- **Tests**: Build and run tmux with custom flags.

## Phase 10: Client + Visualization

**Goal**: Native Rust client with tmux-compatible rendering.

### Step 10.1: Client
- **Files**: New `crates/mux-client/src/lib.rs`
- **Changes**: Terminal client that connects to the server, renders grid,
  handles input. Optionally uses ratatui or custom renderer.
- **Tests**: Headless client tests with mock server.

### Step 10.2: Binary
- **Files**: New `src/main.rs`
- **Changes**: `termforge` binary with tmux-compatible CLI flags.
  `termforge new-session`, `termforge attach`, etc.
- **Tests**: Integration tests running the binary.

## Risk Register

| Risk | Impact | Mitigation |
|------|--------|------------|
| VT parser edge cases | High | Fuzz testing, differential testing against real tmux |
| Wide char rendering | Medium | Test with CJK, emoji, combining marks across tmux versions |
| PTY race conditions | High | Typestate pattern prevents invalid states at compile time |
| Wire protocol drift | Medium | Pin to protocol v8, test against tmux version ranges |
| Feature flag interactions | Low | CI matrix tests all feature combinations |
| Language binding ABI stability | Medium | Semantic versioning, integration tests in CI |
