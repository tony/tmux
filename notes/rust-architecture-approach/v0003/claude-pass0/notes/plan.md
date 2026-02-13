# TermForge v0003 -- Implementation Plan

## Overview

This plan lays out a phased approach to evolving the TermForge scaffold into a
production-grade terminal multiplexer SDK. Each phase builds on the previous one,
maintaining the invariant that `cargo check` and `cargo test --workspace` pass
at the end of every phase.

The scaffold (v0003) provides 23 crates + 4 tools, 367 tests, and addresses all
12 identified architecture gaps at the type/signature level. The phases below
flesh out the implementations from stubs to real behavior.

---

## Phase 0: Scaffold (COMPLETE)

**Status**: Done
**Duration**: Single pass
**Tests**: 367

What was delivered:
- Workspace Cargo.toml with all 23 crates + 4 tools
- Multi-file modules for mux-types (7 files), mux-parser (3 files), mux-grid (4 files),
  mux-kernel (6 files), mux-render (4 files)
- All 12 architecture gaps addressed in code and documentation
- CLAUDE.md, AGENTS.md, README.md, notes/architecture.md (1000+ lines)
- Rust 2024 edition, MSRV 1.85
- Zero compilation errors, zero test failures

**LOC estimate**: ~8,500 lines of Rust + ~2,500 lines of documentation

---

## Phase 1: Foundation Hardening

**Goal**: Make L0 and L1 crates production-ready with real implementations.
**Duration**: 1-2 weeks
**Target tests**: 500+

### 1.1 mux-grapheme-arena (L0)

- Replace stub arena with a real interner backed by `indexmap`
- Support grapheme cluster storage for multi-codepoint characters
- Add `GraphemeArena::shrink()` for memory reclamation
- Add property tests for roundtrip (intern -> resolve -> compare)
- Benchmark: allocation throughput, deduplication ratio on real terminal output

**LOC**: ~300 new, ~100 modified

### 1.2 mux-time (L0)

- Validate `Clock::manual()` covers all kernel test scenarios
- Add `Clock::scaled(factor)` for slow-motion replay/debugging
- Add `Deadline::is_expired(&self, now: Timestamp)` ergonomics
- Ensure `Timestamp` arithmetic never panics (saturating ops)

**LOC**: ~150 new, ~50 modified

### 1.3 mux-types (L1)

- **CellFlags**: Add exhaustive property tests verifying bit values match tmux.h:742-749
- **Colour**: Implement full RGB/256/16 colour space with pack/unpack roundtrip property tests
- **Attrs**: Add underline style variants (curly, dotted, dashed) matching modern terminals
- **KeyEvent**: Implement kitty keyboard protocol encoding for extended key reporting
- **IdGenerator**: Add thread-safe variant using `AtomicU64` for multi-threaded test harnesses
- **Geometry**: Add `Rect::intersect()`, `Rect::contains()`, `Size::area()`

**LOC**: ~600 new, ~200 modified

### Phase 1 Exit Criteria

- All L0/L1 crates have 10+ tests each
- Property tests cover all pack/unpack and roundtrip operations
- `CellFlags` bit values verified against tmux source constants
- No `todo!()` or placeholder logic remains in L0/L1

---

## Phase 2: Data Layer

**Goal**: Make L2 crates functional with real parsing, storage, and protocol handling.
**Duration**: 2-3 weeks
**Target tests**: 750+

### 2.1 mux-grid

- Implement real chunked ring buffer with configurable scrollback (default 2000 lines)
- Arc-COW line semantics: `Line::make_mut()` triggers copy only when `Arc::strong_count() > 1`
- `ChunkedGrid::resize()` with reflow (word-wrap aware line breaking)
- `ScrollRegion::scroll_up()` / `scroll_down()` with proper margin handling
- Dirty line tracking: per-line generation counter for damage-based rendering
- Wide character handling: cells with `GRID_FLAG_PADDING` for double-width chars

**LOC**: ~1,200 new, ~400 modified
**Key invariant**: `grid.line(row).len() == grid.cols()` always holds

### 2.2 mux-parser

