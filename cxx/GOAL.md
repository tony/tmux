# Independent C++23 tmux implementation goal

Status: active implementation. The user activated the complete supplied objective on 2026-10-05. This acceptance contract retains the original implementation scope and the approved architecture, naming, documentation, tooling, and compatibility requirements.

Implement an independently buildable C++23-or-newer tmux implementation
entirely inside `cxx/`. Deliver a working terminal multiplexer, compatibility
with explicitly tested stock tmux clients, and typed native tmux-control bindings
for Python through nanobind and Node.js through Node-API.

This is an implementation goal, not a scaffolding exercise. A successful
configure, a renamed source tree, an importable extension, or mocked binding
tests do not constitute completion. Follow `cxx/AGENTS.md` throughout.

## Repository boundary

Treat the existing tmux tree outside `cxx/` as read-only. Do not change its C
sources, headers, build system, configuration, tests, documentation, formatter
rules, ignore files, or CI. Do not move existing files into `cxx/`.

All first-party native implementation in `cxx/`, including both binding
adapters and generated native code, must compile as C++. Use `.cpp` and `.hpp`
for new native files. Do not compile original tmux `.c` files as C++, include
original implementation files, link original tmux object files, or depend on
headers or generated configuration from the parent tree.

Port copies of upstream implementation into `cxx/` where appropriate. Preserve
licenses and attribution, and record the source revision and original paths.
Preserve behavior when porting, while giving the C++ implementation concrete entity and operation boundaries.
The resulting production build must not require the original source tree.

External C libraries and their C ABI headers, including libevent and terminal
libraries, are allowed dependencies. Python tooling/tests, JavaScript or
TypeScript loaders/tests, CMake files, schemas, and documentation are also
allowed. They must not become alternative implementations of the engine.

Keep generated files, build trees, environments, packages, caches, and
project-local tools under `cxx/`. Tests may use owned temporary runtime
directories. Never use or terminate the user's existing tmux server.

## Naming and entity architecture

Put aesthetic clarity and maintainability first: directories, files, types, methods, errors, and metadata should name the tmux concepts they represent. Use a tmux-shaped C++ model rather than a generic application framework. Preserve upstream behavior when translating it, while separating mechanical porting from structural or semantic changes.

Use `Server`, `Session`, `Window`, `WindowLink`, `Pane`, and `Client` as distinct concepts. The server owns persistent catalogs and runtime coordination; entity components own their invariants. Windows can be linked into several sessions. A `WindowLink` carries session-local membership/index information; a window must not have one exclusive parent session. Add concrete `SessionGroup`, `Environment`, `OptionTable`, `KeyTable`, `PasteBuffer`, `Job`, and copy-mode concepts when needed.

Place implementation under `src/{server,session,window,window_link,pane,client}/` and matching public headers under `include/tmux_cxx/`. Put each operation in its entity's `operations/` directory, for example `session/operations/rename_session.cpp` and `pane/operations/split_pane.cpp`. Give typed declarations accompanying headers as needed. Keep the implementation, request/result meaning, validation, and registration description together.

Use snake_case directories/files, PascalCase C++ types, snake_case native functions and Python methods, and camelCase Node methods. Use entity-specific operation names such as `RenameSession`, `SplitPane`, and `AttachClient`. Name the native controller façade `ServerConnection`; reserve `Client` for the tmux client entity. Reserve `Connection` for a logical endpoint or accepted transport lifetime. Name the accepted-client wire state `AcceptedClientProtocol` and the one-command reverse state `StockCommandExchange`; do not hide their unequal roles behind a symmetric `Peer` name. Distinguish handle types from materialized values such as `SessionSnapshot`. Use `TmuxError` with structured, meaningful codes and context; do not export `DomainError` or lose error classifications in translation.

