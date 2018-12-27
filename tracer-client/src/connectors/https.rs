use super::http::TracingConnector;
use crate::client::Metric;
use futures::prelude::*;
use hyper::client::connect::Connect;
use hyper::client::connect::Connected;
use hyper::client::connect::Destination;
use hyper_rustls::HttpsConnector;
use hyper_rustls::MaybeHttpsStream;
use rustls::ClientConfig;
use std::convert::From;
use std::io;
use std::sync::Arc;
use tokio_rustls::TlsConnector;
use tracer_metrics::{CollectorHandle, Stopwatch};
use webpki::{DNSName, DNSNameRef};

#[derive(Clone)]
pub struct TracingHttpsConnector {
    http: TracingConnector,
    tls_config: Arc<ClientConfig>,
    collector: CollectorHandle<Metric>,
}

impl TracingHttpsConnector {
    pub fn new(
        nodelay: bool,
        threads: usize,
        collector: CollectorHandle<Metric>,
    ) -> TracingHttpsConnector {
        let mut http = TracingConnector::new(threads, collector.clone());
        http.set_nodelay(nodelay);
        let mut config = ClientConfig::new();
        config
            .root_store
            .add_server_trust_anchors(&webpki_roots::TLS_SERVER_ROOTS);
        TracingHttpsConnector {
            http,
            tls_config: Arc::new(config),
            collector,
        }
    }
}

impl From<(TracingConnector, ClientConfig, CollectorHandle<Metric>)> for TracingHttpsConnector {
    fn from(
        args: (TracingConnector, ClientConfig, CollectorHandle<Metric>),
    ) -> TracingHttpsConnector {
        TracingHttpsConnector {
            http: args.0,
            tls_config: Arc::new(args.1),
            collector: args.2,
        }
    }
}

impl Connect for TracingHttpsConnector {
    type Transport = <HttpsConnector<TracingConnector> as Connect>::Transport;
    type Error = <HttpsConnector<TracingConnector> as Connect>::Error;
    type Future = TracingHttpsConnecting<<TracingConnector as Connect>::Transport>;

    fn connect(&self, dst: Destination) -> Self::Future {
        let is_https = dst.scheme() == "https";
        let hostname = dst.host().to_string();
        let connecting = self.http.connect(dst);
        let collector = self.collector.clone();
        if !is_https {
            let fut = connecting.map(|(tcp, conn)| (MaybeHttpsStream::Http(tcp), conn));
            TracingHttpsConnecting(Box::new(fut))
        } else {
            let cfg = self.tls_config.clone();
            let connector = TlsConnector::from(cfg);
            let fut = connecting
                .map(|(tcp, conn)| (tcp, conn, hostname))
                .and_then(
                    |(tcp, conn, hostname)| match DNSNameRef::try_from_ascii_str(&hostname) {
                        Ok(dnsname) => Ok((tcp, conn, DNSName::from(dnsname))),
                        Err(_) => Err(io::Error::new(io::ErrorKind::Other, "invalid dnsname")),
                    },
                )
                .and_then(move |(tcp, conn, dnsname)| {
                    let stopwatch = Stopwatch::new();
                    connector
                        .connect(dnsname.as_ref(), tcp)
                        .inspect(move |_| collector.send(stopwatch.elapsed(Metric::Tls)))
                        .and_then(|tls| Ok((MaybeHttpsStream::Https(tls), conn)))
                        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
                });
            TracingHttpsConnecting(Box::new(fut))
        }
    }
}

pub struct TracingHttpsConnecting<T>(
    Box<Future<Item = (MaybeHttpsStream<T>, Connected), Error = io::Error> + Send>,
);

impl<T> Future for TracingHttpsConnecting<T> {
    type Item = (MaybeHttpsStream<T>, Connected);
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.0.poll()
    }
}
