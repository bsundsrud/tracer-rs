use events::{Event, EventCollector};
use failure::Error;
use futures::future;
use futures::future::FutureResult;
use futures::prelude::*;
use futures_cpupool::CpuPool;
use http::Uri;
use std::net::IpAddr;
use std::net::ToSocketAddrs;
use std::sync::Arc;
use std::sync::RwLock;
use tower_service::{NewService, Service};

pub struct HostDnsService {
    pool: CpuPool,
    collector: EventCollector,
}

pub struct NewHostDnsService {
    pool: CpuPool,
    collector: EventCollector,
}

impl NewHostDnsService {
    pub fn new(threads: usize) -> Self {
        Self {
            pool: CpuPool::new(threads),
            collector: EventCollector::new(),
        }
    }
}

impl NewService for NewHostDnsService {
    type Request = Uri;
    type Response = IpAddrs;
    type Error = DnsError;
    type InitError = DnsError;
    type Service = HostDnsService;
    type Future = FutureResult<Self::Service, Self::InitError>;

    fn new_service(&self) -> Self::Future {
        future::ok(HostDnsService {
            pool: self.pool.clone(),
            collector: self.collector.clone(),
        })
    }
}

#[derive(Debug)]
pub struct IpAddrs {
    pub addrs: Vec<IpAddr>,
}

#[derive(Debug, Fail)]
pub enum DnsError {
    #[fail(display = "Could not create new service")]
    NewServiceFailed,
    #[fail(display = "Could not resolve host {}: {}", _0, _1)]
    ResolutionFailed(String, Error),
}

impl Service for HostDnsService {
    type Request = Uri;
    type Response = IpAddrs;
    type Error = DnsError;
    type Future = Box<Future<Item = IpAddrs, Error = DnsError>>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        Ok(Async::Ready(()))
    }

    fn call(&mut self, req: Uri) -> Self::Future {
        let host_str = req.host().unwrap_or("127.0.0.1").to_string();
        let host = host_str.clone();
        let collector = self.collector.clone();
        let fut = self
            .pool
            .spawn_fn(move || {
                // Is it already an IP?
                if let Some(ip) = host.parse::<IpAddr>().ok() {
                    collector.add(Event::DnsResolved);
                    return Ok(IpAddrs { addrs: vec![ip] });
                }
                // Not already an IP, do lookup
                let resolved = format!("{}:{}", host, 80).to_socket_addrs()?;
                collector.add(Event::DnsResolved);
                Ok(IpAddrs {
                    addrs: resolved.map(|s| s.ip()).collect(),
                })
            }).map_err(|e: Error| DnsError::ResolutionFailed(host_str, e.into()));
        Box::new(fut)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::net::Ipv4Addr;
    use std::time::Instant;
    #[test]
    fn resolve_addesses() {
        let started = Instant::now();
        let factory = NewHostDnsService::new(1);
        let mut service = factory.new_service().wait().unwrap();
        let fut = service.call("http://google.com".parse().unwrap());
        let ipaddrs = fut.wait().unwrap();
        let localhost = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        //assert_eq!(ipaddrs.addrs.len(), 1);
        //assert_eq!(ipaddrs.addrs[0], localhost);
        factory.collector.since(started).iter().for_each(|(e, d)| {
            println!("{:?}: {:?}", e, d);
        });
    }
}
