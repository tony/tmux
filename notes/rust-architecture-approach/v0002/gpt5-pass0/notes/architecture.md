# TermForge v0002 Architecture

## 1. PTY Allocation
- Linux-first path uses `rustix` + `TIOCGPTPEER` for race-safe slave acquisition.
- Fallback uses `portable-pty` abstraction for macOS/BSD portability.
- `mux-pty` exposes a backend trait; unsafe syscalls stay contained in that crate.
- `SCM_RIGHTS` fd passing is supported in IPC for pane migration and external attach.

## 2. Terminal I/O and Signals
- Raw mode lifecycle is scoped: enter on attach, restore on detach and panic paths.
- `SIGWINCH` events are captured in IO runtime, forwarded to kernel via bounded channels.
- Decision: use `rustix`/`nix` for signal + fd primitives; keep `crossterm` optional and client-side only.

## 3. VT Parsing
- Baseline parser uses `vte`-style state-machine model (byte-stream deterministic parser).
- Parser output emits semantic terminal ops consumed by `mux-grid`.
- Rationale: keeps render path terminal-focused without pulling GPU emulator assumptions.

## 4. Screen Buffer and Diff
- Grid model: `Vec<Arc<LineChunk>>`, `CHUNK_SIZE=256`, COW writes.
- Dirty tracking: single `line_dirty: BitVec` plus global `revision` counter.
- Render pipeline: kernel state -> render planner -> diff encoder -> client output frames.

## 5. Process Management
- PTY children tracked by pid and pane id; kernel owns lifecycle table.
- IO task handles `SIGCHLD`, reaps zombies, emits `ChildExited` events.
- Job control passthrough is preserved by forwarding foreground process group semantics.

## 6. Multiplexer Architecture
- Settled 3-thread model:
  - Kernel: single `std::thread`, owns mutable world state.
  - IO runtime: `tokio` tasks for sockets, PTY streams, signals.
  - Render runtime: `tokio` tasks for coalesced frame production.
- Bounded lane defaults: control=512, data=1024*64KiB envelopes, render=256.

## 7. IPC and Wire Protocol
- Dual protocol:
  - Native TF01 framing (16-byte header, magic `TF01`).
  - tmux imsg compatibility mode for existing clients.
- Protocol discriminator is safe because TF01 magic starts with `0x54`.
- IPC transport is UNIX sockets with optional credential/fd passing.

## 8. Config System
- Command language parser uses recursive-descent + combinators (`mux-parser`, `mux-cmd-parse`).
- Structured config is TOML for SDK embedding defaults and profile overlays.
- Hot reload: config watcher emits deltas to kernel; invalid deltas are rejected atomically.

## 9. Test Infrastructure
- `mux-test-support` provisions unique socket paths like `/tmp/termforge-test-<pid>-<uuid>.sock`.
- `TestGuard` Drop cleanup kills child server and unlinks socket.
- tmux versions are managed by `tools/tmux-builder` and `tools/tmux-vm`.
- Snapshot tests use `insta` over captured terminal text and style metadata.

## 10. Language Bindings
- PyO3 and napi-rs bindings target the same ORM surface (`filter/get/where`).
- Bindings call into `mux-ffi` DTOs and avoid transport-specific internals.
- Async boundaries are explicit: blocking kernel calls are wrapped in runtime-safe adapters.

## 11. Termlets
- Termlet is a lightweight test pod API around PTY + parser + snapshot capture.
- Builder includes size, shell, env, timeout options.
- APIs exist in Rust, Python, Node with symmetric semantics for send/wait/capture.

## 12. OpenTelemetry
- `mux-otel` initializes tracing providers and propagates span context across lanes.
- Correlation IDs are attached to protocol envelopes and binding entrypoints.
- Metrics include render latency, command latency, PTY throughput, and queue depth.

## 13. CRDT
- Optional feature `crdt` in kernel/state crates.
- State ops are represented as causal events with vector clocks.
- Conflict strategy: pane text uses operation tombstones + last-writer tie-break on metadata.
- Non-CRDT builds pay zero runtime overhead beyond feature gating.
