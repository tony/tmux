# TermForge v11 Architecture Specification Pass 2 Codex

This Pass 2 spec cross pollinates the three Pass 1 variants, resolves conflicts, and normalizes all 32 sections to the required four part format.

## 1. Project Identity
### Design Decisions
- [v11 P2] TermForge is a Rust first tmux compatible multiplexer with SDK first automation for Rust Python and Node.
- [v11 P2] Product strategy is parity plus programmable Termlets as first class API.
- [v11 P2] Canonical binary is termforge with alias tf and crate namespace mux-.

### Rust Example
~~~rust
pub const PROJECT_NAME: &str = "termforge";
pub const CLI_ALIAS: &str = "tf";
pub const CRATE_PREFIX: &str = "mux-";
~~~

### Test Strategy
1. TST-001 binary name check for termforge.
2. TST-002 workspace crate prefix check for mux-.
3. TST-003 alias execution check for tf.

### AGENTS.md Rules
- RULE-S01-01: All workspace crates must use mux- prefix. Enforcement: CI Cargo.toml scanner rejects non matching names.
- RULE-S01-02: CLI must expose termforge and tf. Enforcement: integration test executes both commands.

## 2. Acceptance Criteria and Gates
### Design Decisions
- [v11 P2] Gate model is measurable across parity reliability performance and ecosystem.
- [v11 P2] Every merged feature maps to at least one gate and at least one automated test.
- [v11 P2] Added Termlet specific gates for pool orchestration snapshot diff and drain latency.

### Rust Example
~~~rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Gate {
    P10TermletPoolBatch,
    P11SnapshotDiffFidelity,
    B13DrainOutputLatency,
    E3BindingSnapshotParity,
}
~~~

### Test Strategy
1. TST-010 gate to test mapping completeness.
2. TST-011 CI failure when gate threshold is missing.
3. TST-012 PR metadata check for declared gates.

### AGENTS.md Rules
- RULE-S02-01: Each feature PR must declare gate IDs. Enforcement: PR template plus CI metadata validator.
- RULE-S02-02: Gates are append only. Enforcement: CI diff guard on gate enum modifications.

## 3. Crate Dependency Rules
### Design Decisions
- [v11 P2] Layering remains strict with L0 core L1 adapters L2 facades.
- [v11 P2] mux-termlet cannot depend on mux-server mux-api or mux-orm.
- [v11 P2] PtyBackend requires Send plus Any for safe downcast in test support.

### Rust Example
~~~rust
pub trait PtyBackend: Send + std::any::Any {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}
~~~

### Test Strategy
1. TST-020 cargo tree deny list for forbidden dependency edges.
2. TST-021 compile check verifies PtyBackend supertraits.
3. TST-022 termlet dependency policy regression test.

### AGENTS.md Rules
- RULE-S03-01: Layer violations are build blocking. Enforcement: cargo deny plus dependency graph policy job.
- RULE-S03-02: PtyBackend must include Send and Any bounds. Enforcement: compile fail test when bounds are removed.

## 4. Workspace Layout
### Design Decisions
- [v11 P2] Workspace keeps stable crate paths and dedicated tool crates.
- [v11 P2] examples directory is mandatory for SDK onboarding and docs parity.
- [v11 P2] Benchmark dependencies are isolated from runtime crates.

### Rust Example
~~~rust
pub const WORKSPACE_MEMBERS: &[&str] = &[
    "crates/mux-core",
    "crates/mux-termlet",
    "bindings/python",
    "bindings/node",
    "tools/format-audit",
    "tools/tmux-builder",
];
~~~

### Test Strategy
1. TST-030 workspace member existence check.
2. TST-031 cargo build examples in CI.
3. TST-032 benchmark crate isolation test.

### AGENTS.md Rules
- RULE-S04-01: New crate path changes require layer declaration. Enforcement: manifest diff lint.
- RULE-S04-02: Examples must compile in CI. Enforcement: dedicated examples pipeline stage.

## 5. Core State Graph
### Design Decisions
- [v11 P2] GraphState read path is immutable and writes happen only through reducer events.
- [v11 P2] Entity stores use HashMap for O1 lookup.
- [v11 P2] ApplyOutcome carries non fatal warnings to avoid panic flow.

### Rust Example
~~~rust
use std::collections::HashMap;

pub struct GraphState {
    pub sessions: HashMap<SessionId, Session>,
    pub windows: HashMap<WindowId, Window>,
    pub panes: HashMap<PaneId, Pane>,
}
~~~

### Test Strategy
1. TST-040 reducer invariant property tests.
2. TST-041 O1 lookup regression benchmark.
3. TST-042 warning path validation for non fatal mutations.

### AGENTS.md Rules
- RULE-S05-01: No direct graph mutation outside reducer. Enforcement: static grep plus review checklist.
- RULE-S05-02: Entity storage stays keyed map based. Enforcement: architecture lint over GraphState shape.