- Complete Paul Williams VT state machine with all 14 states and transitions
- CSI parameter parsing with colon-separated sub-parameters (SGR extended colors)
- OSC string collection (window title, clipboard, hyperlinks)
- DCS passthrough (for sixel/graphics, consumed and discarded per INV-220)
- UTF-8 decoder: handle partial sequences at buffer boundaries
- Golden tests: byte-for-byte expected output for known escape sequences

**LOC**: ~1,500 new, ~300 modified
**Key test**: Parse `vttest` output and verify state machine transitions

### 2.3 mux-options

- Implement tmux option inheritance: server -> session -> window -> pane
- Type-safe option values with validation (e.g., `status-interval` must be >= 0)
- Option change notifications for reactive updates
- Support for user-defined options (`@user-option`)

**LOC**: ~400 new, ~150 modified

### 2.4 mux-target

- Full tmux target resolution: `session:window.pane` with glob patterns
- Support for `+`, `-`, `!`, `{last}`, `{next}`, `{previous}` special tokens
- Ambiguous target detection with clear error messages

**LOC**: ~350 new, ~100 modified

### 2.5 mux-format

- Implement conditional expressions: `#{?test,true,false}`
- String comparison operators: `#{==:a,b}`, `#{!=:a,b}`
- Nested format expansion
- Literal escaping for all special characters

**LOC**: ~500 new, ~150 modified

### 2.6 mux-cmd-parse

- Full tmux command grammar: flags, arguments, quoting, continuation
- Command table with argument specs (min/max args, flag definitions)
- Error recovery: partial parse results with error position

**LOC**: ~600 new, ~200 modified

### 2.7 mux-proto

- TF01 wire protocol: frame encode/decode with CRC32 validation
- Message types: command, response, notification, data
- Backpressure: flow control frames
- Version negotiation handshake

**LOC**: ~500 new, ~150 modified

### 2.8 mux-crdt

- Vector clock with efficient merge (skip identical entries)
- LWW register with wall-clock tiebreaker
- Convergence property tests with random concurrent mutation generators
- Delta-state CRDT for incremental sync

**LOC**: ~400 new, ~100 modified

### 2.9 mux-fdpass

- FdEnvelope serialization/deserialization
- Validation: fd count <= FD_LIMIT (16)
- Integration with mux-pty for actual sendmsg/recvmsg (unsafe quarantined there)

**LOC**: ~200 new, ~50 modified

### Phase 2 Exit Criteria

- VT parser passes golden tests for all common escape sequences
- Grid resize produces correct output for all edge cases (shrink, grow, reflow)
- Option inheritance chain resolves correctly
- Wire protocol roundtrips all message types
- CRDT convergence verified via proptest

---

## Phase 3: Logic Layer

**Goal**: Make L3 crates functional -- the kernel processes real events, the renderer
produces real terminal output.
**Duration**: 3-4 weeks
**Target tests**: 1,100+

### 3.1 mux-kernel Core

- Event processing loop: `KernelEvent` -> state mutation -> `Vec<KernelEffect>`
- Session/Window/Pane CRUD operations with proper cleanup
- Client attach/detach with size negotiation (smallest-client-wins)
- Tick-based timer dispatch for status line refresh, activity monitoring

**LOC**: ~800 new, ~300 modified

### 3.2 mux-kernel Layout Engine

- All 5 built-in layout algorithms:
  - `even-horizontal`: equal-width columns
  - `even-vertical`: equal-height rows
  - `main-horizontal`: large top pane + equal-height bottom panes
  - `main-vertical`: large left pane + equal-width right panes
  - `tiled`: grid arrangement minimizing aspect ratio deviation
- Custom layout string parser with checksum validation
- Recursive resize propagation: parent resize -> child recalculation -> TIOCSWINSZ
- Minimum pane size enforcement (2 cols x 1 row)
- Remainder distribution: left-to-right / top-to-bottom for fractional pixels

**LOC**: ~1,000 new, ~200 modified
**Key invariant**: `sum_child_sizes() == parent_dimension` after every layout operation

### 3.3 mux-kernel Copy Mode

