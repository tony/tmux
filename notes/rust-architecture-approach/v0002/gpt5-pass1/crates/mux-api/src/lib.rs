//! # mux-api
//!
//! Ergonomic SDK entry point for embedding TermForge.

#![forbid(unsafe_code)]

use crossbeam_channel::{bounded, Receiver, Sender};
use serde::{Deserialize, Serialize};

use mux_kernel::{Kernel, KernelEffect, KernelEvent};
use mux_render::{Compositor, FramePolicy};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerminalModePolicy {
    /// SDK transitions stdin/stdout into raw mode and restores on drop.
    ManageRaw,
    /// Caller already owns raw mode; SDK must not touch termios.
    AssumeExternalRaw,
    /// SDK probes and only enters raw mode when needed.
    ProbeAndManage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerminalModeState {
    Canonical,
    RawOwned,
    RawExternal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaneCapacity {
    pub control: usize,
    pub data_frames: usize,
    pub data_frame_bytes: usize,
    pub render: usize,
}

impl Default for LaneCapacity {
    fn default() -> Self {
        Self {
            control: 512,
            data_frames: 1024,
            data_frame_bytes: 64 * 1024,
            render: 256,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigHotReload {
    pub enabled: bool,
    pub path: String,
}

impl Default for ConfigHotReload {
    fn default() -> Self {
        Self {
            enabled: true,
            path: "~/.config/termforge/config.toml".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TermForgeConfig {
    pub terminal_mode: TerminalModePolicy,
    pub lane_capacity: LaneCapacity,
    pub hot_reload: ConfigHotReload,
    pub enable_crash_frame: bool,
}

impl Default for TermForgeConfig {
    fn default() -> Self {
        Self {
            terminal_mode: TerminalModePolicy::ProbeAndManage,
            lane_capacity: LaneCapacity::default(),
            hot_reload: ConfigHotReload::default(),
            enable_crash_frame: true,
        }
    }
}

#[derive(Debug)]
pub struct TermForgeBuilder {
    config: TermForgeConfig,
    rows: usize,
    cols: usize,
    frame_policy: FramePolicy,
}

impl TermForgeBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: TermForgeConfig::default(),
            rows: 24,
            cols: 80,
            frame_policy: FramePolicy::default(),
        }
    }

    #[must_use]
    pub fn terminal_mode(mut self, policy: TerminalModePolicy) -> Self {
        self.config.terminal_mode = policy;
        self
    }

    #[must_use]
    pub fn dimensions(mut self, rows: usize, cols: usize) -> Self {
        self.rows = rows.max(1);
        self.cols = cols.max(1);
        self
    }

    #[must_use]
    pub fn lane_capacity(mut self, lane_capacity: LaneCapacity) -> Self {
        self.config.lane_capacity = lane_capacity;
        self
    }

    #[must_use]
    pub fn frame_policy(mut self, frame_policy: FramePolicy) -> Self {
        self.frame_policy = frame_policy;
        self
    }

    #[must_use]
    pub fn from_json(mut self, raw: &str) -> Result<Self, String> {
        let cfg = serde_json::from_str::<TermForgeConfig>(raw)
            .map_err(|err| format!("invalid config json: {err}"))?;
        self.config = cfg;
        Ok(self)
    }

    #[must_use]
    pub fn build(self) -> RuntimeHandle {
        let (event_tx, event_rx) = bounded(self.config.lane_capacity.control);
        let (effect_tx, effect_rx) = bounded(self.config.lane_capacity.control);
        RuntimeHandle {
            config: self.config,
            mode_state: TerminalModeState::Canonical,
            kernel: Kernel::new(),
            compositor: Compositor::new(self.rows, self.cols, self.frame_policy),
            event_tx,
            event_rx,
            effect_tx,
            effect_rx,
        }
    }
}

impl Default for TermForgeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct RuntimeHandle {
    config: TermForgeConfig,
    mode_state: TerminalModeState,
    kernel: Kernel,
    compositor: Compositor,
    event_tx: Sender<KernelEvent>,
    event_rx: Receiver<KernelEvent>,
    effect_tx: Sender<KernelEffect>,
    effect_rx: Receiver<KernelEffect>,
}

impl RuntimeHandle {
    #[must_use]
    pub fn config(&self) -> &TermForgeConfig {
        &self.config
    }

    #[must_use]
    pub fn mode_state(&self) -> TerminalModeState {
        self.mode_state
    }

    #[must_use]
    pub fn compositor(&self) -> &Compositor {
        &self.compositor
    }

    pub fn enter_terminal_mode(&mut self, external_is_raw: bool) {
        self.mode_state = match self.config.terminal_mode {
            TerminalModePolicy::ManageRaw => TerminalModeState::RawOwned,
            TerminalModePolicy::AssumeExternalRaw => TerminalModeState::RawExternal,
            TerminalModePolicy::ProbeAndManage => {
                if external_is_raw {
                    TerminalModeState::RawExternal
                } else {
                    TerminalModeState::RawOwned
                }
            }
        };
    }

    pub fn leave_terminal_mode(&mut self) {
        self.mode_state = TerminalModeState::Canonical;
    }

    #[must_use]
    pub fn event_sender(&self) -> Sender<KernelEvent> {
        self.event_tx.clone()
    }

    #[must_use]
    pub fn effect_receiver(&self) -> Receiver<KernelEffect> {
        self.effect_rx.clone()
    }

    pub fn poll_once(&mut self) -> usize {
        let mut produced = 0usize;
        while let Ok(event) = self.event_rx.try_recv() {
            let effects = self.kernel.step(event);
            for effect in effects {
                if self.effect_tx.try_send(effect).is_ok() {
                    produced = produced.saturating_add(1);
                }
            }
        }
        produced
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingBoundary {
    Blocking,
    Callback,
    AsyncPoll,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PollToken(pub u64);

#[derive(Debug)]
pub struct BindingBridge {
    pub boundary: BindingBoundary,
    next_token: u64,
}

impl BindingBridge {
    #[must_use]
    pub fn new(boundary: BindingBoundary) -> Self {
        Self {
            boundary,
            next_token: 1,
        }
    }

    #[must_use]
    pub fn issue_token(&mut self) -> PollToken {
        let token = PollToken(self.next_token);
        self.next_token = self.next_token.saturating_add(1);
        token
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_default_capacities_match_architecture() {
        let runtime = TermForgeBuilder::new().build();
        let lane = runtime.config().lane_capacity;
        assert_eq!(lane.control, 512);
        assert_eq!(lane.data_frames, 1024);
        assert_eq!(lane.data_frame_bytes, 64 * 1024);
        assert_eq!(lane.render, 256);
    }

    #[test]
    fn terminal_mode_policy_probe_uses_external_raw_when_available() {
        let mut runtime = TermForgeBuilder::new()
            .terminal_mode(TerminalModePolicy::ProbeAndManage)
            .build();

        runtime.enter_terminal_mode(true);
        assert_eq!(runtime.mode_state(), TerminalModeState::RawExternal);

        runtime.leave_terminal_mode();
        assert_eq!(runtime.mode_state(), TerminalModeState::Canonical);
    }

    #[test]
    fn runtime_polls_kernel_events() {
        let mut runtime = TermForgeBuilder::new().build();
        let tx = runtime.event_sender();
        let sent = tx.send(KernelEvent::Tick);
        assert!(sent.is_ok());

        let produced = runtime.poll_once();
        assert!(produced >= 1);

        let rx = runtime.effect_receiver();
        let effect = rx.try_recv();
        assert!(effect.is_ok());
    }

    #[test]
    fn config_round_trip_json() {
        let raw = r#"{
            "terminal_mode":"ManageRaw",
            "lane_capacity":{"control":512,"data_frames":1024,"data_frame_bytes":65536,"render":256},
            "hot_reload":{"enabled":true,"path":"/tmp/tf.toml"},
            "enable_crash_frame":true
        }"#;
        let builder = TermForgeBuilder::new().from_json(raw);
        assert!(builder.is_ok());
    }

    #[test]
    fn binding_bridge_issues_monotonic_tokens() {
        let mut bridge = BindingBridge::new(BindingBoundary::AsyncPoll);
        let a = bridge.issue_token();
        let b = bridge.issue_token();
        assert!(b.0 > a.0);
    }
}
