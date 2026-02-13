# TermForge Developer Guidelines

## Code Style & Comments
*   **Explain Why, Not What:** Comments should explain architectural decisions, tricky logic, or invariants. Do not narrate the code (e.g., avoid `// Increment i`).
*   **Doc Comments:** Use `///` for public APIs in `mux-api` and `mux-protocol`. Internal crates can use `//`.
*   **Panic Policy:** `unwrap()` and `expect()` are FORBIDDEN in `mux-kernel` and `mux-term` (except during initialization). Use `Result` propagation.
*   **Imports:** Group imports by crate (std, external, internal).

## Invariants
1.  **3-Thread Model:** Never block the IO thread with heavy computation. Never block the Kernel thread with blocking IO.
2.  **Raw Mode:** If the SDK enables raw mode, it MUST guarantee restoration on drop/panic.
3.  **Grid Immutability:** Rendering code sees an `Arc<Grid>` or `Cow<Line>`. It must NEVER modify the grid state directly.
4.  **Layouts:** The Layout Tree must always fully cover the available terminal area (no gaps).

## Testing
*   Unit tests go in the same file or `tests/` module.
*   Integration tests go in `tests/` directory at crate root.
*   Run `cargo test --workspace` before pushing.
