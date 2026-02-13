#![forbid(unsafe_code)]

use mux_config::Config;
use mux_effects::{CONTROL_BOUND, DATA_BOUND, DATA_FRAME_BYTES, EFFECT_BOUND, RENDER_BOUND};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalModePolicy {
    ManageRaw,
    AssumeExternalRaw,
    ProbeAndManage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PassthroughPolicy {
    pub graphics_enabled: bool,
    pub allow_dcs: bool,
    pub allow_osc: bool,
}

impl Default for PassthroughPolicy {
    fn default() -> Self {
        Self { graphics_enabled: false, allow_dcs: false, allow_osc: false }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrashFrame {
    pub panic_message: String,
    pub shutdown_steps: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct MuxServer {
    pub config: Config,
    pub terminal_mode_policy: TerminalModePolicy,
    pub passthrough: PassthroughPolicy,
    pub running: bool,
}

#[derive(Debug, Clone)]
pub struct MuxBuilder {
    config: Config,
    terminal_mode_policy: TerminalModePolicy,
    passthrough: PassthroughPolicy,
}

impl MuxBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: Config::default(),
            terminal_mode_policy: TerminalModePolicy::ManageRaw,
            passthrough: PassthroughPolicy::default(),
        }
    }

    #[must_use]
    pub fn with_config(mut self, config: Config) -> Self {
        self.config = config;
        self
    }

    #[must_use]
    pub fn terminal_mode_policy(mut self, p: TerminalModePolicy) -> Self {
        self.terminal_mode_policy = p;
        self
    }

    #[must_use]
    pub fn passthrough(mut self, p: PassthroughPolicy) -> Self {
        self.passthrough = p;
        self
    }

    #[must_use]
    pub fn build(self) -> MuxServer {
        MuxServer {
            config: self.config,
            terminal_mode_policy: self.terminal_mode_policy,
            passthrough: self.passthrough,
            running: false,
        }
    }
}

impl Default for MuxBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl MuxServer {
    pub fn start(&mut self) {
        self.running = true;
    }

    pub fn stop(&mut self) {
        self.running = false;
    }

    #[must_use]
    pub fn panic_recovery_frame(msg: &str) -> CrashFrame {
        CrashFrame {
            panic_message: msg.to_string(),
            shutdown_steps: vec![
                "freeze_accept_loop",
                "flush_control_queue",
                "broadcast_crash_frame",
                "close_ptys",
                "persist_snapshot",
                "terminate",
            ],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelSpec {
    pub control: usize,
    pub data: usize,
    pub data_frame_bytes: usize,
    pub render: usize,
    pub effect: usize,
}

pub const CHANNEL_SPEC: ChannelSpec = ChannelSpec {
    control: CONTROL_BOUND,
    data: DATA_BOUND,
    data_frame_bytes: DATA_FRAME_BYTES,
    render: RENDER_BOUND,
    effect: EFFECT_BOUND,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_defaults_to_manage_raw() {
        let server = MuxBuilder::new().build();
        assert_eq!(server.terminal_mode_policy, TerminalModePolicy::ManageRaw);
    }

    #[test]
    fn builder_supports_policy_override() {
        let server = MuxBuilder::new()
            .terminal_mode_policy(TerminalModePolicy::ProbeAndManage)
            .build();
        assert_eq!(server.terminal_mode_policy, TerminalModePolicy::ProbeAndManage);
    }

    #[test]
    fn passthrough_is_disabled_by_default() {
        let server = MuxBuilder::new().build();
        assert!(!server.passthrough.graphics_enabled);
    }

    #[test]
    fn passthrough_can_enable_dcs_and_osc() {
        let p = PassthroughPolicy { graphics_enabled: true, allow_dcs: true, allow_osc: true };
        let server = MuxBuilder::new().passthrough(p).build();
        assert!(server.passthrough.allow_dcs && server.passthrough.allow_osc);
    }

    #[test]
    fn server_lifecycle_start_stop() {
        let mut s = MuxBuilder::new().build();
        s.start();
        assert!(s.running);
        s.stop();
        assert!(!s.running);
    }

    #[test]
    fn panic_frame_has_ordered_steps() {
        let frame = MuxServer::panic_recovery_frame("boom");
        assert_eq!(frame.shutdown_steps.first().copied(), Some("freeze_accept_loop"));
        assert_eq!(frame.shutdown_steps.last().copied(), Some("terminate"));
    }

    #[test]
    fn channel_spec_matches_required_bounds() {
        assert_eq!(CHANNEL_SPEC.control, 512);
        assert_eq!(CHANNEL_SPEC.data, 1024);
        assert_eq!(CHANNEL_SPEC.data_frame_bytes, 64 * 1024);
        assert_eq!(CHANNEL_SPEC.render, 256);
        assert_eq!(CHANNEL_SPEC.effect, 1024);
    }

    #[test]
    fn builder_accepts_custom_config() {
        let mut c = Config::default();
        c.server.max_clients = 99;
        let s = MuxBuilder::new().with_config(c.clone()).build();
        assert_eq!(s.config, c);
    }
}
