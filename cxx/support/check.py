"""Run scoped checks without installing tools or building production targets."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import time
from pathlib import Path
from tempfile import TemporaryDirectory

from native_sources import installed_consumer_sources, native_sources

ROOT = Path(__file__).resolve().parents[1]
PYTHON = ROOT / ".venv/bin/python"
NODE = ROOT / "_build/tools/node_current/bin/node"
NPM = ROOT / "_build/tools/npm/bin/npm-cli.js"
ENVIRONMENT = dict(
    os.environ,
    PATH=str(NODE.parent) + os.pathsep + os.environ["PATH"],
    PYTHONPATH=str(ROOT / "_build/dev/python"),
    LD_LIBRARY_PATH=str(ROOT / "_build/tools/llvm_runtime/usr/lib/x86_64-linux-gnu")
    + os.pathsep
    + str(ROOT / "_build/tools/llvm/lib")
    + os.pathsep
    + os.environ.get("LD_LIBRARY_PATH", ""),
)
DEADLINE = float("inf")


def run(
    arguments: list[str],
    *,
    cwd: Path = ROOT,
    diagnostics: tuple[str, ...] = (),
    report: Path | None = None,
) -> None:
    """Require the expected exit status and diagnostics; save requested output."""
    remaining = DEADLINE - time.perf_counter()
    if remaining <= 0:
        raise RuntimeError("the complete check command exhausted its time budget")
    completed = subprocess.run(
        arguments,
        cwd=cwd,
        env=ENVIRONMENT,
        text=True,
        capture_output=True,
        timeout=remaining,
    )
    output = completed.stdout + completed.stderr
    if report is not None:
        report.parent.mkdir(parents=True, exist_ok=True)
        report.write_text(output)
    expected = 1 if diagnostics else 0
    if completed.returncode != expected or any(item not in output for item in diagnostics):
        print(output, end="")
        raise RuntimeError(f"check failed: {' '.join(arguments)}")


def formatting(*, fix: bool = False) -> None:
    """Check authored C++ formatting or apply an explicitly requested format pass."""
    flags = ["-i"] if fix else ["--dry-run", "--Werror"]
    run(
        [
            str(ROOT / "_build/tools/llvm/bin/clang-format"),
            "--style=file:.clang-format",
            *flags,
            *(str(path) for path in native_sources()),
        ]
    )


def documentation() -> None:
    """Verify brief coverage and controlled failures across C++, Python, and Node."""
    run([str(PYTHON), "support/check_documentation.py"])
    run([str(PYTHON), "support/documentation_probes.py"])


def native() -> None:
    """Run discovered native tests and reject an unexpectedly empty suite."""
    run([str(ROOT / ".venv/bin/ctest"), "--preset", "dev", "--no-tests=error", "-L", "inner"])


def analysis() -> None:
    """Check compiled authored C++ units and prove analyzer diagnostics remain fatal."""
    expected = {path.resolve() for path in native_sources() if path.suffix == ".cpp"}
    # This isolated negative executable intentionally leaks and accesses outside an allocation.
    expected.remove(ROOT / "tests/cpp/sanitizer_probe.cpp")
    # Installed consumers are analyzed below but compile against the installed package elsewhere.
    expected.difference_update(path.resolve() for path in installed_consumer_sources())
    expected.add(ROOT / "_build/dev/generated/exposure.cpp")
    records = json.loads((ROOT / "_build/dev/compile_commands.json").read_text())
    compiled = {Path(record["file"]).resolve() for record in records}
    if not expected or expected - compiled:
        raise RuntimeError("native analysis requires every authored C++ translation unit")
    probe = ROOT / "_build/probes/analysis/rejected.cpp"
    probe.parent.mkdir(parents=True, exist_ok=True)
    probe.write_text(
        "/** @brief Exercise fatal division diagnostics in the configured analyzer. */\n"
        "int main() { int divisor = 0; return 10 / divisor; }\n"
    )
    run(
        [
            str(ROOT / "_build/tools/llvm/bin/clang-tidy"),
            "--quiet",
            "--config-file=.clang-tidy",
            str(probe),
            "--",
            "-std=c++23",
        ],
        diagnostics=("clang-analyzer-core.DivideZero",),
        report=ROOT / "_build/reports/analysis-negative.log",
    )
    run(
        [
            str(PYTHON),
            "_build/tools/llvm/bin/run-clang-tidy",
            "-clang-tidy-binary",
            str(ROOT / "_build/tools/llvm/bin/clang-tidy"),
            "-quiet",
            "-config-file",
            str(ROOT / ".clang-tidy"),
            "-p",
            "_build/dev",
            "-j",
            "4",
            *(str(path) for path in sorted(expected)),
        ],
        report=ROOT / "_build/reports/analysis.log",
    )
    for source in installed_consumer_sources():
        run(
            [
                str(ROOT / "_build/tools/llvm/bin/clang-tidy"),
                "--quiet",
                "--config-file=.clang-tidy",
                str(source),
                "--",
                "-std=c++23",
                f"-I{ROOT / 'include'}",
            ]
        )


def python() -> None:
    """Require Ruff lint, format, ty positives, and three invalid-consumer diagnostics."""
    scopes = ["support", "tests", "bindings/python"]
    run([str(PYTHON), "-m", "ruff", "check", *scopes])
    run([str(PYTHON), "-m", "ruff", "format", "--check", *scopes])
    run([str(ROOT / ".venv/bin/ty"), "check"])
    run(
        [
            str(ROOT / ".venv/bin/ty"),
            "check",
            "tests/integration/types_negative.py",
            "--output-format",
            "concise",
        ],
        diagnostics=(
            "error[invalid-argument-type]",
            "error[invalid-assignment]",
            "error[unresolved-attribute]",
        ),
    )


def node() -> None:
    """Keep Oxc type-aware lint and compiler type checks independently required."""
    for script in ("lint", "format-check", "lint-type-aware", "type-check"):
        run([str(NODE), str(NPM), "--prefix", "bindings/node", "run", script])
    run(
        [
            str(NODE),
            "node_modules/typescript/bin/tsc",
            "--ignoreConfig",
            "--noEmit",
            "--strict",
            "--module",
            "NodeNext",
            "--target",
            "ES2022",
            "--types",
            "node",
            "tests/types-negative.cts",
        ],
        cwd=ROOT / "bindings/node",
        diagnostics=("TS2345", "TS2322", "TS2339"),
    )


def generation() -> None:
    """Reject missing or changed declarations, compiled exposure, and Python stubs."""
    run(
        [
            str(PYTHON),
            "support/codegen/native_contract.py",
            "--check",
            "--output",
            "_build/dev/generated/exposure.cpp",
        ]
    )
    run(
        [
            str(PYTHON),
            "support/codegen/binding_documentation.py",
            "--check",
            "--output",
            "_build/dev/generated/binding_documentation.hpp",
        ]
    )
    run([str(PYTHON), "support/codegen/node/generate.py", "--check"])
    run(
        [
            str(PYTHON),
            "support/codegen/python/generate.py",
            "--check",
            "--python-path",
            "_build/dev/python",
            "--output",
            "_build/dev/python/tmux_cxx/_native.pyi",
        ]
    )


def integration() -> None:
    """Run genuine Python and Node server suites through their configured CTest environment."""
    run([str(ROOT / ".venv/bin/ctest"), "--preset", "dev", "--no-tests=error", "-L", "mid"])


def sanitizers() -> None:
    """Require instrumented native and binding suites with fatal sanitizer probes."""
    run([str(ROOT / ".venv/bin/cmake"), "--build", "--preset", "sanitize"])
    run(
        [str(ROOT / ".venv/bin/ctest"), "--preset", "sanitize", "--no-tests=error"],
        report=ROOT / "_build/reports/sanitizer-tests.log",
    )
    for name, value, diagnostic in (
        ("address", "1", "ERROR: AddressSanitizer: heap-buffer-overflow"),
        ("undefined", "2147483647", "runtime error: signed integer overflow"),
        ("leak", "8", "ERROR: LeakSanitizer: detected memory leaks"),
    ):
        run(
            [
                str(ROOT / ".venv/bin/cmake"),
                "-E",
                "env",
                f"LSAN_OPTIONS=suppressions={ROOT}/support/sanitizers/external_leaks.supp",
                f"ASAN_SYMBOLIZER_PATH={ROOT}/_build/tools/llvm/bin/llvm-symbolizer",
                "ASAN_OPTIONS=detect_leaks=1:halt_on_error=1",
                "UBSAN_OPTIONS=halt_on_error=1:print_stacktrace=1",
                str(ROOT / "_build/sanitize/sanitizer_probe"),
                name,
                value,
            ],
            diagnostics=(diagnostic,),
            report=ROOT / f"_build/reports/sanitizer-negative-{name}.log",
        )


def fuzzing() -> None:
    """Fuzz instrumented wire and terminal invariants with bounded seed replay."""
    run([str(ROOT / ".venv/bin/cmake"), "--build", "--preset", "sanitize"])
    directory = ROOT / "_build/fuzz"
    directory.mkdir(parents=True, exist_ok=True)
    artifacts = directory / "failures"
    artifacts.mkdir(exist_ok=True)
    for component in ("native_protocol", "tmux_protocol", "terminal_state"):
        seeds = ROOT / "tests/cpp/fuzz/corpus" / component
        inputs = tuple(seeds.iterdir())
        if not inputs or any(
            not path.is_file() or not 0 < path.stat().st_size <= 4096 for path in inputs
        ):
            raise RuntimeError(f"{component}: a nonempty bounded seed corpus is required")
        with TemporaryDirectory(prefix=component + "-", dir=directory) as output:
            run(
                [
                    str(ROOT / ".venv/bin/cmake"),
                    "-E",
                    "env",
                    "--unset=LD_PRELOAD",
                    "--unset=LSAN_OPTIONS",
                    f"ASAN_SYMBOLIZER_PATH={ROOT}/_build/tools/llvm/bin/llvm-symbolizer",
                    "ASAN_OPTIONS=detect_leaks=1:halt_on_error=1",
                    "UBSAN_OPTIONS=halt_on_error=1:print_stacktrace=1",
                    str(ROOT / "_build/sanitize" / (component + "_fuzz")),
                    "-runs=2000",
                    "-seed=1",
                    "-max_len=4096",
                    "-timeout=1",
                    "-rss_limit_mb=512",
                    f"-artifact_prefix={artifacts}/",
                    output,
                    str(seeds),
                ],
                report=ROOT / f"_build/reports/fuzz-{component}.log",
            )


def packages() -> None:
    """Rebuild and consume the Python and Node artifacts without network access."""
    run([str(PYTHON), "support/check_packages.py"])


def standalone() -> None:
    """Build and consume a clean source copy with both bindings enabled."""
    run(
        [str(PYTHON), "support/check_standalone_source.py"],
        report=ROOT / "_build/reports/standalone.log",
    )


def main() -> None:
    """Select a check group and enforce its total elapsed-time budget."""
    global DEADLINE
    checks = {
        "native": native,
        "python": python,
        "node": node,
        "generation": generation,
        "integration": integration,
        "formatting": formatting,
        "documentation": documentation,
        "analysis": analysis,
        "sanitizers": sanitizers,
        "fuzzing": fuzzing,
        "packages": packages,
        "standalone": standalone,
    }
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("group", choices=["inner", "mid", *checks])
    parser.add_argument(
        "--fix", action="store_true", help="apply only the native formatting recipe"
    )
    arguments = parser.parse_args()
    os.chdir(ROOT)
    start = time.perf_counter()
    budget = (
        300
        if arguments.group
        in ("documentation", "analysis", "sanitizers", "fuzzing", "packages", "standalone")
        else (5 if arguments.group in ("inner", "native") else 30)
    )
    DEADLINE = start + budget
    if arguments.fix:
        if arguments.group != "formatting":
            parser.error("--fix is available only for the native formatting recipe")
        formatting(fix=True)
        return
    selected = (
        [native]
        if arguments.group == "inner"
        else (
            [
                checks[name]
                for name in ("native", "python", "node", "formatting", "generation", "integration")
            ]
            if arguments.group == "mid"
            else [checks[arguments.group]]
        )
    )
    passed = False
    try:
        for check in selected:
            check()
        passed = True
    finally:
        elapsed = time.perf_counter() - start
        report = {
            "group": arguments.group,
            "passed": passed,
            "seconds": elapsed,
            "budget_seconds": budget,
            "checks": [check.__name__ for check in selected],
        }
        destination = ROOT / f"_build/reports/check-{arguments.group}.json"
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_text(json.dumps(report, indent=2) + "\n")
    if elapsed >= budget:
        raise RuntimeError(f"{arguments.group} exceeded its {budget}-second budget")
    print(f"PASS {arguments.group}: {elapsed:.3f}s")


if __name__ == "__main__":
    main()
