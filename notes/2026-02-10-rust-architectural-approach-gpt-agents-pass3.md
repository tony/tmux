# AGENTS.md (Human + LLM Guardrails)

This repo is intended to be safe for both humans and LLM coding agents. These rules are non-negotiable.

## Rule 0: Never Kill Real tmux

- Never run `kill-server` against a developer's real tmux socket.
- All tests must use isolated sockets inside a temp directory (`TMUX_TMPDIR`).
- Tests must `env_remove("TMUX")` for all subprocesses.
- Socket name `"default"` is forbidden in tests.

## Mission

Build a layered Rust terminal multiplexer with tmux compatibility using a deterministic pure core and an imperative runtime.

## Layering (Hard Boundary)

PURE crates (compile to `wasm32-unknown-unknown`):

- `mux-types`
- `mux-core`
- `mux-grid`
- `mux-proto`
- `mux-query`
- `mux-view`
- `mux-conf`
- `mux-command`
- `mux-control`
- `mux-crdt`
- `mux-pty-fake`

PURE means:

- No `tokio`, no `async/.await`
- No `std::fs`, `std::net`, PTYs, sockets, threads, signals
- No `nix`/`libc`
- No handwritten `unsafe`
- No reading `SystemTime` or global RNG (time/randomness must be injected by explicit events)

IMPURE crates:

- `mux-os` (unsafe quarantine; handwritten `unsafe` allowed with `// SAFETY:` and tests)
- `mux-pty` / `mux-pty-portable` / `mux-pty-recorder`
- `mux-runtime`, `mux-server`, `mux-client`
- `mux-api` (connect + submit + subscribe facade)
- `mux-orm` (libtmux-style ergonomics facade)
- tools/ and bindings/

## Enforced Purity (CI Gate)

CI must run at minimum:

- `cargo check --target wasm32-unknown-unknown -p mux-types`
- `cargo check --target wasm32-unknown-unknown -p mux-core`
- `cargo check --target wasm32-unknown-unknown -p mux-grid`
- `cargo check --target wasm32-unknown-unknown -p mux-proto`

## Entity Model Rules

- Parent-to-child relationships are stored as `Vec<ChildId>` inside entities (`Session.windows`, `Window.panes`).
- Do not store reverse edges in the authoritative graph. Reverse lookups belong in snapshots (`mux-view` indices).
- Do not introduce persistent data structures (`im-rs`) until profiling proves cloning is a bottleneck.
- Use `Arc<Grid>` with copy-on-write (`Arc::make_mut`) for grid-heavy state.

## Determinism Rules

- All writes go through `ServerGraph::apply(Event) -> ApplyOutcome`.
- Any time-based behavior is driven by explicit events (`Event::Tick`, `TimerFired`, etc.), never `SystemTime` reads.
- All randomness must be injected as data in events, never from global RNG in PURE code.

## Terminal Parsing Rules

- `mux-grid` owns VT/ANSI parsing.
- The parser must match tmux `input.c` (state table; 17-state DEC ANSI design; tmux extensions).
- Do not use the `vte` crate for tmux compatibility surfaces.

## Format Engine Rules

- The format string engine lives in `mux-core/src/format.rs`.
- It is a pure function: parse to AST, evaluate with a `VarSource` that reads from snapshots + explicit context.
- Format parity is proven via fixtures against real tmux (`display-message -p` corpus).

## Key Binding Rules

- Key dispatch is table-based (`mux-core/src/key_table.rs`) with `root`, `prefix`, `copy-mode`, `copy-mode-vi`.
- Prefix timeout is event-driven (timer event), not `sleep`.
- Bindings are data; behavior lives in reducers.

## Copy Mode Rules

- Copy mode is per-pane: `CopyModeState` in the pure core.
- vi/emacs behaviors are expressed via key tables mapping to `CopyOp` actions.
- Selection/search/extraction is deterministic and covered by snapshot tests.

## ORM / Bindings Rules

- `mux-orm` is a convenience layer only. It must not become a second business-logic engine.
- Query matching and filtering execute in Rust (`mux-query` over snapshots). Bindings may only build query specs.
- Public bindings APIs expose the tmux object graph (server/sessions/windows/panes + commands), not internal names (frame/store/json).

## Protocol Rules

- Protocol decoding/encoding is explicit field parsing with length validation and allocation caps.
- Chunking robustness tests are required: decoding must be invariant under arbitrary segmentation.
- SCM_RIGHTS FDs must be bound to frames and closed on drop. Validate FD counts.

## Testing Rules

- Unit tests first (PURE crates).
- Deterministic integration tests use `mux-pty-fake`.
- Real-world parity uses ScenarioRecorder/Replayer (`mux-pty-recorder` JSON) and hermetic servers (`mux-test-support`).
- Never let tests touch user config: run tmux with `-f /dev/null`.

## Before Commit (Minimum)

- `cargo fmt --all`
- `cargo clippy --workspace --all-targets`
- `cargo test --workspace`
- WASM purity checks for PURE crates (see "Enforced Purity")
