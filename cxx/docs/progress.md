# Execution ledger: independent tmux implementation

Spec: `../GOAL.md`; plan: `implementation.md`.

Starting state: branch `cpp`, HEAD `d6a84376f`; staged and unstaged diffs
empty; no untracked files. The parent C source revision is
`d1fedcc9a` (resolve its full immutable ID before recording attribution).
There was no `cxx/` implementation at start.

Ruling: keep plans and execution artifacts here instead of the skills'
repository-root defaults. The explicit cxx-only boundary takes precedence.

Ruling: execute the user's detailed implementation objective without another
design-approval prompt. The user has authorized implementation and ongoing
goal work. The supplied objective remains the binding specification.

Pre-flight: both bindings use one native `ServerConnection` and one API
description. Stock tmux traffic and native API requests reach the same server
operations. No independent binding-side server semantics are permitted.

2026-10-05: Read the complete supplied objective and root rules. Host is
Ubuntu 24.04.5 x86_64. Refreshed package lists inside `_build/apt`. Newest
final Ubuntu is 26.04.1; 26.10 is beta. Docker has no running daemon;
unprivileged user namespaces are available. No functional gates have run.

2026-10-05 architecture activation: the current starting state is branch `cpp`, HEAD `d6a84376f`, with the pre-existing untracked `cxx/` scaffold and empty staged/unstaged diffs. Preserved first-party inputs and hashes under `_build/recovery/architecture-start/` and `_build/reports/architecture-start.json`. The complete revised objective is now in `GOAL.md`; documentation/tooling policy is in `docs/documentation.md`.

Ruling: reuse and revise the existing inline implementation plan against the activated architecture. The user supplied the reviewed skeleton as the active objective; further design approval is already covered by that authorization. The full goal remains open.

Ruling: use Clang 23 with libstdc++ consistently for the default build. The locked libc++ headers do not define `std::move_only_function`; a direct Clang/libstdc++ compile probe succeeds. Added a required continuation facility gate instead of replacing the C++23 facility with a custom callback abstraction. Latest stable GCC provisioning and alternate compiler/runtime verification remain open.

Tool repair evidence: the initial full build failed on LLVM LLD's absent ICU 70 runtime and Node-API's qualified registration macro. Pinned the verified Ubuntu runtime archive and GoogleTest 1.18.0 immutable source archive, provisioned them project-locally, and restored full native linking. Node runtime test passed. Python context-manager tests initially failed on None arguments; explicit nanobind None acceptance restored those three existing tests. New Python/Node error-fidelity assertions fail on missing code/context as expected. Native registry compile-contract gate fails on missing entity/registry interfaces before implementation.

2026-10-05 current architecture: removed the central native operation switch and hard-coded stock command executor. Native operations use entity-owned typed descriptions and validated frozen registration. Stock commands have their own typed registration, option parsing, target resolution, and format helpers. The shared facade is `ServerConnection`; errors retain tmux classifications, operation, and observed server identity.

Ownership: the server now owns actual `Session`, `Window`, `WindowLink`, and `Pane` entities. Memberships own session indices. Linked-window tests prove deleting one session preserves a shared window and pane until the final membership disappears. The C++ connection, nanobind, and Node-API products expose link/unlink and materialized window/membership values.

Lifecycle: the HUP-ignoring regression originally failed when session deletion exceeded the 900-millisecond native request deadline. Pane removal now separates PTY disposal from `PaneProgram` termination/reaping, observes pidfds, escalates after 250 milliseconds, and bounds explicit server shutdown at 700 milliseconds. The regression passes and verifies the owned direct child was reaped. A separate controlled daemon-death test exposed a surviving direct child; parent-death KILL now passes that case.

Identity: a connection whose first operation failed originally accepted a replacement daemon and mutated its new session. The controlled restart test failed before the fix. Error replies now pin the observed instance; the same endpoint's replacement rejects the old connection before mutation. This does not yet provide owner-qualified live handles or native capability negotiation.

Functional milestone: the owned shared-server test passes through the C++ CLI, Python, Node Current, and the isolated unmodified Ubuntu tmux 3.4 binary. It covers PTY input/output, a real error, generated/type-checked interfaces, and persistence after connection disposal. Stock evidence remains a narrow command subset; interactive, control, older-release, and reverse-direction baselines are unverified.

Tooling: scoped `just` recipes delegate to CMake/CTest and the thin check runner. Ruff/format/ty and oxlint/oxfmt/oxlint-tsgolint/TypeScript are separate required gates, including invalid consumers. API exposure assertions compile; nanobind generates actual built-module stubs; Node declarations and Python runtime briefs share canonical C++ contracts. Raw bytes preserve invalid UTF-8 and NUL. Node-API now enables its catch-all C++ exception boundary, with a compile-time requirement.

Documentation: Doxygen 1.18 strict HTML/XML generation has zero warnings. Clang 23 inventories 250 native callables using real compile flags and checks that authored native inputs were compiled. Canonical copy references and parameter tags resolve against compiler declarations. Python and Node AST inventories check briefs. Controlled negatives pass for public/private/static/lambda omissions, bad parameters, ambiguous and missing references, budgets, selected filler, suppressions, and duplicated contracts. Positive fixtures extract actual concepts, `std::expected`, and `std::move_only_function`. The pinned stop-slop policy and license are provisioned in the local tool directory. Doxygen's bundled Clang parser is disabled because it cannot parse the required standard-library types; independent Clang coverage remains required.

Verification checkpoint: the configured mid loop most recently passed in 3.289 seconds before the mechanical drain/reaper naming sweep. The documentation command passed within the 60-second stretch budget. Subsequent source changes require fresh checks. Fixtures now keep temporary directories inside the configured C++ build directory and use short owned socket names.

Still open: full native negotiation, runtime exposure/capability validation, entity generation/owner-qualified cursors, terminal/client models and usable interactive behavior, both stock roles and declared release profiles/quirks, metadata/raw/view subscriptions, cancellation and async delivery lifetimes, bounded addon admission, package/install consumers, alternate compiler/runtime and Ubuntu gates, sanitizer/fuzz/analysis gates, and the final full objective audit. `GOAL.md` remains active and complete in scope. No commit, push, tag, publication, or parent-tree edit has occurred.

2026-10-06 protocol and terminal checkpoint: native negotiation declares version, payload bounds, frozen operation identities, effects, and capability requirements. Entity-local connection adapters retain shared C++ validation. The shared reply codec rejects malformed ownership, unknown error classes, invalid UTF-8 failure text, and trailing error bytes before identity pinning. Four adversarial peer cases failed before the codec fix and now pass.

Stock ownership: `AcceptedClientProtocol` owns the wire state bound to one accepted stock client: profile detection, identification, transferred descriptors, mode admission, and output-stream acknowledgement. The event loop owns its socket. Native and stock states occupy separate transport variants. Real tmux 3.4 and 3.7 clients pass detached creation, listing, and renaming. Controlled regressions prove late mode flags and a second command during output acknowledgement cannot reach mutation. Release identity remains unknown from framing alone; full control mode, reverse-role, and older-release support remain open.

Terminal ownership: pinned external libvterm 0.3.3 supplies retained parser/screens through a C++ owner. Tests cover chunked UTF-8/CSI, cursor replies, alternate-screen restoration, bounded history, resize, and explicit callback/reply-capacity failure. A real PTY supplies a cursor-query response and copied pane text to the bindings. Owner-qualified text and parser freshness remain distinct from raw output and metadata revisions. Reflow, client composition, full terminal fidelity, and exposed pane geometry remain open.

