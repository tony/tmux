//! PackedCell: 64-bit compact cell representation.
//!
//! ## Layout (S94)
//! ```text
//! Bits [63:43] = scalar (21 bits) - Unicode scalar value
//! Bits [42:28] = style  (15 bits) - Packed style index
//! Bits [27:16] = flags  (12 bits) - Cell flags
//! Bits [15:14] = width  (2 bits)  - Display width (0, 1, 2)
//! Bits [13:0]  = ext    (14 bits) - GraphemeArena extension index
//! ```
//!
//! ## Invariants
//! - INV-007: PackedCell is exactly 64 bits.
//! - S94: Layout is scalar:21 + style:15 + flags:12 + width:2 + ext:14.
//! - INV-029: Width field is 0, 1, or 2.
//! - INV-030: ext index 0x0000 means "no extension".

use core::fmt;

/// Bit widths for each field (GPT-style compile-time const assertions).
const SCALAR_BITS: u32 = 21;
const STYLE_BITS: u32 = 15;
const FLAGS_BITS: u32 = 12;
const WIDTH_BITS: u32 = 2;
const EXT_BITS: u32 = 14;

/// Compile-time assertion: bit widths sum to exactly 64.
const _ASSERT_BITS_SUM_64: () = {
    assert!(
        SCALAR_BITS + STYLE_BITS + FLAGS_BITS + WIDTH_BITS + EXT_BITS == 64,
        "PackedCell bit fields must sum to 64"
    );
};

/// INV-007: PackedCell is exactly 64 bits.
/// S94: scalar:21 + style:15 + flags:12 + width:2 + ext:14 = 64.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PackedCell(u64);

/// Validation errors for [`PackedCell::try_new`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PackedCellError {
    ScalarOverflow { value: u32, max: u32 },
    StyleIndexOverflow { value: u16, max: u16 },
    FlagsOverflow { value: u16, max: u16 },
    InvalidWidth(u8),
    ExtOverflow { value: u16, max: u16 },
}

impl fmt::Display for PackedCellError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ScalarOverflow { value, max } => {
                write!(f, "scalar overflow: {value} > {max}")
            }
            Self::StyleIndexOverflow { value, max } => {
                write!(f, "style index overflow: {value} > {max}")
            }
            Self::FlagsOverflow { value, max } => write!(f, "flags overflow: {value} > {max}"),
            Self::InvalidWidth(width) => write!(f, "invalid width: {width} (expected 0..=2)"),
            Self::ExtOverflow { value, max } => write!(f, "ext overflow: {value} > {max}"),
        }
    }
}

impl std::error::Error for PackedCellError {}

// INV-005: PackedCellError is Send + Sync + 'static.
const _: () = {
    fn assert_send_sync<T: Send + Sync + 'static>() {}
    fn check() {
        assert_send_sync::<PackedCellError>();
    }
};

impl PackedCell {
    // Bit positions and masks
    const SCALAR_SHIFT: u32 = 43;
    const SCALAR_MASK: u64 = (1u64 << SCALAR_BITS) - 1; // 21 bits

    const STYLE_SHIFT: u32 = 28;
    const STYLE_MASK: u64 = (1u64 << STYLE_BITS) - 1; // 15 bits

    const FLAGS_SHIFT: u32 = 16;
    const FLAGS_MASK: u64 = (1u64 << FLAGS_BITS) - 1; // 12 bits

    const WIDTH_SHIFT: u32 = 14;
    const WIDTH_MASK: u64 = (1u64 << WIDTH_BITS) - 1; // 2 bits

    const EXT_MASK: u64 = (1u64 << EXT_BITS) - 1; // 14 bits

    /// Maximum ext index value (14 bits).
    /// INV-030: Index 0x0000 is reserved for "no extension".
    pub const EXT_MAX: u16 = 0x3FFF;

    /// Create a new PackedCell from its components.
    ///
    /// - `scalar`: Unicode scalar value (up to U+1FFFFF, 21 bits)
    /// - `style_index`: Packed style palette index (15 bits)
    /// - `flags`: Cell flags (12 bits)
    /// - `width`: Display width (0, 1, or 2)
    /// - `ext`: GraphemeArena extension index (14 bits, 0 = no extension)
    ///
    /// This constructor masks overflowing fields to fit bit-width constraints.
    /// Use [`Self::try_new`] when strict validation is required.
    #[must_use]
    pub fn new(scalar: u32, style_index: u16, flags: u16, width: u8, ext: u16) -> Self {
        let scalar = (scalar as u64 & Self::SCALAR_MASK) << Self::SCALAR_SHIFT;
        let style = (style_index as u64 & Self::STYLE_MASK) << Self::STYLE_SHIFT;
        let flags = (flags as u64 & Self::FLAGS_MASK) << Self::FLAGS_SHIFT;
        let width = (width as u64 & Self::WIDTH_MASK) << Self::WIDTH_SHIFT;
        let ext = ext as u64 & Self::EXT_MASK;
        Self(scalar | style | flags | width | ext)
    }

