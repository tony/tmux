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

## Window creation and selection slice

`CreateWindow` creates one PTY-backed shared window and its first membership
at an explicit unused session index. It preserves the session's current
membership. `SelectWindow` resolves a `WindowLinkId`, changes only its owning
session's current membership, and returns the resulting `SessionSnapshot`.
Both operations have entity-local registration and native connection adapters.

Registered `new-window`/`neww` adapters accept `[-d] -t session[:index]
-n name [command]`; omitted indices use the first free index from zero.
They invoke creation followed by selection unless detached. Registered
`select-window`/`selectw` accepts an exact `session:index` target. Parsing
rejects unsupported flags before effects; target resolution occurs at execution.
Implicit targets, automatic naming, hooks, and broader grammar remain open.

Prove detached creation, independent selection of linked windows, stale
membership rejection, invalid-before-spawn behavior, and actual PTY traffic.
Exercise the same operations through native, Python, Node, and real tmux 3.4
and 3.7 clients. Run the configured mid, documentation, analysis, sanitizer,
package, and standalone gates before recording the slice as verified.

## Interactive window navigation slice

Resolve each matched key against the invoking client's current session and
window membership. Prefix-n and prefix-p traverse memberships in index order
and wrap at either end. Prefix-0 through prefix-9 select that exact session
index. Missing indices preserve the attachment and selection; error/status
presentation remains an explicit compatibility limit.

Keep bindings beside window-membership operations and register their typed
adapters in the immutable key-table catalog. Use the existing `SelectWindow`
operation for mutation, sizing, and session ownership. Input routing must
resolve the newly selected window and pane before forwarding trailing bytes.
Creation via prefix-c remains separate until its implicit command and naming
policy is defined. Broader key parsing and configurable tables remain open.

Prove sparse-index wrapping, independent selections for linked sessions,
missing-index preservation, and same-read input redirection. Exercise real
tmux 3.4 and 3.7 attachments and restoration, then run configured gates.

## Client input fairness slice

Bound one client's key-routing work across selection changes in a daemon
turn. Retain the active table and unconsumed bytes when that allowance ends.
Expose deferred routing through the captured client route so the daemon
continues it on its next turn without waiting for more terminal bytes.
Effect-free program backpressure must still wait for useful readiness.

Prove that a large key batch yields, that selection changes cannot reset
the allowance, and that later turns finish ordered input without another
write. Use real tmux 3.4 and 3.7 attachments to test the poll continuation.

## Shared window naming slice

`RenameWindow` resolves a live `WindowId`, validates the replacement name,
and changes that shared window without changing memberships or selections.
An unchanged name preserves the window revision. Return a copied
`WindowSnapshot` through C++, Python, and Node.

Register `rename-window`/`renamew` with an explicit `session:index` target.
Resolve its membership at execution, then invoke the same typed operation.
Reuse creation's name constraints and reject malformed traffic before
mutation. Window events, automatic names, formats, and interactive rename
prompts remain separate unfinished behavior.

Prove shared-window ownership, unchanged-name behavior, invalid and stale
requests, real tmux 3.4/3.7 commands, and both generated language contracts.

## Shared window navigation slice

`NextWindow` and `PreviousWindow` resolve a live session's selected membership
and traverse its ordered indices with wrapping. They reuse `SelectWindow`
for mutation and sizing. Expose both through typed native requests and
copied session results in C++, Python, and Node.

Register `next-window`/`next` and `previous-window`/`prev` with explicit
`-t session` targets. Key bindings invoke those same operations; their
adapters only translate the resulting selection into input-routing effects.
Alert filters, implicit targets, hooks, and notifications remain open.

Prove sparse wrapping, single-window stability, stale sessions, independent
shared-window selection, real 3.4/3.7 commands, and generated interfaces.

## Pane border resizing slice

`ResizePane` moves the nearest matching layout border left, right, up, or
down by an unsigned cell count. It preserves the outer window size and
active pane, clamps at recursive pane minima, and returns the target's
copied geometry. A last child uses its preceding border, as tmux does.
Zero movement and an absent matching border preserve the revision.

Keep tree geometry in `WindowLayout` and PTY/model resizing in the pane
boundary. Reuse terminal-size validation and recovery across splitting,
client-driven window sizing, and manual resizing. Recovery can restore
sizes but cannot retract a program's already delivered SIGWINCH.

Register the typed operation and `resize-pane`/`resizep` adapter beside
other pane operations. Expose `resize_pane` and `resizePane` through the
existing binding workers and generated contracts. Initially accept one
direction, an exact pane target, and an optional positive signed-32-bit count.
Absolute dimensions, zoom, mouse resizing, implicit targets, percentage
sizes, notifications, and resize key bindings remain subsequent work.

