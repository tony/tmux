# TermForge Architecture Reference v0002 (Pass 1)

**Status:** Draft / Refinement
**Pass:** 1 (Gemini)
**Previous:** Pass 0 (Scaffolding)

## 1. Executive Summary

TermForge is a Software Development Kit (SDK) for building terminal multiplexers in Rust. Unlike a monolithic application (like `tmux`), TermForge provides the building blocks—PTY management, VT emulation, Grid state, Layout engine, and Rendering—as a library. This allows developers to build custom multiplexers, terminal emulators, or TUI workspaces with `tmux` wire-protocol compatibility.

This document details the architectural decisions for **Pass 1**, specifically addressing the gaps identified in the initial scaffolding (composition, layouts, signals, etc.).

## 2. Core Architecture: The "Tri-Thread" Model

As established in Pass 0, TermForge uses a strict 3-thread model to separate concerns and ensure responsiveness.

1.  **IO Thread (`mux-term`):**
    *   **Responsibility:** Interfacing with the OS.
    *   **Tasks:** PTY reads/writes, signal handling (SIGWINCH, SIGCHLD), socket listening (Unix Domain Sockets), reading stdin/writing stdout.
    *   **Runtime:** `tokio` (Async).
2.  **Kernel Thread (`mux-kernel`):**
    *   **Responsibility:** The "Brain". Single source of truth.
    *   **Tasks:** Managing state (Sessions, Windows, Panes), Layout logic, VT parsing (writing bytes to Grid), handling User Commands.
    *   **Runtime:** `std::thread` (Synchronous, Event Loop). Sans-IO design.
3.  **Render Thread (`mux-render`):**
    *   **Responsibility:** Output generation.
    *   **Tasks:** Composing panes into a virtual screen, diffing against the previous frame, generating optimized ANSI output.
    *   **Runtime:** `tokio` (Async/Sync hybrid).

## 3. Critical Gap Resolutions

### 3.1. Composition & Diff Rendering Strategy

**Problem:** How do we efficiently merge multiple pane grids into a single terminal output without excessive copying or tearing?

**Decision:** **Snapshot-based Composition with Double-Buffering and Dirty-Line Diffing.**

1.  **Snapshotting:** The Kernel pushes an immutable `RenderState` to the Render Thread. This state contains `Arc` pointers to the Pane Grids (using the Chunked COW structure). This is cheap (pointer copies).
2.  **Virtual Surface:** The Renderer maintains two buffers: `FrontBuffer` (what is on screen) and `BackBuffer` (what we are building).
3.  **Composition:**
    *   The `BackBuffer` is cleared/resized.
    *   Iterate through visible Panes in the Layout.
    *   For each Pane, copy its visible region (viewport) into the `BackBuffer` at the correct offset.
    *   *Optimization:* If a Pane hasn't changed (dirty bit check), we skip copying if we know the Layout hasn't moved.
4.  **Diffing:**
    *   Compare `BackBuffer` vs. `FrontBuffer`.
    *   Use a **Cost-Based Diff Algorithm**:
        *   Skip identical lines.
        *   For changed lines, compute the cost of redrawing the whole line vs. moving the cursor and updating changed cells.
        *   Output minimal ANSI sequence.
5.  **Output Coalescing:**
    *   Rendering is triggered by a "Dirty" flag, but capped at a max framerate (e.g., 60 FPS) to prevent output flooding.

**Code Concept (`mux-render/src/lib.rs`):**

```rust
pub struct RenderState {
    pub cursor_visible: bool,
    pub cursor_pos: (u16, u16),
    pub panes: Vec<PaneRenderData>, // Position, Z-index, content (Arc<Grid>)
}

pub struct Renderer {
    front: Surface,
    back: Surface,
    term: TerminalCaps,
}

impl Renderer {
    pub fn render(&mut self, state: RenderState) -> String {
        self.compose(&state);
        let output = self.diff();
        self.front.swap(&self.back);
        output
    }
}
```

### 3.2. Layout Engine

**Problem:** How to support tmux's 5 standard layouts + arbitrary user layouts?

**Decision:** **Binary Space Partitioning (BSP) Tree.**

*   **Structure:** A `LayoutNode` is either a `Leaf` (Pane) or a `Split` (Vertical/Horizontal, containing 2 children).
*   **Resize Algorithm:** Recursive redistribution. When the root resizes, it propagates the available space down based on:
    *   Fixed size constraints (if any).
    *   Percentage-based distribution.
*   **Tmux Compatibility:** We map standard tmux layouts (Main-Vertical, Tiled, etc.) to specific BSP tree configurations.

**Data Structure (`mux-kernel/src/layout.rs`):**

```rust
pub enum SplitType { Vertical, Horizontal }

pub struct LayoutNode {
    pub id: NodeId,
    pub size: Rect, // Calculated absolute position
    pub kind: NodeKind,
}

pub enum NodeKind {
    Pane(PaneId),
    Split {
        dir: SplitType,
        ratio: f32, // 0.0 - 1.0
        left: Box<LayoutNode>,
        right: Box<LayoutNode>,
    },
}
```

### 3.3. Raw Mode Lifecycle

**Problem:** Who owns the terminal state?

**Decision:** **Explicit `TerminalMode` State Machine, owned by `mux-api`.**

*   **Standalone Mode:** The SDK (via `mux-api::Builder`) sets the terminal to Raw Mode on startup and restores it on shutdown/panic.
*   **Library Mode:** The caller tells the SDK "I am already in raw mode" or "Manage it for me".
*   **Invariant:** `Drop` implementation on the `TermInterface` MUST restore canonical mode to prevent a broken terminal on crash.

