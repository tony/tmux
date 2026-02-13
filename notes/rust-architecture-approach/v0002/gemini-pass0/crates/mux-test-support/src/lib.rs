use std::path::PathBuf;
use uuid::Uuid;

pub struct TestGuard {
    pub socket_path: PathBuf,
}

impl TestGuard {
    pub fn new() -> Self {
        let id = Uuid::new_v4();
        let pid = std::process::id();
        let path = PathBuf::from(format!("/tmp/termforge-test-{}-{}", pid, id));
        Self { socket_path: path }
    }
}

impl Drop for TestGuard {
    fn drop(&mut self) {
        // cleanup socket
        let _ = std::fs::remove_file(&self.socket_path);
    }
}
--- END FILE ---
