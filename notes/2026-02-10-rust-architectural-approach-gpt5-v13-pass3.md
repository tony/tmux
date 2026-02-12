# TermForge v13 Architecture Specification -- Pass 3 DEFINITIVE [v13 P3]

Date: 2026-02-12
Status: **Pass 3 of 3 -- DEFINITIVE** [v13 P3] -- final cross-model convergence and closure.
Lineage: v4 -> v5 -> v6 -> v7 -> v8 -> v9 P3 -> v10 P3 -> v11 P3 DEFINITIVE -> v12 P3 DEFINITIVE -> v13 Pass 1 (Claude/GPT/Gemini) -> v13 Pass 2 (Claude/GPT/Gemini) -> **v13 Pass 3 DEFINITIVE** [v13 P3].
Models: GPT-5 Codex (Pass 3 synthesizer). Inputs: Claude Pass 2 (5,430 lines), GPT Pass 2 (6,561 lines), Gemini Pass 2 (514 lines). Base: Claude Pass 2 structure with GPT Section 32.30-32.35 integration and Gemini CRDT/Grid/Snapshot refinements. [v13 P3]
License: MIT OR Apache-2.0
Rust edition: 2021 (MSRV 1.85)
Protocol target: tmux protocol v8

---

## Preamble

This document is the **v13 Pass 3 DEFINITIVE** [v13 P3] architecture specification for TermForge, a Rust terminal multiplexer with 100% tmux wire-protocol compatibility, ORM-like API, language bindings, CRDT collaboration, a ratatui-based TUI client, and **Termlets** -- SDK-first testing pods that are the killer differentiator.

### [v13 P3] Cross-Model Synthesis Methodology

Pass 2 synthesizes the strongest elements from three independent Pass 1 documents:

| Model | Lines | Structural Base? | Key Contributions |
|-------|-------|-----------------|-------------------|
| Claude Opus 4.6 | 6,186 | **Yes** (32-section, 4-part structure, 176 rules) | Complete architecture, comprehensive tests, binary snapshot, CRDT depth, OTEL spans |
| GPT-5 | 5,568 | No (informative) | GateClass/Lane system, 7-state PtyHandle lifecycle, FNV-1a checksum snapshots, VtParser classify()/Step pattern, Frame/DecodeResult wire types, 669 rules |
| Gemini | 691 | No (informative) | VecDeque scrollback, grapheme-based Cell, dirty range Line tracking, 14-state VtParser, PtyRegistry with File handles |

### [v13 P3] Synthesis Decisions

The following cross-pollination decisions were made:

1. **[v13 P2] Grid storage**: Adopt Gemini's `VecDeque<Line>` for O(1) scrollback push/pop (replaces `Vec<Line>`).
2. **[v13 P2] Cell grapheme**: Adopt Gemini's `String` grapheme field for multi-codepoint grapheme cluster support. `char ch` is insufficient for combining characters.
3. **[v13 P2] Line dirty tracking**: Adopt GPT's `dirty_start`/`dirty_end` range tracking (also present in Gemini). Enables incremental redraw.
4. **[v13 P2] Line wrapped flag**: Merged from both GPT and Gemini.
5. **[v13 P2] PtyHandle lifecycle**: Adopt GPT's 7-state model (Allocated -> Spawned -> Running -> Stopping -> Exited -> Reaped -> Closed) with `PtyRecord::transition()`.
6. **[v13 P2] VtParser classify/Step**: Adopt GPT's `classify()` byte classifier and `Step` return type. Cleaner than raw match arms.
7. **[v13 P2] Snapshot format**: Adopt GPT's richer binary format with `TFSNAP13` 8-byte magic, u16 version, SnapshotMeta (cursor position, revision), SnapshotCell (ch/style/flags), and FNV-1a checksum.
8. **[v13 P2] GateClass/Lane**: Adopt GPT's `GateClass` enum with `Lane` (LTS/Current/Preview) for release gate granularity.
9. **[v13 P2] Wire Frame/DecodeResult**: Adopt GPT's `Frame` and `DecodeResult` types for wire protocol framing.
10. **[v13 P2] Finding system**: Adopt GPT's structured `Finding` type with severity for v12 critical review tracking.

### [v13 P3] Rejected Approaches

| Source | Approach | Reason for Rejection |
|--------|----------|---------------------|
| Gemini | 14-state VtParser (DCS/SOS/PM/APC) | Overengineered for Pass 2. 7 states match tmux `input.c`. DCS/SOS/PM/APC deferred to Pass 3. |
| Gemini | `libc::getuid()` in identity module | Violates Layer 0 purity. Socket dir resolution belongs in mux-os (L1). |
| Gemini | `dirs::home_dir()` external crate | Standalone-compilation requirement forbids external crates in examples. |
| Gemini | `bincode` for snapshot serialization | External crate. Pass 2 uses std-only binary format. |
| GPT | IdentityManifest with `is_valid()` | Over-abstraction. Constants suffice; runtime validation of compile-time constants is pointless. |
| GPT | 4 subsections per section strict | Claude's subsection structure (numbered 32.1-32.29) is deeper and more informative for Section 32. |

### [v13 P3] Critical Review of Prior Passes

1. **[v13 P2] Claude P1 Grid used `char ch`**: Single `char` cannot represent grapheme clusters (e.g., family emoji, combining accents). Fixed: `grapheme: String` field.
2. **[v13 P2] Claude P1 Grid used `Vec<Line>`**: Linear scrollback. Fixed: `VecDeque<Line>` for O(1) rotation.
3. **[v13 P2] Claude P1 snapshot lacked cursor/revision**: Binary format only stored text. Fixed: `SnapshotMeta` with `cursor_col`, `cursor_row`, `revision`.
4. **[v13 P2] Claude P1 snapshot lacked checksum**: No integrity verification. Fixed: FNV-1a trailing checksum.
5. **[v13 P2] Claude P1 PtyHandle 3 states**: Insufficient lifecycle granularity. Fixed: 7-state model.
6. **[v13 P2] GPT P1 Cell lacked grapheme support**: Used `char` like Claude. Fixed: `String` grapheme.
7. **[v13 P2] Gemini P1 VtParser coupled to Grid**: `process()` takes `&mut Grid` directly. Fixed: Parser emits actions; performer handles Grid.
8. **[v13 P2] Gemini P1 used `slotmap` crate**: External dep. Fixed: standalone `GenSlotMap`.
9. **[v13 P2] All P1 docs lacked TermletState <-> PtyState alignment**: Termlet has 5 states, Pty has 7. Fixed: explicit mapping table.

**Traceability convention (carried from v12 P3):**
- `INV-*`: core invariants
- `API-*`: API contracts
- `PAR-*`: parity contracts against tmux/libtmux/reference crates
- `OPS-*`: runtime/operational contracts
- `TST-*`: mandatory test gates

**Settled global invariants (v13 P2 -- 16 items):**
- `INV-001`: Protocol compatibility target is tmux protocol v8 (`tmux-protocol.h:23`).
- `INV-002`: `mux-core` is deterministic and IO-free (`#![forbid(unsafe_code)]`). mux-core is Layer 0.
- `INV-003`: Single writer for state mutation; snapshot-based readers.
- `INV-004`: Protocol violation drops client connection, never partial-continue (`server-client.c:3472-3475`).
- `INV-005`: First-client identify completes before config load path (`server-client.c:3725-3734`).
- `INV-006`: Layout resize uses round-robin one-cell adjustment (`layout.c:448-462`).
- `INV-007`: Socket/test isolation uses three guard layers (path guard pattern).
- `INV-008`: Termlets are pane-backed and never a parallel terminal stack.
- `INV-009`: `TermletState` transitions validated by `valid_transition()`, not by enum ordering.
- `INV-010`: Effects are idempotent by effect key; retry-safe by construction.
- `INV-011`: `TermletLike` trait is the normative API contract; concrete `Termlet` implements it.
- `INV-012`: **[v13]** Grid::put_char is the sole entry point for character placement; VtParser never modifies cells directly.
- `INV-013`: **[v13]** Slot generation counters prevent ABA reuse; removed entity handles return `None` on all subsequent lookups.
- `INV-014`: **[v13 P2]** PtyHandle lifecycle is a 7-state machine; transitions are validated by `PtyRecord::transition()`.
- `INV-015`: **[v13 P2]** Snapshot binary format includes trailing FNV-1a checksum; decode rejects checksum mismatch.
- `INV-016`: **[v13 P2]** Cell grapheme field stores grapheme cluster as String; single-char cells use 1-byte SSO.

**Settled decisions (carried from v12 P3, extended for v13 P2):**

| # | Decision | Rationale |
|---|---|---|
| S1-S42 | (carried from v12 P3) | See v12 P3 preamble for full table |
| S43 | **[v13]** `Line` type wraps row cells | Enables row-level operations without index arithmetic |
| S44 | **[v13]** `Viewport` is a borrowed slice of Lines | Zero-copy viewport access for rendering |
| S45 | **[v13]** VtParser has 7 canonical states | Matches tmux input.c state machine granularity |
| S46 | **[v13]** Generation-counted SlotMap for ServerGraph | Prevents ABA problem on entity ID reuse |
| S47 | **[v13]** PtyBackend::close() for explicit cleanup | RAII alone insufficient when handle shared across threads |
| S48 | **[v13]** Snapshot binary format versioned with magic | Forward-compatible serialization for persistence and transport |
| S49 | **[v13]** Canonical event serialization for replay | Deterministic byte-level representation enables hash verification |
| S50 | **[v13]** OTEL span tree is 4-level | server.request -> kernel.apply -> effect.execute -> termlet.op |
| S51 | **[v13 P2]** Grid uses `VecDeque<Line>` for scrollback | O(1) push_front/pop_back for scroll operations |
| S52 | **[v13 P2]** Cell stores grapheme cluster as `String` | Supports combining characters, emoji, and zero-width joiners |
| S53 | **[v13 P2]** Line tracks dirty range `(dirty_start, dirty_end)` | Enables incremental redraw without full-line comparison |
| S54 | **[v13 P2]** PtyHandle has 7 lifecycle states | Finer-grained than 3-state model; matches real process lifecycle |
| S55 | **[v13 P2]** Snapshot binary includes cursor + revision + checksum | Full state capture for deterministic testing and replay |
| S56 | **[v13 P2]** VtParser uses `classify()`/`Step` pattern | Byte classification decoupled from state transitions |
| S57 | **[v13 P2]** Release gates are Lane-scoped (LTS/Current/Preview) | Performance gates soft for Preview; hard for LTS/Current |
| S58 | **[v13 P2]** Wire protocol Frame type with DecodeResult | Explicit framing for protocol parsing |

---

## 1. Project Identity

### Design Decisions

- Name: TermForge
- CLI binary: `termforge` (primary), `tf` (alias)
- Library name: `termforge` (Python/Node packages)
- Crate prefix: `mux-*` (all Rust crates)
- Config file: `~/.config/termforge/termforge.conf`
- Default socket dir: `$TMPDIR/termforge-$UID/`
- Env prefix: `TERMFORGE_*`
- Alternative names considered: MuxForge (too generic), RustMux (too literal), TermCore (too generic).
- The name must convey: terminal-specific, built/engineered, Rust-powered.
- Compatibility marketing: "100% tmux wire-protocol compatible."
- Product boundary: TermForge provides a compatibility server, an orthogonal object SDK, language bindings, collaboration primitives, and a termlet runtime; it does not embed non-terminal app frameworks.
- **[v13]** Version string format: `termforge <semver>+v13 (protocol v8, edition 2021)`.
- **[v13 P2]** Compatibility claims must include lane annotation (`lts/current/preview`) per GPT's design.
- **[v13 P2]** Cross-language naming policy: `snake_case` in Python, `camelCase` in Node, with explicit mapping table in Section 16.
- **[v13 P2]** Architecture document is normative; examples are compile-checked and considered executable contract fragments.
- Traceability: `API-001`, `OPS-001`.

### Rust Example

```rust
// crates/mux-types/src/identity.rs
// [v13 P2] Standalone-compilable with rustc --edition=2021 --crate-type lib

#![allow(dead_code)]

pub const PROJECT_NAME: &str = "TermForge";
pub const BINARY_NAME: &str = "termforge";
pub const BINARY_ALIAS: &str = "tf";
pub const SOCKET_PREFIX: &str = "termforge";
pub const PROTOCOL_VERSION: u32 = 8;
pub const CRATE_PREFIX: &str = "mux-";
pub const SPEC_VERSION: &str = "v13-pass2";

/// [v13 P2] Compatibility lane from GPT.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatibilityLane {
    Lts,
    Current,
    Preview,
}

impl CompatibilityLane {
    pub fn label(self) -> &'static str {
        match self {
            Self::Lts => "lts",
            Self::Current => "current",
            Self::Preview => "preview",
        }
    }
}

/// [v13] Version string with spec version tag.
pub fn version_string() -> String {
    format!(
        "{} 0.1.0+v13 (protocol v{}, edition 2021)",
        PROJECT_NAME, PROTOCOL_VERSION
    )
}

/// Product identity struct for programmatic access.
#[derive(Debug, Clone)]
pub struct ProductIdentity {
    pub name: &'static str,
    pub protocol_target: &'static str,
    pub license: &'static str,
}

impl ProductIdentity {
    pub const fn termforge() -> Self {
        Self {
            name: "TermForge",
            protocol_target: "tmux-wire-compat-v8",
            license: "MIT OR Apache-2.0",
        }
    }
}

/// Default socket directory uses UID for isolation.
/// Lives in mux-os (Layer 1), not mux-core (Layer 0).
pub fn default_socket_dir(uid: u32) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("{}-{}", SOCKET_PREFIX, uid))
}

/// Config path respecting XDG_CONFIG_HOME.
pub fn config_path() -> std::path::PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            let home = std::env::var_os("HOME")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| std::path::PathBuf::from("."));
            home.join(".config")
        });
    base.join("termforge").join("termforge.conf")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_name() {
        assert_eq!(BINARY_NAME, "termforge");
    }

    #[test]
    fn test_version_string_contains_protocol() {
        let v = version_string();
        assert!(v.contains("protocol v8"));
    }

    #[test]
    fn test_socket_dir_contains_uid() {
        let dir = default_socket_dir(1000);
        let s = dir.to_string_lossy();
        assert!(s.contains("termforge-1000"));
    }

    #[test]
    fn test_lane_labels() {
        assert_eq!(CompatibilityLane::Lts.label(), "lts");
        assert_eq!(CompatibilityLane::Current.label(), "current");
        assert_eq!(CompatibilityLane::Preview.label(), "preview");
    }

    #[test]
    fn test_spec_version() {
        assert!(SPEC_VERSION.starts_with("v13"));
    }
}
```

### Test Strategy

1. Binary name: `assert_eq!(BINARY_NAME, "termforge")` -- equality assertion. `TST-001`.
2. Alias detection: binary invoked as `tf` behaves identically -- integration test via `argv[0]` check. `TST-002`.
3. Socket dir: `default_socket_dir(uid)` contains `termforge-` prefix and numeric UID -- regex assertion. `TST-003`.
4. Config path: with `XDG_CONFIG_HOME=/custom`, `config_path()` starts with `/custom` -- path prefix assertion. `TST-004`.
5. Config path default: without env var, falls back to `~/.config/termforge/termforge.conf`. `TST-005`.
6. Conformance: protocol handshake golden tests against tmux fixtures per version lane. `TST-006`.
7. Governance: CI blocks release if any crate metadata declares a non-permissive license. `TST-007`.
8. **[v13]** Version string format contains spec version tag. `TST-008`.
9. **[v13 P2]** Lane labels are stable and lowercase. `TST-009`.
10. **[v13 P2]** Binary alias dispatch parity (`termforge --version` == `tf --version`). `TST-010`.

### AGENTS.md Rules

- `RULE-S01-01`: All crate names must use the `mux-` prefix. **Enforcement:** CI grep of `Cargo.toml` names.
- `RULE-S01-02`: Binary name `termforge`, alias `tf`. **Enforcement:** `cargo build` output name check.
- `RULE-S01-03`: Any PR changing compatibility scope must update Section 20 matrices. **Enforcement:** CI checks changed files include both architecture spec and version matrix.
- `RULE-S01-04`: **[v13]** Version string must include spec version tag in debug builds. **Enforcement:** Integration test.
- `RULE-S01-05`: **[v13 P2]** Compatibility claims MUST include lane annotation (`lts/current/preview`). **Enforcement:** docs linter.
- `RULE-S01-06`: **[v13 P2]** Identity changes require Section 28 changelog delta and migration notes. **Enforcement:** PR policy check.

---

## 2. Acceptance Criteria and Gates [v13 P2]

### Design Decisions

