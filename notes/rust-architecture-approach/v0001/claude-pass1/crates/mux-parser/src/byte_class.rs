//! ByteClass: classification of input bytes for the parser state machine.
//!
//! INV-026: Exactly 7 variants. DcsEntry for 0x90.
//! S95: CLASS_TABLE[256] static LUT is canonical.
//! RULE-S06-02: classify_match() is the reference oracle.

/// ByteClass: classification of input bytes for the parser state machine.
///
/// INV-026: Exactly 7 variants. DcsEntry for 0x90.
/// RULE-S06-04: #[repr(u8)] for compact storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ByteClass {
    /// Printable characters: 0x20..=0x7E
    Printable = 0,
    /// C0 control codes: 0x00..=0x1F (except ESC 0x1B), 0x7F
    Control = 1,
    /// ESC: 0x1B
    Escape = 2,
    /// CSI entry: 0x9B
    CsiEntry = 3,
    /// DCS entry: 0x90
    DcsEntry = 4,
    /// OSC entry: 0x9D
    OscEntry = 5,
    /// UTF-8 lead bytes and other high bytes
    Utf8Lead = 6,
}

/// GPT-style: const fn classifier for building the table at compile time.
const fn classify_const(b: u8) -> ByteClass {
    match b {
        0x1B => ByteClass::Escape,
        0x9B => ByteClass::CsiEntry,
        0x9D => ByteClass::OscEntry,
        0x90 => ByteClass::DcsEntry,
        0x20..=0x7E => ByteClass::Printable,
        0x00..=0x1A | 0x1C..=0x1F | 0x7F => ByteClass::Control,
        _ => ByteClass::Utf8Lead,
    }
}

/// Build the lookup table at compile time (GPT improvement).
const fn build_table() -> [ByteClass; 256] {
    let mut table = [ByteClass::Control; 256];
    let mut i = 0;
    while i < 256 {
        table[i] = classify_const(i as u8);
        i += 1;
    }
    table
}

/// S95: CLASS_TABLE[256] static lookup table.
///
/// Maps every byte value to its ByteClass. This is the canonical hot path;
/// `classify_match()` is retained only as a test oracle.
pub static CLASS_TABLE: [ByteClass; 256] = build_table();

/// Inline lookup via the table (canonical path).
#[inline]
#[must_use]
pub fn classify(byte: u8) -> ByteClass {
    CLASS_TABLE[byte as usize]
}

/// Match-based byte classifier retained as oracle for testing.
///
/// RULE-S06-02: classify_match() is the reference implementation.
/// RULE-S06-08: CLASS_TABLE must match this oracle for all 256 bytes.
#[must_use]
pub fn classify_match(byte: u8) -> ByteClass {
    match byte {
        0x00..=0x1A | 0x1C..=0x1F | 0x7F => ByteClass::Control,
        0x1B => ByteClass::Escape,
        0x20..=0x7E => ByteClass::Printable,
        0x90 => ByteClass::DcsEntry,
        0x9B => ByteClass::CsiEntry,
        0x9D => ByteClass::OscEntry,
        _ => ByteClass::Utf8Lead,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RULE-S06-08: CLASS_TABLE matches oracle for all 256 bytes.
    #[test]
    fn test_class_table_matches_oracle_all_256() {
        for byte in 0u16..=255 {
            let b = byte as u8;
            assert_eq!(
                CLASS_TABLE[b as usize],
                classify_match(b),
                "mismatch at byte 0x{b:02X}"
            );
        }
    }

    /// INV-026: ByteClass has 7 variants.
    #[test]
    fn test_byte_class_has_7_variants() {
        let variants = [
            ByteClass::Printable,
            ByteClass::Control,
            ByteClass::Escape,
            ByteClass::CsiEntry,
            ByteClass::DcsEntry,
            ByteClass::OscEntry,
            ByteClass::Utf8Lead,
        ];
        assert_eq!(variants.len(), 7);
    }

    /// RULE-S06-05: DcsEntry at 0x90.
    #[test]
    fn test_dcs_entry_at_0x90() {
        assert_eq!(CLASS_TABLE[0x90], ByteClass::DcsEntry);
    }

    /// Printable ASCII range.
    #[test]
    fn test_printable_ascii_range() {
        for b in 0x20u8..=0x7E {
            assert_eq!(classify(b), ByteClass::Printable, "byte 0x{b:02X}");
        }
    }

    /// UTF-8 lead byte range.
    #[test]
    fn test_utf8_lead_range() {
        assert_eq!(classify(0xC2), ByteClass::Utf8Lead);
        assert_eq!(classify(0xF4), ByteClass::Utf8Lead);
    }

    /// DEL (0x7F) is Control.
    #[test]
    fn test_del_is_control() {
        assert_eq!(classify(0x7F), ByteClass::Control);
    }

    /// ESC (0x1B) is Escape.
    #[test]
    fn test_esc_is_escape() {
        assert_eq!(classify(0x1B), ByteClass::Escape);
    }

    /// CSI (0x9B) is CsiEntry.
    #[test]
    fn test_csi_entry() {
        assert_eq!(classify(0x9B), ByteClass::CsiEntry);
    }

    /// OSC (0x9D) is OscEntry.
    #[test]
    fn test_osc_entry() {
        assert_eq!(classify(0x9D), ByteClass::OscEntry);
    }
}
