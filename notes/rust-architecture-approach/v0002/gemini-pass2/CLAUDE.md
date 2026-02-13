# Developer Guide & Invariants (CLAUDE.md)

## Build Commands
- **Build**: `cargo build`
- **Check**: `cargo check` (Must pass with 0 errors)
- **Test**: `cargo test`
- **Release**: `cargo build --release`
- **Lint**: `cargo clippy`

## Coding Standards
- **Edition**: 2024
- **MSRV**: 1.85
- **Unsafe**: MUST be quarantined in `unsafe {}` blocks with `// SAFETY:` comments.
- **Panic**: `unwrap()` and `expect()` are FORBIDDEN in `mux-kernel` and `mux-server` logic. Use `Result` and propagate errors. Panics are only allowed during startup/initialization.
- **Async**: We use `mio` for polling. `tokio` is avoided in the core kernel to keep the footprint small and deterministic, though the binary entry points may use it if necessary (currently preferring synchronous polling for the muxer).

## System Invariants
The system relies on strict invariants. Violating these leads to undefined behavior or deadlocks.

**INV-117 (Layout Integrity)**: The sum of `LayoutCell` sizes in a split MUST equal the parent size (minus separators).
**INV-120 (Grid Bounds)**: Accessing `(x, y)` outside `(width, height)` MUST return the default cell or `None`, never panic.
**INV-135 (Raw Mode)**: `RawModeGuard` MUST restore the TTY state when dropped.
**INV-142 (Signal Safety)**: Signal handlers MUST only write to a lock-free atomic flag or self-pipe. NO complex logic.
**INV-150 (PTY Ownership)**: The Server process owns the PTY Master FD. The Client NEVER owns the PTY FD directly.
**INV-166 (Render Idempotency)**: Rendering the same Grid state twice MUST produce identical output (or empty diff).
**INV-189 (Event Ordering)**: User input events MUST be processed in FIFO order.
**INV-201 (Zombie Reaping)**: The server MUST reap all child processes to prevent zombie accumulation.
**INV-210 (UTF-8)**: All internal string handling MUST be valid UTF-8. Binary data is handled as `Vec<u8>`.
**INV-231 (Channel Backpressure)**: If the Data channel is full, the PTY reader MUST stop reading (apply backpressure to the OS buffer).

## Thread Architecture
1.  **Main Thread (Kernel)**: Runs the `mio` event loop. Handles PTY I/O, Signals, Client Connections, Layout logic.
2.  **Render Thread (Optional)**: In high-perf modes, rendering (diffing) can be offloaded. Default is single-threaded for simplicity.
3.  **IO Threads**: `mux-io` may spawn threads for blocking file operations if necessary (rare).

## Channel Bounds
- **Control**: 512
- **Data**: 1024 (64KiB chunks)
- **Render**: 256
- **Effect**: 1024

## What NOT to Do
- **Do NOT use `crossterm`** for core terminal logic. We manage our own TTY state.
- **Do NOT use `vt100` crate**. We use our own `mux-parser` for precise control over state transitions.
- **Do NOT use unbound channels**. Always use `bounded`.
- **Do NOT block the main loop**. Any blocking operation > 1ms is a bug.
