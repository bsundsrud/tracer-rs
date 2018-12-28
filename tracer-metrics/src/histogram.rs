use fnv::FnvHashMap;
use hdrhistogram::Histogram;
use std::hash::Hash;

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
