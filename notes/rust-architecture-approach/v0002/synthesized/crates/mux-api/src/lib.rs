//! # mux-api
//!
//! SDK entry-point crate for TermForge.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

use mux_kernel::{Kernel, KernelEffect, KernelEvent};
use mux_time::Clock;
use mux_types::{ClientId, PaneId, Size};

/// Who owns the raw mode transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalModePolicy {
    Managed,
    External,
    SaveRestore,
}

impl Default for TerminalModePolicy {
    fn default() -> Self { Self::SaveRestore }
}

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

    #[must_use]
    pub const fn terminal_mode(mut self, policy: TerminalModePolicy) -> Self {
        self.terminal_mode = policy;
        self
    }

    #[must_use]
    pub fn socket_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.socket_path = Some(path.into());
        self
    }

    #[must_use]
    pub const fn history_limit(mut self, limit: u32) -> Self {
        self.history_limit = limit;
        self
    }

    #[must_use]
    pub fn default_shell(mut self, shell: impl Into<String>) -> Self {
        self.default_shell = shell.into();
        self
    }

    #[must_use]
    pub const fn escape_time(mut self, ms: u32) -> Self {
        self.escape_time = ms;
        self
    }

    #[must_use]
    pub const fn focus_events(mut self, enabled: bool) -> Self {
        self.focus_events = enabled;
        self
    }

    #[must_use]
    pub const fn allow_passthrough(mut self, enabled: bool) -> Self {
        self.allow_passthrough = enabled;
        self
    }

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
    fn default() -> Self { Self::new() }
}

pub struct MuxServer {
    kernel: Kernel,
    terminal_mode: TerminalModePolicy,
    socket_path: Option<PathBuf>,
    history_limit: u32,
    default_shell: String,
    running: bool,
}

impl MuxServer {
    #[must_use]
    pub fn builder() -> MuxServerBuilder { MuxServerBuilder::new() }

    #[must_use]
    pub const fn terminal_mode(&self) -> TerminalModePolicy { self.terminal_mode }

    #[must_use]
    pub fn socket_path(&self) -> Option<&Path> { self.socket_path.as_deref() }

    #[must_use]
    pub const fn history_limit(&self) -> u32 { self.history_limit }

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
        Ok(SessionHandle { name: name.to_owned() })
    }

    #[must_use]
    pub fn session_count(&self) -> usize { self.kernel.session_count() }

    #[must_use]
    pub fn kernel(&self) -> &Kernel { &self.kernel }

    pub fn kernel_mut(&mut self) -> &mut Kernel { &mut self.kernel }
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

#[derive(Debug, Clone)]
pub struct SessionHandle { pub name: String }

#[derive(Debug, Clone)]
pub struct WindowHandle { pub session: String, pub index: u32 }

#[derive(Debug, Clone)]
pub struct PaneHandle { pub id: PaneId }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_defaults() {
        let builder = MuxServerBuilder::new();
        assert_eq!(builder.terminal_mode, TerminalModePolicy::SaveRestore);
        assert_eq!(builder.history_limit, 10_000);
        assert!(!builder.allow_passthrough);
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
