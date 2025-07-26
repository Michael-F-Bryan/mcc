use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use clap::Parser;

/// Compile a file.
#[derive(Debug, Parser)]
pub struct Compile {
    #[clap(flatten)]
    stop_at: Stage,
    /// Keep the assembly file.
    #[clap(short = 'S')]
    keep_assembly: bool,
    /// The C compiler to use.
    #[clap(long, env = "CC", hide = true, default_value = "cc")]
    cc: OsString,
    /// The output file to write the compiled object code to.
    #[clap(short, long, default_value = "a.out")]
    output: PathBuf,
    input: PathBuf,
}

impl Compile {
    #[tracing::instrument(skip_all)]
    pub fn run(self) -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;

        let preprocessed = temp.path().join("preprocessed.c");
        preprocess(&self.cc, &self.input, &preprocessed)?;

        let ast = parse(&preprocessed)?;

        if self.stop_at.lex || self.stop_at.parse {
            return Ok(());
        }

        let asm = temp.path().join("assembly.s");
        let assembly = compile(ast)?;
        std::fs::write(&asm, assembly)?;

        if self.keep_assembly {
            std::fs::copy(&asm, self.output.with_extension("s"))?;
        }

        if self.stop_at.codegen {
            return Ok(());
        }

        assemble_and_link(&self.cc, &asm, &self.output)?;

        Ok(())
    }
}

pub fn preprocess(cc: impl AsRef<OsStr>, src: &Path, dest: &Path) -> anyhow::Result<()> {
    run_cmd(
        Command::new(cc)
            .arg("-E")
            .arg("-P")
            .arg(src)
            .arg("-o")
            .arg(dest)
            .stdin(Stdio::null()),
    )?;

    Ok(())
}

#[derive(Debug, Clone)]
pub struct AST;

pub fn parse(path: &Path) -> anyhow::Result<AST> {
    let _src = std::fs::read_to_string(path)?;
    todo!();
}

/// Compile a source file into assembly.
pub fn compile(ast: AST) -> anyhow::Result<String> {
    todo!();
}

/// Assemble an assembly file into object code.
pub fn assemble_and_link(cc: &OsString, assembly: &Path, dest: &Path) -> anyhow::Result<()> {
    run_cmd(Command::new(cc).arg("-c").arg(assembly).arg("-o").arg(dest))?;
    Ok(())
}

#[tracing::instrument(skip_all)]
fn run_cmd(cmd: &mut Command) -> anyhow::Result<String> {
    tracing::debug!(cmd=?cmd, "Running command");
    let output = cmd.output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.trim().is_empty() {
            tracing::warn!("Stderr: {}", stderr);
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.trim().is_empty() {
            tracing::warn!("Stdout: {}", stdout);
        }

        anyhow::bail!("Preprocessing failed");
    }

    let stdout = String::from_utf8(output.stdout)?;
    Ok(stdout)
}

#[derive(Debug, Parser)]
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
