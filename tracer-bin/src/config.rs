use failure::{Error, Fail};
use hyper::Uri;
use serde_derive::Deserialize;
use std::collections::HashSet;
use std::convert::From;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use toml;

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum PayloadConfig {
    File { file: String },
    Value { value: String },
}

impl PayloadConfig {
    fn make_absolute(mut self, parent: &Path) -> PayloadConfig {
        match self {
            PayloadConfig::File { ref mut file } => {
                *file = parent.join(&file).to_string_lossy().into();
            }
            PayloadConfig::Value { .. } => {}
        }
        self
    }
}

#[derive(Debug, Deserialize)]
pub struct CaptureHeaderFileConfig {
    all: Option<bool>,
    list: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct FileTestConfig {
    pub name: String,
    pub url: Option<String>,
    pub method: Option<String>,
    pub headers: Option<Vec<String>>,
    pub payload: Option<PayloadConfig>,
    pub capture_headers: Option<CaptureHeaderFileConfig>,
}

#[derive(Debug, Deserialize)]
pub struct FileConfig {
    pub default_url: Option<String>,
    pub default_method: Option<String>,
    pub default_headers: Option<Vec<String>>,
    pub default_capture_headers: Option<CaptureHeaderFileConfig>,
    pub tests: Vec<FileTestConfig>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub tests: Vec<TestConfig>,
}

#[derive(Debug, Clone)]
pub struct TestConfig {
    pub name: String,
    pub url: Uri,
    pub method: String,
    pub headers: Vec<String>,
    pub payload: Option<PayloadConfig>,
    pub capture_headers: CaptureHeaderConfig,
}

#[derive(Debug, Clone)]
pub enum CaptureHeaderConfig {
    All,
    List(HashSet<String>),
}

impl Default for CaptureHeaderConfig {
    fn default() -> CaptureHeaderConfig {
        CaptureHeaderConfig::List(HashSet::new())
    }
}

impl From<CaptureHeaderFileConfig> for CaptureHeaderConfig {
    fn from(fc: CaptureHeaderFileConfig) -> CaptureHeaderConfig {
        if fc.all.unwrap_or(false) {
            CaptureHeaderConfig::All
        } else {
            CaptureHeaderConfig::List(
                fc.list
                    .map(|v| v.into_iter().map(|val| val.to_lowercase()).collect())
                    .unwrap_or_else(|| HashSet::new()),
            )
        }
    }
}

#[derive(Debug, Fail)]
pub enum ConfigError {
    #[fail(display = "Missing url for test '{}' and no default_url set.", _0)]
    MissingUrl(String),
}

impl Config {
    fn fill_defaults(unresolved: FileConfig, path: &Path) -> Result<Config, Error> {
        let default_method = unresolved.default_method.unwrap_or_else(|| "GET".into());
        let default_headers = unresolved.default_headers.unwrap_or_else(|| Vec::new());

        let default_capture_headers = unresolved
            .default_capture_headers
            .map(CaptureHeaderConfig::from)
            .unwrap_or_else(|| CaptureHeaderConfig::default());

        let default_url = match unresolved.default_url.as_ref() {
            Some(ref url_res) => Some(url_res.parse::<Uri>()?),
            None => {
                for test in unresolved.tests.iter() {
                    if test.url.is_none() {
                        return Err(ConfigError::MissingUrl(test.name.clone()).into());
                    }
                }
                None
            }
        };

        let tests: Result<Vec<TestConfig>, Error> = unresolved
            .tests
            .into_iter()
            .map(move |t| {
                let url = match t.url.as_ref() {
                    Some(ref url) => url.parse::<Uri>(),
                    None => Ok(default_url.clone().unwrap()),
                };

                Ok(TestConfig {
                    name: t.name,
                    url: url?,
                    method: t.method.unwrap_or_else(|| default_method.clone()),
                    headers: t.headers.unwrap_or_else(|| default_headers.clone()),
                    capture_headers: t
                        .capture_headers
                        .map(CaptureHeaderConfig::from)
                        .unwrap_or_else(|| default_capture_headers.clone()),
                    payload: t.payload.map(|p| p.make_absolute(&path)),
                })
            })
            .collect();

        Ok(Config { tests: tests? })
    }

    pub fn load<P: AsRef<Path>>(path: P) -> Result<Config, Error> {
        let mut f = File::open(path.as_ref())?;
        let mut contents = String::new();
        f.read_to_string(&mut contents)?;
        let config: FileConfig = toml::from_str(&contents)?;
        let p = path.as_ref().parent().unwrap_or_else(|| Path::new("/"));
        Config::fill_defaults(config, &p)
    }
}
