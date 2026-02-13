# AGENTS.md

## Project Rules for AI Agents

### What this codebase is

TermForge is a Rust-native terminal multiplexer that aims for tmux protocol
compatibility while providing a modern SDK-first API. The codebase follows
a strict crate layering from L0 (no deps) through L6 (user-facing tools).

### What agents should know

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
   integration testing.

5. **No panics**: The workspace denies `unwrap_used`, `expect_used`, and
   `panic` via clippy. Use proper error handling with Result types.

6. **Unsafe quarantine**: Only mux-pty and mux-ffi may use unsafe code.
   Both must use `#![deny(unsafe_op_in_unsafe_fn)]` and every unsafe
   block must have a `// SAFETY:` comment.

### Style

- Module organization follows GPT's multi-file pattern for larger crates
  (mux-types, mux-parser, mux-grid, mux-kernel, mux-render).
- Smaller crates use single-file lib.rs.
- All public types derive Debug.
- Use `#[must_use]` on constructors and pure functions.

### tmux source reference

The tmux C source is at `/home/d/study/c/tmux/`. Key files:
- `tmux.h`: Core type definitions, flag constants
- `grid.c`: Grid implementation
- `input.c`: VT parser / input processor
- `layout.c` / `layout-set.c`: Layout engine
- `cmd-*.c`: Command implementations
- `tty.c`: Terminal output / escape sequences
