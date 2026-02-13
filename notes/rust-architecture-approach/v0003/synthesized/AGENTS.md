# AGENTS.md -- Agent Roles, Quality Gates, and tmux Reference

## What This Codebase Is

TermForge is a Rust-native terminal multiplexer that aims for tmux protocol
compatibility while providing a modern SDK-first API. The codebase follows
a strict crate layering from L0 (no deps) through L6 (user-facing tools).

## What Agents Should Know

1. **Sans-IO kernel**: The kernel never does I/O. It receives events and
   returns effects. If you need to add functionality that requires I/O,
   add a new KernelEffect variant and handle it in the IO layer.

2. **Crate layers**: Respect the dependency DAG. L0 crates (grapheme-arena,
   time) must not depend on anything. L1 (types) depends only on L0.
   Never create circular dependencies.

3. **Cell flags**: GRID_FLAG values are verified against tmux source
   (tmux.h:742-749). Do not change these values without verifying against
   the tmux C source at /home/d/study/c/tmux/.

4. **Testing**: Always add tests when adding code. Use `Clock::manual()`
   for deterministic time. Use `TestServer` from mux-test-support for
   integration testing. Socket paths must use `isolated_socket_path()`.

5. **No panics**: The workspace denies `unwrap_used`, `expect_used`, and
   `panic` via clippy. Use proper error handling with Result types.

6. **Unsafe quarantine**: Only mux-pty and mux-ffi may use unsafe code.
   Both must use `#![deny(unsafe_op_in_unsafe_fn)]` and every unsafe
   block must have a `// SAFETY:` comment.

7. **Rust 2024**: The `gen` keyword is reserved. Use `id_gen` instead.
   FFI functions use `#[unsafe(no_mangle)]` syntax.

## Agent Roles

### Architect Agent
- Maintains the crate DAG and layering rules.
- Ensures no cross-layer dependency cycles.
- Reviews Cargo.toml changes for dependency correctness.
- Enforces the 6-layer hierarchy (L0-L6).
- Validates that new crates are placed in the correct layer.
- Reviews architecture.md for accuracy after changes.

### Implementation Agent
- Writes crate implementations following coding rules.
- Ensures `#![forbid(unsafe_code)]` in all crates except mux-pty and mux-ffi.
- References tmux source file and line for invariant verification.
- Uses `thiserror` for library errors, `anyhow` only in binaries.
- Follows the multi-file module pattern for larger crates.
- Uses `#[must_use]` on constructors and pure functions.

### Test Agent
- Writes and maintains tests for all crates.
- Ensures deterministic tests (no wall-clock sleeps).
- Uses `Clock::manual()` for time-dependent tests.
- Maintains minimum 12 tests per L4-L6 crate, 3 per L0-L3 crate.
- Runs property tests for invariant verification.
- Ensures mux-api has 20+ tests with builder pattern demonstrations.
- Ensures mux-termlet has 20+ tests with full lifecycle coverage.
- Writes proptest for grid COW, parser fuzz, render diff, CRDT convergence.

### Review Agent
- Enforces quality gates before merge.
- Checks invariant documentation completeness.
- Verifies gap coverage (all 12 identified gaps).
- Runs full test suite, clippy, and fmt.
- Validates documentation on new public items.

## Style

- Module organization follows multi-file pattern for larger crates
  (mux-types, mux-parser, mux-grid, mux-kernel, mux-render).
- Smaller crates use single-file lib.rs.
- All public types derive Debug.
- Use `#[must_use]` on constructors and pure functions.
- Comments explain why, not what.
- Use `const fn` where possible.

## tmux Source Reference

