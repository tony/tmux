use mux_kernel::{Kernel};
use mux_term::IoThread;
use mux_protocol::{KernelEvent};
use crossbeam_channel::{unbounded, Sender};
use std::thread;

pub struct TermForge {
    pub kernel_tx: Sender<KernelEvent>,
}

pub struct Builder {
    // Config options
    pub default_shell: String,
    pub raw_mode: bool,
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            default_shell: "/bin/bash".into(),
            raw_mode: true,
        }
    }
}

impl Builder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_shell(mut self, shell: &str) -> Self {
        self.default_shell = shell.into();
        self
    }

    pub fn with_raw_mode(mut self, enable: bool) -> Self {
        self.raw_mode = enable;
        self
    }

    pub fn build(self) -> TermForge {
        let (tx, rx) = unbounded();

        // 1. Initialize Kernel
        let mut kernel = Kernel::new(rx);
        
        // 2. Initialize IO
        let io_thread = IoThread::new(tx.clone());
        io_thread.start();

        // 3. Start Kernel Thread
        thread::spawn(move || {
            kernel.run();
        });

        Self::set_panic_hook();

        TermForge {
            kernel_tx: tx,
        }
    }

    fn set_panic_hook() {
        let default_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            // 1. Restore Terminal Mode (Critical!)
            // In a real impl, we'd have a static reference or use the raw fd
            // unsafe { libc::tcsetattr(...) };
            println!("\x1b[?1049l"); // Switch back to main screen
            println!("\x1b[?25h");   // Show cursor

            // 2. Print error
            default_hook(info);
            
            // 3. Dump state (optional)
        }));
    }
}

impl TermForge {
    pub fn builder() -> Builder {
        Builder::new()
    }

    pub fn resize(&self, width: u16, height: u16) {
        self.kernel_tx.send(KernelEvent::Resize(width, height)).unwrap();
    }
    
    // API methods for external callers (embedding)
}
