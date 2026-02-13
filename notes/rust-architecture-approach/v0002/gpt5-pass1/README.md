# TermForge v0002 Pass 1 Scaffold (GPT-5)

Rust terminal multiplexer SDK scaffold targeting tmux wire protocol v8 compatibility.

## Build

```bash
cargo check --workspace
cargo test --workspace
```

## Architecture Map

```text
                                +------------------+
                                |     mux-api      |
                                | SDK entry point  |
                                +---------+--------+
                                          |
                     +--------------------+--------------------+
                     |                                         |
              +------+-------+                          +------+-------+
              |  mux-kernel  |                          |  mux-render  |
              | Sans-IO core |                          | compose+diff |
              +------+-------+                          +------+-------+
                     |                                         |
         +-----------+-------------+                     +-----+------+
         |                         |                     |            |
   +-----+------+           +------+-----+         +-----+----+  +---+--------+
   |  mux-grid  |           |  mux-proto |         | mux-types |  | mux-time   |
   +------------+           +------------+         +----------+  +------------+

L0 support crates: mux-parser, mux-grapheme-arena, mux-orm, mux-test-support
L1 support crates: mux-pty, mux-snapshot
L2 support crate : mux-termlet
```

## Crates

- `mux-api`: Builder API, terminal mode lifecycle policy, binding boundary stubs.
- `mux-kernel`: Sans-IO reducer, layout engine, copy mode state machine.
- `mux-render`: Pane composition, dirty-line diff, output coalescing.
- `mux-types`: core cells/line/style/error identity types.
- `mux-grid`: terminal grid and scrollback.
- `mux-parser`: VT parser.
- `mux-proto`: protocol framing.
- `mux-pty`: PTY lifecycle model.
- `mux-time`: deterministic clocks.
- `mux-snapshot`: snapshot encoding.
- `mux-orm`: query surface.
- `mux-test-support`: harness utilities.
- `mux-termlet`: termlet runtime scaffold.