## 6. Entity Relationships
### Design Decisions
- [v11 P2] Relationships are explicit with typed IDs and reverse maps.
- [v11 P2] Destroy cascades are deterministic and effect emitted.
- [v11 P2] Traversal APIs avoid full graph scans for hot operations.

### Rust Example
~~~rust
pub struct Edges {
    pub pane_to_window: std::collections::HashMap<PaneId, WindowId>,
    pub window_to_session: std::collections::HashMap<WindowId, SessionId>,
}
~~~

### Test Strategy
1. TST-050 exact effect count on cascade delete.
2. TST-051 orphan detection after random create destroy sequences.
3. TST-052 relationship property test for referential integrity.

### AGENTS.md Rules
- RULE-S06-01: All entity IDs must be strong typed keys. Enforcement: compile checks disallow raw u64 in graph APIs.
- RULE-S06-02: Hot path traversals cannot use full collection scan. Enforcement: perf test plus review gate.

## 7. Event and Effect Enums
### Design Decisions
- [v11 P2] Core emits typed effects and runtime executes side effects.
- [v11 P2] Event application remains deterministic and replay safe.
- [v11 P2] Invalid transitions return typed error not panic.

### Rust Example
~~~rust
pub enum Effect {
    SpawnProcess { pane: PaneId },
    KillProcess { pane: PaneId, signal: i32 },
    NotifyClient { client: ClientId, payload: Vec<u8> },
}
~~~

### Test Strategy
1. TST-060 deterministic replay test.
2. TST-061 exhaustive match compile guard.
3. TST-062 invalid transition error variant check.

### AGENTS.md Rules
- RULE-S07-01: Reducer cannot perform IO. Enforcement: crate level lint blocks IO dependencies in reducer modules.
- RULE-S07-02: Public event effect enums are append only. Enforcement: API compatibility CI check.

## 8. Error Handling
### Design Decisions
- [v11 P2] Library errors use typed enums with thiserror and binary edges may wrap with anyhow.
- [v11 P2] ErrorCode is mandatory for all errors crossing binding boundaries.
- [v11 P2] Error domains remain separated for protocol core query and termlet.

### Rust Example
~~~rust
pub trait ErrorCode {
    fn code(&self) -> &str;
}

#[derive(thiserror::Error, Debug)]
pub enum ProtocolError {
    #[error("payload too large")]
    PayloadTooLarge,
}

impl ErrorCode for ProtocolError {
    fn code(&self) -> &str {
        "PROTOCOL_PAYLOAD_TOO_LARGE"
    }
}
~~~

### Test Strategy
1. TST-070 compile tests for ErrorCode implementations.
2. TST-071 stable code string semver guard.
3. TST-072 binding exception serialization includes canonical code.

### AGENTS.md Rules
- RULE-S08-01: Public library APIs cannot expose anyhow error types. Enforcement: static source lint.
- RULE-S08-02: Cross binding errors must implement ErrorCode. Enforcement: compile fail harness and binding integration checks.
## 9. Wire Protocol
### Design Decisions
- [v11 P2] Protocol frames are length prefixed and versioned with strict payload bounds.
- [v11 P2] Decode corruption is fatal for the connection path.
- [v11 P2] Compatibility policy is explicit with reject paths for unsupported versions.

### Rust Example
~~~rust
pub const MAX_PAYLOAD_SIZE: usize = 64 * 1024;

pub enum DecodeOutcome {
    Message(Vec<u8>),
    Fatal(ProtocolError),
}
~~~

### Test Strategy
1. TST-080 fuzz and property roundtrip for codec.
2. TST-081 oversized payload fatal path.
3. TST-082 version compatibility matrix.

### AGENTS.md Rules
- RULE-S09-01: Malformed frames must terminate decode session. Enforcement: integration test validates fatal close.
- RULE-S09-02: Payload size must be bounded by MAX_PAYLOAD_SIZE. Enforcement: codec unit tests plus fuzz corpus.

## 10. Configuration Engine
### Design Decisions
- [v11 P2] Config loading occurs after identify burst to match tmux lifecycle semantics.
- [v11 P2] Option fallback order is fixed pane to window to session to global.
- [v11 P2] Parsing rejects ambiguous definitions and duplicate key conflicts.

### Rust Example
~~~rust
pub fn resolve_option(
    key: &str,
    pane: &Pane,
    window: &Window,
    session: &Session,
    global: &Global,
) -> Option<String> {
    pane.get(key)
        .or_else(|| window.get(key))
        .or_else(|| session.get(key))
        .or_else(|| global.get(key))
}
~~~

### Test Strategy
1. TST-090 startup sequencing integration test.
2. TST-091 fallback order property tests.
3. TST-092 duplicate key error path tests.

### AGENTS.md Rules
- RULE-S10-01: Config load trigger remains post identify. Enforcement: startup integration contract test.
- RULE-S10-02: Option fallback order is immutable unless spec amended. Enforcement: property tests with fixture corpus.

## 11. Layout Engine
### Design Decisions
- [v11 P2] All layout mutators preserve invariants on bounds and minimum pane size.
- [v11 P2] Debug assertions for layout_check remain active in CI debug mode.
- [v11 P2] Format expansion parity is validated by dedicated format audit corpus.

