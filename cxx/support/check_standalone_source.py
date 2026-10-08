#!/usr/bin/env python3
"""Build, install, and consume a clean source copy with both bindings enabled."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
from pathlib import Path
from tempfile import TemporaryDirectory

ROOT = Path(__file__).resolve().parents[1]
BUILD = ROOT / "_build"
CMAKE = ROOT / ".venv/bin/cmake"
NINJA = ROOT / ".venv/bin/ninja"
PYTHON = ROOT / ".venv/bin/python"
CLANG = ROOT / "_build/tools/llvm/bin/clang++"
NODE_ROOT = ROOT / "_build/tools/node_current"
NODE = NODE_ROOT / "bin/node"
NPM = ROOT / "_build/tools/npm/bin/npm-cli.js"


def environment() -> dict[str, str]:
    """Expose only the locked external tools and runtimes required by the copied source."""
    return dict(
        os.environ,
        PATH=str(NODE.parent) + os.pathsep + os.environ["PATH"],
        LD_LIBRARY_PATH=str(ROOT / "_build/tools/llvm_runtime/usr/lib/x86_64-linux-gnu")
        + os.pathsep
        + str(ROOT / "_build/tools/llvm/lib")
        + os.pathsep
        + os.environ.get("LD_LIBRARY_PATH", ""),
        npm_config_cache=str(ROOT / "_build/cache/npm"),
        npm_config_offline="true",
    )


def run(arguments: list[str], *, cwd: Path, env: dict[str, str]) -> None:
    """Run one visible standalone-consumer step and reject any failure."""
    print("+", " ".join(arguments), flush=True)
    subprocess.run(arguments, cwd=cwd, env=env, check=True)


def copy_authored_source(destination: Path) -> None:
    """Copy authored inputs while excluding every local environment and generated build tree."""
    shutil.copytree(
        ROOT,
        destination,
        ignore=shutil.ignore_patterns(
            "_build", ".venv", "node_modules", "__pycache__", ".pytest_cache"
        ),
    )


def configure_arguments(source: Path, build: Path, install: Path) -> list[str]:
    """Describe a normal release build with both required language adapters enabled."""
    return [
        str(CMAKE),
        "-S",
        str(source),
        "-B",
        str(build),
        "-G",
        "Ninja",
        "-DBUILD_TESTING=OFF",
        "-DCMAKE_BUILD_TYPE=Release",
        "-DCMAKE_EXPORT_COMPILE_COMMANDS=ON",
        f"-DCMAKE_CXX_COMPILER={CLANG}",
        f"-DCMAKE_INSTALL_PREFIX={install}",
        f"-DCMAKE_MAKE_PROGRAM={NINJA}",
        f"-DPython_EXECUTABLE={PYTHON}",
        "-DTMUX_CXX_BUILD_NODE_BINDING=ON",
        "-DTMUX_CXX_BUILD_PYTHON_BINDING=ON",
        f"-DTMUX_CXX_NODE_ROOT={NODE_ROOT}",
    ]


def require_copied_translation_units(source: Path, build: Path) -> None:
    """Reject compilation of first-party units from the original checkout."""
    records = json.loads((build / "compile_commands.json").read_text())
    copied = source.resolve()
    original_components = {"bindings", "include", "src", "tests"}
    for record in records:
        translation_unit = Path(record["file"]).resolve()
        if translation_unit.is_relative_to(copied):
            continue
        if translation_unit.is_relative_to(ROOT):
            relative = translation_unit.relative_to(ROOT)
            if relative.parts and relative.parts[0] in original_components:
                raise RuntimeError(f"standalone build compiled original source: {relative}")


def require_products(source: Path, build: Path, install: Path) -> None:
    """Require the executable, both adapters, exported native target, and copied provenance."""
    python_extensions = tuple((build / "python/tmux_cxx").glob("_native*.so"))
    required = (
        build / "tmux-cxx",
        build / "node/tmux_cxx.node",
        install / "bin/tmux-cxx",
        install / "lib/cmake/tmux_cxx/tmux_cxxConfig.cmake",
        install / "share/tmux-cxx/licenses/libvterm.txt",
    )
    if len(python_extensions) != 1 or any(not path.is_file() for path in required):
        raise RuntimeError("standalone build omitted a required native or binding product")
    for metadata in (install / "lib/cmake/tmux_cxx").glob("*.cmake"):
        if str(source) in metadata.read_text():
            raise RuntimeError(f"installed metadata retains copied source path: {metadata.name}")


def load_bindings(source: Path, build: Path, env: dict[str, str]) -> None:
    """Load both freshly built adapters without falling back to the original checkout."""
    python_environment = dict(env, PYTHONPATH=str(build / "python"))
    run(
        [str(PYTHON), "-c", "import tmux_cxx; assert callable(tmux_cxx.ServerConnection)"],
        cwd=source,
        env=python_environment,
    )
    node_environment = dict(env, TMUX_CXX_ADDON=str(build / "node/tmux_cxx.node"))
    run(
        [
            str(NODE),
            "-e",
            'const api = require("./bindings/node"); '
            'if (typeof api.ServerConnection !== "function") process.exit(1);',
        ],
        cwd=source,
        env=node_environment,
    )


def consume_install(source: Path, install: Path, workspace: Path, env: dict[str, str]) -> None:
    """Configure and run an external C++ consumer against only the installed package."""
    consumer_build = workspace / "consumer-build"
    run(
        [
            str(CMAKE),
            "-S",
            str(source / "tests/installed/native"),
            "-B",
            str(consumer_build),
            "-G",
            "Ninja",
            "-DCMAKE_BUILD_TYPE=Release",
            f"-DCMAKE_CXX_COMPILER={CLANG}",
            f"-DCMAKE_MAKE_PROGRAM={NINJA}",
            f"-DCMAKE_PREFIX_PATH={install}",
        ],
        cwd=workspace,
        env=env,
    )
    run(
        [str(CMAKE), "--build", str(consumer_build), "--parallel", "4"],
        cwd=workspace,
        env=env,
    )
    run([str(consumer_build / "tmux_cxx_installed_consumer")], cwd=workspace, env=env)


def main() -> None:
    """Prove a debris-free source copy builds both adapters and an independent consumer."""
    workspace_root = BUILD / "standalone"
    workspace_root.mkdir(parents=True, exist_ok=True)
    env = environment()
    with TemporaryDirectory(prefix="source-", dir=workspace_root) as directory:
        workspace = Path(directory)
        source = workspace / "source"
        build = workspace / "build"
        install = workspace / "install"
        copy_authored_source(source)
        run(
            [str(NODE), str(NPM), "ci", "--ignore-scripts", "--offline", "--no-audit", "--no-fund"],
            cwd=source / "bindings/node",
            env=env,
        )
        run(configure_arguments(source, build, install), cwd=workspace, env=env)
        run([str(CMAKE), "--build", str(build), "--parallel", "4"], cwd=workspace, env=env)
        require_copied_translation_units(source, build)
        run([str(CMAKE), "--install", str(build)], cwd=workspace, env=env)
        require_products(source, build, install)
        load_bindings(source, build, env)
        consume_install(source, install, workspace, env)


if __name__ == "__main__":
    main()
