# TermForge Architecture Reference (v15 DEFINITIVE)

## Key Decisions Table

| ID | Decision | Status | Rationale |
|----|----------|--------|-----------|
| S91 | CompactString for Cell graphemes | SETTLED | 24-byte inline buffer, no heap for ASCII/BMP |
| S92 | GraphemeArena with 14-bit ext index | SETTLED | 16,383 extended graphemes per arena, dedup |
| S93 | CRC32C canonical encode; FNV-1a decode-only | SETTLED | Hardware CRC32C on x86; software fallback |
| S94 | PackedCell 64-bit layout: 21+15+12+2+14 | SETTLED | Cache-line efficient, covers full Unicode |
| S95 | CLASS_TABLE[256] static LUT for ByteClass | SETTLED | Branchless dispatch, 256-byte footprint |
| S96 | Arc COW scrollback; Vec for live grid | SETTLED | Zero-copy sharing for copy mode |
| S97 | DeterministicTimeSource (wall+Lamport+vector) | SETTLED | Testable, reproducible, causal ordering |
| S98 | TermletError 16 variants | SETTLED | Covers all termlet failure modes |
| S99 | Unified InnerPtyState 7-state enum | SETTLED | Single source of truth for PTY lifecycle |
| S100 | All public APIs deterministic under fixed seed | SETTLED | Replay, testing, debugging |

## 34 Invariants

| ID | Invariant | Enforcement |
|----|-----------|-------------|
| INV-001 | tmux wire-protocol v8 compatibility | Golden tests |
| INV-002 | All library crates use `mux-` prefix | Name lint |
| INV-003 | No unsafe outside `mux-pty` | forbid(unsafe_code) |
| INV-004 | All public types derive Debug | Clippy lint |
| INV-005 | All errors: Error + Send + Sync + 'static | Trait bound test |
| INV-006 | No panicking in library code | Clippy disallowed_methods |
| INV-007 | Grid coordinates zero-based | Index test |
| INV-008 | Cell grapheme is valid UTF-8 | Fuzz test |
| INV-009 | Style is bitfield-packed | Layout test |
| INV-010 | All parser transitions defined | Totality test |
| INV-011 | Platform-endian-independent LE encoding | Cross-platform test |
| INV-012 | Grid mutation only via put_char/put_grapheme | API audit |
| INV-013 | PTY lifecycle is monotonic | Transition test |
| INV-014 | Socket path uses termforge-<uid> prefix | Path test |
| INV-015 | tmux-compatible command parity | Differential test |
| INV-016 | Layout deterministic for identical input | Determinism test |
| INV-017 | OTEL spans at cross-crate boundaries | Span audit |
| INV-018 | Feature flags additive-only | Feature audit |
| INV-019 | Gate results are Clone + PartialEq + Eq | Trait test |
| INV-020 | Frames are length-delimited | Frame test |
| INV-021 | Clipboard size limit enforced | Size test |
| INV-022 | Key bindings hierarchically scoped | Scope test |
| INV-023 | Snapshot header is 31 bytes | Size test |
| INV-024 | SmallVec rejected FINAL for line cells | Dep check |
| INV-025 | VectorClock monotonic per node | Monotonic test |
| INV-026 | ByteClass has 7 variants, DcsEntry for 0x90 | Variant count test |
| INV-027 | Parser step() is pure | No side-effect audit |
| INV-028 | GraphemeArena is optional snapshot trailer | Format test |
| INV-029 | Cell width 0/1/2, explicit not inferred | Range test |
| INV-030 | GraphemeArena ext index 0x0000 = no extension | Reserved test |
| INV-031 | CRC32C canonical for encode | Algorithm test |
| INV-032 | Arc<Vec<Cell>> for scrollback | Type test |
| INV-033 | Deterministic replay under fixed clock/seed | Replay test |
| INV-034 | Canonical serialization (sorted keys) | Round-trip test |

## 100 Settled Decisions (S1-S100)

S1-S50: Core data structures, parser design, grid semantics.
S51-S75: Protocol, config, layout, model, key bindings.
S76-S90: Clipboard, mouse, status, copy mode, OTEL, errors.
S91-S100: Final concrete choices (CompactString, GraphemeArena, CRC32C,
PackedCell, CLASS_TABLE, Arc COW, DeterministicTimeSource, TermletError,
InnerPtyState, Deterministic APIs).

## Crate Dependency Graph

```
L0 (no inter-crate deps):
  mux-types
  mux-parser
  mux-grapheme-arena
  mux-proto

L1 (depends on L0):
  mux-grid -> mux-types, mux-grapheme-arena
  mux-snapshot -> mux-types, mux-grapheme-arena
  mux-time -> mux-types
  mux-pty -> mux-types
  mux-kernel -> mux-time, tracing

L2 (depends on L0+L1):
  mux-termlet -> mux-types, mux-grid, mux-pty, mux-time
  mux-orm -> mux-types
  mux-test-support -> mux-types
```

## Pass 1 Structural Notes

- `mux-grid` uses `grid.rs` + `scrollback.rs` modules with `pub use` from `lib.rs`.
- `mux-parser` keeps the full parser state machine in `lib.rs`, with byte classification extracted to `byte_class.rs`.
- `mux-snapshot` keeps header/checksum logic in `lib.rs`, with packed-cell binary helpers in `packed_cell.rs`.
- `mux-kernel` provides Sans-IO reduction (`Event` -> `Effect`) and includes `ServerGraph` backed by a generational slot map.
- `#[cfg(feature = "crdt")]` stubs are present in `mux-types` and `mux-kernel`.

## PackedCell Bit Layout (64 bits total)

```
Bits [63:43] = scalar (21 bits) - Unicode scalar value (up to U+1FFFFF)
Bits [42:28] = style  (15 bits) - Packed style attributes
Bits [27:16] = flags  (12 bits) - Cell flags
Bits [15:14] = width  (2 bits)  - Display width: 0, 1, or 2
Bits [13:0]  = ext    (14 bits) - GraphemeArena extension index
```

## ByteClass Variants (7 total)

```
Printable   - 0x20..=0x7E
Control     - 0x00..=0x1F (except ESC)
Escape      - 0x1B
CsiEntry    - 0x9B
DcsEntry    - 0x90
OscEntry    - 0x9D
Utf8Lead    - 0x80..=0xFE (except C1 codes above)
```

## Snapshot Header (31 bytes)

```
Offset  Size  Field
0       8     Magic ("TFSNAP13")
8       2     Format version (u16 LE)
10      1     Checksum algorithm (0=CRC32C, 1=FNV-1a)
11      4     Checksum (u32 LE)
15      4     Row count (u32 LE)
19      4     Col count (u32 LE)
23      4     Cell count (u32 LE)
27      4     Arena size (u32 LE)
```

## Error Code Ranges

| Range | Domain |
|-------|--------|
| 1000-1999 | Grid errors |
| 2000-2999 | Parser errors |
| 3000-3999 | Snapshot errors |
| 4000-4999 | Protocol errors |
| 5000-5999 | PTY errors |
| 6000-6999 | Config errors |
| 7000-7999 | Clipboard errors |
| 8000-8999 | Termlet errors |

## Wire Protocol v8 Frame Structure

```
[1 byte]  Frame type (FrameType enum)
[4 bytes] Payload length (u32 LE)
[N bytes] Payload
```

Frame types: Handshake(0x01), Command(0x02), Response(0x03), Event(0x04),
Error(0x05), Ping(0x06), Pong(0x07), Resize(0x08), Data(0x09), Close(0x0A).
