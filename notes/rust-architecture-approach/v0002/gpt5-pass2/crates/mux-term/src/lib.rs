#![forbid(unsafe_code)]

use crossbeam_channel::{Receiver, Sender};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalEvent {
    SigWinch,
    SigChld,
    Terminate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolEvent {
    ResizeBroadcast,
    ReapChildren,
    OrderedShutdown,
}

#[derive(Debug)]
pub struct SigwinchFlow {
    pub tx: Sender<ProtocolEvent>,
}

impl SigwinchFlow {
    pub fn on_sigwinch(&self) {
        let _ = self.tx.send(ProtocolEvent::ResizeBroadcast);
    }
}

#[derive(Debug)]
pub struct ZombieReaper {
    pub tx: Sender<ProtocolEvent>,
}

impl ZombieReaper {
    pub fn on_sigchld(&self) {
        // Non-interference: only queues reap request.
        let _ = self.tx.send(ProtocolEvent::ReapChildren);
    }
}

#[must_use]
pub fn propagate_signal(signal: SignalEvent) -> ProtocolEvent {
    match signal {
        SignalEvent::SigWinch => ProtocolEvent::ResizeBroadcast,
        SignalEvent::SigChld => ProtocolEvent::ReapChildren,
        SignalEvent::Terminate => ProtocolEvent::OrderedShutdown,
    }
}

#[must_use]
pub fn run_signal_bridge(rx: Receiver<SignalEvent>, tx: Sender<ProtocolEvent>) -> usize {
    let mut processed = 0usize;
    while let Ok(sig) = rx.try_recv() {
        let _ = tx.send(propagate_signal(sig));
        processed += 1;
    }
    processed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam_channel::unbounded;

    #[test]
    fn sigwinch_maps_to_resize() {
        assert_eq!(propagate_signal(SignalEvent::SigWinch), ProtocolEvent::ResizeBroadcast);
    }

    #[test]
    fn sigchld_maps_to_reap() {
        assert_eq!(propagate_signal(SignalEvent::SigChld), ProtocolEvent::ReapChildren);
    }

    #[test]
    fn terminate_maps_to_shutdown() {
        assert_eq!(propagate_signal(SignalEvent::Terminate), ProtocolEvent::OrderedShutdown);
    }

    #[test]
    fn sigwinch_flow_sends_event() {
        let (tx, rx) = unbounded();
        SigwinchFlow { tx }.on_sigwinch();
        assert_eq!(rx.recv().expect("event"), ProtocolEvent::ResizeBroadcast);
    }

    #[test]
    fn zombie_reaper_sends_reap_request() {
        let (tx, rx) = unbounded();
        ZombieReaper { tx }.on_sigchld();
        assert_eq!(rx.recv().expect("event"), ProtocolEvent::ReapChildren);
    }

    #[test]
    fn bridge_processes_multiple_signals() {
        let (stx, srx) = unbounded();
        let (ptx, prx) = unbounded();
        stx.send(SignalEvent::SigWinch).expect("send");
        stx.send(SignalEvent::SigChld).expect("send");
        let n = run_signal_bridge(srx, ptx);
        assert_eq!(n, 2);
        assert_eq!(prx.recv().expect("event"), ProtocolEvent::ResizeBroadcast);
    }
}