Keep terminal parsing/state under `terminal/`, registered command behavior/targets/queues under `commands/`, native connection lifetimes under `connection/`, and wire behavior under `protocol/{native,tmux}/`. Keep the tmux control channel, command-line grammar, guarded results, notifications, and pane-output encoding under `protocol/tmux/control/`. Split build targets only at useful dependency boundaries; directory structure alone does not require another library.

Use strong identities and explicit ownership, value snapshots, RAII, `std::expected<T, TmuxError>`, constrained registration contracts, and compile-time descriptor validation where applicable. Use `std::move_only_function` for genuinely move-owned continuations and typed variants at closed wire/state boundaries. Borrow byte spans only within valid lifetimes. Use readiness-driven continuations or coroutines with explicit cancellation where useful. Do not introduce unnecessary inheritance, pervasive shared ownership, raw owning pointers, or generic frameworks without demonstrated reuse. Prevent C++ exceptions from escaping C callback boundaries. Resource cleanup must not block indefinitely waiting for child exit.

## Registered operations and command execution

Each operation owns a typed request/result and registration description. The description records exposed native API identity, entity/action name, aliases/options where applicable, required capabilities, effect classification, and the execution entry point. Validate duplicates, schema mappings, and capability references. Preserve stable wire identities when the contract promises them.

Each entity explicitly registers its operations. Startup consumes each validated registry into a distinct immutable catalog. Runtime lookup and invocation receive only the catalog; they must not contain a hardcoded branch for every operation. Adding an operation changes its local implementation/description and explicit build/exposure inputs, without operation-specific edits to transports, the event loop, or language-side semantics. Keep type erasure confined to a narrow lookup boundary and preserve typed identities and results throughout execution.

Implement command parsing, target resolution, command queues, hooks, grouping/error ordering, and waiting continuations separately from catalog lookup. Preserve their supported tmux semantics. Resolve targets at the proper execution point. One waiting client must not block all clients. Native API IDs, stock commands, CLI actions, and interactive actions resolve to the same C++ operations without duplicating engine behavior. Keep daemon startup parsing in `main.cpp`; declare CLI operation adapters through a typed registry and immutable catalog rather than extending an action-specific branch there.

## Current toolchain, reproducible builds

At implementation time, inspect `/etc/os-release`, the architecture, installed
tools, and refreshed Ubuntu package candidates. Verify the newest final Ubuntu
release separately from development images. Use the host's current Ubuntu
release and establish a reproducible test environment on current Ubuntu.

Resolve the latest stable upstream versions that work on the target, with the
latest available stable Ubuntu packages as the minimum acceptable fallback.
Do not copy old version pins from examples or mistake a nightly for a release.
A fallback requires a concrete incompatibility or availability explanation;
do not silently weaken the C++23 requirement.

Resolve and record:

- CMake, CTest, CPack, Ninja, just, Doxygen, Graphviz, and GoogleTest.
- A coherent LLVM suite: Clang/clang++, clang-format, clang-tidy, clangd,
  LLD, LLDB, compiler-rt, llvm-cov, and llvm-profdata.
- The selected C++ standard library and runtime; current GCC for a second
  compiler check.
- Stable CPython, uv, nanobind, scikit-build-core, pytest, Ruff, and ty.
- Node.js Current and the active LTS line, npm, node-addon-api, TypeScript,
  Vitest, oxlint, oxfmt, and oxlint-tsgolint (tsgolint). Select a
  non-experimental Node-API level supported by both
  tested Node lines.
- Required native dependencies and parser/build generators actually used.

Prefer verified upstream distributions, correctly selected LLVM release
packages, or pinned source builds when Ubuntu packages lag. Do not assume a
version-suffixed package is a stable release. Keep installations project-local
or in the test container where practical; do not replace system defaults.

Resolve latest versions during explicit bootstrap or dependency updates, then
pin them. Ordinary builds and tests must not contact a floating `latest`,
rewrite lockfiles, or silently upgrade tools. Record exact versions, sources,
hashes or immutable revisions, Ubuntu identity, and standard-library choice
in `cxx/support/toolchain.lock.toml` and generated build reports. Keep Python
and npm dependency graphs in their own lockfiles rather than duplicating them.
Pin or constrain isolated Python build dependencies too.

