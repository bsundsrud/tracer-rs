use crate::util;
use failure::Fail;
use fnv::FnvHashMap;
use histogram::Histogram;
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
    data: FnvHashMap<T, Histogram>,
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
        self.data.insert(key, Histogram::new());
    }

    pub fn interested(&self, key: &T) -> bool {
        self.data.contains_key(&key)
    }

    pub fn increment(&mut self, key: &T, bucket: u64) {
        self.increment_by(key, bucket, 1);
    }

    pub fn increment_by(&mut self, key: &T, bucket: u64, val: u64) {
        if let Some(h) = self.data.get_mut(&key) {
            let _ = h.increment_by(bucket, val);
        }
    }

    pub fn clear(&mut self, key: &T) {
        if let Some(h) = self.data.get_mut(&key) {
            h.clear();
        }
    }

    pub fn get(&self, key: &T) -> Option<Histogram> {
        self.data.get(&key).map(|h| h.clone())
    }

    pub fn percentile(&self, key: &T, perc: f64) -> Result<u64, HistogramError> {
        if let Some(h) = self.data.get(&key) {
            h.percentile(perc)
                .map_err(move |_| HistogramError::NoDataForPercentile(perc))
        } else {
            Err(HistogramError::NoDataForKey)
        }
    }

    pub fn remove(&mut self, key: &T) {
        self.data.remove(&key);
    }
}

#[derive(Clone)]
pub struct LatencyHistogram(Histogram);

impl From<Histogram> for LatencyHistogram {
    fn from(h: Histogram) -> LatencyHistogram {
        LatencyHistogram(h)
    }
}

impl LatencyHistogram {
    pub fn min(&self) -> Duration {
        util::u64_to_dur(self.0.minimum().unwrap())
    }

    pub fn max(&self) -> Duration {
        util::u64_to_dur(self.0.maximum().unwrap())
    }

    pub fn percentile(&self, percentile: f64) -> Duration {
        util::u64_to_dur(self.0.percentile(percentile).unwrap())
    }

    pub fn mean(&self) -> Duration {
        util::u64_to_dur(self.0.mean().unwrap())
    }

    pub fn stddev(&self) -> Duration {
        util::u64_to_dur(self.0.stddev().unwrap())
    }
}
