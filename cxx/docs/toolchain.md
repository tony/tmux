# Toolchain support

Ubuntu 26.04.1 is the native development baseline. C++23 remains required;
the `cxx26` preset checks the same implementation in a newer language mode.
Normal builds consume locks and provisioned files. Release resolution and
installation are explicit bootstrap actions.

## Locked tools

| Tool | Version | Release source |
| --- | --- | --- |
| Clang, clang-format, clang-tidy, clangd, LLD, LLDB, compiler-rt | 23.1.3 | [LLVM release](https://github.com/llvm/llvm-project/releases/tag/llvmorg-23.1.3) |
| CMake | 4.4.4 | [CMake release](https://github.com/Kitware/CMake/releases/tag/v4.4.4) |
| Ninja | 1.13.2 | [Ninja release](https://github.com/ninja-build/ninja/releases/tag/v1.13.2) |
| Doxygen | 1.18.0 | [Doxygen release](https://github.com/doxygen/doxygen/releases/tag/Release_1_18_0) |
| GCC | 15.2.0-16ubuntu1 | [Ubuntu package](https://packages.ubuntu.com/resolute/g%2B%2B-15) |
| GoogleTest | 1.18.0 | [GoogleTest release](https://github.com/google/googletest/releases/tag/v1.18.0) |
| just | 1.58.0 | [just release](https://github.com/casey/just/releases/tag/1.58.0) |
| Graphviz | 16.1.0 | [Graphviz release](https://gitlab.com/graphviz/graphviz/-/releases/16.1.0) |
| CPython | 3.15.0 | [Python release](https://www.python.org/downloads/release/python-3150/) |
| uv | 0.13.0 | [uv release](https://github.com/astral-sh/uv/releases/tag/0.13.0) |
| Ruff | 0.17.0 | [Ruff release](https://github.com/astral-sh/ruff/releases/tag/0.17.0) |
| ty | 0.0.86 | [ty release](https://github.com/astral-sh/ty/releases/tag/0.0.86) |
| Node Current / LTS | 26.11.1 / 24.21.0 | [Node distributions](https://nodejs.org/dist/) |
| npm | 12.2.0 | [npm release](https://github.com/npm/cli/releases/tag/v12.2.0) |
| oxlint / oxfmt / Oxc parser | 1.88.0 / 0.73.0 / 0.154.0 | [Oxc releases](https://github.com/oxc-project/oxc/releases) |
| oxlint-tsgolint | 7.0.2004 | [Registry release](https://www.npmjs.com/package/oxlint-tsgolint/v/7.0.2004) |
| TypeScript / Vitest | 7.0.2 / 5.0.4 | [TypeScript releases](https://github.com/microsoft/TypeScript/releases), [Vitest releases](https://github.com/vitest-dev/vitest/releases) |

The native lock records distributions, digests, and fixture provenance.
`uv.lock` and the Node package lock record their complete dependency graphs.
Ubuntu's default Clang and CMake packages predate the locked upstream
releases. GCC uses the final Ubuntu 26.04 distribution. Upstream GCC 16.2
is a candidate; its local build and verification remain open.

## Supported facilities

Configuration probes the selected standard library for `std::expected`
including monadic operations, `std::span`, `std::to_underlying`, and
`std::move_only_function`. The implementation uses explicit results, borrowed
byte spans, typed enum conversions, and move-only continuations. Host
libstdc++ supplies those facilities; the bundled libc++ cannot supply the
required move-only callable. That libc++ configuration fails explicitly.

CMake uses preset schema 12 and target-scoped `LINKER_TYPE` selection.
Clang-tidy's selected bugprone checks include the stable Clang 23 additions.
Its analyzer diagnostics remain fatal. Doxygen and the compiler-derived
inventory require brief callable contracts and reject malformed, duplicated,
missing, or oversized documentation through controlled negative cases.

Node-API 10 is the current stable ABI supported by both locked Node runtimes.
Its changed thread-safe callback exception handling applies when that API is
used. New addon work must retain JavaScript-thread ownership and explicit
exception translation. Experimental Node APIs remain outside this baseline.

## Support age and upgrades

Python requires 3.13 or newer. Python 3.12 was released in October 2023 and
has passed the requested three-year support window. Tooling targets the
declared minimum; development uses current stable CPython, with Ubuntu's
Python checked separately.

Support branches for tools and language runtimes older than three years may
be retired after checking their callers, fixtures, and published requirements.
An old release number alone does not establish that a maintained dependency
is obsolete. Keep necessary external libraries and declared protocol fixtures
until their replacement has equivalent verified behavior. tmux 3.4 and 3.7
remain the real compatibility fixtures for their documented subsets.

Upgrades keep compiler tools and parser bindings at one LLVM release,
preserve release checksums, refresh language locks, and reconfigure affected
CMake presets with `--fresh`. Python provisioning verifies uv's version and
selects the locked interpreter even when a workspace already exists. The
LLVM tools run with the host runtime; they require no legacy ICU package or
project-wide library-path override.

Run the configured native, binding, type, generation, formatting, analysis,
documentation, sanitizer, fuzz, package, and standalone checks after an
upgrade. Record measured evidence and unresolved compatibility gates in
[progress](progress.md).