Use C++23 as the required baseline, with standard extensions disabled. Set
`CXX_STANDARD_REQUIRED` and target compile features explicitly. Compile-probe
required library facilities. Allow a separate C++26-or-newer preset without
making experimental features mandatory for the C++23 build.

## Build and layout

Use a standalone, target-oriented CMake project with Ninja. The first-party
project declares `LANGUAGES CXX`. Keep include paths, warnings, dependencies,
visibility, PIC, and sanitizer options target-scoped. Explicitly list native
sources. Do not introduce a second authoritative native build in node-gyp.

Use these boundaries, adding files only when implementation needs them:

| Path inside `cxx/` | Responsibility |
|---|---|
| `AGENTS.md`, `GOAL.md`, `README.md` | Rules, acceptance contract, durable usage/status |
| `CMakeLists.txt`, `CMakePresets.json`, `cmake/` | Authoritative target-scoped native build |
| `justfile` | Self-listing, grouped recipes delegating to the authoritative tools |
| `.clang-format`, `.clang-tidy`, `.clangd`, `.gitignore` | Explicit project-scoped tooling |
| `include/tmux_cxx/`, `src/` | Named tmux entities, operations, terminal, commands, connections, protocols |
| `api/` | Small public exposure contract compiled against typed operation descriptions |
| `bindings/python/`, `bindings/node/` | Required native language adapters and package inputs |
| `pyproject.toml`, `uv.lock` | Locked Python build/development workspace |
| `tests/cpp/`, `tests/compat/`, `tests/integration/` | Native, real-client compatibility, and binding acceptance |
| `docs/` | Usage, ownership, errors, compatibility, and API documentation |
| `support/bootstrap.py`, `support/check.py`, `support/toolchain.lock.toml` | Thin bootstrap/check entry points and locked tooling |
| `support/codegen/{python,node}/`, `support/codegen/common/` | Language generation; share only reused machinery |
| `_build/` | Builds, generated output, tools, packages, caches, and reports |

Provide development, release, ASan/UBSan, and GCC presets with separate build
directories. Generate the compilation database inside `cxx/`; configure
clangd to find it without a repository-root symlink or editor configuration.
Formatting and analysis must not inherit incompatible parent-tree settings.

Build a reusable native core, a shared C++ server connection, a separately named `tmux-cxx`
executable, the nanobind extension, and the Node-API addon. Both bindings are
required in the default development and verification configurations. Missing
dependencies must fail clearly, not silently turn a binding off. An explicitly
requested core-only build is not evidence that the full goal passes.

Package adapters may invoke CMake, but must not duplicate its source lists or
compiler configuration. Select a consistent C++ runtime within each build;
validate an alternate standard-library preset independently.

## Prove both bindings in the first functional milestone

Before expanding the feature surface, prove an end-to-end operation through
C++, pytest, and Vitest against the actual C++ server:

1. Start an isolated server built from `cxx/` and create a real PTY-backed
   session and pane.
2. Use the Python extension to mutate that session through the typed tmux API.
3. Use the Node addon to observe and mutate the same server state.
4. Use an unmodified, supported stock tmux client to inspect that state.
5. Send input, observe output, and exercise a real TmuxError.
6. Generate both languages' types and type-check small consumers.
7. Close the language clients and confirm the server/session remains alive.

Run these checks continuously from that milestone onward. Import-only tests,
mock engines, canned replies, and delegation to a stock tmux server do not
satisfy this requirement.

## One authoritative native tmux implementation

Use one authoritative C++ server state and shared registered operations for CLI, interactive actions, stock-client compatibility, and language bindings. Bindings and protocol adapters must not bypass validation, hooks, layout updates, or events, or independently implement tmux behavior.

Provide typed server, session, window, window-link, pane, and client identities; value snapshots; structured `TmuxError` results; capabilities; and scoped subscriptions. Preserve linked-window semantics.

