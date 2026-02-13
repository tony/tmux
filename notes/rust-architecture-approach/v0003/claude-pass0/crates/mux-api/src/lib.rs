//! # mux-api
//!
//! SDK entry point for TermForge. Provides a builder-pattern API for
//! creating and managing a terminal multiplexer server.
//!
//! ## Gap-11 (SDK Ergonomics)
//! This crate is the primary entry point. Users interact with TermForge
//! through `MuxServer::builder()`.
//!
//! ## Gap-12 (Graphics Passthrough)
//! Graphics protocols (sixel, iTerm2, Kitty) are acknowledged as future
//! considerations. The `allow_passthrough` option defaults to `false` (INV-220).
//! DCS sequences from these protocols are consumed and discarded when disabled.
//!
//! L4 integration crate.

#![forbid(unsafe_code)]

use mux_kernel::Kernel;
use mux_pty::TerminalModePolicy;
use mux_time::Clock;
use mux_types::Size;
use thiserror::Error;

/// API errors.
#[derive(Debug, Error)]
pub enum ApiError {
    /// Configuration error.
    #[error("config error: {0}")]
    Config(String),
    /// Kernel error.
    #[error("kernel error: {0}")]
    Kernel(String),
    /// The server is not running.
    #[error("server not running")]
    NotRunning,
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
}

impl MuxServerBuilder {
    /// Create a new builder with defaults.
    #[must_use]
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

    /// Build the MuxServer.
    ///
    /// # Errors
    ///
    /// Returns `ApiError::Config` if configuration is invalid.
    pub fn build(self) -> Result<MuxServer, ApiError> {
        let clock = self.clock.unwrap_or_else(Clock::system);
        let kernel = Kernel::new(clock);

        Ok(MuxServer {
            kernel,
            terminal_mode: self.terminal_mode,
            socket_path: self.socket_path,
            default_shell: self.default_shell,
            running: false,
        })
    }
}

/// The main TermForge server handle.
pub struct MuxServer {
    kernel: Kernel,
    terminal_mode: TerminalModePolicy,
    socket_path: String,
    default_shell: String,
    running: bool,
}

impl MuxServer {
    /// Create a new builder.
    #[must_use]
    pub fn builder() -> MuxServerBuilder {
        MuxServerBuilder::new()
    }

    /// Create a new session.
    ///
    /// # Errors
    ///
    /// Returns `ApiError::Kernel` if the session cannot be created.
    pub fn new_session(&mut self, name: &str) -> Result<mux_types::SessionId, ApiError> {
        let (id, _effects) = self.kernel.create_session(name, Size::new(80, 24));
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_defaults() {
        let server = MuxServer::builder()
            .clock(Clock::manual())
            .build();
        assert!(server.is_ok());
        let server = server.unwrap_or_else(|_| MuxServer::builder().clock(Clock::manual()).build().unwrap_or_else(|_| unreachable!()));
        assert_eq!(server.session_count(), 0);
    }

    #[test]
    fn builder_custom_config() {
        let server = MuxServer::builder()
            .terminal_mode(TerminalModePolicy::External)
            .socket_path("/tmp/test.sock")
            .history_limit(50_000)
            .default_shell("/bin/zsh")
            .escape_time(100)
            .focus_events(true)
            .allow_passthrough(false)
            .clock(Clock::manual())
            .build();
        assert!(server.is_ok());
        let server = server.unwrap_or_else(|_| unreachable!());
        assert_eq!(server.socket_path(), "/tmp/test.sock");
        assert_eq!(server.terminal_mode(), TerminalModePolicy::External);
        assert_eq!(server.default_shell(), "/bin/zsh");
    }

    #[test]
    fn create_session() {
        let mut server = MuxServer::builder()
            .clock(Clock::manual())
            .build()
            .unwrap_or_else(|_| unreachable!());
        let id = server.new_session("test");
        assert!(id.is_ok());
        assert_eq!(server.session_count(), 1);
    }

    #[test]
    fn multiple_sessions() {
        let mut server = MuxServer::builder()
            .clock(Clock::manual())
            .build()
            .unwrap_or_else(|_| unreachable!());
        let _ = server.new_session("s1");
        let _ = server.new_session("s2");
        assert_eq!(server.session_count(), 2);
    }

    #[test]
    fn passthrough_default_off() {
        // INV-220: verify graphics passthrough defaults to off
        let builder = MuxServerBuilder::new();
        assert!(!builder.allow_passthrough);
    }
}
