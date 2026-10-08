"""Verify generated language contracts use the owning C++ declaration."""

from support.codegen.exposure import load, snapshot_brief


def test_snapshot_brief_is_scoped_to_its_declaring_type() -> None:
    """Repeated field names resolve inside their declared snapshot type."""
    exposure = load()
    renamed = next(value for value in exposure.snapshots if value.type == "SessionRenamedEvent")
    closed = next(value for value in exposure.snapshots if value.type == "SessionClosedEvent")
    assert snapshot_brief(renamed, "session") == "Renamed session identity."
    assert snapshot_brief(closed, "session") == "Removed session identity."
