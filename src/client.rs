use connectors::TracingHttpsConnector;
use events::{Event, EventCollector};
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

pub struct Client<C, B> {
    client: HyperClient<C, B>,
    collector: EventCollector,
}

impl Client<TracingHttpsConnector, Body> {
    pub fn new() -> Client<TracingHttpsConnector, Body> {
        Client::default()
    }
}

impl Default for Client<TracingHttpsConnector, Body> {
    fn default() -> Client<TracingHttpsConnector, Body> {
        let collector = EventCollector::new();
        let connector = TracingHttpsConnector::new(false, 4, collector.clone());
        let client = HyperClient::builder().keep_alive(false).build(connector);
        Client { client, collector }
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
    pub fn collector(&self) -> EventCollector {
        self.collector.clone()
    }

    pub fn request(&self, req: Request<B>) -> ResponseFuture {
        let collector = self.collector.clone();
        let send = self.client.request(req);
        let fut = future::lazy(move || {
            collector.add(Event::Initiated);
            send.inspect(move |_resp| collector.add(Event::HeadersReceived))
        });
        ResponseFuture::new(Box::new(fut), self.collector.clone())
    }
}

#[must_use = "futures do nothing unless polled"]
pub struct ResponseFuture {
    inner: Box<Future<Item = Response<Body>, Error = HyperError> + Send>,
    collector: EventCollector,
}

impl ResponseFuture {
    fn new(
        fut: Box<Future<Item = Response<Body>, Error = HyperError> + Send>,
        collector: EventCollector,
    ) -> Self {
        Self {
            inner: fut,
            collector,
        }
    }

    pub fn full_response(
        self,
    ) -> impl Future<Item = (Parts, Chunk, EventCollector), Error = HyperError> {
        self.and_then(|(res, collector)| {
            let (parts, body) = res.into_parts();
            FullResponseFuture {
                collector,
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
    type Item = (Response<Body>, EventCollector);
    type Error = HyperError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.inner.poll() {
            Ok(v) => match v {
                Async::Ready(v) => Ok(Async::Ready((v, self.collector.clone()))),
                Async::NotReady => Ok(Async::NotReady),
            },
            Err(e) => {
                self.collector.add(Event::ConnectionError);
                Err(e)
            }
        }
    }
}

#[must_use = "futures do nothing unless polled"]
pub struct FullResponseFuture {
    collector: EventCollector,
    parts: Option<Parts>,
    body: Concat2<Body>,
}

impl fmt::Debug for FullResponseFuture {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.pad("Future<(Parts, Chunk)>")
    }
}

impl Future for FullResponseFuture {
    type Item = (Parts, Chunk, EventCollector);
    type Error = HyperError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let val = match self.body.poll()? {
            Async::NotReady => Async::NotReady,
            Async::Ready(c) => {
                self.collector.add(Event::FullResponse);
                Async::Ready((
                    self.parts.take().expect("polled more than once"),
                    c,
                    self.collector.clone(),
                ))
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
        let c = Client::new();
        let req = Request::builder()
            .uri("https://badssl.com/")
            .body(Body::empty())
            .unwrap();
        let fut = c.request(req).full_response();
        let mut rt = tokio::runtime::Runtime::new().unwrap();
        match rt.block_on(fut) {
            Ok((res, body, collector)) => {
                println!("status: {}", res.status);
                res.headers.iter().for_each(|(k, v)| {
                    println!("{:?}: {:?}", k, v);
                });
                println!("Length: {}", body.len());
                collector.since_initiated().for_each(|(e, d)| {
                    println!("{:?}: {:?}", e, d);
                });
            }
            Err(e) => println!("ERROR: {}", e),
        }

        rt.shutdown_on_idle().wait().unwrap();
    }
}
