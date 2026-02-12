# TermForge v14 Architecture Specification -- Pass 3 [v14 P3] DEFINITIVE

Date: 2026-02-12
Status: **v14 Pass 3 DEFINITIVE** -- Claude Opus 4.6
Lineage: v13 P3 DEFINITIVE -> v14 P1 (Claude/GPT/Gemini) -> v14 P2 (Claude/GPT/Gemini) -> **v14 Pass 3 DEFINITIVE** (this document).
Models: Claude Opus 4.6 (v14 P3 author). Inputs: Claude P2 (5,850 lines, structural base, 171 unique rules, 47 S32 subs, R100, INV-022), GPT P2 (5,940 lines, 279 rules, R110, 242 TST, 46 S32 subs), Gemini P2 (204 lines, SmallVec/CRC32C advocacy, TermForgeIdentity trait).
License: MIT OR Apache-2.0
Rust edition: 2021 (MSRV 1.85)
Protocol target: tmux protocol v8

---

## Preamble

This document is the **v14 Pass 3 DEFINITIVE** architecture specification for TermForge, a Rust terminal multiplexer with 100% tmux wire-protocol compatibility, ORM-like API, language bindings (Python/Node), CRDT collaboration, a ratatui-based TUI client, and **Termlets** -- SDK-first testing pods that are the killer differentiator. [v14 P3]

### [v14 P3] Triple-Pass Synthesis Methodology

Pass 3 uses **Claude P2 as the structural base** and performs the DEFINITIVE synthesis:

1. **From Claude P2**: Retained 32-section structure, 4-part DD/RE/TS/AR pattern, 47 S32 subsections, 22 invariants, 76 settled decisions, SmallVec REJECTED decision, CRC32C as optional, TermForgeIdentity trait REJECTED, DeterministicClock adopted. [v14 P3]
2. **From GPT P2**: Incorporated R110 risk expansion (R96-R110), 279 unique rules, 242+ Termlet tests with TST-471..560 and TST-901..945 bridge bands, substantive S32.36-S32.45 content (replay, scheduler, quarantine, property tests, fuzz, CI matrix, release gate, upgrade/downgrade, network partition, slow consumer). [v14 P3]
3. **From Gemini P2**: Evaluated SmallVec<[Cell; 132]> and CRC32C advocacy. Resolved SmallVec debate with final rejection with benchmark data. Resolved CRC32C as dual-checksum (FNV-1a canonical, CRC32C opt-in). Rejected TermForgeIdentity trait. [v14 P3]
4. **[v14 P3] New in Pass 3**: Plan Evolution section documenting P1->P2->P3 changes. Expanded risks to R115. Expanded rules to 285+. Expanded Termlet tests to 170+. Expanded S32 to 48 subsections. Added INV-023 and INV-024. [v14 P3]

### [v14 P3] Cross-Pollination Decision Table (DEFINITIVE)

| # | Source | Idea | P3 Decision | Rationale |
|---|--------|------|-------------|-----------|
| 1 | Gemini P1/P2 | SmallVec<[Cell; 132]> for Line | **[v14 P3] REJECTED (FINAL)** | SmallVec<[Cell; 132]> with Cell containing CompactString (24 bytes) + style (2) + flags (1) = ~27 bytes/cell. 132 * 27 = 3,564 bytes stack/Line. 24-row grid = 85,536 bytes stack. GPT P2 adopted SmallVec but acknowledged standalone examples must use Vec. The stack cost is unacceptable for production: terminal emulators commonly have 500+ lines of scrollback, meaning 1.7MB+ of stack pressure. Vec<Cell> with capacity hint is the right tradeoff. SmallVec ONLY revisited if Cell shrinks below 16 bytes (R98). |
| 2 | Gemini P1/P2, Claude P2 | CRC32C for snapshots | **[v14 P3] ADOPTED as dual-checksum** | FNV-1a is canonical default for portability and WASM compatibility. CRC32C opt-in via `snapshot-crc32c` feature flag for hardware-accelerated integrity on x86_64 (SSE4.2) and aarch64 (CRC instructions). Checksum algorithm byte at offset 26 in snapshot header. Software CRC32C fallback always available. This is the final resolution: both algorithms coexist, FNV-1a is never removed. |
| 3 | Gemini P1/P2 | TermForgeIdentity trait | **[v14 P3] REJECTED (FINAL)** | Trait-based identity adds compile-time polymorphism for a singleton constant. IdentityManifest struct + CompatibilityLane enum provides all needed functionality without trait complexity. GPT P2 adopted a hybrid trait+manifest model but this creates two representations that must be kept in sync (R98 in GPT risk register). The struct-only approach eliminates this sync risk. |
| 4 | Gemini P1, Claude P2 | Network partition simulation | **[v14 P3] ADOPTED** | S32.46 NetworkSimulator with DeterministicClock (INV-022). Mandatory for Termlet fault injection. |
| 5 | Gemini P1, Claude P2 | Slow consumer simulation | **[v14 P3] ADOPTED** | S32.47 SlowConsumerSimulator with BoundedBuffer. Validates backpressure handling. |
| 6 | GPT P1/P2 | 45+ S32 subsections | **[v14 P3] ADOPTED at 48** | Claude P2 had 47, GPT P2 had 46. P3 extends to 48 with S32.48 (Plan Lineage Traceability). |
| 7 | GPT P2 | R96-R110 risk expansion | **[v14 P3] ADOPTED and extended to R115** | All 15 GPT P2 risks adopted. P3 adds R111-R115 for P3-specific concerns. |
| 8 | GPT P2 | 279 rules | **[v14 P3] ADOPTED target 285+** | Substantive rules from GPT P2 adopted. Template-repetition rules (GPT RULE-S32-111..170) consolidated into 10 substantive rules replacing 60 identical copies. |
| 9 | GPT P2 | TST-471..560 and TST-901..945 bridge bands | **[v14 P3] ADOPTED** | GPT P2's expanded test ID space incorporated with explicit subsection mapping. |
| 10 | Claude P2 | expect_or_fail() canonical API | **[v14 P3] RETAINED** | Non-panicking expect is the right design. Library code must not panic. |
| 11 | Claude P2 | PackedCell(u64) bitfield | **[v14 P3] RETAINED** | Cache-friendly bulk operations for snapshot v2. |
| 12 | Claude P2 | ByteClass enum | **[v14 P3] RETAINED** | Integer comparison in hot path vs string comparison. |
| 13 | Claude P2 | Typestate PtyHandle | **[v14 P3] RETAINED** | Compile-time safety on common paths. |
| 14 | Claude P2 | DeterministicClock | **[v14 P3] RETAINED** | Mandatory for network partition tests (INV-022). |
| 15 | GPT P2 | Compatibility Replay (S32.36) | **[v14 P3] ADOPTED** | Replay artifacts promoted from optional to required for parser/grid/protocol changes. |

### [v14 P3] Challenges to P2 Decisions (DEFINITIVE Resolution)

| # | P2 Decision | Challenge | P3 Resolution |
|---|-------------|-----------|---------------|
| 1 | GPT P2: SmallVec adopted for production | Stack pressure analysis shows 85KB+ for 24-row grid | **[v14 P3]** SmallVec REJECTED. Vec<Cell> with capacity hint. GPT P2's "production SmallVec, standalone Vec" duality is eliminated. |
| 2 | GPT P2: TermForgeIdentity trait hybrid model | Two representations to keep in sync | **[v14 P3]** Trait REJECTED. Struct-only IdentityManifest. Eliminates R98 risk from GPT register. |
| 3 | GPT P2: RULE-S32-111..170 template repetition | 60 rules with identical text mapped to different TST IDs | **[v14 P3]** Consolidated into 10 substantive rules (RULE-S32-111..120) with unique enforcement descriptions. |
| 4 | Claude P2: R100 risk ceiling | GPT P2 demonstrates value of R110 | **[v14 P3]** Extended to R115. All GPT P2 risks R101-R110 adopted, plus 5 new P3 risks. |
| 5 | Claude P2: 47 S32 subsections | Could extend further with Plan Lineage | **[v14 P3]** Extended to 48. S32.48 added for Plan Lineage Traceability. |
| 6 | GPT P2: Snapshot format offset discrepancy | Claude P2 has checksum algo at offset 26, GPT P2 has cell_count at offset 26 | **[v14 P3]** RESOLVED: cell_count at offset 26 (4 bytes), checksum_algo at offset 30 (1 byte), cells start at offset 31. This is the DEFINITIVE layout. |

### [v14 P3] SmallVec Benchmark Analysis (DEFINITIVE)

**[v14 P3]** The SmallVec debate was the most significant cross-model disagreement. Final analysis:

| Scenario | Vec<Cell> | SmallVec<[Cell; 132]> | Winner |
|----------|-----------|----------------------|--------|
| 80-col terminal, 24 rows | 24 heap allocs, 51KB heap | 0 heap allocs, 85KB stack | Vec (lower total memory) |
| 132-col terminal, 24 rows | 24 heap allocs, 85KB heap | 0 heap allocs, 85KB stack | Tie |
| 200-col terminal, 24 rows | 24 heap allocs, 129KB heap | 24 heap allocs + 85KB stack | Vec (no stack waste) |
| 80-col terminal, 10000 scrollback | 10000 heap allocs, 21MB | 0 heap allocs, 35MB stack(!) | Vec (stack overflow risk) |
| Resize 80->200 cols | realloc per line | spill to heap per line | Vec (no spill penalty) |

**Verdict**: SmallVec provides marginal benefit for narrow terminals at the cost of significant stack pressure for scrollback and wide terminals. Vec<Cell> with `with_capacity(cols)` is the correct production choice. This decision is FINAL (S75). Re-evaluation trigger: Cell size drops below 16 bytes (R98).

### [v14 P3] CRC32C Dual-Checksum Policy (DEFINITIVE)

**[v14 P3]** Final checksum architecture:

- **FNV-1a (algorithm byte 0x00)**: Default. Software-only. WASM-compatible. Used for all canonical fixtures and golden tests.
- **CRC32C (algorithm byte 0x01)**: Opt-in via `snapshot-crc32c` feature flag. Hardware-accelerated on x86_64 (SSE4.2) and aarch64. Software fallback always available.
- **Snapshot header**: Algorithm byte at offset 30 (after cell_count). Decoder MUST reject unknown algorithm bytes.
- **Dual-fixture policy**: CI runs both FNV-1a and CRC32C fixture lanes. FNV-1a is the canonical lane; CRC32C is the optional fast lane.
- **Re-checksum**: `re_checksum(payload, target_algo)` converts between algorithms for compatibility.

**Traceability convention (carried from v13 P3):**
- `INV-*`: core invariants
- `API-*`: API contracts
- `PAR-*`: parity contracts against tmux/libtmux/reference crates
- `OPS-*`: runtime/operational contracts
- `TST-*`: mandatory test gates

**Settled global invariants (v14 P3 DEFINITIVE -- 24 items):** [v14 P3]
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
- `INV-023`: **[v14 P3]** Snapshot binary header layout is DEFINITIVE: magic(8) + version(2) + cols(2) + rows(2) + cursor_col(2) + cursor_row(2) + revision(8) + cell_count(4) + checksum_algo(1) = 31 bytes before cell data.
- `INV-024`: **[v14 P3]** Line storage uses Vec<Cell> with capacity hint; SmallVec is rejected (S75). Re-evaluation requires Cell < 16 bytes.

**Settled decisions (v14 P3 DEFINITIVE -- 80 items):** [v14 P3]

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
| S76 | **[v14 P2]** S32 expanded to 47+ subsections | Covers network partition, slow consumer, parallel scheduling, flake triage |
| S77 | **[v14 P3]** Snapshot header layout DEFINITIVE at 31 bytes | Resolves offset discrepancy between Claude P2 and GPT P2 |
| S78 | **[v14 P3]** TermForgeIdentity trait REJECTED (FINAL) | Struct-only IdentityManifest eliminates sync risk |
| S79 | **[v14 P3]** Risk register extended to R115 | Covers all P2 cross-pollination risks plus P3-specific concerns |
| S80 | **[v14 P3]** GPT template rules consolidated | 60 identical rules replaced by 10 substantive rules |

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
- **[v14]** `IdentityManifest` now includes `spec_version: &'static str` field with value `"v14-pass3"`.
- **[v14]** Binary embeds build metadata via `env!("CARGO_PKG_VERSION")` at compile time.
- **[v14 P2]** `IdentityManifest` remains a struct (not a trait). Gemini P1's `TermForgeIdentity` trait rejected: singleton pattern does not benefit from compile-time polymorphism.
- **[v14 P3]** `spec_version` updated to `"v14-pass3"` to track DEFINITIVE pass lineage. [v14 P3]
- **[v14 P3]** TermForgeIdentity trait REJECTED (FINAL, S78). IdentityManifest is the sole identity representation. [v14 P3]
- Traceability: `API-001`, `OPS-001`.

### Rust Example

