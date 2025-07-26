pub mod compile;

use crate::compile::Compile;

use clap::Parser;
use tracing_subscriber::EnvFilter;

const LOG_FILTERS: &[&str] = &["warn", "mcc=debug"];

pub fn main() -> anyhow::Result<()> {
    let cli = App::parse();

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| LOG_FILTERS.join(",").parse().unwrap());

    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    match cli.into_command() {
        Command::Compile(compile) => compile.run(),
    }
}

/// The main command line interface for the mcc compiler.
#[derive(Debug, Parser)]
#[clap(version, about, author)]
struct App {
    #[command(subcommand)]
    command: Option<Command>,
    #[command(flatten)]
    compile: Option<Compile>,
}

impl App {
    fn into_command(self) -> Command {
        match self {
            App {
                command: Some(command),
                compile: None,
            } => command,
            App {
                command: None,
                compile: Some(compile),
            } => Command::Compile(compile),
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Parser)]
enum Command {
    Compile(Compile),
}
