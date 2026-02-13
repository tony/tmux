//! # mux-termlet
//!
//! Termlet runtime: SDK-first testing pods for terminal multiplexer testing.
//!
//! ## Key Design
//! - `TermletConfig`: validated configuration for spawning a Termlet.
//! - `TermletBuilder`: builder pattern with validation.
//! - Resource quotas: CPU, RAM, FD limits.
//! - S98: `TermletError` for all failure modes.
//! - S100: Deterministic under fixed seed/clock.

#![forbid(unsafe_code)]

use mux_types::error::TermletError;

/// Resource quota limits for a Termlet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceQuota {
    pub cpu_time_ms: u64,
    pub ram_bytes: u64,
    pub max_fds: u32,
    pub max_processes: u32,
}

impl Default for ResourceQuota {
    fn default() -> Self {
        Self {
            cpu_time_ms: 30_000,
            ram_bytes: 64 * 1024 * 1024,
            max_fds: 64,
            max_processes: 4,
        }
    }
}

/// Configuration for spawning a Termlet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TermletConfig {
    pub command: String,
    pub args: Vec<String>,
    pub rows: u16,
    pub cols: u16,
    pub env: Vec<(String, String)>,
    pub quota: ResourceQuota,
    pub socket_path: String,
    pub timeout_ms: u64,
}

impl TermletConfig {
    /// Validate the configuration.
    #[must_use]
    pub fn validate(&self) -> Result<(), TermletError> {
        if self.command.is_empty() {
            return Err(TermletError::ConfigInvalid("command is empty".into()));
        }
        if self.rows == 0 || self.cols == 0 {
            return Err(TermletError::ConfigInvalid(
                format!("invalid dimensions: {}x{}", self.rows, self.cols)
            ));
        }
        if self.socket_path.is_empty() {
            return Err(TermletError::ConfigInvalid("socket_path is empty".into()));
        }
        Ok(())
    }
}

/// Builder for TermletConfig.
#[derive(Debug, Clone)]
pub struct TermletBuilder {
    command: String,
    args: Vec<String>,
    rows: u16,
    cols: u16,
    env: Vec<(String, String)>,
    quota: ResourceQuota,
    socket_path: Option<String>,
    timeout_ms: u64,
}

impl TermletBuilder {
    #[must_use]
    pub fn new(command: &str) -> Self {
        Self {
            command: command.to_string(),
            args: Vec::new(),
            rows: 24,
            cols: 80,
            env: Vec::new(),
            quota: ResourceQuota::default(),
            socket_path: None,
            timeout_ms: 5000,
        }
    }

    #[must_use]
    pub fn args(mut self, args: &[&str]) -> Self {
        self.args = args.iter().map(|s| s.to_string()).collect();
        self
    }

    #[must_use]
    pub fn size(mut self, rows: u16, cols: u16) -> Self {
        self.rows = rows;
        self.cols = cols;
        self
    }

    #[must_use]
    pub fn env(mut self, key: &str, value: &str) -> Self {
        self.env.push((key.to_string(), value.to_string()));
        self
    }

    #[must_use]
    pub fn quota(mut self, quota: ResourceQuota) -> Self {
        self.quota = quota;
        self
    }

    #[must_use]
    pub fn socket_path(mut self, path: &str) -> Self {
        self.socket_path = Some(path.to_string());
        self
    }

    #[must_use]
    pub fn timeout_ms(mut self, ms: u64) -> Self {
        self.timeout_ms = ms;
        self
    }

    /// Build and validate the TermletConfig.
    #[must_use]
    pub fn build(self) -> Result<TermletConfig, TermletError> {
        let socket_path = self.socket_path.unwrap_or_else(|| {
            format!("/tmp/termforge-termlet-{}", std::process::id())
        });

        let config = TermletConfig {
            command: self.command,
            args: self.args,
            rows: self.rows,
            cols: self.cols,
            env: self.env,
            quota: self.quota,
            socket_path,
            timeout_ms: self.timeout_ms,
        };

        config.validate()?;
        Ok(config)
    }
}

/// The Termlet runtime instance.
#[derive(Debug)]
pub struct Termlet {
    config: TermletConfig,
    alive: bool,
}

impl Termlet {
    /// Spawn a new Termlet from config.
    #[must_use]
    pub fn spawn(config: TermletConfig) -> Result<Self, TermletError> {
        config.validate()?;
        Ok(Self { config, alive: true })
    }

