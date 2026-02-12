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

/// INV-007: PackedCell is exactly 64 bits.
/// S94: scalar:21 + style:15 + flags:12 + width:2 + ext:14 = 64.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PackedCell(u64);

impl PackedCell {
    // Compile-time field widths (S94).
    pub const SCALAR_BITS: u32 = 21;
    pub const STYLE_BITS: u32 = 15;
    pub const FLAGS_BITS: u32 = 12;
    pub const WIDTH_BITS: u32 = 2;
    pub const EXT_BITS: u32 = 14;
    pub const TOTAL_BITS: u32 =
        Self::SCALAR_BITS + Self::STYLE_BITS + Self::FLAGS_BITS + Self::WIDTH_BITS + Self::EXT_BITS;

    // Compile-time validation that packed layout is exactly 64 bits.
    const _LAYOUT_ASSERT: [(); 64] = [(); Self::TOTAL_BITS as usize];

    // Bit positions and masks
    const SCALAR_SHIFT: u32 = 43;
    const SCALAR_MASK: u64 = 0x1F_FFFF; // 21 bits

    const STYLE_SHIFT: u32 = 28;
    const STYLE_MASK: u64 = 0x7FFF; // 15 bits

    const FLAGS_SHIFT: u32 = 16;
    const FLAGS_MASK: u64 = 0xFFF; // 12 bits

    const WIDTH_SHIFT: u32 = 14;
    const WIDTH_MASK: u64 = 0x3; // 2 bits

    const EXT_MASK: u64 = 0x3FFF; // 14 bits

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
    #[must_use]
    pub fn new(scalar: u32, style_index: u16, flags: u16, width: u8, ext: u16) -> Self {
        let scalar = (scalar as u64 & Self::SCALAR_MASK) << Self::SCALAR_SHIFT;
        let style = (style_index as u64 & Self::STYLE_MASK) << Self::STYLE_SHIFT;
        let flags = (flags as u64 & Self::FLAGS_MASK) << Self::FLAGS_SHIFT;
        let width = (width as u64 & Self::WIDTH_MASK) << Self::WIDTH_SHIFT;
        let ext = ext as u64 & Self::EXT_MASK;
        Self(scalar | style | flags | width | ext)
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

    /// INV-007: PackedCell is exactly 64 bits (8 bytes).
    #[test]
    fn test_packed_cell_size_is_64_bits() {
        assert_eq!(mem::size_of::<PackedCell>(), 8);
    }

    /// S94: Compile-time field sum stays at exactly 64 bits.
    #[test]
    fn test_packed_cell_bit_sum_is_64() {
        assert_eq!(PackedCell::TOTAL_BITS, 64);
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
}
