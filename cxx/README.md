# tmux-cxx

A C++23 tmux implementation in development. One C++ daemon owns persistent sessions, shared windows, window memberships, panes, and PTY programs. The Python and Node extensions use the same native `ServerConnection`.

The current implementation supports creating, listing, renaming, and deleting sessions; linking and unlinking windows; splitting, selecting, and listing panes; sending or observing pane bytes and interpreted text; and storing, reading, deleting, and pasting explicitly named buffers. Each window owns its pane layout, each pane retains its PTY, terminal parser, screen, and bounded history, and the server owns paste buffers independently of panes and clients. Session deletion preserves windows linked into other sessions. Connection disposal preserves sessions. Pane deletion schedules child termination and reaping without blocking other requests.

Stock tmux 3.4 and 3.7 clients can use the declared command subset, including `split-window`, `select-pane`, `set-buffer`, `paste-buffer`, and `delete-buffer`, plus attach, detach, resize, and input. Buffer commands currently require explicit names, and paste requires an exact pane target. Automatic buffers, buffer listing, delete-after, custom separators, raw paste, bracketed paste, and copy mode remain open. Ordinary-client redraw composes every pane, basic ASCII borders, and the active-pane cursor from copied state. Capability-sensitive border glyphs and styles, modes, control-mode format subscriptions, and broader command compatibility remain unfinished. A profile-selected C++ adapter can query session names from owned tmux 3.4 and 3.7 servers. The retained terminal component establishes tested parser behavior, not complete terminal fidelity. [GOAL.md](GOAL.md) retains the complete objective; [the implementation plan](docs/implementation.md) tracks its milestones.

Single `-C` clients from tmux 3.4 and 3.7 can run an initial registered command and later newline-delimited registered commands with matching `%begin`, `%end`, and `%error` guards. Attached control clients can switch sessions, receive selection changes, observe new raw bytes from linked panes, and receive committed session catalog and rename notifications. Attachment and switching establish fresh per-pane output baselines, so earlier raw bytes are not replayed. The adapter bounds command lines, argument count, commands per readiness turn, queued output, pane output, and a 4096-record metadata journal. Echo-disabled `-CC`, `no-output`, `pause-after`, and `wait-exit` are rejected during identification before command effects. Command groups, configuration grammar, asynchronous commands, other notifications, pause/continue, control-mode format subscriptions, and control-client gap resynchronization remain unsupported. The named `unattached_control_channel_stays_open` quirk exposes the deliberate user-facing difference from stock tmux: an unattached limited client stays open after its initial command.

## Build and check

Run these commands from this directory. Bootstrap installs locked tools and language dependencies locally. The stock test fixture requires the exact Ubuntu tmux 3.4 binary recorded in the lock; another binary fails verification.

Explicit tool updates resolve current stable releases and pin their sources and checksums in `support/toolchain.lock.toml`. LLVM and its parser bindings move together. npm has its own current stable pin because Node's bundled npm can lag. Python and Node package dependencies retain their separate lockfiles.

The modern fixture builds unmodified tmux 3.7 from its locked source archive with the locked Clang. It requires the recorded host Autoconf, Automake, Make, libevent, and ncurses development tools. Reference fixture binaries remain test inputs.

```console
$ python3 support/bootstrap.py \
    --provision \
    --python \
    --node \
    --modern-fixture \
    --stock-fixture /usr/bin/tmux
```

Configure and build both native extensions with the C++ daemon.

```console
$ _build/tools/just/just configure
```

```console
$ _build/tools/just/just build
```

Run native tests, real-server pytest and Vitest suites, Ruff, ty, Oxc syntax and type-aware lint, TypeScript, formatting, and generated-interface drift checks.

```console
$ _build/tools/just/just check mid
```

Generate Doxygen HTML/XML and check compiler-derived callable coverage, Python docstrings, and JSDoc. Controlled negatives prove missing documentation, malformed tags/references, filler, duplicated contracts, and excess words fail.

```console
$ _build/tools/just/just docs
```

Run clang-tidy across compiled first-party units and the generated exposure contract. The isolated sanitizer fault probe is excluded because it deliberately leaks and accesses outside an allocation. A controlled division error proves analyzer findings remain fatal; reports retain other diagnostics for review.

```console
$ _build/tools/just/just analyze
```