- Vi and Emacs key table implementations
- Selection modes: character, line, block (rectangular)
- Search: forward/reverse with regex support via `regex` crate
- Buffer ring: bounded (max 50), FIFO eviction
- Cursor movement: word, paragraph, screen-page, half-page
- Mark/jump register

**LOC**: ~700 new, ~200 modified

### 3.4 mux-kernel Pane VT Dispatch

- Wire pane output through mux-parser into grid mutations
- C0 control dispatch: BEL, BS, HT, LF, VT, FF, CR
- CSI dispatch: cursor movement (CUU/CUD/CUF/CUB/CUP), erase (ED/EL/ECH),
  scroll (SU/SD), SGR, insert/delete line (IL/DL), insert/delete char (ICH/DCH),
  set scrolling region (DECSTBM), mode set/reset (SM/RM, DECSET/DECRST)
- OSC dispatch: set title, set clipboard
- Alternate screen buffer (DECSET 1049)

**LOC**: ~1,200 new, ~300 modified

### 3.5 mux-snapshot

- Create GridSnapshot from live ChunkedGrid via Arc cloning (zero-copy for unchanged lines)
- Text extraction with style annotations
- Rectangle extraction for copy mode
- Snapshot diffing for incremental transfer

**LOC**: ~300 new, ~100 modified

### 3.6 mux-render Composition Pipeline

- CompositeGrid: blit visible panes into a single screen-sized buffer
- Border drawing: single/double/heavy line characters, active pane highlighting
- Status line composition: left, center, right sections with format evaluation
- Pane title overlay

**LOC**: ~600 new, ~200 modified

### 3.7 mux-render Diff Engine

- Double-buffer diff: compare prev vs next CompositeGrid cell-by-cell
- Emit minimal escape sequence stream:
  - CUP only when cursor position is non-adjacent
  - SGR only when attributes change from previous cell
  - Wide-char invalidation: re-emit padding cells with their parent
- Output buffer with configurable flush strategy

**LOC**: ~500 new, ~150 modified

### 3.8 mux-render SGR Encoder

- Full SGR attribute encoding: bold, dim, italic, underline (5 styles),
  blink, reverse, hidden, strikethrough
- Foreground/background color encoding: 16, 256, RGB
- SGR optimization: track current state, only emit deltas
- Reset optimization: use SGR 0 when it saves bytes vs individual resets

**LOC**: ~400 new, ~100 modified

### 3.9 mux-render Flood Fairness

- Per-pane row quota with configurable limit (default 256 rows/tick)
- Stride rotation: round-robin starting pane index to prevent starvation
- Deferred row tracking: rows that could not be rendered are queued for next tick
- Adaptive scheduling: increase quota for idle panes, decrease for floody ones

**LOC**: ~300 new, ~100 modified

### 3.10 mux-orm

- `ServerView` traversal: server -> sessions -> windows -> panes
- QuerySet-like filtering: `.sessions().filter(|s| s.name().contains("dev"))`
- Lazy evaluation: queries build filter chains, execute on iteration
- Read-only views that do not require mutable access to the kernel

**LOC**: ~500 new, ~150 modified

### 3.11 mux-pty

- PTY allocation with TIOCGPTPEER on Linux 4.13+ (race-free)
- Fallback to traditional openpty() on older kernels / BSD
- RawModeGuard RAII: save terminal state, cfmakeraw, restore on drop
- Terminal mode policies: Managed, External, SaveRestore
- TIOCSWINSZ propagation for pane resize
- SCM_RIGHTS sendmsg/recvmsg for fd passing (unsafe quarantined here)

**LOC**: ~800 new, ~200 modified

### Phase 3 Exit Criteria

- Kernel creates sessions/windows/panes and routes PTY output to grid
- Layout engine produces correct geometry for all 5 built-in algorithms
- Renderer produces minimal escape sequences (verified by golden tests)
- Copy mode Vi/Emacs key tables navigate and select correctly
- Flood fairness prevents starvation (verified by multi-pane stress test)
- ORM traversal matches kernel state exactly

---

## Phase 4: Integration Layer

