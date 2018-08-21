#[macro_use]
extern crate slog;

use slog::{Drain, Level, Logger};
use std::path::PathBuf;
use std::process;
use structopt::StructOpt;

fn main() {
    let args = Args::from_args();
    let logger = initialize_logging(args.verbosity);

    if let Err(e) = compile_test::run(&args.fixture_dir, &logger) {
        error!(logger, "Testing Failed";
               "error" => e.to_string());

        for cause in e.iter_causes() {
            warn!(logger, "Caused By"; "cause" => cause.to_string());
        }

        drop(logger);
        let bt = e.backtrace().to_string();
        if !bt.is_empty() {
            eprintln!("{}", bt);
        }

        process::exit(1);
    }
}

fn initialize_logging(verbosity: u64) -> Logger {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();

    let level = match verbosity {
        0 => Level::Warning,
        1 => Level::Info,
        2 => Level::Debug,
        _ => Level::Trace,
    };

    let drain = drain.filter_level(level).fuse();

    Logger::root(drain, o!())
}

#[derive(Debug, StructOpt)]
pub struct Args {
    #[structopt(name = "verbosity", short = "v", parse(from_occurrences))]
    verbosity: u64,
    #[structopt(
        name = "fixtures",
        default_value = "tests",
        parse(from_os_str)
    )]
    fixture_dir: PathBuf,
}
