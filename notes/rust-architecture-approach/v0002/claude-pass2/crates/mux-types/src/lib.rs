//! # mux-types
//!
//! Core types for the TermForge terminal multiplexer.
//! L1 crate -- depends only on mux-grapheme-arena.
//!
//! ## Module Organization (adopted from GPT's multi-file pattern)
//! - [`cell`]: Terminal cell with grapheme, width, flags, attributes, colours.
//! - [`style`]: Colour, Attrs, underline styles.
//! - [`identity`]: Strongly-typed entity IDs (SessionId, WindowId, PaneId, ClientId).
//! - [`error`]: Core error type and Result alias.
//! - [`input`]: KeyCode, KeyModifiers, KeyEvent, MouseEvent.
//! - [`geometry`]: Size type and related geometry helpers.

#![forbid(unsafe_code)]

pub mod cell;
pub mod style;
pub mod identity;
pub mod error;
pub mod input;
pub mod geometry;

// Re-exports for ergonomic imports
pub use cell::{Cell, CellFlags};
pub use style::{Colour, Attrs};
pub use identity::{SessionId, WindowId, PaneId, ClientId};
pub use error::{TermForgeError, Result};
pub use input::{KeyCode, KeyModifiers, KeyEvent, MouseButton, MouseEventKind, MouseEvent};
pub use geometry::Size;
