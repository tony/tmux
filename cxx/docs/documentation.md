# Function documentation and quality gates

Document each first-party callable with one concise contract. Cover public and private functions, helpers, constructors/destructors, overloads, operators, templates, callbacks, and lambdas. Keep the information callers or maintainers need to use the function correctly.

These are implementation requirements. Writing this policy does not implement or pass its checks.

## Canonical documentation

Put C++ documentation at the canonical declaration; document local/inline definitions at their definition. Use an explicit `@brief` sentence. Give each authored callback/lambda an adjacent brief or a resolved reference to its owning contract. Deduplicate declarations/definitions and template instantiations in coverage; compiler-generated implicit functions and third-party sources are outside the authored-function set.

Generate native adapter, Python docstring/stub, and Node JSDoc/declaration text from the authoritative API descriptions where generation owns those functions. Document hand-written Python/JS/TS functions in their language's normal form. Tests and tooling follow the same brief-contract requirement. Name test contracts by the behavior they assert.

Use existing language tooling or a small syntax-tree check for Python/JS/TS callables omitted by doc-presence rules. Validate generated interfaces against their canonical contracts. Hidden private/local functions must not count as documented through exclusion.

Use `@copydoc` or `@copybrief` for the same contract when the reference resolves to the intended overload. Document changed semantics on an override. A declaration and its definition must not carry two hand-maintained copies of the same text.

The compiler inventory resolves copied contracts before checking parameters and budgets. Controlled probes cover missing, ambiguous, and signature-incompatible copies. Separate executable and fuzz entrypoints stay distinct despite Clang's shared symbol identities; regressions prove each remains covered. The compiled exposure file remains inside coverage despite its build-directory location. Generated contents and expanded compiler response-file arguments invalidate the inventory cache. Generators normalize wrapped Doxygen briefs before producing Python docstrings and Node JSDoc.

## Content and word budgets

| Documentation | Default budget |
|---|---|
| Brief | One sentence, at most 25 prose words |
| Whole function contract | At most 100 prose words, excluding markup and referenced identifiers |
| Ordinary function | Aim for a brief and zero to two short contract lines |
| Longer shared invariant/protocol explanation | One named component/topic page, linked from functions |

These are project policy limits, not Doxygen requirements. A necessary exception identifies one symbol and its reason, lives in a small reviewed policy list, and must remain visible to the check. Blanket exemptions and silent truncation are prohibited. Preserve necessary ownership, failure, concurrency, ordering, security, and compatibility facts when tightening text.

The checker enforces two additional contract lines. Type-only JSDoc annotations supply JavaScript signatures and do not count as prose detail lines. There are no approved budget exceptions. Controlled C++, Python, and Node syntax-tree fixtures prove omissions, excess text, filler, and callable coverage failures.

Add details when the signature cannot express units/bounds, ownership or borrowing, invalidation, effects, recoverable errors, caller obligations, thread/GIL/JS-thread requirements, cancellation, partial effects, or release quirks.

Use `@param`, `@tparam`, and `@return` when they add a caller-relevant fact. Keep parameter tags complete when used, or express a narrow constraint in prose. Do not generate filler descriptions that repeat names, types, defaults, or obvious return values. Describe `std::expected` failures as error results; use exception tags only for exceptions the function can throw. Distinguish checked invalid input from an actual caller precondition.

Let Doxygen render concepts, requires clauses, attributes, and signatures. Document semantics those declarations do not carry. Validate overloads, constrained templates, operators, explicit-object members if used, callbacks, and `std::expected` results against the compiled C++23 API.

## Doxygen baseline and configuration

