# Vibe-Tmux: North Star Architecture

This document serves as the architectural blueprint for `vibe-tmux`, a next-generation terminal multiplexer written in Rust. It aims for **100% strict tmux compatibility** while introducing a modern, layered architecture, an ORM-like API for automation, and CRDT-backed state management for robustness and future distributed capabilities.

## 1. Vision & Core Philosophy

**"Strictly Compatible, Architecturally Liberated."**

*   **User Perspective:** It *is* tmux. It reads `.tmux.conf`, accepts `tmux` commands, and behaves exactly as expected.
*   **Developer Perspective:** It is a modern Rust library ecosystem. You can import `vibe` into your Rust project to manage sessions programmatically (ORM-like) without spawning shell processes.
*   **System Perspective:** It is a resilient, async, event-driven system where state is immutable or versioned (CRDT), ensuring stability even under heavy load or distributed scenarios.

## 2. High-Level Architecture

The project is structured as a workspace with distinct crates, ensuring separation of concerns and enabling the "library-first" approach.

```mermaid
graph TD
    CLI[vibe-cli (bin)] --> API[vibe-api (lib)]
    API --> Proto[vibe-protocol]
    API --> Server[vibe-server]
    Server --> Core[vibe-core]
    Server --> PTY[vibe-pty]
    Server --> Render[vibe-render]
    Core --> Proto
```

### The Crate Ecosystem

| Crate | Responsibility | Dependencies |
| :--- | :--- | :--- |
| **`vibe-core`** | Domain models (Session, Window, Pane), State (CRDT), Business Logic. **Pure Rust, no I/O.** | `serde`, `thiserror`, `crdt_engine` |
| **`vibe-protocol`** | Command enums, IPC messages, serialization, `tmux` command parsing. | `vibe-core`, `serde` |
| **`vibe-pty`** | Abstraction over PTY interaction (Linux/macOS/BSD specific). | `nix`, `libc` |
| **`vibe-render`** | Grid model, Sixel support, ANSI sequence parsing (VTE), Diff-based rendering. | `ratatui` (maybe), `vte` |
| **`vibe-server`** | The `tokio` runtime, Client connection handling, State Actor, Event Loop. | `vibe-core`, `vibe-protocol`, `tokio` |
| **`vibe-api`** | The "ORM" high-level library. Provides the `libtmux`-like experience. | `vibe-protocol`, `ipc-channel` |
| **`vibe-cli`** | The binary entry point. Parses args, invokes `vibe-api` or connects to server. | `clap`, `vibe-api` |

## 3. Detailed Component Design

### 3.1. `vibe-core`: The CRDT State Model

