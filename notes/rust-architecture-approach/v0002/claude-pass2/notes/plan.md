# TermForge Implementation Plan -- v0002 Pass 2 (FINAL)

## Overview

This plan describes the phased implementation of TermForge from the current
scaffold (21 crates, 4 tools, 192 tests) to a production-ready terminal
multiplexer SDK with full tmux v3.5a compatibility.

The scaffold already compiles (`cargo check`) and all 192 tests pass
(`cargo test`). Each phase builds on the previous, maintaining this invariant:
every commit must pass `cargo check && cargo test`.

---

## LOC Estimates

| Crate | Scaffold LOC | Estimated Final LOC | Phase |
|-------|-------------|---------------------|-------|
| mux-grapheme-arena | ~130 | 250 | P0 |
| mux-time | ~100 | 200 | P0 |
| mux-types | ~400 | 600 | P0 |
| mux-grid | ~300 | 1,200 | P0 |
| mux-parser | ~400 | 2,500 | P1 |
| mux-proto | ~120 | 1,500 | P1 |
| mux-options | ~100 | 600 | P1 |
| mux-target | ~120 | 400 | P0 |
| mux-format | ~100 | 800 | P1 |
| mux-config | ~80 | 500 | P1 |
| mux-snapshot | ~70 | 400 | P0 |
| mux-cmd-parse | ~120 | 1,200 | P1 |
| mux-orm | ~100 | 400 | P1 |
| mux-render | ~300 | 3,000 | P1 |
| mux-pty | ~80 | 1,000 | P1 |
| mux-kernel | ~600 | 8,000 | P1-P2 |
| mux-termlet | ~80 | 600 | P1 |
| mux-api | ~120 | 800 | P1 |
| mux-test-support | ~80 | 500 | P1 |
| mux-ffi | ~80 | 400 | P2 |
| mux-otel | ~60 | 300 | P2 |
| tmux-builder | stub | 2,000 | P2 |
| tmux-vm | stub | 1,500 | P2 |
| tmux-sniff | stub | 800 | P2 |
| mux-doctor | stub | 500 | P2 |
| Integration tests | 0 | 2,000 | P1-P2 |
| **Total** | **~3,540** | **~31,450** | |

---

## Phase P0: Foundation (Weeks 1-3) -- Mostly Complete in Scaffold

### Goal
All L0-L1 crates are feature-complete with comprehensive tests.

### Deliverables

- **mux-grapheme-arena**: Complete arena allocator with deduplication, inline
  optimization for ASCII/BMP characters, and test coverage for collision
  handling and capacity limits.

- **mux-time**: Injectable Clock with System and Manual variants. MonoTime
  and WallTime types. Clock::manual() enables fully deterministic tests.

- **mux-types**: All 6 modules feature-complete:
  - `cell.rs`: Cell struct with CellFlags matching tmux's GRID_FLAG_* exactly
    (INV-119, verified against tmux.h:742-749).
  - `style.rs`: Colour (Default/Indexed/Rgb) with pack/unpack roundtrip.
    Attrs bitflags with 13 attribute flags.
  - `identity.rs`: SessionId, WindowId, PaneId, ClientId newtypes.
  - `error.rs`: TermForgeError with 17 variants via thiserror.
  - `input.rs`: KeyCode, KeyModifiers, KeyEvent, MouseEvent.
  - `geometry.rs`: Size with area() and meets_minimum().

- **mux-grid**: Chunked COW grid (64-line chunks) with:
  - Arc-based Line with COW via Arc::make_mut.
  - Dirty tracking per-row.
  - Scrollback management with hlimit.
  - Grid reflow (marks all lines dirty).
  - GridSnapshot for render pipeline.

- **mux-target**: tmux target syntax parser (session:window.pane).

- **mux-snapshot**: Grid text extraction and diff.

### Acceptance Criteria
- `cargo test` passes with 200+ tests.
- All property tests (proptest) for colour roundtrip and layout dimensions.
- CellFlags values verified against tmux source.

### Status
**~80% complete**. The scaffold has all types, grid, and tests. Remaining
work: expand grapheme arena capacity testing, add proptest for all roundtrips,
expand grid reflow to handle wide characters.

