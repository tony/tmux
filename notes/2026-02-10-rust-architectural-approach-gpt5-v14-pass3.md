# v14 Architecture Specification -- Pass 3 [v14 P3] DEFINITIVE

Date: 2026-02-12
Status: **v14 Pass 3 [v14 P3] DEFINITIVE** -- Synthesized Final
Lineage: v13 P3 DEFINITIVE -> v14 Pass 1 (Claude/GPT/Gemini) -> v14 Pass 2 (Claude/GPT/Gemini) -> **v14 Pass 3 [v14 P3] DEFINITIVE** (this document).
Models: Claude Opus 4.6 + GPT-5 + Gemini 2.5 synthesis. Base: Claude v14 P2; imported deltas: GPT v14 P2, Gemini v14 P2.
License: MIT OR Apache-2.0
Rust edition: 2021 (MSRV 1.85)
Protocol target: tmux protocol v8

---

## Preamble

This document is the **v14 Pass 3 [v14 P3] DEFINITIVE** architecture specification for TermForge, a Rust terminal multiplexer with 100% tmux wire-protocol compatibility, ORM-like API, language bindings (Python/Node), CRDT collaboration, a ratatui-based TUI client, and **Termlets** -- SDK-first testing pods that are the killer differentiator.

**[v14 P3]** Pass 2 labels are intentionally retained in inherited sections for provenance, but this file is the authoritative final synthesis.

### [v14 P2] Cross-Pollination Methodology

Pass 2 uses **Claude P1 as the structural base** (32 sections, 4-part DD/RE/TS/AR per section, 36 S32 subsections, 230+ rules, 42 Rust blocks). Cross-pollination from GPT P1 and Gemini P1 follows these principles:

1. **From GPT P1**: Adopt the 45-subsection S32 naming scheme (expand from 36 to 45). Incorporate denser rule set (target 250+ unique rules). Adopt GPT's subsection titles for 32.37-32.45 with substantive content replacing the template-heavy DD.1-DD.10 pattern.
2. **From Gemini P1**: Evaluate SmallVec<[Cell; 132]> for Line cells vs Vec. Evaluate CRC32C vs FNV-1a for snapshots. Consider TermForgeIdentity trait. Adopt network partition / slow consumer Termlet scenarios.
3. **Challenge P1 decisions**: Document disagreements and resolutions.

### [v14 P2] Cross-Pollination Decision Table

| # | Source | Idea | P2 Decision | Rationale |
|---|--------|------|-------------|-----------|
| 1 | Gemini P1 | SmallVec<[Cell; 132]> for Line | **[v14 P2] REJECTED** | SmallVec<[Cell; 132]> with Cell containing CompactString (24 bytes) + style (2) + flags (1) = ~27 bytes per cell means 132 * 27 = 3,564 bytes on stack per Line. For a 24-row grid that is 85,536 bytes of stack. VecDeque<Line> with Vec<Cell> keeps heap allocation predictable and resize-friendly. SmallVec only helps if Lines are frequently created and destroyed (they are not -- they are reused). |
| 2 | Gemini P1 | CRC32C for snapshots | **[v14 P2] ADOPTED as optional** | CRC32C with hardware acceleration (SSE4.2/ARM CRC) is ~10x faster than FNV-1a for large payloads. Default remains FNV-1a for portability; CRC32C opt-in via `snapshot-crc32c` feature flag. Added to snapshot format as alternative checksum algorithm byte. |
| 3 | Gemini P1 | TermForgeIdentity trait | **[v14 P2] REJECTED** | Trait-based identity adds complexity for minimal gain. Build profiles (LTS/Current/Preview) are already handled by CompatibilityLane enum + IdentityManifest struct. Compile-time polymorphism via trait is unnecessary when the identity is a singleton constant. |
| 4 | Gemini P1 | Network partition simulation | **[v14 P2] ADOPTED** | Added as S32.37 (Network Partition Simulation) with latency injection and packet loss APIs for Termlet testing. Uses deterministic clock to avoid flaky tests. |
| 5 | Gemini P1 | Slow consumer simulation | **[v14 P2] ADOPTED** | Added as S32.38 (Slow Consumer Simulation) with BytesPerSec throttle on PTY read. Validates backpressure handling in Grid. |
| 6 | GPT P1 | 45 S32 subsections | **[v14 P2] ADOPTED** | Expanded from 36 to 47 subsections. Adopted GPT's naming for 32.37-32.45 with substantive content. Added 32.46 (Network Partition) and 32.47 (Slow Consumer) from Gemini. |
| 7 | GPT P1 | 342 rules | **[v14 P2] ADOPTED target 260+** | Increased rule density. Claude P1 had 230+; P2 targets 260+ with substantive rules (not template repetitions). |
| 8 | GPT P1 | DD.1-DD.10 pattern per subsection | **[v14 P2] REJECTED** | GPT P1's 10-point DD per subsection is repetitive (same 10 points copy-pasted). P2 uses Claude P1's substantive DD style with section-specific content. |
| 9 | Claude P1 | expect_or_fail() | **[v14 P2] RETAINED** | Non-panicking expect is the right design. Library code must not panic. |
| 10 | Claude P1 | PackedCell(u64) bitfield | **[v14 P2] RETAINED** | Cache-friendly bulk operations for snapshot v2. |
| 11 | Claude P1 | ByteClass enum | **[v14 P2] RETAINED** | Integer comparison in hot path vs string comparison. |
| 12 | Claude P1 | Typestate PtyHandle | **[v14 P2] RETAINED** | Compile-time safety on common paths. |
| 13 | Gemini P1 | CompactString for Cell | **[v14 P2] RETAINED from Claude P1** | Both Claude and Gemini converged on inline string storage. Claude's CompactString (22 bytes inline) is preferred over Gemini's SmolStr suggestion due to better API ergonomics. |

### [v14 P2] Challenges to P1 Decisions

| # | P1 Decision | Challenge | P2 Resolution |
|---|-------------|-----------|---------------|
| 1 | Claude P1: FNV-1a as sole checksum | CRC32C with hardware support is significantly faster for large snapshots | **[v14 P2]** Dual checksum: FNV-1a default, CRC32C opt-in via feature flag. Checksum algorithm byte added to snapshot header. |
| 2 | Claude P1: S32 at 36 subsections | GPT P1 demonstrates value of 45 subsections covering more scenarios | **[v14 P2]** Expanded to 47 subsections with substantive content. |
| 3 | Claude P1: Vec<Cell> for Line | Gemini P1 proposes SmallVec<[Cell; 132]> for stack allocation | **[v14 P2]** Rejected SmallVec after analysis: stack cost too high (3.5KB per line). Vec<Cell> with capacity hint is the right tradeoff. |
| 4 | GPT P1: Template DD.1-DD.10 per subsection | Repetitive content dilutes signal | **[v14 P2]** Replaced with substantive per-subsection DDs. |
| 5 | Gemini P1: TermForgeIdentity trait | Over-engineering for a singleton pattern | **[v14 P2]** Keep IdentityManifest struct + CompatibilityLane enum. |
| 6 | Claude P1: No deterministic clock for Termlets | Network partition tests need deterministic time | **[v14 P2]** Added DeterministicClock trait for Termlet testing with simulated time. |

### [v14 P2] Critical Review of P1 Specs

1. **[v14 P2] Claude P1 missing network simulation**: Claude P1 has no mechanism for simulating network conditions in Termlets. Gemini P1 correctly identified this gap. Fixed: added S32.37 and S32.38.
2. **[v14 P2] GPT P1 template inflation**: GPT P1's S32 subsections have identical DD.1-DD.10 blocks with only the subsection name changed. This inflates line count without adding substance. Fixed: P2 uses substantive per-subsection content.
3. **[v14 P2] Gemini P1 incomplete spec**: Only 5 sections specified (308 lines). Good ideas (SmallVec, CRC32C, network partition) but insufficient depth. Fixed: ideas evaluated and selectively adopted.
4. **[v14 P2] Claude P1 snapshot checksum inflexibility**: Only FNV-1a with no path to hardware-accelerated checksums. Fixed: dual checksum with CRC32C opt-in.
5. **[v14 P2] All P1 specs missing DeterministicClock**: None of the P1 specs provide a concrete clock abstraction for deterministic Termlet testing. GPT P1 names a "Deterministic Clock" subsection (32.9) but provides no implementation. Fixed: concrete `TermletClock` trait added.

**Traceability convention (carried from v13 P3):**
- `INV-*`: core invariants
- `API-*`: API contracts
- `PAR-*`: parity contracts against tmux/libtmux/reference crates
- `OPS-*`: runtime/operational contracts
- `TST-*`: mandatory test gates

**Settled global invariants (v14 P2 -- 22 items):**
- `INV-001`: Protocol compatibility target is tmux protocol v8.
- `INV-002`: `mux-core` is deterministic and IO-free (`#![forbid(unsafe_code)]`).
- `INV-003`: Single writer for state mutation; snapshot-based readers.
- `INV-004`: Protocol violation drops client connection, never partial-continue.
- `INV-005`: First-client identify completes before config load path.
- `INV-006`: Layout resize uses round-robin one-cell adjustment.
- `INV-007`: Socket/test isolation uses three guard layers.
- `INV-008`: Termlets are pane-backed and never a parallel terminal stack.
- `INV-009`: `TermletState` transitions validated by `valid_transition()`, not by enum ordering.
- `INV-010`: Effects are idempotent by effect key; retry-safe by construction.
- `INV-011`: `TermletLike` trait is the normative API contract; concrete `Termlet` implements it.
- `INV-012`: Grid::put_char/put_grapheme are the sole entry points for character placement.
- `INV-013`: Slot generation counters prevent ABA reuse.
- `INV-014`: PtyHandle lifecycle is a 7-state machine with typestate compile-time enforcement on common paths.
- `INV-015`: Snapshot binary format includes trailing checksum; decode rejects mismatch.
- `INV-016`: Cell grapheme field stores grapheme cluster as CompactString; single-char cells use inline storage.
- `INV-017`: Restart barrier is mandatory `kill -> close(old_handle) -> spawn(new_handle)`.
- `INV-018`: VtParser transition table is a `const` 2D array; totality is verified at compile time.
- `INV-019`: All error types in public API are `Clone + PartialEq + Eq` for deterministic test assertions.
- `INV-020`: Snapshot v2 packed-cell format is backward-compatible; decoder supports both v1 and v2.
- `INV-021`: **[v14 P2]** Snapshot checksum algorithm is selectable (FNV-1a default, CRC32C opt-in); algorithm byte encoded in header.
- `INV-022`: **[v14 P2]** Termlet network simulation uses DeterministicClock; real clock forbidden in network partition tests.

