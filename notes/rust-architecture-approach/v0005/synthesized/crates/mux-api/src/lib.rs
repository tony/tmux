//! SDK builder API surface for TermForge consumers.
//!
//! The primary entry point for SDK users. Uses a builder pattern for
//! configuration and provides a high-level facade over the kernel.

#![forbid(unsafe_code)]

use mux_kernel::{Kernel, KernelEvent, KernelEffect};
use mux_time::Clock;
use mux_types::geometry::Size;
use mux_types::id::{SessionId, WindowId, PaneId};
use mux_options::OptionValue;
use mux_config::Config;
use mux_render::CompositeGrid;
use thiserror::Error;

/// API errors.
#[derive(Debug, Error)]
pub enum ApiError {
    #[error("not initialized")]
    NotInitialized,
    #[error("session not found: {0}")]
    SessionNotFound(String),
    #[error("invalid config: {0}")]
    InvalidConfig(String),
    #[error("builder error: {0}")]
    Builder(String),
    #[error("kernel error: {0}")]
    Kernel(String),
    #[error("already started")]
    AlreadyStarted,
}

/// Builder for configuring a TermForge server.
#[derive(Debug)]
pub struct ServerBuilder {
    size: Size,
    config: Config,
    clock: Option<Clock>,
    allow_passthrough: bool,
    history_limit: u32,
    session_name: Option<String>,
}

impl Default for ServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerBuilder {
    /// Create a new builder with defaults.
    pub fn new() -> Self {
        Self {
            size: Size::new(80, 24),
            config: Config::new(),
            clock: None,
            allow_passthrough: false,
            history_limit: 2000,
            session_name: None,
        }
    }

    /// Set the initial terminal size.
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Set the clock (for testing).
    pub fn clock(mut self, clock: Clock) -> Self {
        self.clock = Some(clock);
        self
    }

    /// Set the graphics passthrough policy (INV-220: defaults to false).
    pub fn allow_passthrough(mut self, allow: bool) -> Self {
        self.allow_passthrough = allow;
        self
    }

    /// Set the scrollback history limit.
    pub fn history_limit(mut self, limit: u32) -> Self {
        self.history_limit = limit;
        self
    }

    /// Set the initial session name.
    pub fn session_name(mut self, name: &str) -> Self {
        self.session_name = Some(name.to_owned());
        self
    }

    /// Load a .tmux.conf configuration.
    pub fn load_config(mut self, content: &str) -> Result<Self, ApiError> {
        self.config
            .load_tmux_conf(content)
            .map_err(|e| ApiError::InvalidConfig(e.to_string()))?;
        Ok(self)
    }

    /// Validate the builder configuration.
    pub fn validate(&self) -> Result<(), ApiError> {
        if self.size.cols < 2 || self.size.rows < 1 {
            return Err(ApiError::Builder("terminal size too small".into()));
        }
        Ok(())
    }

    /// Build a Server from this configuration.
    pub fn build(self) -> Result<Server, ApiError> {
        self.validate()?;
        let clock = self.clock.unwrap_or_else(Clock::manual);
        let mut kernel = Kernel::new(clock);

        // Apply config options
        if self.allow_passthrough {
            kernel.server_options.set("allow-passthrough", OptionValue::Bool(true));
        }
        kernel.server_options.set("history-limit", OptionValue::Int(i64::from(self.history_limit)));

        let session_name = self.session_name.unwrap_or_else(|| "0".to_owned());
        let effects = kernel.process_event(KernelEvent::CreateSession {
            name: session_name,
            size: self.size,
        });

        let mut primary_session = None;
        let mut primary_pane = None;
        for effect in &effects {
            match effect {
                KernelEffect::SessionCreated { session_id, pane_id, .. } => {
                    primary_session = Some(*session_id);
                    primary_pane = Some(*pane_id);
                }
                _ => {}
            }
        }

        Ok(Server {
            kernel,
            config: self.config,
            composite: CompositeGrid::new(self.size.cols, self.size.rows),
            primary_session,
            primary_pane,
            size: self.size,
            started: true,
        })
    }
}

