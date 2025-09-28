use std::{
    ffi::OsStr,
    fmt::{self, Display, Formatter},
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    str::FromStr,
};

use anyhow::{Context, Error};
use libtest_mimic::{Failed, Trial};
use mcc::{SourceFile, Text, diagnostics::Diagnostics};
use mcc_driver::{Config as DriverConfig, Outcome, run as driver_run};
use std::ops::ControlFlow;

#[derive(Debug, Clone)]
pub struct Config {
    /// An executable that can be used to run the compiler.
    pub compiler_driver: PathBuf,
    /// The `writing-a-c-compiler-tests/` directory.
    pub test_root: PathBuf,
    pub max_chapter: u32,
}

pub fn discover(
    test_root: &Path,
    expected_results: &ExpectedResults,
) -> Result<Vec<TestCase>, Error> {
    let tests_dir = test_root.join("tests");
    let mut tests = Vec::new();

    for entry in std::fs::read_dir(&tests_dir)? {
        let entry = entry?;
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };

        let Some(chapter) = name.strip_prefix("chapter_") else {
            continue;
        };
        let chapter: u32 = chapter.parse()?;

        for entry in path.read_dir()? {
            let entry = entry?;
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            let Ok(kind) = Kind::from_str(name) else {
                continue;
            };

            for entry in path.read_dir()? {
                let entry = entry?;
                let path = entry.path();
                if path.extension() != Some(OsStr::new("c")) {
                    continue;
                }
                let name = path.file_stem().unwrap().to_str().unwrap();
                let name = format!("chapter_{chapter}::{kind}::{name}");

                let trimmed_path = path.strip_prefix(&tests_dir).unwrap();
                let expected = expected_results
                    .0
                    .get(trimmed_path.to_str().unwrap())
                    .cloned();

                tests.push(TestCase {
                    chapter,
                    kind: kind.clone(),
                    path,
                    name,
                    expected,
                });
            }
        }
    }

    tests.sort_by_cached_key(|t| (t.chapter, t.kind.clone(), t.name.clone()));

    Ok(tests)
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ExpectedResults(std::collections::HashMap<String, TestResult>);

impl ExpectedResults {
    pub fn load(path: &Path) -> Result<Self, Error> {
        let file = std::fs::read_to_string(path)?;
        let expected = serde_json::from_str(&file)?;
        Ok(expected)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TestResult {
    pub return_code: i32,
    #[serde(default)]
    pub stdout: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestCase {
    pub chapter: u32,
    pub kind: Kind,
    pub path: PathBuf,
    pub name: String,
    pub expected: Option<TestResult>,
}

impl TestCase {
    pub fn trial(self) -> Trial {
        let cc = std::env::var_os("CC").unwrap_or_else(|| "cc".into());

        let TestCase {
            kind,
            path,
            name,
            expected,
            ..
        } = self;

        Trial::test(name, move || {
            let db = mcc::Database::default();
            let temp = tempfile::tempdir()?;
            let target = mcc::default_target();
            let src = std::fs::read_to_string(&path)?;
            let path_text = Text::from(path.display().to_string());

            let source_file = SourceFile::new(&db, path_text, src.into());

            let output_path = temp.path().join("output_bin");

            let expectation = match &kind {
                Kind::Valid => Expectation::Success(expected.unwrap()),
                Kind::Invalid(reason) => Expectation::FailAtStage(reason.clone()),
            };

            let mut cb = Callbacks { expectation };

            let cfg = DriverConfig {
                db,
                target,
                cc: cc.clone(),
                output: Some(output_path.clone()),
                input: source_file,
            };

            match driver_run(&mut cb, cfg) {
                Outcome::Ok => {
                    // Compilation succeeded and we didn't error out
                    if !output_path.exists() {
                        return Err(Failed::from(anyhow::anyhow!(
                            "compilation succeeded but output file does not exist"
                        )));
                    }
                }
                Outcome::EarlyReturn(Ok(())) => return Ok(()),
                Outcome::EarlyReturn(Err(e)) => return Err(Failed::from(e)),
                Outcome::Err(e) => return Err(Failed::from(e)),
            }

            if let Kind::Invalid(reason) = kind {
                // If we reached here without early return, then expected error didn't occur
                return Err(Failed::from(format!(
                    "expected error at {reason}, but compilation succeeded"
                )));
            }

            Ok(())
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd)]
pub enum Kind {
    Valid,
    Invalid(String),
}

impl Kind {
    pub fn invalid_reason(&self) -> Option<&str> {
        match self {
            Kind::Invalid(reason) => Some(reason),
            Kind::Valid => None,
        }
    }
}

impl Display for Kind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Kind::Valid => write!(f, "valid"),
            Kind::Invalid(reason) => write!(f, "invalid_{reason}"),
        }
    }
}

impl FromStr for Kind {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "valid" => Ok(Kind::Valid),
            s if s.starts_with("invalid_") => {
                Ok(Kind::Invalid(s.trim_start_matches("invalid_").to_string()))
            }
            _ => anyhow::bail!("invalid kind: {}", s),
        }
    }
}

