"""Preserve tool provenance through explicit dependency resolution."""

import hashlib
import json
import tomllib
from pathlib import Path

import pytest

import support.bootstrap as bootstrap


def test_nested_toolchain_provenance_survives_resolution() -> None:
    """Lock encoding preserves parser patches, tool digests, and platform fallback facts."""
    path = Path(__file__).resolve().parents[2] / "support/toolchain.lock.toml"
    original = tomllib.loads(path.read_text())
    assert original["libvterm"]["backports"]["csi_numeric_source"]
    assert original["gcc"]["fallback"]
    assert tomllib.loads(bootstrap.encode_toolchain(original)) == original


def test_deb_bundle_provisioning_verifies_and_stamps_every_package(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    """A locked Debian bundle extracts in stable order and skips only an identical manifest."""
    monkeypatch.setattr(bootstrap, "BUILD", tmp_path)
    downloads = tmp_path / "downloads"
    downloads.mkdir()
    packages: dict[str, dict[str, str]] = {}
    for name, content in (("compiler", b"compiler"), ("runtime", b"runtime")):
        filename = name + ".deb"
        (downloads / filename).write_bytes(content)
        packages[name] = {
            "filename": filename,
            "url": "https://packages.invalid/" + filename,
            "sha256": hashlib.sha256(content).hexdigest(),
        }
    commands: list[list[str]] = []

    def record_extract(arguments: list[str], **_: object) -> None:
        """Record deterministic extraction and materialize its destination for the stamp."""
        commands.append(arguments)
        Path(arguments[-1]).mkdir(parents=True, exist_ok=True)

    monkeypatch.setattr(bootstrap, "run", record_extract)
    bootstrap.provision_deb_bundle("gcc", {"packages": packages})
    assert [Path(command[2]).name for command in commands] == ["compiler.deb", "runtime.deb"]
    stamp = tmp_path / "tools/gcc/.deb-packages.json"
    assert json.loads(stamp.read_text()) == {
        "compiler.deb": packages["compiler"]["sha256"],
        "runtime.deb": packages["runtime"]["sha256"],
    }
    commands.clear()
    bootstrap.provision_deb_bundle("gcc", {"packages": packages})
    assert commands == []


def test_deb_bundle_provisioning_rejects_a_mismatched_package(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    """A cached Debian package with the wrong digest cannot enter the tool directory."""
    monkeypatch.setattr(bootstrap, "BUILD", tmp_path)
    downloads = tmp_path / "downloads"
    downloads.mkdir()
    (downloads / "compiler.deb").write_bytes(b"wrong")
    record: dict[str, object] = {
        "packages": {
            "compiler": {
                "filename": "compiler.deb",
                "url": "https://packages.invalid/compiler.deb",
                "sha256": hashlib.sha256(b"expected").hexdigest(),
            }
        }
    }
    with pytest.raises(RuntimeError, match="gcc/compiler: package hash mismatch"):
        bootstrap.provision_deb_bundle("gcc", record)


def test_python_provisioning_replaces_stale_uv_and_selects_locked_python(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    """Provisioning replaces stale uv and selects the locked workspace interpreter."""
    monkeypatch.setattr(bootstrap, "ROOT", tmp_path)
    monkeypatch.setattr(bootstrap, "BUILD", tmp_path / "_build")

    def available_uv(_name: str) -> str:
        """Locate the bootstrap executable independently of the stale project tool."""
        return "bootstrap-uv"

    monkeypatch.setattr(bootstrap.shutil, "which", available_uv)
    uv = tmp_path / "_build/tools/uv-env/bin/uv"
    uv.parent.mkdir(parents=True)
    uv.touch()
    (tmp_path / ".venv").mkdir()
    python = str(tmp_path / "_build/tools/python/bin/python3.15")
    commands: list[list[str]] = []
    environments: list[dict[str, str]] = []

    def capture(arguments: list[str], *, env: dict[str, str]) -> None:
        """Capture explicit provisioning commands without downloading or installing tools."""
        commands.append(arguments)
        environments.append(env)

    def installed_output(arguments: list[str], **_: object) -> str:
        """Report a stale uv and the requested managed Python executable."""
        return "uv 0.12.23\n" if arguments[-1] == "--version" else python + "\n"

    monkeypatch.setattr(bootstrap, "run", capture)
    monkeypatch.setattr(bootstrap.subprocess, "check_output", installed_output)
    bootstrap.provision_python({"uv": {"version": "0.13.0"}, "python": {"version": "3.15.0"}})
    assert any("uv==0.13.0" in command for command in commands)
    sync = next(command for command in commands if "sync" in command)
    assert sync[sync.index("--python") + 1] == python
    assert all(
        environment.get("UV_PYTHON_BIN_DIR") == str(tmp_path / "_build/tools/python-bin")
        for environment in environments
    )