### Rust Example
~~~rust
pub fn split_vertical(root: &mut LayoutNode) {
    debug_assert!(layout_check(root));
    root.split_vertical();
    debug_assert!(layout_check(root));
}
~~~

### Test Strategy
1. TST-100 proptest for random split and resize sequences.
2. TST-101 debug assertion CI run.
3. TST-102 format audit parity checks.

### AGENTS.md Rules
- RULE-S11-01: Layout mutators must assert invariants in debug builds. Enforcement: CI with debug assertions enabled.
- RULE-S11-02: Format parity corpus must pass before release. Enforcement: format audit release gate.

## 12. ORM Query Layer
### Design Decisions
- [v11 P2] Query layer is graph oriented and avoids SQL style ad hoc modeling.
- [v11 P2] Query operations are typed and deterministic.
- [v11 P2] Output ordering and filtering semantics are stable across bindings.

### Rust Example
~~~rust
pub enum QueryOp {
    Sessions,
    WindowsBySession(SessionId),
    PanesByWindow(WindowId),
}
~~~

### Test Strategy
1. TST-110 deterministic query ordering tests.
2. TST-111 invalid query op typed error tests.
3. TST-112 cross language query parity snapshots.

### AGENTS.md Rules
- RULE-S12-01: Query APIs must stay graph native. Enforcement: review checklist and lint against SQL style helpers.
- RULE-S12-02: Documented query order guarantees must be test enforced. Enforcement: snapshot and golden tests.

## 13. Concurrency Model
### Design Decisions
- [v11 P2] Single writer actor owns all mutable graph transitions.
- [v11 P2] Readers consume immutable snapshots via ArcSwap.
- [v11 P2] Ordering and visibility are verified by model tests.

### Rust Example
~~~rust
pub struct StateHandle {
    inner: arc_swap::ArcSwap<GraphState>,
}
~~~

### Test Strategy
1. TST-130 message ordering model tests.
2. TST-131 ArcSwap freshness checks after commit.
3. TST-132 multi reader stress run.

### AGENTS.md Rules
- RULE-S13-01: Only actor loop mutates canonical state. Enforcement: architecture tests and code ownership checks.
- RULE-S13-02: Read path cannot use Arc Mutex wrapping graph snapshots. Enforcement: static grep plus review gate.

## 14. Server Lifecycle
### Design Decisions
- [v11 P2] Startup sequence is lock then socket then identify then config then serve.
- [v11 P2] Lock and process cleanup use RAII drop guards.
- [v11 P2] Shutdown supports bounded graceful timeout before forced teardown.

### Rust Example
~~~rust
pub struct LockGuard(std::fs::File);

impl Drop for LockGuard {
    fn drop(&mut self) {
        let _ = fs2::FileExt::unlock(&self.0);
    }
}
~~~

### Test Strategy
1. TST-140 startup order integration.
2. TST-141 crash mid startup lock cleanup recovery.
3. TST-142 graceful shutdown timeout coverage.

### AGENTS.md Rules
- RULE-S14-01: Lock management must be RAII based. Enforcement: grep rejects manual unlock choreography.
- RULE-S14-02: Lifecycle startup order is contract stable. Enforcement: startup integration fixture.

## 15. Control Mode
### Design Decisions
- [v11 P2] Control notifications are hints and clients must reconcile using refresh queries.
- [v11 P2] Parser must tolerate fragmented frames and burst delivery.
- [v11 P2] Eventual consistency is an explicit requirement under notification loss.

### Rust Example
~~~rust
pub enum ControlNotification {
    WindowChanged(WindowId),
    PaneOutput(PaneId),
}
~~~

### Test Strategy
1. TST-150 dropped notification recovery test.
2. TST-151 fragmented frame parser tests.
3. TST-152 eventual consistency convergence test.

### AGENTS.md Rules
- RULE-S15-01: Notifications are non authoritative hints only. Enforcement: integration tests validate refresh correctness.
- RULE-S15-02: Parser must support partial frame input. Enforcement: parser fuzz tests.

## 16. Language Bindings
### Design Decisions
- [v11 P2] Bindings depend on mux-api and optional mux-orm but not mux-core.
- [v11 P2] Error translation must preserve canonical error codes.
- [v11 P2] Binding API parity matrix includes Termlet lifecycle methods.

### Rust Example
~~~rust
pub struct BindingError {
    pub code: String,
    pub message: String,
}
~~~

### Test Strategy
1. TST-160 API surface parity tests for Rust Python Node.
2. TST-161 error code propagation tests in Python and Node.
3. TST-162 package install and smoke tests for pip and npm.

### AGENTS.md Rules
- RULE-S16-01: Bindings cannot directly depend on mux-core. Enforcement: dependency graph check.
- RULE-S16-02: Binding exceptions must include canonical error code. Enforcement: integration tests inspect exception objects.
## 17. CRDT Transaction Layer
### Design Decisions
- [v11 P2] CRDT layer targets eventual consistency between replicated instances.
- [v11 P2] Transaction metadata includes logical clock and actor identifiers.
- [v11 P2] Merge behavior is deterministic and tombstone aware.

