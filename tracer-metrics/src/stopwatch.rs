use crate::sample::Sample;
use std::fmt::Display;
use std::hash::Hash;
use std::time::Instant;

#[derive(Debug)]
pub struct Stopwatch {
    start: Instant,
}

impl Default for Stopwatch {
    fn default() -> Self {
        Stopwatch::new()
    }
}

impl Stopwatch {
    pub fn new() -> Stopwatch {
        Stopwatch {
            start: Instant::now(),
        }
    }

    pub fn elapsed<T: Eq + Hash + Send + Display + Clone>(&self, key: T) -> Sample<T> {
        Sample::elapsed(key, self.start.elapsed())
    }
}
