use crossbeam::queue::SegQueue;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Initiated,
    DnsResolutionStarted,
    DnsResolutionFinished,
    ConnectionStarted,
    Connected,
    TlsNegotiationStarted,
    TlsNegotiated,
    HeadersReceived,
    FullResponse,
    ConnectionError,
}

#[derive(Debug)]
pub struct EventCollector(Arc<SegQueue<(Event, Instant)>>);

impl Clone for EventCollector {
    fn clone(&self) -> EventCollector {
        EventCollector(self.0.clone())
    }
}

impl EventCollector {
    pub fn new() -> Self {
        EventCollector(Arc::new(SegQueue::new()))
    }

    pub fn add(&self, e: Event) {
        let collector = self.0.clone();
        collector.push((e, Instant::now()));
    }

    pub fn drain_events(&self) -> Vec<(Event, Instant)> {
        let collector = self.0.clone();
        let mut r = Vec::new();
        while let Some((e, t)) = collector.try_pop() {
            r.push((e, t));
        }
        r
    }

    ///Drains all events and returns them with `Duration`s relative to the given `Instant`
    pub fn since(&self, earlier: Instant) -> impl Iterator<Item = (Event, Duration)> {
        self.drain_events()
            .into_iter()
            .map(move |(e, t)| (e, t.duration_since(earlier)))
    }

    /// Drains all events and returns those after the first `Initiated` Event it finds,
    /// with `Duration`s relative to that event
    pub fn since_initiated(&self) -> impl Iterator<Item = (Event, Duration)> {
        self.drain_events()
            .into_iter()
            .scan(None, |init, (e, t)| {
                // save the `Instant` from `Initiated` for relative comparison down the line.
                // Return `Option<Option<(Event, Duration)>>` to skip spurious events before `Initiated`
                if e == Event::Initiated && init.is_none() {
                    *init = Some(t);
                }

                if let Some(i) = init {
                    Some(Some((e, t.duration_since(*i))))
                } else {
                    Some(None)
                }
            })
            .filter_map(|opt| opt) // Unwrap a layer from above, ignoring any `None` elements
    }
}
