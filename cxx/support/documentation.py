"""Check concise function contracts without rewriting code, identifiers, quotations, or licenses."""

from __future__ import annotations

import ast
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
STOP_SLOP = "https://github.com/hardikpandya/stop-slop/blob/8da1f030185bdfe8471220585162991eaeb970e9/SKILL.md"
FILLER = re.compile(
    r"\b(?:delve|leverage|seamlessly|game.changer|it.s worth noting|at its core)\b", re.IGNORECASE
)


def prose(document: str) -> str:
    """Remove documentation markup and literal code while retaining the contract's prose."""
    text = re.sub(r"\\code.*?\\endcode|```.*?```|`[^`]+`", "", document, flags=re.DOTALL)
    text = text.removeprefix("/**").removesuffix("*/")
    text = re.sub(r"(?m)^\s*\* ?", "", text)
    text = re.sub(r'"(?:\\.|[^"\\])*"|(?<!\w)\x27[^\x27\n]+\x27(?!\w)', "", text)
    text = re.sub(r"(?m)^\s*>.*$", "", text)
    text = re.sub(r"[@\\]brief\s+", "", text)
    return text.strip()


def brief_text(document: str) -> str:
    """Read the contract brief separately from tags and longer shared invariants."""
    return re.split(r"\n\s*\n|\n\s*[@\\]", prose(document), maxsplit=1)[0].strip()


def violations(document: str) -> list[str]:
    """Reject empty, oversized, multi-sentence, or filler-heavy function contracts."""
    if not document.strip():
        return ["missing brief"]
    text = prose(document)
    brief = brief_text(document)
    words = re.findall(r"\b[A-Za-z]+(?:'[a-z]+)?\b", brief)
    failures = []
    if not words:
        failures.append("empty brief")
    if len(words) > 25:
        failures.append("brief exceeds 25 words")
    if len(re.findall(r"\b[A-Za-z]+(?:'[a-z]+)?\b", text)) > 100:
        failures.append("function contract exceeds 100 words")
    parts = re.split(r"\n\s*\n|\n(?=\s*[@\\])", text, maxsplit=1)
    details = parts[1].splitlines() if len(parts) > 1 else []
    contracts = [
        line
        for line in details
        if line.strip()
        and not re.fullmatch(r"\s*[@\\](?:param|returns?)\s+\{.*\}(?:\s+[\w.[\]=]+)?\s*", line)
    ]
    if len(contracts) > 2:
        failures.append("function contract exceeds two detail lines")
    if re.search(r"[.!?]\s+[A-Z]", brief):
        failures.append("brief contains multiple sentences")
    if FILLER.search(text):
        failures.append("selected stop-slop filler")
    if re.search(r"\b(?:docs|slop)[-_: ](?:ignore|disable|suppress)\b", text, re.IGNORECASE):
        failures.append("unapproved documentation suppression")
    return failures


def python_contracts(paths: tuple[Path, ...] | None = None) -> list[dict[str, object]]:
    """Inventory Python functions and generated stub methods through their syntax trees."""
    entries: list[dict[str, object]] = []
    if paths is None:
        files = subprocess.check_output(
            ["rg", "--files", "support", "tests", "bindings/python", "-g", "*.py", "-g", "*.pyi"],
            cwd=ROOT,
            text=True,
        ).splitlines()
        paths = tuple(ROOT / filename for filename in sorted(files))
    for path in paths:
        source = path.read_text()
        for node in ast.walk(ast.parse(source, filename=str(path))):
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
                entries.append(
                    {
                        "file": str(path.relative_to(ROOT)),
                        "line": node.lineno,
                        "symbol": node.name,
                        "documentation": ast.get_docstring(node) or "",
                    }
                )
            elif isinstance(node, ast.Lambda):
                entries.append(
                    {
                        "file": str(path.relative_to(ROOT)),
                        "line": node.lineno,
                        "symbol": "lambda",
                        "documentation": "",
                    }
                )
    return entries


def check(entries: list[dict[str, object]]) -> None:
    """Report function-specific contract failures and reject an unexpectedly empty inventory."""
    if not entries:
        raise RuntimeError("function documentation inventory is unexpectedly empty")
    failures = 0
    contracts: dict[str, tuple[str, int]] = {}
    for entry in entries:
        document = str(entry["documentation"])
        problems = violations(document)
        brief = brief_text(document)
        generated = str(entry["file"]).endswith((".pyi", "index.d.ts"))
        copied = bool(entry.get("copied"))
        if not generated and not copied and len(brief.split()) >= 12:
            previous = contracts.get(brief)
            if previous is not None:
                problems.append(f"duplicated contract from {previous[0]}:{previous[1]}")
            contracts[brief] = str(entry["file"]), int(str(entry["line"]))
        for failure in problems:
            print(f"{entry['file']}:{entry['line']}: {entry['symbol']}: {failure}")
            failures += 1
    if failures:
        raise RuntimeError(f"{failures} function documentation violations")


def self_test() -> None:
    """Prove brief-only contracts pass and deliberate omissions, filler, and bloat fail."""
    assert violations("Return retained bytes or report an output gap.") == []
    assert violations("/** @brief Await bytes within 1..750 milliseconds. */") == []
    assert "missing brief" in violations("")
    assert "empty brief" in violations("/** @brief */")
    assert "brief exceeds 25 words" in violations("word " * 26)
    assert "function contract exceeds 100 words" in violations("Brief.\n\n" + "word " * 100)
    assert "brief contains multiple sentences" in violations("Return bytes. Destroy the session.")
    assert "selected stop-slop filler" in violations("Seamlessly leverage the API.")
    assert violations('Preserve the protocol literal "leverage" and `seamlessly` bytes.') == []
    assert "unapproved documentation suppression" in violations("Return bytes.\n\nslop-ignore")
    assert "function contract exceeds two detail lines" in violations(
        "Return bytes.\n@note Preserve ownership.\n@note Bound traffic.\n@note Declare errors."
    )


if __name__ == "__main__":
    self_test()
    check(python_contracts())
