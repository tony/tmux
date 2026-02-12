//! # mux-types
//!
//! Core types for the TermForge terminal multiplexer.
//! This is an L0 crate with no internal dependencies.
//!
//! ## Key Types
//! - [`Cell`]: A single terminal cell holding a grapheme cluster and style.
//! - [`PackedCell`]: A 64-bit packed representation of a Cell (INV-007, S94).
//! - [`Line`]: A row of cells backed by `Arc<Vec<Cell>>` for COW (S96).
//! - [`Style`]: Visual style with Color enum, Attrs bitfield (INV-009).
//! - [`IdentityManifest`]: Project identity metadata (S01).
//! - [`TermletError`]: 16-variant error enum for termlet operations (S98).

#![forbid(unsafe_code)]

pub mod cell;
pub mod packed_cell;
pub mod line;
pub mod style;
pub mod identity;
pub mod error;

// GPT-style pub use re-exports for ergonomic imports
pub use cell::Cell;
pub use packed_cell::PackedCell;
pub use line::Line;
pub use style::{Style, Color, Attrs};
pub use identity::{IdentityManifest, BuildProfile};
pub use error::TermletError;
