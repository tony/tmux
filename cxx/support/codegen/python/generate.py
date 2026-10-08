"""Generate built-module Python stubs with the native error context exposed accurately."""

from __future__ import annotations

import argparse
import os
import subprocess
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from exposure import ROOT, load, write


def main() -> None:
    """Run nanobind's built-module generator and supply compiled error-field declarations.

    Load a requested sanitizer runtime only into the built-module import process.
    Keep leak detection in the dedicated sanitizer gate rather than documentation generation.
    """
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--python-path", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--sanitizer-runtime", type=Path)
    arguments = parser.parse_args()
    import_environment = dict(os.environ)
    if arguments.sanitizer_runtime is not None:
        import_environment.update(
            LD_PRELOAD=str(arguments.sanitizer_runtime),
            ASAN_SYMBOLIZER_PATH=str(ROOT / "_build/tools/llvm/bin/llvm-symbolizer"),
            ASAN_OPTIONS="detect_leaks=1:halt_on_error=1",
            LSAN_OPTIONS="detect_leaks=0",
            UBSAN_OPTIONS="halt_on_error=1:print_stacktrace=1",
        )
    temporary_root = ROOT / "_build/codegen"
    temporary_root.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=temporary_root) as directory:
        pattern, output = Path(directory) / "error.pat", Path(directory) / "_native.pyi"
        fields = [f"    {f.name}: str" for f in load().error]
        pattern.write_text("tmux_cxx._native.TmuxError.__prefix__:\n" + "\n".join(fields) + "\n")
        subprocess.run(
            [
                sys.executable,
                "-m",
                "nanobind.stubgen",
                "-m",
                "tmux_cxx._native",
                "-i",
                str(arguments.python_path),
                "-o",
                str(output),
                "-p",
                str(pattern),
            ],
            check=True,
            env=import_environment,
        )
        subprocess.run([sys.executable, "-m", "ruff", "format", str(output)], check=True)
        write(arguments.output, output.read_text(), check=arguments.check)
        write(
            ROOT / "bindings/python/tmux_cxx/_native.pyi", output.read_text(), check=arguments.check
        )
        write(ROOT / "bindings/python/tmux_cxx/py.typed", "", check=arguments.check)


if __name__ == "__main__":
    main()
