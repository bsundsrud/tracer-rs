use std::fmt::Display;
use std::hash::Hash;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SampleValue {
    /// Elapsed values can be added to Counters, Gauges, and Histograms
    Elapsed(Duration),
    /// Count values can be added to Counters
    Count(u64),
    /// Value values can be added to Counters and Gauges
    Value(u64),
}

#[derive(Debug)]
pub struct Sample<T> {
    key: T,
    pub value: SampleValue,
}

impl<T: Hash + Eq + Send + Display + Clone> Sample<T> {
    fn new(key: T, value: SampleValue) -> Sample<T> {
        Sample { key, value }
    }

    /// Create an `Elapsed` sample from the given key and duration
    pub fn elapsed(key: T, duration: Duration) -> Sample<T> {
        Sample::new(key, SampleValue::Elapsed(duration))
    }

    /// Create a `Count` sample from the given key and value
    pub fn count(key: T, count: u64) -> Sample<T> {
        Sample::new(key, SampleValue::Count(count))
    }

    /// Create a `Value` sample from the given key and value
    pub fn value(key: T, value: u64) -> Sample<T> {
        Sample::new(key, SampleValue::Value(value))
    }

    /// Get the key of this `Sample`
    pub fn key(&self) -> T {
        self.key.clone()
    }
}
