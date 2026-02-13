//! # mux-options
//!
//! Typed layered option tables. Options are resolved via layered lookup:
//! pane -> window -> session -> server -> built-in defaults.
//!
//! L2 data crate.

#![forbid(unsafe_code)]

use std::collections::HashMap;

/// An option value that can be stored in an option table.
#[derive(Debug, Clone, PartialEq)]
pub enum OptionValue {
    /// Boolean option.
    Bool(bool),
    /// Integer option.
    Int(i64),
    /// String option.
    String(String),
    /// Colour option (stored as string for now).
    Colour(String),
}

impl OptionValue {
    /// Try to extract as bool.
    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Try to extract as i64.
    #[must_use]
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Self::Int(n) => Some(*n),
            _ => None,
        }
    }

    /// Try to extract as string.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) | Self::Colour(s) => Some(s),
            _ => None,
        }
    }
}

/// A single-level option table. Multiple tables are stacked for layered resolution.
#[derive(Debug, Clone, Default)]
pub struct OptionTable {
    entries: HashMap<String, OptionValue>,
}

impl OptionTable {
    /// Create a new empty option table.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set an option value.
    pub fn set(&mut self, key: impl Into<String>, value: OptionValue) {
        self.entries.insert(key.into(), value);
    }

    /// Get an option value from this table only (no fallback).
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&OptionValue> {
        self.entries.get(key)
    }

    /// Remove an option.
    pub fn remove(&mut self, key: &str) -> Option<OptionValue> {
        self.entries.remove(key)
    }

    /// Number of entries in this table.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether this table is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterate over all key-value pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &OptionValue)> {
        self.entries.iter().map(|(k, v)| (k.as_str(), v))
    }
}

/// Resolve an option across a stack of tables (first match wins).
#[must_use]
pub fn resolve_option<'a>(key: &str, tables: &'a [&OptionTable]) -> Option<&'a OptionValue> {
    for table in tables {
        if let Some(v) = table.get(key) {
            return Some(v);
        }
    }
    None
}

/// Built-in default option table.
#[must_use]
pub fn builtin_defaults() -> OptionTable {
    let mut t = OptionTable::new();
    t.set("history-limit", OptionValue::Int(2000));
    t.set("escape-time", OptionValue::Int(500));
    t.set("focus-events", OptionValue::Bool(false));
    t.set("mouse", OptionValue::Bool(false));
    t.set("status", OptionValue::Bool(true));
    t.set("status-position", OptionValue::String("bottom".into()));
    t.set("base-index", OptionValue::Int(0));
    t.set("renumber-windows", OptionValue::Bool(false));
    t.set(
        "default-terminal",
        OptionValue::String("tmux-256color".into()),
    );
    t.set("allow-passthrough", OptionValue::Bool(false)); // INV-220
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn option_value_as_bool() {
        assert_eq!(OptionValue::Bool(true).as_bool(), Some(true));
        assert_eq!(OptionValue::Int(0).as_bool(), None);
    }

    #[test]
    fn option_value_as_int() {
        assert_eq!(OptionValue::Int(42).as_int(), Some(42));
        assert_eq!(OptionValue::Bool(true).as_int(), None);
    }

    #[test]
    fn option_value_as_str() {
        assert_eq!(
            OptionValue::String("hello".into()).as_str(),
            Some("hello")
        );
        assert_eq!(OptionValue::Int(0).as_str(), None);
    }

    #[test]
    fn table_set_get() {
        let mut t = OptionTable::new();
        t.set("mouse", OptionValue::Bool(true));
        assert_eq!(t.get("mouse"), Some(&OptionValue::Bool(true)));
    }

    #[test]
    fn table_remove() {
        let mut t = OptionTable::new();
        t.set("mouse", OptionValue::Bool(true));
        assert!(t.remove("mouse").is_some());
        assert!(t.get("mouse").is_none());
    }

    #[test]
    fn layered_resolution() {
        let mut pane = OptionTable::new();
        let mut session = OptionTable::new();
        session.set("mouse", OptionValue::Bool(false));
        pane.set("mouse", OptionValue::Bool(true));

        let tables = [&pane, &session];
        let val = resolve_option("mouse", &tables);
        assert_eq!(val, Some(&OptionValue::Bool(true)));
    }

    #[test]
    fn layered_fallback() {
        let pane = OptionTable::new();
        let mut session = OptionTable::new();
        session.set("mouse", OptionValue::Bool(true));

        let tables = [&pane, &session];
        let val = resolve_option("mouse", &tables);
        assert_eq!(val, Some(&OptionValue::Bool(true)));
    }

    #[test]
    fn layered_not_found() {
        let pane = OptionTable::new();
        let session = OptionTable::new();
        let tables = [&pane, &session];
        assert!(resolve_option("nonexistent", &tables).is_none());
    }

    #[test]
    fn builtin_defaults_has_history_limit() {
        let defaults = builtin_defaults();
        let val = defaults.get("history-limit");
        assert_eq!(val, Some(&OptionValue::Int(2000)));
    }

    #[test]
    fn builtin_passthrough_disabled() {
        let defaults = builtin_defaults();
        let val = defaults.get("allow-passthrough");
        assert_eq!(val, Some(&OptionValue::Bool(false)));
    }

    #[test]
    fn table_len() {
        let mut t = OptionTable::new();
        assert!(t.is_empty());
        t.set("a", OptionValue::Bool(true));
        t.set("b", OptionValue::Int(1));
        assert_eq!(t.len(), 2);
    }

    #[test]
    fn colour_value_as_str() {
        let v = OptionValue::Colour("red".into());
        assert_eq!(v.as_str(), Some("red"));
    }

    #[test]
    fn option_value_as_bool_with_type_check() {
        let v = OptionValue::Bool(true);
        assert_eq!(v.as_bool(), Some(true));
        let v2 = OptionValue::Int(42);
        assert_eq!(v2.as_bool(), None);
    }

    #[test]
    fn resolve_option_nearest_wins() {
        let mut pane = OptionTable::new();
        let mut session = OptionTable::new();
        pane.set("color", OptionValue::String("blue".into()));
        session.set("color", OptionValue::String("red".into()));
        let tables = vec![&pane, &session];
        assert_eq!(
            resolve_option("color", &tables),
            Some(&OptionValue::String("blue".into()))
        );
    }
}
