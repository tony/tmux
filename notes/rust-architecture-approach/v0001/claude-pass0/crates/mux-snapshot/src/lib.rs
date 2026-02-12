//! # mux-snapshot
//!
//! Binary snapshot format for terminal grid state.
//!
//! ## Header (INV-023: exactly 31 bytes)
//! ```text
//! Offset  Size  Field
//! 0       8     Magic ("TFSNAP13")
//! 8       2     Format version (u16 LE)
//! 10      1     Checksum algorithm (0=CRC32C, 1=FNV-1a)
//! 11      4     Checksum (u32 LE)
//! 15      4     Row count (u32 LE)
//! 19      4     Col count (u32 LE)
//! 23      4     Cell count (u32 LE)
//! 27      4     Arena size (u32 LE)
//! ```
//!
//! ## Invariants
//! - INV-023: Header is exactly 31 bytes.
//! - S93: CRC32C is canonical for encode. FNV-1a is decode-only.
//! - INV-031: CRC32C is the only checksum used for new snapshots.
//! - INV-011: Little-endian canonical encoding.

#![forbid(unsafe_code)]

use mux_types::packed_cell::PackedCell;

/// Snapshot magic bytes.
pub const MAGIC: &[u8; 8] = b"TFSNAP13";

/// Current format version.
pub const FORMAT_VERSION: u16 = 1;

/// Header size in bytes.
/// INV-023: Exactly 31 bytes.
pub const HEADER_SIZE: usize = 31;

/// Checksum algorithm identifiers.
/// S93: CRC32C is canonical for encode. FNV-1a is decode-only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ChecksumAlgorithm {
    /// CRC32C (canonical for encode). INV-031.
    Crc32c = 0,
    /// FNV-1a (decode-only backward compatibility).
    Fnv1a = 1,
}

impl ChecksumAlgorithm {
    /// Parse from a byte value.
    pub fn from_byte(b: u8) -> Option<Self> {
        match b {
            0 => Some(Self::Crc32c),
            1 => Some(Self::Fnv1a),
            _ => None,
        }
    }
}

/// Snapshot header.
/// INV-023: Exactly 31 bytes when serialized.
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
    /// Create a new header for encoding.
    pub fn new(row_count: u32, col_count: u32, cell_count: u32, arena_size: u32) -> Self {
        Self {
            magic: *MAGIC,
            format_version: FORMAT_VERSION,
            checksum_algorithm: ChecksumAlgorithm::Crc32c,
            checksum: 0, // computed later
            row_count,
            col_count,
            cell_count,
            arena_size,
        }
    }

    /// Encode the header to bytes.
    /// INV-011: Little-endian encoding.
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

    /// Decode a header from bytes.
    pub fn decode(data: &[u8]) -> Result<Self, SnapshotError> {
        if data.len() < HEADER_SIZE {
            return Err(SnapshotError::TruncatedHeader {
                expected: HEADER_SIZE,
                actual: data.len(),
            });
        }

        let magic: [u8; 8] = data[0..8].try_into().unwrap();
        if &magic != MAGIC {
            return Err(SnapshotError::InvalidMagic(magic));
        }

        let format_version = u16::from_le_bytes([data[8], data[9]]);
        let algo_byte = data[10];
        let checksum_algorithm = ChecksumAlgorithm::from_byte(algo_byte)
            .ok_or(SnapshotError::UnknownChecksumAlgorithm(algo_byte))?;
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

/// Snapshot error types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotError {
    TruncatedHeader { expected: usize, actual: usize },
    InvalidMagic([u8; 8]),
    UnknownChecksumAlgorithm(u8),
    ChecksumMismatch { expected: u32, actual: u32 },
    TruncatedPayload { expected: usize, actual: usize },
}

impl std::fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TruncatedHeader { expected, actual } =>
                write!(f, "truncated header: expected {expected} bytes, got {actual}"),
            Self::InvalidMagic(m) =>
                write!(f, "invalid magic: {:?}", String::from_utf8_lossy(m)),
            Self::UnknownChecksumAlgorithm(a) =>
                write!(f, "unknown checksum algorithm: {a}"),
            Self::ChecksumMismatch { expected, actual } =>
                write!(f, "checksum mismatch: expected 0x{expected:08X}, got 0x{actual:08X}"),
            Self::TruncatedPayload { expected, actual } =>
                write!(f, "truncated payload: expected {expected} bytes, got {actual}"),
        }
    }
}

impl std::error::Error for SnapshotError {}

