//! # mux-kernel
//!
//! Sans-IO reducer for mux state transitions.
//! It provides:
//! - `Event` / `Effect` boundary types.
//! - `GenSlotMap<T>` generational slot map keys.
//! - `ServerGraph` for server topology.
//! - OTEL-style span helpers for cross-crate boundary instrumentation.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use mux_time::LamportClock;

/// Lightweight OTEL boundary span helper.
#[must_use]
pub fn otel_boundary_span(name: &'static str) -> tracing::Span {
    tracing::info_span!(target: "termforge::otel", "cross_crate_boundary", op = name)
}

/// Create a boundary span using a concise macro form.
#[macro_export]
macro_rules! otel_span {
    ($name:expr) => {
        $crate::otel_boundary_span($name)
    };
}

/// Enter an OTEL boundary scope for the current lexical block.
#[macro_export]
macro_rules! otel_scope {
    ($name:expr) => {
        let __span = $crate::otel_span!($name);
        let _guard = __span.enter();
    };
}

/// Key into a generational slot map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GenKey {
    slot: u32,
    generation: u32,
}

impl GenKey {
    #[must_use]
    pub fn slot(self) -> u32 {
        self.slot
    }

    #[must_use]
    pub fn generation(self) -> u32 {
        self.generation
    }
}

#[derive(Debug)]
struct Slot<T> {
    generation: u32,
    value: Option<T>,
}

/// Kernel-level errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelError {
    SlotOverflow,
    MissingServer(GenKey),
    DuplicateEdge { a: GenKey, b: GenKey },
    SelfEdge(GenKey),
}

impl std::fmt::Display for KernelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SlotOverflow => f.write_str("slot map overflow"),
            Self::MissingServer(key) => write!(f, "server missing: slot={} gen={}", key.slot(), key.generation()),
            Self::DuplicateEdge { a, b } => write!(f, "duplicate edge: ({a:?}, {b:?})"),
            Self::SelfEdge(key) => write!(f, "self-edge is invalid: {key:?}"),
        }
    }
}

impl std::error::Error for KernelError {}

/// Generational slot map with stable keys and ABA resistance.
#[derive(Debug)]
pub struct GenSlotMap<T> {
    slots: Vec<Slot<T>>,
    free: Vec<usize>,
    len: usize,
}

impl<T> Default for GenSlotMap<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> GenSlotMap<T> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            free: Vec::new(),
            len: 0,
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[must_use]
    pub fn contains_key(&self, key: GenKey) -> bool {
        self.get(key).is_some()
    }

    #[must_use]
    pub fn get(&self, key: GenKey) -> Option<&T> {
        let slot = self.slots.get(key.slot as usize)?;
        if slot.generation != key.generation {
            return None;
        }
        slot.value.as_ref()
    }

    pub fn get_mut(&mut self, key: GenKey) -> Option<&mut T> {
        let slot = self.slots.get_mut(key.slot as usize)?;
        if slot.generation != key.generation {
            return None;
        }
        slot.value.as_mut()
    }

    #[must_use]
    pub fn iter(&self) -> impl Iterator<Item = (GenKey, &T)> {
        self.slots.iter().enumerate().filter_map(|(slot, entry)| {
            let value = entry.value.as_ref()?;
            let key = GenKey {
                slot: slot as u32,
                generation: entry.generation,
            };
            Some((key, value))
        })
    }

    #[must_use]
    pub fn insert(&mut self, value: T) -> Result<GenKey, KernelError> {
        if let Some(index) = self.free.pop() {
            let entry = &mut self.slots[index];
            entry.value = Some(value);
            self.len += 1;
            return Ok(GenKey {
                slot: index as u32,
                generation: entry.generation,
            });
        }

        if self.slots.len() == u32::MAX as usize {
            return Err(KernelError::SlotOverflow);
        }

        self.slots.push(Slot {
            generation: 0,
            value: Some(value),
        });
        self.len += 1;
        Ok(GenKey {
            slot: (self.slots.len() - 1) as u32,
            generation: 0,
        })
    }

    #[must_use]
    pub fn remove(&mut self, key: GenKey) -> Result<T, KernelError> {
        let slot_idx = key.slot as usize;
        let entry = self
            .slots
            .get_mut(slot_idx)
            .ok_or(KernelError::MissingServer(key))?;

        if entry.generation != key.generation {
            return Err(KernelError::MissingServer(key));
        }

        let value = entry.value.take().ok_or(KernelError::MissingServer(key))?;
        entry.generation = entry.generation.wrapping_add(1);
        self.free.push(slot_idx);
        self.len = self.len.saturating_sub(1);
        Ok(value)
    }
}