Input and children: the bounded program-input queue preserves nonblocking write remainders and earlier terminal replies. The 64-KiB real-program transfer failed before the queue fix and passes now. Unexpected external collection relinquishes child ownership and retains an unknown exit status; direct-child signals use pidfd when available. Sole child-reaper/process-group ownership remains a caller obligation. Broader descendant cleanup and asynchronous failure delivery still require proof.

Verification: the configured mid loop passed in 3.669 seconds, including 21 native tests, 24 pytest cases, and the genuine Node scenario on Current and LTS with one addon. Ruff/ty, oxlint/oxfmt/tsgolint, compiler type checks, formatting, and generation drift remain independently required. Strict documentation passed in 49.808 seconds with 329 compiler-inventoried callables, language AST checks, and controlled probes. Clang-tidy checked 60 authored/generated translation units in 26.582 seconds and proved a controlled division error remains fatal; nonfatal warnings still need review. Reports retain complete output under `_build/reports/`.

Installed native evidence: the exported server target creates a real PTY pane, reads copied terminal text, deletes the session, and shuts down through an independent CMake consumer. This does not establish fresh source-distribution, wheel, or npm products. Locked just 1.58.0 now provisions locally and lists the scoped recipes. Other tool records retain their 2026-10-05 resolution date.

The complete goal remains active. Remaining gates include runtime exposure/public reports, live handles and subscriptions, cancellation and bounded binding lifetimes, usable multiplexing and both stock roles, installed language products, alternate compiler/current-Ubuntu verification, instrumentation/fuzzing, and the complete line-by-line architectural audit. The current sanitizer build covers native targets only so far. An independent review of the implemented terminal/protocol/lifetime slice is in progress. Root staged/unstaged diffs remain empty with only `cxx/` untracked; no parent-tree or publication change has occurred.

## 2026-10-06: Pane servicing and instrumented products

Pane I/O now belongs to `src/pane/io.cpp`. One readiness pass shares separate 64-KiB read/write allowances across every nested flush; operations admit input without performing writes. Controlled regressions failed for both original budget defects before the fixes. Additional cases verify exact patterned EAGAIN writes, queue-pressure reply ordering, and EOF admission closure. Removing EOF closure deliberately fails its regression; restoring it passes. The independent reviewer confirms both budget findings resolved and found no Critical or Important regression from the move.

The short-read socket fixture proves the byte allowance. Real-PTY SHA256 verification checks exact patterned input delivery. Full queue/reply pressure remains socket-fixture evidence. Malformed-envelope cases now include invalid UTF-8 and reuse one compiled connection with a different valid owner to prove rejection precedes pinning.

| Gate | Result | Complete gate command |
|---|---|---:|
| Native/language/type/format/generation mid loop | PASS | 6.263 s |
| Strict documentation and negative probes | PASS; 341 C++ callables, zero Doxygen warnings | 52.885 s |
| Clang-tidy and fatal negative probe | PASS; other diagnostics retained for review | 33.055 s |
| ASan/UBSan native/Python/Node and fault probes | PASS | 5.406 s |

The compiled suites contain 26 native tests, 25 pytest cases, and real-server Vitest coverage on Node Current/LTS. Native code, both bindings, nanobind, and libvterm use coherent instrumentation. Matching shared-runtime loading is explicit; fixture environments prevent injection into unrelated shell programs. Empty Node hosts and a Vitest fixture without our addon reproduce two external leaks. Narrow OpenSSL/Rolldown leak suppressions leave first-party memory, overflow, and leak probes fatal.

Compiler documentation coverage previously merged independent executable entrypoints under Clang's shared `main` identity. A failing-then-passing regression now preserves both entries. The isolated sanitizer fault executable retains documentation/format coverage and has one named analyzer exclusion for its intentional defects.

The full product objective remains active: interactive/client/layout behavior, complete stock roles and older subsets, public reports, runtime exposure validation, subscriptions/cancellation/bounded binding lifetimes, installed language consumers, alternate compiler/current-Ubuntu verification, fuzzing, and the final full audit are unfinished. Repository isolation and publication state remain unchanged.

2026-10-06 public catalog checkpoint: a shared generator emits compile assertions and the immutable operation catalog from the exposure description and entity-owned metadata. The installed connection archive exports it without a language-runtime dependency. Startup validates required identities, names, effects, and capability references before freezing lookup and opening sockets. Four deliberately mismatched descriptions fail the focused test; omitting the actual pane-text registration stops the daemon before readiness or socket creation in 0.0014 seconds. The source is restored and the shared-server suites pass.

Compiler documentation coverage now includes the exact generated exposure file and fingerprints its contents. The generated-function probe failed before that inclusion and passes after it. Fresh checks pass: mid 4.663 seconds, strict documentation 56.238 seconds with 344 native callables and zero Doxygen warnings, analysis 36.050 seconds with other diagnostics retained, and sanitizer suites/probes 5.318 seconds. The configured suites contain 27 native tests, 25 pytest cases, and real-server Node Current/LTS coverage. The installed native consumer successfully calls the catalog and creates/observes/removes a PTY-backed session.

This completes the startup exposure-validation slice. Public capability reports, full protocol/product behavior, language packages, broader lifetimes, compiler/platform checks, fuzzing, and the final audit remain open. Independent review of this slice is pending.

## 2026-10-07: Current tools and terminal hardening

Explicit resolution now records Doxygen 1.18.0, CMake 4.4.4, LLVM 23.1.3,
Python 3.14.8, Node Current 26.11.0, Node LTS 24.21.0, npm 12.2.0,
Graphviz 16.1.0, Ruff 0.16.10, and ty 0.0.85. The lock encoder preserves
nested libvterm backport provenance; a focused tooling test round-trips the
complete lock. GCC 16.2 is the current upstream stable release, but a matching
project-local Ubuntu package remains unprovisioned. Ubuntu 26.04.1 is the
newest final release; 26.10 remains beta at this checkpoint.

Terminal fuzzing exposed four distinct libvterm boundaries: encoded C1
controls could move the cursor before the screen, large cursor movements could
overflow coordinates before clipping, an oversized OSC number could wrap into
a recognized command, and REP before any graphic character could loop without
progress. Exact Vim fixes cover the control-width and REP defects. Two
C-linkage functions implemented in C++23 saturate untrusted coordinate and OSC
arithmetic before the generated external C call sites apply libvterm bounds.
Provisioned dependency sources remain unchanged. Four retained inputs reproduce
the defects, and the sanitizer corpus replays them on every fuzz run.

| Gate | Result | Complete gate command |
|---|---|---:|
| Native/language/type/format/generation mid loop | PASS | 11.862 s |
| Doxygen 1.18 and documentation probes | PASS; 357 C++ callables, zero warnings | 17.639 s |
| Clang-tidy and fatal negative probe | PASS; retained diagnostics remain reviewable | 59.944 s |
| ASan/UBSan/LSan suites and fault probes | PASS | 6.663 s |
| Three sanitizer fuzz targets, 2,000 runs each | PASS | 2.479 s |
| Clean installed native consumer | PASS | session, terminal, catalog, and shutdown exercised |

