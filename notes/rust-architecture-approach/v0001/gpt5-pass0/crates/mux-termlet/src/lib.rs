#![forbid(unsafe_code)]

use mux_pty::{DynPtyHandle, InnerPtyState};
use mux_time::DeterministicTimeSource;
use mux_types::TermletError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceQuota {
    pub max_cpu_millis: u64,
    pub max_memory_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SandboxPolicy {
    pub allow_network: bool,
    pub allow_fs_write: bool,
}

#[derive(Debug, Clone)]
pub struct TermletRuntime {
    quota: ResourceQuota,
    sandbox: SandboxPolicy,
    used_cpu_millis: u64,
    used_memory_bytes: u64,
    time: DeterministicTimeSource,
    recorder: Vec<String>,
}

impl TermletRuntime {
    pub fn new(quota: ResourceQuota, sandbox: SandboxPolicy, time: DeterministicTimeSource) -> Self {
        Self {
            quota,
            sandbox,
            used_cpu_millis: 0,
            used_memory_bytes: 0,
            time,
            recorder: Vec::new(),
        }
    }

    pub fn sandbox(&self) -> SandboxPolicy {
        self.sandbox
    }

    pub fn time(&self) -> &DeterministicTimeSource {
        &self.time
    }

    pub fn recorder(&self) -> &[String] {
        &self.recorder
    }

    pub fn admit(&mut self, cpu_millis: u64, memory_bytes: u64) -> Result<(), TermletError> {
        let cpu_next = self.used_cpu_millis.saturating_add(cpu_millis);
        let mem_next = self.used_memory_bytes.saturating_add(memory_bytes);
        if cpu_next > self.quota.max_cpu_millis || mem_next > self.quota.max_memory_bytes {
            return Err(TermletError::ResourceQuotaExceeded);
        }
        self.used_cpu_millis = cpu_next;
        self.used_memory_bytes = mem_next;
        Ok(())
    }

    pub fn ensure_sandbox_action(&self, wants_network: bool, wants_fs_write: bool) -> Result<(), TermletError> {
        if wants_network && !self.sandbox.allow_network {
            return Err(TermletError::SandboxViolation);
        }
        if wants_fs_write && !self.sandbox.allow_fs_write {
            return Err(TermletError::SandboxViolation);
        }
        Ok(())
    }

    pub fn run_step(&mut self, pty: &DynPtyHandle, node: &str, event: &str) -> Result<(), TermletError> {
        if pty.state() != InnerPtyState::Running {
            return Err(TermletError::InvalidState);
        }
        self.time.tick_wall(1);
        self.time.event(node);
        self.recorder.push(format!(
            "t={} lamport={} event={event}",
            self.time.now_wall_millis(),
            self.time.lamport()
        ));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mux_pty::{Allocated, PtyHandle};

    fn running_dyn_handle() -> DynPtyHandle {
        PtyHandle::<Allocated>::new(1)
            .spawn(123)
            .unwrap()
            .start()
            .into_dyn()
    }

    #[test]
    fn quota_enforced() {
        let mut rt = TermletRuntime::new(
            ResourceQuota {
                max_cpu_millis: 10,
                max_memory_bytes: 100,
            },
            SandboxPolicy {
                allow_network: false,
                allow_fs_write: false,
            },
            DeterministicTimeSource::new(0),
        );
        assert!(rt.admit(5, 50).is_ok());
        assert!(matches!(rt.admit(6, 1), Err(TermletError::ResourceQuotaExceeded)));
    }

    #[test]
    fn sandbox_enforced() {
        let rt = TermletRuntime::new(
            ResourceQuota {
                max_cpu_millis: 10,
                max_memory_bytes: 100,
            },
            SandboxPolicy {
                allow_network: false,
                allow_fs_write: true,
            },
            DeterministicTimeSource::new(0),
        );
        assert!(matches!(
            rt.ensure_sandbox_action(true, false),
            Err(TermletError::SandboxViolation)
        ));
        assert!(rt.ensure_sandbox_action(false, true).is_ok());
    }

    #[test]
    fn run_step_updates_time_and_recorder() {
        let mut rt = TermletRuntime::new(
            ResourceQuota {
                max_cpu_millis: 100,
                max_memory_bytes: 1000,
            },
            SandboxPolicy {
                allow_network: false,
                allow_fs_write: false,
            },
            DeterministicTimeSource::new(10),
        );
        let pty = running_dyn_handle();
        rt.run_step(&pty, "node-a", "tick").unwrap();
        assert_eq!(rt.time().now_wall_millis(), 11);
        assert_eq!(rt.time().lamport(), 1);
        assert_eq!(rt.recorder().len(), 1);
    }

    #[test]
    fn run_step_requires_running_state() {
        let mut rt = TermletRuntime::new(
            ResourceQuota {
                max_cpu_millis: 100,
                max_memory_bytes: 1000,
            },
            SandboxPolicy {
                allow_network: false,
                allow_fs_write: false,
            },
            DeterministicTimeSource::new(10),
        );
        let pty = PtyHandle::<Allocated>::new(1).into_dyn();
        assert!(matches!(rt.run_step(&pty, "node-a", "tick"), Err(TermletError::InvalidState)));
    }
}
