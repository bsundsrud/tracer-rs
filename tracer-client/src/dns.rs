use crate::client::Metric;
use crate::FutureResponse;
use futures::prelude::*;
use hyper::client::connect::dns::Name;
use hyper::service::Service;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6, ToSocketAddrs};
use std::task::Context;
use std::task::Poll;
use tracer_metrics::{CollectorHandle, Stopwatch};

#[derive(Clone)]
pub struct TracingResolver {
    collector: CollectorHandle<Metric>,
}

impl Service<Name> for TracingResolver {
    type Response = IpAddrs;
    type Error = std::io::Error;
    type Future = FutureResponse<Self::Response, Self::Error>;

    fn poll_ready(&mut self, _: &mut Context) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, name: Name) -> Self::Future {
        let collector = self.collector.clone();
        async move {
            if let Some(addrs) = try_parse_ipaddr(&name) {
                return Ok(addrs);
            }
            tokio::task::spawn_blocking(move || {
                let stopwatch = Stopwatch::new();
                let ipaddrs = resolve(&name);
                collector.send(stopwatch.elapsed(Metric::Dns));
                ipaddrs
            })
            .await?
        }
        .boxed()
    }
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
    pub fn new(collector: CollectorHandle<Metric>) -> TracingResolver {
        TracingResolver { collector }
    }
}

fn try_parse_ipaddr(host: &Name) -> Option<IpAddrs> {
    if let Ok(addr) = host.as_str().parse::<Ipv4Addr>() {
        let addr = SocketAddrV4::new(addr, 0);
        Some(IpAddrs {
            inner: vec![SocketAddr::V4(addr)].into_iter(),
        })
    } else if let Ok(addr) = host.as_str().parse::<Ipv6Addr>() {
        let addr = SocketAddrV6::new(addr, 0, 0, 0);
        Some(IpAddrs {
            inner: vec![SocketAddr::V6(addr)].into_iter(),
        })
    } else {
        None
    }
}

fn resolve(name: &Name) -> Result<IpAddrs, std::io::Error> {
    (name.as_str(), 0)
        .to_socket_addrs()
        .map(|sockets| IpAddrs { inner: sockets })
}