The verification sandbox denies the socket-buffer controls used by pane-I/O
fixtures and prevents LeakSanitizer from inspecting traced processes. Final mid,
sanitizer, and fuzz results ran outside those restrictions with leak detection
active. The installed libvterm archive contains both C++ arithmetic functions,
and a clean external consumer links and runs against that archive.

The complete goal remains active. Client entities, interactive attachment,
layouts and splits, control mode, the reverse stock role, public compatibility
reports, subscriptions and recovery cursors, asynchronous binding lifetimes,
installed Python and Node products, GCC/current-Ubuntu execution, and the final
line audit remain open.

## 2026-10-07: Role-specific stock adapters

The accepted-client adapter is `AcceptedClientProtocol`. It owns detection,
identification, mode, and stream state; the server event loop owns its socket.
Shared tmux declarations
now cover host-ABI payloads, the active message catalog, and ordered legacy and
modern identification. `StockCommandExchange` owns one outbound command's wire
state without sharing accepted-client lifecycle.

`StockServerConnection` selects an explicit profile, opens fresh bounded
command sockets, and obtains release evidence from the connected server through
`#{version}`. Its public surface no longer accepts arbitrary argv. The first
reverse operation, `session.list_names`, lives under `session/operations/`,
owns fixed command construction and bounded parsing, and contributes its
`list-sessions` spelling through the session component's validated reverse
catalog. This preserves the entity hierarchy and prevents undeclared commands
from bypassing operation policy.

Real tmux 3.4/legacy and 3.7/modern server pairs pass release verification and
the typed session-name query. Both crossed profile pairs are rejected, after
which the same owned servers accept the declared profiles. A live timeout was
traced to stock stdout/stderr streams: streams 1 and 2 end at `MSG_EXIT` and do
not receive `MSG_WRITE_CLOSE`. A failing scripted case now models that rule,
and the message catalog includes the previously omitted active
`MSG_WRITE_DONE` identity.

This completes only the narrow reverse command slice. Broader typed reverse
operations, control mode, older-release subsets, full interactive behavior,
language-visible reverse reports, and the remaining product objective stay
open.

## 2026-10-07: Entity hierarchy and protocol roles

The source tree now gives `WindowLink` its own public and implementation
hierarchy. Entity operations keep their behavior, native adapter, and
registration under `session/`, `window/`, `window_link/`, `pane/`, or
`client/`. `build_server_api_catalogs` is the explicit composition root: it
names entity registrars, validates public native descriptions and command
references, then freezes both catalogs. The server daemon contains no
operation-specific registration or dispatch branch.

The inbound stock adapter is `AcceptedClientProtocol`. It retains profile detection,
identification, command-stream, attach, and detach state while
`AcceptedConnection` owns the socket and queued transport bytes.
`StockCommandExchange` retains the shorter one-command reverse role. Both
message state machines delegate named transitions instead of accumulating
command and stream behavior in one conditional chain. Protocol message
branching remains in each role because it expresses closed wire state, while
tmux command behavior continues through the frozen command registry.

Reverse adapters no longer appear in a central description array. The session
component registers `ListStockServerSessionNames` into a validated
`StockServerOperationCatalog`; the connection composition root freezes it and
derives compatibility commands from it. A focused test proves entity
registration and late-registration rejection. The operation reports that its
multiple live queries can fail if sessions change between identity and name
reads.

The terminal-model review tightened the observation boundary. `ClientView`
composes current retained state, and `record_queued_redraw` records only queue
acceptance. It does not claim transport completion, downstream terminal
parsing, presentation, or historical replay. Raw PTY output, terminal state,
queued redraws, delivered prefixes, and client views remain distinct.

Fresh verification passes: the incremental build completed in 3.05 seconds;
the focused protocol/catalog suite passed eight tests; all 15 owned tmux 3.4
and 3.7 compatibility cases passed in 0.44 seconds; the complete mid gate
passed in 5.193 seconds; and strict Doxygen/compiler/language documentation
passed in 8.132 seconds. Restricted execution could not create tmux sockets or
spawn ripgrep from Node, so the stock and complete documentation results ran
outside that sandbox. The complete product goal and final full line audit
remain open.

## 2026-10-07: Layout ownership and terminal boundaries

`Window` now owns a recursive left-right or top-bottom pane layout. `SplitPane`
and `ListPanes` live with the pane operations and are exposed through the same
C++, Python, and Node catalogs. The pane component also registers
`split-window` and `splitw`; the transport loop contains no operation-specific
branch. Real tmux 3.4 and 3.7 clients pass horizontal and vertical splitting.

The `latest` size policy applies a client offer to the complete candidate
layout. It preserves recursive pane minima, distributes same-axis cells in
stable round-robin order, resizes every pane PTY and terminal, and invalidates
each client viewing the window. Ordinary-client composition still draws only
the active pane without borders, so the layout model is ahead of the current
presentation path.

The terminal review produced separate owners for retained state, client view
selection, and descriptor delivery. `ClientViewRefresh` composes and queues
current-state redraws. `ClientTerminalRouting` applies readiness and routes
active-pane input and output. `ClientTerminal` owns the outer descriptors,
bounded output, and terminal restoration. `PaneProgramInput` names the
program-facing queue. These boundaries keep queue admission, delivered bytes,
downstream terminal interpretation, and presentation as different facts.

The naming sweep also replaced the ambiguous compatibility surface with
`StockConnectionCompatibility` and `ReadStockClientCompatibility`, and the
native codec now uses `NativePayloadReader` and `NativePayloadWriter`. Live
source contains no `DomainError`, central domain dispatcher,
`StockClientConnection`, `ClientIo`, or unqualified `ProgramInput`.

A sanitizer-only terminal-restoration test was flaky because a full PTY can
briefly accept more bytes while the kernel moves data between internal queues.
The fixture now substitutes a nonblocking full pipe for the output descriptor,
making teardown backpressure deterministic. The focused sanitizer case passes
100 consecutive repetitions. Fresh complete gates pass: mid in 4.351 seconds,
analysis in 60.235 seconds, documentation in 91.725 seconds, sanitizers in
6.251 seconds, and fuzzing in 2.071 seconds. Analysis and documentation remain
inside the five-minute outer budget but exceed its 60-second stretch target.

The complete goal remains active. Multi-pane composition and borders, other
size policies, control mode, older named compatibility subsets, broader reverse
operations, subscriptions and recovery cursors, installed language packages,
alternate compiler/current-Ubuntu execution, and the final product audit remain
open.

## 2026-10-07: Multi-pane client view and selection

`ClientView` now consumes a copied `ClientWindowViewSnapshot` containing the
window revision, complete layout geometry, and every pane's terminal snapshot.
It validates identity, bounds, overlap, terminal dimensions, and one active
pane before producing bytes. The redraw places each pane at window coordinates,
derives basic ASCII separators from unoccupied layout cells, and translates
only the selected pane's cursor. Its queued baseline retains window and pane
revisions, so layout, selection, terminal, or viewport changes request a fresh
current-state view without claiming raw-output replay or presentation.

`SelectPane` is operation 20 under `pane/operations/`. C++, Python, and Node
use its typed native adapter. The pane component registers `select-pane` and
`selectp` through the command catalog, with exact `%pane` target parsing shared
with `split-window`. Selection mutates only the owning window, leaves repeat
selection revision-stable, and makes redraw and input routing observe the same
active pane through existing window state.

