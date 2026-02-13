# TermForge

A Rust terminal multiplexer SDK with tmux v3.5a protocol compatibility.

## Architecture

TermForge uses a 6-layer crate hierarchy with a Sans-IO kernel at its core:

| Layer | Crates | Purpose |
|-------|--------|---------|
| L0 | mux-grapheme-arena, mux-time | Zero-dependency leaf crates |
| L1 | mux-types | Core type definitions |
| L2 | mux-grid, mux-parser, mux-options, mux-target, mux-format, mux-cmd-parse, mux-proto, mux-crdt, mux-fdpass | Data structures and parsers |
| L3 | mux-snapshot, mux-kernel, mux-render, mux-orm, mux-pty | Business logic |
| L4 | mux-termlet, mux-config, mux-api, mux-test-support | Integration |
| L5 | mux-ffi, mux-otel | External surface (FFI, tracing) |
| L6 | tmux-builder, tmux-vm, tmux-sniff, mux-doctor | User-facing tools |

## Quick Start

```bash
cargo check                    # Type-check everything
cargo test --workspace         # Run all tests
cargo clippy --workspace --all-targets -- -D warnings
```

## SDK Usage

```rust
use mux_api::{TermForge, PaneConfig};

let mut server = TermForge::builder()
    .socket_path("/tmp/my-app.sock")
    .build()?;

let session_id = server
    .session("dev")
    .window("editor")
    .pane(PaneConfig::new().command("vim"))
    .window("terminal")
    .pane(PaneConfig::default())
    .build()?;
```

## Termlets (Testing Pods)

```rust
use mux_termlet::Termlet;
use mux_types::Size;

let mut t = Termlet::new(Size::new(80, 24));
t.spawn("/bin/sh")?;
t.send_keys("echo hello\n");
let output = t.capture();
assert!(output[0].contains("hello"));
t.shutdown()?;
```

## License

MIT OR Apache-2.0
