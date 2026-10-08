"""Read the binding exposure contract and validate it against C++ during compilation."""

from __future__ import annotations

import json
import re
import tomllib
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
KINDS = {
    "text": ("std::string", "string"),
    "bytes": ("std::string", "Buffer"),
    "session_id": ("SessionId", "bigint"),
    "window_id": ("WindowId", "bigint"),
    "window_link_id": ("WindowLinkId", "bigint"),
    "pane_id": ("PaneId", "bigint"),
    "pane_split_orientation": (
        "PaneSplitOrientation",
        '"left_right" | "top_bottom"',
    ),
    "client_id": ("ClientId", "bigint"),
    "optional_session_id": ("std::optional<SessionId>", "bigint | null"),
    "revision": ("std::uint64_t", "bigint"),
    "milliseconds": ("std::uint32_t", "number"),
    "index": ("std::uint32_t", "number"),
    "session": ("SessionSnapshot", "SessionSnapshot"),
    "sessions": ("std::vector<SessionSnapshot>", "SessionSnapshot[]"),
    "session_observation": ("SessionObservation", "SessionObservation"),
    "session_event_cursor": ("SessionEventCursor", "SessionEventCursor"),
    "session_event": ("SessionEvent", "SessionEvent"),
    "session_event_records": ("std::vector<SessionEventRecord>", "SessionEventRecord[]"),
    "session_event_batch": ("SessionEventBatch", "SessionEventBatch"),
    "window_link": ("WindowLinkSnapshot", "WindowLinkSnapshot"),
    "window_links": ("std::vector<WindowLinkSnapshot>", "WindowLinkSnapshot[]"),
    "window": ("WindowSnapshot", "WindowSnapshot"),
    "void": ("void", "void"),
    "error_code": ("TmuxErrorCode", "string"),
    "pane_text": ("PaneTextSnapshot", "PaneTextSnapshot"),
    "pane": ("PaneSnapshot", "PaneSnapshot"),
    "panes": ("std::vector<PaneSnapshot>", "PaneSnapshot[]"),
    "client": ("ClientSnapshot", "ClientSnapshot"),
    "clients": ("std::vector<ClientSnapshot>", "ClientSnapshot[]"),
    "stock_connection_compatibility": (
        "StockConnectionCompatibility",
        "StockConnectionCompatibility",
    ),
    "protocol_direction": ("protocol::tmux::ProtocolDirection", "string"),
    "connection_mode": ("protocol::tmux::ConnectionMode", "string"),
    "support_tier": ("protocol::tmux::SupportTier", "string"),
    "capability_support": ("protocol::tmux::CapabilitySupport", "string"),
    "compatibility_verification": ("protocol::tmux::CompatibilityVerification", "string"),
    "count": ("std::uint32_t", "number"),
    "boolean": ("bool", "boolean"),
    "text_lines": ("std::vector<std::string>", "string[]"),
}


@dataclass(frozen=True)
class Field:
    """Describe a compiled field or parameter and its language presentation."""

    name: str
    kind: str
    node: str
    default: str | None = None


@dataclass(frozen=True)
class Operation:
    """Name a native operation and its exposed connection method."""

    type: str
    header: str
    method: str
    node: str
    id: int
    identity: str
    result: str
    parameters: tuple[Field, ...]


@dataclass(frozen=True)
class Snapshot:
    """Describe one compiled value type and its canonical property contracts."""

    type: str
    header: str
    prefix: str
    fields: tuple[Field, ...]


@dataclass(frozen=True)
class VariantAlternative:
    """Name one C++ variant alternative and its language discriminator."""

    type: str
    kind: str


@dataclass(frozen=True)
class Variant:
    """Describe one closed C++ variant and its discriminated language union."""

    type: str
    header: str
    alternatives: tuple[VariantAlternative, ...]


@dataclass(frozen=True)
class Stream:
    """Describe one owner-scoped subscription and its language names."""

    type: str
    header: str
    prefix: str
    subscribe: str
    node: str
    initial: str
    item: str