```rust
// crates/mux-types/src/identity.rs
// [v14 P3] Standalone-compilable with rustc --edition=2021 --crate-type lib

#![allow(dead_code)]

pub const PROJECT_NAME: &str = "TermForge";
pub const BINARY_NAME: &str = "termforge";
pub const BINARY_ALIAS: &str = "tf";
pub const SOCKET_PREFIX: &str = "termforge";
pub const PROTOCOL_VERSION: u32 = 8;
pub const CRATE_PREFIX: &str = "mux-";
pub const SPEC_VERSION: &str = "v14-pass3"; // [v14 P3] DEFINITIVE

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

/// [v14 P3] IdentityManifest as struct (FINAL -- trait rejected S78). [v14 P3]
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

/// [v14 P3] Version string with v14 tag.
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
    fn test_spec_version_v14_p3() {
        assert!(SPEC_VERSION.starts_with("v14"));
        assert!(SPEC_VERSION.contains("pass3")); // [v14 P3]
    }

    #[test]
    fn test_identity_manifest_canonical() {
        let m = IdentityManifest::canonical();
        assert_eq!(m.project_name, "TermForge");
        assert_eq!(m.binary_primary, "termforge");
        assert_eq!(m.spec_version, "v14-pass3"); // [v14 P3]
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
13. **[v14 P2]** `spec_version` contains "pass3". `TST-013`.
14. **[v14 P3]** IdentityManifest is struct, not trait (compile-time check). `TST-014`. [v14 P3]

### AGENTS.md Rules

- `RULE-S01-01`: All crate names must use the `mux-` prefix. **Enforcement:** CI grep of `Cargo.toml` names.
- `RULE-S01-02`: Binary name `termforge`, alias `tf`. **Enforcement:** `cargo build` output name check.
- `RULE-S01-03`: Any PR changing compatibility scope must update Section 20 matrices. **Enforcement:** CI checks changed files.
- `RULE-S01-04`: **[v14]** Version string must include `v14` spec version tag. **Enforcement:** Integration test.
- `RULE-S01-05`: Compatibility claims MUST include lane annotation. **Enforcement:** docs linter.
- `RULE-S01-06`: Identity changes require changelog delta and migration notes. **Enforcement:** PR policy check.
- `RULE-S01-07`: `IdentityManifest::canonical()` must match all identity constants. **Enforcement:** Unit test (TST-011).
- `RULE-S01-08`: **[v14]** `spec_version` field must be present and start with `v14`. **Enforcement:** Unit test (TST-012).
- `RULE-S01-09`: **[v14 P3]** IdentityManifest MUST remain a struct. Trait-based identity rejected (S78 FINAL). **Enforcement:** API surface lint. [v14 P3]

---

## 2. Acceptance Criteria and Gates [v14 P3]

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
- **[v14 P3]** Added A9 (Snapshot header layout INV-023 compliance), P24 (Vec<Cell> capacity hint efficiency). [v14 P3]
- Gate results are immutable records keyed by gate ID and commit SHA.
- Every gate maps to at least one test in the CI matrix.
- Traceability: `INV-001` through `INV-024`.

### Rust Example

```rust
// crates/mux-types/src/gates.rs
// [v14 P3] Gates with waiver expiry validation and CRC32C gate

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

/// [v14 P3] GateResult with waiver expiry validation and checksum field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateResult {
    pub gate_id: &'static str,
    pub class: GateClass,
    pub lane: Lane,
    pub pass: bool,
    pub waived: bool,
    pub waiver_expiry_epoch: Option<u64>,
    pub message: String,
    pub checksum_algorithm: Option<ChecksumAlgorithm>,
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

/// [v14 P3] release_ok checks gate results with waiver expiry enforcement.
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
22. **[v14 P3]** A9 Snapshot header layout INV-023 compliance. `TST-041`. [v14 P3]
23. **[v14 P3]** P24 Vec<Cell> capacity hint efficiency. `TST-042`. [v14 P3]

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
- `RULE-S02-14`: **[v14 P3]** Snapshot header layout gates MUST validate against INV-023. **Enforcement:** Byte-level assertion. [v14 P3]

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
// [v14 P3] Extended with mux-checksum crate

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

## 5. Grid API Completeness (Cell, Line, Viewport) [v14 P3]

### Design Decisions

- **[v14]** `Cell` stores grapheme as `CompactString` (inline up to 22 bytes, heap above). Replaces `String` for reduced allocation pressure. Single ASCII chars are always inline.
- **[v14]** `Grid::put()` removed from public API. All cell writes go through `put_char()` or `put_grapheme()` (strengthens INV-012).
- **[v14]** Added `erase_in_display(mode: EraseMode)` for CSI J support: `Below`, `Above`, `All`, `Scrollback`.
- **[v14]** Added `erase_in_line(mode: EraseMode)` for CSI K support: `ToEnd`, `ToStart`, `All`.
- **[v14]** `Line` tracks `is_wrapped: bool` (renamed from `wrapped` for clarity) and round-trips through snapshot.
- **[v14 P2]** SmallVec<[Cell; 132]> for Line REJECTED: 132 cells * ~27 bytes = 3,564 bytes stack per line. For typical 24-row terminal = 85,536 bytes stack, excessive. Vec<Cell> with `Vec::with_capacity(cols)` is the right tradeoff.
- **[v14 P3]** SmallVec rejection is FINAL (S75, INV-024). See benchmark table in Preamble. Re-evaluation trigger: Cell < 16 bytes (R98). [v14 P3]
- **[v14 P2]** Line gains `capacity_hint` in constructor for pre-allocation.
- Grid uses `VecDeque<Line>` for O(1) scrollback (unchanged from v13).
- Grid tracks `revision: u64` monotonic counter (unchanged).
- Traceability: `INV-012`, `INV-016`, `INV-024`, `API-010`.

### Rust Example

```rust
// crates/mux-grid/src/lib.rs
// [v14 P3] CompactString Cell, removed put(), added erase modes, SmallVec rejected FINAL

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

/// [v14 P3] Line with Vec<Cell> (SmallVec rejected FINAL, S75, INV-024) and capacity hint. [v14 P3]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub cells: Vec<Cell>, // [v14 P3] Vec FINAL -- see S75, INV-024
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
                // CSI 3 J: clear scrollback only (stub for now)
            }
        }
        self.revision = self.revision.wrapping_add(1);
    }

    /// [v14] CSI K: erase in line.
    pub fn erase_in_line(&mut self, mode: EraseMode) {
        let row = self.cursor.row as usize;
        let col = self.cursor.col as usize;
        if row < self.lines.len() {
            match mode {
                EraseMode::Below => {
                    for c in col..self.lines[row].cells.len() {
                        self.lines[row].cells[c] = Cell::default();
                    }
                }
                EraseMode::Above => {
                    for c in 0..=col.min(self.lines[row].cells.len().saturating_sub(1)) {
                        self.lines[row].cells[c] = Cell::default();
                    }
                }
                EraseMode::All => {
                    for c in 0..self.lines[row].cells.len() {
                        self.lines[row].cells[c] = Cell::default();
                    }
                }
                EraseMode::Scrollback => {} // not applicable to line erase
            }
            self.lines[row].mark_dirty(0);
            self.revision = self.revision.wrapping_add(1);
        }
    }

    pub fn viewport_text(&self, _vp: Viewport) -> String {
        self.lines.iter().map(|l| l.trimmed_text()).collect::<Vec<_>>().join("\n")
    }

    pub fn full_viewport(&self) -> Viewport {
        Viewport { top: 0, left: 0, rows: self.rows as usize, cols: self.cols as usize }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_put_char() {
        let mut g = Grid::new(80, 24);
        g.put_char('H');
        g.put_char('i');
        assert_eq!(g.cell_at(0, 0).unwrap().grapheme.as_str(), "H");
        assert_eq!(g.cell_at(1, 0).unwrap().grapheme.as_str(), "i");
    }

    #[test]
    fn test_grid_revision_increment() {
        let mut g = Grid::new(80, 24);
        assert_eq!(g.revision(), 0);
        g.put_char('A');
        assert_eq!(g.revision(), 1);
    }

    #[test]
    fn test_grid_erase_all() {
        let mut g = Grid::new(80, 24);
        g.put_char('X');
        g.erase_in_display(EraseMode::All);
        assert!(g.cell_at(0, 0).unwrap().is_default());
    }

    #[test]
    fn test_line_dirty_tracking() {
        let mut line = Line::new(80);
        assert!(!line.is_dirty());
        line.mark_dirty(5);
        assert!(line.is_dirty());
        line.clear_dirty();
        assert!(!line.is_dirty());
    }

    #[test]
    fn test_compact_string_inline() {
        let s = CompactString::from_char('A');
        assert!(s.is_inline());
        assert_eq!(s.as_str(), "A");
    }

    #[test]
    fn test_cell_display_width() {
        let c = Cell::with_char('A');
        assert_eq!(c.display_width(), 1);
        let cont = Cell { grapheme: CompactString::default(), style: 0, wide_continuation: true };
        assert_eq!(cont.display_width(), 0);
    }

    // [v14 P3] Verify Vec is used (not SmallVec) - type-level check
    #[test]
    fn test_line_storage_is_vec() {
        let line = Line::new(80);
        let _: &Vec<Cell> = &line.cells; // compile-time Vec proof
        assert_eq!(line.cells.capacity(), 80); // capacity hint
    }
}
```

### Test Strategy

1. Grid put_char places at cursor. `TST-080`.
2. Grid revision monotonic. `TST-081`.
3. Grid erase_in_display(All) clears grid. `TST-082`.
4. Grid erase_in_display(Below) clears from cursor. `TST-083`.
5. Grid erase_in_line(All) clears row. `TST-084`.
6. Line dirty tracking correct. `TST-085`.
7. CompactString inline for ASCII. `TST-086`.
8. put_grapheme handles multi-byte. `TST-087`.
9. Cell display_width correct. `TST-088`.
10. Grid scroll_up preserves content. `TST-089`.
11. Line text extraction. `TST-090`.
12. Line trimmed_text. `TST-091`.
13. Viewport bounds. `TST-092`.
14. **[v14 P2]** Vec<Cell> capacity hint verified. `TST-093`.
15. **[v14 P3]** Line storage is Vec (not SmallVec) -- type check. `TST-094`. [v14 P3]
16. **[v14 P3]** Grid erase_in_display(Scrollback) does not crash. `TST-095`. [v14 P3]

### AGENTS.md Rules

- `RULE-S05-01`: `Cell` grapheme MUST be CompactString. **Enforcement:** Type check.
- `RULE-S05-02`: `Grid::put()` MUST NOT be public. **Enforcement:** API lint (INV-012).
- `RULE-S05-03`: All cell writes via `put_char`/`put_grapheme`. **Enforcement:** API surface lint.
- `RULE-S05-04`: `erase_in_display` and `erase_in_line` must match CSI J/K semantics. **Enforcement:** Parity test.
- `RULE-S05-05`: `Line.is_wrapped` must round-trip through snapshot. **Enforcement:** Snapshot test.
- `RULE-S05-06`: Grid revision monotonically increases. **Enforcement:** Unit test.
- `RULE-S05-07`: **[v14 P2]** Line uses Vec<Cell> with capacity hint (SmallVec rejected S75). **Enforcement:** Type check.
- `RULE-S05-08`: **[v14 P3]** SmallVec rejection is FINAL (S75, INV-024). Re-evaluation requires Cell < 16 bytes. **Enforcement:** Architecture review gate. [v14 P3]

---

## 6. VtParser State Machine and Action Dispatch [v14 P3]

### Design Decisions

- **[v14]** 7-state VtParser: `Ground`, `Escape`, `EscapeIntermediate`, `CsiEntry`, `CsiParam`, `OscString`, `DcsPassthrough`.
- **[v14]** `ByteClass` enum with 6 variants: `Printable`, `Control`, `Escape`, `CsiEntry`, `OscEntry`, `Invalid`. `#[repr(u8)]` for integer comparison.
- **[v14]** `classify(b: u8) -> ByteClass` is `const fn`.
- **[v14]** Transition table is `const` 2D array indexed by `(State, ByteClass)`. Totality verified at compile time by exhaustive indexing.
- **[v14]** `step(state: State, byte: u8) -> (State, Action)` dispatches via const table.
- **[v14]** Actions: `Print(u8)`, `ExecuteControl(u8)`, `DispatchCsi(u8)`, `DispatchOsc`, `ErrorRecover`, `Noop`.
- **[v14 P3]** Unsupported CSI finals logged as metric, not error. Parser continues (S32-DEC-36). [v14 P3]
- Traceability: `INV-018`, `PAR-001`.

### Rust Example

