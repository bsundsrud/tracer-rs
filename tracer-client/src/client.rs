use crate::connectors::TracingHttpsConnector;
use futures::future;
use futures::prelude::*;
use hyper::body::Payload;
use hyper::client::connect::Connect;
use hyper::client::Client as HyperClient;
use hyper::http::response::Parts;
use hyper::http::{Request, Response};
use hyper::Body;
use hyper::Chunk;
use hyper::Error as HyperError;
use std::fmt;
use tokio::prelude::stream::Concat2;
use tracer_metrics::data::Snapshot;
use tracer_metrics::{Collector, CollectorHandle, Interest, Stopwatch};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Metric {
    Dns,
    Connection,
    Tls,
    Headers,
    FullResponse,
}

impl Metric {
    pub fn all_metrics(collector: &Collector<Metric>) -> Vec<Snapshot<Metric>> {
        static ALL_METRICS: &'static [Metric] = &[
            Metric::Dns,
            Metric::Connection,
            Metric::Tls,
            Metric::Headers,
            Metric::FullResponse,
        ];
        ALL_METRICS.iter().map(|m| collector.snapshot(m)).collect()
    }
}

impl fmt::Display for Metric {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", &self)?;
        Ok(())
    }
}

pub struct Client<C, B> {
    client: HyperClient<C, B>,
    collector: CollectorHandle<Metric>,
}

impl Client<TracingHttpsConnector, Body> {
    pub fn configure_collector_defaults(collector: &mut Collector<Metric>) {
        collector.register(Interest::Count(Metric::Connection));
        collector.register(Interest::Count(Metric::Dns));
        collector.register(Interest::Count(Metric::Tls));
        collector.register(Interest::Count(Metric::Headers));
        collector.register(Interest::Count(Metric::FullResponse));

        collector.register(Interest::LatencyPercentile(Metric::Connection));
        collector.register(Interest::LatencyPercentile(Metric::Dns));
        collector.register(Interest::LatencyPercentile(Metric::Tls));
        collector.register(Interest::LatencyPercentile(Metric::Headers));
        collector.register(Interest::LatencyPercentile(Metric::FullResponse));

        collector.register(Interest::Gauge(Metric::Connection));
        collector.register(Interest::Gauge(Metric::Dns));
        collector.register(Interest::Gauge(Metric::Tls));
        collector.register(Interest::Gauge(Metric::Headers));
        collector.register(Interest::Gauge(Metric::FullResponse));
    }

    pub fn new_with_collector_handle(
        handle: CollectorHandle<Metric>,
    ) -> Client<TracingHttpsConnector, Body> {
        let connector = TracingHttpsConnector::new(true, 4, handle.clone());
        let client = HyperClient::builder().keep_alive(false).build(connector);
        Client {
            client,
            collector: handle,
        }
    }
    pub fn new_with_collector(
        collector: &mut Collector<Metric>,
    ) -> Client<TracingHttpsConnector, Body> {
        Client::configure_collector_defaults(collector);
        Client::new_with_collector_handle(collector.handle())
    }

    pub fn new_client_and_collector() -> (Client<TracingHttpsConnector, Body>, Collector<Metric>) {
        let mut collector = Collector::new();
        let client = Client::new_with_collector(&mut collector);
        (client, collector)
    }
}

impl<C, B> Client<C, B>
where
    C: Connect + Sync + 'static,
    C::Transport: 'static,
    C::Future: 'static,
    B: Payload + Send + 'static,
    B::Data: Send,
{
    pub fn request(&self, req: Request<B>) -> ResponseFuture {
        let handle = self.collector.clone();
        let send = self.client.request(req);
        let fut = future::lazy(move || {
            let stopwatch = Stopwatch::new();
            send.map(move |resp| {
                handle.send(stopwatch.elapsed(Metric::Headers));
                (resp, stopwatch)
            })
        });
        ResponseFuture::new(Box::new(fut), self.collector.clone())
    }
}

#[must_use = "futures do nothing unless polled"]
pub struct ResponseFuture {
    inner: Box<Future<Item = (Response<Body>, Stopwatch), Error = HyperError> + Send>,
    collector: CollectorHandle<Metric>,
}

impl ResponseFuture {
    fn new(
        fut: Box<Future<Item = (Response<Body>, Stopwatch), Error = HyperError> + Send>,
        collector: CollectorHandle<Metric>,
    ) -> Self {
        Self {
            inner: fut,
            collector,
        }
    }

    pub fn full_response(self) -> impl Future<Item = (Parts, Chunk), Error = HyperError> {
        let collector = self.collector.clone();
        self.and_then(|(res, stopwatch)| {
            let (parts, body) = res.into_parts();
            FullResponseFuture {
                stopwatch,
                collector: collector,
                parts: Some(parts),
                body: body.concat2(),
            }
        })
    }
}

impl fmt::Debug for ResponseFuture {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.pad("Future<Response>")
    }
}

impl Future for ResponseFuture {
    type Item = (Response<Body>, Stopwatch);
    type Error = HyperError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.inner.poll()
    }
}

#[must_use = "futures do nothing unless polled"]
pub struct FullResponseFuture {
    stopwatch: Stopwatch,
    collector: CollectorHandle<Metric>,
    parts: Option<Parts>,
    body: Concat2<Body>,
}

impl fmt::Debug for FullResponseFuture {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.pad("Future<(Parts, Chunk)>")
    }
}

impl Future for FullResponseFuture {
    type Item = (Parts, Chunk);
    type Error = HyperError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let val = match self.body.poll()? {
            Async::NotReady => Async::NotReady,
            Async::Ready(c) => {
                self.collector
                    .send(self.stopwatch.elapsed(Metric::FullResponse));
                Async::Ready((self.parts.take().expect("polled more than once"), c))
            }
        };
        Ok(val)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn client_test() {
        let (c, mut collector) = Client::new_client_and_collector();
        let req = Request::builder()
            .uri("https://badssl.com/")
            .body(Body::empty())
            .unwrap();
        let fut = c.request(req).full_response();
        let mut rt = tokio::runtime::Runtime::new().unwrap();
        match rt.block_on(fut) {
            Ok((res, body)) => {
                println!("status: {}", res.status);
                res.headers.iter().for_each(|(k, v)| {
                    println!("{:?}: {:?}", k, v);
                });
                println!("Length: {}", body.len());

                collector.process_outstanding();
                let snapshots = Metric::all_metrics(&collector);
                for snapshot in &snapshots {
                    println!(
                        "{}: {:?}",
                        snapshot.key(),
                        snapshot.gauge_as_duration().unwrap()
                    );
                }
            }
            Err(e) => println!("ERROR: {}", e),
        }

        rt.shutdown_on_idle().wait().unwrap();
    }
}
