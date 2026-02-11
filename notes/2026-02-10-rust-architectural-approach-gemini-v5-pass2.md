# TermForge v5 Pass 2 Refinement (gemini-v5)

Date: 2026-02-11
Variant: gemini-v5
Scope: Pass 1 critique + definitive resolutions for open questions + architecture/test/risk hardening.

## 1) Pass 1 Weaknesses (gemini-v5)

1. The document includes execution chatter and tool limitations instead of clean architecture output.
2. Several proposed types/crates are aspirational and not mapped to current `vibe-tmux` crate boundaries.
3. Protocol details include assumptions not backed by tmux wire behavior (for binary protocol violations tmux kills peer, not “soft error” handling).
4. Layout and options sections are concise but miss decisive source-backed answers for checksum/versioning and `set -u/-U` semantics.
5. Benchmark section lacks measured baseline and misses current wiring bug (criterion bench not actually running).

## 2) Definitive Resolutions for 7 Open Questions

### Q1. Layout string checksum: exact tmux or stronger hash?

Decision:
1. Wire compatibility path must use tmux checksum exactly.
2. Optional stronger hash may be added only in internal envelope format.

Evidence:
1. Checksum algorithm: rotate-right + add (`layout-custom.c:45-56`).
2. Emission format `%04hx,layout` (`layout-custom.c:69`).
3. Parse rejects mismatch (`layout-custom.c:165-173`).

Rust API:
```rust
pub enum LayoutWireFormat { TmuxV1, TermForgeV2 }

pub fn dump_layout_tmux_v1(tree: &LayoutTree) -> String;
pub fn parse_layout_tmux_v1(input: &str) -> Result<LayoutTree, LayoutParseError>;
```

### Q2. Option inheritance vs override (`set -u`)

Decision:
1. Local set shadows inherited parent value.
2. `set -u` removes local value and resumes inheritance.
3. `set -U` additionally unsets pane-local values for pane options in window scope.

Evidence:
1. Parent-chain lookup (`options_get`) (`options.c:229-240`).
2. Unset implementation (`cmd-set-option.c:180-187`).
3. Remove/default behavior (`options.c:1270-1282`).
4. Man page semantics (`tmux.1:4075-4084`).

Rust API:
```rust
pub enum UnsetMode { LocalOnly, LocalAndPanes }

pub fn set_local(scope: ScopeRef, key: OptionKey, value: OptionValue) -> Result<(), OptionError>;
pub fn unset_local(scope: ScopeRef, key: OptionKey, mode: UnsetMode) -> Result<(), OptionError>;
pub fn get_effective(scope: ScopeRef, key: OptionKey) -> OptionValue;
```

### Q3. Control mode authentication and rate limiting

Decision:
1. tmux control mode has no additional auth handshake beyond socket+ACL.
2. Keep ACL/socket-permission model for compatibility.
3. Add explicit command rate limiting in TermForge.

Evidence:
1. Socket creation and permissions (`server.c:107-130`).
2. ACL admission check (`server.c:396-399`, `server-acl.c:164-179`).
3. tmux docs on ACL/socket trust boundary (`tmux.1:1572-1578`).
4. Backpressure behavior for lagging control clients (`control.c:450-461`, `control.c:739-741`).

Rust API:
```rust
pub struct ControlRateLimiter { /* token bucket */ }
pub fn allow_control_command(client: ClientId, now: Instant) -> bool;
```

### Q4. CRDT vector clock size

Decision:
1. Use dotted version vectors with compaction to active membership.
2. Do not use unbounded full vector clocks.

Evidence:
1. No CRDT/vector-clock implementation currently exists in `vibe-tmux` crates/bindings.
2. tmux has no CRDT precedent; this is a TermForge design choice.

Rust API:
```rust
pub struct Dot { pub node: NodeId, pub counter: u64 }
pub struct Dvv { pub summary: SmallVec<[(NodeId, u64); 8]>, pub dot: Dot }

pub fn compact_dvv(dvv: &mut Dvv, active_nodes: &[NodeId], epoch: u64);
```

### Q5. Benchmark baseline realism

Decision:
1. Current measurable baseline is unavailable because criterion benches are not wired to execute.
2. Fix bench harness first, then collect baseline, then enforce regression budget.

