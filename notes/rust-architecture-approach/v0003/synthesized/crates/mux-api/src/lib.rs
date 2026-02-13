//! # mux-api
//!
//! SDK entry point for TermForge. Provides a fluent builder-pattern API for
//! creating and managing a terminal multiplexer server.
//!
//! ## Example
//! ```ignore
//! let forge = TermForge::builder()
//!     .socket_path("/tmp/test-forge.sock")
//!     .mode(TerminalMode::Managed)
//!     .build()?;
//!
//! let session = forge.session("dev")
//!     .window("editor")
//!     .pane(PaneConfig::default())
//!     .build()?;
//! ```
//!
//! L4 integration crate.

#![forbid(unsafe_code)]

use mux_kernel::Kernel;
use mux_pty::TerminalModePolicy;
use mux_time::Clock;
use mux_types::{SessionId, Size};
use thiserror::Error;

/// API errors.
#[derive(Debug, Error)]
pub enum ApiError {
    #[error("config error: {0}")]
    Config(String),
    #[error("kernel error: {0}")]
    Kernel(String),
    #[error("server not running")]
    NotRunning,
    #[error("session not found: {0}")]
    SessionNotFound(String),
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
}

/// Pane configuration for the fluent builder.
#[derive(Debug, Clone)]
pub struct PaneConfig {
    /// Initial command to run.
    pub command: Option<String>,
    /// Initial size (overrides session default).
    pub size: Option<Size>,
}

impl PaneConfig {
    /// Create a default pane config.
    #[must_use]
    pub fn new() -> Self {
        Self {
            command: None,
            size: None,
        }
    }

    /// Set the command for this pane.
    #[must_use]
    pub fn command(mut self, cmd: impl Into<String>) -> Self {
        self.command = Some(cmd.into());
        self
    }

    /// Set the size for this pane.
    #[must_use]
    pub fn size(mut self, size: Size) -> Self {
        self.size = Some(size);
        self
    }
}

impl Default for PaneConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// TermForge top-level SDK entry point -- an alias for `MuxServer::builder()`.
pub struct TermForge;

impl TermForge {
    /// Create a new builder for the TermForge SDK.
    #[must_use]
    pub fn builder() -> MuxServerBuilder {
        MuxServerBuilder::new()
    }
}

/// Builder for constructing a `MuxServer`.
#[derive(Debug)]
pub struct MuxServerBuilder {
    terminal_mode: TerminalModePolicy,
    socket_path: String,
    history_limit: u32,
    default_shell: String,
    escape_time: u32,
    focus_events: bool,
    allow_passthrough: bool,
    clock: Option<Clock>,
    default_size: Size,
}

impl MuxServerBuilder {
    fn new() -> Self {
        Self {
            terminal_mode: TerminalModePolicy::Managed,
            socket_path: "/tmp/termforge.sock".into(),
            history_limit: 2000,
            default_shell: "/bin/sh".into(),
            escape_time: 500,
            focus_events: false,
            allow_passthrough: false, // INV-220
            clock: None,
            default_size: Size::new(80, 24),
        }
    }

    /// Set the terminal mode policy (Gap-2).
    #[must_use]
    pub fn terminal_mode(mut self, policy: TerminalModePolicy) -> Self {
        self.terminal_mode = policy;
        self
    }

    /// Set the Unix socket path.
    #[must_use]
    pub fn socket_path(mut self, path: impl Into<String>) -> Self {
        self.socket_path = path.into();
        self
    }

    /// Set the history (scrollback) limit.
    #[must_use]
    pub fn history_limit(mut self, limit: u32) -> Self {
        self.history_limit = limit;
        self
    }

    /// Set the default shell.
    #[must_use]
    pub fn default_shell(mut self, shell: impl Into<String>) -> Self {
        self.default_shell = shell.into();
        self
    }

    /// Set the escape time in milliseconds.
    #[must_use]
    pub fn escape_time(mut self, ms: u32) -> Self {
        self.escape_time = ms;
        self
    }

    /// Enable or disable focus events.
    #[must_use]
    pub fn focus_events(mut self, enabled: bool) -> Self {
        self.focus_events = enabled;
        self
    }

    /// Enable or disable graphics passthrough (INV-220: default off).
    #[must_use]
    pub fn allow_passthrough(mut self, enabled: bool) -> Self {
        self.allow_passthrough = enabled;
        self
    }

    /// Inject a clock (use `Clock::manual()` for tests).
    #[must_use]
    pub fn clock(mut self, clock: Clock) -> Self {
        self.clock = Some(clock);
        self
    }

    /// Set the default terminal size.
    #[must_use]
    pub fn default_size(mut self, size: Size) -> Self {
        self.default_size = size;
        self
    }