    /// Create a new PackedCell with strict invariant validation.
    #[must_use]
    pub fn try_new(
        scalar: u32,
        style_index: u16,
        flags: u16,
        width: u8,
        ext: u16,
    ) -> Result<Self, PackedCellError> {
        let scalar_max = Self::SCALAR_MASK as u32;
        let style_max = Self::STYLE_MASK as u16;
        let flags_max = Self::FLAGS_MASK as u16;
        let ext_max = Self::EXT_MASK as u16;

        if scalar > scalar_max {
            return Err(PackedCellError::ScalarOverflow {
                value: scalar,
                max: scalar_max,
            });
        }
        if style_index > style_max {
            return Err(PackedCellError::StyleIndexOverflow {
                value: style_index,
                max: style_max,
            });
        }
        if flags > flags_max {
            return Err(PackedCellError::FlagsOverflow {
                value: flags,
                max: flags_max,
            });
        }
        if width > 2 {
            return Err(PackedCellError::InvalidWidth(width));
        }
        if ext > ext_max {
            return Err(PackedCellError::ExtOverflow {
                value: ext,
                max: ext_max,
            });
        }

        Ok(Self::new(scalar, style_index, flags, width, ext))
    }

    /// Create a blank PackedCell (space character, no style, width 1).
    #[must_use]
    pub fn blank() -> Self {
        Self::new(0x20, 0, 0, 1, 0)
    }

    /// The Unicode scalar value (21 bits).
    #[must_use]
    pub fn scalar(&self) -> u32 {
        ((self.0 >> Self::SCALAR_SHIFT) & Self::SCALAR_MASK) as u32
    }

    /// The style palette index (15 bits).
    #[must_use]
    pub fn style_index(&self) -> u16 {
        ((self.0 >> Self::STYLE_SHIFT) & Self::STYLE_MASK) as u16
    }

    /// The cell flags (12 bits).
    #[must_use]
    pub fn flags(&self) -> u16 {
        ((self.0 >> Self::FLAGS_SHIFT) & Self::FLAGS_MASK) as u16
    }

    /// The display width (2 bits: 0, 1, or 2).
    /// INV-029: Width is explicit, never inferred.
    #[must_use]
    pub fn width(&self) -> u8 {
        ((self.0 >> Self::WIDTH_SHIFT) & Self::WIDTH_MASK) as u8
    }

    /// The GraphemeArena extension index (14 bits).
    /// INV-030: 0x0000 means "no extension".
    #[must_use]
    pub fn ext(&self) -> u16 {
        (self.0 & Self::EXT_MASK) as u16
    }

    /// True if this cell has an extension in the GraphemeArena.
    #[must_use]
    pub fn has_ext(&self) -> bool {
        self.ext() != 0
    }

    /// The raw 64-bit value.
    #[must_use]
    pub fn raw(&self) -> u64 {
        self.0
    }

    /// Create from a raw 64-bit value.
    #[must_use]
    pub fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Encode to little-endian bytes.
    /// INV-011: Platform-endian-independent LE canonical.
    #[must_use]
    pub fn to_le_bytes(&self) -> [u8; 8] {
        self.0.to_le_bytes()
    }

