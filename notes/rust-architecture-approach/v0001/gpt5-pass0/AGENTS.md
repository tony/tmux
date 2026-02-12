# TermForge AGENTS Rulebook (v16 scaffold)

Consolidated **405** normative rules from v15 definitive spec, grouped by section and mapped to crates/domains.

- Source: `notes/2026-02-10-rust-architectural-approach-synthesized-v15.md`
- Rule count: `405`
- Enforcement metadata is preserved from definitive bullet entries or expanded from definitive ranges.

## Crate Mapping

| Section | Primary crate/domain |
|---|---|
| S01 | mux-types |
| S02 | workspace/CI |
| S03 | workspace |
| S04 | workspace |
| S05 | mux-types |
| S06 | mux-parser |
| S07 | mux-pty |
| S08 | mux-snapshot |
| S09 | mux-proto |
| S10 | mux-grid |
| S11 | mux-proto |
| S12 | mux-orm |
| S13 | mux-grid |
| S14 | mux-orm |
| S15 | mux-termlet |
| S16 | mux-test-support |
| S17 | mux-proto |
| S18 | mux-parser |
| S19 | mux-time |
| S20 | mux-termlet |
| S21 | mux-pty |
| S22 | mux-proto |
| S23 | mux-orm |
| S24 | mux-grid |
| S25 | mux-grid |
| S26 | AGENTS governance |
| S27 | risk governance |
| S28 | release governance |
| S29 | testing governance |
| S30 | evolution governance |
| S31 | infrastructure |
| S32 | mux-termlet + integration |

## S01 — Project Identity (TermForge) (mux-types)

- `RULE-S01-01`: Project name is TermForge. **Enforcement:** Const check.
- `RULE-S01-02`: Binary name is termforge. **Enforcement:** Cargo.toml lint.
- `RULE-S01-03`: Library prefix is mux-. **Enforcement:** Crate name lint.
- `RULE-S01-04`: Rust edition is 2021. **Enforcement:** Cargo.toml check.
- `RULE-S01-05`: MSRV is 1.75.0. **Enforcement:** CI MSRV gate.
- `RULE-S01-06`: License is MIT OR Apache-2.0. **Enforcement:** License check.
- `RULE-S01-07`: IdentityManifest is canonical metadata struct. **Enforcement:** Import lint.
- `RULE-S01-08`: BuildProfile has 3 variants. **Enforcement:** Variant count test.
- `RULE-S01-09`: git_sha populated in release builds. **Enforcement:** CI build check.
- `RULE-S01-10`: IdentityManifest derives Clone + PartialEq + Eq. **Enforcement:** Trait impl test.
- `RULE-S01-11`: **[v15]** BuildProfile::Profiling variant present. **Enforcement:** Variant test.
- `RULE-S01-12`: **[v15 P2]** target_triple MUST be non-empty. **Enforcement:** Field presence test.
- `RULE-S01-13`: **[v15 P2]** feature_flags MUST record active Cargo features. **Enforcement:** Build injection test.
- `RULE-S01-14`: **[v15 P3]** version_string() MUST contain spec version tag. **Enforcement:** String match test. [v15 P3]
- `RULE-S01-15`: **[v15 P3]** Identity serialization MUST be canonical (sorted keys). **Enforcement:** Round-trip test (INV-034). [v15 P3]

## S02 — Quality Gates and CI Matrix (workspace/CI)

- `RULE-S02-01`: Three release lanes: LTS, Current, Preview. **Enforcement:** Lane enum test.
- `RULE-S02-02`: Preview failures are warnings. **Enforcement:** Gate logic test.
- `RULE-S02-03`: LTS/Current failures block release. **Enforcement:** Gate logic test.
- `RULE-S02-04`: Waivers have expiry timestamps. **Enforcement:** Expiry field test.
- `RULE-S02-05`: Expired waivers block release. **Enforcement:** Expiry logic test.
- `RULE-S02-06`: CI matrix has 16 jobs. **Enforcement:** Matrix count test.
- `RULE-S02-07`: GateOutcome has 5 variants. **Enforcement:** Variant count test.
- `RULE-S02-08`: Gate results are Clone + PartialEq + Eq. **Enforcement:** Trait impl test.
- `RULE-S02-09`: **[v15 P2]** FailWithBypass for security hotfixes. **Enforcement:** Bypass test.
- `RULE-S02-10`: **[v15 P2]** 4 feature sets: default, crdt, crc32c, wasm. **Enforcement:** Feature test.
- `RULE-S02-11`: **[v15 P3]** Gate evaluation MUST be deterministic (INV-033). **Enforcement:** Repeated run test. [v15 P3]
- `RULE-S02-12`: **[v15 P3]** Gate audit trail MUST be JSON-serializable (INV-034). **Enforcement:** Serde test. [v15 P3]

## S03 — Crate DAG and Dependency Policy (workspace)

- `RULE-S03-01`: Crate dependency graph is acyclic. **Enforcement:** `cargo deny` CI.
- `RULE-S03-02`: L0 crates have no external deps. **Enforcement:** Dep count check.
- `RULE-S03-03`: All library crates use `mux-` prefix. **Enforcement:** Name lint.
- `RULE-S03-04`: WASM target for L0. **Enforcement:** `cargo build --target wasm32`.
- `RULE-S03-05`: Feature flags are additive-only. **Enforcement:** Feature audit.
- `RULE-S03-06`: **[v15 P2]** 4 feature sets: default, crdt, crc32c, wasm. **Enforcement:** CI matrix.
- `RULE-S03-07`: **[v15 P3]** crdt and wasm MUST be mutually exclusive. **Enforcement:** Feature conflict test. [v15 P3]
- `RULE-S03-08`: **[v15 P3]** Supply chain security via cargo-vet. **Enforcement:** CI audit gate. [v15 P3]
- `RULE-S03-09`: **[v15 P3]** snapshot-compress feature is non-normative. **Enforcement:** Feature doc.
- `RULE-S03-10`: **[v15 P3]** Dependency additions require RFC review. **Enforcement:** PR policy. [v15 P3]

## S04 — Workspace Layout (workspace)

- `RULE-S04-01`: Workspace uses `crates/` directory. **Enforcement:** Path check.
- `RULE-S04-02`: Benchmarks in `benchmarks/`. **Enforcement:** Path check.
- `RULE-S04-03`: Fuzz targets in `fuzz/`. **Enforcement:** Path check.
- `RULE-S04-04`: Golden fixtures in `fixtures/`. **Enforcement:** Path check.
- `RULE-S04-05`: Integration tests in `tests/`. **Enforcement:** Path check.
- `RULE-S04-06`: **[v15 P2]** Examples in `examples/`. **Enforcement:** Path check.
- `RULE-S04-07`: **[v15 P3]** CI MUST validate directory structure. **Enforcement:** CI step. [v15 P3]
- `RULE-S04-08`: **[v15 P3]** Each crate MUST have README.md. **Enforcement:** File presence check. [v15 P3]
- `RULE-S04-09`: **[v15 P3]** Workspace layout changes require RFC. **Enforcement:** PR policy. [v15 P3]
- `RULE-S04-10`: **[v15 P3]** All crate Cargo.toml MUST reference workspace version. **Enforcement:** Cargo lint. [v15 P3]

## S05 — Grid API (Cell, Line, Viewport) (mux-types)

