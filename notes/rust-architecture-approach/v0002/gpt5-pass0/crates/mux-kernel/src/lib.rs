use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;

use mux_grid::Grid;
use mux_types::{Cell, ChannelCaps, PaneId, Point, Size};
use thiserror::Error;
use tracing::error;

#[derive(Debug, Clone)]
pub struct KernelConfig {
    pub channels: ChannelCaps,
}

impl Default for KernelConfig {
    fn default() -> Self {
        Self {
            channels: ChannelCaps::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum KernelCommand {
    CreatePane { pane_id: PaneId, size: Size },
    PutCell { pane_id: PaneId, at: Point, cell: Cell },
    Shutdown,
}

#[derive(Debug, Clone)]
pub enum KernelEvent {
    PaneCreated(PaneId),
    GridUpdated { pane_id: PaneId, revision: u64 },
    PanicRecovered,
    Stopped,
}

#[derive(Debug, Error)]
pub enum KernelError {
    #[error("kernel channel closed")]
    ChannelClosed,
}

#[derive(Debug)]
pub struct KernelHandle {
    command_tx: mpsc::Sender<KernelCommand>,
    event_rx: mpsc::Receiver<KernelEvent>,
}

impl KernelHandle {
    pub fn send(&self, command: KernelCommand) -> Result<(), KernelError> {
        self.command_tx
            .send(command)
            .map_err(|_| KernelError::ChannelClosed)
    }

    pub fn recv(&self) -> Result<KernelEvent, KernelError> {
        self.event_rx.recv().map_err(|_| KernelError::ChannelClosed)
    }
}

#[derive(Debug, Default)]
struct KernelState {
    panes: HashMap<PaneId, Grid>,
}

pub fn spawn_kernel(config: KernelConfig) -> KernelHandle {
    let (command_tx, command_rx) = mpsc::channel::<KernelCommand>();
    let (event_tx, event_rx) = mpsc::channel::<KernelEvent>();
    let panic_flag = Arc::new(AtomicBool::new(false));

    let panic_flag_worker = panic_flag.clone();
    thread::Builder::new()
        .name("termforge-kernel".to_owned())
        .spawn(move || {
            let result = std::panic::catch_unwind(move || run_kernel(command_rx, event_tx, config));
            if result.is_err() {
                panic_flag_worker.store(true, Ordering::SeqCst);
                error!("kernel panic recovered");
            }
        })
        .expect("failed to spawn kernel thread");

    if panic_flag.load(Ordering::SeqCst) {
        let _ = command_tx.send(KernelCommand::Shutdown);
    }

    KernelHandle {
        command_tx,
        event_rx,
    }
}

fn run_kernel(
    command_rx: mpsc::Receiver<KernelCommand>,
    event_tx: mpsc::Sender<KernelEvent>,
    _config: KernelConfig,
) {
    let mut state = KernelState::default();

    while let Ok(command) = command_rx.recv() {
        match command {
            KernelCommand::CreatePane { pane_id, size } => {
                state.panes.insert(pane_id, Grid::new(size));
                let _ = event_tx.send(KernelEvent::PaneCreated(pane_id));
            }
            KernelCommand::PutCell { pane_id, at, cell } => {
                if let Some(grid) = state.panes.get_mut(&pane_id)
                    && grid.put_cell(at, cell).is_ok()
                {
                    let _ = event_tx.send(KernelEvent::GridUpdated {
                        pane_id,
                        revision: grid.revision(),
                    });
                }
            }
            KernelCommand::Shutdown => {
                let _ = event_tx.send(KernelEvent::Stopped);
                break;
            }
        }
    }
}
