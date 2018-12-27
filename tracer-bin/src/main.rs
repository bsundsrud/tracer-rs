mod config;
mod http;
mod reporting;
mod timing;

use crate::config::Config;
use crate::http::TestExecutor;
use clap::{App, Arg, SubCommand};
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

fn run_tests(logger: slog::Logger, config: Config) -> Result<(), Error> {
    let t = TestExecutor::new(config, logger);
    let mut rt = Runtime::new()?;

    rt.spawn(future::lazy(move || t.execute_all_tests()));
    rt.shutdown_on_idle().wait().unwrap();
    Ok(())
}

fn main() {
    let matches = App::new("Cache Blaster")
        .version("1.0")
        .author("Benn Sundsrud <benn.sundsrud@gmail.com>")
        .about("Test web endpoints and measure response times")
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("FILE")
                .help("Path to config file")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help("Sets verbosity level"),
        )
        .subcommand(SubCommand::with_name("test").about("run tests"))
        .subcommand(
            SubCommand::with_name("checkpoint")
                .about("create a reference checkpoint for the given config"),
        )
        .get_matches();
    let config_path = matches.value_of("config").unwrap();
    let config = match Config::load(&config_path) {
        Ok(conf) => conf,
        Err(e) => {
            eprintln!("Could not load config: {}", e);
            std::process::exit(1);
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
    match run_tests(logger.clone(), config) {
        Err(e) => {
            eprintln!("Error running tests: {}", e);
            std::process::exit(1);
        }
        _ => {}
    }
}
