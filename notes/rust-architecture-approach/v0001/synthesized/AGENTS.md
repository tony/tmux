# AGENTS.md -- TermForge v15 Rule Catalog

All 405+ rules from the v15 DEFINITIVE specification, organized by section.
Each rule has a unique ID (`RULE-Snn-xx`), description, and enforcement method.

---

## S01: Project Identity (15 rules)

| Rule | Description | Enforcement |
|------|-------------|-------------|
| RULE-S01-01 | Project name is TermForge | Const check |
| RULE-S01-02 | Binary name is `termforge` | Cargo.toml lint |
| RULE-S01-03 | Library prefix is `mux-` | Crate name lint |
| RULE-S01-04 | Rust edition is 2021 | Cargo.toml check |
| RULE-S01-05 | MSRV is 1.75.0 | CI MSRV gate |
| RULE-S01-06 | License is MIT OR Apache-2.0 | License check |
| RULE-S01-07 | IdentityManifest is canonical metadata struct | Import lint |
| RULE-S01-08 | BuildProfile has 3 variants (Debug, Release, Profiling) | Variant count test |
| RULE-S01-09 | git_sha populated in release builds | CI build check |
| RULE-S01-10 | IdentityManifest derives Clone + PartialEq + Eq | Trait impl test |
| RULE-S01-11 | BuildProfile::Profiling variant present | Variant test |
| RULE-S01-12 | target_triple MUST be non-empty | Field presence test |
| RULE-S01-13 | feature_flags MUST record active Cargo features | Build injection test |
| RULE-S01-14 | version_string() MUST contain spec version tag | String match test |
| RULE-S01-15 | Identity serialization MUST be canonical (sorted keys, INV-034) | Round-trip test |

## S02: Quality Gates and CI Matrix (12 rules)

| Rule | Description | Enforcement |
|------|-------------|-------------|
| RULE-S02-01 | Three release lanes: LTS, Current, Preview | Lane enum test |
| RULE-S02-02 | Preview failures are warnings | Gate logic test |
| RULE-S02-03 | LTS/Current failures block release | Gate logic test |
| RULE-S02-04 | Waivers have expiry timestamps | Expiry field test |
| RULE-S02-05 | Expired waivers block release | Expiry logic test |
| RULE-S02-06 | CI matrix has 16 jobs (2 OS x 2 channels x 4 features) | Matrix count test |
| RULE-S02-07 | GateOutcome has 5 variants | Variant count test |
| RULE-S02-08 | Gate results are Clone + PartialEq + Eq (INV-019) | Trait impl test |
| RULE-S02-09 | FailWithBypass for security hotfixes | Bypass test |
| RULE-S02-10 | 4 feature sets: default, crdt, crc32c, wasm | Feature test |
| RULE-S02-11 | Gate evaluation MUST be deterministic (INV-033) | Repeated run test |
| RULE-S02-12 | Gate audit trail MUST be JSON-serializable (INV-034) | Serde test |

## S03: Crate DAG and Dependency Policy (10 rules)

| Rule | Description | Enforcement |
|------|-------------|-------------|
| RULE-S03-01 | Crate dependency graph is acyclic | `cargo deny` CI |
| RULE-S03-02 | L0 crates have no external deps (except compact_str) | Dep count check |
| RULE-S03-03 | All library crates use `mux-` prefix (INV-002) | Name lint |
| RULE-S03-04 | WASM target for L0 crates | `cargo build --target wasm32` |
| RULE-S03-05 | Feature flags are additive-only (INV-018) | Feature audit |
| RULE-S03-06 | 4 feature sets: default, crdt, crc32c, wasm | CI matrix |
| RULE-S03-07 | crdt and wasm MUST be mutually exclusive | Feature conflict test |
| RULE-S03-08 | Supply chain security via cargo-vet | CI audit gate |
| RULE-S03-09 | snapshot-compress feature is non-normative | Feature doc |
| RULE-S03-10 | Dependency additions require RFC review | PR policy |

## S04: Workspace Layout (10 rules)