Expose handles resolved against live server state, not mutable implementation pointers. Distinguish server-instance identity, entity lifetime/generation, state revision, and observation cursor. Reject stale or wrong-server handles with a defined error. Garbage collection of a handle must not kill its entity, and connection disposal must not destroy persistent sessions.

Use daemon-backed native tmux control by default. Compile the same C++ `ServerConnection` into both extensions. Communicate with the C++ server using a separately versioned, bounded, framed local native API protocol with explicit version/capability negotiation and server identity. Keep this channel separate from stock tmux IPC and its identification/version rules. Do not implement ordinary binding methods by formatting stock command strings, parsing CLI output, or launching the existing tmux executable.

Explicit startup may launch the new C++ daemon. Module import, type generation, and object finalization must not create sessions or run shell commands. Embedded-engine support is not required.

Maintain separate program/PTY lifetime, persistent terminal parser/state, raw-output retention, and client-specific view/delivery state. Route keys through modes/key tables before program input encoding. Preserve terminal query/reply ordering. Keep client viewport, window layout geometry, pane/program size, and pixel/cell metrics explicit, including multiple-client size arbitration.

Define snapshot consistency, subscription establishment without a missed-event window, bounded queues, and explicit resynchronization after a gap. Separate metadata events, raw pane bytes, interpreted terminal snapshots, and composed client views. Give each representation its own owner-qualified baseline and gaps. A metadata/screen snapshot does not recover discarded raw bytes. Reconnect or redraw must not promise lossless replay without evidence.

Specify cancellation, timeouts, partial failure, effects, and retry behavior. Cancelling a request cannot reverse executed operations, spawned processes, or delivered PTY bytes. Do not retry an uncertain mutation while reconnecting or selecting another protocol profile. Deliver native/language callbacks outside mutation and locking regions, with explicit disposal/shutdown rules.

## Working tmux behavior and protocol compatibility

Select and document concrete upstream releases and client modes for a declared baseline. The existing design names tmux 3.4 as the first target; preserve or explicitly revise that choice through source and real-binary evidence. A target or matching protocol number is not accepted compatibility. Define additional older-release general support as a smaller, named, tested operation/mode subset; do not promise complete upstream parity for older versions.

Provide adapters for both directions: unmodified stock clients connecting to the C++ server, and the C++ client connecting to declared stock servers. Share typed codecs/profile descriptions while keeping connection state machines role-specific. Native bindings continue to exercise the authoritative C++ server by default; stock-server testing must not substitute for the shared-server binding milestone.

Keep transport, wire state, and tmux client attachment distinct. `AcceptedConnection` owns the socket, queued bytes, delivered prefix, and closure point. `AcceptedClientProtocol` owns one accepted client's identification and later wire phases. `Server` owns the corresponding `Client`. `StockCommandExchange` owns only one reverse command exchange. Represent identification, one-shot command/output, attached-terminal, control-mode, draining, detaching, and transport-closing phases as a closed C++ state variant rather than a cross-product of booleans.

Declare `TmuxProtocolProfile` with its protocol number/check rule, framing and descriptor contract, message catalog, payload layouts and ABI requirements, identification sequence, control-mode grammar, and codec references. Declare `TmuxCompatibilityPolicy` with exact releases or justified spans, direction, support tier, operation/mode limits, permitted adaptations, and conformance evidence. Expose `StockConnectionCompatibility` with selected profile, release/evidence, uncertainty, effective support, active quirks, and limits through C++, Python, and Node.

Implement `StockProfileDetector` using observable framing/identification evidence, bounded candidate state, and explicit unknown/ambiguous results. Protocol 8 spans incompatible release details, including the 3.6 header/descriptor change, so its number alone cannot identify a dialect. For ambiguous remote endpoints, use trusted endpoint metadata, a declared profile, or a documented conservative fallback. Do not guess capabilities or trial newly introduced messages against an unknown endpoint. Verify a stock server release with `#{version}` after a compatible connection where available; a local executable's `tmux -V` is not server-release evidence. Revalidate connection assumptions after reconnect.

