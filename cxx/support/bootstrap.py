#!/usr/bin/env python3
"""Explicit tool resolution and project-local provisioning; never a build system."""

from __future__ import annotations

import argparse
import base64
import datetime
import hashlib
import json
import os
import platform
import re
import shutil
import subprocess
import sys
import tarfile
import tomllib
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BUILD = ROOT / "_build"
LOCK = ROOT / "support/toolchain.lock.toml"


def read_json(url: str) -> object:
    """Read bounded remote release metadata during explicit tool resolution."""
    with urllib.request.urlopen(url, timeout=30) as response:
        return json.load(response)


def run(args: list[str], *, cwd: Path = ROOT, env: dict[str, str] | None = None) -> None:
    """Run an explicit provisioning command and propagate its failure."""
    print("+", " ".join(args), flush=True)
    subprocess.run(args, cwd=cwd, env=env, check=True)


def encode_toolchain(records: dict[str, object], table: str = "") -> str:
    """Preserve nested tool provenance when encoding the lock's scalar and table values."""
    lines = [f"[{table}]"] if table else []
    for name, value in records.items():
        if not isinstance(value, dict):
            lines.append(f"{name} = {json.dumps(value)}")
    for name, value in records.items():
        if isinstance(value, dict):
            lines += ["", encode_toolchain(value, f"{table}.{name}" if table else name)]
    return "\n".join(lines)


def resolve() -> None:
    """Refresh distributed tools while preserving verified fixtures, platform facts, and patches."""
    if platform.machine() != "x86_64":
        raise RuntimeError("the initial locked distribution targets Linux x86_64")
    with LOCK.open("rb") as source:
        previous = tomllib.load(source)
    records = previous.copy()
    resolved = datetime.datetime.now(datetime.UTC).date().isoformat()
    for name, repo, pattern, layout in (
        ("llvm", "llvm/llvm-project", r"LLVM-.*-Linux-X64\.tar\.zst", "prefixed"),
        ("doxygen", "doxygen/doxygen", r"doxygen-.*\.linux\.bin\.tar\.gz", "prefixed"),
        ("just", "casey/just", r"just-.*-x86_64-unknown-linux-musl\.tar\.gz", "flat"),
    ):
        release = read_json(f"https://api.github.com/repos/{repo}/releases/latest")
        assert isinstance(release, dict)
        if release["prerelease"] or release["draft"]:
            raise RuntimeError(f"{name}: release endpoint returned a prerelease")
        asset = next(a for a in release["assets"] if re.fullmatch(pattern, a["name"]))
        digest = asset.get("digest", "")
        if not digest.startswith("sha256:"):
            raise RuntimeError(f"{name}: release has no SHA-256 digest")
        records[name] = {
            "version": release["tag_name"]
            .removeprefix("llvmorg-")
            .removeprefix("Release_")
            .replace("_", "."),
            "url": asset["browser_download_url"],
            "sha256": digest.removeprefix("sha256:"),
            "source": f"https://github.com/{repo}/releases/tag/{release['tag_name']}",
            "layout": layout,
            "resolved": resolved,
        }
    llvm_version = records["llvm"]["version"]
    clang_python = {"version": llvm_version, "source": previous["clang_python"]["source"]}
    for filename, prefix in (("__init__.py", "init"), ("cindex.py", "cindex")):
        url = (
            f"https://raw.githubusercontent.com/llvm/llvm-project/llvmorg-{llvm_version}/"
            f"clang/bindings/python/clang/{filename}"
        )
        with urllib.request.urlopen(url, timeout=30) as response:
            clang_python[f"{prefix}_sha256"] = hashlib.sha256(response.read()).hexdigest()
        clang_python[f"{prefix}_url"] = url
    records["clang_python"] = clang_python
    nodes = read_json("https://nodejs.org/dist/index.json")
    assert isinstance(nodes, list)
    for name, want_lts in (("node_current", False), ("node_lts", True)):
        node = next(n for n in nodes if bool(n["lts"]) == want_lts and "linux-x64" in n["files"])
        version = node["version"]
        base = f"https://nodejs.org/dist/{version}"
        archive = f"node-{version}-linux-x64.tar.xz"
        with urllib.request.urlopen(f"{base}/SHASUMS256.txt", timeout=30) as response:
            sums = response.read().decode()
        digest = next(line.split()[0] for line in sums.splitlines() if line.split()[-1] == archive)
        records[name] = {
            "version": version.removeprefix("v"),
            "npm": node["npm"],
            "url": f"{base}/{archive}",
            "sha256": digest,
            "resolved": resolved,
        }
    npm = read_json("https://registry.npmjs.org/npm/latest")
    assert isinstance(npm, dict)
    with urllib.request.urlopen(npm["dist"]["tarball"], timeout=30) as response:
        archive = response.read()
    integrity = "sha512-" + base64.b64encode(hashlib.sha512(archive).digest()).decode()
    if integrity != npm["dist"]["integrity"]:
        raise RuntimeError("npm: release archive differs from registry integrity")
    records["npm"] = {
        "version": npm["version"],
        "url": npm["dist"]["tarball"],
        "sha256": hashlib.sha256(archive).hexdigest(),
        "node": npm["engines"]["node"],
        "resolved": resolved,
    }
    python_releases = read_json(
        "https://www.python.org/api/v2/downloads/release/?is_published=true"
    )
    assert isinstance(python_releases, list)
    stable = [
        r["name"].removeprefix("Python ")
        for r in python_releases
        if re.fullmatch(r"Python 3\.\d+\.\d+", r["name"])
    ]

    def python_version(value: str) -> tuple[int, ...]:
        """Compare final Python releases numerically during explicit resolution."""
        return tuple(map(int, value.split(".")))

    records["python"] = {
        "version": max(stable, key=python_version),
        "source": "https://www.python.org/downloads/",
        "ubuntu_version": previous["python"]["ubuntu_version"],
        "graph": "uv.lock",
        "resolved": resolved,
    }
    uv = read_json("https://pypi.org/pypi/uv/json")
    assert isinstance(uv, dict)
    records["uv"] = {
        "version": uv["info"]["version"],
        "source": "https://pypi.org/project/uv/",
        "graph": "uv.lock",
        "resolved": resolved,
    }
    encoded = encode_toolchain(records) + "\n"
    if tomllib.loads(encoded) != records:
        raise RuntimeError("resolved lock cannot preserve all tool provenance")
    replacement = LOCK.with_suffix(".toml.part")
    replacement.write_text(encoded)
    replacement.replace(LOCK)
    print("Resolved", LOCK.relative_to(ROOT))


