from tmux_cxx import ServerConnection


def consumer(client: ServerConnection) -> None:
    """Require argument, return-assignment, and missing-export diagnostics."""
    client.rename_session("not-an-id", "next")
    wrong: str = client.sessions()
    client.absent_export()
    print(wrong)