Instead of mutable global state (like C tmux's global structures), `vibe-core` uses a **Command-Sourcing** or **CRDT** approach.

*   **State:** A single `State` struct containing `HashMap<SessionId, Session>`, `HashMap<WindowId, Window>`, etc.
*   **Mutation:** State is *never* mutated directly by the server loop. It is mutated by applying an `Action`.
*   **CRDT Integration:** The State is backed by a CRDT (Conflict-free Replicated Data Type) structure. This allows:
    *   **Transactional Updates:** Multiple commands can be batched atomically.
    *   **History/Undo:** Trivial implementation of undo/redo.
    *   **Distributed Future:** Synchronization between two `vibe` servers over SSH (mosh-style).

```rust
// vibe-core/src/state.rs

pub struct State {
    sessions: LWWMap<SessionId, Session>, // Last-Write-Wins Map
    // ...
}

pub enum Action {
    NewSession { name: String, ... },
    SplitWindow { target: PaneId, direction: Direction },
    ResizePane { id: PaneId, delta: isize },
}

impl State {
    pub fn apply(&mut self, action: Action) -> Result<Event, Error> {
        // ... applies to CRDT ...
    }
}
```

### 3.2. `vibe-api`: The "ORM" Layer

This is the "Killer Feature" for developers. It mirrors Python's `libtmux` but is typesafe and native.

*   **Mode 1: IPC Mode:** Connects to a running server.
*   **Mode 2: Embedded Mode:** Runs the core logic in-process (headless).

```rust
use vibe_api::{Client, SessionBuilder};

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::connect().await?;
    
    // ORM-like usage
    let session = client.new_session("my_project").create().await?;
    let window = session.active_window().await?;
    
    // Split pane and run command
    let pane_bottom = window.split_window().direction(Split::Vertical).exec().await?;
    pane_bottom.send_keys("cargo test").await?;
    
    Ok(())
}
```

### 3.3. `vibe-server`: The Async Engine

*   **Runtime:** `tokio`.
*   **Architecture:** Actor Model.
    *   **ClientActor:** Handles one connected client (socket).
    *   **PtyActor:** Manages one running shell process. Reads stdout/stderr, writes stdin.
    *   **StateActor:** Owns the `vibe-core::State`. Receives `Action` messages, applies them, and broadcasts `StateChanged` events.
*   **Concurrency:**
    *   C tmux is single-threaded. `vibe` will be multi-threaded but state-safe.
    *   Rendering can happen on a separate thread from PTY reading.

### 3.4. Compatibility Layer

To achieve 100% compatibility, we treat the `tmux` CLI syntax as a "DSL" that compiles down to `vibe-core` Actions.

*   **Parser:** A parser in `vibe-protocol` that takes `["new-session", "-s", "foo"]` and converts it to `Action::NewSession { name: "foo" }`.
*   **Config:** A parser for `.tmux.conf` that maps legacy settings to the new internal configuration structs.

## 4. Testing Strategy (The "Test Framework" Aspect)

Since `vibe-core` is pure Rust and decoupled from I/O, we can write **deterministic simulation tests**.

*   **Headless Testing:** Spin up `State` in memory, apply 1000 actions, assert the final layout. No actual PTYs or logic required.
*   **Fuzzing:** Fuzz the `apply(Action)` method to find invalid state transitions.
*   **Snapshot Testing:** Render the Grid to a string and compare against expected visual output.

## 5. Development Plan & AGENTS.md Rules

### Phase 1: The Skeleton
1.  Setup workspace (`core`, `protocol`, `server`, `cli`).
2.  Implement `vibe-core` with basic `Session`/`Window`/`Pane` structs (no logic yet).
3.  Implement `vibe-protocol` with basic `Action` enum.

### Phase 2: The Loop
1.  Implement `vibe-server` with `tokio`.
2.  Create the `StateActor`.
3.  Get a basic `vibe-cli` connecting to the server and sending a "Ping".

### Phase 3: The PTY & Grid
1.  Integrate `portable-pty` or custom `vibe-pty`.
2.  Implement `vibe-render` (grid storage).
3.  Connect PTY output -> Grid -> Client.

### Phase 4: Compatibility & Polish
1.  Implement `tmux` command parser.
2.  Implement `.tmux.conf` loader.
3.  Implement the ORM API.

### `AGENTS.md` Rules for AI Contributors

1.  **Strict Layering:** Never import `tokio` in `vibe-core`. Core must remain pure.
2.  **Compatibility First:** If a feature exists in tmux, implement it exactly as specified (flags, behavior).
3.  **Test the Core:** Every change to logic in `vibe-core` requires a unit test.
4.  **Async Safety:** Use channels for communication. Avoid `Mutex<GlobalState>` if possible; prefer the Actor pattern.

---

**Risks & Mitigations:**
*   **Performance:** CRDTs can be slow. *Mitigation:* Use highly optimized CRDT libraries or a simplified "Last-Write-Wins" map for most state, only using full CRDT logs for text buffers if needed (though text buffers are usually just byte streams).
*   **Terminal Quirks:** TTY handling is notoriously hard. *Mitigation:* Lean heavily on `alacritty_terminal` or `ratatui` underlying libraries initially.
