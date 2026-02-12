//! # mux-snapshot
//!
//! Binary snapshot format for terminal grid state.

#![forbid(unsafe_code)]

pub mod packed_cell;

use mux_types::packed_cell::PackedCell;

pub use packed_cell::{decode_packed_cells, encode_packed_cells};

/// Snapshot magic bytes.
pub const MAGIC: &[u8; 8] = b"TFSNAP13";

/// Current format version.
pub const FORMAT_VERSION: u16 = 1;

/// Header size in bytes (INV-023).
pub const HEADER_SIZE: usize = 31;

/// Checksum algorithm identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ChecksumAlgorithm {
    /// CRC32C canonical encode (INV-031).
    Crc32c = 0,
    /// FNV-1a decode-only compatibility (S93).
    Fnv1a = 1,
}

impl ChecksumAlgorithm {
    #[must_use]
    pub fn from_byte(b: u8) -> Option<Self> {
        match b {
            0 => Some(Self::Crc32c),
            1 => Some(Self::Fnv1a),
            _ => None,
        }
    }
}

/// Snapshot header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotHeader {
    pub magic: [u8; 8],
    pub format_version: u16,
    pub checksum_algorithm: ChecksumAlgorithm,
    pub checksum: u32,
    pub row_count: u32,
    pub col_count: u32,
    pub cell_count: u32,
    pub arena_size: u32,
}

impl SnapshotHeader {
    #[must_use]
    pub fn new(row_count: u32, col_count: u32, cell_count: u32, arena_size: u32) -> Self {
        Self {
            magic: *MAGIC,
            format_version: FORMAT_VERSION,
            checksum_algorithm: ChecksumAlgorithm::Crc32c,
            checksum: 0,
            row_count,
            col_count,
            cell_count,
            arena_size,
        }
    }

    #[must_use]
    pub fn encode(&self) -> [u8; HEADER_SIZE] {
        let mut buf = [0u8; HEADER_SIZE];
        buf[0..8].copy_from_slice(&self.magic);
        buf[8..10].copy_from_slice(&self.format_version.to_le_bytes());
        buf[10] = self.checksum_algorithm as u8;
        buf[11..15].copy_from_slice(&self.checksum.to_le_bytes());
        buf[15..19].copy_from_slice(&self.row_count.to_le_bytes());
        buf[19..23].copy_from_slice(&self.col_count.to_le_bytes());
        buf[23..27].copy_from_slice(&self.cell_count.to_le_bytes());
        buf[27..31].copy_from_slice(&self.arena_size.to_le_bytes());
        buf
    }