- `RULE-S05-01`: Grid coordinates are zero-based. **Enforcement:** Index test.
- `RULE-S05-02`: Mutation only via put_char/put_grapheme. **Enforcement:** API audit (INV-012).
- `RULE-S05-03`: SmallVec rejected FINAL for line cells. **Enforcement:** Dep check (INV-024).
- `RULE-S05-04`: Cell grapheme is valid UTF-8. **Enforcement:** Fuzz test (INV-008).
- `RULE-S05-05`: Style is bitfield-packed. **Enforcement:** Layout test (INV-009).
- `RULE-S05-06`: **[v15 P2]** Cell width field MUST be 0, 1, or 2. **Enforcement:** Range test (INV-029).
- `RULE-S05-07`: **[v15 P2]** CompactString for Cell grapheme. **Enforcement:** Type test (S91).
- `RULE-S05-08`: **[v15 P2]** Arc<Vec<Cell>> for scrollback. **Enforcement:** Type test (S96, INV-032).
- `RULE-S05-09`: **[v15 P2]** VecDeque for grid lines. **Enforcement:** Collection type test.
- `RULE-S05-10`: **[v15 P3]** Unicode scalar validated at insertion, not encoding. **Enforcement:** Boundary test. [v15 P3]
- `RULE-S05-11`: **[v15 P3]** Scrollback limit is configurable. **Enforcement:** Config test. [v15 P3]
- `RULE-S05-12`: **[v15 P3]** Scroll region bounds MUST be validated. **Enforcement:** Bounds test. [v15 P3]

## S06 — VtParser State Machine and Action Dispatch (mux-parser)

- `RULE-S06-01`: CLASS_TABLE[256] is canonical hot path. **Enforcement:** LUT presence test (S95).
- `RULE-S06-02`: classify_match() retained as oracle. **Enforcement:** Oracle test.
- `RULE-S06-03`: ByteClass has 7 variants. **Enforcement:** Variant count test (INV-026).
- `RULE-S06-04`: ByteClass is #[repr(u8)]. **Enforcement:** Attribute check.
- `RULE-S06-05`: DcsEntry for 0x90. **Enforcement:** Byte test.
- `RULE-S06-06`: Step function is pure. **Enforcement:** No side-effect audit (INV-027).
- `RULE-S06-07`: All transitions defined. **Enforcement:** Totality test (INV-010).
- `RULE-S06-08`: **[v15 P2]** CLASS_TABLE matches oracle for all 256 bytes. **Enforcement:** Differential test.
- `RULE-S06-09`: **[v15 P3]** Actions are effect-free records. **Enforcement:** Type audit. [v15 P3]
- `RULE-S06-10`: **[v15 P3]** Parser fuzz target present. **Enforcement:** Fuzz target check. [v15 P3]
- `RULE-S06-11`: **[v15 P3]** Unsupported CSI finals logged as metrics, not panics. **Enforcement:** Panic-free test. [v15 P3]
- `RULE-S06-12`: **[v15 P3]** Parser state machine is serializable for replay. **Enforcement:** Serde test. [v15 P3]

## S07 — PtyHandle Lifecycle and Resource Cleanup (mux-pty)

- `RULE-S07-01`: PtyHandle has 7 states. **Enforcement:** State count test.
- `RULE-S07-02`: Lifecycle is monotonic (INV-013). **Enforcement:** Transition test.
- `RULE-S07-03`: Typestate PtyHandle for Rust core. **Enforcement:** API audit.
- `RULE-S07-04`: DynPtyHandle for FFI boundary. **Enforcement:** FFI test.
- `RULE-S07-05`: TryFrom<DynPtyHandle> for typed recovery. **Enforcement:** Conversion test (S82).
- `RULE-S07-06`: **[v15 P2]** Generation counting prevents ABA. **Enforcement:** Generation test.
- `RULE-S07-07`: **[v15 P2]** validate_generation before slot ops. **Enforcement:** Pre-check test.
- `RULE-S07-08`: **[v15 P3]** Unified InnerPtyState enum (S99). **Enforcement:** Single enum test. [v15 P3]
- `RULE-S07-09`: **[v15 P3]** Restart barrier enforced. **Enforcement:** Order test. [v15 P3]
- `RULE-S07-10`: **[v15 P3]** Closed handle returns HandleClosed deterministically. **Enforcement:** Error test. [v15 P3]
- `RULE-S07-11`: **[v15 P3]** PtyHandleError implements std::error::Error. **Enforcement:** Trait test (INV-005). [v15 P3]
- `RULE-S07-12`: **[v15 P3]** Drop on PtyHandle forces kill if not terminal. **Enforcement:** Drop test. [v15 P3]

## S08 — Snapshot Format and Versioning (mux-snapshot)

- `RULE-S08-01`: Snapshot magic is "TFSNAP13". **Enforcement:** Magic check.
- `RULE-S08-02`: Header is 31 bytes (INV-023). **Enforcement:** Size test.
- `RULE-S08-03`: CRC32C canonical for encode (S93, INV-031). **Enforcement:** Algorithm test.
- `RULE-S08-04`: FNV-1a decode-only. **Enforcement:** Decode test.
- `RULE-S08-05`: PackedCell is 64 bits (S94). **Enforcement:** Size test.
- `RULE-S08-06`: PackedCell layout: scalar:21+style:15+flags:12+width:2+ext:14. **Enforcement:** Bit test.
- `RULE-S08-07`: GraphemeArena is optional trailer (INV-028). **Enforcement:** Format test.
- `RULE-S08-08`: Platform-endian-independent (LE canonical, INV-011). **Enforcement:** Cross-platform test.
- `RULE-S08-09`: **[v15 P2]** Unknown algorithms are hard errors. **Enforcement:** Rejection test.
- `RULE-S08-10`: **[v15 P3]** Software CRC32C fallback mandatory. **Enforcement:** Fallback test. [v15 P3]
- `RULE-S08-11`: **[v15 P3]** Truncated payloads rejected before UTF-8. **Enforcement:** Order test. [v15 P3]
- `RULE-S08-12`: **[v15 P3]** Snapshot round-trip MUST preserve all fields. **Enforcement:** Property test. [v15 P3]

## S09 — Wire Protocol Compatibility (tmux) (mux-proto)

- `RULE-S09-01`: 100% tmux wire-protocol v8 compat (INV-001). **Enforcement:** Golden tests.
- `RULE-S09-02`: Frames are length-delimited (INV-020). **Enforcement:** Frame test.
- `RULE-S09-03`: Socket path uses termforge-<uid> prefix (INV-014). **Enforcement:** Path test.
- `RULE-S09-04`: **[v15 P2]** SO_PASSCRED on Linux. **Enforcement:** Platform test.
- `RULE-S09-05`: **[v15 P3]** Frame size limit MUST be enforced. **Enforcement:** Size test. [v15 P3]
- `RULE-S09-06`: **[v15 P3]** Replay MUST be idempotent by message ID. **Enforcement:** Replay test. [v15 P3]
- `RULE-S09-07`: **[v15 P3]** FrameTooLarge error MUST include size and limit. **Enforcement:** Field test. [v15 P3]
- `RULE-S09-08`: **[v15 P3]** Protocol version mismatch MUST be a hard error. **Enforcement:** Version test. [v15 P3]
- `RULE-S09-09`: **[v15 P3]** Differential parity against tmux mandatory for wire-visible paths. **Enforcement:** Parity test. [v15 P3]
- `RULE-S09-10`: **[v15 P3]** DCS passthrough piped correctly (INV-026). **Enforcement:** DCS test. [v15 P3]

