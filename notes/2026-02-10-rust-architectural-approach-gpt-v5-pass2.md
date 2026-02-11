`notes/2026-02-11-rust-architectural-approach-pass2-refined.md` now contains the full Pass 2 refined architecture with all 23 required sections and concrete resolutions for all 10 open questions.

Key fixes vs Pass 1 (implemented in the doc):
- Explicitly grounded what actually exists in this repo today (tmux C files like `format.c`, `window-copy.c`, `key-bindings.c`, `grid.c`, `screen.c`) and called out that the Rust crates are proposed, not present.
- Concrete decisions for Open Questions 1-10, including:
  - Hybrid adjacency storage using `SecondaryMap` + `SmallVec` in `mux-core` (not Vec-in-entity).
  - Practical WASM compile gates for select pure crates as a purity tripwire.
  - `mux-types` split kept minimal and stable.
  - ORM moved to `mux-orm` (separate from `mux-api`), with `mux-query` pure over snapshots.
  - Fake PTY scenario recorder format (`Scenario` + `PtyOp`) and deterministic playback rules.
  - Explicit “no persistent collections in core initially” decision and where to introduce them later if needed.
  - VT/ANSI parsing strategy: `vte` parser + custom pure `mux-grid` performer, plus fuzz + vttest-like conformance.
  - Format engine: pure AST + bytecode + `VarSource` evaluator + parity harness against tmux.
  - Key binding system: `mux-keys` resolver + per-client `KeyState` in core, prefix timeouts via timer effects/events.
  - Copy mode: pure per-pane state machine (`CopyState`) with deterministic selection/search and test plan referencing tmux behavior in `window-copy.c`.
- Strengthened test strategy: unit/property/snapshot/fuzz/parity layers, plus explicit parity approach for format and copy-mode behavior.

If you want, I can also add a top-level `ARCHITECTURE.md` that points to this Pass 2 document and summarizes the layering contract in a shorter “engineer quickstart” form.
