use fnv::FnvHashMap;
use std::hash::Hash;
pub struct Counters<T> {
    data: FnvHashMap<T, u64>,
}
impl<T> Counters<T>
where
    T: Hash + Eq,
{
    pub fn new() -> Counters<T> {
        Counters {
            data: FnvHashMap::default(),
        }
    }

    pub fn interested(&self, key: &T) -> bool {
        self.data.contains_key(&key)
    }

    pub fn init(&mut self, key: T) {
        self.data.insert(key, 0);
    }

    pub fn increment_by(&mut self, key: &T, val: u64) {
        if let Some(v) = self.data.get_mut(&key) {
            *v += val;
        }
    }

    pub fn increment(&mut self, key: &T) {
        self.increment_by(key, 1);
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