/// Server node metadata in the kernel graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerNode {
    pub name: String,
}

pub type ServerKey = GenKey;

/// Simple server topology graph backed by `GenSlotMap`.
#[derive(Debug, Default)]
pub struct ServerGraph {
    servers: GenSlotMap<ServerNode>,
    edges: BTreeSet<(ServerKey, ServerKey)>,
}

impl ServerGraph {
    #[must_use]
    pub fn new() -> Self {
        Self {
            servers: GenSlotMap::new(),
            edges: BTreeSet::new(),
        }
    }

    #[must_use]
    pub fn server_count(&self) -> usize {
        self.servers.len()
    }

    #[must_use]
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    #[must_use]
    pub fn contains_server(&self, key: ServerKey) -> bool {
        self.servers.contains_key(key)
    }

    #[must_use]
    pub fn server(&self, key: ServerKey) -> Option<&ServerNode> {
        self.servers.get(key)
    }

    #[must_use]
    pub fn add_server(&mut self, name: impl Into<String>) -> Result<ServerKey, KernelError> {
        self.servers.insert(ServerNode { name: name.into() })
    }

    #[must_use]
    pub fn remove_server(&mut self, key: ServerKey) -> Result<ServerNode, KernelError> {
        let removed = self.servers.remove(key)?;
        self.edges.retain(|(a, b)| *a != key && *b != key);
        Ok(removed)
    }

    #[must_use]
    pub fn connect(&mut self, a: ServerKey, b: ServerKey) -> Result<(), KernelError> {
        if a == b {
            return Err(KernelError::SelfEdge(a));
        }
        if !self.contains_server(a) {
            return Err(KernelError::MissingServer(a));
        }
        if !self.contains_server(b) {
            return Err(KernelError::MissingServer(b));
        }

        let edge = canonical_edge(a, b);
        if !self.edges.insert(edge) {
            return Err(KernelError::DuplicateEdge { a: edge.0, b: edge.1 });
        }
        Ok(())
    }

    #[must_use]
    pub fn disconnect(&mut self, a: ServerKey, b: ServerKey) -> bool {
        self.edges.remove(&canonical_edge(a, b))
    }

    #[must_use]
    pub fn neighbors(&self, key: ServerKey) -> Result<Vec<ServerKey>, KernelError> {
        if !self.contains_server(key) {
            return Err(KernelError::MissingServer(key));
        }

        let mut out = Vec::new();
        for (a, b) in &self.edges {
            if *a == key {
                out.push(*b);
            } else if *b == key {
                out.push(*a);
            }
        }
        out.sort_unstable();
        Ok(out)
    }
}

fn canonical_edge(a: ServerKey, b: ServerKey) -> (ServerKey, ServerKey) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

/// Kernel events (inputs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    AddServer { name: String },
    RemoveServer { server: ServerKey },
    Connect { a: ServerKey, b: ServerKey },
    Disconnect { a: ServerKey, b: ServerKey },
    TickLamport { server: ServerKey, remote: u64 },
}

/// Kernel effects (outputs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    ServerAdded { key: ServerKey, name: String },
    ServerRemoved { key: ServerKey },
    Connected { a: ServerKey, b: ServerKey },
    Disconnected { a: ServerKey, b: ServerKey },
    LamportAdvanced {
        server: ServerKey,
        before: u64,
        after: u64,
    },
    #[cfg(feature = "crdt")]
    CrdtApplied { server: ServerKey },
    Rejected { reason: &'static str },
}

