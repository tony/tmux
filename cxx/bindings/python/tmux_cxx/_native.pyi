import enum

class PaneSplitOrientation(enum.Enum):
    LEFT_RIGHT = 0

    TOP_BOTTOM = 1

class PaneResizeDirection(enum.Enum):
    LEFT = 0

    RIGHT = 1

    UP = 2

    DOWN = 3

class WindowLayoutPreset(enum.Enum):
    EVEN_HORIZONTAL = 0

    EVEN_VERTICAL = 1

    MAIN_HORIZONTAL = 2

    MAIN_HORIZONTAL_MIRRORED = 3

    MAIN_VERTICAL = 4

    MAIN_VERTICAL_MIRRORED = 5

    TILED = 6

class WindowModeKeys(enum.Enum):
    EMACS = 0

    VI = 1

class CopyCursorDirection(enum.Enum):
    LEFT = 0

    RIGHT = 1

    UP = 2

    DOWN = 3

class CopyCursorDestination(enum.Enum):
    LINE_START = 0

    LINE_END = 1

    HISTORY_START = 2

    HISTORY_END = 3

class CopyScrollMotion(enum.Enum):
    LINE_UP = 0

    LINE_DOWN = 1

    HALF_PAGE_UP = 2

    HALF_PAGE_DOWN = 3

    PAGE_UP = 4

    PAGE_DOWN = 5

class CopySelectionDisposition(enum.Enum):
    KEEP_SELECTION = 0

    CLEAR_SELECTION = 1

    CANCEL_MODE = 2

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

class PasteBufferSnapshot:
    """
    Copy server-global paste-buffer metadata without retaining its stored bytes.
    """

    @property
    def name(self) -> str:
        """Name resolved against the current server store."""

    @property
    def byte_count(self) -> int:
        """Stored byte count, bounded to 256 KiB."""

    @property
    def automatic(self) -> bool:
        """Automatic buffers participate in default paste and eviction."""

    @property
    def revision(self) -> int:
        """Server mutation revision at the buffer's last replacement."""

class WindowOptionsSnapshot:
    """
    Copy effective window options with their server owner and mutation revision.
    """

    @property
    def server_instance(self) -> str:
        """Server lifetime owning these options."""

    @property
    def window(self) -> int | None:
        """Shared window identity, absent for server-wide window defaults."""

    @property
    def mode_keys(self) -> WindowModeKeys:
        """Effective emacs or vi copy-mode key convention."""

    @property
    def revision(self) -> int:
        """
        Window metadata revision, or current server metadata revision for global defaults.
        """

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

class CopyPosition:
    """
    Locate a cell in chronological frozen copy-mode backing, including retained history.
    """

    @property
    def row(self) -> int:
        """Absolute zero-based backing row."""

    @property
    def column(self) -> int:
        """Zero-based cell column, including a permitted end-of-line position."""

class CopySelectionSnapshot:
    """Copy an explicit character selection in frozen backing coordinates."""

    @property
    def anchor(self) -> CopyPosition:
        """Absolute backing cell where character selection began."""

    @property
    def end(self) -> CopyPosition:
        """
        Current cursor endpoint; extraction inclusion follows the live window key convention.
        """

class CopyModeStateSnapshot:
    """Own the visible copy viewport, selection, and frozen-source evidence."""

    @property
    def rows(self) -> int:
        """Current copy viewport height in character cells."""

    @property
    def columns(self) -> int:
        """Current copy viewport width in character cells."""

    @property
    def backing_lines(self) -> int:
        """Retained backing row count after any copy-owned reflow."""

    @property
    def viewport_top(self) -> int:
        """Absolute backing row displayed at the viewport's top."""

    @property
    def cursor(self) -> CopyPosition:
        """
        Absolute frozen-backing cursor, including a permitted end-of-line position.
        """

    @property
    def selection(self) -> CopySelectionSnapshot | None:
        """Explicit character selection, absent until selection begins."""

    @property
    def source_revision(self) -> int:
        """
        Program parser revision captured on entry; later output does not change it.
        """

    @property
    def source_history_lines(self) -> int:
        """
        Primary terminal-history rows captured on entry, before any copy-owned reflow.
        """

    @property
    def alternate_screen(self) -> bool:
        """Entry captured the alternate screen without hidden primary history."""

    @property
    def complete(self) -> bool:
        """
        The captured terminal state completed without a contained parser failure.
        """

    @property
    def history_gap(self) -> bool:
        """A history prefix had already been discarded before entry."""

    @property
    def mode_keys(self) -> WindowModeKeys:
        """Live inherited window convention for input, movement, and extraction."""

    @property
    def highlight_keys(self) -> WindowModeKeys:
        """Selection-highlight convention captured when this mode was entered."""

    @property
    def lines(self) -> list[str]:
        """
        Owned UTF-8 visible rows; wide-cell continuations add no duplicate text.
        """

