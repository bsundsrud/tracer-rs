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

#[derive(Debug, Default)]
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

    pub fn drain_events(&self) -> EventSet {
        let collector = self.0.clone();
        let mut r = Vec::new();
        while let Some((e, t)) = collector.try_pop() {
            r.push((e, t));
        }
        EventSet(r)
    }
}

#[derive(Debug)]
pub struct EventSet(Vec<(Event, Instant)>);

impl IntoIterator for EventSet {
    type Item = (Event, Instant);
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl EventSet {
    pub fn iter(&self) -> impl Iterator<Item = &(Event, Instant)> {
        self.0.iter()
    }

    ///Drains all events and returns them with `Duration`s relative to the given `Instant`
    pub fn since<'a>(&'a self, earlier: Instant) -> impl Iterator<Item = (Event, Duration)> + 'a {
        self.iter()
            .map(move |&(e, t)| (e, t.duration_since(earlier)))
    }

    /// Drains all events and returns those after the first `Initiated` Event it finds,
    /// with `Duration`s relative to that event
    pub fn since_initiated<'a>(&'a self) -> impl Iterator<Item = (Event, Duration)> + 'a {
        self.iter()
            .scan(None, |init, &(e, t)| {
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

    pub fn initiated_at(&self) -> Option<Instant> {
        self.find_event(Event::Initiated)
    }

    pub fn find_event(&self, ev: Event) -> Option<Instant> {
        self.iter().find(|&(e, _t)| *e == ev).map(|&(_e, t)| t)
    }

    pub fn time_between(&self, first: Event, second: Event) -> Option<Duration> {
        let start = self.find_event(first);
        let end = self.find_event(second);
        start.and_then(|s| end.map(|e| e.duration_since(s)))
    }
}
