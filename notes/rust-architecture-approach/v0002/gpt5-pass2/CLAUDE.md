# CLAUDE.md - TermForge Pass 2 Canonical Guidance

## Build and Test Commands

- `cargo check`
- `cargo test --workspace`
- `cargo test -p mux-kernel`
- `cargo test -p mux-render`
- `cargo test -p mux-api`
- `cargo test -p mux-config`
- `cargo test -p mux-parser`
- `cargo fmt --all`
- `cargo clippy --workspace --all-targets -- -D warnings`

## Coding Standards

- Edition: Rust 2024 (mandatory)
- MSRV: 1.85 (mandatory)
- Safety: unsafe quarantined to `mux-pty` only
- Panic policy: avoid panics in steady-state paths
- Error policy: return typed errors; keep messages actionable
- API policy: prefer deterministic behavior and bounded resources

## Comment Guidance

Explain why, not what.
Do not narrate obvious syntax.
Comment invariants, protocol assumptions, and failure boundaries.

## Thread Architecture Diagram

```text
+-------------------------+      +--------------------------+
| accept loop             | ---> | control lane (bound 512) |
+-------------------------+      +--------------------------+
            |                                  |
            v                                  v
+-------------------------+      +--------------------------+
| signal bridge loop      | ---> | effect lane (bound 1024) |
+-------------------------+      +--------------------------+
            |                                  |
            v                                  v
+-------------------------+      +--------------------------+
| PTY I/O loop            | ---> | data lane (1024 x 64KiB) |
+-------------------------+      +--------------------------+
                                               |
                                               v
                                   +--------------------------+
                                   | render lane (bound 256)  |
                                   +--------------------------+
```

## Channel Bounds

- control: 512
- data: 1024 frames
- data frame size: 64 KiB
- render: 256
- effect: 1024

## Crate Dependency Layers

L0 core:
- mux-types
- mux-parser
- mux-grapheme-arena
- mux-time
- mux-orm
- mux-test-support
- mux-crdt
- mux-fdpass
- mux-effects

L1 engine:
- mux-grid
- mux-proto
- mux-snapshot
- mux-pty
- mux-input
- mux-render
- mux-kernel
- mux-config
- mux-term

L2 orchestration:
- mux-api
- mux-client
- mux-server
- mux-termlet

Tools:
- tfctl
- tfrecord
- tfbench
- tfdecode

## Commit Conventions

- Prefix: `crate(scope): summary`
- Examples:
  - `mux-render(diff): tighten fairness quota`
  - `mux-kernel(layout): fix redistribution remainder`
  - `mux-api(policy): keep graphics passthrough off by default`
- Each commit must mention invariant impact when relevant.

## Test Patterns

- Unit tests per crate are mandatory.
- Deterministic tests only; avoid wall-clock sleeps.
- Golden-style parser/render tests should encode exact outputs.
- For bug fixes: add regression test first when feasible.

## Invariants (INV-117..INV-231)
- INV-117: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-118: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-119: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-120: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-121: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-122: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-123: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-124: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-125: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-126: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-127: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-128: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-129: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-130: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-131: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-132: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-133: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-134: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-135: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-136: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-137: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-138: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-139: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-140: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-141: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-142: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-143: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-144: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-145: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-146: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-147: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-148: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-149: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-150: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-151: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-152: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-153: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-154: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-155: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-156: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-157: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-158: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-159: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-160: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-161: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-162: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-163: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-164: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-165: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-166: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-167: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-168: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-169: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-170: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-171: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-172: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-173: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-174: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-175: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-176: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-177: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-178: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-179: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-180: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-181: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-182: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-183: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-184: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-185: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-186: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-187: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-188: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-189: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-190: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-191: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-192: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-193: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-194: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-195: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-196: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-197: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-198: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-199: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-200: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-201: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-202: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-203: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-204: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-205: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-206: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-207: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-208: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-209: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-210: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-211: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-212: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-213: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-214: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-215: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-216: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-217: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-218: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-219: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-220: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-221: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-222: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-223: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-224: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-225: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-226: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-227: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-228: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-229: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-230: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md
- INV-231: canonical pass2 invariant recorded in mux-kernel::invariants and architecture.md

## Layout Engine Invariants

- exactly seven algorithms are supported
- resize redistribution preserves totals
- layout checksum is deterministic
- custom layout parsing is side-effect free
- mirrored variants preserve pane identity order constraints

## What NOT to Do

- Do not adopt crossterm as the runtime core.
- Do not replace the parser core with vt100/termwiz as authoritative behavior.
- Do not use SmallVec as a hidden optimization dependency in line/cell structures.
- Do not perform heavy work inside signal handlers.
- Do not introduce unbounded channels.
- Do not enable graphics passthrough by default.
- Do not bypass RawModeGuard policy rules.
- Do not add unsafe outside `mux-pty`.
- Do not make shutdown ordering best-effort; preserve crash-frame order.
- Do not add cross-layer dependency cycles.

## Required Decisions Snapshot

- Composition: CompositeGrid double-buffer diff with four optimizations and fairness quota.
- Layout: tree model + seven algorithms + checksum + redistribution.
- Raw mode: RAII guard and explicit policy enum.
- SIGWINCH: protocol-level propagation sequence.
- SIGCHLD: non-interfering reap signaling.
- crossterm: rejected.
- vt100/termwiz as core: rejected.
- Copy mode: vi/emacs and four selection types.
- Graphics passthrough: deny by default.
- signal-hook: hybrid with channel propagation.
- CRDT: vector dominance and LWW tiebreak.
- SCM_RIGHTS: fd envelope architecture.
- Config: TOML + hot-reload plan.
- Panic recovery: crash frame then ordered shutdown.
- Channels: fixed bounded capacities.
- Termlets: SDK-first testing pods.
