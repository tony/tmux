//! # mux-render
//!
//! Composition and rendering pipeline for terminal multiplexer output.
//!
//! ## Module Organization (GPT multi-file pattern)
//! - [`composite`]: CompositeGrid double-buffer with cell source tracking.
//! - [`diff`]: Cell-by-cell diff with wide-char invalidation.
//! - [`output`]: RenderOutput escape sequence builder with SGR/CUP optimization.
//! - [`flood`]: Flood fairness model -- per-pane row quota with stride rotation.
//! - [`blit`]: Pane grid -> composite grid blitting.

#![forbid(unsafe_code)]

pub mod composite;
pub mod diff;
pub mod output;
pub mod flood;
pub mod blit;

pub use composite::{CompositeGrid, CompositeCell, CellSource};
pub use diff::{CellUpdate, diff};
pub use output::RenderOutput;
pub use flood::FloodScheduler;
pub use blit::{PaneGeometry, blit_pane, render_borders};
