use crate::connectors::TracingHttpsConnector;
use hyper::body::Bytes;
use hyper::client::Client as HyperClient;
use hyper::http::response::Parts;
use hyper::http::{Request, Response};
use hyper::Body;
use hyper::Error as HyperError;
use std::fmt;
use tracer_metrics::data::Snapshot;
use tracer_metrics::{Collector, CollectorHandle, Interest, Stopwatch};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Metric {
    Dns,
    Connection,
    Tls,
    Headers,
    FullResponse,
    HeaderLen,
    BodyLen,
}

impl Metric {
    pub fn all_metrics() -> &'static [Metric] {
        &[
            Metric::Dns,
            Metric::Connection,
            Metric::Tls,
            Metric::Headers,
            Metric::HeaderLen,
            Metric::FullResponse,
            Metric::BodyLen,
        ]
    }

    pub fn latency_metrics() -> &'static [Metric] {
        &[
            Metric::Dns,
            Metric::Connection,
            Metric::Tls,
            Metric::Headers,
            Metric::FullResponse,
        ]
    }

    pub fn size_metrics() -> &'static [Metric] {
        &[Metric::HeaderLen, Metric::BodyLen]
    }

    pub fn get_metrics(m: &[Metric], collector: &Collector<Metric>) -> Vec<Snapshot<Metric>> {
        m.iter()
            .filter_map(|m| {
                let snapshot = collector.snapshot(m);
                if snapshot.count().unwrap_or(0) > 0 {
                    Some(collector.snapshot(m))
                } else {
                    None
                }
            })
            .collect()
    }
    pub fn get_all_metrics(collector: &Collector<Metric>) -> Vec<Snapshot<Metric>> {
        Metric::get_metrics(Metric::all_metrics(), collector)
    }

    pub fn get_latency_metrics(collector: &Collector<Metric>) -> Vec<Snapshot<Metric>> {
        Metric::get_metrics(Metric::latency_metrics(), collector)
    }

    pub fn get_size_metrics(collector: &Collector<Metric>) -> Vec<Snapshot<Metric>> {
        Metric::get_metrics(Metric::size_metrics(), collector)
    }
}

impl fmt::Display for Metric {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", &self)?;
        Ok(())
    }
}

pub struct Client<C> {
    client: HyperClient<C, Body>,
    collector: CollectorHandle<Metric>,
}

impl Client<TracingHttpsConnector> {
    pub fn configure_collector_defaults(collector: &mut Collector<Metric>) {
        collector.register(Interest::Count(Metric::Connection));
        collector.register(Interest::Count(Metric::Dns));
        collector.register(Interest::Count(Metric::Tls));
        collector.register(Interest::Count(Metric::Headers));
        collector.register(Interest::Count(Metric::FullResponse));
        collector.register(Interest::Count(Metric::BodyLen));
        collector.register(Interest::Count(Metric::HeaderLen));

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
        collector.register(Interest::Gauge(Metric::BodyLen));
        collector.register(Interest::Gauge(Metric::HeaderLen));
    }

    pub fn new_with_collector_handle(
        handle: CollectorHandle<Metric>,
    ) -> Client<TracingHttpsConnector> {
        let connector = TracingHttpsConnector::new(true, handle.clone());
        let client = HyperClient::builder().keep_alive(false).build(connector);
        Client {
            client,
            collector: handle,
        }
    }
    pub fn new_with_collector(collector: &mut Collector<Metric>) -> Client<TracingHttpsConnector> {
        Client::configure_collector_defaults(collector);
        Client::new_with_collector_handle(collector.handle())
    }

    pub fn new_client_and_collector() -> (Client<TracingHttpsConnector>, Collector<Metric>) {
        let mut collector = Collector::new();
        let client = Client::new_with_collector(&mut collector);
        (client, collector)
    }

    pub async fn request(&self, req: Request<Body>) -> Result<Response<Body>, HyperError> {
        let handle = self.collector.clone();
        let stopwatch = Stopwatch::new();
        let resp = self.client.request(req).await?;
        handle.send(stopwatch.elapsed(Metric::Headers));
        Ok(resp)
    }

    pub async fn request_fully(&self, req: Request<Body>) -> Result<(Parts, Bytes), HyperError> {
        let handle = self.collector.clone();
        let stopwatch = Stopwatch::new();
        let resp = self.request(req).await?;
        let (headers, body) = resp.into_parts();
        let full_body = hyper::body::to_bytes(body).await?;
        handle.send(stopwatch.elapsed(Metric::FullResponse));
        Ok((headers, full_body))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn client_test() {
        let (c, collector) = Client::new_client_and_collector();
        let req = Request::builder()
            .uri("https://badssl.com/")
            .body(Body::empty())
            .unwrap();
        let fut = c.request_fully(req);
        let mut rt = tokio::runtime::Runtime::new().unwrap();
        match rt.block_on(fut) {
            Ok((res, body)) => {
                println!("status: {}", res.status);
                res.headers.iter().for_each(|(k, v)| {
                    println!("{:?}: {:?}", k, v);
                });
                println!("Length: {}", body.len());
                let handle = collector.handle();
                handle.send_value(Metric::BodyLen, body.len() as u64);
                collector.process_outstanding();
                let snapshots = Metric::get_all_metrics(&collector);
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
    }
}