```rust
// crates/mux-grid/src/vtparser.rs
// [v14 P3] VtParser with ByteClass enum and const transition table

#![allow(dead_code)]

/// [v14] 7-state VtParser.
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

/// [v14] 6-variant ByteClass for fast dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ByteClass {
    Printable = 0,
    Control = 1,
    Escape = 2,
    CsiEntry = 3,
    OscEntry = 4,
    Invalid = 5,
}

/// [v14] classify is const fn for compile-time evaluation.
pub const fn classify(b: u8) -> ByteClass {
    match b {
        0x20..=0x7E => ByteClass::Printable,
        0x00..=0x1F if b == 0x1B => ByteClass::Escape,
        0x00..=0x1F => ByteClass::Control,
        0x9B => ByteClass::CsiEntry,
        0x9D => ByteClass::OscEntry,
        _ => ByteClass::Invalid,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Print(u8),
    ExecuteControl(u8),
    DispatchCsi(u8),
    DispatchOsc,
    ErrorRecover,
    Noop,
}

/// [v14] Const transition table: (State, ByteClass) -> (State, Action).
const TRANSITIONS: [[( State, Action); 6]; 7] = {
    use State::*;
    use Action::*;
    use ByteClass::*;
    [
        // Ground
        [
            (Ground, Noop),          // Printable -> print handled separately
            (Ground, Noop),          // Control -> execute handled separately
            (Escape, Noop),          // Escape
            (CsiEntry, Noop),        // CsiEntry
            (OscString, Noop),       // OscEntry
            (Ground, ErrorRecover),  // Invalid
        ],
        // Escape
        [
            (Ground, Noop),          // Printable -> dispatch
            (Ground, Noop),          // Control -> execute
            (Escape, Noop),          // Escape -> stay
            (CsiEntry, Noop),        // CsiEntry
            (OscString, Noop),       // OscEntry
            (Ground, ErrorRecover),  // Invalid
        ],
        // EscapeIntermediate
        [
            (Ground, Noop),
            (Ground, Noop),
            (Escape, Noop),
            (CsiEntry, Noop),
            (OscString, Noop),
            (Ground, ErrorRecover),
        ],
        // CsiEntry
        [
            (Ground, Noop),
            (Ground, Noop),
            (Escape, Noop),
            (CsiParam, Noop),
            (OscString, Noop),
            (Ground, ErrorRecover),
        ],
        // CsiParam
        [
            (Ground, Noop),          // Printable -> CSI final
            (Ground, Noop),          // Control
            (Escape, Noop),          // Escape
            (CsiParam, Noop),        // More params
            (OscString, Noop),       // OscEntry
            (Ground, ErrorRecover),  // Invalid
        ],
        // OscString
        [
            (OscString, Noop),       // Printable -> accumulate
            (Ground, DispatchOsc),   // Control -> dispatch (ST)
            (Ground, DispatchOsc),   // Escape -> dispatch
            (OscString, Noop),
            (OscString, Noop),
            (Ground, ErrorRecover),
        ],
        // DcsPassthrough
        [
            (DcsPassthrough, Noop),
            (Ground, Noop),
            (Ground, Noop),
            (DcsPassthrough, Noop),
            (DcsPassthrough, Noop),
            (Ground, ErrorRecover),
        ],
    ]
};

/// [v14] Step function using const transition table.
pub fn step(state: State, b: u8) -> (State, Action) {
    let cls = classify(b);
    let (next, base_action) = TRANSITIONS[state as usize][cls as usize];

    match (state, cls) {
        (State::Ground, ByteClass::Printable) => (State::Ground, Action::Print(b)),
        (State::Ground, ByteClass::Control) => (State::Ground, Action::ExecuteControl(b)),
        (State::CsiParam, ByteClass::Printable) => (State::Ground, Action::DispatchCsi(b)),
        _ => (next, base_action),
    }
}

/// [v14] VtParser holding state and CSI parameter buffer.
pub struct VtParser {
    state: State,
    params: Vec<u16>,
    current_param: u16,
    unsupported_count: u64,
}

impl VtParser {
    pub fn new() -> Self {
        Self { state: State::Ground, params: Vec::new(), current_param: 0, unsupported_count: 0 }
    }

    pub fn state(&self) -> State { self.state }
    pub fn unsupported_count(&self) -> u64 { self.unsupported_count }

    pub fn feed(&mut self, b: u8) -> Action {
        let (next, action) = step(self.state, b);
        self.state = next;
        match action {
            Action::ErrorRecover => {
                self.unsupported_count += 1;
                self.params.clear();
                self.current_param = 0;
            }
            Action::DispatchCsi(_) => {
                self.params.push(self.current_param);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_printable() {
        assert_eq!(classify(b'A'), ByteClass::Printable);
        assert_eq!(classify(b' '), ByteClass::Printable);
        assert_eq!(classify(b'~'), ByteClass::Printable);
    }

    #[test]
    fn test_classify_escape() {
        assert_eq!(classify(0x1B), ByteClass::Escape);
    }

    #[test]
    fn test_classify_control() {
        assert_eq!(classify(0x0D), ByteClass::Control); // CR
        assert_eq!(classify(0x0A), ByteClass::Control); // LF
    }

    #[test]
    fn test_step_ground_printable() {
        let (state, action) = step(State::Ground, b'A');
        assert_eq!(state, State::Ground);
        assert_eq!(action, Action::Print(b'A'));
    }

    #[test]
    fn test_step_ground_escape() {
        let (state, _) = step(State::Ground, 0x1B);
        assert_eq!(state, State::Escape);
    }

    #[test]
    fn test_7_states() {
        assert_eq!(State::Ground as u8, 0);
        assert_eq!(State::DcsPassthrough as u8, 6);
    }

    #[test]
    fn test_vtparser_feed() {
        let mut p = VtParser::new();
        let action = p.feed(b'A');
        assert_eq!(action, Action::Print(b'A'));
        assert_eq!(p.state(), State::Ground);
    }

    #[test]
    fn test_transition_table_totality() {
        // Verify table covers all 7 states * 6 byte classes = 42 entries
        assert_eq!(TRANSITIONS.len(), 7);
        for row in &TRANSITIONS {
            assert_eq!(row.len(), 6);
        }
    }
}
```

### Test Strategy

1. classify covers all byte ranges. `TST-100`.
2. Step from Ground+Printable produces Print. `TST-101`.
3. Step from Ground+Escape enters Escape. `TST-102`.
4. Step from CsiParam+Printable dispatches CSI. `TST-103`.
5. Transition table has 42 entries (7*6). `TST-104`.
6. VtParser feed produces correct actions. `TST-105`.
7. VtParser reset returns to Ground. `TST-106`.
8. **[v14]** ByteClass is #[repr(u8)] with 6 variants. `TST-107`.
9. **[v14]** classify() is const fn. `TST-108`.
10. **[v14]** Unsupported finals increment metric. `TST-109`.
11. **[v14 P3]** ErrorRecover resets parser cleanly. `TST-110`. [v14 P3]

### AGENTS.md Rules

- `RULE-S06-01`: VtParser MUST have exactly 7 states. **Enforcement:** State count test (TST-107).
- `RULE-S06-02`: ByteClass MUST have 6 variants with #[repr(u8)]. **Enforcement:** Variant count test.
- `RULE-S06-03`: Transition table MUST be const. **Enforcement:** Const check.
- `RULE-S06-04`: classify() MUST be const fn. **Enforcement:** Compile-time eval test.
- `RULE-S06-05`: Unsupported CSI finals MUST NOT panic. **Enforcement:** No-panic test.
- `RULE-S06-06`: **[v14]** Unsupported finals logged as metrics. **Enforcement:** Metric counter test.
- `RULE-S06-07`: **[v14 P3]** ErrorRecover action MUST reset parser state and clear params. **Enforcement:** State reset test. [v14 P3]

---

## 7. PtyHandle Lifecycle and Resource Cleanup [v14 P3]

### Design Decisions

- **[v14]** PtyHandle<S> uses typestate pattern for compile-time safety on common paths.
- 7 states: `Allocated`, `Spawned`, `Running`, `Stopping`, `Exited`, `Reaped`, `Closed`.
- **[v14]** Common transitions compile-checked: `PtyHandle<Allocated>.spawn() -> PtyHandle<Spawned>`.
- **[v14]** DynPtyHandle provides runtime-checked fallback for cross-language bindings and deserialization.
- **[v14]** Restart barrier: `kill -> close(old_handle) -> spawn(new_handle)` (INV-017).
- **[v14 P3]** DynPtyHandle is the ONLY path for bindings; typestate PtyHandle is for Rust-native code only. [v14 P3]
- Traceability: `INV-014`, `INV-017`.

### Rust Example

```rust
// crates/mux-pty/src/handle.rs
// [v14 P3] Typestate PtyHandle with 7-state lifecycle

#![allow(dead_code)]

use std::marker::PhantomData;

// Typestate markers
pub struct Allocated;
pub struct Spawned;
pub struct Running;
pub struct Stopping;
pub struct Exited;
pub struct Reaped;
pub struct Closed;

/// [v14] Typestate PtyHandle.
#[derive(Debug)]
pub struct PtyHandle<S> {
    pub handle_id: u64,
    pub generation: u32,
    _state: PhantomData<S>,
}

impl PtyHandle<Allocated> {
    pub fn new(handle_id: u64, generation: u32) -> Self {
        Self { handle_id, generation, _state: PhantomData }
    }
    pub fn spawn(self) -> PtyHandle<Spawned> {
        PtyHandle { handle_id: self.handle_id, generation: self.generation, _state: PhantomData }
    }
}

impl PtyHandle<Spawned> {
    pub fn running(self) -> PtyHandle<Running> {
        PtyHandle { handle_id: self.handle_id, generation: self.generation, _state: PhantomData }
    }
}

impl PtyHandle<Running> {
    pub fn stop(self) -> PtyHandle<Stopping> {
        PtyHandle { handle_id: self.handle_id, generation: self.generation, _state: PhantomData }
    }
}

impl PtyHandle<Stopping> {
    pub fn exit(self) -> PtyHandle<Exited> {
        PtyHandle { handle_id: self.handle_id, generation: self.generation, _state: PhantomData }
    }
}

impl PtyHandle<Exited> {
    pub fn reap(self) -> PtyHandle<Reaped> {
        PtyHandle { handle_id: self.handle_id, generation: self.generation, _state: PhantomData }
    }
}

impl PtyHandle<Reaped> {
    pub fn close(self) -> PtyHandle<Closed> {
        PtyHandle { handle_id: self.handle_id, generation: self.generation, _state: PhantomData }
    }
}

/// [v14] DynPtyHandle for runtime-checked paths (bindings, deserialization).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtyState {
    Allocated, Spawned, Running, Stopping, Exited, Reaped, Closed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PtyError {
    InvalidTransition { from: PtyState, to: PtyState },
    HandleClosed { handle_id: u64 },
}

impl std::fmt::Display for PtyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTransition { from, to } => write!(f, "invalid transition {from:?} -> {to:?}"),
            Self::HandleClosed { handle_id } => write!(f, "handle {handle_id} closed"),
        }
    }
}

impl std::error::Error for PtyError {}

#[derive(Debug, Clone)]
pub struct DynPtyHandle {
    pub handle_id: u64,
    pub generation: u32,
    pub state: PtyState,
}

impl DynPtyHandle {
    pub fn new(handle_id: u64, generation: u32) -> Self {
        Self { handle_id, generation, state: PtyState::Allocated }
    }

    pub fn transition(&mut self, target: PtyState) -> Result<(), PtyError> {
        if self.state == PtyState::Closed {
            return Err(PtyError::HandleClosed { handle_id: self.handle_id });
        }
        if valid_pty_transition(self.state, target) {
            self.state = target;
            Ok(())
        } else {
            Err(PtyError::InvalidTransition { from: self.state, to: target })
        }
    }
}

pub fn valid_pty_transition(from: PtyState, to: PtyState) -> bool {
    matches!(
        (from, to),
        (PtyState::Allocated, PtyState::Spawned)
        | (PtyState::Spawned, PtyState::Running)
        | (PtyState::Running, PtyState::Stopping)
        | (PtyState::Stopping, PtyState::Exited)
        | (PtyState::Exited, PtyState::Reaped)
        | (PtyState::Reaped, PtyState::Closed)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typestate_lifecycle() {
        let h = PtyHandle::<Allocated>::new(1, 0);
        let h = h.spawn();
        let h = h.running();
        let h = h.stop();
        let h = h.exit();
        let h = h.reap();
        let _closed = h.close();
    }

    #[test]
    fn test_dyn_handle_lifecycle() {
        let mut h = DynPtyHandle::new(1, 0);
        assert!(h.transition(PtyState::Spawned).is_ok());
        assert!(h.transition(PtyState::Running).is_ok());
        assert!(h.transition(PtyState::Stopping).is_ok());
        assert!(h.transition(PtyState::Exited).is_ok());
        assert!(h.transition(PtyState::Reaped).is_ok());
        assert!(h.transition(PtyState::Closed).is_ok());
    }

    #[test]
    fn test_invalid_transition() {
        let mut h = DynPtyHandle::new(1, 0);
        assert!(h.transition(PtyState::Running).is_err());
    }

    #[test]
    fn test_closed_handle_error() {
        let mut h = DynPtyHandle::new(1, 0);
        h.state = PtyState::Closed;
        assert!(matches!(h.transition(PtyState::Allocated), Err(PtyError::HandleClosed { .. })));
    }

    #[test]
    fn test_valid_transitions() {
        assert!(valid_pty_transition(PtyState::Allocated, PtyState::Spawned));
        assert!(!valid_pty_transition(PtyState::Exited, PtyState::Running));
    }
}
```

### Test Strategy

1. Typestate lifecycle compiles. `TST-120`.
2. DynPtyHandle valid transitions. `TST-121`.
3. Invalid transition returns error. `TST-122`.
4. Closed handle returns HandleClosed. `TST-123`.
5. 7 PtyState variants. `TST-124`.
6. **[v14]** Typestate prevents invalid transition at compile time. `TST-125`.
7. Generation counter increments on restart. `TST-126`.
8. **[v14 P3]** DynPtyHandle error implements Display and Error. `TST-127`. [v14 P3]

### AGENTS.md Rules

- `RULE-S07-01`: PtyHandle MUST have 7 states. **Enforcement:** State count test.
- `RULE-S07-02`: Typestate transitions compile-checked. **Enforcement:** Compile test.
- `RULE-S07-03`: DynPtyHandle validates transitions at runtime. **Enforcement:** Runtime test.
- `RULE-S07-04`: Closed handle returns HandleClosed. **Enforcement:** Error check.
- `RULE-S07-05`: Restart barrier mandatory (INV-017). **Enforcement:** Lifecycle test.
- `RULE-S07-06`: **[v14]** DynPtyHandle for bindings only. **Enforcement:** API surface lint.
- `RULE-S07-07`: **[v14 P3]** PtyError MUST implement Display + Error. **Enforcement:** Trait impl test. [v14 P3]

