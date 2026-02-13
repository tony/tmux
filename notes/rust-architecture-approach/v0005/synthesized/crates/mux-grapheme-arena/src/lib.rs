//! Arena allocator for extended grapheme clusters.
//!
//! Terminal cells frequently share the same grapheme (e.g. space, ASCII letters).
//! This arena deduplicates storage by interning grapheme strings and returning
//! a compact `GraphemeId` handle. The arena owns all grapheme data; callers
//! hold lightweight IDs.
//!
//! # Design rationale
//!
//! Using an arena avoids per-cell `String` allocations for the common case
//! where thousands of cells contain the same character. The `GraphemeId` is a
//! `u32`, keeping `Cell` structs small and cache-friendly.

#![forbid(unsafe_code)]

use std::collections::HashMap;

/// Compact handle referencing an interned grapheme cluster.
///
/// The id `0` is reserved for the empty/default grapheme (single space).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct GraphemeId(u32);

impl GraphemeId {
    /// The default grapheme id representing a single space character.
    pub const DEFAULT: Self = Self(0);

    /// Returns the raw numeric value.
    pub const fn raw(self) -> u32 {
        self.0
    }

    /// Construct from a raw numeric value. Used only for deserialization.
    pub const fn from_raw(v: u32) -> Self {
        Self(v)
    }
}

/// Arena that interns grapheme clusters for deduplication.
///
/// Thread-safety: this type is **not** `Sync`. Each pane owns its own arena.
#[derive(Debug, Clone)]
pub struct GraphemeArena {
    /// Map from grapheme string to its id for O(1) dedup lookup.
    index: HashMap<String, GraphemeId>,
    /// Indexed storage: `strings[id.raw()]` gives back the grapheme.
    strings: Vec<String>,
}

impl Default for GraphemeArena {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphemeArena {
    /// Create a new arena pre-seeded with the default space grapheme at index 0.
    pub fn new() -> Self {
        let default_grapheme = " ".to_owned();
        let mut index = HashMap::new();
        index.insert(default_grapheme.clone(), GraphemeId::DEFAULT);
        Self {
            index,
            strings: vec![default_grapheme],
        }
    }

    /// Intern a grapheme cluster, returning its stable id.
    ///
    /// If the grapheme was already interned, the existing id is returned.
    /// Otherwise a new slot is allocated.
    pub fn intern(&mut self, grapheme: &str) -> GraphemeId {
        if let Some(&id) = self.index.get(grapheme) {
            return id;
        }
        let id = GraphemeId(self.strings.len() as u32);
        self.strings.push(grapheme.to_owned());
        self.index.insert(grapheme.to_owned(), id);
        id
    }

    /// Resolve an id back to its grapheme string.
    ///
    /// Returns `" "` (space) if the id is out of range, ensuring we never
    /// produce invalid output even with stale ids.
    pub fn resolve(&self, id: GraphemeId) -> &str {
        self.strings
            .get(id.0 as usize)
            .map(String::as_str)
            .unwrap_or(" ")
    }

    /// Number of unique graphemes currently interned.
    pub fn len(&self) -> usize {
        self.strings.len()
    }

    /// Whether the arena has only the default grapheme.
    pub fn is_empty(&self) -> bool {
        self.strings.len() <= 1
    }

    /// Iterate over all interned graphemes with their ids.
    pub fn iter(&self) -> impl Iterator<Item = (GraphemeId, &str)> {
        self.strings
            .iter()
            .enumerate()
            .map(|(i, s)| (GraphemeId(i as u32), s.as_str()))
    }

    /// Returns true if the given grapheme is already interned.
    pub fn contains(&self, grapheme: &str) -> bool {
        self.index.contains_key(grapheme)
    }

    /// Intern each grapheme cluster from a unicode string, returning all ids.
    pub fn intern_graphemes(&mut self, text: &str) -> Vec<GraphemeId> {
        use unicode_segmentation::UnicodeSegmentation;
        text.graphemes(true).map(|g| self.intern(g)).collect()
    }