**Goal**: Wire everything together into a usable SDK and test framework.
**Duration**: 2-3 weeks
**Target tests**: 1,400+

### 4.1 mux-termlet

- Termlet: self-contained mini terminal emulator for testing
- API: `Termlet::new(cols, rows)` -> spawn shell -> send keys -> capture snapshot
- Resize support: `termlet.resize(new_cols, new_rows)`
- Content waiting: `termlet.wait_for_content("pattern", timeout)`
- Snapshot comparison: `termlet.assert_snapshot("test-name")`
- Background task support: send to background, check status, bring foreground

**LOC**: ~800 new, ~100 modified

### 4.2 mux-config

- TOML config file parsing with serde
- Config -> OptionTable conversion
- Config file watching with debounced reload
- Merge chain: defaults -> system config -> user config -> session overrides
- Validation with clear error messages and line numbers

**LOC**: ~400 new, ~100 modified

### 4.3 mux-api

- MuxServerBuilder: fluent API for server construction
- INV-220 enforcement: `allow_passthrough(false)` is the default, builder warns on enable
- Server lifecycle: start, attach client, detach, shutdown
- In-process mode: no Unix socket, direct function calls
- Embedded mode: Unix socket server for external client connections

**LOC**: ~600 new, ~200 modified

### 4.4 mux-test-support

- TestServer: ephemeral server with isolated socket path
- Socket naming: `/tmp/mux-test-<PID>-<RANDOM>/default` (never conflicts with live tmux)
- Automatic cleanup on drop (even on panic)
- Clock injection: `Clock::manual()` for deterministic time
- Tmux version awareness: skip tests requiring features above the available tmux version
- Test fixtures: pre-built sessions, windows, panes for common scenarios
- Snapshot testing: `assert_snapshot!()` macro with `insta`-style review workflow

**LOC**: ~700 new, ~150 modified

### Phase 4 Exit Criteria

- Termlet can spawn a shell, send commands, and verify output
- Config file round-trips through parse -> serialize
- TestServer creates isolated environments that never touch default tmux sockets
- MuxServerBuilder produces a functional server in both in-process and socket modes
- Snapshot tests produce reproducible output across runs

---

## Phase 5: Surface Layer

**Goal**: FFI, telemetry, and external interfaces.
**Duration**: 1-2 weeks
**Target tests**: 1,550+

### 5.1 mux-ffi

- C API: `mux_server_new()`, `mux_session_create()`, `mux_pane_send_keys()`, etc.
- `catch_unwind` at every FFI boundary (INV-117)
- Opaque handle types with explicit lifetime management
- Error reporting: `mux_last_error()` returns C string
- Header generation: `cbindgen` for automatic C header
- Python/Node binding compatibility: handle types are pointer-sized

**LOC**: ~600 new, ~200 modified

### 5.2 mux-otel

- OpenTelemetry integration with configurable exporter
- Span creation for: session lifecycle, pane I/O, render cycles, command dispatch
- Context propagation across thread boundaries (IO -> Kernel -> Render)
- Disabled by default, zero-cost when off (compile-time feature gate)
- Trace ID correlation across client/server boundary

**LOC**: ~400 new, ~100 modified

### Phase 5 Exit Criteria

- C header compiles with a C compiler
- FFI functions handle null pointers, panics, and invalid handles gracefully
- OTel spans appear in a local Jaeger instance when enabled
- OTel has zero overhead when disabled (verified by benchmark)

---

## Phase 6: Tooling

**Goal**: Build the developer tools for tmux version management, debugging, and diagnostics.
**Duration**: 1-2 weeks
**Target tests**: 1,650+

### 6.1 tmux-builder

- Download and compile tmux from source at any version (3.0a through 3.5a)
- Cache compiled binaries in `~/.local/share/mux-builder/`
- Version parsing and comparison
- Cross-compilation support (if host tools available)
- Checksum verification of downloaded tarballs

**LOC**: ~500 new, ~100 modified

### 6.2 tmux-vm

- Manage multiple tmux installations
- `tmux-vm install 3.3a` -> download, compile, install to managed path
- `tmux-vm use 3.3a` -> symlink to PATH-accessible location
- `tmux-vm run 3.3a -- list-sessions` -> run specific version
- Integration with mux-test-support for version-specific testing