## S10 — Configuration and Options (mux-grid)

- `RULE-S10-01`: tmux-compatible option names for all `set-option` keys (INV-015). **Enforcement:** Name parity audit against tmux source.
- `RULE-S10-02`: Hierarchical scoping with 4 levels (server/session/window/pane). **Enforcement:** Scope resolution test per level.
- `RULE-S10-03`: TOML configuration format (no YAML, no JSON). **Enforcement:** Parse test with valid TOML.
- `RULE-S10-04`: XDG base directory support per XDG Base Directory Specification. **Enforcement:** Path test with `$XDG_CONFIG_HOME` set and unset.
- `RULE-S10-05`: **[v15 P3]** Hot-reload MUST NOT require server restart. File-system watcher applies diffs atomically. **Enforcement:** Integration test: modify config file, verify server picks up change within 1 second. [v15 P3]
- `RULE-S10-06`: **[v15 P3]** Config serialization is canonical (sorted keys, INV-034). Two identical configs produce byte-identical TOML. **Enforcement:** Round-trip test: serialize -> deserialize -> serialize, compare bytes. [v15 P3]
- `RULE-S10-07`: **[v15 P3]** Invalid keys produce ConfigError::UnknownKey with the key name. **Enforcement:** Error variant test. [v15 P3]
- `RULE-S10-08`: **[v15 P3]** Config changes logged to OTEL with old and new values. **Enforcement:** OTEL span collector test. [v15 P3]
- `RULE-S10-09`: **[v15 P3]** Type-safe values: ConfigError::TypeMismatch for wrong types. **Enforcement:** Type mismatch test. [v15 P3]
- `RULE-S10-10`: **[v15 P3]** Config diff is deterministic. **Enforcement:** Diff symmetry test (diff(a,b) == diff(b,a) in terms of changed keys). [v15 P3]
- `RULE-S10-11`: **[v15 P3]** Failed hot-reload rolls back, logs warning. **Enforcement:** Rollback test with malformed TOML. [v15 P3]
- `RULE-S10-12`: **[v15 P3]** Config values support bool, integer, string, enum, color types. **Enforcement:** Type coverage test. [v15 P3]

## S11 — Layout Engine (mux-proto)

- `RULE-S11-01`: Deterministic layout for identical input (INV-016). **Enforcement:** Determinism test.
- `RULE-S11-02`: Minimum pane size 1x1. **Enforcement:** Size rejection test.
- `RULE-S11-03`: tmux-compatible split semantics. **Enforcement:** Parity test against tmux.
- `RULE-S11-04`: Split directions: horizontal, vertical. **Enforcement:** Direction exhaustiveness test.
- `RULE-S11-05`: **[v15 P3]** Layout engine is pure (no IO, no global state). **Enforcement:** No-IO audit. [v15 P3]
- `RULE-S11-06`: **[v15 P3]** Layout changes logged to OTEL with pane geometry. **Enforcement:** Span test. [v15 P3]
- `RULE-S11-07`: **[v15 P3]** Resize preserves split ratios. **Enforcement:** Ratio preservation test. [v15 P3]
- `RULE-S11-08`: **[v15 P3]** Layout state serializable for snapshots. **Enforcement:** Serde round-trip test. [v15 P3]
- `RULE-S11-09`: **[v15 P3]** Layout tree supports recursive nesting. **Enforcement:** Depth > 2 test. [v15 P3]
- `RULE-S11-10`: **[v15 P3]** find_pane() traverses all children. **Enforcement:** Deep find test. [v15 P3]

## S12 — Session, Window, Pane Model (mux-orm)

- `RULE-S12-01`: Server-Session-Window-Pane hierarchy. **Enforcement:** Structure test.
- `RULE-S12-02`: QueryList with typed errors. **Enforcement:** Error type test.
- `RULE-S12-03`: ObjectDoesNotExist and MultipleObjectsReturned variants. **Enforcement:** Variant test.
- `RULE-S12-04`: Pane owns Grid and PtyHandle. **Enforcement:** Ownership test.
- `RULE-S12-05`: **[v15 P3]** IDs are monotonic. **Enforcement:** Monotonic test. [v15 P3]
- `RULE-S12-06`: **[v15 P3]** Key bindings hierarchically scoped (INV-022). **Enforcement:** Scope test. [v15 P3]
- `RULE-S12-07`: **[v15 P3]** Session/Window serializable. **Enforcement:** Serde test. [v15 P3]
- `RULE-S12-08`: **[v15 P3]** Pane destruction releases PtyHandle. **Enforcement:** Drop test. [v15 P3]
- `RULE-S12-09`: **[v15 P3]** QueryList supports filter() for multi-result queries. **Enforcement:** Filter test. [v15 P3]
- `RULE-S12-10`: **[v15 P3]** QueryError is Send + Sync. **Enforcement:** Trait bound test. [v15 P3]

## S13 — Key Bindings (mux-grid)

- `RULE-S13-01`: Hierarchical scoping: Global < Session < Window < Pane (INV-022). **Enforcement:** 4-scope resolution test.
- `RULE-S13-02`: tmux-compatible key notation (C-, M-, S-, F1-F12). **Enforcement:** Parse test against tmux key table.
- `RULE-S13-03`: Description field MUST be non-empty for all built-in bindings. **Enforcement:** Description presence lint.
- `RULE-S13-04`: Most-specific scope wins. **Enforcement:** Multi-scope resolution test.
- `RULE-S13-05`: **[v15 P3]** Last-writer-wins within same scope is deterministic. **Enforcement:** LWW test with insertion order. [v15 P3]
- `RULE-S13-06`: **[v15 P3]** Key binding changes emit OTEL span with old and new command. **Enforcement:** Span collector test. [v15 P3]
- `RULE-S13-07`: **[v15 P3]** Key table names match tmux (`prefix`, `root`, `copy-mode-vi`, `copy-mode`). **Enforcement:** Table name parity test. [v15 P3]
- `RULE-S13-08`: **[v15 P3]** Invalid modifiers produce KeyParseError::InvalidModifier. **Enforcement:** Error variant test. [v15 P3]
- `RULE-S13-09`: **[v15 P3]** Repeat flag (-r) support within repeat-time. **Enforcement:** Repeat flag integration test. [v15 P3]
- `RULE-S13-10`: **[v15 P3]** Custom key tables do not pollute default tables. **Enforcement:** Table isolation test. [v15 P3]

## S14 — Clipboard (mux-orm)

- `RULE-S14-01`: Clipboard size limit enforced (INV-021). **Enforcement:** Size test exceeding limit produces SizeLimitExceeded.
- `RULE-S14-02`: OSC 52 set and get operations. **Enforcement:** Protocol integration test.
- `RULE-S14-03`: **[v15 P3]** Clipboard content sanitized: CSI, OSC, DCS sequences stripped before storage. **Enforcement:** Sanitize fuzz test with known escape sequences. [v15 P3]
- `RULE-S14-04`: **[v15 P3]** Clipboard changes emit OTEL span with buffer name and size. **Enforcement:** Span collector test. [v15 P3]
- `RULE-S14-05`: **[v15 P3]** Named buffers use BTreeMap for deterministic order (INV-034). **Enforcement:** Iteration order test. [v15 P3]
- `RULE-S14-06`: **[v15 P3]** History ring respects configurable size, evicts FIFO. **Enforcement:** Ring overflow test. [v15 P3]
- `RULE-S14-07`: **[v15 P3]** BufferNotFound error for missing buffer names. **Enforcement:** Error variant test. [v15 P3]
- `RULE-S14-08`: **[v15 P3]** OSC 52 query returns base64-encoded current buffer. **Enforcement:** Base64 round-trip test. [v15 P3]

