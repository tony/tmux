#!/usr/bin/env python3
"""Build and consume the Python and Node packages without source-tree fallbacks."""

from __future__ import annotations

import hashlib
import json
import os
import selectors
import shutil
import signal
import subprocess
import tarfile
import tomllib
import zipfile
from dataclasses import dataclass
from pathlib import Path
from tempfile import TemporaryDirectory

ROOT = Path(__file__).resolve().parents[1]
BUILD = ROOT / "_build"
PYTHON = ROOT / ".venv/bin/python"
UV = ROOT / ".venv/bin/uv"
NODE = ROOT / "_build/tools/node_current/bin/node"
NODE_LTS = ROOT / "_build/tools/node_lts/bin/node"
NPM = ROOT / "_build/tools/npm/bin/npm-cli.js"
TY = ROOT / ".venv/bin/ty"
VTERM_ARCHIVE = "third_party/libvterm/libvterm-0.3.3.tar.gz"
MAXIMUM_SDIST_BYTES = 5 * 1024 * 1024


@dataclass(frozen=True)
class InstalledPython:
    """Locate one fresh wheel consumer and its packaged daemon."""

    executable: Path
    prefix: Path
    daemon: Path


def consumer_environment(environment: dict[str, str]) -> dict[str, str]:
    """Remove source-tree loader overrides while preserving package-build tool settings."""
    overrides = {
        "PYTHONPATH",
        "PYTHONHOME",
        "TMUX_CXX_ADDON",
        "TMUX_CXX_DAEMON",
        "NODE_PATH",
        "NODE_OPTIONS",
    }
    return {name: value for name, value in environment.items() if name not in overrides}


def run(
    arguments: list[str],
    *,
    cwd: Path = ROOT,
    environment: dict[str, str] | None = None,
    timeout_seconds: float | None = None,
) -> None:
    """Run one package step and reject any nonzero exit status."""
    print("+", " ".join(arguments), flush=True)
    subprocess.run(arguments, cwd=cwd, env=environment, check=True, timeout=timeout_seconds)


