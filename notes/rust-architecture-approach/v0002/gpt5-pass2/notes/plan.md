# Implementation Plan (10 Phases)

## Phase 1: Toolchain and CI Baseline

- Pin Rust toolchain/MSRV and CI matrix.
- Add workspace lint/test/build jobs for default + all-features.
- Gate merges on `cargo test --workspace`.

## Phase 2: Core Type Hardening (`mux-types`)

- Stabilize `Cell`, `Line`, `Style`, `StylePack`, and `PackedCell` APIs.
- Add strict conversion/validation paths (`PackedCell::try_new`).
- Lock layout invariants with compile-time and runtime tests.

## Phase 3: Parser Completion (`mux-parser`)

- Finish unsupported-sequence behavior and logging hooks.
- Expand transition/property tests and fuzz corpus.
- Preserve `CLASS_TABLE` hot path and oracle parity.

## Phase 4: Grid + Scrollback Integration (`mux-grid`)

- Wire parser actions to grid mutations.
- Add richer scrollback query APIs and copy-mode hooks.
- Validate large-grid performance envelopes.

## Phase 5: Snapshot Reliability (`mux-snapshot`)

- Add compatibility fixtures and round-trip corpus tests.
- Integrate grapheme-arena trailer scenarios.
- Add corruption/recovery diagnostics and tooling.

## Phase 6: PTY Runtime Bridge (`mux-pty`)

- Implement platform backends for spawn/resize/kill.
- Preserve typestate guarantees and generation checks.
- Add lifecycle stress tests and failure-injection tests.

## Phase 7: Protocol + Time (`mux-proto`, `mux-time`)

- Connect wire frame decode to typed commands/events.
- Finalize deterministic clock injection points.
- Harden CRDT-gated vector-clock merge behavior.

## Phase 8: Kernel Orchestration (`mux-kernel`)

- Map parsed client commands to reducer `Event`s.
- Route `Effect`s into I/O adapter layer.
- Add replay tests for deterministic reducer traces.

## Phase 9: Higher-Level Runtime (`mux-termlet`, `mux-orm`, `mux-test-support`)

- Build end-to-end harness APIs for test workflows.
- Expand ORM query ergonomics and diagnostics.
- Add differential test runners against tmux.

## Phase 10: Productionization and Compatibility

- Add benchmarks, fuzzing gates, and compatibility scorecards.
- Enforce AGENTS rule coverage and invariant test mappings.
- Document operational runbooks and release criteria.