### Rust Example
~~~rust
pub struct Hlc {
    pub wall_ms: u64,
    pub counter: u32,
    pub actor: u32,
}
~~~

### Test Strategy
1. TST-170 merge commutativity property tests.
2. TST-171 tombstone retention and compaction tests.
3. TST-172 cross node conflict convergence simulation.

### AGENTS.md Rules
- RULE-S17-01: CRDT merges must be deterministic across node orderings. Enforcement: property tests over randomized merge orders.
- RULE-S17-02: Tombstones cannot be dropped before safe horizon. Enforcement: retention policy integration tests.

## 18. Security Model
### Design Decisions
- [v11 P2] Local socket permissions are hardened by default.
- [v11 P2] Sensitive data never appears in telemetry attributes or default logs.
- [v11 P2] Permission failures are explicit typed errors not silent fallback.

### Rust Example
~~~rust
pub struct PermissionModel {
    pub socket_dir_mode: u32,
    pub socket_mode: u32,
}

pub const DEFAULT_PERMISSION_MODEL: PermissionModel = PermissionModel {
    socket_dir_mode: 0o700,
    socket_mode: 0o600,
};
~~~

### Test Strategy
1. TST-180 filesystem permission mode tests.
2. TST-181 auth and handshake failure tests.
3. TST-182 redaction tests for logs and telemetry.

### AGENTS.md Rules
- RULE-S18-01: Socket directory and socket file must default to 0700 and 0600. Enforcement: integration test stat checks.
- RULE-S18-02: Secrets must be redacted from logs and spans. Enforcement: snapshot tests over emitted telemetry.

## 19. OpenTelemetry
### Design Decisions
- [v11 P2] Dual provider model remains server provider and client provider with isolated lifecycle.
- [v11 P2] Shutdown path uses bounded timeout to prevent process hang.
- [v11 P2] Added Termlet span contract with required attributes termlet_id pane_id backend_mode.

### Rust Example
~~~rust
pub fn span_termlet_wait(termlet_id: u64, pane_id: u64, backend_mode: &str) {
    tracing::info!(
        target: "termforge.termlet",
        termlet_id,
        pane_id,
        backend_mode,
        "termlet.wait_for"
    );
}
~~~

### Test Strategy
1. TST-190 provider init and shutdown lifecycle tests.
2. TST-191 span attribute presence tests.
3. TST-192 telemetry off mode overhead baseline.

### AGENTS.md Rules
- RULE-S19-01: Server and client telemetry providers stay isolated. Enforcement: architecture tests validate separate init paths.
- RULE-S19-02: Termlet spans must include required attributes. Enforcement: telemetry fixture assertions.

## 20. tmux Version Management
### Design Decisions
- [v11 P2] tmux build cache keys include version platform and feature fingerprint.
- [v11 P2] Build publication is atomic rename never copy.
- [v11 P2] Parallel builds use file lock guard and deterministic temporary sibling directory.

### Rust Example
~~~rust
pub fn compute_cache_key(version: &str, host: &str, os: &str, cfg12: &str) -> String {
    format!("tmux-{}__{}__{}__cfg{}", version, host, os, cfg12)
}
~~~

### Test Strategy
1. TST-200 cache key uniqueness tests.
2. TST-201 lock contention tests for parallel builder runs.
3. TST-202 atomic publication rollback tests.

### AGENTS.md Rules
- RULE-S20-01: Build publication must use atomic rename. Enforcement: static lint forbids copy based publish flow.
- RULE-S20-02: Lock file guard must release on drop. Enforcement: fault injection tests verify unlock on early return.

## 21. Test Support and Fake PTY
### Design Decisions
- [v11 P2] Fake PTY backend is deterministic and scenario driven.
- [v11 P2] ScenarioRecorder JSON format is canonical fixture format.
- [v11 P2] Three layer socket validation remains required in test harnesses.

### Rust Example
~~~rust
pub struct ScenarioRecorder {
    pub events: Vec<FakePtyEvent>,
}

impl ScenarioRecorder {
    pub fn record(&mut self, event: FakePtyEvent) {
        self.events.push(event);
    }
}
~~~

### Test Strategy
1. TST-210 deterministic replay tests for fake PTY event logs.
2. TST-211 scenario recorder serialization compatibility tests.
3. TST-212 path guard tests for socket safety in harness.

### AGENTS.md Rules
- RULE-S21-01: Fake PTY tests must be deterministic and timing independent. Enforcement: flaky test detector with repeated runs.
- RULE-S21-02: ScenarioRecorder JSON schema is contract stable. Enforcement: schema validation in CI.

## 22. Binding Test Frameworks
### Design Decisions
- [v11 P2] Python and Node test harnesses must isolate config and enforce cleanup.
- [v11 P2] Termlet tests execute in real and fake backend modes.
- [v11 P2] Resource leak detection for child processes is mandatory.

