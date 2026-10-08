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
