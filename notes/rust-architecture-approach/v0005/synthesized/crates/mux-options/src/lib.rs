//! Typed layered option tables with hierarchical resolution.
//!
//! Options are resolved from most specific (pane) to least specific (server defaults).
//! User options prefixed with `@` have no built-in default.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use mux_types::colour::Colour;

/// Option value types matching tmux's option system.
#[derive(Debug, Clone, PartialEq)]
pub enum OptionValue {
    /// String value.
    String(String),
    /// Integer value.
    Int(i64),
    /// Boolean value.
    Bool(bool),
    /// Colour value.
    Colour(Colour),
}

impl OptionValue {
    /// Try to get as a string reference.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    /// Try to get as an integer.
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Self::Int(v) => Some(*v),
            _ => None,
        }
    }

    /// Try to get as a boolean.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(v) => Some(*v),
            _ => None,
        }
    }
}

/// An option table for a single entity level.
#[derive(Debug, Clone, Default)]
pub struct OptionTable {
    entries: HashMap<String, OptionValue>,
}

impl OptionTable {
    /// Create an empty option table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set an option value.
    pub fn set(&mut self, key: impl Into<String>, value: OptionValue) {
        self.entries.insert(key.into(), value);
    }

    /// Get an option value.
    pub fn get(&self, key: &str) -> Option<&OptionValue> {
        self.entries.get(key)
    }

    /// Remove an option.
    pub fn remove(&mut self, key: &str) -> Option<OptionValue> {
        self.entries.remove(key)
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Is empty?
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterate over all entries.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &OptionValue)> {
        self.entries.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Check if an option exists.
    pub fn contains(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }
}

/// Resolve an option through a chain of option tables.
///
/// Tables are searched from first (most specific) to last (least specific).
/// The first table containing the key wins.
pub fn resolve_option<'a>(key: &str, chain: &[&'a OptionTable]) -> Option<&'a OptionValue> {
    for table in chain {
        if let Some(val) = table.get(key) {
            return Some(val);
        }
    }
    None
}

/// Built-in default options matching tmux's defaults.
pub fn builtin_defaults() -> OptionTable {
    let mut t = OptionTable::new();
    t.set("default-terminal", OptionValue::String("tmux-256color".into()));
    t.set("escape-time", OptionValue::Int(500));
    t.set("history-limit", OptionValue::Int(2000));
    t.set("focus-events", OptionValue::Bool(false));
    t.set("mouse", OptionValue::Bool(false));
    t.set("allow-passthrough", OptionValue::Bool(false));
    t.set("status", OptionValue::Bool(true));
    t.set("base-index", OptionValue::Int(0));
    t.set("pane-base-index", OptionValue::Int(0));
    t.set("display-time", OptionValue::Int(750));
    t.set("repeat-time", OptionValue::Int(500));
    t.set("word-separators", OptionValue::String(" -_@".into()));
    t.set("renumber-windows", OptionValue::Bool(false));
    t.set("set-clipboard", OptionValue::String("external".into()));
    t.set("bell-action", OptionValue::String("any".into()));
    t.set("visual-bell", OptionValue::Bool(false));
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_and_get() {
        let mut t = OptionTable::new();
        t.set("foo", OptionValue::Int(42));
        assert_eq!(t.get("foo").and_then(OptionValue::as_int), Some(42));
    }

    #[test]
    fn get_missing() {
        let t = OptionTable::new();
        assert!(t.get("nonexistent").is_none());
    }

    #[test]
    fn remove_option() {
        let mut t = OptionTable::new();
        t.set("x", OptionValue::Bool(true));
        assert!(t.remove("x").is_some());
        assert!(t.get("x").is_none());
    }

    #[test]
    fn resolve_chain_most_specific_wins() {
        let mut pane = OptionTable::new();
        pane.set("history-limit", OptionValue::Int(5000));
        let defaults = builtin_defaults();
        let result = resolve_option("history-limit", &[&pane, &defaults]);
        assert_eq!(result.and_then(OptionValue::as_int), Some(5000));
    }

    #[test]
    fn resolve_chain_falls_through() {
        let pane = OptionTable::new();
        let defaults = builtin_defaults();
        let result = resolve_option("history-limit", &[&pane, &defaults]);
        assert_eq!(result.and_then(OptionValue::as_int), Some(2000));
    }

    #[test]
    fn resolve_chain_not_found() {
        let pane = OptionTable::new();
        let result = resolve_option("nonexistent", &[&pane]);
        assert!(result.is_none());
    }

    #[test]
    fn builtin_defaults_passthrough_off() {
        let defaults = builtin_defaults();
        assert_eq!(
            defaults.get("allow-passthrough").and_then(OptionValue::as_bool),
            Some(false)
        );
    }

    #[test]
    fn builtin_defaults_escape_time() {
        let defaults = builtin_defaults();
        assert_eq!(
            defaults.get("escape-time").and_then(OptionValue::as_int),
            Some(500)
        );
    }

    #[test]
    fn user_option_with_at_prefix() {
        let mut t = OptionTable::new();
        t.set("@my-plugin-var", OptionValue::String("value".into()));
        assert_eq!(
            t.get("@my-plugin-var").and_then(OptionValue::as_str),
            Some("value")
        );
    }

    #[test]
    fn option_value_as_str() {
        let v = OptionValue::String("hello".into());
        assert_eq!(v.as_str(), Some("hello"));
        assert!(v.as_int().is_none());
    }

    #[test]
    fn table_len() {
        let mut t = OptionTable::new();
        assert_eq!(t.len(), 0);
        t.set("a", OptionValue::Bool(true));
        assert_eq!(t.len(), 1);
    }

    #[test]
    fn table_is_empty() {
        let t = OptionTable::new();
        assert!(t.is_empty());
    }

    #[test]
    fn table_contains() {
        let mut t = OptionTable::new();
        t.set("x", OptionValue::Int(1));
        assert!(t.contains("x"));
        assert!(!t.contains("y"));
    }

    #[test]
    fn table_iter() {
        let mut t = OptionTable::new();
        t.set("a", OptionValue::Int(1));
        t.set("b", OptionValue::Int(2));
        assert_eq!(t.iter().count(), 2);
    }

    #[test]
    fn builtin_defaults_count() {
        let defaults = builtin_defaults();
        assert!(defaults.len() >= 10);
    }

    #[test]
    fn colour_option_value() {
        let v = OptionValue::Colour(Colour::Indexed(5));
        assert!(v.as_int().is_none());
        assert!(v.as_bool().is_none());
    }

    #[test]
    fn overwrite_option() {
        let mut t = OptionTable::new();
        t.set("x", OptionValue::Int(1));
        t.set("x", OptionValue::Int(2));
        assert_eq!(t.get("x").and_then(OptionValue::as_int), Some(2));
    }

    mod prop {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn resolve_most_specific_wins(v1 in 0i64..1000, v2 in 0i64..1000) {
                let mut t1 = OptionTable::new();
                let mut t2 = OptionTable::new();
                t1.set("k", OptionValue::Int(v1));
                t2.set("k", OptionValue::Int(v2));
                let result = resolve_option("k", &[&t1, &t2]);
                prop_assert_eq!(result.and_then(OptionValue::as_int), Some(v1));
            }

            #[test]
            fn set_get_roundtrip(key in "[a-z]{1,10}", val in -1000i64..1000) {
                let mut t = OptionTable::new();
                t.set(key.clone(), OptionValue::Int(val));
                prop_assert_eq!(t.get(&key).and_then(OptionValue::as_int), Some(val));
            }
        }
    }
}
