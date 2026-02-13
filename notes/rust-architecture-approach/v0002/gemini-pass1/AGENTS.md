# Agent Roles & Capabilities

## Sub-Agents
*   **Protocol Agent:** Owner of `mux-protocol`. Responsible for binary compatibility with `tmux`.
*   **Kernel Agent:** Owner of `mux-kernel`. Responsible for Layouts (BSP), Session state, and Copy Mode state machine.
*   **Render Agent:** Owner of `mux-render`. Responsible for Diffing algorithms, ANSI generation, and Sixel support.

## Feature Policies

### Copy Mode
*   **Behavior:** Modal overlay.
*   **Keys:** Vi-keys (h,j,k,l) and Emacs-keys (Ctrl-P, Ctrl-N).
*   **Selection:** Visual character/line/block selection.
*   **Search:** Reverse regex search in scrollback.

### Graphics Protocol
*   **Phase 1:** Passthrough (`\033Ptmux;...`).
*   **Phase 2:** Sixel parsing to virtual framebuffer.
*   **Phase 3:** Kitty graphics protocol.
*   **Rule:** If a protocol is not supported, silently ignore the escape sequence (do not print garbage).
