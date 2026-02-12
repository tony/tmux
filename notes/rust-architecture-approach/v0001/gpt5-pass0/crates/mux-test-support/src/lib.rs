#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Isolated socket path helper for tests.
#[derive(Debug)]
pub struct IsolatedSocketPath {
    dir: PathBuf,
    socket: PathBuf,
}

impl IsolatedSocketPath {
    pub fn new() -> std::io::Result<Self> {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let base = std::env::temp_dir().join(format!("termforge-{}-{n}", std::process::id()));
        fs::create_dir_all(&base)?;
        let socket = base.join("mux.sock");
        Ok(Self { dir: base, socket })
    }

    pub fn socket_path(&self) -> &Path {
        &self.socket
    }

    pub fn root_dir(&self) -> &Path {
        &self.dir
    }
}

impl Drop for IsolatedSocketPath {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.socket);
        let _ = fs::remove_dir_all(&self.dir);
    }
}

pub fn assert_eventually<F>(mut f: F, attempts: usize)
where
    F: FnMut() -> bool,
{
    for _ in 0..attempts {
        if f() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    panic!("condition did not become true in {attempts} attempts");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_isolated_directory() {
        let p = IsolatedSocketPath::new().unwrap();
        assert!(p.root_dir().exists());
        assert!(p.socket_path().ends_with("mux.sock"));
    }

    #[test]
    fn eventually_helper_succeeds() {
        let mut n = 0;
        assert_eventually(
            || {
                n += 1;
                n > 2
            },
            10,
        );
    }

    #[test]
    #[should_panic(expected = "condition did not become true")]
    fn eventually_helper_panics() {
        assert_eventually(|| false, 2);
    }
}