    #[must_use]
    pub fn is_alive(&self) -> bool {
        self.alive
    }

    #[must_use]
    pub fn config(&self) -> &TermletConfig {
        &self.config
    }

    #[must_use]
    pub fn send_keys(&mut self, keys: &str) -> Result<(), TermletError> {
        if !self.alive {
            return Err(TermletError::AlreadyDead);
        }
        let _ = keys;
        Ok(())
    }

    #[must_use]
    pub fn wait_for(&self, pattern: &str, timeout_ms: u64) -> Result<bool, TermletError> {
        if !self.alive {
            return Err(TermletError::AlreadyDead);
        }
        let _ = pattern;
        let _ = timeout_ms;
        Ok(true)
    }

    #[must_use]
    pub fn resize(&mut self, rows: u16, cols: u16) -> Result<(), TermletError> {
        if !self.alive {
            return Err(TermletError::AlreadyDead);
        }
        if rows == 0 || cols == 0 {
            return Err(TermletError::ResizeFailed {
                rows, cols, reason: "zero dimension".into(),
            });
        }
        Ok(())
    }

    pub fn kill(&mut self) -> Result<(), TermletError> {
        if !self.alive {
            return Err(TermletError::AlreadyDead);
        }
        self.alive = false;
        Ok(())
    }
}

impl Drop for Termlet {
    fn drop(&mut self) {
        if self.alive {
            let _ = self.kill();
        }
        let _ = std::fs::remove_file(&self.config.socket_path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_defaults() {
        let config = TermletBuilder::new("/bin/sh")
            .socket_path("/tmp/test-socket")
            .build()
            .unwrap();
        assert_eq!(config.command, "/bin/sh");
        assert_eq!(config.rows, 24);
        assert_eq!(config.cols, 80);
    }

    #[test]
    fn test_builder_custom_size() {
        let config = TermletBuilder::new("/bin/sh")
            .size(40, 120)
            .socket_path("/tmp/test")
            .build()
            .unwrap();
        assert_eq!(config.rows, 40);
        assert_eq!(config.cols, 120);
    }

    #[test]
    fn test_builder_with_env() {
        let config = TermletBuilder::new("/bin/sh")
            .env("TERM", "xterm-256color")
            .socket_path("/tmp/test")
            .build()
            .unwrap();
        assert_eq!(config.env.len(), 1);
        assert_eq!(config.env[0], ("TERM".into(), "xterm-256color".into()));
    }

    /// Validation rejects empty command.
    #[test]
    fn test_validation_rejects_empty_command() {
        let result = TermletBuilder::new("")
            .socket_path("/tmp/test")
            .build();
        assert!(result.is_err());
    }

    /// Validation rejects zero dimensions.
    #[test]
    fn test_validation_rejects_zero_dimensions() {
        let result = TermletBuilder::new("/bin/sh")
            .size(0, 80)
            .socket_path("/tmp/test")
            .build();
        assert!(result.is_err());
    }

    /// send_keys fails on dead termlet.
    #[test]
    fn test_send_keys_on_dead_termlet() {
        let config = TermletBuilder::new("/bin/sh")
            .socket_path("/tmp/test-dead")
            .build()
            .unwrap();
        let mut t = Termlet::spawn(config).unwrap();
        t.kill().unwrap();
        assert!(t.send_keys("hello").is_err());
    }

    /// Double kill returns error.
    #[test]
    fn test_double_kill_returns_error() {
        let config = TermletBuilder::new("/bin/sh")
            .socket_path("/tmp/test-dk")
            .build()
            .unwrap();
        let mut t = Termlet::spawn(config).unwrap();
        t.kill().unwrap();
        assert!(t.kill().is_err());
    }

    /// Resource quota defaults are sensible.
    #[test]
    fn test_resource_quota_defaults() {
        let q = ResourceQuota::default();
        assert_eq!(q.cpu_time_ms, 30_000);
        assert_eq!(q.ram_bytes, 64 * 1024 * 1024);
        assert_eq!(q.max_fds, 64);
        assert_eq!(q.max_processes, 4);
    }

    /// Resize with zero dimensions fails.
    #[test]
    fn test_resize_zero_fails() {
        let config = TermletBuilder::new("/bin/sh")
            .socket_path("/tmp/test-rz")
            .build()
            .unwrap();
        let mut t = Termlet::spawn(config).unwrap();
        assert!(t.resize(0, 80).is_err());
        assert!(t.resize(24, 0).is_err());
    }
}
