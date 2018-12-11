use events::{Event, EventCollector};
use futures_cpupool::{Builder as CpuPoolBuilder, CpuFuture, CpuPool};
use hyper::client::connect::dns::Name;
use hyper::client::connect::dns::Resolve;
use std::net::IpAddr;
use std::net::SocketAddr;
use std::net::ToSocketAddrs;

#[derive(Clone)]
pub struct TracingResolver {
    executor: CpuPool,
    collector: EventCollector,
}

pub struct IpAddrs {
    inner: std::vec::IntoIter<SocketAddr>,
}

impl Iterator for IpAddrs {
    type Item = IpAddr;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|s| s.ip())
    }
}

impl TracingResolver {
    pub fn new(threads: usize, collector: EventCollector) -> TracingResolver {
        TracingResolver {
            executor: CpuPoolBuilder::new()
                .name_prefix("hyper-tracing-dns")
                .pool_size(threads)
                .create(),
            collector,
        }
    }
}

impl Resolve for TracingResolver {
    type Addrs = IpAddrs;
    type Future = CpuFuture<IpAddrs, std::io::Error>;

    fn resolve(&self, name: Name) -> Self::Future {
        let collector = self.collector.clone();
        self.executor.spawn_fn(move || {
            collector.add(Event::DnsResolutionStarted);
            let ipaddrs = resolve(name.as_str());
            collector.add(Event::DnsResolutionFinished);
            ipaddrs
        })
    }
}

fn resolve(name: &str) -> Result<IpAddrs, std::io::Error> {
    (name, 0)
        .to_socket_addrs()
        .map(|sockets| IpAddrs { inner: sockets })
}
