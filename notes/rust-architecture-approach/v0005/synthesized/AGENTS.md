# AGENTS.md -- TermForge v0005 Rule Catalog and Agent Guidelines

This document defines the rules, invariants, and agent guidelines for
developing the TermForge terminal multiplexer SDK. All rules have unique
IDs and concrete enforcement methods.

---

## Part 1: Invariants

### Core Invariants (INV-001 through INV-034)

| ID | Invariant | Enforcement |
|----|-----------|-------------|
| INV-001 | tmux wire-protocol v8 compatibility | Frame model + golden tests in mux-proto |
| INV-002 | All library crates use `mux-` prefix | Workspace Cargo.toml naming |
| INV-003 | No unsafe outside mux-pty and mux-ffi | `#![forbid(unsafe_code)]` in all other crates |
| INV-004 | All public types derive Debug | Derives verified by compile |
| INV-005 | Errors implement Error + Send + Sync + 'static | thiserror + trait bound tests |
| INV-006 | No panicking in library code | clippy deny(unwrap_used, expect_used, panic) |
| INV-007 | Grid coordinates are zero-based | mux-grid cursor/coordinate tests |
| INV-008 | Cell grapheme is valid UTF-8 | Cell API validation + fuzz |
| INV-009 | Style is bitfield-packed | Attrs bitflags + StylePack |
| INV-010 | Parser transitions are total | 14-state x 256-byte transition matrix tests |
| INV-011 | Canonical little-endian encoding | PackedCell/snapshot LE encoding |
| INV-012 | Grid mutation through dedicated APIs | put_cell / write_char only |
| INV-013 | PTY lifecycle monotonic | State machine transition checks |
| INV-014 | Socket path uses isolated naming | Test support unique path generation |
| INV-015 | tmux command/behavior parity | Differential test hooks |
| INV-016 | Layout determinism for equal input | Proptest determinism |
| INV-017 | OTEL cross-crate span boundaries | mux-otel span macros |
| INV-018 | Feature flags additive-only | Feature gate tests |
| INV-019 | Gate results Clone/Eq semantics | Derive checks |
| INV-020 | Frames are length-delimited | Frame encode/decode roundtrip |
| INV-021 | Clipboard size limits enforced | Size check tests |
| INV-022 | Key binding hierarchical scope | 4-scope resolution tests |
| INV-023 | Snapshot header size fixed | Compile-time constant |
| INV-024 | No SmallVec for line-cell storage | Dependency audit |
| INV-025 | VectorClock monotonic per actor | mux-crdt proptest |
| INV-026 | ByteClass DcsEntry at 0x90 | Parser byte-class tests |
| INV-027 | Parser/kernel stepping pure | No side-effect audit |
| INV-028 | GraphemeArena trailer optional | Snapshot codec model |
| INV-029 | Cell width 0, 1, or 2 | Cell validation tests |
| INV-030 | Extension index 0x0000 means none | Arena + packed-cell tests |
| INV-031 | CRC32C canonical for encode | Snapshot encode path |
| INV-032 | Scrollback uses Arc-backed line sharing | Arc<Vec<Cell>> type enforcement |
| INV-033 | Deterministic replay under fixed clock | Deterministic time + proptest |
| INV-034 | Canonical serialization | Stable codecs + round-trip tests |

### Extended Invariants (v0005)

| ID | Invariant | Enforcement |
|----|-----------|-------------|
| INV-117 | Panic recovery via catch_unwind at FFI boundaries | panic = "unwind" in all profiles |
| INV-119 | CellFlags bit values match tmux GRID_FLAG_* | Const assertion in mux-types |
| INV-220 | Graphics passthrough defaults to false | Config default tests |

---

## Part 2: Crate Rules

### R-TYPES: mux-types Rules

| Rule | Description | Enforcement |
|------|-------------|-------------|
| R-TYPES-01 | Colour enum has Default, Indexed(u8), Rgb{r,g,b} variants | Variant presence test |
| R-TYPES-02 | Attrs uses bitflags with BOLD, DIM, ITALIC, UNDERLINE, BLINK, REVERSE, HIDDEN, STRIKETHROUGH | Flag presence test |
| R-TYPES-03 | CellFlags uses bitflags with WIDE, PADDING, WRAPPED, SELECTED, SEARCHED | Flag presence + INV-119 |
| R-TYPES-04 | Cell has grapheme, width, fg, bg, attrs, flags, link fields | Field presence test |
| R-TYPES-05 | Size and Rect are Copy types | Trait bound test |
| R-TYPES-06 | Key enum covers Named, Char, Function variants | Variant coverage test |
| R-TYPES-07 | All ID types (SessionId, WindowId, PaneId) are Copy + Hash + Eq | Trait bound test |
| R-TYPES-08 | MuxError is the unified error enum | Variant coverage test |
| R-TYPES-09 | StyleDelta tracks which attributes changed | Field test |