**LOC**: ~400 new, ~100 modified

### 6.3 tmux-sniff

- Protocol sniffer for tmux wire protocol
- Live capture from Unix socket
- Hex dump and structured decode of TF01 frames
- Filter by message type, session, or pane
- Replay from capture file

**LOC**: ~400 new, ~100 modified

### 6.4 mux-doctor

- Environment diagnostic tool
- Check: TERM, LANG/LC_*, terminal capabilities (via terminfo)
- Check: tmux socket accessibility, running servers
- Check: PTY availability (try allocating one)
- Check: signal-hook compatibility
- Output: structured JSON or human-readable report

**LOC**: ~300 new, ~100 modified

### Phase 6 Exit Criteria

- `tmux-builder` can compile tmux 3.5a from source
- `tmux-vm` can switch between installed versions
- `tmux-sniff` can decode live protocol traffic
- `mux-doctor` produces accurate environment reports

---

## Phase 7: Language Bindings

**Goal**: Python and Node.js bindings with expressive APIs matching libtmux's ergonomics.
**Duration**: 3-4 weeks
**Target tests**: 2,000+ (including binding tests)

### 7.1 Python Bindings (PyO3)

- `mux-python` crate using PyO3
- Object graph traversal: `server.sessions[0].windows[0].panes[0]`
- QuerySet-like filtering: `server.sessions.filter(name="dev")`
- Termlet API: `Termlet(cols=80, rows=24)` with context manager
- Snapshot testing: `termlet.assert_snapshot("test-name")` with pytest plugin
- Async support: `await termlet.wait_for_content("pattern")`

**LOC**: ~1,500 Rust + ~800 Python

### 7.2 Node.js Bindings (napi-rs)

- `mux-node` crate using napi-rs
- Object graph traversal matching Python API shape
- Termlet API with vitest snapshot integration
- Promise-based async: `await termlet.waitForContent("pattern")`
- TypeScript type definitions generated from Rust types

**LOC**: ~1,500 Rust + ~800 TypeScript

### 7.3 pytest Plugin

- `pytest-mux` package
- Fixtures: `mux_server`, `mux_session`, `termlet`
- Automatic cleanup of test servers and sockets
- Snapshot assertion with `--snapshot-update` flag
- tmux version parametrize: `@pytest.mark.tmux_version(">=3.3a")`

**LOC**: ~600 Python

### 7.4 vitest Plugin

- `vitest-mux` package
- Test helpers: `createTermlet()`, `createServer()`
- Snapshot matchers: `expect(termlet).toMatchSnapshot()`
- Automatic cleanup via `afterEach` hooks

**LOC**: ~500 TypeScript

### Phase 7 Exit Criteria

- Python: `pip install mux-python` works, pytest plugin discovers fixtures
- Node.js: `npm install @mux/node` works, vitest plugin integrates
- Both bindings support full object graph traversal
- Termlet works identically in Rust, Python, and Node.js
- Snapshot tests produce identical output across all three languages

---

## Phase 8: IO Thread and Server Runtime

**Goal**: Build the real IO thread with tokio, Unix socket server, and client handling.
**Duration**: 2-3 weeks
**Target tests**: 2,200+

### 8.1 IO Thread

- Tokio runtime on dedicated OS thread
- Accept loop for Unix domain socket connections
- PTY I/O multiplexing: read from all active PTYs, dispatch to kernel via crossbeam
- Client I/O: read commands, write rendered frames
- Signal bridge: convert Unix signals to KernelEvents via pipe

**LOC**: ~1,000 new

### 8.2 Channel Architecture

- crossbeam bounded channels with documented bounds:
  - control: 512 (client commands)
  - data: 1024 x 64KiB (PTY output)
  - render: 256 (rendered frames)
  - effect: 1024 (kernel effects)
- Backpressure handling: drop oldest PTY data frame (not command frames)
- Channel health monitoring: track queue depth, alert on sustained high-water

**LOC**: ~400 new

### 8.3 Render Thread

