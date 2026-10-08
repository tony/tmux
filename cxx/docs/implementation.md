# Independent tmux implementation plan

> Use the executing-plans workflow inline. Keep progress in `progress.md`.

**Goal:** Implement and verify every requirement in `../GOAL.md`.

**Architecture:** One C++ daemon owns persistent state and PTYs. Stock tmux
traffic and native `ServerConnection` requests invoke entity-owned operations
through frozen catalogs. Nanobind and Node-API adapt language types and
scheduling only.

**Spec:** `../GOAL.md` and `design.md`.

## Global constraints

- All repository mutations and artifacts stay in `cxx/`.
- Native first-party code is C++23 or newer; extensions are disabled.
- CMake owns the build; both bindings are required by default.
- Ordinary builds do not resolve latest dependencies or rewrite locks.
- Never contact, replace, or terminate the user's tmux server.
- Each behavior change has a focused failing-then-passing test.
- Full completion requires interactive use and every configured gate.

## Review focus

- A remote endpoint disconnects halfway through a frame: release owned resources.
- A PTY child exits while output is pending: retain bytes and reap the child.
- A handle outlives deletion or daemon restart: return a defined error.
- A consumer stops reading events: bound memory and signal a gap.
- A rejected optional command includes earlier effects: reject before effects.

## Milestones

- [ ] 1. Resolve tools, refresh host Ubuntu candidates, lock dependencies,
  and implement project-local bootstrap and environment reports. Produce a
  reproducible current-Ubuntu fixture; record any unavailable provisioning.
- [x] 2. Build the first functional slice: C++ PTY/entity operations/native
  protocol, `tmux-cxx`, nanobind, Node-API, CTest, pytest, Vitest, generated
  declarations, and strict consumers. A single shared-server integration
  test exercises all three native clients and unmodified tmux 3.4.
- [ ] 3. Translate stock identification, descriptor passing, attachment,
  resize, detach, exit, and control framing. Test concurrent real clients,
  malformed IPC, and non-destructive rejection with an upstream fixture.
- [ ] 4. Implement screen/grid/input behavior, persistent windows and links,
  pane splits, selection, resizing, layouts, restoration, scrollback,
  copy/paste, copy mode, multiple clients, essential commands, configuration,
  keys, and formats. Add differential fixtures per behavior.
- [ ] 5. Complete snapshots, capabilities, subscriptions, separate byte
  streams, backpressure, cancellation, reconnection, stale handles, hooks,
  lifetime tests, and asynchronous Python/Node scheduling.
- [ ] 6. Complete deterministic generation, compile-validated mappings,
  nanobind stubgen, exports, distributed typing, clangd navigation, and
  controlled-negative checks for missing exports and incorrect signatures.
- [ ] 7. Complete release/GCC/libc++/ASan/UBSan presets, real binding
  instrumentation, clang-format, clang-tidy, Doxygen HTML/XML, protocol
  fuzzing and regression corpus. Enforce timed inner/mid/outer loops.
- [ ] 8. Verify standalone configure/build/install, exported native target,
  fresh wheel/sdist/npm consumers, Ubuntu/stable Python, Node Current/LTS
  with one addon, daemon discovery, no-op build and generation stability.
- [ ] 9. Audit every objective requirement against current authoritative
  evidence in a fresh source copy and current Ubuntu. Keep incomplete or
  unverified requirements open; review and fix findings before completion.

## First functional slice

Files follow tmux ownership. Public declarations live in `include/tmux_cxx/{server,session,window,window_link,pane,client,connection,protocol}/`. Entity implementations and operations live in matching `src/` directories. `Server::State` owns the graph; `ServerConnection` controls it through `protocol/native/`. Stock messages use `protocol/tmux/`; registered tmux commands use `commands/` and entity-local command files. Entity-local CLI action files adapt process arguments to the same connection operations through `cli/`. `src/server/daemon.cpp` owns listeners and readiness routing. `src/main.cpp` owns process entry, daemon-startup parsing, and catalog invocation without operation-specific branches.

Language adapters live in `bindings/{python,node}`; generators live in `support/codegen/{python,node}`. `api/exposure.toml` is compile-validated against operation and value declarations. Native CMake source lists are explicit.

Test first: create a session via the C++ client, rename via Python, observe
and rename via Node, inspect with stock tmux, send a token into `/bin/sh`,
observe its PTY output through ServerConnection, reject a stale identity,
close both adapters, and prove the same server/session remains. Each suite
also runs directly. Failure before production code exists establishes the
first gate. Type failures use incorrect arguments/return assignments and a
deliberately absent export, then restore the contract.

The native interface returns `std::expected<T, TmuxError>` and materialized
snapshot values. Python translates errors into a TmuxError exception. Node
rejects worker promises with a stable error code. Identity data is synchronous
and operations involving IPC use workers. Closing a client is idempotent and
never sends a server shutdown request.

## Execution record

Run focused native/binding checks after their changes, the configured mid
loop before handoff, and full outer gates before claiming final completion.
Record whole-command elapsed time, failures, artifact locations, and tool
versions under `_build/reports/`; summarize durable milestones in
`progress.md`. Stage named files only and inspect both Git diffs each commit.

## Architecture progress

Named server/session/window/pane components, entity-local operation descriptions, and validated immutable native lookup now replace the generic domain module and operation switch. The public facade is `ServerConnection`, snapshots are values, and recoverable failures are `TmuxError`. Session/window membership and pane-program cleanup have real ownership models.

