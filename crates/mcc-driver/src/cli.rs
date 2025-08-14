use std::{ffi::OsString, ops::ControlFlow, path::PathBuf, str::FromStr, sync::LazyLock};

use clap::{ColorChoice as ClapColor, Parser};
use codespan_reporting::term::{self, termcolor::ColorChoice as TermColor};
use mcc::{
    Files, Text, codegen::asm, diagnostics::Diagnostics, lowering::tacky, target_lexicon::Triple,
    types::Ast,
};
use tracing_subscriber::{EnvFilter, fmt::format::FmtSpan};

use crate::{Callbacks, callbacks::Config};

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

        let mut cb = DefaultCallbacks {
            stop_at: self.stop_at,
            diags: Vec::new(),
            assembly: None,
        };

        crate::callbacks::run(&mut cb, cfg)?;

        self.emit_diagnostics(&files, &cb.diags)?;

        if let Some(assembly) = cb.assembly {
            let dest = self
                .output
                .unwrap_or_else(|| self.input.with_extension("s"));
            std::fs::write(dest, assembly)?;
        }

        Ok(())
    }

    fn emit_diagnostics(&self, files: &Files, diags: &[Diagnostics]) -> anyhow::Result<()> {
        let color = match self.color.color {
            ClapColor::Auto => TermColor::Auto,
            ClapColor::Always => TermColor::Always,
            ClapColor::Never => TermColor::Never,
        };
        let mut writer = codespan_reporting::term::termcolor::StandardStream::stderr(color);

        let cfg = codespan_reporting::term::Config::default();

        for diag in diags {
            term::emit(&mut writer, &cfg, files, &diag.0)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
struct DefaultCallbacks {
    assembly: Option<Text>,
    stop_at: Stage,
    diags: Vec<Diagnostics>,
}

impl Callbacks for DefaultCallbacks {
    fn after_parse<'db>(
        &mut self,
        _db: &'db dyn mcc::Db,
        _source_file: mcc::types::SourceFile,
        _ast: Ast<'db>,
        diags: Vec<&Diagnostics>,
    ) -> ControlFlow<()> {
        self.diags.extend(diags.into_iter().cloned());

        if self.stop_at.parse || self.stop_at.lex {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    }

    fn after_lower<'db>(
        &mut self,
        _db: &'db dyn mcc::Db,
        _tacky: tacky::Program<'db>,
        diags: Vec<&Diagnostics>,
    ) -> ControlFlow<()> {
        self.diags.extend(diags.into_iter().cloned());

        if self.stop_at.tacky {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    }

    fn after_codegen<'db>(
        &mut self,
        _db: &'db dyn mcc::Db,
        _asm: asm::Program<'db>,
        diags: Vec<&Diagnostics>,
    ) -> ControlFlow<()> {
        self.diags.extend(diags.into_iter().cloned());

        if self.stop_at.codegen {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    }

    fn after_render_assembly(
        &mut self,
        _db: &dyn mcc::Db,
        asm: Text,
        diags: Vec<&Diagnostics>,
    ) -> ControlFlow<()> {
        self.diags.extend(diags.into_iter().cloned());
        self.assembly = Some(asm);
        ControlFlow::Continue(())
    }

    fn after_compile(&mut self, _db: &dyn mcc::Db, _binary: PathBuf) -> ControlFlow<()> {
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
