//! Comprehensive unit tests for mux-types.

use mux_types::colour::Colour;
use mux_types::attrs::{Attrs, CellFlags};
use mux_types::geometry::{Size, Position, Rect};
use mux_types::key::{Key, Modifiers, KeyPress};
use mux_types::style::Style;
use mux_types::cell::Cell;
use mux_types::id::{SessionId, WindowId, PaneId, ClientId, IdGenerator};
use mux_types::error::MuxError;
use mux_grapheme_arena::GraphemeId;

// ---- Colour comprehensive tests ----

#[test]
fn colour_default_pack() {
    let packed = Colour::Default.pack();
    let unpacked = Colour::unpack(packed);
    assert_eq!(unpacked, Colour::Default);
}

#[test]
fn colour_all_indexed() {
    for i in 0..=255u8 {
        let c = Colour::Indexed(i);
        assert_eq!(c, Colour::unpack(c.pack()));
    }
}

#[test]
fn colour_rgb_extremes() {
    let black = Colour::Rgb { r: 0, g: 0, b: 0 };
    let white = Colour::Rgb { r: 255, g: 255, b: 255 };
    assert_ne!(black, white);
    assert_eq!(black, Colour::unpack(black.pack()));
    assert_eq!(white, Colour::unpack(white.pack()));
}

#[test]
fn colour_debug() {
    let c = Colour::Rgb { r: 128, g: 64, b: 32 };
    let dbg = format!("{c:?}");
    assert!(dbg.contains("Rgb"));
}

#[test]
fn colour_equality() {
    assert_eq!(Colour::Indexed(5), Colour::Indexed(5));
    assert_ne!(Colour::Indexed(5), Colour::Indexed(6));
}

#[test]
fn colour_hash_in_set() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(Colour::Default);
    set.insert(Colour::Default);
    assert_eq!(set.len(), 1);
    set.insert(Colour::Indexed(1));
    assert_eq!(set.len(), 2);
}

// ---- CellFlags comprehensive tests ----

#[test]
fn cellflags_inv119_values() {
    // INV-119: CellFlags bit values must match tmux's GRID_FLAG_*
    assert_eq!(CellFlags::PADDING.bits(), 0x04);
    assert_eq!(CellFlags::EXTENDED.bits(), 0x08);
    assert_eq!(CellFlags::SELECTED.bits(), 0x10);
    assert_eq!(CellFlags::CLEARED.bits(), 0x40);
    assert_eq!(CellFlags::TAB.bits(), 0x80);
}

#[test]
fn cellflags_empty() {
    let f = CellFlags::empty();
    assert!(f.is_empty());
    assert!(!f.contains(CellFlags::PADDING));
}

#[test]
fn cellflags_all_flags() {
    let all = CellFlags::PADDING
        | CellFlags::EXTENDED
        | CellFlags::SELECTED
        | CellFlags::CLEARED
        | CellFlags::TAB;
    assert!(all.contains(CellFlags::PADDING));
    assert!(all.contains(CellFlags::TAB));
}

#[test]
fn cellflags_remove() {
    let mut f = CellFlags::PADDING | CellFlags::TAB;
    f.remove(CellFlags::PADDING);
    assert!(!f.contains(CellFlags::PADDING));
    assert!(f.contains(CellFlags::TAB));
}

#[test]
fn cellflags_toggle() {
    let mut f = CellFlags::SELECTED;
    f.toggle(CellFlags::SELECTED);
    assert!(!f.contains(CellFlags::SELECTED));
    f.toggle(CellFlags::SELECTED);
    assert!(f.contains(CellFlags::SELECTED));
}

// ---- Attrs comprehensive tests ----

#[test]
fn attrs_bold() {
    let a = Attrs::BOLD;
    assert!(a.contains(Attrs::BOLD));
    assert!(!a.contains(Attrs::ITALIC));
}