---

## Phase P1: Core Engine (Weeks 4-10)

### Goal
The kernel can manage sessions/windows/panes, parse VT sequences,
render output, and handle commands.

### P1.1: VT Parser (Week 4-5)

Expand the 14-state Paul Williams parser to handle the full CSI/OSC/DCS
sequence repertoire that tmux's `input.c` processes:

- **CSI sequences (~40)**: Cursor movement (CUP, CUU, CUD, CUF, CUB, CNL,
  CPL, CHA, VPA, HPA), editing (ICH, DCH, IL, DL, ECH, ED, EL), scrolling
  (SU, SD, DECSTBM), modes (SM, RM, DECSM, DECRM), attributes (full SGR
  including 256-color and true-color), device status (DSR, DA, DECRPM),
  tabs (HTS, TBC).

- **OSC sequences**: Window title (OSC 0/2), clipboard (OSC 52),
  hyperlinks (OSC 8), color queries.

- **DCS sequences**: DECRQSS, XTGETTCAP, tmux passthrough.

- **Testing**: Add at least 30 parser tests covering edge cases (malformed
  sequences, UTF-8 in parameters, very long parameter lists, interleaved
  C0 controls).

### P1.2: PTY and Process Management (Week 5)

- **mux-pty**: Full PTY allocation via `posix_openpt()`/`grantpt()`/
  `unlockpt()`/`ptsname()`. TIOCGPTPEER on Linux 4.13+. RawModeGuard
  RAII pattern.

- **Process spawn**: `fork()` -> `setsid()` -> `TIOCSCTTY` -> `dup2()` ->
  `exec()`. Handle TIOCSCTTY platform variance (Linux stealing vs BSD
  fail-on-controlled).

- **Zombie prevention**: Install signal-hook pipe for SIGCHLD. IO thread
  drains `waitpid(-1, WNOHANG)` in a loop.

- **Testing**: Spawn child processes in test harness with unique socket
  names. Verify cleanup on test failure.

### P1.3: Kernel Entity CRUD (Week 6-7)

Expand the kernel's entity model to handle full lifecycle:

- `new-session`, `kill-session`, `rename-session`
- `new-window`, `kill-window`, `rename-window`, `select-window`, `swap-window`
- `split-window`, `kill-pane`, `select-pane`, `swap-pane`, `resize-pane`
- `attach-session`, `detach-client`, `switch-client`
- `send-keys` (key encoding and dispatch)

### P1.4: Layout Engine (Week 7-8)

Expand the binary tree layout engine:

- 5 built-in algorithms (even-horizontal, even-vertical, main-horizontal,
  main-vertical, tiled) -- already in scaffold.
- Resize redistribution with remainder pixel distribution.
- Custom layout string parsing with checksum validation.
- Minimum pane size enforcement (2 cols x 1 row).
- Proportional resize on terminal size change.

### P1.5: Render Pipeline (Week 8-9)

Complete the rendering system:

- **CompositeGrid**: Full double-buffer implementation with swap.
- **Blit**: Pane grid to composite grid with offset and clipping.
- **Borders**: Unicode box-drawing characters for pane separators.
- **Diff**: Cell-by-cell comparison with wide-char invalidation.
- **Output**: SGR optimization (emit only on change), CUP optimization
  (skip for adjacent cells).
- **Flood fairness**: Per-pane row quota with stride rotation (already
  in scaffold).
- **Status line**: Format string evaluation and rendering.

### P1.6: Command System (Week 9-10)

Expand mux-cmd-parse to handle the initial 30 commands:

- Session: `new-session`, `kill-session`, `has-session`, `list-sessions`,
  `rename-session`, `attach-session`, `detach-client`, `switch-client`.
- Window: `new-window`, `kill-window`, `select-window`, `rename-window`,
  `list-windows`, `swap-window`, `next-window`, `previous-window`,
  `last-window`.
- Pane: `split-window`, `kill-pane`, `select-pane`, `swap-pane`,
  `resize-pane`, `display-panes`.
