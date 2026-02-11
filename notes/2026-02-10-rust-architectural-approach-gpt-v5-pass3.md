Created the definitive north-star architecture and repo-ready guardrails as real files in `/home/d/study/c/tmux`:

- `ARCHITECTURE.md` (3006 lines): Pass 3 final synthesis incorporating all settled Pass 2 decisions, plus explicit subsections for:
  - VT100/DEC ANSI parser design (tmux `input.c`-aligned 17-state table, CSI dispatch, tmux extensions)
  - Format string engine (`crates/mux-core/src/format.rs`, AST + pure eval)
  - Key binding system (`crates/mux-core/src/key_table.rs`, root/prefix/copy-mode/copy-mode-vi tables)
  - Copy mode (`crates/mux-core/src/copy_mode.rs`, per-pane `CopyModeState`, search/selection/yank)
  - Fake PTY Scenario Recorder/Replayer (JSON schema + recorder/replayer contracts)
  - WASM purity gate commands and enforcement
  - Vec-in-entity adjacency policy + reverse indices computed from snapshots
  - `mux-types` split, `mux-orm` split, and “no im-rs initially; `Arc<Grid>` COW” policy

- `AGENTS.md` (119 lines): complete, directly-usable repo guardrails reflecting the same decisions (purity boundary + wasm gate, unsafe quarantine, protocol rules, key/copy/format rules, scenario recorder/replayer, test isolation).

Paths:
- `/home/d/study/c/tmux/ARCHITECTURE.md`
- `/home/d/study/c/tmux/AGENTS.md`