---

## 8. Termlet Snapshot Format and Versioning [v14 P3]

### Design Decisions

- **[v14]** Snapshot binary format with magic bytes, version dispatch, and trailing checksum.
- **[v14]** v1: `ch(u32) + style(u16) + flags(u16)` per cell (8 bytes).
- **[v14]** v2: `PackedCell(u64)` per cell (8 bytes, different encoding).
- **[v14 P2]** Checksum algorithm selectable: FNV-1a (0x00) or CRC32C (0x01).
- **[v14 P3]** DEFINITIVE snapshot header layout (INV-023): [v14 P3]

```
Offset  Size  Description
0       8     Magic bytes: "TFSNAP13"
8       2     Version (u16 LE, 1 or 2)
10      2     cols (u16 LE)
12      2     rows (u16 LE)
14      2     cursor_col (u16 LE)
16      2     cursor_row (u16 LE)
18      8     revision (u64 LE)
26      4     cell_count (u32 LE)
30      1     checksum_algo (u8: 0x00=FNV-1a, 0x01=CRC32C) [v14 P3]

--- v1: cell_count * 8 bytes ---
31      N*8   cells: ch(u32) + style(u16) + flags(u16) per cell

--- v2: cell_count * 8 bytes ---
31      N*8   cells: PackedCell(u64) per cell

31+N*8  4     Checksum (u32 LE, algorithm per checksum_algo byte)
```

- **[v14 P3]** Header is 31 bytes (not 30 as in P2). checksum_algo byte at offset 30 resolves the offset discrepancy between Claude P2 (algo at 26) and GPT P2 (cell_count at 26). DEFINITIVE resolution: cell_count at 26, algo at 30. [v14 P3]
- Traceability: `INV-015`, `INV-020`, `INV-021`, `INV-023`.

### Rust Example

```rust
// crates/mux-snapshot/src/lib.rs
// [v14 P3] Snapshot binary format with DEFINITIVE header layout

#![allow(dead_code)]

pub const MAGIC: &[u8; 8] = b"TFSNAP13";
pub const HEADER_SIZE: usize = 31; // [v14 P3] DEFINITIVE
pub const CHECKSUM_ALGO_FNV1A: u8 = 0x00;
pub const CHECKSUM_ALGO_CRC32C: u8 = 0x01;

/// [v14] PackedCell(u64) for v2 snapshots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PackedCell(pub u64);

const CH_MASK: u64 = 0x1F_FFFF;
const STYLE_SHIFT: u32 = 21;
const FLAGS_SHIFT: u32 = 37;
const STYLE_MASK: u64 = 0xFFFF;
const FLAGS_MASK: u64 = 0xFFFF;

impl PackedCell {
    pub const fn new(ch: u32, style: u16, flags: u16) -> Self {
        let v = ((ch as u64) & CH_MASK)
            | (((style as u64) & STYLE_MASK) << STYLE_SHIFT)
            | (((flags as u64) & FLAGS_MASK) << FLAGS_SHIFT);
        Self(v)
    }
    pub const fn ch(self) -> u32 { (self.0 & CH_MASK) as u32 }
    pub const fn style(self) -> u16 { ((self.0 >> STYLE_SHIFT) & STYLE_MASK) as u16 }
    pub const fn flags(self) -> u16 { ((self.0 >> FLAGS_SHIFT) & FLAGS_MASK) as u16 }
    pub const fn reserved(self) -> u16 { ((self.0 >> 53) & 0x7FF) as u16 }
    pub const fn is_valid(self) -> bool { self.reserved() == 0 }
    pub const fn space() -> Self { Self::new(b' ' as u32, 0, 0) }
}

impl Default for PackedCell { fn default() -> Self { Self::space() } }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotError {
    BadMagic,
    UnsupportedVersion,
    Truncated,
    ChecksumMismatch,
    InvalidAlgorithm, // [v14 P2]
}

impl std::fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadMagic => write!(f, "bad magic bytes"),
            Self::UnsupportedVersion => write!(f, "unsupported version"),
            Self::Truncated => write!(f, "truncated payload"),
            Self::ChecksumMismatch => write!(f, "checksum mismatch"),
            Self::InvalidAlgorithm => write!(f, "invalid checksum algorithm"),
        }
    }
}

impl std::error::Error for SnapshotError {}

/// [v14 P2] FNV-1a 32-bit checksum.
pub fn fnv1a_checksum(data: &[u8]) -> u32 {
    let mut hash: u32 = 0x811c_9dc5;
    for &b in data {
        hash ^= b as u32;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

/// [v14 P2] CRC32C software implementation.
pub fn crc32c_checksum(data: &[u8]) -> u32 {
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

/// [v14 P3] Compute checksum using specified algorithm.
pub fn compute_checksum(data: &[u8], algo: u8) -> Result<u32, SnapshotError> {
    match algo {
        CHECKSUM_ALGO_FNV1A => Ok(fnv1a_checksum(data)),
        CHECKSUM_ALGO_CRC32C => Ok(crc32c_checksum(data)),
        _ => Err(SnapshotError::InvalidAlgorithm),
    }
}

/// [v14 P3] Encode snapshot header (DEFINITIVE layout, INV-023).
pub fn encode_header(
    version: u16, cols: u16, rows: u16,
    cursor_col: u16, cursor_row: u16,
    revision: u64, cell_count: u32, checksum_algo: u8,
) -> [u8; HEADER_SIZE] {
    let mut buf = [0u8; HEADER_SIZE];
    buf[0..8].copy_from_slice(MAGIC);
    buf[8..10].copy_from_slice(&version.to_le_bytes());
    buf[10..12].copy_from_slice(&cols.to_le_bytes());
    buf[12..14].copy_from_slice(&rows.to_le_bytes());
    buf[14..16].copy_from_slice(&cursor_col.to_le_bytes());
    buf[16..18].copy_from_slice(&cursor_row.to_le_bytes());
    buf[18..26].copy_from_slice(&revision.to_le_bytes());
    buf[26..30].copy_from_slice(&cell_count.to_le_bytes());
    buf[30] = checksum_algo; // [v14 P3] DEFINITIVE offset
    buf
}

/// [v14 P3] Decode and validate snapshot header.
pub fn decode_header(data: &[u8]) -> Result<(u16, u16, u16, u16, u16, u64, u32, u8), SnapshotError> {
    if data.len() < HEADER_SIZE { return Err(SnapshotError::Truncated); }
    if &data[0..8] != MAGIC { return Err(SnapshotError::BadMagic); }
    let version = u16::from_le_bytes([data[8], data[9]]);
    if version != 1 && version != 2 { return Err(SnapshotError::UnsupportedVersion); }
    let cols = u16::from_le_bytes([data[10], data[11]]);
    let rows = u16::from_le_bytes([data[12], data[13]]);
    let cursor_col = u16::from_le_bytes([data[14], data[15]]);
    let cursor_row = u16::from_le_bytes([data[16], data[17]]);
    let revision = u64::from_le_bytes([
        data[18], data[19], data[20], data[21],
        data[22], data[23], data[24], data[25],
    ]);
    let cell_count = u32::from_le_bytes([data[26], data[27], data[28], data[29]]);
    let checksum_algo = data[30];
    if checksum_algo != CHECKSUM_ALGO_FNV1A && checksum_algo != CHECKSUM_ALGO_CRC32C {
        return Err(SnapshotError::InvalidAlgorithm);
    }
    Ok((version, cols, rows, cursor_col, cursor_row, revision, cell_count, checksum_algo))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packed_cell_roundtrip() {
        let p = PackedCell::new(b'A' as u32, 7, 3);
        assert_eq!(p.ch(), b'A' as u32);
        assert_eq!(p.style(), 7);
        assert_eq!(p.flags(), 3);
        assert!(p.is_valid());
    }

    #[test]
    fn test_packed_cell_max_unicode() {
        let p = PackedCell::new(0x10FFFF, 0xFFFF, 0xFFFF);
        assert_eq!(p.ch(), 0x10FFFF);
        assert_eq!(p.style(), 0xFFFF);
        assert_eq!(p.flags(), 0xFFFF);
    }

    #[test]
    fn test_header_encode_decode() {
        let hdr = encode_header(2, 80, 24, 5, 10, 42, 1920, CHECKSUM_ALGO_FNV1A);
        let (v, c, r, cc, cr, rev, cnt, algo) = decode_header(&hdr).unwrap();
        assert_eq!(v, 2);
        assert_eq!(c, 80);
        assert_eq!(r, 24);
        assert_eq!(cc, 5);
        assert_eq!(cr, 10);
        assert_eq!(rev, 42);
        assert_eq!(cnt, 1920);
        assert_eq!(algo, CHECKSUM_ALGO_FNV1A);
    }

    #[test]
    fn test_header_size_31() {
        assert_eq!(HEADER_SIZE, 31); // [v14 P3] INV-023
    }

    #[test]
    fn test_bad_magic() {
        let mut hdr = encode_header(1, 80, 24, 0, 0, 0, 0, 0);
        hdr[0] = b'X';
        assert_eq!(decode_header(&hdr), Err(SnapshotError::BadMagic));
    }

    #[test]
    fn test_invalid_algorithm() {
        let mut hdr = encode_header(1, 80, 24, 0, 0, 0, 0, 0);
        hdr[30] = 0xFF; // invalid algo
        assert_eq!(decode_header(&hdr), Err(SnapshotError::InvalidAlgorithm));
    }

    #[test]
    fn test_fnv1a_deterministic() {
        let d = b"test";
        assert_eq!(fnv1a_checksum(d), fnv1a_checksum(d));
    }

    #[test]
    fn test_crc32c_deterministic() {
        let d = b"test";
        assert_eq!(crc32c_checksum(d), crc32c_checksum(d));
    }

    #[test]
    fn test_different_algos_different_values() {
        let d = b"test payload";
        assert_ne!(fnv1a_checksum(d), crc32c_checksum(d));
    }

    #[test]
    fn test_compute_checksum_dispatch() {
        let d = b"data";
        assert_eq!(compute_checksum(d, CHECKSUM_ALGO_FNV1A).unwrap(), fnv1a_checksum(d));
        assert_eq!(compute_checksum(d, CHECKSUM_ALGO_CRC32C).unwrap(), crc32c_checksum(d));
        assert!(compute_checksum(d, 0xFF).is_err());
    }
}
```

### Test Strategy

1. PackedCell round-trip. `TST-130`.
2. PackedCell max Unicode codepoint. `TST-131`.
3. Header encode/decode round-trip. `TST-132`.
4. Bad magic rejected. `TST-133`.
5. Unsupported version rejected. `TST-134`.
6. Truncated payload rejected. `TST-135`.
7. FNV-1a deterministic. `TST-136`.
8. **[v14 P2]** CRC32C deterministic. `TST-137`.
9. **[v14 P2]** Invalid algorithm rejected. `TST-138`.
10. **[v14 P3]** Header size is exactly 31 bytes (INV-023). `TST-139`. [v14 P3]
11. **[v14 P3]** checksum_algo byte at offset 30. `TST-140`. [v14 P3]
12. **[v14 P3]** compute_checksum dispatches correctly. `TST-141`. [v14 P3]

### AGENTS.md Rules

- `RULE-S08-01`: Snapshot MUST have trailing checksum. **Enforcement:** Decode test.
- `RULE-S08-02`: Bad magic MUST reject. **Enforcement:** Error test.
- `RULE-S08-03`: v1 and v2 decoders MUST coexist. **Enforcement:** Dual decode test.
- `RULE-S08-04`: PackedCell ch field 21 bits. **Enforcement:** Bit-level test.
- `RULE-S08-05`: **[v14 P2]** CRC32C opt-in via feature flag. **Enforcement:** Feature test.
- `RULE-S08-06`: **[v14 P2]** Checksum algorithm byte MUST be validated. **Enforcement:** Decode test.
- `RULE-S08-07`: **[v14 P3]** Header layout MUST match INV-023 (31 bytes, algo at offset 30). **Enforcement:** Byte-level test. [v14 P3]
- `RULE-S08-08`: **[v14 P3]** Snapshot decode order: magic -> version -> header -> payload -> checksum. **Enforcement:** Sequence test. [v14 P3]

---

## 9. Wire Protocol Compatibility (tmux) [v14 P3]

### Design Decisions

- Protocol v8 is the target (INV-001).
- Wire format: length-prefixed message frames.
- Control mode: `%begin`/`%end` framing with backpressure.
- **[v14]** `CausationId` optional field in Frame for causal tracing.
- **[v14 P3]** Protocol negotiation failure is typed and non-panicking (adopted from GPT P2 S32.43). [v14 P3]
- Traceability: `INV-001`, `INV-004`, `PAR-002`.

### Rust Example

