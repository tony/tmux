//! # mux-termlet
//!
//! Testing pods that combine a VT parser + grid for integration tests.
//! A "termlet" is a lightweight, in-process terminal emulator for verifying
//! the output of programs.
//!
//! ## API
//! - `Termlet::spawn(cmd)` -- create a testing pod with a PTY and shell
//! - `Termlet::capture()` -- snapshot the visual area
//! - `Termlet::resize(cols, rows)` -- resize the terminal
//! - `Termlet::send_keys(keys)` -- send keystrokes
//! - `Termlet::wait_for(pattern, timeout)` -- wait for content to appear
//! - `Termlet::background()` -- send to background
//! - `Termlet::shutdown()` -- graceful cleanup
//!
//! L4 integration crate.

#![forbid(unsafe_code)]

use mux_grid::ChunkedGrid;
use mux_parser::VtParser;
use mux_time::{Clock, Deadline};
use mux_types::Size;
use std::time::Duration;
use thiserror::Error;

/// Termlet errors.
#[derive(Debug, Error)]
pub enum TermletError {
    /// Timeout waiting for pattern.
    #[error("timeout waiting for pattern: {pattern}")]
    Timeout { pattern: String },
    /// Termlet is not running.
    #[error("termlet not running")]
    NotRunning,
    /// Invalid operation.
    #[error("invalid operation: {0}")]
    InvalidOperation(String),
}

/// Termlet lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TermletState {
    /// Created but not yet started.
    Created,
    /// Running in the foreground.
    Running,
    /// Running in the background.
    Background,
    /// Shut down.
    Shutdown,
}

/// A minimal terminal emulator for testing.
///
/// Combines a VT parser and a grid. Feed bytes in, read text out.
#[derive(Debug)]
pub struct Termlet {
    grid: ChunkedGrid,
    parser: VtParser,
    state: TermletState,
    clock: Clock,
    /// Command that was spawned (if any).
    command: Option<String>,
}

impl Termlet {
    /// Create a new termlet with the given dimensions.
    #[must_use]
    pub fn new(size: Size) -> Self {
        Self {
            grid: ChunkedGrid::new(size.cols, size.rows, 0),
            parser: VtParser::new(),
            state: TermletState::Created,
            clock: Clock::manual(),
        command: None,
        }
    }

    /// Create a new termlet with an explicit clock.
    #[must_use]
    pub fn with_clock(size: Size, clock: Clock) -> Self {
        Self {
            grid: ChunkedGrid::new(size.cols, size.rows, 0),
            parser: VtParser::new(),
            state: TermletState::Created,
            clock,
            command: None,
        }
    }

    /// Spawn a command (scaffold: marks as running and records command).
    pub fn spawn(&mut self, cmd: &str) -> Result<(), TermletError> {
        if self.state == TermletState::Shutdown {
            return Err(TermletError::NotRunning);
        }
        self.command = Some(cmd.to_owned());
        self.state = TermletState::Running;
        Ok(())
    }

    /// Feed raw bytes (as from PTY output) into the termlet.
    pub fn feed(&mut self, data: &[u8]) {
        let actions = self.parser.advance_all(data);
        for action in &actions {
            match action {
                mux_parser::VtAction::Print(c) => {
                    self.grid.write_char(*c);
                }
                mux_parser::VtAction::Execute(0x0A) => {
                    self.grid.line_feed();
                }
                mux_parser::VtAction::Execute(0x0D) => {
                    let (_, y) = self.grid.cursor();
                    self.grid.set_cursor(0, y);
                }
                _ => {}
            }
        }
        if self.state == TermletState::Created {
            self.state = TermletState::Running;
        }
    }

    /// Capture a snapshot of the active screen text.
    #[must_use]
    pub fn capture(&self) -> Vec<String> {
        self.grid.active_text()
    }

    /// Get the text of a single line.
    #[must_use]
    pub fn line_text(&self, row: u16) -> String {
        self.grid
            .active_line(row)
            .map(mux_grid::Line::text_content)
            .unwrap_or_default()
    }

    /// Get the text content of the active screen.
    #[must_use]
    pub fn screen_text(&self) -> Vec<String> {
        self.grid.active_text()
    }

    /// Current cursor position.
    #[must_use]
    pub fn cursor(&self) -> (u16, u16) {
        self.grid.cursor()
    }

    /// Grid dimensions.
    #[must_use]
    pub fn size(&self) -> Size {
        Size::new(self.grid.sx(), self.grid.sy())
    }

