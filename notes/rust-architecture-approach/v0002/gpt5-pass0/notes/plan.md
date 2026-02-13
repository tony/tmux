# Implementation Plan

## Phase P1 (Foundations)
- Crates: `mux-types`, `mux-time`, `mux-grapheme-arena`, `mux-grid`, `mux-proto`.
- Outcome: core data model, grid COW writes, TF01 encoder/decoder.
- Tests: cell invariants, header size checks, grid dirty/revision semantics.

## Phase P2 (Kernel and Runtime)
- Crates: `mux-pty`, `mux-render`, `mux-cmd-parse`, `mux-parser`, `mux-options`, `mux-target`, `mux-format`, `mux-kernel`.
- Outcome: single-thread kernel loop, bounded channels, command parse + execution skeleton.
- Tests: command routing, channel backpressure, panic-recovery harness.

## Phase P3 (SDK Surface)
- Crates: `mux-snapshot`, `mux-termlet`, `mux-orm`, `mux-ffi`, `mux-otel`.
- Outcome: ORM traversal API and termlet capture APIs with tracing propagation.
- Tests: QuerySet behavior parity, termlet send/wait/capture snapshots.

## Phase P4 (Tooling, Compatibility, Bindings)
- Crates/tools: `mux-test-support`, `tmux-builder`, `tmux-vm`, `tmux-sniff`, `mux-doctor`, bindings.
- Outcome: tmux version matrix tests, socket isolation fixtures, Python/Node parity tests.
- Tests: compatibility suites against selected tmux versions.

## Phase P4+
- Optional CRDT feature rollout and distributed session replication tests.

## LOC Estimates (P1-P3 target ~34.6k)
- `mux-kernel`: 7600
- `mux-render`: 2100
- `mux-grid`: 2600
- `mux-proto`: 2200
- `mux-pty`: 2100
- `mux-termlet`: 1700
- `mux-orm`: 2900
- `mux-parser` + `mux-cmd-parse`: 3000
- `mux-options` + `mux-target` + `mux-format`: 2400
- `mux-test-support`: 1800
- `mux-otel`: 850
- `mux-types` + `mux-time` + `mux-snapshot` + `mux-grapheme-arena` + `mux-ffi`: 6100

## Dependency Ordering
1. `mux-types`, `mux-time`
2. `mux-grapheme-arena`, `mux-grid`
3. `mux-proto`, `mux-options`, `mux-target`, `mux-format`, `mux-parser`, `mux-cmd-parse`
4. `mux-pty`, `mux-render`
5. `mux-kernel`
6. `mux-snapshot`, `mux-termlet`, `mux-orm`, `mux-otel`, `mux-ffi`
7. `mux-test-support` and tools/bindings

## Risk Register
- PTY portability divergence across Linux/macOS/BSD.
- tmux imsg edge-case drift across versions.
- Render diff correctness under wide grapheme invalidation.
- Binding API drift from Rust ORM surface.
- Snapshot flakiness due to shell startup nondeterminism.
