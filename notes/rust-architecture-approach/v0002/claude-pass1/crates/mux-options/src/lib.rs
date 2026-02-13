//! # mux-options
//!
//! Typed option system matching tmux's `set-option`/`show-options`.
//! Options are stored in layered maps. Lookup walks the chain:
//! pane -> window -> session -> server -> default.

#![forbid(unsafe_code)]

use std::collections::HashMap;

/// Option scope levels, matching tmux.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OptionScope { Server, Session, Window, Pane }

/// Option value types.
#[derive(Debug, Clone, PartialEq)]
pub enum OptionValue {
    String(String),
    Number(i64),
    Boolean(bool),
    Colour(String),
    Style(String),
    Array(Vec<String>),
}

impl OptionValue {
    #[must_use] pub fn as_str(&self) -> Option<&str> { match self { Self::String(s) | Self::Colour(s) | Self::Style(s) => Some(s), _ => None } }
    #[must_use] pub const fn as_number(&self) -> Option<i64> { match self { Self::Number(n) => Some(*n), _ => None } }
    #[must_use] pub const fn as_bool(&self) -> Option<bool> { match self { Self::Boolean(b) => Some(*b), _ => None } }
}

/// A set of options at a single scope level.
#[derive(Debug, Clone, Default)]
pub struct OptionTable { values: HashMap<String, OptionValue> }

impl OptionTable {
    #[must_use] pub fn new() -> Self { Self::default() }
    pub fn set(&mut self, name: &str, value: OptionValue) { self.values.insert(name.to_owned(), value); }
    #[must_use] pub fn get(&self, name: &str) -> Option<&OptionValue> { self.values.get(name) }
    pub fn unset(&mut self, name: &str) -> Option<OptionValue> { self.values.remove(name) }
    pub fn iter(&self) -> impl Iterator<Item = (&str, &OptionValue)> { self.values.iter().map(|(k, v)| (k.as_str(), v)) }
}

/// Layered option lookup across scopes.
#[must_use]
pub fn lookup_option<'a>(name: &str, layers: &'a [&'a OptionTable], defaults: &'a HashMap<String, OptionValue>) -> Option<&'a OptionValue> {
    for layer in layers { if let Some(v) = layer.get(name) { return Some(v); } }
    defaults.get(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn option_table_set_get() {
        let mut t = OptionTable::new();
        t.set("status", OptionValue::Boolean(true));
        assert_eq!(t.get("status").and_then(OptionValue::as_bool), Some(true));
    }

    #[test]
    fn layered_lookup_prefers_inner() {
        let mut srv = OptionTable::new(); srv.set("base-index", OptionValue::Number(0));
        let mut sess = OptionTable::new(); sess.set("base-index", OptionValue::Number(1));
        let defaults = HashMap::new();
        assert_eq!(lookup_option("base-index", &[&sess, &srv], &defaults).and_then(OptionValue::as_number), Some(1));
    }

    #[test]
    fn layered_lookup_falls_to_defaults() {
        let t = OptionTable::new();
        let mut defaults = HashMap::new();
        defaults.insert("history-limit".into(), OptionValue::Number(2000));
        assert_eq!(lookup_option("history-limit", &[&t], &defaults).and_then(OptionValue::as_number), Some(2000));
    }
}
