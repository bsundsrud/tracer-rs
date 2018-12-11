use connectors::TracingHttpsConnector;
use events::{Event, EventCollector};
use futures::future;
use futures::prelude::*;
use hyper::body::Payload;
use hyper::client::connect::Connect;
use hyper::client::Client as HyperClient;
use hyper::http::{Request, Response};
use hyper::Body;
use hyper::Error as HyperError;
use std::fmt;

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
        let connector = TracingHttpsConnector::new(4, collector.clone());
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
        let collector2 = self.collector.clone();
        let start = future::lazy(move || {
            collector.add(Event::Initiated);
            future::ok::<(), ()>(())
        });
        let send = self.client.request(req);
        let fut = start
            .then(|_| send)
            .inspect(move |_resp| collector2.add(Event::HeadersReceived));
        ResponseFuture::new(Box::new(fut))
    }
}

#[must_use = "futures do nothing unless polled"]
pub struct ResponseFuture {
    inner: Box<Future<Item = Response<Body>, Error = HyperError> + Send>,
}

impl ResponseFuture {
    fn new(fut: Box<Future<Item = Response<Body>, Error = HyperError> + Send>) -> Self {
        Self { inner: fut }
    }
}

impl fmt::Debug for ResponseFuture {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.pad("Future<Response>")
    }
}

impl Future for ResponseFuture {
    type Item = Response<Body>;
    type Error = HyperError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.inner.poll()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn client_test() {
        let c = Client::new();
        let collector = c.collector();
        let req = Request::builder()
            .method("GET")
            .uri("https://www.google.com")
            .body(Body::empty())
            .unwrap();
        let fut = c
            .request(req)
            .and_then(|res: Response<_>| {
                println!("status: {}", res.status());
                res.headers().iter().for_each(|(k, v)| {
                    println!("{:?}: {:?}", k, v);
                });
                res.into_body().concat2()
            })
            .and_then(|body| {
                println!("Length: {}", body.len());
                Ok(())
            })
            .inspect(move |_| {
                collector.add(Event::FullResponse);
                collector.since_initiated().for_each(|(e, d)| {
                    println!("{:?}: {:?}", e, d);
                });
            })
            .map_err(|e| {
                println!("Error: {}", e);
            });
        let mut rt = tokio::runtime::Runtime::new().unwrap();
        rt.spawn(fut);
        rt.shutdown_on_idle().wait().unwrap();
    }
}