@dataclass(frozen=True)
class Exposure:
    """Retain validated operations and value fields in declaration order."""

    operations: tuple[Operation, ...]
    snapshots: tuple[Snapshot, ...]
    variants: tuple[Variant, ...]
    streams: tuple[Stream, ...]
    error: tuple[Field, ...]


def text(record: dict[str, object], key: str) -> str:
    """Require a nonempty string instead of coercing malformed exposure metadata."""
    value = record[key]
    if not isinstance(value, str) or not value:
        raise ValueError(f"{key}: expected nonempty text")
    return value


def records(value: object) -> list[dict[str, object]]:
    """Require TOML table rows before reading their binding contracts."""
    if not isinstance(value, list) or not all(isinstance(row, dict) for row in value):
        raise ValueError("expected a list of exposure tables")
    return value


def field(record: dict[str, object]) -> Field:
    """Validate a field's identifier, kind, and optional default."""
    name, kind = text(record, "name"), text(record, "kind")
    if not re.fullmatch(r"[a-z][a-z0-9_]*", name) or kind not in KINDS:
        raise ValueError(f"invalid field: {name}/{kind}")
    default = record.get("default")
    if default is not None and not isinstance(default, str):
        raise ValueError(f"{name}: default must be text")
    node = record.get("node", name)
    if not isinstance(node, str):
        raise ValueError(f"{name}: Node name must be text")
    return Field(name, kind, node, default)


def variant_alternative(record: dict[str, object]) -> VariantAlternative:
    """Validate one variant type and its stable lowercase discriminator."""
    alternative_type, kind = text(record, "type"), text(record, "kind")
    if not re.fullmatch(r"[A-Z][A-Za-z0-9]*", alternative_type):
        raise ValueError(f"invalid variant alternative type: {alternative_type}")
    if not re.fullmatch(r"[a-z][a-z0-9_]*", kind):
        raise ValueError(f"invalid variant discriminator: {kind}")
    return VariantAlternative(alternative_type, kind)


def load(path: Path = ROOT / "api/exposure.toml") -> Exposure:
    """Reject duplicate operation identities and malformed exposure records."""
    with path.open("rb") as source:
        data = tomllib.load(source)
    if data["schema"] != 3:
        raise ValueError("unsupported exposure schema")
    operations = []
    for row in records(data["operations"]):
        identity = text(row, "identity")
        number, result = row["id"], text(row, "result")
        if not isinstance(number, int) or isinstance(number, bool) or not 0 < number < 65536:
            raise ValueError(f"{identity}: invalid operation id")
        if result not in KINDS:
            raise ValueError(f"{identity}: unsupported result kind")
        operation = Operation(
            text(row, "type"),
            text(row, "header"),
            text(row, "method"),
            text(row, "node"),
            number,
            identity,
            result,
            tuple(field(p) for p in records(row["parameters"])),
        )
        operations.append(operation)
    for attribute in ("id", "identity", "method", "node"):
        values = [getattr(operation, attribute) for operation in operations]
        if len(set(values)) != len(values) or not values:
            raise ValueError(f"duplicate or empty exposure: {attribute}")
    snapshots = tuple(
        Snapshot(
            text(row, "type"),
            text(row, "header"),
            text(row, "prefix"),
            tuple(field(f) for f in records(row["fields"])),
        )
        for row in records(data["snapshots"])
    )
    variants = tuple(
        Variant(
            text(row, "type"),
            text(row, "header"),
            tuple(variant_alternative(alternative) for alternative in records(row["alternatives"])),
        )
        for row in records(data.get("variants", []))
    )
    streams = []
    variant_types = {variant.type for variant in variants}
    for row in records(data.get("streams", [])):
        initial, item = text(row, "initial"), text(row, "item")
        if initial not in KINDS or item not in variant_types:
            raise ValueError(f"invalid stream contract: {initial}/{item}")
        streams.append(
            Stream(
                text(row, "type"),
                text(row, "header"),
                text(row, "prefix"),
                text(row, "subscribe"),
                text(row, "node"),
                initial,
                item,
            )
        )
    for attribute in ("type", "prefix", "subscribe", "node"):
        values = [getattr(stream, attribute) for stream in streams]
        if len(set(values)) != len(values):
            raise ValueError(f"duplicate stream exposure: {attribute}")
    return Exposure(
        tuple(operations),
        snapshots,
        variants,
        tuple(streams),
        tuple(field(f) for f in records(data["error_fields"])),
    )


