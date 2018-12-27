use crate::client::Metric;
use crate::dns::ResolveFuture;
use crate::dns::TracingResolver;
use futures::prelude::*;
use hyper::client::connect::Connect;
use hyper::client::connect::Connected;
use hyper::client::connect::Destination;
use std::io;
use std::net::SocketAddr;
use tokio_tcp::ConnectFuture;
use tokio_tcp::TcpStream;
use tracer_metrics::{CollectorHandle, Stopwatch};

#[derive(Clone)]
pub struct TracingConnector {
    resolver: TracingResolver,
    collector: CollectorHandle<Metric>,
    nodelay: bool,
}

impl TracingConnector {
    pub fn new(threads: usize, collector: CollectorHandle<Metric>) -> TracingConnector {
        let resolver = TracingResolver::new(threads, collector.clone());
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

impl Connect for TracingConnector {
    type Transport = TcpStream;
    type Error = std::io::Error;
    type Future = TracingConnecting;

    fn connect(&self, dst: Destination) -> Self::Future {
        let host = match dst.host() {
            "" => {
                return TracingConnecting {
                    collector: self.collector.clone(),
                    nodelay: self.nodelay,
                    port: 0,
                    state: invalid_url(),
                };
            }
            host => host.to_string(),
        };
        let is_https = dst.scheme() == "https";
        let port = dst
            .port()
            .unwrap_or_else(|| if is_https { 443 } else { 80 });
        TracingConnecting {
            collector: self.collector.clone(),
            nodelay: self.nodelay,
            port,
            state: State::Resolving(self.resolver.resolve(host)),
        }
    }
}

fn invalid_url() -> State {
    State::Error(Some(io::Error::new(
        io::ErrorKind::InvalidInput,
        "Did not resolve an address",
    )))
}

pub struct TracingConnecting {
    collector: CollectorHandle<Metric>,
    nodelay: bool,
    port: u16,
    state: State,
}

enum State {
    Resolving(ResolveFuture),
    Connecting(ConnectFuture, Stopwatch),
    Error(Option<std::io::Error>),
}

impl Future for TracingConnecting {
    type Item = (TcpStream, Connected);
    type Error = std::io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            let state;
            match self.state {
                State::Resolving(ref mut r) => match r.poll()? {
                    Async::NotReady => return Ok(Async::NotReady),
                    Async::Ready(addrs) => {
                        let port = self.port;
                        let addr = addrs.map(|a| SocketAddr::new(a, port)).next();
                        state = match addr {
                            Some(a) => State::Connecting(TcpStream::connect(&a), Stopwatch::new()),
                            None => invalid_url(),
                        };
                    }
                },
                State::Connecting(ref mut fut, ref stopwatch) => match fut.poll()? {
                    Async::NotReady => return Ok(Async::NotReady),
                    Async::Ready(stream) => {
                        self.collector.send(stopwatch.elapsed(Metric::Connection));
                        stream.set_nodelay(self.nodelay)?;
                        let connected = Connected::new();
                        return Ok(Async::Ready((stream, connected)));
                    }
                },
                State::Error(ref mut e) => return Err(e.take().expect("polled more than once")),
            }
            self.state = state;
        }
    }
}
