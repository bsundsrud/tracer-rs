# tracer-metrics

Performant metrics and measurements.  Currently supports counters, gauges,
and latency histograms.

Libraries add instrumentation calls, users register interest in various
bits of instrumentation and then inspect the collector for the values.

Example:

```rust
#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
enum Interests {
    FooTiming,
    BarTiming,
}

fn foo() {
    std::thread::sleep(Duration::from_millis(1500));
}

fn main() {
    # Create the collector and register interests
    let mut collector: Collector<Interests> = Collector::new();
    collector.register(Interest::Count(Interests::FooTiming));
    collector.register(Interest::LatencyPercentile(Interests::FooTiming));
    collector.register(Interest::Gauge(Interests::BarTiming));

    # Get a handle to the collector
    let handle = collector.handle();

    # Create a Stopwatch, call the function, send the measurement to the handle
    let stopwatch = Stopwatch::new();
    foo();
    handle.send(stopwatch.elapsed(Interests::FooTiming));

    # process any outstanding metrics
    collector.process_outstanding();

    # get a snapshot for our FooTiming interest
    let snapshot = collector.snapshot(&Interests::FooTiming);

    assert_eq!(1, snapshot.count().unwrap());

    let histo = snapshot.latency_histogram().unwrap();

    # thread::sleep has jitter so we can't expect an exact value
    assert!(histo.min() >= Duration::from_secs(1));
}
```
