#![forbid(unsafe_code)]

pub mod packed_cell;

use crc32fast::Hasher;
use mux_grapheme_arena::GraphemeArena;
use mux_types::PackedCell;
use serde::{Deserialize, Serialize};

pub use packed_cell::{decode_packed_cells, encode_packed_cells};

pub const HEADER_LEN: usize = 31;
pub const CHECKSUM_OFFSET: usize = 30;
pub const CHECKSUM_ALGO_CRC32C: u8 = 1;
pub const CHECKSUM_ALGO_FNV1A_LEGACY: u8 = 2;
const MAGIC: &[u8; 8] = b"TFSNAP15";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    pub rows: u16,
    pub cols: u16,
    pub cells: Vec<PackedCell>,
    pub arena: Option<GraphemeArena>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotError {
    Truncated,
    BadMagic,
    UnsupportedChecksum(u8),
    ChecksumMismatch,
    Malformed,
}

impl std::fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Truncated => f.write_str("snapshot truncated"),
            Self::BadMagic => f.write_str("snapshot bad magic"),
            Self::UnsupportedChecksum(c) => write!(f, "unsupported checksum algorithm: {c}"),
            Self::ChecksumMismatch => f.write_str("snapshot checksum mismatch"),
            Self::Malformed => f.write_str("snapshot malformed"),
        }
    }
}

impl std::error::Error for SnapshotError {}

impl Snapshot {
    pub fn encode(&self) -> Vec<u8> {
        let mut out = vec![0u8; HEADER_LEN];
        out[..8].copy_from_slice(MAGIC);
        out[8] = 15; // schema version
        out[9..11].copy_from_slice(&self.rows.to_le_bytes());
        out[11..13].copy_from_slice(&self.cols.to_le_bytes());
        out[13..17].copy_from_slice(&(self.cells.len() as u32).to_le_bytes());
        let arena_count = self.arena.as_ref().map(|a| a.as_entries().len()).unwrap_or(0) as u32;
        out[17..21].copy_from_slice(&arena_count.to_le_bytes());
        out[CHECKSUM_OFFSET] = CHECKSUM_ALGO_CRC32C;

        out.extend_from_slice(&encode_packed_cells(&self.cells));

        if let Some(arena) = &self.arena {
            // INV-028: optional trailing arena section after cells.
            for entry in arena.as_entries() {
                let bytes = entry.as_bytes();
                let len = bytes.len() as u16;
                out.extend_from_slice(&len.to_le_bytes());
                out.extend_from_slice(bytes);
            }
        }

        let checksum = crc32c(&out);
        out.extend_from_slice(&checksum.to_le_bytes());
        out
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, SnapshotError> {
        if bytes.len() < HEADER_LEN + 4 {
            return Err(SnapshotError::Truncated);
        }
        if &bytes[..8] != MAGIC {
            return Err(SnapshotError::BadMagic);
        }
        let algo = bytes[CHECKSUM_OFFSET];

        let payload_end = bytes.len() - 4;
        let expected = u32::from_le_bytes(bytes[payload_end..].try_into().map_err(|_| SnapshotError::Malformed)?);
        let got = match algo {
            CHECKSUM_ALGO_CRC32C => crc32c(&bytes[..payload_end]),
            CHECKSUM_ALGO_FNV1A_LEGACY => fnv1a32(&bytes[..payload_end]),
            other => return Err(SnapshotError::UnsupportedChecksum(other)),
        };
        if expected != got {
            return Err(SnapshotError::ChecksumMismatch);
        }

        let rows = u16::from_le_bytes(bytes[9..11].try_into().map_err(|_| SnapshotError::Malformed)?);
        let cols = u16::from_le_bytes(bytes[11..13].try_into().map_err(|_| SnapshotError::Malformed)?);
        let cell_count =
            u32::from_le_bytes(bytes[13..17].try_into().map_err(|_| SnapshotError::Malformed)?) as usize;

        let cells_start = HEADER_LEN;
        let cells_len = cell_count.checked_mul(8).ok_or(SnapshotError::Malformed)?;
        let cells_end = cells_start.checked_add(cells_len).ok_or(SnapshotError::Malformed)?;
        if cells_end > payload_end {
            return Err(SnapshotError::Truncated);
        }
        let cells = decode_packed_cells(&bytes[cells_start..cells_end])?;

        Ok(Self {
            rows,
            cols,
            cells,
            arena: None,
        })
    }
}

pub fn crc32c(data: &[u8]) -> u32 {
    let mut hasher = Hasher::new();
    hasher.update(data);
    hasher.finalize()
}

pub fn fnv1a32(data: &[u8]) -> u32 {
    const OFFSET: u32 = 0x811c9dc5;
    const PRIME: u32 = 0x01000193;
    let mut hash = OFFSET;
    for b in data {
        hash ^= *b as u32;
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

#[cfg(test)]
mod tests {
    use mux_types::PackedCell;

    use super::*;

    #[test]
    fn header_contract() {
        assert_eq!(HEADER_LEN, 31);
        assert_eq!(CHECKSUM_OFFSET, 30);
    }

    #[test]
    fn roundtrip_crc32c_snapshot() {
        let snap = Snapshot {
            rows: 2,
            cols: 2,
            cells: vec![PackedCell::pack('a' as u32, 0, 0, 1, 0).unwrap()],
            arena: None,
        };
        let bytes = snap.encode();
        let out = Snapshot::decode(&bytes).unwrap();
        assert_eq!(out.rows, 2);
        assert_eq!(out.cells.len(), 1);
    }

    #[test]
    fn checksum_mismatch_detected() {
        let snap = Snapshot {
            rows: 1,
            cols: 1,
            cells: vec![PackedCell::pack('x' as u32, 0, 0, 1, 0).unwrap()],
            arena: None,
        };
        let mut bytes = snap.encode();
        let idx = HEADER_LEN;
        bytes[idx] ^= 0x1;
        assert!(matches!(Snapshot::decode(&bytes), Err(SnapshotError::ChecksumMismatch)));
    }

    #[test]
    fn decode_supports_legacy_fnv() {
        let snap = Snapshot {
            rows: 1,
            cols: 1,
            cells: vec![PackedCell::pack('x' as u32, 0, 0, 1, 0).unwrap()],
            arena: None,
        };
        let mut bytes = snap.encode();
        bytes[CHECKSUM_OFFSET] = CHECKSUM_ALGO_FNV1A_LEGACY;
        let end = bytes.len() - 4;
        let checksum = fnv1a32(&bytes[..end]);
        bytes[end..].copy_from_slice(&checksum.to_le_bytes());
        let out = Snapshot::decode(&bytes).unwrap();
        assert_eq!(out.rows, 1);
    }
}
