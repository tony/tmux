#![forbid(unsafe_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BorrowedFd(pub i32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FdEnvelope {
    pub command: u16,
    pub seq: u64,
    pub fds: Vec<BorrowedFd>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FdPassError {
    TooManyFds,
    EmptyPayload,
    InvalidHeader,
}

impl FdEnvelope {
    #[must_use]
    pub fn new(command: u16, seq: u64) -> Self {
        Self { command, seq, fds: Vec::new() }
    }

    pub fn push_fd(&mut self, fd: BorrowedFd) -> Result<(), FdPassError> {
        if self.fds.len() >= 16 {
            return Err(FdPassError::TooManyFds);
        }
        self.fds.push(fd);
        Ok(())
    }

    #[must_use]
    pub fn encode_header(&self) -> [u8; 12] {
        let mut out = [0u8; 12];
        out[0..2].copy_from_slice(&self.command.to_be_bytes());
        out[2..4].copy_from_slice(&(self.fds.len() as u16).to_be_bytes());
        out[4..12].copy_from_slice(&self.seq.to_be_bytes());
        out
    }

    pub fn decode_header(input: &[u8]) -> Result<(u16, u16, u64), FdPassError> {
        if input.len() < 12 {
            return Err(FdPassError::InvalidHeader);
        }
        let cmd = u16::from_be_bytes([input[0], input[1]]);
        let count = u16::from_be_bytes([input[2], input[3]]);
        let mut seq = [0u8; 8];
        seq.copy_from_slice(&input[4..12]);
        Ok((cmd, count, u64::from_be_bytes(seq)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_round_trip() {
        let mut e = FdEnvelope::new(9, 42);
        e.push_fd(BorrowedFd(3)).expect("fd");
        let h = e.encode_header();
        let (cmd, count, seq) = FdEnvelope::decode_header(&h).expect("decode");
        assert_eq!((cmd, count, seq), (9, 1, 42));
    }

    #[test]
    fn enforces_fd_limit() {
        let mut e = FdEnvelope::new(1, 0);
        for i in 0..16 {
            e.push_fd(BorrowedFd(i)).expect("fd");
        }
        assert!(matches!(e.push_fd(BorrowedFd(99)), Err(FdPassError::TooManyFds)));
    }

    #[test]
    fn short_header_rejected() {
        assert!(matches!(FdEnvelope::decode_header(&[1, 2]), Err(FdPassError::InvalidHeader)));
    }

    #[test]
    fn new_envelope_empty() {
        let e = FdEnvelope::new(7, 1);
        assert!(e.fds.is_empty());
    }
}
