# TermForge v16 Scaffold

Rust workspace scaffold materializing the v15 definitive architecture into compilable crates.

## Workspace Crates

- `mux-types`: Core types (`Cell`, `PackedCell`, `Line`, `IdentityManifest`, `TermletError`).
- `mux-grid`: Live grid and scrollback model.
- `mux-parser`: VT byte classifier with static `CLASS_TABLE[256]`.
- `mux-snapshot`: Snapshot encoder/decoder with canonical checksum lane.
- `mux-grapheme-arena`: Grapheme extension arena with 14-bit indices.
- `mux-pty`: PTY typestate handle and unified dynamic state enum.
- `mux-proto`: Length-delimited wire protocol frames.
- `mux-time`: Deterministic wall/lamport/vector time source.
- `mux-termlet`: Termlet runtime with quota and sandbox checks.
- `mux-orm`: ORM-like query API with typed errors.
- `mux-test-support`: Test harness helpers (isolated sockets, cleanup, bounded eventually checks).

## Build & Test

```bash
cargo test --manifest-path notes/rust-architecture-approach/v0001/synthesized/Cargo.toml
```

## Design Anchors

- `compact_str::CompactString` for cell grapheme payloads (S91)
- PackedCell layout `21/15/12/2/14` (S94)
- Arc-backed COW scrollback lines (S96)
- Static parser LUT `CLASS_TABLE[256]` (S95)
- Deterministic time source (S97)
- 16-variant termlet error model (S98)
- Unified PTY state enum + typestate wrapper (S99)

## Governance Docs

- `AGENTS.md`: all 405 v15 rules with enforcement mapping.
- `notes/architecture.md`: condensed architecture reference.
- `notes/plan.md`: phase-based implementation roadmap.
