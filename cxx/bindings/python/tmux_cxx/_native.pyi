import enum

class PaneSplitOrientation(enum.Enum):
    LEFT_RIGHT = 0

    TOP_BOTTOM = 1

class TmuxError(RuntimeError):
    code: str
    operation: str
    server_instance: str

class SessionSnapshot:
    """A session value that survives deletion and connection disposal."""

    @property
    def id(self) -> int:
        """Stable session identity within the observed server instance."""

    @property
    def window(self) -> int:
        """Window selected through this session's current link."""

    @property
    def link(self) -> int:
        """Session membership connecting the selected window to this session."""

    @property
    def pane(self) -> int:
        """Selected pane within the linked window."""

    @property
    def name(self) -> str:
        """Session name at this observation revision."""

    @property
    def revision(self) -> int:
        """Server mutation revision of this session's most recent change."""

class SessionEventCursor:
    """Qualify one next-session-event position by its owning server lifetime."""

    @property
    def server_instance(self) -> str:
        """Server lifetime that issued this cursor."""

    @property
    def next_sequence(self) -> int:
        """Absolute sequence of the next session event to read."""

class SessionObservation:
    """
    Pair materialized sessions with the first committed session event after their snapshot.
    """

    @property
    def sessions(self) -> list[SessionSnapshot]:
        """Sessions materialized at one server-thread boundary."""

    @property
    def next_event(self) -> SessionEventCursor:
        """First event committed after that snapshot boundary."""

class SessionCreatedEvent:
    """
    Record one fully materialized session creation after its entity graph is consistent.
    """

    @property
    def session(self) -> SessionSnapshot:
        """Created session value at the committed revision."""

class SessionRenamedEvent:
    """Record one committed session-name change."""

    @property
    def session(self) -> int:
        """Renamed session identity."""

    @property
    def name(self) -> str:
        """Replacement name after validation."""

    @property
    def revision(self) -> int:
        """Server mutation revision that committed the name."""

class SessionClosedEvent:
    """
    Preserve a removed session's final identity and name for later observers.
    """

    @property
    def session(self) -> int:
        """Removed session identity."""

    @property
    def name(self) -> str:
        """Final session name before removal."""

    @property
    def revision(self) -> int:
        """Server mutation revision that committed removal."""

class SessionEventRecord:
    """
    Pair one committed session event with its absolute observation sequence.
    """

    @property
    def sequence(self) -> int:
        """Absolute sequence assigned after the session mutation commits."""

    @property
    def event(self) -> SessionCreatedEvent | SessionRenamedEvent | SessionClosedEvent:
        """Copied session fact retained independently of later state."""

class SessionEventBatch:
    """
    Carry copied session-event records and the cursor immediately following them.
    """

    @property
    def records(self) -> list[SessionEventRecord]:
        """Ordered session records at or after the cursor."""

    @property
    def next_cursor(self) -> SessionEventCursor:
        """Cursor following every returned record."""

class SessionEventGap:
    """
    Report discarded session history together with its replacement atomic observation.
    """

    @property
    def missed_from(self) -> SessionEventCursor:
        """Cursor whose unread session-event prefix is no longer retained."""

    @property
    def replacement(self) -> SessionObservation:
        """
        Current sessions and the first event following their replacement snapshot.
        """

class SessionEventStream:
    """
    Own one session observer's cursor, buffered batch, cancellation, and recovery path.
    """

    @property
    def initial(self) -> SessionObservation:
        """
        Copy the atomic session baseline established when this stream was created.
        """

    def next(self, timeout_ms: int = 500) -> SessionEventRecord | SessionEventGap | None:
        """
        Return one event, gap recovery, or no item after a 1..750 millisecond wait.
        """

    def close(self) -> None:
        """
        Cancel an admitted wait and reject later reads while preserving server state.
        """

    def __enter__(self) -> SessionEventStream:
        """Borrow this stream without transferring its observer-owned cursor."""

    def __exit__(
        self, exc_type: object | None, exc_value: object | None, traceback: object | None
    ) -> None:
        """Close the stream without suppressing the context's exception."""

class WindowSnapshot:
    """
    Materialized window state shared by all sessions linked to that window.
    """

    @property
    def id(self) -> int:
        """Stable window identity within the observed server."""

    @property
    def pane(self) -> int:
        """Active pane shared by every link to this window."""

    @property
    def name(self) -> str:
        """Window name at this revision."""

    @property
    def rows(self) -> int:
        """Shared program-terminal height selected by attached clients."""

    @property
    def columns(self) -> int:
        """Shared program-terminal width selected by attached clients."""

    @property
    def revision(self) -> int:
        """Server mutation revision of the window's most recent change."""

