use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use crossbeam_channel::Sender;
use signal_hook::iterator::Signals;
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use mux_protocol::KernelEvent;

pub struct IoThread {
    pub kernel_tx: Sender<KernelEvent>,
    pub running: Arc<Mutex<bool>>,
}

impl IoThread {
    pub fn new(kernel_tx: Sender<KernelEvent>) -> Self {
        Self {
            kernel_tx,
            running: Arc::new(Mutex::new(true)),
        }
    }

    pub fn start(&self) {
        let tx = self.kernel_tx.clone();
        let running = self.running.clone();

        thread::spawn(move || {
            // Setup Signal Handling
            let mut signals = Signals::new(&[
                signal_hook::consts::SIGWINCH,
                signal_hook::consts::SIGCHLD,
                signal_hook::consts::SIGTERM,
                signal_hook::consts::SIGINT,
            ]).unwrap();

            while *running.lock().unwrap() {
                for signal in signals.pending() {
                    match signal {
                        signal_hook::consts::SIGWINCH => {
                            // Debounce logic would go here
                            // Use ioctl(TIOCGWINSZ) to get new size
                            // For now, hardcode stub
                            tx.send(KernelEvent::Resize(80, 24)).unwrap();
                        },
                        signal_hook::consts::SIGCHLD => {
                            // Zombie Reaping
                            loop {
                                match waitpid(None, Some(WaitPidFlag::WNOHANG)) {
                                    Ok(WaitStatus::Exited(_pid, _status)) => {
                                        // Map Pid to PaneId (needs shared map)
                                        // For now, just log
                                        // tx.send(KernelEvent::PaneExited(0, status)).unwrap();
                                    },
                                    Ok(WaitStatus::Signaled(_pid, _sig, _)) => {
                                        // Handle killed by signal
                                    },
                                    Ok(WaitStatus::StillAlive) => break,
                                    Err(_) => break, // ECHILD or other error
                                    _ => break,
                                }
                            }
                        },
                        signal_hook::consts::SIGTERM | signal_hook::consts::SIGINT => {
                            // Graceful shutdown
                            *running.lock().unwrap() = false;
                        },
                        _ => unreachable!(),
                    }
                }
                
                // Event Loop Sleep (Simulated)
                thread::sleep(Duration::from_millis(10));
            }
        });
    }
}