def canonical_brief(document: str) -> str:
    """Read a brief paragraph without copying Doxygen comment stars into language documentation."""
    text = re.sub(r"(?m)^\s*\* ?", "", document)
    return " ".join(text.split("\n\n", maxsplit=1)[0].split())


def callable_brief(header: str, method: str) -> str:
    """Read one callable's canonical brief from its owning public header."""
    source = (ROOT / "include/tmux_cxx" / header).read_text()
    pattern = rf"/\*\*\s*@brief\s+((?:(?!\*/).)+)\*/\s*[^;{{}}]*\b{re.escape(method)}\("
    match = re.search(pattern, source, re.DOTALL)
    if match is None:
        raise ValueError(f"{method}: missing canonical C++ brief")
    return canonical_brief(match[1])


def brief(method: str) -> str:
    """Read a connection method's canonical C++ brief."""
    return callable_brief("connection/server_connection.hpp", method)


def declaration_brief(header: str, type_name: str) -> str:
    """Read one public class or struct brief from its owning header."""
    source = (ROOT / "include/tmux_cxx" / header).read_text()
    match = re.search(
        rf"/\*\*\s*@brief\s+((?:(?!\*/).)+)\*/\s*(?:class|struct)\s+{re.escape(type_name)}\b",
        source,
        re.DOTALL,
    )
    if match is None:
        raise ValueError(f"{type_name}: missing value type brief")
    return canonical_brief(match[1])


def write(path: Path, content: str, *, check: bool) -> None:
    """Write changed output or reject drift without modifying generated files."""
    if path.exists() and path.read_text() == content:
        return
    if check:
        raise ValueError(f"generated output differs: {path.relative_to(ROOT)}")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content)


def snapshot_brief(snapshot: Snapshot, name: str) -> str:
    """Read a snapshot property's canonical field contract for language documentation."""
    source = (ROOT / "include/tmux_cxx" / snapshot.header).read_text()
    declaration = re.search(
        rf"\bstruct\s+{re.escape(snapshot.type)}\s*\{{(?P<body>.*?)\n\}};",
        source,
        re.DOTALL,
    )
    if declaration is None:
        raise ValueError(f"{snapshot.type}: missing snapshot declaration")
    match = re.search(
        rf"/\*\*\s*@brief\s+((?:(?!\*/).)+)\*/\s*[^;{{}}]*\b{name}"
        rf"(?:\s*=\s*[^;]+)?\s*;",
        declaration.group("body"),
        re.DOTALL,
    )
    if match is None:
        raise ValueError(f"{name}: missing snapshot field brief")
    return canonical_brief(match[1])


def snapshot_type_brief(snapshot: Snapshot) -> str:
    """Read the canonical description of a materialized C++ value type."""
    return declaration_brief(snapshot.header, snapshot.type)