class WindowLinkSnapshot:
    """
    Materialized membership assigning a shared window an index within one session.
    """

    @property
    def id(self) -> int:
        """Stable membership identity within the observed server."""

    @property
    def session(self) -> int:
        """Session owning this membership and its index."""

    @property
    def window(self) -> int:
        """Shared window retained while any membership exists."""

    @property
    def index(self) -> int:
        """Window index within this membership's session."""

    @property
    def revision(self) -> int:
        """Server mutation revision establishing this membership."""

class PaneSnapshot:
    """
    Materialize one pane's window ownership, layout geometry, and selection state.
    """

    @property
    def id(self) -> int:
        """Stable pane identity within the observed server."""

    @property
    def window(self) -> int:
        """Shared window that exclusively owns this pane."""

    @property
    def top(self) -> int:
        """Zero-based top row within the shared window."""

    @property
    def left(self) -> int:
        """Zero-based left column within the shared window."""

    @property
    def rows(self) -> int:
        """Program-terminal height excluding layout borders."""

    @property
    def columns(self) -> int:
        """Program-terminal width excluding layout borders."""

    @property
    def active(self) -> bool:
        """Whether the owning window currently selects this pane."""

    @property
    def revision(self) -> int:
        """Owning window revision that produced this geometry."""

class PaneTextSnapshot:
    """
    Own interpreted pane text and freshness independently of raw bytes and client rendering.
    """

    @property
    def pane(self) -> int:
        """Pane identity within the observed server instance."""

    @property
    def server_instance(self) -> str:
        """Server instance owning this text observation."""

    @property
    def revision(self) -> int:
        """
        Terminal parser/resize sequence, distinct from server metadata revisions.
        """

    @property
    def rows(self) -> int:
        """Program terminal height in character cells."""

    @property
    def columns(self) -> int:
        """Program terminal width in character cells."""

    @property
    def cursor_row(self) -> int:
        """Zero-based cursor row within the active screen."""

    @property
    def cursor_column(self) -> int:
        """Zero-based cursor column within the active screen."""

    @property
    def cursor_visible(self) -> bool:
        """Requested terminal cursor visibility."""

    @property
    def alternate_screen(self) -> bool:
        """The alternate screen is active."""

    @property
    def complete(self) -> bool:
        """
        Every consumed chunk completed without a contained terminal callback failure.
        """

    @property
    def lines(self) -> list[str]:
        """Owned UTF-8 rows; wide-cell continuations add no extra text or spaces."""

    @property
    def history_lines(self) -> int:
        """Number of retained primary terminal-history rows."""

    @property
    def history_gap(self) -> bool:
        """
        An earlier terminal-history prefix was discarded; this cannot recover raw bytes.
        """

    @property
    def input_open(self) -> bool:
        """The PTY and program-input queue still accept input."""

    @property
    def input_pending(self) -> bool:
        """Accepted input or terminal replies still await PTY delivery."""

class ClientSnapshot:
    """
    Copy one client's attachment and terminal size without exposing live resources.
    """

    @property
    def id(self) -> int:
        """Stable identity for this server-side client lifetime."""

    @property
    def session(self) -> int | None:
        """Attached retained session, absent while detached."""

    @property
    def detached_from(self) -> str:
        """Session name reported by the latest completed detachment."""

    @property
    def identified(self) -> bool:
        """Stock identification completed for this client."""

    @property
    def has_terminal(self) -> bool:
        """An ordinary terminal route is available for attachment."""

    @property
    def rows(self) -> int:
        """Outer terminal rows, or zero without a usable terminal."""

    @property
    def columns(self) -> int:
        """Outer terminal columns, or zero without a usable terminal."""

    @property
    def revision(self) -> int:
        """Most recent client mutation in the server sequence."""

class StockConnectionCompatibility:
    """
    Report selected framing, unknown release evidence, effective support, and active adaptations.
    """

    @property
    def profile(self) -> str:
        """Selected wire layout identity, separate from a tmux release."""

    @property
    def release(self) -> str:
        """
        Remote tmux release from trusted metadata or a verified server response.
        """

    @property
    def release_uncertain(self) -> bool:
        """
        True until the remote tmux endpoint supplies trustworthy release evidence.
        """

    @property
    def direction(self) -> str:
        """Adapter role whose behavior this report assesses."""

    @property
    def mode(self) -> str:
        """Requested stock connection behavior."""

    @property
    def tier(self) -> str:
        """
        Effective release-support tier without implying complete older-version parity.
        """

    @property
    def support(self) -> str:
        """Effective capability fidelity for the requested mode."""

    @property
    def verification(self) -> str:
        """Evidence status; a profile match alone proves no behavioral baseline."""

    @property
    def commands(self) -> list[str]:
        """Registered command spellings available under this connection policy."""

    @property
    def quirks(self) -> list[str]:
        """Active named adaptations; an empty list means none."""

    @property
    def evidence(self) -> list[str]:
        """Conformance and detection facts supporting this report."""

    @property
    def limitations(self) -> list[str]:
        """Observable restrictions and missing release evidence."""