The focused native selection, registry, and view tests pass. The complete mid
gate passes in 4.255 seconds, including Python, Node Current/LTS, unmodified
tmux 3.4 and 3.7 clients, full two-pane redraw, stock `select-pane`, redraw
after native selection, and input reaching the newly selected pane. The first
documentation run rejected an undocumented private border predicate; after its
brief was added, the strict Doxygen/compiler/language/prose gate passed in
92.306 seconds. Capability-sensitive border glyphs and styles, overlays,
modes, copy mode, control mode, other size policies, and the remaining product
objective stay open.

## 2026-10-07: Protocol roles and named paste buffers

The accepted-client wire state is now `AcceptedClientProtocol`. It owns
identification, connection mode, command-output acknowledgement, attachment,
and detach transitions while `AcceptedConnection` owns the socket and queued
transport bytes and `Server` owns the corresponding tmux `Client`. The reverse
one-command state is `StockCommandExchange`; its shorter lifetime is no longer
hidden by a symmetric `Peer` name. The accepted connection's variant is
`protocol_state`, and its native alternative is `NativeRequestState`.

The terminal-model review supports the same cut. Terminal transport, tmux wire
state, tmux client attachment, retained pane state, per-client composition, and
presentation are distinct responsibilities. Zellij's typed edge routing,
Ghostty's composition and mailbox boundaries, WezTerm's mux/remote-ID split,
and zmx's per-client reassembly all reinforce the current owners. The accepted
protocol now uses a closed variant for identifying, awaiting-command,
command-output, attached, detaching, and closing phases. Descriptors, detected
profile, compatibility, output, and deadlines live only in phases that use
them; the former interacting phase booleans and optionals are gone.

The naming sweep replaced `service` with `route_ready_io`, generic registry
`Entry` records with `Registration`, paste-buffer `data` with `bytes`, and the
generic server `runtime.cpp` file with `daemon.cpp`. Context-qualified names
such as `Server::State`, operation-local `Request` and `Response`, and protocol
`Bytes` remain because their owners supply the missing meaning. The daemon has
no operation-specific dispatch branch. The process CLI now uses a validated,
frozen `ActionRegistry`; session-owned action files parse `create` and `list`,
invoke `ServerConnection`, and declare their usage. `main.cpp` retains process
role selection and daemon-startup syntax without naming either operation.
Focused tests first failed for the missing catalog and usage API, then passed
with duplicate, late, unknown, and malformed selection checks.

`PasteBuffer` is a server-global entity with typed set, read, delete, and
paste-into-pane operations in separate files. The server retains explicit
UTF-8 names and arbitrary bytes, with a 50-buffer and 256-KiB-per-buffer bound.
Paste retains the buffer, maps line feed to carriage return, and atomically
uses the pane's existing program-input queue. The stock registry exposes
`set-buffer`, `paste-buffer`, and `delete-buffer`; real tmux 3.4 and 3.7 clients
pass storage, exact-pane paste, native observation, and deletion.

Compatibility reports now disclose the implemented subset: explicit names,
exact `%pane` paste targets, and the name/count/byte bounds. Automatic buffers,
listing, delete-after, custom separators, raw paste, bracketed paste, and copy
mode remain unsupported. A focused policy assertion failed before these limits
were declared and passed afterward. The documentation gate also rejected one
undocumented paste-name predicate lambda before its minimal brief was added.

After the CLI catalog change, the focused action tests pass in 0.06 seconds,
the inner gate passes in 0.834 seconds, and the mid gate passes in 15.216
seconds. A final cached mid run passes in 5.139 seconds. The documentation
gate passes in 11.591 seconds, analysis in 80.247
seconds, sanitizers in 6.585 seconds, and fuzzing in 2.052 seconds.
The complete goal remains active: control mode, basic copy
mode, automatic buffers and listing, older named compatibility subsets,
subscriptions and recovery cursors, installed language packages, alternate
compiler/current-Ubuntu execution, and the final product audit remain open.

The final terminal fuzz pass found a chunk-sensitive libvterm character-set
case after the CLI work: `ESC`, malformed UTF-8, a DEC GL designation, and
later ASCII produced `r` as one write and U+23BC across three writes. The
eight-byte input now has a focused native regression and retained corpus seed.
The generated libvterm correction uses one streaming UTF-8 decoder, then maps
decoded ASCII through the active GL set. All 12 terminal-state tests pass, and
the rebuilt three-target sanitizer fuzz gate passes in 2.190 seconds.

The accepted-client phase regression first proved that a raw client could send
`new-session` after `attach-session` reached `MSG_READY`; the server created the
session and returned an exit frame. The closed variant rejects that transition
before command lookup. The focused Python suite now passes all 51 cases in
1.25 seconds while preserving stock attach, resize, detach, and command-output
behavior.

After the phase change, the inner gate passes in 0.200 seconds and the mid gate
in 4.413 seconds. The rebuilt sanitizer suite passes in 7.001 seconds, fuzzing
in 2.082 seconds, full C++ analysis in 68.224 seconds, and strict documentation
in 109.571 seconds.

## 2026-10-07: Guarded stock control subset

The first control-mode slice now accepts unmodified single-`-C` clients from
the tmux 3.4 and 3.7 fixtures. The packed startup command uses guard flag zero;
later newline commands use flag one. Matching `%begin` and `%end` or `%error`
records carry wall-clock time and one daemon-wide 32-bit command sequence.
Blank input drains guarded output before the socket exit message, allowing the
stock client to emit its own final `%exit`.

Control behavior has its own `protocol/tmux/control/` hierarchy.
`control::Channel` owns transferred descriptors, nonblocking flag restoration,
partial input, ordered output, and capacity bounds. `control::CommandSequence`,
`CommandOrigin`, and `CommandOutcome` model the guard contract without boolean
switches. The command-line subset and result formatting have separate files.
`AcceptedClientProtocol` retains only the surrounding closed phases: awaiting
the control startup command, active control mode, and draining final output.
Registered command behavior remains in entity-owned adapters behind the frozen
command registry.

The subset accepts one registered command per line with quotes and escapes. It
rejects groups before effects and declares its 64-KiB line/read allowance,
1000-argument limit, 128-command turn limit, 64-KiB write allowance, and 1-MiB
output backlog. Configuration grammar, asynchronous commands, attachment and
session switching, pane output, notifications, pause/continue, subscriptions,
and wait-exit remain unsupported.

Identification now accumulates client flags across validated frames. Single
`-C` is the only admitted control transport. Echo-disabled `-CC`, `no-output`,
`pause-after`, and `wait-exit` close during identification before the initial
command can run. Raw controlled negatives failed before the admission fix and
now pass. Unmodified tmux 3.4 and 3.7 `-CC` clients also close without creating
their requested session, while both single-`-C` clients retain their guarded
command behavior.

Exact stock-server probes showed that tmux 3.4 and 3.7 both close an unattached
control client after its startup command. This implementation deliberately
keeps the limited unattached channel open for later registered commands.
`unattached_control_channel_stays_open` now exposes that fidelity-changing
behavior through compatibility reports. Its policy test failed before the
declaration and passed afterward. A grouped `new-session` integration case
also proves rejection precedes server mutation.

| Gate | Result | Whole command |
|---|---|---:|
| Focused supported/rejected control cases | PASS; 8 cases | 0.30 s |
| Inner | PASS | 0.225 s |
| Mid native/language/type/format/generation suite | PASS | 4.668 s |
| ASan/UBSan/LSan suites and negative probes | PASS | 6.837 s |
| Three sanitizer fuzz targets, 2,000 runs each | PASS | 2.098 s |
| Clang-tidy and fatal negative probe | PASS | 69.325 s |
| Doxygen/compiler/language documentation | PASS; 734 callables, zero Doxygen warnings | 121.120 s |

