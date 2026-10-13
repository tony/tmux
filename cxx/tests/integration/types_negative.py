from tmux_cxx import ServerConnection


def consumer(client: ServerConnection) -> None:
    """Require argument, return-assignment, and missing-export diagnostics."""
    client.rename_session("not-an-id", "next")
    client.select_layout(1, "even-horizontal")
    client.set_window_option("not-a-window", "mode-keys", "vi")
    client.window_options("not-a-window")
    wrong: str = client.sessions()
    client.absent_export()
    print(wrong)
