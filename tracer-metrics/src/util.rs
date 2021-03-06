use std::time::Duration;

/// Convert a duration to microseconds.  Max value is `u64::max_value()`
pub fn dur_to_u64(d: Duration) -> u64 {
    d.as_secs()
        .saturating_mul(1_000_000)
        .saturating_add(d.subsec_micros().into())
}

/// Convert microseconds to a Duration
pub fn u64_to_dur(v: u64) -> Duration {
    Duration::from_micros(v)
}