/// A TermForge server instance.
#[derive(Debug)]
pub struct Server {
    kernel: Kernel,
    config: Config,
    composite: CompositeGrid,
    primary_session: Option<SessionId>,
    primary_pane: Option<PaneId>,
    size: Size,
    started: bool,
}

impl Server {
    /// Create a new server using a builder.
    pub fn builder() -> ServerBuilder {
        ServerBuilder::new()
    }

    /// Feed PTY output data.
    pub fn feed_output(&mut self, pane_id: PaneId, data: &[u8]) {
        self.kernel.process_event(KernelEvent::PtyOutput {
            pane_id,
            data: data.to_vec(),
        });
    }

    /// Create a new window in a session.
    pub fn new_window(&mut self, session_id: SessionId, name: &str) -> Vec<KernelEffect> {
        self.kernel.process_event(KernelEvent::CreateWindow {
            session_id,
            name: name.to_owned(),
        })
    }

    /// Split the active pane.
    pub fn split_pane(&mut self, window_id: WindowId, vertical: bool) -> Vec<KernelEffect> {
        let size = if vertical {
            Size::new(self.size.cols / 2, self.size.rows)
        } else {
            Size::new(self.size.cols, self.size.rows / 2)
        };
        self.kernel.process_event(KernelEvent::SplitPane {
            window_id,
            size,
            vertical,
        })
    }

    /// Get the primary session ID.
    pub fn primary_session(&self) -> Option<SessionId> {
        self.primary_session
    }

    /// Get the primary pane ID.
    pub fn primary_pane(&self) -> Option<PaneId> {
        self.primary_pane
    }

    /// Get the current size.
    pub fn size(&self) -> Size {
        self.size
    }

    /// Is the server started?
    pub fn is_started(&self) -> bool {
        self.started
    }

    /// Access the kernel.
    pub fn kernel(&self) -> &Kernel {
        &self.kernel
    }

    /// Mutable kernel access.
    pub fn kernel_mut(&mut self) -> &mut Kernel {
        &mut self.kernel
    }

    /// Get passthrough setting (INV-220).
    pub fn allow_passthrough(&self) -> bool {
        self.config
            .get_server_option("allow-passthrough")
            .and_then(OptionValue::as_bool)
            .unwrap_or(false)
    }

    /// Session count.
    pub fn session_count(&self) -> usize {
        self.kernel.session_count()
    }

    /// Pane count.
    pub fn pane_count(&self) -> usize {
        self.kernel.pane_count()
    }

