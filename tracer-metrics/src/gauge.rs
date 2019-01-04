use fnv::FnvHashMap;
use std::fmt::Display;
use std::hash::Hash;
pub struct Gauges<T> {
    data: FnvHashMap<T, u64>,
}
impl<T> Gauges<T>
where
    T: Hash + Eq + Display,
{
    pub fn new() -> Gauges<T> {
        Gauges {
            data: FnvHashMap::default(),
        }
    }

    pub fn init(&mut self, key: T) {
        self.data.insert(key, 0);
    }

    pub fn interested(&self, key: &T) -> bool {
        self.data.contains_key(&key)
    }

    pub fn set(&mut self, key: &T, val: u64) {
        if let Some(v) = self.data.get_mut(&key) {
            *v = val;
        }
    }

    pub fn clear(&mut self, key: &T) {
        if let Some(v) = self.data.get_mut(&key) {
            *v = 0;
        }
    }

    pub fn get(&self, key: &T) -> Option<u64> {
        self.data.get(&key).map(|&v| v)
    }

    pub fn remove(&mut self, key: &T) {
        self.data.remove(&key);
    }
}
