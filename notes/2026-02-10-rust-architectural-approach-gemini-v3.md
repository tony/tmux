# TermForge: A Layered, CRDT-Ready Terminal Multiplexer with tmux Compatibility

## 1. Vision & Core Philosophy

**"Strictly Compatible, Architecturally Liberated."**

*   **User Perspective:** It *is* tmux. It reads `.tmux.conf`, accepts standard tmux commands, and real tmux clients can attach to it. It behaves exactly as expected, matching the behavior of the C reference implementation in `~/study/c/tmux`.
*   **Developer Perspective:** It is a modern Rust library ecosystem. You can import `termforge` crates into any Rust project to manage sessions programmatically (ORM-like) without spawning shell processes.
*   **System Perspective:** It is a resilient, async, event-driven system built on **Tokio**. State is managed through a deterministic pure kernel, CRDT-versioned where needed, ensuring stability even under heavy load or distributed/collaborative scenarios.
*   **Library Perspective:** The kernel is a new multiplexer—not a 1:1 tmux port. tmux is treated as an external **compatibility profile** (protocol + semantics + config grammar), not as the fundamental internal model.

---

## 2. North Star Acceptance Criteria

### 2.1 Compatibility Acceptance
1. Real tmux clients can attach to our server via the **imsg** protocol defined in `tmux-protocol.h`.
2. Our client can attach to real tmux servers and interact via **control mode** (`tmux -C`).
3. Protocol parsing/encoding is fixture-backed; no "invented" framing.
4. Command behavior is audit-checked against upstream tmux's command registry in `cmd.c`.
5. Any tmux-compat claim must cite upstream tmux source/headers or captured fixtures.

### 2.2 Architectural Acceptance
1. **Pure Core:** The kernel crate (`tf-core`) has no `tokio`, no IO, no `nix`/`libc`, and no `unsafe`.
2. **Stream-First:** UI/bindings update via typed streams; no steady-state polling; one persistent connection per socket.
3. **Unsafe Quarantine:** Only `tf-os` contains handwritten `unsafe`, with `// SAFETY:` documentation and focused tests.
4. **No Panics in Hot Paths:** Protocol decode/encode, server loops, and PTY loops use `Result` exclusively.
5. **One-Way Dependency Rules:** Everything points inward. Pure crates never depend on runtime/OS/bindings.

---

## 3. High-Level Architecture

### 3.1 The Layer Cake

1.  **Layer 0: Pure Kernel (`tf-core`, `tf-types`):** State graph, typed IDs, reducer, transaction engine.
2.  **Layer 1: Views & Query (`tf-view`, `tf-query`):** Immutable snapshots and Django-style traversal.
3.  **Layer 1b: Protocol (`tf-proto`, `tf-control`):** tmux binary protocol codec, control-mode parser.
4.  **Layer 2: Compatibility (`tf-compat`):** tmux semantics mapping (commands, options, formats).
5.  **Layer 3: Runtime (`tf-server`, `tf-client`, `tf-pty`, `tf-os`):** Tokio actors, sockets, PTY management.
6.  **Layer 4: Facade API (`tf-api`):** Managed per-socket actors, view store, refresh hints.
7.  **Layer 5: ORM API:** `libtmux`-shaped object graph layered on views + execution.
8.  **Layer 6: CRDT (`tf-crdt`):** Op-log + merge policies + replication.
9.  **Layer 7: Bindings:** Thin wrappers for Python (PyO3), Node (Neon), and C++.

### 3.2 Component Responsibility Table

| Crate | Responsibility | Purity | Key Source Ref |
| :--- | :--- | :--- | :--- |
| `tf-types` | Foundational types, typed IDs, geometry | **PURE** | `tmux.h` |
| `tf-core` | State graph (`ServerGraph`), Transaction engine | **PURE** | `session.c`, `window.c` |
| `tf-proto` | tmux binary protocol codec (imsg) | **PURE** | `tmux-protocol.h` |
| `tf-query` | Django-style filtering DSL (`QueryList`) | **PURE** | `libtmux` pattern |
| `tf-grid` | Terminal grid model, ANSI parser | **PURE** | `grid.c`, `input.c` |
| `tf-os` | Unsafe quarantine: PTYs, SCM_RIGHTS | **UNSAFE** | `osdep-*.c` |
| `tf-server` | Async actor system, connection manager | **IMPURE** | `server.c` |
| `tf-api` | Public facade, `ManagedMux`, `View` store | **FACADE** | New |

---

## 4. Kernel Design: Transaction-First State

The kernel owns the authoritative state graph using `slotmap` for O(1) lookup and generational safety.