#[test]
fn attrs_all_variants() {
    let all = Attrs::BOLD
        | Attrs::DIM
        | Attrs::ITALIC
        | Attrs::UNDERSCORE
        | Attrs::BLINK
        | Attrs::REVERSE
        | Attrs::HIDDEN
        | Attrs::STRIKETHROUGH;
    assert!(all.contains(Attrs::BOLD));
    assert!(all.contains(Attrs::STRIKETHROUGH));
}

#[test]
fn attrs_empty_check() {
    assert!(Attrs::empty().is_empty_attrs());
    assert!(!Attrs::BOLD.is_empty_attrs());
}

#[test]
fn attrs_sgr_params_bold_italic() {
    let a = Attrs::BOLD | Attrs::ITALIC | Attrs::UNDERSCORE;
    let params = a.sgr_params();
    assert!(params.contains(&1)); // BOLD
    assert!(params.contains(&3)); // ITALIC
    assert!(params.contains(&4)); // UNDERSCORE
}

// ---- Geometry comprehensive tests ----

#[test]
fn size_new() {
    let s = Size::new(80, 24);
    assert_eq!(s.cols, 80);
    assert_eq!(s.rows, 24);
}

#[test]
fn size_zero() {
    let s = Size::new(0, 0);
    assert!(s.is_zero());
    assert_eq!(s.area(), 0);
}

#[test]
fn position_new() {
    let p = Position { x: 5, y: 10 };
    assert_eq!(p.x, 5);
    assert_eq!(p.y, 10);
}

#[test]
fn rect_new() {
    let r = Rect {
        origin: Position::ORIGIN,
        size: Size::new(80, 24),
    };
    assert!(r.contains(Position { x: 0, y: 0 }));
    assert!(r.contains(Position { x: 79, y: 23 }));
    assert!(!r.contains(Position { x: 80, y: 0 }));
    assert!(!r.contains(Position { x: 0, y: 24 }));
}

#[test]
fn rect_intersect_overlapping() {
    let r1 = Rect { origin: Position { x: 0, y: 0 }, size: Size::new(10, 10) };
    let r2 = Rect { origin: Position { x: 5, y: 5 }, size: Size::new(10, 10) };
    let inter = r1.intersect(r2);
    assert!(inter.is_some());
    let inter = inter.unwrap_or_default();
    assert_eq!(inter.origin, Position { x: 5, y: 5 });
    assert_eq!(inter.size, Size::new(5, 5));
}

#[test]
fn rect_intersect_non_overlapping() {
    let r1 = Rect { origin: Position { x: 0, y: 0 }, size: Size::new(5, 5) };
    let r2 = Rect { origin: Position { x: 10, y: 10 }, size: Size::new(5, 5) };
    assert!(r1.intersect(r2).is_none());
}

#[test]
fn rect_intersect_adjacent() {
    let r1 = Rect { origin: Position { x: 0, y: 0 }, size: Size::new(5, 5) };
    let r2 = Rect { origin: Position { x: 5, y: 0 }, size: Size::new(5, 5) };
    assert!(r1.intersect(r2).is_none());
}

#[test]
fn rect_contains_all_corners() {
    let r = Rect { origin: Position { x: 10, y: 10 }, size: Size::new(20, 10) };
    assert!(r.contains(Position { x: 10, y: 10 })); // top-left
    assert!(r.contains(Position { x: 29, y: 10 })); // top-right
    assert!(r.contains(Position { x: 10, y: 19 })); // bottom-left
    assert!(r.contains(Position { x: 29, y: 19 })); // bottom-right
}

// ---- Key comprehensive tests ----

#[test]
fn key_char() {
    let k = Key::Char('a');
    assert!(matches!(k, Key::Char('a')));
}

#[test]
fn key_function_keys() {
    let k = Key::F(1);
    assert!(matches!(k, Key::F(1)));
}

#[test]
fn key_special_keys() {
    let keys = [Key::Enter, Key::Tab, Key::Backspace, Key::Escape, Key::Up, Key::Down, Key::Left, Key::Right];
    assert_eq!(keys.len(), 8);
}