Keep terminal capabilities distinct from server/API capabilities. Tmux identification feature bits describe the client terminal and have release-sensitive positions. Stock version checking is not native API capability negotiation. Honor selected host byte order, payload widths, descriptor transfer, framing bounds, and profile-specific version masking. Do not reuse the native little-endian codec for host-endian stock messages without an explicit contract.

Track supported, adapted, limited, unsupported, and unknown capability states independently from support tiers and test evidence. Baseline support must deliver working interactive behavior; general older support must enumerate its smaller guarantees. Recognized/unverified profiles must remain marked unverified. Reject known unsupported operations before their effects and preserve unrelated work.

Keep required wire rules in codecs. Declare named quirks/adjustments with applicability, source/rationale, observable consequences, and tests. Apply only policy-permitted safe adaptations; expose changed fidelity, output gaps, and unknown capabilities. Strict behavior must reject an incompatible fidelity-changing fallback when requested. Never fabricate success, silently ignore restrictions, claim rollback of effects, or advertise unimplemented terminal capability.

Implement identification, descriptors, command responses, attach/detach, resize, exit, and supported control-mode framing/notifications for each declared baseline. Parse command-output blocks and asynchronous notifications statefully, preserve unknown notifications, and use the selected guard/output grammar. Declare older grammar, missing-notification, pause/continue, and buffering limitations explicitly.

Use a dedicated socket and the separately named `tmux-cxx` executable. Do not replace the system tmux binary or assume a stock client's auto-start path launches this implementation.

Deliver persistent sessions, windows and linked windows, panes, shell/process lifecycle, splits, selection, resizing, layouts, terminal input/output, terminal-state restoration, scrollback, copy/paste, basic copy mode, and multiple clients. Preserve essential supported commands, configuration, key bindings, formats, hooks, and command ordering. Core lifecycle, correct PTY handling, and usable interactive terminal behavior are required; completion cannot be reduced to a controller-server demonstration.

Use differential tests against pinned upstream releases for supported behavior and profile boundaries. Build references only in disposable owned fixtures; their C objects/executables must never become production dependencies. Test real unmodified client/server binaries and both connection directions. Include controlled-negative cases for invalid/ambiguous/mismatched profiles, malformed traffic, descriptor ownership, and non-destructive unsupported operations.

## Python: nanobind, pytest, and distributed typing

Build the extension through nanobind and CMake, with scikit-build-core wheel
and source-distribution packaging. Keep packaging inside `cxx/` and use a
project-local uv environment/workspace with one authoritative uv lockfile.

Expose idiomatic Python names, real typed values, explicit exceptions, context
management, and synchronous tmux operations. Provide asynchronous operations
and subscriptions with specified event-loop delivery and cancellation.
Release the GIL around blocking native work where appropriate; reacquire it
before accessing Python objects. Do not call Python while mutating server
state or after interpreter teardown.

Run Ruff lint and Ruff format checks, and use ty for strict type checking, from the locked uv workspace. Scope inputs to first-party Python tooling, loaders, tests, and generated package interfaces. Configure docstring-presence checks without boilerplate parameter/type repetition. Verify positive installed-package consumers and run intentionally invalid consumers in a harness that expects specific ty failures. Fail warnings and unused/broad suppressions according to the scoped policy; do not replace ty or weaken checks to hide unsupported typing.

Use nanobind's stub generator, preferably its CMake integration, against the
built extension. Ship accurate `.pyi` files and `py.typed` in the wheel.
Generate signatures for arguments, defaults, overloads, return values, enums,
context managers, asynchronous operations, and events. Do not erase the API to
`Any` merely to get type checks passing.

