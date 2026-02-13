//! # mux-config
//!
//! TOML configuration system for TermForge.
//!
//! ## File hierarchy (highest to lowest priority)
//! 1. Programmatic: `MuxServer::builder().history_limit(50_000)`
//! 2. User config: `$XDG_CONFIG_HOME/termforge/config.toml`
//! 3. System config: `/etc/termforge/config.toml`
//! 4. tmux compat: `~/.tmux.conf` (parsed via mux-cmd-parse)
//!
//! L4 integration crate.

#![forbid(unsafe_code)]

use mux_options::{OptionTable, OptionValue};
use serde::Deserialize;
use thiserror::Error;

/// Configuration errors.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// TOML parse error.
    #[error("toml parse error: {0}")]
    TomlParse(String),
    /// Invalid value.
    #[error("invalid config value: {key} = {value}")]
    InvalidValue { key: String, value: String },
    /// I/O error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Top-level TOML config structure.
#[derive(Debug, Deserialize, Default)]
pub struct Config {
    /// Server-level options.
    #[serde(default)]
    pub server: ServerConfig,
    /// Session-level defaults.
    #[serde(default)]
    pub session: SessionConfig,
}

/// Server configuration section.
#[derive(Debug, Deserialize, Default)]
pub struct ServerConfig {
    pub socket_path: Option<String>,
    pub default_shell: Option<String>,
    pub history_limit: Option<i64>,
    pub escape_time: Option<i64>,
    pub focus_events: Option<bool>,
    pub allow_passthrough: Option<bool>,
    pub default_terminal: Option<String>,
    pub mouse: Option<bool>,
}

/// Session configuration section.
#[derive(Debug, Deserialize, Default)]
pub struct SessionConfig {
    pub base_index: Option<i64>,
    pub renumber_windows: Option<bool>,
    pub status: Option<bool>,
    pub status_position: Option<String>,
}

impl Config {
    /// Parse a TOML config string.
    ///
    /// # Errors
    ///
    /// Returns `ConfigError::TomlParse` for invalid TOML.
    pub fn from_toml(input: &str) -> Result<Self, ConfigError> {
        toml::from_str(input).map_err(|e| ConfigError::TomlParse(e.to_string()))
    }

    /// Convert config into an OptionTable.
    #[must_use]
    pub fn to_option_table(&self) -> OptionTable {
        let mut table = OptionTable::new();

        if let Some(v) = self.server.history_limit {
            table.set("history-limit", OptionValue::Int(v));
        }
        if let Some(v) = self.server.escape_time {
            table.set("escape-time", OptionValue::Int(v));
        }
        if let Some(v) = self.server.focus_events {
            table.set("focus-events", OptionValue::Bool(v));
        }
        if let Some(v) = self.server.mouse {
            table.set("mouse", OptionValue::Bool(v));
        }
        if let Some(v) = self.server.allow_passthrough {
            table.set("allow-passthrough", OptionValue::Bool(v));
        }
        if let Some(ref v) = self.server.default_terminal {
            table.set("default-terminal", OptionValue::String(v.clone()));
        }
        if let Some(v) = self.session.base_index {
            table.set("base-index", OptionValue::Int(v));
        }
        if let Some(v) = self.session.status {
            table.set("status", OptionValue::Bool(v));
        }

        table
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_config() {
        let toml = r#"
[server]
history_limit = 50000
mouse = true
"#;
        let config = Config::from_toml(toml);
        assert!(config.is_ok());
        let config = config.unwrap_or_default();
        assert_eq!(config.server.history_limit, Some(50000));
        assert_eq!(config.server.mouse, Some(true));
    }

    #[test]
    fn parse_empty_config() {
        let config = Config::from_toml("");
        assert!(config.is_ok());
    }

    #[test]
    fn parse_full_config() {
        let toml = r#"
[server]
socket_path = "/tmp/termforge.sock"
default_shell = "/bin/zsh"
history_limit = 50000
escape_time = 100
focus_events = true
allow_passthrough = false
mouse = true

[session]
base_index = 1
renumber_windows = true
status = true
status_position = "top"
"#;
        let config = Config::from_toml(toml);
        assert!(config.is_ok());
    }

    #[test]
    fn to_option_table() {
        let toml = r#"
[server]
history_limit = 10000
mouse = true
allow_passthrough = false
"#;
        let config = Config::from_toml(toml).unwrap_or_default();
        let table = config.to_option_table();
        assert_eq!(table.get("history-limit"), Some(&OptionValue::Int(10000)));
        assert_eq!(table.get("mouse"), Some(&OptionValue::Bool(true)));
        assert_eq!(
            table.get("allow-passthrough"),
            Some(&OptionValue::Bool(false))
        );
    }

    #[test]
    fn invalid_toml() {
        let result = Config::from_toml("not valid [toml");
        assert!(result.is_err());
    }

    #[test]
    fn config_default() {
        let config = Config::default();
        assert!(config.server.history_limit.is_none());
    }
}
