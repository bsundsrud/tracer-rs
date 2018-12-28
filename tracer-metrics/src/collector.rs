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
use std::time::Duration;

#[derive(Debug)]
pub enum Interest<T> {
    Count(T),
    Gauge(T),
    LatencyPercentile(T),
}

pub struct Collector<T> {
    counters: Counters<T>,
    gauges: Gauges<T>,
    latency_histograms: Histograms<T>,
    tx: Sender<Sample<T>>,
    rx: Receiver<Sample<T>>,
    percentiles: Vec<Percentile>,
}

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

impl<T> Collector<T>
where
    T: Hash + Eq + Send + Display + Clone,
{
    pub fn new() -> Collector<T> {
        let (tx, rx) = unbounded();
        Collector {
            counters: Counters::new(),
            gauges: Gauges::new(),
            latency_histograms: Histograms::new(),
            tx,
            rx,
            percentiles: default_percentiles(),
        }
    }

    pub fn register(&mut self, ty: Interest<T>) {
        use self::Interest::*;
        match ty {
            Count(key) => self.counters.init(key),
            LatencyPercentile(key) => self.latency_histograms.init(key),
            Gauge(key) => self.gauges.init(key),
        }
    }

    pub fn handle(&self) -> CollectorHandle<T> {
        CollectorHandle {
            sender: self.tx.clone(),
        }
    }

    pub fn process_outstanding(&mut self) {
        let rx = self.rx.clone();
        while let Ok(sample) = rx.try_recv() {
            let key = sample.key();
            use crate::sample::SampleValue::*;
            match sample.value {
                Elapsed(d) => {
                    self.counters.increment(&key);
                    let nanos = util::dur_to_u64(d);
                    self.gauges.set(&key, nanos);
                    self.latency_histograms.increment(&key, nanos);
                }
                Count(c) => {
                    self.counters.increment_by(&key, c);
                }
                Value(v) => {
                    self.counters.increment(&key);
                    self.gauges.set(&key, v);
                }
            }
        }
    }

    pub fn snapshot(&self, key: &T) -> Snapshot<T> {
        Snapshot::new(
            key.clone(),
            self.counters.get(&key),
            self.gauges.get(&key),
            self.latency_histograms.get(&key),
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
