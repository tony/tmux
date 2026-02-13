//! Config loading and option resolution tests.
#![allow(clippy::unwrap_used)]

use mux_config::{Config, ConfigError};
use mux_options::OptionValue;

#[test]
fn load_empty_config() {
    let mut c = Config::new();
    c.load_tmux_conf("").unwrap();
    assert_eq!(c.command_count(), 0);
}

#[test]
fn load_comment_only() {
    let mut c = Config::new();
    c.load_tmux_conf("# comment\n# another comment\n").unwrap();
    assert_eq!(c.command_count(), 0);
}

#[test]
fn load_set_global_bool() {
    let mut c = Config::new();
    c.load_tmux_conf("set -g mouse on\n").unwrap();
    assert_eq!(c.get_server_option("mouse").and_then(OptionValue::as_bool), Some(true));
}

#[test]
fn load_set_global_int() {
    let mut c = Config::new();
    c.load_tmux_conf("set -g history-limit 10000\n").unwrap();
    assert_eq!(c.get_server_option("history-limit").and_then(OptionValue::as_int), Some(10000));
}

#[test]
fn load_set_global_string() {
    let mut c = Config::new();
    c.load_tmux_conf("set -g default-terminal screen-256color\n").unwrap();
    assert_eq!(c.get_server_option("default-terminal").and_then(OptionValue::as_str), Some("screen-256color"));
}

#[test]
fn load_multiple_options() {
    let mut c = Config::new();
    c.load_tmux_conf("set -g mouse on\nset -g status off\nset -g history-limit 5000\n").unwrap();
    assert_eq!(c.command_count(), 3);
}

#[test]
fn load_mixed_comments_and_commands() {
    let mut c = Config::new();
    c.load_tmux_conf("# header\nset -g mouse on\n# separator\nset -g status off\n").unwrap();
    assert_eq!(c.command_count(), 2);
}

#[test]
fn override_option() {
    let mut c = Config::new();
    c.load_tmux_conf("set -g mouse on\nset -g mouse off\n").unwrap();
    assert_eq!(c.get_server_option("mouse").and_then(OptionValue::as_bool), Some(false));
}

#[test]
fn non_global_set() {
    let mut c = Config::new();
    c.load_tmux_conf("set mouse on\n").unwrap();
    assert!(c.session_options.get("mouse").is_some());
}

#[test]
fn defaults_present() {
    let c = Config::new();
    assert!(c.get_server_option("escape-time").is_some());
    assert!(c.get_server_option("history-limit").is_some());
}

#[test]
fn set_option_alias() {
    let mut c = Config::new();
    c.load_tmux_conf("set-option -g mouse on\n").unwrap();
    assert_eq!(c.get_server_option("mouse").and_then(OptionValue::as_bool), Some(true));
}

#[test]
fn bind_command_counted() {
    let mut c = Config::new();
    c.load_tmux_conf("bind r source-file\n").unwrap();
    assert_eq!(c.command_count(), 1);
}

#[test]
fn config_error_unknown_option() {
    let e = ConfigError::UnknownOption("foo".into());
    assert!(e.to_string().contains("unknown option"));
}

#[test]
fn config_error_parse() {
    let e = ConfigError::Parse("bad syntax".into());
    assert!(e.to_string().contains("parse error"));
}

#[test]
fn config_error_io() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "missing");
    let e: ConfigError = io_err.into();
    assert!(e.to_string().contains("I/O error"));
}

#[test]
fn config_clone_preserves() {
    let mut c = Config::new();
    c.load_tmux_conf("set -g mouse on\n").unwrap();
    let c2 = c.clone();
    assert_eq!(c.command_count(), c2.command_count());
}

#[test]
fn config_default_is_new() {
    let c1 = Config::new();
    let c2 = Config::default();
    assert_eq!(c1.command_count(), c2.command_count());
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
fn set_true_value() {
    let mut c = Config::new();
    c.load_tmux_conf("set -g mouse true\n").unwrap();
    assert_eq!(c.get_server_option("mouse").and_then(OptionValue::as_bool), Some(true));
}

#[test]
fn set_false_value() {
    let mut c = Config::new();
    c.load_tmux_conf("set -g mouse false\n").unwrap();
    assert_eq!(c.get_server_option("mouse").and_then(OptionValue::as_bool), Some(false));
}
