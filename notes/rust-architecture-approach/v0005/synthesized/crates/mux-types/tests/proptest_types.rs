//! Property-based tests for mux-types.

use proptest::prelude::*;
use mux_types::colour::Colour;
use mux_types::attrs::{Attrs, CellFlags};
use mux_types::geometry::{Size, Position, Rect};
use mux_types::key::{Key, Modifiers, KeyPress};
use mux_types::style::Style;
use mux_types::id::IdGenerator;
use mux_types::cell::Cell;
use mux_grapheme_arena::GraphemeId;

// --- Colour property tests ---

proptest! {
    #[test]
    fn colour_pack_unpack_roundtrip(idx in 0..=255u8) {
        let c = Colour::Indexed(idx);
        let packed = c.pack();
        let unpacked = Colour::unpack(packed);
        prop_assert_eq!(c, unpacked);
    }

    #[test]
    fn colour_rgb_pack_roundtrip(r in 0..=255u8, g in 0..=255u8, b in 0..=255u8) {
        let c = Colour::Rgb { r, g, b };
        let packed = c.pack();
        let unpacked = Colour::unpack(packed);
        prop_assert_eq!(c, unpacked);
    }

    #[test]
    fn colour_default_is_default(_dummy in 0..1u8) {
        prop_assert!(Colour::Default.is_default());
    }

    #[test]
    fn colour_non_default_not_default(idx in 0..=255u8) {
        prop_assert!(!Colour::Indexed(idx).is_default());
    }
}

// --- Attrs property tests ---

proptest! {
    #[test]
    fn attrs_union_contains_both(a_bits in 0..=0x07FFu16, b_bits in 0..=0x07FFu16) {
        let a = Attrs::from_bits_truncate(a_bits);
        let b = Attrs::from_bits_truncate(b_bits);
        let union = a | b;
        prop_assert!(union.contains(a));
        prop_assert!(union.contains(b));
    }

    #[test]
    fn attrs_intersection_subset(a_bits in 0..=0x07FFu16, b_bits in 0..=0x07FFu16) {
        let a = Attrs::from_bits_truncate(a_bits);
        let b = Attrs::from_bits_truncate(b_bits);
        let inter = a & b;
        prop_assert!(a.contains(inter));
        prop_assert!(b.contains(inter));
    }

    #[test]
    fn cellflags_roundtrip(bits in 0..=0xFFu8) {
        let flags = CellFlags::from_bits_truncate(bits);
        let raw = flags.bits();
        let restored = CellFlags::from_bits_truncate(raw);
        prop_assert_eq!(flags, restored);
    }
}

// --- Geometry property tests ---

proptest! {
    #[test]
    fn size_area_is_product(cols in 0..=500u16, rows in 0..=500u16) {
        let s = Size::new(cols, rows);
        prop_assert_eq!(s.area(), cols as u32 * rows as u32);
    }

    #[test]
    fn size_is_zero_when_zero(cols in 0..=100u16, rows in 0..=100u16) {
        let s = Size::new(cols, rows);
        prop_assert_eq!(s.is_zero(), cols == 0 || rows == 0);
    }

    #[test]
    fn position_origin_is_zero(_dummy in 0..1u8) {
        prop_assert_eq!(Position::ORIGIN.x, 0);
        prop_assert_eq!(Position::ORIGIN.y, 0);
    }

    #[test]
    fn rect_intersect_commutative(
        x1 in 0..50u16, y1 in 0..50u16, w1 in 1..50u16, h1 in 1..50u16,
        x2 in 0..50u16, y2 in 0..50u16, w2 in 1..50u16, h2 in 1..50u16,
    ) {
        let r1 = Rect { origin: Position { x: x1, y: y1 }, size: Size::new(w1, h1) };
        let r2 = Rect { origin: Position { x: x2, y: y2 }, size: Size::new(w2, h2) };
        prop_assert_eq!(r1.intersect(r2), r2.intersect(r1));
    }

    #[test]
    fn rect_contains_own_origin(x in 0..100u16, y in 0..100u16, w in 1..100u16, h in 1..100u16) {
        let origin = Position { x, y };
        let r = Rect { origin, size: Size::new(w, h) };
        let test_pos = Position { x, y };
        prop_assert!(r.contains(test_pos));
    }

    #[test]
    fn rect_does_not_contain_outside(x in 0..50u16, y in 0..50u16, w in 1..50u16, h in 1..50u16) {
        let r = Rect { origin: Position { x, y }, size: Size::new(w, h) };
        let far = Position { x: x.saturating_add(w).saturating_add(1), y: y.saturating_add(h).saturating_add(1) };
        prop_assert!(!r.contains(far));
    }
}

// --- Key property tests ---

proptest! {
    #[test]
    fn key_char_roundtrip(c in proptest::char::any()) {
        let k = Key::Char(c);
        if let Key::Char(stored) = k {
            prop_assert_eq!(stored, c);
        } else {
            prop_assert!(false, "expected Key::Char");
        }
    }

    #[test]
    fn modifiers_union_idempotent(bits in 0..=0xFFu8) {
        let m = Modifiers::from_bits_truncate(bits);
        prop_assert_eq!(m | m, m);
    }

    #[test]
    fn keypress_with_modifiers(c in proptest::char::range('a', 'z'), ctrl in proptest::bool::ANY) {
        let mods = if ctrl { Modifiers::CTRL } else { Modifiers::empty() };
        let kp = KeyPress { key: Key::Char(c), modifiers: mods };
        prop_assert_eq!(kp.modifiers.contains(Modifiers::CTRL), ctrl);
    }
}

// --- Cell property tests ---

proptest! {
    #[test]
    fn cell_clear_always_cleared(gid in 0..1000u32, width in 0..=3u8) {
        let mut c = Cell::with_grapheme(GraphemeId::from_raw(gid), width);
        c.clear();
        prop_assert!(c.is_cleared());
        prop_assert_eq!(c.grapheme, GraphemeId::DEFAULT);
        prop_assert_eq!(c.width, 1);
    }

    #[test]
    fn cell_visually_equal_reflexive(gid in 0..100u32) {
        let c = Cell::with_grapheme(GraphemeId::from_raw(gid), 1);
        prop_assert!(c.visually_equal(&c));
    }
}

// --- Style property tests ---

proptest! {
    #[test]
    fn style_needs_reset_when_attrs_removed(
        a_bits in 1..=0x07FFu16,
        b_bits in 0..=0x07FFu16,
    ) {
        let prev = Style { attrs: Attrs::from_bits_truncate(a_bits | b_bits), ..Style::new() };
        let next = Style { attrs: Attrs::from_bits_truncate(b_bits & !a_bits), ..Style::new() };
        // If prev had bits that next does not, needs_reset should be true
        let removed = prev.attrs & !next.attrs;
        if !removed.is_empty_attrs() {
            prop_assert!(next.needs_reset(&prev));
        }
    }
}

// --- IdGenerator property tests ---

proptest! {
    #[test]
    fn id_generator_monotonic(count in 1..100usize) {
        let mut generate = IdGenerator::new();
        let mut prev = 0u64;
        for _ in 0..count {
            let id = generate.next_session();
            prop_assert!(id.0 > prev);
            prev = id.0;
        }
    }

    #[test]
    fn id_generator_starts_at_one(_dummy in 0..1u8) {
        let mut generate = IdGenerator::new();
        prop_assert_eq!(generate.next_session().0, 1);
    }
}
