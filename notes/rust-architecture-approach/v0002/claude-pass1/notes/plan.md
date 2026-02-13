# TermForge Implementation Plan -- v0002 Pass 1

## LOC Estimates (revised from GPT's ~34.6k)

| Crate | Estimated LOC | Phase |
|-------|--------------|-------|
| mux-grapheme-arena | 250 | P0 |
| mux-time | 200 | P0 |
| mux-types | 500 | P0 |
| mux-grid | 1,200 | P0 |
| mux-parser | 2,500 | P1 |
| mux-proto | 1,500 | P1 |
| mux-options | 600 | P1 |
| mux-target | 400 | P0 |
| mux-format | 800 | P1 |
| mux-config | 500 | P1 |
| mux-snapshot | 400 | P0 |
| mux-cmd-parse | 1,200 | P1 |
| mux-orm | 400 | P1 |
| mux-render | 2,800 | P1 |
| mux-pty | 1,000 | P1 |
| mux-kernel | 7,600 | P1-P2 |
| mux-termlet | 600 | P1 |
| mux-api | 800 | P1 |
| mux-test-support | 500 | P1 |
| mux-ffi | 400 | P2 |
| mux-otel | 300 | P2 |
| tmux-builder | 2,000 | P2 |
| tmux-vm | 1,500 | P2 |
| tmux-sniff | 800 | P2 |
| mux-doctor | 500 | P2 |
| Integration tests | 2,000 | P1-P2 |
| **Total** | **~30,750** | |

## Phase Schedule

### P0: Foundation (Weeks 1-3) -- Complete in scaffold
- mux-grapheme-arena, mux-time, mux-types, mux-grid, mux-snapshot, mux-target
- All types compile, all unit tests pass
- Workspace Cargo.toml with all lints and profiles

### P1: Core Engine (Weeks 4-10)
- mux-parser: Full Paul Williams state machine, CSI/OSC/DCS dispatch
- mux-proto: TF01 encode/decode with CRC32, imsg compat layer, SCM_RIGHTS
- mux-kernel:
  - Entity model CRUD (session, window, pane, client)
  - Layout engine (7 algorithms + custom layout strings)
  - Copy mode state machine (vi + emacs bindings)
  - Key binding dispatch (prefix mode, root mode, copy mode)
  - VT output processing (~40 CSI sequences, SGR, cursor movement)
  - Mouse dispatch (pane selection, drag resize, scroll)
- mux-render: CompositeGrid, dirty-line diff, output coalescing, border rendering
- mux-pty: PTY allocation, spawn_child (fork/setsid/exec), TIOCGPTPEER
- mux-cmd-parse: ~30 initial commands (new-session, split-window, etc.)
- mux-api: MuxServerBuilder, MuxServer handle, session/window/pane API
- mux-config: TOML parser, config hierarchy
- mux-options: Default option values for all tmux-compatible options
- mux-orm: ServerView construction from kernel state
- mux-termlet: Full integration with PTY and VT parser
- mux-test-support: Fixtures, assertion helpers, TestServer

### P2: Full Compatibility (Weeks 11-18)
- mux-kernel: Remaining ~110 tmux commands, alert system, environment inheritance
- mux-render: Status line, pane title, clock mode
- mux-ffi: Full C ABI surface for grid, parser, protocol
- mux-otel: OTLP export, span filtering
- tmux-builder: YAML/TOML workspace config
- tmux-vm: Headless server for CI
- tmux-sniff: Wire protocol inspector
- mux-doctor: Diagnostic tool
- Graphics passthrough (sixel, Kitty, iTerm2)

### P3: Advanced Features (Weeks 19-24)
- CRDT for collaborative sessions
- Native graphics protocol support (sixel rendering)
- WASM plugin model
- Floating/stacked panes
- Multi-server federation
- Performance benchmarks (latency, throughput, memory)

## Risk Register

| Risk | Impact | Mitigation |
|------|--------|------------|
| Layout engine complexity | High | Port tmux's tested algorithms directly |
| Copy mode edge cases | Medium | Use tmux's test suite as oracle |
| Graphics passthrough security | High | Disabled by default, audit DCS sequences |
| Performance regression | High | Benchmark suite from P1, regression CI |
| signal-hook + tokio interaction | Medium | Pipe-based integration, not raw handlers |
| Wide char reflow correctness | High | Proptest with CJK character generators |
| Custom layout string parsing | Medium | Fuzz with tmux-generated layout strings |
