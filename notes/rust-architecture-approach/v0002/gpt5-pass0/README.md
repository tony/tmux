# TermForge v0002 (Pass 0 Scaffold)

TermForge is an SDK-first Rust terminal multiplexer with tmux wire-protocol compatibility (v8) and a native TF01 framing mode.

## Vision
- Embed multiplexer primitives into Rust apps; daemon mode is optional.
- Expose libtmux-style traversal: `Server -> Session -> Window -> Pane` with `filter/get/where` query semantics.
- Provide Termlets as lightweight PTY + VT testing pods across Rust, Python, and Node.
- Support optional CRDT collaboration and OpenTelemetry end-to-end tracing.

## Architecture
```text
+--------------------------- Host App ----------------------------+
|  mux-orm  |  mux-termlet  |  mux-ffi (PyO3/napi-rs adapters)   |
+------------------------+------------------+---------------------+
                         |                  |
                         v                  v
+--------------------- mux-kernel (std::thread) ------------------+
| entity graph (slotmap) | layout | command router | revisions    |
+------------------+-------------------+--------------------------+
                   |                   |
                   v                   v
          IO lane (tokio)         Render lane (tokio)
          mux-proto + mux-pty      mux-grid + mux-render + snapshot
```

## Crate Graph
```text
mux-types
  -> mux-grid -> mux-snapshot -> mux-render
  -> mux-target
  -> mux-options
  -> mux-pty -> mux-termlet -> mux-test-support
mux-parser -> mux-cmd-parse -> mux-kernel
mux-format -----------------> mux-kernel
mux-proto ------------------> mux-kernel
mux-time -------------------> mux-kernel
mux-otel -------------------> mux-kernel
mux-kernel -----------------> mux-orm -> mux-ffi
```

## Quickstart
```bash
cd notes/rust-architecture-approach/v0002/gpt5-pass0
cargo check
```

## Comparison
- tmux: mature CLI multiplexer; C daemon-first architecture.
- Zellij: modern Rust multiplexer with plugin-first UX.
- WezTerm: terminal emulator first; multiplexer is integrated but emulator-led.
- TermForge: library-first multiplexer SDK with explicit tmux wire compatibility and test pod APIs.