| Rule | Description | Enforcement |
|------|-------------|-------------|
| RULE-S04-01 | Workspace uses `crates/` directory | Path check |
| RULE-S04-02 | Benchmarks in `benchmarks/` | Path check |
| RULE-S04-03 | Fuzz targets in `fuzz/` | Path check |
| RULE-S04-04 | Golden fixtures in `fixtures/` | Path check |
| RULE-S04-05 | Integration tests in `tests/` | Path check |
| RULE-S04-06 | Examples in `examples/` | Path check |
| RULE-S04-07 | CI MUST validate directory structure | CI step |
| RULE-S04-08 | Each crate MUST have README.md | File presence check |
| RULE-S04-09 | Workspace layout changes require RFC | PR policy |
| RULE-S04-10 | All crate Cargo.toml MUST reference workspace version | Cargo lint |

## S05: Grid API (12 rules)

| Rule | Description | Enforcement |
|------|-------------|-------------|
| RULE-S05-01 | Grid coordinates are zero-based (INV-007) | Index test |
| RULE-S05-02 | Mutation only via put_char/put_grapheme (INV-012) | API audit |
| RULE-S05-03 | SmallVec rejected FINAL for line cells (INV-024) | Dep check |
| RULE-S05-04 | Cell grapheme is valid UTF-8 (INV-008) | Fuzz test |
| RULE-S05-05 | Style is bitfield-packed (INV-009) | Layout test |
| RULE-S05-06 | Cell width field MUST be 0, 1, or 2 (INV-029) | Range test |
| RULE-S05-07 | CompactString for Cell grapheme (S91) | Type test |
| RULE-S05-08 | Arc<Vec<Cell>> for scrollback (S96, INV-032) | Type test |
| RULE-S05-09 | VecDeque for grid lines | Collection type test |
| RULE-S05-10 | Unicode scalar validated at insertion, not encoding | Boundary test |
| RULE-S05-11 | Scrollback limit is configurable | Config test |
| RULE-S05-12 | Scroll region bounds MUST be validated | Bounds test |

## S06: VtParser State Machine (12 rules)

| Rule | Description | Enforcement |
|------|-------------|-------------|
| RULE-S06-01 | CLASS_TABLE[256] is canonical hot path (S95) | LUT presence test |
| RULE-S06-02 | classify_match() retained as oracle | Oracle test |
| RULE-S06-03 | ByteClass has 7 variants (INV-026) | Variant count test |
| RULE-S06-04 | ByteClass is #[repr(u8)] | Attribute check |
| RULE-S06-05 | DcsEntry for 0x90 | Byte test |
| RULE-S06-06 | Step function is pure (INV-027) | No side-effect audit |
| RULE-S06-07 | All transitions defined (INV-010) | Totality test |
| RULE-S06-08 | CLASS_TABLE matches oracle for all 256 bytes | Differential test |
| RULE-S06-09 | Actions are effect-free records | Type audit |
| RULE-S06-10 | Parser fuzz target present | Fuzz target check |
| RULE-S06-11 | Unsupported CSI finals logged, not panics | Panic-free test |
| RULE-S06-12 | Parser state machine serializable for replay | Serde test |

## S07: PtyHandle Lifecycle (12 rules)

| Rule | Description | Enforcement |
|------|-------------|-------------|
| RULE-S07-01 | PtyHandle has 7 states | State count test |
| RULE-S07-02 | Lifecycle is monotonic (INV-013) | Transition test |
| RULE-S07-03 | Typestate PtyHandle for Rust core | API audit |
| RULE-S07-04 | DynPtyHandle for FFI boundary | FFI test |
| RULE-S07-05 | TryFrom<DynPtyHandle> for typed recovery (S82) | Conversion test |
| RULE-S07-06 | Generation counting prevents ABA | Generation test |
| RULE-S07-07 | validate_generation before slot ops | Pre-check test |
| RULE-S07-08 | Unified InnerPtyState enum (S99) | Single enum test |
| RULE-S07-09 | Restart barrier enforced (kill -> close -> spawn) | Order test |
| RULE-S07-10 | Closed handle returns HandleClosed deterministically | Error test |
| RULE-S07-11 | PtyHandleError implements std::error::Error (INV-005) | Trait test |
| RULE-S07-12 | Drop on PtyHandle forces kill if not terminal | Drop test |

## S08: Snapshot Format (12 rules)