/// Compute CRC32C checksum of a byte slice.
/// S93: CRC32C is canonical for encode.
pub fn crc32c(data: &[u8]) -> u32 {
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(data);
    hasher.finalize()
}

/// Encode a slice of PackedCells to bytes.
/// INV-011: Little-endian encoding.
pub fn encode_cells(cells: &[PackedCell]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(cells.len() * 8);
    for cell in cells {
        buf.extend_from_slice(&cell.to_le_bytes());
    }
    buf
}

/// Decode PackedCells from bytes.
pub fn decode_cells(data: &[u8]) -> Vec<PackedCell> {
    let mut cells = Vec::with_capacity(data.len() / 8);
    let mut offset = 0;
    while offset + 8 <= data.len() {
        let bytes: [u8; 8] = data[offset..offset + 8].try_into().unwrap();
        cells.push(PackedCell::from_le_bytes(bytes));
        offset += 8;
    }
    cells
}

/// Encode a complete snapshot: header + cells + optional arena trailer.
pub fn encode_snapshot(
    rows: u32,
    cols: u32,
    cells: &[PackedCell],
    arena_data: Option<&[u8]>,
) -> Vec<u8> {
    let cell_data = encode_cells(cells);
    let arena_size = arena_data.map(|a| a.len() as u32).unwrap_or(0);

    let mut header = SnapshotHeader::new(rows, cols, cells.len() as u32, arena_size);

    // Compute checksum over cell data + arena data
    let mut to_checksum = Vec::new();
    to_checksum.extend_from_slice(&cell_data);
    if let Some(arena) = arena_data {
        to_checksum.extend_from_slice(arena);
    }
    header.checksum = crc32c(&to_checksum);

    let mut buf = Vec::new();
    buf.extend_from_slice(&header.encode());
    buf.extend_from_slice(&cell_data);
    if let Some(arena) = arena_data {
        buf.extend_from_slice(arena);
    }
    buf
}

/// Decode a complete snapshot, verifying the checksum.
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
    let arena_data = &data[HEADER_SIZE + cell_data_len..HEADER_SIZE + cell_data_len + arena_data_len];

    // Verify checksum (CRC32C only for now; FNV-1a decode support is TODO)
    let mut payload = Vec::new();
    payload.extend_from_slice(cell_data);
    payload.extend_from_slice(arena_data);

    let computed_checksum = crc32c(&payload);
    if computed_checksum != header.checksum {
        return Err(SnapshotError::ChecksumMismatch {
            expected: header.checksum,
            actual: computed_checksum,
        });
    }

    let cells = decode_cells(cell_data);

    Ok((header, cells, arena_data.to_vec()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// INV-023: Header is exactly 31 bytes.
    #[test]
    fn test_header_size_is_31() {
        assert_eq!(HEADER_SIZE, 31);
        let header = SnapshotHeader::new(24, 80, 1920, 0);
        let encoded = header.encode();
        assert_eq!(encoded.len(), 31);
    }

    /// Header encode/decode round-trip.
    #[test]
    fn test_header_round_trip() {
        let header = SnapshotHeader::new(24, 80, 1920, 128);
        let encoded = header.encode();
        let decoded = SnapshotHeader::decode(&encoded).unwrap();
        assert_eq!(decoded.magic, *MAGIC);
        assert_eq!(decoded.row_count, 24);
        assert_eq!(decoded.col_count, 80);
        assert_eq!(decoded.cell_count, 1920);
        assert_eq!(decoded.arena_size, 128);
    }

    /// Full snapshot encode/decode round-trip with CRC32C.
    #[test]
    fn test_snapshot_round_trip() {
        let cells = vec![
            PackedCell::blank(),
            PackedCell::new(0x41, 0, 0, 1, 0), // 'A'
            PackedCell::new(0x4E16, 0, 0, 2, 0), // CJK char
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

    /// Truncated header is rejected.
    #[test]
    fn test_truncated_header_rejected() {
        let data = [0u8; 10]; // too short
        assert!(matches!(
            SnapshotHeader::decode(&data),
            Err(SnapshotError::TruncatedHeader { .. })
        ));
    }

    /// Invalid magic is rejected.
    #[test]
    fn test_invalid_magic_rejected() {
        let mut data = [0u8; 31];
        data[0..8].copy_from_slice(b"NOTMAGIC");
        assert!(matches!(
            SnapshotHeader::decode(&data),
            Err(SnapshotError::InvalidMagic(_))
        ));
    }
}