## S15 — Mouse Support (mux-termlet)

- `RULE-S15-01`: SGR extended mouse protocol (CSI < Pb;Px;Py M/m). **Enforcement:** Protocol encode/decode test.
- `RULE-S15-02`: Cell and pixel modes both supported. **Enforcement:** Mode switching test.
- `RULE-S15-03`: **[v15 P2]** pixel_mode field with px_x/px_y coordinates. **Enforcement:** Pixel field test.
- `RULE-S15-04`: **[v15 P3]** Mouse events deterministic in replay (INV-033). **Enforcement:** Record/replay test. [v15 P3]
- `RULE-S15-05`: **[v15 P3]** Button code decode handles all SGR button values. **Enforcement:** Exhaustive decode test. [v15 P3]
- `RULE-S15-06`: **[v15 P3]** Modifier bit extraction (shift=4, meta=8, ctrl=16, motion=32). **Enforcement:** Bitmask test. [v15 P3]
- `RULE-S15-07`: **[v15 P3]** Drag events have motion flag set. **Enforcement:** Drag detection test. [v15 P3]
- `RULE-S15-08`: **[v15 P3]** Mouse events emit OTEL spans in debug mode. **Enforcement:** Span test. [v15 P3]

## S16 — Status Bar (mux-test-support)

