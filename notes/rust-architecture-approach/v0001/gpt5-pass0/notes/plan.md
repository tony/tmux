# TermForge v16 Implementation Plan

## Phase 1 — Foundations (`mux-types` + `mux-parser`)

- Implement `Cell`, `PackedCell`, `Line`, `IdentityManifest`, and `TermletError` with unit coverage.
- Lock parser LUT (`CLASS_TABLE[256]`) and oracle parity tests.
- Exit criteria: all foundation crates pass tests and compile under Rust 2021.

## Phase 2 — State Core (`mux-grid` + `mux-snapshot`)

- Implement live grid mutation APIs and Arc-backed scrollback snapshots.
- Implement snapshot encode/decode with fixed header and checksum validation.
- Integrate packed-cell payload codec and basic forward/backward compatibility checks.
- Exit criteria: deterministic grid/snapshot round-trips with corruption detection tests.

## Phase 3 — Runtime IO (`mux-pty` + `mux-proto`)

- Implement typestate `PtyHandle<S>` and shared `InnerPtyState` dynamic model.
- Implement length-delimited wire protocol frames and strict decoding.
- Exit criteria: transition legality tests and protocol frame fuzz-smoke suite.

## Phase 4 — Execution Harness (`mux-termlet` + `mux-test-support`)

- Build termlet runtime with quota admission, sandbox checks, and deterministic recorder output.
- Provide test harness primitives for isolated sockets and bounded async assertions.
- Exit criteria: integration tests validate deterministic replay and error typing.

## Phase 5 — Data API + Bindings (`mux-orm` + language bindings)

- Implement ORM-like `QueryList` API with typed retrieval errors.
- Add Python/Node binding adapters over stable wire and error contracts.
- Exit criteria: cross-language parity tests for query semantics and error codes.

## Cross-Phase Quality Gates

- Determinism: fixed seed + deterministic time required in all public API tests.
- Compatibility: snapshot/protocol compatibility tests in CI matrix.
- Safety: no panics in library paths; typed errors only.
- Governance: rule/risk updates require AGENTS + architecture doc delta in same change.