    #[must_use]
    pub fn decode(data: &[u8]) -> Result<Self, SnapshotError> {
        if data.len() < HEADER_SIZE {
            return Err(SnapshotError::TruncatedHeader {
                expected: HEADER_SIZE,
                actual: data.len(),
            });
        }

        let magic: [u8; 8] = data[0..8]
            .try_into()
            .expect("slice length is validated above");
        if &magic != MAGIC {
            return Err(SnapshotError::InvalidMagic(magic));
        }

        let format_version = u16::from_le_bytes([data[8], data[9]]);
        let algo_byte = data[10];
        let checksum_algorithm =
            ChecksumAlgorithm::from_byte(algo_byte).ok_or(SnapshotError::UnknownChecksumAlgorithm(algo_byte))?;
        let checksum = u32::from_le_bytes([data[11], data[12], data[13], data[14]]);
        let row_count = u32::from_le_bytes([data[15], data[16], data[17], data[18]]);
        let col_count = u32::from_le_bytes([data[19], data[20], data[21], data[22]]);
        let cell_count = u32::from_le_bytes([data[23], data[24], data[25], data[26]]);
        let arena_size = u32::from_le_bytes([data[27], data[28], data[29], data[30]]);

        Ok(Self {
            magic,
            format_version,
            checksum_algorithm,
            checksum,
            row_count,
            col_count,
            cell_count,
            arena_size,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotError {
    TruncatedHeader { expected: usize, actual: usize },
    InvalidMagic([u8; 8]),
    UnknownChecksumAlgorithm(u8),
    ChecksumMismatch { expected: u32, actual: u32 },
    TruncatedPayload { expected: usize, actual: usize },
    MalformedPackedCells(usize),
}

impl std::fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TruncatedHeader { expected, actual } => {
                write!(f, "truncated header: expected {expected} bytes, got {actual}")
            }
            Self::InvalidMagic(m) => write!(f, "invalid magic: {:?}", String::from_utf8_lossy(m)),
            Self::UnknownChecksumAlgorithm(a) => write!(f, "unknown checksum algorithm: {a}"),
            Self::ChecksumMismatch { expected, actual } => {
                write!(f, "checksum mismatch: expected 0x{expected:08X}, got 0x{actual:08X}")
            }
            Self::TruncatedPayload { expected, actual } => {
                write!(f, "truncated payload: expected {expected} bytes, got {actual}")
            }
            Self::MalformedPackedCells(len) => {
                write!(f, "malformed packed-cell section length: {len}")
            }
        }
    }
}

impl std::error::Error for SnapshotError {}

#[must_use]
pub fn crc32c(data: &[u8]) -> u32 {
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(data);
    hasher.finalize()
}

#[must_use]
pub fn encode_cells(cells: &[PackedCell]) -> Vec<u8> {
    encode_packed_cells(cells)
}

#[must_use]
pub fn decode_cells(data: &[u8]) -> Result<Vec<PackedCell>, SnapshotError> {
    decode_packed_cells(data)
}

#[must_use]
pub fn encode_snapshot(
    rows: u32,
    cols: u32,
    cells: &[PackedCell],
    arena_data: Option<&[u8]>,
) -> Vec<u8> {
    let cell_data = encode_cells(cells);
    let arena_size = arena_data.map_or(0, |a| a.len() as u32);

    let mut header = SnapshotHeader::new(rows, cols, cells.len() as u32, arena_size);

    let mut payload = Vec::with_capacity(cell_data.len() + arena_data.map_or(0, |a| a.len()));
    payload.extend_from_slice(&cell_data);
    if let Some(arena) = arena_data {
        payload.extend_from_slice(arena);
    }
    header.checksum = crc32c(&payload);

    let mut buf = Vec::with_capacity(HEADER_SIZE + payload.len());
    buf.extend_from_slice(&header.encode());
    buf.extend_from_slice(&payload);
    buf
}

#[must_use]
pub fn decode_snapshot(data: &[u8]) -> Result<(SnapshotHeader, Vec<PackedCell>, Vec<u8>), SnapshotError> {
    let header = SnapshotHeader::decode(data)?;

    let cell_data_len = header.cell_count as usize * 8;
    let arena_data_len = header.arena_size as usize;
    let expected_len = HEADER_SIZE + cell_data_len + arena_data_len;
    if data.len() < expected_len {
        return Err(SnapshotError::TruncatedPayload {
            expected: expected_len,
            actual: data.len(),
        });
    }

    let cell_data = &data[HEADER_SIZE..HEADER_SIZE + cell_data_len];
    let arena_data = &data[HEADER_SIZE + cell_data_len..expected_len];

    let mut payload = Vec::with_capacity(cell_data.len() + arena_data.len());
    payload.extend_from_slice(cell_data);
    payload.extend_from_slice(arena_data);
    let computed_checksum = crc32c(&payload);
    if computed_checksum != header.checksum {
        return Err(SnapshotError::ChecksumMismatch {
            expected: header.checksum,
            actual: computed_checksum,
        });
    }

    let cells = decode_cells(cell_data)?;
    Ok((header, cells, arena_data.to_vec()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_size_is_31_bytes() {
        assert_eq!(HEADER_SIZE, 31);
        let header = SnapshotHeader::new(24, 80, 1920, 0);
        assert_eq!(header.encode().len(), 31);
    }

    #[test]
    fn checksum_algo_offset_30() {
        let header = SnapshotHeader::new(1, 1, 1, 0);
        let encoded = header.encode();
        assert_eq!(encoded[30], 0);
    }

    #[test]
    fn header_round_trip() {
        let header = SnapshotHeader::new(24, 80, 1920, 128);
        let encoded = header.encode();
        let decoded = SnapshotHeader::decode(&encoded).unwrap();
        assert_eq!(decoded.magic, *MAGIC);
        assert_eq!(decoded.row_count, 24);
        assert_eq!(decoded.col_count, 80);
        assert_eq!(decoded.cell_count, 1920);
        assert_eq!(decoded.arena_size, 128);
    }

    #[test]
    fn snapshot_round_trip() {
        let cells = vec![
            PackedCell::blank(),
            PackedCell::new(0x41, 0, 0, 1, 0),
            PackedCell::new(0x4E16, 0, 0, 2, 0),
        ];
        let arena_data = b"test arena data";

        let encoded = encode_snapshot(1, 3, &cells, Some(arena_data));
        let (header, decoded_cells, decoded_arena) = decode_snapshot(&encoded).unwrap();

        assert_eq!(header.row_count, 1);
        assert_eq!(header.col_count, 3);
        assert_eq!(decoded_cells.len(), 3);
        assert_eq!(decoded_cells[1].scalar(), 0x41);
        assert_eq!(decoded_arena, arena_data);
    }

    #[test]
    fn truncated_header_rejected() {
        let data = [0u8; 10];
        assert!(matches!(
            SnapshotHeader::decode(&data),
            Err(SnapshotError::TruncatedHeader { .. })
        ));
    }

    #[test]
    fn invalid_magic_rejected() {
        let mut data = [0u8; 31];
        data[0..8].copy_from_slice(b"NOTMAGIC");
        assert!(matches!(
            SnapshotHeader::decode(&data),
            Err(SnapshotError::InvalidMagic(_))
        ));
    }

    #[test]
    fn checksum_mismatch_rejected() {
        let cells = vec![PackedCell::new(0x41, 0, 0, 1, 0)];
        let mut encoded = encode_snapshot(1, 1, &cells, None);
        encoded[HEADER_SIZE] ^= 0x01;
        assert!(matches!(
            decode_snapshot(&encoded),
            Err(SnapshotError::ChecksumMismatch { .. })
        ));
    }

    #[test]
    fn malformed_cell_section_rejected() {
        assert!(matches!(
            decode_cells(&[1, 2, 3]),
            Err(SnapshotError::MalformedPackedCells(3))
        ));
    }
}