- Misc: `send-keys`, `send-prefix`, `copy-mode`, `paste-buffer`,
  `list-keys`, `bind-key`, `unbind-key`.

### P1.7: SDK and Testing (Week 10)

- **mux-api**: Complete MuxServerBuilder with all configuration options.
  SessionHandle, WindowHandle, PaneHandle wrappers.
- **mux-termlet**: Full Termlet lifecycle -- spawn, send_keys, capture,
  resize, wait_for_content, shutdown.
- **mux-test-support**: TestServer with unique socket names, fixture
  builders, assertion helpers, cleanup on Drop.
- **mux-orm**: ServerView construction from kernel state. QuerySet with
  filter/get/iter.

### P1 Acceptance Criteria
- 500+ tests passing.
- tmux-compatible session/window/pane lifecycle via `mux-api`.
- Termlet can spawn a shell, send commands, and capture output.
- Render pipeline produces correct escape sequences for multi-pane layouts.
- `cargo clippy -- -D warnings` clean.

---

## Phase P2: Full Compatibility (Weeks 11-18)

### Goal
Complete tmux command coverage, language bindings scaffold, and tooling.

### P2.1: Remaining Commands (Week 11-13)

Implement the remaining ~110 tmux commands:
- `set-option`, `show-options`, `set-environment`, `show-environment`
- `source-file`, `if-shell`, `run-shell`, `wait-for`
- `display-message`, `confirm-before`, `command-prompt`
- `capture-pane`, `save-buffer`, `load-buffer`, `delete-buffer`
- `choose-tree`, `choose-client`, `choose-buffer`
- `pipe-pane`, `respawn-pane`, `respawn-window`
- `move-window`, `move-pane`, `join-pane`, `break-pane`
- `lock-server`, `lock-session`, `lock-client`
- Alert system: `monitor-activity`, `monitor-bell`, `monitor-silence`
- Environment inheritance chain

### P2.2: Protocol Layer (Week 13-14)

- **TF01**: Complete frame types (Hello, Data, Resize, Command,
  CommandResponse, KeyInput, Shutdown, Error).
- **tmux imsg**: Backward-compatible server that detects tmux clients
  by checking first 4 bytes (TFO1 magic vs imsg header).
- **SCM_RIGHTS**: fd passing for client attach and process migration
  (Unix domain sockets only).

### P2.3: FFI Layer (Week 14-15)

- Complete C API surface: grid lifecycle, parser, protocol detection,
  session/window/pane management, snapshot capture.
- cbindgen-generated header file.
- Comprehensive null-pointer and catch_unwind safety.

### P2.4: OpenTelemetry (Week 15)

- OTLP span export for server lifecycle events.
- Trace context propagation across client-server boundary.
- Per-pane command spans with PTY output correlation.
- Environment flag `TERMFORGE_OTEL_ENABLE` for opt-in.

### P2.5: Tooling (Week 16-18)

- **tmux-builder**: YAML/TOML workspace definition -> session layout.
- **tmux-vm**: tmux version manager. Download, compile, and cache tmux
  at specific versions. Provides `tmux-vm install 3.5a`, `tmux-vm use 3.5a`.
- **tmux-sniff**: Wire protocol inspector. Attach to a Unix socket and
  decode TF01/imsg frames in real-time.
- **mux-doctor**: Check environment (terminal capabilities, PTY availability,
  tmux versions, socket paths, library versions).

### P2 Acceptance Criteria
- 1000+ tests passing.
- All 59 tmux built-in commands implemented.
- TF01 protocol handles full client session lifecycle.
- C FFI tested from a C test harness.
- OTLP spans visible in Jaeger/Grafana.
- tmux-vm can install and test against tmux 3.3a, 3.4, 3.5a.

---

## Phase P3: Advanced Features (Weeks 19-24)

### P3.1: CRDT Collaborative Sessions (Week 19-20)

- Vector clock implementation for causal ordering.
- LWW register for cell-level conflict resolution.
- Multi-server federation protocol over TF01.
- Proptest verification of convergence with concurrent mutations.

### P3.2: Language Bindings (Week 20-22)

