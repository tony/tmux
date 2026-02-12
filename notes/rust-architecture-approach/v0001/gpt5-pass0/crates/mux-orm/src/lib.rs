#![forbid(unsafe_code)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryError {
    ObjectDoesNotExist,
    MultipleObjectsReturned,
}

impl std::fmt::Display for QueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ObjectDoesNotExist => f.write_str("object does not exist"),
            Self::MultipleObjectsReturned => f.write_str("multiple objects returned"),
        }
    }
}

impl std::error::Error for QueryError {}

/// ORM-like immutable query wrapper.
#[derive(Debug, Clone)]
pub struct QueryList<T> {
    items: Vec<T>,
}

impl<T> QueryList<T> {
    pub fn new(items: Vec<T>) -> Self {
        Self { items }
    }

    pub fn count(&self) -> usize {
        self.items.len()
    }

    pub fn all(&self) -> &[T] {
        &self.items
    }

    pub fn filter<F>(&self, mut predicate: F) -> Self
    where
        F: FnMut(&T) -> bool,
        T: Clone,
    {
        Self {
            items: self.items.iter().filter(|x| predicate(x)).cloned().collect(),
        }
    }

    pub fn exclude<F>(&self, mut predicate: F) -> Self
    where
        F: FnMut(&T) -> bool,
        T: Clone,
    {
        Self {
            items: self.items.iter().filter(|x| !predicate(x)).cloned().collect(),
        }
    }

    pub fn first(&self) -> Option<&T> {
        self.items.first()
    }

    pub fn get_single(&self) -> Result<&T, QueryError> {
        match self.items.len() {
            0 => Err(QueryError::ObjectDoesNotExist),
            1 => Ok(&self.items[0]),
            _ => Err(QueryError::MultipleObjectsReturned),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_and_count() {
        let q = QueryList::new(vec![1, 2, 3, 4]);
        let evens = q.filter(|x| *x % 2 == 0);
        assert_eq!(evens.count(), 2);
    }

    #[test]
    fn get_single_success() {
        let q = QueryList::new(vec![42]);
        assert_eq!(q.get_single().unwrap(), &42);
    }

    #[test]
    fn get_single_object_missing() {
        let q = QueryList::<u8>::new(vec![]);
        assert!(matches!(q.get_single(), Err(QueryError::ObjectDoesNotExist)));
    }

    #[test]
    fn get_single_multiple_returned() {
        let q = QueryList::new(vec![1, 2]);
        assert!(matches!(q.get_single(), Err(QueryError::MultipleObjectsReturned)));
    }
}
