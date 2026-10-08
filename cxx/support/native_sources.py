"""Discover first-party C++ inputs with explicit scopes for formatting and documentation."""

from __future__ import annotations

import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def installed_consumer_sources() -> list[Path]:
    """List C++ consumers whose real build must use the installed package."""
    return sorted((ROOT / "tests/installed").rglob("*.cpp"))


def native_sources() -> list[Path]:
    """List authored C++ production, binding, header, and test inputs."""
    files = subprocess.check_output(
        [
            "rg",
            "--files",
            "include",
            "src",
            "tests/cpp",
            "bindings/python/module.cpp",
            "bindings/node/addon.cpp",
            "-g",
            "*.cpp",
            "-g",
            "*.hpp",
        ],
        cwd=ROOT,
        text=True,
    ).splitlines()
    return [ROOT / filename for filename in sorted(files)] + installed_consumer_sources()
