# Deep-Dive Review: v7 Spec Sections §12, §16, §19, §20-22
## Date: 2026-02-11
## Source: Codebase study of libtmux, vibe-tmux, OpenTelemetry Rust SDK

This review compares the TermForge v7 architecture spec (`synthesized-v7.md`) against the actual source code of three reference codebases:
- `~/work/python/libtmux/` — QueryList/QuerySet patterns, ORM traversal
- `~/work/rust/vibe-tmux/` — Test support, socket isolation, version management
- `~/study/otel/opentelemetry-rust/` — OTEL SDK types, context propagation

---

## §12 — ORM Query API: Gaps vs libtmux's Actual QueryList

### Missing Operators
- v7 spec defines: `Exact, Contains, IContains, StartsWith, IStartsWith, EndsWith, IEndsWith, Regex, In, Gt, Gte, Lt, Lte, IsNone, IsSome`
- libtmux actual (`_internal/query_list.py` LOOKUP_NAME_MAP): `eq, exact, iexact, contains, icontains, startswith, istartswith, endswith, iendswith, in, nin, regex, iregex`
- **Missing from v7**: `Nin` (not-in) and `IRegex` (case-insensitive regex) — both are in libtmux today
- **v7 additions** (`Gt`, `Gte`, `Lt`, `Lte`, `IsNone`, `IsSome`) don't exist in libtmux — these are reasonable extensions

### Missing Features
1. **Callable matcher** — libtmux's `filter()` accepts a callable as first positional arg: `sessions.filter(lambda s: s.session_name.startswith("work"))`. v7 only shows `QuerySpec`-based filtering.
2. **kwargs-style filter** — libtmux uses `filter(session_name="work")` and `filter(session_name__startswith="wo")` as keyword arguments. v7 uses builder syntax `q("name").exact("work")` which is less ergonomic for Python bindings.
3. **`get(default=None)`** — libtmux's `.get()` supports a `default` parameter to return instead of raising. v7's `get()` only returns `Result<&T, QueryError>`.
4. **Specific exceptions** — libtmux raises `ObjectDoesNotExist` and `MultipleObjectsReturned` from `.get()`. v7 uses generic `QueryError`.
5. **Nested field access (keygetter)** — libtmux's `keygetter` function navigates `food__fruit__in="banana"` through nested dicts/objects using `__` as path separator. v7 uses `__` for operator syntax but doesn't show deep field traversal.
6. **Context manager support** — libtmux objects support `with server.new_session() as session:` for automatic cleanup. Not mentioned in ORM layer.

### Recommendation
Add `Nin`, `IRegex` to `QueryOp`. Add `default` parameter to `QueryList::get()`. For Python bindings, expose kwargs-style filter (convert `name__startswith="wo"` to `QuerySpec` under the hood).

---

## §16 — Language Bindings: QueryList Not Exposed to Python/Node

### Critical Gap
The v7 spec's Python binding at §16.3 returns `Vec<PySession>`:
```rust
#[getter]
fn sessions(&self) -> PyResult<Vec<PySession>> {
    Ok(self.inner.sessions().items().map(PySession::from).collect())
}
```
But §22.3 test code shows `server.sessions.filter(name="beta")` — this requires a Python `QueryList` wrapper class, which is never defined in §16.

The binding must expose a `PyQueryList` that wraps `QueryList<T>` and supports:
- `filter(**kwargs)` with `__` operator syntax
- `get(**kwargs)` with `default` parameter
- `len()`, `__iter__`, `__getitem__` for Pythonic behavior

### Same Issue for Node
§22.6 shows `server.sessions({ name: "work" })` but §16.4 doesn't define how filtering works.

### Recommendation
Add `PyQueryList` class to §16.3 and `JsQueryList` pattern to §16.4 that wrap the Rust `QueryList` with native filtering APIs.

---

## §19 — OpenTelemetry: Missing Dual Provider, Thread-Local Stack, TOML Config

### Gap 1: Dual Provider Architecture
vibe-tmux uses TWO providers (`mux-otel/src/otel.rs`):
- `OTEL_PROVIDER`: Global provider for server/TUI
- `MUX_CLIENT_PROVIDER`: Separate provider for client spans, initialized on-demand

v7 §19.3 shows a single `init_telemetry()` function. Need dual-provider to prevent client blocking.

### Gap 2: Thread-Local Trace Headers Stack
vibe-tmux propagates trace context via `TRACE_HEADERS_STACK` (thread-local `RefCell<Vec<TraceHeaders>>`) with `TraceHeadersGuard` for RAII cleanup:
1. Python sets `TRACEPARENT` env var
2. Rust client reads via `attach_traceparent_from_env()`
3. Pushes to thread-local stack
4. Guard pops on drop

v7 §19.7 shows binding side but doesn't detail this propagation mechanism.

### Gap 3: TOML Config with Priority Chain
vibe-tmux uses priority chain:
1. Built-in defaults
2. `~/.config/vibe-tmux/otel.toml`
3. `.vibe-tmux/otel.toml`
4. `.vibe-tmux/otel.local.toml`
5. `VIBE_TMUX_OTEL_CONFIG` env var

v7 uses glob-pattern matching. vibe-tmux uses prefix/exact matching with allow/deny — simpler and more predictable.

### Gap 4: Composite Propagator
vibe-tmux chains `BaggagePropagator` + `TraceContextPropagator` into `TextMapCompositePropagator`. v7 doesn't mention baggage propagation.

