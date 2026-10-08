"""Prove C++ callable documentation gates with controlled negative fixtures."""

from __future__ import annotations

import io
import json
import shlex
import subprocess
import xml.etree.ElementTree as ET
from contextlib import redirect_stdout
from pathlib import Path
from unittest.mock import patch

from check_documentation import (
    ROOT,
    cached_inventory,
    compiler_arguments,
    compiler_inventory_records,
    cpp_contracts,
    inventory,
    owned,
)
from documentation import check, python_contracts
from native_sources import native_sources

SOURCE = r"""
#include <expected>
#include <functional>
#include <span>

/** @brief Controlled compiler and documentation extraction fixtures. */
namespace docs_probe {
/** @brief Accept terminal values with an equality comparison. */
template<class T> concept TerminalValue = requires(T value) { value == value; };
/** @brief Exercise public and private function extraction. */
class Probe {
public:
    /** @brief Return a terminal count through an explicit result. */
    std::expected<int, int> public_operation(int count) { return private_operation(count); }
private:
    /** @brief Preserve the terminal count before static conversion. */
    int private_operation(int count) { return static_operation(count); }
    /** @brief Preserve the count without accessing instance state. */
    static int static_operation(int count) { return count; }
};
/** @brief Return the supplied cell count. */
int overloaded(int count) { return count; }
/** @brief Return the number of retained terminal bytes. */
int overloaded(std::span<const char> bytes) { return static_cast<int>(bytes.size()); }
/** @copybrief docs_probe::overloaded(int) */
int copied(int count) { return overloaded(count); }
/** @brief Return a move-owned terminal continuation. */
std::move_only_function<int()> continuation() {
    return /** @brief Return the terminal continuation's cell count. */ [] { return 1; };
}
}
"""


def native_probe(source: str, expected: str | None) -> None:
    """Require a valid C++23 fixture to pass or fail for its specified documentation defect."""
    directory = ROOT / "_build/docs/probes"
    directory.mkdir(parents=True, exist_ok=True)
    path = directory / "callables.cpp"
    path.write_text(source)
    records = json.loads((ROOT / "_build/dev/compile_commands.json").read_text())
    baseline = next(r for r in records if r["file"].endswith("src/protocol/native/framing.cpp"))
    record = dict(
        directory=baseline["directory"],
        file=str(path),
        command=shlex.join(
            [
                str(ROOT / "_build/tools/llvm/bin/clang++"),
                *compiler_arguments(baseline),
                "-c",
                str(path),
            ]
        ),
    )
    output = io.StringIO()
    try:
        with redirect_stdout(output):
            check(cpp_contracts(inventory([record], probe=path)))
    except RuntimeError as error:
        details = output.getvalue() + str(error)
        if expected is None or expected not in details:
            raise RuntimeError(
                "native documentation probe failed for an unexpected reason"
            ) from error
    else:
        if expected is not None:
            raise RuntimeError(
                f"native documentation gate accepted the deliberate defect: {expected}"
            )


def doxygen_probe(source: str, name: str, expected: str | None) -> None:
    """Prove strict Doxygen tags and extraction of C++23 results and continuations."""
    directory = ROOT / "_build/docs/probes" / name
    directory.mkdir(parents=True, exist_ok=True)
    path = directory / "callables.cpp"
    path.write_text(source)
    configuration = directory / "Doxyfile"
    warnings = directory / "warnings.log"
    configuration.write_text(
        f"@INCLUDE = {ROOT / 'support/Doxyfile'}\nINPUT = {path}\n"
        f"OUTPUT_DIRECTORY = {directory}\nWARN_LOGFILE = {warnings}\nSOURCE_BROWSER = NO\n"
    )
    completed = subprocess.run(
        [str(ROOT / "_build/tools/doxygen/bin/doxygen"), str(configuration)],
        cwd=ROOT,
        capture_output=True,
        text=True,
        timeout=10,
    )
    if expected is not None:
        details = warnings.read_text() + completed.stdout + completed.stderr
        if completed.returncode == 0 or expected not in details:
            raise RuntimeError(
                f"Doxygen accepted a deliberate {name} defect or failed for another reason"
            )
        return
    if completed.returncode != 0 or warnings.read_text():
        raise RuntimeError("positive Doxygen brief-only fixture failed: " + warnings.read_text())
    index = ET.parse(directory / "xml/index.xml")
    concepts = [
        element.findtext("name") for element in index.findall(".//compound[@kind='concept']")
    ]
    if "docs_probe::TerminalValue" not in concepts:
        raise RuntimeError("Doxygen omitted the documented C++23 concept")
    types = []
    for filename in ("classdocs__probe_1_1Probe.xml", "namespacedocs__probe.xml"):
        tree = ET.parse(directory / "xml" / filename)
        for element in tree.findall(".//memberdef/type"):
            types.append("".join(element.itertext()))
    if not any("std::expected" in value for value in types):
        raise RuntimeError("Doxygen omitted the actual std::expected result type")
    if not any("std::move_only_function" in value for value in types):
        raise RuntimeError("Doxygen omitted the actual move-owned continuation type")


