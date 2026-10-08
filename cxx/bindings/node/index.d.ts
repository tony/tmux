// Generated from api/exposure.toml and C++ briefs; do not edit.
import type { Buffer } from "node:buffer";

/** A session value that survives deletion and connection disposal. */
export interface SessionSnapshot {
  /** Stable session identity within the observed server instance. */
  readonly id: bigint;
  /** Window selected through this session's current link. */
  readonly window: bigint;
  /** Session membership connecting the selected window to this session. */
  readonly link: bigint;
  /** Selected pane within the linked window. */
  readonly pane: bigint;
  /** Session name at this observation revision. */
  readonly name: string;
  /** Server mutation revision of this session's most recent change. */
  readonly revision: bigint;
}

/** Qualify one next-session-event position by its owning server lifetime. */
export interface SessionEventCursor {
  /** Server lifetime that issued this cursor. */
  readonly serverInstance: string;
  /** Absolute sequence of the next session event to read. */
  readonly nextSequence: bigint;
}

/** Pair materialized sessions with the first committed session event after their snapshot. */
export interface SessionObservation {
  /** Sessions materialized at one server-thread boundary. */
  readonly sessions: SessionSnapshot[];
  /** First event committed after that snapshot boundary. */
  readonly nextEvent: SessionEventCursor;
}

/** Record one fully materialized session creation after its entity graph is consistent. */
export interface SessionCreatedEvent {
  /** Created session value at the committed revision. */
  readonly session: SessionSnapshot;
}

/** Record one committed session-name change. */
export interface SessionRenamedEvent {
  /** Renamed session identity. */
  readonly session: bigint;
  /** Replacement name after validation. */
  readonly name: string;
  /** Server mutation revision that committed the name. */
  readonly revision: bigint;
}

/** Preserve a removed session's final identity and name for later observers. */
export interface SessionClosedEvent {
  /** Removed session identity. */
  readonly session: bigint;
  /** Final session name before removal. */
  readonly name: string;
  /** Server mutation revision that committed removal. */
  readonly revision: bigint;
}

/** Pair one committed session event with its absolute observation sequence. */
export interface SessionEventRecord {
  /** Absolute sequence assigned after the session mutation commits. */
  readonly sequence: bigint;
  /** Copied session fact retained independently of later state. */
  readonly event: SessionEvent;
}

/** Carry copied session-event records and the cursor immediately following them. */
export interface SessionEventBatch {
  /** Ordered session records at or after the cursor. */
  readonly records: SessionEventRecord[];
  /** Cursor following every returned record. */
  readonly nextCursor: SessionEventCursor;
}

/** Report discarded session history together with its replacement atomic observation. */
export interface SessionEventGap {
  /** Cursor whose unread session-event prefix is no longer retained. */
  readonly missedFrom: SessionEventCursor;
  /** Current sessions and the first event following their replacement snapshot. */
  readonly replacement: SessionObservation;
}

/** Own interpreted pane text and freshness independently of raw bytes and client rendering. */
export interface PaneTextSnapshot {
  /** Server instance owning this text observation. */
  readonly serverInstance: string;
  /** Pane identity within the observed server instance. */
  readonly pane: bigint;
  /** Terminal parser/resize sequence, distinct from server metadata revisions. */
  readonly revision: bigint;
  /** Program terminal height in character cells. */
  readonly rows: number;
  /** Program terminal width in character cells. */
  readonly columns: number;
  /** Zero-based cursor row within the active screen. */
  readonly cursorRow: number;
  /** Zero-based cursor column within the active screen. */
  readonly cursorColumn: number;
  /** Requested terminal cursor visibility. */
  readonly cursorVisible: boolean;
  /** The alternate screen is active. */
  readonly alternateScreen: boolean;
  /** Every consumed chunk completed without a contained terminal callback failure. */
  readonly complete: boolean;
  /** Owned UTF-8 rows; wide-cell continuations add no extra text or spaces. */
  readonly lines: string[];
  /** Number of retained primary terminal-history rows. */
  readonly historyLines: number;
  /** An earlier terminal-history prefix was discarded; this cannot recover raw bytes. */
  readonly historyGap: boolean;
  /** The PTY and program-input queue still accept input. */
  readonly inputOpen: boolean;
  /** Accepted input or terminal replies still await PTY delivery. */
  readonly inputPending: boolean;
}

