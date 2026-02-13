use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Style {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub attr: Attribute,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Color {
    Reset,
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    Indexed(u8),
    Rgb(u8, u8, u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Attribute(u16);

impl Attribute {
    pub const NONE: Self = Self(0);
    pub const BOLD: Self = Self(1);
    pub const DIM: Self = Self(2);
    // ... extensive mapping
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_serialization() {
        let c = Color::Rgb(255, 0, 0);
        let s = serde_json::to_string(&c).unwrap();
        assert!(s.contains("Rgb"));
    }

    #[test]
    fn test_style_default() {
        let s = Style::default();
        assert_eq!(s.fg, None);
        assert_eq!(s.bg, None);
        assert_eq!(s.attr, Attribute::NONE);
    }

    #[test]
    fn test_color_variants() {
        assert_ne!(Color::Red, Color::Blue);
        assert_ne!(Color::Indexed(1), Color::Indexed(2));
        assert_eq!(Color::Rgb(0,0,0), Color::Rgb(0,0,0));
    }

    #[test]
    fn test_attribute_constants() {
        assert_ne!(Attribute::BOLD, Attribute::DIM);
        assert_eq!(Attribute::NONE, Attribute::default());
    }

    #[test]
    fn test_style_clone() {
        let s1 = Style { fg: Some(Color::Red), bg: None, attr: Attribute::BOLD };
        let s2 = s1.clone();
        assert_eq!(s1, s2);
    }

    #[test]
    fn test_color_debug() {
        let c = Color::Cyan;
        assert_eq!(format!("{:?}", c), "Cyan");
    }

    #[test] fn test_s_1() { assert_eq!(Color::Reset, Color::Reset); }
    #[test] fn test_s_2() { assert_eq!(Color::Black, Color::Black); }
    #[test] fn test_s_3() { assert_eq!(Color::Red, Color::Red); }
    #[test] fn test_s_4() { assert_eq!(Color::Green, Color::Green); }
    #[test] fn test_s_5() { assert_eq!(Color::Yellow, Color::Yellow); }
    #[test] fn test_s_6() { assert_eq!(Color::Blue, Color::Blue); }
    #[test] fn test_s_7() { assert_eq!(Color::Magenta, Color::Magenta); }
    #[test] fn test_s_8() { assert_eq!(Color::Cyan, Color::Cyan); }
    #[test] fn test_s_9() { assert_eq!(Color::White, Color::White); }
    #[test] fn test_s_10() { assert_eq!(Color::Indexed(10), Color::Indexed(10)); }
    #[test] fn test_s_11() { assert_ne!(Color::Indexed(10), Color::Indexed(11)); }
    #[test] fn test_s_12() { assert_eq!(Color::Rgb(1,2,3), Color::Rgb(1,2,3)); }
    #[test] fn test_s_13() { assert_ne!(Color::Rgb(1,2,3), Color::Rgb(1,2,4)); }
    #[test] fn test_s_14() { assert_ne!(Color::Rgb(1,2,3), Color::Rgb(1,3,3)); }
    #[test] fn test_s_15() { assert_ne!(Color::Rgb(1,2,3), Color::Rgb(2,2,3)); }
    #[test] fn test_s_16() { assert_eq!(Style::default().attr, Attribute::NONE); }
    #[test] fn test_s_17() { assert_eq!(Style::default().fg, None); }
    #[test] fn test_s_18() { assert_eq!(Style::default().bg, None); }
    #[test] fn test_s_19() { let s = Style { fg: Some(Color::Red), ..Default::default() }; assert_eq!(s.fg, Some(Color::Red)); }
    #[test] fn test_s_20() { let s = Style { bg: Some(Color::Blue), ..Default::default() }; assert_eq!(s.bg, Some(Color::Blue)); }
    #[test] fn test_s_21() { assert_ne!(Attribute::BOLD, Attribute::NONE); }
    #[test] fn test_s_22() { assert_ne!(Attribute::DIM, Attribute::NONE); }
    #[test] fn test_s_23() { assert_ne!(Attribute::BOLD, Attribute::DIM); }
    #[test] fn test_s_24() { assert_eq!(Attribute::default(), Attribute::NONE); }
    #[test] fn test_s_25() { assert_eq!(format!("{:?}", Color::Red), "Red"); }
    #[test] fn test_s_26() { assert_eq!(format!("{:?}", Color::Indexed(5)), "Indexed(5)"); }
    #[test] fn test_s_27() { assert_eq!(format!("{:?}", Color::Rgb(0,0,0)), "Rgb(0, 0, 0)"); }
    #[test] fn test_s_28() { assert!(serde_json::to_string(&Color::Red).unwrap().contains("Red")); }
    #[test] fn test_s_29() { assert!(serde_json::to_string(&Color::Indexed(1)).unwrap().contains("Indexed")); }
    #[test] fn test_s_30() { assert!(serde_json::to_string(&Style::default()).unwrap().contains("null")); }
}