### R-GRID: mux-grid Rules

| Rule | Description | Enforcement |
|------|-------------|-------------|
| R-GRID-01 | ChunkedGrid stores lines as Vec of Arc<Vec<Cell>> | Type structure |
| R-GRID-02 | write_char advances cursor and handles line wrap | Behavioral test |
| R-GRID-03 | scroll_up/scroll_down move lines within scroll region | Scroll region test |
| R-GRID-04 | resize preserves content where possible | Resize test |
| R-GRID-05 | clear_all resets cursor to (0,0) | State reset test |
| R-GRID-06 | set_cursor clamps to grid bounds | Bounds test |
| R-GRID-07 | snapshot returns Arc-cloned lines (COW) | Isolation test |
| R-GRID-08 | dirty tracking per mutation | Dirty flag test |
| R-GRID-09 | erase_in_line handles left and right modes | Mode test |
| R-GRID-10 | insert_lines/delete_lines shift within scroll region | Region test |

### R-PARSER: mux-parser Rules

| Rule | Description | Enforcement |
|------|-------------|-------------|
| R-PARSER-01 | 14 states as defined by Paul Williams spec | State enum count |
| R-PARSER-02 | CLASS_TABLE[256] maps all bytes | Table size assertion |
| R-PARSER-03 | ESC transitions to Escape from any state | Anywhere transition test |
| R-PARSER-04 | CSI entry via 0x9B or ESC [ | Dual entry test |
| R-PARSER-05 | OSC terminated by ST or BEL | Terminator test |
| R-PARSER-06 | DCS passthrough entered via 0x90 | INV-026 |
| R-PARSER-07 | Printable ASCII dispatched as Print action | Ground state test |
| R-PARSER-08 | C0 controls execute in all states | Anywhere execution test |
| R-PARSER-09 | UTF-8 lead bytes detected | UTF-8 class test |
| R-PARSER-10 | Unknown CSI finals logged, not panicked | Error handling test |

### R-KERNEL: mux-kernel Rules

| Rule | Description | Enforcement |
|------|-------------|-------------|
| R-KERNEL-01 | Kernel is sans-IO: processes KernelEvent, returns KernelEffect | Type signature |
| R-KERNEL-02 | Sessions, windows, panes stored in SlotMap | SlotMap type check |
| R-KERNEL-03 | LayoutNode is a binary tree with Leaf/HSplit/VSplit | Variant coverage |
| R-KERNEL-04 | distribute_evenly allocates pixel-perfect sizes | Proptest uniformity |
| R-KERNEL-05 | CopyModeState tracks cursor, selection, scroll offset | Field presence |
| R-KERNEL-06 | PasteBufferRing has configurable capacity with FIFO eviction | Overflow test |
| R-KERNEL-07 | Session has name, windows list, active_window | Structure test |
| R-KERNEL-08 | Window has name, panes list, layout | Structure test |
| R-KERNEL-09 | Pane has grid, cursor, process info | Structure test |
| R-KERNEL-10 | Kernel event dispatch is deterministic | Proptest determinism |

### R-RENDER: mux-render Rules

| Rule | Description | Enforcement |
|------|-------------|-------------|
| R-RENDER-01 | RenderBuffer is double-buffered (prev/next) | Structure test |
| R-RENDER-02 | diff_buffers returns only changed cells | Equality test |
| R-RENDER-03 | encode_diff produces valid escape sequences | Byte output test |
| R-RENDER-04 | SGR encoding covers bold, dim, italic, underline, blink, reverse | Attr coverage test |
| R-RENDER-05 | Colour encoding handles Default, Indexed, Rgb | Colour variant test |
| R-RENDER-06 | Cursor movement uses CUP escape | Position test |
| R-RENDER-07 | Composite buffer composites multiple panes | Composite test |
| R-RENDER-08 | Flood fairness via per-pane row quota | Stride rotation test |

### R-PROTO: mux-proto Rules

| Rule | Description | Enforcement |
|------|-------------|-------------|
| R-PROTO-01 | Frame header is 16 bytes: magic(4)+type(2)+flags(2)+len(4)+crc(4) | Size assertion |
| R-PROTO-02 | CRC32 covers type+flags+payload | CRC scope test |
| R-PROTO-03 | Encode/decode is roundtrip-safe | Proptest roundtrip |
| R-PROTO-04 | Corrupted CRC detected | Corruption test |
| R-PROTO-05 | Empty payload frames are valid | Edge case test |

### R-ORM: mux-orm Rules

| Rule | Description | Enforcement |
|------|-------------|-------------|
| R-ORM-01 | QuerySet supports filter(), first(), get(), count() | Method presence test |
| R-ORM-02 | filter() accepts closure predicates | Closure test |
| R-ORM-03 | get() returns error on 0 or 2+ results | Error variant test |
| R-ORM-04 | QuerySet is iterable | Iterator trait test |
| R-ORM-05 | Chained filters compose conjunctively | Composition test |

### R-API: mux-api Rules

| Rule | Description | Enforcement |
|------|-------------|-------------|
| R-API-01 | ServerBuilder validates size before build | Validation test |
| R-API-02 | create_session returns unique SessionId | Uniqueness test |
| R-API-03 | destroy_session is idempotent | Double-destroy test |
| R-API-04 | feed_pane routes data to pane's grid | Data flow test |
| R-API-05 | kernel() provides access to internal kernel | Accessor test |

### R-CONFIG: mux-config Rules

| Rule | Description | Enforcement |
|------|-------------|-------------|
| R-CONFIG-01 | load_tmux_conf parses set/set-option/bind commands | Parse test |
| R-CONFIG-02 | Comments (# lines) and empty lines skipped | Filter test |
| R-CONFIG-03 | -g flag sets server-scope options | Scope test |
| R-CONFIG-04 | Without -g, options are session-scoped | Scope test |
| R-CONFIG-05 | Boolean values: on/off/true/false | Parse test |
| R-CONFIG-06 | Integer values parsed | Parse test |
| R-CONFIG-07 | Option override: last writer wins | Override test |
| R-CONFIG-08 | Defaults present for escape-time, history-limit | Default test |

### R-TEST: mux-test-support Rules

| Rule | Description | Enforcement |
|------|-------------|-------------|
| R-TEST-01 | TestServer creates session+window+pane on construction | Creation test |
| R-TEST-02 | feed() routes bytes through VT parser to grid | Data flow test |
| R-TEST-03 | pane_text() extracts row text via arena | Extraction test |
| R-TEST-04 | cursor() returns current grid cursor position | Position test |
| R-TEST-05 | isolated_socket_path() returns unique paths | Uniqueness test (20 parallel) |
| R-TEST-06 | CleanupGuard removes file on drop | Cleanup test |
| R-TEST-07 | CleanupGuard does not panic on missing file | No-panic test |

### R-CRDT: mux-crdt Rules

| Rule | Description | Enforcement |
|------|-------------|-------------|
| R-CRDT-01 | VectorClock merge is commutative | Proptest commutativity |
| R-CRDT-02 | VectorClock merge is idempotent | Proptest idempotency |
| R-CRDT-03 | VectorClock merge is associative | Proptest associativity |
| R-CRDT-04 | LwwRegister last-write-wins by timestamp | Timestamp comparison test |
| R-CRDT-05 | LwwRegister merge is commutative | Proptest commutativity |
| R-CRDT-06 | increment increases version | Monotonicity test |
| R-CRDT-07 | happens_before is a partial order | Partial order test |

### R-TERMLET: mux-termlet Rules

| Rule | Description | Enforcement |
|------|-------------|-------------|
| R-TERMLET-01 | TermletBuilder validates config | Validation test |
| R-TERMLET-02 | Termlet has Created/Running/Stopped states | State machine test |
| R-TERMLET-03 | start() transitions Created to Running | Transition test |
| R-TERMLET-04 | feed() only works in Running state | State guard test |
| R-TERMLET-05 | resize() propagates to grid | Resize test |
| R-TERMLET-06 | stop() transitions Running to Stopped | Transition test |

---

## Part 3: Agent Guidelines

### Agent: Implementer

**Role**: Implement crate functionality following the architecture.

**Rules**:
1. Read CLAUDE.md before starting any implementation work.
2. Never add `unsafe` code outside mux-pty and mux-ffi.
3. Every public function must have a doc comment.
4. Every module must have `#[cfg(test)] mod tests` with at least 3 tests.
5. Use `thiserror` for error types. Do not use `anyhow` in library crates.
6. Prefer returning `Result` over panicking. Use `unwrap_or_default()` when a
   safe default exists.
7. When adding a dependency, check it exists in `[workspace.dependencies]`
   first. If not, discuss before adding.
8. Run `cargo clippy --all-targets` before submitting work.
9. Follow the layer dependency rules: L(N) crates may only depend on L(0..N-1).

### Agent: Tester

**Role**: Write and maintain tests across all crates.

**Rules**:
1. Unit tests go in `#[cfg(test)] mod tests` inside the source file.
2. Integration tests go in `crates/<name>/tests/*.rs`.
3. Property tests use `proptest` and go in a `mod prop` submodule or
   separate `tests/proptest_*.rs` file.
4. Every roundtrip operation needs a proptest (encode/decode, serialize/deserialize).
5. Every algebraic property needs a proptest (commutativity, idempotency,
   associativity, monotonicity).
6. Test names describe the behavior being verified, not the function name.
   Good: `scroll_up_moves_row1_to_row0`. Bad: `test_scroll_up`.
7. Socket paths in tests must use `isolated_socket_path()` from mux-test-support.
8. All test resources must be cleaned up. Use `CleanupGuard` for file cleanup.
9. Use `insta::assert_yaml_snapshot!` for complex output assertions.

### Agent: Reviewer

**Role**: Review code changes for adherence to invariants and conventions.

**Rules**:
1. Check that `#![forbid(unsafe_code)]` is present in all crates except mux-pty and mux-ffi.
2. Check that error types implement Error + Send + Sync via thiserror.
3. Check that no `unwrap()` or `expect()` appears in library code paths.
4. Check that `gen` is not used as an identifier (reserved in edition 2024).
5. Verify new public types derive Debug.
6. Verify test socket paths are isolated (not using default tmux paths).
7. Check layer dependency violations: no upward dependencies in the crate DAG.
8. Verify proptest coverage for any new encoding/decoding functions.

### Agent: Architect

**Role**: Make design decisions and update architecture documentation.

**Rules**:
1. All design decisions must reference at least one invariant or gap resolution.
2. New crates must be placed in the correct layer and added to Cargo.toml.
3. Cross-crate API changes must update both the providing and consuming crate.
4. Breaking changes must be documented in plan.md with migration instructions.
5. Performance-critical paths must have benchmark targets.

---

## Part 4: Settled Decisions

| ID | Decision | Rationale |
|----|----------|-----------|
| SD-01 | GraphemeArena for grapheme interning | Avoids per-cell String allocation |
| SD-02 | Arc<Vec<Cell>> for grid lines | Copy-on-write snapshot isolation |
| SD-03 | CRC32C for wire protocol checksums | Fast, hardware-accelerable |
| SD-04 | Paul Williams 14-state machine | Industry standard VT parsing |
| SD-05 | Sans-IO kernel pattern | Testability, determinism |
| SD-06 | Double-buffer diff rendering | Minimal escape sequence output |
| SD-07 | Binary tree layout engine | Supports recursive nesting |
| SD-08 | SlotMap for entity storage | O(1) lookup with generational safety |
| SD-09 | crossbeam-channel for kernel bridge | Bounded, high-performance MPSC |
| SD-10 | signal-hook for signal handling | Ecosystem standard, tokio-compatible |
| SD-11 | FdEnvelope with 16-fd cap | tmux SCM_RIGHTS compatibility |
| SD-12 | RawModeGuard RAII pattern | Automatic terminal state restore |
| SD-13 | Stride rotation for flood fairness | Prevents pane output starvation |
| SD-14 | VectorClock behind feature gate | Optional CRDT support |
| SD-15 | winnow for command parsing | Zero-copy parser combinators |

---

## Part 5: Quality Gates

### Minimum Thresholds

| Metric | Threshold |
|--------|-----------|
| Test count | 1,100+ |
| Proptest references | 130+ |
| Integration test files | 30+ |
| Clippy warnings | 0 |
| Unsafe code outside mux-pty/mux-ffi | 0 |
| Panics in library code | 0 |

### CI Pipeline

1. `cargo check --workspace`
2. `cargo clippy --workspace --all-targets`
3. `cargo test --workspace`
4. `cargo doc --workspace --no-deps`
5. `cargo deny check` (dependency audit)

---

## Part 6: Testing Matrix

### Test Types

| Type | Location | Framework |
|------|----------|-----------|
| Unit | `src/*.rs` `#[cfg(test)]` | std test |
| Property | `mod prop` or `tests/proptest_*.rs` | proptest |
| Integration | `tests/*.rs` | std test |
| Snapshot | `tests/*.rs` | insta |
| Fuzz | `fuzz/` | cargo-fuzz (future) |
| Benchmark | `benchmarks/` | criterion (future) |
| Differential | `tests/compat/` | tmux binary comparison (future) |

### Per-Crate Test Distribution

| Crate | Unit | Proptest | Integration |
|-------|------|----------|-------------|
| mux-types | 80+ | 20+ | 2 |
| mux-grid | 25+ | 8+ | 2 |
| mux-parser | 40+ | 10+ | 2 |
| mux-kernel | 55+ | 15+ | 3 |
| mux-render | 15+ | 8+ | 1 |
| mux-proto | 10+ | 6+ | 1 |
| mux-crdt | 10+ | 9+ | 1 |
| mux-orm | 10+ | 7+ | 2 |
| mux-api | 20+ | 5+ | 2 |
| mux-config | 20+ | 0 | 2 |
| mux-test-support | 20+ | 0 | 1 |
| mux-termlet | 10+ | 6+ | 1 |
| Others | 5-15 each | 3-6 each | 1 each |