### Rust Example
~~~rust
pub struct BindingFixtureConfig {
    pub use_fake_pty: bool,
    pub null_config: bool,
}
~~~

### Test Strategy
1. TST-220 Python fixture cleanup tests for context manager semantics.
2. TST-221 Node fixture cleanup tests for finally semantics.
3. TST-222 dual mode parameterized binding runs.

### AGENTS.md Rules
- RULE-S22-01: Binding fixtures must auto cleanup spawned Termlets. Enforcement: leak detector checks after each suite.
- RULE-S22-02: Binding suites must run real and fake modes. Enforcement: CI matrix includes both modes.

## 23. Test Framework and Harness Design
### Design Decisions
- [v11 P2] Test taxonomy is explicit unit integration property fuzz snapshot perf.
- [v11 P2] Naming convention is test_component_behavior.
- [v11 P2] Rule coverage report must map each RULE to at least one automated check.

### Rust Example
~~~rust
pub enum TestKind {
    Unit,
    Integration,
    Property,
    Fuzz,
    Snapshot,
    Perf,
}
~~~

### Test Strategy
1. TST-230 naming convention lint tests.
2. TST-231 rule to test coverage report generation.
3. TST-232 nightly fuzz corpus growth checks.

### AGENTS.md Rules
- RULE-S23-01: Every rule needs at least one automated enforcement test. Enforcement: rule coverage report gate.
- RULE-S23-02: New tests must declare TestKind metadata. Enforcement: test macro lint.

## 24. Performance Targets
### Design Decisions
- [v11 P2] Performance budgets are defined for spawn snapshot wait and drain paths.
- [v11 P2] Criterion benchmarks are warn on PR and gate on release thresholds.
- [v11 P2] Termlet fake spawn and snapshot latency are tracked as headline KPIs.

### Rust Example
~~~rust
pub struct PerfBudget {
    pub metric: String,
    pub target_us: u64,
    pub warn_us: u64,
}
~~~

### Test Strategy
1. TST-240 criterion baseline capture and compare.
2. TST-241 release threshold gate for critical metrics.
3. TST-242 regression alert for drain_output latency.

### AGENTS.md Rules
- RULE-S24-01: Critical performance metrics require numeric thresholds. Enforcement: release script parses benchmark output.
- RULE-S24-02: Benchmark baselines must be versioned. Enforcement: CI checks baseline artifact updates.
## 25. Visual Client / TUI
### Design Decisions
- [v11 P2] View model is pure and snapshot testable.
- [v11 P2] UI rendering pipeline remains non blocking with optional debug panels.
- [v11 P2] Theme and layout output preserve parity constraints where declared.

### Rust Example
~~~rust
pub struct ViewModel {
    pub status_line: String,
    pub pane_titles: Vec<String>,
}
~~~

### Test Strategy
1. TST-250 view model snapshot tests.
2. TST-251 renderer non blocking checks under high event rate.
3. TST-252 optional debug panel feature gate tests.

### AGENTS.md Rules
- RULE-S25-01: View model construction must be pure from immutable snapshot input. Enforcement: unit tests avoid runtime dependencies.
- RULE-S25-02: Debug panels are optional and non blocking. Enforcement: feature flag tests and perf checks.

## 26. AGENTS.md Rules
### Design Decisions
- [v11 P2] Rule naming is normalized to RULE-Snn-xx across all sections with unique IDs.
- [v11 P2] Every rule must include explicit enforcement mechanism and at least one mapped test.
- [v11 P2] Global consistency pass validates rule IDs, section prefixes, uniqueness, and missing enforcement text.

### Rust Example
~~~rust
pub struct RuleSpec {
    pub id: String,
    pub text: String,
    pub enforcement: String,
}

pub fn validate_rule_id(id: &str) -> bool {
    let bytes = id.as_bytes();
    bytes.len() == 11
        && &id[0..7] == "RULE-S"
        && id[9..10] == "-"
        && id[7..9].chars().all(|c| c.is_ascii_digit())
        && id[10..11].chars().all(|c| c.is_ascii_digit())
}
~~~

### Test Strategy
1. TST-260 parse all section rules and verify RULE-Snn-xx format.
2. TST-261 verify no duplicate rule IDs and prefix matches section number.
3. TST-262 fail if any rule line lacks Enforcement clause.

### AGENTS.md Rules
- RULE-S26-01: Rules are immutable identifiers after publication. Enforcement: CI diff gate rejects text mutation without changelog entry.
- RULE-S26-02: Every rule must define enforcement in the same section. Enforcement: markdown parser CI check.

## 27. Risks and Mitigations
### Design Decisions
- [v11 P2] Risk register includes architecture runtime test and release risks.
- [v11 P2] Each high severity risk has a mitigation owner and automated sentinel test where possible.
- [v11 P2] Termlet leak and blocking IO risks are elevated to high visibility release blockers.

### Rust Example
~~~rust
pub struct RiskItem {
    pub id: String,
    pub severity: String,
    pub mitigation: String,
}
~~~

