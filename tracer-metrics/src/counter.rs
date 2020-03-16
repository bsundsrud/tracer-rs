use fnv::FnvHashMap;
use std::hash::Hash;
pub struct Counters<T> {
    data: FnvHashMap<T, u64>,
}

impl<T> Default for Counters<T>
where
    T: Hash + Eq,
{
    fn default() -> Self {
        Counters::new()
    }
}

impl<T> Counters<T>
where
    T: Hash + Eq,
{
    /// Create a blank `Counters` object
    pub fn new() -> Counters<T> {
        Counters {
            data: FnvHashMap::default(),
        }
    }

    /// Check if the given interest has been registered with this object
    pub fn interested(&self, key: &T) -> bool {
        self.data.contains_key(&key)
    }

    /// Register interest in the key and zero the counter
    pub fn init(&mut self, key: T) {
        self.data.insert(key, 0);
    }

    /// Increment the counter with the given key by the given value.
    /// If the key hasn't been registered, this is ignored and the `Counters` object will not be updated.
    pub fn increment_by(&mut self, key: &T, val: u64) {
        if let Some(v) = self.data.get_mut(&key) {
            *v += val;
        }
    }

    /// Increment the counter with the given key by one.
    /// If the key hasn't been registered, this is ignored and the `Counters` object will not be updated.
    pub fn increment(&mut self, key: &T) {
        self.increment_by(key, 1);
    }

    /// Reset the counter with the given key to zero.
    /// If the key hasn't been registered, this is ignored and the `Counters` object will not be updated.
    pub fn clear(&mut self, key: &T) {
        if let Some(v) = self.data.get_mut(&key) {
            *v = 0;
        }
    }

    /// Get the value of the counter with the given key, if that key exists in the `Counters`
    pub fn get(&self, key: &T) -> Option<u64> {
        self.data.get(&key).copied()
    }

    /// Remove (and unregister) the given key from the `Counters` object.
    pub fn remove(&mut self, key: &T) {
        self.data.remove(&key);
    }
}
