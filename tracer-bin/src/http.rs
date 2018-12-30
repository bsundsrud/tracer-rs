use crate::config::{Config, PayloadConfig, TestConfig};
use crate::reporting::TestReport;
use failure::Error;
use futures::future;
use futures::prelude::*;
use tracer_client::client::Metric;

use http::Request;
use hyper::{Body, Chunk, Error as HyperError};
use sha2::{Digest, Sha256};
use slog;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
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

    pub fn execute_repeated_tests<R: Into<Option<usize>>>(
        self,
        repetitions: R,
    ) -> impl Future<Item = (), Error = ()> {
        let logger = self.logger.clone();
        let repetitions = repetitions.into();
        let chain = self.tests_and_collectors().map(move |(t, c)| {
            let err_logger = logger.clone();
            future::loop_fn((t, c, 1), move |(t, c, it)| {
                execute_test(c, t).and_then(move |(report, collector)| {
                    println!("{}", report);
                    if let Some(n) = repetitions {
                        if it >= n {
                            return Ok(future::Loop::Break((report.take_config(), collector, it)));
                        }
                    }
                    Ok(future::Loop::Continue((
                        report.take_config(),
                        collector,
                        it + 1,
                    )))
                })
            })
            .map(|_| ())
            .map_err(move |e| slog::error!(err_logger, "{}", e))
        });
        future::join_all(chain).map(|_| ())
    }
}

pub fn execute_test(
    mut collector: Collector<Metric>,
    config: TestConfig,
) -> impl Future<Item = (TestReport, Collector<Metric>), Error = HyperError> {
    let client = Client::new_with_collector_handle(collector.handle());

    let mut builder = Request::builder();
    builder.uri(config.url.clone()).method(&*config.method);
    let req: Request<Body> = match config.payload {
        None => builder.body(Body::empty()).unwrap(),
        Some(ref p) => match p {
            PayloadConfig::File { file: f } => {
                builder.body(load_payload_body(&f).unwrap().into()).unwrap()
            }
            PayloadConfig::Value { value: v } => builder.body(v.clone().into()).unwrap(),
        },
    };
    client.request(req).full_response().map(move |(res, body)| {
        collector.process_outstanding();
        let tr = TestReport::new(
            config,
            Metric::all_metrics(&collector),
            res,
            hash_body(body),
        );
        (tr, collector)
    })
}

fn hash_body(c: Chunk) -> String {
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