The restricted documentation attempt reached the final Node source inventory
before `spawnSync rg` was denied. The exact gate passed outside that restriction.
The complete goal remains active: full control attachment, notifications and
pane-output continuity; older named support subsets; broader reverse
operations; copy mode and remaining multiplexer behavior; subscriptions and
recovery cursors; installed language packages; alternate compiler/current-
Ubuntu execution; and the final product audit remain open.

## 2026-10-08: Control attachment observations

The accepted inbound state remains `AcceptedClientProtocol`; there is no
`StockClientConnection`. `AcceptedConnection` owns the socket and queued
transport bytes, `AcceptedClientProtocol` owns the tmux wire phases, and
`Server` owns the corresponding `Client`. The reverse adapter remains
`StockServerConnection` because it owns endpoint policy, release evidence, and
fresh outbound command connections. These unequal roles do not share a
generic connection abstraction.

Committed session observation now uses typed `SessionCreatedEvent`,
`SessionRenamedEvent`, and `SessionClosedEvent` values. `ServerEventJournal`
owns a bounded sequence of committed records, while `ServerEventReader`
validates owner-qualified cursors. Create, effective rename, and removal append
events only after mutation commits; a same-name rename remains revision-stable
and emits no event. The journal retains 4,096 records. A lagging or wrong-server
cursor returns a typed error rather than silently skipping history.

Control delivery has two explicitly different baselines.
`SessionNotificationFeed` translates committed metadata events into
`%session-renamed` and `%sessions-changed` records. `SelectedSessionOutputFeed`
owns the attached session relationship and one raw-output cursor per linked
pane. Attachment and session switching establish current pane tails, so they
do not claim historical raw-output replay. Pane bytes preserve order within
each pane and control-channel enqueue order, without claiming reconstructed
cross-pane chronology. A metadata gap closes this limited control connection;
snapshot resynchronization, pause/continue, subscriptions, and the remaining
notification catalog are still unsupported.

The expanded real-client test initially failed for tmux 3.4 and 3.7 while
waiting for `%session-renamed`. After the committed-event path was added, both
versions passed attach and switch notices, selected-session output filtering,
rename and catalog notifications, and session-removal detachment while the
control channel remained usable. The guarded-command test then exposed a test
assumption: tmux correctly queued the asynchronous rename notification after
the command's `%end`, in the same read. The retained assertion now proves that
ordering explicitly.

The line audit replaced generic transition and observation names with
role-specific terms and retained operation behavior in entity-owned adapters.
Live source contains no `DomainError`, central domain dispatcher, or
`StockClientConnection`. Final gate evidence for this slice is recorded after
fresh verification below.

| Gate | Result | Whole command |
|---|---|---:|
| Mid native/language/type/format/generation suite | PASS | 4.692 s |
| ASan/UBSan/LSan suites and negative probes | PASS | 7.749 s |
| Three sanitizer fuzz targets | PASS | 2.136 s |
| Clang-tidy and fatal negative probe | PASS | 69.277 s |
| Doxygen/compiler/language documentation | PASS; zero Doxygen warnings | 8.679 s |

The complete goal remains active. Other control notifications,
pause/continue, subscriptions and snapshot recovery, copy mode, named older
release subsets, broader reverse operations, installed language packages,
alternate compiler/current-Ubuntu execution, and the final product audit
remain open.

### Native pull observation

`ObserveSessions` now returns one copied session catalog and the first event
after that atomic boundary. `ReadServerEvents` continues the cursor
immediately; `WaitServerEvents` uses an operation-owned continuation and a
1-to-750-millisecond deadline. Python releases the GIL for these operations,
and Node exposes the same C++ behavior through promises. Neither binding owns
event semantics.

The native event decoder now accepts the requested cursor as part of its
contract. It rejects a wrong cursor owner, a first record after the requested
position, a discontinuity, sequence exhaustion, or a following cursor that
does not exactly follow the returned records. Entity-local adapters also
compare observation cursors with the server identity pinned by the reply
envelope. A focused C++ test first failed to compile because the decoder lacked
the cursor contract. The malformed-reply suite then failed only the new
wrong-owner case, with 44 existing cases passing. Both negatives pass after
the boundary checks.

The sanitizer build exposed test-process startup as the reason the former
62-case Python CTest entry exceeded its unchanged five-second limit. The same
tests now run as server, protocol-compatibility, and tooling entries with the
same timeout; no case was removed and no limit was raised. The restricted
sandbox still rejects the owned socket-buffer fixture and LeakSanitizer
finalization under ptrace. Exact unrestricted gates pass.

| Gate | Result | Whole command |
|---|---|---:|
| Protocol compatibility suite | PASS; 45 cases | 1.96 s |
| Inner native suite | PASS; 70 cases | 0.343 s |
| Generated-interface drift | PASS | 0.852 s |
| Mid native/language/type/format/generation suite | PASS | 11.487 s |
| ASan/UBSan/LSan suites and negative probes | PASS | 12.081 s |
| Three sanitizer fuzz targets | PASS | 3.013 s |
| Clang-tidy and fatal negative probe | PASS | 145.356 s |
| Doxygen/compiler/language documentation | PASS; zero reported failures | 16.908 s |

This remains a bounded pull API. Callback subscriptions, cancellation of an
admitted wait, and automatic snapshot recovery after a journal gap remain
open.

## 2026-10-08: Release and package audit

The fresh release preset configures with locked Clang 23, builds, and passes
all 75 CTest entries in 3.09 seconds. A separate install prefix supplies an
external CMake consumer that creates a PTY-backed session, reads retained
terminal state, removes the session, and shuts down. That consumer passes with
locked Clang 23 and Ubuntu GCC 13. A stock Clang 18 invocation fails in the
public `std::expected` headers because it selects an older standard library;
the exported package does not yet diagnose that unsupported combination.

The Python wheel builds and installs into a fresh Python 3.14 environment with
no source import path. Its installed extension and packaged daemon pass a real
create, input, output, and removal flow. The source distribution is not
independently buildable: configuring a wheel from the archive fails because
the locked libvterm sources live under excluded `_build/` state. The 44 MiB
archive also contains 1,420 `node_modules` entries despite the intended
exclusion. These are release blockers; the passing wheel does not mask them.

The npm tarball contains only the CommonJS loader, declarations, addon, and
package metadata. The same packaged addon passes a real server flow on locked
Node 26.11 and Node 24.21. A normal fresh offline install still requests
`node-addon-api`, although the prebuilt addon does not load that package at
runtime. Dependency classification and the fresh-install gate remain open.

Current GCC 16.2 and current-Ubuntu execution remain unverified because the
declared project-local GCC compiler is not provisioned. Packaging fixes,
installed positive and negative type consumers, standalone source builds, and
the complete platform matrix remain required.

### Vocabulary follow-up

The second line audit found one substantive domain leak after the accepted
client rename. `ServerEvent*` currently contains only session create, rename,
and close facts, while `ServerEventReader::observe_sessions` crosses the
server/session boundary. It should become a session-owned event vocabulary
before subscriptions are designed. The same sweep found four smaller
placement/name corrections: stock-only `ClientLifecycle`, `pane/io.cpp`,
`pane/resize.cpp`, and POSIX descriptor/error helpers under `connection/`.
Native Unix-socket request I/O also belongs beside connections rather than
native framing. These are recorded as a proposal in `implementation.md`; no
production rename has been made in this review step.

