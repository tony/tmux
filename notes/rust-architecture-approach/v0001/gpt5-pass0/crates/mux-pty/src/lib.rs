use std::marker::PhantomData;

/// Unified runtime PTY state shared between typestate and dynamic handles (S99, INV-013).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InnerPtyState {
    Allocated,
    Spawned,
    Running,
    Stopped,
    Exited,
    Reaped,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PtyError {
    SpawnFailed,
    InvalidTransition { from: InnerPtyState, to: InnerPtyState },
}

impl std::fmt::Display for PtyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SpawnFailed => f.write_str("pty spawn failed"),
            Self::InvalidTransition { from, to } => {
                write!(f, "invalid pty transition: {from:?} -> {to:?}")
            }
        }
    }
}

impl std::error::Error for PtyError {}

pub trait State {
    const VALUE: InnerPtyState;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Allocated;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spawned;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Running;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stopped;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Exited;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reaped;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Closed;

impl State for Allocated {
    const VALUE: InnerPtyState = InnerPtyState::Allocated;
}
impl State for Spawned {
    const VALUE: InnerPtyState = InnerPtyState::Spawned;
}
impl State for Running {
    const VALUE: InnerPtyState = InnerPtyState::Running;
}
impl State for Stopped {
    const VALUE: InnerPtyState = InnerPtyState::Stopped;
}
impl State for Exited {
    const VALUE: InnerPtyState = InnerPtyState::Exited;
}
impl State for Reaped {
    const VALUE: InnerPtyState = InnerPtyState::Reaped;
}
impl State for Closed {
    const VALUE: InnerPtyState = InnerPtyState::Closed;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PtyHandle<S: State> {
    id: u64,
    pid: Option<u32>,
    _state: PhantomData<S>,
}

impl PtyHandle<Allocated> {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            pid: None,
            _state: PhantomData,
        }
    }

    pub fn spawn(self, pid: u32) -> Result<PtyHandle<Spawned>, PtyError> {
        if pid == 0 {
            return Err(PtyError::SpawnFailed);
        }
        Ok(PtyHandle {
            id: self.id,
            pid: Some(pid),
            _state: PhantomData,
        })
    }
}

impl PtyHandle<Spawned> {
    pub fn start(self) -> PtyHandle<Running> {
        PtyHandle {
            id: self.id,
            pid: self.pid,
            _state: PhantomData,
        }
    }
}

impl PtyHandle<Running> {
    pub fn stop(self) -> PtyHandle<Stopped> {
        PtyHandle {
            id: self.id,
            pid: self.pid,
            _state: PhantomData,
        }
    }

    pub fn exit(self) -> PtyHandle<Exited> {
        PtyHandle {
            id: self.id,
            pid: self.pid,
            _state: PhantomData,
        }
    }
}

impl PtyHandle<Stopped> {
    pub fn exit(self) -> PtyHandle<Exited> {
        PtyHandle {
            id: self.id,
            pid: self.pid,
            _state: PhantomData,
        }
    }
}

impl PtyHandle<Exited> {
    pub fn reap(self) -> PtyHandle<Reaped> {
        PtyHandle {
            id: self.id,
            pid: self.pid,
            _state: PhantomData,
        }
    }
}

impl PtyHandle<Reaped> {
    pub fn close(self) -> PtyHandle<Closed> {
        PtyHandle {
            id: self.id,
            pid: self.pid,
            _state: PhantomData,
        }
    }
}

impl<S: State> PtyHandle<S> {
    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn pid(&self) -> Option<u32> {
        self.pid
    }

    pub fn state(&self) -> InnerPtyState {
        S::VALUE
    }

    pub fn into_dyn(self) -> DynPtyHandle {
        DynPtyHandle {
            id: self.id,
            pid: self.pid,
            state: S::VALUE,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynPtyHandle {
    id: u64,
    pid: Option<u32>,
    state: InnerPtyState,
}

impl DynPtyHandle {
    pub fn state(&self) -> InnerPtyState {
        self.state
    }

    pub fn transition(&mut self, to: InnerPtyState) -> Result<(), PtyError> {
        let ok = matches!(
            (self.state, to),
            (InnerPtyState::Allocated, InnerPtyState::Spawned)
                | (InnerPtyState::Spawned, InnerPtyState::Running)
                | (InnerPtyState::Running, InnerPtyState::Stopped)
                | (InnerPtyState::Running, InnerPtyState::Exited)
                | (InnerPtyState::Stopped, InnerPtyState::Exited)
                | (InnerPtyState::Exited, InnerPtyState::Reaped)
                | (InnerPtyState::Reaped, InnerPtyState::Closed)
        );
        if !ok {
            return Err(PtyError::InvalidTransition {
                from: self.state,
                to,
            });
        }
        self.state = to;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typestate_lifecycle_is_monotonic() {
        let allocated = PtyHandle::<Allocated>::new(7);
        let spawned = allocated.spawn(1234).unwrap();
        let running = spawned.start();
        let exited = running.exit();
        let reaped = exited.reap();
        let closed = reaped.close();
        assert_eq!(closed.state(), InnerPtyState::Closed);
    }

    #[test]
    fn spawn_rejects_zero_pid() {
        let allocated = PtyHandle::<Allocated>::new(7);
        assert!(matches!(allocated.spawn(0), Err(PtyError::SpawnFailed)));
    }

    #[test]
    fn dyn_transition_valid_path() {
        let mut d = PtyHandle::<Allocated>::new(1).into_dyn();
        d.transition(InnerPtyState::Spawned).unwrap();
        d.transition(InnerPtyState::Running).unwrap();
        d.transition(InnerPtyState::Exited).unwrap();
        d.transition(InnerPtyState::Reaped).unwrap();
        d.transition(InnerPtyState::Closed).unwrap();
        assert_eq!(d.state(), InnerPtyState::Closed);
    }

    #[test]
    fn dyn_transition_invalid_path_rejected() {
        let mut d = PtyHandle::<Allocated>::new(1).into_dyn();
        let err = d.transition(InnerPtyState::Running).unwrap_err();
        assert!(matches!(err, PtyError::InvalidTransition { .. }));
    }
}
