Created all three Pass 2 refinement docs with source-backed decisions and line-level references:

1. `notes/2026-02-10-rust-architectural-approach-claude-v5-pass2.md`
2. `notes/2026-02-10-rust-architectural-approach-gemini-v5-pass2.md`
3. `notes/2026-02-10-rust-architectural-approach-gpt-v6-pass2.md`

Each file includes:

- Variant-specific Pass 1 weaknesses
- Definitive resolutions to all 7 open questions (with tmux/vibe/libtmux citations)
- Concrete Rust types/signatures for the decisions
- Strengthened test strategy
- Added risks/edge cases
- Existence verification table for claimed files/functions/APIs

I also verified the benchmark baseline issue directly: `cargo bench -p mux-refresh --bench latency` currently reports no criterion measurements because `crates/mux-refresh/Cargo.toml` is missing `[[bench]] harness = false`.
/bindings.

## 2) Definitive Resolutions for 7 Open Questions

### Q1. Layout checksum compatibility

Decision:
1. Match tmux checksum exactly on tmux-compatible paths.
2. Add stronger hash only in internal, versioned format.

Evidence:
1. tmux checksum algorithm (`layout-custom.c:45-56`).
2. tmux serialization includes 4-hex checksum prefix (`layout-custom.c:69`).
3. parser rejects mismatch (`layout-custom.c:165-173`).

Rust API:
```rust
pub enum LayoutWireFormat { TmuxV1, TermForgeV2 }

pub fn dump_layout_tmux_v1(tree: &LayoutTree) -> String;
pub fn parse_layout_tmux_v1(s: &str) -> Result<LayoutTree, LayoutParseError>;
```

### Q2. Option inheritance and `set -u`

Decision:
1. Option override persists only while local entry exists.
2. `set -u` removes local entry and re-enables inheritance.
3. `set -U` includes pane-level unsets for pane options at window scope.

Evidence:
1. Parent-chain lookup in `options_get` (`options.c:229-240`).
2. Unset logic in `cmd-set-option` (`cmd-set-option.c:180-187`).
3. Removal/default semantics (`options.c:1270-1282`).
4. tmux man semantics (`tmux.1:4075-4084`).

Rust API:
```rust
pub enum UnsetMode { LocalOnly, LocalAndPanes }

pub fn get_effective(scope: ScopeRef, key: OptionKey) -> OptionValue;
pub fn unset_local(scope: ScopeRef, key: OptionKey, mode: UnsetMode) -> Result<(), OptionError>;
```

### Q3. Control mode auth and rate limiting

Decision:
1. No extra control-mode auth handshake (tmux-compatible).
2. AuthZ is Unix socket permissions + UID ACL.
3. Add explicit control-command ingress rate limits in TermForge.

Evidence:
1. Socket creation + perms (`server.c:107-130`).
2. ACL admission (`server.c:396-399`, `server-acl.c:164-179`).
3. server-access docs and trust warning (`tmux.1:1541-1578`).
4. tmux output backpressure/pause behavior (`control.c:450-461`, `control.c:739-741`).

Rust API:
```rust
pub struct ControlRateLimiter { /* token bucket state */ }

pub fn allow_control_command(client: ClientId, now: Instant) -> bool;
```

### Q4. CRDT vector clock growth

Decision:
1. Use dotted version vectors with membership-based compaction.
2. Keep vector size bounded to active replicas + retention window.

Evidence:
1. No CRDT/HLC/vector-clock implementation currently present in `vibe-tmux` code under `crates/` or `bindings/`.
2. tmux provides no CRDT guidance; this is a new architecture concern.

Rust API:
```rust
pub struct Dot { pub node: NodeId, pub counter: u64 }
pub struct Dvv { pub summary: SmallVec<[(NodeId, u64); 8]>, pub dot: Dot }

pub fn compact_dvv(dvv: &mut Dvv, active_nodes: &[NodeId], epoch: u64);
```

### Q5. Benchmark baseline realism

Decision:
1. Today’s measured baseline is effectively unavailable; criterion benches are defined but not executed as criterion benches.
2. Fix harness first, collect baseline in CI, then enforce regression gates.

Evidence:
1. Bench functions exist (`crates/mux-refresh/benches/latency.rs:13-48`).
2. Missing `[[bench]] harness = false` in `crates/mux-refresh/Cargo.toml:1-18`.
3. Current `cargo bench -p mux-refresh --bench latency` output reports no benchmark measurements.

