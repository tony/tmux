"""Generate the C++ operation contract without loading either language runtime."""

from __future__ import annotations

import argparse
from pathlib import Path

from exposure import compiled_contract, load, write


def main() -> None:
    """Emit compile assertions and the startup catalog, or reject generated drift."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--check", action="store_true")
    arguments = parser.parse_args()
    write(arguments.output, compiled_contract(load()), check=arguments.check)


if __name__ == "__main__":
    main()
