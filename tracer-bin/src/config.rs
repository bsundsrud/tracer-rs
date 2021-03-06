use anyhow::Error as AnyError;
use http::Uri;
use serde::Deserialize;
use std::collections::HashMap;
use std::collections::HashSet;
use std::convert::From;
use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use thiserror::Error;
use toml;

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum PayloadConfig {
    File { file: String },
    Value { value: String },
}

impl PayloadConfig {
    pub fn relative_to_current<S: AsRef<str>>(file: S) -> PayloadConfig {
        let curdir = env::current_dir().expect("Couldn't get current working directory");
        let abspath = Path::new(&curdir).join(file.as_ref());
        PayloadConfig::File {
            file: abspath.to_string_lossy().into(),
        }
    }

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

#[derive(Debug, Clone, Deserialize)]
pub struct CaptureHeaderFileConfig {
    all: Option<bool>,
    list: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct FileTestConfig {
    pub name: String,
    pub url: Option<String>,
    pub method: Option<String>,
    pub headers: Option<HashMap<String, String>>,
    pub payload: Option<PayloadConfig>,
    pub capture_headers: Option<CaptureHeaderFileConfig>,
}

#[derive(Debug, Deserialize)]
pub struct DefaultsConfig {
    pub url: Option<String>,
    pub method: Option<String>,
    pub headers: Option<HashMap<String, String>>,
    pub capture_headers: Option<CaptureHeaderFileConfig>,
}

#[derive(Debug, Deserialize)]
pub struct FileConfig {
    pub defaults: Option<DefaultsConfig>,
    #[serde(rename = "test")]
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
    pub headers: HashMap<String, String>,
    pub payload: Option<PayloadConfig>,
    pub capture_headers: CaptureHeaderConfig,
}

#[derive(Debug, Clone)]
pub enum CaptureHeaderConfig {
    All,
    List(HashSet<String>),
}

impl CaptureHeaderConfig {
    pub fn empty() -> CaptureHeaderConfig {
        CaptureHeaderConfig::List(HashSet::new())
    }

    pub fn all() -> CaptureHeaderConfig {
        CaptureHeaderConfig::All
    }

    pub fn list<S: AsRef<str> + Into<String> + Clone, L: AsRef<[S]>>(
        headers: L,
    ) -> CaptureHeaderConfig {
        let l: HashSet<String> = headers.as_ref().iter().map(|s| s.clone().into()).collect();
        CaptureHeaderConfig::List(l)
    }
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
                    .unwrap_or_else(HashSet::new),
            )
        }
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Missing url for test '{0}' and no default_url set.")]
    MissingUrl(String),
}

impl Config {
    pub fn single(
        url: Uri,
        method: String,
        headers: HashMap<String, String>,
        payload: Option<PayloadConfig>,
        capture_headers: CaptureHeaderConfig,
    ) -> Config {
        let t = TestConfig {
            name: url.to_string(),
            url,
            method,
            headers,
            payload,
            capture_headers,
        };

        Config { tests: vec![t] }
    }

    fn fill_defaults(unresolved: FileConfig, path: &Path) -> Result<Config, AnyError> {
        let default_method = unresolved
            .defaults
            .as_ref()
            .and_then(|d| d.method.clone())
            .unwrap_or_else(|| "GET".into());
        let default_headers = unresolved
            .defaults
            .as_ref()
            .and_then(|d| d.headers.clone())
            .unwrap_or_else(HashMap::new);

        let default_capture_headers = unresolved
            .defaults
            .as_ref()
            .and_then(|d| d.capture_headers.clone())
            .map(CaptureHeaderConfig::from)
            .unwrap_or_else(CaptureHeaderConfig::default);

        let default_url = match unresolved
            .defaults
            .as_ref()
            .and_then(|d| d.url.clone())
            .as_ref()
        {
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

        let tests: Result<Vec<TestConfig>, AnyError> = unresolved
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

    pub fn load<P: AsRef<Path>>(path: P) -> Result<Config, AnyError> {
        let mut f = File::open(path.as_ref())?;
        let mut contents = String::new();
        f.read_to_string(&mut contents)?;
        let config: FileConfig = toml::from_str(&contents)?;
        let p = path.as_ref().parent().unwrap_or_else(|| Path::new("/"));
        Config::fill_defaults(config, &p)
    }
}
