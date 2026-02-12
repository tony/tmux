//! # mux-pty
//!
//! PTY (pseudo-terminal) handle with typestate lifecycle management.
//!
//! ## Key Design
//! - S99: Unified `InnerPtyState` enum with 7 states.
//! - S82: `PtyHandle<S>` typestate pattern for compile-time lifecycle safety.
//! - `DynPtyHandle` for FFI boundary (runtime state tracking).
//! - INV-013: Lifecycle is monotonic (no going back to earlier states).
//! - INV-003: This is the ONLY crate allowed to use `unsafe`.
//!
//! ## States
//! Created -> Spawning -> Running -> Paused -> Killing -> Closed -> Error

// NOTE: mux-pty is the ONLY crate that may use unsafe (INV-003).
// We do NOT set forbid(unsafe_code) here.

use std::fmt;
use std::marker::PhantomData;

/// S99: InnerPtyState -- unified 7-state enum for PTY lifecycle.
///
/// RULE-S07-01: Exactly 7 states.
/// INV-013: Lifecycle transitions are monotonic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InnerPtyState {
    Created,
    Spawning,
    Running,
    Paused,
    Killing,
    Closed,
    Error,
}

impl InnerPtyState {
    /// Ordinal for monotonicity checks.
    /// INV-013: Each state has a strictly increasing ordinal
    /// (except Error, which can be reached from any state).
    fn ordinal(self) -> u8 {
        match self {
            Self::Created => 0,
            Self::Spawning => 1,
            Self::Running => 2,
            Self::Paused => 3,
            Self::Killing => 4,
            Self::Closed => 5,
            Self::Error => 255, // can be reached from anywhere
        }
    }

    /// Check if transitioning to `next` is valid.
    pub fn can_transition_to(self, next: Self) -> bool {
        if next == Self::Error {
            return true; // Error is reachable from any state
        }
        if self == Self::Error || self == Self::Closed {
            return false; // Terminal states
        }
        next.ordinal() > self.ordinal()
    }

    /// Label for display.
    pub fn label(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Spawning => "spawning",
            Self::Running => "running",
            Self::Paused => "paused",
            Self::Killing => "killing",
            Self::Closed => "closed",
            Self::Error => "error",
        }
    }
}

/// PTY handle error type.
///
/// INV-005: Implements Error + Send + Sync + 'static.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PtyHandleError {
    /// The handle has been closed.
    HandleClosed,
    /// Invalid state transition attempted.
    InvalidTransition { from: InnerPtyState, to: InnerPtyState },
    /// Generation counter mismatch (ABA prevention).
    GenerationMismatch { expected: u64, actual: u64 },
    /// The PTY process exited unexpectedly.
    ProcessExited { exit_code: Option<i32> },
    /// Platform-level error.
    PlatformError(String),
}

impl fmt::Display for PtyHandleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HandleClosed => write!(f, "pty handle is closed"),
            Self::InvalidTransition { from, to } =>
                write!(f, "invalid pty transition: {} -> {}", from.label(), to.label()),
            Self::GenerationMismatch { expected, actual } =>
                write!(f, "pty generation mismatch: expected {expected}, got {actual}"),
            Self::ProcessExited { exit_code } =>
                write!(f, "pty process exited with code {exit_code:?}"),
            Self::PlatformError(msg) => write!(f, "pty platform error: {msg}"),
        }
    }
}

impl std::error::Error for PtyHandleError {}

// INV-005: Compile-time check
const _: fn() = || {
    fn assert_send_sync<T: Send + Sync + 'static>() {}
    assert_send_sync::<PtyHandleError>();
};

// --- Typestate markers ---

/// Typestate: the PTY has been created but not yet spawned.
#[derive(Debug)]
pub struct Created;
/// Typestate: the PTY has a running child process.
#[derive(Debug)]
pub struct Running;
/// Typestate: the PTY is being killed.
#[derive(Debug)]
pub struct Killing;
/// Typestate: the PTY has been closed.
#[derive(Debug)]
pub struct Closed;

/// S82: Typestate `PtyHandle<S>` for compile-time lifecycle safety.
///
/// The type parameter `S` tracks the current state at compile time.
/// Transitions consume `self` and return a new handle in the next state.
#[derive(Debug)]
pub struct PtyHandle<S> {
    /// Unique identifier for this handle.
    id: u64,
    /// Generation counter for ABA prevention (RULE-S07-06).
    generation: u64,
    /// File descriptor of the PTY master (platform-specific).
    master_fd: Option<i32>,
    /// Child process ID.
    child_pid: Option<u32>,
    /// Phantom data for the typestate.
    _state: PhantomData<S>,
}

impl PtyHandle<Created> {
    /// Create a new PTY handle in the Created state.
    pub fn new(id: u64) -> Self {
        Self {
            id,
            generation: 0,
            master_fd: None,
            child_pid: None,
            _state: PhantomData,
        }
    }

    /// Spawn a child process, transitioning to Running state.
    ///
    /// In a real implementation, this would call `openpty(2)` + `fork(2)`.
    #[must_use]
    pub fn spawn(self, _cmd: &str, _args: &[&str]) -> Result<PtyHandle<Running>, PtyHandleError> {
        // In production, this would do the actual PTY/fork work.
        Ok(PtyHandle {
            id: self.id,
            generation: self.generation + 1,
            master_fd: Some(42), // placeholder FD
            child_pid: Some(12345), // placeholder PID
            _state: PhantomData,
        })
    }
}

impl PtyHandle<Running> {
    /// The child process ID.
    pub fn pid(&self) -> Option<u32> {
        self.child_pid
    }

    /// The master FD.
    pub fn master_fd(&self) -> Option<i32> {
        self.master_fd
    }

