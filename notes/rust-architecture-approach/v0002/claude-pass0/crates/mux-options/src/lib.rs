//! # mux-options
//!
//! Typed option system matching tmux's `set-option`/`show-options`.
//!
//! ## Option Scopes
//! - Server: global server-wide options
//! - Session: per-session options (with inheritance from server)
//! - Window: per-window options (with inheritance from session)
//! - Pane: per-pane options (with inheritance from window)
//!
//! ## Design
//! Options are stored in layered maps. Lookup walks the chain:
//! pane -> window -> session -> server -> default.

#![forbid(unsafe_code)]

use std::collections::HashMap;

/// Option scope levels, matching tmux.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OptionScope {
    Server,
    Session,
    Window,
    Pane,
}

/// Option value types.
#[derive(Debug, Clone, PartialEq)]
pub enum OptionValue {
    String(String),
    Number(i64),
    Boolean(bool),
    Colour(String),
    Style(String),
    /// Array/list option.
    Array(Vec<String>),
}

impl OptionValue {
    /// Get as string, if the value is a string.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) | Self::Colour(s) | Self::Style(s) => Some(s),
            _ => None,
        }
    }

    /// Get as number, if the value is a number.
    #[must_use]
    pub const fn as_number(&self) -> Option<i64> {
        match self {
            Self::Number(n) => Some(*n),
            _ => None,
        }
    }

    /// Get as boolean, if the value is a boolean.
    #[must_use]
    pub const fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Boolean(b) => Some(*b),
            _ => None,
        }
    }
}

/// Description of a known option.
#[derive(Debug, Clone)]
pub struct OptionDef {
    /// Option name (e.g., "status-style").
    pub name: &'static str,
    /// Which scopes this option applies to.
    pub scope: OptionScope,
    /// Default value.
    pub default: OptionValue,
    /// Whether this option is an array type.
    pub is_array: bool,
}

/// A set of options at a single scope level.
#[derive(Debug, Clone, Default)]
pub struct OptionTable {
    values: HashMap<String, OptionValue>,
}

impl OptionTable {
    /// Create a new empty option table.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set an option value.
    pub fn set(&mut self, name: &str, value: OptionValue) {
        self.values.insert(name.to_owned(), value);
    }

    /// Get an option value from this table only (no inheritance).
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&OptionValue> {
        self.values.get(name)
    }

    /// Remove an option from this table.
    pub fn unset(&mut self, name: &str) -> Option<OptionValue> {
        self.values.remove(name)
    }

    /// Iterate over all options in this table.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &OptionValue)> {
        self.values.iter().map(|(k, v)| (k.as_str(), v))
    }
}

/// Layered option lookup across scopes.
///
/// Walks the inheritance chain: pane -> window -> session -> server -> default.
#[must_use]
pub fn lookup_option<'a>(
    name: &str,
    layers: &'a [&'a OptionTable],
    defaults: &'a HashMap<String, OptionValue>,
) -> Option<&'a OptionValue> {
    for layer in layers {
        if let Some(value) = layer.get(name) {
            return Some(value);
        }
    }
    defaults.get(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn option_table_set_get() {
        let mut table = OptionTable::new();
        table.set("status", OptionValue::Boolean(true));
        assert_eq!(
            table.get("status").and_then(OptionValue::as_bool),
            Some(true)
        );
    }

    #[test]
    fn layered_lookup_prefers_inner_scope() {
        let mut server = OptionTable::new();
        server.set("base-index", OptionValue::Number(0));

        let mut session = OptionTable::new();
        session.set("base-index", OptionValue::Number(1));

        let defaults = HashMap::new();
        let layers = [&session, &server];
        let result = lookup_option("base-index", &layers, &defaults);
        assert_eq!(result.and_then(OptionValue::as_number), Some(1));
    }

    #[test]
    fn layered_lookup_falls_through_to_defaults() {
        let table = OptionTable::new();
        let mut defaults = HashMap::new();
        defaults.insert("history-limit".into(), OptionValue::Number(2000));

        let layers = [&table];
        let result = lookup_option("history-limit", &layers, &defaults);
        assert_eq!(result.and_then(OptionValue::as_number), Some(2000));
    }
}
