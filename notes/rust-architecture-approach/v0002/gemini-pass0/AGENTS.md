# TermForge Agent Guidelines

## Role: Architect
- **Responsibility:** Define crate boundaries, public APIs, and data flows.
- **Checklist:**
  - Does this change violate the 3-thread model?
  - Is the memory cost acceptable (COW implications)?
  - Does this break tmux protocol compatibility?

## Role: Implementer
- **Responsibility:** Write correct, idiomatic Rust.
- **Checklist:**
  - `cargo check` passes.
  - No `unwrap()` in `src/`.
  - `tracing::instrument` added to public methods.
  - Error types defined in `error.rs` or inline with `thiserror`.

## Role: Reviewer
- **Responsibility:** Enforce quality and patterns.
- **Checklist:**
  - Are tests present? (Unit + Snapshot)
  - Is `unsafe` used? (If yes, strict scrutiny required).
  - Are dependencies minimal?
  - Does the code follow `CLAUDE.md`?

## Role: Tester
- **Responsibility:** Break the system.
- **Strategy:**
  - Fuzz the wire protocol.
  - Spawn 1000 Termlets.
  - Force disconnect clients.
  - Verify socket cleanup on panic.

## Workflows
1. **New Feature:**
   - Define types in `mux-types` or `mux-api`.
   - Implement logic in `mux-kernel`.
   - Add protocol frames in `mux-proto`.
   - Add Termlet test case.

2. **Bug Fix:**
   - Reproduce with a Termlet test case (failing test).
   - Fix in `mux-kernel` or relevant crate.
   - Verify with `cargo test`.
--- END FILE ---