class CopyModeSnapshot:
    """
    Observe one pane's copy mode without borrowing server or backing storage.
    """

    @property
    def server_instance(self) -> str:
        """Server instance owning this pane and copy observation."""

    @property
    def pane(self) -> int:
        """Pane identity within the observed server instance."""

    @property
    def display_revision(self) -> int:
        """
        Freshness of the pane's chosen display, independent of program-parser revisions.
        """

    @property
    def mode(self) -> CopyModeStateSnapshot | None:
        """Frozen mode state, absent while the pane displays its program screen."""

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

    def rename_window(self, window: int, name: str) -> WindowSnapshot:
        """
        Rename a shared window while preserving every session membership and selection.
        """

    def select_layout(self, window: int, preset: WindowLayoutPreset) -> WindowSnapshot:
        """
        Arrange a shared window using a named tmux preset, preserving pane order and selection.
        """

    def set_window_option(self, target: int | None, name: str, value: str) -> None:
        """
        Assign a global window default or one shared window's explicit option override.
        """

    def window_options(self, target: int | None) -> WindowOptionsSnapshot:
        """
        Copy owner-qualified effective window options or their global defaults.
        """

    def create_window(
        self, session: int, index: int, name: str, command: str = "/bin/sh"
    ) -> WindowLinkSnapshot:
        """
        Create a PTY-backed window at an unused session index without selecting it.
        """

    def select_window(self, window_link: int) -> SessionSnapshot:
        """Select a window membership and return its owning session."""

    def next_window(self, session: int) -> SessionSnapshot:
        """
        Select the session's next window index, wrapping after its final membership.
        """

    def previous_window(self, session: int) -> SessionSnapshot:
        """
        Select the session's previous window index, wrapping before its first membership.
        """

    def split_pane(
        self, pane: int, orientation: PaneSplitOrientation, command: str = "/bin/sh"
    ) -> PaneSnapshot:
        """
        Split one pane, start its command, and materialize the selected new pane.
        """

    def resize_pane(
        self, pane: int, direction: PaneResizeDirection, cells: int = 1
    ) -> PaneSnapshot:
        """Move a pane border by cells, preserving window size and selection."""

    def select_pane(self, pane: int) -> PaneSnapshot:
        """Select one live pane in its shared window and materialize the result."""

    def panes(self, window: int) -> list[PaneSnapshot]:
        """Materialize a shared window's panes in logical pane order."""

    def set_paste_buffer(self, name: str, bytes: bytes) -> None:
        """Store up to 256 KiB under an explicit server-global paste-buffer name."""

    def add_paste_buffer(self, bytes: bytes) -> PasteBufferSnapshot | None:
        """
        Create a nonempty automatic buffer, evicting the oldest automatic entry at capacity.
        """

    def paste_buffers(self) -> list[PasteBufferSnapshot]:
        """
        Copy named and automatic buffer metadata in newest-first storage order.
        """

    def read_paste_buffer(self, name: str) -> bytes:
        """
        Copy arbitrary bytes from one explicitly named server-global paste buffer.
        """

    def delete_paste_buffer(self, name: str) -> None:
        """Delete one explicitly named server-global paste buffer."""

    def paste_buffer_into_pane(self, name: str, pane: int) -> None:
        """Queue one named buffer for a pane using tmux's default line separator."""

    def paste_top_buffer(self, pane: int) -> None:
        """
        Queue the newest automatic buffer for a live pane; an empty automatic store adds no input.
        """

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

    def enter_copy_mode(self, pane: int) -> None:
        """
        Freeze retained pane backing; re-entry preserves the existing mode and selection.
        """

    def move_copy_cursor(self, pane: int, direction: CopyCursorDirection, count: int = 1) -> None:
        """
        Move a frozen cursor by relative cells; zero count preserves mode state.
        """

    def move_copy_cursor_to(self, pane: int, destination: CopyCursorDestination) -> None:
        """Move a frozen cursor to an uncounted logical-line or history boundary."""

    def scroll_copy_mode(self, pane: int, motion: CopyScrollMotion, count: int = 1) -> None:
        """
        Scroll frozen backing by line or page steps; zero count preserves mode state.
        """

    def begin_copy_selection(self, pane: int) -> None:
        """Begin character selection at the current frozen cursor."""

    def clear_copy_selection(self, pane: int) -> None:
        """Clear selection and clip its cursor using current window keys."""

    def copy_selection(
        self,
        pane: int,
        disposition: CopySelectionDisposition = CopySelectionDisposition.CLEAR_SELECTION,
    ) -> PasteBufferSnapshot | None:
        """
        Store selected cells as an automatic buffer before clearing selection or leaving copy mode.
        """

    def cancel_copy_mode(self, pane: int) -> None:
        """
        Leave copy mode for the live program display; inactive panes stay unchanged.
        """

    def copy_mode(self, pane: int) -> CopyModeSnapshot:
        """
        Copy owner-qualified mode state and display freshness, including inactive observations.
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
