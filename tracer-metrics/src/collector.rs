use crate::counter::Counters;
use crate::gauge::Gauges;
use crate::histogram::Histograms;
use crate::sample::Sample;
use crate::snapshots::{Percentile, Snapshot};
use crate::stopwatch::Stopwatch;
use crate::util;
use crossbeam::channel::{unbounded, Receiver, Sender};
use std::fmt::Display;
use std::hash::Hash;
use std::sync::RwLock;
use std::time::Duration;

#[derive(Debug)]
pub enum Interest<T> {
    Count(T),
    Gauge(T),
    LatencyPercentile(T),
}

pub struct Collector<T> {
    counters: RwLock<Counters<T>>,
    gauges: RwLock<Gauges<T>>,
    latency_histograms: RwLock<Histograms<T>>,
    tx: Sender<Sample<T>>,
    rx: Receiver<Sample<T>>,
    percentiles: Vec<Percentile>,
}

/// Default percentiles of interest.  Includes 50th, 75th, 90th, 95th, 99th, 99.9th.
pub fn default_percentiles() -> Vec<Percentile> {
    vec![
        Percentile::new("p50", 50.0),
        Percentile::new("p75", 75.0),
        Percentile::new("p90", 90.0),
        Percentile::new("p95", 95.0),
        Percentile::new("p99", 99.0),
        Percentile::new("p99.9", 99.9),
    ]
}

impl<T> Default for Collector<T>
where
    T: Hash + Eq + Send + Display + Clone,
{
    fn default() -> Self {
        Collector::new()
    }
}

impl<T> Collector<T>
where
    T: Hash + Eq + Send + Display + Clone,
{
    /// Create a new collector with zeroed counters, gauges, and histograms and with default percentiles.
    pub fn new() -> Collector<T> {
        let (tx, rx) = unbounded();
        Collector {
            counters: RwLock::new(Counters::new()),
            gauges: RwLock::new(Gauges::new()),
            latency_histograms: RwLock::new(Histograms::new()),
            tx,
            rx,
            percentiles: default_percentiles(),
        }
    }

    /// Register an interest in a given type.  Without registration, metrics of that type will be ignored.
    pub fn register(&mut self, ty: Interest<T>) {
        use self::Interest::*;
        match ty {
            Count(key) => self.counters.write().unwrap().init(key),
            LatencyPercentile(key) => self.latency_histograms.write().unwrap().init(key),
            Gauge(key) => self.gauges.write().unwrap().init(key),
        }
    }

    /// Get a handle to this collector that can be used to send samples
    pub fn handle(&self) -> CollectorHandle<T> {
        CollectorHandle {
            sender: self.tx.clone(),
        }
    }

    fn record_sample(&self, sample: Sample<T>) {
        let key = sample.key();
        use crate::sample::SampleValue::*;
        match sample.value {
            Elapsed(d) => {
                let mut counters = self.counters.write().unwrap();
                let mut gauges = self.gauges.write().unwrap();
                let mut histograms = self.latency_histograms.write().unwrap();
                counters.increment(&key);
                let nanos = util::dur_to_u64(d);
                gauges.set(&key, nanos);
                histograms.record(&key, nanos);
            }
            Count(c) => {
                let mut counters = self.counters.write().unwrap();
                counters.increment_by(&key, c);
            }
            Value(v) => {
                let mut counters = self.counters.write().unwrap();
                let mut gauges = self.gauges.write().unwrap();
                counters.increment(&key);
                gauges.set(&key, v);
            }
        }
    }

    /// Process all Samples that have been sent to this Collector but not processed yet. Needs to be repeatedly called
    pub fn process_outstanding(&self) {
        let rx = self.rx.clone();
        while let Ok(sample) = rx.try_recv() {
            self.record_sample(sample);
        }
    }

    /// Receive Samples in a loop and block until the channel is closed.
    ///
    /// Returns `Err` on channel close.
    pub fn process_blocking(&self) -> Result<(), Box<dyn std::error::Error>> {
        let rx = self.rx.clone();
        loop {
            let sample = rx.recv()?;
            self.record_sample(sample);
        }
    }

    /// Retrieve a current snapshot of all values in this collector.
    pub fn snapshot(&self, key: &T) -> Snapshot<T> {
        Snapshot::new(
            key.clone(),
            self.counters.read().unwrap().get(&key),
            self.gauges.read().unwrap().get(&key),
            self.latency_histograms.read().unwrap().get(&key),
            self.percentiles.clone(),
        )
    }
}

#[derive(Debug, Clone)]
pub struct CollectorHandle<T> {
    sender: Sender<Sample<T>>,
}

impl<T: Hash + Eq + Send + Display + Clone> CollectorHandle<T> {
    pub fn stopwatch(&self) -> Stopwatch {
        Stopwatch::new()
    }

    pub fn send(&self, sample: Sample<T>) {
        self.sender.try_send(sample).unwrap();
    }

    pub fn send_elapsed(&self, key: T, d: Duration) {
        self.send(Sample::elapsed(key, d))
    }

    pub fn send_count(&self, key: T, c: u64) {
        self.send(Sample::count(key, c))
    }

    pub fn send_value(&self, key: T, v: u64) {
        self.send(Sample::value(key, v))
    }
}
