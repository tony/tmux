use mux_types::PackedCell;

use crate::SnapshotError;

pub fn encode_packed_cells(cells: &[PackedCell]) -> Vec<u8> {
    let mut out = Vec::with_capacity(cells.len() * 8);
    for c in cells {
        out.extend_from_slice(&c.0.to_le_bytes());
    }
    out
}

pub fn decode_packed_cells(bytes: &[u8]) -> Result<Vec<PackedCell>, SnapshotError> {
    if bytes.len() % 8 != 0 {
        return Err(SnapshotError::Malformed);
    }
    let mut out = Vec::with_capacity(bytes.len() / 8);
    for chunk in bytes.chunks_exact(8) {
        let raw = u64::from_le_bytes(chunk.try_into().map_err(|_| SnapshotError::Malformed)?);
        out.push(PackedCell(raw));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packed_cells_roundtrip() {
        let cells = vec![PackedCell(1), PackedCell(u64::MAX - 1)];
        let bytes = encode_packed_cells(&cells);
        let got = decode_packed_cells(&bytes).unwrap();
        assert_eq!(got, cells);
    }

    #[test]
    fn malformed_length_rejected() {
        assert!(decode_packed_cells(&[1, 2, 3]).is_err());
    }

    #[test]
    fn empty_ok() {
        let got = decode_packed_cells(&[]).unwrap();
        assert!(got.is_empty());
    }
}