### 3.4. SIGWINCH Propagation Flow

**Decision:** Strict linear propagation.

```text
[OS] -> SIGWINCH
  |
[IO Thread (mux-term)]
  |-- (Debounce 10ms)
  |-- ioctl(TIOCGWINSZ) -> Get new physical size
  |-- Send KernelEvent::Resize(w, h)
  v
[Kernel Thread (mux-kernel)]
  |-- Update Session/Window dimensions
  |-- Layout Engine::resize(w, h) -> Recalculate Pane Rects
  |-- For each Pane:
       |-- Resize Grid
       |-- Send PtyInstruction::Resize(cols, rows) -> to IO Thread
       |-- Trigger Render
  v
[IO Thread]
  |-- ioctl(TIOCSWINSZ) on specific PTY fds
```

### 3.5. Zombie Reaping (SIGCHLD)

**Problem:** Determining when a pane's process dies.

**Decision:** **`signal-hook` + `waitpid` loop.**

*   We use `signal-hook` to catch `SIGCHLD` in the IO Thread.
*   When received, we call `waitpid(-1, WNOHANG)` in a loop until it returns -1.
*   We map the returned `pid` to a `PaneId` (via a synchronized map).
*   Send `KernelEvent::PaneExited(PaneId, ExitStatus)` to the Kernel.

### 3.6. `crossterm` vs `nix`/`rustix`

**Decision:** **Reject `crossterm`. Use `rustix` (via `nix` or direct).**

*   **Rationale:** `crossterm` is excellent for *applications* (TUIs), but TermForge is an *infrastructure SDK*.
    *   We need raw access to PTYs (`openpty`, `forkpty`).
    *   We need precise control over signal masks (blocking SIGWINCH in some threads).
    *   We need `SCM_RIGHTS` (FD passing) which `crossterm` does not cover.
    *   `crossterm`'s global state management for raw mode interferes with our explicit lifecycle requirements.

### 3.7. `vt100` vs `termwiz` vs Custom

**Decision:** **Custom `Grid` + `vte` crate for parsing.**

*   **Parser:** Use `vte` (Alacritty's parser). It is fast, correct, and state-machine based.
*   **Grid:** Custom.
    *   **Why?** We need specific `tmux` compatibility flags (CellFlags) that other crates don't mirror exactly (e.g., `PADDING` bit).
    *   **Architecture:** `Vec<Arc<LineChunk>>` (COW).
*   **Diffing:** Custom (as described in 3.1). `vt100::contents_diff` is good but tightly coupled to its own grid representation.

### 3.8. Copy Mode

**Decision:** **Overlay Mode in Kernel.**

*   **State:** `Pane` has a `mode: PaneMode` field.
*   **Enum:** `PaneMode::Standard` vs `PaneMode::Copy(CopyState)`.
*   **CopyState:** Contains:
    *   `scroll_offset: usize`
    *   `selection_start: Option<Point>`
    *   `cursor: Point`
    *   `search_query: Option<String>`
*   **Rendering:** The Renderer checks `PaneMode`. If `Copy`, it renders the scrollback buffer instead of the active grid, and overlays the selection highlight.

### 3.9. Graphics Protocol Stance

**Decision:** **Passthrough (Primary), Sixel (Secondary/Planned).**

*   **v1:** Support `tmux`'s passthrough escape sequence (`\033Ptmux;...`). The parser detects this and forwards the payload directly to the client if enabled.
*   **v2:** Parse Sixel/Kitty graphics to a virtual frame buffer for resizing/redrawing. (Deferred).

### 3.10. `signal-hook` vs `tokio` signals

**Decision:** **`signal-hook` with `mio-0.8` integration.**

*   **Rationale:** `signal-hook` provides unsafe but necessary access to `sigaction`, `sigprocmask`.
*   We need to mask signals in the Kernel thread so they are *only* delivered to the IO thread. `tokio` signals are convenient but `signal-hook` allows iterator-based non-async consumption which bridges well with our `mio` or `poll` based loops if needed, though we will likely feed them into a channel for the tokio runtime.

## 4. Other Key Decisions

### 4.1. CRDT & Collaboration
To support multiple clients viewing the same session (collaborative coding), the Kernel is the authority.
*   Input is serialized to a `Log`.
*   We use a simplified Last-Write-Wins (LWW) approach for settings.
*   Text content is not CRDT-managed; the PTY is the source of truth. We sync *views*, not *files*.

### 4.2. SCM_RIGHTS (FD Passing)
For `tmux` compatibility (client-server model), we must support passing file descriptors over Unix Sockets.
*   Use `nix::sys::socket::sendmsg` with `ControlMessage::ScmRights`.
*   Used for handing off the PTY FD from a dying server to a new one (seamless upgrade) or attaching a client's TTY.

### 4.3. Configuration
*   **Format:** TOML (serialized via `serde`).
*   **Hot Reload:** The IO thread watches the config file (using `notify` crate) and sends `KernelEvent::ConfigReload` to the Kernel.

### 4.4. Panic Recovery
*   Use `std::panic::set_hook`.
*   On panic, the hook:
    1.  Restores terminal to Canonical mode (Critical!).
    2.  Prints the stack trace to `stderr`.
    3.  Attempts to dump the current Session state to a crash file in `/tmp`.

## 5. Crate Structure (Refined)

*   `mux-api`: **(New)** Public-facing SDK. `TermForge::builder()...`.
*   `mux-kernel`: Logic, State, Layouts.
*   `mux-protocol`: Wire format, Primitives (`Cell`, `Style`).
*   `mux-render`: Composition, Diffing, ANSI generation.
*   `mux-term`: IO, PTYs, Signals, Sockets.