Prove nested ancestor choice, edge-pane border movement, donor ordering,
minimum clamping, shared-window ownership, preserved selection, and
invalid-before-effects behavior. Compare geometry against owned tmux
3.4 and 3.7 servers and check a real program's accepted PTY dimensions.
Run focused RED/GREEN checks, mid checks, and all configured outer gates
before committing this slice locally.

## Named window layout slice

`Window` owns logical pane order independently of `WindowLayout` geometry.
Splitting inserts after the target in that order. Mirrored layouts move
the main pane without reordering pane identities or changing selection.

Register `SelectLayout` and `select-layout`/`selectl` under the window
component. A fixed enum and immutable name catalog declare all seven
presets. Build and validate a candidate partition tree before resizing
PTYs, then adopt geometry and revision together. Repeated identical trees
preserve the revision; a manual border move can be reset by reselecting.

Compare all seven presets with tmux 3.7 using both stock client versions,
owned servers, correlated pane identities, and actual program dimensions.
Characterize 3.4's older residual allocation as an explicit limit. Cover
residual cells, minimum expansion, invalid traffic before effects, linked
ownership, and splitting after a mirror. Expose the same operation through
Python and Node. Configurable main sizes, layout cycling, custom layouts,
border options, hooks, and notifications remain subsequent work.

## Window mode-key options

`SetWindowOption` and `ReadWindowOptions` provide typed global-or-window
targets through one mutation and one query identity. `Window` owns local
overrides, and server-owned global defaults remain live parents. Copied
snapshots include the server owner, optional window, effective enum, and
metadata revision. Both native bindings use the same connection operations.

The cross-scope `set-option` adapter belongs under `commands/`. Immutable
option definitions select session or window assignment variants; flags have
their own immutable description table. `set-window-option` shares parsing
but forces window scope. Resolve membership targets when executing.
Only `prefix`, `prefix2`, and `mode-keys` are available; unset, listing,
implicit targets, and further option types remain open.

Critical cases cover live defaults before and after window creation,
explicit equal-value pins, linked ownership, unchanged assignments,
editor-derived startup, malformed requests/replies, and rejected flags
without state changes. Real tmux 3.4/3.7 commands compare effective values
with their own reference servers. Installed Python and Node Current/LTS
must mutate and observe the same packaged window options.

## Basic copy-mode slice

Keep this milestone open until a real attached client enters copy mode,
navigates retained history, selects and copies bytes, leaves the mode, and
pastes into its persistent program. Python, Node, and stock commands must
reach the same typed operations.

`Pane` owns the optional `CopyMode`; the mode owns copied backing rows,
cursor, selection, viewport, and display freshness. tmux 3.4 and 3.7 copy
their backing grid on entry. Program parsing, raw retention, query replies,
and child lifecycle continue behind the frozen display. Cancellation selects
the current program screen without replaying raw output. Re-entry preserves
the existing mode and selection. Detaching a client preserves pane mode.
Resizing reflows the mode-owned backing, remaps its cursor, clears selection,
and advances display freshness; program-terminal resize remains separate.
Mode entry, cancellation, and retained-display resize must work after PTY
EOF. Prepare all mode reflows before notifying live programs; preserve the
original mode if preparation fails. Delivered program effects cannot be
rolled back by restoring dimensions.

`mode-keys` belongs to the shared `Window`. Its existing `OptionTable` inherits
from server-owned global window defaults, with typed `WindowModeKeys`
values in a window-local immutable option catalog. `SetWindowOption` owns
the typed global-or-window target and one wire identity. Linked sessions
observe the same override. Assigning the inherited value still creates an
override; repeated local assignment is a no-op. Session prefix options
remain separate; both bindings expose window assignment and effective reads.
Global changes advance affected inherited windows' revisions and preserve
explicit overrides. Unknown names, cross-scope assignments, malformed
values, and stale targets fail before changing values or revisions.

Read effective mode keys for the current key table, cursor movement, and
selection extraction. Capture the upstream selection-highlight keys on
mode entry; changing the window option must preserve backing and selection.
Entry accepts only the pane target, with no separate key-style override.
The window option defaults already follow tmux's `VISUAL` or `EDITOR`
startup rule. Copy-mode interpretation and highlight caching remain open.

`TerminalHistorySnapshot` supplies chronological owned rows at their original
widths, continuation-from-previous flags, cursor, parser revision,
completeness, and discarded-prefix evidence. Alternate-screen capture excludes
hidden primary history. `PaneScreenSnapshot` supplies only the pane's chosen
display; client composition tracks display freshness independently of parser
facts. Mode entry, navigation, resize, and exit must invalidate that baseline
even when the parser has not advanced.