```rust
// tf-core/src/graph.rs
pub struct ServerGraph {
    sessions: SlotMap<SessionId, Session>,
    windows: SlotMap<WindowId, Window>,
    panes: SlotMap<PaneId, Pane>,
    clients: SlotMap<ClientId, Client>,
    options: OptionsStore, // Global, session, window scopes
}
```

### 4.1 Transaction Engine
Every state mutation flows through the transaction system, enabling CRDT replication and undo/redo.

```rust
pub enum KernelOp {
    CreateSession { name: String },
    CreateWindow { session_id: SessionId, name: String },
    SplitPane { window_id: WindowId, direction: SplitDirection },
    SetOption { scope: OptionScope, key: String, value: OptionValue },
}

pub struct Transaction {
    pub id: TxnId,
    pub ops: Vec<KernelOp>,
    pub preconditions: Vec<Precondition>, // e.g., SessionExists(id)
}
```

---

## 5. API Layers & ORM Usage

### 5.1 Django-style Querying (`tf-query`)
Inspired by `libtmux`, providing expressive filtering over the multiplexer object graph.

```rust
// Rust Usage
let session = view.sessions()
    .filter(session::name().icontains("dev"))
    .get_one()?;

let active_pane = session.windows()
    .filter(window::name().startswith("editor"))
    .get_one()?
    .panes()
    .filter(pane::active().is_true())
    .get_one()?;
```

### 5.2 Managed Facade (`tf-api`)
The "Managed" mode handles the actor lifecycle and provides lock-free view reads via `ArcSwap`.

```rust
let mux = ManagedMux::tmux(Some("/tmp/tmux-1000/default"))?;
let view = mux.view()?; // O(1) lock-free snapshot
let mut hints = mux.subscribe();

while let Ok(hint) = hints.recv().await {
    // React to specific changes (e.g., PaneOutput, WindowRenamed)
}
```

---

## 6. Testing Strategy: `tf-test-framework`

The architecture enables a first-class test framework for terminal applications.

```rust
#[test]
fn test_vim_integration() {
    let session = TestSession::builder()
        .window("editor", |w| w.pane("bash", "vim file.rs"))
        .build()?;

    session.active_pane().send_keys("iHello World ESC :wq ENTER");
    
    session.wait_for(|view| view.active_pane().exited(), Duration::from_secs(5))?;
    assert!(fs::metadata("file.rs").is_ok());
}
```

---

## 7. AGENTS.md / CLAUDE.md Rules

### Rule 0: Never Kill Real tmux
*   **NEVER** run `kill-server` against the developer's real tmux socket.
*   Always use dedicated test sockets (`/tmp/tf-test-*`).
*   Verify the socket path before issuing destructive commands.

### Purity Boundary
*   `tf-core`, `tf-types`, `tf-proto`, `tf-query`, `tf-grid` are **PURE**.
*   No `tokio`, no `async`, no `nix`, no `unsafe`, no filesystem access.
*   Violation of purity is a CI failure.

### Protocol Integrity
*   Explicit field parsing only; **NEVER** cast bytes to structs.
*   Validate all lengths and cap allocations.
*   Protocol claims must be backed by `~/study/c/tmux` source.

---

## 8. Implementation Roadmap

### Phase 1: Foundations (Weeks 1-4)
*   Implement `tf-types` and `tf-query` (porting from `vibe-tmux`).
*   Define `tf-core` `ServerGraph` and `Transaction` engine.
*   Establish `AGENTS.md` and purity lints in CI.

### Phase 2: Protocol & Terminal (Weeks 5-8)
*   Implement `tf-proto` (imsg codec) matching `tmux-protocol.h`.
*   Implement `tf-grid` ANSI parser and grid storage.
*   Port protocol fixtures and snapshot tests.

### Phase 3: Runtime & Compatibility (Weeks 9-14)
*   Build `tf-os` (SCM_RIGHTS, PTYs) and `tf-server` (Actor system).
*   Implement 40+ core tmux commands in `tf-core/src/command/`.
*   Accept connections from real `tmux attach`.

### Phase 4: API & Bindings (Weeks 15-20)
*   Build `tf-api` facade and `ManagedMux`.
*   Implement Python (PyO3) and Node (Neon) bindings.
*   Deliver `tf-test-framework` as a standalone crate.

---

## 9. Risks and Mitigations

*   **Risk:** SCM_RIGHTS FD leaks. **Mitigation:** Strict `FdGuard` types in `tf-os` that close on drop; tests with PTY leak detection.
*   **Risk:** CRDT performance on large grids. **Mitigation:** Use Last-Write-Wins (LWW) for structural state; keep full operation logs optional.
*   **Risk:** tmux Version Drift. **Mitigation:** Maintain `mux-vm` (from `vibe-tmux`) to test against 3.3a, 3.4, and 3.5 binaries simultaneously.
