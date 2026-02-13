# Development Plan (v0002)

## Phase 1: Core Infrastructure (Pass 1 - Done)
- [x] Defines workspace crates.
- [x] Basic crate structure (`mux-api`, `mux-kernel`, `mux-render`, `mux-term`, `mux-protocol`).
- [x] Architecture Deep Dive (Composition, Layouts, Signals, etc.).

## Phase 2: IO & PTY (Next)
- [ ] `mux-term`: Implement `IoThread` with real `nix` calls.
- [ ] `mux-term`: Implement `Pty` wrapper (`openpty`, `forkpty`).
- [ ] `mux-protocol`: Define full set of `KernelEvent` and `RenderInstruction`.
- [ ] `mux-term`: Setup `signal-hook` integration.

## Phase 3: Kernel Logic
- [ ] `mux-kernel`: Implement `Session` manager (add/remove windows/panes).
- [ ] `mux-kernel`: Implement full `Layout` engine (recursive resize).
- [ ] `mux-kernel`: Integrate `vte` parser to feed `Grid`.
- [ ] `mux-kernel`: Implement `CopyMode` state machine logic (movement, selection).

## Phase 4: Rendering Pipeline
- [ ] `mux-render`: Implement `Surface` blitting (copy rects).
- [ ] `mux-render`: Implement smart `diff` algorithm (skip clean lines).
- [ ] `mux-render`: Implement `ANSI` generation (SGR optimizations).

## Phase 5: Public API & Polish
- [ ] `mux-api`: Flesh out `Builder` options.
- [ ] `mux-api`: Add `Client` concept for multi-client support.
- [ ] `mux-api`: Add error handling and panic hooks.
- [ ] CI/CD Setup (GitHub Actions).

## Estimated Size
*   **Total LOC:** ~35k
*   **Timeline:** 3-6 months for MVP.
