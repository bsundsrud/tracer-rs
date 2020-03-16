use fnv::FnvHashMap;
use std::fmt::Display;
use std::hash::Hash;

pub struct Gauges<T> {
    data: FnvHashMap<T, u64>,
}

impl<T> Default for Gauges<T>
where
    T: Hash + Eq + Display,
{
    fn default() -> Self {
        Gauges::new()
    }
}

impl<T> Gauges<T>
where
    T: Hash + Eq + Display,
{
    /// Create a new `Gauges` object with no interests.
    pub fn new() -> Gauges<T> {
        Gauges {
            data: FnvHashMap::default(),
        }
    }

    /// Register an interest and set its value to 0
    pub fn init(&mut self, key: T) {
        self.data.insert(key, 0);
    }

    /// Check if the given interest has been registered with this object
    pub fn interested(&self, key: &T) -> bool {
        self.data.contains_key(&key)
    }

    /// Set the gauge with the given key to the given value
    /// If the key hasn't been registered, this is ignored and the `Gauges` object will not be updated.
    pub fn set(&mut self, key: &T, val: u64) {
        if let Some(v) = self.data.get_mut(&key) {
            *v = val;
        }
    }

    /// Set the gauge with the given key to 0
    /// If the key hasn't been registered, this is ignored and the `Gauges` object will not be updated.
    pub fn clear(&mut self, key: &T) {
        if let Some(v) = self.data.get_mut(&key) {
            *v = 0;
        }
    }

    /// Get the value of the gauge with the given key, if that key exists
    pub fn get(&self, key: &T) -> Option<u64> {
        self.data.get(&key).copied()
    }

    /// Remove (and unregister) the given key
    pub fn remove(&mut self, key: &T) {
        self.data.remove(&key);
    }
}
