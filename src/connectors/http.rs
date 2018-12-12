use dns::ResolveFuture;
use dns::TracingResolver;
use events::{Event, EventCollector};
use futures::prelude::*;
use hyper::client::connect::Connect;
use hyper::client::connect::Connected;
use hyper::client::connect::Destination;
use std::io;
use std::net::SocketAddr;
use tokio_tcp::ConnectFuture;
use tokio_tcp::TcpStream;

#[derive(Clone)]
pub struct TracingConnector {
    resolver: TracingResolver,
    collector: EventCollector,
    nodelay: bool,
}

impl TracingConnector {
    pub fn new(threads: usize, collector: EventCollector) -> TracingConnector {
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
    collector: EventCollector,
    nodelay: bool,
    port: u16,
    state: State,
}

enum State {
    Resolving(ResolveFuture),
    Connecting(ConnectFuture),
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
                            Some(a) => State::Connecting(TcpStream::connect(&a)),
                            None => invalid_url(),
                        };
                        self.collector.add(Event::ConnectionStarted);
                    }
                },
                State::Connecting(ref mut fut) => match fut.poll()? {
                    Async::NotReady => return Ok(Async::NotReady),
                    Async::Ready(stream) => {
                        stream.set_nodelay(self.nodelay)?;
                        let connected = Connected::new();
                        self.collector.add(Event::Connected);
                        return Ok(Async::Ready((stream, connected)));
                    }
                },
                State::Error(ref mut e) => return Err(e.take().expect("polled more than once")),
            }
            self.state = state;
        }
    }
}
