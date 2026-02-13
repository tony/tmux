//! # mux-grid
//!
//! Terminal grid with chunked storage, Arc-based COW lines, and dirty tracking.
//!
//! ## Module Organization
//! - [`line`]: Arc-COW line with cells and dirty flag.
//! - [`chunk`]: Chunk-based storage (64 lines per chunk).
//! - [`region`]: Scrollback region management and active screen.
//! - [`grid`]: The ChunkedGrid entry point.

#![forbid(unsafe_code)]

pub mod line;
pub mod chunk;
pub mod region;
pub mod grid;

pub use line::Line;
pub use chunk::{Chunk, CHUNK_SIZE};
pub use region::ScrollRegion;
pub use grid::ChunkedGrid;