The tmux C source is at `/home/d/study/c/tmux/`. Key files:
- `tmux.h`: Core type definitions, flag constants (GRID_FLAG_* at lines 742-749)
- `grid.c` / `grid-reader.c` / `grid-view.c`: Grid implementation
- `input.c`: VT parser / input processor (state machine)
- `layout.c` / `layout-set.c` / `layout-custom.c`: Layout engine
- `cmd-*.c`: Command implementations (tmux CLI commands)
- `tty.c` / `tty-keys.c` / `tty-term.c` / `tty-features.c`: Terminal output / escape sequences
- `screen.c` / `screen-redraw.c` / `screen-write.c`: Screen buffer management
- `server.c` / `server-fn.c` / `server-client.c`: Server event loop
- `client.c`: Client implementation
- `session.c` / `window.c`: Session/window entity management
- `options.c` / `options-table.c`: Options system
- `format.c`: Format string expansion
- `paste.c`: Paste buffer management
- `key-string.c` / `key-bindings.c`: Key handling
- `control.c` / `control-notify.c`: Control mode (tmux -C)
- `job.c` / `spawn.c`: Process spawning
- `style.c`: Style parsing (colours, attributes)

## Quality Gates

### Gate 1: Compilation
- `cargo check` must pass with zero errors.
- `cargo check --tests` must pass.

### Gate 2: Tests
- `cargo test --workspace` must pass with zero failures.
- Target: 500+ tests (current scaffold).
- All tests must be deterministic.
- No wall-clock sleeps or timing-dependent assertions.

### Gate 3: Lints
- `cargo clippy --workspace --all-targets -- -D warnings` must pass.
- Exceptions documented in workspace Cargo.toml.
- No clippy::unwrap_used, clippy::expect_used, or clippy::panic.

### Gate 4: Documentation
- All public items have doc comments.
- Functions returning Result have `# Errors` sections.
- Invariants reference tmux source lines.
- Architecture docs updated when architecture changes.

### Gate 5: Architecture
- No cross-layer dependency cycles.
- New dependencies reviewed by Architect Agent.
- `#![forbid(unsafe_code)]` maintained in all safe crates.
- New crates placed in correct layer.

## Invariant Rules

### INV-117: Panic Recovery
- FFI boundaries use `catch_unwind`.
- `panic = "unwind"` required in all profiles (dev, test, release).
- Clippy denies `unwrap_used`, `expect_used`, `panic`.
- FFI functions return null/0 on panic, never propagate.

### INV-119: CellFlags Compatibility
- Bit values must match tmux.h:742-749 exactly:
  - PADDING = 0x04, EXTENDED = 0x08, SELECTED = 0x10
  - CLEARED = 0x40, TAB = 0x80
- Tests verify each flag value against known constants.
- Changes require cross-referencing tmux C source.

### INV-220: Graphics Passthrough
- `allow-passthrough` defaults to `false`.
- Tests verify the default is off.
- API builder preserves the default.
- DCS sequences consumed and discarded when disabled.

### Layout Invariants
- `sum_child_sizes()` must equal parent dimension.
- Minimum pane: 2 cols x 1 row.
- Remainder distribution is left-to-right / top-to-bottom.
- Verified via proptest (any number of panes, any total width).

### Signal Invariants
- Signal handlers write to pipes only.
- SIGCHLD drains waitpid(-1, WNOHANG) in a loop.
- SIGWINCH triggers query -> propagate -> render.
- Job control signals not intercepted (SIGTSTP, SIGCONT, etc.).

### Channel Invariants
- All channels have fixed bounds.
- No unbounded channels anywhere.
- Bounds: control=512, data=1024, render=256, effect=1024.
- Backpressure strategy documented per channel.

## v0003 Delta Rules

1. Prefer `mux-api` builder usage in examples and tests; this is the primary SDK surface.
2. Keep `allow_passthrough` disabled unless a test explicitly enables it.
3. Signal handling remains pipe-based: handlers enqueue only; no heavy work in handlers.
4. For PTY peer opening, preserve `TIOCGPTPEER`-first strategy on Linux with fallback path.
5. Keep `mux-fdpass` envelope cap at 16 fds per message for tmux-compat mode.
6. Every L4-L6 crate must have at least 12 tests.
7. `mux-api` and `mux-termlet` must each have at least 20 tests demonstrating SDK ergonomics.
8. proptest coverage must include: grid COW isolation, parser fuzz, render diff roundtrip, CRDT convergence, layout dimension sums, colour pack/unpack.
9. architecture.md must cover all 26 sections (1,000+ lines).
10. plan.md must cover all 10 phases with exit criteria (800+ lines).
