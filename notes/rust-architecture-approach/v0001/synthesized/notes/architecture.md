# TermForge Architecture Reference (v16 Definitive Scaffold)

## Settled Decisions

| ID | Decision |
|---|---|
| S91 | `compact_str::CompactString` for cell graphemes |
| S92 | GraphemeArena with 14-bit extension index |
| S93 | CRC32C is canonical encode checksum; FNV-1a decode-only |
| S94 | PackedCell layout: `scalar:21 + style:15 + flags:12 + width:2 + ext:14` |
| S95 | Parser byte dispatch via `CLASS_TABLE[256]` |
| S96 | Arc COW line/scrollback strategy |
| S97 | Deterministic time source with Lamport + feature-gated vector clock |
| S98 | Unified `TermletError` domain enum |
| S99 | Unified `InnerPtyState` lifecycle enum |
| S100 | Deterministic public API behavior under fixed seed/clock |

## Invariants (All 34)

| ID | Invariant | Scaffold Enforcement |
|---|---|---|
| INV-001 | tmux wire-protocol v8 compatibility | `mux-proto` frame model + tests |
| INV-002 | All library crates use `mux-` prefix | Workspace crate names |
| INV-003 | No unsafe outside `mux-pty` | `#![forbid(unsafe_code)]` in other crates |
| INV-004 | All public types derive Debug | Derives across crates |
| INV-005 | Errors are `Error + Send + Sync + 'static` | Trait impls + compile-time checks |
| INV-006 | No panicking in library code paths | API design + tests |
| INV-007 | Grid coordinates are zero-based | `mux-grid` cursor/coordinate logic |
| INV-008 | Cell grapheme is valid UTF-8 | `Cell` API and tests |
| INV-009 | Style is bitfield-packed | `Attrs` + `StylePack` |
| INV-010 | Parser transitions are total | 14-state transition tests |
| INV-011 | Canonical little-endian encoding | PackedCell/snapshot encoding |
| INV-012 | Grid mutation through dedicated APIs | `put_char` / `put_grapheme` |
| INV-013 | PTY lifecycle monotonic | typestate + transition checks |
| INV-014 | Socket path naming policy | test-support + termlet conventions |
| INV-015 | tmux command/behavior parity target | differential-test scaffold hooks |
| INV-016 | Layout determinism for equal input | deterministic APIs |
| INV-017 | OTEL cross-crate span boundaries | `otel_span!` / `otel_scope!` stubs |
| INV-018 | Feature flags additive-only | crate feature layout |
| INV-019 | Gate results Clone/Eq semantics | rule catalog + test policy |
| INV-020 | Frames are length-delimited | `Frame` encode/decode |
| INV-021 | Clipboard size limits enforced | rule catalog + TODO roadmap |
| INV-022 | Key binding hierarchical scope | rule catalog + roadmap |
| INV-023 | Snapshot header exactly 31 bytes | constants + tests |
| INV-024 | No `SmallVec` for line-cell storage | data structure choices |
| INV-025 | VectorClock monotonic per actor | `mux-time` tests under `crdt` |
| INV-026 | ByteClass has 7 variants, `DcsEntry` at `0x90` | parser byte-class tests |
| INV-027 | Parser/kernel stepping remains pure | transition/reducer design |
| INV-028 | GraphemeArena trailer optional in snapshots | snapshot codec model |
| INV-029 | Cell width explicit (`0/1/2`) | cell/packed-cell validation |
| INV-030 | ext index `0x0000` means no extension | arena + packed-cell tests |
| INV-031 | CRC32C canonical for encode | snapshot encode path |
| INV-032 | Scrollback uses Arc-backed line sharing | line + scrollback model |
| INV-033 | Deterministic replay under fixed clock/seed | deterministic time + tests |
| INV-034 | Canonical serialization | stable codecs + test policy |

## Crate Dependency Graph

```text
L0: mux-types, mux-parser, mux-grapheme-arena, mux-proto, mux-kernel
L1: mux-grid -> mux-types
    mux-snapshot -> mux-types
    mux-pty
    mux-time
L2: mux-termlet -> mux-types
    mux-orm
    mux-test-support
```

## Required Type Layouts

### PackedCell (64-bit)

```text
[63:43] scalar (21)
[42:28] style  (15)
[27:16] flags  (12)
[15:14] width  (2)
[13:0]  ext    (14)
```

### Snapshot Header (31 bytes)

```text
0..8    magic      ("TFSNAP13")
8..10   version    (u16 LE)
10      checksum algo
11..15  checksum   (u32 LE)
15..19  rows       (u32 LE)
19..23  cols       (u32 LE)
23..27  cells      (u32 LE)
27..31  arena size (u32 LE)
```

### Parser State Machine

- 14 states (`Ground`, `Escape`, `EscapeIntermediate`, `CsiEntry`, `CsiParam`, `CsiIntermediate`, `CsiIgnore`, `DcsEntry`, `DcsParam`, `DcsIntermediate`, `DcsPassthrough`, `DcsIgnore`, `OscString`, `SosPmApcString`)
- Byte classes: `Printable`, `Control`, `Escape`, `CsiEntry`, `DcsEntry`, `OscEntry`, `Utf8Lead`
- `classify_match()` remains the oracle for validating `CLASS_TABLE`