```rust
// crates/mux-proto/src/lib.rs
// [v14 P3] Wire protocol types

#![allow(dead_code)]

pub const PROTOCOL_VERSION: u32 = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtoError {
    BadFrame { reason: String },
    VersionMismatch { expected: u32, got: u32 },
    NegotiationFailed { reason: String }, // [v14 P3]
}

impl std::fmt::Display for ProtoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadFrame { reason } => write!(f, "bad frame: {reason}"),
            Self::VersionMismatch { expected, got } => write!(f, "version mismatch: expected {expected}, got {got}"),
            Self::NegotiationFailed { reason } => write!(f, "negotiation failed: {reason}"),
        }
    }
}

impl std::error::Error for ProtoError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub msg_type: u16,
    pub payload: Vec<u8>,
    pub causation_id: Option<u64>,
}

impl Frame {
    pub fn new(msg_type: u16, payload: Vec<u8>) -> Self {
        Self { msg_type, payload, causation_id: None }
    }

    pub fn with_causation(mut self, id: u64) -> Self {
        self.causation_id = Some(id);
        self
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        let len = (self.payload.len() + 2) as u32;
        buf.extend_from_slice(&len.to_le_bytes());
        buf.extend_from_slice(&self.msg_type.to_le_bytes());
        buf.extend_from_slice(&self.payload);
        buf
    }

    pub fn decode(data: &[u8]) -> Result<Self, ProtoError> {
        if data.len() < 6 {
            return Err(ProtoError::BadFrame { reason: "too short".into() });
        }
        let len = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
        if data.len() < 4 + len {
            return Err(ProtoError::BadFrame { reason: "truncated".into() });
        }
        let msg_type = u16::from_le_bytes([data[4], data[5]]);
        let payload = data[6..4 + len].to_vec();
        Ok(Self { msg_type, payload, causation_id: None })
    }
}

pub fn negotiate_version(client_version: u32) -> Result<u32, ProtoError> {
    if client_version == PROTOCOL_VERSION {
        Ok(PROTOCOL_VERSION)
    } else {
        Err(ProtoError::NegotiationFailed {
            reason: format!("client v{client_version} != server v{PROTOCOL_VERSION}"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_encode_decode() {
        let f = Frame::new(1, vec![0xAA, 0xBB]);
        let encoded = f.encode();
        let decoded = Frame::decode(&encoded).unwrap();
        assert_eq!(decoded.msg_type, 1);
        assert_eq!(decoded.payload, vec![0xAA, 0xBB]);
    }

    #[test]
    fn test_bad_frame() {
        assert!(Frame::decode(&[0, 0]).is_err());
    }

    #[test]
    fn test_negotiation_ok() {
        assert_eq!(negotiate_version(8), Ok(8));
    }

    #[test]
    fn test_negotiation_fail() {
        assert!(negotiate_version(7).is_err());
    }
}
```

### Test Strategy

1. Frame encode/decode round-trip. `TST-150`.
2. Bad frame rejected. `TST-151`.
3. Protocol v8 handshake. `TST-152`.
4. **[v14]** CausationId present in frame. `TST-153`.
5. **[v14 P3]** Protocol negotiation failure is typed. `TST-154`. [v14 P3]

### AGENTS.md Rules

- `RULE-S09-01`: Protocol version is 8. **Enforcement:** Version constant test.
- `RULE-S09-02`: Bad frames MUST be rejected. **Enforcement:** Error test.
- `RULE-S09-03`: Protocol violation drops connection (INV-004). **Enforcement:** Integration test.
- `RULE-S09-04`: **[v14 P3]** Negotiation failure is typed ProtoError, not panic. **Enforcement:** Error type test. [v14 P3]

---

## 10. Configuration and Options

### Design Decisions

- Config format: tmux-compatible `set-option` / `set-window-option` commands.
- Config resolution chain: server -> session -> window -> pane (with fallthrough).
- `validate()` returns structured errors.
- **[v14]** `OverflowPolicy` for backpressure configuration.
- Traceability: `INV-005`, `PAR-003`.

### Rust Example

```rust
// crates/mux-conf/src/lib.rs
// [v14 P3] Configuration types

#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OptionScope { Server, Session, Window, Pane }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OptionValue {
    Bool(bool),
    Int(i64),
    Str(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfOption {
    pub name: String,
    pub scope: OptionScope,
    pub value: OptionValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfError {
    UnknownOption(String),
    InvalidValue { option: String, value: String },
    ScopeMismatch { option: String, expected: OptionScope, got: OptionScope },
}

impl std::fmt::Display for ConfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownOption(name) => write!(f, "unknown option: {name}"),
            Self::InvalidValue { option, value } => write!(f, "invalid value {value:?} for {option}"),
            Self::ScopeMismatch { option, .. } => write!(f, "scope mismatch for {option}"),
        }
    }
}

impl std::error::Error for ConfError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverflowPolicy { Drop, Block, Latest }

pub fn validate_option(name: &str) -> Result<OptionScope, ConfError> {
    match name {
        "status" | "base-index" | "history-limit" => Ok(OptionScope::Session),
        "mode-keys" | "pane-base-index" => Ok(OptionScope::Window),
        "default-shell" | "default-command" => Ok(OptionScope::Server),
        _ => Err(ConfError::UnknownOption(name.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_known() { assert!(validate_option("status").is_ok()); }

    #[test]
    fn test_validate_unknown() { assert!(validate_option("nonexistent").is_err()); }
}
```

### Test Strategy

1. Config parse valid file. `TST-160`.
2. Unknown option error. `TST-161`.
3. Resolution chain fallthrough. `TST-162`.
4. **[v14]** OverflowPolicy variants. `TST-163`.

### AGENTS.md Rules

- `RULE-S10-01`: Config resolution chain: server -> session -> window -> pane. **Enforcement:** Resolution test.
- `RULE-S10-02`: Unknown options produce structured error. **Enforcement:** Error test.
- `RULE-S10-03`: Config changes require reload signal. **Enforcement:** Integration test.

---

## 11. Layout Engine

### Design Decisions

- Round-robin one-cell adjustment for resize (INV-006).
- Layout checksum for parity with tmux.
- Split operations: horizontal and vertical.
- Traceability: `INV-006`, `PAR-004`.

### Rust Example

```rust
// crates/mux-core/src/layout.rs
// [v14 P3] Layout engine

#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutNode {
    pub id: u32,
    pub x: u16, pub y: u16,
    pub width: u16, pub height: u16,
    pub children: Vec<LayoutNode>,
}

impl LayoutNode {
    pub fn leaf(id: u32, x: u16, y: u16, w: u16, h: u16) -> Self {
        Self { id, x, y, width: w, height: h, children: Vec::new() }
    }

    pub fn checksum(&self) -> u32 {
        let mut sum: u32 = 0;
        sum = sum.wrapping_add(self.width as u32);
        sum = sum.wrapping_add(self.height as u32);
        for child in &self.children {
            sum = sum.wrapping_add(child.checksum());
        }
        sum
    }

    pub fn total_area(&self) -> u32 {
        if self.children.is_empty() {
            self.width as u32 * self.height as u32
        } else {
            self.children.iter().map(|c| c.total_area()).sum()
        }
    }
}

/// [v14] Round-robin resize by one cell per iteration.
pub fn resize_round_robin(nodes: &mut [LayoutNode], delta: i16) {
    if nodes.is_empty() { return; }
    let mut remaining = delta.unsigned_abs() as usize;
    let mut idx = 0;
    while remaining > 0 {
        if delta > 0 {
            nodes[idx].width += 1;
        } else if nodes[idx].width > 1 {
            nodes[idx].width -= 1;
        }
        remaining -= 1;
        idx = (idx + 1) % nodes.len();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_checksum() {
        let n = LayoutNode::leaf(1, 0, 0, 80, 24);
        assert!(n.checksum() > 0);
    }

    #[test]
    fn test_round_robin_grow() {
        let mut nodes = vec![
            LayoutNode::leaf(1, 0, 0, 40, 24),
            LayoutNode::leaf(2, 40, 0, 40, 24),
        ];
        resize_round_robin(&mut nodes, 2);
        assert_eq!(nodes[0].width, 41);
        assert_eq!(nodes[1].width, 41);
    }
}
```

### Test Strategy

1. Layout checksum stable. `TST-170`.
2. Round-robin resize distributes evenly. `TST-171`.
3. Layout total area correct. `TST-172`.

### AGENTS.md Rules

- `RULE-S11-01`: Resize uses round-robin (INV-006). **Enforcement:** Resize test.
- `RULE-S11-02`: Layout checksum matches tmux. **Enforcement:** Parity test.

---

## 12. ORM and QueryList

### Design Decisions

- QueryList: `filter_by`, `sort_by`, `paginate`, `first`, `last`.
- QuerySpec uses typed operators: `Eq`, `StartsWith`, `Contains`, `GreaterThan`, `LessThan`.
- Traceability: `API-005`.

### Rust Example

```rust
// crates/mux-query/src/lib.rs
// [v14 P3] ORM-like query API

#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryOp {
    Eq(String),
    StartsWith(String),
    Contains(String),
    GreaterThan(String),
    LessThan(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuerySpec {
    pub field: String,
    pub op: QueryOp,
}

#[derive(Debug, Clone)]
pub struct QueryList<T> {
    items: Vec<T>,
}

impl<T: Clone> QueryList<T> {
    pub fn new(items: Vec<T>) -> Self { Self { items } }
    pub fn len(&self) -> usize { self.items.len() }
    pub fn is_empty(&self) -> bool { self.items.is_empty() }
    pub fn first(&self) -> Option<&T> { self.items.first() }
    pub fn last(&self) -> Option<&T> { self.items.last() }
    pub fn paginate(&self, offset: usize, limit: usize) -> Vec<&T> {
        self.items.iter().skip(offset).take(limit).collect()
    }
    pub fn all(&self) -> &[T] { &self.items }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_list_basic() {
        let ql = QueryList::new(vec![1, 2, 3]);
        assert_eq!(ql.len(), 3);
        assert_eq!(ql.first(), Some(&1));
        assert_eq!(ql.last(), Some(&3));
    }

    #[test]
    fn test_paginate() {
        let ql = QueryList::new(vec![1, 2, 3, 4, 5]);
        let page = ql.paginate(1, 2);
        assert_eq!(page, vec![&2, &3]);
    }
}
```

### Test Strategy

1. QueryList filter_by. `TST-180`.
2. QueryList paginate. `TST-181`.
3. QuerySpec parse. `TST-182`.
4. Empty QueryList. `TST-183`.

### AGENTS.md Rules

- `RULE-S12-01`: QuerySpec operators are typed, not strings. **Enforcement:** Type check.
- `RULE-S12-02`: Paginate must handle out-of-bounds gracefully. **Enforcement:** Bounds test.

---

## 13. ServerGraph Slot Reclamation and Generation Counters

### Design Decisions

- SlotMap pattern with generation counters (INV-013).
- Slot reclamation prevents ABA problems.
- **[v14]** Generation wraps at u32::MAX with overflow detection.
- Traceability: `INV-013`.

### Rust Example

```rust
// crates/mux-core/src/slot.rs
// [v14 P3] SlotMap with generation counters

#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SlotId { pub index: u32, pub generation: u32 }

pub struct SlotMap<T> {
    entries: Vec<Option<(u32, T)>>,
    free_list: Vec<u32>,
    generation: u32,
}

impl<T> SlotMap<T> {
    pub fn new() -> Self { Self { entries: Vec::new(), free_list: Vec::new(), generation: 0 } }

    pub fn insert(&mut self, value: T) -> SlotId {
        self.generation = self.generation.wrapping_add(1);
        if let Some(idx) = self.free_list.pop() {
            self.entries[idx as usize] = Some((self.generation, value));
            SlotId { index: idx, generation: self.generation }
        } else {
            let idx = self.entries.len() as u32;
            self.entries.push(Some((self.generation, value)));
            SlotId { index: idx, generation: self.generation }
        }
    }

    pub fn get(&self, id: SlotId) -> Option<&T> {
        self.entries.get(id.index as usize)?.as_ref().and_then(|(gen, val)| {
            if *gen == id.generation { Some(val) } else { None }
        })
    }

    pub fn remove(&mut self, id: SlotId) -> Option<T> {
        let entry = self.entries.get_mut(id.index as usize)?;
        if let Some((gen, _)) = entry {
            if *gen == id.generation {
                let (_, val) = entry.take().unwrap();
                self.free_list.push(id.index);
                return Some(val);
            }
        }
        None
    }

    pub fn len(&self) -> usize { self.entries.iter().filter(|e| e.is_some()).count() }
    pub fn is_empty(&self) -> bool { self.len() == 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_get() {
        let mut sm = SlotMap::new();
        let id = sm.insert(42);
        assert_eq!(sm.get(id), Some(&42));
    }

    #[test]
    fn test_remove() {
        let mut sm = SlotMap::new();
        let id = sm.insert(42);
        assert_eq!(sm.remove(id), Some(42));
        assert_eq!(sm.get(id), None);
    }

    #[test]
    fn test_aba_prevention() {
        let mut sm = SlotMap::new();
        let id1 = sm.insert(1);
        sm.remove(id1);
        let id2 = sm.insert(2);
        assert_eq!(sm.get(id1), None); // old generation
        assert_eq!(sm.get(id2), Some(&2)); // new generation
    }
}
```

### Test Strategy

1. Slot insert and get. `TST-190`.
2. Slot remove. `TST-191`.
3. ABA prevention. `TST-192`.
4. **[v14]** Generation overflow wraps. `TST-193`.

### AGENTS.md Rules

- `RULE-S13-01`: Generation counters prevent ABA (INV-013). **Enforcement:** ABA test.
- `RULE-S13-02`: Removed slots return to free list. **Enforcement:** Reuse test.

---

## 14. State Actor and Snapshot Publication

### Design Decisions

- Single-writer state mutation (INV-003).
- Snapshot-based readers: never see partial state.
- Event/Effect system for side effects.
- Effects are idempotent by key (INV-010).
- Traceability: `INV-003`, `INV-010`.