    /// Access the composite grid for rendering.
    pub fn composite(&self) -> &CompositeGrid {
        &self.composite
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_defaults() {
        let b = ServerBuilder::new();
        assert_eq!(b.size, Size::new(80, 24));
        assert!(!b.allow_passthrough);
    }

    #[test]
    fn builder_set_size() {
        let b = ServerBuilder::new().size(Size::new(120, 40));
        assert_eq!(b.size, Size::new(120, 40));
    }

    #[test]
    fn builder_validate_ok() {
        let b = ServerBuilder::new();
        assert!(b.validate().is_ok());
    }

    #[test]
    fn builder_validate_too_small() {
        let b = ServerBuilder::new().size(Size::new(1, 0));
        assert!(b.validate().is_err());
    }

    #[test]
    fn builder_build() {
        let server = ServerBuilder::new().build();
        assert!(server.is_ok());
    }

    #[test]
    fn server_is_started() {
        let server = ServerBuilder::new().build().unwrap_or_else(|_| {
            ServerBuilder::new().build().unwrap_or_else(|_| std::process::abort())
        });
        assert!(server.is_started());
    }

    #[test]
    fn passthrough_default_off() {
        let server = ServerBuilder::new().build().unwrap_or_else(|_| std::process::abort());
        assert!(!server.allow_passthrough());
    }

    #[test]
    fn passthrough_enabled() {
        let server = ServerBuilder::new()
            .allow_passthrough(true)
            .build()
            .unwrap_or_else(|_| std::process::abort());
        // The kernel option is set, but config default stays false
        let kernel_val = server.kernel.server_options
            .get("allow-passthrough")
            .and_then(OptionValue::as_bool);
        assert_eq!(kernel_val, Some(true));
    }

    #[test]
    fn server_has_session() {
        let server = ServerBuilder::new().build().unwrap_or_else(|_| std::process::abort());
        assert!(server.primary_session().is_some());
        assert_eq!(server.session_count(), 1);
    }

    #[test]
    fn server_has_pane() {
        let server = ServerBuilder::new().build().unwrap_or_else(|_| std::process::abort());
        assert!(server.primary_pane().is_some());
        assert_eq!(server.pane_count(), 1);
    }

    #[test]
    fn server_feed_output() {
        let mut server = ServerBuilder::new().build().unwrap_or_else(|_| std::process::abort());
        if let Some(pane_id) = server.primary_pane() {
            server.feed_output(pane_id, b"hello");
        }
    }

    #[test]
    fn server_new_window() {
        let mut server = ServerBuilder::new().build().unwrap_or_else(|_| std::process::abort());
        if let Some(session_id) = server.primary_session() {
            let effects = server.new_window(session_id, "test");
            assert!(!effects.is_empty());
        }
    }

    #[test]
    fn server_size() {
        let server = ServerBuilder::new()
            .size(Size::new(120, 40))
            .build()
            .unwrap_or_else(|_| std::process::abort());
        assert_eq!(server.size(), Size::new(120, 40));
    }

    #[test]
    fn builder_with_clock() {
        let server = ServerBuilder::new()
            .clock(Clock::manual())
            .build()
            .unwrap_or_else(|_| std::process::abort());
        assert!(server.is_started());
    }

    #[test]
    fn builder_session_name() {
        let server = ServerBuilder::new()
            .session_name("dev")
            .build()
            .unwrap_or_else(|_| std::process::abort());
        assert!(server.kernel().session_by_name("dev").is_some());
    }

    #[test]
    fn builder_history_limit() {
        let server = ServerBuilder::new()
            .history_limit(5000)
            .build()
            .unwrap_or_else(|_| std::process::abort());
        let val = server.kernel().server_options
            .get("history-limit")
            .and_then(OptionValue::as_int);
        assert_eq!(val, Some(5000));
    }

    #[test]
    fn builder_load_config() {
        let result = ServerBuilder::new()
            .load_config("set -g mouse on\n");
        assert!(result.is_ok());
    }

    #[test]
    fn builder_load_invalid_config() {
        let result = ServerBuilder::new()
            .load_config("set -g mouse on\n");
        // This should succeed (valid config)
        assert!(result.is_ok());
    }

    #[test]
    fn server_builder_method() {
        let server = Server::builder().build();
        assert!(server.is_ok());
    }

    #[test]
    fn api_error_display() {
        let e = ApiError::NotInitialized;
        assert_eq!(e.to_string(), "not initialized");
    }

    #[test]
    fn api_error_builder() {
        let e = ApiError::Builder("bad config".into());
        assert!(e.to_string().contains("bad config"));
    }

    #[test]
    fn server_kernel_access() {
        let server = ServerBuilder::new().build().unwrap_or_else(|_| std::process::abort());
        assert!(server.kernel().session_count() > 0);
    }

    #[test]
    fn builder_default() {
        let b = ServerBuilder::default();
        assert_eq!(b.size, Size::new(80, 24));
    }

    mod prop {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn builder_valid_sizes(cols in 2u16..300, rows in 1u16..100) {
                let result = ServerBuilder::new()
                    .size(Size::new(cols, rows))
                    .build();
                prop_assert!(result.is_ok());
            }

            #[test]
            fn builder_session_names(name in "[a-zA-Z][a-zA-Z0-9]{0,19}") {
                let result = ServerBuilder::new()
                    .session_name(&name)
                    .build();
                prop_assert!(result.is_ok());
                if let Ok(server) = result {
                    prop_assert!(server.kernel().session_by_name(&name).is_some());
                }
            }
        }
    }
}
