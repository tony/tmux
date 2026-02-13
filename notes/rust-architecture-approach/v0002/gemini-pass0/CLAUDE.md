# TermForge AI Agent Conventions

## Core Philosophy
TermForge is an **SDK-first** terminal multiplexer. We build libraries that happen to power a daemon.
- **Protocol:** Perfect tmux wire-protocol v8 compatibility is mandatory.
- **Architecture:** Clean-room implementation. NOT a port of C code.
- **Threading:** 3-thread model (Kernel, IO, Render).

## Coding Standards
- **Errors:** Use `thiserror` for library crates, `anyhow` for binaries/tools.
- **Async:** `tokio` for IO. `std::thread` for Kernel (logic owner).
- **Safety:** No `unsafe` allowed outside `mux-pty` and `mux-ffi`.
- **Panics:** No `unwrap()` or `expect()` in library code. Propagate `Result`.
- **Comments:** Explain *why*, not *what*. Reference tmux behavior/flags where relevant (e.g., `// Matches GRID_FLAG_PADDING`).

## Type Invariants
- **Grid:** Chunked COW (`Vec<Arc<LineChunk>>`). Clone is cheap.
- **Channels:** All IO is bounded. Kernel is sans-IO.
- **IDs:** Use `slotmap` keys (SessionId, WindowId, PaneId).

## Testing Patterns
- **Unit:** Standard `#[test]`.
- **Snapshot:** Use `insta::assert_snapshot!` for Grid/Terminal state.
- **Integration:** Use `mux-termlet` for interaction tests.
- **Isolation:** ALWAYS use unique socket paths (`/tmp/tf-test-{pid}-{uuid}`).

## Architecture References
- **TF01:** Native wire protocol (Magic: 0x54).
- **v8:** tmux compatibility protocol.

## What NOT To Do
- **Do NOT** introduce global mutable state (statics).
- **Do NOT** use `SmallVec` for grid lines (proven inefficient for this specific use case).
- **Do NOT** assume standard standard streams (stdin/stdout) in the kernel.
--- END FILE ---
