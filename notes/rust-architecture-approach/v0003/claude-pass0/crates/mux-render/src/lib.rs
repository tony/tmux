//! # mux-render
//!
//! Composition and rendering pipeline for terminal multiplexer output.
//! Terminal-to-terminal rendering (not GPU). Uses double-buffer diffing
//! to emit minimal escape sequences.
//!
//! ## Module Organization
//! - [`composite`]: CompositeGrid double-buffer with cell source tracking.
//! - [`diff`]: Cell-by-cell diff with wide-char invalidation.
//! - [`sgr`]: SGR attribute encoding and optimization.
//! - [`flood`]: Flood fairness model -- per-pane row quota with stride rotation.

#![forbid(unsafe_code)]

pub mod composite;
pub mod diff;
pub mod sgr;
pub mod flood;

pub use composite::{CompositeGrid, CompositeCell, CellSource};
pub use diff::{CellUpdate, diff_grids};
pub use sgr::SgrEncoder;
pub use flood::FloodScheduler;