Use entity-local `EnterCopyMode`, `MoveCopyCursor`, `ScrollCopyMode`,
`BeginCopySelection`, `ClearCopySelection`, `CopySelection`, `CancelCopyMode`,
and `ReadCopyMode` descriptions. Register `send-keys -X` names in an immutable
copy-mode command catalog. Persistent `copy-mode` and `copy-mode-vi` key tables
consume unbound mode input; the prefix table keeps precedence. `CopySelection`
adds bytes through the existing automatic paste-buffer operation and changes
selection only after successful storage.

The incremental terminal key decoder retains raw spans separately from
normalized keys. Bound CSI/SS3 and UTF-8 parsing; pass unmatched program input
through unchanged. Hold an incomplete Escape token against one monotonic
deadline without extending it as fragments arrive. The readiness loop folds
that deadline into its existing poll timeout. Backpressure preserves complete
tokens and prefix state; mode entry or exit causes routing to resolve the pane
again before processing the next key.

Prove source-defined emacs half-open and vi inclusive selections, reverse
selection, wide/combining glyphs, soft-wrap joining, hard newlines, and empty
selection. Real-client cases must cover frozen display while program/query
state advances, two clients sharing one pane mode, resize, eviction after
entry, split input encodings, cancellation with following input, prefix
precedence, and unknown keys reaching no PTY. Automatic-buffer capacity or
input backpressure must preserve selection or queued input before effects.

The initial cursor rule follows tmux 3.7; declare 3.4's permitted one-past-end
vi position separately. Vi Escape clears selection and stays in mode;
emacs Escape cancels. The default vi copy key is Enter, without a `y`
binding. Vi extraction includes the terminal cell, emacs excludes it, and
only complete Unicode cells are copied. Preserve hard newlines and join
soft rows only when selection reaches their original end. Vi retains the
final newline only when its extraction endpoint passes the trimmed final
row; an empty selected row can yield a newline. Original tab
reconstruction needs parser provenance that the current terminal model
does not retain. Clipboard and copy-pipe effects remain explicit limits.

Source contracts use [tmux 3.7 copy mode][copy-37],
[tmux 3.4 copy mode][copy-34], [window option ownership][window-37],
and [default key bindings][keys-37] at pinned revisions. The current parser
history has reflow disabled; the mode-owned backing still requires its own
reflow implementation. The incremental key decoder and every mode operation
remain unfinished.

[copy-37]: https://github.com/tmux/tmux/blob/81f88f8517c9fc5371b56cf117530c6b477c96ac/window-copy.c
[copy-34]: https://github.com/tmux/tmux/blob/9ae69c3795ab5ef6b4d760f6398cd9281151f632/window-copy.c
[window-37]: https://github.com/tmux/tmux/blob/81f88f8517c9fc5371b56cf117530c6b477c96ac/window.c
[keys-37]: https://github.com/tmux/tmux/blob/81f88f8517c9fc5371b56cf117530c6b477c96ac/key-bindings.c

## Architecture progress

Named server/session/window/pane components, entity-local operation descriptions, and validated immutable native lookup now replace the generic domain module and operation switch. The public facade is `ServerConnection`, snapshots are values, and recoverable failures are `TmuxError`. Session/window membership and pane-program cleanup have real ownership models.

The first shared-server milestone passes with C++, Python, Node, and stock tmux 3.4. Native registration, malformed-before-effects, duplicate/closed registration, immutable catalog lookup, linked-window lifetime, binary bytes, stale IDs, restart identity, and direct-child cleanup have focused cases.

Documentation has compiler coverage, strict Doxygen HTML/XML, language AST checks, budget/filler rules, and controlled negative probes. Ruff/ty and Oxc/tsgolint/TypeScript remain independent gates.

Native negotiation now declares the cataloged operations, effects, capabilities, and payload limit. Entity operations and native adapters have separate files. Stock profile detection distinguishes tmux 3.4 and 3.7 headers using descriptor evidence; real clients pass the declared one-shot and guarded control-command subsets. Accepted-client identification, one-shot admission/output acknowledgement, terminal attachment/detachment, active control mode, control-output draining, and transport closure use exclusive variant phases with phase-owned descriptors, policy, output, and deadlines.

The terminal foundation retains chunked VT parsing, screens, cursor/rendition, bounded history, and ordered query replies. Real PTY tests exercise query responses and nonblocking input remainders. Owner-qualified pane text, pane splitting and selection, recursive window layouts, the `latest` window-size policy, and named or automatic paste buffers are available through the shared C++ server and all three native clients. Native buffer enumeration exposes copied metadata. Ordinary-client redraw composes every pane with basic ASCII borders and the selected cursor. Stock buffer listing, richer paste modes, copy mode, capability-sensitive borders, other size policies, and broader terminal fidelity keep milestone 4 open.

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
