"""Compare layout geometry with independent stock processes in the outer loop."""

import os
import subprocess
from pathlib import Path

import pytest
import tmux_cxx

from tests.conftest import stock_fixture_environment

pytestmark = pytest.mark.slow


@pytest.mark.parametrize("fixture", ["TMUX_CXX_STOCK_CLIENT", "TMUX_CXX_MODERN_STOCK_CLIENT"])
def test_named_layouts_follow_modern_geometry_through_stock_clients(
    server: str, fixture: str
) -> None:
    """Both stock clients select modern layouts without changing pane order or program identity."""
    binary = os.environ[fixture]
    environment = stock_fixture_environment(binary)
    modern = os.environ["TMUX_CXX_MODERN_STOCK_CLIENT"]
    reference_socket = Path(server).with_name("layout-reference")
    assert not reference_socket.exists()
    reference = [modern, "-f", "/dev/null", "-S", str(reference_socket)]
    candidate = [binary, "-N", "-f", "/dev/null", "-S", server]
    program = 'while IFS= read -r token; do printf "%s " "$token"; stty size; done'
    names = [
        "even-horizontal",
        "even-vertical",
        "main-horizontal",
        "main-vertical",
        "tiled",
        "main-horizontal-mirrored",
        "main-vertical-mirrored",
    ]

    def run(endpoint: list[str], *arguments: str) -> bytes:
        """Execute one bounded command against an owned layout-test endpoint."""
        return subprocess.run(
            [*endpoint, *arguments],
            env=stock_fixture_environment(endpoint[0]),
            check=True,
            capture_output=True,
            timeout=0.9,
        ).stdout

    def reference_panes(target: str) -> list[list[int]]:
        """Read logical pane identities, geometry, and selection from the reference window."""
        output = run(
            reference,
            "list-panes",
            "-t",
            target,
            "-F",
            "#{pane_id} #{pane_top} #{pane_left} #{pane_height} #{pane_width} #{pane_active}",
        )
        return [
            [int(part.removeprefix("%")) for part in row.split()]
            for row in output.decode().splitlines()
        ]

    def area(pane: tmux_cxx.PaneSnapshot) -> int:
        """Choose a splittable largest pane without changing logical order."""
        return pane.rows * pane.columns

    try:
        with tmux_cxx.ServerConnection(server) as observer:
            for count in (2, 3, 5, 8):
                target = f"layout{count}:0"
                run(
                    reference,
                    "new-session",
                    "-d",
                    "-s",
                    f"layout{count}",
                    "-x",
                    "80",
                    "-y",
                    "24",
                    program,
                )
                run(reference, "set-option", "-g", "status", "off")
                run(reference, "resize-window", "-t", target, "-x", "80", "-y", "24")
                session = observer.create_session(f"layout{count}", program)
                for ordinal in range(1, count):
                    panes = observer.panes(session.window)

                    chosen = max(panes, key=area)
                    index = panes.index(chosen)
                    orientation = "-h" if ordinal % 2 else "-v"
                    run(candidate, "split-window", orientation, "-t", f"%{chosen.id}", program)
                    run(
                        reference,
                        "split-window",
                        orientation,
                        "-t",
                        f"%{reference_panes(target)[index][0]}",
                        program,
                    )
                identities = [pane.id for pane in observer.panes(session.window)]
                for ordinal, name in enumerate(names):
                    run(candidate, "selectl", "-t", target, name)
                    run(reference, "select-layout", "-t", target, name)
                    panes = observer.panes(session.window)
                    assert [pane.id for pane in panes] == identities
                    assert [
                        [pane.top, pane.left, pane.rows, pane.columns, int(pane.active)]
                        for pane in panes
                    ] == [row[1:] for row in reference_panes(target)]
                    token = f"layout-size-{count}-{ordinal}"
                    observer.send_pane_input(session.pane, f"{token}\n".encode())
                    expected = f"{token} {panes[0].rows} {panes[0].columns}".encode()
                    assert expected in observer.wait_pane_output(session.pane, expected, 500)
                before = observer.window(session.window)
                rejected = subprocess.run(
                    [*candidate, "select-layout", "-t", target, "main-h"],
                    env=environment,
                    capture_output=True,
                    timeout=0.9,
                )
                assert rejected.returncode != 0
                assert observer.window(session.window).revision == before.revision
    finally:
        subprocess.run(
            [*reference, "kill-server"],
            env=stock_fixture_environment(modern),
            check=False,
            capture_output=True,
            timeout=0.9,
        )
        reference_socket.unlink(missing_ok=True)


