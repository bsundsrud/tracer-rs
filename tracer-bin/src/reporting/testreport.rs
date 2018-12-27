use crate::config::CaptureHeaderConfig;
use crate::config::TestConfig;
use crate::timing::Timing;
use http::response::Parts;
use http::HeaderMap;
use std::collections::HashMap;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::time::Duration;

pub struct TestReport {
    config: TestConfig,
    timings: Timing,
    res: Parts,
    body_hash: String,
    captured_headers: HashMap<String, String>,
}

impl TestReport {
    pub fn new(config: TestConfig, timings: Timing, res: Parts, body_hash: String) -> TestReport {
        let captured_headers = extract_configured_headers(&config.capture_headers, &res.headers);
        TestReport {
            config,
            timings,
            res,
            body_hash,
            captured_headers,
        }
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

impl Display for TestReport {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{} {}", self.config.name, self.res.status)?;
        let t = &self.timings;
        if let Some(d) = t.dns_resolution {
            write!(f, " | DNS: {:>7}", fmt_duration(&d))?;
        }
        if let Some(d) = t.connection {
            write!(f, " | C: {:>7}", fmt_duration(&d))?;
        }
        if let Some(d) = t.tls_negotiation {
            write!(f, " | TLS: {:>7}", fmt_duration(&d))?;
        }
        if let Some(d) = t.headers {
            write!(f, " | Hd: {:>7}", fmt_duration(&d))?;
        }
        if let Some(d) = t.full_response {
            write!(f, " | Resp: {:>7}", fmt_duration(&d))?;
        }
        write!(f, " | Hash: {}", &self.body_hash[0..8])?;
        for (k, v) in self.captured_headers.iter() {
            write!(f, "\n  {}: {}", k, v)?;
        }

        Ok(())
    }
}
