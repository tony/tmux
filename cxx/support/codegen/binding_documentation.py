"""Generate the C++ header that shares canonical briefs with language bindings."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from exposure import ROOT, binding_documentation, load, write


def main() -> None:
    """Write the shared brief header or reject drift in its generated contents."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    parser.add_argument(
        "--output",
        type=Path,
        default=ROOT / "_build/generated/binding_documentation.hpp",
    )
    arguments = parser.parse_args()
    write(arguments.output, binding_documentation(load()), check=arguments.check)


if __name__ == "__main__":
    main()
