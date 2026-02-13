#![forbid(unsafe_code)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientCommand {
    pub seq: u64,
    pub payload: Vec<u8>,
}

#[derive(Debug, Default)]
pub struct ClientSession {
    pub pending: Vec<ClientCommand>,
}

impl ClientSession {
    pub fn enqueue(&mut self, cmd: ClientCommand) {
        self.pending.push(cmd);
    }

    #[must_use]
    pub fn flush(&mut self) -> Vec<ClientCommand> {
        std::mem::take(&mut self.pending)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enqueue_and_flush_round_trip() {
        let mut c = ClientSession::default();
        c.enqueue(ClientCommand { seq: 1, payload: vec![1] });
        assert_eq!(c.flush().len(), 1);
        assert!(c.pending.is_empty());
    }

    #[test]
    fn flush_is_idempotent_when_empty() {
        let mut c = ClientSession::default();
        assert!(c.flush().is_empty());
    }
}