    /// Clear all entries except the default space grapheme.
    pub fn clear(&mut self) {
        self.strings.truncate(1);
        self.index.clear();
        self.index
            .insert(" ".to_owned(), GraphemeId::DEFAULT);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_grapheme_is_space() {
        let arena = GraphemeArena::new();
        assert_eq!(arena.resolve(GraphemeId::DEFAULT), " ");
    }

    #[test]
    fn intern_deduplicates() {
        let mut arena = GraphemeArena::new();
        let id1 = arena.intern("A");
        let id2 = arena.intern("A");
        assert_eq!(id1, id2);
        assert_eq!(arena.len(), 2); // default + A
    }

    #[test]
    fn intern_distinct_graphemes() {
        let mut arena = GraphemeArena::new();
        let a = arena.intern("a");
        let b = arena.intern("b");
        assert_ne!(a, b);
        assert_eq!(arena.resolve(a), "a");
        assert_eq!(arena.resolve(b), "b");
    }

    #[test]
    fn resolve_invalid_id_returns_space() {
        let arena = GraphemeArena::new();
        assert_eq!(arena.resolve(GraphemeId::from_raw(999)), " ");
    }

    #[test]
    fn intern_unicode_grapheme_clusters() {
        let mut arena = GraphemeArena::new();
        // Family emoji is a single grapheme cluster
        let emoji = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}";
        let id = arena.intern(emoji);
        assert_eq!(arena.resolve(id), emoji);
    }

    #[test]
    fn intern_cjk_character() {
        let mut arena = GraphemeArena::new();
        let id = arena.intern("\u{4e16}"); // CJK unified ideograph
        assert_eq!(arena.resolve(id), "\u{4e16}");
    }

    #[test]
    fn contains_check() {
        let mut arena = GraphemeArena::new();
        assert!(!arena.contains("X"));
        arena.intern("X");
        assert!(arena.contains("X"));
    }

    #[test]
    fn intern_graphemes_splits_correctly() {
        let mut arena = GraphemeArena::new();
        let ids = arena.intern_graphemes("abc");
        assert_eq!(ids.len(), 3);
        assert_eq!(arena.resolve(ids[0]), "a");
        assert_eq!(arena.resolve(ids[1]), "b");
        assert_eq!(arena.resolve(ids[2]), "c");
    }

    #[test]
    fn clear_resets_to_default() {
        let mut arena = GraphemeArena::new();
        arena.intern("x");
        arena.intern("y");
        assert!(arena.len() >= 3);
        arena.clear();
        assert_eq!(arena.len(), 1);
        assert_eq!(arena.resolve(GraphemeId::DEFAULT), " ");
    }

    #[test]
    fn is_empty_after_creation() {
        let arena = GraphemeArena::new();
        assert!(arena.is_empty());
    }

    #[test]
    fn not_empty_after_intern() {
        let mut arena = GraphemeArena::new();
        arena.intern("Z");
        assert!(!arena.is_empty());
    }

    #[test]
    fn iter_includes_default() {
        let arena = GraphemeArena::new();
        let entries: Vec<_> = arena.iter().collect();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].1, " ");
    }

    #[test]
    fn grapheme_id_raw_roundtrip() {
        let id = GraphemeId::from_raw(42);
        assert_eq!(id.raw(), 42);
    }

    #[test]
    fn large_intern_count() {
        let mut arena = GraphemeArena::new();
        for i in 0u32..500 {
            let s = format!("g{i}");
            let id = arena.intern(&s);
            assert_eq!(arena.resolve(id), s);
        }
        assert_eq!(arena.len(), 501); // default + 500
    }

    #[test]
    fn intern_empty_string() {
        let mut arena = GraphemeArena::new();
        let id = arena.intern("");
        assert_eq!(arena.resolve(id), "");
    }

    // proptest: interning any ASCII string is idempotent
    mod prop {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn intern_idempotent(s in "[a-z]{1,8}") {
                let mut arena = GraphemeArena::new();
                let id1 = arena.intern(&s);
                let id2 = arena.intern(&s);
                prop_assert_eq!(id1, id2);
                prop_assert_eq!(arena.resolve(id1), s.as_str());
            }

            #[test]
            fn intern_graphemes_roundtrip(s in "[a-zA-Z0-9]{1,20}") {
                let mut arena = GraphemeArena::new();
                let ids = arena.intern_graphemes(&s);
                let reconstructed: String = ids.iter().map(|id| arena.resolve(*id)).collect();
                prop_assert_eq!(reconstructed, s);
            }

            #[test]
            fn grapheme_id_raw_roundtrip_prop(v in 0u32..100_000) {
                let id = GraphemeId::from_raw(v);
                prop_assert_eq!(id.raw(), v);
            }
        }
    }
}