Evidence:
1. Bench functions are present (`crates/mux-refresh/benches/latency.rs:13-48`).
2. `mux-refresh` Cargo.toml lacks `[[bench]]` harness override (`crates/mux-refresh/Cargo.toml:1-18`).
3. Running `cargo bench -p mux-refresh --bench latency` currently yields zero benchmark measurements.

Cargo fix:
```toml
[[bench]]
name = "latency"
harness = false
```

### Q6. Python binding async model

Decision:
1. Keep synchronous PyO3 API as canonical surface.
2. Provide async wrappers in Python (`asyncio.to_thread`) before introducing `pyo3-asyncio`.

Evidence:
1. `libtmux` is sync command wrapper model (`src/libtmux/options.py:701-787`, `src/libtmux/options.py:958-1002`).
2. `vibe-tmux` python binding depends on `pyo3` only, no `pyo3-asyncio` (`bindings/python/Cargo.toml:11-23`).

### Q7. Layout minimum size behavior

Decision:
1. Current tmux defines `PANE_MINIMUM` as 1 (`tmux.h:100`).
2. Resize requests are clamped/partially applied to preserve minimums.
3. If no sibling can donate space, remaining resize delta is ignored.

Evidence:
1. `layout_resize_check` minimum accounting (`layout.c:367-415`).
2. Clamp logic (`layout.c:553-557`, `layout.c:568-570`).
3. Stop when no change possible (`layout.c:636-637`, `layout.c:701-703`, `layout.c:728-734`).

Rust API:
```rust
pub struct ResizeResult { pub requested: i32, pub applied: i32 }

pub fn resize_pane_with_min(tree: &mut LayoutTree, pane: PaneId, axis: Axis, delta: i32) -> ResizeResult;
```

## 3) Better Architecture Where Pass 1 Was Thin

1. Introduce a strict compatibility boundary: binary protocol path follows tmux kill-peer behavior for violations (`server-client.c:3377-3475`), control mode remains hint channel.
2. Split layout serialization into `TmuxV1` and internal future format; avoid conflating compatibility with persistence.
3. Model options as sparse overrides + parent chain, not copied parent snapshots.
4. Keep control notifications typed incrementally: start with current `name/raw`, then promote common names into enum variants.

## 4) Strengthened Test Strategy

1. Layout checksum golden tests against tmux-generated layout strings.
2. Options inheritance table tests for set, unset `-u`, unset `-U`, global defaults.
3. Binary protocol fuzz tests that assert disconnect on protocol violation.
4. Control mode parser tests for `%begin/%end/%error` framing and notification extraction.
5. Benchmark guard test: fail CI if no criterion report generated.
6. Python async-wrapper tests validating cancellation and thread handoff behavior.

## 5) Added Risks and Edge Cases

1. 16-bit checksum collisions if reused outside compatibility protocol.
2. PID reuse risk in lockfile stale-owner recovery.
3. Control backlog starvation among panes if scheduling is unfair.
4. Array option index unset semantics can drift if tmux index handling is not mirrored.
5. FD ancillary payload with extra descriptors must close extras deterministically.

## 6) Reference Verification (Existence Check)

| Claim | Status | Evidence |
|---|---|---|
| Identify msg range 100-112 | Exists | `tmux-protocol.h:29-41`, `crates/mux-proto/src/msg.rs:40-67` |
| Protocol violation kill path | Exists | `server-client.c:3377-3475` |
| Current typed control notification enum (20+ variants) | Missing | Current generic `ControlNotification { name, raw }` in `crates/mux-client/src/control.rs:31-36` |
| `mux-types` crate in workspace | Missing | Not listed in workspace members (`~/work/rust/vibe-tmux/Cargo.toml`) |
| CRDT/HLC implementation in code | Missing | No matching symbols under `crates/`/`bindings/` |
| Criterion bench currently producing metrics | Missing | Harness misconfigured (`crates/mux-refresh/Cargo.toml`) |

## 7) DO/DON'T updates (delta)

1. DO cite tmux source line numbers for compatibility claims.
2. DO separate compatibility format from internal persistence format.
3. DO treat `set -u` as local override removal.
4. DON’T claim benchmark targets are met without runnable baseline.
5. DON’T treat control mode auth as protocol concern; it is socket/ACL concern.