def test_tmux_34_layout_remainders_are_an_explicit_compatibility_limit(server: str) -> None:
    """Characterize older residual allocation without treating it as modern geometry."""
    binary = os.environ["TMUX_CXX_STOCK_CLIENT"]
    environment = stock_fixture_environment(binary)
    endpoint = Path(server).with_name("legacy-layout-reference")
    assert not endpoint.exists()
    reference = [binary, "-f", "/dev/null", "-S", str(endpoint)]

    def run(*arguments: str) -> bytes:
        """Execute one bounded command on the owned tmux 3.4 reference."""
        return subprocess.run(
            [*reference, *arguments],
            env=environment,
            check=True,
            capture_output=True,
            timeout=0.9,
        ).stdout

    try:
        for count in (2, 3, 5, 8):
            target = f"legacy{count}:0"
            run("new-session", "-d", "-s", f"legacy{count}", "-x", "80", "-y", "24", "/bin/cat")
            run("set-option", "-g", "status", "off")
            run("resize-window", "-t", target, "-x", "80", "-y", "24")
            for _ in range(1, count):
                run("split-window", "-h", "-t", target, "/bin/cat")
                run("select-layout", "-t", target, "even-horizontal")
            expected = {
                "even-horizontal": (
                    (39, 40),
                    (26, 26, 26),
                    (15, 15, 15, 15, 16),
                    (9, 9, 9, 9, 9, 9, 9, 10),
                ),
                "even-vertical": ((11, 12), (7, 7, 8), (4, 4, 4, 4, 4), (2, 2, 2, 2, 2, 2, 2, 3)),
                "main-horizontal": (
                    (80,),
                    (39, 40),
                    (19, 19, 19, 20),
                    (10, 10, 10, 10, 10, 10, 14),
                ),
                "main-vertical": ((24,), (11, 12), (5, 5, 5, 6), (2, 2, 2, 2, 2, 2, 6)),
            }
            index = (2, 3, 5, 8).index(count)
            for name, extents in expected.items():
                run("select-layout", "-t", target, name)
                field = "#{pane_width}" if name.endswith("horizontal") else "#{pane_height}"
                observed = tuple(
                    int(value) for value in run("list-panes", "-t", target, "-F", field).split()
                )
                assert (observed[1:] if name.startswith("main") else observed) == extents[index]
            run("select-layout", "-t", target, "tiled")
            tiled = tuple(
                tuple(int(value) for value in row.split())
                for row in run(
                    "list-panes", "-t", target, "-F", "#{pane_height} #{pane_width}"
                ).splitlines()
            )
            assert (
                tiled
                == (
                    ((11, 80), (12, 80)),
                    ((11, 39), (11, 40), (12, 80)),
                    ((7, 39), (7, 40), (7, 39), (7, 40), (8, 80)),
                    ((7, 26), (7, 26), (7, 26), (7, 26), (7, 26), (7, 26), (8, 26), (8, 53)),
                )[index]
            )
            for name in ("main-horizontal-mirrored", "main-vertical-mirrored"):
                rejected = subprocess.run(
                    [*reference, "select-layout", "-t", target, name],
                    env=environment,
                    capture_output=True,
                    timeout=0.9,
                )
                assert rejected.returncode != 0
    finally:
        subprocess.run(
            [*reference, "kill-server"],
            env=environment,
            check=False,
            capture_output=True,
            timeout=0.9,
        )
        endpoint.unlink(missing_ok=True)


