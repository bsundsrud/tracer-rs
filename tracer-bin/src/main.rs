mod config;
mod http;
mod reporting;

use hyper::Uri;
use crate::config::{PayloadConfig, Config, CaptureHeaderConfig};
use crate::http::TestExecutor;
use clap::{value_t, App, Arg, SubCommand};
use failure::Error;
use futures::future;
use futures::Future;
use slog::{o, Drain, Level};
use tokio::runtime::Runtime;

fn root_logger(level: Level) -> slog::Logger {
    let decorator = slog_term::TermDecorator::new().stdout().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let async_drain = slog_async::Async::new(drain).build().fuse();
    let level_filter = slog::LevelFilter(async_drain, level).fuse();
    slog::Logger::root(level_filter, o!())
}

fn run_tests(logger: slog::Logger, config: Config, repeat: Option<usize>) -> Result<(), Error> {
    let t = TestExecutor::new(config, logger);
    let mut rt = Runtime::new()?;
    rt.spawn(future::lazy(move || t.execute_repeated_tests(repeat)));
    rt.shutdown_on_idle().wait().unwrap();
    Ok(())
}

fn main() {
    let matches = App::new("Tracer")
        .version("1.0")
        .author("Benn Sundsrud <benn.sundsrud@gmail.com>")
        .about("Test web endpoints and measure response times")
        .arg(
            Arg::with_name("C")
                .short("C")
                .long("continuous")
                .help("Continuous mode")
                .conflicts_with("n")
                .required(false),
        )
        .arg(
            Arg::with_name("n")
                .short("n")
                .value_name("COUNT")
                .help("Repeat request a set number of times")
                .takes_value(true)
                .conflicts_with("C")
                .required(false),
        )
        .arg(
            Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help("Sets verbosity level"),
        )
        .arg(
            Arg::with_name("header")
                .value_name("HEADER")
                .short("H")
                .long("header")
                .help("Header to include in request, in HEADER=VALUE format.  Can be specified multiple times. Case insensitive")
                .takes_value(true)
                .multiple(true)
                .required(false)
        )
        .arg(
            Arg::with_name("capture")
                .value_name("HEADER")
                .short("i")
                .long("capture")
                .help("Header to capture from request. Can be specified multiple times. Case insensitive.")
                .conflicts_with("capture-all")
                .takes_value(true)
                .multiple(true)
                .required(false)
        )
        .arg(
            Arg::with_name("capture-all")
                .long("capture-all")
                .help("Capture all headers from response")
                .conflicts_with("capture")
                .required(false)
        )
        .arg(
            Arg::with_name("body-file")
                .value_name("BODY_FILE")
                .short("f")
                .long("body")
                .help("File to use as request body")
                .required(false)
                .takes_value(true)
        )
        .arg(
            Arg::with_name("method")
                .value_name("METHOD")
                .short("X")
                .long("method")
                .help("HTTP Method to use (Default GET)")
                .required(false)
                .takes_value(true)
        )
        .arg(
            Arg::with_name("URL")
                .takes_value(true)
                .index(1)
                .help("URL to test")
        )
        .subcommand(
            SubCommand::with_name("test")
                .arg(Arg::with_name("config")
                     .value_name("CONFIG")
                     .index(1)
                     .takes_value(true)
                     .required(true)
                     .help("Config file that specifies the test(s) to run")
                )
        )
                         
        .get_matches();
    let config = if let Some(m) = matches.subcommand_matches("test") {
        let config_path = m.value_of("config").unwrap();
        match Config::load(&config_path) {
            Ok(conf) => conf,
            Err(e) => {
                eprintln!("Could not load config: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        let url = matches.value_of("URL").expect("No URL value").parse::<Uri>().expect("Invalid URL");
        let method = matches.value_of("method").unwrap_or("GET").to_string();
        let headers: Vec<String> = matches.values_of("header").map(|h| h.map(|s| s.to_string()).collect()).unwrap_or(Vec::new());
        let payload = matches.value_of("body-file").map(|f| PayloadConfig::relative_to_current(f));
        let capture_headers = if matches.is_present("capture-all") {
            CaptureHeaderConfig::all()
        } else if matches.is_present("capture") {
            let captures: Vec<String> = matches.values_of("capture").map(|v| v.map(|s| s.to_string()).collect()).unwrap_or(Vec::new());
            CaptureHeaderConfig::list(&captures)
        } else {
            CaptureHeaderConfig::empty()
        };
        Config::single(url, method, headers, payload, capture_headers)
    };
   ;
    let repeat = if matches.is_present("C") {
        None
    } else {
        if matches.is_present("n") {
            let count = value_t!(matches, "n", usize).unwrap_or_else(|e| e.exit());
            Some(count)
        } else {
            Some(1)
        }
    };
    let level = match matches.occurrences_of("v") {
        0 => Level::Warning,
        1 => Level::Info,
        2 => Level::Debug,
        3 => Level::Trace,
        _ => {
            eprintln!("WARNING: more than -vvv is ignored");
            Level::Trace
        }
    };
    let logger = root_logger(level);
    match run_tests(logger.clone(), config, repeat) {
        Err(e) => {
            eprintln!("Error running tests: {}", e);
            std::process::exit(1);
        }
        _ => {}
    }
}