Build the sanitizer preset, then run instrumented native, Python, and Node suites. Controlled memory, overflow, and leak probes must fail even with the documented external-library leak suppressions.

```console
$ _build/tools/just/just configure sanitize
```

```console
$ _build/tools/just/just build sanitize
```

```console
$ _build/tools/just/just sanitizers
```

Replay the retained protocol and terminal seeds, then run 2000 bounded fuzz iterations per target under ASan/UBSan. Generated corpus files are temporary; failure inputs remain available for diagnosis and regression coverage.

```console
$ _build/tools/just/just fuzz
```

Build and consume the Python sdist/wheel and npm tarball without network or
source-tree fallbacks.

```console
$ _build/tools/just/just package-check
```

Copy authored sources without build state, rebuild both bindings, install the
native package, load both adapters, and run an external C++ consumer.

```console
$ _build/tools/just/just standalone-check
```

Checks preserve source files. Formatting recipes are separate. The locked
`just` lists recipe groups when invoked without arguments. The release build
and exported CMake package pass with locked Clang 23. The complete configured
suite also passes with the locked Ubuntu GCC 14 fallback. The source
distribution carries the verified libvterm source archive and excludes local
dependency trees; the npm package installs its prebuilt addon without runtime
dependencies. A clean source copy builds both bindings and an installed native
consumer without borrowing first-party sources from the working tree. Current
upstream GCC, broader binding lifetimes, daemon discovery, and current-Ubuntu
verification remain open.
The same addon runs against locked Node Current and LTS binaries.

## Session event streams

Set `TMUX_CXX_SOCKET` to a running C++ server. Python offers the same synchronous stream as C++ plus an asyncio adapter whose cancellation closes only that observer.

```python
import asyncio
import os

import tmux_cxx


async def main() -> None:
    async with tmux_cxx.ServerConnection(os.environ["TMUX_CXX_SOCKET"]) as connection:
        stream = await connection.subscribe_sessions_async()
        async with stream:
            print(stream.initial.sessions)
            print(await stream.next(timeout_ms=500))


asyncio.run(main())
```

Node exposes promise-backed reads and pull-driven async iteration. An `AbortSignal` closes the stream and rejects with its reason.

```javascript
import tmux from "tmux-cxx";

const socket = process.env.TMUX_CXX_SOCKET;
if (!socket) throw new Error("TMUX_CXX_SOCKET is required");

const connection = new tmux.ServerConnection(socket);
const stream = await connection.subscribeSessions();
try {
  console.log(stream.initial.sessions);
  console.log(await stream.next({ timeoutMs: 500 }));
} finally {
  stream.close();
  connection.close();
}
```

## Architecture

`src/server/`, `src/session/`, `src/window/`, `src/window_link/`, `src/pane/`, and `src/client/` follow tmux ownership. `WindowLink` owns a session's window index; the shared `Window` owns its layout and panes. Public snapshots contain copied values and expose no entity pointers.

Each operation declares its request, result, native identity, effects, and required capabilities. Startup checks entity registrations against the generated public operation catalog, then consumes the registry into immutable runtime lookup before opening sockets. Missing identities, changed descriptions, or undeclared capabilities/effects stop startup. Registered stock commands parse their own options and resolve targets at execution. Typed key bindings declare their table, normalized key, tmux command, and required operation; startup freezes them into an immutable `KeyTableCatalog`. Entity-local CLI adapters own their argument decoding, invocation, and usage synopsis; `main.cpp` selects the process role and invokes the immutable action catalog. The event loop handles readiness and operation-owned continuations.

`protocol/native/` owns framing and version/capability negotiation. `connection/` owns native request sockets, admission, and pinned server identity. Entity-local connection adapters encode each operation. Python and Node translate values, errors, bytes, and scheduling; C++ owns tmux validation and behavior. All production inputs remain independent of the surrounding read-only C tmux tree.

`observe_sessions`/`observeSessions` returns copied sessions and the first committed event after that atomic snapshot. `read_session_events`/`readSessionEvents` reads retained changes immediately; `wait_session_events`/`waitSessionEvents` waits for at most 750 milliseconds and returns an empty batch at its deadline. Native replies must retain the pinned server owner and continue the requested sequence exactly. The 4,096-record journal reports a gap after discarded history.

