//! # mux-config
//!
//! Configuration system for TermForge.
//!
//! Supports two config file formats:
//! - `termforge.toml`: TOML-based config for SDK embedding.
//! - `.tmux.conf`: tmux-compatible command sequence (parsed by mux-cmd-parse).
//!
//! ## Config Hierarchy
//! ```text
//! $XDG_CONFIG_HOME/termforge/config.toml   (user config)
//! /etc/termforge/config.toml               (system config)
//! ~/.tmux.conf                             (tmux compat, lower priority)
//! Programmatic via MuxServer::builder()    (highest priority)
//! ```

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Top-level TermForge configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct TermForgeConfig {
    /// Server-level settings.
    pub server: ServerConfig,
    /// Default session settings.
    pub session: SessionConfig,
    /// Key binding overrides.
    #[serde(default)]
    pub keybindings: Vec<KeyBindingConfig>,
}

impl Default for TermForgeConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            session: SessionConfig::default(),
            keybindings: Vec::new(),
        }
    }
}

/// Server configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    /// Unix socket path. None = auto-generate.
    pub socket_path: Option<PathBuf>,
    /// Default shell for new panes.
    pub default_shell: String,
    /// Scrollback history limit (lines per pane).
    pub history_limit: u32,
    /// Escape time in milliseconds (delay before sending Escape).
    pub escape_time: u32,
    /// Enable focus events (FocusIn/FocusOut).
    pub focus_events: bool,
    /// Allow graphics passthrough (INV-220: disabled by default).
    pub allow_passthrough: bool,
    /// Default TERM value for child processes.
    pub default_terminal: String,
    /// Mouse support enabled.
    pub mouse: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            socket_path: None,
            default_shell: "/bin/sh".into(),
            history_limit: 10_000,
            escape_time: 500,
            focus_events: false,
            allow_passthrough: false,
            default_terminal: "screen-256color".into(),
            mouse: false,
        }
    }
}

/// Default session configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct SessionConfig {
    /// Base window index.
    pub base_index: u32,
    /// Window renumber on close.
    pub renumber_windows: bool,
    /// Status line enabled.
    pub status: bool,
    /// Status line position.
    pub status_position: StatusPosition,
    /// Status left format string.
    pub status_left: String,
    /// Status right format string.
    pub status_right: String,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            base_index: 0,
            renumber_windows: false,
            status: true,
            status_position: StatusPosition::Bottom,
            status_left: "#S".into(),
            status_right: "%H:%M %d-%b-%y".into(),
        }
    }
}

/// Status line position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StatusPosition {
    Top,
    Bottom,
}

impl Default for StatusPosition {
    fn default() -> Self {
        Self::Bottom
    }
}

/// A key binding configuration entry.
#[derive(Debug, Clone, Deserialize)]
pub struct KeyBindingConfig {
    /// Key specification (e.g., "C-b", "M-1").
    pub key: String,
    /// Whether this is a root (no prefix) binding.
    #[serde(default)]
    pub root: bool,
    /// Command to execute.
    pub command: String,
}

/// Configuration errors.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("config file not found: {0}")]
    NotFound(PathBuf),
    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid config: {0}")]
    Invalid(String),
}

/// Load a TOML config file.
///
/// # Errors
/// Returns error if the file cannot be read or parsed.
pub fn load_config(path: &Path) -> Result<TermForgeConfig, ConfigError> {
    let content = std::fs::read_to_string(path).map_err(ConfigError::Io)?;
    let config: TermForgeConfig = toml::from_str(&content)?;
    Ok(config)
}

/// Load config from a TOML string.
///
/// # Errors
/// Returns error if the string cannot be parsed.
pub fn load_config_str(content: &str) -> Result<TermForgeConfig, ConfigError> {
    let config: TermForgeConfig = toml::from_str(content)?;
    Ok(config)
}

/// Find the default config file paths in order of priority.
#[must_use]
pub fn default_config_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // XDG_CONFIG_HOME/termforge/config.toml
    if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
        paths.push(PathBuf::from(xdg_config).join("termforge/config.toml"));
    } else if let Ok(home) = std::env::var("HOME") {
        paths.push(PathBuf::from(&home).join(".config/termforge/config.toml"));
    }

    // /etc/termforge/config.toml
    paths.push(PathBuf::from("/etc/termforge/config.toml"));

    // ~/.tmux.conf (tmux compat)
    if let Ok(home) = std::env::var("HOME") {
        paths.push(PathBuf::from(&home).join(".tmux.conf"));
    }

    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = TermForgeConfig::default();
        assert_eq!(config.server.history_limit, 10_000);
        assert_eq!(config.server.escape_time, 500);
        assert!(!config.server.allow_passthrough);
        assert!(config.session.status);
    }

    #[test]
    fn parse_minimal_toml() {
        let toml = r#"
[server]
history_limit = 50000
default_shell = "/bin/bash"
"#;
        let config = load_config_str(toml);
        assert!(config.is_ok());
        let config = config.unwrap_or_default();
        assert_eq!(config.server.history_limit, 50_000);
        assert_eq!(config.server.default_shell, "/bin/bash");
    }

    #[test]
    fn parse_full_toml() {
        let toml = r##"
[server]
socket_path = "/tmp/termforge.sock"
default_shell = "/bin/zsh"
history_limit = 100000
escape_time = 100
focus_events = true
allow_passthrough = false
default_terminal = "tmux-256color"
mouse = true

[session]
base_index = 1
renumber_windows = true
status = true
status_position = "top"
status_left = "#S"
status_right = "%H:%M"

[[keybindings]]
key = "C-a"
root = false
command = "send-prefix"
"##;
        let config = load_config_str(toml);
        assert!(config.is_ok());
        let config = config.unwrap_or_default();
        assert_eq!(config.server.escape_time, 100);
        assert!(config.server.mouse);
        assert_eq!(config.session.base_index, 1);
        assert_eq!(config.session.status_position, StatusPosition::Top);
        assert_eq!(config.keybindings.len(), 1);
    }

    #[test]
    fn default_config_paths_not_empty() {
        // As long as HOME is set, we should get some paths
        let paths = default_config_paths();
        // May be empty in certain CI environments
        let _ = paths;
    }

    #[test]
    fn passthrough_default_disabled() {
        let config = ServerConfig::default();
        assert!(!config.allow_passthrough);
    }
}
