use core::fmt;

use serde::{Deserialize, Serialize};

use crate::{Cell, CellWidth};

const SCALAR_BITS: u64 = 21;
const STYLE_BITS: u64 = 15;
const FLAGS_BITS: u64 = 12;
const WIDTH_BITS: u64 = 2;
const EXT_BITS: u64 = 14;

const SCALAR_SHIFT: u64 = 0;
const STYLE_SHIFT: u64 = SCALAR_SHIFT + SCALAR_BITS;
const FLAGS_SHIFT: u64 = STYLE_SHIFT + STYLE_BITS;
const WIDTH_SHIFT: u64 = FLAGS_SHIFT + FLAGS_BITS;
const EXT_SHIFT: u64 = WIDTH_SHIFT + WIDTH_BITS;

const fn mask(bits: u64) -> u64 {
    (1u64 << bits) - 1
}

const SCALAR_MASK: u64 = mask(SCALAR_BITS);
const STYLE_MASK: u64 = mask(STYLE_BITS);
const FLAGS_MASK: u64 = mask(FLAGS_BITS);
const WIDTH_MASK: u64 = mask(WIDTH_BITS);
const EXT_MASK: u64 = mask(EXT_BITS);

/// PackedCell layout (S94): scalar:21 + style:15 + flags:12 + width:2 + ext:14.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(Serialize, Deserialize)]
pub struct PackedCell(pub u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackedCellError {
    ScalarOutOfRange(u32),
    StyleOutOfRange(u16),
    FlagsOutOfRange(u16),
    WidthOutOfRange(u8),
    ExtOutOfRange(u16),
}

impl fmt::Display for PackedCellError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ScalarOutOfRange(v) => write!(f, "scalar out of range: {v}"),
            Self::StyleOutOfRange(v) => write!(f, "style out of range: {v}"),
            Self::FlagsOutOfRange(v) => write!(f, "flags out of range: {v}"),
            Self::WidthOutOfRange(v) => write!(f, "width out of range: {v}"),
            Self::ExtOutOfRange(v) => write!(f, "ext out of range: {v}"),
        }
    }
}

impl std::error::Error for PackedCellError {}

impl PackedCell {
    pub fn pack(
        scalar: u32,
        style: u16,
        flags: u16,
        width: u8,
        ext: u16,
    ) -> Result<Self, PackedCellError> {
        if (scalar as u64) > SCALAR_MASK {
            return Err(PackedCellError::ScalarOutOfRange(scalar));
        }
        if (style as u64) > STYLE_MASK {
            return Err(PackedCellError::StyleOutOfRange(style));
        }
        if (flags as u64) > FLAGS_MASK {
            return Err(PackedCellError::FlagsOutOfRange(flags));
        }
        if (width as u64) > WIDTH_MASK {
            return Err(PackedCellError::WidthOutOfRange(width));
        }
        if (ext as u64) > EXT_MASK {
            return Err(PackedCellError::ExtOutOfRange(ext));
        }

        let raw = ((scalar as u64) << SCALAR_SHIFT)
            | ((style as u64) << STYLE_SHIFT)
            | ((flags as u64) << FLAGS_SHIFT)
            | ((width as u64) << WIDTH_SHIFT)
            | ((ext as u64) << EXT_SHIFT);
        Ok(Self(raw))
    }

    pub fn unpack(self) -> (u32, u16, u16, u8, u16) {
        let scalar = ((self.0 >> SCALAR_SHIFT) & SCALAR_MASK) as u32;
        let style = ((self.0 >> STYLE_SHIFT) & STYLE_MASK) as u16;
        let flags = ((self.0 >> FLAGS_SHIFT) & FLAGS_MASK) as u16;
        let width = ((self.0 >> WIDTH_SHIFT) & WIDTH_MASK) as u8;
        let ext = ((self.0 >> EXT_SHIFT) & EXT_MASK) as u16;
        (scalar, style, flags, width, ext)
    }

    pub fn scalar(self) -> u32 {
        ((self.0 >> SCALAR_SHIFT) & SCALAR_MASK) as u32
    }

    pub fn width(self) -> u8 {
        ((self.0 >> WIDTH_SHIFT) & WIDTH_MASK) as u8
    }

    pub fn ext(self) -> u16 {
        ((self.0 >> EXT_SHIFT) & EXT_MASK) as u16
    }

    pub fn from_cell(cell: &Cell) -> Result<Self, PackedCellError> {
        let scalar = cell.scalar().unwrap_or('\u{FFFD}') as u32;
        Self::pack(scalar, cell.style & 0x7fff, cell.flags & 0x0fff, cell.width as u8, cell.ext)
    }

    pub fn to_cell(self) -> Cell {
        let (scalar, style, flags, width, ext) = self.unpack();
        let ch = char::from_u32(scalar).unwrap_or('\u{FFFD}');
        let width = match width {
            0 => CellWidth::Zero,
            1 => CellWidth::One,
            2 => CellWidth::Two,
            _ => CellWidth::Ambiguous,
        };
        Cell::new(ch.to_string(), style, flags, width).with_ext(ext)
    }
}

impl fmt::Display for PackedCell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PackedCell(0x{:016x})", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_pack_unpack() {
        let p = PackedCell::pack('A' as u32, 0x1234, 0x0567, 1, 0x02aa).unwrap();
        assert_eq!(p.unpack(), ('A' as u32, 0x1234, 0x0567, 1, 0x02aa));
    }

    #[test]
    fn rejects_out_of_range_values() {
        assert!(matches!(
            PackedCell::pack(1 << 21, 0, 0, 0, 0),
            Err(PackedCellError::ScalarOutOfRange(_))
        ));
        assert!(matches!(
            PackedCell::pack(0, 1 << 15, 0, 0, 0),
            Err(PackedCellError::StyleOutOfRange(_))
        ));
    }

    #[test]
    fn cell_conversion_roundtrip() {
        let c = Cell::new("Z", 11, 7, CellWidth::One).with_ext(9);
        let p = PackedCell::from_cell(&c).unwrap();
        let c2 = p.to_cell();
        assert_eq!(c2.grapheme, "Z");
        assert_eq!(c2.style, 11);
        assert_eq!(c2.flags, 7);
        assert_eq!(c2.ext, 9);
    }

    #[test]
    fn exact_64_bit_layout() {
        assert_eq!(SCALAR_BITS + STYLE_BITS + FLAGS_BITS + WIDTH_BITS + EXT_BITS, 64);
        assert_eq!(EXT_SHIFT, 50);
    }
}