def language_probe(language: str, source: str, expected: str | None) -> None:
    """Check real Python and Node syntax trees for the specified documentation defect."""
    directory = ROOT / "_build/docs/probes/languages"
    directory.mkdir(parents=True, exist_ok=True)
    path = directory / ("functions.py" if language == "python" else "functions.cts")
    path.write_text(source)
    entries = (
        python_contracts((path,))
        if language == "python"
        else json.loads(
            subprocess.check_output(
                [
                    str(ROOT / "_build/tools/node_current/bin/node"),
                    "bindings/node/tools/documentation.cjs",
                    str(path),
                ],
                cwd=ROOT,
                text=True,
            )
        )
    )
    output = io.StringIO()
    try:
        with redirect_stdout(output):
            check(entries)
    except RuntimeError as error:
        if expected is None or expected not in output.getvalue() + str(error):
            raise RuntimeError("language documentation probe failed unexpectedly") from error
    else:
        if expected is not None:
            raise RuntimeError(f"{language} accepted the deliberate defect: {expected}")


def callback_boundary_probe() -> None:
    """Prove the real Node addon cannot compile without complete callback exception translation."""
    records = json.loads((ROOT / "_build/dev/compile_commands.json").read_text())
    record = next(r for r in records if r["file"].endswith("bindings/node/addon.cpp"))
    completed = subprocess.run(
        [
            str(ROOT / "_build/tools/llvm/bin/clang++"),
            *compiler_arguments(record),
            "-UNODE_ADDON_API_CPP_EXCEPTIONS_ALL",
            "-fsyntax-only",
            record["file"],
        ],
        cwd=record["directory"],
        text=True,
        capture_output=True,
        timeout=10,
    )
    if completed.returncode == 0 or (
        "Node callback boundaries must translate all C++ exceptions" not in completed.stderr
    ):
        raise RuntimeError("the real Node callback exception gate accepted its deliberate break")
    (ROOT / "_build/reports/node-callback-negative.log").write_text(completed.stderr)


def entrypoint_probe() -> None:
    """Keep each executable and fuzz entrypoint despite Clang's shared function identity."""
    sources = {ROOT / "src/main.cpp", ROOT / "tests/cpp/sanitizer_probe.cpp"}
    sources.update(path for path in native_sources() if path.parent == ROOT / "tests/cpp/fuzz")
    records = [
        record
        for record in json.loads((ROOT / "_build/dev/compile_commands.json").read_text())
        if record["file"] in {str(path) for path in sources}
    ]
    entries = inventory(records, probe=ROOT / "src/main.cpp")
    actual = {
        str(entry["file"])
        for entry in entries
        if str(entry["symbol"]).split("(", 1)[0] in {"main", "LLVMFuzzerTestOneInput"}
    }
    expected = {str(path.relative_to(ROOT)) for path in sources}
    if actual != expected:
        raise RuntimeError("compiler inventory merged separate executable entrypoints")


def generated_contract_probe() -> None:
    """Require canonical documentation coverage for compiled generated functions."""
    path = ROOT / "_build/dev/generated/exposure.cpp"
    if not owned(path):
        raise RuntimeError("compiler inventory excluded first-party generated functions")
    records = json.loads((ROOT / "_build/dev/compile_commands.json").read_text())
    record = next(record for record in records if record["file"] == str(path))
    entries = inventory([record], probe=ROOT / "src/main.cpp")
    definitions = [
        entry
        for entry in entries
        if entry["qualified"] == "tmux_cxx::protocol::native::api_operations"
    ]
    if len(definitions) != 1 or definitions[0]["file"] != str(path.relative_to(ROOT)):
        raise RuntimeError("compiler inventory omitted or duplicated the generated definition")
    check(cpp_contracts(definitions))


def compiler_flags_cache_probe() -> None:
    """Require changed response-file flags to invalidate cached compiler coverage."""
    records = compiler_inventory_records(
        [
            record
            for record in json.loads((ROOT / "_build/dev/compile_commands.json").read_text())
            if owned(Path(record["file"]))
        ]
    )
    record = next(record for record in records if "@" in record["command"])
    response = next(
        argument[1:] for argument in shlex.split(record["command"]) if argument.startswith("@")
    )
    response_path = Path(record["directory"]) / response
    cached_inventory(records)
    original_read = Path.read_text

    def changed_flags(path: Path, encoding: str | None = None, errors: str | None = None) -> str:
        """Change one observed compiler define without modifying the response file."""
        content = original_read(path, encoding=encoding, errors=errors)
        return (
            content + "\n-DTMUX_CXX_DOCUMENTATION_CACHE_PROBE=1\n"
            if path == response_path
            else content
        )

    def require_parse(_records: list[dict[str, str]]) -> list[dict[str, object]]:
        """Signal that changed flags required a fresh parse without writing inventory artifacts."""
        raise RuntimeError("controlled compiler cache invalidation")

    with (
        patch("check_documentation.inventory", require_parse),
        patch.object(Path, "read_text", changed_flags),
    ):
        try:
            cached_inventory(records)
        except RuntimeError as error:
            if str(error) != "controlled compiler cache invalidation":
                raise
        else:
            raise RuntimeError(
                "compiler inventory reused cached coverage after response flags changed"
            )