#[derive(Debug, Clone)]
enum Expectation {
    FailAtStage(String),
    Success(TestResult),
}

#[derive(Debug, Clone)]
struct Callbacks {
    expectation: Expectation,
}

impl Callbacks {
    fn handle_diags(
        &self,
        stages: &[&str],
        diags: Vec<&Diagnostics>,
    ) -> ControlFlow<Result<(), Error>> {
        match &self.expectation {
            Expectation::FailAtStage(stage) if stages.contains(&stage.as_str()) => {
                if diags.is_empty() {
                    ControlFlow::Break(Err(anyhow::anyhow!(
                        "expected to error at the \"{stage}\" stage, but no diagnostics were emitted"
                    )))
                } else {
                    ControlFlow::Break(Ok(()))
                }
            }
            _ => {
                if diags.is_empty() {
                    ControlFlow::Continue(())
                } else {
                    ControlFlow::Break(Err(anyhow::anyhow!(
                        "unexpected error at the {stages:?} stage:\n{diags:#?}"
                    )))
                }
            }
        }
    }
}

impl mcc_driver::Callbacks for Callbacks {
    type Output = Result<(), Error>;

    fn after_parse<'db>(
        &mut self,
        _db: &'db dyn mcc::Db,
        _source_file: mcc::SourceFile,
        _ast: mcc::Ast<'db>,
        diags: Vec<&Diagnostics>,
    ) -> ControlFlow<Result<(), Error>> {
        self.handle_diags(&["lex", "parse"], diags)
    }

    fn after_lower<'db>(
        &mut self,
        _db: &'db dyn mcc::Db,
        _tacky: mcc::lowering::tacky::Program<'db>,
        diags: Vec<&Diagnostics>,
    ) -> ControlFlow<Result<(), Error>> {
        self.handle_diags(&["tacky"], diags)
    }

    fn after_codegen<'db>(
        &mut self,
        _db: &'db dyn mcc::Db,
        _asm: mcc::codegen::asm::Program<'db>,
        diags: Vec<&Diagnostics>,
    ) -> ControlFlow<Result<(), Error>> {
        self.handle_diags(&["codegen"], diags)
    }

    fn after_render_assembly(
        &mut self,
        _db: &dyn mcc::Db,
        _asm: Text,
        diags: Vec<&Diagnostics>,
    ) -> ControlFlow<Result<(), Error>> {
        if !diags.is_empty() {
            return ControlFlow::Break(Err(anyhow::anyhow!(
                "Unexpected errors when rendering assembly"
            )));
        }

        ControlFlow::Continue(())
    }

    fn after_compile(&mut self, _db: &dyn mcc::Db, binary: PathBuf) -> ControlFlow<Self::Output> {
        let TestResult {
            return_code,
            stdout: expected_stdout,
        } = match &self.expectation {
            Expectation::FailAtStage(stage) => {
                return ControlFlow::Break(Err(anyhow::anyhow!(
                    "Compilation should have errored out at the \"{stage}\" stage"
                )));
            }
            Expectation::Success(expected) => expected,
        };

        let Output { status, stdout, .. } = match Command::new(&binary)
            .stdin(Stdio::null())
            .output()
            .with_context(|| format!("failed to spawn \"{}\"", binary.display()))
        {
            Ok(output) => output,
            Err(e) => {
                return ControlFlow::Break(Err(e));
            }
        };

        if status.code() != Some(*return_code) {
            let err = anyhow::anyhow!("expected return code {return_code}, got {status}");
            return ControlFlow::Break(Err(err));
        }

        if let Some(expected_stdout) = expected_stdout {
            let stdout = String::from_utf8_lossy(&stdout);
            if stdout != *expected_stdout {
                let err = anyhow::anyhow!(
                    "expected stdout to be \"{}\", got \"{}\"",
                    expected_stdout,
                    stdout
                );
                return ControlFlow::Break(Err(err));
            }
        }

        ControlFlow::Continue(())
    }
}
