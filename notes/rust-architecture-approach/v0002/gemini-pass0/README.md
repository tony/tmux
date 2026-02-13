# TermForge

**TermForge** is a next-generation terminal multiplexer SDK written in Rust.

It is designed to be **wire-compatible with tmux (protocol v8)** while offering a modern, type-safe, and embeddable architecture.

## Vision

1.  **SDK-First**: Build terminal applications by embedding TermForge.
2.  **ORM-like API**: Traverse `Server.sessions.filter(name="foo").windows` easily.
3.  **Termlets**: Lightweight, scriptable terminal pods for testing and automation.

## Architecture Overview

```mermaid
graph TD
    Client[Client (TF01/tmux)] -->|Socket| IO[IO Thread (Tokio)]
    IO -->|Channels| Kernel[Kernel Thread (std)]
    Kernel -->|Update| Grid[Grid (COW)]
    Kernel -->|Render Op| Render[Render Thread (Tokio)]
    Render -->|VT Seq| Client
    
    subgraph "TermForge SDK"
        IO
        Kernel
        Grid
        Render
    end
```

## Crate Structure (Partial)

- **mux-kernel**: The brain. Single-threaded, owns the state.
- **mux-grid**: Efficient, chunked-COW terminal grid.
- **mux-proto**: TF01 and tmux imsg protocol definitions.
- **mux-termlet**: Testing pods for terminal apps.
- **mux-orm**: High-level API for traversal.

## Quickstart

```rust
// Using the ORM API
use mux_orm::Server;

fn main() -> anyhow::Result<()> {
    let server = Server::new()?;
    let session = server.new_session("my-session")?;
    let window = session.active_window()?;
    let pane = window.active_pane()?;
    
    pane.send_keys("echo 'Hello TermForge'\n")?;
    
    Ok(())
}
```

## Comparison

| Feature | TermForge | tmux | Zellij |
| :--- | :--- | :--- | :--- |
| **Language** | Rust | C | Rust |
| **Protocol** | TF01 + tmux v8 | tmux | Custom |
| **Architecture** | Embeddable SDK | Daemon | Daemon |
| **Scripting** | Python/Node bindings | Shell/Tcl | WASM |

## License
MIT
--- END FILE ---