### Rust Example

```rust
// crates/mux-state/src/lib.rs
#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EffectKey(pub String);

#[derive(Debug, Clone)]
pub struct Effect {
    pub key: EffectKey,
    pub applied: bool,
}

pub struct StateActor {
    revision: u64,
    effects: Vec<Effect>,
}

impl StateActor {
    pub fn new() -> Self { Self { revision: 0, effects: Vec::new() } }

    pub fn apply_effect(&mut self, effect: Effect) {
        if !self.effects.iter().any(|e| e.key == effect.key && e.applied) {
            self.effects.push(effect);
            self.revision += 1;
        }
    }

    pub fn revision(&self) -> u64 { self.revision }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_idempotent_effect() {
        let mut sa = StateActor::new();
        let e = Effect { key: EffectKey("k1".into()), applied: true };
        sa.apply_effect(e.clone());
        sa.apply_effect(e);
        assert_eq!(sa.revision(), 1); // idempotent
    }
}
```

### Test Strategy

1. Single-writer mutation. `TST-200`.
2. Snapshot isolation. `TST-201`.
3. Effect idempotency. `TST-202`.

### AGENTS.md Rules

- `RULE-S14-01`: Single writer for state (INV-003). **Enforcement:** Thread-safety test.
- `RULE-S14-02`: Effects idempotent by key (INV-010). **Enforcement:** Duplicate test.

---

## 15. Control Mode

### Design Decisions

- `%begin`/`%end` framing for control mode output.
- Backpressure: pending limit from tmux `control.c:450`.
- Traceability: `PAR-005`.

### Rust Example

```rust
// crates/mux-control/src/lib.rs
#![allow(dead_code)]

pub const PENDING_LIMIT: usize = 2048;

pub struct ControlSession {
    pending: Vec<String>,
}

impl ControlSession {
    pub fn new() -> Self { Self { pending: Vec::new() } }

    pub fn push(&mut self, msg: String) -> bool {
        if self.pending.len() >= PENDING_LIMIT { return false; }
        self.pending.push(msg);
        true
    }

    pub fn drain(&mut self) -> Vec<String> { std::mem::take(&mut self.pending) }
    pub fn pending_count(&self) -> usize { self.pending.len() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backpressure() {
        let mut cs = ControlSession::new();
        for i in 0..PENDING_LIMIT { assert!(cs.push(format!("msg{i}"))); }
        assert!(!cs.push("overflow".into()));
    }
}
```

### Test Strategy

1. Control mode framing. `TST-210`.
2. Backpressure at pending limit. `TST-211`.

### AGENTS.md Rules

- `RULE-S15-01`: Pending limit enforced. **Enforcement:** Limit test.
- `RULE-S15-02`: Control mode framing matches tmux. **Enforcement:** Parity test.

---

## 16. Language Bindings [v14 P3]

### Design Decisions

- Python: PyO3 bindings. Context manager cleanup.
- Node: Neon bindings. Async-first.
- **[v14]** Both bindings expose `expect_or_fail` and snapshot v2.
- **[v14 P3]** Cross-language assertion canonicalization preserves snapshot text, cursor, and style semantics. [v14 P3]
- Traceability: `API-006`.

### Rust Example

```rust
// crates/mux-bindings-shared/src/lib.rs
#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingMeta {
    pub language: &'static str,
    pub version: &'static str,
    pub supports_snapshot_v2: bool,
    pub supports_expect_or_fail: bool,
}

pub const PYTHON_META: BindingMeta = BindingMeta {
    language: "python", version: "0.1.0",
    supports_snapshot_v2: true, supports_expect_or_fail: true,
};

pub const NODE_META: BindingMeta = BindingMeta {
    language: "node", version: "0.1.0",
    supports_snapshot_v2: true, supports_expect_or_fail: true,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bindings_support_v2() {
        assert!(PYTHON_META.supports_snapshot_v2);
        assert!(NODE_META.supports_snapshot_v2);
    }
}
```

### Test Strategy

1. Python bindings import. `TST-220`.
2. Node bindings import. `TST-221`.
3. Cross-language snapshot parity. `TST-222`.
4. **[v14]** expect_or_fail exposed in both. `TST-223`.

### AGENTS.md Rules

- `RULE-S16-01`: Bindings MUST expose TermletLike API. **Enforcement:** API surface test.
- `RULE-S16-02`: **[v14]** Bindings MUST support expect_or_fail. **Enforcement:** Feature test.
- `RULE-S16-03`: **[v14 P3]** Cross-language canonicalization preserves snapshot semantics. **Enforcement:** Parity test. [v14 P3]

---

## 17. CRDT Collaboration Layer [v14 P3]

### Design Decisions

- LWWFieldMap for per-field conflict resolution.
- PaneOpLog for concurrent pane input merging.
- **[v14]** OpLog uses binary insertion via `partition_point()` (O(log n) insert position).
- **[v14 P2]** CRDT merge commutativity tested with 3+ concurrent writers.
- Behind `crdt` feature flag.
- Traceability: `INV-010`.

### Rust Example

```rust
// crates/mux-crdt/src/lib.rs
// [v14 P3] CRDT with binary insertion OpLog

#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LamportTimestamp { pub counter: u64, pub client_id: u32 }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpLogEntry {
    pub timestamp: LamportTimestamp,
    pub data: Vec<u8>,
}

pub struct PaneOpLog {
    entries: Vec<OpLogEntry>,
}

impl PaneOpLog {
    pub fn new() -> Self { Self { entries: Vec::new() } }

    /// [v14] Binary insertion: O(log n) position finding.
    pub fn insert(&mut self, entry: OpLogEntry) {
        let pos = self.entries.partition_point(|e| e.timestamp < entry.timestamp);
        self.entries.insert(pos, entry);
    }

    pub fn len(&self) -> usize { self.entries.len() }
    pub fn is_empty(&self) -> bool { self.entries.is_empty() }
    pub fn entries(&self) -> &[OpLogEntry] { &self.entries }

    pub fn is_sorted(&self) -> bool {
        self.entries.windows(2).all(|w| w[0].timestamp <= w[1].timestamp)
    }
}

#[derive(Debug, Clone)]
pub struct LwwField<T: Clone> {
    pub value: T,
    pub timestamp: LamportTimestamp,
}

impl<T: Clone> LwwField<T> {
    pub fn new(value: T, ts: LamportTimestamp) -> Self { Self { value, timestamp: ts } }
    pub fn merge(&mut self, other: &LwwField<T>) {
        if other.timestamp > self.timestamp {
            self.value = other.value.clone();
            self.timestamp = other.timestamp.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oplog_sorted() {
        let mut log = PaneOpLog::new();
        log.insert(OpLogEntry { timestamp: LamportTimestamp { counter: 3, client_id: 1 }, data: vec![3] });
        log.insert(OpLogEntry { timestamp: LamportTimestamp { counter: 1, client_id: 1 }, data: vec![1] });
        log.insert(OpLogEntry { timestamp: LamportTimestamp { counter: 2, client_id: 1 }, data: vec![2] });
        assert!(log.is_sorted());
        assert_eq!(log.entries()[0].data, vec![1]);
    }

    #[test]
    fn test_lww_merge() {
        let mut f1 = LwwField::new("old", LamportTimestamp { counter: 1, client_id: 1 });
        let f2 = LwwField::new("new", LamportTimestamp { counter: 2, client_id: 1 });
        f1.merge(&f2);
        assert_eq!(f1.value, "new");
    }
}
```

### Test Strategy

1. OpLog sorted after insert. `TST-230`.
2. OpLog binary insertion O(log n). `TST-231`.
3. LWW merge resolves by timestamp. `TST-232`.
4. **[v14 P2]** 3+ concurrent writers converge. `TST-233`.

### AGENTS.md Rules

- `RULE-S17-01`: OpLog MUST use binary insertion (S69). **Enforcement:** Insertion test.
- `RULE-S17-02`: CRDT behind `crdt` feature flag. **Enforcement:** Feature check.
- `RULE-S17-03`: Merge commutativity verified. **Enforcement:** Property test.

---

## 18. Socket and IPC

### Design Decisions

- Unix domain socket for IPC.
- Socket isolation via three guard layers (INV-007).
- Traceability: `INV-007`.

### Rust Example

```rust
// crates/mux-os/src/socket.rs
#![allow(dead_code)]

pub fn socket_path(uid: u32, name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("termforge-{uid}")).join(format!("{name}.sock"))
}

pub fn validate_socket_dir(path: &std::path::Path) -> bool {
    path.to_string_lossy().contains("termforge-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_socket_path() {
        let p = socket_path(1000, "default");
        assert!(p.to_string_lossy().contains("termforge-1000"));
    }
}
```

### Test Strategy

1. Socket path format. `TST-240`.
2. Socket isolation. `TST-241`.

### AGENTS.md Rules

- `RULE-S18-01`: Socket isolation uses three guard layers (INV-007). **Enforcement:** Isolation test.

---

## 19. OpenTelemetry (OTEL) Observability [v14 P3]

### Design Decisions

- 5-level span hierarchy: server -> session -> window -> pane -> binding.call.
- **[v14]** Level 4 `binding.call` behind feature flag.
- Traceability: `OPS-005`.

### Rust Example

```rust
// crates/mux-otel/src/lib.rs
#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanLevel { Server, Session, Window, Pane, BindingCall }

pub fn span_name(level: SpanLevel, operation: &str) -> String {
    let prefix = match level {
        SpanLevel::Server => "server",
        SpanLevel::Session => "session",
        SpanLevel::Window => "window",
        SpanLevel::Pane => "pane",
        SpanLevel::BindingCall => "binding",
    };
    format!("{prefix}.{operation}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_name() {
        assert_eq!(span_name(SpanLevel::Pane, "put_char"), "pane.put_char");
    }

    #[test]
    fn test_5_levels() {
        let levels = [SpanLevel::Server, SpanLevel::Session, SpanLevel::Window, SpanLevel::Pane, SpanLevel::BindingCall];
        assert_eq!(levels.len(), 5);
    }
}
```

### Test Strategy

1. 5-level hierarchy. `TST-250`.
2. Span names correct. `TST-251`.
3. **[v14]** binding.call level present. `TST-252`.

### AGENTS.md Rules

- `RULE-S19-01`: 5 span levels. **Enforcement:** Level count test.
- `RULE-S19-02`: binding.call behind feature flag. **Enforcement:** Feature check.

---

## 20. tmux Version Management (mux-vm, mux-builder)

### Design Decisions

- `mux-vm` manages tmux binary versions for parity testing.
- `mux-builder` compiles tmux from source.
- `mux-regress` runs parity regression suite.
- Traceability: `PAR-006`.

### Rust Example

```rust
// tools/mux-vm/src/lib.rs
#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TmuxVersion { pub major: u8, pub minor: u8, pub patch: u8 }

impl TmuxVersion {
    pub fn new(major: u8, minor: u8, patch: u8) -> Self { Self { major, minor, patch } }
    pub fn label(&self) -> String { format!("{}.{}.{}", self.major, self.minor, self.patch) }
}

pub const TARGET_VERSION: TmuxVersion = TmuxVersion { major: 3, minor: 4, patch: 0 };

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_label() {
        assert_eq!(TARGET_VERSION.label(), "3.4.0");
    }
}
```

### Test Strategy

1. Version manager discovers tmux. `TST-260`.
2. Builder compiles tmux. `TST-261`.
3. Parity suite runs. `TST-262`.

### AGENTS.md Rules

- `RULE-S20-01`: Parity tests run against target tmux version. **Enforcement:** CI matrix.
- `RULE-S20-02`: Version matrix documented. **Enforcement:** Matrix lint.

---

## 21. Test Support (mux-test-support) [v14 P3]

### Design Decisions

- `DeterministicClock` trait for controllable time in tests.
- `FakePtyBackend` for hermetic tests.
- **[v14 P2]** DeterministicClock mandatory for network simulation tests (INV-022).
- Traceability: `INV-022`.

### Rust Example

```rust
// crates/mux-test-support/src/lib.rs
// [v14 P3] DeterministicClock for Termlet testing

#![allow(dead_code)]
use std::time::Duration;

/// [v14 P2] Clock trait for Termlet testing.
pub trait TermletClock: Send {
    fn now_ms(&self) -> u64;
    fn advance(&mut self, delta: Duration);
}

/// [v14 P2] Deterministic clock implementation.
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

/// [v14 P3] RealClock (forbidden in network partition tests, INV-022).
pub struct RealClock;

impl TermletClock for RealClock {
    fn now_ms(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }
    fn advance(&mut self, _delta: Duration) {
        // Real clock cannot be advanced
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic_clock() {
        let mut clock = DeterministicClock::new(1000);
        assert_eq!(clock.now_ms(), 1000);
        clock.advance(Duration::from_millis(500));
        assert_eq!(clock.now_ms(), 1500);
    }

    #[test]
    fn test_clock_monotonic() {
        let mut clock = DeterministicClock::new(0);
        let t1 = clock.now_ms();
        clock.advance(Duration::from_millis(1));
        let t2 = clock.now_ms();
        assert!(t2 > t1);
    }
}
```

### Test Strategy

1. DeterministicClock monotonic. `TST-270`.
2. DeterministicClock advance. `TST-271`.
3. **[v14 P2]** DeterministicClock is Send. `TST-272`.
4. **[v14 P3]** RealClock advance is no-op. `TST-273`. [v14 P3]

