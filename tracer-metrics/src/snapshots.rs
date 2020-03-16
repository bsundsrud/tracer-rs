use crate::util;
use hdrhistogram::Histogram;
use std::fmt::{self, Display};
use std::hash::Hash;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Percentile {
    label: String,
    percentile: f64,
}

impl Percentile {
    pub fn new<S: Into<String>>(label: S, percentile: f64) -> Percentile {
        Percentile {
            label: label.into(),
            percentile,
        }
    }

    pub fn label(&self) -> &str {
        &self.label
    }
}

impl Display for Percentile {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.label)
    }
}

#[derive(Debug, Clone)]
pub struct HistoSnapshot<T> {
    min: T,
    max: T,
    mean: T,
    stdev: T,
    percentiles: Vec<(Percentile, T)>,
}

impl<T: Clone> HistoSnapshot<T> {
    /// Get the minimum value in this Snapshot
    pub fn min(&self) -> T {
        self.min.clone()
    }

    /// Get the maximum value in this Snapshot
    pub fn max(&self) -> T {
        self.max.clone()
    }

    /// Get the mean value in this Snapshot
    pub fn mean(&self) -> T {
        self.mean.clone()
    }

    /// Get the standard deviation in this Snapshot
    pub fn stdev(&self) -> T {
        self.stdev.clone()
    }

    /// Get the percentiles captured by this Snapshot
    pub fn percentiles(&self) -> Vec<(Percentile, T)> {
        self.percentiles.clone()
    }
}

impl HistoSnapshot<Duration> {
    /// Create a Snapshot from a given Histogram and the desired Percentiles
    pub fn from_histo(
        histo: &Histogram<u64>,
        percentiles: Vec<Percentile>,
    ) -> HistoSnapshot<Duration> {
        let min = util::u64_to_dur(histo.min());
        let max = util::u64_to_dur(histo.max());
        let mean = util::u64_to_dur(histo.mean().trunc() as u64);
        let stdev = util::u64_to_dur(histo.stdev().trunc() as u64);
        let values = percentiles
            .into_iter()
            .map(|p| {
                let perc = p.percentile;
                (p, util::u64_to_dur(histo.value_at_percentile(perc)))
            })
            .collect();
        HistoSnapshot {
            min,
            max,
            mean,
            stdev,
            percentiles: values,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Snapshot<T> {
    key: T,
    count: Option<u64>,
    gauge: Option<u64>,
    latency_snapshot: Option<HistoSnapshot<Duration>>,
}

impl<T: Eq + Hash + Display + Send + Clone> Snapshot<T> {
    /// Create a new Snapshot from the given count, gauage, histogram, and percentiles
    pub fn new(
        key: T,
        count: Option<u64>,
        gauge: Option<u64>,
        latency_histogram: Option<Histogram<u64>>,
        percentiles: Vec<Percentile>,
    ) -> Snapshot<T> {
        Snapshot {
            key,
            count,
            gauge,
            latency_snapshot: latency_histogram.map(|h| HistoSnapshot::from_histo(&h, percentiles)),
        }
    }

    /// Get the key for this Snapshot
    pub fn key(&self) -> T {
        self.key.clone()
    }

    /// Get the count value for this Snapshot, if it exists
    pub fn count(&self) -> Option<u64> {
        self.count
    }

    /// Get the gauge value for this Snapshot, if it exists
    pub fn gauge(&self) -> Option<u64> {
        self.gauge
    }

    /// Get the gauge value for this Snapshot as a `Duration`, if it exists
    pub fn gauge_as_duration(&self) -> Option<Duration> {
        self.gauge.map(util::u64_to_dur)
    }

    /// Get the latency histogram for this Snapshot, if it exists
    pub fn latency_histogram(&self) -> Option<HistoSnapshot<Duration>> {
        self.latency_snapshot.clone()
    }
}
