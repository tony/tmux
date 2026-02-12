# TermForge v16 Pass0 (gpt5) — Spec-to-Scaffold Draft

## Objective

Translate v15 definitive architecture into a real Rust workspace with compilable crates and enforceable rule coverage.

## Pass0 Decisions

1. Use `notes/rust-architecture-approach/v0001/synthesized` as the implementation root.
2. Materialize core architecture first (`mux-types`, `mux-parser`) and build outward.
3. Keep deterministic behavior explicit in API design (time injection, monotonic transitions, stable serialization).
4. Use snapshot/header contracts directly in code (`HEADER_LEN=31`, checksum algorithm offset `30`).
5. Keep PTY state modeling dual-path: typestate API and dynamic state API over a shared `InnerPtyState` enum.

## Core Data Model Plan

- `Cell` stores grapheme via `compact_str::CompactString`.
- `PackedCell` enforces exact `21/15/12/2/14` packing with checked constructors.
- `Line` wraps `Arc<Vec<Cell>>` and uses `Arc::make_mut` for COW mutation.
- `GraphemeArena` provides deduped 14-bit extension indices with `0` reserved.

## Runtime Model Plan

- `Grid` keeps mutable live rows and pushes snapshot lines to scrollback.
- `Snapshot` encode/decode validates checksum and malformed/truncated payloads.
- `DeterministicTimeSource` provides wall, Lamport, vector clock operations.
- `TermletRuntime` enforces resource quota and sandbox checks before execution.

## Protocol/Interop Plan

- `mux-proto` implements length-delimited frame codec with strict bounds checks.
- `mux-orm` provides QueryList semantics with typed retrieval errors.
- `mux-test-support` provides isolated socket path and bounded eventual assertions.

## Documentation Plan

- Generate `AGENTS.md` from definitive v15 rules with enforcement metadata.
- Produce condensed `notes/architecture.md` containing conflicts, invariants, settled decisions, and rule index.
- Produce `notes/plan.md` with phase-by-phase implementation roadmap.

## Exit Criteria

- Workspace compiles and tests pass.
- All required files and crate modules exist.
- AGENTS contains all 405 rules from v15 definitive source.
- Architecture reference is concise but operational for day-to-day implementation.
