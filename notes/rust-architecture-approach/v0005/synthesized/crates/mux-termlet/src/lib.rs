//! Embeddable terminal widget for SDK consumers.
//!
//! Provides a high-level `Termlet` type that wraps the kernel, grid, and render
//! pipeline into a single API for embedding a terminal pane into an application.

#![forbid(unsafe_code)]

use mux_grapheme_arena::GraphemeArena;
use mux_kernel::{Kernel, KernelEvent, KernelEffect};
use mux_render::{CompositeGrid, encode_diff};
use mux_time::Clock;
use mux_types::geometry::Size;
use mux_types::id::PaneId;
use thiserror::Error;

/// Termlet errors.
#[derive(Debug, Error)]
pub enum TermletError {
    #[error("termlet not initialized")]
    NotInitialized,
    #[error("termlet error: {0}")]
    General(String),
}

/// Lifecycle state of a Termlet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TermletState {
    Created,
    Running,
    Stopped,
}

/// An embeddable terminal widget.
#[derive(Debug)]
pub struct Termlet {
    kernel: Kernel,
    composite: CompositeGrid,
    arena: GraphemeArena,
    size: Size,
    state: TermletState,
    primary_pane: Option<PaneId>,
}

impl Termlet {
    /// Create a new termlet with the given size.
    pub fn new(size: Size) -> Self {
        Self {
            kernel: Kernel::new(Clock::manual()),
            composite: CompositeGrid::new(size.cols, size.rows),
            arena: GraphemeArena::new(),
            size,
            state: TermletState::Created,
            primary_pane: None,
        }
    }

    /// Create with a custom clock.
    pub fn with_clock(size: Size, clock: Clock) -> Self {
        Self {
            kernel: Kernel::new(clock),
            composite: CompositeGrid::new(size.cols, size.rows),
            arena: GraphemeArena::new(),
            size,
            state: TermletState::Created,
            primary_pane: None,
        }
    }

    /// Initialize and create a session with a default pane.
    pub fn start(&mut self, session_name: &str) -> Result<(), TermletError> {
        let effects = self.kernel.process_event(KernelEvent::CreateSession {
            name: session_name.to_owned(),
            size: self.size,
        });
        for effect in &effects {
            if let KernelEffect::SessionCreated { pane_id, .. } = effect {
                self.primary_pane = Some(*pane_id);
            }
        }
        self.state = TermletState::Running;
        Ok(())
    }

    /// Feed data from a PTY into the pane.
    pub fn feed(&mut self, data: &[u8]) -> Result<(), TermletError> {
        if self.state != TermletState::Running {
            return Err(TermletError::NotInitialized);
        }
        if let Some(pane_id) = self.primary_pane {
            self.kernel.process_event(KernelEvent::PtyOutput {
                pane_id,
                data: data.to_vec(),
            });
        }
        Ok(())
    }

    /// Get a rendered snapshot of the screen.
    pub fn render(&mut self) -> Vec<u8> {
        self.composite.swap();
        // Fill next buffer from grid
        if let Some(pane_id) = self.primary_pane {
            if let Some(grid) = self.kernel.pane_grid(pane_id) {
                let snap = grid.snapshot();
                self.composite.next.fill_from_lines(&snap, &self.arena);
            }
        }
        let changes = self.composite.diff();
        encode_diff(&changes, &self.arena)
    }

    /// Current termlet state.
    pub fn state(&self) -> TermletState {
        self.state
    }

    /// Resize the termlet.
    pub fn resize(&mut self, new_size: Size) {
        self.size = new_size;
        self.composite.resize(new_size.cols, new_size.rows);
        if let Some(pane_id) = self.primary_pane {
            if let Some(grid) = self.kernel.pane_grid_mut(pane_id) {
                grid.resize(new_size.cols, new_size.rows);
            }
        }
    }

    /// Stop the termlet.
    pub fn stop(&mut self) {
        self.state = TermletState::Stopped;
    }

    /// Get the primary pane ID.
    pub fn primary_pane(&self) -> Option<PaneId> {
        self.primary_pane
    }

    /// Get the current size.
    pub fn size(&self) -> Size {
        self.size
    }

    /// Access the kernel for advanced operations.
    pub fn kernel(&self) -> &Kernel {
        &self.kernel
    }

    /// Mutable access to the kernel.
    pub fn kernel_mut(&mut self) -> &mut Kernel {
        &mut self.kernel
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn termlet_creation() {
        let t = Termlet::new(Size::new(80, 24));
        assert_eq!(t.state(), TermletState::Created);
        assert_eq!(t.size(), Size::new(80, 24));
    }

    #[test]
    fn termlet_start() {
        let mut t = Termlet::new(Size::new(80, 24));
        assert!(t.start("test").is_ok());
        assert_eq!(t.state(), TermletState::Running);
        assert!(t.primary_pane().is_some());
    }

    #[test]
    fn termlet_feed() {
        let mut t = Termlet::new(Size::new(80, 24));
        t.start("test").unwrap_or(());
        assert!(t.feed(b"hello").is_ok());
    }

    #[test]
    fn termlet_feed_before_start() {
        let mut t = Termlet::new(Size::new(80, 24));
        assert!(t.feed(b"data").is_err());
    }

    #[test]
    fn termlet_render() {
        let mut t = Termlet::new(Size::new(80, 24));
        t.start("test").unwrap_or(());
        t.feed(b"Hello World").unwrap_or(());
        let output = t.render();
        assert!(!output.is_empty());
    }

    #[test]
    fn termlet_resize() {
        let mut t = Termlet::new(Size::new(80, 24));
        t.start("test").unwrap_or(());
        t.resize(Size::new(120, 40));
        assert_eq!(t.size(), Size::new(120, 40));
    }

    #[test]
    fn termlet_stop() {
        let mut t = Termlet::new(Size::new(80, 24));
        t.start("test").unwrap_or(());
        t.stop();
        assert_eq!(t.state(), TermletState::Stopped);
    }

    #[test]
    fn termlet_kernel_access() {
        let mut t = Termlet::new(Size::new(80, 24));
        t.start("test").unwrap_or(());
        assert!(t.kernel().session_count() > 0);
    }

    #[test]
    fn termlet_with_clock() {
        let clock = Clock::manual();
        let t = Termlet::with_clock(Size::new(80, 24), clock);
        assert_eq!(t.state(), TermletState::Created);
    }

    #[test]
    fn termlet_multiple_feeds() {
        let mut t = Termlet::new(Size::new(80, 24));
        t.start("test").unwrap_or(());
        for i in 0..10 {
            let data = format!("line {i}\r\n");
            t.feed(data.as_bytes()).unwrap_or(());
        }
        let output = t.render();
        assert!(!output.is_empty());
    }

    #[test]
    fn termlet_render_empty() {
        let mut t = Termlet::new(Size::new(80, 24));
        t.start("test").unwrap_or(());
        // No data fed
        let output = t.render();
        // May or may not have output depending on diff
        let _ = output;
    }

    #[test]
    fn termlet_lifecycle() {
        let mut t = Termlet::new(Size::new(80, 24));
        assert_eq!(t.state(), TermletState::Created);
        t.start("lifecycle").unwrap_or(());
        assert_eq!(t.state(), TermletState::Running);
        t.feed(b"data").unwrap_or(());
        t.resize(Size::new(100, 30));
        t.stop();
        assert_eq!(t.state(), TermletState::Stopped);
    }

    #[test]
    fn termlet_primary_pane_before_start() {
        let t = Termlet::new(Size::new(80, 24));
        assert!(t.primary_pane().is_none());
    }
}
