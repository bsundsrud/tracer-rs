use super::http::TracingConnector;
use crate::client::Metric;
use crate::FutureResponse;
use futures::prelude::*;
use hyper::service::Service;
use hyper::Uri;
use hyper_rustls::HttpsConnector;
use hyper_rustls::MaybeHttpsStream;
use rustls::ClientConfig;
use std::convert::From;
use std::error::Error;
use std::io;
use std::sync::Arc;
use std::task::Context;
use std::task::Poll;
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tracer_metrics::{CollectorHandle, Stopwatch};
use webpki::DNSNameRef;

#[derive(Clone)]
pub struct TracingHttpsConnector {
    http: TracingConnector,
    tls_config: Arc<ClientConfig>,
    collector: CollectorHandle<Metric>,
}

impl TracingHttpsConnector {
    pub fn new(nodelay: bool, collector: CollectorHandle<Metric>) -> TracingHttpsConnector {
        let mut http = TracingConnector::new(collector.clone());
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

impl Service<Uri> for TracingHttpsConnector {
    type Response = <HttpsConnector<TracingConnector> as Service<Uri>>::Response;
    type Error = <HttpsConnector<TracingConnector> as Service<Uri>>::Error;
    type Future =
        FutureResponse<MaybeHttpsStream<TcpStream>, Box<dyn Error + Send + Sync + 'static>>;

    fn call(&mut self, dst: Uri) -> Self::Future {
        let collector = self.collector.clone();
        let connecting = self.http.call(dst.clone());
        let cfg = self.tls_config.clone();
        async move {
            let is_https = dst.scheme().filter(|s| *s == "https").is_some();
            let tcp = connecting.await?;
            if !is_https {
                return Ok(MaybeHttpsStream::Http(tcp));
            }

            let connector = TlsConnector::from(cfg);
            let hostname = if let Some(h) = dst.host() {
                h.to_string()
            } else {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "Missing Host").into());
            };

            let dnsname = match DNSNameRef::try_from_ascii_str(&hostname) {
                Ok(dnsname) => dnsname,
                Err(e) => {
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        format!("invalid dnsname: {}", e),
                    )
                    .into())
                }
            };
            let stopwatch = Stopwatch::new();
            let tls = connector.connect(dnsname, tcp).await?;
            collector.send(stopwatch.elapsed(Metric::Tls));
            Ok(MaybeHttpsStream::Https(tls))
        }
        .boxed()
    }
    fn poll_ready(&mut self, _: &mut Context) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}
