from tmux_cxx import ServerConnection


def consumer(client: ServerConnection) -> None:
    """Require argument, return-assignment, and missing-export diagnostics."""
    client.rename_session("not-an-id", "next")
    client.select_layout(1, "even-horizontal")
    client.set_window_option("not-a-window", "mode-keys", "vi")
    client.window_options("not-a-window")
    client.enter_copy_mode("not-a-pane")
    client.move_copy_cursor(1, "right", 2)
    client.move_copy_cursor_to(1, "line_start")
    client.scroll_copy_mode(1, "page_up")
    client.copy_selection(1, "cancel_mode")
    wrong_copy: str = client.copy_mode(1)
    wrong: str = client.sessions()
    client.absent_export()
    print(wrong)
    print(wrong_copy)