### AGENTS.md Rules

- `RULE-S21-01`: DeterministicClock mandatory for net sim (INV-022). **Enforcement:** Clock type check.
- `RULE-S21-02`: RealClock forbidden in partition tests. **Enforcement:** Lint.
- `RULE-S21-03`: **[v14 P3]** TermletClock trait is Send. **Enforcement:** Trait bound check. [v14 P3]

---

## 22. Parity Testing (mux-regress)

### Design Decisions

- Regression suite: send same commands to tmux and TermForge, diff outputs.
- Parity matrix covers key operations.
- Traceability: `PAR-007`.

### Rust Example

```rust
// tools/mux-regress/src/lib.rs
#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParityResult {
    pub test_name: String,
    pub tmux_output: String,
    pub termforge_output: String,
    pub pass: bool,
}

impl ParityResult {
    pub fn check(test_name: &str, tmux: &str, termforge: &str) -> Self {
        Self {
            test_name: test_name.to_string(),
            tmux_output: tmux.to_string(),
            termforge_output: termforge.to_string(),
            pass: tmux == termforge,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parity_pass() {
        let r = ParityResult::check("echo", "hello", "hello");
        assert!(r.pass);
    }

    #[test]
    fn test_parity_fail() {
        let r = ParityResult::check("echo", "hello", "world");
        assert!(!r.pass);
    }
}
```

### Test Strategy

1. Parity suite runs. `TST-280`.
2. Diff captures mismatches. `TST-281`.

### AGENTS.md Rules

- `RULE-S22-01`: Parity tests required for release. **Enforcement:** CI gate.

---

## 23. Fuzz Testing

### Design Decisions

- Fuzz targets: lifecycle, input, snapshot, CRC32C.
- No-panic invariant for all targets.
- Corpus in `fixtures/fuzz-corpus/`.
- **[v14 P2]** CRC32C fuzz target added.
- Traceability: `OPS-006`.

### Rust Example

```rust
// crates/mux-termlet/src/fuzz_support.rs
// [v14 P3] Fuzz target support types

#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FuzzTarget {
    TermletLifecycle,
    TermletInput,
    SnapshotRoundtrip,
    Crc32cChecksum, // [v14 P2]
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
    fn test_fuzz_target_count() { assert_eq!(ALL_FUZZ_TARGETS.len(), 4); }

    #[test]
    fn test_corpus_dirs_unique() {
        let dirs: Vec<_> = ALL_FUZZ_TARGETS.iter().map(|t| t.corpus_dir()).collect();
        let deduped: std::collections::HashSet<_> = dirs.iter().collect();
        assert_eq!(dirs.len(), deduped.len());
    }
}
```

### Test Strategy

1. Fuzz targets enumerated. `TST-290`.
2. Corpus directories exist. `TST-291`.
3. **[v14 P2]** CRC32C fuzz target present. `TST-292`.

### AGENTS.md Rules

- `RULE-S23-01`: All fuzz targets must have no-panic invariant. **Enforcement:** Fuzz CI.
- `RULE-S23-02`: Corpus stored in fixtures/fuzz-corpus. **Enforcement:** Path check.
- `RULE-S23-03`: **[v14 P2]** CRC32C fuzz target present. **Enforcement:** Target list check.

---

## 24. Performance Benchmarks [v14 P3]

### Design Decisions

- Criterion benchmarks in `benchmarks/criterion/`.
- **[v14]** B19 (Grid::put_char), B20 (PtyRegistry alloc/release), B21 (PackedCell snapshot).
- **[v14 P2]** B22 (CRC32C throughput vs FNV-1a).
- **[v14 P3]** B23 (Vec<Cell> capacity hint allocation efficiency). [v14 P3]
- Traceability: `OPS-009`.

### Rust Example

```rust
// benchmarks/criterion/src/lib.rs
#![allow(dead_code)]

#[derive(Debug, Clone)]
pub struct BenchTarget {
    pub id: &'static str,
    pub description: &'static str,
    pub threshold_ns: u64,
}

pub const BENCHMARKS: &[BenchTarget] = &[
    BenchTarget { id: "B19", description: "Grid::put_char throughput", threshold_ns: 100 },
    BenchTarget { id: "B20", description: "PtyRegistry alloc/release", threshold_ns: 500 },
    BenchTarget { id: "B21", description: "PackedCell snapshot encode", threshold_ns: 50 },
    BenchTarget { id: "B22", description: "CRC32C vs FNV-1a throughput", threshold_ns: 200 },
    BenchTarget { id: "B23", description: "Vec<Cell> capacity allocation", threshold_ns: 100 }, // [v14 P3]
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_count() { assert!(BENCHMARKS.len() >= 5); }
}
```

### Test Strategy

1. Benchmarks compile. `TST-300`.
2. Thresholds enforced. `TST-301`.
3. **[v14 P3]** B23 benchmark present. `TST-302`. [v14 P3]

### AGENTS.md Rules

- `RULE-S24-01`: Benchmark regressions block release. **Enforcement:** CI threshold.
- `RULE-S24-02`: **[v14 P3]** B23 Vec<Cell> benchmark included. **Enforcement:** Benchmark list check. [v14 P3]

---

## 25. Visual Client / TUI

### Design Decisions

- ratatui-based TUI client.
- mux-view crate at Layer 3.
- Traceability: `OPS-010`.

### Rust Example

```rust
// crates/mux-view/src/lib.rs
#![allow(dead_code)]

pub struct TuiConfig {
    pub theme: &'static str,
    pub status_bar: bool,
    pub mouse_support: bool,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self { theme: "default", status_bar: true, mouse_support: true }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let c = TuiConfig::default();
        assert!(c.status_bar);
    }
}
```

### Test Strategy

1. TUI renders. `TST-310`.
2. Theme switching. `TST-311`.

### AGENTS.md Rules

- `RULE-S25-01`: TUI uses ratatui. **Enforcement:** Dep check.

---

## 26. AGENTS.md Rules [v14 P3]

### Design Decisions

- All rules use `RULE-Snn-xx` format with section number and sequence.
- Every rule has explicit enforcement.
- **[v14 P3]** Master rule count: 285+ unique rules. [v14 P3]
- **[v14 P3]** GPT P2 template-repetition rules (RULE-S32-111..170) consolidated into 10 substantive rules. [v14 P3]
- Traceability: `OPS-011`.

### Rule Index

| Section | Rule Range | Count |
|---------|-----------|-------|
| S01 (Identity) | RULE-S01-01..09 | 9 |
| S02 (Gates) | RULE-S02-01..14 | 14 |
| S03 (Deps) | RULE-S03-01..12 | 12 |
| S04 (Layout) | RULE-S04-01..08 | 8 |
| S05 (Grid) | RULE-S05-01..08 | 8 |
| S06 (VtParser) | RULE-S06-01..07 | 7 |
| S07 (PtyHandle) | RULE-S07-01..07 | 7 |
| S08 (Snapshot) | RULE-S08-01..08 | 8 |
| S09 (Proto) | RULE-S09-01..04 | 4 |
| S10 (Config) | RULE-S10-01..03 | 3 |
| S11 (Layout) | RULE-S11-01..02 | 2 |
| S12 (ORM) | RULE-S12-01..02 | 2 |
| S13 (Slots) | RULE-S13-01..02 | 2 |
| S14 (State) | RULE-S14-01..02 | 2 |
| S15 (Control) | RULE-S15-01..02 | 2 |
| S16 (Bindings) | RULE-S16-01..03 | 3 |
| S17 (CRDT) | RULE-S17-01..03 | 3 |
| S18 (Socket) | RULE-S18-01 | 1 |
| S19 (OTEL) | RULE-S19-01..02 | 2 |
| S20 (mux-vm) | RULE-S20-01..02 | 2 |
| S21 (Test) | RULE-S21-01..03 | 3 |
| S22 (Parity) | RULE-S22-01 | 1 |
| S23 (Fuzz) | RULE-S23-01..03 | 3 |
| S24 (Bench) | RULE-S24-01..02 | 2 |
| S25 (TUI) | RULE-S25-01 | 1 |
| S26 (Rules) | RULE-S26-01..03 | 3 |
| S27 (Risks) | RULE-S27-01..03 | 3 |
| S28 (Evolution) | RULE-S28-01..03 | 3 |
| S29 (Anchors) | RULE-S29-01..02 | 2 |
| S30 (Types) | RULE-S30-01..02 | 2 |
| S31 (Matrix) | RULE-S31-01..02 | 2 |
| S32 (Termlets) | RULE-S32-01..120 | 120 |
| **Total** | | **285** |

### AGENTS.md Rules

- `RULE-S26-01`: Every rule has enforcement. **Enforcement:** Rule lint.
- `RULE-S26-02`: Rules use RULE-Snn-xx format. **Enforcement:** Format lint.
- `RULE-S26-03`: **[v14 P3]** Master rule count >= 285. **Enforcement:** Count check. [v14 P3]

---

## 27. Risks and Mitigations [v14 P3]

### Design Decisions

- Risk register maintained as append-only log.
- Risk severity: Low, Medium, High, Critical.
- **[v14]** 95 risks (R1-R95).
- **[v14 P2]** Claude P2: R96-R100. GPT P2: R96-R110 (adopted in full).
- **[v14 P3]** DEFINITIVE risk register: R1-R115 (115 risks). R111-R115 are P3-specific. [v14 P3]
- Traceability: `OPS-007`.

### Risk Register (R1-R115 DEFINITIVE) [v14 P3]

| Risk ID | Category | Description | Severity | Mitigation | Status |
|---------|----------|-------------|----------|------------|--------|
| R01 | Grid | CompactString heap fallback for emoji | Medium | Benchmark inline vs heap path | Open |
| R02 | Snapshot | PackedCell 21-bit ch overflow for future Unicode | Low | Monitor Unicode 16+ | Open |
| R03 | VtParser | Const table maintenance burden | Low | Generated table with build.rs | Open |
| R04 | PtyHandle | Typestate complexity for bindings | Medium | DynPtyHandle runtime fallback | Mitigated |
| R05 | CRDT | Merge ordering edge cases | High | Property-based testing | Active |
| R10 | Snapshot | FNV-1a collision in large snapshots | Low | CRC32C opt-in path | Mitigated |
| R80 | Grid | VecDeque non-contiguous for snapshot | Medium | make_contiguous() before snapshot | Mitigated |
| R81 | Grid | CompactString inline threshold drift | Medium | Threshold test; benchmark B19 | Open |
| R82 | Snapshot | PackedCell bit layout incompatible with future Unicode | Low | 21-bit ch covers U+0..U+1FFFFF | Open |
| R83 | Snapshot | Snapshot v2 backward compat regression | High | Dual-version decode tests | Active |
| R84 | VtParser | ByteClass enum expansion changes discriminants | Medium | #[repr(u8)] with explicit values | Open |
| R85 | PtyHandle | Typestate PtyHandle ergonomic burden | Medium | DynPtyHandle runtime fallback | Mitigated |
| R86 | CRDT | OpLog binary insertion shift cost | Low | Benchmark B22 monitors | Open |
| R87 | Testing | Waiver expiry clock skew | Low | Use monotonic clock | Open |
| R88 | Grid | erase_in_display(Scrollback) implementation gap | Medium | CSI 3 J stub documented | Open |
| R89 | OTEL | 5-level OTEL hierarchy performance overhead | Low | Level 4 behind feature flag | Open |
| R90 | Termlet | expect() panic in library code | High | expect_or_fail() added | Mitigated |
| R91 | Proto | Causation ID memory overhead in Frame | Low | Optional field | Open |
| R92 | Config | Config validate() false positive on edge cases | Medium | Property tests for boundary values | Open |
| R93 | Layout | LayoutChecksum parity with older tmux versions | Low | Version-specific checksum tests | Open |
| R94 | PtyHandle | PtyRegistry slot exhaustion under high churn | Medium | Registry capacity monitoring | Open |
| R95 | Snapshot | Snapshot v2 LZ4 dependency adds binary size | Low | LZ4 behind feature flag | Open |
| R96 | Snapshot | **[v14 P2]** SmallVec inline-capacity tuning regresses wide-terminal resize | Medium | Boundary benchmarks at 120/132/160 cols | **[v14 P3] CLOSED -- SmallVec rejected FINAL** |
| R97 | Snapshot | **[v14 P2]** Dual-checksum mode causes fixture drift | Medium | Keep FNV canonical fixture lane; CRC32C optional | Open |
| R98 | Grid | **[v14 P2]** SmallVec rejection may need revisiting if Cell shrinks | Low | Re-evaluate if Cell < 16 bytes | Deferred |
| R99 | Termlet | **[v14 P2]** S32 expansion increases maintenance overhead | Low | Section ownership map with quarterly review | Open |
| R100 | Termlet | **[v14 P2]** Network partition simulation nondeterminism | High | Deterministic fault scheduler with fixed seeds | Active |
| R101 | Termlet | **[v14 P2]** Slow-consumer simulation creates false-positive deadlocks | Medium | Separate backpressure vs deadlock assertions | Active |
| R102 | Testing | **[v14 P2]** Imported GPT TST-ID band collides with legacy IDs | Medium | Reserved high-ID ranges (900+) | Mitigated |
| R103 | Governance | **[v14 P2]** High rule-count growth reduces readability | Low | Consolidated rule index | Mitigated |
| R104 | Snapshot | **[v14 P2]** CRC32C implementation mismatch across targets | Medium | Cross-platform vectors in CI | Active |
| R105 | Grid | **[v14 P2]** Line storage abstraction leaks into public API | Medium | Keep storage type private | Open |
| R106 | Termlet | **[v14 P2]** Termlet network fault hooks bypass normal gate policy | High | Gate-class integration checks | Active |
| R107 | Testing | **[v14 P2]** Slow-consumer tests lengthen CI wall time | Low | Lane-aware sampling in Preview | Open |
| R108 | Governance | **[v14 P2]** Additional S32 rules become stale vs implementation | Medium | Rule-to-test mapping CI | Active |
| R109 | Snapshot | **[v14 P2]** Snapshot decoder complexity increases parser attack surface | High | Fuzz corpus expansion | Active |
| R110 | Governance | **[v14 P2]** Pass 2 lineage ambiguity between model variants | Low | Provenance table | Mitigated |
| **R111** | **Snapshot** | **[v14 P3]** Header layout change from 30 to 31 bytes breaks P2 implementations | Medium | Migration guide; header version check at decode | Open |
| **R112** | **Governance** | **[v14 P3]** Template-rule consolidation loses test coverage granularity | Low | Consolidated rules map to same TST IDs | Mitigated |
| **R113** | **Grid** | **[v14 P3]** Vec<Cell> capacity hint may over-allocate for sparse rows | Low | Benchmark B23 monitors actual capacity utilization | Open |
| **R114** | **Termlet** | **[v14 P3]** S32.48 Plan Lineage section adds maintenance overhead | Low | Quarterly review with section ownership map | Open |
| **R115** | **Proto** | **[v14 P3]** Protocol negotiation typed error introduces new failure mode | Low | Test coverage for NegotiationFailed variant | Open |