### Test Strategy
1. TST-270 risk sentinel suite for high severity items.
2. TST-271 quarterly risk review automation report.
3. TST-272 release blocker check for unresolved high risk items.

### AGENTS.md Rules
- RULE-S27-01: High severity unresolved risks block release. Enforcement: release checklist gate.
- RULE-S27-02: Each risk requires explicit mitigation owner. Enforcement: risk table lint.

## 28. Plan Evolution and Changelog
### Design Decisions
- [v11 P2] Changelog entries must include date rationale and impacted sections.
- [v11 P2] Pass evolution follows additive refinement and conflict resolution notes.
- [v11 P2] Rule changes require linked migration notes and test impact summary.

### Rust Example
~~~rust
pub struct ChangelogEntry {
    pub date_iso: String,
    pub summary: String,
    pub sections: Vec<u8>,
}
~~~

### Test Strategy
1. TST-280 changelog schema validation.
2. TST-281 section reference integrity tests.
3. TST-282 rule change migration note presence test.

### AGENTS.md Rules
- RULE-S28-01: Every spec update must include changelog entry. Enforcement: CI requires date stamped entry in section 28.
- RULE-S28-02: Rule modifications require migration notes. Enforcement: CI scan for linked migration identifiers.

## 29. Reference Anchors
### Design Decisions
- [v11 P2] Compatibility claims require source anchors to tmux behavior or internal canonical references.
- [v11 P2] Anchor IDs are stable and referenced in tests where practical.
- [v11 P2] Unknown anchor references are CI failures.

### Rust Example
~~~rust
pub struct Anchor {
    pub id: String,
    pub source: String,
    pub note: String,
}
~~~

### Test Strategy
1. TST-290 anchor existence and uniqueness checks.
2. TST-291 compatibility claim to anchor link validation.
3. TST-292 stale anchor detector for moved references.

### AGENTS.md Rules
- RULE-S29-01: Behavioral parity claims must cite anchors. Enforcement: docs lint requires anchor tags.
- RULE-S29-02: Anchor IDs are append only. Enforcement: CI diff checker.

## 30. Appendix Canonical Type Quick Reference
### Design Decisions
- [v11 P2] Canonical type table defines names ownership and crate boundaries.
- [v11 P2] Shared types avoid duplicate parallel definitions across crates.
- [v11 P2] Termlet specific types are explicitly separated from server types.

### Rust Example
~~~rust
pub struct TermletPaneId(pub u64);
pub struct PaneId(pub u64);
~~~

### Test Strategy
1. TST-300 canonical type registry generation test.
2. TST-301 duplicate type detector across crates.
3. TST-302 termlet pane id and server pane id non interchange compile tests.

### AGENTS.md Rules
- RULE-S30-01: Canonical shared types must live in designated crates. Enforcement: dependency and path lint.
- RULE-S30-02: TermletPaneId must remain distinct from PaneId. Enforcement: compile fail tests against accidental coercion.

## 31. Supplemental Test Matrix
### Design Decisions
- [v11 P2] Matrix covers all gate categories and all runtime modes.
- [v11 P2] Matrix explicitly includes rust python node and tmux version permutations.
- [v11 P2] Matrix includes flake resistance strategy with repeat runs for timing sensitive suites.

### Rust Example
~~~rust
pub struct MatrixAxis {
    pub name: String,
    pub values: Vec<String>,
}
~~~

### Test Strategy
1. TST-310 matrix completeness checker for gates and sections.
2. TST-311 CI matrix scheduler smoke test.
3. TST-312 repeat run detector for flaky scenarios.

### AGENTS.md Rules
- RULE-S31-01: Each gate must map to at least one matrix cell. Enforcement: generated coverage report gate.
- RULE-S31-02: Timing sensitive tests require repeated execution policy. Enforcement: CI rerun harness for tagged tests.
## 32. Termlets
### Design Decisions
- [v11 P2] Termlets are SDK first testing pods and the primary programmable differentiator.

#### 32.1 Architectural role
- [v11 P2] Termlet is pane backed and reuses grid parser and PTY semantics.
- [v11 P2] No parallel terminal core is allowed.

#### 32.2 Layering and dependencies
- [v11 P2] mux-termlet may depend on mux-grid mux-pty mux-types and optional mux-otel.
- [v11 P2] mux-termlet must not depend on mux-server mux-api or mux-orm.

#### 32.3 Lifecycle state machine
- [v11 P2] States are Created Spawning Running Stopping Exited and SpawnFailed.
- [v11 P2] State transitions are monotonic and method guards enforce allowed calls.

#### 32.4 Core API contract
- [v11 P2] Required methods are spawn send_keys wait_for snapshot resize kill is_alive.
- [v11 P2] Optional methods include wait_for_async pool orchestration and snapshot diff.

#### 32.5 Wait semantics
- [v11 P2] wait_for compiles pattern once via PatternMatcher and reuses compiled matcher in loop.
- [v11 P2] timeout zero means immediate single check.
- [v11 P2] timeout and process exit are distinct error variants.

