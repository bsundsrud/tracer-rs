use crate::client::Metric;
use crate::dns::TracingResolver;
use crate::FutureResponse;
use futures::prelude::*;
use hyper::client::connect::dns::Name;
use hyper::service::Service;
use hyper::Uri;
use std::io;
use std::net::SocketAddr;
use std::str::FromStr;
use std::task::Context;
use std::task::Poll;
use tokio::net::TcpStream;
use tracer_metrics::{CollectorHandle, Stopwatch};

#[derive(Clone)]
pub struct TracingConnector {
    resolver: TracingResolver,
    collector: CollectorHandle<Metric>,
    nodelay: bool,
}

impl TracingConnector {
    pub fn new(collector: CollectorHandle<Metric>) -> TracingConnector {
        let resolver = TracingResolver::new(collector.clone());
        TracingConnector {
            resolver,
            collector,
            nodelay: false,
        }
    }

    pub fn set_nodelay(&mut self, nodelay: bool) {
        self.nodelay = nodelay;
    }
}

impl Service<Uri> for TracingConnector {
    type Response = TcpStream;
    type Error = std::io::Error;
    type Future = FutureResponse<Self::Response, Self::Error>;

    fn poll_ready(&mut self, _: &mut Context) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, dst: Uri) -> Self::Future {
        let host = match dst.host() {
            None => {
                return future::err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid Host"))
                    .boxed();
            }
            Some(host) => host.to_string(),
        };
        let is_https = dst.scheme().filter(|s| *s == "https").is_some();
        let port = dst
            .port_u16()
            .unwrap_or_else(|| if is_https { 443 } else { 80 });
        let nodelay = self.nodelay;
        let collector = self.collector.clone();
        let mut resolver = self.resolver.clone();
        async move {
            let name = Name::from_str(&host).map_err(|e| {
                io::Error::new(io::ErrorKind::InvalidInput, format!("Invalid Host: {}", e))
            })?;
            let addrs = resolver.call(name).await?;
            let addr = addrs.map(|a| SocketAddr::new(a, port)).next();
            let addr = if let Some(a) = addr {
                a
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Did not resolve an address",
                ));
            };
            let stopwatch = Stopwatch::new();
            let stream = TcpStream::connect(&addr).await?;
            collector.send(stopwatch.elapsed(Metric::Connection));
            stream.set_nodelay(nodelay)?;
            Ok(stream)
        }
        .boxed()
    }
}