/** Materialize one pane's window ownership, layout geometry, and selection state. */
export interface PaneSnapshot {
  /** Stable pane identity within the observed server. */
  readonly id: bigint;
  /** Shared window that exclusively owns this pane. */
  readonly window: bigint;
  /** Zero-based top row within the shared window. */
  readonly top: number;
  /** Zero-based left column within the shared window. */
  readonly left: number;
  /** Program-terminal height excluding layout borders. */
  readonly rows: number;
  /** Program-terminal width excluding layout borders. */
  readonly columns: number;
  /** Whether the owning window currently selects this pane. */
  readonly active: boolean;
  /** Owning window revision that produced this geometry. */
  readonly revision: bigint;
}

/** Copy one client's attachment and terminal size without exposing live resources. */
export interface ClientSnapshot {
  /** Stable identity for this server-side client lifetime. */
  readonly id: bigint;
  /** Attached retained session, absent while detached. */
  readonly session: bigint | null;
  /** Session name reported by the latest completed detachment. */
  readonly detachedFrom: string;
  /** Stock identification completed for this client. */
  readonly identified: boolean;
  /** An ordinary terminal route is available for attachment. */
  readonly hasTerminal: boolean;
  /** Outer terminal rows, or zero without a usable terminal. */
  readonly rows: number;
  /** Outer terminal columns, or zero without a usable terminal. */
  readonly columns: number;
  /** Most recent client mutation in the server sequence. */
  readonly revision: bigint;
}

/** Report selected framing, unknown release evidence, effective support, and active adaptations. */
export interface StockConnectionCompatibility {
  /** Selected wire layout identity, separate from a tmux release. */
  readonly profile: string;
  /** Remote tmux release from trusted metadata or a verified server response. */
  readonly release: string;
  /** True until the remote tmux endpoint supplies trustworthy release evidence. */
  readonly releaseUncertain: boolean;
  /** Adapter role whose behavior this report assesses. */
  readonly direction: string;
  /** Requested stock connection behavior. */
  readonly mode: string;
  /** Effective release-support tier without implying complete older-version parity. */
  readonly tier: string;
  /** Effective capability fidelity for the requested mode. */
  readonly support: string;
  /** Evidence status; a profile match alone proves no behavioral baseline. */
  readonly verification: string;
  /** Registered command spellings available under this connection policy. */
  readonly commands: string[];
  /** Active named adaptations; an empty list means none. */
  readonly quirks: string[];
  /** Conformance and detection facts supporting this report. */
  readonly evidence: string[];
  /** Observable restrictions and missing release evidence. */
  readonly limitations: string[];
}

/** Materialized window state shared by all sessions linked to that window. */
export interface WindowSnapshot {
  /** Stable window identity within the observed server. */
  readonly id: bigint;
  /** Active pane shared by every link to this window. */
  readonly pane: bigint;
  /** Window name at this revision. */
  readonly name: string;
  /** Shared program-terminal height selected by attached clients. */
  readonly rows: number;
  /** Shared program-terminal width selected by attached clients. */
  readonly columns: number;
  /** Server mutation revision of the window's most recent change. */
  readonly revision: bigint;
}

/** Materialized membership assigning a shared window an index within one session. */
export interface WindowLinkSnapshot {
  /** Stable membership identity within the observed server. */
  readonly id: bigint;
  /** Session owning this membership and its index. */
  readonly session: bigint;
  /** Shared window retained while any membership exists. */
  readonly window: bigint;
  /** Window index within this membership's session. */
  readonly index: number;
  /** Server mutation revision establishing this membership. */
  readonly revision: bigint;
}

export type SessionEvent =
  | (SessionCreatedEvent & { readonly kind: "session_created" })
  | (SessionRenamedEvent & { readonly kind: "session_renamed" })
  | (SessionClosedEvent & { readonly kind: "session_closed" });

export type SessionEventStreamItem =
  | (SessionEventRecord & { readonly kind: "event" })
  | (SessionEventGap & { readonly kind: "gap" });

/** Bound one stream wait and its cancellation to the caller. */
export interface SessionEventStreamOptions {
  /** Wait from 1 through 750 milliseconds before returning null. */
  readonly timeoutMs?: number;
  /** Close this stream when the caller aborts an admitted wait. */
  readonly signal?: AbortSignal;
}

/** Own one session observer's cursor, buffered batch, cancellation, and recovery path. */
export interface SessionEventStream extends AsyncIterable<SessionEventStreamItem> {
  /** Copy the atomic session baseline established when this stream was created. */
  readonly initial: SessionObservation;
  /** Return one event, gap recovery, or no item after a 1..750 millisecond wait. */
  next(options?: SessionEventStreamOptions): Promise<SessionEventStreamItem | null>;
  /** Iterate committed items until closure or cancellation. */
  events(options?: SessionEventStreamOptions): AsyncIterable<SessionEventStreamItem>;
  /** Cancel an admitted wait and reject later reads while preserving server state. */
  close(): void;
  /** Close this stream during asynchronous resource disposal. */
  [Symbol.asyncDispose](): Promise<void>;
}