/// Sans-IO event reducer.
#[derive(Debug, Default)]
pub struct Kernel {
    graph: ServerGraph,
    lamports: BTreeMap<ServerKey, LamportClock>,
    #[cfg(feature = "crdt")]
    crdt_log: BTreeMap<ServerKey, Vec<CrdtDelta>>,
}

impl Kernel {
    #[must_use]
    pub fn new() -> Self {
        Self {
            graph: ServerGraph::new(),
            lamports: BTreeMap::new(),
            #[cfg(feature = "crdt")]
            crdt_log: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn graph(&self) -> &ServerGraph {
        &self.graph
    }

    #[must_use]
    pub fn reduce(&mut self, event: Event) -> Vec<Effect> {
        let _span = otel_span!("mux-kernel::Kernel::reduce").entered();
        match event {
            Event::AddServer { name } => match self.graph.add_server(name.clone()) {
                Ok(key) => {
                    self.lamports.insert(key, LamportClock::new());
                    vec![Effect::ServerAdded { key, name }]
                }
                Err(_) => vec![Effect::Rejected {
                    reason: "slot-overflow",
                }],
            },
            Event::RemoveServer { server } => {
                if self.graph.remove_server(server).is_ok() {
                    self.lamports.remove(&server);
                    #[cfg(feature = "crdt")]
                    {
                        self.crdt_log.remove(&server);
                    }
                    vec![Effect::ServerRemoved { key: server }]
                } else {
                    vec![Effect::Rejected {
                        reason: "missing-server",
                    }]
                }
            }
            Event::Connect { a, b } => match self.graph.connect(a, b) {
                Ok(()) => vec![Effect::Connected { a, b }],
                Err(KernelError::SelfEdge(_)) => vec![Effect::Rejected {
                    reason: "self-edge",
                }],
                Err(KernelError::DuplicateEdge { .. }) => vec![Effect::Rejected {
                    reason: "duplicate-edge",
                }],
                Err(_) => vec![Effect::Rejected {
                    reason: "missing-server",
                }],
            },
            Event::Disconnect { a, b } => {
                if self.graph.disconnect(a, b) {
                    vec![Effect::Disconnected { a, b }]
                } else {
                    vec![Effect::Rejected {
                        reason: "missing-edge",
                    }]
                }
            }
            Event::TickLamport { server, remote } => {
                if let Some(clock) = self.lamports.get_mut(&server) {
                    let before = clock.now();
                    let after = clock.receive(remote);
                    vec![Effect::LamportAdvanced {
                        server,
                        before,
                        after,
                    }]
                } else {
                    vec![Effect::Rejected {
                        reason: "missing-server",
                    }]
                }
            }
        }
    }
}

/// Optional CRDT shape behind feature flag.
#[cfg(feature = "crdt")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrdtDelta {
    pub actor: u32,
    pub counter: u64,
    pub payload: Vec<u8>,
}

#[cfg(feature = "crdt")]
impl Kernel {
    #[must_use]
    pub fn apply_crdt_delta(
        &mut self,
        server: ServerKey,
        delta: CrdtDelta,
    ) -> Result<Effect, KernelError> {
        let _span = otel_span!("mux-kernel::Kernel::apply_crdt_delta").entered();
        if !self.graph.contains_server(server) {
            return Err(KernelError::MissingServer(server));
        }

        self.crdt_log.entry(server).or_default().push(delta);
        Ok(Effect::CrdtApplied { server })
    }