## 2026-10-08: Session-owned observation vocabulary

Session observation now lives entirely in the session domain. `SessionEvent`,
`SessionEventCursor`, `SessionEventRecord`, `SessionEventBatch`,
`SessionEventJournal`, and `SessionEventReader` replace the generic server
names. `ReadSessionEvents` and `WaitSessionEvents` keep native operation IDs 26
and 27 while exposing `session.read_events` and `session.wait_events` through
C++, Python, and Node. The codec retains its numeric event tags and moves only
to session-specific names.

The compile-contract negative changed the native tests to the accepted paths
and symbols first. The build failed on absent
`session/session_event_journal.hpp` and
`protocol/native/session_event_codec.hpp`, then passed after the journal,
reader, codec, operations, registration, exposure schema, adapters, and
generated declarations moved together. A source sweep finds no old
`ServerEvent*` or server-event operation names outside historical progress and
the completed plan item.

The first mid rerun rejected Python export ordering. Ruff fixed the canonical
source export list. The next rerun reached the previously recorded ty failure:
one tooling-test literal inferred a nested invariant dictionary narrower than
`dict[str, object]`. Giving that fixture its intended annotation makes the
independent ty gate pass without changing bootstrap behavior.

| Gate | Result | Whole command |
|---|---|---:|
| Session-event compile-contract negative | PASS; failed on both absent accepted headers | 0.201 s |
| Native inner suite | PASS; 70 cases | 0.260 s |
| Default build with both bindings and regenerated interfaces | PASS | 19.324 s |
| Mid native/language/type/format/generation suite | PASS | 5.723 s |
| Doxygen/compiler/language documentation and controlled negatives | PASS | 115.476 s |

Evidence applies to parent HEAD `d6a84376f` with the pre-existing untracked
`cxx/` implementation and this uncommitted slice. The complete goal remains
active. The next ready vocabulary task renames stock-only `ClientLifecycle` to
`StockClientLifecycle`; pane filenames, the POSIX boundary, and native request
transport placement remain open after it.

## 2026-10-08: Concrete lifecycle and I/O boundaries

The remaining vocabulary corrections now describe their actual owners.
`StockClientLifecycle` translates accepted stock-client traffic into the
server-owned `Client` model. Pane implementation files name PTY servicing and
terminal resizing directly. Reusable descriptor ownership and errno capture
live under `platform/posix/`; `errno_error` snapshots errno before allocating
its contextual message.

`NativeRequestTransport` now derives the private native endpoint and owns one
request's connect, readiness, send, and receive work. `NativeServerChannel`
owns that transport while retaining negotiation and pinned-server identity.
The native protocol directory retains framing, payloads, handshakes, reply
envelopes, and the session-event codec; it no longer performs socket I/O.

Each placement change had a controlled compile-contract failure before its
implementation moved. The stock lifecycle build rejected the accepted header,
the pane build rejected both accepted filenames, the POSIX build rejected the
accepted ownership header, and CMake rejected the accepted native transport
source. Restoring each source under its intended owner made the same focused
build pass. A source audit finds no former production paths or unqualified
names outside historical progress and completed checklist text.

| Gate | Result | Whole command |
|---|---|---:|
| Stock-lifecycle compile-contract negative | PASS; missing accepted header | 0.496 s |
| Pane-filename configure negative | PASS; missing both accepted sources | 0.503 s |
| POSIX-boundary compile-contract negative | PASS; missing accepted header | < 0.1 s |
| Native-transport configure negative | PASS; missing accepted source | 0.69 s |
| Default build with Python and Node extensions | PASS | 0.87 s |
| Native inner suite | PASS; 70 cases | 0.234 s |
| Mid native/language/type/format/generation suite | PASS | 4.767 s |
| Doxygen/compiler/language documentation and controlled negatives | PASS | 113.461 s |

Evidence applies to parent HEAD `d6a84376f` with the pre-existing untracked
`cxx/` implementation and this uncommitted slice. The complete goal remains
active. All five prerequisite vocabulary corrections are complete; the next
ready work is the subscription, cancellation, and recovery contract.

## 2026-10-08: Scoped session event streams

`SessionEventStream` now owns one atomic `SessionObservation`, its advancing
cursor, unread records from the current batch, cancellation, and replacement
snapshot recovery. Closing a stream interrupts an admitted native receive
through `std::stop_token` without destroying persistent sessions. One stream
serializes cursor consumption and returns an explicit `SessionEventGap` before
continuing from the replacement observation.

Python exposes synchronous context-managed stream reads and releases the GIL
during each bounded wait. Its typed `AsyncSessionEventStream` adapter leaves the
event loop responsive, rejects overlapping reads, supplies async iteration and
context management, and closes its observer before propagating task
cancellation. Node performs native waits on an async worker, rejects
overlapping reads, exposes a pull-driven `AsyncIterable`, preserves an
`AbortSignal` reason, and supports asynchronous disposal. Its public boundary
rejects waits outside 1 through 750 milliseconds with `RangeError` before
crossing into C++; the focused test first received the native
`invalid_argument` error and then passed with facade validation. Generated
declarations cover the Node stream item variant, options, methods, and
subscription return type; ty checks the Python async facade. The Python package
keeps connection scheduling and session-stream scheduling in domain-named
modules. Node keeps its stream facade in `session_event_stream.cjs`; both
package entry points only compose the public surface.

| Gate | Result | Whole command |
|---|---|---:|
| Native admitted-receive cancellation | PASS | 0.03 s |
| Python sync/async session streams | PASS; 20 real-server cases | 0.89 s |
| Node Current and LTS real-server streams | PASS | 0.77 s |
| Mid native/language/type/format/generation suite | PASS | 5.249 s |
| Doxygen/compiler/language documentation and controlled negatives | PASS | 7.898 s |
| Clang-tidy and fatal negative probe | PASS | 75.564 s |
| ASan/UBSan native, Python, Node, and fault probes | PASS | 9.422 s |

The real-server binding tests cover atomic baselines, ordered delivery,
replacement-snapshot recovery after 4,097 mutations, continued delivery,
event-loop progress, async iteration, overlapping-read rejection, close, and
caller cancellation. Callback delivery, reconnection, resumable cursors, and
bounded push queues remain open. The complete goal remains active; no
parent-tree or publication change has occurred.

Fresh package evidence covers the new facades. The first isolated offline
wheel build failed after native compilation because stub generation invoked
Ruff without declaring it in the PEP 517 environment. Adding the already
locked Ruff version to `build-system.requires` made the same build pass. The
wheel contains `server_connection.py` and `session_event_stream.py`; a fresh
environment with isolated imports used the installed asyncio stream to create,
rename, observe, close, and remove a real session. An npm pack dry run contains
the CommonJS stream module beside the loader, declarations, addon, and package
metadata. Existing source-distribution and npm dependency-classification
blockers remain open.

## 2026-10-08: Self-contained language packages

The Python source distribution now owns every input required to rebuild its
wheel. It includes the locked upstream libvterm archive under `third_party/`;
CMake verifies the recorded SHA-256 digest and extracts the source into each
build tree. Python and Node binding switches are independent, while normal
development presets still build both. The Python package build disables only
the Node adapter and test targets that do not belong in its wheel.