| Rule | Description | Enforcement |
|------|-------------|-------------|
| RULE-S08-01 | Snapshot magic is "TFSNAP13" | Magic check |
| RULE-S08-02 | Header is 31 bytes (INV-023) | Size test |
| RULE-S08-03 | CRC32C canonical for encode (S93, INV-031) | Algorithm test |
| RULE-S08-04 | FNV-1a decode-only | Decode test |
| RULE-S08-05 | PackedCell is 64 bits (S94) | Size test |
| RULE-S08-06 | PackedCell layout: scalar:21+style:15+flags:12+width:2+ext:14 | Bit test |
| RULE-S08-07 | GraphemeArena is optional trailer (INV-028) | Format test |
| RULE-S08-08 | Platform-endian-independent LE canonical (INV-011) | Cross-platform test |
| RULE-S08-09 | Unknown algorithms are hard errors | Rejection test |
| RULE-S08-10 | Software CRC32C fallback mandatory | Fallback test |
| RULE-S08-11 | Truncated payloads rejected before UTF-8 | Order test |
| RULE-S08-12 | Snapshot round-trip MUST preserve all fields | Property test |

## S09: Wire Protocol (10 rules)

| Rule | Description | Enforcement |
|------|-------------|-------------|
| RULE-S09-01 | 100% tmux wire-protocol v8 compat (INV-001) | Golden tests |
| RULE-S09-02 | Frames are length-delimited (INV-020) | Frame test |
| RULE-S09-03 | Socket path uses termforge-<uid> prefix (INV-014) | Path test |
| RULE-S09-04 | SO_PASSCRED on Linux | Platform test |
| RULE-S09-05 | Frame size limit MUST be enforced | Size test |
| RULE-S09-06 | Replay MUST be idempotent by message ID | Replay test |
| RULE-S09-07 | FrameTooLarge error MUST include size and limit | Field test |
| RULE-S09-08 | Protocol version mismatch MUST be hard error | Version test |
| RULE-S09-09 | Differential parity against tmux mandatory | Parity test |
| RULE-S09-10 | DCS passthrough piped correctly (INV-026) | DCS test |

## S10: Configuration (12 rules)

| Rule | Description | Enforcement |
|------|-------------|-------------|
| RULE-S10-01 | tmux-compatible option names (INV-015) | Name parity audit |
| RULE-S10-02 | Hierarchical scoping: server/session/window/pane | Scope resolution test |
| RULE-S10-03 | TOML configuration format | Parse test |
| RULE-S10-04 | XDG base directory support | Path test |
| RULE-S10-05 | Hot-reload MUST NOT require server restart | Integration test |
| RULE-S10-06 | Config serialization is canonical (INV-034) | Round-trip test |
| RULE-S10-07 | Invalid keys produce ConfigError::UnknownKey | Error variant test |
| RULE-S10-08 | Config changes logged to OTEL | OTEL span test |
| RULE-S10-09 | Type-safe values: ConfigError::TypeMismatch | Type mismatch test |
| RULE-S10-10 | Config diff is deterministic | Diff symmetry test |
| RULE-S10-11 | Failed hot-reload rolls back, logs warning | Rollback test |
| RULE-S10-12 | Config values support bool, integer, string, enum, color | Type coverage test |

## S11: Layout Engine (10 rules)

| Rule | Description | Enforcement |
|------|-------------|-------------|
| RULE-S11-01 | Deterministic layout for identical input (INV-016) | Determinism test |
| RULE-S11-02 | Minimum pane size 1x1 | Size rejection test |
| RULE-S11-03 | tmux-compatible split semantics | Parity test |
| RULE-S11-04 | Split directions: horizontal, vertical | Exhaustiveness test |
| RULE-S11-05 | Layout engine is pure (no IO, no global state) | No-IO audit |
| RULE-S11-06 | Layout changes logged to OTEL | Span test |
| RULE-S11-07 | Resize preserves split ratios | Ratio preservation test |
| RULE-S11-08 | Layout state serializable for snapshots | Serde round-trip test |
| RULE-S11-09 | Layout tree supports recursive nesting | Depth > 2 test |
| RULE-S11-10 | find_pane() traverses all children | Deep find test |

## S12: Session/Window/Pane Model (10 rules)

| Rule | Description | Enforcement |
|------|-------------|-------------|
| RULE-S12-01 | Server-Session-Window-Pane hierarchy | Structure test |
| RULE-S12-02 | QueryList with typed errors | Error type test |
| RULE-S12-03 | ObjectDoesNotExist and MultipleObjectsReturned | Variant test |
| RULE-S12-04 | Pane owns Grid and PtyHandle | Ownership test |
| RULE-S12-05 | IDs are monotonically increasing | Monotonic test |
| RULE-S12-06 | Key bindings hierarchically scoped (INV-022) | Scope test |
| RULE-S12-07 | Session/Window serializable | Serde test |
| RULE-S12-08 | Pane destruction releases PtyHandle | Drop test |
| RULE-S12-09 | QueryList supports filter() | Filter test |
| RULE-S12-10 | QueryError is Send + Sync | Trait bound test |