    #[must_use]
    pub fn crdt_log_len(&self, server: ServerKey) -> usize {
        self.crdt_log.get(&server).map_or(0, Vec::len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_map_insert_get_remove_roundtrip() {
        let mut map = GenSlotMap::new();
        let key = map.insert("hello").unwrap();
        assert_eq!(map.get(key), Some(&"hello"));
        let v = map.remove(key).unwrap();
        assert_eq!(v, "hello");
        assert!(map.get(key).is_none());
    }

    #[test]
    fn slot_map_generation_changes_on_reuse() {
        let mut map = GenSlotMap::new();
        let first = map.insert(10).unwrap();
        map.remove(first).unwrap();
        let second = map.insert(20).unwrap();
        assert_eq!(first.slot(), second.slot());
        assert_ne!(first.generation(), second.generation());
    }

    #[test]
    fn slot_map_contains_key_tracks_removal() {
        let mut map = GenSlotMap::new();
        let key = map.insert(1).unwrap();
        assert!(map.contains_key(key));
        map.remove(key).unwrap();
        assert!(!map.contains_key(key));
    }

    #[test]
    fn slot_map_iter_only_live_entries() {
        let mut map = GenSlotMap::new();
        let a = map.insert(1).unwrap();
        let b = map.insert(2).unwrap();
        map.remove(a).unwrap();
        let values: Vec<(GenKey, i32)> = map.iter().map(|(k, v)| (k, *v)).collect();
        assert_eq!(values, vec![(b, 2)]);
    }

    #[test]
    fn server_graph_add_and_lookup() {
        let mut graph = ServerGraph::new();
        let key = graph.add_server("alpha").unwrap();
        assert_eq!(graph.server_count(), 1);
        assert_eq!(graph.server(key).map(|n| n.name.as_str()), Some("alpha"));
    }

    #[test]
    fn server_graph_connect_disconnect() {
        let mut graph = ServerGraph::new();
        let a = graph.add_server("a").unwrap();
        let b = graph.add_server("b").unwrap();
        graph.connect(a, b).unwrap();
        assert_eq!(graph.edge_count(), 1);
        assert!(graph.disconnect(a, b));
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn server_graph_rejects_self_edge() {
        let mut graph = ServerGraph::new();
        let a = graph.add_server("a").unwrap();
        assert!(matches!(graph.connect(a, a), Err(KernelError::SelfEdge(_))));
    }

    #[test]
    fn server_graph_rejects_duplicate_edge() {
        let mut graph = ServerGraph::new();
        let a = graph.add_server("a").unwrap();
        let b = graph.add_server("b").unwrap();
        graph.connect(a, b).unwrap();
        assert!(matches!(
            graph.connect(a, b),
            Err(KernelError::DuplicateEdge { .. })
        ));
    }

    #[test]
    fn server_graph_remove_prunes_edges() {
        let mut graph = ServerGraph::new();
        let a = graph.add_server("a").unwrap();
        let b = graph.add_server("b").unwrap();
        graph.connect(a, b).unwrap();
        graph.remove_server(a).unwrap();
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn server_graph_neighbors_sorted() {
        let mut graph = ServerGraph::new();
        let a = graph.add_server("a").unwrap();
        let b = graph.add_server("b").unwrap();
        let c = graph.add_server("c").unwrap();
        graph.connect(a, c).unwrap();
        graph.connect(a, b).unwrap();
        let neighbors = graph.neighbors(a).unwrap();
        assert_eq!(neighbors, vec![b, c]);
    }

    #[test]
    fn kernel_add_server_emits_effect() {
        let mut kernel = Kernel::new();
        let effects = kernel.reduce(Event::AddServer {
            name: "one".to_string(),
        });
        assert!(matches!(
            effects.as_slice(),
            [Effect::ServerAdded { name, .. }] if name == "one"
        ));
    }

    #[test]
    fn kernel_connect_and_tick_lamport() {
        let mut kernel = Kernel::new();
        let a = match kernel.reduce(Event::AddServer { name: "a".into() }).pop().unwrap() {
            Effect::ServerAdded { key, .. } => key,
            other => panic!("unexpected effect: {other:?}"),
        };
        let b = match kernel.reduce(Event::AddServer { name: "b".into() }).pop().unwrap() {
            Effect::ServerAdded { key, .. } => key,
            other => panic!("unexpected effect: {other:?}"),
        };

        assert!(matches!(
            kernel.reduce(Event::Connect { a, b }).as_slice(),
            [Effect::Connected { .. }]
        ));

        let tick = kernel.reduce(Event::TickLamport { server: a, remote: 7 });
        assert!(matches!(
            tick.as_slice(),
            [Effect::LamportAdvanced {
                before: 0,
                after: 8,
                ..
            }]
        ));
    }

    #[test]
    fn kernel_rejects_missing_server_tick() {
        let mut kernel = Kernel::new();
        let fake = GenKey {
            slot: 123,
            generation: 55,
        };
        let effects = kernel.reduce(Event::TickLamport {
            server: fake,
            remote: 1,
        });
        assert_eq!(effects, vec![Effect::Rejected { reason: "missing-server" }]);
    }

    #[test]
    fn kernel_rejects_missing_edge_disconnect() {
        let mut kernel = Kernel::new();
        let a = match kernel.reduce(Event::AddServer { name: "a".into() }).pop().unwrap() {
            Effect::ServerAdded { key, .. } => key,
            other => panic!("unexpected effect: {other:?}"),
        };
        let b = match kernel.reduce(Event::AddServer { name: "b".into() }).pop().unwrap() {
            Effect::ServerAdded { key, .. } => key,
            other => panic!("unexpected effect: {other:?}"),
        };
        let effects = kernel.reduce(Event::Disconnect { a, b });
        assert_eq!(effects, vec![Effect::Rejected { reason: "missing-edge" }]);
    }

    #[test]
    fn kernel_reducer_is_deterministic() {
        fn run() -> Vec<Effect> {
            let mut kernel = Kernel::new();
            let mut out = Vec::new();
            out.extend(kernel.reduce(Event::AddServer { name: "a".into() }));
            let a = match out.last().unwrap() {
                Effect::ServerAdded { key, .. } => *key,
                other => panic!("unexpected effect: {other:?}"),
            };
            out.extend(kernel.reduce(Event::TickLamport {
                server: a,
                remote: 3,
            }));
            out
        }

        assert_eq!(run(), run());
    }

    #[test]
    fn otel_boundary_span_can_be_created() {
        let span = otel_boundary_span("test-span");
        let _guard = span.enter();
    }

    #[test]
    fn otel_scope_macro_enters_span() {
        otel_scope!("test-scope");
        let _x = 1 + 1;
        assert_eq!(_x, 2);
    }

    #[test]
    fn event_and_effect_are_debuggable() {
        let event = Event::AddServer {
            name: "dbg".into(),
        };
        let effect = Effect::Rejected { reason: "dbg" };
        assert!(format!("{event:?}").contains("AddServer"));
        assert!(format!("{effect:?}").contains("Rejected"));
    }

    #[test]
    fn remove_server_emits_effect() {
        let mut kernel = Kernel::new();
        let key = match kernel.reduce(Event::AddServer { name: "x".into() }).pop().unwrap() {
            Effect::ServerAdded { key, .. } => key,
            other => panic!("unexpected effect: {other:?}"),
        };
        assert_eq!(
            kernel.reduce(Event::RemoveServer { server: key }),
            vec![Effect::ServerRemoved { key }]
        );
    }

    #[test]
    fn remove_server_missing_rejected() {
        let mut kernel = Kernel::new();
        let fake = GenKey {
            slot: 1,
            generation: 1,
        };
        assert_eq!(
            kernel.reduce(Event::RemoveServer { server: fake }),
            vec![Effect::Rejected {
                reason: "missing-server"
            }]
        );
    }

    #[cfg(feature = "crdt")]
    #[test]
    fn crdt_delta_requires_known_server() {
        let mut kernel = Kernel::new();
        let fake = GenKey {
            slot: 9,
            generation: 1,
        };
        let delta = CrdtDelta {
            actor: 1,
            counter: 1,
            payload: vec![1, 2, 3],
        };
        assert!(matches!(
            kernel.apply_crdt_delta(fake, delta),
            Err(KernelError::MissingServer(_))
        ));
    }

    #[cfg(feature = "crdt")]
    #[test]
    fn crdt_delta_appends_to_log() {
        let mut kernel = Kernel::new();
        let key = match kernel.reduce(Event::AddServer { name: "x".into() }).pop().unwrap() {
            Effect::ServerAdded { key, .. } => key,
            other => panic!("unexpected effect: {other:?}"),
        };

        let delta = CrdtDelta {
            actor: 7,
            counter: 3,
            payload: vec![4, 5],
        };

        let effect = kernel.apply_crdt_delta(key, delta).unwrap();
        assert_eq!(effect, Effect::CrdtApplied { server: key });
        assert_eq!(kernel.crdt_log_len(key), 1);
    }
}
