# TermForge v0002 codex-pass2 Manifest

- Workspace members: 26
- Crates: 22
- Tools: 4
- Edition: 2024
- MSRV: 1.85
- `cargo check`: pass
- `cargo test --workspace`: pass
- Unit tests discovered (`cargo test -- --list`): 303
- Architecture document: `notes/architecture.md` (899 lines)
- Canonical guidance: `CLAUDE.md`

## Key New Crates

- `mux-render`: CompositeGrid, diff optimization, flood fairness
- `mux-api`: builder, policy, channel specs, crash frame
- `mux-config`: TOML serde config + reload planning
- `mux-term`: signal propagation model (SIGWINCH/SIGCHLD)
- `mux-crdt`: VectorClock + LWW tie-breaking
- `mux-fdpass`: SCM_RIGHTS envelope model
- `mux-client`, `mux-server`, `mux-input`, `mux-effects`

## Tools

- `tfctl`
- `tfrecord`
- `tfbench`
- `tfdecode`
