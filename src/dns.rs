use crate::events::{Event, EventCollector};
use futures::prelude::*;
use futures_cpupool::{Builder as CpuPoolBuilder, CpuFuture, CpuPool};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6, ToSocketAddrs};

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

    pub fn resolve(&self, name: String) -> ResolveFuture {
        if let Some(addrs) = try_parse_ipaddr(&name) {
            return ResolveFuture::Ip(Some(addrs));
        }
        let collector = self.collector.clone();
        let fut = self.executor.spawn_fn(move || {
            collector.add(Event::DnsResolutionStarted);
            let ipaddrs = resolve(&name);
            collector.add(Event::DnsResolutionFinished);
            ipaddrs
        });
        ResolveFuture::Dns(fut)
    }
}

pub enum ResolveFuture {
    Ip(Option<IpAddrs>),
    Dns(CpuFuture<IpAddrs, std::io::Error>),
}

impl Future for ResolveFuture {
    type Item = IpAddrs;
    type Error = std::io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match *self {
            ResolveFuture::Ip(ref mut ip) => {
                Ok(Async::Ready(ip.take().expect("polled more than once")))
            }
            ResolveFuture::Dns(ref mut fut) => fut.poll(),
        }
    }
}

fn try_parse_ipaddr(host: &str) -> Option<IpAddrs> {
    if let Ok(addr) = host.parse::<Ipv4Addr>() {
        let addr = SocketAddrV4::new(addr, 0);
        Some(IpAddrs {
            inner: vec![SocketAddr::V4(addr)].into_iter(),
        })
    } else if let Ok(addr) = host.parse::<Ipv6Addr>() {
        let addr = SocketAddrV6::new(addr, 0, 0, 0);
        Some(IpAddrs {
            inner: vec![SocketAddr::V6(addr)].into_iter(),
        })
    } else {
        None
    }
}

fn resolve(name: &str) -> Result<IpAddrs, std::io::Error> {
    (name, 0)
        .to_socket_addrs()
        .map(|sockets| IpAddrs { inner: sockets })
}