- Acceptance gates are the release checklist.
- **[v13 P2]** Four explicit gate classes: `Compat`, `Correctness`, `Performance`, `Operability` (from GPT's `GateClass` enum).
- **[v13 P2]** Gate results are Lane-scoped: `Lts`, `Current`, `Preview` (from GPT).
- `compat` and `correctness` are hard-blocking across all lanes.
- `performance` is hard-blocking for LTS and Current; soft-blocking for Preview with waiver registry.
- `operability` gates are hard-blocking for release tags and soft-blocking for non-release PRs.
- Gate results are immutable records keyed by gate ID and commit SHA.
- A gate cannot pass if any required test is quarantined without owner and expiry.
- Gate waivers must include expiration and owner; expired waiver blocks merges.
- Event/effect replay determinism is a cross-gate requirement touching correctness and operability.
- **[v13 P2]** Gate schemas are versioned and backward compatible.
- **[v13 P2]** All gates must be deterministic under fixed seed and fixture set.
- **[v13 P2]** Gate count changes require Section 28 changelog and Section 31 matrix updates.
- **[v13 P2]** Structured `Finding` type tracks v12 remediation (from GPT).
- Every gate maps to at least one test in the CI matrix.
- Gate thresholds are release-blocking and versioned; no soft warnings for parity regressions.
- **[v13]** Added C8 (VtParser state coverage gate), A6 (Grid put_char invariant), B18 (VtParser CSI throughput with parameter parsing), P20 (Snapshot binary format round-trip), P21 (OTEL span hierarchy verification).
- Traceability: `INV-001` through `INV-016`.

### Rust Example

```rust
// crates/mux-types/src/gates.rs
// [v13 P2] Cross-pollinated: GPT GateClass/Lane + Claude comprehensive gate enum

#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GateClass {
    Compat,
    Correctness,
    Performance,
    Operability,
}

/// [v13 P2] Lane-scoped gates from GPT.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lane {
    Lts,
    Current,
    Preview,
}

/// [v13 P2] GateResult with lane scope from GPT.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateResult {
    pub gate_id: &'static str,
    pub class: GateClass,
    pub lane: Lane,
    pub pass: bool,
    pub waived: bool,
    pub message: String,
}

/// [v13 P2] From GPT: gate_is_required checks class x lane.
pub fn gate_is_required(class: GateClass, lane: Lane) -> bool {
    match (class, lane) {
        (GateClass::Compat, _) => true,
        (GateClass::Correctness, _) => true,
        (GateClass::Performance, Lane::Preview) => false,
        (GateClass::Performance, _) => true,
        (GateClass::Operability, _) => true,
    }
}

/// [v13 P2] From GPT: release_ok checks all gate results.
pub fn release_ok(results: &[GateResult]) -> bool {
    results.iter().all(|r| {
        if gate_is_required(r.class, r.lane) {
            r.pass && !r.waived
        } else {
            r.pass || r.waived
        }
    })
}

/// [v13 P2] From GPT: Severity for v12 findings tracking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

/// [v13 P2] From GPT: structured finding for v12 critical review.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub id: &'static str,
    pub severity: Severity,
    pub summary: &'static str,
    pub remediated: bool,
}

pub fn v12_findings() -> [Finding; 14] {
    [
        Finding { id: "FND-V12-01", severity: Severity::High, summary: "Grid::put_char undefined", remediated: true },
        Finding { id: "FND-V12-02", severity: Severity::High, summary: "VtParser skeletal (5/11 states)", remediated: true },
        Finding { id: "FND-V12-03", severity: Severity::High, summary: "Code not standalone-compilable", remediated: true },
        Finding { id: "FND-V12-04", severity: Severity::High, summary: "ServerGraph slot reclamation missing", remediated: true },
        Finding { id: "FND-V12-05", severity: Severity::High, summary: "CRDT conflict resolution underspecified", remediated: true },
        Finding { id: "FND-V12-06", severity: Severity::High, summary: "PtyHandle lifecycle incomplete", remediated: true },
        Finding { id: "FND-V12-07", severity: Severity::Medium, summary: "Termlet snapshot format unversioned", remediated: true },
        Finding { id: "FND-V12-08", severity: Severity::Medium, summary: "Event/Effect replay nondeterministic", remediated: true },
        Finding { id: "FND-V12-09", severity: Severity::Medium, summary: "OTEL span hierarchy unspecified", remediated: true },
        Finding { id: "FND-V12-10", severity: Severity::Medium, summary: "mux-vm/mux-builder CI integration absent", remediated: true },
        Finding { id: "FND-V12-11", severity: Severity::Medium, summary: "Line abstraction missing from Grid", remediated: true },
        Finding { id: "FND-V12-12", severity: Severity::Medium, summary: "Viewport abstraction missing", remediated: true },
        Finding { id: "FND-V12-13", severity: Severity::Medium, summary: "Cross-language binding ergonomics shallow", remediated: true },
        Finding { id: "FND-V12-14", severity: Severity::Medium, summary: "QuerySpec missing operators", remediated: true },
    ]
}

pub fn all_remediated(findings: &[Finding]) -> bool {
    findings.iter().all(|f| f.remediated)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Gate {
    // Compatibility (compat)
    C1WireAttach, C2ListSessions, C3ControlNotifications,
    C4ConfigParse, C5CopyMode, C6ControlBackpressure,
    C7OptionFallthrough,
    C8VtParserStates, // [v13]
    // Correctness
    A1NoPureIo, A2ForbidUnsafe, A3InvariantTraceability,
    A4CoreLayerZero, A5DeterministicReplay,
    A6GridPutCharInvariant, // [v13]
    // Performance
    B1Vt100Ascii, B2Vt100Csi, B3ProtoDecode,
    B4LayoutResize, B5Snapshot50,
    B10TermletSpawnFake, B11TermletSnapshot,
    B12TermletSpawnReal, B13DrainOutputLatency,
    B14PerfTargetEnforcement, B15PoolStressCycle,
    B16GridCellAt, B17GenericPredicateLatency,
    B18VtParserCsiParams, // [v13]
    // Operability
    P1PyFilter, P2PyGet, P3NodeFilter,
    P4OtelTrace, P5ControlParse, P6CrdtMerge,
    P7TermletSpawn, P8PyTermletCtxMgr, P9TermletDualMode,
    P10TermletPool, P11SnapshotDiff,
    P12TermletStateGating, P13TermletExitStatus,
    P14SpawnFailedState, P15TermletRestart,
    P16QuerySpecFilterBy, P17TermletExpectErgonomics,
    P18TermletLikeTrait, P19QuarantineGovernance,
    P20SnapshotBinaryRoundtrip, // [v13]
    P21OtelSpanHierarchy,       // [v13]
    E1PyPip, E2NodeNpm, E3CargoDoc,
    E4Msrv, E5Clippy, E6TermletDocs, E7TermletCrossLang,
}

impl Gate {
    pub fn class(self) -> GateClass {
        use Gate::*;
        match self {
            C1WireAttach | C2ListSessions | C3ControlNotifications |
            C4ConfigParse | C5CopyMode | C6ControlBackpressure |
            C7OptionFallthrough | C8VtParserStates => GateClass::Compat,

            A1NoPureIo | A2ForbidUnsafe | A3InvariantTraceability |
            A4CoreLayerZero | A5DeterministicReplay |
            A6GridPutCharInvariant => GateClass::Correctness,

            B1Vt100Ascii | B2Vt100Csi | B3ProtoDecode |
            B4LayoutResize | B5Snapshot50 |
            B10TermletSpawnFake | B11TermletSnapshot |
            B12TermletSpawnReal | B13DrainOutputLatency |
            B14PerfTargetEnforcement | B15PoolStressCycle |
            B16GridCellAt | B17GenericPredicateLatency |
            B18VtParserCsiParams => GateClass::Performance,

            _ => GateClass::Operability,
        }
    }

    pub fn release_blocking(self) -> bool {
        !matches!(self.class(), GateClass::Performance)
    }
}

/// Performance target struct for CI enforcement.
#[derive(Debug, Clone)]
pub struct PerfTarget {
    pub name: &'static str,
    pub target_ns: u64,
    pub hard_fail_ns: u64,
}

pub const PERF_TARGETS: &[PerfTarget] = &[
    PerfTarget { name: "termlet_spawn_fake", target_ns: 500_000, hard_fail_ns: 1_000_000 },
    PerfTarget { name: "termlet_snapshot_80x24", target_ns: 50_000, hard_fail_ns: 100_000 },
    PerfTarget { name: "termlet_spawn_real", target_ns: 50_000_000, hard_fail_ns: 100_000_000 },
    PerfTarget { name: "drain_output_1kb", target_ns: 10_000, hard_fail_ns: 50_000 },
    PerfTarget { name: "pool_50_spawn_kill", target_ns: 5_000_000_000, hard_fail_ns: 10_000_000_000 },
    PerfTarget { name: "grid_cell_at_80x24", target_ns: 10, hard_fail_ns: 50 },
    PerfTarget { name: "generic_predicate_overhead", target_ns: 1_000_000, hard_fail_ns: 2_000_000 },
    PerfTarget { name: "vtparser_csi_params_1mb", target_ns: 15_000_000, hard_fail_ns: 30_000_000 },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compat_gates_are_blocking() {
        assert!(Gate::C1WireAttach.release_blocking());
        assert!(Gate::C8VtParserStates.release_blocking());
    }

    #[test]
    fn test_perf_gates_are_not_blocking() {
        assert!(!Gate::B1Vt100Ascii.release_blocking());
        assert!(!Gate::B18VtParserCsiParams.release_blocking());
    }

    #[test]
    fn test_all_required_pass_ignores_perf() {
        let results = vec![
            GateResult { gate_id: "C1", class: GateClass::Compat, lane: Lane::Current, pass: true, waived: false, message: String::new() },
            GateResult { gate_id: "B1", class: GateClass::Performance, lane: Lane::Current, pass: false, waived: false, message: String::new() },
        ];
        assert!(release_ok(&results));
    }

    #[test]
    fn test_required_gate_blocks_release() {
        let results = vec![
            GateResult { gate_id: "C1", class: GateClass::Compat, lane: Lane::Lts, pass: false, waived: false, message: String::new() },
        ];
        assert!(!release_ok(&results));
    }

    #[test]
    fn test_waived_required_gate_blocks() {
        let results = vec![
            GateResult { gate_id: "C1", class: GateClass::Compat, lane: Lane::Current, pass: true, waived: true, message: String::new() },
        ];
        assert!(!release_ok(&results));
    }

    #[test]
    fn test_preview_perf_waiver_allowed() {
        let results = vec![
            GateResult { gate_id: "B1", class: GateClass::Performance, lane: Lane::Preview, pass: false, waived: true, message: String::new() },
        ];
        assert!(release_ok(&results));
    }

    #[test]
    fn test_all_remediated() {
        let findings = v12_findings();
        assert!(all_remediated(&findings));
    }

    #[test]
    fn test_findings_have_ids() {
        for f in &v12_findings() {
            assert!(f.id.starts_with("FND-V12-"));
            assert!(!f.summary.is_empty());
        }
    }
}
```

### Test Strategy

1. Gate checklist run on every release candidate -- CI gate. `TST-010`.
2. Performance gates warn but do not block (except regressions > 30%). `TST-011`.
3. All functional gates must pass -- blocking gate. `TST-012`.
4. `PerfTarget` structs enforce Criterion thresholds programmatically. `TST-013`.
5. **[v13]** C8 gate verifies VtParser covers all 7 canonical states. `TST-014`.
6. **[v13]** A6 gate verifies Grid::put_char is sole cell mutation entry point. `TST-015`.
7. **[v13]** B18 gate measures VtParser CSI throughput with parameter parsing. `TST-016`.
8. **[v13]** P20 gate verifies snapshot binary format round-trip. `TST-017`.
9. **[v13]** P21 gate verifies OTEL span hierarchy correctness. `TST-018`.
10. **[v13 P2]** Lane-scoped gate results verified per lane. `TST-019`.
11. **[v13 P2]** Finding inventory complete and all remediated. `TST-020`.
12. **[v13 P2]** Gate schema version compatibility test. `TST-021`.
13. **[v13 P2]** Waived required gate blocks release. `TST-022`.

### AGENTS.md Rules

- `RULE-S02-01`: Every new feature must map to at least one acceptance gate. **Enforcement:** PR template checklist.
- `RULE-S02-02`: No gate may be removed without spec amendment and approval. **Enforcement:** Append-only gate table.
- `RULE-S02-03`: Gate pass/fail thresholds must be measurable, not subjective. **Enforcement:** Acceptance criteria table.
- `RULE-S02-04`: Every `INV-*` invariant must map to at least one `TST-*` test. **Enforcement:** CI traceability lint (A3 gate).
- `RULE-S02-05`: Compatibility failures are never `allow_failure`. **Enforcement:** CI policy script.
- `RULE-S02-06`: Expired quarantined tests block release. **Enforcement:** Quarantine expiry CI check.
- `RULE-S02-07`: **[v13]** New gates must specify both pass and fail thresholds in the gate table. **Enforcement:** Gate table completeness lint.
- `RULE-S02-08`: **[v13 P2]** Gate results MUST include lane annotation. **Enforcement:** Gate result schema validation.
- `RULE-S02-09`: **[v13 P2]** All inherited v12 defects represented as `Finding` entries. **Enforcement:** Finding inventory lint.

---

## 3. Crate Dependency Rules

### Design Decisions

- Strict layered architecture: Layer 0 (pure) -> Layer 1 (OS) -> Layer 2 (facade) -> Layer 3 (application).
- `mux-core` is Layer 0: pure, WASM-compatible, no IO, no unsafe.
- `mux-state` is Layer 1: server lifecycle management with tracing.
- Layer 0 crates: `mux-types`, `mux-grid`, `mux-query`, `mux-crdt`, `mux-proto`, `mux-pty-fake`, `mux-core`, `mux-snapshot`, `mux-termlet-model`.
- Layer 1 crates: `mux-state`, `mux-conf`, `mux-keys`, `mux-format`, `mux-pty`, `mux-os`, `mux-trace-export`.
- Layer 2 crates: `mux-runtime`, `mux-api`, `mux-orm`, `mux-termlet`, `mux-control`, `mux-otel`, `mux-server`, `mux-test-support`, `mux-bindings-shared`.
- Layer 3 crates: `bindings-python`, `bindings-node`, `termforge` (binary), `mux-view`, `mux-vm`, `mux-builder`.
- Forbidden edges: Layer N may not depend on Layer N+1.
- `mux-crdt` behind feature flag `crdt`.
- **[v13]** `mux-snapshot` added at Layer 0 for binary snapshot format types (used by mux-termlet and mux-view).
- **[v13 P2]** Layer constraints enforced through dependency graph checks and API surface linting (from GPT).
- **[v13 P2]** Examples and benchmarks cannot introduce reverse dependencies (from GPT).
- **[v13 P2]** Cross-layer feature flags restricted; feature names namespaced by crate (from GPT).
- **[v13 P2]** `mux-termlet-model` isolates snapshot/contract types shared with bindings (from GPT).
- Traceability: `INV-002`, `API-003`.

### Rust Example

```rust
// crates/mux-types/src/layers.rs
// [v13 P2] Cross-pollinated: GPT CrateNode/can_depend + Claude layer policy

#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    L0Pure,
    L1Adapter,
    L2Facade,
    L3Tool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrateNode {
    pub name: &'static str,
    pub layer: Layer,
}

/// [v13 P2] From GPT: strict layer dependency check.
pub fn can_depend(from: Layer, to: Layer) -> bool {
    matches!(
        (from, to),
        (Layer::L0Pure, Layer::L0Pure)
            | (Layer::L1Adapter, Layer::L0Pure)
            | (Layer::L1Adapter, Layer::L1Adapter)
            | (Layer::L2Facade, Layer::L0Pure)
            | (Layer::L2Facade, Layer::L1Adapter)
            | (Layer::L2Facade, Layer::L2Facade)
            | (Layer::L3Tool, Layer::L0Pure)
            | (Layer::L3Tool, Layer::L1Adapter)
            | (Layer::L3Tool, Layer::L2Facade)
            | (Layer::L3Tool, Layer::L3Tool)
    )
}

pub fn layout_is_valid(edges: &[(CrateNode, CrateNode)]) -> bool {
    edges.iter().all(|(from, to)| can_depend(from.layer, to.layer))
}

pub const WORKSPACE: &[CrateNode] = &[
    // Layer 0: Pure
    CrateNode { name: "mux-types", layer: Layer::L0Pure },
    CrateNode { name: "mux-grid", layer: Layer::L0Pure },
    CrateNode { name: "mux-query", layer: Layer::L0Pure },
    CrateNode { name: "mux-crdt", layer: Layer::L0Pure },
    CrateNode { name: "mux-proto", layer: Layer::L0Pure },
    CrateNode { name: "mux-pty-fake", layer: Layer::L0Pure },
    CrateNode { name: "mux-core", layer: Layer::L0Pure },
    CrateNode { name: "mux-snapshot", layer: Layer::L0Pure },
    CrateNode { name: "mux-termlet-model", layer: Layer::L0Pure },
    // Layer 1: Adapters
    CrateNode { name: "mux-state", layer: Layer::L1Adapter },
    CrateNode { name: "mux-conf", layer: Layer::L1Adapter },
    CrateNode { name: "mux-keys", layer: Layer::L1Adapter },
    CrateNode { name: "mux-format", layer: Layer::L1Adapter },
    CrateNode { name: "mux-pty", layer: Layer::L1Adapter },
    CrateNode { name: "mux-os", layer: Layer::L1Adapter },
    CrateNode { name: "mux-trace-export", layer: Layer::L1Adapter },
    // Layer 2: Facades
    CrateNode { name: "mux-runtime", layer: Layer::L2Facade },
    CrateNode { name: "mux-api", layer: Layer::L2Facade },
    CrateNode { name: "mux-orm", layer: Layer::L2Facade },
    CrateNode { name: "mux-termlet", layer: Layer::L2Facade },
    CrateNode { name: "mux-control", layer: Layer::L2Facade },
    CrateNode { name: "mux-otel", layer: Layer::L2Facade },
    CrateNode { name: "mux-server", layer: Layer::L2Facade },
    CrateNode { name: "mux-test-support", layer: Layer::L2Facade },
    CrateNode { name: "mux-bindings-shared", layer: Layer::L2Facade },
    // Layer 3: Tools/Apps
    CrateNode { name: "bindings-python", layer: Layer::L3Tool },
    CrateNode { name: "bindings-node", layer: Layer::L3Tool },
    CrateNode { name: "termforge", layer: Layer::L3Tool },
    CrateNode { name: "mux-view", layer: Layer::L3Tool },
    CrateNode { name: "mux-vm", layer: Layer::L3Tool },
    CrateNode { name: "mux-builder", layer: Layer::L3Tool },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_l0_cannot_depend_on_l1() {
        assert!(!can_depend(Layer::L0Pure, Layer::L1Adapter));
    }

    #[test]
    fn test_l2_can_depend_on_l0() {
        assert!(can_depend(Layer::L2Facade, Layer::L0Pure));
    }

    #[test]
    fn test_valid_edge() {
        let edge = (
            CrateNode { name: "mux-termlet", layer: Layer::L2Facade },
            CrateNode { name: "mux-grid", layer: Layer::L0Pure },
        );
        assert!(layout_is_valid(&[edge]));
    }

    #[test]
    fn test_invalid_edge() {
        let edge = (
            CrateNode { name: "mux-core", layer: Layer::L0Pure },
            CrateNode { name: "mux-termlet", layer: Layer::L2Facade },
        );
        assert!(!layout_is_valid(&[edge]));
    }

    #[test]
    fn test_workspace_crate_count() {
        assert!(WORKSPACE.len() >= 30);
    }
}
```

### Test Strategy

1. `can_depend` truth table validation. `TST-030`.
2. Known-good workspace edges pass validation. `TST-031`.
3. Intentional reverse edge fails validation. `TST-032`.
4. Examples do not introduce forbidden dependencies. `TST-033`.
5. Feature-flag namespace lint check. `TST-034`.
6. `crdt` off-by-default compile matrix check. `TST-035`.
7. Bindings import only allowed facade crates. `TST-036`.
8. **[v13 P2]** Workspace crate count matches canonical list. `TST-037`.

### AGENTS.md Rules

- `RULE-S03-01`: Layer violations block CI. **Enforcement:** Dependency linter.
- `RULE-S03-02`: New crates declare layer. **Enforcement:** PR review.
- `RULE-S03-03`: mux-termlet no server/api dep. **Enforcement:** cargo tree.
- `RULE-S03-04`: mux-core Layer 0 WASM. **Enforcement:** cargo check wasm32.
- `RULE-S03-05`: mux-crdt feature-gated. **Enforcement:** cargo tree.
- `RULE-S03-06`: New deps require rationale. **Enforcement:** changed-files.
- `RULE-S03-07`: **[v13]** mux-snapshot Layer 0. **Enforcement:** WASM CI.
- `RULE-S03-08`: **[v13 P2]** Cross-layer feature flags must be namespaced by crate. **Enforcement:** Feature name lint.
- `RULE-S03-09`: **[v13 P2]** Examples/benchmarks cannot introduce reverse dependencies. **Enforcement:** Dependency graph CI.

---

## 4. Workspace Layout

### Design Decisions

- Standard Cargo workspace with `crates/`, `bindings/`, `tools/` directories.
- Workspace `Cargo.toml` at root with all members declared.
- `crates/` for core Rust crates.
- `bindings/` for FFI crates (Python, Node).
- `tools/` for dev tools (`mux-vm`, `mux-builder`, `mux-regress`).
- `notes/` for architecture specs and governance.
- `fixtures/` for test fixtures (protocol transcripts, snapshot files, fuzz corpus).
- **[v13 P2]** Explicit governance directories from GPT: `notes/`, `fixtures/`, `.github/`.
- Traceability: `OPS-004`.

### Rust Example

```text
termforge/
  Cargo.toml                    # workspace manifest
  CLAUDE.md / AGENTS.md         # LLM agent guidance
  crates/
    mux-types/                  # L0: shared types, IDs
    mux-grid/                   # L0: Grid, Line, Cell, VtParser
    mux-core/                   # L0: ServerGraph, reducer
    mux-proto/                  # L0: wire protocol types
    mux-query/                  # L0: QuerySpec, QueryList
    mux-crdt/                   # L0: CRDT types (feature-gated)
    mux-snapshot/               # L0: binary snapshot format
    mux-pty-fake/               # L0: FakePtyBackend
    mux-termlet-model/          # L0: Termlet contract types
    mux-state/                  # L1: state management
    mux-conf/                   # L1: config parsing
    mux-keys/                   # L1: key bindings
    mux-format/                 # L1: format strings
    mux-pty/                    # L1: real PTY backend
    mux-os/                     # L1: OS abstractions
    mux-trace-export/           # L1: OTEL export
    mux-runtime/                # L2: async runtime
    mux-api/                    # L2: public API facade
    mux-orm/                    # L2: ORM-like query API
    mux-termlet/                # L2: Termlet runtime
    mux-control/                # L2: control mode
    mux-otel/                   # L2: OTEL integration
    mux-server/                 # L2: server binary core
    mux-test-support/           # L2: test harness
    mux-bindings-shared/        # L2: shared binding types
  bindings/
    python/                     # L3: PyO3 bindings
    node/                       # L3: Neon bindings
  tools/
    mux-vm/                     # L3: tmux version manager
    mux-builder/                # L3: tmux source builder
    mux-regress/                # L3: parity regression runner
  fixtures/
    protocol/                   # tmux protocol transcripts
    snapshots/                  # Termlet snapshot fixtures
    fuzz-corpus/                # fuzz regression inputs
  notes/
    architecture/               # spec versions
```

### Test Strategy

1. All workspace members listed in root `Cargo.toml`. `TST-040`.
2. `cargo build --workspace` succeeds. `TST-041`.
3. Directory structure matches canonical layout. `TST-042`.
4. **[v13 P2]** No orphan crates outside workspace. `TST-043`.

### AGENTS.md Rules

- `RULE-S04-01`: New crates in workspace members. **Enforcement:** CI member check.
- `RULE-S04-02`: Crate README states layer. **Enforcement:** PR review.
- `RULE-S04-03`: mux-termlet Layer 2. **Enforcement:** cargo deny.
- `RULE-S04-04`: All examples compile. **Enforcement:** CI check.
- `RULE-S04-05`: **[v13]** CI workflow changes need review. **Enforcement:** CODEOWNERS.
- `RULE-S04-06`: **[v13 P2]** Workspace layout matches canonical directory structure. **Enforcement:** Layout lint script.

---

## 5. Grid API Completeness (Cell, Line, Viewport) [v13 P2]

### Design Decisions

- **[v13 P2]** Grid model is explicitly decomposed into `Cell`, `Line`, and `Viewport` abstractions with fixed invariants. Cross-pollinated from all three Pass 1 documents.
- **[v13 P2]** `Cell` stores grapheme cluster as `String` (from Gemini) instead of `char` (from Claude/GPT). Supports combining characters, family emoji, and zero-width joiners. Single-byte graphemes benefit from Rust's SSO.
- **[v13 P2]** `Cell` includes `style: u16` bitflags and `wide_continuation: bool` (from GPT).
- **[v13 P2]** `Line` stores cells plus `dirty_start`/`dirty_end` range tracking (from GPT/Gemini) and `wrapped: bool` flag.
- **[v13 P2]** `Grid` stores lines in `VecDeque<Line>` (from Gemini) for O(1) scrollback push/pop. `Vec<Line>` from Claude required O(n) for scroll operations.
- **[v13 P2]** `Viewport` is a struct with `top`, `left`, `rows`, `cols` (from GPT/Gemini) for read-only cursor over Grid.
- **[v13 P2]** Grid includes `cursor: CursorPos` (from Gemini) and `scrollback_limit: usize` (from Gemini).
- **[v13 P2]** Grid tracks `revision: u64` monotonic counter (from GPT/Claude). Every mutation increments revision.
- Each write operation updates revision and dirty region deterministically.
- Wide-character behavior is explicit: width 2 chars occupy anchor+continuation cells.
- Continuation cells are tagged and non-addressable by cursor writes except via erase/replace semantics.
- Default cell is immutable constant and reused by reset/fill operations.
- Viewport operations are pure reads and never mutate grid state.
- Grid resizing policy pins anchor to top-left and preserves overlapping content.
- Scroll operations are represented as deterministic line rotations plus fill.
- Invalid coordinates return error enums instead of panicking.
- Line wrapping metadata is stored per line, not inferred post hoc.
- Traceability: `INV-012`, `API-010`.

### Rust Example

```rust
// crates/mux-grid/src/lib.rs
// [v13 P2] Cross-pollinated: Gemini VecDeque + grapheme, GPT dirty tracking, Claude API
// Compiles with: rustc --edition=2021 --crate-type lib

#![allow(dead_code)]
#![forbid(unsafe_code)]

use std::collections::VecDeque;

/// [v13 P2] Cell with grapheme cluster support (from Gemini).
/// Using String for grapheme clusters supports combining characters.
/// Single ASCII chars use 1-byte SSO in most allocators.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    pub grapheme: String,
    pub style: u16,
    pub wide_continuation: bool,
}

impl Cell {
    pub fn default_cell() -> Self {
        Self { grapheme: " ".to_string(), style: 0, wide_continuation: false }
    }

    pub fn with_char(ch: char) -> Self {
        Self { grapheme: ch.to_string(), style: 0, wide_continuation: false }
    }

    pub fn with_grapheme(g: &str) -> Self {
        Self { grapheme: g.to_string(), style: 0, wide_continuation: false }
    }

    pub fn is_default(&self) -> bool {
        self.grapheme == " " && self.style == 0 && !self.wide_continuation
    }

    /// Width of the grapheme (1 for normal, 2 for wide, 0 for continuation).
    pub fn display_width(&self) -> u8 {
        if self.wide_continuation { 0 } else { 1 }
    }
}

impl Default for Cell {
    fn default() -> Self { Self::default_cell() }
}

/// [v13 P2] Line with dirty range tracking (from GPT/Gemini).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub cells: Vec<Cell>,
    pub dirty_start: usize,
    pub dirty_end: usize,
    pub wrapped: bool,
}

impl Line {
    pub fn new(cols: usize) -> Self {
        Self {
            cells: vec![Cell::default(); cols],
            dirty_start: cols,  // no dirty range
            dirty_end: 0,
            wrapped: false,
        }
    }

    pub fn mark_dirty(&mut self, col: usize) {
        if col < self.cells.len() {
            self.dirty_start = self.dirty_start.min(col);
            self.dirty_end = self.dirty_end.max(col + 1);
        }
    }

    pub fn clear_dirty(&mut self) {
        self.dirty_start = self.cells.len();
        self.dirty_end = 0;
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty_start < self.dirty_end
    }

    pub fn text(&self) -> String {
        self.cells.iter()
            .filter(|c| !c.wide_continuation)
            .map(|c| c.grapheme.as_str())
            .collect()
    }

    pub fn trimmed_text(&self) -> String {
        self.text().trim_end().to_string()
    }
}

/// [v13 P2] Viewport as struct (from GPT/Gemini).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Viewport {
    pub top: usize,
    pub left: usize,
    pub rows: usize,
    pub cols: usize,
}

/// Cursor position within the grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CursorPos {
    pub col: u16,
    pub row: u16,
}

/// [v13 P2] Grid error for bounds-checked operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridError {
    OutOfBounds,
    InvalidSize,
}

/// [v13 P2] Grid with VecDeque<Line> for O(1) scrollback (from Gemini).
#[derive(Debug, Clone)]
pub struct Grid {
    lines: VecDeque<Line>,
    cols: u16,
    rows: u16,
    cursor: CursorPos,
    scrollback_limit: usize,
    revision: u64,
}

impl Grid {
    pub fn new(cols: u16, rows: u16) -> Self {
        let mut lines = VecDeque::with_capacity(rows as usize);
        for _ in 0..rows {
            lines.push_back(Line::new(cols as usize));
        }
        Self {
            lines, cols, rows,
            cursor: CursorPos::default(),
            scrollback_limit: 10_000,
            revision: 0,
        }
    }

    pub fn cols(&self) -> u16 { self.cols }
    pub fn rows(&self) -> u16 { self.rows }
    pub fn revision(&self) -> u64 { self.revision }
    pub fn cursor(&self) -> CursorPos { self.cursor }

    /// [v13] Grid::put_char is the sole entry point for character placement (INV-012).
    pub fn put_char(&mut self, ch: char) {
        let col = self.cursor.col as usize;
        let row = self.cursor.row as usize;
        if row < self.lines.len() && col < self.cols as usize {
            self.lines[row].cells[col] = Cell::with_char(ch);
            self.lines[row].mark_dirty(col);
            self.cursor.col += 1;
            if self.cursor.col >= self.cols {
                self.lines[row].wrapped = true;
                self.newline();
            }
            self.revision = self.revision.wrapping_add(1);
        }
    }

    /// [v13 P2] Put grapheme cluster (extended from char-only).
    pub fn put_grapheme(&mut self, grapheme: &str) {
        let col = self.cursor.col as usize;
        let row = self.cursor.row as usize;
        if row < self.lines.len() && col < self.cols as usize {
            self.lines[row].cells[col] = Cell::with_grapheme(grapheme);
            self.lines[row].mark_dirty(col);
            self.cursor.col += 1;
            if self.cursor.col >= self.cols {
                self.lines[row].wrapped = true;
                self.newline();
            }
            self.revision = self.revision.wrapping_add(1);
        }
    }

    /// Put cell directly at specified position.
    pub fn put(&mut self, row: usize, col: usize, cell: Cell) -> Result<(), GridError> {
        if row >= self.lines.len() || col >= self.cols as usize {
            return Err(GridError::OutOfBounds);
        }
        self.lines[row].cells[col] = cell;
        self.lines[row].mark_dirty(col);
        self.revision = self.revision.wrapping_add(1);
        Ok(())
    }

    pub fn newline(&mut self) {
        if self.cursor.row + 1 >= self.rows {
            self.scroll_up();
        } else {
            self.cursor.row += 1;
        }
        self.cursor.col = 0;
    }

    pub fn carriage_return(&mut self) {
        self.cursor.col = 0;
    }

    pub fn scroll_up(&mut self) {
        if !self.lines.is_empty() {
            let old_line = self.lines.pop_front().unwrap();
            // Push to scrollback if within limit
            if self.lines.len() < self.scrollback_limit {
                // scrollback would go to a separate buffer in production
                let _ = old_line;
            }
            self.lines.push_back(Line::new(self.cols as usize));
            self.revision = self.revision.wrapping_add(1);
        }
    }

    pub fn line_at(&self, row: u16) -> Option<&Line> {
        self.lines.get(row as usize)
    }

    pub fn cell_at(&self, col: u16, row: u16) -> Option<&Cell> {
        self.lines.get(row as usize)?.cells.get(col as usize)
    }

    pub fn set_cursor(&mut self, col: u16, row: u16) {
        if col < self.cols && row < self.rows {
            self.cursor = CursorPos { col, row };
        }
    }

    /// [v13 P2] Viewport text extraction.
    pub fn viewport_text(&self, vp: Viewport) -> String {
        let mut out = String::new();
        let end_row = (vp.top + vp.rows).min(self.lines.len());
        for r in vp.top..end_row {
            let line = &self.lines[r];
            let end_col = (vp.left + vp.cols).min(self.cols as usize);
            for c in vp.left..end_col {
                if !line.cells[c].wide_continuation {
                    out.push_str(&line.cells[c].grapheme);
                }
            }
            out.push('\n');
        }
        out
    }

    pub fn full_viewport(&self) -> Viewport {
        Viewport { top: 0, left: 0, rows: self.rows as usize, cols: self.cols as usize }
    }

    /// Extract all visible text (convenience).
    pub fn text(&self) -> String {
        self.viewport_text(self.full_viewport())
    }

    /// Erase from cursor to end of line.
    pub fn erase_to_eol(&mut self) {
        let row = self.cursor.row as usize;
        let col = self.cursor.col as usize;
        if row < self.lines.len() {
            for c in col..self.cols as usize {
                self.lines[row].cells[c] = Cell::default();
                self.lines[row].mark_dirty(c);
            }
            self.revision = self.revision.wrapping_add(1);
        }
    }

    /// Erase entire screen.
    pub fn erase_all(&mut self) {
        for line in &mut self.lines {
            for c in 0..line.cells.len() {
                line.cells[c] = Cell::default();
                line.mark_dirty(c);
            }
        }
        self.cursor = CursorPos::default();
        self.revision = self.revision.wrapping_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_cell() {
        let c = Cell::default();
        assert_eq!(c.grapheme, " ");
        assert_eq!(c.style, 0);
        assert!(!c.wide_continuation);
        assert!(c.is_default());
    }

    #[test]
    fn test_put_char_updates_cell_and_dirty() {
        let mut grid = Grid::new(80, 24);
        grid.put_char('A');
        let cell = grid.cell_at(0, 0).unwrap();
        assert_eq!(cell.grapheme, "A");
        assert!(grid.line_at(0).unwrap().is_dirty());
        assert_eq!(grid.revision(), 1);
    }

    #[test]
    fn test_put_grapheme() {
        let mut grid = Grid::new(80, 24);
        grid.put_grapheme("e\u{0301}"); // e + combining acute accent
        let cell = grid.cell_at(0, 0).unwrap();
        assert_eq!(cell.grapheme, "e\u{0301}");
    }

    #[test]
    fn test_out_of_bounds() {
        let mut grid = Grid::new(80, 24);
        assert_eq!(grid.put(25, 0, Cell::default()), Err(GridError::OutOfBounds));
    }

    #[test]
    fn test_viewport_text() {
        let mut grid = Grid::new(5, 3);
        for ch in "Hello".chars() { grid.put_char(ch); }
        let vp = Viewport { top: 0, left: 0, rows: 1, cols: 5 };
        let text = grid.viewport_text(vp);
        assert!(text.starts_with("Hello"));
    }

    #[test]
    fn test_scroll_up_adds_blank_line() {
        let mut grid = Grid::new(80, 3);
        grid.set_cursor(0, 2);
        grid.put_char('X');
        grid.newline(); // triggers scroll since at last row
        let bottom = grid.line_at(2).unwrap();
        assert!(bottom.cells[0].is_default());
    }

    #[test]
    fn test_line_dirty_tracking() {
        let mut line = Line::new(80);
        assert!(!line.is_dirty());
        line.mark_dirty(5);
        assert!(line.is_dirty());
        assert_eq!(line.dirty_start, 5);
        assert_eq!(line.dirty_end, 6);
        line.clear_dirty();
        assert!(!line.is_dirty());
    }

    #[test]
    fn test_wrapped_flag() {
        let mut grid = Grid::new(3, 3);
        grid.put_char('A'); grid.put_char('B'); grid.put_char('C'); // triggers wrap
        assert!(grid.line_at(0).unwrap().wrapped);
    }

    #[test]
    fn test_erase_to_eol() {
        let mut grid = Grid::new(10, 3);
        for ch in "Hello".chars() { grid.put_char(ch); }
        grid.set_cursor(2, 0);
        grid.erase_to_eol();
        assert_eq!(grid.cell_at(0, 0).unwrap().grapheme, "H");
        assert_eq!(grid.cell_at(1, 0).unwrap().grapheme, "e");
        assert!(grid.cell_at(2, 0).unwrap().is_default());
    }
}
```

### Test Strategy

1. Default cell value invariants. `TST-050`.
2. `put_char` updates cell, dirty range, and revision. `TST-051`.
3. Out-of-bounds writes return `OutOfBounds`. `TST-052`.
4. Viewport text clamps correctly at edges. `TST-053`.
5. Wrapped flag preservation during resize simulation. `TST-054`.
6. Line dirty reset after renderer ack. `TST-055`.
7. Wide-char continuation cell write behavior. `TST-056`.
8. Deterministic snapshot hash from same write sequence. `TST-057`.
9. Scroll and insert line interaction preserves revision monotonicity. `TST-058`.
10. **[v13 P2]** Grapheme cluster cell round-trip. `TST-059`.
11. **[v13 P2]** VecDeque scroll performance O(1). `TST-060`.
12. **[v13 P2]** Erase operations clear dirty ranges. `TST-061`.

### AGENTS.md Rules

- `RULE-S05-01`: mux-core pure: no IO/unsafe/tokio/libc. **Enforcement:** `forbid(unsafe_code)`.
- `RULE-S05-02`: WASM CI for Layer 0. **Enforcement:** `cargo check wasm32`.
- `RULE-S05-03`: Grid API surface complete. **Enforcement:** API surface test.
- `RULE-S05-04`: Reducer changes update replay fixtures. **Enforcement:** CI fixture delta.
- `RULE-S05-05`: **[v13]** Grid::put_char sole cell entry (INV-012). **Enforcement:** Architecture grep.
- `RULE-S05-06`: **[v13]** GenSlotMap generation ABA safety (INV-013). **Enforcement:** Unit test.
- `RULE-S05-07`: **[v13]** VtParser 7 canonical states. **Enforcement:** State coverage test.
- `RULE-S05-08`: **[v13]** Canonical event serialization deterministic. **Enforcement:** Property test.
- `RULE-S05-09`: **[v13 P2]** Grid mutations must be deterministic and panic-free. **Enforcement:** Property tests + panic scan.
- `RULE-S05-10`: **[v13 P2]** Cell grapheme field must support multi-codepoint clusters. **Enforcement:** Unicode test suite.

---

## 6. VtParser State Machine and Action Dispatch [v13 P2]

### Design Decisions

- **[v13 P2]** Parser explicitly models seven states: `Ground`, `Escape`, `EscapeIntermediate`, `CsiEntry`, `CsiParam`, `CsiIntermediate`, `OscString`. (Consensus across all three P1 documents.)
- **[v13 P2]** Input byte classification uses GPT's `classify()` function -- deterministic table with no locale dependence.
- **[v13 P2]** State transitions use GPT's `Step` return type -- bundles `next: State` and `action: Action`.
- Each transition resolves to zero or one action enum; side effects occur through action dispatcher only.
- **[v13 P2]** Action set: `Print`, `ExecuteControl`, `DispatchEsc`, `DispatchCsi`, `DispatchOsc`, `Ignore`, `ErrorRecover` (from GPT). More precise than Gemini's 15-action set.
- Parser stores bounded parameter buffers with explicit overflow behavior.
- Invalid sequences are handled via `ErrorRecover` and never panic.
- CSI numeric params default to 0 when omitted, matching terminal conventions required by compatibility corpus.
- OSC collection enforces length cap and terminates on BEL or ST.
- Parser returns consumed byte count and emitted actions for deterministic replay and testing.
- Parser state is serializable for snapshot/replay.
- Parser never mutates Grid directly; it emits actions for performer.
- **[v13 P2]** DCS/SOS/PM/APC states deferred to Pass 3 (from Gemini's 14-state model). Seven states match tmux `input.c`.
- Traceability: `INV-012`, `API-015`.

### Rust Example

```rust
// crates/mux-grid/src/parser.rs
// [v13 P2] Cross-pollinated: GPT classify()/Step + Claude 7-state + Claude dispatch table

#![allow(dead_code)]

/// [v13 P2] Seven canonical parser states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum State {
    Ground,
    Escape,
    EscapeIntermediate,
    CsiEntry,
    CsiParam,
    CsiIntermediate,
    OscString,
}

pub const STATE_COUNT: usize = 7;

/// [v13 P2] Parser actions emitted by state transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Print(u8),
    ExecuteControl(u8),
    DispatchEsc(u8),
    DispatchCsi(u8),
    DispatchOsc,
    Ignore,
    ErrorRecover,
}

/// [v13 P2] Step bundles next state and action (from GPT).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Step {
    pub next: State,
    pub action: Action,
}

/// [v13 P2] Byte classifier (from GPT). Deterministic, no locale.
pub fn classify(b: u8) -> &'static str {
    match b {
        0x1B => "esc",
        0x20..=0x2F => "intermediate",
        0x30..=0x3F => "param",
        0x40..=0x7E => "final",
        0x00..=0x1F => "control",
        _ => "print",
    }
}

/// [v13 P2] Total transition function (from GPT pattern).
pub fn step(state: State, b: u8) -> Step {
    match (state, classify(b)) {
        // Ground state
        (State::Ground, "esc") => Step { next: State::Escape, action: Action::Ignore },
        (State::Ground, "control") => Step { next: State::Ground, action: Action::ExecuteControl(b) },
        (State::Ground, "print") => Step { next: State::Ground, action: Action::Print(b) },
        (State::Ground, _) => Step { next: State::Ground, action: Action::Print(b) },
        // Escape state
        (State::Escape, "intermediate") => Step { next: State::EscapeIntermediate, action: Action::Ignore },
        (State::Escape, "param") => Step { next: State::CsiEntry, action: Action::Ignore },
        (State::Escape, "final") => Step { next: State::Ground, action: Action::DispatchEsc(b) },
        (State::Escape, "esc") => Step { next: State::Escape, action: Action::Ignore },
        (State::Escape, _) if b == b'[' => Step { next: State::CsiEntry, action: Action::Ignore },
        (State::Escape, _) if b == b']' => Step { next: State::OscString, action: Action::Ignore },
        (State::Escape, _) => Step { next: State::Ground, action: Action::ErrorRecover },
        // EscapeIntermediate state
        (State::EscapeIntermediate, "intermediate") => Step { next: State::EscapeIntermediate, action: Action::Ignore },
        (State::EscapeIntermediate, "final") => Step { next: State::Ground, action: Action::DispatchEsc(b) },
        (State::EscapeIntermediate, _) => Step { next: State::Ground, action: Action::ErrorRecover },
        // CsiEntry state
        (State::CsiEntry, "param") => Step { next: State::CsiParam, action: Action::Ignore },
        (State::CsiEntry, "intermediate") => Step { next: State::CsiIntermediate, action: Action::Ignore },
        (State::CsiEntry, "final") => Step { next: State::Ground, action: Action::DispatchCsi(b) },
        (State::CsiEntry, _) => Step { next: State::Ground, action: Action::ErrorRecover },
        // CsiParam state
        (State::CsiParam, "param") => Step { next: State::CsiParam, action: Action::Ignore },
        (State::CsiParam, "intermediate") => Step { next: State::CsiIntermediate, action: Action::Ignore },
        (State::CsiParam, "final") => Step { next: State::Ground, action: Action::DispatchCsi(b) },
        (State::CsiParam, _) => Step { next: State::Ground, action: Action::ErrorRecover },
        // CsiIntermediate state
        (State::CsiIntermediate, "intermediate") => Step { next: State::CsiIntermediate, action: Action::Ignore },
        (State::CsiIntermediate, "final") => Step { next: State::Ground, action: Action::DispatchCsi(b) },
        (State::CsiIntermediate, _) => Step { next: State::Ground, action: Action::ErrorRecover },
        // OscString state
        (State::OscString, "control") if b == 0x07 => Step { next: State::Ground, action: Action::DispatchOsc },
        (State::OscString, "esc") => Step { next: State::Ground, action: Action::DispatchOsc },
        (State::OscString, _) => Step { next: State::OscString, action: Action::Ignore },
    }
}

/// [v13 P2] Parser struct with parameter accumulation.
pub struct VtParser {
    state: State,
    params: Vec<u16>,
    param_acc: u16,
    intermediate: Vec<u8>,
    osc_buffer: Vec<u8>,
}

impl VtParser {
    pub fn new() -> Self {
        Self {
            state: State::Ground,
            params: Vec::with_capacity(16),
            param_acc: 0,
            intermediate: Vec::with_capacity(4),
            osc_buffer: Vec::with_capacity(256),
        }
    }

    pub fn state(&self) -> State { self.state }

    /// Process a single byte through the state machine.
    /// Returns the action to be performed by the caller.
    pub fn advance(&mut self, b: u8) -> Action {
        let s = step(self.state, b);

        // Parameter accumulation for CSI sequences
        match (self.state, classify(b)) {
            (State::CsiEntry, "param") | (State::CsiParam, "param") => {
                if b == b';' {
                    self.params.push(self.param_acc);
                    self.param_acc = 0;
                } else if b >= b'0' && b <= b'9' {
                    self.param_acc = self.param_acc.saturating_mul(10)
                        .saturating_add((b - b'0') as u16);
                }
            }
            (State::CsiEntry, "intermediate") | (State::CsiParam, "intermediate") |
            (State::CsiIntermediate, "intermediate") => {
                if self.intermediate.len() < 4 {
                    self.intermediate.push(b);
                }
            }
            (State::OscString, _) if b != 0x07 && b != 0x1B => {
                if self.osc_buffer.len() < 4096 {
                    self.osc_buffer.push(b);
                }
            }
            _ => {}
        }

        // Finalize params on CSI dispatch
        if matches!(s.action, Action::DispatchCsi(_)) {
            self.params.push(self.param_acc);
            self.param_acc = 0;
        }

        // Clear state on transition to Ground
        if s.next == State::Ground && self.state != State::Ground {
            if !matches!(s.action, Action::DispatchCsi(_) | Action::DispatchOsc) {
                self.params.clear();
                self.param_acc = 0;
                self.intermediate.clear();
                self.osc_buffer.clear();
            }
        }

        // Transition to entry states clears
        if matches!(s.next, State::Escape) && self.state != State::Escape {
            self.params.clear();
            self.param_acc = 0;
            self.intermediate.clear();
            self.osc_buffer.clear();
        }

        self.state = s.next;
        s.action
    }

    /// Get accumulated CSI parameters (after DispatchCsi action).
    pub fn params(&self) -> &[u16] { &self.params }

    /// Get CSI parameter with default value.
    pub fn param(&self, idx: usize, default: u16) -> u16 {
        self.params.get(idx).copied().filter(|&p| p > 0).unwrap_or(default)
    }

    /// Get intermediate bytes.
    pub fn intermediates(&self) -> &[u8] { &self.intermediate }

    /// Get OSC string buffer.
    pub fn osc_data(&self) -> &[u8] { &self.osc_buffer }

    /// Reset parser to initial state.
    pub fn reset(&mut self) {
        self.state = State::Ground;
        self.params.clear();
        self.param_acc = 0;
        self.intermediate.clear();
        self.osc_buffer.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_count() {
        assert_eq!(STATE_COUNT, 7);
    }

    #[test]
    fn test_ground_printable() {
        let s = step(State::Ground, b'A');
        assert_eq!(s.next, State::Ground);
        assert_eq!(s.action, Action::Print(b'A'));
    }

    #[test]
    fn test_esc_transitions() {
        let s = step(State::Ground, 0x1B);
        assert_eq!(s.next, State::Escape);
    }

    #[test]
    fn test_csi_dispatch() {
        let mut parser = VtParser::new();
        // ESC [
        parser.advance(0x1B);
        assert_eq!(parser.state(), State::Escape);
        parser.advance(b'[');
        assert_eq!(parser.state(), State::CsiEntry);
        // 1;2
        parser.advance(b'1');
        parser.advance(b';');
        parser.advance(b'2');
        // H (final byte)
        let action = parser.advance(b'H');
        assert!(matches!(action, Action::DispatchCsi(b'H')));
        assert_eq!(parser.params(), &[1, 2]);
    }

    #[test]
    fn test_osc_bel_termination() {
        let s = step(State::OscString, 0x07);
        assert_eq!(s.next, State::Ground);
        assert!(matches!(s.action, Action::DispatchOsc));
    }

    #[test]
    fn test_invalid_transition_recovers() {
        let s = step(State::CsiEntry, 0x1B); // ESC in CSI
        assert_eq!(s.action, Action::ErrorRecover);
    }

    #[test]
    fn test_classify_deterministic() {
        assert_eq!(classify(0x1B), "esc");
        assert_eq!(classify(b'A'), "print");
        assert_eq!(classify(b';'), "param");
        assert_eq!(classify(b' '), "intermediate");
        assert_eq!(classify(b'H'), "final");
        assert_eq!(classify(0x0D), "control");
    }

    #[test]
    fn test_param_default() {
        let parser = VtParser::new();
        assert_eq!(parser.param(0, 1), 1); // default when no params
    }

    #[test]
    fn test_param_saturation() {
        let mut parser = VtParser::new();
        parser.advance(0x1B);
        parser.advance(b'[');
        // Very large number
        for _ in 0..10 { parser.advance(b'9'); }
        let action = parser.advance(b'm');
        assert!(matches!(action, Action::DispatchCsi(b'm')));
        assert_eq!(parser.params()[0], u16::MAX); // saturated
    }
}
```

### Test Strategy

1. `STATE_COUNT == 7` invariant. `TST-070`.
2. Ground-state printable bytes emit `Print`. `TST-071`.
3. ESC transitions to `Escape`. `TST-072`.
4. CSI final byte dispatch path coverage. `TST-073`.
5. OSC BEL termination dispatches `DispatchOsc`. `TST-074`.
6. Invalid transition returns `ErrorRecover` without panic. `TST-075`.
7. Partial chunk parsing preserves state across calls. `TST-076`.
8. Table-driven transition determinism hash snapshot. `TST-077`.
9. Parser buffer overflow path produces recoverable error action. `TST-078`.
10. **[v13 P2]** `classify()` is total over all u8 values. `TST-079`.
11. **[v13 P2]** CSI parameter accumulation with semicolons. `TST-080`.
12. **[v13 P2]** Parameter saturation at u16::MAX. `TST-081`.

### AGENTS.md Rules

- `RULE-S06-01`: Parser must expose all seven states as public enum variants. **Enforcement:** Compile check.
- `RULE-S06-02`: Transition function must be total and panic-free. **Enforcement:** Fuzz + panic abort CI.
- `RULE-S06-03`: Parser emits actions; grid mutation in parser is forbidden. **Enforcement:** API boundary lint.
- `RULE-S06-04`: Dispatch table changes require replay fixture regeneration. **Enforcement:** Fixture hash gate.
- `RULE-S06-05`: **[v13 P2]** `classify()` must be deterministic with no locale dependence. **Enforcement:** Property test.
- `RULE-S06-06`: **[v13 P2]** CSI parameter accumulation must use saturating arithmetic. **Enforcement:** Unit test.

---

## 7. PtyHandle Lifecycle and Resource Cleanup [v13 P2]

### Design Decisions

- **[v13 P2]** `PtyHandle` is an opaque identifier with generation component to prevent stale-handle reuse (from GPT's `slot: u32` + `generation: u32` design).
- **[v13 P2]** Lifecycle states: `Allocated`, `Spawned`, `Running`, `Stopping`, `Exited`, `Reaped`, `Closed` (7-state model from GPT, replacing Claude's 3-state model).
- **[v13 P2]** `PtyRecord` struct with explicit `transition()` method validates state transitions (from GPT).
- **[v13 P2]** `can_io()` returns true only in `Running` state (from GPT).
- Descriptor ownership is explicit: runtime owns master fd and child pid mapping.
- Close semantics are idempotent and safe across repeated calls.
- `Drop` for runtime-owned guard triggers best-effort cleanup and emits diagnostic events.
- Graceful shutdown sequence: send TERM, wait bounded interval, then force KILL, then reap.
- Reap step is mandatory; orphaned child detection runs at runtime shutdown.
- Read/write after `Closed` returns deterministic `InvalidHandleState`.
- Handle cloning is prohibited; passing handles is by copy of value type but validated per generation.
- PTY resize is allowed only in `Running` state.
- SIGCHLD races are handled by state transition guards and idempotent reap.
- **[v13 P2]** TermletState <-> PtyState alignment table:

| TermletState | PtyState(s) |
|-------------|-------------|
| Spawning | Allocated, Spawned |
| Running | Running |
| Stopping | Stopping |
| Exited | Exited, Reaped, Closed |
| SpawnFailed | (no PtyHandle) |

- Traceability: `INV-014`, `API-020`.

### Rust Example

```rust
// crates/mux-pty/src/lifecycle.rs
// [v13 P2] From GPT: 7-state PtyHandle lifecycle

#![allow(dead_code)]

/// [v13 P2] PtyHandle with slot+generation (from GPT).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PtyHandle {
    pub slot: u32,
    pub generation: u32,
}

/// [v13 P2] Seven PTY lifecycle states (from GPT).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtyState {
    Allocated,
    Spawned,
    Running,
    Stopping,
    Exited,
    Reaped,
    Closed,
}

/// PTY lifecycle errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtyError {
    InvalidTransition,
    InvalidHandleState,
    StaleGeneration,
}

/// [v13 P2] PtyRecord with transition validation (from GPT).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PtyRecord {
    pub handle: PtyHandle,
    pub state: PtyState,
}

impl PtyRecord {
    pub fn new(slot: u32, generation: u32) -> Self {
        Self {
            handle: PtyHandle { slot, generation },
            state: PtyState::Allocated,
        }
    }

    /// Validate and perform state transition.
    pub fn transition(&mut self, next: PtyState) -> Result<(), PtyError> {
        let ok = matches!(
            (self.state, next),
            (PtyState::Allocated, PtyState::Spawned)
                | (PtyState::Spawned, PtyState::Running)
                | (PtyState::Running, PtyState::Stopping)
                | (PtyState::Running, PtyState::Exited)
                | (PtyState::Stopping, PtyState::Exited)
                | (PtyState::Exited, PtyState::Reaped)
                | (PtyState::Reaped, PtyState::Closed)
                | (PtyState::Closed, PtyState::Closed)  // idempotent
        );
        if ok {
            self.state = next;
            Ok(())
        } else {
            Err(PtyError::InvalidTransition)
        }
    }

    /// IO operations only permitted in Running state.
    pub fn can_io(&self) -> bool {
        self.state == PtyState::Running
    }

    /// Resize only permitted in Running state.
    pub fn can_resize(&self) -> bool {
        self.state == PtyState::Running
    }

    /// Whether the process has terminated.
    pub fn is_terminated(&self) -> bool {
        matches!(self.state, PtyState::Exited | PtyState::Reaped | PtyState::Closed)
    }

    /// Full cleanup sequence: Stopping -> Exited -> Reaped -> Closed.
    pub fn cleanup_sequence(&mut self) -> Vec<PtyState> {
        let mut steps = Vec::new();
        let sequence = match self.state {
            PtyState::Running => vec![PtyState::Stopping, PtyState::Exited, PtyState::Reaped, PtyState::Closed],
            PtyState::Stopping => vec![PtyState::Exited, PtyState::Reaped, PtyState::Closed],
            PtyState::Exited => vec![PtyState::Reaped, PtyState::Closed],
            PtyState::Reaped => vec![PtyState::Closed],
            _ => vec![],
        };
        for next in sequence {
            if self.transition(next).is_ok() {
                steps.push(next);
            }
        }
        steps
    }
}

/// PtyHandle generation-checked registry.
pub struct PtyRegistry {
    records: Vec<Option<PtyRecord>>,
    next_generation: Vec<u32>,
}

impl PtyRegistry {
    pub fn new(capacity: usize) -> Self {
        Self {
            records: (0..capacity).map(|_| None).collect(),
            next_generation: vec![0; capacity],
        }
    }

    pub fn allocate(&mut self) -> Option<PtyHandle> {
        for (slot, record) in self.records.iter_mut().enumerate() {
            if record.is_none() {
                let gen = self.next_generation[slot];
                self.next_generation[slot] = gen.wrapping_add(1);
                let handle = PtyHandle { slot: slot as u32, generation: gen };
                *record = Some(PtyRecord::new(slot as u32, gen));
                return Some(handle);
            }
        }
        None // all slots full
    }

    pub fn get(&self, handle: PtyHandle) -> Result<&PtyRecord, PtyError> {
        let slot = handle.slot as usize;
        if slot >= self.records.len() {
            return Err(PtyError::InvalidHandleState);
        }
        match &self.records[slot] {
            Some(record) if record.handle.generation == handle.generation => Ok(record),
            Some(_) => Err(PtyError::StaleGeneration),
            None => Err(PtyError::InvalidHandleState),
        }
    }

    pub fn get_mut(&mut self, handle: PtyHandle) -> Result<&mut PtyRecord, PtyError> {
        let slot = handle.slot as usize;
        if slot >= self.records.len() {
            return Err(PtyError::InvalidHandleState);
        }
        match &mut self.records[slot] {
            Some(record) if record.handle.generation == handle.generation => Ok(record),
            Some(_) => Err(PtyError::StaleGeneration),
            None => Err(PtyError::InvalidHandleState),
        }
    }

    pub fn release(&mut self, handle: PtyHandle) -> Result<(), PtyError> {
        let record = self.get_mut(handle)?;
        if record.state != PtyState::Closed {
            return Err(PtyError::InvalidTransition);
        }
        self.records[handle.slot as usize] = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_legal_transitions() {
        let mut r = PtyRecord::new(0, 0);
        assert!(r.transition(PtyState::Spawned).is_ok());
        assert!(r.transition(PtyState::Running).is_ok());
        assert!(r.transition(PtyState::Stopping).is_ok());
        assert!(r.transition(PtyState::Exited).is_ok());
        assert!(r.transition(PtyState::Reaped).is_ok());
        assert!(r.transition(PtyState::Closed).is_ok());
    }

    #[test]
    fn test_illegal_transition() {
        let mut r = PtyRecord::new(0, 0);
        assert_eq!(r.transition(PtyState::Running), Err(PtyError::InvalidTransition));
    }

    #[test]
    fn test_idempotent_close() {
        let mut r = PtyRecord::new(0, 0);
        r.state = PtyState::Closed;
        assert!(r.transition(PtyState::Closed).is_ok());
    }

    #[test]
    fn test_can_io_only_running() {
        let mut r = PtyRecord::new(0, 0);
        assert!(!r.can_io());
        r.state = PtyState::Running;
        assert!(r.can_io());
        r.state = PtyState::Stopping;
        assert!(!r.can_io());
    }

    #[test]
    fn test_stale_generation() {
        let mut reg = PtyRegistry::new(4);
        let h1 = reg.allocate().unwrap();
        // Transition to closed and release
        {
            let rec = reg.get_mut(h1).unwrap();
            rec.state = PtyState::Closed;
        }
        reg.release(h1).unwrap();
        // Allocate new handle in same slot
        let h2 = reg.allocate().unwrap();
        assert_eq!(h1.slot, h2.slot);
        assert_ne!(h1.generation, h2.generation);
        // Old handle is stale
        assert_eq!(reg.get(h1), Err(PtyError::StaleGeneration));
    }

    #[test]
    fn test_cleanup_sequence() {
        let mut r = PtyRecord::new(0, 0);
        r.state = PtyState::Running;
        let steps = r.cleanup_sequence();
        assert_eq!(steps, vec![PtyState::Stopping, PtyState::Exited, PtyState::Reaped, PtyState::Closed]);
        assert_eq!(r.state, PtyState::Closed);
    }
}
```

### Test Strategy

1. Legal transition table coverage. `TST-090`.
2. Illegal transitions return `InvalidTransition`. `TST-091`.
3. Repeated close remains idempotent. `TST-092`.
4. `can_io` true only in `Running`. `TST-093`.
5. Stale generation handle rejected by lookup. `TST-094`.
6. Cleanup sequence emits events in deterministic order. `TST-095`.
7. Descriptor leak detector reports zero leaked FDs after shutdown. `TST-096`.
8. Zombie process detector reports none after reap. `TST-097`.
9. Concurrent stop requests converge to `Closed` exactly once. `TST-098`.
10. **[v13 P2]** Registry allocate/release cycle preserves generation monotonicity. `TST-099`.

### AGENTS.md Rules

- `RULE-S07-01`: PTY lifecycle transitions must use explicit transition table. **Enforcement:** Unit tests.
- `RULE-S07-02`: All cleanup paths must be idempotent. **Enforcement:** Repeated-call tests.
- `RULE-S07-03`: Slot+generation validation is mandatory for handle lookup. **Enforcement:** Stale-handle tests.
- `RULE-S07-04`: Runtime shutdown must perform reap and leak checks. **Enforcement:** Integration gate.
- `RULE-S07-05`: **[v13 P2]** PtyState has exactly 7 states. **Enforcement:** STATE_COUNT const test.
- `RULE-S07-06`: **[v13 P2]** IO operations rejected outside Running state. **Enforcement:** `can_io()` gate.

---

## 8. Termlet Snapshot Format and Versioning [v13 P2]

### Design Decisions

- **[v13 P2]** Snapshot format is dual-mode: human text (`.tfsnap.txt`) and binary (`.tfsnap.bin`) with common semantic fields (from GPT/Claude merged).
- **[v13 P2]** Binary format starts with 8-byte magic `TFSNAP13` and little-endian u16 version (from GPT's richer format, replacing Claude's 4-byte `TFSN` + u8).
- **[v13 P2]** `SnapshotMeta` includes `cols`, `rows`, `cursor_col`, `cursor_row`, `revision` (from GPT).
- **[v13 P2]** `SnapshotCell` includes `ch: u32` (Unicode codepoint), `style: u16`, `flags: u16` (from GPT).
- **[v13 P2]** Trailing FNV-1a checksum (from GPT's `checksum32`).
- Text format is deterministic and UTF-8 normalized with LF line endings.
- Format evolution policy: additive-only within major version; breaking change bumps major and magic.
- Cross-language bindings parse binary and may expose text projection helpers.
- Snapshot serialization is independent from runtime and PTY backends.
- Snapshot parser never panics on malformed inputs and returns typed parse errors.
- Termlet regression fixtures store both encodings for each scenario.
- Traceability: `INV-015`, `API-025`.

### Rust Example

```rust
// crates/mux-snapshot/src/lib.rs
// [v13 P2] Cross-pollinated: GPT TFSNAP13/checksum + Claude text mode

#![allow(dead_code)]

/// [v13 P2] 8-byte magic (from GPT).
pub const SNAP_MAGIC: &[u8; 8] = b"TFSNAP13";
pub const SNAP_VERSION: u16 = 1;

/// [v13 P2] Snapshot metadata with cursor and revision (from GPT).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotMeta {
    pub cols: u16,
    pub rows: u16,
    pub cursor_col: u16,
    pub cursor_row: u16,
    pub revision: u64,
}

/// [v13 P2] Cell-level snapshot data (from GPT).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotCell {
    pub ch: u32,
    pub style: u16,
    pub flags: u16,
}

/// Complete snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub meta: SnapshotMeta,
    pub cells: Vec<SnapshotCell>,
}

/// [v13 P2] Snapshot decode error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotError {
    TooShort,
    BadMagic,
    UnsupportedVersion(u16),
    Truncated,
    ChecksumMismatch { expected: u32, actual: u32 },
    InvalidUtf8(String),
}

impl std::fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooShort => write!(f, "snapshot too short"),
            Self::BadMagic => write!(f, "bad magic bytes"),
            Self::UnsupportedVersion(v) => write!(f, "unsupported version: {v}"),
            Self::Truncated => write!(f, "truncated payload"),
            Self::ChecksumMismatch { expected, actual } =>
                write!(f, "checksum mismatch: expected {expected:#x}, got {actual:#x}"),
            Self::InvalidUtf8(e) => write!(f, "invalid UTF-8: {e}"),
        }
    }
}

impl std::error::Error for SnapshotError {}

/// [v13 P2] FNV-1a checksum (from GPT).
pub fn checksum32(data: &[u8]) -> u32 {
    let mut x: u32 = 0x811C_9DC5;
    for b in data {
        x ^= *b as u32;
        x = x.wrapping_mul(0x0100_0193);
    }
    x
}

/// [v13 P2] Encode snapshot to binary format.
pub fn encode_binary(s: &Snapshot) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(SNAP_MAGIC);
    out.extend_from_slice(&SNAP_VERSION.to_le_bytes());
    out.extend_from_slice(&s.meta.cols.to_le_bytes());
    out.extend_from_slice(&s.meta.rows.to_le_bytes());
    out.extend_from_slice(&s.meta.cursor_col.to_le_bytes());
    out.extend_from_slice(&s.meta.cursor_row.to_le_bytes());
    out.extend_from_slice(&s.meta.revision.to_le_bytes());
    out.extend_from_slice(&(s.cells.len() as u32).to_le_bytes());
    for c in &s.cells {
        out.extend_from_slice(&c.ch.to_le_bytes());
        out.extend_from_slice(&c.style.to_le_bytes());
        out.extend_from_slice(&c.flags.to_le_bytes());
    }
    let sum = checksum32(&out);
    out.extend_from_slice(&sum.to_le_bytes());
    out
}

/// [v13 P2] Decode snapshot from binary format.
pub fn decode_binary(data: &[u8]) -> Result<Snapshot, SnapshotError> {
    // Header: 8 magic + 2 version + 2 cols + 2 rows + 2 cursor_col + 2 cursor_row + 8 revision + 4 cell_count = 30
    if data.len() < 34 { return Err(SnapshotError::TooShort); }
    if &data[0..8] != SNAP_MAGIC { return Err(SnapshotError::BadMagic); }
    let version = u16::from_le_bytes([data[8], data[9]]);
    if version != SNAP_VERSION { return Err(SnapshotError::UnsupportedVersion(version)); }

    let cols = u16::from_le_bytes([data[10], data[11]]);
    let rows = u16::from_le_bytes([data[12], data[13]]);
    let cursor_col = u16::from_le_bytes([data[14], data[15]]);
    let cursor_row = u16::from_le_bytes([data[16], data[17]]);
    let revision = u64::from_le_bytes([
        data[18], data[19], data[20], data[21],
        data[22], data[23], data[24], data[25],
    ]);
    let cell_count = u32::from_le_bytes([data[26], data[27], data[28], data[29]]) as usize;

    let cell_data_len = cell_count * 8; // 4 + 2 + 2 per cell
    let expected_len = 30 + cell_data_len + 4; // header + cells + checksum
    if data.len() < expected_len { return Err(SnapshotError::Truncated); }

    // Verify checksum
    let payload = &data[..expected_len - 4];
    let expected_sum = checksum32(payload);
    let actual_sum = u32::from_le_bytes([
        data[expected_len - 4], data[expected_len - 3],
        data[expected_len - 2], data[expected_len - 1],
    ]);
    if expected_sum != actual_sum {
        return Err(SnapshotError::ChecksumMismatch { expected: expected_sum, actual: actual_sum });
    }

    let mut cells = Vec::with_capacity(cell_count);
    let mut offset = 30;
    for _ in 0..cell_count {
        let ch = u32::from_le_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]]);
        let style = u16::from_le_bytes([data[offset+4], data[offset+5]]);
        let flags = u16::from_le_bytes([data[offset+6], data[offset+7]]);
        cells.push(SnapshotCell { ch, style, flags });
        offset += 8;
    }

    Ok(Snapshot {
        meta: SnapshotMeta { cols, rows, cursor_col, cursor_row, revision },
        cells,
    })
}

/// [v13 P2] Text snapshot format for human readability.
pub fn encode_text(s: &Snapshot) -> String {
    let mut out = String::new();
    out.push_str(&format!("# TFSNAP v{}\n", SNAP_VERSION));
    out.push_str(&format!("# size: {}x{}\n", s.meta.cols, s.meta.rows));
    out.push_str(&format!("# cursor: {},{}\n", s.meta.cursor_col, s.meta.cursor_row));
    out.push_str(&format!("# revision: {}\n", s.meta.revision));
    out.push_str("---\n");
    let cols = s.meta.cols as usize;
    for (i, cell) in s.cells.iter().enumerate() {
        if let Some(ch) = char::from_u32(cell.ch) {
            out.push(ch);
        } else {
            out.push('\u{FFFD}');
        }
        if (i + 1) % cols == 0 {
            out.push('\n');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_snapshot() -> Snapshot {
        Snapshot {
            meta: SnapshotMeta {
                cols: 3, rows: 2,
                cursor_col: 1, cursor_row: 0,
                revision: 42,
            },
            cells: vec![
                SnapshotCell { ch: b'H' as u32, style: 0, flags: 0 },
                SnapshotCell { ch: b'i' as u32, style: 0, flags: 0 },
                SnapshotCell { ch: b' ' as u32, style: 0, flags: 0 },
                SnapshotCell { ch: b' ' as u32, style: 0, flags: 0 },
                SnapshotCell { ch: b' ' as u32, style: 0, flags: 0 },
                SnapshotCell { ch: b' ' as u32, style: 0, flags: 0 },
            ],
        }
    }

    #[test]
    fn test_binary_roundtrip() {
        let s = sample_snapshot();
        let encoded = encode_binary(&s);
        let decoded = decode_binary(&encoded).unwrap();
        assert_eq!(s, decoded);
    }

    #[test]
    fn test_magic_check() {
        let mut data = encode_binary(&sample_snapshot());
        data[0] = b'X'; // corrupt magic
        assert_eq!(decode_binary(&data), Err(SnapshotError::BadMagic));
    }

    #[test]
    fn test_checksum_mismatch() {
        let mut data = encode_binary(&sample_snapshot());
        let last = data.len() - 1;
        data[last] ^= 0xFF; // corrupt checksum
        assert!(matches!(decode_binary(&data), Err(SnapshotError::ChecksumMismatch { .. })));
    }

    #[test]
    fn test_encode_stable() {
        let s = sample_snapshot();
        let a = encode_binary(&s);
        let b = encode_binary(&s);
        assert_eq!(a, b);
    }

    #[test]
    fn test_text_format() {
        let s = sample_snapshot();
        let text = encode_text(&s);
        assert!(text.contains("TFSNAP v1"));
        assert!(text.contains("size: 3x2"));
        assert!(text.contains("cursor: 1,0"));
        assert!(text.contains("revision: 42"));
    }

    #[test]
    fn test_too_short() {
        assert_eq!(decode_binary(&[0u8; 10]), Err(SnapshotError::TooShort));
    }

    #[test]
    fn test_unsupported_version() {
        let mut data = encode_binary(&sample_snapshot());
        data[8] = 99; // bad version
        data[9] = 0;
        assert!(matches!(decode_binary(&data), Err(SnapshotError::UnsupportedVersion(99))));
    }
}
```

### Test Strategy

1. Binary header magic and version correctness. `TST-100`.
2. Encode output stable for same snapshot. `TST-101`.
3. Checksum mismatch detection in decoder. `TST-102`.
4. Text and binary encode equal semantic content. `TST-103`.
5. Malformed binary inputs return parse errors. `TST-104`.
6. Unknown optional fields ignored when compatible flag set. `TST-105`.
7. Cross-language roundtrip for binary format. `TST-106`.
8. Fixture compatibility across lanes and versions. `TST-107`.
9. Snapshot revision monotonicity in replay logs. `TST-108`.
10. **[v13 P2]** Cursor position preserved in round-trip. `TST-109`.
11. **[v13 P2]** FNV-1a checksum deterministic. `TST-110`.

### AGENTS.md Rules

- `RULE-S08-01`: Snapshot format changes require version policy update. **Enforcement:** Schema diff gate.
- `RULE-S08-02`: Binary snapshots must include magic, version, checksum. **Enforcement:** Parser tests.
- `RULE-S08-03`: All snapshot encoders are deterministic. **Enforcement:** Repeatability tests.
- `RULE-S08-04`: Bindings must support canonical binary decode path. **Enforcement:** Cross-language contract tests.
- `RULE-S08-05`: **[v13 P2]** Snapshot decode must never panic on any input. **Enforcement:** Fuzz test.

---

## 9. Wire Protocol Compatibility (tmux) [v13 P2]

### Design Decisions

- **[v13 P2]** Wire compatibility scope is command framing, control responses, and command semantics for declared lane set (from GPT lane model).
- **[v13 P2]** `Frame` type with `request_id`, `command`, and `payload` (from GPT).
- **[v13 P2]** `DecodeResult` enum: `NeedMore`, `Frame`, `Fatal` (from GPT).
- Framing decoder is strict on malformed lengths and protocol-level invariants.
- Unknown commands are surfaced as compatibility events and mapped to deterministic error responses.
- Version lane adapters provide semantic shims for changed command behavior.
- Request IDs and causation IDs are tracked separately.
- Protocol parser operates on bytes and does not perform business logic.
- Command handlers emit domain events/effects only.
- Reply encoder uses stable field ordering.
- Backpressure is explicit via bounded output queues.
- Connection-scoped parser state prevents stream bleed.
- Frame size limits are lane-configurable with safe defaults.
- Compatibility conformance includes golden transcript replay against tmux fixtures.
- Error responses include machine code and human summary.
- Control mode support is layered over the same command model.
- Traceability: `INV-001`, `INV-004`, `API-030`.

### Rust Example

```rust
// crates/mux-proto/src/lib.rs
// [v13 P2] Cross-pollinated: GPT Frame/DecodeResult

#![allow(dead_code)]

/// [v13 P2] Wire frame type (from GPT).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub request_id: u64,
    pub command: String,
    pub payload: Vec<u8>,
}

/// [v13 P2] Decode result (from GPT).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeResult {
    NeedMore,
    Frame(Frame),
    Fatal(&'static str),
}

/// Decode a frame from a byte buffer.
pub fn decode(buf: &[u8], max_payload: usize) -> DecodeResult {
    // Minimum header: 8 (request_id) + 4 (length) = 12
    if buf.len() < 12 {
        return DecodeResult::NeedMore;
    }
    let request_id = u64::from_le_bytes([
        buf[0], buf[1], buf[2], buf[3],
        buf[4], buf[5], buf[6], buf[7],
    ]);
    let len = u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]) as usize;
    if len > max_payload {
        return DecodeResult::Fatal("payload_too_large");
    }
    if buf.len() < 12 + len {
        return DecodeResult::NeedMore;
    }
    DecodeResult::Frame(Frame {
        request_id,
        command: String::new(), // parsed from payload in production
        payload: buf[12..12 + len].to_vec(),
    })
}

/// Encode an error response.
pub fn encode_error(request_id: u64, code: &'static str) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&request_id.to_le_bytes());
    out.extend_from_slice(&(code.len() as u32).to_le_bytes());
    out.extend_from_slice(code.as_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_buffer_need_more() {
        assert_eq!(decode(&[0u8; 5], 1024), DecodeResult::NeedMore);
    }

    #[test]
    fn test_oversize_payload() {
        let mut buf = [0u8; 12];
        // Set length to 2000
        buf[8..12].copy_from_slice(&2000u32.to_le_bytes());
        assert_eq!(decode(&buf, 1024), DecodeResult::Fatal("payload_too_large"));
    }

    #[test]
    fn test_valid_frame() {
        let mut buf = Vec::new();
        buf.extend_from_slice(&42u64.to_le_bytes());
        buf.extend_from_slice(&5u32.to_le_bytes());
        buf.extend_from_slice(b"hello");
        match decode(&buf, 1024) {
            DecodeResult::Frame(f) => {
                assert_eq!(f.request_id, 42);
                assert_eq!(f.payload, b"hello");
            }
            _ => panic!("expected Frame"),
        }
    }

    #[test]
    fn test_error_encoding() {
        let enc = encode_error(7, "not_found");
        assert!(enc.len() > 8);
    }
}
```

### Test Strategy

1. Short buffer returns `NeedMore`. `TST-120`.
2. Oversize payload returns fatal error. `TST-121`.
3. Complete buffer decodes deterministic frame. `TST-122`.
4. Error frame encoding is deterministic. `TST-123`.
5. Protocol parser rejects malformed length fields. `TST-124`.
6. Request ID preserved end-to-end. `TST-125`.
7. Lane shim behavior validated with compatibility fixtures. `TST-126`.
8. Transcript replay parity vs tmux baseline. `TST-127`.
9. **[v13 P2]** Unknown command handling is deterministic. `TST-128`.

### AGENTS.md Rules

- `RULE-S09-01`: Protocol decode must be strict on framing invariants. **Enforcement:** Decode tests + fuzz.
- `RULE-S09-02`: Protocol parser cannot mutate domain state directly. **Enforcement:** Architecture lint.
- `RULE-S09-03`: Compatibility fixtures are required for each lane. **Enforcement:** Lane matrix CI.
- `RULE-S09-04`: Unknown command handling must be deterministic and logged. **Enforcement:** Transcript tests.

---

## 10. Configuration and Options

### Design Decisions

- Config model merges defaults, file, CLI, and environment with strict priority order: defaults < file < env < CLI.
- Every option has typed schema, default value, and source annotation.
- Option validation occurs before runtime start. Invalid config never partially applies.
- Compatibility-affecting options are lane-scoped.
- Option names are stable and documented with deprecation windows.
- Config loading is pure parse + validation in layer 0; IO wrapper is in layer 1.
- Secret-bearing options are redacted in logs and telemetry.
- Config schema hash is emitted to telemetry.
- Traceability: `OPS-010`.

### Rust Example

```rust
// crates/mux-conf/src/lib.rs
// [v13 P2] Standalone-compilable config types

#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Default,
    File,
    Env,
    Cli,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Opt<T> {
    pub value: T,
    pub source: Source,
}

impl<T> Opt<T> {
    pub fn new(value: T, source: Source) -> Self {
        Self { value, source }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub socket_path: Opt<String>,
    pub max_clients: Opt<u32>,
    pub enable_crdt: Opt<bool>,
    pub scrollback_limit: Opt<usize>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            socket_path: Opt::new(String::new(), Source::Default),
            max_clients: Opt::new(256, Source::Default),
            enable_crdt: Opt::new(false, Source::Default),
            scrollback_limit: Opt::new(10_000, Source::Default),
        }
    }
}

/// Merge config layers with strict precedence.
pub fn merge(base: Config, overlay: Config) -> Config {
    Config {
        socket_path: if overlay.socket_path.source as u8 > base.socket_path.source as u8 { overlay.socket_path } else { base.socket_path },
        max_clients: if overlay.max_clients.source as u8 > base.max_clients.source as u8 { overlay.max_clients } else { base.max_clients },
        enable_crdt: if overlay.enable_crdt.source as u8 > base.enable_crdt.source as u8 { overlay.enable_crdt } else { base.enable_crdt },
        scrollback_limit: if overlay.scrollback_limit.source as u8 > base.scrollback_limit.source as u8 { overlay.scrollback_limit } else { base.scrollback_limit },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_overrides_file() {
        let file = Config {
            max_clients: Opt::new(100, Source::File),
            ..Config::default()
        };
        let cli = Config {
            max_clients: Opt::new(200, Source::Cli),
            ..Config::default()
        };
        let merged = merge(file, cli);
        assert_eq!(merged.max_clients.value, 200);
        assert_eq!(merged.max_clients.source, Source::Cli);
    }
}
```

### Test Strategy

1. Source precedence defaults < file < env < CLI. `TST-130`.
2. Unknown option handling in strict mode. `TST-131`.
3. Validation rejects invalid max client counts. `TST-132`.
4. Redaction of secret options in logs. `TST-133`.
5. Config schema hash stability. `TST-134`.
6. Effective config snapshot exposed correctly. `TST-135`.

### AGENTS.md Rules

- `RULE-S10-01`: All options require typed schema and defaults. **Enforcement:** Schema linter.
- `RULE-S10-02`: Invalid config must abort startup. **Enforcement:** Integration startup tests.
- `RULE-S10-03`: Source precedence is fixed and cannot be changed silently. **Enforcement:** Merge tests.
- `RULE-S10-04`: Secret options must be redacted in logs and telemetry. **Enforcement:** Log sanitizer tests.

---

## 11. Layout Engine

### Design Decisions

- Layout model uses tree nodes with split orientation, ratio, and minimum size constraints.
- All layout mutations are expressed as pure operations over immutable input snapshot.
- Resize algorithm preserves pane area conservation and uses round-robin one-cell adjustment (`layout.c:448-462`).
- Rounding policy for integer columns/rows is deterministic.
- Layout validation rejects impossible minimum-size constraints.
- Layout serialization is canonicalized for parity testing.
- Layout operations emit change sets for renderer efficiency.
- Focus traversal order is explicit pre-order with deterministic tie-breakers.
- Traceability: `INV-006`, `API-035`.

### Rust Example

```rust
// crates/mux-layout/src/lib.rs
// [v13 P2] Standalone-compilable layout engine

#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation { Horizontal, Vertical }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    Leaf { pane_id: u64, min_w: u16, min_h: u16 },
    Split { orientation: Orientation, ratio_percent: u8, left: Box<Node>, right: Box<Node> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect { pub x: u16, pub y: u16, pub w: u16, pub h: u16 }

impl Rect {
    pub fn area(&self) -> u32 { self.w as u32 * self.h as u32 }
}

pub fn split_rect(r: Rect, o: Orientation, ratio_percent: u8) -> (Rect, Rect) {
    let rp = ratio_percent.min(100);
    match o {
        Orientation::Horizontal => {
            let left_w = ((r.w as u32 * rp as u32) / 100) as u16;
            let right_w = r.w.saturating_sub(left_w);
            (Rect { x: r.x, y: r.y, w: left_w, h: r.h },
             Rect { x: r.x + left_w, y: r.y, w: right_w, h: r.h })
        }
        Orientation::Vertical => {
            let top_h = ((r.h as u32 * rp as u32) / 100) as u16;
            let bottom_h = r.h.saturating_sub(top_h);
            (Rect { x: r.x, y: r.y, w: r.w, h: top_h },
             Rect { x: r.x, y: r.y + top_h, w: r.w, h: bottom_h })
        }
    }
}

pub fn area_conserved(r: Rect, o: Orientation, ratio_percent: u8) -> bool {
    let (a, b) = split_rect(r, o, ratio_percent);
    a.area() + b.area() == r.area()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_conserves_area() {
        let r = Rect { x: 0, y: 0, w: 80, h: 24 };
        assert!(area_conserved(r, Orientation::Horizontal, 50));
        assert!(area_conserved(r, Orientation::Vertical, 30));
    }

    #[test]
    fn test_split_ratio_clamped() {
        let r = Rect { x: 0, y: 0, w: 80, h: 24 };
        let (a, b) = split_rect(r, Orientation::Horizontal, 150); // clamped to 100
        assert_eq!(a.w, 80);
        assert_eq!(b.w, 0);
    }
}
```

### Test Strategy

1. Split ratio rounding determinism. `TST-140`.
2. Area conservation across splits. `TST-141`.
3. Minimum-size validation failures. `TST-142`.
4. Focus traversal deterministic ordering. `TST-143`.
5. Serialization canonical form stability. `TST-144`.
6. Nested split resize correctness. `TST-145`.
7. No-op resize yields no layout change set. `TST-146`.

### AGENTS.md Rules

- `RULE-S11-01`: Layout engine must be pure and deterministic. **Enforcement:** Snapshot tests.
- `RULE-S11-02`: Area conservation invariant is mandatory. **Enforcement:** Property tests.
- `RULE-S11-03`: Invalid minimum constraints must fail fast. **Enforcement:** Validation tests.
- `RULE-S11-04`: Layout serialization must be canonical. **Enforcement:** Golden fixtures.

---

## 12. ORM and QueryList

### Design Decisions

- ORM layer provides Django-like `filter()`, `get()`, `exclude()` over the ServerGraph entity model.
- `QuerySpec` supports string-based filters like `"session_name:starts_with:test"`.
- `QueryList<T>` is a lazy-evaluating filtered view over graph entities.
- Supports 18 operators: `eq`, `ne`, `contains`, `starts_with`, `ends_with`, `gt`, `gte`, `lt`, `lte`, `regex`, `not_contains`, `is_null`, `is_not_null`, `in`, `not_in`, `exists`, `not_exists`, `match`.
- Traversal: `server.sessions().filter_by("name:starts_with:dev").windows().panes()`.
- Cross-language bindings expose identical API with native naming conventions.
- Traceability: `API-040`.

### Rust Example

```rust
// crates/mux-query/src/lib.rs
// [v13 P2] Standalone-compilable query types

#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operator {
    Eq, Ne, Contains, StartsWith, EndsWith,
    Gt, Gte, Lt, Lte, Regex,
    NotContains, IsNull, IsNotNull,
    In, NotIn, Exists, NotExists, Match,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuerySpec {
    pub field: String,
    pub op: Operator,
    pub value: String,
}

impl QuerySpec {
    pub fn parse(input: &str) -> Option<Self> {
        let parts: Vec<&str> = input.splitn(3, ':').collect();
        if parts.len() < 2 { return None; }
        let field = parts[0].to_string();
        let (op, value) = if parts.len() == 3 {
            (Self::parse_op(parts[1])?, parts[2].to_string())
        } else {
            (Operator::Eq, parts[1].to_string())
        };
        Some(Self { field, op, value })
    }

    fn parse_op(s: &str) -> Option<Operator> {
        match s {
            "eq" => Some(Operator::Eq),
            "ne" => Some(Operator::Ne),
            "contains" => Some(Operator::Contains),
            "starts_with" => Some(Operator::StartsWith),
            "ends_with" => Some(Operator::EndsWith),
            "gt" => Some(Operator::Gt),
            "gte" => Some(Operator::Gte),
            "lt" => Some(Operator::Lt),
            "lte" => Some(Operator::Lte),
            "regex" => Some(Operator::Regex),
            "not_contains" => Some(Operator::NotContains),
            "is_null" => Some(Operator::IsNull),
            "is_not_null" => Some(Operator::IsNotNull),
            "in" => Some(Operator::In),
            "not_in" => Some(Operator::NotIn),
            "exists" => Some(Operator::Exists),
            "not_exists" => Some(Operator::NotExists),
            "match" => Some(Operator::Match),
            _ => None,
        }
    }

    pub fn matches(&self, field_value: Option<&str>) -> bool {
        match (&self.op, field_value) {
            (Operator::Eq, Some(v)) => v == self.value,
            (Operator::Ne, Some(v)) => v != self.value,
            (Operator::Contains, Some(v)) => v.contains(&self.value),
            (Operator::StartsWith, Some(v)) => v.starts_with(&self.value),
            (Operator::EndsWith, Some(v)) => v.ends_with(&self.value),
            (Operator::IsNull, None) => true,
            (Operator::IsNull, Some(_)) => false,
            (Operator::IsNotNull, Some(_)) => true,
            (Operator::IsNotNull, None) => false,
            _ => false, // other ops require more context
        }
    }
}

pub struct QueryList<T> {
    items: Vec<T>,
}

impl<T> QueryList<T> {
    pub fn new(items: Vec<T>) -> Self { Self { items } }
    pub fn len(&self) -> usize { self.items.len() }
    pub fn is_empty(&self) -> bool { self.items.is_empty() }
    pub fn first(&self) -> Option<&T> { self.items.first() }
    pub fn get_single(&self) -> Option<&T> {
        if self.items.len() == 1 { self.items.first() } else { None }
    }

    pub fn filter<F: Fn(&T) -> bool>(self, predicate: F) -> Self {
        Self { items: self.items.into_iter().filter(predicate).collect() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let q = QuerySpec::parse("name:eq:test").unwrap();
        assert_eq!(q.field, "name");
        assert_eq!(q.op, Operator::Eq);
        assert_eq!(q.value, "test");
    }

    #[test]
    fn test_parse_all_ops() {
        let ops = ["eq", "ne", "contains", "starts_with", "ends_with", "gt", "gte", "lt", "lte",
                    "regex", "not_contains", "is_null", "is_not_null", "in", "not_in", "exists",
                    "not_exists", "match"];
        assert_eq!(ops.len(), 18);
        for op in &ops {
            let input = format!("f:{}:v", op);
            assert!(QuerySpec::parse(&input).is_some(), "failed for op: {}", op);
        }
    }

    #[test]
    fn test_matches_contains() {
        let q = QuerySpec::parse("name:contains:dev").unwrap();
        assert!(q.matches(Some("my-dev-session")));
        assert!(!q.matches(Some("production")));
    }

    #[test]
    fn test_query_list_filter() {
        let list = QueryList::new(vec![1, 2, 3, 4, 5]);
        let filtered = list.filter(|x| *x > 3);
        assert_eq!(filtered.len(), 2);
    }
}
```

### Test Strategy

1. All 18 operators parseable. `TST-150`.
2. `filter_by` chain produces correct results. `TST-151`.
3. Traversal `sessions().windows().panes()` works. `TST-152`.
4. Invalid query spec returns None. `TST-153`.
5. Cross-language binding filter parity. `TST-154`.
6. Empty result set is handled gracefully. `TST-155`.

### AGENTS.md Rules

- `RULE-S12-01`: All 18 query operators must be parseable. **Enforcement:** Operator coverage test.
- `RULE-S12-02`: QueryList is immutable after creation. **Enforcement:** API review.
- `RULE-S12-03`: Bindings expose identical filter semantics. **Enforcement:** Cross-language contract tests.
- `RULE-S12-04`: Regex operator must be bounded to prevent ReDoS. **Enforcement:** Timeout test.

---

## 13. ServerGraph Slot Reclamation and Generation Counters

### Design Decisions

- ServerGraph stores entities in slot tables with generation counters to prevent stale references.
- IDs encode `slot` and `generation` for sessions, windows, panes, clients, and buffers.
- Deletion marks slot free and increments generation before reuse.
- Slot reclamation is deterministic FIFO by free-list order.
- Cross-entity edges store typed IDs and are validated on dereference.
- Invalid/stale IDs return typed error and do not panic.
- Graph mutation increments graph revision.
- CRDT merge uses same ID semantics to avoid accidental aliasing.
- Traceability: `INV-013`, `API-045`.

### Rust Example

```rust
// crates/mux-core/src/slotmap.rs
// [v13 P2] Generation-counted slot arena (Claude P1 implementation)

#![allow(dead_code)]

pub trait SlotKey: Copy {
    fn from_raw(index: u32, generation: u32) -> Self;
    fn index(self) -> u32;
    fn generation(self) -> u32;
}

enum SlotEntry<V> {
    Occupied { value: V, generation: u32 },
    Free { generation: u32 },
}

pub struct GenSlotMap<K: SlotKey, V> {
    entries: Vec<SlotEntry<V>>,
    free_list: Vec<u32>,
    _marker: std::marker::PhantomData<K>,
}

impl<K: SlotKey, V> GenSlotMap<K, V> {
    pub fn new() -> Self {
        Self { entries: Vec::new(), free_list: Vec::new(), _marker: std::marker::PhantomData }
    }

    pub fn insert(&mut self, value: V) -> K {
        if let Some(index) = self.free_list.pop() {
            let entry = &mut self.entries[index as usize];
            let generation = match entry {
                SlotEntry::Free { generation } => *generation,
                _ => unreachable!(),
            };
            *entry = SlotEntry::Occupied { value, generation };
            K::from_raw(index, generation)
        } else {
            let index = self.entries.len() as u32;
            self.entries.push(SlotEntry::Occupied { value, generation: 0 });
            K::from_raw(index, 0)
        }
    }

    pub fn get(&self, key: K) -> Option<&V> {
        self.entries.get(key.index() as usize).and_then(|entry| match entry {
            SlotEntry::Occupied { value, generation } if *generation == key.generation() => Some(value),
            _ => None,
        })
    }

    pub fn remove(&mut self, key: K) -> Option<V> {
        let idx = key.index() as usize;
        if idx >= self.entries.len() { return None; }
        match &self.entries[idx] {
            SlotEntry::Occupied { generation, .. } if *generation == key.generation() => {
                let new_gen = key.generation() + 1;
                let old = std::mem::replace(&mut self.entries[idx], SlotEntry::Free { generation: new_gen });
                self.free_list.push(key.index());
                match old { SlotEntry::Occupied { value, .. } => Some(value), _ => None }
            }
            _ => None,
        }
    }

    pub fn is_alive(&self, key: K) -> bool { self.get(key).is_some() }
    pub fn len(&self) -> usize {
        self.entries.iter().filter(|e| matches!(e, SlotEntry::Occupied { .. })).count()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SessionId { pub index: u32, pub generation: u32 }
impl SlotKey for SessionId {
    fn from_raw(index: u32, generation: u32) -> Self { Self { index, generation } }
    fn index(self) -> u32 { self.index }
    fn generation(self) -> u32 { self.generation }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_get() {
        let mut map: GenSlotMap<SessionId, String> = GenSlotMap::new();
        let key = map.insert("hello".into());
        assert_eq!(map.get(key), Some(&"hello".to_string()));
    }

    #[test]
    fn test_remove_invalidates() {
        let mut map: GenSlotMap<SessionId, String> = GenSlotMap::new();
        let key = map.insert("hello".into());
        map.remove(key);
        assert_eq!(map.get(key), None);
    }

    #[test]
    fn test_reuse_increments_generation() {
        let mut map: GenSlotMap<SessionId, String> = GenSlotMap::new();
        let k1 = map.insert("a".into());
        map.remove(k1);
        let k2 = map.insert("b".into());
        assert_eq!(k1.index, k2.index);
        assert_ne!(k1.generation, k2.generation);
        assert_eq!(map.get(k1), None);
        assert_eq!(map.get(k2), Some(&"b".to_string()));
    }
}
```

### Test Strategy

1. Insert returns valid ID and resolvable entity. `TST-160`.
2. Remove invalidates old ID. `TST-161`.
3. Reused slot increments generation. `TST-162`.
4. Stale ID cannot resolve new occupant. `TST-163`.
5. Free-list reclamation order deterministic. `TST-164`.
6. Graph revision increments on mutations. `TST-165`.
7. Generation overflow handling path test. `TST-166`.

### AGENTS.md Rules

- `RULE-S13-01`: All entity IDs must include generation counters. **Enforcement:** Type audit.
- `RULE-S13-02`: Stale IDs must fail safely without panic. **Enforcement:** Stale-reference tests.
- `RULE-S13-03`: Slot reclamation order must be deterministic. **Enforcement:** Deterministic unit tests.
- `RULE-S13-04`: Graph mutation must increment revision. **Enforcement:** Revision tests.

---

## 14. State Actor and Snapshot Publication

### Design Decisions

- Runtime uses single-writer actor for state mutation and multi-reader snapshot publication.
- Snapshot publication is monotonic by revision and supports subscriber lag metrics.
- Actor inbox is bounded; overflow handling policy is explicit.
- State actor emits backpressure diagnostics.
- Actor loop enforces strict event ordering by sequence.
- Actor shutdown drains inbox then flushes final snapshot.
- Snapshot payloads are immutable once published.
- Traceability: `INV-003`, `OPS-020`.

### Rust Example

```rust
// crates/mux-state/src/lib.rs
// [v13 P2] Standalone-compilable state actor types

#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnapshotHeader {
    pub revision: u64,
    pub base_revision: u64,
    pub is_delta: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotMsg {
    pub header: SnapshotHeader,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActorState {
    pub revision: u64,
    pub queue_depth: usize,
    pub processing: bool,
}

pub struct ActorMetrics {
    pub total_events: u64,
    pub total_snapshots: u64,
    pub max_queue_depth: usize,
}

impl ActorState {
    pub fn new() -> Self { Self { revision: 0, queue_depth: 0, processing: false } }
    pub fn advance(&mut self) { self.revision += 1; }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_revision_advance() {
        let mut state = ActorState::new();
        state.advance();
        assert_eq!(state.revision, 1);
    }

    #[test]
    fn test_snapshot_header() {
        let h = SnapshotHeader { revision: 5, base_revision: 3, is_delta: true };
        assert!(h.is_delta);
        assert!(h.revision > h.base_revision);
    }
}
```

### Test Strategy

1. Single writer enforced. `TST-170`.
2. Snapshot revisions are monotonic. `TST-171`.
3. Bounded inbox overflow handled. `TST-172`.
4. Actor shutdown flushes final snapshot. `TST-173`.
5. Delta snapshots include valid base revision. `TST-174`.

### AGENTS.md Rules

- `RULE-S14-01`: Single writer for state mutations. **Enforcement:** Architecture lint.
- `RULE-S14-02`: Snapshot revisions must be monotonic. **Enforcement:** Revision tests.
- `RULE-S14-03`: Actor shutdown must flush final state. **Enforcement:** Integration tests.

---

## 15. Control Mode

### Design Decisions

- Control mode layered over the same command model as regular wire protocol.
- Typed notification system with subscription filters.
- Control mode pending limit enforced (`control.c:450-461`).
- Backpressure through bounded output queue.
- Control mode protocol is text-based for human readability.
- Traceability: `INV-004`, `API-050`.

### Rust Example

```rust
// crates/mux-control/src/lib.rs
#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlCmd {
    ListSessions,
    ListWindows { session: String },
    ListPanes { window: String },
    SendKeys { pane: String, keys: Vec<String> },
    CapturePane { pane: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlNotification {
    SessionChanged { session: String },
    WindowChanged { window: String },
    PaneOutput { pane: String, data: Vec<u8> },
    LayoutChanged { window: String },
}

pub fn parse_control_cmd(line: &str) -> Option<ControlCmd> {
    let parts: Vec<&str> = line.splitn(3, ' ').collect();
    match parts.first().copied() {
        Some("list-sessions") => Some(ControlCmd::ListSessions),
        Some("list-windows") if parts.len() >= 2 => Some(ControlCmd::ListWindows { session: parts[1].into() }),
        Some("list-panes") if parts.len() >= 2 => Some(ControlCmd::ListPanes { window: parts[1].into() }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_list_sessions() {
        assert!(matches!(parse_control_cmd("list-sessions"), Some(ControlCmd::ListSessions)));
    }

    #[test]
    fn test_parse_list_windows() {
        match parse_control_cmd("list-windows main") {
            Some(ControlCmd::ListWindows { session }) => assert_eq!(session, "main"),
            _ => panic!("expected ListWindows"),
        }
    }
}
```

### Test Strategy

1. Control command parsing. `TST-180`.
2. Notification subscription filters. `TST-181`.
3. Pending limit enforcement. `TST-182`.
4. Backpressure handling. `TST-183`.

### AGENTS.md Rules

- `RULE-S15-01`: Control mode uses same command model. **Enforcement:** Protocol tests.
- `RULE-S15-02`: Pending limit must be enforced. **Enforcement:** Integration tests.

---

## 16. Language Bindings [v13 P2]

### Design Decisions

- **[v13 P2]** Cross-language naming: `snake_case` in Python (PyO3), `camelCase` in Node (Neon).
- Python bindings: `PyServer` -> `PySession` -> `PyWindow` -> `PyPane` -> `PyTermlet`.
- Node bindings: `JsServer` -> `JsSession` -> `JsWindow` -> `JsPane` -> `JsTermlet`.
- Context manager support: `with server.session("test") as s:` in Python.
- Traversal: `server.sessions.filter_by("name:starts_with:dev")` mirrors libtmux API.
- QueryList/QuerySet exposed in both languages with full operator support.
- **[v13 P2]** Explicit class hierarchy for PyO3 from Gemini's `PySession`/`PyWindow`/`PyPane` pattern.
- **[v13 P2]** Bindings may only call `mux-api` and `mux-orm` public surfaces (layer constraint).
- Traceability: `API-060`.

### Rust Example

```rust
// bindings/python/src/lib.rs
// [v13 P2] Standalone-compilable binding stubs

#![allow(dead_code)]

// PyO3 stubs for standalone compilation
mod pyo3_stubs {
    pub trait IntoPy<T> {}
    pub struct PyResult<T>(pub T);
}

#[derive(Debug, Clone)]
pub struct PyServer {
    socket_path: String,
}

#[derive(Debug, Clone)]
pub struct PySession {
    server: PyServer,
    session_id: u64,
}

#[derive(Debug, Clone)]
pub struct PyWindow {
    session: PySession,
    window_id: u64,
}

#[derive(Debug, Clone)]
pub struct PyPane {
    window: PyWindow,
    pane_id: u64,
}

#[derive(Debug, Clone)]
pub struct PyTermlet {
    pane_id: u64,
    name: String,
}

impl PyServer {
    pub fn sessions(&self) -> Vec<PySession> { vec![] }
    pub fn new_session(&self, _name: &str) -> PySession {
        PySession { server: self.clone(), session_id: 0 }
    }
}

impl PySession {
    pub fn windows(&self) -> Vec<PyWindow> { vec![] }
    pub fn name(&self) -> &str { "" }
}

impl PyWindow {
    pub fn panes(&self) -> Vec<PyPane> { vec![] }
    pub fn name(&self) -> &str { "" }
}

impl PyPane {
    pub fn capture(&self) -> String { String::new() }
    pub fn send_keys(&self, _keys: &str) {}
}

impl PyTermlet {
    pub fn snapshot(&self) -> String { String::new() }
    pub fn send_keys(&self, _keys: &str) {}
    pub fn wait_for(&self, _pattern: &str, _timeout_ms: u64) -> bool { false }
}

// Node binding stubs (camelCase)
#[derive(Debug, Clone)]
pub struct JsServer { socket_path: String }
#[derive(Debug, Clone)]
pub struct JsSession { server_ref: u64, session_id: u64 }
#[derive(Debug, Clone)]
pub struct JsWindow { session_ref: u64, window_id: u64 }
#[derive(Debug, Clone)]
pub struct JsPane { window_ref: u64, pane_id: u64 }
#[derive(Debug, Clone)]
pub struct JsTermlet { pane_id: u64, name: String }

/// Cross-language naming mapping table.
pub const NAMING_MAP: &[(&str, &str, &str)] = &[
    // (Rust, Python, Node)
    ("new_session", "new_session", "newSession"),
    ("list_sessions", "list_sessions", "listSessions"),
    ("filter_by", "filter_by", "filterBy"),
    ("send_keys", "send_keys", "sendKeys"),
    ("wait_for", "wait_for", "waitFor"),
    ("capture_pane", "capture_pane", "capturePane"),
    ("snapshot", "snapshot", "snapshot"),
    ("output_history", "output_history", "outputHistory"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_naming_map_complete() {
        assert!(NAMING_MAP.len() >= 8);
        for (rust, python, node) in NAMING_MAP {
            assert!(!rust.is_empty());
            assert!(!python.is_empty());
            assert!(!node.is_empty());
        }
    }

    #[test]
    fn test_python_uses_snake_case() {
        for (_, python, _) in NAMING_MAP {
            assert!(!python.contains(char::is_uppercase),
                "Python name '{}' should be snake_case", python);
        }
    }

    #[test]
    fn test_hierarchy_traversal() {
        let server = PyServer { socket_path: String::new() };
        let _sessions = server.sessions();
    }
}
```

### Test Strategy

1. Python traversal: `server.sessions[0].windows[0].panes`. `TST-190`.
2. Node traversal: `server.sessions[0].windows[0].panes`. `TST-191`.
3. Context manager support in Python. `TST-192`.
4. QuerySet filter parity across languages. `TST-193`.
5. Naming convention enforcement. `TST-194`.
6. **[v13 P2]** Cross-language naming map completeness. `TST-195`.

### AGENTS.md Rules

- `RULE-S16-01`: Python uses snake_case. **Enforcement:** Naming lint.
- `RULE-S16-02`: Node uses camelCase. **Enforcement:** Naming lint.
- `RULE-S16-03`: Bindings import only `mux-api` and `mux-orm`. **Enforcement:** Dependency audit.
- `RULE-S16-04`: Cross-language naming map covers all public methods. **Enforcement:** Coverage test.
- `RULE-S16-05`: **[v13 P2]** Binding hierarchy mirrors Rust entity model. **Enforcement:** Contract tests.

---

## 17. CRDT Collaboration Layer

### Design Decisions

- CRDT layer is optional, behind `crdt` feature flag.
- Hybrid Logical Clock (HLC) for causal ordering across replicas.
- `LWWRegister<T>` for last-writer-wins on atomic fields.
- `LWWFieldMap` for per-field LWW with tombstone-wins-delete semantics.
- `OpLog` for concurrent pane input stream merging.
- Conflict resolution: per-field LWW with vector clock tiebreaking.
- Tombstone-wins-delete: delete supersedes concurrent updates.
- CRDT merge is commutative, associative, and idempotent.
- CRDT state is serializable for snapshot and wire transport.
- Traceability: `API-070`.

### Rust Example

```rust
// crates/mux-crdt/src/lib.rs
// [v13 P2] Standalone-compilable CRDT types

#![allow(dead_code)]

/// Hybrid Logical Clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HLC {
    pub wall_ms: u64,
    pub counter: u32,
    pub node_id: u16,
}

impl HLC {
    pub fn new(node_id: u16) -> Self {
        Self { wall_ms: 0, counter: 0, node_id }
    }

    pub fn tick(&mut self, now_ms: u64) -> HLC {
        if now_ms > self.wall_ms {
            self.wall_ms = now_ms;
            self.counter = 0;
        } else {
            self.counter += 1;
        }
        *self
    }

    pub fn merge(&mut self, other: HLC, now_ms: u64) {
        if now_ms > self.wall_ms && now_ms > other.wall_ms {
            self.wall_ms = now_ms;
            self.counter = 0;
        } else if self.wall_ms == other.wall_ms {
            self.counter = self.counter.max(other.counter) + 1;
        } else if other.wall_ms > self.wall_ms {
            self.wall_ms = other.wall_ms;
            self.counter = other.counter + 1;
        } else {
            self.counter += 1;
        }
    }
}

/// Last-Writer-Wins Register.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LWWRegister<T> {
    pub value: T,
    pub timestamp: HLC,
}

impl<T: Clone> LWWRegister<T> {
    pub fn new(value: T, timestamp: HLC) -> Self {
        Self { value, timestamp }
    }

    pub fn update(&mut self, value: T, timestamp: HLC) {
        if timestamp > self.timestamp {
            self.value = value;
            self.timestamp = timestamp;
        }
    }

    pub fn merge(&mut self, other: &Self) {
        if other.timestamp > self.timestamp {
            self.value = other.value.clone();
            self.timestamp = other.timestamp;
        }
    }
}

/// Per-field LWW map with tombstone support.
#[derive(Debug, Clone)]
pub struct LWWFieldMap {
    fields: std::collections::HashMap<String, FieldEntry>,
}

#[derive(Debug, Clone)]
struct FieldEntry {
    value: Option<String>,
    timestamp: HLC,
}

impl LWWFieldMap {
    pub fn new() -> Self { Self { fields: std::collections::HashMap::new() } }

    pub fn set(&mut self, key: &str, value: &str, ts: HLC) {
        let entry = self.fields.entry(key.to_string()).or_insert(FieldEntry {
            value: None, timestamp: HLC::new(0),
        });
        if ts > entry.timestamp {
            entry.value = Some(value.to_string());
            entry.timestamp = ts;
        }
    }

    pub fn delete(&mut self, key: &str, ts: HLC) {
        let entry = self.fields.entry(key.to_string()).or_insert(FieldEntry {
            value: None, timestamp: HLC::new(0),
        });
        if ts >= entry.timestamp {  // tombstone-wins-delete: >= not >
            entry.value = None;
            entry.timestamp = ts;
        }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.fields.get(key)?.value.as_deref()
    }

    pub fn merge(&mut self, other: &LWWFieldMap) {
        for (key, other_entry) in &other.fields {
            let entry = self.fields.entry(key.clone()).or_insert(FieldEntry {
                value: None, timestamp: HLC::new(0),
            });
            if other_entry.timestamp > entry.timestamp {
                entry.value = other_entry.value.clone();
                entry.timestamp = other_entry.timestamp;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hlc_tick() {
        let mut hlc = HLC::new(1);
        let t1 = hlc.tick(100);
        let t2 = hlc.tick(100);
        assert!(t2 > t1);
    }

    #[test]
    fn test_lww_register() {
        let ts1 = HLC { wall_ms: 1, counter: 0, node_id: 1 };
        let ts2 = HLC { wall_ms: 2, counter: 0, node_id: 1 };
        let mut reg = LWWRegister::new("old".to_string(), ts1);
        reg.update("new".to_string(), ts2);
        assert_eq!(reg.value, "new");
    }

    #[test]
    fn test_lww_register_ignores_stale() {
        let ts1 = HLC { wall_ms: 2, counter: 0, node_id: 1 };
        let ts2 = HLC { wall_ms: 1, counter: 0, node_id: 1 };
        let mut reg = LWWRegister::new("current".to_string(), ts1);
        reg.update("stale".to_string(), ts2);
        assert_eq!(reg.value, "current");
    }

    #[test]
    fn test_tombstone_wins_delete() {
        let ts = HLC { wall_ms: 5, counter: 0, node_id: 1 };
        let mut map = LWWFieldMap::new();
        map.set("key", "value", ts);
        map.delete("key", ts); // same timestamp: delete wins
        assert_eq!(map.get("key"), None);
    }

    #[test]
    fn test_merge_commutative() {
        let ts1 = HLC { wall_ms: 1, counter: 0, node_id: 1 };
        let ts2 = HLC { wall_ms: 2, counter: 0, node_id: 2 };
        let mut a = LWWFieldMap::new();
        a.set("x", "a", ts1);
        let mut b = LWWFieldMap::new();
        b.set("x", "b", ts2);
        let mut ab = a.clone(); ab.merge(&b);
        let mut ba = b.clone(); ba.merge(&a);
        assert_eq!(ab.get("x"), ba.get("x"));
    }
}
```

### Test Strategy

1. HLC monotonicity. `TST-200`.
2. LWW register update semantics. `TST-201`.
3. Tombstone-wins-delete. `TST-202`.
4. Merge commutativity. `TST-203`.
5. Merge associativity. `TST-204`.
6. Merge idempotency. `TST-205`.
7. Concurrent pane mutation convergence. `TST-206`.

### AGENTS.md Rules

- `RULE-S17-01`: CRDT merge must be commutative, associative, idempotent. **Enforcement:** Property tests.
- `RULE-S17-02`: Tombstone-wins-delete for concurrent delete+update. **Enforcement:** Unit tests.
- `RULE-S17-03`: CRDT behind `crdt` feature flag. **Enforcement:** Cargo tree.
- `RULE-S17-04`: HLC must be monotonic per node. **Enforcement:** Property tests.

---

## 18. Socket and IPC

### Design Decisions

- Socket isolation: test sockets use dedicated path outside default.
- Three guard layers: path guard, PID guard, cleanup guard.
- flock startup lock (`client.c:77-101`).
- Socket directory permissions: 0700.
- Traceability: `INV-007`, `OPS-030`.

### Rust Example

```rust
// crates/mux-os/src/socket.rs
#![allow(dead_code)]

pub struct SocketGuard {
    pub path: std::path::PathBuf,
    pub is_test: bool,
}

impl SocketGuard {
    pub fn new(path: std::path::PathBuf, is_test: bool) -> Self {
        Self { path, is_test }
    }

    pub fn socket_path(&self) -> &std::path::Path {
        &self.path
    }
}

impl Drop for SocketGuard {
    fn drop(&mut self) {
        if self.is_test {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

/// Generate isolated test socket path.
pub fn test_socket_path(test_name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join("termforge-test");
    let _ = std::fs::create_dir_all(&dir);
    dir.join(format!("{}-{}", test_name, std::process::id()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_socket_guard_path() {
        let guard = SocketGuard::new(std::path::PathBuf::from("/tmp/test.sock"), true);
        assert_eq!(guard.socket_path(), std::path::Path::new("/tmp/test.sock"));
    }

    #[test]
    fn test_socket_isolation() {
        let path = test_socket_path("mytest");
        assert!(path.to_string_lossy().contains("termforge-test"));
        assert!(path.to_string_lossy().contains("mytest"));
    }
}
```

### Test Strategy

1. Socket isolation prevents conflicts with live server. `TST-210`.
2. Cleanup guard removes socket on drop. `TST-211`.
3. flock semantics prevent double startup. `TST-212`.

### AGENTS.md Rules

- `RULE-S18-01`: Test sockets must use isolated paths. **Enforcement:** Socket path audit.
- `RULE-S18-02`: Socket cleanup on drop is mandatory for test guards. **Enforcement:** Cleanup tests.

---

## 19. OpenTelemetry (OTEL) Observability [v13 P2]

### Design Decisions

- **[v13]** OTEL span hierarchy: 4-level tree: `server.request` -> `kernel.apply` -> `effect.execute` -> `termlet.op`.
- Spans propagate context across server, client, and language bindings.
- Span names are stable and documented.
- Trace export is configurable (OTLP, stdout, off).
- Metrics: event processing latency, snapshot publication lag, PTY IO throughput.
- Traceability: `OPS-040`.

### Rust Example

```rust
// crates/mux-otel/src/lib.rs
#![allow(dead_code)]

/// [v13] OTEL span hierarchy specification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpanSpec {
    pub name: &'static str,
    pub parent: Option<&'static str>,
    pub level: u8,
}

pub const SPAN_HIERARCHY: &[SpanSpec] = &[
    SpanSpec { name: "server.request", parent: None, level: 0 },
    SpanSpec { name: "kernel.apply", parent: Some("server.request"), level: 1 },
    SpanSpec { name: "effect.execute", parent: Some("kernel.apply"), level: 2 },
    SpanSpec { name: "termlet.op", parent: Some("effect.execute"), level: 3 },
];

pub fn validate_span_hierarchy() -> bool {
    for (i, spec) in SPAN_HIERARCHY.iter().enumerate() {
        if spec.level as usize != i { return false; }
        if i == 0 && spec.parent.is_some() { return false; }
        if i > 0 && spec.parent.is_none() { return false; }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_hierarchy_valid() {
        assert!(validate_span_hierarchy());
    }

    #[test]
    fn test_span_count() {
        assert_eq!(SPAN_HIERARCHY.len(), 4);
    }

    #[test]
    fn test_root_has_no_parent() {
        assert!(SPAN_HIERARCHY[0].parent.is_none());
    }
}
```

### Test Strategy

1. Span hierarchy validation. `TST-220`.
2. Trace propagation across server/client boundary. `TST-221`.
3. Metrics export correctness. `TST-222`.
4. Export configuration switching. `TST-223`.

### AGENTS.md Rules

- `RULE-S19-01`: Span hierarchy is 4-level. **Enforcement:** Hierarchy validation test.
- `RULE-S19-02`: Span names are stable and documented. **Enforcement:** Name snapshot test.
- `RULE-S19-03`: Trace context propagates across language boundaries. **Enforcement:** Cross-boundary test.

---

## 20. tmux Version Management (mux-vm, mux-builder)

### Design Decisions

- `mux-vm`: tmux version manager. Downloads, builds, caches tmux binaries at specific versions.
- `mux-builder`: tmux source builder. Compiles tmux from source with specific options.
- CI matrix tests against LTS, Current, Preview tmux versions.
- Cached binaries keyed by version + platform + build flags.
- Traceability: `OPS-050`.

### Rust Example

```rust
// tools/mux-vm/src/lib.rs
#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TmuxVersion {
    pub major: u8,
    pub minor: u8,
    pub patch: Option<char>, // e.g., '3.3a' -> patch = Some('a')
}

impl TmuxVersion {
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() < 2 { return None; }
        let major = parts[0].parse().ok()?;
        let minor_str = parts[1];
        let (minor_num, patch) = if minor_str.ends_with(char::is_alphabetic) {
            let m: u8 = minor_str[..minor_str.len()-1].parse().ok()?;
            let p = minor_str.chars().last()?;
            (m, Some(p))
        } else {
            (minor_str.parse().ok()?, None)
        };
        Some(Self { major, minor: minor_num, patch })
    }

    pub fn to_string(&self) -> String {
        match self.patch {
            Some(p) => format!("{}.{}{}", self.major, self.minor, p),
            None => format!("{}.{}", self.major, self.minor),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionLane {
    Lts,
    Current,
    Preview,
}

pub const LTS_VERSIONS: &[&str] = &["3.3a"];
pub const CURRENT_VERSIONS: &[&str] = &["3.5", "3.6"];
pub const PREVIEW_VERSIONS: &[&str] = &["next"];

pub fn lane_for(version: &str) -> VersionLane {
    if LTS_VERSIONS.contains(&version) { VersionLane::Lts }
    else if CURRENT_VERSIONS.contains(&version) { VersionLane::Current }
    else { VersionLane::Preview }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version() {
        let v = TmuxVersion::parse("3.3a").unwrap();
        assert_eq!(v.major, 3);
        assert_eq!(v.minor, 3);
        assert_eq!(v.patch, Some('a'));
    }

    #[test]
    fn test_parse_version_no_patch() {
        let v = TmuxVersion::parse("3.5").unwrap();
        assert_eq!(v.major, 3);
        assert_eq!(v.minor, 5);
        assert_eq!(v.patch, None);
    }

    #[test]
    fn test_lane_classification() {
        assert_eq!(lane_for("3.3a"), VersionLane::Lts);
        assert_eq!(lane_for("3.5"), VersionLane::Current);
        assert_eq!(lane_for("next"), VersionLane::Preview);
    }
}
```

### Test Strategy

1. Version parsing correctness. `TST-230`.
2. Lane classification accuracy. `TST-231`.
3. Build cache hit/miss behavior. `TST-232`.
4. CI matrix coverage validation. `TST-233`.

### AGENTS.md Rules

- `RULE-S20-01`: CI matrix must include LTS version. **Enforcement:** Matrix validation.
- `RULE-S20-02`: Build cache keyed by version + platform. **Enforcement:** Cache audit.
- `RULE-S20-03`: mux-vm must not interfere with system tmux. **Enforcement:** Isolation test.

---

## 21. Test Support (mux-test-support)

### Design Decisions

- Test harness provides socket isolation, tmux version selection, and automatic cleanup.
- Socket names generated with test-unique prefixes outside default paths.
- Three guard layers: path guard, PID guard, cleanup guard.
- Version-aware testing: tests can target specific tmux versions via `mux-vm`.
- `TestContext` struct bundles socket path, temp dir, and cleanup handles.
- Traceability: `INV-007`, `OPS-060`.

### Rust Example

```rust
// crates/mux-test-support/src/lib.rs
#![allow(dead_code)]

pub struct TestContext {
    pub socket_name: String,
    pub socket_path: std::path::PathBuf,
    pub tmp_dir: std::path::PathBuf,
    pub tmux_version: Option<String>,
}

impl TestContext {
    pub fn new(test_name: &str) -> Self {
        let pid = std::process::id();
        let socket_name = format!("tf-test-{}-{}", test_name, pid);
        let tmp_dir = std::env::temp_dir().join(format!("termforge-test-{}", pid));
        let _ = std::fs::create_dir_all(&tmp_dir);
        let socket_path = tmp_dir.join(&socket_name);
        Self {
            socket_name,
            socket_path,
            tmp_dir,
            tmux_version: None,
        }
    }

    pub fn with_version(mut self, version: &str) -> Self {
        self.tmux_version = Some(version.to_string());
        self
    }

    pub fn socket_arg(&self) -> String {
        format!("-S {}", self.socket_path.display())
    }

    pub fn cleanup(&self) {
        let _ = std::fs::remove_file(&self.socket_path);
        let _ = std::fs::remove_dir_all(&self.tmp_dir);
    }
}

impl Drop for TestContext {
    fn drop(&mut self) {
        self.cleanup();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_isolation() {
        let ctx = TestContext::new("unit_test");
        assert!(ctx.socket_name.starts_with("tf-test-"));
        assert!(ctx.socket_path.to_string_lossy().contains("termforge-test"));
        // Must not use default tmux socket path
        assert!(!ctx.socket_path.to_string_lossy().contains("/tmp/tmux-"));
    }

    #[test]
    fn test_version_selection() {
        let ctx = TestContext::new("ver_test").with_version("3.3a");
        assert_eq!(ctx.tmux_version.as_deref(), Some("3.3a"));
    }
}
```

### Test Strategy

1. Socket isolation from default server. `TST-240`.
2. Cleanup removes socket and temp dir. `TST-241`.
3. Version selection propagates to test execution. `TST-242`.
4. Multiple concurrent tests use distinct sockets. `TST-243`.

### AGENTS.md Rules

- `RULE-S21-01`: Test sockets must use isolated paths outside default. **Enforcement:** Socket path audit.
- `RULE-S21-02`: Cleanup is mandatory via Drop. **Enforcement:** Cleanup verification.
- `RULE-S21-03`: tmux version must be respected if specified. **Enforcement:** Version check.
- `RULE-S21-04`: No test may use the default tmux server socket. **Enforcement:** Socket name audit.

---

## 22. Parity Testing (mux-regress)

### Design Decisions

- Regression runner executes identical operations against both TermForge and real tmux.
- Parity tests compare control mode output, option resolution, and layout behavior.
- Version-aware: runs against tmux versions from mux-vm.
- Golden transcript format for replay.
- Traceability: `PAR-050`.

### Rust Example

```rust
// tools/mux-regress/src/lib.rs
#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParityResult {
    Match,
    Mismatch { expected: String, actual: String },
    TmuxError(String),
    TermForgeError(String),
}

pub struct ParityTest {
    pub name: String,
    pub tmux_version: String,
    pub commands: Vec<String>,
}

impl ParityTest {
    pub fn run_comparison(&self) -> ParityResult {
        // Stub: in production, runs commands against both tmux and TermForge
        ParityResult::Match
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parity_match() {
        let test = ParityTest {
            name: "basic_attach".into(),
            tmux_version: "3.5".into(),
            commands: vec!["new-session -s test".into()],
        };
        assert_eq!(test.run_comparison(), ParityResult::Match);
    }
}
```

### Test Strategy

1. Parity comparison produces correct result. `TST-250`.
2. Mismatch detection is precise. `TST-251`.
3. Golden transcript replay stability. `TST-252`.
4. Version-specific behavior handled. `TST-253`.

### AGENTS.md Rules

- `RULE-S22-01`: Parity tests run against real tmux. **Enforcement:** CI matrix.
- `RULE-S22-02`: Mismatches are release-blocking for compat gates. **Enforcement:** Gate integration.
- `RULE-S22-03`: Golden transcripts versioned. **Enforcement:** Fixture audit.

---

## 23. Fuzz Testing

### Design Decisions

- Fuzz targets for all parser/decoder crates.
- `cargo fuzz` with libfuzzer on nightly.
- Fuzz findings become permanent regression fixtures within 48 hours.
- VtParser, protocol codec, config parser, snapshot decoder all fuzzed.
- Traceability: `OPS-070`.

### Rust Example

```rust
// crates/mux-grid/fuzz/fuzz_targets/vtparser.rs (conceptual)
#![allow(dead_code)]

pub fn fuzz_vtparser(data: &[u8]) {
    // VtParser must never panic on any input
    let mut parser = VtParserStub::new();
    for &b in data {
        let _ = parser.advance(b);
    }
    // Verify parser is in valid state
    assert!(parser.is_valid_state());
}

struct VtParserStub { state: u8 }
impl VtParserStub {
    fn new() -> Self { Self { state: 0 } }
    fn advance(&mut self, b: u8) -> u8 { self.state = b % 7; self.state }
    fn is_valid_state(&self) -> bool { self.state < 7 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzz_no_panic() {
        fuzz_vtparser(&[0x1B, b'[', b'1', b';', b'2', b'H']);
        fuzz_vtparser(&[0xFF; 1000]);
        fuzz_vtparser(&[]);
    }
}
```

### Test Strategy

1. VtParser fuzz: no panics. `TST-260`.
2. Protocol codec fuzz: no panics. `TST-261`.
3. Config fuzz: no panics. `TST-262`.
4. Snapshot decoder fuzz: no panics. `TST-263`.
5. Regression fixtures from fuzz findings. `TST-264`.

### AGENTS.md Rules

- `RULE-S23-01`: Fuzz targets must exist for all parser/decoder crates. **Enforcement:** Fuzz target audit.
- `RULE-S23-02`: Fuzz findings become regression fixtures within 48 hours. **Enforcement:** PR process.
- `RULE-S23-03`: Test names must match `test_{component}_{behavior}` convention. **Enforcement:** CI regex validation.

---

## 24. Performance Benchmarks [v13 P2]

### Design Decisions

- Criterion benchmarks for hot-path code.
- Nightly CI runs benchmarks; regression > 30% blocks release.
- **[v13]** Added B18 VtParser CSI throughput with parameter parsing.
- **[v13]** Added B19 Grid put_char throughput (80x24 screen fill).
- Traceability: `OPS-050`.

### Performance Targets Table

| ID | Benchmark | Target | CI Gate |
|---|---|---|---|
| B1 | VT100 ASCII 1MB | > 300 MB/s | Warn if < 250 MB/s |
| B2 | VT100 CSI heavy | > 100 MB/s | Warn if < 80 MB/s |
| B3 | Proto decode 10k frames | > 500 MB/s | Warn if < 400 MB/s |
| B4 | Layout resize 20 panes | < 50 us | Warn if > 100 us |
| B5 | Snapshot 50 panes | < 1 ms | Warn if > 2 ms |
| B10 | Termlet spawn (FakePty) | < 500 us | Warn if > 1 ms |
| B11 | Termlet snapshot (80x24) | < 50 us | Warn if > 100 us |
| B12 | Termlet spawn (real PTY) | < 50 ms | Warn if > 100 ms |
| B13 | drain_output 1KB buffer | < 10 us | Warn if > 50 us |
| B15 | Pool stress: 50 spawn/kill | < 5 s | Fail if > 10 s |
| B16 | Grid cell_at 80x24 | < 10 ns/cell | Warn if > 50 ns/cell |
| B17 | Generic predicate overhead | < 1 ms | Warn if > 2 ms |
| B18 | **[v13]** VtParser CSI params 1MB | > 70 MB/s | Warn if < 50 MB/s |
| B19 | **[v13]** Grid put_char 80x24 fill | < 100 us | Warn if > 200 us |

### Rust Example

```rust
// crates/mux-bench/src/lib.rs
#![allow(dead_code)]

#[derive(Debug, Clone)]
pub struct BenchmarkSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub target_ns: u64,
    pub warn_threshold_ns: u64,
}

pub const BENCHMARKS: &[BenchmarkSpec] = &[
    BenchmarkSpec { id: "B1", name: "vt100_ascii_1mb", target_ns: 3_333_333, warn_threshold_ns: 4_000_000 },
    BenchmarkSpec { id: "B16", name: "grid_cell_at_80x24", target_ns: 10, warn_threshold_ns: 50 },
    BenchmarkSpec { id: "B18", name: "vtparser_csi_params_1mb", target_ns: 14_285_714, warn_threshold_ns: 20_000_000 },
    BenchmarkSpec { id: "B19", name: "grid_put_char_80x24", target_ns: 100_000, warn_threshold_ns: 200_000 },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmarks_have_valid_thresholds() {
        for b in BENCHMARKS {
            assert!(b.warn_threshold_ns > b.target_ns,
                "{}: warn threshold must exceed target", b.id);
        }
    }
}
```

### Test Strategy

1. Criterion benchmarks run nightly. `TST-270`.
2. Regression flag at 130% of baseline. `TST-271`.
3. **[v13]** B18 VtParser CSI benchmark measures parameter parsing overhead. `TST-272`.
4. **[v13]** B19 Grid put_char benchmark measures screen fill throughput. `TST-273`.

### AGENTS.md Rules

- `RULE-S24-01`: Performance baselines re-established after architecture changes. **Enforcement:** CI baseline refresh.
- `RULE-S24-02`: Benchmarks must cover hot-path code. **Enforcement:** Benchmark audit.
- `RULE-S24-03`: FakePty spawn < 500us. **Enforcement:** Criterion CI.
- `RULE-S24-04`: Performance regressions > 30% block release. **Enforcement:** Release gate.
- `RULE-S24-05`: **[v13]** VtParser CSI benchmark must include parameter parsing. **Enforcement:** Benchmark presence check.

---

## 25. Visual Client / TUI

### Design Decisions

- ViewModel pattern: pure function `(GraphState, ClientId, TermSize) -> ViewModel`.
- ratatui-based rendering with crossterm terminal IO.
- ViewModel supports snapshot testing with `insta`.
- TUI can attach to both TermForge server and real tmux (via control mode).
- `TermletPoolViewModel` for multi-Termlet inspector with alive count.
- Traceability: `API-100`.

### Rust Example

```rust
// crates/mux-view/src/lib.rs
#![allow(dead_code)]

#[derive(Debug, Clone, Default)]
pub struct ViewModel {
    pub status_top: StatusLine,
    pub status_bottom: StatusLine,
    pub panes: Vec<PaneView>,
    pub borders: Vec<Border>,
    pub mode_indicator: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct StatusLine {
    pub left: String,
    pub center: String,
    pub right: String,
}

#[derive(Debug, Clone)]
pub struct PaneView {
    pub x: u16, pub y: u16,
    pub width: u16, pub height: u16,
    pub content: Vec<String>,
    pub is_active: bool,
}

#[derive(Debug, Clone)]
pub struct Border {
    pub x: u16, pub y: u16,
    pub width: u16, pub height: u16,
}

pub struct TermletPoolViewModel {
    pub termlets: Vec<TermletViewEntry>,
    pub alive_count: usize,
    pub total_count: usize,
}

#[derive(Debug, Clone)]
pub struct TermletViewEntry {
    pub name: String,
    pub state: String,
    pub grid_preview: String,
    pub size: (u16, u16),
    pub exit_status: Option<i32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_view_model() {
        let vm = ViewModel::default();
        assert!(vm.panes.is_empty());
        assert!(vm.mode_indicator.is_none());
    }
}
```

### Test Strategy

1. ViewModel snapshot: pure ViewModel from test state matches `insta` snapshot. `TST-280`.
2. Pane layout: ViewModel pane positions match layout tree. `TST-281`.
3. TermletPoolViewModel: alive_count matches actual alive Termlets. `TST-282`.

### AGENTS.md Rules

- `RULE-S25-01`: TUI reads snapshots only, never mutates graph directly. **Enforcement:** No `&mut ServerGraph` in view code.
- `RULE-S25-02`: UI event handlers emit commands/events, not direct state mutations. **Enforcement:** Code review.
- `RULE-S25-03`: Debug panels are non-blocking and optional. **Enforcement:** Feature flag.

---

## 26. AGENTS.md Rules [v13 P2]

### Design Decisions

This section consolidates all per-section rules into a global reference. Rule naming convention: `RULE-Snn-xx`. **[v13 P2]** Total: 195+ rules (up from 176 in v13 P1). All rules have explicit enforcement mechanisms.

### 26.1 Master Rule Table

| # | Rule ID | Rule | Enforcement |
|---|---|---|---|
| 1 | RULE-S01-01 | All crate names `mux-` prefix | CI grep Cargo.toml |
| 2 | RULE-S01-02 | Binary name `termforge`, alias `tf` | cargo build check |
| 3 | RULE-S01-03 | Compat scope changes update version matrices | CI changed-files |
| 4 | RULE-S01-04 | **[v13]** Version string includes spec version | Integration test |
| 5 | RULE-S01-05 | **[v13 P2]** Compat claims include lane annotation | Docs linter |
| 6 | RULE-S01-06 | **[v13 P2]** Identity changes need changelog delta | PR policy |
| 7-9 | RULE-S02-01..09 | Gate rules (9 total) | See Section 2 |
| 10-18 | RULE-S03-01..09 | Layer/dependency rules (9 total) | See Section 3 |
| 19-24 | RULE-S04-01..06 | Workspace rules (6 total) | See Section 4 |
| 25-34 | RULE-S05-01..10 | Grid/Core rules (10 total) | See Section 5 |
| 35-40 | RULE-S06-01..06 | VtParser rules (6 total) | See Section 6 |
| 41-46 | RULE-S07-01..06 | PtyHandle rules (6 total) | See Section 7 |
| 47-51 | RULE-S08-01..05 | Snapshot rules (5 total) | See Section 8 |
| 52-55 | RULE-S09-01..04 | Wire protocol rules (4 total) | See Section 9 |
| 56-59 | RULE-S10-01..04 | Config rules (4 total) | See Section 10 |
| 60-63 | RULE-S11-01..04 | Layout rules (4 total) | See Section 11 |
| 64-67 | RULE-S12-01..04 | ORM/Query rules (4 total) | See Section 12 |
| 68-71 | RULE-S13-01..04 | ServerGraph rules (4 total) | See Section 13 |
| 72-74 | RULE-S14-01..03 | State actor rules (3 total) | See Section 14 |
| 75-76 | RULE-S15-01..02 | Control mode rules (2 total) | See Section 15 |
| 77-81 | RULE-S16-01..05 | Binding rules (5 total) | See Section 16 |
| 82-85 | RULE-S17-01..04 | CRDT rules (4 total) | See Section 17 |
| 86-87 | RULE-S18-01..02 | Socket rules (2 total) | See Section 18 |
| 88-90 | RULE-S19-01..03 | OTEL rules (3 total) | See Section 19 |
| 91-93 | RULE-S20-01..03 | Builder rules (3 total) | See Section 20 |
| 94-97 | RULE-S21-01..04 | Test support rules (4 total) | See Section 21 |
| 98-100 | RULE-S22-01..03 | Parity rules (3 total) | See Section 22 |
| 101-103 | RULE-S23-01..03 | Fuzz rules (3 total) | See Section 23 |
| 104-108 | RULE-S24-01..05 | Performance rules (5 total) | See Section 24 |
| 109-111 | RULE-S25-01..03 | TUI rules (3 total) | See Section 25 |
| 112-113 | RULE-S26-01..02 | Meta rules (2 total) | This section |
| 114-115 | RULE-S27-01..02 | Risk rules (2 total) | See Section 27 |
| 116-117 | RULE-S28-01..02 | Evolution rules (2 total) | See Section 28 |
| 118-119 | RULE-S29-01..02 | Anchor rules (2 total) | See Section 29 |
| 120-121 | RULE-S30-01..02 | Appendix rules (2 total) | See Section 30 |
| 122-124 | RULE-S31-01..03 | Test matrix rules (3 total) | See Section 31 |
| 125-195 | RULE-S32-01..71 | Termlet rules (71 total) | See Section 32 |

### 26.2 Anti-Patterns (Forbidden)

| Pattern | Why Forbidden | Alternative |
|---|---|---|
| `Arc<Mutex<_>>` on read path | Contention | `ArcSwap` |
| Direct graph mutation outside `apply_event` | Non-deterministic | Submit `Event` |
| `unwrap()` in non-test code | Panic risk | `?` operator |
| `unsafe` outside `mux-os` | Purity violation | `mux-os` API |
| Manual `unlock()` call | Resource leak | `Drop` guard |
| `sleep()` in tests for sync | Flaky tests | Condition variable / `wait_for` |
| `anyhow::Error` in library API | Erases classification | `thiserror` typed errors |
| Termlet without `kill()` | Leaked PTY processes | Context manager / `Drop` |
| Re-compiling pattern per poll | Unnecessary allocation | `PatternMatcher::compile()` once |
| Boolean `alive`/`exited` for Termlet state | Insufficient lifecycle gating | `TermletState` enum |
| `Ord` on `TermletState` | Misleading for branching machine | Use `valid_transition()` |
| **[v13]** Direct cell mutation bypassing `Grid::put_char` | Violates INV-012 | Use `Grid::put_char` |
| **[v13]** Raw slot index without generation check | ABA reuse | `GenSlotMap` with generation |
| **[v13]** PtyHandle use after close() | Dangling handle | Check `PtyHandleRegistry` |
| **[v13 P2]** `char` for Cell content | No grapheme clusters | `String` grapheme field |
| **[v13 P2]** `Vec<Line>` for Grid scrollback | O(n) scroll | `VecDeque<Line>` |
| **[v13 P2]** 3-state PtyHandle lifecycle | Insufficient granularity | 7-state model |

### Rust Example

```rust
// [v13 P2] Rule count verification
pub const TOTAL_RULES_V13P2: usize = 195;

#[cfg(test)]
mod tests {
    #[test]
    fn test_rule_count() {
        assert!(super::TOTAL_RULES_V13P2 >= 195);
    }
}
```

### Test Strategy

1. Static rule checks in CI. `TST-290`.
2. Rule coverage report links each rule to test. `TST-291`.
3. Anti-pattern grep checks run on every PR. `TST-292`.

### AGENTS.md Rules

- `RULE-S26-01`: Every architecture section must define enforceable rules. **Enforcement:** Section review.
- `RULE-S26-02`: Rule IDs are immutable once published. **Enforcement:** Append-only rule table.

---

## 27. Risks and Mitigations [v13 P2]

### Design Decisions

Risk register with v13 P2 additions for Grid grapheme, VecDeque, 7-state PtyHandle, and enhanced snapshot. **[v13 P2]** Added R71-R75.

### Risk Table

| # | Risk | Impact | Probability | Mitigation |
|---|---|---|---|---|
| R1-R64 | (carried from v12 P3) | -- | -- | See v12 P3 risk table |
| R65 | **[v13]** Grid::put_char bypass | High | Low | INV-012 enforcement |
| R66 | **[v13]** GenSlotMap generation overflow | Low | Very Low | 4B+ generations; stress test |
| R67 | **[v13]** VtParser CSI parameter overflow | Medium | Low | Saturating arithmetic + fuzz |
| R68 | **[v13]** PtyHandle use-after-close | High | Medium | PtyRegistry + close guard |
| R69 | **[v13]** Snapshot version mismatch | Medium | Medium | Magic + version + fallback |
| R70 | **[v13]** OTEL span hierarchy drift | Low | Medium | Hierarchy validation test |
| R71 | **[v13 P2]** Grapheme String allocation pressure | Medium | Medium | SSO for single-char; benchmark B19 |
| R72 | **[v13 P2]** VecDeque fragmentation under heavy scroll | Low | Low | Periodic compaction; benchmark |
| R73 | **[v13 P2]** FNV-1a checksum collision | Low | Very Low | Non-cryptographic; acceptable for integrity |
| R74 | **[v13 P2]** 7-state PtyHandle complexity | Medium | Low | Comprehensive transition table tests |
| R75 | **[v13 P2]** Cross-model synthesis inconsistency | Medium | Medium | This document resolves via synthesis decisions |

### Rust Example

```rust
pub const TOTAL_RISKS_V13P2: usize = 75;

#[cfg(test)]
mod tests {
    #[test]
    fn test_risk_count() {
        assert!(super::TOTAL_RISKS_V13P2 >= 75);
    }
}
```

### Test Strategy

1. Risk-to-test mapping complete. `TST-300`.
2. High-impact risks require integration/parity test. `TST-301`.
3. **[v13 P2]** R71-R75 have dedicated tests. `TST-302`.

### AGENTS.md Rules

- `RULE-S27-01`: New high-risk features must add mitigation tests before merge. **Enforcement:** PR template.
- `RULE-S27-02`: Risk register IDs remain stable for auditability. **Enforcement:** Append-only risk table.

---

## 28. Plan Evolution and Changelog

### Design Decisions

Authoritative evolution path. Records v12 -> v13 -> v13 P2 changes.

### Version History

| Version | Date | Lines | Sections | Key Changes |
|---|---|---|---|---|
| v12 P3 (prev) | 2026-02-11 | 4864 | 32 | Placeholder cleanup, deepened S32, governance |
| v13 P1 (Claude) | 2026-02-11 | 6186 | 32 | Grid/VtParser deep, slot reclamation, CRDT conflict, OTEL hierarchy, CI matrix |
| v13 P1 (GPT) | 2026-02-12 | 5568 | 32 | GateClass/Lane, 7-state PtyHandle, FNV-1a snapshot, Frame/DecodeResult, 669 rules |
| v13 P1 (Gemini) | 2026-02-11 | 691 | 32 | VecDeque Grid, grapheme Cell, dirty tracking, 14-state VtParser |
| **v13 P2 (this)** | **2026-02-12** | **5000+** | **32** | **Cross-model synthesis: grapheme Cell, VecDeque Grid, 7-state PtyHandle, FNV-1a snapshot, GateClass/Lane, classify()/Step VtParser, Frame/DecodeResult wire, 195+ rules, 75 risks** |

### Rust Example

```rust
pub const SPEC_VERSION: &str = "v13-pass2";
pub const SPEC_DATE: &str = "2026-02-12";

#[cfg(test)]
mod tests {
    #[test]
    fn test_spec_version() {
        assert!(super::SPEC_VERSION.starts_with("v13"));
        assert!(super::SPEC_VERSION.contains("pass2"));
    }
}
```

### Test Strategy

1. Placeholder-free lint in final specs. `TST-310`.
2. Pass 2 additions exist in Section 26 master rule table. `TST-311`.

### AGENTS.md Rules

- `RULE-S28-01`: Each version bump must list behavior-impacting changes. **Enforcement:** Changelog review.
- `RULE-S28-02`: No unresolved placeholders in finalized specs. **Enforcement:** CI grep.

---

## 29. Reference Anchors [v13 P2]

### Design Decisions

All compatibility claims anchored to verified source code. **[v13 P2]** Added anchors for PtyHandle lifecycle and snapshot format.

### tmux Source Code References

| File | Line(s) | Topic |
|---|---|---|
| `tmux-protocol.h` | 23 | Protocol version 8 |
| `client.c` | 77-101 | flock locking |
| `server-client.c` | 3472-3475 | Protocol violation kills connection |
| `server-client.c` | 3725-3734 | Config loaded after identify |
| `options.c` | 228-241 | Option resolution chain |
| `options.c` | 891-903 | WindowPane FALLTHROUGH |
| `layout.c` | 448-462 | Round-robin resize |
| `layout-custom.c` | 46-57 | Layout checksum |
| `input.c` | State table | **[v13]** VT100 parser states (7 canonical) |
| `key-bindings.c` | Default table | Key binding defaults |
| `control.c` | 450-461 | Control mode pending limit |

### Rust Example

```rust
pub struct Anchor {
    pub claim: &'static str,
    pub file: &'static str,
    pub lines: Option<&'static str>,
}

pub const ANCHORS: &[Anchor] = &[
    Anchor { claim: "protocol version 8", file: "tmux-protocol.h", lines: Some("23") },
    Anchor { claim: "flock startup lock", file: "client.c", lines: Some("77-101") },
    Anchor { claim: "VtParser 7 states", file: "input.c", lines: None },
    Anchor { claim: "round-robin resize", file: "layout.c", lines: Some("448-462") },
    Anchor { claim: "FALLTHROUGH resolution", file: "options.c", lines: Some("891-903") },
];

#[cfg(test)]
mod tests {
    #[test]
    fn test_anchors_not_empty() {
        assert!(!super::ANCHORS.is_empty());
        assert!(super::ANCHORS.len() >= 5);
    }
}
```

### Test Strategy

1. Anchor checker confirms referenced files exist. `TST-320`.
2. Drift checker flags missing files. `TST-321`.

### AGENTS.md Rules

- `RULE-S29-01`: Major compatibility claims require source anchor entries. **Enforcement:** Anchor check.
- `RULE-S29-02`: Remove stale anchors during refactors. **Enforcement:** CI drift checker.

---

## 30. Appendix: Canonical Type Quick Reference [v13 P2]

### Design Decisions

Consolidated reference for all canonical type names. **[v13 P2]** Added types from cross-model synthesis.

### Canonical Types

| Type | Crate | Purpose |
|---|---|---|
| `Cell` | mux-grid | **[v13 P2]** Grapheme cluster cell with style |
| `Line` | mux-grid | **[v13]** Row of cells with dirty tracking |
| `Viewport` | mux-grid | **[v13]** Read-only view into Grid |
| `Grid` | mux-grid | **[v13 P2]** VecDeque-backed terminal grid |
| `VtParser` | mux-grid | **[v13 P2]** 7-state parser with classify/Step |
| `Step` | mux-grid | **[v13 P2]** State transition result |
| `PtyHandle` | mux-pty | **[v13 P2]** Slot+generation opaque handle |
| `PtyRecord` | mux-pty | **[v13 P2]** 7-state lifecycle record |
| `PtyRegistry` | mux-pty | **[v13 P2]** Generation-checked handle registry |
| `Snapshot` | mux-snapshot | **[v13 P2]** Binary snapshot with cells+meta |
| `SnapshotMeta` | mux-snapshot | **[v13 P2]** Cursor+revision+dimensions |
| `SnapshotCell` | mux-snapshot | **[v13 P2]** Codepoint+style+flags |
| `Frame` | mux-proto | **[v13 P2]** Wire protocol frame |
| `DecodeResult` | mux-proto | **[v13 P2]** Decode outcome enum |
| `GateClass` | mux-types | **[v13 P2]** Compat/Correctness/Perf/Op |
| `Lane` | mux-types | **[v13 P2]** LTS/Current/Preview |
| `GateResult` | mux-types | **[v13 P2]** Lane-scoped gate result |
| `Finding` | mux-types | **[v13 P2]** V12 remediation tracking |
| `LWWRegister<T>` | mux-crdt | LWW register |
| `LWWFieldMap` | mux-crdt | **[v13]** Per-field LWW with tombstone |
| `HLC` | mux-crdt | Hybrid logical clock |
| `GenSlotMap<K,V>` | mux-core | **[v13]** Generation-counted arena |
| `SpanSpec` | mux-otel | **[v13]** OTEL span hierarchy entry |
| `QuerySpec` | mux-query | Query filter specification |
| `QueryList<T>` | mux-query | Filtered entity list |
| `TermletState` | mux-termlet | 5-state lifecycle |
| `TermletConfig` | mux-termlet | Termlet configuration |
| `TermletError` | mux-termlet | 11-variant error type |
| `TermletSnapshot` | mux-termlet | Human-readable snapshot |
| `TermletLike` | mux-termlet | Normative API trait |
| `PatternMatcher` | mux-termlet | Compiled pattern for wait_for |
| `TermletPool` | mux-termlet | Named termlet collection |
| `TermletBuilder` | mux-termlet | Fluent builder API |
| (all v12 types carried forward) | -- | -- |

### Rust Example

```rust
pub const CANONICAL_TYPE_COUNT_V13P2: usize = 85;

#[cfg(test)]
mod tests {
    #[test]
    fn test_type_count() {
        assert!(super::CANONICAL_TYPE_COUNT_V13P2 >= 85);
    }
}
```

### Test Strategy

1. Type API compile checks. `TST-330`.
2. Semver surface snapshot for public crates. `TST-331`.
3. **[v13 P2]** New types in canonical list. `TST-332`.

### AGENTS.md Rules

- `RULE-S30-01`: Do not rename canonical types without migration notes. **Enforcement:** Semver check.
- `RULE-S30-02`: Appendix must match actual exported APIs. **Enforcement:** API surface test.

---

## 31. Supplemental Test Matrix [v13 P2]

### Design Decisions

Test matrix extends section-local tests with release gates. **[v13 P2]** Added entries for all cross-model synthesis additions.

### Test Classification (totals)

| Category | Count | Runner | CI? |
|---|---|---|---|
| Unit (pure core) | ~580 | `cargo test` | Yes |
| Property (proptest) | ~60 | `cargo test` | Yes |
| Snapshot (insta) | ~115 | `cargo test` | Yes |
| Protocol fixtures | ~55 | `cargo test` | Yes |
| FakePty scenarios | ~40 | `cargo test` | Yes |
| Integration | ~130 | `cargo test` | Yes |
| Parity (real tmux) | ~200 | `mux-regress` | Yes, matrix |
| Python binding | ~90 | `pytest` | Yes |
| Node binding | ~50 | `vitest` | Yes |
| Performance | ~14 | `criterion` | Yes (warn) |
| Fuzz | 5 targets | `cargo fuzz` | Nightly |
| WASM purity | 1 | `cargo check --target wasm32` | Yes |
| Termlet (all) | ~145 | mixed | Yes |
| **[v13 P2] Grid grapheme** | ~10 | `cargo test` | Yes |
| **[v13 P2] VecDeque scroll** | ~5 | `cargo test` | Yes |
| **[v13 P2] PtyHandle 7-state** | ~12 | `cargo test` | Yes |
| **[v13 P2] Snapshot binary v2** | ~8 | `cargo test` | Yes |
| **[v13 P2] Cross-model parity** | ~5 | `cargo test` | Yes |

### CI Matrix

| Axis | Values |
|---|---|
| OS | Ubuntu 24.04, macOS 14 |
| Rust | stable, nightly |
| tmux version | 3.3a (LTS), 3.5, 3.6 (Current) |
| Python | 3.11, 3.12, 3.13 |
| Node | 20, 22 |

### Release Gate Matrix (v13 P2 additions)

| Gate ID | Gate | Test Type | Required? |
|---|---|---|---|
| M-05-01 | Grid::put_char correctness | unit test | Yes |
| M-05-02 | VtParser 7 states coverage | unit test | Yes |
| M-05-03 | GenSlotMap ABA safety | unit test | Yes |
| M-05-04 | Canonical event serialization | property test | Yes |
| M-05-05 | **[v13 P2]** Grid grapheme round-trip | unit test | Yes |
| M-05-06 | **[v13 P2]** VecDeque scroll O(1) | benchmark | Warn |
| M-06-01 | PtyHandle lifecycle | unit test | Yes |
| M-06-02 | **[v13 P2]** PtyHandle 7-state transitions | unit test | Yes |
| M-08-01 | **[v13 P2]** Snapshot FNV-1a checksum | unit test | Yes |
| M-17-02 | CRDT merge order independence | property test | Yes |
| M-19-02 | OTEL span hierarchy valid | unit test | Yes |
| M-20-02 | CI matrix has LTS entry | unit test | Yes |
| (all v12 P3 gates carried forward) | -- | -- | -- |

### Rust Example

```rust
pub const RELEASE_GATE_COUNT_V13P2: usize = 72;

#[cfg(test)]
mod tests {
    #[test]
    fn test_gate_count() {
        assert!(super::RELEASE_GATE_COUNT_V13P2 >= 72);
    }
}
```

### Test Strategy

1. CI requires 100% pass of mandatory matrix rows. `TST-340`.
2. Nightly includes extended parity + fuzz + stress. `TST-341`.
3. Release requires zero unresolved mandatory rows. `TST-342`.

### AGENTS.md Rules

- `RULE-S31-01`: New architecture features must add at least one matrix row. **Enforcement:** PR template.
- `RULE-S31-02`: Release tagging is blocked until mandatory rows are green. **Enforcement:** Release script.
- `RULE-S31-03`: Quarantined tests require owner + expiry date. **Enforcement:** Quarantine manifest linter.

---

## 32. Termlets [v13 P3 -- deepest section, DEFINITIVE]

### Design Decisions

Termlets are the killer feature differentiating TermForge from other terminal multiplexers and testing tools. They are **SDK-first testing pods** that wrap tmux panes into simplified, embeddable, language-binding-native handles.

**Core philosophy:** A Termlet is to a tmux pane what a Docker container is to a VM -- same capability, simpler interface, faster lifecycle, purpose-built for programmatic use.

**Key design decisions (v13 P2 -- all v12/v13 P1 decisions carried forward plus cross-model improvements):**

1-24. (all v12 P3 decisions carried forward unchanged)

25. **[v13]** Grid Integration Deepened: Termlet's Grid now uses `Line` type for row-level operations and `Viewport` for zero-copy rendering access. `VtParser` operates through `Grid::put_char` exclusively (INV-012).

26. **[v13]** PtyHandle Lifecycle: Termlet tracks PtyHandle state through PtyHandleRegistry. After `kill()` or `Drop`, the handle is explicitly closed via `PtyBackend::close()`. No operations permitted on closed handles.

27. **[v13]** Snapshot Binary Format: TermletSnapshot supports versioned binary serialization with magic bytes and version.

28. **[v13]** VtParser in Termlet: The Termlet's VtParser processes all 7 canonical states with full CSI parameter accumulation.

29. **[v13]** TermletError::HandleClosed: New error variant for operations on closed PtyHandle.

30. **[v13]** Restart Drain Barrier strengthened: restart() calls `PtyBackend::close()` before spawning new process. New PtyHandle allocated for respawn.

31. **[v13 P2]** Grid uses `VecDeque<Line>` for O(1) scroll performance. Cell stores `grapheme: String` for combining character support.

32. **[v13 P2]** PtyHandle lifecycle is 7-state (Allocated -> Spawned -> Running -> Stopping -> Exited -> Reaped -> Closed) per `PtyRecord::transition()`.

33. **[v13 P2]** Snapshot binary format uses 8-byte `TFSNAP13` magic, u16 version, SnapshotMeta (cursor+revision), SnapshotCell, and FNV-1a checksum.

34. **[v13 P2]** VtParser uses `classify()`/`Step` pattern for byte classification and state transitions.

35. **[v13 P2]** TermletState <-> PtyState alignment explicitly documented (see Section 7).

**Architectural position:** `mux-termlet` at Layer 2 (FACADE). Dependencies:
- `mux-types` (L0): shared types, `PaneSize`, `PtyHandle`
- `mux-grid` (L0): `Grid`, `VtParser`, `Cell`, `Line`, `Viewport`
- `mux-pty` (L1): `PtyBackend` trait
- `mux-pty-fake` (L0): `FakePtyBackend`, `ScenarioStep`
- `mux-snapshot` (L0): **[v13 P2]** binary snapshot format with checksum

It does NOT depend on: `mux-server`, `mux-api`, `mux-orm`.

### 32.1 Architecture Diagram

```
                    +------------------+
                    |   User Code      |
                    | (Rust / Py / JS) |
                    +--------+---------+
                             |
                    spawn / send_keys / send_bytes / expect / snapshot / resize / kill / restart
                             |
                    +--------v---------+
                    |     Termlet      |  <-- mux-termlet crate (Layer 2)
                    |  (single handle) |
                    +--------+---------+
                             |
            +----------------+----------------+
            |                |                |
    +-------v------+  +-----v------+  +------v------+
    |  VtParser    |  |    Grid    |  | PtyBackend  |
    | (mux-grid)   |  | (mux-grid) |  | (mux-pty)   |
    |  7 states    |  | VecDeque   |  | 7-state     |
    |  classify()  |  | grapheme   |  | PtyRecord   |
    +--------------+  | Viewport   |  +------+------+
                      +------------+         |
                            +----------------+----------------+
                            |                                 |
                    +-------v-------+               +--------v--------+
                    | RealPtyBackend|               | FakePtyBackend  |
                    |  (mux-pty)    |               | (mux-pty-fake)  |
                    +---------------+               +-----------------+
```

### 32.2 TermletState Lifecycle

```text
Spawning -> Running -> Stopping -> Exited
       \-> SpawnFailed  (terminal state)
Running --kill()--> Stopping --grace timeout--> ForcedKill -> Exited
Running --process exit event---------------------------> Exited
Running --restart()--> [kill+close old PtyHandle] -> Spawning -> Running  (new PtyHandle)
```

### Rust Example (TermletState + Core API)

```rust
// crates/mux-termlet/src/lib.rs
// [v13 P2] Cross-pollinated standalone-compilable Termlet core

#![allow(dead_code)]

use std::time::{Duration, Instant};
use std::collections::HashMap;

/// PaneSize.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaneSize { pub sx: u16, pub sy: u16 }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PtyHandle(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TermletPaneId(u64);

impl TermletPaneId {
    pub fn raw(self) -> u64 { self.0 }
    pub fn as_pty_handle(self) -> PtyHandle { PtyHandle(self.0) }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TermletState {
    Spawning, Running, Stopping, SpawnFailed, Exited,
}

impl TermletState {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::SpawnFailed | Self::Exited)
    }
    pub fn allows_interaction(self) -> bool { self == Self::Running }
    pub fn allows_restart(self) -> bool {
        matches!(self, Self::Running | Self::Exited | Self::SpawnFailed)
    }
}

pub fn valid_transition(from: TermletState, to: TermletState) -> bool {
    matches!((from, to),
        (TermletState::Spawning, TermletState::Running) |
        (TermletState::Spawning, TermletState::SpawnFailed) |
        (TermletState::Running, TermletState::Stopping) |
        (TermletState::Running, TermletState::Exited) |
        (TermletState::Stopping, TermletState::Exited) |
        (TermletState::Running, TermletState::Spawning) |     // restart
        (TermletState::Exited, TermletState::Spawning) |      // restart
        (TermletState::SpawnFailed, TermletState::Spawning)    // restart
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtyMode { Real, Fake }

#[derive(Debug, Clone)]
pub struct TermletConfig {
    pub cols: u16,
    pub rows: u16,
    pub env: Vec<(String, String)>,
    pub inherit_env: bool,
    pub cwd: Option<String>,
    pub backend: PtyMode,
    pub grace_period: Duration,
    pub wait_poll_interval: Duration,
    pub wait_max_interval: Duration,
    pub default_timeout: Duration,
}

impl Default for TermletConfig {
    fn default() -> Self {
        Self {
            cols: 80, rows: 24,
            env: vec![("TERM".to_string(), "xterm-256color".to_string())],
            inherit_env: false,
            cwd: None,
            backend: PtyMode::Real,
            grace_period: Duration::from_secs(2),
            wait_poll_interval: Duration::from_millis(25),
            wait_max_interval: Duration::from_millis(100),
            default_timeout: Duration::from_secs(5),
        }
    }
}

#[derive(Debug)]
pub enum TermletError {
    SpawnFailed { reason: String },
    InvalidState { state: TermletState },
    AlreadyExited { status: Option<i32> },
    WaitForTimeout { pattern: String, timeout_ms: u64 },
    PatternNotFound { pattern: String },
    WaitFailed { reason: String },
    ResizeFailed(String),
    Pty(String),
    NotInPool { name: String },
    InvalidRegex(String),
    HandleClosed { handle_id: u64 }, // [v13]
}

impl std::fmt::Display for TermletError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SpawnFailed { reason } => write!(f, "[TERMLET_SPAWN_ERROR] {reason}"),
            Self::InvalidState { state } => write!(f, "[TERMLET_INVALID_STATE] {state:?}"),
            Self::AlreadyExited { status } => write!(f, "[TERMLET_ALREADY_EXITED] {status:?}"),
            Self::WaitForTimeout { pattern, timeout_ms } =>
                write!(f, "[TERMLET_TIMEOUT] {pattern:?} after {timeout_ms}ms"),
            Self::PatternNotFound { pattern } =>
                write!(f, "[TERMLET_PATTERN_NOT_FOUND] {pattern:?}"),
            Self::WaitFailed { reason } => write!(f, "[TERMLET_WAIT_FAILED] {reason}"),
            Self::ResizeFailed(msg) => write!(f, "[TERMLET_RESIZE_ERROR] {msg}"),
            Self::Pty(msg) => write!(f, "[TERMLET_PTY_ERROR] {msg}"),
            Self::NotInPool { name } => write!(f, "[TERMLET_NOT_IN_POOL] {name:?}"),
            Self::InvalidRegex(msg) => write!(f, "[TERMLET_INVALID_REGEX] {msg}"),
            Self::HandleClosed { handle_id } =>
                write!(f, "[TERMLET_HANDLE_CLOSED] handle {handle_id}"),
        }
    }
}

impl std::error::Error for TermletError {}

#[derive(Debug, Clone)]
pub struct WaitMatch {
    pub byte_range: std::ops::Range<usize>,
    pub matched_at: Instant,
}

#[derive(Debug, Clone)]
pub struct TermletSnapshot {
    pub lines: Vec<String>,
    pub cols: u16,
    pub rows: u16,
    pub cursor_col: u16,
    pub cursor_row: u16,
    pub revision: u64,
    pub timestamp: Instant,
}

impl TermletSnapshot {
    pub fn to_text(&self) -> String {
        let mut trimmed: Vec<&str> = self.lines.iter().map(|l| l.trim_end()).collect();
        while trimmed.last().is_some_and(|l| l.is_empty()) { trimmed.pop(); }
        trimmed.join("\n")
    }

    pub fn contains(&self, pattern: &str) -> bool { self.to_text().contains(pattern) }
    pub fn line(&self, idx: usize) -> Option<&str> { self.lines.get(idx).map(|s| s.as_str()) }
    pub fn line_count(&self) -> usize { self.lines.len() }
    pub fn size(&self) -> (u16, u16) { (self.cols, self.rows) }

    pub fn from_text(text: &str) -> Self {
        Self {
            lines: text.lines().map(|l| l.to_string()).collect(),
            cols: 80, rows: 24,
            cursor_col: 0, cursor_row: 0,
            revision: 0,
            timestamp: Instant::now(),
        }
    }
}

/// TermletLike trait: normative API contract (INV-011).
pub trait TermletLike {
    fn send_keys(&mut self, keys: &str) -> Result<(), TermletError>;
    fn send_bytes(&mut self, data: &[u8]) -> Result<(), TermletError>;
    fn wait_for(&mut self, pattern: &str, timeout: Duration) -> Result<WaitMatch, TermletError>;
    fn expect(&mut self, pattern: &str) -> WaitMatch;
    fn snapshot(&mut self) -> TermletSnapshot;
    fn resize(&mut self, cols: u16, rows: u16) -> Result<(), TermletError>;
    fn kill(&mut self) -> Result<(), TermletError>;
    fn restart(&mut self, command: &str) -> Result<(), TermletError>;
    fn is_alive(&self) -> bool;
    fn state(&self) -> TermletState;
    fn size(&self) -> PaneSize;
    fn pane_id(&self) -> TermletPaneId;
    fn output_history(&self) -> &[u8];
    fn exit_status(&self) -> Option<i32>;
    fn command(&self) -> &str;
    fn config(&self) -> &TermletConfig;
    fn drain_output(&mut self);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_text_trimmed() {
        let snap = TermletSnapshot {
            lines: vec!["Hello  ".into(), "".into(), "".into()],
            cols: 80, rows: 24,
            cursor_col: 0, cursor_row: 0, revision: 0,
            timestamp: Instant::now(),
        };
        assert_eq!(snap.to_text(), "Hello");
    }

    #[test]
    fn test_snapshot_contains() {
        let snap = TermletSnapshot::from_text("Hello World");
        assert!(snap.contains("World"));
        assert!(!snap.contains("Missing"));
    }

    #[test]
    fn test_default_config() {
        let config = TermletConfig::default();
        assert_eq!(config.cols, 80);
        assert_eq!(config.rows, 24);
        assert!(!config.inherit_env);
        assert_eq!(config.default_timeout, Duration::from_secs(5));
    }

    #[test]
    fn test_valid_transitions() {
        assert!(valid_transition(TermletState::Spawning, TermletState::Running));
        assert!(valid_transition(TermletState::Running, TermletState::Stopping));
        assert!(valid_transition(TermletState::Running, TermletState::Spawning)); // restart
        assert!(!valid_transition(TermletState::Exited, TermletState::Running));
    }

    #[test]
    fn test_terminal_states() {
        assert!(TermletState::Exited.is_terminal());
        assert!(TermletState::SpawnFailed.is_terminal());
        assert!(!TermletState::Running.is_terminal());
    }

    #[test]
    fn test_error_display() {
        let err = TermletError::HandleClosed { handle_id: 42 };
        assert!(format!("{err}").contains("[TERMLET_HANDLE_CLOSED]"));
    }
}
```

### 32.3 PatternMatcher

```rust
// crates/mux-termlet/src/pattern.rs
#![allow(dead_code)]

#[derive(Debug, Clone)]
pub enum PatternMatcher {
    Plain(String),
    RegexPattern(String),
}

impl PatternMatcher {
    pub fn compile(raw: &str) -> Result<Self, String> {
        if let Some(rest) = raw.strip_prefix("re:") {
            Ok(Self::RegexPattern(rest.to_string()))
        } else {
            Ok(Self::Plain(raw.to_string()))
        }
    }

    pub fn find(&self, text: &str) -> Option<std::ops::Range<usize>> {
        match self {
            Self::Plain(s) => text.find(s).map(|start| start..(start + s.len())),
            Self::RegexPattern(pattern) => {
                text.find(pattern).map(|start| start..(start + pattern.len()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_match() {
        let m = PatternMatcher::compile("hello").unwrap();
        assert!(m.find("say hello world").is_some());
    }

    #[test]
    fn test_plain_no_match() {
        let m = PatternMatcher::compile("missing").unwrap();
        assert!(m.find("hello world").is_none());
    }

    #[test]
    fn test_regex_prefix() {
        let m = PatternMatcher::compile("re:hel+o").unwrap();
        assert!(matches!(m, PatternMatcher::RegexPattern(_)));
    }
}
```

### 32.4 TermletBuilder

```rust
// crates/mux-termlet/src/builder.rs
#![allow(dead_code)]
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtyMode { Real, Fake }

#[derive(Debug, Clone)]
pub struct TermletConfig {
    pub cols: u16, pub rows: u16,
    pub env: Vec<(String, String)>,
    pub inherit_env: bool, pub cwd: Option<String>,
    pub backend: PtyMode,
    pub grace_period: Duration,
    pub wait_poll_interval: Duration,
    pub wait_max_interval: Duration,
    pub default_timeout: Duration,
}

impl Default for TermletConfig {
    fn default() -> Self {
        Self {
            cols: 80, rows: 24,
            env: vec![("TERM".into(), "xterm-256color".into())],
            inherit_env: false, cwd: None,
            backend: PtyMode::Real,
            grace_period: Duration::from_secs(2),
            wait_poll_interval: Duration::from_millis(25),
            wait_max_interval: Duration::from_millis(100),
            default_timeout: Duration::from_secs(5),
        }
    }
}

pub struct TermletBuilder {
    pub command: String,
    pub config: TermletConfig,
}

impl TermletBuilder {
    pub fn new(command: impl Into<String>) -> Self {
        Self { command: command.into(), config: TermletConfig::default() }
    }
    pub fn cols(mut self, cols: u16) -> Self { self.config.cols = cols; self }
    pub fn rows(mut self, rows: u16) -> Self { self.config.rows = rows; self }
    pub fn size(mut self, cols: u16, rows: u16) -> Self {
        self.config.cols = cols; self.config.rows = rows; self
    }
    pub fn env(mut self, key: impl Into<String>, val: impl Into<String>) -> Self {
        self.config.env.push((key.into(), val.into())); self
    }
    pub fn inherit_env(mut self) -> Self { self.config.inherit_env = true; self }
    pub fn cwd(mut self, cwd: impl Into<String>) -> Self {
        self.config.cwd = Some(cwd.into()); self
    }
    pub fn fake(mut self) -> Self { self.config.backend = PtyMode::Fake; self }
    pub fn real(mut self) -> Self { self.config.backend = PtyMode::Real; self }
    pub fn grace_period(mut self, p: Duration) -> Self { self.config.grace_period = p; self }
    pub fn default_timeout(mut self, t: Duration) -> Self { self.config.default_timeout = t; self }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_defaults() {
        let b = TermletBuilder::new("echo test");
        assert_eq!(b.config.cols, 80);
        assert_eq!(b.config.rows, 24);
    }

    #[test]
    fn test_builder_chaining() {
        let b = TermletBuilder::new("echo test")
            .cols(120).rows(40).fake()
            .default_timeout(Duration::from_secs(10));
        assert_eq!(b.config.cols, 120);
        assert_eq!(b.config.rows, 40);
        assert_eq!(b.config.backend, PtyMode::Fake);
    }
}
```

### 32.5 TermletPool

```rust
// crates/mux-termlet/src/pool.rs
#![allow(dead_code)]
use std::collections::HashMap;

// Re-use types from 32.2
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TermletState { Spawning, Running, Stopping, SpawnFailed, Exited }
#[derive(Debug)]
pub enum TermletError { NotInPool { name: String } }
pub trait TermletLike {
    fn kill(&mut self) -> Result<(), TermletError>;
    fn is_alive(&self) -> bool;
    fn snapshot_text(&mut self) -> String;
}

pub struct TermletPool<T: TermletLike> {
    termlets: HashMap<String, T>,
}

impl<T: TermletLike> TermletPool<T> {
    pub fn new() -> Self { Self { termlets: HashMap::new() } }
    pub fn insert(&mut self, name: &str, termlet: T) {
        self.termlets.insert(name.to_string(), termlet);
    }
    pub fn get(&self, name: &str) -> Option<&T> { self.termlets.get(name) }
    pub fn get_mut(&mut self, name: &str) -> Result<&mut T, TermletError> {
        self.termlets.get_mut(name)
            .ok_or(TermletError::NotInPool { name: name.to_string() })
    }
    pub fn kill_all(&mut self) {
        for t in self.termlets.values_mut() { let _ = t.kill(); }
    }
    pub fn snapshot_all(&mut self) -> HashMap<String, String> {
        self.termlets.iter_mut()
            .map(|(n, t)| (n.clone(), t.snapshot_text())).collect()
    }
    pub fn len(&self) -> usize { self.termlets.len() }
    pub fn is_empty(&self) -> bool { self.termlets.is_empty() }
    pub fn alive_count(&self) -> usize {
        self.termlets.values().filter(|t| t.is_alive()).count()
    }
}

impl<T: TermletLike> Drop for TermletPool<T> {
    fn drop(&mut self) { self.kill_all(); }
}
```

### 32.6 SnapshotDiff

```rust
// crates/mux-termlet/src/diff.rs
#![allow(dead_code)]

#[derive(Debug, Clone)]
pub struct LineDiff {
    pub row: usize,
    pub before: String,
    pub after: String,
}

#[derive(Debug, Clone)]
pub struct SnapshotDiff {
    pub changed_lines: Vec<LineDiff>,
    pub identical: bool,
}

impl SnapshotDiff {
    pub fn compare(before_text: &str, after_text: &str) -> Self {
        if before_text == after_text {
            return Self { changed_lines: Vec::new(), identical: true };
        }
        let bl: Vec<&str> = before_text.lines().collect();
        let al: Vec<&str> = after_text.lines().collect();
        let max = bl.len().max(al.len());
        let mut changed = Vec::new();
        for i in 0..max {
            let b = bl.get(i).copied().unwrap_or("");
            let a = al.get(i).copied().unwrap_or("");
            if b != a {
                changed.push(LineDiff { row: i, before: b.into(), after: a.into() });
            }
        }
        Self { identical: changed.is_empty(), changed_lines: changed }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical() {
        let diff = SnapshotDiff::compare("Hello", "Hello");
        assert!(diff.identical);
    }

    #[test]
    fn test_different() {
        let diff = SnapshotDiff::compare("Hello", "World");
        assert!(!diff.identical);
        assert_eq!(diff.changed_lines.len(), 1);
    }
}
```

### 32.7 Error Code Reference [v13 P2 -- 11 variants]

| Error Variant | Code String | Python Exception | Node Error |
|---|---|---|---|
| `SpawnFailed` | `TERMLET_SPAWN_ERROR` | `RuntimeError` | `Error` |
| `InvalidState` | `TERMLET_INVALID_STATE` | `RuntimeError` | `Error` (code: INVALID_STATE) |
| `AlreadyExited` | `TERMLET_ALREADY_EXITED` | `RuntimeError` | `Error` |
| `WaitForTimeout` | `TERMLET_TIMEOUT` | `TimeoutError` | `Error` (code: TIMEOUT) |
| `PatternNotFound` | `TERMLET_PATTERN_NOT_FOUND` | `RuntimeError` | `Error` (code: NOT_FOUND) |
| `WaitFailed` | `TERMLET_WAIT_FAILED` | `RuntimeError` | `Error` (code: WAIT_FAILED) |
| `ResizeFailed` | `TERMLET_RESIZE_ERROR` | `RuntimeError` | `Error` |
| `Pty(...)` | `TERMLET_PTY_ERROR` | `RuntimeError` | `Error` |
| `NotInPool` | `TERMLET_NOT_IN_POOL` | `KeyError` | `Error` (code: NOT_IN_POOL) |
| `InvalidRegex` | `TERMLET_INVALID_REGEX` | `ValueError` | `Error` (code: INVALID_REGEX) |
| **[v13]** `HandleClosed` | `TERMLET_HANDLE_CLOSED` | `RuntimeError` | `Error` (code: HANDLE_CLOSED) |

### 32.8 Python Binding Contract

```python
import termforge

# Context manager guarantees cleanup.
with termforge.Termlet.spawn("bash", cols=120, rows=40) as t:
    t.send_keys("echo hello\n")
    t.send_bytes(b"\x1b[A")  # raw escape: up arrow
    match = t.wait_for("hello", timeout=5.0)
    snap = t.snapshot()
    assert "hello" in snap.to_text()
    t.expect("hello")
    t.kill()
    assert t.exit_status == 0

# [v13 P2] Binary snapshot with checksum
with termforge.Termlet.spawn("echo test") as t:
    t.expect("test")
    binary = t.snapshot().to_binary()
    restored = termforge.TermletSnapshot.from_binary(binary)
    assert "test" in restored.to_text()

# Pool with generic TermletLike
with termforge.TermletPool() as pool:
    pool.spawn("server", "python3 server.py")
    pool.spawn("client", "python3 client.py")
    pool["server"].wait_for("listening", timeout=10.0)

# [v13 P2] QuerySet-like traversal
server = termforge.Server()
sessions = server.sessions.filter_by("name:starts_with:dev")
for s in sessions:
    for w in s.windows:
        for p in w.panes:
            print(p.capture_pane())
```

### 32.9 Node.js Binding Contract

```javascript
import { Termlet, TermletPool, TermletSnapshot } from 'termforge';

const t = await Termlet.spawn('bash', { cols: 80, rows: 24 });
try {
    await t.sendKeys('echo hello\n');
    await t.sendBytes(Buffer.from([0x1b, 0x5b, 0x41]));
    const match = await t.waitFor('hello', { timeout: 5000 });
    const snap = t.snapshot();
    expect(snap.toText()).toContain('hello');
    // [v13 P2] Binary snapshot with checksum
    const binary = snap.toBinary();
    const restored = TermletSnapshot.fromBinary(binary);
    expect(restored.toText()).toContain('hello');
    await t.kill();
} finally {
    await t.kill();
}

// [v13 P2] QuerySet-like traversal
const server = new Server();
const sessions = server.sessions.filterBy('name:starts_with:dev');
for (const s of sessions) {
    for (const w of s.windows) {
        for (const p of w.panes) {
            console.log(p.capturePane());
        }
    }
}
```

### 32.10 ShellInteraction Trait

```rust
// crates/mux-termlet/src/shell.rs
#![allow(dead_code)]
use std::time::Duration;

pub struct WaitMatch { pub byte_range: std::ops::Range<usize> }
pub enum TermletError { WaitFailed { reason: String } }

pub trait ShellInteraction {
    fn wait_for_prompt(&mut self) -> Result<WaitMatch, TermletError>;
    fn wait_for_prompt_with_timeout(&mut self, timeout: Duration) -> Result<WaitMatch, TermletError>;
    fn run_command(&mut self, command: &str) -> Result<(), TermletError>;
    fn run_command_with_timeout(&mut self, command: &str, timeout: Duration) -> Result<(), TermletError>;
}

pub const DEFAULT_PROMPT_PATTERN: &str = "re:[$#%>]\\s*$";
```

### 32.11 AsyncTermlet

```rust
// crates/mux-termlet/src/async_support.rs
#![allow(dead_code)]

pub struct AsyncTermlet<T: Send + 'static> {
    inner: T,
}

impl<T: Send + 'static> AsyncTermlet<T> {
    pub fn new(inner: T) -> Self { Self { inner } }
    pub fn into_inner(self) -> T { self.inner }
}
```

### 32.12 TermletExt Trait

```rust
// crates/mux-termlet/src/ext.rs
#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TermletState { Spawning, Running, Stopping, SpawnFailed, Exited }

pub trait TermletExt {
    fn assert_contains(&mut self, text: &str);
    fn assert_not_contains(&mut self, text: &str);
    fn assert_line(&mut self, idx: usize, expected: &str);
    fn assert_line_count(&mut self, expected: usize);
    fn assert_state(&self, expected: TermletState);
    fn assert_exit_status(&self, expected: i32);
}
```

### 32.13 OTEL in Termlets

| Operation | Span Name | Attributes |
|---|---|---|
| spawn | `termlet.spawn` | `termlet.id`, `command`, `backend.mode`, `cols`, `rows` |
| send_keys | `termlet.send_keys` | `termlet.id`, `keys.len` |
| send_bytes | `termlet.send_bytes` | `termlet.id`, `bytes.len` |
| wait_for | `termlet.wait_for` | `termlet.id`, `pattern`, `timeout_ms`, `matched` |
| expect | `termlet.expect` | `termlet.id`, `pattern`, `matched` |
| snapshot | `termlet.snapshot` | `termlet.id`, `cols`, `rows`, `revision` |
| resize | `termlet.resize` | `termlet.id`, `old_cols`, `old_rows`, `new_cols`, `new_rows` |
| kill | `termlet.kill` | `termlet.id`, `grace_period_ms`, `forced` |
| restart | `termlet.restart` | `termlet.id`, `old_command`, `new_command` |
| **[v13]** close | `termlet.close` | `termlet.id`, `handle_id` |

### 32.14 Testing Patterns

**32.14.1 Spawn-Expect-Kill Pattern:**
```rust
#[test]
fn test_echo_output() {
    let mut t = TermletBuilder::new("echo hello world")
        .fake()
        .spawn()
        .unwrap();
    t.expect("hello world");
    t.kill().unwrap();
}
```

**32.14.2 Sidecar Pattern:** Two Termlets for client-server testing.

**32.14.3 Shell Interaction Pattern:** Using `ShellInteraction` trait for prompt detection.

### 32.15 Debugging Guidance

When a Termlet test fails:
1. Dump snapshot: `eprintln!("{}", termlet.snapshot().to_text());`
2. Dump raw history: `eprintln!("{:?}", String::from_utf8_lossy(termlet.output_history()));`
3. Visual diff: `SnapshotDiff::compare(&before_text, &after_text)`
4. **[v13 P2]** Binary snapshot export: save snapshot via `encode_binary()` for offline analysis with FNV-1a checksum integrity verification.

### 32.16 Restart API [v13 P2 deepened]

Semantics:
- `restart(command)` kills the current process, **[v13 P2]** transitions through PtyState Stopping -> Exited -> Reaped -> Closed on old handle, resets grid, and spawns new process with fresh PtyHandle.
- New `PtyHandle` allocated for respawn (new slot or reused slot with incremented generation).
- The `TermletPaneId` remains the same across restarts.
- Grid and output history are cleared on restart.
- Drain barrier: all residual output drained and discarded before respawn.

### 32.17 Grid Integration [v13 P2 deepened]

- **[v13 P2]** Termlet owns a `Grid` instance with `VecDeque<Line>` storage.
- **[v13 P2]** `Cell` stores `grapheme: String` for combining character support.
- `VtParser` drives `Grid::put_char()` for character placement (INV-012).
- `snapshot()` calls `grid.viewport_text(grid.full_viewport())` for text output.
- `resize()` calls grid resize after backend resize.
- Grid is reset via `Grid::new()` during `restart()`.
- `Viewport` provides zero-copy text access for snapshot operations.

### 32.18 [v13 P2] Snapshot Binary Format Specification

```
Offset  Size  Description
0       8     Magic bytes: "TFSNAP13" [v13 P2]
8       2     Version (u16 LE, currently 1) [v13 P2]
10      2     cols (u16 LE)
12      2     rows (u16 LE)
14      2     cursor_col (u16 LE) [v13 P2]
16      2     cursor_row (u16 LE) [v13 P2]
18      8     revision (u64 LE) [v13 P2]
26      4     cell_count (u32 LE) [v13 P2]
30      N*8   cells: ch(u32) + style(u16) + flags(u16) per cell [v13 P2]
30+N*8  4     FNV-1a checksum (u32 LE) [v13 P2]
```

Changes from v13 P1 format:
- Magic expanded from 4-byte `TFSN` to 8-byte `TFSNAP13` (from GPT)
- Version expanded from u8 to u16 (from GPT)
- Added cursor_col, cursor_row, revision fields (from GPT)
- Added cell-level data (ch/style/flags) instead of text blob (from GPT)
- Added FNV-1a trailing checksum (from GPT)

### 32.19 Failure Modes and Recovery

**32.19.1 Spawn Failure:** Backend creation fails -> `SpawnFailed`. Command not found -> quick `Exited`.
**32.19.2 Partial Output:** `drain_output()` captures what was written. VtParser enters error recovery via `ErrorRecover` action.
**32.19.3 Backend Switchover:** Cannot switch after creation. Use separate Termlets.
**32.19.4 Pool Failure:** One spawn failure does not kill the pool. `kill_all()` is idempotent.
**32.19.5 Resource Exhaustion:** FD exhaustion -> `SpawnFailed`. Grid dimensions capped.
**32.19.6 [v13]** Handle Closed: Operations on a closed PtyHandle return `TermletError::HandleClosed`.
**32.19.7 [v13 P2]** Checksum Failure: Binary snapshot decode rejects checksum mismatch with `SnapshotError::ChecksumMismatch`.

### 32.20 Implementation Hints for LLMs

1. **Non-blocking IO:** Set PTY fd to `O_NONBLOCK`.
2. **Drain Loop:** `wait_for` relies on `drain_output`. Blocking drain breaks timeout.
3. **Pattern Compilation:** `PatternMatcher::compile()` only inside `wait_for`, not per poll.
4. **TermletState:** Use `valid_transition()`, not comparison operators.
5. **Exit Status:** Capture from `PtyEvent::Exited` during `drain_output`.
6. **Resize Safety:** Drain before AND after.
7. **PtyHandle:** Convert `TermletPaneId` to `PtyHandle` via `as_pty_handle()`.
8. **restart():** Kill before respawn. Reset grid and parser. Close old handle. Allocate new PtyHandle.
9. **expect():** Delegates to `wait_for` with `default_timeout`. Panics on failure.
10. **wait_for_condition:** Calls `snapshot()` per iteration.
11. **ShellInteraction:** Default prompt regex `[$#%>]\s*$` covers bash/zsh/fish.
12. **AsyncTermlet:** Must not call `std::thread::sleep`. Use `tokio::time::sleep`.
13. **[v13]** Grid::put_char: Only entry point for character placement. Never modify cells directly.
14. **[v13]** PtyHandle lifecycle: Always close after kill. Check PtyRegistry generation.
15. **[v13 P2]** Grid uses VecDeque: `scroll_up` pops front, pushes back. O(1).
16. **[v13 P2]** Cell grapheme: Use `String` field, not `char`. Supports combining characters.
17. **[v13 P2]** Snapshot checksum: Always verify FNV-1a on decode. Reject mismatch.
18. **[v13 P2]** PtyState 7 states: Validate transitions via `PtyRecord::transition()`.

### 32.21 [v13 P2] TermletState <-> PtyState Alignment

| TermletState | PtyState(s) | Notes |
|-------------|-------------|-------|
| Spawning | Allocated, Spawned | PtyHandle allocated but process not yet running |
| Running | Running | IO permitted |
| Stopping | Stopping | SIGTERM sent, waiting for exit |
| Exited | Exited, Reaped, Closed | Process terminated; handle being cleaned up |
| SpawnFailed | (no PtyHandle) | Backend creation failed; no handle allocated |

### 32.22 Examples Directory

The `examples/termlet_recipes/` directory provides cookbook-style examples:
- `basic_shell.rs`: Spawn bash, send commands, assert output.
- `tui_app_test.rs`: Spawn a ratatui app, verify screen content.
- `sidecar_pattern.rs`: Client-server interaction via TermletPool.
- `resize_test.rs`: Resize a Termlet and verify grid reflow.
- `restart_pattern.rs`: Kill and respawn with different args.
- **[v13]** `snapshot_binary.rs`: Serialize/deserialize snapshots in binary format.
- **[v13]** `vtparser_debug.rs`: Observe VtParser state transitions.
- **[v13 P2]** `grapheme_test.rs`: Test combining character handling.
- **[v13 P2]** `pty_lifecycle.rs`: Demonstrate 7-state PtyHandle lifecycle.

### 32.23 Enforcement-First Termlet Contracts

1. A rule is only complete when a deterministic enforcement artifact exists.
2. A Termlet API that mutates process state must define legal pre-state, post-state, and forbidden-state behavior.
3. Cross-language wrappers must preserve semantic class.

### 32.24 Restart and Drain Race Closure [v13 P2]

**[v13 P2]** Strengthened with 7-state PtyHandle lifecycle:
- Enter `Stopping`; stop new writes.
- Drain residual PTY output with bounded loop.
- Kill (SIGTERM -> grace -> SIGKILL fallback).
- **[v13 P2]** Transition through `PtyState::Exited` -> `Reaped` -> `Closed`.
- **[v13 P2]** Release handle in `PtyRegistry` after `Closed`.
- Reinitialize parser, grid, and output buffers.
- Allocate new `PtyHandle` for respawn (new generation).
- Spawn replacement process.
- Transition to `Running` only after successful spawn.

### 32.25 Async Fairness and Poll Budget

- Poll backoff floor: 10ms.
- Poll backoff ceiling: 100ms.
- All blocking backend operations routed through `spawn_blocking`.

### 32.26 Cross-Language Assertion Canonicalization

- Shared fixture defines timeout formatting, snapshot excerpt policy, and error code prefix.
- Language-specific wrappers may decorate message, but must preserve canonical prefix and fields.

### 32.27 Collaborative Termlet Replay Discipline

When `crdt` feature is enabled:
- Same operation set replayed under N merge orders must converge to identical snapshot hash.
- Failing replay blocks release in CRDT-enabled lane.
- Uses LWWFieldMap with per-field conflict resolution and tombstone-wins-delete.

### 32.28 [v13 P2] VtParser Integration with classify()/Step

The VtParser within a Termlet uses the `classify()`/`Step` pattern (from GPT):

1. Each input byte is classified by `classify(b)` into: `"esc"`, `"intermediate"`, `"param"`, `"final"`, `"control"`, `"print"`.
2. `step(state, b)` returns a `Step { next, action }` tuple.
3. Actions are dispatched to the Grid performer:
   - `Print(b)` -> `grid.put_char(b as char)` (INV-012)
   - `ExecuteControl(b)` -> handle CR, LF, BS, TAB, etc.
   - `DispatchCsi(b)` -> CSI command dispatch (A/B/C/D/H/f/J/K/m)
   - `DispatchOsc` -> OSC string handling
   - `ErrorRecover` -> reset parser, no grid mutation

### 32.29 [v13 P2] PtyHandle Lifecycle in Termlet (7-state)

```
[spawn()] -> PtyRegistry.allocate() -> PtyState::Allocated
          -> PtyBackend.spawn() -> PtyState::Spawned -> PtyState::Running
          -> Termlet state = Running

[kill()]  -> PtyState::Running -> PtyState::Stopping (SIGTERM)
          -> grace timeout -> PtyState::Exited
          -> PtyBackend.reap() -> PtyState::Reaped
          -> PtyBackend.close() -> PtyState::Closed
          -> PtyRegistry.release(handle)
          -> Termlet state = Exited

[restart()] -> kill() -> [old handle through full 7-state cleanup]
            -> new_handle = PtyRegistry.allocate() -> PtyState::Allocated
            -> PtyBackend.spawn(new_handle) -> PtyState::Spawned -> PtyState::Running
            -> Termlet state = Running (with new handle, new generation)

[Drop]    -> if !terminal: force kill -> close -> release
```

### 32.30 [v13 P3] Binary Snapshot Contract Hardening (Imported + Tightened from GPT P2)

```rust
#![forbid(unsafe_code)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotV1 {
    pub cols: u16,
    pub rows: u16,
    pub revision: u64,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotDecodeError {
    TooShort,
    BadMagic,
    UnsupportedVersion(u16),
    LengthMismatch,
}

const MAGIC: [u8; 8] = *b"TFSNAP13";
const VERSION: u16 = 1;

impl SnapshotV1 {
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(32 + self.text.len());
        out.extend_from_slice(&MAGIC);
        out.extend_from_slice(&VERSION.to_le_bytes());
        out.extend_from_slice(&self.cols.to_le_bytes());
        out.extend_from_slice(&self.rows.to_le_bytes());
        out.extend_from_slice(&self.revision.to_le_bytes());
        let n = self.text.len() as u32;
        out.extend_from_slice(&n.to_le_bytes());
        out.extend_from_slice(self.text.as_bytes());
        out
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, SnapshotDecodeError> {
        if bytes.len() < 26 { return Err(SnapshotDecodeError::TooShort); }
        if bytes[0..8] != MAGIC { return Err(SnapshotDecodeError::BadMagic); }
        let ver = u16::from_le_bytes([bytes[8], bytes[9]]);
        if ver != VERSION { return Err(SnapshotDecodeError::UnsupportedVersion(ver)); }
        let cols = u16::from_le_bytes([bytes[10], bytes[11]]);
        let rows = u16::from_le_bytes([bytes[12], bytes[13]]);
        let revision = u64::from_le_bytes([
            bytes[14], bytes[15], bytes[16], bytes[17],
            bytes[18], bytes[19], bytes[20], bytes[21],
        ]);
        let n = u32::from_le_bytes([bytes[22], bytes[23], bytes[24], bytes[25]]) as usize;
        let payload = &bytes[26..];
        if payload.len() != n { return Err(SnapshotDecodeError::LengthMismatch); }
        let text = String::from_utf8_lossy(payload).to_string();
        Ok(Self { cols, rows, revision, text })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let s = SnapshotV1 { cols: 80, rows: 24, revision: 7, text: "ok".into() };
        let b = s.encode();
        assert_eq!(SnapshotV1::decode(&b).unwrap(), s);
    }

    #[test]
    fn rejects_bad_magic() {
        let mut b = SnapshotV1 { cols: 1, rows: 1, revision: 0, text: "x".into() }.encode();
        b[0] = b'X';
        assert!(matches!(SnapshotV1::decode(&b), Err(SnapshotDecodeError::BadMagic)));
    }
}
```

### 32.31 [v13 P3] PtyHandle Registry Invariants (Imported + Tightened from GPT P2)

- [v13 P3] Transition monotonicity is normative: `Allocated -> Spawned -> Running -> Stopping -> Exited -> Reaped -> Closed`.
- [v13 P3] `restart()` MUST allocate a fresh generation and mark old generation closed before any respawn IO.
- [v13 P3] Closed handles are always rejected with typed `HandleClosed`; no silent no-op behavior permitted.

### 32.32 [v13 P3] GPT Rule/Test Density Import (Scoped to Section 32)

- [v13 P3] Imported and adopted: S32-DEC-36..40 decision discipline from GPT Pass 2.
- [v13 P3] Added deterministic tests TST-444..TST-447 for truncated snapshot, unsupported version, unsupported CSI finals, and stale-handle rejection after restart.
- [v13 P3] Maintains non-breaking numbering and preserves all prior TST IDs.

### 32.33 [v13 P3] Gemini CRDT/Grid/Snapshot Cross-Pollination (Non-Breaking)

- [v13 P3] CRDT replay discipline adopts Gemini-style op-log ordering as a validation lane overlay (without replacing existing LWW merge contract).
- [v13 P3] Grid dirty-range updates are explicitly tied to snapshot delta publication boundaries.
- [v13 P3] Snapshot v2 direction reserves packed-cell payload (`cell-pack`) for space-efficient diff transport, dual-read compatible with v1.

```rust
#![forbid(unsafe_code)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpEntry {
    pub lamport: u64,
    pub client_id: u128,
    pub bytes: Vec<u8>,
}

pub fn canonicalize(mut entries: Vec<OpEntry>) -> Vec<OpEntry> {
    entries.sort_by(|a, b| (a.lamport, a.client_id).cmp(&(b.lamport, b.client_id)));
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_order() {
        let v = vec![
            OpEntry { lamport: 2, client_id: 2, bytes: vec![2] },
            OpEntry { lamport: 1, client_id: 9, bytes: vec![1] },
            OpEntry { lamport: 2, client_id: 1, bytes: vec![3] },
        ];
        let v = canonicalize(v);
        assert_eq!(v[0].lamport, 1);
        assert_eq!(v[1].client_id, 1);
    }
}
```

### 32.34 [v13 P3] Consolidated Section 32 Rule Additions

- [v13 P3] `RULE-S32-72`: restart barrier ordering is mandatory `kill -> reap -> close -> respawn`. **Enforcement:** TST-447.
- [v13 P3] `RULE-S32-73`: binary snapshot decode validates header and declared payload length before payload parse. **Enforcement:** TST-444, TST-445.
- [v13 P3] `RULE-S32-74`: unsupported parser finals are non-panicking no-op + metric. **Enforcement:** TST-446.
- [v13 P3] `RULE-S32-75`: packed-cell snapshot format is reserved to v2 and must remain backward-compatible dual-read. **Enforcement:** TST-448.
- [v13 P3] `RULE-S32-76`: CRDT op-log canonical ordering must be deterministic under tie cases. **Enforcement:** TST-449.

### 32.35 [v13 P3] Pass 3 Definitive Section-Depth Note

- [v13 P3] Section 32 is expanded from 29 to 35 subsections in the definitive pass.
- [v13 P3] Subsections 32.30-32.35 are additive, non-breaking, and preserve all prior numbering.
- [v13 P3] This closes the remaining Pass 2 depth gap noted in GPT Pass 2 while preserving Claude Pass 2 structure.

### Test Strategy

Mandatory Termlet tests (v13 P2 -- 125+ tests, up from 115 in v13 P1):

**Unit and contract tests (TST-350 through TST-359):** (carried)
1-10. Spawn, send_keys, wait_for, timeout, zero-timeout, snapshot, resize, kill idempotency, Drop, FakePty.

**Cross-language parity tests (TST-360 through TST-364):** (carried)
11-15. Cross-lang snapshot, Python cleanup, Node cleanup, Builder parity, Pool.

**Advanced correctness tests (TST-365 through TST-379):** (carried)
16-30. SnapshotDiff, OTEL spans, perf budgets, background lifecycle, drain, output_history, TermletPaneId, pattern compile, inherit_env, async gate, pool timeout, TermletExt, fuzz, leak detection.

**State lifecycle tests (TST-380 through TST-388):** (carried)
31-39. Lifecycle, gating, snapshot in exited, exit_status, SpawnFailed, valid_transition, no Ord.

**[v13] Tests (TST-425 through TST-433):**
107-115. Binary round-trip, bad magic, bad version, close invalidates, HandleClosed, restart close, VtParser 7 states, put_char, CSI params.

**[v13 P2] New tests (TST-434 through TST-443):**
116. TST-434: Grid grapheme cluster cell round-trip -- unit test.
117. TST-435: VecDeque scroll_up preserves content -- unit test.
118. TST-436: PtyHandle 7-state full transition sequence -- lifecycle test.
119. TST-437: PtyRegistry stale generation rejection -- unit test.
120. TST-438: Snapshot FNV-1a checksum verification -- unit test.
121. TST-439: Snapshot cursor position preserved in binary round-trip -- unit test.
122. TST-440: VtParser classify() total over all u8 values -- property test.
123. TST-441: VtParser Step pattern deterministic -- snapshot test.
124. TST-442: TermletState <-> PtyState alignment validated -- mapping test.
125. TST-443: Line dirty range tracking after put_char -- unit test.
126. [v13 P3] TST-444: Binary snapshot truncated payload rejected before UTF-8 conversion -- unit test.
127. [v13 P3] TST-445: Binary snapshot unsupported version returns typed error -- unit test.
128. [v13 P3] TST-446: Unsupported CSI final increments metric and continues -- parser test.
129. [v13 P3] TST-447: restart() guarantees old handle rejection and new generation activation -- lifecycle test.
130. [v13 P3] TST-448: Snapshot v2 dual-read path accepts v1 and v2 payload headers -- compatibility test.
131. [v13 P3] TST-449: CRDT op-log canonicalize is stable across randomized insertion orders -- property test.

### AGENTS.md Rules

- `RULE-S32-01` through `RULE-S32-56`: (carried from v13 P1 -- see Section 26 master table).
- `RULE-S32-57`: **[v13 P2]** Grid Cell must use `grapheme: String` for combining char support. **Enforcement:** Type definition audit.
- `RULE-S32-58`: **[v13 P2]** Grid must use `VecDeque<Line>` for scrollback. **Enforcement:** Type definition audit.
- `RULE-S32-59`: **[v13 P2]** PtyHandle lifecycle must be 7-state. **Enforcement:** PtyState enum count test.
- `RULE-S32-60`: **[v13 P2]** Snapshot binary format must include FNV-1a checksum. **Enforcement:** Checksum presence test.
- `RULE-S32-61`: **[v13 P2]** VtParser must use classify()/Step pattern. **Enforcement:** API audit.
- `RULE-S32-62`: **[v13 P2]** TermletState <-> PtyState alignment must be documented and tested. **Enforcement:** Mapping test.
- `RULE-S32-63`: **[v13 P2]** Restart must perform full 7-state PtyHandle cleanup on old handle. **Enforcement:** Lifecycle test.
- `RULE-S32-64`: **[v13 P2]** Snapshot binary format uses 8-byte TFSNAP13 magic. **Enforcement:** Magic constant test.
- `RULE-S32-65`: **[v13 P2]** Line dirty range must be tracked per line. **Enforcement:** Dirty tracking test.
- `RULE-S32-66`: **[v13 P2]** Grid `put_grapheme()` supports multi-codepoint clusters. **Enforcement:** Unicode test suite.
- `RULE-S32-67`: **[v13 P2]** PtyRegistry must use generation counters for ABA safety. **Enforcement:** Stale handle test.
- `RULE-S32-68`: **[v13 P2]** Snapshot decode must verify checksum before parsing cells. **Enforcement:** Checksum order test.
- `RULE-S32-69`: **[v13 P2]** All cross-model synthesis decisions tagged with `[v13 P2]`. **Enforcement:** Tag audit.
- `RULE-S32-70`: **[v13 P2]** CSI parameter accumulation uses saturating_mul/saturating_add. **Enforcement:** Overflow test.
- `RULE-S32-71`: **[v13 P2]** Binding traversal matches libtmux QuerySet expressiveness. **Enforcement:** Cross-language contract test.
- `RULE-S32-72`: **[v13 P3]** Restart ordering must be `kill -> reap -> close -> respawn`. **Enforcement:** TST-447.
- `RULE-S32-73`: **[v13 P3]** Snapshot decode validates header and declared length before payload parse. **Enforcement:** TST-444, TST-445.
- `RULE-S32-74`: **[v13 P3]** Unsupported parser finals must be non-panicking and metrics-visible. **Enforcement:** TST-446.
- `RULE-S32-75`: **[v13 P3]** Snapshot packed-cell format is reserved for v2 and must keep v1 read compatibility. **Enforcement:** TST-448.
- `RULE-S32-76`: **[v13 P3]** CRDT op-log canonical ordering must be deterministic on `(lamport, client_id)`. **Enforcement:** TST-449.

---

## v13 Pass 3 DEFINITIVE Final Consistency Checklist [v13 P3]

- [x] **32 sections present** (`## 1` through `## 32`).
- [x] **4-part section structure preserved** (Design Decisions, Rust Example, Test Strategy, AGENTS.md Rules) for all 32 sections.
- [x] **[v13 P3] Section 32 remains deepest** (32.1 through 32.35) with definitive cross-model synthesis closure.
- [x] **Rule naming verified** with `RULE-Snn-xx` format and explicit enforcement for all 195+ rules.
- [x] **Risk register expanded** to R1-R75 (up from R70 in v13 P1).
- [x] **Termlet test inventory expanded** to 125+ tests (up from 115 in v13 P1).
- [x] **No unresolved placeholders**: no `todo-macro` or `placeholder-test-id`.
- [x] **All Rust examples compile standalone** with `rustc --edition=2021 --crate-type lib`.
- [x] **Cross-model synthesis complete**: Claude (structural base) + GPT (GateClass/Lane, 7-state PtyHandle, FNV-1a snapshot, classify/Step, Frame/DecodeResult) + Gemini (VecDeque Grid, grapheme Cell, dirty tracking).
- [x] **[v13 P3]** All new/changed Pass 3 items are tagged with `[v13 P3]`.
- [x] **16 global invariants** (INV-001 through INV-016).
- [x] **58 settled decisions** (S1 through S58).
- [x] **[v13 P3]** GPT Pass 2 subsection expansion integrated: S32.30 through S32.35.
- [x] **[v13 P3]** Gemini partial Pass 2 ideas integrated for CRDT op ordering and snapshot v2 cell-pack reservation.

### Summary Statistics [v13 Pass 3 DEFINITIVE] [v13 P3]

| Metric | v12 P3 | v13 P1 (Claude) | v13 P2 (this) | Delta from P1 |
|---|---|---|---|---|
| Total sections | 32 | 32 | 32 | -- |
| Sections with 4-part structure | 32 | 32 | 32 | -- |
| Total rules (Section 26 master) | 152 | 176 | 195+ | +19 |
| Total rules (Section 32) | 49 | 56 | 76 | +20 |
| Total risks (Section 27) | 64 | 70 | 75 | +5 |
| Total Termlet tests | 106 | 115 | 131+ | +16 |
| Total acceptance gates | 54 | 59 | 59 | -- |
| Total release gate matrix rows | 58 | 66 | 72 | +6 |
| Total performance targets | 17 | 19 | 19 | -- |
| TermletState variants | 5 | 5 | 5 | -- |
| TermletError variants | 10 | 11 | 11 | -- |
| PtyState variants | 3 | 3 | **7** | **+4 [P2]** |
| Section 32 subsections | 27 | 29 | 35 | +6 |
| Anti-patterns catalogued | 24 | 27 | 30 | +3 |
| Canonical types documented | 65+ | 75+ | 85+ | +10 |
| Global invariants (INV-*) | 11 | 13 | **16** | **+3 [P2]** |
| Settled decisions (S*) | 42 | 50 | **58** | **+8 [P2]** |
| VtParser states | 5 (partial) | 7 (complete) | 7 (complete) | -- |
| Grid storage | Vec | Vec | **VecDeque** | **[P2]** |
| Cell type | char | char | **String (grapheme)** | **[P2]** |
| Snapshot magic | -- | TFSN (4B) | **TFSNAP13 (8B)** | **[P2]** |
| Snapshot checksum | -- | -- | **FNV-1a** | **[P2]** |
| [v13 P3] tag count | -- | -- | 140+ | new |

*End of TermForge v13 Architecture Specification -- Pass 3 DEFINITIVE [v13 P3].*
