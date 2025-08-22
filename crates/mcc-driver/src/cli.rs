use std::{ffi::OsString, ops::ControlFlow, path::PathBuf, str::FromStr, sync::LazyLock};

use anyhow::Context;
use clap::{ColorChoice as ClapColor, Parser};
use codespan_reporting::{
    diagnostic::Severity,
    term::{self, termcolor::ColorChoice as TermColor},
};
use mcc::{
    Files, Text, codegen::asm, diagnostics::Diagnostics, lowering::tacky, target_lexicon::Triple,
    types::Ast,
};
use tracing_subscriber::{EnvFilter, fmt::format::FmtSpan};

use crate::{Callbacks, Config, Outcome};

const LOG_FILTERS: &[&str] = &["warn", "mcc=debug", "mcc-syntax=debug", "mcc-driver=debug"];

/// A `main()` function that parses the command line arguments and runs the
/// compiler.
#[doc(hidden)]
pub fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| LOG_FILTERS.join(",").parse().unwrap());

    tracing_subscriber::fmt()
        .compact()
        .with_env_filter(env_filter)
        .with_span_events(FmtSpan::CLOSE)
        .init();

    cli.run()
}

/// Compile a file.
#[derive(Debug, clap::Parser)]
struct Cli {
    #[clap(flatten)]
    stop_at: Stage,
    /// Keep the assembly file.
    #[clap(short = 'S')]
    keep_assembly: bool,
    /// The C compiler to use.
    #[clap(long, env = "CC", hide = true, default_value = "cc")]
    cc: OsString,
    /// The output file to write the compiled object code to.
    #[clap(short, long)]
    output: Option<PathBuf>,
    #[clap(flatten)]
    color: colorchoice_clap::Color,
    #[clap(long, default_value_t = DEFAULT_TARGET.clone(), value_parser = parse_target)]
    target: Triple,
    input: PathBuf,
}

impl Cli {
    #[tracing::instrument(level = "info", skip_all)]
    pub fn run(self) -> anyhow::Result<()> {
        let src = std::fs::read_to_string(&self.input)?;
        let db = mcc::Database::default();

        let source_file = mcc::types::SourceFile::new(
            &db,
            Text::from(self.input.display().to_string()),
            src.into(),
        );
        let mut files = Files::new();
        files.add(&db, source_file);

        let cfg = Config {
            db,
            target: self.target.clone(),
            input: source_file,
            cc: self.cc.clone(),
            output: self.output.clone(),
        };

        let assembly_path = if self.keep_assembly {
            Some(
                self.output
                    .clone()
                    .unwrap_or_else(|| self.input.clone())
                    .with_extension("s"),
            )
        } else {
            None
        };

        let mut cb = DefaultCallbacks::new(self.stop_at, self.color.color, files, assembly_path);

        match crate::callbacks::run(&mut cb, cfg) {
            Outcome::Ok => {}
            Outcome::Err(e) => {
                return Err(e);
            }
            Outcome::EarlyReturn(_) => {
                return Err(anyhow::anyhow!("Compilation failed"));
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
struct DefaultCallbacks {
    assembly_path: Option<PathBuf>,
    stop_at: Stage,
    colour: TermColor,
    files: Files,
}

impl DefaultCallbacks {
    fn new(
        stop_at: Stage,
        colour: colorchoice_clap::ColorChoice,
        files: Files,
        assembly_path: Option<PathBuf>,
    ) -> Self {
        let colour = match colour {
            ClapColor::Auto => TermColor::Auto,
            ClapColor::Always => TermColor::Always,
            ClapColor::Never => TermColor::Never,
        };
        DefaultCallbacks {
            assembly_path,
            stop_at,
            colour,
            files,
        }
    }

    fn emit_diagnostics(&self, diags: &[&Diagnostics]) -> Result<(), anyhow::Error> {
        let mut writer = codespan_reporting::term::termcolor::StandardStream::stderr(self.colour);

        let cfg = codespan_reporting::term::Config::default();

        for diag in diags {
            term::emit(&mut writer, &cfg, &self.files, &diag.0)?;
        }

        Ok(())
    }

    fn handle_diags(&mut self, diags: &[&Diagnostics]) -> ControlFlow<Result<(), anyhow::Error>> {
        if let Err(e) = self.emit_diagnostics(diags) {
            return ControlFlow::Break(Err(e));
        }

        if diags.iter().any(|d| d.0.severity >= Severity::Error) {
            return ControlFlow::Break(Err(anyhow::anyhow!("Compilation failed")));
        }

        ControlFlow::Continue(())
    }
}

impl Callbacks for DefaultCallbacks {
    type Output = Result<(), anyhow::Error>;

    fn after_parse<'db>(
        &mut self,
        _db: &'db dyn mcc::Db,
        _source_file: mcc::types::SourceFile,
        _ast: Ast<'db>,
        diags: Vec<&Diagnostics>,
    ) -> ControlFlow<Self::Output> {
        self.handle_diags(&diags)?;

        if self.stop_at.parse || self.stop_at.lex {
            ControlFlow::Break(Ok(()))
        } else {
            ControlFlow::Continue(())
        }
    }

    fn after_lower<'db>(
        &mut self,
        _db: &'db dyn mcc::Db,
        _tacky: tacky::Program<'db>,
        diags: Vec<&Diagnostics>,
    ) -> ControlFlow<Self::Output> {
        self.handle_diags(&diags)?;

        if self.stop_at.tacky {
            ControlFlow::Break(Ok(()))
        } else {
            ControlFlow::Continue(())
        }
    }

    fn after_codegen<'db>(
        &mut self,
        _db: &'db dyn mcc::Db,
        _asm: asm::Program<'db>,
        diags: Vec<&Diagnostics>,
    ) -> ControlFlow<Self::Output> {
        self.handle_diags(&diags)?;

        if self.stop_at.codegen {
            ControlFlow::Break(Ok(()))
        } else {
            ControlFlow::Continue(())
        }
    }

    fn after_render_assembly(
        &mut self,
        _db: &dyn mcc::Db,
        asm: Text,
        diags: Vec<&Diagnostics>,
    ) -> ControlFlow<Self::Output> {
        self.handle_diags(&diags)?;

        if let Some(path) = self.assembly_path.as_mut() {
            tracing::info!(path = %path.display(), "Writing assembly to disk");
            if let Err(e) = std::fs::write(&path, asm)
                .with_context(|| format!("Failed to write assembly to {}", path.display()))
            {
                return ControlFlow::Break(Err(e));
            }
        }

        ControlFlow::Continue(())
    }

    fn after_compile(&mut self, _db: &dyn mcc::Db, _binary: PathBuf) -> ControlFlow<Self::Output> {
        ControlFlow::Continue(())
    }
}

#[derive(Debug, Copy, Clone, clap::Parser)]
struct Stage {
    /// Stop after lexing.
    #[clap(long, group = "stage")]
    lex: bool,
    /// Stop after parsing the file.
    #[clap(long, group = "stage")]
    parse: bool,
    /// Stop after lowering to Three Address Code.
    #[clap(long, group = "stage")]
    tacky: bool,
    /// Stop after generating assembly.
    #[clap(long, group = "stage")]
    codegen: bool,
}

static DEFAULT_TARGET: LazyLock<Triple> = LazyLock::new(mcc::default_target);

fn parse_target(s: &str) -> anyhow::Result<Triple> {
    Triple::from_str(s).map_err(|e| anyhow::anyhow!("{}", e))
}