Run pytest against the real compiled module. Test an installed wheel in a
fresh environment with no source-tree import fallback, and prove the sdist
contains the native sources and can build without the parent tmux tree.
Run strict positive and negative type fixtures against the installed package.
Test stable CPython and the Ubuntu-provided version where distinct. Do not
claim free-threaded or stable-ABI support without dedicated evidence.

## Node.js: Node-API, Vitest, and declarations

Use Node-API through `node-addon-api`, not NAN or direct V8 APIs. Build the
`.node` addon through the same CMake project. Keep `package.json`, its lockfile,
loaders, TypeScript configuration, and tests under `cxx/bindings/node/`.

Return promises for operations that can wait. Keep identities and materialized
snapshot data synchronous. Deliver events on the correct JavaScript thread,
with bounded queues, explicit disposal, and environment-cleanup handling.
Do not block the JavaScript event loop or expose unprotected native pointers.
Do not represent integers beyond JavaScript's exact-number range as `number`.

Generate `.d.ts` declarations and useful JSDoc from the authoritative binding
contract. Define package exports and type resolution for every module format
actually shipped. Do not claim unsupported ESM/CommonJS entry points.

Require oxlint linting, oxfmt formatting checks, and tsgolint through the pinned `oxlint-tsgolint` type-aware integration. Lock a compatible TypeScript/Oxc/Vitest set and explicit package inputs. Enable unused-suppression diagnostics. Type-aware linting does not replace compiler type checking: require compiler diagnostics through the proven locked `--type-check` integration or a compatible explicit compiler gate. Verify positive and negative installed-package consumers, and document hand-written functions and generated JSDoc from the canonical contract.

Run Vitest runtime tests against the actual addon and keep lint, format, type-aware lint, and compiler checks independently runnable. A runtime test run does not replace type verification.

Create an npm tarball and install it into a fresh consumer fixture. Verify its
addon, loader, declarations, native dependencies, and daemon discovery without
source-tree paths. Test Node Current and active LTS; verify a single addon
artifact across both where the selected Node-API/OS/architecture contract
claims compatibility. Do not describe this as universal C++ ABI stability.

## Code generation and autocompletion

Keep language-specific generators in `cxx/support/codegen/python/` and
`cxx/support/codegen/node/`; share only reused machinery in
`cxx/support/codegen/common/`.

Use a small explicit public API/binding exposure description under `cxx/api/`, with
compile-validated mappings to entity-owned typed operation descriptions and
registration metadata. Keep behavior authoritative in those C++ operations. Generate or validate the
adapters and declarations from it. Python stub generation must also inspect
the built nanobind module. Prevent independently handwritten Python and
TypeScript contracts from drifting apart. Do not build a general-purpose
binding framework or assume C++23 offers automatic static reflection.

Use `uvx` for pinned packaged CLI tools. Use `uv run --locked` in the local
workspace for project scripts and generators that import project dependencies.
`uvx` tool environments must not be assumed to inherit the workspace lock.
Do not use global pip installs. Dependency-heavy CI generation belongs in the
locked workspace rather than an unconstrained temporary tool environment.

Give generators explicit inputs and outputs, stable ordering, write-if-changed
behavior, and correct CMake dependencies. Write build outputs under `_build/`.
For committed generated interfaces, offer an explicit update and a read-only
check mode that detects missing, changed, and unexpected files. Never hand-edit
generated output to fix a generator defect.

Prove completion and navigation through clangd's compilation database, Python
stubs, and TypeScript declarations. Verify inferred return types and members,
not merely that declaration files exist. Test a missing export or incorrect
signature to prove each verification gate can fail.

## Brief function documentation and prose safeguards

Document every authored first-party callable: public/private functions, helpers, constructors/destructors, overloads, operators, templates, callbacks/lambdas, bindings, tests, and tooling. Use one canonical C++ declaration or adjacent local-definition contract, and normal Python docstrings/JS JSDoc. Generated functions obtain their documentation from the authoritative API contract. Deduplicate declarations/definitions and instantiations; compiler-generated implicit functions and third-party code are outside the authored set.