def provision_archive(name: str, record: dict[str, str]) -> None:
    """Verify a locked archive before extracting it into the project-local tool directory."""
    destination = BUILD / "tools" / name
    stamp = destination / ".archive-sha256"
    if stamp.exists() and stamp.read_text().strip() == record["sha256"]:
        return
    archive = BUILD / "downloads" / record["url"].rsplit("/", 1)[-1]
    archive.parent.mkdir(parents=True, exist_ok=True)
    if not archive.exists():
        part = archive.with_suffix(archive.suffix + ".part")
        run(
            [
                "curl",
                "--fail",
                "--location",
                "--silent",
                "--show-error",
                "--output",
                str(part),
                record["url"],
            ]
        )
        part.rename(archive)
    with archive.open("rb") as source:
        digest = hashlib.file_digest(source, "sha256").hexdigest()
    if digest != record["sha256"]:
        raise RuntimeError(f"{name}: archive hash mismatch")
    if destination.exists():
        raise RuntimeError(f"{name}: incomplete install exists; inspect it before replacement")
    destination.mkdir(parents=True)
    if record.get("format") == "deb":
        run(["dpkg-deb", "--extract", str(archive), str(destination)])
    elif archive.name.endswith(".zst"):
        run(
            [
                "tar",
                "--use-compress-program=zstd --long=30 -d",
                "-xf",
                str(archive),
                "--strip-components=1",
                "-C",
                str(destination),
            ]
        )
    else:
        with tarfile.open(archive) as source:
            members = source.getmembers()
            layout = record.get("layout", "prefixed")
            if layout not in ("flat", "prefixed"):
                raise RuntimeError(f"{name}: unknown archive layout")
            if layout == "prefixed":
                for member in members:
                    _, _, member.name = member.name.partition("/")
            source.extractall(destination, members=(m for m in members if m.name), filter="data")
    stamp.write_text(record["sha256"] + "\n")


