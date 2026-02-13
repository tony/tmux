use nix::pty::{openpty, Winsize};
use std::os::fd::{OwnedFd, AsRawFd, BorrowedFd};

pub struct Pty {
    pub master: OwnedFd,
    pub slave: OwnedFd,
}

impl Pty {
    pub fn new() -> anyhow::Result<Self> {
        let res = openpty(None, None)?;
        Ok(Self {
            master: res.master,
            slave: res.slave,
        })
    }

    pub fn resize(&self, cols: u16, rows: u16) -> anyhow::Result<()> {
        let ws = libc::winsize {
            ws_row: rows,
            ws_col: cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        // SAFETY: Simple ioctl call
        let res = unsafe {
            libc::ioctl(self.master.as_raw_fd(), libc::TIOCSWINSZ, &ws)
        };
        if res < 0 {
            return Err(anyhow::Error::from(std::io::Error::last_os_error()));
        }
        Ok(())
    }
}

pub struct RawModeGuard {
    original_termios: nix::sys::termios::Termios,
    fd: std::os::fd::RawFd,
}

impl RawModeGuard {
    pub fn new(fd: std::os::fd::RawFd) -> anyhow::Result<Self> {
        // SAFETY: fd is assumed valid by caller
        let borrowed_fd = unsafe { BorrowedFd::borrow_raw(fd) };
        let original_termios = nix::sys::termios::tcgetattr(borrowed_fd)?;
        let mut raw = original_termios.clone();
        nix::sys::termios::cfmakeraw(&mut raw);
        nix::sys::termios::tcsetattr(borrowed_fd, nix::sys::termios::SetArg::TCSANOW, &raw)?;
        Ok(Self { original_termios, fd })
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        // SAFETY: fd is assumed valid
        let borrowed_fd = unsafe { BorrowedFd::borrow_raw(self.fd) };
        let _ = nix::sys::termios::tcsetattr(borrowed_fd, nix::sys::termios::SetArg::TCSANOW, &self.original_termios);
    }
}
