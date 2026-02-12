# CLAUDE.md

## Coding Standards

- Rust edition: 2021.
- Prefer explicit types and small focused modules.
- No panics in library runtime paths; return typed `Result` errors.
- Keep public APIs deterministic with injectable time/seed dependencies.
- Avoid `unsafe` outside explicitly justified low-level platform code.

## Commit Conventions

- Use conventional prefixes: `feat:`, `fix:`, `refactor:`, `test:`, `docs:`.
- Keep commits crate-scoped where possible.
- Include spec anchor IDs (`Sxx`, `INV-xxx`, `RULE-Sxx-yyy`) in commit body for normative changes.

## Test Patterns

- Minimum 3 unit tests per crate module touched.
- Prefer deterministic fixtures and fixed clocks.
- For parser/table logic: always include oracle parity tests.
- For serialization: include round-trip + corruption-path tests.
- For typestate: include valid and invalid transition coverage.

## What Not To Do

- Do not introduce nondeterministic wall-clock reads in logic paths.
- Do not bypass typed errors with generic string errors.
- Do not change PackedCell bit layout without migration + compatibility tests.
- Do not change snapshot header contracts without updating rule references and tests.
- Do not add crate dependencies without explicit architecture review.