    /// Kill the child process, transitioning to Killing state.
    #[must_use]
    pub fn kill(self) -> Result<PtyHandle<Killing>, PtyHandleError> {
        // In production, this would send SIGKILL.
        Ok(PtyHandle {
            id: self.id,
            generation: self.generation + 1,
            master_fd: self.master_fd,
            child_pid: self.child_pid,
            _state: PhantomData,
        })
    }
}

impl PtyHandle<Killing> {
    /// Close the PTY, transitioning to Closed state.
    ///
    /// RULE-S07-09: Restart barrier enforced (kill -> close -> spawn).
    pub fn close(self) -> PtyHandle<Closed> {
        PtyHandle {
            id: self.id,
            generation: self.generation + 1,
            master_fd: None,
            child_pid: None,
            _state: PhantomData,
        }
    }
}

impl<S> PtyHandle<S> {
    /// The unique handle ID.
    pub fn id(&self) -> u64 { self.id }
    /// The generation counter (RULE-S07-06).
    pub fn generation(&self) -> u64 { self.generation }
}

/// DynPtyHandle: runtime-tracked PTY handle for FFI boundary.
///
/// RULE-S07-04: DynPtyHandle uses runtime state enum instead of typestate.
/// S82: TryFrom<DynPtyHandle> for typed recovery.
#[derive(Debug)]
pub struct DynPtyHandle {
    id: u64,
    generation: u64,
    state: InnerPtyState,
    master_fd: Option<i32>,
    child_pid: Option<u32>,
}

impl DynPtyHandle {
    /// Create a new DynPtyHandle in Created state.
    pub fn new(id: u64) -> Self {
        Self {
            id,
            generation: 0,
            state: InnerPtyState::Created,
            master_fd: None,
            child_pid: None,
        }
    }

    /// Current state.
    pub fn state(&self) -> InnerPtyState { self.state }

    /// Unique ID.
    pub fn id(&self) -> u64 { self.id }

    /// Generation counter.
    pub fn generation(&self) -> u64 { self.generation }

    /// Transition to a new state (with runtime validation).
    #[must_use]
    pub fn transition(&mut self, new_state: InnerPtyState) -> Result<(), PtyHandleError> {
        if !self.state.can_transition_to(new_state) {
            return Err(PtyHandleError::InvalidTransition {
                from: self.state,
                to: new_state,
            });
        }
        self.state = new_state;
        self.generation += 1;
        Ok(())
    }

    /// Validate that the generation matches expected (ABA prevention).
    /// RULE-S07-06.
    #[must_use]
    pub fn validate_generation(&self, expected: u64) -> Result<(), PtyHandleError> {
        if self.generation != expected {
            return Err(PtyHandleError::GenerationMismatch {
                expected,
                actual: self.generation,
            });
        }
        Ok(())
    }
}

/// S82: TryFrom<DynPtyHandle> for typed recovery.
impl TryFrom<DynPtyHandle> for PtyHandle<Running> {
    type Error = PtyHandleError;
    fn try_from(dyn_handle: DynPtyHandle) -> Result<Self, Self::Error> {
        if dyn_handle.state != InnerPtyState::Running {
            return Err(PtyHandleError::InvalidTransition {
                from: dyn_handle.state,
                to: InnerPtyState::Running,
            });
        }
        Ok(PtyHandle {
            id: dyn_handle.id,
            generation: dyn_handle.generation,
            master_fd: dyn_handle.master_fd,
            child_pid: dyn_handle.child_pid,
            _state: PhantomData,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RULE-S07-01: InnerPtyState has 7 states.
    #[test]
    fn test_seven_states() {
        let states = [
            InnerPtyState::Created,
            InnerPtyState::Spawning,
            InnerPtyState::Running,
            InnerPtyState::Paused,
            InnerPtyState::Killing,
            InnerPtyState::Closed,
            InnerPtyState::Error,
        ];
        assert_eq!(states.len(), 7);
    }

    /// INV-013: Lifecycle transitions are monotonic.
    #[test]
    fn test_monotonic_transitions() {
        assert!(InnerPtyState::Created.can_transition_to(InnerPtyState::Running));
        assert!(InnerPtyState::Running.can_transition_to(InnerPtyState::Killing));
        assert!(!InnerPtyState::Running.can_transition_to(InnerPtyState::Created));
        assert!(!InnerPtyState::Closed.can_transition_to(InnerPtyState::Running));
    }

    /// Error is reachable from any state.
    #[test]
    fn test_error_reachable_from_any() {
        for state in [
            InnerPtyState::Created, InnerPtyState::Running,
            InnerPtyState::Paused, InnerPtyState::Killing,
        ] {
            assert!(state.can_transition_to(InnerPtyState::Error));
        }
    }

    /// Typestate lifecycle: Created -> Running -> Killing -> Closed.
    #[test]
    fn test_typestate_lifecycle() {
        let handle = PtyHandle::<Created>::new(1);
        assert_eq!(handle.id(), 1);
        assert_eq!(handle.generation(), 0);

        let running = handle.spawn("/bin/sh", &[]).unwrap();
        assert_eq!(running.generation(), 1);

        let killing = running.kill().unwrap();
        assert_eq!(killing.generation(), 2);

        let closed = killing.close();
        assert_eq!(closed.generation(), 3);
    }

    /// RULE-S07-06: Generation counter prevents ABA.
    #[test]
    fn test_generation_validation() {
        let mut dyn_h = DynPtyHandle::new(42);
        assert!(dyn_h.validate_generation(0).is_ok());
        dyn_h.transition(InnerPtyState::Running).unwrap();
        assert!(dyn_h.validate_generation(0).is_err()); // stale generation
        assert!(dyn_h.validate_generation(1).is_ok());
    }
}