class ServerConnection:
    def __init__(self, socket: str) -> None:
        """
        Bind native requests to the named server endpoint without starting a daemon.
        """

    def create_session(self, name: str, command: str = "/bin/sh") -> SessionSnapshot:
        """
        Create a persistent PTY-backed session with a valid UTF-8 name on the connected server.
        """

    def sessions(self) -> list[SessionSnapshot]:
        """Materialize session values from the connected server."""

    def observe_sessions(self) -> SessionObservation:
        """
        Materialize sessions and the first committed event after their snapshot boundary.
        """

    def subscribe_sessions(self) -> SessionEventStream:
        """
        Establish an atomic session baseline and an independently cancellable event stream.
        """

    def read_session_events(self, cursor: SessionEventCursor) -> SessionEventBatch:
        """Read retained committed events from an owner-qualified cursor."""

    def wait_session_events(
        self, cursor: SessionEventCursor, timeout_ms: int = 500
    ) -> SessionEventBatch:
        """
        Await retained events for 1..750 milliseconds, returning an empty batch on expiry.
        """

    def rename_session(self, id: int, name: str) -> None:
        """Rename a session resolved against the current server instance."""

    def kill_session(self, id: int) -> None:
        """
        Explicitly remove a session; closing this connection never invokes this operation.
        """

    def clients(self) -> list[ClientSnapshot]:
        """Materialize connection-lifetime clients in stable identity order."""

    def stock_client_compatibility(self, client: int) -> StockConnectionCompatibility:
        """
        Copy one stock client's selected wire profile, evidence, support, and limits.
        """

    def attach_client(self, client: int, session: int) -> ClientSnapshot:
        """Attach an identified ordinary client to a retained session."""

    def detach_client(self, client: int) -> ClientSnapshot:
        """End one client's attachment while retaining both client and session."""

    def resize_client(self, client: int) -> ClientSnapshot:
        """
        Remeasure an attached client's retained TTY and resize its selected window.
        """

    def link_window(self, window: int, session: int, index: int) -> WindowLinkSnapshot:
        """Link a shared window at an unused index in the destination session."""

    def unlink_window(self, id: int) -> None:
        """
        Remove one window membership while preserving windows retained by other memberships.
        """

    def window_links(self, session: int) -> list[WindowLinkSnapshot]:
        """Materialize a session's window memberships in index order."""

    def window(self, id: int) -> WindowSnapshot:
        """
        Materialize shared window state independently of its session memberships.
        """

    def split_pane(
        self, pane: int, orientation: PaneSplitOrientation, command: str = "/bin/sh"
    ) -> PaneSnapshot:
        """
        Split one pane, start its command, and materialize the selected new pane.
        """

    def select_pane(self, pane: int) -> PaneSnapshot:
        """Select one live pane in its shared window and materialize the result."""

    def panes(self, window: int) -> list[PaneSnapshot]:
        """Materialize a shared window's panes in layout order."""

    def set_paste_buffer(self, name: str, bytes: bytes) -> None:
        """Store up to 256 KiB under an explicit server-global paste-buffer name."""

    def read_paste_buffer(self, name: str) -> bytes:
        """
        Copy arbitrary bytes from one explicitly named server-global paste buffer.
        """

    def delete_paste_buffer(self, name: str) -> None:
        """Delete one explicitly named server-global paste buffer."""

    def paste_buffer_into_pane(self, name: str, pane: int) -> None:
        """Queue one named buffer for a pane using tmux's default line separator."""

    def send_pane_input(self, pane: int, bytes: bytes) -> None:
        """
        Queue up to 64 KiB of input; accepted bytes may be delivered after completion.
        """

    def read_pane_output(self, pane: int) -> bytes:
        """Read retained raw pane output and report discarded history as a gap."""

    def read_pane_text(self, id: int) -> PaneTextSnapshot:
        """
        Copy pane text with terminal freshness and program-input state, separately from raw bytes.
        """

    def wait_pane_output(self, pane: int, needle: bytes, timeout_ms: int = 500) -> bytes:
        """Await a nonempty byte pattern with a deadline of 1..750 milliseconds."""

    def close(self) -> None:
        """
        Stop admission for this connection and streams sharing its native channel.
        """

    def __enter__(self) -> ServerConnection:
        """
        Borrow this connection for a context without transferring server ownership.
        """

    def __exit__(
        self, exc_type: object | None, exc_value: object | None, traceback: object | None
    ) -> None:
        """
        Close admission without suppressing exceptions or destroying persistent sessions.
        """
