/// Parser byte class (S95, INV-026).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ByteClass {
    Printable = 0,
    Control = 1,
    Escape = 2,
    CsiEntry = 3,
    DcsEntry = 4,
    OscEntry = 5,
    Utf8Lead = 6,
}

const fn classify_const(byte: u8) -> ByteClass {
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

const fn build_table() -> [ByteClass; 256] {
    let mut table = [ByteClass::Utf8Lead; 256];
    let mut i = 0;
    while i < 256 {
        table[i] = classify_const(i as u8);
        i += 1;
    }
    table
}

/// Static classification table for parser hot path.
pub static CLASS_TABLE: [ByteClass; 256] = build_table();

#[must_use]
#[inline]
pub fn classify(byte: u8) -> ByteClass {
    CLASS_TABLE[byte as usize]
}

#[must_use]
#[inline]
pub fn classify_oracle(byte: u8) -> ByteClass {
    classify_const(byte)
}

#[must_use]
#[inline]
pub fn classify_match(byte: u8) -> ByteClass {
    classify_oracle(byte)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_matches_oracle_for_all_bytes() {
        for b in 0u8..=255 {
            assert_eq!(classify(b), classify_oracle(b), "byte=0x{b:02x}");
        }
    }

    #[test]
    fn dcs_entry_mapped_at_0x90() {
        assert_eq!(classify(0x90), ByteClass::DcsEntry);
    }

    #[test]
    fn printable_ascii_mapped_correctly() {
        for b in 0x20u8..=0x7E {
            assert_eq!(classify(b), ByteClass::Printable);
        }
    }

    #[test]
    fn control_and_escape_ranges_mapped() {
        assert_eq!(classify(0x00), ByteClass::Control);
        assert_eq!(classify(0x1B), ByteClass::Escape);
        assert_eq!(classify(0x7F), ByteClass::Control);
    }

    #[test]
    fn variant_count_is_seven() {
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
}
