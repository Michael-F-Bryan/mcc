use std::{
    fmt::{self, Display, Formatter},
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::Error;
use libtest_mimic::{Failed, Trial};
use mcc::{Text, diagnostics::Diagnostics, types::SourceFile};
use mcc_driver::{Config as DriverConfig, run as driver_run};
use std::ops::ControlFlow;

#[derive(Debug, Clone)]
pub struct Config {
    /// An executable that can be used to run the compiler.
    pub compiler_driver: PathBuf,
    /// The `writing-a-c-compiler-tests/` directory.
    pub test_root: PathBuf,
    pub max_chapter: u32,
}

pub fn discover(test_root: &Path) -> Result<Vec<TestCase>, Error> {
    let tests_dir = test_root.join("tests");
    let mut tests = Vec::new();

    for entry in std::fs::read_dir(tests_dir)? {
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
                let name = path.file_stem().unwrap().to_str().unwrap();
                let name = format!("chapter_{chapter}::{kind}::{name}");

                tests.push(TestCase {
                    chapter,
                    kind: kind.clone(),
                    path,
                    name,
                });
            }
        }
    }

    tests.sort();

    Ok(tests)
}

#[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd)]
pub struct TestCase {
    pub chapter: u32,
    pub kind: Kind,
    pub path: PathBuf,
    pub name: String,
}

impl TestCase {
    pub fn trial(self) -> Trial {
        let cc = std::env::var_os("CC").unwrap_or_else(|| "cc".into());

        let TestCase {
            kind, path, name, ..
        } = self;

        Trial::test(name, move || {
            let db = mcc::Database::default();
            let temp = tempfile::tempdir()?;
            let target = mcc::default_target();
            let src = std::fs::read_to_string(&path)?;
            let path_text = Text::from(path.display().to_string());
            let kind_str = kind.invalid_reason();

            let source_file = SourceFile::new(&db, path_text, src.into());

            let output_path = temp.path().join("output_bin");

            let mut cb = Callbacks {
                expected: kind_str.map(|s| s.to_string()),
                parse_diags: Vec::new(),
                lower_diags: Vec::new(),
                codegen_diags: Vec::new(),
                render_diags: Vec::new(),
            };

            let cfg = DriverConfig {
                db,
                target,
                cc: cc.clone(),
                output: Some(output_path.clone()),
                input: source_file,
            };

            match driver_run(&mut cb, cfg) {
                Err(e) => {
                    if let Some("codegen") = kind_str {
                        return Ok(());
                    }
                    return Err(Failed::from(e));
                }
                Ok(()) => match kind_str {
                    Some("lex" | "parse") => {
                        if !cb.parse_diags.is_empty() {
                            return Ok(());
                        }
                    }
                    Some("tacky") => {
                        if !cb.lower_diags.is_empty() {
                            return Ok(());
                        }
                    }
                    Some("codegen") => {
                        if !cb.codegen_diags.is_empty() || !cb.render_diags.is_empty() {
                            return Ok(());
                        }
                    }
                    None => {
                        if !cb.parse_diags.is_empty()
                            || !cb.lower_diags.is_empty()
                            || !cb.codegen_diags.is_empty()
                            || !cb.render_diags.is_empty()
                        {
                            return Err(Failed::from(
                                "expected no errors, but diagnostics were emitted",
                            ));
                        }
                    }
                    Some(other) => {
                        return Err(Failed::from(format!("unknown invalid stage: {other}")));
                    }
                },
            }

            dbg!(&cb);

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
struct Callbacks {
    expected: Option<String>,
    parse_diags: Vec<Diagnostics>,
    lower_diags: Vec<Diagnostics>,
    codegen_diags: Vec<Diagnostics>,
    render_diags: Vec<Diagnostics>,
}

impl mcc_driver::Callbacks for Callbacks {
    fn after_parse<'db>(
        &mut self,
        _db: &'db dyn mcc::Db,
        _source_file: mcc::types::SourceFile,
        _ast: mcc::types::Ast<'db>,
        diags: Vec<&Diagnostics>,
    ) -> ControlFlow<()> {
        self.parse_diags.extend(diags.into_iter().cloned());
        match (self.expected.as_deref(), self.parse_diags.is_empty()) {
            (Some("lex" | "parse"), false) => ControlFlow::Break(()),
            _ => ControlFlow::Continue(()),
        }
    }

    fn after_lower<'db>(
        &mut self,
        _db: &'db dyn mcc::Db,
        _tacky: mcc::lowering::tacky::Program<'db>,
        diags: Vec<&Diagnostics>,
    ) -> ControlFlow<()> {
        self.lower_diags.extend(diags.into_iter().cloned());
        match (self.expected.as_deref(), self.lower_diags.is_empty()) {
            (Some("tacky"), false) => ControlFlow::Break(()),
            _ => ControlFlow::Continue(()),
        }
    }

    fn after_codegen<'db>(
        &mut self,
        _db: &'db dyn mcc::Db,
        _asm: mcc::codegen::asm::Program<'db>,
        diags: Vec<&Diagnostics>,
    ) -> ControlFlow<()> {
        self.codegen_diags.extend(diags.into_iter().cloned());
        match (self.expected.as_deref(), self.codegen_diags.is_empty()) {
            (Some("codegen"), false) => ControlFlow::Break(()),
            _ => ControlFlow::Continue(()),
        }
    }

    fn after_render_assembly(
        &mut self,
        _db: &dyn mcc::Db,
        _asm: Text,
        diags: Vec<&Diagnostics>,
    ) -> ControlFlow<()> {
        self.render_diags.extend(diags.into_iter().cloned());
        match (self.expected.as_deref(), self.render_diags.is_empty()) {
            (Some("codegen"), false) => ControlFlow::Break(()),
            _ => ControlFlow::Continue(()),
        }
    }
}
