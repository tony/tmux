//! Integration tests: config loading and options resolution pipeline.

use mux_config::Config;
use mux_options::{OptionTable, OptionValue, resolve_option};

#[test]
fn load_empty_config_applies_defaults() {
    let mut cfg = Config::new();
    cfg.load_tmux_conf("").unwrap_or(());
    assert_eq!(cfg.command_count(), 0);
    // Defaults should still exist
    assert!(cfg.get_server_option("history-limit").is_some());
}

#[test]
fn set_option_updates_table() {
    let mut table = OptionTable::new();
    table.set("status".to_string(), OptionValue::String("on".into()));
    let val = table.get("status");
    assert!(val.is_some());
    assert_eq!(val.and_then(OptionValue::as_str), Some("on"));
}

#[test]
fn config_lines_parsed_into_commands() {
    let mut cfg = Config::new();
    cfg.load_tmux_conf("set -g status off\nset -g prefix C-a\n").unwrap_or(());
    assert_eq!(cfg.command_count(), 2);
}

#[test]
fn config_applies_to_option_table() {
    let mut cfg = Config::new();
    cfg.load_tmux_conf("set -g default-terminal screen-256color").unwrap_or(());
    assert_eq!(
        cfg.get_server_option("default-terminal").and_then(OptionValue::as_str),
        Some("screen-256color")
    );
}

#[test]
fn session_scope_overrides_global() {
    let mut global = OptionTable::new();
    let mut session = OptionTable::new();
    global.set("status".to_string(), OptionValue::String("on".into()));
    session.set("status".to_string(), OptionValue::String("off".into()));
    // Session scope should take precedence (searched first)
    let result = resolve_option("status", &[&session, &global]);
    assert!(result.is_some());
    assert_eq!(result.and_then(OptionValue::as_str), Some("off"));
    // Global still has its own value
    let global_val = global.get("status");
    assert_eq!(global_val.and_then(OptionValue::as_str), Some("on"));
}

#[test]
fn bool_option_on_off() {
    let mut cfg = Config::new();
    cfg.load_tmux_conf("set -g mouse on\n").unwrap_or(());
    assert_eq!(
        cfg.get_server_option("mouse").and_then(OptionValue::as_bool),
        Some(true)
    );
}

#[test]
fn bool_option_off() {
    let mut cfg = Config::new();
    cfg.load_tmux_conf("set -g mouse off\n").unwrap_or(());
    assert_eq!(
        cfg.get_server_option("mouse").and_then(OptionValue::as_bool),
        Some(false)
    );
}

#[test]
fn integer_option() {
    let mut cfg = Config::new();
    cfg.load_tmux_conf("set -g escape-time 50\n").unwrap_or(());
    assert_eq!(
        cfg.get_server_option("escape-time").and_then(OptionValue::as_int),
        Some(50)
    );
}

#[test]
fn override_same_key() {
    let mut cfg = Config::new();
    cfg.load_tmux_conf("set -g mouse on\nset -g mouse off\n").unwrap_or(());
    assert_eq!(
        cfg.get_server_option("mouse").and_then(OptionValue::as_bool),
        Some(false)
    );
}

#[test]
fn non_global_set_goes_to_session() {
    let mut cfg = Config::new();
    cfg.load_tmux_conf("set status on\n").unwrap_or(());
    assert!(cfg.session_options.get("status").is_some());
}

#[test]
fn config_with_comments() {
    let mut cfg = Config::new();
    cfg.load_tmux_conf("# comment\nset -g status on\n").unwrap_or(());
    assert_eq!(cfg.command_count(), 1);
}

#[test]
fn config_default() {
    let cfg = Config::default();
    assert!(cfg.get_server_option("escape-time").is_some());
}

#[test]
fn resolve_option_chain() {
    let mut global = OptionTable::new();
    let mut session = OptionTable::new();
    let window = OptionTable::new();
    global.set("key".to_string(), OptionValue::String("global".into()));
    session.set("key".to_string(), OptionValue::String("session".into()));
    // Window has no value, session overrides global
    let result = resolve_option("key", &[&window, &session, &global]);
    assert_eq!(result.and_then(OptionValue::as_str), Some("session"));
}
