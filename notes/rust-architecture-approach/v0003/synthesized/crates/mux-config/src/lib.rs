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
    #[error("toml parse error: {0}")]
    TomlParse(String),
    #[error("invalid config value: {key} = {value}")]
    InvalidValue { key: String, value: String },
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("validation error: {0}")]
    Validation(String),
}

/// Top-level TOML config structure.
#[derive(Debug, Deserialize, Default, Clone)]
pub struct Config {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub session: SessionConfig,
}

/// Server configuration section.
#[derive(Debug, Deserialize, Default, Clone)]
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
#[derive(Debug, Deserialize, Default, Clone)]
pub struct SessionConfig {
    pub base_index: Option<i64>,
    pub renumber_windows: Option<bool>,
    pub status: Option<bool>,
    pub status_position: Option<String>,
}

impl Config {
    /// Parse a TOML config string.
    pub fn from_toml(input: &str) -> Result<Self, ConfigError> {
        toml::from_str(input).map_err(|e| ConfigError::TomlParse(e.to_string()))
    }

    /// Validate the configuration.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if let Some(hl) = self.server.history_limit {
            if hl < 0 {
                return Err(ConfigError::Validation(
                    "history_limit must be non-negative".into(),
                ));
            }
        }
        if let Some(et) = self.server.escape_time {
            if et < 0 || et > 10000 {
                return Err(ConfigError::Validation(
                    "escape_time must be 0-10000".into(),
                ));
            }
        }
        if let Some(ref pos) = self.session.status_position {
            if pos != "top" && pos != "bottom" {
                return Err(ConfigError::Validation(
                    "status_position must be 'top' or 'bottom'".into(),
                ));
            }
        }
        Ok(())
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
        if let Some(ref v) = self.session.status_position {
            table.set("status-position", OptionValue::String(v.clone()));
        }

        table
    }

    /// Merge another config into this one (other takes precedence).
    pub fn merge(&mut self, other: &Self) {
        if other.server.socket_path.is_some() {
            self.server.socket_path = other.server.socket_path.clone();
        }
        if other.server.default_shell.is_some() {
            self.server.default_shell = other.server.default_shell.clone();
        }
        if other.server.history_limit.is_some() {
            self.server.history_limit = other.server.history_limit;
        }
        if other.server.escape_time.is_some() {
            self.server.escape_time = other.server.escape_time;
        }
        if other.server.focus_events.is_some() {
            self.server.focus_events = other.server.focus_events;
        }
        if other.server.allow_passthrough.is_some() {
            self.server.allow_passthrough = other.server.allow_passthrough;
        }
        if other.server.mouse.is_some() {
            self.server.mouse = other.server.mouse;
        }
        if other.session.base_index.is_some() {
            self.session.base_index = other.session.base_index;
        }
        if other.session.status.is_some() {
            self.session.status = other.session.status;
        }
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

    #[test]
    fn validate_good_config() {
        let toml = r#"
[server]
history_limit = 5000
escape_time = 100
[session]
status_position = "top"
"#;
        let config = Config::from_toml(toml).unwrap_or_default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn validate_negative_history_limit() {
        let toml = r#"
[server]
history_limit = -1
"#;
        let config = Config::from_toml(toml).unwrap_or_default();
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_bad_escape_time() {
        let toml = r#"
[server]
escape_time = 99999
"#;
        let config = Config::from_toml(toml).unwrap_or_default();
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_bad_status_position() {
        let toml = r#"
[session]
status_position = "left"
"#;
        let config = Config::from_toml(toml).unwrap_or_default();
        assert!(config.validate().is_err());
    }

    #[test]
    fn merge_configs() {
        let mut base = Config::default();
        base.server.history_limit = Some(2000);
        base.server.mouse = Some(false);

        let overlay = Config::from_toml(r#"
[server]
history_limit = 50000
"#).unwrap_or_default();

        base.merge(&overlay);
        assert_eq!(base.server.history_limit, Some(50000));
        assert_eq!(base.server.mouse, Some(false)); // not overridden
    }

    #[test]
    fn option_table_has_status_position() {
        let toml = r#"
[session]
status_position = "bottom"
"#;
        let config = Config::from_toml(toml).unwrap_or_default();
        let table = config.to_option_table();
        assert_eq!(
            table.get("status-position"),
            Some(&OptionValue::String("bottom".into()))
        );
    }

    #[test]
    fn config_clone() {
        let toml = r#"
[server]
history_limit = 5000
"#;
        let config = Config::from_toml(toml).unwrap_or_default();
        let config2 = config.clone();
        assert_eq!(config.server.history_limit, config2.server.history_limit);
    }

    #[test]
    fn validate_zero_escape_time() {
        let toml = r#"
[server]
escape_time = 0
"#;
        let config = Config::from_toml(toml).unwrap_or_default();
        assert!(config.validate().is_ok());
    }
}
