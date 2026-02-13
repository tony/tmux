//! Core types for TermForge: Cell, Colour, Attrs, entity IDs, geometry, errors.
//!
//! This crate defines all shared types used across the workspace. It sits at L1
//! in the dependency DAG, depending only on L0 crates (mux-grapheme-arena, mux-time).

#![forbid(unsafe_code)]

pub mod attrs;
pub mod cell;
pub mod colour;
pub mod error;
pub mod geometry;
pub mod id;
pub mod key;
pub mod style;

pub use attrs::{Attrs, CellFlags};
pub use cell::Cell;
pub use colour::Colour;
pub use error::MuxError;
pub use geometry::{Position, Rect, Size};
pub use id::{ClientId, IdGenerator, PaneId, SessionId, WindowId};
pub use key::{Key, Modifiers};
pub use style::Style;
