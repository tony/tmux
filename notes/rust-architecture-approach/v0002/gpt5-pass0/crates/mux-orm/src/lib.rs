use mux_kernel::{KernelCommand, KernelHandle};
use mux_types::{PaneId, SessionId, Size, WindowId};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub id: SessionId,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Window {
    pub id: WindowId,
    pub session_id: SessionId,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pane {
    pub id: PaneId,
    pub window_id: WindowId,
}

#[derive(Debug, Error)]
pub enum OrmError {
    #[error("record not found")]
    NotFound,
    #[error("multiple records found")]
    Multiple,
    #[error("kernel operation failed")]
    Kernel,
}

#[derive(Debug, Clone)]
pub struct QuerySet<T> {
    items: Vec<T>,
}

impl<T> QuerySet<T>
where
    T: Clone,
{
    #[must_use]
    pub fn new(items: Vec<T>) -> Self {
        Self { items }
    }

    #[must_use]
    pub fn filter<F>(&self, predicate: F) -> Self
    where
        F: Fn(&T) -> bool,
    {
        Self {
            items: self.items.iter().filter(|item| predicate(item)).cloned().collect(),
        }
    }

    #[must_use]
    pub fn where_(self, predicate: impl Fn(&T) -> bool) -> Self {
        self.filter(predicate)
    }

    pub fn get(&self, predicate: impl Fn(&T) -> bool) -> Result<T, OrmError> {
        let mut matches = self.items.iter().filter(|item| predicate(item));
        let first = matches.next().cloned().ok_or(OrmError::NotFound)?;
        if matches.next().is_some() {
            return Err(OrmError::Multiple);
        }
        Ok(first)
    }

    #[must_use]
    pub fn all(&self) -> Vec<T> {
        self.items.clone()
    }
}

pub struct Server {
    kernel: KernelHandle,
    sessions: Vec<Session>,
    windows: Vec<Window>,
    panes: Vec<Pane>,
}

impl Server {
    #[must_use]
    pub fn new(kernel: KernelHandle) -> Self {
        Self {
            kernel,
            sessions: Vec::new(),
            windows: Vec::new(),
            panes: Vec::new(),
        }
    }

    #[must_use]
    pub fn sessions(&self) -> QuerySet<Session> {
        QuerySet::new(self.sessions.clone())
    }

    #[must_use]
    pub fn windows(&self) -> QuerySet<Window> {
        QuerySet::new(self.windows.clone())
    }

    #[must_use]
    pub fn panes(&self) -> QuerySet<Pane> {
        QuerySet::new(self.panes.clone())
    }

    pub fn create_pane(&self, pane_id: PaneId, size: Size) -> Result<(), OrmError> {
        self.kernel
            .send(KernelCommand::CreatePane { pane_id, size })
            .map_err(|_| OrmError::Kernel)
    }
}
