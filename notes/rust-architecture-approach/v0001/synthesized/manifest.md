# TermForge v16 Definitive Scaffold Manifest

## Summary

- Crates: 12
- Rust source files: 20
- Total Rust LOC: 6,602
- Tests declared in source (`#[test]`): 233
- Executed tests (default): 225
- Executed tests (`--all-features`): 232
- Workspace test failures: 0

## Verification

- `cargo test --workspace` passed.
- `cargo test --workspace --all-features` passed.

## Per-Crate Breakdown

| Crate | Level | Rust files | LOC | `#[test]` count | Default run | All-features run |
|---|---:|---:|---:|---:|---:|---:|
| `mux-types` | L0 | 7 | 1,456 | 62 | 62 | 62 |
| `mux-parser` | L0 | 2 | 763 | 30 | 30 | 30 |
| `mux-grid` | L1 | 2 | 550 | 20 | 20 | 20 |
| `mux-snapshot` | L1 | 1 | 411 | 11 | 11 | 11 |
| `mux-grapheme-arena` | L0 | 1 | 271 | 9 | 9 | 9 |
| `mux-pty` | L1 | 1 | 411 | 11 | 11 | 11 |
| `mux-proto` | L0 | 1 | 328 | 10 | 10 | 10 |
| `mux-time` | L1 | 1 | 408 | 13 | 6 | 13 |
| `mux-termlet` | L2 | 1 | 333 | 9 | 9 | 9 |
| `mux-orm` | L2 | 1 | 280 | 14 | 14 | 14 |
| `mux-test-support` | L2 | 1 | 401 | 14 | 14 | 14 |
| `mux-kernel` | L0 | 1 | 990 | 30 | 29 | 29 |

## Root Inventory

| Path | Lines | Notes |
|---|---:|---|
| `Cargo.toml` | 42 | Workspace members, resolver, shared deps |
| `Cargo.lock` | generated | Lockfile |
| `README.md` | 36 | Project overview and build instructions |
| `AGENTS.md` | 498 | 405+ rule catalog with enforcement methods |
| `CLAUDE.md` | 39 | Coding conventions and invariants |
| `manifest.md` | 80 | Inventory and counts |
| `notes/architecture.md` | 99 | Condensed architecture with all 34 invariants |
| `notes/plan.md` | 61 | 10-phase implementation roadmap |

## Source Inventory

### `crates/mux-types/src/`

- `lib.rs`
- `cell.rs`
- `packed_cell.rs`
- `line.rs`
- `style.rs`
- `identity.rs`
- `error.rs`

### `crates/mux-parser/src/`

- `lib.rs`
- `byte_class.rs`

### `crates/mux-grid/src/`

- `lib.rs`
- `scrollback.rs`

### Single-file crate roots

- `crates/mux-grapheme-arena/src/lib.rs`
- `crates/mux-kernel/src/lib.rs`
- `crates/mux-orm/src/lib.rs`
- `crates/mux-proto/src/lib.rs`
- `crates/mux-pty/src/lib.rs`
- `crates/mux-snapshot/src/lib.rs`
- `crates/mux-termlet/src/lib.rs`
- `crates/mux-test-support/src/lib.rs`
- `crates/mux-time/src/lib.rs`