    /// Decode from little-endian bytes.
    #[must_use]
    pub fn from_le_bytes(bytes: [u8; 8]) -> Self {
        Self(u64::from_le_bytes(bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem;

    /// INV-007: PackedCell is exactly 64 bits (8 bytes) -- runtime check.
    #[test]
    fn test_packed_cell_size_is_64_bits() {
        assert_eq!(mem::size_of::<PackedCell>(), 8);
    }

    /// S94: Compile-time const assertion that bit fields sum to 64.
    /// (The const assertion above fires at compile time; this test
    /// re-validates at runtime for double safety.)
    #[test]
    fn test_bit_sum_is_64_const() {
        let _ = _ASSERT_BITS_SUM_64;
        assert_eq!(SCALAR_BITS + STYLE_BITS + FLAGS_BITS + WIDTH_BITS + EXT_BITS, 64);
    }

    /// S94: Round-trip all fields through pack/unpack.
    #[test]
    fn test_packed_cell_round_trip() {
        let pc = PackedCell::new(0x4E16, 0x1234, 0xABC, 2, 0x1FF);
        assert_eq!(pc.scalar(), 0x4E16);
        assert_eq!(pc.style_index(), 0x1234);
        assert_eq!(pc.flags(), 0xABC);
        assert_eq!(pc.width(), 2);
        assert_eq!(pc.ext(), 0x1FF);
    }

    /// Round-trip with maximum values for all fields.
    #[test]
    fn test_packed_cell_max_values() {
        let pc = PackedCell::new(0x1FFFFF, 0x7FFF, 0xFFF, 3, 0x3FFF);
        assert_eq!(pc.scalar(), 0x1FFFFF);
        assert_eq!(pc.style_index(), 0x7FFF);
        assert_eq!(pc.flags(), 0xFFF);
        assert_eq!(pc.width(), 3);
        assert_eq!(pc.ext(), 0x3FFF);
    }

    /// Test blank cell is space with width 1.
    #[test]
    fn test_packed_cell_blank() {
        let pc = PackedCell::blank();
        assert_eq!(pc.scalar(), 0x20); // space
        assert_eq!(pc.width(), 1);
        assert_eq!(pc.ext(), 0);
        assert!(!pc.has_ext());
    }

    /// INV-030: ext index 0 means no extension.
    #[test]
    fn test_ext_zero_means_no_extension() {
        let pc = PackedCell::new(0x41, 0, 0, 1, 0);
        assert!(!pc.has_ext());
        let pc2 = PackedCell::new(0x41, 0, 0, 1, 1);
        assert!(pc2.has_ext());
    }

    /// INV-011: LE byte encoding round-trip.
    #[test]
    fn test_le_bytes_round_trip() {
        let pc = PackedCell::new(0x1F600, 0x7FFF, 0xFFF, 2, 0x3FFF);
        let bytes = pc.to_le_bytes();
        let pc2 = PackedCell::from_le_bytes(bytes);
        assert_eq!(pc, pc2);
    }

    /// Raw round-trip.
    #[test]
    fn test_raw_round_trip() {
        let pc = PackedCell::new(0x41, 0x10, 0x20, 1, 0x30);
        let raw = pc.raw();
        let pc2 = PackedCell::from_raw(raw);
        assert_eq!(pc, pc2);
    }

    /// Width 0, 1, 2 are all valid.
    #[test]
    fn test_width_valid_values() {
        for w in 0..=2u8 {
            let pc = PackedCell::new(0x41, 0, 0, w, 0);
            assert_eq!(pc.width(), w);
        }
    }

    #[test]
    fn test_try_new_success() {
        let pc = PackedCell::try_new(0x41, 2, 3, 2, 1).expect("valid packed cell");
        assert_eq!(pc.scalar(), 0x41);
        assert_eq!(pc.style_index(), 2);
        assert_eq!(pc.flags(), 3);
        assert_eq!(pc.width(), 2);
        assert_eq!(pc.ext(), 1);
    }

    #[test]
    fn test_try_new_rejects_scalar_overflow() {
        let err = PackedCell::try_new(0x20_0000, 0, 0, 1, 0).unwrap_err();
        assert!(matches!(err, PackedCellError::ScalarOverflow { .. }));
    }

    #[test]
    fn test_try_new_rejects_style_index_overflow() {
        let err = PackedCell::try_new(0x41, 0x8000, 0, 1, 0).unwrap_err();
        assert!(matches!(err, PackedCellError::StyleIndexOverflow { .. }));
    }

    #[test]
    fn test_try_new_rejects_flags_overflow() {
        let err = PackedCell::try_new(0x41, 0, 0x1000, 1, 0).unwrap_err();
        assert!(matches!(err, PackedCellError::FlagsOverflow { .. }));
    }

    #[test]
    fn test_try_new_rejects_invalid_width() {
        let err = PackedCell::try_new(0x41, 0, 0, 3, 0).unwrap_err();
        assert_eq!(err, PackedCellError::InvalidWidth(3));
    }

    #[test]
    fn test_try_new_rejects_ext_overflow() {
        let err = PackedCell::try_new(0x41, 0, 0, 1, 0x4000).unwrap_err();
        assert!(matches!(err, PackedCellError::ExtOverflow { .. }));
    }
}
