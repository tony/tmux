#!/usr/bin/env python3
"""Build and consume the Python and Node packages without source-tree fallbacks."""

from __future__ import annotations

import hashlib
import json
import os
import shutil
import subprocess
import tarfile
import tomllib
from pathlib import Path
from tempfile import TemporaryDirectory

ROOT = Path(__file__).resolve().parents[1]
BUILD = ROOT / "_build"
PYTHON = ROOT / ".venv/bin/python"
UV = ROOT / ".venv/bin/uv"
NODE = ROOT / "_build/tools/node_current/bin/node"
NPM = ROOT / "_build/tools/npm/bin/npm-cli.js"
TY = ROOT / ".venv/bin/ty"
VTERM_ARCHIVE = "third_party/libvterm/libvterm-0.3.3.tar.gz"
MAXIMUM_SDIST_BYTES = 5 * 1024 * 1024


def run(
    arguments: list[str], *, cwd: Path = ROOT, environment: dict[str, str] | None = None
) -> None:
    """Run one package step and reject any nonzero exit status."""
    print("+", " ".join(arguments), flush=True)
    subprocess.run(arguments, cwd=cwd, env=environment, check=True)


def require_rejection(
    arguments: list[str],
    diagnostics: tuple[str, ...],
    *,
    cwd: Path = ROOT,
    environment: dict[str, str] | None = None,
) -> None:
    """Require one failing consumer and every expected type diagnostic."""
    completed = subprocess.run(
        arguments,
        cwd=cwd,
        env=environment,
        text=True,
        capture_output=True,
        check=False,
    )
    output = completed.stdout + completed.stderr
    if completed.returncode == 0 or any(diagnostic not in output for diagnostic in diagnostics):
        print(output, end="")
        raise RuntimeError(f"consumer did not fail as required: {' '.join(arguments)}")


def only_file(directory: Path, pattern: str) -> Path:
    """Return the sole artifact matching a package-build pattern."""
    matches = tuple(directory.glob(pattern))
    if len(matches) != 1:
        raise RuntimeError(f"expected one {pattern} artifact in {directory}, found {len(matches)}")
    return matches[0]


def offline_environment() -> dict[str, str]:
    """Use provisioned project caches while disabling all network access."""
    return dict(
        os.environ,
        UV_CACHE_DIR=str(BUILD / "cache/uv"),
        UV_OFFLINE="1",
        npm_config_cache=str(BUILD / "cache/npm"),
        npm_config_offline="true",
    )


def inspect_source_distribution(archive: Path) -> None:
    """Require a bounded sdist with the complete locked native source graph."""
    if archive.stat().st_size > MAXIMUM_SDIST_BYTES:
        raise RuntimeError(f"source distribution exceeds {MAXIMUM_SDIST_BYTES} bytes")
    with tarfile.open(archive, "r:gz") as source:
        members = source.getmembers()
        names = {member.name for member in members}
        if any("/node_modules/" in name for name in names):
            raise RuntimeError("source distribution contains bindings/node/node_modules")
        vendor = next((member for member in members if member.name.endswith(VTERM_ARCHIVE)), None)
        if vendor is None:
            raise RuntimeError(f"source distribution is missing {VTERM_ARCHIVE}")
        payload = source.extractfile(vendor)
        if payload is None:
            raise RuntimeError(f"source distribution cannot read {VTERM_ARCHIVE}")
        with (ROOT / "support/toolchain.lock.toml").open("rb") as lockfile:
            expected = tomllib.load(lockfile)["libvterm"]["sha256"]
        if hashlib.sha256(payload.read()).hexdigest() != expected:
            raise RuntimeError("source distribution contains an unrecognized libvterm archive")


def extract_source_distribution(archive: Path, destination: Path) -> Path:
    """Extract one checked sdist and return its single project directory."""
    with tarfile.open(archive, "r:gz") as source:
        source.extractall(destination, filter="data")
    projects = tuple(path for path in destination.iterdir() if path.is_dir())
    if len(projects) != 1:
        raise RuntimeError(f"expected one extracted project, found {len(projects)}")
    return projects[0]