/** A native failure with its classification and operation context. */
export interface TmuxError extends Error {
  readonly name: "TmuxError";
  readonly code: string;
  readonly operation: string;
  readonly serverInstance: string;
}

/** Control a persistent server without owning its sessions. */
export class ServerConnection {
  /** Bind native requests to the named server endpoint without starting a daemon. */
  constructor(socket: string);
  /** Create a persistent PTY-backed session with a valid UTF-8 name on the connected server. */
  createSession(name: string, command?: string): Promise<SessionSnapshot>;
  /** Materialize session values from the connected server. */
  sessions(): Promise<SessionSnapshot[]>;
  /** Materialize sessions and the first committed event after their snapshot boundary. */
  observeSessions(): Promise<SessionObservation>;
  /** Read retained committed events from an owner-qualified cursor. */
  readSessionEvents(cursor: SessionEventCursor): Promise<SessionEventBatch>;
  /** Await retained events for 1..750 milliseconds, returning an empty batch on expiry. */
  waitSessionEvents(cursor: SessionEventCursor, timeoutMs?: number): Promise<SessionEventBatch>;
  /** Rename a session resolved against the current server instance. */
  renameSession(session: bigint, name: string): Promise<void>;
  /** Explicitly remove a session; closing this connection never invokes this operation. */
  killSession(session: bigint): Promise<void>;
  /** Queue up to 64 KiB of input; accepted bytes may be delivered after completion. */
  sendPaneInput(pane: bigint, bytes: Uint8Array): Promise<void>;
  /** Read retained raw pane output and report discarded history as a gap. */
  readPaneOutput(pane: bigint): Promise<Buffer>;
  /** Await a nonempty byte pattern with a deadline of 1..750 milliseconds. */
  waitPaneOutput(pane: bigint, needle: Uint8Array, timeoutMs?: number): Promise<Buffer>;
  /** Link a shared window at an unused index in the destination session. */
  linkWindow(window: bigint, session: bigint, index: number): Promise<WindowLinkSnapshot>;
  /** Remove one window membership while preserving windows retained by other memberships. */
  unlinkWindow(windowLink: bigint): Promise<void>;
  /** Materialize a session's window memberships in index order. */
  windowLinks(session: bigint): Promise<WindowLinkSnapshot[]>;
  /** Materialize shared window state independently of its session memberships. */
  window(window: bigint): Promise<WindowSnapshot>;
  /** Copy pane text with terminal freshness and program-input state, separately from raw bytes. */
  readPaneText(pane: bigint): Promise<PaneTextSnapshot>;
  /** Materialize connection-lifetime clients in stable identity order. */
  clients(): Promise<ClientSnapshot[]>;
  /** Attach an identified ordinary client to a retained session. */
  attachClient(client: bigint, session: bigint): Promise<ClientSnapshot>;
  /** End one client's attachment while retaining both client and session. */
  detachClient(client: bigint): Promise<ClientSnapshot>;
  /** Remeasure an attached client's retained TTY and resize its selected window. */
  resizeClient(client: bigint): Promise<ClientSnapshot>;
  /** Copy one stock client's selected wire profile, evidence, support, and limits. */
  stockClientCompatibility(client: bigint): Promise<StockConnectionCompatibility>;
  /** Split one pane, start its command, and materialize the selected new pane. */
  splitPane(
    pane: bigint,
    orientation: "left_right" | "top_bottom",
    command?: string,
  ): Promise<PaneSnapshot>;
  /** Materialize a shared window's panes in layout order. */
  panes(window: bigint): Promise<PaneSnapshot[]>;
  /** Select one live pane in its shared window and materialize the result. */
  selectPane(pane: bigint): Promise<PaneSnapshot>;
  /** Store up to 256 KiB under an explicit server-global paste-buffer name. */
  setPasteBuffer(name: string, bytes: Uint8Array): Promise<void>;
  /** Copy arbitrary bytes from one explicitly named server-global paste buffer. */
  readPasteBuffer(name: string): Promise<Buffer>;
  /** Delete one explicitly named server-global paste buffer. */
  deletePasteBuffer(name: string): Promise<void>;
  /** Queue one named buffer for a pane using tmux's default line separator. */
  pasteBufferIntoPane(name: string, pane: bigint): Promise<void>;
  /** Establish an atomic session baseline and an independently cancellable event stream. */
  subscribeSessions(): Promise<SessionEventStream>;
  /** Stop admission for this connection and streams sharing its native channel. */
  close(): void;
}
