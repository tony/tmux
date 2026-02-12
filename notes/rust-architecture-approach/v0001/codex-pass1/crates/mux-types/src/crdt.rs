//! Optional CRDT transport types.

/// CRDT operation identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OpId {
    pub actor: u32,
    pub counter: u64,
}

/// Minimal CRDT envelope used by feature-gated collaboration paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrdtEnvelope {
    pub id: OpId,
    pub payload: Vec<u8>,
}

impl CrdtEnvelope {
    #[must_use]
    pub fn new(actor: u32, counter: u64, payload: Vec<u8>) -> Self {
        Self {
            id: OpId { actor, counter },
            payload,
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.payload.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_round_trip_fields() {
        let env = CrdtEnvelope::new(7, 9, vec![1, 2, 3]);
        assert_eq!(env.id.actor, 7);
        assert_eq!(env.id.counter, 9);
        assert_eq!(env.payload, vec![1, 2, 3]);
    }

    #[test]
    fn envelope_empty_payload_detected() {
        let env = CrdtEnvelope::new(1, 1, vec![]);
        assert!(env.is_empty());
    }
}
