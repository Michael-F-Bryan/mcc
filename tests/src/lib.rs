use std::{
    fmt::{self, Display, Formatter},
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::Error;
use libtest_mimic::{Failed, Trial};

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
                tests.push(TestCase {
                    chapter,
                    kind: kind.clone(),
                    path,
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
}

impl TestCase {
    pub fn trial(self, max_chapter: u32) -> Trial {
        let cc = std::env::var_os("CC").unwrap_or_else(|| "cc".into());

        let TestCase {
            chapter,
            kind,
            path,
        } = self;

        let name = path.file_stem().unwrap().to_str().unwrap();

        Trial::test(format!("chapter_{chapter}::{kind}::{name}"), move || {
            let temp = tempfile::tempdir()?;
            let preprocessed = temp.path().join("preprocessed.c");
            let kind_str = kind.invalid_reason();

            if let Err(e) = mcc::compile::preprocess("cc", &path, &preprocessed) {
                if let Some("lex" | "parse") = kind_str {
                    // Expected error
                    return Ok(());
                } else {
                    return Err(Failed::from(e));
                }
            }

            let ast = match mcc::compile::parse(&preprocessed) {
                Ok(ast) => ast,
                Err(e) => {
                    if let Some("codegen") = kind_str {
                        // Expected error
                        return Ok(());
                    } else {
                        return Err(Failed::from(e));
                    }
                }
            };

            let assembly = match mcc::compile::compile(ast) {
                Ok(assembly) => assembly,
                Err(e) => {
                    if let Some("codegen") = kind_str {
                        // Expected error
                        return Ok(());
                    } else {
                        return Err(Failed::from(e));
                    }
                }
            };

            let asm = temp.path().join("assembly.s");
            std::fs::write(&asm, assembly)?;

            let object_code = temp.path().join("object_code.o");

            if let Err(e) = mcc::compile::assemble_and_link(&cc, &asm, &object_code) {
                if let Some("codegen") = kind_str {
                    // Expected error
                    return Ok(());
                } else {
                    return Err(Failed::from(e));
                }
            };

            Ok(())
        })
        .with_ignored_flag(chapter > max_chapter)
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
            Kind::Invalid(reason) => write!(f, "invalid_{}", reason),
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
