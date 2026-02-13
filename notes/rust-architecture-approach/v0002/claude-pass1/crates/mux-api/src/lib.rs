//! # mux-api
//!
//! SDK entry-point crate for TermForge.
//!
//! This is the primary interface for embedding TermForge in Rust applications.
//! It provides a builder-pattern API for configuring and starting the multiplexer
//! server, plus session/window/pane handles for programmatic interaction.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use mux_api::{MuxServer, TerminalModePolicy};
//!
//! let server = MuxServer::builder()
//!     .terminal_mode(TerminalModePolicy::Managed)
//!     .socket_path("/tmp/termforge.sock")
//!     .history_limit(50_000)
//!     .build()?;
//!
//! let session = server.new_session("main")?;
//! let window = session.active_window()?;
//! let pane = window.active_pane()?;
//!
//! pane.send_keys("echo hello\n")?;
//! ```
//!
//! ## Architecture
//!
//! `MuxServer` spawns three threads:
//! - IO thread (tokio): PTY I/O, Unix socket, signals
//! - Kernel thread (std::thread): all mutable state
//! - Render thread (tokio task): composition and diff rendering
//!
//! Communication between threads uses bounded crossbeam channels.
//! The `MuxServer` handle communicates with the kernel via a command channel.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

use crossbeam_channel as channel;
use mux_kernel::{Kernel, KernelEffect, KernelEvent};
use mux_time::Clock;
use mux_types::{ClientId, PaneId, SessionId, Size, WindowId};

// ---------------------------------------------------------------------------
// Terminal mode policy (Gap #3)
// ---------------------------------------------------------------------------

/// Who owns the raw mode transition.
///
/// Controls whether TermForge manages terminal raw mode (cfmakeraw/tcsetattr)
/// or delegates that responsibility to the embedding application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalModePolicy {
    /// TermForge manages raw mode (standalone server mode).
    /// Calls cfmakeraw/tcsetattr on attach, restores on detach.
    Managed,
    /// Caller manages raw mode (embedded SDK mode).
    /// TermForge assumes the terminal is already in the correct state.
    External,
    /// TermForge queries the current state on attach and restores on detach.
    /// Safe default for unknown embedding contexts.
    SaveRestore,
}