## S13: Key Bindings (10 rules)

| Rule | Description | Enforcement |
|------|-------------|-------------|
| RULE-S13-01 | Hierarchical scoping: Global < Session < Window < Pane (INV-022) | 4-scope test |
| RULE-S13-02 | tmux-compatible key notation (C-, M-, S-, F1-F12) | Parse test |
| RULE-S13-03 | Description field MUST be non-empty for built-in bindings | Presence lint |
| RULE-S13-04 | Most-specific scope wins | Resolution test |
| RULE-S13-05 | Last-writer-wins within same scope | LWW test |
| RULE-S13-06 | Key binding changes emit OTEL span | Span test |
| RULE-S13-07 | Key table names match tmux (prefix, root, copy-mode-vi) | Parity test |
| RULE-S13-08 | Invalid modifiers produce KeyParseError | Error variant test |
| RULE-S13-09 | Repeat flag (-r) support within repeat-time | Repeat test |
| RULE-S13-10 | Custom key tables do not pollute default tables | Isolation test |

## S14: Clipboard (8 rules)

| Rule | Description | Enforcement |
|------|-------------|-------------|
| RULE-S14-01 | Clipboard size limit enforced (INV-021) | Size test |
| RULE-S14-02 | OSC 52 set and get operations | Protocol test |
| RULE-S14-03 | Clipboard sanitized: CSI, OSC, DCS stripped | Fuzz test |
| RULE-S14-04 | Clipboard changes emit OTEL span | Span test |
| RULE-S14-05 | Named buffers use BTreeMap (INV-034) | Order test |
| RULE-S14-06 | History ring respects configurable size, FIFO eviction | Overflow test |
| RULE-S14-07 | BufferNotFound error for missing names | Error variant test |
| RULE-S14-08 | OSC 52 query returns base64-encoded buffer | Base64 test |

## S15: Mouse Support (8 rules)

| Rule | Description | Enforcement |
|------|-------------|-------------|
| RULE-S15-01 | SGR extended mouse protocol | Encode/decode test |
| RULE-S15-02 | Cell and pixel modes both supported | Mode switching test |
| RULE-S15-03 | pixel_mode field with px_x/px_y | Pixel field test |
| RULE-S15-04 | Mouse events deterministic in replay (INV-033) | Record/replay test |
| RULE-S15-05 | Button code decode handles all SGR values | Exhaustive decode test |
| RULE-S15-06 | Modifier bit extraction (shift=4, meta=8, ctrl=16, motion=32) | Bitmask test |
| RULE-S15-07 | Drag events have motion flag set | Drag detection test |
| RULE-S15-08 | Mouse events emit OTEL spans in debug mode | Span test |

## S16: Status Bar (8 rules)

