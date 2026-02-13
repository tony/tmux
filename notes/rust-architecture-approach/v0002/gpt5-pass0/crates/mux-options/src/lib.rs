use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OptionScope {
    Server,
    Session,
    Window,
    Pane,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OptionKey(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptionValue {
    Bool(bool),
    Number(i64),
    Text(String),
}

#[derive(Debug, Error)]
pub enum OptionError {
    #[error("option not found: scope={scope:?}, key={key}")]
    NotFound { scope: OptionScope, key: String },
}

#[derive(Debug, Default)]
pub struct OptionStore {
    values: HashMap<(OptionScope, OptionKey), OptionValue>,
}

impl OptionStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, scope: OptionScope, key: impl Into<String>, value: OptionValue) {
        self.values.insert((scope, OptionKey(key.into())), value);
    }

    pub fn get(&self, scope: OptionScope, key: &str) -> Result<&OptionValue, OptionError> {
        self.values
            .get(&(scope, OptionKey(key.to_owned())))
            .ok_or_else(|| OptionError::NotFound {
                scope,
                key: key.to_owned(),
            })
    }
}