def provision_deb_bundle(name: str, record: dict[str, object]) -> None:
    """Verify and extract a locked Debian package set into one project-local tool prefix."""
    package_records = record.get("packages")
    if not isinstance(package_records, dict) or not package_records:
        raise RuntimeError(f"{name}: locked Debian package set is empty")
    packages: list[tuple[str, Path, str, str]] = []
    manifest: dict[str, str] = {}
    for package_name, package_record in sorted(package_records.items()):
        if not isinstance(package_record, dict):
            raise RuntimeError(f"{name}/{package_name}: invalid package record")
        filename = str(package_record.get("filename", ""))
        url = str(package_record.get("url", ""))
        digest = str(package_record.get("sha256", ""))
        if not filename or Path(filename).name != filename or not url or not digest:
            raise RuntimeError(f"{name}/{package_name}: incomplete package record")
        archive = BUILD / "downloads" / filename
        packages.append((str(package_name), archive, url, digest))
        manifest[filename] = digest
    destination = BUILD / "tools" / name
    stamp = destination / ".deb-packages.json"
    if stamp.exists() and json.loads(stamp.read_text()) == manifest:
        return
    if destination.exists():
        raise RuntimeError(f"{name}: incomplete install exists; inspect it before replacement")
    for package_name, archive, url, digest in packages:
        archive.parent.mkdir(parents=True, exist_ok=True)
        if not archive.exists():
            part = archive.with_suffix(archive.suffix + ".part")
            run(
                [
                    "curl",
                    "--fail",
                    "--location",
                    "--silent",
                    "--show-error",
                    "--output",
                    str(part),
                    url,
                ]
            )
            part.rename(archive)
        with archive.open("rb") as source:
            actual_digest = hashlib.file_digest(source, "sha256").hexdigest()
        if actual_digest != digest:
            raise RuntimeError(f"{name}/{package_name}: package hash mismatch")
    destination.mkdir(parents=True)
    for _, archive, _, _ in packages:
        run(["dpkg-deb", "--extract", str(archive), str(destination)])
    stamp.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")


def provision_python(lock: dict[str, object]) -> None:
    """Provision the locked Python interpreter and dependency graph within this project."""
    uv_bootstrap = shutil.which("uv")
    if not uv_bootstrap:
        raise RuntimeError("install a uv bootstrap executable before provisioning")
    toolenv = BUILD / "tools/uv-env"
    env = dict(
        os.environ,
        UV_CACHE_DIR=str(BUILD / "cache/uv"),
        UV_PYTHON_INSTALL_DIR=str(BUILD / "tools/python"),
    )
    if not (toolenv / "bin/uv").exists():
        run([uv_bootstrap, "venv", str(toolenv)], env=env)
        uv_record = lock["uv"]
        assert isinstance(uv_record, dict)
        run(
            [
                uv_bootstrap,
                "pip",
                "install",
                "--python",
                str(toolenv / "bin/python"),
                f"uv=={uv_record['version']}",
            ],
            env=env,
        )
    uv = str(toolenv / "bin/uv")
    python_record = lock["python"]
    assert isinstance(python_record, dict)
    version = str(python_record["version"])
    run([uv, "python", "install", version], env=env)
    python = subprocess.check_output(
        [uv, "python", "find", "--managed-python", version], env=env, text=True
    ).strip()
    if not (ROOT / ".venv").exists():
        run([uv, "venv", "--python", python, ".venv"], env=env)
    run([uv, "sync", "--locked", "--no-install-project"], env=env)


def provision_clang_python(record: dict[str, str]) -> None:
    """Verify LLVM's Python binding sources before matching them to the locked libclang."""
    directory = BUILD / "tools/clang_python/clang"
    directory.mkdir(parents=True, exist_ok=True)
    for name, prefix in (("__init__.py", "init"), ("cindex.py", "cindex")):
        destination = directory / name
        content = destination.read_bytes() if destination.exists() else b""
        if hashlib.sha256(content).hexdigest() != record[f"{prefix}_sha256"]:
            with urllib.request.urlopen(record[f"{prefix}_url"], timeout=20) as response:
                content = response.read()
            if hashlib.sha256(content).hexdigest() != record[f"{prefix}_sha256"]:
                raise RuntimeError("LLVM Python binding source hash mismatch")
            destination.write_bytes(content)


def provision_tmux_fixture(source: Path) -> None:
    """Copy an explicitly selected, hash-verified stock client into the isolated test fixture."""
    with LOCK.open("rb") as lockfile:
        record = tomllib.load(lockfile)["tmux_fixture"]
    with source.open("rb") as binary:
        digest = hashlib.file_digest(binary, "sha256").hexdigest()
    if digest != record["sha256"]:
        raise RuntimeError("stock fixture differs from the locked Ubuntu tmux package")
    destination = BUILD / "fixtures/tmux-3.4/bin/tmux"
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination)


