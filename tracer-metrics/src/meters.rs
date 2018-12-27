use crate::histogram::LatencyHistogram;
use crate::util;
use hdrhistogram::Histogram;
use std::fmt::Display;
use std::hash::Hash;
use std::time::Duration;

pub struct Meters<T> {
    key: T,
    count: Option<u64>,
    gauge: Option<u64>,
    latency_histogram: Option<Histogram<u64>>,
}

impl<T: Eq + Hash + Display + Send + Clone> Meters<T> {
    pub fn new(
        key: T,
        count: Option<u64>,
        gauge: Option<u64>,
        latency_histogram: Option<Histogram<u64>>,
    ) -> Meters<T> {
        Meters {
            key,
            count: count.into(),
            gauge: gauge.into(),
            latency_histogram: latency_histogram.into(),
        }
    }

    pub fn key(&self) -> T {
        self.key.clone()
    }

    pub fn count(&self) -> Option<u64> {
        self.count
    }

    pub fn gauge(&self) -> Option<u64> {
        self.gauge
    }

    pub fn gauge_as_duration(&self) -> Option<Duration> {
        self.gauge.map(util::u64_to_dur)
    }

    pub fn latency_histogram(&self) -> Option<LatencyHistogram> {
        self.latency_histogram.clone().map(|h| h.into())
    }
}
