# AGENTS.md

## Agent Roles
- Architect: validates crate boundaries, invariants, and dependency direction.
- Implementer: writes minimal coherent code plus tests for touched invariants.
- Reviewer: focuses on regressions, API drift, and tmux compatibility impact.
- Tester: expands integration coverage, fixture isolation, and snapshot reliability.

## Workflow
1. Read `notes/architecture.md` section relevant to the change.
2. Confirm invariants in touched crate(s) before coding.
3. Implement smallest vertical slice with tests.
4. Run quality gates.
5. Document tradeoffs in PR notes.

## Quality Gates
- `cargo check --workspace`
- `cargo test -p mux-grid -p mux-proto -p mux-kernel`
- `cargo clippy --workspace --all-targets`
- Snapshot diffs reviewed for intentionality.

## Review Checklist
- Are threading boundaries preserved (kernel thread vs IO/render tokio tasks)?
- Are channel capacities configurable and bounded?
- Does code preserve TF01 header/size semantics?
- Are socket paths isolated and cleaned via RAII guards?
- Are public APIs mockable for tests and bindings?

## Actionable Rules
- Prefer adding traits at boundaries (`PtyBackend`, `Clock`, `OrmBackend`) before concrete impls.
- Keep protocol encoding/decoding independent from business logic.
- Add one invariant test when changing any core data structure.
- If a change affects tmux compatibility, include both TF01 and tmux-mode tests.