- `RULE-S16-01`: tmux-compatible format strings (#S, #W, #I, #P, #T, #H, #F, #{...}). **Enforcement:** Format parity test against tmux.
- `RULE-S16-02`: pane_count and TermForge-specific variables. **Enforcement:** Variable presence test.
- `RULE-S16-03`: **[v15 P3]** Status bar rendering is pure function of (format, context). **Enforcement:** Determinism test with fixed context. [v15 P3]
- `RULE-S16-04`: **[v15 P3]** Custom variables MUST be documented in help text. **Enforcement:** Doc coverage lint. [v15 P3]
- `RULE-S16-05`: **[v15 P3]** Missing variables produce empty string, not error. **Enforcement:** Missing var test. [v15 P3]
- `RULE-S16-06`: **[v15 P3]** Status truncation preserves left content with "..." suffix. **Enforcement:** Truncation test. [v15 P3]
- `RULE-S16-07`: **[v15 P3]** Style directives (#[...]) parsed and applied. **Enforcement:** Style parse test. [v15 P3]
- `RULE-S16-08`: **[v15 P3]** Rate-limited updates at configurable interval. **Enforcement:** Timer test. [v15 P3]

## S17 — Copy Mode (mux-proto)

- `RULE-S17-01`: Vi and Emacs copy mode styles with full tmux keybinding parity. **Enforcement:** Key parity test.
- `RULE-S17-02`: Search within scrollback (forward and backward). **Enforcement:** Search integration test.
- `RULE-S17-03`: **[v15 P2]** Copy mode operates on COW scrollback snapshot. **Enforcement:** Arc clone count test.
- `RULE-S17-04`: **[v15 P3]** Search results deterministic (INV-033). **Enforcement:** Fixed-content search determinism test. [v15 P3]
- `RULE-S17-05`: **[v15 P3]** Three selection modes: character, line, block. **Enforcement:** Selection mode exhaustiveness test. [v15 P3]
- `RULE-S17-06`: **[v15 P3]** Yanked text goes through clipboard sanitization. **Enforcement:** Clipboard integration test. [v15 P3]
- `RULE-S17-07`: **[v15 P3]** Search match wraps around at end of buffer. **Enforcement:** Wrap test. [v15 P3]
- `RULE-S17-08`: **[v15 P3]** Copy mode entry/exit emits OTEL span. **Enforcement:** Span test. [v15 P3]

## S18 — OTEL Observability (mux-parser)

- `RULE-S18-01`: OTEL spans at cross-crate boundaries (INV-017). **Enforcement:** Span audit.
- `RULE-S18-02`: Metrics for key operations. **Enforcement:** Metric test.
- `RULE-S18-03`: **[v15 P2]** grapheme_arena.size metric present. **Enforcement:** Metric name test.
- `RULE-S18-04`: **[v15 P2]** cow_trigger_count metric present. **Enforcement:** Metric name test.
- `RULE-S18-05`: **[v15 P3]** OTEL opt-in via feature flag. **Enforcement:** Feature test. [v15 P3]
- `RULE-S18-06`: **[v15 P3]** Disabled path has zero overhead. **Enforcement:** Benchmark (no-op vs enabled). [v15 P3]
- `RULE-S18-07`: **[v15 P3]** Semantic naming convention: `<namespace>.<metric_name>`. **Enforcement:** Name regex lint. [v15 P3]
- `RULE-S18-08`: **[v15 P3]** Dimension labels on all metrics (session_id, pane_id, crate_name). **Enforcement:** Label presence test. [v15 P3]
- `RULE-S18-09`: **[v15 P3]** Trace context propagation across async boundaries. **Enforcement:** Cross-boundary span test. [v15 P3]
- `RULE-S18-10`: **[v15 P3]** OTEL exporter configurable (Jaeger, OTLP, stdout). **Enforcement:** Exporter config test. [v15 P3]

## S19 — Error Handling Strategy (mux-time)

- `RULE-S19-01`: All errors implement `std::error::Error + Send + Sync + 'static` (INV-005). **Enforcement:** Trait bound compile test.
- `RULE-S19-02`: No panicking in library code (INV-006). No `unwrap()`, `expect()`, `panic!()` outside tests. **Enforcement:** clippy `disallowed_methods` lint.
- `RULE-S19-03`: Structured error types with per-variant diagnostic context. **Enforcement:** Context coverage test.
- `RULE-S19-04`: **[v15 P3]** Error chains via `source()` for nested errors. **Enforcement:** Source chain traversal test. [v15 P3]
- `RULE-S19-05`: **[v15 P3]** All errors derive Debug + Clone. **Enforcement:** Derive test. [v15 P3]
- `RULE-S19-06`: **[v15 P3]** Unique u16 error codes per variant via `error_code()`. **Enforcement:** Code uniqueness test. [v15 P3]
- `RULE-S19-07`: **[v15 P3]** Error codes are stable: existing codes MUST NOT change. **Enforcement:** Golden file test. [v15 P3]
- `RULE-S19-08`: **[v15 P3]** ErrorContext feeds into OTEL error spans. **Enforcement:** Span attribute test. [v15 P3]
- `RULE-S19-09`: **[v15 P3]** Error Display includes [Ecode] prefix for log parsing. **Enforcement:** Display format test. [v15 P3]
- `RULE-S19-10`: **[v15 P3]** Parser errors include byte_offset for diagnostic precision. **Enforcement:** Offset test. [v15 P3]

## S20 — Compatibility Matrix (mux-termlet)

- `RULE-S20-01`: Per-lane compatibility tracking for LTS, Current, Preview. **Enforcement:** Matrix non-empty assertion.
- `RULE-S20-02`: Parity percentage tracked and reported in CI. **Enforcement:** Percentage calculation test.
- `RULE-S20-03`: **[v15 P3]** Compatibility evidence machine-verifiable: every compat entry has a test_id. **Enforcement:** test_id presence lint. [v15 P3]
- `RULE-S20-04`: **[v15 P3]** New tmux commands MUST be added to compat matrix before implementation. **Enforcement:** Matrix coverage audit. [v15 P3]
- `RULE-S20-05`: **[v15 P3]** Differential tests compare TermForge output against tmux binary output. **Enforcement:** Differential test suite. [v15 P3]
- `RULE-S20-06`: **[v15 P3]** Unsupported commands return typed error, not panic. **Enforcement:** Error type test. [v15 P3]
- `RULE-S20-07`: **[v15 P3]** Category grouping covers all 10 categories. **Enforcement:** Category exhaustiveness test. [v15 P3]
- `RULE-S20-08`: **[v15 P3]** Compat matrix is const and requires no I/O. **Enforcement:** Const assertion. [v15 P3]

## S21 — Feature Flags (mux-pty)

- `RULE-S21-01`: Features additive-only (INV-018). Enabling a feature MUST NOT break existing tests. **Enforcement:** CI runs with each feature individually + all combinations.
- `RULE-S21-02`: crdt and wasm mutually exclusive via compile_error!(). **Enforcement:** Conflict test.
- `RULE-S21-03`: Unknown features rejected at both compile time (Cargo) and runtime (from_name). **Enforcement:** Validation test.
- `RULE-S21-04`: **[v15 P3]** Feature flags documented in crate README with description. **Enforcement:** Doc presence lint. [v15 P3]
- `RULE-S21-05`: **[v15 P3]** Feature names are kebab-case and match Cargo feature names. **Enforcement:** Name consistency test. [v15 P3]
- `RULE-S21-06`: **[v15 P3]** FeatureSet::active() provides runtime detection. **Enforcement:** Runtime detection test. [v15 P3]
- `RULE-S21-07`: **[v15 P3]** crc32c feature is always enabled (default). **Enforcement:** Default feature test. [v15 P3]
- `RULE-S21-08`: **[v15 P3]** Feature error types implement std::error::Error. **Enforcement:** Trait test. [v15 P3]

## S22 — Platform Abstraction (mux-proto)

- `RULE-S22-01`: Platform-specific code in mux-pty only. **Enforcement:** `cargo clippy` + grep for `#[cfg(target_os` outside mux-pty.
- `RULE-S22-02`: No unsafe outside mux-pty (INV-003). **Enforcement:** `#![forbid(unsafe_code)]` in all non-mux-pty crates.
- `RULE-S22-03`: **[v15 P3]** Platform divergence explicit via PlatformCaps. Code MUST check caps before using platform features. **Enforcement:** Capability check audit. [v15 P3]
- `RULE-S22-04`: **[v15 P3]** PtySpawner trait abstracts all platform PTY differences. **Enforcement:** Trait implementation test per platform. [v15 P3]
- `RULE-S22-05`: **[v15 P3]** FreeBSD support: kqueue IO backend, posix_openpt PTY. **Enforcement:** FreeBSD CI target. [v15 P3]
- `RULE-S22-06`: **[v15 P3]** Signal handling isolated in mux-pty. **Enforcement:** Signal handler test. [v15 P3]
- `RULE-S22-07`: **[v15 P3]** IO backend fallback to poll when neither epoll nor kqueue available. **Enforcement:** Fallback test. [v15 P3]
- `RULE-S22-08`: **[v15 P3]** WASM resize is a no-op. **Enforcement:** WASM compile test. [v15 P3]

## S23 — CRDT and Collaborative Features (mux-orm)

- `RULE-S23-01`: CRDT behind feature flag. **Enforcement:** Feature gate test.
- `RULE-S23-02`: VectorClock monotonic (INV-025). **Enforcement:** Monotonic test.
- `RULE-S23-03`: DeterministicTimeSource unified (S97). **Enforcement:** Type test.
- `RULE-S23-04`: OpLog uses partition_point. **Enforcement:** Insertion test.
- `RULE-S23-05`: **[v15 P2]** NemesisScheduler present. **Enforcement:** Scheduler test.
- `RULE-S23-06`: **[v15 P2]** HistoryChecker validates consistency. **Enforcement:** History test.
- `RULE-S23-07`: **[v15 P3]** LWWFieldMap merge commutative. **Enforcement:** Commutativity test. [v15 P3]
- `RULE-S23-08`: **[v15 P3]** CRDT operations idempotent. **Enforcement:** Idempotency test. [v15 P3]
- `RULE-S23-09`: **[v15 P3]** Replay convergence for N merge orders. **Enforcement:** Convergence test. [v15 P3]
- `RULE-S23-10`: **[v15 P3]** LWW tie-break by actor_id. **Enforcement:** Tie-break test. [v15 P3]

## S24 — WASM Support (mux-grid)

- `RULE-S24-01`: L0 crates compile to wasm32-unknown-unknown. **Enforcement:** CI cross-compilation test.
- `RULE-S24-02`: WASM and CRDT mutually exclusive via compile_error!(). **Enforcement:** Feature conflict compilation test.
- `RULE-S24-03`: **[v15 P3]** WASM crates use no-std + alloc, no std::io or std::net. **Enforcement:** no-std compilation test. [v15 P3]
- `RULE-S24-04`: **[v15 P3]** WASM binary size under 256 KiB (gzipped). **Enforcement:** CI size check. [v15 P3]
- `RULE-S24-05`: **[v15 P3]** wasm-bindgen exports for PackedCell, Grid viewport, parser. **Enforcement:** wasm-pack test. [v15 P3]
- `RULE-S24-06`: **[v15 P3]** WASM crate list maintained (exactly 2 L0 crates). **Enforcement:** Crate list assertion. [v15 P3]
- `RULE-S24-07`: **[v15 P3]** WASM viewport provides cell_count() for browser rendering. **Enforcement:** Viewport test. [v15 P3]
- `RULE-S24-08`: **[v15 P3]** Size budget enforced in CI with regression alert. **Enforcement:** Size regression test. [v15 P3]

## S25 — DCS Passthrough (mux-grid)

- `RULE-S25-01`: DCS 0x90 classified as DcsEntry in CLASS_TABLE (INV-026). **Enforcement:** LUT byte test.
- `RULE-S25-02`: DCS passthrough piped transparently to client. **Enforcement:** Pipe integration test.
- `RULE-S25-03`: **[v15 P3]** DCS fuzz target in `fuzz/fuzz_dcs.rs`. **Enforcement:** Fuzz CI target. [v15 P3]
- `RULE-S25-04`: **[v15 P3]** DCS notification emitted on passthrough. **Enforcement:** Event listener test. [v15 P3]
- `RULE-S25-05`: **[v15 P3]** DCS payload size limited to 16 MiB. **Enforcement:** Size overflow test. [v15 P3]
- `RULE-S25-06`: **[v15 P3]** DCS subtype classification for sixel, DECDLD, DECRQSS, tmux control, iTerm2. **Enforcement:** Classification test per subtype. [v15 P3]
- `RULE-S25-07`: **[v15 P3]** DCS errors typed (PayloadTooLarge, InvalidStringTerminator). **Enforcement:** Error variant test. [v15 P3]
- `RULE-S25-08`: **[v15 P3]** DCS passthrough logged to OTEL with subtype. **Enforcement:** Span attribute test. [v15 P3]

## S26 — Consolidated Rules (AGENTS governance)

- `RULE-S26-01`: Every section contributes rules to this consolidated index. **Enforcement:** Section audit (min 3 rules/section).
- `RULE-S26-02`: Rules use `RULE-Snn-xx` format with explicit enforcement. **Enforcement:** Regex format lint.
- `RULE-S26-03`: **[v15 P3]** Every rule has concrete enforcement artifact (test, lint, or CI check). **Enforcement:** Enforcement string non-empty audit. [v15 P3]
- `RULE-S26-04`: **[v15 P3]** Rules MUST NOT be removed, only deprecated. **Enforcement:** Rule ID stability test (compare against golden file). [v15 P3]
- `RULE-S26-05`: **[v15 P3]** Rule IDs are unique across the entire registry. **Enforcement:** Uniqueness test. [v15 P3]

## S27 — Risk Register (risk governance)

- `RULE-S27-01`: Risks numbered R001-R145+ with sequential IDs. **Enforcement:** Risk count assertion.
- `RULE-S27-02`: Each risk has non-empty mitigation string. **Enforcement:** Mitigation presence audit.
- `RULE-S27-03`: **[v15 P2]** Per-risk mitigation contracts (testable assertions). **Enforcement:** Contract test.
- `RULE-S27-04`: **[v15 P3]** Every risk has associated TST in test_id field. **Enforcement:** TST linkage audit. [v15 P3]
- `RULE-S27-05`: **[v15 P3]** H/H and M/H risks MUST be mitigated before feature ships. **Enforcement:** Severity gate in CI. [v15 P3]
- `RULE-S27-06`: **[v15 P3]** Risk IDs are unique and sequential. **Enforcement:** Uniqueness test. [v15 P3]
- `RULE-S27-07`: **[v15 P3]** New risks added on each spec version bump. **Enforcement:** Version-tagged risk audit. [v15 P3]
- `RULE-S27-08`: **[v15 P3]** Risk descriptions are non-empty and specific. **Enforcement:** Description length test. [v15 P3]

## S28 — Plan Evolution and Migration (release governance)

- `RULE-S28-01`: Plan evolution table maintained. **Enforcement:** Table presence.
- `RULE-S28-02`: Breaking changes documented with migration. **Enforcement:** Migration audit.
- `RULE-S28-03`: **[v15 P3]** Definitive passes marked as such. **Enforcement:** Status flag test. [v15 P3]
- `RULE-S28-04`: **[v15 P3]** Breaking changes carry migration instructions. **Enforcement:** Migration field non-empty test. [v15 P3]
- `RULE-S28-05`: **[v15 P3]** Spec version enum is exhaustive. **Enforcement:** Variant count test. [v15 P3]

## S29 — Testing Strategy (testing governance)

- `RULE-S29-01`: Unit tests per module with `#[cfg(test)] mod tests`. **Enforcement:** Test presence lint.
- `RULE-S29-02`: 5+ fuzz targets in `fuzz/`. **Enforcement:** Target count assertion.
- `RULE-S29-03`: Property tests (proptest) for all encoding round-trips. **Enforcement:** Proptest presence per crate.
- `RULE-S29-04`: **[v15 P3]** Every section has 5+ TSTs. **Enforcement:** CI TST count audit. [v15 P3]
- `RULE-S29-05`: **[v15 P3]** Differential testing against tmux binary. **Enforcement:** Parity test suite. [v15 P3]
- `RULE-S29-06`: **[v15 P3]** Test isolation via unique socket names. **Enforcement:** Socket name uniqueness test. [v15 P3]
- `RULE-S29-07`: **[v15 P3]** Snapshot tests with insta for grid state golden files. **Enforcement:** insta snapshot review in PR. [v15 P3]
- `RULE-S29-08`: **[v15 P3]** TestGuard MUST clean up all resources on drop. **Enforcement:** Cleanup verification test. [v15 P3]

## S30 — Benchmarks (evolution governance)

- `RULE-S30-01`: Criterion benchmarks for all hot paths. **Enforcement:** Bench target presence.
- `RULE-S30-02`: 12+ benchmark targets. **Enforcement:** Target count assertion.
- `RULE-S30-03`: **[v15 P3]** Benchmark results stored as CI artifacts. **Enforcement:** CI artifact upload check. [v15 P3]
- `RULE-S30-04`: **[v15 P3]** Micro-benchmarks MUST have ns-level threshold. **Enforcement:** Threshold presence test. [v15 P3]
- `RULE-S30-05`: **[v15 P3]** >5% regression on any benchmark fails CI. **Enforcement:** critcmp threshold check. [v15 P3]
- `RULE-S30-06`: **[v15 P3]** Benchmark names are unique across all targets. **Enforcement:** Uniqueness test. [v15 P3]

## S31 — Governance and RFC Process (infrastructure)

- `RULE-S31-01`: Architecture changes require RFC with decision record. **Enforcement:** PR policy (no merge without DR link).
- `RULE-S31-02`: Breaking changes need 2/3 maintainer approval. **Enforcement:** Approval gate (`has_quorum()` check).
- `RULE-S31-03`: Decision records maintained in `docs/decisions/`. **Enforcement:** DR file presence lint.
- `RULE-S31-04`: **[v15 P3]** DRs reference invariants affected. **Enforcement:** Invariant linkage audit (non-empty for breaking changes). [v15 P3]
- `RULE-S31-05`: **[v15 P3]** DRs reference settled decisions affected. **Enforcement:** SD linkage audit. [v15 P3]
- `RULE-S31-06`: **[v15 P3]** RFC lifecycle: Proposed -> UnderReview -> Accepted/Rejected -> Superseded. **Enforcement:** Status transition test. [v15 P3]
- `RULE-S31-07`: **[v15 P3]** External contributor RFCs require one core-team sponsor. **Enforcement:** Sponsor field presence. [v15 P3]
- `RULE-S31-08`: **[v15 P3]** RFC template fields: Problem, Solution, Alternatives, Impact, Migration. **Enforcement:** Template coverage lint. [v15 P3]
- `RULE-S31-09`: **[v15 P3]** Superseded DRs MUST reference the superseding DR. **Enforcement:** Linkage audit. [v15 P3]
- `RULE-S31-10`: **[v15 P3]** DR status transitions logged with timestamp. **Enforcement:** Transition log test. [v15 P3]

## S32 — Termlet Testing Framework (mux-termlet + integration)

- `RULE-S32-01`: Termlet trait is object-safe. **Enforcement:** Trait object test. (v13 P3)
- `RULE-S32-02`: Termlet::spawn() returns typed error. **Enforcement:** Error type test. (v13 P3)
- `RULE-S32-03`: Termlet::send_keys() accepts tmux key notation. **Enforcement:** Key parse test. (v13 P3)
- `RULE-S32-04`: Termlet::wait_for() has configurable timeout. **Enforcement:** Timeout test. (v13 P3)
- `RULE-S32-05`: Termlet::snapshot() is deterministic (INV-033). **Enforcement:** Determinism test. (v13 P3)
- `RULE-S32-06`: Termlet::resize() validates minimum 1x1. **Enforcement:** Bounds test. (v13 P3)
- `RULE-S32-07`: Termlet::kill() releases all resources. **Enforcement:** Drop test. (v13 P3)
- `RULE-S32-08`: TermletError has 16 variants. **Enforcement:** Variant count test. (v13 P3)
- `RULE-S32-09`: TermletError implements std::error::Error. **Enforcement:** Trait test. (v13 P3)
- `RULE-S32-10`: TermletError::error_code() returns unique u16. **Enforcement:** Uniqueness test. (v13 P3)
- `RULE-S32-11`: FakePty produces deterministic output. **Enforcement:** Output assertion. (v13 P3)
- `RULE-S32-12`: FakePty supports resize. **Enforcement:** Resize test. (v13 P3)
- `RULE-S32-13`: TermletBuilder follows builder pattern. **Enforcement:** Chain test. (v13 P3)
- `RULE-S32-14`: TermletBuilder default shell is $SHELL. **Enforcement:** Env test. (v13 P3)
- `RULE-S32-15`: TermletBuilder socket_path is isolated. **Enforcement:** Path test. (v13 P3)
- `RULE-S32-16`: Resource quotas enforce max_ptys. **Enforcement:** Quota test. (v13 P3)
- `RULE-S32-17`: Resource quotas enforce max_memory. **Enforcement:** Memory test. (v13 P3)
- `RULE-S32-18`: Resource quotas enforce max_duration. **Enforcement:** Duration test. (v13 P3)
- `RULE-S32-19`: Sandbox isolates file system. **Enforcement:** FS isolation test. (v13 P3)
- `RULE-S32-20`: Sandbox isolates network. **Enforcement:** Network isolation test. (v13 P3)
- `RULE-S32-21`: Recorder captures events with timestamps. **Enforcement:** Event test. (v13 P3)
- `RULE-S32-22`: Recorder uses .tlet file format. **Enforcement:** Magic test. (v13 P3)
- `RULE-S32-23`: Recorder size limit prevents unbounded growth. **Enforcement:** Size test. (v13 P3)
- `RULE-S32-24`: FFI exports via cbindgen. **Enforcement:** ABI test. (v13 P3)
- `RULE-S32-25`: FFI handles NULL pointers safely. **Enforcement:** NULL test. (v13 P3)
- `RULE-S32-26`: GraphemeArena::insert returns valid index. **Enforcement:** Index bounds test. (v13 P3)
- `RULE-S32-27`: GraphemeArena dedup stores identical graphemes once. **Enforcement:** Dedup count test. (v13 P3)
- `RULE-S32-28`: GraphemeArena::lookup returns correct string. **Enforcement:** Round-trip test. (v13 P3)
- `RULE-S32-29`: GraphemeArena capacity tracked. **Enforcement:** Capacity test. (v13 P3)
- `RULE-S32-30`: GraphemeArena overflow returns CapacityExceeded. **Enforcement:** Overflow test. (v13 P3)
- `RULE-S32-31`: GraphemeArena 14-bit index cap enforced. **Enforcement:** Index limit test. (v13 P3)
- `RULE-S32-32`: GraphemeArena thread-safe via interior mutability. **Enforcement:** Send+Sync test. (v13 P3)
- `RULE-S32-33`: GraphemeArena metrics emitted to OTEL. **Enforcement:** Metric test. (v13 P3)
- `RULE-S32-34`: GraphemeArena clear resets all indices. **Enforcement:** Clear test. (v13 P3)
- `RULE-S32-35`: GraphemeArena snapshot serializable. **Enforcement:** Serde test. (v13 P3)
- `RULE-S32-36`: GraphemeArena edge cases (empty string, max-length, combining chars). (v13 P3). **Enforcement:** Range-expanded harness validation (S32 integration matrix).
- `RULE-S32-40`: GraphemeArena edge cases (empty string, max-length, combining chars). (v13 P3). **Enforcement:** Range-expanded harness validation (S32 integration matrix).
- `RULE-S32-41`: COW scrollback uses Arc<Vec<Cell>>. **Enforcement:** Type assertion. (v13 P3)
- `RULE-S32-42`: COW clone is O(1) (Arc::clone). **Enforcement:** Timing test. (v13 P3)
- `RULE-S32-43`: COW mutation uses Arc::make_mut. **Enforcement:** Mutation test. (v13 P3)
- `RULE-S32-44`: COW eviction respects scrollback limit. **Enforcement:** Limit test. (v13 P3)
- `RULE-S32-45`: COW comparison is byte-level. **Enforcement:** Comparison test. (v13 P3)
- `RULE-S32-46`: COW trigger count tracked in metrics. **Enforcement:** Metric test. (v13 P3)
- `RULE-S32-47`: COW scrollback deterministic in replay. **Enforcement:** Determinism test. (v13 P3)
- `RULE-S32-48`: COW edge cases (empty, max-size, concurrent readers). (v13 P3). **Enforcement:** Range-expanded harness validation (S32 integration matrix).
- `RULE-S32-55`: COW edge cases (empty, max-size, concurrent readers). (v13 P3). **Enforcement:** Range-expanded harness validation (S32 integration matrix).
- `RULE-S32-56`: DeterministicTimeSource wall clock access. **Enforcement:** Wall test. (v13 P3)
- `RULE-S32-57`: DeterministicTimeSource Lamport clock monotonic. **Enforcement:** Monotonic test. (v13 P3)
- `RULE-S32-58`: DeterministicTimeSource vector clock merge. **Enforcement:** Merge test. (v13 P3)
- `RULE-S32-59`: DeterministicTimeSource monotonic guard prevents backward jumps. **Enforcement:** Guard test. (v13 P3)
- `RULE-S32-60`: DeterministicTimeSource seed-based determinism. **Enforcement:** Seed test. (v13 P3)
- `RULE-S32-61`: TimeSource edge cases (overflow, precision, timezone). (v13 P3). **Enforcement:** Range-expanded harness validation (S32 integration matrix).
- `RULE-S32-70`: TimeSource edge cases (overflow, precision, timezone). (v13 P3). **Enforcement:** Range-expanded harness validation (S32 integration matrix).
- `RULE-S32-71`: NemesisScheduler delay injection. **Enforcement:** Delay test. (v13 P3)
- `RULE-S32-72`: NemesisScheduler partition injection. **Enforcement:** Partition test. (v13 P3)
- `RULE-S32-73`: NemesisScheduler reorder injection. **Enforcement:** Reorder test. (v13 P3)
- `RULE-S32-74`: HistoryChecker linearizability verification. **Enforcement:** Linearity test. (v13 P3)
- `RULE-S32-75`: HistoryChecker serializable history. **Enforcement:** Serializable test. (v13 P3)
- `RULE-S32-76`: Nemesis+History edge cases (empty history, single-op, concurrent). (v13 P3). **Enforcement:** Range-expanded harness validation (S32 integration matrix).
- `RULE-S32-80`: Nemesis+History edge cases (empty history, single-op, concurrent). (v13 P3). **Enforcement:** Range-expanded harness validation (S32 integration matrix).
- `RULE-S32-81`: PtyHandle Allocated state initial. **Enforcement:** State test. (v14 P3)
- `RULE-S32-82`: PtyHandle Spawned state after spawn. **Enforcement:** Transition test. (v14 P3)
- `RULE-S32-83`: PtyHandle Running state during operation. **Enforcement:** State test. (v14 P3)
- `RULE-S32-84`: PtyHandle Stopping state on graceful shutdown. **Enforcement:** Transition test. (v14 P3)
- `RULE-S32-85`: PtyHandle Exited state after child exit. **Enforcement:** Exit test. (v14 P3)
- `RULE-S32-86`: PtyHandle Reaped state after waitpid. **Enforcement:** Reap test. (v14 P3)
- `RULE-S32-87`: PtyHandle Closed terminal state. **Enforcement:** Terminal test. (v14 P3)
- `RULE-S32-88`: PtyHandle invalid transitions rejected. **Enforcement:** Invalid transition test. (v14 P3)
- `RULE-S32-89`: PtyHandle generation counter prevents ABA. **Enforcement:** Generation test. (v14 P3)
- `RULE-S32-90`: PtyHandle Drop releases resources. **Enforcement:** Drop test. (v14 P3)
- `RULE-S32-91`: Normative v15 rule carried from definitive registry. **Enforcement:** Conformance test and CI policy check.
- `RULE-S32-100`: PtyHandle edge cases (double-close, signal-during-spawn, zombie). (v14 P3). **Enforcement:** Range-expanded harness validation (S32 integration matrix).
- `RULE-S32-101`: Grid::put_char writes at cursor position. **Enforcement:** Position test. (v14 P3)
- `RULE-S32-102`: Grid::scroll_up moves lines correctly. **Enforcement:** Scroll test. (v14 P3)
- `RULE-S32-103`: Grid::resize preserves visible content. **Enforcement:** Resize content test. (v14 P3)
- `RULE-S32-104`: Grid::clear resets all cells. **Enforcement:** Clear test. (v14 P3)
- `RULE-S32-105`: Grid cursor wraps at right margin. **Enforcement:** Wrap test. (v14 P3)
- `RULE-S32-106`: Grid edge cases (zero-width, overflow, CJK). (v14 P3). **Enforcement:** Range-expanded harness validation (S32 integration matrix).
- `RULE-S32-110`: Grid edge cases (zero-width, overflow, CJK). (v14 P3). **Enforcement:** Range-expanded harness validation (S32 integration matrix).
- `RULE-S32-111`: Python binding round-trip for Session. **Enforcement:** PyO3 test. (v14 P3)
- `RULE-S32-112`: Python binding round-trip for Window. **Enforcement:** PyO3 test. (v14 P3)
- `RULE-S32-113`: Python binding round-trip for Pane. **Enforcement:** PyO3 test. (v14 P3)
- `RULE-S32-114`: Node.js binding round-trip for Session. **Enforcement:** napi test. (v14 P3)
- `RULE-S32-115`: Node.js binding round-trip for Window. **Enforcement:** napi test. (v14 P3)
- `RULE-S32-116`: Node.js binding round-trip for Pane. **Enforcement:** napi test. (v14 P3)
- `RULE-S32-117`: Python QuerySet-like filtering. **Enforcement:** filter() test. (v14 P3)
- `RULE-S32-118`: Node.js QuerySet-like filtering. **Enforcement:** filter() test. (v14 P3)
- `RULE-S32-119`: Language binding error propagation. (v14 P3). **Enforcement:** Range-expanded harness validation (S32 integration matrix).
- `RULE-S32-120`: Language binding error propagation. (v14 P3). **Enforcement:** Range-expanded harness validation (S32 integration matrix).
- `RULE-S32-121`: Termlet lifecycle: spawn creates PTY. **Enforcement:** Spawn test. (v15 P1)
- `RULE-S32-122`: Termlet lifecycle: interact sends keys. **Enforcement:** Interact test. (v15 P1)
- `RULE-S32-123`: Termlet lifecycle: capture reads output. **Enforcement:** Capture test. (v15 P1)
- `RULE-S32-124`: Termlet lifecycle: teardown releases resources. **Enforcement:** Teardown test. (v15 P1)
- `RULE-S32-125`: Termlet lifecycle edge cases (timeout, crash, signal). (v15 P1). **Enforcement:** Range-expanded harness validation (S32 integration matrix).
- `RULE-S32-130`: Termlet lifecycle edge cases (timeout, crash, signal). (v15 P1). **Enforcement:** Range-expanded harness validation (S32 integration matrix).
- `RULE-S32-131`: Snapshot encode produces valid bytes. **Enforcement:** Encode test. (v15 P2)
- `RULE-S32-132`: Snapshot decode recovers original. **Enforcement:** Decode test. (v15 P2)
- `RULE-S32-133`: Recording replay produces identical output. **Enforcement:** Replay test. (v15 P2)
- `RULE-S32-134`: Snapshot/recording edge cases (corrupt, truncated, version). (v15 P2). **Enforcement:** Range-expanded harness validation (S32 integration matrix).
- `RULE-S32-140`: Snapshot/recording edge cases (corrupt, truncated, version). (v15 P2). **Enforcement:** Range-expanded harness validation (S32 integration matrix).
- `RULE-S32-141`: CRDT merge produces convergent state. **Enforcement:** Convergence test. (v15 P2)
- `RULE-S32-142`: CRDT actor management (add, remove, GC). **Enforcement:** Actor test. (v15 P2)
- `RULE-S32-143`: CRDT edge cases (empty, single-actor, clock overflow). (v15 P2). **Enforcement:** Range-expanded harness validation (S32 integration matrix).
- `RULE-S32-145`: CRDT edge cases (empty, single-actor, clock overflow). (v15 P2). **Enforcement:** Range-expanded harness validation (S32 integration matrix).
- `RULE-S32-146`: **[v15 P3]** InnerPtyState MUST be shared by typestate and dynamic handles (S99). **Enforcement:** Single enum test. [v15 P3]
- `RULE-S32-147`: **[v15 P3]** All Termlet public APIs MUST be deterministic under fixed seed (INV-033, S100). **Enforcement:** Determinism test. [v15 P3]
- `RULE-S32-148`: **[v15 P3]** Backpressure on send_keys when output buffer full. **Enforcement:** Buffer test. [v15 P3]
- `RULE-S32-149`: **[v15 P3]** Replay MUST be idempotent by key. **Enforcement:** Idempotency test. [v15 P3]
- `RULE-S32-150`: **[v15 P3]** ResizePayload encode/decode MUST round-trip. **Enforcement:** Payload test. [v15 P3]
- `RULE-S32-151`: **[v15 P3]** PtyRegistry MUST track capacity utilization. **Enforcement:** Utilization test. [v15 P3]
- `RULE-S32-152`: **[v15 P3]** PtyRegistry capacity exceeded MUST return typed error. **Enforcement:** Error test. [v15 P3]
- `RULE-S32-153`: **[v15 P3]** Sandbox mode opt-in via TermletBuilder. **Enforcement:** Builder test. [v15 P3]
- `RULE-S32-154`: **[v15 P3]** Recorder size limit MUST prevent unbounded growth. **Enforcement:** Limit test. [v15 P3]
- `RULE-S32-155`: **[v15 P3]** All error types in Section 32 implement std::error::Error + Send + Sync. **Enforcement:** Trait test. [v15 P3]