@pytest.mark.parametrize("fixture", ["TMUX_CXX_STOCK_CLIENT", "TMUX_CXX_MODERN_STOCK_CLIENT"])
def test_pane_border_resizing_matches_stock_layouts(server: str, fixture: str) -> None:
    """Owned reference layouts match nested border movement and actual C++ program dimensions."""
    binary = os.environ[fixture]
    environment = stock_fixture_environment(binary)
    reference_socket = Path(server).with_name("reference")
    assert not reference_socket.exists()
    reference = [binary, "-f", "/dev/null", "-S", str(reference_socket)]
    candidate = [binary, "-N", "-f", "/dev/null", "-S", server]
    program = 'while IFS= read -r token; do printf "%s " "$token"; stty size; done'

    def run(endpoint: list[str], *arguments: str) -> bytes:
        """Execute one bounded command against this test's owned endpoint."""
        return subprocess.run(
            [*endpoint, *arguments],
            env=environment,
            check=True,
            capture_output=True,
            timeout=0.9,
        ).stdout

    def reference_panes(index: int = 0) -> list[list[int]]:
        """Copy reference geometry, selection, and identities in layout order."""
        rows = (
            run(
                reference,
                "list-panes",
                "-t",
                f"reference:{index}",
                "-F",
                "#{pane_id} #{pane_top} #{pane_left} #{pane_height} #{pane_width} #{pane_active}",
            )
            .decode()
            .splitlines()
        )
        return [[int(part.removeprefix("%")) for part in row.split()] for row in rows]

    try:
        run(reference, "new-session", "-d", "-s", "reference", "-x", "80", "-y", "24", program)
        run(reference, "set-option", "-g", "status", "off")
        run(reference, "resize-window", "-t", "reference:0", "-x", "80", "-y", "24")
        with tmux_cxx.ServerConnection(server) as observer:
            session = observer.create_session("candidate", program)

            def compare(window: int = session.window, index: int = 0) -> None:
                """Require every candidate pane's geometry and selection to match the reference."""
                assert [
                    [pane.top, pane.left, pane.rows, pane.columns, int(pane.active)]
                    for pane in observer.panes(window)
                ] == [row[1:] for row in reference_panes(index)]

            compare()
            for orientation in ("-h", "-v"):
                run(candidate, "split-window", orientation, "-t", f"%{session.pane}", program)
                run(
                    reference,
                    "split-window",
                    orientation,
                    "-t",
                    f"%{reference_panes()[0][0]}",
                    program,
                )
                compare()
            for index, direction, cells in (
                (1, "-R", "5"),
                (2, "-L", "7"),
                (1, "-U", "3"),
                (0, "-D", "2147483647"),
                (1, "-U", "2147483647"),
                (1, "-R", "2147483647"),
            ):
                pane = observer.panes(session.window)[index]
                target = reference_panes()[index][0]
                run(candidate, "resizep", direction, "-t", f"%{pane.id}", cells)
                run(reference, "resize-pane", direction, "-t", f"%{target}", cells)
                compare()
            pane = observer.panes(session.window)[1]
            for endpoint, target in ((candidate, pane.id), (reference, reference_panes()[1][0])):
                rejected = subprocess.run(
                    [*endpoint, "resize-pane", "-R", "-t", f"%{target}", "0"],
                    env=environment,
                    capture_output=True,
                    timeout=0.9,
                )
                assert rejected.returncode != 0
            compare()
            observer.send_pane_input(pane.id, b"accepted\n")
            expected = f"accepted {pane.rows} {pane.columns}".encode()
            assert expected in observer.wait_pane_output(pane.id, expected, 500)
            assert (pane.rows, pane.columns) == (22, 78)

            flat = observer.create_window(session.id, 1, "flat", program)
            run(reference, "new-window", "-d", "-t", "reference:1", "-n", "flat", program)
            run(reference, "resize-window", "-t", "reference:1", "-x", "80", "-y", "24")
            for index in (0, 1):
                target = observer.panes(flat.window)[index].id
                observer.split_pane(target, tmux_cxx.PaneSplitOrientation.LEFT_RIGHT, program)
                run(
                    reference,
                    "split-window",
                    "-h",
                    "-t",
                    f"%{reference_panes(1)[index][0]}",
                    program,
                )
                compare(flat.window, 1)
            for index, direction in ((1, "-R"), (2, "-L")):
                target = observer.panes(flat.window)[index].id
                run(candidate, "resizep", direction, "-t", f"%{target}", "2147483647")
                run(
                    reference,
                    "resizep",
                    direction,
                    "-t",
                    f"%{reference_panes(1)[index][0]}",
                    "2147483647",
                )
                compare(flat.window, 1)
    finally:
        subprocess.run(
            [*reference, "kill-server"],
            env=environment,
            check=False,
            capture_output=True,
            timeout=0.9,
        )
        reference_socket.unlink(missing_ok=True)