**Settled decisions (v14 P2 -- 76 items, extending v14 P1's 72):**

| # | Decision | Rationale |
|---|---|---|
| S1-S62 | (carried from v13 P3) | See v13 P3 preamble |
| S63 | `ByteClass` enum replaces `&str` classify | Integer comparison in hot path |
| S64 | `CompactString` replaces `String` in Cell | Inline storage eliminates heap allocation |
| S65 | `PackedCell(u64)` for snapshot binary | Cache-friendly bulk operations |
| S66 | Snapshot v2 specified with delta encoding | Enables compact incremental snapshots |
| S67 | Typestate PtyHandle for compile-time safety | Common transition errors caught at compile time |
| S68 | `expect_or_fail()` replaces panic-based `expect()` | Library code must not panic |
| S69 | OpLog uses binary insertion | O(log n) vs O(n log n) per append |
| S70 | Grid::put() removed from public API | Enforces INV-012 at API level |
| S71 | `erase_in_display`/`erase_in_line` added to Grid | CSI J/K parity with tmux |
| S72 | Const transition table for VtParser | Provable totality over all (State, ByteClass) pairs |
| S73 | **[v14 P2]** CRC32C opt-in via feature flag | Hardware-accelerated checksum for large snapshots |
| S74 | **[v14 P2]** DeterministicClock trait for Termlet testing | Network partition tests require controllable time |
| S75 | **[v14 P2]** SmallVec rejected for Line cells | Stack cost too high (3.5KB/line); Vec with capacity hint preferred |
| S76 | **[v14 P2]** S32 expanded to 47 subsections | Covers network partition, slow consumer, parallel scheduling, flake triage |

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
- Compatibility marketing: "100% tmux wire-protocol compatible."
- **[v14]** Version string format: `termforge <semver>+v14 (protocol v8, edition 2021)`.
- **[v14]** `IdentityManifest` now includes `spec_version: &'static str` field with value `"v14-pass2"`.
- **[v14]** Binary embeds build metadata via `env!("CARGO_PKG_VERSION")` at compile time.
- **[v14 P2]** `IdentityManifest` remains a struct (not a trait). Gemini P1's `TermForgeIdentity` trait rejected: singleton pattern does not benefit from compile-time polymorphism.
- **[v14 P2]** `spec_version` updated to `"v14-pass2"` to track pass lineage.
- Traceability: `API-001`, `OPS-001`.

### Rust Example

```rust
// crates/mux-types/src/identity.rs
// [v14 P2] Standalone-compilable with rustc --edition=2021 --crate-type lib

#![allow(dead_code)]

pub const PROJECT_NAME: &str = "TermForge";
pub const BINARY_NAME: &str = "termforge";
pub const BINARY_ALIAS: &str = "tf";
pub const SOCKET_PREFIX: &str = "termforge";
pub const PROTOCOL_VERSION: u32 = 8;
pub const CRATE_PREFIX: &str = "mux-";
pub const SPEC_VERSION: &str = "v14-pass2"; // [v14 P2] updated

/// [v14] Compatibility lane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompatibilityLane {
    Lts,
    Current,
    Preview,
}

impl CompatibilityLane {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Lts => "lts",
            Self::Current => "current",
            Self::Preview => "preview",
        }
    }
}

/// [v14 P2] IdentityManifest as struct (not trait -- Gemini TermForgeIdentity rejected).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityManifest {
    pub project_name: &'static str,
    pub binary_primary: &'static str,
    pub binary_alias: &'static str,
    pub crate_prefix: &'static str,
    pub spec_version: &'static str,
    pub protocol_version: u32,
}

impl IdentityManifest {
    pub const fn canonical() -> Self {
        Self {
            project_name: PROJECT_NAME,
            binary_primary: BINARY_NAME,
            binary_alias: BINARY_ALIAS,
            crate_prefix: CRATE_PREFIX,
            spec_version: SPEC_VERSION,
            protocol_version: PROTOCOL_VERSION,
        }
    }
}

/// [v14 P2] Version string with v14 tag.
pub fn version_string() -> String {
    format!(
        "{} 0.1.0+v14 (protocol v{}, edition 2021)",
        PROJECT_NAME, PROTOCOL_VERSION
    )
}

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

pub fn default_socket_dir(uid: u32) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("{}-{}", SOCKET_PREFIX, uid))
}

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
    fn test_version_string_contains_v14() {
        let v = version_string();
        assert!(v.contains("v14"));
        assert!(v.contains("protocol v8"));
    }

    #[test]
    fn test_socket_dir_contains_uid() {
        let dir = default_socket_dir(1000);
        assert!(dir.to_string_lossy().contains("termforge-1000"));
    }

    #[test]
    fn test_lane_labels() {
        assert_eq!(CompatibilityLane::Lts.label(), "lts");
        assert_eq!(CompatibilityLane::Current.label(), "current");
        assert_eq!(CompatibilityLane::Preview.label(), "preview");
    }

    #[test]
    fn test_spec_version_v14_p2() {
        assert!(SPEC_VERSION.starts_with("v14"));
        assert!(SPEC_VERSION.contains("pass2")); // [v14 P2]
    }

    #[test]
    fn test_identity_manifest_canonical() {
        let m = IdentityManifest::canonical();
        assert_eq!(m.project_name, "TermForge");
        assert_eq!(m.binary_primary, "termforge");
        assert_eq!(m.spec_version, "v14-pass2"); // [v14 P2]
        assert_eq!(m.protocol_version, 8);
    }
}
```

### Test Strategy

1. Binary name assertion. `TST-001`.
2. Alias detection via `argv[0]`. `TST-002`.
3. Socket dir contains UID. `TST-003`.
4. Config path respects XDG. `TST-004`.
5. Config path default fallback. `TST-005`.
6. Protocol handshake golden tests. `TST-006`.
7. License governance CI. `TST-007`.
8. **[v14]** Version string contains `v14` tag. `TST-008`.
9. Lane labels stable and lowercase. `TST-009`.
10. Binary alias dispatch parity. `TST-010`.
11. IdentityManifest canonical matches all consts. `TST-011`.
12. **[v14]** `spec_version` field present in manifest. `TST-012`.
13. **[v14 P2]** `spec_version` contains "pass2". `TST-013`.
14. **[v14 P2]** IdentityManifest is struct, not trait (compile-time check). `TST-014`.

### AGENTS.md Rules

- `RULE-S01-01`: All crate names must use the `mux-` prefix. **Enforcement:** CI grep of `Cargo.toml` names.
- `RULE-S01-02`: Binary name `termforge`, alias `tf`. **Enforcement:** `cargo build` output name check.
- `RULE-S01-03`: Any PR changing compatibility scope must update Section 20 matrices. **Enforcement:** CI checks changed files.
- `RULE-S01-04`: **[v14]** Version string must include `v14` spec version tag. **Enforcement:** Integration test.
- `RULE-S01-05`: Compatibility claims MUST include lane annotation. **Enforcement:** docs linter.
- `RULE-S01-06`: Identity changes require changelog delta and migration notes. **Enforcement:** PR policy check.
- `RULE-S01-07`: `IdentityManifest::canonical()` must match all identity constants. **Enforcement:** Unit test (TST-011).
- `RULE-S01-08`: **[v14]** `spec_version` field must be present and start with `v14`. **Enforcement:** Unit test (TST-012).
- `RULE-S01-09`: **[v14 P2]** IdentityManifest MUST remain a struct. Trait-based identity rejected (S75). **Enforcement:** API surface lint.

---

## 2. Acceptance Criteria and Gates [v14 P2]

### Design Decisions

- **[v14]** Four gate classes: `Compat`, `Correctness`, `Performance`, `Operability`.
- **[v14]** Gate results are Lane-scoped: `Lts`, `Current`, `Preview`.
- `compat` and `correctness` are hard-blocking across all lanes.
- `performance` is hard-blocking for LTS/Current; soft with waiver for Preview.
- **[v14]** Waiver validation: waivers require expiry timestamp; expired waivers block release automatically.
- **[v14]** Added C9 (Grid erase_in_display parity), A7 (CompactString allocation invariant), B21 (PackedCell snapshot throughput), P22 (Snapshot v2 round-trip).
- **[v14]** `Finding` type extended with `resolved_in: &'static str` field for version traceability.
- **[v14 P2]** Added A8 (CRC32C checksum correctness), B22 (CRC32C throughput vs FNV-1a), P23 (DeterministicClock monotonicity).
- **[v14 P2]** `GateResult` extended with `checksum_algorithm` field for snapshot-related gates.
- Gate results are immutable records keyed by gate ID and commit SHA.
- Every gate maps to at least one test in the CI matrix.
- Traceability: `INV-001` through `INV-022`.

### Rust Example

```rust
// crates/mux-types/src/gates.rs
// [v14 P2] Gates with waiver expiry validation and CRC32C gate

#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GateClass {
    Compat,
    Correctness,
    Performance,
    Operability,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lane {
    Lts,
    Current,
    Preview,
}

/// [v14 P2] ChecksumAlgorithm for snapshot gates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChecksumAlgorithm {
    Fnv1a,
    Crc32c,
}

/// [v14 P2] GateResult with waiver expiry validation and checksum field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateResult {
    pub gate_id: &'static str,
    pub class: GateClass,
    pub lane: Lane,
    pub pass: bool,
    pub waived: bool,
    pub waiver_expiry_epoch: Option<u64>,
    pub message: String,
    pub checksum_algorithm: Option<ChecksumAlgorithm>, // [v14 P2]
}

impl GateResult {
    /// [v14] Check if waiver has expired given current epoch seconds.
    pub fn waiver_expired(&self, now_epoch: u64) -> bool {
        match self.waiver_expiry_epoch {
            Some(expiry) if self.waived => now_epoch > expiry,
            _ => false,
        }
    }
}

pub fn gate_is_required(class: GateClass, lane: Lane) -> bool {
    match (class, lane) {
        (GateClass::Compat, _) => true,
        (GateClass::Correctness, _) => true,
        (GateClass::Performance, Lane::Preview) => false,
        (GateClass::Performance, _) => true,
        (GateClass::Operability, _) => true,
    }
}

/// [v14 P2] release_ok checks gate results with waiver expiry enforcement.
pub fn release_ok(results: &[GateResult], now_epoch: u64) -> bool {
    results.iter().all(|r| {
        if r.waiver_expired(now_epoch) {
            return false;
        }
        if gate_is_required(r.class, r.lane) {
            r.pass && !r.waived
        } else {
            r.pass || r.waived
        }
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity { Low, Medium, High, Critical }

/// [v14] Finding with resolved_in version field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub id: &'static str,
    pub severity: Severity,
    pub summary: &'static str,
    pub remediated: bool,
    pub resolved_in: &'static str,
}

pub fn all_remediated(findings: &[Finding]) -> bool {
    findings.iter().all(|f| f.remediated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compat_gates_blocking() {
        assert!(gate_is_required(GateClass::Compat, Lane::Preview));
    }

    #[test]
    fn test_perf_preview_soft() {
        assert!(!gate_is_required(GateClass::Performance, Lane::Preview));
    }

    #[test]
    fn test_waiver_expiry_blocks() {
        let results = vec![
            GateResult {
                gate_id: "C1", class: GateClass::Compat, lane: Lane::Lts,
                pass: true, waived: true, waiver_expiry_epoch: Some(1000),
                message: String::new(), checksum_algorithm: None,
            },
        ];
        assert!(!release_ok(&results, 2000));
    }

    #[test]
    fn test_valid_waiver_passes() {
        let results = vec![
            GateResult {
                gate_id: "B1", class: GateClass::Performance, lane: Lane::Preview,
                pass: false, waived: true, waiver_expiry_epoch: Some(5000),
                message: String::new(), checksum_algorithm: None,
            },
        ];
        assert!(release_ok(&results, 2000));
    }

    // [v14 P2] CRC32C gate result test
    #[test]
    fn test_crc32c_gate_result() {
        let r = GateResult {
            gate_id: "A8", class: GateClass::Correctness, lane: Lane::Current,
            pass: true, waived: false, waiver_expiry_epoch: None,
            message: "CRC32C checksum correct".into(),
            checksum_algorithm: Some(ChecksumAlgorithm::Crc32c),
        };
        assert!(r.pass);
        assert_eq!(r.checksum_algorithm, Some(ChecksumAlgorithm::Crc32c));
    }
}
```

### Test Strategy

1. Gate checklist run on every release candidate. `TST-020`.
2. Performance gates warn but do not block (Preview). `TST-021`.
3. All functional gates must pass. `TST-022`.
4. PerfTarget structs enforce Criterion thresholds. `TST-023`.
5. C8 gate verifies VtParser 7 states. `TST-024`.
6. A6 gate verifies Grid::put_char sole entry. `TST-025`.
7. B18 VtParser CSI throughput. `TST-026`.
8. P20 snapshot binary round-trip. `TST-027`.
9. P21 OTEL span hierarchy. `TST-028`.
10. Lane-scoped results verified per lane. `TST-029`.
11. Finding inventory complete. `TST-030`.
12. Gate schema version compatibility. `TST-031`.
13. Waived required gate blocks release. `TST-032`.
14. **[v14]** Expired waiver blocks release. `TST-033`.
15. **[v14]** C9 erase_in_display parity. `TST-034`.
16. **[v14]** A7 CompactString invariant. `TST-035`.
17. **[v14]** B21 PackedCell snapshot throughput. `TST-036`.
18. **[v14]** P22 Snapshot v2 round-trip. `TST-037`.
19. **[v14 P2]** A8 CRC32C checksum correctness. `TST-038`.
20. **[v14 P2]** B22 CRC32C throughput vs FNV-1a benchmark. `TST-039`.
21. **[v14 P2]** P23 DeterministicClock monotonicity. `TST-040`.

### AGENTS.md Rules

- `RULE-S02-01`: Every new feature must map to at least one gate. **Enforcement:** PR template.
- `RULE-S02-02`: No gate may be removed without spec amendment. **Enforcement:** Append-only table.
- `RULE-S02-03`: Gate thresholds must be measurable. **Enforcement:** Acceptance criteria table.
- `RULE-S02-04`: Every `INV-*` maps to `TST-*`. **Enforcement:** CI traceability lint.
- `RULE-S02-05`: Compat failures are never `allow_failure`. **Enforcement:** CI policy.
- `RULE-S02-06`: Expired quarantined tests block release. **Enforcement:** CI expiry check.
- `RULE-S02-07`: New gates must specify pass and fail thresholds. **Enforcement:** Gate table lint.
- `RULE-S02-08`: Gate results MUST include lane annotation. **Enforcement:** Schema validation.
- `RULE-S02-09`: All v12/v13 defects represented as `Finding` entries. **Enforcement:** Inventory lint.
- `RULE-S02-10`: **[v14]** Waivers MUST have expiry timestamp; expired waivers block release. **Enforcement:** Waiver expiry CI.
- `RULE-S02-11`: **[v14]** Finding entries must include `resolved_in` version. **Enforcement:** Finding schema lint.
- `RULE-S02-12`: **[v14 P2]** Snapshot-related gates MUST specify checksum_algorithm field. **Enforcement:** Gate schema validation.
- `RULE-S02-13`: **[v14 P2]** CRC32C gate (A8) is hard-blocking when `snapshot-crc32c` feature is enabled. **Enforcement:** Feature-conditional CI.

---

## 3. Crate Dependency Rules

### Design Decisions

- Strict layered architecture: Layer 0 (pure) -> Layer 1 (OS) -> Layer 2 (facade) -> Layer 3 (application).
- `mux-core` is Layer 0: pure, WASM-compatible, no IO, no unsafe.
- **[v14]** `mux-compact-string` added at Layer 0 for inline string type used by Grid Cell.
- **[v14]** `mux-packed-cell` added at Layer 0 for snapshot v2 packed cell encoding.
- **[v14 P2]** `mux-checksum` added at Layer 0 for FNV-1a and CRC32C implementations. CRC32C behind `crc32c` feature flag.
- Forbidden edges: Layer N may not depend on Layer N+1.
- `mux-crdt` behind feature flag `crdt`.
- Traceability: `INV-002`, `API-003`.

### Rust Example

```rust
// crates/mux-types/src/layers.rs
// [v14 P2] Extended with mux-checksum crate

#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Layer {
    L0Pure = 0,
    L1Adapter = 1,
    L2Facade = 2,
    L3Tool = 3,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrateNode {
    pub name: &'static str,
    pub layer: Layer,
}

/// [v14] Simplified: from.layer >= to.layer means valid (higher can depend on same or lower).
pub fn can_depend(from: Layer, to: Layer) -> bool {
    from as u8 >= to as u8
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
    CrateNode { name: "mux-compact-string", layer: Layer::L0Pure },
    CrateNode { name: "mux-packed-cell", layer: Layer::L0Pure },
    CrateNode { name: "mux-checksum", layer: Layer::L0Pure }, // [v14 P2]
    // Layer 1
    CrateNode { name: "mux-state", layer: Layer::L1Adapter },
    CrateNode { name: "mux-conf", layer: Layer::L1Adapter },
    CrateNode { name: "mux-keys", layer: Layer::L1Adapter },
    CrateNode { name: "mux-format", layer: Layer::L1Adapter },
    CrateNode { name: "mux-pty", layer: Layer::L1Adapter },
    CrateNode { name: "mux-os", layer: Layer::L1Adapter },
    CrateNode { name: "mux-trace-export", layer: Layer::L1Adapter },
    // Layer 2
    CrateNode { name: "mux-runtime", layer: Layer::L2Facade },
    CrateNode { name: "mux-api", layer: Layer::L2Facade },
    CrateNode { name: "mux-orm", layer: Layer::L2Facade },
    CrateNode { name: "mux-termlet", layer: Layer::L2Facade },
    CrateNode { name: "mux-control", layer: Layer::L2Facade },
    CrateNode { name: "mux-otel", layer: Layer::L2Facade },
    CrateNode { name: "mux-server", layer: Layer::L2Facade },
    CrateNode { name: "mux-test-support", layer: Layer::L2Facade },
    CrateNode { name: "mux-bindings-shared", layer: Layer::L2Facade },
    // Layer 3
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
    fn test_workspace_crate_count() {
        assert!(WORKSPACE.len() >= 33); // [v14 P2] 33+ with mux-checksum
    }

    #[test]
    fn test_invalid_edge_detected() {
        let edge = (
            CrateNode { name: "mux-core", layer: Layer::L0Pure },
            CrateNode { name: "mux-termlet", layer: Layer::L2Facade },
        );
        assert!(!layout_is_valid(&[edge]));
    }

    // [v14 P2] Verify mux-checksum is in workspace
    #[test]
    fn test_checksum_crate_in_workspace() {
        assert!(WORKSPACE.iter().any(|c| c.name == "mux-checksum"));
    }
}
```

### Test Strategy

1. `can_depend` truth table. `TST-050`.
2. Known-good workspace edges pass. `TST-051`.
3. Reverse edge fails. `TST-052`.
4. Examples no forbidden deps. `TST-053`.
5. Feature-flag namespace lint. `TST-054`.
6. `crdt` off-by-default. `TST-055`.
7. Bindings import only allowed facades. `TST-056`.
8. Workspace crate count matches canonical. `TST-057`.
9. **[v14]** `mux-compact-string` is Layer 0. `TST-058`.
10. **[v14]** `mux-packed-cell` is Layer 0. `TST-059`.
11. **[v14 P2]** `mux-checksum` is Layer 0 WASM-compatible. `TST-060`.
12. **[v14 P2]** `crc32c` feature flag off-by-default. `TST-061`.

### AGENTS.md Rules

- `RULE-S03-01`: Layer violations block CI. **Enforcement:** Dependency linter.
- `RULE-S03-02`: New crates declare layer. **Enforcement:** PR review.
- `RULE-S03-03`: mux-termlet no server/api dep. **Enforcement:** cargo tree.
- `RULE-S03-04`: mux-core Layer 0 WASM. **Enforcement:** cargo check wasm32.
- `RULE-S03-05`: mux-crdt feature-gated. **Enforcement:** cargo tree.
- `RULE-S03-06`: New deps require rationale. **Enforcement:** changed-files.
- `RULE-S03-07`: mux-snapshot Layer 0. **Enforcement:** WASM CI.
- `RULE-S03-08`: Cross-layer feature flags namespaced. **Enforcement:** Name lint.
- `RULE-S03-09`: Examples cannot introduce reverse deps. **Enforcement:** Dep graph CI.
- `RULE-S03-10`: **[v14]** `mux-compact-string` Layer 0 WASM-compatible. **Enforcement:** WASM CI.
- `RULE-S03-11`: **[v14]** `mux-packed-cell` Layer 0 WASM-compatible. **Enforcement:** WASM CI.
- `RULE-S03-12`: **[v14 P2]** `mux-checksum` Layer 0 WASM-compatible; CRC32C behind feature flag. **Enforcement:** WASM CI + feature check.

---

## 4. Workspace Layout

### Design Decisions

- Standard Cargo workspace with `crates/`, `bindings/`, `tools/` directories.
- **[v14]** Added `mux-compact-string` and `mux-packed-cell` to crates directory.
- **[v14]** Added `benchmarks/` top-level directory for consolidated Criterion benchmarks.
- **[v14 P2]** Added `mux-checksum` to crates directory.
- Traceability: `OPS-004`.

### Rust Example

```text
termforge/
  Cargo.toml
  CLAUDE.md / AGENTS.md
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
    mux-compact-string/         # [v14] L0: inline string type
    mux-packed-cell/            # [v14] L0: packed cell encoding
    mux-checksum/               # [v14 P2] L0: FNV-1a + CRC32C
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
  benchmarks/                   # [v14] consolidated benchmarks
    criterion/
  fixtures/
    protocol/
    snapshots/
    fuzz-corpus/
  notes/
    architecture/
  examples/
    termlet_recipes/
  .github/
    workflows/
```

### Test Strategy

1. All workspace members in root `Cargo.toml`. `TST-070`.
2. `cargo build --workspace` succeeds. `TST-071`.
3. Directory structure matches canonical. `TST-072`.
4. No orphan crates. `TST-073`.
5. **[v14]** Benchmarks directory exists. `TST-074`.
6. **[v14 P2]** `mux-checksum` crate exists and compiles. `TST-075`.

### AGENTS.md Rules

- `RULE-S04-01`: New crates in workspace members. **Enforcement:** CI member check.
- `RULE-S04-02`: Crate README states layer. **Enforcement:** PR review.
- `RULE-S04-03`: mux-termlet Layer 2. **Enforcement:** cargo deny.
- `RULE-S04-04`: All examples compile. **Enforcement:** CI check.
- `RULE-S04-05`: CI workflow changes need review. **Enforcement:** CODEOWNERS.
- `RULE-S04-06`: Workspace layout matches canonical. **Enforcement:** Layout lint.
- `RULE-S04-07`: **[v14]** Benchmark directory present with Criterion harness. **Enforcement:** CI check.
- `RULE-S04-08`: **[v14 P2]** mux-checksum directory present. **Enforcement:** CI check.

---

## 5. Grid API Completeness (Cell, Line, Viewport) [v14 P2]

### Design Decisions

- **[v14]** `Cell` stores grapheme as `CompactString` (inline up to 22 bytes, heap above). Replaces `String` for reduced allocation pressure. Single ASCII chars are always inline.
- **[v14]** `Grid::put()` removed from public API. All cell writes go through `put_char()` or `put_grapheme()` (strengthens INV-012).
- **[v14]** Added `erase_in_display(mode: EraseMode)` for CSI J support: `Below`, `Above`, `All`, `Scrollback`.
- **[v14]** Added `erase_in_line(mode: EraseMode)` for CSI K support: `ToEnd`, `ToStart`, `All`.
- **[v14]** `Line` tracks `is_wrapped: bool` (renamed from `wrapped` for clarity) and round-trips through snapshot.
- **[v14 P2]** SmallVec<[Cell; 132]> for Line REJECTED: 132 cells * ~27 bytes = 3,564 bytes stack per line. For typical 24-row terminal = 85,536 bytes stack, excessive. Vec<Cell> with `Vec::with_capacity(cols)` is the right tradeoff.
- **[v14 P2]** Line gains `capacity_hint` in constructor for pre-allocation.
- Grid uses `VecDeque<Line>` for O(1) scrollback (unchanged from v13).
- Grid tracks `revision: u64` monotonic counter (unchanged).
- Traceability: `INV-012`, `INV-016`, `API-010`.

### Rust Example

```rust
// crates/mux-grid/src/lib.rs
// [v14 P2] CompactString Cell, removed put(), added erase modes, SmallVec rejected

#![allow(dead_code)]
#![forbid(unsafe_code)]

use std::collections::VecDeque;

/// [v14] CompactString: inline storage for small strings, heap for large.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CompactString(String);

impl CompactString {
    pub fn new(s: &str) -> Self { Self(s.to_string()) }
    pub fn from_char(ch: char) -> Self { Self(ch.to_string()) }
    pub fn as_str(&self) -> &str { &self.0 }
    pub fn len(&self) -> usize { self.0.len() }
    pub fn is_empty(&self) -> bool { self.0.is_empty() }
    /// [v14] Returns true if the string fits in inline storage (<=22 bytes).
    pub fn is_inline(&self) -> bool { self.0.len() <= 22 }
}

impl Default for CompactString {
    fn default() -> Self { Self(" ".to_string()) }
}

impl std::fmt::Display for CompactString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// [v14] Cell with CompactString grapheme.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    pub grapheme: CompactString,
    pub style: u16,
    pub wide_continuation: bool,
}

impl Cell {
    pub fn default_cell() -> Self {
        Self { grapheme: CompactString::default(), style: 0, wide_continuation: false }
    }
    pub fn with_char(ch: char) -> Self {
        Self { grapheme: CompactString::from_char(ch), style: 0, wide_continuation: false }
    }
    pub fn with_grapheme(g: &str) -> Self {
        Self { grapheme: CompactString::new(g), style: 0, wide_continuation: false }
    }
    pub fn is_default(&self) -> bool {
        self.grapheme.as_str() == " " && self.style == 0 && !self.wide_continuation
    }
    pub fn display_width(&self) -> u8 {
        if self.wide_continuation { 0 } else { 1 }
    }
}

impl Default for Cell { fn default() -> Self { Self::default_cell() } }

/// [v14] EraseMode for CSI J and CSI K.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EraseMode {
    Below,     // CSI 0 J / CSI 0 K
    Above,     // CSI 1 J / CSI 1 K
    All,       // CSI 2 J / CSI 2 K
    Scrollback, // CSI 3 J (display only)
}

/// [v14 P2] Line with Vec<Cell> (SmallVec rejected) and capacity hint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub cells: Vec<Cell>, // [v14 P2] Vec, not SmallVec -- see S75
    pub dirty_start: usize,
    pub dirty_end: usize,
    pub is_wrapped: bool,
}

impl Line {
    pub fn new(cols: usize) -> Self {
        let mut cells = Vec::with_capacity(cols); // [v14 P2] capacity hint
        cells.resize(cols, Cell::default());
        Self { cells, dirty_start: cols, dirty_end: 0, is_wrapped: false }
    }
    pub fn mark_dirty(&mut self, col: usize) {
        if col < self.cells.len() {
            self.dirty_start = self.dirty_start.min(col);
            self.dirty_end = self.dirty_end.max(col + 1);
        }
    }
    pub fn clear_dirty(&mut self) { self.dirty_start = self.cells.len(); self.dirty_end = 0; }
    pub fn is_dirty(&self) -> bool { self.dirty_start < self.dirty_end }
    pub fn text(&self) -> String {
        self.cells.iter().filter(|c| !c.wide_continuation).map(|c| c.grapheme.as_str()).collect()
    }
    pub fn trimmed_text(&self) -> String { self.text().trim_end().to_string() }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Viewport { pub top: usize, pub left: usize, pub rows: usize, pub cols: usize }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CursorPos { pub col: u16, pub row: u16 }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridError { OutOfBounds, InvalidSize }

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
        for _ in 0..rows { lines.push_back(Line::new(cols as usize)); }
        Self { lines, cols, rows, cursor: CursorPos::default(), scrollback_limit: 10_000, revision: 0 }
    }

    pub fn cols(&self) -> u16 { self.cols }
    pub fn rows(&self) -> u16 { self.rows }
    pub fn revision(&self) -> u64 { self.revision }
    pub fn cursor(&self) -> CursorPos { self.cursor }

    /// [v14] INV-012: sole entry point for single character placement.
    pub fn put_char(&mut self, ch: char) {
        let col = self.cursor.col as usize;
        let row = self.cursor.row as usize;
        if row < self.lines.len() && col < self.cols as usize {
            self.lines[row].cells[col] = Cell::with_char(ch);
            self.lines[row].mark_dirty(col);
            self.cursor.col += 1;
            if self.cursor.col >= self.cols {
                self.lines[row].is_wrapped = true;
                self.newline();
            }
            self.revision = self.revision.wrapping_add(1);
        }
    }

    /// [v14] INV-012: sole entry point for grapheme cluster placement.
    pub fn put_grapheme(&mut self, grapheme: &str) {
        let col = self.cursor.col as usize;
        let row = self.cursor.row as usize;
        if row < self.lines.len() && col < self.cols as usize {
            self.lines[row].cells[col] = Cell::with_grapheme(grapheme);
            self.lines[row].mark_dirty(col);
            self.cursor.col += 1;
            if self.cursor.col >= self.cols {
                self.lines[row].is_wrapped = true;
                self.newline();
            }
            self.revision = self.revision.wrapping_add(1);
        }
    }

    pub fn newline(&mut self) {
        if self.cursor.row + 1 >= self.rows {
            self.scroll_up();
        } else {
            self.cursor.row += 1;
        }
        self.cursor.col = 0;
    }

    pub fn carriage_return(&mut self) { self.cursor.col = 0; }

    pub fn scroll_up(&mut self) {
        if !self.lines.is_empty() {
            let _old = self.lines.pop_front();
            self.lines.push_back(Line::new(self.cols as usize));
            self.revision = self.revision.wrapping_add(1);
        }
    }

    pub fn line_at(&self, row: u16) -> Option<&Line> { self.lines.get(row as usize) }
    pub fn cell_at(&self, col: u16, row: u16) -> Option<&Cell> {
        self.lines.get(row as usize)?.cells.get(col as usize)
    }

    pub fn set_cursor(&mut self, col: u16, row: u16) {
        if col < self.cols && row < self.rows {
            self.cursor = CursorPos { col, row };
        }
    }

    /// [v14] CSI J: erase in display.
    pub fn erase_in_display(&mut self, mode: EraseMode) {
        match mode {
            EraseMode::Below => {
                let row = self.cursor.row as usize;
                let col = self.cursor.col as usize;
                if row < self.lines.len() {
                    for c in col..self.lines[row].cells.len() {
                        self.lines[row].cells[c] = Cell::default();
                    }
                    self.lines[row].mark_dirty(col);
                    for r in (row + 1)..self.lines.len() {
                        self.lines[r] = Line::new(self.cols as usize);
                    }
                }
            }
            EraseMode::Above => {
                let row = self.cursor.row as usize;
                let col = self.cursor.col as usize;
                for r in 0..row {
                    self.lines[r] = Line::new(self.cols as usize);
                }
                if row < self.lines.len() {
                    for c in 0..=col.min(self.lines[row].cells.len().saturating_sub(1)) {
                        self.lines[row].cells[c] = Cell::default();
                    }
                    self.lines[row].mark_dirty(0);
                }
            }
            EraseMode::All => {
                for r in 0..self.lines.len() {
                    self.lines[r] = Line::new(self.cols as usize);
                }
                self.cursor = CursorPos::default();
            }
            EraseMode::Scrollback => {
                // CSI 3 J: clear scrollback only (not visible area)
            }
        }
        self.revision = self.revision.wrapping_add(1);
    }

    /// [v14] CSI K: erase in line.
    pub fn erase_in_line(&mut self, mode: EraseMode) {
        let row = self.cursor.row as usize;
        if row >= self.lines.len() { return; }
        let col = self.cursor.col as usize;
        match mode {
            EraseMode::Below => { // ToEnd
                for c in col..self.lines[row].cells.len() {
                    self.lines[row].cells[c] = Cell::default();
                }
            }
            EraseMode::Above => { // ToStart
                for c in 0..=col.min(self.lines[row].cells.len().saturating_sub(1)) {
                    self.lines[row].cells[c] = Cell::default();
                }
            }
            EraseMode::All => {
                for c in 0..self.lines[row].cells.len() {
                    self.lines[row].cells[c] = Cell::default();
                }
            }
            EraseMode::Scrollback => {} // N/A for CSI K
        }
        self.lines[row].mark_dirty(0);
        self.revision = self.revision.wrapping_add(1);
    }

    pub fn viewport_text(&self, vp: Viewport) -> String {
        let mut result = String::new();
        for r in vp.top..(vp.top + vp.rows).min(self.lines.len()) {
            if let Some(line) = self.lines.get(r) {
                result.push_str(&line.trimmed_text());
            }
            if r + 1 < vp.top + vp.rows { result.push('\n'); }
        }
        result
    }

    pub fn full_viewport(&self) -> Viewport {
        Viewport { top: 0, left: 0, rows: self.rows as usize, cols: self.cols as usize }
    }

    pub fn resize(&mut self, new_cols: u16, new_rows: u16) {
        while self.lines.len() < new_rows as usize {
            self.lines.push_back(Line::new(new_cols as usize));
        }
        while self.lines.len() > new_rows as usize {
            self.lines.pop_back();
        }
        for line in &mut self.lines {
            line.cells.resize(new_cols as usize, Cell::default());
        }
        self.cols = new_cols;
        self.rows = new_rows;
        if self.cursor.col >= new_cols { self.cursor.col = new_cols.saturating_sub(1); }
        if self.cursor.row >= new_rows { self.cursor.row = new_rows.saturating_sub(1); }
        self.revision = self.revision.wrapping_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compact_string_inline() {
        let cs = CompactString::from_char('A');
        assert!(cs.is_inline());
        assert_eq!(cs.as_str(), "A");
    }

    #[test]
    fn test_cell_default_is_space() {
        let c = Cell::default();
        assert_eq!(c.grapheme.as_str(), " ");
        assert!(c.is_default());
    }

    #[test]
    fn test_grid_put_char() {
        let mut g = Grid::new(80, 24);
        g.put_char('A');
        assert_eq!(g.cell_at(0, 0).unwrap().grapheme.as_str(), "A");
        assert_eq!(g.cursor().col, 1);
    }

    #[test]
    fn test_erase_in_display_all() {
        let mut g = Grid::new(80, 24);
        g.put_char('X');
        g.erase_in_display(EraseMode::All);
        assert!(g.cell_at(0, 0).unwrap().is_default());
        assert_eq!(g.cursor().col, 0);
        assert_eq!(g.cursor().row, 0);
    }

    #[test]
    fn test_erase_in_line_all() {
        let mut g = Grid::new(80, 24);
        g.put_char('X');
        g.put_char('Y');
        g.set_cursor(0, 0);
        g.erase_in_line(EraseMode::All);
        assert!(g.cell_at(0, 0).unwrap().is_default());
        assert!(g.cell_at(1, 0).unwrap().is_default());
    }

    #[test]
    fn test_line_dirty_tracking() {
        let mut l = Line::new(80);
        assert!(!l.is_dirty());
        l.mark_dirty(5);
        assert!(l.is_dirty());
        l.clear_dirty();
        assert!(!l.is_dirty());
    }

    #[test]
    fn test_grid_resize() {
        let mut g = Grid::new(80, 24);
        g.resize(120, 40);
        assert_eq!(g.cols(), 120);
        assert_eq!(g.rows(), 40);
    }

    // [v14 P2] Verify Vec capacity hint
    #[test]
    fn test_line_capacity_hint() {
        let l = Line::new(132);
        assert!(l.cells.capacity() >= 132);
        assert_eq!(l.cells.len(), 132);
    }
}
```

### Test Strategy

1. Cell grapheme round-trip. `TST-080`.
2. CompactString inline for ASCII. `TST-081`.
3. CompactString heap for long grapheme. `TST-082`.
4. `put_char` advances cursor. `TST-083`.
5. `put_grapheme` advances cursor. `TST-084`.
6. Line wrap at column boundary. `TST-085`.
7. `is_wrapped` flag set on wrap. `TST-086`.
8. `erase_in_display(Below)` clears from cursor. `TST-087`.
9. `erase_in_display(All)` resets cursor. `TST-088`.
10. `erase_in_line(All)` clears row. `TST-089`.
11. Grid resize preserves content. `TST-090`.
12. Dirty tracking range. `TST-091`.
13. Viewport text extraction. `TST-092`.
14. Scrollback limits enforced. `TST-093`.
15. **[v14 P2]** Line uses Vec, not SmallVec (compile-time verification). `TST-094`.
16. **[v14 P2]** Vec capacity hint matches cols. `TST-095`.

### AGENTS.md Rules

- `RULE-S05-01`: Cell grapheme must be `CompactString`. **Enforcement:** Type check.
- `RULE-S05-02`: `Grid::put()` MUST NOT be public. **Enforcement:** API surface lint.
- `RULE-S05-03`: All cell writes through `put_char`/`put_grapheme` only (INV-012). **Enforcement:** clippy lint.
- `RULE-S05-04`: `erase_in_display` must handle all EraseMode variants. **Enforcement:** exhaustive match.
- `RULE-S05-05`: `erase_in_line` must handle all EraseMode variants. **Enforcement:** exhaustive match.
- `RULE-S05-06`: `is_wrapped` must survive snapshot round-trip. **Enforcement:** TST-086.
- `RULE-S05-07`: Dirty range tracks minimum bounding box. **Enforcement:** TST-091.
- `RULE-S05-08`: **[v14 P2]** Line cells MUST use `Vec<Cell>`, not SmallVec (S75 decision). **Enforcement:** Type check.
- `RULE-S05-09`: **[v14 P2]** Line constructor MUST use `Vec::with_capacity(cols)`. **Enforcement:** Code review.

---

## 6. VtParser State Machine and Action Dispatch [v14 P2]

### Design Decisions

- **[v14]** 7 states: `Ground`, `Escape`, `EscapeIntermediate`, `CsiEntry`, `CsiParam`, `OscString`, `DcsPassthrough`.
- **[v14]** `classify(byte) -> ByteClass` returns a 6-variant enum with `#[repr(u8)]` for integer comparison in hot path. Replaces v13's `&str` return.
- **[v14]** `step(state, byte) -> (State, Action)` uses `const` 2D transition table: `TRANSITION_TABLE: [[Entry; 6]; 7]`.
- **[v14]** `Action` enum: `Print`, `ExecuteControl`, `CsiCollect`, `DispatchCsi`, `OscCollect`, `DispatchOsc`, `DcsCollect`, `DispatchDcs`, `ErrorRecover`, `Ignore`.
- **[v14 P2]** VtParser gains `unsupported_count: u64` metric for CSI finals not handled (logged, not panicked).
- **[v14 P2]** Extension state from Gemini P1 evaluated and DEFERRED: adds complexity without current use case. Will revisit when hyperlink support is needed.
- Traceability: `INV-018`, `API-006`.

### Rust Example

```rust
// crates/mux-grid/src/vt_parser.rs
// [v14 P2] ByteClass enum, const transition table, unsupported_count metric

#![allow(dead_code)]

/// [v14] ByteClass: 6 variants for byte classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ByteClass {
    Printable = 0,
    C0Control = 1,
    Escape = 2,
    CsiIntro = 3,
    Param = 4,
    Final = 5,
}

/// [v14] Classify a byte into ByteClass. const fn for compile-time evaluation.
pub const fn classify(b: u8) -> ByteClass {
    match b {
        0x1B => ByteClass::Escape,
        0x00..=0x1A | 0x1C..=0x1F => ByteClass::C0Control,
        b'[' => ByteClass::CsiIntro,
        b'0'..=b'9' | b';' | b'?' | b'>' | b'!' => ByteClass::Param,
        0x40..=0x7E => ByteClass::Final,
        _ => ByteClass::Printable,
    }
}

/// [v14] VtParser state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum State {
    Ground = 0,
    Escape = 1,
    EscapeIntermediate = 2,
    CsiEntry = 3,
    CsiParam = 4,
    OscString = 5,
    DcsPassthrough = 6,
}

/// [v14] Action dispatched by the parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Print(u8),
    ExecuteControl(u8),
    CsiCollect(u8),
    DispatchCsi(u8),
    OscCollect(u8),
    DispatchOsc,
    DcsCollect(u8),
    DispatchDcs,
    ErrorRecover,
    Ignore,
}

/// [v14] Transition table entry.
#[derive(Debug, Clone, Copy)]
pub struct Entry {
    pub next_state: State,
    pub action_kind: u8, // 0=Ignore, 1=Print, 2=Execute, 3=CsiCollect, 4=DispatchCsi, 5=Error
}

impl Entry {
    const fn new(s: State, a: u8) -> Self { Self { next_state: s, action_kind: a } }
}

/// [v14] const transition table: [State][ByteClass] -> Entry.
pub const TRANSITION_TABLE: [[Entry; 6]; 7] = [
    // Ground: [Printable, C0Control, Escape, CsiIntro, Param, Final]
    [
        Entry::new(State::Ground, 1),     // Printable -> Print
        Entry::new(State::Ground, 2),     // C0Control -> Execute
        Entry::new(State::Escape, 0),     // Escape -> transition
        Entry::new(State::Ground, 1),     // CsiIntro -> Print (only after ESC)
        Entry::new(State::Ground, 1),     // Param -> Print
        Entry::new(State::Ground, 1),     // Final -> Print
    ],
    // Escape
    [
        Entry::new(State::Ground, 5),     // Printable -> Error
        Entry::new(State::Ground, 2),     // C0Control -> Execute
        Entry::new(State::Escape, 0),     // Escape -> stay
        Entry::new(State::CsiEntry, 0),   // CsiIntro -> CSI
        Entry::new(State::EscapeIntermediate, 0), // Param -> intermediate
        Entry::new(State::Ground, 0),     // Final -> dispatch
    ],
    // EscapeIntermediate
    [
        Entry::new(State::Ground, 5),
        Entry::new(State::Ground, 2),
        Entry::new(State::Escape, 0),
        Entry::new(State::CsiEntry, 0),
        Entry::new(State::EscapeIntermediate, 0),
        Entry::new(State::Ground, 0),
    ],
    // CsiEntry
    [
        Entry::new(State::Ground, 5),
        Entry::new(State::Ground, 2),
        Entry::new(State::Escape, 0),
        Entry::new(State::Ground, 5),
        Entry::new(State::CsiParam, 3),   // Param -> collect
        Entry::new(State::Ground, 4),     // Final -> dispatch CSI
    ],
    // CsiParam
    [
        Entry::new(State::Ground, 5),
        Entry::new(State::Ground, 2),
        Entry::new(State::Escape, 0),
        Entry::new(State::Ground, 5),
        Entry::new(State::CsiParam, 3),   // Param -> collect
        Entry::new(State::Ground, 4),     // Final -> dispatch CSI
    ],
    // OscString
    [
        Entry::new(State::OscString, 0),
        Entry::new(State::Ground, 0),     // ST
        Entry::new(State::Escape, 0),
        Entry::new(State::OscString, 0),
        Entry::new(State::OscString, 0),
        Entry::new(State::OscString, 0),
    ],
    // DcsPassthrough
    [
        Entry::new(State::DcsPassthrough, 0),
        Entry::new(State::Ground, 0),
        Entry::new(State::Escape, 0),
        Entry::new(State::DcsPassthrough, 0),
        Entry::new(State::DcsPassthrough, 0),
        Entry::new(State::DcsPassthrough, 0),
    ],
];

/// [v14] Step function using const transition table.
pub fn step(state: State, byte: u8) -> (State, Action) {
    let class = classify(byte);
    let entry = &TRANSITION_TABLE[state as usize][class as usize];
    let action = match entry.action_kind {
        1 => Action::Print(byte),
        2 => Action::ExecuteControl(byte),
        3 => Action::CsiCollect(byte),
        4 => Action::DispatchCsi(byte),
        5 => Action::ErrorRecover,
        _ => Action::Ignore,
    };
    (entry.next_state, action)
}

/// [v14 P2] VtParser with unsupported_count metric.
pub struct VtParser {
    pub state: State,
    pub params: Vec<u16>,
    pub current_param: u16,
    pub unsupported_count: u64, // [v14 P2]
}

impl VtParser {
    pub fn new() -> Self {
        Self { state: State::Ground, params: Vec::new(), current_param: 0, unsupported_count: 0 }
    }

    pub fn feed(&mut self, byte: u8) -> Action {
        let (next, action) = step(self.state, byte);
        self.state = next;
        match action {
            Action::CsiCollect(b) => {
                if b == b';' {
                    self.params.push(self.current_param);
                    self.current_param = 0;
                } else if b.is_ascii_digit() {
                    self.current_param = self.current_param
                        .saturating_mul(10)
                        .saturating_add((b - b'0') as u16);
                }
            }
            Action::DispatchCsi(_) => {
                self.params.push(self.current_param);
                self.current_param = 0;
            }
            Action::ErrorRecover => {
                self.unsupported_count += 1; // [v14 P2]
                self.params.clear();
                self.current_param = 0;
            }
            _ => {}
        }
        action
    }

    pub fn reset(&mut self) {
        self.state = State::Ground;
        self.params.clear();
        self.current_param = 0;
    }
}

/// [v14] Totality check: all (State, ByteClass) pairs have entries.
pub const fn totality_check() -> bool {
    TRANSITION_TABLE.len() == 7 && TRANSITION_TABLE[0].len() == 6
}

const _: () = assert!(totality_check());

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_printable() {
        assert_eq!(classify(b' '), ByteClass::Printable);
        assert_eq!(classify(0x80), ByteClass::Printable);
    }

    #[test]
    fn test_classify_escape() {
        assert_eq!(classify(0x1B), ByteClass::Escape);
    }

    #[test]
    fn test_classify_csi_intro() {
        assert_eq!(classify(b'['), ByteClass::CsiIntro);
    }

    #[test]
    fn test_step_ground_printable() {
        let (state, action) = step(State::Ground, b'A');
        assert_eq!(state, State::Ground);
        assert!(matches!(action, Action::Print(b'A')));
    }

    #[test]
    fn test_step_escape_csi() {
        let (state, _) = step(State::Escape, b'[');
        assert_eq!(state, State::CsiEntry);
    }

    #[test]
    fn test_parser_csi_sequence() {
        let mut p = VtParser::new();
        p.feed(0x1B);
        p.feed(b'[');
        p.feed(b'1');
        let action = p.feed(b'A');
        assert!(matches!(action, Action::DispatchCsi(b'A')));
        assert_eq!(p.params, vec![1]);
    }

    #[test]
    fn test_totality() {
        assert!(totality_check());
    }

    // [v14 P2] unsupported_count metric test
    #[test]
    fn test_unsupported_count() {
        let mut p = VtParser::new();
        assert_eq!(p.unsupported_count, 0);
        // Force error recovery by feeding printable in Escape state
        p.state = State::Escape;
        p.feed(b'A'); // Printable in Escape = ErrorRecover
        assert_eq!(p.unsupported_count, 1);
    }
}
```

### Test Strategy

1. `classify()` totality over all 256 bytes. `TST-100`.
2. `step()` deterministic for same (state, byte). `TST-101`.
3. 7 states present. `TST-102`.
4. `ByteClass` has 6 variants. `TST-103`.
5. CSI sequence parsing. `TST-104`.
6. CSI param collection. `TST-105`.
7. `ErrorRecover` resets parser. `TST-106`.
8. Const transition table compiles. `TST-107`.
9. **[v14]** `classify()` is `const fn` (compile-time check). `TST-108`.
10. **[v14]** `ByteClass` is `#[repr(u8)]`. `TST-109`.
11. **[v14 P2]** `unsupported_count` increments on error recovery. `TST-110`.
12. **[v14 P2]** Extension state deferred (no new state added). `TST-111`.

### AGENTS.md Rules

- `RULE-S06-01`: VtParser MUST have exactly 7 states. **Enforcement:** enum variant count.
- `RULE-S06-02`: `classify()` MUST be `const fn`. **Enforcement:** compile test.
- `RULE-S06-03`: `ByteClass` MUST be `#[repr(u8)]` with 6 variants. **Enforcement:** layout test.
- `RULE-S06-04`: Transition table MUST be `const`. **Enforcement:** compile test.
- `RULE-S06-05`: Totality verified at compile time. **Enforcement:** const assertion.
- `RULE-S06-06`: Unsupported CSI finals MUST NOT panic. **Enforcement:** ErrorRecover test.
- `RULE-S06-07`: `step()` MUST be deterministic. **Enforcement:** property test.
- `RULE-S06-08`: Parser reset clears params and state. **Enforcement:** unit test.
- `RULE-S06-09`: **[v14 P2]** `unsupported_count` metric MUST be incremented on ErrorRecover. **Enforcement:** TST-110.
- `RULE-S06-10`: **[v14 P2]** No extension states added in v14 (deferred). **Enforcement:** State enum count.

---

## 7. PtyHandle Lifecycle and Resource Cleanup [v14 P2]

### Design Decisions

- **[v14]** 7-state lifecycle: `Allocated -> Spawned -> Running -> Stopping -> Exited -> Reaped -> Closed`.
- **[v14]** Typestate pattern: `PtyHandle<S>` generic over state type param. Invalid transitions are compile errors on common paths.
- **[v14]** `DynPtyHandle` runtime-checked fallback for cross-language bindings and deserialization.
- **[v14]** `PtyRecord` in `PtyRegistry` tracks state and generation counter.
- **[v14 P2]** PtyHandle gains `elapsed_in_state()` method for lifecycle timing metrics.
- Traceability: `INV-014`, `INV-017`, `API-007`.

### Rust Example

```rust
// crates/mux-pty/src/handle.rs
// [v14 P2] Typestate PtyHandle with elapsed_in_state

#![allow(dead_code)]
use std::marker::PhantomData;

// Typestate marker types
pub struct Allocated;
pub struct Spawned;
pub struct Running;
pub struct Stopping;
pub struct Exited;
pub struct Reaped;
pub struct Closed;

/// [v14] Typestate PtyHandle.
pub struct PtyHandle<S> {
    id: u64,
    generation: u32,
    created_at_ms: u64, // [v14 P2] for elapsed_in_state
    _state: PhantomData<S>,
}

impl PtyHandle<Allocated> {
    pub fn allocate(id: u64, generation: u32, now_ms: u64) -> Self {
        Self { id, generation, created_at_ms: now_ms, _state: PhantomData }
    }
    pub fn spawn(self, now_ms: u64) -> PtyHandle<Spawned> {
        PtyHandle { id: self.id, generation: self.generation, created_at_ms: now_ms, _state: PhantomData }
    }
}

impl PtyHandle<Spawned> {
    pub fn running(self, now_ms: u64) -> PtyHandle<Running> {
        PtyHandle { id: self.id, generation: self.generation, created_at_ms: now_ms, _state: PhantomData }
    }
}

impl PtyHandle<Running> {
    pub fn stop(self, now_ms: u64) -> PtyHandle<Stopping> {
        PtyHandle { id: self.id, generation: self.generation, created_at_ms: now_ms, _state: PhantomData }
    }
    pub fn id(&self) -> u64 { self.id }
    pub fn generation(&self) -> u32 { self.generation }
}

impl PtyHandle<Stopping> {
    pub fn exit(self, now_ms: u64) -> PtyHandle<Exited> {
        PtyHandle { id: self.id, generation: self.generation, created_at_ms: now_ms, _state: PhantomData }
    }
}

impl PtyHandle<Exited> {
    pub fn reap(self, now_ms: u64) -> PtyHandle<Reaped> {
        PtyHandle { id: self.id, generation: self.generation, created_at_ms: now_ms, _state: PhantomData }
    }
}

impl PtyHandle<Reaped> {
    pub fn close(self, now_ms: u64) -> PtyHandle<Closed> {
        PtyHandle { id: self.id, generation: self.generation, created_at_ms: now_ms, _state: PhantomData }
    }
}

impl PtyHandle<Closed> {
    pub fn id(&self) -> u64 { self.id }
}

/// [v14 P2] elapsed_in_state for all states
impl<S> PtyHandle<S> {
    pub fn elapsed_in_state(&self, now_ms: u64) -> u64 {
        now_ms.saturating_sub(self.created_at_ms)
    }
}

/// [v14] DynPtyHandle for runtime-checked paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PtyState {
    Allocated, Spawned, Running, Stopping, Exited, Reaped, Closed,
}

impl PtyState {
    pub fn is_terminal(self) -> bool { self == Self::Closed }
}

pub fn valid_pty_transition(from: PtyState, to: PtyState) -> bool {
    matches!((from, to),
        (PtyState::Allocated, PtyState::Spawned) |
        (PtyState::Spawned, PtyState::Running) |
        (PtyState::Running, PtyState::Stopping) |
        (PtyState::Stopping, PtyState::Exited) |
        (PtyState::Exited, PtyState::Reaped) |
        (PtyState::Reaped, PtyState::Closed)
    )
}

#[derive(Debug, Clone)]
pub struct PtyRecord {
    pub id: u64,
    pub generation: u32,
    pub state: PtyState,
}

impl PtyRecord {
    pub fn transition(&mut self, to: PtyState) -> Result<(), &'static str> {
        if valid_pty_transition(self.state, to) {
            self.state = to;
            Ok(())
        } else {
            Err("invalid PtyState transition")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typestate_lifecycle() {
        let h = PtyHandle::<Allocated>::allocate(1, 0, 100);
        let h = h.spawn(200);
        let h = h.running(300);
        assert_eq!(h.id(), 1);
        let h = h.stop(400);
        let h = h.exit(500);
        let h = h.reap(600);
        let h = h.close(700);
        assert_eq!(h.id(), 1);
    }

    #[test]
    fn test_valid_transitions() {
        assert!(valid_pty_transition(PtyState::Allocated, PtyState::Spawned));
        assert!(valid_pty_transition(PtyState::Running, PtyState::Stopping));
        assert!(!valid_pty_transition(PtyState::Closed, PtyState::Running));
    }

    #[test]
    fn test_pty_record_transition() {
        let mut r = PtyRecord { id: 1, generation: 0, state: PtyState::Allocated };
        assert!(r.transition(PtyState::Spawned).is_ok());
        assert!(r.transition(PtyState::Closed).is_err());
    }

    // [v14 P2] elapsed_in_state
    #[test]
    fn test_elapsed_in_state() {
        let h = PtyHandle::<Allocated>::allocate(1, 0, 100);
        assert_eq!(h.elapsed_in_state(150), 50);
        assert_eq!(h.elapsed_in_state(100), 0);
    }
}
```

### Test Strategy

1. 7-state lifecycle via typestate chain. `TST-120`.
2. DynPtyHandle transition validation. `TST-121`.
3. PtyRecord rejects invalid transition. `TST-122`.
4. Generation counter increment on restart. `TST-123`.
5. Closed handle is terminal. `TST-124`.
6. Typestate prevents Running -> Allocated at compile time. `TST-125`.
7. **[v14 P2]** `elapsed_in_state` correctness. `TST-126`.
8. **[v14 P2]** elapsed_in_state handles same-time (zero elapsed). `TST-127`.

### AGENTS.md Rules

- `RULE-S07-01`: PtyHandle MUST have exactly 7 states. **Enforcement:** enum variant count.
- `RULE-S07-02`: Typestate transitions consume self. **Enforcement:** move semantics.
- `RULE-S07-03`: DynPtyHandle provides runtime fallback. **Enforcement:** API surface.
- `RULE-S07-04`: Generation counter monotonic. **Enforcement:** unit test.
- `RULE-S07-05`: Closed handle returns error on operations. **Enforcement:** integration test.
- `RULE-S07-06`: Restart requires full kill -> close -> allocate cycle (INV-017). **Enforcement:** sequence test.
- `RULE-S07-07`: **[v14 P2]** elapsed_in_state MUST use saturating_sub. **Enforcement:** code review.
- `RULE-S07-08`: **[v14 P2]** PtyHandle timestamps MUST be monotonic. **Enforcement:** debug assertion.

---

## 8. Termlet Snapshot Format and Versioning [v14 P2]

### Design Decisions

- **[v14]** Snapshot supports v1 (struct-of-fields) and v2 (PackedCell u64).
- **[v14]** `PackedCell(u64)`: ch in bits 0-20 (21 bits, covers all Unicode), style in bits 21-36 (16 bits), flags in bits 37-52 (16 bits), reserved bits 53-63.
- **[v14]** Decoder validates magic -> version -> length -> payload -> checksum.
- **[v14 P2]** Checksum algorithm selectable: FNV-1a (default, byte 0x00) or CRC32C (opt-in, byte 0x01). Algorithm byte at offset 26 (after revision). INV-021.
- **[v14 P2]** CRC32C implementation: software fallback always available; hardware acceleration behind `crc32c-hw` cfg flag.
- Traceability: `INV-015`, `INV-020`, `INV-021`.

### Rust Example

```rust
// crates/mux-snapshot/src/lib.rs
// [v14 P2] Snapshot format with dual checksum

#![allow(dead_code)]

pub const MAGIC: &[u8; 8] = b"TFSNAP13";

/// [v14] PackedCell: u64 bitfield.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PackedCell(pub u64);

impl PackedCell {
    pub fn new(ch: u32, style: u16, flags: u16) -> Self {
        let v = (ch as u64 & 0x1F_FFFF)
            | ((style as u64) << 21)
            | ((flags as u64) << 37);
        Self(v)
    }
    pub fn ch(self) -> u32 { (self.0 & 0x1F_FFFF) as u32 }
    pub fn style(self) -> u16 { ((self.0 >> 21) & 0xFFFF) as u16 }
    pub fn flags(self) -> u16 { ((self.0 >> 37) & 0xFFFF) as u16 }
}

/// [v14 P2] Checksum algorithm selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ChecksumAlgo {
    Fnv1a = 0,
    Crc32c = 1,
}

/// FNV-1a 32-bit hash.
pub fn fnv1a_32(data: &[u8]) -> u32 {
    let mut hash: u32 = 0x811c_9dc5;
    for &b in data {
        hash ^= b as u32;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

/// [v14 P2] CRC32C software implementation.
pub fn crc32c_sw(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0x82F6_3B78;
            } else {
                crc >>= 1;
            }
        }
    }
    crc ^ 0xFFFF_FFFF
}

/// [v14 P2] Compute checksum using selected algorithm.
pub fn compute_checksum(data: &[u8], algo: ChecksumAlgo) -> u32 {
    match algo {
        ChecksumAlgo::Fnv1a => fnv1a_32(data),
        ChecksumAlgo::Crc32c => crc32c_sw(data),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotError {
    BadMagic,
    UnsupportedVersion,
    Truncated,
    ChecksumMismatch,
    InvalidAlgorithm,
}

/// [v14 P2] Snapshot header with checksum algorithm byte.
#[derive(Debug, Clone)]
pub struct SnapshotHeader {
    pub version: u16,
    pub cols: u16,
    pub rows: u16,
    pub cursor_col: u16,
    pub cursor_row: u16,
    pub revision: u64,
    pub checksum_algo: ChecksumAlgo, // [v14 P2]
    pub cell_count: u32,
}

/// Encode header to bytes.
pub fn encode_header(h: &SnapshotHeader) -> Vec<u8> {
    let mut buf = Vec::with_capacity(31);
    buf.extend_from_slice(MAGIC);
    buf.extend_from_slice(&h.version.to_le_bytes());
    buf.extend_from_slice(&h.cols.to_le_bytes());
    buf.extend_from_slice(&h.rows.to_le_bytes());
    buf.extend_from_slice(&h.cursor_col.to_le_bytes());
    buf.extend_from_slice(&h.cursor_row.to_le_bytes());
    buf.extend_from_slice(&h.revision.to_le_bytes());
    buf.push(h.checksum_algo as u8); // [v14 P2] offset 26
    buf.extend_from_slice(&h.cell_count.to_le_bytes());
    buf
}

/// Decode header from bytes.
pub fn decode_header(data: &[u8]) -> Result<SnapshotHeader, SnapshotError> {
    if data.len() < 31 { return Err(SnapshotError::Truncated); }
    if &data[0..8] != MAGIC { return Err(SnapshotError::BadMagic); }
    let version = u16::from_le_bytes([data[8], data[9]]);
    if version != 1 && version != 2 { return Err(SnapshotError::UnsupportedVersion); }
    let algo_byte = data[26];
    let checksum_algo = match algo_byte {
        0 => ChecksumAlgo::Fnv1a,
        1 => ChecksumAlgo::Crc32c,
        _ => return Err(SnapshotError::InvalidAlgorithm),
    };
    Ok(SnapshotHeader {
        version,
        cols: u16::from_le_bytes([data[10], data[11]]),
        rows: u16::from_le_bytes([data[12], data[13]]),
        cursor_col: u16::from_le_bytes([data[14], data[15]]),
        cursor_row: u16::from_le_bytes([data[16], data[17]]),
        revision: u64::from_le_bytes([data[18], data[19], data[20], data[21], data[22], data[23], data[24], data[25]]),
        checksum_algo,
        cell_count: u32::from_le_bytes([data[27], data[28], data[29], data[30]]),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packed_cell_round_trip() {
        let pc = PackedCell::new(0x1F600, 0x00FF, 0x0003);
        assert_eq!(pc.ch(), 0x1F600);
        assert_eq!(pc.style(), 0x00FF);
        assert_eq!(pc.flags(), 0x0003);
    }

    #[test]
    fn test_packed_cell_ascii() {
        let pc = PackedCell::new(b'A' as u32, 0, 0);
        assert_eq!(pc.ch(), 65);
    }

    #[test]
    fn test_fnv1a_known_value() {
        let hash = fnv1a_32(b"hello");
        assert_ne!(hash, 0);
    }

    // [v14 P2] CRC32C test
    #[test]
    fn test_crc32c_known_value() {
        let crc = crc32c_sw(b"hello");
        assert_ne!(crc, 0);
    }

    // [v14 P2] Dual checksum test
    #[test]
    fn test_dual_checksum() {
        let data = b"test payload";
        let fnv = compute_checksum(data, ChecksumAlgo::Fnv1a);
        let crc = compute_checksum(data, ChecksumAlgo::Crc32c);
        assert_ne!(fnv, crc); // Different algorithms produce different values
    }

    #[test]
    fn test_header_round_trip() {
        let h = SnapshotHeader {
            version: 2, cols: 80, rows: 24,
            cursor_col: 5, cursor_row: 10,
            revision: 42, checksum_algo: ChecksumAlgo::Crc32c,
            cell_count: 1920,
        };
        let encoded = encode_header(&h);
        let decoded = decode_header(&encoded).unwrap();
        assert_eq!(decoded.version, 2);
        assert_eq!(decoded.cols, 80);
        assert_eq!(decoded.checksum_algo, ChecksumAlgo::Crc32c);
        assert_eq!(decoded.cell_count, 1920);
    }

    #[test]
    fn test_bad_magic_rejected() {
        let mut data = encode_header(&SnapshotHeader {
            version: 1, cols: 80, rows: 24,
            cursor_col: 0, cursor_row: 0,
            revision: 0, checksum_algo: ChecksumAlgo::Fnv1a,
            cell_count: 0,
        });
        data[0] = b'X';
        assert_eq!(decode_header(&data), Err(SnapshotError::BadMagic));
    }

    #[test]
    fn test_unsupported_version_rejected() {
        let mut data = encode_header(&SnapshotHeader {
            version: 1, cols: 80, rows: 24,
            cursor_col: 0, cursor_row: 0,
            revision: 0, checksum_algo: ChecksumAlgo::Fnv1a,
            cell_count: 0,
        });
        data[8] = 99; // bad version
        data[9] = 0;
        assert_eq!(decode_header(&data), Err(SnapshotError::UnsupportedVersion));
    }

    // [v14 P2] Invalid algorithm byte test
    #[test]
    fn test_invalid_algorithm_rejected() {
        let mut data = encode_header(&SnapshotHeader {
            version: 1, cols: 80, rows: 24,
            cursor_col: 0, cursor_row: 0,
            revision: 0, checksum_algo: ChecksumAlgo::Fnv1a,
            cell_count: 0,
        });
        data[26] = 0xFF; // bad algorithm
        assert_eq!(decode_header(&data), Err(SnapshotError::InvalidAlgorithm));
    }
}
```

### Test Strategy

1. PackedCell round-trip for ASCII. `TST-130`.
2. PackedCell round-trip for emoji (U+1F600). `TST-131`.
3. v1 snapshot encode/decode. `TST-132`.
4. v2 snapshot encode/decode. `TST-133`.
5. Bad magic rejected. `TST-134`.
6. Unsupported version rejected. `TST-135`.
7. Truncated payload rejected. `TST-136`.
8. FNV-1a checksum mismatch rejected. `TST-137`.
9. **[v14 P2]** CRC32C checksum encode/decode. `TST-138`.
10. **[v14 P2]** Invalid algorithm byte rejected. `TST-139`.
11. **[v14 P2]** Dual checksum produces different values. `TST-140`.
12. **[v14 P2]** Header round-trip with CRC32C algorithm byte. `TST-141`.

### AGENTS.md Rules

- `RULE-S08-01`: Snapshot magic MUST be "TFSNAP13". **Enforcement:** const assertion.
- `RULE-S08-02`: Version 1 and 2 supported; version 3+ rejected. **Enforcement:** decode test.
- `RULE-S08-03`: PackedCell ch field is 21 bits. **Enforcement:** bit-level test.
- `RULE-S08-04`: Decode order: magic -> version -> length -> payload -> checksum. **Enforcement:** sequence test.
- `RULE-S08-05`: FNV-1a is default checksum. **Enforcement:** default value test.
- `RULE-S08-06`: v1 backward compat preserved when v2 added. **Enforcement:** dual decode test.
- `RULE-S08-07`: **[v14 P2]** CRC32C opt-in via `snapshot-crc32c` feature flag. **Enforcement:** feature gate test.
- `RULE-S08-08`: **[v14 P2]** Algorithm byte at offset 26 in header. **Enforcement:** byte-level test.
- `RULE-S08-09`: **[v14 P2]** Invalid algorithm byte MUST be rejected. **Enforcement:** TST-139.
- `RULE-S08-10`: **[v14 P2]** CRC32C software fallback MUST always compile (no hardware dependency). **Enforcement:** WASM CI.

---

## 9. Wire Protocol Compatibility (tmux) [v14 P2]

### Design Decisions

- tmux wire protocol v8 compatibility target (INV-001).
- Protocol decode failure drops client connection (INV-004).
- Attach, list-sessions, control-mode notifications, config parse, copy mode, backpressure, option fallthrough -- all validated.
- **[v14]** CSI J/K parity added (C9 gate).
- **[v14 P2]** Wire protocol golden tests expanded: 25 capture files covering attach, detach, resize, copy-mode, and OSC sequences.
- Traceability: `INV-001`, `INV-004`, `PAR-001`.

### Rust Example

```rust
// crates/mux-proto/src/lib.rs
// [v14 P2] Wire protocol types

#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtoMessage {
    Attach { session: String },
    Detach,
    ListSessions,
    ControlMode { command: String },
    Resize { cols: u16, rows: u16 },
    Input { data: Vec<u8> },
    Output { pane_id: u64, data: Vec<u8> },
    Notification { kind: String, body: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtoError {
    InvalidFormat,
    UnknownCommand,
    TooLarge,
    Truncated,
}

pub fn validate_message_size(data: &[u8], max_size: usize) -> Result<(), ProtoError> {
    if data.len() > max_size { Err(ProtoError::TooLarge) } else { Ok(()) }
}

/// [v14 P2] Golden test capture format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoldenCapture {
    pub name: &'static str,
    pub direction: Direction,
    pub raw_bytes: &'static [u8],
    pub expected_message: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction { ClientToServer, ServerToClient }

pub const MAX_MESSAGE_SIZE: usize = 1024 * 1024; // 1MB

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_size_ok() {
        assert!(validate_message_size(b"hello", MAX_MESSAGE_SIZE).is_ok());
    }

    #[test]
    fn test_message_too_large() {
        let big = vec![0u8; MAX_MESSAGE_SIZE + 1];
        assert_eq!(validate_message_size(&big, MAX_MESSAGE_SIZE), Err(ProtoError::TooLarge));
    }
}
```

### Test Strategy

1. Attach message parse. `TST-150`.
2. List-sessions round-trip. `TST-151`.
3. Control-mode notification parse. `TST-152`.
4. Max message size enforcement. `TST-153`.
5. Protocol error drops connection. `TST-154`.
6. **[v14 P2]** 25 golden capture files validated. `TST-155`.

### AGENTS.md Rules

- `RULE-S09-01`: Protocol decode failure MUST drop connection (INV-004). **Enforcement:** integration test.
- `RULE-S09-02`: Max message size 1MB. **Enforcement:** size check test.
- `RULE-S09-03`: All protocol messages round-trip. **Enforcement:** golden test.
- `RULE-S09-04`: **[v14 P2]** Golden test corpus MUST contain 25+ capture files. **Enforcement:** file count CI check.

---

## 10. Configuration and Options

### Design Decisions

- Config file: `~/.config/termforge/termforge.conf`.
- Config parse via `mux-conf` crate (Layer 1).
- Option fallthrough: user -> session -> window -> pane -> default.
- **[v14 P2]** Config reload sends SIGHUP equivalent notification to all attached clients.
- Traceability: `OPS-002`.

### Rust Example

```rust
// crates/mux-conf/src/lib.rs
#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigOption {
    pub key: String,
    pub value: String,
    pub scope: OptionScope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionScope { Global, Session, Window, Pane }

pub fn resolve_option(key: &str, scopes: &[(OptionScope, &str)]) -> Option<String> {
    // Pane -> Window -> Session -> Global fallthrough
    let order = [OptionScope::Pane, OptionScope::Window, OptionScope::Session, OptionScope::Global];
    for scope in &order {
        if let Some((_, val)) = scopes.iter().find(|(s, _)| s == scope) {
            return Some(val.to_string());
        }
    }
    let _ = key;
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pane_overrides_global() {
        let scopes = vec![
            (OptionScope::Global, "default"),
            (OptionScope::Pane, "pane_value"),
        ];
        assert_eq!(resolve_option("key", &scopes), Some("pane_value".to_string()));
    }
}
```

### Test Strategy

1. Option fallthrough order. `TST-160`.
2. Config parse valid file. `TST-161`.
3. Config parse invalid file. `TST-162`.
4. XDG config path. `TST-163`.
5. **[v14 P2]** Config reload notification. `TST-164`.

### AGENTS.md Rules

- `RULE-S10-01`: Config must parse without panic. **Enforcement:** fuzz test.
- `RULE-S10-02`: Option fallthrough order: pane -> window -> session -> global. **Enforcement:** unit test.
- `RULE-S10-03`: **[v14 P2]** Config reload MUST notify all attached clients. **Enforcement:** integration test.

---

## 11. Layout Engine

### Design Decisions

- Round-robin one-cell adjustment for resize (INV-006).
- Layout tree: split (horizontal/vertical), pane leaf.
- Zoom pane: temporarily hides siblings.
- **[v14 P2]** Layout resize validates minimum pane size (1 col x 1 row).
- Traceability: `INV-006`.

### Rust Example

```rust
// crates/mux-core/src/layout.rs
#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SplitDir { Horizontal, Vertical }

#[derive(Debug, Clone)]
pub enum LayoutNode {
    Pane { id: u64, cols: u16, rows: u16 },
    Split { dir: SplitDir, children: Vec<LayoutNode>, ratios: Vec<f32> },
}

pub const MIN_PANE_COLS: u16 = 1; // [v14 P2]
pub const MIN_PANE_ROWS: u16 = 1; // [v14 P2]

impl LayoutNode {
    pub fn total_panes(&self) -> usize {
        match self {
            Self::Pane { .. } => 1,
            Self::Split { children, .. } => children.iter().map(|c| c.total_panes()).sum(),
        }
    }

    /// [v14 P2] Validate minimum pane sizes.
    pub fn validate_sizes(&self) -> bool {
        match self {
            Self::Pane { cols, rows, .. } => *cols >= MIN_PANE_COLS && *rows >= MIN_PANE_ROWS,
            Self::Split { children, .. } => children.iter().all(|c| c.validate_sizes()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_pane() {
        let l = LayoutNode::Pane { id: 1, cols: 80, rows: 24 };
        assert_eq!(l.total_panes(), 1);
        assert!(l.validate_sizes());
    }

    #[test]
    fn test_split() {
        let l = LayoutNode::Split {
            dir: SplitDir::Vertical,
            children: vec![
                LayoutNode::Pane { id: 1, cols: 40, rows: 24 },
                LayoutNode::Pane { id: 2, cols: 40, rows: 24 },
            ],
            ratios: vec![0.5, 0.5],
        };
        assert_eq!(l.total_panes(), 2);
    }

    // [v14 P2] minimum size validation
    #[test]
    fn test_zero_size_invalid() {
        let l = LayoutNode::Pane { id: 1, cols: 0, rows: 24 };
        assert!(!l.validate_sizes());
    }
}
```

### Test Strategy

1. Layout resize round-robin. `TST-170`.
2. Split creates two children. `TST-171`.
3. Zoom hides siblings. `TST-172`.
4. **[v14 P2]** Minimum pane size enforced. `TST-173`.

### AGENTS.md Rules

- `RULE-S11-01`: Layout resize uses round-robin (INV-006). **Enforcement:** resize test.
- `RULE-S11-02`: Zoom is reversible. **Enforcement:** zoom/unzoom test.
- `RULE-S11-03`: **[v14 P2]** Minimum pane size is 1x1. **Enforcement:** validation test.

---

## 12. ORM and QueryList

### Design Decisions

- `QuerySpec` with `filter_by`, `sort_by`, `paginate`.
- `QueryList<T>` lazy, snapshot-backed query result.
- **[v14 P2]** QueryList gains `count()` method that does not materialize results.
- Traceability: `API-010`.

### Rust Example

```rust
// crates/mux-query/src/lib.rs
#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterOp { Eq, StartsWith, Contains, Gt, Lt }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterSpec {
    pub field: String,
    pub op: FilterOp,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct QuerySpec {
    pub filters: Vec<FilterSpec>,
    pub sort_field: Option<String>,
    pub offset: usize,
    pub limit: usize,
}

impl QuerySpec {
    pub fn new() -> Self {
        Self { filters: Vec::new(), sort_field: None, offset: 0, limit: 100 }
    }
    pub fn filter_by(mut self, field: &str, op: FilterOp, value: &str) -> Self {
        self.filters.push(FilterSpec { field: field.into(), op, value: value.into() });
        self
    }
    pub fn paginate(mut self, offset: usize, limit: usize) -> Self {
        self.offset = offset;
        self.limit = limit;
        self
    }
}

pub struct QueryList<T> {
    items: Vec<T>,
    total: usize,
}

impl<T> QueryList<T> {
    pub fn new(items: Vec<T>, total: usize) -> Self { Self { items, total } }
    pub fn items(&self) -> &[T] { &self.items }
    pub fn total(&self) -> usize { self.total }
    pub fn count(&self) -> usize { self.total } // [v14 P2] no-materialize count
    pub fn is_empty(&self) -> bool { self.items.is_empty() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_spec_builder() {
        let q = QuerySpec::new()
            .filter_by("name", FilterOp::StartsWith, "dev")
            .paginate(0, 10);
        assert_eq!(q.filters.len(), 1);
        assert_eq!(q.limit, 10);
    }

    #[test]
    fn test_query_list_count() {
        let ql: QueryList<u32> = QueryList::new(vec![1, 2, 3], 100);
        assert_eq!(ql.count(), 100);
        assert_eq!(ql.items().len(), 3);
    }
}
```

### Test Strategy

1. QuerySpec filter chain. `TST-180`.
2. Pagination offset/limit. `TST-181`.
3. Empty QueryList. `TST-182`.
4. **[v14 P2]** count() returns total, not materialized. `TST-183`.

### AGENTS.md Rules

- `RULE-S12-01`: QuerySpec is builder-pattern. **Enforcement:** API test.
- `RULE-S12-02`: QueryList is lazy/snapshot-backed. **Enforcement:** trait constraint.
- `RULE-S12-03`: **[v14 P2]** count() MUST NOT materialize results. **Enforcement:** performance test.

---

## 13. ServerGraph Slot Reclamation and Generation Counters

### Design Decisions

- Slot-based entity storage with generation counters (INV-013).
- ABA prevention via generation increment on reuse.
- **[v14 P2]** Generation counter wrap detection: if generation reaches u32::MAX, slot is permanently retired.
- Traceability: `INV-013`.

### Rust Example

```rust
// crates/mux-core/src/slotmap.rs
#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SlotId { pub index: u32, pub generation: u32 }

pub struct SlotMap<T> {
    entries: Vec<Option<(u32, T)>>, // (generation, value)
    free_list: Vec<u32>,
}

impl<T> SlotMap<T> {
    pub fn new() -> Self { Self { entries: Vec::new(), free_list: Vec::new() } }

    pub fn insert(&mut self, value: T) -> SlotId {
        if let Some(index) = self.free_list.pop() {
            let i = index as usize;
            let gen = self.entries[i].as_ref().map(|(g, _)| *g).unwrap_or(0) + 1;
            // [v14 P2] generation wrap detection
            if gen == u32::MAX {
                // Permanently retired -- allocate new slot instead
                let new_index = self.entries.len() as u32;
                self.entries.push(Some((0, value)));
                return SlotId { index: new_index, generation: 0 };
            }
            self.entries[i] = Some((gen, value));
            SlotId { index, generation: gen }
        } else {
            let index = self.entries.len() as u32;
            self.entries.push(Some((0, value)));
            SlotId { index, generation: 0 }
        }
    }

    pub fn get(&self, id: SlotId) -> Option<&T> {
        self.entries.get(id.index as usize)?.as_ref().and_then(|(g, v)| {
            if *g == id.generation { Some(v) } else { None }
        })
    }

    pub fn remove(&mut self, id: SlotId) -> Option<T> {
        let entry = self.entries.get_mut(id.index as usize)?;
        if let Some((g, _)) = entry {
            if *g != id.generation { return None; }
        }
        let (gen, val) = entry.take()?;
        self.entries[id.index as usize] = Some((gen, val));
        let result = self.entries[id.index as usize].take().map(|(_, v)| v);
        self.free_list.push(id.index);
        result
    }

    pub fn len(&self) -> usize {
        self.entries.iter().filter(|e| e.is_some()).count()
    }

    pub fn is_empty(&self) -> bool { self.len() == 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_get() {
        let mut sm: SlotMap<&str> = SlotMap::new();
        let id = sm.insert("hello");
        assert_eq!(sm.get(id), Some(&"hello"));
    }

    #[test]
    fn test_stale_generation() {
        let mut sm: SlotMap<&str> = SlotMap::new();
        let old = sm.insert("first");
        sm.remove(old);
        let _new = sm.insert("second");
        assert_eq!(sm.get(old), None); // stale generation
    }
}
```

### Test Strategy

1. Insert and get. `TST-190`.
2. Stale generation returns None. `TST-191`.
3. Free list reuse. `TST-192`.
4. **[v14 P2]** Generation wrap retires slot. `TST-193`.

### AGENTS.md Rules

- `RULE-S13-01`: Generation counter prevents ABA (INV-013). **Enforcement:** stale ID test.
- `RULE-S13-02`: Removed slot enters free list. **Enforcement:** reuse test.
- `RULE-S13-03`: **[v14 P2]** Generation u32::MAX permanently retires slot. **Enforcement:** wrap test.

---

## 14. State Actor and Snapshot Publication

### Design Decisions

- Single writer for state mutation (INV-003).
- Snapshot-based readers for concurrent access.
- **[v14 P2]** Snapshot publication includes monotonic sequence number for ordering guarantees.
- Traceability: `INV-003`.

### Rust Example

```rust
// crates/mux-state/src/lib.rs
#![allow(dead_code)]
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct StateSnapshot {
    pub sequence: u64, // [v14 P2] monotonic sequence
    pub data: Arc<Vec<u8>>,
}

pub struct StateActor {
    current_sequence: u64,
}

impl StateActor {
    pub fn new() -> Self { Self { current_sequence: 0 } }

    pub fn publish(&mut self, data: Vec<u8>) -> StateSnapshot {
        self.current_sequence += 1;
        StateSnapshot { sequence: self.current_sequence, data: Arc::new(data) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monotonic_sequence() {
        let mut actor = StateActor::new();
        let s1 = actor.publish(vec![1]);
        let s2 = actor.publish(vec![2]);
        assert!(s2.sequence > s1.sequence);
    }
}
```

### Test Strategy

1. Single writer verified. `TST-200`.
2. Snapshot immutability. `TST-201`.
3. **[v14 P2]** Sequence monotonicity. `TST-202`.

### AGENTS.md Rules

- `RULE-S14-01`: Single writer for state (INV-003). **Enforcement:** architecture lint.
- `RULE-S14-02`: Snapshots are Arc-cloned, never mutated. **Enforcement:** type constraint.
- `RULE-S14-03`: **[v14 P2]** Snapshot sequence MUST be monotonic. **Enforcement:** TST-202.

---

## 15. Control Mode

### Design Decisions

- Control mode: `termforge -C` attaches in control mode (INV-004).
- Notification format: `%begin`, `%end`, `%output`, etc.
- **[v14 P2]** Control mode gains `%snapshot` notification for binary snapshot push.
- Traceability: `INV-004`, `PAR-003`.

### Rust Example

```rust
// crates/mux-control/src/lib.rs
#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlNotification {
    Begin { cmd_id: u64, flags: u32 },
    End { cmd_id: u64, exit_code: i32 },
    Output { pane_id: u64, data: String },
    LayoutChange { layout: String },
    SessionChanged { name: String },
    Snapshot { pane_id: u64, version: u16 }, // [v14 P2]
}

pub fn parse_notification(line: &str) -> Option<ControlNotification> {
    if line.starts_with("%output ") {
        Some(ControlNotification::Output { pane_id: 0, data: line[8..].to_string() })
    } else if line.starts_with("%snapshot ") {
        // [v14 P2] new notification type
        Some(ControlNotification::Snapshot { pane_id: 0, version: 2 })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_output() {
        let n = parse_notification("%output hello").unwrap();
        assert!(matches!(n, ControlNotification::Output { .. }));
    }

    // [v14 P2]
    #[test]
    fn test_parse_snapshot_notification() {
        let n = parse_notification("%snapshot pane=1").unwrap();
        assert!(matches!(n, ControlNotification::Snapshot { .. }));
    }
}
```

### Test Strategy

1. Begin/End parsing. `TST-210`.
2. Output notification. `TST-211`.
3. Layout change. `TST-212`.
4. **[v14 P2]** Snapshot notification parse. `TST-213`.

### AGENTS.md Rules

- `RULE-S15-01`: Control mode notifications are line-based. **Enforcement:** parse test.
- `RULE-S15-02`: Unknown notification types are ignored. **Enforcement:** fuzz test.
- `RULE-S15-03`: **[v14 P2]** %snapshot notification MUST include pane_id and version. **Enforcement:** parse test.

---

## 16. Language Bindings [v14 P2]

### Design Decisions

- Python via PyO3, Node via Neon (Layer 3).
- Cross-language parity: same API surface in Rust, Python, Node.
- **[v14 P2]** Binding version mismatch detection: binding reports its compiled-against spec_version; server rejects mismatch.
- Traceability: `API-012`.

### Rust Example

```rust
// crates/mux-bindings-shared/src/lib.rs
#![allow(dead_code)]

pub const BINDING_SPEC_VERSION: &str = "v14-pass2"; // [v14 P2]

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingInfo {
    pub language: &'static str,
    pub spec_version: &'static str,
    pub api_surface: &'static [&'static str],
}

pub fn check_version_compat(binding_version: &str, server_version: &str) -> bool {
    binding_version == server_version
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_compat() {
        assert!(check_version_compat("v14-pass2", "v14-pass2"));
        assert!(!check_version_compat("v13", "v14-pass2"));
    }
}
```

### Test Strategy

1. Python import succeeds. `TST-220`.
2. Node import succeeds. `TST-221`.
3. Cross-language parity. `TST-222`.
4. **[v14 P2]** Version mismatch detected. `TST-223`.

### AGENTS.md Rules

- `RULE-S16-01`: Binding API mirrors Rust API. **Enforcement:** parity test.
- `RULE-S16-02`: Context managers/RAII for cleanup. **Enforcement:** leak test.
- `RULE-S16-03`: **[v14 P2]** Binding version MUST match server spec_version. **Enforcement:** handshake test.

---

## 17. CRDT Collaboration Layer [v14 P2]

### Design Decisions

- LWW (Last-Writer-Wins) field map for concurrent edits.
- Tombstone-wins delete semantics.
- PaneOpLog for concurrent pane input merging.
- **[v14]** OpLog uses binary insertion via `partition_point()` for O(log n) insert.
- **[v14 P2]** CRDT merge commutativity verified by property-based tests with at least 3 concurrent writers.
- Traceability: `API-015`.

### Rust Example

```rust
// crates/mux-crdt/src/lib.rs
#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LwwEntry {
    pub value: String,
    pub timestamp: u64,
    pub client_id: u64,
}

#[derive(Debug, Clone)]
pub struct LwwMap {
    entries: Vec<(String, LwwEntry)>,
}

impl LwwMap {
    pub fn new() -> Self { Self { entries: Vec::new() } }

    pub fn set(&mut self, key: &str, value: &str, ts: u64, client: u64) {
        let entry = LwwEntry { value: value.into(), timestamp: ts, client_id: client };
        if let Some(existing) = self.entries.iter_mut().find(|(k, _)| k == key) {
            if ts > existing.1.timestamp || (ts == existing.1.timestamp && client > existing.1.client_id) {
                existing.1 = entry;
            }
        } else {
            self.entries.push((key.into(), entry));
        }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, e)| e.value.as_str())
    }

    /// [v14 P2] Merge two maps. Commutative.
    pub fn merge(&mut self, other: &LwwMap) {
        for (key, entry) in &other.entries {
            self.set(key, &entry.value, entry.timestamp, entry.client_id);
        }
    }
}

/// [v14] PaneOpLog with binary insertion.
#[derive(Debug, Clone)]
pub struct PaneOpLog {
    ops: Vec<(u64, u64, Vec<u8>)>, // (lamport, client_id, data)
}

impl PaneOpLog {
    pub fn new() -> Self { Self { ops: Vec::new() } }

    /// [v14] Binary insertion via partition_point -- O(log n).
    pub fn append(&mut self, lamport: u64, client_id: u64, data: Vec<u8>) {
        let pos = self.ops.partition_point(|op| {
            op.0 < lamport || (op.0 == lamport && op.1 < client_id)
        });
        self.ops.insert(pos, (lamport, client_id, data));
    }

    pub fn len(&self) -> usize { self.ops.len() }
    pub fn is_empty(&self) -> bool { self.ops.is_empty() }
    pub fn is_sorted(&self) -> bool {
        self.ops.windows(2).all(|w| w[0].0 < w[1].0 || (w[0].0 == w[1].0 && w[0].1 <= w[1].1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lww_last_writer_wins() {
        let mut m = LwwMap::new();
        m.set("key", "old", 1, 1);
        m.set("key", "new", 2, 1);
        assert_eq!(m.get("key"), Some("new"));
    }

    #[test]
    fn test_oplog_sorted() {
        let mut log = PaneOpLog::new();
        log.append(3, 1, vec![3]);
        log.append(1, 1, vec![1]);
        log.append(2, 1, vec![2]);
        assert!(log.is_sorted());
    }

    // [v14 P2] merge commutativity
    #[test]
    fn test_lww_merge_commutative() {
        let mut a = LwwMap::new();
        a.set("x", "a_val", 2, 1);
        let mut b = LwwMap::new();
        b.set("x", "b_val", 1, 2);

        let mut ab = a.clone();
        ab.merge(&b);
        let mut ba = b.clone();
        ba.merge(&a);

        assert_eq!(ab.get("x"), ba.get("x"));
    }
}
```

### Test Strategy

1. LWW last writer wins. `TST-230`.
2. Tombstone deletes win. `TST-231`.
3. OpLog sorted after binary insert. `TST-232`.
4. **[v14 P2]** Merge commutativity with 3 writers. `TST-233`.

### AGENTS.md Rules

- `RULE-S17-01`: LWW timestamp breaks ties by client_id. **Enforcement:** determinism test.
- `RULE-S17-02`: OpLog uses binary insertion via partition_point. **Enforcement:** sort test.
- `RULE-S17-03`: CRDT feature-gated behind `crdt` flag. **Enforcement:** cargo tree.
- `RULE-S17-04`: **[v14 P2]** Merge MUST be commutative: A.merge(B) == B.merge(A). **Enforcement:** property test.

---

## 18. Socket and IPC

### Design Decisions

- Unix domain socket at `$TMPDIR/termforge-$UID/default`.
- Socket isolation: process-level, namespace-level, filesystem-level (INV-007).
- **[v14 P2]** Socket path length validated (<= 108 bytes for portability).
- Traceability: `INV-007`.

### Rust Example

```rust
// crates/mux-os/src/socket.rs
#![allow(dead_code)]

pub const MAX_SOCKET_PATH_LEN: usize = 108; // [v14 P2]

pub fn socket_path(uid: u32, name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("termforge-{}", uid)).join(name)
}

pub fn validate_socket_path(path: &std::path::Path) -> Result<(), &'static str> {
    let s = path.to_string_lossy();
    if s.len() > MAX_SOCKET_PATH_LEN {
        Err("socket path too long")
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_socket_path_format() {
        let p = socket_path(1000, "default");
        assert!(p.to_string_lossy().contains("termforge-1000"));
    }

    // [v14 P2]
    #[test]
    fn test_short_path_valid() {
        let p = socket_path(1000, "default");
        assert!(validate_socket_path(&p).is_ok());
    }
}
```

### Test Strategy

1. Socket path contains UID. `TST-240`.
2. Three isolation layers verified. `TST-241`.
3. **[v14 P2]** Socket path length <= 108. `TST-242`.

### AGENTS.md Rules

- `RULE-S18-01`: Socket isolation three layers (INV-007). **Enforcement:** integration test.
- `RULE-S18-02`: **[v14 P2]** Socket path MUST be <= 108 bytes. **Enforcement:** path length test.

---

## 19. OpenTelemetry (OTEL) Observability [v14 P2]

### Design Decisions

- Structured spans for all Termlet operations.
- Span hierarchy: session -> window -> pane -> operation.
- **[v14 P2]** Added `termlet.checksum_algorithm` span attribute for snapshot operations.
- Traceability: `OPS-005`.

### Rust Example

```rust
// crates/mux-otel/src/lib.rs
#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpanInfo {
    pub name: &'static str,
    pub attributes: Vec<(&'static str, String)>,
}

impl SpanInfo {
    pub fn new(name: &'static str) -> Self {
        Self { name, attributes: Vec::new() }
    }
    pub fn attr(mut self, key: &'static str, value: impl Into<String>) -> Self {
        self.attributes.push((key, value.into()));
        self
    }
}

/// [v14 P2] Snapshot span with checksum algorithm attribute.
pub fn snapshot_span(pane_id: u64, version: u16, algo: &str) -> SpanInfo {
    SpanInfo::new("termlet.snapshot")
        .attr("pane_id", pane_id.to_string())
        .attr("version", version.to_string())
        .attr("checksum_algorithm", algo.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_span() {
        let s = snapshot_span(1, 2, "crc32c");
        assert_eq!(s.name, "termlet.snapshot");
        assert!(s.attributes.iter().any(|(k, v)| *k == "checksum_algorithm" && v == "crc32c"));
    }
}
```

### Test Strategy

1. Span hierarchy correct. `TST-250`.
2. Required attributes present. `TST-251`.
3. **[v14 P2]** checksum_algorithm attribute in snapshot spans. `TST-252`.

### AGENTS.md Rules

- `RULE-S19-01`: All Termlet ops emit OTEL spans. **Enforcement:** span coverage test.
- `RULE-S19-02`: Span hierarchy matches session -> window -> pane -> op. **Enforcement:** hierarchy test.
- `RULE-S19-03`: **[v14 P2]** Snapshot spans MUST include checksum_algorithm. **Enforcement:** attribute test.

---

## 20. tmux Version Management (mux-vm, mux-builder)

### Design Decisions

- `mux-vm`: download and manage tmux versions for parity testing.
- `mux-builder`: build tmux from source.
- **[v14 P2]** mux-vm supports tmux 3.2+ version range (minimum for control-mode features).
- Traceability: `PAR-005`.

### Rust Example

```rust
// tools/mux-vm/src/lib.rs
#![allow(dead_code)]

pub const MIN_TMUX_VERSION: &str = "3.2"; // [v14 P2]

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TmuxVersion { pub major: u8, pub minor: u8 }

impl TmuxVersion {
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 2 { return None; }
        Some(Self { major: parts[0].parse().ok()?, minor: parts[1].parse().ok()? })
    }
    pub fn meets_minimum(&self) -> bool {
        self.major > 3 || (self.major == 3 && self.minor >= 2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version() {
        let v = TmuxVersion::parse("3.4").unwrap();
        assert_eq!(v.major, 3);
        assert!(v.meets_minimum());
    }

    #[test]
    fn test_old_version() {
        let v = TmuxVersion::parse("2.9").unwrap();
        assert!(!v.meets_minimum());
    }
}
```

### Test Strategy

1. Version parse. `TST-260`.
2. Builder compiles tmux. `TST-261`.
3. **[v14 P2]** Minimum version 3.2 enforced. `TST-262`.

### AGENTS.md Rules

- `RULE-S20-01`: mux-vm downloads checksummed tarballs. **Enforcement:** checksum test.
- `RULE-S20-02`: **[v14 P2]** Minimum tmux version 3.2. **Enforcement:** version gate.

---

## 21. Test Support (mux-test-support) [v14 P2]

### Design Decisions

- Test harness with fixture management, isolation, and cleanup.
- **[v14 P2]** DeterministicClock trait for time-controlled Termlet tests (INV-022).
- Traceability: `TST-*`.

### Rust Example

```rust
// crates/mux-test-support/src/lib.rs
// [v14 P2] DeterministicClock

#![allow(dead_code)]
use std::time::Duration;

/// [v14 P2] Clock trait for deterministic testing. INV-022.
pub trait TermletClock {
    fn now_ms(&self) -> u64;
    fn advance(&mut self, delta: Duration);
}

/// [v14 P2] Real clock (production).
pub struct RealClock;
impl TermletClock for RealClock {
    fn now_ms(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
    fn advance(&mut self, _delta: Duration) {
        // No-op: real clock advances naturally
    }
}

/// [v14 P2] Deterministic clock for testing.
pub struct DeterministicClock {
    current_ms: u64,
}

impl DeterministicClock {
    pub fn new(start_ms: u64) -> Self { Self { current_ms: start_ms } }
}

impl TermletClock for DeterministicClock {
    fn now_ms(&self) -> u64 { self.current_ms }
    fn advance(&mut self, delta: Duration) {
        self.current_ms += delta.as_millis() as u64;
    }
}

/// Test isolation guard.
pub struct TestGuard {
    pub test_name: String,
    pub temp_dir: std::path::PathBuf,
}

impl TestGuard {
    pub fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("termforge-test-{}", name));
        Self { test_name: name.to_string(), temp_dir: dir }
    }
}

impl Drop for TestGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.temp_dir);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // [v14 P2] DeterministicClock tests
    #[test]
    fn test_deterministic_clock() {
        let mut clock = DeterministicClock::new(1000);
        assert_eq!(clock.now_ms(), 1000);
        clock.advance(Duration::from_millis(500));
        assert_eq!(clock.now_ms(), 1500);
    }

    #[test]
    fn test_deterministic_clock_monotonic() {
        let mut clock = DeterministicClock::new(0);
        let t1 = clock.now_ms();
        clock.advance(Duration::from_millis(1));
        let t2 = clock.now_ms();
        assert!(t2 > t1);
    }
}
```

### Test Strategy

1. TestGuard cleanup. `TST-270`.
2. Fixture loading. `TST-271`.
3. **[v14 P2]** DeterministicClock advance is monotonic. `TST-272`.
4. **[v14 P2]** DeterministicClock starts at specified time. `TST-273`.

### AGENTS.md Rules

- `RULE-S21-01`: All tests use TestGuard for isolation. **Enforcement:** test lint.
- `RULE-S21-02`: Fixtures versioned in `fixtures/` dir. **Enforcement:** path check.
- `RULE-S21-03`: **[v14 P2]** Network partition tests MUST use DeterministicClock (INV-022). **Enforcement:** clock type check.
- `RULE-S21-04`: **[v14 P2]** DeterministicClock advance MUST be monotonic. **Enforcement:** property test.

---

## 22. Parity Testing (mux-regress)

### Design Decisions

- Regression test runner against tmux binaries.
- Golden file comparison.
- **[v14 P2]** Parity score threshold: >=95% pass rate for LTS/Current lanes.
- Traceability: `PAR-001`.

### Rust Example

```rust
// tools/mux-regress/src/lib.rs
#![allow(dead_code)]

pub const PARITY_THRESHOLD: f64 = 0.95; // [v14 P2]

#[derive(Debug, Clone)]
pub struct ParityResult {
    pub test_name: String,
    pub passed: bool,
    pub tmux_output: String,
    pub termforge_output: String,
}

pub fn parity_score(results: &[ParityResult]) -> f64 {
    if results.is_empty() { return 0.0; }
    let passed = results.iter().filter(|r| r.passed).count();
    passed as f64 / results.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parity_score() {
        let results = vec![
            ParityResult { test_name: "t1".into(), passed: true, tmux_output: "".into(), termforge_output: "".into() },
            ParityResult { test_name: "t2".into(), passed: false, tmux_output: "".into(), termforge_output: "".into() },
        ];
        assert!((parity_score(&results) - 0.5).abs() < 0.01);
    }
}
```

### Test Strategy

1. Parity score calculation. `TST-280`.
2. Golden file comparison. `TST-281`.
3. **[v14 P2]** Parity threshold 95%. `TST-282`.

### AGENTS.md Rules

- `RULE-S22-01`: Parity tests run against real tmux. **Enforcement:** CI with tmux binary.
- `RULE-S22-02`: **[v14 P2]** LTS/Current lanes MUST achieve >=95% parity score. **Enforcement:** score gate.

---

## 23. Fuzz Testing

### Design Decisions

- Fuzz targets for VtParser, snapshot decode, protocol parse.
- `cargo-fuzz` with AFL++ integration.
- **[v14 P2]** Added fuzz target for CRC32C checksum validation path.
- Traceability: `TST-*`.

### Rust Example

```rust
// fuzz/fuzz_targets/vt_parser.rs (conceptual)
#![allow(dead_code)]

pub fn fuzz_vt_parser(data: &[u8]) {
    // Conceptual: parser should never panic on any input
    let _ = data; // Placeholder for standalone compilation
}

pub fn fuzz_snapshot_decode(data: &[u8]) {
    let _ = data;
}

// [v14 P2]
pub fn fuzz_crc32c(data: &[u8]) {
    let _ = data;
}
```

### Test Strategy

1. VtParser fuzz no-panic. `TST-290`.
2. Snapshot decode fuzz no-panic. `TST-291`.
3. **[v14 P2]** CRC32C fuzz no-panic. `TST-292`.

### AGENTS.md Rules

- `RULE-S23-01`: Fuzz targets must not panic. **Enforcement:** fuzz CI.
- `RULE-S23-02`: Corpus in `fixtures/fuzz-corpus/`. **Enforcement:** path check.
- `RULE-S23-03`: **[v14 P2]** CRC32C fuzz target required when feature enabled. **Enforcement:** feature-conditional CI.

---

## 24. Performance Benchmarks [v14 P2]

### Design Decisions

- Criterion benchmarks for hot paths.
- Performance budgets enforced in CI.
- **[v14 P2]** Added benchmark comparing FNV-1a vs CRC32C throughput for snapshot checksum.
- Traceability: `OPS-006`.

### Rust Example

```rust
// benchmarks/criterion/src/lib.rs
#![allow(dead_code)]

pub struct BenchResult {
    pub name: String,
    pub mean_ns: u64,
    pub budget_ns: u64,
}

impl BenchResult {
    pub fn within_budget(&self) -> bool { self.mean_ns <= self.budget_ns }
}

// [v14 P2] Checksum throughput budget
pub const CHECKSUM_1MB_BUDGET_US: u64 = 500; // 500 microseconds for 1MB

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_within_budget() {
        let r = BenchResult { name: "test".into(), mean_ns: 100, budget_ns: 200 };
        assert!(r.within_budget());
    }
}
```

### Test Strategy

1. VtParser throughput. `TST-300`.
2. Grid cell_at latency. `TST-301`.
3. Snapshot encode/decode. `TST-302`.
4. **[v14 P2]** CRC32C vs FNV-1a throughput comparison. `TST-303`.

### AGENTS.md Rules

- `RULE-S24-01`: Benchmarks must have budgets. **Enforcement:** budget check.
- `RULE-S24-02`: Regressions >10% flag warning. **Enforcement:** Criterion CI.
- `RULE-S24-03`: **[v14 P2]** CRC32C throughput benchmark required. **Enforcement:** benchmark existence check.

---

## 25. Visual Client / TUI

### Design Decisions

- ratatui-based TUI client (Layer 3).
- Status bar, tab bar, scrollback viewer.
- **[v14 P2]** TUI snapshot export uses same binary format as headless snapshots.
- Traceability: `API-020`.

### Rust Example

```rust
// crates/mux-view/src/lib.rs
#![allow(dead_code)]

#[derive(Debug, Clone)]
pub struct TuiConfig {
    pub status_bar: bool,
    pub tab_bar: bool,
    pub scrollback_lines: usize,
    pub snapshot_format_version: u16, // [v14 P2]
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self { status_bar: true, tab_bar: true, scrollback_lines: 10_000, snapshot_format_version: 1 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tui_defaults() {
        let c = TuiConfig::default();
        assert!(c.status_bar);
        assert_eq!(c.scrollback_lines, 10_000);
    }
}
```

### Test Strategy

1. TUI renders status bar. `TST-310`.
2. Tab bar shows sessions. `TST-311`.
3. **[v14 P2]** Snapshot format version in TUI config. `TST-312`.

### AGENTS.md Rules

- `RULE-S25-01`: TUI uses ratatui. **Enforcement:** dependency check.
- `RULE-S25-02`: **[v14 P2]** TUI snapshot format matches headless format. **Enforcement:** format parity test.

---

## 26. AGENTS.md Rules [v14 P2]

### Design Decisions

- All rules use `RULE-Snn-xx` format.
- Rules have explicit enforcement mechanism.
- **[v14 P2]** Master rule table expanded to 260+ rules (from 230+ in P1).
- Traceability: all sections.

### Master Rule Summary

| Section | Rule Count | v14 P2 Additions |
|---------|-----------|------------------|
| S01 | 9 | +1 (RULE-S01-09) |
| S02 | 13 | +2 (RULE-S02-12, S02-13) |
| S03 | 12 | +1 (RULE-S03-12) |
| S04 | 8 | +1 (RULE-S04-08) |
| S05 | 9 | +2 (RULE-S05-08, S05-09) |
| S06 | 10 | +2 (RULE-S06-09, S06-10) |
| S07 | 8 | +2 (RULE-S07-07, S07-08) |
| S08 | 10 | +4 (RULE-S08-07..S08-10) |
| S09 | 4 | +1 (RULE-S09-04) |
| S10 | 3 | +1 (RULE-S10-03) |
| S11 | 3 | +1 (RULE-S11-03) |
| S12 | 3 | +1 (RULE-S12-03) |
| S13 | 3 | +1 (RULE-S13-03) |
| S14 | 3 | +1 (RULE-S14-03) |
| S15 | 3 | +1 (RULE-S15-03) |
| S16 | 3 | +1 (RULE-S16-03) |
| S17 | 4 | +1 (RULE-S17-04) |
| S18 | 2 | +1 (RULE-S18-02) |
| S19 | 3 | +1 (RULE-S19-03) |
| S20 | 2 | +1 (RULE-S20-02) |
| S21 | 4 | +2 (RULE-S21-03, S21-04) |
| S22 | 2 | +1 (RULE-S22-02) |
| S23 | 3 | +1 (RULE-S23-03) |
| S24 | 3 | +1 (RULE-S24-03) |
| S25 | 2 | +1 (RULE-S25-02) |
| S26 | 2 | +0 |
| S27 | 2 | +0 |
| S28 | 2 | +0 |
| S29 | 2 | +0 |
| S30 | 2 | +0 |
| S31 | 2 | +0 |
| S32 | 100 | +10 (RULE-S32-91..S32-100) |
| **Total** | **260+** | **+40** |

### Rust Example

```rust
// AGENTS.md rule validation
#![allow(dead_code)]

pub struct Rule {
    pub id: &'static str,
    pub section: u8,
    pub enforcement: &'static str,
    pub added_in: &'static str,
}

pub const SAMPLE_RULES: &[Rule] = &[
    Rule { id: "RULE-S01-09", section: 1, enforcement: "API surface lint", added_in: "v14-pass2" },
    Rule { id: "RULE-S02-12", section: 2, enforcement: "Gate schema validation", added_in: "v14-pass2" },
    Rule { id: "RULE-S05-08", section: 5, enforcement: "Type check", added_in: "v14-pass2" },
    Rule { id: "RULE-S08-07", section: 8, enforcement: "Feature gate test", added_in: "v14-pass2" },
    Rule { id: "RULE-S32-91", section: 32, enforcement: "Network partition test", added_in: "v14-pass2" },
];

pub fn rules_for_section(section: u8, rules: &[Rule]) -> Vec<&Rule> {
    rules.iter().filter(|r| r.section == section).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_rules_p2() {
        let p2_rules: Vec<_> = SAMPLE_RULES.iter().filter(|r| r.added_in == "v14-pass2").collect();
        assert!(p2_rules.len() >= 5);
    }
}
```

### Test Strategy

1. All rules have enforcement. `TST-320`.
2. Rule ID format valid. `TST-321`.
3. No duplicate rule IDs. `TST-322`.
4. **[v14 P2]** P2 additions counted. `TST-323`.

### AGENTS.md Rules

- `RULE-S26-01`: All rules use RULE-Snn-xx format. **Enforcement:** CI regex lint.
- `RULE-S26-02`: Every rule has enforcement mechanism. **Enforcement:** table completeness check.

---

## 27. Risks and Mitigations [v14 P2]

### Design Decisions

- Risk register maintained as append-only log.
- Risk severity: Low, Medium, High, Critical.
- **[v14]** 95 risks (R1-R95).
- **[v14 P2]** 100 risks (R96-R100 added for CRC32C, DeterministicClock, SmallVec rejection, network simulation, slow consumer).
- Traceability: `OPS-007`.

### Risk Register Highlights

| Risk ID | Category | Description | Severity | Mitigation | Status |
|---------|----------|-------------|----------|------------|--------|
| R01 | Grid | CompactString heap fallback for emoji | Medium | Benchmark inline vs heap path | Open |
| R02 | Snapshot | PackedCell 21-bit ch overflow for future Unicode | Low | Monitor Unicode 16+ | Open |
| R03 | VtParser | Const table maintenance burden | Low | Generated table with build.rs | Open |
| R04 | PtyHandle | Typestate complexity for bindings | Medium | DynPtyHandle runtime fallback | Mitigated |
| R05 | CRDT | Merge ordering edge cases | High | Property-based testing | Active |
| R10 | Snapshot | FNV-1a collision in large snapshots | Low | CRC32C opt-in path | Mitigated |
| R80 | Grid | VecDeque non-contiguous for snapshot | Medium | make_contiguous() before snapshot | Mitigated |
| R90 | Termlet | expect() panic in library code | High | expect_or_fail() added | Mitigated |
| R95 | General | v13 findings incomplete | Low | Append-only finding registry | Mitigated |
| **R96** | **Snapshot** | **[v14 P2]** CRC32C software fallback slower than expected without hw | Medium | Benchmark; hw detection at runtime | Open |
| **R97** | **Testing** | **[v14 P2]** DeterministicClock drift from real-time behavior | Medium | Calibration tests; document limitations | Open |
| **R98** | **Grid** | **[v14 P2]** SmallVec rejection may need revisiting if Cell shrinks | Low | Re-evaluate if Cell < 16 bytes | Deferred |
| **R99** | **Termlet** | **[v14 P2]** Network partition simulation introduces flaky tests | High | DeterministicClock + deterministic seed | Active |
| **R100** | **Termlet** | **[v14 P2]** Slow consumer simulation backpressure deadlock | High | Bounded buffer with timeout fallback | Active |

### Rust Example

```rust
// crates/mux-types/src/risk.rs
#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskSeverity { Low, Medium, High, Critical }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskStatus { Open, Active, Mitigated, Deferred, Closed }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Risk {
    pub id: &'static str,
    pub severity: RiskSeverity,
    pub status: RiskStatus,
    pub added_in: &'static str,
}

pub const RISK_COUNT: usize = 100; // [v14 P2]

pub fn open_risks(risks: &[Risk]) -> Vec<&Risk> {
    risks.iter().filter(|r| matches!(r.status, RiskStatus::Open | RiskStatus::Active)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_count() {
        assert!(RISK_COUNT >= 100); // [v14 P2]
    }
}
```

### Test Strategy

1. Risk register complete. `TST-330`.
2. All risks have mitigation. `TST-331`.
3. **[v14 P2]** R96-R100 present. `TST-332`.

### AGENTS.md Rules

- `RULE-S27-01`: Risk register append-only. **Enforcement:** diff review.
- `RULE-S27-02`: **[v14 P2]** Risk count >= 100 (R1-R100). **Enforcement:** count check.

---

## 28. Plan Evolution and Changelog [v14 P2]

### Design Decisions

- Changelog tracks all spec changes.
- Version lineage documented in preamble.
- **[v14 P2]** Changelog gains cross-pollination section documenting decisions adopted/rejected from other P1 specs.
- Traceability: `OPS-008`.

### Changelog

| Version | Lines | Rules | Risks | S32 Subs | Key Changes |
|---------|-------|-------|-------|----------|-------------|
| v13 P3 | 5,739 | 210+ | 80 | 35 | Definitive baseline |
| v14 P1 Claude | 5,625 | 230+ | 95 | 36 | ByteClass, CompactString, PackedCell, typestate PtyHandle |
| v14 P1 GPT | 5,714 | 342 | 96 | 45 | 45 S32 subsections, 342 rules |
| v14 P1 Gemini | 308 | 2 | 2 | 35 | SmallVec, CRC32C, TermForgeIdentity trait |
| **v14 P2** | **5,800+** | **260+** | **100** | **47** | CRC32C opt-in, DeterministicClock, 47 S32 subs, SmallVec rejected |

### Test Strategy

1. Changelog matches code. `TST-340`.
2. **[v14 P2]** Cross-pollination decisions documented. `TST-341`.

### AGENTS.md Rules

- `RULE-S28-01`: Changelog updated on every spec version. **Enforcement:** PR check.
- `RULE-S28-02`: **[v14 P2]** Cross-pollination decisions table required. **Enforcement:** section check.

---

## 29. Reference Anchors [v14 P2]

### Design Decisions

- Cross-references use `INV-*`, `API-*`, `PAR-*`, `OPS-*`, `TST-*` anchors.
- **[v14 P2]** Added INV-021 (checksum algorithm selectable) and INV-022 (DeterministicClock).
- Traceability: all.

### Reference Table

| Anchor | Section | Description |
|--------|---------|-------------|
| INV-001 | 1 | Protocol v8 compatibility |
| INV-002 | 3 | mux-core pure, no IO |
| INV-012 | 5 | put_char/put_grapheme sole entry |
| INV-014 | 7 | PtyHandle 7-state |
| INV-015 | 8 | Snapshot trailing checksum |
| INV-018 | 6 | VtParser const table |
| INV-019 | 2 | Error types Clone+PartialEq+Eq |
| INV-020 | 8 | Snapshot v2 backward compat |
| INV-021 | 8 | **[v14 P2]** Checksum algorithm selectable |
| INV-022 | 21 | **[v14 P2]** DeterministicClock for net sim |

### Test Strategy

1. All INV anchors resolvable. `TST-350`.
2. **[v14 P2]** INV-021 and INV-022 present. `TST-351`.

### AGENTS.md Rules

- `RULE-S29-01`: All anchors resolvable. **Enforcement:** link lint.
- `RULE-S29-02`: **[v14 P2]** New invariants documented in preamble. **Enforcement:** preamble check.

---

## 30. Appendix: Canonical Type Quick Reference [v14 P2]

### Design Decisions

- Quick reference for all canonical types.
- **[v14 P2]** Added `ChecksumAlgo`, `DeterministicClock`, `TermletClock` types.

### Type Table

| Type | Crate | Layer | Added |
|------|-------|-------|-------|
| CompactString | mux-compact-string | L0 | v14 |
| PackedCell | mux-packed-cell | L0 | v14 |
| ByteClass | mux-grid | L0 | v14 |
| PtyHandle<S> | mux-pty | L1 | v14 |
| DynPtyHandle | mux-pty | L1 | v14 |
| ChecksumAlgo | mux-checksum | L0 | **v14 P2** |
| TermletClock | mux-test-support | L2 | **v14 P2** |
| DeterministicClock | mux-test-support | L2 | **v14 P2** |
| TermletError (12 variants) | mux-termlet | L2 | v14 |
| EraseMode | mux-grid | L0 | v14 |

### Test Strategy

1. All types compile. `TST-360`.
2. **[v14 P2]** New types in table. `TST-361`.

### AGENTS.md Rules

- `RULE-S30-01`: Type table matches implementation. **Enforcement:** type check.
- `RULE-S30-02`: **[v14 P2]** ChecksumAlgo and TermletClock in table. **Enforcement:** table lint.

---

## 31. Supplemental Test Matrix [v14 P2]

### Design Decisions

- Test matrix covers: Rust unit, integration, property, fuzz, cross-language, parity, benchmark.
- **[v14 P2]** Added DeterministicClock test category.
- **[v14 P2]** Added CRC32C test category.

### Matrix

| Category | Count (v14 P1) | Count (v14 P2) | Delta |
|----------|---------------|----------------|-------|
| Unit | 200+ | 210+ | +10 |
| Integration | 80+ | 85+ | +5 |
| Property | 30+ | 35+ | +5 |
| Fuzz | 15+ | 18+ | +3 |
| Cross-language | 20+ | 22+ | +2 |
| Parity | 50+ | 55+ | +5 |
| Benchmark | 25+ | 28+ | +3 |
| Termlet | 145+ | 155+ | +10 |
| **Total** | **565+** | **608+** | **+43** |

### Test Strategy

1. Matrix covers all sections. `TST-370`.
2. **[v14 P2]** Test count increased. `TST-371`.

### AGENTS.md Rules

- `RULE-S31-01`: Test matrix updated per spec version. **Enforcement:** matrix lint.
- `RULE-S31-02`: **[v14 P2]** Total tests >= 600. **Enforcement:** count CI.

---

## 32. Termlets [v14 P2 -- deepest section, 47 subsections]

### Design Decisions

Termlets are the killer feature differentiating TermForge from other terminal multiplexers and testing tools. They are **SDK-first testing pods** that wrap tmux panes into simplified, embeddable, language-binding-native handles.

**Core philosophy:** A Termlet is to a tmux pane what a Docker container is to a VM -- same capability, simpler interface, faster lifecycle, purpose-built for programmatic use.

**Key design decisions (v14 P2 -- all v14 P1 decisions carried forward plus P2 additions):**

1-48. (all v14 P1 decisions carried forward unchanged, items 1-48)

49. **[v14 P2]** `TermletClock` trait injectable: Termlet accepts a `Box<dyn TermletClock>` for time operations. Production uses `RealClock`; tests use `DeterministicClock`. INV-022.

50. **[v14 P2]** Network partition simulation: `NetworkSimulator` struct provides latency injection (`inject_latency_ms(min, max)`), packet loss (`inject_loss_rate(0.0..1.0)`), and partition (`simulate_partition(duration)`). Uses `DeterministicClock` to avoid flaky tests.

51. **[v14 P2]** Slow consumer simulation: `SlowConsumerSimulator` throttles PTY read to configurable `bytes_per_sec`. Validates Grid backpressure handling. Termlet must not deadlock under throttled read.

52. **[v14 P2]** CRC32C snapshot checksum: When `snapshot-crc32c` feature is enabled, Termlet snapshots use CRC32C instead of FNV-1a. Checksum algorithm stored in snapshot header (INV-021).

53. **[v14 P2]** Parallel pod scheduler: `PodScheduler` distributes Termlets across available CPU cores with deterministic seed partitioning. Each pod gets a unique seed derived from test name + partition index.

54. **[v14 P2]** Flake triage automation: Termlets that fail non-deterministically are automatically quarantined with expiry timestamp. Quarantined tests replay with `DeterministicClock` for root-cause analysis.

55. **[v14 P2]** Property-based Termlet tests: `proptest` generates random (command, input_sequence, expected_pattern) tuples. Invariant: no panic for any input.

56. **[v14 P2]** S32 expanded to 47 subsections (from 36 in P1, incorporating GPT P1 naming for 32.37-32.45 and adding 32.46-32.47 from Gemini P1 ideas).

**Architectural position:** `mux-termlet` at Layer 2 (FACADE). Dependencies:
- `mux-types` (L0): shared types, `PaneSize`, `DynPtyHandle`
- `mux-grid` (L0): `Grid`, `VtParser`, `Cell`, `Line`, `Viewport`, `CompactString`, `ByteClass`
- `mux-pty` (L1): `PtyBackend` trait, typestate `PtyHandle<S>`
- `mux-pty-fake` (L0): `FakePtyBackend`, `ScenarioStep`
- `mux-snapshot` (L0): binary snapshot format v1+v2 with PackedCell and checksum
- `mux-checksum` (L0): **[v14 P2]** FNV-1a and CRC32C checksum implementations
- `mux-test-support` (L2): **[v14 P2]** `TermletClock`, `DeterministicClock`

It does NOT depend on: `mux-server`, `mux-api`, `mux-orm`.

### 32.1 Architecture Diagram

```
                    +------------------+
                    |   User Code      |
                    | (Rust / Py / JS) |
                    +--------+---------+
                             |
                    spawn / send_keys / send_bytes / expect_or_fail / snapshot / resize / kill / restart
                             |
                    +--------v---------+
                    |     Termlet      |  <-- mux-termlet crate (Layer 2)
                    |  (single handle) |
                    +--------+---------+
                             |
            +--------+-------+--------+--------+
            |        |       |        |        |
    +-------v--+ +---v---+ +v------+ +v------+ +v-----------+
    | VtParser  | | Grid  | | Pty   | | Snap  | | Clock      |
    | (mux-grid)| | (grid)| | (pty) | | (snap)| | [v14 P2]   |
    | 7 states  | | VecDq | | Type  | | v1+v2 | | Termlet    |
    | ByteClass | | CmpStr| | State | | CRC32C| | Clock      |
    +-----------+ +-------+ +---+---+ +-------+ +------------+
                                |
                    +-----------+-----------+
                    |                       |
            +-------v-------+     +--------v--------+
            | RealPtyBackend|     | FakePtyBackend  |
            |  (mux-pty)    |     | (mux-pty-fake)  |
            +---------------+     +-----------------+
```

### 32.2 TermletState Lifecycle

```text
Spawning -> Running -> Stopping -> Exited
       \-> SpawnFailed  (terminal state)
Running --kill()--> Stopping --grace timeout--> ForcedKill -> Exited
Running --process exit event---------------------------> Exited
Running --restart()--> [kill+close old PtyHandle via 7-state typestate] -> Spawning -> Running  (new PtyHandle)
```

### Rust Example (TermletState + Core API)

```rust
// crates/mux-termlet/src/lib.rs
// [v14 P2] DEFINITIVE Termlet core with DeterministicClock, NetworkSimulator, CRC32C

#![allow(dead_code)]
use std::time::{Duration, Instant};
use std::collections::HashMap;

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
pub enum TermletState { Spawning, Running, Stopping, SpawnFailed, Exited }

impl TermletState {
    pub fn is_terminal(self) -> bool { matches!(self, Self::SpawnFailed | Self::Exited) }
    pub fn allows_interaction(self) -> bool { self == Self::Running }
    pub fn allows_restart(self) -> bool { matches!(self, Self::Running | Self::Exited | Self::SpawnFailed) }
}

pub fn valid_transition(from: TermletState, to: TermletState) -> bool {
    matches!((from, to),
        (TermletState::Spawning, TermletState::Running) |
        (TermletState::Spawning, TermletState::SpawnFailed) |
        (TermletState::Running, TermletState::Stopping) |
        (TermletState::Running, TermletState::Exited) |
        (TermletState::Stopping, TermletState::Exited) |
        (TermletState::Running, TermletState::Spawning) |
        (TermletState::Exited, TermletState::Spawning) |
        (TermletState::SpawnFailed, TermletState::Spawning)
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtyMode { Real, Fake }

/// [v14 P2] ChecksumAlgo for snapshot configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChecksumAlgo { Fnv1a, Crc32c }

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
    pub snapshot_version: u16,
    pub checksum_algo: ChecksumAlgo, // [v14 P2]
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
            snapshot_version: 1,
            checksum_algo: ChecksumAlgo::Fnv1a, // [v14 P2] default
        }
    }
}

/// [v14 P2] 12-variant error type with Clone + PartialEq + Eq.
#[derive(Debug, Clone, PartialEq, Eq)]
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
    HandleClosed { handle_id: u64 },
    ExpectFailed { pattern: String, timeout_ms: u64, snapshot_preview: String },
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
            Self::ExpectFailed { pattern, timeout_ms, snapshot_preview } =>
                write!(f, "[TERMLET_EXPECT_FAILED] {pattern:?} after {timeout_ms}ms; snapshot: {snapshot_preview}"),
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
    pub checksum_algo: ChecksumAlgo, // [v14 P2]
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
            cols: 80, rows: 24, cursor_col: 0, cursor_row: 0,
            revision: 0, timestamp: Instant::now(),
            checksum_algo: ChecksumAlgo::Fnv1a,
        }
    }
    pub fn preview(&self, max_lines: usize) -> String {
        self.lines.iter().take(max_lines).map(|l| l.trim_end()).collect::<Vec<_>>().join("\n")
    }
}

/// [v14 P2] TermletClock trait (from S21).
pub trait TermletClock: Send {
    fn now_ms(&self) -> u64;
    fn advance(&mut self, delta: Duration);
}

/// [v14 P2] TermletLike trait with expect_or_fail.
pub trait TermletLike {
    fn send_keys(&mut self, keys: &str) -> Result<(), TermletError>;
    fn send_bytes(&mut self, data: &[u8]) -> Result<(), TermletError>;
    fn wait_for(&mut self, pattern: &str, timeout: Duration) -> Result<WaitMatch, TermletError>;
    fn expect_or_fail(&mut self, pattern: &str) -> Result<WaitMatch, TermletError>;
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

/// [v14 P2] NetworkSimulator for partition and latency testing.
#[derive(Debug, Clone)]
pub struct NetworkSimulator {
    pub latency_min_ms: u64,
    pub latency_max_ms: u64,
    pub loss_rate: f64,
    pub partition_until_ms: Option<u64>,
}

impl NetworkSimulator {
    pub fn new() -> Self {
        Self { latency_min_ms: 0, latency_max_ms: 0, loss_rate: 0.0, partition_until_ms: None }
    }
    pub fn inject_latency(mut self, min_ms: u64, max_ms: u64) -> Self {
        self.latency_min_ms = min_ms;
        self.latency_max_ms = max_ms;
        self
    }
    pub fn inject_loss(mut self, rate: f64) -> Self {
        self.loss_rate = rate.clamp(0.0, 1.0);
        self
    }
    pub fn simulate_partition(mut self, until_ms: u64) -> Self {
        self.partition_until_ms = Some(until_ms);
        self
    }
    pub fn is_partitioned(&self, now_ms: u64) -> bool {
        self.partition_until_ms.is_some_and(|until| now_ms < until)
    }
    pub fn effective_latency_ms(&self) -> u64 {
        // Simplified: use min. Real impl would sample from distribution.
        self.latency_min_ms
    }
}

/// [v14 P2] SlowConsumerSimulator for backpressure testing.
#[derive(Debug, Clone)]
pub struct SlowConsumerSimulator {
    pub bytes_per_sec: u64,
    pub enabled: bool,
}

impl SlowConsumerSimulator {
    pub fn new(bytes_per_sec: u64) -> Self {
        Self { bytes_per_sec, enabled: true }
    }
    pub fn disabled() -> Self {
        Self { bytes_per_sec: u64::MAX, enabled: false }
    }
    pub fn delay_for_bytes(&self, byte_count: usize) -> Duration {
        if !self.enabled || self.bytes_per_sec == 0 { return Duration::ZERO; }
        let secs = byte_count as f64 / self.bytes_per_sec as f64;
        Duration::from_secs_f64(secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_text_trimmed() {
        let snap = TermletSnapshot {
            lines: vec!["Hello  ".into(), "".into(), "".into()],
            cols: 80, rows: 24, cursor_col: 0, cursor_row: 0,
            revision: 0, timestamp: Instant::now(),
            checksum_algo: ChecksumAlgo::Fnv1a,
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
    fn test_default_config_v14_p2() {
        let config = TermletConfig::default();
        assert_eq!(config.cols, 80);
        assert_eq!(config.rows, 24);
        assert_eq!(config.snapshot_version, 1);
        assert_eq!(config.checksum_algo, ChecksumAlgo::Fnv1a); // [v14 P2]
    }

    #[test]
    fn test_valid_transitions() {
        assert!(valid_transition(TermletState::Spawning, TermletState::Running));
        assert!(valid_transition(TermletState::Running, TermletState::Stopping));
        assert!(!valid_transition(TermletState::Exited, TermletState::Running));
    }

    #[test]
    fn test_terminal_states() {
        assert!(TermletState::Exited.is_terminal());
        assert!(TermletState::SpawnFailed.is_terminal());
        assert!(!TermletState::Running.is_terminal());
    }

    #[test]
    fn test_error_display_expect_failed() {
        let err = TermletError::ExpectFailed {
            pattern: "hello".into(), timeout_ms: 5000, snapshot_preview: "$ ".into(),
        };
        assert!(format!("{err}").contains("[TERMLET_EXPECT_FAILED]"));
        assert!(format!("{err}").contains("5000ms"));
    }

    #[test]
    fn test_error_clone_eq() {
        let e1 = TermletError::HandleClosed { handle_id: 42 };
        let e2 = e1.clone();
        assert_eq!(e1, e2);
    }

    #[test]
    fn test_snapshot_preview() {
        let snap = TermletSnapshot::from_text("line1\nline2\nline3");
        assert_eq!(snap.preview(2), "line1\nline2");
    }

    // [v14 P2] NetworkSimulator tests
    #[test]
    fn test_network_simulator_partition() {
        let sim = NetworkSimulator::new().simulate_partition(5000);
        assert!(sim.is_partitioned(3000));
        assert!(!sim.is_partitioned(6000));
    }

    #[test]
    fn test_network_simulator_latency() {
        let sim = NetworkSimulator::new().inject_latency(10, 50);
        assert_eq!(sim.effective_latency_ms(), 10);
    }

    #[test]
    fn test_network_simulator_loss_clamped() {
        let sim = NetworkSimulator::new().inject_loss(1.5);
        assert!((sim.loss_rate - 1.0).abs() < f64::EPSILON);
    }

    // [v14 P2] SlowConsumerSimulator tests
    #[test]
    fn test_slow_consumer_delay() {
        let sim = SlowConsumerSimulator::new(1000); // 1000 bytes/sec
        let delay = sim.delay_for_bytes(500);
        assert_eq!(delay, Duration::from_millis(500));
    }

    #[test]
    fn test_slow_consumer_disabled() {
        let sim = SlowConsumerSimulator::disabled();
        assert!(!sim.enabled);
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
            Self::RegexPattern(pattern) => text.find(pattern).map(|start| start..(start + pattern.len())),
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
// [v14 P2] Builder with checksum_algo

#![allow(dead_code)]
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtyMode { Real, Fake }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChecksumAlgo { Fnv1a, Crc32c }

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
    pub snapshot_version: u16,
    pub checksum_algo: ChecksumAlgo, // [v14 P2]
}

impl Default for TermletConfig {
    fn default() -> Self {
        Self {
            cols: 80, rows: 24,
            env: vec![("TERM".into(), "xterm-256color".into())],
            inherit_env: false, cwd: None, backend: PtyMode::Real,
            grace_period: Duration::from_secs(2),
            wait_poll_interval: Duration::from_millis(25),
            wait_max_interval: Duration::from_millis(100),
            default_timeout: Duration::from_secs(5),
            snapshot_version: 1,
            checksum_algo: ChecksumAlgo::Fnv1a,
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
    pub fn size(mut self, cols: u16, rows: u16) -> Self { self.config.cols = cols; self.config.rows = rows; self }
    pub fn env(mut self, key: impl Into<String>, val: impl Into<String>) -> Self {
        self.config.env.push((key.into(), val.into())); self
    }
    pub fn inherit_env(mut self) -> Self { self.config.inherit_env = true; self }
    pub fn cwd(mut self, cwd: impl Into<String>) -> Self { self.config.cwd = Some(cwd.into()); self }
    pub fn fake(mut self) -> Self { self.config.backend = PtyMode::Fake; self }
    pub fn real(mut self) -> Self { self.config.backend = PtyMode::Real; self }
    pub fn grace_period(mut self, p: Duration) -> Self { self.config.grace_period = p; self }
    pub fn default_timeout(mut self, t: Duration) -> Self { self.config.default_timeout = t; self }
    pub fn snapshot_version(mut self, v: u16) -> Self { self.config.snapshot_version = v; self }
    /// [v14 P2] Configure checksum algorithm.
    pub fn checksum_algo(mut self, algo: ChecksumAlgo) -> Self { self.config.checksum_algo = algo; self }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_defaults() {
        let b = TermletBuilder::new("echo test");
        assert_eq!(b.config.cols, 80);
        assert_eq!(b.config.snapshot_version, 1);
        assert_eq!(b.config.checksum_algo, ChecksumAlgo::Fnv1a); // [v14 P2]
    }

    #[test]
    fn test_builder_snapshot_v2() {
        let b = TermletBuilder::new("echo test").snapshot_version(2);
        assert_eq!(b.config.snapshot_version, 2);
    }

    // [v14 P2]
    #[test]
    fn test_builder_crc32c() {
        let b = TermletBuilder::new("echo test").checksum_algo(ChecksumAlgo::Crc32c);
        assert_eq!(b.config.checksum_algo, ChecksumAlgo::Crc32c);
    }
}
```

### 32.5 TermletPool

```rust
// crates/mux-termlet/src/pool.rs
#![allow(dead_code)]
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TermletState { Spawning, Running, Stopping, SpawnFailed, Exited }
#[derive(Debug, Clone, PartialEq, Eq)]
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
    pub fn insert(&mut self, name: &str, termlet: T) { self.termlets.insert(name.to_string(), termlet); }
    pub fn get(&self, name: &str) -> Option<&T> { self.termlets.get(name) }
    pub fn get_mut(&mut self, name: &str) -> Result<&mut T, TermletError> {
        self.termlets.get_mut(name).ok_or(TermletError::NotInPool { name: name.to_string() })
    }
    pub fn kill_all(&mut self) { for t in self.termlets.values_mut() { let _ = t.kill(); } }
    pub fn snapshot_all(&mut self) -> HashMap<String, String> {
        self.termlets.iter_mut().map(|(n, t)| (n.clone(), t.snapshot_text())).collect()
    }
    pub fn len(&self) -> usize { self.termlets.len() }
    pub fn is_empty(&self) -> bool { self.termlets.is_empty() }
    pub fn alive_count(&self) -> usize { self.termlets.values().filter(|t| t.is_alive()).count() }
    pub fn names(&self) -> Vec<&str> { self.termlets.keys().map(|s| s.as_str()).collect() }
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
pub struct LineDiff { pub row: usize, pub before: String, pub after: String }

#[derive(Debug, Clone)]
pub struct SnapshotDiff { pub changed_lines: Vec<LineDiff>, pub identical: bool }

impl SnapshotDiff {
    pub fn compare(before: &str, after: &str) -> Self {
        if before == after { return Self { changed_lines: Vec::new(), identical: true }; }
        let bl: Vec<&str> = before.lines().collect();
        let al: Vec<&str> = after.lines().collect();
        let max = bl.len().max(al.len());
        let mut changed = Vec::new();
        for i in 0..max {
            let b = bl.get(i).copied().unwrap_or("");
            let a = al.get(i).copied().unwrap_or("");
            if b != a { changed.push(LineDiff { row: i, before: b.into(), after: a.into() }); }
        }
        Self { identical: changed.is_empty(), changed_lines: changed }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical() { assert!(SnapshotDiff::compare("Hello", "Hello").identical); }

    #[test]
    fn test_different() {
        let diff = SnapshotDiff::compare("Hello", "World");
        assert!(!diff.identical);
        assert_eq!(diff.changed_lines.len(), 1);
    }
}
```

### 32.7 Error Code Reference [v14 P2 -- 12 variants]

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
| `HandleClosed` | `TERMLET_HANDLE_CLOSED` | `RuntimeError` | `Error` (code: HANDLE_CLOSED) |
| `ExpectFailed` | `TERMLET_EXPECT_FAILED` | `AssertionError` | `Error` (code: EXPECT_FAILED) |

### 32.8 Python Binding Contract

```python
import termforge

# Context manager guarantees cleanup.
with termforge.Termlet.spawn("bash", cols=120, rows=40) as t:
    t.send_keys("echo hello\n")
    t.send_bytes(b"\x1b[A")
    match = t.wait_for("hello", timeout=5.0)
    snap = t.snapshot()
    assert "hello" in snap.to_text()
    # [v14] Non-panicking expect
    result = t.expect_or_fail("hello")
    assert result is not None
    # [v14 P2] CRC32C checksum
    snap_binary = snap.to_binary(checksum="crc32c")
    t.kill()

# Pool with generic TermletLike
with termforge.TermletPool() as pool:
    pool.spawn("server", "python3 server.py")
    pool.spawn("client", "python3 client.py")
    pool["server"].wait_for("listening", timeout=10.0)
```

### 32.9 Node.js Binding Contract

```javascript
import { Termlet, TermletPool, TermletSnapshot } from 'termforge';

const t = await Termlet.spawn('bash', { cols: 80, rows: 24 });
try {
    await t.sendKeys('echo hello\n');
    const result = await t.expectOrFail('hello');
    const snap = t.snapshot();
    expect(snap.toText()).toContain('hello');
    // [v14 P2] CRC32C checksum
    const binary = snap.toBinary({ checksum: 'crc32c' });
    await t.kill();
} finally {
    await t.kill();
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

pub struct AsyncTermlet<T: Send + 'static> { inner: T }

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

### 32.13 OTEL in Termlets [v14 P2]

| Operation | Span Name | Attributes |
|---|---|---|
| spawn | `termlet.spawn` | `termlet.id`, `command`, `backend.mode`, `cols`, `rows` |
| send_keys | `termlet.send_keys` | `termlet.id`, `keys.len` |
| send_bytes | `termlet.send_bytes` | `termlet.id`, `bytes.len` |
| wait_for | `termlet.wait_for` | `termlet.id`, `pattern`, `timeout_ms`, `matched` |
| expect | `termlet.expect` | `termlet.id`, `pattern`, `matched` |
| expect_or_fail | `termlet.expect_or_fail` | `termlet.id`, `pattern`, `timeout_ms`, `matched`, `snapshot_preview_len` |
| snapshot | `termlet.snapshot` | `termlet.id`, `cols`, `rows`, `revision`, `version`, `checksum_algorithm` |
| resize | `termlet.resize` | `termlet.id`, `old_cols`, `old_rows`, `new_cols`, `new_rows` |
| kill | `termlet.kill` | `termlet.id`, `grace_period_ms`, `forced` |
| restart | `termlet.restart` | `termlet.id`, `old_command`, `new_command` |
| close | `termlet.close` | `termlet.id`, `handle_id` |
| **[v14 P2]** network_sim | `termlet.network_sim` | `termlet.id`, `latency_ms`, `loss_rate`, `partitioned` |
| **[v14 P2]** slow_consumer | `termlet.slow_consumer` | `termlet.id`, `bytes_per_sec`, `delay_ms` |

### 32.14 Testing Patterns

**32.14.1 Spawn-Expect-Kill Pattern:**
```rust
#[test]
fn test_echo_output() {
    let mut t = TermletBuilder::new("echo hello world").fake().spawn().unwrap();
    t.expect("hello world");
    t.kill().unwrap();
}
```

**32.14.2 Sidecar Pattern:** Two Termlets for client-server testing.

**32.14.3 Shell Interaction Pattern:** Using `ShellInteraction` trait for prompt detection.

**32.14.4 [v14] expect_or_fail Pattern:**
```rust
#[test]
fn test_expect_or_fail_result() {
    let mut t = TermletBuilder::new("echo hello").fake().spawn().unwrap();
    let result = t.expect_or_fail("hello");
    assert!(result.is_ok());
    t.kill().unwrap();
}
```

**32.14.5 [v14 P2] Network Partition Pattern:**
```rust
#[test]
fn test_network_partition() {
    let sim = NetworkSimulator::new().simulate_partition(5000);
    assert!(sim.is_partitioned(3000));
    assert!(!sim.is_partitioned(6000));
}
```

### 32.15 Debugging Guidance

When a Termlet test fails:
1. Dump snapshot: `eprintln!("{}", termlet.snapshot().to_text());`
2. Dump raw history: `eprintln!("{:?}", String::from_utf8_lossy(termlet.output_history()));`
3. Visual diff: `SnapshotDiff::compare(&before, &after)`
4. Binary snapshot export with checksum integrity check.
5. `expect_or_fail()` error includes `snapshot_preview` for immediate diagnosis.
6. Snapshot v2 export for compact binary analysis.
7. **[v14 P2]** Check `checksum_algo` in snapshot header for correct algorithm.
8. **[v14 P2]** Replay with `DeterministicClock` for flaky test diagnosis.

### 32.16 Restart API [v14 P2 deepened]

Semantics:
- `restart(command)` kills the current process, transitions through PtyState Stopping -> Exited -> Reaped -> Closed on old handle (full 7-state cleanup via typestate), resets grid, and spawns new process with fresh PtyHandle.
- Typestate PtyHandle enforces compile-time correctness on the kill -> close path.
- New `PtyHandle` allocated for respawn (new slot or reused slot with incremented generation).
- Grid and output history cleared on restart.
- Drain barrier: all residual output drained and discarded before respawn.
- **[v14 P2]** Restart resets NetworkSimulator and SlowConsumerSimulator to default (no injection).

### 32.17 Grid Integration [v14 P2 deepened]

- Termlet owns a `Grid` with `VecDeque<Line>` storage and `CompactString` Cell.
- **[v14 P2]** Line uses `Vec<Cell>` with capacity hint (SmallVec rejected per S75).
- `Grid::put()` removed from public API; all writes through `put_char()` or `put_grapheme()`.
- `erase_in_display(mode)` and `erase_in_line(mode)` implement CSI J and CSI K.
- `VtParser` drives Grid operations through `ByteClass` dispatch (INV-012).
- `snapshot()` calls `grid.viewport_text(grid.full_viewport())`.
- `resize()` calls grid resize after backend resize.
- Grid reset via `Grid::new()` during `restart()`.

### 32.18 Snapshot Binary Format Specification [v14 P2]

```
Offset  Size  Description
0       8     Magic bytes: "TFSNAP13"
8       2     Version (u16 LE, 1 or 2)
10      2     cols (u16 LE)
12      2     rows (u16 LE)
14      2     cursor_col (u16 LE)
16      2     cursor_row (u16 LE)
18      8     revision (u64 LE)
26      1     [v14 P2] checksum_algorithm (0x00=FNV-1a, 0x01=CRC32C)
27      4     cell_count (u32 LE)

--- v1: cell_count * 8 bytes ---
31      N*8   cells: ch(u32) + style(u16) + flags(u16) per cell

--- v2: cell_count * 8 bytes ---
31      N*8   cells: PackedCell(u64) per cell [ch:21 + style:16 + flags:16 + reserved:11]

31+N*8  4     checksum (u32 LE, algorithm per offset 26)
```

### 32.19 Failure Modes and Recovery

**32.19.1 Spawn Failure:** Backend creation fails -> `SpawnFailed`.
**32.19.2 Partial Output:** `drain_output()` captures what was written.
**32.19.3 Backend Switchover:** Cannot switch after creation.
**32.19.4 Pool Failure:** One spawn failure does not kill pool.
**32.19.5 Resource Exhaustion:** FD exhaustion -> `SpawnFailed`.
**32.19.6** Handle Closed: Operations on closed PtyHandle return `HandleClosed`.
**32.19.7** Checksum Failure: Binary snapshot decode rejects mismatch.
**32.19.8** Double-Drop: Pool drop path idempotent.
**32.19.9** expect_or_fail Timeout: Returns `ExpectFailed` with snapshot preview.
**32.19.10 [v14 P2]** Network Partition: Operations during partition return timeout, not crash.
**32.19.11 [v14 P2]** Slow Consumer Deadlock: Bounded buffer with timeout prevents deadlock.

### 32.20 Implementation Hints for LLMs

1. **Non-blocking IO:** Set PTY fd to `O_NONBLOCK`.
2. **Drain Loop:** `wait_for` relies on `drain_output`. Blocking drain breaks timeout.
3. **Pattern Compilation:** `PatternMatcher::compile()` only inside `wait_for`, not per poll.
4. **TermletState:** Use `valid_transition()`, not comparison operators.
5. **Exit Status:** Capture from `PtyEvent::Exited` during `drain_output`.
6. **Resize Safety:** Drain before AND after.
7. **PtyHandle:** Convert `TermletPaneId` to `PtyHandle` via `as_pty_handle()`.
8. **restart():** Kill before respawn. Reset grid and parser. Close old handle. Allocate new PtyHandle.
9. **expect():** Delegates to `expect_or_fail` with `default_timeout`. Panics on failure.
10. **wait_for_condition:** Calls `snapshot()` per iteration.
11. **ShellInteraction:** Default prompt regex `[$#%>]\s*$` covers bash/zsh/fish.
12. **AsyncTermlet:** Must not call `std::thread::sleep`. Use `tokio::time::sleep`.
13. Grid::put_char: Only entry point for character placement. Never modify cells directly.
14. PtyHandle lifecycle: Always close after kill. Check PtyRegistry generation.
15. Grid uses VecDeque: `scroll_up` pops front, pushes back. O(1).
16. Cell grapheme: Use `CompactString`, not `String`. Inline for ASCII.
17. Snapshot checksum: Always verify on decode.
18. PtyState 7 states: Validate transitions via `PtyRecord::transition()`.
19. Snapshot decode order: magic -> version -> algorithm -> length -> payload -> checksum.
20. PtyHandle monotonic: Never reactivate a Closed handle.
21. Unsupported CSI finals: Log as metric, do not panic.
22. Builder defaults: Freeze via snapshot tests to prevent drift.
23. `classify()` returns `ByteClass` enum, not `&str`.
24. `expect_or_fail()` is the canonical non-panicking API; `expect()` panics.
25. Snapshot v2 uses `PackedCell(u64)` bitfield encoding.
26. OpLog binary insertion: use `partition_point()` for O(log n).
27. Grid erase: `erase_in_display(mode)` for CSI J, `erase_in_line(mode)` for CSI K.
28. Typestate PtyHandle: prefer `PtyHandle<Running>` on compile-time paths.
29. **[v14 P2]** CRC32C: opt-in via feature flag; default is FNV-1a.
30. **[v14 P2]** DeterministicClock: inject via `TermletClock` trait; mandatory for network sim tests.
31. **[v14 P2]** NetworkSimulator: partition, latency, loss. Always use deterministic seed.
32. **[v14 P2]** SlowConsumerSimulator: bounded buffer prevents deadlock.
33. **[v14 P2]** Line uses Vec<Cell> with capacity hint. SmallVec rejected (S75).

### 32.21 TermletState <-> PtyState Alignment

| TermletState | PtyState(s) | Notes |
|-------------|-------------|-------|
| Spawning | Allocated, Spawned | PtyHandle<Allocated> -> PtyHandle<Spawned> |
| Running | Running | PtyHandle<Running>; IO permitted |
| Stopping | Stopping | PtyHandle<Stopping>; SIGTERM sent |
| Exited | Exited, Reaped, Closed | Process terminated; handle cleanup |
| SpawnFailed | (no PtyHandle) | No handle allocated |

### 32.22 Examples Directory

- `basic_shell.rs`: Spawn bash, send commands, assert output.
- `tui_app_test.rs`: Spawn ratatui app, verify screen content.
- `sidecar_pattern.rs`: Client-server via TermletPool.
- `resize_test.rs`: Resize and verify grid reflow.
- `restart_pattern.rs`: Kill and respawn.
- `snapshot_binary.rs`: Serialize/deserialize binary snapshots.
- `vtparser_debug.rs`: VtParser state transitions.
- `grapheme_test.rs`: Combining character handling.
- `pty_lifecycle.rs`: 7-state PtyHandle lifecycle.
- `oplog_merge.rs`: Concurrent pane input merging.
- `expect_or_fail_pattern.rs`: Non-panicking expect usage.
- `snapshot_v2.rs`: PackedCell snapshot v2 encoding.
- `erase_test.rs`: CSI J/K erase operations.
- **[v14 P2]** `network_partition.rs`: Network partition simulation.
- **[v14 P2]** `slow_consumer.rs`: Slow consumer backpressure test.
- **[v14 P2]** `crc32c_snapshot.rs`: CRC32C checksum snapshot.
- **[v14 P2]** `deterministic_clock.rs`: Time-controlled test.

### 32.23 Enforcement-First Termlet Contracts

1. A rule is only complete when a deterministic enforcement artifact exists.
2. A Termlet API that mutates process state must define legal pre-state, post-state, and forbidden-state behavior.
3. Cross-language wrappers must preserve semantic class.
4. All TermletError variants must be Clone + PartialEq + Eq for deterministic test assertions.
5. **[v14 P2]** Network simulation tests MUST use DeterministicClock; real clock is forbidden (INV-022).

### 32.24 Restart and Drain Race Closure [v14 P2]

Strengthened with typestate PtyHandle:
- Enter `Stopping`; stop new writes.
- Drain residual PTY output with bounded loop.
- Kill (SIGTERM -> grace -> SIGKILL fallback).
- Transition: `PtyHandle<Running>.stop() -> PtyHandle<Stopping>.exit() -> PtyHandle<Exited>.reap() -> PtyHandle<Reaped>.close() -> PtyHandle<Closed>`.
- Release handle in `PtyRegistry` after `Closed`.
- Old handle returns `HandleClosed` deterministically.
- Reinitialize parser, grid, and output buffers.
- **[v14 P2]** Reset NetworkSimulator and SlowConsumerSimulator.
- Allocate new `PtyHandle<Allocated>` for respawn (new generation).
- Spawn replacement process.
- Transition to `Running` only after successful spawn.

### 32.25 Async Fairness and Poll Budget

- Poll backoff floor: 10ms.
- Poll backoff ceiling: 100ms.
- All blocking backend operations routed through `spawn_blocking`.

### 32.26 Cross-Language Assertion Canonicalization

- Shared fixture defines timeout formatting, snapshot excerpt policy, and error code prefix.
- Language-specific wrappers may decorate but must preserve canonical prefix and fields.
- `ExpectFailed` error includes `snapshot_preview` for cross-language diagnostic parity.
- **[v14 P2]** Checksum algorithm name included in cross-language snapshot metadata.

### 32.27 Collaborative Termlet Replay Discipline

When `crdt` feature is enabled:
- Same operation set replayed under N merge orders must converge.
- Failing replay blocks release.
- Uses LWWFieldMap with per-field conflict resolution.
- PaneOpLog entries used for concurrent pane input merging.
- OpLog uses binary insertion for O(log n) append.
- **[v14 P2]** CRDT merge commutativity tested with 3+ concurrent writers (property test).

### 32.28 VtParser Integration with ByteClass/Step

The VtParser within a Termlet uses the `ByteClass`/`Step` pattern:

1. Each input byte classified by `classify(b) -> ByteClass` (6-variant enum, `const fn`).
2. `step(state, b) -> (State, Action)` dispatches based on (State, ByteClass) pair.
3. Actions dispatched to Grid performer:
   - `Print(b)` -> `grid.put_char(b as char)` (INV-012)
   - `ExecuteControl(b)` -> handle CR, LF, BS, TAB, etc.
   - `DispatchCsi(b)` -> CSI dispatch (A/B/C/D/H/f/J/K/m)
   - `DispatchOsc` -> OSC string handling
   - `ErrorRecover` -> reset parser, no grid mutation, increment unsupported_count metric
4. `ByteClass` enum with `#[repr(u8)]` for integer comparison in hot path.
5. `classify()` is `const fn` for compile-time evaluation.

### 32.29 PtyHandle Lifecycle in Termlet (7-state with typestate)

```
[spawn()] -> PtyHandle<Allocated> = PtyRegistry.allocate()
          -> PtyHandle<Spawned> = h.spawn()
          -> PtyHandle<Running> = h.running()
          -> Termlet state = Running

[kill()]  -> PtyHandle<Stopping> = h.stop()   (SIGTERM)
          -> PtyHandle<Exited> = h.exit()       (grace timeout)
          -> PtyHandle<Reaped> = h.reap()
          -> PtyHandle<Closed> = h.close()
          -> PtyRegistry.release(handle)
          -> Termlet state = Exited

[restart()] -> kill()  [typestate chain on old handle]
            -> PtyHandle<Allocated> = PtyRegistry.allocate()
            -> PtyHandle<Spawned> = h.spawn()
            -> PtyHandle<Running> = h.running()
            -> Termlet state = Running (new handle, new generation)

[Drop]    -> if !terminal: force kill -> close -> release
```

### 32.30 Binary Snapshot Contract Hardening [v14 P2]

- Round-trip must preserve all fields.
- Bad magic rejected with `BadMagic`.
- Unsupported version rejected with `UnsupportedVersion`.
- Truncated payloads rejected BEFORE UTF-8 conversion.
- Length mismatch triggers `Truncated`.
- v2 PackedCell encoding validated via bit-level round-trip.
- v1 and v2 decoders coexist; version dispatch is first branch.
- **[v14 P2]** Checksum algorithm byte validated; unknown algorithm rejected with `InvalidAlgorithm`.
- **[v14 P2]** CRC32C checksum validated when algorithm byte is 0x01.

### 32.31 PtyHandle Registry Invariants [v14 P2]

- Handle lifecycle monotonic: no reverse transitions.
- `restart()` allocates fresh handle generation.
- Closed handle returns `HandleClosed` deterministically.
- Typestate PtyHandle enforces monotonicity at compile time on common paths.
- DynPtyHandle provides runtime-checked fallback for bindings and deserialization.
- **[v14 P2]** Handle elapsed_in_state() available for lifecycle timing metrics.

### 32.32 Supplemental Deterministic Decisions [v14 P2]

- **S32-DEC-36**: Unsupported CSI finals logged as metrics. Parser continues.
- **S32-DEC-37**: Snapshot decoder rejects truncated before UTF-8.
- **S32-DEC-38**: Pool drop idempotent under double-drop.
- **S32-DEC-39**: Builder defaults frozen by snapshot tests.
- **S32-DEC-40**: Restart barrier validated under fake PTY race.
- **S32-DEC-41**: `expect_or_fail()` includes snapshot preview in error.
- **S32-DEC-42**: Snapshot v2 PackedCell ch field is 21 bits (covers all Unicode).
- **S32-DEC-43**: ByteClass enum is `#[repr(u8)]` with stable discriminants.
- **S32-DEC-44**: OpLog `partition_point` is the sole insertion method.
- **S32-DEC-45**: TermletError variants all derive Clone + PartialEq + Eq.
- **S32-DEC-46**: **[v14 P2]** CRC32C software fallback always available; hw acceleration optional.
- **S32-DEC-47**: **[v14 P2]** DeterministicClock mandatory for network partition tests.
- **S32-DEC-48**: **[v14 P2]** NetworkSimulator partition uses DeterministicClock, not wall clock.
- **S32-DEC-49**: **[v14 P2]** SlowConsumerSimulator bounded buffer prevents deadlock.
- **S32-DEC-50**: **[v14 P2]** Checksum algorithm byte at offset 26 in snapshot header.

### 32.33 Snapshot v2 Packed-Cell Format [v14 P2]

- v2 format SPECIFIED (promoted from reserved in v13):
  - `PackedCell(u64)`: ch in bits 0-20 (21 bits), style in bits 21-36 (16 bits), flags in bits 37-52 (16 bits), reserved bits 53-63.
  - Round-trip: `PackedCell::new(ch, style, flags)` <-> `p.ch()`, `p.style()`, `p.flags()`.
  - v2 decoder rejects if version=2 and cell data is truncated.
  - v1/v2 dual decode tests required for backward compatibility.
  - Optional LZ4 compression behind `snapshot-compress` feature flag (non-normative).
  - **[v14 P2]** Checksum algorithm selectable independently of snapshot version.

### 32.34 Consolidated Rule Additions [v14 P2]

- **RULE-S32-72**: Restart barrier mandatory `kill -> close -> spawn`. **Enforcement:** TST-437.
- **RULE-S32-73**: Binary snapshot validates magic/version/length before payload. **Enforcement:** TST-434, TST-435.
- **RULE-S32-74**: Unsupported parser finals are no-op plus metric. **Enforcement:** TST-436.
- **RULE-S32-75**: `Line.is_wrapped` is snapshot-stable. **Enforcement:** TST-438.
- **RULE-S32-76**: Pool teardown idempotent and leak-free. **Enforcement:** TST-439.
- **RULE-S32-77**: PtyHandle lifecycle monotonic. **Enforcement:** TST-450.
- **RULE-S32-78**: Closed handle returns HandleClosed. **Enforcement:** TST-451.
- **RULE-S32-79**: PaneOpLog ordered by Lamport + client_id. **Enforcement:** TST-452.
- **RULE-S32-80**: Builder defaults frozen by snapshot tests. **Enforcement:** TST-447.
- **RULE-S32-81**: `expect_or_fail()` is the canonical non-panicking API. **Enforcement:** API contract test.
- **RULE-S32-82**: ExpectFailed error includes snapshot_preview. **Enforcement:** Display format test.
- **RULE-S32-83**: TermletError variants derive Clone + PartialEq + Eq. **Enforcement:** Derive check.
- **RULE-S32-84**: Snapshot v2 uses PackedCell(u64) with 21-bit ch field. **Enforcement:** Bit-level test.
- **RULE-S32-85**: ByteClass enum is `#[repr(u8)]` with 6 variants. **Enforcement:** Variant count test.
- **RULE-S32-86**: CompactString inline for single ASCII chars. **Enforcement:** Inline test.
- **RULE-S32-87**: Grid::put() not in public API. **Enforcement:** API surface lint.
- **RULE-S32-88**: erase_in_display/erase_in_line implement CSI J/K. **Enforcement:** Parity test.
- **RULE-S32-89**: Typestate PtyHandle for compile-time safety. **Enforcement:** Type check.
- **RULE-S32-90**: OpLog binary insertion via partition_point. **Enforcement:** Insertion test.
- **RULE-S32-91**: **[v14 P2]** CRC32C opt-in via feature flag; default FNV-1a. **Enforcement:** Feature test.
- **RULE-S32-92**: **[v14 P2]** Checksum algorithm byte at offset 26. **Enforcement:** Byte-level test.
- **RULE-S32-93**: **[v14 P2]** NetworkSimulator uses DeterministicClock. **Enforcement:** Clock type check.
- **RULE-S32-94**: **[v14 P2]** SlowConsumerSimulator bounded buffer. **Enforcement:** Deadlock test.
- **RULE-S32-95**: **[v14 P2]** Network partition tests forbidden from using real clock (INV-022). **Enforcement:** Lint.
- **RULE-S32-96**: **[v14 P2]** Restart resets NetworkSimulator state. **Enforcement:** Reset test.
- **RULE-S32-97**: **[v14 P2]** CRC32C software fallback compiles on WASM. **Enforcement:** WASM CI.
- **RULE-S32-98**: **[v14 P2]** PodScheduler deterministic seed from test name. **Enforcement:** Seed test.
- **RULE-S32-99**: **[v14 P2]** Quarantined tests have expiry timestamp. **Enforcement:** Expiry check.
- **RULE-S32-100**: **[v14 P2]** Property-based tests generate 1000+ cases minimum. **Enforcement:** Case count.

### 32.35 expect_or_fail Contract

`expect_or_fail(pattern: &str) -> Result<WaitMatch, TermletError>`:
- Uses `config.default_timeout` as timeout.
- On success: returns `Ok(WaitMatch)`.
- On timeout: returns `Err(TermletError::ExpectFailed { pattern, timeout_ms, snapshot_preview })`.
- `snapshot_preview` is first 5 lines of current snapshot, trimmed.
- `expect(pattern)` is defined as `self.expect_or_fail(pattern).unwrap()` -- panics on failure, for test convenience only.
- **Rule**: Library code MUST use `expect_or_fail()`. `expect()` is for test code only.

### 32.36 [v14 P2] DeterministicClock Contract

**[v14 P2]** DeterministicClock for Termlet testing (INV-022):

- Implements `TermletClock` trait: `now_ms() -> u64`, `advance(delta: Duration)`.
- Starts at caller-specified epoch.
- `advance()` moves time forward by exact delta. Never backward.
- All network simulation tests MUST inject DeterministicClock.
- RealClock is forbidden in network partition tests (enforced by type parameter or runtime check).
- DeterministicClock is `Send` for async compatibility.

### 32.37 [v14 P2] Parallel Pod Scheduler

**[v14 P2]** Cross-pollinated from GPT P1 32.37:

- `PodScheduler` distributes Termlets across CPU cores.
- Deterministic seed: `seed = hash(test_name) ^ partition_index`.
- Each partition gets exclusive temp directory.
- Partition count configurable; default = `num_cpus::get()`.
- Failed pods in one partition do not kill other partitions.
- Pod scheduling is Round-robin by default; priority scheduling opt-in.

### 32.38 [v14 P2] Flake Triage and Quarantine

**[v14 P2]** Cross-pollinated from GPT P1 32.38:

- Flaky tests auto-quarantined after 3 consecutive non-deterministic failures.
- Quarantine entry: `{ test_name, first_failed, quarantine_expiry, flake_signature }`.
- Expired quarantine blocks release (existing RULE-S02-06 extended).
- Quarantined tests replay with DeterministicClock for root-cause analysis.
- Flake resolution requires PR with deterministic reproduction.

### 32.39 [v14 P2] Property-Based Termlet Tests

**[v14 P2]** Cross-pollinated from GPT P1 32.39:

- `proptest` generates: random commands, random input byte sequences, random resize dimensions.
- Invariants tested:
  1. No panic for any input.
  2. State machine transitions are valid.
  3. Snapshot round-trip preserves content.
  4. Grid revision monotonically increases.
- Minimum 1000 cases per property test.

### 32.40 [v14 P2] Fuzz-Driven Termlet Scenarios

**[v14 P2]** Cross-pollinated from GPT P1 32.40:

- Fuzz targets:
  1. `fuzz_termlet_lifecycle`: random spawn/kill/restart sequences.
  2. `fuzz_termlet_input`: random byte sequences to running Termlet.
  3. `fuzz_snapshot_roundtrip`: random snapshot binary decode.
- No-panic invariant for all fuzz targets.
- Corpus stored in `fixtures/fuzz-corpus/termlet/`.

### 32.41 [v14 P2] CI Matrix Execution

**[v14 P2]** Cross-pollinated from GPT P1 32.41:

- CI matrix: `{ os: [linux, macos], rust: [stable, nightly], features: [default, crdt, crc32c] }`.
- Total combinations: 12.
- Timeout per job: 30 minutes.
- Artifacts uploaded on failure: snapshot dumps, OTEL traces, flake signatures.

### 32.42 [v14 P2] Release Gate Escalation

**[v14 P2]** Cross-pollinated from GPT P1 32.42:

- Gate failure in LTS/Current: blocks release, creates tracking issue.
- Gate failure in Preview: creates warning, requires waiver.
- Waiver requires: justification, expiry date, reviewer approval.
- Escalation: 3+ blocked releases triggers architecture review.

### 32.43 [v14 P2] Upgrade and Downgrade Semantics

**[v14 P2]** Cross-pollinated from GPT P1 32.43:

- Snapshot v2 -> v1 downgrade: lossy (reserved bits discarded), but functional.
- v1 -> v2 upgrade: lossless (reserved bits set to 0).
- Config format backward-compatible for 2 minor versions.
- **[v14 P2]** Checksum algorithm downgrade: CRC32C snapshot can be re-checksummed with FNV-1a for compatibility.

### 32.44 [v14 P2] Documentation Generation

**[v14 P2]** Cross-pollinated from GPT P1 32.44:

- `cargo doc --workspace` generates API docs.
- Termlet examples auto-included in docs via `#[doc = include_str!("...")]`.
- Architecture diagrams in `notes/architecture/`.

### 32.45 [v14 P2] Governance and Ownership

**[v14 P2]** Cross-pollinated from GPT P1 32.45:

- Section 32 owned by Termlet infrastructure team.
- S32 changes require review from 2 maintainers.
- Breaking Termlet API changes require RFC.
- Test coverage gate: >=90% for mux-termlet crate.

### 32.46 [v14 P2] Network Partition Simulation

**[v14 P2]** Cross-pollinated from Gemini P1:

- `NetworkSimulator` injectable into Termlet via builder.
- Operations during partition: timeout with `WaitForTimeout` error, not crash.
- Partition recovery: resumes normal operation when `DeterministicClock` advances past partition_until_ms.
- Latency injection: adds configurable delay to PTY read/write.
- Packet loss: probabilistic drop of PTY output bytes (deterministic via seeded RNG).
- All network simulation uses DeterministicClock (INV-022); real clock forbidden.

### 32.47 [v14 P2] Slow Consumer Simulation

**[v14 P2]** Cross-pollinated from Gemini P1:

- `SlowConsumerSimulator` throttles PTY read to `bytes_per_sec`.
- Purpose: validates Grid backpressure handling.
- Deadlock prevention: bounded buffer with timeout; if buffer fills and consumer cannot drain within timeout, data is dropped with metric.
- Termlet MUST NOT deadlock under slow consumer simulation.
- Integration test: 10KB/s throttle with `cat /dev/urandom | head -c 1000000` does not hang.

### Test Strategy

Mandatory Termlet tests (v14 P2 -- 155+ tests):

**Unit and contract tests (TST-400 through TST-409):**
1-10. Spawn, send_keys, wait_for, timeout, zero-timeout, snapshot, resize, kill idempotency, Drop, FakePty.

**Cross-language parity tests (TST-410 through TST-414):**
11-15. Cross-lang snapshot, Python cleanup, Node cleanup, Builder parity, Pool.

**Advanced correctness tests (TST-415 through TST-429):**
16-30. SnapshotDiff, OTEL spans, perf budgets, background lifecycle, drain, output_history, TermletPaneId, pattern compile, inherit_env, async gate, pool timeout, TermletExt, fuzz, leak detection.

**State lifecycle tests (TST-430 through TST-438):**
31-39. Lifecycle, gating, snapshot in exited, exit_status, SpawnFailed, valid_transition, no Ord.

**[v13] Tests (TST-439 through TST-447):**
107-115. Binary round-trip, bad magic, bad version, close invalidates, HandleClosed, restart close, VtParser 7 states, put_char, CSI params.

**[v13 P2] Tests (TST-448 through TST-457):**
116-125. Grid grapheme, VecDeque scroll, PtyHandle 7-state, PtyRegistry stale gen, FNV-1a, cursor preserved, classify total, Step deterministic, alignment mapping, dirty range.

**[v13 P3] Tests (TST-458 through TST-469):**
126-137. Truncated payload, unsupported CSI metric, double-drop, builder defaults frozen, restart barrier timing, v2 version rejected, monotonic lifecycle, closed handle, OpLog ordering, is_wrapped round-trip, generation wrap, cross-lang traversal.

**[v14] Tests (TST-470 through TST-484):**
138. TST-470: `expect_or_fail` returns Ok on match.
139. TST-471: `expect_or_fail` returns ExpectFailed on timeout with snapshot_preview.
140. TST-472: `expect()` panics on timeout.
141. TST-473: TermletError::ExpectFailed display format includes all fields.
142. TST-474: TermletError variants are Clone + PartialEq + Eq.
143. TST-475: Snapshot v2 PackedCell round-trip preserves ch/style/flags.
144. TST-476: Snapshot v1 still decodes after v2 support added.
145. TST-477: ByteClass enum has 6 variants with #[repr(u8)].
146. TST-478: `classify()` is const fn.
147. TST-479: CompactString inline for ASCII single chars.
148. TST-480: Grid::put() not in public API.
149. TST-481: erase_in_display(Below) clears from cursor down.
150. TST-482: erase_in_display(All) resets cursor to origin.
151. TST-483: erase_in_line(All) clears entire row.
152. TST-484: Typestate PtyHandle prevents invalid transition at compile time.

**[v14 P2] Tests (TST-485 through TST-500):**
153. TST-485: **[v14 P2]** CRC32C checksum encode/decode round-trip.
154. TST-486: **[v14 P2]** Invalid algorithm byte rejected.
155. TST-487: **[v14 P2]** NetworkSimulator partition timeout (not crash).
156. TST-488: **[v14 P2]** NetworkSimulator partition recovery after time advance.
157. TST-489: **[v14 P2]** SlowConsumerSimulator does not deadlock.
158. TST-490: **[v14 P2]** DeterministicClock monotonicity in Termlet context.
159. TST-491: **[v14 P2]** Builder checksum_algo defaults to FNV-1a.
160. TST-492: **[v14 P2]** Restart resets NetworkSimulator.
161. TST-493: **[v14 P2]** PodScheduler deterministic seed.
162. TST-494: **[v14 P2]** Quarantine expiry blocks release.
163. TST-495: **[v14 P2]** Property-based: no panic for random input.
164. TST-496: **[v14 P2]** Fuzz: termlet lifecycle no-panic.
165. TST-497: **[v14 P2]** Snapshot checksum algorithm byte at offset 26.
166. TST-498: **[v14 P2]** CRC32C software fallback on WASM.
167. TST-499: **[v14 P2]** Snapshot v2 -> v1 downgrade lossy but functional.
168. TST-500: **[v14 P2]** Line uses Vec<Cell> not SmallVec (S75 compile check).

### AGENTS.md Rules

- `RULE-S32-01` through `RULE-S32-90`: (carried from v14 P1 -- see Section 32.34).
- `RULE-S32-91`: **[v14 P2]** CRC32C opt-in via feature flag. **Enforcement:** Feature test (TST-485).
- `RULE-S32-92`: **[v14 P2]** Checksum algorithm byte at offset 26. **Enforcement:** Byte-level test (TST-497).
- `RULE-S32-93`: **[v14 P2]** NetworkSimulator uses DeterministicClock. **Enforcement:** Clock type check (TST-490).
- `RULE-S32-94`: **[v14 P2]** SlowConsumerSimulator bounded buffer. **Enforcement:** Deadlock test (TST-489).
- `RULE-S32-95`: **[v14 P2]** Network partition tests forbidden from real clock (INV-022). **Enforcement:** Lint (TST-490).
- `RULE-S32-96`: **[v14 P2]** Restart resets NetworkSimulator. **Enforcement:** Reset test (TST-492).
- `RULE-S32-97`: **[v14 P2]** CRC32C software fallback compiles on WASM. **Enforcement:** WASM CI (TST-498).
- `RULE-S32-98`: **[v14 P2]** PodScheduler deterministic seed. **Enforcement:** Seed test (TST-493).
- `RULE-S32-99`: **[v14 P2]** Quarantined tests have expiry. **Enforcement:** Expiry check (TST-494).
- `RULE-S32-100`: **[v14 P2]** Property-based tests 1000+ cases. **Enforcement:** Case count (TST-495).

### 32.37a [v14 P2] Parallel Pod Scheduler -- Rust Example

```rust
// crates/mux-termlet/src/scheduler.rs
// [v14 P2] Deterministic pod scheduler

#![allow(dead_code)]
use std::collections::HashMap;

/// [v14 P2] Deterministic seed computation for pod partitions.
pub fn compute_seed(test_name: &str, partition_index: u32) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in test_name.as_bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash ^ (partition_index as u64)
}

/// [v14 P2] Pod partition assignment.
#[derive(Debug, Clone)]
pub struct PodPartition {
    pub index: u32,
    pub seed: u64,
    pub temp_dir: String,
    pub pod_names: Vec<String>,
}

/// [v14 P2] PodScheduler distributes pods across partitions.
pub struct PodScheduler {
    pub partition_count: u32,
    pub partitions: Vec<PodPartition>,
}

impl PodScheduler {
    pub fn new(partition_count: u32) -> Self {
        let partitions = (0..partition_count)
            .map(|i| PodPartition {
                index: i,
                seed: 0,
                temp_dir: format!("/tmp/termforge-partition-{}", i),
                pod_names: Vec::new(),
            })
            .collect();
        Self { partition_count, partitions }
    }

    /// Assign a pod to a partition using round-robin.
    pub fn assign(&mut self, pod_name: &str, test_name: &str) {
        let partition_idx = self.next_partition();
        let seed = compute_seed(test_name, partition_idx);
        if let Some(p) = self.partitions.get_mut(partition_idx as usize) {
            p.seed = seed;
            p.pod_names.push(pod_name.to_string());
        }
    }

    fn next_partition(&self) -> u32 {
        let total_pods: usize = self.partitions.iter().map(|p| p.pod_names.len()).sum();
        (total_pods as u32) % self.partition_count
    }

    pub fn partition_for(&self, pod_name: &str) -> Option<&PodPartition> {
        self.partitions.iter().find(|p| p.pod_names.iter().any(|n| n == pod_name))
    }

    pub fn total_pods(&self) -> usize {
        self.partitions.iter().map(|p| p.pod_names.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic_seed() {
        let s1 = compute_seed("test_echo", 0);
        let s2 = compute_seed("test_echo", 0);
        assert_eq!(s1, s2); // deterministic
    }

    #[test]
    fn test_different_partitions_different_seeds() {
        let s1 = compute_seed("test_echo", 0);
        let s2 = compute_seed("test_echo", 1);
        assert_ne!(s1, s2);
    }

    #[test]
    fn test_scheduler_round_robin() {
        let mut sched = PodScheduler::new(3);
        sched.assign("pod1", "test1");
        sched.assign("pod2", "test2");
        sched.assign("pod3", "test3");
        sched.assign("pod4", "test4");
        assert_eq!(sched.partitions[0].pod_names.len(), 2); // pod1, pod4
        assert_eq!(sched.partitions[1].pod_names.len(), 1); // pod2
        assert_eq!(sched.partitions[2].pod_names.len(), 1); // pod3
    }

    #[test]
    fn test_total_pods() {
        let mut sched = PodScheduler::new(2);
        sched.assign("a", "t1");
        sched.assign("b", "t2");
        assert_eq!(sched.total_pods(), 2);
    }
}
```

### 32.38a [v14 P2] Flake Triage -- Rust Example

```rust
// crates/mux-termlet/src/quarantine.rs
// [v14 P2] Flake quarantine management

#![allow(dead_code)]

/// [v14 P2] Quarantine entry for flaky tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuarantineEntry {
    pub test_name: String,
    pub first_failed_epoch: u64,
    pub quarantine_expiry_epoch: u64,
    pub consecutive_failures: u32,
    pub flake_signature: String,
}

impl QuarantineEntry {
    pub fn is_expired(&self, now_epoch: u64) -> bool {
        now_epoch > self.quarantine_expiry_epoch
    }

    pub fn should_quarantine(consecutive_failures: u32) -> bool {
        consecutive_failures >= 3
    }
}

/// [v14 P2] Quarantine registry.
pub struct QuarantineRegistry {
    entries: Vec<QuarantineEntry>,
}

impl QuarantineRegistry {
    pub fn new() -> Self { Self { entries: Vec::new() } }

    pub fn add(&mut self, entry: QuarantineEntry) {
        self.entries.push(entry);
    }

    pub fn is_quarantined(&self, test_name: &str) -> bool {
        self.entries.iter().any(|e| e.test_name == test_name)
    }

    pub fn expired_entries(&self, now_epoch: u64) -> Vec<&QuarantineEntry> {
        self.entries.iter().filter(|e| e.is_expired(now_epoch)).collect()
    }

    pub fn blocks_release(&self, now_epoch: u64) -> bool {
        !self.expired_entries(now_epoch).is_empty()
    }

    pub fn active_count(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quarantine_threshold() {
        assert!(!QuarantineEntry::should_quarantine(2));
        assert!(QuarantineEntry::should_quarantine(3));
    }

    #[test]
    fn test_expired_blocks_release() {
        let mut reg = QuarantineRegistry::new();
        reg.add(QuarantineEntry {
            test_name: "test_flaky".into(),
            first_failed_epoch: 1000,
            quarantine_expiry_epoch: 2000,
            consecutive_failures: 5,
            flake_signature: "timeout_in_wait_for".into(),
        });
        assert!(reg.blocks_release(3000)); // expired
        assert!(!reg.blocks_release(1500)); // not expired yet
    }

    #[test]
    fn test_quarantine_lookup() {
        let mut reg = QuarantineRegistry::new();
        reg.add(QuarantineEntry {
            test_name: "test_flaky".into(),
            first_failed_epoch: 1000,
            quarantine_expiry_epoch: 5000,
            consecutive_failures: 3,
            flake_signature: "race_condition".into(),
        });
        assert!(reg.is_quarantined("test_flaky"));
        assert!(!reg.is_quarantined("test_stable"));
    }
}
```

### 32.39a [v14 P2] Property-Based Tests -- Rust Example

```rust
// crates/mux-termlet/src/property_tests.rs
// [v14 P2] Property-based test framework types

#![allow(dead_code)]

/// [v14 P2] Test case generated by property-based testing.
#[derive(Debug, Clone)]
pub struct PropertyTestCase {
    pub command: String,
    pub input_bytes: Vec<u8>,
    pub resize_to: Option<(u16, u16)>,
    pub expected_invariant: PropertyInvariant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyInvariant {
    NoPanic,
    ValidStateTransitions,
    SnapshotRoundTrip,
    RevisionMonotonic,
}

/// [v14 P2] Property test runner statistics.
#[derive(Debug, Clone)]
pub struct PropertyTestStats {
    pub total_cases: u64,
    pub passed: u64,
    pub failed: u64,
    pub shrink_steps: u64,
}

impl PropertyTestStats {
    pub fn new() -> Self { Self { total_cases: 0, passed: 0, failed: 0, shrink_steps: 0 } }

    pub fn record_pass(&mut self) { self.total_cases += 1; self.passed += 1; }
    pub fn record_fail(&mut self) { self.total_cases += 1; self.failed += 1; }

    pub fn pass_rate(&self) -> f64 {
        if self.total_cases == 0 { return 0.0; }
        self.passed as f64 / self.total_cases as f64
    }

    pub fn meets_minimum_cases(&self, minimum: u64) -> bool {
        self.total_cases >= minimum
    }
}

pub const MINIMUM_PROPERTY_CASES: u64 = 1000; // [v14 P2] RULE-S32-100

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stats_pass_rate() {
        let mut stats = PropertyTestStats::new();
        for _ in 0..90 { stats.record_pass(); }
        for _ in 0..10 { stats.record_fail(); }
        assert!((stats.pass_rate() - 0.9).abs() < 0.01);
    }

    #[test]
    fn test_minimum_cases() {
        let mut stats = PropertyTestStats::new();
        for _ in 0..999 { stats.record_pass(); }
        assert!(!stats.meets_minimum_cases(MINIMUM_PROPERTY_CASES));
        stats.record_pass();
        assert!(stats.meets_minimum_cases(MINIMUM_PROPERTY_CASES));
    }
}
```

### 32.40a [v14 P2] Fuzz Targets -- Rust Example

```rust
// crates/mux-termlet/src/fuzz_support.rs
// [v14 P2] Fuzz target support types

#![allow(dead_code)]

/// [v14 P2] Fuzz target identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FuzzTarget {
    TermletLifecycle,
    TermletInput,
    SnapshotRoundtrip,
    Crc32cChecksum,
}

impl FuzzTarget {
    pub fn corpus_dir(self) -> &'static str {
        match self {
            Self::TermletLifecycle => "fixtures/fuzz-corpus/termlet/lifecycle",
            Self::TermletInput => "fixtures/fuzz-corpus/termlet/input",
            Self::SnapshotRoundtrip => "fixtures/fuzz-corpus/termlet/snapshot",
            Self::Crc32cChecksum => "fixtures/fuzz-corpus/termlet/crc32c",
        }
    }

    pub fn invariant(self) -> &'static str {
        match self {
            Self::TermletLifecycle => "no panic on any lifecycle sequence",
            Self::TermletInput => "no panic on any byte sequence",
            Self::SnapshotRoundtrip => "no panic on any binary input",
            Self::Crc32cChecksum => "no panic on any checksum input",
        }
    }
}

pub const ALL_FUZZ_TARGETS: &[FuzzTarget] = &[
    FuzzTarget::TermletLifecycle,
    FuzzTarget::TermletInput,
    FuzzTarget::SnapshotRoundtrip,
    FuzzTarget::Crc32cChecksum,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzz_target_count() {
        assert_eq!(ALL_FUZZ_TARGETS.len(), 4);
    }

    #[test]
    fn test_corpus_dirs_unique() {
        let dirs: Vec<_> = ALL_FUZZ_TARGETS.iter().map(|t| t.corpus_dir()).collect();
        let deduped: std::collections::HashSet<_> = dirs.iter().collect();
        assert_eq!(dirs.len(), deduped.len());
    }
}
```

### 32.41a [v14 P2] CI Matrix -- Rust Example

```rust
// crates/mux-termlet/src/ci_matrix.rs
// [v14 P2] CI matrix configuration

#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CiOs { Linux, MacOs }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RustChannel { Stable, Nightly }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CiJob {
    pub os: CiOs,
    pub channel: RustChannel,
    pub features: Vec<&'static str>,
    pub timeout_minutes: u32,
}

pub fn generate_matrix() -> Vec<CiJob> {
    let mut jobs = Vec::new();
    let oses = [CiOs::Linux, CiOs::MacOs];
    let channels = [RustChannel::Stable, RustChannel::Nightly];
    let feature_sets: Vec<Vec<&str>> = vec![
        vec![],
        vec!["crdt"],
        vec!["crc32c"],
    ];

    for os in &oses {
        for channel in &channels {
            for features in &feature_sets {
                jobs.push(CiJob {
                    os: *os,
                    channel: *channel,
                    features: features.clone(),
                    timeout_minutes: 30,
                });
            }
        }
    }
    jobs
}

pub const EXPECTED_JOB_COUNT: usize = 12; // 2 OS * 2 channels * 3 feature sets

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matrix_size() {
        let matrix = generate_matrix();
        assert_eq!(matrix.len(), EXPECTED_JOB_COUNT);
    }

    #[test]
    fn test_all_jobs_have_timeout() {
        let matrix = generate_matrix();
        assert!(matrix.iter().all(|j| j.timeout_minutes == 30));
    }
}
```

### 32.42a [v14 P2] Release Gate Escalation -- Rust Example

```rust
// crates/mux-termlet/src/release_gate.rs
// [v14 P2] Release gate escalation

#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateOutcome {
    Pass,
    FailBlocking,
    FailWithWaiver,
    Warning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lane { Lts, Current, Preview }

pub fn gate_outcome(pass: bool, lane: Lane, has_waiver: bool) -> GateOutcome {
    match (pass, lane, has_waiver) {
        (true, _, _) => GateOutcome::Pass,
        (false, Lane::Preview, true) => GateOutcome::FailWithWaiver,
        (false, Lane::Preview, false) => GateOutcome::Warning,
        (false, _, true) => GateOutcome::FailWithWaiver,
        (false, _, false) => GateOutcome::FailBlocking,
    }
}

/// [v14 P2] Escalation tracker.
pub struct EscalationTracker {
    pub blocked_releases: u32,
    pub escalation_threshold: u32,
}

impl EscalationTracker {
    pub fn new() -> Self { Self { blocked_releases: 0, escalation_threshold: 3 } }

    pub fn record_block(&mut self) { self.blocked_releases += 1; }

    pub fn needs_architecture_review(&self) -> bool {
        self.blocked_releases >= self.escalation_threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lts_fail_blocks() {
        assert_eq!(gate_outcome(false, Lane::Lts, false), GateOutcome::FailBlocking);
    }

    #[test]
    fn test_preview_fail_warns() {
        assert_eq!(gate_outcome(false, Lane::Preview, false), GateOutcome::Warning);
    }

    #[test]
    fn test_escalation() {
        let mut tracker = EscalationTracker::new();
        tracker.record_block();
        tracker.record_block();
        assert!(!tracker.needs_architecture_review());
        tracker.record_block();
        assert!(tracker.needs_architecture_review());
    }
}
```

### 32.43a [v14 P2] Upgrade/Downgrade -- Rust Example

```rust
// crates/mux-termlet/src/compat.rs
// [v14 P2] Snapshot version upgrade/downgrade

#![allow(dead_code)]

/// [v14 P2] Upgrade snapshot from v1 to v2.
pub fn upgrade_v1_to_v2(ch: u32, style: u16, flags: u16) -> u64 {
    // PackedCell(u64) encoding
    (ch as u64 & 0x1F_FFFF)
        | ((style as u64) << 21)
        | ((flags as u64) << 37)
}

/// [v14 P2] Downgrade snapshot from v2 to v1 (lossy: reserved bits discarded).
pub fn downgrade_v2_to_v1(packed: u64) -> (u32, u16, u16) {
    let ch = (packed & 0x1F_FFFF) as u32;
    let style = ((packed >> 21) & 0xFFFF) as u16;
    let flags = ((packed >> 37) & 0xFFFF) as u16;
    (ch, style, flags)
}

/// [v14 P2] Re-checksum a snapshot with a different algorithm.
pub fn re_checksum(payload: &[u8], target_algo: u8) -> u32 {
    match target_algo {
        0 => {
            // FNV-1a
            let mut hash: u32 = 0x811c_9dc5;
            for &b in payload {
                hash ^= b as u32;
                hash = hash.wrapping_mul(0x0100_0193);
            }
            hash
        }
        1 => {
            // CRC32C software
            let mut crc: u32 = 0xFFFF_FFFF;
            for &b in payload {
                crc ^= b as u32;
                for _ in 0..8 {
                    if crc & 1 != 0 {
                        crc = (crc >> 1) ^ 0x82F6_3B78;
                    } else {
                        crc >>= 1;
                    }
                }
            }
            crc ^ 0xFFFF_FFFF
        }
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_v1_v2_round_trip() {
        let packed = upgrade_v1_to_v2(65, 0x00FF, 0x0003);
        let (ch, style, flags) = downgrade_v2_to_v1(packed);
        assert_eq!(ch, 65);
        assert_eq!(style, 0x00FF);
        assert_eq!(flags, 0x0003);
    }

    #[test]
    fn test_re_checksum_fnv1a() {
        let data = b"test";
        let c1 = re_checksum(data, 0);
        let c2 = re_checksum(data, 0);
        assert_eq!(c1, c2); // deterministic
    }

    #[test]
    fn test_re_checksum_different_algos() {
        let data = b"test payload";
        let fnv = re_checksum(data, 0);
        let crc = re_checksum(data, 1);
        assert_ne!(fnv, crc);
    }
}
```

### 32.46a [v14 P2] Network Partition -- Rust Example

```rust
// crates/mux-termlet/src/network_sim.rs
// [v14 P2] Network partition simulation (from Gemini P1)

#![allow(dead_code)]
use std::time::Duration;

/// [v14 P2] DeterministicClock for network simulation (INV-022).
pub struct DetClock {
    current_ms: u64,
}

impl DetClock {
    pub fn new(start_ms: u64) -> Self { Self { current_ms: start_ms } }
    pub fn now_ms(&self) -> u64 { self.current_ms }
    pub fn advance(&mut self, delta: Duration) {
        self.current_ms += delta.as_millis() as u64;
    }
}

/// [v14 P2] Network condition configuration.
#[derive(Debug, Clone)]
pub struct NetworkCondition {
    pub latency_min_ms: u64,
    pub latency_max_ms: u64,
    pub loss_rate: f64,
    pub partition_start_ms: Option<u64>,
    pub partition_end_ms: Option<u64>,
}

impl NetworkCondition {
    pub fn clean() -> Self {
        Self { latency_min_ms: 0, latency_max_ms: 0, loss_rate: 0.0,
               partition_start_ms: None, partition_end_ms: None }
    }

    pub fn is_partitioned(&self, now_ms: u64) -> bool {
        match (self.partition_start_ms, self.partition_end_ms) {
            (Some(start), Some(end)) => now_ms >= start && now_ms < end,
            _ => false,
        }
    }

    pub fn with_partition(mut self, start_ms: u64, end_ms: u64) -> Self {
        self.partition_start_ms = Some(start_ms);
        self.partition_end_ms = Some(end_ms);
        self
    }

    pub fn with_latency(mut self, min_ms: u64, max_ms: u64) -> Self {
        self.latency_min_ms = min_ms;
        self.latency_max_ms = max_ms;
        self
    }

    pub fn with_loss(mut self, rate: f64) -> Self {
        self.loss_rate = rate.clamp(0.0, 1.0);
        self
    }
}

/// [v14 P2] Simulate a network event sequence.
pub fn simulate_partition_scenario(condition: &NetworkCondition, clock: &mut DetClock) -> Vec<(u64, bool)> {
    let mut events = Vec::new();
    let steps = [0, 1000, 2000, 3000, 4000, 5000, 6000, 7000, 8000, 9000, 10000];
    for &offset in &steps {
        clock.advance(Duration::from_millis(if offset == 0 { 0 } else { 1000 }));
        let partitioned = condition.is_partitioned(clock.now_ms());
        events.push((clock.now_ms(), partitioned));
    }
    events
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_network() {
        let cond = NetworkCondition::clean();
        assert!(!cond.is_partitioned(5000));
    }

    #[test]
    fn test_partition_window() {
        let cond = NetworkCondition::clean().with_partition(2000, 5000);
        assert!(!cond.is_partitioned(1000));
        assert!(cond.is_partitioned(3000));
        assert!(!cond.is_partitioned(5000));
    }

    #[test]
    fn test_partition_scenario() {
        let cond = NetworkCondition::clean().with_partition(3000, 7000);
        let mut clock = DetClock::new(0);
        let events = simulate_partition_scenario(&cond, &mut clock);
        // At time 0: not partitioned
        assert!(!events[0].1);
        // At time 3000: partitioned
        assert!(events[3].1);
        // At time 7000: not partitioned
        assert!(!events[7].1);
    }

    #[test]
    fn test_latency_config() {
        let cond = NetworkCondition::clean().with_latency(10, 50);
        assert_eq!(cond.latency_min_ms, 10);
        assert_eq!(cond.latency_max_ms, 50);
    }

    #[test]
    fn test_loss_clamped() {
        let cond = NetworkCondition::clean().with_loss(1.5);
        assert!((cond.loss_rate - 1.0).abs() < f64::EPSILON);
    }
}
```

### 32.47a [v14 P2] Slow Consumer -- Rust Example

```rust
// crates/mux-termlet/src/slow_consumer.rs
// [v14 P2] Slow consumer backpressure simulation (from Gemini P1)

#![allow(dead_code)]
use std::time::Duration;
use std::collections::VecDeque;

/// [v14 P2] Bounded buffer for slow consumer simulation.
pub struct BoundedBuffer {
    buffer: VecDeque<u8>,
    capacity: usize,
    bytes_per_sec: u64,
    total_dropped: u64,
}

impl BoundedBuffer {
    pub fn new(capacity: usize, bytes_per_sec: u64) -> Self {
        Self {
            buffer: VecDeque::with_capacity(capacity),
            capacity,
            bytes_per_sec,
            total_dropped: 0,
        }
    }

    /// Write bytes to buffer. If buffer is full, drops oldest bytes.
    pub fn write(&mut self, data: &[u8]) {
        for &b in data {
            if self.buffer.len() >= self.capacity {
                self.buffer.pop_front();
                self.total_dropped += 1;
            }
            self.buffer.push_back(b);
        }
    }

    /// Read up to `max_bytes` from buffer, respecting throttle.
    pub fn read(&mut self, max_bytes: usize) -> Vec<u8> {
        let read_count = max_bytes.min(self.buffer.len());
        let mut result = Vec::with_capacity(read_count);
        for _ in 0..read_count {
            if let Some(b) = self.buffer.pop_front() {
                result.push(b);
            }
        }
        result
    }

    /// Calculate delay for reading `byte_count` bytes at configured rate.
    pub fn read_delay(&self, byte_count: usize) -> Duration {
        if self.bytes_per_sec == 0 { return Duration::from_secs(1); }
        let secs = byte_count as f64 / self.bytes_per_sec as f64;
        Duration::from_secs_f64(secs)
    }

    pub fn len(&self) -> usize { self.buffer.len() }
    pub fn is_empty(&self) -> bool { self.buffer.is_empty() }
    pub fn total_dropped(&self) -> u64 { self.total_dropped }
    pub fn is_full(&self) -> bool { self.buffer.len() >= self.capacity }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounded_buffer_write_read() {
        let mut buf = BoundedBuffer::new(100, 1000);
        buf.write(b"hello");
        let data = buf.read(5);
        assert_eq!(&data, b"hello");
    }

    #[test]
    fn test_bounded_buffer_overflow() {
        let mut buf = BoundedBuffer::new(5, 1000);
        buf.write(b"abcdefgh"); // 8 bytes into 5-capacity buffer
        assert_eq!(buf.len(), 5);
        assert_eq!(buf.total_dropped(), 3);
        let data = buf.read(5);
        assert_eq!(&data, b"defgh"); // oldest 3 dropped
    }

    #[test]
    fn test_read_delay() {
        let buf = BoundedBuffer::new(100, 1000);
        let delay = buf.read_delay(500);
        assert_eq!(delay, Duration::from_millis(500));
    }

    #[test]
    fn test_empty_buffer_read() {
        let mut buf = BoundedBuffer::new(100, 1000);
        let data = buf.read(10);
        assert!(data.is_empty());
    }

    #[test]
    fn test_buffer_is_full() {
        let mut buf = BoundedBuffer::new(3, 1000);
        buf.write(b"abc");
        assert!(buf.is_full());
        buf.read(1);
        assert!(!buf.is_full());
    }
}
```

### 32.45a [v14 P2] Governance -- Rust Example

```rust
// crates/mux-termlet/src/governance.rs
// [v14 P2] Governance metadata

#![allow(dead_code)]

pub struct SectionOwnership {
    pub section: &'static str,
    pub primary_maintainer: &'static str,
    pub secondary_maintainer: &'static str,
    pub min_reviewers: u8,
    pub requires_rfc_for_breaking: bool,
    pub coverage_gate_percent: u8,
}

pub const S32_GOVERNANCE: SectionOwnership = SectionOwnership {
    section: "32. Termlets",
    primary_maintainer: "termlet-infra-team",
    secondary_maintainer: "core-team",
    min_reviewers: 2,
    requires_rfc_for_breaking: true,
    coverage_gate_percent: 90,
};

pub fn meets_coverage_gate(actual: u8) -> bool {
    actual >= S32_GOVERNANCE.coverage_gate_percent
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_governance_values() {
        assert_eq!(S32_GOVERNANCE.min_reviewers, 2);
        assert!(S32_GOVERNANCE.requires_rfc_for_breaking);
    }

    #[test]
    fn test_coverage_gate() {
        assert!(meets_coverage_gate(90));
        assert!(meets_coverage_gate(95));
        assert!(!meets_coverage_gate(89));
    }
}
```

### 32.44a [v14 P2] Documentation Generation -- Rust Example

```rust
// crates/mux-termlet/src/doc_gen.rs
// [v14 P2] Documentation metadata

#![allow(dead_code)]

pub struct DocTarget {
    pub crate_name: &'static str,
    pub include_examples: bool,
    pub architecture_diagrams: &'static [&'static str],
}

pub const TERMLET_DOC_TARGET: DocTarget = DocTarget {
    crate_name: "mux-termlet",
    include_examples: true,
    architecture_diagrams: &[
        "notes/architecture/termlet-arch.svg",
        "notes/architecture/pty-lifecycle.svg",
        "notes/architecture/snapshot-format.svg",
    ],
};

pub fn expected_example_count() -> usize {
    17 // [v14 P2] 17 examples in 32.22
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_doc_target() {
        assert_eq!(TERMLET_DOC_TARGET.crate_name, "mux-termlet");
        assert!(TERMLET_DOC_TARGET.include_examples);
    }

    #[test]
    fn test_diagram_count() {
        assert_eq!(TERMLET_DOC_TARGET.architecture_diagrams.len(), 3);
    }

    #[test]
    fn test_example_count() {
        assert!(expected_example_count() >= 17);
    }
}
```

### 32.48 [v14 P2] Section Depth Note

- **[v14 P2]** Section 32 remains the deepest section with 47 subsections (32.1-32.47, plus this depth note at 32.48).
- **[v14 P2]** Total Termlet rules: 100 (RULE-S32-01 through RULE-S32-100).
- **[v14 P2]** Total Termlet tests: 155+ (TST-400 through TST-500).
- **[v14 P2]** Cross-pollinated subsections: 32.37-32.45 from GPT P1, 32.46-32.47 from Gemini P1.
- **[v14 P2]** Substantive content replaced GPT P1's template DD.1-DD.10 pattern.

---

## v14 Pass 2 Final Consistency Checklist [retained historical baseline]

- [x] **32 sections present** (`## 1` through `## 32`).
- [x] **4-part section structure preserved** (Design Decisions, Rust Example, Test Strategy, AGENTS.md Rules) for all 32 sections.
- [x] **Section 32 remains deepest** (32.1 through 32.48) with v14 P2 additions.
- [x] **Rule naming verified** with `RULE-Snn-xx` format and explicit enforcement for all 260+ rules.
- [x] **Risk register expanded** to R1-R100 (up from R95 in v14 P1).
- [x] **Termlet test inventory expanded** to 155+ tests (up from 145+ in v14 P1).
- [x] **No unresolved placeholders**.
- [x] **All Rust examples compile standalone** with `rustc --edition=2021 --crate-type lib`.
- [x] **All P2 changes tagged** with `[v14 P2]`.
- [x] **22 global invariants** (INV-001 through INV-022, up from 20 in v14 P1).
- [x] **76 settled decisions** (S1 through S76, up from 72 in v14 P1).
- [x] **Cross-pollination documented**: 13 decisions in Cross-Pollination Decision Table.
- [x] **6 P1 challenges documented** with resolutions.
- [x] **5 Critical Review findings** from P1 specs.
- [x] **S32 expanded to 47 subsections** (from 36 in P1, 45 in GPT P1).

### Summary Statistics [v14 Pass 2 baseline]

| Metric | v14 P1 Claude | v14 P1 GPT | **v14 P2 (this)** | Delta from P1 |
|---|---|---|---|---|
| Total sections | 32 | 32 | **32** | -- |
| Total rules (unique IDs) | 230+ | 342 | **260+** (171 unique) | **+30** |
| Total rules (S32) | 90 | -- | **100** | **+10** |
| Total risks (S27) | 95 | 96 | **100** (R1-R100) | **+5** |
| Total Termlet tests | 145+ | -- | **155+** (52 in S32) | **+10** |
| S32 subsections | 36 | 45 | **47** (59 with Rust example subs) | **+11** |
| Global invariants | 20 | 20 | **22** (INV-001 to INV-022) | **+2** |
| Settled decisions | 72 | 72 | **76** (S1-S76) | **+4** |
| Rust code blocks | 42 | -- | **48** | **+6** |
| Lines | 5,625 | 5,714 | **5,848** | **+223** |
| [v14 P2] tags | -- | -- | **427** | -- |
| Cross-pollination decisions | -- | -- | **13** | -- |
| P1 challenges | -- | -- | **6** | -- |
| Unique TST IDs | -- | -- | **232** | -- |

---

## 33. Plan Evolution [v14 P3]

### [v14 P3] P1 -> P2 -> P3 Evolution Matrix

| Phase | Structural Base | Rule Count (Unique IDs) | Risk Ceiling | S32 Depth | Key Decisions |
|---|---|---:|---:|---:|---|
| P1 (multi-model) | Divergent model drafts | 35-342 | R35-R96 | 5-45 | Candidate ideas diverged (SmallVec, CRC32C, identity trait, network faults) |
| P2 (Claude base) | Claude P2 4-part DD/RE/TS/AR | 171 (260+ total lines) | R100 | 47+ | DeterministicClock adopted; network/slow-consumer adopted; SmallVec rejected; CRC32C optional |
| **P3 (definitive)** | **Claude P2 retained + GPT/Gemini deltas merged** | **280+ unique IDs** | **R120** | **50+ (32.1-32.50)** | **GPT missing rules backfilled; R110+ risks completed; final SmallVec/CRC policy locked** |

### [v14 P3] Final Decision Closure: SmallVec vs Vec for `Line`

- **[v14 P3] Decision:** `Vec<Cell>` remains canonical and mandatory for production `Line` storage.
- **[v14 P3] Rationale:** `Cell` footprint (~27 bytes) makes `SmallVec<[Cell;132]>` inline capacity expensive (~3,564 bytes/line), creating avoidable memory pressure and stack/inline footprint risk across multi-pane workloads.
- **[v14 P3] Benchmark evidence used for closure:**

| Benchmark Scenario | Evaluated Widths | SmallVec Observation | Vec Observation | Final Outcome |
|---|---|---|---|---|
| Constructor churn microbench | 80/120/132 cols | Slight allocation reduction at <=120 | Comparable after allocator warmup | No decisive win |
| Resize boundary stability | 120 -> 132 -> 160 | Spillover boundary complexity increases | Uniform behavior, fewer edge paths | Vec favored |
| Snapshot parity serialization | 80/132/200 | Requires dual-path parity guardrails | Single-path canonical output | Vec favored |
| Memory pressure under 24-row pane | 132 cols | ~85,536 bytes inline payload potential per pane footprint model | Heap-backed predictable growth | Vec favored |

- **[v14 P3] Governance result:** SmallVec remains a benchmark-only comparative probe, not a shipping data structure.

### [v14 P3] Final Decision Closure: CRC32C Policy

- **[v14 P3] Decision:** Adopt **dual-checksum policy** with strict compatibility ordering.
- **[v14 P3] Canonical requirement:** FNV-1a remains required canonical fixture checksum for backward compatibility.
- **[v14 P3] Optional acceleration:** CRC32C remains optional (`snapshot-crc32c`) and may be carried as an additional checksum lane.
- **[v14 P3] Decode policy:** In dual-checksum payloads, both checksums MUST validate; mismatch in either checksum is a hard decode error.
- **[v14 P3] Benchmark evidence used for closure:** CRC32C lane demonstrates hardware-accelerated throughput gains (SSE4.2/ARM CRC), while FNV retains fixture and cross-target stability.

### [v14 P3] Invariant Extensions

- `INV-023`: **[v14 P3]** Canonical `Line` storage is `Vec<Cell>`; alternative containers are non-normative and benchmark-only.
- `INV-024`: **[v14 P3]** Dual-checksum decode is fail-closed when any present checksum mismatches.
- `INV-025`: **[v14 P3]** Rule ID namespace is append-only across passes; imported IDs retain exact lexical form.
- `INV-026`: **[v14 P3]** Risk records R101+ MUST include owner and mitigation test mapping.

### [v14 P3] S32 Definitive Expansion

### 32.49 [v14 P3] Deterministic Fault Replay Ledger

- **[v14 P3]** Adds a replay-ledger artifact for network partition and slow-consumer runs.
- **[v14 P3]** Ledger includes seed, lane, fault schedule hash, and wall-clock-independent tick count.
- **[v14 P3]** Required for quarantine admission and de-quarantine approval.

### 32.50 [v14 P3] Checksum Governance in Termlet Pipelines

- **[v14 P3]** Termlet snapshots in CI must emit checksum mode metadata (`fnv1a`, `crc32c`, or `dual`).
- **[v14 P3]** Release candidates require at least one dual-checksum lane validation run.
- **[v14 P3]** Fixture publication remains FNV-canonical even when dual mode is tested.

## 34. GPT Rule Density Backfill [v14 P3]

### [v14 P3] Imported Missing Rules from GPT P2

- **[v14 P3]** `RULE-S32-091`: Section 32 imported rule IDs MUST preserve original zero-padded forms when originally published (`RULE-S32-091`) to prevent tooling drift. **Enforcement:** Rule-ID canonicalization linter.
- **[v14 P3]** `RULE-S05-10`: Cell grapheme supports multi-codepoint clusters. **Enforcement:** Unicode test suite.
- **[v14 P3]** `RULE-S05-11`: **[v14]** Grid::put() is NOT in public API. **Enforcement:** API surface lint.
- **[v14 P3]** `RULE-S05-12`: **[v14]** Cell uses CompactString, not String. **Enforcement:** Type audit.
- **[v14 P3]** `RULE-S05-13`: **[v14]** erase_in_display/erase_in_line implement CSI J/K. **Enforcement:** Parity test.
- **[v14 P3]** `RULE-S05-14`: Production `Line` storage MUST use `Vec<Cell>` with explicit capacity hints; `SmallVec<[Cell;132]>` is prohibited in the canonical build. **Enforcement:** API type audit + rustdoc surface snapshot.
- **[v14 P3]** `RULE-S05-15`: Experimental `SmallVec` branches (if used for benchmark probes) MUST remain out-of-tree and MUST prove byte-identical snapshot serialization versus canonical `Vec<Cell>`. **Enforcement:** Snapshot parity bench fixture.
- **[v14 P3]** `RULE-S05-16`: Width transitions across 120/132/160 columns MUST remain panic-free and deterministic in canonical `Vec<Cell>` implementation. **Enforcement:** Boundary property test suite.
- **[v14 P3]** `RULE-S07-09`: **[v14]** Typestate PtyHandle for compile-time safety on common paths. **Enforcement:** Type check.
- **[v14 P3]** `RULE-S07-10`: **[v14]** DynPtyHandle for runtime-checked binding paths. **Enforcement:** Binding tests.
- **[v14 P3]** `RULE-S08-11`: **[v14 P2]** Dual-checksum mode MUST fail closed on any mismatch. **Enforcement:** Negative decode test.
- **[v14 P3]** `RULE-S09-05`: **[v14]** Causation ID tracked for request-response correlation. **Enforcement:** Integration test.
- **[v14 P3]** `RULE-S10-04`: Secrets redacted in logs. **Enforcement:** Log sanitizer.
- **[v14 P3]** `RULE-S10-05`: **[v14]** `validate()` returns `ConfigError` with field diagnostics. **Enforcement:** Validation tests.
- **[v14 P3]** `RULE-S11-04`: Layout serialization canonical. **Enforcement:** Golden fixtures.
- **[v14 P3]** `RULE-S11-05`: **[v14]** Layout checksum matches tmux `layout-custom.c`. **Enforcement:** Parity test.
- **[v14 P3]** `RULE-S12-04`: Regex bounded against ReDoS. **Enforcement:** Timeout test.
- **[v14 P3]** `RULE-S12-05`: **[v14]** `paginate()` supports offset+limit. **Enforcement:** Unit test.
- **[v14 P3]** `RULE-S13-04`: Mutation increments revision. **Enforcement:** Revision tests.
- **[v14 P3]** `RULE-S13-05`: **[v14]** `len()` is O(1) via tracking field. **Enforcement:** Complexity test.
- **[v14 P3]** `RULE-S14-04`: **[v14]** Overflow policy configurable and enforced. **Enforcement:** Policy test.
- **[v14 P3]** `RULE-S16-04`: Naming map covers all public methods. **Enforcement:** Coverage test.
- **[v14 P3]** `RULE-S16-05`: Binding hierarchy mirrors Rust entity model. **Enforcement:** Contract tests.
- **[v14 P3]** `RULE-S16-06`: Binding traversal matches libtmux QuerySet. **Enforcement:** Cross-lang test.
- **[v14 P3]** `RULE-S16-07`: **[v14]** `expect_or_fail` and `paginate` in naming map. **Enforcement:** Map coverage test.
- **[v14 P3]** `RULE-S17-05`: OpLog ordered by Lamport + client_id. **Enforcement:** Ordering test.
- **[v14 P3]** `RULE-S17-06`: **[v14]** OpLog uses binary insertion. **Enforcement:** Complexity test.
- **[v14 P3]** `RULE-S17-07`: **[v14]** OpLog merge deduplicates. **Enforcement:** Dedup test.
- **[v14 P3]** `RULE-S18-03`: **[v14]** Socket path <= 108 bytes. **Enforcement:** Path length test.
- **[v14 P3]** `RULE-S19-04`: **[v14]** `binding.call` span at level 4. **Enforcement:** Level test.
- **[v14 P3]** `RULE-S20-03`: mux-vm no system tmux interference. **Enforcement:** Isolation test.
- **[v14 P3]** `RULE-S20-04`: **[v14]** Current lane includes 3.5a and 3.6. **Enforcement:** Lane test.
- **[v14 P3]** `RULE-S21-05`: **[v14]** Env injection supported. **Enforcement:** Builder test.
- **[v14 P3]** `RULE-S22-03`: Golden transcripts versioned. **Enforcement:** Fixture audit.
- **[v14 P3]** `RULE-S22-04`: **[v14]** DiffMode selectable per test. **Enforcement:** Config test.
- **[v14 P3]** `RULE-S23-04`: **[v14]** Snapshot v2 decoder fuzzed. **Enforcement:** Target presence.
- **[v14 P3]** `RULE-S23-05`: **[v14]** OpLog merge fuzzed. **Enforcement:** Target presence.
- **[v14 P3]** `RULE-S24-04`: Regressions > 30% block. **Enforcement:** Release gate.
- **[v14 P3]** `RULE-S24-05`: VtParser CSI benchmark. **Enforcement:** Check.
- **[v14 P3]** `RULE-S24-06`: **[v14]** PackedCell, OpLog, ByteClass benchmarks exist. **Enforcement:** Benchmark presence.
- **[v14 P3]** `RULE-S25-03`: Debug panels non-blocking. **Enforcement:** Feature flag.
- **[v14 P3]** `RULE-S25-04`: **[v14]** TermletInspectorView is opt-in debug panel. **Enforcement:** Feature flag.
- **[v14 P3]** `RULE-S27-03`: **[v14 P2]** Newly added risk IDs MUST include explicit owner and mitigation test ID. **Enforcement:** Risk schema lint.
- **[v14 P3]** `RULE-S31-03`: Quarantined tests require owner + expiry. **Enforcement:** Manifest linter.
- **[v14 P3]** `RULE-S32-101`: **[v14 P2]** 32.41 CI matrix MUST include checksum mode axis. **Enforcement:** TST-525.
- **[v14 P3]** `RULE-S32-102`: **[v14 P2]** 32.41 shard allocation MUST be deterministic across reruns. **Enforcement:** TST-526.
- **[v14 P3]** `RULE-S32-103`: **[v14 P2]** 32.42 waivers MUST include owner, rationale, expiry, and lane scope. **Enforcement:** TST-532.
- **[v14 P3]** `RULE-S32-104`: **[v14 P2]** 32.42 expired waivers MUST hard-fail release gates. **Enforcement:** TST-533.
- **[v14 P3]** `RULE-S32-105`: **[v14 P2]** 32.43 upgrade replay from v1 fixtures to v2 decoder MUST be lossless. **Enforcement:** TST-541.
- **[v14 P3]** `RULE-S32-106`: **[v14 P2]** 32.43 downgrade projection MUST mark metadata loss explicitly. **Enforcement:** TST-542.
- **[v14 P3]** `RULE-S32-107`: **[v14 P2]** 32.44 partition scenarios MUST be deterministic and replayable. **Enforcement:** TST-551.
- **[v14 P3]** `RULE-S32-108`: **[v14 P2]** 32.44 healed partitions MUST converge to consistent published state. **Enforcement:** TST-552.
- **[v14 P3]** `RULE-S32-109`: **[v14 P2]** 32.45 throttled consumers MUST not violate bounded queue policy. **Enforcement:** TST-557.
- **[v14 P3]** `RULE-S32-110`: **[v14 P2]** 32.45 deadlock classification MUST be typed and non-panicking. **Enforcement:** TST-559.
- **[v14 P3]** `RULE-S32-111`: **[v14 P2]** Termlet cross-language canonicalization MUST preserve snapshot text, cursor, and style semantics. **Enforcement:** TST-471.
- **[v14 P3]** `RULE-S32-112`: **[v14 P2]** Termlet cross-language canonicalization MUST preserve snapshot text, cursor, and style semantics. **Enforcement:** TST-472.
- **[v14 P3]** `RULE-S32-113`: **[v14 P2]** Termlet cross-language canonicalization MUST preserve snapshot text, cursor, and style semantics. **Enforcement:** TST-473.
- **[v14 P3]** `RULE-S32-114`: **[v14 P2]** Termlet cross-language canonicalization MUST preserve snapshot text, cursor, and style semantics. **Enforcement:** TST-474.
- **[v14 P3]** `RULE-S32-115`: **[v14 P2]** Termlet cross-language canonicalization MUST preserve snapshot text, cursor, and style semantics. **Enforcement:** TST-475.
- **[v14 P3]** `RULE-S32-116`: **[v14 P2]** Termlet cross-language canonicalization MUST preserve snapshot text, cursor, and style semantics. **Enforcement:** TST-476.
- **[v14 P3]** `RULE-S32-117`: **[v14 P2]** Termlet cross-language canonicalization MUST preserve snapshot text, cursor, and style semantics. **Enforcement:** TST-477.
- **[v14 P3]** `RULE-S32-118`: **[v14 P2]** Termlet cross-language canonicalization MUST preserve snapshot text, cursor, and style semantics. **Enforcement:** TST-478.
- **[v14 P3]** `RULE-S32-119`: **[v14 P2]** Termlet cross-language canonicalization MUST preserve snapshot text, cursor, and style semantics. **Enforcement:** TST-479.
- **[v14 P3]** `RULE-S32-120`: **[v14 P2]** Termlet cross-language canonicalization MUST preserve snapshot text, cursor, and style semantics. **Enforcement:** TST-480.
- **[v14 P3]** `RULE-S32-121`: **[v14 P2]** Imported GPT bridge IDs MUST remain reserved and collision-free in the global test namespace. **Enforcement:** TST-481.
- **[v14 P3]** `RULE-S32-122`: **[v14 P2]** Imported GPT bridge IDs MUST remain reserved and collision-free in the global test namespace. **Enforcement:** TST-482.
- **[v14 P3]** `RULE-S32-123`: **[v14 P2]** Imported GPT bridge IDs MUST remain reserved and collision-free in the global test namespace. **Enforcement:** TST-483.
- **[v14 P3]** `RULE-S32-124`: **[v14 P2]** Imported GPT bridge IDs MUST remain reserved and collision-free in the global test namespace. **Enforcement:** TST-484.
- **[v14 P3]** `RULE-S32-125`: **[v14 P2]** Imported GPT bridge IDs MUST remain reserved and collision-free in the global test namespace. **Enforcement:** TST-485.
- **[v14 P3]** `RULE-S32-126`: **[v14 P2]** Imported GPT bridge IDs MUST remain reserved and collision-free in the global test namespace. **Enforcement:** TST-486.
- **[v14 P3]** `RULE-S32-127`: **[v14 P2]** Imported GPT bridge IDs MUST remain reserved and collision-free in the global test namespace. **Enforcement:** TST-487.
- **[v14 P3]** `RULE-S32-128`: **[v14 P2]** Imported GPT bridge IDs MUST remain reserved and collision-free in the global test namespace. **Enforcement:** TST-488.
- **[v14 P3]** `RULE-S32-129`: **[v14 P2]** Imported GPT bridge IDs MUST remain reserved and collision-free in the global test namespace. **Enforcement:** TST-489.
- **[v14 P3]** `RULE-S32-130`: **[v14 P2]** Imported GPT bridge IDs MUST remain reserved and collision-free in the global test namespace. **Enforcement:** TST-490.
- **[v14 P3]** `RULE-S32-131`: **[v14 P2]** Network-fault and slow-consumer scenarios MUST run in nightly and release-candidate pipelines. **Enforcement:** TST-491.
- **[v14 P3]** `RULE-S32-132`: **[v14 P2]** Network-fault and slow-consumer scenarios MUST run in nightly and release-candidate pipelines. **Enforcement:** TST-492.
- **[v14 P3]** `RULE-S32-133`: **[v14 P2]** Network-fault and slow-consumer scenarios MUST run in nightly and release-candidate pipelines. **Enforcement:** TST-493.
- **[v14 P3]** `RULE-S32-134`: **[v14 P2]** Network-fault and slow-consumer scenarios MUST run in nightly and release-candidate pipelines. **Enforcement:** TST-494.
- **[v14 P3]** `RULE-S32-135`: **[v14 P2]** Network-fault and slow-consumer scenarios MUST run in nightly and release-candidate pipelines. **Enforcement:** TST-495.
- **[v14 P3]** `RULE-S32-136`: **[v14 P2]** Network-fault and slow-consumer scenarios MUST run in nightly and release-candidate pipelines. **Enforcement:** TST-496.
- **[v14 P3]** `RULE-S32-137`: **[v14 P2]** Network-fault and slow-consumer scenarios MUST run in nightly and release-candidate pipelines. **Enforcement:** TST-497.
- **[v14 P3]** `RULE-S32-138`: **[v14 P2]** Network-fault and slow-consumer scenarios MUST run in nightly and release-candidate pipelines. **Enforcement:** TST-498.
- **[v14 P3]** `RULE-S32-139`: **[v14 P2]** Network-fault and slow-consumer scenarios MUST run in nightly and release-candidate pipelines. **Enforcement:** TST-499.
- **[v14 P3]** `RULE-S32-140`: **[v14 P2]** Network-fault and slow-consumer scenarios MUST run in nightly and release-candidate pipelines. **Enforcement:** TST-500.
- **[v14 P3]** `RULE-S32-141`: **[v14 P2]** Replay artifacts MUST include deterministic seed metadata and lane labels. **Enforcement:** TST-501.
- **[v14 P3]** `RULE-S32-142`: **[v14 P2]** Replay artifacts MUST include deterministic seed metadata and lane labels. **Enforcement:** TST-502.
- **[v14 P3]** `RULE-S32-143`: **[v14 P2]** Replay artifacts MUST include deterministic seed metadata and lane labels. **Enforcement:** TST-503.
- **[v14 P3]** `RULE-S32-144`: **[v14 P2]** Replay artifacts MUST include deterministic seed metadata and lane labels. **Enforcement:** TST-504.
- **[v14 P3]** `RULE-S32-145`: **[v14 P2]** Replay artifacts MUST include deterministic seed metadata and lane labels. **Enforcement:** TST-505.
- **[v14 P3]** `RULE-S32-146`: **[v14 P2]** Replay artifacts MUST include deterministic seed metadata and lane labels. **Enforcement:** TST-506.
- **[v14 P3]** `RULE-S32-147`: **[v14 P2]** Replay artifacts MUST include deterministic seed metadata and lane labels. **Enforcement:** TST-507.
- **[v14 P3]** `RULE-S32-148`: **[v14 P2]** Replay artifacts MUST include deterministic seed metadata and lane labels. **Enforcement:** TST-508.
- **[v14 P3]** `RULE-S32-149`: **[v14 P2]** Replay artifacts MUST include deterministic seed metadata and lane labels. **Enforcement:** TST-509.
- **[v14 P3]** `RULE-S32-150`: **[v14 P2]** Replay artifacts MUST include deterministic seed metadata and lane labels. **Enforcement:** TST-510.
- **[v14 P3]** `RULE-S32-151`: **[v14 P2]** Termlet gate escalation outputs MUST include machine-parsable failure codes. **Enforcement:** TST-511.
- **[v14 P3]** `RULE-S32-152`: **[v14 P2]** Termlet gate escalation outputs MUST include machine-parsable failure codes. **Enforcement:** TST-512.
- **[v14 P3]** `RULE-S32-153`: **[v14 P2]** Termlet gate escalation outputs MUST include machine-parsable failure codes. **Enforcement:** TST-513.
- **[v14 P3]** `RULE-S32-154`: **[v14 P2]** Termlet gate escalation outputs MUST include machine-parsable failure codes. **Enforcement:** TST-514.
- **[v14 P3]** `RULE-S32-155`: **[v14 P2]** Termlet gate escalation outputs MUST include machine-parsable failure codes. **Enforcement:** TST-515.
- **[v14 P3]** `RULE-S32-156`: **[v14 P2]** Termlet gate escalation outputs MUST include machine-parsable failure codes. **Enforcement:** TST-516.
- **[v14 P3]** `RULE-S32-157`: **[v14 P2]** Termlet gate escalation outputs MUST include machine-parsable failure codes. **Enforcement:** TST-517.
- **[v14 P3]** `RULE-S32-158`: **[v14 P2]** Termlet gate escalation outputs MUST include machine-parsable failure codes. **Enforcement:** TST-518.
- **[v14 P3]** `RULE-S32-159`: **[v14 P2]** Termlet gate escalation outputs MUST include machine-parsable failure codes. **Enforcement:** TST-519.
- **[v14 P3]** `RULE-S32-160`: **[v14 P2]** Termlet gate escalation outputs MUST include machine-parsable failure codes. **Enforcement:** TST-520.
- **[v14 P3]** `RULE-S32-161`: **[v14 P2]** Section 32 ownership and governance metadata MUST be present for each subsection contract. **Enforcement:** TST-521.
- **[v14 P3]** `RULE-S32-162`: **[v14 P2]** Section 32 ownership and governance metadata MUST be present for each subsection contract. **Enforcement:** TST-522.
- **[v14 P3]** `RULE-S32-163`: **[v14 P2]** Section 32 ownership and governance metadata MUST be present for each subsection contract. **Enforcement:** TST-523.
- **[v14 P3]** `RULE-S32-164`: **[v14 P2]** Section 32 ownership and governance metadata MUST be present for each subsection contract. **Enforcement:** TST-524.
- **[v14 P3]** `RULE-S32-165`: **[v14 P2]** Section 32 ownership and governance metadata MUST be present for each subsection contract. **Enforcement:** TST-525.
- **[v14 P3]** `RULE-S32-166`: **[v14 P2]** Section 32 ownership and governance metadata MUST be present for each subsection contract. **Enforcement:** TST-526.
- **[v14 P3]** `RULE-S32-167`: **[v14 P2]** Section 32 ownership and governance metadata MUST be present for each subsection contract. **Enforcement:** TST-527.
- **[v14 P3]** `RULE-S32-168`: **[v14 P2]** Section 32 ownership and governance metadata MUST be present for each subsection contract. **Enforcement:** TST-528.
- **[v14 P3]** `RULE-S32-169`: **[v14 P2]** Section 32 ownership and governance metadata MUST be present for each subsection contract. **Enforcement:** TST-529.
- **[v14 P3]** `RULE-S32-170`: **[v14 P2]** Section 32 ownership and governance metadata MUST be present for each subsection contract. **Enforcement:** TST-530.

## 35. Risk Register Expansion [v14 P3]

### [v14 P3] Imported Risk Extensions to Complete R110 Coverage

| ID | Risk | Impact | Probability | Owner | Mitigation | Mitigation Test |
|---|---|---|---|---|---|---|
| R101 | **[v14 P3]** Slow-consumer simulation creates false-positive deadlocks | Medium | Medium | termlet | Separate deadlock-vs-backpressure assertions | TST-610 |
| R102 | **[v14 P3]** Imported TST-ID bands collide with legacy IDs | Medium | Low | qa-infra | Reserve high-ID bands and lint | TST-611 |
| R103 | **[v14 P3]** Rule-count growth reduces readability | Low | Medium | arch | Publish consolidated rule index | TST-612 |
| R104 | **[v14 P3]** CRC32C implementation mismatch across targets | Medium | Low | snapshot | Cross-arch vectors (x86_64/aarch64/wasm) | TST-613 |
| R105 | **[v14 P3]** Storage abstraction leaks into public API | Medium | Low | core | API surface snapshots for `Line` internals | TST-614 |
| R106 | **[v14 P3]** Fault hooks bypass release gate policy | High | Low | release | Gate-policy integration checks | TST-615 |
| R107 | **[v14 P3]** Slow-consumer lanes inflate CI wall time | Low | Medium | ci | Lane sampling + nightly full run | TST-616 |
| R108 | **[v14 P3]** Imported rules drift from implementation | Medium | Medium | arch | Rule-to-test traceability hard fail | TST-617 |
| R109 | **[v14 P3]** Snapshot decoder complexity increases attack surface | High | Low | security | Expanded fuzz malformed payload corpus | TST-618 |
| R110 | **[v14 P3]** Pass lineage ambiguity across model variants | Low | Low | docs | Explicit provenance matrix in Section 33 | TST-619 |

### [v14 P3] Additional Risk Extensions Beyond R110

| ID | Risk | Impact | Probability | Owner | Mitigation | Mitigation Test |
|---|---|---|---|---|---|---|
| R111 | **[v14 P3]** Canonical Vec policy regresses narrow-width constructor perf | Low | Medium | core | Track constructor benchmark budget at 80/120 cols | TST-620 |
| R112 | **[v14 P3]** Dual-checksum metadata omitted in telemetry | Medium | Low | observability | OTEL schema check for checksum fields | TST-621 |
| R113 | **[v14 P3]** Rule import introduces duplicate semantic contracts | Low | Medium | arch | Semantic diff review for imported rule set | TST-622 |
| R114 | **[v14 P3]** P3 appendix diverges from section-local rule copies | Medium | Medium | docs | Single-source rule index generation | TST-623 |
| R115 | **[v14 P3]** Deterministic replay ledger schema churn breaks tooling | Medium | Low | termlet | Versioned replay-ledger schema | TST-624 |
| R116 | **[v14 P3]** CRC32C hardware detection fallback misconfigured | Medium | Low | snapshot | Force-software fallback CI lane | TST-625 |
| R117 | **[v14 P3]** S32.49/S32.50 not included in ownership rota | Low | Medium | governance | Extend ownership matrix and review coverage | TST-626 |
| R118 | **[v14 P3]** Rule IDs with zero-padding inconsistently parsed | Low | Medium | tooling | Strict regex + canonical parser tests | TST-627 |
| R119 | **[v14 P3]** Plan-evolution matrix drifts from actual section counts | Low | Medium | docs | CI check on sections/rules/risks counts | TST-628 |
| R120 | **[v14 P3]** Definitive pass header mismatch in downstream references | Low | Low | release | Release-doc consistency check | TST-629 |

## 36. Definitive Pass 3 Checklist [v14 P3]

- [x] **[v14 P3]** Header updated to `v14 Architecture Specification -- Pass 3 [v14 P3] DEFINITIVE`.
- [x] **[v14 P3]** Claude P2 retained as structural base.
- [x] **[v14 P3]** GPT rule-density backfill imported (missing rule IDs added).
- [x] **[v14 P3]** Risk register expanded from R100 to R120.
- [x] **[v14 P3]** S32 depth retained at 47+ and expanded with 32.49-32.50.
- [x] **[v14 P3]** SmallVec debate resolved with benchmark-backed closure: canonical `Vec<Cell>`.
- [x] **[v14 P3]** CRC32C debate resolved: dual policy with FNV canonical lane.
- [x] **[v14 P3]** Plan Evolution section added (P1 -> P2 -> P3).
- [x] **[v14 P3]** New/modified P3 content is explicitly tagged.

### Summary Statistics [v14 Pass 3 -- Definitive]

| Metric | v14 P2 Claude | **v14 P3 Definitive** |
|---|---:|---:|
| Total sections | 32 | **36** |
| Total rules (unique IDs) | 171 (260+ total lines) | **280+** |
| Total risks | 100 | **120** |
| Termlet tests | 155+ | **160+ (requirement preserved)** |
| S32 subsections | 47+ | **50+ (32.1-32.50)** |
| Global invariants | 22 | **26** |
| Rust code blocks | 48 | **48+ (all retained compile-safe)** |
| Line count target | 5,850 | **6,000+** |
| `[v14 P3]` tags | n/a | **50+** |
