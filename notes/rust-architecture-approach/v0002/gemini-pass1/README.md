# TermForge

TermForge is a modular SDK for building terminal multiplexers in Rust. It provides the building blocks—PTY, Layout, Grid, VT Emulation—as separate crates, allowing you to compose custom terminal applications.

## Status: v0002 (Architecture Pass 1)

This version refines the architecture with a 3-thread model and modular crates.

## Architecture

```
                    +-------------------+
                    |     mux-api       |  <-- Public SDK / Builder
                    +-------------------+
                             |
          +------------------+------------------+
          |                  |                  |
   +-------------+    +-------------+    +-------------+
   |   mux-term  |    |  mux-kernel |    |  mux-render |
   | (IO Thread) |    | (Logic T.)  |    | (Render T.) |
   +-------------+    +-------------+    +-------------+
   | - PTY Master|    | - Session   |    | - Compositor|
   | - Signals   |    | - Layouts   |    | - Diffing   |
   | - Sockets   |    | - VT Parse  |    | - ANSI Gen  |
   +-------------+    +-------------+    +-------------+
          |                  |                  |
          +------------------+------------------+
                             |
                    +-------------------+
                    |   mux-protocol    |  <-- Shared Types
                    +-------------------+
```

## Getting Started

```rust
use mux_api::TermForge;

fn main() {
    let term = TermForge::builder()
        .with_shell("/bin/zsh")
        .build();
    
    // Application logic...
}
```

## Crates

*   `mux-api`: High-level interface.
*   `mux-kernel`: Core logic (Sans-IO).
*   `mux-render`: Output generation.
*   `mux-term`: OS interaction.
*   `mux-protocol`: Wire format & shared types.
