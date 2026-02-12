# TermForge v16 Pass 2 Refined Scaffold (Claude)

A Rust terminal multiplexer scaffold implementing the v15 Pass 3 DEFINITIVE architecture spec.

## Building

```bash
cargo test --workspace
```

## Crate Map

| Crate | Level | Description |
|-------|-------|-------------|
| mux-types | L0 | Core types: Cell, PackedCell, Line, Style, Error |
| mux-parser | L0 | VT parser with CLASS_TABLE[256] + state machine |
| mux-grapheme-arena | L0 | Arena for extended grapheme clusters |
| mux-kernel | L0 | Sans-IO Event/Effect reducer + ServerGraph |
| mux-grid | L1 | Terminal grid with Arc COW scrollback |
| mux-snapshot | L1 | Binary snapshot format with CRC32C |
| mux-pty | L1 | PTY handle with typestate lifecycle |
| mux-proto | L1 | Wire protocol v8 TLV frames |
| mux-time | L1 | Deterministic time: Lamport + VectorClock |
| mux-termlet | L2 | Termlet testing runtime |
| mux-orm | L0 | ORM-like QueryList interface |
| mux-test-support | L0 | Test harness utilities |