| Rule | Description | Enforcement |
|------|-------------|-------------|
| RULE-S16-01 | tmux-compatible format strings (#S, #W, etc.) | Format parity test |
| RULE-S16-02 | pane_count and TermForge-specific variables | Variable test |
| RULE-S16-03 | Rendering is pure function of (format, context) | Determinism test |
| RULE-S16-04 | Custom variables documented in help | Doc coverage lint |
| RULE-S16-05 | Missing variables produce empty string | Missing var test |
| RULE-S16-06 | Truncation preserves left content with "..." | Truncation test |
| RULE-S16-07 | Style directives (#[...]) parsed | Style parse test |
| RULE-S16-08 | Rate-limited updates at configurable interval | Timer test |

## S17: Copy Mode (8 rules)

| Rule | Description | Enforcement |
|------|-------------|-------------|
| RULE-S17-01 | Vi and Emacs styles with tmux parity | Key parity test |
| RULE-S17-02 | Search within scrollback | Search test |
| RULE-S17-03 | Copy mode operates on COW scrollback snapshot | Arc clone test |
| RULE-S17-04 | Search results deterministic (INV-033) | Determinism test |
| RULE-S17-05 | Three selection modes: character, line, block | Exhaustiveness test |
| RULE-S17-06 | Yanked text goes through clipboard sanitization | Integration test |
| RULE-S17-07 | Search match wraps around at end | Wrap test |
| RULE-S17-08 | Copy mode entry/exit emits OTEL span | Span test |

## S18: OTEL Observability (10 rules)

| Rule | Description | Enforcement |
|------|-------------|-------------|
| RULE-S18-01 | OTEL spans at cross-crate boundaries (INV-017) | Span audit |
| RULE-S18-02 | Metrics for key operations | Metric test |
| RULE-S18-03 | grapheme_arena.size metric present | Metric name test |
| RULE-S18-04 | cow_trigger_count metric present | Metric name test |
| RULE-S18-05 | OTEL opt-in via feature flag | Feature test |
| RULE-S18-06 | Disabled path has zero overhead | Benchmark |
| RULE-S18-07 | Semantic naming: `<namespace>.<metric_name>` | Name regex lint |
| RULE-S18-08 | Dimension labels on all metrics | Label presence test |
| RULE-S18-09 | Trace context propagation across async | Cross-boundary test |
| RULE-S18-10 | OTEL exporter configurable (Jaeger, OTLP, stdout) | Config test |

## S19: Error Handling (10 rules)

| Rule | Description | Enforcement |
|------|-------------|-------------|
| RULE-S19-01 | All errors impl Error + Send + Sync + 'static (INV-005) | Trait bound test |
| RULE-S19-02 | No panicking in library code (INV-006) | clippy lint |
| RULE-S19-03 | Structured error types with diagnostic context | Context coverage test |
| RULE-S19-04 | Error chains via source() | Source chain test |
| RULE-S19-05 | All errors derive Debug + Clone | Derive test |
| RULE-S19-06 | Unique u16 error codes via error_code() | Uniqueness test |
| RULE-S19-07 | Error codes are stable across versions | Golden file test |
| RULE-S19-08 | ErrorContext feeds into OTEL spans | Span attribute test |
| RULE-S19-09 | Display includes [Ecode] prefix | Display format test |
| RULE-S19-10 | Parser errors include byte_offset | Offset test |

## S20: Compatibility Matrix (8 rules)

| Rule | Description | Enforcement |
|------|-------------|-------------|
| RULE-S20-01 | Per-lane compatibility tracking | Matrix assertion |
| RULE-S20-02 | Parity percentage tracked in CI | Percentage test |
| RULE-S20-03 | Every compat entry has test_id | test_id lint |
| RULE-S20-04 | New commands added to matrix before implementation | Coverage audit |
| RULE-S20-05 | Differential tests compare against tmux binary | Diff test suite |
| RULE-S20-06 | Unsupported commands return typed error | Error type test |
| RULE-S20-07 | Category grouping covers all 10 categories | Exhaustiveness test |
| RULE-S20-08 | Compat matrix is const (no I/O) | Const assertion |

## S21: Feature Flags (8 rules)

| Rule | Description | Enforcement |
|------|-------------|-------------|
| RULE-S21-01 | Features additive-only (INV-018) | CI feature test |
| RULE-S21-02 | crdt and wasm mutually exclusive | compile_error! test |
| RULE-S21-03 | Unknown features rejected | Validation test |
| RULE-S21-04 | Feature flags documented in crate README | Doc lint |
| RULE-S21-05 | Feature names are kebab-case | Name consistency test |
| RULE-S21-06 | FeatureSet::active() provides runtime detection | Runtime test |
| RULE-S21-07 | crc32c feature always enabled (default) | Default feature test |
| RULE-S21-08 | Feature error types impl std::error::Error | Trait test |

## S22: Platform Abstraction (8 rules)

| Rule | Description | Enforcement |
|------|-------------|-------------|
| RULE-S22-01 | Platform-specific code in mux-pty only | grep audit |
| RULE-S22-02 | No unsafe outside mux-pty (INV-003) | forbid(unsafe_code) |
| RULE-S22-03 | Platform divergence explicit via PlatformCaps | Capability check |
| RULE-S22-04 | PtySpawner trait abstracts platform differences | Trait impl test |
| RULE-S22-05 | FreeBSD support: kqueue, posix_openpt | FreeBSD CI |
| RULE-S22-06 | Signal handling isolated in mux-pty | Signal handler test |
| RULE-S22-07 | IO backend fallback to poll | Fallback test |
| RULE-S22-08 | WASM resize is a no-op | WASM compile test |

## S23: CRDT and Collaboration (10 rules)

| Rule | Description | Enforcement |
|------|-------------|-------------|
| RULE-S23-01 | CRDT behind feature flag | Feature gate test |
| RULE-S23-02 | VectorClock monotonic (INV-025) | Monotonic test |
| RULE-S23-03 | DeterministicTimeSource unified (S97) | Type test |
| RULE-S23-04 | OpLog uses partition_point for O(log n) | Insertion test |
| RULE-S23-05 | NemesisScheduler present | Scheduler test |
| RULE-S23-06 | HistoryChecker validates consistency | History test |
| RULE-S23-07 | LWWFieldMap merge commutative | Commutativity test |
| RULE-S23-08 | CRDT operations idempotent | Idempotency test |
| RULE-S23-09 | Replay convergence for N merge orders | Convergence test |
| RULE-S23-10 | LWW tie-break by actor_id | Tie-break test |

## S24: WASM Support (8 rules)

| Rule | Description | Enforcement |
|------|-------------|-------------|
| RULE-S24-01 | L0 crates compile to wasm32-unknown-unknown | Cross-compile test |
| RULE-S24-02 | WASM and CRDT mutually exclusive | compile_error! test |
| RULE-S24-03 | WASM crates use no-std + alloc | no-std compile test |
| RULE-S24-04 | WASM binary size under 256 KiB (gzipped) | CI size check |
| RULE-S24-05 | wasm-bindgen exports for PackedCell, Grid, parser | wasm-pack test |
| RULE-S24-06 | WASM crate list maintained (2 L0 crates) | Assertion |
| RULE-S24-07 | WASM viewport provides cell_count() | Viewport test |
| RULE-S24-08 | Size budget enforced in CI with regression alert | Size regression |

## S25: DCS Passthrough (8 rules)

| Rule | Description | Enforcement |
|------|-------------|-------------|
| RULE-S25-01 | DCS 0x90 classified as DcsEntry (INV-026) | LUT byte test |
| RULE-S25-02 | DCS passthrough piped transparently | Pipe test |
| RULE-S25-03 | DCS fuzz target in fuzz/ | Fuzz CI |
| RULE-S25-04 | DCS notification emitted on passthrough | Event listener test |
| RULE-S25-05 | DCS payload size limited to 16 MiB | Size overflow test |
| RULE-S25-06 | DCS subtype classification | Per-subtype test |
| RULE-S25-07 | DCS errors typed | Error variant test |
| RULE-S25-08 | DCS passthrough logged to OTEL | Span attribute test |

## S26: Consolidated Rules (5 rules)

| Rule | Description | Enforcement |
|------|-------------|-------------|
| RULE-S26-01 | Every section contributes rules (min 3/section) | Section audit |
| RULE-S26-02 | Rules use RULE-Snn-xx format | Regex format lint |
| RULE-S26-03 | Every rule has concrete enforcement artifact | Non-empty audit |
| RULE-S26-04 | Rules MUST NOT be removed, only deprecated | Stability test |
| RULE-S26-05 | Rule IDs are unique | Uniqueness test |

## S27: Risk Register (8 rules)

| Rule | Description | Enforcement |
|------|-------------|-------------|
| RULE-S27-01 | Risks numbered R001-R145+ | Count assertion |
| RULE-S27-02 | Each risk has non-empty mitigation | Presence audit |
| RULE-S27-03 | Per-risk mitigation contracts | Contract test |
| RULE-S27-04 | Every risk has associated TST | TST linkage audit |
| RULE-S27-05 | H/H and M/H risks mitigated before ship | Severity gate |
| RULE-S27-06 | Risk IDs unique and sequential | Uniqueness test |
| RULE-S27-07 | New risks added per spec version | Version audit |
| RULE-S27-08 | Risk descriptions are specific | Description length |

## S28: Plan Evolution (5 rules)

| Rule | Description | Enforcement |
|------|-------------|-------------|
| RULE-S28-01 | Plan evolution table maintained | Table presence |
| RULE-S28-02 | Breaking changes documented with migration | Migration audit |
| RULE-S28-03 | Definitive passes marked | Status flag test |
| RULE-S28-04 | Breaking changes carry migration instructions | Non-empty test |
| RULE-S28-05 | Spec version enum is exhaustive | Variant count test |

## S29: Testing Strategy (8 rules)

| Rule | Description | Enforcement |
|------|-------------|-------------|
| RULE-S29-01 | Unit tests per module with #[cfg(test)] | Presence lint |
| RULE-S29-02 | 5+ fuzz targets in fuzz/ | Target count |
| RULE-S29-03 | Property tests for all encoding round-trips | proptest presence |
| RULE-S29-04 | Every section has 5+ TSTs | CI TST count |
| RULE-S29-05 | Differential testing against tmux binary | Parity suite |
| RULE-S29-06 | Test isolation via unique socket names | Socket uniqueness |
| RULE-S29-07 | Snapshot tests with insta golden files | insta review |
| RULE-S29-08 | TestGuard MUST clean up all resources on drop | Cleanup test |

## S30: Benchmarks (6 rules)

| Rule | Description | Enforcement |
|------|-------------|-------------|
| RULE-S30-01 | Criterion benchmarks for all hot paths | Bench target presence |
| RULE-S30-02 | 12+ benchmark targets | Target count |
| RULE-S30-03 | Results stored as CI artifacts | CI upload check |
| RULE-S30-04 | Micro-benchmarks have ns-level threshold | Threshold test |
| RULE-S30-05 | >5% regression fails CI | critcmp threshold |
| RULE-S30-06 | Benchmark names unique | Uniqueness test |

## S31: Governance (10 rules)

| Rule | Description | Enforcement |
|------|-------------|-------------|
| RULE-S31-01 | Multi-review for contract changes | PR policy |
| RULE-S31-02 | Spec version tracked in IdentityManifest | Version field |
| RULE-S31-03 | All normative changes tagged with version | Tag audit |
| RULE-S31-04 | Breaking changes require migration path | Migration path test |
| RULE-S31-05 | Deprecation follows three-release cycle | Deprecation audit |
| RULE-S31-06 | Rule IDs stable once published | Golden file |
| RULE-S31-07 | Risk IDs stable once published | Golden file |
| RULE-S31-08 | Test IDs stable once published | Golden file |
| RULE-S31-09 | Settled decisions immutable once published | Decision audit |
| RULE-S31-10 | Footer includes consistency checklist | Footer check |

## S32: Termlet Subsections (155 rules, RULE-S32-001 through RULE-S32-155)

Section 32 covers the Termlet runtime, SDK-first testing pods, resource quotas,
sandboxing, recorder, FFI stability, and the complete integration test harness.

Key subsection rules (representative sample):

| Rule | Description | Enforcement |
|------|-------------|-------------|
| RULE-S32-001 | TermletConfig has all required fields | Field presence test |
| RULE-S32-002 | TermletBuilder validates config before spawn | Validation test |
| RULE-S32-003 | send_keys() forwards to child PTY | Key forwarding test |
| RULE-S32-004 | wait_for() with timeout returns Result | Timeout test |
| RULE-S32-005 | expect_or_fail() carries snapshot digest (S47) | Digest field test |
| RULE-S32-006 | Termlet captures grid snapshot | Snapshot capture test |
| RULE-S32-007 | Termlet resize propagates to child | Resize test |
| RULE-S32-008 | Termlet kill sends SIGKILL | Kill signal test |
| RULE-S32-009 | Termlet cleanup on drop | Drop cleanup test |
| RULE-S32-010 | Unique socket per Termlet | Socket uniqueness test |
| RULE-S32-011 | TermletError has 16 variants (S98) | Variant count test |
| RULE-S32-012 | error_code() returns unique u16 per variant | Code uniqueness test |
| RULE-S32-013 | TermletError is Send + Sync (INV-005) | Trait bound test |
| RULE-S32-014 | Resource quota CPU limit enforced (INV-033) | Quota test |
| RULE-S32-015 | Resource quota RAM limit enforced | Quota test |
| RULE-S32-016 | Quota violation returns QuotaExceeded error | Error variant test |
| RULE-S32-017 | Sandbox namespace isolation | Namespace test |
| RULE-S32-018 | Sandbox violation is typed error (INV-034) | Error type test |
| RULE-S32-019 | Recorder produces .tlet files | File extension test |
| RULE-S32-020 | Recorder is deterministic under fixed clock (INV-035) | Determinism test |
| RULE-S32-021..155 | (Additional 135 rules covering all 58 S32 subsections) | Various |

---

## Summary Statistics

| Category | Count |
|----------|-------|
| Structural | 76 |
| Behavioral | 74 |
| Safety | 50 |
| Compatibility | 62 |
| Testing | 43 |
| Observability | 10 |
| Termlet (S32) | 155 |
| **Total** | **405+** |
