# AGENTS.md -- Quality Gates and Agent Rules

## Quality Gates

### Gate 1: Compilation (Blocking)
- `cargo check` must pass on all workspace members.
- `cargo clippy -- -D warnings` must pass.
- Zero `unsafe` outside `mux-pty` and `mux-ffi`.

### Gate 2: Tests (Blocking)
- `cargo test` must pass with zero failures.
- Every public function has at least one unit test.
- New crates must have at least 3 tests before merge.

### Gate 3: Documentation (Blocking)
- All public types and functions have doc comments.
- All crate root modules have module-level documentation.
- Error types document when each variant is returned.

### Gate 4: Architecture (Non-blocking, reviewer-assessed)
- New crates follow the dependency layer rules (L0-L6).
- Channel bounds are documented for any new channel.
- Invariants referenced in code comments match CLAUDE.md list.

### Gate 5: Performance (Non-blocking, benchmark-assessed)
- Grid operations: < 1us per cell mutation.
- Render diff: < 1ms for 80x24 terminal with 10% dirty cells.
- VT parser: > 100 MB/s throughput.

## Invariant Rules

- INV-nnn references MUST be added when implementing code that enforces an invariant.
- New invariants MUST be added to CLAUDE.md before being referenced in code.
- Invariant violations in tests MUST use `assert!` with the INV-nnn reference.

## Layout Engine Rules

- Layout algorithms MUST produce the same geometry as tmux for the same input.
- Custom layout string parsing MUST handle tmux's checksum format.
- Minimum pane size is 2 columns x 1 row (matching tmux).
- Resize MUST NOT lose panes; if space is insufficient, return an error.

## Copy Mode Rules

- Copy mode MUST support both vi and emacs key bindings.
- Selection modes: char, word, line, rectangle.
- Search MUST support both literal and regex patterns.
- Copy mode exit MUST restore the live view at the current scroll position.
- Paste buffers are a LIFO stack (matching tmux).

## Graphics Protocol Rules

- Graphics passthrough MUST be disabled by default (INV-220).
- DCS passthrough sequences MUST be length-limited (16 MiB max).
- Graphics data MUST NOT be stored in the grid's Cell struct.
- Graphics rendering MUST be delegated to the render thread.

## Signal Handling Rules

- SIGWINCH and SIGCHLD MUST use signal-hook pipe (INV-200, INV-210).
- MUST NOT install handlers for SIGTSTP, SIGCONT, SIGPIPE (INV-230).
- All handlers MUST use SA_RESTART (INV-231).
- Only the IO thread may call waitpid (INV-212).

## Workflow: Adding a New Crate

1. Create `crates/<name>/Cargo.toml` with workspace references.
2. Create `crates/<name>/src/lib.rs` with `#![forbid(unsafe_code)]` (unless quarantine).
3. Add to `[workspace.members]` in root Cargo.toml.
4. Add to `[workspace.dependencies]` as internal dep.
5. Add to the layer diagram in CLAUDE.md.
6. Add at least 3 unit tests.
7. Run `cargo check` and `cargo test`.

## Workflow: Adding a New Tool

1. Create `tools/<name>/Cargo.toml` with workspace references.
2. Create both `src/lib.rs` and `src/main.rs`.
3. `src/main.rs` calls into `src/lib.rs` for testability.
4. Add to `[workspace.members]` in root Cargo.toml.
5. Add at least 1 integration test via `assert_cmd`.
