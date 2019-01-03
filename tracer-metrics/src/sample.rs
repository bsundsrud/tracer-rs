use std::fmt::Display;
use std::hash::Hash;
use std::time::Duration;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SampleValue {
    Elapsed(Duration),
    Count(u64),
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
    pub fn elapsed(key: T, duration: Duration) -> Sample<T> {
        Sample::new(key, SampleValue::Elapsed(duration))
    }
    pub fn count(key: T, count: u64) -> Sample<T> {
        Sample::new(key, SampleValue::Count(count))
    }

    pub fn value(key: T, value: u64) -> Sample<T> {
        Sample::new(key, SampleValue::Value(value))
    }

    pub fn key(&self) -> T {
        self.key.clone()
    }
}
