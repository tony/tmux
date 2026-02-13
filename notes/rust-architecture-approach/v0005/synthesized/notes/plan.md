# TermForge v0005 Implementation Plan

## Table of Contents

1. [Plan Overview](#1-plan-overview)
2. [Phase 0: Foundation (Current State)](#2-phase-0-foundation)
3. [Phase 1: Core Types and Grid](#3-phase-1-core-types-and-grid)
4. [Phase 2: VT Parser Completion](#4-phase-2-vt-parser-completion)
5. [Phase 3: Kernel Internals](#5-phase-3-kernel-internals)
6. [Phase 4: PTY Integration](#6-phase-4-pty-integration)
7. [Phase 5: Rendering Pipeline](#7-phase-5-rendering-pipeline)
8. [Phase 6: Configuration and Commands](#8-phase-6-configuration-and-commands)
9. [Phase 7: Wire Protocol](#9-phase-7-wire-protocol)
10. [Phase 8: SDK API and Language Bindings](#10-phase-8-sdk-api-and-language-bindings)
11. [Phase 9: Testing Infrastructure](#11-phase-9-testing-infrastructure)
12. [Phase 10: tmux Compatibility](#12-phase-10-tmux-compatibility)
13. [Milestone Summary](#13-milestone-summary)
14. [Dependency Order](#14-dependency-order)
15. [Risk Register](#15-risk-register)
16. [Acceptance Criteria](#16-acceptance-criteria)
17. [Crate-by-Crate Implementation Status](#17-crate-by-crate-status)
18. [Test Coverage Plan](#18-test-coverage-plan)
19. [Documentation Plan](#19-documentation-plan)
20. [CI/CD Pipeline Plan](#20-cicd-pipeline-plan)
21. [Benchmark Plan](#21-benchmark-plan)
22. [Fuzz Testing Plan](#22-fuzz-testing-plan)
23. [Differential Testing Plan](#23-differential-testing-plan)
24. [Language Binding Plan](#24-language-binding-plan)
25. [Version Management Plan](#25-version-management-plan)
26. [OpenTelemetry Integration Plan](#26-opentelemetry-integration-plan)
27. [CRDT Integration Plan](#27-crdt-integration-plan)
28. [Migration Guide](#28-migration-guide)

---

## 1. Plan Overview

### 1.1 Goals

The TermForge implementation plan covers the path from the current scaffold
(v0005) to a functional terminal multiplexer SDK. The plan is organized into
11 phases (0-10), each building on the previous.

### 1.2 Current State

The v0005 scaffold provides:
- **23 library crates** with real implementations (not stubs).
- **4 tool binaries** with placeholder main functions.
- **1,100+ tests** across all crates.
- **135+ proptest** references for property-based testing.
- **36 integration test files** in `crates/*/tests/`.
- **Complete type system** in mux-types (8 modules).
- **14-state VT parser** with CSI dispatch and grid integration.
- **Grid data structure** with Arc-COW lines and scrollback.
- **Sans-IO kernel** with event/effect dispatch.
- **Double-buffer diff renderer** with composite support.
- **CRDT primitives** with verified convergence.
- **Wire protocol** frame model with CRC32.
- **ORM QuerySet** API with filter/first/get/count.
- **Termlet** testing pods with state machine.
- **FFI surface** with catch_unwind panic recovery.
- **OpenTelemetry** span wrapper.

### 1.3 What Remains

To reach a functional multiplexer:
1. Wire the kernel to actual PTY processes.
2. Implement the async I/O layer with tokio.
3. Complete the VT parser for all common sequences.
4. Build the client-server protocol.
5. Implement the tmux command set.
6. Build the terminal UI client.
7. Create language bindings.
8. Set up differential testing against tmux.

### 1.4 Guiding Principles

- **Bottom-up**: Build lower layers first, verify with tests, then integrate.
- **One crate at a time**: Each phase focuses on specific crates.
- **Test-driven**: Every feature starts with a failing test.
- **Incremental**: Each phase produces a testable increment.
- **tmux-compatible**: Every behavioral decision checks tmux for reference.

---

## 2. Phase 0: Foundation (Current State)

### 2.1 Completed Work

#### 2.1.1 Workspace Configuration

- [x] Cargo.toml with 23 crates + 4 tools
- [x] Edition 2024, MSRV 1.85
- [x] Clippy configuration (warn all/pedantic/nursery/cargo)
- [x] Safety denials (unwrap_used, expect_used, panic)
- [x] Profile configuration (panic = "unwind" for catch_unwind)
- [x] Workspace dependencies centralized
- [x] Workspace lints configured

#### 2.1.2 Layer 0: Leaf Crates

- [x] mux-grapheme-arena: String interning with GraphemeId
- [x] mux-time: MonotonicClock, Timestamp, TimerWheel

#### 2.1.3 Layer 1: Core Types

- [x] mux-types/attrs.rs: Attrs and CellFlags bitflags
- [x] mux-types/cell.rs: Cell struct with all fields
- [x] mux-types/colour.rs: Colour enum (Default, Indexed, Rgb)
- [x] mux-types/error.rs: MuxError enum
- [x] mux-types/geometry.rs: Size, Rect, Position
- [x] mux-types/id.rs: SessionId, WindowId, PaneId with IdGenerator
- [x] mux-types/key.rs: Key, NamedKey, Modifiers, KeyEvent
- [x] mux-types/style.rs: StyleDelta

#### 2.1.4 Layer 2: Data Structures

- [x] mux-grid: ChunkedGrid with Arc-COW lines
- [x] mux-parser: 14-state VT parser + CSI dispatch + InputParser
- [x] mux-options: OptionTable with scoped resolution
- [x] mux-target: Target syntax parser
- [x] mux-format: Format string engine
- [x] mux-cmd-parse: Command tokenizer
- [x] mux-proto: Wire protocol frame model
- [x] mux-crdt: VectorClock + LwwRegister
- [x] mux-fdpass: FdEnvelope for SCM_RIGHTS

#### 2.1.5 Layer 3: Business Logic

- [x] mux-kernel: Sans-IO kernel with event/effect dispatch
- [x] mux-kernel/layout.rs: Binary tree layout engine
- [x] mux-kernel/copy_mode.rs: Copy mode state machine
- [x] mux-kernel/event.rs: KernelEvent and KernelEffect enums
- [x] mux-render: Double-buffer diff renderer + CompositeBuffer
- [x] mux-snapshot: GridSnapshot with Arc-COW capture
- [x] mux-orm: QuerySet with filter/first/get/count
- [x] mux-pty: PTY model (PtyPair, SpawnContext, PtyError)

#### 2.1.6 Layer 4: Integration

- [x] mux-api: ServerBuilder + Server
- [x] mux-config: Configuration loading from tmux format
- [x] mux-termlet: Termlet with Created/Running/Stopped states
- [x] mux-test-support: TestServer, CleanupGuard, isolated_socket_path

#### 2.1.7 Layer 5: External Surface

- [x] mux-ffi: C FFI surface with catch_unwind
- [x] mux-otel: OpenTelemetry span wrapper

#### 2.1.8 Layer 6: Tools

- [x] tmux-builder: Placeholder main
- [x] tmux-vm: Placeholder main
- [x] tmux-sniff: Placeholder main
- [x] mux-doctor: Placeholder main

#### 2.1.9 Testing

- [x] 1,100+ unit tests
- [x] 135+ proptest references
- [x] 36 integration test files
- [x] Property tests for all encoding roundtrips
- [x] Property tests for CRDT convergence
- [x] Socket isolation in test support

#### 2.1.10 Documentation

- [x] CLAUDE.md: Project conventions
- [x] AGENTS.md: Rule catalog and agent guidelines
- [x] notes/architecture.md: Architecture document
- [x] notes/plan.md: This document

### 2.2 Known Gaps

- [ ] mux-kernel/session.rs is a stub
- [ ] mux-kernel/window.rs is a stub
- [ ] mux-kernel/pane.rs is a stub
- [ ] No actual PTY syscalls (mux-pty is a model)
- [ ] No tokio runtime wiring
- [ ] No actual signal handling
- [ ] Tool binaries are placeholders
- [ ] No benchmark targets
- [ ] No fuzz targets
- [ ] No differential tests
- [ ] No language bindings

---

## 3. Phase 1: Core Types and Grid

### 3.1 Objective

Ensure the type system and grid are complete and battle-tested.

### 3.2 Tasks

#### 3.2.1 mux-types Completion

- [ ] Add Hyperlink type for OSC 8 support
- [ ] Add TerminalMode enum (application cursor, bracketed paste, etc.)
- [ ] Add MouseEvent struct (button, position, modifiers)
- [ ] Add PaneMode enum (Normal, CopyMode, CommandMode)
- [ ] Verify INV-119: CellFlags match tmux GRID_FLAG_* with const assertions
- [ ] Add Display implementations for all types
- [ ] Add From/Into conversions between related types

#### 3.2.2 mux-grid Hardening

- [ ] Implement tab stop handling in write_char
- [ ] Implement line wrapping flag (CellFlags::WRAPPED)
- [ ] Implement alternate screen buffer (primary/alternate switch)
- [ ] Add scrollback pruning when limit reached
- [ ] Implement insert_chars and delete_chars for ICH/DCH
- [ ] Add saved cursor position (DECSC/DECRC)
- [ ] Test wide character overwrite edge cases
- [ ] Fuzz test grid operations with random sequences

#### 3.2.3 mux-grapheme-arena Optimization

- [ ] Add arena compaction (remove unreferenced entries)
- [ ] Add memory usage reporting
- [ ] Benchmark intern() performance
- [ ] Add WASM compilation test

### 3.3 Acceptance Criteria

- All mux-types types have Display implementations.
- Grid handles tab stops correctly.
- Alternate screen buffer switches work.
- Scrollback limit is enforced.
- All new features have proptest coverage.

### 3.4 Estimated Effort

2-3 weeks for a single developer.

---

## 4. Phase 2: VT Parser Completion

### 4.1 Objective

Complete the VT parser to handle all common terminal escape sequences.

### 4.2 Tasks

#### 4.2.1 State Machine Hardening

- [ ] Verify all 14 x 256 transitions against Paul Williams' spec
- [ ] Add fuzz target for state machine
- [ ] Benchmark step() per-byte throughput
- [ ] Verify CLASS_TABLE[256] against classify_match() oracle

#### 4.2.2 CSI Dispatch Completion

- [ ] Implement all cursor movement: CUU, CUD, CUF, CUB, CUP, HVP
- [ ] Implement erase: ED (0, 1, 2, 3), EL (0, 1, 2)
- [ ] Implement insert/delete: IL, DL, ICH, DCH
- [ ] Implement scroll: SU, SD
- [ ] Implement DECSTBM (set scroll region)
- [ ] Implement DECSC/DECRC (save/restore cursor)
- [ ] Implement DECSET/DECRST for:
  - [ ] Mode 1 (DECCKM - cursor keys)
  - [ ] Mode 7 (DECAWM - autowrap)
  - [ ] Mode 12 (cursor blink)
  - [ ] Mode 25 (cursor visible)
  - [ ] Mode 47/1047 (alternate screen)
  - [ ] Mode 1000/1002/1003/1006 (mouse modes)
  - [ ] Mode 1049 (alternate screen + save cursor)
  - [ ] Mode 2004 (bracketed paste)
- [ ] Implement full SGR with all parameters

#### 4.2.3 OSC Dispatch

- [ ] Implement OSC 0: Set window title
- [ ] Implement OSC 2: Set window title
- [ ] Implement OSC 7: Working directory
- [ ] Implement OSC 8: Hyperlinks
- [ ] Implement OSC 52: Clipboard

#### 4.2.4 DCS Passthrough

- [ ] Implement DCS passthrough piping
- [ ] Add DCS payload size limit (16 MiB)
- [ ] Test Sixel data passthrough

#### 4.2.5 InputParser Integration

- [ ] Wire all CSI dispatches to grid operations
- [ ] Wire OSC dispatches to metadata updates
- [ ] Add attribute state tracking (current fg, bg, attrs)
- [ ] Implement SGR push/pop

### 4.3 Acceptance Criteria

- All common escape sequences parsed correctly.
- `cat /etc/motd`, `ls --color`, `vim`, `htop` produce correct grid state.
- Fuzz target runs for 1M iterations without crash.
- Proptest verifies state machine determinism.

### 4.4 Estimated Effort

3-4 weeks.

---

## 5. Phase 3: Kernel Internals

### 5.1 Objective

Complete the kernel's internal state management for sessions, windows, panes.

### 5.2 Tasks

#### 5.2.1 Session Model (mux-kernel/session.rs)

- [ ] Implement KernelSession with:
  - [ ] name: String
  - [ ] windows: Vec<WindowId>
  - [ ] active_window: WindowId
  - [ ] created_at: Timestamp
  - [ ] options: OptionTable (session scope)
- [ ] Implement session creation with default window
- [ ] Implement session destruction (cascade to windows/panes)
- [ ] Implement session rename
- [ ] Implement session listing

#### 5.2.2 Window Model (mux-kernel/window.rs)

- [ ] Implement KernelWindow with:
  - [ ] name: String
  - [ ] session_id: SessionId
  - [ ] panes: Vec<PaneId>
  - [ ] active_pane: PaneId
  - [ ] layout: LayoutNode
  - [ ] options: OptionTable (window scope)
- [ ] Implement window creation with default pane
- [ ] Implement window destruction (cascade to panes)
- [ ] Implement window rename
- [ ] Implement window split (horizontal/vertical)
- [ ] Implement active pane cycling (next/previous)

#### 5.2.3 Pane Model (mux-kernel/pane.rs)

- [ ] Implement KernelPane with:
  - [ ] grid: ChunkedGrid
  - [ ] parser: InputParser
  - [ ] arena: GraphemeArena (or shared reference)
  - [ ] size: Size
  - [ ] pid: Option<u32>
  - [ ] title: String
  - [ ] mode: PaneMode
  - [ ] copy_state: Option<CopyModeState>
- [ ] Implement pane creation
- [ ] Implement pane destruction (with PTY cleanup effect)
- [ ] Implement pane resize
- [ ] Implement data feeding (PTY output -> parser -> grid)
- [ ] Implement capture-pane (snapshot extraction)

#### 5.2.4 Event Dispatch Completion

- [ ] Handle all KernelEvent variants
- [ ] Produce correct KernelEffect for each event
- [ ] Implement key routing (prefix key detection, command dispatch)
- [ ] Implement mode switching (normal -> copy mode -> normal)

#### 5.2.5 Layout Integration

- [ ] Wire layout engine to window operations
- [ ] Implement split operations that update the layout tree
- [ ] Implement resize propagation through the layout tree
- [ ] Implement zoom (single pane fills window)

### 5.3 Acceptance Criteria

- Can create server -> session -> window -> pane programmatically.
- Data fed to a pane appears in the correct grid.
- Window splits produce correct layout geometry.
- Session destruction cascades correctly.
- All operations produce correct effects.

### 5.4 Estimated Effort

4-5 weeks.

---

## 6. Phase 4: PTY Integration

### 6.1 Objective

Wire the kernel to actual PTY processes via the async I/O layer.

### 6.2 Tasks

#### 6.2.1 PTY Syscalls (mux-pty)

- [ ] Implement posix_openpt + grantpt + unlockpt + ptsname
- [ ] Implement PTY pair creation
- [ ] Implement child process spawning (fork + setsid + TIOCSCTTY + dup2 + exec)
- [ ] Implement TIOCSWINSZ for resize
- [ ] Implement TIOCGWINSZ for size query
- [ ] Test on Linux
- [ ] Test on macOS

#### 6.2.2 Signal Handling (mux-pty)

- [ ] Wire signal-hook for SIGWINCH
- [ ] Wire signal-hook for SIGCHLD with waitpid loop
- [ ] Wire signal-hook for SIGTERM/SIGINT
- [ ] Test signal delivery in integration tests

#### 6.2.3 Raw Mode (mux-pty)

- [ ] Implement RawModeGuard with tcgetattr/tcsetattr/cfmakeraw
- [ ] Implement Managed/External/SaveRestore modes
- [ ] Test terminal state restoration on drop
- [ ] Test panic recovery (guard still restores)

#### 6.2.4 Async I/O Bridge

- [ ] Create tokio task for PTY read (master fd -> bytes)
- [ ] Create tokio task for PTY write (bytes -> master fd)
- [ ] Wire PTY read to KernelEvent::PtyOutput via channel
- [ ] Wire KernelEffect::SendPtyData to PTY write via channel
- [ ] Implement bounded channels (data=1024, control=512)

#### 6.2.5 SCM_RIGHTS (mux-fdpass + mux-pty)

- [ ] Implement sendmsg with SCM_RIGHTS
- [ ] Implement recvmsg with SCM_RIGHTS
- [ ] Wire FdEnvelope serialization to sendmsg payload
- [ ] Test fd passing between processes

### 6.3 Acceptance Criteria

- A pane can spawn a shell process.
- Typing in the terminal sends input to the shell.
- Shell output appears in the grid.
- Resizing the terminal propagates to the shell.
- Ctrl-C sends SIGINT to the foreground process.
- Closing a pane kills the child process.

### 6.4 Estimated Effort

4-5 weeks.

---

## 7. Phase 5: Rendering Pipeline

### 7.1 Objective

Wire the rendering pipeline from grid state to terminal output.

### 7.2 Tasks

#### 7.2.1 Frame Rendering

- [ ] Implement per-frame render loop:
  1. Collect dirty panes
  2. For each dirty pane, take grid snapshot
  3. Composite into output buffer
  4. Diff against previous frame
  5. Encode diff to escape sequences
  6. Write to stdout
- [ ] Implement frame rate limiting (configurable, default 60fps)
- [ ] Implement flood fairness with stride rotation

#### 7.2.2 Status Bar

- [ ] Implement status bar rendering
- [ ] Wire format string evaluation to status bar
- [ ] Implement left/center/right alignment
- [ ] Implement status bar colour
- [ ] Implement refresh interval

#### 7.2.3 Border Drawing

- [ ] Implement pane borders with box-drawing characters
- [ ] Implement active pane highlight
- [ ] Implement pane number overlay (for display-panes)

#### 7.2.4 Mouse Support

- [ ] Implement SGR mouse encoding
- [ ] Wire mouse events to pane selection
- [ ] Wire mouse events to scroll
- [ ] Wire mouse events to application passthrough

### 7.3 Acceptance Criteria

- Multiple panes render correctly with borders.
- Status bar shows session/window information.
- Resize redraws correctly.
- Mouse clicks select panes.
- High-output panes do not starve others.

### 7.4 Estimated Effort

3-4 weeks.

---

## 8. Phase 6: Configuration and Commands

### 8.1 Objective

Implement the tmux-compatible command system.

### 8.2 Tasks

#### 8.2.1 Command Registry

- [ ] Build command registry mapping names to handlers
- [ ] Implement command aliases
- [ ] Implement argument parsing per command
- [ ] Implement command chaining with semicolons

#### 8.2.2 Core Commands

Session commands:
- [ ] new-session
- [ ] kill-session
- [ ] rename-session
- [ ] list-sessions
- [ ] attach-session
- [ ] detach-client
- [ ] switch-client

Window commands:
- [ ] new-window
- [ ] kill-window
- [ ] rename-window
- [ ] select-window
- [ ] next-window
- [ ] previous-window
- [ ] last-window
- [ ] list-windows

Pane commands:
- [ ] split-window
- [ ] kill-pane
- [ ] select-pane
- [ ] resize-pane
- [ ] swap-pane
- [ ] display-panes
- [ ] capture-pane

Option commands:
- [ ] set-option / set
- [ ] show-options / show
- [ ] set-window-option / setw

Binding commands:
- [ ] bind-key / bind
- [ ] unbind-key / unbind
- [ ] list-keys

Other commands:
- [ ] source-file
- [ ] display-message
- [ ] send-keys
- [ ] copy-mode
- [ ] paste-buffer
- [ ] choose-tree

#### 8.2.3 Key Binding System

- [ ] Implement prefix key (default: Ctrl-b)
- [ ] Implement key table switching
- [ ] Implement default key bindings
- [ ] Implement key repeat (-r flag)
- [ ] Implement hierarchical scope resolution

#### 8.2.4 Configuration File

- [ ] Implement source-file command
- [ ] Implement ~/.tmux.conf loading on startup
- [ ] Implement XDG config path support
- [ ] Implement hot-reload via source-file
- [ ] Implement error reporting for config parsing

### 8.3 Acceptance Criteria

- Basic tmux workflow works: new-session, split-window, select-pane.
- Key bindings respond correctly.
- Configuration file is loaded on startup.
- source-file reloads configuration.
- show-options displays current options.

### 8.4 Estimated Effort

5-6 weeks.

---

## 9. Phase 7: Wire Protocol

### 9.1 Objective

Implement client-server communication for detach/attach.

### 9.2 Tasks

#### 9.2.1 Server Socket

- [ ] Create Unix domain socket at /tmp/termforge-<uid>/default
- [ ] Accept client connections
- [ ] Implement message framing with TF01 protocol
- [ ] Implement client authentication

#### 9.2.2 Client Connection

- [ ] Connect to server socket
- [ ] Send terminal size on connect
- [ ] Forward input to server
- [ ] Receive render data from server
- [ ] Handle detach/reattach

#### 9.2.3 tmux Wire v8 Compatibility

- [ ] Implement tmux protocol message parsing
- [ ] Implement tmux protocol message encoding
- [ ] Implement protocol version negotiation
- [ ] Implement dual-protocol support (TF01 + tmux v8)

#### 9.2.4 Multi-Client Support

- [ ] Track connected clients
- [ ] Implement client size negotiation (smallest wins)
- [ ] Implement per-client cursor position

### 9.3 Acceptance Criteria

- Can start server daemon.
- Can attach client to running server.
- Can detach and reattach without losing state.
- Multiple clients can attach to same session.
- tmux client can attach to TermForge server (stretch goal).

### 9.4 Estimated Effort

4-5 weeks.

---

## 10. Phase 8: SDK API and Language Bindings

### 10.1 Objective

Polish the SDK API and create language bindings.

### 10.2 Tasks

#### 10.2.1 Rust API (mux-api)

- [ ] Implement async Server with tokio
- [ ] Implement session/window/pane management
- [ ] Implement wait_for_content with timeout
- [ ] Implement send_keys
- [ ] Implement capture_pane
- [ ] Implement snapshot comparison
- [ ] Add comprehensive examples

#### 10.2.2 C FFI (mux-ffi)

- [ ] Expose all Server operations
- [ ] Expose session/window/pane operations
- [ ] Expose Termlet operations
- [ ] Generate C header file
- [ ] Test from C program

#### 10.2.3 Python Bindings

- [ ] Set up PyO3 project in bindings/python
- [ ] Expose Server, Session, Window, Pane classes
- [ ] Expose Termlet class
- [ ] Implement QuerySet with Python iteration
- [ ] Create pytest plugin for Termlet fixtures
- [ ] Write Python examples
- [ ] Publish to PyPI

#### 10.2.4 Node.js Bindings

- [ ] Set up napi-rs project in bindings/node
- [ ] Expose Server, Session, Window, Pane classes
- [ ] Expose Termlet class
- [ ] Implement async operations via Node.js event loop
- [ ] Create vitest plugin for snapshot testing
- [ ] Write Node.js examples
- [ ] Publish to npm

### 10.3 Acceptance Criteria

- Rust API: Full CRUD for sessions/windows/panes.
- Python: pytest fixtures work with snapshot assertions.
- Node.js: vitest integration works with snapshot assertions.
- C: Header file compiles and links correctly.

### 10.4 Estimated Effort

6-8 weeks.

---

## 11. Phase 9: Testing Infrastructure

### 11.1 Objective

Build comprehensive testing infrastructure.

### 11.2 Tasks

#### 11.2.1 Fuzz Targets

- [ ] VT parser state machine fuzzing
- [ ] CSI parameter parsing fuzzing
- [ ] Wire protocol frame fuzzing
- [ ] Configuration file parsing fuzzing
- [ ] Target syntax parsing fuzzing
- [ ] Target: 5+ fuzz targets with CI integration

#### 11.2.2 Snapshot Tests

- [ ] Set up insta snapshot infrastructure
- [ ] Create golden fixtures for common terminal output
- [ ] Snapshot tests for SGR rendering
- [ ] Snapshot tests for layout geometry
- [ ] Snapshot tests for status bar formatting

#### 11.2.3 Differential Tests

- [ ] Use tmux-builder to build reference tmux binary
- [ ] Create test harness that runs same commands in both
- [ ] Compare grid output cell-by-cell
- [ ] Track parity percentage in CI
- [ ] Cover: set options, key bindings, window splits, copy mode

#### 11.2.4 Performance Tests

- [ ] Criterion benchmarks for:
  - [ ] VT parser throughput (bytes/sec)
  - [ ] Grid write throughput (cells/sec)
  - [ ] Render diff throughput (cells/sec)
  - [ ] Arena intern throughput (strings/sec)
  - [ ] Frame encode throughput (bytes/sec)
  - [ ] Snapshot capture latency (ns)
- [ ] CI regression detection (>5% = fail)
- [ ] Target: 12+ benchmark targets

#### 11.2.5 Termlet Integration Tests

- [ ] Full lifecycle test: create -> feed -> snapshot -> stop
- [ ] Multi-termlet parallel test
- [ ] Resize during operation
- [ ] Escape sequence rendering verification
- [ ] Cross-crate integration with mux-test-support

### 11.3 Acceptance Criteria

- 5+ fuzz targets running in CI.
- 20+ snapshot test fixtures.
- 30+ differential test cases.
- 12+ benchmark targets.
- Parity tracking dashboard in CI.

### 11.4 Estimated Effort

4-5 weeks.

---

## 12. Phase 10: tmux Compatibility

### 12.1 Objective

Achieve high tmux behavioral parity.

### 12.2 Tasks

#### 12.2.1 Command Parity

Track implementation status of all 100+ tmux commands:

| Category | Commands | Priority |
|----------|----------|----------|
| Session | new-session, kill-session, etc. | P0 |
| Window | new-window, kill-window, etc. | P0 |
| Pane | split-window, select-pane, etc. | P0 |
| Option | set, show, setw | P0 |
| Binding | bind, unbind, list-keys | P1 |
| Copy | copy-mode, paste-buffer | P1 |
| Client | attach, detach, switch-client | P1 |
| Display | display-message, display-panes | P2 |
| Hook | set-hook, show-hooks | P2 |
| Format | display -p, if-shell | P2 |
| Buffer | list-buffers, save-buffer, etc. | P3 |
| Misc | clock-mode, choose-tree, etc. | P3 |

#### 12.2.2 Option Parity

Implement all common tmux options:

| Option | Type | Default |
|--------|------|---------|
| default-terminal | string | screen-256color |
| escape-time | int | 500 |
| history-limit | int | 2000 |
| mouse | bool | off |
| status | bool | on |
| status-style | style | default |
| mode-keys | enum | emacs |
| prefix | key | C-b |
| base-index | int | 0 |
| pane-base-index | int | 0 |
| renumber-windows | bool | off |
| set-clipboard | enum | external |
| allow-passthrough | bool | off |
| focus-events | bool | off |

#### 12.2.3 Behavioral Parity

- [ ] Window numbering matches tmux (base-index)
- [ ] Pane numbering matches tmux (pane-base-index)
- [ ] Key binding resolution matches tmux
- [ ] Option scoping matches tmux
- [ ] Escape timing matches tmux
- [ ] Mouse handling matches tmux

### 12.3 Acceptance Criteria

- 80%+ command parity for P0/P1 commands.
- All P0 options implemented.
- Differential tests pass for core workflows.
- A user's existing .tmux.conf loads without errors for basic configs.

### 12.4 Estimated Effort

8-10 weeks (ongoing).

---

## 13. Milestone Summary

| Milestone | Phase | Description | Target |
|-----------|-------|-------------|--------|
| M0 | 0 | Scaffold complete | Done |
| M1 | 1 | Types + grid battle-tested | +3 weeks |
| M2 | 2 | VT parser complete | +7 weeks |
| M3 | 3 | Kernel internals | +12 weeks |
| M4 | 4 | PTY integration | +17 weeks |
| M5 | 5 | Rendering pipeline | +21 weeks |
| M6 | 6 | Commands + config | +27 weeks |
| M7 | 7 | Wire protocol | +32 weeks |
| M8 | 8 | SDK + bindings | +40 weeks |
| M9 | 9 | Testing infra | +45 weeks |
| M10 | 10 | tmux compat | +55 weeks |

Total estimated: ~55 weeks for a single full-time developer.
Parallelizable: Phases 5-6 can overlap. Phases 8-9 can overlap. Phases 9-10 are ongoing.

---

## 14. Dependency Order

Critical path:

```
Phase 0 (Done) -> Phase 1 -> Phase 2 -> Phase 3 -> Phase 4
                                                       |
                                                       v
Phase 5 <-------------- Phase 6 <--------------- Phase 7
    |                      |                         |
    v                      v                         v
Phase 8 (Python/Node) Phase 9 (Testing)      Phase 10 (Compat)
```

Parallelizable:
- Phase 1 + Phase 2 can overlap (different crates).
- Phase 5 + Phase 6 can overlap (rendering + commands).
- Phase 8 + Phase 9 can overlap (bindings + testing).
- Phase 10 is ongoing alongside all later phases.

---

## 15. Risk Register

### R001: VT Parser Incompleteness

**Risk**: The VT parser may not handle all terminal applications correctly.
**Likelihood**: High. **Impact**: High.
**Mitigation**: Fuzz testing, differential testing against tmux, iterative
improvement based on real-world testing with vim, htop, less, etc.

### R002: Platform PTY Differences

**Risk**: PTY behavior differs between Linux, macOS, and BSDs.
**Likelihood**: Medium. **Impact**: High.
**Mitigation**: Platform abstraction in mux-pty, CI testing on all platforms,
PlatformCaps for capability detection.

### R003: Performance Regression

**Risk**: Rendering or parsing too slow for interactive use.
**Likelihood**: Low. **Impact**: High.
**Mitigation**: Criterion benchmarks with CI regression detection.
CLASS_TABLE[256] for parser hot path. Arc-COW for snapshot efficiency.

### R004: tmux Compatibility Complexity

**Risk**: tmux has 20+ years of behavior, not all documented.
**Likelihood**: High. **Impact**: Medium.
**Mitigation**: Incremental compatibility, differential testing, focus on
P0/P1 commands first.

### R005: Tokio Runtime Complexity

**Risk**: Async runtime adds complexity for signal handling and PTY I/O.
**Likelihood**: Medium. **Impact**: Medium.
**Mitigation**: Sans-IO kernel isolates complexity. signal-hook integrates
with tokio. Bounded channels prevent backpressure issues.

### R006: Language Binding Maintenance

**Risk**: Python and Node.js bindings fall out of sync with Rust API.
**Likelihood**: Medium. **Impact**: Medium.
**Mitigation**: Automated binding generation where possible. CI tests for
all bindings.

### R007: CRDT Overhead

**Risk**: Vector clock operations add latency to hot paths.
**Likelihood**: Low. **Impact**: Low.
**Mitigation**: Feature-gated behind `crdt`. Lamport-only fallback.

### R008: Signal Race Conditions

**Risk**: SIGCHLD arriving between fork() and waitpid() setup.
**Likelihood**: Low. **Impact**: Medium.
**Mitigation**: signal-hook blocks signals during critical sections.
Kernel-level retry logic for transient failures.

### R009: Memory Growth

**Risk**: Scrollback and arena grow without bound.
**Likelihood**: Medium. **Impact**: Medium.
**Mitigation**: Scrollback limit enforcement. Arena compaction (Phase 1).
Bounded channels prevent queue growth.

### R010: Edition 2024 Ecosystem Readiness

**Risk**: Some dependencies may not yet support edition 2024.
**Likelihood**: Low. **Impact**: Low.
**Mitigation**: MSRV 1.85. Dependencies use their own edition. Workspace
builds with any Rust >= 1.85.

---

## 16. Acceptance Criteria

### 16.1 Scaffold (Phase 0) -- COMPLETE

- [x] 23 library crates with real implementations
- [x] 4 tool binaries
- [x] 1,100+ tests
- [x] 135+ proptest references
- [x] 36+ integration test files
- [x] Arc-COW grid lines
- [x] 14-state VT parser
- [x] Sans-IO kernel
- [x] Double-buffer diff renderer
- [x] CRDT with verified convergence
- [x] Wire protocol frame model
- [x] ORM QuerySet API
- [x] Termlet testing pods
- [x] FFI with catch_unwind
- [x] OpenTelemetry span wrapper
- [x] All 9 gaps addressed
- [x] Architecture documentation
- [x] Plan documentation

### 16.2 Alpha (Phase 4 Complete)

- [ ] Can spawn a shell in a pane
- [ ] Can type and see output
- [ ] Can resize the terminal
- [ ] Can create multiple panes
- [ ] Signal handling works

### 16.3 Beta (Phase 7 Complete)

- [ ] Client-server detach/attach works
- [ ] Configuration file loading works
- [ ] Key bindings work
- [ ] Copy mode works
- [ ] Status bar renders

### 16.4 RC (Phase 10 Partial)

- [ ] 80%+ P0/P1 command parity
- [ ] Python bindings functional
- [ ] Node.js bindings functional
- [ ] Differential tests passing for core workflows
- [ ] Performance within 2x of tmux for common operations

---

## 17. Crate-by-Crate Implementation Status

### 17.1 Status Legend

| Symbol | Meaning |
|--------|---------|
| DONE | Complete with tests |
| PARTIAL | Core functionality implemented, gaps remain |
| STUB | Type definitions only, no real implementation |
| TODO | Not started |

### 17.2 Status Table

| Crate | Layer | Status | Tests | Gaps |
|-------|-------|--------|-------|------|
| mux-grapheme-arena | L0 | DONE | 13 | Arena compaction |
| mux-time | L0 | DONE | 14 | Real time source |
| mux-types | L1 | DONE | 80+ | Hyperlink, MouseEvent |
| mux-grid | L2 | DONE | 40+ | Tab stops, alt screen |
| mux-parser | L2 | DONE | 77+ | Full CSI/OSC/DCS dispatch |
| mux-options | L2 | DONE | 10+ | Scope chain resolution |
| mux-target | L2 | DONE | 15+ | None |
| mux-format | L2 | DONE | 11+ | Conditionals |
| mux-cmd-parse | L2 | DONE | 13+ | Full command set |
| mux-proto | L2 | DONE | 9+ | Real framing |
| mux-crdt | L2 | DONE | 19+ | None |
| mux-fdpass | L2 | DONE | 18+ | Real sendmsg/recvmsg |
| mux-snapshot | L3 | DONE | 13+ | None |
| mux-kernel | L3 | PARTIAL | 68+ | session/window/pane stubs |
| mux-render | L3 | DONE | 24+ | None |
| mux-orm | L3 | DONE | 11+ | None |
| mux-pty | L3 | PARTIAL | 15+ | Real PTY syscalls |
| mux-termlet | L4 | DONE | 11+ | None |
| mux-config | L4 | DONE | 29+ | Full command set |
| mux-api | L4 | DONE | 25+ | Async integration |
| mux-test-support | L4 | DONE | 27+ | None |
| mux-ffi | L5 | DONE | 7+ | Full API surface |
| mux-otel | L5 | DONE | 14+ | Real OTEL export |

### 17.3 Lines of Code

| Component | Lines |
|-----------|-------|
| Source (lib.rs + modules) | ~12,400 |
| Tests (inline + integration) | ~7,500 |
| Configuration (Cargo.toml) | ~1,200 |
| Documentation (md) | ~8,000+ |
| **Total** | **~29,100+** |

---

## 18. Test Coverage Plan

### 18.1 Current Coverage

| Category | Count | Target |
|----------|-------|--------|
| Unit tests | ~900 | 1,500+ |
| Property tests | 135 | 200+ |
| Integration tests | 36 files | 50+ files |
| Snapshot tests | 0 | 50+ |
| Fuzz targets | 0 | 5+ |
| Benchmarks | 0 | 12+ |
| Differential | 0 | 50+ cases |

### 18.2 Per-Phase Test Additions

| Phase | Test Type | Count |
|-------|-----------|-------|
| 1 | Unit + proptest for grid/types | +100 |
| 2 | Parser fuzz + CSI tests | +150 |
| 3 | Kernel integration tests | +100 |
| 4 | PTY integration tests | +50 |
| 5 | Rendering snapshot tests | +50 |
| 6 | Command integration tests | +100 |
| 7 | Protocol roundtrip tests | +50 |
| 8 | SDK examples as tests | +50 |
| 9 | Fuzz + differential + benchmarks | +100 |
| 10 | tmux parity tests | +200 |

### 18.3 Critical Properties to Test

| Property | Framework | Crate |
|----------|-----------|-------|
| Encode/decode roundtrip | proptest | mux-proto, mux-fdpass |
| Merge commutativity | proptest | mux-crdt |
| Merge idempotency | proptest | mux-crdt |
| Merge associativity | proptest | mux-crdt |
| Grid resize preserves content | proptest | mux-grid |
| Parser state determinism | proptest | mux-parser |
| Cursor bounds | proptest | mux-grid |
| Layout distribute uniformity | proptest | mux-kernel |
| Snapshot isolation | unit | mux-snapshot |
| Socket uniqueness | unit | mux-test-support |
| Cleanup on drop | unit | mux-test-support |

---

## 19. Documentation Plan

### 19.1 User Documentation

| Document | Target | Status |
|----------|--------|--------|
| README.md | Project overview | TODO |
| CLAUDE.md | Project conventions | DONE |
| AGENTS.md | Rule catalog | DONE |
| CHANGELOG.md | Release notes | TODO |
| CONTRIBUTING.md | Contributor guide | TODO |

### 19.2 Architecture Documentation

| Document | Target | Status |
|----------|--------|--------|
| notes/architecture.md | Full architecture | DONE |
| notes/plan.md | Implementation plan | DONE |
| notes/decisions/ | ADR directory | TODO |

### 19.3 API Documentation

| Component | Target | Status |
|-----------|--------|--------|
| Rust docs (cargo doc) | All public API | TODO |
| Python API reference | PyO3 bindings | TODO |
| Node.js API reference | napi-rs bindings | TODO |
| C header docs | FFI surface | TODO |

### 19.4 Per-Crate READMEs

Each crate should have a README.md with:
- Purpose and scope
- Key types and traits
- Usage examples
- Invariants that apply
- Testing instructions

---

## 20. CI/CD Pipeline Plan

### 20.1 Pipeline Stages

```
[Lint] -> [Build] -> [Test] -> [Fuzz] -> [Bench] -> [Doc] -> [Publish]
```

### 20.2 Lint Stage

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo deny check` (dependency audit)
- MSRV check: `cargo +1.85 check`

### 20.3 Build Stage

- `cargo build --workspace`
- `cargo build --workspace --release`
- Cross-compilation targets: Linux x86_64, Linux aarch64, macOS x86_64, macOS aarch64

### 20.4 Test Stage

- `cargo test --workspace`
- `cargo test --workspace --release` (release-mode tests)
- Feature matrix: default, crdt, all

### 20.5 Fuzz Stage (Nightly)

- Run fuzz targets for 1M iterations each
- Report new findings

### 20.6 Bench Stage

- Run criterion benchmarks
- Compare against previous baseline
- Fail if >5% regression

### 20.7 Doc Stage

- `cargo doc --workspace --no-deps`
- Verify no broken doc links

### 20.8 Publish Stage (Release Only)

- Version bump
- Cargo publish (dry run)
- Create GitHub release

---

## 21. Benchmark Plan

### 21.1 Targets

| Target | Metric | Expected |
|--------|--------|----------|
| parser_throughput | bytes/sec | 500+ MB/s |
| grid_write | cells/sec | 10M+ |
| grid_snapshot | ns/op | <1000ns (80x24) |
| render_diff | cells/sec | 5M+ |
| arena_intern | ops/sec | 5M+ |
| frame_encode | bytes/sec | 200+ MB/s |
| frame_decode | bytes/sec | 200+ MB/s |
| option_resolve | ns/op | <100ns |
| target_parse | ns/op | <500ns |
| format_eval | ns/op | <1000ns |
| cmd_tokenize | ns/op | <500ns |
| crdt_merge | ns/op | <100ns |

### 21.2 Baseline

Benchmarks will be baselined against:
1. The first measurement (for trend tracking).
2. tmux equivalent operations where measurable.

---

## 22. Fuzz Testing Plan

### 22.1 Targets

| Target | Input | Harness |
|--------|-------|---------|
| vt_parser | Random byte sequences | cargo-fuzz |
| csi_params | Random CSI parameter strings | cargo-fuzz |
| frame_decode | Random byte sequences | cargo-fuzz |
| config_parse | Random .tmux.conf content | cargo-fuzz |
| target_parse | Random target strings | cargo-fuzz |
| cmd_tokenize | Random command strings | cargo-fuzz |
| format_eval | Random format strings | cargo-fuzz |

### 22.2 Corpus

- Seed corpus from real terminal output captures.
- Seed corpus from real .tmux.conf files.
- Seed corpus from tmux command examples.

### 22.3 CI Integration

- Run fuzz targets for 1M iterations nightly.
- Report crashes immediately.
- Minimized crash inputs added to regression tests.

---

## 23. Differential Testing Plan

### 23.1 Architecture

```
Test Case Definition
       |
       v
+------+------+
|             |
v             v
TermForge     tmux (via tmux-vm)
|             |
v             v
Grid State    capture-pane output
|             |
v             v
+------+------+
       |
       v
Cell-by-cell comparison
       |
       v
Parity report
```

### 23.2 Test Categories

| Category | Commands | Expected Parity |
|----------|----------|-----------------|
| Basic output | echo, ls, cat | 100% |
| Cursor movement | vim navigation | 95%+ |
| Colours | ls --color, htop | 90%+ |
| Scroll | less, man | 95%+ |
| Window management | split, resize | 95%+ |
| Copy mode | search, yank | 90%+ |
| Options | set, show | 100% |

### 23.3 tmux Versions

Test against multiple tmux versions using tmux-vm:
- tmux 3.0 (minimum supported)
- tmux 3.2
- tmux 3.4
- tmux 3.5 (latest stable)

---

## 24. Language Binding Plan

### 24.1 Python Bindings

#### Setup
- PyO3 with maturin for build
- `bindings/python/` directory
- `pyproject.toml` with maturin build backend

#### API Surface

```python
import termforge

# Server creation
server = termforge.Server(cols=80, rows=24)
session = server.new_session("dev")
window = session.new_window("editor")
pane = window.split(direction="vertical")

# Data flow
pane.send_keys("ls -la\n")
pane.wait_for_content("total", timeout=5.0)
snapshot = pane.capture()
assert "main.py" in snapshot.text()

# QuerySet
devs = server.sessions.filter(name__contains="dev")
assert devs.count() == 1

# Termlet
termlet = termforge.Termlet(cols=80, rows=24)
termlet.feed(b"Hello World")
assert termlet.text(0) == "Hello World"
```

#### pytest Integration

```python
# conftest.py
import termforge

@pytest.fixture
def server():
    s = termforge.Server(cols=80, rows=24)
    yield s
    s.shutdown()

@pytest.fixture
def termlet():
    t = termforge.Termlet(cols=80, rows=24)
    yield t
    t.stop()
```

### 24.2 Node.js Bindings

#### Setup
- napi-rs for native addon
- `bindings/node/` directory
- `package.json` with napi build scripts

#### API Surface

```javascript
import { Server, Termlet } from '@termforge/termforge';

// Server creation
const server = new Server({ cols: 80, rows: 24 });
const session = await server.createSession('dev');
const window = await session.createWindow('editor');
const pane = await window.split({ direction: 'vertical' });

// Data flow
await pane.sendKeys('ls -la\n');
await pane.waitForContent('total', { timeout: 5000 });
const snapshot = await pane.capture();
expect(snapshot.text()).toContain('main.ts');

// Termlet
const termlet = new Termlet({ cols: 80, rows: 24 });
termlet.feed(Buffer.from('Hello World'));
expect(termlet.text(0)).toBe('Hello World');
```

#### vitest Integration

```javascript
import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { Termlet } from '@termforge/termforge';

describe('terminal output', () => {
  let termlet;

  beforeEach(() => {
    termlet = new Termlet({ cols: 80, rows: 24 });
  });

  afterEach(() => {
    termlet.stop();
  });

  it('renders text correctly', () => {
    termlet.feed(Buffer.from('Hello World'));
    expect(termlet.text(0)).toMatchSnapshot();
  });
});
```

---

## 25. Version Management Plan

### 25.1 tmux-builder

Build tmux from source at a specific version:

```bash
tmux-builder build 3.4
tmux-builder build 3.5
```

Output: `~/.termforge/builds/tmux-3.4/bin/tmux`

### 25.2 tmux-vm

Manage installed tmux versions:

```bash
tmux-vm install 3.4
tmux-vm install 3.5
tmux-vm use 3.4
tmux-vm list
tmux-vm run 3.5 -- new-session -d
```

### 25.3 CI Integration

CI matrix tests against multiple tmux versions:

```yaml
strategy:
  matrix:
    tmux-version: ['3.0', '3.2', '3.4', '3.5']
```

---

## 26. OpenTelemetry Integration Plan

### 26.1 Span Placement

| Span | Crate | When |
|------|-------|------|
| kernel.process_event | mux-kernel | Every event dispatch |
| parser.parse_bytes | mux-parser | VT byte processing |
| render.frame | mux-render | Each render frame |
| pty.spawn | mux-pty | Process spawning |
| pty.resize | mux-pty | PTY resize |
| config.load | mux-config | Configuration loading |
| proto.encode_frame | mux-proto | Frame encoding |
| proto.decode_frame | mux-proto | Frame decoding |
| api.create_session | mux-api | Session creation |
| termlet.feed | mux-termlet | Data feeding |

### 26.2 Metrics

| Metric | Type | Crate |
|--------|------|-------|
| termforge.grid.cell_writes | Counter | mux-grid |
| termforge.grid.snapshots | Counter | mux-snapshot |
| termforge.parser.bytes_processed | Counter | mux-parser |
| termforge.render.frames | Counter | mux-render |
| termforge.render.diff_cells | Histogram | mux-render |
| termforge.arena.size | Gauge | mux-grapheme-arena |
| termforge.sessions.active | Gauge | mux-kernel |
| termforge.panes.active | Gauge | mux-kernel |

### 26.3 Exporter Configuration

Support multiple exporters via configuration:

```toml
[otel]
enabled = true
exporter = "otlp"    # otlp, jaeger, stdout
endpoint = "http://localhost:4317"
service_name = "termforge"
```

---

## 27. CRDT Integration Plan

### 27.1 Feature Gate

All CRDT code is behind `#[cfg(feature = "crdt")]`:

```toml
[features]
default = []
crdt = ["mux-crdt"]
```

### 27.2 Use Cases

1. **Multi-user collaboration**: Multiple users editing the same session.
2. **State synchronization**: Replicating session state across servers.
3. **Conflict resolution**: Merging divergent option changes.

### 27.3 Integration Points

| Component | CRDT Application |
|-----------|-----------------|
| Options | LwwRegister for option values |
| Session metadata | LwwRegister for session name |
| Pane title | LwwRegister for title |
| Cursor position | LwwRegister per client |

### 27.4 Deterministic Replay

CRDT operations support deterministic replay:

1. Record all operations with vector clock timestamps.
2. Replay operations in any order.
3. Final state is identical regardless of replay order.

This is verified by proptest for merge commutativity, idempotency, and
associativity.

---

## 28. Migration Guide

### 28.1 From tmux

For users migrating from tmux:

1. **Configuration**: `.tmux.conf` files are compatible (basic set/bind).
2. **Key bindings**: Default bindings match tmux (Ctrl-b prefix).
3. **Commands**: Core commands (`new-session`, `split-window`, etc.) work.
4. **Environment**: `$TERM` is set to `screen-256color` by default.

### 28.2 From Zellij

Key differences:
- TermForge uses tmux-compatible key bindings (prefix model).
- TermForge uses tmux command syntax.
- TermForge supports `.tmux.conf` format.
- Zellij layouts are not compatible (use split-window instead).

### 28.3 API Migration (from libtmux)

Python libtmux users can migrate to the TermForge Python bindings:

| libtmux | TermForge |
|---------|-----------|
| `Server()` | `termforge.Server()` |
| `server.new_session("dev")` | `server.new_session("dev")` |
| `session.windows` | `session.windows` |
| `window.split_window()` | `window.split()` |
| `pane.send_keys("ls")` | `pane.send_keys("ls\n")` |
| `pane.capture_pane()` | `pane.capture().text()` |

### 28.4 Breaking Changes

v0005 breaks from v0004 in the following ways:
- Edition 2024 (was 2021). `gen` is now reserved.
- MSRV 1.85 (was 1.75).
- `#[unsafe(no_mangle)]` syntax required for FFI (was `#[no_mangle]`).
- signal-hook added as explicit dependency (was implicit via tokio).
- FdEnvelope cap is 16 (was undefined).
- Grid constructor takes scrollback_limit parameter.
- TerminalMode policy formalized (Managed/External/SaveRestore).

---

## 29. Detailed Crate Implementation Guides

### 29.1 mux-grapheme-arena Implementation Guide

**Current state**: Complete with interning, lookup, and DEFAULT sentinel.

**Remaining work**:
- Arena compaction: Remove unreferenced entries to reclaim memory.
- Memory reporting: `fn memory_usage(&self) -> usize` for OTEL metrics.
- WASM compilation: Verify `cargo build --target wasm32-unknown-unknown`.

**Implementation approach**:
1. Add reference counting per GraphemeId.
2. Compaction sweeps entries with refcount == 0.
3. Compaction remaps IDs (requires grid cooperation).

**Tests to add**:
- Compaction removes unreferenced entries.
- Compaction preserves referenced entries.
- Memory usage reports accurate numbers.
- WASM compilation succeeds.

### 29.2 mux-time Implementation Guide

**Current state**: Complete with MonotonicClock, Timestamp, TimerWheel.

**Remaining work**:
- Real time source using `std::time::Instant`.
- Deterministic time source for testing/replay.
- Wall clock integration for status bar timestamps.

**Implementation approach**:
1. `ClockSource` trait with `RealClock` and `DeterministicClock` implementations.
2. `DeterministicClock::advance(duration)` for controlled stepping.
3. `RealClock::now()` wraps `Instant::now()`.

**Tests to add**:
- Real clock advances monotonically.
- Deterministic clock only advances when stepped.
- Timer wheel fires at correct deterministic times.

### 29.3 mux-types Implementation Guide

**Current state**: Complete with 8 modules covering all core types.

**Remaining work**:
- `Hyperlink` struct for OSC 8 support.
- `TerminalMode` enum for DECSET/DECRST modes.
- `MouseEvent` struct with button, position, modifiers.
- `PaneMode` enum (Normal, CopyMode, CommandMode).
- Display implementations for all types.

**Implementation approach for Hyperlink**:
```rust
pub struct Hyperlink {
    pub id: Option<String>,
    pub uri: String,
}
```

Store hyperlinks in a per-pane table indexed by the Cell's `link` field.
Link ID 0 means no hyperlink.

**Implementation approach for TerminalMode**:
```rust
bitflags! {
    pub struct TerminalModes: u32 {
        const CURSOR_KEYS = 1 << 0;      // DECCKM (mode 1)
        const AUTOWRAP = 1 << 1;          // DECAWM (mode 7)
        const CURSOR_BLINK = 1 << 2;      // Mode 12
        const CURSOR_VISIBLE = 1 << 3;    // Mode 25
        const ALT_SCREEN = 1 << 4;        // Mode 47/1047
        const MOUSE_CLICK = 1 << 5;       // Mode 1000
        const MOUSE_DRAG = 1 << 6;        // Mode 1002
        const MOUSE_ALL = 1 << 7;         // Mode 1003
        const MOUSE_SGR = 1 << 8;         // Mode 1006
        const ALT_SCREEN_SAVE = 1 << 9;   // Mode 1049
        const BRACKETED_PASTE = 1 << 10;  // Mode 2004
        const FOCUS_EVENTS = 1 << 11;     // Mode 1004
    }
}
```

### 29.4 mux-grid Implementation Guide

**Current state**: Complete with ChunkedGrid, Arc-COW lines, scrollback.

**Remaining work**:
- Tab stop handling in write_char.
- Alternate screen buffer (primary/alternate switch).
- Saved cursor position (DECSC/DECRC).
- Insert/delete characters (ICH/DCH).
- Line wrapping flag management.

**Tab stop implementation**:
```rust
pub struct ChunkedGrid {
    // ... existing fields ...
    tab_stops: Vec<bool>,  // tab_stops[col] = true if tab stop at col
    saved_cursor: Option<(u16, u16)>,
    alt_grid: Option<Box<ChunkedGrid>>,  // alternate screen
}
```

Default tab stops at every 8 columns. `HT` (0x09) advances cursor to next
tab stop.

**Alternate screen**:
```rust
pub fn enter_alt_screen(&mut self) {
    let alt = ChunkedGrid::new(self.cols, self.rows, 0);
    self.alt_grid = Some(Box::new(std::mem::replace(self, alt)));
}

pub fn exit_alt_screen(&mut self) {
    if let Some(primary) = self.alt_grid.take() {
        *self = *primary;
    }
}
```

### 29.5 mux-parser Implementation Guide

**Current state**: 14-state machine with CSI dispatch and InputParser.

**Remaining work**: See Phase 2 task list. Key items:

**DECSET/DECRST implementation**:
```rust
fn handle_private_mode(&mut self, mode: u16, enable: bool) {
    match mode {
        1 => self.modes.set(TerminalModes::CURSOR_KEYS, enable),
        7 => self.modes.set(TerminalModes::AUTOWRAP, enable),
        25 => self.modes.set(TerminalModes::CURSOR_VISIBLE, enable),
        47 | 1047 => {
            if enable { self.grid.enter_alt_screen(); }
            else { self.grid.exit_alt_screen(); }
        }
        1000 => self.modes.set(TerminalModes::MOUSE_CLICK, enable),
        1006 => self.modes.set(TerminalModes::MOUSE_SGR, enable),
        1049 => {
            if enable {
                self.grid.save_cursor();
                self.grid.enter_alt_screen();
            } else {
                self.grid.exit_alt_screen();
                self.grid.restore_cursor();
            }
        }
        2004 => self.modes.set(TerminalModes::BRACKETED_PASTE, enable),
        _ => {} // Unknown modes are silently ignored
    }
}
```

### 29.6 mux-kernel Implementation Guide

**Current state**: Kernel struct with event/effect dispatch. Layout engine
and copy mode implemented. Session/window/pane models are stubs.

**Session implementation (session.rs)**:

```rust
pub struct KernelSession {
    pub id: SessionId,
    pub name: String,
    pub windows: Vec<WindowId>,
    pub active_window_idx: usize,
    pub created_at: u64,
    pub options: OptionTable,
}

impl KernelSession {
    pub fn new(id: SessionId, name: String) -> Self {
        Self {
            id,
            name,
            windows: Vec::new(),
            active_window_idx: 0,
            created_at: 0,
            options: OptionTable::new(OptionScope::Session),
        }
    }

    pub fn active_window(&self) -> Option<WindowId> {
        self.windows.get(self.active_window_idx).copied()
    }

    pub fn add_window(&mut self, wid: WindowId) {
        self.windows.push(wid);
    }

    pub fn remove_window(&mut self, wid: WindowId) {
        self.windows.retain(|w| *w != wid);
        if self.active_window_idx >= self.windows.len() && !self.windows.is_empty() {
            self.active_window_idx = self.windows.len() - 1;
        }
    }
}
```

**Window implementation (window.rs)**:

```rust
pub struct KernelWindow {
    pub id: WindowId,
    pub session_id: SessionId,
    pub name: String,
    pub panes: Vec<PaneId>,
    pub active_pane_idx: usize,
    pub layout: LayoutNode,
    pub options: OptionTable,
}
```

**Pane implementation (pane.rs)**:

```rust
pub struct KernelPane {
    pub id: PaneId,
    pub window_id: WindowId,
    pub size: Size,
    pub pid: Option<u32>,
    pub title: String,
    pub mode: PaneMode,
    pub copy_state: Option<CopyModeState>,
}
```

Note: The grid, parser, and arena are stored in the Kernel struct (not in
KernelPane) to avoid lifetime/ownership complexity. KernelPane stores the
PaneId which is used to look up the grid in `Kernel::grids`.

### 29.7 mux-render Implementation Guide

**Current state**: Complete with RenderBuffer, CompositeBuffer, diff_buffers,
encode_diff.

**Remaining work**:
- Status bar rendering.
- Border drawing with box-drawing characters.
- Active pane highlight.
- Frame rate limiting.

**Status bar rendering**:
```rust
pub fn render_status_bar(
    format: &str,
    context: &FormatContext,
    cols: u16,
) -> Vec<Cell> {
    let text = mux_format::evaluate(format, context);
    let mut cells = Vec::with_capacity(cols as usize);
    for (i, ch) in text.chars().enumerate() {
        if i >= cols as usize { break; }
        cells.push(Cell::with_grapheme(arena.intern(&ch.to_string()), 1));
    }
    // Pad with spaces
    while cells.len() < cols as usize {
        cells.push(Cell::default());
    }
    cells
}
```

**Border drawing**:
```rust
const BORDER_H: char = '\u{2500}';  // ─
const BORDER_V: char = '\u{2502}';  // │
const BORDER_TL: char = '\u{250C}'; // ┌
const BORDER_TR: char = '\u{2510}'; // ┐
const BORDER_BL: char = '\u{2514}'; // └
const BORDER_BR: char = '\u{2518}'; // ┘
const BORDER_T: char = '\u{252C}';  // ┬
const BORDER_B: char = '\u{2534}';  // ┴
const BORDER_L: char = '\u{251C}';  // ├
const BORDER_R: char = '\u{2524}';  // ┤
const BORDER_X: char = '\u{253C}';  // ┼
```

### 29.8 mux-pty Implementation Guide

**Current state**: Model types (PtyPair, SpawnContext, PtyError, RawModeGuard,
SignalBridge). No actual syscalls.

**Implementation order**:
1. PTY pair creation via `posix_openpt`.
2. Child process spawning via `fork` + `exec`.
3. TIOCSWINSZ for resize.
4. Signal handling via `signal-hook`.
5. Raw mode via `tcgetattr`/`tcsetattr`.

**PTY creation (Linux)**:
```rust
pub fn open_pty() -> Result<PtyPair, PtyError> {
    use nix::pty::openpty;
    let pty = openpty(None, None)
        .map_err(|e| PtyError::Io(std::io::Error::from_raw_os_error(e as i32)))?;
    Ok(PtyPair {
        master: pty.master,
        slave: pty.slave,
    })
}
```

**Child spawning**:
```rust
pub fn spawn(ctx: &SpawnContext, slave_fd: RawFd) -> Result<u32, PtyError> {
    use nix::unistd::{fork, ForkResult, setsid, dup2, execvp};
    match unsafe { fork() } {
        Ok(ForkResult::Child) => {
            setsid().ok();
            dup2(slave_fd, 0).ok();  // stdin
            dup2(slave_fd, 1).ok();  // stdout
            dup2(slave_fd, 2).ok();  // stderr
            // Close other fds
            // exec
            unreachable!();
        }
        Ok(ForkResult::Parent { child }) => Ok(child.as_raw() as u32),
        Err(e) => Err(PtyError::SpawnFailed(e.to_string())),
    }
}
```

### 29.9 mux-api Implementation Guide

**Current state**: ServerBuilder + Server with session/window/pane management.

**Remaining work**:
- Async integration with tokio.
- PTY process spawning.
- Channel-based kernel bridge.
- Client connection handling.

**Async server architecture**:
```rust
pub struct AsyncServer {
    kernel: Kernel,
    event_tx: mpsc::Sender<KernelEvent>,
    effect_rx: mpsc::Receiver<KernelEffect>,
    pty_handles: HashMap<PaneId, PtyHandle>,
}

impl AsyncServer {
    pub async fn run(&mut self) -> Result<(), MuxError> {
        loop {
            tokio::select! {
                event = self.event_tx.recv() => {
                    let effects = self.kernel.process(event);
                    for effect in effects {
                        self.handle_effect(effect).await?;
                    }
                }
                // ... PTY read, client connections, signals
            }
        }
    }
}
```

### 29.10 mux-config Implementation Guide

**Current state**: Parses set/set-option/bind from tmux format.

**Remaining work**:
- Full command dispatch (not just set/bind).
- TOML configuration support.
- XDG path resolution.
- Hot-reload via source-file.

**XDG path resolution**:
```rust
pub fn config_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // XDG config
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        paths.push(PathBuf::from(xdg).join("termforge/config.toml"));
    } else if let Ok(home) = std::env::var("HOME") {
        paths.push(PathBuf::from(home).join(".config/termforge/config.toml"));
    }

    // tmux compat
    if let Ok(home) = std::env::var("HOME") {
        paths.push(PathBuf::from(home).join(".tmux.conf"));
    }

    paths
}
```

---

## 30. Invariant Verification Matrix

This matrix maps each invariant to the specific test(s) that verify it.

### 30.1 Core Invariants

| Invariant | Test Location | Test Name |
|-----------|---------------|-----------|
| INV-001 | mux-proto/tests/proptest_proto.rs | encode_decode_roundtrip |
| INV-002 | Cargo.toml | Workspace crate names |
| INV-003 | All crates except mux-pty, mux-ffi | `#![forbid(unsafe_code)]` |
| INV-004 | All crates | Compile-time derive check |
| INV-005 | mux-types/src/error.rs | error_trait_compliance |
| INV-006 | Clippy configuration | deny(unwrap_used, expect_used, panic) |
| INV-007 | mux-grid/tests/grid_operations.rs | new_grid_all_defaults |
| INV-008 | mux-types/src/cell.rs | Cell API tests |
| INV-009 | mux-types/src/attrs.rs | Bitflags tests |
| INV-010 | mux-parser/tests/state_machine_proptest.rs | deterministic_transitions |
| INV-011 | mux-proto/src/lib.rs | LE encoding tests |
| INV-012 | mux-grid/src/lib.rs | put_cell / write_char tests |
| INV-013 | mux-pty/src/lib.rs | State machine tests |
| INV-014 | mux-test-support/src/lib.rs | socket_paths_unique_parallel |
| INV-015 | Future: differential tests | tmux comparison |
| INV-016 | mux-kernel/tests/layout_tests.rs | distribute_evenly |
| INV-017 | mux-otel/src/lib.rs | Span tests |
| INV-018 | Cargo.toml feature configuration | Feature gate tests |
| INV-019 | mux-types derives | Clone + Eq |
| INV-020 | mux-proto/src/lib.rs | Frame encode tests |
| INV-021 | Future: clipboard implementation | Size limit tests |
| INV-022 | Future: key binding implementation | Scope tests |
| INV-023 | mux-proto constants | Header size assertion |
| INV-024 | Cargo.toml dependency audit | No SmallVec |
| INV-025 | mux-crdt/tests/proptest_crdt.rs | merge_commutative |
| INV-026 | mux-parser/tests/vt_sequences.rs | dcs_entry_byte |
| INV-027 | mux-kernel/src/lib.rs | Pure function design |
| INV-028 | mux-snapshot/src/lib.rs | Snapshot codec |
| INV-029 | mux-types/src/cell.rs | Width validation |
| INV-030 | mux-grapheme-arena/src/lib.rs | DEFAULT constant |
| INV-031 | mux-proto/src/lib.rs | CRC32 encode |
| INV-032 | mux-grid/src/lib.rs | Arc<Vec<Cell>> type |
| INV-033 | mux-crdt/tests/proptest_crdt.rs | Convergence tests |
| INV-034 | Various serde round-trip tests | Canonical serialization |

### 30.2 Extended Invariants

| Invariant | Test Location | Test Name |
|-----------|---------------|-----------|
| INV-117 | Cargo.toml profiles | panic = "unwind" |
| INV-119 | mux-types/tests/comprehensive_types.rs | cell_flags_match_tmux |
| INV-220 | mux-config/tests/config_loading.rs | allow_passthrough_default_off |

---

## 31. Crate API Surface Summary

### 31.1 mux-grapheme-arena

```rust
pub struct GraphemeArena { ... }
pub struct GraphemeId(pub u32);

impl GraphemeArena {
    pub fn new() -> Self;
    pub fn intern(&mut self, s: &str) -> GraphemeId;
    pub fn resolve(&self, id: GraphemeId) -> &str;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
}

impl GraphemeId {
    pub const DEFAULT: Self;
    pub fn from_raw(v: u32) -> Self;
}
```

### 31.2 mux-time

```rust
pub struct MonotonicClock { ... }
pub struct Timestamp(pub u64);
pub struct TimerWheel { ... }
pub struct TimerId(pub u64);

impl MonotonicClock {
    pub fn new() -> Self;
    pub fn now(&self) -> Timestamp;
    pub fn advance(&mut self, ms: u64);
}

impl Timestamp {
    pub const ZERO: Self;
    pub fn elapsed_since(&self, other: Timestamp) -> u64;
}

impl TimerWheel {
    pub fn new(slots: usize) -> Self;
    pub fn insert(&mut self, deadline: Timestamp) -> TimerId;
    pub fn cancel(&mut self, id: TimerId) -> bool;
    pub fn expire(&mut self, now: Timestamp) -> Vec<TimerId>;
}
```

### 31.3 mux-types (selected)

```rust
// Cell
pub struct Cell { pub grapheme: GraphemeId, pub width: u8, ... }
impl Cell {
    pub fn default() -> Self;
    pub fn with_grapheme(id: GraphemeId, width: u8) -> Self;
    pub fn is_wide(&self) -> bool;
    pub fn is_padding(&self) -> bool;
    pub fn clear(&mut self);
}

// Colour
pub enum Colour { Default, Indexed(u8), Rgb { r: u8, g: u8, b: u8 } }

// Geometry
pub struct Size { pub cols: u16, pub rows: u16 }
pub struct Rect { pub x: u16, pub y: u16, pub width: u16, pub height: u16 }

// IDs
pub struct SessionId(pub u64);
pub struct WindowId(pub u64);
pub struct PaneId(pub u64);

// Error
pub enum MuxError { Kernel(String), Config(String), ... }
```

### 31.4 mux-grid

```rust
pub struct ChunkedGrid { ... }
pub type Line = Arc<Vec<Cell>>;

impl ChunkedGrid {
    pub fn new(cols: u16, rows: u16, scrollback_limit: usize) -> Self;
    pub fn write_char(&mut self, grapheme: GraphemeId, width: u8);
    pub fn put_cell(&mut self, col: u16, row: u16, cell: Cell);
    pub fn cell(&self, col: u16, row: u16) -> &Cell;
    pub fn set_cursor(&mut self, x: u16, y: u16);
    pub fn cursor(&self) -> (u16, u16);
    pub fn scroll_up(&mut self, n: u16);
    pub fn scroll_down(&mut self, n: u16);
    pub fn resize(&mut self, cols: u16, rows: u16);
    pub fn clear_all(&mut self);
    pub fn clear_line(&mut self, row: u16);
    pub fn erase_in_line(&mut self, col: u16, row: u16, left: bool);
    pub fn insert_lines(&mut self, row: u16, n: u16);
    pub fn delete_lines(&mut self, row: u16, n: u16);
    pub fn set_scroll_region(&mut self, top: u16, bottom: u16);
    pub fn reset_scroll_region(&mut self);
    pub fn snapshot(&self) -> Vec<Line>;
    pub fn line_text(&self, row: u16, arena: &GraphemeArena) -> String;
    pub fn is_dirty(&self) -> bool;
    pub fn clear_dirty(&mut self);
    pub fn cols(&self) -> u16;
    pub fn rows(&self) -> u16;
    pub fn size(&self) -> Size;
}
```

### 31.5 mux-parser

```rust
pub struct VtStateMachine { ... }
pub struct InputParser { ... }

impl VtStateMachine {
    pub fn new() -> Self;
    pub fn step(&mut self, byte: u8) -> Action;
    pub fn state(&self) -> State;
}

impl InputParser {
    pub fn new(cols: u16, rows: u16) -> Self;
    pub fn feed(&mut self, data: &[u8], grid: &mut ChunkedGrid, arena: &mut GraphemeArena);
}
```

### 31.6 mux-kernel

```rust
pub struct Kernel { ... }
pub enum KernelEvent { ... }
pub enum KernelEffect { ... }
pub enum LayoutNode { ... }
pub struct CopyModeState { ... }
pub struct PasteBufferRing { ... }

impl Kernel {
    pub fn new(size: Size) -> Self;
    pub fn process(&mut self, event: KernelEvent) -> Vec<KernelEffect>;
    pub fn session_count(&self) -> usize;
    pub fn create_session(&mut self, name: &str) -> SessionId;
    pub fn destroy_session(&mut self, id: SessionId);
    // ...
}
```

### 31.7 mux-render

```rust
pub struct RenderBuffer { ... }
pub struct CompositeBuffer { ... }
pub struct DiffEntry { pub col: u16, pub row: u16, pub cell: Cell }

impl RenderBuffer {
    pub fn new(cols: u16, rows: u16) -> Self;
    pub fn set(&mut self, col: u16, row: u16, cell: Cell);
    pub fn get(&self, col: u16, row: u16) -> &Cell;
    pub fn cols(&self) -> u16;
    pub fn rows(&self) -> u16;
}

pub fn diff_buffers(prev: &RenderBuffer, next: &RenderBuffer) -> Vec<DiffEntry>;
pub fn encode_diff(diff: &[DiffEntry], arena: &GraphemeArena) -> Vec<u8>;
```

### 31.8 mux-api

```rust
pub struct ServerBuilder { ... }
pub struct Server { ... }

impl ServerBuilder {
    pub fn new() -> Self;
    pub fn name(self, name: &str) -> Self;
    pub fn size(self, size: Size) -> Self;
    pub fn build(self) -> Result<Server, MuxError>;
}

impl Server {
    pub fn session_count(&self) -> usize;
    pub fn create_session(&mut self, name: &str) -> Result<SessionId, MuxError>;
    pub fn destroy_session(&mut self, id: SessionId);
    pub fn create_window(&mut self, sid: SessionId, name: &str) -> Result<WindowId, MuxError>;
    pub fn window_count(&self, sid: SessionId) -> usize;
    pub fn first_pane(&self, sid: SessionId) -> Option<PaneId>;
    pub fn feed_pane(&mut self, pid: PaneId, data: &[u8]) -> Result<(), MuxError>;
    pub fn rename_session(&mut self, id: SessionId, name: &str);
    pub fn kernel(&self) -> &Kernel;
}
```

### 31.9 mux-test-support

```rust
pub struct TestServer { ... }
pub struct CleanupGuard { ... }

impl TestServer {
    pub fn new() -> Self;
    pub fn with_size(size: Size) -> Self;
    pub fn feed(&mut self, data: &[u8]);
    pub fn pane_text(&self, row: u16) -> String;
    pub fn cursor(&self) -> (u16, u16);
}

impl CleanupGuard {
    pub fn new(path: PathBuf) -> Self;
}

pub fn isolated_socket_path() -> PathBuf;
```

---

## 32. Weekly Sprint Template

For teams working on TermForge, here is a suggested weekly sprint template:

### Monday: Planning
- Review phase objectives.
- Identify crates to work on this week.
- Write failing tests for the week's features.

### Tuesday-Thursday: Implementation
- Implement features to make tests pass.
- Add proptest coverage for new round-trips.
- Run clippy after each significant change.

### Friday: Integration + Review
- Run full test suite.
- Check test count and proptest coverage.
- Update documentation if APIs changed.
- Review code for invariant compliance.

### Sprint Review Checklist
- [ ] All new tests pass.
- [ ] Test count did not decrease.
- [ ] Clippy produces zero warnings.
- [ ] No new `unsafe` outside mux-pty/mux-ffi.
- [ ] No `unwrap()`/`expect()` in library code.
- [ ] Documentation updated for API changes.
- [ ] AGENTS.md updated if new rules added.

---

## 33. Glossary

| Term | Definition |
|------|-----------|
| **Arena** | An interning table that deduplicates strings and returns integer handles |
| **Arc-COW** | Copy-on-write using Rust's `Arc` reference counting |
| **CSI** | Control Sequence Introducer (ESC [) |
| **DCS** | Device Control String (0x90) |
| **DECSET/DECRST** | DEC Private Mode Set/Reset |
| **FFI** | Foreign Function Interface |
| **INV** | Invariant -- a property that must always hold |
| **Kernel** | The sans-IO core that processes events and produces effects |
| **L0-L6** | Layer numbers in the dependency graph |
| **MSRV** | Minimum Supported Rust Version |
| **OTEL** | OpenTelemetry |
| **OSC** | Operating System Command (ESC ]) |
| **PTY** | Pseudoterminal |
| **Sans-IO** | Pattern where core logic has no I/O, only pure state transitions |
| **SCM_RIGHTS** | Unix socket mechanism for passing file descriptors |
| **SD** | Settled Decision |
| **SGR** | Select Graphic Rendition (ESC [ ... m) |
| **SIGCHLD** | Signal sent when a child process exits |
| **SIGWINCH** | Signal sent when the terminal is resized |
| **SlotMap** | A generational arena data structure |
| **Termlet** | A lightweight terminal instance for programmatic testing |
| **TF01** | TermForge wire protocol version 1 |
| **VT** | Video Terminal (VT100/VT220 escape sequence family) |
