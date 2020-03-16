use crate::config::{Config, PayloadConfig, TestConfig};
use crate::interrupt::Interrupted;
use crate::reporting::TestReport;
use anyhow::Error;
use futures::future;
use futures::TryFutureExt;
use http::header::HeaderValue;
use http::HeaderMap;
use http::Request;
use hyper::{body::Bytes, Body, Error as HyperError};
use sha2::{Digest, Sha256};
use slog;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use tracer_client::client::Metric;
use tracer_client::Client;
use tracer_metrics::Collector;

pub struct TestExecutor {
    config: Config,
    logger: slog::Logger,
}

impl TestExecutor {
    pub fn new(config: Config, logger: slog::Logger) -> TestExecutor {
        TestExecutor { config, logger }
    }

    fn tests_and_collectors(self) -> impl Iterator<Item = (TestConfig, Collector<Metric>)> {
        self.config.tests.into_iter().map(|t| {
            let mut c = Collector::new();
            Client::configure_collector_defaults(&mut c);
            (t, c)
        })
    }

    pub async fn execute_repeated_tests<R: Into<Option<usize>>>(
        self,
        repetitions: R,
        interrupted: Interrupted,
    ) -> Vec<Result<(TestConfig, Collector<Metric>), ()>> {
        let logger = self.logger.clone();
        let repetitions = repetitions.into();
        let chain = self
            .tests_and_collectors()
            .map(|(t, c)| async {
                let interrupted = interrupted.clone();
                let mut iterations = 0;
                let mut test = t;
                let mut collector = c;
                while !interrupted.interrupted() {
                    let (report, c) = execute_test(test, collector).await?;
                    println!("{}", report);
                    test = report.take_config();
                    collector = c;
                    iterations += 1;
                    if let Some(n) = repetitions {
                        if iterations >= n {
                            break;
                        }
                    }
                }
                Ok::<_, HyperError>((test, collector))
            })
            .map(|f| f.map_err(|e| slog::error!(logger, "{}", e)));
        future::join_all(chain).await
    }
}

fn calculate_header_size(h: &HeaderMap<HeaderValue>) -> usize {
    // Assume header is in the canonical form of <HEADER-NAME><COLON><SPACE><HEADER-VALUE>\r\n
    h.keys()
        .map::<usize, _>(|k| h.len() + h.get_all(k).iter().map(|v| v.len()).sum::<usize>() + 4)
        .sum::<usize>() as usize
}

pub async fn execute_test(
    config: TestConfig,
    collector: Collector<Metric>,
) -> Result<(TestReport, Collector<Metric>), HyperError> {
    let client = Client::new_with_collector_handle(collector.handle());

    let mut builder = Request::builder()
        .uri(config.url.clone())
        .method(&*config.method);
    for (k, v) in config.headers.iter() {
        builder = builder.header(k.as_str(), v.as_str());
    }
    let req: Request<Body> = match config.payload {
        None => builder.body(Body::empty()).unwrap(),
        Some(ref p) => match p {
            PayloadConfig::File { file: f } => {
                builder.body(load_payload_body(&f).unwrap().into()).unwrap()
            }
            PayloadConfig::Value { value: v } => builder.body(v.clone().into()).unwrap(),
        },
    };
    let (res, body) = client.request_fully(req).await?;
    let body_size = body.len();
    let header_size = calculate_header_size(&res.headers);
    let handle = collector.handle();
    handle.send_value(Metric::BodyLen, body_size as u64);
    handle.send_value(Metric::HeaderLen, header_size as u64);
    collector.process_outstanding();
    let tr = TestReport::new(
        config,
        Metric::get_all_metrics(&collector),
        res,
        hash_body(body),
    );
    Ok((tr, collector))
}

fn hash_body(c: Bytes) -> String {
    let mut h = Sha256::default();
    h.input(c);
    format!("{:x}", h.result())
}

fn load_payload_body<P: AsRef<Path>>(path: P) -> Result<String, Error> {
    let mut f = File::open(path.as_ref())?;
    let mut contents = String::new();
    f.read_to_string(&mut contents)?;
    Ok(contents)
}