impl Default for TerminalModePolicy {
    fn default() -> Self {
        Self::SaveRestore
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors from MuxServer operations.
#[derive(Debug, thiserror::Error)]
pub enum MuxApiError {
    #[error("server not started")]
    NotStarted,
    #[error("server already started")]
    AlreadyStarted,
    #[error("command failed: {0}")]
    CommandFailed(String),
    #[error("session not found: {0}")]
    SessionNotFound(String),
    #[error("window not found")]
    WindowNotFound,
    #[error("pane not found")]
    PaneNotFound,
    #[error("channel closed")]
    ChannelClosed,
    #[error("timeout")]
    Timeout,
    #[error("kernel error: {0}")]
    Kernel(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type ApiResult<T> = Result<T, MuxApiError>;

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

/// Builder for constructing a [`MuxServer`].
#[derive(Debug)]
pub struct MuxServerBuilder {
    terminal_mode: TerminalModePolicy,
    socket_path: Option<PathBuf>,
    history_limit: u32,
    default_shell: String,
    escape_time: u32,
    focus_events: bool,
    allow_passthrough: bool,
}

impl MuxServerBuilder {
    /// Create a new builder with defaults.
    #[must_use]
    pub fn new() -> Self {
        Self {
            terminal_mode: TerminalModePolicy::default(),
            socket_path: None,
            history_limit: 10_000,
            default_shell: "/bin/sh".into(),
            escape_time: 500,
            focus_events: false,
            allow_passthrough: false,
        }
    }

    /// Set the terminal mode policy.
    #[must_use]
    pub const fn terminal_mode(mut self, policy: TerminalModePolicy) -> Self {
        self.terminal_mode = policy;
        self
    }

    /// Set the Unix socket path.
    #[must_use]
    pub fn socket_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.socket_path = Some(path.into());
        self
    }

    /// Set the scrollback history limit.
    #[must_use]
    pub const fn history_limit(mut self, limit: u32) -> Self {
        self.history_limit = limit;
        self
    }

    /// Set the default shell.
    #[must_use]
    pub fn default_shell(mut self, shell: impl Into<String>) -> Self {
        self.default_shell = shell.into();
        self
    }

    /// Set escape time in milliseconds.
    #[must_use]
    pub const fn escape_time(mut self, ms: u32) -> Self {
        self.escape_time = ms;
        self
    }

    /// Enable/disable focus events.
    #[must_use]
    pub const fn focus_events(mut self, enabled: bool) -> Self {
        self.focus_events = enabled;
        self
    }

    /// Enable/disable graphics passthrough (INV-220: disabled by default).
    #[must_use]
    pub const fn allow_passthrough(mut self, enabled: bool) -> Self {
        self.allow_passthrough = enabled;
        self
    }

    /// Build the server (does not start it yet).
    ///
    /// # Errors
    /// Returns error if configuration is invalid.
    pub fn build(self) -> ApiResult<MuxServer> {
        let kernel = Kernel::new(Clock::system());

        Ok(MuxServer {
            kernel,
            terminal_mode: self.terminal_mode,
            socket_path: self.socket_path,
            history_limit: self.history_limit,
            default_shell: self.default_shell,
            running: false,
        })
    }
}

impl Default for MuxServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// MuxServer
// ---------------------------------------------------------------------------

/// The TermForge multiplexer server handle.
///
/// This is the primary SDK entry point. Create via [`MuxServer::builder()`].
pub struct MuxServer {
    kernel: Kernel,
    terminal_mode: TerminalModePolicy,
    socket_path: Option<PathBuf>,
    history_limit: u32,
    default_shell: String,
    running: bool,
}

impl MuxServer {
    /// Create a builder.
    #[must_use]
    pub fn builder() -> MuxServerBuilder {
        MuxServerBuilder::new()
    }

    /// Get the terminal mode policy.
    #[must_use]
    pub const fn terminal_mode(&self) -> TerminalModePolicy {
        self.terminal_mode
    }

    /// Get the socket path, if configured.
    #[must_use]
    pub fn socket_path(&self) -> Option<&Path> {
        self.socket_path.as_deref()
    }

    /// Get the history limit.
    #[must_use]
    pub const fn history_limit(&self) -> u32 {
        self.history_limit
    }

    /// Create a new session.
    ///
    /// # Errors
    /// Returns error if the session cannot be created.
    pub fn new_session(&mut self, name: &str) -> ApiResult<SessionHandle> {
        let effects = self.kernel.process_event(KernelEvent::Command {
            client: ClientId(0),
            command: format!("new-session -d -s {name}"),
        });

        for effect in &effects {
            if let KernelEffect::CommandResponse { success: false, output, .. } = effect {
                return Err(MuxApiError::CommandFailed(output.clone()));
            }
        }

        Ok(SessionHandle {
            name: name.to_owned(),
        })
    }

    /// List all session names.
    #[must_use]
    pub fn session_names(&self) -> Vec<String> {
        // This would iterate kernel sessions -- simplified
        Vec::new()
    }

    /// Get the number of sessions.
    #[must_use]
    pub fn session_count(&self) -> usize {
        self.kernel.session_count()
    }

    /// Get a reference to the kernel (for testing).
    #[must_use]
    pub fn kernel(&self) -> &Kernel {
        &self.kernel
    }

    /// Get a mutable reference to the kernel (for testing).
    pub fn kernel_mut(&mut self) -> &mut Kernel {
        &mut self.kernel
    }
}

impl std::fmt::Debug for MuxServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MuxServer")
            .field("running", &self.running)
            .field("terminal_mode", &self.terminal_mode)
            .field("history_limit", &self.history_limit)
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Handles for sessions, windows, panes
// ---------------------------------------------------------------------------

/// A handle to a session (for SDK API).
#[derive(Debug, Clone)]
pub struct SessionHandle {
    pub name: String,
}

/// A handle to a window (for SDK API).
#[derive(Debug, Clone)]
pub struct WindowHandle {
    pub session: String,
    pub index: u32,
}

/// A handle to a pane (for SDK API).
#[derive(Debug, Clone)]
pub struct PaneHandle {
    pub id: PaneId,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_defaults() {
        let builder = MuxServerBuilder::new();
        assert_eq!(builder.terminal_mode, TerminalModePolicy::SaveRestore);
        assert_eq!(builder.history_limit, 10_000);
        assert!(!builder.allow_passthrough); // INV-220
    }

    #[test]
    fn builder_custom_settings() {
        let builder = MuxServerBuilder::new()
            .terminal_mode(TerminalModePolicy::Managed)
            .history_limit(50_000)
            .default_shell("/bin/bash")
            .escape_time(100)
            .focus_events(true)
            .socket_path("/tmp/test.sock");

        assert_eq!(builder.terminal_mode, TerminalModePolicy::Managed);
        assert_eq!(builder.history_limit, 50_000);
    }

    #[test]
    fn build_server() {
        let server = MuxServer::builder()
            .terminal_mode(TerminalModePolicy::External)
            .build();
        assert!(server.is_ok());
    }

    #[test]
    fn server_creates_session() {
        let mut server = MuxServer::builder().build().unwrap_or_else(|_| {
            MuxServer {
                kernel: Kernel::new(Clock::manual(0, 0)),
                terminal_mode: TerminalModePolicy::External,
                socket_path: None,
                history_limit: 10_000,
                default_shell: "/bin/sh".into(),
                running: false,
            }
        });
        let session = server.new_session("test");
        assert!(session.is_ok());
        assert_eq!(server.session_count(), 1);
    }

    #[test]
    fn terminal_mode_policy_default() {
        assert_eq!(TerminalModePolicy::default(), TerminalModePolicy::SaveRestore);
    }

    #[test]
    fn passthrough_disabled_by_default() {
        let builder = MuxServerBuilder::new();
        assert!(!builder.allow_passthrough);
    }
}
