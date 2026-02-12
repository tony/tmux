# AGENTS.md -- Agent Instructions

## For all agents working on this codebase:

1. Read CLAUDE.md first for project conventions.
2. Every public function MUST have `#[must_use]` if it returns a Result or a pure value.
3. Every error type MUST implement `std::error::Error + Send + Sync + 'static`.
4. Run `cargo test --workspace` before committing.
5. Reference INV-xxx and S-xxx in comments for traceability.
6. The mux-kernel crate is the Sans-IO reducer -- no std::io allowed.
7. Feature gates: `#[cfg(feature = "crdt")]` for CRDT-specific code.
