# TermForge v16 Definitive Scaffold

This directory is the definitive Rust scaffold for the TermForge architecture.

## Workspace

- Rust edition: 2021
- Workspace resolver: `2`
- Crates: 12 (`mux-*`)
- Unsafe policy: `#![forbid(unsafe_code)]` on all crates except `mux-pty`

## Crate Map

| Crate | Level | Purpose |
|---|---|---|
| `mux-types` | L0 | Core terminal data model (`Cell`, `Line`, `PackedCell`, style, identity, errors) |
| `mux-parser` | L0 | Full 14-state VT parser, `CLASS_TABLE[256]`, `classify_match()` oracle |
| `mux-grapheme-arena` | L0 | 14-bit grapheme extension arena with dedup |
| `mux-proto` | L0 | tmux-compatible wire protocol frame model |
| `mux-kernel` | L0 | Sans-IO reducer (`Event -> Vec<Effect>`), `GenSlotMap`, `ServerGraph`, OTEL stubs |
| `mux-grid` | L1 | Mutable terminal grid + bounded searchable scrollback |
| `mux-snapshot` | L1 | 31-byte snapshot header and CRC32C-based encode/decode |
| `mux-pty` | L1 | Typestate PTY handle lifecycle (`InnerPtyState`) |
| `mux-time` | L1 | Lamport clock + feature-gated VectorClock deterministic time source |
| `mux-termlet` | L2 | Termlet configuration/runtime helpers |
| `mux-orm` | L2 | Django-style `QueryList<T>` and typed query errors |
| `mux-test-support` | L2 | Differential and snapshot testing primitives |

## Build and Test

```bash
cargo test --workspace
cargo test --workspace --all-features
```

Both commands pass in this scaffold.