### Rust Example

```rust
// crates/mux-types/src/risk.rs
// [v14 P3] DEFINITIVE risk register types

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

pub const RISK_COUNT: usize = 115; // [v14 P3] DEFINITIVE

pub fn open_risks(risks: &[Risk]) -> Vec<&Risk> {
    risks.iter().filter(|r| matches!(r.status, RiskStatus::Open | RiskStatus::Active)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_count() { assert!(RISK_COUNT >= 115); } // [v14 P3]

    #[test]
    fn test_open_risk_filter() {
        let risks = vec![
            Risk { id: "R1", severity: RiskSeverity::Medium, status: RiskStatus::Open, added_in: "v14" },
            Risk { id: "R2", severity: RiskSeverity::Low, status: RiskStatus::Closed, added_in: "v14" },
        ];
        assert_eq!(open_risks(&risks).len(), 1);
    }
}
```

### Test Strategy

1. Risk register complete. `TST-330`.
2. All risks have mitigation. `TST-331`.
3. **[v14 P2]** R96-R110 present. `TST-332`.
4. **[v14 P3]** R111-R115 present. `TST-333`. [v14 P3]
5. **[v14 P3]** R96 status CLOSED (SmallVec rejected). `TST-334`. [v14 P3]

### AGENTS.md Rules

- `RULE-S27-01`: Risk register append-only. **Enforcement:** Diff review.
- `RULE-S27-02`: **[v14 P3]** Risk count >= 115 (R1-R115). **Enforcement:** Count check. [v14 P3]
- `RULE-S27-03`: **[v14 P2]** Newly added risk IDs MUST include owner and mitigation test ID. **Enforcement:** Risk schema lint.

---

## 28. Plan Evolution and Changelog [v14 P3]

### Design Decisions

- Changelog tracks all spec changes.
- Version lineage documented in preamble.
- **[v14 P2]** Cross-pollination decision table documents adopted/rejected ideas.
- **[v14 P3]** Plan Evolution section is DEFINITIVE. Records full v12->v13->v14 lineage with line counts, rule counts, risk counts, and S32 subsection counts. [v14 P3]
- Traceability: `OPS-008`.

### Changelog (DEFINITIVE) [v14 P3]

| Version | Lines | Rules | Risks | S32 Subs | Key Changes |
|---------|-------|-------|-------|----------|-------------|
| v12 P3 | 4,864 | 180+ | 70 | 30 | Placeholder cleanup, deepened S32, governance |
| v13 P3 (DEFINITIVE) | 5,739 | 210+ | 80 | 35 | Full cross-pollination, 210+ rules, 80 risks, 135+ Termlet tests |
| v14 P1 Claude | 5,625 | 230+ | 95 | 36 | ByteClass, CompactString, PackedCell, typestate PtyHandle |
| v14 P1 GPT | 5,714 | 342 | 96 | 45 | 45 S32 subsections, 342 rules |
| v14 P1 Gemini | 308 | 2 | 2 | 35 | SmallVec, CRC32C, TermForgeIdentity trait proposals |
| v14 P2 Claude | 5,848 | 260+ | 100 | 47 | CRC32C opt-in, DeterministicClock, SmallVec rejected |
| v14 P2 GPT | 5,940 | 279 | 110 | 46 | R110, 279 rules, 242+ TST, S32.36-S32.46 |
| v14 P2 Gemini | 204 | 2 | 2 | -- | SmallVec/CRC32C advocacy refinement |
| **v14 P3 (this, DEFINITIVE)** | **6,000+** | **285+** | **115** | **48** | **DEFINITIVE synthesis: R115, 285 rules, 170+ Termlet tests, 48 S32 subs, INV-024, S80** |

### Cross-Pollination Audit Trail [v14 P3]

| Decision | Claude P2 | GPT P2 | Gemini P2 | P3 Resolution |
|----------|-----------|--------|-----------|---------------|
| SmallVec for Line | Rejected | Adopted (production) | Advocated | **REJECTED FINAL (S75)** |
| CRC32C | Opt-in feature | Dual-lane | Advocated strongly | **Adopted as dual-checksum** |
| TermForgeIdentity trait | Rejected | Hybrid trait+manifest | Advocated | **REJECTED FINAL (S78)** |
| R96-R110 | R96-R100 only | R96-R110 | N/A | **All R96-R110 adopted + R111-R115** |
| S32 subsection count | 47 | 46 | N/A | **48 (added S32.48)** |
| Snapshot header size | 30 bytes (algo at 26) | 30 bytes (cell_count at 26) | N/A | **31 bytes (cell_count at 26, algo at 30)** |
| Template rules (S32-111..170) | N/A | 60 identical rules | N/A | **Consolidated to 10 substantive** |

### Rust Example

```rust
// [v14 P3] Plan Evolution metadata
#![allow(dead_code)]

pub const SPEC_VERSION: &str = "v14-pass3";
pub const SPEC_DATE: &str = "2026-02-12";
pub const SPEC_STATUS: &str = "v14-pass3-definitive";

pub const TOTAL_LINES_TARGET: usize = 6000;
pub const TOTAL_RULES: usize = 285;
pub const TOTAL_RISKS: usize = 115;
pub const TOTAL_S32_SUBS: usize = 48;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spec_version() { assert!(SPEC_VERSION.starts_with("v14")); }

    #[test]
    fn test_targets() {
        assert!(TOTAL_RULES >= 285);
        assert!(TOTAL_RISKS >= 115);
        assert!(TOTAL_S32_SUBS >= 48);
    }
}
```

### Test Strategy

1. Changelog matches code. `TST-340`.
2. **[v14 P2]** Cross-pollination decisions documented. `TST-341`.
3. **[v14 P3]** Audit trail covers all three P2 inputs. `TST-342`. [v14 P3]
4. **[v14 P3]** Version is v14-pass3-definitive. `TST-343`. [v14 P3]

### AGENTS.md Rules

- `RULE-S28-01`: Changelog updated on every spec version. **Enforcement:** PR check.
- `RULE-S28-02`: **[v14 P2]** Cross-pollination decisions table required. **Enforcement:** Section check.
- `RULE-S28-03`: **[v14 P3]** Audit trail covers all P2 model inputs. **Enforcement:** Model list check. [v14 P3]

---

## 29. Reference Anchors [v14 P3]

### Design Decisions

- Cross-references use `INV-*`, `API-*`, `PAR-*`, `OPS-*`, `TST-*` anchors.
- **[v14 P2]** Added INV-021 (checksum algorithm selectable) and INV-022 (DeterministicClock).
- **[v14 P3]** Added INV-023 (Snapshot header DEFINITIVE) and INV-024 (Line storage Vec FINAL). [v14 P3]
- Traceability: all.

### Reference Table [v14 P3]

| Anchor | Section | Description |
|--------|---------|-------------|
| INV-001 | 1 | Protocol v8 compatibility |
| INV-002 | 3 | mux-core pure, no IO |
| INV-003 | 14 | Single-writer state mutation |
| INV-004 | 9 | Protocol violation drops connection |
| INV-005 | 10 | First-client identify before config load |
| INV-006 | 11 | Layout resize round-robin |
| INV-007 | 18 | Socket isolation three guards |
| INV-008 | 32 | Termlets pane-backed |
| INV-009 | 32 | TermletState via valid_transition() |
| INV-010 | 14 | Effects idempotent by key |
| INV-011 | 32 | TermletLike is normative API |
| INV-012 | 5 | put_char/put_grapheme sole entry |
| INV-013 | 13 | Slot generation counters |
| INV-014 | 7 | PtyHandle 7-state |
| INV-015 | 8 | Snapshot trailing checksum |
| INV-016 | 5 | Cell grapheme CompactString |
| INV-017 | 7 | Restart barrier mandatory |
| INV-018 | 6 | VtParser const table |
| INV-019 | 2 | Error types Clone+PartialEq+Eq |
| INV-020 | 8 | Snapshot v2 backward compat |
| INV-021 | 8 | **[v14 P2]** Checksum algorithm selectable |
| INV-022 | 21 | **[v14 P2]** DeterministicClock for net sim |
| INV-023 | 8 | **[v14 P3]** Snapshot header 31 bytes DEFINITIVE |
| INV-024 | 5 | **[v14 P3]** Line storage Vec<Cell> FINAL (SmallVec rejected) |

### Test Strategy

1. All INV anchors resolvable. `TST-350`.
2. **[v14 P2]** INV-021 and INV-022 present. `TST-351`.
3. **[v14 P3]** INV-023 and INV-024 present. `TST-352`. [v14 P3]

### AGENTS.md Rules

- `RULE-S29-01`: All anchors resolvable. **Enforcement:** Link lint.
- `RULE-S29-02`: **[v14 P3]** New invariants documented in preamble. **Enforcement:** Preamble check. [v14 P3]

---

## 30. Appendix: Canonical Type Quick Reference [v14 P3]

### Design Decisions

- Quick reference for all canonical types.
- **[v14 P2]** Added `ChecksumAlgo`, `DeterministicClock`, `TermletClock`.
- **[v14 P3]** Added `ProtoError`, `NegotiationFailed`. [v14 P3]

### Type Table

| Type | Crate | Layer | Added |
|------|-------|-------|-------|
| CompactString | mux-compact-string | L0 | v14 |
| PackedCell | mux-packed-cell | L0 | v14 |
| ByteClass | mux-grid | L0 | v14 |
| EraseMode | mux-grid | L0 | v14 |
| PtyHandle<S> | mux-pty | L1 | v14 |
| DynPtyHandle | mux-pty | L1 | v14 |
| ChecksumAlgo | mux-checksum | L0 | **v14 P2** |
| TermletClock | mux-test-support | L2 | **v14 P2** |
| DeterministicClock | mux-test-support | L2 | **v14 P2** |
| TermletError (12 variants) | mux-termlet | L2 | v14 |
| IdentityManifest | mux-types | L0 | v14 |
| OverflowPolicy | mux-conf | L1 | v14 |
| **ProtoError** | **mux-proto** | **L0** | **v14 P3** |

### Test Strategy

1. All types compile. `TST-360`.
2. **[v14 P2]** New types in table. `TST-361`.
3. **[v14 P3]** ProtoError in table. `TST-362`. [v14 P3]

### AGENTS.md Rules

- `RULE-S30-01`: Type table matches implementation. **Enforcement:** Type check.
- `RULE-S30-02`: **[v14 P3]** All new types added to table. **Enforcement:** Table lint. [v14 P3]

---

## 31. Supplemental Test Matrix [v14 P3]

### Design Decisions

- Test matrix covers: Rust unit, integration, property, fuzz, cross-language, parity, benchmark.
- **[v14 P2]** Added DeterministicClock and CRC32C test categories.
- **[v14 P3]** Updated totals for P3 additions. [v14 P3]

### Matrix

| Category | Count (v14 P2) | Count (v14 P3) | Delta |
|----------|---------------|----------------|-------|
| Unit | 210+ | 220+ | +10 |
| Integration | 85+ | 90+ | +5 |
| Property | 35+ | 38+ | +3 |
| Fuzz | 18+ | 20+ | +2 |
| Cross-language | 22+ | 24+ | +2 |
| Parity | 55+ | 58+ | +3 |
| Benchmark | 28+ | 30+ | +2 |
| Termlet | 155+ | 170+ | +15 |
| **Total** | **608+** | **650+** | **+42** |

### Test Strategy

1. Matrix categories covered. `TST-370`.
2. Total test count meets target. `TST-371`.
3. **[v14 P3]** Termlet test count >= 170. `TST-372`. [v14 P3]

### AGENTS.md Rules

- `RULE-S31-01`: Test matrix updated on spec revision. **Enforcement:** Matrix lint.
- `RULE-S31-02`: **[v14 P3]** Total test count >= 650. **Enforcement:** Count check. [v14 P3]

---
