use crate::config::{Config, PayloadConfig, TestConfig};
use crate::reporting::TestReport;
use crate::timing::Timing;
use failure::Error;
use futures::future;
use futures::prelude::*;

use http::Request;
use hyper::{Body, Chunk};
use sha2::{Digest, Sha256};
use slog;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use tracer_client::Client;

pub struct TestExecutor {
    config: Config,
    logger: slog::Logger,
}

impl TestExecutor {
    pub fn new(config: Config, logger: slog::Logger) -> TestExecutor {
        TestExecutor { config, logger }
    }

    pub fn execute_all_tests(self) -> impl Future<Item = (), Error = ()> {
        let logger = self.logger.clone();
        future::join_all(self.config.tests.into_iter().map(move |c| {
            let err_logger = logger.clone();
            let logger = logger.clone();
            execute_test(logger.clone(), c.clone())
                .inspect(|res| println!("{}", res))
                .map(|_| ())
                .map_err(move |e| slog::error!(err_logger, "{}", e))
        }))
        .map(|_| ())
    }
}

pub fn execute_test(
    logger: slog::Logger,
    config: TestConfig,
) -> impl Future<Item = TestReport, Error = Error> {
    let client = Client::new();

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
    let logger = logger.new(slog::o!("test" => config.name.clone()));
    client.request(req).full_response().then(move |result| {
        let (res, body, collector) = result?;
        let events = collector.drain_events();
        let t = Timing::from_events(events)?;
        slog::debug!(logger, "Response: {:?}", res);
        slog::debug!(logger, "Timings: {}", t);
        let r = TestReport::new(config, t, res, hash_body(body));
        Ok(r)
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
