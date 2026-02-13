//! Property-based tests for mux-options.
#![allow(clippy::unwrap_used)]

use proptest::prelude::*;
use mux_options::{OptionTable, OptionValue, resolve_option, builtin_defaults};

proptest! {
    #[test]
    fn set_then_get(key in "[a-z_]{1,20}", value in "[a-zA-Z0-9]{1,30}") {
        let mut table = OptionTable::new();
        table.set(key.clone(), OptionValue::String(value.clone()));
        let val = table.get(&key);
        prop_assert!(val.is_some());
        prop_assert_eq!(val.unwrap().as_str(), Some(value.as_str()));
    }

    #[test]
    fn missing_key_none(key in "[a-z_]{1,20}") {
        let table = OptionTable::new();
        prop_assert!(table.get(&key).is_none());
    }

    #[test]
    fn remove_key(key in "[a-z_]{1,10}", value in "[a-zA-Z]{1,10}") {
        let mut table = OptionTable::new();
        table.set(key.clone(), OptionValue::String(value));
        let removed = table.remove(&key);
        prop_assert!(removed.is_some());
        prop_assert!(table.get(&key).is_none());
    }

    #[test]
    fn bulk_set_get(entries in proptest::collection::vec(("[a-z]{1,5}", "[A-Z]{1,5}"), 1..30)) {
        let mut table = OptionTable::new();
        for (k, v) in &entries {
            table.set(k.clone(), OptionValue::String(v.clone()));
        }
        // Build expected map: last value for each key wins
        let mut expected = std::collections::HashMap::new();
        for (k, v) in &entries {
            expected.insert(k.as_str(), v.as_str());
        }
        for (k, v) in &expected {
            let val = table.get(*k);
            prop_assert!(val.is_some());
            prop_assert_eq!(val.unwrap().as_str(), Some(*v));
        }
    }

    #[test]
    fn table_length_matches_keys(entries in proptest::collection::vec(("[a-z]{1,3}", "[A-Z]{1,3}"), 1..10)) {
        let mut table = OptionTable::new();
        let mut unique_keys = std::collections::HashSet::new();
        for (k, v) in &entries {
            table.set(k.clone(), OptionValue::String(v.clone()));
            unique_keys.insert(k.clone());
        }
        prop_assert_eq!(table.len(), unique_keys.len());
    }
}

#[test]
fn builtin_defaults_exist() {
    let table = builtin_defaults();
    assert!(!table.is_empty());
}

#[test]
fn resolve_option_chain() {
    let mut global = OptionTable::new();
    let mut session = OptionTable::new();
    global.set("key", OptionValue::String("global".into()));
    session.set("key", OptionValue::String("session".into()));
    // Session should override global
    let result = resolve_option("key", &[&session, &global]);
    assert!(result.is_some());
    assert_eq!(result.unwrap().as_str(), Some("session"));
}

#[test]
fn option_value_types() {
    assert_eq!(OptionValue::Int(42).as_int(), Some(42));
    assert_eq!(OptionValue::Bool(true).as_bool(), Some(true));
    assert_eq!(OptionValue::String("hello".into()).as_str(), Some("hello"));
}

#[test]
fn table_contains() {
    let mut table = OptionTable::new();
    table.set("key", OptionValue::Int(1));
    assert!(table.contains("key"));
    assert!(!table.contains("missing"));
}

#[test]
fn table_iter() {
    let mut table = OptionTable::new();
    table.set("a", OptionValue::Int(1));
    table.set("b", OptionValue::Int(2));
    let count = table.iter().count();
    assert_eq!(count, 2);
}
