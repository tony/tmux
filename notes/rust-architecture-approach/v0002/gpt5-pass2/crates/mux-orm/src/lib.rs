//! # mux-orm
//!
//! ORM-like query interface for terminal multiplexer objects
//! (sessions, windows, panes).
//!
//! ## Key Design
//! - `QueryList<T>`: typed collection with filter/get semantics.
//! - `Queryable` trait: objects that can be queried by name or ID.
//! - `QueryError`: ObjectDoesNotExist, MultipleObjectsReturned.

#![forbid(unsafe_code)]

use std::fmt;

/// Ergonomic re-exports for downstream crates.
pub mod prelude {
    pub use super::{QueryError, QueryList, Queryable};
}

/// Error type for query operations.
///
/// RULE-S12-03: Two error variants for exact-match queries.
/// INV-005: Implements Error + Send + Sync + 'static.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryError {
    ObjectDoesNotExist { query: String },
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
const _: () = {
    fn assert_send_sync<T: Send + Sync + 'static>() {}
    fn check() { assert_send_sync::<QueryError>(); }
};

/// Trait for objects that can be queried by name or ID.
pub trait Queryable {
    fn id(&self) -> u64;
    fn name(&self) -> &str;
}

/// An ORM-like query list supporting filter and get operations.
///
/// Provides Django-style `.filter()` and `.get()` semantics.
#[derive(Debug, Clone)]
pub struct QueryList<T> {
    items: Vec<T>,
}

impl<T: Queryable + Clone> QueryList<T> {
    #[must_use]
    pub fn new(items: Vec<T>) -> Self {
        Self { items }
    }

    #[must_use]
    pub fn empty() -> Self {
        Self { items: Vec::new() }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    #[must_use]
    pub fn all(&self) -> &[T] {
        &self.items
    }

    /// Filter items by a predicate, returning a new QueryList.
    #[must_use]
    pub fn filter<F: Fn(&T) -> bool>(&self, predicate: F) -> QueryList<T> {
        QueryList {
            items: self.items.iter().filter(|item| predicate(item)).cloned().collect(),
        }
    }

    /// Get exactly one item matching the predicate.
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

    /// Count items matching a predicate.
    #[must_use]
    pub fn count<F: Fn(&T) -> bool>(&self, predicate: F) -> usize {
        self.items.iter().filter(|item| predicate(item)).count()
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

    #[test]
    fn test_object_does_not_exist() {
        let sessions = sample_sessions();
        let result = sessions.get_by_name("nonexistent");
        assert!(matches!(result, Err(QueryError::ObjectDoesNotExist { .. })));
    }

    #[test]
    fn test_multiple_objects_returned() {
        let sessions = QueryList::new(vec![
            TestSession { id: 1, name: "dup".into() },
            TestSession { id: 2, name: "dup".into() },
        ]);
        let result = sessions.get_by_name("dup");
        assert!(matches!(result, Err(QueryError::MultipleObjectsReturned { count: 2, .. })));
    }

    #[test]
    fn test_first() {
        let sessions = sample_sessions();
        let s = sessions.first().unwrap();
        assert_eq!(s.id(), 1);
    }

    #[test]
    fn test_first_empty() {
        let sessions: QueryList<TestSession> = QueryList::empty();
        assert!(sessions.first().is_err());
    }

    #[test]
    fn test_push() {
        let mut list = QueryList::<TestSession>::empty();
        list.push(TestSession { id: 99, name: "added".into() });
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn test_count() {
        let sessions = sample_sessions();
        assert_eq!(sessions.count(|s| s.id() > 1), 2);
    }

    #[test]
    fn test_into_iter() {
        let sessions = sample_sessions();
        let names: Vec<String> = sessions.into_iter().map(|s| s.name).collect();
        assert_eq!(names, vec!["main", "work", "test"]);
    }

    #[test]
    fn test_default_is_empty() {
        let list: QueryList<TestSession> = QueryList::default();
        assert!(list.is_empty());
    }

    #[test]
    fn test_get_on_empty_list_returns_object_does_not_exist() {
        let empty: QueryList<TestSession> = QueryList::empty();
        let result = empty.get(|_| true, "any");
        assert!(matches!(result, Err(QueryError::ObjectDoesNotExist { .. })));
    }

    #[test]
    fn test_get_by_id_multiple_matches_returns_multiple_error() {
        let sessions = QueryList::new(vec![
            TestSession { id: 7, name: "a".into() },
            TestSession { id: 7, name: "b".into() },
        ]);
        let result = sessions.get_by_id(7);
        assert!(matches!(result, Err(QueryError::MultipleObjectsReturned { count: 2, .. })));
    }

    #[test]
    fn test_filter_on_empty_list_stays_empty() {
        let empty: QueryList<TestSession> = QueryList::empty();
        let filtered = empty.filter(|s| s.id == 1);
        assert!(filtered.is_empty());
    }
}
