use std::path::PathBuf;

use clap::Parser;
use tracing_subscriber::EnvFilter;

const LOG_FILTERS: &[&str] = &["warn", "mcc=info"];

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| LOG_FILTERS.join(",").parse().unwrap());

    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    match cli.into_command() {
        Command::Compile(compile) => {
            println!("Compiling {:?}", compile.input);
            Ok(())
        }
    }
}

/// The main command line interface for the mcc compiler.
#[derive(Debug, Parser)]
#[clap(version, about, author)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
    #[command(flatten)]
    compile: Option<Compile>,
}

impl Cli {
    fn into_command(self) -> Command {
        match self {
            Self {
                command: Some(command),
                compile: None,
            } => command,
            Self {
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

/// Compile a file.
#[derive(Debug, Parser)]
struct Compile {
    input: PathBuf,
}