Scikit-build-core's explicit inclusion mode lets the sdist retain the complete
authored source graph while removing `node_modules` and build state. Shared C++
binding briefs now have a language-neutral generator, so a Python build no
longer invokes `oxfmt` through the TypeScript generator. The prebuilt npm
artifact keeps `node-addon-api` as a development dependency and declares no
runtime packages.

The package contract first rejected the 46.2 MB source archive, its 1,420
`node_modules` entries, and its missing libvterm input. After the correction it
emits a 519,140-byte source archive with 324 members and no `node_modules`.
It rebuilt a wheel from the extracted sdist with package indexes disabled,
installed and imported that wheel in a fresh environment, packed the Node
addon, installed it from an empty offline npm cache, and loaded its native
`ServerConnection` export. Ty and TypeScript then accepted the positive
installed-package consumers and rejected invalid arguments, assignments, and
missing members with the required diagnostics.

| Gate | Result | Whole command |
|---|---|---:|
| Python sdist rebuild and fresh wheel import | PASS | included below |
| npm empty-cache install and native-addon load | PASS | included below |
| Installed Python and Node positive/negative types | PASS | included below |
| Combined offline package contract | PASS | 58.292 s |
| Mid native/language/generation suite | PASS | 5.310 s |
| Doxygen/compiler/language documentation | PASS | 121.725 s |
| ASan/UBSan/LSan suites and negative probes | PASS | 9.477 s |
| Ubuntu GCC 14.2 configure and full build | PASS | 58.68 s |
| Ubuntu GCC 14.2 configured tests | PASS; 76 cases | 4.03 s |

The GCC build exposed two signed/unsigned geometry comparisons in client-view
validation. A controlled `-Werror=sign-compare` build failed on both sites;
explicit conversion after the existing nonnegative bounds checks restored the
Clang and GCC builds. The warning is now a first-party build error.

Daemon discovery, current GCC, and current-Ubuntu verification remain open.
The complete goal remains active.

## 2026-10-08: Standalone source and install consumer

The standalone gate copies authored inputs without `_build`, `.venv`,
`node_modules`, or Python caches. It restores the locked Node development
dependencies from the offline project cache, configures the ordinary release
build with both bindings explicitly enabled, and compiles every configured
first-party production and binding unit from the copied tree. Installed CMake
metadata must not retain that copied source path.

The initial clean configure rejected the absent `node-addon-api` headers rather
than silently disabling Node. The first external consumer then failed to
compile because an old build artifact called the removed `Server::instance()`
spelling; after changing the source-controlled fixture to
`Server::server_instance()`, its stale exact operation-count assertion failed.
The final consumer checks the nonempty public catalog and exercises installed
session creation, terminal observation, session removal, and bounded shutdown.

A controlled consumer exit of 9 made the gate fail after installation and
adapter loading. Restoring its success exit made the same 33.675-second gate
build `tmux-cxx`, the nanobind extension, and the Node-API addon; install the
native libraries, executable, headers, libvterm license, and CMake package;
load both copied-tree adapters; and run the external installed consumer.
Daemon discovery remains a separate public lifecycle decision.

The first documentation run rejected the installed consumer because the
authored-source catalog included it but the development compilation database
did not. The catalog now names installed consumers explicitly. The compiler
inventory parses them with the project's configured C++23 flags, Doxygen
includes them, and clang-tidy analyzes them separately; their real build still
uses only the installed package. The documentation cache negative uses the
same record set, preventing the gate from silently dropping this boundary.

| Gate | Result | Wall time |
|---|---|---:|
| Standalone source, install, adapters, and native consumer | PASS | 33.675 s |
| Formatting | PASS | 0.396 s |
| Mid native/language/type/generation suite | PASS | 5.059 s |
| Documentation coverage and controlled negatives | PASS; 861 callables | 7.844 s |
| clang-tidy and fatal negative probe | PASS | 65.356 s |
| Offline installed package consumers | PASS | 34.382 s |

## 2026-10-08: Immutable runtime catalogs

Four extension boundaries now distinguish construction from runtime use.
`OperationRegistry`, `CommandRegistry`, `ActionRegistry`, and
`StockServerOperationRegistry` accept entity-owned registrations only during
startup. Their rvalue-qualified `freeze()` functions consume the validated
entries into `OperationCatalog`, `CommandCatalog`, `ActionCatalog`, and
`StockServerOperationCatalog`. Those catalogs have private construction and no
mutation API. The daemon, accepted-client protocol, native request executor,
CLI, and reverse stock-server adapter receive only the immutable types.

This removes the misleading frozen-registry state from runtime ownership while
preserving typed registration and one operation file per entity behavior.
Session, pane, window, window-link, client, and paste-buffer modules still own
their operations and command adapters. Startup composition verifies duplicate
names, aliases, public operation declarations, command dependencies, and the
nonempty reverse adapter set before any socket accepts traffic. No central
operation switch or generic domain dispatcher was introduced.

Each boundary first failed at compile time when its accepted `freeze()` result
changed from `Result<void>` to the corresponding catalog. The reverse adapter
test also failed until its builder moved into the dedicated registry header.
The focused catalog suite now covers duplicate and late registration, required
public declarations, missing command dependencies, typed execution, unknown
CLI selections, and entity-owned reverse registration.

The sanitizer and fuzz commands now rebuild the instrumented preset before
execution, preventing a source change from being checked against stale
binaries. The package check uses project-owned uv and npm caches and the locked
development environment while rebuilding its wheel from the emitted source
archive. It no longer writes to a user-level cache.

| Gate | Result | Wall time |
|---|---|---:|
| Focused Clang catalog/registry suite | PASS; 12 cases | 0.11 s |
| Mid native/language/type/format/generation suite | PASS | 4.875 s |
| Doxygen/compiler/language documentation and controlled negatives | PASS; 865 callables | 125.815 s |
| clang-tidy and fatal negative probe | PASS | 104.319 s |
| Python lint, format, and type contracts | PASS | 0.321 s |
| Node lint, format, and type contracts | PASS | 2.235 s |
| Deterministic generated-interface drift checks | PASS | 0.613 s |
| Offline installed package consumers | PASS | 30.611 s |
| Standalone source, install, adapters, and native consumer | PASS | 43.942 s |
| GCC configure and complete current-tree build | PASS | 17.291 s |
| Focused ASan/UBSan catalog/registry suite | PASS; 12 cases | 0.024 s |
| ASan/UBSan bounded fuzz replay | PASS; 6,000 total runs | 1.7 s |

The managed runner currently denies the `ptrace` operation LeakSanitizer needs,
Unix socket binding, and pipe-capacity changes. The strict sanitizer and fuzz
gates therefore remain unverified under LSan here. The current GCC test run
passed 65 of 71 native cases; the other six stopped at those denied syscalls,
and the daemon-backed Python and Node cases could not start. The strict gates
remain unchanged. Daemon executable discovery still awaits its public
lifecycle decision, and the complete goal remains active.

## 2026-10-08: Key tables before pane input

Interactive terminal bytes now pass through a tmux-shaped key-table boundary
before `SendPaneInput`. `ClientKeyInput` owns each client's active table and a
64-KiB pending-input bound, so a prefix remains meaningful when descriptor
reads split it from the following key. `KeyBindingRegistry` validates typed
entity-owned bindings and their native-operation dependencies during startup,
then produces an immutable `KeyTableCatalog` for the event loop. The first
binding lives with `DetachClient` and implements the default `C-b d` behavior;
the input loop contains no detach-specific branch.