def require_rejection(
    arguments: list[str],
    diagnostics: tuple[str, ...],
    *,
    cwd: Path = ROOT,
    environment: dict[str, str] | None = None,
    timeout_seconds: float | None = None,
) -> None:
    """Require one failing consumer and every expected type diagnostic."""
    completed = subprocess.run(
        arguments,
        cwd=cwd,
        env=environment,
        text=True,
        capture_output=True,
        check=False,
        timeout=timeout_seconds,
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


def check_python_package(workspace: Path) -> InstalledPython:
    """Build an sdist-only wheel and verify its independently installed types and payload."""
    environment = offline_environment()
    environment["CXX"] = str(BUILD / "tools/llvm/bin/clang++")
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
    with zipfile.ZipFile(wheel) as package:
        members = package.namelist()
        if "bin/tmux-cxx" not in members:
            raise RuntimeError("Python wheel is missing its bin/tmux-cxx daemon")

    environment_dir = workspace / "python-consumer"
    run([str(UV), "venv", "--python", str(PYTHON), str(environment_dir)], environment=environment)
    consumer_python = environment_dir / "bin/python"
    run(
        [str(UV), "pip", "install", "--python", str(consumer_python), "--no-deps", str(wheel)],
        environment=environment,
    )
    installed_environment = consumer_environment(environment)
    purelib = subprocess.check_output(
        [
            str(consumer_python),
            "-I",
            "-c",
            "import sysconfig; print(sysconfig.get_path('purelib'))",
        ],
        cwd=workspace,
        env=installed_environment,
        text=True,
    ).strip()
    daemon = Path(purelib) / "bin/tmux-cxx"
    if not daemon.is_file() or not os.access(daemon, os.X_OK):
        raise RuntimeError("installed Python wheel has no executable daemon payload")
    type_project = workspace / "python-types"
    type_project.mkdir()
    (type_project / "pyproject.toml").write_text(
        '[project]\nname = "tmux-cxx-package-check"\nversion = "0"\nrequires-python = ">=3.13"\n'
    )
    positive = type_project / "positive.py"
    negative = type_project / "negative.py"
    shutil.copy2(ROOT / "tests/integration/types_positive.py", positive)
    shutil.copy2(ROOT / "tests/integration/types_negative.py", negative)
    type_arguments = ["--project", str(type_project), "--python", str(consumer_python)]
    run([str(TY), "check", *type_arguments, str(positive)], environment=installed_environment)
    require_rejection(
        [str(TY), "check", *type_arguments, "--output-format", "concise", str(negative)],
        (
            "error[invalid-argument-type]",
            "error[invalid-assignment]",
            "error[unresolved-attribute]",
        ),
        environment=installed_environment,
    )
    return InstalledPython(consumer_python, environment_dir, daemon)


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


def check_node_package(workspace: Path) -> Path:
    """Install the packed addon and check types through an empty offline npm cache."""
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
    installed_environment = consumer_environment(environment)
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
    run([*type_arguments, str(positive)], cwd=consumer, environment=installed_environment)
    require_rejection(
        [*type_arguments, str(negative)],
        ("TS2345", "TS2322", "TS2339"),
        cwd=consumer,
        environment=installed_environment,
    )
    return consumer


def require_installed_addon(consumer: Path, environment: dict[str, str]) -> None:
    """Reject a missing installed addon even when a working build-tree override is supplied."""
    addon = consumer / "node_modules/tmux-cxx/dist/tmux_cxx.node"
    retained = addon.with_suffix(".retained")
    fallback = BUILD / "dev/node/tmux_cxx.node"
    if not fallback.is_file():
        raise RuntimeError("missing build-tree addon for the loader-isolation negative")
    addon.rename(retained)
    try:
        poison = dict(environment, TMUX_CXX_ADDON=str(fallback))
        run(
            [str(NODE), "-e", 'require("tmux-cxx")'],
            cwd=consumer,
            environment=poison,
            timeout_seconds=0.9,
        )
        require_rejection(
            [str(NODE), "-e", 'require("tmux-cxx")'],
            ("Cannot find module './dist/tmux_cxx.node'",),
            cwd=consumer,
            environment=consumer_environment(poison),
            timeout_seconds=0.9,
        )
    finally:
        retained.rename(addon)


def check_installed_runtime(workspace: Path, python: InstalledPython, node: Path) -> None:
    """Mutate one packaged daemon through both installed bindings and verify owned cleanup."""
    environment = consumer_environment(offline_environment())
    python_script = workspace / "shared_server.py"
    node_script = node / "shared_server.mts"
    shutil.copy2(ROOT / "tests/installed/python/shared_server.py", python_script)
    shutil.copy2(ROOT / "bindings/node/tests/installed_server.mts", node_script)
    socket = workspace / "socket"
    record = workspace / "session.txt"
    python_arguments = [str(python.executable), "-I", str(python_script)]
    consumer_arguments = [str(socket), str(python.prefix), str(record)]
    run(
        [*python_arguments, "probe", *consumer_arguments],
        cwd=workspace,
        environment=environment,
        timeout_seconds=0.9,
    )
    require_installed_addon(node, environment)
    daemon_environment = dict(environment)
    # The owned runtime fixture starts from emacs independently of the caller's editor.
    daemon_environment.pop("VISUAL", None)
    daemon_environment.pop("EDITOR", None)
    read_fd, write_fd = os.pipe()
    try:
        daemon = subprocess.Popen(
            [str(python.daemon), "serve", "--socket", str(socket), "--ready-fd", str(write_fd)],
            cwd=workspace,
            env=daemon_environment,
            pass_fds=(write_fd,),
        )
    except BaseException:
        os.close(read_fd)
        raise
    finally:
        os.close(write_fd)
    try:
        with selectors.DefaultSelector() as readiness:
            readiness.register(read_fd, selectors.EVENT_READ)
            if not readiness.select(timeout=0.75) or os.read(read_fd, 1) != b"R":
                raise RuntimeError("packaged daemon did not signal readiness")
        run(
            [*python_arguments, "create", *consumer_arguments],
            cwd=workspace,
            environment=environment,
            timeout_seconds=0.9,
        )
        for runtime, expected_name, token in (
            (NODE, "installed-python", "node-current-pty"),
            (NODE_LTS, "installed-node", "node-lts-pty"),
        ):
            run(
                [
                    str(runtime),
                    str(node_script),
                    str(socket),
                    str(record),
                    str(node),
                    expected_name,
                    token,
                ],
                cwd=node,
                environment=environment,
                timeout_seconds=0.9,
            )
        run(
            [*python_arguments, "observe", *consumer_arguments],
            cwd=workspace,
            environment=environment,
            timeout_seconds=0.9,
        )
    finally:
        os.close(read_fd)
        if daemon.poll() is None:
            daemon.send_signal(signal.SIGTERM)
        try:
            code = daemon.wait(timeout=0.85)
        except subprocess.TimeoutExpired:
            daemon.kill()
            daemon.wait(timeout=0.15)
            raise RuntimeError("packaged daemon exceeded its shutdown bound") from None
        if code != 0:
            raise RuntimeError(f"packaged daemon exited with status {code}")
    print("PASS installed Python/Node shared-daemon mutations, PTY traffic, errors, persistence")


def main() -> None:
    """Check both release artifacts inside one disposable project-local workspace."""
    probes = BUILD / "probes"
    probes.mkdir(parents=True, exist_ok=True)
    with TemporaryDirectory(prefix="packages-", dir=probes) as temporary:
        workspace = Path(temporary)
        python = check_python_package(workspace)
        node = check_node_package(workspace)
        check_installed_runtime(workspace, python, node)


if __name__ == "__main__":
    main()
