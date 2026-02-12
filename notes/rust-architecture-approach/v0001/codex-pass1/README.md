# TermForge (codex-pass1)

A Rust-first terminal multiplexer scaffold focused on v15 invariants and crate
boundaries.

## Architecture

- **Edition**: Rust 2021, MSRV 1.75.0
- **License**: MIT OR Apache-2.0
- **Protocol**: tmux wire-protocol v8 compatible
- **Spec Version**: v15 Pass 3 DEFINITIVE

## Crate Map

| Crate | Layer | Purpose |
|-------|-------|---------|
| `mux-types` | L0 | Core types: Cell, PackedCell, Line, IdentityManifest, TermletError |
| `mux-parser` | L0 | VT parser with CLASS_TABLE[256] static LUT, ByteClass dispatch |
| `mux-grid` | L0 | Grid with VecDeque lines, Arc COW scrollback, viewport |
| `mux-snapshot` | L0 | Binary snapshot format: 31-byte header, CRC32C, PackedCell encode/decode |
| `mux-grapheme-arena` | L0 | GraphemeArena with 14-bit ext index, deduplication |
| `mux-proto` | L0 | Wire protocol v8 TLV frames, handshake, frame validation |
| `mux-time` | L1 | DeterministicTimeSource: wall + Lamport + vector clock |
| `mux-pty` | L1 | PtyHandle typestate, InnerPtyState, DynPtyHandle |
| `mux-termlet` | L2 | Termlet runtime, TermletConfig, TermletBuilder, resource quotas |
| `mux-orm` | L2 | QueryList, Queryable trait, filter/get semantics |
| `mux-test-support` | L2 | Test harness: isolated sockets, cleanup, differential testing |
| `mux-kernel` | L1 | Sans-IO reducer (`Event`/`Effect`), `ServerGraph`, `GenSlotMap`, OTEL span macros |

## Key Decisions (v15 DEFINITIVE)

- S91: `compact_str::CompactString` for Cell graphemes
- S94: PackedCell layout: scalar:21 + style:15 + flags:12 + width:2 + ext:14 = 64 bits
- S95: CLASS_TABLE[256] static LUT for ByteClass + oracle classifier
- S96: Arc<Vec<Cell>> COW for scrollback; Vec<Cell> for live grid
- S93: CRC32C canonical encode; FNV-1a decode-only backward compat
- S92: GraphemeArena with 14-bit ext index
- S97: DeterministicTimeSource (wall + Lamport + vector clock)
- S98: TermletError 16 variants with error_code()
- S99: Unified InnerPtyState enum
- S100: All public APIs deterministic under fixed seed
- `crdt` feature stubs on `mux-types` and `mux-kernel` (`#[cfg(feature = "crdt")]`)
- OTEL boundary macros: `otel_span!` and `otel_scope!` in `mux-kernel`

## Building

```sh
cargo build --workspace
cargo test --workspace
```

Current scaffold status:
- 12 crates
- 131 unit tests passing
- `cargo test --workspace` clean