- Tokio task on the IO thread's runtime
- Receives GridSnapshots from kernel via crossbeam
- Runs composition + diff pipeline
- Writes rendered bytes to client connections
- Frame rate limiting: max 60fps, adaptive based on client write speed

**LOC**: ~500 new

### 8.4 Ordered Shutdown

- Shutdown sequence: stop accept -> drain clients -> flush render -> stop kernel -> close PTYs
- Crash frame: on panic, capture kernel state snapshot before unwinding
- Client disconnect: graceful removal from render pipeline
- SIGTERM/SIGINT: initiate ordered shutdown

**LOC**: ~400 new

### Phase 8 Exit Criteria

- Server accepts client connections over Unix socket
- PTY output flows through kernel to rendered client output
- Channel backpressure does not cause deadlocks
- Shutdown is ordered and does not lose in-flight data
- Crash frame captures useful diagnostic state

---

## Phase 9: tmux Compatibility Mode

**Goal**: Wire-level compatibility with tmux v3.5a clients and servers.
**Duration**: 3-4 weeks
**Target tests**: 2,500+

### 9.1 tmux Wire Protocol v8

- Parse tmux's native wire format (not TF01)
- Command dispatch: map tmux commands to kernel operations
- Response formatting: match tmux's output format exactly
- Control mode (-C): structured output with %begin/%end delimiters

**LOC**: ~1,500 new

### 9.2 tmux Command Set

- Implement the most-used tmux commands (prioritized by frequency):
  - Session: new-session, kill-session, switch-client, list-sessions
  - Window: new-window, kill-window, select-window, list-windows, rename-window
  - Pane: split-window, select-pane, resize-pane, list-panes, send-keys
  - Layout: select-layout, next-layout
  - Buffer: copy-mode, paste-buffer, list-buffers
  - Config: set-option, show-options, source-file, bind-key, unbind-key
  - Info: display-message, list-keys, show-environment

**LOC**: ~2,000 new

### 9.3 Compatibility Testing

- For each implemented command, run against real tmux (via tmux-vm) and compare output
- Golden test suite: capture tmux output, verify TermForge matches byte-for-byte
- Behavioral tests: create sessions/windows/panes via tmux protocol, verify state

**LOC**: ~1,000 tests

### Phase 9 Exit Criteria

- A tmux client can connect to TermForge server and perform basic operations
- `list-sessions`, `list-windows`, `list-panes` output matches tmux format
- `send-keys` and `capture-pane` work correctly
- Control mode produces valid structured output

---

## Phase 10: Polish and Production Readiness

**Goal**: Error handling, documentation, benchmarks, and release preparation.
**Duration**: 2-3 weeks
**Target tests**: 2,800+

### 10.1 Error Handling

- Audit all `Result` return types for appropriate error variants
- PTY exhaustion: graceful degradation with clear error message
- Client disconnect: clean removal without affecting other clients
- Signal races: verify SIGCHLD handling is race-free
- Add `# Errors` doc section to all public functions returning Result

**LOC**: ~500 modified

### 10.2 Documentation

- All public items have doc comments
- Crate-level documentation with examples
- Architecture guide for contributors
- API reference with cross-links between crates
- Invariant catalog with tmux source references

**LOC**: ~3,000 lines of documentation

### 10.3 Benchmarks

- Grid operations: write, scroll, resize, snapshot
- Parser: throughput on realistic terminal output
- Renderer: diff speed, SGR encoding, composition
- Protocol: frame encode/decode throughput
- End-to-end: PTY output to rendered client bytes

**LOC**: ~500 benchmark code

### 10.4 CI/CD

- GitHub Actions workflow: check, test, clippy, fmt, doc, miri
- Platform matrix: Linux x86_64, Linux aarch64, macOS x86_64, macOS aarch64
- Release automation: version bump, changelog, crate publish order
- Coverage tracking with threshold gates

**LOC**: ~300 YAML

### Phase 10 Exit Criteria

- All public APIs have doc comments with examples
- Benchmarks establish performance baselines
- CI passes on all supported platforms
- No `clippy::unwrap_used`, `clippy::expect_used`, or `clippy::panic` violations
- Ready for initial crate publish