The first shared-server milestone passes with C++, Python, Node, and stock tmux 3.4. Native registration, malformed-before-effects, duplicate/closed registration, immutable catalog lookup, linked-window lifetime, binary bytes, stale IDs, restart identity, and direct-child cleanup have focused cases.

Documentation has compiler coverage, strict Doxygen HTML/XML, language AST checks, budget/filler rules, and controlled negative probes. Ruff/ty and Oxc/tsgolint/TypeScript remain independent gates.

Native negotiation now declares the cataloged operations, effects, capabilities, and payload limit. Entity operations and native adapters have separate files. Stock profile detection distinguishes tmux 3.4 and 3.7 headers using descriptor evidence; real clients pass the declared one-shot and guarded control-command subsets. Accepted-client identification, one-shot admission/output acknowledgement, terminal attachment/detachment, active control mode, control-output draining, and transport closure use exclusive variant phases with phase-owned descriptors, policy, output, and deadlines.

The terminal foundation retains chunked VT parsing, screens, cursor/rendition, bounded history, and ordered query replies. Real PTY tests exercise query responses and nonblocking input remainders. Owner-qualified pane text, pane splitting and selection, recursive window layouts, the `latest` window-size policy, and explicit named paste buffers are available through the shared C++ server and all three native clients. Ordinary-client redraw composes every pane with basic ASCII borders and the selected cursor. Automatic paste buffers, buffer listing, richer paste modes, copy mode, capability-sensitive borders, other size policies, and broader terminal fidelity keep milestone 4 open.

Startup now validates entity registrations against the public exposure catalog before opening sockets. A controlled omission stops the actual daemon before readiness; restored registration passes the shared-server suites. The installed native archive exports the generated catalog without linking language runtimes.

Bounded native, stock-codec, and terminal fuzz targets now compile against production components. Retained terminal regressions cover UTF-8 decoder ownership, designated-character-set mapping, right-edge combining, and CSI bounds. Upstream library corrections and the local GL-mapping correction are generated from hash-checked inputs with explicit attribution. Full protocol-role/lifecycle fuzzing and terminal differential coverage remain open.

The reverse stock role now has explicit profile selection, connected-server release evidence, a separate `StockCommandExchange`, and a typed session-name adapter. The accepted role has a `protocol/tmux/control/` hierarchy for the bounded descriptor channel, command-line subset, result guards, server-wide numbering, session notification delivery, and selected-session raw output. Session create, rename, and close observation uses `SessionEvent*` values, storage, codecs, and entity-local read/wait operations under `session/`. `SessionEventStream` adds an atomic baseline, one owned cursor, cancellation of admitted native waits, and explicit replacement-snapshot recovery after a journal gap. Python exposes synchronous context-managed reads plus a typed asyncio adapter that closes the observer on task cancellation. Node exposes promise-backed reads, pull-driven async iteration, `AbortSignal`, and asynchronous disposal. Callback delivery, reconnection, resumable cursors, and bounded push queues remain open. Exact tmux 3.4 and 3.7 probes establish that upstream closes unattached control clients after startup; the current user-facing extension is exposed as `unattached_control_channel_stays_open`. Identification admits only the tested single-`-C` variant and rejects `-CC`, `no-output`, `pause-after`, and `wait-exit` before command effects. The next work retains the full objective: broader control notifications, pause/continue and format subscriptions; broader stock operations and older profiles; terminal/client models and usable multiplexing; remaining async lifetimes; installed language products; compiler/platform gates; and the final line-by-line audit. Passing these slices does not complete those requirements.

## Vocabulary correction before subscriptions

Keep `AcceptedClientProtocol`: it owns inbound tmux wire phases and does not
claim socket or tmux-client ownership. Keep `AcceptedConnection` private to the
daemon while that event loop is its only user. Keep the outbound asymmetry:
`StockCommandExchange` owns one wire exchange, while
`StockServerConnection` retains endpoint policy and release evidence.

Complete these structural corrections before adding another observation API:

- [x] Replace `ServerEvent`, `ServerEventCursor`, `ServerEventRecord`,
  `ServerEventBatch`, `ServerEventJournal`, and `ServerEventReader` with
  session-specific values and storage under `session/`. Replace
  `ReadServerEvents` and `WaitServerEvents` with entity-local session
  operations. Do not reserve a generic server event framework for hypothetical
  window, pane, or client reuse.
- [x] Replace `ClientLifecycle` with `StockClientLifecycle`; every current caller
  translates one accepted stock client into a server-owned `Client`.
- [x] Rename `pane/io.cpp` to `pane/pty_io.cpp` and `pane/resize.cpp` to
  `pane/terminal_resize.cpp`; their functions are already PTY- and
  terminal-specific.
- [x] Move descriptor ownership and errno translation out of `connection/` into a
  small POSIX boundary. `posix::OwnedFd` and `posix::errno_error` describe
  their actual reuse by sockets, PTYs, terminal routes, and control channels.
- [x] Move the Unix-socket connect/poll/send/receive implementation out of
  `protocol/native/`. A connection-side `native_request_transport` should use
  the native framing codec; the protocol directory should not own socket I/O.

Retain `OperationRegistry`, `CommandRegistry`, `KeyBindingRegistry`, and
`ActionRegistry` only for explicit startup registration. Each is consumed into
its corresponding immutable catalog before runtime lookup. `ServerApiCatalogs`
owns the native-operation, stock-command, and key-table catalogs. These types
have demonstrated multi-entry reuse and narrow responsibilities. Do not add a
base dispatcher, manager, service, peer hierarchy, or static self-registration
framework.