`subscribe_sessions`/`subscribeSessions` creates a `SessionEventStream` that owns that baseline, its cursor, buffered records, cancellation, and gap recovery. A discarded prefix produces one `SessionEventGap` with a replacement atomic observation, then later reads continue from its cursor. C++ and Python expose bounded synchronous `next` calls; Python also provides `subscribe_sessions_async`, asynchronous iteration, and task-cancellation cleanup while releasing the GIL during native waits. Node exposes promise-based `next`, a pull-driven `AsyncIterable`, `AbortSignal` cancellation, and asynchronous disposal without blocking the JavaScript thread. One stream admits one read at a time. Callback delivery, reconnection, and resumable cursors remain open.

`AcceptedClientProtocol` retains tmux wire state for one accepted client; the server event loop owns its socket and the server model owns the corresponding `Client`. A closed variant gives identification, one-shot command admission/output, terminal attachment/detachment, control mode, control-output draining, and transport closing exclusive state and ownership. `protocol/tmux/control/` owns the bounded control channel, command-line subset, guarded result blocks, shared server command numbering, `SelectedSessionOutputFeed`, and `SessionNotificationFeed`. Raw pane offsets and committed metadata cursors remain separate observer-owned baselines. `StockCommandExchange` retains the shorter one-command reverse wire state used by `StockServerConnection`. Shared tmux codecs and profile declarations contain no role lifecycle. Reverse operations remain entity-local and typed; raw stock argv exchange is private to their adapters.

`terminal/` owns retained libvterm state independently of PTYs and client rendering. `ClientViewRefresh` selects and queues current-state window redraws; `ClientTerminalRouting` applies descriptor readiness; `ClientKeyInput` retains each client's active key table and bounded undecided input; `ClientTerminal` owns delivery and restoration. Input reaches the selected pane only after key-table lookup. `set-option -g prefix|prefix2 value` changes inherited defaults; `set-option -t target prefix|prefix2 value` changes one resolved session through the same typed operation. Supported values are `None`, one ASCII byte, and `C-a` through `C-z`. The prefix table binds `C-b` to send the configured primary prefix, `"`/`%` to split, `d` to detach, and `o` to select the next pane. It consumes unmatched prefix-table input as tmux does and retries a binding that reports effect-free backpressure. `read_pane_text`/`readPaneText` returns copied UTF-8 rows, server/pane ownership, parser freshness, and input status. `send_pane_input`/`sendPaneInput` admits at most 64 KiB per request into a bounded ordered queue; completion can precede delivery. Terminal replies retain stream order through partial PTY writes. See [ownership and observation limits](docs/design.md).

`paste_buffer/` owns server-global named byte buffers and one typed file per operation. Names are valid UTF-8 without control bytes and use at most 128 bytes. The server retains at most 50 buffers of 256 KiB each. Pasting maps line feed to carriage return and admits the complete request through the pane's existing program-input queue; it does not make the buffer pane-owned or consume it.

Session names must be valid UTF-8 and satisfy the documented name bounds. Shared server validation rejects malformed names before creation or rename effects. Program commands and raw pane traffic retain their byte representation.

## Compatibility evidence

The shared-server integration test creates a PTY-backed session through the C++ CLI, mutates it through Python and Node, and inspects that same state with an unmodified tmux 3.4 client. It also exercises bytes, errors, and persistence after disposal.

Separate real-client tests cover detached named creation, listing, renaming, global and targeted prefix assignment, pane splitting and selection, named-buffer storage/paste/deletion, multi-pane redraw, attachment, resize, active-pane traffic, detachment, and the guarded control-command subset with unmodified tmux 3.4 and 3.7 binaries. Control cases also cover no-replay attachment, session switching, linked-pane output, session rename/catalog notifications, and detachment after selected-session removal. The stock adapter detects the clients' distinct imsg layouts from descriptor markers. It retains unknown release evidence because both layouts use protocol integer 8. A grouped control command is rejected before session or program effects.

Reverse-role tests select a declared profile before connecting, obtain release evidence from the connected server through `#{version}`, and query typed session names. Both mismatched profile pairs are rejected before the same owned server accepts the correct pair. This establishes a narrow command-mode subset, not general command execution.

Complete key decoding, configurable tables and bindings, repeat handling, modes, broader control grammar and notifications, pause/continue, control-mode format subscriptions, older release subsets, and language-visible reverse compatibility reports remain open.

See [documentation policy](docs/documentation.md), [source provenance](docs/attribution.md), and [verification progress](docs/progress.md).
