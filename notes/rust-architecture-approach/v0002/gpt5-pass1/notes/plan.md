# TermForge v0002 Implementation Plan

## Scope Baseline

- Estimated production code size: **~34.6k Rust LOC** (from synthesis calibration).
- Current pass is architecture + stubs for kernel layout/copy mode, render pipeline, and SDK API.

## Phase 1: Runtime Spine (Weeks 1-2)

1. Wire `mux-api` builder to real runtime threads.
2. Implement kernel-thread, io-thread, render-thread contract with bounded lanes.
3. Add crash frame emission path to connected clients before shutdown.

## Phase 2: Layout Engine Completion (Weeks 2-4)

1. Implement full tmux-compatible parser for custom layout tree strings.
2. Add checksum compatibility tests against tmux fixtures.
3. Complete resize redistribution edge cases (min-size pressure, zoom transitions).
4. Add property tests for geometry conservation invariants.

## Phase 3: Composition and Diff Optimization (Weeks 3-5)

1. Finish dirty-region tracking integration with pane output stream.
2. Tune coalescing and frame-budget heuristics using benchmark traces.
3. Add starvation tests for flood panes and multi-client fanout.
4. Add protocol-level render regression snapshots.

## Phase 4: Signals and Process Lifecycle (Weeks 4-6)

1. Integrate signal-hook receiver with runtime queue.
2. Implement SIGWINCH and SIGCHLD full loop with waitpid drain.
3. Add process-group non-interference tests and zombie leak checks.
4. Implement TIOCSWINSZ propagation to all pane PTYs.

## Phase 5: Copy Mode (Weeks 5-7)

1. Implement full vi/emacs key tables.
2. Add character/line/block selection semantics parity tests.
3. Implement scrollback incremental search with highlight state.
4. Add copy buffer management and paste ring integration.

## Phase 6: Graphics and Protocol Edges (Weeks 6-8)

1. Implement passthrough handling for sixel/iTerm2/kitty sequences in parser path.
2. Ensure payload transparency over tmux-compatible control transport.
3. Add compatibility tests for escape passthrough integrity.

## Phase 7: Bindings and SDK Stabilization (Weeks 8-10)

1. Define async boundary adapters for Python and Node.
2. Add API-level stability tests for builder and config reload behavior.
3. Document embedding contracts for raw mode ownership and signal interaction.

## Test Targets

- Unit tests: 100+ (already exceeded in scaffold).
- Property tests: layout conservation + diff determinism.
- Differential tests: tmux 3.3a/3.4/3.5/HEAD behavior traces.
- Stress tests: pane flood, resize storms, rapid process churn.
