use codespan::CodeMap;
use codespan_reporting::Diagnostic;
use crate::{Outcome, TestCase};
use failure::{Error, ResultExt};
use serde_json;
use std::fs::File;
use std::path::{Path, PathBuf};
use syntax;

#[derive(Debug, Clone)]
pub struct ParseFail {
    test_fixture: PathBuf,
    expected_errors: Option<Diagnostic>,
}

impl ParseFail {
    pub fn for_file<P: Into<PathBuf>>(fixture: P) -> Result<ParseFail, Error> {
        let test_fixture = fixture.into();

        let expected_errors = load_expected_errors(&test_fixture)?;

        Ok(ParseFail {
            test_fixture,
            expected_errors,
        })
    }

    pub fn file_name(&self) -> &Path {
        &self.test_fixture
    }
}

fn load_expected_errors(filename: &Path) -> Result<Option<Diagnostic>, Error> {
    let filename = filename.with_extension("errors.json");

    if !filename.exists() {
        return Ok(None);
    }

    let f = File::open(filename).context("Unable to open the errors file")?;
    let diags = serde_json::from_reader(f).context("Unable to deserialize the Diagnostics")?;

    Ok(Some(diags))
}

impl TestCase for ParseFail {
    fn run(&self) -> Outcome {
        let mut code_map = CodeMap::new();

        let fm = match code_map.add_filemap_from_disk(&self.test_fixture) {
            Ok(f) => f,
            Err(e) => return Outcome::SetupFail(e.into()),
        };

        match syntax::parse(&fm) {
            Ok(_) => Outcome::Fail(failure::err_msg("The test fixture shouldn't have parsed")),
            Err(e) => {
                if let Some(diag) = self.expected_errors.as_ref() {
                    if diag.message != e.message {
                        let msg = format!("Expected {:?} but got {:?}", diag, e);
                        return Outcome::Fail(failure::err_msg(msg));
                    }
                }

                Outcome::Pass
            }
        }
    }

    fn name(&self) -> &str {
        self.test_fixture.file_stem().unwrap().to_str().unwrap()
    }

    fn category(&self) -> &str {
        "parse-fail"
    }
}
