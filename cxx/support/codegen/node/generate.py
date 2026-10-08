"""Generate Node declarations from compile-validated C++ operation exposure."""

from __future__ import annotations

import argparse
import os
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from exposure import (
    KINDS,
    ROOT,
    brief,
    callable_brief,
    declaration_brief,
    load,
    snapshot_brief,
    snapshot_type_brief,
    write,
)


def declarations() -> str:
    """Generate accurate promise, identity, byte, snapshot, and error types."""
    exposure = load()
    lines = [
        "// Generated from api/exposure.toml and C++ briefs; do not edit.",
        'import type { Buffer } from "node:buffer";',
        "",
    ]
    for snapshot in exposure.snapshots:
        lines += [f"/** {snapshot_type_brief(snapshot)} */", f"export interface {snapshot.type} {{"]
        for field in snapshot.fields:
            lines += [
                f"  /** {snapshot_brief(snapshot, field.name)} */",
                f"  readonly {field.node}: {KINDS[field.kind][1]};",
            ]
        lines += ["}", ""]
    for variant in exposure.variants:
        lines += [f"export type {variant.type} ="]
        lines += [
            f'  | ({alternative.type} & {{ readonly kind: "{alternative.kind}" }})'
            for alternative in variant.alternatives
        ]
        lines[-1] += ";"
        lines += [""]
    for stream in exposure.streams:
        options = f"{stream.type}Options"
        lines += [
            "/** Bound one stream wait and its cancellation to the caller. */",
            f"export interface {options} {{",
            "  /** Wait from 1 through 750 milliseconds before returning null. */",
            "  readonly timeoutMs?: number;",
            "  /** Close this stream when the caller aborts an admitted wait. */",
            "  readonly signal?: AbortSignal;",
            "}",
            "",
            f"/** {declaration_brief(stream.header, stream.type)} */",
            f"export interface {stream.type} extends AsyncIterable<{stream.item}> {{",
            f"  /** {callable_brief(stream.header, 'initial')} */",
            f"  readonly initial: {KINDS[stream.initial][1]};",
            f"  /** {callable_brief(stream.header, 'next')} */",
            f"  next(options?: {options}): Promise<{stream.item} | null>;",
            "  /** Iterate committed items until closure or cancellation. */",
            f"  events(options?: {options}): AsyncIterable<{stream.item}>;",
            f"  /** {callable_brief(stream.header, 'close')} */",
            "  close(): void;",
            "  /** Close this stream during asynchronous resource disposal. */",
            "  [Symbol.asyncDispose](): Promise<void>;",
            "}",
            "",
        ]
    lines += [
        "/** A native failure with its classification and operation context. */",
        "export interface TmuxError extends Error {",
        '  readonly name: "TmuxError";',
    ]
    lines += [f"  readonly {f.node}: {KINDS[f.kind][1]};" for f in exposure.error]
    lines += [
        "}",
        "",
        "/** Control a persistent server without owning its sessions. */",
        "export class ServerConnection {",
        f"  /** {brief('ServerConnection')} */",
        "  constructor(socket: string);",
    ]
    for operation in exposure.operations:
        parameters = ", ".join(
            f"{p.node}{'?' if p.default is not None else ''}: "
            f"{'Uint8Array' if p.kind == 'bytes' else KINDS[p.kind][1]}"
            for p in operation.parameters
        )
        lines += [
            f"  /** {brief(operation.method)} */",
            f"  {operation.node}({parameters}): Promise<{KINDS[operation.result][1]}>;",
        ]
    for stream in exposure.streams:
        lines += [
            f"  /** {brief(stream.subscribe)} */",
            f"  {stream.node}(): Promise<{stream.type}>;",
        ]
    lines += [f"  /** {brief('close')} */", "  close(): void;", "}", ""]
    environment = dict(
        os.environ,
        PATH=str(ROOT / "_build/tools/node_current/bin") + os.pathsep + os.environ["PATH"],
    )
    return subprocess.check_output(
        [
            str(ROOT / "bindings/node/node_modules/.bin/oxfmt"),
            "-c",
            str(ROOT / "bindings/node/.oxfmtrc.json"),
            "--stdin-filepath",
            "index.d.ts",
        ],
        input="\n".join(lines),
        text=True,
        env=environment,
    )


def main() -> None:
    """Generate TypeScript declarations or reject drift in their contents."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    arguments = parser.parse_args()
    write(ROOT / "bindings/node/index.d.ts", declarations(), check=arguments.check)


if __name__ == "__main__":
    main()