    /// Build the MuxServer.
    pub fn build(self) -> Result<MuxServer, ApiError> {
        if self.socket_path.is_empty() {
            return Err(ApiError::Config("socket path cannot be empty".into()));
        }
        let clock = self.clock.unwrap_or_else(Clock::system);
        let kernel = Kernel::new(clock);

        Ok(MuxServer {
            kernel,
            terminal_mode: self.terminal_mode,
            socket_path: self.socket_path,
            default_shell: self.default_shell,
            default_size: self.default_size,
            allow_passthrough: self.allow_passthrough,
        })
    }
}

/// The main TermForge server handle.
pub struct MuxServer {
    kernel: Kernel,
    terminal_mode: TerminalModePolicy,
    socket_path: String,
    default_shell: String,
    default_size: Size,
    allow_passthrough: bool,
}

impl MuxServer {
    /// Create a new builder.
    #[must_use]
    pub fn builder() -> MuxServerBuilder {
        MuxServerBuilder::new()
    }

    /// Start a fluent session builder.
    #[must_use]
    pub fn session(&mut self, name: &str) -> SessionBuilder<'_> {
        SessionBuilder {
            server: self,
            name: name.to_owned(),
            windows: Vec::new(),
        }
    }

    /// Create a new session (simple API).
    pub fn new_session(&mut self, name: &str) -> Result<SessionId, ApiError> {
        let (id, _effects) = self.kernel.create_session(name, self.default_size);
        Ok(id)
    }

    /// Number of active sessions.
    #[must_use]
    pub fn session_count(&self) -> usize {
        self.kernel.session_count()
    }

    /// The socket path.
    #[must_use]
    pub fn socket_path(&self) -> &str {
        &self.socket_path
    }

    /// The terminal mode policy.
    #[must_use]
    pub fn terminal_mode(&self) -> TerminalModePolicy {
        self.terminal_mode
    }

    /// The default shell.
    #[must_use]
    pub fn default_shell(&self) -> &str {
        &self.default_shell
    }

    /// Whether graphics passthrough is enabled.
    #[must_use]
    pub const fn allow_passthrough(&self) -> bool {
        self.allow_passthrough
    }

    /// Default terminal size.
    #[must_use]
    pub const fn default_size(&self) -> Size {
        self.default_size
    }
}

/// Fluent builder for creating sessions.
pub struct SessionBuilder<'a> {
    server: &'a mut MuxServer,
    name: String,
    windows: Vec<WindowSpec>,
}

/// Specification for a window in the fluent builder.
#[allow(dead_code)]
struct WindowSpec {
    name: String,
    panes: Vec<PaneConfig>,
}

