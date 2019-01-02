use crate::config::CaptureHeaderConfig;
use crate::config::TestConfig;
use http::response::Parts;
use http::HeaderMap;
use std::collections::HashMap;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::time::Duration;
use tracer_client::client::Metric;
use tracer_metrics::data::Snapshot;

pub struct TestReport {
    config: TestConfig,
    snapshots: Vec<Snapshot<Metric>>,
    res: Parts,
    body_hash: String,
    captured_headers: HashMap<String, String>,
}

impl TestReport {
    pub fn new(
        config: TestConfig,
        snapshots: Vec<Snapshot<Metric>>,
        res: Parts,
        body_hash: String,
    ) -> TestReport {
        let captured_headers = extract_configured_headers(&config.capture_headers, &res.headers);
        TestReport {
            config,
            snapshots,
            res,
            body_hash,
            captured_headers,
        }
    }
    pub fn take_config(self) -> TestConfig {
        self.config
    }
}

fn fmt_duration(d: &Duration) -> String {
    if d.as_secs() >= 5 {
        let s: f64 = d.as_secs() as f64 + (d.subsec_millis() as f64 / 1000.0);
        format!("{:.3}s", s)
    } else {
        format!("{}ms", (d.as_secs() * 1000) + (d.subsec_millis() as u64))
    }
}

fn extract_configured_headers(
    conf: &CaptureHeaderConfig,
    headers: &HeaderMap,
) -> HashMap<String, String> {
    headers
        .iter()
        .filter(|(k, _v)| match conf {
            CaptureHeaderConfig::All => true,
            CaptureHeaderConfig::List(ref whitelist) => {
                whitelist.contains(&k.as_str().to_lowercase())
            }
        })
        .map(|(k, v)| {
            (
                k.to_string(),
                v.to_str().unwrap_or("<Non-ASCII>").to_string(),
            )
        })
        .collect()
}

pub fn format_snapshot_stats(s: &Snapshot<Metric>) -> String {
    if let Some(h) = s.latency_histogram() {
        format!(
            "count {}/min {}/avg {}/max {}/stdev {}",
            s.count().unwrap_or(0),
            fmt_duration(&h.min()),
            fmt_duration(&h.mean()),
            fmt_duration(&h.max()),
            fmt_duration(&h.stdev())
        )
    } else {
        String::new()
    }
}

fn abbrev_metric(m: &Metric) -> &'static str {
    use tracer_client::client::Metric::*;
    match m {
        Dns => "DNS",
        Connection => "Conn",
        Tls => "TLS",
        Headers => "Hdrs",
        FullResponse => "Resp",
    }
}

fn format_snapshot(s: &Snapshot<Metric>, f: &mut Formatter) -> FmtResult {
    write!(
        f,
        "{}: {} ",
        abbrev_metric(&s.key()),
        fmt_duration(&s.gauge_as_duration().unwrap())
    )
}

impl Display for TestReport {
    fn fmt(&self, mut f: &mut Formatter) -> FmtResult {
        write!(
            f,
            "* {} ({}) Hash: {} ",
            self.config.name,
            self.res.status,
            &self.body_hash[0..8]
        )?;
        for s in &self.snapshots {
            format_snapshot(&s, &mut f)?;
        }
        if self.captured_headers.len() > 0 {
            for (k, v) in self.captured_headers.iter() {
                write!(f, "\n    {}: {}", k, v)?;
            }
        }

        Ok(())
    }
}
