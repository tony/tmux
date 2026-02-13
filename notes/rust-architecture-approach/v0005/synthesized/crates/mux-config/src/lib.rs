//! Config loading from TOML and .tmux.conf files.

#![forbid(unsafe_code)]

use mux_options::{OptionTable, OptionValue, builtin_defaults};
use mux_cmd_parse::{parse_config, ParsedCommand, CmdParseError};
use thiserror::Error;

/// Configuration errors.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("config parse error: {0}")]
    Parse(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("unknown option: {0}")]
    UnknownOption(String),
}

impl From<CmdParseError> for ConfigError {
    fn from(e: CmdParseError) -> Self {
        Self::Parse(e.to_string())
    }
}

/// A loaded configuration.
#[derive(Debug, Clone)]
pub struct Config {
    pub server_options: OptionTable,
    pub session_options: OptionTable,
    pub window_options: OptionTable,
    pub commands: Vec<ParsedCommand>,
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

impl Config {
    /// Create a new config with defaults.
    pub fn new() -> Self {
        Self {
            server_options: builtin_defaults(),
            session_options: OptionTable::new(),
            window_options: OptionTable::new(),
            commands: Vec::new(),
        }
    }

    /// Load from a .tmux.conf style string.
    pub fn load_tmux_conf(&mut self, content: &str) -> Result<(), ConfigError> {
        let commands = parse_config(content)?;
        for cmd in &commands {
            if cmd.name == "set" || cmd.name == "set-option" {
                self.apply_set_command(cmd);
            }
        }
        self.commands.extend(commands);
        Ok(())
    }

    fn apply_set_command(&mut self, cmd: &ParsedCommand) {
        let is_global = cmd.flags.contains(&'g');
        if cmd.args.len() >= 2 {
            let key = &cmd.args[0];
            let value = &cmd.args[1];
            let opt_value = if value == "on" || value == "true" {
                OptionValue::Bool(true)
            } else if value == "off" || value == "false" {
                OptionValue::Bool(false)
            } else if let Ok(n) = value.parse::<i64>() {
                OptionValue::Int(n)
            } else {
                OptionValue::String(value.clone())
            };

            if is_global {
                self.server_options.set(key.clone(), opt_value);
            } else {
                self.session_options.set(key.clone(), opt_value);
            }
        }
    }

    /// Get a server option.
    pub fn get_server_option(&self, key: &str) -> Option<&OptionValue> {
        self.server_options.get(key)
    }

    /// Total number of commands loaded.
    pub fn command_count(&self) -> usize {
        self.commands.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let c = Config::new();
        assert!(c.get_server_option("history-limit").is_some());
    }

    #[test]
    fn load_tmux_conf() {
        let mut c = Config::new();
        c.load_tmux_conf("set -g mouse on\nset -g history-limit 5000\n").unwrap_or(());
        assert_eq!(
            c.get_server_option("mouse").and_then(OptionValue::as_bool),
            Some(true)
        );
        assert_eq!(
            c.get_server_option("history-limit").and_then(OptionValue::as_int),
            Some(5000)
        );
    }

    #[test]
    fn load_tmux_conf_comments() {
        let mut c = Config::new();
        c.load_tmux_conf("# comment\nset -g status on\n").unwrap_or(());
        assert_eq!(c.command_count(), 1);
    }

    #[test]
    fn load_tmux_conf_bool_off() {
        let mut c = Config::new();
        c.load_tmux_conf("set -g mouse off\n").unwrap_or(());
        assert_eq!(
            c.get_server_option("mouse").and_then(OptionValue::as_bool),
            Some(false)
        );
    }

    #[test]
    fn load_tmux_conf_string_value() {
        let mut c = Config::new();
        c.load_tmux_conf("set -g default-terminal tmux-256color\n").unwrap_or(());
        assert_eq!(
            c.get_server_option("default-terminal").and_then(OptionValue::as_str),
            Some("tmux-256color")
        );
    }

    #[test]
    fn allow_passthrough_default_off() {
        let c = Config::new();
        assert_eq!(
            c.get_server_option("allow-passthrough").and_then(OptionValue::as_bool),
            Some(false)
        );
    }

    #[test]
    fn command_count() {
        let mut c = Config::new();
        c.load_tmux_conf("set -g a on\nset -g b off\n").unwrap_or(());
        assert_eq!(c.command_count(), 2);
    }

    #[test]
    fn empty_config() {
        let mut c = Config::new();
        c.load_tmux_conf("").unwrap_or(());
        assert_eq!(c.command_count(), 0);
    }

    #[test]
    fn non_global_set() {
        let mut c = Config::new();
        c.load_tmux_conf("set status on\n").unwrap_or(());
        assert!(c.session_options.get("status").is_some());
    }

    #[test]
    fn config_default() {
        let c = Config::default();
        assert!(c.get_server_option("escape-time").is_some());
    }

    #[test]
    fn config_error_display() {
        let e = ConfigError::UnknownOption("foo".into());
        assert!(e.to_string().contains("foo"));
    }

    #[test]
    fn multiple_commands() {
        let mut c = Config::new();
        c.load_tmux_conf("set -g mouse on\nbind r source-file\nset -g status off\n").unwrap_or(());
        assert!(c.command_count() >= 2);
    }

    #[test]
    fn load_tmux_conf_true() {
        let mut c = Config::new();
        c.load_tmux_conf("set -g mouse true\n").unwrap_or(());
        assert_eq!(
            c.get_server_option("mouse").and_then(OptionValue::as_bool),
            Some(true)
        );
    }

    #[test]
    fn load_tmux_conf_false() {
        let mut c = Config::new();
        c.load_tmux_conf("set -g mouse false\n").unwrap_or(());
        assert_eq!(
            c.get_server_option("mouse").and_then(OptionValue::as_bool),
            Some(false)
        );
    }

    #[test]
    fn config_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "missing");
        let e: ConfigError = io_err.into();
        assert!(matches!(e, ConfigError::Io(_)));
    }

    #[test]
    fn config_error_display_parse() {
        let e = ConfigError::Parse("bad syntax".into());
        assert!(e.to_string().contains("bad syntax"));
    }

    #[test]
    fn config_error_display_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
        let e: ConfigError = io_err.into();
        assert!(e.to_string().contains("I/O error"));
    }

    #[test]
    fn override_same_key() {
        let mut c = Config::new();
        c.load_tmux_conf("set -g mouse on\nset -g mouse off\n").unwrap_or(());
        assert_eq!(
            c.get_server_option("mouse").and_then(OptionValue::as_bool),
            Some(false)
        );
    }

    #[test]
    fn integer_option() {
        let mut c = Config::new();
        c.load_tmux_conf("set -g escape-time 50\n").unwrap_or(());
        assert_eq!(
            c.get_server_option("escape-time").and_then(OptionValue::as_int),
            Some(50)
        );
    }

    #[test]
    fn config_clone() {
        let c1 = Config::new();
        let c2 = c1.clone();
        assert_eq!(c1.command_count(), c2.command_count());
    }

    #[test]
    fn config_debug() {
        let c = Config::new();
        let dbg = format!("{c:?}");
        assert!(dbg.contains("Config"));
    }
}
