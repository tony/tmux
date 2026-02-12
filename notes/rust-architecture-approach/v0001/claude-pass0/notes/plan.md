# TermForge Implementation Roadmap

## Phase 1: Foundation (Weeks 1-4)

### Milestone: Core types compile, all invariants have test scaffolding

- [ ] `mux-types`: Cell, PackedCell, Line, Style, IdentityManifest, error types
- [ ] `mux-parser`: CLASS_TABLE[256], ByteClass, step() function, ParserState
- [ ] `mux-grapheme-arena`: GraphemeArena with 14-bit ext index, dedup
- [ ] Workspace Cargo.toml with all crates, CI pipeline skeleton
- [ ] Property tests for PackedCell round-trip, CLASS_TABLE oracle match
- [ ] Fuzz targets: fuzz_parser, fuzz_grapheme

### Quality gates:
- `cargo build --workspace` succeeds
- `cargo test --workspace` passes
- `cargo clippy --workspace` clean
- All INV-* have at least one test

## Phase 2: Grid and Snapshot (Weeks 5-8)

### Milestone: Grid renders content, snapshots round-trip

- [ ] `mux-grid`: VecDeque<Line>, put_char/put_grapheme, scroll operations
- [ ] `mux-grid`: Arc COW scrollback (S96), viewport management
- [ ] `mux-snapshot`: 31-byte header, CRC32C encode/decode, PackedCell serialization
- [ ] `mux-snapshot`: GraphemeArena trailer, FNV-1a decode backward compat
- [ ] Property tests for snapshot round-trip (encode -> decode -> compare)
- [ ] Fuzz targets: fuzz_snapshot, fuzz_dcs

### Quality gates:
- Grid operations are correct for 24x80, 100x200, 1x1 dimensions
- Snapshot encode/decode round-trip for 1000+ random grids
- CRC32C checksum validates on all platforms

## Phase 3: PTY and Protocol (Weeks 9-12)

### Milestone: PTY lifecycle works, wire protocol frames encode/decode

- [ ] `mux-pty`: PtyHandle typestate, InnerPtyState, DynPtyHandle
- [ ] `mux-pty`: Platform abstraction (Linux openpty, macOS posix_openpt)
- [ ] `mux-proto`: Wire protocol v8 TLV frames, FrameType enum
- [ ] `mux-proto`: Frame validation, size limits, version handshake
- [ ] `mux-time`: DeterministicTimeSource, MonotonicGuard, Lamport clock
- [ ] Fuzz target: fuzz_protocol

### Quality gates:
- PTY spawn/kill/restart lifecycle test passes
- Wire protocol frames encode/decode round-trip
- Frame size limit enforced (64 KiB)
- DeterministicTimeSource produces reproducible timestamps

## Phase 4: Termlet and ORM (Weeks 13-16)

### Milestone: Termlet SDK works end-to-end, ORM queries functional

- [ ] `mux-termlet`: TermletConfig, TermletBuilder, resource quotas
- [ ] `mux-termlet`: send_keys(), wait_for(), expect_or_fail(), snapshot()
- [ ] `mux-termlet`: TermletGuard with Drop cleanup
- [ ] `mux-orm`: QueryList<T>, Queryable trait, filter/get semantics
- [ ] `mux-test-support`: Test harness, TestGuard, socket isolation
- [ ] `mux-test-support`: Differential test runner, snapshot test utilities
- [ ] Integration tests: spawn session, send keys, verify output

### Quality gates:
- Termlet spawn/send_keys/capture round-trip works
- QueryList filter/get with proper error handling
- Test isolation: 100 parallel tests with no interference
- Resource quotas enforced and tested

## Phase 5: Polish and Compatibility (Weeks 17-20)

### Milestone: tmux compatibility parity >= 80%, benchmarks established

- [ ] Compatibility matrix with differential tests against tmux binary
- [ ] All 12+ Criterion benchmark targets implemented
- [ ] OTEL integration (feature-gated)
- [ ] Config hot-reload with TOML support
- [ ] Key binding system with hierarchical scoping
- [ ] Copy mode with Vi/Emacs bindings
- [ ] Status bar rendering with tmux format compatibility
- [ ] Mouse support (SGR extended protocol)
- [ ] Documentation: rustdoc for all public APIs

### Quality gates:
- tmux compatibility parity >= 80% for core commands
- No benchmark regression > 5% vs baseline
- All 405+ rules have enforcement artifacts
- All 34 invariants have passing tests
- All fuzz targets run for >= 1 hour with no panics