The first compile failed because the binding file did not yet exist. The
completed PTY-backed test sends prefix and command in separate readiness turns,
proves root input reaches the pane, proves an unmatched prefix-table key does
not reach the pane, detaches the client, restores its terminal mode, and leaves
the session alive. A second pane-owned binding implements tmux's default
`C-b o`: it resolves the next pane from current layout order at execution, then
invokes `SelectPane`. A selection-change outcome makes terminal routing resolve
the current session, window, and pane again before consuming later bytes from
the same descriptor read. The first `C-b o` compile failed because key routing
did not carry `WindowId`; a later regression failed when same-read input went
to the old pane. The final PTY-backed case sends `C-b o z` together and proves
`z` reaches only the newly selected pane. Upstream `server-client.c` confirms
the implemented unmatched-key transition: retry the root table, reset the
client's table, and consume the key when lookup began in a non-root table.

The final line audit removed the single-field `KeyBindingOutcome` wrapper and
replaced the ambiguous `attached` result with `KeyBindingEffect` values that
name the selected-pane or client-lifecycle change. A focused root-table probe
now proves the documented fallback rather than only the final consume rule.
A controlled one-pane `C-b o` case first failed because the binding reported a
selection change even though `SelectPane` performed no mutation; it now reports
`selected_pane_unchanged`. Composition-root variables name their registry role,
and borrowed operation descriptions explicitly cover runtime catalog lookup.
The final vocabulary sweep calls the binding result an `effect` and names the
private shared-channel consumer as the session-event stream. Catalog adapters
now name action results, command outcomes, and native operation replies directly.

The default `C-b C-b` binding now invokes `SendPaneInput` for the pane selected
at execution. Its first backpressure test failed because routing reset the
client to `root` and surfaced an error. A backpressured binding now retains the
command key and original table for a later event-loop turn; the binding contract
requires that result before effects. Default `C-b %` and `C-b "` bindings invoke
`SplitPane` with left-right and top-bottom layouts. Their focused test first
observed one pane and an unchanged selection, then passed with two successive
typed splits and exactly one selected pane.

| Gate | Result | Wall time |
|---|---|---:|
| Focused Clang key-table and compatibility suite | PASS; 8 cases | 0.007 s |
| Focused GCC key-table and compatibility suite | PASS; 8 cases | 0.005 s |
| Focused ASan/UBSan key-table and compatibility suite | PASS; 8 cases | 0.045 s |
| Full native binary | PARTIAL; 72 of 78 cases | 0.054 s |
| Python lint, format, and type contracts | PASS | 0.288 s |
| Node lint, format, and type contracts | PASS | 2.016 s |
| Deterministic generated-interface drift checks | PASS | 0.587 s |
| Formatting | PASS | 0.476 s |
| Doxygen/compiler/language documentation | PASS; 899 callables | 153.439 s |
| clang-tidy and fatal negative probe | PASS | 99.527 s |
| Offline installed package consumers | PASS | 72.458 s |
| Standalone source, install, adapters, and native consumer | PASS | 88.795 s |
| Live Python, Node, and stock-client integration | ENVIRONMENT-UNVERIFIED; socket bind denied | 2.11 s |
| Strict sanitizer suite | ENVIRONMENT-UNVERIFIED; LSan `ptrace` denied | 0.27 s |
| Bounded fuzz replay | ENVIRONMENT-UNVERIFIED after 2,000 native-protocol runs | 0.47 s |

The six native failures remain the managed runner's Unix-socket and
pipe-capacity denials. The live Python, Node, and stock-client integration gate
is environment-unverified because neither daemon can bind its owned socket.
The strict sanitizer gate is environment-unverified because LeakSanitizer
cannot use `ptrace`; the focused ASan/UBSan execution used `detect_leaks=0`.
Complete key decoding, configurable key tables and bindings, repeat handling,
mode tables, and additional bindings remain open.

## 2026-10-08: Session-owned prefix options

The server now owns a global session `OptionTable`; each `Session` owns local
overrides that inherit from it. The immutable `OptionCatalog` describes
`prefix` and `prefix2`, their session scope, key value type, and tmux defaults.
It is catalog knowledge rather than mutable runtime registration. Compile-time
validation rejects empty or duplicate definitions and incompatible defaults.

`SetSessionOption` owns native identity 28 and the `session_options` capability.
Its request uses a closed `GlobalSessionOptions` or `SessionId` target.
Registered `set-option` and `set` command adapters accept exact `-g` and
`-t target` assignment forms. They resolve textual targets at execution, then
invoke the same operation. The native request retains global assignment and
adds an optional resolved session identity. The initial parser accepts `None`,
one ASCII byte, and `C-a` through `C-z`; it rejects broader key syntax before
mutation. `ClientTerminalRouting` resolves the current session's effective
prefix pair before routing each pending batch. `ClientKeyInput` only asks
whether a key enters the prefix table and contains no option-specific operation
branches.

The first test compile failed because `session/session_options.hpp` did not
exist. The completed test proves default inheritance, local override and
unset, effect-free invalid values and scope, primary and secondary prefix
routing, and pass-through of the replaced default prefix. The compatibility
test first failed against the previous fixed-prefix declaration; the policy
now reports the accepted key grammar and the still-fixed prefix-table bindings
separately.

The targeted command test then failed because the global-only parser rejected
`-t`. The passing test creates two sessions, assigns one inherited global
prefix, overrides only the named session, and proves missing targets and
invalid values preserve both effective tables. A native-protocol case proves
the same closed target alternatives and rejects a truncated session identity
before mutation.

The final sweep removed the option-name list from `SetOptionCommand`; it now
owns only global-or-targeted assignment syntax and leaves supported names and
values to the session `OptionCatalog`. `parse_single_byte_key` names its
limited grammar instead of implying complete tmux key parsing. Compile-time
`control_key('b')` expresses the default and send-prefix binding without
exposing numeric control encoding. Binding briefs name prefix-table keys, so
they remain accurate after changing the configured prefix.

The existing real-client matrix now has tmux 3.4 and 3.7 issue canonical
global and targeted `set-option` plus alias `set` assignments. Both pass
against the C++ daemon.
The resumed runner admitted the socket, pipe-capacity, and LeakSanitizer
operations denied by the earlier managed profile, so fresh current-tree
evidence replaces the prior environment-unverified results without weakening
any gate.

| Gate | Result | Wall time |
|---|---|---:|
| Focused Clang option and compatibility suite | PASS; 5 cases | 0.003 s |
| Focused GCC option and compatibility suite | PASS; 5 cases | 0.003 s |
| Real tmux 3.4 and 3.7 compatibility suite | PASS | 0.950 s |
| Full native binary | PASS; 82 cases | 0.037 s |
| Mid native, Python, Node, generation, and live integration suite | PASS | 5.625 s |
| Strict ASan/UBSan/LeakSanitizer suite | PASS | 28.657 s |
| Native protocol, tmux protocol, and terminal fuzz replay | PASS; 6,000 runs | 1.948 s |
| Doxygen/compiler/language documentation and controlled negatives | PASS | 139.902 s |
| clang-tidy and fatal negative probe | PASS | 73.613 s |
| Offline installed package consumers | PASS | 53.277 s |
| Standalone source, install, adapters, and native consumer | PASS | 45.020 s |
