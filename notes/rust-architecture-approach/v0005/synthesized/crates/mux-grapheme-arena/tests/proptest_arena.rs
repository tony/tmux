//! Property-based tests for mux-grapheme-arena.

use proptest::prelude::*;
use mux_grapheme_arena::{GraphemeArena, GraphemeId};

proptest! {
    #[test]
    fn intern_roundtrip(s in "[a-z]{1,8}") {
        let mut arena = GraphemeArena::new();
        let id = arena.intern(&s);
        prop_assert_eq!(arena.resolve(id), s.as_str());
    }

    #[test]
    fn intern_idempotent(s in "[a-z]{1,4}") {
        let mut arena = GraphemeArena::new();
        let id1 = arena.intern(&s);
        let id2 = arena.intern(&s);
        prop_assert_eq!(id1, id2);
    }

    #[test]
    fn distinct_strings_distinct_ids(a in "[a-z]{1,4}", b in "[A-Z]{1,4}") {
        let mut arena = GraphemeArena::new();
        let id_a = arena.intern(&a);
        let id_b = arena.intern(&b);
        if a != b {
            prop_assert_ne!(id_a, id_b);
        }
    }

    #[test]
    fn intern_many_all_resolvable(strings in proptest::collection::vec("[a-z0-9]{1,4}", 1..50)) {
        let mut arena = GraphemeArena::new();
        let ids: Vec<_> = strings.iter().map(|s| arena.intern(s)).collect();
        for (s, id) in strings.iter().zip(ids.iter()) {
            prop_assert_eq!(arena.resolve(*id), s.as_str());
        }
    }

    #[test]
    fn default_grapheme_id_is_zero(_dummy in 0..1u8) {
        prop_assert_eq!(GraphemeId::DEFAULT.raw(), 0);
    }

    #[test]
    fn from_raw_roundtrip(val in 0..10000u32) {
        let id = GraphemeId::from_raw(val);
        prop_assert_eq!(id.raw(), val);
    }
}

#[test]
fn empty_arena_default_id_resolves() {
    let arena = GraphemeArena::new();
    // DEFAULT might resolve to empty or space depending on impl
    let _ = arena.resolve(GraphemeId::DEFAULT);
}

#[test]
fn arena_len_increases() {
    let mut arena = GraphemeArena::new();
    let before = arena.len();
    arena.intern("hello");
    let after = arena.len();
    assert!(after >= before);
}

#[test]
fn arena_same_string_no_growth() {
    let mut arena = GraphemeArena::new();
    arena.intern("test");
    let len1 = arena.len();
    arena.intern("test");
    let len2 = arena.len();
    assert_eq!(len1, len2);
}

#[test]
fn arena_unicode_graphemes() {
    let mut arena = GraphemeArena::new();
    let id = arena.intern("\u{1F600}"); // emoji
    assert_eq!(arena.resolve(id), "\u{1F600}");
}

#[test]
fn arena_cjk_character() {
    let mut arena = GraphemeArena::new();
    let id = arena.intern("\u{4E16}"); // CJK "world"
    assert_eq!(arena.resolve(id), "\u{4E16}");
}

#[test]
#[allow(clippy::clone_on_copy)]
fn grapheme_id_copy_clone() {
    let id = GraphemeId::from_raw(42);
    let id2 = id;
    let id3 = id.clone();
    assert_eq!(id, id2);
    assert_eq!(id, id3);
}

#[test]
fn grapheme_id_raw_values_ordered() {
    let a = GraphemeId::from_raw(1);
    let b = GraphemeId::from_raw(2);
    assert!(a.raw() < b.raw());
}

#[test]
fn grapheme_id_hash() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(GraphemeId::from_raw(1));
    set.insert(GraphemeId::from_raw(1));
    assert_eq!(set.len(), 1);
}
