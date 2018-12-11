use super::http::TracingConnector;
use events::{Event, EventCollector};
use futures::prelude::*;
use hyper::client::connect::Connect;
use hyper::client::connect::Connected;
use hyper::client::connect::Destination;
use hyper_rustls::HttpsConnector;
use hyper_rustls::MaybeHttpsStream;
use rustls::ClientConfig;
use std::convert::From;

pub struct TracingHttpsConnector {
    https: HttpsConnector<TracingConnector>,
    collector: EventCollector,
}

impl TracingHttpsConnector {
    pub fn new(threads: usize, collector: EventCollector) -> TracingHttpsConnector {
        let mut http = TracingConnector::new(threads, collector.clone());
        http.connector().enforce_http(false);
        let mut config = ClientConfig::new();
        config
            .root_store
            .add_server_trust_anchors(&webpki_roots::TLS_SERVER_ROOTS);
        let https = (http, config).into();
        TracingHttpsConnector { https, collector }
    }
}

impl From<(TracingConnector, ClientConfig, EventCollector)> for TracingHttpsConnector {
    fn from(args: (TracingConnector, ClientConfig, EventCollector)) -> TracingHttpsConnector {
        TracingHttpsConnector {
            https: (args.0, args.1).into(),
            collector: args.2,
        }
    }
}

impl Connect for TracingHttpsConnector {
    type Transport = <HttpsConnector<TracingConnector> as Connect>::Transport;
    type Error = <HttpsConnector<TracingConnector> as Connect>::Error;
    type Future = TracingHttpsConnecting<<TracingConnector as Connect>::Transport>;

    fn connect(&self, dst: Destination) -> Self::Future {
        TracingHttpsConnecting {
            fut: Box::new(self.https.connect(dst)),
            collector: self.collector.clone(),
        }
    }
}

pub struct TracingHttpsConnecting<T> {
    fut: Box<Future<Item = (MaybeHttpsStream<T>, Connected), Error = std::io::Error> + Send>,
    collector: EventCollector,
}

impl<T> Future for TracingHttpsConnecting<T> {
    type Item = (MaybeHttpsStream<T>, Connected);
    type Error = std::io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let val = match self.fut.poll()? {
            Async::NotReady => Async::NotReady,
            Async::Ready(v) => {
                self.collector.add(Event::TlsNegotiated);
                Async::Ready(v)
            }
        };
        Ok(val)
    }
}
