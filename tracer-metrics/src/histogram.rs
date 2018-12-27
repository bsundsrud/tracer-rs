use crate::util;
use failure::Fail;
use fnv::FnvHashMap;
use hdrhistogram::Histogram;
use std::hash::Hash;
use std::time::Duration;

#[derive(Debug, Fail)]
pub enum HistogramError {
    #[fail(display = "No data")]
    NoDataForKey,
    #[fail(display = "No data for percentile {}", _0)]
    NoDataForPercentile(f64),
}

pub struct Histograms<T> {
    data: FnvHashMap<T, Histogram<u64>>,
}

impl<T> Histograms<T>
where
    T: Hash + Eq,
{
    pub fn new() -> Histograms<T> {
        Histograms {
            data: FnvHashMap::default(),
        }
    }

    pub fn init(&mut self, key: T) {
        self.data.insert(
            key,
            Histogram::new_with_max(60 * 1000 * 1000, 3).expect("Could not create histogram"),
        );
    }

    pub fn interested(&self, key: &T) -> bool {
        self.data.contains_key(&key)
    }

    pub fn increment(&mut self, key: &T, value: u64) {
        self.increment_by(key, value, 1);
    }

    pub fn increment_by(&mut self, key: &T, value: u64, count: u64) {
        if let Some(h) = self.data.get_mut(&key) {
            h.saturating_record_n(value, count);
        }
    }

    pub fn clear(&mut self, key: &T) {
        if let Some(h) = self.data.get_mut(&key) {
            h.clear();
        }
    }

    pub fn get(&self, key: &T) -> Option<Histogram<u64>> {
        self.data.get(&key).map(|h| h.clone())
    }

    pub fn quantile(&self, key: &T, q: f64) -> Option<u64> {
        self.data.get(&key).map(|h| h.value_at_quantile(q))
    }

    pub fn remove(&mut self, key: &T) {
        self.data.remove(&key);
    }
}

#[derive(Clone)]
pub struct LatencyHistogram(Histogram<u64>);

impl From<Histogram<u64>> for LatencyHistogram {
    fn from(h: Histogram<u64>) -> LatencyHistogram {
        LatencyHistogram(h)
    }
}

impl LatencyHistogram {
    pub fn min(&self) -> Duration {
        util::u64_to_dur(self.0.min())
    }

    pub fn max(&self) -> Duration {
        util::u64_to_dur(self.0.max())
    }

    pub fn quantile(&self, q: f64) -> Duration {
        util::u64_to_dur(self.0.value_at_quantile(q))
    }

    pub fn mean(&self) -> Duration {
        util::u64_to_dur(self.0.mean().trunc() as u64)
    }

    pub fn stddev(&self) -> Duration {
        util::u64_to_dur(self.0.stdev().trunc() as u64)
    }
}