---

## LOC Summary by Phase

| Phase | Description | Estimated LOC | Cumulative Tests |
|-------|-------------|---------------|------------------|
| 0 | Scaffold (complete) | 11,000 | 367 |
| 1 | Foundation Hardening | 1,350 | 500+ |
| 2 | Data Layer | 5,650 | 750+ |
| 3 | Logic Layer | 7,600 | 1,100+ |
| 4 | Integration Layer | 2,700 | 1,400+ |
| 5 | Surface Layer | 1,300 | 1,550+ |
| 6 | Tooling | 1,700 | 1,650+ |
| 7 | Language Bindings | 5,700 | 2,000+ |
| 8 | IO Thread + Server | 2,300 | 2,200+ |
| 9 | tmux Compatibility | 4,500 | 2,500+ |
| 10 | Polish + Production | 4,300 | 2,800+ |
| **Total** | | **~48,100** | **2,800+** |

---

## Dependency Graph Between Phases

```
Phase 0 (scaffold)
    |
    v
Phase 1 (L0/L1 hardening)
    |
    v
Phase 2 (L2 data) --------+
    |                      |
    v                      v
Phase 3 (L3 logic) --> Phase 4 (L4 integration)
    |                      |
    +------+------+--------+
           |      |
           v      v
     Phase 5   Phase 6
     (FFI/    (tooling)
      OTel)       |
           |      |
           v      v
         Phase 7 (language bindings)
           |
           v
         Phase 8 (IO thread + server)
           |
           v
         Phase 9 (tmux compat)
           |
           v
         Phase 10 (polish)
```

Phases 5, 6, and 7 can proceed in parallel once Phase 4 is complete.
Phase 7 can start partially during Phase 4 (binding scaffolding).

---

## Risk Register

### R1: VT Parser Completeness (Phase 2/3)
**Risk**: Real-world terminal output triggers parser states not covered by golden tests.
**Mitigation**: Run parser against `vttest` output, use fuzzing (`cargo-fuzz`) to find edge cases.
**Probability**: High. **Impact**: Medium.

### R2: Grid Reflow Correctness (Phase 2)
**Risk**: Resize with reflow breaks wrapped lines in subtle ways.
**Mitigation**: Port tmux's reflow test cases. Add property tests: reflow(reflow(grid, w1), w2) where w2 == original width should reproduce original grid.
**Probability**: High. **Impact**: Medium.

### R3: tmux Wire Protocol Drift (Phase 9)
**Risk**: tmux wire protocol is undocumented and changes between versions.
**Mitigation**: tmux-vm provides instant access to multiple versions. Golden tests against each supported version. Binary diffing of protocol frames.
**Probability**: Medium. **Impact**: High.

### R4: Language Binding Maintenance Burden (Phase 7)
**Risk**: PyO3/napi-rs version churn causes build failures.
**Mitigation**: Pin binding dependency versions. CI tests both latest and pinned versions. Binding API is thin wrapper over mux-ffi, reducing surface area.
**Probability**: Medium. **Impact**: Medium.

### R5: Channel Deadlock Under Load (Phase 8)
**Risk**: Bounded channels with complex flow between 3 threads can deadlock.
**Mitigation**: Dedicated backpressure strategy (drop oldest data, never drop commands). Timeout on channel sends with diagnostic logging. Deadlock detection in test harness using `Clock::manual()` to simulate slow consumers.
**Probability**: Medium. **Impact**: High.

### R6: SCM_RIGHTS Portability (Phase 3)
**Risk**: fd passing via sendmsg/recvmsg has platform-specific behavior (Linux vs macOS vs BSD).
**Mitigation**: Platform-specific implementations behind cfg attributes. Test on all target platforms in CI. Fallback to fd number sharing for in-process mode.
**Probability**: Low. **Impact**: Medium.

---

## Quality Gates (Applied at Every Phase)