Doxygen 1.18.0 is the latest stable release verified on 2026-10-07. Resolve and lock the newest reachable stable release during explicit tool updates; normal builds use that exact pin. A fallback needs an observed compatibility or availability constraint. [Release](https://github.com/doxygen/doxygen/releases/tag/Release_1_18_0), [official downloads and checksums](https://doxygen.nl/download.html).

| Coverage configuration | Value |
|---|---|
| `EXTRACT_ALL` | `NO` |
| `EXTRACT_PRIVATE`, `EXTRACT_STATIC`, `EXTRACT_ANON_NSPACES` | `YES` |
| `WARNINGS`, `WARN_IF_UNDOCUMENTED`, `WARN_IF_DOC_ERROR` | `YES` |
| `WARN_IF_INCOMPLETE_DOC` | `YES` |
| `WARN_NO_PARAMDOC` | `NO` |
| `WARN_AS_ERROR` | `FAIL_ON_WARNINGS` |
| `GENERATE_HTML`, `GENERATE_XML` | `YES` |

This keeps missing-member diagnostics, validates tags, and avoids forcing redundant parameter/return blocks. Separate public navigation from the complete internal coverage inventory. Scope inputs to first-party code and keep generated documentation under `_build/docs/`.

Reference: [Doxygen configuration](https://doxygen.nl/manual/config.html). Keep configuration and its negative tests version-locked.

Use the real compilation database and build include/define settings. Enable Clang-assisted parsing only in a pinned Doxygen build that provides compatible libclang support. A compiler-derived inventory must supplement XML where Doxygen omits authored callables. Check semantic identities/overloads and source ownership, not a regex count of function-like strings. Cache unchanged inventory work to respect the test budgets. Reuse compiler tooling and the existing check entry point; do not create a general documentation framework.

The callable inventory loads libclang and its Python bindings from the same locked LLVM release. Updating LLVM must refresh both. Graphviz 16.1.0 is provisioned locally for diagrams; automatic API graphs remain disabled to keep generated pages focused.

## Stop-slop and bloat safeguards

Use [stop-slop at revision 8da1f030185bdfe8471220585162991eaeb970e9](https://github.com/hardikpandya/stop-slop/blob/8da1f030185bdfe8471220585162991eaeb970e9/SKILL.md) as the prose review policy. It is a writing skill, not an executable linter.

Write direct contracts with concrete tmux nouns. Cut introductions, praise, hedging, repeated signatures, rhetorical contrasts, implementation narration, and development history. Keep rationale in commits and durable shared invariants in component documentation. Keep required limitations and caller obligations.

Enforce deterministic checks for empty briefs, word-budget violations, duplicate contract blocks, unresolved copy references, malformed tags, and selected filler phrases from a small explicit rule list. Report file/symbol/line and exit nonzero. Review semantic accuracy and necessity separately; a pattern scan cannot prove them.

Apply prose checks to authored comments, docstrings, JSDoc, descriptions, and generator inputs/outputs. Protect code, protocol literals, public identifiers, citations, quoted source, and licenses from mechanical prose rewriting. The `just` command must not become a banned-word finding. Each suppression needs a narrow location and reason; reject unused or blanket suppressions. Do not auto-rewrite code or change behavior as part of a docs check.

## Language tools

| Language | Required checks |
|---|---|
| C++ | clang-format check, clang-tidy, native GoogleTest/CTest suite, Doxygen coverage/contract checks |
| Python | Ruff lint, Ruff format check, ty type checking, pytest against the compiled/installed extension |
| Node JS/TS | oxlint, oxfmt check, tsgolint through `oxlint-tsgolint` type-aware integration, compiler diagnostics, Vitest against the addon |

Run Python tools from the locked uv workspace. Verify positive and negative installed-package consumers with ty; do not suppress diagnostics to make invalid fixtures pass. Run intentionally invalid fixtures in a harness that expects their specific failures.

Lock a compatible oxlint, oxfmt, oxlint-tsgolint, TypeScript, and Vitest set. Type-aware linting and compiler type checking are separate requirements. The current Oxc integration supports type checking through `--type-check`; retain an explicit compatible compiler gate unless the selected locked integration passes the required consumer/negative fixtures. Enable unused-suppression diagnostics. Scope lint/type inputs so they do not traverse builds, dependencies, or unrelated parent projects.

References: [Ruff lint](https://docs.astral.sh/ruff/linter/), [Ruff formatting](https://docs.astral.sh/ruff/formatter/), [ty rules](https://docs.astral.sh/ty/rules/), [Oxc type-aware integration](https://oxc.rs/docs/guide/usage/linter/type-aware.html), [Oxfmt](https://oxc.rs/docs/guide/usage/formatter.html).

## Justfile and native suite

Add a small `cxx/justfile` with a self-listing default and grouped build/test/lint/docs/check recipes. Anchor execution inside `cxx/`; use a strict shell and explicit scopes. Keep checks read-only and formatting/fixes in separate recipes.

Delegate to CMake/Ninja/CTest, locked uv tools, package scripts, and thin `support/check.py` entry points. Define each gate once. Keep justfile recipes, CMake targets, and check scripts from growing independent build/check graphs. Inner/mid recipes must not hide installs, reconfiguration, full builds, or network work in their dependencies.

Use one pinned GoogleTest suite under `tests/cpp/`, orchestrated by CTest, matching the existing C++ port's pattern. GoogleTest v1.18.0 was the latest stable release verified on 2026-10-05; resolve/lock it during bootstrap. Keep test dependencies out of the installed production interface. [GoogleTest CMake guide](https://google.github.io/googletest/quickstart-cmake.html), [release](https://github.com/google/googletest/releases/tag/v1.18.0).

Test real critical behavior: identities, linked windows, registration, errors, terminal state, framing, descriptors, lifecycle, and continuations. Isolated interoperability remains real-binary evidence. Test discovery must not start servers or run setup with external effects. Fail empty discovery. Avoid repeated fixtures and tests that restate implementation.

The isolated `sanitizer_probe` executable is a test tool with deliberate memory, arithmetic, and leak defects. It is never installed or run by ordinary CTest discovery. Function coverage and formatting include it; clang-tidy excludes this named negative input. The sanitizer gate requires its specific fatal diagnostics. Leak suppressions match only independently reproduced Node OpenSSL initialization and the external Rolldown DSO; the probe proves first-party leaks remain fatal.

LibFuzzer targets share production codecs and terminal ownership. Normal development builds compile their documented hooks; the sanitizer preset supplies instrumented drivers. `just fuzz` runs outside inner/mid loops, with 4096-byte inputs, a one-second per-input timeout, a 512-MiB memory bound, and 2000 iterations per target. Source seeds remain read-only, temporary generated corpora are removed, and failure inputs are retained. The targets check bounded native round trips, stock headers/profile evidence/argument decoding, and terminal meaning/replies across fragments. They do not establish full stock-role, descriptor-lifecycle, or terminal conformance.

## Prove the safeguards

Use controlled-negative fixtures to prove failures for a removed brief, missing private/static/lambda documentation, wrong parameter/reference, unresolved copied overload, oversized contract, filler phrase, and unused/blanket suppression. Include a positive brief-only function to prove that complete coverage does not force empty parameter boilerplate.

Prove scope with excluded third-party/implicit functions and included generated first-party wrappers. Verify modern C++23 symbols survive extraction. Test malformed Python/Node types and ensure type-aware linting cannot stand in for compiler diagnostics.

Place full Doxygen generation, compiler-inventory refresh, native compilation, and installed consumers in the outer loop unless measured evidence fits a smaller loop. Keep focused cached prose/coverage checks in inner/mid loops. Record whole-command time and actual outcomes; this policy is not evidence that its gates already exist.