Use one brief sentence of at most 25 prose words, marked `@brief` in C++. Limit a function contract to 100 prose words by default. Add only caller-relevant information absent from the signature: units/bounds, ownership, borrowing/invalidation, effects, errors, caller obligations, thread/GIL/JS-thread rules, cancellation/partial effects, or protocol quirks. Link a shared invariant rather than duplicating it. A necessary budget exception must identify one symbol and a reviewed reason. Preserve required contract facts; do not truncate documentation to pass a limit.

Use parameter/template/return tags where they add information; keep tags complete and valid when used. Avoid descriptions that repeat names, types, or defaults. Document `std::expected` failures as results, and exception tags only for actual throws. Reuse identical contracts with resolved `@copydoc`/`@copybrief`, including the intended overload; document changes on overrides. Give callbacks/lambdas an adjacent brief or resolved owning-contract reference.

Use current stable Doxygen, resolve and lock it during bootstrap, and record provenance. Doxygen 1.18.0 was verified as current stable on 2026-10-07. Build HTML and XML under `_build/docs/`; use real build settings and a verified compatible Clang integration where available. Prove C++23 extraction for the API features in use, including constrained templates, overloads, operators, attributes, and explicit-object members if present.

The complete coverage configuration requires `EXTRACT_ALL=NO`, private/static/anonymous-namespace extraction, `WARNINGS=YES`, missing-member/documentation-error/incomplete-tag warnings, `WARN_NO_PARAMDOC=NO`, and `WARN_AS_ERROR=FAIL_ON_WARNINGS`. Keep public navigation separate from internal coverage. Supplement XML with a narrowly scoped compiler-derived authored-callable inventory where Doxygen omits functions. Reuse existing compiler/check tooling and cache unchanged work. Empty or hidden inventories must fail, not imply coverage.

Apply the pinned stop-slop prose policy at revision `8da1f030185bdfe8471220585162991eaeb970e9`. Add deterministic checks for missing/empty briefs, word-budget violations, duplicate contracts, malformed or unresolved references/tags, selected filler phrases, and unused or blanket suppressions. Scope checks to prose while protecting code, identifiers, protocol literals, quotations, and licenses. Review semantic accuracy and necessity separately. Stop-slop provides writing guidance; do not invent a `stop-slop` executable or treat a phrase scan as complete review.

Prove the docs/coverage/slop checks with controlled-negative fixtures for missing public/private/static/lambda briefs, incorrect parameters, unresolved copied overloads, oversized text, filler phrases, and suppressions. Include a positive brief-only case, generated first-party functions, and excluded implicit/third-party functions. Put full generation and uncached compiler inventory in the outer loop; focused cached checks must respect inner/mid budgets. See `docs/documentation.md` for the detailed policy and primary sources.

## Justfile and native C++ test suite

Provide a small `cxx/justfile` with a self-listing default and grouped build/test/lint/docs/check recipes, matching the established project conventions. Anchor commands within `cxx/`, use a strict shell, and separate read-only checks from formatting/fixes. Delegate to CMake/Ninja/CTest, locked uv tools, package scripts, and thin `support/check.py`. Define each gate once; do not create another build system or duplicate check graphs. Inner/mid recipes must not hide installs, network work, reconfiguration, or full builds.

Use one pinned GoogleTest suite under `tests/cpp/`, orchestrated by CTest, following the existing native C++ port's pattern. GoogleTest v1.18.0 was verified as current stable on 2026-10-05; resolve and lock the implementation-time stable version. Keep test dependencies out of installed production targets, prevent discovery from spawning servers or producing external effects, fail empty discovery, and time whole commands. Test critical behavior and controlled negatives without redundant fixtures, implementation-mirroring tests, or a second unit framework. Real interoperability and both binding suites remain required.

## Quality, documentation, and packaging gates