1. **Compilation**: `cargo check` and `cargo check --tests` pass with zero errors
2. **Tests**: `cargo test --workspace` passes with zero failures
3. **Lints**: `cargo clippy --workspace --all-targets -- -D warnings` clean
4. **Format**: `cargo fmt --all -- --check` clean
5. **Documentation**: All new public items have doc comments
6. **Invariants**: Any invariant touched has a corresponding test
7. **Architecture**: No cross-layer dependency cycles introduced

---

## Decision Log

### D1: Own VT Parser vs. vte/vt100/termwiz
**Decision**: Own parser (mux-parser).
**Rationale**: tmux behavioral compatibility requires matching tmux's specific VT interpretation, which differs from standard terminal emulators in edge cases (e.g., how tmux handles malformed CSI sequences, its specific DECSTBM behavior). An owned parser allows exact behavioral matching. The vte crate is a push-based state machine that would require significant wrapping to match tmux's pull-based processing model.

### D2: Own Grid vs. alacritty_terminal
**Decision**: Own grid (mux-grid).
**Rationale**: alacritty_terminal's grid is optimized for GPU rendering (single contiguous buffer, damage tracking for texture upload). A multiplexer needs terminal-to-terminal rendering where Arc-COW lines enable cheap snapshots for the render pipeline. The chunked ring buffer design matches tmux's scrollback architecture more closely.

### D3: signal-hook vs. tokio signals
**Decision**: signal-hook for the pipe-based signal bridge.
**Rationale**: The kernel thread is not a tokio task -- it runs on `std::thread`. signal-hook provides pipe-based signal delivery that integrates with any event loop, while tokio's signal support requires a tokio runtime context. signal-hook's `SignalFd` can be polled alongside crossbeam channels using `crossbeam-channel::Select`.

### D4: crossterm Rejection
**Decision**: Do not use crossterm.
**Rationale**: crossterm abstracts away the platform-specific ioctls (TIOCGWINSZ, TIOCGPTPEER, TIOCSWINSZ) that a multiplexer needs direct access to. crossterm's event model assumes a single terminal, not a multiplexer managing many PTYs. Direct `nix`/`libc` calls provide the required control.

### D5: Sans-IO Kernel Threading
**Decision**: Single-threaded kernel on `std::thread`, not tokio.
**Rationale**: The kernel is a pure state machine with no I/O. Running it on a dedicated OS thread (not a tokio task) ensures it is never preempted by other async tasks and has predictable, low-jitter latency. All communication is via crossbeam bounded channels, which provide efficient cross-thread messaging without async runtime overhead.

### D6: Bincode for Internal Protocol, tmux Format for Compatibility
**Decision**: Dual protocol.
**Rationale**: The TF01 native protocol uses bincode (via serde) for type-safe, version-matched Rust-to-Rust communication. The tmux compatibility layer speaks tmux's native wire format. Both share the same kernel command model -- the protocol layer is a thin translation.

---

## Milestone Dates (Estimated)

These estimates assume a single full-time developer. Parallelism across contributors
would compress the timeline, particularly for Phases 5-7 which are independent.

| Milestone | Phase | Estimated Completion |
|-----------|-------|---------------------|
| Foundation solid | 1 | Week 2 |
| Data layer functional | 2 | Week 5 |
| Kernel + renderer working | 3 | Week 9 |
| SDK usable for testing | 4 | Week 12 |
| FFI + OTel ready | 5 | Week 14 |
| Tooling complete | 6 | Week 14 (parallel with 5) |
| Language bindings alpha | 7 | Week 18 |
| Server runtime | 8 | Week 21 |
| tmux compat alpha | 9 | Week 25 |
| Production ready | 10 | Week 28 |

---

## Success Criteria

The TermForge project is considered complete when:

1. A tmux client can connect to a TermForge server and work normally
2. Python and Node.js bindings provide libtmux-equivalent expressiveness
3. Termlets enable snapshot testing of terminal interactions in all three languages
4. The test suite exceeds 2,500 tests with zero flaky tests
5. OpenTelemetry traces flow across server, client, and language bindings
6. `mux-doctor` reports a clean environment on Linux and macOS
7. All 12 architecture gaps from the original analysis are fully implemented
8. Performance benchmarks show parity with or better than tmux for common operations
