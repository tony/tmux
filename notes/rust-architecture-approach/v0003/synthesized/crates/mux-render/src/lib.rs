//! # mux-render
//!
//! Double-buffer diff renderer for terminal output.
//!
//! ## Module Organization
//! - [`composite`]: CompositeGrid for frame composition.
//! - [`diff`]: Cell-by-cell diff algorithm.
//! - [`sgr`]: SGR (Select Graphic Rendition) encoder.
//! - [`flood`]: Per-pane flood fairness engine.

#![forbid(unsafe_code)]

pub mod composite;
pub mod diff;
pub mod sgr;
pub mod flood;
