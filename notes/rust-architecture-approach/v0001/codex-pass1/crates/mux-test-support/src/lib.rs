//! # mux-test-support
//!
//! Test harness for TermForge: isolated sockets, cleanup guards,
//! differential testing against tmux, and snapshot test utilities.
//!
//! ## Key Design
//! - RULE-S29-06: Test isolation via unique socket names.
//! - RULE-S29-08: TestGuard cleans up all resources on drop.
//! - RULE-S29-05: Differential testing against tmux binary.
//! - RULE-S29-07: Snapshot tests with golden files.

#![forbid(unsafe_code)]

use std::collections::HashMap;

/// Minimum required fuzz targets.
pub const MIN_FUZZ_TARGETS: usize = 5;

/// Known fuzz target names.
pub const FUZZ_TARGETS: &[&str] = &[
    "fuzz_parser",
    "fuzz_snapshot",
    "fuzz_dcs",
    "fuzz_protocol",
    "fuzz_grapheme",
];

/// Minimum TSTs (tests) per section.
pub const MIN_TSTS_PER_SECTION: usize = 5;

/// Supported tmux version ranges for testing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TmuxVersionRange {
    /// LTS: tmux 3.3a - 3.4
    Lts,
    /// Current: tmux 3.5+
    Current,
    /// Preview: tmux HEAD (nightly)
    Preview,
}

impl TmuxVersionRange {
    pub fn label(self) -> &'static str {
        match self {
            Self::Lts => "3.3a-3.4",
            Self::Current => "3.5+",
            Self::Preview => "HEAD",
        }
    }
}

/// Generate a unique socket name for test isolation.
///
/// RULE-S29-06: Every test uses a unique socket name.
/// Uses test name + process ID for uniqueness.
pub fn test_socket_name(test_name: &str) -> String {
    format!(
        "/tmp/termforge-test-{}-{}",
        test_name,
        std::process::id()
    )
}

/// Validate that all sections meet the minimum TST coverage.
///
/// Returns a list of section numbers that are below threshold.
pub fn validate_section_coverage(section_tst_counts: &HashMap<u8, usize>) -> Vec<u8> {
    let mut failures: Vec<u8> = section_tst_counts
        .iter()
        .filter(|(_, &count)| count < MIN_TSTS_PER_SECTION)
        .map(|(&section, _)| section)
        .collect();
    failures.sort();
    failures
}

/// Compare two output byte slices with context on mismatch.
///
/// RULE-S29-05: Differential testing utility.
#[must_use]
pub fn assert_output_equivalent(termforge: &[u8], tmux: &[u8]) -> Result<(), String> {
    if termforge == tmux {
        return Ok(());
    }
    for (i, (a, b)) in termforge.iter().zip(tmux.iter()).enumerate() {
        if a != b {
            return Err(format!(
                "output diverges at byte {i}: termforge=0x{a:02x}, tmux=0x{b:02x}"
            ));
        }
    }
    Err(format!(
        "output length differs: termforge={}, tmux={}",
        termforge.len(),
        tmux.len()
    ))
}

/// Result of a differential test comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffResult {
    pub command: String,
    pub termforge_output: Vec<u8>,
    pub tmux_output: Vec<u8>,
    pub passed: bool,
    pub first_diff_byte: Option<usize>,
}

impl DiffResult {
    /// Compare two outputs for a given command.
    pub fn compare(command: &str, termforge: &[u8], tmux: &[u8]) -> Self {
        let first_diff = termforge.iter().zip(tmux.iter())
            .position(|(a, b)| a != b)
            .or_else(|| {
                if termforge.len() != tmux.len() {
                    Some(termforge.len().min(tmux.len()))
                } else {
                    None
                }
            });
        Self {
            command: command.to_string(),
            termforge_output: termforge.to_vec(),
            tmux_output: tmux.to_vec(),
            passed: first_diff.is_none(),
            first_diff_byte: first_diff,
        }
    }
}

/// Aggregated differential test report.
#[derive(Debug, Clone)]
pub struct DiffReport {
    pub results: Vec<DiffResult>,
}

impl DiffReport {
    pub fn new() -> Self {
        Self { results: Vec::new() }
    }

    pub fn add(&mut self, result: DiffResult) {
        self.results.push(result);
    }

    /// Percentage of tests that passed.
    pub fn pass_rate(&self) -> f64 {
        if self.results.is_empty() {
            return 0.0;
        }
        let passed = self.results.iter().filter(|r| r.passed).count();
        passed as f64 / self.results.len() as f64 * 100.0
    }

