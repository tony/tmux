use mux_snapshot::Snapshot;
use mux_termlet::Termlet;
use mux_types::Size;

#[derive(Debug, Clone)]
pub struct FfiSize {
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug, Clone)]
pub struct FfiSnapshot {
    pub text: String,
    pub cols: u16,
    pub rows: u16,
    pub revision: u64,
}

impl From<Size> for FfiSize {
    fn from(value: Size) -> Self {
        Self {
            cols: value.cols,
            rows: value.rows,
        }
    }
}

impl From<&Snapshot> for FfiSnapshot {
    fn from(snapshot: &Snapshot) -> Self {
        Self {
            text: snapshot.text(),
            cols: snapshot.size.cols,
            rows: snapshot.size.rows,
            revision: snapshot.revision,
        }
    }
}

pub fn capture_termlet(termlet: &Termlet) -> Result<FfiSnapshot, mux_termlet::TermletError> {
    let snapshot = termlet.capture()?;
    Ok(FfiSnapshot::from(&snapshot))
}
