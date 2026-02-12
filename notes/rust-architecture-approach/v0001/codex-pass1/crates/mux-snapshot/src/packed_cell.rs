use mux_types::packed_cell::PackedCell;

use crate::SnapshotError;

#[must_use]
pub fn encode_packed_cells(cells: &[PackedCell]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(cells.len() * 8);
    for cell in cells {
        buf.extend_from_slice(&cell.to_le_bytes());
    }
    buf
}

#[must_use]
pub fn decode_packed_cells(data: &[u8]) -> Result<Vec<PackedCell>, SnapshotError> {
    if data.len() % 8 != 0 {
        return Err(SnapshotError::MalformedPackedCells(data.len()));
    }

    let mut out = Vec::with_capacity(data.len() / 8);
    let mut offset = 0;
    while offset < data.len() {
        let bytes: [u8; 8] = data[offset..offset + 8]
            .try_into()
            .expect("chunk length is always 8");
        out.push(PackedCell::from_le_bytes(bytes));
        offset += 8;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packed_cells_round_trip() {
        let cells = vec![PackedCell::blank(), PackedCell::new(0x4E16, 1, 2, 2, 3)];
        let bytes = encode_packed_cells(&cells);
        let decoded = decode_packed_cells(&bytes).unwrap();
        assert_eq!(decoded, cells);
    }

    #[test]
    fn malformed_length_rejected() {
        assert!(matches!(
            decode_packed_cells(&[1, 2, 3]),
            Err(SnapshotError::MalformedPackedCells(3))
        ));
    }

    #[test]
    fn empty_ok() {
        let decoded = decode_packed_cells(&[]).unwrap();
        assert!(decoded.is_empty());
    }
}
