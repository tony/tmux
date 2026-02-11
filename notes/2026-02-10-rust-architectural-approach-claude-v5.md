# TermForge: v5 Implementation Detail Specification

Date: 2026-02-10
Extends: v4 Final Synthesis (same directory)
Focus: 10 areas v4 left underspecified
License: MIT OR Apache-2.0
Rust edition: 2024 (MSRV 1.85)

---

## Preamble

This document extends the v4 architectural north star with concrete, implementation-ready specifications for the 10 areas v4 left underspecified. The 10 settled decisions from v4 are not revisited. All file paths reference the tmux C source at `~/study/c/tmux/`, the existing Rust prototype at `~/work/rust/vibe-tmux/`, and the Python libtmux at `~/work/python/libtmux/`.

---

## Table of Contents

A. [Concrete Error Handling](#a-concrete-error-handling)
B. [Configuration System](#b-configuration-system)
C. [Layout Engine](#c-layout-engine)
D. [OpenTelemetry](#d-opentelemetry)
E. [Language Bindings](#e-language-bindings)
F. [CRDT Detail](#f-crdt-detail)
G. [Control Mode](#g-control-mode)
H. [Server Lifecycle](#h-server-lifecycle)
I. [Security Model](#i-security-model)
J. [Performance Targets](#j-performance-targets)

---

## A. Concrete Error Handling

### A.1 Design Principles

1. **Each crate owns its error type.** No shared "uber-error" that everything wraps.
2. **Pure crates return `Result` values, never panic.** Impure crates may panic only on truly unrecoverable states (e.g. the event channel closing).
3. **Error propagation across the pure/impure boundary uses `Effect::Error`.** The pure engine returns errors as data; the runtime decides disposition.
4. **Codec errors are recoverable.** A malformed frame drops that message and logs; the connection stays open.
5. **Every error implements `std::error::Error` and `Display`.** Binary-sized concerns do not apply -- this is not `no_std`.

### A.2 Error Types per Crate

#### mux-types (shared, pure)

```rust
// crates/mux-types/src/errors.rs

/// Errors from entity lookup operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntityError {
    /// Entity with the given ID does not exist or has been removed.
    NotFound { kind: EntityKind, id_debug: String },
    /// Relationship constraint violated (e.g., pane already in a window).
    RelationshipViolation { message: &'static str },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityKind {
    Session, Window, Pane, Client, Job, Buffer,
}

impl std::fmt::Display for EntityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound { kind, id_debug } => {
                write!(f, "{kind:?} not found: {id_debug}")
            }
            Self::RelationshipViolation { message } => {
                write!(f, "relationship violation: {message}")
            }
        }
    }
}

impl std::error::Error for EntityError {}
```

#### mux-core (pure kernel)

```rust
// crates/mux-core/src/error.rs

/// Errors from the pure engine.
/// These never escape to the user directly -- they are wrapped in ApplyOutcome
/// and the runtime decides whether to log, notify the client, or both.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum CoreError {
    /// Entity referenced by an event does not exist.
    Entity(EntityError),

    /// Command string was unparseable or unknown.
    InvalidCommand {
        command: String,
        reason: String,
    },

    /// Option key or value is invalid for the given scope.
    InvalidOption {
        scope: OptionScope,
        key: String,
        reason: String,
    },

    /// Layout operation could not be performed (e.g., pane too small to split).
    LayoutConstraint {
        reason: String,
    },

    /// Format string expansion failed (e.g., infinite recursion guard tripped).
    FormatExpansion {
        template: String,
        reason: String,
    },

    /// Copy mode action invalid in current state (e.g., search with no pattern).
    CopyMode {
        reason: String,
    },
}

impl std::fmt::Display for CoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Entity(e) => write!(f, "entity: {e}"),
            Self::InvalidCommand { command, reason } => {
                write!(f, "invalid command '{command}': {reason}")
            }
            Self::InvalidOption { scope, key, reason } => {
                write!(f, "invalid option '{key}' at {scope:?}: {reason}")
            }
            Self::LayoutConstraint { reason } => {
                write!(f, "layout constraint: {reason}")
            }
            Self::FormatExpansion { template, reason } => {
                write!(f, "format expansion of '{template}': {reason}")
            }
            Self::CopyMode { reason } => {
                write!(f, "copy mode: {reason}")
            }
        }
    }
}

impl std::error::Error for CoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Entity(e) => Some(e),
            _ => None,
        }
    }
}

impl From<EntityError> for CoreError {
    fn from(e: EntityError) -> Self {
        Self::Entity(e)
    }
}
```

**ApplyOutcome carries errors as data, not as Result:**

```rust
// crates/mux-core/src/engine.rs

#[derive(Debug, Default, Clone)]
pub struct ApplyOutcome {
    pub effects: Vec<Effect>,
    pub created_session: Option<SessionId>,
    pub created_window: Option<WindowId>,
    pub created_pane: Option<PaneId>,
    /// Non-fatal error. The engine always produces a valid graph;
    /// errors describe why an operation was partially or fully rejected.
    pub error: Option<CoreError>,
}
```

#### mux-proto (protocol codec, pure)

The existing error type in `~/work/rust/vibe-tmux/crates/mux-proto/src/error.rs` (lines 14-33) is well-designed. We extend it:

```rust
// crates/mux-proto/src/error.rs

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProtocolError {
    /// Not enough bytes for header (need 16).
    Incomplete,

    /// Message length below minimum (16 bytes).
    LengthTooSmall { len: u32 },

    /// Message length exceeds maximum (16384 bytes).
    LengthTooLarge { len: u32 },

    /// Unknown or unsupported message type.
    UnknownMessageType { msg_type: u32 },

    /// Payload data is invalid for the message type.
    InvalidPayload { reason: &'static str },

    /// Not an identify message when one was expected.
    NotIdentifyMessage { msg_type: u32 },

    /// NUL terminator missing in a string payload field.
    MissingNul { field: &'static str },

    /// String payload is not valid UTF-8.
    InvalidUtf8 { field: &'static str },
}

/// Codec error recovery strategy.
/// The ImsgCodec returns io::Error to tokio_util::codec::Decoder.
/// The server translates these to log + drop-frame, not connection-kill.
impl From<ProtocolError> for std::io::Error {
    fn from(e: ProtocolError) -> Self {
        match e {
            ProtocolError::Incomplete => {
                std::io::Error::new(std::io::ErrorKind::WouldBlock, e)
            }
            ProtocolError::LengthTooSmall { .. }
            | ProtocolError::LengthTooLarge { .. }
            | ProtocolError::UnknownMessageType { .. }
            | ProtocolError::InvalidPayload { .. }
            | ProtocolError::NotIdentifyMessage { .. }
            | ProtocolError::MissingNul { .. }
            | ProtocolError::InvalidUtf8 { .. } => {
                std::io::Error::new(std::io::ErrorKind::InvalidData, e)
            }
        }
    }
}
```

#### mux-grid (VT100 parser, pure)

```rust
// crates/mux-grid/src/error.rs

/// Grid operations that can fail.
/// The parser itself never errors -- unknown sequences are silently consumed
/// (matching tmux behavior). Grid resize can fail on constraints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GridError {
    /// Requested size is below minimum (1x1).
    SizeTooSmall { cols: u16, rows: u16 },
    /// Scrollback exceeded hard limit.
    ScrollbackOverflow { max: u32 },
}
```

#### mux-conf (config parser, pure)

```rust
// crates/mux-conf/src/error.rs

#[derive(Debug, Clone)]
pub struct ConfError {
    pub kind: ConfErrorKind,
    pub line: u32,
    pub column: u32,
    pub source_line: String,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum ConfErrorKind {
    /// Lexer: unexpected character.
    UnexpectedChar(char),
    /// Lexer: unterminated string.
    UnterminatedString,
    /// Parser: expected a command, got end of input.
    UnexpectedEof,
    /// Parser: unknown command name.
    UnknownCommand(String),
    /// Parser: invalid flag or argument.
    InvalidArgument { flag: String, reason: String },
    /// Evaluator: if-shell condition evaluation failed.
    ConditionError(String),
    /// Source file not found (path is the logical name, not filesystem path).
    SourceNotFound(String),
}

impl std::fmt::Display for ConfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}: {}: {}",
            self.line, self.column, self.kind, self.source_line,
        )
    }
}

impl std::fmt::Display for ConfErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnexpectedChar(c) => write!(f, "unexpected character '{c}'"),
            Self::UnterminatedString => write!(f, "unterminated string"),
            Self::UnexpectedEof => write!(f, "unexpected end of input"),
            Self::UnknownCommand(name) => write!(f, "unknown command: {name}"),
            Self::InvalidArgument { flag, reason } => {
                write!(f, "invalid argument '{flag}': {reason}")
            }
            Self::ConditionError(msg) => write!(f, "condition: {msg}"),
            Self::SourceNotFound(path) => write!(f, "source file not found: {path}"),
        }
    }
}

impl std::error::Error for ConfError {}
```

#### mux-server (impure runtime)

```rust
// crates/mux-server/src/error.rs

/// Server runtime errors. These are logged and handled; they never panic.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ServerError {
    #[error("socket error: {0}")]
    Socket(#[from] std::io::Error),

    #[error("protocol error on client {client_id}: {error}")]
    Protocol {
        client_id: u64,
        error: ProtocolError,
    },

    #[error("pty error for pane {pane_id}: {error}")]
    Pty {
        pane_id: String,
        error: PtyError,
    },

    #[error("state actor channel closed")]
    ChannelClosed,

    #[error("startup failed: {reason}")]
    Startup { reason: String },

    #[error("SCM_RIGHTS error: {0}")]
    ScmRights(std::io::Error),
}
```

#### mux-api (facade, impure)

The existing `ManagedMuxError` in `~/work/rust/vibe-tmux/crates/mux-api/src/managed.rs` (lines 54-73) is the reference. We adopt it with refinements:

```rust
// crates/mux-api/src/error.rs

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ApiError {
    #[error("backend error: {0}")]
    Backend(#[from] BackendError),

    #[error("control mode error: {0}")]
    Control(#[from] anyhow::Error),

    #[error("view store unavailable")]
    MissingViewStore,

    #[error("socket actor not running")]
    ActorStopped,

    #[error("operation timed out after {0:?}")]
    Timeout(std::time::Duration),

    #[error("command failed: {0}")]
    CommandFailed(String),
}
```

### A.3 Error Propagation Across Pure/Impure Boundary

```
                Pure Core                      Impure Runtime
                --------                       ---------------

Event arrives  ------>  apply_event()
                        |
                        v
                  CoreError produced?
                        |
                  +-----+-----+
                  | Yes        | No
                  v            v
          ApplyOutcome {    ApplyOutcome {
            error: Some(e),   effects: [...],
            effects: [        error: None,
              Effect::NotifyClient {          }
                client_id, message: e.to_string()
              }
            ]
          }
                  |            |
                  v            v
          <------ runtime dispatches effects ------>
                  |
                  v
          Protocol error in codec?
                  |
            InvalidData => log, drop frame, continue
            WouldBlock  => wait for more bytes (normal)
            Other IO    => disconnect that client
```

### A.4 SocketActor Error Handling

The SocketActor (control mode client in `mux-api`) handles errors by:

1. **Transient errors** (timeout, parse): retry with exponential backoff (100ms, 200ms, 400ms, max 5s).
2. **Connection lost**: attempt reconnect; publish `RefreshHint::Disconnected` to subscribers.
3. **Protocol errors**: log and drop the offending notification; do not crash the actor.
4. **Shutdown errors**: swallowed; the actor is already stopping.

```rust
// crates/mux-api/src/managed.rs (SocketActor error loop)

impl SocketActor {
    async fn run_loop(&mut self) {
        let mut backoff = ExponentialBackoff::new(
            Duration::from_millis(100),
            Duration::from_secs(5),
            2.0,
        );
        loop {
            match self.read_next_event().await {
                Ok(event) => {
                    backoff.reset();
                    self.handle_event(event);
                }
                Err(SocketError::Disconnected) => {
                    self.publish_hint(RefreshHint::Disconnected);
                    if self.reconnect(&mut backoff).await.is_err() {
                        tracing::error!("socket actor: giving up reconnection");
                        return;
                    }
                }
                Err(SocketError::Protocol(e)) => {
                    tracing::warn!("socket actor: protocol error: {e}, dropping frame");
                    // continue reading
                }
                Err(SocketError::Shutdown) => return,
            }
        }
    }
}

struct ExponentialBackoff {
    current: Duration,
    max: Duration,
    factor: f64,
}

impl ExponentialBackoff {
    fn new(initial: Duration, max: Duration, factor: f64) -> Self {
        Self { current: initial, max, factor }
    }
    fn reset(&mut self) {
        self.current = Duration::from_millis(100);
    }
    async fn wait(&mut self) {
        tokio::time::sleep(self.current).await;
        self.current = self.current.mul_f64(self.factor).min(self.max);
    }
}
```

---

## B. Configuration System

### B.1 tmux.conf Grammar

tmux.conf is a line-oriented command language. Each line is one command invocation, with shell-like quoting. Studied from `~/work/rust/vibe-tmux/crates/mux-conf/src/` (lexer.rs, parser.rs, ast.rs) and `~/study/c/tmux/options-table.c`.

#### Grammar (PEG-style)

```
config     = line*
line       = ws? (comment | conditional | command) ws? newline
comment    = '#' [^\n]*
conditional = 'if-shell' arg arg ('else' arg)?
command    = word (ws arg)*
arg        = single_quoted | double_quoted | variable | word
single_quoted = "'" [^']* "'"
double_quoted = '"' (escape_seq | [^"\\])* '"'
variable   = '$' '{' word '}'  |  '$' word
escape_seq = '\\' .
word       = [^ \t\n;#'"\\]+
ws         = [ \t]+
newline    = '\n' | ';' | EOF
```

#### AST Types

```rust
// crates/mux-conf/src/ast.rs

/// A parsed config file.
#[derive(Debug, Clone)]
pub struct ConfigFile {
    pub commands: Vec<ConfigCommand>,
    pub errors: Vec<ConfError>,
}

/// A single command with arguments.
#[derive(Debug, Clone)]
pub struct ConfigCommand {
    pub name: String,
    pub args: Vec<ConfigArg>,
    pub line: u32,
    pub conditional: Option<Conditional>,
}

/// A command argument, preserving quote style for round-tripping.
#[derive(Debug, Clone)]
pub enum ConfigArg {
    Bare(String),
    SingleQuoted(String),
    DoubleQuoted(String),
    Variable(String),
}

impl ConfigArg {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Bare(s) | Self::SingleQuoted(s)
            | Self::DoubleQuoted(s) | Self::Variable(s) => s,
        }
    }
}

/// if-shell conditional.
#[derive(Debug, Clone)]
pub struct Conditional {
    pub condition: String,
    pub then_commands: Vec<ConfigCommand>,
    pub else_commands: Vec<ConfigCommand>,
}
```

### B.2 Option Inheritance: Server -> Session -> Window -> Pane

tmux options have a scope hierarchy, visible in `~/study/c/tmux/options-table.c` (lines 26-32) and `~/study/c/tmux/options.c` where `struct options` has a `parent` pointer (line 64).

```rust
// crates/mux-core/src/options.rs

/// Option scope determines where an option can be set and how inheritance works.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OptionScope {
    Server,
    Session,
    Window,
    Pane,
}

/// Option value types, matching tmux's options_table_type in tmux.h lines 2178-2184.
#[derive(Debug, Clone, PartialEq)]
pub enum OptionValue {
    String(String),
    Number(i64),
    Flag(bool),
    Colour(u32),
    Key(u64),
    Choice { index: usize, choices: &'static [&'static str] },
    Array(Vec<String>),
    Command(Vec<String>),
    Style(StyleSpec),
}

/// A single option table entry, matching tmux's options_table_entry
/// in tmux.h lines 2197-2217.
#[derive(Debug, Clone)]
pub struct OptionTableEntry {
    pub name: &'static str,
    pub alternative_name: Option<&'static str>,
    pub scope: OptionScope,
    pub value_type: OptionValueType,
    pub default: OptionValue,
    pub minimum: Option<i64>,
    pub maximum: Option<i64>,
    pub choices: Option<&'static [&'static str]>,
    pub is_array: bool,
    pub is_hook: bool,
    pub is_style: bool,
    pub text: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionValueType {
    String, Number, Key, Colour, Flag, Choice, Command,
}

/// Options store with parent-chain inheritance.
/// Each Session, Window, and Pane has one of these.
/// Mirrors tmux's `struct options` (~/study/c/tmux/options.c line 63-66)
/// where options walk up to `parent` when a key is not found locally.
#[derive(Debug, Clone)]
pub struct OptionStore {
    /// Locally-set options override the parent.
    local: BTreeMap<String, OptionValue>,
}

impl OptionStore {
    pub fn new() -> Self {
        Self { local: BTreeMap::new() }
    }

    /// Set an option locally.
    pub fn set(&mut self, key: &str, value: OptionValue) {
        self.local.insert(key.to_string(), value);
    }

    /// Remove a local override, reverting to the inherited value.
    pub fn unset(&mut self, key: &str) -> Option<OptionValue> {
        self.local.remove(key)
    }

    /// Get a locally-set option (does NOT walk up the chain).
    pub fn get_local(&self, key: &str) -> Option<&OptionValue> {
        self.local.get(key)
    }

    /// Check if this key has a local override.
    pub fn is_set_locally(&self, key: &str) -> bool {
        self.local.contains_key(key)
    }

    /// Iterate all local overrides.
    pub fn iter_local(&self) -> impl Iterator<Item = (&str, &OptionValue)> {
        self.local.iter().map(|(k, v)| (k.as_str(), v))
    }
}

/// Resolve an option by walking up the scope chain.
/// This is a free function because it traverses the graph, not a single store.
///
/// Resolution order for a pane option:
///   pane.options -> window.options -> session.options -> server.global_options
///   -> OPTION_TABLE default
pub fn resolve_option(
    graph: &ServerGraph,
    scope: OptionScope,
    target_id: TargetId,
    key: &str,
) -> Option<OptionValue> {
    match scope {
        OptionScope::Pane => {
            if let TargetId::Pane(pane_id) = target_id {
                if let Some(pane) = graph.pane(pane_id) {
                    if let Some(v) = pane.options.get_local(key) {
                        return Some(v.clone());
                    }
                }
                // Walk up to window
                if let Some(window_id) = graph.pane_to_window(pane_id) {
                    let window_result = resolve_option(
                        graph,
                        OptionScope::Window,
                        TargetId::Window(window_id),
                        key,
                    );
                    if window_result.is_some() {
                        return window_result;
                    }
                }
            }
            // Fall through to global
            graph.global_options.get_local(key).cloned()
                .or_else(|| option_table_default(key))
        }
        OptionScope::Window => {
            if let TargetId::Window(window_id) = target_id {
                if let Some(window) = graph.window(window_id) {
                    if let Some(v) = window.options.get_local(key) {
                        return Some(v.clone());
                    }
                }
                // Walk up to session
                if let Some(session_id) = graph.window_to_session(window_id) {
                    let session_result = resolve_option(
                        graph,
                        OptionScope::Session,
                        TargetId::Session(session_id),
                        key,
                    );
                    if session_result.is_some() {
                        return session_result;
                    }
                }
            }
            graph.global_options.get_local(key).cloned()
                .or_else(|| option_table_default(key))
        }
        OptionScope::Session => {
            if let TargetId::Session(session_id) = target_id {
                if let Some(session) = graph.session(session_id) {
                    if let Some(v) = session.options.get_local(key) {
                        return Some(v.clone());
                    }
                }
            }
            graph.global_options.get_local(key).cloned()
                .or_else(|| option_table_default(key))
        }
        OptionScope::Server => {
            graph.global_options.get_local(key).cloned()
                .or_else(|| option_table_default(key))
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TargetId {
    Session(SessionId),
    Window(WindowId),
    Pane(PaneId),
    Server,
}
```

### B.3 Hot Reload

Config hot reload is an `Event::ReloadConfig` processed by the pure engine:

```rust
// In Event enum:
Event::LoadConfig {
    commands: Vec<ConfigCommand>,
    source: String, // filename for error reporting
},

// In apply_event:
Event::LoadConfig { commands, source } => {
    let mut effects = Vec::new();
    let mut errors = Vec::new();
    for cmd in &commands {
        match apply_config_command(graph, cmd, ctx) {
            Ok(outcome) => effects.extend(outcome.effects),
            Err(e) => errors.push(ConfError {
                kind: ConfErrorKind::InvalidArgument {
                    flag: String::new(),
                    reason: e.to_string(),
                },
                line: cmd.line,
                column: 0,
                source_line: source.clone(),
            }),
        }
    }
    if !errors.is_empty() {
        effects.push(Effect::Log {
            level: LogLevel::Warn,
            message: format!("{}: {} errors during config reload", source, errors.len()),
        });
    }
    ApplyOutcome::with_effects(effects)
}
```

The runtime triggers this:

```rust
// crates/mux-server/src/config.rs

/// Watch the config file for changes and send LoadConfig events.
pub async fn config_watcher(
    path: PathBuf,
    event_tx: mpsc::Sender<Event>,
) {
    use notify::{Watcher, RecursiveMode, recommended_watcher};

    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher = recommended_watcher(tx).expect("file watcher");
    watcher.watch(&path, RecursiveMode::NonRecursive).expect("watch config");

    while let Ok(_event) = rx.recv() {
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                let config = mux_conf::parser::parse(&content);
                let _ = event_tx.send(Event::LoadConfig {
                    commands: config.commands,
                    source: path.display().to_string(),
                }).await;
            }
            Err(e) => {
                tracing::warn!("failed to read config on change: {e}");
            }
        }
    }
}
```

### B.4 mux-conf / mux-command Relationship

```
mux-conf:   text -> ConfigCommand (parsed, unvalidated)
mux-command: ConfigCommand -> Event (validated, semantic)
mux-core:   Event -> graph mutation + Effects
```

`mux-conf` is purely syntactic. It knows how to parse `set-option -g status off` into `ConfigCommand { name: "set-option", args: ["-g", "status", "off"] }`. It does NOT know whether `status` is a valid option or what `-g` means.

`mux-command` takes a `ConfigCommand`, validates it against the command table (flags, argument counts, target resolution), and produces one or more `Event` variants.

```rust
// crates/mux-command/src/table.rs

pub struct CommandDef {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub flags: &'static str,     // getopt-style: "dt:s:"
    pub min_args: usize,
    pub max_args: usize,
    pub handler: fn(&ParsedArgs, &ServerGraph) -> Result<Vec<Event>, CoreError>,
}

pub fn dispatch_command(
    name: &str,
    args: &[ConfigArg],
    graph: &ServerGraph,
) -> Result<Vec<Event>, CoreError> {
    let def = COMMAND_TABLE.iter()
        .find(|d| d.name == name || d.aliases.contains(&name))
        .ok_or_else(|| CoreError::InvalidCommand {
            command: name.to_string(),
            reason: "unknown command".to_string(),
        })?;

    let parsed = parse_flags(args, def.flags, def.min_args, def.max_args)?;
    (def.handler)(&parsed, graph)
}
```

---

## C. Layout Engine

### C.1 Layout Cell Tree

tmux's layout is a tree of cells (see `~/study/c/tmux/tmux.h` lines 1381-1406 and `~/study/c/tmux/layout.c`). Each cell is either a leaf (containing a pane) or a container (left-right or top-bottom) with children.

The existing vibe-tmux layout (`~/work/rust/vibe-tmux/crates/mux-core/src/layout.rs`) implements only `Rect::split_horizontal/split_vertical`. v5 replaces this with a full tree.

```rust
// crates/mux-core/src/layout.rs

/// Minimum pane dimension in either axis (matches tmux's PANE_MINIMUM).
pub const PANE_MINIMUM: u16 = 1;

/// Border separator width between sibling panes.
pub const PANE_BORDER: u16 = 1;

/// Layout cell type, matching tmux's layout_type enum
/// (~/study/c/tmux/tmux.h lines 1381-1384).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutType {
    /// Leaf node: contains a single pane.
    WindowPane,
    /// Horizontal container: children arranged left-to-right.
    LeftRight,
    /// Vertical container: children arranged top-to-bottom.
    TopBottom,
}

/// A node in the layout tree.
/// Mirrors tmux's `struct layout_cell` (tmux.h lines 1391-1406).
///
/// Unlike tmux's pointer-based tree, we use a flat Vec with indices.
/// This is compatible with the v4 settled "Vec inside entity" decision
/// and avoids Box<> indirection for a WASM-pure type.
#[derive(Debug, Clone)]
pub struct LayoutCell {
    pub cell_type: LayoutType,
    /// Width in columns.
    pub sx: u16,
    /// Height in rows.
    pub sy: u16,
    /// X offset from window origin.
    pub xoff: u16,
    /// Y offset from window origin.
    pub yoff: u16,
    /// Pane ID for leaf nodes (WindowPane type).
    pub pane_id: Option<PaneId>,
    /// Children indices for container nodes.
    pub children: Vec<usize>,
    /// Parent index (None for root).
    pub parent: Option<usize>,
}

/// The layout tree for a single window.
/// Stored as a flat arena of cells. Index 0 is always the root.
#[derive(Debug, Clone)]
pub struct LayoutTree {
    cells: Vec<LayoutCell>,
}

impl LayoutTree {
    /// Create a single-pane layout filling the given size.
    pub fn single_pane(pane_id: PaneId, sx: u16, sy: u16) -> Self {
        Self {
            cells: vec![LayoutCell {
                cell_type: LayoutType::WindowPane,
                sx, sy, xoff: 0, yoff: 0,
                pane_id: Some(pane_id),
                children: Vec::new(),
                parent: None,
            }],
        }
    }

    /// Root cell reference.
    pub fn root(&self) -> &LayoutCell {
        &self.cells[0]
    }

    /// Find the cell containing a given pane.
    pub fn find_pane(&self, pane_id: PaneId) -> Option<usize> {
        self.cells.iter().position(|c| c.pane_id == Some(pane_id))
    }

    /// Count leaf cells (panes).
    pub fn pane_count(&self) -> usize {
        self.cells.iter().filter(|c| c.cell_type == LayoutType::WindowPane).count()
    }

    /// Get cell by index.
    pub fn cell(&self, index: usize) -> Option<&LayoutCell> {
        self.cells.get(index)
    }

    /// Get mutable cell by index.
    pub fn cell_mut(&mut self, index: usize) -> Option<&mut LayoutCell> {
        self.cells.get_mut(index)
    }

    /// Split a leaf pane, creating a new container and two leaf children.
    /// Returns the index of the new pane's cell.
    ///
    /// Matches tmux's `layout_split_pane` in layout.c.
    pub fn split_pane(
        &mut self,
        target_pane: PaneId,
        new_pane: PaneId,
        direction: SplitDirection,
        size_hint: SplitSize,
    ) -> Result<usize, LayoutError> {
        let target_idx = self.find_pane(target_pane)
            .ok_or(LayoutError::PaneNotFound(target_pane))?;

        let target = &self.cells[target_idx];
        let (container_type, can_split) = match direction {
            SplitDirection::Horizontal => {
                (LayoutType::LeftRight, target.sx >= PANE_MINIMUM * 2 + PANE_BORDER)
            }
            SplitDirection::Vertical => {
                (LayoutType::TopBottom, target.sy >= PANE_MINIMUM * 2 + PANE_BORDER)
            }
        };
        if !can_split {
            return Err(LayoutError::TooSmall {
                available: match direction {
                    SplitDirection::Horizontal => target.sx,
                    SplitDirection::Vertical => target.sy,
                },
                needed: PANE_MINIMUM * 2 + PANE_BORDER,
            });
        }

        let target_parent = target.parent;
        let old_sx = target.sx;
        let old_sy = target.sy;
        let old_xoff = target.xoff;
        let old_yoff = target.yoff;

        // Compute sizes for the two children
        let (first_size, second_size) = self.compute_split_sizes(
            direction, old_sx, old_sy, &size_hint,
        );

        // Convert the target cell into a container
        self.cells[target_idx].cell_type = container_type;
        self.cells[target_idx].pane_id = None;

        // Create first child (keeps the original pane)
        let first_child_idx = self.cells.len();
        let (first_sx, first_sy, first_xoff, first_yoff) = match direction {
            SplitDirection::Horizontal => (first_size, old_sy, old_xoff, old_yoff),
            SplitDirection::Vertical => (old_sx, first_size, old_xoff, old_yoff),
        };
        self.cells.push(LayoutCell {
            cell_type: LayoutType::WindowPane,
            sx: first_sx, sy: first_sy,
            xoff: first_xoff, yoff: first_yoff,
            pane_id: Some(target_pane),
            children: Vec::new(),
            parent: Some(target_idx),
        });

        // Create second child (the new pane)
        let second_child_idx = self.cells.len();
        let (second_sx, second_sy, second_xoff, second_yoff) = match direction {
            SplitDirection::Horizontal => {
                (second_size, old_sy, old_xoff + first_size + PANE_BORDER, old_yoff)
            }
            SplitDirection::Vertical => {
                (old_sx, second_size, old_xoff, old_yoff + first_size + PANE_BORDER)
            }
        };
        self.cells.push(LayoutCell {
            cell_type: LayoutType::WindowPane,
            sx: second_sx, sy: second_sy,
            xoff: second_xoff, yoff: second_yoff,
            pane_id: Some(new_pane),
            children: Vec::new(),
            parent: Some(target_idx),
        });

        // Update the container's children
        self.cells[target_idx].children = vec![first_child_idx, second_child_idx];

        Ok(second_child_idx)
    }

    fn compute_split_sizes(
        &self,
        direction: SplitDirection,
        sx: u16,
        sy: u16,
        hint: &SplitSize,
    ) -> (u16, u16) {
        let total = match direction {
            SplitDirection::Horizontal => sx,
            SplitDirection::Vertical => sy,
        };
        let usable = total.saturating_sub(PANE_BORDER);
        let first = match hint {
            SplitSize::Half => usable / 2,
            SplitSize::Cells(n) => (*n).min(usable.saturating_sub(PANE_MINIMUM)),
            SplitSize::Percent(pct) => {
                let computed = (usable as f64 * pct / 100.0) as u16;
                computed.max(PANE_MINIMUM).min(usable.saturating_sub(PANE_MINIMUM))
            }
        };
        let second = usable.saturating_sub(first);
        (first.max(PANE_MINIMUM), second.max(PANE_MINIMUM))
    }

    /// Recompute cell offsets after a resize.
    /// Mirrors tmux's `layout_fix_offsets` (layout.c lines 230-238).
    pub fn fix_offsets(&mut self) {
        if self.cells.is_empty() { return; }
        self.cells[0].xoff = 0;
        self.cells[0].yoff = 0;
        self.fix_offsets_recursive(0);
    }

    fn fix_offsets_recursive(&mut self, idx: usize) {
        let cell_type = self.cells[idx].cell_type;
        let xoff = self.cells[idx].xoff;
        let yoff = self.cells[idx].yoff;
        let children: Vec<usize> = self.cells[idx].children.clone();

        match cell_type {
            LayoutType::LeftRight => {
                let mut x = xoff;
                for &child_idx in &children {
                    self.cells[child_idx].xoff = x;
                    self.cells[child_idx].yoff = yoff;
                    x += self.cells[child_idx].sx + PANE_BORDER;
                    self.fix_offsets_recursive(child_idx);
                }
            }
            LayoutType::TopBottom => {
                let mut y = yoff;
                for &child_idx in &children {
                    self.cells[child_idx].xoff = xoff;
                    self.cells[child_idx].yoff = y;
                    y += self.cells[child_idx].sy + PANE_BORDER;
                    self.fix_offsets_recursive(child_idx);
                }
            }
            LayoutType::WindowPane => {}
        }
    }

    /// Collect pane bounds from leaf cells.
    pub fn pane_bounds(&self) -> Vec<(PaneId, Rect)> {
        self.cells.iter()
            .filter_map(|c| {
                c.pane_id.map(|id| (id, Rect {
                    x: c.xoff, y: c.yoff,
                    width: c.sx, height: c.sy,
                }))
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
pub enum SplitSize {
    Half,
    Cells(u16),
    Percent(f64),
}

#[derive(Debug, Clone)]
pub enum LayoutError {
    PaneNotFound(PaneId),
    TooSmall { available: u16, needed: u16 },
    InvalidLayout(String),
}

impl std::fmt::Display for LayoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PaneNotFound(id) => write!(f, "pane {id:?} not found in layout"),
            Self::TooSmall { available, needed } => {
                write!(f, "pane too small to split: {available} < {needed}")
            }
            Self::InvalidLayout(msg) => write!(f, "invalid layout: {msg}"),
        }
    }
}

impl std::error::Error for LayoutError {}
```

### C.2 Predefined Layouts

Matching the 7 layout algorithms from `~/study/c/tmux/layout-set.c` lines 39-50:

```rust
// crates/mux-core/src/layout_set.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutPreset {
    EvenHorizontal,
    EvenVertical,
    MainHorizontal,
    MainHorizontalMirrored,
    MainVertical,
    MainVerticalMirrored,
    Tiled,
}

impl LayoutPreset {
    pub const ALL: &'static [Self] = &[
        Self::EvenHorizontal,
        Self::EvenVertical,
        Self::MainHorizontal,
        Self::MainHorizontalMirrored,
        Self::MainVertical,
        Self::MainVerticalMirrored,
        Self::Tiled,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::EvenHorizontal => "even-horizontal",
            Self::EvenVertical => "even-vertical",
            Self::MainHorizontal => "main-horizontal",
            Self::MainHorizontalMirrored => "main-horizontal-mirrored",
            Self::MainVertical => "main-vertical",
            Self::MainVerticalMirrored => "main-vertical-mirrored",
            Self::Tiled => "tiled",
        }
    }

    /// Build a layout tree for the given panes and window size.
    pub fn arrange(
        self,
        panes: &[PaneId],
        sx: u16,
        sy: u16,
        main_pane_size: Option<u16>,
    ) -> LayoutTree {
        match self {
            Self::EvenHorizontal => arrange_even(panes, sx, sy, LayoutType::LeftRight),
            Self::EvenVertical => arrange_even(panes, sx, sy, LayoutType::TopBottom),
            Self::MainHorizontal => arrange_main(panes, sx, sy, LayoutType::TopBottom, false, main_pane_size),
            Self::MainHorizontalMirrored => arrange_main(panes, sx, sy, LayoutType::TopBottom, true, main_pane_size),
            Self::MainVertical => arrange_main(panes, sx, sy, LayoutType::LeftRight, false, main_pane_size),
            Self::MainVerticalMirrored => arrange_main(panes, sx, sy, LayoutType::LeftRight, true, main_pane_size),
            Self::Tiled => arrange_tiled(panes, sx, sy),
        }
    }
}

/// Even layout: all panes get equal space.
/// Mirrors `layout_set_even` in layout-set.c lines 127-178.
fn arrange_even(
    panes: &[PaneId],
    sx: u16,
    sy: u16,
    container_type: LayoutType,
) -> LayoutTree {
    if panes.len() <= 1 {
        return LayoutTree::single_pane(panes[0], sx, sy);
    }
    let n = panes.len() as u16;
    let total = match container_type {
        LayoutType::LeftRight => sx,
        LayoutType::TopBottom => sy,
        _ => unreachable!(),
    };
    // Each pane gets (total - (n-1) borders) / n cells
    let borders = (n - 1) * PANE_BORDER;
    let usable = total.saturating_sub(borders);
    let per_pane = usable / n;
    let remainder = usable - per_pane * n;

    let mut cells = Vec::with_capacity(1 + panes.len());
    // Root container
    cells.push(LayoutCell {
        cell_type: container_type,
        sx, sy, xoff: 0, yoff: 0,
        pane_id: None,
        children: (1..=panes.len()).collect(),
        parent: None,
    });
    // Children
    let mut offset: u16 = 0;
    for (i, &pane_id) in panes.iter().enumerate() {
        let size = per_pane + if (i as u16) < remainder { 1 } else { 0 };
        let (cx, cy, csx, csy) = match container_type {
            LayoutType::LeftRight => (offset, 0, size, sy),
            LayoutType::TopBottom => (0, offset, sx, size),
            _ => unreachable!(),
        };
        cells.push(LayoutCell {
            cell_type: LayoutType::WindowPane,
            sx: csx, sy: csy, xoff: cx, yoff: cy,
            pane_id: Some(pane_id),
            children: Vec::new(),
            parent: Some(0),
        });
        offset += size + PANE_BORDER;
    }

    LayoutTree { cells }
}
```

### C.3 Layout String Serialization

tmux's layout string format (from `~/study/c/tmux/layout-custom.c` lines 60-117):
`<checksum>,<cell>` where `<cell>` is `WxH,X,Y[,pane_id]` optionally followed by `{children}` (left-right) or `[children]` (top-bottom).

```rust
// crates/mux-core/src/layout_string.rs

/// Serialize a layout tree to tmux's layout string format.
/// Matches `layout_dump` in layout-custom.c lines 60-71.
pub fn layout_dump(tree: &LayoutTree) -> String {
    let mut buf = String::with_capacity(256);
    layout_append(&tree.cells, 0, &mut buf);
    let checksum = layout_checksum(&buf);
    format!("{checksum:04x},{buf}")
}

fn layout_append(cells: &[LayoutCell], idx: usize, buf: &mut String) {
    let cell = &cells[idx];
    use std::fmt::Write;
    if let Some(pane_id) = cell.pane_id {
        write!(buf, "{}x{},{},{},{}", cell.sx, cell.sy, cell.xoff, cell.yoff,
               pane_id_to_u32(pane_id)).ok();
    } else {
        write!(buf, "{}x{},{},{}", cell.sx, cell.sy, cell.xoff, cell.yoff).ok();
    }
    let (open, close) = match cell.cell_type {
        LayoutType::LeftRight => ('{', '}'),
        LayoutType::TopBottom => ('[', ']'),
        LayoutType::WindowPane => return,
    };
    buf.push(open);
    for (i, &child_idx) in cell.children.iter().enumerate() {
        if i > 0 { buf.push(','); }
        layout_append(cells, child_idx, buf);
    }
    buf.push(close);
}

/// Parse a layout string back into a LayoutTree.
/// Matches `layout_parse` in layout-custom.c.
pub fn layout_parse(input: &str) -> Result<LayoutTree, LayoutError> {
    // Skip checksum: "xxxx,"
    let rest = input.get(5..).ok_or_else(|| {
        LayoutError::InvalidLayout("too short for checksum prefix".into())
    })?;
    let mut cells = Vec::new();
    let mut pos = 0;
    layout_construct(rest.as_bytes(), &mut pos, &mut cells, None)?;
    Ok(LayoutTree { cells })
}

/// Layout checksum matching tmux's layout_checksum (layout-custom.c lines 47-57).
fn layout_checksum(layout: &str) -> u16 {
    let mut csum: u16 = 0;
    for &b in layout.as_bytes() {
        csum = (csum >> 1) | ((csum & 1) << 15);
        csum = csum.wrapping_add(b as u16);
    }
    csum
}
```

### C.4 Resize Propagation

Window resize redistributes space proportionally among children:

```rust
// crates/mux-core/src/layout.rs (continued)

impl LayoutTree {
    /// Resize the root cell and propagate to all children proportionally.
    pub fn resize(&mut self, new_sx: u16, new_sy: u16) {
        if self.cells.is_empty() { return; }
        let old_sx = self.cells[0].sx;
        let old_sy = self.cells[0].sy;
        self.cells[0].sx = new_sx;
        self.cells[0].sy = new_sy;
        self.resize_children(0, old_sx, old_sy, new_sx, new_sy);
        self.fix_offsets();
    }

    fn resize_children(
        &mut self,
        idx: usize,
        old_sx: u16, old_sy: u16,
        new_sx: u16, new_sy: u16,
    ) {
        let children: Vec<usize> = self.cells[idx].children.clone();
        if children.is_empty() {
            self.cells[idx].sx = new_sx;
            self.cells[idx].sy = new_sy;
            return;
        }

        let cell_type = self.cells[idx].cell_type;
        match cell_type {
            LayoutType::LeftRight => {
                let borders = (children.len() as u16 - 1) * PANE_BORDER;
                let old_usable = old_sx.saturating_sub(borders);
                let new_usable = new_sx.saturating_sub(borders);
                let mut remaining = new_usable;

                for (i, &child_idx) in children.iter().enumerate() {
                    let child_old_sx = self.cells[child_idx].sx;
                    let child_new_sx = if i == children.len() - 1 {
                        remaining // last child gets whatever is left
                    } else {
                        let proportional = if old_usable > 0 {
                            (child_old_sx as u32 * new_usable as u32 / old_usable as u32) as u16
                        } else {
                            new_usable / children.len() as u16
                        };
                        proportional.max(PANE_MINIMUM)
                    };
                    remaining = remaining.saturating_sub(child_new_sx);
                    self.resize_children(child_idx, child_old_sx, old_sy, child_new_sx, new_sy);
                }
            }
            LayoutType::TopBottom => {
                let borders = (children.len() as u16 - 1) * PANE_BORDER;
                let old_usable = old_sy.saturating_sub(borders);
                let new_usable = new_sy.saturating_sub(borders);
                let mut remaining = new_usable;

                for (i, &child_idx) in children.iter().enumerate() {
                    let child_old_sy = self.cells[child_idx].sy;
                    let child_new_sy = if i == children.len() - 1 {
                        remaining
                    } else {
                        let proportional = if old_usable > 0 {
                            (child_old_sy as u32 * new_usable as u32 / old_usable as u32) as u16
                        } else {
                            new_usable / children.len() as u16
                        };
                        proportional.max(PANE_MINIMUM)
                    };
                    remaining = remaining.saturating_sub(child_new_sy);
                    self.resize_children(child_idx, old_sx, child_old_sy, new_sx, child_new_sy);
                }
            }
            LayoutType::WindowPane => {
                self.cells[idx].sx = new_sx;
                self.cells[idx].sy = new_sy;
            }
        }
    }
}
```

---

## D. OpenTelemetry

### D.1 Architecture

The existing `~/work/rust/vibe-tmux/crates/mux-otel/` provides the foundation. The key design: tracing spans propagate across the server/client boundary via W3C trace context headers embedded in the control mode protocol.

```
+------------------+     +-------------------+     +------------------+
| mux CLI client   |     | muxd server       |     | Python binding   |
|                  |     |                   |     |                  |
| TraceCtx {       |     | TraceCtx injected |     | traceparent in   |
|   traceparent    | --> |   into CoreCtx    | <-- |   TRACEPARENT    |
|   tracestate     |     |   from client hdr |     |   env var        |
| }                |     |                   |     |                  |
+------------------+     +-------------------+     +------------------+
         |                       |                          |
         v                       v                          v
    OTLP exporter           OTLP exporter            OTLP exporter
         |                       |                          |
         +-------+-------+-------+-------+------------------+
                 |
            Jaeger / Tempo
```

### D.2 Span Design

```rust
// crates/mux-telemetry/src/spans.rs

/// Top-level span names. Each maps to a tracing span.
pub mod span_names {
    pub const SERVER_ACCEPT: &str = "server.accept";
    pub const CLIENT_IDENTIFY: &str = "client.identify";
    pub const ENGINE_APPLY: &str = "engine.apply_event";
    pub const EFFECT_DISPATCH: &str = "effect.dispatch";
    pub const PTY_READ: &str = "pty.read";
    pub const PTY_WRITE: &str = "pty.write";
    pub const CODEC_DECODE: &str = "codec.decode";
    pub const CODEC_ENCODE: &str = "codec.encode";
    pub const SNAPSHOT_BUILD: &str = "snapshot.build";
    pub const SOCKET_ACTOR_CYCLE: &str = "socket_actor.cycle";
    pub const CONFIG_RELOAD: &str = "config.reload";
    pub const LAYOUT_RESIZE: &str = "layout.resize";
    pub const FORMAT_EXPAND: &str = "format.expand";
}
```

### D.3 Pure/Impure Boundary Crossing

The pure engine cannot create tracing spans (that would require `std::time`). Instead, the runtime wraps each `apply_event` call in a span:

```rust
// crates/mux-server/src/state.rs

use tracing::instrument;

impl StateActor {
    pub async fn run(mut self) {
        while let Some(event) = self.event_rx.recv().await {
            self.process_event(event);
        }
    }

    #[instrument(
        name = "engine.apply_event",
        skip(self),
        fields(
            event_type = %event_type_name(&event),
            effects_count = tracing::field::Empty,
            error = tracing::field::Empty,
        )
    )]
    fn process_event(&mut self, event: Event) {
        let span = tracing::Span::current();
        let ctx = CoreCtx {
            now_ms: now_millis(),
            rand_u64: rand::random(),
        };
        let outcome = apply_event(&mut self.graph, event, &ctx);

        span.record("effects_count", outcome.effects.len());
        if let Some(ref error) = outcome.error {
            span.record("error", error.to_string().as_str());
        }

        for effect in outcome.effects {
            self.dispatch_effect(effect);
        }
        self.publish_snapshot();
    }
}
```

### D.4 SocketActor Integration

The SocketActor propagates trace context from the tmux control mode connection:

```rust
// crates/mux-api/src/managed.rs

impl SocketActor {
    fn handle_control_event(&mut self, event: ControlEvent) {
        let span = tracing::info_span!(
            "socket_actor.handle",
            event_type = %event.event_type(),
            socket = %self.socket_path.display(),
        );
        let _guard = span.enter();
        // ... process event
    }
}
```

### D.5 Configuration

```rust
// crates/mux-telemetry/src/config.rs

/// OTEL configuration, loaded from `$XDG_CONFIG_HOME/termforge/otel.toml`
/// or environment variables.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct OtelConfig {
    /// Enable/disable OTEL export. Default: auto-detect from env.
    #[serde(default)]
    pub enabled: Option<bool>,

    /// OTLP endpoint. Default: http://localhost:4318
    #[serde(default = "default_endpoint")]
    pub endpoint: String,

    /// Service name. Default: "termforge"
    #[serde(default = "default_service_name")]
    pub service_name: String,

    /// Export filter: only export spans matching these module prefixes.
    #[serde(default)]
    pub export_filter: ExportFilterConfig,

    /// Batch export delay in ms. Default: 100
    #[serde(default = "default_batch_delay")]
    pub batch_delay_ms: u64,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ExportFilterConfig {
    /// Module prefixes to include. Default: ["mux_", "termforge"]
    #[serde(default = "default_include_prefixes")]
    pub include_prefixes: Vec<String>,
    /// Module prefixes to exclude. Default: ["h2", "tonic", "hyper"]
    #[serde(default = "default_exclude_prefixes")]
    pub exclude_prefixes: Vec<String>,
}

impl ExportFilterConfig {
    pub fn should_export(&self, target: &str) -> bool {
        if self.exclude_prefixes.iter().any(|p| target.starts_with(p.as_str())) {
            return false;
        }
        self.include_prefixes.iter().any(|p| target.starts_with(p.as_str()))
    }
}
```

---

## E. Language Bindings

### E.1 Python (PyO3)

#### Class Hierarchy

```rust
// bindings/python/src/lib.rs

use pyo3::prelude::*;
use mux_orm::{OrmServer, OrmSession, OrmWindow, OrmPane};

#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyServer>()?;
    m.add_class::<PySession>()?;
    m.add_class::<PyWindow>()?;
    m.add_class::<PyPane>()?;
    m.add_class::<PyQueryList>()?;
    Ok(())
}

/// Server manages a connection to a tmux/termforge server.
/// Thread-safe: all operations release the GIL during IO.
#[pyclass(name = "Server")]
pub struct PyServer {
    inner: Arc<Mutex<OrmServer>>,
}

#[pymethods]
impl PyServer {
    #[new]
    #[pyo3(signature = (*, socket_name=None, socket_path=None))]
    fn new(socket_name: Option<&str>, socket_path: Option<&str>) -> PyResult<Self> {
        let orm = OrmServer::connect(socket_name, socket_path)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyConnectionError, _>(e.to_string()))?;
        Ok(Self { inner: Arc::new(Mutex::new(orm)) })
    }

    /// Execute a tmux command string.
    fn cmd(&self, py: Python<'_>, cmd: &str) -> PyResult<String> {
        let inner = self.inner.clone();
        let cmd = cmd.to_string();
        // Release GIL during the blocking command execution
        py.allow_threads(move || {
            let server = inner.lock().map_err(|_| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("lock poisoned")
            })?;
            server.cmd(&cmd).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string())
            })
        })
    }

    /// Get all sessions, optionally filtered.
    #[getter]
    fn sessions(&self) -> PyResult<Vec<PySession>> {
        let server = self.inner.lock().map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("lock poisoned")
        })?;
        Ok(server.sessions().iter().map(|s| PySession {
            inner: s.clone(),
            server: self.inner.clone(),
        }).collect())
    }

    /// Kill the server.
    fn kill_server(&self, py: Python<'_>) -> PyResult<()> {
        let inner = self.inner.clone();
        py.allow_threads(move || {
            let server = inner.lock().map_err(|_| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("lock poisoned")
            })?;
            server.cmd("kill-server").map(|_| ()).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string())
            })
        })
    }

    fn __repr__(&self) -> String {
        format!("Server()")
    }
}

#[pyclass(name = "Session")]
pub struct PySession {
    inner: OrmSession,
    server: Arc<Mutex<OrmServer>>,
}

#[pymethods]
impl PySession {
    #[getter]
    fn name(&self) -> &str {
        self.inner.name()
    }

    #[getter]
    fn id(&self) -> String {
        self.inner.id()
    }

    #[getter]
    fn windows(&self) -> PyResult<Vec<PyWindow>> {
        Ok(self.inner.windows().iter().map(|w| PyWindow {
            inner: w.clone(),
            server: self.server.clone(),
        }).collect())
    }

    fn cmd(&self, py: Python<'_>, cmd: &str) -> PyResult<String> {
        let server = self.server.clone();
        let target = format!("-t {}", self.inner.id());
        let full_cmd = format!("{cmd} {target}");
        py.allow_threads(move || {
            let server = server.lock().map_err(|_| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("lock poisoned")
            })?;
            server.cmd(&full_cmd).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string())
            })
        })
    }

    fn __repr__(&self) -> String {
        format!("Session(name='{}')", self.inner.name())
    }
}

#[pyclass(name = "Window")]
pub struct PyWindow {
    inner: OrmWindow,
    server: Arc<Mutex<OrmServer>>,
}

#[pymethods]
impl PyWindow {
    #[getter]
    fn name(&self) -> &str { self.inner.name() }

    #[getter]
    fn id(&self) -> String { self.inner.id() }

    #[getter]
    fn panes(&self) -> PyResult<Vec<PyPane>> {
        Ok(self.inner.panes().iter().map(|p| PyPane {
            inner: p.clone(),
            server: self.server.clone(),
        }).collect())
    }

    fn __repr__(&self) -> String {
        format!("Window(name='{}')", self.inner.name())
    }
}

#[pyclass(name = "Pane")]
pub struct PyPane {
    inner: OrmPane,
    server: Arc<Mutex<OrmServer>>,
}

#[pymethods]
impl PyPane {
    #[getter]
    fn id(&self) -> String { self.inner.id() }

    fn send_keys(&self, py: Python<'_>, keys: &str) -> PyResult<()> {
        let server = self.server.clone();
        let target = self.inner.id();
        let cmd = format!("send-keys -t {target} {keys}");
        py.allow_threads(move || {
            let server = server.lock().map_err(|_| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("lock poisoned")
            })?;
            server.cmd(&cmd).map(|_| ()).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string())
            })
        })
    }

    fn capture_pane(&self, py: Python<'_>) -> PyResult<String> {
        let server = self.server.clone();
        let target = self.inner.id();
        let cmd = format!("capture-pane -p -t {target}");
        py.allow_threads(move || {
            let server = server.lock().map_err(|_| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("lock poisoned")
            })?;
            server.cmd(&cmd).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string())
            })
        })
    }

    fn __repr__(&self) -> String {
        format!("Pane(id='{}')", self.inner.id())
    }
}
```

#### GIL and Async

- All IO-bound methods use `py.allow_threads(...)` to release the GIL.
- Async Python (asyncio) support is deferred. Initial release uses synchronous API.
- The `OrmServer` mutex is never held across `py.allow_threads` -- we clone the `Arc` and lock inside the closure.

#### Error Mapping

| Rust Error | Python Exception |
|---|---|
| `ApiError::Backend` | `ConnectionError` or `RuntimeError` |
| `ApiError::Timeout` | `TimeoutError` |
| `ApiError::CommandFailed` | `RuntimeError` |
| `CoreError::InvalidCommand` | `ValueError` |
| `CoreError::Entity` | `KeyError` |
| Lock poisoned | `RuntimeError` |

#### Packaging

```toml
# bindings/python/pyproject.toml
[build-system]
requires = ["maturin>=1.5"]
build-backend = "maturin"

[project]
name = "termforge"
requires-python = ">=3.9"
classifiers = [
    "Programming Language :: Rust",
    "Programming Language :: Python :: Implementation :: CPython",
]

[tool.maturin]
features = ["pyo3/extension-module"]
```

### E.2 Node (NAPI-RS)

```rust
// bindings/node/src/lib.rs

use napi_derive::napi;
use napi::{Error, Result, Status};
use mux_orm::OrmServer;

#[napi]
pub struct Server {
    inner: OrmServer,
}

#[napi]
impl Server {
    #[napi(constructor)]
    pub fn new(socket_name: Option<String>, socket_path: Option<String>) -> Result<Self> {
        let orm = OrmServer::connect(
            socket_name.as_deref(),
            socket_path.as_deref(),
        ).map_err(|e| Error::new(Status::GenericFailure, e.to_string()))?;
        Ok(Self { inner: orm })
    }

    /// Execute a tmux command. Returns the command output.
    #[napi]
    pub fn cmd(&self, cmd: String) -> Result<String> {
        self.inner.cmd(&cmd)
            .map_err(|e| Error::new(Status::GenericFailure, e.to_string()))
    }

    /// Get all sessions.
    #[napi]
    pub fn sessions(&self) -> Result<Vec<Session>> {
        Ok(self.inner.sessions().iter()
            .map(|s| Session { name: s.name().to_string(), id: s.id() })
            .collect())
    }

    /// Kill the server.
    #[napi]
    pub fn kill_server(&self) -> Result<()> {
        self.inner.cmd("kill-server").map(|_| ())
            .map_err(|e| Error::new(Status::GenericFailure, e.to_string()))
    }
}

#[napi(object)]
pub struct Session {
    pub name: String,
    pub id: String,
}
```

Node bindings use NAPI-RS's built-in tokio integration for async operations. TypeScript declarations are auto-generated by NAPI-RS.

### E.3 C++ (cxx)

```rust
// crates/mux-cxx/src/lib.rs

#[cxx::bridge(namespace = "termforge")]
mod ffi {
    /// Server handle for C++ consumers.
    struct CxxServer {
        opaque: Box<ServerBridge>,
    }

    extern "Rust" {
        type ServerBridge;

        fn server_connect(
            socket_name: &str,
            socket_path: &str,
        ) -> Result<Box<ServerBridge>>;

        fn cmd(self: &ServerBridge, cmd: &str) -> Result<String>;
        fn session_count(self: &ServerBridge) -> usize;
        fn session_name(self: &ServerBridge, index: usize) -> Result<String>;
        fn kill_server(self: &ServerBridge) -> Result<()>;
    }
}

pub struct ServerBridge {
    inner: OrmServer,
}

fn server_connect(socket_name: &str, socket_path: &str) -> Result<Box<ServerBridge>, anyhow::Error> {
    let name = if socket_name.is_empty() { None } else { Some(socket_name) };
    let path = if socket_path.is_empty() { None } else { Some(socket_path) };
    let orm = OrmServer::connect(name, path)?;
    Ok(Box::new(ServerBridge { inner: orm }))
}
```

### E.4 Snapshot Reads from Bindings

All binding reads go through snapshots, never the mutable graph:

1. Binding calls `server.sessions()`.
2. `OrmServer` calls `self.api.snapshot()` which reads the current `ArcSwap<GraphState>`.
3. Snapshot data is copied into binding-native objects (PySession, JS Session, etc.).
4. The snapshot is immutable -- no locks held during binding-side iteration.

---

## F. CRDT Detail

### F.1 Concrete CRDT Types

```rust
// crates/mux-crdt/src/types.rs

/// Unique node identifier for a server in a collaborative session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(pub u64);

/// Hybrid Logical Clock timestamp.
/// Provides causal ordering across nodes.
///
/// Invariant: for any two events a, b on the same node,
///   a.counter < b.counter if a happened before b.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HlcTimestamp {
    /// Wall-clock milliseconds (injected, not read from OS in pure code).
    pub millis: u64,
    /// Logical counter, incremented on local events and merges.
    pub counter: u32,
    /// Originating node.
    pub node_id: NodeId,
}

/// Last-Writer-Wins Register.
/// The register with the highest timestamp wins.
#[derive(Debug, Clone)]
pub struct LwwRegister<T> {
    pub value: T,
    pub timestamp: HlcTimestamp,
}

impl<T: Clone> LwwRegister<T> {
    pub fn new(value: T, ts: HlcTimestamp) -> Self {
        Self { value, timestamp: ts }
    }

    /// Merge with another register. Higher timestamp wins.
    /// On tie, higher node_id wins (deterministic tiebreak).
    pub fn merge(&mut self, other: &Self) {
        if other.timestamp > self.timestamp {
            self.value = other.value.clone();
            self.timestamp = other.timestamp;
        }
    }
}

/// Add-Remove Set (OR-Set) for entity existence.
/// An entity exists if it has been added and not subsequently removed
/// with a higher timestamp.
#[derive(Debug, Clone)]
pub struct OrSet<K: Eq + Hash> {
    entries: HashMap<K, OrSetEntry>,
}

#[derive(Debug, Clone)]
struct OrSetEntry {
    add_ts: HlcTimestamp,
    remove_ts: Option<HlcTimestamp>,
}

impl<K: Eq + Hash + Clone> OrSet<K> {
    pub fn new() -> Self {
        Self { entries: HashMap::new() }
    }

    pub fn add(&mut self, key: K, ts: HlcTimestamp) {
        let entry = self.entries.entry(key).or_insert(OrSetEntry {
            add_ts: ts,
            remove_ts: None,
        });
        if ts > entry.add_ts {
            entry.add_ts = ts;
        }
        // If re-adding after removal, clear the removal if add is newer
        if let Some(remove_ts) = entry.remove_ts {
            if ts > remove_ts {
                entry.remove_ts = None;
            }
        }
    }

    pub fn remove(&mut self, key: &K, ts: HlcTimestamp) {
        if let Some(entry) = self.entries.get_mut(key) {
            match entry.remove_ts {
                Some(existing) if ts > existing => entry.remove_ts = Some(ts),
                None => entry.remove_ts = Some(ts),
                _ => {}
            }
        }
    }

    pub fn contains(&self, key: &K) -> bool {
        self.entries.get(key).map_or(false, |e| {
            e.remove_ts.map_or(true, |rt| e.add_ts > rt)
        })
    }

    pub fn merge(&mut self, other: &Self) {
        for (key, other_entry) in &other.entries {
            match self.entries.get_mut(key) {
                Some(entry) => {
                    if other_entry.add_ts > entry.add_ts {
                        entry.add_ts = other_entry.add_ts;
                    }
                    match (entry.remove_ts, other_entry.remove_ts) {
                        (Some(a), Some(b)) => entry.remove_ts = Some(a.max(b)),
                        (None, Some(b)) => entry.remove_ts = Some(b),
                        _ => {}
                    }
                }
                None => {
                    self.entries.insert(key.clone(), other_entry.clone());
                }
            }
        }
    }
}
```

### F.2 HLC Implementation

```rust
// crates/mux-crdt/src/clock.rs

/// Hybrid Logical Clock.
/// Pure: takes wall_clock_ms as an argument, never reads OS time.
pub struct HybridClock {
    node_id: NodeId,
    last: HlcTimestamp,
}

impl HybridClock {
    pub fn new(node_id: NodeId) -> Self {
        Self {
            node_id,
            last: HlcTimestamp { millis: 0, counter: 0, node_id },
        }
    }

    /// Generate a new timestamp for a local event.
    /// `wall_ms` is injected (from CoreCtx.now_ms in practice).
    pub fn now(&mut self, wall_ms: u64) -> HlcTimestamp {
        let millis = wall_ms.max(self.last.millis);
        let counter = if millis == self.last.millis {
            self.last.counter + 1
        } else {
            0
        };
        self.last = HlcTimestamp { millis, counter, node_id: self.node_id };
        self.last
    }

    /// Update the clock on receiving a remote timestamp.
    /// Returns a new local timestamp that is causally after both
    /// the local state and the received timestamp.
    pub fn receive(&mut self, remote: HlcTimestamp, wall_ms: u64) -> HlcTimestamp {
        let millis = wall_ms.max(self.last.millis).max(remote.millis);
        let counter = if millis == self.last.millis && millis == remote.millis {
            self.last.counter.max(remote.counter) + 1
        } else if millis == self.last.millis {
            self.last.counter + 1
        } else if millis == remote.millis {
            remote.counter + 1
        } else {
            0
        };
        self.last = HlcTimestamp { millis, counter, node_id: self.node_id };
        self.last
    }
}
```

### F.3 OpLog and Sync Protocol

```rust
// crates/mux-crdt/src/oplog.rs

/// A CRDT operation in the log.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum CrdtOp {
    /// Set an option value.
    SetOption {
        scope: OptionScope,
        key: String,
        value: String,
        ts: HlcTimestamp,
    },
    /// Put data into a paste buffer.
    BufferPut {
        name: String,
        data: Vec<u8>,
        ts: HlcTimestamp,
    },
    /// Delete a paste buffer.
    BufferDelete {
        name: String,
        ts: HlcTimestamp,
    },
    /// Create an entity (session, window).
    EntityCreate {
        kind: EntityKind,
        name: String,
        ts: HlcTimestamp,
    },
    /// Delete an entity.
    EntityDelete {
        kind: EntityKind,
        name: String,
        ts: HlcTimestamp,
    },
    /// Rename an entity.
    EntityRename {
        kind: EntityKind,
        old_name: String,
        new_name: String,
        ts: HlcTimestamp,
    },
}

impl CrdtOp {
    pub fn timestamp(&self) -> HlcTimestamp {
        match self {
            Self::SetOption { ts, .. }
            | Self::BufferPut { ts, .. }
            | Self::BufferDelete { ts, .. }
            | Self::EntityCreate { ts, .. }
            | Self::EntityDelete { ts, .. }
            | Self::EntityRename { ts, .. } => *ts,
        }
    }
}

/// Operation log: ordered sequence of CRDT ops.
/// Each node maintains its own log and exchanges ops during sync.
pub struct OpLog {
    ops: Vec<CrdtOp>,
    /// Track which ops we have seen from each node.
    /// Key: node_id, Value: highest counter we have from that node.
    vector_clock: HashMap<NodeId, u32>,
}

impl OpLog {
    pub fn new() -> Self {
        Self { ops: Vec::new(), vector_clock: HashMap::new() }
    }

    pub fn append(&mut self, op: CrdtOp) {
        let ts = op.timestamp();
        let entry = self.vector_clock.entry(ts.node_id).or_insert(0);
        *entry = (*entry).max(ts.counter);
        self.ops.push(op);
    }

    /// Get ops that the remote has not seen, based on their vector clock.
    pub fn ops_since(&self, remote_vc: &HashMap<NodeId, u32>) -> Vec<CrdtOp> {
        self.ops.iter().filter(|op| {
            let ts = op.timestamp();
            let remote_counter = remote_vc.get(&ts.node_id).copied().unwrap_or(0);
            ts.counter > remote_counter
        }).cloned().collect()
    }

    /// Merge incoming ops from a remote node.
    pub fn merge(&mut self, remote_ops: Vec<CrdtOp>) {
        for op in remote_ops {
            let ts = op.timestamp();
            let entry = self.vector_clock.entry(ts.node_id).or_insert(0);
            if ts.counter > *entry {
                *entry = ts.counter;
                self.ops.push(op);
            }
            // Duplicate ops (same node+counter) are silently ignored (idempotent)
        }
    }

    pub fn vector_clock(&self) -> &HashMap<NodeId, u32> {
        &self.vector_clock
    }
}
```

### F.4 Conflict Resolution Examples

**Example 1: Concurrent option set**

Node A sets `status-style "fg=red"` at HLC(100, 0, A).
Node B sets `status-style "fg=blue"` at HLC(100, 0, B).

Resolution: LWW with node_id tiebreak. If A.node_id > B.node_id, `fg=red` wins. Deterministic on all nodes.

**Example 2: Concurrent session create + delete**

Node A creates session "work" at HLC(100, 0, A).
Node B deletes session "work" at HLC(100, 1, B).

Resolution: Delete-wins-over-create for topology operations. The session is deleted because B's timestamp is higher (counter 1 > 0).

**Example 3: Concurrent buffer writes**

Node A writes "hello" to buffer "0" at HLC(100, 0, A).
Node B writes "world" to buffer "0" at HLC(100, 0, B).

Resolution: LWW register. Higher node_id wins. Both nodes converge to the same value.

---

## G. Control Mode

### G.1 Protocol Overview

tmux control mode (`tmux -C`) uses a line-based text protocol over stdin/stdout. Notifications are prefixed with `%%`. Command responses are bracketed:

```
%begin <timestamp> <command-number> <flags>
<output lines>
%end <timestamp> <command-number> <flags>

# or on error:
%begin <timestamp> <command-number> <flags>
<error message>
%error <timestamp> <command-number> <flags>
```

Studied from `~/study/c/tmux/control.c` (lines 29-138) and `control-notify.c`.

### G.2 Notification Types

From `~/study/c/tmux/control-notify.c`:

```rust
// crates/mux-control/src/notification.rs

/// Control mode notifications, matching tmux's control_notify_* functions
/// in control-notify.c.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum ControlNotification {
    // === Session lifecycle ===
    SessionsChanged,
    SessionChanged { session_id: u32, name: String },
    SessionRenamed { session_id: u32, name: String },
    SessionWindowChanged { session_id: u32, window_id: u32 },

    // === Window lifecycle ===
    WindowAdd { window_id: u32 },
    WindowClose { window_id: u32 },
    WindowRenamed { window_id: u32, name: String },
    WindowPaneChanged { window_id: u32, pane_id: u32 },

    // === Unlinked window variants ===
    UnlinkedWindowAdd { window_id: u32 },
    UnlinkedWindowClose { window_id: u32 },
    UnlinkedWindowRenamed { window_id: u32, name: String },

    // === Pane ===
    PaneModeChanged { pane_id: u32 },
    Output { pane_id: u32, data: Vec<u8> },

    // === Layout ===
    LayoutChange { window_id: u32, layout: String },

    // === Client ===
    ClientSessionChanged { client_name: String, session_id: u32, name: String },
    ClientDetached { client_name: String },

    // === Buffers ===
    PasteBufferChanged { name: String },
    PasteBufferDeleted { name: String },

    // === Subscription ===
    SubscriptionChanged { name: String, session_id: Option<u32>, window_id: Option<u32>, pane_id: Option<u32>, value: String },

    // === Pause ===
    Pause { pane_id: u32 },
    Continue { pane_id: u32 },

    // === Exit ===
    Exit { reason: Option<String> },
}
```

### G.3 Parser

```rust
// crates/mux-control/src/parser.rs

/// Parse a single control mode notification line.
/// Lines that do not start with `%` are command output (part of %begin/%end blocks).
pub fn parse_notification(line: &str) -> Result<ControlNotification, ControlParseError> {
    let line = line.trim_end_matches('\n');
    if !line.starts_with('%') {
        return Err(ControlParseError::NotANotification);
    }
    let parts: Vec<&str> = line.splitn(3, ' ').collect();
    let tag = parts[0];

    match tag {
        "%sessions-changed" => Ok(ControlNotification::SessionsChanged),
        "%session-changed" => {
            let session_id = parse_session_id(parts.get(1))?;
            let name = parts.get(2).unwrap_or(&"").to_string();
            Ok(ControlNotification::SessionChanged { session_id, name })
        }
        "%session-renamed" => {
            let session_id = parse_session_id(parts.get(1))?;
            let name = parts.get(2).unwrap_or(&"").to_string();
            Ok(ControlNotification::SessionRenamed { session_id, name })
        }
        "%session-window-changed" => {
            let session_id = parse_session_id(parts.get(1))?;
            let window_id = parse_window_id(parts.get(2))?;
            Ok(ControlNotification::SessionWindowChanged { session_id, window_id })
        }
        "%window-add" => {
            let window_id = parse_window_id(parts.get(1))?;
            Ok(ControlNotification::WindowAdd { window_id })
        }
        "%window-close" => {
            let window_id = parse_window_id(parts.get(1))?;
            Ok(ControlNotification::WindowClose { window_id })
        }
        "%window-renamed" => {
            let window_id = parse_window_id(parts.get(1))?;
            let name = parts.get(2).unwrap_or(&"").to_string();
            Ok(ControlNotification::WindowRenamed { window_id, name })
        }
        "%window-pane-changed" => {
            let window_id = parse_window_id(parts.get(1))?;
            let pane_id = parse_pane_id(parts.get(2))?;
            Ok(ControlNotification::WindowPaneChanged { window_id, pane_id })
        }
        "%pane-mode-changed" => {
            let pane_id = parse_pane_id(parts.get(1))?;
            Ok(ControlNotification::PaneModeChanged { pane_id })
        }
        "%output" => {
            let pane_id = parse_pane_id(parts.get(1))?;
            let data = parts.get(2)
                .map(|s| unescape_control_output(s))
                .unwrap_or_default();
            Ok(ControlNotification::Output { pane_id, data })
        }
        "%layout-change" => {
            // %layout-change @<window_id> <layout_string>
            let window_id = parse_window_id(parts.get(1))?;
            let layout = parts.get(2).unwrap_or(&"").to_string();
            Ok(ControlNotification::LayoutChange { window_id, layout })
        }
        "%client-session-changed" => {
            // %client-session-changed <client> $<session_id> <name>
            let client_name = parts.get(1).unwrap_or(&"").to_string();
            let rest: Vec<&str> = line.splitn(4, ' ').collect();
            let session_id = parse_session_id(rest.get(2))?;
            let name = rest.get(3).unwrap_or(&"").to_string();
            Ok(ControlNotification::ClientSessionChanged { client_name, session_id, name })
        }
        "%client-detached" => {
            let client_name = parts.get(1).unwrap_or(&"").to_string();
            Ok(ControlNotification::ClientDetached { client_name })
        }
        "%paste-buffer-changed" => {
            let name = parts.get(1).unwrap_or(&"").to_string();
            Ok(ControlNotification::PasteBufferChanged { name })
        }
        "%paste-buffer-deleted" => {
            let name = parts.get(1).unwrap_or(&"").to_string();
            Ok(ControlNotification::PasteBufferDeleted { name })
        }
        "%exit" => {
            let reason = parts.get(1).map(|s| s.to_string());
            Ok(ControlNotification::Exit { reason })
        }
        _ => Err(ControlParseError::UnknownNotification(tag.to_string())),
    }
}

fn parse_session_id(s: Option<&&str>) -> Result<u32, ControlParseError> {
    s.and_then(|s| s.strip_prefix('$'))
        .and_then(|s| s.parse().ok())
        .ok_or(ControlParseError::InvalidId("session"))
}

fn parse_window_id(s: Option<&&str>) -> Result<u32, ControlParseError> {
    s.and_then(|s| s.strip_prefix('@'))
        .and_then(|s| s.parse().ok())
        .ok_or(ControlParseError::InvalidId("window"))
}

fn parse_pane_id(s: Option<&&str>) -> Result<u32, ControlParseError> {
    s.and_then(|s| s.strip_prefix('%'))
        .and_then(|s| s.parse().ok())
        .ok_or(ControlParseError::InvalidId("pane"))
}

#[derive(Debug, Clone)]
pub enum ControlParseError {
    NotANotification,
    UnknownNotification(String),
    InvalidId(&'static str),
    MalformedLine(String),
}
```

### G.4 Control as Hints vs Binary as Authority

**Design principle:** Control mode notifications are *hints* that trigger a refresh. The binary protocol is the *authority* for state.

When `mux-api` receives a `%sessions-changed` notification, it does NOT directly mutate its view. Instead, it:
1. Records the hint type (`RefreshHint::SessionsChanged`).
2. Schedules a full view refresh via the backend.
3. The backend queries the server for authoritative state.
4. The view is atomically swapped via `ArcSwap`.

This means control mode is a push-notification optimization, not a state replication channel. If a notification is dropped or arrives out of order, the next periodic refresh corrects the view.

```rust
// crates/mux-api/src/managed.rs

impl SocketActor {
    fn notification_to_hint(&self, notif: &ControlNotification) -> RefreshHint {
        match notif {
            ControlNotification::SessionsChanged
            | ControlNotification::SessionChanged { .. }
            | ControlNotification::SessionRenamed { .. } => RefreshHint::Sessions,

            ControlNotification::WindowAdd { .. }
            | ControlNotification::WindowClose { .. }
            | ControlNotification::WindowRenamed { .. }
            | ControlNotification::SessionWindowChanged { .. } => RefreshHint::Windows,

            ControlNotification::WindowPaneChanged { .. }
            | ControlNotification::PaneModeChanged { .. } => RefreshHint::Panes,

            ControlNotification::LayoutChange { .. } => RefreshHint::Layout,

            ControlNotification::PasteBufferChanged { .. }
            | ControlNotification::PasteBufferDeleted { .. } => RefreshHint::Buffers,

            ControlNotification::Output { .. } => RefreshHint::PaneOutput,

            ControlNotification::ClientDetached { .. }
            | ControlNotification::ClientSessionChanged { .. } => RefreshHint::Clients,

            ControlNotification::Exit { .. } => RefreshHint::Disconnected,

            _ => RefreshHint::Full,
        }
    }
}
```

---

## H. Server Lifecycle

### H.1 Startup Sequence

Matching tmux's `server_start` in `~/study/c/tmux/server.c` lines 175-260:

```rust
// crates/mux-server/src/main.rs

pub async fn server_main(config: ServerConfig) -> Result<(), ServerError> {
    // 1. Acquire lock file (prevents duplicate servers on same socket)
    let lock = acquire_lock_file(&config.socket_path)?;

    // 2. Create Unix domain socket
    let listener = create_server_socket(&config.socket_path, &config.flags)?;

    // 3. Initialize global state
    let mut graph = ServerGraph::new();

    // 4. Load default key bindings
    key_bindings_init(&mut graph);

    // 5. Load config file (if exists)
    if let Some(config_path) = &config.config_file {
        load_config_file(config_path, &mut graph)?;
    }

    // 6. Set up channels
    let (event_tx, event_rx) = tokio::sync::mpsc::channel::<Event>(4096);
    let (effect_tx, effect_rx) = tokio::sync::mpsc::channel::<Effect>(4096);

    // 7. Start state actor
    let state_actor = StateActor::new(graph, event_rx, effect_tx.clone());
    let state_handle = tokio::spawn(state_actor.run());

    // 8. Start effect dispatcher
    let effect_dispatcher = EffectDispatcher::new(effect_rx, event_tx.clone());
    let effect_handle = tokio::spawn(effect_dispatcher.run());

    // 9. Start accept loop
    let accept_handle = tokio::spawn(accept_loop(listener, event_tx.clone()));

    // 10. Start config watcher (optional)
    if let Some(config_path) = &config.config_file {
        tokio::spawn(config_watcher(config_path.clone(), event_tx.clone()));
    }

    // 11. Start tidy timer (cleanup every hour, matching tmux)
    let tidy_handle = tokio::spawn(tidy_loop(event_tx.clone()));

    // 12. Wait for shutdown signal
    let shutdown = tokio::signal::ctrl_c();
    tokio::select! {
        _ = shutdown => {
            tracing::info!("received SIGINT, shutting down");
        }
        _ = state_handle => {
            tracing::info!("state actor exited");
        }
    }

    // 13. Graceful shutdown
    graceful_shutdown(event_tx, lock, &config.socket_path).await;

    Ok(())
}
```

### H.2 Client Handshake (Identify Burst)

When a tmux client connects, it sends an identify burst of 13 message types (MsgType 100-112). The server must receive all of them before the client is "identified".

```rust
// crates/mux-server/src/identify.rs

/// State machine for tracking the identify burst from a connecting client.
pub struct IdentifyState {
    received: HashSet<u32>,
    pub flags: u64,
    pub term: Option<String>,
    pub ttyname: Option<String>,
    pub cwd: Option<String>,
    pub environ: Vec<(String, String)>,
    pub client_pid: Option<u32>,
    pub features: Option<u32>,
    pub stdin_fd: Option<RawFd>,
    pub stdout_fd: Option<RawFd>,
}

impl IdentifyState {
    pub fn new() -> Self {
        Self {
            received: HashSet::new(),
            flags: 0,
            term: None, ttyname: None, cwd: None,
            environ: Vec::new(),
            client_pid: None, features: None,
            stdin_fd: None, stdout_fd: None,
        }
    }

    /// Process one identify message. Returns true when the burst is complete.
    pub fn process_frame(&mut self, frame: &ImsgFrame) -> Result<bool, ProtocolError> {
        let msg_type = frame.header.msg_type;
        match MsgType::from_u32(msg_type) {
            Some(MsgType::IdentifyFlags) => {
                self.flags = decode_u64(&frame.payload)?;
            }
            Some(MsgType::IdentifyTerm) => {
                self.term = Some(decode_nul_string(&frame.payload)?);
            }
            Some(MsgType::IdentifyTtyname) => {
                self.ttyname = Some(decode_nul_string(&frame.payload)?);
            }
            Some(MsgType::IdentifyCwd) => {
                self.cwd = Some(decode_nul_string(&frame.payload)?);
            }
            Some(MsgType::IdentifyEnviron) => {
                let kv = decode_nul_string(&frame.payload)?;
                if let Some((k, v)) = kv.split_once('=') {
                    self.environ.push((k.to_string(), v.to_string()));
                }
            }
            Some(MsgType::IdentifyClientpid) => {
                self.client_pid = Some(decode_u32(&frame.payload)?);
            }
            Some(MsgType::IdentifyFeatures) => {
                self.features = Some(decode_u32(&frame.payload)?);
            }
            Some(MsgType::IdentifyDone) => {
                self.received.insert(msg_type);
                return Ok(true); // burst complete
            }
            Some(MsgType::IdentifyStdin) | Some(MsgType::IdentifyStdout) => {
                // FD received via SCM_RIGHTS, stored separately
            }
            _ => {
                return Err(ProtocolError::NotIdentifyMessage { msg_type });
            }
        }
        self.received.insert(msg_type);
        Ok(false)
    }
}
```

### H.3 Graceful Shutdown

```rust
// crates/mux-server/src/shutdown.rs

/// Graceful shutdown sequence, matching tmux's server_send_exit
/// (~/study/c/tmux/server.c lines 309-329).
pub async fn graceful_shutdown(
    event_tx: mpsc::Sender<Event>,
    lock: LockFile,
    socket_path: &Path,
) {
    // 1. Send exit to all clients
    let _ = event_tx.send(Event::ServerShutdown).await;

    // 2. Wait for clients to disconnect (with timeout)
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if event_tx.is_closed() { break; }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }).await.ok();

    // 3. Remove socket file
    let _ = std::fs::remove_file(socket_path);

    // 4. Release lock file (RAII, but explicit for clarity)
    drop(lock);

    tracing::info!("server shutdown complete");
}
```

### H.4 Crash Recovery

If the server crashes, the socket file and lock file may remain. On next startup:

```rust
// crates/mux-server/src/lock.rs

pub struct LockFile {
    path: PathBuf,
    _fd: std::fs::File,
}

pub fn acquire_lock_file(socket_path: &Path) -> Result<LockFile, ServerError> {
    let lock_path = socket_path.with_extension("lock");

    // Try to create lock file with O_CREAT | O_EXCL
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock_path)
    {
        Ok(fd) => {
            // Write our PID
            use std::io::Write;
            let mut fd = fd;
            writeln!(fd, "{}", std::process::id()).ok();
            Ok(LockFile { path: lock_path, _fd: fd })
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            // Check if the PID in the lock file is still alive
            let content = std::fs::read_to_string(&lock_path).unwrap_or_default();
            let pid: u32 = content.trim().parse().unwrap_or(0);

            if pid > 0 && process_alive(pid) {
                Err(ServerError::Startup {
                    reason: format!(
                        "another server is running (pid {pid}, lock {lock_path:?})"
                    ),
                })
            } else {
                // Stale lock file from crashed server
                tracing::warn!("removing stale lock file from pid {pid}");
                std::fs::remove_file(&lock_path).ok();
                // Also remove stale socket
                std::fs::remove_file(socket_path).ok();
                // Retry
                acquire_lock_file(socket_path)
            }
        }
        Err(e) => Err(ServerError::Socket(e)),
    }
}

impl Drop for LockFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

#[cfg(unix)]
fn process_alive(pid: u32) -> bool {
    // kill(pid, 0) checks if process exists without sending a signal
    unsafe { libc::kill(pid as i32, 0) == 0 }
}
```

---

## I. Security Model

### I.1 Socket Permissions

Matching tmux's `server_create_socket` (`~/study/c/tmux/server.c` lines 107-153):

```rust
// crates/mux-os/src/socket.rs

/// Create the server socket with appropriate permissions.
/// Default socket: mode 0700 (owner-only).
/// Named socket (-L): mode 0770 (owner + group).
///
/// # Safety
/// Calls libc::umask, which is async-signal-unsafe but safe
/// when called before any async runtime is started.
pub fn create_server_socket(
    path: &Path,
    is_default_socket: bool,
) -> Result<UnixListener, std::io::Error> {
    // Remove any stale socket file
    let _ = std::fs::remove_file(path);

    let listener = UnixListener::bind(path)?;

    // Set socket file permissions
    // SAFETY: umask is safe when no other threads are running yet.
    // This is called during server startup before tokio::spawn.
    let mode = if is_default_socket {
        // Owner read/write/execute only (0700)
        0o077 // umask: block group and other
    } else {
        // Owner + group (0770)
        0o007 // umask: block other only
    };
    unsafe {
        let old_mask = libc::umask(mode);
        // Rebind with correct permissions
        let _ = std::fs::remove_file(path);
        let listener = UnixListener::bind(path)?;
        libc::umask(old_mask);
        return Ok(listener);
    }
}
```

### I.2 SCM_RIGHTS Security

File descriptors passed via SCM_RIGHTS must be validated:

```rust
// crates/mux-os/src/scm_rights.rs

/// Validate a received file descriptor before using it.
///
/// # Safety
/// The fd must be a valid, open file descriptor received via SCM_RIGHTS.
pub unsafe fn validate_received_fd(fd: RawFd) -> Result<ValidatedFd, ScmError> {
    // 1. Check fd is valid
    if fd < 0 {
        return Err(ScmError::InvalidFd(fd));
    }

    // 2. Check it is not stdin/stdout/stderr (0, 1, 2)
    if fd <= 2 {
        return Err(ScmError::ReservedFd(fd));
    }

    // 3. Verify it is a TTY (for stdin/stdout FDs from identify burst)
    // SAFETY: isatty is safe for any valid fd
    let is_tty = unsafe { libc::isatty(fd) == 1 };

    // 4. Set CLOEXEC to prevent leaking to child processes
    // SAFETY: fcntl with F_SETFD is safe for valid fds
    unsafe {
        let flags = libc::fcntl(fd, libc::F_GETFD);
        if flags < 0 {
            return Err(ScmError::FcntlFailed(std::io::Error::last_os_error()));
        }
        if libc::fcntl(fd, libc::F_SETFD, flags | libc::FD_CLOEXEC) < 0 {
            return Err(ScmError::FcntlFailed(std::io::Error::last_os_error()));
        }
    }

    Ok(ValidatedFd { fd, is_tty })
}

pub struct ValidatedFd {
    pub fd: RawFd,
    pub is_tty: bool,
}

#[derive(Debug)]
pub enum ScmError {
    InvalidFd(RawFd),
    ReservedFd(RawFd),
    FcntlFailed(std::io::Error),
    SendFailed(std::io::Error),
    RecvFailed(std::io::Error),
}
```

### I.3 Input Validation Boundaries

```
                     UNTRUSTED                    TRUSTED
                     ---------                    -------

  Socket bytes  --->  ImsgCodec  --[ProtocolError]--->  ImsgFrame
                      (validates header, length)

  ImsgFrame     --->  PayloadParser  --[ProtocolError]--->  TypedPayload
                      (validates NUL terminators, UTF-8, field ranges)

  TypedPayload  --->  CommandDispatch  --[CoreError]--->  Event
                      (validates command name, flags, targets)

  Event         --->  apply_event()  ---> graph mutation
                      (validates entity existence, constraints)
```

Every boundary produces a typed error. Nothing from the socket reaches the engine without validation.

### I.4 Pane Sandboxing

Panes run arbitrary user programs. Isolation measures:

1. **FD inheritance:** Only the PTY master FD is passed to child processes. All other server FDs have `CLOEXEC` set.
2. **Environment scrubbing:** The `TMUX` environment variable is set correctly for the child. Server-internal variables are not leaked.
3. **Signal isolation:** The server installs signal handlers for `SIGCHLD`, `SIGTERM`, `SIGINT`, `SIGUSR1`. Child processes inherit default signal disposition.
4. **Resource limits:** Scrollback history has a configurable maximum (`history-limit` option, default 2000 lines). Prevents runaway memory from a single pane.

```rust
// crates/mux-os/src/pty.rs

/// PTY spawn configuration with security defaults.
pub struct PtySpawnConfig {
    pub command: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub env: Vec<(String, String)>,
    pub size: PaneSize,
    /// Set CLOEXEC on all FDs >= 3 before exec.
    pub close_fds: bool,
}

impl Default for PtySpawnConfig {
    fn default() -> Self {
        Self {
            command: vec!["sh".into()],
            cwd: None,
            env: Vec::new(),
            size: PaneSize::DEFAULT,
            close_fds: true,
        }
    }
}
```

---

## J. Performance Targets

### J.1 Latency Budgets

| Operation | Target | Measurement Point |
|---|---|---|
| Keystroke to PTY write | < 1 ms | `Event::Key` received to `Effect::WritePane` dispatched |
| PTY output to screen update | < 5 ms | `Event::PaneOutput` to `Effect::Redraw` dispatched |
| Snapshot publish | < 2 ms | `graph_state()` to `ArcSwap::store()` |
| Protocol frame decode | < 10 us | `ImsgCodec::decode()` per frame |
| Protocol frame encode | < 10 us | `ImsgCodec::encode()` per frame |
| Format string expand | < 100 us | `format_expand()` for typical status line |
| Config file parse | < 10 ms | `parse()` for a 500-line tmux.conf |
| Layout resize | < 1 ms | `LayoutTree::resize()` for 20-pane window |
| Client identify burst | < 5 ms | First frame to `CLIENT_IDENTIFIED` |
| View capture (ORM) | < 5 ms | `OrmServer::sessions()` for 10 sessions |

### J.2 Memory Budgets

| Resource | Target | Notes |
|---|---|---|
| Base server (no sessions) | < 5 MB RSS | SlotMaps + key tables + options |
| Per pane (empty grid) | < 50 KB | Grid cells + parser state |
| Per pane (full scrollback, 2000 lines, 200 cols) | < 4 MB | `2000 * 200 * sizeof(Cell)` |
| Per `Cell` | 16 bytes | character (4) + fg (4) + bg (4) + attrs (4) |
| Per snapshot (10 sessions, 50 panes) | < 200 KB | Without grid data (grids are `Arc<Grid>`) |
| Grid snapshot (Arc COW) | 0 bytes extra | `Arc::clone` is pointer-copy until mutation |

### J.3 Parser Throughput

| Benchmark | Target | Notes |
|---|---|---|
| VT100 parser, plain ASCII | > 500 MB/s | Table-driven dispatch, no allocations on hot path |
| VT100 parser, heavy CSI | > 200 MB/s | CSI parameter parsing + grid updates |
| VT100 parser, UTF-8 text | > 300 MB/s | UTF-8 decode + wide character handling |
| Protocol codec decode | > 1 GB/s | 16-byte header + small payloads |
| Layout string parse | > 10 MB/s | Tree reconstruction from string |

### J.4 Benchmark Infrastructure

```rust
// benches/engine.rs
use criterion::{criterion_group, criterion_main, Criterion, black_box};

fn bench_apply_event(c: &mut Criterion) {
    let mut graph = ServerGraph::new();
    let session = graph.create_session("bench");
    let window = graph.create_window("w1");
    graph.add_window_to_session(session, window);
    let pane = graph.create_pane(PaneSize::DEFAULT);
    graph.add_pane_to_window(window, pane);
    let ctx = CoreCtx { now_ms: 0, rand_u64: 0 };

    c.bench_function("apply_event_pane_output_1kb", |b| {
        let data = vec![b'A'; 1024];
        b.iter(|| {
            let event = Event::PaneOutput {
                pane_id: pane,
                data: data.clone(),
            };
            black_box(apply_event(&mut graph, event, &ctx));
        });
    });

    c.bench_function("graph_state_snapshot", |b| {
        b.iter(|| {
            black_box(graph.graph_state());
        });
    });
}

fn bench_vt100_parser(c: &mut Criterion) {
    let mut grid = Grid::new(80, 24);
    let mut parser = Parser::new();
    let ascii_1mb: Vec<u8> = (0..1_000_000).map(|i| b'A' + (i % 26) as u8).collect();

    c.bench_function("vt100_parse_ascii_1mb", |b| {
        b.iter(|| {
            parser.parse(black_box(&ascii_1mb), &mut grid);
        });
    });

    let csi_heavy: Vec<u8> = b"\x1b[1;31mHello\x1b[0m \x1b[32mWorld\x1b[0m\n"
        .repeat(50_000)
        .into_iter()
        .collect();

    c.bench_function("vt100_parse_csi_heavy", |b| {
        b.iter(|| {
            parser.parse(black_box(&csi_heavy), &mut grid);
        });
    });
}

fn bench_format_expand(c: &mut Criterion) {
    let mut graph = ServerGraph::new();
    let session = graph.create_session("work");
    let ctx = FormatContext {
        graph: &graph,
        session: Some(session),
        window: None,
        pane: None,
        client: None,
        runtime_vars: &HashMap::new(),
    };

    c.bench_function("format_expand_status_line", |b| {
        let template = "[#S] #I:#W#{?window_flags,#{window_flags},}";
        b.iter(|| {
            black_box(format_expand(template, &ctx));
        });
    });
}

fn bench_layout_resize(c: &mut Criterion) {
    // Create a 20-pane tiled layout
    let panes: Vec<PaneId> = (0..20).map(|_| {
        // Generate fake PaneIds for benchmarking
        PaneId::default()
    }).collect();
    let mut tree = LayoutPreset::Tiled.arrange(&panes, 200, 60, None);

    c.bench_function("layout_resize_20_panes", |b| {
        b.iter(|| {
            tree.resize(black_box(180), black_box(50));
            tree.resize(black_box(200), black_box(60));
        });
    });
}

fn bench_codec(c: &mut Criterion) {
    use bytes::BytesMut;

    let frame = ImsgFrame {
        header: ImsgHdr {
            msg_type: 200,
            len: 16 + 64,
            peerid: 0,
            pid: 12345,
            has_fd: false,
        },
        payload: BytesMut::from(&[0u8; 64][..]),
    };

    c.bench_function("codec_encode_frame", |b| {
        let mut buf = BytesMut::with_capacity(128);
        let mut codec = ImsgCodec::new();
        b.iter(|| {
            buf.clear();
            codec.encode(black_box(frame.clone()), &mut buf).unwrap();
        });
    });

    c.bench_function("codec_decode_frame", |b| {
        let mut codec = ImsgCodec::new();
        let mut encode_buf = BytesMut::with_capacity(128);
        codec.encode(frame.clone(), &mut encode_buf).unwrap();
        let raw = encode_buf.freeze();

        b.iter(|| {
            let mut buf = BytesMut::from(&raw[..]);
            black_box(codec.decode(&mut buf).unwrap());
        });
    });
}

criterion_group!(
    benches,
    bench_apply_event,
    bench_vt100_parser,
    bench_format_expand,
    bench_layout_resize,
    bench_codec,
);
criterion_main!(benches);
```

### J.5 CI Performance Regression Gate

```yaml
# .github/workflows/bench.yml (excerpt)
- name: Run benchmarks
  run: cargo bench --bench engine -- --output-format bencher | tee bench_output.txt

- name: Compare against baseline
  uses: benchmark-action/github-action-benchmark@v1
  with:
    tool: 'cargo'
    output-file-path: bench_output.txt
    alert-threshold: '120%'  # fail if 20% slower
    fail-on-alert: true
    comment-on-alert: true
```

---

## Cross-Cutting: Updated Effect Enum

To support the features specified above, the Effect enum from v4 gains new variants:

```rust
// Additional Effect variants needed by v5 features:
pub enum Effect {
    // ... all v4 variants plus:

    /// Notify a control mode client of a state change.
    ControlNotify {
        client_id: ClientId,
        notification: ControlNotification,
    },

    /// Error notification to be sent to the client that caused it.
    ErrorReply {
        client_id: ClientId,
        error_message: String,
    },

    /// Request a config file reload.
    ReloadConfig {
        path: String,
    },

    /// CRDT operation to be replicated to peer nodes.
    ReplicateOp {
        op: CrdtOp,
    },
}
```

---

## Summary of What v5 Adds to v4

| Area | v4 State | v5 Contribution |
|---|---|---|
| Error Handling | "Return Result from all fallible operations" | Concrete error types per crate, propagation flow, codec recovery, SocketActor backoff |
| Configuration | "mux-conf: tmux.conf parser (PURE)" | Grammar PEG, AST types, option inheritance chain, hot reload, mux-conf/mux-command boundary |
| Layout Engine | "LayoutTree + tiling algorithms" (placeholder) | Full tree data structure, split/resize algorithms, 7 preset layouts, layout string serialization |
| OpenTelemetry | "tracing span context" (mux-telemetry) | Span catalog, pure/impure crossing, SocketActor integration, OTEL config spec |
| Language Bindings | "Thin wrapper philosophy" | Full PyO3 class hierarchy with GIL, NAPI-RS, cxx bridge, error mapping, snapshot reads |
| CRDT Detail | "CrdtOp, HLC, merge semantics" (sketched) | LwwRegister, OrSet, HLC implementation, OpLog with vector clocks, conflict resolution examples |
| Control Mode | "Control mode parser" (placeholder) | Full notification enum, parser implementation, hints-vs-authority design, command response framing |
| Server Lifecycle | "tokio runtime, socket accept loop" | 13-step startup, identify burst state machine, graceful shutdown, crash recovery with lock files |
| Security Model | "SCM_RIGHTS security" (placeholder) | Socket permissions, FD validation, input validation boundaries, pane sandboxing |
| Performance Targets | Not specified | Latency/memory budgets, parser throughput, criterion benchmarks, CI regression gate |