### Gap 5: Enable/Disable Toggle
vibe-tmux checks `VIBE_TMUX_OTEL` or `OTEL_EXPORTER_OTLP_ENDPOINT`. v7 has `TelemetryConfig.enabled` but no env var activation.

### Gap 6: OnceLock Lazy Initialization
Both providers use `OnceLock` for safe one-time init. Not mentioned in spec.

---

## §20 — tmux Version Management: Missing Build Safety Mechanisms

### Gap 1: BLAKE3 Cache Key
vibe-tmux computes: `tmux-<version>__<host>__<os>-<os_ver>__cfg<configure_hash>__mk<make_hash>__tb<builder_version>`
Uses BLAKE3 hash. v7 mentions cache keys but no hashing detail.

### Gap 2: File-Based Locking
vibe-tmux uses file locks: `lock_repo_clone()` (prevents parallel clone races), `lock_cache_key()` (prevents concurrent build corruption). Lock guards auto-release via `Drop`. v7 doesn't mention locking.

### Gap 3: Atomic Build Publication
Build in `.tmp-<name>-<pid>-<nanos>` then atomic rename. Not in v7.

### Gap 4: Environment Variable Controls
| Variable | Purpose |
|---|---|
| `VIBE_TMUX_VERSION` | Desired version |
| `VIBE_TMUX_AUTO_BUILD` | Auto-build from source |
| `VIBE_TMUX_OFFLINE` | Reject network ops |
| `VIBE_TMUX_CACHE_DIR` | Custom cache dir |
| `VIBE_TMUX_REPO` | Explicit repo path |
| `VIBE_TMUX_BUILD_JOBS` | Parallel jobs |
| `VIBE_TMUX_CONFIGURE_FLAGS` | Configure flags |
| `VIBE_TMUX_MAKE_FLAGS` | Make flags |
| `TMUX_BIN` | Explicit binary path |

v7 shows CLI args but not env var interface for CI.

### Gap 5: Version Satisfies Logic
`version_satisfies("3.4a", "3.4")` returns true (prefix matching). v7 doesn't define this.

---

## §21 — Test Support: Missing Multi-Layer Socket Safety

### Gap 1: Three-Layer Socket Validation
vibe-tmux `path_guard.rs` has:
1. `ensure_not_default_socket_name()` — rejects "default"
2. `ensure_socket_within_tempdir()` — socket must be within temp dir
3. `ensure_socket_not_tmux_env()` — must not match `$TMUX`

v7 PathGuard just creates temp dir with UUID naming. Lacks all three checks.

### Gap 2: Socket Path Format
vibe-tmux: `$TMUX_TMPDIR/tmux-<uid>/socket-<pid>-<nanos>` (deterministic, debuggable)
v7: UUID-based (`termforge-test-{uuid}`)

### Gap 3: Permission Hardening
vibe-tmux: `chmod_0700(socket_dir)` with test assertions. Not in v7.

### Gap 4: Config Isolation
vibe-tmux: Every tmux invocation uses `-f /dev/null`. Not explicit in v7 TmuxTestServer.

### Gap 5: Socket Readiness Check
vibe-tmux: `wait_for_socket(path, 2.0_seconds)` with UnixStream::connect(). Not in v7.

### Gap 6: Clipboard Isolation
vibe-tmux: `set-clipboard off`, env var escape hatch. Not in v7.

### Gap 7: MuxServerTestServer Architecture
v7 §21.2 describes "in-process server using ServerGraph + FakePtyBackend".
vibe-tmux actual: spawns separate `mux-server` binary with SIGTERM→SIGKILL shutdown.
Need both modes: truly in-process (unit tests) AND subprocess (integration/e2e).

---

## §22 — Binding Tests: Internal Consistency Issue

Test code in §22.3 (`server.sessions.filter(name="beta")`) and §22.6 (`server.sessions({ name: "work" })`) relies on filtering at binding level, but §16 doesn't implement the filtering API. §16 must include `PyQueryList` and Node equivalent.

---

## Summary of Required Amendments

| Section | Issue | Priority |
|---|---|---|
| §12 | Add `Nin`, `IRegex` operators to match libtmux | High |
| §12 | Add `get(default=...)` parameter | Medium |
| §12 | Add callable matcher support to `filter()` | Low |
| §16 | Add `PyQueryList` class with kwargs filter API | **Critical** |
| §16 | Add Node `QueryList` equivalent | **Critical** |
| §19 | Add dual-provider architecture | High |
| §19 | Add thread-local TRACE_HEADERS_STACK + Guard | High |
| §19 | Add TOML config priority chain | Medium |
| §19 | Add composite propagator (Baggage + TraceContext) | Medium |
| §19 | Add env var enable toggle pattern | Medium |
| §20 | Add BLAKE3 cache key computation | High |
| §20 | Add file-based locking for parallel safety | **Critical** |
| §20 | Add atomic build publication | High |
| §20 | Add comprehensive env var interface | High |
| §21 | Add three-layer socket validation | **Critical** |
| §21 | Add permission hardening (0700) | High |
| §21 | Add `-f /dev/null` config isolation | High |
| §21 | Add socket readiness check | High |
| §21 | Add clipboard isolation | Medium |
| §21 | Clarify MuxServerTestServer = subprocess, not in-process | High |
