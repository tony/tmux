# TermForge Architecture v0002

## 1. Threading Model
We use a **3-thread architecture** to balance safety, simplicity, and performance.

1.  **Kernel Thread (`std::thread`)**:
    -   **Role**: The "Owner". Owns the entire `Server` state (Sessions, Windows, Panes, Grids).
    -   **Concurrency**: Single-threaded. No `Mutex<Server>` needed inside the kernel.
    -   **Communication**: Receives `Msg` enums from IO/Render threads via `crossbeam::channel`.
    -   **Logic**: Layout calculations, state mutations, command processing.

2.  **IO Thread (`tokio`)**:
    -   **Role**: Handles all external I/O (Sockets, PTY master fds, Signals).
    -   **Concurrency**: Async (M:N).
    -   **Duty**: Reads bytes, parses frames, sends `Msg::ClientInput` or `Msg::PtyOutput` to Kernel. Writes bytes back to clients/PTYs.

3.  **Render Thread (`tokio`)**:
    -   **Role**: Composes the view and generates output sequences.
    -   **Duty**: Receives "Render Ops" or Grid snapshots from Kernel. Diffs against previous state. Emits VT sequences to the IO thread for delivery to clients.

## 2. The Grid (Data Structure)
The terminal grid is the most memory-intensive component.
-   **Structure**: `Vec<Arc<LineChunk>>` where `CHUNK_SIZE = 256` (lines).
-   **COW**: Copy-On-Write at the chunk level. A snapshot of a Grid is cheap (cloning the Vec of Arcs).
-   **Line**: `Vec<Cell>`. `Cell` is a packed struct (char + attrs).
-   **Dirty Tracking**: `BitVec` for line dirtiness. Global revision counter.

## 3. Wire Protocol (Dual Mode)
TermForge speaks two languages:
1.  **TF01**: Native, optimized Rust protocol.
    -   Header: `Magic (4B) | Ver (2B) | Flags (2B) | Len (4B) | Type (4B)`
    -   Magic: `0x54463031` ("TF01")
2.  **tmux imsg (v8)**: Legacy support.
    -   We implement the full `imsg` struct layout.
    -   Allows `tmux attach` to connect to a TermForge server.

## 4. Termlets
Termlets are testing pods.
-   They combine a PTY, a hidden Grid, and a simplified API.
-   Used for integration testing TermForge itself, AND exported as a library for users to test their TUI apps.

## 5. Panic Recovery
-   The Kernel thread runs inside `std::panic::catch_unwind`.
-   On panic, we attempt to flush a final "Crash Report" frame to all clients before shutting down.
-   State is potentially corrupt, so we do not attempt to continue, but we exit cleanly (reaping PTYs).

## 6. OpenTelemetry
-   Every "Action" (user keypress, timer, signal) starts a Trace Span.
-   Spans propagate across threads (IO -> Kernel -> Render).
-   Critical for debugging latency in the mux pipeline.
--- END FILE ---
