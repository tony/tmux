"""Generate strict Doxygen output and inventory callables with the locked C++ compiler."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shlex
import subprocess
import sys
import tomllib
from pathlib import Path

from documentation import brief_text, check, prose, python_contracts, self_test
from native_sources import installed_consumer_sources, native_sources

ROOT = Path(__file__).resolve().parents[1]
TOOLCHAIN = tomllib.loads((ROOT / "support/toolchain.lock.toml").read_text())
LLVM_MAJOR = TOOLCHAIN["llvm"]["version"].split(".", 1)[0]
LIBCLANG = ROOT / "_build/tools/llvm/lib/libclang.so"
if TOOLCHAIN["llvm"]["version"] != TOOLCHAIN["clang_python"]["version"]:
    raise RuntimeError("compiler inventory requires matching LLVM and Python binding versions")
sys.path.insert(0, str(ROOT / "_build/tools/clang_python"))
from clang.cindex import (  # noqa: E402 - load the hash-locked LLVM bindings
    Config,
    Cursor,
    CursorKind,
    Diagnostic,
    Index,
)

FUNCTIONS = {
    CursorKind.FUNCTION_DECL,
    CursorKind.CXX_METHOD,
    CursorKind.CONSTRUCTOR,
    CursorKind.DESTRUCTOR,
    CursorKind.CONVERSION_FUNCTION,
    CursorKind.FUNCTION_TEMPLATE,
    CursorKind.LAMBDA_EXPR,
}
PREFIXES = (
    "include/",
    "src/",
    "bindings/python/",
    "bindings/node/",
    "tests/cpp/",
    "tests/installed/",
)
GENERATED_CONTRACT = ROOT / "_build/dev/generated/exposure.cpp"


def node_sources() -> list[str]:
    """Discover authored Node and TypeScript documentation inputs with project-scoped rg."""
    return subprocess.check_output(
        [
            "rg",
            "--files",
            "bindings/node",
            "-g",
            "!node_modules/**",
            "-g",
            "!dist/**",
            "-g",
            "*.cjs",
            "-g",
            "*.mjs",
            "-g",
            "*.js",
            "-g",
            "*.cts",
            "-g",
            "*.mts",
            "-g",
            "*.ts",
        ],
        cwd=ROOT,
        text=True,
    ).splitlines()


def owned(path: Path) -> bool:
    """Include authored native callables and compiled exposure; exclude dependency trees."""
    return path == GENERATED_CONTRACT or (
        path.is_relative_to(ROOT)
        and not {"node_modules", "_build", ".venv"}.intersection(path.relative_to(ROOT).parts)
        and path.relative_to(ROOT).as_posix().startswith(PREFIXES)
    )


def compiler_arguments(record: dict[str, str]) -> list[str]:
    """Expand CMake response files and retain actual flags for compiler-derived documentation."""
    arguments = shlex.split(record["command"])[1:]
    expanded = []
    for argument in arguments:
        if argument.startswith("@"):
            expanded += shlex.split((Path(record["directory"]) / argument[1:]).read_text())
        else:
            expanded.append(argument)
    result = []
    skip = False
    for argument in expanded:
        if skip:
            skip = False
        elif argument == "-o":
            skip = True
        elif argument not in ("-c", record["file"]):
            result.append(argument)
    return [*result, f"-resource-dir={ROOT / '_build/tools/llvm/lib/clang' / LLVM_MAJOR}"]


def compiler_inventory_records(records: list[dict[str, str]]) -> list[dict[str, str]]:
    """Add installed consumers to documentation parsing with real project compiler flags."""
    baseline = next(record for record in records if record["file"].endswith("/src/main.cpp"))
    compiler = shlex.split(baseline["command"])[0]
    external = [
        dict(
            directory=baseline["directory"],
            file=str(source),
            command=shlex.join([compiler, *compiler_arguments(baseline), "-c", str(source)]),
        )
        for source in installed_consumer_sources()
    ]
    return [*records, *external]


def comment(cursor: Cursor) -> str:
    """Resolve canonical declarations and adjacent lambda briefs."""
    if cursor.kind != CursorKind.LAMBDA_EXPR and next(cursor.get_tokens(), None) is not None:
        return cursor.raw_comment or cursor.canonical.raw_comment or ""
    location = cursor.extent.start if cursor.kind == CursorKind.LAMBDA_EXPR else cursor.location
    prefix = Path(str(location.file)).read_bytes()[: location.offset].decode()
    match = re.search(
        r"(/\*\*(?:(?!\*/).)*\*/)\s*(?:(?:const\s+)?auto\s+\w+\s*=\s*)?\+?$",
        prefix,
        re.DOTALL,
    )
    return match[1] if match else ""


def qualified_name(cursor: Cursor) -> str:
    """Retain namespace and class ownership when resolving documented overload references."""
    names = []
    current = cursor
    while current.kind != CursorKind.TRANSLATION_UNIT:
        if current.spelling:
            names.append(current.spelling)
        current = current.semantic_parent
        if current is None:
            break
    return "::".join(reversed(names))


def inventory(
    records: list[dict[str, str]], *, probe: Path | None = None
) -> list[dict[str, object]]:
    """Collect explicit C++ functions, private helpers, operators, templates, and lambdas."""
    if not Config.loaded:
        Config.set_library_file(str(LIBCLANG))
    index = Index.create()
    callables: dict[str, dict[str, object]] = {}
    paths: dict[str, Path] = {}
    covered: set[Path] = set()

    def visit(cursor: Cursor) -> None:
        """Prune third-party branches and deduplicate function redeclarations."""
        location = cursor.location
        if location.file:
            filename = str(location.file)
            if filename not in paths:
                paths[filename] = Path(filename).resolve()
            path = paths[filename]
            if not owned(path) and path != probe:
                return
            if cursor.kind in FUNCTIONS:
                tokens = list(cursor.get_tokens())
                if cursor.kind != CursorKind.LAMBDA_EXPR and not tokens:
                    source = path.read_bytes()[cursor.location.offset : cursor.location.offset + 20]
                    authored_body = cursor.is_definition() and (
                        (cursor.spelling == "TestBody" and source.startswith(b"TEST("))
                        or (
                            cursor.spelling.endswith("exec_impl")
                            and source.startswith(b"NB_MODULE(")
                        )
                    )
                    if not authored_body:
                        return
                key = cursor.get_usr() or f"{path}:{cursor.extent.start.offset}"
                if cursor.kind == CursorKind.FUNCTION_DECL and cursor.spelling in (
                    "main",
                    "LLVMFuzzerTestOneInput",
                ):
                    # Independent executable and fuzz entrypoints share one Clang identity.
                    key += ":" + record["file"]
                documentation = comment(cursor)
                entry = {
                    "file": str(path.relative_to(ROOT)),
                    "line": cursor.location.line,
                    "symbol": cursor.displayname or "lambda",
                    "kind": cursor.kind.name,
                    "documentation": documentation,
                    "qualified": qualified_name(cursor),
                    "parameters": [
                        child.spelling
                        for child in cursor.get_children()
                        if child.kind == CursorKind.PARM_DECL
                    ],
                    "template_parameters": [
                        child.spelling
                        for child in cursor.get_children()
                        if child.kind
                        in (
                            CursorKind.TEMPLATE_TYPE_PARAMETER,
                            CursorKind.TEMPLATE_NON_TYPE_PARAMETER,
                            CursorKind.TEMPLATE_TEMPLATE_PARAMETER,
                        )
                    ],
                    "signature": qualified_name(cursor)
                    + "("
                    + ",".join(arg.type.spelling for arg in (cursor.get_arguments() or ()))
                    + ")",
                    "key": key,
                }
                if key not in callables or documentation:
                    callables[key] = entry
        for child in cursor.get_children():
            visit(child)

    for record in records:
        unit = index.parse(record["file"], args=compiler_arguments(record))
        covered.add(Path(record["file"]).resolve())
        for included in unit.get_includes():
            filename = str(included.include)
            if filename.startswith(str(ROOT) + os.sep):
                if filename not in paths:
                    paths[filename] = Path(filename).resolve()
                covered.add(paths[filename])
        failures = [str(d) for d in unit.diagnostics if d.severity >= Diagnostic.Error]
        if failures:
            raise RuntimeError("compiler inventory cannot parse source:\n" + "\n".join(failures))
        visit(unit.cursor)
    if not callables:
        raise RuntimeError("compiler callable inventory is unexpectedly empty")
    if probe is None:
        missing = set(native_sources()) - covered
        if missing:
            raise RuntimeError(
                "authored native files were never compiled: "
                + ", ".join(str(path.relative_to(ROOT)) for path in sorted(missing))
            )

    def location_order(item: dict[str, object]) -> tuple[str, int]:
        """Keep the callable report ordered by source file and line."""
        return str(item["file"]), int(str(item["line"]))

    return sorted(callables.values(), key=location_order)


def cached_inventory(records: list[dict[str, str]]) -> list[dict[str, object]]:
    """Reuse compiler output when native inputs, flags, tool locks, and checking code match."""
    fingerprint = hashlib.sha256(json.dumps(records, sort_keys=True).encode())
    for record in records:
        fingerprint.update(json.dumps(compiler_arguments(record)).encode())
    for tool in (
        ROOT / "_build/tools/llvm/bin/clang++",
        LIBCLANG,
        ROOT / "_build/tools/clang_python/clang/cindex.py",
    ):
        identity = tool.stat()
        fingerprint.update(
            repr(
                (
                    identity.st_dev,
                    identity.st_ino,
                    identity.st_size,
                    identity.st_mtime_ns,
                    identity.st_ctime_ns,
                )
            ).encode()
        )
    inputs = [
        *native_sources(),
        Path(__file__),
        ROOT / "support/toolchain.lock.toml",
        ROOT / "uv.lock",
        ROOT / "bindings/node/package-lock.json",
        ROOT / "_build/dev/generated/binding_documentation.hpp",
        GENERATED_CONTRACT,
    ]
    for path in inputs:
        fingerprint.update(str(path.relative_to(ROOT)).encode())
        fingerprint.update(path.read_bytes())
    cache = ROOT / "_build/reports/compiler-inventory.json"
    key = fingerprint.hexdigest()
    if cache.exists():
        stored = json.loads(cache.read_text())
        if stored["fingerprint"] == key:
            return stored["callables"]
    entries = inventory(records)
    cache.write_text(json.dumps(dict(fingerprint=key, callables=entries), indent=2) + "\n")
    return entries


def cpp_contracts(entries: list[dict[str, object]]) -> list[dict[str, object]]:
    """Resolve copied briefs and reject ambiguous references or nonexistent parameter tags."""
    by_name: dict[str, list[dict[str, object]]] = {}
    by_signature: dict[str, list[dict[str, object]]] = {}
    for entry in entries:
        if entry["kind"] != "LAMBDA_EXPR":
            by_name.setdefault(str(entry["qualified"]), []).append(entry)
            by_signature.setdefault(str(entry["signature"]).replace(" ", ""), []).append(entry)

    def resolve(entry: dict[str, object], chain: frozenset[str] = frozenset()) -> str:
        """Reject copy cycles and require one compiler-visible target for each copied brief."""
        key, document = str(entry["key"]), str(entry["documentation"])
        if key in chain:
            raise RuntimeError(f"{entry['file']}:{entry['line']}: copied brief forms a cycle")
        for match in re.finditer(r"[@\\]copy(brief|doc)\s+([\w:]+(?:\([^)]*\))?)", document):
            reference = match[2]
            candidates = (
                by_signature.get(reference.replace(" ", ""), [])
                if "(" in reference
                else by_name.get(reference, [])
            )
            if len(candidates) != 1:
                raise RuntimeError(
                    f"{entry['file']}:{entry['line']}: "
                    f"unresolved or ambiguous copy{match[1]} {reference}"
                )
            target = resolve(candidates[0], chain | {key})
            document = document.replace(
                match[0], brief_text(target) if match[1] == "brief" else prose(target)
            )
        return document

    resolved = []
    for entry in entries:
        document = resolve(entry)
        for tag, field in (("param", "parameters"), ("tparam", "template_parameters")):
            parameters = entry[field]
            if not isinstance(parameters, list):
                raise RuntimeError("compiler parameter inventory is malformed")
            for parameter in re.findall(rf"[@\\]{tag}(?:\[[^\]]+\])?\s+(\w+)", document):
                if parameter not in parameters:
                    raise RuntimeError(
                        f"{entry['file']}:{entry['line']}: "
                        f"{tag} names nonexistent parameter {parameter}"
                    )
        resolved.append(
            dict(
                entry,
                documentation=document,
                copied=bool(re.search(r"[@\\]copy(?:brief|doc)\b", str(entry["documentation"]))),
            )
        )
    return resolved


def main() -> None:
    """Build documentation from real compiler settings and reject missing callable briefs."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inventory-only", action="store_true")
    arguments = parser.parse_args()
    os.chdir(ROOT)
    configured_records = [
        r
        for r in json.loads((ROOT / "_build/dev/compile_commands.json").read_text())
        if owned(Path(r["file"]))
    ]
    records = compiler_inventory_records(configured_records)
    directory = ROOT / "_build/docs/compiler"
    directory.mkdir(parents=True, exist_ok=True)
    database = [
        dict(
            r,
            arguments=[
                str(ROOT / "_build/tools/llvm/bin/clang++"),
                *compiler_arguments(r),
                "-c",
                r["file"],
            ],
        )
        for r in records
    ]
    for record in database:
        record.pop("command")
    (directory / "compile_commands.json").write_text(json.dumps(database, indent=2) + "\n")
    configuration = ROOT / "_build/docs/Doxyfile"
    configuration.write_text("@INCLUDE = support/Doxyfile\n")
    if not arguments.inventory_only:
        subprocess.run(
            [str(ROOT / "_build/tools/doxygen/bin/doxygen"), str(configuration)], check=True
        )
    callables = cached_inventory(records)
    (ROOT / "_build/reports/callables.json").write_text(json.dumps(callables, indent=2) + "\n")
    self_test()
    check(cpp_contracts(callables))
    check(python_contracts())
    node_contracts = subprocess.check_output(
        [
            str(ROOT / "_build/tools/node_current/bin/node"),
            "bindings/node/tools/documentation.cjs",
            *node_sources(),
        ],
        text=True,
    )
    check(json.loads(node_contracts))
    print(f"PASS compiler inventory: {len(callables)} documented callables")


if __name__ == "__main__":
    main()
