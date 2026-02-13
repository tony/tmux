//! # mux-grapheme-arena
//!
//! Interned grapheme cluster arena for compact cell storage.
//! Extended grapheme clusters (emoji sequences, combining characters) are stored
//! once and referenced by a 32-bit handle.
//!
//! L0 leaf crate -- no internal dependencies.

#![forbid(unsafe_code)]

use std::collections::HashMap;

/// A handle to an interned grapheme cluster.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GraphemeHandle(u32);

impl GraphemeHandle {
    /// The null/empty handle representing a single space.
    pub const EMPTY: Self = Self(0);

    /// Raw index for storage.
    #[must_use]
    pub const fn index(self) -> u32 {
        self.0
    }
}

/// Arena for interning grapheme clusters.
#[derive(Debug)]
pub struct GraphemeArena {
    /// Stored grapheme clusters indexed by handle.
    strings: Vec<String>,
    /// Reverse map for deduplication.
    lookup: HashMap<String, GraphemeHandle>,
}

impl GraphemeArena {
    /// Create a new arena with the empty grapheme pre-allocated.
    #[must_use]
    pub fn new() -> Self {
        let mut arena = Self {
            strings: Vec::new(),
            lookup: HashMap::new(),
        };
        // Slot 0 is always the empty/space grapheme.
        arena.strings.push(" ".into());
        arena.lookup.insert(" ".into(), GraphemeHandle::EMPTY);
        arena
    }

    /// Intern a grapheme cluster, returning a handle.
    /// If the grapheme is already interned, returns the existing handle.
    pub fn intern(&mut self, grapheme: &str) -> GraphemeHandle {
        if let Some(&handle) = self.lookup.get(grapheme) {
            return handle;
        }
        let handle = GraphemeHandle(self.strings.len() as u32);
        self.strings.push(grapheme.to_owned());
        self.lookup.insert(grapheme.to_owned(), handle);
        handle
    }

    /// Resolve a handle to its grapheme cluster string.
    #[must_use]
    pub fn resolve(&self, handle: GraphemeHandle) -> Option<&str> {
        self.strings.get(handle.0 as usize).map(String::as_str)
    }

    /// Number of interned graphemes (including the empty slot).
    #[must_use]
    pub fn len(&self) -> usize {
        self.strings.len()
    }

    /// Whether the arena is empty (only the empty slot).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.strings.len() <= 1
    }
}

impl Default for GraphemeArena {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_handle_resolves_to_space() {
        let arena = GraphemeArena::new();
        assert_eq!(arena.resolve(GraphemeHandle::EMPTY), Some(" "));
    }

    #[test]
    fn intern_ascii() {
        let mut arena = GraphemeArena::new();
        let h = arena.intern("A");
        assert_eq!(arena.resolve(h), Some("A"));
    }

    #[test]
    fn deduplication() {
        let mut arena = GraphemeArena::new();
        let h1 = arena.intern("hello");
        let h2 = arena.intern("hello");
        assert_eq!(h1, h2);
        // Only 2 entries: empty + "hello"
        assert_eq!(arena.len(), 2);
    }

    #[test]
    fn intern_emoji() {
        let mut arena = GraphemeArena::new();
        let h = arena.intern("\u{1F600}");
        assert_eq!(arena.resolve(h), Some("\u{1F600}"));
    }

    #[test]
    fn intern_combining() {
        let mut arena = GraphemeArena::new();
        let h = arena.intern("e\u{0301}"); // e + combining acute accent
        assert_eq!(arena.resolve(h), Some("e\u{0301}"));
    }

    #[test]
    fn new_arena_is_empty() {
        let arena = GraphemeArena::new();
        assert!(arena.is_empty());
    }

    #[test]
    fn arena_not_empty_after_intern() {
        let mut arena = GraphemeArena::new();
        arena.intern("X");
        assert!(!arena.is_empty());
    }

    #[test]
    fn invalid_handle_returns_none() {
        let arena = GraphemeArena::new();
        assert!(arena.resolve(GraphemeHandle(9999)).is_none());
    }

    #[test]
    fn multiple_distinct_strings() {
        let mut arena = GraphemeArena::new();
        let h1 = arena.intern("alpha");
        let h2 = arena.intern("beta");
        let h3 = arena.intern("gamma");
        assert_ne!(h1, h2);
        assert_ne!(h2, h3);
        assert_eq!(arena.resolve(h1), Some("alpha"));
        assert_eq!(arena.resolve(h2), Some("beta"));
        assert_eq!(arena.resolve(h3), Some("gamma"));
    }

    #[test]
    fn handle_equality() {
        let a = GraphemeHandle(1);
        let b = GraphemeHandle(1);
        assert_eq!(a, b);
    }

    #[test]
    fn handle_empty_constant() {
        assert_eq!(GraphemeHandle::EMPTY.0, 0);
    }

    #[test]
    fn default_arena() {
        let arena = GraphemeArena::default();
        assert!(arena.is_empty());
    }
}
