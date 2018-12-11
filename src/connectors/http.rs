use dns::TracingResolver;
use events::{Event, EventCollector};
use futures::prelude::*;
use hyper::client::connect::Connect;
use hyper::client::connect::Connected;
use hyper::client::connect::Destination;
use hyper::client::HttpConnector;
use tokio_tcp::TcpStream;

pub struct TracingConnector {
    http: HttpConnector<TracingResolver>,
    collector: EventCollector,
}

impl TracingConnector {
    pub fn new(threads: usize, collector: EventCollector) -> TracingConnector {
        let resolver = TracingResolver::new(threads, collector.clone());
        let http = HttpConnector::new_with_resolver(resolver);
        TracingConnector { http, collector }
    }

    pub fn connector(&mut self) -> &mut HttpConnector<TracingResolver> {
        &mut self.http
    }
}

impl Connect for TracingConnector {
    type Transport = <HttpConnector<TracingResolver> as Connect>::Transport;
    type Error = <HttpConnector<TracingResolver> as Connect>::Error;
    type Future = TracingConnecting;

    fn connect(&self, dst: Destination) -> Self::Future {
        let conn = self.http.connect(dst);

        TracingConnecting {
            fut: Box::new(conn),
            collector: self.collector.clone(),
        }
    }
}

pub struct TracingConnecting {
    fut: Box<Future<Item = (TcpStream, Connected), Error = std::io::Error> + Send>,
    collector: EventCollector,
}

impl Future for TracingConnecting {
    type Item = (TcpStream, Connected);
    type Error = std::io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let val = match self.fut.poll()? {
            Async::NotReady => Async::NotReady,
            Async::Ready(v) => {
                self.collector.add(Event::Connected);
                Async::Ready(v)
            }
        };
        Ok(val)
    }
}