    /// Get only the failures.
    pub fn failures(&self) -> Vec<&DiffResult> {
        self.results.iter().filter(|r| !r.passed).collect()
    }
}

impl Default for DiffReport {
    fn default() -> Self {
        Self::new()
    }
}

/// Test cleanup guard: ensures resources are released on drop.
///
/// RULE-S29-08: TestGuard cleans up all resources on drop.
pub struct TestGuard {
    socket_path: String,
    cleanup_fns: Vec<Box<dyn FnOnce()>>,
}

impl TestGuard {
    pub fn new(socket_path: &str) -> Self {
        Self {
            socket_path: socket_path.to_string(),
            cleanup_fns: Vec::new(),
        }
    }

    /// Register a cleanup function to run on drop.
    pub fn on_cleanup(&mut self, f: impl FnOnce() + 'static) {
        self.cleanup_fns.push(Box::new(f));
    }

    /// Get the socket path.
    pub fn socket_path(&self) -> &str {
        &self.socket_path
    }
}

impl Drop for TestGuard {
    fn drop(&mut self) {
        // Run cleanup functions in reverse order
        while let Some(f) = self.cleanup_fns.pop() {
            f();
        }
        // Remove socket file
        let _ = std::fs::remove_file(&self.socket_path);
    }
}

/// Captured grid state for snapshot comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GridSnapshot {
    pub rows: usize,
    pub cols: usize,
    pub cells: Vec<Vec<String>>,
    pub cursor_row: usize,
    pub cursor_col: usize,
}

impl GridSnapshot {
    pub fn new(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            cells: vec![vec![String::new(); cols]; rows],
            cursor_row: 0,
            cursor_col: 0,
        }
    }

    /// Set a cell value.
    pub fn set(&mut self, row: usize, col: usize, value: &str) {
        if row < self.rows && col < self.cols {
            self.cells[row][col] = value.to_string();
        }
    }

    /// Render the grid as plain text.
    pub fn to_text(&self) -> String {
        self.cells.iter()
            .map(|row| {
                let line: String = row.iter()
                    .map(|c| if c.is_empty() { " " } else { c.as_str() })
                    .collect();
                line.trim_end().to_string()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Compare two grid snapshots, returning human-readable diffs.
pub fn snapshot_diff(expected: &GridSnapshot, actual: &GridSnapshot) -> Vec<String> {
    let mut diffs = Vec::new();
    if expected.rows != actual.rows || expected.cols != actual.cols {
        diffs.push(format!(
            "dimension mismatch: expected {}x{}, got {}x{}",
            expected.rows, expected.cols, actual.rows, actual.cols
        ));
        return diffs;
    }
    for row in 0..expected.rows {
        for col in 0..expected.cols {
            if expected.cells[row][col] != actual.cells[row][col] {
                diffs.push(format!(
                    "({},{}) expected {:?}, got {:?}",
                    row, col, expected.cells[row][col], actual.cells[row][col]
                ));
            }
        }
    }
    diffs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzz_targets_count() {
        assert!(FUZZ_TARGETS.len() >= MIN_FUZZ_TARGETS);
    }

    #[test]
    fn test_socket_name_unique() {
        let s1 = test_socket_name("test_a");
        let s2 = test_socket_name("test_b");
        assert_ne!(s1, s2);
        assert!(s1.starts_with("/tmp/termforge-test-"));
    }

    #[test]
    fn test_output_equivalent_identical() {
        assert!(assert_output_equivalent(b"hello", b"hello").is_ok());
    }

    #[test]
    fn test_output_equivalent_different() {
        let result = assert_output_equivalent(b"hello", b"hallo");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("byte 1"));
    }

    #[test]
    fn test_diff_report_pass_rate() {
        let mut report = DiffReport::new();
        report.add(DiffResult::compare("a", b"ok", b"ok"));
        report.add(DiffResult::compare("b", b"ok", b"no"));
        assert!((report.pass_rate() - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_grid_snapshot_to_text() {
        let mut snap = GridSnapshot::new(2, 5);
        snap.set(0, 0, "H");
        snap.set(0, 1, "i");
        let text = snap.to_text();
        assert!(text.starts_with("Hi"));
    }

    #[test]
    fn test_snapshot_diff_identical() {
        let a = GridSnapshot::new(2, 2);
        let b = GridSnapshot::new(2, 2);
        assert!(snapshot_diff(&a, &b).is_empty());
    }
}
