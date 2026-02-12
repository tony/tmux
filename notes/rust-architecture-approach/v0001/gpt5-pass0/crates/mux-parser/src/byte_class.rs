/// Parser byte classes (S95, INV-026).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ByteClass {
    Printable = 0,
    Control = 1,
    Escape = 2,
    CsiEntry = 3,
    OscString = 4,
    DcsEntry = 5,
    Utf8Lead = 6,
}

const fn classify_const(b: u8) -> ByteClass {
    match b {
        0x1b => ByteClass::Escape,
        0x9b => ByteClass::CsiEntry,
        0x9d => ByteClass::OscString,
        0x90 => ByteClass::DcsEntry,
        0x20..=0x7e => ByteClass::Printable,
        0xc2..=0xf4 => ByteClass::Utf8Lead,
        _ if b < 0x20 || b == 0x7f => ByteClass::Control,
        _ => ByteClass::Printable,
    }
}

const fn build_table() -> [ByteClass; 256] {
    let mut table = [ByteClass::Control; 256];
    let mut i = 0;
    while i < 256 {
        table[i] = classify_const(i as u8);
        i += 1;
    }
    table
}

pub static CLASS_TABLE: [ByteClass; 256] = build_table();

#[inline]
pub fn classify(byte: u8) -> ByteClass {
    CLASS_TABLE[byte as usize]
}

#[inline]
pub fn classify_oracle(byte: u8) -> ByteClass {
    classify_const(byte)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dcs_entry_is_0x90() {
        assert_eq!(classify(0x90), ByteClass::DcsEntry);
    }

    #[test]
    fn table_matches_oracle_for_all_bytes() {
        for b in 0u8..=255 {
            assert_eq!(classify(b), classify_oracle(b), "byte=0x{b:02x}");
        }
    }

    #[test]
    fn printable_ascii_range() {
        for b in 0x20u8..=0x7e {
            assert_eq!(classify(b), ByteClass::Printable);
        }
    }

    #[test]
    fn utf8_lead_range() {
        assert_eq!(classify(0xc2), ByteClass::Utf8Lead);
        assert_eq!(classify(0xf4), ByteClass::Utf8Lead);
    }
}