Cargo fix:
```toml
[[bench]]
name = "latency"
harness = false
```

### Q6. Python async bindings

Decision:
1. Keep sync API as primary ABI surface.
2. Add async convenience wrappers in Python first.
3. Consider `pyo3-asyncio` only for long-lived stream APIs.

Evidence:
1. `libtmux` API model is synchronous (`src/libtmux/options.py:701-787`).
2. `vibe-tmux` Python binding has `pyo3` but no `pyo3-asyncio` dependency (`bindings/python/Cargo.toml:11-23`).

### Q7. Layout minimum-size behavior

Decision:
1. `PANE_MINIMUM` is currently 1 in tmux (`tmux.h:100`).
2. Resize operations clamp to feasible minima and may partially apply.
3. If no sibling can donate space, remaining delta is not applied.

Evidence:
1. Minimum checks and available-space computation (`layout.c:367-415`).
2. Clamp of requested change (`layout.c:553-557`, `layout.c:568-570`).
3. Stop on no more possible change (`layout.c:636-637`, `layout.c:701-703`, `layout.c:728-734`).

Rust API:
```rust
pub struct ResizeResult { pub requested: i32, pub applied: i32 }

pub fn resize_pane_with_min(tree: &mut LayoutTree, pane: PaneId, axis: Axis, delta: i32) -> ResizeResult;
```

## 3) Better Architecture Where Pass 1 Had Flaws

1. Replace non-existent `mux-types`/`mux-orm` references with current crates (`mux-core`, `mux-api`, `mux-query`, `mux-client`, `mux-refresh`).
2. Keep GPT error taxonomy (Transient/ProtocolViolation/UserError/Bug) but implement mapping tables in existing crates only.
3. Preserve “control mode hints, binary protocol authority” and enforce tmux-compatible binary violation handling (`server-client.c:3377-3475`).
4. Keep AGENTS-rule additions, but mark them “policy” not “implemented code” where no code exists yet.

## 4) Strengthened Test Strategy

1. Protocol: fixture and fuzz tests proving violation classification and disconnect policy.
2. Identify burst: ordering parity tests against `client_send_identify` (`client.c:450-495`) including duplicate `LONGFLAGS`.
3. Options: end-to-end unset semantics tests (`-u`, `-U`, globals, array index unsets).
4. Layout: checksum parity tests + min-size invariant property tests.
5. Control mode: `%begin/%end/%error` framing tests and notification mapping tests.
6. Bench pipeline: CI assertion that criterion JSON/artifacts are produced; fail if empty.
7. Python: sync + async-wrapper interop tests (thread offload, cancellation, no GIL deadlocks).

## 5) Added Risks and Edge Cases

1. Claiming compatibility while using stronger checksum on tmux path would break `select-layout` interop.
2. ACL-only trust model can be undermined by weak socket path permissions.
3. Control output lag handling may disconnect clients unexpectedly under high throughput.
4. PID reuse can break naive stale-lock recovery.
5. Vector metadata growth can still regress memory if membership pruning is skipped.

## 6) Reference Verification (Existence Check)

| Claim from Pass 1 | Status | Evidence |
|---|---|---|
| `goto bad -> proc_kill_peer` | Exists | `server-client.c:3377-3475` |
| `mux-types` crate exists | Missing | Not in workspace members (`~/work/rust/vibe-tmux/Cargo.toml`) |
| `mux-orm` crate exists | Missing | No `crates/mux-orm` |
| Typed control notifications already implemented | Missing | current `ControlNotification { name, raw }` (`crates/mux-client/src/control.rs:31-36`) |
| Benchmark infra currently yields criterion values | Missing | harness not configured (`crates/mux-refresh/Cargo.toml`) |
| Identify message constants and helpers | Exists | `crates/mux-proto/src/msg.rs:40-67`, `:180-202` |

## 7) DO/DON'T updates (delta)

1. DO keep taxonomy categories but bind them to existing crates.
2. DO enforce tmux-compatible disconnect behavior for binary protocol violations.
3. DO treat benchmark numbers as unknown until harness is fixed.
4. DON’T introduce phantom crates into acceptance criteria.
5. DON’T conflate policy docs with implemented code paths.