#### 32.6 Snapshot semantics
- [v11 P2] snapshot text trims trailing spaces per line and trailing blank lines for stable assertions.
- [v11 P2] snapshot parity across Rust Python and Node uses shared fixtures.

#### 32.7 Process management
- [v11 P2] kill implements SIGTERM then grace period then SIGKILL escalation.
- [v11 P2] kill is idempotent and Drop performs best effort forced cleanup.

#### 32.8 Real and fake backends
- [v11 P2] Real backend is integration oriented and fake backend is deterministic.
- [v11 P2] Fake mode supports injected output and deterministic replay logs.

#### 32.9 Non blocking drain contract
- [v11 P2] Real backend read path must use non blocking IO with WouldBlock handling.
- [v11 P2] drain_output must never block wait_for timeout progression.

#### 32.10 Identity and type safety
- [v11 P2] TermletPaneId is a distinct type and cannot be confused with server PaneId.
- [v11 P2] Type separation is compile enforced.

#### 32.11 Builder and pool
- [v11 P2] TermletBuilder and direct config spawn path must be behaviorally equivalent.
- [v11 P2] TermletPool uses named HashMap access and Drop kill_all cleanup.

#### 32.12 SnapshotDiff
- [v11 P2] SnapshotDiff includes identical fast path with no allocation and changed line reporting.
- [v11 P2] Diff output is concise and CI friendly.

#### 32.13 Async support
- [v11 P2] wait_for_async is feature gated under async.
- [v11 P2] Async pool helpers are optional wrappers and preserve sync semantics.

#### 32.14 Binding contracts
- [v11 P2] Python uses context manager cleanup and explicit exception classes.
- [v11 P2] Node uses Promise APIs with required finally cleanup path.

#### 32.15 Telemetry and diagnostics
- [v11 P2] Termlet spans include termlet_id pane_id backend_mode and operation name.
- [v11 P2] output_history returns immutable bytes view for advanced assertions and diagnostics.