def binding_documentation(exposure: Exposure) -> str:
    """Share C++ briefs with runtime bindings and built-module stubs."""
    values = {op.method: brief(op.method) for op in exposure.operations}
    values.update(connection=brief("ServerConnection"), close=brief("close"))
    for stream in exposure.streams:
        values[stream.subscribe] = brief(stream.subscribe)
        values[f"{stream.prefix}_type"] = declaration_brief(stream.header, stream.type)
        for method in ("initial", "next", "close"):
            values[f"{stream.prefix}_{method}"] = callable_brief(stream.header, method)
    for snapshot in exposure.snapshots:
        values[f"{snapshot.prefix}_type"] = snapshot_type_brief(snapshot)
        values.update(
            {
                f"{snapshot.prefix}_{f.name}": snapshot_brief(snapshot, f.name)
                for f in snapshot.fields
            }
        )
    lines = [
        "// Generated canonical binding briefs; do not edit.",
        "#pragma once",
        "namespace tmux_cxx::binding_documentation {",
    ]
    lines += [
        f"inline constexpr char {name}[] = {json.dumps(value)};" for name, value in values.items()
    ]
    lines += ["} // namespace tmux_cxx::binding_documentation", ""]
    return "\n".join(lines)


def compiled_contract(exposure: Exposure) -> str:
    """Compile-check operation identities, request fields, and connection signatures."""
    lines = ["// Generated from api/exposure.toml; change the contract, not this file."]
    lines += [f"#include <tmux_cxx/{op.header}>" for op in exposure.operations]
    lines += [f"#include <tmux_cxx/{variant.header}>" for variant in exposure.variants]
    lines += [f"#include <tmux_cxx/{stream.header}>" for stream in exposure.streams]
    lines += [
        "#include <tmux_cxx/connection/server_connection.hpp>",
        "#include <tmux_cxx/protocol/native/api_operations.hpp>",
        "#include <array>",
        "#include <type_traits>",
        "#include <variant>",
        "namespace tmux_cxx {",
    ]
    for op in exposure.operations:
        lines += [
            f"static_assert({op.type}::description.id.value == {op.id});",
            f'static_assert({op.type}::description.name == "{op.identity}");',
        ]
        arguments = ", ".join(KINDS[p.kind][0] for p in op.parameters)
        signature = f"Result<{KINDS[op.result][0]}>(ServerConnection::*)({arguments})"
        lines.append(
            f"static_assert(std::is_same_v<decltype(&ServerConnection::{op.method}), {signature}>);"
        )
        for parameter in op.parameters:
            lines.append(
                f"static_assert(std::is_same_v<decltype({op.type}::Request::{parameter.name}), "
                f"{KINDS[parameter.kind][0]}>);"
            )
    for snapshot in exposure.snapshots:
        for value in snapshot.fields:
            lines.append(
                f"static_assert(std::is_same_v<decltype({snapshot.type}::{value.name}), "
                f"{KINDS[value.kind][0]}>);"
            )
    for variant in exposure.variants:
        for index, alternative in enumerate(variant.alternatives):
            lines.append(
                f"static_assert(std::is_same_v<std::variant_alternative_t<{index}, "
                f"{variant.type}>, {alternative.type}>);"
            )
    for stream in exposure.streams:
        lines += [
            f"static_assert(std::is_same_v<decltype(&ServerConnection::{stream.subscribe}), "
            f"Result<{stream.type}>(ServerConnection::*)()>);",
            f"static_assert(std::is_same_v<decltype(&{stream.type}::initial), "
            f"{KINDS[stream.initial][0]}({stream.type}::*)() const>);",
            f"static_assert(std::is_same_v<decltype(&{stream.type}::next), "
            f"Result<std::optional<{stream.item}>>({stream.type}::*)(std::uint32_t)>);",
            f"static_assert(std::is_same_v<decltype(&{stream.type}::close), "
            f"void({stream.type}::*)() noexcept>);",
        ]
    for value in exposure.error:
        lines.append(
            f"static_assert(std::is_same_v<decltype(TmuxError::{value.name}), "
            f"{KINDS[value.kind][0]}>);"
        )
    lines += [
        "} // namespace tmux_cxx",
        "namespace tmux_cxx::protocol::native {",
        "std::span<const OperationDescription> api_operations() {",
        "    static constexpr std::array operations{",
        *[f"        {operation.type}::description," for operation in exposure.operations],
        "    };",
        "    return operations;",
        "}",
        "} // namespace tmux_cxx::protocol::native",
        "",
    ]
    return "\n".join(lines)