Require a line-by-line architectural/name review of every first-party source, binding, API description, generated interface, test helper, and user-facing example introduced for this work. Replace generic names where concrete tmux meaning improves comprehension. Review responsibility boundaries, ownership, duplicated contracts, operation registration, error fidelity, and unnecessary abstractions. Preserve invariant comments and useful reused technical helpers. Do not declare this sweep complete from a filename-only search.

Enforce whole-command test budgets: inner under 5 seconds (stretch under 2), mid under 30 seconds (stretch under 10), and outer under 5 minutes (stretch under 60 seconds); libraries aim for stretch budgets. Keep network, installs, production builds, browser launches, broad corpus scans, and sleeps out of inner/mid loops. Treat waits over one second as structural issues to investigate and replace with readiness/subscriptions where the contract permits. Tag slow tests with a reason and keep them in the outer loop. Fix overruns rather than raising budgets. Benchmarks are separate, with each run under ten minutes and a full sweep under one hour.

Require clean first-party builds with `-Wall -Wextra -Wpedantic`; promote
selected warnings to errors in verification. Run clang-format checks and
clang-tidy with the real compilation database. Keep suppression narrow and
justified; exclude third-party dependencies without excluding our adapters.

Coordinate integrated gates through CTest while keeping native, Python, and
Node suites independently runnable. Add ASan/UBSan checks for core behavior
and binding lifetimes with correct sanitizer-runtime setup. Fuzz untrusted
protocol/parser boundaries using LLVM tooling and retain regression inputs.
Do not count an uninstrumented addon as sanitizer-tested.

Cover linked windows, deletion by another client, stale handles, child exit,
PTY EOF, reconnection, subscriber backpressure, malformed requests, omitted
features, Python/Node shutdown, and resource cleanup. Use deadlines and
observable readiness instead of arbitrary sleeps. No blanket retries,
placeholder passes, zero-test success, or silent skipping of required suites.

Build Doxygen HTML and XML with the brief-contract coverage, word budgets,
warning policy, and stop-slop safeguards specified above. Document ownership,
errors, threading, lifetimes, and compatibility limits when they affect callers.
Keep all generated docs under `_build/`.
Use normal headers unless a separately proven modules benefit justifies more
build complexity.

Provide native install/export checks from an external consumer, wheel/sdist
checks, and npm package checks. Document how language packages discover the
separately installed or explicitly supplied `tmux-cxx` daemon. No source-tree
path assumptions or unsafe implicit server replacement.

Implement `cxx/support/bootstrap.py` and `cxx/support/check.py` as thin entry
points over the actual tools, not a new build system. The check entry point
must offer focused checks and a full verification run with nonzero failure.
Record the commands, selected versions, suite outcomes, and artifact paths.
Keep automation implementation inside `cxx/`; wiring root CI is out of scope.

## Completion evidence

From a fresh checkout and provisioned current Ubuntu environment, demonstrate:

- Independent configure/build/install of `cxx/`, including both bindings,
  without reading parent-tree tmux implementation files.
- Working interactive sessions with supported stock clients and concurrent
  C++, Python, and Node native control over the same C++ server.
- Passing declared C++-client-to-stock-server profiles and older-release
  general-support subsets, with honest limits and connection reports.
- Entity-owned registered operations and a completed line-by-line naming,
  ownership, error, and responsibility review.
- Passing GoogleTest/CTest, pytest, Vitest, Ruff, ty, oxlint, oxfmt,
  tsgolint/type-aware and compiler checks, generation/drift, native
  formatting/analysis, sanitizer, Doxygen coverage/bloat, prose-policy,
  documentation, and installed-package checks.
- Repeated generation and no-op builds without unexplained output changes.
- Explicit, non-destructive behavior for every declared unsupported feature.
- No modifications or generated debris outside `cxx/` in the repository;
  preserve any changes that predated the task.

Keep commits small, coherent, and independently buildable once the initial
build exists. Follow the tmuxp-derived title and `why:`/`what:` convention in
`cxx/AGENTS.md`. Separate mechanical porting from semantic changes. Report
actual verification and remaining failures; never weaken the gates to declare
completion.
