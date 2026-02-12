//! # mux-orm
//!
//! ORM-like query interface for terminal multiplexer objects
//! (sessions, windows, panes).
//!
//! ## Key Design
//! - `QueryList<T>`: typed collection with filter/get semantics.
//! - `Queryable` trait: objects that can be queried by name or ID.
//! - `QueryError`: ObjectDoesNotExist, MultipleObjectsReturned.
//! - RULE-S12-02: QueryList with typed errors.
//! - RULE-S12-03: ObjectDoesNotExist and MultipleObjectsReturned.

#![forbid(unsafe_code)]

use std::fmt;

/// Error type for query operations.
///
/// RULE-S12-03: Two error variants for exact-match queries.
/// INV-005: Implements Error + Send + Sync + 'static.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryError {
    /// No object matched the query.
    ObjectDoesNotExist { query: String },
    /// Multiple objects matched a query that expected exactly one.
    MultipleObjectsReturned { query: String, count: usize },
}

impl fmt::Display for QueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ObjectDoesNotExist { query } =>
                write!(f, "object does not exist: '{query}'"),
            Self::MultipleObjectsReturned { query, count } =>
                write!(f, "multiple objects returned for '{query}' (found {count})"),
        }
    }
}

impl std::error::Error for QueryError {}

// INV-005: Compile-time check
const _: fn() = || {
    fn assert_send_sync<T: Send + Sync + 'static>() {}
    assert_send_sync::<QueryError>();
};

/// Trait for objects that can be queried by name or ID.
///
/// Implementors provide a unique ID and a human-readable name.
pub trait Queryable {
    /// Unique numeric identifier.
    fn id(&self) -> u64;
    /// Human-readable name.
    fn name(&self) -> &str;
}

/// An ORM-like query list supporting filter and get operations.
///
/// RULE-S12-02: QueryList with typed errors.
/// Provides Django-style `.filter()` and `.get()` semantics.
#[derive(Debug, Clone)]
pub struct QueryList<T> {
    items: Vec<T>,
}

impl<T: Queryable + Clone> QueryList<T> {
    /// Create a new QueryList from a vector of items.
    pub fn new(items: Vec<T>) -> Self {
        Self { items }
    }

    /// Create an empty QueryList.
    pub fn empty() -> Self {
        Self { items: Vec::new() }
    }

    /// Number of items.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// True if empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Get all items as a slice.
    pub fn all(&self) -> &[T] {
        &self.items
    }

    /// Filter items by a predicate, returning a new QueryList.
    ///
    /// RULE-S12-09: QueryList supports filter().
    pub fn filter<F: Fn(&T) -> bool>(&self, predicate: F) -> QueryList<T> {
        QueryList {
            items: self.items.iter().filter(|item| predicate(item)).cloned().collect(),
        }
    }

    /// Get exactly one item matching the predicate.
    ///
    /// Returns `ObjectDoesNotExist` if no items match, or
    /// `MultipleObjectsReturned` if more than one matches.
    #[must_use]
    pub fn get<F: Fn(&T) -> bool>(&self, predicate: F, query_desc: &str) -> Result<T, QueryError> {
        let matches: Vec<&T> = self.items.iter().filter(|item| predicate(item)).collect();
        match matches.len() {
            0 => Err(QueryError::ObjectDoesNotExist {
                query: query_desc.to_string(),
            }),
            1 => Ok(matches[0].clone()),
            n => Err(QueryError::MultipleObjectsReturned {
                query: query_desc.to_string(),
                count: n,
            }),
        }
    }

    /// Get an item by its unique ID.
    #[must_use]
    pub fn get_by_id(&self, id: u64) -> Result<T, QueryError> {
        self.get(|item| item.id() == id, &format!("id={id}"))
    }

    /// Get an item by its name.
    #[must_use]
    pub fn get_by_name(&self, name: &str) -> Result<T, QueryError> {
        self.get(|item| item.name() == name, &format!("name='{name}'"))
    }

    /// Get the first item, or ObjectDoesNotExist.
    #[must_use]
    pub fn first(&self) -> Result<T, QueryError> {
        self.items.first()
            .cloned()
            .ok_or_else(|| QueryError::ObjectDoesNotExist {
                query: "first()".to_string(),
            })
    }

    /// Add an item to the list.
    pub fn push(&mut self, item: T) {
        self.items.push(item);
    }
}

impl<T> Default for QueryList<T> {
    fn default() -> Self {
        Self { items: Vec::new() }
    }
}

impl<T> IntoIterator for QueryList<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    struct TestSession {
        id: u64,
        name: String,
    }

    impl Queryable for TestSession {
        fn id(&self) -> u64 { self.id }
        fn name(&self) -> &str { &self.name }
    }

    fn sample_sessions() -> QueryList<TestSession> {
        QueryList::new(vec![
            TestSession { id: 1, name: "main".into() },
            TestSession { id: 2, name: "work".into() },
            TestSession { id: 3, name: "test".into() },
        ])
    }

    #[test]
    fn test_filter() {
        let sessions = sample_sessions();
        let filtered = sessions.filter(|s| s.name().starts_with('m'));
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered.all()[0].name(), "main");
    }

    #[test]
    fn test_get_by_id() {
        let sessions = sample_sessions();
        let s = sessions.get_by_id(2).unwrap();
        assert_eq!(s.name(), "work");
    }

    #[test]
    fn test_get_by_name() {
        let sessions = sample_sessions();
        let s = sessions.get_by_name("test").unwrap();
        assert_eq!(s.id(), 3);
    }

    /// RULE-S12-03: ObjectDoesNotExist.
    #[test]
    fn test_object_does_not_exist() {
        let sessions = sample_sessions();
        let result = sessions.get_by_name("nonexistent");
        assert!(matches!(result, Err(QueryError::ObjectDoesNotExist { .. })));
    }

    /// RULE-S12-03: MultipleObjectsReturned.
    #[test]
    fn test_multiple_objects_returned() {
        let sessions = QueryList::new(vec![
            TestSession { id: 1, name: "dup".into() },
            TestSession { id: 2, name: "dup".into() },
        ]);
        let result = sessions.get_by_name("dup");
        assert!(matches!(result, Err(QueryError::MultipleObjectsReturned { count: 2, .. })));
    }
}
