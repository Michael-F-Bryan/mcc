use std::{ffi::OsString, path::PathBuf};

use clap::{ColorChoice as ClapColor, Parser};
use codespan_reporting::term::{self, Config, termcolor::ColorChoice as TermColor};
use mcc::{Files, Text};
use tracing_subscriber::EnvFilter;

const LOG_FILTERS: &[&str] = &["warn", "mcc=debug"];

pub fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| LOG_FILTERS.join(",").parse().unwrap());

    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    cli.run()
}

/// Compile a file.
#[derive(Debug, clap::Parser)]
pub struct Cli {
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
    input: PathBuf,
}

impl Cli {
    #[tracing::instrument(skip_all)]
    pub fn run(self) -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;

        let db = mcc::Database::default();

        let src = std::fs::read_to_string(&self.input)?;

        let source_file = mcc::types::SourceFile::new(
            &db,
            Text::from(self.input.display().to_string()),
            src.into(),
        );
        let mut files = Files::new();
        files.add(&db, source_file);

        let preprocessed = mcc::preprocess(&db, self.cc.clone(), source_file)?;

        let preprocessed_path = temp.path().join("preprocessed.c");
        std::fs::write(&preprocessed_path, preprocessed)?;

        let ast = mcc::parse(&db, source_file);
        let diags = mcc::parse::accumulated::<mcc::diagnostics::Diagnostic>(&db, source_file);
        if !diags.is_empty() {
            self.emit_diagnostics(&files, &diags)?;
            anyhow::bail!("Compilation failed");
        }

        if self.stop_at.lex || self.stop_at.parse {
            return Ok(());
        }

        let asm = temp.path().join("assembly.s");
        let assembly = mcc::compile(&db, ast);
        std::fs::write(&asm, assembly)?;

        if self.keep_assembly {
            std::fs::copy(&asm, self.input.with_extension("s"))?;
        }

        if self.stop_at.codegen {
            return Ok(());
        }

        let output = self.output.unwrap_or_else(|| self.input.with_extension(""));

        let assembly_input = mcc::AssemblyInput::new(&db, self.cc.clone(), asm, output.clone());
        let _assembled = mcc::assemble_and_link(&db, assembly_input);

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o755);
            std::fs::set_permissions(&output, perms)?;
        }

        Ok(())
    }

    fn emit_diagnostics(
        &self,
        files: &Files,
        diags: &[&mcc::diagnostics::Diagnostic],
    ) -> anyhow::Result<()> {
        let color = match self.color.color {
            ClapColor::Auto => TermColor::Auto,
            ClapColor::Always => TermColor::Always,
            ClapColor::Never => TermColor::Never,
        };
        let mut writer = codespan_reporting::term::termcolor::StandardStream::stderr(color);

        let cfg = Config::default();

        for diag in diags {
            let diag = diag.to_codespan();
            term::emit(&mut writer, &cfg, files, &diag)?;
        }

        Ok(())
    }
}

#[derive(Debug, clap::Parser)]
struct Stage {
    /// Stop after lexing.
    #[clap(long, group = "stage")]
    lex: bool,
    /// Stop after parsing the file.
    #[clap(long, group = "stage")]
    parse: bool,
    /// Stop after generating assembly.
    #[clap(long, group = "stage")]
    codegen: bool,
}