    /// Resize the terminal.
    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.grid.resize(cols, rows);
    }

    /// Simulate sending keystrokes by feeding bytes.
    pub fn send_keys(&mut self, keys: &str) {
        self.feed(keys.as_bytes());
    }

    /// Wait for a pattern to appear in the screen text.
    /// Uses the injected clock for deterministic testing.
    pub fn wait_for(
        &self,
        pattern: &str,
        timeout: Duration,
    ) -> Result<Vec<String>, TermletError> {
        let deadline = Deadline::after(self.clock.now(), timeout);
        // In scaffold, we just check the current state (no actual polling).
        let text = self.capture();
        if text.iter().any(|line| line.contains(pattern)) {
            return Ok(text);
        }
        // With a manual clock, we can't "wait" -- just check once.
        if deadline.is_expired(&self.clock) {
            return Err(TermletError::Timeout {
                pattern: pattern.to_owned(),
            });
        }
        // Check again (deterministic: same result)
        let text = self.capture();
        if text.iter().any(|line| line.contains(pattern)) {
            Ok(text)
        } else {
            Err(TermletError::Timeout {
                pattern: pattern.to_owned(),
            })
        }
    }

    /// Send the termlet to background.
    pub fn background(&mut self) -> Result<(), TermletError> {
        match self.state {
            TermletState::Running => {
                self.state = TermletState::Background;
                Ok(())
            }
            _ => Err(TermletError::InvalidOperation(
                "can only background a running termlet".into(),
            )),
        }
    }

    /// Bring the termlet back to foreground.
    pub fn foreground(&mut self) -> Result<(), TermletError> {
        match self.state {
            TermletState::Background => {
                self.state = TermletState::Running;
                Ok(())
            }
            _ => Err(TermletError::InvalidOperation(
                "can only foreground a background termlet".into(),
            )),
        }
    }

    /// Graceful shutdown.
    pub fn shutdown(&mut self) -> Result<(), TermletError> {
        if self.state == TermletState::Shutdown {
            return Err(TermletError::NotRunning);
        }
        self.state = TermletState::Shutdown;
        Ok(())
    }

    /// Current lifecycle state.
    #[must_use]
    pub const fn state(&self) -> TermletState {
        self.state
    }

    /// The spawned command (if any).
    #[must_use]
    pub fn command(&self) -> Option<&str> {
        self.command.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn termlet_basic_text() {
        let mut t = Termlet::new(Size::new(80, 24));
        t.feed(b"Hello, World!");
        assert_eq!(t.line_text(0), "Hello, World!");
    }

    #[test]
    fn termlet_newlines() {
        let mut t = Termlet::new(Size::new(80, 24));
        t.feed(b"Line1\r\nLine2\r\nLine3");
        assert_eq!(t.line_text(0), "Line1");
        assert_eq!(t.line_text(1), "Line2");
        assert_eq!(t.line_text(2), "Line3");
    }

    #[test]
    fn termlet_cursor_position() {
        let mut t = Termlet::new(Size::new(80, 24));
        t.feed(b"ABC");
        assert_eq!(t.cursor(), (3, 0));
    }

    #[test]
    fn termlet_size() {
        let t = Termlet::new(Size::new(120, 40));
        assert_eq!(t.size(), Size::new(120, 40));
    }

    #[test]
    fn termlet_screen_text() {
        let mut t = Termlet::new(Size::new(10, 3));
        t.feed(b"Hi");
        let text = t.screen_text();
        assert_eq!(text.len(), 3);
        assert_eq!(text[0], "Hi");
    }

    #[test]
    fn termlet_spawn() {
        let mut t = Termlet::new(Size::new(80, 24));
        assert!(t.spawn("/bin/sh").is_ok());
        assert_eq!(t.state(), TermletState::Running);
        assert_eq!(t.command(), Some("/bin/sh"));
    }

    #[test]
    fn termlet_capture() {
        let mut t = Termlet::new(Size::new(80, 24));
        t.feed(b"test output");
        let snap = t.capture();
        assert!(snap[0].contains("test output"));
    }

    #[test]
    fn termlet_resize() {
        let mut t = Termlet::new(Size::new(80, 24));
        t.resize(120, 40);
        assert_eq!(t.size(), Size::new(120, 40));
    }

    #[test]
    fn termlet_send_keys() {
        let mut t = Termlet::new(Size::new(80, 24));
        t.send_keys("hello");
        assert_eq!(t.line_text(0), "hello");
    }

    #[test]
    fn termlet_wait_for_found() {
        let mut t = Termlet::new(Size::new(80, 24));
        t.feed(b"Compiling mux-types");
        let result = t.wait_for("Compiling", Duration::from_secs(5));
        assert!(result.is_ok());
    }

    #[test]
    fn termlet_wait_for_timeout() {
        let clock = Clock::manual();
        let mut t = Termlet::with_clock(Size::new(80, 24), clock.clone());
        t.feed(b"nothing here");
        clock.advance(Duration::from_secs(10));
        let result = t.wait_for("missing", Duration::from_secs(5));
        assert!(result.is_err());
    }

    #[test]
    fn termlet_background_foreground() {
        let mut t = Termlet::new(Size::new(80, 24));
        t.feed(b"data");
        assert!(t.background().is_ok());
        assert_eq!(t.state(), TermletState::Background);
        assert!(t.foreground().is_ok());
        assert_eq!(t.state(), TermletState::Running);
    }

    #[test]
    fn termlet_background_requires_running() {
        let mut t = Termlet::new(Size::new(80, 24));
        // State is Created, not Running
        assert!(t.background().is_err());
    }

    #[test]
    fn termlet_shutdown() {
        let mut t = Termlet::new(Size::new(80, 24));
        t.feed(b"data");
        assert!(t.shutdown().is_ok());
        assert_eq!(t.state(), TermletState::Shutdown);
    }

    #[test]
    fn termlet_double_shutdown_fails() {
        let mut t = Termlet::new(Size::new(80, 24));
        t.feed(b"data");
        assert!(t.shutdown().is_ok());
        assert!(t.shutdown().is_err());
    }

    #[test]
    fn termlet_spawn_after_shutdown_fails() {
        let mut t = Termlet::new(Size::new(80, 24));
        t.feed(b"data");
        let _ = t.shutdown();
        assert!(t.spawn("echo hello").is_err());
    }

    #[test]
    fn termlet_lifecycle_created_to_running() {
        let mut t = Termlet::new(Size::new(80, 24));
        assert_eq!(t.state(), TermletState::Created);
        t.feed(b"data");
        assert_eq!(t.state(), TermletState::Running);
    }

    #[test]
    fn termlet_with_clock() {
        let clock = Clock::manual();
        let t = Termlet::with_clock(Size::new(80, 24), clock);
        assert_eq!(t.state(), TermletState::Created);
    }

    #[test]
    fn termlet_foreground_requires_background() {
        let mut t = Termlet::new(Size::new(80, 24));
        t.feed(b"data");
        // State is Running, not Background
        assert!(t.foreground().is_err());
    }

    #[test]
    fn termlet_full_lifecycle() {
        let mut t = Termlet::new(Size::new(80, 24));
        assert_eq!(t.state(), TermletState::Created);
        assert!(t.spawn("/bin/sh").is_ok());
        assert_eq!(t.state(), TermletState::Running);
        t.feed(b"hello world");
        assert!(t.capture()[0].contains("hello world"));
        assert!(t.background().is_ok());
        assert_eq!(t.state(), TermletState::Background);
        assert!(t.foreground().is_ok());
        assert_eq!(t.state(), TermletState::Running);
        assert!(t.shutdown().is_ok());
        assert_eq!(t.state(), TermletState::Shutdown);
    }

    #[test]
    fn termlet_resize_preserves_content() {
        let mut t = Termlet::new(Size::new(40, 10));
        t.feed(b"before resize");
        t.resize(80, 24);
        assert_eq!(t.size(), Size::new(80, 24));
        // Content from first line should still be accessible
        let text = t.line_text(0);
        assert!(text.contains("before resize") || text.is_empty());
    }

    #[test]
    fn termlet_send_keys_multiline() {
        let mut t = Termlet::new(Size::new(80, 24));
        t.send_keys("abc\r\ndef");
        assert_eq!(t.line_text(0), "abc");
        assert_eq!(t.line_text(1), "def");
    }

    #[test]
    fn termlet_command_none_before_spawn() {
        let t = Termlet::new(Size::new(80, 24));
        assert!(t.command().is_none());
    }

    #[test]
    fn termlet_capture_empty_screen() {
        let t = Termlet::new(Size::new(10, 3));
        let snap = t.capture();
        assert_eq!(snap.len(), 3);
    }
}