def provision_modern_tmux_fixture(lock: dict[str, object]) -> None:
    """Build the locked upstream reference with isolated sources, tools, and installation output."""
    record, compiler = lock["modern_tmux_fixture"], lock["llvm"]
    assert isinstance(record, dict) and isinstance(compiler, dict)
    producer = {
        "revision": record["revision"],
        "source_sha256": record["sha256"],
        "llvm_sha256": compiler["sha256"],
        "c_standard": "gnu17",
    }
    for name in ("autoconf", "automake", "make"):
        first = subprocess.check_output([name, "--version"], text=True).splitlines()[0]
        if not first.endswith(" " + record[name]):
            raise RuntimeError(f"{name}: reference fixture requires locked version {record[name]}")
        producer[name] = first
    destination = BUILD / "fixtures/tmux-3.7"
    stamp = destination / "producer.json"
    binary = destination / "bin/tmux"
    if stamp.exists() and binary.exists():
        existing = json.loads(stamp.read_text())
        if all(existing.get(key) == value for key, value in producer.items()):
            with binary.open("rb") as source:
                if hashlib.file_digest(source, "sha256").hexdigest() == existing["binary_sha256"]:
                    return
    provision_archive("tmux_modern_fixture", record)
    source_directory = BUILD / "tools/tmux_modern_fixture"
    environment = dict(os.environ, CC=str(BUILD / "tools/llvm/bin/clang") + " -std=gnu17")
    destination.mkdir(parents=True, exist_ok=True)
    run(["sh", "autogen.sh"], cwd=source_directory, env=environment)
    run(["./configure", f"--prefix={destination}"], cwd=source_directory, env=environment)
    run(["make", "-j4"], cwd=source_directory, env=environment)
    run(["make", "install"], cwd=source_directory, env=environment)
    version = subprocess.check_output([str(binary), "-V"], text=True).strip()
    if version != "tmux " + record["version"]:
        raise RuntimeError("reference fixture reports an unexpected release")
    with binary.open("rb") as source:
        producer["binary_sha256"] = hashlib.file_digest(source, "sha256").hexdigest()
    stamp.write_text(json.dumps(producer, indent=2) + "\n")


def main() -> int:
    """Separate explicit resolution and installation from ordinary build commands."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--resolve", action="store_true", help="explicitly update the native tool lock"
    )
    parser.add_argument(
        "--provision", action="store_true", help="download and verify the locked tools"
    )
    parser.add_argument(
        "--python", action="store_true", help="provision the locked Python workspace"
    )
    parser.add_argument("--node", action="store_true", help="install the locked npm graph")
    parser.add_argument(
        "--stock-fixture", type=Path, help="copy the hash-verified tmux test binary"
    )
    parser.add_argument(
        "--modern-fixture",
        action="store_true",
        help="build the locked modern stock reference fixture",
    )
    parser.add_argument("--just", action="store_true", help="provision the locked recipe runner")
    parser.add_argument(
        "--gcc", action="store_true", help="provision the locked Ubuntu GCC fallback"
    )
    args = parser.parse_args()
    os.chdir(ROOT)
    if args.resolve:
        resolve()
    if args.stock_fixture:
        provision_tmux_fixture(args.stock_fixture)
    if args.provision or args.python or args.node or args.modern_fixture or args.just or args.gcc:
        with LOCK.open("rb") as source:
            lock = tomllib.load(source)
        if args.provision:
            provision_clang_python(lock["clang_python"])
            for name in (
                "llvm",
                "llvm_runtime",
                "doxygen",
                "graphviz",
                "node_current",
                "node_lts",
                "npm",
                "googletest",
                "stop_slop",
                "just",
            ):
                provision_archive(name, lock[name])
            provision_deb_bundle("gcc", lock["gcc"])
        if args.python:
            provision_python(lock)
        if args.modern_fixture:
            provision_modern_tmux_fixture(lock)
        if args.just:
            provision_archive("just", lock["just"])
        if args.gcc:
            provision_deb_bundle("gcc", lock["gcc"])
        if args.node:
            nodebin = BUILD / "tools/node_current/bin"
            env = dict(
                os.environ,
                PATH=str(nodebin) + os.pathsep + os.environ["PATH"],
                npm_config_cache=str(BUILD / "cache/npm"),
            )
            run(
                [
                    str(nodebin / "node"),
                    str(BUILD / "tools/npm/bin/npm-cli.js"),
                    "ci",
                    "--ignore-scripts",
                ],
                cwd=ROOT / "bindings/node",
                env=env,
            )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, RuntimeError, subprocess.CalledProcessError) as error:
        print(f"bootstrap: {error}", file=sys.stderr)
        raise SystemExit(1) from error
