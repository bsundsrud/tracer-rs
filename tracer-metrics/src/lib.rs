mod collector;
mod counter;
mod gauge;
mod histogram;
mod sample;
mod snapshots;
mod stopwatch;
mod util;

pub use self::collector::{Collector, CollectorHandle, Interest};
pub use self::stopwatch::Stopwatch;
pub mod metrics {
    pub use crate::counter::Counters;
    pub use crate::gauge::Gauges;
    pub use crate::histogram::Histograms;
}
pub mod data {
    pub use crate::sample::{Sample, SampleValue};
    pub use crate::snapshots::{HistoSnapshot, Percentile, Snapshot};
}

#[cfg(test)]
mod test {
    use super::*;
    use std::time::Duration;

    #[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
    enum Interests {
        FooTiming,
        BarTiming,
    }

    #[test]
    fn test_collection() {
        let mut collector: Collector<Interests> = Collector::new();
        collector.register(Interest::Count(Interests::FooTiming));
        collector.register(Interest::LatencyPercentile(Interests::FooTiming));
        collector.register(Interest::Gauge(Interests::BarTiming));
        let handle = collector.handle();

        let stopwatch = Stopwatch::new();
        std::thread::sleep(Duration::from_millis(1_500));
        handle.send(stopwatch.elapsed(Interests::FooTiming));
        collector.process_outstanding();
        let snapshot = collector.snapshot(&Interests::FooTiming);
        assert_eq!(1, snapshot.count().unwrap());
        let histo = snapshot.latency_histogram().unwrap();
        assert!(
            histo.min() >= Duration::from_secs(1),
            "Got {:?}, expected >= {:?}",
            histo.min(),
            Duration::from_secs(1)
        );
    }
}
