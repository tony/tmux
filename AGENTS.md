# AGENTS.md

`cxx/` is the independent C++23-or-newer tmux implementation. The surrounding
repository remains normal upstream-style C tmux. Neither implementation may
acquire a build or source dependency on the other.

## Scope and isolation

These instructions apply to `cxx/` and its descendants. They do not impose C++
conventions on the rest of tmux or override applicable ancestor instructions.

All first-party native production and binding code here is C++. External C
libraries are permitted. Python tooling/tests, TypeScript or JavaScript
loaders/tests, build files, schemas, and documentation are explicit exceptions
to the native-language rule, not permission to move engine logic out of C++.

Do not modify, move, format, regenerate, or stage files outside `cxx/` for this
work. In particular, leave root C sources, headers, Autotools files, CI, ignore
rules, and editor settings unchanged. Do not create root build directories or
a root compilation-database symlink.

Do not include parent-tree tmux headers or implementation files, compile the
original `.c` files, link original tmux objects, or invoke the old executable
as the new implementation. A read-only upstream source reference and an
isolated upstream test fixture are allowed; neither is a production dependency.

Keep derived implementation and attribution inside `cxx/`. Keep builds,
packages, generated output, local environments, and tools here too. Configure
formatters, analyzers, test discovery, and generators with explicit scope.
Do not let parent configuration silently control this project.

## Working method

Read `GOAL.md`, `README.md`, and the relevant code before changing behavior.
Use the documented bootstrap and check entry points once implemented; do not
invent successful command results or describe missing tooling as present.

Preserve pre-existing user changes. Record the starting Git state and inspect
both staged and unstaged diffs before each commit. Stage only named files
belonging to the current change. Verify the task introduced no changes or
untracked build debris outside `cxx/`; do not revert unrelated work to obtain
a clean-looking tree.

Make the smallest coherent change. Reuse existing files, tests, helpers, and
APIs before adding another. Keep mechanical translation, formatting,
dependency updates, and behavioral changes in separate commits where practical.
Use `rg` and `fd` for targeted inspection rather than dumping the repository.

## C++ and build discipline

Use required C++23, standard extensions off, and target-scoped CMake settings.
CMake owns the native build; Ninja executes it. An optional newer-standard
preset must not make the baseline silently depend on experimental features.

Prefer value types, strong identities, RAII, explicit ownership, and readable
control flow. Use `std::expected` or equivalent explicit results for recoverable
domain errors. Prevent C++ exceptions from escaping C callback boundaries.
Avoid unnecessary inheritance, pervasive shared ownership, raw owning
pointers, and generic frameworks without demonstrated reuse.

Do not publish internal entity pointers through a binding. Resolve handles
against current server state. Preserve tmux's linked-window relationships.
Dropping a wrapper or connection must not destroy a persistent session.

Resolve current stable tooling explicitly and lock the result. Normal builds
must not upgrade dependencies or fetch floating branches. Keep compiler,
standard-library, sanitizer, and addon runtime choices consistent and tested.
Do not disable warnings globally to accommodate a local defect.

## Bindings and generated interfaces

Nanobind and Node-API are required products, not deferred integrations. Keep
both enabled in normal development and verification. Maintain genuine pytest
and Vitest coverage of the built extensions against the C++ server.

Keep domain behavior in the shared C++ implementation. Binding adapters may
translate types, exceptions, futures, and lifetimes, but must not independently
implement server semantics or shell out to tmux for ordinary domain methods.

Use the GIL and JavaScript-thread rules correctly. Deliver callbacks outside
server mutation/locking regions. Specify disposal, shutdown, cancellation, and
bounded buffering. Never advertise rollback of effects that already occurred.

Keep generators under `support/codegen/{python,node}/`, with shared code only
where reused. Use pinned `uvx` tools or the locked uv workspace as appropriate;
do not assume `uvx` consumes `uv.lock`. No global pip installation.

Treat generated `.pyi`, `py.typed`, `.d.ts`, and API metadata as tested package
contents. Change the generator or authoritative API description, not generated
output by hand. Generation must be deterministic and support drift checks.
Do not replace useful types with `Any`, `any`, or broad casts to hide failures.

## Compatibility and verification

Preserve the explicitly documented stock-client and control-mode baselines.
Test real client binaries, not just protocol version integers. Optional
features may be rejected or degraded only as documented and without destructive
side effects. Do not claim full upstream parity from a passing subset.

Every behavior change needs a focused test. Prove a new gate can fail with a
controlled negative case or temporary deliberate break, then remove the break.
A skipped suite, import-only check, mock, or empty test run is not integration
evidence. Do not hide failures with blanket retries or disabled assertions.

Run relevant native, pytest, Vitest, type, generation, formatting, analysis,
and documentation checks. Run the full configured gates and installed-package
checks before claiming completion. Report unrun checks and actual blockers.
Keep tests on owned server sockets and clean up only their own processes.

## Commits

Follow tmuxp's scoped title convention:

```text
Scope(type[detail]): concise description

why: Explain the necessity or impact.

what:
- State the specific technical changes
- Keep the commit focused on one topic
```

Keep the subject at most 50 characters, excluding a trailing `(#NN)` reference.
Wrap body lines at 72 characters. Separate `why:` and `what:` with a blank line.
Use meaningful scopes and lowercase types such as `feat`, `fix`, `refactor`,
`docs`, `test`, `style`, `chore`, or `ci`.

Behavior-changing subjects retain the colon. Routine maintenance drops the
colon and uses a capitalized description. Examples:

```text
Pane(fix[close]): Reject stale handles
cxx(deps[dev]) Update LLVM tools
ai(rules[AGENTS]) Define the C++ boundary
```

An implementation commit should explain why the change is necessary, not
merely narrate its diff. Include relevant verification evidence. Do not squash
unrelated work into a single omnibus change. Do not push, tag, publish, rewrite
shared history, or amend another person's commit without authorization.

Convention source: tmuxp's `AGENTS.md` delegates writing and commit policy to
[`.github/WRITING.md` at the referenced revision][tmuxp-writing].

[tmuxp-writing]: https://github.com/tmux-python/tmuxp/blob/fa1bd94/.github/WRITING.md#commits

## Documentation

Describe current behavior, ownership, failure, and compatibility limits.
Keep plans in `GOAL.md`, historical rationale in commits, and durable usage in
`README.md` and API documentation. Do not narrate signatures or obvious code.
Preserve comments that explain invariants or otherwise surprising constraints.
No invented performance claims, fictional examples, or claims of checks that
were not run.