def main() -> None:
    """Prove C++23 extraction and reject missing, malformed, or bloated contracts."""
    native_probe(SOURCE, None)
    entrypoint_probe()
    generated_contract_probe()
    compiler_flags_cache_probe()
    doxygen_probe(SOURCE, "positive", None)
    briefs = (
        "Return a terminal count through an explicit result.",
        "Preserve the terminal count before static conversion.",
        "Preserve the count without accessing instance state.",
        "Return the terminal continuation's cell count.",
    )
    for brief in briefs:
        native_probe(SOURCE.replace(f"/** @brief {brief} */", ""), "missing brief")
    copied = SOURCE.replace("docs_probe::overloaded(int)", "docs_probe::overloaded")
    native_probe(copied, "unresolved or ambiguous copybrief")
    native_probe(
        SOURCE.replace("docs_probe::overloaded(int)", "docs_probe::missing"),
        "unresolved or ambiguous copybrief",
    )
    full_contract = SOURCE.replace("@copybrief", "@copydoc")
    native_probe(full_contract, None)
    native_probe(
        full_contract.replace("docs_probe::overloaded(int)", "docs_probe::missing"),
        "unresolved or ambiguous copydoc",
    )
    native_probe(
        full_contract.replace(
            "Return the supplied cell count.",
            "Return the supplied cell count.\n@param wrong A missing parameter.",
        ),
        "nonexistent parameter wrong",
    )
    wrong_parameter = SOURCE.replace(briefs[0], briefs[0] + "\n @param wrong Number of cells.")
    native_probe(wrong_parameter, "nonexistent parameter wrong")
    doxygen_probe(wrong_parameter, "wrong-parameter", "not found in the argument list")
    bad_reference = SOURCE.replace("docs_probe::overloaded(int)", "docs_probe::missing")
    doxygen_probe(bad_reference, "unresolved-reference", "not found")
    native_probe(SOURCE.replace(briefs[0], "word " * 26), "brief exceeds 25 words")
    native_probe(
        SOURCE.replace(
            briefs[0],
            briefs[0] + "\n@note Preserve ownership.\n@note Bound traffic.\n@note Declare errors.",
        ),
        "function contract exceeds two detail lines",
    )
    native_probe(
        SOURCE.replace(briefs[0], "Seamlessly leverage this operation."),
        "selected stop-slop filler",
    )
    native_probe(
        SOURCE.replace(briefs[0], briefs[0] + "\n\nslop-ignore"),
        "unapproved documentation suppression",
    )
    duplicate = (
        "Return the retained terminal cell count after preserving "
        "this caller's explicit parser continuation."
    )
    native_probe(
        SOURCE.replace(briefs[0], duplicate).replace(briefs[1], duplicate), "duplicated contract"
    )
    python = 'def pane_count() -> int:\n    """Return the selected pane count."""\n    return 1\n'
    node = "/** Return the selected pane count. */\nfunction paneCount(): number { return 1; }\n"
    for language, source in (("python", python), ("node", node)):
        language_probe(language, source, None)
        language_probe(
            language,
            source.replace("Return the selected pane count.", ""),
            "missing brief" if language == "python" else "empty brief",
        )
        language_probe(
            language,
            source.replace(
                "Return the selected pane count.", "Seamlessly leverage the pane count."
            ),
            "selected stop-slop filler",
        )
    language_probe("python", "value = lambda: 1\n", "missing brief")
    language_probe("node", "const value = () => 1;\n", "missing brief")
    callback_boundary_probe()
    (ROOT / "_build/reports/documentation-probes.json").write_text(
        json.dumps(
            dict(
                passed=True,
                missing=["public", "private", "static", "lambda"],
                negative=[
                    "ambiguous overload",
                    "unresolved reference",
                    "wrong parameter",
                    "brief budget",
                    "detail line budget",
                    "filler",
                    "suppression",
                    "duplicate",
                ],
                positive=[
                    "brief only",
                    "concept",
                    "std::expected",
                    "std::move_only_function",
                    "compiled generated function",
                    "response-file cache invalidation",
                ],
                language_ast=["python", "node", "missing lambda", "filler"],
                callback_boundary="Node exception macro rejected when deliberately undefined",
            ),
            indent=2,
        )
        + "\n"
    )
    print("PASS native documentation controlled negatives and C++23 extraction")


if __name__ == "__main__":
    main()