#[test]
fn modifiers_ctrl_shift() {
    let m = Modifiers::CTRL | Modifiers::SHIFT;
    assert!(m.contains(Modifiers::CTRL));
    assert!(m.contains(Modifiers::SHIFT));
    assert!(!m.contains(Modifiers::ALT));
}

#[test]
fn keypress_creation() {
    let kp = KeyPress {
        key: Key::Char('c'),
        modifiers: Modifiers::CTRL,
    };
    assert_eq!(kp.key, Key::Char('c'));
    assert!(kp.modifiers.contains(Modifiers::CTRL));
}

#[test]
fn keypress_no_modifiers() {
    let kp = KeyPress {
        key: Key::Enter,
        modifiers: Modifiers::empty(),
    };
    assert!(kp.modifiers.is_empty());
}

// ---- Cell comprehensive tests ----

#[test]
fn cell_all_flags_combo() {
    let mut c = Cell::default();
    c.flags = CellFlags::PADDING | CellFlags::TAB | CellFlags::SELECTED;
    assert!(c.is_padding());
    assert!(c.is_tab());
    assert!(c.is_selected());
}

#[test]
fn cell_attrs_combo() {
    let mut c = Cell::default();
    c.attrs = Attrs::BOLD | Attrs::ITALIC | Attrs::UNDERSCORE;
    c.fg = Colour::Indexed(1);
    c.bg = Colour::Indexed(2);
    c.us = Colour::Indexed(3);
    assert!(c.has_attrs());
}

#[test]
fn cell_link_hyperlink() {
    let mut c = Cell::default();
    c.link = 42;
    assert_eq!(c.link, 42);
}

#[test]
fn cell_visually_equal_ignores_flags() {
    let mut c1 = Cell::default();
    let c2 = Cell::default();
    c1.flags = CellFlags::SELECTED;
    // visually_equal doesn't check flags
    assert!(c1.visually_equal(&c2));
}

// ---- Error comprehensive tests ----

#[test]
fn error_all_variants_display() {
    let errors: Vec<MuxError> = vec![
        MuxError::Grid("g".into()),
        MuxError::Parse("p".into()),
        MuxError::Kernel("k".into()),
        MuxError::Config("c".into()),
        MuxError::Protocol("r".into()),
        MuxError::Api("a".into()),
        MuxError::NotFound("n".into()),
        MuxError::InvalidArgument("i".into()),
        MuxError::CapacityExceeded("e".into()),
    ];
    for e in &errors {
        assert!(!e.to_string().is_empty());
    }
}

#[test]
fn error_debug_format() {
    let e = MuxError::Grid("test".into());
    let dbg = format!("{e:?}");
    assert!(dbg.contains("Grid"));
}

// ---- Style comprehensive tests ----

#[test]
fn style_with_all_colours() {
    let s = Style::new()
        .with_fg(Colour::Indexed(1))
        .with_bg(Colour::Indexed(2));
    assert_eq!(s.fg, Colour::Indexed(1));
    assert_eq!(s.bg, Colour::Indexed(2));
}

#[test]
fn style_needs_reset_bold_to_none() {
    let prev = Style::bold();
    let next = Style::new();
    assert!(next.needs_reset(&prev));
}

#[test]
fn style_no_reset_none_to_bold() {
    let prev = Style::new();
    let next = Style::bold();
    assert!(!next.needs_reset(&prev));
}

// ---- ID comprehensive tests ----

#[test]
fn id_types_in_hashmap() {
    use std::collections::HashMap;
    let mut map = HashMap::new();
    map.insert(SessionId(1), "session1");
    map.insert(SessionId(2), "session2");
    assert_eq!(map.get(&SessionId(1)), Some(&"session1"));
}

#[test]
fn id_sorting() {
    let mut ids = vec![PaneId(5), PaneId(1), PaneId(3)];
    ids.sort();
    assert_eq!(ids, vec![PaneId(1), PaneId(3), PaneId(5)]);
}

#[test]
fn id_generator_1000_ids() {
    let mut gen_id = IdGenerator::new();
    for i in 1..=1000u64 {
        let id = gen_id.next_pane();
        assert_eq!(id.0, i);
    }
}
