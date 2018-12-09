use std::sync::Arc;
use std::sync::RwLock;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy)]
pub enum Event {
    Initiated,
    DnsResolved,
    Connected,
    TlsNegotiated,
    FirstByte,
    FullResponse,
}
#[derive(Debug, Clone)]
pub struct EventCollector(Arc<RwLock<EventCollectorInner>>);

#[derive(Debug, Clone)]
pub struct EventCollectorInner {
    events: Vec<(Event, Instant)>,
}

impl EventCollector {
    pub fn new() -> Self {
        EventCollector(Arc::new(RwLock::new(EventCollectorInner {
            events: Vec::new(),
        })))
    }

    pub fn add(&self, e: Event) {
        let collector = self.0.clone();
        let mut collector = collector.write().unwrap();
        collector.events.push((e, Instant::now()));
    }

    pub fn since(&self, earlier: Instant) -> Vec<(Event, Duration)> {
        let collector = self.0.clone();
        let collector = collector.read().unwrap();
        collector
            .events
            .iter()
            .map(move |(e, t)| (*e, t.duration_since(earlier)))
            .collect()
    }
}