- **Python (PyO3)**: `mux-py` crate in `bindings/python/`.
  QuerySet-style traversal matching libtmux's expressiveness.
  Pytest fixtures for snapshot testing against Termlets.

- **Node.js (napi-rs)**: `mux-node` crate in `bindings/node/`.
  Promise-based API. Vitest bindings for snapshot testing.

- Both bindings expose:
  - Server/Session/Window/Pane object graph traversal.
  - Termlet creation and interaction.
  - Snapshot capture and comparison.
  - Filter/QuerySet patterns (e.g., `server.windows.filter(name="main")`).

### P3.3: Graphics Passthrough (Week 22-23)

- Sixel sequence detection and passthrough.
- Kitty graphics protocol passthrough.
- iTerm2 inline image passthrough.
- Disabled by default (INV-220), opt-in via configuration.
- DCS sequence auditing to prevent escape attacks.

### P3.4: Extended Features (Week 23-24)

- Floating panes (overlay on top of layout tree).
- Stacked panes (tab-like grouping within a layout cell).
- Custom key tables (beyond Vi/Emacs copy mode).
- WASM plugin model (Zellij-inspired, sandboxed).
- Performance benchmarks: latency, throughput, memory.

### P3 Acceptance Criteria
- 2000+ tests passing (including language binding tests).
- CRDT convergence verified via proptest (100k+ random mutations).
- Python and Node.js bindings published to PyPI / npm.
- Graphics passthrough works with Kitty terminal.
- Benchmark suite with CI regression detection.

---

## Phase P4: Production Hardening (Weeks 25-30)

### P4.1: Compatibility Testing

- Run tmux's own test suite against TermForge.
- Comparison testing: same commands -> same visual output.
- tmux-vm automates testing against tmux 3.3a, 3.4, 3.5a.

### P4.2: Fuzz Testing

- `cargo-fuzz` targets for:
  - VT parser (arbitrary byte sequences).
  - Command parser (malformed commands).
  - Protocol decoder (corrupted frames).
  - Layout string parser (invalid checksum/dimensions).
  - Grid operations (random cell mutations + reflow).

### P4.3: Performance Optimization

- Profile render pipeline with `perf` / `flamegraph`.
- Optimize hot paths: VT parser state transitions, grid COW,
  diff algorithm, SGR output.
- Memory profiling: verify grid memory is bounded by hlimit.
- Latency target: less than 1ms from PTY read to client write for single pane.

### P4.4: Documentation

- `cargo doc` for all public APIs.
- Architecture guide with diagrams.
- Quickstart tutorial for embedding.
- Migration guide from tmux to TermForge.

### P4 Acceptance Criteria
- Zero crashes from 48-hour fuzz run.
- tmux compatibility test suite pass rate above 95%.
- Render latency under 1ms (p99) for single-pane output.
- All public APIs documented with examples.

---

## Test Isolation Strategy

All test harnesses enforce isolation from the user's running tmux:

1. **Socket names**: Every test uses a unique socket name generated from
   `uuid::Uuid::new_v4()`. Socket paths use `$TMPDIR/termforge-test-<uuid>/`
   to avoid collision with `/tmp/tmux-<UID>/default`.

2. **Environment**: Tests unset `TMUX` to prevent interaction with the
   user's live tmux session.

3. **Cleanup**: `TestServer` implements `Drop` to kill child processes,
   remove socket files, and clean up temp directories. Tests that spawn
   tmux (via tmux-vm) always use `-S <unique_socket_path>`.

4. **tmux version awareness**: `TestServer::with_tmux_version("3.5a")`
   uses tmux-vm to locate or build the specified version. Tests can be
   conditionally skipped if the required version is unavailable.

---

## Termlet-First Testing Strategy

Termlets are the primary testing primitive for TermForge. They wrap a
PTY + VT parser + shell into a lightweight pod that can be used from
Rust, Python, and Node.js.

### Rust API

```rust
let termlet = Termlet::builder()
    .size(80, 24)
    .shell("/bin/bash")
    .env("TERM", "xterm-256color")
    .build()?;

termlet.send_keys("echo hello\n")?;
termlet.wait_for_text("hello", Duration::from_secs(5))?;

let snapshot = termlet.capture()?;
insta::assert_snapshot!(snapshot.text());
```

