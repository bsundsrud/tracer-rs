use std::collections::HashMap;
use std::time::Duration;

pub struct CallStats {
    calls: usize,
    dns: Vec<Duration>,
    connection: Vec<Duration>,
    tls: Vec<Duration>,
    headers: Vec<Duration>,
    response: Vec<Duration>,
    last_hash: Option<String>,
}

pub struct StatsCollector {
    tests: HashMap<String, CallStats>,
}
