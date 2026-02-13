# TermForge Implementation Plan

## Phase 1: The Core (Weeks 1-4)
**Goal**: A headless server that can spawn a PTY and update an internal Grid.
-   [ ] **mux-types**: Define Cell, Grid, Style.
-   [ ] **mux-grid**: Implement Chunked COW grid.
-   [ ] **mux-pty**: Portable PTY spawning (fork/exec).
-   [ ] **mux-kernel**: Basic event loop.
-   [ ] **Test**: Verify `grid.write("hello")` updates memory.

## Phase 2: The Protocol (Weeks 5-8)
**Goal**: `tmux attach` works (read-only).
-   [ ] **mux-proto**: Implement tmux imsg v8 parsing.
-   [ ] **mux-server**: Unix domain socket listener.
-   [ ] **mux-render**: Basic grid-to-vt sequence generation.
-   [ ] **Test**: Connect with standard tmux client and see "Hello".

## Phase 3: Interaction (Weeks 9-12)
**Goal**: Full interactive session.
-   [ ] **mux-input**: Key binding mapping.
-   [ ] **mux-cmd-parse**: Parse `new-window`, `split-window`.
-   [ ] **mux-orm**: Implement the public SDK API.
-   [ ] **Test**: Termlet integration tests.

## Phase 4: Polish & Parity (Weeks 13+)
**Goal**: Replace daily driver.
-   [ ] **mux-options**: Full `set -g` support.
-   [ ] **mux-format**: `status-left "#{session_name}"`.
-   [ ] **Bindings**: Python/Node.js packages.

## Risk Register
1.  **tmux protocol quirkiness**: undocumented behavior in imsg.
    *Mitigation*: Use `tmux-sniff` tool to capture real traffic.
2.  **Performance**: COW Grid too slow on large resizes?
    *Mitigation*: Benchmark early. Optimize `LineChunk` size.
3.  **Pty portability**: BSD/macOS vs Linux differences.
    *Mitigation*: Isolate in `mux-pty`. Use `nix` crate abstractly.
--- END FILE ---