def check_python_package(workspace: Path) -> None:
    """Rebuild and import a wheel using only the emitted source distribution."""
    environment = offline_environment()
    distributions = workspace / "python"
    distributions.mkdir()
    run(
        [
            str(UV),
            "build",
            "--sdist",
            "--no-build-isolation",
            "--out-dir",
            str(distributions),
        ],
        environment=environment,
    )

    source_archive = only_file(distributions, "*.tar.gz")
    inspect_source_distribution(source_archive)

    extracted = workspace / "source"
    extracted.mkdir()
    project = extract_source_distribution(source_archive, extracted)
    wheels = workspace / "wheels"
    run(
        [
            str(UV),
            "build",
            "--wheel",
            "--no-build-isolation",
            "--out-dir",
            str(wheels),
            str(project),
        ],
        environment=environment,
    )
    wheel = only_file(wheels, "*.whl")

    environment_dir = workspace / "python-consumer"
    run([str(UV), "venv", "--python", str(PYTHON), str(environment_dir)], environment=environment)
    consumer_python = environment_dir / "bin/python"
    run(
        [str(UV), "pip", "install", "--python", str(consumer_python), "--no-deps", str(wheel)],
        environment=environment,
    )
    run(
        [
            str(consumer_python),
            "-c",
            "import tmux_cxx; assert callable(tmux_cxx.ServerConnection)",
        ],
        cwd=workspace,
        environment=environment,
    )
    type_project = workspace / "python-types"
    type_project.mkdir()
    (type_project / "pyproject.toml").write_text(
        '[project]\nname = "tmux-cxx-package-check"\nversion = "0"\nrequires-python = ">=3.12"\n'
    )
    positive = type_project / "positive.py"
    negative = type_project / "negative.py"
    shutil.copy2(ROOT / "tests/integration/types_positive.py", positive)
    shutil.copy2(ROOT / "tests/integration/types_negative.py", negative)
    type_arguments = ["--project", str(type_project), "--python", str(consumer_python)]
    run([str(TY), "check", *type_arguments, str(positive)], environment=environment)
    require_rejection(
        [str(TY), "check", *type_arguments, "--output-format", "concise", str(negative)],
        (
            "error[invalid-argument-type]",
            "error[invalid-assignment]",
            "error[unresolved-attribute]",
        ),
        environment=environment,
    )


def inspect_node_package(archive: Path) -> None:
    """Reject runtime dependencies and require the documented addon payload."""
    with tarfile.open(archive, "r:gz") as package:
        manifest_file = package.extractfile("package/package.json")
        if manifest_file is None:
            raise RuntimeError("npm package is missing package.json")
        manifest = json.load(manifest_file)
        if manifest.get("dependencies"):
            raise RuntimeError("prebuilt npm package declares runtime dependencies")
        names = {member.name for member in package.getmembers()}
    required = {
        "package/dist/tmux_cxx.node",
        "package/index.cjs",
        "package/index.d.ts",
        "package/session_event_stream.cjs",
    }
    missing = required - names
    if missing:
        raise RuntimeError(f"npm package is missing: {', '.join(sorted(missing))}")


def check_node_package(workspace: Path) -> None:
    """Install and load the packed addon with an empty offline npm cache."""
    environment = offline_environment()
    packages = workspace / "node"
    packages.mkdir()
    run(
        [
            str(NODE),
            str(NPM),
            "pack",
            str(ROOT / "bindings/node"),
            "--pack-destination",
            str(packages),
        ],
        environment=environment,
    )
    archive = only_file(packages, "*.tgz")
    inspect_node_package(archive)

    consumer = workspace / "node-consumer"
    consumer.mkdir()
    (consumer / "package.json").write_text('{"name":"package-check","private":true}\n')
    cache = workspace / "empty-npm-cache"
    run(
        [
            str(NODE),
            str(NPM),
            "install",
            "--prefix",
            str(consumer),
            "--cache",
            str(cache),
            "--offline",
            str(archive),
        ],
        environment=environment,
    )
    run(
        [
            str(NODE),
            "-e",
            'const api = require("tmux-cxx"); '
            'if (typeof api.ServerConnection !== "function") process.exit(1);',
        ],
        cwd=consumer,
        environment=environment,
    )
    positive = consumer / "positive.cts"
    negative = consumer / "negative.cts"
    shutil.copy2(ROOT / "bindings/node/tests/types-positive.cts", positive)
    shutil.copy2(ROOT / "bindings/node/tests/types-negative.cts", negative)
    compiler = ROOT / "bindings/node/node_modules/typescript/bin/tsc"
    type_arguments = [
        str(NODE),
        str(compiler),
        "--ignoreConfig",
        "--noEmit",
        "--strict",
        "--module",
        "NodeNext",
        "--target",
        "ES2022",
        "--types",
        "node",
        "--typeRoots",
        str(ROOT / "bindings/node/node_modules/@types"),
    ]
    run([*type_arguments, str(positive)], cwd=consumer, environment=environment)
    require_rejection(
        [*type_arguments, str(negative)],
        ("TS2345", "TS2322", "TS2339"),
        cwd=consumer,
        environment=environment,
    )


def main() -> None:
    """Check both release artifacts inside one disposable project-local workspace."""
    probes = BUILD / "probes"
    probes.mkdir(parents=True, exist_ok=True)
    with TemporaryDirectory(prefix="packages-", dir=probes) as temporary:
        workspace = Path(temporary)
        check_python_package(workspace)
        check_node_package(workspace)


if __name__ == "__main__":
    main()