### Rust Example
~~~rust
use regex::Regex;
use std::any::Any;
use std::ops::Range;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtyMode {
    Real,
    Fake,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TermletPaneId(pub u64);

#[derive(Debug, Clone)]
pub struct TermletConfig {
    pub cols: u16,
    pub rows: u16,
    pub backend: PtyMode,
    pub wait_poll_interval: Duration,
    pub wait_max_interval: Duration,
    pub grace_period: Duration,
    pub inherit_env: bool,
}

pub trait PtyBackend: Send + Any {
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn spawn(&mut self, pane: TermletPaneId, argv: &[String], cols: u16, rows: u16) -> Result<(), String>;
    fn write(&mut self, pane: TermletPaneId, bytes: &[u8]) -> Result<(), String>;
    fn resize(&mut self, pane: TermletPaneId, cols: u16, rows: u16) -> Result<(), String>;
    fn kill(&mut self, pane: TermletPaneId, signal: i32) -> Result<(), String>;
    fn read_events_nonblocking(&mut self, pane: TermletPaneId) -> Vec<PtyEvent>;
}

#[derive(Debug, Clone)]
pub enum PtyEvent {
    Output(Vec<u8>),
    Exited(i32),
}

#[derive(Debug, Clone)]
pub enum PatternMatcher {
    Plain(String),
    Regex(Regex),
}

impl PatternMatcher {
    pub fn compile(raw: &str) -> Result<Self, String> {
        if let Some(rest) = raw.strip_prefix("re:") {
            Regex::new(rest)
                .map(PatternMatcher::Regex)
                .map_err(|e| format!("invalid regex: {}", e))
        } else {
            Ok(PatternMatcher::Plain(raw.to_string()))
        }
    }

    pub fn find(&self, text: &str) -> Option<Range<usize>> {
        match self {
            PatternMatcher::Plain(s) => text.find(s).map(|i| i..(i + s.len())),
            PatternMatcher::Regex(re) => re.find(text).map(|m| m.start()..m.end()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct WaitMatch {
    pub byte_range: Range<usize>,
    pub matched_at: Instant,
}

pub struct Termlet {
    pane: TermletPaneId,
    backend: Box<dyn PtyBackend>,
    output_bytes: Vec<u8>,
    alive: bool,
    cfg: TermletConfig,
}

impl Termlet {
    pub fn wait_for(&mut self, pattern: &str, timeout: Duration) -> Result<WaitMatch, String> {
        let matcher = PatternMatcher::compile(pattern)?;
        let started = Instant::now();
        let mut sleep_for = self.cfg.wait_poll_interval;

        loop {
            self.drain_output();
            let text = String::from_utf8_lossy(&self.output_bytes);
            if let Some(range) = matcher.find(&text) {
                return Ok(WaitMatch { byte_range: range, matched_at: Instant::now() });
            }
            if !self.alive {
                return Err("TERMLET_PATTERN_NOT_FOUND".to_string());
            }
            if started.elapsed() >= timeout {
                return Err("TERMLET_TIMEOUT".to_string());
            }
            std::thread::sleep(sleep_for);
            sleep_for = (sleep_for * 2).min(self.cfg.wait_max_interval);
        }
    }

    pub fn kill(&mut self) {
        if !self.alive {
            return;
        }
        let _ = self.backend.kill(self.pane, libc::SIGTERM);
        let deadline = Instant::now() + self.cfg.grace_period;
        while self.alive && Instant::now() < deadline {
            self.drain_output();
            std::thread::sleep(Duration::from_millis(25));
        }
        if self.alive {
            let _ = self.backend.kill(self.pane, libc::SIGKILL);
            self.alive = false;
        }
    }

    pub fn output_history(&self) -> &[u8] {
        &self.output_bytes
    }

    fn drain_output(&mut self) {
        for evt in self.backend.read_events_nonblocking(self.pane) {
            match evt {
                PtyEvent::Output(bytes) => self.output_bytes.extend_from_slice(&bytes),
                PtyEvent::Exited(_) => self.alive = false,
            }
        }
    }
}
~~~

### Test Strategy
- [v11 P2] Mandatory Termlet matrix is deeper than all other sections and is release blocking.

#### Unit and contract tests
1. TST-320 spawn in real and fake modes.
2. TST-321 send_keys plus wait_for plain string success path.
3. TST-322 regex wait_for success and invalid regex error.
4. TST-323 timeout versus process exited distinction.
5. TST-324 zero timeout immediate check semantics.
6. TST-325 snapshot normalization trimming rules.
7. TST-326 resize propagates to backend and local state.
8. TST-327 kill idempotency.
9. TST-328 Drop forced kill fallback when still alive.
10. TST-329 fake backend deterministic replay.

#### Cross language parity tests
11. TST-330 shared fixture snapshot parity across Rust Python Node.
12. TST-331 Python context manager cleanup no leak.
13. TST-332 Node finally cleanup no leak.
14. TST-333 builder path parity with direct config.
15. TST-334 pool kill_all and Drop behavior.

#### Advanced correctness tests
16. TST-335 SnapshotDiff identical fast path no allocation.
17. TST-336 SnapshotDiff changed line accuracy.
18. TST-337 telemetry spans include required attributes.
19. TST-338 fake and real spawn plus snapshot performance budgets.
20. TST-339 background lifecycle test for detached child then kill.
21. TST-340 non blocking drain verifies wait_for timeout is not stalled.
22. TST-341 output_history immutability API test.
23. TST-342 TermletPaneId and PaneId type separation compile fail test.
24. TST-343 wait_for matcher compile once instrumentation check.
25. TST-344 inherit_env behavior tests with explicit override precedence.
26. TST-345 async feature gate compile tests for wait_for_async and async pool.
27. TST-346 pool per termlet timeout semantics test.
28. TST-347 TermletExt extension methods cannot mutate core state test harness.
29. TST-348 fuzz parser bytes through fake backend and snapshot stability assertions.
30. TST-349 long run leak detection for process and descriptor cleanup.

#### CI policy
31. TST-350 Termlet suite runs on Linux in real and fake modes for every PR.
32. TST-351 nightly run adds stress repeats and leak sanitizer checks.
33. TST-352 release run requires all TST-320 to TST-351 pass.

### AGENTS.md Rules
- RULE-S32-01: Snapshot text must follow normalization contract. Enforcement: TST-325 snapshot golden tests.
- RULE-S32-02: kill must be idempotent. Enforcement: TST-327 double kill assertions.
- RULE-S32-03: Drop must force cleanup of live child process. Enforcement: TST-328 leak detector.
- RULE-S32-04: Fake backend tests must be deterministic and timing independent. Enforcement: TST-329 repeated run stability checks.
- RULE-S32-05: Bindings must provide cleanup idioms context manager or finally. Enforcement: TST-331 and TST-332.
- RULE-S32-06: mux-termlet dependency deny list is strict. Enforcement: dependency graph CI gate.
- RULE-S32-07: Every public Termlet API is tested in Rust and at least one binding. Enforcement: API coverage report.
- RULE-S32-08: wait_for must compile matcher once per call. Enforcement: TST-343 instrumentation check.
- RULE-S32-09: Termlet remains pane backed and cannot fork a parallel emulator core. Enforcement: architecture review plus dependency audit.
- RULE-S32-10: Cross language snapshot parity tests are required on each release. Enforcement: TST-330 release gate.
- RULE-S32-11: Builder and direct config must be equivalent. Enforcement: TST-333 parity test.
- RULE-S32-12: SnapshotDiff identical case must avoid allocation. Enforcement: TST-335 allocation profiler check.
- RULE-S32-13: TermletPool Drop must kill all managed Termlets. Enforcement: TST-334 pool cleanup checks.
- RULE-S32-14: Async support must remain feature gated. Enforcement: TST-345 compile matrix.
- RULE-S32-15: Real backend event reads must be non blocking with WouldBlock handling. Enforcement: TST-340 timeout stall guard and code audit.
