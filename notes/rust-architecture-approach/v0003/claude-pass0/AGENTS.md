# AGENTS.md -- Agent Roles and Quality Gates

## Agent Roles

### Architect Agent
- Maintains the crate DAG and layering rules.
- Ensures no cross-layer dependency cycles.
- Reviews Cargo.toml changes for dependency correctness.
- Enforces the 6-layer hierarchy (L0-L6).

### Implementation Agent
- Writes crate implementations following coding rules.
- Ensures `#![forbid(unsafe_code)]` in all crates except mux-pty and mux-ffi.
- References tmux source file and line for invariant verification.
- Uses `thiserror` for library errors, `anyhow` only in binaries.

### Test Agent
- Writes and maintains tests for all crates.
- Ensures deterministic tests (no wall-clock sleeps).
- Uses `Clock::manual()` for time-dependent tests.
- Maintains minimum 3 tests per crate.
- Runs property tests for invariant verification.

### Review Agent
- Enforces quality gates before merge.
- Checks invariant documentation completeness.
- Verifies gap coverage (12 identified gaps).
- Runs full test suite and clippy.

## Quality Gates

### Gate 1: Compilation
- `cargo check` must pass with zero errors.
- `cargo check --tests` must pass.

### Gate 2: Tests
- `cargo test --workspace` must pass with zero failures.
- Target: 250+ tests (current: 367).
- All tests must be deterministic.

### Gate 3: Lints
- `cargo clippy --workspace --all-targets -- -D warnings` should pass.
- Exceptions documented in workspace Cargo.toml.

### Gate 4: Documentation
- All public items have doc comments.
- Functions returning Result have `# Errors` sections.
- Invariants reference tmux source lines.

### Gate 5: Architecture
- No cross-layer dependency cycles.
- New dependencies reviewed by Architect Agent.
- `#![forbid(unsafe_code)]` maintained in all safe crates.

## Invariant Rules

### INV-117: Panic Recovery
- FFI boundaries use `catch_unwind`.
- `panic = "unwind"` required in all profiles.
- Clippy denies `unwrap_used`, `expect_used`, `panic`.

### INV-119: CellFlags Compatibility
- Bit values must match tmux.h:742-749 exactly.
- Tests verify each flag value against known constants.

### INV-220: Graphics Passthrough
- `allow-passthrough` defaults to `false`.
- Tests verify the default is off.
- API builder preserves the default.

### Layout Invariants
- `sum_child_sizes()` must equal parent dimension.
- Minimum pane: 2 cols x 1 row.
- Remainder distribution is left-to-right / top-to-bottom.

### Signal Invariants
- Signal handlers write to pipes only.
- SIGCHLD drains waitpid(-1, WNOHANG) in a loop.
- SIGWINCH triggers query -> propagate -> render.

### Channel Invariants
- All channels have fixed bounds.
- No unbounded channels anywhere.
- Bounds: control=512, data=1024, render=256, effect=1024.