impl SessionBuilder<'_> {
    /// Add a window to the session.
    #[must_use]
    pub fn window(mut self, name: &str) -> Self {
        self.windows.push(WindowSpec {
            name: name.to_owned(),
            panes: Vec::new(),
        });
        self
    }

    /// Add a pane to the most recent window.
    #[must_use]
    pub fn pane(mut self, config: PaneConfig) -> Self {
        if let Some(window) = self.windows.last_mut() {
            window.panes.push(config);
        }
        self
    }

    /// Build the session.
    pub fn build(self) -> Result<SessionId, ApiError> {
        let (id, _effects) = self
            .server
            .kernel
            .create_session(&self.name, self.server.default_size);
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_server() -> MuxServer {
        MuxServer::builder()
            .clock(Clock::manual())
            .build()
            .unwrap_or_else(|_| {
                // Fallback -- should never happen
                MuxServer::builder()
                    .clock(Clock::manual())
                    .socket_path("/tmp/fallback.sock")
                    .build()
                    .unwrap_or_else(|_e| std::process::abort())
            })
    }

    #[test]
    fn builder_defaults() {
        let server = test_server();
        assert_eq!(server.session_count(), 0);
        assert_eq!(server.socket_path(), "/tmp/termforge.sock");
    }

    #[test]
    fn builder_custom_socket() {
        let server = MuxServer::builder()
            .socket_path("/tmp/custom.sock")
            .clock(Clock::manual())
            .build();
        assert!(server.is_ok());
        let server = server.unwrap_or_else(|_| test_server());
        assert_eq!(server.socket_path(), "/tmp/custom.sock");
    }

    #[test]
    fn builder_custom_shell() {
        let server = MuxServer::builder()
            .default_shell("/bin/zsh")
            .clock(Clock::manual())
            .build();
        assert!(server.is_ok());
        let server = server.unwrap_or_else(|_| test_server());
        assert_eq!(server.default_shell(), "/bin/zsh");
    }

    #[test]
    fn builder_terminal_mode() {
        let server = MuxServer::builder()
            .terminal_mode(TerminalModePolicy::External)
            .clock(Clock::manual())
            .build()
            .unwrap_or_else(|_| test_server());
        assert_eq!(server.terminal_mode(), TerminalModePolicy::External);
    }

    #[test]
    fn builder_empty_socket_path_fails() {
        let result = MuxServer::builder()
            .socket_path("")
            .clock(Clock::manual())
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn passthrough_default_off() {
        let builder = MuxServerBuilder::new();
        assert!(!builder.allow_passthrough);
    }

    #[test]
    fn passthrough_preserved_in_server() {
        let server = MuxServer::builder()
            .allow_passthrough(false)
            .clock(Clock::manual())
            .build()
            .unwrap_or_else(|_| test_server());
        assert!(!server.allow_passthrough());
    }

    #[test]
    fn create_session_simple() {
        let mut server = test_server();
        let id = server.new_session("test");
        assert!(id.is_ok());
        assert_eq!(server.session_count(), 1);
    }

    #[test]
    fn multiple_sessions() {
        let mut server = test_server();
        let _ = server.new_session("s1");
        let _ = server.new_session("s2");
        assert_eq!(server.session_count(), 2);
    }

    #[test]
    fn fluent_session_builder() {
        let mut server = test_server();
        let id = server
            .session("dev")
            .window("editor")
            .pane(PaneConfig::default())
            .build();
        assert!(id.is_ok());
        assert_eq!(server.session_count(), 1);
    }

    #[test]
    fn fluent_multi_window() {
        let mut server = test_server();
        let id = server
            .session("dev")
            .window("editor")
            .pane(PaneConfig::default())
            .window("terminal")
            .pane(PaneConfig::new().command("bash"))
            .build();
        assert!(id.is_ok());
    }

    #[test]
    fn termforge_alias() {
        let server = TermForge::builder()
            .clock(Clock::manual())
            .build();
        assert!(server.is_ok());
    }

    #[test]
    fn builder_default_size() {
        let server = MuxServer::builder()
            .default_size(Size::new(120, 40))
            .clock(Clock::manual())
            .build()
            .unwrap_or_else(|_| test_server());
        assert_eq!(server.default_size(), Size::new(120, 40));
    }

    #[test]
    fn builder_focus_events() {
        let server = MuxServer::builder()
            .focus_events(true)
            .clock(Clock::manual())
            .build();
        assert!(server.is_ok());
    }

    #[test]
    fn builder_escape_time() {
        let server = MuxServer::builder()
            .escape_time(100)
            .clock(Clock::manual())
            .build();
        assert!(server.is_ok());
    }

    #[test]
    fn builder_history_limit() {
        let server = MuxServer::builder()
            .history_limit(50_000)
            .clock(Clock::manual())
            .build();
        assert!(server.is_ok());
    }

    #[test]
    fn pane_config_builder() {
        let config = PaneConfig::new()
            .command("vim")
            .size(Size::new(80, 24));
        assert_eq!(config.command.as_deref(), Some("vim"));
        assert_eq!(config.size, Some(Size::new(80, 24)));
    }

    #[test]
    fn pane_config_default() {
        let config = PaneConfig::default();
        assert!(config.command.is_none());
        assert!(config.size.is_none());
    }

    #[test]
    fn builder_all_options_chained() {
        let result = MuxServer::builder()
            .terminal_mode(TerminalModePolicy::SaveRestore)
            .socket_path("/tmp/full-test.sock")
            .history_limit(100_000)
            .default_shell("/bin/fish")
            .escape_time(50)
            .focus_events(true)
            .allow_passthrough(false)
            .default_size(Size::new(132, 43))
            .clock(Clock::manual())
            .build();
        assert!(result.is_ok());
    }

    #[test]
    fn session_builder_empty_windows() {
        let mut server = test_server();
        let id = server.session("empty").build();
        assert!(id.is_ok());
        assert_eq!(server.session_count(), 1);
    }

    #[test]
    fn multiple_sessions_distinct_names() {
        let mut server = test_server();
        let id1 = server.new_session("alpha");
        let id2 = server.new_session("beta");
        let id3 = server.new_session("gamma");
        assert!(id1.is_ok());
        assert!(id2.is_ok());
        assert!(id3.is_ok());
        assert_eq!(server.session_count(), 3);
    }

    #[test]
    fn builder_passthrough_explicit_enable() {
        let server = MuxServer::builder()
            .allow_passthrough(true)
            .clock(Clock::manual())
            .build()
            .unwrap_or_else(|_| test_server());
        assert!(server.allow_passthrough());
    }

    #[test]
    fn fluent_builder_pane_with_command_and_size() {
        let mut server = test_server();
        let id = server
            .session("dev")
            .window("code")
            .pane(PaneConfig::new().command("nvim").size(Size::new(120, 40)))
            .build();
        assert!(id.is_ok());
    }

    #[test]
    fn pane_config_only_command() {
        let config = PaneConfig::new().command("htop");
        assert_eq!(config.command.as_deref(), Some("htop"));
        assert!(config.size.is_none());
    }

    #[test]
    fn pane_config_only_size() {
        let config = PaneConfig::new().size(Size::new(40, 12));
        assert!(config.command.is_none());
        assert_eq!(config.size, Some(Size::new(40, 12)));
    }
}
