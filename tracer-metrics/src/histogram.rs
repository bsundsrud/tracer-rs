use fnv::FnvHashMap;
use hdrhistogram::Histogram;
use std::hash::Hash;

pub struct Histograms<T> {
    data: FnvHashMap<T, Histogram<u64>>,
}

impl<T> Default for Histograms<T>
where
    T: Hash + Eq,
{
    fn default() -> Self {
        Histograms::new()
    }
}

impl<T> Histograms<T>
where
    T: Hash + Eq,
{
    /// Create a new `Histograms` object with no interests.
    pub fn new() -> Histograms<T> {
        Histograms {
            data: FnvHashMap::default(),
        }
    }

    /// Register an interest and initialize a blank histogram
    pub fn init(&mut self, key: T) {
        self.data.insert(
            key,
            Histogram::new_with_max(60 * 1000 * 1000, 3).expect("Could not create histogram"),
        );
    }

    /// Check if the given interest has been registered with this object
    pub fn interested(&self, key: &T) -> bool {
        self.data.contains_key(&key)
    }

    /// Record a value in the histogram with the given key.
    /// If the key hasn't been registered, this is ignored and the `Histograms` object will not be updated.
    pub fn record(&mut self, key: &T, value: u64) {
        self.record_multiple(key, value, 1);
    }

    /// Record multiple occurrences of a value in the histogram with the given key.
    /// If the key hasn't been registered, this is ignored and the `Histograms` object will not be updated.
    pub fn record_multiple(&mut self, key: &T, value: u64, count: u64) {
        if let Some(h) = self.data.get_mut(&key) {
            h.saturating_record_n(value, count);
        }
    }

    /// Clear the histogram with the given key
    /// If the key hasn't been registered, this is ignored and the `Histograms` object will not be updated.
    pub fn clear(&mut self, key: &T) {
        if let Some(h) = self.data.get_mut(&key) {
            h.clear();
        }
    }

    /// Get the histogram for the given key, if it exists.
    pub fn get(&self, key: &T) -> Option<Histogram<u64>> {
        self.data.get(&key).cloned()
    }

    /// Get the value at the given **quantile** for the given key, if it exists.
    pub fn quantile(&self, key: &T, q: f64) -> Option<u64> {
        self.data.get(&key).map(|h| h.value_at_quantile(q))
    }

    /// Remove (and unregister) the histogram with the given key
    pub fn remove(&mut self, key: &T) {
        self.data.remove(&key);
    }
}