### Python API (planned)

```python
from mux import Termlet

with Termlet(cols=80, rows=24, shell="/bin/bash") as t:
    t.send_keys("echo hello\n")
    t.wait_for_text("hello", timeout=5.0)

    snapshot = t.capture()
    assert "hello" in snapshot.text
```

### Node.js API (planned)

```typescript
import { Termlet } from '@termforge/mux';

const t = await Termlet.create({ cols: 80, rows: 24, shell: '/bin/bash' });
await t.sendKeys('echo hello\n');
await t.waitForText('hello', { timeout: 5000 });

const snapshot = await t.capture();
expect(snapshot.text).toContain('hello');
await t.close();
```

---

## Risk Register

| Risk | Impact | Likelihood | Mitigation |
|------|--------|-----------|------------|
| Layout engine complexity | High | Medium | Port tmux's tested algorithms directly from layout-set.c |
| Copy mode edge cases | Medium | Medium | Use tmux's test suite as oracle |
| Graphics passthrough security | High | Low | Disabled by default (INV-220), audit DCS sequences |
| Performance regression | High | Medium | Benchmark suite from P1, regression CI |
| signal-hook + tokio interaction | Medium | Low | Pipe-based integration, not raw handlers |
| Wide char reflow correctness | High | Medium | Proptest with CJK character generators |
| Custom layout string parsing | Medium | Medium | Fuzz with tmux-generated layout strings |
| CRDT convergence bugs | High | Medium | Proptest with concurrent mutation generators |
| SCM_RIGHTS fd validation | Medium | Low | fstat + type check on received fds |
| Panic in FFI boundary | Critical | Low | catch_unwind + null return on all FFI fns |
| PyO3/napi-rs API surface | Medium | Medium | Start with minimal binding, expand incrementally |
| tmux wire protocol reverse-engineering | High | Medium | Wireshark dissector + tmux-sniff for validation |

---

## Dependency Decisions

### Adopted

| Crate | Version | Purpose | Rationale |
|-------|---------|---------|-----------|
| tokio | 1.x | Async IO thread | De facto async runtime |
| crossbeam-channel | 0.5 | Kernel-IO bridge | Lock-free, bounded, works with std::thread |
| signal-hook | 0.4 | Signal handling | Pipe-based, integrates with tokio via AsyncFd |
| nix | 0.31 | Unix syscalls | Direct control over PTY, termios, ioctl |
| bitflags | 2.x | CellFlags, Attrs | Standard bitflag crate |
| thiserror | 2.x | Library errors | Derive Error trait |
| serde + toml | 1.x / 0.8 | Configuration | Standard serialization |
| crc32fast | 1.x | Protocol CRC | Fast CRC32 for frame validation |
| insta | 1.x | Snapshot testing | Golden file comparison |
| proptest | 1.x | Property testing | Invariant verification |
| opentelemetry | 0.28 | Observability | Cross-service tracing |

### Explicitly Not Adopted

| Crate | Reason |
|-------|--------|
| crossterm | TermForge uses nix + libc for terminal I/O; crossterm adds unnecessary abstraction for a multiplexer that needs direct control |
| vt100 | TermForge has its own VT parser for full tmux behavioral compatibility |
| termwiz | Same rationale as vt100; own parser allows exact tmux sequence handling |
| portable-pty | mux-pty uses nix directly for PTY allocation to support TIOCGPTPEER and platform-specific behavior |
| pty-process | Same rationale as portable-pty |
| Ion Shell | Evaluated; non-POSIX nature is a compatibility risk. Delegation to system shells is preferred |

---

## Version Milestones

| Version | Phase | Description |
|---------|-------|-------------|
| 0.1.0 | P0 | Foundation types, grid, scaffold passes all tests |
| 0.2.0 | P1 | Core engine: VT parser, kernel, render, basic commands |
| 0.3.0 | P2 | Full tmux command set, protocol, FFI, tooling |
| 0.4.0 | P3 | CRDT, language bindings, graphics passthrough |
| 1.0.0 | P4 | Production hardened, documented, benchmarked |
